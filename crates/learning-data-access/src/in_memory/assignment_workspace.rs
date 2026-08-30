use super::course_assignments::retirement_would_orphan_active_attempt;
use super::*;
use crate::{ActorContext, assignment_revision_checked_next};

pub(super) async fn create_assignment_draft(
    store: &MemoryStore,
    context: ActorContext,
    command: CreateAssignmentDraftCommand,
) -> Result<StoredAssignment, StoreError> {
    let CreateAssignmentDraftCommand {
        actor,
        course,
        assignment,
        title,
    } = command;
    let mut state = store.write_state()?;
    super::course_assignments::require_assignment_editor(&state, course, actor)?;
    let draft = crate::new_assignment_draft(course, assignment, title);
    let snapshot = state.clone();
    let result = super::course_assignments::materialize_assignment_locked(
        &mut state,
        draft.record,
        draft.base_policy,
    );
    if let Err(error) = result {
        *state = snapshot;
        return Err(error);
    }
    result
}

pub(super) async fn replace_assignment_content(
    store: &MemoryStore,
    context: ActorContext,
    command: ReplaceAssignmentContentCommand,
) -> Result<ReplaceAssignmentContentOutcome, StoreError> {
    let ReplaceAssignmentContentCommand {
        actor,
        course,
        assignment,
        expected_revision,
        update,
    } = command;
    let mut state = store.write_state()?;
    super::course_assignments::require_assignment_editor(&state, course, actor)?;
    let (key, existing, current) = load_current_assignment(&state, context, course, assignment)?;
    if current != expected_revision {
        return Ok(ReplaceAssignmentContentOutcome::RevisionConflict);
    }
    let replacement = existing.with_content_update(update);
    let issued_work_change = assignment_content_changes_issued_work(&existing, &replacement);
    validate_assignment(&replacement)?;
    validate_memory_assignment_references(&state, &replacement)?;
    if issued_work_change && super::course_policy::memory_assignment_has_run(&state, &existing) {
        return Ok(ReplaceAssignmentContentOutcome::Issued);
    }
    let snapshot = state.clone();
    let base_policy = state
        .assignment_base_policy
        .get(&assignment)
        .ok_or(StoreError::NotFound)?
        .policy;
    let stored = match stage_assignment_replacement(
        &mut state,
        context,
        &existing,
        replacement,
        base_policy,
        actor,
    ) {
        Ok(stored) => stored,
        Err(error) => {
            *state = snapshot;
            return Err(error);
        }
    };
    if let Err(error) =
        super::course_policy::reresolve_active_assignment_attempts(&mut state, course, assignment)
    {
        *state = snapshot;
        return Err(error);
    }
    Ok(ReplaceAssignmentContentOutcome::Replaced(Box::new(stored)))
}

pub(super) async fn replace_assignment_policies(
    store: &MemoryStore,
    context: ActorContext,
    command: ReplaceAssignmentPoliciesCommand,
) -> Result<ReplaceAssignmentPoliciesOutcome, StoreError> {
    let ReplaceAssignmentPoliciesCommand {
        actor,
        course,
        assignment,
        expected_revision,
        update,
    } = command;
    let mut state = store.write_state()?;
    super::course_assignments::require_assignment_editor(&state, course, actor)?;
    let (_key, existing, current) = load_current_assignment(&state, context, course, assignment)?;
    if current != expected_revision {
        return Ok(ReplaceAssignmentPoliciesOutcome::RevisionConflict);
    }
    validate_assignment_audience(&state, course, &update.audience)?;
    let course_term = state
        .courses
        .get(&course)
        .ok_or(StoreError::NotFound)?
        .term
        .clone();
    domain::effective_assignment_policy::validate_base_assignment_policy_for_course_term(
        update.teaching_settings.base_policy,
        &course_term,
    )
    .map_err(|error| {
        StoreError::InvalidRecord(format!("invalid assignment teaching settings: {error:?}"))
    })?;
    let base_policy = update.teaching_settings.base_policy;
    let replacement = existing.with_policies_update(update);
    validate_assignment(&replacement)?;
    let snapshot = state.clone();
    let stored = match stage_assignment_replacement(
        &mut state,
        context,
        &existing,
        replacement,
        base_policy,
        actor,
    ) {
        Ok(stored) => stored,
        Err(error) => {
            *state = snapshot;
            return Err(error);
        }
    };
    if let Err(error) =
        super::course_policy::reresolve_active_assignment_attempts(&mut state, course, assignment)
    {
        *state = snapshot;
        return Err(error);
    }
    if let Err(error) =
        super::curriculum_adoption::advance_course_schedule_revision(&mut state, course)
    {
        *state = snapshot;
        return Err(error);
    }
    Ok(ReplaceAssignmentPoliciesOutcome::Replaced(Box::new(stored)))
}

