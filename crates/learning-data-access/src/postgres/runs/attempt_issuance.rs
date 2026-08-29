use objects::Sha256Digest;
use question_model::{
    AttemptStatus, AttemptTimerRecord, CourseId, PresentationBindingV1, QuestionAttempt,
    QuestionEnvelope, TenantId,
};
use sqlx::{Postgres, Row, Transaction};

use crate::{
    IssueQuestionAttemptCommand, IssuedQuestionSnapshotV1, JobId, JobPayload,
    PrefetchedQuestionDescriptorV1, PresentationCapability, ReceiptNextAttempt,
    ReceiptPresentationSnapshot, StoreError, TenantContext, WebworkReplayMappingV1,
    issued_attempt_capability_from_issue, validate_issued_flat_grading,
    validate_issued_presentation, validate_issued_qti_grading, validate_issued_webwork_grading,
    validate_issued_webwork_replay, webwork_replay_state_from_issue,
};

use super::super::assignment_timing;
use super::super::connection::map_sqlx_error;
use super::super::row_decode::{
    decode_current_attempt_row, decode_payload_row, decode_presentation_binding_row, encode_payload,
};
use super::super::transaction_context::database_timestamp;
use super::authored_timing::{issued_timer, validate_postgres_assignment_position};
use super::student_transition::{
    lock_prepared_predecessor_for_student_run, record_submission_successor,
};

