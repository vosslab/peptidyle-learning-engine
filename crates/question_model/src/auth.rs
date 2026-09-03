//! Browser-safe global account identity and account roles.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// One global login account, distinct from a course enrollment's student record.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct AccountId(Uuid);

impl AccountId {
    /// Wraps an identity read from trusted authentication storage.
    pub fn from_uuid(value: Uuid) -> Self {
        Self(value)
    }

    /// Returns the UUID used by storage and logging.
    pub fn as_uuid(&self) -> Uuid {
        self.0
    }

    /// Mints a fresh server-owned account identifier.
    #[cfg(feature = "generate")]
    pub fn generate() -> Self {
        Self(Uuid::now_v7())
    }
}

impl std::fmt::Display for AccountId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}", self.0)
    }
}

/// The one immutable global role assigned to an account.
///
/// Course-specific permissions remain exact course relationships and do not
/// become global session roles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ProductRole {
    /// Completes assigned Questions and views personal results.
    Student,
    /// Authors content and manages courses and assignments.
    Instructor,
    /// Manages the platform and approves real-person instructor access.
    Sysadmin,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn account_identity_round_trips_without_becoming_a_student_identity() {
        let value = Uuid::from_u128(42);
        let account = AccountId::from_uuid(value);

        assert_eq!(account.as_uuid(), value);
        assert_eq!(account.to_string(), value.to_string());
    }

    #[test]
    fn roles_use_lower_camel_wire_names() {
        let encoded = serde_json::to_string(&[
            ProductRole::Student,
            ProductRole::Instructor,
            ProductRole::Sysadmin,
        ])
        .expect("roles should serialize");

        assert_eq!(encoded, "[\"student\",\"instructor\",\"sysadmin\"]");
    }
}
