//! Owner and Sysadmin creation of an ordinary Draft for current Question content correction.

use std::sync::Arc;

use axum::{
    Json, Router,
    body::Bytes,
    extract::{Path, State},
    http::{HeaderMap, StatusCode, header::COOKIE},
    response::{IntoResponse, Response},
    routing::post,
};
use learning_data_access::{
    AuthoringDraftStore, CreateAuthoringDraftInput, DraftQuestionImageStore, DraftQuestionUuid,
    QuestionLibraryStore, RegisterDraftQuestionImageInput, SaveAuthoringDraftGeneralFeedbackInput,
    SessionTokenHash, StoreError,
    postgres::{
        PostgresAuthoringDraftStore, PostgresDraftQuestionImageStore, PostgresQuestionForkStore,
        PostgresQuestionLibraryStore, PostgresSessionStore,
    },
};
use objects::{ObjectAddress, ObjectStore, PutObject, ResolvedQuestionSource, s3::S3ObjectStore};
use question_model::{
    ObjectId as ModelObjectId, PublishedQuestionId, PublishedQuestionRevisionTuple,
    QuestionBackend, QuestionMetadata, QuestionRevisionNumber, UserRole,
};
use serde::Serialize;
use uuid::Uuid;

use crate::auth::{AuthError, resolve_session};

#[derive(Clone)]
struct RouteState {
    sessions: Arc<PostgresSessionStore>,
    library: PostgresQuestionLibraryStore,
    forks: PostgresQuestionForkStore,
    drafts: PostgresAuthoringDraftStore,
    images: PostgresDraftQuestionImageStore,
    objects: S3ObjectStore,
}

pub fn question_correction_draft_router(
    sessions: Arc<PostgresSessionStore>,
    library: PostgresQuestionLibraryStore,
    forks: PostgresQuestionForkStore,
    drafts: PostgresAuthoringDraftStore,
    images: PostgresDraftQuestionImageStore,
    objects: S3ObjectStore,
) -> Router {
    Router::new()
        .route(
            "/api/questions/by-id/{question_id}/revisions/{revision_number}/correction-draft",
            post(create_correction_draft),
        )
        .with_state(RouteState {
            sessions,
            library,
            forks,
            drafts,
            images,
            objects,
        })
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CorrectionDraftResponse {
    draft_question: Uuid,
    published_question_revision_tuple: PublishedQuestionRevisionTuple,
}

async fn create_correction_draft(
    State(state): State<RouteState>,
    headers: HeaderMap,
    Path((question_id, revision_number)): Path<(String, String)>,
    body: Bytes,
) -> Response {
    if !body.is_empty() {
        return route_error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Question correction request is invalid",
        );
    }
    let session = match correction_session(&state, &headers).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let parent = match current_tuple(question_id, revision_number) {
        Some(value) => value,
        None => return concealed(),
    };
    let current = match state
        .library
        .load_published_question_library_entry(session, &parent.published_question_id)
        .await
    {
        Ok(entry)
            if entry.published_question_revision_tuple == parent
                && entry.availability == question_model::QuestionAvailability::Available
                && entry.viewer_may_edit_metadata =>
        {
            entry
        }
        Ok(_)
        | Err(StoreError::NotFound | StoreError::Forbidden | StoreError::OwnershipMismatch) => {
            return concealed();
        }
        Err(_) => return unavailable(),
    };
    let entry = match state
        .library
        .load_published_question_revision_library_entry(session, &parent)
        .await
    {
        Ok(entry)
            if entry.published_question_revision_tuple
                == current.published_question_revision_tuple
                && entry.availability == question_model::QuestionAvailability::Available
                && entry.viewer_may_edit_metadata =>
        {
            entry
        }
        Ok(_)
        | Err(StoreError::NotFound | StoreError::Forbidden | StoreError::OwnershipMismatch) => {
            return concealed();
        }
        Err(_) => return unavailable(),
    };
    if entry.backend == QuestionBackend::Imathas {
        return route_error(
            StatusCode::NOT_IMPLEMENTED,
            "Content correction is unavailable for this Question Backend",
        );
    }
    let source = match ResolvedQuestionSource::resolve(
        &state.objects,
        parent.clone(),
        entry.source_object_id,
        entry.source_object_checksum.clone(),
    )
    .await
    {
        Ok(source) if source.media_type() == entry.source_media_type => source,
        _ => return unavailable(),
    };
    let images = match crate::question_fork_images::load_question_images(
        &state.forks,
        &state.objects,
        session,
        entry.backend,
        &parent,
        source.bytes(),
    )
    .await
    {
        Ok(value) => value,
        Err(()) => return unavailable(),
    };
    let workspace = match state
        .drafts
        .ensure_own_authoring_workspace(session, Uuid::now_v7())
        .await
    {
        Ok(value) => value,
        Err(_) => return unavailable(),
    };
    let draft_question_uuid = DraftQuestionUuid::from_uuid(Uuid::now_v7());
    let source_record = match state
        .objects
        .put(PutObject {
            address: ObjectAddress::WorkspaceQuestionSource {
                workspace_id: workspace,
                object_id: ModelObjectId::generate(),
            },
            bytes: source.bytes().to_vec(),
            media_type: source.media_type().to_owned(),
            created_at: crate::authoring::now(),
        })
        .await
    {
        Ok(record) => record,
        Err(_) => return unavailable(),
    };
    let source_record_cleanup = source_record.clone();
    let target_images = match crate::question_fork_images::copy_question_images(
        &state.objects,
        workspace,
        draft_question_uuid.as_uuid(),
        &images,
    )
    .await
    {
        Ok(value) => value,
        Err(_) => {
            let _ = state.objects.delete(&source_record.address).await;
            return unavailable();
        }
    };
    let image_records = target_images
        .iter()
        .map(|image| image.target_record.clone())
        .collect::<Vec<_>>();
    let metadata = QuestionMetadata {
        question_title: entry.question_title.clone(),
        question_description: entry.question_description.clone(),
        tags: entry.shared_metadata.tags.clone(),
        question_license: Some(entry.question_license.clone()),
        question_citation: entry.question_citation.clone(),
        language: entry.language.clone(),
    };
    let draft = match state
        .drafts
        .create_authoring_draft(
            session,
            workspace,
            CreateAuthoringDraftInput {
                draft_question_uuid,
                source_record,
                question_format: entry.question_format,
                webwork_pg_path: entry.webwork_pg_path.clone(),
                metadata: metadata.clone(),
                question_type: Some(entry.question_type),
            },
        )
        .await
    {
        Ok(value) => value,
        Err(_) => {
            cleanup_object(&state.objects, Some(&source_record_cleanup)).await;
            cleanup_images(&state.objects, &image_records).await;
            return unavailable();
        }
    };
    for image in target_images {
        if state
            .images
            .register_draft_question_image(
                session,
                RegisterDraftQuestionImageInput {
                    draft_question_uuid,
                    expected_edit_number: draft.edit_number,
                    question_image_asset_id: image.question_image_asset_id,
                    source_relative_name: image.source_relative_name,
                    source_record: image.target_record,
                    intrinsic_width: image.intrinsic_width,
                    intrinsic_height: image.intrinsic_height,
                },
            )
            .await
            .is_err()
        {
            cleanup_unpublished_draft(&state, session, draft_question_uuid, &image_records).await;
            return unavailable();
        }
    }
    let support_saved = state
        .drafts
        .save_authoring_draft_general_feedback(
            session,
            SaveAuthoringDraftGeneralFeedbackInput {
                draft_question_uuid,
                expected_edit_number: draft.edit_number,
                metadata,
                general_feedback: entry.revision_general_feedback,
                hint: entry.revision_hint,
                worked_solution: entry.revision_worked_solution,
                replace_support: true,
            },
        )
        .await;
    let saved = match support_saved {
        Ok(value) => value,
        Err(_) => {
            cleanup_unpublished_draft(&state, session, draft_question_uuid, &image_records).await;
            return unavailable();
        }
    };
    crate::auth::no_store(
        (
            StatusCode::CREATED,
            Json(CorrectionDraftResponse {
                draft_question: saved.draft_question_uuid.as_uuid(),
                published_question_revision_tuple: parent,
            }),
        )
            .into_response(),
    )
}

