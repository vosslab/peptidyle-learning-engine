//! Score selection and compact summary projection (MOD-SCORE).
//!
//! `score` selects a completed run for reconciliation and explicit instructor
//! actions. `project_summary` updates the ordinary course-page projection one
//! transition at a time, so storage never scans run or attempt history on a
//! synchronous page request. Both functions are pure.

use std::collections::HashSet;

use question_model::{ActivityTimestamp, GradePolicy, RunId, StudentAssignmentSummary};

use crate::run::RunModelError;

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

/// One completed run eligible for grade selection.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CompletedRunScore {
    /// Durable identity of the completed run.
    pub run: RunId,
    /// One-based sequence number within the enrollment.
    pub run_number: u32,
    /// Current score ratio. Extra and negative credit may put it outside 0..=1.
    pub score: f64,
}

/// Run and score selected by a grade policy.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GradeSelection {
    /// Selected completed run.
    pub run: RunId,
    /// Selected score fraction.
    pub score: f64,
}

/// A rejected completed-run set or instructor selection.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ScoreModelError {
    /// A completed run used the invalid zero sequence number.
    InvalidRunNumber {
        /// Run carrying the invalid number.
        run: RunId,
    },
    /// A score was non-finite or outside the bounded current-score range.
    InvalidScore {
        /// Run carrying the invalid score.
        run: RunId,
        /// Rejected score.
        score: f64,
    },
    /// The completed-run set repeated one durable run identity.
    DuplicateRun {
        /// Repeated run identity.
        run: RunId,
    },
    /// The completed-run set repeated a one-based sequence number.
    DuplicateRunNumber {
        /// Repeated sequence number.
        run_number: u32,
    },
    /// An instructor selected a run outside the completed-run set.
    UnknownInstructorSelection {
        /// Unknown selected run.
        run: RunId,
    },
    /// An automatic policy received an instructor-only selection.
    UnexpectedInstructorSelection,
}

impl std::fmt::Display for ScoreModelError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidRunNumber { run } => write!(formatter, "run {run} has run number zero"),
            Self::InvalidScore { run, score } => {
                write!(formatter, "run {run} has invalid score {score}")
            }
            Self::DuplicateRun { run } => write!(formatter, "run {run} appears more than once"),
            Self::DuplicateRunNumber { run_number } => {
                write!(formatter, "run number {run_number} appears more than once")
            }
            Self::UnknownInstructorSelection { run } => {
                write!(formatter, "instructor-selected run {run} is not completed")
            }
            Self::UnexpectedInstructorSelection => {
                formatter.write_str("only instructor-selected grading accepts a selected run")
            }
        }
    }
}

impl std::error::Error for ScoreModelError {}

/// Selects the completed run whose score reaches the gradebook.
///
/// First and latest use one-based run number rather than slice order. Highest
/// keeps the earlier run when scores tie, matching incremental projection and
/// keeping a stable grade pointer. `InstructorSelected` returns `None` until an
/// instructor explicitly selects a completed run.
///
/// # Errors
///
/// Returns [`ScoreModelError`] when a run number or score is invalid, an
/// identity or run number repeats, an automatic policy receives a selection,
/// or an instructor selects a run outside the completed set.
pub fn score(
    completed_runs: &[CompletedRunScore],
    policy: GradePolicy,
    instructor_selected: Option<RunId>,
) -> Result<Option<GradeSelection>, ScoreModelError> {
    validate_completed_runs(completed_runs)?;

    if policy != GradePolicy::InstructorSelected && instructor_selected.is_some() {
        return Err(ScoreModelError::UnexpectedInstructorSelection);
    }

    let selected = match policy {
        GradePolicy::First => completed_runs.iter().min_by_key(|run| run.run_number),
        GradePolicy::Latest => completed_runs.iter().max_by_key(|run| run.run_number),
        GradePolicy::Highest => highest_run(completed_runs),
        GradePolicy::InstructorSelected => match instructor_selected {
            Some(selected_run) => Some(
                completed_runs
                    .iter()
                    .find(|run| run.run == selected_run)
                    .ok_or(ScoreModelError::UnknownInstructorSelection { run: selected_run })?,
            ),
            None => None,
        },
    };

    Ok(selected.map(|run| GradeSelection {
        run: run.run,
        score: run.score,
    }))
}

