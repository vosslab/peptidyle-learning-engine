use question_model::{AttemptStatus, AttemptTimerRecord, QuestionAttempt};

use crate::{
    IssueQuestionAttemptCommand, JobId, JobPayload, JobState, StoreError, TenantContext,
    webwork_replay_state_from_issue,
};

use super::super::{
    MemoryAttemptTiming, MemoryStore, StoredJob, assignment_record, enrollment_record,
    issued_timer, memory_resolved_assignment_policy, projected_attempt,
    require_course_records_accessible, resolved_memory_attempt_timing, timing_policy_grace_seconds,
    validate_assignment_position,
};

pub(super) async fn issue_or_resume_question_attempt(
    store: &MemoryStore,
    context: TenantContext,
    command: IssueQuestionAttemptCommand,
) -> Result<QuestionAttempt, StoreError> {
    let mut state = store.write_state()?;
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
            || Some(prefetched.presentation) != command.presentation
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
                && projected_attempt(&state, tenant, attempt).status == AttemptStatus::InProgress
        })
        .max_by_key(|attempt| (attempt.timer.issued_at, attempt.id));
    if let Some(active) = unresolved.cloned() {
        if active.assignment_position == command.assignment_position {
            if state.attempt_presentations.get(&(tenant, active.id))
                != command.presentation.as_ref()
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
        .map(|value| Some(value.presentation))
        .unwrap_or(command.presentation);
    let webwork_replay = prefetched
        .and_then(|value| value.webwork_replay.clone())
        .or(command.webwork_replay.clone());
    if parameter_hash.trim().is_empty() || provenance.rendered_question_sha256.trim().is_empty() {
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
            let presentation = presentation.ok_or_else(|| {
                StoreError::InvalidRecord("WeBWorK replay lacks a presentation binding".to_string())
            })?;
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
    if let Some(presentation) = presentation {
        state
            .attempt_presentations
            .insert((tenant, attempt.id), presentation);
    }
    if let Some(replay) = webwork_replay {
        state
            .webwork_grade_replay
            .insert((tenant, attempt.id), replay);
    }
    Ok(attempt)
}
