//! Authenticated run, attempt, submission, and grading-summary routes (MOD-API-RUN).
//!
//! The store owns timestamps, run numbers, one-active-question enforcement,
//! idempotency, and transactional summary changes. A pluggable server-only
//! backend owns rendering provenance and correctness so this route group does
//! not choose the first native family or expose an answer key.

use std::sync::Arc;

use async_trait::async_trait;
use axum::body::to_bytes;
use axum::extract::{DefaultBodyLimit, Path, Query, State};
#[cfg(test)]
use axum::http::HeaderValue;
use axum::http::{HeaderMap, StatusCode};
use axum::middleware;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use grading::GradeOutcome;
use learning_data_access::{
    CatalogStore, Cursor, IssueQuestionAttemptCommand, ManualGradingStore, PageRequest, PageSize,
    PaginationError, SessionStore, Store, StoreError, SubmissionIdempotencyKey, SubmissionRecord,
    SubmitQuestionAttemptCommand, TenantContext,
};
use question_model::generation::Seed;
use question_model::run_policy::FeedbackDisclosure;
use question_model::{
    AssignmentEnrollment, AssignmentId, AssignmentRun, AttemptProvenance, AttemptResult,
    DisclosedFeedback, FeedbackContent, ProblemVersionRef, QuestionAttempt, QuestionAttemptId,
    QuestionDefinition, QuestionEnvelope, RunId, StudentAssignmentSummary, StudentResponse,
    UserRole,
};
use serde::{Deserialize, Serialize};

use crate::auth::{AuthenticatedSession, auth_error_response, no_store, resolve_request_session};
use crate::feedback::{FeedbackDisclosureState, project_feedback};

const DEFAULT_PAGE_SIZE: u16 = 50;
const INTERNAL_ATTEMPT_PAGE_SIZE: u16 = PageSize::MAX;
const MAX_SUBMISSION_BODY_BYTES: usize = 64 * 1_024;
const IDEMPOTENCY_HEADER: &str = "idempotency-key";
const MAX_JSON_SAFE_INTEGER: u64 = (1_u64 << 53) - 1;

/// Key-free metadata produced while a trusted adapter issues one instance.
#[derive(Debug, Clone, PartialEq)]
pub struct IssuedAttemptMetadata {
    /// The key-free rendered envelope prepared by the trusted backend.
    pub envelope: QuestionEnvelope,
    /// SHA-256 of generated parameter values.
    pub parameter_hash: String,
    /// Complete reproducibility record without an answer or key.
    ///
    /// The backend owns the canonical rendered artifact covered by
    /// `rendered_question_sha256`. For example, WeBWorK includes its
    /// sanitized renderer markup in addition to the shared envelope.
    pub provenance: AttemptProvenance,
}

/// The durable disposition of a response chosen by its trusted backend.
///
/// Most backends return [`Self::Grade`] and let the generic run route persist
/// it.  A server-mediated external tool instead owns an all-or-nothing broker
/// transaction and returns [`Self::Committed`].  Keeping that distinction in
/// this server-only seam prevents a provider grade from being observed before
/// its attempt record is durably committed.
#[derive(Clone, PartialEq)]
pub enum SubmissionDisposition {
    /// A normal server-only grade that the generic attempt store must commit.
    Grade(GradeReceipt),
    /// A valid response whose trusted backend requires an instructor's
    /// server-side evaluation before a numeric result exists.
    NeedsManualGrading,
    /// A record already atomically committed by a backend-owned broker.
    Committed(Box<SubmissionRecord>),
}

impl std::fmt::Debug for SubmissionDisposition {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Grade(_) => formatter.debug_tuple("Grade").field(&"[redacted]").finish(),
            Self::NeedsManualGrading => formatter.write_str("NeedsManualGrading"),
            Self::Committed(record) => formatter.debug_tuple("Committed").field(record).finish(),
        }
    }
}

/// One server-only grade and its private, sanitized teaching material.
///
/// This deliberately has no wire traits and no debug implementation: answer
/// keys and feedback live only long enough to enter the trusted store command.
#[derive(Clone, PartialEq)]
pub struct GradeReceipt {
    pub result: AttemptResult,
    pub feedback: FeedbackContent,
}

impl GradeReceipt {
    pub fn empty(result: AttemptResult) -> Self {
        Self {
            result,
            feedback: FeedbackContent::default(),
        }
    }
}

/// Complete trusted input to a backend-owned submission transition.
///
/// This is intentionally constructed only after the route has authenticated
/// the actor, completed replay lookup, loaded the tenant-visible attempt, and
/// validated the browser response shape.
pub struct RunSubmission<'a> {
    pub context: TenantContext,
    pub actor: question_model::UserId,
    pub idempotency_key: SubmissionIdempotencyKey,
    pub reference: ProblemVersionRef,
    pub question: &'a QuestionDefinition,
    pub attempt: &'a QuestionAttempt,
    pub response: &'a StudentResponse,
}

/// Adapter-owned server boundary used by the generic run routes.
#[async_trait]
pub trait RunBackend: Send + Sync {
    /// Generates or renders one fresh instance from the server-owned seed.
    async fn issue(
        &self,
        context: TenantContext,
        reference: ProblemVersionRef,
        question: &QuestionDefinition,
        seed: u64,
    ) -> Result<IssuedAttemptMetadata, RunBackendError>;

    /// Rebuilds the exact key-free envelope that was issued for an attempt.
    ///
    /// The backend verifies the persisted seed, parameter hash, provenance,
    /// and immutable version before this browser-facing representation leaves
    /// the server.
    async fn reproduce(
        &self,
        context: TenantContext,
        reference: ProblemVersionRef,
        question: &QuestionDefinition,
        attempt: &QuestionAttempt,
    ) -> Result<QuestionEnvelope, RunBackendError>;

    /// Confirms that this exact issued external-tool attempt may use the
    /// server-owned launch broker. It deliberately returns no provider data.
    async fn prepare_external_tool_launch(
        &self,
        _context: TenantContext,
        _reference: ProblemVersionRef,
        _question: &QuestionDefinition,
        _attempt: &QuestionAttempt,
    ) -> Result<(), RunBackendError> {
        Err(RunBackendError::Unsupported(
            "this question backend does not provide an external-tool launch".to_string(),
        ))
    }

    /// Grades one response without returning or serializing its key.
    async fn grade(
        &self,
        context: TenantContext,
        reference: ProblemVersionRef,
        question: &QuestionDefinition,
        attempt: &QuestionAttempt,
        response: &StudentResponse,
    ) -> Result<GradeOutcome, RunBackendError>;

    /// Submits one response after route-level replay and format validation.
    ///
    /// The actor and idempotency key deliberately cross this boundary so an
    /// external backend can bind its provider exchange to the exact tenant
    /// record that will be committed.  The default preserves the ordinary
    /// Native and WeBWorK grade-then-store behavior.
    async fn submit(
        &self,
        submission: RunSubmission<'_>,
    ) -> Result<SubmissionDisposition, RunBackendError> {
        let _ = (submission.actor, &submission.idempotency_key);
        self.grade(
            submission.context,
            submission.reference,
            submission.question,
            submission.attempt,
            submission.response,
        )
        .await
        .and_then(|outcome| match outcome {
            GradeOutcome::Graded(result) => {
                Ok(SubmissionDisposition::Grade(GradeReceipt::empty(result)))
            }
            GradeOutcome::NeedsManualGrading => Ok(SubmissionDisposition::NeedsManualGrading),
            GradeOutcome::Ungraded => Err(RunBackendError::Unsupported(
                "this run backend does not produce a server grade".to_string(),
            )),
        })
    }
}

mod external_tool;
mod manual_grading;
pub use external_tool::{
    ExternalToolLaunch, ExternalToolLaunchBackend, router as external_tool_router,
};

/// Failure from the selected trusted adapter or grading implementation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RunBackendError {
    /// The selected backend does not implement the requested behavior.
    Unsupported(String),
    /// The published definition or private backend material is invalid.
    Invalid(String),
    /// A renderer or backend dependency is temporarily unavailable.
    Unavailable(String),
}

impl std::fmt::Display for RunBackendError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unsupported(message) => write!(formatter, "unsupported run behavior: {message}"),
            Self::Invalid(message) => write!(formatter, "invalid run backend data: {message}"),
            Self::Unavailable(message) => write!(formatter, "run backend unavailable: {message}"),
        }
    }
}

impl std::error::Error for RunBackendError {}

/// Builds the authenticated run route group around a shared store and backend registry.
pub fn router<S, B>(store: Arc<S>, backend: Arc<B>) -> Router
where
    S: Store + CatalogStore + ManualGradingStore + SessionStore + 'static,
    B: RunBackend + 'static,
{
    Router::new()
        .route("/api/runs", post(start_run::<S, B>))
        .route("/api/runs/{run}", get(get_run::<S, B>))
        .route("/api/runs/{run}/summary", get(get_run_summary::<S, B>))
        .route("/api/runs/{run}/attempts", get(list_attempts::<S, B>))
        .route("/api/attempts/{attempt}", get(get_attempt::<S, B>))
        .route(
            "/api/attempts/{attempt}/question",
            get(get_attempt_question::<S, B>),
        )
        .route(
            "/api/attempts/{attempt}/prefetch-next",
            post(prefetch_next_question::<S, B>),
        )
        .route(
            "/api/attempts/{attempt}/external-tool-launch",
            get(get_external_tool_launch::<S, B>),
        )
        .route("/api/submissions/{attempt}", post(submit_response::<S, B>))
        .route(
            "/api/attempts/{attempt}/manual-grade",
            get(manual_grading::get_manual_grade::<S, B>)
                .put(manual_grading::put_manual_grade::<S, B>),
        )
        .route(
            "/api/attempts/{attempt}/feedback-release",
            post(release_attempt_feedback::<S, B>),
        )
        .route(
            "/api/grading/summaries/{enrollment}",
            get(get_summary::<S, B>),
        )
        .route("/api/enrollments/{enrollment}", get(get_enrollment::<S, B>))
        .route("/api/enrollments/{enrollment}/runs", get(list_runs::<S, B>))
        .layer(DefaultBodyLimit::max(MAX_SUBMISSION_BODY_BYTES))
        // Also covers extractor rejections such as malformed JSON, oversized
        // bodies, and malformed typed path values before a handler runs.
        .layer(middleware::map_response(no_store_response))
        .with_state(RunRouteState { store, backend })
}

async fn no_store_response(response: Response) -> Response {
    no_store(response)
}

