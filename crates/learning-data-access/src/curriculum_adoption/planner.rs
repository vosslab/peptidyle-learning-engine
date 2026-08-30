//! ID-free destination materialization and Blueprint assignment source comparison.

use question_model::curriculum_adoption::{
    CurriculumSemanticAssignment, CurriculumSemanticAssignmentEntry,
};
use question_model::{
    AssignmentInstructions, AssignmentScoringMode, CourseTerm, PointValue, PoolDrawAlgorithm,
    ProblemVersionRef, ResolvedRelativeAssignmentSchedule, ReusableAssignmentDefaults,
    SelectionOrdering,
};
use serde::{Deserialize, Serialize};

#[cfg(any(test, feature = "test-support"))]
use question_model::BaseAssignmentPolicy;

use super::semantic_snapshot::SemanticPlannerError;

/// One exact pool candidate in an ID-free destination plan.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct AssignmentMaterializationCandidate {
    pub(crate) position: u32,
    pub(crate) reference: ProblemVersionRef,
}

/// One ordered destination entry before adapter-local storage IDs are minted.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub(crate) enum AssignmentMaterializationEntry {
    Fixed {
        position: u32,
        reference: ProblemVersionRef,
        points_possible: PointValue,
        scoring_mode: AssignmentScoringMode,
    },
    Pool {
        position: u32,
        candidates: Vec<AssignmentMaterializationCandidate>,
        draw_count: u32,
        points_per_item: PointValue,
        ordering: SelectionOrdering,
        algorithm: PoolDrawAlgorithm,
    },
}

/// Complete assignment meaning with a qmodel-resolved target schedule and no storage IDs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct AssignmentMaterializationPlan {
    pub(crate) title: String,
    pub(crate) instructions: AssignmentInstructions,
    pub(crate) entries: Vec<AssignmentMaterializationEntry>,
    pub(crate) defaults: ReusableAssignmentDefaults,
    pub(crate) schedule: ResolvedRelativeAssignmentSchedule,
}

impl AssignmentMaterializationPlan {
    /// Projects the plan's resolved schedule and reusable limits into ordinary policy storage.
    #[cfg(any(test, feature = "test-support"))]
    pub(crate) fn base_policy(&self) -> BaseAssignmentPolicy {
        BaseAssignmentPolicy {
            available_at: self
                .schedule
                .available_at
                .as_ref()
                .map(|value| value.timestamp),
            due_at: self.schedule.due_at.as_ref().map(|value| value.timestamp),
            closes_at: self
                .schedule
                .closes_at
                .as_ref()
                .map(|value| value.timestamp),
            time_limit_seconds: self.defaults.time_limit_seconds,
            attempt_limit: self.defaults.attempt_limit,
            late_submission: self.defaults.late_submission,
            deadline_behavior: self.defaults.deadline_behavior,
        }
    }
}

/// Resolves one validated assignment for an adapter-owned destination transaction.
pub(crate) fn plan_assignment_materialization(
    assignment: &CurriculumSemanticAssignment,
    target_term: &CourseTerm,
) -> Result<AssignmentMaterializationPlan, SemanticPlannerError> {
    let schedule = assignment
        .schedule()
        .resolve_for_target_term(target_term)
        .map_err(|error| SemanticPlannerError::Schedule(error.to_string()))?;
    Ok(AssignmentMaterializationPlan {
        title: assignment.title().to_owned(),
        instructions: assignment.instructions().clone(),
        entries: plan_assignment_entries(assignment)?,
        defaults: assignment.defaults().clone(),
        schedule,
    })
}

pub(crate) fn plan_assignment_entries(
    assignment: &CurriculumSemanticAssignment,
) -> Result<Vec<AssignmentMaterializationEntry>, SemanticPlannerError> {
    assignment
        .entries()
        .iter()
        .enumerate()
        .map(|(position, entry)| {
            let position = u32::try_from(position).map_err(|_| {
                SemanticPlannerError::InvalidPosition(
                    "assignment materialization position exceeds u32".into(),
                )
            })?;
            match entry {
                CurriculumSemanticAssignmentEntry::Fixed {
                    reference,
                    points_possible,
                    scoring_mode,
                } => Ok(AssignmentMaterializationEntry::Fixed {
                    position,
                    reference: *reference,
                    points_possible: *points_possible,
                    scoring_mode: *scoring_mode,
                }),
                CurriculumSemanticAssignmentEntry::Pool(pool) => {
                    let candidates = pool
                        .candidates()
                        .iter()
                        .enumerate()
                        .map(|(position, reference)| {
                            Ok(AssignmentMaterializationCandidate {
                                position: u32::try_from(position).map_err(|_| {
                                    SemanticPlannerError::InvalidPosition(
                                        "pool materialization position exceeds u32".into(),
                                    )
                                })?,
                                reference: *reference,
                            })
                        })
                        .collect::<Result<Vec<_>, SemanticPlannerError>>()?;
                    Ok(AssignmentMaterializationEntry::Pool {
                        position,
                        candidates,
                        draw_count: pool.draw_count(),
                        points_per_item: pool.points_per_item(),
                        ordering: pool.ordering(),
                        algorithm: pool.algorithm(),
                    })
                }
            }
        })
        .collect()
}
