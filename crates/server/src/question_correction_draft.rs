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
    ForkPublishedQuestionImageInput, PublishedQuestionForkImage, QuestionForkStore,
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
    QuestionBackend, QuestionMetadata, QuestionResponseFormat, QuestionRevisionNumber,
    QuestionType, UserRole,
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

struct VerifiedHotspotImage {
    evidence: PublishedQuestionForkImage,
    bytes: Vec<u8>,
}

#[derive(Debug, PartialEq, Eq)]
enum HotspotImageLookup {
    NotRequired,
    Native,
    UnsupportedBackend,
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
    let hotspot = match load_hotspot_image(
        &state,
        session,
        &entry.question_type,
        &entry.backend,
        &parent,
        source.bytes(),
    )
    .await
    {
        Ok(value) => value,
        Err(response) => return *response,
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
    let target_image = match copy_hotspot_image(
        &state.objects,
        workspace,
        draft_question_uuid,
        hotspot.as_ref(),
    )
    .await
    {
        Ok(value) => value,
        Err(_) => {
            let _ = state.objects.delete(&source_record.address).await;
            return unavailable();
        }
    };
    let image_record = target_image
        .as_ref()
        .map(|image| image.target_record.clone());
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
            cleanup_object(&state.objects, image_record.as_ref()).await;
            return unavailable();
        }
    };
    if let Some(image) = target_image
        && state
            .images
            .register_draft_question_image(
                session,
                RegisterDraftQuestionImageInput {
                    draft_question_uuid,
                    expected_edit_number: draft.edit_number,
                    question_image_asset_id: image.question_image_asset_id,
                    source_record: image.target_record,
                    intrinsic_width: image.intrinsic_width,
                    intrinsic_height: image.intrinsic_height,
                },
            )
            .await
            .is_err()
    {
        cleanup_unpublished_draft(&state, session, draft_question_uuid, image_record.as_ref())
            .await;
        return unavailable();
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
            cleanup_unpublished_draft(&state, session, draft_question_uuid, image_record.as_ref())
                .await;
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

async fn load_hotspot_image(
    state: &RouteState,
    session: SessionTokenHash,
    question_type: &QuestionType,
    backend: &QuestionBackend,
    parent: &PublishedQuestionRevisionTuple,
    source: &[u8],
) -> Result<Option<VerifiedHotspotImage>, Box<Response>> {
    match hotspot_image_lookup(backend, question_type) {
        HotspotImageLookup::NotRequired => return Ok(None),
        HotspotImageLookup::Native => {}
        HotspotImageLookup::UnsupportedBackend => return Err(Box::new(unavailable())),
    }
    let document = adapter_ple::question_json::PleQuestionJsonDocument::parse(source)
        .map_err(|_| Box::new(unavailable()))?;
    let compiled = document.compile().map_err(|_| Box::new(unavailable()))?;
    let QuestionResponseFormat::Hotspot {
        question_image_asset_tuple,
        ..
    } = compiled.presentation().response()
    else {
        return Err(Box::new(unavailable()));
    };
    let evidence = state
        .forks
        .load_published_question_fork_asset(session, parent)
        .await
        .map_err(|_| Box::new(unavailable()))?
        .ok_or_else(|| Box::new(unavailable()))?;
    if evidence.question_image_asset_id != question_image_asset_tuple.question_image_asset_id
        || evidence.source_record.sha256.to_string() != question_image_asset_tuple.checksum
    {
        return Err(Box::new(unavailable()));
    }
    let source_image = state
        .objects
        .get(&evidence.source_record.address)
        .await
        .map_err(|_| Box::new(unavailable()))?;
    if source_image.record != evidence.source_record {
        return Err(Box::new(unavailable()));
    }
    let verified = objects::image_validation::verify_still_image(&source_image.bytes)
        .map_err(|_| Box::new(unavailable()))?;
    if verified.media_type.canonical_media_type() != evidence.source_record.media_type
        || verified.width != evidence.intrinsic_width
        || verified.height != evidence.intrinsic_height
    {
        return Err(Box::new(unavailable()));
    }
    Ok(Some(VerifiedHotspotImage {
        evidence,
        bytes: source_image.bytes,
    }))
}

fn hotspot_image_lookup(
    backend: &QuestionBackend,
    question_type: &QuestionType,
) -> HotspotImageLookup {
    if *backend == QuestionBackend::Webwork {
        return HotspotImageLookup::NotRequired;
    }
    if *question_type != QuestionType::Hotspot {
        return HotspotImageLookup::NotRequired;
    }
    if *backend == QuestionBackend::Ple {
        return HotspotImageLookup::Native;
    }
    HotspotImageLookup::UnsupportedBackend
}

async fn copy_hotspot_image(
    objects: &S3ObjectStore,
    workspace: question_model::WorkspaceId,
    draft_question_uuid: DraftQuestionUuid,
    image: Option<&VerifiedHotspotImage>,
) -> Result<Option<ForkPublishedQuestionImageInput>, objects::ObjectStoreError> {
    let Some(image) = image else { return Ok(None) };
    let target_record = objects
        .put(PutObject {
            address: ObjectAddress::DraftQuestionImage {
                workspace_id: workspace,
                draft_question_id: draft_question_uuid.as_uuid(),
                question_image_asset_id: image.evidence.question_image_asset_id,
                object_id: ModelObjectId::generate(),
            },
            bytes: image.bytes.clone(),
            media_type: image.evidence.source_record.media_type.clone(),
            created_at: crate::authoring::now(),
        })
        .await?;
    Ok(Some(ForkPublishedQuestionImageInput {
        question_image_asset_id: image.evidence.question_image_asset_id,
        target_record,
        intrinsic_width: image.evidence.intrinsic_width,
        intrinsic_height: image.evidence.intrinsic_height,
    }))
}

async fn cleanup_unpublished_draft(
    state: &RouteState,
    session: SessionTokenHash,
    draft_question_uuid: DraftQuestionUuid,
    image_record: Option<&objects::ObjectRecord>,
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
        cleanup_object(&state.objects, image_record).await;
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

#[cfg(test)]
mod tests {
    use super::{HotspotImageLookup, hotspot_image_lookup};
    use question_model::{QuestionBackend, QuestionType};

    #[test]
    fn correction_hotspot_asset_read_keeps_role_and_asset_validation_contract() {
        let sql = include_str!("../../../schemas/base_schema/50_functions/question_images.sql");
        let function = sql
            .split_once("CREATE FUNCTION ple_private.load_question_fork_asset(")
            .expect("published fork asset reader")
            .1
            .split_once("$$;")
            .expect("function body terminator")
            .0;
        let normalized = function.split_whitespace().collect::<Vec<_>>().join(" ");
        assert!(normalized.contains(
            "WHERE (ple_api.current_session_account_is_instructor() OR ple_api.current_session_account_is_sysadmin())"
        ));

        for required in [
            "revision.backend = 'ple'",
            "metadata.question_type = 'hotspot'",
            "publication.published_question_id = p_published_question_id",
            "publication.revision_number = p_revision_number",
            "record.sha256 = publication.source_object_checksum",
            "record.media_type = publication.verified_media_type::text",
            "record.object_address = pg_catalog.jsonb_build_object(",
            "record.object_storage_area = 'private-content'",
            "record.object_data_class = 'question-image'",
        ] {
            assert!(
                function.contains(required),
                "missing SQL contract: {required}"
            );
        }
    }

    #[test]
    fn correction_hotspot_image_lookup_is_native_only() {
        assert_eq!(
            hotspot_image_lookup(&QuestionBackend::Webwork, &QuestionType::Hotspot),
            HotspotImageLookup::NotRequired,
        );
        assert_eq!(
            hotspot_image_lookup(&QuestionBackend::Ple, &QuestionType::Hotspot),
            HotspotImageLookup::Native,
        );
    }
}
