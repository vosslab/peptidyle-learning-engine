//! Session-authorized Course Banner persistence contracts.

use async_trait::async_trait;
use objects::{ObjectAddress, Sha256Checksum};
use question_model::{
    CourseBanner, CourseBannerReference, CourseBannerUpdate, CourseBannerUploadReference, CourseId,
};

use crate::{SessionTokenHash, StoreError};

/// Metadata returned only after a staged upload has passed its exact Course and
/// Account binding.  It contains no object-store path.
#[derive(Debug, Clone)]
pub struct ClaimedCourseBannerUpload {
    pub upload: CourseBannerUploadReference,
    /// The immutable, typed temporary address.  This deliberately is not an
    /// ObjectRecord: the database cannot truthfully invent an object-store
    /// creation timestamp before the external put succeeds.
    pub address: ObjectAddress,
    pub object_id: question_model::ObjectId,
    pub sha256: Sha256Checksum,
    pub byte_length: u64,
    pub canonical_media_type: String,
    pub put_work_id: uuid::Uuid,
    pub width: u32,
    pub height: u32,
}

/// Verified bytes which have not yet been put into object storage.
///
/// This is intentionally separate from [`ObjectRecord`].  A promotion first
/// persists this exact metadata and its deterministic object identity, then
/// the server performs the external immutable put.
#[derive(Debug, Clone)]
pub struct CourseBannerObjectMetadata {
    pub object_id: question_model::ObjectId,
    pub sha256: Sha256Checksum,
    pub byte_length: u64,
    pub media_type: String,
}

/// Staged upload address and its exact pre-put work identity.
#[derive(Debug, Clone)]
pub struct StagedCourseBannerUpload {
    pub address: ObjectAddress,
    pub put_work_id: uuid::Uuid,
}

/// Database-prepared, hidden promotion state.  The server uses only the typed
/// addresses and opaque work identities; it never derives a storage path.
#[derive(Debug, Clone)]
pub struct PreparedCourseBannerPromotion {
    pub banner: CourseBannerReference,
    pub source: ObjectAddress,
    pub hero: ObjectAddress,
    pub card: ObjectAddress,
    pub source_put_work_id: uuid::Uuid,
    pub hero_put_work_id: uuid::Uuid,
    pub card_put_work_id: uuid::Uuid,
}

/// All database facts that must be durable before the upload object put.
#[derive(Debug, Clone)]
pub struct StageCourseBannerUpload {
    pub course: CourseId,
    pub upload: CourseBannerUploadReference,
    pub metadata: CourseBannerObjectMetadata,
    pub width: u32,
    pub height: u32,
    pub expires_at_unix_millis: i64,
}

/// All database facts that must be durable before source/rendition puts.
#[derive(Debug, Clone)]
pub struct PrepareCourseBannerPromotion {
    pub course: CourseId,
    pub upload: CourseBannerUploadReference,
    pub banner: CourseBannerReference,
    pub update: CourseBannerUpdate,
    pub source: CourseBannerObjectMetadata,
    pub hero: CourseBannerObjectMetadata,
    pub card: CourseBannerObjectMetadata,
}

/// Retired current banner objects that must be deleted or placed in repair.
#[derive(Debug, Clone)]
pub struct PreparedCourseBannerRemoval {
    pub banner: CourseBannerReference,
    pub source: ObjectAddress,
    pub hero: ObjectAddress,
    pub card: ObjectAddress,
    pub source_put_work_id: uuid::Uuid,
    pub hero_put_work_id: uuid::Uuid,
    pub card_put_work_id: uuid::Uuid,
}

/// Exact durable authorization to perform one external deletion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CourseBannerDeleteWork {
    pub work_id: uuid::Uuid,
}

/// A real object-store observation used to enter the cleanup-manifest lineage.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CourseBannerStorageCheckResult {
    Verified,
    Missing,
    Mismatched,
}

/// Visible promotion result and the private objects now queued for cleanup.
#[derive(Debug, Clone)]
pub struct FinalizedCourseBannerPromotion {
    pub banner: CourseBanner,
    pub upload: ObjectAddress,
    pub upload_put_work_id: uuid::Uuid,
    pub retired: Option<PreparedCourseBannerRemoval>,
}

