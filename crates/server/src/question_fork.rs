//! Active-Instructor command for forked private Draft Questions.
//!
//! The canonical path is the sole browser source selector. Its empty request
//! body deliberately cannot carry a Question ID, source bytes, attribution,
//! authorship, workspace, or Draft payload.

use std::sync::Arc;

use axum::{
    Json, Router,
    body::Bytes,
    extract::{Path, State},
    http::{
        HeaderMap, StatusCode,
        header::{COOKIE, HeaderName},
    },
    response::{IntoResponse, Response},
    routing::post,
};
use learning_data_access::{
    AuthoringDraftStore, ForkPublishedQuestionInput, QuestionForkStore, QuestionLibraryStore,
    SessionTokenHash, StoreError,
    postgres::{
        PostgresAuthoringDraftStore, PostgresQuestionForkStore, PostgresQuestionLibraryStore,
        PostgresSessionStore,
    },
};
use objects::{
    ObjectAddress, ObjectStore, ObjectStoreError, PutObject, ResolvedQuestionSource,
    s3::S3ObjectStore,
};
use question_model::{
    ObjectId, PublishedQuestionId, PublishedQuestionRevisionTuple, QuestionRevisionNumber, UserRole,
};
use serde::Serialize;
use uuid::Uuid;

use crate::auth::{AuthError, resolve_session};

const IDEMPOTENCY_KEY: HeaderName = HeaderName::from_static("idempotency-key");

#[derive(Clone)]
struct RouteState {
    sessions: Arc<PostgresSessionStore>,
    forks: Arc<dyn QuestionForkStore>,
    library: PostgresQuestionLibraryStore,
    drafts: PostgresAuthoringDraftStore,
    objects: S3ObjectStore,
}

/// Registers the canonical active-Instructor fork command.
pub fn question_fork_router(
    sessions: Arc<PostgresSessionStore>,
    forks: PostgresQuestionForkStore,
    library: PostgresQuestionLibraryStore,
    drafts: PostgresAuthoringDraftStore,
    objects: S3ObjectStore,
) -> Router {
    question_fork_router_with_trusted_dependencies(
        sessions,
        Arc::new(forks),
        library,
        drafts,
        objects,
    )
}

fn question_fork_router_with_trusted_dependencies(
    sessions: Arc<PostgresSessionStore>,
    forks: Arc<dyn QuestionForkStore>,
    library: PostgresQuestionLibraryStore,
    drafts: PostgresAuthoringDraftStore,
    objects: S3ObjectStore,
) -> Router {
    Router::new()
        .route(
            "/api/questions/by-id/{question_id}/revisions/{revision_number}/fork",
            post(fork_published_question),
        )
        .with_state(RouteState {
            sessions,
            forks,
            library,
            drafts,
            objects,
        })
}

/// Answer-free navigation result for one distinct private Draft.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ForkedQuestionResponse {
    draft_question: Uuid,
}

async fn fork_published_question(
    State(state): State<RouteState>,
    headers: HeaderMap,
    Path((question_id, revision_number)): Path<(String, String)>,
    body: Bytes,
) -> Response {
    if !body.is_empty() {
        return route_error(StatusCode::UNPROCESSABLE_ENTITY, "Question Fork is invalid");
    }
    let session = match instructor_session_hash(&state, &headers).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let idempotency_key = match idempotency_key(&headers) {
        Some(value) => value,
        None => {
            return route_error(
                StatusCode::BAD_REQUEST,
                "Question Fork retry key is invalid",
            );
        }
    };
    let source_published_question_revision_tuple =
        match canonical_source(question_id, revision_number) {
            Some(value) => value,
            None => return concealed(),
        };
    let library_entry = match state
        .library
        .load_published_question_revision_library_entry(
            session,
            &source_published_question_revision_tuple,
        )
        .await
    {
        Ok(value) => value,
        Err(error) => return store_error_response(error),
    };
    let source = match ResolvedQuestionSource::resolve(
        &state.objects,
        source_published_question_revision_tuple.clone(),
        library_entry.source_object_id,
        library_entry.source_object_checksum.clone(),
    )
    .await
    {
        Ok(value) if value.media_type() == library_entry.source_media_type => value,
        Ok(_) | Err(_) => return unavailable(),
    };
    let workspace = match state
        .drafts
        .ensure_own_authoring_workspace(session, Uuid::now_v7())
        .await
    {
        Ok(value) => value,
        Err(error) => return store_error_response(error),
    };
    let question_images = match crate::question_fork_images::load_question_images(
        state.forks.as_ref(),
        &state.objects,
        session,
        library_entry.backend,
        &source_published_question_revision_tuple,
        source.bytes(),
    )
    .await
    {
        Ok(value) => value,
        Err(()) => return unavailable(),
    };

    let proposed_draft_question_id = Uuid::now_v7();
    let target_source_record = match state
        .objects
        .put(PutObject {
            address: ObjectAddress::WorkspaceQuestionSource {
                workspace_id: workspace,
                object_id: ObjectId::generate(),
            },
            bytes: source.bytes().to_vec(),
            media_type: source.media_type().to_owned(),
            created_at: crate::authoring::now(),
        })
        .await
    {
        Ok(value) => value,
        Err(_) => return unavailable(),
    };
    let target_source_address = target_source_record.address.clone();
    let target_question_images = match crate::question_fork_images::copy_question_images(
        &state.objects,
        workspace,
        proposed_draft_question_id,
        &question_images,
    )
    .await
    {
        Ok(value) => value,
        Err(_) => {
            if state.objects.delete(&target_source_address).await.is_err() {
                return unavailable();
            }
            return unavailable();
        }
    };
    let target_asset_addresses = target_question_images
        .iter()
        .map(|asset| asset.target_record.address.clone())
        .collect::<Vec<_>>();
    let input = ForkPublishedQuestionInput {
        source_published_question_revision_tuple: source_published_question_revision_tuple.clone(),
        workspace,
        proposed_draft_question_id,
        target_source_record,
        question_images: target_question_images,
        idempotency_key,
    };
    match state
        .forks
        .fork_published_question_to_draft(session, input)
        .await
    {
        Ok(fork) => {
            if !fork.created_new {
                cleanup_replayed_candidates(
                    &state.objects,
                    &target_source_address,
                    &target_asset_addresses,
                )
                .await
            }
            crate::auth::no_store(
                (
                    StatusCode::CREATED,
                    Json(ForkedQuestionResponse {
                        draft_question: fork.draft_question_uuid.as_uuid(),
                    }),
                )
                    .into_response(),
            )
        }
        // An uncertain commit may have registered these candidates. Only an
        // authoritative replay receipt permits deleting unregistered objects.
        Err(error) => store_error_response(error),
    }
}

