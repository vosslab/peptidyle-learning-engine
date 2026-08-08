//! Authenticated run, attempt, submission, and grading-summary routes (MOD-API-RUN).
//!
//! The store owns timestamps, run numbers, one-active-question enforcement,
//! idempotency, and transactional summary changes. A pluggable server-only
//! backend owns rendering provenance and correctness so this route group does
//! not choose the first native family or expose an answer key.

use std::sync::Arc;

use async_trait::async_trait;
use axum::extract::{DefaultBodyLimit, Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use grading::GradeOutcome;
use question_model::run_policy::FeedbackDisclosure;
use question_model::{
    AssignmentEnrollment, AssignmentId, AssignmentRun, AttemptProvenance, ProblemVersionRef,
    QuestionAttempt, QuestionAttemptId, QuestionDefinition, RunId, StudentAssignmentSummary,
    StudentResponse, UserRole,
};
use serde::{Deserialize, Serialize};
use store::{
    CatalogStore, Cursor, IssueQuestionAttemptCommand, PageRequest, PageSize, PaginationError,
    SessionStore, Store, StoreError, SubmissionIdempotencyKey, SubmissionRecord,
    SubmitQuestionAttemptCommand,
};

use crate::auth::{AuthenticatedSession, auth_error_response, no_store, resolve_request_session};

const DEFAULT_PAGE_SIZE: u16 = 50;
const INTERNAL_ATTEMPT_PAGE_SIZE: u16 = PageSize::MAX;
const MAX_SUBMISSION_BODY_BYTES: usize = 64 * 1_024;
const IDEMPOTENCY_HEADER: &str = "idempotency-key";
const MAX_JSON_SAFE_INTEGER: u64 = (1_u64 << 53) - 1;

/// Key-free metadata produced while a trusted adapter issues one instance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IssuedAttemptMetadata {
    /// SHA-256 of generated parameter values.
    pub parameter_hash: String,
    /// Complete reproducibility record without an answer or key.
    pub provenance: AttemptProvenance,
}

/// Adapter-owned server boundary used by the generic run routes.
#[async_trait]
pub trait RunBackend: Send + Sync {
    /// Generates or renders one fresh instance from the server-owned seed.
    async fn issue(
        &self,
        question: &QuestionDefinition,
        seed: u64,
    ) -> Result<IssuedAttemptMetadata, RunBackendError>;

