//! Canonical, answer-free reusable Blueprint Course exchange projection.
//!
//! This is an exchange representation, not persistence. It contains current
//! reusable metadata and teaching content:
//! names, classification, ordered module/Assessment structure, reusable settings,
//! and exact Published Question Revision pins.  It has no owner, lineage or
//! revision identifier, availability, Stars, Watches, Course Instance,
//! Student, delivery, or operational state. Import authenticates and validates
//! this projection before the ordinary creation transaction creates a distinct
//! Private Blueprint Course with fresh local child identities.

use serde::{Deserialize, Serialize};

use crate::{
    AssessmentEntryScoringRule, AssessmentInstructions, AssessmentPointValue,
    BlueprintAssessmentContentInput, BlueprintAssessmentDefaults, BlueprintAssessmentEntryContent,
    BlueprintAssessmentEntryInput, BlueprintCourseContent, BlueprintCourseValidationError,
    BlueprintPoolInputChoice, CreateBlueprintCourseInput, CreateBlueprintModuleInput,
    QuestionAttemptLimit, QuestionAttemptTimeLimit, QuestionPoolSelectionRule,
    QuestionRevisionTuple, ReusableFixedQuestionInput, ReusablePoolInput,
};

/// Deterministic current-metadata and reusable Blueprint Course projection.
///
/// Classification UUIDs identify this installation's shared vocabulary. This
/// does not establish cross-install import matching or historical Revision metadata.
///
/// `serde_json::to_vec` preserves this declared field order and each authored
/// vector order, making the resulting JSON suitable for comparison and later
/// exchange. ASVS 1.1.2: this domain projection retains raw teaching text;
/// any future HTML, download, or other interpreter-specific output encoding
/// belongs at that output boundary.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct CanonicalBlueprintCourse {
    metadata: CanonicalBlueprintMetadata,
    modules: Vec<CanonicalBlueprintModule>,
}

impl CanonicalBlueprintCourse {
    /// Projects validated reusable content without any operational identity.
    ///
    /// The caller supplies current metadata from the selected Blueprint Course. The
    /// persistence/read boundary remains responsible for C73's published
    /// content predicate before constructing the source `BlueprintCourseContent`.
    pub fn export(
        short_name: impl Into<String>,
        long_name: impl Into<String>,
        classification: crate::CourseClassification,
        content: &BlueprintCourseContent,
    ) -> Self {
        Self {
            metadata: CanonicalBlueprintMetadata {
                short_name: short_name.into(),
                long_name: long_name.into(),
                classification,
            },
            modules: content
                .modules()
                .iter()
                .map(CanonicalBlueprintModule::from)
                .collect(),
        }
    }

    /// Returns the canonical lineage names, without identity or visibility.
    pub fn metadata(&self) -> &CanonicalBlueprintMetadata {
        &self.metadata
    }

    /// Returns reusable modules in authored order.
    pub fn modules(&self) -> &[CanonicalBlueprintModule] {
        &self.modules
    }

    /// Serializes the one deterministic JSON exchange representation.
    pub fn json_bytes(&self) -> Vec<u8> {
        serde_json::to_vec(self).expect("canonical Blueprint exchange projection serializes")
    }

    /// Converts strict exchange input into the ordinary new-Blueprint command.
    ///
    /// ASVS 1.5.2, 2.2.1, and 15.3.3: only the closed exchange fields are
    /// accepted, and the complete nested command is validated before it can
    /// reach persistence. No imported identity or operational state exists in
    /// this type.
    ///
    /// # Errors
    ///
    /// Returns [`BlueprintCourseValidationError`] when the decoded metadata or
    /// ordered reusable structure cannot form a valid new Blueprint Course.
    pub fn into_create_input(
        self,
    ) -> Result<CreateBlueprintCourseInput, BlueprintCourseValidationError> {
        let input = CreateBlueprintCourseInput {
            classification: self.metadata.classification,
            short_name: self.metadata.short_name,
            long_name: self.metadata.long_name,
            modules: self
                .modules
                .into_iter()
                .map(CanonicalBlueprintModule::into_create_input)
                .collect(),
        };
        input.validate()?;
        Ok(input)
    }
}

