//! Run-query and learner-projection capability; this module owns its route behavior.

use super::contracts::RunBackend;
use super::submission::{apply_learner_disclosure, project_run_feedback};
use super::support::*;

pub(super) async fn get_run<S, B>(
    State(state): State<RunRouteState<S, B>>,
    headers: HeaderMap,
    Path(run_id): Path<RunId>,
) -> Response
where
    S: Store
        + CatalogStore
        + CourseAppearanceStore
        + CourseItemAnalysisStore
        + SessionStore
        + AuthoritativeTimeStore
        + 'static,
    B: RunBackend + 'static,
{
    let authenticated = match resolve_request_session(state.store.as_ref(), &headers).await {
        Ok(authenticated) => authenticated,
        Err(error) => return auth_error_response(error),
    };
    match authorized_run(state.store.as_ref(), &authenticated, run_id).await {
        Ok(mut run) => {
            if let Err(response) =
                redact_learner_run_score(state.store.as_ref(), &authenticated, &mut run).await
            {
                return response.into_response();
            }
            no_store(Json(run).into_response())
        }
        Err(response) => response.into_response(),
    }
}

/// Returns the current, bounded completion view for one authorized run.
///
/// The enrolled learner receives the S5/S3-redacted DTO; a direct course
/// instructor retains the separate, established raw historical aggregate.
/// The store supplies private feedback and the current S3-backed disclosure
/// input in one authorized page read. Feedback-release receipts are audit
/// evidence only and do not change either GET projection.
pub(super) async fn get_run_summary<S, B>(
    State(state): State<RunRouteState<S, B>>,
    headers: HeaderMap,
    Path(run_id): Path<RunId>,
    Query(query): Query<RunQuery>,
) -> Response
where
    S: Store
        + CatalogStore
        + CourseAppearanceStore
        + CourseItemAnalysisStore
        + SessionStore
        + AuthoritativeTimeStore
        + 'static,
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
    let course = match run_summary_course(state.store.as_ref(), &authenticated, &page).await {
        Ok(course) => course,
        Err(response) => return response.into_response(),
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
            let feedback = project_run_feedback(
                outcome.disclosure.decision(),
                question_model::ScoringStatus::Current,
                outcome.result,
                content,
            );
            RunSummaryOutcome {
                attempt: outcome.attempt,
                assignment_position: outcome.assignment_position,
                submitted_at: outcome.submitted_at,
                response: outcome.response,
                feedback,
                scoring_status: page.scoring_status,
            }
        })
        .collect();
    let mut outcomes = learning_data_access::Page {
        items: outcomes,
        next_cursor: page.outcomes.next_cursor,
    };
    match run_summary_enrollment_access(state.store.as_ref(), &authenticated, page.run.enrollment)
        .await
    {
        Ok(RunSummaryEnrollmentAccess::Instructor) => {
            let mut run = page.run;
            let mut summary = page.summary;
            if !matches!(page.scoring_status, question_model::ScoringStatus::Current) {
                run.score = None;
                summary.current_score = None;
                summary.best_score = None;
                summary.latest_score = None;
            }
            no_store(
                Json(InstructorRunSummaryResponse {
                    course,
                    run,
                    summary,
                    scoring_status: page.scoring_status,
                    practice_allowed: page.practice_allowed,
                    outcomes,
                })
                .into_response(),
            )
        }
        Ok(RunSummaryEnrollmentAccess::Learner(enrollment)) => {
            if !matches!(page.scoring_status, question_model::ScoringStatus::Current) {
                for outcome in &mut outcomes.items {
                    if let Some(feedback) = &mut outcome.feedback {
                        feedback.points_earned = None;
                        feedback.points_possible = None;
                    }
                }
            }
            let (summary, score_disclosed) = match learner_assignment_progress(
                state.store.as_ref(),
                &authenticated,
                enrollment.assignment,
                Some(&learning_data_access::LearnerAssignmentSummarySnapshot {
                    summary: page.summary.clone(),
                    scoring_status: page.scoring_status,
                }),
            )
            .await
            {
                Ok(value) => value,
                Err(response) => return response.into_response(),
            };
            let mut run = page.run;
            if !score_disclosed {
                run.score = None;
            }
            no_store(
                Json(RunSummaryResponse {
                    course,
                    run,
                    summary,
                    practice_allowed: page.practice_allowed,
                    outcomes,
                })
                .into_response(),
            )
        }
        Err(response) => response.into_response(),
    }
}

