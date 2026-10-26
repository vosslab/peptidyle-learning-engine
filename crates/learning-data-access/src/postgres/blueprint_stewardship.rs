//! PostgreSQL implementation of the self-only Blueprint Course stewardship boundary.

use crate::postgres::connection::begin_authenticated_transaction;
use async_trait::async_trait;
use question_model::{BlueprintCourseId, Timestamp};
use sqlx::{Postgres, Row, Transaction};
use uuid::Uuid;

use super::{Pool, connection::map_sqlx_error};
use crate::{
    BlueprintCourseStarProjection, BlueprintCourseStarredInstructor, BlueprintCourseWatchDirection,
    BlueprintCourseWatchEvent, BlueprintCourseWatchEventFacet, BlueprintCourseWatchEventKind,
    BlueprintCourseWatchEventsPage, BlueprintCourseWatchEventsQuery,
    BlueprintCourseWatchProjection, BlueprintStewardshipStore, NumberedPage, PageNumber,
    SessionTokenHash, StoreError,
};

/// PostgreSQL Store for one active Instructor's Blueprint Course stewardship.
#[derive(Clone)]
pub struct PostgresBlueprintStewardshipStore {
    pool: Pool,
}

impl PostgresBlueprintStewardshipStore {
    /// Binds the attested application pool to the closed stewardship procedures.
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }

    async fn read_star_in(
        tx: &mut Transaction<'_, Postgres>,
        blueprint_course_id: BlueprintCourseId,
    ) -> Result<BlueprintCourseStarProjection, StoreError> {
        let row = sqlx::query("SELECT * FROM ple_api.read_current_blueprint_course_star($1)")
            .bind(blueprint_course_id.as_string())
            .fetch_optional(&mut **tx)
            .await
            .map_err(map_sqlx_error)?
            .ok_or(StoreError::NotFound)?;
        let star_count: i64 = row.try_get("star_count").map_err(map_sqlx_error)?;
        Ok(BlueprintCourseStarProjection {
            viewer_has_starred: row.try_get("viewer_has_starred").map_err(map_sqlx_error)?,
            star_count: u64::try_from(star_count)
                .map_err(|_| StoreError::InvalidRecord("Blueprint Course Star count".to_owned()))?,
        })
    }

    async fn read_watch_in(
        tx: &mut Transaction<'_, Postgres>,
        blueprint_course_id: BlueprintCourseId,
    ) -> Result<BlueprintCourseWatchProjection, StoreError> {
        let row = sqlx::query("SELECT * FROM ple_api.read_current_blueprint_course_watch($1)")
            .bind(blueprint_course_id.as_string())
            .fetch_optional(&mut **tx)
            .await
            .map_err(map_sqlx_error)?
            .ok_or(StoreError::NotFound)?;
        Ok(BlueprintCourseWatchProjection {
            watching: row.try_get("watching").map_err(map_sqlx_error)?,
        })
    }

    async fn read_starred_instructors_in(
        tx: &mut Transaction<'_, Postgres>,
        blueprint_course_id: BlueprintCourseId,
    ) -> Result<Vec<BlueprintCourseStarredInstructor>, StoreError> {
        // ASVS 8.2.1--8.3.1: the SQL capability derives the viewer from the
        // installed session; it accepts no Account ID or claimed role.
        let rows = sqlx::query(
            "SELECT display_name FROM ple_api.read_current_blueprint_course_starred_instructors($1)",
        )
        .bind(blueprint_course_id.as_string())
        .fetch_all(&mut **tx)
        .await
        .map_err(map_sqlx_error)?;
        rows.into_iter()
            .map(|row| validated_display_name(row.try_get("display_name").map_err(map_sqlx_error)?))
            .collect()
    }
}

fn validated_display_name(value: String) -> Result<BlueprintCourseStarredInstructor, StoreError> {
    if value != value.trim()
        || value.is_empty()
        || value.chars().count() > 200
        || value.chars().any(char::is_control)
    {
        return Err(StoreError::InvalidRecord(
            "Blueprint Course Star Instructor display identity".to_owned(),
        ));
    }
    Ok(BlueprintCourseStarredInstructor {
        display_name: value,
    })
}

#[async_trait]
impl BlueprintStewardshipStore for PostgresBlueprintStewardshipStore {
    async fn blueprint_course_star_projection(
        &self,
        session_token_hash: SessionTokenHash,
        blueprint_course_id: BlueprintCourseId,
    ) -> Result<BlueprintCourseStarProjection, StoreError> {
        let mut tx = begin_authenticated_transaction(&self.pool, session_token_hash).await?;
        let projection = Self::read_star_in(&mut tx, blueprint_course_id).await?;
        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(projection)
    }

    async fn set_current_blueprint_course_star_projection(
        &self,
        session_token_hash: SessionTokenHash,
        blueprint_course_id: BlueprintCourseId,
        starred: bool,
    ) -> Result<BlueprintCourseStarProjection, StoreError> {
        let mut tx = begin_authenticated_transaction(&self.pool, session_token_hash).await?;
        sqlx::query("SELECT ple_api.set_current_blueprint_course_star($1, $2)")
            .bind(blueprint_course_id.as_string())
            .bind(starred)
            .execute(&mut *tx)
            .await
            .map_err(map_sqlx_error)?;
        let projection = Self::read_star_in(&mut tx, blueprint_course_id).await?;
        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(projection)
    }