    /// Grades one response without returning or serializing its key.
    async fn grade(
        &self,
        question: &QuestionDefinition,
        attempt: &QuestionAttempt,
        response: &StudentResponse,
    ) -> Result<GradeOutcome, RunBackendError>;
}

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
    S: Store + CatalogStore + SessionStore + 'static,
    B: RunBackend + 'static,
{
    Router::new()
        .route("/api/runs", post(start_run::<S, B>))
        .route("/api/runs/{run}", get(get_run::<S, B>))
        .route("/api/runs/{run}/attempts", get(list_attempts::<S, B>))
        .route("/api/attempts/{attempt}", get(get_attempt::<S, B>))
        .route("/api/submissions/{attempt}", post(submit_response::<S, B>))
        .route(
            "/api/grading/summaries/{enrollment}",
            get(get_summary::<S, B>),
        )
        .route("/api/enrollments/{enrollment}", get(get_enrollment::<S, B>))
        .route("/api/enrollments/{enrollment}/runs", get(list_runs::<S, B>))
        .layer(DefaultBodyLimit::max(MAX_SUBMISSION_BODY_BYTES))
        .with_state(RunRouteState { store, backend })
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
#[serde(rename_all = "camelCase")]
struct SubmitResponseRequest {
    response: StudentResponse,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SubmissionReceipt {
    accepted: bool,
    attempt: QuestionAttempt,
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
    S: Store + CatalogStore + SessionStore + 'static,
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
    if let Err(response) = ensure_active_questions(
        state.store.as_ref(),
        state.backend.as_ref(),
        &authenticated,
        &run,
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

async fn submit_response<S, B>(
    State(state): State<RunRouteState<S, B>>,
    headers: HeaderMap,
    Path(attempt_id): Path<QuestionAttemptId>,
    Json(request): Json<SubmitResponseRequest>,
) -> Response
where
    S: Store + CatalogStore + SessionStore + 'static,
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
            return submission_response(state.store.as_ref(), authenticated.tenant_context, record)
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
    let question = match state
        .store
        .get_catalog_problem(authenticated.tenant_context, reference)
        .await
    {
        Ok(Some(record)) => record.question,
        Ok(None) => return error_response(StatusCode::NOT_FOUND, "question version not found"),
        Err(error) => return store_error_response(error),
    };
    let format_report =
        domain::validation::validate_response_format(&question.response, &request.response);
    if !format_report.is_valid() {
        return no_store((StatusCode::UNPROCESSABLE_ENTITY, Json(format_report)).into_response());
    }
    let result = match state
        .backend
        .grade(&question, &attempt, &request.response)
        .await
    {
        Ok(GradeOutcome::Graded(result)) => result,
        Ok(GradeOutcome::Ungraded) => {
            return error_response(
                StatusCode::UNPROCESSABLE_ENTITY,
                "this run backend does not produce a server grade",
            );
        }
        Err(error) => return backend_error_response(error),
    };
    let record = match state
        .store
        .submit_question_attempt(
            authenticated.tenant_context,
            SubmitQuestionAttemptCommand {
                actor,
                attempt: attempt.id,
                response: request.response,
                result,
                idempotency_key,
            },
        )
        .await
    {
        Ok(record) => record,
        Err(error) => return store_error_response(error),
    };
    if record.run.completed_at.is_none()
        && let Err(response) = ensure_active_questions(
            state.store.as_ref(),
            state.backend.as_ref(),
            &authenticated,
            &record.run,
        )
        .await
    {
        return response;
    }
    submission_response(state.store.as_ref(), authenticated.tenant_context, record).await
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
) -> Result<(), Response>
where
    S: Store + CatalogStore,
    B: RunBackend,
{
    if run.completed_at.is_some() {
        return Ok(());
    }
    let enrollment = owned_enrollment(store, authenticated, run.enrollment).await?;
    let assignment = store
        .get_assignment(authenticated.tenant_context, enrollment.assignment)
        .await
        .map_err(store_error_response)?
        .ok_or_else(|| error_response(StatusCode::NOT_FOUND, "assignment not found"))?;
    let attempts = all_attempts(store, authenticated.tenant_context, run.id).await?;

    if attempts.iter().any(|attempt| attempt.response.is_none()) {
        return Ok(());
    }

    for (position, reference) in assignment.problems.iter().copied().enumerate() {
        let position = checked_assignment_position(position)
            .map_err(|message| error_response(StatusCode::UNPROCESSABLE_ENTITY, message))?;
        if attempts
            .iter()
            .all(|attempt| attempt.assignment_position != position)
        {
            let question = load_run_question(store, authenticated, reference).await?;
            issue_question(
                store,
                backend,
                authenticated,
                run,
                position,
                reference,
                &question,
            )
            .await?;
            return Ok(());
        }
    }

    for (position, reference) in assignment.problems.iter().copied().enumerate() {
        let position = checked_assignment_position(position)
            .map_err(|message| error_response(StatusCode::UNPROCESSABLE_ENTITY, message))?;
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
            position,
            reference,
            &question,
        )
        .await?;
        return Ok(());
    }
    Ok(())
}

fn checked_assignment_position(position: usize) -> Result<u32, &'static str> {
    u32::try_from(position).map_err(|_| "assignment has too many question positions")
}

async fn load_run_question<S: CatalogStore>(
    store: &S,
    authenticated: &AuthenticatedSession,
    reference: ProblemVersionRef,
) -> Result<QuestionDefinition, Response> {
    let question = store
        .get_catalog_problem(authenticated.tenant_context, reference)
        .await
        .map_err(store_error_response)?
        .ok_or_else(|| error_response(StatusCode::NOT_FOUND, "question version not found"))
        .map(|record| record.question)?;
    if question.attempt_policy.max_attempts == Some(0) {
        return Err(error_response(
            StatusCode::UNPROCESSABLE_ENTITY,
            "question max attempts must be greater than zero",
        ));
    }
    Ok(question)
}

async fn issue_question<S, B>(
    store: &S,
    backend: &B,
    authenticated: &AuthenticatedSession,
    run: &AssignmentRun,
    assignment_position: u32,
    reference: ProblemVersionRef,
    question: &QuestionDefinition,
) -> Result<QuestionAttempt, Response>
where
    S: Store,
    B: RunBackend,
{
    let seed = fresh_seed().map_err(backend_error_response)?;
    let issued = backend
        .issue(question, seed)
        .await
        .map_err(backend_error_response)?;
    store
        .issue_or_resume_question_attempt(
            authenticated.tenant_context,
            IssueQuestionAttemptCommand {
                actor: authenticated.record.subject.user(),
                attempt: QuestionAttemptId::generate(),
                run: run.id,
                assignment_position,
                problem: reference.problem,
                question_version: reference.version,
                seed,
                parameter_hash: issued.parameter_hash,
                provenance: issued.provenance,
            },
        )
        .await
        .map_err(store_error_response)
}

async fn all_attempts<S: Store>(
    store: &S,
    context: store::TenantContext,
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
    context: store::TenantContext,
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
    let disclose = match question.question.attempt_policy.feedback {
        FeedbackDisclosure::ImmediateFull | FeedbackDisclosure::ImmediateCorrectness => true,
        FeedbackDisclosure::Deferred => run.completed_at.is_some(),
        FeedbackDisclosure::OnRelease => false,
    };
    if !disclose {
        attempt.result = None;
    }
    Ok(())
}

async fn submission_response<S: CatalogStore>(
    store: &S,
    context: store::TenantContext,
    record: SubmissionRecord,
) -> Response {
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
        })
        .into_response(),
    )
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
        StoreError::Unavailable(_) => {
            error_response(StatusCode::SERVICE_UNAVAILABLE, "run storage unavailable")
        }
    }
}

