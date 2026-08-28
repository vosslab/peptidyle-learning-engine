//! Run-submission capability; this module owns its route behavior.

use super::contracts::RunBackend;
use super::prefetch::ensure_active_questions;
use super::submission_status::learner_submission_status_projection;
use super::support::*;
use crate::accepted_submission_worker::AcceptedSubmissionHandlerResult;

pub(super) async fn submit_response<S, B>(
    State(state): State<RunRouteState<S, B>>,
    headers: HeaderMap,
    Path((course, assignment, attempt_id)): Path<(CourseId, AssignmentId, QuestionAttemptId)>,
    Json(request): Json<SubmitResponseRequest>,
) -> Response
where
    S: Store + CatalogStore + ManualGradingStore + SessionStore + 'static,
    B: RunBackend + 'static,
{
    // ASVS 2.2.1 and 8.3.1: the typed route supplies routing context only;
    // persisted learner authority and exact relationships are server-verified.
    let binding = LearnerWorkRoutingBinding::new(course, assignment);
    let authenticated = match resolve_request_session(state.store.as_ref(), &headers).await {
        Ok(authenticated) => authenticated,
        Err(error) => return auth_error_response(error),
    };
    let idempotency_key = match submission_key(&headers) {
        Ok(key) => key,
        Err(message) => return error_response(StatusCode::BAD_REQUEST, message),
    };
    let actor = authenticated.record.subject.user();
    // First pass is intentionally answer-free. It establishes the route,
    // actor, idempotency fence, and exact replay before any private issued
    // family contract is materialized.
    let intent = match state
        .store
        .prepare_question_submission(
            authenticated.tenant_context,
            actor,
            binding,
            attempt_id,
            &request.response,
            &idempotency_key,
        )
        .await
    {
        Ok(learning_data_access::SubmissionPreparation::Replay(record)) => {
            return finish_submission(
                state.store.as_ref(),
                state.backend.as_ref(),
                &authenticated,
                binding,
                *record,
                SuccessorIssuance::Bound(binding),
            )
            .await;
        }
        Ok(learning_data_access::SubmissionPreparation::AcceptedPending(pending)) => {
            return accepted_pending_response(pending.attempt());
        }
        Ok(learning_data_access::SubmissionPreparation::FirstEffect(intent)) => *intent,
        Err(error) => return store_error_response(error),
    };
    // This snapshot is answer-free first-effect evidence. It validates the
    // browser shape, while the worker's sealed capability owns every private
    // family contract and grading call after durable acceptance.
    let question = intent.issued_question_snapshot.question();
    if matches!(
        &question.response,
        question_model::ResponseDefinition::FileUpload { .. }
    ) {
        return error_response(
            StatusCode::UNPROCESSABLE_ENTITY,
            "file upload submissions are unavailable",
        );
    }
    // The iMathAS launch broker owns its distinct, actor-bound provider
    // exchange. Its response must therefore reach the contracted nested route
    // rather than be converted into generic worker input.
    if matches!(
        &question.response,
        question_model::ResponseDefinition::ExternalTool {}
    ) {
        return error_response(
            StatusCode::UNPROCESSABLE_ENTITY,
            "external-tool submissions require the launch route",
        );
    }
    match intent.presentation.as_ref() {
        Some(snapshot) => {
            let report = domain::validation::validate_presentation_response_format(
                &snapshot.envelope.response,
                &request.response,
            );
            if !report.is_valid() {
                return no_store((StatusCode::UNPROCESSABLE_ENTITY, Json(report)).into_response());
            }
        }
        None => {
            let report =
                domain::validation::validate_response_format(&question.response, &request.response);
            if !report.is_valid() {
                return no_store((StatusCode::UNPROCESSABLE_ENTITY, Json(report)).into_response());
            }
        }
    }
    // ASVS 2.3.1 and 2.3.3: this one durable first effect is the sole
    // transition from a validated browser response into worker-owned grading.
    // It records the original browser response, preserves the replay fence,
    // and gives recovery a server-minted queue identity without releasing any
    // private grading material to the route.
    let execution_job = match learning_data_access::JobId::generate() {
        Ok(job) => job,
        Err(error) => return store_error_response(error),
    };
    let accepted = match state
        .automated_grading
        .accept_automated_submission(
            authenticated.tenant_context,
            learning_data_access::AcceptedSubmissionCommand {
                actor,
                course,
                assignment,
                attempt: attempt_id,
                idempotency_key,
                response: request.response,
                execution_job,
            },
        )
        .await
    {
        Ok(accepted) => accepted,
        Err(error) => return store_error_response(error),
    };
    let target = learning_data_access::AcceptedSubmissionExecutionTarget {
        tenant: accepted.tenant,
        attempt: accepted.attempt,
        submission: accepted.submission,
        job: execution_job,
    };
    match state
        .accepted_submission_fast_path
        .execute_accepted_submission(target)
        .await
    {
        Ok(
            AcceptedSubmissionHandlerResult::Committed | AcceptedSubmissionHandlerResult::Terminal,
        ) => {
            match learner_submission_status_projection(
                &state,
                &authenticated,
                binding,
                accepted.attempt,
            )
            .await
            {
                Ok(response) => response,
                Err(error) => {
                    log_accepted_submission_fast_path_error(
                        course,
                        assignment,
                        accepted.attempt,
                        &error,
                    );
                    accepted_pending_response(accepted.attempt)
                }
            }
        }
        #[cfg(feature = "e2e-grader-fault")]
        Ok(AcceptedSubmissionHandlerResult::RecoveryQueued) => {
            accepted_pending_response(accepted.attempt)
        }
        Ok(
            AcceptedSubmissionHandlerResult::Rescheduled
            | AcceptedSubmissionHandlerResult::ClaimNoLongerActive
            | AcceptedSubmissionHandlerResult::OutcomeUnknown,
        ) => accepted_pending_response(accepted.attempt),
        Err(error) => {
            log_accepted_submission_fast_path_error(course, assignment, accepted.attempt, &error);
            accepted_pending_response(accepted.attempt)
        }
    }
}

