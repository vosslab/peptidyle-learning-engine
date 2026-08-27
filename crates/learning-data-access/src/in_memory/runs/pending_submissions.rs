//! Learner-visible successor-finalization state for one run.

use question_model::{QuestionAttemptId, RunId, UserId};

use super::super::{
    MemoryStore, assignment_record, enrollment_record, require_course_records_accessible,
};
use crate::{StoreError, TenantContext};

pub(super) async fn pending_submission_for_run(
    store: &MemoryStore,
    context: TenantContext,
    actor: UserId,
    run: RunId,
) -> Result<Option<QuestionAttemptId>, StoreError> {
    let state = store.read_state()?;
    let tenant = context.tenant_id();
    let run_record = state.runs.get(&(tenant, run)).ok_or(StoreError::NotFound)?;
    let enrollment = enrollment_record(&state, tenant, run_record.enrollment)?;
    let assignment = assignment_record(&state, tenant, enrollment.assignment)?;
    require_course_records_accessible(&state, tenant, assignment.course_id)?;
    super::super::entitlement::require_current_enrollment_entitlement(
        &state,
        tenant,
        actor,
        assignment.course_id,
        assignment.id,
        &enrollment,
    )?;
    let pending: Vec<_> = state
        .attempts
        .values()
        .filter(|attempt| {
            attempt.tenant == tenant
                && attempt.run == run
                && state
                    .submissions
                    .get(&(tenant, attempt.id))
                    .is_some_and(|submission| submission.completed_record_opt().is_some())
                && !state
                    .submission_next_attempts
                    .contains_key(&(tenant, attempt.id))
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