struct RunRouteState<S, B> {
    store: Arc<S>,
    backend: Arc<B>,
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
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
struct RunQuery {
    cursor: Option<String>,
    page_size: Option<u16>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct StartRunRequest {
    assignment_id: AssignmentId,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SubmitResponseRequest {
    response: StudentResponse,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SubmissionReceipt {
    accepted: bool,
    attempt: QuestionAttempt,
    feedback: Option<DisclosedFeedback>,
    next_issued: Option<NextIssuedAttempt>,
}

/// Browser-safe identity binding for a just-issued next attempt. It excludes
/// provenance and every source/key/provider field.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct NextIssuedAttempt {
    id: QuestionAttemptId,
    run: RunId,
    question_version: question_model::VersionId,
    seed: Seed,
    deadline: Option<question_model::ActivityTimestamp>,
    assignment_position: u32,
    rendered_question_sha256: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PrefetchedNextQuestion {
    predecessor: QuestionAttemptId,
    run: RunId,
    assignment_position: u32,
    question_version: question_model::VersionId,
    seed: Seed,
    rendered_question_sha256: String,
    envelope: QuestionEnvelope,
}

/// Compact, policy-redacted outcome shown only through a tenant-authorized run summary.
///
/// This intentionally excludes the authored question, provenance, provider state, and every
/// private feedback record. The response is the learner's own submitted response; teaching
/// material enters only through the server-redacted `feedback` projection.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct RunSummaryOutcome {
    attempt: QuestionAttemptId,
    assignment_position: u32,
    submitted_at: Option<question_model::ActivityTimestamp>,
    response: Option<StudentResponse>,
    feedback: Option<DisclosedFeedback>,
}

/// Browser-safe run completion projection. The store returns private inputs; this route owns the
/// one-way conversion into this DTO.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct RunSummaryResponse {
    run: AssignmentRun,
    summary: StudentAssignmentSummary,
    practice_allowed: bool,
    outcomes: learning_data_access::Page<RunSummaryOutcome>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct FeedbackReleaseResponse {
    released: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct EnrollmentView {
    enrollment: AssignmentEnrollment,
    summary: StudentAssignmentSummary,
}

async fn start_run<S, B>(
    State(state): State<RunRouteState<S, B>>,
    headers: HeaderMap,
    Json(request): Json<StartRunRequest>,
) -> Response
where
    S: Store + CatalogStore + ManualGradingStore + SessionStore + 'static,
    B: RunBackend + 'static,
{
    let authenticated = match resolve_request_session(state.store.as_ref(), &headers).await {
        Ok(authenticated) => authenticated,
        Err(error) => return auth_error_response(error),
    };
    let actor = authenticated.record.subject.user();
    let run = match state
        .store
        .start_or_resume_run(
            authenticated.tenant_context,
            actor,
            request.assignment_id,
            RunId::generate(),
        )
        .await
    {
        Ok(run) => run,
        Err(error) => return store_error_response(error),
    };
    let attempts =
        match all_attempts(state.store.as_ref(), authenticated.tenant_context, run.id).await {
            Ok(value) => value,
            Err(response) => return response,
        };
    // A resumed run can represent a process that committed submission N but
    // crashed before its receipt successor was linked. Only submission replay
    // may heal that state, because it has the durable predecessor identity.
    let predecessor = if attempts.is_empty() {
        None
    } else {
        match state
            .store
            .pending_submission_for_run(authenticated.tenant_context, actor, run.id)
            .await
        {
            Ok(value) => value,
            Err(error) => return store_error_response(error),
        }
    };
    if (attempts.is_empty() || predecessor.is_some())
        && let Err(response) = ensure_active_questions(
            state.store.as_ref(),
            state.backend.as_ref(),
            &authenticated,
            &run,
            predecessor,
        )
        .await
    {
        return response;
    }
    no_store((StatusCode::CREATED, Json(run)).into_response())
}

async fn get_run<S, B>(
    State(state): State<RunRouteState<S, B>>,
    headers: HeaderMap,
    Path(run_id): Path<RunId>,
) -> Response
where
    S: Store + CatalogStore + SessionStore + 'static,
    B: RunBackend + 'static,
{
    let authenticated = match resolve_request_session(state.store.as_ref(), &headers).await {
        Ok(authenticated) => authenticated,
        Err(error) => return auth_error_response(error),
    };
    match authorized_run(state.store.as_ref(), &authenticated, run_id).await {
        Ok(run) => no_store(Json(run).into_response()),
        Err(response) => response,
    }
}

/// Returns the current, bounded learner-facing completion view for one run.
///
/// The store supplies private feedback and release facts in a single authorized page read. This
/// route performs the only public projection, so a release changes this GET view without rewriting
/// the immutable submission receipt that was returned at grade time.
async fn get_run_summary<S, B>(
    State(state): State<RunRouteState<S, B>>,
    headers: HeaderMap,
    Path(run_id): Path<RunId>,
    Query(query): Query<RunQuery>,
) -> Response
where
    S: Store + CatalogStore + SessionStore + 'static,
    B: RunBackend + 'static,
{
    let authenticated = match resolve_request_session(state.store.as_ref(), &headers).await {
        Ok(authenticated) => authenticated,
        Err(error) => return auth_error_response(error),
    };
    let page = match page_request(query) {
        Ok(page) => page,
        Err(error) => return error_response(StatusCode::BAD_REQUEST, &error.to_string()),
    };
    let page = match state
        .store
        .get_run_summary_page(
            authenticated.tenant_context,
            authenticated.record.subject.user(),
            run_id,
            page,
        )
        .await
    {
        Ok(page) => page,
        Err(error) => return store_error_response(error),
    };
    let outcomes = page
        .outcomes
        .items
        .into_iter()
        .map(|outcome| {
            let empty_feedback = FeedbackContent::default();
            let content = outcome.feedback.as_ref().map_or(
                &empty_feedback,
                learning_data_access::AttemptFeedbackRecord::content,
            );
            let feedback = project_feedback(
                outcome.feedback_policy,
                FeedbackDisclosureState {
                    run_completed: page.run.completed_at.is_some(),
                    released: outcome.release.is_some(),
                },
                outcome.result,
                content,
            );
            RunSummaryOutcome {
                attempt: outcome.attempt,
                assignment_position: outcome.assignment_position,
                submitted_at: outcome.submitted_at,
                response: outcome.response,
                feedback,
            }
        })
        .collect();
    no_store(
        Json(RunSummaryResponse {
            run: page.run,
            summary: page.summary,
            practice_allowed: page.practice_allowed,
            outcomes: learning_data_access::Page {
                items: outcomes,
                next_cursor: page.outcomes.next_cursor,
            },
        })
        .into_response(),
    )
}

/// Releases one completed on-release attempt after the store derives direct instructor authority.
///
/// The response intentionally confirms only the state transition. Private feedback remains in the
/// store and is revealed, if permitted, only by a later run-summary GET projection.
async fn release_attempt_feedback<S, B>(
    State(state): State<RunRouteState<S, B>>,
    headers: HeaderMap,
    Path(attempt): Path<QuestionAttemptId>,
) -> Response
where
    S: Store + CatalogStore + SessionStore + 'static,
    B: RunBackend + 'static,
{
    let authenticated = match resolve_request_session(state.store.as_ref(), &headers).await {
        Ok(authenticated) => authenticated,
        Err(error) => return auth_error_response(error),
    };
    match state
        .store
        .release_attempt_feedback(
            authenticated.tenant_context,
            learning_data_access::ReleaseAttemptFeedbackCommand {
                actor: authenticated.record.subject.user(),
                attempt,
            },
        )
        .await
    {
        Ok(_) => no_store(Json(FeedbackReleaseResponse { released: true }).into_response()),
        Err(error) => store_error_response(error),
    }
}

async fn list_attempts<S, B>(
    State(state): State<RunRouteState<S, B>>,
    headers: HeaderMap,
    Path(run_id): Path<RunId>,
    Query(query): Query<RunQuery>,
) -> Response
where
    S: Store + CatalogStore + SessionStore + 'static,
    B: RunBackend + 'static,
{
    let authenticated = match resolve_request_session(state.store.as_ref(), &headers).await {
        Ok(authenticated) => authenticated,
        Err(error) => return auth_error_response(error),
    };
    let run = match authorized_run(state.store.as_ref(), &authenticated, run_id).await {
        Ok(run) => run,
        Err(response) => return response,
    };
    let page = match page_request(query) {
        Ok(page) => page,
        Err(error) => return error_response(StatusCode::BAD_REQUEST, &error.to_string()),
    };
    let mut page = match state
        .store
        .list_question_attempts(authenticated.tenant_context, run.id, page)
        .await
    {
        Ok(page) => page,
        Err(error) => return store_error_response(error),
    };
    for attempt in &mut page.items {
        if let Err(response) = apply_feedback_disclosure(
            state.store.as_ref(),
            authenticated.tenant_context,
            &run,
            attempt,
        )
        .await
        {
            return response;
        }
    }
    no_store(Json(page).into_response())
}

async fn get_attempt<S, B>(
    State(state): State<RunRouteState<S, B>>,
    headers: HeaderMap,
    Path(attempt_id): Path<QuestionAttemptId>,
) -> Response
where
    S: Store + CatalogStore + SessionStore + 'static,
    B: RunBackend + 'static,
{
    let authenticated = match resolve_request_session(state.store.as_ref(), &headers).await {
        Ok(authenticated) => authenticated,
        Err(error) => return auth_error_response(error),
    };
    let mut attempt = match state
        .store
        .get_question_attempt(authenticated.tenant_context, attempt_id)
        .await
    {
        Ok(Some(attempt)) => attempt,
        Ok(None) => return error_response(StatusCode::NOT_FOUND, "attempt not found"),
        Err(error) => return store_error_response(error),
    };
    let run = match authorized_run(state.store.as_ref(), &authenticated, attempt.run).await {
        Ok(run) => run,
        Err(response) => return response,
    };
    if let Err(response) = apply_feedback_disclosure(
        state.store.as_ref(),
        authenticated.tenant_context,
        &run,
        &mut attempt,
    )
    .await
    {
        return response;
    }
    no_store(Json(attempt).into_response())
}

/// Returns the exact, key-free envelope for an already issued attempt.
///
/// An attempt record is not enough to reconstruct student-facing content: its
/// seed and provenance must be checked by the selected trusted backend. This
/// route deliberately has no authored-content fallback, so an unavailable or
/// inconsistent backend cannot accidentally serve a different variant.
async fn get_attempt_question<S, B>(
    State(state): State<RunRouteState<S, B>>,
    headers: HeaderMap,
    Path(attempt_id): Path<QuestionAttemptId>,
) -> Response
where
    S: Store + CatalogStore + SessionStore + 'static,
    B: RunBackend + 'static,
{
    let authenticated = match resolve_request_session(state.store.as_ref(), &headers).await {
        Ok(authenticated) => authenticated,
        Err(error) => return auth_error_response(error),
    };
    let attempt = match state
        .store
        .get_question_attempt(authenticated.tenant_context, attempt_id)
        .await
    {
        Ok(Some(attempt)) => attempt,
        Ok(None) => return error_response(StatusCode::NOT_FOUND, "attempt not found"),
        Err(error) => return store_error_response(error),
    };
    if let Err(response) = authorized_run(state.store.as_ref(), &authenticated, attempt.run).await {
        return response;
    }
    let reference = ProblemVersionRef {
        problem: attempt.problem,
        version: attempt.question_version,
    };
    let question = match load_run_question(state.store.as_ref(), &authenticated, reference).await {
        Ok(question) => question,
        Err(response) => return response,
    };
    match state
        .backend
        .reproduce(authenticated.tenant_context, reference, &question, &attempt)
        .await
    {
        Ok(envelope) => no_store(Json(envelope).into_response()),
        Err(error) => backend_error_response(error),
    }
}

/// Prepares the next still-unattempted assignment position while the current
/// question remains the sole active attempt. This is intentionally POST: a
/// successful request creates a durable server reservation, but no timer or
/// activity transition.
async fn prefetch_next_question<S, B>(
    State(state): State<RunRouteState<S, B>>,
    headers: HeaderMap,
    Path(predecessor): Path<QuestionAttemptId>,
    body: axum::body::Body,
) -> Response
where
    S: Store + CatalogStore + SessionStore + 'static,
    B: RunBackend + 'static,
{
    let authenticated = match resolve_request_session(state.store.as_ref(), &headers).await {
        Ok(value) => value,
        Err(error) => return auth_error_response(error),
    };
    // This mutation has no browser-controlled parameters. Consume the body so
    // chunked requests cannot smuggle a seed, position, or provenance past a
    // mere Content-Length check.
    let bytes = match to_bytes(body, 1).await {
        Ok(value) => value,
        Err(_) => {
            return error_response(StatusCode::BAD_REQUEST, "prefetch request body is invalid");
        }
    };
    if !bytes.is_empty() {
        return error_response(
            StatusCode::BAD_REQUEST,
            "prefetch request must not contain a body",
        );
    }
    let active = match state
        .store
        .get_question_attempt(authenticated.tenant_context, predecessor)
        .await
    {
        Ok(Some(value)) => value,
        Ok(None) => return error_response(StatusCode::NOT_FOUND, "attempt not found"),
        Err(error) => return store_error_response(error),
    };
    let run = match owned_run(state.store.as_ref(), &authenticated, active.run).await {
        Ok(value) => value,
        Err(response) => return response,
    };
    if active.response.is_some() || run.completed_at.is_some() {
        return error_response(StatusCode::CONFLICT, "attempt is no longer active");
    }
    if let Err(response) =
        owned_assignment_for_run(state.store.as_ref(), &authenticated, &run).await
    {
        return response;
    }
    let run_items = match state
        .store
        .assignment_run_items(authenticated.tenant_context, run.id)
        .await
    {
        Ok(items) => items,
        Err(error) => return store_error_response(error),
    };
    let attempts =
        match all_attempts(state.store.as_ref(), authenticated.tenant_context, run.id).await {
            Ok(value) => value,
            Err(response) => return response,
        };
    if attempts
        .iter()
        .any(|attempt| attempt.response.is_none() && attempt.id != predecessor)
    {
        return error_response(StatusCode::CONFLICT, "another question attempt is active");
    }
    let Some((assignment_position, reference)) = run_items.iter().find_map(|item| {
        let position = item.issued_position;
        attempts
            .iter()
            .all(|attempt| attempt.assignment_position != position)
            .then_some((position, item.reference))
    }) else {
        return no_store(StatusCode::NO_CONTENT.into_response());
    };
    let question = match load_run_question(state.store.as_ref(), &authenticated, reference).await {
        Ok(value) => value,
        Err(response) => return response,
    };
    let actor = authenticated.record.subject.user();
    let existing = match state
        .store
        .get_prefetched_question(
            authenticated.tenant_context,
            actor,
            run.id,
            predecessor,
            assignment_position,
        )
        .await
    {
        Ok(value) => value,
        Err(error) => return store_error_response(error),
    };
    let (reservation, issued) = match existing {
        Some(value) => (value, None),
        None => {
            let seed = match fresh_seed() {
                Ok(value) => value,
                Err(error) => return backend_error_response(error),
            };
            let issued = match state
                .backend
                .issue(authenticated.tenant_context, reference, &question, seed)
                .await
            {
                Ok(value) => value,
                Err(error) => return backend_error_response(error),
            };
            let value = learning_data_access::PrefetchedQuestion {
                tenant: authenticated.tenant_context.tenant_id(),
                run: run.id,
                predecessor,
                assignment_position,
                problem: reference.problem,
                question_version: reference.version,
                seed,
                parameter_hash: issued.parameter_hash.clone(),
                provenance: issued.provenance.clone(),
            };
            let reservation = match state
                .store
                .reserve_or_resume_prefetched_question(
                    authenticated.tenant_context,
                    learning_data_access::ReservePrefetchedQuestionCommand {
                        actor,
                        reservation: value.clone(),
                    },
                )
                .await
            {
                Ok(value) => value,
                Err(StoreError::Conflict) => match state
                    .store
                    .get_prefetched_question(
                        authenticated.tenant_context,
                        actor,
                        run.id,
                        predecessor,
                        assignment_position,
                    )
                    .await
                {
                    Ok(Some(value)) => value,
                    Ok(None) => {
                        return error_response(StatusCode::CONFLICT, "attempt is no longer active");
                    }
                    Err(error) => return store_error_response(error),
                },
                Err(error) => return store_error_response(error),
            };
            let issued = (reservation == value).then_some(issued);
            (reservation, issued)
        }
    };
    let issued = match issued {
        Some(value) => value,
        None => match state
            .backend
            .issue(
                authenticated.tenant_context,
                reference,
                &question,
                reservation.seed,
            )
            .await
        {
            Ok(value) => value,
            Err(error) => return backend_error_response(error),
        },
    };
    if issued.parameter_hash != reservation.parameter_hash
        || issued.provenance != reservation.provenance
        || issued.envelope.version != reservation.question_version
        || issued.envelope.seed != Seed::new(reservation.seed)
    {
        return error_response(
            StatusCode::UNPROCESSABLE_ENTITY,
            "prefetched question did not reproduce exactly",
        );
    }
    no_store(
        Json(PrefetchedNextQuestion {
            predecessor,
            run: run.id,
            assignment_position,
            question_version: reference.version,
            seed: Seed::new(reservation.seed),
            rendered_question_sha256: reservation.provenance.rendered_question_sha256,
            envelope: issued.envelope,
        })
        .into_response(),
    )
}

/// Returns only the same-origin broker path for an eligible external-tool
/// attempt. Provider URLs, credentials, correlation IDs, and grades never
/// cross this projection boundary.
async fn get_external_tool_launch<S, B>(
    State(state): State<RunRouteState<S, B>>,
    headers: HeaderMap,
    Path(attempt_id): Path<QuestionAttemptId>,
) -> Response
where
    S: Store + CatalogStore + SessionStore + 'static,
    B: RunBackend + 'static,
{
    let authenticated = match resolve_request_session(state.store.as_ref(), &headers).await {
        Ok(authenticated) => authenticated,
        Err(error) => return auth_error_response(error),
    };
    let attempt = match state
        .store
        .get_question_attempt(authenticated.tenant_context, attempt_id)
        .await
    {
        Ok(Some(attempt)) => attempt,
        Ok(None) => return error_response(StatusCode::NOT_FOUND, "attempt not found"),
        Err(error) => return store_error_response(error),
    };
    if let Err(response) = owned_run(state.store.as_ref(), &authenticated, attempt.run).await {
        return response;
    }
    let reference = ProblemVersionRef {
        problem: attempt.problem,
        version: attempt.question_version,
    };
    let question = match load_run_question(state.store.as_ref(), &authenticated, reference).await {
        Ok(question) => question,
        Err(response) => return response,
    };
    if !matches!(
        question.response,
        question_model::ResponseDefinition::ExternalTool {}
    ) {
        return error_response(
            StatusCode::NOT_FOUND,
            "external-tool launch is not available",
        );
    }
    if let Err(error) = state
        .backend
        .prepare_external_tool_launch(authenticated.tenant_context, reference, &question, &attempt)
        .await
    {
        return backend_error_response(error);
    }
    no_store(
        Json(ExternalToolLaunch {
            launch_url: format!("/api/attempts/{attempt_id}/external-tool/launch"),
        })
        .into_response(),
    )
}

async fn submit_response<S, B>(
    State(state): State<RunRouteState<S, B>>,
    headers: HeaderMap,
    Path(attempt_id): Path<QuestionAttemptId>,
    Json(request): Json<SubmitResponseRequest>,
) -> Response
where
    S: Store + CatalogStore + ManualGradingStore + SessionStore + 'static,
    B: RunBackend + 'static,
{
    let authenticated = match resolve_request_session(state.store.as_ref(), &headers).await {
        Ok(authenticated) => authenticated,
        Err(error) => return auth_error_response(error),
    };
    let idempotency_key = match submission_key(&headers) {
        Ok(key) => key,
        Err(message) => return error_response(StatusCode::BAD_REQUEST, message),
    };
    let actor = authenticated.record.subject.user();
    match state
        .store
        .replay_submission(
            authenticated.tenant_context,
            actor,
            attempt_id,
            &request.response,
            &idempotency_key,
        )
        .await
    {
        Ok(Some(record)) => {
            return finish_submission(
                state.store.as_ref(),
                state.backend.as_ref(),
                &authenticated,
                record,
            )
            .await;
        }
        Ok(None) => {}
        Err(error) => return store_error_response(error),
    }
    let attempt = match state
        .store
        .get_question_attempt(authenticated.tenant_context, attempt_id)
        .await
    {
        Ok(Some(attempt)) => attempt,
        Ok(None) => return error_response(StatusCode::NOT_FOUND, "attempt not found"),
        Err(error) => return store_error_response(error),
    };
    let run = match owned_run(state.store.as_ref(), &authenticated, attempt.run).await {
        Ok(run) => run,
        Err(response) => return response,
    };
    if run.completed_at.is_some() {
        return error_response(StatusCode::CONFLICT, "run is already complete");
    }
    let reference = ProblemVersionRef {
        problem: attempt.problem,
        version: attempt.question_version,
    };
    let question = match load_run_question(state.store.as_ref(), &authenticated, reference).await {
        Ok(question) => question,
        Err(response) => return response,
    };
    let format_report =
        domain::validation::validate_response_format(&question.response, &request.response);
    if !format_report.is_valid() {
        return no_store((StatusCode::UNPROCESSABLE_ENTITY, Json(format_report)).into_response());
    }
    if matches!(
        &question.response,
        question_model::ResponseDefinition::FileUpload { .. }
    ) {
        return error_response(
            StatusCode::UNPROCESSABLE_ENTITY,
            "file upload submissions are unavailable",
        );
    }
    let disposition = match state
        .backend
        .submit(RunSubmission {
            context: authenticated.tenant_context,
            actor,
            idempotency_key: idempotency_key.clone(),
            reference,
            question: &question,
            attempt: &attempt,
            response: &request.response,
        })
        .await
    {
        Ok(disposition) => disposition,
        Err(error) => return backend_error_response(error),
    };
    let record = match disposition {
        SubmissionDisposition::Committed(record) => *record,
        SubmissionDisposition::Grade(receipt) => match state
            .store
            .submit_question_attempt(
                authenticated.tenant_context,
                SubmitQuestionAttemptCommand {
                    actor,
                    attempt: attempt.id,
                    response: request.response,
                    result: receipt.result,
                    feedback: receipt.feedback,
                    idempotency_key,
                },
            )
            .await
        {
            Ok(record) => record,
            Err(error) => return store_error_response(error),
        },
        SubmissionDisposition::NeedsManualGrading => match state
            .store
            .submit_pending_manual_question_attempt(
                authenticated.tenant_context,
                learning_data_access::SubmitPendingManualQuestionAttemptCommand {
                    actor,
                    attempt: attempt.id,
                    response: request.response,
                    idempotency_key,
                },
            )
            .await
        {
            Ok(record) => record,
            Err(error) => return store_error_response(error),
        },
    };
    finish_submission(
        state.store.as_ref(),
        state.backend.as_ref(),
        &authenticated,
        record,
    )
    .await
}

async fn get_enrollment<S, B>(
    State(state): State<RunRouteState<S, B>>,
    headers: HeaderMap,
    Path(enrollment_id): Path<question_model::EnrollmentId>,
) -> Response
where
    S: Store + CatalogStore + SessionStore + 'static,
    B: RunBackend + 'static,
{
    let authenticated = match resolve_request_session(state.store.as_ref(), &headers).await {
        Ok(authenticated) => authenticated,
        Err(error) => return auth_error_response(error),
    };
    let enrollment =
        match authorized_enrollment(state.store.as_ref(), &authenticated, enrollment_id, false)
            .await
        {
            Ok(enrollment) => enrollment,
            Err(response) => return response,
        };
    let summary = match state
        .store
        .get_summary(authenticated.tenant_context, enrollment.id)
        .await
    {
        Ok(Some(summary)) => summary,
        Ok(None) => return error_response(StatusCode::NOT_FOUND, "summary not found"),
        Err(error) => return store_error_response(error),
    };
    no_store(
        Json(EnrollmentView {
            enrollment,
            summary,
        })
        .into_response(),
    )
}

async fn get_summary<S, B>(
    State(state): State<RunRouteState<S, B>>,
    headers: HeaderMap,
    Path(enrollment_id): Path<question_model::EnrollmentId>,
) -> Response
where
    S: Store + CatalogStore + SessionStore + 'static,
    B: RunBackend + 'static,
{
    let authenticated = match resolve_request_session(state.store.as_ref(), &headers).await {
        Ok(authenticated) => authenticated,
        Err(error) => return auth_error_response(error),
    };
    if let Err(response) =
        authorized_enrollment(state.store.as_ref(), &authenticated, enrollment_id, false).await
    {
        return response;
    }
    match state
        .store
        .get_summary(authenticated.tenant_context, enrollment_id)
        .await
    {
        Ok(Some(summary)) => no_store(Json(summary).into_response()),
        Ok(None) => error_response(StatusCode::NOT_FOUND, "summary not found"),
        Err(error) => store_error_response(error),
    }
}

async fn list_runs<S, B>(
    State(state): State<RunRouteState<S, B>>,
    headers: HeaderMap,
    Path(enrollment_id): Path<question_model::EnrollmentId>,
    Query(query): Query<RunQuery>,
) -> Response
where
    S: Store + CatalogStore + SessionStore + 'static,
    B: RunBackend + 'static,
{
    let authenticated = match resolve_request_session(state.store.as_ref(), &headers).await {
        Ok(authenticated) => authenticated,
        Err(error) => return auth_error_response(error),
    };
    if let Err(response) =
        authorized_enrollment(state.store.as_ref(), &authenticated, enrollment_id, false).await
    {
        return response;
    }
    let page = match page_request(query) {
        Ok(page) => page,
        Err(error) => return error_response(StatusCode::BAD_REQUEST, &error.to_string()),
    };
    match state
        .store
        .list_runs(authenticated.tenant_context, enrollment_id, page)
        .await
    {
        Ok(page) => no_store(Json(page).into_response()),
        Err(error) => store_error_response(error),
    }
}

async fn ensure_active_questions<S, B>(
    store: &S,
    backend: &B,
    authenticated: &AuthenticatedSession,
    run: &AssignmentRun,
    predecessor: Option<QuestionAttemptId>,
) -> Result<(), Response>
where
    S: Store + CatalogStore,
    B: RunBackend,
{
    if run.completed_at.is_some() {
        return Ok(());
    }
    let enrollment = owned_enrollment(store, authenticated, run.enrollment).await?;
    let _assignment = store
        .get_assignment(authenticated.tenant_context, enrollment.assignment)
        .await
        .map_err(store_error_response)?
        .ok_or_else(|| error_response(StatusCode::NOT_FOUND, "assignment not found"))?;
    let run_items = store
        .assignment_run_items(authenticated.tenant_context, run.id)
        .await
        .map_err(store_error_response)?;
    let attempts = all_attempts(store, authenticated.tenant_context, run.id).await?;

    if attempts.iter().any(|attempt| attempt.response.is_none()) {
        return Ok(());
    }

    for item in &run_items {
        let position = item.issued_position;
        let reference = item.reference;
        if attempts
            .iter()
            .all(|attempt| attempt.assignment_position != position)
        {
            let question = load_run_question(store, authenticated, reference).await?;
            let prefetched = match predecessor {
                Some(predecessor) => store
                    .get_prefetched_question(
                        authenticated.tenant_context,
                        authenticated.record.subject.user(),
                        run.id,
                        predecessor,
                        position,
                    )
                    .await
                    .map_err(store_error_response)?,
                None => None,
            }
            .filter(|value| {
                value.tenant == authenticated.tenant_context.tenant_id()
                    && value.run == run.id
                    && value.assignment_position == position
                    && value.problem == reference.problem
                    && value.question_version == reference.version
            });
            issue_question(
                store,
                backend,
                authenticated,
                run,
                IssueQuestionRequest {
                    assignment_position: position,
                    reference,
                    question: &question,
                    prefetched,
                    predecessor_submission: predecessor,
                },
            )
            .await?;
            return Ok(());
        }
    }

    for item in &run_items {
        let position = item.issued_position;
        let reference = item.reference;
        let position_attempts: Vec<_> = attempts
            .iter()
            .filter(|attempt| attempt.assignment_position == position)
            .collect();
        let question = load_run_question(store, authenticated, reference).await?;
        if position_attempts
            .iter()
            .filter_map(|attempt| attempt.result)
            .any(|result| result.correct)
        {
            continue;
        }
        let attempt_count = u32::try_from(position_attempts.len()).map_err(|_| {
            error_response(
                StatusCode::UNPROCESSABLE_ENTITY,
                "question attempt count overflow",
            )
        })?;
        if question
            .attempt_policy
            .max_attempts
            .is_some_and(|maximum| attempt_count >= maximum)
        {
            continue;
        }
        issue_question(
            store,
            backend,
            authenticated,
            run,
            IssueQuestionRequest {
                assignment_position: position,
                reference,
                question: &question,
                prefetched: None,
                predecessor_submission: None,
            },
        )
        .await?;
        return Ok(());
    }
    Ok(())
}

async fn load_run_question<S: CatalogStore>(
    store: &S,
    authenticated: &AuthenticatedSession,
    reference: ProblemVersionRef,
) -> Result<QuestionDefinition, Response> {
    let record = store
        .get_catalog_problem(authenticated.tenant_context, reference)
        .await
        .map_err(store_error_response)?
        .ok_or_else(|| error_response(StatusCode::NOT_FOUND, "question version not found"))?;
    let question = record.question;
    if record.problem != reference.problem
        || record.version != reference.version
        || question.problem != reference.problem
        || question.version != reference.version
    {
        return Err(error_response(
            StatusCode::UNPROCESSABLE_ENTITY,
            "published question identity does not match the requested version",
        ));
    }
    if question.attempt_policy.max_attempts == Some(0) {
        return Err(error_response(
            StatusCode::UNPROCESSABLE_ENTITY,
            "question max attempts must be greater than zero",
        ));
    }
    Ok(question)
}

struct IssueQuestionRequest<'a> {
    assignment_position: u32,
    reference: ProblemVersionRef,
    question: &'a QuestionDefinition,
    prefetched: Option<learning_data_access::PrefetchedQuestion>,
    predecessor_submission: Option<QuestionAttemptId>,
}

async fn issue_question<S, B>(
    store: &S,
    backend: &B,
    authenticated: &AuthenticatedSession,
    run: &AssignmentRun,
    request: IssueQuestionRequest<'_>,
) -> Result<QuestionAttempt, Response>
where
    S: Store,
    B: RunBackend,
{
    let (seed, parameter_hash, provenance) = match request.prefetched.as_ref() {
        Some(value) => (
            value.seed,
            value.parameter_hash.clone(),
            value.provenance.clone(),
        ),
        None => {
            let seed = fresh_seed().map_err(backend_error_response)?;
            let issued = backend
                .issue(
                    authenticated.tenant_context,
                    request.reference,
                    request.question,
                    seed,
                )
                .await
                .map_err(backend_error_response)?;
            (seed, issued.parameter_hash, issued.provenance)
        }
    };
    store
        .issue_or_resume_question_attempt(
            authenticated.tenant_context,
            IssueQuestionAttemptCommand {
                actor: authenticated.record.subject.user(),
                attempt: QuestionAttemptId::generate(),
                run: run.id,
                assignment_position: request.assignment_position,
                problem: request.reference.problem,
                question_version: request.reference.version,
                seed,
                parameter_hash,
                provenance,
                prefetched: request.prefetched,
                predecessor_submission: request.predecessor_submission,
            },
        )
        .await
        .map_err(store_error_response)
}

async fn all_attempts<S: Store>(
    store: &S,
    context: learning_data_access::TenantContext,
    run: RunId,
) -> Result<Vec<QuestionAttempt>, Response> {
    let size = PageSize::new(INTERNAL_ATTEMPT_PAGE_SIZE)
        .map_err(|error| error_response(StatusCode::INTERNAL_SERVER_ERROR, &error.to_string()))?;
    let mut page_request = PageRequest::first(size);
    let mut attempts = Vec::new();
    loop {
        let page = store
            .list_question_attempts(context, run, page_request)
            .await
            .map_err(store_error_response)?;
        attempts.extend(page.items);
        let Some(cursor) = page.next_cursor else {
            return Ok(attempts);
        };
        page_request = PageRequest::after(cursor, size);
    }
}

async fn authorized_run<S: Store>(
    store: &S,
    authenticated: &AuthenticatedSession,
    run_id: RunId,
) -> Result<AssignmentRun, Response> {
    let run = store
        .get_run(authenticated.tenant_context, run_id)
        .await
        .map_err(store_error_response)?
        .ok_or_else(|| error_response(StatusCode::NOT_FOUND, "run not found"))?;
    authorized_enrollment(store, authenticated, run.enrollment, false).await?;
    Ok(run)
}

async fn owned_run<S: Store>(
    store: &S,
    authenticated: &AuthenticatedSession,
    run_id: RunId,
) -> Result<AssignmentRun, Response> {
    let run = store
        .get_run(authenticated.tenant_context, run_id)
        .await
        .map_err(store_error_response)?
        .ok_or_else(|| error_response(StatusCode::NOT_FOUND, "run not found"))?;
    owned_enrollment(store, authenticated, run.enrollment).await?;
    Ok(run)
}

async fn owned_enrollment<S: Store>(
    store: &S,
    authenticated: &AuthenticatedSession,
    enrollment_id: question_model::EnrollmentId,
) -> Result<AssignmentEnrollment, Response> {
    let enrollment = store
        .get_enrollment(authenticated.tenant_context, enrollment_id)
        .await
        .map_err(store_error_response)?
        .ok_or_else(|| error_response(StatusCode::NOT_FOUND, "enrollment not found"))?;
    if enrollment.user != authenticated.record.subject.user() {
        return Err(error_response(
            StatusCode::NOT_FOUND,
            "enrollment not found",
        ));
    }
    Ok(enrollment)
}

async fn owned_assignment_for_run<S: Store>(
    store: &S,
    authenticated: &AuthenticatedSession,
    run: &AssignmentRun,
) -> Result<learning_data_access::AssignmentRecord, Response> {
    let enrollment = owned_enrollment(store, authenticated, run.enrollment).await?;
    store
        .get_assignment(authenticated.tenant_context, enrollment.assignment)
        .await
        .map_err(store_error_response)?
        .ok_or_else(|| error_response(StatusCode::NOT_FOUND, "assignment not found"))
}

async fn authorized_enrollment<S: Store>(
    store: &S,
    authenticated: &AuthenticatedSession,
    enrollment_id: question_model::EnrollmentId,
    require_owner: bool,
) -> Result<AssignmentEnrollment, Response> {
    let enrollment = store
        .get_enrollment(authenticated.tenant_context, enrollment_id)
        .await
        .map_err(store_error_response)?
        .ok_or_else(|| error_response(StatusCode::NOT_FOUND, "enrollment not found"))?;
    if enrollment.user == authenticated.record.subject.user() {
        return Ok(enrollment);
    }
    if require_owner {
        return Err(error_response(
            StatusCode::NOT_FOUND,
            "enrollment not found",
        ));
    }
    let assignment = store
        .get_assignment(authenticated.tenant_context, enrollment.assignment)
        .await
        .map_err(store_error_response)?
        .ok_or_else(|| error_response(StatusCode::NOT_FOUND, "enrollment not found"))?;
    let course = store
        .get_course(authenticated.tenant_context, assignment.course_id)
        .await
        .map_err(store_error_response)?
        .ok_or_else(|| error_response(StatusCode::NOT_FOUND, "enrollment not found"))?;
    let administrator = authenticated
        .record
        .subject
        .roles()
        .contains(&UserRole::Administrator);
    let instructor = course
        .role_for(authenticated.record.subject.user())
        .is_some_and(|role| matches!(role, question_model::CourseRole::Instructor));
    if administrator || instructor {
        Ok(enrollment)
    } else {
        Err(error_response(
            StatusCode::NOT_FOUND,
            "enrollment not found",
        ))
    }
}

async fn apply_feedback_disclosure<S: CatalogStore>(
    store: &S,
    context: learning_data_access::TenantContext,
    run: &AssignmentRun,
    attempt: &mut QuestionAttempt,
) -> Result<(), Response> {
    let question = store
        .get_catalog_problem(
            context,
            ProblemVersionRef {
                problem: attempt.problem,
                version: attempt.question_version,
            },
        )
        .await
        .map_err(store_error_response)?
        .ok_or_else(|| error_response(StatusCode::NOT_FOUND, "question version not found"))?;
    let retain_legacy_result = match question.question.attempt_policy.feedback {
        // AttemptResult includes points, while ImmediateCorrectness permits
        // only correctness. The receipt's allowlist projection carries that
        // fact; this legacy field must not smuggle score data around it.
        FeedbackDisclosure::ImmediateCorrectness | FeedbackDisclosure::OnRelease => false,
        FeedbackDisclosure::ImmediateFull => true,
        FeedbackDisclosure::Deferred => run.completed_at.is_some(),
    };
    if !retain_legacy_result {
        attempt.result = None;
    }
    Ok(())
}

async fn finish_submission<S, B>(
    store: &S,
    backend: &B,
    authenticated: &AuthenticatedSession,
    record: SubmissionRecord,
) -> Response
where
    S: Store + CatalogStore,
    B: RunBackend,
{
    let actor = authenticated.record.subject.user();
    let next_state = match store
        .submission_next_attempt(authenticated.tenant_context, actor, record.attempt.id)
        .await
    {
        Ok(value) => value,
        Err(error) => return store_error_response(error),
    };
    let next_state = if matches!(
        next_state,
        learning_data_access::SubmissionNextAttempt::Pending
    ) {
        // A process can fail after committing the grade and before this route
        // issues/finalizes its successor. Heal using *current* run state, but
        // never derive a replay receipt from whichever later attempt is active.
        let current_run = match store
            .get_run(authenticated.tenant_context, record.run.id)
            .await
        {
            Ok(Some(value)) => value,
            Ok(None) => return error_response(StatusCode::NOT_FOUND, "run not found"),
            Err(error) => return store_error_response(error),
        };
        if current_run.completed_at.is_none()
            && let Err(response) = ensure_active_questions(
                store,
                backend,
                authenticated,
                &current_run,
                Some(record.attempt.id),
            )
            .await
        {
            return response;
        }
        match store
            .submission_next_attempt(authenticated.tenant_context, actor, record.attempt.id)
            .await
        {
            Ok(learning_data_access::SubmissionNextAttempt::Pending) => {
                if let Err(error) = store
                    .finalize_submission_next_attempt(
                        authenticated.tenant_context,
                        actor,
                        record.attempt.id,
                        None,
                    )
                    .await
                {
                    return store_error_response(error);
                }
                learning_data_access::SubmissionNextAttempt::None
            }
            Ok(value) => value,
            Err(error) => return store_error_response(error),
        }
    } else {
        next_state
    };
    let next_issued = match next_state {
        learning_data_access::SubmissionNextAttempt::None => None,
        learning_data_access::SubmissionNextAttempt::Issued(id) => {
            let attempt = match store
                .get_question_attempt(authenticated.tenant_context, id)
                .await
            {
                Ok(Some(value)) => value,
                Ok(None) => {
                    return error_response(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "issued next attempt is unavailable",
                    );
                }
                Err(error) => return store_error_response(error),
            };
            Some(NextIssuedAttempt {
                id: attempt.id,
                run: attempt.run,
                question_version: attempt.question_version,
                seed: Seed::new(attempt.seed),
                deadline: attempt.timer.deadline,
                assignment_position: attempt.assignment_position,
                rendered_question_sha256: attempt.provenance.rendered_question_sha256,
            })
        }
        learning_data_access::SubmissionNextAttempt::Pending => {
            unreachable!("pending state is finalized above")
        }
    };
    submission_response(store, authenticated.tenant_context, record, next_issued).await
}

async fn submission_response<S: CatalogStore>(
    store: &S,
    context: learning_data_access::TenantContext,
    record: SubmissionRecord,
    next_issued: Option<NextIssuedAttempt>,
) -> Response {
    let feedback = match feedback_projection(
        store,
        context,
        &record.run,
        &record.attempt,
        record.feedback.content(),
    )
    .await
    {
        Ok(feedback) => feedback,
        Err(response) => return response,
    };
    let mut attempt = record.attempt;
    if let Err(response) =
        apply_feedback_disclosure(store, context, &record.run, &mut attempt).await
    {
        return response;
    }
    no_store(
        Json(SubmissionReceipt {
            accepted: true,
            attempt,
            feedback,
            next_issued,
        })
        .into_response(),
    )
}

async fn feedback_projection<S: CatalogStore>(
    store: &S,
    context: TenantContext,
    run: &AssignmentRun,
    attempt: &QuestionAttempt,
    content: &FeedbackContent,
) -> Result<Option<DisclosedFeedback>, Response> {
    let question = store
        .get_catalog_problem(
            context,
            ProblemVersionRef {
                problem: attempt.problem,
                version: attempt.question_version,
            },
        )
        .await
        .map_err(store_error_response)?
        .ok_or_else(|| error_response(StatusCode::NOT_FOUND, "question version not found"))?;
    Ok(project_feedback(
        question.question.attempt_policy.feedback,
        FeedbackDisclosureState {
            run_completed: run.completed_at.is_some(),
            // Release records are intentionally not implemented in this work
            // package, so policy remains honestly locked until that boundary.
            released: false,
        },
        attempt.result,
        content,
    ))
}

fn fresh_seed() -> Result<u64, RunBackendError> {
    let mut bytes = [0_u8; 8];
    getrandom::fill(&mut bytes).map_err(|error| RunBackendError::Unavailable(error.to_string()))?;
    // The generated TypeScript contract represents seeds as JavaScript
    // numbers. Restrict newly issued API values to the exact integer range;
    // seeded generation inside Rust still accepts the full u64 domain.
    Ok(u64::from_le_bytes(bytes) & MAX_JSON_SAFE_INTEGER)
}

fn submission_key(headers: &HeaderMap) -> Result<SubmissionIdempotencyKey, &'static str> {
    let value = headers
        .get(IDEMPOTENCY_HEADER)
        .ok_or("idempotency-key is required")?
        .to_str()
        .map_err(|_| "idempotency-key is invalid")?;
    SubmissionIdempotencyKey::parse(value).map_err(|_| "idempotency-key is invalid")
}

fn page_request(query: RunQuery) -> Result<PageRequest, PaginationError> {
    let size = PageSize::new(query.page_size.unwrap_or(DEFAULT_PAGE_SIZE))?;
    match query.cursor {
        Some(cursor) => Ok(PageRequest::after(Cursor::parse(cursor)?, size)),
        None => Ok(PageRequest::first(size)),
    }
}

fn backend_error_response(error: RunBackendError) -> Response {
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

fn store_error_response(error: StoreError) -> Response {
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

fn error_response(status: StatusCode, message: &str) -> Response {
    no_store((status, Json(serde_json::json!({ "error": message }))).into_response())
}

#[cfg(test)]
mod tests {
    mod manual_grading_http;

    use std::collections::BTreeMap;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::*;
    use axum::body::{Body, to_bytes};
    use axum::http::Request;
    use grading::{AnswerKey, GradingError, grade};
    use learning_data_access::in_memory::MemoryStore;
    use learning_data_access::{
        AssignmentRecord, CatalogTransition, CourseRecord, DraftRecord, JobLeaseDuration,
        JobPayload, JobStore, Page, PublishDraftCommand, PublishedProblemRecord,
        RetentionWorkerCommand, RetentionWorkerStore, SessionLifetime, SessionRecord,
        SessionSubject, SessionTokenHash, TenantContext,
    };
    use question_model::answer::{NumericTolerance, SelectionCardinality};
    use question_model::envelope::ContentBlock;
    use question_model::generation::Seed;
    use question_model::generation::{GeneratorReference, ParameterSpec, RandomizationDefinition};
    use question_model::response::ResponseDefinition;
    use question_model::response::{ChoiceId, ChoiceOption};
    use question_model::run_policy::{
        AttemptPolicy, CompletionRequirement, ContinuedPractice, GradePolicy, RunPolicies,
        TimingPolicy, VariationPolicy,
    };
    use question_model::taxonomy::License;
    use question_model::{
        ActivityTimestamp, BackendCapabilities, Capability, CatalogProblemSummary, CourseId,
        CourseMembership, CourseMembershipRole, DraftQuestionDefinition, DraftQuestionSource,
        EnrollmentId, GradingDefinition, ImplementationVersion, ObjectId, ProblemId,
        PublicationScope, QuestionMetadata, QuestionSource, StudentId, TenantId, UserId, VersionId,
        WorkspaceId,
    };
    use tower::ServiceExt;
    use uuid::Uuid;

    use crate::imathas_backend::{ExternalToolSubmissionBackend, ImathasBackend};
    use crate::native_backend::NativeBackend;

    #[derive(Debug, Default)]
    struct NumericBackend {
        grade_calls: AtomicUsize,
        reproduce_calls: AtomicUsize,
        external_launch_calls: AtomicUsize,
        issued_seeds: std::sync::Mutex<Vec<u64>>,
        external_tool_launch_ready: bool,
        manual_grading_required: bool,
    }

    struct CountingNativeBackend {
        inner: NativeBackend<MemoryStore>,
        submissions: AtomicUsize,
    }

    struct OpaqueRenderedHashBackend {
        inner: Arc<CountingNativeBackend>,
    }

    struct CountingExternalRouteBackend {
        inner: Arc<ContractedRouteBackend>,
        create_calls: AtomicUsize,
        proxy_calls: AtomicUsize,
        submission_calls: AtomicUsize,
    }

    #[async_trait]
    impl RunBackend for CountingExternalRouteBackend {
        async fn issue(
            &self,
            context: TenantContext,
            reference: ProblemVersionRef,
            question: &QuestionDefinition,
            seed: u64,
        ) -> Result<IssuedAttemptMetadata, RunBackendError> {
            self.inner.issue(context, reference, question, seed).await
        }

        async fn reproduce(
            &self,
            context: TenantContext,
            reference: ProblemVersionRef,
            question: &QuestionDefinition,
            attempt: &QuestionAttempt,
        ) -> Result<QuestionEnvelope, RunBackendError> {
            self.inner
                .reproduce(context, reference, question, attempt)
                .await
        }

        async fn grade(
            &self,
            context: TenantContext,
            reference: ProblemVersionRef,
            question: &QuestionDefinition,
            attempt: &QuestionAttempt,
            response: &StudentResponse,
        ) -> Result<GradeOutcome, RunBackendError> {
            self.inner
                .grade(context, reference, question, attempt, response)
                .await
        }

        async fn submit(
            &self,
            submission: RunSubmission<'_>,
        ) -> Result<SubmissionDisposition, RunBackendError> {
            self.inner.submit(submission).await
        }
    }

    #[async_trait]
    impl ExternalToolLaunchBackend for CountingExternalRouteBackend {
        async fn create_external_tool_launch(
            &self,
            context: TenantContext,
            actor: UserId,
            reference: ProblemVersionRef,
            question: &QuestionDefinition,
            attempt: &QuestionAttempt,
            aead: &crate::imathas_backend::LaunchStateAead,
        ) -> Result<learning_data_access::CreatedExternalToolLaunchSession, RunBackendError>
        {
            self.create_calls.fetch_add(1, Ordering::SeqCst);
            self.inner
                .create_external_tool_launch(context, actor, reference, question, attempt, aead)
                .await
        }

        async fn proxy_external_tool_activity(
            &self,
            context: TenantContext,
            actor: UserId,
            reference: ProblemVersionRef,
            question: &QuestionDefinition,
            attempt: &QuestionAttempt,
            session_id: Uuid,
            token: &learning_data_access::ExternalToolLaunchToken,
            method: adapter_imathas::broker_provider::ProxyMethod,
            body: &[u8],
            aead: &crate::imathas_backend::LaunchStateAead,
        ) -> Result<adapter_imathas::broker_provider::ProxyResponse, RunBackendError> {
            self.proxy_calls.fetch_add(1, Ordering::SeqCst);
            self.inner
                .proxy_external_tool_activity(
                    context, actor, reference, question, attempt, session_id, token, method, body,
                    aead,
                )
                .await
        }
    }

    #[async_trait]
    impl ExternalToolSubmissionBackend for CountingExternalRouteBackend {
        async fn submit_external_tool(
            &self,
            context: TenantContext,
            actor: UserId,
            reference: ProblemVersionRef,
            question: &QuestionDefinition,
            attempt: &QuestionAttempt,
            idempotency_key: learning_data_access::SubmissionIdempotencyKey,
            launch_proof: learning_data_access::ExternalToolLaunchProof,
            state_aead: &crate::imathas_backend::LaunchStateAead,
        ) -> Result<SubmissionDisposition, RunBackendError> {
            self.submission_calls.fetch_add(1, Ordering::SeqCst);
            self.inner
                .submit_external_tool(
                    context,
                    actor,
                    reference,
                    question,
                    attempt,
                    idempotency_key,
                    launch_proof,
                    state_aead,
                )
                .await
        }
    }

    #[async_trait]
    impl RunBackend for CountingNativeBackend {
        async fn issue(
            &self,
            context: TenantContext,
            reference: ProblemVersionRef,
            question: &QuestionDefinition,
            seed: u64,
        ) -> Result<IssuedAttemptMetadata, RunBackendError> {
            self.inner.issue(context, reference, question, seed).await
        }

        async fn reproduce(
            &self,
            context: TenantContext,
            reference: ProblemVersionRef,
            question: &QuestionDefinition,
            attempt: &QuestionAttempt,
        ) -> Result<QuestionEnvelope, RunBackendError> {
            self.inner
                .reproduce(context, reference, question, attempt)
                .await
        }

        async fn grade(
            &self,
            context: TenantContext,
            reference: ProblemVersionRef,
            question: &QuestionDefinition,
            attempt: &QuestionAttempt,
            response: &StudentResponse,
        ) -> Result<GradeOutcome, RunBackendError> {
            self.inner
                .grade(context, reference, question, attempt, response)
                .await
        }

        async fn submit(
            &self,
            submission: RunSubmission<'_>,
        ) -> Result<SubmissionDisposition, RunBackendError> {
            self.submissions.fetch_add(1, Ordering::SeqCst);
            self.inner.submit(submission).await
        }
    }

    #[async_trait]
    impl RunBackend for OpaqueRenderedHashBackend {
        async fn issue(
            &self,
            context: TenantContext,
            reference: ProblemVersionRef,
            question: &QuestionDefinition,
            seed: u64,
        ) -> Result<IssuedAttemptMetadata, RunBackendError> {
            let mut issued = self.inner.issue(context, reference, question, seed).await?;
            issued.provenance.rendered_question_sha256 =
                format!("backend-owned-render-{seed:016x}");
            Ok(issued)
        }

        async fn reproduce(
            &self,
            context: TenantContext,
            reference: ProblemVersionRef,
            question: &QuestionDefinition,
            attempt: &QuestionAttempt,
        ) -> Result<QuestionEnvelope, RunBackendError> {
            let issued = self
                .issue(context, reference, question, attempt.seed)
                .await?;
            if issued.parameter_hash != attempt.parameter_hash
                || issued.provenance != attempt.provenance
            {
                return Err(RunBackendError::Invalid(
                    "opaque rendered artifact did not reproduce".to_string(),
                ));
            }
            Ok(issued.envelope)
        }

        async fn grade(
            &self,
            _context: TenantContext,
            _reference: ProblemVersionRef,
            _question: &QuestionDefinition,
            _attempt: &QuestionAttempt,
            _response: &StudentResponse,
        ) -> Result<GradeOutcome, RunBackendError> {
            Err(RunBackendError::Unsupported(
                "opaque-render test backend does not grade".to_string(),
            ))
        }
    }

    #[async_trait]
    impl RunBackend for NumericBackend {
        async fn issue(
            &self,
            _context: TenantContext,
            _reference: ProblemVersionRef,
            _question: &QuestionDefinition,
            seed: u64,
        ) -> Result<IssuedAttemptMetadata, RunBackendError> {
            self.issued_seeds.lock().expect("seed record").push(seed);
            Ok(IssuedAttemptMetadata {
                envelope: QuestionEnvelope {
                    version: _question.version,
                    seed: Seed::new(seed),
                    title: _question.metadata.title.clone(),
                    prompt: _question.prompt.clone(),
                    response: _question.response.clone(),
                },
                parameter_hash: format!("parameter-{seed:016x}"),
                provenance: AttemptProvenance {
                    adapter: implementation("test-native"),
                    renderer: None,
                    generator: None,
                    source_artifact: None,
                    asset_objects: Vec::new(),
                    grading: implementation("numeric"),
                    rendered_question_sha256: format!("rendered-{seed:016x}"),
                },
            })
        }

        async fn reproduce(
            &self,
            _context: TenantContext,
            reference: ProblemVersionRef,
            question: &QuestionDefinition,
            attempt: &QuestionAttempt,
        ) -> Result<QuestionEnvelope, RunBackendError> {
            self.reproduce_calls.fetch_add(1, Ordering::SeqCst);
            if attempt.problem != reference.problem
                || attempt.question_version != reference.version
                || question.version != reference.version
                || question.problem != reference.problem
            {
                return Err(RunBackendError::Invalid(
                    "attempt does not match its published question".to_string(),
                ));
            }
            Ok(QuestionEnvelope {
                version: question.version,
                seed: Seed::new(attempt.seed),
                title: question.metadata.title.clone(),
                prompt: question.prompt.clone(),
                response: question.response.clone(),
            })
        }

        async fn prepare_external_tool_launch(
            &self,
            _context: TenantContext,
            _reference: ProblemVersionRef,
            _question: &QuestionDefinition,
            _attempt: &QuestionAttempt,
        ) -> Result<(), RunBackendError> {
            self.external_launch_calls.fetch_add(1, Ordering::SeqCst);
            if self.external_tool_launch_ready {
                Ok(())
            } else {
                Err(RunBackendError::Unsupported(
                    "test backend has no external-tool broker".to_string(),
                ))
            }
        }

        async fn grade(
            &self,
            _context: TenantContext,
            _reference: ProblemVersionRef,
            question: &QuestionDefinition,
            _attempt: &QuestionAttempt,
            response: &StudentResponse,
        ) -> Result<GradeOutcome, RunBackendError> {
            self.grade_calls.fetch_add(1, Ordering::SeqCst);
            if self.manual_grading_required {
                return Ok(GradeOutcome::NeedsManualGrading);
            }
            grade(
                question,
                response,
                Some(&AnswerKey::Numeric { expected: 18.0 }),
            )
            .map_err(grading_error)
        }
    }

    fn grading_error(error: GradingError) -> RunBackendError {
        RunBackendError::Invalid(error.to_string())
    }

    fn implementation(id: &str) -> ImplementationVersion {
        ImplementationVersion {
            id: id.to_string(),
            version: "1".to_string(),
        }
    }

    fn id(value: u128) -> Uuid {
        Uuid::from_u128(value)
    }

    fn assignment_items(references: Vec<ProblemVersionRef>) -> Vec<question_model::AssignmentItem> {
        static NEXT_ITEM_ID: AtomicUsize = AtomicUsize::new(1_000_000);
        references
            .into_iter()
            .enumerate()
            .map(|(position, reference)| question_model::AssignmentItem {
                id: question_model::AssignmentItemId::from_uuid(id(NEXT_ITEM_ID
                    .fetch_add(1, Ordering::Relaxed)
                    as u128)),
                reference,
                position: u32::try_from(position).expect("test assignment position fits u32"),
                points_possible: question_model::PointValue::from_whole(1),
                delivery_state: question_model::AssignmentDeliveryState::Active,
                scoring_mode: question_model::AssignmentScoringMode::Normal,
            })
            .collect()
    }

    /// Deliberately violates the immutable catalog identity contract to prove
    /// that run routes stop before any trusted backend can expose or grade it.
    #[derive(Debug)]
    struct MismatchedCatalog {
        record: PublishedProblemRecord,
    }

    #[async_trait]
    impl CatalogStore for MismatchedCatalog {
        async fn publish_draft(
            &self,
            _context: TenantContext,
            _actor: UserId,
            _command: PublishDraftCommand,
        ) -> Result<PublishedProblemRecord, StoreError> {
            Err(StoreError::InvalidRecord(
                "test catalog is read-only".to_string(),
            ))
        }

        async fn get_catalog_problem(
            &self,
            _context: TenantContext,
            _reference: ProblemVersionRef,
        ) -> Result<Option<PublishedProblemRecord>, StoreError> {
            Ok(Some(self.record.clone()))
        }

        async fn list_catalog(
            &self,
            _context: TenantContext,
            _page: PageRequest,
        ) -> Result<Page<CatalogProblemSummary>, StoreError> {
            Err(StoreError::InvalidRecord(
                "test catalog is read-only".to_string(),
            ))
        }

        async fn list_catalog_taxonomy(
            &self,
            _context: TenantContext,
            _page: PageRequest,
        ) -> Result<Page<question_model::taxonomy::TaxonomyTerm>, StoreError> {
            Err(StoreError::InvalidRecord(
                "test catalog is read-only".to_string(),
            ))
        }

        async fn transition_catalog_problem(
            &self,
            _context: TenantContext,
            _actor: UserId,
            _reference: ProblemVersionRef,
            _transition: CatalogTransition,
        ) -> Result<PublishedProblemRecord, StoreError> {
            Err(StoreError::InvalidRecord(
                "test catalog is read-only".to_string(),
            ))
        }
    }

    fn authenticated_for_test(context: TenantContext) -> AuthenticatedSession {
        let subject = SessionSubject::new(
            context.tenant_id(),
            UserId::from_uuid(id(2)),
            "Route test student",
            vec![UserRole::Student],
        )
        .expect("test session subject");
        AuthenticatedSession {
            record: SessionRecord {
                token_hash: SessionTokenHash::compute(b"run-route-test-session"),
                subject,
                created_at: ActivityTimestamp::from_unix_millis(10_000),
                expires_at: ActivityTimestamp::from_unix_millis(20_000),
            },
            tenant_context: context,
        }
    }

    #[test]
    fn fresh_server_seeds_fit_the_exact_json_integer_range() {
        for _ in 0..128 {
            assert!(fresh_seed().expect("OS random seed") <= MAX_JSON_SAFE_INTEGER);
        }
    }

    #[tokio::test]
    async fn mismatched_published_identity_never_reaches_envelope_or_grading() {
        let (store, _, _, _, _, _, _) = fixture().await;
        let context = TenantContext::from_authenticated_session(TenantId::from_uuid(id(1)));
        let reference = ProblemVersionRef {
            problem: ProblemId::from_uuid(id(8)),
            version: VersionId::from_uuid(id(9)),
        };
        let mut record = store
            .get_catalog_problem(context, reference)
            .await
            .expect("catalog read")
            .expect("fixture published question");
        record.question.problem = ProblemId::from_uuid(id(99));
        let malformed_catalog = MismatchedCatalog { record };

        let response = load_run_question(
            &malformed_catalog,
            &authenticated_for_test(context),
            reference,
        )
        .await
        .expect_err("mismatched immutable question IDs must be refused");

        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }

    async fn fixture() -> (
        Arc<MemoryStore>,
        Arc<NumericBackend>,
        Router,
        String,
        String,
        AssignmentId,
        EnrollmentId,
    ) {
        fixture_with_response(
            ResponseDefinition::Numeric {
                tolerance: NumericTolerance::Absolute { epsilon: 0.1 },
                unit: Some("g/mol".to_string()),
            },
            false,
        )
        .await
    }

    async fn fixture_with_response(
        response: ResponseDefinition,
        external_tool_launch_ready: bool,
    ) -> (
        Arc<MemoryStore>,
        Arc<NumericBackend>,
        Router,
        String,
        String,
        AssignmentId,
        EnrollmentId,
    ) {
        let store = Arc::new(MemoryStore::default());
        store
            .set_authoritative_time(ActivityTimestamp::from_unix_millis(10_000))
            .expect("fixture clock");
        let tenant = TenantId::from_uuid(id(1));
        let context = TenantContext::from_authenticated_session(tenant);
        let instructor = UserId::from_uuid(id(2));
        let student = UserId::from_uuid(id(3));
        let outsider = UserId::from_uuid(id(4));
        let course = CourseId::from_uuid(id(5));
        let assignment = AssignmentId::from_uuid(id(6));
        let enrollment = EnrollmentId::from_uuid(id(7));
        let problem = ProblemId::from_uuid(id(8));
        let version = VersionId::from_uuid(id(9));
        let workspace = WorkspaceId::from_uuid(id(10));
        let draft = DraftRecord {
            tenant,
            question: DraftQuestionDefinition {
                workspace,
                source: DraftQuestionSource::Native {
                    family: "test_numeric".to_string(),
                },
                prompt: vec![ContentBlock::Text {
                    markdown: "What is the molar mass of water?".to_string(),
                }],
                response,
                attempt_policy: AttemptPolicy {
                    max_attempts: None,
                    feedback: FeedbackDisclosure::ImmediateFull,
                },
                timing_policy: TimingPolicy::Untimed,
                randomization: RandomizationDefinition::Static,
                grading: GradingDefinition::AllOrNothing { points: 1.0 },
                metadata: QuestionMetadata {
                    title: "Water molar mass".to_string(),
                    tags: Vec::new(),
                    taxonomy: Vec::new(),
                    license: License::CcBy,
                    language: "en-US".to_string(),
                },
            },
            revises: None,
            derived_from: None,
        };
        let saved = store
            .upsert_draft(context, instructor, None, draft.clone())
            .await
            .expect("draft");
        store
            .publish_draft(
                context,
                instructor,
                PublishDraftCommand {
                    expected_draft: draft,
                    expected_revision: saved.revision,
                    publication: ProblemVersionRef { problem, version },
                    published_source: QuestionSource::Native {
                        family: "test_numeric".to_string(),
                    },
                    source_artifact: None,
                    qti_promotion: None,
                    flat_question_promotion: None,
                    publisher: instructor,
                    scope: PublicationScope::Public,
                    capabilities: BackendCapabilities::from_iter([
                        Capability::AlgorithmicGeneration,
                        Capability::ServerGrading,
                    ]),
                },
            )
            .await
            .expect("publish");
        store
            .upsert_course(
                context,
                CourseRecord {
                    id: course,
                    tenant,
                    title: "Biochemistry".to_string(),
                    members: vec![
                        CourseMembership {
                            user: instructor,
                            role: CourseMembershipRole::Instructor,
                        },
                        CourseMembership {
                            user: student,
                            role: CourseMembershipRole::Student,
                        },
                    ],
                },
            )
            .await
            .expect("course");
        store
            .create_assignment(
                context,
                AssignmentRecord {
                    id: assignment,
                    tenant,
                    course_id: course,
                    title: "Molar mass mastery".to_string(),
                    items: assignment_items(vec![ProblemVersionRef { problem, version }]),
                    selection_groups: Vec::new(),
                    policies: RunPolicies {
                        completion: CompletionRequirement::AllCorrect,
                        grade: GradePolicy::Highest,
                        continued_practice: ContinuedPractice::Unlimited,
                        variation: VariationPolicy::NewSeeds,
                    },
                },
            )
            .await
            .expect("assignment");
        store
            .create_enrollment(
                context,
                AssignmentEnrollment {
                    id: enrollment,
                    tenant,
                    assignment,
                    user: student,
                    student: StudentId::from_uuid(id(11)),
                    first_completed_at: None,
                    current_grade_run: None,
                    best_grade_run: None,
                },
            )
            .await
            .expect("enrollment");
        let student_cookie = issued_cookie(store.as_ref(), student, "Student").await;
        let outsider_cookie = issued_cookie(store.as_ref(), outsider, "Outsider").await;
        let backend = Arc::new(NumericBackend {
            external_tool_launch_ready,
            ..NumericBackend::default()
        });
        let app = router(Arc::clone(&store), Arc::clone(&backend));
        (
            store,
            backend,
            app,
            student_cookie,
            outsider_cookie,
            assignment,
            enrollment,
        )
    }

    async fn prepare_archive_fence(store: &MemoryStore, tenant: TenantId, course: CourseId) {
        store
            .seed_retention_cleanup_for_test(
                tenant,
                course,
                (0..4)
                    .map(|offset| ObjectId::from_uuid(id(900 + offset)))
                    .collect(),
            )
            .expect("archive cleanup fixture");
        let claim = store
            .claim_next_job(
                &learning_data_access::JobClaimFilter::all(),
                JobLeaseDuration::from_seconds(30).expect("lease duration"),
            )
            .await
            .expect("archive claim")
            .expect("archive job");
        let (claimed_course, stage, generation) = match claim.payload {
            JobPayload::Retention {
                course,
                stage,
                generation,
            } => (course, stage, generation),
            _ => panic!("fixture must claim retention work"),
        };
        assert_eq!(claimed_course, course);
        store
            .prepare_retention_work(RetentionWorkerCommand {
                tenant,
                course,
                stage,
                generation,
                job: claim.id,
                lease: claim.lease_token,
            })
            .await
            .expect("archive prepare fence");
    }

    fn peptide_choice(id: &str, body: &str) -> ChoiceOption {
        ChoiceOption {
            id: ChoiceId::new(id),
            body: vec![ContentBlock::Text {
                markdown: body.to_string(),
            }],
        }
    }

    async fn native_feedback_fixture(
        policy: FeedbackDisclosure,
    ) -> (
        Arc<MemoryStore>,
        Arc<CountingNativeBackend>,
        Router,
        String,
        String,
        AssignmentId,
    ) {
        let store = Arc::new(MemoryStore::default());
        store
            .set_authoritative_time(ActivityTimestamp::from_unix_millis(10_000))
            .expect("fixture clock");
        let tenant = TenantId::from_uuid(id(201));
        let context = TenantContext::from_authenticated_session(tenant);
        let instructor = UserId::from_uuid(id(202));
        let student = UserId::from_uuid(id(203));
        let outsider = UserId::from_uuid(id(204));
        let course = CourseId::from_uuid(id(205));
        let assignment = AssignmentId::from_uuid(id(206));
        let problem = ProblemId::from_uuid(id(207));
        let version = VersionId::from_uuid(id(208));
        let workspace = WorkspaceId::from_uuid(id(209));
        let mut parameters = BTreeMap::new();
        parameters.insert(
            "residue".to_string(),
            ParameterSpec::Choice {
                options: vec!["alanine".to_string(), "glycine".to_string()],
            },
        );
        let draft = DraftRecord {
            tenant,
            question: DraftQuestionDefinition {
                workspace,
                source: DraftQuestionSource::Native {
                    family: adapter_native::peptide_bond_geometry::FAMILY_ID.to_string(),
                },
                prompt: vec![ContentBlock::Text {
                    markdown: "In a peptide containing {{residue}}, which linkage is planar?"
                        .to_string(),
                }],
                response: ResponseDefinition::MultipleChoice {
                    choices: vec![
                        peptide_choice("ester", "An ester linkage"),
                        peptide_choice("amide", "The peptide linkage"),
                        peptide_choice("ether", "An ether linkage"),
                    ],
                    selection: SelectionCardinality::ExactlyOne,
                },
                attempt_policy: AttemptPolicy {
                    max_attempts: None,
                    feedback: policy,
                },
                timing_policy: TimingPolicy::Untimed,
                randomization: RandomizationDefinition::Seeded {
                    generator: GeneratorReference {
                        id: adapter_native::peptide_bond_geometry::GENERATOR_ID.to_string(),
                        version: adapter_native::peptide_bond_geometry::GENERATOR_VERSION
                            .to_string(),
                    },
                    parameters,
                },
                grading: GradingDefinition::AllOrNothing { points: 2.0 },
                metadata: QuestionMetadata {
                    title: "Peptide-bond geometry".to_string(),
                    tags: Vec::new(),
                    taxonomy: Vec::new(),
                    license: License::CcBy,
                    language: "en-US".to_string(),
                },
            },
            revises: None,
            derived_from: None,
        };
        let saved = store
            .upsert_draft(context, instructor, None, draft.clone())
            .await
            .expect("draft");
        store
            .publish_draft(
                context,
                instructor,
                PublishDraftCommand {
                    expected_draft: draft,
                    expected_revision: saved.revision,
                    publication: ProblemVersionRef { problem, version },
                    published_source: QuestionSource::Native {
                        family: adapter_native::peptide_bond_geometry::FAMILY_ID.to_string(),
                    },
                    source_artifact: None,
                    qti_promotion: None,
                    flat_question_promotion: None,
                    publisher: instructor,
                    scope: PublicationScope::Institution,
                    capabilities: BackendCapabilities::from_iter([
                        Capability::AlgorithmicGeneration,
                        Capability::ClientRendering,
                        Capability::ServerGrading,
                        Capability::Hints,
                    ]),
                },
            )
            .await
            .expect("publish");
        store
            .upsert_course(
                context,
                CourseRecord {
                    id: course,
                    tenant,
                    title: "Biochemistry".to_string(),
                    members: vec![
                        CourseMembership {
                            user: instructor,
                            role: CourseMembershipRole::Instructor,
                        },
                        CourseMembership {
                            user: student,
                            role: CourseMembershipRole::Student,
                        },
                    ],
                },
            )
            .await
            .expect("course");
        store
            .create_assignment(
                context,
                AssignmentRecord {
                    id: assignment,
                    tenant,
                    course_id: course,
                    title: "Peptide feedback".to_string(),
                    items: assignment_items(vec![
                        ProblemVersionRef { problem, version },
                        ProblemVersionRef { problem, version },
                    ]),
                    selection_groups: Vec::new(),
                    policies: RunPolicies {
                        completion: CompletionRequirement::AnswerAll,
                        grade: GradePolicy::Highest,
                        continued_practice: ContinuedPractice::Unlimited,
                        variation: VariationPolicy::NewSeeds,
                    },
                },
            )
            .await
            .expect("assignment");
        store
            .create_enrollment(
                context,
                AssignmentEnrollment {
                    id: EnrollmentId::from_uuid(id(210)),
                    tenant,
                    assignment,
                    user: student,
                    student: StudentId::from_uuid(id(211)),
                    first_completed_at: None,
                    current_grade_run: None,
                    best_grade_run: None,
                },
            )
            .await
            .expect("enrollment");
        let student_cookie = issued_cookie_for(store.as_ref(), tenant, student, "Student").await;
        let outsider_cookie = issued_cookie_for(store.as_ref(), tenant, outsider, "Outsider").await;
        let backend = Arc::new(CountingNativeBackend {
            inner: NativeBackend::new(
                Arc::new(adapter_native::NativeAdapter::new()),
                Arc::clone(&store),
            ),
            submissions: AtomicUsize::new(0),
        });
        let app = router(Arc::clone(&store), Arc::clone(&backend));
        (
            store,
            backend,
            app,
            student_cookie,
            outsider_cookie,
            assignment,
        )
    }

    async fn issued_cookie(store: &MemoryStore, user: UserId, name: &str) -> String {
        issued_cookie_for(store, TenantId::from_uuid(id(1)), user, name).await
    }

    async fn issued_cookie_for(
        store: &MemoryStore,
        tenant: TenantId,
        user: UserId,
        name: &str,
    ) -> String {
        let subject = SessionSubject::new(tenant, user, name, vec![UserRole::Student])
            .expect("session subject");
        let issued = crate::auth::issue_session(
            store,
            subject,
            crate::auth::SessionConfig::new(
                SessionLifetime::from_seconds(3_600).expect("session lifetime"),
                crate::auth::CookieTransport::LocalHttp,
            ),
        )
        .await
        .expect("session");
        issued
            .set_cookie
            .split(';')
            .next()
            .expect("cookie pair")
            .to_string()
    }

    type ContractedRouteBackend = ImathasBackend<
        MemoryStore,
        objects::memory::MemoryObjectStore,
        adapter_imathas::broker_provider::ContractedScoredEmbedProvider<
            adapter_imathas::test_support::RecordedContractedTransport,
        >,
    >;

    struct ContractedRouteFixture {
        store: Arc<MemoryStore>,
        objects: Arc<objects::memory::MemoryObjectStore>,
        source_key: objects::ObjectKey,
        backend: Arc<ContractedRouteBackend>,
        route_backend: Arc<CountingExternalRouteBackend>,
        transport: adapter_imathas::test_support::RecordedContractedTransport,
        aead: Arc<crate::imathas_backend::LaunchStateAead>,
        app: Router,
        student_cookie: String,
        outsider_cookie: String,
        attempt: QuestionAttempt,
        context: TenantContext,
        question: QuestionDefinition,
    }

    async fn contracted_route_fixture(
        transport_mode: adapter_imathas::test_support::RecordedContractedTransportMode,
    ) -> ContractedRouteFixture {
        use adapter_imathas::test_support::RecordedContractedTransportFactory;
        use learning_data_access::{IssueQuestionAttemptCommand, PublishedSourceArtifact};
        use objects::{ObjectKey, ObjectStore, PutObject, Sha256Digest};
        use question_model::generation::RandomizationDefinition;

        let store = Arc::new(MemoryStore::default());
        store
            .set_authoritative_time(ActivityTimestamp::from_unix_millis(10_000))
            .expect("fixture clock");
        let objects = Arc::new(objects::memory::MemoryObjectStore::default());
        let tenant = TenantId::from_uuid(id(801));
        let context = TenantContext::from_authenticated_session(tenant);
        let instructor = UserId::from_uuid(id(802));
        let actor = UserId::from_uuid(id(803));
        let outsider = UserId::from_uuid(id(804));
        let workspace = WorkspaceId::from_uuid(id(805));
        let problem = ProblemId::from_uuid(id(806));
        let version = VersionId::from_uuid(id(807));
        let snapshot = question_model::ObjectId::from_uuid(id(808));
        let source_bytes = br#"{"recorded":true}"#.to_vec();
        let source_sha256 = Sha256Digest::compute(&source_bytes).to_string();
        let source = QuestionSource::Imathas {
            provider: "institution-imathas".into(),
            item_ref: "17".into(),
            snapshot,
            snapshot_sha256: source_sha256,
            integration_profile: adapter_imathas::scored_embed::SCORED_EMBED_BROKER_PROFILE_ID
                .into(),
        };
        let draft = DraftRecord {
            tenant,
            question: DraftQuestionDefinition {
                workspace,
                source: DraftQuestionSource::Imathas {
                    provider: "institution-imathas".into(),
                    item_ref: "17".into(),
                },
                prompt: Vec::new(),
                response: ResponseDefinition::ExternalTool {},
                attempt_policy: AttemptPolicy {
                    max_attempts: None,
                    feedback: FeedbackDisclosure::ImmediateCorrectness,
                },
                timing_policy: TimingPolicy::Untimed,
                randomization: RandomizationDefinition::Static,
                grading: GradingDefinition::AllOrNothing { points: 1.0 },
                metadata: QuestionMetadata {
                    title: "Recorded contracted iMathAS question".into(),
                    tags: Vec::new(),
                    taxonomy: Vec::new(),
                    license: License::CcBySa,
                    language: "en-US".into(),
                },
            },
            revises: None,
            derived_from: None,
        };
        let reference = ProblemVersionRef { problem, version };
        let object_key = ObjectKey::ProblemSource {
            problem,
            version,
            object: snapshot,
        };
        objects
            .put(PutObject {
                key: object_key.clone(),
                bytes: source_bytes,
                media_type: "application/json".into(),
                license: "CC-BY-SA-4.0".into(),
                provenance: "recorded contracted route fixture".into(),
                created_at: ActivityTimestamp::from_unix_millis(10_000),
            })
            .await
            .expect("source object");
        let saved = store
            .upsert_draft(context, instructor, None, draft.clone())
            .await
            .expect("draft");
        let artifact = objects
            .get(&object_key)
            .await
            .expect("source record")
            .record;
        store
            .publish_draft(
                context,
                instructor,
                PublishDraftCommand {
                    expected_draft: draft,
                    expected_revision: saved.revision,
                    publication: reference,
                    published_source: source,
                    source_artifact: Some(PublishedSourceArtifact {
                        reference,
                        backend: question_model::QuestionBackend::Imathas,
                        object: artifact,
                    }),
                    qti_promotion: None,
                    flat_question_promotion: None,
                    publisher: instructor,
                    scope: PublicationScope::Public,
                    capabilities: BackendCapabilities::from_iter([
                        Capability::AlgorithmicGeneration,
                        Capability::ServerGrading,
                    ]),
                },
            )
            .await
            .expect("publish");
        let question = store
            .get_catalog_problem(context, reference)
            .await
            .expect("catalog")
            .expect("published")
            .question;
        let course = CourseId::from_uuid(id(809));
        let assignment = AssignmentId::from_uuid(id(810));
        let enrollment = EnrollmentId::from_uuid(id(811));
        store
            .upsert_course(
                context,
                CourseRecord {
                    id: course,
                    tenant,
                    title: "Recorded course".into(),
                    members: vec![
                        CourseMembership {
                            user: instructor,
                            role: CourseMembershipRole::Instructor,
                        },
                        CourseMembership {
                            user: actor,
                            role: CourseMembershipRole::Student,
                        },
                    ],
                },
            )
            .await
            .expect("course");
        store
            .create_assignment(
                context,
                AssignmentRecord {
                    id: assignment,
                    tenant,
                    course_id: course,
                    title: "Recorded assignment".into(),
                    items: assignment_items(vec![reference]),
                    selection_groups: Vec::new(),
                    policies: RunPolicies {
                        completion: CompletionRequirement::AllCorrect,
                        grade: GradePolicy::Highest,
                        continued_practice: ContinuedPractice::Unlimited,
                        variation: VariationPolicy::NewSeeds,
                    },
                },
            )
            .await
            .expect("assignment");
        store
            .create_enrollment(
                context,
                AssignmentEnrollment {
                    id: enrollment,
                    tenant,
                    assignment,
                    user: actor,
                    student: StudentId::from_uuid(id(812)),
                    first_completed_at: None,
                    current_grade_run: None,
                    best_grade_run: None,
                },
            )
            .await
            .expect("enrollment");
        let (provider, transport) = RecordedContractedTransportFactory::new(transport_mode)
            .contracted_provider_with_transport();
        let adapter = Arc::new(adapter_imathas::ImathasAdapter::new(
            objects.as_ref().clone(),
            provider,
            [adapter_imathas::SupportedProfile::new(
                adapter_imathas::scored_embed::SCORED_EMBED_BROKER_PROFILE_ID,
                true,
                true,
                true,
            )
            .expect("profile")],
        ));
        let backend = Arc::new(ImathasBackend::new(
            Arc::clone(&store),
            Arc::clone(&objects),
            adapter,
            Arc::new(adapter_imathas::CorrelationIssuer::from_server_secret(
                [83; 32],
            )),
        ));
        let run = store
            .start_or_resume_run(context, actor, assignment, RunId::from_uuid(id(813)))
            .await
            .expect("run");
        let issued = backend
            .issue(context, reference, &question, 17)
            .await
            .expect("issue");
        let attempt = store
            .issue_or_resume_question_attempt(
                context,
                IssueQuestionAttemptCommand {
                    actor,
                    attempt: QuestionAttemptId::from_uuid(id(814)),
                    run: run.id,
                    assignment_position: 0,
                    problem,
                    question_version: version,
                    seed: 17,
                    parameter_hash: issued.parameter_hash,
                    provenance: issued.provenance,
                    prefetched: None,
                    predecessor_submission: None,
                },
            )
            .await
            .expect("attempt");
        let aead = Arc::new(
            crate::imathas_backend::LaunchStateAead::from_server_secret([84; 32]).expect("aead"),
        );
        let route_backend = Arc::new(CountingExternalRouteBackend {
            inner: Arc::clone(&backend),
            create_calls: AtomicUsize::new(0),
            proxy_calls: AtomicUsize::new(0),
            submission_calls: AtomicUsize::new(0),
        });
        let app = external_tool_router(
            Arc::clone(&store),
            Arc::clone(&route_backend),
            Arc::clone(&aead),
        );
        ContractedRouteFixture {
            student_cookie: issued_cookie_for(store.as_ref(), tenant, actor, "Student").await,
            outsider_cookie: issued_cookie_for(store.as_ref(), tenant, outsider, "Outsider").await,
            store,
            objects,
            source_key: object_key,
            backend,
            route_backend,
            transport,
            aead,
            app,
            attempt,
            context,
            question,
        }
    }

    #[tokio::test]
    async fn contracted_imathas_submission_retrieves_once_commits_and_replays_after_revoke() {
        use adapter_imathas::test_support::RecordedContractedTransportMode;

        let fixture = contracted_route_fixture(RecordedContractedTransportMode::Verified).await;
        let launch_path = format!("/api/attempts/{}/external-tool/launch", fixture.attempt.id);
        let launch = fixture
            .app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(&launch_path)
                    .header("cookie", &fixture.student_cookie)
                    .body(Body::empty())
                    .expect("launch request"),
            )
            .await
            .expect("launch response");
        assert_eq!(launch.status(), StatusCode::OK);
        let launch_cookie = launch.headers()["set-cookie"]
            .to_str()
            .expect("launch cookie")
            .split(';')
            .next()
            .expect("cookie pair")
            .to_owned();
        let submission_path = format!("{launch_path}/submission");
        let request = || {
            Request::builder()
                .method("POST")
                .uri(&submission_path)
                .header(
                    "cookie",
                    format!("{}; {launch_cookie}", fixture.student_cookie),
                )
                .header("idempotency-key", "recorded-contracted-submit")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"response":{"kind":"externalTool"}}"#))
                .expect("submission request")
        };
        let first = fixture
            .app
            .clone()
            .oneshot(request())
            .await
            .expect("first response");
        assert_eq!(first.status(), StatusCode::OK);
        assert_eq!(first.headers()["cache-control"], "no-store");
        let first_body = to_bytes(first.into_body(), 256 * 1024)
            .await
            .expect("first body");
        assert!(
            std::str::from_utf8(&first_body)
                .expect("receipt UTF-8")
                .contains("\"accepted\":true")
        );
        assert_eq!(fixture.transport.result_calls(), 1);

        let replay = fixture
            .app
            .clone()
            .oneshot(request())
            .await
            .expect("replay response");
        assert_eq!(replay.status(), StatusCode::OK);
        let replay_body = to_bytes(replay.into_body(), 256 * 1024)
            .await
            .expect("replay body");
        assert_eq!(replay_body, first_body);
        assert_eq!(fixture.transport.result_calls(), 1);
    }

    #[tokio::test]
    async fn contracted_imathas_submission_refuses_missing_copied_and_malformed_markers_before_provider()
     {
        use adapter_imathas::test_support::RecordedContractedTransportMode;

        let fixture = contracted_route_fixture(RecordedContractedTransportMode::Verified).await;
        let launch_path = format!("/api/attempts/{}/external-tool/launch", fixture.attempt.id);
        let launch = fixture
            .app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(&launch_path)
                    .header("cookie", &fixture.student_cookie)
                    .body(Body::empty())
                    .expect("launch"),
            )
            .await
            .expect("launch response");
        let launch_cookie = launch.headers()["set-cookie"]
            .to_str()
            .expect("cookie")
            .split(';')
            .next()
            .expect("pair")
            .to_owned();
        let path = format!("{launch_path}/submission");
        for (cookie, body) in [
            (
                fixture.student_cookie.clone(),
                r#"{"response":{"kind":"externalTool"}}"#,
            ),
            (
                format!("{}; {launch_cookie}", fixture.outsider_cookie),
                r#"{"response":{"kind":"externalTool"}}"#,
            ),
            (
                format!("{}; {launch_cookie}", fixture.student_cookie),
                r#"{"response":{"kind":"externalTool","score":1}}"#,
            ),
        ] {
            let response = fixture
                .app
                .clone()
                .oneshot(
                    Request::builder()
                        .method("POST")
                        .uri(&path)
                        .header("cookie", cookie)
                        .header("idempotency-key", "refused-marker")
                        .header("content-type", "application/json")
                        .body(Body::from(body))
                        .expect("request"),
                )
                .await
                .expect("response");
            assert!(
                matches!(
                    response.status(),
                    StatusCode::NOT_FOUND
                        | StatusCode::BAD_REQUEST
                        | StatusCode::UNPROCESSABLE_ENTITY
                ),
                "unexpected status: {}",
                response.status()
            );
        }
        assert_eq!(fixture.transport.result_calls(), 0);
    }

