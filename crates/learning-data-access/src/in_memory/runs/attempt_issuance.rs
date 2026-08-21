use question_model::{AttemptStatus, AttemptTimerRecord, QuestionAttempt};

use crate::{
    IssueQuestionAttemptCommand, JobId, JobPayload, JobState, ReceiptNextAttempt, StoreError,
    TenantContext, issued_attempt_capability_from_issue, validate_issued_flat_grading,
    validate_issued_presentation, validate_issued_webwork_grading, validate_issued_webwork_replay,
    webwork_replay_state_from_issue,
};

use super::super::{
    MemoryAttemptTiming, MemoryStore, StoredJob, assignment_record, enrollment_record,
    issued_timer, memory_effective_policy_inputs_for_grant, projected_attempt,
    require_course_records_accessible, store_issued_effective_policy_receipt,
    timing_policy_grace_seconds,
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
    let assignment = assignment_record(&state, tenant, enrollment.assignment)?;
    require_course_records_accessible(&state, tenant, assignment.course_id)?;
    let domain::entitlement::EntitlementDecision::Granted(grant) =
        super::super::entitlement::evaluate_locked(
            &state,
            tenant,
            command.actor,
            assignment.course_id,
            assignment.id,
        )?
    else {
        return Err(StoreError::NotFound);
    };
    if grant.student() != enrollment.student {
        return Err(StoreError::NotFound);
    }
    let run_items = state
        .run_items
        .get(&(tenant, command.run))
        .ok_or(StoreError::NotFound)?;
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
            || prefetched.presentation_capability != command.presentation_capability
            || Some(prefetched.presentation) != command.presentation
            || Some(&prefetched.presentation_snapshot) != command.presentation_snapshot.as_ref()
            || Some(&prefetched.grading_envelope) != command.grading_envelope.as_ref()
            || prefetched.flat_grading != command.flat_grading
            || prefetched.flat_grading_capability != command.flat_grading_capability
            || prefetched.webwork_replay != command.webwork_replay
            || prefetched.webwork_grading != command.webwork_grading
            || prefetched.webwork_grading_capability != command.webwork_grading_capability
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
            if state
                .attempt_presentation_capabilities
                .get(&(tenant, active.id))
                != Some(&command.presentation_capability)
            {
                return Err(StoreError::Conflict);
            }
            if state.attempt_presentations.get(&(tenant, active.id))
                != command.presentation.as_ref()
            {
                return Err(StoreError::Conflict);
            }
            if state
                .attempt_presentation_snapshots
                .get(&(tenant, active.id))
                != command.presentation_snapshot.as_ref()
            {
                return Err(StoreError::Conflict);
            }
            if state.attempt_grading_envelopes.get(&(tenant, active.id))
                != command.grading_envelope.as_ref()
            {
                return Err(StoreError::Conflict);
            }
            if state.attempt_flat_grading.get(&(tenant, active.id)) != command.flat_grading.as_ref()
            {
                return Err(StoreError::Conflict);
            }
            if state
                .attempt_flat_grading_capabilities
                .get(&(tenant, active.id))
                != Some(&command.flat_grading_capability)
            {
                return Err(StoreError::Conflict);
            }
            if state.attempt_webwork_grading.get(&(tenant, active.id))
                != command.webwork_grading.as_ref()
                || state
                    .attempt_webwork_grading_capabilities
                    .get(&(tenant, active.id))
                    != Some(&command.webwork_grading_capability)
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
                    Some(Some(existing)) if existing.id != active.id => {
                        return Err(StoreError::Conflict);
                    }
                    Some(None) => return Err(StoreError::Conflict),
                    _ => {
                        state.submission_next_attempts.insert(
                            (tenant, predecessor),
                            Some(ReceiptNextAttempt::from_attempt(&active)),
                        );
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
    let presentation_capability = prefetched
        .map(|value| value.presentation_capability)
        .unwrap_or(command.presentation_capability);
    let presentation = prefetched
        .map(|value| Some(value.presentation))
        .unwrap_or(command.presentation);
    let presentation_snapshot = prefetched
        .map(|value| Some(&value.presentation_snapshot))
        .unwrap_or(command.presentation_snapshot.as_ref());
    let grading_envelope = prefetched
        .map(|value| Some(&value.grading_envelope))
        .unwrap_or(command.grading_envelope.as_ref());
    let flat_grading = prefetched
        .and_then(|value| value.flat_grading.as_ref())
        .or(command.flat_grading.as_ref());
    let flat_grading_capability = prefetched
        .map(|value| value.flat_grading_capability)
        .unwrap_or(command.flat_grading_capability);
    let webwork_replay = prefetched
        .and_then(|value| value.webwork_replay.clone())
        .or(command.webwork_replay.clone());
    let webwork_grading = prefetched
        .and_then(|value| value.webwork_grading.as_ref())
        .or(command.webwork_grading.as_ref());
    let webwork_grading_capability = prefetched
        .map(|value| value.webwork_grading_capability)
        .unwrap_or(command.webwork_grading_capability);
    if parameter_hash.trim().is_empty() || provenance.rendered_question_sha256.trim().is_empty() {
        return Err(StoreError::InvalidRecord(
            "issued attempt hashes must not be empty".to_string(),
        ));
    }
    let question = state
        .published
        .get(&(command.problem, command.question_version))
        .ok_or(StoreError::NotFound)?;
    validate_issued_flat_grading(
        &question.question,
        presentation_capability,
        flat_grading_capability,
        flat_grading,
    )?;
    validate_issued_webwork_grading(
        &question.question,
        webwork_grading_capability,
        webwork_grading,
    )?;
    validate_issued_webwork_replay(webwork_grading_capability, webwork_replay.as_ref())?;
    let issued_capability = issued_attempt_capability_from_issue(
        presentation_capability,
        flat_grading_capability,
        webwork_grading_capability,
    )?;
    let authored_timer = issued_timer(
        state.authoritative_time,
        &run,
        question.question.timing_policy,
    )?;
    let authored_grace_seconds = timing_policy_grace_seconds(question.question.timing_policy);
    let inputs = memory_effective_policy_inputs_for_grant(&state, tenant, assignment.id, &grant)?;
    let decision = domain::effective_assignment_policy::resolve_effective_policy(
        domain::effective_assignment_policy::ResolveEffectivePolicyInput {
            lifecycle: domain::effective_assignment_policy::assignment_lifecycle_gate(
                assignment.lifecycle,
            ),
            entitlement: domain::entitlement::EntitlementDecision::Granted(grant),
            authorization: domain::effective_assignment_policy::AuthorizationGate::Authorized,
            now: state.authoritative_time,
            prior_run_count: run.run_number.saturating_sub(1),
            base: inputs.base,
            group_schedule_offsets: inputs.schedule_offsets,
            group_accommodations: inputs.accommodations,
            individual_exception: inputs.individual,
        },
    )
    .map_err(|error| {
        StoreError::InvalidRecord(format!("invalid effective policy inputs: {error:?}"))
    })?;
    let domain::effective_assignment_policy::EffectivePolicyDecision::Allowed {
        policy,
        start: domain::effective_assignment_policy::StartVerdict::MayStart { .. },
    } = decision
    else {
        return Err(StoreError::NotFound);
    };
    let (effective_deadline, effective_grace_seconds, auto_submit_at) = effective_attempt_deadline(
        &run,
        authored_timer.deadline,
        authored_grace_seconds,
        &policy,
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
        issued_capability,
    };
    // Validate the issued binding before any attempt, receipt, or run mutation.
    // Later submission only copies this owned snapshot; it never reconstructs it.
    let presentation_snapshot = validate_issued_presentation(
        presentation_capability,
        &attempt,
        presentation,
        presentation_snapshot,
        grading_envelope,
    )?;
    let webwork_replay = if webwork_grading_capability.requires_contract() {
        let mapping = webwork_replay.ok_or_else(|| {
            StoreError::InvalidRecord("WeBWorK replay mapping is missing".to_string())
        })?;
        let presentation = presentation.ok_or_else(|| {
            StoreError::InvalidRecord("WeBWorK replay lacks a presentation binding".to_string())
        })?;
        Some(webwork_replay_state_from_issue(
            attempt.problem,
            attempt.question_version,
            attempt.seed,
            &attempt.provenance,
            presentation,
            mapping,
        )?)
    } else {
        None
    };
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
            Some(Some(existing)) if existing.id != attempt.id => {
                return Err(StoreError::Conflict);
            }
            Some(None) => return Err(StoreError::Conflict),
            _ => {}
        }
    }
    let materialized = super::super::entitlement::materialize_locked(
        &mut state,
        tenant,
        crate::MaterializeAssignmentEntitlementCommand::for_learner_action(
            command.actor,
            assignment.course_id,
            assignment.id,
            question_model::EntitlementPurpose::GradeBearingAction,
        )?,
    )?;
    let crate::AssignmentEntitlementMaterialization::Granted(materialized) = materialized else {
        return Err(StoreError::NotFound);
    };
    if materialized.enrollment.id != enrollment.id {
        return Err(StoreError::NotFound);
    }
    if let Some(prefetched) = prefetched {
        state.prefetched_questions.remove(&(
            tenant,
            command.run,
            prefetched.predecessor,
            command.assignment_position,
        ));
    }
    if let Some(predecessor) = command.predecessor_submission {
        state.submission_next_attempts.insert(
            (tenant, predecessor),
            Some(ReceiptNextAttempt::from_attempt(&attempt)),
        );
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
    store_issued_effective_policy_receipt(&mut state, tenant, attempt.id, *policy)?;
    state.attempts.insert((tenant, attempt.id), attempt.clone());
    state
        .attempt_presentation_capabilities
        .insert((tenant, attempt.id), presentation_capability);
    state
        .attempt_flat_grading_capabilities
        .insert((tenant, attempt.id), flat_grading_capability);
    state
        .attempt_webwork_grading_capabilities
        .insert((tenant, attempt.id), webwork_grading_capability);
    if let Some(presentation) = presentation {
        state
            .attempt_presentations
            .insert((tenant, attempt.id), presentation);
    }
    if let Some(snapshot) = presentation_snapshot {
        state
            .attempt_presentation_snapshots
            .insert((tenant, attempt.id), snapshot);
    }
    if let Some(grading_envelope) = grading_envelope {
        state
            .attempt_grading_envelopes
            .insert((tenant, attempt.id), grading_envelope.clone());
    }
    if let Some(flat_grading) = flat_grading {
        state
            .attempt_flat_grading
            .insert((tenant, attempt.id), flat_grading.clone());
    }
    if let Some(webwork_grading) = webwork_grading {
        state
            .attempt_webwork_grading
            .insert((tenant, attempt.id), webwork_grading.clone());
    }
    if let Some(replay) = webwork_replay {
        state
            .webwork_grade_replay
            .insert((tenant, attempt.id), replay);
    }
    Ok(attempt)
}

