//! Recursively closed browser contracts for the rehearsal route.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};

use crate::response::{ChoiceId, HotspotPoint, MatchPair, TextEntryAnswer};
use crate::{
    ActivityTimestamp, PresentationDigestTokenV1, RehearsalBackendSupport, RehearsalRunReceipt,
    RenderedItemIdV1, StudentResponse,
};

/// Maximum content blocks in each visible rehearsal content vector.
pub const MAX_REHEARSAL_PRESENTATION_BLOCKS: usize = 32;
/// Maximum Unicode scalar values in one visible rehearsal string.
pub const MAX_REHEARSAL_PRESENTATION_TEXT_SCALARS: usize = 16 * 1024;
/// Maximum table columns in a visible rehearsal table.
pub const MAX_REHEARSAL_TABLE_COLUMNS: usize = 32;
/// Maximum table rows in a visible rehearsal table.
pub const MAX_REHEARSAL_TABLE_ROWS: usize = 128;

/// Opaque rehearsal asset token resolved only by the authorized server route.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct RehearsalAssetReferenceV1(String);

impl RehearsalAssetReferenceV1 {
    /// Parses the stable public token spelling.
    pub fn parse(value: impl Into<String>) -> Result<Self, &'static str> {
        let value = value.into();
        let valid = value.strip_prefix("RA-").is_some_and(|suffix| {
            (16..=64).contains(&suffix.len())
                && suffix
                    .bytes()
                    .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
        });
        valid.then_some(Self(value)).ok_or(
            "rehearsal asset reference must be RA- plus 16 to 64 lowercase alphanumeric characters",
        )
    }

    /// Returns the validated public token.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for RehearsalAssetReferenceV1 {
    type Error = &'static str;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::parse(value)
    }
}

impl From<RehearsalAssetReferenceV1> for String {
    fn from(value: RehearsalAssetReferenceV1) -> Self {
        value.0
    }
}

/// Recursively closed safe content vocabulary for a rehearsal browser screen.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum RehearsalContentBlockV1 {
    Text {
        markdown: String,
    },
    Math {
        latex: String,
        description: String,
    },
    Image {
        asset: RehearsalAssetReferenceV1,
        description: String,
    },
    Code {
        language: String,
        source: String,
    },
    Table {
        headers: Vec<String>,
        rows: Vec<Vec<String>>,
        description: String,
    },
}

/// One safe selectable item in a rehearsal presentation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RehearsalPresentedChoiceV1 {
    pub id: RenderedItemIdV1,
    pub body: Vec<RehearsalContentBlockV1>,
}

/// One safe text-entry slot in a rehearsal presentation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RehearsalPresentedBlankV1 {
    pub id: RenderedItemIdV1,
    pub label: Vec<RehearsalContentBlockV1>,
    pub max_characters: u32,
}

/// One accessible selectable hotspot region.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RehearsalHotspotRegionV1 {
    pub id: RenderedItemIdV1,
    pub label: Vec<RehearsalContentBlockV1>,
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
}

/// An image-backed normalized hotspot surface.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RehearsalHotspotSurfaceV1 {
    pub id: RenderedItemIdV1,
    pub asset: RehearsalAssetReferenceV1,
    pub description: String,
    pub regions: Vec<RehearsalHotspotRegionV1>,
}

/// Native automated response widgets admitted by the rehearsal public protocol.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum RehearsalResponseSchemaV1 {
    SingleChoice {
        choices: Vec<RehearsalPresentedChoiceV1>,
    },
    MultipleAnswer {
        choices: Vec<RehearsalPresentedChoiceV1>,
        minimum: u32,
        maximum: u32,
    },
    FillIn {
        max_characters: u32,
    },
    MultiFillIn {
        blanks: Vec<RehearsalPresentedBlankV1>,
    },
    Numerical {
        max_characters: u32,
        displayed_unit: Option<String>,
    },
    Matching {
        prompts: Vec<RehearsalPresentedChoiceV1>,
        choices: Vec<RehearsalPresentedChoiceV1>,
        reuse_choices: bool,
    },
    Ordering {
        items: Vec<RehearsalPresentedChoiceV1>,
    },
    Hotspot {
        surface: RehearsalHotspotSurfaceV1,
        minimum: u32,
        maximum: u32,
    },
}

