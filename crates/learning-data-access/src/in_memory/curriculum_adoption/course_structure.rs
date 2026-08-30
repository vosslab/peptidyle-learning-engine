//! Blueprint-backed CourseInstance topology and deterministic rollover traversal.

use question_model::curriculum_adoption::CurriculumSemanticAssignment;
use question_model::{
    AssignmentDefinitionSourceView, AssignmentId, CourseId, CourseInstanceBlueprintApplication,
    ResolvedRelativeAssignmentSchedule,
};

use super::destination;
use crate::StoreError;
use crate::curriculum_adoption::SemanticPlannerError;
use crate::in_memory::State;

#[derive(Debug, Clone)]
pub(crate) struct RolloverAssignmentInput {
    pub(crate) semantic: CurriculumSemanticAssignment,
    pub(crate) source: AssignmentDefinitionSourceView,
    pub(crate) schedule: ResolvedRelativeAssignmentSchedule,
}

/// A structural course snapshot. Original reusable module topology survives
/// rollover; ordinary courses use the deterministic flat teaching projection.
#[derive(Debug, Clone)]
pub(crate) struct RolloverInput {
    pub(crate) title: String,
    pub(crate) blueprint_application: CourseInstanceBlueprintApplication,
    assignments: Vec<RolloverAssignmentInput>,
}

pub(crate) fn course_assignment_ids(state: &State, course: CourseId) -> Vec<AssignmentId> {
    let mut rows = state
        .assignments
        .iter()
        .filter_map(|(assignment, record)| (record.course_id == course).then_some(*assignment))
        .map(|assignment| {
            (
                state.assignment_references.get(&assignment).copied(),
                assignment,
            )
        })
        .collect::<Vec<_>>();
    rows.sort_unstable_by_key(|(reference, assignment)| (*reference, *assignment));
    rows.into_iter().map(|(_, assignment)| assignment).collect()
}

/// Stable presentation/semantic order for course-tree operations. Public
/// assignment references encode durable creation order; random internal IDs do
/// not carry authored meaning.
pub(crate) fn course_assignment_ids_checked(
    state: &State,
    course: CourseId,
) -> Result<Vec<AssignmentId>, StoreError> {
    let mut rows = course_assignment_ids(state, course)
        .into_iter()
        .map(|assignment| {
            state
                .assignment_references
                .get(&assignment)
                .copied()
                .map(|reference| (reference, assignment))
                .ok_or_else(|| destination::integrity("course assignment reference"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    rows.sort_unstable_by_key(|(reference, _)| *reference);
    Ok(rows.into_iter().map(|(_, assignment)| assignment).collect())
}

pub(crate) fn rollover_input(
    state: &State,
    course: CourseId,
    target_term: &question_model::CourseTerm,
) -> Result<RolloverInput, StoreError> {
    let title = state
        .courses
        .get(&course)
        .ok_or(StoreError::NotFound)?
        .title
        .clone();
    let blueprint_application =
        super::authorization::course_instance_blueprint_application(state, course)?;
    let assignments = course_assignment_ids_checked(state, course)?
        .into_iter()
        .map(|assignment| {
            let import = state
                .curriculum_adoption
                .import_records
                .get(&assignment)
                .ok_or(StoreError::Conflict)?;
            let evidence = state
                .curriculum_adoption
                .assignment_evidence
                .get(&(assignment, import.import_revision))
                .ok_or(StoreError::Conflict)?;
            let semantic = current_with_projected_teaching_schedule(state, assignment)?;
            let (schedule, corrections) =
                crate::curriculum_adoption::preview_assignment(&semantic, target_term)
                    .map_err(semantic_error)?;
            if !corrections.is_empty() {
                return Err(StoreError::Conflict);
            }
            Ok(RolloverAssignmentInput {
                semantic,
                source: evidence.source,
                schedule,
            })
        })
        .collect::<Result<Vec<_>, StoreError>>()?;
    Ok(RolloverInput {
        title,
        blueprint_application,
        assignments,
    })
}

impl RolloverInput {
    pub(crate) fn assignments(&self) -> &[RolloverAssignmentInput] {
        &self.assignments
    }
}

fn semantic_error(error: SemanticPlannerError) -> StoreError {
    StoreError::InvalidRecord(error.to_string())
}

pub(crate) fn current_with_projected_teaching_schedule(
    state: &State,
    assignment: AssignmentId,
) -> Result<CurriculumSemanticAssignment, StoreError> {
    let record = state
        .assignments
        .get(&assignment)
        .ok_or(StoreError::NotFound)?;
    let policy = state
        .assignment_base_policy
        .get(&assignment)
        .ok_or_else(|| destination::integrity("assignment base policy"))?;
    let term = &state
        .courses
        .get(&record.course_id)
        .ok_or(StoreError::NotFound)?
        .term;
    let schedule =
        question_model::RelativeAssignmentSchedule::from_base_policy(&policy.policy, term)
            .map_err(|error| StoreError::InvalidRecord(error.to_string()))?;
    destination::current_semantic_assignment(state, assignment, schedule)
}
