//! PostgreSQL immutable asset registration and protected delivery.

use async_trait::async_trait;
use question_model::{AssetId, CourseId, ObjectId, ProblemVersionRef, UserId};
use sqlx::Row;
use sqlx::postgres::PgRow;
use sqlx::types::Uuid;

use super::connection::map_sqlx_error;
use super::{PostgresStore, database_timestamp, decode_payload_row, encode_payload};
use crate::{
    AssetAccessEvent, AssetDeliveryId, AssetDeliveryRecord, AssetDeliveryScope, AssetStore,
    AuthorizedAssetDelivery, CatalogAssetBinding, StoreError, TenantContext, ensure_tenant,
    validate_asset_delivery,
};

#[async_trait]
impl AssetStore for PostgresStore {
    async fn register_asset_delivery(
        &self,
        context: TenantContext,
        record: AssetDeliveryRecord,
    ) -> Result<(), StoreError> {
        validate_asset_delivery(&record)?;
        let (payload, checksum) = encode_payload(&record)?;
        let (kind, tenant, course, problem, version, asset) = match &record.scope {
            AssetDeliveryScope::Catalog { asset, reference } => (
                "catalog",
                None,
                None,
                Some(reference.problem),
                Some(reference.version),
                Some(*asset),
            ),
            AssetDeliveryScope::StudentRecord { tenant, course, .. } => {
                ensure_tenant(context, *tenant)?;
                (
                    "student_record",
                    Some(*tenant),
                    Some(*course),
                    None,
                    None,
                    None,
                )
            }
            AssetDeliveryScope::CourseBanner { .. } => {
                return Err(StoreError::InvalidRecord(
                    "course-banner delivery registration is owned by appearance promotion"
                        .to_string(),
                ));
            }
        };
        let mut transaction = self.begin_tenant(context).await?;
        if let (Some(problem), Some(version)) = (problem, version) {
            let visible: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM problem_version \
                 WHERE problem_id = $1 AND version_id = $2)",
            )
            .bind(problem.as_uuid())
            .bind(version.as_uuid())
            .fetch_one(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
            if !visible {
                return Err(StoreError::NotFound);
            }
        }
        if let Some(course) = course {
            let accessible: bool =
                sqlx::query_scalar("SELECT public.ple_course_records_accessible($1, $2)")
                    .bind(context.tenant_id().as_uuid())
                    .bind(course.as_uuid())
                    .fetch_one(&mut *transaction)
                    .await
                    .map_err(map_sqlx_error)?;
            if !accessible {
                return Err(StoreError::NotFound);
            }
        }
        sqlx::query(
            "INSERT INTO asset_delivery \
             (delivery_id, delivery_kind, tenant_id, course_id, object_id, problem_id, version_id, \
              asset_id, payload, payload_sha256) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
        )
        .bind(record.id.as_uuid())
        .bind(kind)
        .bind(tenant.map(|value| value.as_uuid()))
        .bind(course.map(|value| value.as_uuid()))
        .bind(record.object.id.as_uuid())
        .bind(problem.map(|value| value.as_uuid()))
        .bind(version.map(|value| value.as_uuid()))
        .bind(asset.map(|value| value.as_uuid()))
        .bind(payload)
        .bind(checksum)
        .execute(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        transaction.commit().await.map_err(map_sqlx_error)
    }

    async fn get_public_asset_delivery(
        &self,
        delivery: AssetDeliveryId,
    ) -> Result<Option<AssetDeliveryRecord>, StoreError> {
        let mut transaction = self.begin_app().await?;
        let row = sqlx::query(
            "SELECT ad.payload, ad.payload_sha256 FROM asset_delivery AS ad \
             JOIN problem_version AS pv \
               ON pv.problem_id = ad.problem_id AND pv.version_id = ad.version_id \
             WHERE ad.delivery_id = $1 AND ad.delivery_kind = 'catalog' \
               AND pv.publication_scope = 'public'",
        )
        .bind(delivery.as_uuid())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let record = row.as_ref().map(decode_asset_delivery_row).transpose()?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(record)
    }

