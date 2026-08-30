use async_trait::async_trait;

use super::*;
use crate::{
    ActorContext, ReplaceUnissuedAssignmentDefinitionCommand,
    ReplaceUnissuedAssignmentDefinitionOutcome, assignment_revision_checked_next,
};

#[async_trait]
impl crate::CourseAssignmentStore for MemoryStore {
    async fn create_assignment_impl(
        &self,
        _context: ActorContext,
        command: CreateAssignmentCommand,
    ) -> Result<StoredAssignment, StoreError> {
        let CreateAssignmentCommand {
            actor,
            assignment,
            base_policy,
        } = command;
        if assignment.lifecycle != question_model::AssignmentLifecycle::Draft {
            return Err(StoreError::InvalidRecord(
                "new assignments must begin in the draft lifecycle".to_string(),
            ));
        }
        validate_assignment(&assignment)?;
        let mut state = self.write_state()?;
        require_assignment_editor(&state, assignment.course_id, actor)?;
        let snapshot = state.clone();
        let result = materialize_assignment_locked(&mut state, assignment, base_policy);
        if let Err(error) = result {
            *state = snapshot;
            return Err(error);
        }
        result
    }
    async fn create_assignment_draft_impl(
        &self,
        context: ActorContext,
        command: CreateAssignmentDraftCommand,
    ) -> Result<StoredAssignment, StoreError> {
        super::assignment_workspace::create_assignment_draft(self, context, command).await
    }
    async fn replace_assignment_content_impl(
        &self,
        context: ActorContext,
        command: ReplaceAssignmentContentCommand,
    ) -> Result<ReplaceAssignmentContentOutcome, StoreError> {
        super::assignment_workspace::replace_assignment_content(self, context, command).await
    }
    async fn replace_assignment_policies_impl(
        &self,
        context: ActorContext,
        command: ReplaceAssignmentPoliciesCommand,
    ) -> Result<ReplaceAssignmentPoliciesOutcome, StoreError> {
        super::assignment_workspace::replace_assignment_policies(self, context, command).await
    }
    async fn replace_assignment_impl(
        &self,
        _context: ActorContext,
        command: ReplaceAssignmentCommand,
    ) -> Result<StoredAssignment, StoreError> {
        let ReplaceAssignmentCommand {
            actor,
            course,
            assignment,
            expected_revision,
            update,
        } = command;
        let mut state = self.write_state()?;
        let snapshot = state.clone();
        require_assignment_editor(&state, course, actor)?;
        let key = assignment;
        let existing = state
            .assignments
            .get(&assignment)
            .cloned()
            .ok_or(StoreError::NotFound)?;
        if existing.course_id != course {
            return Err(StoreError::NotFound);
        }
        let current = state
            .assignment_revisions
            .get(&key)
            .copied()
            .ok_or(StoreError::NotFound)?;
        if current != expected_revision {
            return Err(StoreError::Conflict);
        }
        let next_revision = assignment_revision_checked_next(current)?;
        crate::ensure_assignment_update_preserves_references(&existing, &update)?;
        let assignment = AssignmentRecord {
            id: assignment,
            course_id: course,
            title: update.title,
            lifecycle: existing.lifecycle,
            instructions: existing.instructions.clone(),
            audience: update.audience,
            items: update.items,
            selection_groups: update.selection_groups,
            disclosure_policy: update.disclosure_policy,
            policies: update.policies,
        };
        validate_assignment(&assignment)?;
        validate_memory_assignment_references(&state, &assignment)?;
        let previous = state
            .assignments
            .get(&assignment.id)
            .ok_or(StoreError::NotFound)?;
        let course_grade_projection_changed = previous.title != assignment.title;
        if retirement_would_orphan_active_attempt(&state, previous, &assignment)? {
            return Err(StoreError::Conflict);
        }
        let scoring_changed = assignment_scoring_changed(previous, &assignment);
        let (generation, status) = state
            .assignment_scoring
            .get(&key)
            .copied()
            .ok_or(StoreError::NotFound)?;
        let has_results = memory_assignment_has_results(&state, &assignment);
        let (scoring_generation, scoring_status, requires_scoring_invalidation) =
            super::scoring_invalidation::definition_scoring_state(
                generation,
                status,
                scoring_changed,
                has_results,
            )?;
        let mut stored = StoredAssignment {
            record: assignment,
            revision: next_revision,
            base_policy: state
                .assignment_base_policy
                .get(&key)
                .ok_or(StoreError::NotFound)?
                .policy,
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
        if requires_scoring_invalidation {
            let origin = crate::ScoringInvalidationOrigin::assignment_definition(
                stored.record.id.as_uuid(),
                stored.revision,
                actor,
            );
            match super::scoring_invalidation::request_scoring_invalidation(
                &mut state,
                stored.record.course_id,
                stored.record.id,
                origin,
                crate::JobId::from_uuid(origin.id.as_uuid()),
            ) {
                Ok(invalidation) => {
                    stored.scoring_generation = invalidation.generation;
                    stored.scoring_status = ScoringStatus::Recalculating;
                    state
                        .assignment_scoring
                        .insert(key, (stored.scoring_generation, stored.scoring_status));
                }
                Err(error) => {
                    *state = snapshot;
                    return Err(error);
                }
            }
        }
        if course_grade_projection_changed
            && let Err(error) = super::course_gradebook::advance_course_grade_scheme_revision(
                &mut state,
                stored.record.course_id,
            )
        {
            *state = snapshot;
            return Err(error);
        }
        // Audience is an S5 input.  Replacing an assignment definition can
        // therefore revoke an already-issued learner even though no M1--M4
        // policy row changed.  Keep the definition, revision, and mutable
        // active-attempt projection in the same rollback boundary.
        if let Err(error) = super::course_policy::reresolve_active_assignment_attempts(
            &mut state,
            course,
            stored.record.id,
        ) {
            *state = snapshot;
            return Err(error);
        }
        Ok(stored)
    }

    async fn replace_unissued_assignment_definition_impl(
        &self,
        _context: ActorContext,
        command: ReplaceUnissuedAssignmentDefinitionCommand,
    ) -> Result<ReplaceUnissuedAssignmentDefinitionOutcome, StoreError> {
        let ReplaceUnissuedAssignmentDefinitionCommand {
            actor,
            course,
            assignment,
            expected_revision,
            definition,
            base_policy,
        } = command;
        if definition.id != assignment || definition.course_id != course {
            return Err(StoreError::InvalidRecord(
                "unissued definition bindings do not match the command route".to_string(),
            ));
        }
        validate_assignment(&definition)?;
        let mut state = self.write_state()?;
        require_assignment_editor(&state, course, actor)?;
        let key = assignment;
        let existing = state
            .assignments
            .get(&assignment)
            .cloned()
            .ok_or(StoreError::NotFound)?;
        if existing.course_id != course {
            return Err(StoreError::NotFound);
        }
        let current = state
            .assignment_revisions
            .get(&key)
            .copied()
            .ok_or(StoreError::NotFound)?;
        if current != expected_revision {
            return Err(StoreError::Conflict);
        }
        if super::course_policy::memory_assignment_has_run(&state, &existing) {
            return Ok(ReplaceUnissuedAssignmentDefinitionOutcome::Issued);
        }
        validate_memory_assignment_references(&state, &definition)?;
        let course_term = state
            .courses
            .get(&course)
            .ok_or(StoreError::NotFound)?
            .term
            .clone();
        domain::effective_assignment_policy::validate_base_assignment_policy_for_course_term(
            base_policy,
            &course_term,
        )
        .map_err(|error| {
            StoreError::InvalidRecord(format!("invalid assignment base policy: {error:?}"))
        })?;

        let next_revision = assignment_revision_checked_next(current)?;
        let (scoring_generation, scoring_status) = state
            .assignment_scoring
            .get(&key)
            .copied()
            .ok_or(StoreError::NotFound)?;
        let stored = StoredAssignment {
            record: definition,
            revision: next_revision,
            base_policy,
            scoring_generation,
            scoring_status,
        };
        let snapshot = state.clone();
        state.assignments.insert(assignment, stored.record.clone());
        state.assignment_revisions.insert(key, stored.revision);
        state.assignment_base_policy.insert(
            assignment,
            StoredBaseAssignmentPolicy {
                course,
                assignment,
                policy: stored.base_policy,
                revision: stored.revision,
            },
        );
        if existing.title != stored.record.title
            && let Err(error) =
                super::course_gradebook::advance_course_grade_scheme_revision(&mut state, course)
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
        Ok(ReplaceUnissuedAssignmentDefinitionOutcome::Replaced(
            Box::new(stored),
        ))
    }

    async fn replace_assignment_fixed_item_impl(
        &self,
        _context: ActorContext,
        command: ReplaceAssignmentFixedItemCommand,
    ) -> Result<StoredAssignment, StoreError> {
        let mut state = self.write_state()?;
        require_assignment_editor(&state, command.course, command.actor)?;
        let key = command.assignment;
        let existing = state
            .assignments
            .get(&key)
            .cloned()
            .ok_or(StoreError::NotFound)?;
        if existing.course_id != command.course {
            return Err(StoreError::NotFound);
        }
        let revision = state
            .assignment_revisions
            .get(&key)
            .copied()
            .ok_or(StoreError::NotFound)?;
        if revision != command.expected_revision {
            return Err(StoreError::Conflict);
        }
        let mut replacement = existing.clone();
        let item = replacement
            .items
            .iter_mut()
            .find(|item| item.id == command.current_item)
            .ok_or(StoreError::NotFound)?;
        item.reference = command.replacement;
        validate_assignment(&replacement)?;
        validate_memory_assignment_references(&state, &replacement)?;
        let stored = StoredAssignment {
            record: replacement,
            revision: assignment_revision_checked_next(revision)?,
            base_policy: state
                .assignment_base_policy
                .get(&key)
                .ok_or(StoreError::NotFound)?
                .policy,
            scoring_generation: state
                .assignment_scoring
                .get(&key)
                .copied()
                .ok_or(StoreError::NotFound)?
                .0,
            scoring_status: state
                .assignment_scoring
                .get(&key)
                .copied()
                .ok_or(StoreError::NotFound)?
                .1,
        };
        state
            .assignments
            .insert(command.assignment, stored.record.clone());
        state.assignment_revisions.insert(key, stored.revision);
        Ok(stored)
    }
    async fn add_assignment_fixed_item_impl(
        &self,
        _context: ActorContext,
        command: AddAssignmentFixedItemCommand,
    ) -> Result<StoredAssignment, StoreError> {
        let mut state = self.write_state()?;
        require_assignment_editor(&state, command.course, command.actor)?;
        let key = command.assignment;
        let existing = state
            .assignments
            .get(&key)
            .cloned()
            .ok_or(StoreError::NotFound)?;
        if existing.course_id != command.course {
            return Err(StoreError::NotFound);
        }
        let revision = state
            .assignment_revisions
            .get(&key)
            .copied()
            .ok_or(StoreError::NotFound)?;
        if revision != command.expected_revision {
            return Err(StoreError::Conflict);
        }
        if super::course_policy::memory_assignment_has_run(&state, &existing) {
            return Err(StoreError::Conflict);
        }
        if existing.items.iter().any(|item| item.id == command.item.id)
            || existing
                .selection_groups
                .iter()
                .flat_map(|group| &group.candidates)
                .any(|candidate| candidate.id == command.item.id)
        {
            return Err(StoreError::AlreadyExists);
        }
        let source_count = existing
            .items
            .len()
            .checked_add(existing.selection_groups.len())
            .ok_or_else(|| {
                StoreError::Unavailable("assignment source count overflowed".to_string())
            })?;
        if usize::try_from(command.item.position).map_err(|_| {
            StoreError::InvalidRecord("assignment item position is invalid".to_string())
        })? > source_count
        {
            return Err(StoreError::InvalidRecord(
                "assignment item position is outside the assignment source range".to_string(),
            ));
        }
        let mut replacement = existing.clone();
        for item in &mut replacement.items {
            if item.position >= command.item.position {
                item.position = item.position.checked_add(1).ok_or_else(|| {
                    StoreError::Unavailable("assignment item position limit reached".to_string())
                })?;
            }
        }
        for group in &mut replacement.selection_groups {
            if group.position >= command.item.position {
                group.position = group.position.checked_add(1).ok_or_else(|| {
                    StoreError::Unavailable("assignment group position limit reached".to_string())
                })?;
            }
        }
        replacement.items.push(command.item);
        replacement.items.sort_by_key(|item| item.position);
        validate_assignment(&replacement)?;
        validate_memory_assignment_references(&state, &replacement)?;
        let base_policy = state
            .assignment_base_policy
            .get(&key)
            .ok_or(StoreError::NotFound)?
            .policy;
        let (generation, status) = state
            .assignment_scoring
            .get(&key)
            .copied()
            .ok_or(StoreError::NotFound)?;
        let stored = StoredAssignment {
            record: replacement,
            revision: assignment_revision_checked_next(revision)?,
            base_policy,
            scoring_generation: generation,
            scoring_status: status,
        };
        state
            .assignments
            .insert(command.assignment, stored.record.clone());
        state.assignment_revisions.insert(key, stored.revision);
        Ok(stored)
    }
    async fn remove_assignment_fixed_item_impl(
        &self,
        _context: ActorContext,
        command: RemoveAssignmentFixedItemCommand,
    ) -> Result<StoredAssignment, StoreError> {
        let mut state = self.write_state()?;
        require_assignment_editor(&state, command.course, command.actor)?;
        let key = command.assignment;
        let existing = state
            .assignments
            .get(&key)
            .cloned()
            .ok_or(StoreError::NotFound)?;
        if existing.course_id != command.course {
            return Err(StoreError::NotFound);
        }
        let revision = state
            .assignment_revisions
            .get(&key)
            .copied()
            .ok_or(StoreError::NotFound)?;
        if revision != command.expected_revision {
            return Err(StoreError::Conflict);
        }
        if super::course_policy::memory_assignment_has_run(&state, &existing) {
            return Err(StoreError::Conflict);
        }
        let removal = existing
            .items
            .iter()
            .position(|item| item.id == command.item)
            .ok_or(StoreError::NotFound)?;
        let mut replacement = existing.clone();
        let removed_position = replacement.items.remove(removal).position;
        for item in &mut replacement.items {
            if item.position > removed_position {
                item.position -= 1;
            }
        }
        for group in &mut replacement.selection_groups {
            if group.position > removed_position {
                group.position -= 1;
            }
        }
        replacement.items.sort_by_key(|item| item.position);
        validate_assignment(&replacement)?;
        let base_policy = state
            .assignment_base_policy
            .get(&key)
            .ok_or(StoreError::NotFound)?
            .policy;
        let (generation, status) = state
            .assignment_scoring
            .get(&key)
            .copied()
            .ok_or(StoreError::NotFound)?;
        let stored = StoredAssignment {
            record: replacement,
            revision: assignment_revision_checked_next(revision)?,
            base_policy,
            scoring_generation: generation,
            scoring_status: status,
        };
        state
            .assignments
            .insert(command.assignment, stored.record.clone());
        state.assignment_revisions.insert(key, stored.revision);
        Ok(stored)
    }
    async fn delete_and_regrade_assignment_item_impl(
        &self,
        context: ActorContext,
        command: DeleteAndRegradeAssignmentItemCommand,
    ) -> Result<StoredAssignment, StoreError> {
        {
            let state = self.read_state()?;
            require_assignment_editor(&state, command.course, command.actor)?;
        }
        let stored = crate::CourseAssignmentStore::get_assignment_for_edit_impl(
            self,
            context,
            command.assignment,
        )
        .await?
        .ok_or(StoreError::NotFound)?;
        if stored.record.course_id != command.course {
            return Err(StoreError::NotFound);
        }
        if stored.revision != command.expected_revision {
            return Err(StoreError::Conflict);
        }
        let Some(update) = delete_and_regrade_update(&stored, command.item)? else {
            return Ok(stored);
        };
        crate::CourseAssignmentStore::replace_assignment_impl(
            self,
            context,
            ReplaceAssignmentCommand {
                actor: command.actor,
                course: command.course,
                assignment: command.assignment,
                expected_revision: command.expected_revision,
                update,
            },
        )
        .await
    }
    async fn get_assignment_for_edit_impl(
        &self,
        _context: ActorContext,
        assignment: AssignmentId,
    ) -> Result<Option<StoredAssignment>, StoreError> {
        let state = self.read_state()?;
        let key = assignment;
        let Some(record) = state.assignments.get(&assignment).cloned() else {
            return Ok(None);
        };
        let revision = state
            .assignment_revisions
            .get(&key)
            .copied()
            .ok_or_else(|| {
                StoreError::Unavailable(
                    "assignment revision is missing from memory state".to_string(),
                )
            })?;
        let (scoring_generation, scoring_status) =
            state.assignment_scoring.get(&key).copied().ok_or_else(|| {
                StoreError::Unavailable(
                    "assignment scoring state is missing from memory state".to_string(),
                )
            })?;
        Ok(Some(StoredAssignment {
            record,
            revision,
            base_policy: state
                .assignment_base_policy
                .get(&key)
                .ok_or(StoreError::NotFound)?
                .policy,
            scoring_generation,
            scoring_status,
        }))
    }
    async fn get_assignment_impl(
        &self,
        _context: ActorContext,
        assignment: AssignmentId,
    ) -> Result<Option<AssignmentRecord>, StoreError> {
        let state = self.read_state()?;
        let Some(record) = state.assignments.get(&assignment).cloned() else {
            return Ok(None);
        };
        Ok(Some(record))
    }
    async fn list_assignments_impl(
        &self,
        _context: ActorContext,
        course: CourseId,
        page: PageRequest,
    ) -> Result<Page<AssignmentRecord>, StoreError> {
        let state = self.read_state()?;
        if !state.courses.contains_key(&course) {
            return Err(StoreError::NotFound);
        }
        let records = state
            .assignments
            .iter()
            .filter(|(_, record)| record.course_id == course)
            .map(|(assignment, record)| (assignment.to_string(), record.clone()))
            .collect();
        Ok(page_records(records, &page))
    }
    async fn get_enrollment_impl(
        &self,
        _context: ActorContext,
        enrollment: EnrollmentId,
    ) -> Result<Option<AssignmentEnrollment>, StoreError> {
        let state = self.read_state()?;
        let Some(record) = state.enrollments.get(&enrollment).cloned() else {
            return Ok(None);
        };
        let assignment = assignment_record(&state, record.assignment)?;
        if !course_records_accessible(&state, assignment.course_id) {
            return Ok(None);
        }
        Ok(Some(record))
    }
}

/// Inserts one ordinary assignment aggregate while its caller holds the sole
/// Memory write lock. Authority must already have been established against the
/// destination course; all record validation is repeated here.
pub(super) fn materialize_assignment_locked(
    state: &mut State,
    assignment: AssignmentRecord,
    base_policy: question_model::BaseAssignmentPolicy,
) -> Result<StoredAssignment, StoreError> {
    if assignment.lifecycle != question_model::AssignmentLifecycle::Draft {
        return Err(StoreError::InvalidRecord(
            "new assignments must begin in the draft lifecycle".into(),
        ));
    }
    validate_assignment(&assignment)?;
    if state.assignments.contains_key(&assignment.id) {
        return Err(StoreError::AlreadyExists);
    }
    validate_memory_assignment_references(state, &assignment)?;
    let course_term = state
        .courses
        .get(&assignment.course_id)
        .ok_or(StoreError::NotFound)?
        .term
        .clone();
    domain::effective_assignment_policy::validate_base_assignment_policy_for_course_term(
        base_policy,
        &course_term,
    )
    .map_err(|error| {
        StoreError::InvalidRecord(format!("invalid assignment base policy: {error:?}"))
    })?;
    let stored = StoredAssignment {
        record: assignment,
        revision: AssignmentRevision::INITIAL,
        base_policy,
        scoring_generation: ScoringGeneration::INITIAL,
        scoring_status: ScoringStatus::Current,
    };
    super::navigation_references::ensure_assignment_reference(state, stored.record.id)?;
    state
        .assignments
        .insert(stored.record.id, stored.record.clone());
    state
        .assignment_revisions
        .insert(stored.record.id, stored.revision);
    state.assignment_base_policy.insert(
        stored.record.id,
        StoredBaseAssignmentPolicy {
            course: stored.record.course_id,
            assignment: stored.record.id,
            policy: base_policy,
            revision: stored.revision,
        },
    );
    state.assignment_scoring.insert(
        stored.record.id,
        (stored.scoring_generation, stored.scoring_status),
    );
    super::course_gradebook::advance_course_grade_scheme_revision(state, stored.record.course_id)?;
    super::curriculum_adoption::advance_course_schedule_revision(state, stored.record.course_id)?;
    Ok(stored)
}

/// Verifies the command-local actor against current direct Instructor
/// authority before an assignment definition is read or mutated.
pub(super) fn require_assignment_editor(
    state: &State,
    course: CourseId,
    actor: UserId,
) -> Result<(), StoreError> {
    if !state.courses.contains_key(&course)
        || super::entitlement::current_course_role(state, course, actor)
            != Some(CourseMembershipRole::Instructor)
    {
        return Err(StoreError::NotFound);
    }
    Ok(())
}

/// Prevents a revision from retiring an immutable run item while its issued
/// attempt is still answerable. The attempt is tied to the run's immutable
/// item snapshot, not to the mutable assignment definition or an effective
/// policy receipt. That preserves the S3 receipt boundary while retaining the
/// evidence needed to protect an active learner interaction.
pub(super) fn retirement_would_orphan_active_attempt(
    state: &State,
    previous: &AssignmentRecord,
    replacement: &AssignmentRecord,
) -> Result<bool, StoreError> {
    let retired_items = previous
        .items
        .iter()
        .filter(|item| item.delivery_state == question_model::AssignmentDeliveryState::Active)
        .map(|item| item.id)
        .chain(
            previous
                .selection_groups
                .iter()
                .flat_map(|group| group.candidates.iter())
                .filter(|candidate| {
                    candidate.delivery_state == question_model::AssignmentDeliveryState::Active
                })
                .map(|candidate| candidate.id),
        )
        .filter(|item| !assignment_item_is_active(replacement, *item))
        .collect::<std::collections::BTreeSet<_>>();
    if retired_items.is_empty() {
        return Ok(false);
    }

    state
        .attempts
        .values()
        .filter(|attempt| projected_attempt(state, attempt).status == AttemptStatus::InProgress)
        .try_fold(false, |blocked, attempt| {
            let run = state.runs.get(&attempt.run).ok_or_else(|| {
                StoreError::Unavailable("active attempt is missing its run".to_string())
            })?;
            let enrollment = state.enrollments.get(&run.enrollment).ok_or_else(|| {
                StoreError::Unavailable("active attempt run is missing its enrollment".to_string())
            })?;
            if enrollment.assignment != previous.id {
                return Ok(blocked);
            }
            let run_items = state.run_items.get(&run.id).ok_or_else(|| {
                StoreError::Unavailable(
                    "active attempt run is missing its immutable items".to_string(),
                )
            })?;
            let run_item = run_items
                .iter()
                .find(|item| item.issued_position == attempt.assignment_position)
                .ok_or_else(|| {
                    StoreError::Unavailable(
                        "active attempt has no immutable assignment item".to_string(),
                    )
                })?;
            Ok(blocked || retired_items.contains(&run_item.assignment_item))
        })
}

fn assignment_item_is_active(
    assignment: &AssignmentRecord,
    target: question_model::AssignmentItemId,
) -> bool {
    assignment.items.iter().any(|item| {
        item.id == target && item.delivery_state == question_model::AssignmentDeliveryState::Active
    }) || assignment
        .selection_groups
        .iter()
        .flat_map(|group| group.candidates.iter())
        .any(|candidate| {
            candidate.id == target
                && candidate.delivery_state == question_model::AssignmentDeliveryState::Active
        })
}

/// Loads one enrollment inside an already authorized state operation.
pub(super) fn enrollment_record(
    state: &State,
    enrollment: EnrollmentId,
) -> Result<AssignmentEnrollment, StoreError> {
    state
        .enrollments
        .get(&enrollment)
        .cloned()
        .ok_or(StoreError::NotFound)
}

/// Loads the assignment whose grade policy drives summary projection.
pub(super) fn assignment_record(
    state: &State,
    assignment: AssignmentId,
) -> Result<AssignmentRecord, StoreError> {
    state
        .assignments
        .get(&assignment)
        .cloned()
        .ok_or(StoreError::NotFound)
}
