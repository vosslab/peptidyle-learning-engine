//! Browser-safe reusable BlueprintCourse assessments and answer-free views.
//!
//! A reusable content has no course, student, version, or server-private
//! identity. Fixed entries carry exact public Question Revision Tuples
//! checked under destination authority before persistence. Browser views keep the same ordered
//! shape while substituting current answer-free Question Library discovery rows.

use std::num::NonZeroU64;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::BlueprintCourseId;

/// Shared instructor-content bound for reusable titles and module labels.
pub const MAX_BLUEPRINT_COURSE_TITLE_UNICODE_SCALARS: usize = 200;

mod assessment_content;
mod blueprint_children;
/// Canonical reusable Blueprint exchange projection.
pub mod canonical_exchange;
mod fork_apply;
mod fork_comparison;
pub use assessment_content::{
    BlueprintAssessmentContentInput, BlueprintAssessmentContentView, BlueprintAssessmentDefaults,
    BlueprintAssessmentEntryInput, BlueprintAssessmentEntryView, BlueprintPoolInputChoice,
    ReusableFixedQuestionInput, ReusablePoolInput, ReusablePoolView, ReusableQuestionView,
    ReusableSelectionAvailability,
};
pub use blueprint_children::{
    BlueprintAssessmentEditChoice, BlueprintAssessmentId, BlueprintAssessmentReplacementInput,
    BlueprintChildIdError, BlueprintCourseAssessmentContentView, BlueprintModuleEditChoice,
    BlueprintModuleId, BlueprintModuleReplacementInput, BlueprintModuleView,
    CreateBlueprintCourseInput, CreateBlueprintFromCourseInstanceInput, CreateBlueprintModuleInput,
    ReplaceBlueprintCourseContentInput,
};
pub use fork_apply::{
    BlueprintForkApplyAssessmentCopy, BlueprintForkApplyAssessmentDestination,
    BlueprintForkApplyError, BlueprintForkApplyModuleDestination,
    BlueprintForkApplyModuleLabelCopy, BlueprintForkApplyModuleLayout, BlueprintForkApplySelection,
    apply_blueprint_fork,
};
pub use fork_comparison::{
    BlueprintAssessmentRelationship, BlueprintComparison, BlueprintComparisonAssessment,
    BlueprintComparisonError, BlueprintComparisonInventory, BlueprintComparisonModule,
    compare_blueprint_courses,
};

/// Failure to validate a reusable title or module label.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlueprintCourseTitleError {
    /// The value is blank, leading/trailing whitespace-bearing, or too long.
    Invalid,
}

impl std::fmt::Display for BlueprintCourseTitleError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("curriculum text must be trimmed, nonempty, and within its bound")
    }
}

impl std::error::Error for BlueprintCourseTitleError {}

/// Validates a durable reusable content title or module label.
pub fn validate_blueprint_course_title(value: &str) -> Result<(), BlueprintCourseTitleError> {
    (value == value.trim()
        && !value.is_empty()
        && value.chars().count() <= MAX_BLUEPRINT_COURSE_TITLE_UNICODE_SCALARS)
        .then_some(())
        .ok_or(BlueprintCourseTitleError::Invalid)
}

/// Strong Revision Number for one complete Blueprint Course tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct BlueprintRevisionNumber(NonZeroU64);

macro_rules! impl_revision {
    ($name:ident) => {
        impl $name {
            /// Initial revision for a newly stored aggregate.
            pub const INITIAL: Self = Self(NonZeroU64::MIN);

            /// Rebuilds a positive PostgreSQL-bigint revision.
            pub fn new(value: u64) -> Option<Self> {
                (value <= i64::MAX as u64)
                    .then(|| NonZeroU64::new(value))
                    .flatten()
                    .map(Self)
            }

            /// Returns the exact positive revision scalar.
            pub fn value(self) -> u64 {
                self.0.get()
            }

            /// Advances one revision without exceeding PostgreSQL bigint.
            pub fn checked_next(self) -> Option<Self> {
                self.value().checked_add(1).and_then(Self::new)
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(formatter, "{}", self.value())
            }
        }

        impl FromStr for $name {
            type Err = &'static str;

            fn from_str(value: &str) -> Result<Self, Self::Err> {
                if value.is_empty()
                    || (value.len() > 1 && value.starts_with('0'))
                    || !value.bytes().all(|byte| byte.is_ascii_digit())
                {
                    return Err("revision must be a canonical positive decimal string");
                }
                value
                    .parse::<u64>()
                    .ok()
                    .and_then(Self::new)
                    .ok_or("revision must fit a positive PostgreSQL bigint")
            }
        }

        impl TryFrom<String> for $name {
            type Error = &'static str;

            fn try_from(value: String) -> Result<Self, Self::Error> {
                value.parse()
            }
        }

        impl From<$name> for String {
            fn from(value: $name) -> Self {
                value.to_string()
            }
        }
    };
}