pub(super) fn load_current_assignment(
    state: &State,
    _context: ActorContext,
    course: CourseId,
    assignment: AssignmentId,
) -> Result<(AssignmentId, AssignmentRecord, AssignmentRevision), StoreError> {
    let key = assignment;
    let record = state
        .assignments
        .get(&key)
        .cloned()
        .ok_or(StoreError::NotFound)?;
    if record.course_id != course {
        return Err(StoreError::NotFound);
    }
    let revision = state
        .assignment_revisions
        .get(&key)
        .copied()
        .ok_or(StoreError::NotFound)?;
    Ok((key, record, revision))
}

/// Applies one focused slice to the authoritative aggregate and advances the
/// shared revision exactly once. The caller owns the rollback snapshot.
pub(super) fn stage_assignment_replacement(
    state: &mut State,
    _context: ActorContext,
    previous: &AssignmentRecord,
    replacement: AssignmentRecord,
    base_policy: question_model::BaseAssignmentPolicy,
    actor: UserId,
) -> Result<StoredAssignment, StoreError> {
    let key = replacement.id;
    if replacement.course_id != previous.course_id || replacement.id != previous.id {
        return Err(StoreError::NotFound);
    }
    if !state.assignment_base_policy.contains_key(&key) {
        return Err(StoreError::NotFound);
    }
    if retirement_would_orphan_active_attempt(state, previous, &replacement)? {
        return Err(StoreError::Conflict);
    }
    let current = state
        .assignment_revisions
        .get(&key)
        .copied()
        .ok_or(StoreError::NotFound)?;
    let next_revision = assignment_revision_checked_next(current)?;
    let (generation, status) = state
        .assignment_scoring
        .get(&key)
        .copied()
        .ok_or(StoreError::NotFound)?;
    let scoring_changed = assignment_scoring_changed(previous, &replacement);
    let has_results = super::course_policy::memory_assignment_has_results(state, &replacement);
    let (scoring_generation, scoring_status, requires_scoring_invalidation) =
        super::scoring_invalidation::definition_scoring_state(
            generation,
            status,
            scoring_changed,
            has_results,
        )?;
    let mut stored = StoredAssignment {
        record: replacement,
        revision: next_revision,
        base_policy,
        scoring_generation,
        scoring_status,
    };
    state
        .assignments
        .insert(stored.record.id, stored.record.clone());
    state.assignment_revisions.insert(key, stored.revision);
    state
        .assignment_scoring
        .insert(key, (stored.scoring_generation, stored.scoring_status));
    if previous.title != stored.record.title
        || previous.policies.grade != stored.record.policies.grade
    {
        super::course_gradebook::advance_course_grade_scheme_revision(
            state,
            stored.record.course_id,
        )?;
    }
    state.assignment_base_policy.insert(
        stored.record.id,
        StoredBaseAssignmentPolicy {
            course: stored.record.course_id,
            assignment: stored.record.id,
            policy: stored.base_policy,
            revision: stored.revision,
        },
    );
    if requires_scoring_invalidation {
        let invalidation = super::scoring_invalidation::request_scoring_invalidation(
            state,
            stored.record.course_id,
            stored.record.id,
            crate::ScoringInvalidationOrigin::assignment_definition(
                stored.record.id.as_uuid(),
                stored.revision,
                actor,
            ),
            crate::JobId::from_uuid(
                crate::ScoringInvalidationOrigin::assignment_definition(
                    stored.record.id.as_uuid(),
                    stored.revision,
                    actor,
                )
                .id
                .as_uuid(),
            ),
        )?;
        stored.scoring_generation = invalidation.generation;
        stored.scoring_status = ScoringStatus::Recalculating;
        state
            .assignment_scoring
            .insert(key, (stored.scoring_generation, stored.scoring_status));
    }
    Ok(stored)
}

pub(super) fn validate_assignment_audience(
    state: &State,
    course: CourseId,
    audience: &question_model::AssignmentAudience,
) -> Result<(), StoreError> {
    let question_model::AssignmentAudience::AnyOfGroups(groups) = audience else {
        return Ok(());
    };
    for group in groups.iter() {
        if !state
            .course_groups
            .get(&group)
            .is_some_and(|record| record.course == course)
        {
            return Err(StoreError::NotFound);
        }
    }
    Ok(())
}
