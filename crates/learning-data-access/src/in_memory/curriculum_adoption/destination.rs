use question_model::curriculum_adoption::{
    CurriculumSemanticAssignment, CurriculumSemanticAssignmentEntry, CurriculumSemanticPool,
};
use question_model::{
    AssignmentAudience, AssignmentDeliveryState, AssignmentId, AssignmentItem, AssignmentItemId,
    AssignmentLifecycle, AssignmentRevision, AssignmentSelectionCandidate,
    AssignmentSelectionGroup, AssignmentSelectionGroupId, BaseAssignmentPolicy,
    ResolvedRelativeAssignmentSchedule,
};

use super::super::{
    AssignmentRecord, CourseId, State, StoreError, StoredBaseAssignmentPolicy, TenantContext, TenantId,
    validate_assignment, validate_memory_assignment_references,
};

pub(super) fn materialize_semantic_assignment(
    state: &mut State,
    context: TenantContext,
    course: CourseId,
    semantic: &CurriculumSemanticAssignment,
) -> Result<(AssignmentId, question_model::AssignmentReference), StoreError> {
    let tenant = context.tenant_id();
    let assignment = random_assignment_id()?;
    let mut items = Vec::new();
    let mut selection_groups = Vec::new();
    for (position, entry) in semantic.entries().iter().enumerate() {
        let position = u32::try_from(position)
            .map_err(|_| StoreError::InvalidRecord("assignment position overflow".into()))?;
        match entry {
            CurriculumSemanticAssignmentEntry::Fixed {
                reference,
                points_possible,
                scoring_mode,
            } => items.push(AssignmentItem {
                id: random_item_id()?,
                reference: *reference,
                position,
                points_possible: *points_possible,
                delivery_state: AssignmentDeliveryState::Active,
                scoring_mode: *scoring_mode,
            }),
            CurriculumSemanticAssignmentEntry::Pool(pool) => {
                let candidates = pool
                    .candidates()
                    .iter()
                    .enumerate()
                    .map(|(candidate_position, reference)| {
                        Ok(AssignmentSelectionCandidate {
                            id: random_item_id()?,
                            position: u32::try_from(candidate_position).map_err(|_| {
                                StoreError::InvalidRecord("pool position overflow".into())
                            })?,
                            reference: *reference,
                            delivery_state: AssignmentDeliveryState::Active,
                        })
                    })
                    .collect::<Result<Vec<_>, StoreError>>()?;
                selection_groups.push(AssignmentSelectionGroup {
                    id: random_group_id()?,
                    position,
                    draw_count: pool.draw_count(),
                    points_per_item: pool.points_per_item(),
                    ordering: pool.ordering(),
                    algorithm: pool.algorithm(),
                    candidates,
                });
            }
        }
    }
    let resolved = semantic
        .schedule()
        .resolve_for_target_term(
            &state
                .courses
                .get(&(tenant, course))
                .ok_or(StoreError::NotFound)?
                .term,
        )
        .map_err(|error| StoreError::InvalidRecord(error.to_string()))?;
    let defaults = semantic.defaults();
    let record = AssignmentRecord {
        id: assignment,
        tenant,
        course_id: course,
        title: semantic.title().to_owned(),
        lifecycle: AssignmentLifecycle::Draft,
        instructions: semantic.instructions().clone(),
        audience: AssignmentAudience::CourseWide,
        items,
        selection_groups,
        disclosure_policy: defaults.learner_disclosure,
        policies: defaults.run_policies,
    };
    let base_policy = base_policy(defaults, &resolved);
    super::super::course_assignments::materialize_assignment_locked(
        state,
        context,
        record,
        base_policy,
    )?;
    let reference = *state
        .assignment_references
        .get(&(tenant, assignment))
        .ok_or_else(|| integrity("assignment reference"))?;
    Ok((assignment, reference))
}

