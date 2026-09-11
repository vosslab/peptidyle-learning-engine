//! Self-only Instructor Profile thumbnail persistence contract.

use async_trait::async_trait;
use objects::Sha256Checksum;
use question_model::{ObjectId, ProfileThumbnailReference};
use uuid::Uuid;

use crate::{SessionTokenHash, StoreError};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// A prepared object-store put for the authenticated Instructor's next thumbnail.
///
/// The caller uploads `object_id`, then reports the result through `work_id`.
pub struct PreparedProfileThumbnail {
    pub work_id: Uuid,
    pub reference: ProfileThumbnailReference,
    pub object_id: ObjectId,
}

/// Exact durable authorization to remove one normalized thumbnail object.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProfileThumbnailDeleteWork {
    pub work_id: Uuid,
    pub reference: ProfileThumbnailReference,
}

/// The visible replacement and an optional retired object requiring cleanup.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FinalizedProfileThumbnail {
    pub reference: ProfileThumbnailReference,
    pub retired: Option<ProfileThumbnailDeleteWork>,
}

#[async_trait]
/// Durable self-only thumbnail saga boundary for authenticated Instructors.
///
/// Callers prepare a rendition, report its object-store result, finalize the
/// replacement, and perform any returned cleanup work in that order.
pub trait ProfileThumbnailStore: Send + Sync {
    /// Returns the authenticated Instructor's currently visible thumbnail, if any.
    async fn read_current_profile_thumbnail(
        &self,
        token: SessionTokenHash,
    ) -> Result<Option<ProfileThumbnailReference>, StoreError>;
    /// Records the exact rendition expected from the next object-store put.
    async fn prepare_profile_thumbnail(
        &self,
        token: SessionTokenHash,
        reference: ProfileThumbnailReference,
        object_id: ObjectId,
        sha256: Sha256Checksum,
        byte_length: u64,
    ) -> Result<PreparedProfileThumbnail, StoreError>;
    /// Marks a prepared put complete after the object store confirms that rendition.
    async fn complete_profile_thumbnail_put(
        &self,
        token: SessionTokenHash,
        work_id: Uuid,
    ) -> Result<(), StoreError>;
    /// Marks a prepared put for repair after its object-store result cannot be accepted.
    async fn require_profile_thumbnail_repair(
        &self,
        token: SessionTokenHash,
        work_id: Uuid,
    ) -> Result<(), StoreError>;
    /// Prepares exact deletion work for a completed put that cannot be finalized.
    ///
    /// Finalization prepares retired-thumbnail deletion itself.
    async fn prepare_profile_thumbnail_deletion(
        &self,
        token: SessionTokenHash,
        put_work_id: Uuid,
    ) -> Result<ProfileThumbnailDeleteWork, StoreError>;
    /// Records a successful object-store deletion for the prepared work.
    async fn complete_profile_thumbnail_deletion(
        &self,
        token: SessionTokenHash,
        delete_work: ProfileThumbnailDeleteWork,
    ) -> Result<(), StoreError>;
    /// Marks deletion work for repair after deletion cannot be confirmed.
    async fn require_profile_thumbnail_deletion_repair(
        &self,
        token: SessionTokenHash,
        delete_work: ProfileThumbnailDeleteWork,
    ) -> Result<(), StoreError>;
    /// Records the exact object-store observation made while repairing deletion work.
    async fn record_profile_thumbnail_cleanup_check(
        &self,
        token: SessionTokenHash,
        delete_work: ProfileThumbnailDeleteWork,
        object_present: bool,
        observed_checksum: Option<Sha256Checksum>,
    ) -> Result<(), StoreError>;
    /// Makes a completed put current and returns any retired thumbnail cleanup work.
    async fn finalize_profile_thumbnail(
        &self,
        token: SessionTokenHash,
        work_id: Uuid,
    ) -> Result<FinalizedProfileThumbnail, StoreError>;
    /// Resolves an authorized current thumbnail reference to its object identity.
    async fn resolve_current_profile_thumbnail(
        &self,
        token: SessionTokenHash,
        reference: ProfileThumbnailReference,
    ) -> Result<ObjectId, StoreError>;
}
