//! Minimal command and receipt contracts for mutable Blueprint Drafts.
//!
//! Blueprint publication is the reusable-content boundary. Course Instances
//! and Assignments keep current state and retain only exact source provenance.

use std::num::NonZeroU64;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::{
    AccountId, AssignmentReference, BlueprintAssignmentReference, BlueprintCourseReference,
    BlueprintRevision, CreateBlueprintCourseContentInput, ReplaceBlueprintCourseContentInput,
    Timestamp,
};

/// One exact Blueprint Course and immutable Blueprint Revision pair.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct BlueprintRevisionReference {
    pub reference: BlueprintCourseReference,
    pub revision: BlueprintRevision,
}

/// Stable Blueprint Assignment provenance inside one exact Blueprint Revision.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct BlueprintAssignmentSource {
    pub blueprint_revision: BlueprintRevisionReference,
    pub blueprint_assignment_reference: BlueprintAssignmentReference,
}

impl BlueprintAssignmentSource {
    pub const fn new(
        blueprint_revision: BlueprintRevisionReference,
        blueprint_assignment_reference: BlueprintAssignmentReference,
    ) -> Self {
        Self {
            blueprint_revision,
            blueprint_assignment_reference,
        }
    }
}

/// Trusted SHA-256 checksum for one accepted request; never browser authority.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RequestChecksum([u8; 32]);

impl RequestChecksum {
    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    pub const fn into_bytes(self) -> [u8; 32] {
        self.0
    }
}

/// Positive compare-and-swap number for a private mutable Blueprint Draft.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct BlueprintDraftEditNumber(NonZeroU64);

impl BlueprintDraftEditNumber {
    pub const INITIAL: Self = Self(NonZeroU64::MIN);

    pub fn new(value: u64) -> Option<Self> {
        (value > 0 && value <= i64::MAX as u64).then_some(Self(NonZeroU64::new(value)?))
    }

    pub const fn value(self) -> u64 {
        self.0.get()
    }

    pub fn checked_next(self) -> Option<Self> {
        Self::new(self.value().checked_add(1)?)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BlueprintDraftEditNumberError;

impl std::fmt::Display for BlueprintDraftEditNumberError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("blueprint draft edit number must be a canonical positive decimal")
    }
}

impl std::error::Error for BlueprintDraftEditNumberError {}

impl FromStr for BlueprintDraftEditNumber {
    type Err = BlueprintDraftEditNumberError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        if value.is_empty()
            || value.starts_with('0')
            || !value.bytes().all(|byte| byte.is_ascii_digit())
        {
            return Err(BlueprintDraftEditNumberError);
        }
        value
            .parse::<u64>()
            .ok()
            .and_then(Self::new)
            .ok_or(BlueprintDraftEditNumberError)
    }
}

impl TryFrom<String> for BlueprintDraftEditNumber {
    type Error = BlueprintDraftEditNumberError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        value.parse()
    }
}

impl From<BlueprintDraftEditNumber> for String {
    fn from(value: BlueprintDraftEditNumber) -> Self {
        value.to_string()
    }
}

impl std::fmt::Display for BlueprintDraftEditNumber {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.value().fmt(formatter)
    }
}

/// Current stable-lineage availability. Exact revisions remain resolvable
/// when ordinary discovery is archived.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BlueprintAvailability {
    Available,
    Archived,
}

/// Positive compare-and-swap number for a Blueprint Course lineage's
/// availability transition, independent of its mutable draft edit number.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct BlueprintAvailabilityEditNumber(NonZeroU64);

impl BlueprintAvailabilityEditNumber {
    pub const INITIAL: Self = Self(NonZeroU64::MIN);

    pub fn new(value: u64) -> Option<Self> {
        (value > 0 && value <= i64::MAX as u64).then_some(Self(NonZeroU64::new(value)?))
    }

    pub const fn value(self) -> u64 {
        self.0.get()
    }

    pub fn checked_next(self) -> Option<Self> {
        Self::new(self.value().checked_add(1)?)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BlueprintAvailabilityEditNumberError;

impl std::fmt::Display for BlueprintAvailabilityEditNumberError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .write_str("blueprint availability edit number must be a canonical positive decimal")
    }
}

impl std::error::Error for BlueprintAvailabilityEditNumberError {}

impl FromStr for BlueprintAvailabilityEditNumber {
    type Err = BlueprintAvailabilityEditNumberError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        if value.is_empty()
            || value.starts_with('0')
            || !value.bytes().all(|byte| byte.is_ascii_digit())
        {
            return Err(BlueprintAvailabilityEditNumberError);
        }
        value
            .parse::<u64>()
            .ok()
            .and_then(Self::new)
            .ok_or(BlueprintAvailabilityEditNumberError)
    }
}

impl TryFrom<String> for BlueprintAvailabilityEditNumber {
    type Error = BlueprintAvailabilityEditNumberError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        value.parse()
    }
}

impl From<BlueprintAvailabilityEditNumber> for String {
    fn from(value: BlueprintAvailabilityEditNumber) -> Self {
        value.to_string()
    }
}

impl std::fmt::Display for BlueprintAvailabilityEditNumber {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.value().fmt(formatter)
    }
}

