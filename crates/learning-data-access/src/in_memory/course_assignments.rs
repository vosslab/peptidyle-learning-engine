use async_trait::async_trait;

use super::*;

#[async_trait]
impl crate::CourseAssignmentStore for MemoryStore {
    async fn create_assignment_impl(
        &self,
        context: TenantContext,
        command: CreateAssignmentCommand,
    ) -> Result<StoredAssignment, StoreError> {
        let CreateAssignmentCommand {
            actor,
            assignment,
            base_policy,
        } = command;
        ensure_tenant(context, assignment.tenant)?;
        if assignment.lifecycle != question_model::AssignmentLifecycle::Draft {
            return Err(StoreError::InvalidRecord(
                "new assignments must begin in the draft lifecycle".to_string(),
            ));
        }
        validate_assignment(&assignment)?;
        let mut state = self.write_state()?;
        require_assignment_editor(&state, context, assignment.course_id, actor)?;
        let key = (assignment.tenant, assignment.id);
        if state.assignments.contains_key(&key) {
            return Err(StoreError::AlreadyExists);
        }
        validate_memory_assignment_references(&state, context, &assignment)?;
        let course_term = state
            .courses
            .get(&(assignment.tenant, assignment.course_id))
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
        let snapshot = state.clone();
        let stored = StoredAssignment {
            record: assignment,
            revision: AssignmentRevision::INITIAL,
            base_policy,
            scoring_generation: ScoringGeneration::INITIAL,
            scoring_status: ScoringStatus::Current,
        };
        super::navigation_references::ensure_assignment_reference(
            &mut state,
            stored.record.tenant,
            stored.record.id,
        )?;
        state.assignments.insert(key, stored.record.clone());
        state.assignment_revisions.insert(key, stored.revision);
        state.assignment_base_policy.insert(
            key,
            StoredBaseAssignmentPolicy {
                tenant: stored.record.tenant,
                course: stored.record.course_id,
                assignment: stored.record.id,
                policy: base_policy,
                revision: stored.revision,
            },
        );
        state
            .assignment_scoring
            .insert(key, (stored.scoring_generation, stored.scoring_status));
        if let Err(error) = super::course_gradebook::advance_course_grade_scheme_revision(
            &mut state,
            stored.record.tenant,
            stored.record.course_id,
        ) {
            *state = snapshot;
            return Err(error);
        }
        Ok(stored)
    }
    async fn replace_assignment_impl(
        &self,
        context: TenantContext,
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
        require_assignment_editor(&state, context, course, actor)?;
        let key = (context.tenant_id(), assignment);
        let existing = state
            .assignments
            .get(&key)
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
        let next_revision = current.next()?;
        crate::ensure_assignment_update_preserves_references(&existing, &update)?;
        let assignment = AssignmentRecord {
            id: assignment,
            tenant: context.tenant_id(),
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
        validate_memory_assignment_references(&state, context, &assignment)?;
        let previous = state.assignments.get(&key).ok_or(StoreError::NotFound)?;
        let course_grade_projection_changed = previous.title != assignment.title;
        if retirement_would_orphan_active_attempt(
            &state,
            context.tenant_id(),
            previous,
            &assignment,
        )? {
            return Err(StoreError::Conflict);
        }
        let scoring_changed = assignment_scoring_changed(previous, &assignment);
        let (generation, _) = state
            .assignment_scoring
            .get(&key)
            .copied()
            .ok_or(StoreError::NotFound)?;
        let scoring_generation = if scoring_changed {
            generation.next().ok_or(StoreError::Conflict)?
        } else {
            generation
        };
        let scoring_status =
            if scoring_changed && memory_assignment_has_results(&state, &assignment) {
                ScoringStatus::Recalculating
            } else {
                ScoringStatus::Current
            };
        if scoring_status == ScoringStatus::Recalculating {
            let job = crate::JobId::generate()?;
            let queued = StoredJob {
                tenant: assignment.tenant,
                payload: crate::JobPayload::RecalculateAssignment {
                    assignment: assignment.id,
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
                *state = snapshot;
                return Err(StoreError::Conflict);
            }
        }
        let stored = StoredAssignment {
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
        state.assignments.insert(key, stored.record.clone());
        state.assignment_revisions.insert(key, stored.revision);
        if let Err(error) = super::rehearsal::invalidate_assignment_rehearsals(
            &mut state,
            context.tenant_id(),
            stored.record.id,
        ) {
            *state = snapshot;
            return Err(error);
        }
        state
            .assignment_scoring
            .insert(key, (stored.scoring_generation, stored.scoring_status));
        if course_grade_projection_changed
            && let Err(error) = super::course_gradebook::advance_course_grade_scheme_revision(
                &mut state,
                stored.record.tenant,
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
            context.tenant_id(),
            course,
            stored.record.id,
        ) {
            *state = snapshot;
            return Err(error);
        }
        Ok(stored)
    }
    async fn replace_assignment_fixed_item_impl(
        &self,
        context: TenantContext,
        command: ReplaceAssignmentFixedItemCommand,
    ) -> Result<StoredAssignment, StoreError> {
        let mut state = self.write_state()?;
        let snapshot = state.clone();
        require_assignment_editor(&state, context, command.course, command.actor)?;
        let key = (context.tenant_id(), command.assignment);
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
        validate_memory_assignment_references(&state, context, &replacement)?;
        let stored = StoredAssignment {
            record: replacement,
            revision: revision.next()?,
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
        state.assignments.insert(key, stored.record.clone());
        state.assignment_revisions.insert(key, stored.revision);
        if let Err(error) = super::rehearsal::invalidate_assignment_rehearsals(
            &mut state,
            context.tenant_id(),
            stored.record.id,
        ) {
            *state = snapshot;
            return Err(error);
        }
        Ok(stored)
    }
    async fn add_assignment_fixed_item_impl(
        &self,
        context: TenantContext,
        command: AddAssignmentFixedItemCommand,
    ) -> Result<StoredAssignment, StoreError> {
        let mut state = self.write_state()?;
        let snapshot = state.clone();
        require_assignment_editor(&state, context, command.course, command.actor)?;
        let key = (context.tenant_id(), command.assignment);
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
        validate_memory_assignment_references(&state, context, &replacement)?;
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
            revision: revision.next()?,
            base_policy,
            scoring_generation: generation,
            scoring_status: status,
        };
        state.assignments.insert(key, stored.record.clone());
        state.assignment_revisions.insert(key, stored.revision);
        if let Err(error) = super::rehearsal::invalidate_assignment_rehearsals(
            &mut state,
            context.tenant_id(),
            stored.record.id,
        ) {
            *state = snapshot;
            return Err(error);
        }
        Ok(stored)
    }
    async fn remove_assignment_fixed_item_impl(
        &self,
        context: TenantContext,
        command: RemoveAssignmentFixedItemCommand,
    ) -> Result<StoredAssignment, StoreError> {
        let mut state = self.write_state()?;
        let snapshot = state.clone();
        require_assignment_editor(&state, context, command.course, command.actor)?;
        let key = (context.tenant_id(), command.assignment);
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
            revision: revision.next()?,
            base_policy,
            scoring_generation: generation,
            scoring_status: status,
        };
        state.assignments.insert(key, stored.record.clone());
        state.assignment_revisions.insert(key, stored.revision);
        if let Err(error) = super::rehearsal::invalidate_assignment_rehearsals(
            &mut state,
            context.tenant_id(),
            stored.record.id,
        ) {
            *state = snapshot;
            return Err(error);
        }
        Ok(stored)
    }
    async fn delete_and_regrade_assignment_item_impl(
        &self,
        context: TenantContext,
        command: DeleteAndRegradeAssignmentItemCommand,
    ) -> Result<StoredAssignment, StoreError> {
        {
            let state = self.read_state()?;
            require_assignment_editor(&state, context, command.course, command.actor)?;
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
        context: TenantContext,
        assignment: AssignmentId,
    ) -> Result<Option<StoredAssignment>, StoreError> {
        let state = self.read_state()?;
        let key = (context.tenant_id(), assignment);
        let Some(record) = state.assignments.get(&key).cloned() else {
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
        context: TenantContext,
        assignment: AssignmentId,
    ) -> Result<Option<AssignmentRecord>, StoreError> {
        let state = self.read_state()?;
        let Some(record) = state
            .assignments
            .get(&(context.tenant_id(), assignment))
            .cloned()
        else {
            return Ok(None);
        };
        Ok(Some(record))
    }
    async fn list_assignments_impl(
        &self,
        context: TenantContext,
        course: CourseId,
        page: PageRequest,
    ) -> Result<Page<AssignmentRecord>, StoreError> {
        let state = self.read_state()?;
        if !state.courses.contains_key(&(context.tenant_id(), course)) {
            return Err(StoreError::NotFound);
        }
        let records = state
            .assignments
            .iter()
            .filter(|((tenant, _), record)| {
                *tenant == context.tenant_id() && record.course_id == course
            })
            .map(|((_, assignment), record)| (assignment.to_string(), record.clone()))
            .collect();
        Ok(page_records(records, &page))
    }
    async fn get_enrollment_impl(
        &self,
        context: TenantContext,
        enrollment: EnrollmentId,
    ) -> Result<Option<AssignmentEnrollment>, StoreError> {
        let state = self.read_state()?;
        let Some(record) = state
            .enrollments
            .get(&(context.tenant_id(), enrollment))
            .cloned()
        else {
            return Ok(None);
        };
        let assignment = assignment_record(&state, context.tenant_id(), record.assignment)?;
        if !course_records_accessible(&state, context.tenant_id(), assignment.course_id) {
            return Ok(None);
        }
        Ok(Some(record))
    }
}

/// Verifies the command-local actor against current direct Instructor
/// authority before an assignment definition is read or mutated.
fn require_assignment_editor(
    state: &State,
    context: TenantContext,
    course: CourseId,
    actor: UserId,
) -> Result<(), StoreError> {
    let tenant = context.tenant_id();
    if !state.courses.contains_key(&(tenant, course))
        || super::entitlement::current_course_role(state, tenant, course, actor)
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
fn retirement_would_orphan_active_attempt(
    state: &State,
    tenant: TenantId,
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
        .filter(|attempt| attempt.tenant == tenant)
        .filter(|attempt| {
            projected_attempt(state, tenant, attempt).status == AttemptStatus::InProgress
        })
        .try_fold(false, |blocked, attempt| {
            let run = state.runs.get(&(tenant, attempt.run)).ok_or_else(|| {
                StoreError::Unavailable("active attempt is missing its run".to_string())
            })?;
            let enrollment = state
                .enrollments
                .get(&(tenant, run.enrollment))
                .ok_or_else(|| {
                    StoreError::Unavailable(
                        "active attempt run is missing its enrollment".to_string(),
                    )
                })?;
            if enrollment.assignment != previous.id {
                return Ok(blocked);
            }
            let run_items = state.run_items.get(&(tenant, run.id)).ok_or_else(|| {
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

/// Loads one enrollment inside an already tenant-scoped state operation.
pub(super) fn enrollment_record(
    state: &State,
    tenant: TenantId,
    enrollment: EnrollmentId,
) -> Result<AssignmentEnrollment, StoreError> {
    state
        .enrollments
        .get(&(tenant, enrollment))
        .cloned()
        .ok_or(StoreError::NotFound)
}

/// Loads the assignment whose grade policy drives summary projection.
pub(super) fn assignment_record(
    state: &State,
    tenant: TenantId,
    assignment: AssignmentId,
) -> Result<AssignmentRecord, StoreError> {
    state
        .assignments
        .get(&(tenant, assignment))
        .cloned()
        .ok_or(StoreError::NotFound)
}
