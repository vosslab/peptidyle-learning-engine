//! Broker-first bounded preparation for ordinary learner submission.

use question_model::{AttemptStatus, IssuedAttemptCapabilityV1, StudentResponse};
use sqlx::{Postgres, Row, Transaction};

use super::entitlement::{PreparedStudentAttemptWork, hydrate_prepared_student_attempt_work};
use super::student_work_preparation::{
    StudentAttemptPreparationWitness, prepare_student_attempt_work,
};
use super::*;
use crate::{
    AuthorizedSubmissionIntent, StudentWorkRoutingBinding, SubmissionPreparation,
    SubmissionReceiptRead,
};

#[async_trait::async_trait]
impl crate::SealedPrivateExecutionStore for crate::postgres::PostgresGraderStore {
    async fn prepare_sealed_private_execution(
        &self,
        context: TenantContext,
        actor: UserId,
        binding: StudentWorkRoutingBinding,
        intent: AuthorizedSubmissionIntent,
        _response: &StudentResponse,
        _idempotency_key: &SubmissionIdempotencyKey,
    ) -> Result<crate::SealedPrivateExecutionPreparation, StoreError> {
        let mut transaction = self.begin_sealed_reader_tenant(context).await?;
        let row = sqlx::query(
            "SELECT * FROM public.ple_prepare_sealed_private_execution($1,$2,$3,$4,$5)",
        )
        .bind(context.tenant_id().as_uuid())
        .bind(binding.course.as_uuid())
        .bind(binding.assignment.as_uuid())
        .bind(actor.as_uuid())
        .bind(intent.attempt.id.as_uuid())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?
        .ok_or(StoreError::NotFound)?;
        let returned_attempt: uuid::Uuid = row.try_get("attempt_id").map_err(map_sqlx_error)?;
        if returned_attempt != intent.attempt.id.as_uuid() {
            return Err(StoreError::Unavailable(
                "sealed private execution disagrees with its authorized intent".to_string(),
            ));
        }
        let flat_grading = decode_private_json_contract::<crate::IssuedFlatGradingContract>(
            &row,
            "flat_required",
            "flat_payload",
            "flat_payload_sha256",
        )?;
        let webwork_grading = decode_private_json_contract::<crate::IssuedWebworkGradingContract>(
            &row,
            "webwork_required",
            "webwork_payload",
            "webwork_payload_sha256",
        )?;
        let webwork_replay_mapping = decode_private_json_contract::<crate::WebworkReplayMappingV1>(
            &row,
            "webwork_required",
            "webwork_replay_payload",
            "webwork_replay_payload_sha256",
        )?;
        let qti_required: bool = row.try_get("qti_required").map_err(map_sqlx_error)?;
        let qti_payload: Option<Vec<u8>> = row.try_get("qti_payload").map_err(map_sqlx_error)?;
        let qti_sha256: Option<String> =
            row.try_get("qti_payload_sha256").map_err(map_sqlx_error)?;
        let issued_qti_grading = decode_sealed_qti_contract(
            &intent.issued_question_snapshot,
            qti_required,
            qti_payload,
            qti_sha256,
        )?;
        let webwork_capability = if matches!(
            intent.attempt.issued_capability,
            question_model::IssuedAttemptCapabilityV1::WebworkPresentation
        ) {
            crate::WebworkGradingCapability::Required
        } else {
            crate::WebworkGradingCapability::NotApplicable
        };
        crate::validate_issued_webwork_grading(
            intent.issued_question_snapshot.question(),
            webwork_capability,
            webwork_grading.as_ref(),
        )?;
        crate::validate_issued_webwork_replay(webwork_capability, webwork_replay_mapping.as_ref())?;
        let webwork_replay = webwork_replay_mapping
            .map(|mapping| {
                let binding = intent.presentation_binding.ok_or_else(|| {
                    StoreError::Unavailable(
                        "sealed WebWork execution lacks its issued presentation binding"
                            .to_string(),
                    )
                })?;
                crate::webwork_replay_state_from_issue(
                    intent.attempt.problem,
                    intent.attempt.question_version,
                    intent.attempt.seed,
                    &intent.attempt.provenance,
                    binding,
                    mapping,
                )
            })
            .transpose()?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(crate::SealedPrivateExecutionPreparation::Grade(Box::new(
            crate::PreparedQuestionSubmission {
                attempt: intent.attempt,
                issued_question_snapshot: intent.issued_question_snapshot,
                presentation_binding: intent.presentation_binding,
                presentation: intent.presentation,
                grading_envelope: intent.grading_envelope,
                flat_grading,
                webwork_grading,
                issued_qti_grading,
                webwork_replay,
            },
        )))
    }
}

