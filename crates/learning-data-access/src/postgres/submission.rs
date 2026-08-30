use super::*;
use crate::StudentDisclosureInput;
use crate::canonical_json::{CanonicalJsonV1, canonical_json_bytes_v1};

/// Encodes the immutable receipt attempt without retaining the learner answer.
///
/// ASVS 1.1.1, 1.5.3, 2.2.1-2.2.3, and 14.2.4: response evidence remains in
/// the governed submission store; the receipt receives an answer-free typed
/// value with one canonical source, projection, and source-byte digest.
#[cfg(feature = "postgres")]
pub(super) fn encode_receipt_attempt_snapshot(
    attempt: &QuestionAttempt,
) -> Result<CanonicalJsonV1, StoreError> {
    let mut receipt_attempt = attempt.clone();
    receipt_attempt.response = None;
    encode_receipt_snapshot("submission receipt attempt", &receipt_attempt)
}

/// Encodes one immutable receipt member through the sole versioned evidence
/// protocol. Mutable current projections intentionally remain on
/// `encode_payload`; receipts are the byte-attested historical authority.
#[cfg(feature = "postgres")]
pub(super) fn encode_receipt_snapshot<T: Serialize>(
    artifact: &'static str,
    value: &T,
) -> Result<CanonicalJsonV1, StoreError> {
    canonical_json_bytes_v1(artifact, value)
}

