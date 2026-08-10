use question_model::run_policy::TimingPolicy;
use question_model::{
    ActivityTimestamp, AssignmentRun, AttemptStatus, AttemptTimerRecord, CourseId,
    PresentationBindingV1, QuestionAttempt, TenantId,
};
use sqlx::types::Uuid;
use sqlx::{Postgres, Row, Transaction};

use crate::{
    IssueQuestionAttemptCommand, JobId, JobPayload, PrefetchedQuestion, StoreError, TenantContext,
    WebworkReplayMappingV1, webwork_replay_state_from_issue,
};

use super::super::assignment_records::{
    load_assignment_for_share, load_enrollment_for_update, load_run_for_update,
};
use super::super::assignment_timing;
use super::super::connection::map_sqlx_error;
use super::super::row_decode::{
    decode_current_attempt_row, decode_payload_row, decode_presentation_binding_row, encode_payload,
};
use super::super::submission::load_attempt_for_external_update;
use super::super::transaction_context::{database_timestamp, load_published_record};

#[cfg(feature = "postgres")]
pub(super) async fn issue_or_resume_question_attempt(
    transaction: &mut Transaction<'_, Postgres>,
    context: TenantContext,
    command: IssueQuestionAttemptCommand,
) -> Result<QuestionAttempt, StoreError> {
    let tenant = context.tenant_id();
    let run = load_run_for_update(transaction, tenant, command.run).await?;
    if run.completed_at.is_some() || run.score.is_some() {
        return Err(StoreError::InvalidRecord(
            "a completed run cannot issue another question".to_string(),
        ));
    }
    let enrollment = load_enrollment_for_update(transaction, tenant, run.enrollment).await?;
    if enrollment.user != command.actor {
        return Err(StoreError::Forbidden);
    }
    assignment_timing::lock_postgres_assignment_policy(transaction, tenant, enrollment.assignment)
        .await?;
    let assignment_guard =
        load_assignment_for_share(transaction, tenant, enrollment.assignment).await?;
    let resolved_assignment_timing = assignment_timing::load_postgres_resolved_assignment_policy(
        transaction,
        tenant,
        enrollment.assignment,
        &enrollment,
        None,
    )
    .await?;
    validate_postgres_assignment_position(transaction, tenant, &command).await?;
    let assignment_position = i32::try_from(command.assignment_position)
        .map_err(|_| StoreError::InvalidRecord("assignment position is too large".to_string()))?;
    if let Some(prefetched) = command.prefetched.as_ref()
        && (prefetched.tenant != tenant
            || prefetched.run != command.run
            || command.predecessor_submission != Some(prefetched.predecessor)
            || prefetched.assignment_position != command.assignment_position
            || prefetched.problem != command.problem
            || prefetched.question_version != command.question_version
            || Some(prefetched.presentation) != command.presentation
            || prefetched.webwork_replay != command.webwork_replay)
    {
        return Err(StoreError::Conflict);
    }

    let unresolved = sqlx::query(
        "SELECT qa.payload, qa.payload_sha256, \
                qa.presentation_descriptor_version, qa.presentation_nonce, qa.presentation_digest, \
                qa.attempt_status AS current_attempt_status, \
                floor(extract(epoch FROM qa.submitted_at) * 1000)::bigint AS current_submitted_at, \
                floor(extract(epoch FROM timing.effective_deadline) * 1000)::bigint \
                    AS current_deadline_at \
         FROM question_attempt AS qa \
         LEFT JOIN submission_idempotency AS si \
           ON si.tenant_id = qa.tenant_id AND si.attempt_id = qa.attempt_id \
         LEFT JOIN attempt_timing_current AS timing \
           ON timing.tenant_id = qa.tenant_id AND timing.attempt_id = qa.attempt_id \
         WHERE qa.tenant_id = $1 AND qa.run_id = $2 \
           AND qa.attempt_status = 'in_progress' AND si.attempt_id IS NULL \
         ORDER BY qa.occurred_at DESC, qa.attempt_id::text DESC LIMIT 1",
    )
    .bind(tenant.as_uuid())
    .bind(run.id.as_uuid())
    .fetch_optional(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    if let Some(row) = unresolved {
        let active = decode_current_attempt_row(&row)?;
        if active.assignment_position == command.assignment_position {
            if decode_presentation_binding_row(&row)? != command.presentation {
                return Err(StoreError::Conflict);
            }
            if let Some(predecessor) = command.predecessor_submission {
                // Converging healers must attach the already-issued active
                // attempt to the durable predecessor receipt before return.
                // Select the persisted timestamp rather than the public
                // millisecond timer value so the partitioned FK is exact.
                let inserted = sqlx::query(
                    "INSERT INTO submission_next_attempt \
                     (tenant_id, predecessor_attempt_id, next_attempt_id, next_attempt_occurred_at) \
                     SELECT $1, $2, $3, active.occurred_at \
                     FROM question_attempt AS active \
                     JOIN question_attempt AS predecessor_attempt \
                       ON predecessor_attempt.tenant_id = active.tenant_id \
                      AND predecessor_attempt.run_id = active.run_id \
                     JOIN submission_idempotency AS submitted \
                       ON submitted.tenant_id = predecessor_attempt.tenant_id \
                      AND submitted.attempt_id = predecessor_attempt.attempt_id \
                     WHERE active.tenant_id = $1 AND active.attempt_id = $3 \
                       AND predecessor_attempt.attempt_id = $2 \
                     ON CONFLICT (tenant_id, predecessor_attempt_id) DO NOTHING",
                )
                .bind(tenant.as_uuid())
                .bind(predecessor.as_uuid())
                .bind(active.id.as_uuid())
                .execute(&mut **transaction)
                .await
                .map_err(map_sqlx_error)?;
                if inserted.rows_affected() == 0 {
                    let existing: Option<Option<Uuid>> = sqlx::query_scalar(
                        "SELECT next_attempt_id FROM submission_next_attempt \
                         WHERE tenant_id = $1 AND predecessor_attempt_id = $2",
                    )
                    .bind(tenant.as_uuid())
                    .bind(predecessor.as_uuid())
                    .fetch_optional(&mut **transaction)
                    .await
                    .map_err(map_sqlx_error)?
                    .flatten();
                    if existing != Some(Some(active.id.as_uuid())) {
                        return Err(StoreError::Conflict);
                    }
                }
            }
            return Ok(active);
        }
        return Err(StoreError::InvalidRecord(
            "another question attempt is already active in this run".to_string(),
        ));
    }
    let prefetched = command.prefetched.as_ref();
    if let Some(prefetched) = prefetched {
        if prefetched.tenant != tenant
            || prefetched.run != command.run
            || command.predecessor_submission != Some(prefetched.predecessor)
            || prefetched.assignment_position != command.assignment_position
            || prefetched.problem != command.problem
            || prefetched.question_version != command.question_version
            || Some(prefetched.presentation) != command.presentation
            || prefetched.webwork_replay != command.webwork_replay
        {
            return Err(StoreError::Conflict);
        }
        let row = sqlx::query(
            "SELECT payload, payload_sha256, presentation_descriptor_version, \
                    presentation_nonce, presentation_digest FROM question_prefetch \
             WHERE tenant_id = $1 AND run_id = $2 AND predecessor_attempt_id = $3 AND assignment_position = $4 FOR UPDATE",
        )
        .bind(tenant.as_uuid())
        .bind(command.run.as_uuid())
        .bind(prefetched.predecessor.as_uuid())
        .bind(assignment_position)
        .fetch_optional(&mut **transaction)
        .await
        .map_err(map_sqlx_error)?
        .ok_or(StoreError::Conflict)?;
        let stored: PrefetchedQuestion = decode_payload_row(&row)?;
        if decode_presentation_binding_row(&row)? != Some(stored.presentation) {
            return Err(StoreError::Unavailable(
                "stored prefetch presentation disagrees with its columns".to_string(),
            ));
        }
        let predecessor_submitted: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM submission_idempotency WHERE tenant_id = $1 AND attempt_id = $2)",
        )
        .bind(tenant.as_uuid())
        .bind(prefetched.predecessor.as_uuid())
        .fetch_one(&mut **transaction)
        .await
        .map_err(map_sqlx_error)?;
        if stored != *prefetched || !predecessor_submitted {
            return Err(StoreError::Conflict);
        }
    }
    let latest_submission = sqlx::query(
        "SELECT si.payload, si.payload_sha256 FROM question_attempt AS qa \
         JOIN submission_idempotency AS si \
           ON si.tenant_id = qa.tenant_id AND si.attempt_id = qa.attempt_id \
         WHERE qa.tenant_id = $1 AND qa.run_id = $2 AND qa.assignment_position = $3 \
           AND qa.attempt_status NOT IN ('cleared', 'exempt') \
         ORDER BY si.submitted_at DESC, qa.attempt_id::text DESC LIMIT 1",
    )
    .bind(tenant.as_uuid())
    .bind(run.id.as_uuid())
    .bind(assignment_position)
    .fetch_optional(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    if let Some(row) = latest_submission {
        let latest: QuestionAttempt = decode_payload_row(&row)?;
        if latest.result.is_some_and(|result| result.correct) {
            return Err(StoreError::InvalidRecord(
                "a correct question position cannot be retried".to_string(),
            ));
        }
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
    let question =
        load_published_record(transaction, command.problem, command.question_version).await?;
    let issued_at = database_timestamp(transaction).await?;
    let authored_timer = issued_timer(issued_at, &run, question.question.timing_policy)?;
    let authored_grace_seconds =
        assignment_timing::timing_policy_grace_seconds(question.question.timing_policy);
    let assignment_timing::ResolvedPostgresAttemptTiming {
        effective_deadline,
        effective_grace_seconds,
        auto_submit_at,
        resolution_kind,
    } = assignment_timing::resolved_postgres_attempt_timing(
        resolved_assignment_timing.policy,
        &run,
        authored_timer.deadline,
        authored_grace_seconds,
    )?;
    if effective_deadline.is_some_and(|deadline| deadline < issued_at)
        || auto_submit_at.is_some_and(|deadline| deadline <= issued_at)
    {
        return Err(StoreError::TimedOut);
    }
    let timer = AttemptTimerRecord {
        deadline: effective_deadline,
        ..authored_timer
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
    let duplicate: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM question_attempt \
         WHERE tenant_id = $1 AND attempt_id = $2)",
    )
    .bind(tenant.as_uuid())
    .bind(attempt.id.as_uuid())
    .fetch_one(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    if duplicate {
        return Err(StoreError::AlreadyExists);
    }
    let (payload, checksum) = encode_payload(&attempt)?;
    sqlx::query(
        "INSERT INTO question_attempt \
         (tenant_id, attempt_id, run_id, problem_id, version_id, assignment_position, \
          occurred_at, payload, payload_sha256, presentation_descriptor_version, \
          presentation_nonce, presentation_digest) \
         VALUES ($1, $2, $3, $4, $5, $6, transaction_timestamp(), $7, $8, $9, $10, $11)",
    )
    .bind(tenant.as_uuid())
    .bind(attempt.id.as_uuid())
    .bind(attempt.run.as_uuid())
    .bind(attempt.problem.as_uuid())
    .bind(attempt.question_version.as_uuid())
    .bind(assignment_position)
    .bind(payload)
    .bind(checksum)
    .bind(presentation.map(|value| i16::from(value.descriptor_version())))
    .bind(presentation.map(|value| value.nonce().as_bytes().to_vec()))
    .bind(presentation.map(|value| value.digest().as_bytes().to_vec()))
    .execute(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    if let Some(mapping) = webwork_replay {
        let presentation = presentation.ok_or_else(|| {
            StoreError::InvalidRecord("WeBWorK replay lacks a presentation binding".to_string())
        })?;
        insert_webwork_grade_replay_state(
            transaction,
            assignment_guard.course_id,
            &attempt,
            presentation,
            mapping,
        )
        .await?;
    }
    let timing_generation = 1_u64;
    let timing_job = if let Some(available_at) = auto_submit_at {
        let job = JobId::generate()?;
        let payload = serde_json::to_value(JobPayload::AutoSubmitAttempt {
            attempt: attempt.id,
            timing_generation,
        })
        .map_err(|error| StoreError::InvalidRecord(error.to_string()))?;
        sqlx::query(
            "INSERT INTO worker_job \
             (job_id, tenant_id, payload, state, available_at, max_attempts) \
             VALUES ($1, $2, $3, 'ready', \
                TIMESTAMPTZ 'epoch' + $4::bigint * INTERVAL '1 millisecond', 10)",
        )
        .bind(job.as_uuid())
        .bind(tenant.as_uuid())
        .bind(payload)
        .bind(available_at.as_unix_millis())
        .execute(&mut **transaction)
        .await
        .map_err(map_sqlx_error)?;
        Some(job)
    } else {
        None
    };
    let timing_inserted = sqlx::query(
        "INSERT INTO attempt_timing_current \
         (tenant_id, attempt_id, attempt_occurred_at, assignment_id, course_id, \
          authored_deadline, authored_grace_seconds, effective_deadline, \
          effective_grace_seconds, auto_submit_at, resolution_kind, resolved_visible, \
          resolved_available_at, resolved_due_at, resolved_closes_at, \
          resolved_late_submission_policy, resolved_time_limit_seconds, \
          resolved_attempt_limit, resolution_sources, timing_generation, job_id) \
         SELECT $1, $2, attempt.occurred_at, $3, $4, \
                TIMESTAMPTZ 'epoch' + $5::bigint * INTERVAL '1 millisecond', $6, \
                TIMESTAMPTZ 'epoch' + $7::bigint * INTERVAL '1 millisecond', $8, \
                TIMESTAMPTZ 'epoch' + $9::bigint * INTERVAL '1 millisecond', $10, $11, \
                TIMESTAMPTZ 'epoch' + $12::bigint * INTERVAL '1 millisecond', \
                TIMESTAMPTZ 'epoch' + $13::bigint * INTERVAL '1 millisecond', \
                TIMESTAMPTZ 'epoch' + $14::bigint * INTERVAL '1 millisecond', \
                $15, $16, $17, $18, $19, $20 \
           FROM question_attempt AS attempt \
          WHERE attempt.tenant_id = $1 AND attempt.attempt_id = $2",
    )
    .bind(tenant.as_uuid())
    .bind(attempt.id.as_uuid())
    .bind(assignment_guard.id.as_uuid())
    .bind(assignment_guard.course_id.as_uuid())
    .bind(authored_timer.deadline.map(|value| value.as_unix_millis()))
    .bind(i64::from(authored_grace_seconds))
    .bind(effective_deadline.map(|value| value.as_unix_millis()))
    .bind(i64::from(effective_grace_seconds))
    .bind(auto_submit_at.map(|value| value.as_unix_millis()))
    .bind(resolution_kind)
    .bind(resolved_assignment_timing.policy.visible)
    .bind(
        resolved_assignment_timing
            .policy
            .available_at
            .map(|value| value.as_unix_millis()),
    )
    .bind(
        resolved_assignment_timing
            .policy
            .due_at
            .map(|value| value.as_unix_millis()),
    )
    .bind(
        resolved_assignment_timing
            .policy
            .closes_at
            .map(|value| value.as_unix_millis()),
    )
    .bind(assignment_timing::late_submission_policy_name(
        resolved_assignment_timing.policy.late_submission,
    ))
    .bind(
        resolved_assignment_timing
            .policy
            .time_limit_seconds
            .map(i64::from),
    )
    .bind(
        resolved_assignment_timing
            .policy
            .attempt_limit
            .map(i64::from),
    )
    .bind(
        serde_json::to_value(&resolved_assignment_timing.contributors)
            .map_err(|error| StoreError::InvalidRecord(error.to_string()))?,
    )
    .bind(i64::try_from(timing_generation).expect("initial generation fits"))
    .bind(timing_job.map(JobId::as_uuid))
    .execute(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    if timing_inserted.rows_affected() != 1 {
        return Err(StoreError::Conflict);
    }
    if let Some(prefetched) = prefetched {
        sqlx::query(
            "DELETE FROM question_prefetch WHERE tenant_id = $1 AND run_id = $2 AND predecessor_attempt_id = $3 AND assignment_position = $4",
        )
        .bind(tenant.as_uuid())
        .bind(command.run.as_uuid())
        .bind(prefetched.predecessor.as_uuid())
        .bind(assignment_position)
        .execute(&mut **transaction)
        .await
        .map_err(map_sqlx_error)?;
    }
    if let Some(predecessor) = command.predecessor_submission {
        if load_attempt_for_external_update(transaction, tenant, predecessor)
            .await?
            .run
            != command.run
        {
            return Err(StoreError::Conflict);
        }
        let submitted: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM submission_idempotency WHERE tenant_id = $1 AND attempt_id = $2)",
        )
        .bind(tenant.as_uuid())
        .bind(predecessor.as_uuid())
        .fetch_one(&mut **transaction)
        .await
        .map_err(map_sqlx_error)?;
        if !submitted {
            return Err(StoreError::Conflict);
        }
        // `SELECT .. FOR UPDATE` would require a table-wide UPDATE grant even
        // though successor links are immutable. The primary key serializes
        // concurrent insertions; a loser reads and accepts only the exact link.
        let inserted = sqlx::query(
            "INSERT INTO submission_next_attempt \
             (tenant_id, predecessor_attempt_id, next_attempt_id, next_attempt_occurred_at) \
             VALUES ($1, $2, $3, transaction_timestamp()) \
             ON CONFLICT (tenant_id, predecessor_attempt_id) DO NOTHING",
        )
        .bind(tenant.as_uuid())
        .bind(predecessor.as_uuid())
        .bind(attempt.id.as_uuid())
        .execute(&mut **transaction)
        .await
        .map_err(map_sqlx_error)?;
        if inserted.rows_affected() == 0 {
            let existing: Option<Option<Uuid>> = sqlx::query_scalar(
                "SELECT next_attempt_id FROM submission_next_attempt \
                 WHERE tenant_id = $1 AND predecessor_attempt_id = $2",
            )
            .bind(tenant.as_uuid())
            .bind(predecessor.as_uuid())
            .fetch_optional(&mut **transaction)
            .await
            .map_err(map_sqlx_error)?;
            if existing != Some(Some(attempt.id.as_uuid())) {
                return Err(StoreError::Conflict);
            }
        }
    }
    Ok(attempt)
}

async fn insert_webwork_grade_replay_state(
    transaction: &mut Transaction<'_, Postgres>,
    course: CourseId,
    attempt: &QuestionAttempt,
    presentation: PresentationBindingV1,
    mapping: WebworkReplayMappingV1,
) -> Result<(), StoreError> {
    let state = webwork_replay_state_from_issue(
        attempt.problem,
        attempt.question_version,
        attempt.seed,
        &attempt.provenance,
        presentation,
        mapping,
    )?;
    let (mapping, mapping_sha256) = encode_payload(&state.mapping)?;
    let inserted = sqlx::query(
        "INSERT INTO webwork_grade_replay_state \
         (tenant_id, attempt_id, attempt_occurred_at, course_id, problem_id, version_id, \
          source_object_id, source_sha256, seed, renderer_id, renderer_version, \
          presentation_digest, state_version, mapping, mapping_sha256) \
         SELECT $1, $2, qa.occurred_at, $3, $4, $5, $6, $7, $8::numeric, $9, $10, \
                $11, 1, $12, $13 \
           FROM question_attempt qa \
          WHERE qa.tenant_id = $1 AND qa.attempt_id = $2",
    )
    .bind(attempt.tenant.as_uuid())
    .bind(attempt.id.as_uuid())
    .bind(course.as_uuid())
    .bind(state.problem.as_uuid())
    .bind(state.version.as_uuid())
    .bind(state.source_artifact.object.as_uuid())
    .bind(&state.source_artifact.sha256)
    .bind(state.seed.to_string())
    .bind(&state.renderer.id)
    .bind(&state.renderer.version)
    .bind(state.presentation_digest.as_bytes().to_vec())
    .bind(mapping)
    .bind(mapping_sha256)
    .execute(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    if inserted.rows_affected() != 1 {
        return Err(StoreError::Conflict);
    }
    Ok(())
}

#[cfg(feature = "postgres")]
pub(super) async fn validate_postgres_assignment_position(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    command: &IssueQuestionAttemptCommand,
) -> Result<(), StoreError> {
    let position = i32::try_from(command.assignment_position)
        .map_err(|_| StoreError::InvalidRecord("assignment position is too large".to_string()))?;
    let row = sqlx::query(
        "SELECT problem_id, version_id FROM assignment_run_item \
         WHERE tenant_id = $1 AND run_id = $2 AND issued_position = $3",
    )
    .bind(tenant.as_uuid())
    .bind(command.run.as_uuid())
    .bind(position)
    .fetch_optional(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?
    .ok_or_else(|| StoreError::InvalidRecord("question position is outside the run".to_string()))?;
    let problem: Uuid = row.try_get("problem_id").map_err(map_sqlx_error)?;
    let version: Uuid = row.try_get("version_id").map_err(map_sqlx_error)?;
    if problem != command.problem.as_uuid() || version != command.question_version.as_uuid() {
        return Err(StoreError::InvalidRecord(
            "question identity does not match its run position".to_string(),
        ));
    }
    Ok(())
}

#[cfg(feature = "postgres")]
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

#[cfg(feature = "postgres")]
pub(in crate::postgres) fn add_seconds(
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
