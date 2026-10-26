//! Authorized redirect route for immutable published Question Image renditions.
//!
//! The route is deliberately a metadata boundary: PostgreSQL proves the
//! current authority and exact ready rendition, then storage signs a twelve-hour
//! image URL from typed identity. No issued-link records are retained.

use std::{sync::Arc, time::Duration};

use axum::{
    Router,
    extract::{Path, State},
    http::{HeaderMap, HeaderValue, StatusCode, header::COOKIE},
    response::{IntoResponse, Response},
    routing::get,
};
use learning_data_access::{
    QuestionImageDeliveryResolution, QuestionImageDeliveryStore, ReadyQuestionImageDelivery,
    SessionTokenHash, StoreError,
    postgres::{PostgresQuestionImageDeliveryStore, PostgresSessionStore},
};
use objects::{ObjectAddress, ObjectStore, s3::S3ObjectStore};
use question_model::{
    PublishedQuestionId, PublishedQuestionRevisionTuple, QuestionImageAssetId,
    QuestionRevisionNumber, UserRole,
};
use url::Url;
use uuid::Uuid;

use crate::auth::{AuthError, resolve_session};

const PENDING_IMAGE_WAIT_BOUND: Duration =
    crate::public_asset_publisher::PUBLICATION_OPERATION_BOUND;
const PENDING_IMAGE_POLL_INTERVAL: Duration = Duration::from_millis(250);

#[derive(Clone)]
struct QuestionImageDeliveryRouteState {
    sessions: Arc<PostgresSessionStore>,
    store: PostgresQuestionImageDeliveryStore,
    objects: S3ObjectStore,
}

