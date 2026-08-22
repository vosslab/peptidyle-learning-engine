//! Revisioned course appearance, banner promotion, and cleanup persistence.

use std::num::NonZeroU16;

use async_trait::async_trait;
use objects::{ObjectKey, ObjectRecord, Sha256Digest};
use question_model::{
    ActivityTimestamp, CourseAppearance, CourseAppearanceRevision, CourseAppearanceUpdate,
    CourseBannerCandidateId, CourseBannerId, CourseId,
};
use uuid::Uuid;

use crate::{AuthorizedAssetDelivery, SessionTokenHash, StoreError, TenantContext};

/// Exact normalized banner width owned by the server image contract.
pub const COURSE_BANNER_WIDTH: u32 = 1_200;
/// Exact normalized banner height owned by the server image contract.
pub const COURSE_BANNER_HEIGHT: u32 = 328;

/// Candidate metadata persisted only after normalized bytes exist.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegisterCourseBannerCandidate {
    /// Opaque candidate returned to the authorized browser.
    pub candidate: CourseBannerCandidateId,
    /// Exact temporary object metadata returned after storing normalized bytes.
    pub object: ObjectRecord,
    /// Deterministic future delivery identity for an immutable promoted copy.
    pub banner: CourseBannerId,
    /// Decoded normalized width, which must equal [`COURSE_BANNER_WIDTH`].
    pub width: u32,
    /// Decoded normalized height, which must equal [`COURSE_BANNER_HEIGHT`].
    pub height: u32,
    /// Backend-authoritative exclusive expiry.
    pub expires_at: ActivityTimestamp,
}

/// Server-only evidence needed to copy one authorized candidate to its hidden
/// immutable delivery identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CourseBannerPromotion {
    /// Exact candidate whose actor-bound row was revalidated.
    pub candidate: CourseBannerCandidateId,
    /// Server-private future delivery identity minted during upload.
    pub banner: CourseBannerId,
    /// Persisted checksum that candidate bytes must still match.
    pub sha256: Sha256Digest,
    /// Persisted byte count that candidate bytes must still match.
    pub size_bytes: u64,
}

/// Server-prepared atomic appearance mutation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SaveCourseAppearance {
    /// Exact current revision obtained by the author before editing.
    pub expected_revision: CourseAppearanceRevision,
    /// Complete browser-safe theme and banner action.
    pub update: CourseAppearanceUpdate,
    /// Bytes-first immutable copy for a replacement; absent for keep/remove.
    pub promoted_object: Option<ObjectRecord>,
}

/// Positive bounded cleanup claim size.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CourseBannerCleanupBatch(NonZeroU16);

impl CourseBannerCleanupBatch {
    /// Largest number of cleanup claims returned by one backend operation.
    pub const MAX: u16 = 100;

    /// Validates one bounded nonzero batch.
    pub fn new(value: u16) -> Option<Self> {
        (value <= Self::MAX)
            .then(|| NonZeroU16::new(value))
            .flatten()
            .map(Self)
    }

    /// Returns the bounded number of claims.
    pub fn get(self) -> u16 {
        self.0.get()
    }
}

/// Opaque lease protecting one two-phase object cleanup.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CourseBannerCleanupToken(Uuid);

impl CourseBannerCleanupToken {
    pub(crate) fn generate() -> Result<Self, StoreError> {
        crate::random_uuid::random_128_bits(|error| {
            StoreError::Unavailable(format!("banner cleanup token generation failed: {error}"))
        })
        .map(crate::random_uuid::uuid_storage_from_128_random_bits)
        .map(Self)
    }

    #[cfg(feature = "postgres")]
    pub(crate) fn from_uuid(value: Uuid) -> Self {
        Self(value)
    }
}

/// Exact typed objects selected for one cleanup attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CourseBannerCleanupClaim {
    /// Course whose candidate owns the selected objects.
    pub course: CourseId,
    /// Candidate whose persisted state owns all selected objects.
    pub candidate: CourseBannerCandidateId,
    /// Lease token required to commit after object deletion succeeds.
    pub token: CourseBannerCleanupToken,
    /// Temporary normalized object, absent after an earlier successful cleanup.
    pub candidate_object: Option<ObjectKey>,
    /// Unreferenced immutable promoted copy, never the exact current pointer.
    pub promoted_object: Option<ObjectKey>,
}

/// Appearance and course-banner persistence boundary.
#[async_trait]
pub trait CourseAppearanceStore: Send + Sync {
    /// Reads appearance for an active persisted instructor session, or for an active
    /// student membership while learner records remain accessible.
    async fn course_appearance(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        course: CourseId,
    ) -> Result<Option<CourseAppearance>, StoreError>;

    /// Persists one actor-bound normalized candidate after object bytes exist.
    async fn register_course_banner_candidate(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        course: CourseId,
        command: RegisterCourseBannerCandidate,
    ) -> Result<(), StoreError>;

    /// Resolves the hidden immutable identity for one active actor-owned
    /// candidate before the server copies and verifies its normalized bytes.
    async fn course_banner_promotion(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        course: CourseId,
        candidate: CourseBannerCandidateId,
    ) -> Result<CourseBannerPromotion, StoreError>;

    /// Applies one revisioned theme/banner mutation.
    ///
    /// For replacement, `promoted_object` is recorded even when the appearance
    /// revision is stale, keeping a bytes-first copy owned by cleanup rather than
    /// turning it into an untracked orphan.
    async fn save_course_appearance(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        course: CourseId,
        command: SaveCourseAppearance,
    ) -> Result<CourseAppearance, StoreError>;

    /// Authorizes only the exact current banner through persisted session,
    /// membership, retention, and pointer state.
    async fn authorize_course_banner_delivery(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        banner: CourseBannerId,
    ) -> Result<AuthorizedAssetDelivery, StoreError>;

    /// Claims expired candidate bytes and unreferenced promoted copies.
    async fn claim_course_banner_cleanup(
        &self,
        context: TenantContext,
        batch: CourseBannerCleanupBatch,
    ) -> Result<Vec<CourseBannerCleanupClaim>, StoreError>;

    /// Commits one claim only after its exact selected objects were deleted.
    async fn complete_course_banner_cleanup(
        &self,
        context: TenantContext,
        claim: CourseBannerCleanupClaim,
    ) -> Result<bool, StoreError>;
}
