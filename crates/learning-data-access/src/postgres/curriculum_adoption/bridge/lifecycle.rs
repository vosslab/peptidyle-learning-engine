//! Relational-fact projection for rollover and whole-course term shifting.

use std::collections::BTreeSet;

use question_model::curriculum_adoption::CurriculumSemanticPayload;
use question_model::{
    CourseRolloverCommand, CourseRolloverPreviewRequest, CourseRolloverPreviewView,
    CourseScheduleWitness, CourseTerm, CourseTermShiftCommand, CourseTermShiftIneligibility,
    CourseTermShiftPreviewOutcome, CourseTermShiftPreviewRequest, CourseTermShiftPreviewView,
    CourseTermShiftRecoveryAction, CurriculumAssignmentView, CurriculumScheduleCorrection,
    PreparedCurriculumCourseView, ResolvedRelativeAssignmentSchedule, UnavailablePinRecoveryAction,
};
use serde::Serialize;

use crate::StoreError;
use crate::curriculum_adoption::{preview_assignment, preview_course};

use super::{
    LifecycleFactsV1, OrderedRolloverSourceV1, OrderedTermShiftAssignmentV1, PinAvailabilityV1,
    PreparedCourseAssignmentV1, PreparedSemanticV1, TermShiftEligibilityV1,
    prepare_course_assignments, prepare_lifecycle_semantic, validate_prepared_course_assignments,
};

/// Exact rollover plan; SQL rechecks its source witness before minting IDs.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(in crate::postgres::curriculum_adoption) struct PreparedCourseRolloverPlanV1 {
    pub(super) semantic: PreparedSemanticV1,
    pub(super) source_witness: CourseScheduleWitness,
    pub(super) target_term: CourseTerm,
    pub(super) preview: PreparedCurriculumCourseView,
    pub(super) corrections: Vec<CurriculumScheduleCorrection>,
    pub(super) assignments: Vec<PreparedCourseAssignmentV1>,
    pub(super) rollover_sources: Vec<OrderedRolloverSourceV1>,
}

/// One existing assignment whose revision and resolved schedule SQL rechecks.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct PreparedTermShiftRowV1 {
    pub(super) assignment: question_model::AssignmentReference,
    pub(super) expected_revision: question_model::AssignmentRevision,
    pub(super) schedule: ResolvedRelativeAssignmentSchedule,
}

/// Schedule-only plan for one eligible teaching course.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(in crate::postgres::curriculum_adoption) struct PreparedCourseTermShiftPlanV1 {
    pub(super) semantic: PreparedSemanticV1,
    pub(super) course_witness: CourseScheduleWitness,
    pub(super) target_term: CourseTerm,
    pub(super) rows: Vec<PreparedTermShiftRowV1>,
}

/// Produces the answer-free rollover preview from locked relational facts.
pub(in crate::postgres::curriculum_adoption) fn project_rollover(
    request: &CourseRolloverPreviewRequest,
    facts: &LifecycleFactsV1,
) -> Result<CourseRolloverPreviewView, StoreError> {
    let (target_term, witness, title) = rollover_requirements(facts)?;
    require_rollover_preview_binding(
        request,
        &witness,
        &title,
        &target_term,
        &facts.requested_replacements,
    )?;
    let (payload, _) = lifecycle_semantic(facts)?;
    let course = course_payload(payload, "rollover")?;
    validate_rollover_sources(&course, &facts.ordered_rollover_sources)?;
    let (course, corrections) = preview_course(&title, &course, &target_term).map_err(invalid)?;
    Ok(CourseRolloverPreviewView {
        witness,
        target_term,
        course,
        replacements: facts.requested_replacements.clone(),
        corrections,
        pin_correction: lifecycle_pin_correction(facts),
    })
}

