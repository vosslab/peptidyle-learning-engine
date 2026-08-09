//! Private mapped QTI items and their bounded instructor-safe projection.

use std::collections::BTreeSet;
use std::fmt;

use objects::Sha256Digest;

use super::choice_map_payload::QtiChoiceMapPayload;
use super::normalized_digest::{NormalizedProfileItemInput, normalized_profile_item_sha256};
use super::report::{
    MAX_CHOICE_TEXT_CHARS, MAX_PROMPT_CHARS, MAX_SAFE_DIAGNOSTICS,
    MAX_SAFE_SOURCE_IDENTIFIER_CHARS, MAX_SAFE_SOURCE_LOCATION_CHARS, MAX_SAFE_TITLE_CHARS,
    QtiMappedPoints, QtiPleDefault, QtiSafeDiagnostic, QtiSafeItemReport, QtiSafeItemStatus,
    char_count,
};
use super::server_parts::QtiMappedItemServerParts;
use super::{
    QtiChoiceIdMap, QtiImportIntegrityDigests, QtiMappingVersion, QtiPrivateChoiceMapDigestInput,
    QtiPrivateMappingDigestInput, QtiProfileContractError, QtiProfileId, QtiProfileItemDisposition,
    QtiProfileReportDigestInput, QtiProfileVersion, QtiPublicChoiceDigestInput,
    QtiPublicMappingDigestInput,
};

#[allow(dead_code)]
const MAX_SAFE_ITEMS: usize = 100;

/// Refusal reasons for the private mapped-item constructor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QtiMappedItemError {
    EmptySourceLocation,
    EmptySourceIdentifier,
    EmptyTitle,
    EmptyPrompt,
    SourceLocationTooLong,
    SourceIdentifierTooLong,
    TitleTooLong,
    PromptTooLong,
    ChoiceCount,
    InvalidChoiceId,
    DuplicateChoiceId,
    DuplicateVendorChoiceId,
    EmptyChoiceText,
    ChoiceTextTooLong,
    InvalidPoints,
    ProfilePointsPolicy,
    CorrectChoiceMissing,
    ChoiceMapMismatch,
    ChoiceMapOrderMismatch,
    UnsafeDiagnostic,
    DigestEncoding,
}

impl fmt::Display for QtiMappedItemError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::EmptySourceLocation => "QTI mapped item source location is required",
            Self::EmptySourceIdentifier => "QTI mapped item source identifier is required",
            Self::EmptyTitle => "QTI mapped item title is required",
            Self::EmptyPrompt => "QTI mapped item prompt is required",
            Self::SourceLocationTooLong => "QTI mapped item source location exceeds the safe limit",
            Self::SourceIdentifierTooLong => {
                "QTI mapped item source identifier exceeds the safe limit"
            }
            Self::TitleTooLong => "QTI mapped item title exceeds the safe limit",
            Self::PromptTooLong => "QTI mapped item prompt exceeds the safe limit",
            Self::ChoiceCount => "QTI mapped item requires 2 to 100 choices",
            Self::InvalidChoiceId => "QTI mapped item has an invalid PLE choice identifier",
            Self::DuplicateChoiceId => "QTI mapped item has duplicate PLE choice identifiers",
            Self::DuplicateVendorChoiceId => {
                "QTI mapped item has duplicate vendor choice identifiers"
            }
            Self::EmptyChoiceText => "QTI mapped item choice text is required",
            Self::ChoiceTextTooLong => "QTI mapped item choice text exceeds the safe limit",
            Self::InvalidPoints => "QTI mapped item points must be finite and nonnegative",
            Self::ProfilePointsPolicy => {
                "QTI mapped item points default is not allowed for this profile"
            }
            Self::CorrectChoiceMissing => {
                "QTI mapped item correct choice is absent from its choices"
            }
            Self::ChoiceMapMismatch => {
                "QTI mapped item private choice map does not match public choices"
            }
            Self::ChoiceMapOrderMismatch => {
                "QTI mapped item private choice map does not preserve public choice order"
            }
            Self::UnsafeDiagnostic => "QTI item diagnostic is not safe for an instructor report",
            Self::DigestEncoding => "QTI mapped item normalized digest could not be encoded",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for QtiMappedItemError {}

