//! PostgreSQL persistence for the authenticated Instructor thumbnail only.

use async_trait::async_trait;
use objects::Sha256Checksum;
use question_model::{ObjectId, ProfileThumbnailReference};
use sqlx::{Postgres, Row, Transaction};
use uuid::Uuid;

use super::{Pool, connection::map_sqlx_error};
use crate::{
    FinalizedProfileThumbnail, PreparedProfileThumbnail, ProfileThumbnailDeleteWork,
    ProfileThumbnailStore, SessionTokenHash, StoreError,
};

#[derive(Clone)]
/// PostgreSQL store for the authenticated Instructor's self-profile thumbnail.
pub struct PostgresProfileThumbnailStore {
    pool: Pool,
}

impl PostgresProfileThumbnailStore {
    #[must_use]
    /// Creates the store bound to the supplied PostgreSQL connection pool.
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
        let session = sqlx::query(
            "SELECT session_id FROM ple_api.resolve_and_install_session(decode($1, 'hex'))",
        )
        .bind(token.to_string())
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_sqlx_error)?;
        if session.is_none() {
            return Err(StoreError::Forbidden);
        }
        sqlx::query("SET LOCAL ROLE ple_app")
            .execute(&mut *tx)
            .await
            .map_err(map_sqlx_error)?;
        Ok(tx)
    }

    async fn accepted(
        &self,
        token: SessionTokenHash,
        operation: ThumbnailWorkOperation,
        work_id: Uuid,
    ) -> Result<(), StoreError> {
        let mut tx = self.begin(token).await?;
        let accepted = match operation {
            ThumbnailWorkOperation::CompletePut => sqlx::query_scalar::<_, bool>(
                "SELECT ple_api.complete_instructor_profile_thumbnail_put($1)",
            )
            .bind(work_id)
            .fetch_one(&mut *tx)
            .await
            .map_err(map_sqlx_error)?,
            ThumbnailWorkOperation::RequireRepair => sqlx::query_scalar::<_, bool>(
                "SELECT ple_api.require_instructor_profile_thumbnail_repair($1)",
            )
            .bind(work_id)
            .fetch_one(&mut *tx)
            .await
            .map_err(map_sqlx_error)?,
            ThumbnailWorkOperation::CompleteDelete => sqlx::query_scalar::<_, bool>(
                "SELECT ple_api.complete_instructor_profile_thumbnail_deletion($1)",
            )
            .bind(work_id)
            .fetch_one(&mut *tx)
            .await
            .map_err(map_sqlx_error)?,
            ThumbnailWorkOperation::RequireDeleteRepair => sqlx::query_scalar::<_, bool>(
                "SELECT ple_api.require_instructor_profile_thumbnail_deletion_repair($1)",
            )
            .bind(work_id)
            .fetch_one(&mut *tx)
            .await
            .map_err(map_sqlx_error)?,
        };
        if !accepted {
            return Err(StoreError::NotFound);
        }
        tx.commit().await.map_err(map_sqlx_error)
    }
}

#[derive(Clone, Copy)]
enum ThumbnailWorkOperation {
    CompletePut,
    RequireRepair,
    CompleteDelete,
    RequireDeleteRepair,
}

