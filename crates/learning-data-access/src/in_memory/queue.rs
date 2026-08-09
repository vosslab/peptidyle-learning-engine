use super::*;

fn mark_export_failed(state: &mut State, job: JobId) {
    for export in state.exports.values_mut() {
        if export.job == job && export.state == StudentExportState::Queued {
            export.state = StudentExportState::Failed;
        }
    }
}

fn mark_assignment_scoring_failed(state: &mut State, job: JobId) {
    let Some(StoredJob {
        tenant,
        payload:
            JobPayload::RecalculateAssignment {
                assignment,
                generation,
            },
        ..
    }) = state.jobs.get(&job)
    else {
        return;
    };
    let key = (*tenant, *assignment);
    if state.assignment_scoring.get(&key) == Some(&(*generation, ScoringStatus::Recalculating)) {
        state
            .assignment_scoring
            .insert(key, (*generation, ScoringStatus::Failed));
    }
}

fn retry_delay_seconds(attempt_count: u16) -> i32 {
    // 1, 2, 4, ... capped at five minutes. This is intentionally deterministic
    // so a later worker cannot turn retry timing into a second policy engine.
    let shift = u32::from(attempt_count.saturating_sub(1)).min(8);
    i32::try_from(1_u32 << shift).expect("bounded retry delay fits i32")
}

fn add_job_seconds(now: ActivityTimestamp, seconds: i32) -> Result<ActivityTimestamp, StoreError> {
    now.as_unix_millis()
        .checked_add(i64::from(seconds) * 1_000)
        .map(ActivityTimestamp::from_unix_millis)
        .ok_or_else(|| StoreError::InvalidRecord("job timestamp overflow".to_string()))
}

#[async_trait]
impl JobStore for MemoryStore {
    async fn enqueue_job(
        &self,
        context: TenantContext,
        job: EnqueueJob,
    ) -> Result<JobId, StoreError> {
        ensure_tenant(context, job.tenant)?;
        job.validate()?;
        let id = JobId::generate()?;
        let mut state = self.write_state()?;
        let available_at = state.authoritative_time;
        state.jobs.insert(
            id,
            StoredJob {
                tenant: job.tenant,
                payload: job.payload,
                state: JobState::Ready,
                available_at,
                lease_token: None,
                lease_expires_at: None,
                attempt_count: 0,
                max_attempts: job.max_attempts,
                failure: None,
            },
        );
        Ok(id)
    }

    async fn claim_next_job(
        &self,
        filter: &crate::JobClaimFilter,
        lease: JobLeaseDuration,
    ) -> Result<Option<ClaimedJob>, StoreError> {
        let token = JobLeaseToken::generate()?;
        let mut state = self.write_state()?;
        let now = state.authoritative_time;

        // A worker killed after its last allowed claim cannot leave a permanent
        // leased row. Mark it dead before selecting eligible work.
        let mut expired_export_jobs = Vec::new();
        for (id, job) in &mut state.jobs {
            if job.state == JobState::Leased
                && filter.contains(job.payload.kind())
                && job.lease_expires_at.is_some_and(|expiry| expiry <= now)
                && job.attempt_count >= job.max_attempts
            {
                job.state = JobState::Dead;
                job.lease_token = None;
                job.lease_expires_at = None;
                job.failure = Some(JobFailureKind::TimedOut);
                expired_export_jobs.push(*id);
            }
        }
        for id in expired_export_jobs {
            mark_export_failed(&mut state, id);
            mark_assignment_scoring_failed(&mut state, id);
        }

        let id = [false, true].into_iter().find_map(|analysis_only| {
            state.jobs.iter().find_map(|(id, job)| {
                let ready = job.state == JobState::Ready && job.available_at <= now;
                let expired = job.state == JobState::Leased
                    && job.lease_expires_at.is_some_and(|expiry| expiry <= now)
                    && job.attempt_count < job.max_attempts;
                let is_analysis = matches!(
                    job.payload,
                    JobPayload::RecalculateCourseItemAnalysis { .. }
                );
                (filter.contains(job.payload.kind())
                    && analysis_only == is_analysis
                    && (ready || expired))
                    .then_some(*id)
            })
        });
        let Some(id) = id else {
            return Ok(None);
        };
        let job = state
            .jobs
            .get_mut(&id)
            .expect("selected job remains present");
        job.state = JobState::Leased;
        job.attempt_count = job
            .attempt_count
            .checked_add(1)
            .ok_or_else(|| StoreError::InvalidRecord("job attempts overflow".to_string()))?;
        job.lease_token = Some(token);
        job.lease_expires_at = Some(add_job_seconds(now, lease.seconds())?);
        job.failure = None;
        Ok(Some(ClaimedJob {
            id,
            tenant: job.tenant,
            payload: job.payload.clone(),
            lease_token: token,
            attempt_count: job.attempt_count,
        }))
    }

