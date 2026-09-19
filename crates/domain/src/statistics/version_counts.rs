//! Exact issued, blank, answered, and credit-sum counts for one Question Revision.
//!
//! This aggregate is deliberately separate from the cohort-based difficulty,
//! timing, and discrimination rollup. It records one Issued Question at
//! submission time and carries no Account, Course, Student Record, response,
//! or receipt identity, so its persisted snapshot can survive record deletion.

use serde::{Deserialize, Serialize};

use super::StatisticsError;

/// Credit scaled by 10^8 so 1.0 is `100_000_000`.
const CREDIT_SCALE: u64 = 100_000_000;

/// One Issued Question reduced to global-count evidence at submission.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QuestionStatisticsObservation {
    /// No saved response existed at submit.
    Blank,
    /// A graded saved response with normalized credit in `[0, 1]`.
    Answered { credit_e8: u64 },
}

impl QuestionStatisticsObservation {
    /// Builds a blank observation.
    pub const fn blank() -> Self {
        Self::Blank
    }

    /// Builds a graded observation. `credit_e8` is credit times 10^8.
    pub fn answered(credit_e8: u64) -> Result<Self, StatisticsError> {
        if credit_e8 > CREDIT_SCALE {
            return Err(StatisticsError::InvalidCredit);
        }
        Ok(Self::Answered { credit_e8 })
    }

    /// Full-credit graded observation.
    pub const fn correct() -> Self {
        Self::Answered {
            credit_e8: CREDIT_SCALE,
        }
    }

    /// Zero-credit graded observation.
    pub const fn incorrect() -> Self {
        Self::Answered { credit_e8: 0 }
    }
}

/// Exact global counts for one immutable Question Revision.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct QuestionRevisionStatistics {
    issued_count: u64,
    blank_count: u64,
    answered_count: u64,
    correct_count: u64,
    partial_count: u64,
    incorrect_count: u64,
    credit_sum_e8: u128,
    credit_sum_sq_e8: u128,
}

impl QuestionRevisionStatistics {
    /// Creates an aggregate with no observations.
    pub const fn empty() -> Self {
        Self {
            issued_count: 0,
            blank_count: 0,
            answered_count: 0,
            correct_count: 0,
            partial_count: 0,
            incorrect_count: 0,
            credit_sum_e8: 0,
            credit_sum_sq_e8: 0,
        }
    }

    /// Records one Issued Question exactly once at its caller's receipt boundary.
    pub fn record(
        &mut self,
        observation: QuestionStatisticsObservation,
    ) -> Result<(), StatisticsError> {
        let issued_count = self
            .issued_count
            .checked_add(1)
            .ok_or(StatisticsError::CounterOverflow)?;
        let mut next = self.clone();
        next.issued_count = issued_count;
        match observation {
            QuestionStatisticsObservation::Blank => {
                next.blank_count = self
                    .blank_count
                    .checked_add(1)
                    .ok_or(StatisticsError::CounterOverflow)?;
            }
            QuestionStatisticsObservation::Answered { credit_e8 } => {
                next.answered_count = self
                    .answered_count
                    .checked_add(1)
                    .ok_or(StatisticsError::CounterOverflow)?;
                if credit_e8 == CREDIT_SCALE {
                    next.correct_count = self
                        .correct_count
                        .checked_add(1)
                        .ok_or(StatisticsError::CounterOverflow)?;
                } else if credit_e8 == 0 {
                    next.incorrect_count = self
                        .incorrect_count
                        .checked_add(1)
                        .ok_or(StatisticsError::CounterOverflow)?;
                } else {
                    next.partial_count = self
                        .partial_count
                        .checked_add(1)
                        .ok_or(StatisticsError::CounterOverflow)?;
                }
                next.credit_sum_e8 = self
                    .credit_sum_e8
                    .checked_add(u128::from(credit_e8))
                    .ok_or(StatisticsError::CounterOverflow)?;
                let square = u128::from(credit_e8)
                    .checked_mul(u128::from(credit_e8))
                    .ok_or(StatisticsError::CounterOverflow)?
                    / u128::from(CREDIT_SCALE);
                next.credit_sum_sq_e8 = self
                    .credit_sum_sq_e8
                    .checked_add(square)
                    .ok_or(StatisticsError::CounterOverflow)?;
            }
        }
        *self = next;
        Ok(())
    }

    /// Issued Question observations.
    pub const fn issued_count(&self) -> u64 {
        self.issued_count
    }

    /// Blank Issued Questions.
    pub const fn blank_count(&self) -> u64 {
        self.blank_count
    }

    /// Answered Issued Questions.
    pub const fn answered_count(&self) -> u64 {
        self.answered_count
    }

    /// Full-credit answered observations.
    pub const fn correct_count(&self) -> u64 {
        self.correct_count
    }

    /// Partial-credit answered observations.
    pub const fn partial_count(&self) -> u64 {
        self.partial_count
    }

    /// Zero-credit answered observations.
    pub const fn incorrect_count(&self) -> u64 {
        self.incorrect_count
    }

    /// Sum of normalized credit, scaled by 10^8.
    pub const fn credit_sum_e8(&self) -> u128 {
        self.credit_sum_e8
    }

    /// Sum of squared normalized credit, scaled by 10^8.
    pub const fn credit_sum_sq_e8(&self) -> u128 {
        self.credit_sum_sq_e8
    }
}

impl Default for QuestionRevisionStatistics {
    fn default() -> Self {
        Self::empty()
    }
}

/// Pool-level issued count.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct QuestionPoolStatistics {
    issued_count: u64,
}

impl QuestionPoolStatistics {
    /// Empty pool issued count.
    pub const fn empty() -> Self {
        Self { issued_count: 0 }
    }

    /// Increments issued_count by one.
    pub fn record_issue(&mut self) -> Result<(), StatisticsError> {
        self.issued_count = self
            .issued_count
            .checked_add(1)
            .ok_or(StatisticsError::CounterOverflow)?;
        Ok(())
    }

    /// Issued count for this Pool.
    pub const fn issued_count(&self) -> u64 {
        self.issued_count
    }
}

/// Selected count for one Pool member Published Question.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct QuestionPoolMemberStatistics {
    selected_count: u64,
}

impl QuestionPoolMemberStatistics {
    /// Empty member selected count.
    pub const fn empty() -> Self {
        Self { selected_count: 0 }
    }

    /// Increments selected_count by one.
    pub fn record_selection(&mut self) -> Result<(), StatisticsError> {
        self.selected_count = self
            .selected_count
            .checked_add(1)
            .ok_or(StatisticsError::CounterOverflow)?;
        Ok(())
    }

    /// Selected count for this member.
    pub const fn selected_count(&self) -> u64 {
        self.selected_count
    }
}
