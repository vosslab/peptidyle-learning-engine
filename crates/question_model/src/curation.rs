//! Browser-safe private Question Folder contracts.

use std::num::NonZeroU64;

use serde::{Deserialize, Serialize};

use crate::{
    QuestionFolderReference, QuestionId, QuestionSearchFilter, QuestionSummary,
    QuestionVersionAvailability, SavedQuestionSearchReference,
};

/// Maximum ordered Question IDs accepted in one atomic Question Folder replacement.
pub const MAX_QUESTION_FOLDER_MEMBERS: usize = 200;
/// Maximum named Question Folders owned by one Instructor in this installation.
pub const MAX_NAMED_QUESTION_FOLDERS: usize = 100;
/// Maximum personal saved searches owned by one instructor in this installation.
pub const MAX_SAVED_QUESTION_SEARCHES: usize = 100;
/// Maximum trimmed Unicode scalar values in a Question Folder or Saved Question Search title.
pub const MAX_QUESTION_CURATION_TITLE_UNICODE_SCALARS: usize = 200;

/// Strong edit-number evidence for one complete Question Folder state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct QuestionFolderEditNumber(NonZeroU64);

/// Strong edit-number evidence for one saved-search state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct SavedQuestionSearchEditNumber(NonZeroU64);

macro_rules! impl_edit_number {
    ($name:ident) => {
        impl $name {
            pub const INITIAL: Self = Self(NonZeroU64::MIN);
            pub fn new(value: u64) -> Option<Self> {
                (value <= i64::MAX as u64)
                    .then(|| NonZeroU64::new(value))
                    .flatten()
                    .map(Self)
            }
            pub fn value(self) -> u64 {
                self.0.get()
            }
            pub fn checked_next(self) -> Option<Self> {
                self.value().checked_add(1).and_then(Self::new)
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(formatter, "{}", self.value())
            }
        }
        impl std::str::FromStr for $name {
            type Err = &'static str;
            fn from_str(value: &str) -> Result<Self, Self::Err> {
                if value.is_empty()
                    || (value.len() > 1 && value.starts_with('0'))
                    || !value.bytes().all(|byte| byte.is_ascii_digit())
                {
                    return Err("edit number must be a canonical positive decimal string");
                }
                value
                    .parse::<u64>()
                    .ok()
                    .and_then(Self::new)
                    .ok_or("edit number must fit a positive PostgreSQL bigint")
            }
        }
        impl TryFrom<String> for $name {
            type Error = &'static str;
            fn try_from(value: String) -> Result<Self, Self::Error> {
                value.parse()
            }
        }
        impl From<$name> for String {
            fn from(value: $name) -> Self {
                value.to_string()
            }
        }
    };
}

impl_edit_number!(QuestionFolderEditNumber);
impl_edit_number!(SavedQuestionSearchEditNumber);

/// One title validation failure shared by Question Folders and Saved Question Searches.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuestionCurationTitleError {
    Invalid,
}

impl std::fmt::Display for QuestionCurationTitleError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("curation title must be trimmed, nonempty, and within its bound")
    }
}
impl std::error::Error for QuestionCurationTitleError {}

/// Validates the title retained for a curation aggregate.
pub fn validate_question_curation_title(value: &str) -> Result<(), QuestionCurationTitleError> {
    (value == value.trim()
        && !value.is_empty()
        && value.chars().count() <= MAX_QUESTION_CURATION_TITLE_UNICODE_SCALARS)
        .then_some(())
        .ok_or(QuestionCurationTitleError::Invalid)
}

/// Safe current projection of one exact immutable Question Folder member.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QuestionFolderEntryView {
    pub question_id: QuestionId,
    pub summary: QuestionSummary,
    /// Current availability of the entry's exact Question Version.
    pub question_version_availability: QuestionVersionAvailability,
}

/// Browser-safe private Question Folder projection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QuestionFolderSummaryView {
    pub reference: QuestionFolderReference,
    pub title: String,
    pub edit_number: QuestionFolderEditNumber,
}

/// Browser-safe personal saved D1 search meaning.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SavedQuestionSearchView {
    pub reference: SavedQuestionSearchReference,
    pub title: String,
    pub filter: QuestionSearchFilter,
    pub edit_number: SavedQuestionSearchEditNumber,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edit_numbers_are_exact_decimal_strings() {
        let edit_number = QuestionFolderEditNumber::new(42).expect("bounded edit number");
        assert_eq!(serde_json::to_value(edit_number).expect("serializes"), "42");
        assert!("042".parse::<SavedQuestionSearchEditNumber>().is_err());
    }

    #[test]
    fn curation_titles_are_trimmed_and_bounded() {
        assert!(validate_question_curation_title("Exam candidates").is_ok());
        assert!(validate_question_curation_title(" exam candidates").is_err());
        assert!(validate_question_curation_title("").is_err());
    }
}
