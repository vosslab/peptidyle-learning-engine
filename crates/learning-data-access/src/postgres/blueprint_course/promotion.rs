//! Narrow session-authorized promotion persistence, separate from ordinary authoring.

use super::{PostgresBlueprintCourseStore, metadata_etag};
use crate::postgres::connection::map_sqlx_error;
use crate::{BlueprintPromotionStore, SessionTokenHash, StoreError};
use async_trait::async_trait;
use question_model::{BlueprintCourseReference, BlueprintMetadataEtag};
use sqlx::Row;

#[async_trait]
impl BlueprintPromotionStore for PostgresBlueprintCourseStore {
    async fn load_blueprint_promotion(
        &self,
        session: SessionTokenHash,
        reference: BlueprintCourseReference,
    ) -> Result<crate::blueprint_course::StoredBlueprintPromotion, StoreError> {
        let mut transaction = self
            .begin_authenticated_application_transaction(session)
            .await?;
        let row = sqlx::query("SELECT * FROM ple_api.load_blueprint_promotion($1)")
            .bind(reference.as_string())
            .fetch_optional(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?
            .ok_or(StoreError::NotFound)?;
        let result = decode_promotion(&row)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(result)
    }

    async fn set_blueprint_promotion(
        &self,
        session: SessionTokenHash,
        reference: BlueprintCourseReference,
        expected_metadata_etag: BlueprintMetadataEtag,
        promoted: bool,
    ) -> Result<crate::blueprint_course::StoredBlueprintPromotion, StoreError> {
        let mut transaction = self
            .begin_authenticated_application_transaction(session)
            .await?;
        // ASVS 1.2.4: the trusted SQL operation receives only bound typed values.
        let row = sqlx::query("SELECT * FROM ple_api.set_blueprint_promotion($1,$2,$3)")
            .bind(reference.as_string())
            .bind(expected_metadata_etag.into_uuid())
            .bind(promoted)
            .fetch_optional(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?
            .ok_or(StoreError::NotFound)?;
        let result = decode_promotion(&row)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(result)
    }
}

fn decode_promotion(
    row: &sqlx::postgres::PgRow,
) -> Result<crate::blueprint_course::StoredBlueprintPromotion, StoreError> {
    Ok(crate::blueprint_course::StoredBlueprintPromotion {
        promoted: row.try_get("promoted").map_err(map_sqlx_error)?,
        metadata_etag: metadata_etag(row.try_get("metadata_etag").map_err(map_sqlx_error)?),
    })
}
