//! Shared, private route state, browser DTOs, and error projections for runs.

pub(super) use std::sync::Arc;

pub(super) use axum::Json;
pub(super) use axum::body::to_bytes;
pub(super) use axum::extract::{Path, Query, State};
pub(super) use axum::http::{HeaderMap, StatusCode};
pub(super) use axum::response::{IntoResponse, Response};
pub(super) use learning_data_access::{
    CatalogStore, CourseAppearanceStore, Cursor, IssueQuestionAttemptCommand, ManualGradingStore,
    PageRequest, PageSize, PaginationError, SessionStore, Store, StoreError,
    SubmissionIdempotencyKey, SubmissionRecord, SubmitQuestionAttemptCommand, TenantContext,
};
pub(super) use question_model::generation::Seed;
pub(super) use question_model::presentation::{PresentationV1, build_presentation_v1};
pub(super) use question_model::run_policy::FeedbackDisclosure;
pub(super) use question_model::{
    AssignmentEnrollment, AssignmentRun, AttemptResult, CourseAppearance, CourseRole,
    CourseSummary, DisclosedFeedback, FeedbackContent, PresentationBindingV1, ProblemVersionRef,
    QuestionAttempt, QuestionAttemptId, QuestionDefinition, QuestionEnvelope, RunId,
    StudentAssignmentSummary, StudentResponse, UserRole,
};
pub(super) use serde::{Deserialize, Serialize};

pub(super) use crate::auth::{
    AuthenticatedSession, auth_error_response, no_store, resolve_request_session,
};
pub(super) use crate::feedback::{FeedbackDisclosureState, project_feedback};

use super::contracts::RunBackendError;

pub(super) const DEFAULT_PAGE_SIZE: u16 = 50;
pub(super) const INTERNAL_ATTEMPT_PAGE_SIZE: u16 = PageSize::MAX;
pub(super) const MAX_SUBMISSION_BODY_BYTES: usize = 64 * 1024;
const IDEMPOTENCY_HEADER: &str = "idempotency-key";
pub(super) const MAX_JSON_SAFE_INTEGER: u64 = (1_u64 << 53) - 1;

pub(super) struct RunRouteState<S, B> {
    pub(super) store: Arc<S>,
    pub(super) backend: Arc<B>,
}

impl<S, B> Clone for RunRouteState<S, B> {
    fn clone(&self) -> Self {
        Self {
            store: Arc::clone(&self.store),
            backend: Arc::clone(&self.backend),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct RunQuery {
    pub(super) cursor: Option<String>,
    pub(super) page_size: Option<u16>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct StartRunRequest {
    pub(super) assignment_id: question_model::AssignmentId,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct SubmitResponseRequest {
    pub(super) response: StudentResponse,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct SubmissionReceipt {
    pub(super) accepted: bool,
    pub(super) attempt: QuestionAttempt,
    pub(super) feedback: Option<DisclosedFeedback>,
    pub(super) next_issued: Option<NextIssuedAttempt>,
}

/// Browser-safe identity binding for a just-issued next attempt.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct NextIssuedAttempt {
    pub(super) id: QuestionAttemptId,
    pub(super) run: RunId,
    pub(super) question_version: question_model::VersionId,
    pub(super) seed: Seed,
    pub(super) deadline: Option<question_model::ActivityTimestamp>,
    pub(super) assignment_position: u32,
    pub(super) rendered_question_sha256: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct PrefetchedNextQuestion {
    pub(super) predecessor: QuestionAttemptId,
    pub(super) run: RunId,
    pub(super) assignment_position: u32,
    pub(super) question_version: question_model::VersionId,
    pub(super) seed: Seed,
    pub(super) rendered_question_sha256: String,
    pub(super) envelope: QuestionEnvelope,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct RunSummaryOutcome {
    pub(super) attempt: QuestionAttemptId,
    pub(super) assignment_position: u32,
    pub(super) submitted_at: Option<question_model::ActivityTimestamp>,
    pub(super) response: Option<StudentResponse>,
    pub(super) feedback: Option<DisclosedFeedback>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct RunSummaryResponse {
    pub(super) course: CourseRouteData,
    pub(super) run: AssignmentRun,
    pub(super) summary: StudentAssignmentSummary,
    pub(super) practice_allowed: bool,
    pub(super) outcomes: learning_data_access::Page<RunSummaryOutcome>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct CourseRouteData {
    pub(super) summary: CourseSummary,
    pub(super) appearance: CourseAppearance,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct FeedbackReleaseResponse {
    pub(super) released: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct EnrollmentView {
    pub(super) enrollment: AssignmentEnrollment,
    pub(super) summary: StudentAssignmentSummary,
}

pub(super) fn fresh_seed() -> Result<u64, RunBackendError> {
    let mut bytes = [0_u8; 8];
    getrandom::fill(&mut bytes).map_err(|error| RunBackendError::Unavailable(error.to_string()))?;
    Ok(u64::from_le_bytes(bytes) & MAX_JSON_SAFE_INTEGER)
}

pub(super) fn submission_key(
    headers: &HeaderMap,
) -> Result<SubmissionIdempotencyKey, &'static str> {
    let value = headers
        .get(IDEMPOTENCY_HEADER)
        .ok_or("idempotency-key is required")?
        .to_str()
        .map_err(|_| "idempotency-key is invalid")?;
    SubmissionIdempotencyKey::parse(value).map_err(|_| "idempotency-key is invalid")
}

pub(super) fn page_request(query: RunQuery) -> Result<PageRequest, PaginationError> {
    let size = PageSize::new(query.page_size.unwrap_or(DEFAULT_PAGE_SIZE))?;
    match query.cursor {
        Some(cursor) => Ok(PageRequest::after(Cursor::parse(cursor)?, size)),
        None => Ok(PageRequest::first(size)),
    }
}

pub(super) fn backend_error_response(error: RunBackendError) -> Response {
    match error {
        RunBackendError::Unsupported(message) | RunBackendError::Invalid(message) => {
            error_response(StatusCode::UNPROCESSABLE_ENTITY, &message)
        }
        RunBackendError::Unavailable(_) => error_response(
            StatusCode::SERVICE_UNAVAILABLE,
            "question backend unavailable",
        ),
    }
}

pub(super) fn store_error_response(error: StoreError) -> Response {
    match error {
        StoreError::NotFound => error_response(StatusCode::NOT_FOUND, "record not found"),
        StoreError::AlreadyExists | StoreError::Conflict => {
            error_response(StatusCode::CONFLICT, "record changed or already exists")
        }
        StoreError::TenantMismatch | StoreError::Forbidden => {
            error_response(StatusCode::FORBIDDEN, "operation is not authorized")
        }
        StoreError::InvalidRecord(message) => {
            error_response(StatusCode::UNPROCESSABLE_ENTITY, &message)
        }
        StoreError::RunModel(error) => {
            error_response(StatusCode::UNPROCESSABLE_ENTITY, &error.to_string())
        }
        StoreError::TimedOut => error_response(StatusCode::CONFLICT, "question attempt timed out"),
        StoreError::RetryableTransaction | StoreError::Unavailable(_) => {
            error_response(StatusCode::SERVICE_UNAVAILABLE, "run storage unavailable")
        }
    }
}

pub(super) fn error_response(status: StatusCode, message: &str) -> Response {
    no_store((status, Json(serde_json::json!({ "error": message }))).into_response())
}

pub(super) async fn no_store_response(response: Response) -> Response {
    no_store(response)
}
