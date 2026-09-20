//! Current Question Image Asset renditions and Assessment presentation commit payloads.

use sqlx::{Postgres, Row, Transaction};

use super::super::connection::map_sqlx_error;
use crate::StoreError;
use crate::assessment_delivery::ReadyQuestionImageRendition;

const SELECT_READY_QUESTION_IMAGE_RENDITIONS_SQL: &str = "SELECT question_image_asset_id::text, question_image_checksum, rendition_checksum, intrinsic_width, intrinsic_height \
         FROM ple_api.select_ready_question_image_renditions($1, $2)";

pub(super) fn presentation_payloads<'a>(
    values: impl Iterator<
        Item = (
            &'a String,
            &'a question_model::QuestionReproduction,
            &'a serde_json::Value,
            &'a serde_json::Value,
            &'a String,
            &'a String,
            Option<&'a question_model::AuthorContentPresentation>,
            &'a [ReadyQuestionImageRendition],
            &'a str,
            Option<&'a String>,
            &'a [question_model::presentation::DurableResponseItemBinding],
        ),
    >,
) -> Result<serde_json::Value, StoreError> {
    values.map(|(issued_question_id, reproduction, details, presentation, nonce, checksum, author_content, assets, issued_capability, backend_document, response_item_bindings)| {
        let details: question_model::QuestionAttemptReproductionDetails = serde_json::from_value(details.clone())
            .map_err(|_| StoreError::InvalidRecord("Question reproduction details are invalid".to_string()))?;
        let mut payload = serde_json::json!({
            "question_attempt_id": crate::random_uuid::random_uuid_v4(|error| StoreError::Unavailable(format!("Question Attempt ID randomness unavailable: {error}")))?,
            "issued_question_id": issued_question_id,
            "backend_version": details.backend.version,
            "renderer_name": details.renderer_version.as_ref().map(|renderer| renderer.name.as_str()),
            "renderer_version": details.renderer_version.as_ref().map(|renderer| renderer.version.as_str()),
            "grader_name": details.grader.name,
            "grader_version": details.grader.version,
            "rendered_question_sha256": details.rendered_question_sha256,
            "issued_capability": issued_capability,
            "presentation_nonce": nonce,
            "presentation_checksum": checksum,
            "presentation": presentation,
            "response_item_bindings": response_item_bindings.iter().map(|binding| serde_json::json!({
                "presentation_response_item_id": binding.presentation_response_item_id.as_str(),
                "response_item_id": binding.response_item_id.as_str(),
            })).collect::<Vec<_>>(),
            "question_images": assets.iter().map(|asset| serde_json::json!({
                "question_image_asset_id": asset.question_image_asset_id.as_uuid(),
                "question_image_checksum": asset.question_image_checksum,
                "rendition_checksum": asset.rendition_checksum,
                "intrinsic_width": asset.intrinsic_width,
                "intrinsic_height": asset.intrinsic_height,
            })).collect::<Vec<_>>(),
        });
        if let Some(author_content) = author_content {
            payload["author_content"] = serde_json::to_value(author_content).map_err(|_| {
                StoreError::InvalidRecord("Author content evidence is invalid".to_string())
            })?;
        }
        match reproduction {
            question_model::QuestionReproduction::Static => {
                payload["question_seed"] = serde_json::Value::Null;
                payload["generated_parameter_sha256"] = serde_json::Value::Null;
            }
            question_model::QuestionReproduction::Seeded {
                question_seed,
                generated_parameter_sha256,
            } => {
                payload["question_seed"] = serde_json::json!(question_seed.value());
                payload["generated_parameter_sha256"] =
                    serde_json::Value::String(generated_parameter_sha256.clone());
            }
        }
        if let Some(backend_document) = backend_document {
            payload["backend_document"] = serde_json::Value::String(backend_document.clone());
        }
        Ok(payload)
    }).collect::<Result<Vec<_>, StoreError>>().map(serde_json::Value::Array)
}

pub(super) async fn current_ready_question_image_renditions(
    tx: &mut Transaction<'_, Postgres>,
    question_id: &str,
    revision_number: u32,
) -> Result<Vec<ReadyQuestionImageRendition>, StoreError> {
    let rows = sqlx::query(SELECT_READY_QUESTION_IMAGE_RENDITIONS_SQL)
        .bind(question_id)
        .bind(i32::try_from(revision_number).map_err(|_| {
            StoreError::InvalidRecord("Question Revision number is invalid".to_string())
        })?)
        .fetch_all(&mut **tx)
        .await
        .map_err(map_sqlx_error)?;
    rows.into_iter()
        .map(|row| {
            let question_image_asset_id = row
                .try_get::<String, _>("question_image_asset_id")
                .map_err(map_sqlx_error)?;
            Ok(ReadyQuestionImageRendition {
                question_image_asset_id: uuid::Uuid::parse_str(&question_image_asset_id)
                    .map(question_model::QuestionImageAssetId::from_uuid)
                    .map_err(|_| {
                        StoreError::InvalidRecord("Question Image Asset ID is invalid".to_string())
                    })?,
                question_image_checksum: row
                    .try_get("question_image_checksum")
                    .map_err(map_sqlx_error)?,
                rendition_checksum: row.try_get("rendition_checksum").map_err(map_sqlx_error)?,
                intrinsic_width: u32::try_from(
                    row.try_get::<i32, _>("intrinsic_width")
                        .map_err(map_sqlx_error)?,
                )
                .map_err(|_| {
                    StoreError::InvalidRecord("Question Image Asset width is invalid".to_string())
                })?,
                intrinsic_height: u32::try_from(
                    row.try_get::<i32, _>("intrinsic_height")
                        .map_err(map_sqlx_error)?,
                )
                .map_err(|_| {
                    StoreError::InvalidRecord("Question Image Asset height is invalid".to_string())
                })?,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::SELECT_READY_QUESTION_IMAGE_RENDITIONS_SQL;

    #[test]
    fn ready_renditions_query_names_the_canonical_api() {
        assert!(
            SELECT_READY_QUESTION_IMAGE_RENDITIONS_SQL
                .contains("ple_api.select_ready_question_image_renditions")
        );
        assert!(SELECT_READY_QUESTION_IMAGE_RENDITIONS_SQL.contains("question_image_checksum"));
    }
}
