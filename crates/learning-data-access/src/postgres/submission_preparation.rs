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
use crate::{LearnerWorkRoutingBinding, PreparedQuestionSubmission, SubmissionPreparation};

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
            SubmissionPreparation::Grade(Box::new(PreparedQuestionSubmission {
                attempt: prepared.attempt,
                issued_question_snapshot: prepared.issued_question_snapshot,
                presentation_binding: prepared.presentation_binding,
                presentation: prepared.presentation,
                grading_envelope: prepared.grading_envelope,
                flat_grading: prepared.flat_grading,
                webwork_grading: prepared.webwork_grading,
                issued_qti_grading: prepared.issued_qti_grading,
                webwork_replay: prepared.webwork_replay,
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
