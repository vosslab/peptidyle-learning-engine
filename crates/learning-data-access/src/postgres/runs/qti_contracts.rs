//! PostgreSQL decoding for immutable per-attempt QTI authority.

use objects::Sha256Digest;
use question_model::QuestionAttempt;
use sqlx::Row;

use crate::postgres::connection::map_sqlx_error;
use crate::{IssuedQuestionSnapshotV1, QtiGradingCapability, StoreError};

pub(in crate::postgres) fn qti_grading_capability_from_row(
    row: &sqlx::postgres::PgRow,
) -> Result<QtiGradingCapability, StoreError> {
    Ok(
        if row
            .try_get::<bool, _>("qti_grading_required")
            .map_err(map_sqlx_error)?
        {
            QtiGradingCapability::Required
        } else {
            QtiGradingCapability::NotApplicable
        },
    )
}

pub(in crate::postgres) fn decode_attempt_qti_grading(
    row: &sqlx::postgres::PgRow,
    snapshot: &IssuedQuestionSnapshotV1,
) -> Result<Option<crate::IssuedQtiGradingContractV1>, StoreError> {
    let payload: Option<Vec<u8>> = row
        .try_get("issued_qti_grading_payload")
        .map_err(map_sqlx_error)?;
    let checksum: Option<String> = row
        .try_get("issued_qti_grading_payload_sha256")
        .map_err(map_sqlx_error)?;
    match (qti_grading_capability_from_row(row)?, payload, checksum) {
        (QtiGradingCapability::NotApplicable, None, None) => Ok(None),
        (QtiGradingCapability::Required, Some(payload), Some(checksum)) => {
            if Sha256Digest::compute(&payload).to_string() != checksum {
                return Err(StoreError::Unavailable(
                    "stored issued QTI grading payload checksum mismatch".to_string(),
                ));
            }
            let question_model::QuestionSource::Qti { item_id, .. } = &snapshot.question().source
            else {
                return Err(StoreError::Unavailable(
                    "stored QTI grading payload has a non-QTI snapshot".to_string(),
                ));
            };
            let material = crate::QtiImportGradingPayload::new(payload).map_err(|_| {
                StoreError::Unavailable("stored issued QTI grading payload is invalid".to_string())
            })?;
            crate::IssuedQtiGradingContractV1::new(snapshot.question(), item_id.clone(), material)
                .map(Some)
                .map_err(|_| {
                    StoreError::Unavailable(
                        "stored issued QTI grading contract is invalid".to_string(),
                    )
                })
        }
        _ => Err(StoreError::Unavailable(
            "stored QTI grading capability and payload disagree".to_string(),
        )),
    }
}

pub(in crate::postgres) fn validate_attempt_qti_grading(
    row: &sqlx::postgres::PgRow,
    attempt: &QuestionAttempt,
    snapshot: &IssuedQuestionSnapshotV1,
) -> Result<Option<crate::IssuedQtiGradingContractV1>, StoreError> {
    let capability = qti_grading_capability_from_row(row)?;
    let expected = matches!(
        attempt.issued_capability,
        question_model::IssuedAttemptCapabilityV1::QtiPresentation
    );
    if capability.requires_contract() != expected {
        return Err(StoreError::Unavailable(
            "stored QTI grading capability disagrees with its checksummed attempt".to_string(),
        ));
    }
    let contract = decode_attempt_qti_grading(row, snapshot)?;
    crate::validate_issued_qti_grading(snapshot.question(), capability, contract.as_ref())?;
    Ok(contract)
}
