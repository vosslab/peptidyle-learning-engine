//! Parameterized reads of the existing immutable Blueprint history sequences.

use async_trait::async_trait;
use browser_api_contract::blueprint_course::BlueprintHistoryEntryView;
use question_model::{
    BlueprintAvailability, BlueprintCourseReference, BlueprintRevision, Timestamp,
};
use sqlx::Row;

use super::{blueprint_course::PostgresBlueprintCourseStore, connection::map_sqlx_error};
use crate::{
    BlueprintHistoryKind, BlueprintHistoryStore, Cursor, Page, PageRequest, SessionTokenHash,
    StoreError,
};

#[async_trait]
impl BlueprintHistoryStore for PostgresBlueprintCourseStore {
    async fn list_blueprint_history(
        &self,
        session: SessionTokenHash,
        reference: BlueprintCourseReference,
        kind: BlueprintHistoryKind,
        page: PageRequest,
    ) -> Result<Page<BlueprintHistoryEntryView>, StoreError> {
        let after = page.after.as_ref().map(Cursor::as_str);
        let mut transaction = self
            .begin_authenticated_application_transaction(session)
            .await?;
        // ASVS 1.2.4: the course, sequence, stable key and bounded size are bound values.
        let rows = sqlx::query("SELECT * FROM ple_api.list_blueprint_history($1, $2, $3, $4)")
            .bind(reference.as_string())
            .bind(kind.as_str())
            .bind(after)
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
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(Page { items, next_cursor })
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
                revision: BlueprintRevision::new(u64::try_from(number).map_err(|_| invalid())?)
                    .ok_or_else(invalid)?,
                saved_at: recorded_at,
            })
        }
        BlueprintHistoryKind::Metadata => {
            let availability: String = row.try_get("availability").map_err(map_sqlx_error)?;
            let availability = match availability.as_str() {
                "private" => BlueprintAvailability::Private,
                "public" => BlueprintAvailability::Public,
                "archived" => BlueprintAvailability::Archived,
                _ => return Err(invalid()),
            };
            Ok(BlueprintHistoryEntryView::MetadataChange {
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
