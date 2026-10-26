//! PostgreSQL implementation of the protected invitation export boundary.

use crate::postgres::connection::begin_authenticated_transaction;
use async_trait::async_trait;
use question_model::CourseInstanceId;
use sqlx::Row;

use super::{Pool, connection::map_sqlx_error};
use crate::{
    InvitationExportStore, PendingInvitationExport, PendingInvitationRecipient, SessionTokenHash,
    StoreError,
};

/// PostgreSQL Store for the direct-Instructor pending invitation export.
#[derive(Clone)]
pub struct PostgresInvitationExportStore {
    pool: Pool,
}

impl PostgresInvitationExportStore {
    /// Binds the attested API pool to the protected export procedure.
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl InvitationExportStore for PostgresInvitationExportStore {
    async fn export_pending_course_invitations(
        &self,
        session_token_hash: SessionTokenHash,
        course_instance_id: CourseInstanceId,
    ) -> Result<PendingInvitationExport, StoreError> {
        let mut transaction =
            begin_authenticated_transaction(&self.pool, session_token_hash).await?;
        // ASVS 4.1.3 and 8.2.1: the trusted procedure verifies the current
        // direct Instructor Course Membership before it returns private data.
        let course_name = sqlx::query_scalar::<_, String>(
            "SELECT course_name FROM ple_api.load_invitation_export_course($1)",
        )
        .bind(course_instance_id.as_string())
        .fetch_one(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let rows = sqlx::query(
            "SELECT roster_email, roster_id \
             FROM ple_api.export_pending_course_invitations($1)",
        )
        .bind(course_instance_id.as_string())
        .fetch_all(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let recipients = rows
            .iter()
            .map(|row| {
                Ok(PendingInvitationRecipient {
                    email: row.try_get("roster_email").map_err(map_sqlx_error)?,
                    roster_id: row.try_get("roster_id").map_err(map_sqlx_error)?,
                })
            })
            .collect::<Result<Vec<_>, StoreError>>()?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(PendingInvitationExport {
            course_name,
            recipients,
        })
    }
}
