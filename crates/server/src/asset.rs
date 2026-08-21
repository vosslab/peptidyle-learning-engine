//! Public-CDN and authorized short-lived asset delivery (MOD-API-ASSET).
//!
//! The route resolves one immutable database record; it never accepts an
//! object key and never lists a bucket. Public catalog assets redirect to the
//! configured CDN without authentication or object-store signing. Protected
//! content is authorized and audited by the store before the object backend
//! receives the exact typed key.

use std::str::FromStr;
use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::header::{CACHE_CONTROL, ETAG, LOCATION, PRAGMA, REFERRER_POLICY};
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use learning_data_access::{
    AssetDeliveryId, AssetStore, CourseAppearanceStore, SessionStore, StoreError,
};
use objects::{
    Bucket, ObjectCategory, ObjectKey, ObjectRecord, ObjectStore, ObjectStoreError, SignedUrl,
};
use question_model::CourseBannerId;
use serde::Serialize;
use url::Url;

use crate::auth::{auth_error_response, no_store, resolve_request_session};

const CONTENT_URL_MAX_MILLIS: i64 = 60 * 60 * 1_000;
const STUDENT_RECORD_URL_MAX_MILLIS: i64 = 5 * 60 * 1_000;

/// Maps an immutable public object record to its CDN URL.
pub trait PublicAssetUrlResolver: Send + Sync {
    /// Produces a public URL from trusted configuration and the typed key.
    fn public_url(&self, record: &ObjectRecord) -> Result<String, PublicAssetUrlError>;
}

/// Validated base URL for the public-assets bucket CDN.
///
/// A fixed path prefix is supported, for example
/// `https://cdn.example.test/ple-content`. It is normalized once at startup;
/// callers can only append a typed object key below that prefix.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublicAssetBaseUrl(String);

impl PublicAssetBaseUrl {
    /// Validates a production public-asset base. Production always uses HTTPS.
    pub fn new(value: impl Into<String>) -> Result<Self, PublicAssetUrlError> {
        Self::parse(value.into(), false)
    }

    /// Validates the explicit local-development public-asset base.
    ///
    /// HTTP is accepted only through this typed local-development constructor;
    /// production configuration must use [`Self::new`].
    #[cfg_attr(not(feature = "local-development-auth"), allow(dead_code))]
    pub(crate) fn local_development(value: impl Into<String>) -> Result<Self, PublicAssetUrlError> {
        Self::parse(value.into(), true)
    }

    fn parse(value: String, allow_http: bool) -> Result<Self, PublicAssetUrlError> {
        // Whitespace and URL normalizations are configuration mistakes, not
        // something a security boundary should silently reinterpret.
        if value.is_empty() || value.trim() != value {
            return Err(PublicAssetUrlError);
        }
        let parsed = Url::parse(&value).map_err(|_| PublicAssetUrlError)?;
        if parsed.cannot_be_a_base()
            || !matches!(parsed.scheme(), "https" | "http")
            || (parsed.scheme() == "http" && !allow_http)
            || parsed.host_str().is_none()
            || !parsed.username().is_empty()
            || parsed.password().is_some()
            || parsed.query().is_some()
            || parsed.fragment().is_some()
        {
            return Err(PublicAssetUrlError);
        }

        let raw_path = parsed.path();
        if raw_path.ends_with("//") {
            return Err(PublicAssetUrlError);
        }
        let path = raw_path.trim_end_matches('/');
        let normalized_path = if path.is_empty() {
            String::new()
        } else {
            let Some(path) = path.strip_prefix('/') else {
                return Err(PublicAssetUrlError);
            };
            if path.is_empty()
                || path.split('/').any(|segment| {
                    segment.is_empty()
                        || !segment.bytes().all(|byte| {
                            byte.is_ascii_alphanumeric()
                                || matches!(byte, b'-' | b'.' | b'_' | b'~')
                        })
                })
            {
                return Err(PublicAssetUrlError);
            }
            format!("/{path}")
        };
        let canonical = format!(
            "{}{}",
            parsed.origin().ascii_serialization(),
            normalized_path
        );
        // Permit one conventional trailing slash, but reject all other URL
        // parser rewrites (dot segments, escaped separators, default-port
        // spellings, and host/authority ambiguity) rather than changing the
        // effective asset authority behind an operator's back.
        if value.trim_end_matches('/') != canonical {
            return Err(PublicAssetUrlError);
        }
        Ok(Self(canonical))
    }
}