    async fn blueprint_course_starred_instructors(
        &self,
        session_token_hash: SessionTokenHash,
        blueprint_course_id: BlueprintCourseId,
    ) -> Result<Vec<BlueprintCourseStarredInstructor>, StoreError> {
        let mut tx = begin_authenticated_transaction(&self.pool, session_token_hash).await?;
        let instructors = Self::read_starred_instructors_in(&mut tx, blueprint_course_id).await?;
        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(instructors)
    }

    async fn blueprint_course_watch_projection(
        &self,
        session_token_hash: SessionTokenHash,
        blueprint_course_id: BlueprintCourseId,
    ) -> Result<BlueprintCourseWatchProjection, StoreError> {
        let mut tx = begin_authenticated_transaction(&self.pool, session_token_hash).await?;
        let projection = Self::read_watch_in(&mut tx, blueprint_course_id).await?;
        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(projection)
    }

    async fn set_current_blueprint_course_watch_projection(
        &self,
        session_token_hash: SessionTokenHash,
        blueprint_course_id: BlueprintCourseId,
        watching: bool,
    ) -> Result<BlueprintCourseWatchProjection, StoreError> {
        let mut tx = begin_authenticated_transaction(&self.pool, session_token_hash).await?;
        sqlx::query("SELECT ple_api.set_current_blueprint_course_watch($1, $2)")
            .bind(blueprint_course_id.as_string())
            .bind(watching)
            .execute(&mut *tx)
            .await
            .map_err(map_sqlx_error)?;
        let projection = Self::read_watch_in(&mut tx, blueprint_course_id).await?;
        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(projection)
    }

    async fn blueprint_course_watch_events(
        &self,
        session_token_hash: SessionTokenHash,
        blueprint_course_id: BlueprintCourseId,
        query: BlueprintCourseWatchEventsQuery,
    ) -> Result<BlueprintCourseWatchEventsPage, StoreError> {
        let mut tx = begin_authenticated_transaction(&self.pool, session_token_hash).await?;
        let event_kinds: Vec<&str> = query
            .event_kinds
            .iter()
            .map(|kind| match kind {
                BlueprintCourseWatchEventKind::Revision => "revision",
                BlueprintCourseWatchEventKind::Archived => "archived",
                BlueprintCourseWatchEventKind::Restored => "restored",
            })
            .collect();
        // ASVS 1.2.4/8.2.2: fixed parameters and the installed recipient scope
        // produce the page, exact count, and cleared-kind facets in one statement.
        let rows = sqlx::query(
            "SELECT * FROM ple_api.read_current_blueprint_course_watch_events(\
             $1, $2, $3, $4, $5, $6)",
        )
        .bind(blueprint_course_id.as_string())
        .bind(&query.search)
        .bind(&event_kinds)
        .bind(query.direction == BlueprintCourseWatchDirection::Ascending)
        .bind(i64::from(query.page.page_number.get()))
        .bind(i64::from(query.page.page_size.get()))
        .fetch_all(&mut *tx)
        .await
        .map_err(map_sqlx_error)?;
        let metadata = rows.first().ok_or_else(|| {
            StoreError::InvalidRecord("Blueprint Course Watch page metadata".to_owned())
        })?;
        let total_count: i64 = metadata.try_get("total_count").map_err(map_sqlx_error)?;
        let total_count = u64::try_from(total_count).map_err(|_| {
            StoreError::InvalidRecord("Blueprint Course Watch event count".to_owned())
        })?;
        let resolved_page_number: i64 = metadata
            .try_get("resolved_page_number")
            .map_err(map_sqlx_error)?;
        let page_number = PageNumber::new(u32::try_from(resolved_page_number).map_err(|_| {
            StoreError::InvalidRecord("Blueprint Course Watch page number".to_owned())
        })?)
        .map_err(|_| StoreError::InvalidRecord("Blueprint Course Watch page number".to_owned()))?;
        let mut event_kind_facets = Vec::new();
        for (value, column) in [
            (BlueprintCourseWatchEventKind::Revision, "revision_count"),
            (BlueprintCourseWatchEventKind::Archived, "archived_count"),
            (BlueprintCourseWatchEventKind::Restored, "restored_count"),
        ] {
            let count: i64 = metadata.try_get(column).map_err(map_sqlx_error)?;
            let count = u64::try_from(count).map_err(|_| {
                StoreError::InvalidRecord("Blueprint Course Watch facet count".to_owned())
            })?;
            if count > 0 {
                event_kind_facets.push(BlueprintCourseWatchEventFacet { value, count });
            }
        }
        let mut events = Vec::with_capacity(rows.len());
        for row in rows {
            let notification_id: Option<Uuid> =
                row.try_get("notification_id").map_err(map_sqlx_error)?;
            if let Some(event_id) = notification_id {
                let event_kind: String = row.try_get("event_kind").map_err(map_sqlx_error)?;
                let kind = match event_kind.as_str() {
                    "revision" => BlueprintCourseWatchEventKind::Revision,
                    "archived" => BlueprintCourseWatchEventKind::Archived,
                    "restored" => BlueprintCourseWatchEventKind::Restored,
                    _ => {
                        return Err(StoreError::InvalidRecord(
                            "Blueprint Course Watch event kind".to_owned(),
                        ));
                    }
                };
                let occurred_at_millis: i64 =
                    row.try_get("occurred_at_millis").map_err(map_sqlx_error)?;
                events.push(BlueprintCourseWatchEvent {
                    event_id,
                    kind,
                    occurred_at: Timestamp::from_unix_millis(occurred_at_millis),
                });
            }
        }
        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(BlueprintCourseWatchEventsPage {
            page: NumberedPage {
                items: events,
                page_number,
                page_size: query.page.page_size,
                total_count,
            },
            event_kind_facets,
        })
    }
}
