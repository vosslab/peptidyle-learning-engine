//! Decoding for one retained Assignment Attempt presentation source.

use sqlx::Row;

use super::{assignment_delivery::positive_i32, connection::map_sqlx_error};
use crate::{
    NativePleIssuanceSource, NativeWebworkIssuanceSource, ReadyQuestionAssetRendition, StoreError,
    StudentAssignmentAttemptPresentationEvidence, StudentAssignmentAttemptPresentationSource,
};

pub(super) fn presentation_evidence_from_row(
    row: &sqlx::postgres::PgRow,
) -> Result<StudentAssignmentAttemptPresentationEvidence, StoreError> {
    let question_id = row
        .try_get::<String, _>("question_id")
        .map_err(map_sqlx_error)?
        .parse()
        .map_err(|_| StoreError::InvalidRecord("Issued Question ID is invalid".to_string()))?;
    let revision_number = u32::try_from(
        row.try_get::<i32, _>("revision_number")
            .map_err(map_sqlx_error)?,
    )
    .map_err(|_| StoreError::InvalidRecord("Question Revision number is invalid".to_string()))?;
    let revision_number =
        question_model::QuestionRevisionNumber::new(revision_number).map_err(|_| {
            StoreError::InvalidRecord("Question Revision number is invalid".to_string())
        })?;
    let question_seed = row
        .try_get::<String, _>("question_seed")
        .map_err(map_sqlx_error)?
        .parse()
        .map_err(|_| StoreError::InvalidRecord("Question Seed is invalid".to_string()))?;
    let presentation_nonce = row.try_get("presentation_nonce").map_err(map_sqlx_error)?;
    let presentation_checksum = row
        .try_get("presentation_checksum")
        .map_err(map_sqlx_error)?;
    let presentation = row.try_get("presentation").map_err(map_sqlx_error)?;
    let response_item_bindings = decode_response_item_bindings(
        row.try_get("response_item_bindings")
            .map_err(map_sqlx_error)?,
    )?;
    Ok(StudentAssignmentAttemptPresentationEvidence {
        question_revision: question_model::QuestionRevisionReference {
            question_id,
            revision_number,
        },
        question_seed,
        presentation_nonce,
        presentation_checksum,
        presentation,
        response_item_bindings,
        question_asset_renditions: ready_question_asset_renditions(row)?,
    })
}

#[derive(serde::Deserialize)]
struct ResponseItemBindingRow {
    presentation_response_item_reference: String,
    response_item_reference: String,
}

fn decode_response_item_bindings(
    value: serde_json::Value,
) -> Result<Vec<question_model::presentation::DurableResponseItemBinding>, StoreError> {
    let rows: Vec<ResponseItemBindingRow> = serde_json::from_value(value).map_err(|_| {
        StoreError::InvalidRecord(
            "Question presentation response-item bindings are invalid".to_string(),
        )
    })?;
    rows.into_iter()
        .map(|row| {
            let presentation_response_item_reference =
                question_model::presentation::PresentationResponseItemReference::parse(
                    row.presentation_response_item_reference,
                )
                .map_err(|_| {
                    StoreError::InvalidRecord(
                        "Question presentation response-item bindings are invalid".to_string(),
                    )
                })?;
            if row.response_item_reference.trim().is_empty() {
                return Err(StoreError::InvalidRecord(
                    "Question presentation response-item bindings are invalid".to_string(),
                ));
            }
            Ok(question_model::presentation::DurableResponseItemBinding {
                presentation_response_item_reference,
                response_item_reference: question_model::response::ResponseItemReference::new(
                    row.response_item_reference,
                ),
            })
        })
        .collect()
}

pub(super) fn optional_presentation_evidence_from_row(
    row: &sqlx::postgres::PgRow,
) -> Result<Option<StudentAssignmentAttemptPresentationEvidence>, StoreError> {
    let presentation = row
        .try_get::<Option<serde_json::Value>, _>("presentation")
        .map_err(map_sqlx_error)?;
    if presentation.is_none() {
        return Ok(None);
    }
    presentation_evidence_from_row(row).map(Some)
}

