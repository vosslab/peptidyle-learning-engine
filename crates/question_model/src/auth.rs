//! Browser-safe global account identity and account roles.

use serde::{Deserialize, Serialize};

/// One global login account, distinct from a course enrollment's student record.
///
/// This is the canonical public ID (`UXXXXXXXZ`), not a second identifier.
pub use crate::public_route::AccountId;

/// The one immutable global Product Role assigned to an Account.
///
/// Course-specific permissions remain exact course relationships and do not
/// become global session Product Roles.
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
        let account = AccountId::new("U00000009").expect("fixture is canonical");

        assert_eq!(account.as_str(), "U00000009");
        assert_eq!(account.to_string(), "U00000009");
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
