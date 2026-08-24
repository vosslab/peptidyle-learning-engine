//! Broker-only persistence for answer-bearing issued execution contracts.

use sqlx::{Postgres, Transaction};

use crate::postgres::{connection::map_sqlx_error, row_decode::encode_payload};
use crate::{
    IssueQuestionAttemptCommand, PrefetchedPrivateExecutionV1, PrefetchedQuestionDescriptorV1,
    StoreError, TenantId,
};

/// Compares fresh trusted issue material with a persisted private child using
/// a broker-only equality projection. It cannot read an existing contract.
pub(super) async fn attempt_private_execution_matches(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    attempt: question_model::QuestionAttemptId,
    command: &IssueQuestionAttemptCommand,
) -> Result<bool, StoreError> {
    let private = PrefetchedPrivateExecutionV1 {
        flat_grading: command.flat_grading.clone(),
        webwork_replay: command.webwork_replay.clone(),
        webwork_grading: command.webwork_grading.clone(),
        qti_grading: command.qti_grading.clone(),
    };
    let (flat_payload, flat_sha256) = encode_optional(private.flat_grading.as_ref())?;
    let (webwork_payload, webwork_sha256) = encode_optional(private.webwork_grading.as_ref())?;
    let (replay_payload, replay_sha256) = encode_optional(private.webwork_replay.as_ref())?;
    let (qti_payload, qti_sha256) = qti_payload(&private)?;
    sqlx::query_scalar(
        "SELECT public.ple_write_issued_attempt_private_execution(\
        $1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13)",
    )
    .bind(tenant.as_uuid())
    .bind(attempt.as_uuid())
    .bind(command.flat_grading_capability.requires_contract())
    .bind(flat_payload)
    .bind(flat_sha256)
    .bind(command.webwork_grading_capability.requires_contract())
    .bind(webwork_payload)
    .bind(webwork_sha256)
    .bind(replay_payload)
    .bind(replay_sha256)
    .bind(command.qti_grading_capability.requires_contract())
    .bind(qti_payload)
    .bind(qti_sha256)
    .fetch_one(&mut **transaction)
    .await
    .map_err(map_sqlx_error)
}

/// Writes or verifies the private half of an answer-free prefetch descriptor.
/// The SQL capability owns private-table reads/writes and returns only equality.
pub(super) async fn prefetch_private_execution_matches(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    reservation: &PrefetchedQuestionDescriptorV1,
    private: &PrefetchedPrivateExecutionV1,
) -> Result<bool, StoreError> {
    let (flat_payload, flat_sha256) = encode_optional(private.flat_grading.as_ref())?;
    let (webwork_payload, webwork_sha256) = encode_optional(private.webwork_grading.as_ref())?;
    let (replay_payload, replay_sha256) = encode_optional(private.webwork_replay.as_ref())?;
    let (qti_payload, qti_sha256) = qti_payload(private)?;
    sqlx::query_scalar(
        "SELECT public.ple_write_prefetch_private_execution(\
        $1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15)",
    )
    .bind(tenant.as_uuid())
    .bind(reservation.run.as_uuid())
    .bind(reservation.predecessor.as_uuid())
    .bind(
        i32::try_from(reservation.assignment_position)
            .map_err(|_| StoreError::InvalidRecord("prefetch position is too large".to_string()))?,
    )
    .bind(reservation.flat_grading_capability.requires_contract())
    .bind(flat_payload)
    .bind(flat_sha256)
    .bind(reservation.webwork_grading_capability.requires_contract())
    .bind(webwork_payload)
    .bind(webwork_sha256)
    .bind(replay_payload)
    .bind(replay_sha256)
    .bind(reservation.qti_grading_capability.requires_contract())
    .bind(qti_payload)
    .bind(qti_sha256)
    .fetch_one(&mut **transaction)
    .await
    .map_err(map_sqlx_error)
}

fn encode_optional<T: serde::Serialize>(
    value: Option<&T>,
) -> Result<(Option<sqlx::types::Json<serde_json::Value>>, Option<String>), StoreError> {
    value
        .map(encode_payload)
        .transpose()
        .map(|value| match value {
            Some((payload, checksum)) => (Some(payload), Some(checksum)),
            None => (None, None),
        })
}

fn qti_payload(
    private: &PrefetchedPrivateExecutionV1,
) -> Result<(Option<Vec<u8>>, Option<String>), StoreError> {
    private
        .qti_grading
        .as_ref()
        .map(|contract| {
            let payload = contract.payload()?;
            Ok::<_, StoreError>((
                Some(payload.bytes().to_vec()),
                Some(payload.sha256().to_string()),
            ))
        })
        .transpose()
        .map(|value| value.unwrap_or((None, None)))
}