impl_revision!(BlueprintRevisionNumber);

/// Closed browser-safe classification for one returned Blueprint Course view.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BlueprintCourseReadAccess {
    /// The current Active Instructor Account is the exact Blueprint Course Owner.
    BlueprintCourseOwner,
    /// The current Active Instructor Account reads reusable saved Blueprint content.
    ActiveInstructor,
}

/// Safe compact current Blueprint Course Summary View.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct BlueprintCourseSummaryView {
    /// Independently editable current classification.
    pub classification: crate::CourseClassification,
    /// Blueprint Course ID resolved under current read authority.
    pub id: BlueprintCourseId,
    /// Compact stable-lineage name for constrained navigation.
    pub short_name: String,
    /// Descriptive stable-lineage name for headings and listings.
    pub long_name: String,
    /// Current stable-lineage availability for discovery and new selection.
    pub availability: crate::BlueprintAvailability,
    /// Opaque validator for rename and availability actions.
    pub blueprint_edit_number: crate::BlueprintEditNumber,
    /// Exact current immutable reusable state.
    pub current_revision_tuple: crate::BlueprintRevisionTuple,
    /// Browser-safe classification for this returned Blueprint Course view.
    pub read_access: BlueprintCourseReadAccess,
    /// Lifetime Course Instances adopted from this Blueprint lineage, across Revisions.
    pub total_adoptions: u64,
    /// Students counted once per adopted Course Instance, including ended memberships.
    pub total_students_ever_enrolled: u64,
}

/// Safe current Blueprint Course View of one complete BlueprintCourse tree.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct BlueprintCourseView {
    /// Independently editable current classification.
    pub classification: crate::CourseClassification,
    /// Blueprint Course ID resolved for this returned read view.
    pub id: BlueprintCourseId,
    /// Compact stable-lineage name for constrained navigation.
    pub short_name: String,
    /// Descriptive stable-lineage name for headings and listings.
    pub long_name: String,
    /// Current stable-lineage availability for discovery and new selection.
    pub availability: crate::BlueprintAvailability,
    /// Opaque validator for rename and availability actions.
    pub blueprint_edit_number: crate::BlueprintEditNumber,
    /// Exact current immutable reusable state.
    pub current_revision_tuple: crate::BlueprintRevisionTuple,
    /// Browser-safe classification for this returned Blueprint Course view.
    pub read_access: BlueprintCourseReadAccess,
    /// Exact fork origin; roots and hidden sources both return null.
    pub fork_source_tuple: Option<crate::BlueprintRevisionTuple>,
    /// Answer-free current Revision content.
    pub modules: Vec<BlueprintModuleView>,
}

