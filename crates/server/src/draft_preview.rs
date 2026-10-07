//! Private, transient rendering and grading for the exact current Draft source.

use std::sync::Arc;

use adapter_webwork::{HttpWebworkRenderer, WebworkAdapter};
use axum::{
    Json, Router,
    body::Bytes,
    extract::{DefaultBodyLimit, Path, Query, State},
    http::{
        HeaderMap, StatusCode,
        header::{CONTENT_TYPE, COOKIE},
    },
    response::{IntoResponse, Response},
    routing::{get, post},
};
use learning_data_access::{
    AuthoringDraft, AuthoringDraftStore, DraftQuestionEditNumber, DraftQuestionUuid,
    SessionTokenHash, StoreError,
    postgres::{PostgresAuthoringDraftStore, PostgresSessionStore},
};
use objects::s3::S3ObjectStore;
use question_model::{
    QuestionBackend, QuestionFormat, QuestionResponseFormat, StudentResponse, UserRole,
    generation::QuestionSeed,
};
use serde::{Deserialize, Serialize};

use crate::{
    auth::{AuthError, resolve_session},
    authoring_source::load_verified_source,
};

const MAX_DRAFT_TEST_BYTES: usize = 96 * 1024;

#[derive(Clone)]
struct RouteState {
    sessions: Arc<PostgresSessionStore>,
    drafts: PostgresAuthoringDraftStore,
    operations: Arc<dyn DraftQuestionPreviewOperations>,
}

