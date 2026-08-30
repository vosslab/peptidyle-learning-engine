//! Immutable asset registration and protected-delivery authorization.

use async_trait::async_trait;
use objects::{Bucket, ObjectKey, ObjectRecord, Sha256Digest};
use question_model::{
    ActivityTimestamp, AssetId, CourseBannerId, CourseId, ObjectId, ProblemVersionRef,
    PublicationScope, UserId,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{ActorContext, StoreError};

/// Opaque route identifier for either a logical catalog asset or a course record.
///
/// The identifier is never minted independently: public content reuses its
/// [`AssetId`], and a student-record artifact reuses its [`ObjectId`]. That
/// lets one stable `/api/assets/{id}` route serve both classes without
/// collapsing their distinct model identities.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct AssetDeliveryId(Uuid);

impl AssetDeliveryId {
    /// Builds the route identifier for a logical catalog asset.
    pub fn from_asset(asset: AssetId) -> Self {
        Self(asset.as_uuid())
    }

    /// Builds the route identifier for a course-owned physical artifact.
    pub fn from_object(object: ObjectId) -> Self {
        Self(object.as_uuid())
    }

    /// Builds the route identifier for one immutable course banner.
    pub fn from_course_banner(banner: CourseBannerId) -> Self {
        Self(banner.as_uuid())
    }

    /// Returns the storage UUID.
    pub fn as_uuid(&self) -> Uuid {
        self.0
    }
}

impl std::fmt::Display for AssetDeliveryId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}", self.0)
    }
}

impl std::str::FromStr for AssetDeliveryId {
    type Err = uuid::Error;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Uuid::parse_str(value).map(Self)
    }
}

/// Authorization linkage stored beside one immutable object record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum AssetDeliveryScope {
    /// Content asset whose current visibility comes from its published version.
    Catalog {
        /// Logical asset embedded in browser-safe question markup.
        asset: AssetId,
        /// Exact immutable version owning the asset.
        reference: ProblemVersionRef,
    },
    /// Educational-record artifact visible only to explicitly named users.
    StudentRecord {
        /// Exact course whose retention lifecycle governs this record.
        course: CourseId,
        /// Authenticated users allowed to request a short-lived URL.
        authorized_users: Vec<UserId>,
    },
    /// Course presentation authorized only through the exact current pointer.
    CourseBanner {
        /// Course whose current appearance may select this banner.
        course: CourseId,
        /// Browser-safe identity which must equal the route delivery ID.
        banner: CourseBannerId,
    },
}

/// Database-authoritative mapping from a route ID to exact stored bytes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetDeliveryRecord {
    /// Stable identifier accepted by `/api/assets/{id}`.
    pub id: AssetDeliveryId,
    /// Immutable metadata returned after object bytes were written.
    pub object: ObjectRecord,
    /// Measured raster dimensions when this immutable rendition is an image.
    ///
    /// The values are registered with the exact delivery record rather than
    /// inferred from object storage during issuance or receipt replay.
    #[serde(default)]
    pub intrinsic_width: Option<u32>,
    /// Measured raster dimensions paired with [`Self::intrinsic_width`].
    #[serde(default)]
    pub intrinsic_height: Option<u32>,
    /// Visibility and ownership linkage checked on every protected request.
    pub scope: AssetDeliveryScope,
    /// Database-authoritative public-publication state. A catalog record can
    /// name its final immutable CDN key before that key exists, but the key is
    /// never delivered until the committed publisher marks it ready.
    #[serde(default)]
    pub publication: AssetPublication,
    /// Private immutable source selected while validating publication. It is
    /// retained only while [`AssetPublication::Pending`] so a worker can copy
    /// exact verified bytes after the catalog transaction commits. This is an
    /// internal persistence field, never a browser delivery value.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pending_source: Option<ObjectRecord>,
}

/// Readiness of a catalog asset whose bytes are published asynchronously.
///
/// This deliberately belongs in the durable registry rather than object-store
/// metadata: CDN visibility must follow a committed catalog decision, and an
/// object store cannot participate in that transaction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum AssetPublication {
    /// Bytes are already present at the registered immutable key.
    #[default]
    Ready,
    /// The committed publisher job has not yet materialized the public key.
    Pending,
}

/// Immutable logical-to-physical asset mapping for one published catalog version.
///
/// This is an internal storage result used while reproducing a server-issued
/// question attempt. It deliberately omits object metadata and delivery
/// authorization because neither belongs in browser question delivery.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatalogAssetBinding {
    /// Browser-safe logical asset referenced by immutable authored content.
    pub asset: AssetId,
    /// Exact immutable object selected when this version was published.
    pub object: ObjectId,
    /// Exact immutable storage key selected by the published catalog record.
    ///
    /// The trusted catalog registry, rather than a later storage probe, owns
    /// the public-versus-restricted domain decision.
    pub key: ObjectKey,
    /// SHA-256 of the immutable rendition selected at publication.
    pub rendition_checksum: Sha256Digest,
    /// Media type registered for the immutable rendition.
    pub media_type: String,
    /// Measured raster width, present only for image deliveries.
    pub intrinsic_width: Option<u32>,
    /// Measured raster height, paired with [`Self::intrinsic_width`].
    pub intrinsic_height: Option<u32>,
}