/// Meaning-level validation failure for a Blueprint Course command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlueprintCourseValidationError {
    /// Course hierarchy or Tags violate the current metadata contract.
    InvalidClassification,
    /// A reusable content title is not durable instructor content.
    InvalidContentTitle,
    /// A Blueprint Course lineage name is not durable instructor content.
    InvalidBlueprintName,
    /// A module label is not durable instructor content.
    InvalidModuleLabel,
    /// A reusable content has no usable entries or exceeds its shared bound.
    InvalidEntryCount,
    /// A BlueprintCourse has no usable modules or exceeds its shared bound.
    InvalidModuleCount,
    /// A module has no usable assessments or exceeds its shared bound.
    InvalidModuleAssessmentCount,
    /// A Question Pool Item list has no members or exceeds its shared bound.
    InvalidQuestionPoolItems,
    /// A pool selection count cannot select a meaningful subset of its Question Pool Items.
    InvalidPoolSelectionCount,
    /// A pool repeats a Question Pool Item and therefore changes no selectable meaning.
    DuplicateQuestionPoolItem,
    /// Newly authored Pool members lack explicit interchangeability review.
    QuestionPoolInterchangeabilityNotAttested,
    /// All Question Pool Items exceed the assessment-level shared bound.
    TooManyQuestionPoolItems,
    /// A reusable whole Assessment Attempt time limit exceeds the ordinary assessment bound.
    AssessmentAttemptTimeLimitOutOfRange,
    /// A reusable attempt limit exceeds the ordinary assessment bound.
    AttemptLimitOutOfRange,
    /// A replacement submitted the same retained Blueprint Module ID more than once.
    DuplicateRetainedBlueprintModuleChoice,
    /// A replacement submitted the same retained Blueprint Assessment ID more than once.
    DuplicateRetainedBlueprintAssessmentChoice,
}

impl std::fmt::Display for BlueprintCourseValidationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::InvalidContentTitle => "reusable content title is invalid",
            Self::InvalidBlueprintName => "Blueprint Course name is invalid",
            Self::InvalidClassification => "Course classification is invalid",
            Self::InvalidModuleLabel => "BlueprintCourse module label is invalid",
            Self::InvalidEntryCount => "reusable content must contain bounded ordered entries",
            Self::InvalidModuleCount => "BlueprintCourse must contain bounded modules",
            Self::InvalidModuleAssessmentCount => {
                "BlueprintCourse module must contain bounded Blueprint Assessments"
            }
            Self::InvalidQuestionPoolItems => {
                "Question Pool Items must be present and within their bound"
            }
            Self::InvalidPoolSelectionCount => {
                "Question Pool selection count must be between one and Question Pool Item count"
            }
            Self::DuplicateQuestionPoolItem => "Question Pool Items must be distinct",
            Self::QuestionPoolInterchangeabilityNotAttested => {
                "new Question Pool members require interchangeability attestation"
            }
            Self::TooManyQuestionPoolItems => {
                "Question Pool Items exceed the assessment-level bound"
            }
            Self::AssessmentAttemptTimeLimitOutOfRange => {
                "reusable time limit exceeds the supported range"
            }
            Self::AttemptLimitOutOfRange => "reusable attempt limit exceeds the supported range",
            Self::DuplicateRetainedBlueprintModuleChoice => {
                "Blueprint Course replacement repeats a retained Blueprint Module ID"
            }
            Self::DuplicateRetainedBlueprintAssessmentChoice => {
                "Blueprint Course replacement repeats a retained Blueprint Assessment ID"
            }
        })
    }
}

impl std::error::Error for BlueprintCourseValidationError {}

#[cfg(test)]
mod tests {
    use std::num::NonZeroU32;

    use super::*;
    use crate::{
        AssessmentActivityRules, AssessmentEntryScoringRule, AssessmentInstructions,
        AssessmentPointValue, LateWorkRule, QuestionAttemptLimit, QuestionAttemptTimeLimit,
        QuestionAuthor, QuestionAuthorDisplayName, QuestionAuthorship, QuestionAvailability,
        QuestionBackend, QuestionBackendCapabilities, QuestionFormat, QuestionId, QuestionLicense,
        QuestionMetadata, QuestionPoolEditNumber, QuestionPoolSelectionRule,
        QuestionRevisionNumber, QuestionRevisionTuple, QuestionSearchResult, QuestionStatistics,
        QuestionSummary, QuestionType, StudentFeedbackReleaseRule, Timestamp,
    };
    use uuid::Uuid;

    fn blueprint_module_id() -> BlueprintModuleId {
        BlueprintModuleId::from_uuid(Uuid::from_u128(1))
    }

    fn blueprint_assessment_id() -> BlueprintAssessmentId {
        BlueprintAssessmentId::from_uuid(Uuid::from_u128(2))
    }

    fn question_id() -> QuestionId {
        "7K3M-19QX".parse().expect("valid question ID")
    }