/// Registers read-only Draft preview and transient test routes.
pub fn draft_preview_router(
    sessions: Arc<PostgresSessionStore>,
    drafts: PostgresAuthoringDraftStore,
    objects: S3ObjectStore,
    webwork: Arc<WebworkAdapter<HttpWebworkRenderer>>,
) -> Router {
    let operations = Arc::new(StoredDraftQuestionPreviewOperations {
        drafts: drafts.clone(),
        objects,
        webwork,
    });
    Router::new()
        .route(
            "/api/authoring/drafts/{draft_question_id}/preview",
            get(preview_draft),
        )
        .route(
            "/api/authoring/drafts/{draft_question_id}/test",
            post(test_draft).layer(DefaultBodyLimit::max(MAX_DRAFT_TEST_BYTES)),
        )
        .with_state(RouteState {
            sessions,
            drafts,
            operations,
        })
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct DraftPreviewQuery {
    draft_question_edit_number: String,
    #[serde(default)]
    seed: Option<u64>,
    #[serde(default, rename = "draftTest")]
    _draft_test: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct DraftTestRequest {
    #[serde(default)]
    seed: Option<u64>,
    response: StudentResponse,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct NativeDraftPreview {
    question_title: String,
    prompt: Vec<question_model::QuestionContentBlock>,
    response: QuestionResponseFormat,
}

#[derive(Debug, Serialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
enum DraftTestResponse {
    Evaluated {
        correct: bool,
        normalized_credit: f64,
    },
    Ungraded {},
}

enum DraftPreviewOutput {
    Native(NativeDraftPreview),
    Webwork(Vec<u8>),
}

/// Draft preview/test inputs contain no Published tuple or Student Work identity.
#[derive(Debug, Clone)]
struct CurrentDraftSource {
    workspace: question_model::WorkspaceId,
    question_title: String,
    backend: QuestionBackend,
    format: QuestionFormat,
    webwork_pg_path: Option<String>,
    bytes: Vec<u8>,
}

#[derive(Debug)]
enum DraftPreviewError {
    NotFound,
    Conflict,
    UnsupportedBackend,
    InvalidSource,
    Unavailable,
}

#[async_trait::async_trait]
trait DraftQuestionPreviewOperations: Send + Sync {
    async fn preview(
        &self,
        session: SessionTokenHash,
        draft_question_uuid: DraftQuestionUuid,
        expected_edit_number: DraftQuestionEditNumber,
        seed: Option<QuestionSeed>,
    ) -> Result<DraftPreviewOutput, DraftPreviewError>;

    async fn test(
        &self,
        session: SessionTokenHash,
        draft_question_uuid: DraftQuestionUuid,
        expected_edit_number: DraftQuestionEditNumber,
        seed: Option<QuestionSeed>,
        response: StudentResponse,
    ) -> Result<DraftTestResponse, DraftPreviewError>;
}

struct StoredDraftQuestionPreviewOperations {
    drafts: PostgresAuthoringDraftStore,
    objects: S3ObjectStore,
    webwork: Arc<WebworkAdapter<HttpWebworkRenderer>>,
}

impl StoredDraftQuestionPreviewOperations {
    async fn current_source(
        &self,
        session: SessionTokenHash,
        draft_question_uuid: DraftQuestionUuid,
        expected_edit_number: DraftQuestionEditNumber,
    ) -> Result<CurrentDraftSource, DraftPreviewError> {
        let draft = self
            .drafts
            .load_authoring_draft(session, draft_question_uuid)
            .await
            .map_err(map_store_error)?;
        if draft.edit_number != expected_edit_number {
            return Err(DraftPreviewError::Conflict);
        }
        let bytes = load_verified_source(&self.objects, &draft)
            .await
            .map_err(|_| DraftPreviewError::Unavailable)?;
        current_draft_source(draft, bytes.to_vec())
    }

    async fn preview_current(
        &self,
        source: CurrentDraftSource,
        seed: Option<QuestionSeed>,
    ) -> Result<DraftPreviewOutput, DraftPreviewError> {
        match select_backend(
            source.backend,
            source.format,
            source.webwork_pg_path.as_deref(),
        )? {
            DraftPreviewBackend::Native => {
                if seed.is_some() {
                    return Err(DraftPreviewError::InvalidSource);
                }
                let document =
                    adapter_ple::question_json::PleQuestionJsonDocument::parse(&source.bytes)
                        .map_err(|_| DraftPreviewError::InvalidSource)?;
                let compiled = document
                    .compile()
                    .map_err(|_| DraftPreviewError::InvalidSource)?;
                let preview = domain::draft_preview::preview_ple_draft(
                    &domain::draft_preview::DraftPreviewRequest {
                        workspace: source.workspace,
                        question_backend: source.backend,
                        question_title: source.question_title,
                        prompt: compiled.presentation().prompt().to_vec(),
                        response: compiled.presentation().response().clone(),
                    },
                );
                match preview {
                    domain::draft_preview::DraftPreviewResult::Ready { preview } => {
                        Ok(DraftPreviewOutput::Native(NativeDraftPreview {
                            question_title: preview.question_title,
                            prompt: preview.prompt,
                            response: preview.response,
                        }))
                    }
                    domain::draft_preview::DraftPreviewResult::Unavailable { .. } => {
                        Err(DraftPreviewError::UnsupportedBackend)
                    }
                }
            }
            DraftPreviewBackend::Webwork { pg_path } => {
                let seed = seed.ok_or(DraftPreviewError::InvalidSource)?;
                let document = self
                    .webwork
                    .preview_draft_document(seed, &source.bytes, pg_path)
                    .await
                    .map_err(|_| DraftPreviewError::Unavailable)?;
                Ok(DraftPreviewOutput::Webwork(document))
            }
        }
    }
}

#[async_trait::async_trait]
impl DraftQuestionPreviewOperations for StoredDraftQuestionPreviewOperations {
    async fn preview(
        &self,
        session: SessionTokenHash,
        draft_question_uuid: DraftQuestionUuid,
        expected_edit_number: DraftQuestionEditNumber,
        seed: Option<QuestionSeed>,
    ) -> Result<DraftPreviewOutput, DraftPreviewError> {
        let source = self
            .current_source(session, draft_question_uuid, expected_edit_number)
            .await?;
        self.preview_current(source, seed).await
    }

    async fn test(
        &self,
        session: SessionTokenHash,
        draft_question_uuid: DraftQuestionUuid,
        expected_edit_number: DraftQuestionEditNumber,
        seed: Option<QuestionSeed>,
        response: StudentResponse,
    ) -> Result<DraftTestResponse, DraftPreviewError> {
        let source = self
            .current_source(session, draft_question_uuid, expected_edit_number)
            .await?;
        let outcome = match select_backend(
            source.backend,
            source.format,
            source.webwork_pg_path.as_deref(),
        )? {
            DraftPreviewBackend::Native => {
                if seed.is_some() {
                    return Err(DraftPreviewError::InvalidSource);
                }
                let document =
                    adapter_ple::question_json::PleQuestionJsonDocument::parse(&source.bytes)
                        .map_err(|_| DraftPreviewError::InvalidSource)?;
                let compiled = document
                    .compile()
                    .map_err(|_| DraftPreviewError::InvalidSource)?;
                let evaluation = compiled
                    .private()
                    .evaluate(
                        compiled.private().public_content_checksum(),
                        compiled.presentation().question_type(),
                        compiled.presentation().response(),
                        &response,
                    )
                    .map_err(|_| DraftPreviewError::InvalidSource)?
                    .evaluation;
                return Ok(DraftTestResponse::Evaluated {
                    correct: evaluation.correct(),
                    normalized_credit: evaluation.normalized_credit(),
                });
            }
            DraftPreviewBackend::Webwork { pg_path } => {
                let seed = seed.ok_or(DraftPreviewError::InvalidSource)?;
                self.webwork
                    .grade_draft_response(seed, &source.bytes, pg_path, &response)
                    .await
                    .map_err(|_| DraftPreviewError::Unavailable)?
            }
        };
        Ok(match outcome {
            grading::QuestionGradingOutcome::Evaluated(evaluation) => {
                DraftTestResponse::Evaluated {
                    correct: evaluation.correct(),
                    normalized_credit: evaluation.normalized_credit(),
                }
            }
            grading::QuestionGradingOutcome::Ungraded => DraftTestResponse::Ungraded {},
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DraftPreviewBackend<'a> {
    Native,
    Webwork { pg_path: &'a str },
}

fn select_backend(
    backend: QuestionBackend,
    format: QuestionFormat,
    webwork_pg_path: Option<&str>,
) -> Result<DraftPreviewBackend<'_>, DraftPreviewError> {
    match (backend, format, webwork_pg_path) {
        (QuestionBackend::Ple, QuestionFormat::PleQuestionJson, None) => {
            Ok(DraftPreviewBackend::Native)
        }
        (
            QuestionBackend::Webwork,
            QuestionFormat::WebworkPg | QuestionFormat::WebworkPgml,
            Some(pg_path),
        ) if !pg_path.is_empty() => Ok(DraftPreviewBackend::Webwork { pg_path }),
        (QuestionBackend::Imathas, _, _) => Err(DraftPreviewError::UnsupportedBackend),
        _ => Err(DraftPreviewError::Unavailable),
    }
}

fn current_draft_source(
    draft: AuthoringDraft,
    bytes: Vec<u8>,
) -> Result<CurrentDraftSource, DraftPreviewError> {
    select_backend(
        draft.question_backend,
        draft.question_format,
        draft.webwork_pg_path.as_deref(),
    )?;
    Ok(CurrentDraftSource {
        workspace: draft.workspace,
        question_title: draft.metadata.question_title,
        backend: draft.question_backend,
        format: draft.question_format,
        webwork_pg_path: draft.webwork_pg_path,
        bytes,
    })
}

async fn preview_draft(
    State(state): State<RouteState>,
    headers: HeaderMap,
    Path(draft_question_id): Path<String>,
    Query(query): Query<DraftPreviewQuery>,
) -> Response {
    let (session, draft, expected_edit_number, seed) = match request_identity(
        &state,
        &headers,
        &draft_question_id,
        &query.draft_question_edit_number,
        query.seed,
    )
    .await
    {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let webwork_draft = state
        .drafts
        .load_authoring_draft(session, draft)
        .await
        .is_ok_and(|draft| draft.question_backend == QuestionBackend::Webwork);
    let output = match state
        .operations
        .preview(session, draft, expected_edit_number, seed)
        .await
    {
        Ok(output) => output,
        Err(error) => {
            return if webwork_draft {
                preview_document_error(error)
            } else {
                preview_error(error)
            };
        }
    };
    match output {
        DraftPreviewOutput::Native(preview) => crate::auth::no_store(Json(preview).into_response()),
        DraftPreviewOutput::Webwork(document) => match String::from_utf8(document) {
            Ok(document) => {
                crate::webwork_document_route::preview_backend_document_response(document)
            }
            Err(_) => preview_error(DraftPreviewError::Unavailable),
        },
    }
}

async fn test_draft(
    State(state): State<RouteState>,
    headers: HeaderMap,
    Path(draft_question_id): Path<String>,
    body: Bytes,
) -> Response {
    let media_type = headers
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(';').next())
        .map(str::trim);
    if media_type != Some("application/json") {
        return crate::auth::no_store(
            (
                StatusCode::UNSUPPORTED_MEDIA_TYPE,
                Json(serde_json::json!({ "error": "Draft test must use JSON" })),
            )
                .into_response(),
        );
    }
    let expected_edit_number = match crate::authoring::expected_edit_number(&headers) {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let draft = match crate::authoring::parse_draft_question_uuid(&draft_question_id) {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let session = match authoring_session_hash(&state, &headers).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let request = match serde_json::from_slice::<DraftTestRequest>(&body) {
        Ok(request) => request,
        Err(_) => return preview_error(DraftPreviewError::InvalidSource),
    };
    let seed = request.seed.map(QuestionSeed::new);
    match state
        .operations
        .test(session, draft, expected_edit_number, seed, request.response)
        .await
    {
        Ok(result) => crate::auth::no_store(Json(result).into_response()),
        Err(error) => preview_error(error),
    }
}

async fn request_identity(
    state: &RouteState,
    headers: &HeaderMap,
    draft_question_id: &str,
    edit_number_value: &str,
    seed: Option<u64>,
) -> Result<
    (
        SessionTokenHash,
        DraftQuestionUuid,
        DraftQuestionEditNumber,
        Option<QuestionSeed>,
    ),
    Box<Response>,
> {
    let session = authoring_session_hash(state, headers).await?;
    let draft = crate::authoring::parse_draft_question_uuid(draft_question_id)?;
    let expected_edit_number = edit_number_value
        .parse::<u64>()
        .ok()
        .and_then(|number| DraftQuestionEditNumber::new(number).ok())
        .ok_or_else(|| Box::new(preview_error(DraftPreviewError::Conflict)))?;
    Ok((
        session,
        draft,
        expected_edit_number,
        seed.map(QuestionSeed::new),
    ))
}

async fn authoring_session_hash(
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
        Err(AuthError::Unavailable(_) | AuthError::Randomness(_)) => {
            Err(Box::new(preview_error(DraftPreviewError::Unavailable)))
        }
    }
}

fn map_store_error(error: StoreError) -> DraftPreviewError {
    match error {
        StoreError::NotFound | StoreError::Forbidden | StoreError::OwnershipMismatch => {
            DraftPreviewError::NotFound
        }
        StoreError::Conflict | StoreError::RetryableTransaction => DraftPreviewError::Conflict,
        _ => DraftPreviewError::Unavailable,
    }
}

fn preview_error(error: DraftPreviewError) -> Response {
    let (status, message) = preview_error_details(error);
    crate::auth::no_store((status, Json(serde_json::json!({"error": message}))).into_response())
}

fn preview_document_error(error: DraftPreviewError) -> Response {
    let (status, message) = preview_error_details(error);
    let document = format!(
        "<!doctype html><html lang=\"en\"><meta charset=\"utf-8\"><title>Draft preview failed</title><body><main><h1>Draft preview failed</h1><p>{message}</p></main></body></html>"
    );
    let mut response = crate::webwork_document_route::preview_backend_document_response(document);
    *response.status_mut() = status;
    response
}

fn preview_error_details(error: DraftPreviewError) -> (StatusCode, &'static str) {
    match error {
        DraftPreviewError::NotFound => (StatusCode::NOT_FOUND, "Draft Question is unavailable"),
        DraftPreviewError::Conflict => (StatusCode::PRECONDITION_FAILED, "Draft Question changed"),
        DraftPreviewError::UnsupportedBackend => (
            StatusCode::NOT_IMPLEMENTED,
            "This Question Backend does not support Draft preview or testing",
        ),
        DraftPreviewError::InvalidSource => (
            StatusCode::UNPROCESSABLE_ENTITY,
            "Draft source or test response is invalid; saved Draft work was preserved",
        ),
        DraftPreviewError::Unavailable => (
            StatusCode::SERVICE_UNAVAILABLE,
            "Draft preview or testing is unavailable; saved Draft work was preserved",
        ),
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

#[cfg(test)]
#[path = "draft_preview/tests.rs"]
mod tests;