/// Encodes private feedback as its immutable versioned source plus queryable
/// projection. The representation is the same closed three-position tuple
/// used by `AttemptFeedbackRecord::content_sha256`.
#[cfg(feature = "postgres")]
pub(super) fn encode_feedback_snapshot(
    feedback: &AttemptFeedbackRecord,
) -> Result<CanonicalJsonV1, StoreError> {
    let encoded = crate::feedback::canonical_feedback_json_v1(feedback.content())?;
    if encoded.sha256 != *feedback.content_sha256() {
        return Err(StoreError::InvalidRecord(
            "private feedback canonical evidence disagrees with validated feedback".to_string(),
        ));
    }
    Ok(encoded)
}

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
) -> Result<StudentDisclosureInput, StoreError> {
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
    Ok(StudentDisclosureInput::new(
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
    if !postgres_is_course_instructor(transaction, assignment.course_id, actor).await? {
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
        // ASVS 2.2.1-2.2.3, 2.3.1-2.3.4: persist the only authorized,
        // answer-free closure atomically with its replayable support receipt.
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
    let requires_scoring_invalidation = action == AttemptSupportAction::Clear && has_evaluation;

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
    if requires_scoring_invalidation {
        let binding = super::assignment_recalculation::enqueue_assignment_recalculation(
            transaction,
            tenant,
            assignment.id,
            JobId::from_uuid(action_id.as_uuid()),
        )
        .await?;
        super::scoring_invalidation::bind_attempt_support(
            transaction,
            tenant,
            action_id.as_uuid(),
            binding,
        )
        .await?;
    }
    Ok(AttemptSupportRecord {
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
    match super::submission_preparation::prepared_submission_replay(
        transaction,
        tenant,
        &command.response,
        &command.idempotency_key,
        &prepared,
    )
    .await?
    {
        crate::SubmissionReceiptRead::Completed(replay) => return Ok(*replay),
        crate::SubmissionReceiptRead::AcceptedPending(_) => return Err(StoreError::Conflict),
        crate::SubmissionReceiptRead::Missing => {}
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
        "SELECT CASE WHEN si.request_contract_version = 2 THEN qa.payload ELSE COALESCE(si.payload, qa.payload) END AS payload, \
                CASE WHEN si.request_contract_version = 2 THEN qa.payload_sha256 ELSE COALESCE(si.payload_sha256, qa.payload_sha256) END AS payload_sha256, \
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
    let feedback_snapshot = encode_feedback_snapshot(&feedback)?;
    let feedback_version = i16::try_from(feedback_snapshot.version).map_err(|_| {
        StoreError::InvalidRecord(
            "feedback canonical JSON version exceeds PostgreSQL smallint".to_string(),
        )
    })?;
    sqlx::query(
        "INSERT INTO attempt_feedback \
         (tenant_id, attempt_id, hint, correct_response, rationale, \
          content_canonical_json, content_canonical_json_version, content_sha256, course_id) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
    )
    .bind(tenant.as_uuid())
    .bind(submitted.id.as_uuid())
    .bind(feedback_columns.hint)
    .bind(feedback_columns.correct_response)
    .bind(feedback_columns.rationale)
    .bind(feedback_snapshot.source)
    .bind(feedback_version)
    .bind(feedback_snapshot.sha256.to_string())
    .bind(assignment.course_id.as_uuid())
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
    let receipt_attempt = encode_receipt_attempt_snapshot(&submitted)?;
    let receipt_run = encode_receipt_snapshot("submission receipt run", &run)?;
    let receipt_summary = encode_receipt_snapshot("submission receipt summary", &next)?;
    let receipt_presentation = presentation
        .as_ref()
        .map(|value| encode_receipt_snapshot("submission receipt presentation", value))
        .transpose()?;
    let receipt_version = i16::try_from(receipt_attempt.version).map_err(|_| {
        StoreError::InvalidRecord(
            "receipt canonical JSON version exceeds PostgreSQL smallint".to_string(),
        )
    })?;
    sqlx::query(
        "INSERT INTO submission_receipt_snapshot \
         (tenant_id, attempt_id, canonical_json_version, receipt_attempt_canonical_json, \
          receipt_attempt_payload, receipt_attempt_payload_sha256, run_canonical_json, \
          run_payload, run_payload_sha256, summary_canonical_json, summary_payload, \
          summary_payload_sha256, presentation_canonical_json, presentation_payload, \
          presentation_payload_sha256, presentation_required) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16)",
    )
    .bind(tenant.as_uuid())
    .bind(submitted.id.as_uuid())
    .bind(receipt_version)
    .bind(receipt_attempt.source)
    .bind(Json(receipt_attempt.projection))
    .bind(receipt_attempt.sha256.to_string())
    .bind(receipt_run.source)
    .bind(Json(receipt_run.projection))
    .bind(receipt_run.sha256.to_string())
    .bind(receipt_summary.source)
    .bind(Json(receipt_summary.projection))
    .bind(receipt_summary.sha256.to_string())
    .bind(
        receipt_presentation
            .as_ref()
            .map(|value| value.source.clone()),
    )
    .bind(
        receipt_presentation
            .as_ref()
            .map(|value| Json(value.projection.clone())),
    )
    .bind(
        receipt_presentation
            .as_ref()
            .map(|value| value.sha256.to_string()),
    )
    .bind(presentation_capability.requires_snapshot())
    .execute(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    let mut receipt_attempt_record = submitted.clone();
    receipt_attempt_record.response = None;
    Ok(SubmissionRecord {
        attempt: receipt_attempt_record,
        run,
        summary: next,
        feedback,
        presentation,
        disclosure,
    })
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

#[cfg(all(test, feature = "postgres"))]
mod tests {
    use super::*;
    use question_model::{
        ActivityTimestamp, AttemptProvenance, AttemptTimerRecord, ImplementationVersion,
        IssuedAttemptCapabilityV1, ProblemId, StudentResponse,
    };
    use uuid::Uuid;

    #[test]
    fn receipt_attempt_snapshot_is_answer_free_but_retains_grade_and_timing() {
        let attempt = QuestionAttempt {
            id: QuestionAttemptId::from_uuid(Uuid::from_u128(1)),
            run: RunId::from_uuid(Uuid::from_u128(3)),
            problem: ProblemId::from_uuid(Uuid::from_u128(4)),
            question_version: VersionId::from_uuid(Uuid::from_u128(5)),
            assignment_position: 0,
            seed: 6,
            parameter_hash: "parameters".to_string(),
            response: Some(StudentResponse::Numeric { value: 42.0 }),
            status: AttemptStatus::Submitted,
            result: Some(AttemptResult {
                correct: true,
                points_earned: 1.0,
                points_possible: 1.0,
            }),
            timer: AttemptTimerRecord {
                issued_at: ActivityTimestamp::from_unix_millis(10),
                deadline: Some(ActivityTimestamp::from_unix_millis(20)),
                submitted_at: Some(ActivityTimestamp::from_unix_millis(15)),
            },
            provenance: AttemptProvenance {
                adapter: ImplementationVersion {
                    id: "native".to_string(),
                    version: "1".to_string(),
                },
                renderer: None,
                generator: None,
                source_artifact: None,
                asset_objects: Vec::new(),
                grading: ImplementationVersion {
                    id: "native".to_string(),
                    version: "1".to_string(),
                },
                rendered_question_sha256: "rendered".to_string(),
            },
            issued_capability: IssuedAttemptCapabilityV1::NotApplicable,
        };

        let encoded = encode_receipt_attempt_snapshot(&attempt).expect("encode");
        assert_eq!(
            (encoded.version, encoded.sha256),
            (
                crate::canonical_json::CANONICAL_JSON_V1_VERSION,
                Sha256Digest::compute(encoded.source.as_bytes()),
            )
        );
        let snapshot: QuestionAttempt =
            serde_json::from_value(encoded.projection).expect("closed receipt attempt decodes");
        assert!(snapshot.response.is_none());
        assert_eq!(
            (snapshot.status, snapshot.result, snapshot.timer),
            (AttemptStatus::Submitted, attempt.result, attempt.timer)
        );
    }
}