    fn defaults() -> BlueprintAssessmentDefaults {
        BlueprintAssessmentDefaults {
            assessment_attempt_time_limit_seconds: None,
            attempt_limit: None,
            late_work_rule: LateWorkRule::Accept,
            activity_rules: AssessmentActivityRules {
                question_variation_rule: crate::AssessmentQuestionVariationRule::NewVariation,
                ..AssessmentActivityRules::default()
            },
            student_feedback_release_rule: StudentFeedbackReleaseRule::default(),
        }
    }

    fn input() -> BlueprintAssessmentContentInput {
        BlueprintAssessmentContentInput {
            assessment_type: crate::AssessmentType::PracticeQuestionAssignment,
            title: "Protein structure practice".to_string(),
            instructions: AssessmentInstructions::try_new("Explain each choice.".to_string())
                .expect("valid instructions"),
            entries: vec![
                BlueprintAssessmentEntryInput::Fixed(ReusableFixedQuestionInput {
                    question_revision_tuple: QuestionRevisionTuple {
                        question_id: question_id(),
                        revision_number: QuestionRevisionNumber::new(1).expect("positive Revision"),
                    },
                    points_possible: AssessmentPointValue::from_whole(3),
                    scoring_rule: AssessmentEntryScoringRule::Normal,
                    question_attempt_limit: QuestionAttemptLimit { max_attempts: None },
                    question_attempt_time_limit: QuestionAttemptTimeLimit::Unlimited,
                }),
                BlueprintAssessmentEntryInput::Pool(ReusablePoolInput {
                    pool: BlueprintPoolInputChoice::Import {
                        question_pool_id: "12A4-TBCZ".parse().expect("valid Pool ID"),
                        question_pool_edit_number: QuestionPoolEditNumber::new(1)
                            .expect("valid Pool Edit Number"),
                    },
                    selection_count: NonZeroU32::new(1).expect("positive count"),
                    points_per_item: AssessmentPointValue::from_whole(2),
                    scoring_rule: AssessmentEntryScoringRule::Normal,
                    selection_rule: QuestionPoolSelectionRule {
                        selected_question_order:
                            crate::QuestionPoolSelectedQuestionOrder::RandomOrder,
                    },
                    question_attempt_limit: QuestionAttemptLimit { max_attempts: None },
                    question_attempt_time_limit: QuestionAttemptTimeLimit::Unlimited,
                }),
            ],
            defaults: defaults(),
        }
    }

    fn discovery() -> QuestionSearchResult {
        QuestionSearchResult {
            summary: QuestionSummary {
                question_id: question_id(),
                question_revision_tuple: QuestionRevisionTuple {
                    question_id: question_id(),
                    revision_number: QuestionRevisionNumber::new(1).expect("positive version"),
                },
                backend: QuestionBackend::Ple,
                question_format: QuestionFormat::PleQuestionJson,
                question_type: QuestionType::MultipleChoice,
                capabilities: QuestionBackendCapabilities::none(),
                metadata: QuestionMetadata {
                    question_title: "Safe Question Library row".to_string(),
                    question_description: "Instructor-facing Question Library row fixture."
                        .to_string(),
                    tags: Vec::new(),
                    question_license: Some(QuestionLicense::Cc0_1_0),
                    question_citation: None,
                    language: "en".to_string(),
                },
                authorship: QuestionAuthorship::new(vec![QuestionAuthor {
                    display_name: QuestionAuthorDisplayName::new("Ada Lovelace".to_string())
                        .expect("valid Question Author"),
                }])
                .expect("valid Question Authorship"),
                availability: QuestionAvailability::Available,
                published_at: Timestamp::from_unix_millis(0),
                bloom: Some(crate::BloomClassificationView {
                    cognitive_process: crate::BloomCognitiveProcess::Understand,
                    knowledge_dimension: crate::BloomKnowledgeDimension::ConceptualKnowledge,
                    classification_edit_number: crate::BloomClassificationEditNumber::INITIAL,
                }),
            },
            discipline_name: "Biology".to_string(),
            discipline_is_retired: false,
            evidence: QuestionStatistics::Unavailable,
        }
    }