    async fn catalog_asset_bindings(
        &self,
        context: TenantContext,
        reference: ProblemVersionRef,
    ) -> Result<Vec<CatalogAssetBinding>, StoreError> {
        let mut transaction = self.begin_tenant(context).await?;
        let rows = sqlx::query(
            "SELECT asset_id, object_id FROM asset_delivery \
             WHERE delivery_kind = 'catalog' \
               AND problem_id = $1 AND version_id = $2 \
             ORDER BY asset_id ASC",
        )
        .bind(reference.problem.as_uuid())
        .bind(reference.version.as_uuid())
        .fetch_all(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let bindings = rows
            .iter()
            .map(|row| CatalogAssetBinding {
                asset: AssetId::from_uuid(row.get::<Uuid, _>("asset_id")),
                object: ObjectId::from_uuid(row.get::<Uuid, _>("object_id")),
            })
            .collect();
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(bindings)
    }

    async fn authorize_asset_delivery(
        &self,
        context: TenantContext,
        actor: UserId,
        delivery: AssetDeliveryId,
    ) -> Result<AuthorizedAssetDelivery, StoreError> {
        let mut transaction = self.begin_tenant(context).await?;
        let row = sqlx::query(
            "SELECT payload, payload_sha256, course_id FROM asset_delivery \
             WHERE delivery_id = $1",
        )
        .bind(delivery.as_uuid())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?
        .ok_or(StoreError::NotFound)?;
        let record = decode_asset_delivery_row(&row)?;
        let (scope_text, delivery_id, scope_course_id): (&str, Uuid, Option<Uuid>) =
            match &record.scope {
                AssetDeliveryScope::Catalog { .. } => ("catalog", record.id.as_uuid(), None),
                AssetDeliveryScope::StudentRecord {
                    tenant: scope_tenant,
                    course,
                    ..
                } => {
                    if *scope_tenant != context.tenant_id() {
                        return Err(StoreError::NotFound);
                    }
                    let object_course = row
                        .try_get::<Option<Uuid>, _>("course_id")
                        .map_err(map_sqlx_error)?;
                    if object_course != Some(course.as_uuid()) {
                        return Err(StoreError::NotFound);
                    }
                    (
                        "student_record",
                        record.id.as_uuid(),
                        Some(course.as_uuid()),
                    )
                }
                AssetDeliveryScope::CourseBanner { .. } => return Err(StoreError::NotFound),
            };
        if let AssetDeliveryScope::StudentRecord {
            tenant,
            course: _,
            authorized_users,
        } = &record.scope
            && (*tenant != context.tenant_id() || !authorized_users.contains(&actor))
        {
            return Err(StoreError::NotFound);
        }
        let authorized_at = database_timestamp(&mut transaction).await?;
        let event = AssetAccessEvent {
            tenant: context.tenant_id(),
            actor,
            delivery,
            object: record.object.id,
            bucket: record.object.bucket,
            course: scope_course_id.map(CourseId::from_uuid),
            occurred_at: authorized_at,
        };
        let (payload, checksum) = encode_payload(&event)?;
        sqlx::query(
            "INSERT INTO record_access_log \
             (tenant_id, access_log_id, occurred_at, payload, payload_sha256, \
              delivery_scope, delivery_id, course_id) \
             VALUES ($1, gen_random_uuid(), transaction_timestamp(), $2, $3, $4, $5, $6)",
        )
        .bind(context.tenant_id().as_uuid())
        .bind(payload)
        .bind(checksum)
        .bind(scope_text)
        .bind(delivery_id)
        .bind(scope_course_id)
        .execute(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(AuthorizedAssetDelivery {
            record,
            authorized_at,
        })
    }
}

fn decode_asset_delivery_row(row: &PgRow) -> Result<AssetDeliveryRecord, StoreError> {
    let record: AssetDeliveryRecord = decode_payload_row(row)?;
    validate_asset_delivery(&record).map_err(|error| {
        StoreError::Unavailable(format!("stored asset delivery is invalid: {error}"))
    })?;
    Ok(record)
}
