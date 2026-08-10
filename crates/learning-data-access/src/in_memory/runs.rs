use async_trait::async_trait;

use super::*;

#[async_trait]
impl crate::RunStore for MemoryStore {
    async fn start_or_resume_run_impl(
        &self,
        context: TenantContext,
        actor: UserId,
        assignment_id: AssignmentId,
        proposed_run: RunId,
    ) -> Result<AssignmentRun, StoreError> {
        let mut state = self.write_state()?;
        let tenant = context.tenant_id();
        let enrollment = state
            .enrollments
            .values()
            .find(|enrollment| {
                enrollment.tenant == tenant
                    && enrollment.assignment == assignment_id
                    && enrollment.user == actor
            })
            .cloned()
            .ok_or(StoreError::NotFound)?;
        if let Some(active) = state.runs.values().find(|run| {
            run.tenant == tenant && run.enrollment == enrollment.id && run.completed_at.is_none()
        }) {
            return Ok(active.clone());
        }
        if state.runs.contains_key(&(tenant, proposed_run)) {
            return Err(StoreError::AlreadyExists);
        }
        let assignment = assignment_record(&state, tenant, assignment_id)?;
        require_course_records_accessible(&state, tenant, assignment.course_id)?;
        let timing =
            memory_resolved_assignment_policy(&state, tenant, assignment_id, &enrollment, None)?
                .policy;
        let now = state.authoritative_time;
        if !timing.visible {
            return Err(StoreError::NotFound);
        }
        if timing
            .available_at
            .is_some_and(|available_at| now < available_at)
        {
            return Err(StoreError::InvalidRecord(
                "assignment is not yet available".to_string(),
            ));
        }
        if timing.closes_at.is_some_and(|closes_at| now >= closes_at) {
            return Err(StoreError::InvalidRecord(
                "assignment is closed".to_string(),
            ));
        }
        if timing.late_submission == question_model::LateSubmissionPolicy::Reject
            && timing.due_at.is_some_and(|due_at| now > due_at)
        {
            return Err(StoreError::InvalidRecord(
                "assignment due date has passed".to_string(),
            ));
        }
        let existing_run_count = state
            .runs
            .values()
            .filter(|run| run.tenant == tenant && run.enrollment == enrollment.id)
            .count();
        if timing
            .attempt_limit
            .is_some_and(|limit| existing_run_count >= limit as usize)
        {
            return Err(StoreError::InvalidRecord(
                "assignment attempt limit has been reached".to_string(),
            ));
        }
        let previous = state
            .summaries
            .get(&(tenant, enrollment.id))
            .cloned()
            .ok_or(StoreError::NotFound)?;
        if !continued_practice_allows_run(&previous, assignment.policies.continued_practice) {
            return Err(StoreError::InvalidRecord(
                "continued-practice policy does not permit another run".to_string(),
            ));
        }
        let run_number = state
            .runs
            .values()
            .filter(|run| run.tenant == tenant && run.enrollment == enrollment.id)
            .map(|run| run.run_number)
            .max()
            .unwrap_or(0)
            .checked_add(1)
            .ok_or_else(|| StoreError::InvalidRecord("run number overflow".to_string()))?;
        let run = AssignmentRun {
            id: proposed_run,
            tenant,
            enrollment: enrollment.id,
            run_number,
            started_at: state.authoritative_time,
            completed_at: None,
            score: None,
            mode: match enrollment.status() {
                EnrollmentStatus::InProgress => RunMode::Assigned,
                EnrollmentStatus::Completed => RunMode::Practice,
            },
            variation: assignment.policies.variation,
        };
        let next = project_summary(
            &previous,
            summary_transition(&ActivityTransition::StartRun { run: run.clone() }),
            grade_policy(&assignment),
        )?;
        let run_items = select_assignment_run_items(&assignment, run.id)?;
        state.runs.insert((tenant, run.id), run.clone());
        state.run_items.insert((tenant, run.id), run_items);
        state.summaries.insert((tenant, enrollment.id), next);
        Ok(run)
    }
    async fn assignment_run_items_impl(
        &self,
        context: TenantContext,
        run: RunId,
    ) -> Result<Vec<AssignmentRunItem>, StoreError> {
        let state = self.read_state()?;
        if !state.runs.contains_key(&(context.tenant_id(), run)) {
            return Err(StoreError::NotFound);
        }
        Ok(state
            .run_items
            .get(&(context.tenant_id(), run))
            .cloned()
            .unwrap_or_default())
    }
    async fn issue_or_resume_question_attempt_impl(
        &self,
        context: TenantContext,
        command: IssueQuestionAttemptCommand,
    ) -> Result<QuestionAttempt, StoreError> {
        let mut state = self.write_state()?;
        let tenant = context.tenant_id();
        let run = state
            .runs
            .get(&(tenant, command.run))
            .cloned()
            .ok_or(StoreError::NotFound)?;
        if run.completed_at.is_some() || run.score.is_some() {
            return Err(StoreError::InvalidRecord(
                "a completed run cannot issue another question".to_string(),
            ));
        }
        let enrollment = enrollment_record(&state, tenant, run.enrollment)?;
        if enrollment.user != command.actor {
            return Err(StoreError::Forbidden);
        }
        let assignment = assignment_record(&state, tenant, enrollment.assignment)?;
        require_course_records_accessible(&state, tenant, assignment.course_id)?;
        let run_items = state
            .run_items
            .get(&(tenant, command.run))
            .ok_or(StoreError::NotFound)?;
        validate_assignment_position(run_items, &command)?;

        let prefetched = command.prefetched.as_ref();
        if let Some(prefetched) = prefetched {
            let key = (
                tenant,
                command.run,
                prefetched.predecessor,
                command.assignment_position,
            );
            if prefetched.tenant != tenant
                || prefetched.run != command.run
                || command.predecessor_submission != Some(prefetched.predecessor)
                || prefetched.assignment_position != command.assignment_position
                || prefetched.problem != command.problem
                || prefetched.question_version != command.question_version
                || prefetched.presentation != command.presentation
                || prefetched.webwork_replay != command.webwork_replay
                || state.prefetched_questions.get(&key) != Some(prefetched)
                || !state
                    .submissions
                    .contains_key(&(tenant, prefetched.predecessor))
            {
                return Err(StoreError::Conflict);
            }
        }

        let unresolved = state
            .attempts
            .values()
            .filter(|attempt| {
                attempt.tenant == tenant
                    && attempt.run == run.id
                    && projected_attempt(&state, tenant, attempt).status
                        == AttemptStatus::InProgress
            })
            .max_by_key(|attempt| (attempt.timer.issued_at, attempt.id));
        if let Some(active) = unresolved.cloned() {
            if active.assignment_position == command.assignment_position {
                if state.attempt_presentations.get(&(tenant, active.id))
                    != Some(&command.presentation)
                {
                    return Err(StoreError::Conflict);
                }
                if let Some(mapping) = command.webwork_replay.as_ref()
                    && state
                        .webwork_grade_replay
                        .get(&(tenant, active.id))
                        .map(|value| &value.mapping)
                        != Some(mapping)
                {
                    return Err(StoreError::Conflict);
                }
                if let Some(predecessor) = command.predecessor_submission {
                    if state
                        .attempts
                        .get(&(tenant, predecessor))
                        .is_none_or(|value| value.run != command.run)
                    {
                        return Err(StoreError::Conflict);
                    }
                    if !state.submissions.contains_key(&(tenant, predecessor)) {
                        return Err(StoreError::Conflict);
                    }
                    match state.submission_next_attempts.get(&(tenant, predecessor)) {
                        Some(Some(existing)) if *existing != active.id => {
                            return Err(StoreError::Conflict);
                        }
                        Some(None) => return Err(StoreError::Conflict),
                        _ => {
                            state
                                .submission_next_attempts
                                .insert((tenant, predecessor), Some(active.id));
                        }
                    }
                }
                return Ok(projected_attempt(&state, tenant, &active));
            }
            return Err(StoreError::InvalidRecord(
                "another question attempt is already active in this run".to_string(),
            ));
        }
        let latest_for_position = state
            .attempts
            .values()
            .filter(|attempt| {
                attempt.tenant == tenant
                    && attempt.run == run.id
                    && attempt.assignment_position == command.assignment_position
                    && !matches!(
                        projected_attempt(&state, tenant, attempt).status,
                        AttemptStatus::Cleared | AttemptStatus::Exempt
                    )
            })
            .max_by_key(|attempt| (attempt.timer.issued_at, attempt.id));
        if latest_for_position.is_some_and(|latest| {
            projected_attempt(&state, tenant, latest)
                .result
                .is_some_and(|result| result.correct)
        }) {
            return Err(StoreError::InvalidRecord(
                "a correct question position cannot be retried".to_string(),
            ));
        }
        if state.attempts.contains_key(&(tenant, command.attempt)) {
            return Err(StoreError::AlreadyExists);
        }
        let (seed, parameter_hash, provenance) = match prefetched {
            Some(value) => (
                value.seed,
                value.parameter_hash.clone(),
                value.provenance.clone(),
            ),
            None => (
                command.seed,
                command.parameter_hash.clone(),
                command.provenance.clone(),
            ),
        };
        let presentation = prefetched
            .map(|value| value.presentation)
            .unwrap_or(command.presentation);
        let webwork_replay = prefetched
            .and_then(|value| value.webwork_replay.clone())
            .or(command.webwork_replay.clone());
        if parameter_hash.trim().is_empty() || provenance.rendered_question_sha256.trim().is_empty()
        {
            return Err(StoreError::InvalidRecord(
                "issued attempt hashes must not be empty".to_string(),
            ));
        }
        let question = state
            .published
            .get(&(command.problem, command.question_version))
            .ok_or(StoreError::NotFound)?;
        let authored_timer = issued_timer(
            state.authoritative_time,
            &run,
            question.question.timing_policy,
        )?;
        let authored_grace_seconds = timing_policy_grace_seconds(question.question.timing_policy);
        let resolved_assignment_timing =
            memory_resolved_assignment_policy(&state, tenant, assignment.id, &enrollment, None)?;
        let (effective_deadline, effective_grace_seconds, auto_submit_at) =
            resolved_memory_attempt_timing(
                resolved_assignment_timing.policy,
                &run,
                authored_timer.deadline,
                authored_grace_seconds,
            )?;
        if effective_deadline.is_some_and(|deadline| deadline < state.authoritative_time)
            || auto_submit_at.is_some_and(|deadline| deadline <= state.authoritative_time)
        {
            return Err(StoreError::TimedOut);
        }
        let timer = AttemptTimerRecord {
            deadline: effective_deadline,
            ..authored_timer
        };
        let timing_generation = 1;
        let timing_job = if let Some(available_at) = auto_submit_at {
            let job = loop {
                let candidate = JobId::generate()?;
                if !state.jobs.contains_key(&candidate) {
                    break candidate;
                }
            };
            Some((
                job,
                StoredJob {
                    tenant,
                    payload: JobPayload::AutoSubmitAttempt {
                        attempt: command.attempt,
                        timing_generation,
                    },
                    state: JobState::Ready,
                    available_at,
                    lease_token: None,
                    lease_expires_at: None,
                    attempt_count: 0,
                    max_attempts: 10,
                    failure: None,
                },
            ))
        } else {
            None
        };
        let attempt = QuestionAttempt {
            id: command.attempt,
            tenant,
            run: run.id,
            problem: command.problem,
            question_version: command.question_version,
            assignment_position: command.assignment_position,
            seed,
            parameter_hash,
            response: None,
            status: AttemptStatus::InProgress,
            result: None,
            timer,
            provenance,
        };
        let webwork_replay = webwork_replay
            .map(|mapping| {
                webwork_replay_state_from_issue(
                    attempt.problem,
                    attempt.question_version,
                    attempt.seed,
                    &attempt.provenance,
                    presentation,
                    mapping,
                )
            })
            .transpose()?;
        if let Some(prefetched) = prefetched {
            state.prefetched_questions.remove(&(
                tenant,
                command.run,
                prefetched.predecessor,
                command.assignment_position,
            ));
        }
        if let Some(predecessor) = command.predecessor_submission {
            if state
                .attempts
                .get(&(tenant, predecessor))
                .is_none_or(|value| value.run != command.run)
            {
                return Err(StoreError::Conflict);
            }
            if !state.submissions.contains_key(&(tenant, predecessor)) {
                return Err(StoreError::Conflict);
            }
            match state.submission_next_attempts.get(&(tenant, predecessor)) {
                Some(Some(existing)) if *existing != attempt.id => {
                    return Err(StoreError::Conflict);
                }
                Some(None) => return Err(StoreError::Conflict),
                _ => {
                    state
                        .submission_next_attempts
                        .insert((tenant, predecessor), Some(attempt.id));
                }
            }
        }
        let timing_job_id = timing_job.as_ref().map(|(job, _)| *job);
        if let Some((job, queued)) = timing_job {
            state.jobs.insert(job, queued);
        }
        state.attempt_timing.insert(
            (tenant, attempt.id),
            MemoryAttemptTiming {
                assignment: assignment.id,
                authored_deadline: authored_timer.deadline,
                authored_grace_seconds,
                effective_deadline,
                effective_grace_seconds,
                auto_submit_at,
                generation: timing_generation,
                job: timing_job_id,
            },
        );
        state
            .attempt_timing_resolution
            .insert((tenant, attempt.id), resolved_assignment_timing);
        state.attempts.insert((tenant, attempt.id), attempt.clone());
        state
            .attempt_presentations
            .insert((tenant, attempt.id), presentation);
        if let Some(replay) = webwork_replay {
            state
                .webwork_grade_replay
                .insert((tenant, attempt.id), replay);
        }
        Ok(attempt)
    }

