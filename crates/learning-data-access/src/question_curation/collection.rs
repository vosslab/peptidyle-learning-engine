//! Server-owned named collection aggregate for exact published question pins.
//!
//! This module defines only the value-level ownership, revision, and membership
//! contract. Durable identity allocation, owner-scoped title uniqueness, sharing,
//! and compare-and-swap persistence belong to later Store and schema work.

use std::collections::HashSet;
use std::num::NonZeroU64;

use question_model::{
    AccountId, MAX_PROBLEM_COLLECTION_MEMBERS, ProblemVersionRef, validate_problem_curation_title,
};
use uuid::Uuid;

/// Opaque server-only identity for one named question collection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NamedQuestionCollectionId(Uuid);

impl NamedQuestionCollectionId {
    fn generate() -> Result<Self, NamedQuestionCollectionError> {
        crate::random_uuid::random_uuid_v4(|_| NamedQuestionCollectionError::IdentityUnavailable)
            .map(Self)
    }
}

/// Strong revision evidence for one complete named collection state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NamedQuestionCollectionRevision(NonZeroU64);

impl NamedQuestionCollectionRevision {
    /// Revision assigned to a newly created collection.
    pub const INITIAL: Self = Self(NonZeroU64::MIN);

    /// Creates one storage-safe positive collection revision.
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

/// Why a named collection candidate cannot be created or replaced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NamedQuestionCollectionError {
    /// The collection identity could not be allocated from server randomness.
    IdentityUnavailable,
    /// The retained title violates the shared curation-title policy.
    InvalidTitle,
    /// The ordered member candidate exceeds the shared collection bound.
    MemberLimitExceeded,
    /// The ordered member candidate repeats an exact immutable version pin.
    DuplicateMember(ProblemVersionRef),
    /// The proposed replacement was based on a non-current complete state.
    RevisionConflict {
        expected: NamedQuestionCollectionRevision,
        actual: NamedQuestionCollectionRevision,
    },
    /// Advancing the storage-safe revision would overflow.
    RevisionExhausted,
}

impl std::fmt::Display for NamedQuestionCollectionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::IdentityUnavailable => "named collection identity allocation is unavailable",
            Self::InvalidTitle => "named collection title is invalid",
            Self::MemberLimitExceeded => "named collection member limit exceeded",
            Self::DuplicateMember(_) => "named collection contains a duplicate exact version pin",
            Self::RevisionConflict { .. } => "named collection revision conflict",
            Self::RevisionExhausted => "named collection revision is exhausted",
        })
    }
}

impl std::error::Error for NamedQuestionCollectionError {}

/// Result of replacing a collection's complete candidate state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NamedQuestionCollectionReplacementOutcome {
    /// A different valid state was committed at this revision.
    Replaced(NamedQuestionCollectionRevision),
    /// The valid candidate exactly matched the current state.
    Unchanged(NamedQuestionCollectionRevision),
}

impl NamedQuestionCollectionReplacementOutcome {
    /// Returns the retained current revision after the operation.
    pub fn revision(self) -> NamedQuestionCollectionRevision {
        match self {
            Self::Replaced(revision) | Self::Unchanged(revision) => revision,
        }
    }
}

/// One global account's ordered, exact published-question collection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NamedQuestionCollection {
    id: NamedQuestionCollectionId,
    owner: AccountId,
    title: String,
    revision: NamedQuestionCollectionRevision,
    members: Vec<ProblemVersionRef>,
}

impl NamedQuestionCollection {
    /// Creates a validated collection at its initial revision.
    pub fn new(
        owner: AccountId,
        title: String,
        members: Vec<ProblemVersionRef>,
    ) -> Result<Self, NamedQuestionCollectionError> {
        validate_candidate(&title, &members)?;
        Ok(Self {
            id: NamedQuestionCollectionId::generate()?,
            owner,
            title,
            revision: NamedQuestionCollectionRevision::INITIAL,
            members,
        })
    }