/// Complete profile-mapped item. It remains non-serializable and non-Debug
/// because it binds vendor identifiers and a correct choice.
///
/// ```compile_fail
/// use adapter_qti::profiles::QtiMappedItem;
/// fn needs_debug<T: std::fmt::Debug>() {}
/// needs_debug::<QtiMappedItem>();
/// ```
///
/// ```compile_fail
/// use adapter_qti::profiles::QtiMappedItem;
/// fn needs_serialize<T: serde::Serialize>() {}
/// needs_serialize::<QtiMappedItem>();
/// ```
#[derive(Clone, PartialEq, Eq)]
pub struct QtiMappedItem {
    profile: QtiProfileId,
    profile_version: QtiProfileVersion,
    mapping_version: QtiMappingVersion,
    public_mapping: QtiPublicMappingDigestInput,
    private_mapping: QtiPrivateMappingDigestInput,
    normalized_profile_item_sha256: Sha256Digest,
    correct_ple_choice_id: String,
    choice_map: Vec<QtiChoiceIdMap>,
    safe_report: QtiSafeItemReport,
}

impl QtiMappedItem {
    #[allow(clippy::too_many_arguments)]
    #[allow(dead_code)]
    pub(crate) fn new(
        profile: QtiProfileId,
        source_location: String,
        source_identifier: String,
        title: String,
        prompt_markdown: String,
        choices: Vec<QtiPublicChoiceDigestInput>,
        points: QtiMappedPoints,
        choice_map: Vec<QtiChoiceIdMap>,
        correct_vendor_choice_id: String,
    ) -> Result<Self, QtiMappedItemError> {
        validate_required(&source_location, QtiMappedItemError::EmptySourceLocation)?;
        if char_count(&source_location) > MAX_SAFE_SOURCE_LOCATION_CHARS {
            return Err(QtiMappedItemError::SourceLocationTooLong);
        }
        validate_required(
            &source_identifier,
            QtiMappedItemError::EmptySourceIdentifier,
        )?;
        if char_count(&source_identifier) > MAX_SAFE_SOURCE_IDENTIFIER_CHARS {
            return Err(QtiMappedItemError::SourceIdentifierTooLong);
        }
        validate_required(&title, QtiMappedItemError::EmptyTitle)?;
        if char_count(&title) > MAX_SAFE_TITLE_CHARS {
            return Err(QtiMappedItemError::TitleTooLong);
        }
        validate_required(&prompt_markdown, QtiMappedItemError::EmptyPrompt)?;
        if char_count(&prompt_markdown) > MAX_PROMPT_CHARS {
            return Err(QtiMappedItemError::PromptTooLong);
        }
        validate_choices(&choices)?;
        let blackboard_defaulted_points = matches!(points, QtiMappedPoints::BlackboardDefaulted);
        let (points, warnings, safe_warnings) = points.resolve(profile)?;
        validate_choice_map(&choices, &choice_map, &correct_vendor_choice_id)?;

        let normalized_choices = choice_map
            .iter()
            .zip(&choices)
            .map(|(entry, choice)| {
                (
                    entry.server_vendor_choice_id(),
                    choice.text_markdown.as_str(),
                )
            })
            .collect::<Vec<_>>();
        let normalized_profile_item_sha256 =
            normalized_profile_item_sha256(&NormalizedProfileItemInput {
                profile,
                profile_version: profile.version(),
                title: &title,
                prompt_markdown: &prompt_markdown,
                choices: &normalized_choices,
                correct_vendor_choice_id: &correct_vendor_choice_id,
                canonical_points: &points,
                blackboard_defaulted_points,
            })
            .map_err(|_| QtiMappedItemError::DigestEncoding)?;

        let defaults: Vec<_> = QtiPleDefault::ALL
            .into_iter()
            .map(QtiPleDefault::digest_diagnostic)
            .collect();
        let safe_defaults = QtiPleDefault::ALL
            .into_iter()
            .map(QtiPleDefault::safe_diagnostic)
            .collect();
        let public_mapping = QtiPublicMappingDigestInput {
            source_location,
            source_identifier: source_identifier.clone(),
            title: title.clone(),
            prompt_markdown,
            choices,
            points,
            defaults: defaults.clone(),
            warnings: warnings.clone(),
        };
        let correct_ple_choice_id = choice_map
            .iter()
            .find(|entry| entry.server_vendor_choice_id() == correct_vendor_choice_id)
            .expect("validated correct vendor choice exists")
            .ple_choice_id()
            .to_string();
        let private_mapping = QtiPrivateMappingDigestInput::new(
            correct_vendor_choice_id.clone(),
            correct_ple_choice_id.clone(),
            choice_map
                .iter()
                .map(|entry| {
                    QtiPrivateChoiceMapDigestInput::new(
                        entry.server_vendor_choice_id().to_string(),
                        entry.ple_choice_id().to_string(),
                    )
                })
                .collect(),
            Vec::new(),
        );
        let safe_report = QtiSafeItemReport {
            source_identifier,
            title: Some(title),
            status: QtiSafeItemStatus::Accepted,
            diagnostics: Vec::new(),
            defaults: safe_defaults,
            warnings: safe_warnings,
        };
        Ok(Self {
            profile,
            profile_version: profile.version(),
            mapping_version: QtiMappingVersion::V1,
            public_mapping,
            private_mapping,
            normalized_profile_item_sha256,
            correct_ple_choice_id,
            choice_map,
            safe_report,
        })
    }