fn validate_completed_runs(completed_runs: &[CompletedRunScore]) -> Result<(), ScoreModelError> {
    let mut run_ids = HashSet::with_capacity(completed_runs.len());
    let mut run_numbers = HashSet::with_capacity(completed_runs.len());

    for run in completed_runs {
        if run.run_number == 0 {
            return Err(ScoreModelError::InvalidRunNumber { run: run.run });
        }
        validate_current_score(run.score).map_err(|()| ScoreModelError::InvalidScore {
            run: run.run,
            score: run.score,
        })?;
        if !run_ids.insert(run.run) {
            return Err(ScoreModelError::DuplicateRun { run: run.run });
        }
        if !run_numbers.insert(run.run_number) {
            return Err(ScoreModelError::DuplicateRunNumber {
                run_number: run.run_number,
            });
        }
    }
    Ok(())
}

fn highest_run(completed_runs: &[CompletedRunScore]) -> Option<&CompletedRunScore> {
    completed_runs.iter().reduce(|selected, candidate| {
        if candidate.score > selected.score
            || (candidate.score == selected.score && candidate.run_number < selected.run_number)
        {
            candidate
        } else {
            selected
        }
    })
}

/// A run change that affects the compact assignment summary.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RunTransition {
    /// A new run began at the supplied server timestamp.
    Started {
        /// Authoritative server time for the transition.
        at: ActivityTimestamp,
    },
    /// One question response was recorded.
    QuestionAttemptRecorded {
        /// Authoritative server time for the transition.
        at: ActivityTimestamp,
    },
    /// Derived completion was recorded with its final score.
    Completed {
        /// Score fraction in the inclusive range `-1000.0..=1000.0`.
        /// Extra and negative credit may place it outside `0.0..=1.0`.
        score: f64,
        /// Authoritative server time for the transition.
        at: ActivityTimestamp,
    },
}

/// Projects one run transition into the compact assignment summary.
///
/// The input is left unchanged. A store can persist the transition and the
/// returned projection in the same transaction. `InstructorSelected` keeps
/// `current_score` unchanged because only an explicit instructor action may
/// select a different run.
///
/// # Errors
///
/// Returns [`RunModelError`] for an invalid completed-run score or when a
/// summary counter overflows.
pub fn project_summary(
    previous: &StudentAssignmentSummary,
    transition: RunTransition,
    grade_policy: GradePolicy,
) -> Result<StudentAssignmentSummary, RunModelError> {
    let mut next = previous.clone();

    match transition {
        RunTransition::Started { at } => touch(&mut next, at),
        RunTransition::QuestionAttemptRecorded { at } => {
            next.total_question_attempts = next
                .total_question_attempts
                .checked_add(1)
                .ok_or(RunModelError::SummaryCounterOverflow)?;
            touch(&mut next, at);
        }
        RunTransition::Completed { score, at } => {
            validate_current_score(score).map_err(|()| RunModelError::InvalidScore { score })?;
            next.completed_run_count = next
                .completed_run_count
                .checked_add(1)
                .ok_or(RunModelError::SummaryCounterOverflow)?;
            next.latest_score = Some(score);
            next.best_score = Some(next.best_score.map_or(score, |best| best.max(score)));
            next.current_score = match grade_policy {
                GradePolicy::First => next.current_score.or(Some(score)),
                GradePolicy::Latest => Some(score),
                GradePolicy::Highest => next.best_score,
                GradePolicy::InstructorSelected => next.current_score,
            };
            touch(&mut next, at);
        }
    }

    Ok(next)
}

