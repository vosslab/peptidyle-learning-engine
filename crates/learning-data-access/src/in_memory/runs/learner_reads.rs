//! Learner-owned run reads. Each operation verifies current student membership
//! before returning learner-scoped state, so a roster revocation takes effect
//! for reads as well as mutations.

use super::super::*;

pub(super) async fn assignment_run_items(
    store: &MemoryStore,
    context: TenantContext,
    actor: UserId,
    run: RunId,
) -> Result<Option<Vec<AssignmentRunItem>>, StoreError> {
    let state = store.read_state()?;
    let Some(record) = state.runs.get(&(context.tenant_id(), run)) else {
        return Ok(None);
    };
    let enrollment = enrollment_record(&state, context.tenant_id(), record.enrollment)?;
    let assignment = assignment_record(&state, context.tenant_id(), enrollment.assignment)?;
    require_course_records_accessible(&state, context.tenant_id(), assignment.course_id)?;
    if super::super::entitlement::require_current_enrollment_entitlement(
        &state,
        context.tenant_id(),
        actor,
        assignment.course_id,
        assignment.id,
        &enrollment,
    )
    .is_err()
    {
        return Ok(None);
    }
    Ok(Some(
        state
            .run_items
            .get(&(context.tenant_id(), run))
            .cloned()
            .unwrap_or_default(),
    ))
}

pub(super) async fn prefetched_question(
    store: &MemoryStore,
    context: TenantContext,
    actor: UserId,
    run: RunId,
    predecessor: QuestionAttemptId,
    assignment_position: u32,
) -> Result<Option<PrefetchedQuestion>, StoreError> {
    let state = store.read_state()?;
    let Some(record) = state.runs.get(&(context.tenant_id(), run)) else {
        return Ok(None);
    };
    let enrollment = enrollment_record(&state, context.tenant_id(), record.enrollment)?;
    let assignment = assignment_record(&state, context.tenant_id(), enrollment.assignment)?;
    require_course_records_accessible(&state, context.tenant_id(), assignment.course_id)?;
    if super::super::entitlement::require_current_enrollment_entitlement(
        &state,
        context.tenant_id(),
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
        .get(&(context.tenant_id(), run, predecessor, assignment_position))
        .cloned())
}

pub(super) async fn pending_submission_for_run(
    store: &MemoryStore,
    context: TenantContext,
    actor: UserId,
    run: RunId,
) -> Result<Option<QuestionAttemptId>, StoreError> {
    let state = store.read_state()?;
    let Some(record) = state.runs.get(&(context.tenant_id(), run)) else {
        return Ok(None);
    };
    let enrollment = enrollment_record(&state, context.tenant_id(), record.enrollment)?;
    let assignment = assignment_record(&state, context.tenant_id(), enrollment.assignment)?;
    require_course_records_accessible(&state, context.tenant_id(), assignment.course_id)?;
    if super::super::entitlement::require_current_enrollment_entitlement(
        &state,
        context.tenant_id(),
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
            attempt.tenant == context.tenant_id()
                && attempt.run == run
                && state
                    .submissions
                    .contains_key(&(context.tenant_id(), attempt.id))
                && !state
                    .submission_next_attempts
                    .contains_key(&(context.tenant_id(), attempt.id))
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
    context: TenantContext,
    actor: UserId,
    run: RunId,
    page: PageRequest,
) -> Result<Option<Page<QuestionAttempt>>, StoreError> {
    let state = store.read_state()?;
    let Some(record) = state.runs.get(&(context.tenant_id(), run)) else {
        return Ok(None);
    };
    let enrollment = enrollment_record(&state, context.tenant_id(), record.enrollment)?;
    let assignment = assignment_record(&state, context.tenant_id(), enrollment.assignment)?;
    require_course_records_accessible(&state, context.tenant_id(), assignment.course_id)?;
    if super::super::entitlement::require_current_enrollment_entitlement(
        &state,
        context.tenant_id(),
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
        .filter(|((tenant, _), attempt)| *tenant == context.tenant_id() && attempt.run == run)
        .map(|((_, id), _attempt)| {
            let projected = projected_attempt(
                &state,
                context.tenant_id(),
                state
                    .attempts
                    .get(&(context.tenant_id(), *id))
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