    #[tokio::test]
    async fn archive_fence_refuses_external_tool_routes_before_provider_calls() {
        use adapter_imathas::test_support::RecordedContractedTransportMode;

        let fixture = contracted_route_fixture(RecordedContractedTransportMode::Verified).await;
        let calls_before = (
            fixture.transport.proxy_calls(),
            fixture.transport.result_calls(),
            fixture.route_backend.create_calls.load(Ordering::SeqCst),
            fixture.route_backend.proxy_calls.load(Ordering::SeqCst),
            fixture
                .route_backend
                .submission_calls
                .load(Ordering::SeqCst),
        );
        prepare_archive_fence(
            fixture.store.as_ref(),
            TenantId::from_uuid(id(801)),
            CourseId::from_uuid(id(809)),
        )
        .await;

        let launch_path = format!("/api/attempts/{}/external-tool/launch", fixture.attempt.id);
        let requests = vec![
            Request::builder()
                .uri(&launch_path)
                .header("cookie", &fixture.student_cookie)
                .body(Body::empty())
                .expect("archived shell request"),
            Request::builder()
                .uri(format!("{launch_path}/activity"))
                .header("cookie", &fixture.student_cookie)
                .body(Body::empty())
                .expect("archived activity GET"),
            Request::builder()
                .method("POST")
                .uri(format!("{launch_path}/activity"))
                .header("cookie", &fixture.student_cookie)
                .body(Body::empty())
                .expect("archived activity POST"),
            Request::builder()
                .method("POST")
                .uri(format!("{launch_path}/submission"))
                .header("cookie", &fixture.student_cookie)
                .header("idempotency-key", "archived-external")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"response":{"kind":"externalTool"}}"#))
                .expect("archived external submission"),
        ];
        for request in requests {
            let response = fixture
                .app
                .clone()
                .oneshot(request)
                .await
                .expect("archived external response");
            assert_eq!(response.status(), StatusCode::NOT_FOUND);
            assert_eq!(response.headers()["cache-control"], "no-store");
        }
        assert_eq!(
            (
                fixture.transport.proxy_calls(),
                fixture.transport.result_calls(),
                fixture.route_backend.create_calls.load(Ordering::SeqCst),
                fixture.route_backend.proxy_calls.load(Ordering::SeqCst),
                fixture
                    .route_backend
                    .submission_calls
                    .load(Ordering::SeqCst),
            ),
            calls_before
        );
    }