/// Builds the exact rollover materialization plan after the same normalization.
pub(in crate::postgres::curriculum_adoption) fn prepare_rollover(
    command: &CourseRolloverCommand,
    facts: &LifecycleFactsV1,
) -> Result<PreparedCourseRolloverPlanV1, StoreError> {
    if !matches!(facts.pin_availability, PinAvailabilityV1::Available) {
        return Err(StoreError::Conflict);
    }
    let (target_term, source_witness, title) = rollover_requirements(facts)?;
    require_rollover_command_binding(
        command,
        &source_witness,
        &title,
        &target_term,
        &facts.requested_replacements,
    )?;
    let (payload, semantic) = lifecycle_semantic(facts)?;
    let course = course_payload(payload, "rollover")?;
    validate_rollover_sources(&course, &facts.ordered_rollover_sources)?;
    let (preview, corrections) = preview_course(&title, &course, &target_term).map_err(invalid)?;
    let assignments = prepare_course_assignments(&course, &target_term)?;
    validate_rollover_assignment_rows(
        &course,
        &target_term,
        &assignments,
        &facts.ordered_rollover_sources,
    )?;
    Ok(PreparedCourseRolloverPlanV1 {
        semantic,
        source_witness,
        target_term,
        preview,
        corrections,
        assignments,
        rollover_sources: facts.ordered_rollover_sources.clone(),
    })
}

/// Returns the typed issued-work recovery or the eligible schedule projection.
pub(in crate::postgres::curriculum_adoption) fn project_term_shift(
    request: &CourseTermShiftPreviewRequest,
    facts: &LifecycleFactsV1,
) -> Result<CourseTermShiftPreviewOutcome, StoreError> {
    let witness = required_witness(facts, "term shift")?;
    let target_term = required_target_term(facts, "term shift")?;
    require_term_shift_preview_binding(request, &witness, &target_term)?;
    let TermShiftEligibilityV1::Eligible {
        ordered_assignments,
    } = &facts.term_shift_eligibility
    else {
        return Ok(CourseTermShiftPreviewOutcome::Ineligible {
            course: witness.course,
            reason: CourseTermShiftIneligibility::IssuedWork,
            recovery: CourseTermShiftRecoveryAction::RolloverCourse,
        });
    };
    let (payload, _) = lifecycle_semantic(facts)?;
    let course = course_payload(payload, "term shift")?;
    let (assignments, corrections) =
        project_term_shift_assignments(&course, ordered_assignments, &witness, &target_term)?;
    Ok(CourseTermShiftPreviewOutcome::Eligible {
        preview: CourseTermShiftPreviewView {
            witness,
            target_term,
            assignments,
            corrections,
        },
    })
}

/// Builds a schedule-only plan. Issued work and uncorrected schedules fence apply.
pub(in crate::postgres::curriculum_adoption) fn prepare_term_shift(
    command: &CourseTermShiftCommand,
    facts: &LifecycleFactsV1,
) -> Result<PreparedCourseTermShiftPlanV1, StoreError> {
    let witness = required_witness(facts, "term shift")?;
    let target_term = required_target_term(facts, "term shift")?;
    require_term_shift_command_binding(command, &witness, &target_term)?;
    let TermShiftEligibilityV1::Eligible {
        ordered_assignments,
    } = &facts.term_shift_eligibility
    else {
        return Err(StoreError::Conflict);
    };
    let (payload, semantic) = lifecycle_semantic(facts)?;
    let course = course_payload(payload, "term shift")?;
    let (assignments, corrections) =
        project_term_shift_assignments(&course, ordered_assignments, &witness, &target_term)?;
    if !corrections.is_empty() {
        return Err(StoreError::Conflict);
    }
    let rows = assignments
        .into_iter()
        .map(|assignment| PreparedTermShiftRowV1 {
            assignment: assignment.reference,
            expected_revision: assignment.revision,
            schedule: assignment.schedule,
        })
        .collect();
    Ok(PreparedCourseTermShiftPlanV1 {
        semantic,
        course_witness: witness,
        target_term,
        rows,
    })
}

fn rollover_requirements(
    facts: &LifecycleFactsV1,
) -> Result<
    (
        CourseTerm,
        CourseScheduleWitness,
        question_model::CurriculumAdoptionTitle,
    ),
    StoreError,