#[cfg(feature = "postgres")]
pub(super) async fn issue_or_resume_question_attempt(
    transaction: &mut Transaction<'_, Postgres>,
    context: TenantContext,
    command: IssueQuestionAttemptCommand,
) -> Result<QuestionAttempt, StoreError> {
    let tenant = context.tenant_id();
    validate_issue_command_shape(tenant, &command)?;
    // The 1817 wrapper is deliberately the first protected database
    // operation. It authorizes and locks the exact Student-owned route before
    // an opaque run, predecessor, enrollment, policy, or assignment is read.
    let prepared = match super::super::student_run_preparation::prepare_student_run_work(
        transaction,
        tenant,
        command.binding,
        command.actor,
        command.run,
    )
    .await
    {
        Ok(prepared) => prepared,
        Err(StoreError::Forbidden | StoreError::NotFound) => return Err(StoreError::NotFound),
        Err(error) => return Err(error),
    };
    let run = prepared.run().clone();
    let assignment = prepared.assignment().clone();
    let grant = prepared.grant().clone();
    let prepared_revision = prepared.assignment_revision();
    if prepared.enrollment().id != run.enrollment {
        return Err(StoreError::InvalidRecord(
            "prepared enrollment does not own its run".to_string(),
        ));
    }
    if run.completed_at.is_some() || run.score.is_some() {
        return Err(StoreError::InvalidRecord(
            "a completed run cannot issue another question".to_string(),
        ));
    }
    if let Some(predecessor) = command.predecessor_submission {
        lock_prepared_predecessor_for_student_run(transaction, tenant, command.run, predecessor)
            .await?;
    }
    let (effective_decision, assignment_revision) =
        super::super::course_policy::resolve_granted_effective_policy_read_only(
            transaction,
            grant,
            domain::effective_assignment_policy::AuthorizationGate::Authorized,
            run.run_number.saturating_sub(1),
        )
        .await?;
    if assignment_revision.value() != prepared_revision {
        return Err(StoreError::InvalidRecord(
            "effective policy revision disagrees with learner-work witness".to_string(),
        ));
    }
    let domain::effective_assignment_policy::EffectivePolicyDecision::Allowed {
        policy,
        start: domain::effective_assignment_policy::StartVerdict::MayStart { .. },
    } = effective_decision
    else {
        return Err(StoreError::NotFound);
    };
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
            || prefetched.issued_question_snapshot != command.issued_question_snapshot
            || prefetched.presentation_capability != command.presentation_capability
            || Some(prefetched.presentation) != command.presentation
            || Some(&prefetched.presentation_snapshot) != command.presentation_snapshot.as_ref()
            || Some(&prefetched.grading_envelope) != command.grading_envelope.as_ref()
            || prefetched.flat_grading_capability != command.flat_grading_capability
            || prefetched.webwork_grading_capability != command.webwork_grading_capability
            || prefetched.qti_grading_capability != command.qti_grading_capability)
    {
        return Err(StoreError::Conflict);
    }

    let unresolved = sqlx::query(
        "SELECT qa.payload, qa.payload_sha256, \
                qa.presentation_descriptor_version, qa.presentation_nonce, qa.presentation_digest, \
                qa.presentation_capability, qa.presentation_payload, qa.presentation_payload_sha256, \
                qa.grading_envelope_payload, qa.grading_envelope_payload_sha256, \
                qa.issued_question_snapshot_payload, \
                qa.issued_question_snapshot_payload_sha256, \
                qa.attempt_status AS current_attempt_status, \
                floor(extract(epoch FROM qa.submitted_at) * 1000)::bigint AS current_submitted_at, \
                floor(extract(epoch FROM timing.effective_deadline) * 1000)::bigint \
                    AS current_deadline_at \
         FROM question_attempt AS qa \
         LEFT JOIN submission_idempotency AS si \
           ON si.tenant_id = qa.tenant_id AND si.attempt_id = qa.attempt_id \
         LEFT JOIN attempt_effective_policy_current AS current_effect \
           ON current_effect.tenant_id = qa.tenant_id AND current_effect.attempt_id = qa.attempt_id \
         LEFT JOIN attempt_effective_policy_receipt AS timing \
           ON timing.tenant_id = current_effect.tenant_id AND timing.attempt_id = current_effect.attempt_id \
          AND timing.receipt_generation = current_effect.receipt_generation \
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
            let active_capability = presentation_capability_from_row(&row)?;
            let active_snapshot = decode_attempt_presentation_snapshot(&row, active_capability)?;
            let active_grading_envelope = decode_attempt_grading_envelope(&row, active_capability)?;
            let active_issued_question_snapshot = decode_issued_question_snapshot(&row)?;
            if active_capability != command.presentation_capability
                || active_issued_question_snapshot != command.issued_question_snapshot
                || active_snapshot.as_ref() != command.presentation_snapshot.as_ref()
                || active_grading_envelope.as_ref() != command.grading_envelope.as_ref()
                || !super::private_execution::attempt_private_execution_matches(
                    transaction,
                    tenant,
                    active.id,
                    &command,
                )
                .await?
            {
                return Err(StoreError::Conflict);
            }
            if decode_presentation_binding_row(&row)? != command.presentation {
                return Err(StoreError::Conflict);
            }
            if let Some(predecessor) = command.predecessor_submission {
                // Converging healers must attach the already-issued active
                // attempt to the durable predecessor receipt before return.
                // Select the persisted timestamp rather than the public
                // millisecond timer value so the partitioned FK is exact.
                let next = ReceiptNextAttempt::from_attempt(&active);
                let (next_payload, next_payload_sha256) = encode_payload(&next)?;
                let inserted = sqlx::query(
                    "INSERT INTO submission_next_attempt \
                     (tenant_id, predecessor_attempt_id, next_attempt_id, next_attempt_occurred_at, \
                      next_payload, next_payload_sha256) \
                     SELECT $1, $2, $3, active.occurred_at, $4, $5 \
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
                .bind(next_payload)
                .bind(next_payload_sha256)
                .execute(&mut **transaction)
                .await
                .map_err(map_sqlx_error)?;
                if inserted.rows_affected() == 0 {
                    super::require_exact_submission_successor(
                        transaction,
                        tenant,
                        predecessor,
                        Some(&next),
                    )
                    .await?;
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
            || prefetched.issued_question_snapshot != command.issued_question_snapshot
            || prefetched.presentation_capability != command.presentation_capability
            || Some(prefetched.presentation) != command.presentation
            || Some(&prefetched.presentation_snapshot) != command.presentation_snapshot.as_ref()
            || Some(&prefetched.grading_envelope) != command.grading_envelope.as_ref()
            || prefetched.flat_grading_capability != command.flat_grading_capability
            || prefetched.webwork_grading_capability != command.webwork_grading_capability
            || prefetched.qti_grading_capability != command.qti_grading_capability
        {
            return Err(StoreError::Conflict);
        }
        let row = sqlx::query(
            "SELECT payload, payload_sha256, presentation_descriptor_version, \
                    presentation_nonce, presentation_digest FROM question_prefetch \
             WHERE tenant_id = $1 AND run_id = $2 AND predecessor_attempt_id = $3 AND assignment_position = $4",
        )
        .bind(tenant.as_uuid())
        .bind(command.run.as_uuid())
        .bind(prefetched.predecessor.as_uuid())
        .bind(assignment_position)
        .fetch_optional(&mut **transaction)
        .await
        .map_err(map_sqlx_error)?
        .ok_or(StoreError::Conflict)?;
        let stored: PrefetchedQuestionDescriptorV1 = decode_payload_row(&row)?;
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
        "SELECT CASE WHEN si.request_contract_version = 2 THEN qa.payload ELSE si.payload END AS payload, \
                CASE WHEN si.request_contract_version = 2 THEN qa.payload_sha256 ELSE si.payload_sha256 END AS payload_sha256 FROM question_attempt AS qa \
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
    let flat_grading = command.flat_grading.as_ref();
    let flat_grading_capability = prefetched
        .map(|value| value.flat_grading_capability)
        .unwrap_or(command.flat_grading_capability);
    let webwork_replay = command.webwork_replay.clone();
    let webwork_grading = command.webwork_grading.as_ref();
    let webwork_grading_capability = prefetched
        .map(|value| value.webwork_grading_capability)
        .unwrap_or(command.webwork_grading_capability);
    let qti_grading = command.qti_grading.as_ref();
    let qti_grading_capability = prefetched
        .map(|value| value.qti_grading_capability)
        .unwrap_or(command.qti_grading_capability);
    let issued_question_snapshot = prefetched
        .map(|value| &value.issued_question_snapshot)
        .unwrap_or(&command.issued_question_snapshot);
    if parameter_hash.trim().is_empty() || provenance.rendered_question_sha256.trim().is_empty() {
        return Err(StoreError::InvalidRecord(
            "issued attempt hashes must not be empty".to_string(),
        ));
    }
    issued_question_snapshot.validate_for_attempt(command.problem, command.question_version)?;
    issued_question_snapshot.validate_for_issuance_context(
        flat_grading_capability,
        webwork_grading_capability,
        qti_grading_capability,
        presentation_snapshot,
    )?;
    validate_issued_flat_grading(
        issued_question_snapshot.question(),
        presentation_capability,
        flat_grading_capability,
        flat_grading,
    )?;
    validate_issued_webwork_grading(
        issued_question_snapshot.question(),
        webwork_grading_capability,
        webwork_grading,
    )?;
    validate_issued_qti_grading(
        issued_question_snapshot.question(),
        qti_grading_capability,
        qti_grading,
    )?;
    validate_issued_webwork_replay(webwork_grading_capability, webwork_replay.as_ref())?;
    let issued_capability = issued_attempt_capability_from_issue(
        presentation_capability,
        flat_grading_capability,
        webwork_grading_capability,
        qti_grading_capability,
    )?;
    let issued_at = database_timestamp(transaction).await?;
    let authored_timer = issued_timer(
        issued_at,
        run.started_at,
        issued_question_snapshot.question().timing_policy,
    )?;
    let authored_grace_seconds = assignment_timing::timing_policy_grace_seconds(
        issued_question_snapshot.question().timing_policy,
    );
    let assignment_timing::ResolvedPostgresAttemptTiming {
        effective_deadline,
        effective_grace_seconds,
        auto_submit_at,
    } = assignment_timing::resolved_postgres_attempt_timing(
        &policy,
        run.started_at,
        authored_timer.deadline,
        authored_grace_seconds,
    )?;
    if effective_deadline.is_some_and(|deadline| deadline < issued_at)
        || auto_submit_at.is_some_and(|deadline| deadline <= issued_at)
    {
        return Err(StoreError::TimedOut);
    }
    let authored_deadline = authored_timer.deadline;
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
        issued_capability,
    };
    issued_question_snapshot.validate_native_provenance(&attempt.provenance.asset_objects)?;
    // Validate the issue tuple before any attempt, receipt, or run mutation.
    let presentation_snapshot = validate_issued_presentation(
        presentation_capability,
        &attempt,
        presentation,
        presentation_snapshot,
        grading_envelope,
    )?;
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
    let (issued_question_snapshot_payload, issued_question_snapshot_payload_sha256) =
        issued_question_snapshot.canonical_payload()?;
    let (presentation_payload, presentation_payload_sha256) = presentation_snapshot
        .as_ref()
        .map(encode_payload)
        .transpose()?
        .map_or((None, None), |(payload, checksum)| {
            (Some(payload), Some(checksum))
        });
    let (grading_envelope_payload, grading_envelope_payload_sha256) = grading_envelope
        .map(encode_payload)
        .transpose()?
        .map_or((None, None), |(payload, checksum)| {
            (Some(payload), Some(checksum))
        });
    sqlx::query(
        "INSERT INTO question_attempt \
         (tenant_id, attempt_id, run_id, problem_id, version_id, assignment_position, \
          occurred_at, payload, payload_sha256, presentation_descriptor_version, \
          presentation_nonce, presentation_digest, presentation_capability, presentation_payload, \
          presentation_payload_sha256, grading_envelope_payload, grading_envelope_payload_sha256, \
          issued_question_snapshot_payload, issued_question_snapshot_payload_sha256, \
          authored_timing_deadline, authored_timing_grace_seconds) \
          VALUES ($1, $2, $3, $4, $5, $6, transaction_timestamp(), $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, TIMESTAMPTZ 'epoch' + $19::bigint * INTERVAL '1 millisecond', $20)",
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
    .bind(presentation_capability_name(presentation_capability))
    .bind(presentation_payload)
    .bind(presentation_payload_sha256)
    .bind(grading_envelope_payload)
    .bind(grading_envelope_payload_sha256)
    .bind(issued_question_snapshot_payload)
    .bind(issued_question_snapshot_payload_sha256)
    .bind(
        authored_deadline
            .map(|deadline| deadline.as_unix_millis()),
    )
    .bind(i64::from(authored_grace_seconds))
    .execute(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    let private_written = super::private_execution::attempt_private_execution_matches(
        transaction,
        tenant,
        attempt.id,
        &command,
    )
    .await?;
    if !private_written {
        return Err(StoreError::Conflict);
    }
    if webwork_grading_capability.requires_contract() {
        let mapping = webwork_replay.ok_or_else(|| {
            StoreError::InvalidRecord("WeBWorK replay mapping is missing".to_string())
        })?;
        let presentation = presentation.ok_or_else(|| {
            StoreError::InvalidRecord("WeBWorK replay lacks a presentation binding".to_string())
        })?;
        insert_webwork_grade_replay_state(
            transaction,
            assignment.course_id,
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
    let receipt_generation = 1_i64;
    super::super::effective_policy_receipts::append_sealed_effective_policy_receipt(
        transaction,
        super::super::effective_policy_receipts::EffectivePolicyReceiptWrite {
            tenant,
            course: assignment.course_id,
            assignment: assignment.id,
            attempt: attempt.id,
            generation: receipt_generation,
            policy: &policy,
            effective_deadline,
            effective_grace_seconds,
            auto_submit_at,
            revision: assignment_revision,
        },
    )
    .await?;
    sqlx::query("INSERT INTO attempt_effective_policy_current (tenant_id,attempt_id,attempt_occurred_at,assignment_id,course_id,receipt_generation,timing_generation,job_id) SELECT $1,$2,occurred_at,$3,$4,$5,$6,$7 FROM question_attempt WHERE tenant_id=$1 AND attempt_id=$2")
        .bind(tenant.as_uuid()).bind(attempt.id.as_uuid()).bind(assignment.id.as_uuid()).bind(assignment.course_id.as_uuid()).bind(receipt_generation).bind(i64::try_from(timing_generation).map_err(|_| StoreError::Conflict)?).bind(timing_job.map(JobId::as_uuid)).execute(&mut **transaction).await.map_err(map_sqlx_error)?;
    if let Some(prefetched) = prefetched {
        let promoted: bool = sqlx::query_scalar(
            "SELECT public.ple_promote_prefetch_private_execution($1,$2,$3,$4,$5)",
        )
        .bind(tenant.as_uuid())
        .bind(attempt.id.as_uuid())
        .bind(command.run.as_uuid())
        .bind(prefetched.predecessor.as_uuid())
        .bind(assignment_position)
        .fetch_one(&mut **transaction)
        .await
        .map_err(map_sqlx_error)?;
        if !promoted {
            return Err(StoreError::Conflict);
        }
    }
    if let Some(predecessor) = command.predecessor_submission {
        let next = ReceiptNextAttempt::from_attempt(&attempt);
        record_submission_successor(transaction, tenant, predecessor, &next).await?;
    }
    Ok(attempt)
}

fn validate_issue_command_shape(
    tenant: TenantId,
    command: &IssueQuestionAttemptCommand,
) -> Result<(), StoreError> {
    i32::try_from(command.assignment_position)
        .map_err(|_| StoreError::InvalidRecord("assignment position is too large".to_string()))?;
    issued_attempt_capability_from_issue(
        command.presentation_capability,
        command.flat_grading_capability,
        command.webwork_grading_capability,
        command.qti_grading_capability,
    )?;
    command
        .issued_question_snapshot
        .validate_for_attempt(command.problem, command.question_version)?;
    command
        .issued_question_snapshot
        .validate_for_issuance_context(
            command.flat_grading_capability,
            command.webwork_grading_capability,
            command.qti_grading_capability,
            command.presentation_snapshot.as_ref(),
        )?;
    crate::validate_issued_qti_grading(
        command.issued_question_snapshot.question(),
        command.qti_grading_capability,
        command.qti_grading.as_ref(),
    )?;
    validate_issued_webwork_replay(
        command.webwork_grading_capability,
        command.webwork_replay.as_ref(),
    )?;
    if let Some(prefetched) = command.prefetched.as_ref()
        && (prefetched.tenant != tenant
            || prefetched.run != command.run
            || command.predecessor_submission != Some(prefetched.predecessor)
            || prefetched.assignment_position != command.assignment_position
            || prefetched.problem != command.problem
            || prefetched.question_version != command.question_version
            || prefetched.issued_question_snapshot != command.issued_question_snapshot
            || prefetched.presentation_capability != command.presentation_capability
            || Some(prefetched.presentation) != command.presentation
            || Some(&prefetched.presentation_snapshot) != command.presentation_snapshot.as_ref()
            || Some(&prefetched.grading_envelope) != command.grading_envelope.as_ref()
            || prefetched.flat_grading_capability != command.flat_grading_capability
            || prefetched.webwork_grading_capability != command.webwork_grading_capability
            || prefetched.qti_grading_capability != command.qti_grading_capability)
    {
        return Err(StoreError::Conflict);
    }
    Ok(())
}

pub(in crate::postgres) fn presentation_capability_name(
    value: PresentationCapability,
) -> &'static str {
    match value {
        PresentationCapability::EnvelopeV1 => "envelope_v1",
        PresentationCapability::NotApplicable => "not_applicable",
    }
}

pub(in crate::postgres) fn decode_issued_question_snapshot(
    row: &sqlx::postgres::PgRow,
) -> Result<IssuedQuestionSnapshotV1, StoreError> {
    let payload: serde_json::Value = row
        .try_get("issued_question_snapshot_payload")
        .map_err(map_sqlx_error)?;
    let checksum: String = row
        .try_get("issued_question_snapshot_payload_sha256")
        .map_err(map_sqlx_error)?;
    IssuedQuestionSnapshotV1::decode_checked(payload, &checksum)
}

pub(in crate::postgres) fn presentation_capability_from_row(
    row: &sqlx::postgres::PgRow,
) -> Result<PresentationCapability, StoreError> {
    match row
        .try_get::<String, _>("presentation_capability")
        .map_err(map_sqlx_error)?
        .as_str()
    {
        "envelope_v1" => Ok(PresentationCapability::EnvelopeV1),
        "not_applicable" => Ok(PresentationCapability::NotApplicable),
        _ => Err(StoreError::Unavailable(
            "stored attempt presentation capability is invalid".to_string(),
        )),
    }
}

pub(in crate::postgres) fn decode_attempt_presentation_snapshot(
    row: &sqlx::postgres::PgRow,
    capability: PresentationCapability,
) -> Result<Option<ReceiptPresentationSnapshot>, StoreError> {
    let payload: Option<serde_json::Value> = row
        .try_get("presentation_payload")
        .map_err(map_sqlx_error)?;
    let checksum: Option<String> = row
        .try_get("presentation_payload_sha256")
        .map_err(map_sqlx_error)?;
    match (capability, payload, checksum) {
        (PresentationCapability::NotApplicable, None, None) => Ok(None),
        (PresentationCapability::NotApplicable, _, _) => Err(StoreError::Unavailable(
            "non-presentation attempt carries a snapshot".to_string(),
        )),
        (PresentationCapability::EnvelopeV1, Some(payload), Some(checksum)) => {
            let bytes = serde_json::to_vec(&payload).map_err(|error| {
                StoreError::Unavailable(format!(
                    "stored issued presentation encode failed: {error}"
                ))
            })?;
            if Sha256Digest::compute(&bytes).to_string() != checksum {
                return Err(StoreError::Unavailable(
                    "stored issued presentation checksum mismatch".to_string(),
                ));
            }
            serde_json::from_value(payload).map(Some).map_err(|error| {
                StoreError::Unavailable(format!(
                    "stored issued presentation decode failed: {error}"
                ))
            })
        }
        (PresentationCapability::EnvelopeV1, _, _) => Err(StoreError::Unavailable(
            "presentation-bearing attempt lacks its immutable snapshot".to_string(),
        )),
    }
}

/// Reads the exact answer-free envelope retained only for trusted first-submit
/// validation and private grading. It is paired with the explicit capability,
/// so a missing value is unavailable authority rather than a request to
/// regenerate the attempt from current backend state.
pub(in crate::postgres) fn decode_attempt_grading_envelope(
    row: &sqlx::postgres::PgRow,
    capability: PresentationCapability,
) -> Result<Option<QuestionEnvelope>, StoreError> {
    let payload: Option<serde_json::Value> = row
        .try_get("grading_envelope_payload")
        .map_err(map_sqlx_error)?;
    let checksum: Option<String> = row
        .try_get("grading_envelope_payload_sha256")
        .map_err(map_sqlx_error)?;
    match (capability, payload, checksum) {
        (PresentationCapability::NotApplicable, None, None) => Ok(None),
        (PresentationCapability::NotApplicable, _, _) => Err(StoreError::Unavailable(
            "non-presentation attempt carries a private grading envelope".to_string(),
        )),
        (PresentationCapability::EnvelopeV1, Some(payload), Some(checksum)) => {
            let bytes = serde_json::to_vec(&payload).map_err(|error| {
                StoreError::Unavailable(format!(
                    "stored private grading envelope encode failed: {error}"
                ))
            })?;
            if Sha256Digest::compute(&bytes).to_string() != checksum {
                return Err(StoreError::Unavailable(
                    "stored private grading envelope checksum mismatch".to_string(),
                ));
            }
            serde_json::from_value(payload).map(Some).map_err(|error| {
                StoreError::Unavailable(format!(
                    "stored private grading envelope decode failed: {error}"
                ))
            })
        }
        (PresentationCapability::EnvelopeV1, _, _) => Err(StoreError::Unavailable(
            "presentation-bearing attempt lacks its private grading envelope".to_string(),
        )),
    }
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