    /// Builds a bounded answer-free refusal report from parser diagnostics.
    #[allow(dead_code)]
    pub(crate) fn rejected_safe_report(
        source_identifier: String,
        title: Option<String>,
        diagnostics: Vec<QtiSafeDiagnostic>,
    ) -> Result<QtiSafeItemReport, QtiMappedItemError> {
        validate_required(
            &source_identifier,
            QtiMappedItemError::EmptySourceIdentifier,
        )?;
        if let Some(title) = &title {
            validate_required(title, QtiMappedItemError::EmptyTitle)?;
        }
        if char_count(&source_identifier) > MAX_SAFE_SOURCE_IDENTIFIER_CHARS {
            return Err(QtiMappedItemError::SourceIdentifierTooLong);
        }
        if title
            .as_deref()
            .is_some_and(|title| char_count(title) > MAX_SAFE_TITLE_CHARS)
        {
            return Err(QtiMappedItemError::TitleTooLong);
        }
        if diagnostics.len() > MAX_SAFE_DIAGNOSTICS {
            return Err(QtiMappedItemError::UnsafeDiagnostic);
        }
        Ok(QtiSafeItemReport {
            source_identifier,
            title,
            status: QtiSafeItemStatus::Rejected,
            diagnostics,
            defaults: Vec::new(),
            warnings: Vec::new(),
        })
    }

    /// Builds the same answer-free refusal shape after a hostile source value
    /// failed the ordinary visible-field bounds. This deliberately substitutes
    /// a fixed opaque identity instead of truncating or echoing the source.
    pub(crate) fn rejected_safe_report_lossy(
        source_identifier: &str,
        fallback_identifier: &str,
        title: Option<&str>,
        diagnostics: Vec<QtiSafeDiagnostic>,
    ) -> QtiSafeItemReport {
        let identifier = if source_identifier.trim().is_empty()
            || char_count(source_identifier) > MAX_SAFE_SOURCE_IDENTIFIER_CHARS
        {
            fallback_identifier.to_string()
        } else {
            source_identifier.to_string()
        };
        let title = title
            .filter(|value| !value.trim().is_empty() && char_count(value) <= MAX_SAFE_TITLE_CHARS)
            .map(str::to_string);
        QtiSafeItemReport {
            source_identifier: identifier,
            title,
            status: QtiSafeItemStatus::Rejected,
            diagnostics: diagnostics.into_iter().take(MAX_SAFE_DIAGNOSTICS).collect(),
            defaults: Vec::new(),
            warnings: Vec::new(),
        }
    }

    pub fn safe_report(&self) -> &QtiSafeItemReport {
        &self.safe_report
    }

    /// Public digest input used by the server's committed import contract.
    pub fn public_mapping_digest_input(&self) -> &QtiPublicMappingDigestInput {
        &self.public_mapping
    }

    /// Private digest input used only by the server's committed import contract.
    pub fn private_mapping_digest_input(&self) -> &QtiPrivateMappingDigestInput {
        &self.private_mapping
    }

    /// Opaque normalized source-item fingerprint for later provenance binding.
    pub fn normalized_profile_item_sha256(&self) -> Sha256Digest {
        self.normalized_profile_item_sha256
    }

    /// Computes digests only when report and mapped item share one profile owner.
    pub fn compute_integrity_digests(
        &self,
        report: &QtiProfileReportDigestInput,
    ) -> Result<QtiImportIntegrityDigests, QtiProfileContractError> {
        if report.profile != self.profile
            || report.profile_version != self.profile_version
            || report.mapping_version != self.mapping_version
        {
            return Err(QtiProfileContractError::MappingOwnerMismatch);
        }
        QtiImportIntegrityDigests::compute(report, &self.public_mapping, &self.private_mapping)
    }

