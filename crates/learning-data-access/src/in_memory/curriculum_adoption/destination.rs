use question_model::curriculum_adoption::{
    CurriculumSemanticAssignment, CurriculumSemanticAssignmentEntry, CurriculumSemanticPool,
};
use question_model::{
    AssignmentAudience, AssignmentDeliveryState, AssignmentId, AssignmentItem, AssignmentItemId,
    AssignmentLifecycle, AssignmentRevision, AssignmentSelectionCandidate,
    AssignmentSelectionGroup, AssignmentSelectionGroupId,
};

use super::super::{
    AssignmentRecord, CourseId, State, StoreError, StoredBaseAssignmentPolicy, validate_assignment,
    validate_memory_assignment_references,
};
use crate::curriculum_adoption::{
    AssignmentMaterializationEntry, AssignmentMaterializationPlan, SemanticPlannerError,
    plan_assignment_entries, plan_assignment_materialization,
};

pub(super) fn materialize_semantic_assignment(
    state: &mut State,
    course: CourseId,
    semantic: &CurriculumSemanticAssignment,
) -> Result<(AssignmentId, question_model::AssignmentReference), StoreError> {
    let term = &state.courses.get(&course).ok_or(StoreError::NotFound)?.term;
    let plan: AssignmentMaterializationPlan =
        plan_assignment_materialization(semantic, term).map_err(semantic_error)?;
    let assignment = random_assignment_id()?;
    let (items, selection_groups) = materialize_entries(&plan.entries)?;
    let base_policy = plan.base_policy();
    let record = AssignmentRecord {
        id: assignment,
        course_id: course,
        title: plan.title,
        lifecycle: AssignmentLifecycle::Draft,
        instructions: plan.instructions,
        audience: AssignmentAudience::CourseWide,
        items,
        selection_groups,
        disclosure_policy: plan.defaults.student_disclosure,
        policies: plan.defaults.run_policies,
    };
    super::super::course_assignments::materialize_assignment_locked(state, record, base_policy)?;
    let reference = *state
        .assignment_references
        .get(&assignment)
        .ok_or_else(|| integrity("assignment reference"))?;
    Ok((assignment, reference))
}

pub(super) fn replace_reusable_meaning(
    state: &mut State,
    assignment: AssignmentId,
    semantic: &CurriculumSemanticAssignment,
) -> Result<AssignmentRevision, StoreError> {
    let existing = state
        .assignments
        .get(&assignment)
        .cloned()
        .ok_or(StoreError::NotFound)?;
    let current = *state
        .assignment_revisions
        .get(&assignment)
        .ok_or_else(|| integrity("assignment revision"))?;
    let next = crate::assignment_revision_checked_next(current)?;
    let entries = plan_assignment_entries(semantic).map_err(semantic_error)?;
    let (items, selection_groups) = materialize_entries(&entries)?;
    let mut replacement = existing;
    let title_changed = replacement.title != semantic.title();
    let course = replacement.course_id;
    replacement.title = semantic.title().to_owned();
    replacement.instructions = semantic.instructions().clone();
    replacement.items = items;
    replacement.selection_groups = selection_groups;
    replacement.disclosure_policy = semantic.defaults().student_disclosure;
    replacement.policies = semantic.defaults().run_policies;
    validate_assignment(&replacement)?;
    validate_memory_assignment_references(state, &replacement)?;
    let policy = state
        .assignment_base_policy
        .get(&assignment)
        .copied()
        .ok_or_else(|| integrity("assignment base policy"))?;
    let defaults = semantic.defaults();
    let mut next_policy = policy.policy;
    next_policy.time_limit_seconds = defaults.time_limit_seconds;
    next_policy.attempt_limit = defaults.attempt_limit;
    next_policy.late_submission = defaults.late_submission;
    next_policy.deadline_behavior = defaults.deadline_behavior;
    state.assignments.insert(assignment, replacement);
    state.assignment_revisions.insert(assignment, next);
    state.assignment_base_policy.insert(
        assignment,
        StoredBaseAssignmentPolicy {
            revision: next,
            policy: next_policy,
            ..policy
        },
    );
    if title_changed {
        super::super::course_gradebook::advance_course_grade_scheme_revision(state, course)?;
    }
    Ok(next)
}