    /// Returns this collection's opaque durable identity.
    pub fn id(&self) -> NamedQuestionCollectionId {
        self.id
    }

    /// Returns the immutable global account that owns this collection.
    pub fn owner(&self) -> AccountId {
        self.owner
    }

    /// Returns the validated retained title.
    pub fn title(&self) -> &str {
        &self.title
    }

    /// Returns the strong revision for the complete current state.
    pub fn revision(&self) -> NamedQuestionCollectionRevision {
        self.revision
    }

    /// Returns the ordered exact immutable version pins.
    pub fn members(&self) -> &[ProblemVersionRef] {
        &self.members
    }

    /// Replaces the complete state when the expected revision is current.
    pub fn replace(
        &mut self,
        expected: NamedQuestionCollectionRevision,
        title: String,
        members: Vec<ProblemVersionRef>,
    ) -> Result<NamedQuestionCollectionReplacementOutcome, NamedQuestionCollectionError> {
        if expected != self.revision {
            return Err(NamedQuestionCollectionError::RevisionConflict {
                expected,
                actual: self.revision,
            });
        }
        validate_candidate(&title, &members)?;
        if self.title == title && self.members == members {
            return Ok(NamedQuestionCollectionReplacementOutcome::Unchanged(
                self.revision,
            ));
        }
        let revision = self
            .revision
            .checked_next()
            .ok_or(NamedQuestionCollectionError::RevisionExhausted)?;
        self.title = title;
        self.members = members;
        self.revision = revision;
        Ok(NamedQuestionCollectionReplacementOutcome::Replaced(
            revision,
        ))
    }
}

