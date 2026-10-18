//! Private Draft Question authoring and first-publication Server Routes.
//!
//! An authorized private Draft UUID and its Edit Number cross the browser
//! boundary. Workspace UUIDs, source object addresses, and publication copy
//! inputs remain server-only.

use std::{
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use adapter_webwork::{HttpWebworkRenderer, WebworkAdapter};
use axum::{
    Json, Router,
    body::Bytes,
    extract::{DefaultBodyLimit, Path, State},
    http::{HeaderMap, StatusCode, header::ETAG},
    response::{IntoResponse, Response},
    routing::{delete, get, post},
};
use learning_data_access::{
    AuthoringDraft, AuthoringDraftStore, DeleteAuthoringDraftInput,
    SaveAuthoringDraftGeneralFeedbackInput, SaveAuthoringDraftInput, StoreError,
    postgres::{
        PostgresAuthoringDraftStore, PostgresDraftQuestionImageStore,
        PostgresDraftQuestionSourceBindingStore, PostgresSessionStore,
    },
};
use objects::s3::S3ObjectStore;
use question_model::{
    PublishedQuestionRevisionTuple, QuestionAuthorDisplayName, QuestionBackend, QuestionMetadata,
    QuestionType, Timestamp,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    authoring_source::{
        inspect_source, load_verified_source, matches_source_content_type, put_workspace_source,
        source_response_headers,
    },
    question_publication::{QuestionThumbnailCaptureClient, RandomQuestionIdIssuer},
};

mod create;
mod http;
mod publication;
pub(crate) use http::{
    authoring_session_hash, expected_edit_number, parse_draft_question_uuid, private_store_error,
};
use http::{edit_number_response, etag};

pub(crate) const PLE_QUESTION_JSON_MEDIA_TYPE: &str = "application/vnd.peptidyle.question+json";
const INITIAL_PUBLICATION_REASON: &str = "Initial publication from Authoring Workspace";

#[derive(Clone)]
pub(crate) struct AuthoringRouteState {
    pub(crate) sessions: Arc<PostgresSessionStore>,
    pub(crate) drafts: PostgresAuthoringDraftStore,
    pub(crate) draft_question_images: Arc<PostgresDraftQuestionImageStore>,
    publication: PostgresDraftQuestionSourceBindingStore,
    pub(crate) objects: S3ObjectStore,
    webwork: Arc<WebworkAdapter<HttpWebworkRenderer>>,
    thumbnail_capture: QuestionThumbnailCaptureClient,
    publication_objects: S3ObjectStore,
    question_id_issuer: RandomQuestionIdIssuer,
}

/// Registers private Authoring Workspace routes and the initial publication operation.
#[allow(clippy::too_many_arguments)] // explicit existing service owners plus private capture
pub fn authoring_router(
    sessions: Arc<PostgresSessionStore>,
    drafts: PostgresAuthoringDraftStore,
    draft_question_images: PostgresDraftQuestionImageStore,
    publication: PostgresDraftQuestionSourceBindingStore,
    objects: S3ObjectStore,
    webwork: Arc<WebworkAdapter<HttpWebworkRenderer>>,
    thumbnail_capture: QuestionThumbnailCaptureClient,
    question_id_issuer: RandomQuestionIdIssuer,
) -> Router {
    Router::new()
        .route(
            "/api/authoring/drafts",
            get(list_drafts).merge(create::route()),
        )
        .route(
            "/api/authoring/drafts/{draft_question_id}",
            delete(delete_draft),
        )
        .route(
            "/api/authoring/drafts/{draft_question_id}/images",
            post(crate::draft_question_images::upload).layer(DefaultBodyLimit::max(
                objects::image_validation::MAX_STILL_IMAGE_BYTES,
            )),
        )
        .route(
            "/api/authoring/drafts/{draft_question_id}/images/{question_image_asset_id}",
            get(crate::draft_question_images::preview),
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
            post(publication::publish_draft),
        )
        .route(
            "/api/authoring/drafts/{draft_question_id}/publish-revision",
            post(publication::publish_revision_draft),
        )
        .with_state(AuthoringRouteState {
            sessions,
            drafts,
            draft_question_images: Arc::new(draft_question_images),
            publication,
            publication_objects: objects.with_operation_timeout(std::time::Duration::from_secs(5)),
            objects,
            webwork,
            thumbnail_capture,
            question_id_issuer,
        })
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct DraftSummaryResponse {
    draft_question_id: Uuid,
    draft_question_edit_number: String,
    question_title: String,
    question_description: String,
    parent_published_question_revision_tuple:
        Option<question_model::PublishedQuestionRevisionTuple>,
}

#[derive(Debug, Serialize)]
struct DraftListResponse {
    items: Vec<DraftSummaryResponse>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CreatedDraftResponse {
    draft_question_id: Uuid,
    draft_question_edit_number: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct DraftGeneralFeedbackResponse {
    metadata: QuestionMetadata,
    question_type: Option<QuestionType>,
    general_feedback: Option<String>,
    hint: Option<String>,
    worked_solution: Option<String>,
    authors: Vec<QuestionAuthorDisplayName>,
    parent_published_question_revision_tuple: Option<PublishedQuestionRevisionTuple>,
}

/// A missing key stays `None`. A present JSON null becomes `Some(None)` so a
/// clear is distinct from "leave the stored text unchanged". ASVS 2.2.1.
fn present_optional_text<'de, D>(deserializer: D) -> Result<Option<Option<String>>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Ok(Some(Option::<String>::deserialize(deserializer)?))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct DraftGeneralFeedbackRequest {
    metadata: QuestionMetadata,
    #[serde(default, deserialize_with = "present_optional_question_type")]
    question_type: Option<Option<QuestionType>>,
    general_feedback: Option<String>,
    #[serde(default, deserialize_with = "present_optional_text")]
    hint: Option<Option<String>>,
    #[serde(default, deserialize_with = "present_optional_text")]
    worked_solution: Option<Option<String>>,
}

fn present_optional_question_type<'de, D>(
    deserializer: D,
) -> Result<Option<Option<QuestionType>>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Ok(Some(Option::<QuestionType>::deserialize(deserializer)?))
}

fn allows_question_type_write(
    question_backend: QuestionBackend,
    requested_question_type: Option<Option<QuestionType>>,
) -> bool {
    requested_question_type.is_none() || question_backend == QuestionBackend::Webwork
}

/// Omitted Hint and Worked Solution leave the stored texts unchanged.
/// Both keys together replace them. One key alone is rejected. ASVS 2.2.1, 8.2.3.
fn authored_support_replacement(
    hint: Option<Option<String>>,
    worked_solution: Option<Option<String>>,
) -> Result<(Option<String>, Option<String>, bool), ()> {
    match (hint, worked_solution) {
        (None, None) => Ok((None, None, false)),
        (Some(hint), Some(worked_solution)) => Ok((hint, worked_solution, true)),
        _ => Err(()),
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PublishDraftRequest {
    authors: Vec<String>,
    self_author_position: Option<usize>,
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
    published_question_revision_tuple: PublishedQuestionRevisionTuple,
}

async fn list_drafts(State(state): State<AuthoringRouteState>, headers: HeaderMap) -> Response {
    let session_hash = match authoring_session_hash(&state, &headers).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    match state.drafts.list_authoring_drafts(session_hash).await {
        Ok(drafts) => crate::auth::no_store(
            Json(DraftListResponse {
                items: drafts
                    .into_iter()
                    .map(|draft| DraftSummaryResponse {
                        draft_question_id: draft.draft_question_uuid.as_uuid(),
                        draft_question_edit_number: draft
                            .edit_number
                            .as_postgres_bigint()
                            .to_string(),
                        question_title: draft.metadata.question_title,
                        question_description: draft.metadata.question_description,
                        parent_published_question_revision_tuple: draft
                            .parent_published_question_revision_tuple,
                    })
                    .collect(),
            })
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
    let session_hash = match authoring_session_hash(&state, &headers).await {
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
    let session_hash = match authoring_session_hash(&state, &headers).await {
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
    let source_headers = match source_response_headers(&draft) {
        Ok(headers) => headers,
        Err(()) => {
            return private_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "Authoring draft is unavailable",
            );
        }
    };
    response.headers_mut().extend(source_headers);
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
    let draft_question_uuid = match parse_draft_question_uuid(&draft_question_id) {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let expected_edit_number = match expected_edit_number(&headers) {
        Ok(number) => number,
        Err(response) => return *response,
    };
    let session_hash = match authoring_session_hash(&state, &headers).await {
        Ok(value) => value,
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
    let Some(media_type) = current.source_media_type() else {
        return private_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "Authoring draft is unavailable",
        );
    };
    if !matches_source_content_type(&headers, media_type) {
        return private_error(
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            "Draft Question source media type does not match its registered binding",
        );
    }
    let question_type = inspect_source(&body, current.question_backend);
    let source_record =
        match put_workspace_source(&state.objects, current.workspace, body.to_vec(), media_type)
            .await
        {
            Ok(record) => record,
            Err(()) => {
                return private_error(
                    StatusCode::SERVICE_UNAVAILABLE,
                    "Authoring storage is unavailable",
                );
            }
        };
    let question_type = match current.question_backend {
        QuestionBackend::Ple => question_type,
        QuestionBackend::Webwork => current.question_type,
        QuestionBackend::Imathas => None,
    };
    match state
        .drafts
        .save_authoring_draft(
            session_hash,
            SaveAuthoringDraftInput {
                draft_question_uuid,
                expected_edit_number,
                source_record,
                question_type,
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
    let session_hash = match authoring_session_hash(&state, &headers).await {
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
                    Json(DraftGeneralFeedbackResponse {
                        metadata: draft.metadata,
                        question_type: draft.question_type,
                        general_feedback: draft.general_feedback,
                        hint: draft.hint,
                        worked_solution: draft.worked_solution,
                        authors: draft.authors,
                        parent_published_question_revision_tuple: draft
                            .parent_published_question_revision_tuple,
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
    let session_hash = match authoring_session_hash(&state, &headers).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    if request.question_type.is_some() {
        let draft = match state
            .drafts
            .load_authoring_draft(session_hash, draft_question_uuid)
            .await
        {
            Ok(draft) => draft,
            Err(error) => return private_store_error(error),
        };
        if draft.edit_number != expected_edit_number {
            return private_error(
                StatusCode::PRECONDITION_FAILED,
                "Draft Question changed before this save",
            );
        }
        if !allows_question_type_write(draft.question_backend, request.question_type) {
            return private_error(
                StatusCode::UNPROCESSABLE_ENTITY,
                "Question Type is source-derived for Native Drafts",
            );
        }
    }
    // ASVS 2.2.1: general feedback alone must not clear Hint or Worked Solution.
    // ASVS 8.2.3: these texts are authored fields, not backend source.
    let (hint, worked_solution, replace_support) =
        match authored_support_replacement(request.hint, request.worked_solution) {
            Ok(replacement) => replacement,
            Err(()) => {
                return private_error(
                    StatusCode::UNPROCESSABLE_ENTITY,
                    "Hint and Worked Solution must be saved together",
                );
            }
        };
    match state
        .drafts
        .save_authoring_draft_general_feedback_with_question_type(
            session_hash,
            SaveAuthoringDraftGeneralFeedbackInput {
                draft_question_uuid,
                expected_edit_number,
                metadata: request.metadata,
                general_feedback: request.general_feedback,
                hint,
                worked_solution,
                replace_support,
            },
            request.question_type,
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
