//! Bounded answer-free QTI profile reports and fixed v1 conversion defaults.

use serde::Serialize;

use super::{QtiMappedItemError, QtiProfileDiagnostic, QtiProfileDiagnosticCode, QtiProfileId};

#[allow(dead_code)]
pub(super) const MAX_SAFE_SOURCE_LOCATION_CHARS: usize = 1_024;
#[allow(dead_code)]
pub(super) const MAX_SAFE_SOURCE_IDENTIFIER_CHARS: usize = 1_024;
#[allow(dead_code)]
pub(super) const MAX_SAFE_TITLE_CHARS: usize = 512;
#[allow(dead_code)]
pub(super) const MAX_PROMPT_CHARS: usize = 65_536;
#[allow(dead_code)]
pub(super) const MAX_CHOICE_TEXT_CHARS: usize = 16_384;
#[allow(dead_code)]
pub(super) const MAX_SAFE_DIAGNOSTICS: usize = 32;

/// Closed locations that a profile parser may expose to an instructor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QtiSafeDiagnosticLocation {
    Item,
    Prompt,
    Choice { index: u8 },
    Response,
    Points,
}

impl QtiSafeDiagnosticLocation {
    fn text(self) -> String {
        match self {
            Self::Item => "item".to_string(),
            Self::Prompt => "prompt".to_string(),
            Self::Choice { index } => format!("choice[{index}]"),
            Self::Response => "response".to_string(),
            Self::Points => "points".to_string(),
        }
    }
}

/// Closed stable detail templates with no source or answer interpolation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QtiSafeDiagnosticTemplate {
    PleDefaultUnlimitedAttempts,
    PleDefaultUntimed,
    PleDefaultEnglishUs,
    PleDefaultAllRightsReserved,
    PleDefaultEmptyTags,
    PleDefaultNoFeedback,
    BlackboardPointsDefaulted,
    UnsupportedMarkup,
    UnsupportedResponseProcessing,
    MissingRequiredField,
    UnsupportedItemShape,
}

impl QtiSafeDiagnosticTemplate {
    fn text(self) -> &'static str {
        match self {
            Self::PleDefaultUnlimitedAttempts => "PLE default applied: unlimited attempts.",
            Self::PleDefaultUntimed => "PLE default applied: untimed.",
            Self::PleDefaultEnglishUs => "PLE default applied: en-US.",
            Self::PleDefaultAllRightsReserved => "PLE default applied: allRightsReserved.",
            Self::PleDefaultEmptyTags => "PLE default applied: empty tags.",
            Self::PleDefaultNoFeedback => "PLE default applied: no feedback.",
            Self::BlackboardPointsDefaulted => {
                "Blackboard item points were absent; PLE default 1.0 applied."
            }
            Self::UnsupportedMarkup => "Unsupported markup prevents import.",
            Self::UnsupportedResponseProcessing => {
                "Unsupported response processing prevents import."
            }
            Self::MissingRequiredField => "A required item field is missing.",
            Self::UnsupportedItemShape => {
                "This item uses a structure outside the supported profile."
            }
        }
    }
}

/// Safe, template-backed diagnostic serialized to instructor-facing reports.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct QtiSafeDiagnostic {
    code: QtiProfileDiagnosticCode,
    location: String,
    detail: String,
}