impl PublicAssetUrlResolver for PublicAssetBaseUrl {
    fn public_url(&self, record: &ObjectRecord) -> Result<String, PublicAssetUrlError> {
        // Only the public-assets bucket is CDN-readable. Require the complete
        // trusted record shape, rather than merely a category, before
        // constructing a public URL.
        let ObjectKey::ProblemAsset {
            object, version, ..
        } = &record.key
        else {
            return Err(PublicAssetUrlError);
        };
        if record.id != *object
            || record.bucket != Bucket::PublicAssets
            || record.category != ObjectCategory::Asset
            || record.version != Some(*version)
        {
            return Err(PublicAssetUrlError);
        }
        Ok(format!("{}/{}", self.0, record.key.path()))
    }
}

/// Invalid trusted public-asset URL configuration or mapping.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PublicAssetUrlError;

impl std::fmt::Display for PublicAssetUrlError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("public asset URL is invalid")
    }
}

impl std::error::Error for PublicAssetUrlError {}

/// Builds the asset-delivery route around independent metadata and byte stores.
pub fn router<S, O, C>(store: Arc<S>, objects: Arc<O>, public_assets: Arc<C>) -> Router
where
    S: AssetStore + CourseAppearanceStore + SessionStore + 'static,
    O: ObjectStore + 'static,
    C: PublicAssetUrlResolver + 'static,
{
    Router::new()
        .route("/api/assets/{id}", get(get_asset::<S, O, C>))
        .route(
            "/api/assets/{id}/delivery",
            post(issue_protected_delivery::<S, O, C>),
        )
        .with_state(AssetRouteState {
            store,
            objects,
            public_assets,
        })
}

struct AssetRouteState<S, O, C> {
    store: Arc<S>,
    objects: Arc<O>,
    public_assets: Arc<C>,
}

impl<S, O, C> Clone for AssetRouteState<S, O, C> {
    fn clone(&self) -> Self {
        Self {
            store: Arc::clone(&self.store),
            objects: Arc::clone(&self.objects),
            public_assets: Arc::clone(&self.public_assets),
        }
    }
}

async fn get_asset<S, O, C>(
    State(state): State<AssetRouteState<S, O, C>>,
    Path(raw_id): Path<String>,
) -> Response
where
    S: AssetStore + CourseAppearanceStore + SessionStore + 'static,
    O: ObjectStore + 'static,
    C: PublicAssetUrlResolver + 'static,
{
    let Ok(delivery) = AssetDeliveryId::from_str(&raw_id) else {
        return error_response(StatusCode::NOT_FOUND, "asset not found");
    };
    let public = match state.store.get_public_asset_delivery(delivery).await {
        Ok(record) => record,
        Err(error) => return store_error_response(error),
    };
    if let Some(record) = public {
        let url = match state.public_assets.public_url(&record.object) {
            Ok(url) => url,
            Err(_) => {
                return error_response(
                    StatusCode::SERVICE_UNAVAILABLE,
                    "asset delivery unavailable",
                );
            }
        };
        return public_redirect(url, &record.object);
    }

    // A GET is deliberately incapable of authorizing, auditing, or issuing a
    // bearer capability for a protected object. Apart from avoiding unsafe
    // state changes on navigations, this means a cache or cross-site request
    // cannot distinguish a protected delivery identifier from a nonexistent
    // one. The browser must make the explicit POST below.
    error_response(StatusCode::NOT_FOUND, "asset not found")
}

/// Browser response shape for an authorized, short-lived object capability.
///
/// This is JSON instead of a POST redirect so the browser never turns a
/// signed bearer URL into a navigation/referrer chain. Callers use the URL
/// only as an image/download source after this same-origin request succeeds.
#[derive(Debug, Serialize)]
struct ProtectedAssetDeliveryResponse {
    url: String,
}

async fn issue_protected_delivery<S, O, C>(
    State(state): State<AssetRouteState<S, O, C>>,
    headers: HeaderMap,
    Path(raw_id): Path<String>,
) -> Response
where
    S: AssetStore + CourseAppearanceStore + SessionStore + 'static,
    O: ObjectStore + 'static,
    C: PublicAssetUrlResolver + 'static,
{
    let Ok(delivery) = AssetDeliveryId::from_str(&raw_id) else {
        return error_response(StatusCode::NOT_FOUND, "asset not found");
    };

    // Public assets retain their cacheable GET/CDN path. Do not create a
    // second, stateful delivery protocol for them.
    match state.store.get_public_asset_delivery(delivery).await {
        Ok(Some(_)) => return error_response(StatusCode::METHOD_NOT_ALLOWED, "asset not found"),
        Ok(None) => {}
        Err(error) => return store_error_response(error),
    }

    let authenticated = match resolve_request_session(state.store.as_ref(), &headers).await {
        Ok(authenticated) => authenticated,
        Err(error) => return auth_error_response(error),
    };
    let authorized = match state
        .store
        .authorize_asset_delivery(
            authenticated.tenant_context,
            authenticated.record.subject.user(),
            delivery,
        )
        .await
    {
        Ok(authorized) => authorized,
        Err(StoreError::NotFound) => {
            let banner = CourseBannerId::from_uuid(delivery.as_uuid());
            match state
                .store
                .authorize_course_banner_delivery(
                    authenticated.tenant_context,
                    authenticated.record.token_hash,
                    banner,
                )
                .await
            {
                Ok(authorized) => authorized,
                Err(error) => return store_error_response(error),
            }
        }
        Err(error) => return store_error_response(error),
    };
    let signed = match state
        .objects
        .signed_url(&authorized.record.object.key, authorized.authorized_at)
        .await
    {
        Ok(signed) => signed,
        Err(error) => return object_error_response(error),
    };
    if !valid_signed_lifetime(
        &signed,
        authorized.record.object.bucket,
        authorized.authorized_at,
    ) {
        return error_response(
            StatusCode::SERVICE_UNAVAILABLE,
            "asset delivery unavailable",
        );
    }
    protected_delivery_response(signed.url)
}

