//! Shared authenticated transaction boundary for current Assessment operations.

use super::{Pool, connection::map_sqlx_error};
use crate::{CourseInstancePoolIdIssuer, SessionTokenHash, StoreError};
use sqlx::{Postgres, Transaction};
use std::sync::Arc;

/// PostgreSQL Store for the direct-Instructor Assessment Workspace.
#[derive(Clone)]
pub struct PostgresLiveAssessmentStore {
    pool: Pool,
    pub(super) pool_id_issuer: Option<Arc<dyn CourseInstancePoolIdIssuer>>,
}

impl PostgresLiveAssessmentStore {
    /// Binds the attested API pool to Assessment Workspace procedures.
    pub fn new(pool: Pool) -> Self {
        Self {
            pool,
            pool_id_issuer: None,
        }
    }

    /// Adds the process-held identity issuer needed only for changed reusable Pools.
    pub fn with_pool_id_issuer(mut self, issuer: Arc<dyn CourseInstancePoolIdIssuer>) -> Self {
        self.pool_id_issuer = Some(issuer);
        self
    }

    pub(super) async fn begin(
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