    #[tokio::test]
    async fn contracted_imathas_result_outage_stays_ungraded_and_replica_does_not_reretrieve() {
        use adapter_imathas::test_support::RecordedContractedTransportMode;

        let fixture =
            contracted_route_fixture(RecordedContractedTransportMode::ResultUnavailable).await;
        let actor = UserId::from_uuid(id(803));
        let reference = ProblemVersionRef {
            problem: fixture.attempt.problem,
            version: fixture.attempt.question_version,
        };
        let created = fixture
            .backend
            .create_contracted_launch_session(
                fixture.context,
                actor,
                reference,
                &fixture.question,
                &fixture.attempt,
                fixture.aead.as_ref(),
            )
            .await
            .expect("protected launch");
        let proof = learning_data_access::ExternalToolLaunchProof {
            session_id: created.id,
            token: created.token,
        };
        let key =
            learning_data_access::SubmissionIdempotencyKey::parse("recorded-contracted-outage")
                .expect("idempotency key");
        let first = fixture
            .backend
            .submit_external_tool(
                fixture.context,
                actor,
                reference,
                &fixture.question,
                &fixture.attempt,
                key.clone(),
                proof.clone(),
                fixture.aead.as_ref(),
            )
            .await;
        assert!(matches!(first, Err(RunBackendError::Unavailable(_))));
        assert_eq!(fixture.transport.result_calls(), 1);
        assert!(fixture.attempt.result.is_none());

        let replica = fixture
            .backend
            .submit_external_tool(
                fixture.context,
                actor,
                reference,
                &fixture.question,
                &fixture.attempt,
                key,
                proof,
                fixture.aead.as_ref(),
            )
            .await;
        assert!(matches!(replica, Err(RunBackendError::Unavailable(_))));
        assert_eq!(fixture.transport.result_calls(), 1);
    }

