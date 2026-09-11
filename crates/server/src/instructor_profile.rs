//! Authenticated Instructor self-profile routes.

use std::sync::Arc;

use axum::{
    Json, Router,
    body::to_bytes,
    extract::{Request, State},
    http::{HeaderMap, StatusCode, header::COOKIE},
    response::{IntoResponse, Response},
    routing::get,
};
use learning_data_access::{
    InstructorProfileStore, ProfileThumbnailDeleteWork, ProfileThumbnailStore, SessionTokenHash,
    StoreError, UpdateInstructorProfileInput,
    postgres::{
        PostgresInstructorProfileStore, PostgresProfileThumbnailStore, PostgresSessionStore,
    },
};
use objects::{
    ObjectAddress, ObjectStore, PutObject, Sha256Checksum,
    image_validation::{MAX_STILL_IMAGE_BYTES, normalized_still_image_webp, verify_still_image},
    s3::S3ObjectStore,
};
use question_model::{
    ProductRole, ProfileThumbnailReference, ProfileThumbnailRendition, Timestamp,
};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

use crate::auth::{AuthError, resolve_session};

const MAX_PROFILE_UPDATE_BYTES: usize = 512;
const MAX_PROFILE_THUMBNAIL_BYTES: usize = 2 * 1024 * 1024;

#[derive(Clone)]
struct RouteState {
    sessions: Arc<PostgresSessionStore>,
    profiles: PostgresInstructorProfileStore,
    thumbnails: PostgresProfileThumbnailStore,
    objects: S3ObjectStore,
}

/// Builds authenticated routes for an Instructor's own profile preferences and thumbnail operations.
pub fn instructor_profile_router(
    sessions: Arc<PostgresSessionStore>,
    profiles: PostgresInstructorProfileStore,
    thumbnails: PostgresProfileThumbnailStore,
    objects: S3ObjectStore,
) -> Router {
    Router::new()
        .route(
            "/api/instructor-profile",
            get(read_profile).patch(update_profile),
        )
        .route(
            "/api/instructor-profile/thumbnail",
            get(read_thumbnail).post(replace_thumbnail),
        )
        .route(
            "/api/instructor-profile/thumbnails/{thumbnail}/delivery",
            axum::routing::post(deliver_thumbnail),
        )
        .with_state(RouteState {
            sessions,
            profiles,
            thumbnails,
            objects,
        })
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct CurrentThumbnail {
    reference: Option<ProfileThumbnailReference>,
}

async fn read_thumbnail(State(state): State<RouteState>, headers: HeaderMap) -> Response {
    let token = match instructor_session_hash(&state, &headers).await {
        Ok(token) => token,
        Err(response) => return *response,
    };
    match state.thumbnails.read_current_profile_thumbnail(token).await {
        Ok(reference) => {
            crate::auth::no_store(Json(CurrentThumbnail { reference }).into_response())
        }
        Err(error) => store_error_response(error),
    }
}

async fn replace_thumbnail(State(state): State<RouteState>, request: Request) -> Response {
    let token = match instructor_session_hash(&state, request.headers()).await {
        Ok(token) => token,
        Err(response) => return *response,
    };
    if !has_content_type(request.headers(), "application/octet-stream") {
        return route_error(
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            "Profile thumbnail is invalid",
        );
    }
    let bytes = match to_bytes(request.into_body(), MAX_STILL_IMAGE_BYTES).await {
        Ok(bytes) => bytes.to_vec(),
        Err(_) => {
            return route_error(
                StatusCode::PAYLOAD_TOO_LARGE,
                "Profile thumbnail is too large",
            );
        }
    };
    if verify_still_image(&bytes).is_err() {
        return route_error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Profile thumbnail is invalid",
        );
    }
    let (width, height) = ProfileThumbnailRendition::Square.dimensions();
    let rendition = match normalized_still_image_webp(&bytes, width, height) {
        Ok(value) => value,
        Err(_) => {
            return route_error(
                StatusCode::UNPROCESSABLE_ENTITY,
                "Profile thumbnail is invalid",
            );
        }
    };
    if rendition.is_empty() || rendition.len() > MAX_PROFILE_THUMBNAIL_BYTES {
        return route_error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Profile thumbnail is invalid",
        );
    }
    let reference = ProfileThumbnailReference::generate();
    let address = ObjectAddress::ProfileThumbnail {
        thumbnail: reference,
    };
    let checksum = Sha256Checksum::compute(&rendition);
    let rendition_length = u64::try_from(rendition.len()).unwrap_or(u64::MAX);
    let prepared = match state
        .thumbnails
        .prepare_profile_thumbnail(
            token,
            reference,
            address.object_id(),
            checksum,
            rendition_length,
        )
        .await
    {
        Ok(value) => value,
        Err(error) => return store_error_response(error),
    };
    let record = state
        .objects
        .put(PutObject {
            address: address.clone(),
            bytes: rendition,
            media_type: "image/webp".to_string(),
            created_at: now(),
        })
        .await;
    let valid = matches!(&record, Ok(record) if record.id == prepared.object_id
        && record.address == address
        && record.sha256 == checksum
        && record.media_type == "image/webp"
        && record.size_bytes == rendition_length);
    if !valid {
        let _ = state
            .thumbnails
            .require_profile_thumbnail_repair(token, prepared.work_id)
            .await;
        return route_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "Profile thumbnail unavailable",
        );
    }
    if state
        .thumbnails
        .complete_profile_thumbnail_put(token, prepared.work_id)
        .await
        .is_err()
    {
        let _ = state
            .thumbnails
            .require_profile_thumbnail_repair(token, prepared.work_id)
            .await;
        return route_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "Profile thumbnail unavailable",
        );
    }
    match state
        .thumbnails
        .finalize_profile_thumbnail(token, prepared.work_id)
        .await
    {
        Ok(finalized) => {
            if let Some(retired) = finalized.retired {
                cleanup_thumbnail(&state, token, retired).await;
            }
            crate::auth::no_store(
                Json(CurrentThumbnail {
                    reference: Some(finalized.reference),
                })
                .into_response(),
            )
        }
        Err(error) => {
            match state
                .thumbnails
                .prepare_profile_thumbnail_deletion(token, prepared.work_id)
                .await
            {
                Ok(delete_work) => cleanup_thumbnail(&state, token, delete_work).await,
                Err(_) => {
                    let _ = state
                        .thumbnails
                        .require_profile_thumbnail_repair(token, prepared.work_id)
                        .await;
                }
            }
            store_error_response(error)
        }
    }
}

