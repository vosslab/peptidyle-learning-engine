//! PostgreSQL invitation-delivery API and broker capabilities.

use async_trait::async_trait;
use question_model::{ActivityTimestamp, CourseId, TenantId};
use sqlx::Row;

use super::course_roster::precheck_course_roster_authority;
use super::{PostgresInvitationDeliveryWorkerStore, PostgresStore, map_sqlx_error};
use crate::{
    ClaimedCourseInvitationDelivery, CompleteCourseInvitationDelivery, CourseInvitationDelivery,
    CourseInvitationDeliveryId, CourseInvitationDeliveryLeaseId,
    CourseInvitationDeliveryOutcomeCode, CourseInvitationDeliveryState,
    CourseInvitationDeliveryStore, CourseInvitationDeliveryWorkerStore, CourseInvitationId,
    CourseInvitationSecretHash, CourseRosterId, InvitationDeliveryReissuance,
    PreparedCourseInvitationDelivery, RosterIdempotencyKey, SessionTokenHash, StoreError,
    TenantContext,
};

#[async_trait]
impl CourseInvitationDeliveryStore for PostgresStore {
    async fn course_invitation_delivery_state(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        course: CourseId,
        invitation: CourseInvitationId,
    ) -> Result<Option<CourseInvitationDeliveryState>, StoreError> {
        let mut transaction = self.begin_tenant(context).await?;
        precheck_course_roster_authority(&mut transaction, session, course).await?;
        let state = sqlx::query_scalar::<_, String>(
            "SELECT state FROM course_invitation_delivery \
             WHERE tenant_id = $1 AND course_id = $2 AND invitation_id = $3 \
               AND public.ple_course_records_accessible(tenant_id, course_id)",
        )
        .bind(context.tenant_id().as_uuid())
        .bind(course.as_uuid())
        .bind(invitation.as_uuid())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        state.map(|value| decode_state(&value)).transpose()
    }
}

#[async_trait]
impl CourseInvitationDeliveryWorkerStore for PostgresInvitationDeliveryWorkerStore {
    async fn prepare_course_invitation_delivery(
        &self,
        delivery: CourseInvitationDeliveryId,
        lease: CourseInvitationDeliveryLeaseId,
    ) -> Result<Option<PreparedCourseInvitationDelivery>, StoreError> {
        let mut transaction = self.begin_delivery_worker().await?;
        let row = sqlx::query(
            "SELECT tenant_id, course_id, delivery_id, lease_id, delivery_email, token_hash, roster_id, idempotency_key, \
                    roster_import_id, roster_import_row_number, commit_idempotency_key \
             FROM public.ple_prepare_course_invitation_delivery($1, $2)",
        )
        .bind(delivery.as_uuid())
        .bind(lease.as_uuid())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        row.map(|row| decode_prepared(&row)).transpose()
    }

