//! Exact accepted-grade counts for one immutable Question Revision.
//!
//! This aggregate is deliberately separate from the cohort-based difficulty,
//! timing, and discrimination rollup. It records one accepted graded Question
//! Attempt at a time and carries no Account, Course, Student Record, response,
//! or receipt identity, so its persisted snapshot can survive record deletion.

use std::collections::{BTreeMap, BTreeSet};

use question_model::response::ResponseItemReference;

use super::StatisticsError;

/// One accepted graded Question Attempt reduced to global-count evidence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuestionStatisticsObservation {
    correct: bool,
    eligible_choice_selections: BTreeSet<ResponseItemReference>,
}

impl QuestionStatisticsObservation {
    /// Builds one correctness result and its distinct selected eligible choices.
    ///
    /// The grading boundary supplies only choices that were eligible for this
    /// exact Question Revision. Deduplication here makes the aggregate robust
    /// against a malformed repeated selection at a storage boundary.
    pub fn new(
        correct: bool,
        eligible_choice_selections: impl IntoIterator<Item = ResponseItemReference>,
    ) -> Result<Self, StatisticsError> {
        let selections = eligible_choice_selections
            .into_iter()
            .collect::<BTreeSet<_>>();
        if selections.iter().any(|choice| choice.as_str().is_empty()) {
            return Err(StatisticsError::InvalidChoiceIdentifier);
        }
        Ok(Self {
            correct,
            eligible_choice_selections: selections,
        })
    }

    /// Returns whether the accepted graded Question Attempt was correct.
    pub const fn correct(&self) -> bool {
        self.correct
    }

    /// Returns the selected eligible choices, sorted by their opaque ID.
    pub fn eligible_choice_selections(&self) -> impl Iterator<Item = &ResponseItemReference> {
        self.eligible_choice_selections.iter()
    }
}

/// Retention-safe persisted state for exact Question Revision counts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuestionRevisionStatisticsSnapshot {
    /// Number of accepted graded Question Attempts for this exact version.
    pub accepted_graded_attempt_count: u64,
    /// Number of those accepted grades whose result was correct.
    pub correct_count: u64,
    /// Selection count by opaque eligible choice ID for supported choice formats.
    pub eligible_choice_selection_counts: BTreeMap<ResponseItemReference, u64>,
}

/// Exact global counts for one immutable Question Revision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuestionRevisionStatistics {
    accepted_graded_attempt_count: u64,
    correct_count: u64,
    eligible_choice_selection_counts: BTreeMap<ResponseItemReference, u64>,
}

impl QuestionRevisionStatistics {
    /// Creates an aggregate with no accepted grades.
    pub const fn empty() -> Self {
        Self {
            accepted_graded_attempt_count: 0,
            correct_count: 0,
            eligible_choice_selection_counts: BTreeMap::new(),
        }
    }

    /// Records one accepted graded Question Attempt exactly once at its caller's receipt boundary.
    pub fn record(
        &mut self,
        observation: QuestionStatisticsObservation,
    ) -> Result<(), StatisticsError> {
        let accepted_graded_attempt_count = self
            .accepted_graded_attempt_count
            .checked_add(1)
            .ok_or(StatisticsError::CounterOverflow)?;
        let correct_count = self
            .correct_count
            .checked_add(u64::from(observation.correct()))
            .ok_or(StatisticsError::CounterOverflow)?;
        let mut eligible_choice_selection_counts = self.eligible_choice_selection_counts.clone();
        for choice in observation.eligible_choice_selections() {
            let count = eligible_choice_selection_counts
                .entry(choice.clone())
                .or_default();
            *count = count
                .checked_add(1)
                .ok_or(StatisticsError::CounterOverflow)?;
        }
        *self = Self {
            accepted_graded_attempt_count,
            correct_count,
            eligible_choice_selection_counts,
        };
        Ok(())
    }

    /// Captures the retention-safe persistence representation.
    pub fn snapshot(&self) -> QuestionRevisionStatisticsSnapshot {
        QuestionRevisionStatisticsSnapshot {
            accepted_graded_attempt_count: self.accepted_graded_attempt_count,
            correct_count: self.correct_count,
            eligible_choice_selection_counts: self.eligible_choice_selection_counts.clone(),
        }
    }

    /// Restores only a count snapshot whose invariants hold for individual accepted grades.
    pub fn restore(snapshot: QuestionRevisionStatisticsSnapshot) -> Result<Self, StatisticsError> {
        if snapshot.correct_count > snapshot.accepted_graded_attempt_count
            || snapshot
                .eligible_choice_selection_counts
                .iter()
                .any(|(choice, count)| {
                    choice.as_str().is_empty() || *count > snapshot.accepted_graded_attempt_count
                })
        {
            return Err(StatisticsError::SnapshotInvariant);
        }
        Ok(Self {
            accepted_graded_attempt_count: snapshot.accepted_graded_attempt_count,
            correct_count: snapshot.correct_count,
            eligible_choice_selection_counts: snapshot.eligible_choice_selection_counts,
        })
    }

    /// Returns the number of accepted graded Question Attempts.
    pub const fn accepted_graded_attempt_count(&self) -> u64 {
        self.accepted_graded_attempt_count
    }

    /// Returns the number of correct accepted grades.
    pub const fn correct_count(&self) -> u64 {
        self.correct_count
    }

    /// Returns choice-selection counts by opaque eligible choice ID.
    pub fn eligible_choice_selection_counts(&self) -> &BTreeMap<ResponseItemReference, u64> {
        &self.eligible_choice_selection_counts
    }
}

impl Default for QuestionRevisionStatistics {
    fn default() -> Self {
        Self::empty()
    }
}