/// Answer-free content and response schema for the one active rehearsal item.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RehearsalQuestionPresentationV1 {
    pub title: String,
    pub prompt: Vec<RehearsalContentBlockV1>,
    pub response: RehearsalResponseSchemaV1,
}

/// Full server-only commitment for a version-one rehearsal presentation.
/// This is deliberately not serializable: browsers receive only its fixed
/// 128-bit `PresentationDigestTokenV1` prefix.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RehearsalPresentationDigestV1([u8; 32]);

impl RehearsalPresentationDigestV1 {
    /// Derives the complete canonical commitment after validating public data.
    pub fn derive(
        presentation: &RehearsalQuestionPresentationV1,
    ) -> Result<Self, RehearsalWireValidationError> {
        presentation.validate()?;
        Ok(Self(
            Sha256::digest(rehearsal_presentation_descriptor_bytes_v1(presentation)?).into(),
        ))
    }

    /// Returns the browser-safe fixed-width prefix token.
    pub fn public_token(self) -> PresentationDigestTokenV1 {
        PresentationDigestTokenV1::from_digest(&self.0)
    }

    /// Returns server persistence bytes without exposing a wire encoding.
    pub fn as_bytes(self) -> [u8; 32] {
        self.0
    }

    /// Restores stored server-only commitment bytes for persistence decoding.
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }
}

/// Canonical, versioned descriptor for the complete answer-free rehearsal
/// presentation.  This is the one byte contract used by every server-side
/// presentation commitment; its explicit domain/version prevents accidental
/// reuse of an unrelated JSON hash.
pub fn rehearsal_presentation_descriptor_bytes_v1(
    presentation: &RehearsalQuestionPresentationV1,
) -> Result<Vec<u8>, RehearsalWireValidationError> {
    presentation.validate()?;
    let canonical_json = serde_json::to_vec(presentation)
        .map_err(|_| RehearsalWireValidationError::InvalidPresentation)?;
    let mut bytes = b"ple:rehearsal:presentation:v1\0".to_vec();
    bytes.extend_from_slice(&canonical_json);
    Ok(bytes)
}

/// The complete answer-free screen for an active rehearsal.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RehearsalActiveScreenV1 {
    pub presentation: RehearsalQuestionPresentationV1,
    pub presentation_digest: PresentationDigestTokenV1,
}

/// Browser-safe progress through the immutable rehearsal assignment.  These
/// are display counts, not frozen ordinals or attempt identities.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RehearsalProgressV1 {
    /// One-based visible item number. Zero means no active item remains.
    pub current: u32,
    pub total: u32,
}

impl RehearsalProgressV1 {
    pub fn new(current: u32, total: u32) -> Result<Self, RehearsalWireValidationError> {
        (total > 0 && current <= total)
            .then_some(Self { current, total })
            .ok_or(RehearsalWireValidationError::InvalidProgress)
    }

    fn validate(self) -> Result<(), RehearsalWireValidationError> {
        Self::new(self.current, self.total).map(|_| ())
    }
}

/// The one external operation whose committed outcome needs reconciliation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RehearsalPendingPhaseV1 {
    Delivery,
    Submission,
}

/// Browser-safe evidence state after a server-owned submission is recorded.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RehearsalEvidenceStatusV1 {
    Recorded,
}

/// Minimal course-local evidence fact visible after deterministic grading.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RehearsalEvidenceSummaryV1 {
    pub status: RehearsalEvidenceStatusV1,
    pub recorded_at: ActivityTimestamp,
}

/// Disclosure-filtered feedback expressed only with rehearsal-safe content.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RehearsalDisclosedFeedbackV1 {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub correctness: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub points_earned: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub points_possible: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hint: Option<Vec<RehearsalContentBlockV1>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub correct_response: Option<Vec<RehearsalContentBlockV1>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rationale: Option<Vec<RehearsalContentBlockV1>>,
}

