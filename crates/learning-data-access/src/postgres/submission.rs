use super::*;
use crate::{LearnerDisclosureInput, ReceiptPresentationSnapshot};

/// Reads the current sealed S3 receipt for an already-authorized attempt and
/// pairs it with the current assignment disclosure policy and database clock.
/// A missing or cross-wired receipt is unavailable authority, never a legacy
/// policy fallback.
#[cfg(feature = "postgres")]
pub(super) async fn current_disclosure_input(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    assignment: &AssignmentRecord,
    attempt: QuestionAttemptId,
    submitted_at: Option<ActivityTimestamp>,
) -> Result<LearnerDisclosureInput, StoreError> {
    let bound: bool = sqlx::query_scalar(
        "SELECT EXISTS( \
            SELECT 1 FROM attempt_effective_policy_current AS current_effect \
            JOIN attempt_effective_policy_receipt AS receipt \
              ON receipt.tenant_id=current_effect.tenant_id \
             AND receipt.attempt_id=current_effect.attempt_id \
             AND receipt.receipt_generation=current_effect.receipt_generation \
            WHERE current_effect.tenant_id=$1 AND current_effect.attempt_id=$2 \
              AND receipt.course_id=$3 AND receipt.assignment_id=$4 \
              AND receipt.sealed_at IS NOT NULL)",
    )
    .bind(tenant.as_uuid())
    .bind(attempt.as_uuid())
    .bind(assignment.course_id.as_uuid())
    .bind(assignment.id.as_uuid())
    .fetch_one(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    if !bound {
        return Err(StoreError::Unavailable(
            "current effective-policy receipt does not bind the prepared assignment".to_string(),
        ));
    }
    let receipt = super::effective_policy_receipts::read_current_effective_policy_receipt(
        transaction,
        tenant,
        attempt,
    )
    .await?
    .ok_or_else(|| {
        StoreError::Unavailable("current effective-policy receipt is missing".to_string())
    })?;
    if receipt.attempt != attempt {
        return Err(StoreError::Unavailable(
            "current effective-policy receipt does not bind the attempt".to_string(),
        ));
    }
    Ok(LearnerDisclosureInput::new(
        assignment.disclosure_policy,
        receipt.policy,
        database_timestamp(transaction).await?,
        submitted_at,
    ))
}

#[cfg(feature = "postgres")]
pub(super) async fn apply_postgres_attempt_support(
    transaction: &mut Transaction<'_, Postgres>,
    context: TenantContext,
    action_id: AttemptSupportActionId,
    actor: UserId,
    attempt_id: QuestionAttemptId,
    action: AttemptSupportAction,
) -> Result<AttemptSupportRecord, StoreError> {
    let tenant = context.tenant_id();
    let previous = load_attempt_for_external_update(transaction, tenant, attempt_id).await?;
    let run = load_run_for_update(transaction, tenant, previous.run).await?;
    let enrollment = load_enrollment_for_update(transaction, tenant, run.enrollment).await?;
    let assignment = load_assignment(transaction, tenant, enrollment.assignment).await?;
    if !postgres_is_course_instructor(transaction, tenant, assignment.course_id, actor).await? {
        return Err(StoreError::NotFound);
    }

    // The audit table is time-partitioned, so its primary key necessarily
    // includes occurred_at. Serialize this application-owned identity before
    // querying it to preserve cross-partition, cross-attempt retry safety.
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1::uuid::text, 0))")
        .bind(action_id.as_uuid())
        .execute(&mut **transaction)
        .await
        .map_err(map_sqlx_error)?;
    let prior_rows = sqlx::query(
        "SELECT actor_id, course_id, action, target_kind, target_id, payload, \
                payload_sha256, \
                floor(extract(epoch FROM occurred_at) * 1000)::bigint \
                    AS occurred_at_millis \
         FROM audit_event \
         WHERE tenant_id = $1 AND audit_event_id = $2 \
         ORDER BY occurred_at LIMIT 2",
    )
    .bind(tenant.as_uuid())
    .bind(action_id.as_uuid())
    .fetch_all(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    if prior_rows.len() > 1 {
        return Err(StoreError::Unavailable(
            "attempt support action identity is duplicated".to_string(),
        ));
    }
    if let Some(row) = prior_rows.first() {
        let payload: AttemptSupportAuditPayload = decode_payload_row(row)?;
        let prior_actor: Uuid = row.try_get("actor_id").map_err(map_sqlx_error)?;
        let prior_course: Option<Uuid> = row.try_get("course_id").map_err(map_sqlx_error)?;
        let prior_action: String = row.try_get("action").map_err(map_sqlx_error)?;
        let target_kind: String = row.try_get("target_kind").map_err(map_sqlx_error)?;
        let target_id: Uuid = row.try_get("target_id").map_err(map_sqlx_error)?;
        if prior_actor != actor.as_uuid()
            || prior_course != Some(assignment.course_id.as_uuid())
            || prior_action != action.audit_name()
            || target_kind != "question_attempt"
            || target_id != attempt_id.as_uuid()
        {
            return Err(StoreError::Conflict);
        }
        return Ok(AttemptSupportRecord {
            tenant,
            action: action_id,
            actor,
            attempt: attempt_id,
            kind: action,
            previous_status: payload.previous_status,
            resulting_status: payload.resulting_status,
            occurred_at: ActivityTimestamp::from_unix_millis(
                row.try_get("occurred_at_millis").map_err(map_sqlx_error)?,
            ),
        });
    }

    let resulting_status = match action {
        AttemptSupportAction::ForceSubmit if previous.status == AttemptStatus::InProgress => {
            AttemptStatus::NeedsManualGrading
        }
        AttemptSupportAction::Clear
            if matches!(
                previous.status,
                AttemptStatus::InProgress
                    | AttemptStatus::Submitted
                    | AttemptStatus::AutoSubmitted
                    | AttemptStatus::NeedsManualGrading
            ) =>
        {
            AttemptStatus::Cleared
        }
        _ => return Err(StoreError::Conflict),
    };
    let updated = sqlx::query(
        "UPDATE question_attempt \
         SET attempt_status = $3, \
             submitted_at = CASE WHEN $4 THEN transaction_timestamp() ELSE submitted_at END \
         WHERE tenant_id = $1 AND attempt_id = $2",
    )
    .bind(tenant.as_uuid())
    .bind(attempt_id.as_uuid())
    .bind(attempt_status_name(resulting_status))
    .bind(action == AttemptSupportAction::ForceSubmit)
    .execute(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    if updated.rows_affected() != 1 {
        return Err(StoreError::Conflict);
    }
    sqlx::query(
        "DELETE FROM webwork_grade_replay_state \
         WHERE tenant_id = $1 AND attempt_id = $2",
    )
    .bind(tenant.as_uuid())
    .bind(attempt_id.as_uuid())
    .execute(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    assignment_timing::cancel_postgres_effective_policy_job(transaction, tenant, attempt_id)
        .await?;

    let has_evaluation: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM submission_evaluation \
         WHERE tenant_id = $1 AND attempt_id = $2)",
    )
    .bind(tenant.as_uuid())
    .bind(attempt_id.as_uuid())
    .fetch_one(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    if action == AttemptSupportAction::Clear && has_evaluation {
        super::assignment_recalculation::enqueue_assignment_recalculation(
            transaction,
            tenant,
            assignment.id,
        )
        .await?;
    }

    let occurred_at = database_timestamp(transaction).await?;
    let audit_payload = AttemptSupportAuditPayload {
        previous_status: previous.status,
        resulting_status,
    };
    let (payload, checksum) = encode_payload(&audit_payload)?;
    sqlx::query(
        "INSERT INTO audit_event \
         (tenant_id, audit_event_id, occurred_at, actor_id, course_id, action, \
          target_kind, target_id, payload, payload_sha256) \
         VALUES ($1, $2, transaction_timestamp(), $3, $4, $5, \
                 'question_attempt', $6, $7, $8)",
    )
    .bind(tenant.as_uuid())
    .bind(action_id.as_uuid())
    .bind(actor.as_uuid())
    .bind(assignment.course_id.as_uuid())
    .bind(action.audit_name())
    .bind(attempt_id.as_uuid())
    .bind(payload)
    .bind(checksum)
    .execute(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    Ok(AttemptSupportRecord {
        tenant,
        action: action_id,
        actor,
        attempt: attempt_id,
        kind: action,
        previous_status: previous.status,
        resulting_status,
        occurred_at,
    })
}

#[cfg(feature = "postgres")]
pub(super) async fn load_attempt_for_external_update(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    attempt: QuestionAttemptId,
) -> Result<QuestionAttempt, StoreError> {
    load_attempt_with_query(
        transaction,
        tenant,
        attempt,
        concat!(
            "SELECT attempt.payload, attempt.payload_sha256, \
            attempt.attempt_status AS current_attempt_status, \
            floor(extract(epoch FROM attempt.submitted_at) * 1000)::bigint AS current_submitted_at, \
            floor(extract(epoch FROM timing.effective_deadline) * 1000)::bigint AS current_deadline_at \
        FROM question_attempt AS attempt \
        LEFT JOIN attempt_effective_policy_current AS current_effect \
          ON current_effect.tenant_id = attempt.tenant_id AND current_effect.attempt_id = attempt.attempt_id \
        LEFT JOIN attempt_effective_policy_receipt AS timing \
          ON timing.tenant_id=current_effect.tenant_id AND timing.attempt_id=current_effect.attempt_id AND timing.receipt_generation=current_effect.receipt_generation \
        WHERE attempt.tenant_id = $1 AND attempt.attempt_id = $2 \
        ORDER BY attempt.occurred_at LIMIT 1",
            " FOR UPDATE OF attempt"
        ),
    )
    .await
}

/// Loads the current attempt projection without acquiring mutation authority.
#[cfg(feature = "postgres")]
pub(super) async fn load_attempt_for_read(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    attempt: QuestionAttemptId,
) -> Result<QuestionAttempt, StoreError> {
    load_attempt_with_query(
        transaction,
        tenant,
        attempt,
        "SELECT attempt.payload, attempt.payload_sha256, \
            attempt.attempt_status AS current_attempt_status, \
            floor(extract(epoch FROM attempt.submitted_at) * 1000)::bigint AS current_submitted_at, \
            floor(extract(epoch FROM timing.effective_deadline) * 1000)::bigint AS current_deadline_at \
        FROM question_attempt AS attempt \
        LEFT JOIN attempt_effective_policy_current AS current_effect \
          ON current_effect.tenant_id = attempt.tenant_id AND current_effect.attempt_id = attempt.attempt_id \
        LEFT JOIN attempt_effective_policy_receipt AS timing \
          ON timing.tenant_id=current_effect.tenant_id AND timing.attempt_id=current_effect.attempt_id AND timing.receipt_generation=current_effect.receipt_generation \
        WHERE attempt.tenant_id = $1 AND attempt.attempt_id = $2 \
        ORDER BY attempt.occurred_at LIMIT 1",
    )
    .await
}

#[cfg(feature = "postgres")]
async fn load_attempt_with_query(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    attempt: QuestionAttemptId,
    query: &'static str,
) -> Result<QuestionAttempt, StoreError> {
    let row = sqlx::query(query)
        .bind(tenant.as_uuid())
        .bind(attempt.as_uuid())
        .fetch_optional(&mut **transaction)
        .await
        .map_err(map_sqlx_error)?
        .ok_or(StoreError::NotFound)?;
    decode_current_attempt_row(&row)
}

#[cfg(feature = "postgres")]
pub(super) async fn submit_question_attempt(
    transaction: &mut Transaction<'_, Postgres>,
    context: TenantContext,
    command: SubmitQuestionAttemptCommand,
) -> Result<SubmissionRecord, StoreError> {
    let tenant = context.tenant_id();
    // ASVS 2.3.1/2.3.3/8.2.2: reauthorize the exact aggregate in the
    // retryable commit transaction before any protected source read.
    let prepared = super::submission_preparation::prepare_bound_student_attempt(
        transaction,
        tenant,
        command.binding,
        command.actor,
        command.attempt,
    )
    .await?;
    if let Some(replay) = super::submission_preparation::prepared_submission_replay(
        transaction,
        tenant,
        &command.response,
        &command.idempotency_key,
        &prepared,
    )
    .await?
    {
        return Ok(replay);
    }
    let base = prepared.attempt;
    if base.status != AttemptStatus::InProgress {
        return Err(StoreError::Conflict);
    }
    let presentation_capability = prepared.presentation_capability;
    let presentation = prepared.presentation;
    let feedback = private_feedback_record(command.feedback.clone())?;

    let mut run = prepared.run;
    if run.completed_at.is_some() || run.score.is_some() {
        return Err(StoreError::Conflict);
    }
    let mut enrollment = prepared.enrollment;
    let assignment = prepared.assignment;
    crate::validate_attempt_result(command.result)?;
    let submitted_at = database_timestamp(transaction).await?;
    let mut submitted = base;
    submitted.response = Some(command.response.clone());
    submitted.status = AttemptStatus::Submitted;
    submitted.result = Some(command.result);
    submitted.timer.submitted_at = Some(submitted_at);
    let disclosure = current_disclosure_input(
        transaction,
        tenant,
        &assignment,
        submitted.id,
        submitted.timer.submitted_at,
    )
    .await?;
    let effective_grace: Option<i32> = sqlx::query_scalar(
        "SELECT receipt.effective_grace_seconds \
           FROM attempt_effective_policy_current AS current_effect \
           JOIN attempt_effective_policy_receipt AS receipt \
             ON receipt.tenant_id=current_effect.tenant_id \
            AND receipt.attempt_id=current_effect.attempt_id \
            AND receipt.receipt_generation=current_effect.receipt_generation \
          WHERE current_effect.tenant_id=$1 AND current_effect.attempt_id=$2",
    )
    .bind(tenant.as_uuid())
    .bind(submitted.id.as_uuid())
    .fetch_optional(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?
    .flatten();
    let effective_policy = match effective_grace {
        Some(grace_seconds) if submitted.timer.deadline.is_some() => TimingPolicy::PerQuestion {
            seconds: 1,
            grace_seconds: u32::try_from(grace_seconds).map_err(|_| {
                StoreError::Unavailable("stored effective grace is invalid".to_string())
            })?,
        },
        Some(_) => TimingPolicy::Untimed,
        None => {
            return Err(StoreError::Unavailable(
                "issued timing authority is missing".to_string(),
            ));
        }
    };
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

    let previous = prepared.summary;
    let mut next = project_summary(
        &previous,
        domain::scoring::RunTransition::QuestionAttemptRecorded { at: submitted_at },
        grade_policy(&assignment),
    )?;
    let rows = sqlx::query(
        "SELECT COALESCE(si.payload, qa.payload) AS payload, \
                COALESCE(si.payload_sha256, qa.payload_sha256) AS payload_sha256, \
                evaluation.payload AS evaluation_payload, \
                evaluation.payload_sha256 AS evaluation_payload_sha256, \
                evaluation.grading_status AS evaluation_grading_status, \
                qa.attempt_status AS current_attempt_status, \
                floor(extract(epoch FROM qa.submitted_at) * 1000)::bigint \
                    AS current_submitted_at, \
                floor(extract(epoch FROM timing.effective_deadline) * 1000)::bigint \
                    AS current_deadline_at \
         FROM question_attempt AS qa \
         LEFT JOIN submission_idempotency AS si \
           ON si.tenant_id = qa.tenant_id AND si.attempt_id = qa.attempt_id \
         LEFT JOIN submission_evaluation AS evaluation \
           ON evaluation.tenant_id = qa.tenant_id AND evaluation.attempt_id = qa.attempt_id \
         LEFT JOIN attempt_effective_policy_current AS current_effect \
           ON current_effect.tenant_id = qa.tenant_id AND current_effect.attempt_id = qa.attempt_id \
         LEFT JOIN attempt_effective_policy_receipt AS timing \
           ON timing.tenant_id=current_effect.tenant_id AND timing.attempt_id=current_effect.attempt_id AND timing.receipt_generation=current_effect.receipt_generation \
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
        .collect::<Result<Vec<QuestionAttempt>, StoreError>>()?;
    let run_items = load_assignment_run_items(transaction, tenant, run.id).await?;
    let questions = current_run_questions(&assignment, &run_items, &attempts, &submitted)?;
    let results = questions
        .iter()
        .map(|question| question.map(|question| question.result))
        .collect::<Vec<_>>();
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
            let attempts = attempts
                .iter()
                .map(|attempt| {
                    if attempt.id == submitted.id {
                        submitted.clone()
                    } else {
                        attempt.clone()
                    }
                })
                .collect::<Vec<_>>();
            statistics_contributions = Some(derive_statistics_contributions(
                &run_items, &results, &attempts,
            )?);
        }
    }
    let (attempt_payload, attempt_checksum) = encode_payload(&submitted)?;
    let feedback_columns = encode_feedback_columns(feedback.content())?;
    sqlx::query(
        "INSERT INTO attempt_feedback \
         (tenant_id, attempt_id, hint, correct_response, rationale, content_sha256) \
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
    let (_, response_checksum) = encode_payload(&command.response)?;
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
        "UPDATE question_attempt SET attempt_status = 'submitted', \
             submitted_at = transaction_timestamp() \
         WHERE tenant_id = $1 AND attempt_id = $2",
    )
    .bind(tenant.as_uuid())
    .bind(submitted.id.as_uuid())
    .execute(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    sqlx::query(
        "DELETE FROM webwork_grade_replay_state \
         WHERE tenant_id = $1 AND attempt_id = $2",
    )
    .bind(tenant.as_uuid())
    .bind(submitted.id.as_uuid())
    .execute(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    assignment_timing::cancel_postgres_effective_policy_job(transaction, tenant, submitted.id)
        .await?;
    let (response_payload, response_checksum) = encode_payload(&command.response)?;
    sqlx::query(
        "INSERT INTO submission \
         (tenant_id, submission_id, attempt_id, idempotency_key, occurred_at, payload, payload_sha256) \
         VALUES ($1, $2, $2, $3, transaction_timestamp(), $4, $5)",
    )
    .bind(tenant.as_uuid())
    .bind(submitted.id.as_uuid())
    .bind(command.idempotency_key.as_str())
    .bind(response_payload)
    .bind(response_checksum)
    .execute(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    let (grade_payload, grade_checksum) = encode_payload(&command.result)?;
    let (assignment_item, credit_fraction, earned_points, possible_points) =
        current_attempt_score(transaction, &assignment, &submitted, command.result).await?;
    sqlx::query(
        "INSERT INTO submission_evaluation \
         (tenant_id, attempt_id, submission_id, credit_fraction, correct, grading_status, \
          payload, payload_sha256) \
         VALUES ($1, $2, $2, $3::numeric, $4, 'graded', $5, $6) \
         ON CONFLICT (tenant_id, attempt_id) DO UPDATE \
         SET submission_id = EXCLUDED.submission_id, \
             credit_fraction = EXCLUDED.credit_fraction, correct = EXCLUDED.correct, \
             grading_status = EXCLUDED.grading_status, payload = EXCLUDED.payload, \
             payload_sha256 = EXCLUDED.payload_sha256, \
             evaluated_at = transaction_timestamp()",
    )
    .bind(tenant.as_uuid())
    .bind(submitted.id.as_uuid())
    .bind(credit_fraction)
    .bind(command.result.correct)
    .bind(grade_payload)
    .bind(grade_checksum)
    .execute(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    let scored = sqlx::query(
        "INSERT INTO attempt_score_current \
         (tenant_id, attempt_id, assignment_id, assignment_item_id, scoring_generation, \
          earned_points, possible_points, course_id) \
         SELECT $1, $2, a.assignment_id, $3, a.scoring_generation, $4::numeric, $5::numeric, \
                a.course_id \
           FROM assignment a WHERE a.tenant_id = $1 AND a.assignment_id = $6 \
         ON CONFLICT (tenant_id, attempt_id) DO UPDATE \
         SET assignment_id = EXCLUDED.assignment_id, \
             assignment_item_id = EXCLUDED.assignment_item_id, \
             scoring_generation = EXCLUDED.scoring_generation, \
             earned_points = EXCLUDED.earned_points, \
             possible_points = EXCLUDED.possible_points, \
             course_id = EXCLUDED.course_id, calculated_at = transaction_timestamp()",
    )
    .bind(tenant.as_uuid())
    .bind(submitted.id.as_uuid())
    .bind(assignment_item.as_uuid())
    .bind(earned_points)
    .bind(possible_points)
    .bind(assignment.id.as_uuid())
    .execute(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    if scored.rows_affected() != 1 {
        return Err(StoreError::Conflict);
    }

    if run.completed_at.is_some() {
        let (run_payload, run_checksum) = encode_payload(&run)?;
        sqlx::query(
            "UPDATE assignment_run SET completed_at = transaction_timestamp(), \
             payload = $3, payload_sha256 = $4 WHERE tenant_id = $1 AND run_id = $2",
        )
        .bind(tenant.as_uuid())
        .bind(run.id.as_uuid())
        .bind(run_payload)
        .bind(run_checksum)
        .execute(&mut **transaction)
        .await
        .map_err(map_sqlx_error)?;
        sqlx::query(
            "UPDATE enrollment SET first_completed_at = to_timestamp($3::double precision / 1000), \
                    current_grade_run_id = $4, \
                    best_grade_run_id = $5 \
             WHERE tenant_id = $1 AND enrollment_id = $2",
        )
        .bind(tenant.as_uuid())
        .bind(enrollment.id.as_uuid())
        .bind(
            enrollment
                .first_completed_at
                .map(|value| value.as_unix_millis()),
        )
        .bind(enrollment.current_grade_run.map(|value| value.as_uuid()))
        .bind(enrollment.best_grade_run.map(|value| value.as_uuid()))
        .execute(&mut **transaction)
        .await
        .map_err(map_sqlx_error)?;
    }
    store_summary(transaction, &next).await?;
    if let Some(contributions) = &statistics_contributions {
        for contribution in contributions {
            let recorded: bool = sqlx::query_scalar(
                "SELECT ple_record_question_statistics( \
                    $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)",
            )
            .bind(tenant.as_uuid())
            .bind(enrollment.id.as_uuid())
            .bind(run.id.as_uuid())
            .bind(contribution.first_scored_attempt.as_uuid())
            .bind(contribution.reference.problem.as_uuid())
            .bind(contribution.reference.version.as_uuid())
            .bind(contribution.observation.normalized_score())
            .bind(
                i64::try_from(contribution.observation.attempts()).map_err(|_| {
                    StoreError::InvalidRecord("statistics attempt count is too large".to_string())
                })?,
            )
            .bind(
                i64::try_from(contribution.observation.duration_seconds()).map_err(|_| {
                    StoreError::InvalidRecord("statistics duration is too large".to_string())
                })?,
            )
            .bind(contribution.observation.rest_score())
            .bind(contribution.checksum.as_bytes().to_vec())
            .fetch_one(&mut **transaction)
            .await
            .map_err(map_sqlx_error)?;
            if !recorded {
                return Err(StoreError::Conflict);
            }
        }
    }
    let (receipt_run, receipt_run_sha256) = encode_payload(&run)?;
    let (receipt_summary, receipt_summary_sha256) = encode_payload(&next)?;
    let (receipt_presentation, receipt_presentation_sha256) = presentation
        .as_ref()
        .map(encode_payload)
        .transpose()?
        .map_or((None, None), |(payload, checksum)| {
            (Some(payload), Some(checksum))
        });
    sqlx::query(
        "INSERT INTO submission_receipt_snapshot \
         (tenant_id, attempt_id, run_payload, run_payload_sha256, summary_payload, summary_payload_sha256, \
         presentation_payload, presentation_payload_sha256, presentation_required) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
    )
    .bind(tenant.as_uuid())
    .bind(submitted.id.as_uuid())
    .bind(receipt_run)
    .bind(receipt_run_sha256)
    .bind(receipt_summary)
    .bind(receipt_summary_sha256)
    .bind(receipt_presentation)
    .bind(receipt_presentation_sha256)
    .bind(presentation_capability.requires_snapshot())
    .execute(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    Ok(SubmissionRecord {
        attempt: submitted,
        run,
        summary: next,
        feedback,
        presentation,
        disclosure,
    })
}

#[cfg(feature = "postgres")]
pub(super) async fn load_submission_replay(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    attempt: QuestionAttemptId,
    response: &StudentResponse,
    idempotency_key: &SubmissionIdempotencyKey,
) -> Result<Option<SubmissionRecord>, StoreError> {
    let row = sqlx::query(
        "SELECT idempotency_key, request_contract_version, request_sha256, payload, payload_sha256 \
         FROM submission_idempotency WHERE tenant_id = $1 AND attempt_id = $2",
    )
    .bind(tenant.as_uuid())
    .bind(attempt.as_uuid())
    .fetch_optional(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    let Some(row) = row else {
        return Ok(None);
    };
    let stored_key: String = row.try_get("idempotency_key").map_err(map_sqlx_error)?;
    let request_contract_version: i16 = row
        .try_get("request_contract_version")
        .map_err(map_sqlx_error)?;
    let stored_response_checksum: String = row.try_get("request_sha256").map_err(map_sqlx_error)?;
    let (_, response_checksum) = encode_payload(response)?;
    if request_contract_version != 0
        || stored_key != idempotency_key.as_str()
        || stored_response_checksum != response_checksum
    {
        return Err(StoreError::Conflict);
    }
    load_submission_record(transaction, tenant, attempt).await
}

/// Reads the receipt retained with a committed submission. The idempotency
/// record establishes the committed attempt, the feedback row supplies private
/// teaching material, and the receipt snapshot supplies its immutable public
/// run/summary/presentation projection.
#[cfg(feature = "postgres")]
pub(super) async fn load_submission_record(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    attempt: QuestionAttemptId,
) -> Result<Option<SubmissionRecord>, StoreError> {
    let row = sqlx::query(
        "SELECT payload, payload_sha256 \
         FROM submission_idempotency WHERE tenant_id = $1 AND attempt_id = $2",
    )
    .bind(tenant.as_uuid())
    .bind(attempt.as_uuid())
    .fetch_optional(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    let Some(row) = row else {
        return Ok(None);
    };
    let submitted: QuestionAttempt = decode_payload_row(&row)?;
    let feedback = load_attempt_feedback(transaction, tenant, attempt).await?;
    let (run, summary, presentation) =
        load_submission_receipt_snapshot(transaction, tenant, attempt)
            .await?
            .ok_or_else(|| {
                StoreError::Unavailable(
                    "submission receipt snapshot is missing; it cannot be reconstructed"
                        .to_string(),
                )
            })?;
    let enrollment = load_postgres_enrollment(transaction, tenant, run.enrollment).await?;
    let assignment = load_assignment(transaction, tenant, enrollment.assignment).await?;
    let disclosure = current_disclosure_input(
        transaction,
        tenant,
        &assignment,
        attempt,
        submitted.timer.submitted_at,
    )
    .await?;
    let record = SubmissionRecord {
        attempt: submitted,
        run,
        summary,
        feedback,
        presentation,
        disclosure,
    };
    // The receipt is authoritative, but both receipt GET and idempotent replay
    // must reject a partial or mismatched write rather than treating it as a
    // new presentation. This reads only immutable attempt/receipt storage.
    let issued = load_issued_presentation(transaction, tenant, &record.attempt).await?;
    if record.presentation != issued {
        return Err(StoreError::Unavailable(
            "submission receipt presentation does not match its issued snapshot".to_string(),
        ));
    }
    Ok(Some(record))
}

#[cfg(feature = "postgres")]
pub(super) async fn load_submission_receipt_snapshot(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    attempt: QuestionAttemptId,
) -> Result<
    Option<(
        AssignmentRun,
        StudentAssignmentSummary,
        Option<ReceiptPresentationSnapshot>,
    )>,
    StoreError,
> {
    let row = sqlx::query(
        "SELECT run_payload AS payload, run_payload_sha256 AS payload_sha256, \
                summary_payload, summary_payload_sha256, presentation_payload, \
                presentation_payload_sha256, presentation_required \
         FROM submission_receipt_snapshot WHERE tenant_id = $1 AND attempt_id = $2",
    )
    .bind(tenant.as_uuid())
    .bind(attempt.as_uuid())
    .fetch_optional(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    let Some(row) = row else {
        return Ok(None);
    };
    let run: AssignmentRun = decode_payload_row(&row)?;
    // `decode_payload_row` uses payload/payload_sha256 names, so decode the
    // distinct summary columns explicitly to keep checksum verification exact.
    let summary_payload: Value = row.try_get("summary_payload").map_err(map_sqlx_error)?;
    let summary_sha256: String = row
        .try_get("summary_payload_sha256")
        .map_err(map_sqlx_error)?;
    let summary_bytes = serde_json::to_vec(&summary_payload).map_err(|error| {
        StoreError::InvalidRecord(format!("receipt summary encode failed: {error}"))
    })?;
    if Sha256Digest::compute(&summary_bytes).to_string() != summary_sha256 {
        return Err(StoreError::InvalidRecord(
            "receipt summary checksum mismatch".to_string(),
        ));
    }
    let summary = serde_json::from_value(summary_payload).map_err(|error| {
        StoreError::InvalidRecord(format!("receipt summary decode failed: {error}"))
    })?;
    let presentation_required: bool = row
        .try_get("presentation_required")
        .map_err(map_sqlx_error)?;
    let presentation = decode_receipt_presentation(&row, presentation_required)?;
    Ok(Some((run, summary, presentation)))
}

/// Reads and validates the exact presentation frozen at issue time without
/// acquiring mutation authority. Submission mutation is serialized by its
/// preparation capability and immutable idempotency record; receipt replay is
/// a projection over the same issued evidence.
pub(super) async fn load_issued_presentation(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    attempt: &QuestionAttempt,
) -> Result<Option<ReceiptPresentationSnapshot>, StoreError> {
    let row = sqlx::query(
        "SELECT presentation_descriptor_version, presentation_nonce, presentation_digest, \
                presentation_capability, presentation_payload, presentation_payload_sha256, \
                grading_envelope_payload, grading_envelope_payload_sha256 \
         FROM question_attempt WHERE tenant_id = $1 AND attempt_id = $2 \
         ORDER BY occurred_at LIMIT 1",
    )
    .bind(tenant.as_uuid())
    .bind(attempt.id.as_uuid())
    .fetch_optional(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?
    .ok_or(StoreError::NotFound)?;
    let capability = super::runs::attempt_issuance::presentation_capability_from_row(&row)?;
    let binding = decode_presentation_binding_row(&row)?;
    let snapshot =
        super::runs::attempt_issuance::decode_attempt_presentation_snapshot(&row, capability)?;
    let grading_envelope =
        super::runs::attempt_issuance::decode_attempt_grading_envelope(&row, capability)?;
    let snapshot = crate::validate_issued_presentation(
        capability,
        attempt,
        binding,
        snapshot.as_ref(),
        grading_envelope.as_ref(),
    )?;
    Ok(snapshot)
}

#[cfg(feature = "postgres")]
fn decode_receipt_presentation(
    row: &PgRow,
    required: bool,
) -> Result<Option<ReceiptPresentationSnapshot>, StoreError> {
    let payload: Option<Value> = row
        .try_get("presentation_payload")
        .map_err(map_sqlx_error)?;
    let checksum: Option<String> = row
        .try_get("presentation_payload_sha256")
        .map_err(map_sqlx_error)?;
    match (payload, checksum) {
        (None, None) if !required => Ok(None),
        (None, None) => Err(StoreError::Unavailable(
            "receipt requires a presentation snapshot but it is missing".to_string(),
        )),
        (Some(_), Some(_)) if !required => Err(StoreError::Unavailable(
            "receipt includes a presentation snapshot for a non-presentation family".to_string(),
        )),
        (Some(payload), Some(checksum)) => {
            let bytes = serde_json::to_vec(&payload).map_err(|error| {
                StoreError::InvalidRecord(format!("receipt presentation encode failed: {error}"))
            })?;
            if Sha256Digest::compute(&bytes).to_string() != checksum {
                return Err(StoreError::Unavailable(
                    "receipt presentation checksum mismatch".to_string(),
                ));
            }
            serde_json::from_value(payload).map(Some).map_err(|error| {
                StoreError::Unavailable(format!("receipt presentation decode failed: {error}"))
            })
        }
        _ => Err(StoreError::Unavailable(
            "receipt presentation payload and checksum disagree".to_string(),
        )),
    }
}

#[cfg(feature = "postgres")]
pub(super) async fn current_attempt_score(
    transaction: &mut Transaction<'_, Postgres>,
    assignment: &AssignmentRecord,
    attempt: &QuestionAttempt,
    result: AttemptResult,
) -> Result<(AssignmentItemId, String, String, String), StoreError> {
    let assignment_item =
        sqlx::query_scalar::<_, Uuid>(
            "SELECT assignment_item_id FROM assignment_run_item \
         WHERE tenant_id = $1 AND run_id = $2 AND issued_position = $3",
        )
        .bind(assignment.tenant.as_uuid())
        .bind(attempt.run.as_uuid())
        .bind(i32::try_from(attempt.assignment_position).map_err(|_| {
            StoreError::InvalidRecord("assignment position is too large".to_string())
        })?)
        .fetch_optional(&mut **transaction)
        .await
        .map_err(map_sqlx_error)?
        .map(AssignmentItemId::from_uuid)
        .ok_or_else(|| {
            StoreError::InvalidRecord(
                "submitted attempt does not resolve to an immutable run item".to_string(),
            )
        })?;
    let credit = result.points_earned / result.points_possible;
    let (earned, possible) =
        crate::current_attempt_points(assignment, assignment_item, attempt.status, result)?;
    Ok((
        assignment_item,
        format!("{credit:.12}"),
        format!("{earned:.4}"),
        format!("{possible:.4}"),
    ))
}