fn valid_signed_lifetime(
    signed: &SignedUrl,
    bucket: Bucket,
    authorized_at: question_model::ActivityTimestamp,
) -> bool {
    let maximum = match bucket {
        Bucket::PublicAssets | Bucket::PrivateContent => CONTENT_URL_MAX_MILLIS,
        Bucket::StudentRecords => STUDENT_RECORD_URL_MAX_MILLIS,
        Bucket::TempProcessing => return false,
    };
    let Some(latest) = authorized_at.as_unix_millis().checked_add(maximum) else {
        return false;
    };
    signed.expires_at > authorized_at && signed.expires_at.as_unix_millis() <= latest
}

fn public_redirect(url: String, record: &ObjectRecord) -> Response {
    let Ok(location) = HeaderValue::from_str(&url) else {
        return error_response(
            StatusCode::SERVICE_UNAVAILABLE,
            "asset delivery unavailable",
        );
    };
    let Ok(etag) = HeaderValue::from_str(&format!("\"{}\"", record.sha256)) else {
        return error_response(
            StatusCode::SERVICE_UNAVAILABLE,
            "asset delivery unavailable",
        );
    };
    let mut response = StatusCode::TEMPORARY_REDIRECT.into_response();
    response.headers_mut().insert(LOCATION, location);
    response.headers_mut().insert(
        CACHE_CONTROL,
        HeaderValue::from_static("public, max-age=31536000, immutable"),
    );
    response.headers_mut().insert(ETAG, etag);
    response
}

fn protected_delivery_response(url: String) -> Response {
    // A signed URL is produced by trusted object-storage code, but still
    // refuse values that cannot be represented as a JSON string by serde.
    // (All Rust strings serialize, so this also keeps the response shape
    // simple and fully typed.)
    let mut response = no_store(Json(ProtectedAssetDeliveryResponse { url }).into_response());
    response
        .headers_mut()
        .insert(PRAGMA, HeaderValue::from_static("no-cache"));
    response
        .headers_mut()
        .insert(REFERRER_POLICY, HeaderValue::from_static("no-referrer"));
    response
}

fn store_error_response(error: StoreError) -> Response {
    match error {
        StoreError::NotFound => error_response(StatusCode::NOT_FOUND, "asset not found"),
        StoreError::TenantMismatch | StoreError::Forbidden => {
            error_response(StatusCode::NOT_FOUND, "asset not found")
        }
        StoreError::AlreadyExists
        | StoreError::Conflict
        | StoreError::InvalidRecord(_)
        | StoreError::RunModel(_)
        | StoreError::TimedOut
        | StoreError::RetryableTransaction
        | StoreError::Unavailable(_) => error_response(
            StatusCode::SERVICE_UNAVAILABLE,
            "asset delivery unavailable",
        ),
    }
}

fn object_error_response(error: ObjectStoreError) -> Response {
    match error {
        ObjectStoreError::NotFound | ObjectStoreError::NotSignable => {
            error_response(StatusCode::NOT_FOUND, "asset not found")
        }
        ObjectStoreError::AlreadyExists
        | ObjectStoreError::ChecksumMismatch
        | ObjectStoreError::NumericOverflow
        | ObjectStoreError::Unavailable(_) => error_response(
            StatusCode::SERVICE_UNAVAILABLE,
            "asset delivery unavailable",
        ),
    }
}

fn error_response(status: StatusCode, message: &str) -> Response {
    no_store((status, Json(serde_json::json!({ "error": message }))).into_response())
}

#[cfg(test)]
#[path = "asset/tests.rs"]
mod tests;