    #[test]
    fn curriculum_references_round_trip_as_compact_wire_values() {
        let blueprint: BlueprintCourseId = "BP7K3M2QXH".parse().expect("valid Blueprint Course ID");
        assert_eq!(
            serde_json::to_value(blueprint).expect("serializes"),
            "BP7K3M2QXH"
        );
        assert!("BP7K3M2I".parse::<BlueprintCourseId>().is_err());
        assert!("AC7K3M2Q".parse::<BlueprintCourseId>().is_err());
    }

    #[test]
    fn ordered_content_validation_uses_vector_order_and_pool_meaning() {
        let mut content = input();
        content.defaults.attempt_limit = NonZeroU32::new(3);
        assert!(content.validate().is_ok());
        let wire = serde_json::to_value(&content).expect("content serializes");
        assert_eq!(wire["entries"][0]["kind"], "fixed");
        assert_eq!(
            wire["entries"][0]["question_revision_tuple"]["questionId"],
            "7K3M-19QX"
        );
        assert_eq!(
            wire["entries"][0]["question_revision_tuple"]["revisionNumber"],
            1
        );
        assert_eq!(wire["entries"][0]["points_possible"], "3");
        assert_eq!(wire["entries"][1]["kind"], "pool");
        assert_eq!(wire["entries"][1]["pool"]["kind"], "import");
        assert_eq!(wire["entries"][1]["points_per_item"], "2");
        assert!(wire["defaults"].is_object());
        assert_eq!(wire["defaults"]["assessment_attempt_limit"], 3);
        assert!(wire["defaults"].get("attempt_limit").is_none());
        assert!(wire.get("schedule").is_none());
        assert_eq!(
            serde_json::from_value::<BlueprintAssessmentContentInput>(wire)
                .expect("content round trips"),
            content
        );
        let mut leftover = serde_json::to_value(&content).expect("content serializes");
        leftover["entries"][0]["published_question"] =
            leftover["entries"][0]["question_revision_tuple"].take();
        leftover["entries"][0]
            .as_object_mut()
            .expect("fixed entry")
            .remove("question_revision_tuple");
        assert!(
            serde_json::from_value::<BlueprintAssessmentContentInput>(leftover).is_err(),
            "Tuple JSON is not accepted under leftover published_question"
        );
        let mut number_under_tuple = serde_json::to_value(&content).expect("content serializes");
        number_under_tuple["entries"][0]["question_revision_tuple"] = serde_json::json!(1);
        assert!(
            serde_json::from_value::<BlueprintAssessmentContentInput>(number_under_tuple).is_err(),
            "a lone Revision Number is not accepted under question_revision_tuple"
        );
        let mut legacy_wire = serde_json::to_value(&content).expect("content serializes");
        let defaults = legacy_wire["defaults"]
            .as_object_mut()
            .expect("defaults is an object");
        let attempt_limit = defaults
            .remove("assessment_attempt_limit")
            .expect("canonical key is present");
        defaults.insert("attempt_limit".to_string(), attempt_limit);
        assert!(serde_json::from_value::<BlueprintAssessmentContentInput>(legacy_wire).is_err());
        assert!(content.validate().is_ok());
        let blueprint = CreateBlueprintCourseInput {
            classification: crate::CourseClassification {
                discipline_uuid: uuid::Uuid::from_u128(0xcc01),
                subject_uuid: None,
                topic_uuid: None,
                subtopic_uuid: None,
                tags: Vec::new(),
            },
            short_name: "Biochemistry".to_string(),
            long_name: "Biochemistry Blueprint".to_string(),
            modules: vec![CreateBlueprintModuleInput {
                label: "Week 1".to_string(),
                assessments: vec![input()],
            }],
        };
        assert!(blueprint.validate().is_ok());
    }