async fn cleanup_candidates(
    objects: &S3ObjectStore,
    source: &ObjectAddress,
    assets: &[ObjectAddress],
) -> Result<(), ObjectStoreError> {
    let source_result = objects.delete(source).await;
    let mut result = source_result;
    for asset in assets {
        let asset_result = objects.delete(asset).await;
        result = result.and(asset_result);
    }
    result
}

async fn cleanup_replayed_candidates(
    objects: &S3ObjectStore,
    source: &ObjectAddress,
    assets: &[ObjectAddress],
) {
    if let Err(error) = cleanup_candidates(objects, source, assets).await {
        // The immutable database receipt is authoritative. Cleanup remains a
        // diagnostic concern and must never turn a recovered retry into failure.
        tracing::error!(?error, "Question Fork replay candidate cleanup failed");
    }
}

fn canonical_source(
    question_id: String,
    revision_number: String,
) -> Option<PublishedQuestionRevisionTuple> {
    let question_id = question_id.parse::<PublishedQuestionId>().ok()?;
    let revision_number = revision_number
        .parse::<u32>()
        .ok()
        .and_then(|value| QuestionRevisionNumber::new(value).ok())?;
    Some(PublishedQuestionRevisionTuple {
        published_question_id: question_id,
        revision_number,
    })
}

fn idempotency_key(headers: &HeaderMap) -> Option<Uuid> {
    headers.get(&IDEMPOTENCY_KEY)?.to_str().ok()?.parse().ok()
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
        Ok(session) if session.record.user_role == UserRole::Instructor => Ok(session.session_hash),
        Ok(_) | Err(AuthError::Unauthenticated) => Err(Box::new(concealed())),
        Err(AuthError::Unavailable(_) | AuthError::Randomness(_)) => Err(Box::new(route_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "Question Fork authentication is unavailable",
        ))),
    }
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
        StoreError::InvalidRecord(_) => {
            route_error(StatusCode::UNPROCESSABLE_ENTITY, "Question Fork is invalid")
        }
        StoreError::Conflict | StoreError::RetryableTransaction => {
            route_error(StatusCode::PRECONDITION_FAILED, "Question Fork changed")
        }
        StoreError::LifecycleConflict | StoreError::AlreadyExists => {
            route_error(StatusCode::CONFLICT, "Question Fork conflicts")
        }
        StoreError::AssessmentActivity(_)
        | StoreError::TimedOut
        | StoreError::LeaseLost
        | StoreError::Unavailable(_) => route_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "Question Fork is unavailable",
        ),
    }
}

fn concealed() -> Response {
    route_error(StatusCode::NOT_FOUND, "Question Fork not found")
}

fn route_error(status: StatusCode, message: &'static str) -> Response {
    crate::auth::no_store((status, message).into_response())
}

fn unavailable() -> Response {
    route_error(
        StatusCode::SERVICE_UNAVAILABLE,
        "Question Fork is unavailable",
    )
}