/// Emits bounded, answer-free telemetry after durable acceptance. The learner
/// receives the stable pending projection while recovery owns later progress.
fn log_accepted_submission_fast_path_error(
    course: CourseId,
    assignment: AssignmentId,
    attempt: QuestionAttemptId,
    error: &StoreError,
) {
    tracing::warn!(
        event = "accepted_submission_fast_path_pending",
        error_class = accepted_submission_fast_path_error_class(error),
        course = %course,
        assignment = %assignment,
        attempt = %attempt,
    );
}

fn accepted_submission_fast_path_error_class(error: &StoreError) -> &'static str {
    match error {
        StoreError::NotFound => "not_found",
        StoreError::AlreadyExists => "already_exists",
        StoreError::TenantMismatch => "tenant_mismatch",
        StoreError::Conflict => "conflict",
        StoreError::RetryableTransaction => "retryable_transaction",
        StoreError::Forbidden => "forbidden",
        StoreError::InvalidRecord(_) => "invalid_record",
        StoreError::RunModel(_) => "run_model",
        StoreError::TimedOut => "timed_out",
        StoreError::Unavailable(_) => "unavailable",
    }
}

/// Removes the combined legacy result unless every field it contains is
/// currently disclosed. This prevents points or correctness from leaking
/// through `QuestionAttempt.result` beside the field-by-field feedback DTO.
pub(super) fn apply_learner_disclosure(
    disclosure: domain::disclosure_policy::LearnerDisclosureDecision,
    scoring_status: question_model::ScoringStatus,
    attempt: &mut QuestionAttempt,
) {
    let disclosure = score_current_disclosure(disclosure, scoring_status);
    if !(disclosure.score && disclosure.per_item_correctness) {
        attempt.result = None;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SuccessorIssuance {
    /// Issue through the exact course/assignment route supplied by trusted composition.
    Bound(LearnerWorkRoutingBinding),
    /// Preserve the durable receipt and leave any successor for nested-route recovery.
    ///
    /// ASVS 2.3.1: a flat provider workflow cannot skip the route-bound
    /// recovery step needed before a successor is issued.
    Deferred,
}

pub(super) async fn finish_submission<S, B>(
    store: &S,
    backend: &B,
    authenticated: &AuthenticatedSession,
    binding: LearnerWorkRoutingBinding,
    record: SubmissionRecord,
    successor_issuance: SuccessorIssuance,
) -> Response
where
    S: Store + CatalogStore,
    B: RunBackend,
{
    let actor = authenticated.record.subject.user();
    // A submission receipt is a learner per-item surface. Fail closed if the
    // fresh atomic status snapshot cannot be read.
    let scoring_status = learner_scoring_status(store, authenticated, record.run.enrollment).await;
    let next_state = match store
        .submission_next_attempt(
            authenticated.tenant_context,
            actor,
            binding,
            record.attempt.id,
        )
        .await
    {
        Ok(value) => value,
        Err(_) => return submission_response(record, None, true, scoring_status),
    };
    let bound_successor = match (&next_state, successor_issuance) {
        (
            learning_data_access::SubmissionNextAttempt::Pending,
            SuccessorIssuance::Bound(binding),
        ) => Some(binding),
        _ => None,
    };
    let next_state = if let Some(binding) = bound_successor {
        // A receipt and grade are already durable. Try to issue its successor
        // before the normal first response, but never turn a post-grade
        // delivery failure into a failed submission or a second grade.
        if ensure_active_questions(
            store,
            backend,
            authenticated,
            binding,
            &record.run,
            Some(record.attempt.id),
        )
        .await
        .is_err()
        {
            return submission_response(record, None, true, scoring_status);
        }
        match store
            .submission_next_attempt(
                authenticated.tenant_context,
                actor,
                binding,
                record.attempt.id,
            )
            .await
        {
            Ok(learning_data_access::SubmissionNextAttempt::Pending) => {
                if store
                    .finalize_submission_next_attempt(
                        authenticated.tenant_context,
                        actor,
                        binding,
                        record.attempt.id,
                        None,
                    )
                    .await
                    .is_err()
                {
                    return submission_response(record, None, true, scoring_status);
                }
                learning_data_access::SubmissionNextAttempt::None
            }
            Ok(value) => value,
            Err(_) => return submission_response(record, None, true, scoring_status),
        }
    } else if matches!(
        next_state,
        learning_data_access::SubmissionNextAttempt::Pending
    ) {
        return submission_response(record, None, true, scoring_status);
    } else {
        next_state
    };
    let next_pending = false;
    let next_issued = match next_state {
        learning_data_access::SubmissionNextAttempt::None => None,
        learning_data_access::SubmissionNextAttempt::Issued(next) => Some(next_issued(next)),
        learning_data_access::SubmissionNextAttempt::Pending => None,
    };
    submission_response(record, next_issued, next_pending, scoring_status)
}

fn next_issued(next: ReceiptNextAttempt) -> NextIssuedAttempt {
    NextIssuedAttempt {
        id: next.id,
        run: next.run,
        question_version: next.question_version,
        seed: Seed::new(next.seed),
        deadline: next.deadline,
        assignment_position: next.assignment_position,
        rendered_question_sha256: next.rendered_question_sha256,
    }
}

pub(super) fn submission_response(
    record: SubmissionRecord,
    next_issued: Option<NextIssuedAttempt>,
    next_pending: bool,
    scoring_status: question_model::ScoringStatus,
) -> Response {
    let run_completion_status = record.run.completion_status();
    let (next_issued, next_pending) =
        if run_completion_status == question_model::RunCompletionStatus::Completed {
            (None, false)
        } else {
            (next_issued, next_pending)
        };
    let decision = score_current_disclosure(record.disclosure.decision(), scoring_status);
    let feedback = feedback_projection(
        decision,
        scoring_status,
        &record.attempt,
        record.feedback.content(),
    );
    let mut attempt = record.attempt;
    apply_learner_disclosure(decision, scoring_status, &mut attempt);
    no_store(
        Json(SubmissionReceipt {
            kind: "completed",
            accepted: true,
            attempt,
            feedback,
            scoring_status,
            run_completion_status,
            next_issued,
            next_pending,
        })
        .into_response(),
    )
}

pub(super) fn feedback_projection(
    disclosure: domain::disclosure_policy::LearnerDisclosureDecision,
    scoring_status: question_model::ScoringStatus,
    attempt: &QuestionAttempt,
    content: &FeedbackContent,
) -> Option<DisclosedFeedback> {
    project_run_feedback(disclosure, scoring_status, attempt.result, content)
}

/// Projects trusted feedback from one current, server-side disclosure decision.
pub(super) fn project_run_feedback(
    disclosure: domain::disclosure_policy::LearnerDisclosureDecision,
    scoring_status: question_model::ScoringStatus,
    result: Option<AttemptResult>,
    content: &FeedbackContent,
) -> Option<DisclosedFeedback> {
    project_feedback(
        score_current_disclosure(disclosure, scoring_status),
        result,
        content,
    )
}