> {
    if !matches!(
        facts.term_shift_eligibility,
        TermShiftEligibilityV1::IssuedWork
    ) {
        return Err(unavailable("rollover carries term-shift facts"));
    }
    let title = facts
        .resulting_title
        .clone()
        .ok_or_else(|| unavailable("rollover is missing its resulting title"))?;
    Ok((
        required_target_term(facts, "rollover")?,
        required_witness(facts, "rollover")?,
        title,
    ))
}

fn required_target_term(
    facts: &LifecycleFactsV1,
    operation: &str,
) -> Result<CourseTerm, StoreError> {
    facts
        .target_term
        .clone()
        .ok_or_else(|| unavailable(&format!("{operation} is missing its target term")))
}

fn require_rollover_preview_binding(
    request: &CourseRolloverPreviewRequest,
    witness: &CourseScheduleWitness,
    title: &question_model::CurriculumAdoptionTitle,
    target_term: &CourseTerm,
    replacements: &question_model::CurriculumPinReplacements,
) -> Result<(), StoreError> {
    if request.witness != *witness
        || request.title != *title
        || request.target_term != *target_term
        || request.replacements != *replacements
    {
        return Err(StoreError::Conflict);
    }
    Ok(())
}

fn require_rollover_command_binding(
    command: &CourseRolloverCommand,
    witness: &CourseScheduleWitness,
    title: &question_model::CurriculumAdoptionTitle,
    target_term: &CourseTerm,
    replacements: &question_model::CurriculumPinReplacements,
) -> Result<(), StoreError> {
    if command.preview_witness() != witness
        || command.title() != title
        || command.target_term() != target_term
        || command.replacements() != replacements
    {
        return Err(StoreError::Conflict);
    }
    Ok(())
}

fn require_term_shift_preview_binding(
    request: &CourseTermShiftPreviewRequest,
    witness: &CourseScheduleWitness,
    target_term: &CourseTerm,
) -> Result<(), StoreError> {
    if request.witness != *witness || request.target_term != *target_term {
        return Err(StoreError::Conflict);
    }
    Ok(())
}

fn require_term_shift_command_binding(
    command: &CourseTermShiftCommand,
    witness: &CourseScheduleWitness,
    target_term: &CourseTerm,
) -> Result<(), StoreError> {
    if command.preview_witness() != witness || command.target_term() != target_term {
        return Err(StoreError::Conflict);
    }
    Ok(())
}

fn required_witness(
    facts: &LifecycleFactsV1,
    operation: &str,
) -> Result<CourseScheduleWitness, StoreError> {
    facts
        .witness
        .clone()
        .ok_or_else(|| unavailable(&format!("{operation} is missing its course witness")))
}

fn course_payload(
    payload: CurriculumSemanticPayload,
    operation: &str,
) -> Result<question_model::curriculum_adoption::CurriculumSemanticCourse, StoreError> {
    let CurriculumSemanticPayload::Course(course) = payload else {
        return Err(unavailable(&format!(
            "{operation} has non-course semantic meaning"
        )));
    };
    Ok(course)
}

fn lifecycle_semantic(
    facts: &LifecycleFactsV1,
) -> Result<(CurriculumSemanticPayload, PreparedSemanticV1), StoreError> {
    prepare_lifecycle_semantic(
        &facts.source_title,
        &facts.source_term,
        &facts.modules,
        &facts.resolved_replacements,
    )
}

fn validate_rollover_sources(
    course: &question_model::curriculum_adoption::CurriculumSemanticCourse,
    sources: &[OrderedRolloverSourceV1],
) -> Result<(), StoreError> {
    let positions = course
        .modules()
        .iter()
        .enumerate()
        .flat_map(|(module_position, module)| {
            module
                .assignments()
                .iter()
                .enumerate()
                .map(move |(assignment_position, _)| (module_position, assignment_position))
        })
        .collect::<Vec<_>>();
    if positions.len() != sources.len() {
        return Err(unavailable(
            "rollover source assignment witness count disagrees",
        ));
    }
    for ((module_position, assignment_position), source) in positions.iter().zip(sources) {
        let module_position = u16::try_from(*module_position)
            .map_err(|_| unavailable("rollover module position exceeds the contract bound"))?;
        let assignment_position = u16::try_from(*assignment_position)
            .map_err(|_| unavailable("rollover assignment position exceeds the contract bound"))?;
        if source.module_position != module_position
            || source.assignment_position != assignment_position
        {
            return Err(unavailable("rollover source assignment witness disagrees"));
        }
    }
    Ok(())
}