    #[test]
    fn blueprint_course_view_serializes_answer_free_question_library_rows_and_edit_choices() {
        let view = BlueprintCourseView {
            classification: crate::CourseClassification {
                discipline_uuid: uuid::Uuid::from_u128(0xcc01),
                subject_uuid: None,
                topic_uuid: None,
                subtopic_uuid: None,
                tags: Vec::new(),
            },
            id: "BP7K3M2QXH".parse().expect("valid Blueprint Course ID"),
            short_name: "Biochemistry".to_string(),
            long_name: "Biochemistry Blueprint".to_string(),
            availability: crate::BlueprintAvailability::Public,
            blueprint_edit_number: crate::BlueprintEditNumber::from_edit_number(42),
            current_revision_tuple: crate::BlueprintRevisionTuple {
                blueprint_course_id: "BP7K3M2QXH".parse().expect("valid Blueprint Course ID"),
                revision_number: BlueprintRevisionNumber::INITIAL,
            },
            read_access: BlueprintCourseReadAccess::ActiveInstructor,
            fork_source_tuple: None,
            modules: vec![BlueprintModuleView {
                blueprint_module_id: blueprint_module_id(),
                label: "Week 1".to_string(),
                assessments: vec![BlueprintCourseAssessmentContentView {
                    blueprint_assessment_id: blueprint_assessment_id(),
                    content: BlueprintAssessmentContentView {
                        assessment_type: crate::AssessmentType::PracticeQuestionAssignment,
                        title: "Protein structure practice".to_string(),
                        instructions: AssessmentInstructions::default(),
                        entries: vec![
                            BlueprintAssessmentEntryView::Fixed {
                                question: ReusableQuestionView {
                                    question_revision_tuple: discovery()
                                        .summary
                                        .question_revision_tuple,
                                    question_library: discovery(),
                                    selection_availability:
                                        ReusableSelectionAvailability::Available,
                                }
                                .into(),
                                points_possible: AssessmentPointValue::from_whole(3),
                                scoring_rule: AssessmentEntryScoringRule::Normal,
                                question_attempt_limit: QuestionAttemptLimit { max_attempts: None },
                                question_attempt_time_limit: QuestionAttemptTimeLimit::Unlimited,
                            },
                            BlueprintAssessmentEntryView::Pool(ReusablePoolView {
                                question_pool_id: "12A4-TBCZ".parse().expect("Pool ID"),
                                question_pool_edit_number: QuestionPoolEditNumber::new(1)
                                    .expect("Pool Edit Number"),
                                selection_count: NonZeroU32::new(1).expect("positive count"),
                                points_per_item: AssessmentPointValue::from_whole(2),
                                scoring_rule: AssessmentEntryScoringRule::Normal,
                                selection_rule: QuestionPoolSelectionRule {
                                    selected_question_order:
                                        crate::QuestionPoolSelectedQuestionOrder::QuestionPoolOrder,
                                },
                                question_attempt_limit: QuestionAttemptLimit { max_attempts: None },
                                question_attempt_time_limit: QuestionAttemptTimeLimit::Unlimited,
                            }),
                        ],
                        defaults: defaults(),
                    },
                }],
            }],
        };
        let wire = serde_json::to_value(view).expect("safe view serializes");
        assert_eq!(wire["id"], "BP7K3M2QXH");
        assert_eq!(wire["current_revision_tuple"]["revisionNumber"], "1");
        let mut leftover = wire.clone();
        leftover["current_revision"] = leftover["current_revision_tuple"].clone();
        leftover
            .as_object_mut()
            .expect("view object")
            .remove("current_revision_tuple");
        assert!(
            serde_json::from_value::<BlueprintCourseView>(leftover).is_err(),
            "Tuple JSON is not accepted under leftover current_revision"
        );
        let mut number_under_tuple = wire.clone();
        number_under_tuple["current_revision_tuple"] = serde_json::json!("1");
        assert!(
            serde_json::from_value::<BlueprintCourseView>(number_under_tuple).is_err(),
            "a lone Revision Number is not accepted under current_revision_tuple"
        );
        assert_eq!(
            wire["modules"][0]["assessments"][0]["content"]["entries"][0]["kind"],
            "fixed"
        );
        assert!(
            wire.pointer("/modules/0/assessments/0/content/entries/0/question/question_library")
                .is_some()
        );
        assert_eq!(
            wire.pointer("/modules/0/assessments/0/blueprint_assessment_id"),
            Some(&serde_json::Value::String(
                blueprint_assessment_id().to_string(),
            ))
        );
        assert!(
            wire.pointer("/modules/0/assessments/0/content/entries/0/revision")
                .is_none()
        );
        assert_eq!(
            wire["modules"][0]["assessments"][0]["content"]["entries"][1]["kind"],
            "pool"
        );
        assert_eq!(
            wire.pointer("/modules/0/assessments/0/content/entries/1/question_pool_id"),
            Some(&serde_json::Value::String("12A4-TBCZ".to_string()))
        );
        assert_eq!(
            wire.pointer("/modules/0/assessments/0/content/entries/1/question_pool_edit_number"),
            Some(&serde_json::Value::Number(1.into()))
        );
        assert!(
            wire.pointer("/modules/0/assessments/0/content/entries/1/items")
                .is_none()
        );
    }
}

