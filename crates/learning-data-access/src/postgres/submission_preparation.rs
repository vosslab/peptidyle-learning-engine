//! Broker-first bounded preparation for ordinary learner submission.

use question_model::{AttemptStatus, StudentResponse};
use sqlx::{Postgres, Row, Transaction};

use super::entitlement::{
    PreparedStudentAttemptWork, hydrate_assignment_from_witness,
    hydrate_prepared_student_attempt_work,
};
use super::learner_work_preparation::{
    StudentAttemptPreparationWitness, prepare_student_attempt_work,
};
use super::*;
use crate::{AuthorizedSubmissionIntent, LearnerWorkRoutingBinding, SubmissionPreparation};

#[async_trait::async_trait]
impl crate::SealedPrivateExecutionStore for crate::postgres::PostgresGraderStore {
    async fn prepare_sealed_private_execution(
        &self,
        context: TenantContext,
        actor: UserId,
        binding: LearnerWorkRoutingBinding,
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

/// Runs one short broker-first snapshot and releases its locks before grading.
pub(super) async fn prepare_question_submission(
    store: &PostgresStore,
    context: TenantContext,
    actor: UserId,
    binding: LearnerWorkRoutingBinding,
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
        Some(record) => SubmissionPreparation::Replay(Box::new(record)),
        None => {
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
) -> Result<Option<SubmissionRecord>, StoreError> {
    let row = sqlx::query(
        "SELECT idempotency_key, request_contract_version, request_sha256, \
                payload, payload_sha256 \
           FROM submission_idempotency WHERE tenant_id=$1 AND attempt_id=$2",
    )
    .bind(tenant.as_uuid())
    .bind(witness.attempt.as_uuid())
    .fetch_optional(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    let Some(row) = row else {
        return Ok(None);
    };
    let (_, response_checksum) = encode_payload(response)?;
    if row
        .try_get::<String, _>("idempotency_key")
        .map_err(map_sqlx_error)?
        != idempotency_key.as_str()
        || row
            .try_get::<i16, _>("request_contract_version")
            .map_err(map_sqlx_error)?
            != 0
        || row
            .try_get::<String, _>("request_sha256")
            .map_err(map_sqlx_error)?
            != response_checksum
    {
        return Err(StoreError::Conflict);
    }
    let submitted: QuestionAttempt = decode_payload_row(&row)?;
    if submitted.id != witness.attempt || submitted.tenant != tenant || submitted.run != witness.run
    {
        return Err(StoreError::Unavailable(
            "submission replay disagrees with learner-work witness".to_string(),
        ));
    }
    let feedback = load_attempt_feedback(transaction, tenant, submitted.id).await?;
    let (run, summary, presentation) =
        load_submission_receipt_snapshot(transaction, tenant, submitted.id)
            .await?
            .ok_or_else(|| {
                StoreError::Unavailable(
                    "submission receipt snapshot is missing; it cannot be reconstructed"
                        .to_string(),
                )
            })?;
    let enrollment = witness
        .source
        .existing_enrollment
        .ok_or_else(|| StoreError::Unavailable("replay enrollment is missing".to_string()))?;
    if run.id != witness.run || run.enrollment != enrollment || summary.enrollment != enrollment {
        return Err(StoreError::Unavailable(
            "submission receipt disagrees with learner-work witness".to_string(),
        ));
    }
    let assignment = hydrate_assignment_from_witness(transaction, &witness.source).await?;
    let disclosure = current_disclosure_input(
        transaction,
        tenant,
        &assignment,
        submitted.id,
        submitted.timer.submitted_at,
    )
    .await?;
    Ok(Some(SubmissionRecord {
        attempt: submitted,
        run,
        summary,
        feedback,
        presentation,
        disclosure,
    }))
}

pub(super) async fn prepare_bound_student_attempt(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    binding: LearnerWorkRoutingBinding,
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
) -> Result<Option<SubmissionRecord>, StoreError> {
    let row = sqlx::query(
        "SELECT idempotency_key, request_contract_version, request_sha256, \
                payload, payload_sha256 \
           FROM submission_idempotency WHERE tenant_id=$1 AND attempt_id=$2",
    )
    .bind(tenant.as_uuid())
    .bind(prepared.attempt.id.as_uuid())
    .fetch_optional(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    let Some(row) = row else {
        return Ok(None);
    };
    let (_, response_checksum) = encode_payload(response)?;
    if row
        .try_get::<String, _>("idempotency_key")
        .map_err(map_sqlx_error)?
        != idempotency_key.as_str()
        || row
            .try_get::<i16, _>("request_contract_version")
            .map_err(map_sqlx_error)?
            != 0
        || row
            .try_get::<String, _>("request_sha256")
            .map_err(map_sqlx_error)?
            != response_checksum
    {
        return Err(StoreError::Conflict);
    }
    let submitted: QuestionAttempt = decode_payload_row(&row)?;
    if submitted.id != prepared.attempt.id
        || submitted.tenant != tenant
        || submitted.run != prepared.run.id
    {
        return Err(StoreError::Unavailable(
            "submission replay disagrees with prepared attempt".to_string(),
        ));
    }
    let feedback = load_attempt_feedback(transaction, tenant, submitted.id).await?;
    let (run, summary, presentation) =
        load_submission_receipt_snapshot(transaction, tenant, submitted.id)
            .await?
            .ok_or_else(|| {
                StoreError::Unavailable(
                    "submission receipt snapshot is missing; it cannot be reconstructed"
                        .to_string(),
                )
            })?;
    if run.enrollment != prepared.enrollment.id
        || summary.enrollment != prepared.enrollment.id
        || presentation != prepared.presentation
    {
        return Err(StoreError::Unavailable(
            "submission receipt disagrees with prepared aggregate".to_string(),
        ));
    }
    let disclosure = current_disclosure_input(
        transaction,
        tenant,
        &prepared.assignment,
        submitted.id,
        submitted.timer.submitted_at,
    )
    .await?;
    Ok(Some(SubmissionRecord {
        attempt: submitted,
        run,
        summary,
        feedback,
        presentation,
        disclosure,
    }))
}
