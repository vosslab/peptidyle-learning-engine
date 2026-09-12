//! PostgreSQL implementation of the protected invitation export boundary.

use async_trait::async_trait;
use question_model::CourseInstanceReference;
use sqlx::{Postgres, Row, Transaction};

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

    async fn begin_authenticated_application_transaction(
        &self,
        token_hash: SessionTokenHash,
    ) -> Result<Transaction<'_, Postgres>, StoreError> {
        let mut transaction = self.pool.begin().await.map_err(map_sqlx_error)?;
        sqlx::query("SET LOCAL ROLE ple_auth")
            .execute(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        let session = sqlx::query(
            "SELECT session_id FROM ple_api.resolve_and_install_session(decode($1, 'hex'))",
        )
        .bind(token_hash.to_string())
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

#[async_trait]
impl InvitationExportStore for PostgresInvitationExportStore {
    async fn export_pending_course_invitations(
        &self,
        session_token_hash: SessionTokenHash,
        course: CourseInstanceReference,
    ) -> Result<PendingInvitationExport, StoreError> {
        let mut transaction = self
            .begin_authenticated_application_transaction(session_token_hash)
            .await?;
        // ASVS 4.1.3 and 8.2.1: the trusted procedure verifies the current
        // direct Instructor Course Membership before it returns private data.
        let course_name = sqlx::query_scalar::<_, String>(
            "SELECT course_name FROM ple_api.load_invitation_export_course($1)",
        )
        .bind(i64::from(course.number()))
        .fetch_one(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let rows = sqlx::query(
            "SELECT roster_email, roster_id \
             FROM ple_api.export_pending_course_invitations($1)",
        )
        .bind(i64::from(course.number()))
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
