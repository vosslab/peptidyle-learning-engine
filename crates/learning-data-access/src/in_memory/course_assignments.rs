use async_trait::async_trait;

use super::*;

#[async_trait]
impl crate::CourseAssignmentStore for MemoryStore {
    async fn create_assignment_with_timing_impl(
        &self,
        context: TenantContext,
        assignment: AssignmentRecord,
        assignment_timing: question_model::AssignmentRunTiming,
    ) -> Result<StoredAssignment, StoreError> {
        ensure_tenant(context, assignment.tenant)?;
        validate_assignment(&assignment)?;
        validate_assignment_timing(AssignmentTimingPolicy {
            time_limit_seconds: assignment_timing.time_limit_seconds,
            ..AssignmentTimingPolicy::default()
        })?;
        let mut state = self.write_state()?;
        let key = (assignment.tenant, assignment.id);
        if state.assignments.contains_key(&key) {
            return Err(StoreError::AlreadyExists);
        }
        validate_memory_assignment_references(&state, context, &assignment)?;
        let stored = StoredAssignment {
            record: assignment,
            revision: AssignmentRevision::INITIAL,
            assignment_timing,
            scoring_generation: ScoringGeneration::INITIAL,
            scoring_status: ScoringStatus::Current,
        };
        let snapshot = state.clone();
        super::navigation_references::ensure_assignment_public_id(
            &mut state,
            stored.record.tenant,
            stored.record.id,
        )?;
        state.assignments.insert(key, stored.record.clone());
        state.assignment_revisions.insert(key, stored.revision);
        state.assignment_timing.insert(
            key,
            AssignmentTimingPolicy {
                time_limit_seconds: assignment_timing.time_limit_seconds,
                ..AssignmentTimingPolicy::default()
            },
        );
        state
            .assignment_scoring
            .insert(key, (stored.scoring_generation, stored.scoring_status));
        if let Err(error) =
            super::course_roster::reconcile_new_assignment(&mut state, &stored.record)
        {
            *state = snapshot;
            return Err(error);
        }
        Ok(stored)
    }
    async fn replace_assignment_with_timing_impl(
        &self,
        context: TenantContext,
        course: CourseId,
        assignment: AssignmentId,
        expected_revision: AssignmentRevision,
        update: AssignmentEditorUpdate,
    ) -> Result<StoredAssignment, StoreError> {
        let mut state = self.write_state()?;
        let snapshot = state.clone();
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
        let previous_timing = state
            .assignment_timing
            .get(&key)
            .copied()
            .ok_or(StoreError::NotFound)?;
        validate_assignment_timing(AssignmentTimingPolicy {
            time_limit_seconds: update.assignment_timing.time_limit_seconds,
            ..AssignmentTimingPolicy::default()
        })?;
        crate::ensure_assignment_update_preserves_references(&existing, &update.assignment)?;
        let assignment = AssignmentRecord {
            id: assignment,
            tenant: context.tenant_id(),
            course_id: course,
            title: update.assignment.title,
            items: update.assignment.items,
            selection_groups: update.assignment.selection_groups,
            policies: update.assignment.policies,
        };
        validate_assignment(&assignment)?;
        validate_memory_assignment_references(&state, context, &assignment)?;
        let previous = state.assignments.get(&key).ok_or(StoreError::NotFound)?;
        validate_memory_assignment_content_lock(&state, previous, &assignment)?;
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
            assignment_timing: update.assignment_timing,
            scoring_generation,
            scoring_status,
        };
        state.assignments.insert(key, stored.record.clone());
        state.assignment_revisions.insert(key, stored.revision);
        if previous_timing.time_limit_seconds != update.assignment_timing.time_limit_seconds {
            if let Err(error) = apply_memory_assignment_timing_update(
                &mut state,
                context.tenant_id(),
                stored.record.id,
                Some(AssignmentTimingPolicy {
                    time_limit_seconds: update.assignment_timing.time_limit_seconds,
                    ..previous_timing
                }),
            ) {
                *state = snapshot;
                return Err(error);
            }
            state.assignment_timing.insert(
                key,
                AssignmentTimingPolicy {
                    time_limit_seconds: update.assignment_timing.time_limit_seconds,
                    ..previous_timing
                },
            );
        }
        state
            .assignment_scoring
            .insert(key, (stored.scoring_generation, stored.scoring_status));
        Ok(stored)
    }
    async fn replace_assignment_fixed_item_impl(
        &self,
        context: TenantContext,
        command: ReplaceAssignmentFixedItemCommand,
    ) -> Result<StoredAssignment, StoreError> {
        let mut state = self.write_state()?;
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
            assignment_timing: question_model::AssignmentRunTiming {
                time_limit_seconds: state
                    .assignment_timing
                    .get(&key)
                    .copied()
                    .ok_or(StoreError::NotFound)?
                    .time_limit_seconds,
            },
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
        Ok(stored)
    }
    async fn add_assignment_fixed_item_impl(
        &self,
        context: TenantContext,
        command: AddAssignmentFixedItemCommand,
    ) -> Result<StoredAssignment, StoreError> {
        let mut state = self.write_state()?;
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
        let timing = state
            .assignment_timing
            .get(&key)
            .copied()
            .ok_or(StoreError::NotFound)?;
        let (generation, status) = state
            .assignment_scoring
            .get(&key)
            .copied()
            .ok_or(StoreError::NotFound)?;
        let stored = StoredAssignment {
            record: replacement,
            revision: revision.next()?,
            assignment_timing: question_model::AssignmentRunTiming {
                time_limit_seconds: timing.time_limit_seconds,
            },
            scoring_generation: generation,
            scoring_status: status,
        };
        state.assignments.insert(key, stored.record.clone());
        state.assignment_revisions.insert(key, stored.revision);
        Ok(stored)
    }
    async fn remove_assignment_fixed_item_impl(
        &self,
        context: TenantContext,
        command: RemoveAssignmentFixedItemCommand,
    ) -> Result<StoredAssignment, StoreError> {
        let mut state = self.write_state()?;
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
        let timing = state
            .assignment_timing
            .get(&key)
            .copied()
            .ok_or(StoreError::NotFound)?;
        let (generation, status) = state
            .assignment_scoring
            .get(&key)
            .copied()
            .ok_or(StoreError::NotFound)?;
        let stored = StoredAssignment {
            record: replacement,
            revision: revision.next()?,
            assignment_timing: question_model::AssignmentRunTiming {
                time_limit_seconds: timing.time_limit_seconds,
            },
            scoring_generation: generation,
            scoring_status: status,
        };
        state.assignments.insert(key, stored.record.clone());
        state.assignment_revisions.insert(key, stored.revision);
        Ok(stored)
    }
    async fn delete_and_regrade_assignment_item_impl(
        &self,
        context: TenantContext,
        command: DeleteAndRegradeAssignmentItemCommand,
    ) -> Result<StoredAssignment, StoreError> {
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
        crate::CourseAssignmentStore::replace_assignment_with_timing_impl(
            self,
            context,
            command.course,
            command.assignment,
            command.expected_revision,
            AssignmentEditorUpdate {
                assignment: update,
                assignment_timing: stored.assignment_timing,
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
            assignment_timing: question_model::AssignmentRunTiming {
                time_limit_seconds: state
                    .assignment_timing
                    .get(&key)
                    .copied()
                    .ok_or(StoreError::NotFound)?
                    .time_limit_seconds,
            },
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
    async fn create_enrollment_impl(
        &self,
        context: TenantContext,
        enrollment: AssignmentEnrollment,
    ) -> Result<(), StoreError> {
        ensure_tenant(context, enrollment.tenant)?;
        let mut state = self.write_state()?;
        let assignment = state
            .assignments
            .get(&(enrollment.tenant, enrollment.assignment))
            .ok_or_else(|| {
                StoreError::InvalidRecord("enrollment references a missing assignment".to_string())
            })?;
        require_course_records_accessible(&state, context.tenant_id(), assignment.course_id)?;
        let course = state
            .courses
            .get(&(enrollment.tenant, assignment.course_id))
            .ok_or(StoreError::NotFound)?;
        if course.role_for(enrollment.user) != Some(CourseMembershipRole::Student) {
            return Err(StoreError::InvalidRecord(
                "enrollment user must be a student member of the assignment course".to_string(),
            ));
        }
        let key = (enrollment.tenant, enrollment.id);
        if state.enrollments.contains_key(&key) {
            return Err(StoreError::AlreadyExists);
        }
        if state.enrollments.values().any(|existing| {
            existing.tenant == enrollment.tenant
                && existing.assignment == enrollment.assignment
                && existing.user == enrollment.user
        }) {
            return Err(StoreError::AlreadyExists);
        }
        state.summaries.insert(
            key,
            StudentAssignmentSummary::empty(enrollment.tenant, enrollment.id),
        );
        state.enrollments.insert(key, enrollment);
        Ok(())
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