fn decode_private_json_contract<T: serde::de::DeserializeOwned>(
    row: &sqlx::postgres::PgRow,
    required_column: &str,
    payload_column: &str,
    checksum_column: &str,
) -> Result<Option<T>, StoreError> {
    let required: bool = row.try_get(required_column).map_err(map_sqlx_error)?;
    let payload: Option<serde_json::Value> = row.try_get(payload_column).map_err(map_sqlx_error)?;
    let checksum: Option<String> = row.try_get(checksum_column).map_err(map_sqlx_error)?;
    match (required, payload, checksum) {
        (false, None, None) => Ok(None),
        (true, Some(payload), Some(checksum)) => {
            let bytes = serde_json::to_vec(&payload).map_err(|_| {
                StoreError::Unavailable("sealed private execution payload is invalid".to_string())
            })?;
            if objects::Sha256Digest::compute(&bytes).to_string() != checksum {
                return Err(StoreError::Unavailable(
                    "sealed private execution checksum mismatch".to_string(),
                ));
            }
            serde_json::from_value(payload).map(Some).map_err(|_| {
                StoreError::Unavailable("sealed private execution payload is invalid".to_string())
            })
        }
        _ => Err(StoreError::Unavailable(
            "sealed private execution shape is invalid".to_string(),
        )),
    }
}

fn decode_sealed_qti_contract(
    snapshot: &crate::IssuedQuestionSnapshotV1,
    required: bool,
    payload: Option<Vec<u8>>,
    checksum: Option<String>,
) -> Result<Option<crate::IssuedQtiGradingContractV1>, StoreError> {
    match (required, payload, checksum) {
        (false, None, None) => Ok(None),
        (true, Some(payload), Some(checksum)) => {
            if objects::Sha256Digest::compute(&payload).to_string() != checksum {
                return Err(StoreError::Unavailable(
                    "sealed QTI execution checksum mismatch".to_string(),
                ));
            }
            let question_model::QuestionSource::Qti { item_id, .. } = &snapshot.question().source
            else {
                return Err(StoreError::Unavailable(
                    "sealed QTI execution has a non-QTI snapshot".to_string(),
                ));
            };
            let payload = crate::QtiImportGradingPayload::new(payload).map_err(|_| {
                StoreError::Unavailable("sealed QTI execution payload is invalid".to_string())
            })?;
            crate::IssuedQtiGradingContractV1::new(snapshot.question(), item_id.clone(), payload)
                .map(Some)
                .map_err(|_| {
                    StoreError::Unavailable("sealed QTI execution contract is invalid".to_string())
                })
        }
        _ => Err(StoreError::Unavailable(
            "sealed QTI execution shape is invalid".to_string(),
        )),
    }
}