/// Records an instructor feedback-release audit receipt after direct authority.
///
/// The response intentionally confirms only the content-free audit write.
/// Learner disclosure remains controlled solely by the current S4 decision.
pub(super) async fn release_attempt_feedback<S, B>(
    State(state): State<RunRouteState<S, B>>,
    headers: HeaderMap,
    Path(attempt): Path<QuestionAttemptId>,
) -> Response
where
    S: Store
        + CatalogStore
        + SessionStore
        + AuthoritativeTimeStore
        + CourseItemAnalysisStore
        + 'static,
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

pub(super) async fn list_attempts<S, B>(
    State(state): State<RunRouteState<S, B>>,
    headers: HeaderMap,
    Path(run_id): Path<RunId>,
    Query(query): Query<RunQuery>,
) -> Response
where
    S: Store
        + CatalogStore
        + SessionStore
        + AuthoritativeTimeStore
        + CourseItemAnalysisStore
        + 'static,
    B: RunBackend + 'static,
{
    let authenticated = match resolve_request_session(state.store.as_ref(), &headers).await {
        Ok(authenticated) => authenticated,
        Err(error) => return auth_error_response(error),
    };
    let run = match authorized_run(state.store.as_ref(), &authenticated, run_id).await {
        Ok(run) => run,
        Err(response) => return response.into_response(),
    };
    let page = match page_request(query) {
        Ok(page) => page,
        Err(error) => return error_response(StatusCode::BAD_REQUEST, &error.to_string()),
    };
    let mut page = match state
        .store
        .learner_list_question_attempts(
            authenticated.tenant_context,
            authenticated.record.subject.user(),
            run.id,
            page,
        )
        .await
    {
        Ok(Some(page)) => page,
        Ok(None) => return error_response(StatusCode::NOT_FOUND, "run not found"),
        Err(error) => return store_error_response(error),
    };
    let run_items = match state
        .store
        .learner_assignment_run_items(
            authenticated.tenant_context,
            authenticated.record.subject.user(),
            run.id,
        )
        .await
    {
        Ok(Some(items)) => items,
        Ok(None) => return error_response(StatusCode::NOT_FOUND, "run not found"),
        Err(error) => return store_error_response(error),
    };
    let scoring_status =
        learner_scoring_status(state.store.as_ref(), &authenticated, run.enrollment).await;
    for attempt in &mut page.items {
        if attempt.response.is_some() {
            let record = match state
                .store
                .submission_record(
                    authenticated.tenant_context,
                    authenticated.record.subject.user(),
                    attempt.id,
                )
                .await
            {
                Ok(Some(record)) => record,
                Ok(None) => {
                    return error_response(
                        StatusCode::SERVICE_UNAVAILABLE,
                        "submitted attempt receipt is unavailable",
                    );
                }
                Err(error) => return store_error_response(error),
            };
            *attempt = record.attempt;
            apply_learner_disclosure(record.disclosure.decision(), scoring_status, attempt);
            continue;
        }
    }
    no_store(
        Json(learning_data_access::Page {
            items: page
                .items
                .into_iter()
                .map(|attempt| LearnerAttemptProjection {
                    pool_selection: pool_selection_for_position(
                        &run_items,
                        attempt.assignment_position,
                    ),
                    attempt,
                    scoring_status,
                })
                .collect(),
            next_cursor: page.next_cursor,
        })
        .into_response(),
    )
}

