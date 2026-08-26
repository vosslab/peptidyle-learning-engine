//! Backend-neutral public preview projection from validated reusable meaning.

use question_model::curriculum_adoption::{CurriculumSemanticAssignment, CurriculumSemanticCourse};
use question_model::{
    CourseTerm, CurriculumAdoptionTitle, CurriculumScheduleCorrection,
    PreparedCurriculumAssignmentView, PreparedCurriculumCourseView, RelativeAssignmentSchedule,
};

use super::semantic_snapshot::SemanticPlannerError;

/// Resolves one validated reusable assignment for a target term.
///
/// Invalid local schedule moments are projected as typed browser-safe corrections,
/// with an empty resolved schedule standing in until the instructor corrects them.
pub(crate) fn preview_assignment(
    semantic: &CurriculumSemanticAssignment,
    term: &CourseTerm,
) -> Result<
    (
        PreparedCurriculumAssignmentView,
        Vec<CurriculumScheduleCorrection>,
    ),
    SemanticPlannerError,
> {
    let title = CurriculumAdoptionTitle::parse(semantic.title())
        .map_err(|error| SemanticPlannerError::InvalidMeaning(error.to_string()))?;
    match semantic.schedule().resolve_for_target_term(term) {
        Ok(schedule) => Ok((
            PreparedCurriculumAssignmentView { title, schedule },
            Vec::new(),
        )),
        Err(error) => Ok((
            PreparedCurriculumAssignmentView {
                title,
                schedule: RelativeAssignmentSchedule::default()
                    .resolve_for_target_term(term)
                    .expect("empty schedule resolves for every valid term"),
            },
            vec![error.into()],
        )),
    }
}

/// Preserves authored module and assignment order in the flat public course preview.
pub(crate) fn preview_course(
    title: &CurriculumAdoptionTitle,
    semantic: &CurriculumSemanticCourse,
    term: &CourseTerm,
) -> Result<
    (
        PreparedCurriculumCourseView,
        Vec<CurriculumScheduleCorrection>,
    ),
    SemanticPlannerError,
> {
    let mut assignments = Vec::new();
    let mut corrections = Vec::new();
    for assignment in semantic
        .modules()
        .iter()
        .flat_map(|module| module.assignments())
    {
        let (view, mut assignment_corrections) = preview_assignment(assignment, term)?;
        assignments.push(view);
        corrections.append(&mut assignment_corrections);
    }
    Ok((
        PreparedCurriculumCourseView {
            title: title.clone(),
            assignments,
        },
        corrections,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use question_model::curriculum_adoption::{
        CurriculumSemanticAssignmentEntry, CurriculumSemanticModule,
    };
    use question_model::{
        AssignmentDeadlineBehavior, AssignmentInstructions, AssignmentScoringMode,
        AssignmentTeachingSettingsFailureReason, AssignmentTeachingSettingsField,
        CompletionRequirement, ContinuedPractice, GradePolicy, LateSubmissionPolicy,
        LearnerDisclosurePolicy, LocalTimeOfDay, PointValue, ProblemId, ProblemVersionRef,
        RelativeScheduleMoment, ReusableAssignmentDefaults, RunPolicies, VariationPolicy,
        VersionId,
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

    fn assignment(
        title: &str,
        schedule: RelativeAssignmentSchedule,
        reference_value: u128,
    ) -> CurriculumSemanticAssignment {
        CurriculumSemanticAssignment::new(
            title.into(),
            AssignmentInstructions::default(),
            vec![CurriculumSemanticAssignmentEntry::Fixed {
                reference: reference(reference_value),
                points_possible: PointValue::from_whole(1),
                scoring_mode: AssignmentScoringMode::Normal,
            }],
            defaults(),
            schedule,
        )
        .expect("semantic assignment")
    }

    #[test]
    fn eligible_assignment_projects_qmodel_resolved_schedule() {
        let term =
            CourseTerm::from_parts("2026-08-24", "2026-12-18", "America/Chicago").expect("term");
        let schedule = RelativeAssignmentSchedule {
            available_at: Some(RelativeScheduleMoment {
                day_offset: 1,
                local_time: LocalTimeOfDay::parse("09:00:00.000").expect("time"),
            }),
            due_at: None,
            closes_at: None,
        };
        let semantic = assignment("Quiz", schedule.clone(), 10);

        let (view, corrections) = preview_assignment(&semantic, &term).expect("preview");

        assert_eq!(view.title.as_str(), "Quiz");
        assert_eq!(
            view.schedule,
            schedule.resolve_for_target_term(&term).expect("schedule")
        );
        assert!(corrections.is_empty());
    }

    #[test]
    fn course_preserves_order_and_projects_typed_dst_correction() {
        let term =
            CourseTerm::from_parts("2026-03-08", "2026-11-01", "America/Chicago").expect("term");
        let gap = RelativeAssignmentSchedule {
            available_at: Some(RelativeScheduleMoment {
                day_offset: 0,
                local_time: LocalTimeOfDay::parse("02:30:00.000").expect("gap time"),
            }),
            due_at: None,
            closes_at: None,
        };
        let semantic = CurriculumSemanticCourse::new(
            "Source".into(),
            vec![
                CurriculumSemanticModule::new(
                    "Module A".into(),
                    vec![assignment(
                        "Eligible",
                        RelativeAssignmentSchedule::default(),
                        10,
                    )],
                )
                .expect("module A"),
                CurriculumSemanticModule::new("Module B".into(), vec![assignment("Gap", gap, 20)])
                    .expect("module B"),
            ],
        )
        .expect("semantic course");
        let target_title = CurriculumAdoptionTitle::parse("Adopted course").expect("title");

        let (view, corrections) = preview_course(&target_title, &semantic, &term).expect("preview");

        assert_eq!(view.title, target_title);
        assert_eq!(
            view.assignments
                .iter()
                .map(|assignment| assignment.title.as_str())
                .collect::<Vec<_>>(),
            ["Eligible", "Gap"]
        );
        assert_eq!(corrections.len(), 1);
        assert_eq!(
            corrections[0].correction.field,
            AssignmentTeachingSettingsField::AvailableAt
        );
        assert_eq!(
            corrections[0].correction.reason,
            AssignmentTeachingSettingsFailureReason::NonexistentLocalTime
        );
    }
}