    /// Returns this mapped item's accepted disposition without exposing a detached digest API.
    pub fn accepted_item_disposition(
        &self,
    ) -> Result<QtiProfileItemDisposition, QtiProfileContractError> {
        let public_mapping_sha256 = self.public_mapping.digest()?;
        Ok(QtiProfileItemDisposition {
            source_identifier: self.public_mapping.source_identifier.clone(),
            item_id: Some(self.public_mapping.source_identifier.clone()),
            accepted: true,
            public_mapping_sha256: Some(public_mapping_sha256),
            diagnostics: Vec::new(),
        })
    }

    /// Moves this private binding to server-owned conversion code.
    pub fn into_server_parts(self) -> QtiMappedItemServerParts {
        QtiMappedItemServerParts {
            profile: self.profile,
            profile_version: self.profile_version,
            mapping_version: self.mapping_version,
            public_mapping: self.public_mapping,
            private_mapping: self.private_mapping,
            normalized_profile_item_sha256: self.normalized_profile_item_sha256,
            correct_ple_choice_id: self.correct_ple_choice_id,
            choice_map_payload: QtiChoiceMapPayload::from_ordered_map(&self.choice_map)
                .expect("mapped item validates choice-map bounds before server extraction"),
            choice_map: self.choice_map,
        }
    }
}

#[allow(dead_code)]
fn validate_required(
    value: &str,
    empty_error: QtiMappedItemError,
) -> Result<(), QtiMappedItemError> {
    if value.trim().is_empty() {
        return Err(empty_error);
    }
    Ok(())
}

#[allow(dead_code)]
fn validate_choices(choices: &[QtiPublicChoiceDigestInput]) -> Result<(), QtiMappedItemError> {
    if !(2..=MAX_SAFE_ITEMS).contains(&choices.len()) {
        return Err(QtiMappedItemError::ChoiceCount);
    }
    let mut identifiers = BTreeSet::new();
    for choice in choices {
        if !is_valid_ple_choice_id(&choice.ple_choice_id) {
            return Err(QtiMappedItemError::InvalidChoiceId);
        }
        if !identifiers.insert(choice.ple_choice_id.as_str()) {
            return Err(QtiMappedItemError::DuplicateChoiceId);
        }
        if choice.text_markdown.trim().is_empty() {
            return Err(QtiMappedItemError::EmptyChoiceText);
        }
        if char_count(&choice.text_markdown) > MAX_CHOICE_TEXT_CHARS {
            return Err(QtiMappedItemError::ChoiceTextTooLong);
        }
    }
    Ok(())
}

#[allow(dead_code)]
fn validate_choice_map(
    choices: &[QtiPublicChoiceDigestInput],
    choice_map: &[QtiChoiceIdMap],
    correct_vendor_choice_id: &str,
) -> Result<(), QtiMappedItemError> {
    if choice_map.len() != choices.len() {
        return Err(QtiMappedItemError::ChoiceMapMismatch);
    }
    let public_ids = choices
        .iter()
        .map(|choice| choice.ple_choice_id.as_str())
        .collect::<BTreeSet<_>>();
    let mapped_ids = choice_map
        .iter()
        .map(QtiChoiceIdMap::ple_choice_id)
        .collect::<BTreeSet<_>>();
    let vendor_ids = choice_map
        .iter()
        .map(QtiChoiceIdMap::server_vendor_choice_id)
        .collect::<BTreeSet<_>>();
    if vendor_ids.len() != choice_map.len() {
        return Err(QtiMappedItemError::DuplicateVendorChoiceId);
    }
    if choices
        .iter()
        .zip(choice_map)
        .any(|(choice, entry)| choice.ple_choice_id != entry.ple_choice_id())
    {
        return Err(QtiMappedItemError::ChoiceMapOrderMismatch);
    }
    if public_ids != mapped_ids
        || choice_map
            .iter()
            .filter(|entry| entry.server_vendor_choice_id() == correct_vendor_choice_id)
            .count()
            != 1
    {
        return Err(QtiMappedItemError::ChoiceMapMismatch);
    }
    Ok(())
}

#[allow(dead_code)]
fn is_valid_ple_choice_id(value: &str) -> bool {
    let bytes = value.as_bytes();
    !bytes.is_empty()
        && bytes.len() <= 64
        && bytes[0].is_ascii_lowercase()
        && bytes.iter().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'_' | b'-')
        })
}

#[cfg(test)]
#[path = "mapped_item/tests.rs"]
mod tests;
