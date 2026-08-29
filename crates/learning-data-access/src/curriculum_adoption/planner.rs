//! ID-free destination planning and shared fast-forward decisions.

use question_model::curriculum_adoption::{
    CurriculumSemanticAssignment, CurriculumSemanticAssignmentEntry, CurriculumSemanticComparison,
    CurriculumSemanticPayload,
};
use question_model::{
    AssignmentDefinitionSourceView, AssignmentFastForwardDecision, AssignmentInstructions,
    AssignmentScoringMode, CourseTerm, PointValue, PoolDrawAlgorithm,
    PreservedAssignmentRecoveryAction, ProblemVersionRef, ReplacementQuestionChoices,
    ResolvedRelativeAssignmentSchedule, ReusableAssignmentDefaults, SelectionOrdering,
    UnavailablePinRecoveryAction,
};
use serde::{Deserialize, Serialize};

#[cfg(any(test, feature = "test-support"))]
use question_model::BaseAssignmentPolicy;

use super::PositionedPin;
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
    let entries = plan_assignment_entries(assignment)?;
    Ok(AssignmentMaterializationPlan {
        title: assignment.title().to_owned(),
        instructions: assignment.instructions().clone(),
        entries,
        defaults: assignment.defaults().clone(),
        schedule,
    })
}

/// Adapter-resolved facts needed for the shared fast-forward precedence matrix.
pub(crate) struct FastForwardAssessmentInput<'a> {
    /// `None` means the destination import came from rollover rather than reusable content.
    pub(crate) imported_source: Option<AssignmentDefinitionSourceView>,
    pub(crate) requested_source: AssignmentDefinitionSourceView,
    /// Current authorized source revision. Adapters resolve this only for a matching locator.
    pub(crate) current_source: AssignmentDefinitionSourceView,
    pub(crate) baseline: &'a CurriculumSemanticPayload,
    pub(crate) current: &'a CurriculumSemanticPayload,
    pub(crate) issued_work: bool,
    pub(crate) unavailable_pin: Option<PositionedPin>,
}

/// Shared meaning decision before an adapter adds catalog-backed recovery choices.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub(crate) enum FastForwardAssessment {
    Eligible,
    SourceRevisionDrift {
        source: AssignmentDefinitionSourceView,
    },
    Divergent,
    IssuedWork,
    UnavailablePin {
        pin: PositionedPin,
    },
}

/// Applies the approved public decision precedence to adapter-resolved facts.
pub(crate) fn assess_fast_forward(input: FastForwardAssessmentInput<'_>) -> FastForwardAssessment {
    let Some(imported_source) = input.imported_source else {
        return FastForwardAssessment::SourceRevisionDrift {
            source: input.requested_source,
        };
    };
    if !same_source_locator(imported_source, input.requested_source) {
        return FastForwardAssessment::SourceRevisionDrift {
            source: input.requested_source,
        };
    }
    if input.current_source != input.requested_source
        || !source_is_newer(imported_source, input.requested_source)
    {
        return FastForwardAssessment::SourceRevisionDrift {
            source: input.current_source,
        };
    }
    if matches!(
        input.baseline.compare(input.current),
        CurriculumSemanticComparison::Changed { .. }
    ) {
        return FastForwardAssessment::Divergent;
    }
    if input.issued_work {
        return FastForwardAssessment::IssuedWork;
    }
    if let Some(pin) = input.unavailable_pin {
        return FastForwardAssessment::UnavailablePin { pin };
    }
    FastForwardAssessment::Eligible
}

/// Complete adapter-resolved facts for one public fast-forward projection.
pub(crate) struct FastForwardProjectionInput<'a> {
    /// `None` means the destination import came from rollover rather than reusable content.
    pub(crate) imported_source: Option<AssignmentDefinitionSourceView>,
    pub(crate) requested_source: AssignmentDefinitionSourceView,
    /// Current authorized source revision. Adapters resolve this only for a matching locator.
    pub(crate) current_source: AssignmentDefinitionSourceView,
    pub(crate) baseline: &'a CurriculumSemanticPayload,
    pub(crate) current: &'a CurriculumSemanticPayload,
    pub(crate) issued_work: bool,
    pub(crate) unavailable_pin: Option<PositionedPin>,
    /// Adapter-discovered, bounded public choices for the unavailable pin, when one exists.
    pub(crate) replacement_choices: Option<ReplacementQuestionChoices>,
}

