//! PostgreSQL adapter for the private Library Watch outbox and inbox.

use async_trait::async_trait;
use question_model::{QuestionId, Timestamp};
use sqlx::{Postgres, Row, Transaction};

use super::{Pool, connection::map_sqlx_error};
use crate::{
    LibraryWatchActivity, LibraryWatchInboxStore, LibraryWatchNotification,
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

fn positive_u64(value: i64, name: &str) -> Result<u64, StoreError> {
    let value = u64::try_from(value).map_err(|_| StoreError::InvalidRecord(name.to_owned()))?;
    if value == 0 {
        return Err(StoreError::InvalidRecord(name.to_owned()));
    }
    Ok(value)
}

fn question_id(value: String, name: &str) -> Result<QuestionId, StoreError> {
    value
        .parse::<QuestionId>()
        .map_err(|_| StoreError::InvalidRecord(name.to_owned()))
}

// ASVS 2.2.1/2.2.3: consume the nullable SQL projection once and expose only
// complete, allowlisted Watch activity variants to downstream callers.
fn watch_activity(
    event_kind: String,
    revision_number: Option<i64>,
    forked_public_id: Option<String>,
    activity_id: Option<uuid::Uuid>,
) -> Result<LibraryWatchActivity, StoreError> {
    match (
        event_kind.as_str(),
        revision_number,
        forked_public_id,
        activity_id,
    ) {
        ("revision", Some(revision_number), None, None) => Ok(LibraryWatchActivity::Revision {
            revision_number: positive_u64(revision_number, "Library Watch revision number")?,
        }),
        ("fork", Some(source_revision_number), Some(forked_public_id), None) => {
            Ok(LibraryWatchActivity::Fork {
                source_revision_number: positive_u64(
                    source_revision_number,
                    "Library Watch source Revision number",
                )?,
                forked_public_id: question_id(forked_public_id, "Library Watch fork ID")?,
            })
        }
        ("improvement_thread", Some(creation_revision_number), None, Some(thread_id)) => {
            Ok(LibraryWatchActivity::ImprovementThread {
                creation_revision_number: positive_u64(
                    creation_revision_number,
                    "Library Watch creation Revision number",
                )?,
                thread_id,
            })
        }
        ("impact_notice", affected_revision_number, None, Some(impact_notice_id)) => {
            Ok(LibraryWatchActivity::ImpactNotice {
                affected_revision_number: affected_revision_number
                    .map(|value| positive_u64(value, "Library Watch affected Revision number"))
                    .transpose()?,
                impact_notice_id,
            })
        }
        _ => Err(StoreError::InvalidRecord(
            "Library Watch activity".to_owned(),
        )),
    }
}

fn notification(row: &sqlx::postgres::PgRow) -> Result<LibraryWatchNotification, StoreError> {
    let occurred_at_millis: i64 = row.try_get("occurred_at_millis").map_err(map_sqlx_error)?;
    if occurred_at_millis < 0 {
        return Err(StoreError::InvalidRecord(
            "Library Watch occurrence time".to_owned(),
        ));
    }
    Ok(LibraryWatchNotification {
        target_kind: target_kind(row.try_get("target_kind").map_err(map_sqlx_error)?)?,
        target_public_id: question_id(
            row.try_get("target_public_id").map_err(map_sqlx_error)?,
            "Library Watch target ID",
        )?,
        activity: watch_activity(
            row.try_get("event_kind").map_err(map_sqlx_error)?,
            row.try_get("revision_number").map_err(map_sqlx_error)?,
            row.try_get("forked_public_id").map_err(map_sqlx_error)?,
            row.try_get("activity_id").map_err(map_sqlx_error)?,
        )?,
        occurred_at: Timestamp::from_unix_millis(occurred_at_millis),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn watch_activity_accepts_each_complete_variant() {
        let thread_id = uuid::Uuid::from_u128(1);
        let notice_id = uuid::Uuid::from_u128(2);
        assert_eq!(
            watch_activity("revision".to_owned(), Some(1), None, None),
            Ok(LibraryWatchActivity::Revision { revision_number: 1 })
        );
        assert_eq!(
            watch_activity(
                "fork".to_owned(),
                Some(2),
                Some("1234-H567".to_owned()),
                None,
            ),
            Ok(LibraryWatchActivity::Fork {
                source_revision_number: 2,
                forked_public_id: "1234-H567".parse().expect("fixture ID should be valid"),
            })
        );
        assert_eq!(
            watch_activity(
                "improvement_thread".to_owned(),
                Some(3),
                None,
                Some(thread_id),
            ),
            Ok(LibraryWatchActivity::ImprovementThread {
                creation_revision_number: 3,
                thread_id,
            })
        );
        assert_eq!(
            watch_activity("impact_notice".to_owned(), None, None, Some(notice_id),),
            Ok(LibraryWatchActivity::ImpactNotice {
                affected_revision_number: None,
                impact_notice_id: notice_id,
            })
        );
    }

    #[test]
    fn watch_activity_rejects_cross_variant_evidence() {
        let activity_id = uuid::Uuid::from_u128(1);
        assert!(watch_activity("revision".to_owned(), None, None, None).is_err());
        assert!(
            watch_activity(
                "revision".to_owned(),
                Some(1),
                Some("1234-H567".to_owned()),
                None,
            )
            .is_err()
        );
        assert!(watch_activity("fork".to_owned(), Some(1), None, None).is_err());
        assert!(watch_activity("improvement_thread".to_owned(), Some(1), None, None,).is_err());
        assert!(
            watch_activity(
                "impact_notice".to_owned(),
                None,
                Some("1234-H567".to_owned()),
                Some(activity_id),
            )
            .is_err()
        );
        assert!(watch_activity("revision".to_owned(), Some(0), None, None).is_err());
        assert!(watch_activity("unknown".to_owned(), None, None, None).is_err());
    }
}