#[async_trait]
impl ProfileThumbnailStore for PostgresProfileThumbnailStore {
    async fn read_current_profile_thumbnail(
        &self,
        token: SessionTokenHash,
    ) -> Result<Option<ProfileThumbnailReference>, StoreError> {
        let mut tx = self.begin(token).await?;
        let reference = sqlx::query_scalar::<_, Option<Uuid>>(
            "SELECT ple_api.current_instructor_profile_thumbnail()",
        )
        .fetch_one(&mut *tx)
        .await
        .map_err(map_sqlx_error)?;
        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(reference.map(ProfileThumbnailReference::from_uuid))
    }
    async fn prepare_profile_thumbnail(
        &self,
        token: SessionTokenHash,
        reference: ProfileThumbnailReference,
        object_id: ObjectId,
        sha256: Sha256Checksum,
        byte_length: u64,
    ) -> Result<PreparedProfileThumbnail, StoreError> {
        let byte_length = i64::try_from(byte_length)
            .map_err(|_| StoreError::InvalidRecord("profile thumbnail is too large".to_string()))?;
        let mut tx = self.begin(token).await?;
        let work_id = sqlx::query_scalar::<_, Option<Uuid>>(
            "SELECT ple_api.prepare_instructor_profile_thumbnail($1,$2,$3,$4)",
        )
        .bind(reference.as_uuid())
        .bind(object_id.as_uuid())
        .bind(sha256.as_bytes().to_vec())
        .bind(byte_length)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_sqlx_error)?
        .ok_or(StoreError::NotFound)?;
        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(PreparedProfileThumbnail {
            work_id,
            reference,
            object_id,
        })
    }
    async fn complete_profile_thumbnail_put(
        &self,
        token: SessionTokenHash,
        work_id: Uuid,
    ) -> Result<(), StoreError> {
        self.accepted(token, ThumbnailWorkOperation::CompletePut, work_id)
            .await
    }
    async fn require_profile_thumbnail_repair(
        &self,
        token: SessionTokenHash,
        work_id: Uuid,
    ) -> Result<(), StoreError> {
        self.accepted(token, ThumbnailWorkOperation::RequireRepair, work_id)
            .await
    }
    async fn prepare_profile_thumbnail_deletion(
        &self,
        token: SessionTokenHash,
        put_work_id: Uuid,
    ) -> Result<ProfileThumbnailDeleteWork, StoreError> {
        let mut tx = self.begin(token).await?;
        let row = sqlx::query(
            "SELECT delete_work_id, profile_thumbnail_id \
             FROM ple_api.prepare_instructor_profile_thumbnail_deletion($1)",
        )
        .bind(put_work_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_sqlx_error)?
        .ok_or(StoreError::NotFound)?;
        let work_id = row.try_get("delete_work_id").map_err(map_sqlx_error)?;
        let reference = row
            .try_get("profile_thumbnail_id")
            .map(ProfileThumbnailReference::from_uuid)
            .map_err(map_sqlx_error)?;
        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(ProfileThumbnailDeleteWork { work_id, reference })
    }
    async fn complete_profile_thumbnail_deletion(
        &self,
        token: SessionTokenHash,
        delete_work: ProfileThumbnailDeleteWork,
    ) -> Result<(), StoreError> {
        self.accepted(
            token,
            ThumbnailWorkOperation::CompleteDelete,
            delete_work.work_id,
        )
        .await
    }
    async fn require_profile_thumbnail_deletion_repair(
        &self,
        token: SessionTokenHash,
        delete_work: ProfileThumbnailDeleteWork,
    ) -> Result<(), StoreError> {
        self.accepted(
            token,
            ThumbnailWorkOperation::RequireDeleteRepair,
            delete_work.work_id,
        )
        .await
    }
    async fn record_profile_thumbnail_cleanup_check(
        &self,
        token: SessionTokenHash,
        delete_work: ProfileThumbnailDeleteWork,
        object_present: bool,
        observed_checksum: Option<Sha256Checksum>,
    ) -> Result<(), StoreError> {
        let mut tx = self.begin(token).await?;
        let accepted = sqlx::query_scalar::<_, bool>(
            "SELECT ple_api.record_instructor_profile_thumbnail_cleanup_check($1,$2,$3)",
        )
        .bind(delete_work.work_id)
        .bind(object_present)
        .bind(observed_checksum.map(|checksum| checksum.as_bytes().to_vec()))
        .fetch_one(&mut *tx)
        .await
        .map_err(map_sqlx_error)?;
        if !accepted {
            return Err(StoreError::NotFound);
        }
        tx.commit().await.map_err(map_sqlx_error)
    }
    async fn finalize_profile_thumbnail(
        &self,
        token: SessionTokenHash,
        work_id: Uuid,
    ) -> Result<FinalizedProfileThumbnail, StoreError> {
        let mut tx = self.begin(token).await?;
        let row = sqlx::query(
            "SELECT profile_thumbnail_id, retired_delete_work_id, retired_profile_thumbnail_id \
             FROM ple_api.finalize_instructor_profile_thumbnail($1)",
        )
        .bind(work_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_sqlx_error)?
        .ok_or(StoreError::NotFound)?;
        let reference = row
            .try_get("profile_thumbnail_id")
            .map(ProfileThumbnailReference::from_uuid)
            .map_err(map_sqlx_error)?;
        let retired = match (
            row.try_get::<Option<Uuid>, _>("retired_delete_work_id")
                .map_err(map_sqlx_error)?,
            row.try_get::<Option<Uuid>, _>("retired_profile_thumbnail_id")
                .map_err(map_sqlx_error)?,
        ) {
            (Some(work_id), Some(reference)) => Some(ProfileThumbnailDeleteWork {
                work_id,
                reference: ProfileThumbnailReference::from_uuid(reference),
            }),
            (None, None) => None,
            _ => {
                return Err(StoreError::InvalidRecord(
                    "invalid retired thumbnail cleanup work".to_string(),
                ));
            }
        };
        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(FinalizedProfileThumbnail { reference, retired })
    }
    async fn resolve_current_profile_thumbnail(
        &self,
        token: SessionTokenHash,
        reference: ProfileThumbnailReference,
    ) -> Result<ObjectId, StoreError> {
        let mut tx = self.begin(token).await?;
        let object_id = sqlx::query_scalar::<_, Option<Uuid>>(
            "SELECT ple_api.resolve_current_instructor_profile_thumbnail($1)",
        )
        .bind(reference.as_uuid())
        .fetch_one(&mut *tx)
        .await
        .map_err(map_sqlx_error)?
        .ok_or(StoreError::NotFound)?;
        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(ObjectId::from_uuid(object_id))
    }
}
