//! Run-query and learner-projection capability; this module owns its route behavior.

use super::contracts::RunBackend;
use super::prefetch::load_run_question;
use super::submission::{
    apply_feedback_disclosure, apply_receipt_feedback_disclosure, project_run_feedback,
};
use super::support::*;

pub(super) async fn get_run<S, B>(
    State(state): State<RunRouteState<S, B>>,
    headers: HeaderMap,
    Path(run_id): Path<RunId>,
) -> Response
where
    S: Store + CatalogStore + CourseAppearanceStore + SessionStore + 'static,
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
pub(super) async fn get_run_summary<S, B>(
    State(state): State<RunRouteState<S, B>>,
    headers: HeaderMap,
    Path(run_id): Path<RunId>,
    Query(query): Query<RunQuery>,
) -> Response
where
    S: Store + CatalogStore + CourseAppearanceStore + SessionStore + 'static,
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
        Err(response) => return response,
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
                outcome.feedback_policy,
                &page.run,
                outcome.release.is_some(),
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
            course,
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
pub(super) async fn release_attempt_feedback<S, B>(
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

pub(super) async fn list_attempts<S, B>(
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
            apply_receipt_feedback_disclosure(record.feedback_disclosure, &run, attempt);
            continue;
        }
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
        Err(response) => return response,
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
        apply_receipt_feedback_disclosure(record.feedback_disclosure, &run, &mut attempt);
    } else if let Err(response) = apply_feedback_disclosure(
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
/// Native and WeBWorK attempts carry an owned, answer-free issuance snapshot.
/// In particular, a submitted attempt must never be rebuilt from current
/// catalog or renderer state: a receipt is historical even if authored content
/// later changes. Response families without a native envelope retain the
/// contracted backend path while active, but cannot be reconstructed after a
/// submission.
pub(super) async fn get_attempt_question<S, B>(
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
    if let Err(response) = authorized_run(state.store.as_ref(), &authenticated, attempt.run).await {
        return response;
    }
    if attempt.response.is_some() {
        // A submitted question is historical. Read its immutable receipt,
        // including the receipt copy of the issuance snapshot, rather than
        // allowing a catalog edit or renderer change to alter the GET result.
        return match state
            .store
            .submission_record(
                authenticated.tenant_context,
                authenticated.record.subject.user(),
                attempt.id,
            )
            .await
        {
            Ok(Some(record)) => match record.presentation {
                Some(snapshot) => no_store(Json(snapshot.envelope).into_response()),
                None => error_response(
                    StatusCode::CONFLICT,
                    "submitted attempt has no native presentation receipt",
                ),
            },
            Ok(None) => error_response(
                StatusCode::CONFLICT,
                "submitted attempt receipt is unavailable",
            ),
            Err(error) => store_error_response(error),
        };
    }
    match state
        .store
        .get_attempt_presentation_snapshot(
            authenticated.tenant_context,
            authenticated.record.subject.user(),
            attempt.id,
        )
        .await
    {
        Ok(Some(snapshot)) => return no_store(Json(snapshot.envelope).into_response()),
        Ok(None) => {}
        Err(error) => return store_error_response(error),
    };
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

pub(super) async fn get_enrollment<S, B>(
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
    no_store(
        Json(EnrollmentView {
            enrollment,
            summary,
        })
        .into_response(),
    )
}

pub(super) async fn get_summary<S, B>(
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
        .learner_get_summary(
            authenticated.tenant_context,
            authenticated.record.subject.user(),
            enrollment_id,
        )
        .await
    {
        Ok(Some(summary)) => no_store(Json(summary).into_response()),
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
        .learner_list_runs(
            authenticated.tenant_context,
            authenticated.record.subject.user(),
            enrollment_id,
            page,
        )
        .await
    {
        Ok(Some(page)) => no_store(Json(page).into_response()),
        Ok(None) => error_response(StatusCode::NOT_FOUND, "enrollment not found"),
        Err(error) => store_error_response(error),
    }
}

pub(super) async fn all_attempts<S: Store>(
    store: &S,
    authenticated: &AuthenticatedSession,
    run: RunId,
) -> Result<Vec<QuestionAttempt>, Response> {
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
) -> Result<AssignmentRun, Response> {
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

pub(super) async fn run_summary_course<S>(
    store: &S,
    authenticated: &AuthenticatedSession,
    page: &learning_data_access::RunSummaryPageInput,
) -> Result<CourseRouteData, Response>
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
    let role = record.role_for(actor);
    let Some(role) = role else {
        return Err(error_response(StatusCode::NOT_FOUND, "course not found"));
    };
    let appearance = store
        .course_appearance(
            authenticated.tenant_context,
            authenticated.record.token_hash,
            course_id,
        )
        .await
        .map_err(store_error_response)?
        .ok_or_else(|| error_response(StatusCode::NOT_FOUND, "course appearance not found"))?;
    Ok(CourseRouteData {
        summary: record.summary(role),
        appearance,
    })
}

pub(super) async fn owned_run<S: Store>(
    store: &S,
    authenticated: &AuthenticatedSession,
    run_id: RunId,
) -> Result<AssignmentRun, Response> {
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
) -> Result<AssignmentEnrollment, Response> {
    authorized_enrollment(store, authenticated, enrollment_id, true).await
}

pub(super) async fn owned_assignment_for_run<S: Store>(
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

pub(super) async fn authorized_enrollment<S: Store>(
    store: &S,
    authenticated: &AuthenticatedSession,
    enrollment_id: question_model::EnrollmentId,
    require_owner: bool,
) -> Result<AssignmentEnrollment, Response> {
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
    let course = store
        .get_course(authenticated.tenant_context, assignment.course_id)
        .await
        .map_err(store_error_response)?
        .ok_or_else(|| error_response(StatusCode::NOT_FOUND, "enrollment not found"))?;
    if enrollment.user == actor {
        return Err(error_response(
            StatusCode::NOT_FOUND,
            "enrollment not found",
        ));
    }
    if require_owner {
        return Err(error_response(
            StatusCode::NOT_FOUND,
            "enrollment not found",
        ));
    }
    let instructor = course
        .role_for(actor)
        .is_some_and(|role| matches!(role, question_model::CourseMembershipRole::Instructor));
    if instructor {
        Ok(enrollment)
    } else {
        Err(error_response(
            StatusCode::NOT_FOUND,
            "enrollment not found",
        ))
    }
}