    #[tokio::test]
    async fn contracted_imathas_verified_pending_recovers_without_a_second_retrieval() {
        use adapter_imathas::test_support::RecordedContractedTransportMode;
        use adapter_imathas::{CorrelationIssuer, GradeBinding};
        use learning_data_access::{
            BeginExternalToolGradeCommand, ExternalToolBegin, ExternalToolBrokerStore,
            PersistedCorrelation, StageExternalToolVerificationCommand,
        };
        use objects::Sha256Digest;

        let fixture = contracted_route_fixture(RecordedContractedTransportMode::Verified).await;
        let actor = UserId::from_uuid(id(803));
        let reference = ProblemVersionRef {
            problem: fixture.attempt.problem,
            version: fixture.attempt.question_version,
        };
        let QuestionSource::Imathas {
            provider,
            snapshot,
            snapshot_sha256,
            integration_profile,
            ..
        } = &fixture.question.source
        else {
            panic!("contracted fixture source")
        };
        let response = StudentResponse::ExternalTool {};
        let binding = learning_data_access::ExternalToolBinding {
            provider: provider.clone(),
            problem: fixture.question.problem,
            version: fixture.question.version,
            seed: fixture.attempt.seed,
            source_object: *snapshot,
            source_sha256: snapshot_sha256.clone(),
            integration_profile: integration_profile.clone(),
            response_sha256: Sha256Digest::compute(
                &serde_json::to_vec(&response).expect("response"),
            ),
        };
        let grade_binding = GradeBinding {
            tenant: fixture.context.tenant_id(),
            attempt: fixture.attempt.id,
            problem: fixture.question.problem,
            version: fixture.question.version,
            seed: Seed::new(fixture.attempt.seed),
        };
        let issuer = CorrelationIssuer::from_server_secret([83; 32]);
        let correlation =
            PersistedCorrelation::new(issuer.begin(grade_binding).to_storage_value().into_bytes())
                .expect("correlation");
        let key =
            learning_data_access::SubmissionIdempotencyKey::parse("recorded-contracted-pending")
                .expect("key");
        let ExternalToolBegin::Lease(lease) = fixture
            .store
            .begin_or_resume_external_grade(
                fixture.context,
                BeginExternalToolGradeCommand {
                    actor,
                    attempt: fixture.attempt.id,
                    response: response.clone(),
                    idempotency_key: key.clone(),
                    binding: binding.clone(),
                    proposed_correlation: correlation,
                    lease_millis: 30_000,
                },
            )
            .await
            .expect("lease")
        else {
            panic!("fresh broker lease")
        };
        fixture
            .store
            .stage_external_tool_verification(
                fixture.context,
                StageExternalToolVerificationCommand {
                    actor,
                    attempt: fixture.attempt.id,
                    response,
                    idempotency_key: key.clone(),
                    binding,
                    correlation: lease.correlation,
                    lease_token: lease.token,
                    result: AttemptResult {
                        correct: true,
                        points_earned: 1.0,
                        points_possible: 1.0,
                    },
                },
            )
            .await
            .expect("stage pending");
        let created = fixture
            .backend
            .create_contracted_launch_session(
                fixture.context,
                actor,
                reference,
                &fixture.question,
                &fixture.attempt,
                fixture.aead.as_ref(),
            )
            .await
            .expect("launch proof");
        let recovered = fixture
            .backend
            .submit_external_tool(
                fixture.context,
                actor,
                reference,
                &fixture.question,
                &fixture.attempt,
                key,
                learning_data_access::ExternalToolLaunchProof {
                    session_id: created.id,
                    token: created.token,
                },
                fixture.aead.as_ref(),
            )
            .await
            .expect("commit staged grade");
        assert!(matches!(recovered, SubmissionDisposition::Committed(_)));
        assert_eq!(fixture.transport.result_calls(), 0);
    }