impl RehearsalDisclosedFeedbackV1 {
    /// Builds a disclosure with no optional information.
    pub fn empty() -> Self {
        Self {
            correctness: None,
            points_earned: None,
            points_possible: None,
            hint: None,
            correct_response: None,
            rationale: None,
        }
    }
}

/// Disclosure-filtered deterministic submission result.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RehearsalSubmissionResultV1 {
    pub feedback: RehearsalDisclosedFeedbackV1,
    pub evidence: RehearsalEvidenceSummaryV1,
}

/// Closed route projection for the normal, mutable rehearsal workflow.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum RehearsalRouteViewV1 {
    Ready {
        rehearsal: RehearsalRunReceipt,
        progress: RehearsalProgressV1,
    },
    Active {
        rehearsal: RehearsalRunReceipt,
        progress: RehearsalProgressV1,
        screen: RehearsalActiveScreenV1,
    },
    Pending {
        rehearsal: RehearsalRunReceipt,
        progress: RehearsalProgressV1,
        phase: RehearsalPendingPhaseV1,
    },
    Result {
        rehearsal: RehearsalRunReceipt,
        progress: RehearsalProgressV1,
        result: RehearsalSubmissionResultV1,
    },
    Expired {
        rehearsal: RehearsalRunReceipt,
        progress: RehearsalProgressV1,
    },
    Completed {
        rehearsal: RehearsalRunReceipt,
        progress: RehearsalProgressV1,
    },
    Unsupported {
        rehearsal: RehearsalRunReceipt,
        support: RehearsalBackendSupport,
    },
    Discarded {
        rehearsal: RehearsalRunReceipt,
    },
}

/// One response bound to the active screen's public presentation digest.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RehearsalSubmissionRequestV1 {
    pub presentation_digest: PresentationDigestTokenV1,
    pub response: StudentResponse,
}

/// Exact empty body for a delivery or discard operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RehearsalEmptyMutationRequestV1 {}

/// Validation failure for a public rehearsal wire command or projection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RehearsalWireValidationError {
    InvalidPresentation,
    InvalidDigest,
    InvalidResponseSchema,
    UnsupportedResponseFamily,
    ResponseFamilyMismatch,
    ResponseDoesNotMatchScreen,
    NonFiniteNumericResponse,
    NonFiniteFeedback,
    SubmissionTooLarge,
    TooManySubmissionEntries,
    InvalidProgress,
}

impl RehearsalQuestionPresentationV1 {
    /// Computes the browser-safe token from the canonical version-one descriptor.
    pub fn digest(&self) -> Result<PresentationDigestTokenV1, RehearsalWireValidationError> {
        Ok(RehearsalPresentationDigestV1::derive(self)?.public_token())
    }

    /// Verifies bounded content and a native response schema.
    pub fn validate(&self) -> Result<(), RehearsalWireValidationError> {
        validate_text(&self.title)?;
        validate_content_blocks(&self.prompt)?;
        validate_rehearsal_response_schema(&self.response)
    }
}

impl RehearsalActiveScreenV1 {
    /// Constructs a screen whose digest is bound to its complete safe presentation.
    pub fn new(
        presentation: RehearsalQuestionPresentationV1,
    ) -> Result<Self, RehearsalWireValidationError> {
        let presentation_digest = presentation.digest()?;
        Ok(Self {
            presentation,
            presentation_digest,
        })
    }

    /// Recomputes the canonical digest rather than trusting a persisted token.
    pub fn validate(&self) -> Result<(), RehearsalWireValidationError> {
        (self.presentation.digest()? == self.presentation_digest)
            .then_some(())
            .ok_or(RehearsalWireValidationError::InvalidDigest)
    }

    /// Re-establishes the complete server commitment and validates its public
    /// browser token rather than trusting either persisted field.
    pub fn commitment(
        &self,
    ) -> Result<RehearsalPresentationDigestV1, RehearsalWireValidationError> {
        let commitment = RehearsalPresentationDigestV1::derive(&self.presentation)?;
        (commitment.public_token() == self.presentation_digest)
            .then_some(commitment)
            .ok_or(RehearsalWireValidationError::InvalidDigest)
    }
}

