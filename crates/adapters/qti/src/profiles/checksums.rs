//! Deterministic checksum inputs for a committed QTI profile import.

use std::fmt;

use objects::Sha256Checksum;
use serde::Serialize;

use super::{
    QtiMappedItem, QtiMappingVersion, QtiPleDefault, QtiProfileContractError, QtiProfileDetection,
    QtiProfileDetectionEvidence, QtiProfileDiagnostic, QtiProfileId, QtiProfileVersion,
    QtiSafeItemReport, QtiSafeItemStatus,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
/// The QTI adapter's checksum-safe form of one Workspace Import Item Result.
///
/// The generic durable record belongs to the Workspace Import. This
/// QTI-qualified form retains the QTI mapping checksum and diagnostics while
/// the adapter constructs the committed import-result checksum.
pub struct QtiWorkspaceImportItemResult {
    pub source_identifier: String,
    pub item_id: Option<String>,
    pub accepted: bool,
    pub public_mapping_checksum: Option<Sha256Checksum>,
    pub diagnostics: Vec<QtiProfileDiagnostic>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct QtiImportResultChecksumInput {
    pub profile: QtiProfileId,
    pub profile_version: QtiProfileVersion,
    pub mapping_version: QtiMappingVersion,
    pub detection: QtiProfileDetectionEvidence,
    pub detection_outcome: QtiProfileDetection,
    pub items: Vec<QtiWorkspaceImportItemResult>,
    pub defaults: Vec<QtiProfileDiagnostic>,
}

impl QtiImportResultChecksumInput {
    /// Computes the deterministic safe-report checksum even when no item was accepted.
    pub fn import_result_checksum(&self) -> Result<Sha256Checksum, QtiProfileContractError> {
        validate_profile_report(self)?;
        deterministic_checksum("profile-report", self)
    }
}

pub(super) fn package_import_result_checksum_input(
    profile: QtiProfileId,
    detection: &QtiProfileDetectionEvidence,
    reports: &[QtiSafeItemReport],
    mapped_items: &[QtiMappedItem],
) -> Result<QtiImportResultChecksumInput, QtiProfileContractError> {
    let detection_outcome = super::detect_qti_profile(detection);
    if detection_outcome != QtiProfileDetection::Recognized(profile) {
        return Err(QtiProfileContractError::DetectionMismatch);
    }
    let mut accepted_items = mapped_items.iter();
    let mut items = Vec::with_capacity(reports.len());

    for report in reports {
        let item_result = match report.status() {
            QtiSafeItemStatus::Accepted => {
                let item = accepted_items
                    .next()
                    .ok_or(QtiProfileContractError::MappingOwnerMismatch)?;
                if item.safe_report().source_identifier() != report.source_identifier() {
                    return Err(QtiProfileContractError::MappingOwnerMismatch);
                }
                item.accepted_workspace_import_item_result()?
            }
            QtiSafeItemStatus::Rejected => QtiWorkspaceImportItemResult {
                source_identifier: report.source_identifier().to_string(),
                item_id: None,
                accepted: false,
                public_mapping_checksum: None,
                diagnostics: report
                    .diagnostics()
                    .iter()
                    .chain(report.defaults())
                    .chain(report.warnings())
                    .map(|diagnostic| diagnostic.checksum_diagnostic())
                    .collect(),
            },
        };
        items.push(item_result);
    }

    if accepted_items.next().is_some() {
        return Err(QtiProfileContractError::MappingOwnerMismatch);
    }

    Ok(QtiImportResultChecksumInput {
        profile,
        profile_version: profile.version(),
        mapping_version: QtiMappingVersion::V1,
        detection: detection.clone(),
        detection_outcome,
        items,
        defaults: QtiPleDefault::ALL
            .into_iter()
            .map(QtiPleDefault::checksum_diagnostic)
            .collect(),
    })
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct QtiPublicMappingChecksumInput {
    pub source_location: String,
    pub source_identifier: String,
    pub title: String,
    pub prompt_markdown: String,
    pub choices: Vec<QtiPublicChoiceChecksumInput>,
    pub points: String,
    pub defaults: Vec<QtiProfileDiagnostic>,
    pub warnings: Vec<QtiProfileDiagnostic>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct QtiPublicChoiceChecksumInput {
    pub ple_choice_id: String,
    pub text_markdown: String,
}

/// Server-only QTI private-mapping checksum input. It intentionally cannot be
/// serialized or formatted for logs.
///
/// ```compile_fail
/// use adapter_qti::{QtiPrivateChoiceMapChecksumInput, QtiPrivateMappingChecksumInput};
/// let private = QtiPrivateMappingChecksumInput::new(
///     "vendor".into(), "ple".into(),
///     vec![QtiPrivateChoiceMapChecksumInput::new("vendor".into(), "ple".into())], vec![]
/// );
/// let _ = serde_json::to_string(&private);
/// let _ = format!("{private:?}");
/// ```
#[derive(Clone, PartialEq, Eq)]
pub struct QtiPrivateMappingChecksumInput {
    correct_vendor_choice_id: String,
    correct_ple_choice_id: String,
    ordered_choice_map: Vec<QtiPrivateChoiceMapChecksumInput>,
    mapped_feedback: Vec<QtiPrivateFeedbackChecksumInput>,
}

impl QtiPrivateMappingChecksumInput {
    pub fn new(
        correct_vendor_choice_id: String,
        correct_ple_choice_id: String,
        ordered_choice_map: Vec<QtiPrivateChoiceMapChecksumInput>,
        mapped_feedback: Vec<QtiPrivateFeedbackChecksumInput>,
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
pub struct QtiPrivateChoiceMapChecksumInput {
    vendor_choice_id: String,
    ple_choice_id: String,
}

impl QtiPrivateChoiceMapChecksumInput {
    pub fn new(vendor_choice_id: String, ple_choice_id: String) -> Self {
        Self {
            vendor_choice_id,
            ple_choice_id,
        }
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct QtiPrivateFeedbackChecksumInput {
    location: String,
    content: String,
}

impl QtiPrivateFeedbackChecksumInput {
    pub fn new(location: String, content: String) -> Self {
        Self { location, content }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct QtiImportChecksums {
    pub import_result_checksum: Sha256Checksum,
    pub public_mapping_checksum: Sha256Checksum,
    pub private_mapping_checksum: Sha256Checksum,
    pub mapping_checksum: Sha256Checksum,
    pub warning_checksum: Sha256Checksum,
}

impl fmt::Debug for QtiImportChecksums {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("QtiImportChecksums")
            .field("import_result_checksum", &"<qti-import-checksum>")
            .field("public_mapping_checksum", &"<qti-import-checksum>")
            .field("private_mapping_checksum", &"<qti-import-checksum>")
            .field("mapping_checksum", &"<qti-import-checksum>")
            .field("warning_checksum", &"<qti-import-checksum>")
            .finish()
    }
}

impl QtiImportChecksums {
    pub(crate) fn compute(
        report: &QtiImportResultChecksumInput,
        public_mapping: &QtiPublicMappingChecksumInput,
        private_mapping: &QtiPrivateMappingChecksumInput,
    ) -> Result<Self, QtiProfileContractError> {
        let public_mapping_checksum = public_mapping.checksum()?;
        validate_report(report, public_mapping, public_mapping_checksum)?;
        if !private_mapping.has_correct_binding() {
            return Err(QtiProfileContractError::PrivateBindingMissing);
        }
        let import_result_checksum = report.import_result_checksum()?;
        let private_mapping_checksum = deterministic_checksum(
            "private-mapping",
            &PrivateMappingCanonical::from(private_mapping),
        )?;
        let mapping_checksum = deterministic_checksum(
            "combined-mapping",
            &CombinedMappingCanonical {
                profile: report.profile,
                profile_version: report.profile_version,
                mapping_version: report.mapping_version,
                public_mapping_checksum,
                private_mapping_checksum,
            },
        )?;
        let warning_checksum = deterministic_checksum(
            "warnings",
            &WarningsCanonical {
                defaults: &public_mapping.defaults,
                warnings: &public_mapping.warnings,
            },
        )?;
        Ok(Self {
            import_result_checksum,
            public_mapping_checksum,
            private_mapping_checksum,
            mapping_checksum,
            warning_checksum,
        })
    }
}

impl QtiPublicMappingChecksumInput {
    /// ```compile_fail
    /// use adapter_qti::QtiPublicMappingChecksumInput;
    /// fn detached_checksum(input: &QtiPublicMappingChecksumInput) {
    ///     let _ = input.checksum();
    /// }
    /// ```
    pub(crate) fn checksum(&self) -> Result<Sha256Checksum, QtiProfileContractError> {
        deterministic_checksum("public-mapping", self)
    }
}

fn validate_report(
    report: &QtiImportResultChecksumInput,
    public_mapping: &QtiPublicMappingChecksumInput,
    public_mapping_checksum: Sha256Checksum,
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
    if item.public_mapping_checksum != Some(public_mapping_checksum) {
        return Err(QtiProfileContractError::PublicMappingChecksumMismatch);
    }
    Ok(())
}

fn validate_profile_report(
    report: &QtiImportResultChecksumInput,
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
        if item.accepted != item.public_mapping_checksum.is_some() {
            return Err(QtiProfileContractError::ItemChecksumImportResult);
        }
    }
    Ok(())
}

#[derive(Serialize)]
struct DeterministicChecksumInput<'a, T> {
    schema: &'static str,
    value: &'a T,
}

fn deterministic_checksum<T: Serialize>(
    kind: &'static str,
    value: &T,
) -> Result<Sha256Checksum, QtiProfileContractError> {
    let bytes = serde_json::to_vec(&DeterministicChecksumInput {
        schema: kind,
        value,
    })
    .map_err(|error| QtiProfileContractError::Serialization(error.to_string()))?;
    Ok(Sha256Checksum::compute(&bytes))
}

#[derive(Serialize)]
struct PrivateMappingCanonical<'a> {
    correct_vendor_choice_id: &'a str,
    correct_ple_choice_id: &'a str,
    ordered_choice_map: Vec<PrivateChoiceMapCanonical<'a>>,
    mapped_feedback: Vec<PrivateFeedbackCanonical<'a>>,
}

impl<'a> From<&'a QtiPrivateMappingChecksumInput> for PrivateMappingCanonical<'a> {
    fn from(value: &'a QtiPrivateMappingChecksumInput) -> Self {
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
    public_mapping_checksum: Sha256Checksum,
    private_mapping_checksum: Sha256Checksum,
}

#[derive(Serialize)]
struct WarningsCanonical<'a> {
    defaults: &'a [QtiProfileDiagnostic],
    warnings: &'a [QtiProfileDiagnostic],
}

#[cfg(test)]
#[path = "checksums/tests.rs"]
mod tests;
