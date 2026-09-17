//! PostgreSQL adapter for the private Library Watch outbox and inbox.

use async_trait::async_trait;
use question_model::{QuestionId, Timestamp};
use sqlx::{Postgres, Row, Transaction};

use super::{Pool, connection::map_sqlx_error};
use crate::{
    LibraryWatchEventKind, LibraryWatchInboxStore, LibraryWatchNotification,
    LibraryWatchNotificationStore, LibraryWatchTargetKind, SessionTokenHash, StoreError,
};

#[derive(Clone)]
pub struct PostgresLibraryWatchNotificationStore {
    pool: Pool,
}

impl PostgresLibraryWatchNotificationStore {
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }

    async fn begin_inbox(
        &self,
        token: SessionTokenHash,
    ) -> Result<Transaction<'_, Postgres>, StoreError> {
        let mut transaction = self.pool.begin().await.map_err(map_sqlx_error)?;
        sqlx::query("SET LOCAL ROLE ple_auth")
            .execute(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        let session = sqlx::query(
            "SELECT session_id FROM ple_api.resolve_and_install_session(decode($1, 'hex'))",
        )
        .bind(token.to_string())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        if session.is_none() {
            return Err(StoreError::Forbidden);
        }
        sqlx::query("SET LOCAL ROLE ple_app")
            .execute(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        Ok(transaction)
    }
}

fn target_kind(value: String) -> Result<LibraryWatchTargetKind, StoreError> {
    match value.as_str() {
        "question" => Ok(LibraryWatchTargetKind::Question),
        "question_pool" => Ok(LibraryWatchTargetKind::QuestionPool),
        _ => Err(StoreError::InvalidRecord(
            "Library Watch target kind".to_owned(),
        )),
    }
}

fn event_kind(value: String) -> Result<LibraryWatchEventKind, StoreError> {
    match value.as_str() {
        "revision" => Ok(LibraryWatchEventKind::Revision),
        "fork" => Ok(LibraryWatchEventKind::Fork),
        "improvement_thread" => Ok(LibraryWatchEventKind::ImprovementThread),
        "impact_notice" => Ok(LibraryWatchEventKind::ImpactNotice),
        _ => Err(StoreError::InvalidRecord(
            "Library Watch event kind".to_owned(),
        )),
    }
}

fn notification(row: &sqlx::postgres::PgRow) -> Result<LibraryWatchNotification, StoreError> {
    let revision_number = row
        .try_get::<Option<i64>, _>("revision_number")
        .map_err(map_sqlx_error)?
        .map(u64::try_from)
        .transpose()
        .map_err(|_| StoreError::InvalidRecord("Library Watch revision number".to_owned()))?;
    let target_public_id = row
        .try_get::<String, _>("target_public_id")
        .map_err(map_sqlx_error)?
        .parse::<QuestionId>()
        .map_err(|_| StoreError::InvalidRecord("Library Watch target ID".to_owned()))?;
    let forked_public_id = row
        .try_get::<Option<String>, _>("forked_public_id")
        .map_err(map_sqlx_error)?
        .map(|value| {
            value
                .parse::<QuestionId>()
                .map_err(|_| StoreError::InvalidRecord("Library Watch fork ID".to_owned()))
        })
        .transpose()?;
    let activity_id = row
        .try_get::<Option<uuid::Uuid>, _>("activity_id")
        .map_err(map_sqlx_error)?;
    Ok(LibraryWatchNotification {
        target_kind: target_kind(row.try_get("target_kind").map_err(map_sqlx_error)?)?,
        target_public_id,
        event_kind: event_kind(row.try_get("event_kind").map_err(map_sqlx_error)?)?,
        revision_number,
        forked_public_id,
        activity_id,
        occurred_at: Timestamp::from_unix_millis(
            row.try_get("occurred_at_millis").map_err(map_sqlx_error)?,
        ),
    })
}

#[async_trait]
impl LibraryWatchNotificationStore for PostgresLibraryWatchNotificationStore {
    async fn materialize_library_watch_notifications(&self, limit: u16) -> Result<u32, StoreError> {
        if limit == 0 || limit > 500 {
            return Err(StoreError::InvalidRecord(
                "Library Watch notification limit is invalid".to_owned(),
            ));
        }
        let mut transaction = self.pool.begin().await.map_err(map_sqlx_error)?;
        sqlx::query("SET LOCAL ROLE ple_assessment_attempt_expiry_worker")
            .execute(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        let count: i32 =
            sqlx::query_scalar("SELECT ple_api.materialize_library_watch_notifications($1)")
                .bind(i32::from(limit))
                .fetch_one(&mut *transaction)
                .await
                .map_err(map_sqlx_error)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        u32::try_from(count).map_err(|_| {
            StoreError::InvalidRecord("Library Watch notification count is invalid".to_owned())
        })
    }
}

#[async_trait]
impl LibraryWatchInboxStore for PostgresLibraryWatchNotificationStore {
    async fn library_watch_notifications(
        &self,
        session_token_hash: SessionTokenHash,
        limit: u16,
    ) -> Result<Vec<LibraryWatchNotification>, StoreError> {
        if limit == 0 || limit > 100 {
            return Err(StoreError::InvalidRecord(
                "Library Watch inbox limit is invalid".to_owned(),
            ));
        }
        let mut transaction = self.begin_inbox(session_token_hash).await?;
        let rows =
            sqlx::query("SELECT * FROM ple_api.read_current_library_watch_notifications($1)")
                .bind(i32::from(limit))
                .fetch_all(&mut *transaction)
                .await
                .map_err(map_sqlx_error)?;
        let values = rows.into_iter().map(|row| notification(&row)).collect();
        transaction.commit().await.map_err(map_sqlx_error)?;
        values
    }
}
