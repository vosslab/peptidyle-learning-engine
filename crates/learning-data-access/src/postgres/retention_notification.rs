//! PostgreSQL adapter for C847's dedicated notifier capability.

use async_trait::async_trait;
use question_model::Timestamp;
use sqlx::{Row, Transaction};
use uuid::Uuid;

use super::{Pool, connection::map_sqlx_error};
use crate::{
    ClaimedCourseRetentionNotification, CourseRetentionNotificationAction,
    CourseRetentionNotificationFailure, CourseRetentionNotificationStore, StoreError,
    VerifiedCourseRetentionNotificationDestination,
};

/// Binds an attested notifier pool to C847's four closed procedures only.
#[derive(Clone)]
pub struct PostgresCourseRetentionNotificationStore {
    pool: Pool,
}

impl PostgresCourseRetentionNotificationStore {
    /// Uses a pool whose sole database capability is the retention notifier.
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }

    async fn begin(&self) -> Result<Transaction<'_, sqlx::Postgres>, StoreError> {
        // ASVS 8.2.1: the attested login has direct EXECUTE on the four typed
        // procedures only.  It cannot SET a capability role or read receipts.
        self.pool.begin().await.map_err(map_sqlx_error)
    }
}

#[async_trait]
impl CourseRetentionNotificationStore for PostgresCourseRetentionNotificationStore {
    async fn claim_due_notification(
        &self,
        evaluated_at: Timestamp,
        lease_seconds: u16,
    ) -> Result<Option<ClaimedCourseRetentionNotification>, StoreError> {
        if !(30..=900).contains(&lease_seconds) {
            return Err(StoreError::InvalidRecord(
                "Course-retention notification lease is invalid".to_string(),
            ));
        }
        let mut transaction = self.begin().await?;
        let row = sqlx::query(
            "SELECT notification_id, action_kind, \
                    (extract(epoch FROM due_at) * 1000)::bigint AS due_at_millis, \
                    verified_destination, provider_idempotency_key, lease_token \
             FROM ple_api.claim_course_retention_notification(\
                    to_timestamp($1::double precision / 1000.0), $2)",
        )
        .bind(evaluated_at.as_unix_millis())
        .bind(i32::from(lease_seconds))
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        row.map(decode_claim).transpose()
    }

    async fn record_provider_acceptance(
        &self,
        notification_id: Uuid,
        lease_token: Uuid,
        provider_idempotency_key: Uuid,
        accepted_at: Timestamp,
    ) -> Result<bool, StoreError> {
        self.record_boolean(
            "SELECT ple_api.record_course_retention_notification_provider_acceptance(\
                $1, $2, $3, to_timestamp($4::double precision / 1000.0))",
            &[notification_id, lease_token, provider_idempotency_key],
            accepted_at,
        )
        .await
    }

    async fn record_delivered(
        &self,
        notification_id: Uuid,
        provider_idempotency_key: Uuid,
        delivered_at: Timestamp,
    ) -> Result<bool, StoreError> {
        let mut transaction = self.begin().await?;
        let recorded = sqlx::query_scalar(
            "SELECT ple_api.record_course_retention_notification_delivered(\
                $1, $2, to_timestamp($3::double precision / 1000.0))",
        )
        .bind(notification_id)
        .bind(provider_idempotency_key)
        .bind(delivered_at.as_unix_millis())
        .fetch_one(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(recorded)
    }

    async fn fail_before_acceptance(
        &self,
        notification_id: Uuid,
        lease_token: Uuid,
        failed_at: Timestamp,
        failure: CourseRetentionNotificationFailure,
    ) -> Result<bool, StoreError> {
        let mut transaction = self.begin().await?;
        let recorded = sqlx::query_scalar(
            "SELECT ple_api.fail_course_retention_notification_before_acceptance(\
                $1, $2, to_timestamp($3::double precision / 1000.0), $4)",
        )
        .bind(notification_id)
        .bind(lease_token)
        .bind(failed_at.as_unix_millis())
        .bind(failure_wire(failure))
        .fetch_one(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(recorded)
    }
}

impl PostgresCourseRetentionNotificationStore {
    async fn record_boolean(
        &self,
        statement: &'static str,
        identifiers: &[Uuid; 3],
        occurred_at: Timestamp,
    ) -> Result<bool, StoreError> {
        let mut transaction = self.begin().await?;
        let recorded = sqlx::query_scalar(statement)
            .bind(identifiers[0])
            .bind(identifiers[1])
            .bind(identifiers[2])
            .bind(occurred_at.as_unix_millis())
            .fetch_one(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(recorded)
    }
}

fn decode_claim(
    row: sqlx::postgres::PgRow,
) -> Result<ClaimedCourseRetentionNotification, StoreError> {
    let action = match row
        .try_get::<String, _>("action_kind")
        .map_err(map_sqlx_error)?
        .as_str()
    {
        "warn_inactive" => CourseRetentionNotificationAction::WarnInactive,
        "notify_archive" => CourseRetentionNotificationAction::NotifyArchive,
        _ => {
            return Err(StoreError::InvalidRecord(
                "Course-retention notification action is invalid".to_string(),
            ));
        }
    };
    let verified_destination = row
        .try_get::<String, _>("verified_destination")
        .map_err(map_sqlx_error)?;
    Ok(ClaimedCourseRetentionNotification {
        notification_id: row.try_get("notification_id").map_err(map_sqlx_error)?,
        action,
        due_at: Timestamp::from_unix_millis(row.try_get("due_at_millis").map_err(map_sqlx_error)?),
        verified_destination: VerifiedCourseRetentionNotificationDestination::from_verified(
            verified_destination,
        )?,
        provider_idempotency_key: row
            .try_get("provider_idempotency_key")
            .map_err(map_sqlx_error)?,
        lease_token: row.try_get("lease_token").map_err(map_sqlx_error)?,
    })
}

fn failure_wire(value: CourseRetentionNotificationFailure) -> &'static str {
    match value {
        CourseRetentionNotificationFailure::NotConfigured => "not_configured",
        CourseRetentionNotificationFailure::ProviderTransient => "provider_transient",
        CourseRetentionNotificationFailure::ProviderRejected => "provider_rejected",
    }
}
