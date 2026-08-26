//! Ordered exact-pin traversal and adapter-resolved substitution.

use std::collections::{BTreeMap, BTreeSet};

use question_model::curriculum_adoption::CurriculumSemanticPayload;
use question_model::{CurriculumPinPosition, ProblemVersionRef};
use serde::{Deserialize, Serialize};

use super::semantic_payload_input;
use super::semantic_snapshot::{
    SemanticAssignmentEntryInputV1, SemanticAssignmentInputV1, SemanticPayloadInputV1,
    SemanticPlannerError, normalize_payload,
};

/// One exact semantic position and the immutable publication pin stored there.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct PositionedPin {
    pub(crate) position: CurriculumPinPosition,
    pub(crate) reference: ProblemVersionRef,
}

impl PositionedPin {
    #[cfg(any(test, feature = "test-support"))]
    pub(crate) const fn new(position: CurriculumPinPosition, reference: ProblemVersionRef) -> Self {
        Self {
            position,
            reference,
        }
    }

    pub(crate) const fn position(self) -> CurriculumPinPosition {
        self.position
    }

    #[cfg(any(test, feature = "test-support"))]
    pub(crate) const fn reference(self) -> ProblemVersionRef {
        self.reference
    }
}

/// One replacement already resolved to an exact pin by the owning adapter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ResolvedPinReplacement {
    pub(crate) position: CurriculumPinPosition,
    pub(crate) reference: ProblemVersionRef,
}

/// Returns the first authored pin rejected by adapter-owned destination authority.
#[cfg(any(test, feature = "test-support"))]
pub(crate) fn first_unavailable_pin(
    payload: &CurriculumSemanticPayload,
    mut authorized: impl FnMut(ProblemVersionRef) -> bool,
) -> Result<Option<PositionedPin>, SemanticPlannerError> {
    for pin in positioned_pins(payload)? {
        if !authorized(pin.reference()) {
            return Ok(Some(pin));
        }
    }
    Ok(None)
}

/// Applies exact resolved substitutions without changing any authored ordering.
pub(crate) fn substitute_resolved_pins(
    payload: &CurriculumSemanticPayload,
    replacements: &[ResolvedPinReplacement],
) -> Result<CurriculumSemanticPayload, SemanticPlannerError> {
    let mut replacements_by_position = BTreeMap::new();
    for replacement in replacements {
        if replacements_by_position
            .insert(replacement.position, replacement.reference)
            .is_some()
        {
            return Err(SemanticPlannerError::InvalidReplacement(format!(
                "position {:?} is repeated",
                replacement.position
            )));
        }
    }
    let mut applied = BTreeSet::new();
    let mut input = semantic_payload_input(payload);
    walk_input_pins(&mut input, |position, reference| {
        if let Some(replacement) = replacements_by_position.get(&position) {
            *reference = *replacement;
            applied.insert(position);
        }
    })?;
    if applied.len() != replacements_by_position.len() {
        let missing = replacements_by_position
            .keys()
            .find(|position| !applied.contains(position))
            .expect("unequal sets contain a missing position");
        return Err(SemanticPlannerError::InvalidReplacement(format!(
            "position {missing:?} does not identify a source pin"
        )));
    }
    normalize_payload(input)
}

#[cfg(any(test, feature = "test-support"))]
pub(crate) fn positioned_pins(
    payload: &CurriculumSemanticPayload,
) -> Result<Vec<PositionedPin>, SemanticPlannerError> {
    let mut input = semantic_payload_input(payload);
    let mut pins = Vec::new();
    walk_input_pins(&mut input, |position, reference| {
        pins.push(PositionedPin::new(position, *reference));
    })?;
    Ok(pins)
}

fn walk_input_pins(
    input: &mut SemanticPayloadInputV1,
    mut visit: impl FnMut(CurriculumPinPosition, &mut ProblemVersionRef),
) -> Result<(), SemanticPlannerError> {
    match input {
        SemanticPayloadInputV1::Assignment { definition } => {
            walk_assignment_pins(definition, None, 0, &mut visit)
        }
        SemanticPayloadInputV1::Course { modules, .. } => {
            for (module_index, module) in modules.iter_mut().enumerate() {
                let module_index = bounded_index(module_index, "module")?;
                for (assignment_index, assignment) in module.assignments.iter_mut().enumerate() {
                    walk_assignment_pins(
                        assignment,
                        Some(module_index),
                        bounded_index(assignment_index, "assignment")?,
                        &mut visit,
                    )?;
                }
            }
            Ok(())
        }
    }
}