#[cfg(test)]
mod blueprint_course_tests {
    use super::*;
    use crate::{
        AssessmentActivityRules, AssessmentEntryScoringRule, AssessmentInstructions,
        AssessmentPointValue, LateWorkRule, QuestionAttemptLimit, QuestionAttemptTimeLimit,
        QuestionRevisionNumber, QuestionRevisionTuple, StudentFeedbackReleaseRule,
    };
    use uuid::Uuid;

    #[test]
    fn blueprint_course_input_is_one_nested_question_revision_tree() {
        let input = CreateBlueprintCourseInput {
            classification: crate::CourseClassification {
                discipline_uuid: uuid::Uuid::from_u128(0xcc01),
                subject_uuid: None,
                topic_uuid: None,
                subtopic_uuid: None,
                tags: Vec::new(),
            },
            short_name: "Biochemistry".to_owned(),
            long_name: "Biochemistry Blueprint".to_owned(),
            modules: vec![CreateBlueprintModuleInput {
                label: "Week 1".to_owned(),
                assessments: vec![BlueprintAssessmentContentInput {
                    assessment_type: crate::AssessmentType::RegularAssignment,
                    title: "Protein folding".to_owned(),
                    instructions: AssessmentInstructions::default(),
                    entries: vec![BlueprintAssessmentEntryInput::Fixed(
                        ReusableFixedQuestionInput {
                            question_revision_tuple: QuestionRevisionTuple {
                                question_id: "7K3M-19QX".parse().expect("QuestionId"),
                                revision_number: QuestionRevisionNumber::new(1)
                                    .expect("positive Revision"),
                            },
                            points_possible: AssessmentPointValue::from_whole(1),
                            scoring_rule: AssessmentEntryScoringRule::Normal,
                            question_attempt_limit: QuestionAttemptLimit { max_attempts: None },
                            question_attempt_time_limit: QuestionAttemptTimeLimit::Unlimited,
                        },
                    )],
                    defaults: BlueprintAssessmentDefaults {
                        assessment_attempt_time_limit_seconds: None,
                        attempt_limit: None,
                        late_work_rule: LateWorkRule::Accept,
                        activity_rules: AssessmentActivityRules {
                            question_variation_rule:
                                crate::AssessmentQuestionVariationRule::NewVariation,
                            ..AssessmentActivityRules::default()
                        },
                        student_feedback_release_rule: StudentFeedbackReleaseRule::default(),
                    },
                }],
            }],
        };
        input.validate().expect("valid BlueprintCourse");
        let wire = serde_json::to_value(&input).expect("serializes");
        assert!(wire.get("modules").is_some());
        assert!(wire.to_string().contains("question_revision_tuple"));
        assert!(!wire.to_string().contains("QuestionRevisionTuple"));
        let mut forged = wire;
        forged["owner"] = serde_json::json!("U-1");
        assert!(serde_json::from_value::<CreateBlueprintCourseInput>(forged).is_err());
    }

