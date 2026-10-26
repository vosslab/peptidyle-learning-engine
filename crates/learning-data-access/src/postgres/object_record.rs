//! PostgreSQL registration for private Workspace Question Source Object Records.

use crate::postgres::connection::begin_authenticated_transaction;
use async_trait::async_trait;
use objects::ObjectRecord;
use question_model::WorkspaceId;

use super::Pool;
use super::connection::map_sqlx_error;
use crate::{
    SessionTokenHash, StoreError, WorkspaceQuestionSourceObjectRecordStore,
    validate_workspace_question_source_object_record,
};

/// PostgreSQL implementation of the session-authorized private source-record
/// registration boundary.
#[derive(Clone)]
pub struct PostgresWorkspaceQuestionSourceObjectRecordStore {
    pool: Pool,
}

impl PostgresWorkspaceQuestionSourceObjectRecordStore {
    /// Binds the already-attested API pool to workspace Object Record persistence.
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl WorkspaceQuestionSourceObjectRecordStore for PostgresWorkspaceQuestionSourceObjectRecordStore {
    async fn register_workspace_question_source_object(
        &self,
        session_token_hash: SessionTokenHash,
        workspace: WorkspaceId,
        record: ObjectRecord,
    ) -> Result<(), StoreError> {
        validate_workspace_question_source_object_record(workspace, &record)?;
        let address = serde_json::to_value(&record.address).map_err(|_| {
            StoreError::InvalidRecord(
                "Workspace Question Source Object Address cannot be encoded".to_string(),
            )
        })?;
        let size_bytes = i64::try_from(record.size_bytes).map_err(|_| {
            StoreError::InvalidRecord(
                "Workspace Question Source Object size exceeds PostgreSQL bigint".to_string(),
            )
        })?;
        let mut transaction =
            begin_authenticated_transaction(&self.pool, session_token_hash).await?;
        sqlx::query(
            "SELECT ple_api.register_workspace_question_source_object(\
                $1, $2, $3, $4, $5, $6, $7\
             )",
        )
        .bind(workspace.as_uuid())
        .bind(record.id.as_uuid())
        .bind(address)
        .bind(record.sha256.as_bytes().to_vec())
        .bind(size_bytes)
        .bind(record.media_type)
        .bind(record.created_at.as_unix_millis())
        .execute(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        transaction.commit().await.map_err(map_sqlx_error)
    }
}
