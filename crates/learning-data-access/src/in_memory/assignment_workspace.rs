use super::course_assignments::retirement_would_orphan_active_attempt;
use super::*;
use crate::assignment_revision_checked_next;

pub(super) async fn create_assignment_draft(
    store: &MemoryStore,
    context: TenantContext,
    command: CreateAssignmentDraftCommand,
) -> Result<StoredAssignment, StoreError> {
    let CreateAssignmentDraftCommand {
        actor,
        course,
        assignment,
        title,
    } = command;
    let mut state = store.write_state()?;
    super::course_assignments::require_assignment_editor(&state, context, course, actor)?;
    let draft = crate::new_assignment_draft(context.tenant_id(), course, assignment, title);
    let snapshot = state.clone();
    let result = super::course_assignments::materialize_assignment_locked(
        &mut state,
        context,
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
    context: TenantContext,
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
    super::course_assignments::require_assignment_editor(&state, context, course, actor)?;
    let (key, existing, current) = load_current_assignment(&state, context, course, assignment)?;
    if current != expected_revision {
        return Ok(ReplaceAssignmentContentOutcome::RevisionConflict);
    }
    let replacement = existing.with_content_update(update);
    let structural_change = assignment_content_structurally_changed(&existing, &replacement);
    validate_assignment(&replacement)?;
    validate_memory_assignment_references(&state, context, &replacement)?;
    if structural_change && super::course_policy::memory_assignment_has_run(&state, &existing) {
        return Ok(ReplaceAssignmentContentOutcome::Issued);
    }
    let snapshot = state.clone();
    let base_policy = state
        .assignment_base_policy
        .get(&key)
        .ok_or(StoreError::NotFound)?
        .policy;
    let stored = match stage_assignment_replacement(
        &mut state,
        context,
        &existing,
        replacement,
        base_policy,
    ) {
        Ok(stored) => stored,
        Err(error) => {
            *state = snapshot;
            return Err(error);
        }
    };
    if let Err(error) = super::course_policy::reresolve_active_assignment_attempts(
        &mut state,
        context.tenant_id(),
        course,
        assignment,
    ) {
        *state = snapshot;
        return Err(error);
    }
    Ok(ReplaceAssignmentContentOutcome::Replaced(Box::new(stored)))
}

pub(super) async fn replace_assignment_policies(
    store: &MemoryStore,
    context: TenantContext,
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
    super::course_assignments::require_assignment_editor(&state, context, course, actor)?;
    let (_key, existing, current) = load_current_assignment(&state, context, course, assignment)?;
    if current != expected_revision {
        return Ok(ReplaceAssignmentPoliciesOutcome::RevisionConflict);
    }
    validate_assignment_audience(&state, context.tenant_id(), course, &update.audience)?;
    let course_term = state
        .courses
        .get(&(context.tenant_id(), course))
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
    ) {
        Ok(stored) => stored,
        Err(error) => {
            *state = snapshot;
            return Err(error);
        }
    };
    if let Err(error) = super::course_policy::reresolve_active_assignment_attempts(
        &mut state,
        context.tenant_id(),
        course,
        assignment,
    ) {
        *state = snapshot;
        return Err(error);
    }
    if let Err(error) = super::curriculum_adoption::advance_course_schedule_revision(
        &mut state,
        context.tenant_id(),
        course,
    ) {
        *state = snapshot;
        return Err(error);
    }
    Ok(ReplaceAssignmentPoliciesOutcome::Replaced(Box::new(stored)))
}

pub(super) fn load_current_assignment(
    state: &State,
    context: TenantContext,
    course: CourseId,
    assignment: AssignmentId,
) -> Result<
    (
        (TenantId, AssignmentId),
        AssignmentRecord,
        AssignmentRevision,
    ),
    StoreError,
> {
    let key = (context.tenant_id(), assignment);
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
    context: TenantContext,
    previous: &AssignmentRecord,
    replacement: AssignmentRecord,
    base_policy: question_model::BaseAssignmentPolicy,
) -> Result<StoredAssignment, StoreError> {
    let key = (context.tenant_id(), replacement.id);
    if replacement.tenant != context.tenant_id()
        || replacement.course_id != previous.course_id
        || replacement.id != previous.id
    {
        return Err(StoreError::NotFound);
    }
    if !state.assignment_base_policy.contains_key(&key) {
        return Err(StoreError::NotFound);
    }
    if retirement_would_orphan_active_attempt(state, context.tenant_id(), previous, &replacement)? {
        return Err(StoreError::Conflict);
    }
    let current = state
        .assignment_revisions
        .get(&key)
        .copied()
        .ok_or(StoreError::NotFound)?;
    let next_revision = assignment_revision_checked_next(current)?;
    let (generation, _) = state
        .assignment_scoring
        .get(&key)
        .copied()
        .ok_or(StoreError::NotFound)?;
    let scoring_changed = assignment_scoring_changed(previous, &replacement);
    let scoring_generation = if scoring_changed {
        generation.next().ok_or(StoreError::Conflict)?
    } else {
        generation
    };
    let scoring_status = if scoring_changed
        && super::course_policy::memory_assignment_has_results(state, &replacement)
    {
        ScoringStatus::Recalculating
    } else {
        ScoringStatus::Current
    };
    if scoring_status == ScoringStatus::Recalculating {
        let job = crate::JobId::generate()?;
        let queued = StoredJob {
            tenant: replacement.tenant,
            payload: crate::JobPayload::RecalculateAssignment {
                assignment: replacement.id,
                generation: scoring_generation,
            },
            state: JobState::Ready,
            available_at: state.authoritative_time,
            lease_token: None,
            lease_expires_at: None,
            attempt_count: 0,
            max_attempts: 10,
            failure: None,
        };
        if state.jobs.insert(job, queued).is_some() {
            return Err(StoreError::Conflict);
        }
    }
    let stored = StoredAssignment {
        record: replacement,
        revision: next_revision,
        base_policy,
        scoring_generation,
        scoring_status,
    };
    state.assignments.insert(key, stored.record.clone());
    state.assignment_revisions.insert(key, stored.revision);
    state
        .assignment_scoring
        .insert(key, (stored.scoring_generation, stored.scoring_status));
    if previous.title != stored.record.title
        || previous.policies.grade != stored.record.policies.grade
    {
        super::course_gradebook::advance_course_grade_scheme_revision(
            state,
            stored.record.tenant,
            stored.record.course_id,
        )?;
    }
    state.assignment_base_policy.insert(
        key,
        StoredBaseAssignmentPolicy {
            tenant: stored.record.tenant,
            course: stored.record.course_id,
            assignment: stored.record.id,
            policy: stored.base_policy,
            revision: stored.revision,
        },
    );
    Ok(stored)
}

pub(super) fn validate_assignment_audience(
    state: &State,
    tenant: TenantId,
    course: CourseId,
    audience: &question_model::AssignmentAudience,
) -> Result<(), StoreError> {
    let question_model::AssignmentAudience::AnyOfGroups(groups) = audience else {
        return Ok(());
    };
    for group in groups.iter() {
        if !state
            .course_groups
            .get(&(tenant, group))
            .is_some_and(|record| record.course == course)
        {
            return Err(StoreError::NotFound);
        }
    }
    Ok(())
}