    async fn complete_job(&self, id: JobId, token: JobLeaseToken) -> Result<(), StoreError> {
        let mut state = self.write_state()?;
        let now = state.authoritative_time;
        let job = state.jobs.get_mut(&id).ok_or(StoreError::NotFound)?;
        if job.state != JobState::Leased
            || job.lease_token != Some(token)
            || !job.lease_expires_at.is_some_and(|expiry| expiry > now)
        {
            return Err(StoreError::Conflict);
        }
        job.state = JobState::Completed;
        job.lease_token = None;
        job.lease_expires_at = None;
        Ok(())
    }

    async fn fail_job(
        &self,
        id: JobId,
        token: JobLeaseToken,
        failure: JobFailureKind,
    ) -> Result<JobFailureDisposition, StoreError> {
        let mut state = self.write_state()?;
        let now = state.authoritative_time;
        let job = state.jobs.get_mut(&id).ok_or(StoreError::NotFound)?;
        if job.state != JobState::Leased
            || job.lease_token != Some(token)
            || !job.lease_expires_at.is_some_and(|expiry| expiry > now)
        {
            return Err(StoreError::Conflict);
        }
        job.lease_token = None;
        job.lease_expires_at = None;
        job.failure = Some(failure);
        if failure == JobFailureKind::Permanent || job.attempt_count >= job.max_attempts {
            job.state = JobState::Dead;
            mark_export_failed(&mut state, id);
            mark_assignment_scoring_failed(&mut state, id);
            return Ok(JobFailureDisposition::Dead);
        }
        let delay_seconds = retry_delay_seconds(job.attempt_count);
        job.state = JobState::Ready;
        job.available_at = add_job_seconds(now, delay_seconds)?;
        Ok(JobFailureDisposition::Retrying)
    }

    async fn get_job(
        &self,
        context: TenantContext,
        id: JobId,
    ) -> Result<Option<TenantJobView>, StoreError> {
        let state = self.read_state()?;
        Ok(state
            .jobs
            .get(&id)
            .filter(|job| job.tenant == context.tenant_id())
            .map(|job| TenantJobView {
                id,
                payload: job.payload.clone(),
                state: job.state,
                attempt_count: job.attempt_count,
            }))
    }

    async fn ready_queue_depth(
        &self,
        filter: &crate::JobClaimFilter,
    ) -> Result<QueueDepth, StoreError> {
        let state = self.read_state()?;
        let ready = state
            .jobs
            .values()
            .filter(|job| {
                filter.contains(job.payload.kind())
                    && job.state == JobState::Ready
                    && job.available_at <= state.authoritative_time
            })
            .count();
        Ok(QueueDepth {
            ready: u64::try_from(ready).expect("queue length fits u64"),
        })
    }
}