pub(super) fn source_from_row(
    row: &sqlx::postgres::PgRow,
) -> Result<StudentAssignmentAttemptPresentationSource, StoreError> {
    let backend: String = row.try_get("backend").map_err(map_sqlx_error)?;
    let position = positive_i32(row, "issued_position", "Issued Question position")?;
    let question_attempt = row
        .try_get::<uuid::Uuid, _>("question_attempt_id")
        .map(question_model::QuestionAttemptId::from_uuid)
        .map_err(map_sqlx_error)?;
    let question_id = row
        .try_get::<String, _>("question_id")
        .map_err(map_sqlx_error)?
        .parse()
        .map_err(|_| StoreError::InvalidRecord("Issued Question ID is invalid".to_string()))?;
    let revision_number = u32::try_from(
        row.try_get::<i32, _>("revision_number")
            .map_err(map_sqlx_error)?,
    )
    .map_err(|_| StoreError::InvalidRecord("Question Revision number is invalid".to_string()))?;
    question_model::QuestionRevisionNumber::new(revision_number).map_err(|_| {
        StoreError::InvalidRecord("Question Revision number is invalid".to_string())
    })?;
    let source_object_id: uuid::Uuid = row.try_get("source_object_id").map_err(map_sqlx_error)?;
    let question_seed = row
        .try_get::<String, _>("question_seed")
        .map_err(map_sqlx_error)?
        .parse()
        .map_err(|_| StoreError::InvalidRecord("Question Seed is invalid".to_string()))?;
    let nonce: String = row.try_get("presentation_nonce").map_err(map_sqlx_error)?;
    let checksum: String = row
        .try_get("presentation_checksum")
        .map_err(map_sqlx_error)?;
    match backend.as_str() {
        "ple" => Ok(StudentAssignmentAttemptPresentationSource::Ple {
            question_attempt,
            source: NativePleIssuanceSource {
                issued_question_id: None,
                assignment_entry_id: String::new(),
                position,
                question_id,
                revision_number,
                source_object_id: source_object_id.to_string(),
                source_object_address: row
                    .try_get("source_object_address")
                    .map_err(map_sqlx_error)?,
                source_object_checksum: row
                    .try_get("source_object_checksum")
                    .map_err(map_sqlx_error)?,
                question_seed: Some(question_seed),
                presentation_nonce: Some(nonce),
                presentation_checksum: Some(checksum),
                retained_presentation: None,
                question_asset_renditions: ready_question_asset_renditions(row)?,
            },
        }),
        "webwork" => Ok(StudentAssignmentAttemptPresentationSource::Webwork {
            question_attempt,
            source: NativeWebworkIssuanceSource {
                issued_question_id: None,
                assignment_entry_id: String::new(),
                position,
                question_id,
                revision_number,
                source_object_id: source_object_id.to_string(),
                source_object_checksum: row
                    .try_get("source_object_checksum")
                    .map_err(map_sqlx_error)?,
                webwork_pg_path: row
                    .try_get::<Option<String>, _>("webwork_pg_path")
                    .map_err(map_sqlx_error)?
                    .ok_or_else(|| {
                        StoreError::InvalidRecord("WeBWorK path is missing".to_string())
                    })?,
                question_seed,
                retained_presentation: None,
                question_asset_renditions: ready_question_asset_renditions(row)?,
            },
            question_seed,
            presentation_nonce: nonce,
            presentation_checksum: checksum,
        }),
        _ => Err(StoreError::InvalidRecord(
            "Question backend is unavailable".to_string(),
        )),
    }
}

#[derive(serde::Deserialize)]
struct ReadyQuestionAssetRenditionRow {
    asset_id: String,
    question_asset_checksum: String,
    rendition_checksum: String,
    intrinsic_width: i32,
    intrinsic_height: i32,
}

pub(super) fn ready_question_asset_renditions(
    row: &sqlx::postgres::PgRow,
) -> Result<Vec<ReadyQuestionAssetRendition>, StoreError> {
    decode_retained_question_asset_renditions(
        row.try_get("question_asset_renditions")
            .map_err(map_sqlx_error)?,
    )
}

fn decode_retained_question_asset_renditions(
    value: serde_json::Value,
) -> Result<Vec<ReadyQuestionAssetRendition>, StoreError> {
    let values: Vec<ReadyQuestionAssetRenditionRow> =
        serde_json::from_value(value).map_err(|_| {
            StoreError::InvalidRecord("Question Asset renditions are invalid".to_string())
        })?;
    values
        .into_iter()
        .map(|value| {
            Ok(ReadyQuestionAssetRendition {
                question_asset: uuid::Uuid::parse_str(&value.asset_id)
                    .map(question_model::QuestionAssetId::from_uuid)
                    .map_err(|_| {
                        StoreError::InvalidRecord("Question Asset ID is invalid".to_string())
                    })?,
                question_asset_checksum: value.question_asset_checksum,
                rendition_checksum: value.rendition_checksum,
                intrinsic_width: u32::try_from(value.intrinsic_width).map_err(|_| {
                    StoreError::InvalidRecord("Question Asset width is invalid".to_string())
                })?,
                intrinsic_height: u32::try_from(value.intrinsic_height).map_err(|_| {
                    StoreError::InvalidRecord("Question Asset height is invalid".to_string())
                })?,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retained_renditions_ignore_later_current_publication_values() {
        let retained = serde_json::json!([{
            "asset_id": "00000000-0000-0000-0000-000000000001",
            "question_asset_checksum": "retained-question-asset",
            "rendition_checksum": "retained-rendition",
            "intrinsic_width": 640,
            "intrinsic_height": 480
        }]);
        let later_current_publication = serde_json::json!([{
            "asset_id": "00000000-0000-0000-0000-000000000002",
            "question_asset_checksum": "current-question-asset",
            "rendition_checksum": "current-rendition",
            "intrinsic_width": 1,
            "intrinsic_height": 1
        }]);

        let decoded = decode_retained_question_asset_renditions(retained).expect("retained data");
        assert_eq!(decoded.len(), 1);
        assert_eq!(
            decoded[0].question_asset_checksum,
            "retained-question-asset"
        );
        assert_eq!(decoded[0].rendition_checksum, "retained-rendition");
        assert_ne!(
            serde_json::to_value(&decoded[0].rendition_checksum).expect("checksum JSON"),
            later_current_publication[0]["rendition_checksum"]
        );
    }
}
