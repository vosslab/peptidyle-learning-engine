//! Browser-safe references for the authenticated Instructor profile thumbnail.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Opaque same-origin identity for one normalized profile thumbnail.
///
/// This identifies an authorized delivery, never a storage key or Account.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ProfileThumbnailReference(Uuid);

impl ProfileThumbnailReference {
    /// Rebuilds a reference returned by trusted persistence.
    #[must_use]
    pub const fn from_uuid(value: Uuid) -> Self {
        Self(value)
    }

    /// Returns the opaque UUID at server and persistence boundaries.
    #[must_use]
    pub const fn as_uuid(self) -> Uuid {
        self.0
    }

    /// Mints a new server-owned thumbnail identity.
    #[cfg(feature = "generate")]
    #[must_use]
    pub fn generate() -> Self {
        Self(Uuid::now_v7())
    }
}

impl std::fmt::Display for ProfileThumbnailReference {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}", self.0)
    }
}

/// Closed server-owned profile thumbnail rendition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProfileThumbnailRendition {
    /// A single centered lossless WebP square used everywhere Profile appears.
    Square,
}

impl ProfileThumbnailRendition {
    /// Fixed dimensions of the normalized still image.
    #[must_use]
    pub const fn dimensions(self) -> (u32, u32) {
        (256, 256)
    }
}
