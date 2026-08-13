use async_trait::async_trait;

use super::*;

#[async_trait]
impl crate::AssignmentPolicyStore for MemoryStore {
    async fn get_assignment_timing_impl(
        &self,
        context: TenantContext,
        assignment: AssignmentId,
    ) -> Result<Option<StoredAssignmentTiming>, StoreError> {
        let state = self.read_state()?;
        let key = (context.tenant_id(), assignment);
        let Some(record) = state.assignments.get(&key) else {
            return Ok(None);
        };
        let policy = state.assignment_timing.get(&key).copied().ok_or_else(|| {
            StoreError::Unavailable(
                "assignment timing policy is missing from memory state".to_string(),
            )
        })?;
        let revision = state
            .assignment_revisions
            .get(&key)
            .copied()
            .ok_or(StoreError::NotFound)?;
        Ok(Some(StoredAssignmentTiming {
            tenant: context.tenant_id(),
            course: record.course_id,
            assignment,
            policy,
            revision,
        }))
    }
    async fn update_assignment_timing_impl(
        &self,
        context: TenantContext,
        command: UpdateAssignmentTimingCommand,
    ) -> Result<StoredAssignmentTiming, StoreError> {
        validate_assignment_timing(command.policy)?;
        let tenant = context.tenant_id();
        let key = (tenant, command.assignment);
        let mut state = self.write_state()?;
        let assignment = state
            .assignments
            .get(&key)
            .cloned()
            .ok_or(StoreError::NotFound)?;
        if assignment.course_id != command.course {
            return Err(StoreError::NotFound);
        }
        require_course_records_accessible(&state, tenant, command.course)?;
        let course = state
            .courses
            .get(&(tenant, command.course))
            .ok_or(StoreError::NotFound)?;
        if course.role_for(command.actor) != Some(CourseMembershipRole::Instructor) {
            return Err(StoreError::NotFound);
        }
        let current_revision = state
            .assignment_revisions
            .get(&key)
            .copied()
            .ok_or(StoreError::NotFound)?;
        let current_policy = state
            .assignment_timing
            .get(&key)
            .copied()
            .ok_or(StoreError::NotFound)?;
        if current_policy == command.policy {
            return Ok(StoredAssignmentTiming {
                tenant,
                course: command.course,
                assignment: command.assignment,
                policy: current_policy,
                revision: current_revision,
            });
        }
        if current_revision != command.expected_revision {
            return Err(StoreError::Conflict);
        }
        let revision = current_revision.next()?;
        apply_memory_assignment_timing_update(
            &mut state,
            tenant,
            command.assignment,
            Some(command.policy),
        )?;
        state.assignment_timing.insert(key, command.policy);
        state.assignment_revisions.insert(key, revision);
        Ok(StoredAssignmentTiming {
            tenant,
            course: command.course,
            assignment: command.assignment,
            policy: command.policy,
            revision,
        })
    }
    async fn set_assignment_policy_exception_impl(
        &self,
        context: TenantContext,
        command: SetAssignmentPolicyExceptionCommand,
    ) -> Result<StoredAssignmentPolicyException, StoreError> {
        validate_assignment_policy_exception(&command.exception)?;
        let tenant = context.tenant_id();
        let assignment_key = (tenant, command.assignment);
        let exception_key = (tenant, command.assignment, command.exception.target);
        let mut state = self.write_state()?;
        let assignment = state
            .assignments
            .get(&assignment_key)
            .cloned()
            .ok_or(StoreError::NotFound)?;
        if assignment.course_id != command.course {
            return Err(StoreError::NotFound);
        }
        require_course_records_accessible(&state, tenant, command.course)?;
        let course = state
            .courses
            .get(&(tenant, command.course))
            .ok_or(StoreError::NotFound)?;
        if course.role_for(command.actor) != Some(CourseMembershipRole::Instructor) {
            return Err(StoreError::NotFound);
        }
        match command.exception.target {
            AssignmentPolicyExceptionTarget::Student(student) => {
                if !state.enrollments.values().any(|enrollment| {
                    enrollment.tenant == tenant
                        && enrollment.assignment == command.assignment
                        && enrollment.student == student
                }) {
                    return Err(StoreError::NotFound);
                }
            }
            AssignmentPolicyExceptionTarget::CourseGroup(group) => {
                if state
                    .course_groups
                    .get(&(tenant, group))
                    .is_none_or(|record| record.course != command.course)
                {
                    return Err(StoreError::NotFound);
                }
            }
        }
        if state.assignment_policy_exceptions.iter().any(
            |((record_tenant, record_assignment, target), exception)| {
                *record_tenant == tenant
                    && *record_assignment == command.assignment
                    && *target != command.exception.target
                    && exception.id == command.exception.id
            },
        ) {
            return Err(StoreError::Conflict);
        }
        let current_revision = state
            .assignment_revisions
            .get(&assignment_key)
            .copied()
            .ok_or(StoreError::NotFound)?;
        if let Some(existing) = state.assignment_policy_exceptions.get(&exception_key)
            && existing == &command.exception
        {
            return Ok(StoredAssignmentPolicyException {
                exception: existing.clone(),
                assignment_revision: current_revision,
            });
        }
        if current_revision != command.expected_revision {
            return Err(StoreError::Conflict);
        }
        if state
            .assignment_policy_exceptions
            .get(&exception_key)
            .is_some_and(|existing| existing.id != command.exception.id)
        {
            return Err(StoreError::Conflict);
        }
        let revision = current_revision.next()?;
        let snapshot = state.clone();
        state
            .assignment_policy_exceptions
            .insert(exception_key, command.exception.clone());
        if let Err(error) =
            apply_memory_assignment_timing_update(&mut state, tenant, command.assignment, None)
        {
            *state = snapshot;
            return Err(error);
        }
        state.assignment_revisions.insert(assignment_key, revision);
        Ok(StoredAssignmentPolicyException {
            exception: command.exception,
            assignment_revision: revision,
        })
    }
    async fn delete_assignment_policy_exception_impl(
        &self,
        context: TenantContext,
        command: DeleteAssignmentPolicyExceptionCommand,
    ) -> Result<AssignmentRevision, StoreError> {
        let tenant = context.tenant_id();
        let assignment_key = (tenant, command.assignment);
        let mut state = self.write_state()?;
        let assignment = state
            .assignments
            .get(&assignment_key)
            .cloned()
            .ok_or(StoreError::NotFound)?;
        if assignment.course_id != command.course {
            return Err(StoreError::NotFound);
        }
        require_course_records_accessible(&state, tenant, command.course)?;
        let course = state
            .courses
            .get(&(tenant, command.course))
            .ok_or(StoreError::NotFound)?;
        if course.role_for(command.actor) != Some(CourseMembershipRole::Instructor) {
            return Err(StoreError::NotFound);
        }
        let current_revision = state
            .assignment_revisions
            .get(&assignment_key)
            .copied()
            .ok_or(StoreError::NotFound)?;
        if current_revision != command.expected_revision {
            return Err(StoreError::Conflict);
        }
        let exception_key = state
            .assignment_policy_exceptions
            .iter()
            .find_map(|(key @ (record_tenant, record_assignment, _), exception)| {
                (*record_tenant == tenant
                    && *record_assignment == command.assignment
                    && exception.id == command.exception)
                    .then_some(*key)
            })
            .ok_or(StoreError::NotFound)?;
        let revision = current_revision.next()?;
        let snapshot = state.clone();
        state.assignment_policy_exceptions.remove(&exception_key);
        if let Err(error) =
            apply_memory_assignment_timing_update(&mut state, tenant, command.assignment, None)
        {
            *state = snapshot;
            return Err(error);
        }
        state.assignment_revisions.insert(assignment_key, revision);
        Ok(revision)
    }
    async fn get_assignment_policy_exception_impl(
        &self,
        context: TenantContext,
        assignment: AssignmentId,
        exception: AssignmentPolicyExceptionId,
    ) -> Result<Option<StoredAssignmentPolicyException>, StoreError> {
        let state = self.read_state()?;
        let tenant = context.tenant_id();
        let record = state.assignment_policy_exceptions.iter().find_map(
            |((record_tenant, record_assignment, _), record)| {
                (*record_tenant == tenant
                    && *record_assignment == assignment
                    && record.id == exception)
                    .then_some(record.clone())
            },
        );
        let Some(exception) = record else {
            return Ok(None);
        };
        Ok(Some(StoredAssignmentPolicyException {
            exception,
            assignment_revision: state
                .assignment_revisions
                .get(&(tenant, assignment))
                .copied()
                .ok_or(StoreError::NotFound)?,
        }))
    }
    async fn resolve_assignment_timing_impl(
        &self,
        context: TenantContext,
        assignment: AssignmentId,
        student: StudentId,
    ) -> Result<Option<ResolvedAssignmentTiming>, StoreError> {
        let state = self.read_state()?;
        let tenant = context.tenant_id();
        let Some(record) = state.assignments.get(&(tenant, assignment)) else {
            return Ok(None);
        };
        let Some(enrollment) = state.enrollments.values().find(|enrollment| {
            enrollment.tenant == tenant
                && enrollment.assignment == assignment
                && enrollment.student == student
        }) else {
            return Ok(None);
        };
        let resolved =
            memory_resolved_assignment_policy(&state, tenant, assignment, enrollment, None)?;
        Ok(Some(ResolvedAssignmentTiming {
            tenant,
            course: record.course_id,
            assignment,
            student,
            policy: resolved.policy,
            contributors: resolved.contributors,
            revision: state
                .assignment_revisions
                .get(&(tenant, assignment))
                .copied()
                .ok_or(StoreError::NotFound)?,
        }))
    }
    async fn get_attempt_resolved_timing_impl(
        &self,
        context: TenantContext,
        attempt: QuestionAttemptId,
    ) -> Result<Option<ResolvedAttemptTiming>, StoreError> {
        let state = self.read_state()?;
        let tenant = context.tenant_id();
        let Some(resolution) = state.attempt_timing_resolution.get(&(tenant, attempt)) else {
            return Ok(None);
        };
        Ok(Some(ResolvedAttemptTiming {
            attempt,
            policy: resolution.policy,
            contributors: resolution.contributors.clone(),
        }))
    }
}