    #[test]
    fn replacement_choices_are_explicit_strict_and_unique() {
        let blueprint_module_id = BlueprintModuleId::from_uuid(Uuid::from_u128(1));
        let blueprint_assessment_id = BlueprintAssessmentId::from_uuid(Uuid::from_u128(2));
        assert!(
            "00000000000000000000000000000001"
                .parse::<BlueprintModuleId>()
                .is_err()
        );
        assert!(
            "00000000-0000-0000-0000-00000000000A"
                .parse::<BlueprintAssessmentId>()
                .is_err()
        );
        let content = BlueprintAssessmentContentInput {
            assessment_type: crate::AssessmentType::RegularAssignment,
            title: "Protein folding".to_owned(),
            instructions: AssessmentInstructions::default(),
            entries: vec![BlueprintAssessmentEntryInput::Fixed(
                ReusableFixedQuestionInput {
                    question_revision_tuple: QuestionRevisionTuple {
                        question_id: "7K3M-19QX".parse().expect("QuestionId"),
                        revision_number: QuestionRevisionNumber::new(1).expect("positive Revision"),
                    },
                    points_possible: AssessmentPointValue::from_whole(1),
                    scoring_rule: AssessmentEntryScoringRule::Normal,
                    question_attempt_limit: QuestionAttemptLimit { max_attempts: None },
                    question_attempt_time_limit: QuestionAttemptTimeLimit::Unlimited,
                },
            )],
            defaults: BlueprintAssessmentDefaults {
                assessment_attempt_time_limit_seconds: None,
                attempt_limit: None,
                late_work_rule: LateWorkRule::Accept,
                activity_rules: AssessmentActivityRules {
                    question_variation_rule: crate::AssessmentQuestionVariationRule::NewVariation,
                    ..AssessmentActivityRules::default()
                },
                student_feedback_release_rule: StudentFeedbackReleaseRule::default(),
            },
        };
        let replacement = ReplaceBlueprintCourseContentInput {
            modules: vec![BlueprintModuleReplacementInput {
                choice: BlueprintModuleEditChoice::Retained {
                    blueprint_module_id,
                },
                label: "Week 1".to_owned(),
                assessments: vec![
                    BlueprintAssessmentReplacementInput {
                        choice: BlueprintAssessmentEditChoice::Retained {
                            blueprint_assessment_id,
                        },
                        content: content.clone(),
                    },
                    BlueprintAssessmentReplacementInput {
                        choice: BlueprintAssessmentEditChoice::New,
                        content: content.clone(),
                    },
                ],
            }],
        };
        replacement.validate().expect("valid retained/new tree");
        let wire = serde_json::to_value(&replacement).expect("serializes");
        assert_eq!(wire["modules"][0]["choice"]["kind"], "retained");
        assert_eq!(
            wire["modules"][0]["choice"]["blueprint_module_id"],
            blueprint_module_id.to_string()
        );
        assert_eq!(
            wire["modules"][0]["assessments"][0]["choice"]["blueprint_assessment_id"],
            blueprint_assessment_id.to_string()
        );
        assert_eq!(
            wire["modules"][0]["assessments"][1]["choice"]["kind"],
            "new"
        );
        let retained = wire["modules"][0]["choice"]
            .as_object()
            .expect("retained choice object");
        assert_eq!(retained.len(), 2);
        assert!(retained.contains_key("kind"));
        assert!(retained.contains_key("blueprint_module_id"));
        let mut forged = wire;
        forged["modules"][0]["choice"]["unexpected"] = serde_json::json!(true);
        assert!(serde_json::from_value::<ReplaceBlueprintCourseContentInput>(forged).is_err());

        let duplicated = ReplaceBlueprintCourseContentInput {
            modules: vec![
                BlueprintModuleReplacementInput {
                    choice: BlueprintModuleEditChoice::Retained {
                        blueprint_module_id,
                    },
                    label: "Week 1".to_owned(),
                    assessments: vec![BlueprintAssessmentReplacementInput {
                        choice: BlueprintAssessmentEditChoice::Retained {
                            blueprint_assessment_id,
                        },
                        content: content.clone(),
                    }],
                },
                BlueprintModuleReplacementInput {
                    choice: BlueprintModuleEditChoice::Retained {
                        blueprint_module_id,
                    },
                    label: "Week 2".to_owned(),
                    assessments: vec![BlueprintAssessmentReplacementInput {
                        choice: BlueprintAssessmentEditChoice::New,
                        content,
                    }],
                },
            ],
        };
        assert_eq!(
            duplicated.validate(),
            Err(BlueprintCourseValidationError::DuplicateRetainedBlueprintModuleChoice)
        );
    }
}