    #[tokio::test]
    async fn contracted_imathas_launch_route_is_same_origin_replica_safe_and_secret_free() {
        use adapter_imathas::test_support::RecordedContractedTransportMode;

        let fixture = contracted_route_fixture(RecordedContractedTransportMode::Available).await;
        fixture
            .backend
            .reproduce(
                fixture.context,
                ProblemVersionRef {
                    problem: fixture.attempt.problem,
                    version: fixture.attempt.question_version,
                },
                &fixture.question,
                &fixture.attempt,
            )
            .await
            .expect("preflight reproduce");
        let path = format!("/api/attempts/{}/external-tool/launch", fixture.attempt.id);
        let shell = fixture
            .app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(&path)
                    .header("cookie", &fixture.student_cookie)
                    .body(Body::empty())
                    .expect("shell request"),
            )
            .await
            .expect("shell response");
        if shell.status() != StatusCode::OK {
            let status = shell.status();
            let body = to_bytes(shell.into_body(), 256 * 1_024)
                .await
                .expect("error body");
            panic!(
                "shell returned {status}: {}",
                String::from_utf8_lossy(&body)
            );
        }
        assert_eq!(shell.headers()["cache-control"], "no-store");
        let set_cookie = shell.headers()["set-cookie"]
            .to_str()
            .expect("set cookie")
            .to_owned();
        assert!(set_cookie.starts_with("ple_external_launch="));
        assert!(
            set_cookie.contains("HttpOnly")
                && set_cookie.contains("Secure")
                && set_cookie.contains("SameSite=Strict")
        );
        assert!(set_cookie.contains(&format!("Path={path}")));
        let csp = shell.headers()["content-security-policy"]
            .to_str()
            .expect("csp")
            .to_owned();
        let shell_bytes = to_bytes(shell.into_body(), 256 * 1_024)
            .await
            .expect("shell body");
        let shell_body = std::str::from_utf8(&shell_bytes).expect("shell utf8");
        let nonce = shell_body
            .split("<script nonce=\"")
            .nth(1)
            .and_then(|v| v.split('"').next())
            .expect("script nonce");
        assert!(csp.contains(&format!("'nonce-{nonce}'")));
        assert!(shell_body.contains(&format!("src=\"{path}/activity\"")));
        assert!(shell_body.contains("kind:'ple.externalTool.ready'"));
        assert!(shell_body.contains("ple.externalTool.activityReady"));
        assert!(shell_body.contains(&format!("attemptId:'{}'", fixture.attempt.id)));
        assert!(shell_body.contains("event.source!==frame.contentWindow"));
        assert!(shell_body.contains("event.origin!=='null'"));
        assert!(!shell_body.contains("addEventListener('load'"));
        assert!(!shell_body.contains("allow-same-origin"));
        for secret in [
            "institution-imathas",
            "recorded-proxy-session",
            "jwt",
            "source_sha",
            "score",
            "answer",
        ] {
            assert!(
                !shell_body.to_ascii_lowercase().contains(secret),
                "shell leaked {secret}"
            );
            assert!(
                !set_cookie.to_ascii_lowercase().contains(secret),
                "cookie leaked {secret}"
            );
        }
        let launch_cookie = set_cookie.split(';').next().expect("cookie pair");
        let activity_path = format!("{path}/activity");
        let replica = external_tool_router(
            Arc::clone(&fixture.store),
            Arc::clone(&fixture.backend),
            Arc::new(
                crate::imathas_backend::LaunchStateAead::from_server_secret([84; 32])
                    .expect("replica aead"),
            ),
        );
        for request in [
            Request::builder()
                .uri(&activity_path)
                .header(
                    "cookie",
                    format!("{}; {launch_cookie}", fixture.student_cookie),
                )
                .body(Body::empty())
                .expect("GET"),
            Request::builder()
                .method("POST")
                .uri(&activity_path)
                .header(
                    "cookie",
                    format!("{}; {launch_cookie}", fixture.student_cookie),
                )
                .body(Body::from("answer=kept-local"))
                .expect("POST"),
        ] {
            let response = replica
                .clone()
                .oneshot(request)
                .await
                .expect("activity response");
            assert_eq!(response.status(), StatusCode::OK);
            assert_eq!(response.headers()["cache-control"], "no-store");
            assert_eq!(
                response.headers()["content-type"],
                "text/html; charset=utf-8"
            );
            let activity_csp = response.headers()["content-security-policy"]
                .to_str()
                .expect("activity CSP");
            assert!(activity_csp.starts_with("default-src 'none'; script-src 'nonce-"));
            assert!(!activity_csp.contains("unsafe-inline"));
            let body = to_bytes(response.into_body(), 256 * 1024)
                .await
                .expect("activity body");
            let body = std::str::from_utf8(&body).expect("activity UTF-8");
            assert!(body.starts_with("<!doctype html><title>Recorded protected activity</title>"));
            assert!(body.contains("kind:'ple.externalTool.activityReady'"));
            assert!(body.contains(&format!("attemptId:'{}'", fixture.attempt.id)));
        }
        for (cookie, target) in [
            (
                format!("{}; ple_external_launch=bad", fixture.student_cookie),
                activity_path.clone(),
            ),
            (
                format!("{}; {launch_cookie}", fixture.outsider_cookie),
                activity_path.clone(),
            ),
            (
                format!("{}; {launch_cookie}", fixture.student_cookie),
                format!(
                    "/api/attempts/{}/external-tool/launch/activity",
                    QuestionAttemptId::from_uuid(id(899))
                ),
            ),
        ] {
            let response = fixture
                .app
                .clone()
                .oneshot(
                    Request::builder()
                        .uri(target)
                        .header("cookie", cookie)
                        .body(Body::empty())
                        .expect("copied cookie request"),
                )
                .await
                .expect("copied cookie response");
            assert_eq!(response.status(), StatusCode::NOT_FOUND);
            let body = to_bytes(response.into_body(), 256 * 1024)
                .await
                .expect("copied error body");
            assert!(!String::from_utf8_lossy(&body).contains("activityReady"));
        }
        fixture
            .store
            .set_authoritative_time(ActivityTimestamp::from_unix_millis(40_001))
            .expect("advance clock");
        let expired = fixture
            .app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(&activity_path)
                    .header(
                        "cookie",
                        format!("{}; {launch_cookie}", fixture.student_cookie),
                    )
                    .body(Body::empty())
                    .expect("expired request"),
            )
            .await
            .expect("expired response");
        assert_eq!(expired.status(), StatusCode::NOT_FOUND);
        let expired_body = to_bytes(expired.into_body(), 256 * 1024)
            .await
            .expect("expired body");
        assert!(!String::from_utf8_lossy(&expired_body).contains("activityReady"));

        let created = fixture
            .backend
            .create_external_tool_launch(
                fixture.context,
                UserId::from_uuid(id(803)),
                ProblemVersionRef {
                    problem: fixture.attempt.problem,
                    version: fixture.attempt.question_version,
                },
                &fixture.question,
                &fixture.attempt,
                fixture.aead.as_ref(),
            )
            .await
            .expect("fresh launch session");
        let revoked_cookie = crate::imathas_backend::launch_cookie_value(
            fixture.aead.as_ref(),
            fixture.context,
            UserId::from_uuid(id(803)),
            fixture.attempt.id,
            &created,
        )
        .expect("launch cookie");
        learning_data_access::ExternalToolLaunchSessionStore::revoke_external_tool_launch_session(
            fixture.store.as_ref(),
            fixture.context,
            UserId::from_uuid(id(803)),
            fixture.attempt.id,
            created.id,
        )
        .await
        .expect("revoke launch session");
        let revoked = fixture
            .app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(&activity_path)
                    .header(
                        "cookie",
                        format!(
                            "{}; ple_external_launch={revoked_cookie}",
                            fixture.student_cookie
                        ),
                    )
                    .body(Body::empty())
                    .expect("revoked request"),
            )
            .await
            .expect("revoked response");
        assert_eq!(revoked.status(), StatusCode::NOT_FOUND);
        let revoked_body = to_bytes(revoked.into_body(), 256 * 1024)
            .await
            .expect("revoked body");
        assert!(!String::from_utf8_lossy(&revoked_body).contains("activityReady"));

        let mutated = contracted_route_fixture(RecordedContractedTransportMode::Available).await;
        let mutated_path = format!("/api/attempts/{}/external-tool/launch", mutated.attempt.id);
        let source_shell = mutated
            .app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(&mutated_path)
                    .header("cookie", &mutated.student_cookie)
                    .body(Body::empty())
                    .expect("source shell request"),
            )
            .await
            .expect("source shell response");
        assert_eq!(source_shell.status(), StatusCode::OK);
        let source_cookie = source_shell.headers()["set-cookie"]
            .to_str()
            .expect("source cookie")
            .split(';')
            .next()
            .expect("source cookie pair")
            .to_owned();
        objects::ObjectStore::delete(mutated.objects.as_ref(), &mutated.source_key)
            .await
            .expect("remove source for mutation gate");
        let source_mutated = mutated
            .app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("{mutated_path}/activity"))
                    .header(
                        "cookie",
                        format!("{}; {source_cookie}", mutated.student_cookie),
                    )
                    .body(Body::empty())
                    .expect("mutated source activity"),
            )
            .await
            .expect("mutated source response");
        // A missing immutable object is a local backend outage, but the
        // route refuses before restoring/proxying provider state.
        assert_eq!(source_mutated.status(), StatusCode::SERVICE_UNAVAILABLE);
        let source_mutated_body = to_bytes(source_mutated.into_body(), 256 * 1024)
            .await
            .expect("source mutation body");
        assert!(!String::from_utf8_lossy(&source_mutated_body).contains("activityReady"));
    }

    #[tokio::test]
    async fn contracted_imathas_launch_outage_is_question_local_and_secret_free() {
        use adapter_imathas::test_support::RecordedContractedTransportMode;
        let fixture = contracted_route_fixture(RecordedContractedTransportMode::Unavailable).await;
        let response = fixture
            .app
            .oneshot(
                Request::builder()
                    .uri(format!(
                        "/api/attempts/{}/external-tool/launch",
                        fixture.attempt.id
                    ))
                    .header("cookie", fixture.student_cookie)
                    .body(Body::empty())
                    .expect("outage request"),
            )
            .await
            .expect("outage response");
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
        let body = to_bytes(response.into_body(), 256 * 1024)
            .await
            .expect("outage body");
        let body = std::str::from_utf8(&body).expect("outage utf8");
        assert!(!body.contains("activityReady"));
        for secret in [
            "institution-imathas",
            "recorded-proxy-session",
            "jwt",
            "source",
            "score",
            "answer",
        ] {
            assert!(
                !body.to_ascii_lowercase().contains(secret),
                "outage leaked {secret}"
            );
        }
    }

    async fn json(response: Response) -> serde_json::Value {
        let bytes = to_bytes(response.into_body(), 256 * 1_024)
            .await
            .expect("response bytes");
        serde_json::from_slice(&bytes).expect("JSON response")
    }

    fn post_json(path: &str, cookie: &str, body: serde_json::Value) -> Request<Body> {
        Request::builder()
            .method("POST")
            .uri(path)
            .header("cookie", cookie)
            .header("content-type", "application/json")
            .body(Body::from(body.to_string()))
            .expect("request")
    }

    async fn active_attempt_for(
        app: &Router,
        assignment: AssignmentId,
        cookie: &str,
    ) -> QuestionAttempt {
        let run_response = app
            .clone()
            .oneshot(post_json(
                "/api/runs",
                cookie,
                serde_json::json!({ "assignmentId": assignment }),
            ))
            .await
            .expect("start run response");
        assert_eq!(run_response.status(), StatusCode::CREATED);
        let run: AssignmentRun =
            serde_json::from_value(json(run_response).await).expect("run contract");
        let attempts_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/api/runs/{}/attempts", run.id))
                    .header("cookie", cookie)
                    .body(Body::empty())
                    .expect("attempt request"),
            )
            .await
            .expect("attempt response");
        assert_eq!(attempts_response.status(), StatusCode::OK);
        let attempts: Page<QuestionAttempt> =
            serde_json::from_value(json(attempts_response).await).expect("attempt page");
        attempts.items.into_iter().next().expect("active attempt")
    }

    #[tokio::test]
    async fn archive_fence_refuses_run_aliases_before_any_backend_call() {
        let (store, backend, app, student_cookie, _, assignment, enrollment) = fixture().await;
        let active = active_attempt_for(&app, assignment, &student_cookie).await;
        let issued_before = backend.issued_seeds.lock().expect("seed record").len();
        assert_eq!(issued_before, 1);
        assert_eq!(backend.reproduce_calls.load(Ordering::SeqCst), 0);
        assert_eq!(backend.grade_calls.load(Ordering::SeqCst), 0);
        assert_eq!(backend.external_launch_calls.load(Ordering::SeqCst), 0);

        prepare_archive_fence(
            store.as_ref(),
            TenantId::from_uuid(id(1)),
            CourseId::from_uuid(id(5)),
        )
        .await;

        let requests = vec![
            Request::builder()
                .method("POST")
                .uri("/api/runs")
                .header("cookie", &student_cookie)
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({ "assignmentId": assignment }).to_string(),
                ))
                .expect("archived start request"),
            Request::builder()
                .uri(format!("/api/runs/{}", active.run))
                .header("cookie", &student_cookie)
                .body(Body::empty())
                .expect("archived run request"),
            Request::builder()
                .uri(format!("/api/runs/{}/summary", active.run))
                .header("cookie", &student_cookie)
                .body(Body::empty())
                .expect("archived summary request"),
            Request::builder()
                .uri(format!("/api/runs/{}/attempts", active.run))
                .header("cookie", &student_cookie)
                .body(Body::empty())
                .expect("archived attempt list request"),
            Request::builder()
                .uri(format!("/api/attempts/{}", active.id))
                .header("cookie", &student_cookie)
                .body(Body::empty())
                .expect("archived attempt request"),
            Request::builder()
                .uri(format!("/api/attempts/{}/question", active.id))
                .header("cookie", &student_cookie)
                .body(Body::empty())
                .expect("archived question request"),
            Request::builder()
                .method("POST")
                .uri(format!("/api/attempts/{}/prefetch-next", active.id))
                .header("cookie", &student_cookie)
                .body(Body::empty())
                .expect("archived prefetch request"),
            Request::builder()
                .uri(format!("/api/attempts/{}/external-tool-launch", active.id))
                .header("cookie", &student_cookie)
                .body(Body::empty())
                .expect("archived external projection request"),
            Request::builder()
                .method("POST")
                .uri(format!("/api/submissions/{}", active.id))
                .header("cookie", &student_cookie)
                .header("content-type", "application/json")
                .header("idempotency-key", "archive-refusal")
                .body(Body::from(
                    serde_json::json!({
                        "response": { "kind": "numeric", "value": 18.0 }
                    })
                    .to_string(),
                ))
                .expect("archived submission request"),
            Request::builder()
                .method("POST")
                .uri(format!("/api/attempts/{}/feedback-release", active.id))
                .header("cookie", &student_cookie)
                .body(Body::empty())
                .expect("archived feedback release request"),
            Request::builder()
                .uri(format!("/api/grading/summaries/{enrollment}"))
                .header("cookie", &student_cookie)
                .body(Body::empty())
                .expect("archived grading summary request"),
            Request::builder()
                .uri(format!("/api/enrollments/{enrollment}"))
                .header("cookie", &student_cookie)
                .body(Body::empty())
                .expect("archived enrollment request"),
            Request::builder()
                .uri(format!("/api/enrollments/{enrollment}/runs"))
                .header("cookie", &student_cookie)
                .body(Body::empty())
                .expect("archived enrollment runs request"),
        ];
        for request in requests {
            let response = app
                .clone()
                .oneshot(request)
                .await
                .expect("archived alias response");
            assert_eq!(response.status(), StatusCode::NOT_FOUND);
            assert_eq!(
                response.headers().get("cache-control"),
                Some(&HeaderValue::from_static("no-store"))
            );
        }

        assert_eq!(
            backend.issued_seeds.lock().expect("seed record").len(),
            issued_before
        );
        assert_eq!(backend.reproduce_calls.load(Ordering::SeqCst), 0);
        assert_eq!(backend.grade_calls.load(Ordering::SeqCst), 0);
        assert_eq!(backend.external_launch_calls.load(Ordering::SeqCst), 0);
    }

    async fn next_active_attempt(app: &Router, run: RunId, cookie: &str) -> QuestionAttempt {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/api/runs/{run}/attempts"))
                    .header("cookie", cookie)
                    .body(Body::empty())
                    .expect("attempt list request"),
            )
            .await
            .expect("attempt list response");
        let attempts: Page<QuestionAttempt> =
            serde_json::from_value(json(response).await).expect("attempt page");
        attempts
            .items
            .into_iter()
            .find(|attempt| attempt.response.is_none())
            .expect("next active attempt")
    }

    #[tokio::test]
    async fn native_feedback_http_policy_matrix_is_allowlisted_and_replay_safe() {
        for policy in [
            FeedbackDisclosure::ImmediateCorrectness,
            FeedbackDisclosure::ImmediateFull,
            FeedbackDisclosure::Deferred,
            FeedbackDisclosure::OnRelease,
        ] {
            let (store, backend, app, student_cookie, _outsider_cookie, assignment) =
                native_feedback_fixture(policy).await;
            let first = active_attempt_for(&app, assignment, &student_cookie).await;
            let submit = |attempt: QuestionAttemptId, key: &str, choice: &str| {
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/submissions/{attempt}"))
                    .header("cookie", &student_cookie)
                    .header("content-type", "application/json")
                    .header("idempotency-key", key)
                    .body(Body::from(
                        serde_json::json!({
                            "response": { "kind": "multipleChoice", "selected": [choice] }
                        })
                        .to_string(),
                    ))
                    .expect("native submission")
            };
            let first_response = app
                .clone()
                .oneshot(submit(first.id, "native-feedback-first", "ester"))
                .await
                .expect("first submission");
            assert_eq!(first_response.status(), StatusCode::OK);
            let first_receipt = json(first_response).await;
            let first_raw = first_receipt.to_string();
            for forbidden in [
                "answerKey",
                "expected",
                "checker",
                "provider",
                "solution",
                "feedbackContent",
            ] {
                assert!(
                    !first_raw.contains(forbidden),
                    "{policy:?} receipt leaked {forbidden}"
                );
            }
            assert_eq!(backend.submissions.load(Ordering::SeqCst), 1);
            match policy {
                FeedbackDisclosure::ImmediateCorrectness => {
                    assert_eq!(first_receipt["attempt"]["result"], serde_json::Value::Null);
                    assert_eq!(first_receipt["feedback"]["correctness"], false);
                    assert!(first_receipt["feedback"].get("hint").is_some());
                    for prohibited in [
                        "pointsEarned",
                        "pointsPossible",
                        "correctResponse",
                        "rationale",
                    ] {
                        assert!(first_receipt["feedback"].get(prohibited).is_none());
                    }
                }
                FeedbackDisclosure::ImmediateFull => {
                    assert_eq!(first_receipt["feedback"]["correctness"], false);
                    assert_eq!(first_receipt["feedback"]["pointsEarned"], 0.0);
                    assert_eq!(first_receipt["feedback"]["pointsPossible"], 2.0);
                    assert!(first_receipt["feedback"]["hint"].is_array());
                    assert_eq!(
                        first_receipt["feedback"]["correctResponse"][0]["markdown"],
                        "The peptide linkage"
                    );
                    assert!(
                        first_receipt["feedback"]["rationale"][0]["markdown"]
                            .as_str()
                            .is_some_and(
                                |text| text.contains("resonance") && text.contains("planar")
                            )
                    );
                }
                FeedbackDisclosure::Deferred | FeedbackDisclosure::OnRelease => {
                    assert_eq!(first_receipt["feedback"], serde_json::Value::Null);
                    assert_eq!(first_receipt["attempt"]["result"], serde_json::Value::Null);
                }
            }
            if !matches!(policy, FeedbackDisclosure::Deferred) {
                let replay = app
                    .clone()
                    .oneshot(submit(first.id, "native-feedback-first", "ester"))
                    .await
                    .expect("idempotent replay");
                assert_eq!(replay.status(), StatusCode::OK);
                assert_eq!(json(replay).await, first_receipt);
                assert_eq!(backend.submissions.load(Ordering::SeqCst), 1);
            }

            let foreign_cookie = issued_cookie_for(
                store.as_ref(),
                TenantId::from_uuid(id(299)),
                UserId::from_uuid(id(298)),
                "Foreign",
            )
            .await;
            let foreign = app
                .clone()
                .oneshot(
                    Request::builder()
                        .method("POST")
                        .uri(format!("/api/submissions/{}", first.id))
                        .header("cookie", foreign_cookie)
                        .header("content-type", "application/json")
                        .header("idempotency-key", "foreign-feedback")
                        .body(Body::from(
                            serde_json::json!({
                                "response": { "kind": "multipleChoice", "selected": ["ester"] }
                            })
                            .to_string(),
                        ))
                        .expect("foreign submission"),
                )
                .await
                .expect("foreign response");
            assert_eq!(foreign.status(), StatusCode::NOT_FOUND);

            let next = next_active_attempt(&app, first.run, &student_cookie).await;
            let completion = app
                .clone()
                .oneshot(submit(next.id, "native-feedback-complete", "amide"))
                .await
                .expect("completion submission");
            assert_eq!(completion.status(), StatusCode::OK);
            assert_eq!(backend.submissions.load(Ordering::SeqCst), 2);

            if matches!(policy, FeedbackDisclosure::Deferred) {
                let stored = store
                    .replay_submission(
                        TenantContext::from_authenticated_session(TenantId::from_uuid(id(201))),
                        UserId::from_uuid(id(203)),
                        first.id,
                        &StudentResponse::MultipleChoice {
                            selected: vec![ChoiceId::new("ester")],
                        },
                        &SubmissionIdempotencyKey::parse("native-feedback-first")
                            .expect("valid replay key"),
                    )
                    .await
                    .expect("direct stored replay")
                    .expect("first receipt");
                assert!(stored.run.completed_at.is_none());
            }

            let replay = app
                .clone()
                .oneshot(submit(first.id, "native-feedback-first", "ester"))
                .await
                .expect("post-completion replay");
            assert_eq!(replay.status(), StatusCode::OK);
            let replay_receipt = json(replay).await;
            if matches!(policy, FeedbackDisclosure::OnRelease) {
                assert_eq!(replay_receipt["feedback"], serde_json::Value::Null);
            } else {
                assert_eq!(replay_receipt, first_receipt);
            }
            assert_eq!(backend.submissions.load(Ordering::SeqCst), 2);
        }
    }

    #[tokio::test]
    async fn prefetch_is_body_free_idempotent_and_binds_the_submission_replay() {
        let (_store, _backend, app, student_cookie, outsider_cookie, assignment) =
            native_feedback_fixture(FeedbackDisclosure::ImmediateCorrectness).await;
        let first = active_attempt_for(&app, assignment, &student_cookie).await;
        let prefetch = || {
            Request::builder()
                .method("POST")
                .uri(format!("/api/attempts/{}/prefetch-next", first.id))
                .header("cookie", &student_cookie)
                .body(Body::empty())
                .expect("body-free prefetch request")
        };
        let (first_prefetch, concurrent_prefetch) = tokio::join!(
            app.clone().oneshot(prefetch()),
            app.clone().oneshot(prefetch()),
        );
        let cached = first_prefetch.expect("first concurrent prefetch response");
        let concurrent = concurrent_prefetch.expect("second concurrent prefetch response");
        assert_eq!(cached.status(), StatusCode::OK);
        assert_eq!(concurrent.status(), StatusCode::OK);
        assert_eq!(
            cached.headers().get("cache-control"),
            Some(&HeaderValue::from_static("no-store"))
        );
        let cached = json(cached).await;
        assert_eq!(json(concurrent).await, cached);
        assert_eq!(cached["predecessor"], serde_json::json!(first.id));
        assert_eq!(cached["run"], serde_json::json!(first.run));
        let cached_json = cached.to_string();
        for forbidden in ["answer", "key", "provider", "provenance"] {
            assert!(
                !cached_json.contains(forbidden),
                "prefetch projection must not disclose {forbidden}"
            );
        }
        let repeated = json(
            app.clone()
                .oneshot(prefetch())
                .await
                .expect("repeat prefetch"),
        )
        .await;
        assert_eq!(
            repeated, cached,
            "a retry reproduces the same reserved variation"
        );

        let hostile = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/attempts/{}/prefetch-next", first.id))
                    .header("cookie", &student_cookie)
                    .header("content-type", "application/json")
                    .body(Body::from("{}"))
                    .expect("hostile prefetch"),
            )
            .await
            .expect("hostile response");
        assert_eq!(hostile.status(), StatusCode::BAD_REQUEST);
        let unauthenticated_hostile = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/attempts/{}/prefetch-next", first.id))
                    .header("content-type", "application/json")
                    .body(Body::from("{}"))
                    .expect("unauthenticated hostile prefetch"),
            )
            .await
            .expect("unauthenticated response");
        assert_eq!(
            unauthenticated_hostile.status(),
            StatusCode::UNAUTHORIZED,
            "authentication occurs before body-shape validation",
        );
        let foreign = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/attempts/{}/prefetch-next", first.id))
                    .header("cookie", &outsider_cookie)
                    .body(Body::empty())
                    .expect("foreign prefetch"),
            )
            .await
            .expect("foreign prefetch response");
        assert_eq!(
            foreign.status(),
            StatusCode::NOT_FOUND,
            "a foreign learner cannot enumerate an owned active attempt"
        );

        let submit = |attempt: QuestionAttemptId, key: &str, choice: &str| {
            Request::builder()
                .method("POST")
                .uri(format!("/api/submissions/{attempt}"))
                .header("cookie", &student_cookie)
                .header("content-type", "application/json")
                .header("idempotency-key", key)
                .body(Body::from(
                    serde_json::json!({"response":{"kind":"multipleChoice","selected":[choice]}})
                        .to_string(),
                ))
                .expect("submission")
        };
        let first_response = app
            .clone()
            .oneshot(submit(first.id, "prefetch-first", "ester"))
            .await
            .expect("first submit");
        assert_eq!(first_response.status(), StatusCode::OK);
        let first_receipt = json(first_response).await;
        assert_eq!(first_receipt["nextIssued"]["run"], cached["run"]);
        let next = next_active_attempt(&app, first.run, &student_cookie).await;
        assert_eq!(
            first_receipt["nextIssued"]["id"],
            serde_json::json!(next.id)
        );
        assert_eq!(
            cached["envelope"]["version"],
            serde_json::json!(next.question_version)
        );
        assert_eq!(cached["envelope"]["seed"], serde_json::json!(next.seed));
        let final_position_prefetch = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/attempts/{}/prefetch-next", next.id))
                    .header("cookie", &student_cookie)
                    .body(Body::empty())
                    .expect("final-position prefetch"),
            )
            .await
            .expect("final-position prefetch response");
        assert_eq!(final_position_prefetch.status(), StatusCode::NO_CONTENT);
        assert_eq!(
            final_position_prefetch.headers().get("cache-control"),
            Some(&HeaderValue::from_static("no-store")),
            "no-successor prefetches are not cacheable"
        );
        let completed = app
            .clone()
            .oneshot(submit(next.id, "prefetch-second", "amide"))
            .await
            .expect("next submit");
        assert_eq!(completed.status(), StatusCode::OK);
        let replay = app
            .clone()
            .oneshot(submit(first.id, "prefetch-first", "ester"))
            .await
            .expect("first replay");
        assert_eq!(replay.status(), StatusCode::OK);
        assert_eq!(
            json(replay).await,
            first_receipt,
            "later completion cannot rewrite the earlier nextIssued receipt"
        );
    }

    #[tokio::test]
    async fn prefetch_preserves_a_backend_owned_render_hash() {
        let (store, backend, _app, student_cookie, _outsider_cookie, assignment) =
            native_feedback_fixture(FeedbackDisclosure::ImmediateCorrectness).await;
        let app = router(
            store,
            Arc::new(OpaqueRenderedHashBackend { inner: backend }),
        );
        let first = active_attempt_for(&app, assignment, &student_cookie).await;
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/attempts/{}/prefetch-next", first.id))
                    .header("cookie", &student_cookie)
                    .body(Body::empty())
                    .expect("body-free prefetch request"),
            )
            .await
            .expect("prefetch response");
        assert_eq!(response.status(), StatusCode::OK);
        let body = json(response).await;
        assert!(
            body["renderedQuestionSha256"]
                .as_str()
                .is_some_and(|value| value.starts_with("backend-owned-render-")),
            "the route preserves the trusted backend's canonical rendered-artifact hash",
        );
    }

    #[tokio::test]
    async fn resumed_run_never_issues_an_unlinked_successor_before_submission_replay_heals() {
        let (store, _backend, app, student_cookie, _outsider_cookie, assignment) =
            native_feedback_fixture(FeedbackDisclosure::ImmediateCorrectness).await;
        let first = active_attempt_for(&app, assignment, &student_cookie).await;
        let response = StudentResponse::MultipleChoice {
            selected: vec![ChoiceId::new("ester")],
        };
        let key =
            SubmissionIdempotencyKey::parse("crash-before-successor-link").expect("valid key");
        store
            .submit_question_attempt(
                TenantContext::from_authenticated_session(TenantId::from_uuid(id(201))),
                SubmitQuestionAttemptCommand {
                    actor: UserId::from_uuid(id(203)),
                    attempt: first.id,
                    response,
                    result: AttemptResult {
                        correct: false,
                        points_earned: 0.0,
                        points_possible: 2.0,
                    },
                    feedback: FeedbackContent::default(),
                    idempotency_key: key,
                },
            )
            .await
            .expect("simulate durable grade commit before process crash");
        let resumed = app
            .clone()
            .oneshot(post_json(
                "/api/runs",
                &student_cookie,
                serde_json::json!({ "assignmentId": assignment }),
            ))
            .await
            .expect("resume response");
        assert_eq!(resumed.status(), StatusCode::CREATED);
        let after_resume = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/api/runs/{}/attempts", first.run))
                    .header("cookie", &student_cookie)
                    .body(Body::empty())
                    .expect("attempt page"),
            )
            .await
            .expect("attempts after resume");
        let attempts: Page<QuestionAttempt> =
            serde_json::from_value(json(after_resume).await).expect("attempt page");
        assert_eq!(
            attempts.items.len(),
            2,
            "resume heals only through the durable pending predecessor link",
        );
        let replay = app.clone().oneshot(
            Request::builder().method("POST").uri(format!("/api/submissions/{}", first.id))
                .header("cookie", &student_cookie).header("content-type", "application/json")
                .header("idempotency-key", "crash-before-successor-link")
                .body(Body::from(serde_json::json!({"response":{"kind":"multipleChoice","selected":["ester"]}}).to_string())).expect("replay"),
        ).await.expect("replay response");
        assert_eq!(replay.status(), StatusCode::OK);
        let receipt = json(replay).await;
        assert!(
            receipt["nextIssued"].is_object(),
            "replay heals the exact successor link"
        );
        let next = next_active_attempt(&app, first.run, &student_cookie).await;
        assert_eq!(receipt["nextIssued"]["id"], serde_json::json!(next.id));
    }

    #[tokio::test]
    async fn run_summary_projects_current_disclosure_and_release_without_rewriting_receipts() {
        for policy in [
            FeedbackDisclosure::ImmediateCorrectness,
            FeedbackDisclosure::ImmediateFull,
            FeedbackDisclosure::Deferred,
            FeedbackDisclosure::OnRelease,
        ] {
            let (store, _backend, app, student_cookie, outsider_cookie, assignment) =
                native_feedback_fixture(policy).await;
            let instructor_cookie = issued_cookie_for(
                store.as_ref(),
                TenantId::from_uuid(id(201)),
                UserId::from_uuid(id(202)),
                "Instructor",
            )
            .await;
            let first = active_attempt_for(&app, assignment, &student_cookie).await;
            let submit = |attempt: QuestionAttemptId, key: &str, choice: &str| {
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/submissions/{attempt}"))
                    .header("cookie", &student_cookie)
                    .header("content-type", "application/json")
                    .header("idempotency-key", key)
                    .body(Body::from(
                        serde_json::json!({
                            "response": { "kind": "multipleChoice", "selected": [choice] }
                        })
                        .to_string(),
                    ))
                    .expect("submission request")
            };
            let first_receipt = json(
                app.clone()
                    .oneshot(submit(first.id, "summary-first", "ester"))
                    .await
                    .expect("first submission"),
            )
            .await;

            let summary_path = format!("/api/runs/{}/summary?pageSize=1", first.run);
            let before = app
                .clone()
                .oneshot(
                    Request::builder()
                        .uri(&summary_path)
                        .header("cookie", &student_cookie)
                        .body(Body::empty())
                        .expect("summary request"),
                )
                .await
                .expect("summary response");
            assert_eq!(before.status(), StatusCode::OK);
            assert_eq!(before.headers()["cache-control"], "no-store");
            let before = json(before).await;
            let instructor_summary = app
                .clone()
                .oneshot(
                    Request::builder()
                        .uri(&summary_path)
                        .header("cookie", &instructor_cookie)
                        .body(Body::empty())
                        .expect("instructor summary request"),
                )
                .await
                .expect("instructor summary response");
            assert_eq!(instructor_summary.status(), StatusCode::OK);
            assert_eq!(instructor_summary.headers()["cache-control"], "no-store");
            assert_eq!(
                json(instructor_summary).await["run"]["id"],
                first.run.to_string()
            );
            assert_eq!(
                before["outcomes"]["items"].as_array().map(Vec::len),
                Some(1)
            );
            let feedback_before = &before["outcomes"]["items"][0]["feedback"];
            match policy {
                FeedbackDisclosure::ImmediateCorrectness | FeedbackDisclosure::ImmediateFull => {
                    assert!(feedback_before.is_object());
                }
                FeedbackDisclosure::Deferred | FeedbackDisclosure::OnRelease => {
                    assert_eq!(feedback_before, &serde_json::Value::Null);
                }
            }
            let raw_before = before.to_string();
            for forbidden in [
                "answerKey",
                "checker",
                "provider",
                "provenance",
                "source",
                "launchUrl",
                "feedbackContent",
            ] {
                assert!(
                    !raw_before.contains(forbidden),
                    "run summary leaked {forbidden}"
                );
            }
            let cursor = before["outcomes"]["nextCursor"]
                .as_str()
                .expect("bounded page cursor");
            let continuation = app
                .clone()
                .oneshot(
                    Request::builder()
                        .uri(format!(
                            "/api/runs/{}/summary?pageSize=1&cursor={cursor}",
                            first.run
                        ))
                        .header("cookie", &student_cookie)
                        .body(Body::empty())
                        .expect("summary continuation"),
                )
                .await
                .expect("summary continuation response");
            assert_eq!(continuation.status(), StatusCode::OK);

            let student_release = app
                .clone()
                .oneshot(
                    Request::builder()
                        .method("POST")
                        .uri(format!("/api/attempts/{}/feedback-release", first.id))
                        .header("cookie", &student_cookie)
                        .body(Body::empty())
                        .expect("student release request"),
                )
                .await
                .expect("student release response");
            assert_eq!(student_release.status(), StatusCode::NOT_FOUND);
            let foreign_summary = app
                .clone()
                .oneshot(
                    Request::builder()
                        .uri(&summary_path)
                        .header("cookie", &outsider_cookie)
                        .body(Body::empty())
                        .expect("outsider summary request"),
                )
                .await
                .expect("outsider summary response");
            assert_eq!(foreign_summary.status(), StatusCode::NOT_FOUND);
            let foreign_tenant_cookie = issued_cookie_for(
                store.as_ref(),
                TenantId::from_uuid(id(299)),
                UserId::from_uuid(id(298)),
                "Foreign tenant",
            )
            .await;
            let foreign_tenant_summary = app
                .clone()
                .oneshot(
                    Request::builder()
                        .uri(&summary_path)
                        .header("cookie", &foreign_tenant_cookie)
                        .body(Body::empty())
                        .expect("foreign-tenant summary request"),
                )
                .await
                .expect("foreign-tenant summary response");
            assert_eq!(foreign_tenant_summary.status(), StatusCode::NOT_FOUND);

            let next = next_active_attempt(&app, first.run, &student_cookie).await;
            let completed = app
                .clone()
                .oneshot(submit(next.id, "summary-complete", "amide"))
                .await
                .expect("completion submission");
            assert_eq!(completed.status(), StatusCode::OK);

            if matches!(policy, FeedbackDisclosure::OnRelease) {
                let release = app
                    .clone()
                    .oneshot(
                        Request::builder()
                            .method("POST")
                            .uri(format!("/api/attempts/{}/feedback-release", first.id))
                            .header("cookie", &instructor_cookie)
                            .body(Body::empty())
                            .expect("instructor release request"),
                    )
                    .await
                    .expect("instructor release response");
                assert_eq!(release.status(), StatusCode::OK);
                assert_eq!(release.headers()["cache-control"], "no-store");
                assert_eq!(json(release).await, serde_json::json!({ "released": true }));
            }

            let after = app
                .clone()
                .oneshot(
                    Request::builder()
                        .uri(format!("/api/runs/{}/summary", first.run))
                        .header("cookie", &student_cookie)
                        .body(Body::empty())
                        .expect("completed summary request"),
                )
                .await
                .expect("completed summary response");
            assert_eq!(after.status(), StatusCode::OK);
            let after = json(after).await;
            assert_eq!(after["practiceAllowed"], true);
            let first_feedback = &after["outcomes"]["items"][0]["feedback"];
            match policy {
                FeedbackDisclosure::OnRelease => {
                    assert!(first_feedback.get("correctResponse").is_some());
                    let replay = json(
                        app.clone()
                            .oneshot(submit(first.id, "summary-first", "ester"))
                            .await
                            .expect("receipt replay"),
                    )
                    .await;
                    assert_eq!(
                        replay, first_receipt,
                        "release must not rewrite the receipt"
                    );
                }
                FeedbackDisclosure::Deferred => {
                    assert!(first_feedback.get("correctResponse").is_some())
                }
                FeedbackDisclosure::ImmediateCorrectness => {
                    assert!(first_feedback.get("correctResponse").is_none())
                }
                FeedbackDisclosure::ImmediateFull => {
                    assert!(first_feedback.get("correctResponse").is_some())
                }
            }
        }
    }

    #[tokio::test]
    async fn external_tool_launch_projection_is_owner_only_and_key_free() {
        let (store, _backend, app, student_cookie, outsider_cookie, assignment, _enrollment) =
            fixture_with_response(ResponseDefinition::ExternalTool {}, true).await;
        let attempt = active_attempt_for(&app, assignment, &student_cookie).await;
        let projection_path = format!("/api/attempts/{}/external-tool-launch", attempt.id);

        let owner_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(&projection_path)
                    .header("cookie", &student_cookie)
                    .body(Body::empty())
                    .expect("owner projection request"),
            )
            .await
            .expect("owner projection response");
        assert_eq!(owner_response.status(), StatusCode::OK);
        assert_eq!(owner_response.headers()["cache-control"], "no-store");
        let projection = json(owner_response).await;
        assert_eq!(
            projection,
            serde_json::json!({
                "launchUrl": format!("/api/attempts/{}/external-tool/launch", attempt.id),
            })
        );
        let serialized = projection.to_string();
        for forbidden in [
            "provider",
            "itemRef",
            "snapshot",
            "answer",
            "solution",
            "token",
            "nonce",
            "credential",
            "score",
            "http://",
            "https://",
        ] {
            assert!(
                !serialized.contains(forbidden),
                "projection leaked {forbidden}"
            );
        }

        let outsider_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(&projection_path)
                    .header("cookie", &outsider_cookie)
                    .body(Body::empty())
                    .expect("outsider projection request"),
            )
            .await
            .expect("outsider projection response");
        assert_eq!(outsider_response.status(), StatusCode::NOT_FOUND);

        let anonymous_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(&projection_path)
                    .body(Body::empty())
                    .expect("anonymous projection request"),
            )
            .await
            .expect("anonymous projection response");
        assert_eq!(anonymous_response.status(), StatusCode::UNAUTHORIZED);

        let cross_tenant_subject = SessionSubject::new(
            TenantId::from_uuid(id(101)),
            UserId::from_uuid(id(102)),
            "Other tenant",
            vec![UserRole::Student],
        )
        .expect("cross-tenant subject");
        let cross_tenant = crate::auth::issue_session(
            store.as_ref(),
            cross_tenant_subject,
            crate::auth::SessionConfig::new(
                SessionLifetime::from_seconds(3_600).expect("session lifetime"),
                crate::auth::CookieTransport::LocalHttp,
            ),
        )
        .await
        .expect("cross-tenant session")
        .set_cookie
        .split(';')
        .next()
        .expect("cross-tenant cookie pair")
        .to_string();
        let cross_tenant_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(&projection_path)
                    .header("cookie", cross_tenant)
                    .body(Body::empty())
                    .expect("cross-tenant projection request"),
            )
            .await
            .expect("cross-tenant projection response");
        assert_eq!(cross_tenant_response.status(), StatusCode::NOT_FOUND);

        let copied_broker_path = format!("/api/attempts/{}/external-tool/launch", attempt.id);
        let copied_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(copied_broker_path)
                    .header("cookie", &student_cookie)
                    .body(Body::empty())
                    .expect("unimplemented broker request"),
            )
            .await
            .expect("unimplemented broker response");
        assert_eq!(copied_response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn external_tool_launch_refuses_non_external_and_unsupported_attempts() {
        let (_store, _backend, app, student_cookie, _outsider_cookie, assignment, _enrollment) =
            fixture().await;
        let numeric_attempt = active_attempt_for(&app, assignment, &student_cookie).await;
        let non_external = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!(
                        "/api/attempts/{}/external-tool-launch",
                        numeric_attempt.id
                    ))
                    .header("cookie", &student_cookie)
                    .body(Body::empty())
                    .expect("non-external request"),
            )
            .await
            .expect("non-external response");
        assert_eq!(non_external.status(), StatusCode::NOT_FOUND);

        let (_store, _backend, app, student_cookie, _outsider_cookie, assignment, _enrollment) =
            fixture_with_response(ResponseDefinition::ExternalTool {}, false).await;
        let unsupported_attempt = active_attempt_for(&app, assignment, &student_cookie).await;
        let unsupported = app
            .oneshot(
                Request::builder()
                    .uri(format!(
                        "/api/attempts/{}/external-tool-launch",
                        unsupported_attempt.id
                    ))
                    .header("cookie", student_cookie)
                    .body(Body::empty())
                    .expect("unsupported request"),
            )
            .await
            .expect("unsupported response");
        assert_eq!(unsupported.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[tokio::test]
    async fn file_upload_submission_refuses_untrusted_object_key_before_backend_or_store_mutation()
    {
        let (store, backend, app, student_cookie, _outsider_cookie, assignment, _enrollment) =
            fixture_with_response(
                ResponseDefinition::FileUpload {
                    max_bytes: 1_024,
                    accepted_extensions: vec!["pdf".to_string()],
                },
                false,
            )
            .await;
        let attempt = active_attempt_for(&app, assignment, &student_cookie).await;
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/submissions/{}", attempt.id))
                    .header("cookie", student_cookie)
                    .header("content-type", "application/json")
                    .header("idempotency-key", "forged-file-upload")
                    .body(Body::from(
                        serde_json::json!({
                            "response": {
                                "kind": "fileUpload",
                                "objectKey": "student-records/foreign-tenant/private.pdf",
                            }
                        })
                        .to_string(),
                    ))
                    .expect("forged file-upload request"),
            )
            .await
            .expect("forged file-upload response");

        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(
            json(response).await,
            serde_json::json!({ "error": "file upload submissions are unavailable" })
        );
        assert_eq!(backend.grade_calls.load(Ordering::SeqCst), 0);
        assert_eq!(
            store
                .get_question_attempt(
                    TenantContext::from_authenticated_session(TenantId::from_uuid(id(1))),
                    attempt.id,
                )
                .await
                .expect("attempt read"),
            Some(attempt)
        );
    }

    #[tokio::test]
    async fn runs_resume_submit_idempotently_and_keep_keys_server_only() {
        let (store, backend, app, student_cookie, outsider_cookie, assignment, enrollment) =
            fixture().await;
        let first_response = app
            .clone()
            .oneshot(post_json(
                "/api/runs",
                &student_cookie,
                serde_json::json!({ "assignmentId": assignment }),
            ))
            .await
            .expect("start response");
        assert_eq!(first_response.status(), StatusCode::CREATED);
        let first: AssignmentRun =
            serde_json::from_value(json(first_response).await).expect("run contract");
        assert_eq!(
            first.started_at,
            ActivityTimestamp::from_unix_millis(10_000)
        );

        let attempts_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/api/runs/{}/attempts", first.id))
                    .header("cookie", &student_cookie)
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("attempt response");
        let attempts: Page<QuestionAttempt> =
            serde_json::from_value(json(attempts_response).await).expect("attempt page");
        let issued = attempts.items.first().expect("issued attempt");
        assert_eq!(issued.timer.issued_at, first.started_at);
        assert!(issued.response.is_none());
        assert!(issued.result.is_none());

        let question_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/api/attempts/{}/question", issued.id))
                    .header("cookie", &student_cookie)
                    .body(Body::empty())
                    .expect("issued question request"),
            )
            .await
            .expect("issued question response");
        assert_eq!(question_response.status(), StatusCode::OK);
        let envelope = json(question_response).await;
        assert_eq!(
            envelope["version"],
            serde_json::json!(issued.question_version)
        );
        assert_eq!(envelope["seed"], serde_json::json!(issued.seed));
        assert_eq!(envelope["response"]["kind"], "numeric");
        let serialized_envelope = envelope.to_string();
        for answer_bearing_field in ["answerKey", "expected", "rubric", "grading"] {
            assert!(!serialized_envelope.contains(answer_bearing_field));
        }

        let submission_body = serde_json::json!({
            "response": { "kind": "numeric", "value": 18.0 }
        });
        let submit = |key: &str, body: serde_json::Value| {
            Request::builder()
                .method("POST")
                .uri(format!("/api/submissions/{}", issued.id))
                .header("cookie", &student_cookie)
                .header("content-type", "application/json")
                .header("idempotency-key", key)
                .body(Body::from(body.to_string()))
                .expect("request")
        };
        let malformed_submission = app
            .clone()
            .oneshot(submit(
                "malformed-request",
                serde_json::json!({
                    "response": { "kind": "shortText", "text": "eighteen" }
                }),
            ))
            .await
            .expect("malformed submission response");
        assert_eq!(
            malformed_submission.status(),
            StatusCode::UNPROCESSABLE_ENTITY
        );
        assert_eq!(backend.grade_calls.load(Ordering::SeqCst), 0);

        let first_submission = app
            .clone()
            .oneshot(submit("same-request", submission_body.clone()))
            .await
            .expect("submission response");
        assert_eq!(first_submission.status(), StatusCode::OK);
        let first_receipt = json(first_submission).await;
        assert_eq!(first_receipt["accepted"], true);
        assert_eq!(first_receipt["attempt"]["result"]["correct"], true);
        // The generic NumericBackend takes the default server grade path: it
        // may honestly disclose the grade, but it cannot fabricate native
        // teaching blocks it did not produce.
        assert_eq!(first_receipt["feedback"]["correctness"], true);
        assert_eq!(first_receipt["feedback"]["pointsEarned"], 1.0);
        assert!(first_receipt["feedback"].get("hint").is_none());
        assert!(first_receipt["feedback"].get("correctResponse").is_none());
        assert!(first_receipt["feedback"].get("rationale").is_none());
        let serialized_receipt = first_receipt.to_string();
        for answer_bearing_field in ["answerKey", "expected", "rubric", "feedbackContent"] {
            assert!(!serialized_receipt.contains(answer_bearing_field));
        }
        assert_eq!(backend.grade_calls.load(Ordering::SeqCst), 1);

        let replay = app
            .clone()
            .oneshot(submit("same-request", submission_body.clone()))
            .await
            .expect("replay response");
        assert_eq!(replay.status(), StatusCode::OK);
        assert_eq!(json(replay).await, first_receipt);
        assert_eq!(backend.grade_calls.load(Ordering::SeqCst), 1);

        let changed = app
            .clone()
            .oneshot(submit(
                "same-request",
                serde_json::json!({
                    "response": { "kind": "numeric", "value": 19.0 }
                }),
            ))
            .await
            .expect("changed replay response");
        assert_eq!(changed.status(), StatusCode::CONFLICT);
        assert_eq!(backend.grade_calls.load(Ordering::SeqCst), 1);

        let outsider = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/api/runs/{}", first.id))
                    .header("cookie", &outsider_cookie)
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("outsider response");
        assert_eq!(outsider.status(), StatusCode::NOT_FOUND);

        let outsider_question = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/api/attempts/{}/question", issued.id))
                    .header("cookie", &outsider_cookie)
                    .body(Body::empty())
                    .expect("outsider question request"),
            )
            .await
            .expect("outsider question response");
        assert_eq!(outsider_question.status(), StatusCode::NOT_FOUND);

        store
            .set_authoritative_time(ActivityTimestamp::from_unix_millis(20_000))
            .expect("advance clock");
        let practice_response = app
            .clone()
            .oneshot(post_json(
                "/api/runs",
                &student_cookie,
                serde_json::json!({ "assignmentId": assignment }),
            ))
            .await
            .expect("practice response");
        let practice: AssignmentRun =
            serde_json::from_value(json(practice_response).await).expect("practice run");
        assert_eq!(practice.run_number, 2);
        assert_eq!(
            practice.started_at,
            ActivityTimestamp::from_unix_millis(20_000)
        );
        let seeds = backend.issued_seeds.lock().expect("seed record").clone();
        assert_eq!(seeds.len(), 2);
        assert_ne!(seeds[0], seeds[1]);

        let first_history_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/api/enrollments/{enrollment}/runs?pageSize=1"))
                    .header("cookie", &student_cookie)
                    .body(Body::empty())
                    .expect("run history request"),
            )
            .await
            .expect("first run history response");
        let first_history: Page<AssignmentRun> =
            serde_json::from_value(json(first_history_response).await).expect("run history page");
        let cursor = first_history
            .next_cursor
            .expect("first run history page should continue");
        let second_history_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!(
                        "/api/enrollments/{enrollment}/runs?pageSize=1&cursor={}",
                        cursor.as_str()
                    ))
                    .header("cookie", &student_cookie)
                    .body(Body::empty())
                    .expect("continued run history request"),
            )
            .await
            .expect("continued run history response");
        let second_history: Page<AssignmentRun> =
            serde_json::from_value(json(second_history_response).await)
                .expect("continued run history page");
        assert_eq!(
            (
                first_history.items[0].run_number,
                second_history.items[0].run_number,
                second_history.next_cursor,
            ),
            (1, 2, None)
        );

        for path in [
            format!("/api/runs/{}/attempts", first.id),
            format!("/api/enrollments/{enrollment}/runs"),
        ] {
            for query in ["pageSize=0", "pageSize=101", "cursor=", "offset=1"] {
                let response = app
                    .clone()
                    .oneshot(
                        Request::builder()
                            .uri(format!("{path}?{query}"))
                            .header("cookie", &student_cookie)
                            .body(Body::empty())
                            .expect("invalid pagination request"),
                    )
                    .await
                    .expect("invalid pagination response");
                assert_eq!(
                    response.status(),
                    StatusCode::BAD_REQUEST,
                    "{path}?{query} must be rejected"
                );
            }
        }

        let summary_response = app
            .oneshot(
                Request::builder()
                    .uri(format!("/api/grading/summaries/{enrollment}"))
                    .header("cookie", student_cookie)
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("summary response");
        let summary: StudentAssignmentSummary =
            serde_json::from_value(json(summary_response).await).expect("summary");
        assert_eq!(
            (summary.completed_run_count, summary.total_question_attempts),
            (1, 1)
        );
    }

    #[tokio::test]
    async fn a_run_issues_only_one_active_question_then_advances() {
        let (store, backend, app, student_cookie, _, assignment_id, _) = fixture().await;
        let context = TenantContext::from_authenticated_session(TenantId::from_uuid(id(1)));
        let stored_assignment = store
            .get_assignment_for_edit(context, assignment_id)
            .await
            .expect("assignment read")
            .expect("fixture assignment");
        let mut items = stored_assignment.record.items.clone();
        let mut duplicate = items[0].clone();
        duplicate.id = question_model::AssignmentItemId::from_uuid(id(1_100_000));
        duplicate.position = u32::try_from(items.len()).expect("test assignment position fits u32");
        items.push(duplicate);
        store
            .replace_assignment(
                context,
                stored_assignment.record.course_id,
                assignment_id,
                stored_assignment.revision,
                learning_data_access::AssignmentUpdate {
                    title: stored_assignment.record.title,
                    items,
                    selection_groups: stored_assignment.record.selection_groups,
                    policies: stored_assignment.record.policies,
                },
            )
            .await
            .expect("two-position assignment");

        let started = app
            .clone()
            .oneshot(post_json(
                "/api/runs",
                &student_cookie,
                serde_json::json!({ "assignmentId": assignment_id }),
            ))
            .await
            .expect("start response");
        let run: AssignmentRun = serde_json::from_value(json(started).await).expect("run response");
        let first_page_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/api/runs/{}/attempts?pageSize=1", run.id))
                    .header("cookie", &student_cookie)
                    .body(Body::empty())
                    .expect("attempt request"),
            )
            .await
            .expect("first attempt page");
        let first_page: Page<QuestionAttempt> =
            serde_json::from_value(json(first_page_response).await).expect("attempt page");
        assert_eq!(first_page.items.len(), 1);
        assert_eq!(first_page.items[0].assignment_position, 0);

        let submission = Request::builder()
            .method("POST")
            .uri(format!("/api/submissions/{}", first_page.items[0].id))
            .header("cookie", &student_cookie)
            .header("content-type", "application/json")
            .header("idempotency-key", "advance-to-second")
            .body(Body::from(
                serde_json::json!({
                    "response": { "kind": "numeric", "value": 18.0 }
                })
                .to_string(),
            ))
            .expect("submission request");
        let submission_response = app
            .clone()
            .oneshot(submission)
            .await
            .expect("submission response");
        assert_eq!(submission_response.status(), StatusCode::OK);

        let second_page_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/api/runs/{}/attempts?pageSize=1", run.id))
                    .header("cookie", &student_cookie)
                    .body(Body::empty())
                    .expect("second attempt request"),
            )
            .await
            .expect("second attempt page");
        let second_page: Page<QuestionAttempt> =
            serde_json::from_value(json(second_page_response).await).expect("attempt page");
        assert_eq!(second_page.items.len(), 1);
        assert!(second_page.items[0].response.is_some());
        let cursor = second_page
            .next_cursor
            .expect("bounded first attempt page must continue");
        let continued_page_response = app
            .oneshot(
                Request::builder()
                    .uri(format!(
                        "/api/runs/{}/attempts?pageSize=1&cursor={}",
                        run.id,
                        cursor.as_str()
                    ))
                    .header("cookie", student_cookie)
                    .body(Body::empty())
                    .expect("continued attempt request"),
            )
            .await
            .expect("continued attempt page");
        let continued_page: Page<QuestionAttempt> =
            serde_json::from_value(json(continued_page_response).await).expect("attempt page");
        assert_eq!(continued_page.items.len(), 1);
        assert_ne!(second_page.items[0].id, continued_page.items[0].id);
        assert_eq!(continued_page.items[0].assignment_position, 1);
        assert!(continued_page.items[0].response.is_none());
        assert_eq!(continued_page.next_cursor, None);
        assert_eq!(backend.issued_seeds.lock().expect("seed record").len(), 2);
    }
}
