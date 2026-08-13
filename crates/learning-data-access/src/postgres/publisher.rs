//! Dedicated database capability for public-asset publication.
//!
//! This store deliberately does not wrap [`PostgresStore`]. Its login can enter
//! only `ple_public_asset_publisher`, which has no table privileges and only
//! four narrowly parameterized SECURITY DEFINER functions. Keeping the type
//! separate makes an accidental API or ordinary-worker query fail before it
//! can reach PostgreSQL.

use async_trait::async_trait;
use question_model::ProblemVersionRef;
use sqlx::Row;
use sqlx::types::{Json, Uuid};

use super::connection::map_sqlx_error;
use super::jobs::decode_claimed_job;
use super::{Pool, decode_payload_parts, encode_payload};
use crate::{
    AssetDeliveryRecord, EnqueueJob, JobClaimFilter, JobFailureDisposition, JobFailureKind, JobId,
    JobKind, JobLeaseDuration, JobLeaseToken, JobStore, QueueDepth, StoreError, TenantContext,
    TenantJobView,
};

/// PostgreSQL store held by the public-asset publisher process only.
#[derive(Clone)]
pub struct PostgresPublicAssetPublisherStore {
    pool: Pool,
}

impl PostgresPublicAssetPublisherStore {
    /// Wraps a pool whose login can assume only the publisher capability.
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }

    async fn begin(&self) -> Result<sqlx::Transaction<'_, sqlx::Postgres>, StoreError> {
        let mut transaction = self.pool.begin().await.map_err(map_sqlx_error)?;
        sqlx::query("SET LOCAL ROLE ple_public_asset_publisher")
            .execute(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        Ok(transaction)
    }

    fn publisher_filter(filter: &JobClaimFilter) -> Result<(), StoreError> {
        if filter.kinds().eq([JobKind::PublishPublicAssets]) {
            Ok(())
        } else {
            Err(StoreError::Forbidden)
        }
    }

    fn unsupported() -> StoreError {
        StoreError::Forbidden
    }
}

#[async_trait]
impl JobStore for PostgresPublicAssetPublisherStore {
    async fn enqueue_job(
        &self,
        _context: TenantContext,
        _job: EnqueueJob,
    ) -> Result<JobId, StoreError> {
        Err(Self::unsupported())
    }

    async fn claim_next_job(
        &self,
        filter: &JobClaimFilter,
        lease: JobLeaseDuration,
    ) -> Result<Option<crate::ClaimedJob>, StoreError> {
        Self::publisher_filter(filter)?;
        let token = JobLeaseToken::generate()?;
        let mut transaction = self.begin().await?;
        let row = sqlx::query(
            "SELECT job_id, tenant_id, payload, lease_token, attempt_count \
             FROM ple_claim_public_asset_publication_job($1, $2)",
        )
        .bind(token.as_uuid())
        .bind(lease.seconds())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let claimed = row
            .as_ref()
            .map(|row| decode_claimed_job(row, token))
            .transpose()?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(claimed)
    }

    async fn complete_job(&self, _id: JobId, _token: JobLeaseToken) -> Result<(), StoreError> {
        // Publication activation completes its exact lease atomically.
        Err(Self::unsupported())
    }

    async fn fail_job(
        &self,
        id: JobId,
        token: JobLeaseToken,
        failure: JobFailureKind,
    ) -> Result<JobFailureDisposition, StoreError> {
        let mut transaction = self.begin().await?;
        let disposition: Option<String> =
            sqlx::query_scalar("SELECT ple_fail_public_asset_publication_job($1, $2, $3)")
                .bind(id.as_uuid())
                .bind(token.as_uuid())
                .bind(failure.as_db())
                .fetch_one(&mut *transaction)
                .await
                .map_err(map_sqlx_error)?;
        let result = match disposition.as_deref() {
            Some("retrying") => JobFailureDisposition::Retrying,
            Some("dead") => JobFailureDisposition::Dead,
            None => return Err(StoreError::Conflict),
            Some(_) => {
                return Err(StoreError::Unavailable(
                    "publisher queue broker returned an unknown failure disposition".to_string(),
                ));
            }
        };
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(result)
    }

    async fn get_job(
        &self,
        _context: TenantContext,
        _id: JobId,
    ) -> Result<Option<TenantJobView>, StoreError> {
        Err(Self::unsupported())
    }

