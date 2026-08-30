//! Active-session and current-course authority for curriculum adoption.

use question_model::{
    AssignmentId, CourseId, CourseInstanceBlueprintApplication, CourseInstanceWitness,
    CourseReference, CourseScheduleRevision, ObservedCourseInstanceAssignment,
};

use super::destination;
use crate::in_memory::State;
use crate::{ActorContext, SessionTokenHash, StoreError, UserId};

pub(crate) fn advance_course_schedule_revision(
    state: &mut State,
    course: CourseId,
) -> Result<CourseScheduleRevision, StoreError> {
    let current = state
        .course_schedule_revisions
        .get_mut(&course)
        .ok_or_else(|| destination::integrity("course schedule revision"))?;
    *current = current
        .checked_next()
        .ok_or_else(|| StoreError::Unavailable("course schedule revision exhausted".into()))?;
    Ok(*current)
}

pub(crate) fn authorized_actor(
    state: &State,
    context: ActorContext,
    session: SessionTokenHash,
) -> Result<UserId, StoreError> {
    super::super::reusable_curriculum::require_approved_instructor(state, context, session)
}

pub(crate) fn resolve_course(
    state: &State,
    reference: CourseReference,
) -> Result<CourseId, StoreError> {
    state
        .courses_by_reference
        .get(&reference)
        .copied()
        .ok_or(StoreError::NotFound)
}

pub(crate) fn require_course_instructor(
    state: &State,
    course: CourseId,
    actor: UserId,
) -> Result<(), StoreError> {
    super::super::teaching_authority::require_direct_instructor(state, course, actor).map(|_| ())
}

/// Re-reads the witness under the lock used by the later mutation. Stable
/// assignment identity establishes deterministic lock and write order.
pub(crate) fn course_witness(
    state: &State,
    course: CourseId,
) -> Result<CourseInstanceWitness, StoreError> {
    let course_reference = *state
        .course_references
        .get(&course)
        .ok_or_else(|| destination::integrity("course reference"))?;
    let schedule_revision = *state
        .course_schedule_revisions
        .get(&course)
        .ok_or_else(|| destination::integrity("course schedule revision"))?;
    let assignments = super::course_assignment_ids(state, course)
        .into_iter()
        .map(|assignment| {
            Ok(ObservedCourseInstanceAssignment {
                assignment: *state
                    .assignment_references
                    .get(&assignment)
                    .ok_or_else(|| destination::integrity("assignment reference"))?,
                revision: *state
                    .assignment_revisions
                    .get(&assignment)
                    .ok_or_else(|| destination::integrity("assignment revision"))?,
            })
        })
        .collect::<Result<Vec<_>, StoreError>>()?;
    CourseInstanceWitness::new(course_reference, schedule_revision, assignments)
        .map_err(|error| StoreError::InvalidRecord(error.to_string()))
}

pub(crate) fn require_exact_witness(
    state: &State,
    expected: &CourseInstanceWitness,
) -> Result<CourseId, StoreError> {
    let course = resolve_course(state, expected.course)?;
    let current = course_witness(state, course)?;
    if &current != expected {
        return Err(StoreError::Conflict);
    }
    Ok(course)
}

/// Resolves the one immutable Blueprint application for an existing CourseInstance.
///
/// ASVS 2.3.1/2.3.3: callers use this canonical binding inside their locked
/// transition so parentage cannot be inferred from mutable import projections.
pub(crate) fn course_instance_blueprint_application(
    state: &State,
    course: CourseId,
) -> Result<CourseInstanceBlueprintApplication, StoreError> {
    let application = *state
        .curriculum_adoption
        .course_instance_blueprint_applications
        .get(&course)
        .ok_or_else(|| destination::integrity("CourseInstance Blueprint application"))?;
    if !state.courses.contains_key(&course) {
        return Err(destination::integrity(
            "CourseInstance Blueprint application course",
        ));
    }
    Ok(application)
}

pub(crate) fn course_has_any_run(state: &State, course: CourseId) -> bool {
    state.runs.values().any(|run| {
        state
            .enrollments
            .get(&run.enrollment)
            .and_then(|enrollment| state.assignments.get(&enrollment.assignment))
            .is_some_and(|assignment| assignment.course_id == course)
    })
}

pub(crate) fn assignment_has_run(state: &State, assignment: AssignmentId) -> bool {
    state
        .assignments
        .get(&assignment)
        .is_some_and(|record| super::super::course_policy::memory_assignment_has_run(state, record))
}