impl RehearsalSubmissionResultV1 {
    /// Verifies finite disclosed scores and recursively closed feedback content.
    pub fn validate(&self) -> Result<(), RehearsalWireValidationError> {
        let feedback = &self.feedback;
        if feedback
            .points_earned
            .is_some_and(|value| !value.is_finite())
            || feedback
                .points_possible
                .is_some_and(|value| !value.is_finite())
        {
            return Err(RehearsalWireValidationError::NonFiniteFeedback);
        }
        for content in [
            &feedback.hint,
            &feedback.correct_response,
            &feedback.rationale,
        ]
        .into_iter()
        .flatten()
        {
            validate_content_blocks(content)?;
        }
        Ok(())
    }
}

impl RehearsalRouteViewV1 {
    /// Verifies the closed view before immutable persistence and after hydration.
    pub fn validate(&self) -> Result<(), RehearsalWireValidationError> {
        match self {
            Self::Active {
                progress, screen, ..
            } => {
                progress.validate()?;
                screen.validate()
            }
            Self::Result {
                progress, result, ..
            } => {
                progress.validate()?;
                result.validate()
            }
            Self::Ready { progress, .. }
            | Self::Pending { progress, .. }
            | Self::Expired { progress, .. }
            | Self::Completed { progress, .. } => progress.validate(),
            Self::Unsupported { .. } | Self::Discarded { .. } => Ok(()),
        }
    }
}

impl RehearsalSubmissionRequestV1 {
    /// Enforces direct Store submission bounds as well as HTTP boundaries.
    pub fn validate(&self) -> Result<(), RehearsalWireValidationError> {
        validate_response_size(&self.response)?;
        let bytes = serde_json::to_vec(self)
            .map_err(|_| RehearsalWireValidationError::SubmissionTooLarge)?;
        (bytes.len() <= super::MAX_REHEARSAL_ACCEPTED_SUBMISSION_BYTES)
            .then_some(())
            .ok_or(RehearsalWireValidationError::SubmissionTooLarge)
    }

    /// Rejects same-family malformed answers before deterministic grading.
    pub fn validate_for_screen(
        &self,
        screen: &RehearsalActiveScreenV1,
    ) -> Result<(), RehearsalWireValidationError> {
        self.validate()?;
        screen.validate()?;
        if self.presentation_digest != screen.presentation_digest {
            return Err(RehearsalWireValidationError::InvalidDigest);
        }
        validate_rehearsal_response_for_schema(&screen.presentation.response, &self.response)
    }
}

/// Validates a response against the exact frozen safe response schema.
pub fn validate_rehearsal_response_for_schema(
    schema: &RehearsalResponseSchemaV1,
    response: &StudentResponse,
) -> Result<(), RehearsalWireValidationError> {
    match (schema, response) {
        (
            RehearsalResponseSchemaV1::SingleChoice { choices },
            StudentResponse::MultipleChoice { selected },
        ) => valid_exactly_selected(selected, choices, 1, 1),
        (
            RehearsalResponseSchemaV1::MultipleAnswer {
                choices,
                minimum,
                maximum,
            },
            StudentResponse::MultipleChoice { selected },
        ) => valid_exactly_selected(selected, choices, *minimum as usize, *maximum as usize),
        (
            RehearsalResponseSchemaV1::FillIn { max_characters },
            StudentResponse::ShortText { text },
        ) => valid_text_response(text, *max_characters),
        (
            RehearsalResponseSchemaV1::Numerical { max_characters, .. },
            StudentResponse::Numeric { value },
        ) => {
            if !value.is_finite() {
                return Err(RehearsalWireValidationError::NonFiniteNumericResponse);
            }
            valid_text_response(&value.to_string(), *max_characters)
        }
        (
            RehearsalResponseSchemaV1::MultiFillIn { blanks },
            StudentResponse::MultiBlank { answers },
        ) => valid_blanks(answers, blanks),
        (
            RehearsalResponseSchemaV1::Matching {
                prompts,
                choices,
                reuse_choices,
            },
            StudentResponse::Matching { matches },
        ) => valid_matches(matches, prompts, choices, *reuse_choices),
        (RehearsalResponseSchemaV1::Ordering { items }, StudentResponse::Ordering { order }) => {
            valid_ordering(order, items)
        }
        (
            RehearsalResponseSchemaV1::Hotspot {
                surface,
                minimum,
                maximum,
            },
            StudentResponse::Hotspot { points },
        ) => valid_points(points, surface, *minimum as usize, *maximum as usize),
        (_, StudentResponse::FileUpload { .. } | StudentResponse::ExternalTool {}) => {
            Err(RehearsalWireValidationError::UnsupportedResponseFamily)
        }
        _ => Err(RehearsalWireValidationError::ResponseFamilyMismatch),
    }
}

