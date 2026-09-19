//! PostgreSQL Store for the closed support registry.

use async_trait::async_trait;
use question_model::{AccountId, CourseInstanceId, Timestamp};
use sqlx::{Postgres, Row, Transaction};
use uuid::Uuid;

use super::{Pool, connection::map_sqlx_error};
use crate::random_uuid::random_uuid_v4;
use crate::{
    CourseRosterEntryState, IssueSupportRepairCapabilityInput, SessionTokenHash, StoreError,
    SupportRepairCapabilityReceipt, SupportRepairCapabilityStore,
    SupportRepairCapabilityUseReceipt, SupportRepairResourceClass,
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
impl SupportRepairCapabilityStore for PostgresSupportCapabilityStore {
    async fn issue_support_repair_capability(
        &self,
        token: SessionTokenHash,
        input: IssueSupportRepairCapabilityInput,
    ) -> Result<SupportRepairCapabilityReceipt, StoreError> {
        input.validate()?;
        let mut tx = self.begin(token).await?;
        let row = sqlx::query("SELECT capability_id, sysadmin_account_id, resource_class, resource_path, purpose, expires_at_millis, revoked_at_millis FROM ple_api.issue_support_repair_capability($1, $2, $3, $4, $5)")
            .bind(input.sysadmin_id.as_string())
            .bind(input.resource_class.database_name())
            .bind(&input.resource_path).bind(&input.purpose).bind(random_uuid()?)
            .fetch_optional(&mut *tx).await.map_err(map_sqlx_error)?
            .ok_or(StoreError::NotFound)?;
        let receipt = decode_repair(&row)?;
        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(receipt)
    }

    async fn revoke_support_repair_capability(
        &self,
        token: SessionTokenHash,
        capability_id: Uuid,
    ) -> Result<SupportRepairCapabilityReceipt, StoreError> {
        let mut tx = self.begin(token).await?;
        let row = sqlx::query("SELECT capability_id, sysadmin_account_id, resource_class, resource_path, purpose, expires_at_millis, revoked_at_millis FROM ple_api.revoke_support_repair_capability($1)")
            .bind(capability_id).fetch_optional(&mut *tx).await.map_err(map_sqlx_error)?
            .ok_or(StoreError::NotFound)?;
        let receipt = decode_repair(&row)?;
        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(receipt)
    }

    async fn record_support_repair_capability_use(
        &self,
        token: SessionTokenHash,
        capability_id: Uuid,
        resource_class: SupportRepairResourceClass,
        resource_path: String,
    ) -> Result<SupportRepairCapabilityUseReceipt, StoreError> {
        if resource_path != resource_path.trim()
            || !(1..=512).contains(&resource_path.chars().count())
            || resource_path.chars().any(char::is_control)
        {
            return Err(StoreError::InvalidRecord(
                "Support resource path is invalid".to_string(),
            ));
        }
        let mut tx = self.begin(token).await?;
        let row = sqlx::query("SELECT audit_event_id, capability_id, resource_class, resource_path, used_at_millis FROM ple_api.record_support_repair_capability_use($1, $2, $3)")
            .bind(capability_id).bind(resource_class.database_name()).bind(&resource_path)
            .fetch_optional(&mut *tx).await.map_err(map_sqlx_error)?
            .ok_or(StoreError::NotFound)?;
        let receipt = SupportRepairCapabilityUseReceipt {
            audit_event_id: row.try_get("audit_event_id").map_err(map_sqlx_error)?,
            capability_id: row.try_get("capability_id").map_err(map_sqlx_error)?,
            resource_class: decode_resource_class(
                row.try_get("resource_class").map_err(map_sqlx_error)?,
            )?,
            resource_path: row.try_get("resource_path").map_err(map_sqlx_error)?,
            used_at: Timestamp::from_unix_millis(
                row.try_get("used_at_millis").map_err(map_sqlx_error)?,
            ),
        };
        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(receipt)
    }

    async fn read_course_roster_entry_repair_support(
        &self,
        token: SessionTokenHash,
        capability_id: Uuid,
        course: CourseInstanceId,
        roster_id: String,
    ) -> Result<Option<crate::SupportCourseRosterEntry>, StoreError> {
        let mut tx = self.begin(token).await?;
        let row = sqlx::query(
            "SELECT roster_id, state FROM ple_api.read_course_roster_entry_repair_support($1, $2, $3)",
        )
        .bind(capability_id)
        .bind(course.as_string())
        .bind(roster_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_sqlx_error)?;
        let entry = row
            .map(|row| {
                let state: String = row.try_get("state").map_err(map_sqlx_error)?;
                let state = match state.as_str() {
                    "invitation_pending" => CourseRosterEntryState::InvitationPending,
                    "active_student" => CourseRosterEntryState::ActiveStudent,
                    _ => return Err(invalid("Course Roster state")),
                };
                Ok(crate::SupportCourseRosterEntry {
                    roster_id: row.try_get("roster_id").map_err(map_sqlx_error)?,
                    state,
                })
            })
            .transpose()?;
        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(entry)
    }
}
fn invalid(label: &str) -> StoreError {
    StoreError::InvalidRecord(format!("database returned an invalid {label}"))
}

fn decode_repair(
    row: &sqlx::postgres::PgRow,
) -> Result<SupportRepairCapabilityReceipt, StoreError> {
    let sysadmin_id = AccountId::new(
        row.try_get::<String, _>("sysadmin_account_id")
            .map_err(map_sqlx_error)?,
    )
    .map_err(|_| invalid("Sysadmin Account ID"))?;
    Ok(SupportRepairCapabilityReceipt {
        capability_id: row.try_get("capability_id").map_err(map_sqlx_error)?,
        sysadmin_id: sysadmin_id,
        resource_class: decode_resource_class(
            row.try_get("resource_class").map_err(map_sqlx_error)?,
        )?,
        resource_path: row.try_get("resource_path").map_err(map_sqlx_error)?,
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

fn decode_resource_class(value: String) -> Result<SupportRepairResourceClass, StoreError> {
    match value.as_str() {
        "student" => Ok(SupportRepairResourceClass::Student),
        _ => Err(invalid("Support repair resource class")),
    }
}

fn random_uuid() -> Result<Uuid, StoreError> {
    random_uuid_v4(|_| {
        StoreError::Unavailable("Support capability randomness unavailable".to_string())
    })
}
