//! Server-owned named saved-search aggregate for canonical catalog meaning.
//!
//! This module owns only value-level identity, ownership, and replacement
//! semantics. Store-backed reference allocation, authorization, persistence,
//! and browser projections belong to later work.

use std::num::NonZeroU64;

use question_model::{CatalogSearchFilter, UserId, validate_problem_curation_title};
use uuid::Uuid;

/// Opaque server-only identity for one named question saved search.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NamedQuestionSavedSearchId(Uuid);

impl NamedQuestionSavedSearchId {
    fn generate() -> Result<Self, NamedQuestionSavedSearchError> {
        crate::random_uuid::random_uuid_v4(|_| NamedQuestionSavedSearchError::IdentityUnavailable)
            .map(Self)
    }
}

/// Strong revision evidence for one complete named saved-search state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NamedQuestionSavedSearchRevision(NonZeroU64);

impl NamedQuestionSavedSearchRevision {
    /// Revision assigned to a newly created saved search.
    pub const INITIAL: Self = Self(NonZeroU64::MIN);

    /// Creates one storage-safe positive saved-search revision.
    pub fn new(value: u64) -> Option<Self> {
        (value <= i64::MAX as u64)
            .then(|| NonZeroU64::new(value))
            .flatten()
            .map(Self)
    }

    /// Returns the positive storage value represented by this revision.
    pub fn value(self) -> u64 {
        self.0.get()
    }

    fn checked_next(self) -> Option<Self> {
        self.value().checked_add(1).and_then(Self::new)
    }
}

/// Why a named saved-search candidate cannot be created or replaced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NamedQuestionSavedSearchError {
    /// The saved-search identity could not be allocated from server randomness.
    IdentityUnavailable,
    /// The retained title violates the shared curation-title policy.
    InvalidTitle,
    /// The candidate filter violates the canonical catalog-search contract.
    InvalidFilter,
    /// The proposed replacement was based on a non-current complete state.
    RevisionConflict {
        expected: NamedQuestionSavedSearchRevision,
        actual: NamedQuestionSavedSearchRevision,
    },
    /// Advancing the storage-safe revision would overflow.
    RevisionExhausted,
}

impl std::fmt::Display for NamedQuestionSavedSearchError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::IdentityUnavailable => "named saved-search identity allocation is unavailable",
            Self::InvalidTitle => "named saved-search title is invalid",
            Self::InvalidFilter => "named saved-search filter is invalid",
            Self::RevisionConflict { .. } => "named saved-search revision conflict",
            Self::RevisionExhausted => "named saved-search revision is exhausted",
        })
    }
}

impl std::error::Error for NamedQuestionSavedSearchError {}

/// Result of replacing a saved search's complete candidate state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NamedQuestionSavedSearchReplacementOutcome {
    /// A different valid state was committed at this revision.
    Replaced(NamedQuestionSavedSearchRevision),
    /// The valid candidate exactly matched the current state.
    Unchanged(NamedQuestionSavedSearchRevision),
}

impl NamedQuestionSavedSearchReplacementOutcome {
    /// Returns the retained current revision after the operation.
    pub fn revision(self) -> NamedQuestionSavedSearchRevision {
        match self {
            Self::Replaced(revision) | Self::Unchanged(revision) => revision,
        }
    }
}

/// One global account's personal canonical catalog-search meaning.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NamedQuestionSavedSearch {
    id: NamedQuestionSavedSearchId,
    owner: UserId,
    title: String,
    filter: CatalogSearchFilter,
    revision: NamedQuestionSavedSearchRevision,
}

impl NamedQuestionSavedSearch {
    /// Creates a validated saved search at its initial revision.
    pub fn new(
        owner: UserId,
        title: String,
        filter: CatalogSearchFilter,
    ) -> Result<Self, NamedQuestionSavedSearchError> {
        let filter = validate_candidate(&title, filter)?;
        Ok(Self {
            id: NamedQuestionSavedSearchId::generate()?,
            owner,
            title,
            filter,
            revision: NamedQuestionSavedSearchRevision::INITIAL,
        })
    }

    /// Returns this saved search's opaque durable identity.
    pub fn id(&self) -> NamedQuestionSavedSearchId {
        self.id
    }

    /// Returns the immutable global account that owns this saved search.
    pub fn owner(&self) -> UserId {
        self.owner
    }

    /// Returns the validated retained title.
    pub fn title(&self) -> &str {
        &self.title
    }

    /// Returns the normalized canonical search meaning.
    pub fn filter(&self) -> &CatalogSearchFilter {
        &self.filter
    }

    /// Returns the strong revision for the complete current state.
    pub fn revision(&self) -> NamedQuestionSavedSearchRevision {
        self.revision
    }

    /// Replaces the complete state when the expected revision is current.
    pub fn replace(
        &mut self,
        expected: NamedQuestionSavedSearchRevision,
        title: String,
        filter: CatalogSearchFilter,
    ) -> Result<NamedQuestionSavedSearchReplacementOutcome, NamedQuestionSavedSearchError> {
        if expected != self.revision {
            return Err(NamedQuestionSavedSearchError::RevisionConflict {
                expected,
                actual: self.revision,
            });
        }
        let filter = validate_candidate(&title, filter)?;
        if self.title == title && self.filter == filter {
            return Ok(NamedQuestionSavedSearchReplacementOutcome::Unchanged(
                self.revision,
            ));
        }
        let revision = self
            .revision
            .checked_next()
            .ok_or(NamedQuestionSavedSearchError::RevisionExhausted)?;
        self.title = title;
        self.filter = filter;
        self.revision = revision;
        Ok(NamedQuestionSavedSearchReplacementOutcome::Replaced(
            revision,
        ))
    }
}