/// Explicit current Blueprint metadata accompanying reusable structure.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct CanonicalBlueprintMetadata {
    short_name: String,
    long_name: String,
    classification: crate::CourseClassification,
}

impl CanonicalBlueprintMetadata {
    /// Explicit current metadata; UUID identities are installation-local.
    pub fn classification(&self) -> &crate::CourseClassification {
        &self.classification
    }
    /// Compact reusable Blueprint name.
    pub fn short_name(&self) -> &str {
        &self.short_name
    }

    /// Descriptive reusable Blueprint name.
    pub fn long_name(&self) -> &str {
        &self.long_name
    }
}

/// One labelled reusable module, retaining authored Assessment order.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct CanonicalBlueprintModule {
    label: String,
    assessments: Vec<CanonicalBlueprintAssessment>,
}

impl CanonicalBlueprintModule {
    /// Returns the reusable module label.
    pub fn label(&self) -> &str {
        &self.label
    }

    /// Returns reusable Blueprint Assessments in authored order.
    pub fn assessments(&self) -> &[CanonicalBlueprintAssessment] {
        &self.assessments
    }

    fn into_create_input(self) -> CreateBlueprintModuleInput {
        CreateBlueprintModuleInput {
            label: self.label,
            assessments: self
                .assessments
                .into_iter()
                .map(CanonicalBlueprintAssessment::into_create_input)
                .collect(),
        }
    }
}

/// One reusable Blueprint Assessment and its complete reusable settings.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct CanonicalBlueprintAssessment {
    assessment_type: crate::AssessmentType,
    title: String,
    instructions: AssessmentInstructions,
    entries: Vec<CanonicalBlueprintAssessmentEntry>,
    defaults: BlueprintAssessmentDefaults,
}

impl CanonicalBlueprintAssessment {
    /// Returns the fixed pedagogical purpose of this reusable Assessment.
    pub fn assessment_type(&self) -> crate::AssessmentType {
        self.assessment_type
    }
    /// Returns the reusable Assessment title.
    pub fn title(&self) -> &str {
        &self.title
    }

    /// Returns student-facing reusable instructions.
    pub fn instructions(&self) -> &AssessmentInstructions {
        &self.instructions
    }

    /// Returns fixed Published Questions and published Question Pools in order.
    pub fn entries(&self) -> &[CanonicalBlueprintAssessmentEntry] {
        &self.entries
    }

    /// Returns reusable Assessment settings only.
    pub fn defaults(&self) -> &BlueprintAssessmentDefaults {
        &self.defaults
    }

    fn into_create_input(self) -> BlueprintAssessmentContentInput {
        BlueprintAssessmentContentInput {
            assessment_type: self.assessment_type,
            title: self.title,
            instructions: self.instructions,
            entries: self
                .entries
                .into_iter()
                .map(CanonicalBlueprintAssessmentEntry::into_create_input)
                .collect(),
            defaults: self.defaults,
        }
    }
}

/// One fixed Published Question or one published Question Pool.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum CanonicalBlueprintAssessmentEntry {
    /// One exact immutable Published Question Revision.
    Fixed {
        #[serde(deserialize_with = "deserialize_question_revision_reference")]
        published_question: QuestionRevisionTuple,
        points_possible: AssessmentPointValue,
        scoring_rule: AssessmentEntryScoringRule,
        question_attempt_limit: QuestionAttemptLimit,
        #[serde(deserialize_with = "deserialize_question_attempt_time_limit")]
        question_attempt_time_limit: QuestionAttemptTimeLimit,
    },
    /// One current Question Pool with reusable selection settings.
    Pool {
        question_pool_id: crate::QuestionId,
        question_pool_edit_number: crate::QuestionPoolEditNumber,
        selection_count: std::num::NonZeroU32,
        points_per_item: AssessmentPointValue,
        scoring_rule: AssessmentEntryScoringRule,
        selection_rule: QuestionPoolSelectionRule,
        question_attempt_limit: QuestionAttemptLimit,
        #[serde(deserialize_with = "deserialize_question_attempt_time_limit")]
        question_attempt_time_limit: QuestionAttemptTimeLimit,
    },
}

