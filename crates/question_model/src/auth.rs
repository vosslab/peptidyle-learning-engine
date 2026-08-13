//! Browser-safe authenticated-user identity and authorization roles.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// One authenticated person, distinct from an assignment enrollment's student.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct UserId(Uuid);

impl UserId {
    /// Wraps an identity read from trusted authentication storage.
    pub fn from_uuid(value: Uuid) -> Self {
        Self(value)
    }

    /// Returns the UUID used by storage and logging.
    pub fn as_uuid(&self) -> Uuid {
        self.0
    }

    /// Mints a fresh server-owned user identifier.
    #[cfg(feature = "generate")]
    pub fn generate() -> Self {
        Self(Uuid::now_v7())
    }
}

impl std::fmt::Display for UserId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}", self.0)
    }
}

/// A coarse application role used for route authorization.
///
/// A user may carry more than one role. Course-specific permissions remain
/// tenant records and do not become global session roles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum UserRole {
    /// Works assigned problems and views personal results.
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
    fn user_identity_round_trips_without_becoming_a_student_identity() {
        let value = Uuid::from_u128(42);
        let user = UserId::from_uuid(value);

        assert_eq!(user.as_uuid(), value);
        assert_eq!(user.to_string(), value.to_string());
    }

    #[test]
    fn roles_use_lower_camel_wire_names() {
        let encoded =
            serde_json::to_string(&[UserRole::Student, UserRole::Instructor, UserRole::Sysadmin])
                .expect("roles should serialize");

        assert_eq!(encoded, "[\"student\",\"instructor\",\"sysadmin\"]");
    }
}