fn validate_candidate(
    title: &str,
    filter: CatalogSearchFilter,
) -> Result<CatalogSearchFilter, NamedQuestionSavedSearchError> {
    validate_problem_curation_title(title)
        .map_err(|_| NamedQuestionSavedSearchError::InvalidTitle)?;
    filter
        .normalized()
        .map_err(|_| NamedQuestionSavedSearchError::InvalidFilter)
}

#[cfg(test)]
mod tests {
    use question_model::CatalogSearchFilter;
    use uuid::Uuid;

    use super::*;

    fn user(value: u128) -> UserId {
        UserId::from_uuid(Uuid::from_u128(value))
    }

    fn filter(text: &str) -> CatalogSearchFilter {
        CatalogSearchFilter {
            text: Some(text.to_string()),
            bylines: vec![],
            backends: vec![],
            tags: vec![],
            response_families: vec![],
            taxonomy: vec![],
            capabilities: vec![],
            licenses: vec![],
            evidence: Default::default(),
            used_in_my_courses: Default::default(),
            authorship: Default::default(),
        }
    }

    #[test]
    fn creation_retains_global_owner_and_opaque_server_identity() {
        let owner = user(1);
        let saved_search =
            NamedQuestionSavedSearch::new(owner, "Exam review".to_string(), filter("protein"))
                .expect("valid saved search");

        let _: NamedQuestionSavedSearchId = saved_search.id();
        assert_eq!(saved_search.owner(), owner);
    }

    #[test]
    fn creation_rejects_invalid_titles() {
        for title in [" Exam review", "Exam review ", ""] {
            assert_eq!(
                NamedQuestionSavedSearch::new(user(1), title.to_string(), filter("protein")),
                Err(NamedQuestionSavedSearchError::InvalidTitle)
            );
        }
        assert_eq!(
            NamedQuestionSavedSearch::new(user(1), "x".repeat(201), filter("protein")),
            Err(NamedQuestionSavedSearchError::InvalidTitle)
        );
    }

    #[test]
    fn creation_rejects_invalid_filters() {
        let mut invalid = filter("protein");
        invalid.tags = vec![" ".to_string()];

        assert_eq!(
            NamedQuestionSavedSearch::new(user(1), "Exam review".to_string(), invalid),
            Err(NamedQuestionSavedSearchError::InvalidFilter)
        );
    }

    #[test]
    fn creation_starts_at_initial_revision_and_retains_canonical_fresh_filter() {
        let saved_search = NamedQuestionSavedSearch::new(
            user(1),
            "Exam review".to_string(),
            filter("  Protein   FOLDING "),
        )
        .expect("valid saved search");

        assert_eq!(
            saved_search.revision(),
            NamedQuestionSavedSearchRevision::INITIAL
        );
        assert_eq!(
            saved_search.filter().text.as_deref(),
            Some("protein folding")
        );
        assert_eq!(saved_search.filter().fresh_query().cursor, None);
        assert_eq!(saved_search.filter().fresh_query().page_size, None);
    }

    #[test]
    fn normalization_equivalent_replacement_is_unchanged() {
        let mut saved_search = NamedQuestionSavedSearch::new(
            user(1),
            "Exam review".to_string(),
            filter("protein folding"),
        )
        .expect("valid saved search");
        let revision = saved_search.revision();

        assert_eq!(
            saved_search.replace(
                revision,
                "Exam review".to_string(),
                filter(" Protein   FOLDING ")
            ),
            Ok(NamedQuestionSavedSearchReplacementOutcome::Unchanged(
                revision
            ))
        );
        assert_eq!(saved_search.revision(), revision);
    }

    #[test]
    fn changed_title_and_filter_replace_whole_state_once() {
        let mut saved_search =
            NamedQuestionSavedSearch::new(user(1), "Exam review".to_string(), filter("protein"))
                .expect("valid saved search");

        assert_eq!(
            saved_search.replace(
                saved_search.revision(),
                "Final review".to_string(),
                filter("genetics"),
            ),
            Ok(NamedQuestionSavedSearchReplacementOutcome::Replaced(
                NamedQuestionSavedSearchRevision::new(2).expect("next revision")
            ))
        );
        assert_eq!(saved_search.title(), "Final review");
        assert_eq!(saved_search.filter().text.as_deref(), Some("genetics"));
    }

    #[test]
    fn stale_replacement_reports_both_revisions_without_mutation() {
        let mut saved_search =
            NamedQuestionSavedSearch::new(user(1), "Exam review".to_string(), filter("protein"))
                .expect("valid saved search");
        let initial = saved_search.revision();
        saved_search
            .replace(initial, "Final review".to_string(), filter("genetics"))
            .expect("current replacement");

        assert_eq!(
            saved_search.replace(initial, "Stale title".to_string(), filter("evolution")),
            Err(NamedQuestionSavedSearchError::RevisionConflict {
                expected: initial,
                actual: saved_search.revision(),
            })
        );
        assert_eq!(saved_search.title(), "Final review");
        assert_eq!(saved_search.filter().text.as_deref(), Some("genetics"));
    }

    #[test]
    fn revision_exhaustion_refuses_an_otherwise_valid_replacement() {
        let mut saved_search =
            NamedQuestionSavedSearch::new(user(1), "Exam review".to_string(), filter("protein"))
                .expect("valid saved search");
        saved_search.revision = NamedQuestionSavedSearchRevision::new(i64::MAX as u64)
            .expect("maximum storage-safe revision");

        assert_eq!(
            saved_search.replace(
                saved_search.revision(),
                "Final review".to_string(),
                filter("genetics"),
            ),
            Err(NamedQuestionSavedSearchError::RevisionExhausted)
        );
        assert_eq!(saved_search.title(), "Exam review");
        assert_eq!(saved_search.filter().text.as_deref(), Some("protein"));
    }
}