#[async_trait]
impl crate::AttemptAutoSubmitWorkerStore for MemoryStore {
    async fn commit_attempt_auto_submit(
        &self,
        context: TenantContext,
        command: crate::AttemptAutoSubmitWorkerCommand,
    ) -> Result<crate::AttemptAutoSubmitCommitOutcome, StoreError> {
        let mut state = self.write_state()?;
        let tenant = context.tenant_id();
        let now = state.authoritative_time;
        let expected_payload = JobPayload::AutoSubmitAttempt {
            attempt: command.attempt,
            timing_generation: command.timing_generation,
        };
        let claim_active = state.jobs.get(&command.job).is_some_and(|job| {
            job.tenant == tenant
                && job.state == JobState::Leased
                && job.lease_token == Some(command.lease)
                && job.lease_expires_at.is_some_and(|expiry| expiry > now)
                && job.payload == expected_payload
        });
        if !claim_active {
            return Ok(crate::AttemptAutoSubmitCommitOutcome::ClaimNoLongerActive);
        }

        let timing = state
            .attempt_timing
            .get(&(tenant, command.attempt))
            .copied();
        let base = state.attempts.get(&(tenant, command.attempt)).cloned();
        let active = base.as_ref().is_some_and(|attempt| {
            projected_attempt(&state, tenant, attempt).status == AttemptStatus::InProgress
        });
        let current_job = timing.and_then(|value| value.job);
        if !active || current_job != Some(command.job) {
            complete_memory_job(&mut state, command.job)?;
            return Ok(crate::AttemptAutoSubmitCommitOutcome::Superseded);
        }
        let timing = timing.expect("active timing job has a timing row");
        let Some(auto_submit_at) = timing.auto_submit_at else {
            complete_memory_job(&mut state, command.job)?;
            if let Some(current) = state.attempt_timing.get_mut(&(tenant, command.attempt)) {
                current.job = None;
            }
            return Ok(crate::AttemptAutoSubmitCommitOutcome::Superseded);
        };
        if now < auto_submit_at {
            let job = state
                .jobs
                .get_mut(&command.job)
                .ok_or(StoreError::NotFound)?;
            job.payload = JobPayload::AutoSubmitAttempt {
                attempt: command.attempt,
                timing_generation: timing.generation,
            };
            job.state = JobState::Ready;
            job.available_at = auto_submit_at;
            job.lease_token = None;
            job.lease_expires_at = None;
            job.failure = None;
            return Ok(crate::AttemptAutoSubmitCommitOutcome::Rescheduled);
        }

        let mut current = projected_attempt(
            &state,
            tenant,
            base.as_ref().expect("active attempt remains present"),
        );
        current.status = AttemptStatus::AutoSubmitted;
        current.timer.deadline = timing.effective_deadline;
        current.timer.submitted_at = Some(now);
        state
            .attempt_current
            .insert((tenant, command.attempt), current);
        if let Some(current_timing) = state.attempt_timing.get_mut(&(tenant, command.attempt)) {
            current_timing.job = None;
        }
        complete_memory_job(&mut state, command.job)?;
        Ok(crate::AttemptAutoSubmitCommitOutcome::AutoSubmitted)
    }
}

pub(super) fn complete_memory_job(state: &mut State, job: JobId) -> Result<(), StoreError> {
    let job = state.jobs.get_mut(&job).ok_or(StoreError::NotFound)?;
    job.state = JobState::Completed;
    job.lease_token = None;
    job.lease_expires_at = None;
    Ok(())
}

#[async_trait]
impl crate::AssignmentScoringWorkerStore for MemoryStore {
    async fn prepare_assignment_scoring(
        &self,
        context: TenantContext,
        command: crate::AssignmentScoringWorkerCommand,
    ) -> Result<(), StoreError> {
        let mut state = self.write_state()?;
        let job = state.jobs.get(&command.job).ok_or(StoreError::NotFound)?;
        if job.tenant != context.tenant_id()
            || job.state != JobState::Leased
            || job.lease_token != Some(command.lease)
            || !job
                .lease_expires_at
                .is_some_and(|expiry| expiry > state.authoritative_time)
            || job.payload
                != (JobPayload::RecalculateAssignment {
                    assignment: command.assignment,
                    generation: command.generation,
                })
        {
            return Err(StoreError::Conflict);
        }
        let prepared = build_memory_assignment_scoring(
            &state,
            context.tenant_id(),
            command.assignment,
            command.generation,
        )?;
        state.assignment_score_staging.insert(command.job, prepared);
        Ok(())
    }

