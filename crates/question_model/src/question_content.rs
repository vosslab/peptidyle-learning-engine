//! Browser-safe content atoms and Question Library metadata.
//!
//! Complete source remains format-specific and opaque. This module deliberately
//! does not define a generic draft or published source record.

use serde::{Deserialize, Serialize};

use crate::identity::{QuestionAssetId, WorkspaceId};
use crate::question_citation::QuestionCitation;
use crate::question_license::QuestionLicense;
use crate::question_tag::Tag;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuestionAssetReference {
    pub asset: QuestionAssetId,
    pub checksum: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum QuestionContentBlock {
    Text {
        markdown: String,
    },
    Math {
        latex: String,
        description: String,
    },
    Image {
        asset: QuestionAssetReference,
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

pub const MAX_QUESTION_TITLE_UNICODE_SCALARS: usize = 512;
pub const MAX_QUESTION_DESCRIPTION_UNICODE_SCALARS: usize = 4_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum QuestionFormat {
    PleQuestionJson,
    WebworkPg,
    H5p,
    Imathas,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuestionTitleError {
    Blank,
    TooLong,
}
impl std::fmt::Display for QuestionTitleError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Blank => formatter.write_str("question title must not be blank"),
            Self::TooLong => write!(
                formatter,
                "question title must contain at most {MAX_QUESTION_TITLE_UNICODE_SCALARS} Unicode scalar values"
            ),
        }
    }
}
impl std::error::Error for QuestionTitleError {}
pub fn validate_question_title(title: &str) -> Result<(), QuestionTitleError> {
    if title.trim().is_empty() {
        return Err(QuestionTitleError::Blank);
    }
    if title.chars().count() > MAX_QUESTION_TITLE_UNICODE_SCALARS {
        return Err(QuestionTitleError::TooLong);
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuestionDescriptionError {
    Blank,
    TooLong,
}
impl std::fmt::Display for QuestionDescriptionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Blank => formatter.write_str("question description must not be blank"),
            Self::TooLong => write!(
                formatter,
                "question description must contain at most {MAX_QUESTION_DESCRIPTION_UNICODE_SCALARS} Unicode scalar values"
            ),
        }
    }
}
impl std::error::Error for QuestionDescriptionError {}
pub fn validate_question_description(value: &str) -> Result<(), QuestionDescriptionError> {
    if value.trim().is_empty() {
        return Err(QuestionDescriptionError::Blank);
    }
    if value.chars().count() > MAX_QUESTION_DESCRIPTION_UNICODE_SCALARS {
        return Err(QuestionDescriptionError::TooLong);
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuestionMetadata {
    pub title: String,
    pub question_description: String,
    pub tags: Vec<Tag>,
    pub question_license: Option<QuestionLicense>,
    pub question_citation: Option<QuestionCitation>,
    pub language: String,
}
impl QuestionMetadata {
    pub fn validate_title(&self) -> Result<(), QuestionTitleError> {
        validate_question_title(&self.title)
    }
    pub fn validate_question_description(&self) -> Result<(), QuestionDescriptionError> {
        validate_question_description(&self.question_description)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DraftQuestionSummary {
    pub draft_question: crate::DraftQuestionReference,
    pub workspace: WorkspaceId,
    pub authoring_workspace: crate::AuthoringWorkspaceReference,
    pub title: String,
    pub question_backend: crate::question_library::QuestionBackend,
}
