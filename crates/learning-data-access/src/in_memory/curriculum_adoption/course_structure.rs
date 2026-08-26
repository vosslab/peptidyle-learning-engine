//! Course-tree preservation and deterministic rollover traversal.

use question_model::curriculum_adoption::{
    CurriculumSemanticAssignment, CurriculumSemanticPayload,
};
use question_model::{AssignmentId, CourseId, ObservedAssignmentRevision, TenantId};
use std::collections::BTreeSet;

use super::{StoredWholeCourseAdoption, destination};
use crate::StoreError;
use crate::curriculum_adoption::{RolloverInput as SemanticRolloverInput, SemanticPlannerError};
use crate::in_memory::State;

#[derive(Debug, Clone, Copy)]
pub(crate) struct RolloverAssignmentInput<'a> {
    pub(super) semantic: &'a CurriculumSemanticAssignment,
    pub(super) source: ObservedAssignmentRevision,
}

/// A structural course snapshot. Original reusable module topology survives
/// rollover; ordinary courses use the deterministic flat teaching projection.
#[derive(Debug, Clone)]
pub(crate) struct RolloverInput {
    topology: SemanticRolloverInput,
    sources: Vec<ObservedAssignmentRevision>,
}

pub(crate) fn course_assignment_ids(
    state: &State,
    tenant: TenantId,
    course: CourseId,
) -> Vec<AssignmentId> {
    let mut ids = state
        .assignments
        .iter()
        .filter_map(|((record_tenant, assignment), record)| {
            (*record_tenant == tenant && record.course_id == course).then_some(*assignment)
        })
        .collect::<Vec<_>>();
    ids.sort_unstable();
    ids
}