    async fn claim_due_course_invitation_deliveries(
        &self,
        maximum: u16,
        lease_duration_seconds: u32,
    ) -> Result<Vec<ClaimedCourseInvitationDelivery>, StoreError> {
        let mut transaction = self.begin_delivery_worker().await?;
        let rows = sqlx::query(
            "SELECT tenant_id, course_id, invitation_id, delivery_id, state, attempt_count, \
                    floor(extract(epoch FROM next_attempt_at) * 1000)::bigint AS next_attempt_at_millis, \
                    floor(extract(epoch FROM last_attempt_at) * 1000)::bigint AS last_attempt_at_millis, \
                    lease_id, floor(extract(epoch FROM lease_expires_at) * 1000)::bigint AS lease_expires_at_millis, \
                    floor(extract(epoch FROM dispatch_started_at) * 1000)::bigint AS dispatch_started_at_millis, \
                    outcome_code, floor(extract(epoch FROM created_at) * 1000)::bigint AS created_at_millis, \
                    floor(extract(epoch FROM updated_at) * 1000)::bigint AS updated_at_millis, \
                    floor(extract(epoch FROM accepted_at) * 1000)::bigint AS accepted_at_millis, \
                    floor(extract(epoch FROM terminal_at) * 1000)::bigint AS terminal_at_millis \
             FROM public.ple_claim_course_invitation_deliveries($1, $2)",
        )
        .bind(i32::from(maximum))
        .bind(i32::try_from(lease_duration_seconds).map_err(|_| StoreError::Conflict)?)
        .fetch_all(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        rows.into_iter().map(decode_claimed).collect()
    }

    async fn complete_course_invitation_delivery(
        &self,
        delivery: CourseInvitationDeliveryId,
        lease: CourseInvitationDeliveryLeaseId,
        completion: CompleteCourseInvitationDelivery,
    ) -> Result<bool, StoreError> {
        let (state, next_attempt_at) = match completion {
            CompleteCourseInvitationDelivery::AcceptedByProvider => ("accepted_by_provider", None),
            CompleteCourseInvitationDelivery::RetryableFailed { next_attempt_at } => {
                ("retryable_failed", Some(next_attempt_at))
            }
            CompleteCourseInvitationDelivery::Ambiguous => ("ambiguous", None),
            CompleteCourseInvitationDelivery::PermanentFailed => ("permanent_failed", None),
        };
        let mut transaction = self.begin_delivery_worker().await?;
        let result = sqlx::query_scalar::<_, bool>(
            "SELECT public.ple_complete_course_invitation_delivery( \
                $1, $2, $3, CASE WHEN $4::bigint IS NULL THEN NULL \
                ELSE to_timestamp($4::double precision / 1000.0) END)",
        )
        .bind(delivery.as_uuid())
        .bind(lease.as_uuid())
        .bind(state)
        .bind(next_attempt_at.map(|timestamp| timestamp.as_unix_millis()))
        .fetch_one(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(result)
    }

    async fn revalidate_course_invitation_delivery_lease(
        &self,
        delivery: CourseInvitationDeliveryId,
        lease: CourseInvitationDeliveryLeaseId,
    ) -> Result<bool, StoreError> {
        let mut transaction = self.begin_delivery_worker().await?;
        let valid = sqlx::query_scalar::<_, bool>(
            "SELECT public.ple_revalidate_course_invitation_delivery_lease($1, $2)",
        )
        .bind(delivery.as_uuid())
        .bind(lease.as_uuid())
        .fetch_one(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(valid)
    }
}

fn decode_prepared(
    row: &sqlx::postgres::PgRow,
) -> Result<PreparedCourseInvitationDelivery, StoreError> {
    let delivery =
        CourseInvitationDeliveryId::from_uuid(row.try_get("delivery_id").map_err(map_sqlx_error)?);
    let lease = CourseInvitationDeliveryLeaseId::from_uuid(
        row.try_get("lease_id").map_err(map_sqlx_error)?,
    );
    let hash: Vec<u8> = row.try_get("token_hash").map_err(map_sqlx_error)?;
    let expected_token_hash =
        CourseInvitationSecretHash::from_bytes(hash.try_into().map_err(|_| {
            StoreError::Unavailable("stored invitation token hash is invalid".to_string())
        })?);
    let tenant = TenantId::from_uuid(row.try_get("tenant_id").map_err(map_sqlx_error)?);
    let course = CourseId::from_uuid(row.try_get("course_id").map_err(map_sqlx_error)?);
    let reissuance = match row
        .try_get::<Option<uuid::Uuid>, _>("roster_import_id")
        .map_err(map_sqlx_error)?
    {
        Some(import) => InvitationDeliveryReissuance::Import {
            tenant,
            course,
            import: crate::CourseRosterImportId::from_uuid(import),
            row_number: u16::try_from(
                row.try_get::<i32, _>("roster_import_row_number")
                    .map_err(map_sqlx_error)?,
            )
            .map_err(|_| {
                StoreError::Unavailable("stored invitation import row is invalid".to_string())
            })?,
            commit_idempotency_key: RosterIdempotencyKey::parse(
                &row.try_get::<String, _>("commit_idempotency_key")
                    .map_err(map_sqlx_error)?,
            )
            .map_err(|error| StoreError::Unavailable(error.to_string()))?,
        },
        None => InvitationDeliveryReissuance::Single {
            tenant,
            course,
            roster_id: CourseRosterId::parse(
                &row.try_get::<String, _>("roster_id")
                    .map_err(map_sqlx_error)?,
            )
            .map_err(|error| StoreError::Unavailable(error.to_string()))?,
            idempotency_key: RosterIdempotencyKey::parse(
                &row.try_get::<String, _>("idempotency_key")
                    .map_err(map_sqlx_error)?,
            )
            .map_err(|error| StoreError::Unavailable(error.to_string()))?,
        },
    };
    Ok(PreparedCourseInvitationDelivery {
        delivery,
        lease,
        delivery_email: row.try_get("delivery_email").map_err(map_sqlx_error)?,
        expected_token_hash,
        reissuance,
    })
}

fn decode_claimed(
    row: sqlx::postgres::PgRow,
) -> Result<ClaimedCourseInvitationDelivery, StoreError> {
    let tenant = TenantId::from_uuid(row.try_get("tenant_id").map_err(map_sqlx_error)?);
    let course = CourseId::from_uuid(row.try_get("course_id").map_err(map_sqlx_error)?);
    let invitation =
        CourseInvitationId::from_uuid(row.try_get("invitation_id").map_err(map_sqlx_error)?);
    let delivery = decode_delivery(&row, tenant, course, invitation)?;
    let lease = delivery
        .lease
        .ok_or_else(|| StoreError::Unavailable("claimed delivery has no lease".to_string()))?;
    Ok(ClaimedCourseInvitationDelivery { delivery, lease })
}

fn decode_delivery(
    row: &sqlx::postgres::PgRow,
    tenant: TenantId,
    course: CourseId,
    invitation: CourseInvitationId,
) -> Result<CourseInvitationDelivery, StoreError> {
    Ok(CourseInvitationDelivery {
        tenant,
        course,
        invitation,
        id: CourseInvitationDeliveryId::from_uuid(
            row.try_get("delivery_id").map_err(map_sqlx_error)?,
        ),
        state: decode_state(&row.try_get::<String, _>("state").map_err(map_sqlx_error)?)?,
        attempt_count: u32::try_from(
            row.try_get::<i32, _>("attempt_count")
                .map_err(map_sqlx_error)?,
        )
        .map_err(|_| {
            StoreError::Unavailable(
                "stored invitation delivery attempt count is invalid".to_string(),
            )
        })?,
        next_attempt_at: millis(row, "next_attempt_at_millis")?,
        last_attempt_at: optional_millis(row, "last_attempt_at_millis")?,
        lease: row
            .try_get::<Option<uuid::Uuid>, _>("lease_id")
            .map_err(map_sqlx_error)?
            .map(CourseInvitationDeliveryLeaseId::from_uuid),
        lease_expires_at: optional_millis(row, "lease_expires_at_millis")?,
        dispatch_started_at: optional_millis(row, "dispatch_started_at_millis")?,
        outcome_code: row
            .try_get::<Option<String>, _>("outcome_code")
            .map_err(map_sqlx_error)?
            .map(|value| decode_outcome(&value))
            .transpose()?,
        created_at: millis(row, "created_at_millis")?,
        updated_at: millis(row, "updated_at_millis")?,
        accepted_at: optional_millis(row, "accepted_at_millis")?,
        terminal_at: optional_millis(row, "terminal_at_millis")?,
    })
}

fn millis(row: &sqlx::postgres::PgRow, column: &str) -> Result<ActivityTimestamp, StoreError> {
    Ok(ActivityTimestamp::from_unix_millis(
        row.try_get(column).map_err(map_sqlx_error)?,
    ))
}
fn optional_millis(
    row: &sqlx::postgres::PgRow,
    column: &str,
) -> Result<Option<ActivityTimestamp>, StoreError> {
    row.try_get::<Option<i64>, _>(column)
        .map(|value| value.map(ActivityTimestamp::from_unix_millis))
        .map_err(map_sqlx_error)
}
fn decode_state(value: &str) -> Result<CourseInvitationDeliveryState, StoreError> {
    match value {
        "pending" => Ok(CourseInvitationDeliveryState::Pending),
        "accepted_by_provider" => Ok(CourseInvitationDeliveryState::AcceptedByProvider),
        "retryable_failed" => Ok(CourseInvitationDeliveryState::RetryableFailed),
        "ambiguous" => Ok(CourseInvitationDeliveryState::Ambiguous),
        "permanent_failed" => Ok(CourseInvitationDeliveryState::PermanentFailed),
        "cancelled" => Ok(CourseInvitationDeliveryState::Cancelled),
        _ => Err(StoreError::Unavailable(
            "stored invitation delivery state is invalid".to_string(),
        )),
    }
}
fn decode_outcome(value: &str) -> Result<CourseInvitationDeliveryOutcomeCode, StoreError> {
    match value {
        "accepted" => Ok(CourseInvitationDeliveryOutcomeCode::Accepted),
        "temporary_failure" => Ok(CourseInvitationDeliveryOutcomeCode::TemporaryFailure),
        "permanent_failure" => Ok(CourseInvitationDeliveryOutcomeCode::PermanentFailure),
        "ambiguous_transport" => Ok(CourseInvitationDeliveryOutcomeCode::AmbiguousTransport),
        "cancelled" => Ok(CourseInvitationDeliveryOutcomeCode::Cancelled),
        _ => Err(StoreError::Unavailable(
            "stored invitation delivery outcome is invalid".to_string(),
        )),
    }
}
