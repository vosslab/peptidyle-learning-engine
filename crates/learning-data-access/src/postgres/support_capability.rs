//! PostgreSQL Store for the closed M17 support registry.

use async_trait::async_trait;
use question_model::{AccountReference, CourseInstanceReference, Timestamp};
use sqlx::{Postgres, Row, Transaction};
use uuid::Uuid;

use super::{Pool, connection::map_sqlx_error};
use crate::random_uuid::random_uuid_v4;
use crate::{
    CourseRosterEntry, CourseRosterEntryState, IssueSupportCapabilityInput, SessionTokenHash,
    StoreError, SupportCapabilityReceipt, SupportCapabilityStore, SupportMinimumProjection,
    SupportOperationKind,
};

#[derive(Clone)]
pub struct PostgresSupportCapabilityStore {
    pool: Pool,
}
impl PostgresSupportCapabilityStore {
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
impl SupportCapabilityStore for PostgresSupportCapabilityStore {
    async fn issue_course_roster_support(
        &self,
        token: SessionTokenHash,
        course: CourseInstanceReference,
        input: IssueSupportCapabilityInput,
    ) -> Result<SupportCapabilityReceipt, StoreError> {
        input.validate()?;
        let mut tx = self.begin(token).await?;
        let row = sqlx::query("SELECT capability_id, course_reference_number, sysadmin_reference_number, purpose, expires_at_millis, revoked_at_millis FROM ple_api.issue_live_demo_course_roster_support($1, $2, $3, $4)")
            .bind(i64::from(course.number())).bind(i64::from(input.sysadmin_reference.number())).bind(&input.purpose).bind(random_uuid()?).fetch_optional(&mut *tx).await.map_err(map_sqlx_error)?.ok_or(StoreError::NotFound)?;
        let receipt = decode(&row)?;
        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(receipt)
    }
    async fn revoke_course_roster_support(
        &self,
        token: SessionTokenHash,
        course: CourseInstanceReference,
        capability_id: Uuid,
    ) -> Result<SupportCapabilityReceipt, StoreError> {
        let mut tx = self.begin(token).await?;
        let row = sqlx::query("SELECT capability_id, course_reference_number, sysadmin_reference_number, purpose, expires_at_millis, revoked_at_millis FROM ple_api.revoke_live_demo_course_roster_support($1, $2)")
            .bind(i64::from(course.number())).bind(capability_id).fetch_optional(&mut *tx).await.map_err(map_sqlx_error)?.ok_or(StoreError::NotFound)?;
        let receipt = decode(&row)?;
        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(receipt)
    }
    async fn read_course_roster_support(
        &self,
        token: SessionTokenHash,
        capability_id: Uuid,
    ) -> Result<Vec<CourseRosterEntry>, StoreError> {
        let mut tx = self.begin(token).await?;
        let rows = sqlx::query("SELECT roster_id, roster_email, state FROM ple_api.list_live_demo_support_course_roster($1)")
            .bind(capability_id).fetch_all(&mut *tx).await.map_err(map_sqlx_error)?;
        let entries = rows
            .iter()
            .map(|row| {
                let state: String = row.try_get("state").map_err(map_sqlx_error)?;
                let state = match state.as_str() {
                    "invitation_pending" => CourseRosterEntryState::InvitationPending,
                    "active_student" => CourseRosterEntryState::ActiveStudent,
                    _ => return Err(invalid("Course Roster state")),
                };
                Ok(CourseRosterEntry {
                    roster_id: row.try_get("roster_id").map_err(map_sqlx_error)?,
                    roster_email: row.try_get("roster_email").map_err(map_sqlx_error)?,
                    state,
                })
            })
            .collect::<Result<Vec<_>, StoreError>>()?;
        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(entries)
    }
}
fn decode(row: &sqlx::postgres::PgRow) -> Result<SupportCapabilityReceipt, StoreError> {
    let account = |v: i64| {
        u64::try_from(v)
            .ok()
            .and_then(AccountReference::new)
            .ok_or_else(|| invalid("Sysadmin Reference"))
    };
    let course = |v: i64| {
        u64::try_from(v)
            .ok()
            .and_then(CourseInstanceReference::new)
            .ok_or_else(|| invalid("Course Reference"))
    };
    Ok(SupportCapabilityReceipt {
        capability_id: row.try_get("capability_id").map_err(map_sqlx_error)?,
        course_reference: course(
            row.try_get("course_reference_number")
                .map_err(map_sqlx_error)?,
        )?,
        sysadmin_reference: account(
            row.try_get("sysadmin_reference_number")
                .map_err(map_sqlx_error)?,
        )?,
        operation_kind: SupportOperationKind::CourseRosterSupport,
        minimum_projection: SupportMinimumProjection::CourseRoster,
        purpose: row.try_get("purpose").map_err(map_sqlx_error)?,
        expires_at: Timestamp::from_unix_millis(
            row.try_get("expires_at_millis").map_err(map_sqlx_error)?,
        ),
        revoked_at: row
            .try_get::<Option<i64>, _>("revoked_at_millis")
            .map_err(map_sqlx_error)?
            .map(Timestamp::from_unix_millis),
    })
}
fn invalid(label: &str) -> StoreError {
    StoreError::InvalidRecord(format!("database returned an invalid {label}"))
}

fn random_uuid() -> Result<Uuid, StoreError> {
    random_uuid_v4(|_| {
        StoreError::Unavailable("Support capability randomness unavailable".to_string())
    })
}