/// Registers the only public-asset GET route. The configured base is
/// deployment-owned; a browser never supplies a bucket, key, or hostname.
pub fn question_image_delivery_router(
    sessions: Arc<PostgresSessionStore>,
    store: PostgresQuestionImageDeliveryStore,
    objects: S3ObjectStore,
) -> Router {
    Router::new()
        .route(
            "/api/questions/{question_id}/revisions/{revision_number}/images/{question_image_asset_id}",
            get(get_public_question_image),
        )
        .with_state(QuestionImageDeliveryRouteState {
            sessions,
            store,
            objects,
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

async fn get_public_question_image(
    State(state): State<QuestionImageDeliveryRouteState>,
    headers: HeaderMap,
    Path((question_id, revision_number, question_image_asset_id)): Path<(String, String, String)>,
) -> Response {
    let published_question_revision_tuple =
        match verified_published_question_revision_tuple(&question_id, &revision_number) {
            Some(value) => value,
            None => return concealed(),
        };
    let question_image_asset_id = match Uuid::parse_str(&question_image_asset_id) {
        Ok(value) => QuestionImageAssetId::from_uuid(value),
        Err(_) => return concealed(),
    };
    let rendition = match tokio::time::timeout(
        PENDING_IMAGE_WAIT_BOUND,
        wait_for_question_image_delivery(
            &state,
            &headers,
            published_question_revision_tuple,
            question_image_asset_id,
        ),
    )
    .await
    {
        Ok(Ok(value)) => value,
        Ok(Err(response)) => return *response,
        Err(_) => return unavailable(),
    };
    let address = ObjectAddress::QuestionImage {
        published_question_revision_tuple: rendition.published_question_revision_tuple,
        question_image_asset_id: rendition.question_image_asset_id,
        object_id: rendition.public_object_id,
    };
    let now = crate::authoring::now();
    // ASVS 8.2.2: only the rendition authorized above can receive a bearer URL.
    match state.objects.signed_url(&address, now).await {
        Ok(signed) => redirect_response(&signed.url).unwrap_or_else(|()| unavailable()),
        Err(_) => unavailable(),
    }
}

async fn wait_for_question_image_delivery(
    state: &QuestionImageDeliveryRouteState,
    headers: &HeaderMap,
    published_question_revision_tuple: PublishedQuestionRevisionTuple,
    question_image_asset_id: QuestionImageAssetId,
) -> Result<ReadyQuestionImageDelivery, Box<Response>> {
    let session_hash = asset_session_hash(state, headers).await?;
    poll_question_image_delivery(
        &state.store,
        session_hash,
        published_question_revision_tuple,
        question_image_asset_id,
    )
    .await
}

async fn poll_question_image_delivery<S>(
    store: &S,
    session_hash: SessionTokenHash,
    published_question_revision_tuple: PublishedQuestionRevisionTuple,
    question_image_asset_id: QuestionImageAssetId,
) -> Result<ReadyQuestionImageDelivery, Box<Response>>
where
    S: QuestionImageDeliveryStore + ?Sized,
{
    loop {
        match store
            .resolve_question_image_delivery(
                session_hash,
                published_question_revision_tuple.clone(),
                question_image_asset_id,
            )
            .await
        {
            Ok(QuestionImageDeliveryResolution::Ready(rendition)) => return Ok(rendition),
            // The PostgreSQL store commits before it yields Pending, so this
            // sleep never retains the transaction or its pooled connection.
            Ok(QuestionImageDeliveryResolution::Pending) => {
                tokio::time::sleep(PENDING_IMAGE_POLL_INTERVAL).await;
            }
            Err(StoreError::NotFound | StoreError::Forbidden | StoreError::OwnershipMismatch) => {
                return Err(Box::new(concealed()));
            }
            Err(_) => return Err(Box::new(unavailable())),
        }
    }
}

/// Parses the browser route's complete immutable Question Revision identity.
///
/// The shared model parses the exact checksum-bearing ID before this
/// authorization-sensitive Store lookup (ASVS 2.2.1 and 2.2.2). Invalid and
/// unauthorized Tuples share the opaque response below.
fn verified_published_question_revision_tuple(
    question_id: &str,
    revision_number: &str,
) -> Option<PublishedQuestionRevisionTuple> {
    let question_id = question_id.parse::<PublishedQuestionId>().ok()?;
    let revision_value = revision_number.parse::<u32>().ok()?;
    if revision_value.to_string() != revision_number {
        return None;
    }
    Some(PublishedQuestionRevisionTuple {
        published_question_id: question_id,
        revision_number: QuestionRevisionNumber::new(revision_value).ok()?,
    })
}

async fn asset_session_hash(
    state: &QuestionImageDeliveryRouteState,
    headers: &HeaderMap,
) -> Result<SessionTokenHash, Box<Response>> {
    match resolve_session(state.sessions.as_ref(), cookie(headers).as_deref()).await {
        Ok(value)
            if matches!(
                value.record.user_role,
                UserRole::Instructor | UserRole::Student | UserRole::Sysadmin
            ) =>
        {
            Ok(value.session_hash)
        }
        Ok(_) | Err(AuthError::Unauthenticated) => Err(Box::new(concealed())),
        Err(AuthError::Unavailable(_) | AuthError::Randomness(_)) => Err(Box::new(unavailable())),
    }
}

fn redirect_response(location: &str) -> Result<Response, ()> {
    let location = HeaderValue::from_str(location).map_err(|_| ())?;
    let mut response = StatusCode::FOUND.into_response();
    let headers = response.headers_mut();
    headers.insert("location", location);
    headers.insert("cache-control", HeaderValue::from_static("no-store"));
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
    use super::*;

    #[test]
    fn signed_image_redirect_is_not_cached() {
        let location = "https://example.test/public-assets/questions/image?X-Amz-Signature=example";
        let response = redirect_response(location).unwrap();
        assert_eq!(response.status(), StatusCode::FOUND);
        assert_eq!(response.headers()["location"], location);
        assert_eq!(response.headers()["cache-control"], "no-store");
        assert!(!response.headers().contains_key("etag"));
    }

    #[tokio::test]
    async fn published_image_links_expire_after_twelve_hours() {
        use objects::{PutObject, memory::MemoryObjectStore};
        use question_model::{ObjectId, Timestamp};
        let store = MemoryObjectStore::default();
        let now = Timestamp::from_unix_millis(1_700_000_000_000);
        let address = ObjectAddress::QuestionImage {
            published_question_revision_tuple: PublishedQuestionRevisionTuple {
                published_question_id: PublishedQuestionId::from_random_identifier("1234567")
                    .unwrap(),
                revision_number: QuestionRevisionNumber::new(1).unwrap(),
            },
            question_image_asset_id: QuestionImageAssetId::from_uuid(Uuid::from_u128(1)),
            object_id: ObjectId::from_uuid(Uuid::from_u128(2)),
        };
        store
            .put(PutObject {
                address: address.clone(),
                bytes: vec![1],
                media_type: "image/png".to_owned(),
                created_at: now,
            })
            .await
            .unwrap();
        let signed = store.signed_url(&address, now).await.unwrap();
        assert_eq!(
            signed.expires_at.as_unix_millis() - now.as_unix_millis(),
            12 * 60 * 60 * 1_000
        );
    }
}
