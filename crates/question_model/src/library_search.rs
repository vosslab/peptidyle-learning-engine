//! Mixed Library discovery contracts. Exact Question projections remain Question-only.

use serde::{Deserialize, Serialize};

use crate::{AccountId, LibraryObjectKind, QuestionPoolLibrarySummary, QuestionSearchResult};

/// Result kinds admitted to one globally sorted Library query.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LibrarySearchKind {
    Both,
    #[default]
    Questions,
    Pools,
}

/// Membership restriction for Question rows; Pool rows are unaffected.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LibraryQuestionMembership {
    NoPool,
    #[default]
    All,
}

/// Authorized routing hint for the shared Library detail URL.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LibraryObjectKindResponse {
    pub kind: LibraryObjectKind,
}

/// Counts after domain filters, before kind, membership, and cursor restrictions.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LibrarySearchCategoryCounts {
    pub questions_in_no_pool: u64,
    pub questions_in_pool: u64,
    pub pools: u64,
}

/// One Library row. Question source/evidence is never required for a Pool row.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum LibrarySearchResult {
    Question {
        question: Box<QuestionSearchResult>,
        owner_account_id: AccountId,
    },
    Pool {
        pool: Box<QuestionPoolLibrarySummary>,
    },
}
