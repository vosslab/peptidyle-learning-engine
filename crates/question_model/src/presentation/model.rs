//! Public version-one presentation values.

use base64::Engine as _;
use serde::{Deserialize, Serialize};

use crate::QuestionRevisionReference;
use crate::course_appearance::CourseThemeId;
use crate::envelope::{QuestionAssetReference, QuestionContentBlock};
use crate::generation::QuestionSeed;
use crate::student_work::{AssignmentId, CourseId, QuestionAttemptId, Timestamp};

/// Four-lowercase-hex identifier for one object in one issued presentation.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct PresentationResponseItemReference(String);

impl PresentationResponseItemReference {
    /// Parses the exact browser wire spelling.
    pub fn parse(value: impl Into<String>) -> Result<Self, &'static str> {
        let value = value.into();
        if value.len() == 4
            && value
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            Ok(Self(value))
        } else {
            Err("rendered item ID must be four lowercase hexadecimal characters")
        }
    }

    /// Constructs a wire ID from the CRC bytes.
    pub(crate) fn from_crc(value: u16) -> Self {
        Self(format!("{value:04x}"))
    }

    /// Returns the validated wire spelling.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub(crate) fn as_u16(&self) -> u16 {
        u16::from_str_radix(&self.0, 16).expect("validated rendered item ID")
    }
}

impl TryFrom<String> for PresentationResponseItemReference {
    type Error = &'static str;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::parse(value)
    }
}

impl From<PresentationResponseItemReference> for String {
    fn from(value: PresentationResponseItemReference) -> Self {
        value.0
    }
}

/// Sixteen-byte server-minted nonce, rendered as 32 lowercase hex characters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct QuestionPresentationNonce([u8; 16]);

impl QuestionPresentationNonce {
    /// Wraps the exact raw nonce bytes.
    pub fn from_bytes(bytes: [u8; 16]) -> Self {
        Self(bytes)
    }

    /// Returns the exact raw nonce bytes used by the codec and persistence.
    pub fn as_bytes(self) -> [u8; 16] {
        self.0
    }

    /// Returns the exact lowercase persistence spelling.
    pub fn to_hex(self) -> String {
        let mut value = String::with_capacity(32);
        for byte in self.0 {
            use std::fmt::Write as _;
            let _ = write!(&mut value, "{byte:02x}");
        }
        value
    }

    /// Parses the exact lowercase persistence spelling.
    pub fn parse(value: &str) -> Result<Self, &'static str> {
        if value.len() != 32
            || !value
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            return Err("presentation nonce must be 32 lowercase hexadecimal characters");
        }
        let mut bytes = [0_u8; 16];
        for (index, chunk) in value.as_bytes().as_chunks::<2>().0.iter().enumerate() {
            let text = std::str::from_utf8(chunk).map_err(|_| "invalid presentation nonce")?;
            bytes[index] =
                u8::from_str_radix(text, 16).map_err(|_| "invalid presentation nonce")?;
        }
        Ok(Self(bytes))
    }
}

impl TryFrom<String> for QuestionPresentationNonce {
    type Error = &'static str;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::parse(&value)
    }
}

impl From<QuestionPresentationNonce> for String {
    fn from(value: QuestionPresentationNonce) -> Self {
        value.to_hex()
    }
}

/// Public 128-bit prefix of a full presentation SHA-256 checksum.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct QuestionPresentationToken(String);

impl QuestionPresentationToken {
    pub(crate) fn from_checksum(bytes: &[u8; 32]) -> Self {
        let encoded = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(&bytes[..16]);
        Self(format!("pd1_{encoded}"))
    }

    /// Parses and validates the exact version-one public checksum token.
    pub fn parse(value: impl Into<String>) -> Result<Self, &'static str> {
        let value = value.into();
        let encoded = value
            .strip_prefix("pd1_")
            .ok_or("presentation checksum lacks the pd1_ prefix")?;
        let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(encoded)
            .map_err(|_| "presentation checksum is not base64url")?;
        if bytes.len() != 16
            || base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes) != encoded
        {
            return Err("presentation checksum must contain one canonical 128-bit prefix");
        }
        Ok(Self(value))
    }

    /// Returns the validated public token.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for QuestionPresentationToken {
    type Error = &'static str;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::parse(value)
    }
}

impl From<QuestionPresentationToken> for String {
    fn from(value: QuestionPresentationToken) -> Self {
        value.0
    }
}

/// One logical asset and the exact rendition selected for this presentation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QuestionAssetRendition {
    /// Exact authored Question Asset selected for this rendering. This reference
    /// identifies content but grants no storage or download authority.
    pub question_asset: QuestionAssetReference,
    /// Checksum of the public rendition selected for this rendering.
    pub rendition_checksum: String,
    /// Intrinsic width of the selected public rendition, when known.
    pub intrinsic_width: Option<u32>,
    /// Intrinsic height of the selected public rendition, when known.
    pub intrinsic_height: Option<u32>,
}