fn validate_rehearsal_response_schema(
    schema: &RehearsalResponseSchemaV1,
) -> Result<(), RehearsalWireValidationError> {
    let mut ids = BTreeSet::new();
    match schema {
        RehearsalResponseSchemaV1::SingleChoice { choices } if choices.len() >= 2 => {
            validate_choices(choices, &mut ids)
        }
        RehearsalResponseSchemaV1::MultipleAnswer {
            choices,
            minimum,
            maximum,
        } if minimum <= maximum && *maximum <= choices.len() as u32 => {
            validate_choices(choices, &mut ids)
        }
        RehearsalResponseSchemaV1::FillIn { max_characters }
        | RehearsalResponseSchemaV1::Numerical { max_characters, .. }
            if *max_characters > 0 =>
        {
            Ok(())
        }
        RehearsalResponseSchemaV1::MultiFillIn { blanks }
            if !blanks.is_empty() && blanks.len() <= MAX_REHEARSAL_PRESENTATION_BLOCKS =>
        {
            for blank in blanks {
                if blank.max_characters == 0 || !ids.insert(blank.id.clone()) {
                    return Err(RehearsalWireValidationError::InvalidResponseSchema);
                }
                validate_content_blocks(&blank.label)?;
            }
            Ok(())
        }
        RehearsalResponseSchemaV1::Matching {
            prompts,
            choices,
            reuse_choices,
        } if !prompts.is_empty()
            && !choices.is_empty()
            && (*reuse_choices || prompts.len() <= choices.len()) =>
        {
            validate_choices(prompts, &mut ids)?;
            validate_choices(choices, &mut ids)
        }
        RehearsalResponseSchemaV1::Ordering { items } if items.len() >= 2 => {
            validate_choices(items, &mut ids)
        }
        RehearsalResponseSchemaV1::Hotspot {
            surface,
            minimum,
            maximum,
        } if !surface.regions.is_empty()
            && *minimum <= *maximum
            && *maximum <= surface.regions.len() as u32 =>
        {
            validate_hotspot(surface, &mut ids)
        }
        _ => Err(RehearsalWireValidationError::InvalidResponseSchema),
    }
}

fn validate_choices(
    items: &[RehearsalPresentedChoiceV1],
    ids: &mut BTreeSet<RenderedItemIdV1>,
) -> Result<(), RehearsalWireValidationError> {
    if items.is_empty() || items.len() > MAX_REHEARSAL_PRESENTATION_BLOCKS {
        return Err(RehearsalWireValidationError::InvalidResponseSchema);
    }
    for item in items {
        if !ids.insert(item.id.clone()) {
            return Err(RehearsalWireValidationError::InvalidResponseSchema);
        }
        validate_content_blocks(&item.body)?;
    }
    Ok(())
}