pub(super) async fn get_attempt<S, B>(
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
        .learner_get_question_attempt(
            authenticated.tenant_context,
            authenticated.record.subject.user(),
            attempt_id,
        )
        .await
    {
        Ok(Some(attempt)) => attempt,
        Ok(None) => return error_response(StatusCode::NOT_FOUND, "attempt not found"),
        Err(error) => return store_error_response(error),
    };
    let run = match authorized_run(state.store.as_ref(), &authenticated, attempt.run).await {
        Ok(run) => run,
        Err(response) => return response.into_response(),
    };
    if attempt.response.is_some() {
        let record = match state
            .store
            .submission_record(
                authenticated.tenant_context,
                authenticated.record.subject.user(),
                attempt.id,
            )
            .await
        {
            Ok(Some(record)) => record,
            Ok(None) => {
                return error_response(
                    StatusCode::SERVICE_UNAVAILABLE,
                    "submitted attempt receipt is unavailable",
                );
            }
            Err(error) => return store_error_response(error),
        };
        attempt = record.attempt;
        let scoring_status =
            learner_scoring_status(state.store.as_ref(), &authenticated, run.enrollment).await;
        apply_learner_disclosure(record.disclosure.decision(), scoring_status, &mut attempt);
    }
    let pool_selection = match state
        .store
        .learner_assignment_run_items(
            authenticated.tenant_context,
            authenticated.record.subject.user(),
            run.id,
        )
        .await
    {
        Ok(Some(items)) => pool_selection_for_position(&items, attempt.assignment_position),
        Ok(None) => return error_response(StatusCode::NOT_FOUND, "run not found"),
        Err(error) => return store_error_response(error),
    };
    no_store(
        Json(LearnerAttemptProjection {
            attempt,
            scoring_status: learner_scoring_status(
                state.store.as_ref(),
                &authenticated,
                run.enrollment,
            )
            .await,
            pool_selection,
        })
        .into_response(),
    )
}

/// Returns the exact, answer-free delivery envelope for a route-bound attempt.
///
/// The one Store call atomically verifies the complete route tuple and current
/// learner authority before choosing active versus immutable-receipt delivery:
/// ASVS 2.2.1-2.2.3, 2.3.3, 8.2.2, 8.3.1, 8.4.1, and 15.4.2. It supplies the
/// minimum answer-free representation only (ASVS 14.2.6), and refusal stays
/// generic and fail-closed (ASVS 16.5.1, 16.5.3).
pub(super) async fn get_attempt_question<S, B>(
    State(state): State<RunRouteState<S, B>>,
    headers: HeaderMap,
    Path((course, assignment, attempt_id)): Path<(CourseId, AssignmentId, QuestionAttemptId)>,
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
        .read_issued_attempt_evidence(
            authenticated.tenant_context,
            authenticated.record.subject.user(),
            LearnerWorkRoutingBinding::new(course, assignment),
            attempt_id,
        )
        .await
    {
        // ASVS 2.3.3 and 15.4.2: the broker returns this lifecycle and its
        // immutable evidence from one authorization-bound transaction.
        Ok(IssuedAttemptRead::Active(evidence)) => match evidence.presentation_snapshot() {
            Some(snapshot) => no_store(Json(snapshot.envelope.clone()).into_response()),
            None => error_response(
                StatusCode::SERVICE_UNAVAILABLE,
                "issued presentation is unavailable",
            ),
        },
        Ok(IssuedAttemptRead::Submitted(read)) => match read.presentation() {
            Some(snapshot) => no_store(Json(snapshot.envelope.clone()).into_response()),
            None => error_response(
                StatusCode::SERVICE_UNAVAILABLE,
                "submitted attempt receipt is unavailable",
            ),
        },
        Ok(IssuedAttemptRead::TerminalWithoutReceipt(_)) => error_response(
            StatusCode::CONFLICT,
            "terminal attempt has no question-delivery receipt",
        ),
        Err(StoreError::NotFound | StoreError::Forbidden | StoreError::TenantMismatch) => {
            error_response(StatusCode::NOT_FOUND, "attempt not found")
        }
        Err(error) => store_error_response(error),
    }
}