pub(super) fn validate_assignment_position(
    run_items: &[AssignmentRunItem],
    command: &IssueQuestionAttemptCommand,
) -> Result<(), StoreError> {
    let expected = run_items
        .iter()
        .find(|item| item.issued_position == command.assignment_position)
        .ok_or_else(|| {
            StoreError::InvalidRecord("question position is outside the assignment".to_string())
        })?;
    if expected.reference.problem != command.problem
        || expected.reference.version != command.question_version
    {
        return Err(StoreError::InvalidRecord(
            "question identity does not match its assignment position".to_string(),
        ));
    }
    Ok(())
}

pub(super) fn validate_memory_assignment_references(
    state: &State,
    context: TenantContext,
    assignment: &AssignmentRecord,
) -> Result<(), StoreError> {
    if !state
        .courses
        .contains_key(&(assignment.tenant, assignment.course_id))
    {
        return Err(StoreError::InvalidRecord(
            "assignment references a missing course".to_string(),
        ));
    }
    for reference in assignment.references() {
        let assignable = state
            .published
            .get(&(reference.problem, reference.version))
            .is_some_and(|record| {
                record.lifecycle.is_assignable()
                    && catalog_record_visible(state, context.tenant_id(), record)
            });
        if !assignable {
            return Err(StoreError::InvalidRecord(format!(
                "assignment references a missing, hidden, or inactive published version {}/{}",
                reference.problem, reference.version
            )));
        }
    }
    Ok(())
}

