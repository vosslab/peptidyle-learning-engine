//! Student-owned run reads. Each operation verifies current Student membership
//! before returning Student-scoped state, so a roster revocation takes effect
//! for reads as well as mutations.

use super::super::*;
use crate::{ActorContext, PrefetchedQuestionDescriptorV1};

pub(super) async fn assignment_run_items(
    store: &MemoryStore,
    _context: ActorContext,
    actor: UserId,
    run: RunId,
) -> Result<Option<Vec<AssignmentRunItem>>, StoreError> {
    let state = store.read_state()?;
    let Some(record) = state.runs.get(&run) else {
        return Ok(None);
    };
    let enrollment = enrollment_record(&state, record.enrollment)?;
    let assignment = assignment_record(&state, enrollment.assignment)?;
    require_course_records_accessible(&state, assignment.course_id)?;
    if super::super::entitlement::require_current_enrollment_entitlement(
        &state,
        actor,
        assignment.course_id,
        assignment.id,
        &enrollment,
    )
    .is_err()
    {
        return Ok(None);
    }
    Ok(Some(state.run_items.get(&run).cloned().unwrap_or_default()))
}

pub(super) async fn prefetched_question(
    store: &MemoryStore,
    _context: ActorContext,
    actor: UserId,
    run: RunId,
    predecessor: QuestionAttemptId,
    assignment_position: u32,
) -> Result<Option<PrefetchedQuestionDescriptorV1>, StoreError> {
    let state = store.read_state()?;
    let Some(record) = state.runs.get(&run) else {
        return Ok(None);
    };
    let enrollment = enrollment_record(&state, record.enrollment)?;
    let assignment = assignment_record(&state, enrollment.assignment)?;
    require_course_records_accessible(&state, assignment.course_id)?;
    if super::super::entitlement::require_current_enrollment_entitlement(
        &state,
        actor,
        assignment.course_id,
        assignment.id,
        &enrollment,
    )
    .is_err()
    {
        return Ok(None);
    }
    Ok(state
        .prefetched_questions
        .get(&(run, predecessor, assignment_position))
        .cloned())
}

pub(super) async fn pending_submission_for_run(
    store: &MemoryStore,
    _context: ActorContext,
    actor: UserId,
    run: RunId,
) -> Result<Option<QuestionAttemptId>, StoreError> {
    let state = store.read_state()?;
    let Some(record) = state.runs.get(&run) else {
        return Ok(None);
    };
    let enrollment = enrollment_record(&state, record.enrollment)?;
    let assignment = assignment_record(&state, enrollment.assignment)?;
    require_course_records_accessible(&state, assignment.course_id)?;
    if super::super::entitlement::require_current_enrollment_entitlement(
        &state,
        actor,
        assignment.course_id,
        assignment.id,
        &enrollment,
    )
    .is_err()
    {
        return Ok(None);
    }
    let pending: Vec<_> = state
        .attempts
        .values()
        .filter(|attempt| {
            attempt.run == run
                && state.submissions.contains_key(&attempt.id)
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

pub(super) async fn list_question_attempts(
    store: &MemoryStore,
    _context: ActorContext,
    actor: UserId,
    run: RunId,
    page: PageRequest,
) -> Result<Option<Page<QuestionAttempt>>, StoreError> {
    let state = store.read_state()?;
    let Some(record) = state.runs.get(&run) else {
        return Ok(None);
    };
    let enrollment = enrollment_record(&state, record.enrollment)?;
    let assignment = assignment_record(&state, enrollment.assignment)?;
    require_course_records_accessible(&state, assignment.course_id)?;
    if super::super::entitlement::require_current_enrollment_entitlement(
        &state,
        actor,
        assignment.course_id,
        assignment.id,
        &enrollment,
    )
    .is_err()
    {
        return Ok(None);
    }
    let records = state
        .attempts
        .iter()
        .filter(|(_, attempt)| attempt.run == run)
        .map(|(id, _attempt)| {
            let projected = projected_attempt(
                &state,
                state
                    .attempts
                    .get(id)
                    .expect("iterated attempt remains present"),
            );
            (
                format!(
                    "{:010}/{:020}/{}",
                    projected.assignment_position,
                    projected.timer.issued_at.as_unix_millis(),
                    id
                ),
                projected,
            )
        })
        .collect();
    Ok(Some(page_records(records, &page)))
}