async fn cleanup_thumbnail(
    state: &RouteState,
    token: SessionTokenHash,
    delete_work: ProfileThumbnailDeleteWork,
) {
    let address = ObjectAddress::ProfileThumbnail {
        thumbnail: delete_work.reference,
    };
    match state.objects.delete(&address).await {
        Ok(()) => {
            if state
                .thumbnails
                .complete_profile_thumbnail_deletion(token, delete_work)
                .await
                .is_err()
            {
                let _ = state
                    .thumbnails
                    .require_profile_thumbnail_deletion_repair(token, delete_work)
                    .await;
            }
        }
        Err(_) => {
            if state
                .thumbnails
                .require_profile_thumbnail_deletion_repair(token, delete_work)
                .await
                .is_err()
            {
                return;
            }
            match state.objects.get(&address).await {
                Err(objects::ObjectStoreError::NotFound) => {
                    let _ = state
                        .thumbnails
                        .record_profile_thumbnail_cleanup_check(token, delete_work, false, None)
                        .await;
                }
                Ok(stored) => {
                    let _ = state
                        .thumbnails
                        .record_profile_thumbnail_cleanup_check(
                            token,
                            delete_work,
                            true,
                            Some(stored.record.sha256),
                        )
                        .await;
                }
                Err(_) => {}
            }
        }
    }
}

async fn deliver_thumbnail(
    State(state): State<RouteState>,
    axum::extract::Path(thumbnail): axum::extract::Path<String>,
    headers: HeaderMap,
) -> Response {
    let reference = match Uuid::parse_str(&thumbnail) {
        Ok(value) => ProfileThumbnailReference::from_uuid(value),
        Err(_) => return concealed(),
    };
    let token = match instructor_session_hash(&state, &headers).await {
        Ok(token) => token,
        Err(response) => return *response,
    };
    let object_id = match state
        .thumbnails
        .resolve_current_profile_thumbnail(token, reference)
        .await
    {
        Ok(value) => value,
        Err(StoreError::NotFound | StoreError::Forbidden | StoreError::OwnershipMismatch) => {
            return concealed();
        }
        Err(error) => return store_error_response(error),
    };
    let address = ObjectAddress::ProfileThumbnail {
        thumbnail: reference,
    };
    if address.object_id() != object_id {
        return concealed();
    }
    let object = match state.objects.get(&address).await {
        Ok(value)
            if value.record.media_type == "image/webp"
                && value.bytes.len() <= MAX_PROFILE_THUMBNAIL_BYTES
                && !value.bytes.is_empty() =>
        {
            value
        }
        _ => {
            return route_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "Profile thumbnail unavailable",
            );
        }
    };
    let size = object.bytes.len();
    let mut response = object.bytes.into_response();
    let response_headers = response.headers_mut();
    response_headers.insert(
        axum::http::header::CONTENT_TYPE,
        "image/webp".parse().unwrap(),
    );
    response_headers.insert(
        axum::http::header::CONTENT_DISPOSITION,
        "attachment; filename=\"ple-profile-thumbnail.webp\""
            .parse()
            .unwrap(),
    );
    response_headers.insert(
        axum::http::header::CONTENT_LENGTH,
        size.to_string().parse().unwrap(),
    );
    response_headers.insert(
        axum::http::header::X_CONTENT_TYPE_OPTIONS,
        "nosniff".parse().unwrap(),
    );
    response_headers.insert(
        "cross-origin-resource-policy",
        "same-origin".parse().unwrap(),
    );
    response_headers.insert(
        axum::http::header::REFERRER_POLICY,
        "no-referrer".parse().unwrap(),
    );
    crate::auth::no_store(response)
}

