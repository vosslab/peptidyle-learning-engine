//! Closed exact-pin persistence input and qmodel-owned normalization.

use question_model::curriculum_adoption::{
    CurriculumSemanticAssignment, CurriculumSemanticAssignmentEntry, CurriculumSemanticCourse,
    CurriculumSemanticModule, CurriculumSemanticPayload, CurriculumSemanticPool,
};
use question_model::{
    AssignmentInstructions, AssignmentScoringMode, PointValue, PoolDrawAlgorithm,
    ProblemVersionRef, RelativeAssignmentSchedule, ReusableAssignmentDefaults, SelectionOrdering,
};
use serde::{Deserialize, Serialize};

#[cfg(any(test, feature = "postgres"))]
use question_model::{BaseAssignmentPolicy, CourseTerm};

/// Closed server-side representation of one semantic payload read from adapter storage.
///
/// Unlike B1 browser views, every question is the exact immutable publication
/// reference already resolved under adapter-owned authority.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub(crate) enum SemanticPayloadInputV1 {
    Assignment {
        definition: SemanticAssignmentInputV1,
    },
    Course {
        title: String,
        modules: Vec<SemanticModuleInputV1>,
    },
}

/// Closed exact-pin assignment meaning before qmodel validation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct SemanticAssignmentInputV1 {
    pub(crate) title: String,
    pub(crate) instructions: AssignmentInstructions,
    pub(crate) entries: Vec<SemanticAssignmentEntryInputV1>,
    pub(crate) defaults: ReusableAssignmentDefaults,
    pub(crate) schedule: RelativeAssignmentSchedule,
}

/// Closed teaching-state assignment input before calendar-relative projection.
///
/// The adapter supplies stored teaching policy and the source term separately;
/// qmodel remains the sole authority for projecting absolute teaching times into
/// reusable calendar-relative meaning.
#[cfg(any(test, feature = "postgres"))]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct TeachingAssignmentInputV1 {
    pub(crate) title: String,
    pub(crate) instructions: AssignmentInstructions,
    pub(crate) entries: Vec<SemanticAssignmentEntryInputV1>,
    pub(crate) defaults: ReusableAssignmentDefaults,
    pub(crate) base_policy: BaseAssignmentPolicy,
}

/// Closed module topology before qmodel validation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct SemanticModuleInputV1 {
    pub(crate) label: String,
    pub(crate) assignments: Vec<SemanticAssignmentInputV1>,
}

/// Closed ordered fixed-item or pool meaning before qmodel validation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub(crate) enum SemanticAssignmentEntryInputV1 {
    Fixed {
        reference: ProblemVersionRef,
        points_possible: PointValue,
        scoring_mode: AssignmentScoringMode,
    },
    Pool {
        candidates: Vec<ProblemVersionRef>,
        draw_count: u32,
        points_per_item: PointValue,
        ordering: SelectionOrdering,
        algorithm: PoolDrawAlgorithm,
    },
}

/// Closed exact-pin pool input used by adapter-local source readers.
#[cfg(any(test, feature = "test-support"))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct SemanticPoolInputV1 {
    pub(crate) candidates: Vec<ProblemVersionRef>,
    pub(crate) draw_count: u32,
    pub(crate) points_per_item: PointValue,
    pub(crate) ordering: SelectionOrdering,
    pub(crate) algorithm: PoolDrawAlgorithm,
}

#[cfg(any(test, feature = "test-support"))]
impl From<SemanticPoolInputV1> for SemanticAssignmentEntryInputV1 {
    fn from(pool: SemanticPoolInputV1) -> Self {
        Self::Pool {
            candidates: pool.candidates,
            draw_count: pool.draw_count,
            points_per_item: pool.points_per_item,
            ordering: pool.ordering,
            algorithm: pool.algorithm,
        }
    }
}