pub(super) fn replace_reusable_meaning(
    state: &mut State,
    tenant: TenantId,
    assignment: AssignmentId,
    semantic: &CurriculumSemanticAssignment,
) -> Result<AssignmentRevision, StoreError> {
    let key = (tenant, assignment);
    let existing = state
        .assignments
        .get(&key)
        .cloned()
        .ok_or(StoreError::NotFound)?;
    let current = *state
        .assignment_revisions
        .get(&key)
        .ok_or_else(|| integrity("assignment revision"))?;
    let next = crate::assignment_revision_checked_next(current)?;
    let (items, selection_groups) = materialize_entries(semantic)?;
    let mut replacement = existing;
    let title_changed = replacement.title != semantic.title();
    let course = replacement.course_id;
    replacement.title = semantic.title().to_owned();
    replacement.instructions = semantic.instructions().clone();
    replacement.items = items;
    replacement.selection_groups = selection_groups;
    replacement.disclosure_policy = semantic.defaults().learner_disclosure;
    replacement.policies = semantic.defaults().run_policies;
    validate_assignment(&replacement)?;
    validate_memory_assignment_references(
        state,
        TenantContext::from_authenticated_session(tenant),
        &replacement,
    )?;
    let policy = state
        .assignment_base_policy
        .get(&key)
        .copied()
        .ok_or_else(|| integrity("assignment base policy"))?;
    let defaults = semantic.defaults();
    let mut next_policy = policy.policy;
    next_policy.time_limit_seconds = defaults.time_limit_seconds;
    next_policy.attempt_limit = defaults.attempt_limit;
    next_policy.late_submission = defaults.late_submission;
    next_policy.deadline_behavior = defaults.deadline_behavior;
    state.assignments.insert(key, replacement);
    state.assignment_revisions.insert(key, next);
    state.assignment_base_policy.insert(
        key,
        StoredBaseAssignmentPolicy {
            revision: next,
            policy: next_policy,
            ..policy
        },
    );
    if title_changed {
        super::super::course_gradebook::advance_course_grade_scheme_revision(
            state,
            tenant,
            course,
        )?;
    }
    Ok(next)
}

pub(super) fn current_semantic_assignment(
    state: &State,
    tenant: TenantId,
    assignment: AssignmentId,
    relative_schedule: question_model::RelativeAssignmentSchedule,
) -> Result<CurriculumSemanticAssignment, StoreError> {
    let record = state
        .assignments
        .get(&(tenant, assignment))
        .ok_or(StoreError::NotFound)?;
    let stored_policy = state
        .assignment_base_policy
        .get(&(tenant, assignment))
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
        learner_disclosure: record.disclosure_policy,
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
    semantic: &CurriculumSemanticAssignment,
) -> Result<(Vec<AssignmentItem>, Vec<AssignmentSelectionGroup>), StoreError> {
    let mut items = Vec::new();
    let mut groups = Vec::new();
    for (position, entry) in semantic.entries().iter().enumerate() {
        let position = u32::try_from(position)
            .map_err(|_| StoreError::InvalidRecord("assignment position overflow".into()))?;
        match entry {
            CurriculumSemanticAssignmentEntry::Fixed {
                reference,
                points_possible,
                scoring_mode,
            } => items.push(AssignmentItem {
                id: random_item_id()?,
                reference: *reference,
                position,
                points_possible: *points_possible,
                delivery_state: AssignmentDeliveryState::Active,
                scoring_mode: *scoring_mode,
            }),
            CurriculumSemanticAssignmentEntry::Pool(pool) => {
                groups.push(AssignmentSelectionGroup {
                    id: random_group_id()?,
                    position,
                    draw_count: pool.draw_count(),
                    points_per_item: pool.points_per_item(),
                    ordering: pool.ordering(),
                    algorithm: pool.algorithm(),
                    candidates: pool
                        .candidates()
                        .iter()
                        .enumerate()
                        .map(|(position, reference)| {
                            Ok(AssignmentSelectionCandidate {
                                id: random_item_id()?,
                                position: u32::try_from(position).map_err(|_| {
                                    StoreError::InvalidRecord("pool position overflow".into())
                                })?,
                                reference: *reference,
                                delivery_state: AssignmentDeliveryState::Active,
                            })
                        })
                        .collect::<Result<Vec<_>, StoreError>>()?,
                })
            }
        }
    }
    Ok((items, groups))
}

fn base_policy(
    defaults: &question_model::ReusableAssignmentDefaults,
    schedule: &ResolvedRelativeAssignmentSchedule,
) -> BaseAssignmentPolicy {
    BaseAssignmentPolicy {
        available_at: schedule.available_at.as_ref().map(|value| value.timestamp),
        due_at: schedule.due_at.as_ref().map(|value| value.timestamp),
        closes_at: schedule.closes_at.as_ref().map(|value| value.timestamp),
        time_limit_seconds: defaults.time_limit_seconds,
        attempt_limit: defaults.attempt_limit,
        late_submission: defaults.late_submission,
        deadline_behavior: defaults.deadline_behavior,
    }
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