fn deserialize_question_revision_reference<'de, D>(
    deserializer: D,
) -> Result<QuestionRevisionTuple, D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase", deny_unknown_fields)]
    struct StrictReference {
        question_id: crate::QuestionId,
        revision_number: crate::QuestionRevisionNumber,
    }

    let reference = StrictReference::deserialize(deserializer)?;
    Ok(QuestionRevisionTuple {
        question_id: reference.question_id,
        revision_number: reference.revision_number,
    })
}

fn deserialize_question_attempt_time_limit<'de, D>(
    deserializer: D,
) -> Result<QuestionAttemptTimeLimit, D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(
        tag = "kind",
        rename_all = "camelCase",
        rename_all_fields = "camelCase",
        deny_unknown_fields
    )]
    enum StrictQuestionAttemptTimeLimit {
        Unlimited {},
        Limited { seconds: u32, grace_seconds: u32 },
    }

    Ok(
        match StrictQuestionAttemptTimeLimit::deserialize(deserializer)? {
            StrictQuestionAttemptTimeLimit::Unlimited {} => QuestionAttemptTimeLimit::Unlimited,
            StrictQuestionAttemptTimeLimit::Limited {
                seconds,
                grace_seconds,
            } => QuestionAttemptTimeLimit::Limited {
                seconds,
                grace_seconds,
            },
        },
    )
}

impl CanonicalBlueprintAssessmentEntry {
    fn into_create_input(self) -> BlueprintAssessmentEntryInput {
        match self {
            Self::Fixed {
                published_question,
                points_possible,
                scoring_rule,
                question_attempt_limit,
                question_attempt_time_limit,
            } => BlueprintAssessmentEntryInput::Fixed(ReusableFixedQuestionInput {
                published_question,
                points_possible,
                scoring_rule,
                question_attempt_limit,
                question_attempt_time_limit,
            }),
            Self::Pool {
                question_pool_id,
                question_pool_edit_number,
                selection_count,
                points_per_item,
                scoring_rule,
                selection_rule,
                question_attempt_limit,
                question_attempt_time_limit,
            } => BlueprintAssessmentEntryInput::Pool(ReusablePoolInput {
                pool: BlueprintPoolInputChoice::Import {
                    question_pool_id,
                    question_pool_edit_number,
                },
                selection_count,
                points_per_item,
                scoring_rule,
                selection_rule,
                question_attempt_limit,
                question_attempt_time_limit,
            }),
        }
    }
}

impl From<&crate::BlueprintCourseModuleContent> for CanonicalBlueprintModule {
    fn from(module: &crate::BlueprintCourseModuleContent) -> Self {
        Self {
            label: module.label().to_owned(),
            assessments: module
                .assessments()
                .iter()
                .map(CanonicalBlueprintAssessment::from)
                .collect(),
        }
    }
}

impl From<&crate::BlueprintAssessmentContent> for CanonicalBlueprintAssessment {
    fn from(assessment: &crate::BlueprintAssessmentContent) -> Self {
        Self {
            assessment_type: assessment.assessment_type(),
            title: assessment.title().to_owned(),
            instructions: assessment.instructions().clone(),
            entries: assessment
                .entries()
                .iter()
                .map(CanonicalBlueprintAssessmentEntry::from)
                .collect(),
            defaults: assessment.defaults().clone(),
        }
    }
}