/// Rebuilds one exact grading descriptor from the worker-only execution-load
/// broker projection. The broker has already fenced tenant, lease, job,
/// accepted input, and issued rows; this decoder repeats all identity and
/// checksum checks before a grader can receive the resulting private input.
///
/// The function intentionally consumes only issued evidence projected by the
/// broker. It never consults the mutable catalog or learner authorization
/// state, so a retry remains bound to the originally issued question.
pub(super) fn decode_prepared_accepted_submission_execution(
    row: &sqlx::postgres::PgRow,
) -> Result<crate::PreparedQuestionSubmission, StoreError> {
    let attempt_payload: serde_json::Value =
        row.try_get("attempt_payload").map_err(map_sqlx_error)?;
    let attempt_checksum: String = row
        .try_get("attempt_payload_sha256")
        .map_err(map_sqlx_error)?;
    let attempt: question_model::QuestionAttempt =
        super::row_decode::decode_payload_parts(attempt_payload, attempt_checksum)?;
    let accepted_tenant: uuid::Uuid = row.try_get("accepted_tenant_id").map_err(map_sqlx_error)?;
    let accepted_attempt: uuid::Uuid =
        row.try_get("accepted_attempt_id").map_err(map_sqlx_error)?;
    if attempt.tenant.as_uuid() != accepted_tenant
        || attempt.id.as_uuid() != accepted_attempt
        || attempt.status != AttemptStatus::InProgress
        || attempt.response.is_some()
        || attempt.result.is_some()
    {
        return Err(StoreError::Unavailable(
            "accepted execution attempt disagrees with issued evidence".to_string(),
        ));
    }

    let issued_question_snapshot =
        super::runs::attempt_issuance::decode_issued_question_snapshot(row)?;
    issued_question_snapshot.validate_for_attempt(attempt.problem, attempt.question_version)?;
    issued_question_snapshot.validate_native_provenance(&attempt.provenance.asset_objects)?;

    let presentation_capability =
        super::runs::attempt_issuance::presentation_capability_from_row(row)?;
    let presentation_binding = super::row_decode::decode_presentation_binding_row(row)?;
    let presentation = super::runs::attempt_issuance::decode_attempt_presentation_snapshot(
        row,
        presentation_capability,
    )?;
    let grading_envelope = super::runs::attempt_issuance::decode_attempt_grading_envelope(
        row,
        presentation_capability,
    )?;
    let flat_grading = decode_private_json_contract::<crate::IssuedFlatGradingContract>(
        row,
        "flat_required",
        "flat_payload",
        "flat_payload_sha256",
    )?;
    let webwork_grading = decode_private_json_contract::<crate::IssuedWebworkGradingContract>(
        row,
        "webwork_required",
        "webwork_payload",
        "webwork_payload_sha256",
    )?;
    let webwork_replay_mapping = decode_private_json_contract::<crate::WebworkReplayMappingV1>(
        row,
        "webwork_required",
        "webwork_replay_payload",
        "webwork_replay_payload_sha256",
    )?;
    let qti_required: bool = row.try_get("qti_required").map_err(map_sqlx_error)?;
    let qti_payload: Option<Vec<u8>> = row.try_get("qti_payload").map_err(map_sqlx_error)?;
    let qti_checksum: Option<String> = row.try_get("qti_payload_sha256").map_err(map_sqlx_error)?;
    let issued_qti_grading = decode_sealed_qti_contract(
        &issued_question_snapshot,
        qti_required,
        qti_payload,
        qti_checksum,
    )?;

    let (flat_capability, webwork_capability, qti_capability) = match attempt.issued_capability {
        IssuedAttemptCapabilityV1::FlatPresentation => (
            crate::FlatGradingCapability::Required,
            crate::WebworkGradingCapability::NotApplicable,
            crate::QtiGradingCapability::NotApplicable,
        ),
        IssuedAttemptCapabilityV1::WebworkPresentation => (
            crate::FlatGradingCapability::NotApplicable,
            crate::WebworkGradingCapability::Required,
            crate::QtiGradingCapability::NotApplicable,
        ),
        IssuedAttemptCapabilityV1::QtiPresentation => (
            crate::FlatGradingCapability::NotApplicable,
            crate::WebworkGradingCapability::NotApplicable,
            crate::QtiGradingCapability::Required,
        ),
        IssuedAttemptCapabilityV1::PresentationEnvelope
        | IssuedAttemptCapabilityV1::NotApplicable => (
            crate::FlatGradingCapability::NotApplicable,
            crate::WebworkGradingCapability::NotApplicable,
            crate::QtiGradingCapability::NotApplicable,
        ),
    };
    let expected_capability = crate::issued_attempt_capability_from_issue(
        presentation_capability,
        flat_capability,
        webwork_capability,
        qti_capability,
    )?;
    if attempt.issued_capability != expected_capability {
        return Err(StoreError::Unavailable(
            "accepted execution capability disagrees with issued attempt".to_string(),
        ));
    }
    let presentation = crate::validate_issued_presentation(
        presentation_capability,
        &attempt,
        presentation_binding,
        presentation.as_ref(),
        grading_envelope.as_ref(),
    )?;
    issued_question_snapshot.validate_for_issuance_context(
        flat_capability,
        webwork_capability,
        qti_capability,
        presentation.as_ref(),
    )?;
    crate::validate_issued_flat_grading(
        issued_question_snapshot.question(),
        presentation_capability,
        flat_capability,
        flat_grading.as_ref(),
    )?;
    crate::validate_issued_webwork_grading(
        issued_question_snapshot.question(),
        webwork_capability,
        webwork_grading.as_ref(),
    )?;
    crate::validate_issued_qti_grading(
        issued_question_snapshot.question(),
        qti_capability,
        issued_qti_grading.as_ref(),
    )?;
    crate::validate_issued_webwork_replay(webwork_capability, webwork_replay_mapping.as_ref())?;
    let webwork_replay = webwork_replay_mapping
        .map(|mapping| {
            let binding = presentation_binding.ok_or_else(|| {
                StoreError::Unavailable(
                    "accepted WebWork execution lacks an issued presentation binding".to_string(),
                )
            })?;
            crate::webwork_replay_state_from_issue(
                attempt.problem,
                attempt.question_version,
                attempt.seed,
                &attempt.provenance,
                binding,
                mapping,
            )
        })
        .transpose()?;

    Ok(crate::PreparedQuestionSubmission {
        attempt,
        issued_question_snapshot,
        presentation_binding,
        presentation,
        grading_envelope,
        flat_grading,
        webwork_grading,
        issued_qti_grading,
        webwork_replay,
    })
}

