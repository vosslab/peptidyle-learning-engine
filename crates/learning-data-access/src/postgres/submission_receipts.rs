//! Immutable submission-receipt reads and exact replay binding.

use super::*;
use crate::{LearnerWorkRoutingBinding, ReceiptPresentationSnapshot};
use std::collections::BTreeMap;

#[async_trait::async_trait]
impl crate::LearnerSubmissionStatusStore for PostgresStore {
    async fn learner_submission_status(
        &self,
        context: TenantContext,
        actor: UserId,
        binding: LearnerWorkRoutingBinding,
        attempt: QuestionAttemptId,
    ) -> Result<crate::LearnerSubmissionStatusRead, StoreError> {
        let mut transaction = self.begin_tenant(context).await?;
        // ASVS V8.2.2/V8.3.1: the complete nested route assertion is part of
        // this authoritative status boundary.
        let authorized_binding =
            require_attempt_owner_for_read(&mut transaction, context.tenant_id(), attempt, actor)
                .await?;
        if authorized_binding != binding {
            return Err(StoreError::NotFound);
        }
        let status = learner_submission_status(
            &mut transaction,
            context.tenant_id(),
            authorized_binding,
            attempt,
        )
        .await?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(status)
    }
}

/// Closed durable state used only by the route-bound learner status reader.
///
/// This contains no job, execution, evaluation reason, or score identity;
/// callers combine it with the validated immutable receipt aggregate before
/// producing the public answer-free status projection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum AcceptedSubmissionStatusState {
    Pending,
    InstructorAttention,
    CompletedGraded,
    CompletedExempt,
    Contradictory,
}

