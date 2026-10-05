//! PostgreSQL adapter for retained Question Library impact notices.

use async_trait::async_trait;
use question_model::LibraryObjectKind;
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use super::{Pool, connection::map_sqlx_error as map_shared_sqlx_error};
use crate::{LibraryImpactNoticeStore, LibraryObjectTarget, SessionTokenHash, StoreError};

/// PostgreSQL Store whose procedures derive every actor, owner, and role.
#[derive(Clone)]
pub struct PostgresLibraryImpactNoticeStore {
    pool: Pool,
}

impl PostgresLibraryImpactNoticeStore {
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

const IMPACT_NOTICE_NOT_FOUND_SQLSTATE: &str = "P1D01";
const FOREIGN_KEY_VIOLATION_SQLSTATE: &str = "23503";

fn map_sqlx_error(error: sqlx::Error) -> StoreError {
    if let sqlx::Error::Database(database_error) = &error
        && matches!(
            database_error.code().as_deref(),
            Some(IMPACT_NOTICE_NOT_FOUND_SQLSTATE | FOREIGN_KEY_VIOLATION_SQLSTATE)
        )
    {
        return StoreError::NotFound;
    }
    map_shared_sqlx_error(error)
}

#[async_trait]
impl LibraryImpactNoticeStore for PostgresLibraryImpactNoticeStore {
    async fn create_impact_notice(
        &self,
        session_token_hash: SessionTokenHash,
        target: &LibraryObjectTarget,
        affected_revision_number: Option<u64>,
        body: &str,
    ) -> Result<Uuid, StoreError> {
        let mut tx = self.begin(session_token_hash).await?;
        let id = sqlx::query_scalar("SELECT ple_api.create_library_impact_notice($1, $2, $3, $4)")
            .bind(kind_value(target.kind))
            .bind(target.public_id.as_str())
            .bind(revision_number(affected_revision_number)?)
            .bind(body)
            .fetch_one(&mut *tx)
            .await
            .map_err(map_sqlx_error)?;
        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(id)
    }

    async fn update_impact_notice(
        &self,
        session_token_hash: SessionTokenHash,
        target: &LibraryObjectTarget,
        impact_notice_id: Uuid,
        affected_revision_number: Option<u64>,
        body: &str,
    ) -> Result<(), StoreError> {
        let mut tx = self.begin(session_token_hash).await?;
        sqlx::query("SELECT ple_api.update_library_impact_notice($1, $2, $3, $4, $5)")
            .bind(kind_value(target.kind))
            .bind(target.public_id.as_str())
            .bind(impact_notice_id)
            .bind(revision_number(affected_revision_number)?)
            .bind(body)
            .execute(&mut *tx)
            .await
            .map_err(map_sqlx_error)?;
        tx.commit().await.map_err(map_sqlx_error)
    }

    async fn cancel_impact_notice(
        &self,
        session_token_hash: SessionTokenHash,
        target: &LibraryObjectTarget,
        impact_notice_id: Uuid,
    ) -> Result<(), StoreError> {
        let mut tx = self.begin(session_token_hash).await?;
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

fn revision_number(value: Option<u64>) -> Result<Option<i64>, StoreError> {
    value
        .map(i64::try_from)
        .transpose()
        .map_err(|_| StoreError::InvalidRecord("Impact notice Revision".to_owned()))
}
