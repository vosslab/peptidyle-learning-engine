//! Private original-image registration for native flat-question authoring.
//!
//! This route deliberately accepts image *bytes*, never an object path or a
//! browser-generated identity.  It proves author/workspace access before it
//! consumes the body, then stores the original immutable bytes under a typed
//! private key and registers only server-verified facts.

use std::sync::Arc;

use axum::body::to_bytes;
use axum::extract::{DefaultBodyLimit, Path, Request, State};
use axum::http::{HeaderMap, StatusCode};
use axum::middleware;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router};
use learning_data_access::{
    AuthoritativeTimeStore, FlatQuestionAssetStore, SessionStore, Store, StoreError,
    WorkspaceFlatQuestionAsset,
};
use objects::{ObjectKey, ObjectStore, ObjectStoreError, PutObject, Sha256Digest};
use question_model::{AssetId, ObjectId, UserRole, WorkspaceId};
use serde::Serialize;

use crate::auth::{auth_error_response, no_store, resolve_request_session};
use crate::hotspot_image::verify_hotspot_image;
use crate::http_refusal::HttpResult;

/// Native original images remain modest enough for an instructor browser and
/// are separately constrained by decoded-pixel verification.
const MAX_FLAT_QUESTION_ASSET_BODY_BYTES: usize = 8 * 1024 * 1024;
const ASSET_LABEL_HEADER: &str = "x-ple-asset-label";
const ASSET_PROVENANCE_HEADER: &str = "x-ple-asset-provenance";
const ASSET_LICENSE: &str = "author supplied instructional image";

/// Builds the private authoring image route group.
pub fn router<S, O>(store: Arc<S>, objects: Arc<O>) -> Router
where
    S: Store + FlatQuestionAssetStore + SessionStore + AuthoritativeTimeStore + 'static,
    O: ObjectStore + 'static,
{
    Router::new()
        .route(
            "/api/workspaces/{workspace}/flat-question-assets",
            get(list_flat_question_assets::<S, O>).post(upload_flat_question_asset::<S, O>),
        )
        .layer(DefaultBodyLimit::max(MAX_FLAT_QUESTION_ASSET_BODY_BYTES))
        .layer(middleware::map_response(no_store_response))
        .with_state(FlatQuestionAssetRouteState { store, objects })
}

struct FlatQuestionAssetRouteState<S, O> {
    store: Arc<S>,
    objects: Arc<O>,
}

impl<S, O> Clone for FlatQuestionAssetRouteState<S, O> {
    fn clone(&self) -> Self {
        Self {
            store: Arc::clone(&self.store),
            objects: Arc::clone(&self.objects),
        }
    }
}

/// Browser-safe descriptor. The checksum is a server-verified immutable binding for author source
/// that source may echo; the server still re-resolves it before later use.
/// Object keys, storage timestamps, license, and provenance stay private.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
struct FlatQuestionAssetResponse {
    asset_id: AssetId,
    content_checksum: Sha256Digest,
    display_label: String,
    media_type: String,
    intrinsic_width: u32,
    intrinsic_height: u32,
}

impl From<WorkspaceFlatQuestionAsset> for FlatQuestionAssetResponse {
    fn from(value: WorkspaceFlatQuestionAsset) -> Self {
        Self {
            asset_id: value.asset,
            content_checksum: value.checksum(),
            display_label: value.display_label,
            media_type: value.object.media_type,
            intrinsic_width: value.intrinsic_width,
            intrinsic_height: value.intrinsic_height,
        }
    }
}

