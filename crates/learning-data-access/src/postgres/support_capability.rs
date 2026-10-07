//! PostgreSQL Store for confirmed Sysadmin Student-data access.

use async_trait::async_trait;
use question_model::{CourseInstanceId, Timestamp};
use sqlx::{Postgres, Row, Transaction};

use super::{Pool, connection::map_sqlx_error};
use crate::{
    ConfirmedStudentDataAccess, SessionTokenHash, StoreError, StudentDataAccessAuditReceipt,
    SysadminStudentDataStore, SysadminStudentRosterRecord, SysadminStudentRosterState,
};

#[derive(Clone)]
pub struct PostgresSysadminStudentDataStore {
    pool: Pool,
}

impl PostgresSysadminStudentDataStore {
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
        if sqlx::query(
            "SELECT session_id FROM ple_api.resolve_and_install_session(decode($1, 'hex'))",
        )
        .bind(token.to_string())
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_sqlx_error)?
        .is_none()
        {
            return Err(StoreError::Forbidden);
        }
        sqlx::query("SET LOCAL ROLE ple_app")
            .execute(&mut *tx)
            .await
            .map_err(map_sqlx_error)?;
        Ok(tx)
    }
}

#[async_trait]
impl SysadminStudentDataStore for PostgresSysadminStudentDataStore {
    async fn read_sysadmin_student_roster_record(
        &self,
        token: SessionTokenHash,
        course_instance_id: CourseInstanceId,
        roster_id: String,
        confirmation: ConfirmedStudentDataAccess,
    ) -> Result<Option<SysadminStudentRosterRecord>, StoreError> {
        confirmation.validate()?;
        if roster_id != roster_id.trim()
            || !(1..=128).contains(&roster_id.chars().count())
            || !roster_id
                .chars()
                .all(|character| character.is_ascii_alphanumeric() || "._-".contains(character))
        {
            return Err(StoreError::InvalidRecord(
                "Roster ID is invalid".to_string(),
            ));
        }

        let mut tx = self.begin(token).await?;
        let row = sqlx::query(
            "SELECT course_instance_id, student_account_id, roster_id, roster_name, state, event_id, occurred_at_millis FROM ple_api.read_sysadmin_student_roster_record($1, $2, $3)",
        )
        .bind(course_instance_id.to_string())
        .bind(&roster_id)
        .bind(confirmation.administrative_access_confirmed)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_sqlx_error)?;

        let Some(row) = row else {
            tx.commit().await.map_err(map_sqlx_error)?;
            return Ok(None);
        };
        let record = SysadminStudentRosterRecord {
            course_instance_id: row
                .try_get::<String, _>("course_instance_id")
                .map_err(map_sqlx_error)?
                .parse()
                .map_err(|_| StoreError::InvalidRecord("Course Instance ID is invalid".into()))?,
            student_account_id: row
                .try_get::<String, _>("student_account_id")
                .map_err(map_sqlx_error)?
                .parse()
                .map_err(|_| StoreError::InvalidRecord("Student Account ID is invalid".into()))?,
            roster_id: row.try_get("roster_id").map_err(map_sqlx_error)?,
            roster_name: row.try_get("roster_name").map_err(map_sqlx_error)?,
            state: decode_state(row.try_get("state").map_err(map_sqlx_error)?)?,
            audit: StudentDataAccessAuditReceipt {
                event_id: row.try_get("event_id").map_err(map_sqlx_error)?,
                occurred_at: Timestamp::from_unix_millis(
                    row.try_get("occurred_at_millis").map_err(map_sqlx_error)?,
                ),
            },
        };
        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(Some(record))
    }
}

fn decode_state(value: String) -> Result<SysadminStudentRosterState, StoreError> {
    match value.as_str() {
        "active_student" => Ok(SysadminStudentRosterState::ActiveStudent),
        "invitation_pending" => Ok(SysadminStudentRosterState::InvitationPending),
        "removed" => Ok(SysadminStudentRosterState::Removed),
        _ => Err(StoreError::InvalidRecord(
            "Student roster state is invalid".to_string(),
        )),
    }
}
