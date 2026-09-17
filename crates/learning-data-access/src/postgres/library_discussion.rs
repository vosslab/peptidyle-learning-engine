//! PostgreSQL adapter for retained Question Library discussions.

use async_trait::async_trait;
use question_model::{LibraryObjectKind, Timestamp};
use sqlx::{Postgres, Row, Transaction};
use uuid::Uuid;

use super::{Pool, connection::map_sqlx_error as map_shared_sqlx_error};
use crate::{
    LibraryDiscussionStore, LibraryDiscussionTarget, LibraryDiscussionView, LibraryImpactNotice,
    LibraryImpactNoticeState, LibraryImprovementPost, LibraryImprovementThread,
    LibraryImprovementThreadState, SessionTokenHash, StoreError,
};

/// PostgreSQL Store whose procedures derive every actor, owner, and role.
#[derive(Clone)]
pub struct PostgresLibraryDiscussionStore {
    pool: Pool,
}

impl PostgresLibraryDiscussionStore {
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
}

fn kind_value(kind: LibraryObjectKind) -> &'static str {
    match kind {
        LibraryObjectKind::Question => "question",
        LibraryObjectKind::QuestionPool => "question_pool",
    }
}

const DISCUSSION_NOT_FOUND_SQLSTATE: &str = "P1D01";

fn map_sqlx_error(error: sqlx::Error) -> StoreError {
    if let sqlx::Error::Database(database_error) = &error
        && database_error.code().as_deref() == Some(DISCUSSION_NOT_FOUND_SQLSTATE)
    {
        return StoreError::NotFound;
    }
    map_shared_sqlx_error(error)
}

fn positive_u64(value: i64, name: &str) -> Result<u64, StoreError> {
    u64::try_from(value).map_err(|_| StoreError::InvalidRecord(name.to_owned()))
}

fn timestamp(value: i64, name: &str) -> Result<Timestamp, StoreError> {
    if value < 0 {
        return Err(StoreError::InvalidRecord(name.to_owned()));
    }
    Ok(Timestamp::from_unix_millis(value))
}

fn optional_timestamp(value: Option<i64>, name: &str) -> Result<Option<Timestamp>, StoreError> {
    value.map(|item| timestamp(item, name)).transpose()
}

fn text(value: String, name: &str) -> Result<String, StoreError> {
    if value != value.trim()
        || value.is_empty()
        || value.chars().count() > 4_000
        || value.chars().any(char::is_control)
    {
        return Err(StoreError::InvalidRecord(name.to_owned()));
    }
    Ok(value)
}

fn thread_state(value: String) -> Result<LibraryImprovementThreadState, StoreError> {
    match value.as_str() {
        "open" => Ok(LibraryImprovementThreadState::Open),
        "resolved" => Ok(LibraryImprovementThreadState::Resolved),
        _ => Err(StoreError::InvalidRecord(
            "Improvement thread state".to_owned(),
        )),
    }
}

fn notice_state(value: String) -> Result<LibraryImpactNoticeState, StoreError> {
    match value.as_str() {
        "active" => Ok(LibraryImpactNoticeState::Active),
        "cancelled" => Ok(LibraryImpactNoticeState::Cancelled),
        _ => Err(StoreError::InvalidRecord("Impact notice state".to_owned())),
    }
}

