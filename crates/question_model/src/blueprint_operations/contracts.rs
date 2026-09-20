//! Commands and receipts for immutable Blueprint Revisions and lineage metadata.

use std::str::FromStr;

use crate::{
    AccountId, AssessmentId, BlueprintAssessmentId, BlueprintCourseId, BlueprintRevisionNumber,
    Timestamp,
};
use serde::{Deserialize, Serialize};

/// One exact Blueprint Course and immutable Blueprint Revision pair.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BlueprintRevisionTuple {
    pub blueprint_course_id: BlueprintCourseId,
    pub revision_number: BlueprintRevisionNumber,
}

/// Stable Blueprint Assessment provenance inside one exact Blueprint Revision.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct BlueprintAssessmentSource {
    pub blueprint_revision_tuple: BlueprintRevisionTuple,
    pub blueprint_assessment_id: BlueprintAssessmentId,
}

impl BlueprintAssessmentSource {
    pub const fn new(
        blueprint_revision_tuple: BlueprintRevisionTuple,
        blueprint_assessment_id: BlueprintAssessmentId,
    ) -> Self {
        Self {
            blueprint_revision_tuple,
            blueprint_assessment_id,
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

/// Current stable-lineage lifecycle state. Private Blueprints are visible only
/// to their owner; Public Blueprints are eligible for discovery and adoption;
/// Archived Blueprints are unavailable for new selection. Exact Revisions
/// remain resolvable after either later transition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BlueprintAvailability {
    Private,
    Public,
    Archived,
}

/// Concurrency token for Blueprint Course names and availability.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct BlueprintEditNumber(i64);

impl BlueprintEditNumber {
    /// Wraps a server-generated Edit Number.
    pub const fn from_edit_number(value: i64) -> Self {
        Self(value)
    }

    /// Returns the Edit Number used for compare-and-set.
    pub const fn as_i64(self) -> i64 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BlueprintEditNumberError;

impl std::fmt::Display for BlueprintEditNumberError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("Blueprint metadata Edit Number must be a positive Edit Number")
    }
}

impl std::error::Error for BlueprintEditNumberError {}

impl FromStr for BlueprintEditNumber {
    type Err = BlueprintEditNumberError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let parsed = value.parse::<i64>().map_err(|_| BlueprintEditNumberError)?;
        (parsed > 0 && parsed.to_string() == value)
            .then_some(Self(parsed))
            .ok_or(BlueprintEditNumberError)
    }
}

impl TryFrom<String> for BlueprintEditNumber {
    type Error = BlueprintEditNumberError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        value.parse()
    }
}

impl From<BlueprintEditNumber> for String {
    fn from(value: BlueprintEditNumber) -> Self {
        value.to_string()
    }
}

impl std::fmt::Display for BlueprintEditNumber {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}", self.0)
    }
}

/// Browser input for one lineage-name change.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct RenameBlueprintCourseInput {
    pub short_name: String,
    pub long_name: String,
}

/// Current lineage metadata after an accepted metadata action.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct BlueprintMetadataState {
    pub classification: crate::CourseClassification,
    pub short_name: String,
    pub long_name: String,
    pub availability: BlueprintAvailability,
    pub blueprint_edit_number: BlueprintEditNumber,
}

/// Durable receipt for atomic lineage and Revision 1 creation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateBlueprintCourseReceipt {
    pub blueprint_revision_tuple: BlueprintRevisionTuple,
    pub blueprint_edit_number: BlueprintEditNumber,
    pub actor: AccountId,
    pub request_checksum: RequestChecksum,
    pub accepted_at: Timestamp,
}

/// Durable receipt for a changed or canonical no-op Save.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SaveBlueprintCourseReceipt {
    pub blueprint_revision_tuple: BlueprintRevisionTuple,
    pub changed: bool,
    pub actor: AccountId,
    pub request_checksum: RequestChecksum,
    pub accepted_at: Timestamp,
}

/// Provenance retained by a destination Assessment created from a Blueprint
/// Assessment. The destination is stable current Assessment state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlueprintAssessmentImportReceipt {
    pub source: BlueprintAssessmentSource,
    pub destination_assessment_id: AssessmentId,
    pub actor: AccountId,
    pub request_checksum: RequestChecksum,
    pub accepted_at: Timestamp,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blueprint_edit_numbers_are_canonical_opaque_values() {
        let value = "7";
        let edit_number: BlueprintEditNumber =
            value.parse().expect("canonical Blueprint Edit Number");
        assert_eq!(edit_number.to_string(), value);
        for invalid in ["", "07", "0", "00000000-0000-0000-0000-000000000007"] {
            assert!(invalid.parse::<BlueprintEditNumber>().is_err(), "{invalid}");
        }
    }

    #[test]
    fn creation_receipt_identifies_revision_one() {
        let blueprint = BlueprintCourseId::new("BP7K3M2QXH").expect("valid Blueprint Course");
        let receipt = CreateBlueprintCourseReceipt {
            blueprint_revision_tuple: BlueprintRevisionTuple {
                blueprint_course_id: blueprint.clone(),
                revision_number: BlueprintRevisionNumber::INITIAL,
            },
            blueprint_edit_number: BlueprintEditNumber::from_edit_number(13),
            actor: AccountId::from_debug_serial(14),
            request_checksum: RequestChecksum::from_bytes([15; 32]),
            accepted_at: Timestamp::from_unix_millis(16),
        };

        assert_eq!(
            receipt.blueprint_revision_tuple.blueprint_course_id,
            blueprint
        );
        assert_eq!(
            receipt.blueprint_revision_tuple.revision_number,
            BlueprintRevisionNumber::INITIAL
        );
    }

    #[test]
    fn blueprint_revision_tuple_round_trips_course_id_and_revision_number() {
        let tuple = BlueprintRevisionTuple {
            blueprint_course_id: BlueprintCourseId::new("BP7K3M2QXH")
                .expect("valid Blueprint Course"),
            revision_number: BlueprintRevisionNumber::INITIAL,
        };
        let json = serde_json::to_value(&tuple).expect("tuple serializes");
        assert_eq!(json["blueprintCourseId"], "BP7K3M2QXH");
        assert_eq!(json["revisionNumber"], "1");
        assert!(json.get("reference").is_none());
        assert!(json.get("blueprint_course_id").is_none());
        let decoded: BlueprintRevisionTuple =
            serde_json::from_value(json).expect("tuple deserializes from its members");
        assert_eq!(decoded, tuple);
    }

    #[test]
    fn blueprint_revision_tuple_rejects_legacy_reference_json() {
        assert!(
            serde_json::from_str::<BlueprintRevisionTuple>(
                r#"{"reference":{"blueprintCourseId":"BP7K3M2QXH","revisionNumber":"1"}}"#
            )
            .is_err()
        );
        assert!(
            serde_json::from_str::<BlueprintRevisionTuple>(
                r#"{"blueprintCourseId":"BP7K3M2QXH","revisionNumber":"1","reference":"BP7K3M2QXH"}"#
            )
            .is_err()
        );
        assert!(
            serde_json::from_str::<BlueprintRevisionTuple>(
                r#"{"blueprint_course_id":"BP7K3M2QXH","revision":"1"}"#
            )
            .is_err()
        );
        assert!(
            serde_json::from_value::<BlueprintRevisionTuple>(serde_json::json!("1")).is_err(),
            "a lone Revision Number is not a Blueprint Revision Tuple"
        );
    }
}