    async fn commit_assignment_scoring(
        &self,
        context: TenantContext,
        command: crate::AssignmentScoringWorkerCommand,
    ) -> Result<crate::AssignmentScoringCommitOutcome, StoreError> {
        let mut state = self.write_state()?;
        let claim_active = state.jobs.get(&command.job).is_some_and(|job| {
            job.tenant == context.tenant_id()
                && job.state == JobState::Leased
                && job.lease_token == Some(command.lease)
                && job
                    .lease_expires_at
                    .is_some_and(|expiry| expiry > state.authoritative_time)
                && job.payload
                    == (JobPayload::RecalculateAssignment {
                        assignment: command.assignment,
                        generation: command.generation,
                    })
        });
        if !claim_active {
            return Ok(crate::AssignmentScoringCommitOutcome::ClaimNoLongerActive);
        }
        let prepared = state
            .assignment_score_staging
            .remove(&command.job)
            .ok_or(StoreError::Conflict)?;
        if prepared.tenant != context.tenant_id()
            || prepared.assignment != command.assignment
            || prepared.generation != command.generation
        {
            return Err(StoreError::Conflict);
        }
        let key = (context.tenant_id(), command.assignment);
        let current = state
            .assignment_scoring
            .get(&key)
            .copied()
            .ok_or(StoreError::NotFound)?;
        let superseded = current.0 != command.generation;
        let analysis_job = (!superseded)
            .then(|| super::item_analysis::allocate_course_item_analysis_job(&state))
            .transpose()?;
        if !superseded {
            let current_attempt_count = state
                .submissions
                .values()
                .filter(|submission| {
                    let attempt =
                        projected_attempt(&state, context.tenant_id(), &submission.record.attempt);
                    attempt.tenant == context.tenant_id()
                        && attempt.result.is_some()
                        && !matches!(
                            attempt.status,
                            AttemptStatus::Cleared | AttemptStatus::Exempt
                        )
                        && state
                            .runs
                            .get(&(context.tenant_id(), attempt.run))
                            .and_then(|run| {
                                state
                                    .enrollments
                                    .get(&(context.tenant_id(), run.enrollment))
                            })
                            .is_some_and(|enrollment| enrollment.assignment == command.assignment)
                })
                .count();
            if prepared.attempts.len() != current_attempt_count {
                return Err(StoreError::Conflict);
            }
            let assignment = state.assignments.get(&key).ok_or(StoreError::NotFound)?;
            if prepared.attempts.values().any(|score| {
                score.assignment != command.assignment
                    || score.generation != command.generation
                    || !score.earned_points.is_finite()
                    || !score.possible_points.is_finite()
                    || score.possible_points < 0.0
                    || (!assignment
                        .items
                        .iter()
                        .any(|item| item.id == score.assignment_item)
                        && !assignment.selection_groups.iter().any(|group| {
                            group
                                .candidates
                                .iter()
                                .any(|candidate| candidate.id == score.assignment_item)
                        }))
            }) {
                return Err(StoreError::Conflict);
            }
            state.attempt_scores.retain(|(tenant, _), score| {
                *tenant != context.tenant_id() || score.assignment != command.assignment
            });
            state.attempt_scores.extend(
                prepared
                    .attempts
                    .into_iter()
                    .map(|(attempt, score)| ((context.tenant_id(), attempt), score)),
            );
            for (enrollment, record) in prepared.enrollments {
                state
                    .enrollments
                    .insert((context.tenant_id(), enrollment), record);
            }
            for (enrollment, summary) in prepared.summaries {
                state
                    .summaries
                    .insert((context.tenant_id(), enrollment), summary);
            }
            state
                .assignment_scoring
                .insert(key, (command.generation, ScoringStatus::Current));
            super::item_analysis::enqueue_course_item_analysis_after_scoring(
                &mut state,
                analysis_job.expect("current scoring generation allocates item-analysis work"),
                context.tenant_id(),
                command.assignment,
                command.generation,
            )?;
        }
        let job = state
            .jobs
            .get_mut(&command.job)
            .ok_or(StoreError::NotFound)?;
        job.state = JobState::Completed;
        job.lease_token = None;
        job.lease_expires_at = None;
        Ok(if superseded {
            crate::AssignmentScoringCommitOutcome::Superseded
        } else {
            crate::AssignmentScoringCommitOutcome::Committed
        })
    }
}

