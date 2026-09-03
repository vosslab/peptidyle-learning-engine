//! Assignment Attempt score selection and Assignment Progress projection.
//!
//! `select_assignment_attempt_grade` selects a completed Assignment Attempt for
//! grade selection during recalculation and explicit Instructor
//! actions. `project_assignment_activity` updates the compact Assignment Grade
//! and Assignment Progress records one transition at a time, so storage never scans Assignment Attempt history on a
//! synchronous page request. Both functions are pure.

use std::collections::HashSet;

use question_model::{
    AssignmentAttemptGradeRule, AssignmentAttemptId, AssignmentGrade, AssignmentProgressRecord,
    Timestamp,
};

use crate::assignment_activity::AssignmentActivityError;

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

/// One completed Assignment Attempt eligible for grade selection.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CompletedAssignmentAttemptScore {
    /// Durable identity of the completed Assignment Attempt.
    pub assignment_attempt: AssignmentAttemptId,
    /// One-based sequence number within the enrollment.
    pub attempt_number: u32,
    /// Current score ratio. Extra and negative credit may put it outside 0..=1.
    pub score: f64,
}

/// Assignment Attempt and score selected by an Assignment Attempt Grade Rule.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AssignmentAttemptGradeSelection {
    /// Selected completed Assignment Attempt.
    pub assignment_attempt: AssignmentAttemptId,
    /// Selected score fraction.
    pub score: f64,
}

/// A rejected completed-Assignment-Attempt set or instructor selection.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ScoreModelError {
    /// A completed Assignment Attempt used the invalid zero sequence number.
    InvalidAssignmentAttemptNumber {
        /// Assignment Attempt carrying the invalid number.
        assignment_attempt: AssignmentAttemptId,
    },
    /// A score was non-finite or outside the bounded current-score range.
    InvalidScore {
        /// Assignment Attempt carrying the invalid score.
        assignment_attempt: AssignmentAttemptId,
        /// Rejected score.
        score: f64,
    },
    /// The completed-Assignment-Attempt set repeated one durable identity.
    DuplicateAssignmentAttempt {
        /// Repeated Assignment Attempt identity.
        assignment_attempt: AssignmentAttemptId,
    },
    /// The completed-Assignment-Attempt set repeated a one-based sequence number.
    DuplicateAssignmentAttemptNumber {
        /// Repeated sequence number.
        attempt_number: u32,
    },
    /// An instructor selected an Assignment Attempt outside the completed set.
    UnknownInstructorSelection {
        /// Unknown selected Assignment Attempt.
        assignment_attempt: AssignmentAttemptId,
    },
    /// An automatic policy received an instructor-only selection.
    UnexpectedInstructorSelection,
}

impl std::fmt::Display for ScoreModelError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidAssignmentAttemptNumber { assignment_attempt } => write!(
                formatter,
                "Assignment Attempt {assignment_attempt} has attempt number zero"
            ),
            Self::InvalidScore {
                assignment_attempt,
                score,
            } => {
                write!(
                    formatter,
                    "Assignment Attempt {assignment_attempt} has invalid score {score}"
                )
            }
            Self::DuplicateAssignmentAttempt { assignment_attempt } => write!(
                formatter,
                "Assignment Attempt {assignment_attempt} appears more than once"
            ),
            Self::DuplicateAssignmentAttemptNumber { attempt_number } => {
                write!(
                    formatter,
                    "attempt number {attempt_number} appears more than once"
                )
            }
            Self::UnknownInstructorSelection { assignment_attempt } => {
                write!(
                    formatter,
                    "instructor-selected Assignment Attempt {assignment_attempt} is not completed"
                )
            }
            Self::UnexpectedInstructorSelection => formatter.write_str(
                "only instructor-selected grading accepts a selected Assignment Attempt",
            ),
        }
    }
}

impl std::error::Error for ScoreModelError {}

