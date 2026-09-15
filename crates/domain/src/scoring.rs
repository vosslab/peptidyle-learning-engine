//! Assessment Attempt score selection and Assessment Progress projection.
//!
//! `select_assessment_attempt_grade` selects a completed Assessment Attempt for
//! grade selection during recalculation and explicit Instructor
//! actions. `project_assessment_activity` updates the compact Assessment Grade
//! and Assessment Progress records one transition at a time, so storage never scans Assessment Attempt history on a
//! synchronous page request. Both functions are pure.

use std::collections::HashSet;

use question_model::{
    AssessmentAttemptGradeRule, AssessmentAttemptId, AssessmentGrade, AssessmentProgressRecord,
    Timestamp,
};

use crate::assessment_activity::AssessmentActivityError;

const MAX_ABSOLUTE_CURRENT_SCORE: f64 = 1_000.0;

fn validate_current_score(score: f64) -> Result<(), ()> {
    if score.is_finite()
        && (-MAX_ABSOLUTE_CURRENT_SCORE..=MAX_ABSOLUTE_CURRENT_SCORE).contains(&score)
    {
        Ok(())
    } else {
        Err(())
    }
}

/// One completed Assessment Attempt eligible for grade selection.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CompletedAssessmentAttemptScore {
    /// Durable identity of the completed Assessment Attempt.
    pub assessment_attempt: AssessmentAttemptId,
    /// One-based sequence number within the enrollment.
    pub attempt_number: u32,
    /// Current score ratio. Extra and negative credit may put it outside 0..=1.
    pub score: f64,
}

/// Assessment Attempt and score selected by an Assessment Attempt Grade Rule.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AssessmentAttemptGradeSelection {
    /// Selected completed Assessment Attempt.
    pub assessment_attempt: AssessmentAttemptId,
    /// Selected score fraction.
    pub score: f64,
}

/// A rejected completed-Assessment-Attempt set or instructor selection.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ScoreModelError {
    /// A completed Assessment Attempt used the invalid zero sequence number.
    InvalidAssessmentAttemptNumber {
        /// Assessment Attempt carrying the invalid number.
        assessment_attempt: AssessmentAttemptId,
    },
    /// A score was non-finite or outside the bounded current-score range.
    InvalidScore {
        /// Assessment Attempt carrying the invalid score.
        assessment_attempt: AssessmentAttemptId,
        /// Rejected score.
        score: f64,
    },
    /// The completed-Assessment-Attempt set repeated one durable identity.
    DuplicateAssessmentAttempt {
        /// Repeated Assessment Attempt identity.
        assessment_attempt: AssessmentAttemptId,
    },
    /// The completed-Assessment-Attempt set repeated a one-based sequence number.
    DuplicateAssessmentAttemptNumber {
        /// Repeated sequence number.
        attempt_number: u32,
    },
    /// An instructor selected an Assessment Attempt outside the completed set.
    UnknownInstructorSelection {
        /// Unknown selected Assessment Attempt.
        assessment_attempt: AssessmentAttemptId,
    },
    /// An automatic policy received an instructor-only selection.
    UnexpectedInstructorSelection,
}

impl std::fmt::Display for ScoreModelError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidAssessmentAttemptNumber { assessment_attempt } => write!(
                formatter,
                "Assessment Attempt {assessment_attempt} has attempt number zero"
            ),
            Self::InvalidScore {
                assessment_attempt,
                score,
            } => {
                write!(
                    formatter,
                    "Assessment Attempt {assessment_attempt} has invalid score {score}"
                )
            }
            Self::DuplicateAssessmentAttempt { assessment_attempt } => write!(
                formatter,
                "Assessment Attempt {assessment_attempt} appears more than once"
            ),
            Self::DuplicateAssessmentAttemptNumber { attempt_number } => {
                write!(
                    formatter,
                    "attempt number {attempt_number} appears more than once"
                )
            }
            Self::UnknownInstructorSelection { assessment_attempt } => {
                write!(
                    formatter,
                    "instructor-selected Assessment Attempt {assessment_attempt} is not completed"
                )
            }
            Self::UnexpectedInstructorSelection => formatter.write_str(
                "only instructor-selected grading accepts a selected Assessment Attempt",
            ),
        }
    }
}