fn error_response(status: StatusCode, message: &str) -> Response {
    no_store((status, Json(serde_json::json!({ "error": message }))).into_response())
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::*;
    use axum::body::{Body, to_bytes};
    use axum::http::Request;
    use grading::{AnswerKey, GradingError, grade};
    use question_model::answer::NumericTolerance;
    use question_model::envelope::ContentBlock;
    use question_model::generation::RandomizationDefinition;
    use question_model::response::ResponseDefinition;
    use question_model::run_policy::{
        AttemptPolicy, CompletionRequirement, ContinuedPractice, GradePolicy, RunPolicies,
        TimingPolicy, VariationPolicy,
    };
    use question_model::taxonomy::License;
    use question_model::{
        ActivityTimestamp, BackendCapabilities, Capability, CourseId, CourseMembership,
        CourseMembershipRole, EnrollmentId, GradingDefinition, ImplementationVersion, ProblemId,
        PublicationScope, QuestionMetadata, QuestionSource, StudentId, TenantId, UserId, VersionId,
        WorkspaceId,
    };
    use store::memory::MemoryStore;
    use store::{
        AssignmentRecord, CourseRecord, DraftRecord, Page, PublishDraftCommand, SessionLifetime,
        SessionSubject, TenantContext,
    };
    use tower::ServiceExt;
    use uuid::Uuid;

    #[derive(Debug, Default)]
    struct NumericBackend {
        grade_calls: AtomicUsize,
        issued_seeds: std::sync::Mutex<Vec<u64>>,
    }

    #[async_trait]
    impl RunBackend for NumericBackend {
        async fn issue(
            &self,
            _question: &QuestionDefinition,
            seed: u64,
        ) -> Result<IssuedAttemptMetadata, RunBackendError> {
            self.issued_seeds.lock().expect("seed record").push(seed);
            Ok(IssuedAttemptMetadata {
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

        async fn grade(
            &self,
            question: &QuestionDefinition,
            _attempt: &QuestionAttempt,
            response: &StudentResponse,
        ) -> Result<GradeOutcome, RunBackendError> {
            self.grade_calls.fetch_add(1, Ordering::SeqCst);
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

    #[test]
    fn fresh_server_seeds_fit_the_exact_json_integer_range() {
        for _ in 0..128 {
            assert!(fresh_seed().expect("OS random seed") <= MAX_JSON_SAFE_INTEGER);
        }
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
            question: QuestionDefinition {
                version,
                problem: None,
                workspace,
                source: QuestionSource::Native {
                    family: "test_numeric".to_string(),
                },
                prompt: vec![ContentBlock::Text {
                    markdown: "What is the molar mass of water?".to_string(),
                }],
                response: ResponseDefinition::Numeric {
                    tolerance: NumericTolerance::Absolute { epsilon: 0.1 },
                    unit: Some("g/mol".to_string()),
                },
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
        store
            .upsert_draft(context, draft.clone())
            .await
            .expect("draft");
        store
            .publish_draft(
                context,
                PublishDraftCommand {
                    expected_draft: draft,
                    problem,
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
            .upsert_assignment(
                context,
                AssignmentRecord {
                    id: assignment,
                    tenant,
                    course_id: course,
                    title: "Molar mass mastery".to_string(),
                    problems: vec![ProblemVersionRef { problem, version }],
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
        let backend = Arc::new(NumericBackend::default());
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

    async fn issued_cookie(store: &MemoryStore, user: UserId, name: &str) -> String {
        let subject = SessionSubject::new(
            TenantId::from_uuid(id(1)),
            user,
            name,
            vec![UserRole::Student],
        )
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
        let serialized_receipt = first_receipt.to_string();
        for answer_bearing_field in ["answerKey", "expected", "rubric"] {
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
                    .header("cookie", outsider_cookie)
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("outsider response");
        assert_eq!(outsider.status(), StatusCode::NOT_FOUND);

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
        let mut assignment = store
            .get_assignment(context, assignment_id)
            .await
            .expect("assignment read")
            .expect("fixture assignment");
        assignment.problems.push(assignment.problems[0]);
        store
            .upsert_assignment(context, assignment)
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
                    .uri(format!("/api/runs/{}/attempts", run.id))
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
            .oneshot(
                Request::builder()
                    .uri(format!("/api/runs/{}/attempts", run.id))
                    .header("cookie", student_cookie)
                    .body(Body::empty())
                    .expect("second attempt request"),
            )
            .await
            .expect("second attempt page");
        let second_page: Page<QuestionAttempt> =
            serde_json::from_value(json(second_page_response).await).expect("attempt page");
        assert_eq!(second_page.items.len(), 2);
        assert!(second_page.items[0].response.is_some());
        assert_eq!(second_page.items[1].assignment_position, 1);
        assert!(second_page.items[1].response.is_none());
        assert_eq!(backend.issued_seeds.lock().expect("seed record").len(), 2);
    }
}
