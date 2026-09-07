//! PostgreSQL implementation of the M9 Course Roster lifecycle.

use async_trait::async_trait;
use question_model::CourseInstanceReference;
use sqlx::{Postgres, Row, Transaction};

use super::Pool;
use super::connection::map_sqlx_error;
use crate::{
    ClaimedCourseInvitation, CourseRosterEntry, CourseRosterEntryState, CourseRosterImportInput,
    CourseRosterStore, SessionTokenHash, StoreError,
};

/// PostgreSQL Store for Course Roster Import, claim, and access revocation.
#[derive(Clone)]
pub struct PostgresCourseRosterStore {
    pool: Pool,
}

impl PostgresCourseRosterStore {
    /// Binds the attested API pool to the Course Roster procedures.
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
impl CourseRosterStore for PostgresCourseRosterStore {
    async fn list_course_roster(
        &self,
        session_token_hash: SessionTokenHash,
        course: CourseInstanceReference,
    ) -> Result<Vec<CourseRosterEntry>, StoreError> {
        let mut transaction = self
            .begin_authenticated_application_transaction(session_token_hash)
            .await?;
        let rows = sqlx::query(
            "SELECT roster_id, roster_email, state FROM ple_api.list_live_demo_course_roster($1)",
        )
        .bind(i64::from(course.number()))
        .fetch_all(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let entries = rows
            .iter()
            .map(decode_entry)
            .collect::<Result<Vec<_>, _>>()?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(entries)
    }

    async fn import_course_roster(
        &self,
        session_token_hash: SessionTokenHash,
        course: CourseInstanceReference,
        input: CourseRosterImportInput,
    ) -> Result<Vec<CourseRosterEntry>, StoreError> {
        let entries = input.validated_entries()?;
        let normalized_emails = entries
            .iter()
            .map(|entry| entry.normalized_email.clone())
            .collect::<Vec<_>>();
        let delivery_emails = entries
            .iter()
            .map(|entry| entry.delivery_email.clone())
            .collect::<Vec<_>>();
        let roster_ids = entries
            .iter()
            .map(|entry| entry.roster_id.clone())
            .collect::<Vec<_>>();
        let mut transaction = self
            .begin_authenticated_application_transaction(session_token_hash)
            .await?;
        let rows = sqlx::query(
            "SELECT roster_id, roster_email, state \
             FROM ple_api.import_live_demo_course_roster($1, $2, $3, $4)",
        )
        .bind(i64::from(course.number()))
        .bind(normalized_emails)
        .bind(delivery_emails)
        .bind(roster_ids)
        .fetch_all(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let imported = rows
            .iter()
            .map(decode_entry)
            .collect::<Result<Vec<_>, _>>()?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(imported)
    }

    async fn claim_course_invitation(
        &self,
        session_token_hash: SessionTokenHash,
        course: CourseInstanceReference,
    ) -> Result<ClaimedCourseInvitation, StoreError> {
        let mut transaction = self
            .begin_authenticated_application_transaction(session_token_hash)
            .await?;
        let active_student_membership = sqlx::query_scalar::<_, bool>(
            "SELECT active_student_membership \
             FROM ple_api.claim_live_demo_course_invitation($1, $2, $3, $4)",
        )
        .bind(random_uuid()?)
        .bind(random_uuid()?)
        .bind(random_uuid()?)
        .bind(i64::from(course.number()))
        .fetch_one(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(ClaimedCourseInvitation {
            active_student_membership,
        })
    }

    async fn revoke_course_roster_entry(
        &self,
        session_token_hash: SessionTokenHash,
        course: CourseInstanceReference,
        roster_id: String,
    ) -> Result<(), StoreError> {
        if roster_id.is_empty() || roster_id.len() > 64 {
            return Err(StoreError::InvalidRecord(
                "Course roster identifier is invalid".to_string(),
            ));
        }
        let mut transaction = self
            .begin_authenticated_application_transaction(session_token_hash)
            .await?;
        sqlx::query("SELECT ple_api.revoke_live_demo_course_roster_entry($1, $2, $3)")
            .bind(random_uuid()?)
            .bind(i64::from(course.number()))
            .bind(&roster_id)
            .execute(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(())
    }
}

fn decode_entry(row: &sqlx::postgres::PgRow) -> Result<CourseRosterEntry, StoreError> {
    let state: String = row.try_get("state").map_err(map_sqlx_error)?;
    let state = match state.as_str() {
        "invitation_pending" => CourseRosterEntryState::InvitationPending,
        "active_student" => CourseRosterEntryState::ActiveStudent,
        _ => {
            return Err(StoreError::InvalidRecord(
                "database returned an invalid Course Roster state".to_string(),
            ));
        }
    };
    Ok(CourseRosterEntry {
        roster_id: row.try_get("roster_id").map_err(map_sqlx_error)?,
        roster_email: row.try_get("roster_email").map_err(map_sqlx_error)?,
        state,
    })
}

fn random_uuid() -> Result<uuid::Uuid, StoreError> {
    crate::random_uuid::random_uuid_v4(|_| {
        StoreError::Unavailable("Course Roster UUID randomness unavailable".to_string())
    })
}