pub(super) async fn get_enrollment<S, B>(
    State(state): State<RunRouteState<S, B>>,
    headers: HeaderMap,
    Path(enrollment_id): Path<question_model::EnrollmentId>,
) -> Response
where
    S: Store
        + CatalogStore
        + SessionStore
        + AuthoritativeTimeStore
        + CourseItemAnalysisStore
        + 'static,
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
            Err(response) => return response.into_response(),
        };
    let summary = match state
        .store
        .learner_get_summary(
            authenticated.tenant_context,
            authenticated.record.subject.user(),
            enrollment.id,
        )
        .await
    {
        Ok(Some(summary)) => summary,
        Ok(None) => return error_response(StatusCode::NOT_FOUND, "summary not found"),
        Err(error) => return store_error_response(error),
    };
    if enrollment.user == authenticated.record.subject.user() {
        let (summary, _) = match learner_assignment_progress(
            state.store.as_ref(),
            &authenticated,
            enrollment.assignment,
            Some(&summary),
        )
        .await
        {
            Ok(value) => value,
            Err(response) => return response.into_response(),
        };
        no_store(
            Json(EnrollmentView {
                enrollment,
                summary,
            })
            .into_response(),
        )
    } else {
        no_store(
            Json(serde_json::json!({ "enrollment": enrollment, "summary": summary.summary }))
                .into_response(),
        )
    }
}

pub(super) async fn get_summary<S, B>(
    State(state): State<RunRouteState<S, B>>,
    headers: HeaderMap,
    Path(enrollment_id): Path<question_model::EnrollmentId>,
) -> Response
where
    S: Store
        + CatalogStore
        + SessionStore
        + AuthoritativeTimeStore
        + CourseItemAnalysisStore
        + 'static,
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
            Err(response) => return response.into_response(),
        };
    match state
        .store
        .learner_get_summary(
            authenticated.tenant_context,
            authenticated.record.subject.user(),
            enrollment_id,
        )
        .await
    {
        Ok(Some(summary)) if enrollment.user == authenticated.record.subject.user() => {
            match learner_assignment_progress(
                state.store.as_ref(),
                &authenticated,
                enrollment.assignment,
                Some(&summary),
            )
            .await
            {
                Ok((summary, _)) => no_store(Json(summary).into_response()),
                Err(response) => response.into_response(),
            }
        }
        Ok(Some(summary)) => no_store(Json(summary.summary).into_response()),
        Ok(None) => error_response(StatusCode::NOT_FOUND, "summary not found"),
        Err(error) => store_error_response(error),
    }
}

pub(super) async fn list_runs<S, B>(
    State(state): State<RunRouteState<S, B>>,
    headers: HeaderMap,
    Path(enrollment_id): Path<question_model::EnrollmentId>,
    Query(query): Query<RunQuery>,
) -> Response
where
    S: Store
        + CatalogStore
        + SessionStore
        + AuthoritativeTimeStore
        + CourseItemAnalysisStore
        + 'static,
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
            Err(response) => return response.into_response(),
        };
    let page = match page_request(query) {
        Ok(page) => page,
        Err(error) => return error_response(StatusCode::BAD_REQUEST, &error.to_string()),
    };
    let actor = authenticated.record.subject.user();
    let result = if enrollment.user == actor {
        state
            .store
            .learner_list_runs(authenticated.tenant_context, actor, enrollment_id, page)
            .await
    } else {
        state
            .store
            .instructor_list_runs(authenticated.tenant_context, actor, enrollment_id, page)
            .await
    };
    match result {
        Ok(Some(mut page)) => {
            if enrollment.user == actor {
                let summary = match state
                    .store
                    .learner_get_summary(authenticated.tenant_context, actor, enrollment_id)
                    .await
                {
                    Ok(Some(summary)) => summary,
                    Ok(None) => return error_response(StatusCode::NOT_FOUND, "summary not found"),
                    Err(error) => return store_error_response(error),
                };
                let (_, score_disclosed) = match learner_assignment_progress(
                    state.store.as_ref(),
                    &authenticated,
                    enrollment.assignment,
                    Some(&summary),
                )
                .await
                {
                    Ok(value) => value,
                    Err(response) => return response.into_response(),
                };
                if !score_disclosed {
                    for run in &mut page.items {
                        run.score = None;
                    }
                }
            }
            no_store(Json(page).into_response())
        }
        Ok(None) => error_response(StatusCode::NOT_FOUND, "enrollment not found"),
        Err(error) => store_error_response(error),
    }
}