fn walk_assignment_pins(
    assignment: &mut SemanticAssignmentInputV1,
    module_index: Option<u16>,
    assignment_index: u16,
    visit: &mut impl FnMut(CurriculumPinPosition, &mut ProblemVersionRef),
) -> Result<(), SemanticPlannerError> {
    for (entry_index, entry) in assignment.entries.iter_mut().enumerate() {
        let entry_index = bounded_index(entry_index, "entry")?;
        match entry {
            SemanticAssignmentEntryInputV1::Fixed { reference, .. } => {
                let position = position(module_index, assignment_index, entry_index, None)?;
                visit(position, reference);
            }
            SemanticAssignmentEntryInputV1::Pool { candidates, .. } => {
                for (candidate_index, reference) in candidates.iter_mut().enumerate() {
                    let position = position(
                        module_index,
                        assignment_index,
                        entry_index,
                        Some(bounded_index(candidate_index, "pool candidate")?),
                    )?;
                    visit(position, reference);
                }
            }
        }
    }
    Ok(())
}

fn bounded_index(index: usize, label: &str) -> Result<u16, SemanticPlannerError> {
    u16::try_from(index)
        .map_err(|_| SemanticPlannerError::InvalidPosition(format!("{label} index exceeds u16")))
}

fn position(
    module_index: Option<u16>,
    assignment_index: u16,
    entry_index: u16,
    candidate_index: Option<u16>,
) -> Result<CurriculumPinPosition, SemanticPlannerError> {
    CurriculumPinPosition::new(module_index, assignment_index, entry_index, candidate_index)
        .map_err(|error| SemanticPlannerError::InvalidPosition(error.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use question_model::curriculum_adoption::{
        CurriculumSemanticAssignment, CurriculumSemanticAssignmentEntry, CurriculumSemanticCourse,
        CurriculumSemanticModule, CurriculumSemanticPool,
    };
    use question_model::{
        AssignmentDeadlineBehavior, AssignmentInstructions, AssignmentScoringMode,
        CompletionRequirement, ContinuedPractice, GradePolicy, LateSubmissionPolicy,
        LearnerDisclosurePolicy, PointValue, ProblemId, RelativeAssignmentSchedule,
        ReusableAssignmentDefaults, RunPolicies, VariationPolicy, VersionId,
    };

    fn reference(value: u128) -> ProblemVersionRef {
        ProblemVersionRef {
            problem: ProblemId::from_uuid(uuid::Uuid::from_u128(value)),
            version: VersionId::from_uuid(uuid::Uuid::from_u128(value + 1)),
        }
    }

    fn payload() -> CurriculumSemanticPayload {
        CurriculumSemanticPayload::assignment(
            CurriculumSemanticAssignment::new(
                "Ordered".into(),
                AssignmentInstructions::default(),
                vec![
                    CurriculumSemanticAssignmentEntry::Fixed {
                        reference: reference(10),
                        points_possible: PointValue::from_whole(1),
                        scoring_mode: AssignmentScoringMode::Normal,
                    },
                    CurriculumSemanticAssignmentEntry::Fixed {
                        reference: reference(20),
                        points_possible: PointValue::from_whole(2),
                        scoring_mode: AssignmentScoringMode::ExtraCredit,
                    },
                ],
                defaults(),
                RelativeAssignmentSchedule::default(),
            )
            .expect("semantic assignment"),
        )
    }

    fn course_payload() -> CurriculumSemanticPayload {
        let assignment = |title: &str, entries| {
            CurriculumSemanticAssignment::new(
                title.into(),
                AssignmentInstructions::default(),
                entries,
                defaults(),
                RelativeAssignmentSchedule::default(),
            )
            .expect("semantic assignment")
        };
        let fixed = |value| CurriculumSemanticAssignmentEntry::Fixed {
            reference: reference(value),
            points_possible: PointValue::from_whole(1),
            scoring_mode: AssignmentScoringMode::Normal,
        };
        let pool = CurriculumSemanticAssignmentEntry::Pool(
            CurriculumSemanticPool::new(
                vec![reference(20), reference(30)],
                1,
                PointValue::from_whole(1),
                question_model::SelectionOrdering::CandidateOrder,
                question_model::PoolDrawAlgorithm::V1,
            )
            .expect("semantic pool"),
        );
        CurriculumSemanticPayload::course(
            CurriculumSemanticCourse::new(
                "Course".into(),
                vec![
                    CurriculumSemanticModule::new(
                        "Module A".into(),
                        vec![assignment("A0", vec![fixed(10), pool])],
                    )
                    .expect("module"),
                    CurriculumSemanticModule::new(
                        "Module B".into(),
                        vec![
                            assignment("B0", vec![fixed(40)]),
                            assignment("B1", vec![fixed(50)]),
                        ],
                    )
                    .expect("module"),
                ],
            )
            .expect("semantic course"),
        )
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
            learner_disclosure: LearnerDisclosurePolicy::default(),
        }
    }

    #[test]
    fn traversal_reports_the_first_unavailable_pin_in_authored_order() {
        let unavailable = first_unavailable_pin(&payload(), |pin| pin != reference(20))
            .expect("positions are valid")
            .expect("one pin is unavailable");
        assert_eq!(unavailable.reference, reference(20));
        assert_eq!(unavailable.position.entry_index(), 1);
    }

    #[test]
    fn resolved_substitution_preserves_order_and_requires_exact_positions() {
        let earlier = CurriculumPinPosition::new(None, 0, 0, None).expect("position");
        let later = CurriculumPinPosition::new(None, 0, 1, None).expect("position");
        let replaced = substitute_resolved_pins(
            &payload(),
            &[
                ResolvedPinReplacement {
                    position: later,
                    reference: reference(30),
                },
                ResolvedPinReplacement {
                    position: earlier,
                    reference: reference(40),
                },
            ],
        )
        .expect("resolved replacement applies");
        let CurriculumSemanticPayload::Assignment(replaced) = replaced else {
            panic!("assignment payload remains assignment-sized")
        };
        let references = replaced
            .entries()
            .iter()
            .map(|entry| match entry {
                CurriculumSemanticAssignmentEntry::Fixed { reference, .. } => *reference,
                CurriculumSemanticAssignmentEntry::Pool(_) => panic!("fixed fixture"),
            })
            .collect::<Vec<_>>();
        assert_eq!(references, vec![reference(40), reference(30)]);

        let missing = CurriculumPinPosition::new(None, 0, 3, None).expect("bounded position");
        assert!(
            substitute_resolved_pins(
                &payload(),
                &[ResolvedPinReplacement {
                    position: missing,
                    reference: reference(40),
                }],
            )
            .is_err()
        );
    }

    #[test]
    fn course_positions_and_substitution_preserve_exact_nested_coordinates() {
        let positions = [
            CurriculumPinPosition::new(Some(0), 0, 0, None).expect("fixed position"),
            CurriculumPinPosition::new(Some(0), 0, 1, Some(0)).expect("pool position"),
            CurriculumPinPosition::new(Some(0), 0, 1, Some(1)).expect("pool position"),
            CurriculumPinPosition::new(Some(1), 0, 0, None).expect("fixed position"),
            CurriculumPinPosition::new(Some(1), 1, 0, None).expect("fixed position"),
        ];
        let payload = course_payload();
        assert_eq!(
            positioned_pins(&payload).expect("positions"),
            positions
                .into_iter()
                .zip([10, 20, 30, 40, 50])
                .map(|(position, value)| PositionedPin {
                    position,
                    reference: reference(value),
                })
                .collect::<Vec<_>>()
        );

        let substituted = substitute_resolved_pins(
            &payload,
            &[
                ResolvedPinReplacement {
                    position: positions[2],
                    reference: reference(31),
                },
                ResolvedPinReplacement {
                    position: positions[4],
                    reference: reference(51),
                },
            ],
        )
        .expect("exact nested substitutions");
        assert_eq!(
            positioned_pins(&substituted)
                .expect("positions")
                .into_iter()
                .map(|pin| (pin.position, pin.reference))
                .collect::<Vec<_>>(),
            positions
                .into_iter()
                .zip([10, 20, 31, 40, 51])
                .map(|(position, value)| (position, reference(value)))
                .collect::<Vec<_>>()
        );
    }
}
