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
use axum::routing::get;
use axum::{Json, Router};
use objects::{Bucket, ObjectRecord, ObjectStore, ObjectStoreError, SignedUrl};
use store::{AssetDeliveryId, AssetStore, SessionStore, StoreError};

use crate::auth::{auth_error_response, no_store, resolve_request_session};

const CONTENT_URL_MAX_MILLIS: i64 = 60 * 60 * 1_000;
const STUDENT_RECORD_URL_MAX_MILLIS: i64 = 5 * 60 * 1_000;

/// Maps an immutable public object record to its CDN URL.
pub trait PublicAssetUrlResolver: Send + Sync {
    /// Produces a public URL from trusted configuration and the typed key.
    fn public_url(&self, record: &ObjectRecord) -> Result<String, PublicAssetUrlError>;
}

/// Validated base URL for the public `content` bucket CDN.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublicAssetBaseUrl(String);

impl PublicAssetBaseUrl {
    /// Accepts an HTTP(S) origin/path without query or fragment components.
    pub fn new(value: impl Into<String>) -> Result<Self, PublicAssetUrlError> {
        let value = value.into();
        let normalized = value.trim_end_matches('/');
        if normalized.is_empty()
            || !(normalized.starts_with("https://") || normalized.starts_with("http://"))
            || normalized.contains('?')
            || normalized.contains('#')
            || HeaderValue::from_str(normalized).is_err()
        {
            return Err(PublicAssetUrlError);
        }
        Ok(Self(normalized.to_string()))
    }
}

