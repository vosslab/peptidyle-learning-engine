//! Exact-course Sysadmin support-capability issuance and revocation.

use async_trait::async_trait;
use question_model::{AccountId, CourseInstanceId, Timestamp};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{CourseRosterEntryState, SessionTokenHash, StoreError};

/// Existing task-specific repair projection excludes ordinary Course roster names.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SupportCourseRosterEntry {
    pub roster_id: String,
    pub state: CourseRosterEntryState,
}

/// Only the implemented Course-local Student roster repair scope is supported.
/// SQL resolves an existing canonical profile and checks the original issuer's
/// current exact Course authority at issuance and use. Course/content are future work.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SupportRepairResourceClass {
    Student,
}

impl SupportRepairResourceClass {
    pub(crate) fn database_name(self) -> &'static str {
        match self {
            Self::Student => "student",
        }
    }
}

/// Rust bounds clean input; SQL resolves the exact roster scope and derives expiry.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct IssueSupportRepairCapabilityInput {
    pub sysadmin_id: AccountId,
    pub resource_class: SupportRepairResourceClass,
    pub resource_path: String,
    pub purpose: String,
}

impl IssueSupportRepairCapabilityInput {
    pub fn validate(&self) -> Result<(), StoreError> {
        for (label, value, maximum) in [
            ("Support resource path", &self.resource_path, 512),
            ("Support purpose", &self.purpose, 1_000),
        ] {
            if value != value.trim()
                || !(1..=maximum).contains(&value.chars().count())
                || value.chars().any(char::is_control)
            {
                return Err(StoreError::InvalidRecord(format!("{label} is invalid")));
            }
        }
        Ok(())
    }
}

/// Safe request receipt; it never contains the repaired resource's data.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SupportRepairCapabilityReceipt {
    pub capability_id: Uuid,
    pub sysadmin_id: AccountId,
    pub resource_class: SupportRepairResourceClass,
    pub resource_path: String,
    pub purpose: String,
    pub expires_at: Timestamp,
    pub revoked_at: Option<Timestamp>,
}

/// Immutable evidence that C26 consumed an exact, active capability.  It is
/// an audit receipt, not a projection of Course, Student, or content data.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SupportRepairCapabilityUseReceipt {
    pub audit_event_id: Uuid,
    pub capability_id: Uuid,
    pub resource_class: SupportRepairResourceClass,
    pub resource_path: String,
    pub used_at: Timestamp,
}

#[async_trait]
pub trait SupportRepairCapabilityStore: Send + Sync {
    async fn issue_support_repair_capability(
        &self,
        token: SessionTokenHash,
        input: IssueSupportRepairCapabilityInput,
    ) -> Result<SupportRepairCapabilityReceipt, StoreError>;
    async fn revoke_support_repair_capability(
        &self,
        token: SessionTokenHash,
        capability_id: Uuid,
    ) -> Result<SupportRepairCapabilityReceipt, StoreError>;
    async fn record_support_repair_capability_use(
        &self,
        token: SessionTokenHash,
        capability_id: Uuid,
        resource_class: SupportRepairResourceClass,
        resource_path: String,
    ) -> Result<SupportRepairCapabilityUseReceipt, StoreError>;
    /// Reads one named roster record for an active, exact-course repair
    /// capability.  This is deliberately not a generic or list reader.
    async fn read_course_roster_entry_repair_support(
        &self,
        token: SessionTokenHash,
        capability_id: Uuid,
        course_instance_id: CourseInstanceId,
        roster_id: String,
    ) -> Result<Option<SupportCourseRosterEntry>, StoreError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(resource_path: &str, purpose: &str) -> IssueSupportRepairCapabilityInput {
        IssueSupportRepairCapabilityInput {
            sysadmin_id: AccountId::from_random_identity("7K3M2QX").expect("valid Account ID"),
            resource_class: SupportRepairResourceClass::Student,
            resource_path: resource_path.to_owned(),
            purpose: purpose.to_owned(),
        }
    }

    #[test]
    fn support_repair_request_bounds_clean_fields_before_database_resolution() {
        assert!(
            input(
                "course-instance/CI7K3M2Q/roster/student-42",
                "Correct a roster mismatch"
            )
            .validate()
            .is_ok()
        );
        assert!(
            input(" resource", "Correct a roster mismatch")
                .validate()
                .is_err()
        );
        assert!(
            input("resource\n", "Correct a roster mismatch")
                .validate()
                .is_err()
        );
        assert!(
            input(&"x".repeat(513), "Correct a roster mismatch")
                .validate()
                .is_err()
        );
    }

    #[test]
    fn unsupported_repair_classes_fail_deserialization() {
        for class in ["course", "content"] {
            assert!(
                serde_json::from_str::<SupportRepairResourceClass>(&format!("\"{class}\""))
                    .is_err()
            );
        }
    }
}
