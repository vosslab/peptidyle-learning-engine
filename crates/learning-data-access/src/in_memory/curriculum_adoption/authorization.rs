//! Active-session and current-course authority for curriculum adoption.

use question_model::{
    AssignmentId, CourseId, CourseReference, CourseScheduleRevision, CourseScheduleWitness,
    ObservedAssignmentRevision,
};

use super::destination;
use crate::in_memory::State;
use crate::{SessionTokenHash, StoreError, TenantContext, TenantId, UserId};

pub(crate) fn advance_course_schedule_revision(
    state: &mut State,
    tenant: TenantId,
    course: CourseId,
) -> Result<CourseScheduleRevision, StoreError> {
    let current = state
        .course_schedule_revisions
        .get_mut(&(tenant, course))
        .ok_or_else(|| destination::integrity("course schedule revision"))?;
    *current = current
        .checked_next()
        .ok_or_else(|| StoreError::Unavailable("course schedule revision exhausted".into()))?;
    Ok(*current)
}

pub(crate) fn authorized_actor(
    state: &State,
    context: TenantContext,
    session: SessionTokenHash,
) -> Result<UserId, StoreError> {
    super::super::reusable_curriculum::require_approved_instructor(state, context, session)
}

pub(crate) fn resolve_course(
    state: &State,
    tenant: TenantId,
    reference: CourseReference,
) -> Result<CourseId, StoreError> {
    state
        .courses_by_reference
        .get(&(tenant, reference))
        .copied()
        .ok_or(StoreError::NotFound)
}

pub(crate) fn require_course_instructor(
    state: &State,
    tenant: TenantId,
    course: CourseId,
    actor: UserId,
) -> Result<(), StoreError> {
    super::super::teaching_authority::require_direct_instructor(state, tenant, course, actor)
        .map(|_| ())
}

/// Re-reads the witness under the lock used by the later mutation. Stable
/// assignment identity establishes deterministic lock and write order.
pub(crate) fn course_witness(
    state: &State,
    tenant: TenantId,
    course: CourseId,
) -> Result<CourseScheduleWitness, StoreError> {
    let course_reference = *state
        .course_references
        .get(&(tenant, course))
        .ok_or_else(|| destination::integrity("course reference"))?;
    let schedule_revision = *state
        .course_schedule_revisions
        .get(&(tenant, course))
        .ok_or_else(|| destination::integrity("course schedule revision"))?;
    let assignments = super::course_assignment_ids(state, tenant, course)
        .into_iter()
        .map(|assignment| {
            Ok(ObservedAssignmentRevision {
                assignment: *state
                    .assignment_references
                    .get(&(tenant, assignment))
                    .ok_or_else(|| destination::integrity("assignment reference"))?,
                revision: *state
                    .assignment_revisions
                    .get(&(tenant, assignment))
                    .ok_or_else(|| destination::integrity("assignment revision"))?,
            })
        })
        .collect::<Result<Vec<_>, StoreError>>()?;
    CourseScheduleWitness::new(course_reference, schedule_revision, assignments)
        .map_err(|error| StoreError::InvalidRecord(error.to_string()))
}

pub(crate) fn require_exact_witness(
    state: &State,
    tenant: TenantId,
    expected: &CourseScheduleWitness,
) -> Result<CourseId, StoreError> {
    let course = resolve_course(state, tenant, expected.course)?;
    let current = course_witness(state, tenant, course)?;
    if &current != expected {
        return Err(StoreError::Conflict);
    }
    Ok(course)
}

pub(crate) fn course_has_any_run(state: &State, tenant: TenantId, course: CourseId) -> bool {
    state.runs.values().any(|run| {
        run.tenant == tenant
            && state
                .enrollments
                .get(&(tenant, run.enrollment))
                .and_then(|enrollment| state.assignments.get(&(tenant, enrollment.assignment)))
                .is_some_and(|assignment| assignment.course_id == course)
    })
}

pub(crate) fn assignment_has_run(
    state: &State,
    tenant: TenantId,
    assignment: AssignmentId,
) -> bool {
    state
        .assignments
        .get(&(tenant, assignment))
        .is_some_and(|record| super::super::course_policy::memory_assignment_has_run(state, record))
}