    async fn get_attempt_presentation_binding_impl(
        &self,
        context: TenantContext,
        actor: UserId,
        attempt: QuestionAttemptId,
    ) -> Result<Option<PresentationBindingV1>, StoreError> {
        let state = self.read_state()?;
        let tenant = context.tenant_id();
        let record = state
            .attempts
            .get(&(tenant, attempt))
            .ok_or(StoreError::NotFound)?;
        let run = state
            .runs
            .get(&(tenant, record.run))
            .ok_or(StoreError::NotFound)?;
        let enrollment = enrollment_record(&state, tenant, run.enrollment)?;
        if enrollment.user != actor {
            return Err(StoreError::Forbidden);
        }
        Ok(state.attempt_presentations.get(&(tenant, attempt)).copied())
    }

    async fn get_webwork_grade_replay_state_impl(
        &self,
        context: TenantContext,
        actor: UserId,
        attempt: QuestionAttemptId,
    ) -> Result<Option<WebworkGradeReplayStateV1>, StoreError> {
        let state = self.read_state()?;
        let tenant = context.tenant_id();
        let record = state
            .attempts
            .get(&(tenant, attempt))
            .ok_or(StoreError::NotFound)?;
        let run = state
            .runs
            .get(&(tenant, record.run))
            .ok_or(StoreError::NotFound)?;
        let enrollment = enrollment_record(&state, tenant, run.enrollment)?;
        if enrollment.user != actor {
            return Err(StoreError::Forbidden);
        }
        Ok(state.webwork_grade_replay.get(&(tenant, attempt)).cloned())
    }
    async fn reserve_or_resume_prefetched_question_impl(
        &self,
        context: TenantContext,
        command: ReservePrefetchedQuestionCommand,
    ) -> Result<PrefetchedQuestion, StoreError> {
        let mut state = self.write_state()?;
        let tenant = context.tenant_id();
        let reservation = command.reservation;
        if reservation.tenant != tenant
            || reservation.parameter_hash.trim().is_empty()
            || reservation
                .provenance
                .rendered_question_sha256
                .trim()
                .is_empty()
        {
            return Err(StoreError::InvalidRecord(
                "invalid prefetch reservation".to_string(),
            ));
        }
        let run = state
            .runs
            .get(&(tenant, reservation.run))
            .ok_or(StoreError::NotFound)?;
        if run.completed_at.is_some() || run.score.is_some() {
            return Err(StoreError::Conflict);
        }
        let enrollment = enrollment_record(&state, tenant, run.enrollment)?;
        if enrollment.user != command.actor {
            return Err(StoreError::Forbidden);
        }
        let predecessor = state
            .attempts
            .get(&(tenant, reservation.predecessor))
            .ok_or(StoreError::NotFound)?;
        if predecessor.run != reservation.run
            || state.submissions.contains_key(&(tenant, predecessor.id))
        {
            return Err(StoreError::Conflict);
        }
        let assignment = assignment_record(&state, tenant, enrollment.assignment)?;
        require_course_records_accessible(&state, tenant, assignment.course_id)?;
        let expected = assignment
            .active_item_at(reservation.assignment_position)
            .ok_or_else(|| {
                StoreError::InvalidRecord("prefetch position is outside the assignment".to_string())
            })?;
        if expected.reference.problem != reservation.problem
            || expected.reference.version != reservation.question_version
        {
            return Err(StoreError::InvalidRecord(
                "prefetch identity does not match assignment position".to_string(),
            ));
        }
        if state.attempts.values().any(|attempt| {
            attempt.tenant == tenant
                && attempt.run == reservation.run
                && attempt.assignment_position == reservation.assignment_position
        }) {
            return Err(StoreError::Conflict);
        }
        let key = (
            tenant,
            reservation.run,
            reservation.predecessor,
            reservation.assignment_position,
        );
        if let Some(existing) = state.prefetched_questions.get(&key) {
            return if existing == &reservation {
                Ok(existing.clone())
            } else {
                Err(StoreError::Conflict)
            };
        }
        state.prefetched_questions.insert(key, reservation.clone());
        Ok(reservation)
    }
    async fn get_prefetched_question_impl(
        &self,
        context: TenantContext,
        actor: UserId,
        run: RunId,
        predecessor: QuestionAttemptId,
        assignment_position: u32,
    ) -> Result<Option<PrefetchedQuestion>, StoreError> {
        let state = self.read_state()?;
        let run_record = state
            .runs
            .get(&(context.tenant_id(), run))
            .ok_or(StoreError::NotFound)?;
        let enrollment = enrollment_record(&state, context.tenant_id(), run_record.enrollment)?;
        if enrollment.user != actor {
            return Err(StoreError::Forbidden);
        }
        let assignment = assignment_record(&state, context.tenant_id(), enrollment.assignment)?;
        require_course_records_accessible(&state, context.tenant_id(), assignment.course_id)?;
        Ok(state
            .prefetched_questions
            .get(&(context.tenant_id(), run, predecessor, assignment_position))
            .cloned())
    }
    async fn submission_next_attempt_impl(
        &self,
        context: TenantContext,
        actor: UserId,
        predecessor: QuestionAttemptId,
    ) -> Result<SubmissionNextAttempt, StoreError> {
        let state = self.read_state()?;
        let attempt = state
            .attempts
            .get(&(context.tenant_id(), predecessor))
            .ok_or(StoreError::NotFound)?;
        let run = state
            .runs
            .get(&(context.tenant_id(), attempt.run))
            .ok_or(StoreError::NotFound)?;
        let enrollment = enrollment_record(&state, context.tenant_id(), run.enrollment)?;
        let assignment = assignment_record(&state, context.tenant_id(), enrollment.assignment)?;
        require_course_records_accessible(&state, context.tenant_id(), assignment.course_id)?;
        require_attempt_owner(&state, context.tenant_id(), attempt, actor)?;
        if !state
            .submissions
            .contains_key(&(context.tenant_id(), predecessor))
        {
            return Err(StoreError::Conflict);
        }
        Ok(
            match state
                .submission_next_attempts
                .get(&(context.tenant_id(), predecessor))
            {
                None => SubmissionNextAttempt::Pending,
                Some(None) => SubmissionNextAttempt::None,
                Some(Some(next)) => SubmissionNextAttempt::Issued(*next),
            },
        )
    }
    async fn pending_submission_for_run_impl(
        &self,
        context: TenantContext,
        actor: UserId,
        run: RunId,
    ) -> Result<Option<QuestionAttemptId>, StoreError> {
        let state = self.read_state()?;
        let run_record = state
            .runs
            .get(&(context.tenant_id(), run))
            .ok_or(StoreError::NotFound)?;
        let enrollment = enrollment_record(&state, context.tenant_id(), run_record.enrollment)?;
        let assignment = assignment_record(&state, context.tenant_id(), enrollment.assignment)?;
        require_course_records_accessible(&state, context.tenant_id(), assignment.course_id)?;
        if enrollment.user != actor {
            return Err(StoreError::Forbidden);
        }
        let pending: Vec<_> = state
            .attempts
            .values()
            .filter(|attempt| {
                attempt.tenant == context.tenant_id()
                    && attempt.run == run
                    && state
                        .submissions
                        .contains_key(&(context.tenant_id(), attempt.id))
                    && !state
                        .submission_next_attempts
                        .contains_key(&(context.tenant_id(), attempt.id))
            })
            .map(|attempt| attempt.id)
            .take(2)
            .collect();
        match pending.as_slice() {
            [] => Ok(None),
            [id] => Ok(Some(*id)),
            _ => Err(StoreError::Conflict),
        }
    }
    async fn finalize_submission_next_attempt_impl(
        &self,
        context: TenantContext,
        actor: UserId,
        predecessor: QuestionAttemptId,
        next: Option<QuestionAttemptId>,
    ) -> Result<(), StoreError> {
        let mut state = self.write_state()?;
        let tenant = context.tenant_id();
        let attempt = state
            .attempts
            .get(&(tenant, predecessor))
            .ok_or(StoreError::NotFound)?
            .clone();
        let run = state
            .runs
            .get(&(tenant, attempt.run))
            .ok_or(StoreError::NotFound)?;
        let enrollment = enrollment_record(&state, tenant, run.enrollment)?;
        let assignment = assignment_record(&state, tenant, enrollment.assignment)?;
        require_course_records_accessible(&state, tenant, assignment.course_id)?;
        require_attempt_owner(&state, tenant, &attempt, actor)?;
        if !state.submissions.contains_key(&(tenant, predecessor)) {
            return Err(StoreError::Conflict);
        }
        if let Some(next) = next {
            let next_attempt = state
                .attempts
                .get(&(tenant, next))
                .ok_or(StoreError::NotFound)?;
            if next_attempt.run != attempt.run {
                return Err(StoreError::Conflict);
            }
        }
        match state.submission_next_attempts.get(&(tenant, predecessor)) {
            Some(existing) if *existing != next => Err(StoreError::Conflict),
            _ => {
                state
                    .submission_next_attempts
                    .insert((tenant, predecessor), next);
                Ok(())
            }
        }
    }
    async fn list_question_attempts_impl(
        &self,
        context: TenantContext,
        run: RunId,
        page: PageRequest,
    ) -> Result<Page<QuestionAttempt>, StoreError> {
        let state = self.read_state()?;
        let run_record = state
            .runs
            .get(&(context.tenant_id(), run))
            .ok_or(StoreError::NotFound)?;
        let enrollment = enrollment_record(&state, context.tenant_id(), run_record.enrollment)?;
        let assignment = assignment_record(&state, context.tenant_id(), enrollment.assignment)?;
        require_course_records_accessible(&state, context.tenant_id(), assignment.course_id)?;
        let records = state
            .attempts
            .values()
            .filter(|attempt| attempt.tenant == context.tenant_id() && attempt.run == run)
            .map(|attempt| {
                let projected = projected_attempt(&state, context.tenant_id(), attempt);
                (
                    format!(
                        "{:010}/{:020}/{}",
                        projected.assignment_position,
                        projected.timer.issued_at.as_unix_millis(),
                        projected.id
                    ),
                    projected,
                )
            })
            .collect();
        Ok(page_records(records, &page))
    }
    async fn replay_submission_impl(
        &self,
        context: TenantContext,
        actor: UserId,
        attempt_id: QuestionAttemptId,
        response: &StudentResponse,
        idempotency_key: &SubmissionIdempotencyKey,
    ) -> Result<Option<SubmissionRecord>, StoreError> {
        let state = self.read_state()?;
        let tenant = context.tenant_id();
        let attempt = state
            .attempts
            .get(&(tenant, attempt_id))
            .ok_or(StoreError::NotFound)?;
        let run = state
            .runs
            .get(&(tenant, attempt.run))
            .ok_or(StoreError::NotFound)?;
        let enrollment = enrollment_record(&state, tenant, run.enrollment)?;
        let assignment = assignment_record(&state, tenant, enrollment.assignment)?;
        require_course_records_accessible(&state, tenant, assignment.course_id)?;
        require_attempt_owner(&state, tenant, attempt, actor)?;
        let Some(stored) = state.submissions.get(&(tenant, attempt_id)) else {
            return Ok(None);
        };
        if &stored.key != idempotency_key || &stored.response != response {
            return Err(StoreError::Conflict);
        }
        Ok(Some(stored.record.clone()))
    }
    async fn submit_question_attempt_impl(
        &self,
        context: TenantContext,
        command: SubmitQuestionAttemptCommand,
    ) -> Result<SubmissionRecord, StoreError> {
        let mut state = self.write_state()?;
        let tenant = context.tenant_id();
        let attempt = state
            .attempts
            .get(&(tenant, command.attempt))
            .ok_or(StoreError::NotFound)?;
        let run = state
            .runs
            .get(&(tenant, attempt.run))
            .ok_or(StoreError::NotFound)?;
        let enrollment = enrollment_record(&state, tenant, run.enrollment)?;
        let assignment = assignment_record(&state, tenant, enrollment.assignment)?;
        require_course_records_accessible(&state, tenant, assignment.course_id)?;
        submit_question_attempt_locked(&mut state, context, command)
    }
    async fn force_submit_attempt_impl(
        &self,
        context: TenantContext,
        command: ForceSubmitAttemptCommand,
    ) -> Result<AttemptSupportRecord, StoreError> {
        let mut state = self.write_state()?;
        apply_memory_attempt_support(
            &mut state,
            context,
            command.action,
            command.actor,
            command.attempt,
            AttemptSupportAction::ForceSubmit,
        )
    }
    async fn clear_attempt_impl(
        &self,
        context: TenantContext,
        command: ClearAttemptCommand,
    ) -> Result<AttemptSupportRecord, StoreError> {
        let mut state = self.write_state()?;
        apply_memory_attempt_support(
            &mut state,
            context,
            command.action,
            command.actor,
            command.attempt,
            AttemptSupportAction::Clear,
        )
    }
}