/// Applies the shared precedence matrix and closes it into the public decision contract.
pub(crate) fn project_fast_forward_decision(
    input: FastForwardProjectionInput<'_>,
) -> Result<AssignmentFastForwardDecision, SemanticPlannerError> {
    let assessment = assess_fast_forward(FastForwardAssessmentInput {
        imported_source: input.imported_source,
        requested_source: input.requested_source,
        current_source: input.current_source,
        baseline: input.baseline,
        current: input.current,
        issued_work: input.issued_work,
        unavailable_pin: input.unavailable_pin,
    });
    match (assessment, input.replacement_choices) {
        (FastForwardAssessment::Eligible, None) => Ok(AssignmentFastForwardDecision::Eligible),
        (FastForwardAssessment::SourceRevisionDrift { source }, None) => {
            Ok(AssignmentFastForwardDecision::SourceRevisionDrift { source })
        }
        (FastForwardAssessment::Divergent, None) => Ok(AssignmentFastForwardDecision::Divergent {
            recovery: PreservedAssignmentRecoveryAction::CreateSourceDerivedAssignment,
        }),
        (FastForwardAssessment::IssuedWork, None) => {
            Ok(AssignmentFastForwardDecision::IssuedWork {
                recovery: PreservedAssignmentRecoveryAction::CreateSourceDerivedAssignment,
            })
        }
        (FastForwardAssessment::UnavailablePin { pin }, Some(candidates)) => {
            Ok(AssignmentFastForwardDecision::UnavailablePin {
                recovery: UnavailablePinRecoveryAction::SelectReplacementQuestion {
                    position: pin.position(),
                    candidates,
                },
            })
        }
        (FastForwardAssessment::UnavailablePin { .. }, None) => {
            Err(SemanticPlannerError::InvalidReplacement(
                "an unavailable pin requires bounded replacement choices".into(),
            ))
        }
        (_, Some(_)) => Err(SemanticPlannerError::InvalidReplacement(
            "replacement choices require an unavailable pin".into(),
        )),
    }
}

/// Returns whether two observed sources name the same reusable definition locator.
pub(crate) fn same_source_locator(
    left: AssignmentDefinitionSourceView,
    right: AssignmentDefinitionSourceView,
) -> bool {
    match (left, right) {
        (
            AssignmentDefinitionSourceView::Blueprint(left),
            AssignmentDefinitionSourceView::Blueprint(right),
        ) => left.reference == right.reference,
        (
            AssignmentDefinitionSourceView::Alpha(left),
            AssignmentDefinitionSourceView::Alpha(right),
        ) => {
            left.source().reference == right.source().reference
                && left.module_index() == right.module_index()
                && left.assignment_index() == right.assignment_index()
        }
        _ => false,
    }
}

fn source_is_newer(
    old: AssignmentDefinitionSourceView,
    new: AssignmentDefinitionSourceView,
) -> bool {
    match (old, new) {
        (
            AssignmentDefinitionSourceView::Blueprint(old),
            AssignmentDefinitionSourceView::Blueprint(new),
        ) => new.revision.value() > old.revision.value(),
        (
            AssignmentDefinitionSourceView::Alpha(old),
            AssignmentDefinitionSourceView::Alpha(new),
        ) => new.source().revision.value() > old.source().revision.value(),
        _ => false,
    }
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
                            let position = u32::try_from(position).map_err(|_| {
                                SemanticPlannerError::InvalidPosition(
                                    "pool materialization position exceeds u32".into(),
                                )
                            })?;
                            Ok(AssignmentMaterializationCandidate {
                                position,
                                reference: *reference,
                            })
                        })
                        .collect::<Result<Vec<_>, _>>()?;
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

#[cfg(test)]
mod tests {
    use super::*;
    use question_model::curriculum_adoption::{
        CurriculumSemanticAssignment, CurriculumSemanticAssignmentEntry, CurriculumSemanticPool,
    };
    use question_model::{
        AssignmentDeadlineBehavior, BlueprintReference, BlueprintRevision, CompletionRequirement,
        ContinuedPractice, CurriculumPinPosition, GradePolicy, LateSubmissionPolicy,
        LocalTimeOfDay, ObservedBlueprintSource, ProblemId, RelativeAssignmentSchedule,
        RelativeScheduleMoment, RunPolicies, StudentDisclosurePolicy, VariationPolicy, VersionId,
    };