#[async_trait]
impl LibraryDiscussionStore for PostgresLibraryDiscussionStore {
    async fn library_discussion_view(
        &self,
        session_token_hash: SessionTokenHash,
        target: &LibraryDiscussionTarget,
    ) -> Result<LibraryDiscussionView, StoreError> {
        let mut tx = self.begin(session_token_hash).await?;
        let kind = kind_value(target.kind);
        let id = target.public_id.as_str();
        let thread_rows =
            sqlx::query("SELECT * FROM ple_api.read_library_improvement_threads($1, $2)")
                .bind(kind)
                .bind(id)
                .fetch_all(&mut *tx)
                .await
                .map_err(map_sqlx_error)?;
        let mut threads = Vec::with_capacity(thread_rows.len());
        for row in thread_rows {
            let thread_id: Uuid = row.try_get("thread_id").map_err(map_sqlx_error)?;
            let post_rows = sqlx::query("SELECT * FROM ple_api.read_library_improvement_posts($1)")
                .bind(thread_id)
                .fetch_all(&mut *tx)
                .await
                .map_err(map_sqlx_error)?;
            let posts = post_rows
                .iter()
                .map(|post| {
                    Ok(LibraryImprovementPost {
                        post_id: post.try_get("post_id").map_err(map_sqlx_error)?,
                        author_display_name: text(
                            post.try_get("author_display_name")
                                .map_err(map_sqlx_error)?,
                            "Improvement post author",
                        )?,
                        body: text(
                            post.try_get("body").map_err(map_sqlx_error)?,
                            "Improvement post body",
                        )?,
                        created_at: timestamp(
                            post.try_get("created_at_millis").map_err(map_sqlx_error)?,
                            "Improvement post created time",
                        )?,
                        updated_at: optional_timestamp(
                            post.try_get("updated_at_millis").map_err(map_sqlx_error)?,
                            "Improvement post updated time",
                        )?,
                        viewer_may_edit: post.try_get("viewer_may_edit").map_err(map_sqlx_error)?,
                    })
                })
                .collect::<Result<Vec<_>, StoreError>>()?;
            threads.push(LibraryImprovementThread {
                thread_id,
                creation_revision_number: positive_u64(
                    row.try_get("creation_revision_number")
                        .map_err(map_sqlx_error)?,
                    "Improvement thread creation Revision",
                )?,
                state: thread_state(row.try_get("state").map_err(map_sqlx_error)?)?,
                created_at: timestamp(
                    row.try_get("created_at_millis").map_err(map_sqlx_error)?,
                    "Improvement thread created time",
                )?,
                resolved_at: optional_timestamp(
                    row.try_get("resolved_at_millis").map_err(map_sqlx_error)?,
                    "Improvement thread resolved time",
                )?,
                viewer_may_resolve: row.try_get("viewer_may_resolve").map_err(map_sqlx_error)?,
                posts,
            });
        }
        let notices = sqlx::query("SELECT * FROM ple_api.read_library_impact_notices($1, $2)")
            .bind(kind)
            .bind(id)
            .fetch_all(&mut *tx)
            .await
            .map_err(map_sqlx_error)?
            .iter()
            .map(|row| {
                Ok(LibraryImpactNotice {
                    impact_notice_id: row.try_get("impact_notice_id").map_err(map_sqlx_error)?,
                    affected_revision_number: row
                        .try_get::<Option<i64>, _>("affected_revision_number")
                        .map_err(map_sqlx_error)?
                        .map(|value| positive_u64(value, "Impact notice affected Revision"))
                        .transpose()?,
                    author_display_name: text(
                        row.try_get("author_display_name").map_err(map_sqlx_error)?,
                        "Impact notice author",
                    )?,
                    body: text(
                        row.try_get("body").map_err(map_sqlx_error)?,
                        "Impact notice body",
                    )?,
                    state: notice_state(row.try_get("state").map_err(map_sqlx_error)?)?,
                    created_at: timestamp(
                        row.try_get("created_at_millis").map_err(map_sqlx_error)?,
                        "Impact notice created time",
                    )?,
                    updated_at: timestamp(
                        row.try_get("updated_at_millis").map_err(map_sqlx_error)?,
                        "Impact notice updated time",
                    )?,
                    cancelled_at: optional_timestamp(
                        row.try_get("cancelled_at_millis").map_err(map_sqlx_error)?,
                        "Impact notice cancelled time",
                    )?,
                    viewer_may_manage: row.try_get("viewer_may_manage").map_err(map_sqlx_error)?,
                })
            })
            .collect::<Result<Vec<_>, StoreError>>()?;
        let viewer_may_manage: bool =
            sqlx::query_scalar("SELECT ple_api.read_library_object_discussion_management($1, $2)")
                .bind(kind)
                .bind(id)
                .fetch_one(&mut *tx)
                .await
                .map_err(map_sqlx_error)?;
        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(LibraryDiscussionView {
            viewer_may_manage,
            threads,
            impact_notices: notices,
        })
    }

    async fn create_improvement_thread(
        &self,
        token: SessionTokenHash,
        target: &LibraryDiscussionTarget,
        body: &str,
    ) -> Result<Uuid, StoreError> {
        let mut tx = self.begin(token).await?;
        let id = sqlx::query_scalar("SELECT ple_api.create_library_improvement_thread($1, $2, $3)")
            .bind(kind_value(target.kind))
            .bind(target.public_id.as_str())
            .bind(body)
            .fetch_one(&mut *tx)
            .await
            .map_err(map_sqlx_error)?;
        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(id)
    }