/// Descriptive refusal from the backend-neutral semantic planner.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SemanticPlannerError {
    InvalidMeaning(String),
    InvalidPosition(String),
    InvalidReplacement(String),
    #[cfg(feature = "postgres")]
    InvalidEvidence(SemanticEvidenceMismatch),
    #[cfg(feature = "postgres")]
    InvalidInspection(String),
    Schedule(String),
}

impl std::fmt::Display for SemanticPlannerError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidMeaning(reason) => {
                write!(
                    formatter,
                    "curriculum semantic payload is invalid: {reason}"
                )
            }
            Self::InvalidPosition(reason) => {
                write!(
                    formatter,
                    "curriculum semantic position is invalid: {reason}"
                )
            }
            Self::InvalidReplacement(reason) => {
                write!(formatter, "curriculum pin replacement is invalid: {reason}")
            }
            #[cfg(feature = "postgres")]
            Self::InvalidEvidence(mismatch) => {
                write!(
                    formatter,
                    "curriculum semantic evidence is invalid: {mismatch}"
                )
            }
            #[cfg(feature = "postgres")]
            Self::InvalidInspection(reason) => {
                write!(
                    formatter,
                    "curriculum import inspection is invalid: {reason}"
                )
            }
            Self::Schedule(reason) => {
                write!(formatter, "curriculum target schedule is invalid: {reason}")
            }
        }
    }
}

impl std::error::Error for SemanticPlannerError {}

/// Exact canonical envelope field that disagreed with normalized meaning.
#[cfg(feature = "postgres")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SemanticEvidenceMismatch {
    CanonicalVersion,
    CanonicalBytes,
    Digest,
}

#[cfg(feature = "postgres")]
impl std::fmt::Display for SemanticEvidenceMismatch {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::CanonicalVersion => "canonical version differs from normalized meaning",
            Self::CanonicalBytes => "canonical bytes differ from normalized meaning",
            Self::Digest => "digest differs from normalized meaning",
        })
    }
}

/// Validates a closed adapter input through qmodel's sole semantic authority.
pub(crate) fn normalize_payload(
    input: SemanticPayloadInputV1,
) -> Result<CurriculumSemanticPayload, SemanticPlannerError> {
    match input {
        SemanticPayloadInputV1::Assignment { definition } => {
            normalize_assignment(definition).map(CurriculumSemanticPayload::assignment)
        }
        SemanticPayloadInputV1::Course { title, modules } => {
            let modules = modules
                .into_iter()
                .map(normalize_module)
                .collect::<Result<Vec<_>, _>>()?;
            CurriculumSemanticCourse::new(title, modules)
                .map(CurriculumSemanticPayload::course)
                .map_err(invalid_meaning)
        }
    }
}

/// Projects one adapter-owned teaching assignment into validated reusable meaning.
#[cfg(any(test, feature = "postgres"))]
pub(crate) fn normalize_teaching_assignment(
    input: TeachingAssignmentInputV1,
    source_term: &CourseTerm,
) -> Result<CurriculumSemanticAssignment, SemanticPlannerError> {
    let schedule = RelativeAssignmentSchedule::from_base_policy(&input.base_policy, source_term)
        .map_err(|error| SemanticPlannerError::Schedule(error.to_string()))?;
    normalize_assignment(SemanticAssignmentInputV1 {
        title: input.title,
        instructions: input.instructions,
        entries: input.entries,
        defaults: input.defaults,
        schedule,
    })
}

/// Projects validated qmodel meaning into the closed adapter persistence shape.
pub(crate) fn semantic_payload_input(
    payload: &CurriculumSemanticPayload,
) -> SemanticPayloadInputV1 {
    match payload {
        CurriculumSemanticPayload::Assignment(assignment) => SemanticPayloadInputV1::Assignment {
            definition: semantic_assignment_input(assignment),
        },
        CurriculumSemanticPayload::Course(course) => SemanticPayloadInputV1::Course {
            title: course.title().to_owned(),
            modules: course
                .modules()
                .iter()
                .map(|module| SemanticModuleInputV1 {
                    label: module.label().to_owned(),
                    assignments: module
                        .assignments()
                        .iter()
                        .map(semantic_assignment_input)
                        .collect(),
                })
                .collect(),
        },
    }
}

