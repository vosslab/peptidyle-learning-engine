//! Parameterized reads of the existing immutable Blueprint history sequences.

use async_trait::async_trait;
use browser_api_contract::blueprint_course::BlueprintHistoryEntryView;
use question_model::{
    BlueprintAvailability, BlueprintCourseId, BlueprintEditNumber, BlueprintRevisionNumber,
    Timestamp,
};
use sqlx::Row;

use super::{blueprint_course::PostgresBlueprintCourseStore, connection::map_sqlx_error};
use crate::{
    BlueprintHistoryKind, BlueprintHistoryQuery, BlueprintHistoryResult, BlueprintHistoryStore,
    NumberedPage, NumberedPageRequest, PageNumber, SessionTokenHash, StoreError,
};

#[async_trait]
impl BlueprintHistoryStore for PostgresBlueprintCourseStore {
    async fn list_blueprint_history(
        &self,
        session: SessionTokenHash,
        blueprint_course_id: BlueprintCourseId,
        kind: BlueprintHistoryKind,
        query: BlueprintHistoryQuery,
        page: NumberedPageRequest,
    ) -> Result<BlueprintHistoryResult, StoreError> {
        let mut transaction = self
            .begin_authenticated_application_transaction(session)
            .await?;
        // ASVS 1.2.4: query, scope, and bounded numbered page are bound values.
        let rows = sqlx::query(
            "SELECT * FROM ple_api.list_blueprint_history($1, $2, $3, $4, $5, $6, $7, $8)",
        )
        .bind(blueprint_course_id.as_string())
        .bind(kind.as_str())
        .bind(&query.text)
        .bind(query.sort.as_str())
        .bind(if query.ascending {
            "ascending"
        } else {
            "descending"
        })
        .bind(&query.availability)
        .bind(i64::from(page.page_number.get()))
        .bind(i64::from(page.page_size.get()))
        .fetch_all(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let metadata = rows.first().ok_or_else(invalid)?;
        let total_count = u64::try_from(
            metadata
                .try_get::<i64, _>("total_count")
                .map_err(map_sqlx_error)?,
        )
        .map_err(|_| invalid())?;
        let page_number = u32::try_from(
            metadata
                .try_get::<i64, _>("resolved_page_number")
                .map_err(map_sqlx_error)?,
        )
        .ok()
        .and_then(|number| PageNumber::new(number).ok())
        .ok_or_else(invalid)?;
        let mut items = Vec::with_capacity(rows.len());
        for row in &rows {
            if row
                .try_get::<Option<i64>, _>("revision_number")
                .map_err(map_sqlx_error)?
                .is_some()
            {
                items.push(decode_entry(row, kind)?);
            }
        }
        let (mut active_count, mut archived_count) = (0, 0);
        if kind == BlueprintHistoryKind::Metadata {
            let facets =
                sqlx::query("SELECT * FROM ple_api.blueprint_history_availability_facets($1, $2)")
                    .bind(blueprint_course_id.as_string())
                    .bind(&query.text)
                    .fetch_all(&mut *transaction)
                    .await
                    .map_err(map_sqlx_error)?;
            for facet in facets {
                let count: i64 = facet.try_get("result_count").map_err(map_sqlx_error)?;
                let count = u64::try_from(count).map_err(|_| invalid())?;
                match facet
                    .try_get::<String, _>("availability")
                    .map_err(map_sqlx_error)?
                    .as_str()
                {
                    "active" => active_count = count,
                    "archived" => archived_count = count,
                    _ => return Err(invalid()),
                }
            }
        }
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(BlueprintHistoryResult {
            page: NumberedPage {
                items,
                page_number,
                page_size: page.page_size,
                total_count,
            },
            active_count,
            archived_count,
        })
    }
}

fn decode_entry(
    row: &sqlx::postgres::PgRow,
    kind: BlueprintHistoryKind,
) -> Result<BlueprintHistoryEntryView, StoreError> {
    let recorded_at =
        Timestamp::from_unix_millis(row.try_get("recorded_at_ms").map_err(map_sqlx_error)?);
    match kind {
        BlueprintHistoryKind::Revisions => {
            let number: i64 = row.try_get("revision_number").map_err(map_sqlx_error)?;
            Ok(BlueprintHistoryEntryView::SavedRevision {
                revision_number: BlueprintRevisionNumber::new(
                    u64::try_from(number).map_err(|_| invalid())?,
                )
                .ok_or_else(invalid)?,
                saved_at: recorded_at,
            })
        }
        BlueprintHistoryKind::Metadata => {
            let availability: String = row.try_get("availability").map_err(map_sqlx_error)?;
            let availability = match availability.as_str() {
                "active" => BlueprintAvailability::Active,
                "archived" => BlueprintAvailability::Archived,
                _ => return Err(invalid()),
            };
            Ok(BlueprintHistoryEntryView::MetadataChange {
                blueprint_edit_number: BlueprintEditNumber::from_edit_number(
                    row.try_get("revision_number").map_err(map_sqlx_error)?,
                ),
                classification: super::blueprint_course::decode_classification(row)?,
                theme: row
                    .try_get::<String, _>("theme_id")
                    .map_err(map_sqlx_error)?
                    .parse()
                    .map_err(|_| invalid())?,
                short_name: row.try_get("short_name").map_err(map_sqlx_error)?,
                long_name: row.try_get("long_name").map_err(map_sqlx_error)?,
                availability,
                recorded_at,
            })
        }
    }
}

fn invalid() -> StoreError {
    StoreError::InvalidRecord("Blueprint history record is invalid".into())
}
