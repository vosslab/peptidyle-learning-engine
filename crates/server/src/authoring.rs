//! Private Draft Question authoring and first-publication Server Routes.
//!
//! An authorized private Draft UUID and its Edit Number cross the browser
//! boundary. Workspace UUIDs, source object addresses, and publication copy
//! inputs remain server-only.

use std::{
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use axum::{
    Json, Router,
    body::Bytes,
    extract::{DefaultBodyLimit, Path, State},
    http::{
        HeaderMap, HeaderValue, StatusCode,
        header::{CONTENT_TYPE, ETAG},
    },
    response::{IntoResponse, Response},
    routing::{delete, get, post},
};
use learning_data_access::{
    AuthoringDraftStore, CreateAuthoringDraftInput, DeleteAuthoringDraftInput, DraftQuestionUuid,
    SaveAuthoringDraftGeneralFeedbackInput, SaveAuthoringDraftInput, StoreError,
    postgres::{
        PostgresAuthoringAssetsStore, PostgresAuthoringDraftStore,
        PostgresDraftQuestionSourceBindingStore, PostgresSessionStore,
    },
};
use objects::s3::S3ObjectStore;
use question_model::{QuestionFormat, QuestionRevisionReason, QuestionRevisionTuple, Timestamp};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    authoring_source::{load_verified_source, put_workspace_source, validated_source},
    question_publication::{
        AuthoringAssetContext, ExistingQuestionRevisionPublicationCommand,
        ExistingQuestionRevisionPublisher, NewQuestionLineagePublicationCommand,
        NewQuestionLineagePublisher, RandomQuestionIdIssuer,
    },
};

mod http;
use http::{
    edit_number_response, etag, existing_parent_question_revision, is_ple_question_json_request,
    publication_error, question_authorship,
};
pub(crate) use http::{
    expected_edit_number, instructor_session_hash, parse_draft_question_uuid, private_store_error,
};

pub(crate) const PLE_QUESTION_JSON_MEDIA_TYPE: &str = "application/vnd.peptidyle.question+json";
const INITIAL_PUBLICATION_REASON: &str = "Initial publication from Authoring Workspace";

#[derive(Clone)]
pub(crate) struct AuthoringRouteState {
    pub(crate) sessions: Arc<PostgresSessionStore>,
    pub(crate) drafts: PostgresAuthoringDraftStore,
    pub(crate) assets: Arc<PostgresAuthoringAssetsStore>,
    publication: PostgresDraftQuestionSourceBindingStore,
    pub(crate) objects: S3ObjectStore,
    question_id_issuer: RandomQuestionIdIssuer,
}

