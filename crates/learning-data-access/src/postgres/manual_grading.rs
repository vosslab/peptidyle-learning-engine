//! PostgreSQL implementation of the response-bearing, current-only manual
//! evaluation contract. Immutable submission evidence remains separate from
//! the mutable evaluation row and minimal action receipt.

use async_trait::async_trait;
use bigdecimal::BigDecimal;
use question_model::{AttemptResult, AttemptStatus, FeedbackContent};
use sqlx::Row;

use super::*;
use crate::manual_grading::request_digest;
use crate::{
    EvaluationRevision, ManualCredit, ManualEvaluationRecord, ManualEvaluationStatus,
    ManualGradeActionId, ManualGradeReceipt, ManualGradingStore, SetManualGradeCommand,
    SubmitPendingManualQuestionAttemptCommand,
};

#[async_trait]
impl ManualGradingStore for PostgresStore {
    async fn get_manual_evaluation_with_response_for_edit(
        &self,
        context: TenantContext,
        actor: UserId,
        attempt: QuestionAttemptId,
    ) -> Result<Option<(ManualEvaluationRecord, StudentResponse)>, StoreError> {
        let tenant = context.tenant_id();
        let mut transaction = self.begin_tenant(context).await?;
        let attempt_record =
            load_manual_attempt_for_update(&mut transaction, tenant, attempt).await?;
        let run = load_run_for_update(&mut transaction, tenant, attempt_record.run).await?;
        let enrollment =
            load_enrollment_for_update(&mut transaction, tenant, run.enrollment).await?;
        let assignment = load_assignment(&mut transaction, tenant, enrollment.assignment).await?;
        let accessible: bool =
            sqlx::query_scalar("SELECT public.ple_course_records_accessible($1, $2)")
                .bind(tenant.as_uuid())
                .bind(assignment.course_id.as_uuid())
                .fetch_one(&mut *transaction)
                .await
                .map_err(map_sqlx_error)?;
        if !accessible
            || !postgres_is_course_instructor(&mut transaction, tenant, assignment.course_id, actor)
                .await?
        {
            return Err(StoreError::NotFound);
        }
        let record = load_manual_evaluation(&mut transaction, tenant, attempt).await?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(record.zip(attempt_record.response))
    }
    async fn submit_pending_manual_question_attempt(
        &self,
        context: TenantContext,
        command: SubmitPendingManualQuestionAttemptCommand,
    ) -> Result<SubmissionRecord, StoreError> {
        let mut transaction = self.begin_tenant(context).await?;
        let record =
            submit_pending_manual_question_attempt(&mut transaction, context, command).await?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(record)
    }

    async fn get_manual_evaluation_for_edit(
        &self,
        context: TenantContext,
        actor: UserId,
        attempt: QuestionAttemptId,
    ) -> Result<Option<ManualEvaluationRecord>, StoreError> {
        let tenant = context.tenant_id();
        let mut transaction = self.begin_tenant(context).await?;
        let attempt_record =
            load_manual_attempt_for_update(&mut transaction, tenant, attempt).await?;
        let run = load_run_for_update(&mut transaction, tenant, attempt_record.run).await?;
        let enrollment =
            load_enrollment_for_update(&mut transaction, tenant, run.enrollment).await?;
        let assignment = load_assignment(&mut transaction, tenant, enrollment.assignment).await?;
        let accessible: bool =
            sqlx::query_scalar("SELECT public.ple_course_records_accessible($1, $2)")
                .bind(tenant.as_uuid())
                .bind(assignment.course_id.as_uuid())
                .fetch_one(&mut *transaction)
                .await
                .map_err(map_sqlx_error)?;
        if !accessible
            || !postgres_is_course_instructor(&mut transaction, tenant, assignment.course_id, actor)
                .await?
        {
            return Err(StoreError::NotFound);
        }
        let record = load_manual_evaluation(&mut transaction, tenant, attempt).await?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(record)
    }

    async fn set_manual_grade(
        &self,
        context: TenantContext,
        command: SetManualGradeCommand,
    ) -> Result<ManualGradeReceipt, StoreError> {
        retry_transaction(|| {
            let command = command.clone();
            async move {
                let mut transaction = self.begin_tenant(context).await?;
                let receipt = set_postgres_manual_grade(&mut transaction, context, command).await?;
                transaction.commit().await.map_err(map_sqlx_error)?;
                Ok(receipt)
            }
        })
        .await
    }
}