pub(super) fn current_semantic_assignment(
    state: &State,
    assignment: AssignmentId,
    relative_schedule: question_model::RelativeAssignmentSchedule,
) -> Result<CurriculumSemanticAssignment, StoreError> {
    let record = state
        .assignments
        .get(&assignment)
        .ok_or(StoreError::NotFound)?;
    let stored_policy = state
        .assignment_base_policy
        .get(&assignment)
        .ok_or_else(|| integrity("assignment base policy"))?;
    let mut positioned = record
        .items
        .iter()
        .filter(|item| item.delivery_state == AssignmentDeliveryState::Active)
        .map(|item| {
            (
                item.position,
                CurriculumSemanticAssignmentEntry::Fixed {
                    reference: item.reference,
                    points_possible: item.points_possible,
                    scoring_mode: item.scoring_mode,
                },
            )
        })
        .collect::<Vec<_>>();
    for group in &record.selection_groups {
        let pool = CurriculumSemanticPool::new(
            group
                .candidates
                .iter()
                .filter(|candidate| candidate.delivery_state == AssignmentDeliveryState::Active)
                .map(|candidate| candidate.reference)
                .collect(),
            group.draw_count,
            group.points_per_item,
            group.ordering,
            group.algorithm,
        )
        .map_err(|error| StoreError::InvalidRecord(error.to_string()))?;
        positioned.push((
            group.position,
            CurriculumSemanticAssignmentEntry::Pool(pool),
        ));
    }
    positioned.sort_by_key(|(position, _)| *position);
    let defaults = question_model::ReusableAssignmentDefaults {
        time_limit_seconds: stored_policy.policy.time_limit_seconds,
        attempt_limit: stored_policy.policy.attempt_limit,
        late_submission: stored_policy.policy.late_submission,
        deadline_behavior: stored_policy.policy.deadline_behavior,
        run_policies: record.policies,
        student_disclosure: record.disclosure_policy,
    };
    CurriculumSemanticAssignment::new(
        record.title.clone(),
        record.instructions.clone(),
        positioned.into_iter().map(|(_, entry)| entry).collect(),
        defaults,
        relative_schedule,
    )
    .map_err(|error| StoreError::InvalidRecord(error.to_string()))
}

fn materialize_entries(
    entries: &[AssignmentMaterializationEntry],
) -> Result<(Vec<AssignmentItem>, Vec<AssignmentSelectionGroup>), StoreError> {
    let mut items = Vec::new();
    let mut groups = Vec::new();
    for entry in entries {
        match entry {
            AssignmentMaterializationEntry::Fixed {
                position,
                reference,
                points_possible,
                scoring_mode,
            } => items.push(AssignmentItem {
                id: random_item_id()?,
                reference: *reference,
                position: *position,
                points_possible: *points_possible,
                delivery_state: AssignmentDeliveryState::Active,
                scoring_mode: *scoring_mode,
            }),
            AssignmentMaterializationEntry::Pool {
                position,
                candidates,
                draw_count,
                points_per_item,
                ordering,
                algorithm,
            } => groups.push(AssignmentSelectionGroup {
                id: random_group_id()?,
                position: *position,
                draw_count: *draw_count,
                points_per_item: *points_per_item,
                ordering: *ordering,
                algorithm: *algorithm,
                candidates: candidates
                    .iter()
                    .map(|candidate| {
                        Ok(AssignmentSelectionCandidate {
                            id: random_item_id()?,
                            position: candidate.position,
                            reference: candidate.reference,
                            delivery_state: AssignmentDeliveryState::Active,
                        })
                    })
                    .collect::<Result<Vec<_>, StoreError>>()?,
            }),
        }
    }
    Ok((items, groups))
}

fn semantic_error(error: SemanticPlannerError) -> StoreError {
    StoreError::InvalidRecord(error.to_string())
}

fn random_assignment_id() -> Result<AssignmentId, StoreError> {
    random_uuid("assignment").map(AssignmentId::from_uuid)
}

fn random_item_id() -> Result<AssignmentItemId, StoreError> {
    random_uuid("assignment item").map(AssignmentItemId::from_uuid)
}

fn random_group_id() -> Result<AssignmentSelectionGroupId, StoreError> {
    random_uuid("assignment selection group").map(AssignmentSelectionGroupId::from_uuid)
}

fn random_uuid(label: &str) -> Result<uuid::Uuid, StoreError> {
    crate::random_uuid::random_uuid_v4(|error| {
        StoreError::Unavailable(format!("{label} ID randomness unavailable: {error}"))
    })
}

pub(super) fn integrity(missing: &str) -> StoreError {
    StoreError::Unavailable(format!(
        "curriculum adoption integrity failure: missing {missing}"
    ))
}
