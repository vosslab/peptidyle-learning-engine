//! PostgreSQL implementation of the self-only Blueprint Course stewardship boundary.

use async_trait::async_trait;
use question_model::{BlueprintCourseId, Timestamp};
use sqlx::{Postgres, Row, Transaction};

use super::{Pool, connection::map_sqlx_error};
use crate::{
    BlueprintCourseStarProjection, BlueprintCourseStarredInstructor, BlueprintCourseWatchEvent,
    BlueprintCourseWatchEventKind, BlueprintCourseWatchProjection, BlueprintStewardshipStore,
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

    async fn begin(
        &self,
        token: SessionTokenHash,
    ) -> Result<Transaction<'_, Postgres>, StoreError> {
        let mut tx = self.pool.begin().await.map_err(map_sqlx_error)?;
        sqlx::query("SET LOCAL ROLE ple_auth")
            .execute(&mut *tx)
            .await
            .map_err(map_sqlx_error)?;
        let session = sqlx::query(
            "SELECT session_id FROM ple_api.resolve_and_install_session(decode($1, 'hex'))",
        )
        .bind(token.to_string())
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_sqlx_error)?;
        if session.is_none() {
            return Err(StoreError::Forbidden);
        }
        sqlx::query("SET LOCAL ROLE ple_app")
            .execute(&mut *tx)
            .await
            .map_err(map_sqlx_error)?;
        Ok(tx)
    }

    async fn read_star_in(
        tx: &mut Transaction<'_, Postgres>,
        reference: BlueprintCourseId,
    ) -> Result<BlueprintCourseStarProjection, StoreError> {
        let row = sqlx::query("SELECT * FROM ple_api.read_current_blueprint_course_star($1)")
            .bind(reference.as_string())
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
        reference: BlueprintCourseId,
    ) -> Result<BlueprintCourseWatchProjection, StoreError> {
        let row = sqlx::query("SELECT * FROM ple_api.read_current_blueprint_course_watch($1)")
            .bind(reference.as_string())
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
        reference: BlueprintCourseId,
    ) -> Result<Vec<BlueprintCourseStarredInstructor>, StoreError> {
        // ASVS 8.2.1--8.3.1: the SQL capability derives the viewer from the
        // installed session; it accepts no Account ID or claimed role.
        let rows = sqlx::query(
            "SELECT display_name FROM ple_api.read_current_blueprint_course_starred_instructors($1)",
        )
        .bind(reference.as_string())
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
        let mut tx = self.begin(session_token_hash).await?;
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
        let mut tx = self.begin(session_token_hash).await?;
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
        let mut tx = self.begin(session_token_hash).await?;
        let instructors = Self::read_starred_instructors_in(&mut tx, blueprint_course_id).await?;
        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(instructors)
    }

    async fn blueprint_course_watch_projection(
        &self,
        session_token_hash: SessionTokenHash,
        blueprint_course_id: BlueprintCourseId,
    ) -> Result<BlueprintCourseWatchProjection, StoreError> {
        let mut tx = self.begin(session_token_hash).await?;
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
        let mut tx = self.begin(session_token_hash).await?;
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
        limit: u16,
    ) -> Result<Vec<BlueprintCourseWatchEvent>, StoreError> {
        if limit == 0 || limit > 100 {
            return Err(StoreError::InvalidRecord(
                "Blueprint Course Watch event limit is invalid".to_owned(),
            ));
        }
        let mut tx = self.begin(session_token_hash).await?;
        let rows =
            sqlx::query("SELECT * FROM ple_api.read_current_blueprint_course_watch_events($1, $2)")
                .bind(blueprint_course_id.as_string())
                .bind(i32::from(limit))
                .fetch_all(&mut *tx)
                .await
                .map_err(map_sqlx_error)?;
        let events = rows
            .into_iter()
            .map(|row| {
                let event_kind: String = row.try_get("event_kind").map_err(map_sqlx_error)?;
                let kind = match event_kind.as_str() {
                    "revision" => BlueprintCourseWatchEventKind::Revision,
                    "published" => BlueprintCourseWatchEventKind::Published,
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
                Ok(BlueprintCourseWatchEvent {
                    kind,
                    occurred_at: Timestamp::from_unix_millis(occurred_at_millis),
                })
            })
            .collect::<Result<Vec<_>, StoreError>>()?;
        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(events)
    }
}
