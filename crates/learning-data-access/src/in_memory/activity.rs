use async_trait::async_trait;

use super::*;
use crate::{ActorContext, StudentAssignmentSummarySnapshot};

#[async_trait]
impl crate::ActivityStore for MemoryStore {
    async fn instructor_get_enrollment_impl(
        &self,
        context: ActorContext,
        actor: UserId,
        enrollment: EnrollmentId,
    ) -> Result<Option<AssignmentEnrollment>, StoreError> {
        let state = self.read_state()?;
        if context.user_id() != actor {
            return Ok(None);
        }
        let Some(record) = state.enrollments.get(&enrollment).cloned() else {
            return Ok(None);
        };
        let assignment = assignment_record(&state, record.assignment)?;
        require_course_records_accessible(&state, assignment.course_id)?;
        let instructor =
            super::entitlement::active_membership_for(&state, assignment.course_id, actor)
                .is_some_and(|membership| membership.role == CourseMembershipRole::Instructor);
        if instructor {
            Ok(Some(record))
        } else {
            Ok(None)
        }
    }
    async fn student_get_enrollment_impl(
        &self,
        context: ActorContext,
        actor: UserId,
        enrollment: EnrollmentId,
    ) -> Result<Option<AssignmentEnrollment>, StoreError> {
        let state = self.read_state()?;
        if context.user_id() != actor {
            return Ok(None);
        }
        let Some(record) = state.enrollments.get(&enrollment).cloned() else {
            return Ok(None);
        };
        let assignment = assignment_record(&state, record.assignment)?;
        require_course_records_accessible(&state, assignment.course_id)?;
        if super::entitlement::require_current_enrollment_entitlement(
            &state,
            actor,
            assignment.course_id,
            assignment.id,
            &record,
        )
        .is_err()
        {
            return Ok(None);
        }
        Ok(Some(record))
    }
    async fn student_get_enrollment_for_assignment_impl(
        &self,
        context: ActorContext,
        actor: UserId,
        assignment: AssignmentId,
    ) -> Result<Option<AssignmentEnrollment>, StoreError> {
        let state = self.read_state()?;
        if context.user_id() != actor {
            return Ok(None);
        }
        let current_student = super::entitlement::active_membership_for(
            &state,
            assignment_record(&state, assignment)?.course_id,
            actor,
        )
        .and_then(|membership| membership.student);
        let Some(current_student) = current_student else {
            return Ok(None);
        };
        let Some(record) = state
            .enrollments
            .values()
            .find(|record| record.assignment == assignment && record.student == current_student)
            .cloned()
        else {
            return Ok(None);
        };
        let assignment = assignment_record(&state, record.assignment)?;
        require_course_records_accessible(&state, assignment.course_id)?;
        if super::entitlement::require_current_enrollment_entitlement(
            &state,
            actor,
            assignment.course_id,
            assignment.id,
            &record,
        )
        .is_err()
        {
            return Ok(None);
        }
        Ok(Some(record))
    }
    async fn student_get_run_impl(
        &self,
        context: ActorContext,
        actor: UserId,
        run: RunId,
    ) -> Result<Option<AssignmentRun>, StoreError> {
        let state = self.read_state()?;
        if context.user_id() != actor {
            return Ok(None);
        }
        let Some(run) = state.runs.get(&run).cloned() else {
            return Ok(None);
        };
        let enrollment = enrollment_record(&state, run.enrollment)?;
        let assignment = assignment_record(&state, enrollment.assignment)?;
        require_course_records_accessible(&state, assignment.course_id)?;
        if super::entitlement::require_current_enrollment_entitlement(
            &state,
            actor,
            assignment.course_id,
            assignment.id,
            &enrollment,
        )
        .is_err()
        {
            return Ok(None);
        }
        Ok(Some(run))
    }
    async fn apply_activity_transition_impl(
        &self,
        _context: ActorContext,
        transition: ActivityTransition,
    ) -> Result<StudentAssignmentSummary, StoreError> {
        let mut state = self.write_state()?;

        let (enrollment_id, assignment, domain_transition) = match &transition {
            ActivityTransition::StartRun { run } => {
                if run.run_number == 0 || run.completed_at.is_some() || run.score.is_some() {
                    return Err(StoreError::InvalidRecord(
                        "new run must be one-based and incomplete".to_string(),
                    ));
                }
                if state.runs.contains_key(&run.id) {
                    return Err(StoreError::AlreadyExists);
                }
                let enrollment = enrollment_record(&state, run.enrollment)?;
                let assignment = assignment_record(&state, enrollment.assignment)?;
                require_course_records_accessible(&state, assignment.course_id)?;
                let expected_mode = match enrollment.status() {
                    EnrollmentStatus::InProgress => RunMode::Assigned,
                    EnrollmentStatus::Completed => RunMode::Practice,
                };
                if run.mode != expected_mode {
                    return Err(StoreError::InvalidRecord(format!(
                        "run mode must be {expected_mode:?} for this enrollment"
                    )));
                }
                if run.variation != assignment.policies.variation {
                    return Err(StoreError::InvalidRecord(
                        "run variation must match its assignment policy".to_string(),
                    ));
                }
                if state.runs.values().any(|existing| {
                    existing.enrollment == run.enrollment && existing.completed_at.is_none()
                }) {
                    return Err(StoreError::InvalidRecord(
                        "an enrollment cannot have two in-progress runs".to_string(),
                    ));
                }
                let expected_run_number = state
                    .runs
                    .values()
                    .filter(|existing| existing.enrollment == run.enrollment)
                    .map(|existing| existing.run_number)
                    .max()
                    .unwrap_or(0)
                    .checked_add(1)
                    .ok_or_else(|| StoreError::InvalidRecord("run number overflow".to_string()))?;
                if run.run_number != expected_run_number {
                    return Err(StoreError::InvalidRecord(format!(
                        "run number must be the next one-based value {expected_run_number}"
                    )));
                }
                (enrollment.id, assignment, summary_transition(&transition))
            }
            ActivityTransition::RecordQuestionAttempt { attempt } => {
                if state.attempts.contains_key(&attempt.id) {
                    return Err(StoreError::AlreadyExists);
                }
                let run = state.runs.get(&attempt.run).ok_or(StoreError::NotFound)?;
                if run.completed_at.is_some() || run.score.is_some() {
                    return Err(StoreError::InvalidRecord(
                        "question attempts cannot be added to a completed run".to_string(),
                    ));
                }
                let enrollment = enrollment_record(&state, run.enrollment)?;
                let assignment = assignment_record(&state, enrollment.assignment)?;
                require_course_records_accessible(&state, assignment.course_id)?;
                let matches_run_item = state.run_items.get(&attempt.run).is_some_and(|items| {
                    items.iter().any(|item| {
                        item.issued_position == attempt.assignment_position
                            && item.reference.problem == attempt.problem
                            && item.reference.version == attempt.question_version
                    })
                });
                if !matches_run_item {
                    return Err(StoreError::InvalidRecord(
                        "question attempt must match an immutable run item".to_string(),
                    ));
                }
                (enrollment.id, assignment, summary_transition(&transition))
            }
            ActivityTransition::CompleteRun { run, .. } => {
                let run_record = state.runs.get(run).ok_or(StoreError::NotFound)?;
                if run_record.completed_at.is_some() || run_record.score.is_some() {
                    return Err(StoreError::InvalidRecord(
                        "completed run cannot be completed again".to_string(),
                    ));
                }
                let enrollment = enrollment_record(&state, run_record.enrollment)?;
                (
                    enrollment.id,
                    {
                        let assignment = assignment_record(&state, enrollment.assignment)?;
                        require_course_records_accessible(&state, assignment.course_id)?;
                        assignment
                    },
                    summary_transition(&transition),
                )
            }
        };

        let summary_key = enrollment_id;
        let enrollment_key = enrollment_id;
        let previous = state
            .summaries
            .get(&summary_key)
            .cloned()
            .ok_or(StoreError::NotFound)?;
        let grade = grade_policy(&assignment);
        if matches!(&transition, ActivityTransition::StartRun { .. })
            && !continued_practice_allows_run(&previous, assignment.policies.continued_practice)
        {
            return Err(StoreError::InvalidRecord(
                "continued-practice policy does not permit another run".to_string(),
            ));
        }
        let next = project_summary(&previous, domain_transition, grade)?;

        match transition {
            ActivityTransition::StartRun { run } => {
                let run_items = select_assignment_run_items(&assignment, &run)?;
                state.run_items.insert(run.id, run_items);
                state.runs.insert(run.id, run);
            }
            ActivityTransition::RecordQuestionAttempt { attempt } => {
                state.attempts.insert(attempt.id, *attempt);
            }
            ActivityTransition::CompleteRun { run, score, at } => {
                {
                    let run_record = state.runs.get_mut(&run).ok_or(StoreError::NotFound)?;
                    run_record.completed_at = Some(at);
                    run_record.score = Some(score);
                }
                let enrollment = state
                    .enrollments
                    .get_mut(&enrollment_key)
                    .ok_or(StoreError::NotFound)?;
                project_enrollment_completion(enrollment, &previous, grade, run, score, at);
            }
        }
        state.summaries.insert(summary_key, next.clone());
        Ok(next)
    }
    async fn get_run_impl(
        &self,
        _context: ActorContext,
        run: RunId,
    ) -> Result<Option<AssignmentRun>, StoreError> {
        let state = self.read_state()?;
        let Some(record) = state.runs.get(&run).cloned() else {
            return Ok(None);
        };
        let enrollment = enrollment_record(&state, record.enrollment)?;
        let assignment = assignment_record(&state, enrollment.assignment)?;
        if !course_records_accessible(&state, assignment.course_id) {
            return Ok(None);
        }
        Ok(Some(record))
    }
    async fn list_runs_impl(
        &self,
        _context: ActorContext,
        enrollment: EnrollmentId,
        page: PageRequest,
    ) -> Result<Page<AssignmentRun>, StoreError> {
        let state = self.read_state()?;
        let enrollment_record = state
            .enrollments
            .get(&enrollment)
            .ok_or(StoreError::NotFound)?;
        let assignment = assignment_record(&state, enrollment_record.assignment)?;
        require_course_records_accessible(&state, assignment.course_id)?;
        let records = state
            .runs
            .iter()
            .filter(|(_, run)| run.enrollment == enrollment)
            .map(|(run_id, run)| (format!("{:010}/{run_id}", run.run_number), run.clone()))
            .collect();
        Ok(page_records(records, &page))
    }
    async fn instructor_list_runs_impl(
        &self,
        context: ActorContext,
        actor: UserId,
        enrollment: EnrollmentId,
        page: PageRequest,
    ) -> Result<Option<Page<AssignmentRun>>, StoreError> {
        let state = self.read_state()?;
        if context.user_id() != actor {
            return Ok(None);
        }
        let Some(record) = state.enrollments.get(&enrollment) else {
            return Ok(None);
        };
        let assignment = assignment_record(&state, record.assignment)?;
        require_course_records_accessible(&state, assignment.course_id)?;
        if super::entitlement::require_current_enrollment_entitlement(
            &state,
            actor,
            assignment.course_id,
            assignment.id,
            record,
        )
        .is_ok()
        {
            return Ok(None);
        }
        let instructor =
            super::entitlement::active_membership_for(&state, assignment.course_id, actor)
                .is_some_and(|membership| membership.role == CourseMembershipRole::Instructor);
        if !instructor {
            return Ok(None);
        }
        let records = state
            .runs
            .iter()
            .filter(|(_, run)| run.enrollment == enrollment)
            .map(|(id, run)| (format!("{:010}/{}", run.run_number, id), run.clone()))
            .collect();
        Ok(Some(page_records(records, &page)))
    }
    async fn student_list_runs_impl(
        &self,
        context: ActorContext,
        actor: UserId,
        enrollment: EnrollmentId,
        page: PageRequest,
    ) -> Result<Option<Page<AssignmentRun>>, StoreError> {
        let state = self.read_state()?;
        if context.user_id() != actor {
            return Ok(None);
        }
        let Some(record) = state.enrollments.get(&enrollment) else {
            return Ok(None);
        };
        let assignment = assignment_record(&state, record.assignment)?;
        require_course_records_accessible(&state, assignment.course_id)?;
        if super::entitlement::require_current_enrollment_entitlement(
            &state,
            actor,
            assignment.course_id,
            assignment.id,
            record,
        )
        .is_err()
        {
            return Ok(None);
        }
        let records = state
            .runs
            .iter()
            .filter(|(_, run)| run.enrollment == enrollment)
            .map(|(id, run)| (format!("{:010}/{}", run.run_number, id), run.clone()))
            .collect();
        Ok(Some(page_records(records, &page)))
    }
    async fn get_question_attempt_impl(
        &self,
        _context: ActorContext,
        attempt: QuestionAttemptId,
    ) -> Result<Option<QuestionAttempt>, StoreError> {
        let state = self.read_state()?;
        let Some(record) = state.attempts.get(&attempt) else {
            return Ok(None);
        };
        let run = state.runs.get(&record.run).ok_or(StoreError::NotFound)?;
        let enrollment = enrollment_record(&state, run.enrollment)?;
        let assignment = assignment_record(&state, enrollment.assignment)?;
        if !course_records_accessible(&state, assignment.course_id) {
            return Ok(None);
        }
        Ok(Some(projected_attempt(&state, record)))
    }
    async fn student_get_question_attempt_impl(
        &self,
        context: ActorContext,
        actor: UserId,
        attempt: QuestionAttemptId,
    ) -> Result<Option<QuestionAttempt>, StoreError> {
        let state = self.read_state()?;
        if context.user_id() != actor {
            return Ok(None);
        }
        let Some(record) = state.attempts.get(&attempt).cloned() else {
            return Ok(None);
        };
        let run = state.runs.get(&record.run).ok_or(StoreError::NotFound)?;
        let enrollment = enrollment_record(&state, run.enrollment)?;
        let assignment = assignment_record(&state, enrollment.assignment)?;
        require_course_records_accessible(&state, assignment.course_id)?;
        if super::entitlement::require_current_enrollment_entitlement(
            &state,
            actor,
            assignment.course_id,
            assignment.id,
            &enrollment,
        )
        .is_err()
        {
            return Ok(None);
        }
        Ok(Some(projected_attempt(&state, &record)))
    }
    async fn get_summary_impl(
        &self,
        _context: ActorContext,
        enrollment: EnrollmentId,
    ) -> Result<Option<StudentAssignmentSummary>, StoreError> {
        let state = self.read_state()?;
        let Some(record) = state.summaries.get(&enrollment).cloned() else {
            return Ok(None);
        };
        let enrollment_record = state
            .enrollments
            .get(&enrollment)
            .ok_or(StoreError::NotFound)?;
        let assignment = assignment_record(&state, enrollment_record.assignment)?;
        if !course_records_accessible(&state, assignment.course_id) {
            return Ok(None);
        }
        Ok(Some(record))
    }
    async fn student_get_summary_impl(
        &self,
        context: ActorContext,
        actor: UserId,
        enrollment: EnrollmentId,
    ) -> Result<Option<StudentAssignmentSummarySnapshot>, StoreError> {
        let state = self.read_state()?;
        if context.user_id() != actor {
            return Ok(None);
        }
        let Some(record) = state.enrollments.get(&enrollment) else {
            return Ok(None);
        };
        let assignment = assignment_record(&state, record.assignment)?;
        require_course_records_accessible(&state, assignment.course_id)?;
        if super::entitlement::require_current_enrollment_entitlement(
            &state,
            actor,
            assignment.course_id,
            assignment.id,
            record,
        )
        .is_err()
        {
            return Ok(None);
        }
        let Some(summary) = state.summaries.get(&enrollment).cloned() else {
            return Ok(None);
        };
        let scoring_status = state
            .assignment_scoring
            .get(&assignment.id)
            .ok_or(StoreError::NotFound)?
            .1;
        Ok(Some(StudentAssignmentSummarySnapshot {
            summary,
            scoring_status,
        }))
    }
}