/// Runs one short broker-first snapshot and releases its locks before grading.
pub(super) async fn prepare_question_submission(
    store: &PostgresStore,
    context: TenantContext,
    actor: UserId,
    binding: StudentWorkRoutingBinding,
    attempt: QuestionAttemptId,
    response: &StudentResponse,
    idempotency_key: &SubmissionIdempotencyKey,
) -> Result<SubmissionPreparation, StoreError> {
    let tenant = context.tenant_id();
    let mut transaction = store.begin_tenant(context).await?;
    // ASVS 2.3.1/2.3.3 and 8.2.2: the exact broker capability is the first
    // protected operation; every hydrated identifier must match its witness.
    let witness =
        prepare_student_attempt_work(&mut transaction, tenant, binding, actor, attempt).await?;
    let result = match prepared_submission_replay_for_witness(
        &mut transaction,
        tenant,
        response,
        idempotency_key,
        &witness,
    )
    .await?
    {
        SubmissionReceiptRead::Completed(record) => SubmissionPreparation::Replay(record),
        SubmissionReceiptRead::AcceptedPending(pending) => {
            SubmissionPreparation::AcceptedPending(pending)
        }
        SubmissionReceiptRead::Missing => {
            let prepared =
                hydrate_prepared_student_attempt_work(&mut transaction, &witness).await?;
            if prepared.attempt.status != AttemptStatus::InProgress
                || prepared.run.completed_at.is_some()
                || prepared.run.score.is_some()
            {
                return Err(StoreError::Conflict);
            }
            SubmissionPreparation::FirstEffect(Box::new(AuthorizedSubmissionIntent {
                attempt: prepared.attempt,
                issued_question_snapshot: prepared.issued_question_snapshot,
                presentation_binding: prepared.presentation_binding,
                presentation: prepared.presentation,
                grading_envelope: prepared.grading_envelope,
            }))
        }
    };
    transaction.commit().await.map_err(map_sqlx_error)?;
    Ok(result)
}