fn now() -> Timestamp {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| value.as_millis())
        .unwrap_or_default();
    Timestamp::from_unix_millis(i64::try_from(millis).unwrap_or(i64::MAX))
}

async fn read_profile(State(state): State<RouteState>, headers: HeaderMap) -> Response {
    let token = match instructor_session_hash(&state, &headers).await {
        Ok(token) => token,
        Err(response) => return *response,
    };
    match state.profiles.read_instructor_profile(token).await {
        Ok(profile) => crate::auth::no_store(Json(profile).into_response()),
        Err(error) => store_error_response(error),
    }
}

async fn update_profile(State(state): State<RouteState>, request: Request) -> Response {
    let token = match instructor_session_hash(&state, request.headers()).await {
        Ok(token) => token,
        Err(response) => return *response,
    };
    if !has_content_type(request.headers(), "application/json") {
        return route_error(
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            "Instructor Profile is invalid",
        );
    }
    let input = match to_bytes(request.into_body(), MAX_PROFILE_UPDATE_BYTES).await {
        Ok(bytes) => match serde_json::from_slice::<UpdateInstructorProfileInput>(&bytes) {
            Ok(input) => input,
            Err(_) => {
                return route_error(
                    StatusCode::UNPROCESSABLE_ENTITY,
                    "Instructor Profile is invalid",
                );
            }
        },
        Err(_) => {
            return route_error(
                StatusCode::PAYLOAD_TOO_LARGE,
                "Instructor Profile is too large",
            );
        }
    };
    match state.profiles.update_instructor_profile(token, input).await {
        Ok(profile) => crate::auth::no_store(Json(profile).into_response()),
        Err(error) => store_error_response(error),
    }
}

async fn instructor_session_hash(
    state: &RouteState,
    headers: &HeaderMap,
) -> Result<SessionTokenHash, Box<Response>> {
    match resolve_session(
        state.sessions.as_ref(),
        joined_cookie_header(headers).as_deref(),
    )
    .await
    {
        Ok(session) if session.record.product_role == ProductRole::Instructor => {
            Ok(session.session_hash)
        }
        Ok(_) | Err(AuthError::Unauthenticated) => Err(Box::new(concealed())),
        Err(AuthError::Unavailable(_) | AuthError::Randomness(_)) => Err(Box::new(route_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "Instructor Profile authentication unavailable",
        ))),
    }
}

fn has_content_type(headers: &HeaderMap, expected: &str) -> bool {
    headers
        .get("content-type")
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| {
            value
                .split(';')
                .next()
                .is_some_and(|media_type| media_type.trim().eq_ignore_ascii_case(expected))
        })
}

fn joined_cookie_header(headers: &HeaderMap) -> Option<String> {
    let values = headers
        .get_all(COOKIE)
        .iter()
        .map(|value| value.to_str().ok())
        .collect::<Option<Vec<_>>>()?;
    (!values.is_empty()).then(|| values.join("; "))
}

fn store_error_response(error: StoreError) -> Response {
    match error {
        StoreError::NotFound | StoreError::Forbidden | StoreError::OwnershipMismatch => concealed(),
        StoreError::InvalidRecord(_) => route_error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Instructor Profile is invalid",
        ),
        StoreError::Conflict | StoreError::RetryableTransaction => route_error(
            StatusCode::PRECONDITION_FAILED,
            "Instructor Profile changed",
        ),
        StoreError::AlreadyExists => {
            route_error(StatusCode::CONFLICT, "Instructor Profile conflict")
        }
        StoreError::AssignmentActivity(_) | StoreError::TimedOut | StoreError::Unavailable(_) => {
            route_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "Instructor Profile unavailable",
            )
        }
    }
}

fn concealed() -> Response {
    route_error(StatusCode::NOT_FOUND, "Instructor Profile not found")
}
fn route_error(status: StatusCode, message: &'static str) -> Response {
    crate::auth::no_store((status, message).into_response())
}