pub(super) fn issued_timer(
    issued_at: ActivityTimestamp,
    run: &AssignmentRun,
    policy: TimingPolicy,
) -> Result<AttemptTimerRecord, StoreError> {
    let deadline = match policy {
        TimingPolicy::Untimed => None,
        TimingPolicy::PerQuestion { seconds, .. } => {
            Some(add_seconds(issued_at, seconds, "question deadline")?)
        }
        TimingPolicy::PerAttempt { seconds, .. } => {
            let deadline = add_seconds(run.started_at, seconds, "run deadline")?;
            if deadline < issued_at {
                return Err(StoreError::TimedOut);
            }
            Some(deadline)
        }
    };
    Ok(AttemptTimerRecord {
        issued_at,
        deadline,
        submitted_at: None,
    })
}

pub(super) fn timing_policy_grace_seconds(policy: TimingPolicy) -> u32 {
    match policy {
        TimingPolicy::Untimed => 0,
        TimingPolicy::PerQuestion { grace_seconds, .. }
        | TimingPolicy::PerAttempt { grace_seconds, .. } => grace_seconds,
    }
}

pub(super) fn add_seconds(
    timestamp: ActivityTimestamp,
    seconds: u32,
    description: &str,
) -> Result<ActivityTimestamp, StoreError> {
    timestamp
        .as_unix_millis()
        .checked_add(i64::from(seconds) * 1_000)
        .map(ActivityTimestamp::from_unix_millis)
        .ok_or_else(|| StoreError::InvalidRecord(format!("{description} overflow")))
}

