//! Exact-course Sysadmin support-capability issuance and revocation.

use async_trait::async_trait;
use question_model::{AccountReference, CourseInstanceReference, Timestamp};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{CourseRosterEntry, SessionTokenHash, StoreError};

/// The only resource classes that an explicit support-repair request may name.
/// The reference stays opaque here: C26 resolves and enforces it at the
/// resource-owning boundary, never through a generic data reader.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SupportRepairResourceClass {
    Course,
    Student,
    Content,
}

impl SupportRepairResourceClass {
    pub(crate) fn database_name(self) -> &'static str {
        match self {
            Self::Course => "course",
            Self::Student => "student",
            Self::Content => "content",
        }
    }
}

/// Server validates the bounded opaque reference and derives the expiry.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct IssueSupportRepairCapabilityInput {
    pub sysadmin_reference: AccountReference,
    pub resource_class: SupportRepairResourceClass,
    pub resource_reference: String,
    pub purpose: String,
}

impl IssueSupportRepairCapabilityInput {
    pub fn validate(&self) -> Result<(), StoreError> {
        for (label, value, maximum) in [
            ("Support resource reference", &self.resource_reference, 512),
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
    pub sysadmin_reference: AccountReference,
    pub resource_class: SupportRepairResourceClass,
    pub resource_reference: String,
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
    pub resource_reference: String,
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
        resource_reference: String,
    ) -> Result<SupportRepairCapabilityUseReceipt, StoreError>;
    /// Reads one named roster record for an active, exact-course repair
    /// capability.  This is deliberately not a generic or list reader.
    async fn read_course_roster_entry_repair_support(
        &self,
        token: SessionTokenHash,
        capability_id: Uuid,
        course: CourseInstanceReference,
        roster_id: String,
    ) -> Result<Option<CourseRosterEntry>, StoreError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(reference: &str, purpose: &str) -> IssueSupportRepairCapabilityInput {
        IssueSupportRepairCapabilityInput {
            sysadmin_reference: AccountReference::new("U7K3M2Q").expect("valid reference"),
            resource_class: SupportRepairResourceClass::Student,
            resource_reference: reference.to_owned(),
            purpose: purpose.to_owned(),
        }
    }

    #[test]
    fn support_repair_request_accepts_only_bounded_clean_opaque_fields() {
        assert!(
            input("student-record:opaque-42", "Correct a roster mismatch")
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
}