impl From<&BlueprintAssessmentEntryContent> for CanonicalBlueprintAssessmentEntry {
    fn from(entry: &BlueprintAssessmentEntryContent) -> Self {
        match entry {
            BlueprintAssessmentEntryContent::Fixed {
                question_revision,
                points_possible,
                scoring_rule,
                question_attempt_limit,
                question_attempt_time_limit,
            } => Self::Fixed {
                published_question: question_revision.clone(),
                points_possible: *points_possible,
                scoring_rule: *scoring_rule,
                question_attempt_limit: *question_attempt_limit,
                question_attempt_time_limit: *question_attempt_time_limit,
            },
            BlueprintAssessmentEntryContent::Pool(pool) => Self::Pool {
                question_pool_id: pool.question_pool_id().clone(),
                question_pool_edit_number: pool.question_pool_edit_number(),
                selection_count: pool.selection_count(),
                points_per_item: pool.points_per_item(),
                scoring_rule: pool.scoring_rule(),
                selection_rule: pool.selection_rule(),
                question_attempt_limit: *pool.question_attempt_limit(),
                question_attempt_time_limit: *pool.question_attempt_time_limit(),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroU32;

    use super::*;
    use crate::{
        AssessmentActivityRules, AssessmentTitle, BlueprintAssessmentContent,
        BlueprintAssessmentId, BlueprintCourseModuleContent, BlueprintModuleId,
        BlueprintQuestionPoolContent, LateWorkRule, QuestionPoolEditNumber,
        QuestionPoolSelectedQuestionOrder, QuestionRevisionNumber, ReusablePoolView,
        StudentFeedbackReleaseRule,
    };
    use uuid::Uuid;

    #[test]
    fn strict_exchange_recreates_the_ordered_create_command_semantically() {
        // Regression contract: import must preserve reusable meaning while
        // refusing transferred authority. On failure, repair this projection
        // or conversion; never admit owner or operational fields.
        let fixed = QuestionRevisionTuple {
            question_id: "7K3M-19QX".parse().expect("Question ID"),
            revision_number: QuestionRevisionNumber::new(2).expect("Question Revision"),
        };
        let question_pool_id: crate::QuestionId = "12A4-TBCZ".parse().expect("Pool ID");
        let question_pool_edit_number = QuestionPoolEditNumber::new(3).expect("Pool Edit Number");
        let defaults = BlueprintAssessmentDefaults {
            assessment_attempt_time_limit_seconds: NonZeroU32::new(900),
            attempt_limit: NonZeroU32::new(2),
            late_work_rule: LateWorkRule::MarkLate,
            activity_rules: AssessmentActivityRules::default(),
            student_feedback_release_rule: StudentFeedbackReleaseRule::default(),
        };
        let content = BlueprintCourseContent::new(vec![
            BlueprintCourseModuleContent::new(
                BlueprintModuleId::from_uuid(Uuid::from_u128(1)),
                "Module 1".to_owned(),
                vec![
                    BlueprintAssessmentContent::new(
                        BlueprintAssessmentId::from_uuid(Uuid::from_u128(2)),
                        crate::AssessmentType::Quiz,
                        AssessmentTitle::try_new("Structure check".to_owned()).expect("title"),
                        AssessmentInstructions::try_new("Explain each choice.".to_owned())
                            .expect("instructions"),
                        vec![
                            BlueprintAssessmentEntryContent::Fixed {
                                question_revision: fixed.clone(),
                                points_possible: AssessmentPointValue::from_whole(3),
                                scoring_rule: AssessmentEntryScoringRule::Normal,
                                question_attempt_limit: QuestionAttemptLimit {
                                    max_attempts: Some(2),
                                },
                                question_attempt_time_limit: QuestionAttemptTimeLimit::Unlimited,
                            },
                            BlueprintAssessmentEntryContent::Pool(
                                BlueprintQuestionPoolContent::new(ReusablePoolView {
                                    question_pool_id: question_pool_id.clone(),
                                    question_pool_edit_number,
                                    selection_count: NonZeroU32::new(2).expect("selection count"),
                                    points_per_item: AssessmentPointValue::from_whole(4),
                                    scoring_rule: AssessmentEntryScoringRule::ExtraCredit,
                                    selection_rule: QuestionPoolSelectionRule {
                                        selected_question_order:
                                            QuestionPoolSelectedQuestionOrder::QuestionPoolOrder,
                                    },
                                    question_attempt_limit: QuestionAttemptLimit {
                                        max_attempts: None,
                                    },
                                    question_attempt_time_limit:
                                        QuestionAttemptTimeLimit::Unlimited,
                                })
                                .expect("Pool content"),
                            ),
                        ],
                        defaults,
                    )
                    .expect("Assessment content"),
                ],
            )
            .expect("Module content"),
        ])
        .expect("Course content");
        let classification = crate::CourseClassification {
            discipline_uuid: Uuid::from_u128(3),
            subject_uuid: None,
            topic_uuid: None,
            subtopic_uuid: None,
            tags: Vec::new(),
        };
        let exchange = CanonicalBlueprintCourse::export(
            "BIO 101",
            "Biology 101",
            classification.clone(),
            &content,
        );
        let decoded: CanonicalBlueprintCourse =
            serde_json::from_slice(&exchange.json_bytes()).expect("strict canonical JSON");
        assert_eq!(decoded, exchange);

        let input = decoded.into_create_input().expect("valid create command");
        assert_eq!(input.classification, classification);
        assert_eq!(input.short_name, "BIO 101");
        assert_eq!(input.long_name, "Biology 101");
        assert_eq!(input.modules[0].label, "Module 1");
        assert_eq!(input.modules[0].assessments[0].entries.len(), 2);
        assert!(matches!(
            &input.modules[0].assessments[0].entries[0],
            BlueprintAssessmentEntryInput::Fixed(value)
                if value.published_question == fixed
        ));
        assert!(matches!(
            &input.modules[0].assessments[0].entries[1],
            BlueprintAssessmentEntryInput::Pool(ReusablePoolInput {
                pool: BlueprintPoolInputChoice::Import {
                    question_pool_id: imported_pool_id,
                    question_pool_edit_number: imported_edit_number,
                },
                ..
            }) if imported_pool_id.as_str() == "12A4-TBCZ"
                && imported_edit_number.get() == 3
        ));

        let mut injected = serde_json::to_value(exchange).expect("canonical value");
        injected
            .as_object_mut()
            .expect("exchange object")
            .insert("owner".to_owned(), serde_json::json!("not accepted"));
        assert!(serde_json::from_value::<CanonicalBlueprintCourse>(injected).is_err());

        let mut injected = serde_json::to_value(CanonicalBlueprintCourse::export(
            "BIO 101",
            "Biology 101",
            classification.clone(),
            &content,
        ))
        .expect("canonical value");
        injected
            .pointer_mut("/modules/0/assessments/0/entries/0/question_attempt_time_limit")
            .and_then(serde_json::Value::as_object_mut)
            .expect("Question Attempt time limit")
            .insert("seconds".to_owned(), serde_json::json!(30));
        assert!(serde_json::from_value::<CanonicalBlueprintCourse>(injected).is_err());

        let mut injected = serde_json::to_value(CanonicalBlueprintCourse::export(
            "BIO 101",
            "Biology 101",
            classification,
            &content,
        ))
        .expect("canonical value");
        injected
            .pointer_mut("/modules/0/assessments/0/entries/0/published_question")
            .and_then(serde_json::Value::as_object_mut)
            .expect("Question Revision Tuple")
            .insert("owner".to_owned(), serde_json::json!("not accepted"));
        assert!(serde_json::from_value::<CanonicalBlueprintCourse>(injected).is_err());
    }
}