/// One displayed Question Choice with a presentation-scoped ID.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PresentedQuestionChoice {
    pub id: PresentationResponseItemReference,
    pub body: Vec<QuestionContentBlock>,
}

/// One displayed Matching Prompt with a presentation-scoped ID.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PresentedMatchingPrompt {
    pub id: PresentationResponseItemReference,
    pub body: Vec<QuestionContentBlock>,
}

/// One displayed Matching Choice with a presentation-scoped ID.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PresentedMatchingChoice {
    pub id: PresentationResponseItemReference,
    pub body: Vec<QuestionContentBlock>,
}

/// One displayed Ordering Item with a presentation-scoped ID.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PresentedOrderingItem {
    pub id: PresentationResponseItemReference,
    pub body: Vec<QuestionContentBlock>,
}

/// Shared descriptor and asset access for exact public response-item records.
pub trait PresentedResponseItemContent {
    fn presentation_item_id(&self) -> &PresentationResponseItemReference;
    fn presentation_item_body(&self) -> &[QuestionContentBlock];
}

macro_rules! presented_response_item {
    ($record:ty) => {
        impl PresentedResponseItemContent for $record {
            fn presentation_item_id(&self) -> &PresentationResponseItemReference {
                &self.id
            }

            fn presentation_item_body(&self) -> &[QuestionContentBlock] {
                &self.body
            }
        }
    };
}

presented_response_item!(PresentedQuestionChoice);
presented_response_item!(PresentedMatchingPrompt);
presented_response_item!(PresentedMatchingChoice);
presented_response_item!(PresentedOrderingItem);

/// One displayed text-entry target in a multi-blank response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PresentedTextEntrySlot {
    pub id: PresentationResponseItemReference,
    pub label: Vec<QuestionContentBlock>,
    pub max_characters: u32,
}

/// One public Hotspot Region used by both pointer and no-mouse controls.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PresentedHotspotRegion {
    /// Presentation-scoped identifier for this selectable region.
    pub id: PresentationResponseItemReference,
    pub label: Vec<QuestionContentBlock>,
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
}

/// Image-backed surface containing selectable regions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PresentedHotspotSurface {
    pub id: PresentationResponseItemReference,
    pub asset: QuestionAssetReference,
    pub description: String,
    pub regions: Vec<PresentedHotspotRegion>,
}

/// Answer-free browser widget schema selected by the persisted attempt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum QuestionPresentationResponseFormat {
    SingleChoice {
        choices: Vec<PresentedQuestionChoice>,
    },
    MultipleAnswer {
        choices: Vec<PresentedQuestionChoice>,
        minimum: u32,
        maximum: u32,
    },
    FillIn {
        max_characters: u32,
    },
    MultiFillIn {
        blanks: Vec<PresentedTextEntrySlot>,
    },
    Numerical {
        max_characters: u32,
        displayed_unit: Option<String>,
    },
    Matching {
        prompts: Vec<PresentedMatchingPrompt>,
        choices: Vec<PresentedMatchingChoice>,
        reuse_choices: bool,
    },
    Ordering {
        items: Vec<PresentedOrderingItem>,
    },
    Hotspot {
        surface: PresentedHotspotSurface,
        minimum: u32,
        maximum: u32,
    },
}

/// Complete answer-free question state presented to one student.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QuestionPresentation {
    pub question_revision: QuestionRevisionReference,
    pub seed: QuestionSeed,
    pub presentation_nonce: QuestionPresentationNonce,
    pub title: String,
    pub prompt: Vec<QuestionContentBlock>,
    pub response: QuestionPresentationResponseFormat,
}

/// Minimal active attempt fields needed by the student screen.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StudentAttemptDescriptor {
    pub id: QuestionAttemptId,
    pub deadline: Option<Timestamp>,
    pub presentation_token: QuestionPresentationToken,
}

/// Course and Assignment shell needed for authorized Student navigation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StudentAssignmentAttemptScreenScope {
    pub course: CourseId,
    pub assignment: AssignmentId,
    pub theme: CourseThemeId,
}

/// Student-visible Assignment Attempt context, without storage or policy internals.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StudentAssignmentAttemptScreenAttempt {
    pub number: u32,
}

/// One consolidated active student screen response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StudentAssignmentAttemptScreen {
    pub scope: StudentAssignmentAttemptScreenScope,
    pub assignment_attempt: StudentAssignmentAttemptScreenAttempt,
    pub attempt: StudentAttemptDescriptor,
    pub presentation: QuestionPresentation,
}