impl PublicAssetUrlResolver for PublicAssetBaseUrl {
    fn public_url(&self, record: &ObjectRecord) -> Result<String, PublicAssetUrlError> {
        if record.bucket != Bucket::Content {
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
    S: AssetStore + SessionStore + 'static,
    O: ObjectStore + 'static,
    C: PublicAssetUrlResolver + 'static,
{
    Router::new()
        .route("/api/assets/{id}", get(get_asset::<S, O, C>))
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
    headers: HeaderMap,
    Path(raw_id): Path<String>,
) -> Response
where
    S: AssetStore + SessionStore + 'static,
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
    protected_redirect(signed.url)
}

fn valid_signed_lifetime(
    signed: &SignedUrl,
    bucket: Bucket,
    authorized_at: question_model::ActivityTimestamp,
) -> bool {
    let maximum = match bucket {
        Bucket::Content => CONTENT_URL_MAX_MILLIS,
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

fn protected_redirect(url: String) -> Response {
    let Ok(location) = HeaderValue::from_str(&url) else {
        return error_response(
            StatusCode::SERVICE_UNAVAILABLE,
            "asset delivery unavailable",
        );
    };
    let mut response = no_store(StatusCode::TEMPORARY_REDIRECT.into_response());
    response.headers_mut().insert(LOCATION, location);
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
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request;
    use objects::memory::MemoryObjectStore;
    use objects::{ObjectKey, PutObject};
    use question_model::answer::NumericTolerance;
    use question_model::envelope::ContentBlock;
    use question_model::generation::RandomizationDefinition;
    use question_model::response::ResponseDefinition;
    use question_model::run_policy::{AttemptPolicy, FeedbackDisclosure, TimingPolicy};
    use question_model::taxonomy::License;
    use question_model::{
        ActivityTimestamp, AssetId, BackendCapabilities, Capability, GradingDefinition, ObjectId,
        ProblemId, ProblemVersionRef, PublicationScope, QuestionDefinition, QuestionMetadata,
        QuestionSource, TenantId, UserId, UserRole, VersionId, WorkspaceId,
    };
    use store::memory::MemoryStore;
    use store::{
        AssetDeliveryRecord, AssetDeliveryScope, CatalogStore, DraftRecord, PublishDraftCommand,
        SessionLifetime, SessionSubject, Store, TenantContext,
    };
    use tower::ServiceExt;
    use uuid::Uuid;

    fn id(value: u128) -> Uuid {
        Uuid::from_u128(value)
    }

    fn question(version: VersionId, workspace: WorkspaceId) -> QuestionDefinition {
        QuestionDefinition {
            version,
            problem: None,
            workspace,
            source: QuestionSource::Native {
                family: "asset-fixture".to_string(),
            },
            prompt: vec![ContentBlock::Text {
                markdown: "Identify the peptide bond.".to_string(),
            }],
            response: ResponseDefinition::Numeric {
                tolerance: NumericTolerance::Absolute { epsilon: 0.0 },
                unit: None,
            },
            attempt_policy: AttemptPolicy {
                max_attempts: None,
                feedback: FeedbackDisclosure::ImmediateFull,
            },
            timing_policy: TimingPolicy::Untimed,
            randomization: RandomizationDefinition::Static,
            grading: GradingDefinition::AllOrNothing { points: 1.0 },
            metadata: QuestionMetadata {
                title: "Peptide bond".to_string(),
                tags: Vec::new(),
                taxonomy: Vec::new(),
                license: License::CcBySa,
                language: "en-US".to_string(),
            },
        }
    }

    async fn publish(
        store: &MemoryStore,
        context: TenantContext,
        publisher: UserId,
        problem: ProblemId,
        version: VersionId,
        workspace: WorkspaceId,
        scope: PublicationScope,
    ) {
        let draft = DraftRecord {
            tenant: context.tenant_id(),
            question: question(version, workspace),
            revises: None,
            derived_from: None,
        };
        store
            .upsert_draft(context, draft.clone())
            .await
            .expect("draft");
        store
            .publish_draft(
                context,
                PublishDraftCommand {
                    expected_draft: draft,
                    problem,
                    publisher,
                    scope,
                    capabilities: BackendCapabilities::from_iter([Capability::ServerGrading]),
                },
            )
            .await
            .expect("publish");
    }

    async fn cookie(store: &MemoryStore, tenant: TenantId, user: UserId) -> String {
        let subject = SessionSubject::new(tenant, user, "Asset Fixture", vec![UserRole::Student])
            .expect("session subject");
        crate::auth::issue_session(
            store,
            subject,
            crate::auth::SessionConfig::new(
                SessionLifetime::from_seconds(3_600).expect("session lifetime"),
                crate::auth::CookieTransport::LocalHttp,
            ),
        )
        .await
        .expect("session")
        .set_cookie
        .split(';')
        .next()
        .expect("cookie pair")
        .to_string()
    }

    async fn fixture() -> (
        Arc<MemoryStore>,
        Router,
        String,
        String,
        AssetDeliveryId,
        AssetDeliveryId,
        AssetDeliveryId,
    ) {
        let tenant = TenantId::from_uuid(id(1));
        let context = TenantContext::from_authenticated_session(tenant);
        let publisher = UserId::from_uuid(id(2));
        let student = UserId::from_uuid(id(3));
        let outsider = UserId::from_uuid(id(4));
        let store = Arc::new(MemoryStore::default());
        store
            .set_authoritative_time(ActivityTimestamp::from_unix_millis(10_000))
            .expect("clock");
        let objects = Arc::new(MemoryObjectStore::default());

        let public_problem = ProblemId::from_uuid(id(10));
        let public_version = VersionId::from_uuid(id(11));
        publish(
            store.as_ref(),
            context,
            publisher,
            public_problem,
            public_version,
            WorkspaceId::from_uuid(id(12)),
            PublicationScope::Public,
        )
        .await;
        let institution_problem = ProblemId::from_uuid(id(20));
        let institution_version = VersionId::from_uuid(id(21));
        publish(
            store.as_ref(),
            context,
            publisher,
            institution_problem,
            institution_version,
            WorkspaceId::from_uuid(id(22)),
            PublicationScope::Institution,
        )
        .await;

        let public_asset = AssetId::from_uuid(id(30));
        let public_key = ObjectKey::ProblemAsset {
            problem: public_problem,
            version: public_version,
            asset: public_asset,
            object: ObjectId::from_uuid(id(31)),
        };
        let public_object = objects
            .put(PutObject {
                key: public_key,
                bytes: b"public asset".to_vec(),
                media_type: "image/svg+xml".to_string(),
                license: "CC BY-SA 4.0".to_string(),
                provenance: "test".to_string(),
                created_at: ActivityTimestamp::from_unix_millis(10_000),
            })
            .await
            .expect("public bytes");
        let public = AssetDeliveryRecord {
            id: AssetDeliveryId::from_asset(public_asset),
            object: public_object,
            scope: AssetDeliveryScope::Catalog {
                asset: public_asset,
                reference: ProblemVersionRef {
                    problem: public_problem,
                    version: public_version,
                },
            },
        };

        let institution_asset = AssetId::from_uuid(id(40));
        let institution_key = ObjectKey::ProblemAsset {
            problem: institution_problem,
            version: institution_version,
            asset: institution_asset,
            object: ObjectId::from_uuid(id(41)),
        };
        let institution_object = objects
            .put(PutObject {
                key: institution_key,
                bytes: b"institution asset".to_vec(),
                media_type: "image/png".to_string(),
                license: "institution".to_string(),
                provenance: "test".to_string(),
                created_at: ActivityTimestamp::from_unix_millis(10_000),
            })
            .await
            .expect("institution bytes");
        let institution = AssetDeliveryRecord {
            id: AssetDeliveryId::from_asset(institution_asset),
            object: institution_object,
            scope: AssetDeliveryScope::Catalog {
                asset: institution_asset,
                reference: ProblemVersionRef {
                    problem: institution_problem,
                    version: institution_version,
                },
            },
        };

        let student_object_id = ObjectId::from_uuid(id(50));
        let student_object = objects
            .put(PutObject {
                key: ObjectKey::StudentRecord {
                    tenant,
                    object: student_object_id,
                },
                bytes: b"student export".to_vec(),
                media_type: "application/pdf".to_string(),
                license: "educational record".to_string(),
                provenance: "test export".to_string(),
                created_at: ActivityTimestamp::from_unix_millis(10_000),
            })
            .await
            .expect("student bytes");
        let student_record = AssetDeliveryRecord {
            id: AssetDeliveryId::from_object(student_object_id),
            object: student_object,
            scope: AssetDeliveryScope::StudentRecord {
                tenant,
                authorized_users: vec![student],
            },
        };

        for record in [&public, &institution, &student_record] {
            store
                .register_asset_delivery(context, record.clone())
                .await
                .expect("register delivery");
        }
        let student_cookie = cookie(store.as_ref(), tenant, student).await;
        let outsider_cookie = cookie(store.as_ref(), tenant, outsider).await;
        let app = router(
            Arc::clone(&store),
            objects,
            Arc::new(
                PublicAssetBaseUrl::new("https://cdn.example.test/content").expect("CDN base"),
            ),
        );
        (
            store,
            app,
            student_cookie,
            outsider_cookie,
            public.id,
            institution.id,
            student_record.id,
        )
    }

    #[tokio::test]
    async fn public_assets_bypass_auth_and_signing_for_immutable_cdn_urls() {
        let (store, app, _, _, public, _, _) = fixture().await;
        let response = app
            .oneshot(
                Request::builder()
                    .uri(format!("/api/assets/{public}"))
                    .body(Body::empty())
                    .expect("public request"),
            )
            .await
            .expect("public response");
        assert_eq!(response.status(), StatusCode::TEMPORARY_REDIRECT);
        assert!(
            response.headers()[LOCATION]
                .to_str()
                .expect("location")
                .starts_with("https://cdn.example.test/content/problems/")
        );
        assert_eq!(
            response.headers()[CACHE_CONTROL],
            "public, max-age=31536000, immutable"
        );
        assert!(response.headers().contains_key(ETAG));
        assert!(
            store
                .asset_access_events()
                .expect("audit events")
                .is_empty()
        );
    }

    #[tokio::test]
    async fn protected_assets_require_authorization_log_and_use_bucket_lifetimes() {
        let (store, app, student_cookie, outsider_cookie, _, institution, student_record) =
            fixture().await;
        let unauthenticated = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/api/assets/{institution}"))
                    .body(Body::empty())
                    .expect("unauthenticated request"),
            )
            .await
            .expect("unauthenticated response");
        assert_eq!(unauthenticated.status(), StatusCode::UNAUTHORIZED);

        let institution_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/api/assets/{institution}"))
                    .header("cookie", &student_cookie)
                    .body(Body::empty())
                    .expect("institution request"),
            )
            .await
            .expect("institution response");
        assert_eq!(
            institution_response.status(),
            StatusCode::TEMPORARY_REDIRECT
        );
        assert!(
            institution_response.headers()[LOCATION]
                .to_str()
                .expect("institution location")
                .ends_with("?expires=3610000")
        );
        assert_eq!(institution_response.headers()[CACHE_CONTROL], "no-store");
        assert_eq!(
            institution_response.headers()[REFERRER_POLICY],
            "no-referrer"
        );

        let student_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/api/assets/{student_record}"))
                    .header("cookie", &student_cookie)
                    .body(Body::empty())
                    .expect("student record request"),
            )
            .await
            .expect("student record response");
        assert_eq!(student_response.status(), StatusCode::TEMPORARY_REDIRECT);
        assert!(
            student_response.headers()[LOCATION]
                .to_str()
                .expect("student location")
                .ends_with("?expires=310000")
        );

        let hidden = app
            .oneshot(
                Request::builder()
                    .uri(format!("/api/assets/{student_record}"))
                    .header("cookie", outsider_cookie)
                    .body(Body::empty())
                    .expect("hidden request"),
            )
            .await
            .expect("hidden response");
        assert_eq!(hidden.status(), StatusCode::NOT_FOUND);
        let events = store.asset_access_events().expect("audit events");
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].delivery, institution);
        assert_eq!(events[1].delivery, student_record);
        assert!(
            events
                .iter()
                .all(|event| event.occurred_at == ActivityTimestamp::from_unix_millis(10_000))
        );
    }

    #[test]
    fn public_base_url_rejects_unsafe_or_stateful_values() {
        for value in [
            "",
            "cdn.example.test/content",
            "javascript:alert(1)",
            "https://cdn.example.test/content?token=secret",
            "https://cdn.example.test/content#fragment",
        ] {
            assert_eq!(PublicAssetBaseUrl::new(value), Err(PublicAssetUrlError));
        }
    }
}