async fn cleanup_unpublished_draft(
    state: &RouteState,
    session: SessionTokenHash,
    draft_question_uuid: DraftQuestionUuid,
    image_records: &[objects::ObjectRecord],
) {
    let draft = state
        .drafts
        .load_authoring_draft(session, draft_question_uuid)
        .await;
    let Some(draft) = draft.ok() else { return };
    if state
        .drafts
        .delete_authoring_draft(
            session,
            learning_data_access::DeleteAuthoringDraftInput {
                draft_question_uuid,
                expected_edit_number: draft.edit_number,
            },
        )
        .await
        .is_ok()
    {
        cleanup_object(&state.objects, Some(&draft.source_record)).await;
        cleanup_images(&state.objects, image_records).await;
    }
}

async fn cleanup_images(objects: &S3ObjectStore, images: &[objects::ObjectRecord]) {
    for image in images {
        let _ = objects.delete(&image.address).await;
    }
}

async fn cleanup_object(objects: &S3ObjectStore, record: Option<&objects::ObjectRecord>) {
    if let Some(record) = record {
        let _ = objects.delete(&record.address).await;
    }
}

fn current_tuple(
    question_id: String,
    revision_number: String,
) -> Option<PublishedQuestionRevisionTuple> {
    let published_question_id = question_id.parse::<PublishedQuestionId>().ok()?;
    if published_question_id.to_string() != question_id {
        return None;
    }
    let revision_number = revision_number.parse::<u32>().ok()?;
    Some(PublishedQuestionRevisionTuple {
        published_question_id,
        revision_number: QuestionRevisionNumber::new(revision_number).ok()?,
    })
}

async fn correction_session(
    state: &RouteState,
    headers: &HeaderMap,
) -> Result<SessionTokenHash, Box<Response>> {
    let cookies = headers
        .get_all(COOKIE)
        .iter()
        .map(|value| value.to_str().ok())
        .collect::<Option<Vec<_>>>()
        .filter(|values| !values.is_empty())
        .map(|values| values.join("; "));
    match resolve_session(state.sessions.as_ref(), cookies.as_deref()).await {
        Ok(session)
            if matches!(
                session.record.user_role,
                UserRole::Instructor | UserRole::Sysadmin
            ) =>
        {
            Ok(session.session_hash)
        }
        Ok(_) | Err(AuthError::Unauthenticated) => Err(Box::new(concealed())),
        Err(AuthError::Unavailable(_) | AuthError::Randomness(_)) => Err(Box::new(unavailable())),
    }
}

fn concealed() -> Response {
    crate::auth::no_store(
        (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "Not found" })),
        )
            .into_response(),
    )
}

fn unavailable() -> Response {
    crate::auth::no_store(
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({ "error": "Question correction Draft is unavailable" })),
        )
            .into_response(),
    )
}

fn route_error(status: StatusCode, message: &'static str) -> Response {
    crate::auth::no_store((status, Json(serde_json::json!({ "error": message }))).into_response())
}
