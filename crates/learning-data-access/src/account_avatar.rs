//! Role-neutral, self-only Account avatar persistence contract.

use async_trait::async_trait;
use objects::{ObjectAddress, Sha256Checksum};
pub use question_model::ProfileImageReference;
use question_model::{ObjectId, avatar_catalog_generated::PROVIDED_AVATAR_CATALOG};
use serde::Serialize;
use uuid::Uuid;

use crate::{SessionTokenHash, StoreError};

/// Closed identifier for a PLE-provided avatar.
///
/// This mirrors the database identifier grammar so callers cannot pass an
/// arbitrary display label into the persistence boundary.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(transparent)]
pub struct ProvidedAvatarId(String);

impl ProvidedAvatarId {
    /// Validates a canonical provided-avatar identifier.
    pub fn parse(value: impl Into<String>) -> Result<Self, StoreError> {
        let value = value.into();
        let valid = !value.is_empty()
            && value.len() <= 64
            && value.bytes().enumerate().all(|(index, byte)| match byte {
                b'a'..=b'z' => true,
                b'0'..=b'9' | b'-' => index > 0,
                _ => false,
            });
        if valid {
            Ok(Self(value))
        } else {
            Err(StoreError::InvalidRecord(
                "provided avatar id must be 1-64 lowercase ASCII letters, digits, or hyphens and start with a letter".to_owned(),
            ))
        }
    }

    /// Returns the database identifier without exposing a storage location.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for ProvidedAvatarId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// A provided-avatar identifier that is selectable in the generated PLE catalog.
///
/// This is distinct from [`ProvidedAvatarId`]: reads must continue to decode an
/// already selected, now-retired catalog asset so historical profile rendering
/// remains possible. Only a new selection requires `is_selectable`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SelectableProvidedAvatarId(ProvidedAvatarId);

impl SelectableProvidedAvatarId {
    /// Positively validates a new selection against the generated catalog.
    ///
    /// ASVS 2.2.1/2.2.2: selection input is allow-listed at the trusted
    /// server/domain boundary; the database independently enforces the same
    /// current-catalog rule before persistence.
    pub fn parse(value: impl Into<String>) -> Result<Self, StoreError> {
        let id = ProvidedAvatarId::parse(value)?;
        if PROVIDED_AVATAR_CATALOG
            .iter()
            .any(|entry| entry.id == id.as_str() && entry.is_selectable)
        {
            Ok(Self(id))
        } else {
            Err(StoreError::InvalidRecord(
                "provided avatar is not selectable".to_owned(),
            ))
        }
    }

    /// Returns the canonical catalog identifier for the persistence boundary.
    #[must_use]
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

/// The one active, discriminated avatar choice for an Account.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AccountAvatar {
    /// A PLE-provided avatar; this is the only selection available to Students.
    Provided(ProvidedAvatarId),
    /// A finalized self-owned Profile image; only Instructor and Sysadmin flows can create it.
    ProfileImage(ProfileImageReference),
}

/// A prepared object-store put for the authenticated Account's next Profile image.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PreparedAccountProfileImage {
    pub work_id: Uuid,
    pub reference: ProfileImageReference,
    pub object_id: ObjectId,
}

impl PreparedAccountProfileImage {
    /// Returns the sole typed private object address for this prepared image.
    #[must_use]
    pub fn object_address(&self) -> ObjectAddress {
        ObjectAddress::ProfileImage {
            image: self.reference,
            object: self.object_id,
        }
    }
}

/// Exact durable authorization to remove one self-owned Profile-image object.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AccountProfileImageDeleteWork {
    pub work_id: Uuid,
    pub reference: ProfileImageReference,
    pub object_id: ObjectId,
}

impl AccountProfileImageDeleteWork {
    /// Returns the exact typed object address that this cleanup work may remove.
    #[must_use]
    pub fn object_address(&self) -> ObjectAddress {
        ObjectAddress::ProfileImage {
            image: self.reference,
            object: self.object_id,
        }
    }
}

/// The visible Profile image replacement and any retired self-owned cleanup work.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FinalizedAccountProfileImage {
    pub reference: ProfileImageReference,
    pub object_id: ObjectId,
    pub retired: Option<AccountProfileImageDeleteWork>,
}

impl FinalizedAccountProfileImage {
    /// Returns the exact typed address now made current for the Account.
    #[must_use]
    pub fn object_address(&self) -> ObjectAddress {
        ObjectAddress::ProfileImage {
            image: self.reference,
            object: self.object_id,
        }
    }
}

#[async_trait]
/// Durable self-only Account-avatar boundary.
///
/// The database derives the Account from `token`; callers never supply an
/// Account id. Image work is role-gated by the database and provided-avatar
/// selection is constrained to the closed PLE collection.
pub trait AccountAvatarGallery: Send + Sync {
    /// Returns the authenticated Account's selected avatar, if one exists.
    async fn read_current_account_avatar(
        &self,
        token: SessionTokenHash,
    ) -> Result<Option<AccountAvatar>, StoreError>;
    /// Selects a validated PLE-provided avatar for the authenticated Account.
    async fn select_provided_account_avatar(
        &self,
        token: SessionTokenHash,
        avatar_id: SelectableProvidedAvatarId,
    ) -> Result<(), StoreError>;
    /// Records the exact object expected from an Instructor or Sysadmin image upload.
    async fn prepare_account_profile_image(
        &self,
        token: SessionTokenHash,
        reference: ProfileImageReference,
        object_id: ObjectId,
        sha256: Sha256Checksum,
        byte_length: u64,
    ) -> Result<PreparedAccountProfileImage, StoreError>;
    /// Marks a prepared self-owned image put complete.
    async fn complete_account_profile_image_put(
        &self,
        token: SessionTokenHash,
        work_id: Uuid,
    ) -> Result<(), StoreError>;
    /// Marks a prepared image put for repair after its object-store result cannot be accepted.
    async fn require_account_profile_image_repair(
        &self,
        token: SessionTokenHash,
        work_id: Uuid,
    ) -> Result<(), StoreError>;
    /// Prepares exact deletion work for a completed image put that will not be finalized.
    async fn prepare_account_profile_image_deletion(
        &self,
        token: SessionTokenHash,
        put_work_id: Uuid,
    ) -> Result<AccountProfileImageDeleteWork, StoreError>;
    /// Records a successful object-store deletion for self-owned image work.
    async fn complete_account_profile_image_deletion(
        &self,
        token: SessionTokenHash,
        delete_work: AccountProfileImageDeleteWork,
    ) -> Result<(), StoreError>;
    /// Marks self-owned deletion work for repair.
    async fn require_account_profile_image_deletion_repair(
        &self,
        token: SessionTokenHash,
        delete_work: AccountProfileImageDeleteWork,
    ) -> Result<(), StoreError>;
    /// Records the exact object-store observation while repairing deletion work.
    async fn record_account_profile_image_cleanup_check(
        &self,
        token: SessionTokenHash,
        delete_work: AccountProfileImageDeleteWork,
        object_present: bool,
        observed_checksum: Option<Sha256Checksum>,
    ) -> Result<(), StoreError>;
    /// Makes a completed self-owned image current and returns retired cleanup work.
    async fn finalize_account_profile_image(
        &self,
        token: SessionTokenHash,
        work_id: Uuid,
    ) -> Result<FinalizedAccountProfileImage, StoreError>;
    /// Resolves an authorized current self-owned Profile image to its object identity.
    async fn resolve_current_account_profile_image(
        &self,
        token: SessionTokenHash,
        reference: ProfileImageReference,
    ) -> Result<ObjectId, StoreError>;
}