fn build_memory_assignment_scoring(
    state: &State,
    tenant: TenantId,
    assignment_id: AssignmentId,
    generation: ScoringGeneration,
) -> Result<PreparedAssignmentScoring, StoreError> {
    let assignment = state
        .assignments
        .get(&(tenant, assignment_id))
        .ok_or(StoreError::NotFound)?;
    if state.assignment_scoring.get(&(tenant, assignment_id))
        != Some(&(generation, ScoringStatus::Recalculating))
    {
        return Err(StoreError::Conflict);
    }
    let mut attempts = BTreeMap::new();
    let mut latest_by_position = BTreeMap::new();
    for (key, submission) in &state.submissions {
        if key.0 != tenant {
            continue;
        }
        let attempted = projected_attempt(state, tenant, &submission.record.attempt);
        if matches!(
            attempted.status,
            AttemptStatus::Cleared | AttemptStatus::Exempt
        ) {
            continue;
        }
        let Some(result) = attempted.result else {
            continue;
        };
        let Some(run) = state.runs.get(&(tenant, attempted.run)) else {
            continue;
        };
        let Some(enrollment) = state.enrollments.get(&(tenant, run.enrollment)) else {
            continue;
        };
        if enrollment.assignment != assignment_id {
            continue;
        }
        let run_item = state
            .run_items
            .get(&(tenant, run.id))
            .and_then(|items| {
                items
                    .iter()
                    .find(|item| item.issued_position == attempted.assignment_position)
            })
            .ok_or_else(|| {
                StoreError::Unavailable("submitted attempt has no immutable run item".to_string())
            })?;
        let (earned_points, possible_points) = crate::current_attempt_points(
            assignment,
            run_item.assignment_item,
            attempted.status,
            result,
        )?;
        attempts.insert(
            attempted.id,
            MemoryAttemptScore {
                assignment: assignment_id,
                assignment_item: run_item.assignment_item,
                generation,
                earned_points,
                possible_points,
            },
        );
        let submitted_at = attempted.timer.submitted_at.ok_or_else(|| {
            StoreError::Unavailable("stored submission has no submission time".to_string())
        })?;
        let position_key = (run.id, attempted.assignment_position);
        let candidate = (submitted_at, attempted.id, earned_points, possible_points);
        if latest_by_position
            .get(&position_key)
            .is_none_or(|existing| candidate > *existing)
        {
            latest_by_position.insert(position_key, candidate);
        }
    }
    let mut enrollments = BTreeMap::new();
    let mut summaries = BTreeMap::new();
    for enrollment in state
        .enrollments
        .values()
        .filter(|record| record.tenant == tenant && record.assignment == assignment_id)
    {
        let mut completed = Vec::new();
        let mut first_completed_at: Option<ActivityTimestamp> = None;
        for run in state.runs.values().filter(|run| {
            run.tenant == tenant && run.enrollment == enrollment.id && run.completed_at.is_some()
        }) {
            let (earned, possible) = latest_by_position
                .iter()
                .filter(|((candidate_run, _), _)| *candidate_run == run.id)
                .fold((0.0, 0.0), |(earned, possible), (_, score)| {
                    (earned + score.2, possible + score.3)
                });
            let score = if possible > 0.0 {
                earned / possible
            } else {
                earned
            };
            if !score.is_finite() {
                return Err(StoreError::InvalidRecord(
                    "recalculated run score is not finite".to_string(),
                ));
            }
            if latest_by_position
                .keys()
                .any(|(candidate_run, _)| *candidate_run == run.id)
            {
                first_completed_at = match (first_completed_at, run.completed_at) {
                    (Some(current), Some(candidate)) => Some(current.min(candidate)),
                    (None, candidate) => candidate,
                    (current, None) => current,
                };
                completed.push(domain::scoring::CompletedRunScore {
                    run: run.id,
                    run_number: run.run_number,
                    score: crate::score_precision::round_for_persistence(score),
                });
            }
        }
        let summary = state
            .summaries
            .get(&(tenant, enrollment.id))
            .cloned()
            .ok_or(StoreError::NotFound)?;
        let (enrollment, summary) = crate::recalculated_enrollment_projection(
            enrollment.clone(),
            summary,
            assignment.policies.grade,
            completed,
            first_completed_at,
        )?;
        enrollments.insert(enrollment.id, enrollment);
        summaries.insert(summary.enrollment, summary);
    }
    Ok(PreparedAssignmentScoring {
        tenant,
        assignment: assignment_id,
        generation,
        attempts,
        enrollments,
        summaries,
    })
}
