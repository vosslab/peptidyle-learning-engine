//! Browser-safe content atoms and Question Library metadata.
//!
//! Complete source remains format-specific and opaque. This module deliberately
//! does not define a generic draft or published source record.

use serde::{Deserialize, Serialize};

use crate::identity::{QuestionImageAssetId, WorkspaceId};
use crate::question_citation::QuestionCitation;
use crate::question_license::QuestionLicense;
use crate::question_tag::Tag;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuestionImageAssetTuple {
    pub question_image_asset_id: QuestionImageAssetId,
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
        question_image_asset_tuple: QuestionImageAssetTuple,
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
    /// Fully reviewed PGML source supplied with explicit import/authoring
    /// provenance. The suffix alone never classifies a Question Source.
    WebworkPgml,
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
pub fn validate_question_title(question_title: &str) -> Result<(), QuestionTitleError> {
    if question_title.trim().is_empty() {
        return Err(QuestionTitleError::Blank);
    }
    if question_title.chars().count() > MAX_QUESTION_TITLE_UNICODE_SCALARS {
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
    pub question_title: String,
    pub question_description: String,
    pub tags: Vec<Tag>,
    pub question_license: Option<QuestionLicense>,
    pub question_citation: Option<QuestionCitation>,
    pub language: String,
}
impl QuestionMetadata {
    pub fn validate_question_title(&self) -> Result<(), QuestionTitleError> {
        validate_question_title(&self.question_title)
    }
    pub fn validate_question_description(&self) -> Result<(), QuestionDescriptionError> {
        validate_question_description(&self.question_description)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DraftQuestionSummary {
    pub draft_question_id: uuid::Uuid,
    pub workspace_id: WorkspaceId,
    pub question_title: String,
    pub question_backend: crate::question_library::QuestionBackend,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::QuestionImageAssetId;
    use uuid::Uuid;

    #[test]
    fn question_image_asset_tuple_serializes_question_image_asset_id() {
        let tuple = QuestionImageAssetTuple {
            question_image_asset_id: QuestionImageAssetId::from_uuid(Uuid::from_u128(7)),
            checksum: "a".repeat(64),
        };
        let wire = serde_json::to_value(&tuple).expect("tuple serializes");
        assert_eq!(
            wire["questionImageAssetId"],
            "00000000-0000-0000-0000-000000000007"
        );
        assert!(wire.get("questionAssetId").is_none());
        assert!(
            serde_json::from_value::<QuestionImageAssetTuple>(serde_json::json!({
                "questionAssetId": "00000000-0000-0000-0000-000000000007",
                "checksum": "a".repeat(64)
            }))
            .is_err()
        );
    }
}
