//! Run-submission capability; this module owns its route behavior.

use super::contracts::{RunBackend, RunSubmission, SubmissionDisposition};
use super::prefetch::{ensure_active_questions, load_run_question};
use super::queries::owned_run;
use super::support::*;

pub(super) async fn submit_response<S, B>(
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
    if matches!(
        &question.response,
        question_model::ResponseDefinition::FileUpload { .. }
    ) {
        return error_response(
            StatusCode::UNPROCESSABLE_ENTITY,
            "file upload submissions are unavailable",
        );
    }
    // Generated backends can assign attempt-specific choice/slot identifiers.
    // Validate against the exact provenance-checked envelope issued for this
    // attempt, never the published source template's response definition.
    let issued_envelope = match state
        .backend
        .reproduce(authenticated.tenant_context, reference, &question, &attempt)
        .await
    {
        Ok(envelope) => envelope,
        Err(error) => return backend_error_response(error),
    };
    let format_report =
        domain::validation::validate_response_format(&issued_envelope.response, &request.response);
    if !format_report.is_valid() {
        return no_store((StatusCode::UNPROCESSABLE_ENTITY, Json(format_report)).into_response());
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

pub(super) async fn apply_feedback_disclosure<S: CatalogStore>(
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

pub(super) async fn finish_submission<S, B>(
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

pub(super) async fn submission_response<S: CatalogStore>(
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

pub(super) async fn feedback_projection<S: CatalogStore>(
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
    // A submission receipt is a historical result from the grade transition.
    // An instructor can create an OnRelease record only after that transition,
    // so this initial projection is unreleased. Replayed receipts retain this
    // immutable state; the current run-summary projection receives the stored
    // release fact above.
    Ok(project_run_feedback(
        question.question.attempt_policy.feedback,
        run,
        false,
        attempt.result,
        content,
    ))
}

/// Projects trusted feedback with the authoritative run-completion and release facts.
///
/// This is the sole server projection seam for both immutable submission receipts and current
/// run summaries. The caller supplies `released` from the durable feedback-release record when a
/// current view is requested; the initial receipt is necessarily unreleased and remains an
/// immutable historical response on idempotent replay.
pub(super) fn project_run_feedback(
    policy: FeedbackDisclosure,
    run: &AssignmentRun,
    released: bool,
    result: Option<AttemptResult>,
    content: &FeedbackContent,
) -> Option<DisclosedFeedback> {
    project_feedback(
        policy,
        FeedbackDisclosureState {
            run_completed: run.completed_at.is_some(),
            released,
        },
        result,
        content,
    )
}