fn normalize_module(
    input: SemanticModuleInputV1,
) -> Result<CurriculumSemanticModule, SemanticPlannerError> {
    let assignments = input
        .assignments
        .into_iter()
        .map(normalize_assignment)
        .collect::<Result<Vec<_>, _>>()?;
    CurriculumSemanticModule::new(input.label, assignments).map_err(invalid_meaning)
}

fn normalize_assignment(
    input: SemanticAssignmentInputV1,
) -> Result<CurriculumSemanticAssignment, SemanticPlannerError> {
    let entries = input
        .entries
        .into_iter()
        .map(normalize_entry)
        .collect::<Result<Vec<_>, _>>()?;
    CurriculumSemanticAssignment::new(
        input.title,
        input.instructions,
        entries,
        input.defaults,
        input.schedule,
    )
    .map_err(invalid_meaning)
}

fn normalize_entry(
    input: SemanticAssignmentEntryInputV1,
) -> Result<CurriculumSemanticAssignmentEntry, SemanticPlannerError> {
    match input {
        SemanticAssignmentEntryInputV1::Fixed {
            reference,
            points_possible,
            scoring_mode,
        } => Ok(CurriculumSemanticAssignmentEntry::Fixed {
            reference,
            points_possible,
            scoring_mode,
        }),
        SemanticAssignmentEntryInputV1::Pool {
            candidates,
            draw_count,
            points_per_item,
            ordering,
            algorithm,
        } => CurriculumSemanticPool::new(
            candidates,
            draw_count,
            points_per_item,
            ordering,
            algorithm,
        )
        .map(CurriculumSemanticAssignmentEntry::Pool)
        .map_err(invalid_meaning),
    }
}

fn semantic_assignment_input(
    assignment: &CurriculumSemanticAssignment,
) -> SemanticAssignmentInputV1 {
    SemanticAssignmentInputV1 {
        title: assignment.title().to_owned(),
        instructions: assignment.instructions().clone(),
        entries: assignment
            .entries()
            .iter()
            .map(|entry| match entry {
                CurriculumSemanticAssignmentEntry::Fixed {
                    reference,
                    points_possible,
                    scoring_mode,
                } => SemanticAssignmentEntryInputV1::Fixed {
                    reference: *reference,
                    points_possible: *points_possible,
                    scoring_mode: *scoring_mode,
                },
                CurriculumSemanticAssignmentEntry::Pool(pool) => {
                    SemanticAssignmentEntryInputV1::Pool {
                        candidates: pool.candidates().to_vec(),
                        draw_count: pool.draw_count(),
                        points_per_item: pool.points_per_item(),
                        ordering: pool.ordering(),
                        algorithm: pool.algorithm(),
                    }
                }
            })
            .collect(),
        defaults: assignment.defaults().clone(),
        schedule: assignment.schedule().clone(),
    }
}

