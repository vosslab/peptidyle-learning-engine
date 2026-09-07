//! Authorized redirect route for immutable published Question Asset renditions.
//!
//! The route is deliberately a metadata boundary: PostgreSQL proves the
//! current authority and exact ready rendition, then this module derives one
//! CDN URL from typed identity. It neither reads asset bytes nor creates a
//! bearer URL.

use std::sync::Arc;

use axum::{
    Router,
    extract::{Path, State},
    http::{HeaderMap, HeaderValue, StatusCode, header::COOKIE},
    response::{IntoResponse, Response},
    routing::get,
};
use learning_data_access::{
    QuestionAssetDeliveryStore, ReadyQuestionAssetDelivery, SessionTokenHash, StoreError,
    postgres::{PostgresQuestionAssetDeliveryStore, PostgresSessionStore},
};
use objects::ObjectAddress;
use question_model::{ProductRole, QuestionAssetId};
use url::Url;
use uuid::Uuid;

use crate::auth::{AuthError, resolve_session};

#[derive(Clone)]
struct QuestionAssetDeliveryRouteState {
    sessions: Arc<PostgresSessionStore>,
    store: PostgresQuestionAssetDeliveryStore,
    public_asset_base_url: Url,
}

/// Registers the only public-asset GET route. The configured base is
/// deployment-owned; a browser never supplies a bucket, key, or hostname.
pub fn question_asset_delivery_router(
    sessions: Arc<PostgresSessionStore>,
    store: PostgresQuestionAssetDeliveryStore,
    public_asset_base_url: Url,
) -> Router {
    Router::new()
        .route("/api/assets/{asset_id}", get(get_public_question_asset))
        .with_state(QuestionAssetDeliveryRouteState {
            sessions,
            store,
            public_asset_base_url,
        })
}

/// Validates the configured immutable-public asset base URL once at startup.
pub(crate) fn public_asset_base_url(value: &str) -> Result<Url, String> {
    let mut parsed =
        Url::parse(value).map_err(|_| "public asset base URL must be absolute".to_string())?;
    if !matches!(parsed.scheme(), "http" | "https")
        || parsed.host_str().is_none()
        || !parsed.username().is_empty()
        || parsed.password().is_some()
        || parsed.query().is_some()
        || parsed.fragment().is_some()
    {
        return Err(
            "public asset base URL must be an origin path without credentials, query, or fragment"
                .to_string(),
        );
    }
    if !parsed.path().ends_with('/') {
        let path = format!("{}/", parsed.path());
        parsed.set_path(&path);
    }
    Ok(parsed)
}

async fn get_public_question_asset(
    State(state): State<QuestionAssetDeliveryRouteState>,
    headers: HeaderMap,
    Path(asset_id): Path<String>,
) -> Response {
    let asset_id = match Uuid::parse_str(&asset_id) {
        Ok(value) => QuestionAssetId::from_uuid(value),
        Err(_) => return concealed(),
    };
    let session_hash = match asset_session_hash(&state, &headers).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let rendition = match state
        .store
        .resolve_ready_question_asset_delivery(session_hash, asset_id)
        .await
    {
        Ok(value) => value,
        Err(StoreError::NotFound | StoreError::Forbidden | StoreError::OwnershipMismatch) => {
            return concealed();
        }
        Err(_) => return unavailable(),
    };
    match redirect_response(&state.public_asset_base_url, &rendition) {
        Ok(response) => response,
        Err(()) => unavailable(),
    }
}

async fn asset_session_hash(
    state: &QuestionAssetDeliveryRouteState,
    headers: &HeaderMap,
) -> Result<SessionTokenHash, Box<Response>> {
    match resolve_session(state.sessions.as_ref(), cookie(headers).as_deref()).await {
        Ok(value)
            if matches!(
                value.record.product_role,
                ProductRole::Instructor | ProductRole::Student
            ) =>
        {
            Ok(value.session_hash)
        }
        Ok(_) | Err(AuthError::Unauthenticated) => Err(Box::new(concealed())),
        Err(AuthError::Unavailable(_) | AuthError::Randomness(_)) => Err(Box::new(unavailable())),
    }
}