impl QtiSafeDiagnostic {
    pub fn new(
        code: QtiProfileDiagnosticCode,
        location: QtiSafeDiagnosticLocation,
        template: QtiSafeDiagnosticTemplate,
    ) -> Result<Self, QtiMappedItemError> {
        if matches!(location, QtiSafeDiagnosticLocation::Choice { index: 0 }) {
            return Err(QtiMappedItemError::UnsafeDiagnostic);
        }
        let allowed = matches!(
            (code, template),
            (
                QtiProfileDiagnosticCode::Policy,
                QtiSafeDiagnosticTemplate::PleDefaultUnlimitedAttempts
            ) | (
                QtiProfileDiagnosticCode::Policy,
                QtiSafeDiagnosticTemplate::PleDefaultUntimed
            ) | (
                QtiProfileDiagnosticCode::Policy,
                QtiSafeDiagnosticTemplate::PleDefaultEnglishUs
            ) | (
                QtiProfileDiagnosticCode::Policy,
                QtiSafeDiagnosticTemplate::PleDefaultAllRightsReserved
            ) | (
                QtiProfileDiagnosticCode::Policy,
                QtiSafeDiagnosticTemplate::PleDefaultEmptyTags
            ) | (
                QtiProfileDiagnosticCode::Policy,
                QtiSafeDiagnosticTemplate::PleDefaultNoFeedback
            ) | (
                QtiProfileDiagnosticCode::Points,
                QtiSafeDiagnosticTemplate::BlackboardPointsDefaulted
            ) | (
                QtiProfileDiagnosticCode::Markup,
                QtiSafeDiagnosticTemplate::UnsupportedMarkup
            ) | (
                QtiProfileDiagnosticCode::ResponseProcessing,
                QtiSafeDiagnosticTemplate::UnsupportedResponseProcessing
            ) | (
                QtiProfileDiagnosticCode::Shuffle,
                QtiSafeDiagnosticTemplate::UnsupportedItemShape
            ) | (
                QtiProfileDiagnosticCode::ItemShape,
                QtiSafeDiagnosticTemplate::MissingRequiredField
            ) | (
                QtiProfileDiagnosticCode::Points,
                QtiSafeDiagnosticTemplate::MissingRequiredField
            ) | (
                QtiProfileDiagnosticCode::ResponseCardinality,
                QtiSafeDiagnosticTemplate::MissingRequiredField
            ) | (
                QtiProfileDiagnosticCode::DuplicateChoiceId,
                QtiSafeDiagnosticTemplate::MissingRequiredField
            ) | (
                QtiProfileDiagnosticCode::ItemShape,
                QtiSafeDiagnosticTemplate::UnsupportedItemShape
            ) | (
                QtiProfileDiagnosticCode::QuestionType,
                QtiSafeDiagnosticTemplate::UnsupportedItemShape
            ) | (
                QtiProfileDiagnosticCode::ResponseCardinality,
                QtiSafeDiagnosticTemplate::UnsupportedItemShape
            ) | (
                QtiProfileDiagnosticCode::ChoiceCount,
                QtiSafeDiagnosticTemplate::UnsupportedItemShape
            ) | (
                QtiProfileDiagnosticCode::DuplicateChoiceId,
                QtiSafeDiagnosticTemplate::UnsupportedItemShape
            ) | (
                QtiProfileDiagnosticCode::CorrectResponse,
                QtiSafeDiagnosticTemplate::UnsupportedItemShape
            ) | (
                QtiProfileDiagnosticCode::Points,
                QtiSafeDiagnosticTemplate::UnsupportedItemShape
            ) | (
                QtiProfileDiagnosticCode::Feedback,
                QtiSafeDiagnosticTemplate::UnsupportedItemShape
            ) | (
                QtiProfileDiagnosticCode::Media,
                QtiSafeDiagnosticTemplate::UnsupportedItemShape
            )
        );
        if !allowed {
            return Err(QtiMappedItemError::UnsafeDiagnostic);
        }
        Ok(Self {
            code,
            location: location.text(),
            detail: template.text().to_string(),
        })
    }

    /// Stable code safe to project into an instructor-visible report.
    pub fn code(&self) -> QtiProfileDiagnosticCode {
        self.code
    }

    /// Bounded structural location with no source-value interpolation.
    pub fn location(&self) -> &str {
        &self.location
    }

    /// Closed template text with no source-value interpolation.
    pub fn detail(&self) -> &str {
        &self.detail
    }

    pub(super) fn checksum_diagnostic(&self) -> QtiProfileDiagnostic {
        QtiProfileDiagnostic {
            code: self.code,
            location: self.location.clone(),
            detail: self.detail.clone(),
        }
    }
}

/// Closed PLE authoring defaults that every v1 QTI conversion applies.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QtiPleDefault {
    UnlimitedAttempts,
    Untimed,
    EnglishUs,
    AllRightsReserved,
    EmptyTags,
    NoFeedback,
}

