//! Run-submission capability; this module owns its route behavior.

use super::contracts::{RunBackend, RunSubmission, SubmissionDisposition};
use super::prefetch::ensure_active_questions;
use super::support::*;
use question_model::presentation::{reproduce_presentation_v1, translate_rendered_response_v1};

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
        Ok(learning_data_access::SubmissionPreparation::FirstEffect(intent)) => *intent,
        Err(error) => return store_error_response(error),
    };
    // The store validates the immutable issue tuple. Presentation-bearing
    // attempts therefore return their exact answer-free schema here, while a
    // missing or corrupt required snapshot fails closed before any grade or
    // receipt mutation. Envelope-less families legitimately return None.
    let expected_attempt = intent.attempt.id;
    let presentation = intent.presentation.clone();
    let grading_envelope = intent.grading_envelope.clone();
    let presentation_binding = intent.presentation_binding;
    // This snapshot is answer-free first-effect evidence. It can validate the
    // public browser shape, but cannot construct a backend grade: the sealed
    // facade below supplies the complete family-private preparation only once
    // replay and current authorization have both been ruled out.
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
    let (submission_response, _) = match (presentation.as_ref(), grading_envelope.as_ref()) {
        (Some(snapshot), Some(envelope)) => {
            let report = domain::validation::validate_presentation_response_format(
                &snapshot.envelope.response,
                &request.response,
            );
            if !report.is_valid() {
                return no_store((StatusCode::UNPROCESSABLE_ENTITY, Json(report)).into_response());
            }
            let Some(presentation_binding) = presentation_binding else {
                return error_response(
                    StatusCode::SERVICE_UNAVAILABLE,
                    "issued presentation binding is unavailable",
                );
            };
            let issued = match reproduce_presentation_v1(
                envelope,
                &snapshot.asset_bindings,
                presentation_binding,
            ) {
                Ok(issued) if issued.envelope == snapshot.envelope => issued,
                Ok(_) => {
                    return error_response(
                        StatusCode::SERVICE_UNAVAILABLE,
                        "issued presentation contract is unavailable",
                    );
                }
                Err(_) => {
                    return error_response(
                        StatusCode::SERVICE_UNAVAILABLE,
                        "issued presentation contract is unavailable",
                    );
                }
            };
            let translated = match translate_rendered_response_v1(&request.response, &issued) {
                Ok(response) => response,
                Err(_) => {
                    return error_response(
                        StatusCode::SERVICE_UNAVAILABLE,
                        "issued presentation contract is unavailable",
                    );
                }
            };
            let private_report =
                domain::validation::validate_response_format(&envelope.response, &translated);
            if !private_report.is_valid() {
                return error_response(
                    StatusCode::SERVICE_UNAVAILABLE,
                    "issued presentation contract is unavailable",
                );
            }
            (translated, Some(envelope))
        }
        (None, None) => {
            let report =
                domain::validation::validate_response_format(&question.response, &request.response);
            if !report.is_valid() {
                return no_store((StatusCode::UNPROCESSABLE_ENTITY, Json(report)).into_response());
            }
            (request.response.clone(), None)
        }
        _ => {
            return error_response(
                StatusCode::SERVICE_UNAVAILABLE,
                "issued grading contract is unavailable",
            );
        }
    };
    let sealed_preparation = match state
        .sealed_execution
        .prepare_sealed_private_execution(
            authenticated.tenant_context,
            actor,
            binding,
            intent,
            &request.response,
            &idempotency_key,
        )
        .await
    {
        Ok(learning_data_access::SealedPrivateExecutionPreparation::Replay(record)) => {
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
        Ok(learning_data_access::SealedPrivateExecutionPreparation::Grade(prepared)) => *prepared,
        Err(error) => return store_error_response(error),
    };
    // Only the sealed result reaches a trusted backend. In particular, this
    // prevents adding a private family field to ordinary `Store` preparation
    // from silently re-enabling a route-to-grader data path.
    let attempt = sealed_preparation.attempt;
    let reference = ProblemVersionRef {
        problem: attempt.problem,
        version: attempt.question_version,
    };
    let issued_question_snapshot = sealed_preparation.issued_question_snapshot;
    let sealed_presentation = sealed_preparation.presentation;
    let sealed_grading_envelope = sealed_preparation.grading_envelope;
    let flat_grading = sealed_preparation.flat_grading;
    let webwork_grading = sealed_preparation.webwork_grading;
    let issued_qti_grading = sealed_preparation.issued_qti_grading;
    let webwork_replay = sealed_preparation.webwork_replay;
    let sealed_presentation_binding = sealed_preparation.presentation_binding;
    // The sealed broker has reauthorized the exact route under its own locks.
    // A changed attempt/presentation witness fails closed rather than mixing
    // ordinary public validation with another attempt's private contract.
    if attempt_id != expected_attempt
        || attempt_id != attempt.id
        || sealed_presentation_binding != presentation_binding
        || sealed_presentation != presentation
        || sealed_grading_envelope != grading_envelope
    {
        return error_response(
            StatusCode::SERVICE_UNAVAILABLE,
            "issued grading contract is unavailable",
        );
    }
    let disposition = match state
        .backend
        .submit(RunSubmission {
            context: authenticated.tenant_context,
            actor,
            idempotency_key: idempotency_key.clone(),
            reference,
            issued_question_snapshot: &issued_question_snapshot,
            attempt: &attempt,
            issued_grading_envelope: sealed_grading_envelope.as_ref(),
            issued_flat_grading: flat_grading.as_ref(),
            issued_webwork_grading: webwork_grading.as_ref(),
            issued_qti_grading: issued_qti_grading.as_ref(),
            issued_webwork_replay: webwork_replay.as_ref(),
            issued_presentation_binding: sealed_presentation_binding,
            issued_presentation: sealed_presentation.as_ref(),
            response: &submission_response,
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
                    binding,
                    attempt: attempt.id,
                    // The idempotency fence records exactly what the browser
                    // sent. `submission_response` is the private
                    // issuance-envelope translation used only for grading.
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
                    binding,
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
        binding,
        record,
        SuccessorIssuance::Bound(binding),
    )
    .await
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
