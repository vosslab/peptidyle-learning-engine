//! Backend-neutral schedule preparation from validated reusable meaning.

use question_model::curriculum_adoption::{CurriculumSemanticAssignment, CurriculumSemanticCourse};
use question_model::{
    AssignmentTeachingSettingsFailureReason, AssignmentTeachingSettingsField,
    CourseInstanceScheduleCorrection, CourseInstanceScheduleField, CourseInstanceScheduleReason,
    CourseTerm, RelativeAssignmentSchedule, ResolvedRelativeAssignmentSchedule,
};

use super::semantic_snapshot::SemanticPlannerError;

/// Resolves one reusable assignment's schedule for a target CourseInstance term.
///
/// Invalid local moments remain a typed correction and use an empty resolved schedule until a
/// later source operation revalidates an explicitly corrected request.
pub(crate) fn preview_assignment(
    semantic: &CurriculumSemanticAssignment,
    term: &CourseTerm,
) -> Result<
    (
        ResolvedRelativeAssignmentSchedule,
        Vec<CourseInstanceScheduleCorrection>,
    ),
    SemanticPlannerError,
> {
    match semantic.schedule().resolve_for_target_term(term) {
        Ok(schedule) => Ok((schedule, Vec::new())),
        Err(error) => Ok((
            RelativeAssignmentSchedule::default()
                .resolve_for_target_term(term)
                .expect("empty schedule resolves for every valid term"),
            vec![CourseInstanceScheduleCorrection {
                field: schedule_field(error.field()),
                reason: schedule_reason(error.reason()),
            }],
        )),
    }
}

/// Preserves authored module and assignment order while preparing target-term schedules.
pub(crate) fn preview_course(
    semantic: &CurriculumSemanticCourse,
    term: &CourseTerm,
) -> Result<
    (
        Vec<ResolvedRelativeAssignmentSchedule>,
        Vec<CourseInstanceScheduleCorrection>,
    ),
    SemanticPlannerError,
> {
    let mut schedules = Vec::new();
    let mut corrections = Vec::new();
    for assignment in semantic
        .modules()
        .iter()
        .flat_map(|module| module.assignments())
    {
        let (schedule, mut assignment_corrections) = preview_assignment(assignment, term)?;
        schedules.push(schedule);
        corrections.append(&mut assignment_corrections);
    }
    Ok((schedules, corrections))
}

fn schedule_field(field: AssignmentTeachingSettingsField) -> CourseInstanceScheduleField {
    match field {
        AssignmentTeachingSettingsField::AvailableAt => CourseInstanceScheduleField::AvailableAt,
        AssignmentTeachingSettingsField::DueAt => CourseInstanceScheduleField::DueAt,
        AssignmentTeachingSettingsField::ClosesAt => CourseInstanceScheduleField::ClosesAt,
        AssignmentTeachingSettingsField::Schedule => CourseInstanceScheduleField::Schedule,
        AssignmentTeachingSettingsField::TeachingSettings
        | AssignmentTeachingSettingsField::TimeZone
        | AssignmentTeachingSettingsField::TimeLimitSeconds
        | AssignmentTeachingSettingsField::AttemptLimit
        | AssignmentTeachingSettingsField::Lifecycle
        | AssignmentTeachingSettingsField::Instructions => CourseInstanceScheduleField::TargetTerm,
    }
}

fn schedule_reason(
    reason: AssignmentTeachingSettingsFailureReason,
) -> CourseInstanceScheduleReason {
    match reason {
        AssignmentTeachingSettingsFailureReason::OutsideCourseTerm => {
            CourseInstanceScheduleReason::OutsideTargetTerm
        }
        AssignmentTeachingSettingsFailureReason::NonexistentLocalTime => {
            CourseInstanceScheduleReason::NonexistentLocalTime
        }
        AssignmentTeachingSettingsFailureReason::AmbiguousLocalTime => {
            CourseInstanceScheduleReason::AmbiguousLocalTime
        }
        AssignmentTeachingSettingsFailureReason::ScheduleOutOfOrder => {
            CourseInstanceScheduleReason::OutOfOrder
        }
        AssignmentTeachingSettingsFailureReason::TimestampOutOfRange => {
            CourseInstanceScheduleReason::TimestampOutOfRange
        }
        AssignmentTeachingSettingsFailureReason::InvalidInput
        | AssignmentTeachingSettingsFailureReason::CourseTimeZoneMismatch
        | AssignmentTeachingSettingsFailureReason::TimeLimitOutOfRange
        | AssignmentTeachingSettingsFailureReason::AttemptLimitOutOfRange
        | AssignmentTeachingSettingsFailureReason::IllegalLifecycleTransition
        | AssignmentTeachingSettingsFailureReason::InvalidInstructions => {
            CourseInstanceScheduleReason::OutOfOrder
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use question_model::curriculum_adoption::CurriculumSemanticAssignmentEntry;
    use question_model::{
        AssignmentDeadlineBehavior, AssignmentInstructions, AssignmentScoringMode,
        CompletionRequirement, ContinuedPractice, GradePolicy, LateSubmissionPolicy,
        LocalTimeOfDay, PointValue, ProblemId, ProblemVersionRef, RelativeScheduleMoment,
        ReusableAssignmentDefaults, RunPolicies, StudentDisclosurePolicy, VariationPolicy,
        VersionId,
    };

    #[test]
    fn preview_returns_current_dst_correction() {
        let assignment = CurriculumSemanticAssignment::new(
            "Quiz".into(),
            AssignmentInstructions::default(),
            vec![CurriculumSemanticAssignmentEntry::Fixed {
                reference: ProblemVersionRef {
                    problem: ProblemId::from_uuid(uuid::Uuid::from_u128(1)),
                    version: VersionId::from_uuid(uuid::Uuid::from_u128(2)),
                },
                points_possible: PointValue::from_whole(1),
                scoring_mode: AssignmentScoringMode::Normal,
            }],
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
            },
            RelativeAssignmentSchedule {
                available_at: Some(RelativeScheduleMoment {
                    day_offset: 0,
                    local_time: LocalTimeOfDay::parse("02:30:00.000").expect("time"),
                }),
                due_at: None,
                closes_at: None,
            },
        )
        .expect("assignment");
        let term =
            CourseTerm::from_parts("2026-03-08", "2026-03-08", "America/Chicago").expect("term");

        let (_, corrections) = preview_assignment(&assignment, &term).expect("preview");

        assert_eq!(corrections.len(), 1);
        assert_eq!(
            corrections[0].field,
            CourseInstanceScheduleField::AvailableAt
        );
        assert_eq!(
            corrections[0].reason,
            CourseInstanceScheduleReason::NonexistentLocalTime
        );
    }
}
