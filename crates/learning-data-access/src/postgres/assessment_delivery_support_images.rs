//! Exact image retention after server-owned support disclosure.

use question_model::AssessmentAttemptId;

use super::{assessment_delivery::PostgresLiveAssessmentDeliveryStore, connection::map_sqlx_error};
use crate::{SessionTokenHash, StoreError, StudentAssessmentAttemptSupportImages};

pub(super) async fn register(
    store: &PostgresLiveAssessmentDeliveryStore,
    token: SessionTokenHash,
    assessment_attempt: AssessmentAttemptId,
    disclosed: &[StudentAssessmentAttemptSupportImages],
) -> Result<(), StoreError> {
    if disclosed.is_empty() {
        return Ok(());
    }
    // One history response has one retention transaction: a later item error
    // rolls back earlier grants instead of authorizing an unreturned response.
    let mut tx = store.begin(token).await?;
    for item in disclosed {
        let position = i32::try_from(item.position)
            .ok()
            .filter(|position| *position > 0)
            .ok_or_else(|| StoreError::InvalidRecord("History position is invalid".to_string()))?;
        let revision_number = i32::try_from(item.revision.revision_number.get()).map_err(|_| {
            StoreError::InvalidRecord("Question Revision number is invalid".to_string())
        })?;
        let bindings = serde_json::Value::Array(item.images.iter().map(|image| {
        serde_json::json!({
            "source_relative_name": image.source_relative_name,
            "question_image_asset_id": image.question_image_asset_tuple.question_image_asset_id.as_uuid(),
            "question_image_checksum": image.question_image_asset_tuple.checksum,
        })
    }).collect());
        // ASVS 8.2.2/8.2.3: SQL repeats owner and exact source proofs; only the
        // server's projected support fields supply this internal operation.
        sqlx::query(
            "SELECT ple_api.register_student_assessment_attempt_support_images($1, $2, $3, $4, $5)",
        )
        .bind(assessment_attempt.as_uuid())
        .bind(position)
        .bind(item.revision.published_question_id.as_str())
        .bind(revision_number)
        .bind(bindings)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_error)?;
    }
    tx.commit().await.map_err(map_sqlx_error)
}
