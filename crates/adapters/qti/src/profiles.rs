//! Closed QTI profile identities and integrity contracts.
//!
//! This module names the profile a future parser has proven, but does not
//! parse vendor XML.  Keeping the evidence and digest contract here prevents
//! a parser, HTTP route, or persistence backend from inventing an ad-hoc
//! compatibility label.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

mod blackboard;
mod canvas;
mod checksums;
mod choice_ids;
mod choice_map_payload;
mod mapped_item;
#[allow(dead_code)] // The next profile parsers consume this shared, crate-private boundary.
mod markup;
mod matrix;
mod normalized_fingerprint;
mod report;
mod server_parts;

pub use blackboard::{BlackboardQtiImportError, BlackboardQtiPackage, import_blackboard_qti21};
pub use canvas::{CanvasQtiImportError, CanvasQtiPackage, import_canvas_qti12};
pub use checksums::{
    QtiImportChecksums, QtiImportResultChecksumInput, QtiPrivateChoiceMapChecksumInput,
    QtiPrivateFeedbackChecksumInput, QtiPrivateMappingChecksumInput, QtiProfileItemDisposition,
    QtiPublicChoiceChecksumInput, QtiPublicMappingChecksumInput,
};
pub use choice_ids::{QtiChoiceIdMap, QtiChoiceIdMappingError, map_qti_choice_ids};
pub use choice_map_payload::QtiChoiceMapPayload;
pub use mapped_item::{QtiMappedItem, QtiMappedItemError};
pub use matrix::{
    BLACKBOARD_ITEM_NAMESPACE, CANVAS_ITEM_NAMESPACE, IMS_CONTENT_PACKAGING_NAMESPACE,
    QTI_PROFILE_MATRIX, QtiProfileMatrixDetail,
};
pub use normalized_fingerprint::NormalizedQtiItemFingerprint;
pub use report::{
    QtiMappedPoints, QtiPleDefault, QtiSafeDiagnostic, QtiSafeDiagnosticLocation,
    QtiSafeDiagnosticTemplate, QtiSafeItemReport, QtiSafeItemStatus,
};
pub use server_parts::QtiMappedItemServerParts;

/// The only profile identifiers that this adapter may write.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum QtiProfileId {
    CanvasQti12StaticSingleChoice,
    BlackboardQti21StaticSingleChoicePool,
    PleQtiAssessmentItemSingleChoice,
}

impl QtiProfileId {
    pub const CANVAS: Self = Self::CanvasQti12StaticSingleChoice;
    pub const BLACKBOARD: Self = Self::BlackboardQti21StaticSingleChoicePool;
    pub const GENERIC: Self = Self::PleQtiAssessmentItemSingleChoice;

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::CanvasQti12StaticSingleChoice => "canvas-qti-1.2-static-single-choice/v1",
            Self::BlackboardQti21StaticSingleChoicePool => {
                "blackboard-qti-2.1-static-single-choice-pool/v1"
            }
            Self::PleQtiAssessmentItemSingleChoice => "ple-qti-assessment-item-single-choice/v1",
        }
    }

    pub const fn version(self) -> QtiProfileVersion {
        QtiProfileVersion::V1
    }
}

impl fmt::Display for QtiProfileId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl FromStr for QtiProfileId {
    type Err = QtiProfileContractError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "canvas-qti-1.2-static-single-choice/v1" => Ok(Self::CANVAS),
            "blackboard-qti-2.1-static-single-choice-pool/v1" => Ok(Self::BLACKBOARD),
            "ple-qti-assessment-item-single-choice/v1" => Ok(Self::GENERIC),
            _ => Err(QtiProfileContractError::UnknownProfile(value.to_string())),
        }
    }
}

impl Serialize for QtiProfileId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for QtiProfileId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        value.parse().map_err(serde::de::Error::custom)
    }
}

/// Version of an immutable profile contract, independent of crate releases.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum QtiProfileVersion {
    V1,
}

impl Serialize for QtiProfileVersion {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for QtiProfileVersion {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        match value.as_str() {
            "v1" => Ok(Self::V1),
            _ => Err(serde::de::Error::custom(format!(
                "unknown QTI profile version `{value}`"
            ))),
        }
    }
}

impl QtiProfileVersion {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::V1 => "v1",
        }
    }
}

/// Version of the mapping and digest encoding for a profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum QtiMappingVersion {
    V1,
}

impl Serialize for QtiMappingVersion {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for QtiMappingVersion {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        match value.as_str() {
            "v1" => Ok(Self::V1),
            _ => Err(serde::de::Error::custom(format!(
                "unknown QTI mapping version `{value}`"
            ))),
        }
    }
}

impl QtiMappingVersion {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::V1 => "v1",
        }
    }
}

/// Stable, intentionally closed diagnostics for profile parsing and mapping.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum QtiProfileDiagnosticCode {
    ProfileAmbiguous,
    ManifestNamespace,
    ManifestSchema,
    ResourceType,
    ResourcePath,
    UnexpectedEntry,
    ItemNamespace,
    ItemShape,
    QuestionType,
    ResponseCardinality,
    ChoiceCount,
    DuplicateChoiceId,
    CorrectResponse,
    ResponseProcessing,
    Points,
    Feedback,
    Markup,
    Media,
    Shuffle,
    Policy,
}