/// Immutable evidence for an availability transition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BlueprintAvailabilityEvent {
    pub blueprint: BlueprintCourseReference,
    pub actor: AccountId,
    pub availability: BlueprintAvailability,
    pub edit_number: BlueprintAvailabilityEditNumber,
    pub recorded_at: Timestamp,
}

/// A private editable Blueprint Draft. Creation deliberately has no revision.
#[derive(Debug, Clone, PartialEq)]
pub struct BlueprintDraft {
    pub blueprint: BlueprintCourseReference,
    pub edit_number: BlueprintDraftEditNumber,
    pub content: ReplaceBlueprintCourseContentInput,
}

/// Server-accepted creation of a lineage and private draft.
#[derive(Debug, Clone, PartialEq)]
pub struct CreateBlueprintDraftCommand {
    pub actor: AccountId,
    pub request_checksum: RequestChecksum,
    pub content: CreateBlueprintCourseContentInput,
}

/// Durable receipt for creation. It intentionally carries no revision reference.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CreateBlueprintDraftReceipt {
    pub blueprint: BlueprintCourseReference,
    pub draft_edit_number: BlueprintDraftEditNumber,
    pub actor: AccountId,
    pub request_checksum: RequestChecksum,
    pub accepted_at: Timestamp,
}

/// Compare-and-swap save of a private Blueprint Draft.
#[derive(Debug, Clone, PartialEq)]
pub struct SaveBlueprintDraftCommand {
    pub blueprint: BlueprintCourseReference,
    pub expected_edit_number: BlueprintDraftEditNumber,
    pub actor: AccountId,
    pub request_checksum: RequestChecksum,
    pub content: ReplaceBlueprintCourseContentInput,
}

/// An unchanged draft save keeps its edit number.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SaveBlueprintDraftReceipt {
    pub blueprint: BlueprintCourseReference,
    pub resulting_edit_number: BlueprintDraftEditNumber,
    pub changed: bool,
    pub actor: AccountId,
    pub request_checksum: RequestChecksum,
    pub accepted_at: Timestamp,
}

/// Explicit publication of the current private draft. Every deliberately
/// accepted publication creates one new immutable Blueprint Revision. Request
/// replay returns the receipt first accepted for its request checksum.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PublishBlueprintDraftCommand {
    pub blueprint: BlueprintCourseReference,
    pub expected_edit_number: BlueprintDraftEditNumber,
    pub actor: AccountId,
    pub request_checksum: RequestChecksum,
}

/// Immutable publication evidence for one deliberate publication.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PublishBlueprintDraftReceipt {
    pub blueprint_revision: BlueprintRevisionReference,
    pub actor: AccountId,
    pub request_checksum: RequestChecksum,
    pub accepted_at: Timestamp,
}

/// Provenance retained by a destination Assignment created from a Blueprint
/// Assignment. The destination is stable current Assignment state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BlueprintAssignmentImportReceipt {
    pub source: BlueprintAssignmentSource,
    pub destination_assignment: AssignmentReference,
    pub actor: AccountId,
    pub request_checksum: RequestChecksum,
    pub accepted_at: Timestamp,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        AccountId, BlueprintAvailability, BlueprintCourseReadAccess, BlueprintCourseSummaryView,
        BlueprintRevision, Timestamp,
    };
    use uuid::Uuid;

    #[test]
    fn draft_edit_numbers_are_qualified_cas_values() {
        let edit: BlueprintDraftEditNumber = "7".parse().expect("canonical edit number");
        assert_eq!(edit.checked_next(), BlueprintDraftEditNumber::new(8));
        for invalid in ["", "0", "07", "+7", "9223372036854775808"] {
            assert!(
                invalid.parse::<BlueprintDraftEditNumber>().is_err(),
                "{invalid}"
            );
        }
    }

    #[test]
    fn creation_has_a_private_draft_boundary_before_publication() {
        let blueprint = BlueprintCourseReference::new(12).expect("valid Blueprint Course");
        let actor = AccountId::from_uuid(Uuid::from_u128(13));
        let checksum = RequestChecksum::from_bytes([14; 32]);
        let created = CreateBlueprintDraftReceipt {
            blueprint,
            draft_edit_number: BlueprintDraftEditNumber::INITIAL,
            actor,
            request_checksum: checksum,
            accepted_at: Timestamp::from_unix_millis(15),
        };
        let before_publication = BlueprintCourseSummaryView {
            reference: blueprint,
            title: "Biochemistry Blueprint".to_string(),
            availability: BlueprintAvailability::Available,
            availability_edit_number: BlueprintAvailabilityEditNumber::INITIAL,
            latest_published_revision: None,
            read_access: BlueprintCourseReadAccess::BlueprintCourseOwner,
        };
        let published = PublishBlueprintDraftReceipt {
            blueprint_revision: BlueprintRevisionReference {
                reference: blueprint,
                revision: BlueprintRevision::INITIAL,
            },
            actor,
            request_checksum: checksum,
            accepted_at: Timestamp::from_unix_millis(16),
        };

        assert_eq!(created.blueprint, before_publication.reference);
        assert_eq!(created.draft_edit_number, BlueprintDraftEditNumber::INITIAL);
        assert_eq!(before_publication.latest_published_revision, None);
        assert_eq!(published.blueprint_revision.reference, created.blueprint);
        assert_eq!(
            published.blueprint_revision.revision,
            BlueprintRevision::INITIAL
        );
    }
}
