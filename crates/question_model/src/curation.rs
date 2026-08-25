//! Browser-safe personal and institution problem-curation contracts.

use std::num::NonZeroU64;

use serde::{Deserialize, Serialize};

use crate::{
    CatalogProblemSummary, CatalogSearchFilter, ProblemCollectionReference, QuestionId,
    SavedProblemSearchReference,
};

/// Maximum ordered Question IDs accepted in one atomic collection replacement.
pub const MAX_PROBLEM_COLLECTION_MEMBERS: usize = 200;
/// Maximum named collections owned by one instructor in one tenant.
pub const MAX_NAMED_PROBLEM_COLLECTIONS: usize = 100;
/// Maximum personal saved searches owned by one instructor in one tenant.
pub const MAX_SAVED_PROBLEM_SEARCHES: usize = 100;
/// Maximum trimmed Unicode scalar values in a collection or saved-search title.
pub const MAX_PROBLEM_CURATION_TITLE_UNICODE_SCALARS: usize = 200;

/// Fixed Favorites or an ordinary named collection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ProblemCollectionKind {
    Favorites,
    Named,
}

/// Visibility chosen for a named collection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ProblemCollectionVisibility {
    Private,
    Institution,
}

/// Current safe selection state for a retained exact collection member.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ProblemCollectionSelectionAvailability {
    /// The current publication remains eligible for a new selection.
    Available,
    /// The exact immutable member remains inspectable but cannot be newly selected.
    Retained,
}

/// Authority by which the current session may inspect a safe collection projection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ProblemCollectionAccess {
    /// The active Instructor owns and may mutate this personal collection.
    Owner,
    /// A catalog-authorized same-tenant reader may inspect this institution collection.
    InstitutionReader,
}

/// Strong revision evidence for one complete collection state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct ProblemCollectionRevision(NonZeroU64);

/// Strong revision evidence for one saved-search state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct SavedProblemSearchRevision(NonZeroU64);

macro_rules! impl_revision {
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
                    return Err("revision must be a canonical positive decimal string");
                }
                value
                    .parse::<u64>()
                    .ok()
                    .and_then(Self::new)
                    .ok_or("revision must fit a positive PostgreSQL bigint")
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

impl_revision!(ProblemCollectionRevision);
impl_revision!(SavedProblemSearchRevision);

/// One title validation failure shared by collections and saved searches.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProblemCurationTitleError {
    Invalid,
}

impl std::fmt::Display for ProblemCurationTitleError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("curation title must be trimmed, nonempty, and within its bound")
    }
}
impl std::error::Error for ProblemCurationTitleError {}

/// Validates the title retained for a curation aggregate.
pub fn validate_problem_curation_title(value: &str) -> Result<(), ProblemCurationTitleError> {
    (value == value.trim()
        && !value.is_empty()
        && value.chars().count() <= MAX_PROBLEM_CURATION_TITLE_UNICODE_SCALARS)
        .then_some(())
        .ok_or(ProblemCurationTitleError::Invalid)
}

/// Safe current projection of one exact immutable collection member.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProblemCollectionMemberView {
    pub question_id: QuestionId,
    pub summary: CatalogProblemSummary,
    pub selection_availability: ProblemCollectionSelectionAvailability,
}

/// Browser-safe collection projection. Owner and exact version identities remain server-owned.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProblemCollectionSummaryView {
    pub reference: ProblemCollectionReference,
    pub kind: ProblemCollectionKind,
    pub title: String,
    pub visibility: ProblemCollectionVisibility,
    pub revision: ProblemCollectionRevision,
    pub access: ProblemCollectionAccess,
}

/// Browser-safe personal saved D1 search meaning.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SavedProblemSearchView {
    pub reference: SavedProblemSearchReference,
    pub title: String,
    pub filter: CatalogSearchFilter,
    pub revision: SavedProblemSearchRevision,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn revisions_are_exact_decimal_strings() {
        let revision = ProblemCollectionRevision::new(42).expect("bounded revision");
        assert_eq!(serde_json::to_value(revision).expect("serializes"), "42");
        assert!("042".parse::<SavedProblemSearchRevision>().is_err());
    }

    #[test]
    fn curation_titles_are_trimmed_and_bounded() {
        assert!(validate_problem_curation_title("Exam candidates").is_ok());
        assert!(validate_problem_curation_title(" exam candidates").is_err());
        assert!(validate_problem_curation_title("").is_err());
    }
}