impl QtiProfileDiagnosticCode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ProfileAmbiguous => "profile-ambiguous",
            Self::ManifestNamespace => "manifest-namespace",
            Self::ManifestSchema => "manifest-schema",
            Self::ResourceType => "resource-type",
            Self::ResourcePath => "resource-path",
            Self::UnexpectedEntry => "unexpected-entry",
            Self::ItemNamespace => "item-namespace",
            Self::ItemShape => "item-shape",
            Self::QuestionType => "question-type",
            Self::ResponseCardinality => "response-cardinality",
            Self::ChoiceCount => "choice-count",
            Self::DuplicateChoiceId => "duplicate-choice-id",
            Self::CorrectResponse => "correct-response",
            Self::ResponseProcessing => "response-processing",
            Self::Points => "points",
            Self::Feedback => "feedback",
            Self::Markup => "markup",
            Self::Media => "media",
            Self::Shuffle => "shuffle",
            Self::Policy => "policy",
        }
    }
}

impl fmt::Display for QtiProfileDiagnosticCode {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// One safe, actionable diagnostic attached to a package or item report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QtiProfileDiagnostic {
    pub code: QtiProfileDiagnosticCode,
    pub location: String,
    pub detail: String,
}

/// The bounded manifest facts used for vendor-profile recognition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QtiProfileDetectionEvidence {
    pub manifest_namespace: String,
    pub manifest_schema: Option<String>,
    pub resources: Vec<QtiProfileResourceEvidence>,
    pub items: Vec<QtiProfileItemEvidence>,
}

/// A manifest resource considered by the detector; it contains no XML body.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QtiProfileResourceEvidence {
    pub identifier: String,
    pub resource_type: Option<String>,
    pub href: Option<String>,
    pub dependencies: Vec<String>,
}

/// A parsed item root considered by the detector; it contains no item content.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QtiProfileItemEvidence {
    pub resource_identifier: String,
    pub path: String,
    pub namespace: String,
    pub root: String,
}

/// A deterministic detector outcome. Generic compatibility is not a vendor claim.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum QtiProfileDetection {
    Recognized(QtiProfileId),
    GenericCompatibility,
    Ambiguous,
    Rejected(QtiProfileDiagnosticCode),
}

/// Detects only the two exact vendor profiles. Anything else remains generic
/// compatibility until a parser validates its own accepted grammar.
pub fn detect_qti_profile(evidence: &QtiProfileDetectionEvidence) -> QtiProfileDetection {
    let mut candidates = Vec::new();
    let mut rejection = None;
    for profile in [QtiProfileId::CANVAS, QtiProfileId::BLACKBOARD] {
        match matrix::validate_profile_evidence(profile, evidence) {
            Ok(true) => candidates.push(profile),
            Ok(false) => {}
            Err(code) => {
                rejection.get_or_insert(code);
            }
        }
    }
    match candidates.as_slice() {
        [profile] => QtiProfileDetection::Recognized(*profile),
        [] => rejection
            .map(QtiProfileDetection::Rejected)
            .unwrap_or(QtiProfileDetection::GenericCompatibility),
        _ => QtiProfileDetection::Ambiguous,
    }
}

/// Contract construction failures are explicit so callers never substitute a
/// display string or a partial checksum for immutable import evidence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QtiProfileContractError {
    UnknownProfile(String),
    Serialization(String),
    ProfileVersionMismatch,
    DetectionMismatch,
    ItemChecksumDisposition,
    SourceItemNotAccepted,
    PublicMappingChecksumMismatch,
    PrivateBindingMissing,
    MappingOwnerMismatch,
}

impl fmt::Display for QtiProfileContractError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownProfile(value) => write!(formatter, "unknown QTI profile `{value}`"),
            Self::Serialization(error) => write!(formatter, "cannot encode QTI contract: {error}"),
            Self::ProfileVersionMismatch => {
                write!(formatter, "QTI profile version does not match profile")
            }
            Self::DetectionMismatch => {
                write!(formatter, "QTI detection evidence and report disagree")
            }
            Self::ItemChecksumDisposition => {
                write!(formatter, "QTI item checksum contradicts its disposition")
            }
            Self::SourceItemNotAccepted => write!(
                formatter,
                "QTI public mapping does not name an accepted item"
            ),
            Self::PublicMappingChecksumMismatch => {
                write!(
                    formatter,
                    "QTI public mapping checksum does not match report"
                )
            }
            Self::PrivateBindingMissing => write!(
                formatter,
                "QTI private correct binding is absent from choice map"
            ),
            Self::MappingOwnerMismatch => {
                write!(
                    formatter,
                    "QTI report and mapped item have different profile ownership"
                )
            }
        }
    }
}

impl std::error::Error for QtiProfileContractError {}

#[cfg(test)]
#[path = "profiles/tests.rs"]
mod tests;