pub(super) async fn all_attempts<S: Store>(
    store: &S,
    authenticated: &AuthenticatedSession,
    run: RunId,
) -> HttpResult<Vec<QuestionAttempt>> {
    let size = PageSize::new(INTERNAL_ATTEMPT_PAGE_SIZE)
        .map_err(|error| error_response(StatusCode::INTERNAL_SERVER_ERROR, &error.to_string()))?;
    let mut page_request = PageRequest::first(size);
    let mut attempts = Vec::new();
    loop {
        let page = store
            .learner_list_question_attempts(
                authenticated.tenant_context,
                authenticated.record.subject.user(),
                run,
                page_request,
            )
            .await
            .map_err(store_error_response)?;
        let page = page.ok_or_else(|| error_response(StatusCode::NOT_FOUND, "run not found"))?;
        attempts.extend(page.items);
        let Some(cursor) = page.next_cursor else {
            return Ok(attempts);
        };
        page_request = PageRequest::after(cursor, size);
    }
}

pub(super) async fn authorized_run<S: Store>(
    store: &S,
    authenticated: &AuthenticatedSession,
    run_id: RunId,
) -> HttpResult<AssignmentRun> {
    let run = store
        .learner_get_run(
            authenticated.tenant_context,
            authenticated.record.subject.user(),
            run_id,
        )
        .await
        .map_err(store_error_response)?
        .ok_or_else(|| error_response(StatusCode::NOT_FOUND, "run not found"))?;
    authorized_enrollment(store, authenticated, run.enrollment, false).await?;
    Ok(run)
}

/// Redacts a run score only when the currently authenticated actor owns the
/// enrollment. Staff historical inspection keeps its separate raw capability.
async fn redact_learner_run_score<S: Store + AuthoritativeTimeStore + CourseItemAnalysisStore>(
    store: &S,
    authenticated: &AuthenticatedSession,
    run: &mut AssignmentRun,
) -> HttpResult<()> {
    let actor = authenticated.record.subject.user();
    let Some(enrollment) = store
        .learner_get_enrollment(authenticated.tenant_context, actor, run.enrollment)
        .await
        .map_err(store_error_response)?
    else {
        return Ok(());
    };
    let summary = store
        .learner_get_summary(authenticated.tenant_context, actor, enrollment.id)
        .await
        .map_err(store_error_response)?
        .ok_or_else(|| error_response(StatusCode::NOT_FOUND, "summary not found"))?;
    let (_, score_disclosed) =
        learner_assignment_progress(store, authenticated, enrollment.assignment, Some(&summary))
            .await?;
    if !score_disclosed {
        run.score = None;
    }
    Ok(())
}

/// The store has already authorized the summary-page read. Resolve which of
/// its two retained record capabilities applies before touching learner-only
/// accessors: instructors never traverse a learner entitlement query.
enum RunSummaryEnrollmentAccess {
    Learner(AssignmentEnrollment),
    Instructor,
}

async fn run_summary_enrollment_access<S: Store>(
    store: &S,
    authenticated: &AuthenticatedSession,
    enrollment_id: question_model::EnrollmentId,
) -> HttpResult<RunSummaryEnrollmentAccess> {
    let actor = authenticated.record.subject.user();
    if let Some(enrollment) = store
        .instructor_get_enrollment(authenticated.tenant_context, actor, enrollment_id)
        .await
        .map_err(store_error_response)?
        && enrollment.user != actor
    {
        return Ok(RunSummaryEnrollmentAccess::Instructor);
    }
    let enrollment = store
        .learner_get_enrollment(authenticated.tenant_context, actor, enrollment_id)
        .await
        .map_err(store_error_response)?
        .ok_or_else(|| error_response(StatusCode::NOT_FOUND, "enrollment not found"))?;
    Ok(RunSummaryEnrollmentAccess::Learner(enrollment))
}