    fn reference(value: u128) -> ProblemVersionRef {
        ProblemVersionRef {
            problem: ProblemId::from_uuid(uuid::Uuid::from_u128(value)),
            version: VersionId::from_uuid(uuid::Uuid::from_u128(value + 1)),
        }
    }

    fn assignment(
        title: &str,
        schedule: RelativeAssignmentSchedule,
    ) -> CurriculumSemanticAssignment {
        CurriculumSemanticAssignment::new(
            title.into(),
            AssignmentInstructions::default(),
            vec![CurriculumSemanticAssignmentEntry::Fixed {
                reference: reference(10),
                points_possible: PointValue::from_whole(1),
                scoring_mode: AssignmentScoringMode::Normal,
            }],
            defaults(),
            schedule,
        )
        .expect("semantic assignment")
    }

    fn defaults() -> ReusableAssignmentDefaults {
        ReusableAssignmentDefaults {
            time_limit_seconds: None,
            attempt_limit: None,
            late_submission: LateSubmissionPolicy::Accept,
            deadline_behavior: AssignmentDeadlineBehavior::AutoSubmit,
            run_policies: RunPolicies {
                completion: CompletionRequirement::AnswerAll,
                grade: GradePolicy::Highest,
                continued_practice: ContinuedPractice::Unlimited,
                variation: VariationPolicy::NewSeeds,
            },
            student_disclosure: StudentDisclosurePolicy::default(),
        }
    }

    fn source(reference: u64, revision: u64) -> AssignmentDefinitionSourceView {
        AssignmentDefinitionSourceView::Blueprint(ObservedBlueprintSource {
            reference: BlueprintReference::new(reference).expect("reference"),
            revision: BlueprintRevision::new(revision).expect("revision"),
        })
    }

    #[test]
    fn materialization_plan_is_id_free_ordered_and_qmodel_resolved() {
        let term =
            CourseTerm::from_parts("2026-08-24", "2026-12-18", "America/Chicago").expect("term");
        let schedule = RelativeAssignmentSchedule {
            available_at: Some(RelativeScheduleMoment {
                day_offset: 0,
                local_time: LocalTimeOfDay::parse("09:00:00.000").expect("time"),
            }),
            due_at: None,
            closes_at: None,
        };
        let semantic = CurriculumSemanticAssignment::new(
            "Quiz".into(),
            AssignmentInstructions::default(),
            vec![
                CurriculumSemanticAssignmentEntry::Fixed {
                    reference: reference(10),
                    points_possible: PointValue::from_whole(1),
                    scoring_mode: AssignmentScoringMode::Normal,
                },
                CurriculumSemanticAssignmentEntry::Pool(
                    CurriculumSemanticPool::new(
                        vec![reference(20), reference(30)],
                        1,
                        PointValue::from_whole(2),
                        SelectionOrdering::CandidateOrder,
                        PoolDrawAlgorithm::V1,
                    )
                    .expect("pool"),
                ),
            ],
            defaults(),
            schedule,
        )
        .expect("semantic assignment");
        let plan = plan_assignment_materialization(&semantic, &term).expect("materialization plan");
        assert!(matches!(
            plan.entries.as_slice(),
            [
                AssignmentMaterializationEntry::Fixed { position: 0, reference: fixed, .. },
                AssignmentMaterializationEntry::Pool { position: 1, candidates, .. }
            ] if *fixed == reference(10)
                && candidates.as_slice() == [
                    AssignmentMaterializationCandidate { position: 0, reference: reference(20) },
                    AssignmentMaterializationCandidate { position: 1, reference: reference(30) },
                ]
        ));
        assert_eq!(
            plan.base_policy().available_at,
            plan.schedule
                .available_at
                .as_ref()
                .map(|value| value.timestamp)
        );
    }

    #[test]
    fn materialization_plan_returns_qmodel_dst_correction_as_error() {
        let term = CourseTerm::from_parts("2026-03-08", "2026-03-08", "America/Chicago")
            .expect("spring-forward term");
        let schedule = RelativeAssignmentSchedule {
            available_at: Some(RelativeScheduleMoment {
                day_offset: 0,
                local_time: LocalTimeOfDay::parse("02:30:00.000").expect("wall clock"),
            }),
            due_at: None,
            closes_at: None,
        };
        let error = plan_assignment_materialization(&assignment("Gap", schedule), &term)
            .expect_err("DST gap requires correction");
        assert!(matches!(error, SemanticPlannerError::Schedule(_)));
    }