/// Checks an exact receipt after the broker proved current authority but
/// before decoding the first-effect-only issued source snapshot.  This keeps
/// retries recoverable if a later catalog withdrawal, renderer outage, or
/// deliberately removed snapshot artifact cannot serve a new grade.
async fn prepared_submission_replay_for_witness(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    response: &StudentResponse,
    idempotency_key: &SubmissionIdempotencyKey,
    witness: &StudentAttemptPreparationWitness,
) -> Result<SubmissionReceiptRead, StoreError> {
    let Some(metadata) = super::submission_receipts::load_submission_replay_metadata(
        transaction,
        tenant,
        witness.attempt,
    )
    .await?
    else {
        return Ok(SubmissionReceiptRead::Missing);
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
            // ASVS 1.5.2-1.5.3, 2.2.1-2.2.3, and 2.3.1-2.3.3: validate
            // the closed replay identity before selecting the durable receipt
            // state. An exact retry therefore converges on completion when
            // the immutable snapshot exists, while incomplete evaluation
            // remains the closed accepted-pending projection.
            super::submission_receipts::validate_accepted_replay_metadata(
                &metadata,
                response,
                idempotency_key,
            )?;
        }
        _ => return Err(StoreError::Conflict),
    }
    let receipt =
        super::submission_receipts::load_submission_record(transaction, tenant, witness.attempt)
            .await?;
    let SubmissionReceiptRead::Completed(record) = receipt else {
        return Ok(receipt);
    };
    let enrollment = witness
        .source
        .existing_enrollment
        .ok_or_else(|| StoreError::Unavailable("replay enrollment is missing".to_string()))?;
    if record.attempt.id != witness.attempt
        || record.attempt.tenant != tenant
        || record.attempt.run != witness.run
        || record.run.id != witness.run
        || record.run.enrollment != enrollment
        || record.summary.enrollment != enrollment
    {
        return Err(StoreError::Unavailable(
            "submission receipt disagrees with learner-work witness".to_string(),
        ));
    }
    Ok(SubmissionReceiptRead::Completed(record))
}

pub(super) async fn prepare_bound_student_attempt(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    binding: StudentWorkRoutingBinding,
    actor: UserId,
    attempt: QuestionAttemptId,
) -> Result<PreparedStudentAttemptWork, StoreError> {
    let witness =
        prepare_student_attempt_work(transaction, tenant, binding, actor, attempt).await?;
    hydrate_prepared_student_attempt_work(transaction, &witness).await
}

pub(super) async fn prepared_submission_replay(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    response: &StudentResponse,
    idempotency_key: &SubmissionIdempotencyKey,
    prepared: &PreparedStudentAttemptWork,
) -> Result<SubmissionReceiptRead, StoreError> {
    let Some(metadata) = super::submission_receipts::load_submission_replay_metadata(
        transaction,
        tenant,
        prepared.attempt.id,
    )
    .await?
    else {
        return Ok(SubmissionReceiptRead::Missing);
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
            // ASVS 1.5.2-1.5.3, 2.2.1-2.2.3, and 2.3.1-2.3.3: the durable
            // receipt is the sole pending-versus-completed authority after
            // the exact typed request identity is verified.
            super::submission_receipts::validate_accepted_replay_metadata(
                &metadata,
                response,
                idempotency_key,
            )?;
        }
        _ => return Err(StoreError::Conflict),
    }
    let receipt = super::submission_receipts::load_submission_record(
        transaction,
        tenant,
        prepared.attempt.id,
    )
    .await?;
    let SubmissionReceiptRead::Completed(record) = receipt else {
        return Ok(receipt);
    };
    if record.attempt.id != prepared.attempt.id
        || record.attempt.tenant != tenant
        || record.attempt.run != prepared.run.id
        || record.run.enrollment != prepared.enrollment.id
        || record.summary.enrollment != prepared.enrollment.id
        || record.presentation != prepared.presentation
    {
        return Err(StoreError::Unavailable(
            "submission receipt disagrees with prepared aggregate".to_string(),
        ));
    }
    Ok(SubmissionReceiptRead::Completed(record))
}
