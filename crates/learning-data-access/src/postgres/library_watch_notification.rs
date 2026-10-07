//! PostgreSQL adapter for the private Library Watch outbox and inbox.

use async_trait::async_trait;
use question_model::{LibraryObjectId, QuestionPoolEditNumber, QuestionRevisionNumber, Timestamp};
use sqlx::{Postgres, Row, Transaction};

use super::{Pool, connection::map_sqlx_error};
use crate::{
    LibraryWatchActivity, LibraryWatchInboxStore, LibraryWatchNotification, LibraryWatchTargetKind,
    SessionTokenHash, StoreError,
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

fn decode_question_revision_number(value: i64) -> Result<QuestionRevisionNumber, StoreError> {
    let value = u32::try_from(value).map_err(|_| {
        StoreError::InvalidRecord("Library Watch Question Revision Number".to_owned())
    })?;
    QuestionRevisionNumber::new(value)
        .map_err(|_| StoreError::InvalidRecord("Library Watch Question Revision Number".to_owned()))
}

fn decode_question_pool_edit_number(value: i64) -> Result<QuestionPoolEditNumber, StoreError> {
    let value = u64::try_from(value).map_err(|_| {
        StoreError::InvalidRecord("Library Watch Question Pool Edit Number".to_owned())
    })?;
    QuestionPoolEditNumber::new(value).map_err(|_| {
        StoreError::InvalidRecord("Library Watch Question Pool Edit Number".to_owned())
    })
}

fn library_object_id(value: String, name: &str) -> Result<LibraryObjectId, StoreError> {
    value
        .parse::<LibraryObjectId>()
        .map_err(|_| StoreError::InvalidRecord(name.to_owned()))
}

// ASVS 2.2.1/2.2.3: consume the nullable SQL projection once and expose only
// complete, allowlisted Watch activity variants to downstream callers.
fn watch_activity(
    target_kind: LibraryWatchTargetKind,
    event_kind: String,
    question_revision_number: Option<i64>,
    question_pool_edit_number: Option<i64>,
    forked_public_id: Option<String>,
) -> Result<LibraryWatchActivity, StoreError> {
    match (
        target_kind,
        event_kind.as_str(),
        question_revision_number,
        question_pool_edit_number,
        forked_public_id,
    ) {
        (LibraryWatchTargetKind::Question, "revision", Some(number), None, None) => {
            Ok(LibraryWatchActivity::Revision {
                question_revision_number: decode_question_revision_number(number)?,
            })
        }
        // ASVS 5.1.1: a membership edit carries the Pool Edit Number and no fork evidence.
        (LibraryWatchTargetKind::QuestionPool, "members_changed", None, Some(number), None) => {
            Ok(LibraryWatchActivity::MembersChanged {
                question_pool_edit_number: decode_question_pool_edit_number(number)?,
            })
        }
        (LibraryWatchTargetKind::Question, "fork", Some(number), None, Some(forked_public_id)) => {
            Ok(LibraryWatchActivity::QuestionFork {
                source_question_revision_number: decode_question_revision_number(number)?,
                forked_public_id: library_object_id(forked_public_id, "Library Watch fork ID")?,
            })
        }
        (
            LibraryWatchTargetKind::QuestionPool,
            "fork",
            None,
            Some(number),
            Some(forked_public_id),
        ) => Ok(LibraryWatchActivity::QuestionPoolFork {
            source_question_pool_edit_number: decode_question_pool_edit_number(number)?,
            forked_public_id: library_object_id(forked_public_id, "Library Watch fork ID")?,
        }),
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
    let target_kind = target_kind(row.try_get("target_kind").map_err(map_sqlx_error)?)?;
    let activity = watch_activity(
        target_kind,
        row.try_get("event_kind").map_err(map_sqlx_error)?,
        row.try_get("question_revision_number")
            .map_err(map_sqlx_error)?,
        row.try_get("question_pool_edit_number")
            .map_err(map_sqlx_error)?,
        row.try_get("forked_public_id").map_err(map_sqlx_error)?,
    )?;
    Ok(LibraryWatchNotification {
        target_kind,
        target_public_id: library_object_id(
            row.try_get("target_public_id").map_err(map_sqlx_error)?,
            "Library Watch target ID",
        )?,
        activity,
        occurred_at: Timestamp::from_unix_millis(occurred_at_millis),
    })
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
        let forked_public_id = || {
            "1234-H567"
                .parse::<LibraryObjectId>()
                .expect("fixture ID should be valid")
        };
        assert_eq!(
            watch_activity(
                LibraryWatchTargetKind::Question,
                "revision".to_owned(),
                Some(1),
                None,
                None
            ),
            Ok(LibraryWatchActivity::Revision {
                question_revision_number: QuestionRevisionNumber::new(1).expect("revision"),
            })
        );
        assert_eq!(
            watch_activity(
                LibraryWatchTargetKind::QuestionPool,
                "members_changed".to_owned(),
                None,
                Some(2),
                None
            ),
            Ok(LibraryWatchActivity::MembersChanged {
                question_pool_edit_number: QuestionPoolEditNumber::new(2)
                    .expect("Pool Edit Number"),
            })
        );
        assert_eq!(
            watch_activity(
                LibraryWatchTargetKind::Question,
                "fork".to_owned(),
                Some(2),
                None,
                Some("1234-H567".to_owned())
            ),
            Ok(LibraryWatchActivity::QuestionFork {
                source_question_revision_number: QuestionRevisionNumber::new(2).expect("revision"),
                forked_public_id: forked_public_id(),
            })
        );
        assert_eq!(
            watch_activity(
                LibraryWatchTargetKind::QuestionPool,
                "fork".to_owned(),
                None,
                Some(3),
                Some("1234-H567".to_owned())
            ),
            Ok(LibraryWatchActivity::QuestionPoolFork {
                source_question_pool_edit_number: QuestionPoolEditNumber::new(3)
                    .expect("Pool Edit Number"),
                forked_public_id: forked_public_id(),
            })
        );
    }

    #[test]
    fn watch_activity_rejects_cross_variant_evidence() {
        assert!(
            watch_activity(
                LibraryWatchTargetKind::Question,
                "revision".to_owned(),
                None,
                None,
                None
            )
            .is_err()
        );
        assert!(
            watch_activity(
                LibraryWatchTargetKind::Question,
                "revision".to_owned(),
                Some(1),
                None,
                Some("1234-H567".to_owned()),
            )
            .is_err()
        );
        assert!(
            watch_activity(
                LibraryWatchTargetKind::Question,
                "fork".to_owned(),
                Some(1),
                None,
                None
            )
            .is_err()
        );
        assert!(
            watch_activity(
                LibraryWatchTargetKind::Question,
                "unknown".to_owned(),
                None,
                None,
                None
            )
            .is_err()
        );
        assert!(
            watch_activity(
                LibraryWatchTargetKind::Question,
                "revision".to_owned(),
                Some(0),
                None,
                None
            )
            .is_err()
        );
        assert!(
            watch_activity(
                LibraryWatchTargetKind::QuestionPool,
                "members_changed".to_owned(),
                None,
                None,
                None
            )
            .is_err()
        );
        assert!(
            watch_activity(
                LibraryWatchTargetKind::QuestionPool,
                "members_changed".to_owned(),
                None,
                Some(2),
                Some("1234-H567".to_owned()),
            )
            .is_err()
        );
        assert!(
            watch_activity(
                LibraryWatchTargetKind::QuestionPool,
                "members_changed".to_owned(),
                None,
                Some(0),
                None
            )
            .is_err()
        );
        assert!(
            watch_activity(
                LibraryWatchTargetKind::Question,
                "revision".to_owned(),
                Some(1),
                Some(1),
                None
            )
            .is_err()
        );
    }
}
