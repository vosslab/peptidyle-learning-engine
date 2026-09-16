//! Authenticated, self-only Account avatar routes.

use std::{
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use axum::{
    Json, Router,
    body::to_bytes,
    extract::{Path, Request, State},
    http::{HeaderMap, StatusCode, header::COOKIE},
    response::{IntoResponse, Response},
    routing::get,
};
use learning_data_access::{
    AccountAvatar, AccountAvatarGallery, AccountProfileImageDeleteWork, SelectableProvidedAvatarId,
    SessionTokenHash, StoreError,
    postgres::{PostgresAccountAvatarGallery, PostgresSessionStore},
};
use objects::{
    ObjectAddress, ObjectStore, PutObject, Sha256Checksum,
    image_validation::{MAX_STILL_IMAGE_BYTES, normalized_still_image_webp, verify_still_image},
    s3::S3ObjectStore,
};
use question_model::{ObjectId, ProductRole, ProfileImageReference, Timestamp};
use uuid::Uuid;

use crate::auth::{AuthError, resolve_session};

const MAX_PROFILE_IMAGE_BYTES: usize = 2 * 1024 * 1024;
const PROFILE_IMAGE_SIDE_PIXELS: u32 = 256;

#[derive(Clone)]
struct RouteState {
    sessions: Arc<PostgresSessionStore>,
    avatars: PostgresAccountAvatarGallery,
    objects: S3ObjectStore,
}

/// Builds the authenticated Account's role-neutral avatar route surface.
pub fn profile_avatar_router(
    sessions: Arc<PostgresSessionStore>,
    avatars: PostgresAccountAvatarGallery,
    objects: S3ObjectStore,
) -> Router {
    Router::new()
        .route(
            "/api/profile/avatar",
            get(read_avatar).put(select_provided_avatar),
        )
        .route(
            "/api/profile/avatar/profile-image",
            axum::routing::post(replace_profile_image),
        )
        .route(
            "/api/profile/avatar/profile-images/{image}/delivery",
            axum::routing::post(deliver_profile_image),
        )
        .with_state(RouteState {
            sessions,
            avatars,
            objects,
        })
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct CurrentAvatar {
    avatar: Option<AvatarChoice>,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
enum AvatarChoice {
    Provided {
        #[serde(rename = "providedAvatarId")]
        provided_avatar_id: String,
    },
    ProfileImage {
        #[serde(rename = "profileImageId")]
        profile_image_id: ProfileImageReference,
    },
}

impl From<AccountAvatar> for AvatarChoice {
    fn from(value: AccountAvatar) -> Self {
        match value {
            AccountAvatar::Provided(id) => Self::Provided {
                provided_avatar_id: id.to_string(),
            },
            AccountAvatar::ProfileImage(reference) => Self::ProfileImage {
                profile_image_id: reference,
            },
        }
    }
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SelectProvidedAvatar {
    provided_avatar_id: String,
}

async fn read_avatar(State(state): State<RouteState>, headers: HeaderMap) -> Response {
    let session = match self_session(&state, &headers).await {
        Ok(session) => session,
        Err(response) => return *response,
    };
    match state
        .avatars
        .read_current_account_avatar(session.token)
        .await
    {
        Ok(avatar) => crate::auth::no_store(
            Json(CurrentAvatar {
                avatar: avatar.map(Into::into),
            })
            .into_response(),
        ),
        Err(error) => store_error_response(error),
    }
}

async fn select_provided_avatar(State(state): State<RouteState>, request: Request) -> Response {
    let session = match self_session(&state, request.headers()).await {
        Ok(session) => session,
        Err(response) => return *response,
    };
    if !has_content_type(request.headers(), "application/json") {
        return route_error(
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            "Profile avatar is invalid",
        );
    }
    let input = match to_bytes(request.into_body(), 256).await {
        Ok(bytes) => match serde_json::from_slice::<SelectProvidedAvatar>(&bytes) {
            Ok(input) => input,
            Err(_) => {
                return route_error(
                    StatusCode::UNPROCESSABLE_ENTITY,
                    "Profile avatar is invalid",
                );
            }
        },
        Err(_) => return route_error(StatusCode::PAYLOAD_TOO_LARGE, "Profile avatar is too large"),
    };
    let avatar_id = match SelectableProvidedAvatarId::parse(input.provided_avatar_id) {
        Ok(id) => id,
        Err(_) => {
            return route_error(
                StatusCode::UNPROCESSABLE_ENTITY,
                "Profile avatar is invalid",
            );
        }
    };
    match state
        .avatars
        .select_provided_account_avatar(session.token, avatar_id)
        .await
    {
        Ok(()) => crate::auth::no_store(StatusCode::NO_CONTENT.into_response()),
        Err(error) => store_error_response(error),
    }
}

async fn replace_profile_image(State(state): State<RouteState>, request: Request) -> Response {
    let session = match self_session(&state, request.headers()).await {
        Ok(session) => session,
        Err(response) => return *response,
    };
    if !matches!(
        session.role,
        ProductRole::Instructor | ProductRole::Sysadmin
    ) {
        return route_error(
            StatusCode::FORBIDDEN,
            "Profile image upload is not available",
        );
    }
    if !has_content_type(request.headers(), "application/octet-stream") {
        return route_error(
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            "Profile image is invalid",
        );
    }
    let bytes = match to_bytes(request.into_body(), MAX_STILL_IMAGE_BYTES).await {
        Ok(bytes) => bytes.to_vec(),
        Err(_) => return route_error(StatusCode::PAYLOAD_TOO_LARGE, "Profile image is too large"),
    };
    if verify_still_image(&bytes).is_err() {
        return route_error(StatusCode::UNPROCESSABLE_ENTITY, "Profile image is invalid");
    }
    let image = match normalized_still_image_webp(
        &bytes,
        PROFILE_IMAGE_SIDE_PIXELS,
        PROFILE_IMAGE_SIDE_PIXELS,
    ) {
        Ok(image) if !image.is_empty() && image.len() <= MAX_PROFILE_IMAGE_BYTES => image,
        _ => return route_error(StatusCode::UNPROCESSABLE_ENTITY, "Profile image is invalid"),
    };
    let reference = ProfileImageReference::generate();
    let object = ObjectId::from_uuid(Uuid::now_v7());
    let address = ObjectAddress::ProfileImage {
        image: reference,
        object,
    };
    let checksum = Sha256Checksum::compute(&image);
    let length = u64::try_from(image.len()).unwrap_or(u64::MAX);
    let prepared = match state
        .avatars
        .prepare_account_profile_image(session.token, reference, object, checksum, length)
        .await
    {
        Ok(prepared) => prepared,
        Err(error) => return store_error_response(error),
    };
    let record = state
        .objects
        .put(PutObject {
            address: address.clone(),
            bytes: image,
            media_type: "image/webp".to_owned(),
            created_at: now(),
        })
        .await;
    let valid = matches!(&record, Ok(record) if record.id == prepared.object_id
        && record.address == address && record.sha256 == checksum
        && record.media_type == "image/webp" && record.size_bytes == length);
    if !valid {
        let _ = state
            .avatars
            .require_account_profile_image_repair(session.token, prepared.work_id)
            .await;
        return route_error(StatusCode::SERVICE_UNAVAILABLE, "Profile image unavailable");
    }
    if state
        .avatars
        .complete_account_profile_image_put(session.token, prepared.work_id)
        .await
        .is_err()
    {
        let _ = state
            .avatars
            .require_account_profile_image_repair(session.token, prepared.work_id)
            .await;
        return route_error(StatusCode::SERVICE_UNAVAILABLE, "Profile image unavailable");
    }
    match state
        .avatars
        .finalize_account_profile_image(session.token, prepared.work_id)
        .await
    {
        Ok(finalized) => {
            if let Some(retired) = finalized.retired {
                cleanup_profile_image(&state, session.token, retired).await;
            }
            crate::auth::no_store(
                Json(CurrentAvatar {
                    avatar: Some(AvatarChoice::ProfileImage {
                        profile_image_id: finalized.reference,
                    }),
                })
                .into_response(),
            )
        }
        Err(error) => {
            match state
                .avatars
                .prepare_account_profile_image_deletion(session.token, prepared.work_id)
                .await
            {
                Ok(work) => cleanup_profile_image(&state, session.token, work).await,
                Err(_) => {
                    let _ = state
                        .avatars
                        .require_account_profile_image_repair(session.token, prepared.work_id)
                        .await;
                }
            }
            store_error_response(error)
        }
    }
}

async fn cleanup_profile_image(
    state: &RouteState,
    token: SessionTokenHash,
    work: AccountProfileImageDeleteWork,
) {
    let address = ObjectAddress::ProfileImage {
        image: work.reference,
        object: work.object_id,
    };
    match state.objects.delete(&address).await {
        Ok(()) => {
            if state
                .avatars
                .complete_account_profile_image_deletion(token, work)
                .await
                .is_err()
            {
                let _ = state
                    .avatars
                    .require_account_profile_image_deletion_repair(token, work)
                    .await;
            }
        }
        Err(_) => {
            if state
                .avatars
                .require_account_profile_image_deletion_repair(token, work)
                .await
                .is_err()
            {
                return;
            }
            match state.objects.get(&address).await {
                Err(objects::ObjectStoreError::NotFound) => {
                    let _ = state
                        .avatars
                        .record_account_profile_image_cleanup_check(token, work, false, None)
                        .await;
                }
                Ok(stored) => {
                    let _ = state
                        .avatars
                        .record_account_profile_image_cleanup_check(
                            token,
                            work,
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

async fn deliver_profile_image(
    State(state): State<RouteState>,
    Path(image): Path<String>,
    headers: HeaderMap,
) -> Response {
    let reference = match Uuid::parse_str(&image) {
        Ok(value) => ProfileImageReference::from_uuid(value),
        Err(_) => return concealed(),
    };
    let session = match self_session(&state, &headers).await {
        Ok(session) => session,
        Err(response) => return *response,
    };
    if !matches!(
        session.role,
        ProductRole::Instructor | ProductRole::Sysadmin
    ) {
        return concealed();
    }
    let object = match state
        .avatars
        .resolve_current_account_profile_image(session.token, reference)
        .await
    {
        Ok(object) => object,
        Err(StoreError::NotFound | StoreError::Forbidden | StoreError::OwnershipMismatch) => {
            return concealed();
        }
        Err(error) => return store_error_response(error),
    };
    let address = ObjectAddress::ProfileImage {
        image: reference,
        object,
    };
    let stored = match state.objects.get(&address).await {
        Ok(value)
            if value.record.media_type == "image/webp"
                && !value.bytes.is_empty()
                && value.bytes.len() <= MAX_PROFILE_IMAGE_BYTES =>
        {
            value
        }
        _ => return route_error(StatusCode::SERVICE_UNAVAILABLE, "Profile image unavailable"),
    };
    let size = stored.bytes.len();
    let mut response = stored.bytes.into_response();
    let headers = response.headers_mut();
    headers.insert(
        axum::http::header::CONTENT_TYPE,
        "image/webp".parse().unwrap(),
    );
    headers.insert(
        axum::http::header::CONTENT_DISPOSITION,
        "attachment; filename=\"ple-profile-image.webp\""
            .parse()
            .unwrap(),
    );
    headers.insert(
        axum::http::header::CONTENT_LENGTH,
        size.to_string().parse().unwrap(),
    );
    headers.insert(
        axum::http::header::X_CONTENT_TYPE_OPTIONS,
        "nosniff".parse().unwrap(),
    );
    headers.insert(
        "cross-origin-resource-policy",
        "same-origin".parse().unwrap(),
    );
    headers.insert(
        axum::http::header::REFERRER_POLICY,
        "no-referrer".parse().unwrap(),
    );
    crate::auth::no_store(response)
}

struct SelfSession {
    token: SessionTokenHash,
    role: ProductRole,
}

async fn self_session(
    state: &RouteState,
    headers: &HeaderMap,
) -> Result<SelfSession, Box<Response>> {
    match resolve_session(
        state.sessions.as_ref(),
        joined_cookie_header(headers).as_deref(),
    )
    .await
    {
        Ok(session) => Ok(SelfSession {
            token: session.session_hash,
            role: session.record.product_role,
        }),
        Err(AuthError::Unauthenticated) => Err(Box::new(concealed())),
        Err(AuthError::Unavailable(_) | AuthError::Randomness(_)) => Err(Box::new(route_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "Profile authentication unavailable",
        ))),
    }
}

fn now() -> Timestamp {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| value.as_millis())
        .unwrap_or_default();
    Timestamp::from_unix_millis(i64::try_from(millis).unwrap_or(i64::MAX))
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
            "Profile avatar is invalid",
        ),
        StoreError::Conflict | StoreError::RetryableTransaction => {
            route_error(StatusCode::PRECONDITION_FAILED, "Profile avatar changed")
        }
        StoreError::LifecycleConflict => {
            route_error(StatusCode::CONFLICT, "Profile avatar lifecycle conflict")
        }
        StoreError::AlreadyExists => route_error(StatusCode::CONFLICT, "Profile avatar conflict"),
        StoreError::AssessmentActivity(_)
        | StoreError::TimedOut
        | StoreError::LeaseLost
        | StoreError::Unavailable(_) => route_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "Profile avatar unavailable",
        ),
    }
}

fn concealed() -> Response {
    route_error(StatusCode::NOT_FOUND, "Profile avatar not found")
}
fn route_error(status: StatusCode, message: &'static str) -> Response {
    crate::auth::no_store((status, message).into_response())
}