/// Database boundary for the Course Banner half of Course Appearance.
#[async_trait]
pub trait CourseBannerStore: Send + Sync {
    /// Persists the Account-and-Course-bound upload plus its temporary object
    /// subject and pending put work before the external put starts.
    async fn stage_course_banner_upload(
        &self,
        session_token_hash: SessionTokenHash,
        request: StageCourseBannerUpload,
    ) -> Result<StagedCourseBannerUpload, StoreError>;

    /// Creates one exact pending deletion work item from a completed or
    /// unresolved put work before the caller touches object storage.
    async fn prepare_course_banner_object_deletion(
        &self,
        session_token_hash: SessionTokenHash,
        put_work_id: uuid::Uuid,
    ) -> Result<CourseBannerDeleteWork, StoreError>;

    /// Records that the exact pre-staged upload put completed.  A failed or
    /// uncertain put is left pending/repairable rather than being forgotten.
    async fn finalize_course_banner_upload_stage(
        &self,
        session_token_hash: SessionTokenHash,
        course: CourseId,
        upload: CourseBannerUploadReference,
    ) -> Result<(), StoreError>;

    /// Creates a hidden source, fixed hero/card pending deliveries, and all
    /// associated pending work before any source or rendition external put.
    async fn prepare_course_banner_promotion(
        &self,
        session_token_hash: SessionTokenHash,
        request: PrepareCourseBannerPromotion,
    ) -> Result<PreparedCourseBannerPromotion, StoreError>;

    /// Marks one exact prepared external put as completed.  The object id is
    /// selected from server-owned prepared state, not supplied by the browser.
    async fn complete_prepared_course_banner_object(
        &self,
        session_token_hash: SessionTokenHash,
        course: CourseId,
        banner: CourseBannerReference,
        object_id: question_model::ObjectId,
    ) -> Result<(), StoreError>;

    /// Marks the exact unresolved object put/delete as repair-required.  It
    /// does not fabricate an object-storage check result.
    async fn require_course_banner_object_repair(
        &self,
        session_token_hash: SessionTokenHash,
        course: CourseId,
        banner: Option<CourseBannerReference>,
        object_id: question_model::ObjectId,
    ) -> Result<(), StoreError>;

    /// Records a confirmed deletion only for an exact pending delete work row.
    async fn complete_course_banner_object_deletion(
        &self,
        session_token_hash: SessionTokenHash,
        delete_work: CourseBannerDeleteWork,
    ) -> Result<(), StoreError>;

    /// Records an uncertain external deletion against its exact pre-delete
    /// work; an eventual worker performs the storage check and cleanup bridge.
    async fn require_course_banner_deletion_repair(
        &self,
        session_token_hash: SessionTokenHash,
        delete_work: CourseBannerDeleteWork,
    ) -> Result<(), StoreError>;

    async fn record_course_banner_cleanup_check(
        &self,
        session_token_hash: SessionTokenHash,
        delete_work: CourseBannerDeleteWork,
        object_present: bool,
        observed_checksum: Option<Sha256Checksum>,
    ) -> Result<(), StoreError>;

    /// Rechecks and locks the exact prepared state, makes both deliveries
    /// available together, then advances the course pointer.
    async fn finalize_course_banner_promotion(
        &self,
        session_token_hash: SessionTokenHash,
        course: CourseId,
        upload: CourseBannerUploadReference,
        banner: CourseBannerReference,
    ) -> Result<FinalizedCourseBannerPromotion, StoreError>;
    async fn read_current_course_banner(
        &self,
        session_token_hash: SessionTokenHash,
        course: CourseId,
    ) -> Result<Option<CourseBanner>, StoreError>;

    /// Resolves an opaque current banner reference only for an active member.
    async fn resolve_current_course_banner(
        &self,
        session_token_hash: SessionTokenHash,
        banner: CourseBannerReference,
    ) -> Result<CourseId, StoreError>;

    /// Reads a still-live, completed upload for the exact current Instructor.
    /// It does not consume the upload; only finalization consumes it after both
    /// delivery objects are complete.
    async fn read_staged_course_banner_upload(
        &self,
        session_token_hash: SessionTokenHash,
        course: CourseId,
        upload: CourseBannerUploadReference,
    ) -> Result<ClaimedCourseBannerUpload, StoreError>;

    /// Clears the current pointer and retires both deliveries before returning
    /// the typed objects that the server must delete or mark repair-required.
    async fn prepare_course_banner_removal(
        &self,
        session_token_hash: SessionTokenHash,
        course: CourseId,
    ) -> Result<PreparedCourseBannerRemoval, StoreError>;
}