    async fn ready_queue_depth(&self, filter: &JobClaimFilter) -> Result<QueueDepth, StoreError> {
        Self::publisher_filter(filter)?;
        let mut transaction = self.begin().await?;
        let ready: i64 =
            sqlx::query_scalar("SELECT ple_ready_public_asset_publication_queue_depth()")
                .fetch_one(&mut *transaction)
                .await
                .map_err(map_sqlx_error)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(QueueDepth {
            ready: u64::try_from(ready).map_err(|_| {
                StoreError::Unavailable(
                    "publisher queue broker returned a negative depth".to_string(),
                )
            })?,
        })
    }
}

#[async_trait]
impl crate::PublicAssetPublicationStore for PostgresPublicAssetPublisherStore {
    async fn pending_public_asset_publication(
        &self,
        job: JobId,
        lease: JobLeaseToken,
        reference: ProblemVersionRef,
    ) -> Result<Vec<AssetDeliveryRecord>, StoreError> {
        let mut transaction = self.begin().await?;
        let rows = sqlx::query(
            "SELECT payload, payload_sha256 \
             FROM ple_read_pending_public_asset_publication($1, $2, $3, $4)",
        )
        .bind(job.as_uuid())
        .bind(lease.as_uuid())
        .bind(reference.problem.as_uuid())
        .bind(reference.version.as_uuid())
        .fetch_all(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let records = rows
            .iter()
            .map(|row| {
                let Json(payload) = row.try_get("payload").map_err(map_sqlx_error)?;
                let checksum: String = row.try_get("payload_sha256").map_err(map_sqlx_error)?;
                let record = decode_payload_parts(payload, checksum)?;
                crate::validate_asset_delivery(&record).map_err(|error| {
                    StoreError::Unavailable(format!(
                        "publisher registry delivery is invalid: {error}"
                    ))
                })?;
                Ok(record)
            })
            .collect::<Result<Vec<_>, StoreError>>()?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(records)
    }

    async fn activate_public_asset_publication(
        &self,
        job: JobId,
        lease: JobLeaseToken,
        reference: ProblemVersionRef,
    ) -> Result<(), StoreError> {
        let mut transaction = self.begin().await?;
        let rows = sqlx::query(
            "SELECT delivery_id, payload, payload_sha256 \
             FROM ple_read_pending_public_asset_publication($1, $2, $3, $4)",
        )
        .bind(job.as_uuid())
        .bind(lease.as_uuid())
        .bind(reference.problem.as_uuid())
        .bind(reference.version.as_uuid())
        .fetch_all(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let mut transitions = serde_json::Map::new();
        for row in rows {
            let delivery_id: Uuid = row.try_get("delivery_id").map_err(map_sqlx_error)?;
            let Json(payload) = row.try_get("payload").map_err(map_sqlx_error)?;
            let checksum: String = row.try_get("payload_sha256").map_err(map_sqlx_error)?;
            let mut record: AssetDeliveryRecord = decode_payload_parts(payload, checksum)?;
            crate::validate_asset_delivery(&record).map_err(|error| {
                StoreError::Unavailable(format!("publisher registry delivery is invalid: {error}"))
            })?;
            if record.publication != crate::AssetPublication::Pending
                || record.pending_source.is_none()
            {
                return Err(StoreError::Unavailable(
                    "publisher registry contains an invalid pending delivery".to_string(),
                ));
            }
            record.publication = crate::AssetPublication::Ready;
            record.pending_source = None;
            let (payload, checksum) = encode_payload(&record)?;
            transitions.insert(
                delivery_id.to_string(),
                serde_json::json!({ "payload": payload.0, "payloadSha256": checksum }),
            );
        }
        let activated: bool =
            sqlx::query_scalar("SELECT ple_activate_public_asset_publication($1, $2, $3, $4, $5)")
                .bind(job.as_uuid())
                .bind(lease.as_uuid())
                .bind(reference.problem.as_uuid())
                .bind(reference.version.as_uuid())
                .bind(serde_json::Value::Object(transitions))
                .fetch_one(&mut *transaction)
                .await
                .map_err(map_sqlx_error)?;
        if !activated {
            return Err(StoreError::Conflict);
        }
        transaction.commit().await.map_err(map_sqlx_error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn publisher_filter_refuses_every_non_publisher_family() {
        let allowed = JobClaimFilter::new([JobKind::PublishPublicAssets]).expect("filter");
        let denied = JobClaimFilter::new([JobKind::Export]).expect("filter");
        assert!(PostgresPublicAssetPublisherStore::publisher_filter(&allowed).is_ok());
        assert_eq!(
            PostgresPublicAssetPublisherStore::publisher_filter(&denied),
            Err(StoreError::Forbidden)
        );
    }
}