/// Confirms that a catalog delivery uses the one storage domain implied by
/// the immutable publication scope.
///
/// This is intentionally a registry invariant, not a caller convention:
/// public catalog content is the sole class allowed in `PublicAssets`, while
/// institution content remains in `PrivateContent` behind authorization.
pub(crate) fn validate_catalog_asset_delivery_scope(
    record: &AssetDeliveryRecord,
    publication_scope: PublicationScope,
) -> Result<(), StoreError> {
    let AssetDeliveryScope::Catalog { .. } = record.scope else {
        return Ok(());
    };
    let expected = match publication_scope {
        PublicationScope::Public => "public catalog content must use a ProblemAsset key",
        PublicationScope::Institution => {
            "institution catalog content must use a RestrictedProblemAsset key"
        }
    };
    let matches_scope = matches!(
        (publication_scope, &record.object.key),
        (PublicationScope::Public, ObjectKey::ProblemAsset { .. })
            | (
                PublicationScope::Institution,
                ObjectKey::RestrictedProblemAsset { .. }
            )
    );
    if matches_scope {
        Ok(())
    } else {
        Err(StoreError::InvalidRecord(expected.to_string()))
    }
}

/// Audit payload appended before a protected delivery is requested.
///
/// It deliberately contains neither delivery bytes, URLs, nor session credentials.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetAccessEvent {
    /// Authenticated person requesting the protected object.
    pub actor: UserId,
    /// Stable route identifier requested by the actor.
    pub delivery: AssetDeliveryId,
    /// Exact physical object selected for delivery.
    pub object: ObjectId,
    /// Bucket containing the selected object.
    pub bucket: Bucket,
    /// Course that authorized this delivery access, when visible in learner records.
    pub course: Option<CourseId>,
    /// Database-authoritative authorization time.
    pub occurred_at: ActivityTimestamp,
}

/// Protected object record and its database-authoritative authorization time.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthorizedAssetDelivery {
    /// Exact immutable object record selected by the database registry.
    pub record: AssetDeliveryRecord,
    /// Database-authoritative time already captured in the access audit.
    pub authorized_at: ActivityTimestamp,
}

/// Immutable asset registry and protected-delivery authorization boundary.
#[async_trait]
pub trait AssetStore: Send + Sync {
    /// Records metadata only after the owning workflow has stored object bytes.
    async fn register_asset_delivery(
        &self,
        context: ActorContext,
        record: AssetDeliveryRecord,
    ) -> Result<(), StoreError>;

    /// Resolves only globally public catalog content for direct CDN delivery.
    ///
    /// Course content and every educational record deliberately look
    /// absent here so callers cannot bypass the authenticated path.
    async fn get_public_asset_delivery(
        &self,
        delivery: AssetDeliveryId,
    ) -> Result<Option<AssetDeliveryRecord>, StoreError>;

    /// Resolves every catalog asset registered for one exact visible version.
    ///
    /// The result is ordered by logical [`AssetId`] and intentionally excludes
    /// course-owned educational records. This lookup has no delivery audit or
    /// signed-URL side effect: it is solely the trusted bridge from immutable
    /// catalog content to provenance verification.
    async fn catalog_asset_bindings(
        &self,
        context: ActorContext,
        reference: ProblemVersionRef,
    ) -> Result<Vec<CatalogAssetBinding>, StoreError>;

    /// Authorizes one protected request and appends its audit event atomically.
    async fn authorize_asset_delivery(
        &self,
        context: ActorContext,
        actor: UserId,
        delivery: AssetDeliveryId,
    ) -> Result<AuthorizedAssetDelivery, StoreError>;
}

/// Durable boundary for the post-commit public-asset publisher.
///
/// The worker receives a version reference only, then re-resolves both the
/// final `ProblemAsset` keys and their private immutable sources from this
/// registry. `activate_public_asset_publication` must change every pending
/// record and complete the exact queue lease in one database transaction.
#[async_trait]
pub trait PublicAssetPublicationStore: Send + Sync {
    /// Resolves pending public assets only after the catalog publication has
    /// committed and only through the exact active publisher lease. An
    /// private course version has no entries in this outbox.
    async fn pending_public_asset_publication(
        &self,
        job: crate::JobId,
        lease: crate::JobLeaseToken,
        reference: ProblemVersionRef,
    ) -> Result<Vec<AssetDeliveryRecord>, StoreError>;

    /// Atomically activates the prepared immutable assets and completes the
    /// exact publisher job. A stale/reclaimed lease must fail closed.
    async fn activate_public_asset_publication(
        &self,
        job: crate::JobId,
        lease: crate::JobLeaseToken,
        reference: ProblemVersionRef,
    ) -> Result<(), StoreError>;
}