pub(super) fn validate_memory_assignment_content_lock(
    state: &State,
    previous: &AssignmentRecord,
    replacement: &AssignmentRecord,
) -> Result<(), StoreError> {
    let has_run = state.runs.values().any(|run| {
        state
            .enrollments
            .get(&(run.tenant, run.enrollment))
            .is_some_and(|enrollment| {
                enrollment.tenant == previous.tenant && enrollment.assignment == previous.id
            })
    });
    if !has_run {
        return Ok(());
    }
    let retirement_blocked = previous.items.iter().any(|item| {
        item.delivery_state == question_model::AssignmentDeliveryState::Active
            && replacement.items.iter().any(|candidate| {
                candidate.id == item.id
                    && candidate.delivery_state == question_model::AssignmentDeliveryState::Retired
            })
            && memory_item_has_active_attempt(state, previous, item.id)
    }) || previous.selection_groups.iter().any(|group| {
        group.candidates.iter().any(|candidate| {
            candidate.delivery_state == question_model::AssignmentDeliveryState::Active
                && replacement
                    .selection_groups
                    .iter()
                    .any(|replacement_group| {
                        replacement_group.candidates.iter().any(|replacement| {
                            replacement.id == candidate.id
                                && replacement.delivery_state
                                    == question_model::AssignmentDeliveryState::Retired
                        })
                    })
                && memory_item_has_active_attempt(state, previous, candidate.id)
        })
    });
    if retirement_blocked {
        return Err(StoreError::Conflict);
    }
    let previous_items = previous
        .items
        .iter()
        .map(|item| (item.id, item.reference))
        .collect::<BTreeMap<_, _>>();
    let replacement_items = replacement
        .items
        .iter()
        .map(|item| (item.id, item.reference))
        .collect::<BTreeMap<_, _>>();
    let previous_groups = previous
        .selection_groups
        .iter()
        .map(|group| {
            (
                group.id,
                group
                    .candidates
                    .iter()
                    .map(|candidate| (candidate.id, candidate.reference))
                    .collect::<BTreeMap<_, _>>(),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let replacement_groups = replacement
        .selection_groups
        .iter()
        .map(|group| {
            (
                group.id,
                group
                    .candidates
                    .iter()
                    .map(|candidate| (candidate.id, candidate.reference))
                    .collect::<BTreeMap<_, _>>(),
            )
        })
        .collect::<BTreeMap<_, _>>();
    if previous_items != replacement_items || previous_groups != replacement_groups {
        return Err(StoreError::Conflict);
    }
    Ok(())
}

pub(super) fn memory_item_has_active_attempt(
    state: &State,
    assignment: &AssignmentRecord,
    assignment_item: question_model::AssignmentItemId,
) -> bool {
    state.attempts.values().any(|base| {
        if base.tenant != assignment.tenant
            || projected_attempt(state, assignment.tenant, base).status != AttemptStatus::InProgress
        {
            return false;
        }
        let Some(run) = state.runs.get(&(assignment.tenant, base.run)) else {
            return false;
        };
        let belongs_to_assignment = state
            .enrollments
            .get(&(assignment.tenant, run.enrollment))
            .is_some_and(|enrollment| enrollment.assignment == assignment.id);
        belongs_to_assignment
            && state
                .run_items
                .get(&(assignment.tenant, run.id))
                .and_then(|items| {
                    items
                        .iter()
                        .find(|item| item.issued_position == base.assignment_position)
                })
                .is_some_and(|item| item.assignment_item == assignment_item)
    })
}

pub(super) fn memory_assignment_has_results(state: &State, assignment: &AssignmentRecord) -> bool {
    state.submissions.values().any(|submission| {
        let attempt = &submission.record.attempt;
        attempt.result.is_some()
            && state
                .runs
                .get(&(attempt.tenant, attempt.run))
                .and_then(|run| state.enrollments.get(&(run.tenant, run.enrollment)))
                .is_some_and(|enrollment| enrollment.assignment == assignment.id)
    })
}

pub(super) fn resolved_memory_attempt_timing(
    policy: AssignmentTimingPolicy,
    run: &AssignmentRun,
    authored_deadline: Option<ActivityTimestamp>,
    authored_grace_seconds: u32,
) -> Result<(Option<ActivityTimestamp>, u32, Option<ActivityTimestamp>), StoreError> {
    let mut resolved = authored_deadline.map(|deadline| (deadline, authored_grace_seconds));
    let mut consider = |deadline: ActivityTimestamp, grace_seconds: u32| {
        if resolved.is_none_or(|current| (deadline, grace_seconds) < current) {
            resolved = Some((deadline, grace_seconds));
        }
    };
    if let Some(seconds) = policy.time_limit_seconds {
        consider(
            add_seconds(run.started_at, seconds, "assignment time limit")?,
            0,
        );
    }
    if policy.late_submission == question_model::LateSubmissionPolicy::Reject
        && let Some(due_at) = policy.due_at
    {
        consider(due_at, 0);
    }
    if let Some(closes_at) = policy.closes_at {
        consider(closes_at, 0);
    }
    let auto_submit_at = resolved
        .map(|(deadline, grace_seconds)| {
            add_seconds(deadline, grace_seconds, "attempt auto-submit deadline")
        })
        .transpose()?;
    Ok((
        resolved.map(|(deadline, _)| deadline),
        resolved.map_or(0, |(_, grace_seconds)| grace_seconds),
        auto_submit_at,
    ))
}

pub(super) fn memory_resolved_assignment_policy(
    state: &State,
    tenant: TenantId,
    assignment: AssignmentId,
    enrollment: &AssignmentEnrollment,
    base_override: Option<AssignmentTimingPolicy>,
) -> Result<crate::ResolvedAssignmentTimingPolicy, StoreError> {
    let base = base_override
        .or_else(|| state.assignment_timing.get(&(tenant, assignment)).copied())
        .ok_or_else(|| {
            StoreError::Unavailable(
                "assignment timing policy is missing from memory state".to_string(),
            )
        })?;
    let applicable = state
        .assignment_policy_exceptions
        .iter()
        .filter_map(|((record_tenant, record_assignment, target), exception)| {
            if *record_tenant != tenant || *record_assignment != assignment {
                return None;
            }
            let applies = match target {
                AssignmentPolicyExceptionTarget::Student(student) => *student == enrollment.student,
                AssignmentPolicyExceptionTarget::CourseGroup(group) => state
                    .course_groups
                    .get(&(tenant, *group))
                    .is_some_and(|record| record.members.contains(&enrollment.user)),
            };
            applies.then_some(exception.clone())
        })
        .collect::<Vec<_>>();
    resolve_assignment_policy(base, &applicable)
}

pub(super) fn apply_memory_assignment_timing_update(
    state: &mut State,
    tenant: TenantId,
    assignment: AssignmentId,
    base_override: Option<AssignmentTimingPolicy>,
) -> Result<(), StoreError> {
    #[derive(Debug)]
    enum JobChange {
        Insert(JobId, StoredJob),
        Reschedule(JobId, JobPayload, ActivityTimestamp),
        Complete(JobId),
        None,
    }
    struct Pending {
        attempt: QuestionAttemptId,
        timing: MemoryAttemptTiming,
        resolution: crate::ResolvedAssignmentTimingPolicy,
        current: Option<QuestionAttempt>,
        job_change: JobChange,
    }

    let now = state.authoritative_time;
    let existing = state
        .attempt_timing
        .iter()
        .filter(|((record_tenant, _), timing)| {
            *record_tenant == tenant && timing.assignment == assignment
        })
        .map(|((_, attempt), timing)| (*attempt, *timing))
        .collect::<Vec<_>>();
    let mut reserved_jobs = BTreeSet::new();
    let mut pending = Vec::with_capacity(existing.len());
    for (attempt_id, previous_timing) in existing {
        let base = state
            .attempts
            .get(&(tenant, attempt_id))
            .ok_or(StoreError::NotFound)?;
        if projected_attempt(state, tenant, base).status != AttemptStatus::InProgress {
            continue;
        }
        let run = state
            .runs
            .get(&(tenant, base.run))
            .ok_or(StoreError::NotFound)?;
        let enrollment = state
            .enrollments
            .get(&(tenant, run.enrollment))
            .ok_or(StoreError::NotFound)?;
        let resolution = memory_resolved_assignment_policy(
            state,
            tenant,
            assignment,
            enrollment,
            base_override,
        )?;
        let (effective_deadline, grace_seconds, auto_submit_at) = resolved_memory_attempt_timing(
            resolution.policy,
            run,
            previous_timing.authored_deadline,
            previous_timing.authored_grace_seconds,
        )?;
        let generation = previous_timing
            .generation
            .checked_add(1)
            .ok_or(StoreError::Conflict)?;
        let payload = JobPayload::AutoSubmitAttempt {
            attempt: attempt_id,
            timing_generation: generation,
        };
        let immediate = auto_submit_at.is_some_and(|deadline| deadline <= now);
        let mut current = None;
        let mut job = previous_timing.job;
        let existing_job = job.and_then(|id| state.jobs.get(&id).map(|stored| (id, stored.state)));
        let job_change = if immediate {
            let mut projected = projected_attempt(state, tenant, base);
            projected.status = AttemptStatus::AutoSubmitted;
            projected.timer.deadline = effective_deadline;
            projected.timer.submitted_at = Some(now);
            current = Some(projected);
            let change = match existing_job {
                Some((id, JobState::Ready | JobState::Leased)) => JobChange::Complete(id),
                _ => JobChange::None,
            };
            job = None;
            change
        } else if let Some(available_at) = auto_submit_at {
            match existing_job {
                Some((id, JobState::Ready)) => {
                    JobChange::Reschedule(id, payload.clone(), available_at)
                }
                Some((_id, JobState::Leased)) => JobChange::None,
                Some((_, JobState::Completed | JobState::Dead)) | None => {
                    let id = loop {
                        let candidate = JobId::generate()?;
                        if !state.jobs.contains_key(&candidate) && reserved_jobs.insert(candidate) {
                            break candidate;
                        }
                    };
                    job = Some(id);
                    JobChange::Insert(
                        id,
                        StoredJob {
                            tenant,
                            payload: payload.clone(),
                            state: JobState::Ready,
                            available_at,
                            lease_token: None,
                            lease_expires_at: None,
                            attempt_count: 0,
                            max_attempts: 10,
                            failure: None,
                        },
                    )
                }
            }
        } else {
            let change = match existing_job {
                Some((id, JobState::Ready | JobState::Leased)) => JobChange::Complete(id),
                _ => JobChange::None,
            };
            job = None;
            change
        };
        pending.push(Pending {
            attempt: attempt_id,
            timing: MemoryAttemptTiming {
                assignment,
                authored_deadline: previous_timing.authored_deadline,
                authored_grace_seconds: previous_timing.authored_grace_seconds,
                effective_deadline,
                effective_grace_seconds: grace_seconds,
                auto_submit_at,
                generation,
                job,
            },
            resolution,
            current,
            job_change,
        });
    }
    for update in pending {
        match update.job_change {
            JobChange::Insert(id, job) => {
                state.jobs.insert(id, job);
            }
            JobChange::Reschedule(id, payload, available_at) => {
                let job = state.jobs.get_mut(&id).ok_or(StoreError::NotFound)?;
                job.payload = payload;
                job.available_at = available_at;
                job.failure = None;
            }
            JobChange::Complete(id) => {
                if let Some(job) = state.jobs.get_mut(&id) {
                    job.state = JobState::Completed;
                    job.lease_token = None;
                    job.lease_expires_at = None;
                }
            }
            JobChange::None => {}
        }
        if let Some(current) = update.current {
            state
                .attempt_current
                .insert((tenant, update.attempt), current);
        }
        state
            .attempt_timing
            .insert((tenant, update.attempt), update.timing);
        state
            .attempt_timing_resolution
            .insert((tenant, update.attempt), update.resolution);
    }
    Ok(())
}