fn validate_hotspot(
    surface: &RehearsalHotspotSurfaceV1,
    ids: &mut BTreeSet<RenderedItemIdV1>,
) -> Result<(), RehearsalWireValidationError> {
    if !ids.insert(surface.id.clone()) || surface.regions.len() > MAX_REHEARSAL_PRESENTATION_BLOCKS
    {
        return Err(RehearsalWireValidationError::InvalidResponseSchema);
    }
    validate_text(&surface.description)?;
    for region in &surface.regions {
        if !ids.insert(region.id.clone())
            || region.width == 0
            || region.height == 0
            || u32::from(region.x) + u32::from(region.width) > 10_000
            || u32::from(region.y) + u32::from(region.height) > 10_000
        {
            return Err(RehearsalWireValidationError::InvalidResponseSchema);
        }
        validate_content_blocks(&region.label)?;
    }
    Ok(())
}

fn validate_content_blocks(
    blocks: &[RehearsalContentBlockV1],
) -> Result<(), RehearsalWireValidationError> {
    if blocks.len() > MAX_REHEARSAL_PRESENTATION_BLOCKS {
        return Err(RehearsalWireValidationError::InvalidPresentation);
    }
    for block in blocks {
        match block {
            RehearsalContentBlockV1::Text { markdown } => validate_text(markdown)?,
            RehearsalContentBlockV1::Math { latex, description } => {
                validate_text(latex)?;
                validate_text(description)?;
            }
            RehearsalContentBlockV1::Image { description, .. } => validate_text(description)?,
            RehearsalContentBlockV1::Code { language, source } => {
                validate_text(language)?;
                validate_text(source)?;
            }
            RehearsalContentBlockV1::Table {
                headers,
                rows,
                description,
            } => {
                if headers.is_empty()
                    || headers.len() > MAX_REHEARSAL_TABLE_COLUMNS
                    || rows.len() > MAX_REHEARSAL_TABLE_ROWS
                    || rows.iter().any(|row| row.len() != headers.len())
                {
                    return Err(RehearsalWireValidationError::InvalidPresentation);
                }
                for value in headers
                    .iter()
                    .chain(rows.iter().flatten())
                    .chain(std::iter::once(description))
                {
                    validate_text(value)?;
                }
            }
        }
    }
    Ok(())
}

fn validate_text(value: &str) -> Result<(), RehearsalWireValidationError> {
    (!value
        .chars()
        .any(|character| character.is_control() && !matches!(character, '\n' | '\r' | '\t'))
        && value.chars().count() <= MAX_REHEARSAL_PRESENTATION_TEXT_SCALARS)
        .then_some(())
        .ok_or(RehearsalWireValidationError::InvalidPresentation)
}

fn validate_response_size(response: &StudentResponse) -> Result<(), RehearsalWireValidationError> {
    let count = match response {
        StudentResponse::Numeric { value } => {
            if !value.is_finite() {
                return Err(RehearsalWireValidationError::NonFiniteNumericResponse);
            }
            1
        }
        StudentResponse::MultipleChoice { selected } => selected.len(),
        StudentResponse::ShortText { .. } => 1,
        StudentResponse::MultiBlank { answers } => answers.len(),
        StudentResponse::Matching { matches } => matches.len(),
        StudentResponse::Ordering { order } => order.len(),
        StudentResponse::Hotspot { points } => points.len(),
        StudentResponse::FileUpload { .. } | StudentResponse::ExternalTool {} => {
            return Err(RehearsalWireValidationError::UnsupportedResponseFamily);
        }
    };
    (count <= super::MAX_REHEARSAL_ACCEPTED_SUBMISSION_ENTRIES)
        .then_some(())
        .ok_or(RehearsalWireValidationError::TooManySubmissionEntries)
}

fn valid_exactly_selected(
    selected: &[ChoiceId],
    choices: &[RehearsalPresentedChoiceV1],
    minimum: usize,
    maximum: usize,
) -> Result<(), RehearsalWireValidationError> {
    if selected.len() < minimum || selected.len() > maximum {
        return Err(RehearsalWireValidationError::ResponseDoesNotMatchScreen);
    }
    let allowed: BTreeSet<&str> = choices.iter().map(|choice| choice.id.as_str()).collect();
    let actual: BTreeSet<&str> = selected.iter().map(ChoiceId::as_str).collect();
    (actual.len() == selected.len() && actual.iter().all(|id| allowed.contains(id)))
        .then_some(())
        .ok_or(RehearsalWireValidationError::ResponseDoesNotMatchScreen)
}