#[allow(dead_code)]
impl QtiPleDefault {
    pub(super) const ALL: [Self; 6] = [
        Self::UnlimitedAttempts,
        Self::Untimed,
        Self::EnglishUs,
        Self::AllRightsReserved,
        Self::EmptyTags,
        Self::NoFeedback,
    ];
    pub(super) fn safe_diagnostic(self) -> QtiSafeDiagnostic {
        let template = match self {
            Self::UnlimitedAttempts => QtiSafeDiagnosticTemplate::PleDefaultUnlimitedAttempts,
            Self::Untimed => QtiSafeDiagnosticTemplate::PleDefaultUntimed,
            Self::EnglishUs => QtiSafeDiagnosticTemplate::PleDefaultEnglishUs,
            Self::AllRightsReserved => QtiSafeDiagnosticTemplate::PleDefaultAllRightsReserved,
            Self::EmptyTags => QtiSafeDiagnosticTemplate::PleDefaultEmptyTags,
            Self::NoFeedback => QtiSafeDiagnosticTemplate::PleDefaultNoFeedback,
        };
        QtiSafeDiagnostic::new(
            QtiProfileDiagnosticCode::Policy,
            QtiSafeDiagnosticLocation::Item,
            template,
        )
        .expect("fixed safe diagnostic is valid")
    }
    pub(super) fn checksum_diagnostic(self) -> QtiProfileDiagnostic {
        let safe = self.safe_diagnostic();
        QtiProfileDiagnostic {
            code: QtiProfileDiagnosticCode::Policy,
            location: safe.location,
            detail: safe.detail,
        }
    }
}

/// Exact source-points declaration permitted by the v1 profile contract.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QtiVendorPointsEvidence {
    Declared(String),
    BlackboardDefaulted,
}

#[allow(dead_code)]
impl QtiVendorPointsEvidence {
    pub(super) fn resolve(
        &self,
        profile: QtiProfileId,
    ) -> Result<(String, Vec<QtiProfileDiagnostic>, Vec<QtiSafeDiagnostic>), QtiMappedItemError>
    {
        match (profile, self) {
            (QtiProfileId::CANVAS | QtiProfileId::GENERIC, Self::Declared(points)) => {
                normalized_points(points)
                    .map(|points| (points, Vec::new(), Vec::new()))
                    .ok_or(QtiMappedItemError::InvalidPoints)
            }
            (QtiProfileId::BLACKBOARD, Self::BlackboardDefaulted) => {
                let safe = QtiSafeDiagnostic::new(
                    QtiProfileDiagnosticCode::Points,
                    QtiSafeDiagnosticLocation::Points,
                    QtiSafeDiagnosticTemplate::BlackboardPointsDefaulted,
                )
                .expect("fixed safe diagnostic is valid");
                let diagnostic = QtiProfileDiagnostic {
                    code: QtiProfileDiagnosticCode::Points,
                    location: safe.location.clone(),
                    detail: safe.detail.clone(),
                };
                Ok(("1.0".to_string(), vec![diagnostic], vec![safe]))
            }
            (QtiProfileId::CANVAS, Self::BlackboardDefaulted) => {
                Err(QtiMappedItemError::ProfilePointsPolicy)
            }
            (QtiProfileId::BLACKBOARD, Self::Declared(_)) => {
                Err(QtiMappedItemError::ProfilePointsPolicy)
            }
            (QtiProfileId::GENERIC, Self::BlackboardDefaulted) => {
                Err(QtiMappedItemError::ProfilePointsPolicy)
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum QtiSafeItemStatus {
    Accepted,
    Rejected,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QtiSafeItemReport {
    pub(super) source_identifier: String,
    pub(super) title: Option<String>,
    pub(super) status: QtiSafeItemStatus,
    pub(super) diagnostics: Vec<QtiSafeDiagnostic>,
    pub(super) defaults: Vec<QtiSafeDiagnostic>,
    pub(super) warnings: Vec<QtiSafeDiagnostic>,
}

impl QtiSafeItemReport {
    pub fn source_identifier(&self) -> &str {
        &self.source_identifier
    }
    pub fn title(&self) -> Option<&str> {
        self.title.as_deref()
    }
    pub fn status(&self) -> QtiSafeItemStatus {
        self.status
    }
    pub fn diagnostics(&self) -> &[QtiSafeDiagnostic] {
        &self.diagnostics
    }
    pub fn defaults(&self) -> &[QtiSafeDiagnostic] {
        &self.defaults
    }
    pub fn warnings(&self) -> &[QtiSafeDiagnostic] {
        &self.warnings
    }
}

#[allow(dead_code)]
pub(super) fn char_count(value: &str) -> usize {
    value.chars().count()
}
#[allow(dead_code)]
fn normalized_points(value: &str) -> Option<String> {
    let points = value.parse::<f64>().ok()?;
    if !points.is_finite() || points < 0.0 {
        return None;
    }
    let normalized = if points == 0.0 { 0.0 } else { points };
    Some(format!("{normalized:?}"))
}
