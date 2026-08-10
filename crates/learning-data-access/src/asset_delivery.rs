//! Immutable asset registration and protected-delivery authorization.

use async_trait::async_trait;
use objects::{Bucket, ObjectRecord};
use question_model::{
    ActivityTimestamp, AssetId, CourseBannerId, CourseId, ObjectId, ProblemVersionRef, TenantId,
    UserId,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{StoreError, TenantContext};

/// Opaque route identifier for either a logical catalog asset or a tenant object.
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

    /// Builds the route identifier for a tenant-owned physical artifact.
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
        /// Direct RLS boundary owning the artifact.
        tenant: TenantId,
        /// Exact course whose retention lifecycle governs this record.
        course: CourseId,
        /// Authenticated users allowed to request a short-lived URL.
        authorized_users: Vec<UserId>,
    },
    /// Tenant course presentation authorized only through the exact current pointer.
    CourseBanner {
        /// Direct RLS boundary owning the course.
        tenant: TenantId,
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
    /// Visibility and ownership linkage checked on every protected request.
    pub scope: AssetDeliveryScope,
}

/// Immutable logical-to-physical asset mapping for one published catalog version.
///
/// This is an internal storage result used while reproducing a server-issued
/// question attempt. It deliberately omits object metadata and delivery
/// authorization because neither belongs in browser question delivery.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CatalogAssetBinding {
    /// Browser-safe logical asset referenced by immutable authored content.
    pub asset: AssetId,
    /// Exact immutable object selected when this version was published.
    pub object: ObjectId,
}

/// Audit payload appended before a protected signed URL is requested.
///
/// It deliberately contains neither the signed URL nor session credentials.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetAccessEvent {
    /// Tenant in whose security context the request was authorized.
    pub tenant: TenantId,
    /// Authenticated person requesting the protected object.
    pub actor: UserId,
    /// Stable route identifier requested by the actor.
    pub delivery: AssetDeliveryId,
    /// Exact physical object whose URL may be issued.
    pub object: ObjectId,
    /// Bucket whose fixed delivery lifetime applies.
    pub bucket: Bucket,
    /// Course that authorized this delivery access, when visible in learner records.
    pub course: Option<CourseId>,
    /// Database-authoritative authorization time.
    pub occurred_at: ActivityTimestamp,
}

/// Protected object record and the timestamp used to bound its signed URL.
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
        context: TenantContext,
        record: AssetDeliveryRecord,
    ) -> Result<(), StoreError>;

    /// Resolves only globally public catalog content for direct CDN delivery.
    ///
    /// Institution content and every educational record deliberately look
    /// absent here so callers cannot bypass the authenticated path.
    async fn get_public_asset_delivery(
        &self,
        delivery: AssetDeliveryId,
    ) -> Result<Option<AssetDeliveryRecord>, StoreError>;

    /// Resolves every catalog asset registered for one exact visible version.
    ///
    /// The result is ordered by logical [`AssetId`] and intentionally excludes
    /// tenant-owned educational records. This lookup has no delivery audit or
    /// signed-URL side effect: it is solely the trusted bridge from immutable
    /// catalog content to provenance verification.
    async fn catalog_asset_bindings(
        &self,
        context: TenantContext,
        reference: ProblemVersionRef,
    ) -> Result<Vec<CatalogAssetBinding>, StoreError>;

    /// Authorizes one protected request and appends its audit event atomically.
    async fn authorize_asset_delivery(
        &self,
        context: TenantContext,
        actor: UserId,
        delivery: AssetDeliveryId,
    ) -> Result<AuthorizedAssetDelivery, StoreError>;
}
