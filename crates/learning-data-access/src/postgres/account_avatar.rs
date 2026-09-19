//! PostgreSQL persistence for the authenticated Account avatar only.

use async_trait::async_trait;
use objects::Sha256Checksum;
use question_model::ObjectId;
use sqlx::{Postgres, Row, Transaction};
use uuid::Uuid;

use super::{Pool, connection::map_sqlx_error};
use crate::{
    AccountAvatar, AccountAvatarGallery, AccountProfileImageDeleteWork,
    FinalizedAccountProfileImage, PreparedAccountProfileImage, ProfileImageId, ProvidedAvatarId,
    SelectableProvidedAvatarId, SessionTokenHash, StoreError,
};

#[derive(Clone)]
/// PostgreSQL gallery for the authenticated Account's avatar choice and image saga.
pub struct PostgresAccountAvatarGallery {
    pool: Pool,
}

impl PostgresAccountAvatarGallery {
    #[must_use]
    /// Creates the gallery bound to the supplied PostgreSQL connection pool.
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
        operation: ProfileImageWorkOperation,
        work_id: Uuid,
    ) -> Result<(), StoreError> {
        let mut tx = self.begin(token).await?;
        let accepted = match operation {
            ProfileImageWorkOperation::CompletePut => sqlx::query_scalar::<_, bool>(
                "SELECT ple_api.complete_account_profile_image_put($1)",
            )
            .bind(work_id)
            .fetch_one(&mut *tx)
            .await
            .map_err(map_sqlx_error)?,
            ProfileImageWorkOperation::RequireRepair => sqlx::query_scalar::<_, bool>(
                "SELECT ple_api.require_account_profile_image_repair($1)",
            )
            .bind(work_id)
            .fetch_one(&mut *tx)
            .await
            .map_err(map_sqlx_error)?,
            ProfileImageWorkOperation::CompleteDelete => sqlx::query_scalar::<_, bool>(
                "SELECT ple_api.complete_account_profile_image_deletion($1)",
            )
            .bind(work_id)
            .fetch_one(&mut *tx)
            .await
            .map_err(map_sqlx_error)?,
            ProfileImageWorkOperation::RequireDeleteRepair => sqlx::query_scalar::<_, bool>(
                "SELECT ple_api.require_account_profile_image_deletion_repair($1)",
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
enum ProfileImageWorkOperation {
    CompletePut,
    RequireRepair,
    CompleteDelete,
    RequireDeleteRepair,
}

#[async_trait]
impl AccountAvatarGallery for PostgresAccountAvatarGallery {
    async fn read_current_account_avatar(
        &self,
        token: SessionTokenHash,
    ) -> Result<Option<AccountAvatar>, StoreError> {
        let mut tx = self.begin(token).await?;
        let row = sqlx::query(
            "SELECT avatar_kind, provided_avatar_id, profile_image_id \
             FROM ple_api.current_account_avatar()",
        )
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_sqlx_error)?;
        tx.commit().await.map_err(map_sqlx_error)?;
        row.map(|row| {
            let avatar_kind: String = row.try_get("avatar_kind").map_err(map_sqlx_error)?;
            match avatar_kind.as_str() {
                "provided" => row
                    .try_get::<Option<String>, _>("provided_avatar_id")
                    .map_err(map_sqlx_error)?
                    .ok_or_else(|| {
                        StoreError::InvalidRecord("provided avatar is missing its id".to_owned())
                    })
                    .and_then(ProvidedAvatarId::parse)
                    .map(AccountAvatar::Provided),
                "profile-image" => row
                    .try_get::<Option<Uuid>, _>("profile_image_id")
                    .map_err(map_sqlx_error)?
                    .ok_or_else(|| {
                        StoreError::InvalidRecord(
                            "profile image avatar is missing its id".to_owned(),
                        )
                    })
                    .map(ProfileImageId::from_uuid)
                    .map(AccountAvatar::ProfileImage),
                _ => Err(StoreError::InvalidRecord(
                    "unknown account avatar kind".to_owned(),
                )),
            }
        })
        .transpose()
    }

    async fn select_provided_account_avatar(
        &self,
        token: SessionTokenHash,
        avatar_id: SelectableProvidedAvatarId,
    ) -> Result<(), StoreError> {
        let mut tx = self.begin(token).await?;
        let selected =
            sqlx::query_scalar::<_, bool>("SELECT ple_api.select_provided_account_avatar($1)")
                .bind(avatar_id.as_str())
                .fetch_one(&mut *tx)
                .await
                .map_err(map_sqlx_error)?;
        if !selected {
            return Err(StoreError::NotFound);
        }
        tx.commit().await.map_err(map_sqlx_error)
    }

    async fn prepare_account_profile_image(
        &self,
        token: SessionTokenHash,
        reference: ProfileImageId,
        object_id: ObjectId,
        sha256: Sha256Checksum,
        byte_length: u64,
    ) -> Result<PreparedAccountProfileImage, StoreError> {
        let byte_length = i64::try_from(byte_length)
            .map_err(|_| StoreError::InvalidRecord("profile image is too large".to_owned()))?;
        let mut tx = self.begin(token).await?;
        let row = sqlx::query(
            "SELECT work_id, profile_image_id, object_id \
             FROM ple_api.prepare_account_profile_image($1,$2,$3,$4)",
        )
        .bind(reference.as_uuid())
        .bind(object_id.as_uuid())
        .bind(sha256.as_bytes().to_vec())
        .bind(byte_length)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_sqlx_error)?
        .ok_or(StoreError::NotFound)?;
        let work_id = row.try_get("work_id").map_err(map_sqlx_error)?;
        let reference = row
            .try_get("profile_image_id")
            .map(ProfileImageId::from_uuid)
            .map_err(map_sqlx_error)?;
        let object_id = row
            .try_get("object_id")
            .map(ObjectId::from_uuid)
            .map_err(map_sqlx_error)?;
        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(PreparedAccountProfileImage {
            work_id,
            profile_image_id: reference,
            object_id,
        })
    }

    async fn complete_account_profile_image_put(
        &self,
        token: SessionTokenHash,
        work_id: Uuid,
    ) -> Result<(), StoreError> {
        self.accepted(token, ProfileImageWorkOperation::CompletePut, work_id)
            .await
    }

    async fn require_account_profile_image_repair(
        &self,
        token: SessionTokenHash,
        work_id: Uuid,
    ) -> Result<(), StoreError> {
        self.accepted(token, ProfileImageWorkOperation::RequireRepair, work_id)
            .await
    }

    async fn prepare_account_profile_image_deletion(
        &self,
        token: SessionTokenHash,
        put_work_id: Uuid,
    ) -> Result<AccountProfileImageDeleteWork, StoreError> {
        let mut tx = self.begin(token).await?;
        let row = sqlx::query(
            "SELECT delete_work_id, profile_image_id, object_id \
             FROM ple_api.prepare_account_profile_image_deletion($1)",
        )
        .bind(put_work_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_sqlx_error)?
        .ok_or(StoreError::NotFound)?;
        let work_id = row.try_get("delete_work_id").map_err(map_sqlx_error)?;
        let reference = row
            .try_get("profile_image_id")
            .map(ProfileImageId::from_uuid)
            .map_err(map_sqlx_error)?;
        let object_id = row
            .try_get("object_id")
            .map(ObjectId::from_uuid)
            .map_err(map_sqlx_error)?;
        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(AccountProfileImageDeleteWork {
            work_id,
            profile_image_id: reference,
            object_id,
        })
    }

    async fn complete_account_profile_image_deletion(
        &self,
        token: SessionTokenHash,
        delete_work: AccountProfileImageDeleteWork,
    ) -> Result<(), StoreError> {
        self.accepted(
            token,
            ProfileImageWorkOperation::CompleteDelete,
            delete_work.work_id,
        )
        .await
    }

    async fn require_account_profile_image_deletion_repair(
        &self,
        token: SessionTokenHash,
        delete_work: AccountProfileImageDeleteWork,
    ) -> Result<(), StoreError> {
        self.accepted(
            token,
            ProfileImageWorkOperation::RequireDeleteRepair,
            delete_work.work_id,
        )
        .await
    }

    async fn record_account_profile_image_cleanup_check(
        &self,
        token: SessionTokenHash,
        delete_work: AccountProfileImageDeleteWork,
        object_present: bool,
        observed_checksum: Option<Sha256Checksum>,
    ) -> Result<(), StoreError> {
        let mut tx = self.begin(token).await?;
        let accepted = sqlx::query_scalar::<_, bool>(
            "SELECT ple_api.record_account_profile_image_cleanup_check($1,$2,$3)",
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

    async fn finalize_account_profile_image(
        &self,
        token: SessionTokenHash,
        work_id: Uuid,
    ) -> Result<FinalizedAccountProfileImage, StoreError> {
        let mut tx = self.begin(token).await?;
        let row = sqlx::query(
            "SELECT profile_image_id, object_id, retired_delete_work_id, \
                    retired_profile_image_id, retired_object_id \
             FROM ple_api.finalize_account_profile_image($1)",
        )
        .bind(work_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_sqlx_error)?
        .ok_or(StoreError::NotFound)?;
        let reference = row
            .try_get("profile_image_id")
            .map(ProfileImageId::from_uuid)
            .map_err(map_sqlx_error)?;
        let object_id = row
            .try_get("object_id")
            .map(ObjectId::from_uuid)
            .map_err(map_sqlx_error)?;
        let retired = match (
            row.try_get::<Option<Uuid>, _>("retired_delete_work_id")
                .map_err(map_sqlx_error)?,
            row.try_get::<Option<Uuid>, _>("retired_profile_image_id")
                .map_err(map_sqlx_error)?,
            row.try_get::<Option<Uuid>, _>("retired_object_id")
                .map_err(map_sqlx_error)?,
        ) {
            (Some(work_id), Some(reference), Some(object_id)) => {
                Some(AccountProfileImageDeleteWork {
                    work_id,
                    profile_image_id: ProfileImageId::from_uuid(reference),
                    object_id: ObjectId::from_uuid(object_id),
                })
            }
            (None, None, None) => None,
            _ => {
                return Err(StoreError::InvalidRecord(
                    "invalid retired profile-image cleanup work".to_owned(),
                ));
            }
        };
        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(FinalizedAccountProfileImage {
            profile_image_id: reference,
            object_id,
            retired,
        })
    }

    async fn resolve_current_account_profile_image(
        &self,
        token: SessionTokenHash,
        reference: ProfileImageId,
    ) -> Result<ObjectId, StoreError> {
        let mut tx = self.begin(token).await?;
        let object_id = sqlx::query_scalar::<_, Option<Uuid>>(
            "SELECT ple_api.resolve_current_account_profile_image($1)",
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