fn valid_text_response(text: &str, max: u32) -> Result<(), RehearsalWireValidationError> {
    (text.chars().count() <= max as usize && !text.chars().any(char::is_control))
        .then_some(())
        .ok_or(RehearsalWireValidationError::ResponseDoesNotMatchScreen)
}

fn valid_blanks(
    answers: &[TextEntryAnswer],
    blanks: &[RehearsalPresentedBlankV1],
) -> Result<(), RehearsalWireValidationError> {
    if answers.len() != blanks.len() {
        return Err(RehearsalWireValidationError::ResponseDoesNotMatchScreen);
    }
    let allowed: std::collections::BTreeMap<&str, u32> = blanks
        .iter()
        .map(|blank| (blank.id.as_str(), blank.max_characters))
        .collect();
    let mut seen = BTreeSet::new();
    for answer in answers {
        let Some(max) = allowed.get(answer.slot.as_str()) else {
            return Err(RehearsalWireValidationError::ResponseDoesNotMatchScreen);
        };
        if !seen.insert(answer.slot.as_str()) {
            return Err(RehearsalWireValidationError::ResponseDoesNotMatchScreen);
        }
        valid_text_response(&answer.text, *max)?;
    }
    Ok(())
}

fn valid_matches(
    matches: &[MatchPair],
    prompts: &[RehearsalPresentedChoiceV1],
    choices: &[RehearsalPresentedChoiceV1],
    reuse: bool,
) -> Result<(), RehearsalWireValidationError> {
    if matches.len() != prompts.len() {
        return Err(RehearsalWireValidationError::ResponseDoesNotMatchScreen);
    }
    let prompts: BTreeSet<&str> = prompts.iter().map(|value| value.id.as_str()).collect();
    let choices: BTreeSet<&str> = choices.iter().map(|value| value.id.as_str()).collect();
    let mut seen_prompts = BTreeSet::new();
    let mut seen_choices = BTreeSet::new();
    for pair in matches {
        if !prompts.contains(pair.prompt.as_str())
            || !choices.contains(pair.choice.as_str())
            || !seen_prompts.insert(pair.prompt.as_str())
            || (!reuse && !seen_choices.insert(pair.choice.as_str()))
        {
            return Err(RehearsalWireValidationError::ResponseDoesNotMatchScreen);
        }
    }
    Ok(())
}

fn valid_ordering(
    order: &[ChoiceId],
    items: &[RehearsalPresentedChoiceV1],
) -> Result<(), RehearsalWireValidationError> {
    if order.len() != items.len() {
        return Err(RehearsalWireValidationError::ResponseDoesNotMatchScreen);
    }
    let expected: BTreeSet<&str> = items.iter().map(|item| item.id.as_str()).collect();
    let actual: BTreeSet<&str> = order.iter().map(ChoiceId::as_str).collect();
    (expected == actual && actual.len() == order.len())
        .then_some(())
        .ok_or(RehearsalWireValidationError::ResponseDoesNotMatchScreen)
}

fn valid_points(
    points: &[HotspotPoint],
    surface: &RehearsalHotspotSurfaceV1,
    minimum: usize,
    maximum: usize,
) -> Result<(), RehearsalWireValidationError> {
    if points.len() < minimum || points.len() > maximum {
        return Err(RehearsalWireValidationError::ResponseDoesNotMatchScreen);
    }
    let mut seen = BTreeSet::new();
    for point in points {
        let in_region = surface.regions.iter().any(|region| {
            point.x >= region.x
                && point.x <= region.x.saturating_add(region.width)
                && point.y >= region.y
                && point.y <= region.y.saturating_add(region.height)
        });
        if point.x > 10_000 || point.y > 10_000 || !in_region || !seen.insert((point.x, point.y)) {
            return Err(RehearsalWireValidationError::ResponseDoesNotMatchScreen);
        }
    }
    Ok(())
}