/// Advances the activity timestamp without moving it backward.
fn touch(summary: &mut StudentAssignmentSummary, at: ActivityTimestamp) {
    summary.last_activity_at = Some(
        summary
            .last_activity_at
            .map_or(at, |previous| previous.max(at)),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use question_model::EnrollmentId;
    use uuid::Uuid;

    fn run(id: u128, run_number: u32, score: f64) -> CompletedRunScore {
        CompletedRunScore {
            run: RunId::from_uuid(Uuid::from_u128(id)),
            run_number,
            score,
        }
    }

    fn empty_summary() -> StudentAssignmentSummary {
        StudentAssignmentSummary::empty(EnrollmentId::from_uuid(Uuid::from_u128(11)))
    }

    fn projected_score(completed_runs: &[CompletedRunScore], policy: GradePolicy) -> Option<f64> {
        let mut summary = empty_summary();
        for completed in completed_runs {
            summary = project_summary(
                &summary,
                RunTransition::Completed {
                    score: completed.score,
                    at: ActivityTimestamp::from_unix_millis(i64::from(completed.run_number)),
                },
                policy,
            )
            .expect("fixture scores should project");
        }
        summary.current_score
    }

    #[test]
    fn hand_computed_fixture_agrees_for_batch_and_incremental_scoring() {
        let completed_runs = [run(1, 1, 0.4), run(2, 2, 0.9), run(3, 3, 0.7)];
        let cases = [
            (GradePolicy::First, completed_runs[0]),
            (GradePolicy::Latest, completed_runs[2]),
            (GradePolicy::Highest, completed_runs[1]),
        ];

        for (policy, expected) in cases {
            assert_eq!(
                score(&completed_runs, policy, None),
                Ok(Some(GradeSelection {
                    run: expected.run,
                    score: expected.score,
                })),
                "{policy:?} batch selection"
            );
            assert_eq!(
                projected_score(&completed_runs, policy),
                Some(expected.score),
                "{policy:?} incremental projection"
            );
        }
    }

    #[test]
    fn highest_tie_keeps_the_earlier_run() {
        let completed_runs = [run(1, 2, 0.9), run(2, 1, 0.9)];

        assert_eq!(
            score(&completed_runs, GradePolicy::Highest, None),
            Ok(Some(GradeSelection {
                run: completed_runs[1].run,
                score: 0.9,
            }))
        );
    }

    #[test]
    fn instructor_selection_is_explicit_and_must_name_a_completed_run() {
        let completed_runs = [run(1, 1, 0.4), run(2, 2, 0.9)];

        assert_eq!(
            score(&completed_runs, GradePolicy::InstructorSelected, None),
            Ok(None)
        );
        assert_eq!(
            score(
                &completed_runs,
                GradePolicy::InstructorSelected,
                Some(completed_runs[0].run),
            ),
            Ok(Some(GradeSelection {
                run: completed_runs[0].run,
                score: 0.4,
            }))
        );
        let unknown = RunId::from_uuid(Uuid::from_u128(99));
        assert_eq!(
            score(
                &completed_runs,
                GradePolicy::InstructorSelected,
                Some(unknown),
            ),
            Err(ScoreModelError::UnknownInstructorSelection { run: unknown })
        );
    }

    #[test]
    fn malformed_completed_run_sets_are_rejected() {
        let duplicate_id = [run(1, 1, 0.4), run(1, 2, 0.9)];
        let duplicate_number = [run(1, 1, 0.4), run(2, 1, 0.9)];
        let invalid_score = [run(1, 1, f64::NAN)];

        assert_eq!(
            score(&duplicate_id, GradePolicy::First, None),
            Err(ScoreModelError::DuplicateRun {
                run: duplicate_id[0].run,
            })
        );
        assert_eq!(
            score(&duplicate_number, GradePolicy::First, None),
            Err(ScoreModelError::DuplicateRunNumber { run_number: 1 })
        );
        assert!(matches!(
            score(&invalid_score, GradePolicy::First, None),
            Err(ScoreModelError::InvalidScore { score, .. }) if score.is_nan()
        ));
    }

    #[test]
    fn summary_projection_preserves_instructor_choice_and_monotonic_activity() {
        let mut previous = empty_summary();
        previous.current_score = Some(0.5);
        previous.last_activity_at = Some(ActivityTimestamp::from_unix_millis(10));

        let next = project_summary(
            &previous,
            RunTransition::Completed {
                score: 0.9,
                at: ActivityTimestamp::from_unix_millis(9),
            },
            GradePolicy::InstructorSelected,
        )
        .expect("valid completion should project");

        assert_eq!(next.current_score, Some(0.5));
        assert_eq!(
            next.last_activity_at,
            Some(ActivityTimestamp::from_unix_millis(10))
        );
    }
}