/// Registers private Authoring Workspace routes and the initial publication operation.
pub fn authoring_router(
    sessions: Arc<PostgresSessionStore>,
    drafts: PostgresAuthoringDraftStore,
    assets: PostgresAuthoringAssetsStore,
    publication: PostgresDraftQuestionSourceBindingStore,
    objects: S3ObjectStore,
    question_id_issuer: RandomQuestionIdIssuer,
) -> Router {
    Router::new()
        .route("/api/authoring/drafts", get(list_drafts).post(create_draft))
        .route(
            "/api/authoring/drafts/{draft_question_id}",
            delete(delete_draft),
        )
        .route(
            "/api/authoring/drafts/{draft_question_id}/assets",
            post(crate::authoring_assets::upload).layer(DefaultBodyLimit::max(
                objects::image_validation::MAX_STILL_IMAGE_BYTES,
            )),
        )
        .route(
            "/api/authoring/drafts/{draft_question_id}/assets/{asset}",
            get(crate::authoring_assets::preview),
        )
        .route(
            "/api/authoring/drafts/{draft_question_id}/source",
            get(load_source).put(save_source),
        )
        .route(
            "/api/authoring/drafts/{draft_question_id}/metadata",
            get(load_general_feedback).put(save_general_feedback),
        )
        .route(
            "/api/authoring/drafts/{draft_question_id}/publish",
            post(publish_draft),
        )
        .route(
            "/api/authoring/drafts/{draft_question_id}/publish-revision",
            post(publish_revision_draft),
        )
        .with_state(AuthoringRouteState {
            sessions,
            drafts,
            assets: Arc::new(assets),
            publication,
            objects,
            question_id_issuer,
        })
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct DraftSummaryResponse {
    draft_question: Uuid,
    edit_number: u64,
    question_title: String,
    question_description: String,
}

#[derive(Debug, Serialize)]
struct DraftListResponse {
    items: Vec<DraftSummaryResponse>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CreatedDraftResponse {
    draft_question: Uuid,
    edit_number: u64,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct DraftGeneralFeedbackRequest {
    general_feedback: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PublishDraftRequest {
    authors: Vec<String>,
    discipline_uuid: String,
    subject_uuid: String,
    topic_uuid: Option<String>,
    subtopic_uuid: Option<String>,
}

fn canonical_classification_uuid(value: &str) -> Result<uuid::Uuid, ()> {
    let parsed = uuid::Uuid::parse_str(value).map_err(|_| ())?;
    (parsed.to_string() == value).then_some(parsed).ok_or(())
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PublishedDraftResponse {
    question_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct PublishRevisionDraftRequest {
    question_id: String,
    parent_revision_number: u32,
    reason_for_edit: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PublishedRevisionDraftResponse {
    question_revision_tuple: QuestionRevisionTuple,
}

async fn list_drafts(State(state): State<AuthoringRouteState>, headers: HeaderMap) -> Response {
    let session_hash = match instructor_session_hash(&state, &headers).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    match state.drafts.list_authoring_drafts(session_hash).await {
        Ok(drafts) => crate::auth::no_store(
            Json(DraftListResponse {
                items: drafts
                    .into_iter()
                    .map(|draft| DraftSummaryResponse {
                        draft_question: draft.draft_question_uuid.as_uuid(),
                        edit_number: draft.edit_number.as_postgres_bigint() as u64,
                        question_title: draft.title,
                        question_description: draft.description,
                    })
                    .collect(),
            })
            .into_response(),
        ),
        Err(error) => private_store_error(error),
    }
}

async fn create_draft(
    State(state): State<AuthoringRouteState>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    if !is_ple_question_json_request(&headers) {
        return private_error(
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            "Draft Question source media type is required",
        );
    }
    let session_hash = match instructor_session_hash(&state, &headers).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let source = match validated_source(&body) {
        Ok(source) => source,
        Err(response) => return *response,
    };
    if source.hotspot_surface.is_some() {
        return private_error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Create a Draft Question before uploading its HOTSPOT image",
        );
    }
    let workspace = match state
        .drafts
        .ensure_own_authoring_workspace(session_hash, Uuid::now_v7())
        .await
    {
        Ok(workspace) => workspace,
        Err(error) => return private_store_error(error),
    };
    let source_record =
        match put_workspace_source(&state.objects, workspace, source.bytes.clone()).await {
            Ok(record) => record,
            Err(()) => {
                return private_error(
                    StatusCode::SERVICE_UNAVAILABLE,
                    "Authoring storage is unavailable",
                );
            }
        };
    let created = state
        .drafts
        .create_authoring_draft(
            session_hash,
            workspace,
            CreateAuthoringDraftInput {
                draft_question_uuid: DraftQuestionUuid::from_uuid(Uuid::now_v7()),
                source_record,
                question_format: QuestionFormat::PleQuestionJson,
                webwork_pg_path: None,
                question_type: source.question_type,
                title: source.title,
                description: source.description,
                language: source.language,
            },
        )
        .await;
    match created {
        Ok(draft) => crate::auth::no_store(
            (
                StatusCode::CREATED,
                Json(CreatedDraftResponse {
                    draft_question: draft.draft_question_uuid.as_uuid(),
                    edit_number: draft.edit_number.as_postgres_bigint() as u64,
                }),
            )
                .into_response(),
        ),
        Err(error) => private_store_error(error),
    }
}

async fn delete_draft(
    State(state): State<AuthoringRouteState>,
    headers: HeaderMap,
    Path(draft_question_id): Path<String>,
) -> Response {
    let draft_question_uuid = match parse_draft_question_uuid(&draft_question_id) {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let expected_edit_number = match expected_edit_number(&headers) {
        Ok(number) => number,
        Err(response) => return *response,
    };
    let session_hash = match instructor_session_hash(&state, &headers).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    match state
        .drafts
        .delete_authoring_draft(
            session_hash,
            DeleteAuthoringDraftInput {
                draft_question_uuid,
                expected_edit_number,
            },
        )
        .await
    {
        Ok(()) => crate::auth::no_store(StatusCode::NO_CONTENT.into_response()),
        Err(StoreError::Conflict | StoreError::RetryableTransaction) => private_error(
            StatusCode::PRECONDITION_FAILED,
            "Draft Question changed before deletion",
        ),
        Err(error) => private_store_error(error),
    }
}

async fn load_source(
    State(state): State<AuthoringRouteState>,
    headers: HeaderMap,
    Path(draft_question_id): Path<String>,
) -> Response {
    let draft_question_uuid = match parse_draft_question_uuid(&draft_question_id) {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let session_hash = match instructor_session_hash(&state, &headers).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let draft = match state
        .drafts
        .load_authoring_draft(session_hash, draft_question_uuid)
        .await
    {
        Ok(draft) => draft,
        Err(error) => return private_store_error(error),
    };
    let source = match load_verified_source(&state.objects, &draft).await {
        Ok(bytes) => bytes,
        Err(()) => {
            return private_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "Authoring storage is unavailable",
            );
        }
    };
    let mut response = crate::auth::no_store(source.into_response());
    response.headers_mut().insert(
        CONTENT_TYPE,
        HeaderValue::from_static(PLE_QUESTION_JSON_MEDIA_TYPE),
    );
    match etag(draft.edit_number) {
        Ok(value) => response.headers_mut().insert(ETAG, value),
        Err(()) => {
            return private_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "Authoring draft is unavailable",
            );
        }
    };
    response
}

async fn save_source(
    State(state): State<AuthoringRouteState>,
    headers: HeaderMap,
    Path(draft_question_id): Path<String>,
    body: Bytes,
) -> Response {
    if !is_ple_question_json_request(&headers) {
        return private_error(
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            "Draft Question source media type is required",
        );
    }
    let draft_question_uuid = match parse_draft_question_uuid(&draft_question_id) {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let expected_edit_number = match expected_edit_number(&headers) {
        Ok(number) => number,
        Err(response) => return *response,
    };
    let session_hash = match instructor_session_hash(&state, &headers).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let source = match validated_source(&body) {
        Ok(source) => source,
        Err(response) => return *response,
    };
    let current = match state
        .drafts
        .load_authoring_draft(session_hash, draft_question_uuid)
        .await
    {
        Ok(draft) => draft,
        Err(error) => return private_store_error(error),
    };
    if current.edit_number != expected_edit_number {
        return private_error(
            StatusCode::PRECONDITION_FAILED,
            "Draft Question changed before this save",
        );
    }
    if let Some(surface) = &source.hotspot_surface
        && let Err(error) = crate::authoring_assets::require_surface(
            state.assets.as_ref(),
            session_hash,
            draft_question_uuid,
            surface,
        )
        .await
    {
        return private_store_error(error);
    }
    let source_record =
        match put_workspace_source(&state.objects, current.workspace, source.bytes.clone()).await {
            Ok(record) => record,
            Err(()) => {
                return private_error(
                    StatusCode::SERVICE_UNAVAILABLE,
                    "Authoring storage is unavailable",
                );
            }
        };
    match state
        .drafts
        .save_authoring_draft(
            session_hash,
            SaveAuthoringDraftInput {
                draft_question_uuid,
                expected_edit_number,
                source_record,
                question_type: source.question_type,
                title: source.title,
                description: source.description,
                language: source.language,
                hotspot_surface: source.hotspot_surface,
            },
        )
        .await
    {
        Ok(draft) => edit_number_response(draft.edit_number),
        Err(StoreError::Conflict | StoreError::RetryableTransaction) => private_error(
            StatusCode::PRECONDITION_FAILED,
            "Draft Question changed before this save",
        ),
        Err(StoreError::LifecycleConflict) => {
            private_error(StatusCode::CONFLICT, "Draft Question lifecycle conflict")
        }
        Err(error) => private_store_error(error),
    }
}

async fn load_general_feedback(
    State(state): State<AuthoringRouteState>,
    headers: HeaderMap,
    Path(draft_question_id): Path<String>,
) -> Response {
    let draft_question_uuid = match parse_draft_question_uuid(&draft_question_id) {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let session_hash = match instructor_session_hash(&state, &headers).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    match state
        .drafts
        .load_authoring_draft(session_hash, draft_question_uuid)
        .await
    {
        Ok(draft) => match etag(draft.edit_number) {
            Ok(etag) => {
                let mut response = crate::auth::no_store(
                    Json(DraftGeneralFeedbackRequest {
                        general_feedback: draft.general_feedback,
                    })
                    .into_response(),
                );
                response.headers_mut().insert(ETAG, etag);
                response
            }
            Err(()) => private_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "Authoring draft is unavailable",
            ),
        },
        Err(error) => private_store_error(error),
    }
}

async fn save_general_feedback(
    State(state): State<AuthoringRouteState>,
    headers: HeaderMap,
    Path(draft_question_id): Path<String>,
    Json(request): Json<DraftGeneralFeedbackRequest>,
) -> Response {
    let draft_question_uuid = match parse_draft_question_uuid(&draft_question_id) {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let expected_edit_number = match expected_edit_number(&headers) {
        Ok(number) => number,
        Err(response) => return *response,
    };
    let session_hash = match instructor_session_hash(&state, &headers).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    match state
        .drafts
        .save_authoring_draft_general_feedback(
            session_hash,
            SaveAuthoringDraftGeneralFeedbackInput {
                draft_question_uuid,
                expected_edit_number,
                general_feedback: request.general_feedback,
            },
        )
        .await
    {
        Ok(draft) => edit_number_response(draft.edit_number),
        Err(StoreError::Conflict | StoreError::RetryableTransaction) => private_error(
            StatusCode::PRECONDITION_FAILED,
            "Draft Question changed before this save",
        ),
        Err(error) => private_store_error(error),
    }
}

async fn publish_draft(
    State(state): State<AuthoringRouteState>,
    headers: HeaderMap,
    Path(draft_question_id): Path<String>,
    Json(request): Json<PublishDraftRequest>,
) -> Response {
    // ASVS 2.2.1/2.2.2: canonical UUID input is checked at the trusted boundary.
    let classification = (|| {
        let discipline = canonical_classification_uuid(&request.discipline_uuid)?;
        let subject = canonical_classification_uuid(&request.subject_uuid)?;
        let topic = request
            .topic_uuid
            .as_deref()
            .map(canonical_classification_uuid)
            .transpose()?;
        let subtopic = request
            .subtopic_uuid
            .as_deref()
            .map(canonical_classification_uuid)
            .transpose()?;
        Ok::<_, ()>((discipline, subject, topic, subtopic))
    })();
    let Ok((discipline_uuid, subject_uuid, topic_uuid, subtopic_uuid)) = classification else {
        return private_error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Question classification is invalid",
        );
    };
    let draft_question_uuid = match parse_draft_question_uuid(&draft_question_id) {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let expected_edit_number = match expected_edit_number(&headers) {
        Ok(number) => number,
        Err(response) => return *response,
    };
    let session_hash = match instructor_session_hash(&state, &headers).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let draft = match state
        .drafts
        .load_authoring_draft(session_hash, draft_question_uuid)
        .await
    {
        Ok(draft) => draft,
        Err(error) => return private_store_error(error),
    };
    let bytes = match load_verified_source(&state.objects, &draft).await {
        Ok(bytes) => bytes,
        Err(()) => {
            return private_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "Authoring storage is unavailable",
            );
        }
    };
    let source = match validated_source(&bytes) {
        Ok(source) => source,
        Err(_) => {
            return private_error(
                StatusCode::UNPROCESSABLE_ENTITY,
                "Draft Question cannot be published",
            );
        }
    };
    if source.question_type != draft.question_type {
        return private_error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Draft Question type declaration does not match its PLE source",
        );
    }
    let authorship = match question_authorship(request.authors) {
        Ok(authorship) => authorship,
        Err(()) => {
            return private_error(
                StatusCode::BAD_REQUEST,
                "Question Publication requires reviewed authors",
            );
        }
    };
    let Some(question_license) = source.license else {
        return private_error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Draft Question requires a Question License before publication",
        );
    };
    let command = NewQuestionLineagePublicationCommand {
        draft_question_uuid: draft.draft_question_uuid,
        expected_draft_question_edit_number: expected_edit_number,
        workspace: draft.workspace,
        question_authorship: authorship,
        initial_shared_tags: source.tags,
        discipline_uuid,
        subject_uuid,
        topic_uuid,
        subtopic_uuid,
        question_license,
        question_revision_reason: QuestionRevisionReason::new(
            INITIAL_PUBLICATION_REASON.to_string(),
        )
        .expect("fixed initial publication reason is valid"),
    };
    let publisher = NewQuestionLineagePublisher::new(
        state.objects.clone(),
        state.publication.clone(),
        state.question_id_issuer,
        Some(AuthoringAssetContext {
            store: state.assets.clone(),
            draft_question_uuid,
        }),
    );
    match publisher.publish(session_hash, command, now()).await {
        Ok(revision) => crate::auth::no_store(
            Json(PublishedDraftResponse {
                question_id: revision.question_id.to_string(),
            })
            .into_response(),
        ),
        Err(error) => publication_error(error),
    }
}

async fn publish_revision_draft(
    State(state): State<AuthoringRouteState>,
    headers: HeaderMap,
    Path(draft_question_id): Path<String>,
    Json(request): Json<PublishRevisionDraftRequest>,
) -> Response {
    let draft_question_uuid = match parse_draft_question_uuid(&draft_question_id) {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let expected_edit_number = match expected_edit_number(&headers) {
        Ok(number) => number,
        Err(response) => return *response,
    };
    let parent_question_revision_tuple = match existing_parent_question_revision(
        request.question_id,
        request.parent_revision_number,
    ) {
        Ok(revision) => revision,
        Err(()) => return concealed(),
    };
    let revision_reason = match QuestionRevisionReason::new(request.reason_for_edit) {
        Ok(reason) => reason,
        Err(_) => {
            return private_error(
                StatusCode::UNPROCESSABLE_ENTITY,
                "Question Revision requires a reviewed reason for edit",
            );
        }
    };
    let session_hash = match instructor_session_hash(&state, &headers).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let draft = match state
        .drafts
        .load_authoring_draft(session_hash, draft_question_uuid)
        .await
    {
        Ok(draft) => draft,
        Err(error) => return private_store_error(error),
    };
    let bytes = match load_verified_source(&state.objects, &draft).await {
        Ok(bytes) => bytes,
        Err(()) => {
            return private_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "Authoring storage is unavailable",
            );
        }
    };
    let source = match validated_source(&bytes) {
        Ok(source) => source,
        Err(_) => {
            return private_error(
                StatusCode::UNPROCESSABLE_ENTITY,
                "Draft Question cannot be published",
            );
        }
    };
    if source.question_type != draft.question_type {
        return private_error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Draft Question type declaration does not match its PLE source",
        );
    }
    let publisher = ExistingQuestionRevisionPublisher::new(
        state.objects.clone(),
        state.publication.clone(),
        Some(AuthoringAssetContext {
            store: state.assets.clone(),
            draft_question_uuid,
        }),
    );
    match publisher
        .publish(
            session_hash,
            ExistingQuestionRevisionPublicationCommand {
                draft_question_uuid: draft.draft_question_uuid,
                expected_draft_question_edit_number: expected_edit_number,
                workspace: draft.workspace,
                parent_question_revision_tuple,
                question_revision_reason: revision_reason,
            },
            now(),
        )
        .await
    {
        Ok(question_revision_tuple) => crate::auth::no_store(
            Json(PublishedRevisionDraftResponse {
                question_revision_tuple,
            })
            .into_response(),
        ),
        Err(error) => publication_error(error),
    }
}

pub(crate) fn concealed() -> Response {
    crate::auth::no_store(
        (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "Not found" })),
        )
            .into_response(),
    )
}

pub(crate) fn private_error(status: StatusCode, message: &'static str) -> Response {
    crate::auth::no_store((status, Json(serde_json::json!({ "error": message }))).into_response())
}

pub(crate) fn now() -> Timestamp {
    let milliseconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default();
    Timestamp::from_unix_millis(i64::try_from(milliseconds).unwrap_or(i64::MAX))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::question_publication::QuestionIdIssuer;
    use question_model::QuestionRevisionNumber;

    #[test]
    fn same_lineage_publication_requires_a_server_validated_positive_parent_revision() {
        let issuer = RandomQuestionIdIssuer::new();
        let question_id = issuer.issue_question_id().expect("issued Question ID");

        assert_eq!(
            existing_parent_question_revision(question_id.to_string(), 1),
            Ok(QuestionRevisionTuple {
                question_id: question_id.clone(),
                revision_number: QuestionRevisionNumber::new(1)
                    .expect("positive Question Revision Number"),
            })
        );
        assert!(existing_parent_question_revision(question_id.to_string(), 0).is_err());
        assert!(existing_parent_question_revision("0000-X000".to_string(), 1).is_err());
    }
}