fn validate_rollover_assignment_rows(
    course: &question_model::curriculum_adoption::CurriculumSemanticCourse,
    target_term: &CourseTerm,
    assignments: &[PreparedCourseAssignmentV1],
    sources: &[OrderedRolloverSourceV1],
) -> Result<(), StoreError> {
    validate_prepared_course_assignments(course, target_term, assignments)?;
    if assignments.len() != sources.len() {
        return Err(unavailable(
            "rollover assignment evidence count disagrees with source witnesses",
        ));
    }
    for (assignment, source) in assignments.iter().zip(sources) {
        if assignment.module_position != source.module_position
            || assignment.assignment_position != source.assignment_position
        {
            return Err(unavailable(
                "rollover assignment evidence position disagrees with source witness",
            ));
        }
    }
    Ok(())
}

fn project_term_shift_assignments(
    course: &question_model::curriculum_adoption::CurriculumSemanticCourse,
    ordered: &[OrderedTermShiftAssignmentV1],
    witness: &CourseScheduleWitness,
    target_term: &CourseTerm,
) -> Result<
    (
        Vec<CurriculumAssignmentView>,
        Vec<CurriculumScheduleCorrection>,
    ),
    StoreError,
> {
    let semantic =
        course
            .modules()
            .iter()
            .enumerate()
            .flat_map(|(module_position, module)| {
                module.assignments().iter().enumerate().map(
                    move |(assignment_position, assignment)| {
                        (module_position, assignment_position, assignment)
                    },
                )
            })
            .collect::<Vec<_>>();
    if semantic.len() != ordered.len() || witness.assignment_revisions().len() != ordered.len() {
        return Err(unavailable("term-shift assignment witness count disagrees"));
    }
    let mut seen = BTreeSet::new();
    let mut assignments = Vec::with_capacity(ordered.len());
    let mut corrections = Vec::new();
    for ((module_position, assignment_position, semantic), observed) in semantic.iter().zip(ordered)
    {
        let module_position = u16::try_from(*module_position)
            .map_err(|_| unavailable("term-shift module position exceeds the contract bound"))?;
        let assignment_position = u16::try_from(*assignment_position).map_err(|_| {
            unavailable("term-shift assignment position exceeds the contract bound")
        })?;
        let expected = question_model::ObservedAssignmentRevision {
            assignment: observed.assignment,
            revision: observed.expected_revision,
        };
        if observed.module_position != module_position
            || observed.assignment_position != assignment_position
            || !seen.insert(observed.assignment)
            || !witness.contains_assignment(expected)
        {
            return Err(unavailable("term-shift assignment witness disagrees"));
        }
        let (prepared, mut row_corrections) =
            preview_assignment(semantic, target_term).map_err(invalid)?;
        assignments.push(CurriculumAssignmentView {
            reference: observed.assignment,
            title: prepared.title,
            revision: observed.expected_revision,
            schedule: prepared.schedule,
        });
        corrections.append(&mut row_corrections);
    }
    Ok((assignments, corrections))
}

fn lifecycle_pin_correction(facts: &LifecycleFactsV1) -> Option<UnavailablePinRecoveryAction> {
    match &facts.pin_availability {
        PinAvailabilityV1::Available => None,
        PinAvailabilityV1::Unavailable { pin, candidates } => {
            Some(UnavailablePinRecoveryAction::SelectReplacementQuestion {
                position: pin.position(),
                candidates: candidates.clone(),
            })
        }
    }
}

fn invalid(error: impl std::fmt::Display) -> StoreError {
    unavailable(&format!("curriculum lifecycle facts are invalid: {error}"))
}

