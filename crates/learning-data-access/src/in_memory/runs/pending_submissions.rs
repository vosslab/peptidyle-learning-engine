//! Learner-visible successor-finalization state for one run.

use question_model::{QuestionAttemptId, RunId, UserId};

use super::super::{
    MemoryStore, assignment_record, enrollment_record, require_course_records_accessible,
};
use crate::{ActorContext, StoreError};

pub(super) async fn pending_submission_for_run(
    store: &MemoryStore,
    context: ActorContext,
    actor: UserId,
    run: RunId,
) -> Result<Option<QuestionAttemptId>, StoreError> {
    let state = store.read_state()?;
    if context.user_id() != actor {
        return Err(StoreError::NotFound);
    }
    let run_record = state.runs.get(&run).ok_or(StoreError::NotFound)?;
    let enrollment = enrollment_record(&state, run_record.enrollment)?;
    let assignment = assignment_record(&state, enrollment.assignment)?;
    require_course_records_accessible(&state, assignment.course_id)?;
    super::super::entitlement::require_current_enrollment_entitlement(
        &state,
        actor,
        assignment.course_id,
        assignment.id,
        &enrollment,
    )?;
    let pending: Vec<_> = state
        .attempts
        .values()
        .filter(|attempt| {
            attempt.run == run
                && state
                    .submissions
                    .get(&attempt.id)
                    .is_some_and(|submission| submission.completed_record_opt().is_some())
                && !state.submission_next_attempts.contains_key(&attempt.id)
        })
        .map(|attempt| attempt.id)
        .take(2)
        .collect();
    match pending.as_slice() {
        [] => Ok(None),
        [id] => Ok(Some(*id)),
        _ => Err(StoreError::Conflict),
    }
}