pub(super) async fn run_summary_course<S>(
    store: &S,
    authenticated: &AuthenticatedSession,
    page: &learning_data_access::RunSummaryPageInput,
) -> HttpResult<CourseRouteData>
where
    S: Store + CourseAppearanceStore,
{
    let course_id = page.assignment.course_id;
    let record = store
        .get_course(authenticated.tenant_context, course_id)
        .await
        .map_err(store_error_response)?
        .ok_or_else(|| error_response(StatusCode::NOT_FOUND, "course not found"))?;
    let actor = authenticated.record.subject.user();
    let role = store
        .get_current_course_membership(authenticated.tenant_context, course_id, actor)
        .await
        .map_err(store_error_response)?
        .map(|membership| membership.role)
        .ok_or_else(|| error_response(StatusCode::NOT_FOUND, "course not found"))?;
    let appearance = store
        .course_appearance(
            authenticated.tenant_context,
            authenticated.record.token_hash,
            course_id,
        )
        .await
        .map_err(store_error_response)?
        .ok_or_else(|| error_response(StatusCode::NOT_FOUND, "course appearance not found"))?;
    let public_id = store
        .course_reference(authenticated.tenant_context, actor, course_id)
        .await
        .map_err(store_error_response)?
        .ok_or_else(|| error_response(StatusCode::NOT_FOUND, "course not found"))?;
    Ok(CourseRouteData {
        summary: record.summary(role, public_id),
        appearance,
    })
}

pub(super) async fn owned_run<S: Store>(
    store: &S,
    authenticated: &AuthenticatedSession,
    run_id: RunId,
) -> HttpResult<AssignmentRun> {
    let run = store
        .learner_get_run(
            authenticated.tenant_context,
            authenticated.record.subject.user(),
            run_id,
        )
        .await
        .map_err(store_error_response)?
        .ok_or_else(|| error_response(StatusCode::NOT_FOUND, "run not found"))?;
    owned_enrollment(store, authenticated, run.enrollment).await?;
    Ok(run)
}

pub(super) async fn owned_enrollment<S: Store>(
    store: &S,
    authenticated: &AuthenticatedSession,
    enrollment_id: question_model::EnrollmentId,
) -> HttpResult<AssignmentEnrollment> {
    authorized_enrollment(store, authenticated, enrollment_id, true).await
}

pub(super) async fn owned_assignment_for_run<S: Store>(
    store: &S,
    authenticated: &AuthenticatedSession,
    run: &AssignmentRun,
) -> HttpResult<learning_data_access::AssignmentRecord> {
    let enrollment = owned_enrollment(store, authenticated, run.enrollment).await?;
    store
        .get_assignment(authenticated.tenant_context, enrollment.assignment)
        .await
        .map_err(store_error_response)?
        .ok_or_else(|| error_response(StatusCode::NOT_FOUND, "assignment not found"))
        .map_err(Into::into)
}

pub(super) async fn authorized_enrollment<S: Store>(
    store: &S,
    authenticated: &AuthenticatedSession,
    enrollment_id: question_model::EnrollmentId,
    require_owner: bool,
) -> HttpResult<AssignmentEnrollment> {
    let actor = authenticated.record.subject.user();
    if let Some(enrollment) = store
        .learner_get_enrollment(authenticated.tenant_context, actor, enrollment_id)
        .await
        .map_err(store_error_response)?
    {
        return Ok(enrollment);
    }
    // Staff retain the distinct historical-record capability. Learners never
    // fall through to this raw read: an enrollment belonging to the caller is
    // rejected below unless the learner capability proved active membership.
    let enrollment = store
        .instructor_get_enrollment(authenticated.tenant_context, actor, enrollment_id)
        .await
        .map_err(store_error_response)?
        .ok_or_else(|| error_response(StatusCode::NOT_FOUND, "enrollment not found"))?;
    let assignment = store
        .get_assignment(authenticated.tenant_context, enrollment.assignment)
        .await
        .map_err(store_error_response)?
        .ok_or_else(|| error_response(StatusCode::NOT_FOUND, "enrollment not found"))?;
    if enrollment.user == actor {
        return Err(error_response(StatusCode::NOT_FOUND, "enrollment not found").into());
    }
    if require_owner {
        return Err(error_response(StatusCode::NOT_FOUND, "enrollment not found").into());
    }
    let instructor = store
        .get_current_course_membership(authenticated.tenant_context, assignment.course_id, actor)
        .await
        .map_err(store_error_response)?
        .is_some_and(|membership| {
            matches!(
                membership.role,
                question_model::CourseMembershipRole::Instructor
            )
        });
    if instructor {
        Ok(enrollment)
    } else {
        Err(error_response(StatusCode::NOT_FOUND, "enrollment not found").into())
    }
}
