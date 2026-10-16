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
    Cursor, Page, PageRequest, SessionTokenHash, StoreError,
};

#[async_trait]
impl BlueprintHistoryStore for PostgresBlueprintCourseStore {
    async fn list_blueprint_history(
        &self,
        session: SessionTokenHash,
        blueprint_course_id: BlueprintCourseId,
        kind: BlueprintHistoryKind,
        query: BlueprintHistoryQuery,
        page: PageRequest,
    ) -> Result<BlueprintHistoryResult, StoreError> {
        let after = page
            .after
            .as_ref()
            .map(|cursor| {
                serde_json::from_str::<(String, i64)>(cursor.as_str()).map_err(|_| invalid())
            })
            .transpose()?;
        let mut transaction = self
            .begin_authenticated_application_transaction(session)
            .await?;
        // ASVS 1.2.4: the course, sequence, stable key and bounded size are bound values.
        let rows = sqlx::query(
            "SELECT * FROM ple_api.list_blueprint_history($1, $2, $3, $4, $5, $6, $7, $8, $9)",
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
        .bind(after.as_ref().map(|key| key.0.as_str()))
        .bind(after.as_ref().map(|key| key.1))
        .bind(i32::from(page.size.get()))
        .fetch_all(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let limit = usize::from(page.size.get());
        let items = rows
            .iter()
            .take(limit)
            .map(|row| decode_entry(row, kind))
            .collect::<Result<Vec<_>, _>>()?;
        let next_cursor = if rows.len() > limit {
            let key: String = rows[limit - 1]
                .try_get("continuation_key")
                .map_err(map_sqlx_error)?;
            Some(Cursor::parse(key).map_err(|_| invalid())?)
        } else {
            None
        };
        let (mut public_count, mut archived_count) = (0, 0);
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
                    "public" => public_count = count,
                    "archived" => archived_count = count,
                    _ => return Err(invalid()),
                }
            }
        }
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(BlueprintHistoryResult {
            page: Page { items, next_cursor },
            public_count,
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
                "public" => BlueprintAvailability::Public,
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