async fn list_flat_question_assets<S, O>(
    State(state): State<FlatQuestionAssetRouteState<S, O>>,
    headers: HeaderMap,
    Path(workspace): Path<WorkspaceId>,
) -> Response
where
    S: Store + FlatQuestionAssetStore + SessionStore + AuthoritativeTimeStore + 'static,
    O: ObjectStore + 'static,
{
    let authenticated = match resolve_request_session(state.store.as_ref(), &headers).await {
        Ok(value) => value,
        Err(error) => return auth_error_response(error),
    };
    if !may_author(authenticated.record.subject.roles()) {
        return workspace_not_found();
    }
    if !workspace_is_visible(
        state.store.as_ref(),
        authenticated.tenant_context,
        authenticated.record.subject.user(),
        workspace,
    )
    .await
    {
        return workspace_not_found();
    }
    match state
        .store
        .list_workspace_flat_question_assets(authenticated.tenant_context, workspace)
        .await
    {
        Ok(assets) => no_store(
            Json(
                assets
                    .into_iter()
                    .map(FlatQuestionAssetResponse::from)
                    .collect::<Vec<_>>(),
            )
            .into_response(),
        ),
        Err(error) => asset_store_error(error),
    }
}

async fn upload_flat_question_asset<S, O>(
    State(state): State<FlatQuestionAssetRouteState<S, O>>,
    Path(workspace): Path<WorkspaceId>,
    request: Request,
) -> Response
where
    S: Store + FlatQuestionAssetStore + SessionStore + AuthoritativeTimeStore + 'static,
    O: ObjectStore + 'static,
{
    let authenticated = match resolve_request_session(state.store.as_ref(), request.headers()).await
    {
        Ok(value) => value,
        Err(error) => return auth_error_response(error),
    };
    if !may_author(authenticated.record.subject.roles()) {
        return workspace_not_found();
    }
    if !workspace_is_visible(
        state.store.as_ref(),
        authenticated.tenant_context,
        authenticated.record.subject.user(),
        workspace,
    )
    .await
    {
        return workspace_not_found();
    }
    let metadata = match upload_metadata(request.headers()) {
        Ok(value) => value,
        Err(response) => return response.into_response(),
    };

    // Every authorization and metadata refusal above happens before this read.
    let bytes = match to_bytes(request.into_body(), MAX_FLAT_QUESTION_ASSET_BODY_BYTES).await {
        Ok(value) if !value.is_empty() => value.to_vec(),
        Ok(_) => {
            return error_response(
                StatusCode::UNPROCESSABLE_ENTITY,
                "choose an image to upload",
            );
        }
        Err(_) => {
            return error_response(StatusCode::PAYLOAD_TOO_LARGE, "image upload is too large");
        }
    };
    let verified = match verify_hotspot_image(&bytes) {
        Ok(value) => value,
        Err(error) => {
            return error_response(StatusCode::UNPROCESSABLE_ENTITY, error.user_message());
        }
    };
    let created_at = match state
        .store
        .authoritative_time(authenticated.tenant_context)
        .await
    {
        Ok(value) => value,
        Err(error) => return asset_store_error(error),
    };
    let asset = AssetId::generate();
    let object = ObjectId::generate();
    let key = ObjectKey::WorkspaceQuestionAsset {
        tenant: authenticated.tenant_context.tenant_id(),
        workspace,
        asset,
        object,
    };
    let candidate = PutObject {
        key: key.clone(),
        bytes,
        media_type: verified.media_type.canonical_media_type().to_string(),
        license: ASSET_LICENSE.to_string(),
        provenance: metadata.provenance,
        created_at,
    };
    let object = match state.objects.put(candidate).await {
        Ok(value) => value,
        Err(error) => return object_error_response(error),
    };
    let descriptor = match WorkspaceFlatQuestionAsset::new(
        authenticated.tenant_context.tenant_id(),
        workspace,
        asset,
        object,
        verified.width,
        verified.height,
        metadata.label,
    ) {
        Ok(value) => value,
        Err(error) => {
            return compensate_object_then_error(state.objects.as_ref(), &key, error).await;
        }
    };
    match state
        .store
        .register_workspace_flat_question_asset(authenticated.tenant_context, descriptor)
        .await
    {
        Ok(value) => no_store(
            (
                StatusCode::CREATED,
                Json(FlatQuestionAssetResponse::from(value)),
            )
                .into_response(),
        ),
        Err(error) => compensate_object_then_error(state.objects.as_ref(), &key, error).await,
    }
}

async fn workspace_is_visible<S>(
    store: &S,
    context: learning_data_access::TenantContext,
    actor: question_model::UserId,
    workspace: WorkspaceId,
) -> bool
where
    S: Store,
{
    matches!(
        store.get_draft(context, actor, workspace).await,
        Ok(Some(_))
    )
}