async fn submit_pending_manual_question_attempt(
    transaction: &mut Transaction<'_, Postgres>,
    context: TenantContext,
    command: SubmitPendingManualQuestionAttemptCommand,
) -> Result<SubmissionRecord, StoreError> {
    let tenant = context.tenant_id();
    let base = load_manual_attempt_for_update(transaction, tenant, command.attempt).await?;
    require_attempt_owner(transaction, tenant, base.id, command.actor).await?;
    if let Some(replay) = load_submission_replay(
        transaction,
        tenant,
        base.id,
        &command.response,
        &command.idempotency_key,
    )
    .await?
    {
        return Ok(replay);
    }
    if base.status != AttemptStatus::InProgress {
        return Err(StoreError::Conflict);
    }
    // Validate the issuance-time snapshot before any receipt, attempt, or run
    // mutation. The pending-grade receipt copies it without reconstruction.
    let presentation =
        super::submission::load_issued_presentation(transaction, tenant, &base).await?;
    let run = load_run_for_update(transaction, tenant, base.run).await?;
    if run.completed_at.is_some() || run.score.is_some() {
        return Err(StoreError::Conflict);
    }
    let enrollment = load_enrollment_for_update(transaction, tenant, run.enrollment).await?;
    let assignment = load_assignment_for_share(transaction, tenant, enrollment.assignment).await?;
    let previous = load_summary_for_update(transaction, tenant, enrollment.id).await?;
    let submitted_at = database_timestamp(transaction).await?;
    let mut submitted = base;
    submitted.response = Some(command.response.clone());
    submitted.status = AttemptStatus::NeedsManualGrading;
    submitted.result = None;
    submitted.timer.submitted_at = Some(submitted_at);
    let question =
        load_published_record(transaction, submitted.problem, submitted.question_version).await?;
    let feedback_disclosure =
        super::submission::load_issued_feedback_disclosure(transaction, tenant, submitted.id)
            .await?;
    let effective_grace: Option<i32> = sqlx::query_scalar(
        "SELECT effective_grace_seconds FROM attempt_timing_current \
         WHERE tenant_id = $1 AND attempt_id = $2",
    )
    .bind(tenant.as_uuid())
    .bind(submitted.id.as_uuid())
    .fetch_optional(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?
    .flatten();
    let policy = match effective_grace {
        Some(grace_seconds) if submitted.timer.deadline.is_some() => TimingPolicy::PerQuestion {
            seconds: 1,
            grace_seconds: u32::try_from(grace_seconds).map_err(|_| {
                StoreError::Unavailable("stored effective grace is invalid".to_string())
            })?,
        },
        Some(_) => TimingPolicy::Untimed,
        None => question.question.timing_policy,
    };
    let verdict = timer_verdict(&TimerEvaluation {
        policy,
        timer: submitted.timer,
        evaluated_at: submitted_at,
        pause_extension_millis: 0,
    })
    .map_err(|error| StoreError::InvalidRecord(error.to_string()))?;
    if verdict == TimerVerdict::TimedOut {
        return Err(StoreError::TimedOut);
    }
    let next = project_summary(
        &previous,
        domain::scoring::RunTransition::QuestionAttemptRecorded { at: submitted_at },
        grade_policy(&assignment),
    )?;
    let feedback = private_feedback_record(FeedbackContent::default())?;
    let (attempt_payload, attempt_checksum) = encode_payload(&submitted)?;
    let (_, response_checksum) = encode_payload(&command.response)?;
    let (response_payload, response_payload_checksum) = encode_payload(&command.response)?;
    let (marker_payload, marker_checksum) = encode_payload(&serde_json::json!({}))?;
    let feedback_columns = encode_feedback_columns(feedback.content())?;
    sqlx::query(
        "INSERT INTO attempt_feedback (tenant_id, attempt_id, hint, correct_response, rationale, content_sha256) \
         VALUES ($1, $2, $3, $4, $5, $6)",
    )
    .bind(tenant.as_uuid())
    .bind(submitted.id.as_uuid())
    .bind(feedback_columns.hint)
    .bind(feedback_columns.correct_response)
    .bind(feedback_columns.rationale)
    .bind(feedback.content_sha256().to_string())
    .execute(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    sqlx::query(
        "INSERT INTO submission_idempotency \
         (tenant_id, attempt_id, idempotency_key, request_contract_version, request_sha256, \
          submitted_at, payload, payload_sha256) \
         VALUES ($1, $2, $3, 0, $4, transaction_timestamp(), $5, $6)",
    )
    .bind(tenant.as_uuid())
    .bind(submitted.id.as_uuid())
    .bind(command.idempotency_key.as_str())
    .bind(response_checksum)
    .bind(attempt_payload.clone())
    .bind(attempt_checksum.clone())
    .execute(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    sqlx::query(
        "INSERT INTO submission \
         (tenant_id, submission_id, attempt_id, idempotency_key, occurred_at, payload, payload_sha256) \
         VALUES ($1, $2, $2, $3, transaction_timestamp(), $4, $5)",
    )
    .bind(tenant.as_uuid())
    .bind(submitted.id.as_uuid())
    .bind(command.idempotency_key.as_str())
    .bind(response_payload)
    .bind(response_payload_checksum)
    .execute(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    sqlx::query(
        "INSERT INTO submission_evaluation \
         (tenant_id, attempt_id, submission_id, credit_fraction, correct, grading_status, payload, payload_sha256, evaluation_revision) \
         VALUES ($1, $2, $2, NULL, NULL, 'needs_manual_grading', $3, $4, 1)",
    )
    .bind(tenant.as_uuid())
    .bind(submitted.id.as_uuid())
    .bind(marker_payload)
    .bind(marker_checksum)
    .execute(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    let updated = sqlx::query(
        "UPDATE question_attempt SET attempt_status = 'needs_manual_grading', submitted_at = transaction_timestamp() \
         WHERE tenant_id = $1 AND attempt_id = $2",
    )
    .bind(tenant.as_uuid())
    .bind(submitted.id.as_uuid())
    .execute(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    if updated.rows_affected() != 1 {
        return Err(StoreError::Conflict);
    }
    super::assignment_timing::cancel_postgres_attempt_timing_job(transaction, tenant, submitted.id)
        .await?;
    store_summary(transaction, &next).await?;
    let (run_payload, run_checksum) = encode_payload(&run)?;
    let (summary_payload, summary_checksum) = encode_payload(&next)?;
    let (presentation_payload, presentation_checksum) = presentation
        .as_ref()
        .map(encode_payload)
        .transpose()?
        .map_or((None, None), |(payload, checksum)| {
            (Some(payload), Some(checksum))
        });
    sqlx::query(
        "INSERT INTO submission_receipt_snapshot \
         (tenant_id, attempt_id, run_payload, run_payload_sha256, summary_payload, summary_payload_sha256, \
          presentation_payload, presentation_payload_sha256, presentation_required, feedback_disclosure) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
    )
    .bind(tenant.as_uuid())
    .bind(submitted.id.as_uuid())
    .bind(run_payload)
    .bind(run_checksum)
    .bind(summary_payload)
    .bind(summary_checksum)
    .bind(presentation_payload)
    .bind(presentation_checksum)
    .bind(presentation.is_some())
    .bind(super::submission::feedback_disclosure_name(feedback_disclosure))
    .execute(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    Ok(SubmissionRecord {
        attempt: submitted,
        run,
        summary: next,
        feedback,
        presentation,
        feedback_disclosure,
    })
}

async fn set_postgres_manual_grade(
    transaction: &mut Transaction<'_, Postgres>,
    context: TenantContext,
    command: SetManualGradeCommand,
) -> Result<ManualGradeReceipt, StoreError> {
    let tenant = context.tenant_id();
    let assignment_id: Uuid = sqlx::query_scalar(
        "SELECT e.assignment_id FROM question_attempt qa \
         JOIN assignment_run ar ON ar.tenant_id = qa.tenant_id AND ar.run_id = qa.run_id \
         JOIN enrollment e ON e.tenant_id = ar.tenant_id AND e.enrollment_id = ar.enrollment_id \
         WHERE qa.tenant_id = $1 AND qa.attempt_id = $2 ORDER BY qa.occurred_at LIMIT 1",
    )
    .bind(tenant.as_uuid())
    .bind(command.attempt.as_uuid())
    .fetch_optional(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?
    .ok_or(StoreError::NotFound)?;
    let assignment_id = AssignmentId::from_uuid(assignment_id);
    super::assignment_timing::lock_postgres_assignment_policy(transaction, tenant, assignment_id)
        .await?;
    let attempt = load_manual_attempt_for_update(transaction, tenant, command.attempt).await?;
    let mut run = load_run_for_update(transaction, tenant, attempt.run).await?;
    let enrollment = load_enrollment_for_update(transaction, tenant, run.enrollment).await?;
    let assignment = load_assignment(transaction, tenant, enrollment.assignment).await?;
    let accessible: bool =
        sqlx::query_scalar("SELECT public.ple_course_records_accessible($1, $2)")
            .bind(tenant.as_uuid())
            .bind(assignment.course_id.as_uuid())
            .fetch_one(&mut **transaction)
            .await
            .map_err(map_sqlx_error)?;
    if !accessible
        || assignment.id != assignment_id
        || !postgres_is_course_instructor(transaction, tenant, assignment.course_id, command.actor)
            .await?
    {
        return Err(StoreError::NotFound);
    }
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1::uuid::text, 0))")
        .bind(command.action.as_uuid())
        .execute(&mut **transaction)
        .await
        .map_err(map_sqlx_error)?;
    let digest = request_digest(&command);
    if let Some(row) = sqlx::query(
        "SELECT actor_id, attempt_id, request_sha256, expected_evaluation_revision, \
                resulting_evaluation_revision, scoring_generation, \
                floor(extract(epoch FROM occurred_at) * 1000)::bigint AS occurred_at_millis \
         FROM manual_grade_receipt WHERE tenant_id = $1 AND manual_grade_action_id = $2",
    )
    .bind(tenant.as_uuid())
    .bind(command.action.as_uuid())
    .fetch_optional(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?
    {
        let same = row.try_get::<Uuid, _>("actor_id").map_err(map_sqlx_error)?
            == command.actor.as_uuid()
            && row
                .try_get::<Uuid, _>("attempt_id")
                .map_err(map_sqlx_error)?
                == command.attempt.as_uuid()
            && row
                .try_get::<String, _>("request_sha256")
                .map_err(map_sqlx_error)?
                == digest.to_string()
            && row
                .try_get::<i64, _>("expected_evaluation_revision")
                .map_err(map_sqlx_error)?
                == i64::try_from(command.expected_revision.as_u64())
                    .map_err(|_| StoreError::Conflict)?;
        if !same {
            return Err(StoreError::Conflict);
        }
        return manual_receipt_from_row(&row, command.action, command.attempt);
    }
    if attempt.response.is_none()
        || !matches!(
            attempt.status,
            AttemptStatus::NeedsManualGrading | AttemptStatus::Submitted
        )
    {
        return Err(StoreError::Conflict);
    }
    let evaluation = load_manual_evaluation_for_update(transaction, tenant, command.attempt)
        .await?
        .ok_or(StoreError::Conflict)?;
    if evaluation.revision != command.expected_revision {
        return Err(StoreError::Conflict);
    }
    let resulting_revision = evaluation.revision.next()?;
    let credit = command.credit.as_decimal();
    let correct = credit == &BigDecimal::from(1);
    let result = AttemptResult {
        correct,
        points_earned: command.credit.try_as_f64()?,
        points_possible: 1.0,
    };
    crate::validate_attempt_result(result)?;
    let mut graded = attempt.clone();
    graded.status = AttemptStatus::Submitted;
    graded.result = Some(result);
    let (result_payload, result_checksum) = encode_payload(&result)?;
    let updated = sqlx::query(
        "UPDATE submission_evaluation SET credit_fraction = $3, correct = $4, grading_status = 'graded', \
             payload = $5, payload_sha256 = $6, evaluation_revision = $7, evaluated_at = transaction_timestamp() \
         WHERE tenant_id = $1 AND attempt_id = $2 AND evaluation_revision = $8",
    )
    .bind(tenant.as_uuid())
    .bind(command.attempt.as_uuid())
    .bind(credit)
    .bind(correct)
    .bind(result_payload)
    .bind(result_checksum)
    .bind(i64::try_from(resulting_revision.as_u64()).map_err(|_| StoreError::Conflict)?)
    .bind(i64::try_from(command.expected_revision.as_u64()).map_err(|_| StoreError::Conflict)?)
    .execute(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    if updated.rows_affected() != 1 {
        return Err(StoreError::Conflict);
    }
    let updated_attempt = sqlx::query(
        "UPDATE question_attempt SET attempt_status = 'submitted' WHERE tenant_id = $1 AND attempt_id = $2",
    )
    .bind(tenant.as_uuid())
    .bind(command.attempt.as_uuid())
    .execute(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    if updated_attempt.rows_affected() != 1 {
        return Err(StoreError::Conflict);
    }
    let rows = sqlx::query(
        "SELECT COALESCE(si.payload, qa.payload) AS payload, COALESCE(si.payload_sha256, qa.payload_sha256) AS payload_sha256, \
                evaluation.payload AS evaluation_payload, evaluation.payload_sha256 AS evaluation_payload_sha256, \
                evaluation.grading_status AS evaluation_grading_status, qa.attempt_status AS current_attempt_status, \
                floor(extract(epoch FROM qa.submitted_at) * 1000)::bigint AS current_submitted_at \
         FROM question_attempt qa LEFT JOIN submission_idempotency si ON si.tenant_id = qa.tenant_id AND si.attempt_id = qa.attempt_id \
         LEFT JOIN submission_evaluation evaluation ON evaluation.tenant_id = qa.tenant_id AND evaluation.attempt_id = qa.attempt_id \
         WHERE qa.tenant_id = $1 AND qa.run_id = $2",
    )
    .bind(tenant.as_uuid())
    .bind(run.id.as_uuid())
    .fetch_all(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    let attempts = rows
        .iter()
        .map(decode_current_attempt_with_evaluation_row)
        .collect::<Result<Vec<_>, _>>()?;
    let run_items = load_assignment_run_items(transaction, tenant, run.id).await?;
    let questions = current_run_questions(&assignment, &run_items, &attempts, &graded)?;
    if let Some(score) = completed_run_score(&questions, assignment.policies.completion)? {
        if run.completed_at.is_none() {
            run.completed_at = Some(database_timestamp(transaction).await?);
        }
        run.score = Some(score);
        let (payload, checksum) = encode_payload(&run)?;
        sqlx::query(
            "UPDATE assignment_run \
             SET completed_at = COALESCE(completed_at, transaction_timestamp()), \
                 payload = $3, payload_sha256 = $4 \
             WHERE tenant_id = $1 AND run_id = $2",
        )
        .bind(tenant.as_uuid())
        .bind(run.id.as_uuid())
        .bind(payload)
        .bind(checksum)
        .execute(&mut **transaction)
        .await
        .map_err(map_sqlx_error)?;
    }
    let generation_row = sqlx::query(
        "UPDATE assignment SET scoring_generation = scoring_generation + 1, scoring_status = 'recalculating', \
             updated_at = transaction_timestamp() WHERE tenant_id = $1 AND assignment_id = $2 RETURNING scoring_generation",
    )
    .bind(tenant.as_uuid()).bind(assignment.id.as_uuid()).fetch_optional(&mut **transaction).await.map_err(map_sqlx_error)?
        .ok_or(StoreError::NotFound)?;
    let generation = decode_scoring_generation(&generation_row)?;
    let job = JobId::generate()?;
    let payload = serde_json::to_value(JobPayload::RecalculateAssignment {
        assignment: assignment.id,
        generation,
    })
    .map_err(|error| StoreError::InvalidRecord(error.to_string()))?;
    sqlx::query("INSERT INTO worker_job (job_id, tenant_id, payload, state, max_attempts) VALUES ($1, $2, $3, 'ready', 10)")
        .bind(job.as_uuid()).bind(tenant.as_uuid()).bind(payload).execute(&mut **transaction).await.map_err(map_sqlx_error)?;
    let occurred_at = database_timestamp(transaction).await?;
    sqlx::query(
        "INSERT INTO manual_grade_receipt \
         (tenant_id, manual_grade_action_id, attempt_id, actor_id, request_sha256, expected_evaluation_revision, \
          resulting_evaluation_revision, scoring_generation, occurred_at, course_id) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, transaction_timestamp(), $9)",
    )
    .bind(tenant.as_uuid()).bind(command.action.as_uuid()).bind(command.attempt.as_uuid()).bind(command.actor.as_uuid())
    .bind(digest.to_string()).bind(i64::try_from(command.expected_revision.as_u64()).map_err(|_| StoreError::Conflict)?)
    .bind(i64::try_from(resulting_revision.as_u64()).map_err(|_| StoreError::Conflict)?)
    .bind(i64::try_from(generation.value()).map_err(|_| StoreError::Conflict)?).bind(assignment.course_id.as_uuid())
    .execute(&mut **transaction).await.map_err(map_sqlx_error)?;
    let (audit_payload, audit_checksum) =
        encode_payload(&serde_json::json!({"kind": "manual_grade"}))?;
    sqlx::query(
        "INSERT INTO audit_event (tenant_id, audit_event_id, occurred_at, actor_id, course_id, action, target_kind, target_id, payload, payload_sha256) \
         VALUES ($1, $2, transaction_timestamp(), $3, $4, 'manual_grade_set', 'question_attempt', $5, $6, $7)",
    )
    .bind(tenant.as_uuid()).bind(command.action.as_uuid()).bind(command.actor.as_uuid()).bind(assignment.course_id.as_uuid())
    .bind(command.attempt.as_uuid()).bind(audit_payload).bind(audit_checksum)
    .execute(&mut **transaction).await.map_err(map_sqlx_error)?;
    Ok(ManualGradeReceipt {
        action: command.action,
        attempt: command.attempt,
        resulting_revision,
        scoring_generation: generation,
        occurred_at,
    })
}

async fn load_manual_evaluation(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    attempt: QuestionAttemptId,
) -> Result<Option<ManualEvaluationRecord>, StoreError> {
    manual_evaluation_row(sqlx::query("SELECT grading_status, credit_fraction, evaluation_revision, floor(extract(epoch FROM evaluated_at) * 1000)::bigint AS evaluated_at_millis FROM submission_evaluation WHERE tenant_id = $1 AND attempt_id = $2 AND (grading_status = 'needs_manual_grading' OR EXISTS (SELECT 1 FROM manual_grade_receipt receipt WHERE receipt.tenant_id = submission_evaluation.tenant_id AND receipt.attempt_id = submission_evaluation.attempt_id))")
        .bind(tenant.as_uuid()).bind(attempt.as_uuid()).fetch_optional(&mut **transaction).await.map_err(map_sqlx_error)?, tenant, attempt)
}

async fn load_manual_evaluation_for_update(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    attempt: QuestionAttemptId,
) -> Result<Option<ManualEvaluationRecord>, StoreError> {
    manual_evaluation_row(sqlx::query("SELECT grading_status, credit_fraction, evaluation_revision, floor(extract(epoch FROM evaluated_at) * 1000)::bigint AS evaluated_at_millis FROM submission_evaluation WHERE tenant_id = $1 AND attempt_id = $2 AND (grading_status = 'needs_manual_grading' OR EXISTS (SELECT 1 FROM manual_grade_receipt receipt WHERE receipt.tenant_id = submission_evaluation.tenant_id AND receipt.attempt_id = submission_evaluation.attempt_id)) FOR UPDATE")
        .bind(tenant.as_uuid()).bind(attempt.as_uuid()).fetch_optional(&mut **transaction).await.map_err(map_sqlx_error)?, tenant, attempt)
}

fn manual_evaluation_row(
    row: Option<PgRow>,
    tenant: TenantId,
    attempt: QuestionAttemptId,
) -> Result<Option<ManualEvaluationRecord>, StoreError> {
    row.map(|row| {
        let status: String = row.try_get("grading_status").map_err(map_sqlx_error)?;
        let credit: Option<BigDecimal> = row.try_get("credit_fraction").map_err(map_sqlx_error)?;
        let revision = EvaluationRevision::from_u64(
            u64::try_from(
                row.try_get::<i64, _>("evaluation_revision")
                    .map_err(map_sqlx_error)?,
            )
            .map_err(|_| {
                StoreError::Unavailable("stored evaluation revision is invalid".to_string())
            })?,
        )
        .ok_or_else(|| {
            StoreError::Unavailable("stored evaluation revision is invalid".to_string())
        })?;
        let status = match status.as_str() {
            "needs_manual_grading" => ManualEvaluationStatus::NeedsManualGrading,
            "graded" => ManualEvaluationStatus::Graded,
            _ => {
                return Err(StoreError::Unavailable(
                    "stored manual evaluation status is invalid".to_string(),
                ));
            }
        };
        let credit = credit.map(ManualCredit::new).transpose()?;
        if matches!(status, ManualEvaluationStatus::NeedsManualGrading) != credit.is_none() {
            return Err(StoreError::Unavailable(
                "stored manual evaluation shape is invalid".to_string(),
            ));
        }
        Ok(ManualEvaluationRecord {
            tenant,
            attempt,
            revision,
            status,
            credit,
            evaluated_at: ActivityTimestamp::from_unix_millis(
                row.try_get("evaluated_at_millis").map_err(map_sqlx_error)?,
            ),
        })
    })
    .transpose()
}

fn manual_receipt_from_row(
    row: &PgRow,
    action: ManualGradeActionId,
    attempt: QuestionAttemptId,
) -> Result<ManualGradeReceipt, StoreError> {
    let revision = EvaluationRevision::from_u64(
        u64::try_from(
            row.try_get::<i64, _>("resulting_evaluation_revision")
                .map_err(map_sqlx_error)?,
        )
        .map_err(|_| {
            StoreError::Unavailable("stored manual receipt revision is invalid".to_string())
        })?,
    )
    .ok_or_else(|| {
        StoreError::Unavailable("stored manual receipt revision is invalid".to_string())
    })?;
    let generation = ScoringGeneration::new(
        u64::try_from(
            row.try_get::<i64, _>("scoring_generation")
                .map_err(map_sqlx_error)?,
        )
        .map_err(|_| StoreError::Unavailable("stored scoring generation is invalid".to_string()))?,
    )
    .ok_or_else(|| StoreError::Unavailable("stored scoring generation is invalid".to_string()))?;
    Ok(ManualGradeReceipt {
        action,
        attempt,
        resulting_revision: revision,
        scoring_generation: generation,
        occurred_at: ActivityTimestamp::from_unix_millis(
            row.try_get("occurred_at_millis").map_err(map_sqlx_error)?,
        ),
    })
}

async fn load_manual_attempt_for_update(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    attempt: QuestionAttemptId,
) -> Result<QuestionAttempt, StoreError> {
    let row = sqlx::query(
        "SELECT COALESCE(si.payload, qa.payload) AS payload, \
                COALESCE(si.payload_sha256, qa.payload_sha256) AS payload_sha256, \
                evaluation.payload AS evaluation_payload, \
                evaluation.payload_sha256 AS evaluation_payload_sha256, \
                evaluation.grading_status AS evaluation_grading_status, \
                qa.attempt_status AS current_attempt_status, \
                floor(extract(epoch FROM qa.submitted_at) * 1000)::bigint AS current_submitted_at, \
                floor(extract(epoch FROM timing.effective_deadline) * 1000)::bigint AS current_deadline_at \
         FROM question_attempt qa \
         LEFT JOIN submission_idempotency si \
           ON si.tenant_id = qa.tenant_id AND si.attempt_id = qa.attempt_id \
         LEFT JOIN submission_evaluation evaluation \
           ON evaluation.tenant_id = qa.tenant_id AND evaluation.attempt_id = qa.attempt_id \
         LEFT JOIN attempt_timing_current timing \
           ON timing.tenant_id = qa.tenant_id AND timing.attempt_id = qa.attempt_id \
         WHERE qa.tenant_id = $1 AND qa.attempt_id = $2 \
         ORDER BY qa.occurred_at LIMIT 1 FOR UPDATE OF qa",
    )
    .bind(tenant.as_uuid())
    .bind(attempt.as_uuid())
    .fetch_optional(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?
    .ok_or(StoreError::NotFound)?;
    decode_current_attempt_with_evaluation_row(&row)
}
