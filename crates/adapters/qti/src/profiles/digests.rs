//! Canonical digest inputs for a committed QTI profile import.

use std::fmt;

use objects::Sha256Digest;
use serde::Serialize;

use super::{
    QtiMappedItem, QtiMappingVersion, QtiPleDefault, QtiProfileContractError, QtiProfileDetection,
    QtiProfileDetectionEvidence, QtiProfileDiagnostic, QtiProfileId, QtiProfileVersion,
    QtiSafeItemReport, QtiSafeItemStatus,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct QtiProfileItemDisposition {
    pub source_identifier: String,
    pub item_id: Option<String>,
    pub accepted: bool,
    pub public_mapping_sha256: Option<Sha256Digest>,
    pub diagnostics: Vec<QtiProfileDiagnostic>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct QtiProfileReportDigestInput {
    pub profile: QtiProfileId,
    pub profile_version: QtiProfileVersion,
    pub mapping_version: QtiMappingVersion,
    pub detection: QtiProfileDetectionEvidence,
    pub detection_outcome: QtiProfileDetection,
    pub items: Vec<QtiProfileItemDisposition>,
    pub defaults: Vec<QtiProfileDiagnostic>,
}

impl QtiProfileReportDigestInput {
    /// Computes the canonical safe report digest even when no item was accepted.
    pub fn profile_report_sha256(&self) -> Result<Sha256Digest, QtiProfileContractError> {
        validate_profile_report(self)?;
        canonical_digest("profile-report", self)
    }
}

pub(super) fn package_report_digest_input(
    profile: QtiProfileId,
    detection: &QtiProfileDetectionEvidence,
    reports: &[QtiSafeItemReport],
    mapped_items: &[QtiMappedItem],
) -> Result<QtiProfileReportDigestInput, QtiProfileContractError> {
    let detection_outcome = super::detect_qti_profile(detection);
    if detection_outcome != QtiProfileDetection::Recognized(profile) {
        return Err(QtiProfileContractError::DetectionMismatch);
    }
    let mut accepted_items = mapped_items.iter();
    let mut items = Vec::with_capacity(reports.len());

    for report in reports {
        let disposition = match report.status() {
            QtiSafeItemStatus::Accepted => {
                let item = accepted_items
                    .next()
                    .ok_or(QtiProfileContractError::MappingOwnerMismatch)?;
                if item.safe_report().source_identifier() != report.source_identifier() {
                    return Err(QtiProfileContractError::MappingOwnerMismatch);
                }
                item.accepted_item_disposition()?
            }
            QtiSafeItemStatus::Rejected => QtiProfileItemDisposition {
                source_identifier: report.source_identifier().to_string(),
                item_id: None,
                accepted: false,
                public_mapping_sha256: None,
                diagnostics: report
                    .diagnostics()
                    .iter()
                    .chain(report.defaults())
                    .chain(report.warnings())
                    .map(|diagnostic| diagnostic.digest_diagnostic())
                    .collect(),
            },
        };
        items.push(disposition);
    }

    if accepted_items.next().is_some() {
        return Err(QtiProfileContractError::MappingOwnerMismatch);
    }

    Ok(QtiProfileReportDigestInput {
        profile,
        profile_version: profile.version(),
        mapping_version: QtiMappingVersion::V1,
        detection: detection.clone(),
        detection_outcome,
        items,
        defaults: QtiPleDefault::ALL
            .into_iter()
            .map(QtiPleDefault::digest_diagnostic)
            .collect(),
    })
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct QtiPublicMappingDigestInput {
    pub source_location: String,
    pub source_identifier: String,
    pub title: String,
    pub prompt_markdown: String,
    pub choices: Vec<QtiPublicChoiceDigestInput>,
    pub points: String,
    pub defaults: Vec<QtiProfileDiagnostic>,
    pub warnings: Vec<QtiProfileDiagnostic>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct QtiPublicChoiceDigestInput {
    pub ple_choice_id: String,
    pub text_markdown: String,
}

/// Server-only mapping material. It intentionally cannot be serialized or
/// formatted for logs.
///
/// ```compile_fail
/// use adapter_qti::{QtiPrivateChoiceMapDigestInput, QtiPrivateMappingDigestInput};
/// let private = QtiPrivateMappingDigestInput::new(
///     "vendor".into(), "ple".into(),
///     vec![QtiPrivateChoiceMapDigestInput::new("vendor".into(), "ple".into())], vec![]
/// );
/// let _ = serde_json::to_string(&private);
/// let _ = format!("{private:?}");
/// ```
#[derive(Clone, PartialEq, Eq)]
pub struct QtiPrivateMappingDigestInput {
    correct_vendor_choice_id: String,
    correct_ple_choice_id: String,
    ordered_choice_map: Vec<QtiPrivateChoiceMapDigestInput>,
    mapped_feedback: Vec<QtiPrivateFeedbackDigestInput>,
}

impl QtiPrivateMappingDigestInput {
    pub fn new(
        correct_vendor_choice_id: String,
        correct_ple_choice_id: String,
        ordered_choice_map: Vec<QtiPrivateChoiceMapDigestInput>,
        mapped_feedback: Vec<QtiPrivateFeedbackDigestInput>,
    ) -> Self {
        Self {
            correct_vendor_choice_id,
            correct_ple_choice_id,
            ordered_choice_map,
            mapped_feedback,
        }
    }

    fn has_correct_binding(&self) -> bool {
        self.ordered_choice_map.iter().any(|entry| {
            entry.vendor_choice_id == self.correct_vendor_choice_id
                && entry.ple_choice_id == self.correct_ple_choice_id
        })
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct QtiPrivateChoiceMapDigestInput {
    vendor_choice_id: String,
    ple_choice_id: String,
}

impl QtiPrivateChoiceMapDigestInput {
    pub fn new(vendor_choice_id: String, ple_choice_id: String) -> Self {
        Self {
            vendor_choice_id,
            ple_choice_id,
        }
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct QtiPrivateFeedbackDigestInput {
    location: String,
    content: String,
}

impl QtiPrivateFeedbackDigestInput {
    pub fn new(location: String, content: String) -> Self {
        Self { location, content }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct QtiImportIntegrityDigests {
    pub profile_report_sha256: Sha256Digest,
    pub public_mapping_sha256: Sha256Digest,
    pub private_mapping_sha256: Sha256Digest,
    pub mapping_sha256: Sha256Digest,
    pub warning_sha256: Sha256Digest,
}

impl fmt::Debug for QtiImportIntegrityDigests {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("QtiImportIntegrityDigests")
            .field("profile_report_sha256", &"<integrity-digest>")
            .field("public_mapping_sha256", &"<integrity-digest>")
            .field("private_mapping_sha256", &"<integrity-digest>")
            .field("mapping_sha256", &"<integrity-digest>")
            .field("warning_sha256", &"<integrity-digest>")
            .finish()
    }
}

impl QtiImportIntegrityDigests {
    pub(crate) fn compute(
        report: &QtiProfileReportDigestInput,
        public_mapping: &QtiPublicMappingDigestInput,
        private_mapping: &QtiPrivateMappingDigestInput,
    ) -> Result<Self, QtiProfileContractError> {
        let public_mapping_sha256 = public_mapping.digest()?;
        validate_report(report, public_mapping, public_mapping_sha256)?;
        if !private_mapping.has_correct_binding() {
            return Err(QtiProfileContractError::PrivateBindingMissing);
        }
        let profile_report_sha256 = report.profile_report_sha256()?;
        let private_mapping_sha256 = canonical_digest(
            "private-mapping",
            &PrivateMappingCanonical::from(private_mapping),
        )?;
        let mapping_sha256 = canonical_digest(
            "combined-mapping",
            &CombinedMappingCanonical {
                profile: report.profile,
                profile_version: report.profile_version,
                mapping_version: report.mapping_version,
                public_mapping_sha256,
                private_mapping_sha256,
            },
        )?;
        let warning_sha256 = canonical_digest(
            "warnings",
            &WarningsCanonical {
                defaults: &public_mapping.defaults,
                warnings: &public_mapping.warnings,
            },
        )?;
        Ok(Self {
            profile_report_sha256,
            public_mapping_sha256,
            private_mapping_sha256,
            mapping_sha256,
            warning_sha256,
        })
    }
}

impl QtiPublicMappingDigestInput {
    /// ```compile_fail
    /// use adapter_qti::QtiPublicMappingDigestInput;
    /// fn detached_digest(input: &QtiPublicMappingDigestInput) {
    ///     let _ = input.digest();
    /// }
    /// ```
    pub(crate) fn digest(&self) -> Result<Sha256Digest, QtiProfileContractError> {
        canonical_digest("public-mapping", self)
    }
}

fn validate_report(
    report: &QtiProfileReportDigestInput,
    public_mapping: &QtiPublicMappingDigestInput,
    public_mapping_sha256: Sha256Digest,
) -> Result<(), QtiProfileContractError> {
    validate_profile_report(report)?;
    let Some(item) = report
        .items
        .iter()
        .find(|item| item.source_identifier == public_mapping.source_identifier)
    else {
        return Err(QtiProfileContractError::SourceItemNotAccepted);
    };
    if !item.accepted {
        return Err(QtiProfileContractError::SourceItemNotAccepted);
    }
    if item.public_mapping_sha256 != Some(public_mapping_sha256) {
        return Err(QtiProfileContractError::PublicMappingDigestMismatch);
    }
    Ok(())
}

fn validate_profile_report(
    report: &QtiProfileReportDigestInput,
) -> Result<(), QtiProfileContractError> {
    if report.profile.version() != report.profile_version {
        return Err(QtiProfileContractError::ProfileVersionMismatch);
    }
    let expected_detection = match report.profile {
        QtiProfileId::GENERIC => QtiProfileDetection::GenericCompatibility,
        profile => QtiProfileDetection::Recognized(profile),
    };
    if crate::detect_qti_profile(&report.detection) != expected_detection
        || report.detection_outcome != expected_detection
    {
        return Err(QtiProfileContractError::DetectionMismatch);
    }
    for item in &report.items {
        if item.accepted != item.public_mapping_sha256.is_some() {
            return Err(QtiProfileContractError::ItemDigestDisposition);
        }
    }
    Ok(())
}

#[derive(Serialize)]
struct ContractEnvelope<'a, T> {
    schema: &'static str,
    value: &'a T,
}

fn canonical_digest<T: Serialize>(
    kind: &'static str,
    value: &T,
) -> Result<Sha256Digest, QtiProfileContractError> {
    let bytes = serde_json::to_vec(&ContractEnvelope {
        schema: kind,
        value,
    })
    .map_err(|error| QtiProfileContractError::Serialization(error.to_string()))?;
    Ok(Sha256Digest::compute(&bytes))
}

#[derive(Serialize)]
struct PrivateMappingCanonical<'a> {
    correct_vendor_choice_id: &'a str,
    correct_ple_choice_id: &'a str,
    ordered_choice_map: Vec<PrivateChoiceMapCanonical<'a>>,
    mapped_feedback: Vec<PrivateFeedbackCanonical<'a>>,
}

impl<'a> From<&'a QtiPrivateMappingDigestInput> for PrivateMappingCanonical<'a> {
    fn from(value: &'a QtiPrivateMappingDigestInput) -> Self {
        Self {
            correct_vendor_choice_id: &value.correct_vendor_choice_id,
            correct_ple_choice_id: &value.correct_ple_choice_id,
            ordered_choice_map: value
                .ordered_choice_map
                .iter()
                .map(|entry| PrivateChoiceMapCanonical {
                    vendor_choice_id: &entry.vendor_choice_id,
                    ple_choice_id: &entry.ple_choice_id,
                })
                .collect(),
            mapped_feedback: value
                .mapped_feedback
                .iter()
                .map(|entry| PrivateFeedbackCanonical {
                    location: &entry.location,
                    content: &entry.content,
                })
                .collect(),
        }
    }
}

#[derive(Serialize)]
struct PrivateChoiceMapCanonical<'a> {
    vendor_choice_id: &'a str,
    ple_choice_id: &'a str,
}

#[derive(Serialize)]
struct PrivateFeedbackCanonical<'a> {
    location: &'a str,
    content: &'a str,
}

#[derive(Serialize)]
struct CombinedMappingCanonical {
    profile: QtiProfileId,
    profile_version: QtiProfileVersion,
    mapping_version: QtiMappingVersion,
    public_mapping_sha256: Sha256Digest,
    private_mapping_sha256: Sha256Digest,
}

#[derive(Serialize)]
struct WarningsCanonical<'a> {
    defaults: &'a [QtiProfileDiagnostic],
    warnings: &'a [QtiProfileDiagnostic],
}

#[cfg(test)]
#[path = "digests/tests.rs"]
mod tests;