/// Selects the completed Assignment Attempt whose score reaches the gradebook.
///
/// First and latest use one-based attempt number rather than slice order. Highest
/// keeps the earlier Assignment Attempt when scores tie, matching incremental
/// projection and keeping a stable grade pointer. `InstructorSelected` returns
/// `None` until an instructor explicitly selects a completed Assignment Attempt.
///
/// # Errors
///
/// Returns [`ScoreModelError`] when an attempt number or score is invalid, an
/// identity or attempt number repeats, an automatic policy receives a selection,
/// or an instructor selects an Assignment Attempt outside the completed set.
pub fn select_assignment_attempt_grade(
    completed_assignment_attempts: &[CompletedAssignmentAttemptScore],
    rule: AssignmentAttemptGradeRule,
    instructor_selected: Option<AssignmentAttemptId>,
) -> Result<Option<AssignmentAttemptGradeSelection>, ScoreModelError> {
    validate_completed_assignment_attempts(completed_assignment_attempts)?;

    if rule != AssignmentAttemptGradeRule::InstructorSelected && instructor_selected.is_some() {
        return Err(ScoreModelError::UnexpectedInstructorSelection);
    }

    let selected = match rule {
        AssignmentAttemptGradeRule::First => completed_assignment_attempts
            .iter()
            .min_by_key(|assignment_attempt| assignment_attempt.attempt_number),
        AssignmentAttemptGradeRule::Latest => completed_assignment_attempts
            .iter()
            .max_by_key(|assignment_attempt| assignment_attempt.attempt_number),
        AssignmentAttemptGradeRule::Highest => {
            highest_assignment_attempt(completed_assignment_attempts)
        }
        AssignmentAttemptGradeRule::InstructorSelected => match instructor_selected {
            Some(selected_assignment_attempt) => Some(
                completed_assignment_attempts
                    .iter()
                    .find(|assignment_attempt| {
                        assignment_attempt.assignment_attempt == selected_assignment_attempt
                    })
                    .ok_or(ScoreModelError::UnknownInstructorSelection {
                        assignment_attempt: selected_assignment_attempt,
                    })?,
            ),
            None => None,
        },
    };

    Ok(
        selected.map(|assignment_attempt| AssignmentAttemptGradeSelection {
            assignment_attempt: assignment_attempt.assignment_attempt,
            score: assignment_attempt.score,
        }),
    )
}

fn validate_completed_assignment_attempts(
    completed_assignment_attempts: &[CompletedAssignmentAttemptScore],
) -> Result<(), ScoreModelError> {
    let mut assignment_attempt_ids = HashSet::with_capacity(completed_assignment_attempts.len());
    let mut attempt_numbers = HashSet::with_capacity(completed_assignment_attempts.len());

    for assignment_attempt in completed_assignment_attempts {
        if assignment_attempt.attempt_number == 0 {
            return Err(ScoreModelError::InvalidAssignmentAttemptNumber {
                assignment_attempt: assignment_attempt.assignment_attempt,
            });
        }
        validate_current_score(assignment_attempt.score).map_err(|()| {
            ScoreModelError::InvalidScore {
                assignment_attempt: assignment_attempt.assignment_attempt,
                score: assignment_attempt.score,
            }
        })?;
        if !assignment_attempt_ids.insert(assignment_attempt.assignment_attempt) {
            return Err(ScoreModelError::DuplicateAssignmentAttempt {
                assignment_attempt: assignment_attempt.assignment_attempt,
            });
        }
        if !attempt_numbers.insert(assignment_attempt.attempt_number) {
            return Err(ScoreModelError::DuplicateAssignmentAttemptNumber {
                attempt_number: assignment_attempt.attempt_number,
            });
        }
    }
    Ok(())
}

fn highest_assignment_attempt(
    completed_assignment_attempts: &[CompletedAssignmentAttemptScore],
) -> Option<&CompletedAssignmentAttemptScore> {
    completed_assignment_attempts
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

/// An Assignment Attempt change that affects the compact assignment summary.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AssignmentActivityTransition {
    /// A new Assignment Attempt began at the supplied server timestamp.
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
        /// Completed Assignment Attempt that owns this score.
        assignment_attempt: AssignmentAttemptId,
        /// Score fraction in the inclusive range `-1000.0..=1000.0`.
        /// Extra and negative credit may place it outside `0.0..=1.0`.
        score: f64,
        /// Authoritative server time for the transition.
        at: Timestamp,
    },
}

