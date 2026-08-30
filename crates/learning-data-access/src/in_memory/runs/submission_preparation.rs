//! Memory parity for one coherent pre-grade authorization snapshot.

use domain::entitlement::EntitlementDecision;
use question_model::{AttemptStatus, QuestionAttemptId, StudentResponse, UserId};

use super::super::{
    State, assignment_record, enrollment_record, projected_attempt,
    require_course_records_accessible,
};
use super::issued_contracts::{
    load_issued_presentation, load_submission_record, validate_issued_question_snapshot,
};
use crate::{
    ActorContext, AuthorizedSubmissionIntent, StoreError, StudentWorkRoutingBinding,
    SubmissionIdempotencyKey, SubmissionPreparation, SubmissionReceiptRead,
};

pub(in crate::in_memory) fn prepare_question_submission(
    state: &State,
    context: ActorContext,
    actor: UserId,
    binding: StudentWorkRoutingBinding,
    attempt_id: QuestionAttemptId,
    response: &StudentResponse,
    idempotency_key: &SubmissionIdempotencyKey,
) -> Result<SubmissionPreparation, StoreError> {
    if context.user_id() != actor {
        return Err(StoreError::NotFound);
    }
    // ASVS 8.2.2: establish current Student authority for the route before
    // resolving an opaque attempt identity.
    super::super::entitlement::active_membership_for(state, binding.course, actor)
        .filter(|membership| {
            membership.role == question_model::CourseMembershipRole::Student
                && membership.student.is_some()
        })
        .ok_or(StoreError::NotFound)?;
    let assignment = assignment_record(state, binding.assignment)?;
    if assignment.course_id != binding.course {
        return Err(StoreError::NotFound);
    }
    require_course_records_accessible(state, binding.course)?;
    let EntitlementDecision::Granted(grant) = super::super::entitlement::evaluate_locked(
        state,
        actor,
        binding.course,
        binding.assignment,
    )?
    else {
        return Err(StoreError::NotFound);
    };
    let base = state
        .attempts
        .get(&attempt_id)
        .cloned()
        .ok_or(StoreError::NotFound)?;
    let attempt = projected_attempt(state, &base);
    let run = state
        .runs
        .get(&attempt.run)
        .cloned()
        .ok_or(StoreError::NotFound)?;
    let enrollment = enrollment_record(state, run.enrollment)?;
    if enrollment.assignment != binding.assignment
        || enrollment.user != actor
        || enrollment.student != grant.student()
    {
        return Err(StoreError::NotFound);
    }
    let run_item = state
        .run_items
        .get(&run.id)
        .and_then(|items| {
            items
                .iter()
                .find(|item| item.issued_position == attempt.assignment_position)
        })
        .ok_or_else(|| StoreError::Unavailable("prepared run item is missing".to_string()))?;
    if run_item.reference.problem != attempt.problem
        || run_item.reference.version != attempt.question_version
    {
        return Err(StoreError::Unavailable(
            "prepared run item disagrees with attempt".to_string(),
        ));
    }
    let summary = state
        .summaries
        .get(&enrollment.id)
        .ok_or(StoreError::NotFound)?;
    if summary.enrollment != enrollment.id {
        return Err(StoreError::Unavailable(
            "prepared summary disagrees with enrollment".to_string(),
        ));
    }
    if let Some(stored) = state.submissions.get(&attempt_id) {
        if stored.key != *idempotency_key
            || !super::super::stored_submission_matches_response(state, attempt_id, response)?
        {
            return Err(StoreError::Conflict);
        }
        return match load_submission_record(state, &attempt)? {
            SubmissionReceiptRead::Missing => Err(StoreError::Unavailable(
                "submission receipt disappeared during replay".to_string(),
            )),
            SubmissionReceiptRead::AcceptedPending(pending) => {
                Ok(SubmissionPreparation::AcceptedPending(pending))
            }
            SubmissionReceiptRead::Completed(record) => Ok(SubmissionPreparation::Replay(record)),
        };
    }
    if attempt.status != AttemptStatus::InProgress
        || run.completed_at.is_some()
        || run.score.is_some()
    {
        return Err(StoreError::Conflict);
    }
    // The replay branch above intentionally needs only current disclosure and
    // its durable receipt.  A first effect alone may hydrate the immutable
    // issued snapshot; current catalog visibility is never a grading input.
    let issued_question_snapshot = state
        .attempt_issued_question_snapshots
        .get(&attempt.id)
        .cloned()
        .ok_or_else(|| {
            StoreError::Unavailable("issued question snapshot is missing".to_string())
        })?;
    let flat_capability = state
        .attempt_flat_grading_capabilities
        .get(&attempt.id)
        .copied()
        .ok_or_else(|| {
            StoreError::Unavailable("attempt flat grading capability is missing".to_string())
        })?;
    let webwork_capability = state
        .attempt_webwork_grading_capabilities
        .get(&attempt.id)
        .copied()
        .ok_or_else(|| {
            StoreError::Unavailable("attempt WeBWorK grading capability is missing".to_string())
        })?;
    let qti_capability = state
        .attempt_qti_grading_capabilities
        .get(&attempt.id)
        .copied()
        .ok_or_else(|| {
            StoreError::Unavailable("attempt QTI grading capability is missing".to_string())
        })?;
    validate_issued_question_snapshot(
        &issued_question_snapshot,
        &attempt,
        flat_capability,
        webwork_capability,
        qti_capability,
        state.attempt_presentation_snapshots.get(&attempt.id),
    )?;
    let presentation = load_issued_presentation(state, &attempt)?;
    let presentation_binding = state.attempt_presentations.get(&attempt.id).copied();
    let grading_envelope = state.attempt_grading_envelopes.get(&attempt.id).cloned();
    Ok(SubmissionPreparation::FirstEffect(Box::new(
        AuthorizedSubmissionIntent {
            attempt,
            issued_question_snapshot,
            presentation_binding,
            presentation,
            grading_envelope,
        },
    )))
}