fn invalid_meaning(
    error: question_model::ReusableCurriculumValidationError,
) -> SemanticPlannerError {
    SemanticPlannerError::InvalidMeaning(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use question_model::{
        AssignmentDeadlineBehavior, AssignmentTeachingSettingsField, CompletionRequirement,
        ContinuedPractice, CourseLocalDateTime, GradePolicy, LateSubmissionPolicy, ProblemId,
        RunPolicies, StudentDisclosurePolicy, VariationPolicy, VersionId,
    };

    fn reference(value: u128) -> ProblemVersionRef {
        ProblemVersionRef {
            problem: ProblemId::from_uuid(uuid::Uuid::from_u128(value)),
            version: VersionId::from_uuid(uuid::Uuid::from_u128(value + 1)),
        }
    }

    fn assignment(entries: Vec<SemanticAssignmentEntryInputV1>) -> SemanticAssignmentInputV1 {
        SemanticAssignmentInputV1 {
            title: "Week one".into(),
            instructions: AssignmentInstructions::try_new("Read carefully".into())
                .expect("instructions"),
            entries,
            defaults: defaults(),
            schedule: RelativeAssignmentSchedule::default(),
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
            student_disclosure: StudentDisclosurePolicy::default(),
        }
    }

    #[test]
    fn normalization_uses_qmodel_bounds_and_preserves_exact_pins() {
        let pin = reference(10);
        let normalized = normalize_payload(SemanticPayloadInputV1::Assignment {
            definition: assignment(vec![SemanticAssignmentEntryInputV1::Fixed {
                reference: pin,
                points_possible: PointValue::from_whole(1),
                scoring_mode: AssignmentScoringMode::Normal,
            }]),
        })
        .expect("bounded assignment normalizes");
        let CurriculumSemanticPayload::Assignment(normalized) = normalized else {
            panic!("assignment input remains assignment-sized")
        };
        assert!(matches!(
            normalized.entries(),
            [CurriculumSemanticAssignmentEntry::Fixed { reference, .. }] if *reference == pin
        ));

        let error = normalize_payload(SemanticPayloadInputV1::Assignment {
            definition: assignment(Vec::new()),
        })
        .expect_err("qmodel rejects empty reusable assignments");
        assert!(matches!(error, SemanticPlannerError::InvalidMeaning(_)));
    }

    #[test]
    fn semantic_input_is_closed_and_round_trips_through_qmodel() {
        let input = SemanticPayloadInputV1::Assignment {
            definition: assignment(vec![SemanticAssignmentEntryInputV1::Fixed {
                reference: reference(20),
                points_possible: PointValue::from_whole(2),
                scoring_mode: AssignmentScoringMode::FullCredit,
            }]),
        };
        let value = serde_json::to_value(&input).expect("closed input serializes");
        let normalized = normalize_payload(input).expect("input normalizes");
        assert_eq!(
            semantic_payload_input(&normalized),
            serde_json::from_value(value).expect("input")
        );

        let unknown = serde_json::json!({
            "kind": "assignment",
            "definition": {
                "title": "Week one",
                "instructions": "",
                "entries": [],
                "defaults": defaults(),
                "schedule": RelativeAssignmentSchedule::default(),
                "answerKey": "must not be accepted"
            }
        });
        assert!(serde_json::from_value::<SemanticPayloadInputV1>(unknown).is_err());
    }

    #[test]
    fn teaching_assignment_normalization_projects_the_source_term_schedule() {
        let source_term = CourseTerm::from_parts("2026-08-24", "2026-12-12", "America/Chicago")
            .expect("source term");
        let due_at = CourseLocalDateTime::parse("2026-09-01T17:30:00.000")
            .expect("source local time")
            .resolve_for_course(&source_term, AssignmentTeachingSettingsField::DueAt)
            .expect("source due time");
        let normalized = normalize_teaching_assignment(
            TeachingAssignmentInputV1 {
                title: "Current teaching".into(),
                instructions: AssignmentInstructions::default(),
                entries: vec![SemanticAssignmentEntryInputV1::Fixed {
                    reference: reference(30),
                    points_possible: PointValue::from_whole(1),
                    scoring_mode: AssignmentScoringMode::Normal,
                }],
                defaults: defaults(),
                base_policy: BaseAssignmentPolicy {
                    due_at: Some(due_at),
                    ..BaseAssignmentPolicy::default()
                },
            },
            &source_term,
        )
        .expect("stored teaching state normalizes");

        let due = normalized.schedule().due_at.as_ref().expect("relative due");
        assert_eq!(due.day_offset, 8);
        assert_eq!(due.local_time.as_str(), "17:30:00.000");
    }
}