/// Projects one Assignment Attempt transition into compact Assignment Grade and Assignment Progress records.
///
/// The input is left unchanged. A store can persist the transition and the
/// returned records in the same transaction. `InstructorSelected` keeps the
/// selected grade unchanged because only an explicit instructor action may
/// select a different Assignment Attempt.
///
/// # Errors
///
/// Returns [`AssignmentActivityError`] for an invalid completed-Assignment-Attempt score or when a
/// summary counter overflows.
pub fn project_assignment_activity(
    previous_grade: &AssignmentGrade,
    previous_progress: &AssignmentProgressRecord,
    transition: AssignmentActivityTransition,
    assignment_attempt_grade_rule: AssignmentAttemptGradeRule,
) -> Result<(AssignmentGrade, AssignmentProgressRecord), AssignmentActivityError> {
    let mut next_grade = previous_grade.clone();
    let mut next_progress = previous_progress.clone();

    match transition {
        AssignmentActivityTransition::Started { at } => touch(&mut next_progress, at),
        AssignmentActivityTransition::QuestionAttemptRecorded { at } => {
            next_progress.total_question_attempts = next_progress
                .total_question_attempts
                .checked_add(1)
                .ok_or(AssignmentActivityError::SummaryCounterOverflow)?;
            touch(&mut next_progress, at);
        }
        AssignmentActivityTransition::Completed {
            assignment_attempt,
            score,
            at,
        } => {
            validate_current_score(score)
                .map_err(|()| AssignmentActivityError::InvalidScore { score })?;
            next_progress.completed_assignment_attempt_count = next_progress
                .completed_assignment_attempt_count
                .checked_add(1)
                .ok_or(AssignmentActivityError::SummaryCounterOverflow)?;
            next_grade.first_completed_at = next_grade.first_completed_at.or(Some(at));
            next_grade.latest_assignment_attempt = Some(assignment_attempt);
            next_grade.latest_score = Some(score);
            if next_grade.best_score.is_none_or(|best| score > best) {
                next_grade.best_assignment_attempt = Some(assignment_attempt);
                next_grade.best_score = Some(score);
            }
            match assignment_attempt_grade_rule {
                AssignmentAttemptGradeRule::First if next_grade.current_score.is_none() => {
                    next_grade.current_assignment_attempt = Some(assignment_attempt);
                    next_grade.current_score = Some(score);
                }
                AssignmentAttemptGradeRule::Latest => {
                    next_grade.current_assignment_attempt = Some(assignment_attempt);
                    next_grade.current_score = Some(score);
                }
                AssignmentAttemptGradeRule::Highest => {
                    next_grade.current_assignment_attempt = next_grade.best_assignment_attempt;
                    next_grade.current_score = next_grade.best_score;
                }
                AssignmentAttemptGradeRule::First
                | AssignmentAttemptGradeRule::InstructorSelected => {}
            }
            touch(&mut next_progress, at);
        }
    }

    Ok((next_grade, next_progress))
}