    #[test]
    fn fast_forward_matrix_preserves_decision_precedence() {
        let baseline = CurriculumSemanticPayload::assignment(assignment(
            "Baseline",
            RelativeAssignmentSchedule::default(),
        ));
        let divergent = CurriculumSemanticPayload::assignment(assignment(
            "Changed",
            RelativeAssignmentSchedule::default(),
        ));
        let old = source(1, 1);
        let requested = source(1, 2);
        let pin = PositionedPin {
            position: CurriculumPinPosition::new(None, 0, 0, None).expect("position"),
            reference: reference(10),
        };
        let assess = |imported_source, current, issued_work, unavailable_pin| {
            assess_fast_forward(FastForwardAssessmentInput {
                imported_source,
                requested_source: requested,
                current_source: requested,
                baseline: &baseline,
                current,
                issued_work,
                unavailable_pin,
            })
        };
        assert!(matches!(
            assess(None, &baseline, false, None),
            FastForwardAssessment::SourceRevisionDrift { .. }
        ));
        assert_eq!(
            assess(Some(old), &divergent, true, Some(pin)),
            FastForwardAssessment::Divergent
        );
        assert_eq!(
            assess(Some(old), &baseline, true, Some(pin)),
            FastForwardAssessment::IssuedWork
        );
        assert_eq!(
            assess(Some(old), &baseline, false, Some(pin)),
            FastForwardAssessment::UnavailablePin { pin }
        );
        assert_eq!(
            assess(Some(old), &baseline, false, None),
            FastForwardAssessment::Eligible
        );
    }

    #[test]
    fn fast_forward_requires_the_requested_current_newer_revision() {
        let baseline = CurriculumSemanticPayload::assignment(assignment(
            "Baseline",
            RelativeAssignmentSchedule::default(),
        ));
        let decision = assess_fast_forward(FastForwardAssessmentInput {
            imported_source: Some(source(1, 2)),
            requested_source: source(1, 2),
            current_source: source(1, 3),
            baseline: &baseline,
            current: &baseline,
            issued_work: false,
            unavailable_pin: None,
        });
        assert_eq!(
            decision,
            FastForwardAssessment::SourceRevisionDrift {
                source: source(1, 3)
            }
        );
    }

    #[test]
    fn fast_forward_projection_binds_choices_to_the_exact_unavailable_position() {
        let baseline = CurriculumSemanticPayload::assignment(assignment(
            "Baseline",
            RelativeAssignmentSchedule::default(),
        ));
        let pin = PositionedPin {
            position: CurriculumPinPosition::new(None, 0, 0, None).expect("position"),
            reference: reference(10),
        };
        let candidates =
            ReplacementQuestionChoices::new(vec!["7K3-M9QX".parse().expect("question ID")])
                .expect("bounded choices");
        let decision = project_fast_forward_decision(FastForwardProjectionInput {
            imported_source: Some(source(1, 1)),
            requested_source: source(1, 2),
            current_source: source(1, 2),
            baseline: &baseline,
            current: &baseline,
            issued_work: false,
            unavailable_pin: Some(pin),
            replacement_choices: Some(candidates.clone()),
        })
        .expect("public decision");
        assert_eq!(
            decision,
            AssignmentFastForwardDecision::UnavailablePin {
                recovery: UnavailablePinRecoveryAction::SelectReplacementQuestion {
                    position: pin.position,
                    candidates,
                },
            }
        );

        let error = project_fast_forward_decision(FastForwardProjectionInput {
            imported_source: Some(source(1, 1)),
            requested_source: source(1, 2),
            current_source: source(1, 2),
            baseline: &baseline,
            current: &baseline,
            issued_work: false,
            unavailable_pin: None,
            replacement_choices: ReplacementQuestionChoices::new(vec![
                "7K3-M9QX".parse().expect("question ID"),
            ])
            .ok(),
        })
        .expect_err("choices without an unavailable pin are invalid facts");
        assert!(matches!(error, SemanticPlannerError::InvalidReplacement(_)));
    }
}
