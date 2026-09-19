//! Opaque identity for one self-owned Account Profile image.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Opaque role-neutral identity for one normalized Account Profile image.
///
/// It names neither an Account nor an object-storage path.  Authorization and
/// ownership remain server- and persistence-derived.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ProfileImageId(Uuid);

impl ProfileImageId {
    /// Rebuilds a trusted persistence identity.
    #[must_use]
    pub const fn from_uuid(value: Uuid) -> Self {
        Self(value)
    }

    /// Returns the opaque UUID at trusted persistence boundaries.
    #[must_use]
    pub const fn as_uuid(self) -> Uuid {
        self.0
    }

    /// Mints a new server-owned Profile-image identity.
    #[cfg(feature = "generate")]
    #[must_use]
    pub fn generate() -> Self {
        Self(Uuid::now_v7())
    }
}

impl std::fmt::Display for ProfileImageId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}", self.0)
    }
}