pub(super) fn submit_question_attempt_locked(
    state: &mut State,
    context: TenantContext,
    command: SubmitQuestionAttemptCommand,
) -> Result<SubmissionRecord, StoreError> {
    let tenant = context.tenant_id();
    let base = state
        .attempts
        .get(&(tenant, command.attempt))
        .cloned()
        .ok_or(StoreError::NotFound)?;
    require_attempt_owner(state, tenant, &base, command.actor)?;
    if let Some(stored) = state.submissions.get(&(tenant, command.attempt)) {
        return if stored.key == command.idempotency_key && stored.response == command.response {
            Ok(stored.record.clone())
        } else {
            Err(StoreError::Conflict)
        };
    }
    if projected_attempt(state, tenant, &base).status != AttemptStatus::InProgress {
        return Err(StoreError::Conflict);
    }
    let feedback = private_feedback_record(command.feedback.clone())?;
    let mut run = state
        .runs
        .get(&(tenant, base.run))
        .cloned()
        .ok_or(StoreError::NotFound)?;
    if run.completed_at.is_some() || run.score.is_some() {
        return Err(StoreError::Conflict);
    }
    let mut enrollment = enrollment_record(state, tenant, run.enrollment)?;
    let assignment = assignment_record(state, tenant, enrollment.assignment)?;
    let authored_policy = state
        .published
        .get(&(base.problem, base.question_version))
        .ok_or(StoreError::NotFound)?
        .question
        .timing_policy;
    crate::validate_attempt_result(command.result)?;
    let submitted_at = state.authoritative_time;
    let mut submitted = projected_attempt(state, tenant, &base);
    submitted.response = Some(command.response.clone());
    submitted.status = AttemptStatus::Submitted;
    submitted.result = Some(command.result);
    submitted.timer.submitted_at = Some(submitted_at);
    let effective_policy =
        state
            .attempt_timing
            .get(&(tenant, command.attempt))
            .map_or(authored_policy, |timing| {
                timing
                    .effective_deadline
                    .map_or(TimingPolicy::Untimed, |_| TimingPolicy::PerQuestion {
                        seconds: 1,
                        grace_seconds: timing.effective_grace_seconds,
                    })
            });
    let verdict = timer_verdict(&TimerEvaluation {
        policy: effective_policy,
        timer: submitted.timer,
        evaluated_at: submitted_at,
        pause_extension_millis: 0,
    })
    .map_err(|error| StoreError::InvalidRecord(error.to_string()))?;
    if verdict == TimerVerdict::TimedOut {
        return Err(StoreError::TimedOut);
    }
    require_course_records_accessible(state, tenant, assignment.course_id)?;
    let previous = state
        .summaries
        .get(&(tenant, enrollment.id))
        .cloned()
        .ok_or(StoreError::NotFound)?;
    let mut next = project_summary(
        &previous,
        domain::scoring::RunTransition::QuestionAttemptRecorded { at: submitted_at },
        grade_policy(&assignment),
    )?;
    let run_items = state
        .run_items
        .get(&(tenant, run.id))
        .cloned()
        .ok_or_else(|| StoreError::Unavailable("run has no immutable items".to_string()))?;
    let attempts = state
        .attempts
        .values()
        .filter(|attempt| attempt.tenant == tenant && attempt.run == run.id)
        .map(|attempt| {
            if attempt.id == submitted.id {
                submitted.clone()
            } else {
                projected_attempt(state, tenant, attempt)
            }
        })
        .collect::<Vec<_>>();
    let questions = current_run_questions(&assignment, &run_items, &attempts, &submitted)?;
    let results = questions
        .iter()
        .map(|question| question.map(|question| question.result))
        .collect::<Vec<_>>();
    let submitted_item = run_items
        .iter()
        .find(|item| item.issued_position == submitted.assignment_position)
        .ok_or_else(|| {
            StoreError::Unavailable("submitted attempt has no immutable run item".to_string())
        })?;
    let submitted_assignment_item = submitted_item.assignment_item;
    let (earned_points, possible_points) = crate::current_attempt_points(
        &assignment,
        submitted_assignment_item,
        submitted.status,
        command.result,
    )?;
    let (scoring_generation, _) = state
        .assignment_scoring
        .get(&(tenant, assignment.id))
        .copied()
        .ok_or(StoreError::NotFound)?;
    let mut statistics_contributions = None;
    if let Some(score) = completed_run_score(&questions, assignment.policies.completion)? {
        next = project_summary(
            &next,
            domain::scoring::RunTransition::Completed {
                score,
                at: submitted_at,
            },
            grade_policy(&assignment),
        )?;
        run.completed_at = Some(submitted_at);
        run.score = Some(score);
        project_enrollment_completion(
            &mut enrollment,
            &previous,
            grade_policy(&assignment),
            run.id,
            score,
            submitted_at,
        );
        if run.mode == RunMode::Assigned && previous.completed_run_count == 0 {
            statistics_contributions = Some(derive_statistics_contributions(
                &run_items, &results, &attempts,
            )?);
        }
    }
    if let Some(contributions) = &statistics_contributions {
        stage_statistics_contributions(
            state,
            tenant,
            enrollment.id,
            run.id,
            submitted.id,
            contributions,
        )?;
    }
    let record = SubmissionRecord {
        attempt: submitted,
        run: run.clone(),
        summary: next.clone(),
        feedback,
    };
    state.submissions.insert(
        (tenant, command.attempt),
        StoredSubmission {
            key: command.idempotency_key,
            response: command.response,
            record: record.clone(),
        },
    );
    state.attempt_scores.insert(
        (tenant, command.attempt),
        MemoryAttemptScore {
            assignment: assignment.id,
            assignment_item: submitted_assignment_item,
            generation: scoring_generation,
            earned_points,
            possible_points,
        },
    );
    state.runs.insert((tenant, run.id), run);
    state
        .enrollments
        .insert((tenant, enrollment.id), enrollment);
    state.summaries.insert((tenant, next.enrollment), next);
    complete_memory_attempt_timing_job(state, tenant, command.attempt);
    Ok(record)
}