pub(super) fn require_attempt_owner(
    state: &State,
    attempt: &QuestionAttempt,
    actor: UserId,
) -> Result<(), StoreError> {
    let run = state.runs.get(&attempt.run).ok_or(StoreError::NotFound)?;
    let enrollment = enrollment_record(state, run.enrollment)?;
    let assignment = assignment_record(state, enrollment.assignment)?;
    state
        .courses
        .contains_key(&assignment.course_id)
        .then_some(())
        .ok_or(StoreError::NotFound)?;
    super::entitlement::require_current_enrollment_entitlement(
        state,
        actor,
        assignment.course_id,
        assignment.id,
        &enrollment,
    )
    .map(|_| ())
}

/// Confirms an attempt belongs to the installation scope represented by its
/// authoritative run record. The attempt identity itself is globally unique.
pub(super) fn attempt_belongs_to_course(state: &State, attempt_id: QuestionAttemptId) -> bool {
    state
        .attempts
        .get(&attempt_id)
        .is_some_and(|attempt| state.runs.contains_key(&attempt.run))
}

pub(super) fn apply_memory_attempt_support(
    state: &mut State,
    context: ActorContext,
    action_id: AttemptSupportActionId,
    actor: UserId,
    attempt_id: QuestionAttemptId,
    action: AttemptSupportAction,
) -> Result<AttemptSupportRecord, StoreError> {
    if context.user_id() != actor {
        return Err(StoreError::NotFound);
    }
    let base = state
        .attempts
        .get(&attempt_id)
        .cloned()
        .ok_or(StoreError::NotFound)?;
    let run = state.runs.get(&base.run).ok_or(StoreError::NotFound)?;
    let enrollment = enrollment_record(state, run.enrollment)?;
    let assignment = assignment_record(state, enrollment.assignment)?;
    require_course_records_accessible(state, assignment.course_id)?;
    if !state.courses.contains_key(&assignment.course_id)
        || !super::entitlement::active_membership_for(state, assignment.course_id, actor)
            .is_some_and(|membership| membership.role == CourseMembershipRole::Instructor)
    {
        return Err(StoreError::NotFound);
    }
    if let Some(existing) = state.attempt_support_actions.get(&action_id) {
        return if existing.actor == actor
            && existing.attempt == attempt_id
            && existing.kind == action
        {
            Ok(*existing)
        } else {
            Err(StoreError::Conflict)
        };
    }

    let previous = projected_attempt(state, &base);
    let resulting_status = match action {
        // ASVS 2.2.1-2.2.3, 2.3.1-2.3.4: force-submit has one authorized,
        // answer-free terminal transition. The support-action receipt preserves
        // exact replay while the attempt remains closed.
        AttemptSupportAction::ForceSubmit if previous.status == AttemptStatus::InProgress => {
            AttemptStatus::AutoSubmitted
        }
        AttemptSupportAction::Clear
            if matches!(
                previous.status,
                AttemptStatus::InProgress | AttemptStatus::Submitted | AttemptStatus::AutoSubmitted
            ) =>
        {
            AttemptStatus::Cleared
        }
        _ => return Err(StoreError::Conflict),
    };
    let now = state.authoritative_time;
    let mut current = previous.clone();
    current.status = resulting_status;
    if action == AttemptSupportAction::ForceSubmit {
        current.timer.submitted_at = Some(now);
    }

    let requires_scoring_invalidation =
        action == AttemptSupportAction::Clear && previous.result.is_some();
    let record = AttemptSupportRecord {
        action: action_id,
        actor,
        attempt: attempt_id,
        kind: action,
        previous_status: previous.status,
        resulting_status,
        occurred_at: now,
    };

    if requires_scoring_invalidation {
        super::scoring_invalidation::request_scoring_invalidation(
            state,
            assignment.course_id,
            assignment.id,
            crate::ScoringInvalidationOrigin::student_support(
                crate::ScoringInvalidationOriginId::from_uuid(action_id.as_uuid()),
                actor,
            ),
            crate::JobId::from_uuid(action_id.as_uuid()),
        )?;
    }
    state.attempt_current.insert(attempt_id, current);
    state.webwork_grade_replay.remove(&attempt_id);
    complete_memory_attempt_timing_job(state, attempt_id);
    state.attempt_support_actions.insert(action_id, record);
    Ok(record)
}

pub(super) fn complete_memory_attempt_timing_job(state: &mut State, attempt: QuestionAttemptId) {
    let job = state
        .attempt_timing
        .get_mut(&attempt)
        .and_then(|timing| timing.job.take());
    let Some(job) = job else {
        return;
    };
    if let Some(stored) = state.jobs.get_mut(&job)
        && matches!(stored.state, JobState::Ready | JobState::Leased)
    {
        stored.state = JobState::Completed;
        stored.lease_token = None;
        stored.lease_expires_at = None;
    }
}

pub(super) fn projected_attempt(state: &State, attempt: &QuestionAttempt) -> QuestionAttempt {
    let mut projected = state
        .attempt_current
        .get(&attempt.id)
        .cloned()
        .or_else(|| {
            state
                .submissions
                .get(&attempt.id)
                .and_then(|stored| stored.completed_record_opt())
                .map(|record| record.attempt.clone())
        })
        .unwrap_or_else(|| attempt.clone());
    if let Some(timing) = state.attempt_timing.get(&attempt.id) {
        projected.timer.deadline = timing.effective_deadline;
    }
    projected
}