impl std::error::Error for ScoreModelError {}

/// Selects the completed Assessment Attempt whose score reaches the gradebook.
///
/// First and latest use one-based attempt number rather than slice order. Highest
/// keeps the earlier Assessment Attempt when scores tie, matching incremental
/// projection and keeping a stable grade pointer. `InstructorSelected` returns
/// `None` until an instructor explicitly selects a completed Assessment Attempt.
///
/// # Errors
///
/// Returns [`ScoreModelError`] when an attempt number or score is invalid, an
/// identity or attempt number repeats, an automatic policy receives a selection,
/// or an instructor selects an Assessment Attempt outside the completed set.
pub fn select_assessment_attempt_grade(
    completed_assessment_attempts: &[CompletedAssessmentAttemptScore],
    rule: AssessmentAttemptGradeRule,
    instructor_selected: Option<AssessmentAttemptId>,
) -> Result<Option<AssessmentAttemptGradeSelection>, ScoreModelError> {
    validate_completed_assessment_attempts(completed_assessment_attempts)?;

    if rule != AssessmentAttemptGradeRule::InstructorSelected && instructor_selected.is_some() {
        return Err(ScoreModelError::UnexpectedInstructorSelection);
    }

    let selected = match rule {
        AssessmentAttemptGradeRule::First => completed_assessment_attempts
            .iter()
            .min_by_key(|assessment_attempt| assessment_attempt.attempt_number),
        AssessmentAttemptGradeRule::Latest => completed_assessment_attempts
            .iter()
            .max_by_key(|assessment_attempt| assessment_attempt.attempt_number),
        AssessmentAttemptGradeRule::Highest => {
            highest_assessment_attempt(completed_assessment_attempts)
        }
        AssessmentAttemptGradeRule::InstructorSelected => match instructor_selected {
            Some(selected_assessment_attempt) => Some(
                completed_assessment_attempts
                    .iter()
                    .find(|assessment_attempt| {
                        assessment_attempt.assessment_attempt == selected_assessment_attempt
                    })
                    .ok_or(ScoreModelError::UnknownInstructorSelection {
                        assessment_attempt: selected_assessment_attempt,
                    })?,
            ),
            None => None,
        },
    };

    Ok(
        selected.map(|assessment_attempt| AssessmentAttemptGradeSelection {
            assessment_attempt: assessment_attempt.assessment_attempt,
            score: assessment_attempt.score,
        }),
    )
}

fn validate_completed_assessment_attempts(
    completed_assessment_attempts: &[CompletedAssessmentAttemptScore],
) -> Result<(), ScoreModelError> {
    let mut assessment_attempt_ids = HashSet::with_capacity(completed_assessment_attempts.len());
    let mut attempt_numbers = HashSet::with_capacity(completed_assessment_attempts.len());

    for assessment_attempt in completed_assessment_attempts {
        if assessment_attempt.attempt_number == 0 {
            return Err(ScoreModelError::InvalidAssessmentAttemptNumber {
                assessment_attempt: assessment_attempt.assessment_attempt,
            });
        }
        validate_current_score(assessment_attempt.score).map_err(|()| {
            ScoreModelError::InvalidScore {
                assessment_attempt: assessment_attempt.assessment_attempt,
                score: assessment_attempt.score,
            }
        })?;
        if !assessment_attempt_ids.insert(assessment_attempt.assessment_attempt) {
            return Err(ScoreModelError::DuplicateAssessmentAttempt {
                assessment_attempt: assessment_attempt.assessment_attempt,
            });
        }
        if !attempt_numbers.insert(assessment_attempt.attempt_number) {
            return Err(ScoreModelError::DuplicateAssessmentAttemptNumber {
                attempt_number: assessment_attempt.attempt_number,
            });
        }
    }
    Ok(())
}