fn redirect_response(
    base_url: &Url,
    rendition: &ReadyQuestionAssetDelivery,
) -> Result<Response, ()> {
    let address = ObjectAddress::QuestionAsset {
        question_revision: rendition.question_revision.clone(),
        asset: rendition.asset_id,
        object: rendition.public_object_id,
    };
    let location = base_url.join(&address.path()).map_err(|_| ())?;
    let location = HeaderValue::from_str(location.as_str()).map_err(|_| ())?;
    let etag =
        HeaderValue::from_str(&format!("\"{}\"", rendition.rendition_checksum)).map_err(|_| ())?;
    let mut response = StatusCode::FOUND.into_response();
    let headers = response.headers_mut();
    headers.insert("location", location);
    // Immutable identifiers make this redirect safely cacheable, but no
    // mutable API response or protected asset inherits that exception.
    headers.insert(
        "cache-control",
        HeaderValue::from_static("public, max-age=31536000, immutable"),
    );
    headers.insert("etag", etag);
    headers.insert(
        "x-content-type-options",
        HeaderValue::from_static("nosniff"),
    );
    headers.insert("referrer-policy", HeaderValue::from_static("no-referrer"));
    Ok(response)
}

fn concealed() -> Response {
    StatusCode::NOT_FOUND.into_response()
}

fn unavailable() -> Response {
    StatusCode::SERVICE_UNAVAILABLE.into_response()
}

fn cookie(headers: &HeaderMap) -> Option<String> {
    let values = headers
        .get_all(COOKIE)
        .iter()
        .map(|value| value.to_str().ok())
        .collect::<Option<Vec<_>>>()?;
    (!values.is_empty()).then(|| values.join("; "))
}

#[cfg(test)]
mod tests {
    use objects::Sha256Checksum;
    use question_model::{ObjectId, QuestionId, QuestionRevisionNumber, QuestionRevisionReference};

    use super::*;

    fn rendition() -> ReadyQuestionAssetDelivery {
        ReadyQuestionAssetDelivery {
            question_revision: QuestionRevisionReference {
                question_id: "ABC-DEF1".parse::<QuestionId>().expect("Question ID"),
                revision_number: QuestionRevisionNumber::new(1).expect("revision"),
            },
            asset_id: QuestionAssetId::from_uuid(Uuid::from_u128(2)),
            public_object_id: ObjectId::from_uuid(Uuid::from_u128(3)),
            rendition_checksum: Sha256Checksum::from_bytes([4; 32]),
        }
    }

    #[test]
    fn public_asset_redirect_uses_only_configured_base_and_typed_path() {
        let base =
            public_asset_base_url("https://assets.example.test/public-assets").expect("valid base");
        let response = redirect_response(&base, &rendition()).expect("redirect");
        assert_eq!(response.status(), StatusCode::FOUND);
        assert_eq!(
            response.headers()["location"],
            "https://assets.example.test/public-assets/questions/ABC-DEF1/versions/1/assets/00000000-0000-0000-0000-000000000002/00000000-0000-0000-0000-000000000003"
        );
        assert_eq!(
            response.headers()["cache-control"],
            "public, max-age=31536000, immutable"
        );
        assert_eq!(response.headers()["referrer-policy"], "no-referrer");
        assert_eq!(response.headers()["x-content-type-options"], "nosniff");
    }

    #[test]
    fn public_asset_base_rejects_credentials_and_non_delivery_components() {
        for value in [
            "https://user@example.test/public-assets",
            "https://example.test/public-assets?next=https://other.test",
            "https://example.test/public-assets#fragment",
            "file:///tmp/public-assets",
        ] {
            assert!(public_asset_base_url(value).is_err(), "{value}");
        }
    }
}