/// Stable presentation/semantic order for course-tree operations. Public
/// assignment references encode durable creation order; random internal IDs do
/// not carry authored meaning. Lock-oriented operations retain `course_assignment_ids`.
fn course_assignment_ids_in_semantic_order(
    state: &State,
    tenant: TenantId,
    course: CourseId,
) -> Result<Vec<AssignmentId>, StoreError> {
    let mut rows = course_assignment_ids(state, tenant, course)
        .into_iter()
        .map(|assignment| {
            state
                .assignment_references
                .get(&(tenant, assignment))
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
    tenant: TenantId,
    course: CourseId,
) -> Result<RolloverInput, StoreError> {
    let title = state
        .courses
        .get(&(tenant, course))
        .ok_or(StoreError::NotFound)?
        .title
        .clone();
    match state
        .curriculum_adoption
        .whole_course_adoptions
        .get(&(tenant, course))
    {
        Some(adoption) => adopted_rollover_input(state, tenant, course, title, adoption),
        None => ordinary_rollover_input(state, tenant, course, title),
    }
}

fn adopted_rollover_input(
    state: &State,
    tenant: TenantId,
    course: CourseId,
    title: String,
    adoption: &StoredWholeCourseAdoption,
) -> Result<RolloverInput, StoreError> {
    let expected = adoption
        .payload
        .modules()
        .iter()
        .map(|module| module.assignments().len())
        .sum::<usize>();
    if expected != adoption.destination_assignments.len() {
        return Err(destination::integrity("whole-course semantic positions"));
    }
    let mut ids = adoption.destination_assignments.iter();
    let mut modules = Vec::with_capacity(adoption.payload.modules().len());
    let mut sources = Vec::with_capacity(expected);
    for module in adoption.payload.modules() {
        let mut assignments = Vec::with_capacity(module.assignments().len());
        for _ in module.assignments() {
            let (semantic, source) = rollover_assignment(
                state,
                tenant,
                course,
                *ids.next().expect("checked semantic positions"),
            )?;
            assignments.push(semantic);
            sources.push(source);
        }
        modules.push((module.label().to_owned(), assignments));
    }
    let original = adoption
        .destination_assignments
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    let later = course_assignment_ids_in_semantic_order(state, tenant, course)?
        .into_iter()
        .filter(|assignment| !original.contains(assignment))
        .map(|assignment| rollover_assignment(state, tenant, course, assignment))
        .collect::<Result<Vec<_>, _>>()?;
    if !later.is_empty() {
        sources.extend(later.iter().map(|(_, source)| *source));
        modules.push((
            "Later course assignments".into(),
            later.into_iter().map(|(semantic, _)| semantic).collect(),
        ));
    }
    let topology = SemanticRolloverInput::new(title, modules).map_err(semantic_error)?;
    Ok(RolloverInput { topology, sources })
}

fn ordinary_rollover_input(
    state: &State,
    tenant: TenantId,
    course: CourseId,
    title: String,
) -> Result<RolloverInput, StoreError> {
    let assignments = course_assignment_ids_in_semantic_order(state, tenant, course)?
        .into_iter()
        .map(|assignment| rollover_assignment(state, tenant, course, assignment))
        .collect::<Result<Vec<_>, _>>()?;
    let sources = assignments.iter().map(|(_, source)| *source).collect();
    let modules = vec![(
        "Assignments".into(),
        assignments
            .into_iter()
            .map(|(semantic, _)| semantic)
            .collect(),
    )];
    let topology = SemanticRolloverInput::new(title, modules).map_err(semantic_error)?;
    Ok(RolloverInput { topology, sources })
}

fn rollover_assignment(
    state: &State,
    tenant: TenantId,
    course: CourseId,
    assignment: AssignmentId,
) -> Result<(CurriculumSemanticAssignment, ObservedAssignmentRevision), StoreError> {
    let row = state
        .assignments
        .get(&(tenant, assignment))
        .ok_or_else(|| destination::integrity("source assignment"))?;
    if row.course_id != course {
        return Err(destination::integrity("source assignment course"));
    }
    Ok((
        current_with_projected_teaching_schedule(state, tenant, assignment)?,
        ObservedAssignmentRevision {
            assignment: *state
                .assignment_references
                .get(&(tenant, assignment))
                .ok_or_else(|| destination::integrity("source assignment reference"))?,
            revision: *state
                .assignment_revisions
                .get(&(tenant, assignment))
                .ok_or_else(|| destination::integrity("source assignment revision"))?,
        },
    ))
}

impl RolloverInput {
    pub(crate) fn payload(&self) -> Result<CurriculumSemanticPayload, StoreError> {
        Ok(self.topology.payload())
    }

    pub(crate) fn with_replaced_payload(
        self,
        payload: CurriculumSemanticPayload,
    ) -> Result<Self, StoreError> {
        let topology = self
            .topology
            .with_replaced_payload(payload)
            .map_err(semantic_error)?;
        Ok(Self {
            topology,
            sources: self.sources,
        })
    }

    pub(crate) fn assignments(&self) -> impl Iterator<Item = RolloverAssignmentInput<'_>> {
        self.topology
            .assignments()
            .zip(self.sources.iter().copied())
            .map(|(semantic, source)| RolloverAssignmentInput { semantic, source })
    }
}

fn semantic_error(error: SemanticPlannerError) -> StoreError {
    StoreError::InvalidRecord(error.to_string())
}

pub(crate) fn current_with_projected_teaching_schedule(
    state: &State,
    tenant: TenantId,
    assignment: AssignmentId,
) -> Result<CurriculumSemanticAssignment, StoreError> {
    let record = state
        .assignments
        .get(&(tenant, assignment))
        .ok_or(StoreError::NotFound)?;
    let policy = state
        .assignment_base_policy
        .get(&(tenant, assignment))
        .ok_or_else(|| destination::integrity("assignment base policy"))?;
    let term = &state
        .courses
        .get(&(tenant, record.course_id))
        .ok_or(StoreError::NotFound)?
        .term;
    let schedule =
        question_model::RelativeAssignmentSchedule::from_base_policy(&policy.policy, term)
            .map_err(|error| StoreError::InvalidRecord(error.to_string()))?;
    destination::current_semantic_assignment(state, tenant, assignment, schedule)
}