fn highest_assessment_attempt(
    completed_assessment_attempts: &[CompletedAssessmentAttemptScore],
) -> Option<&CompletedAssessmentAttemptScore> {
    completed_assessment_attempts
        .iter()
        .reduce(|selected, candidate| {
            if candidate.score > selected.score
                || (candidate.score == selected.score
                    && candidate.attempt_number < selected.attempt_number)
            {
                candidate
            } else {
                selected
            }
        })
}

/// An Assessment Attempt change that affects the compact assessment summary.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AssessmentActivityTransition {
    /// A new Assessment Attempt began at the supplied server timestamp.
    Started {
        /// Authoritative server time for the transition.
        at: Timestamp,
    },
    /// One Student Response was recorded through its Question Submission.
    QuestionAttemptRecorded {
        /// Authoritative server time for the transition.
        at: Timestamp,
    },
    /// Derived completion was recorded with its final score.
    Completed {
        /// Completed Assessment Attempt that owns this score.
        assessment_attempt: AssessmentAttemptId,
        /// Score fraction in the inclusive range `-1000.0..=1000.0`.
        /// Extra and negative credit may place it outside `0.0..=1.0`.
        score: f64,
        /// Authoritative server time for the transition.
        at: Timestamp,
    },
}

/// Projects one Assessment Attempt transition into compact Assessment Grade and Assessment Progress records.
///
/// The input is left unchanged. A store can persist the transition and the
/// returned records in the same transaction. `InstructorSelected` keeps the
/// selected grade unchanged because only an explicit instructor action may
/// select a different Assessment Attempt.
///
/// # Errors
///
/// Returns [`AssessmentActivityError`] for an invalid completed-Assessment-Attempt score or when a
/// summary counter overflows.
pub fn project_assessment_activity(
    previous_grade: &AssessmentGrade,
    previous_progress: &AssessmentProgressRecord,
    transition: AssessmentActivityTransition,
    assessment_attempt_grade_rule: AssessmentAttemptGradeRule,
) -> Result<(AssessmentGrade, AssessmentProgressRecord), AssessmentActivityError> {
    let mut next_grade = previous_grade.clone();
    let mut next_progress = previous_progress.clone();

    match transition {
        AssessmentActivityTransition::Started { at } => touch(&mut next_progress, at),
        AssessmentActivityTransition::QuestionAttemptRecorded { at } => {
            next_progress.total_question_attempts = next_progress
                .total_question_attempts
                .checked_add(1)
                .ok_or(AssessmentActivityError::SummaryCounterOverflow)?;
            touch(&mut next_progress, at);
        }
        AssessmentActivityTransition::Completed {
            assessment_attempt,
            score,
            at,
        } => {
            validate_current_score(score)
                .map_err(|()| AssessmentActivityError::InvalidScore { score })?;
            next_progress.completed_assessment_attempt_count = next_progress
                .completed_assessment_attempt_count
                .checked_add(1)
                .ok_or(AssessmentActivityError::SummaryCounterOverflow)?;
            next_grade.first_completed_at = next_grade.first_completed_at.or(Some(at));
            next_grade.latest_assessment_attempt = Some(assessment_attempt);
            next_grade.latest_score = Some(score);
            if next_grade.best_score.is_none_or(|best| score > best) {
                next_grade.best_assessment_attempt = Some(assessment_attempt);
                next_grade.best_score = Some(score);
            }
            match assessment_attempt_grade_rule {
                AssessmentAttemptGradeRule::First if next_grade.current_score.is_none() => {
                    next_grade.current_assessment_attempt = Some(assessment_attempt);
                    next_grade.current_score = Some(score);
                }
                AssessmentAttemptGradeRule::Latest => {
                    next_grade.current_assessment_attempt = Some(assessment_attempt);
                    next_grade.current_score = Some(score);
                }
                AssessmentAttemptGradeRule::Highest => {
                    next_grade.current_assessment_attempt = next_grade.best_assessment_attempt;
                    next_grade.current_score = next_grade.best_score;
                }
                AssessmentAttemptGradeRule::First
                | AssessmentAttemptGradeRule::InstructorSelected => {}
            }
            touch(&mut next_progress, at);
        }
    }

    Ok((next_grade, next_progress))
}