/// Advances the activity timestamp without moving it backward.
fn touch(summary: &mut AssignmentProgressRecord, at: Timestamp) {
    summary.last_activity_at = Some(
        summary
            .last_activity_at
            .map_or(at, |previous| previous.max(at)),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use question_model::{AssignmentId, StudentRecordId};
    use uuid::Uuid;

    fn assignment_attempt(
        id: u128,
        attempt_number: u32,
        score: f64,
    ) -> CompletedAssignmentAttemptScore {
        CompletedAssignmentAttemptScore {
            assignment_attempt: AssignmentAttemptId::from_uuid(Uuid::from_u128(id)),
            attempt_number,
            score,
        }
    }

    fn empty_summary() -> AssignmentProgressRecord {
        AssignmentProgressRecord::empty(
            StudentRecordId::from_uuid(Uuid::from_u128(11)),
            AssignmentId::from_uuid(Uuid::from_u128(12)),
        )
    }

    fn empty_grade() -> AssignmentGrade {
        AssignmentGrade::empty(
            StudentRecordId::from_uuid(Uuid::from_u128(11)),
            AssignmentId::from_uuid(Uuid::from_u128(12)),
        )
    }

    fn projected_score(
        completed_assignment_attempts: &[CompletedAssignmentAttemptScore],
        rule: AssignmentAttemptGradeRule,
    ) -> Option<f64> {
        let mut grade = empty_grade();
        let mut progress = empty_summary();
        for completed in completed_assignment_attempts {
            (grade, progress) = project_assignment_activity(
                &grade,
                &progress,
                AssignmentActivityTransition::Completed {
                    assignment_attempt: completed.assignment_attempt,
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
        let completed_assignment_attempts = [
            assignment_attempt(1, 1, 0.4),
            assignment_attempt(2, 2, 0.9),
            assignment_attempt(3, 3, 0.7),
        ];
        let cases = [
            (
                AssignmentAttemptGradeRule::First,
                completed_assignment_attempts[0],
            ),
            (
                AssignmentAttemptGradeRule::Latest,
                completed_assignment_attempts[2],
            ),
            (
                AssignmentAttemptGradeRule::Highest,
                completed_assignment_attempts[1],
            ),
        ];

        for (policy, expected) in cases {
            assert_eq!(
                select_assignment_attempt_grade(&completed_assignment_attempts, policy, None),
                Ok(Some(AssignmentAttemptGradeSelection {
                    assignment_attempt: expected.assignment_attempt,
                    score: expected.score,
                })),
                "{policy:?} batch selection"
            );
            assert_eq!(
                projected_score(&completed_assignment_attempts, policy),
                Some(expected.score),
                "{policy:?} incremental projection"
            );
        }
    }

    #[test]
    fn highest_tie_keeps_the_earlier_assignment_attempt() {
        let completed_assignment_attempts =
            [assignment_attempt(1, 2, 0.9), assignment_attempt(2, 1, 0.9)];

        assert_eq!(
            select_assignment_attempt_grade(
                &completed_assignment_attempts,
                AssignmentAttemptGradeRule::Highest,
                None
            ),
            Ok(Some(AssignmentAttemptGradeSelection {
                assignment_attempt: completed_assignment_attempts[1].assignment_attempt,
                score: 0.9,
            }))
        );
    }

    #[test]
    fn instructor_selection_is_explicit_and_must_name_a_completed_assignment_attempt() {
        let completed_assignment_attempts =
            [assignment_attempt(1, 1, 0.4), assignment_attempt(2, 2, 0.9)];

        assert_eq!(
            select_assignment_attempt_grade(
                &completed_assignment_attempts,
                AssignmentAttemptGradeRule::InstructorSelected,
                None
            ),
            Ok(None)
        );
        assert_eq!(
            select_assignment_attempt_grade(
                &completed_assignment_attempts,
                AssignmentAttemptGradeRule::InstructorSelected,
                Some(completed_assignment_attempts[0].assignment_attempt),
            ),
            Ok(Some(AssignmentAttemptGradeSelection {
                assignment_attempt: completed_assignment_attempts[0].assignment_attempt,
                score: 0.4,
            }))
        );
        let unknown = AssignmentAttemptId::from_uuid(Uuid::from_u128(99));
        assert_eq!(
            select_assignment_attempt_grade(
                &completed_assignment_attempts,
                AssignmentAttemptGradeRule::InstructorSelected,
                Some(unknown),
            ),
            Err(ScoreModelError::UnknownInstructorSelection {
                assignment_attempt: unknown
            })
        );
    }

    #[test]
    fn malformed_completed_assignment_attempt_sets_are_rejected() {
        let duplicate_id = [assignment_attempt(1, 1, 0.4), assignment_attempt(1, 2, 0.9)];
        let duplicate_number = [assignment_attempt(1, 1, 0.4), assignment_attempt(2, 1, 0.9)];
        let invalid_score = [assignment_attempt(1, 1, f64::NAN)];

        assert_eq!(
            select_assignment_attempt_grade(&duplicate_id, AssignmentAttemptGradeRule::First, None),
            Err(ScoreModelError::DuplicateAssignmentAttempt {
                assignment_attempt: duplicate_id[0].assignment_attempt,
            })
        );
        assert_eq!(
            select_assignment_attempt_grade(
                &duplicate_number,
                AssignmentAttemptGradeRule::First,
                None,
            ),
            Err(ScoreModelError::DuplicateAssignmentAttemptNumber { attempt_number: 1 })
        );
        assert!(matches!(
            select_assignment_attempt_grade(&invalid_score, AssignmentAttemptGradeRule::First, None),
            Err(ScoreModelError::InvalidScore { score, .. }) if score.is_nan()
        ));
    }

    #[test]
    fn activity_projection_preserves_instructor_choice_and_monotonic_activity() {
        let mut previous_grade = empty_grade();
        previous_grade.current_assignment_attempt =
            Some(AssignmentAttemptId::from_uuid(Uuid::from_u128(4)));
        previous_grade.current_score = Some(0.5);
        let mut previous_progress = empty_summary();
        previous_progress.last_activity_at = Some(Timestamp::from_unix_millis(10));

        let (next_grade, next_progress) = project_assignment_activity(
            &previous_grade,
            &previous_progress,
            AssignmentActivityTransition::Completed {
                assignment_attempt: AssignmentAttemptId::from_uuid(Uuid::from_u128(5)),
                score: 0.9,
                at: Timestamp::from_unix_millis(9),
            },
            AssignmentAttemptGradeRule::InstructorSelected,
        )
        .expect("valid completion should project");

        assert_eq!(next_grade.current_score, Some(0.5));
        assert_eq!(
            next_progress.last_activity_at,
            Some(Timestamp::from_unix_millis(10))
        );
    }
}
