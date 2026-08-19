//! Run-submission capability; this module owns its route behavior.

use super::contracts::{RunBackend, RunSubmission, SubmissionDisposition};
use super::prefetch::{ensure_active_questions, load_run_question};
use super::queries::owned_run;
use super::support::*;
use question_model::presentation::{
    PresentationV1, RenderedItemIdV1, RenderedItemRoleV1, reproduce_presentation_v1,
};
use question_model::response::{ChoiceId, MatchPair, StudentResponse, TextEntryAnswer};

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
    let run = match owned_run(state.store.as_ref(), &authenticated, attempt.run).await {
        Ok(run) => run,
        Err(response) => return response,
    };
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
                false,
            )
            .await;
        }
        Ok(None) => {}
        Err(error) => return store_error_response(error),
    }
    if run.completed_at.is_some() {
        return error_response(StatusCode::CONFLICT, "run is already complete");
    }
    let reference = ProblemVersionRef {
        problem: attempt.problem,
        version: attempt.question_version,
    };
    // The store validates the immutable issue tuple. Presentation-bearing
    // attempts therefore return their exact answer-free schema here, while a
    // missing or corrupt required snapshot fails closed before any grade or
    // receipt mutation. Envelope-less families legitimately return None.
    let presentation = match state
        .store
        .get_attempt_presentation_snapshot(authenticated.tenant_context, actor, attempt.id)
        .await
    {
        Ok(presentation) => presentation,
        Err(error) => return store_error_response(error),
    };
    let grading_envelope = match state
        .store
        .get_attempt_grading_envelope(authenticated.tenant_context, actor, attempt.id)
        .await
    {
        Ok(grading_envelope) => grading_envelope,
        Err(error) => return store_error_response(error),
    };
    let flat_grading = match state
        .store
        .get_attempt_flat_grading(authenticated.tenant_context, actor, attempt.id)
        .await
    {
        Ok(flat_grading) => flat_grading,
        Err(error) => return store_error_response(error),
    };
    let webwork_grading = match state
        .store
        .get_attempt_webwork_grading(authenticated.tenant_context, actor, attempt.id)
        .await
    {
        Ok(webwork_grading) => webwork_grading,
        Err(error) => return store_error_response(error),
    };
    // Flat and WeBWorK attempts select their immutable first-grade contracts
    // here. WebWork then resolves only its attempt-bound source artifact and
    // replay state; neither family reloads a current catalog definition. The
    // remaining envelope-less families use their immutable catalog version.
    let question = match flat_grading.as_ref() {
        Some(contract) => contract.question().clone(),
        None => match webwork_grading.as_ref() {
            Some(contract) => contract.question().clone(),
            None => {
                match load_run_question(state.store.as_ref(), &authenticated, reference).await {
                    Ok(question) => question,
                    Err(response) => return response,
                }
            }
        },
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
    let (submission_response, issued_grading_envelope) = match (presentation, grading_envelope) {
        (Some(snapshot), Some(envelope)) => {
            let report = domain::validation::validate_presentation_response_format(
                &snapshot.envelope.response,
                &request.response,
            );
            if !report.is_valid() {
                return no_store((StatusCode::UNPROCESSABLE_ENTITY, Json(report)).into_response());
            }
            let binding = match state
                .store
                .get_attempt_presentation_binding(authenticated.tenant_context, actor, attempt.id)
                .await
            {
                Ok(Some(binding)) => binding,
                Ok(None) => {
                    return error_response(
                        StatusCode::SERVICE_UNAVAILABLE,
                        "issued presentation binding is unavailable",
                    );
                }
                Err(error) => return store_error_response(error),
            };
            let issued =
                match reproduce_presentation_v1(&envelope, &snapshot.asset_bindings, binding) {
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
    let disposition = match state
        .backend
        .submit(RunSubmission {
            context: authenticated.tenant_context,
            actor,
            idempotency_key: idempotency_key.clone(),
            reference,
            question: &question,
            attempt: &attempt,
            issued_grading_envelope: issued_grading_envelope.as_ref(),
            issued_flat_grading: flat_grading.as_ref(),
            issued_webwork_grading: webwork_grading.as_ref(),
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
        true,
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

/// Projects a historical receipt from the disclosure decision committed with
/// that receipt. It intentionally has no catalog lookup.
pub(super) fn apply_receipt_feedback_disclosure(
    disclosure: FeedbackDisclosure,
    run: &AssignmentRun,
    attempt: &mut QuestionAttempt,
) {
    let retain_legacy_result = match disclosure {
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
}

/// Projects a current attempt view from the current catalog policy. Historical
/// submission replay uses [`apply_receipt_feedback_disclosure`] instead.
pub(super) async fn apply_feedback_disclosure<S: CatalogStore>(
    store: &S,
    context: TenantContext,
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
    apply_receipt_feedback_disclosure(question.question.attempt_policy.feedback, run, attempt);
    Ok(())
}

pub(super) async fn finish_submission<S, B>(
    store: &S,
    backend: &B,
    authenticated: &AuthenticatedSession,
    record: SubmissionRecord,
    may_issue_successor: bool,
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
        Err(_) => return submission_response(record, None, true),
    };
    let next_state = if matches!(
        next_state,
        learning_data_access::SubmissionNextAttempt::Pending
    ) && may_issue_successor
    {
        // A receipt and grade are already durable. Try to issue its successor
        // before the normal first response, but never turn a post-grade
        // delivery failure into a failed submission or a second grade.
        if ensure_active_questions(
            store,
            backend,
            authenticated,
            &record.run,
            Some(record.attempt.id),
        )
        .await
        .is_err()
        {
            return submission_response(record, None, true);
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
                    return submission_response(record, None, true);
                }
                learning_data_access::SubmissionNextAttempt::None
            }
            Ok(value) => value,
            Err(_) => return submission_response(record, None, true),
        }
    } else if matches!(
        next_state,
        learning_data_access::SubmissionNextAttempt::Pending
    ) {
        return submission_response(record, None, true);
    } else {
        next_state
    };
    let next_pending = false;
    let next_issued = match next_state {
        learning_data_access::SubmissionNextAttempt::None => None,
        learning_data_access::SubmissionNextAttempt::Issued(next) => Some(next_issued(next)),
        learning_data_access::SubmissionNextAttempt::Pending => None,
    };
    submission_response(record, next_issued, next_pending)
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
) -> Response {
    let feedback = feedback_projection(
        record.feedback_disclosure,
        &record.run,
        &record.attempt,
        record.feedback.content(),
    );
    let mut attempt = record.attempt;
    apply_receipt_feedback_disclosure(record.feedback_disclosure, &record.run, &mut attempt);
    no_store(
        Json(SubmissionReceipt {
            accepted: true,
            attempt,
            feedback,
            next_issued,
            next_pending,
        })
        .into_response(),
    )
}

pub(super) fn feedback_projection(
    disclosure: FeedbackDisclosure,
    run: &AssignmentRun,
    attempt: &QuestionAttempt,
    content: &FeedbackContent,
) -> Option<DisclosedFeedback> {
    // A submission receipt is a historical result from the grade transition.
    // An instructor can create an OnRelease record only after that transition,
    // so this initial projection is unreleased. Replayed receipts retain this
    // immutable state; the current run-summary projection receives the stored
    // release fact above.
    project_run_feedback(disclosure, run, false, attempt.result, content)
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