fn unavailable(message: &str) -> StoreError {
    StoreError::Unavailable(message.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use question_model::curriculum_adoption::{
        CurriculumSemanticAssignment, CurriculumSemanticAssignmentEntry, CurriculumSemanticCourse,
        CurriculumSemanticModule,
    };
    use question_model::{
        AssignmentDeadlineBehavior, AssignmentInstructions, AssignmentScoringMode,
        CompletionRequirement, ContinuedPractice, CourseScheduleRevision, GradePolicy,
        LateSubmissionPolicy, LearnerDisclosurePolicy, ObservedAssignmentRevision, PointValue,
        ProblemId, ProblemVersionRef, RelativeAssignmentSchedule, ReusableAssignmentDefaults,
        RunPolicies, VariationPolicy, VersionId,
    };

    fn reference(value: u128) -> ProblemVersionRef {
        ProblemVersionRef {
            problem: ProblemId::from_uuid(uuid::Uuid::from_u128(value)),
            version: VersionId::from_uuid(uuid::Uuid::from_u128(value + 1)),
        }
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

    fn assignment(title: &str, pin: u128) -> CurriculumSemanticAssignment {
        CurriculumSemanticAssignment::new(
            title.into(),
            AssignmentInstructions::default(),
            vec![CurriculumSemanticAssignmentEntry::Fixed {
                reference: reference(pin),
                points_possible: PointValue::from_whole(1),
                scoring_mode: AssignmentScoringMode::Normal,
            }],
            defaults(),
            RelativeAssignmentSchedule::default(),
        )
        .expect("semantic assignment")
    }

    fn course() -> CurriculumSemanticCourse {
        CurriculumSemanticCourse::new(
            "Source".into(),
            vec![
                CurriculumSemanticModule::new("First".into(), vec![assignment("A", 10)])
                    .expect("first module"),
                CurriculumSemanticModule::new("Second".into(), vec![assignment("B", 20)])
                    .expect("second module"),
            ],
        )
        .expect("semantic course")
    }

    #[test]
    fn rollover_witnesses_follow_authored_module_and_assignment_order() {
        let course = course();
        let sources = vec![
            OrderedRolloverSourceV1 {
                module_position: 0,
                assignment_position: 0,
                source_assignment_id: uuid::Uuid::from_u128(1),
                source_assignment_revision: question_model::AssignmentRevision::INITIAL,
            },
            OrderedRolloverSourceV1 {
                module_position: 1,
                assignment_position: 0,
                source_assignment_id: uuid::Uuid::from_u128(2),
                source_assignment_revision: question_model::AssignmentRevision::INITIAL,
            },
        ];

        assert!(validate_rollover_sources(&course, &sources).is_ok());
        let mut reversed = sources;
        reversed.reverse();
        assert!(validate_rollover_sources(&course, &reversed).is_err());
    }

    #[test]
    fn alpha_and_rollover_rows_keep_assignment_evidence_and_positions_distinct() {
        let course = course();
        let term =
            CourseTerm::from_parts("2027-01-11", "2027-05-07", "America/Chicago").expect("term");
        let rows = prepare_course_assignments(&course, &term).expect("prepared course rows");
        let sources = vec![
            OrderedRolloverSourceV1 {
                module_position: 0,
                assignment_position: 0,
                source_assignment_id: uuid::Uuid::from_u128(1),
                source_assignment_revision: question_model::AssignmentRevision::INITIAL,
            },
            OrderedRolloverSourceV1 {
                module_position: 1,
                assignment_position: 0,
                source_assignment_id: uuid::Uuid::from_u128(2),
                source_assignment_revision: question_model::AssignmentRevision::INITIAL,
            },
        ];

        assert_eq!(
            rows.iter()
                .map(|row| (row.module_position, row.assignment_position))
                .collect::<Vec<_>>(),
            vec![(0, 0), (1, 0)]
        );
        assert_ne!(
            rows[0].semantic.semantic_digest,
            rows[1].semantic.semantic_digest
        );
        assert!(validate_rollover_assignment_rows(&course, &term, &rows, &sources).is_ok());
        let serialized = serde_json::to_value(&rows).expect("course rows serialize");
        assert_eq!(
            serialized[0]
                .as_object()
                .expect("course row is an object")
                .keys()
                .cloned()
                .collect::<BTreeSet<_>>(),
            BTreeSet::from([
                "assignmentPosition".to_string(),
                "materialization".to_string(),
                "modulePosition".to_string(),
                "semantic".to_string(),
            ])
        );
    }

    #[test]
    fn term_shift_rows_bind_public_revisions_in_authored_order() {
        let course = course();
        let first = "A-10".parse().expect("assignment reference");
        let second = "A-20".parse().expect("assignment reference");
        let first_revision = question_model::AssignmentRevision::new(3).expect("revision");
        let second_revision = question_model::AssignmentRevision::new(4).expect("revision");
        let witness = CourseScheduleWitness::new(
            "C-9".parse().expect("course reference"),
            CourseScheduleRevision::INITIAL,
            vec![
                ObservedAssignmentRevision {
                    assignment: second,
                    revision: second_revision,
                },
                ObservedAssignmentRevision {
                    assignment: first,
                    revision: first_revision,
                },
            ],
        )
        .expect("witness");
        let term =
            CourseTerm::from_parts("2027-01-11", "2027-05-07", "America/Chicago").expect("term");
        let rows = vec![
            OrderedTermShiftAssignmentV1 {
                module_position: 0,
                assignment_position: 0,
                assignment: first,
                expected_revision: first_revision,
            },
            OrderedTermShiftAssignmentV1 {
                module_position: 1,
                assignment_position: 0,
                assignment: second,
                expected_revision: second_revision,
            },
        ];

        let (projection, corrections) =
            project_term_shift_assignments(&course, &rows, &witness, &term).expect("projection");

        assert!(corrections.is_empty());
        assert_eq!(
            projection
                .iter()
                .map(|row| row.reference)
                .collect::<Vec<_>>(),
            vec![first, second]
        );
        let mut duplicate = rows;
        duplicate[1].assignment = first;
        assert!(project_term_shift_assignments(&course, &duplicate, &witness, &term).is_err());
    }

    #[test]
    fn preview_and_command_bindings_refuse_a_different_locked_term() {
        let witness = CourseScheduleWitness::new(
            "C-9".parse().expect("course reference"),
            CourseScheduleRevision::INITIAL,
            Vec::new(),
        )
        .expect("witness");
        let term =
            CourseTerm::from_parts("2027-01-11", "2027-05-07", "America/Chicago").expect("term");
        let different_term = CourseTerm::from_parts("2027-08-23", "2027-12-17", "America/Chicago")
            .expect("different term");
        let title =
            question_model::CurriculumAdoptionTitle::parse("Next term").expect("course title");
        let replacements = question_model::CurriculumPinReplacements::default();
        let request = CourseRolloverPreviewRequest {
            witness: witness.clone(),
            title: title.clone(),
            target_term: term.clone(),
            replacements: replacements.clone(),
        };
        assert!(
            require_rollover_preview_binding(&request, &witness, &title, &term, &replacements)
                .is_ok()
        );
        assert!(
            require_rollover_preview_binding(
                &request,
                &witness,
                &title,
                &different_term,
                &replacements
            )
            .is_err()
        );

        let preview = CourseRolloverPreviewView {
            witness: witness.clone(),
            target_term: term.clone(),
            course: PreparedCurriculumCourseView {
                title: title.clone(),
                assignments: Vec::new(),
            },
            replacements: replacements.clone(),
            corrections: Vec::new(),
            pin_correction: None,
        };
        let command = CourseRolloverCommand::from_preview(
            &preview,
            question_model::CurriculumAdoptionIdempotencyKey::parse("binding-check")
                .expect("idempotency key"),
        )
        .expect("command");
        assert!(
            require_rollover_command_binding(&command, &witness, &title, &term, &replacements)
                .is_ok()
        );
        assert!(
            require_rollover_command_binding(
                &command,
                &witness,
                &title,
                &different_term,
                &replacements
            )
            .is_err()
        );
    }
}