/// Reads the coupled accepted-submission execution and evaluation state.
///
/// The caller has already established the exact learner route witness in the
/// same tenant transaction.  The fixed query is deliberately bounded to the
/// opaque attempt and tenant; it returns no response, job, worker, reason,
/// feedback, or result data (ASVS V1.2.4 and V8.2.2).
#[cfg(feature = "postgres")]
pub(super) async fn load_accepted_submission_status_state(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    attempt: QuestionAttemptId,
) -> Result<Option<AcceptedSubmissionStatusState>, StoreError> {
    let row = sqlx::query(
        "SELECT execution.state AS execution_state, evaluation.grading_status AS evaluation_status \
         FROM grading_execution AS execution \
         JOIN submission_evaluation AS evaluation \
           ON evaluation.tenant_id = execution.tenant_id \
          AND evaluation.attempt_id = execution.attempt_id \
         WHERE execution.tenant_id = $1 AND execution.attempt_id = $2",
    )
    .bind(tenant.as_uuid())
    .bind(attempt.as_uuid())
    .fetch_optional(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    row.map(|row| {
        let execution: String = row.try_get("execution_state").map_err(map_sqlx_error)?;
        let evaluation: String = row.try_get("evaluation_status").map_err(map_sqlx_error)?;
        Ok(match (execution.as_str(), evaluation.as_str()) {
            ("ready" | "running" | "retry_wait", "automated_pending") => {
                AcceptedSubmissionStatusState::Pending
            }
            (_, "needs_manual_grading") | ("exception", "automated_exception") => {
                AcceptedSubmissionStatusState::InstructorAttention
            }
            ("completed", "graded") => AcceptedSubmissionStatusState::CompletedGraded,
            ("completed", "exempt") => AcceptedSubmissionStatusState::CompletedExempt,
            _ => AcceptedSubmissionStatusState::Contradictory,
        })
    })
    .transpose()
}

/// Combines the canonical immutable receipt reader with the coupled accepted
/// execution/evaluation state.  The caller establishes authorization before
/// invoking this function in its tenant transaction.
#[cfg(feature = "postgres")]
pub(super) async fn learner_submission_status(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    binding: LearnerWorkRoutingBinding,
    attempt: QuestionAttemptId,
) -> Result<crate::LearnerSubmissionStatusRead, StoreError> {
    let receipt = load_submission_record_with_evaluation_verification(
        transaction,
        tenant,
        attempt,
        AcceptedEvaluationVerification::StatusRoute,
    )
    .await
    .map_err(|error| match error {
        StoreError::Conflict | StoreError::NotFound => StoreError::Unavailable(
            "learner submission status has an invalid durable aggregate".to_string(),
        ),
        other => other,
    })?;
    let durable = load_accepted_submission_status_state(transaction, tenant, attempt).await?;
    match (receipt, durable) {
        (crate::SubmissionReceiptRead::Missing, _) => Err(StoreError::NotFound),
        (
            crate::SubmissionReceiptRead::AcceptedPending(pending),
            Some(AcceptedSubmissionStatusState::Pending),
        ) => Ok(crate::LearnerSubmissionStatusRead::AcceptedPending(pending)),
        (
            crate::SubmissionReceiptRead::AcceptedPending(pending),
            Some(AcceptedSubmissionStatusState::InstructorAttention),
        ) => Ok(crate::LearnerSubmissionStatusRead::InstructorAttention(
            pending,
        )),
        (
            crate::SubmissionReceiptRead::Completed(record),
            Some(AcceptedSubmissionStatusState::CompletedGraded),
        ) => {
            validate_accepted_completed_graded_evaluation(
                transaction,
                tenant,
                binding,
                attempt,
                &record.attempt,
            )
            .await?;
            let next_pending =
                completed_successor_is_eligible(transaction, tenant, &record).await?;
            Ok(crate::LearnerSubmissionStatusRead::Completed {
                record,
                next_pending,
            })
        }
        (
            crate::SubmissionReceiptRead::Completed(record),
            Some(AcceptedSubmissionStatusState::CompletedExempt),
        ) => {
            // Exempt completions have no automated-result canonical evidence.
            // Their existing terminal projection remains receipt-checked here.
            validate_accepted_completed_evaluation(transaction, tenant, attempt, &record.attempt)
                .await?;
            let next_pending =
                completed_successor_is_eligible(transaction, tenant, &record).await?;
            Ok(crate::LearnerSubmissionStatusRead::Completed {
                record,
                next_pending,
            })
        }
        _ => Err(StoreError::Unavailable(
            "learner submission status has no coherent durable aggregate".to_string(),
        )),
    }
}

/// Derives the read-only half of successor delivery from the exact immutable
/// run plan, current attempt results, and pinned question policies.  The
/// status capability never writes `submission_next_attempt`; start/resume is
/// the sole delivery mutation owner (ASVS 2.3.1).
#[cfg(feature = "postgres")]
async fn completed_successor_is_eligible(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    record: &crate::SubmissionRecord,
) -> Result<bool, StoreError> {
    let predecessor_resolved: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM submission_next_attempt \
         WHERE tenant_id = $1 AND predecessor_attempt_id = $2)",
    )
    .bind(tenant.as_uuid())
    .bind(record.attempt.id.as_uuid())
    .fetch_one(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    let run_items =
        super::run_lifecycle::load_assignment_run_items(transaction, tenant, record.run.id).await?;
    let rows = sqlx::query(
        "SELECT CASE WHEN si.request_contract_version = 2 THEN qa.payload ELSE COALESCE(si.payload, qa.payload) END AS payload, \
                CASE WHEN si.request_contract_version = 2 THEN qa.payload_sha256 ELSE COALESCE(si.payload_sha256, qa.payload_sha256) END AS payload_sha256, \
                evaluation.payload AS evaluation_payload, evaluation.payload_sha256 AS evaluation_payload_sha256, \
                evaluation.grading_status AS evaluation_grading_status, qa.attempt_status AS current_attempt_status, \
                floor(extract(epoch FROM qa.submitted_at) * 1000)::bigint AS current_submitted_at, \
                floor(extract(epoch FROM timing.effective_deadline) * 1000)::bigint AS current_deadline_at \
         FROM question_attempt AS qa \
         LEFT JOIN submission_idempotency AS si ON si.tenant_id = qa.tenant_id AND si.attempt_id = qa.attempt_id \
         LEFT JOIN submission_evaluation AS evaluation ON evaluation.tenant_id = qa.tenant_id AND evaluation.attempt_id = qa.attempt_id \
         LEFT JOIN attempt_effective_policy_current AS current_effect ON current_effect.tenant_id = qa.tenant_id AND current_effect.attempt_id = qa.attempt_id \
         LEFT JOIN attempt_effective_policy_receipt AS timing ON timing.tenant_id = current_effect.tenant_id AND timing.attempt_id = current_effect.attempt_id AND timing.receipt_generation = current_effect.receipt_generation \
         WHERE qa.tenant_id = $1 AND qa.run_id = $2 ORDER BY qa.assignment_position, qa.occurred_at, qa.attempt_id::text",
    )
    .bind(tenant.as_uuid())
    .bind(record.run.id.as_uuid())
    .fetch_all(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    let attempts = rows
        .iter()
        .map(decode_current_attempt_with_evaluation_row)
        .collect::<Result<Vec<_>, _>>()?;
    // This is one bounded run-plan query rather than a catalog lookup per
    // item: learner status may be polled repeatedly while a successor waits.
    let question_rows = sqlx::query(
        "SELECT pv.problem_id, p.question_id, pv.version_id, pvp.payload, pvp.payload_sha256, \
                pv.lifecycle, pv.lifecycle_reason, pv.author_ids, pv.public_byline \
         FROM assignment_run_item AS item \
         JOIN problem_version AS pv ON pv.problem_id = item.problem_id AND pv.version_id = item.version_id \
         JOIN problem AS p ON p.problem_id = pv.problem_id \
         JOIN problem_version_payload AS pvp \
           ON pvp.problem_id = pv.problem_id AND pvp.version_id = pv.version_id \
         WHERE item.tenant_id = $1 AND item.run_id = $2 \
         ORDER BY item.issued_position",
    )
    .bind(tenant.as_uuid())
    .bind(record.run.id.as_uuid())
    .fetch_all(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    let mut questions = BTreeMap::new();
    for row in &question_rows {
        let published = decode_catalog_payload_row(row)?;
        questions.insert(
            question_model::ProblemVersionRef {
                problem: published.problem,
                version: published.version,
            },
            published.question,
        );
    }
    if run_items
        .iter()
        .any(|item| !questions.contains_key(&item.reference))
    {
        return Err(StoreError::Unavailable(
            "run item has no immutable published question".to_string(),
        ));
    }
    let questions = questions.into_iter().collect::<Vec<_>>();
    crate::successor_is_eligible(
        &record.run,
        predecessor_resolved,
        &run_items,
        &attempts,
        &questions,
    )
}

/// Returns the durable receipt state for one exact retry before any grader runs.
#[cfg(feature = "postgres")]
pub(super) async fn load_submission_replay(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    attempt: QuestionAttemptId,
    response: &StudentResponse,
    idempotency_key: &SubmissionIdempotencyKey,
) -> Result<crate::SubmissionReceiptRead, StoreError> {
    let Some(metadata) = load_submission_replay_metadata(transaction, tenant, attempt).await?
    else {
        return Ok(crate::SubmissionReceiptRead::Missing);
    };
    if metadata.idempotency_key != idempotency_key.as_str() {
        return Err(StoreError::Conflict);
    }
    match metadata.request_contract_version {
        0 => {
            let (_, response_checksum) = encode_payload(response)?;
            if metadata.request_sha256 != response_checksum {
                return Err(StoreError::Conflict);
            }
        }
        2 => {
            validate_accepted_replay_metadata(&metadata, response, idempotency_key)?;
        }
        _ => return Err(StoreError::Conflict),
    }
    load_submission_record(transaction, tenant, attempt).await
}

pub(super) struct SubmissionReplayMetadata {
    pub(super) idempotency_key: String,
    pub(super) request_contract_version: i16,
    pub(super) request_sha256: String,
}

/// Reads only the stable replay identity shared by legacy and accepted-input
/// flows. Callers request a payload separately only after selecting legacy
/// contract 0 (ASVS 8.1-8.4 and 14.2).
pub(super) async fn load_submission_replay_metadata(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    attempt: QuestionAttemptId,
) -> Result<Option<SubmissionReplayMetadata>, StoreError> {
    sqlx::query(
        "SELECT idempotency_key, request_contract_version, request_sha256 \
         FROM submission_idempotency WHERE tenant_id = $1 AND attempt_id = $2",
    )
    .bind(tenant.as_uuid())
    .bind(attempt.as_uuid())
    .fetch_optional(&mut **transaction)
    .await
    .map_err(map_sqlx_error)
    .and_then(|row| {
        row.map(|row| {
            Ok(SubmissionReplayMetadata {
                idempotency_key: row.try_get("idempotency_key").map_err(map_sqlx_error)?,
                request_contract_version: row
                    .try_get("request_contract_version")
                    .map_err(map_sqlx_error)?,
                request_sha256: row.try_get("request_sha256").map_err(map_sqlx_error)?,
            })
        })
        .transpose()
    })
}

/// Produces the one canonical digest accepted by the G1 broker.
///
/// `StudentResponse` is already a closed type. Its shared canonical serializer
/// is therefore the only digest source; JSONB is parsed solely for structural
/// equality with the immutable stored input.
#[cfg(feature = "postgres")]
pub(super) fn automated_response_digest(response: &StudentResponse) -> Result<String, StoreError> {
    let canonical = crate::canonical_student_response_json(response)?;
    Ok(Sha256Digest::compute(canonical.as_bytes()).to_string())
}

/// Produces the accepted-input replay state from answer-free metadata only.
///
/// ASVS 1.5.2-1.5.3, 2.3, and 14.2: an exact v2 retry binds its typed response
/// to the stored canonical digest without loading any generic response field.
pub(super) fn accepted_pending_replay_from_metadata(
    metadata: &SubmissionReplayMetadata,
    response: &StudentResponse,
    idempotency_key: &SubmissionIdempotencyKey,
    attempt: QuestionAttemptId,
) -> Result<crate::SubmissionReceiptRead, StoreError> {
    validate_accepted_replay_metadata(metadata, response, idempotency_key)?;
    Ok(crate::SubmissionReceiptRead::AcceptedPending(
        crate::AcceptedSubmissionPending::new(attempt),
    ))
}

/// Validates accepted replay identity without selecting a current receipt
/// state. The receipt loader is the durable pending-versus-completed source.
#[cfg(feature = "postgres")]
fn validate_accepted_replay_metadata(
    metadata: &SubmissionReplayMetadata,
    response: &StudentResponse,
    idempotency_key: &SubmissionIdempotencyKey,
) -> Result<(), StoreError> {
    if metadata.idempotency_key != idempotency_key.as_str()
        || metadata.request_contract_version != 2
        || metadata.request_sha256 != automated_response_digest(response)?
    {
        return Err(StoreError::Conflict);
    }
    Ok(())
}

/// Reads the immutable receipt state for one owned submission.
///
/// The idempotency row establishes accepted input; the normalized immutable
/// receipt establishes completion. Receipt reads never rebuild learner work
/// from mutable attempt or catalog records.
#[cfg(feature = "postgres")]
pub(super) async fn load_submission_record(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    attempt: QuestionAttemptId,
) -> Result<crate::SubmissionReceiptRead, StoreError> {
    load_submission_record_with_evaluation_verification(
        transaction,
        tenant,
        attempt,
        AcceptedEvaluationVerification::Direct,
    )
    .await
}

/// Selects how an accepted completed evaluation is verified after receipt
/// decoding. The learner status route supplies the route witness required by
/// the four-key database verifier; general receipt readers retain their
/// established direct projection verification.
#[cfg(feature = "postgres")]
#[derive(Clone, Copy, PartialEq, Eq)]
enum AcceptedEvaluationVerification {
    Direct,
    StatusRoute,
}

#[cfg(feature = "postgres")]
async fn load_submission_record_with_evaluation_verification(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    attempt: QuestionAttemptId,
    evaluation_verification: AcceptedEvaluationVerification,
) -> Result<crate::SubmissionReceiptRead, StoreError> {
    let Some(metadata) = load_submission_replay_metadata(transaction, tenant, attempt).await?
    else {
        return Ok(crate::SubmissionReceiptRead::Missing);
    };
    if !matches!(metadata.request_contract_version, 0 | 2) {
        return Err(StoreError::Conflict);
    }
    let Some(receipt) = load_submission_receipt_snapshot(transaction, tenant, attempt).await?
    else {
        return if metadata.request_contract_version == 2 {
            Ok(crate::SubmissionReceiptRead::AcceptedPending(
                crate::AcceptedSubmissionPending::new(attempt),
            ))
        } else {
            Err(StoreError::Unavailable(
                "completed submission receipt snapshot is missing".to_string(),
            ))
        };
    };
    if metadata.request_contract_version == 2
        && evaluation_verification == AcceptedEvaluationVerification::Direct
    {
        validate_accepted_completed_evaluation(transaction, tenant, attempt, &receipt.attempt)
            .await?;
    }
    let enrollment = load_postgres_enrollment(transaction, tenant, receipt.run.enrollment).await?;
    let assignment = load_assignment(transaction, tenant, enrollment.assignment).await?;
    // Current disclosure is applied only after every immutable receipt field
    // has been decoded and cross-checked.
    let disclosure = super::submission::current_disclosure_input(
        transaction,
        tenant,
        &assignment,
        attempt,
        receipt.attempt.timer.submitted_at,
    )
    .await?;
    Ok(crate::SubmissionReceiptRead::Completed(Box::new(
        receipt.into_submission_record(disclosure),
    )))
}

/// Verifies the app-readable evaluation projection against its frozen receipt.
/// The worker-owned canonical evidence remains outside `ple_app`; the immutable
/// receipt is the disclosure anchor and the evaluation checksum detects drift.
#[cfg(feature = "postgres")]
async fn validate_accepted_completed_evaluation(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    attempt: QuestionAttemptId,
    receipt_attempt: &QuestionAttempt,
) -> Result<(), StoreError> {
    if receipt_attempt.status != AttemptStatus::Submitted || receipt_attempt.result.is_none() {
        return Err(StoreError::Unavailable(
            "accepted completed receipt has an invalid terminal attempt".to_string(),
        ));
    }
    let row = sqlx::query(
        "SELECT grading_status, payload AS evaluation_payload, \
                payload_sha256 AS evaluation_payload_sha256 \
         FROM submission_evaluation \
         WHERE tenant_id = $1 AND attempt_id = $2",
    )
    .bind(tenant.as_uuid())
    .bind(attempt.as_uuid())
    .fetch_optional(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?
    .ok_or_else(|| {
        StoreError::Unavailable("completed receipt evaluation is missing".to_string())
    })?;
    let status: String = row.try_get("grading_status").map_err(map_sqlx_error)?;
    if !matches!(status.as_str(), "graded" | "exempt") {
        return Err(StoreError::Unavailable(
            "completed receipt evaluation is not terminal".to_string(),
        ));
    }
    let projection: Value = row.try_get("evaluation_payload").map_err(map_sqlx_error)?;
    let checksum: String = row
        .try_get("evaluation_payload_sha256")
        .map_err(map_sqlx_error)?;
    let projection: AttemptResult = decode_payload_parts(projection, checksum)?;
    crate::validate_attempt_result(projection).map_err(|_| {
        StoreError::Unavailable("completed receipt result evidence is invalid".to_string())
    })?;
    if receipt_attempt.result != Some(projection) {
        return Err(StoreError::Unavailable(
            "completed receipt result disagrees with evaluation projection".to_string(),
        ));
    }
    Ok(())
}

/// Verifies a graded accepted completion through the route-bound W4 database
/// capability. Its result is a safe projection, never a replacement for the
/// immutable receipt that remains this reader's canonical evidence source.
#[cfg(feature = "postgres")]
async fn validate_accepted_completed_graded_evaluation(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    binding: LearnerWorkRoutingBinding,
    attempt: QuestionAttemptId,
    receipt_attempt: &QuestionAttempt,
) -> Result<(), StoreError> {
    let receipt_result = accepted_completed_receipt_result(receipt_attempt)?;
    // ASVS 1.5.2 and 2.2.1-2.2.3: the SECURITY DEFINER function verifies the
    // complete tenant/course/assignment/attempt tuple and closed JSON source
    // before this typed decode. Actor entitlement is already established in
    // this same transaction by require_attempt_owner_for_read.
    let projection: Value = sqlx::query_scalar::<_, Option<Value>>(
        "SELECT evaluation_payload \
         FROM public.ple_read_accepted_submission_evaluation_v1($1, $2, $3, $4)",
    )
    .bind(tenant.as_uuid())
    .bind(binding.course.as_uuid())
    .bind(binding.assignment.as_uuid())
    .bind(attempt.as_uuid())
    .fetch_optional(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?
    .flatten()
    .ok_or_else(|| {
        StoreError::Unavailable("completed graded evaluation is unavailable".to_string())
    })?;
    let projection: AttemptResult = serde_json::from_value(projection).map_err(|_| {
        StoreError::Unavailable("completed graded evaluation is malformed".to_string())
    })?;
    crate::validate_attempt_result(projection).map_err(|_| {
        StoreError::Unavailable("completed graded evaluation is invalid".to_string())
    })?;
    if receipt_result != projection {
        return Err(StoreError::Unavailable(
            "completed receipt result disagrees with verified evaluation".to_string(),
        ));
    }
    Ok(())
}

#[cfg(feature = "postgres")]
fn accepted_completed_receipt_result(
    receipt_attempt: &QuestionAttempt,
) -> Result<AttemptResult, StoreError> {
    if receipt_attempt.status != AttemptStatus::Submitted {
        return Err(StoreError::Unavailable(
            "accepted completed receipt has an invalid terminal attempt".to_string(),
        ));
    }
    receipt_attempt.result.ok_or_else(|| {
        StoreError::Unavailable(
            "accepted completed receipt has an invalid terminal attempt".to_string(),
        )
    })
}

/// Reads one complete immutable, answer-free receipt aggregate.
#[cfg(feature = "postgres")]
pub(super) async fn load_submission_receipt_snapshot(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    attempt: QuestionAttemptId,
) -> Result<Option<crate::CompletedSubmissionReceipt>, StoreError> {
    let row = sqlx::query(
        "SELECT canonical_json_version, receipt_attempt_canonical_json, receipt_attempt_payload, \
                receipt_attempt_payload_sha256, run_canonical_json, run_payload, \
                run_payload_sha256, summary_canonical_json, summary_payload, \
                summary_payload_sha256, presentation_canonical_json, presentation_payload, \
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
    let version: i16 = row
        .try_get("canonical_json_version")
        .map_err(map_sqlx_error)?;
    let receipt_attempt: QuestionAttempt = decode_canonical_json_row_named(
        &row,
        "submission receipt attempt",
        "canonical_json_version",
        "receipt_attempt_canonical_json",
        "receipt_attempt_payload",
        "receipt_attempt_payload_sha256",
    )?;
    validate_receipt_attempt_snapshot(tenant, attempt, &receipt_attempt)?;
    let run: AssignmentRun = decode_canonical_json_row_named(
        &row,
        "submission receipt run",
        "canonical_json_version",
        "run_canonical_json",
        "run_payload",
        "run_payload_sha256",
    )?;
    let summary: StudentAssignmentSummary = decode_canonical_json_row_named(
        &row,
        "submission receipt summary",
        "canonical_json_version",
        "summary_canonical_json",
        "summary_payload",
        "summary_payload_sha256",
    )?;
    if run.tenant != tenant
        || receipt_attempt.run != run.id
        || summary.tenant != tenant
        || summary.enrollment != run.enrollment
    {
        return Err(StoreError::Unavailable(
            "receipt aggregate identities disagree".to_string(),
        ));
    }
    let presentation_required: bool = row
        .try_get("presentation_required")
        .map_err(map_sqlx_error)?;
    let presentation = decode_receipt_presentation(&row, version, presentation_required)?;
    let feedback = load_attempt_feedback(transaction, tenant, attempt).await?;
    // Receipt presentation must agree with the immutable issuance snapshot.
    // A mismatch is unavailable authority, never a current-catalog fallback.
    let issued = load_issued_presentation(transaction, tenant, &receipt_attempt).await?;
    if presentation != issued {
        return Err(StoreError::Unavailable(
            "submission receipt presentation does not match its issued snapshot".to_string(),
        ));
    }
    Ok(Some(crate::CompletedSubmissionReceipt {
        attempt: receipt_attempt,
        feedback,
        run,
        summary,
        presentation,
    }))
}

#[cfg(feature = "postgres")]
fn validate_receipt_attempt_snapshot(
    tenant: TenantId,
    attempt: QuestionAttemptId,
    receipt_attempt: &QuestionAttempt,
) -> Result<(), StoreError> {
    if receipt_attempt.tenant != tenant || receipt_attempt.id != attempt {
        return Err(StoreError::Unavailable(
            "receipt attempt snapshot identity disagrees with receipt row".to_string(),
        ));
    }
    if receipt_attempt.response.is_some()
        || !matches!(
            receipt_attempt.status,
            AttemptStatus::Submitted
                | AttemptStatus::AutoSubmitted
                | AttemptStatus::NeedsManualGrading
                | AttemptStatus::Exempt
        )
    {
        return Err(StoreError::Unavailable(
            "receipt attempt snapshot is not answer-free terminal evidence".to_string(),
        ));
    }
    Ok(())
}

/// Reads and validates the exact presentation frozen at issue time without
/// acquiring mutation authority. Receipt replay is a projection over the same
/// immutable issued evidence.
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
    version: i16,
    required: bool,
) -> Result<Option<ReceiptPresentationSnapshot>, StoreError> {
    let source: Option<String> = row
        .try_get("presentation_canonical_json")
        .map_err(map_sqlx_error)?;
    let payload: Option<Value> = row
        .try_get("presentation_payload")
        .map_err(map_sqlx_error)?;
    let checksum: Option<String> = row
        .try_get("presentation_payload_sha256")
        .map_err(map_sqlx_error)?;
    match (source, payload, checksum) {
        (None, None, None) if !required => Ok(None),
        (None, None, None) => Err(StoreError::Unavailable(
            "receipt requires a presentation snapshot but it is missing".to_string(),
        )),
        (Some(_), Some(_), Some(_)) if !required => Err(StoreError::Unavailable(
            "receipt includes a presentation snapshot for a non-presentation family".to_string(),
        )),
        (Some(source), Some(payload), Some(checksum)) => decode_canonical_json_parts(
            "submission receipt presentation",
            version,
            source,
            payload,
            checksum,
        )
        .map(Some),
        _ => Err(StoreError::Unavailable(
            "receipt presentation source, payload, and checksum disagree".to_string(),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        SubmissionReplayMetadata, accepted_pending_replay_from_metadata, automated_response_digest,
        validate_receipt_attempt_snapshot,
    };
    use crate::{SubmissionIdempotencyKey, SubmissionReceiptRead};
    use question_model::{
        ActivityTimestamp, AttemptProvenance, AttemptResult, AttemptStatus, AttemptTimerRecord,
        ImplementationVersion, IssuedAttemptCapabilityV1, ProblemId, QuestionAttempt,
        QuestionAttemptId, RunId, StudentResponse, TenantId, VersionId,
    };
    use uuid::Uuid;

    #[test]
    fn accepted_v2_replay_uses_answer_free_metadata_and_canonical_digest() {
        let response = StudentResponse::Numeric { value: 4.0 };
        let key = SubmissionIdempotencyKey::parse("accepted-replay").expect("key");
        let attempt = QuestionAttemptId::from_uuid(Uuid::from_u128(41));
        let metadata = SubmissionReplayMetadata {
            idempotency_key: key.as_str().to_string(),
            request_contract_version: 2,
            request_sha256: automated_response_digest(&response).expect("digest"),
        };

        assert!(matches!(
            accepted_pending_replay_from_metadata(&metadata, &response, &key, attempt),
            Ok(SubmissionReceiptRead::AcceptedPending(_))
        ));
        assert!(matches!(
            accepted_pending_replay_from_metadata(
                &metadata,
                &StudentResponse::Numeric { value: 5.0 },
                &key,
                attempt,
            ),
            Err(crate::StoreError::Conflict)
        ));
    }

    fn receipt_attempt(
        status: AttemptStatus,
        response: Option<StudentResponse>,
    ) -> QuestionAttempt {
        QuestionAttempt {
            id: QuestionAttemptId::from_uuid(Uuid::from_u128(41)),
            tenant: TenantId::from_uuid(Uuid::from_u128(42)),
            run: RunId::from_uuid(Uuid::from_u128(43)),
            problem: ProblemId::from_uuid(Uuid::from_u128(44)),
            question_version: VersionId::from_uuid(Uuid::from_u128(45)),
            assignment_position: 0,
            seed: 46,
            parameter_hash: "parameters".to_string(),
            response,
            status,
            result: Some(AttemptResult {
                correct: true,
                points_earned: 1.0,
                points_possible: 1.0,
            }),
            timer: AttemptTimerRecord {
                issued_at: ActivityTimestamp::from_unix_millis(10),
                deadline: None,
                submitted_at: Some(ActivityTimestamp::from_unix_millis(11)),
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
        }
    }

    #[test]
    fn receipt_attempt_snapshot_requires_exact_answer_free_terminal_identity() {
        let tenant = TenantId::from_uuid(Uuid::from_u128(42));
        let attempt = QuestionAttemptId::from_uuid(Uuid::from_u128(41));
        assert!(
            validate_receipt_attempt_snapshot(
                tenant,
                attempt,
                &receipt_attempt(AttemptStatus::Submitted, None),
            )
            .is_ok()
        );
        assert!(
            validate_receipt_attempt_snapshot(
                tenant,
                attempt,
                &receipt_attempt(AttemptStatus::InProgress, None),
            )
            .is_err()
        );
        assert!(
            validate_receipt_attempt_snapshot(
                tenant,
                attempt,
                &receipt_attempt(
                    AttemptStatus::Submitted,
                    Some(StudentResponse::Numeric { value: 4.0 })
                ),
            )
            .is_err()
        );
    }
}