/// Advances the activity timestamp without moving it backward.
fn touch(summary: &mut AssessmentProgressRecord, at: Timestamp) {
    summary.last_activity_at = Some(
        summary
            .last_activity_at
            .map_or(at, |previous| previous.max(at)),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use question_model::{AssessmentId, StudentRecordId};
    use uuid::Uuid;

    fn assessment_attempt(
        id: u128,
        attempt_number: u32,
        score: f64,
    ) -> CompletedAssessmentAttemptScore {
        CompletedAssessmentAttemptScore {
            assessment_attempt: AssessmentAttemptId::from_uuid(Uuid::from_u128(id)),
            attempt_number,
            score,
        }
    }

    fn empty_summary() -> AssessmentProgressRecord {
        AssessmentProgressRecord::empty(
            StudentRecordId::from_uuid(Uuid::from_u128(11)),
            AssessmentId::from_uuid(Uuid::from_u128(12)),
        )
    }

    fn empty_grade() -> AssessmentGrade {
        AssessmentGrade::empty(
            StudentRecordId::from_uuid(Uuid::from_u128(11)),
            AssessmentId::from_uuid(Uuid::from_u128(12)),
        )
    }

    fn projected_score(
        completed_assessment_attempts: &[CompletedAssessmentAttemptScore],
        rule: AssessmentAttemptGradeRule,
    ) -> Option<f64> {
        let mut grade = empty_grade();
        let mut progress = empty_summary();
        for completed in completed_assessment_attempts {
            (grade, progress) = project_assessment_activity(
                &grade,
                &progress,
                AssessmentActivityTransition::Completed {
                    assessment_attempt: completed.assessment_attempt,
                    score: completed.score,
                    at: Timestamp::from_unix_millis(i64::from(completed.attempt_number)),
                },
                rule,
            )
            .expect("fixture scores should project");
        }
        grade.current_score
    }

    #[test]
    fn hand_computed_fixture_agrees_for_batch_and_incremental_scoring() {
        let completed_assessment_attempts = [
            assessment_attempt(1, 1, 0.4),
            assessment_attempt(2, 2, 0.9),
            assessment_attempt(3, 3, 0.7),
        ];
        let cases = [
            (
                AssessmentAttemptGradeRule::First,
                completed_assessment_attempts[0],
            ),
            (
                AssessmentAttemptGradeRule::Latest,
                completed_assessment_attempts[2],
            ),
            (
                AssessmentAttemptGradeRule::Highest,
                completed_assessment_attempts[1],
            ),
        ];

        for (policy, expected) in cases {
            assert_eq!(
                select_assessment_attempt_grade(&completed_assessment_attempts, policy, None),
                Ok(Some(AssessmentAttemptGradeSelection {
                    assessment_attempt: expected.assessment_attempt,
                    score: expected.score,
                })),
                "{policy:?} batch selection"
            );
            assert_eq!(
                projected_score(&completed_assessment_attempts, policy),
                Some(expected.score),
                "{policy:?} incremental projection"
            );
        }
    }

    #[test]
    fn highest_tie_keeps_the_earlier_assessment_attempt() {
        let completed_assessment_attempts =
            [assessment_attempt(1, 2, 0.9), assessment_attempt(2, 1, 0.9)];

        assert_eq!(
            select_assessment_attempt_grade(
                &completed_assessment_attempts,
                AssessmentAttemptGradeRule::Highest,
                None
            ),
            Ok(Some(AssessmentAttemptGradeSelection {
                assessment_attempt: completed_assessment_attempts[1].assessment_attempt,
                score: 0.9,
            }))
        );
    }

    #[test]
    fn instructor_selection_is_explicit_and_must_name_a_completed_assessment_attempt() {
        let completed_assessment_attempts =
            [assessment_attempt(1, 1, 0.4), assessment_attempt(2, 2, 0.9)];

        assert_eq!(
            select_assessment_attempt_grade(
                &completed_assessment_attempts,
                AssessmentAttemptGradeRule::InstructorSelected,
                None
            ),
            Ok(None)
        );
        assert_eq!(
            select_assessment_attempt_grade(
                &completed_assessment_attempts,
                AssessmentAttemptGradeRule::InstructorSelected,
                Some(completed_assessment_attempts[0].assessment_attempt),
            ),
            Ok(Some(AssessmentAttemptGradeSelection {
                assessment_attempt: completed_assessment_attempts[0].assessment_attempt,
                score: 0.4,
            }))
        );
        let unknown = AssessmentAttemptId::from_uuid(Uuid::from_u128(99));
        assert_eq!(
            select_assessment_attempt_grade(
                &completed_assessment_attempts,
                AssessmentAttemptGradeRule::InstructorSelected,
                Some(unknown),
            ),
            Err(ScoreModelError::UnknownInstructorSelection {
                assessment_attempt: unknown
            })
        );
    }

    #[test]
    fn malformed_completed_assessment_attempt_sets_are_rejected() {
        let duplicate_id = [assessment_attempt(1, 1, 0.4), assessment_attempt(1, 2, 0.9)];
        let duplicate_number = [assessment_attempt(1, 1, 0.4), assessment_attempt(2, 1, 0.9)];
        let invalid_score = [assessment_attempt(1, 1, f64::NAN)];

        assert_eq!(
            select_assessment_attempt_grade(&duplicate_id, AssessmentAttemptGradeRule::First, None),
            Err(ScoreModelError::DuplicateAssessmentAttempt {
                assessment_attempt: duplicate_id[0].assessment_attempt,
            })
        );
        assert_eq!(
            select_assessment_attempt_grade(
                &duplicate_number,
                AssessmentAttemptGradeRule::First,
                None,
            ),
            Err(ScoreModelError::DuplicateAssessmentAttemptNumber { attempt_number: 1 })
        );
        assert!(matches!(
            select_assessment_attempt_grade(&invalid_score, AssessmentAttemptGradeRule::First, None),
            Err(ScoreModelError::InvalidScore { score, .. }) if score.is_nan()
        ));
    }

    #[test]
    fn activity_projection_preserves_instructor_choice_and_monotonic_activity() {
        let mut previous_grade = empty_grade();
        previous_grade.current_assessment_attempt =
            Some(AssessmentAttemptId::from_uuid(Uuid::from_u128(4)));
        previous_grade.current_score = Some(0.5);
        let mut previous_progress = empty_summary();
        previous_progress.last_activity_at = Some(Timestamp::from_unix_millis(10));

        let (next_grade, next_progress) = project_assessment_activity(
            &previous_grade,
            &previous_progress,
            AssessmentActivityTransition::Completed {
                assessment_attempt: AssessmentAttemptId::from_uuid(Uuid::from_u128(5)),
                score: 0.9,
                at: Timestamp::from_unix_millis(9),
            },
            AssessmentAttemptGradeRule::InstructorSelected,
        )
        .expect("valid completion should project");

        assert_eq!(next_grade.current_score, Some(0.5));
        assert_eq!(
            next_progress.last_activity_at,
            Some(Timestamp::from_unix_millis(10))
        );
    }
}
