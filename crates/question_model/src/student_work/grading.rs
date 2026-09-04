//! Server-only Question Backend evaluation and Assignment-owned scoring records.

use serde::{Deserialize, Serialize};

use super::IssuedQuestion;
use crate::assignment::AssignmentEntryScoringRule;

/// Server-only Question Backend evaluation before Assignment scoring.
///
/// This deliberately has no Serde implementation: normalized credit is a
/// trusted backend-to-assignment fact, not a browser contract.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct QuestionEvaluation {
    correct: bool,
    normalized_credit: f64,
}

impl QuestionEvaluation {
    /// Creates one finite normalized backend evaluation.
    pub fn new(correct: bool, normalized_credit: f64) -> Result<Self, QuestionEvaluationError> {
        if !normalized_credit.is_finite() || !(0.0..=1.0).contains(&normalized_credit) {
            return Err(QuestionEvaluationError::InvalidNormalizedCredit);
        }
        Ok(Self {
            correct,
            normalized_credit,
        })
    }

    /// Whether the Question Backend determined the response was fully correct.
    pub const fn correct(self) -> bool {
        self.correct
    }

    /// Credit normalized to the inclusive unit interval.
    pub const fn normalized_credit(self) -> f64 {
        self.normalized_credit
    }
}

/// Rejection reason for one server-only Question Backend evaluation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuestionEvaluationError {
    /// Normalized credit was non-finite, negative, or greater than one.
    InvalidNormalizedCredit,
}

impl std::fmt::Display for QuestionEvaluationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("normalized credit must be finite and within 0 through 1")
    }
}

impl std::error::Error for QuestionEvaluationError {}

/// A grading result without an answer key.
///
/// The server may disclose this according to the assignment feedback policy;
/// the correct response and Question Grader code remain in `grading`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GradingResult {
    /// Whether the submitted response was correct.
    pub correct: bool,
    /// Points awarded by server-side grading.
    pub points_earned: f64,
    /// Maximum points available for this question.
    pub points_possible: f64,
}

impl GradingResult {
    /// Applies the exact issued Assignment Entry scoring policy to one backend evaluation.
    pub fn from_issued_question_evaluation(
        issued_question: &IssuedQuestion,
        evaluation: QuestionEvaluation,
    ) -> Self {
        let point_value = issued_question.point_value.scaled() as f64 / 10_000.0;
        let (points_earned, points_possible) = match issued_question.scoring_rule {
            AssignmentEntryScoringRule::Normal => {
                (point_value * evaluation.normalized_credit(), point_value)
            }
            AssignmentEntryScoringRule::FullCredit => (point_value, point_value),
            AssignmentEntryScoringRule::ExtraCredit => {
                (point_value * evaluation.normalized_credit(), point_value)
            }
            AssignmentEntryScoringRule::Excluded => (0.0, point_value),
        };
        Self {
            correct: evaluation.correct(),
            points_earned,
            points_possible,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        AssignmentAttemptId, AssignmentEntryId, AssignmentPointValue, IssuedQuestionId, QuestionId,
        QuestionRevisionNumber, QuestionRevisionReference,
    };
    use uuid::Uuid;

    fn issued_question(scoring_rule: AssignmentEntryScoringRule) -> IssuedQuestion {
        IssuedQuestion {
            id: IssuedQuestionId::from_uuid(Uuid::from_u128(1)),
            assignment_attempt: AssignmentAttemptId::from_uuid(Uuid::from_u128(2)),
            assignment_entry: AssignmentEntryId::from_uuid(Uuid::from_u128(3)),
            assignment_content_entry_index: 0,
            issued_position: 0,
            reference: QuestionRevisionReference {
                question_id: QuestionId::from_canonical_parts("ABCDEF", 'G').expect("question ID"),
                revision_number: QuestionRevisionNumber::new(1).expect("revision"),
            },
            point_value: AssignmentPointValue::from_whole(8),
            scoring_rule,
            question_statistics_eligibility: true,
            question_pool_selection: None,
            question_pool_item: None,
        }
    }

    #[test]
    fn issued_question_scoring_owns_every_assignment_treatment() {
        let evaluation = QuestionEvaluation::new(false, 0.5).expect("normalized credit");
        let cases = [
            (AssignmentEntryScoringRule::Normal, 4.0, 8.0),
            (AssignmentEntryScoringRule::FullCredit, 8.0, 8.0),
            (AssignmentEntryScoringRule::ExtraCredit, 4.0, 8.0),
            (AssignmentEntryScoringRule::Excluded, 0.0, 8.0),
        ];
        for (rule, points_earned, points_possible) in cases {
            assert_eq!(
                GradingResult::from_issued_question_evaluation(&issued_question(rule), evaluation),
                GradingResult {
                    correct: false,
                    points_earned,
                    points_possible,
                }
            );
        }
    }

    #[test]
    fn evaluation_accepts_signed_zero_credit() {
        assert_eq!(
            QuestionEvaluation::new(false, -0.0)
                .expect("signed zero is normalized")
                .normalized_credit(),
            -0.0
        );
    }
}