fn validate_candidate(
    title: &str,
    members: &[ProblemVersionRef],
) -> Result<(), NamedQuestionCollectionError> {
    validate_problem_curation_title(title)
        .map_err(|_| NamedQuestionCollectionError::InvalidTitle)?;
    if members.len() > MAX_PROBLEM_COLLECTION_MEMBERS {
        return Err(NamedQuestionCollectionError::MemberLimitExceeded);
    }
    let mut seen = HashSet::with_capacity(members.len());
    for member in members {
        if !seen.insert(*member) {
            return Err(NamedQuestionCollectionError::DuplicateMember(*member));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use question_model::{ProblemId, VersionId};
    use uuid::Uuid;

    use super::*;

    fn account(value: u128) -> AccountId {
        AccountId::from_uuid(Uuid::from_u128(value))
    }

    fn reference(problem: u128, version: u128) -> ProblemVersionRef {
        ProblemVersionRef {
            problem: ProblemId::from_uuid(Uuid::from_u128(problem)),
            version: VersionId::from_uuid(Uuid::from_u128(version)),
        }
    }

    #[test]
    fn creation_retains_owner_initial_revision_and_ordered_exact_pins() {
        let owner = account(1);
        let members = vec![reference(2, 3), reference(4, 5)];
        let collection = NamedQuestionCollection::new(owner, "Exam review".to_string(), members)
            .expect("valid collection");

        assert_eq!(collection.owner(), owner);
        assert_eq!(
            collection.revision(),
            NamedQuestionCollectionRevision::INITIAL
        );
        assert_eq!(collection.members(), &[reference(2, 3), reference(4, 5)]);
    }

    #[test]
    fn creation_rejects_invalid_titles() {
        for title in [" Exam review", "Exam review ", ""] {
            let result = NamedQuestionCollection::new(account(1), title.to_string(), vec![]);

            assert_eq!(result, Err(NamedQuestionCollectionError::InvalidTitle));
        }
        let over_bound = "x".repeat(201);
        assert_eq!(
            NamedQuestionCollection::new(account(1), over_bound, vec![]),
            Err(NamedQuestionCollectionError::InvalidTitle)
        );
    }

    #[test]
    fn replacement_rejects_over_bound_and_separated_duplicate_exact_pins() {
        let mut collection =
            NamedQuestionCollection::new(account(1), "Exam review".to_string(), vec![])
                .expect("valid collection");
        let over_bound = vec![reference(1, 1); MAX_PROBLEM_COLLECTION_MEMBERS + 1];
        assert_eq!(
            collection.replace(collection.revision(), "Exam review".to_string(), over_bound),
            Err(NamedQuestionCollectionError::MemberLimitExceeded)
        );
        let duplicate = reference(2, 3);
        assert_eq!(
            collection.replace(
                collection.revision(),
                "Exam review".to_string(),
                vec![duplicate, reference(4, 5), duplicate],
            ),
            Err(NamedQuestionCollectionError::DuplicateMember(duplicate))
        );
    }

    #[test]
    fn replacement_accepts_exactly_the_bounded_number_of_unique_pins() {
        let mut collection =
            NamedQuestionCollection::new(account(1), "Exam review".to_string(), vec![])
                .expect("valid collection");
        let members = (1..=MAX_PROBLEM_COLLECTION_MEMBERS)
            .map(|value| reference(value as u128, (value + 1) as u128))
            .collect();

        assert_eq!(
            collection.replace(collection.revision(), "Exam review".to_string(), members),
            Ok(NamedQuestionCollectionReplacementOutcome::Replaced(
                NamedQuestionCollectionRevision::new(2).expect("next revision")
            ))
        );
    }

    #[test]
    fn stale_replacement_reports_both_revisions_and_retains_state() {
        let mut collection = NamedQuestionCollection::new(
            account(1),
            "Exam review".to_string(),
            vec![reference(2, 3)],
        )
        .expect("valid collection");
        let initial = collection.revision();
        collection
            .replace(initial, "Final review".to_string(), vec![reference(4, 5)])
            .expect("current replacement");

        assert_eq!(
            collection.replace(initial, "Stale title".to_string(), vec![]),
            Err(NamedQuestionCollectionError::RevisionConflict {
                expected: initial,
                actual: collection.revision(),
            })
        );
        assert_eq!(collection.title(), "Final review");
        assert_eq!(collection.members(), &[reference(4, 5)]);
    }

    #[test]
    fn valid_replacement_advances_revision_and_replaces_whole_state() {
        let mut collection = NamedQuestionCollection::new(
            account(1),
            "Exam review".to_string(),
            vec![reference(2, 3)],
        )
        .expect("valid collection");
        let outcome = collection
            .replace(
                collection.revision(),
                "Final review".to_string(),
                vec![reference(4, 5), reference(6, 7)],
            )
            .expect("valid replacement");

        assert_eq!(
            outcome,
            NamedQuestionCollectionReplacementOutcome::Replaced(
                NamedQuestionCollectionRevision::new(2).expect("next revision")
            )
        );
        assert_eq!(collection.title(), "Final review");
        assert_eq!(collection.members(), &[reference(4, 5), reference(6, 7)]);
    }

    #[test]
    fn equal_valid_replacement_retains_its_revision() {
        let mut collection = NamedQuestionCollection::new(
            account(1),
            "Exam review".to_string(),
            vec![reference(2, 3)],
        )
        .expect("valid collection");
        let revision = collection.revision();

        assert_eq!(
            collection.replace(revision, "Exam review".to_string(), vec![reference(2, 3)]),
            Ok(NamedQuestionCollectionReplacementOutcome::Unchanged(
                revision
            ))
        );
        assert_eq!(collection.revision(), revision);
    }

    #[test]
    fn revision_exhaustion_refuses_an_otherwise_valid_replacement() {
        let mut collection =
            NamedQuestionCollection::new(account(1), "Exam review".to_string(), vec![])
                .expect("valid collection");
        collection.revision = NamedQuestionCollectionRevision::new(i64::MAX as u64)
            .expect("maximum storage-safe revision");

        assert_eq!(
            collection.replace(
                collection.revision(),
                "Final review".to_string(),
                vec![reference(2, 3)],
            ),
            Err(NamedQuestionCollectionError::RevisionExhausted)
        );
        assert_eq!(collection.title(), "Exam review");
    }
}