    async fn reply_to_improvement_thread(
        &self,
        token: SessionTokenHash,
        target: &LibraryDiscussionTarget,
        thread_id: Uuid,
        body: &str,
    ) -> Result<Uuid, StoreError> {
        let mut tx = self.begin(token).await?;
        let id = sqlx::query_scalar(
            "SELECT ple_api.reply_to_library_improvement_thread($1, $2, $3, $4)",
        )
        .bind(kind_value(target.kind))
        .bind(target.public_id.as_str())
        .bind(thread_id)
        .bind(body)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_sqlx_error)?;
        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(id)
    }

    async fn edit_own_improvement_post(
        &self,
        token: SessionTokenHash,
        target: &LibraryDiscussionTarget,
        post_id: Uuid,
        body: &str,
    ) -> Result<(), StoreError> {
        let mut tx = self.begin(token).await?;
        sqlx::query("SELECT ple_api.edit_own_library_improvement_post($1, $2, $3, $4)")
            .bind(kind_value(target.kind))
            .bind(target.public_id.as_str())
            .bind(post_id)
            .bind(body)
            .execute(&mut *tx)
            .await
            .map_err(map_sqlx_error)?;
        tx.commit().await.map_err(map_sqlx_error)
    }

    async fn set_improvement_thread_resolved(
        &self,
        token: SessionTokenHash,
        target: &LibraryDiscussionTarget,
        thread_id: Uuid,
        resolved: bool,
    ) -> Result<(), StoreError> {
        let mut tx = self.begin(token).await?;
        sqlx::query("SELECT ple_api.set_library_improvement_thread_state($1, $2, $3, $4)")
            .bind(kind_value(target.kind))
            .bind(target.public_id.as_str())
            .bind(thread_id)
            .bind(resolved)
            .execute(&mut *tx)
            .await
            .map_err(map_sqlx_error)?;
        tx.commit().await.map_err(map_sqlx_error)
    }

    async fn create_impact_notice(
        &self,
        token: SessionTokenHash,
        target: &LibraryDiscussionTarget,
        affected_revision_number: Option<u64>,
        body: &str,
    ) -> Result<Uuid, StoreError> {
        let revision = affected_revision_number
            .map(|value| {
                i64::try_from(value).map_err(|_| {
                    StoreError::InvalidRecord("Impact notice affected Revision".to_owned())
                })
            })
            .transpose()?;
        let mut tx = self.begin(token).await?;
        let id = sqlx::query_scalar("SELECT ple_api.create_library_impact_notice($1, $2, $3, $4)")
            .bind(kind_value(target.kind))
            .bind(target.public_id.as_str())
            .bind(revision)
            .bind(body)
            .fetch_one(&mut *tx)
            .await
            .map_err(map_sqlx_error)?;
        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(id)
    }

    async fn update_impact_notice(
        &self,
        token: SessionTokenHash,
        target: &LibraryDiscussionTarget,
        impact_notice_id: Uuid,
        affected_revision_number: Option<u64>,
        body: &str,
    ) -> Result<(), StoreError> {
        let revision = affected_revision_number
            .map(|value| {
                i64::try_from(value).map_err(|_| {
                    StoreError::InvalidRecord("Impact notice affected Revision".to_owned())
                })
            })
            .transpose()?;
        let mut tx = self.begin(token).await?;
        sqlx::query("SELECT ple_api.update_library_impact_notice($1, $2, $3, $4, $5)")
            .bind(kind_value(target.kind))
            .bind(target.public_id.as_str())
            .bind(impact_notice_id)
            .bind(revision)
            .bind(body)
            .execute(&mut *tx)
            .await
            .map_err(map_sqlx_error)?;
        tx.commit().await.map_err(map_sqlx_error)
    }

    async fn cancel_impact_notice(
        &self,
        token: SessionTokenHash,
        target: &LibraryDiscussionTarget,
        impact_notice_id: Uuid,
    ) -> Result<(), StoreError> {
        let mut tx = self.begin(token).await?;
        sqlx::query("SELECT ple_api.cancel_library_impact_notice($1, $2, $3)")
            .bind(kind_value(target.kind))
            .bind(target.public_id.as_str())
            .bind(impact_notice_id)
            .execute(&mut *tx)
            .await
            .map_err(map_sqlx_error)?;
        tx.commit().await.map_err(map_sqlx_error)
    }
}