struct UploadMetadata {
    label: String,
    provenance: String,
}

fn upload_metadata(headers: &HeaderMap) -> HttpResult<UploadMetadata> {
    for (name, _) in headers {
        if name.as_str().starts_with("x-ple-")
            && name.as_str() != ASSET_LABEL_HEADER
            && name.as_str() != ASSET_PROVENANCE_HEADER
        {
            return Err(error_response(
                StatusCode::BAD_REQUEST,
                "image upload metadata is invalid",
            )
            .into());
        }
    }
    Ok(UploadMetadata {
        label: one_safe_header(headers, ASSET_LABEL_HEADER, "image label")?,
        provenance: one_safe_header(headers, ASSET_PROVENANCE_HEADER, "image provenance")?,
    })
}

fn one_safe_header(headers: &HeaderMap, name: &str, user_name: &str) -> HttpResult<String> {
    let mut values = headers.get_all(name).iter();
    let Some(value) = values.next() else {
        return Err(
            error_response(StatusCode::BAD_REQUEST, &format!("{user_name} is required")).into(),
        );
    };
    if values.next().is_some() {
        return Err(error_response(
            StatusCode::BAD_REQUEST,
            &format!("{user_name} must appear once"),
        )
        .into());
    }
    let Ok(value) = value.to_str() else {
        return Err(
            error_response(StatusCode::BAD_REQUEST, &format!("{user_name} is invalid")).into(),
        );
    };
    if value.trim().is_empty() || value.chars().any(char::is_control) {
        return Err(
            error_response(StatusCode::BAD_REQUEST, &format!("{user_name} is invalid")).into(),
        );
    }
    Ok(value.trim().to_string())
}

async fn compensate_object_then_error<O>(
    objects: &O,
    key: &ObjectKey,
    error: StoreError,
) -> Response
where
    O: ObjectStore,
{
    if objects.delete(key).await.is_err() {
        return error_response(
            StatusCode::SERVICE_UNAVAILABLE,
            "image upload cleanup failed; try again later",
        );
    }
    asset_store_error(error)
}

fn may_author(roles: &[UserRole]) -> bool {
    roles
        .iter()
        .any(|role| matches!(role, UserRole::Instructor | UserRole::Sysadmin))
}

fn workspace_not_found() -> Response {
    error_response(StatusCode::NOT_FOUND, "workspace not found")
}

fn asset_store_error(error: StoreError) -> Response {
    match error {
        StoreError::NotFound | StoreError::TenantMismatch | StoreError::Forbidden => {
            workspace_not_found()
        }
        StoreError::AlreadyExists | StoreError::Conflict | StoreError::TimedOut => error_response(
            StatusCode::CONFLICT,
            "image upload conflicts with existing workspace state",
        ),
        StoreError::InvalidRecord(_) | StoreError::RunModel(_) => error_response(
            StatusCode::UNPROCESSABLE_ENTITY,
            "image metadata is invalid",
        ),
        StoreError::RetryableTransaction | StoreError::Unavailable(_) => error_response(
            StatusCode::SERVICE_UNAVAILABLE,
            "image workspace storage is unavailable",
        ),
    }
}

fn object_error_response(error: ObjectStoreError) -> Response {
    match error {
        ObjectStoreError::Unavailable(_) => error_response(
            StatusCode::SERVICE_UNAVAILABLE,
            "image storage is unavailable",
        ),
        ObjectStoreError::AlreadyExists
        | ObjectStoreError::NotFound
        | ObjectStoreError::ChecksumMismatch
        | ObjectStoreError::NotSignable
        | ObjectStoreError::NumericOverflow => {
            error_response(StatusCode::CONFLICT, "image storage conflict")
        }
    }
}

fn error_response(status: StatusCode, message: &str) -> Response {
    no_store((status, Json(serde_json::json!({"error": message}))).into_response())
}

async fn no_store_response(response: Response) -> Response {
    no_store(response)
}

#[cfg(test)]
#[path = "flat_question_assets/tests.rs"]
mod tests;
