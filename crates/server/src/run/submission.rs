//! Run-submission capability; this module owns its route behavior.

use super::contracts::{RunBackend, RunSubmission, SubmissionDisposition};
use super::prefetch::ensure_active_questions;
use super::support::*;
use question_model::presentation::{
    PresentationV1, RenderedItemIdV1, RenderedItemRoleV1, reproduce_presentation_v1,
};
use question_model::response::{ChoiceId, MatchPair, StudentResponse, TextEntryAnswer};

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
    let prepared = match state
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
                *record,
                SuccessorIssuance::Bound(binding),
            )
            .await;
        }
        Ok(learning_data_access::SubmissionPreparation::Grade(prepared)) => *prepared,
        Err(error) => return store_error_response(error),
    };
    let attempt = prepared.attempt;
    let reference = ProblemVersionRef {
        problem: attempt.problem,
        version: attempt.question_version,
    };
    // The store validates the immutable issue tuple. Presentation-bearing
    // attempts therefore return their exact answer-free schema here, while a
    // missing or corrupt required snapshot fails closed before any grade or
    // receipt mutation. Envelope-less families legitimately return None.
    let presentation = prepared.presentation;
    let grading_envelope = prepared.grading_envelope;
    let flat_grading = prepared.flat_grading;
    let webwork_grading = prepared.webwork_grading;
    let issued_qti_grading = prepared.issued_qti_grading;
    let webwork_replay = prepared.webwork_replay;
    let presentation_binding = prepared.presentation_binding;
    // The exact published question belongs to the same broker-retained
    // preparation snapshot as the attempt and private grading contracts.
    let issued_question_snapshot = prepared.issued_question_snapshot;
    let question = issued_question_snapshot.question();
    if matches!(
        &question.response,
        question_model::ResponseDefinition::FileUpload { .. }
    ) {
        return error_response(
            StatusCode::UNPROCESSABLE_ENTITY,
            "file upload submissions are unavailable",
        );
    }
    let (submission_response, issued_grading_envelope) =
        match (presentation.as_ref(), grading_envelope.as_ref()) {
            (Some(snapshot), Some(envelope)) => {
                let report = domain::validation::validate_presentation_response_format(
                    &snapshot.envelope.response,
                    &request.response,
                );
                if !report.is_valid() {
                    return no_store(
                        (StatusCode::UNPROCESSABLE_ENTITY, Json(report)).into_response(),
                    );
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
                let translated = match translate_issued_response(&request.response, &issued) {
                    Ok(response) => response,
                    Err(()) => {
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
                let report = domain::validation::validate_response_format(
                    &question.response,
                    &request.response,
                );
                if !report.is_valid() {
                    return no_store(
                        (StatusCode::UNPROCESSABLE_ENTITY, Json(report)).into_response(),
                    );
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
    let disposition = match state
        .backend
        .submit(RunSubmission {
            context: authenticated.tenant_context,
            actor,
            idempotency_key: idempotency_key.clone(),
            reference,
            issued_question_snapshot: &issued_question_snapshot,
            attempt: &attempt,
            issued_grading_envelope,
            issued_flat_grading: flat_grading.as_ref(),
            issued_webwork_grading: webwork_grading.as_ref(),
            issued_qti_grading: issued_qti_grading.as_ref(),
            issued_webwork_replay: webwork_replay.as_ref(),
            issued_presentation_binding: presentation_binding,
            issued_presentation: presentation.as_ref(),
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
        record,
        SuccessorIssuance::Bound(binding),
    )
    .await
}

/// Translates browser-visible presentation IDs back to the durable IDs held
/// only in the protected issuance envelope. This happens after public-format
/// validation and before private grading; a replay never reaches this seam.
fn translate_issued_response(
    response: &StudentResponse,
    presentation: &PresentationV1,
) -> Result<StudentResponse, ()> {
    let durable_id = |id: &ChoiceId, role| {
        let rendered = RenderedItemIdV1::parse(id.as_str()).map_err(|_| ())?;
        let mut bindings = presentation
            .item_bindings
            .iter()
            .filter(|binding| binding.role == role && binding.rendered == rendered);
        let binding = bindings.next().ok_or(())?;
        if bindings.next().is_some() {
            return Err(());
        }
        Ok(ChoiceId::new(binding.durable_id.clone()))
    };
    match response {
        StudentResponse::MultipleChoice { selected } => Ok(StudentResponse::MultipleChoice {
            selected: selected
                .iter()
                .map(|id| durable_id(id, RenderedItemRoleV1::Choice))
                .collect::<Result<_, _>>()?,
        }),
        StudentResponse::MultiBlank { answers } => Ok(StudentResponse::MultiBlank {
            answers: answers
                .iter()
                .map(|answer| {
                    Ok::<TextEntryAnswer, ()>(TextEntryAnswer {
                        slot: durable_id(&answer.slot, RenderedItemRoleV1::Blank)?,
                        text: answer.text.clone(),
                    })
                })
                .collect::<Result<_, _>>()?,
        }),
        StudentResponse::Matching { matches } => Ok(StudentResponse::Matching {
            matches: matches
                .iter()
                .map(|pair| {
                    Ok::<MatchPair, ()>(MatchPair {
                        prompt: durable_id(&pair.prompt, RenderedItemRoleV1::MatchPrompt)?,
                        choice: durable_id(&pair.choice, RenderedItemRoleV1::MatchChoice)?,
                    })
                })
                .collect::<Result<_, _>>()?,
        }),
        StudentResponse::Ordering { order } => Ok(StudentResponse::Ordering {
            order: order
                .iter()
                .map(|id| durable_id(id, RenderedItemRoleV1::OrderItem))
                .collect::<Result<_, _>>()?,
        }),
        StudentResponse::Numeric { value } => Ok(StudentResponse::Numeric { value: *value }),
        StudentResponse::ShortText { text } => {
            Ok(StudentResponse::ShortText { text: text.clone() })
        }
        StudentResponse::Hotspot { points } => Ok(StudentResponse::Hotspot {
            points: points.clone(),
        }),
        StudentResponse::FileUpload { object_key } => Ok(StudentResponse::FileUpload {
            object_key: object_key.clone(),
        }),
        StudentResponse::ExternalTool {} => Ok(StudentResponse::ExternalTool {}),
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
        .submission_next_attempt(authenticated.tenant_context, actor, record.attempt.id)
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
            .submission_next_attempt(authenticated.tenant_context, actor, record.attempt.id)
            .await
        {
            Ok(learning_data_access::SubmissionNextAttempt::Pending) => {
                if store
                    .finalize_submission_next_attempt(
                        authenticated.tenant_context,
                        actor,
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