pub(crate) fn effective_attempt_deadline(
    run: &question_model::AssignmentRun,
    authored_deadline: Option<question_model::ActivityTimestamp>,
    authored_grace_seconds: u32,
    policy: &domain::effective_assignment_policy::EffectiveAssignmentPolicy,
) -> Result<
    (
        Option<question_model::ActivityTimestamp>,
        u32,
        Option<question_model::ActivityTimestamp>,
    ),
    StoreError,
> {
    let mut resolved = authored_deadline.map(|deadline| (deadline, authored_grace_seconds));
    let mut consider = |deadline, grace| {
        if resolved.is_none_or(|current| (deadline, grace) < current) {
            resolved = Some((deadline, grace));
        }
    };
    if let Some(limit) = policy.time_limit_seconds.value {
        consider(
            super::super::add_seconds(run.started_at, limit.get(), "assignment time limit")?,
            0,
        );
    }
    if policy.late_submission.value == question_model::LateSubmissionPolicy::Reject
        && let Some(due) = policy.due_at.value
    {
        consider(due, 0);
    }
    if let Some(close) = policy.closes_at.value {
        consider(close, 0);
    }
    let auto_submit_at = resolved
        .map(|(deadline, grace)| {
            super::super::add_seconds(deadline, grace, "attempt auto-submit deadline")
        })
        .transpose()?;
    Ok((
        resolved.map(|(deadline, _)| deadline),
        resolved.map_or(0, |(_, grace)| grace),
        auto_submit_at,
    ))
}
