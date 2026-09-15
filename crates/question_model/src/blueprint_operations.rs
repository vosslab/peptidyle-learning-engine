//! Validated reusable Blueprint Revision Content.

use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::{
    AssessmentEntryScoringRule, AssessmentInstructions, AssessmentPointValue, AssessmentTitle,
    BlueprintAssessmentDefaults, BlueprintAssessmentReference, BlueprintCourseValidationError,
    BlueprintModuleReference, MAX_ASSESSMENT_ORDERED_ENTRIES, QuestionAttemptLimit,
    QuestionAttemptTimeLimit, QuestionRevisionReference, validate_blueprint_course_title,
};

mod contracts;

pub use contracts::*;

const DOMAIN: &[u8] = b"ple:blueprint-revision-content\0";

#[derive(Debug, Clone, PartialEq)]
/// Validated Blueprint Revision Content stored independently from operation evidence.
pub enum BlueprintRevisionContent {
    /// One Blueprint Assessment Content record.
    Assessment(Box<BlueprintAssessmentContent>),
    /// One Blueprint Course Content record.
    Course(BlueprintCourseContent),
}

impl BlueprintRevisionContent {
    /// Wraps one validated Blueprint Assessment Content record.
    pub fn assessment(value: BlueprintAssessmentContent) -> Self {
        Self::Assessment(Box::new(value))
    }
    /// Wraps one validated Blueprint Course Content record.
    pub fn course(value: BlueprintCourseContent) -> Self {
        Self::Course(value)
    }
    /// Produces the one validated persistence record for this Blueprint Revision Content.
    pub fn encoding_record(&self) -> BlueprintRevisionContentRecord {
        let encoded_bytes = deterministic_encoded_bytes(self);
        let checksum = BlueprintContentChecksum(Sha256::digest(&encoded_bytes).into());
        BlueprintRevisionContentRecord {
            encoded_bytes,
            checksum,
        }
    }
    /// Computes the canonical Blueprint Revision Content checksum.
    pub fn checksum(&self) -> BlueprintContentChecksum {
        self.encoding_record().checksum()
    }
    /// Checks Blueprint Revision Content and reports both checksums when it changed.
    pub fn compare(&self, other: &Self) -> BlueprintContentCheck {
        if self == other {
            BlueprintContentCheck::Equivalent {
                checksum: self.checksum(),
            }
        } else {
            BlueprintContentCheck::Changed {
                expected: self.checksum(),
                actual: other.checksum(),
            }
        }
    }
}

/// One validated Blueprint Assessment with trusted immutable question pins.
#[derive(Debug, Clone, PartialEq)]
pub struct BlueprintAssessmentContent {
    blueprint_assessment_reference: BlueprintAssessmentReference,
    assessment_type: crate::AssessmentType,
    title: AssessmentTitle,
    instructions: AssessmentInstructions,
    entries: Vec<BlueprintAssessmentEntryContent>,
    defaults: BlueprintAssessmentDefaults,
}
impl BlueprintAssessmentContent {
    /// Validates all Blueprint Assessment meaning before constructing a baseline.
    pub fn new(
        blueprint_assessment_reference: BlueprintAssessmentReference,
        assessment_type: crate::AssessmentType,
        title: AssessmentTitle,
        instructions: AssessmentInstructions,
        entries: Vec<BlueprintAssessmentEntryContent>,
        defaults: BlueprintAssessmentDefaults,
    ) -> Result<Self, BlueprintCourseValidationError> {
        if entries.is_empty() || entries.len() > MAX_ASSESSMENT_ORDERED_ENTRIES {
            return Err(BlueprintCourseValidationError::InvalidEntryCount);
        }
        defaults.validate()?;
        Ok(Self {
            blueprint_assessment_reference,
            assessment_type,
            title,
            instructions,
            entries,
            defaults,
        })
    }
    /// Returns the stable Blueprint Assessment identity retained across Revisions.
    pub fn blueprint_assessment_reference(&self) -> BlueprintAssessmentReference {
        self.blueprint_assessment_reference
    }
    /// Returns the fixed pedagogical purpose retained across Blueprint Revisions.
    pub fn assessment_type(&self) -> crate::AssessmentType {
        self.assessment_type
    }
    /// Returns the Blueprint Assessment title.
    pub fn title(&self) -> &str {
        self.title.as_str()
    }
    /// Returns the student-facing reusable instructions.
    pub fn instructions(&self) -> &AssessmentInstructions {
        &self.instructions
    }
    /// Returns the validated reusable policy defaults.
    pub fn defaults(&self) -> &BlueprintAssessmentDefaults {
        &self.defaults
    }
    /// Returns fixed questions and pools in meaningful authored order.
    pub fn entries(&self) -> &[BlueprintAssessmentEntryContent] {
        &self.entries
    }
}

/// One validated labelled module in Blueprint Course Content.
#[derive(Debug, Clone, PartialEq)]
pub struct BlueprintCourseModuleContent {
    blueprint_module_reference: BlueprintModuleReference,
    label: String,
    assessments: Vec<BlueprintAssessmentContent>,
}
impl BlueprintCourseModuleContent {
    /// Validates a module label and its nonempty ordered assessments.
    pub fn new(
        blueprint_module_reference: BlueprintModuleReference,
        label: String,
        assessments: Vec<BlueprintAssessmentContent>,
    ) -> Result<Self, BlueprintCourseValidationError> {
        validate_blueprint_course_title(&label)
            .map_err(|_| BlueprintCourseValidationError::InvalidModuleLabel)?;
        if assessments.is_empty() || assessments.len() > MAX_ASSESSMENT_ORDERED_ENTRIES {
            return Err(BlueprintCourseValidationError::InvalidModuleAssessmentCount);
        }
        Ok(Self {
            blueprint_module_reference,
            label,
            assessments,
        })
    }
    /// Returns the stable Blueprint Module identity retained across Revisions.
    pub fn blueprint_module_reference(&self) -> BlueprintModuleReference {
        self.blueprint_module_reference
    }
    /// Returns the reusable module label.
    pub fn label(&self) -> &str {
        &self.label
    }
    /// Returns Blueprint Assessments in meaningful authored order.
    pub fn assessments(&self) -> &[BlueprintAssessmentContent] {
        &self.assessments
    }
}

/// One validated Blueprint Course Content record.
#[derive(Debug, Clone, PartialEq)]
pub struct BlueprintCourseContent {
    modules: Vec<BlueprintCourseModuleContent>,
}
impl BlueprintCourseContent {
    /// Validates nonempty ordered reusable modules.
    pub fn new(
        modules: Vec<BlueprintCourseModuleContent>,
    ) -> Result<Self, BlueprintCourseValidationError> {
        if modules.is_empty() || modules.len() > MAX_ASSESSMENT_ORDERED_ENTRIES {
            return Err(BlueprintCourseValidationError::InvalidModuleCount);
        }
        Ok(Self { modules })
    }
    /// Returns reusable modules in meaningful authored order.
    pub fn modules(&self) -> &[BlueprintCourseModuleContent] {
        &self.modules
    }
}

/// One ordered Blueprint Assessment entry containing only trusted exact pins.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BlueprintAssessmentEntryContent {
    /// One fixed immutable Question Revision and its scoring rule.
    Fixed {
        /// Exact immutable publication pin authorized for the destination.
        reference: QuestionRevisionReference,
        /// Exact points copied into the destination assessment.
        points_possible: AssessmentPointValue,
        /// Scoring treatment copied into the destination assessment.
        scoring_rule: AssessmentEntryScoringRule,
        /// Question Attempt retry bound copied into the destination assessment.
        question_attempt_limit: QuestionAttemptLimit,
        /// Question Attempt timing copied into the destination assessment.
        question_attempt_time_limit: QuestionAttemptTimeLimit,
    },
    /// One validated deterministic item pool.
    Pool(BlueprintQuestionPoolContent),
}
/// One validated exact immutable Pool Revision selected for future adoption.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlueprintQuestionPoolContent {
    question_pool_revision: crate::QuestionPoolRevisionReference,
    selection_count: std::num::NonZeroU32,
    points_per_item: AssessmentPointValue,
    scoring_rule: AssessmentEntryScoringRule,
    selection_rule: crate::QuestionPoolSelectionRule,
    question_attempt_limit: QuestionAttemptLimit,
    question_attempt_time_limit: QuestionAttemptTimeLimit,
}
impl BlueprintQuestionPoolContent {
    /// Validates the exact immutable Pool Revision and entry-owned selection count.
    pub fn new(
        question_pool_revision: crate::QuestionPoolRevisionReference,
        selection_count: std::num::NonZeroU32,
        points_per_item: AssessmentPointValue,
        scoring_rule: AssessmentEntryScoringRule,
        selection_rule: crate::QuestionPoolSelectionRule,
        question_attempt_limit: QuestionAttemptLimit,
        question_attempt_time_limit: QuestionAttemptTimeLimit,
    ) -> Result<Self, BlueprintCourseValidationError> {
        Ok(Self {
            question_pool_revision,
            selection_count,
            points_per_item,
            scoring_rule,
            selection_rule,
            question_attempt_limit,
            question_attempt_time_limit,
        })
    }
    /// Returns the exact immutable Pool Revision.
    pub fn question_pool_revision(&self) -> &crate::QuestionPoolRevisionReference {
        &self.question_pool_revision
    }
    /// Returns the number of Question Pool Items selected for one Assessment Attempt.
    pub fn selection_count(&self) -> std::num::NonZeroU32 {
        self.selection_count
    }
    /// Returns the point value assigned to every selected Question.
    pub fn points_per_item(&self) -> AssessmentPointValue {
        self.points_per_item
    }
    /// Returns the scoring treatment copied to every selected Question.
    pub fn scoring_rule(&self) -> AssessmentEntryScoringRule {
        self.scoring_rule
    }
    /// Returns the complete reviewed selection behavior.
    pub fn selection_rule(&self) -> crate::QuestionPoolSelectionRule {
        self.selection_rule
    }
    /// Returns the uniform Question Attempt retry bound copied to selected Questions.
    pub fn question_attempt_limit(&self) -> &QuestionAttemptLimit {
        &self.question_attempt_limit
    }
    /// Returns the uniform Question Attempt timing copied to selected Questions.
    pub fn question_attempt_time_limit(&self) -> &QuestionAttemptTimeLimit {
        &self.question_attempt_time_limit
    }
}
/// Full server-side SHA-256 binding of encoded Blueprint Revision Content.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BlueprintContentChecksum([u8; 32]);
impl BlueprintContentChecksum {
    /// Returns all persistence bytes without truncation.
    pub fn as_bytes(self) -> [u8; 32] {
        self.0
    }
}

/// Validated server-side encoded Blueprint Revision Content bytes and checksum.
///
/// Construction remains payload-owned so arbitrary persisted bytes cannot be
/// mistaken for normalized qmodel meaning.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlueprintRevisionContentRecord {
    encoded_bytes: Vec<u8>,
    checksum: BlueprintContentChecksum,
}

impl BlueprintRevisionContentRecord {
    /// Borrows the complete domain-separated Blueprint Revision Content encoding.
    pub fn encoded_bytes(&self) -> &[u8] {
        &self.encoded_bytes
    }

    /// Returns the complete SHA-256 checksum of `encoded_bytes`.
    pub const fn checksum(&self) -> BlueprintContentChecksum {
        self.checksum
    }
}
/// Blueprint Revision Content check used by fast-forward and divergence decisions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlueprintContentCheck {
    /// Both Blueprint Revision Content values contain exactly the same content.
    Equivalent {
        /// Shared checksum recorded with the equivalent baseline.
        checksum: BlueprintContentChecksum,
    },
    /// At least one Blueprint Revision Content field differs.
    Changed {
        /// Checksum of the observed Blueprint Revision Content.
        expected: BlueprintContentChecksum,
        /// Checksum of the proposed or current Blueprint Revision Content.
        actual: BlueprintContentChecksum,
    },
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
struct EncodedPayload<'a> {
    meaning: EncodedMeaning<'a>,
}

#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum EncodedMeaning<'a> {
    Assessment { content: EncodedAssessment<'a> },
    Course { modules: Vec<EncodedModule<'a>> },
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
struct EncodedModule<'a> {
    blueprint_module_reference: BlueprintModuleReference,
    label: &'a str,
    assessments: Vec<EncodedAssessment<'a>>,
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
struct EncodedAssessment<'a> {
    blueprint_assessment_reference: BlueprintAssessmentReference,
    assessment_type: crate::AssessmentType,
    title: &'a str,
    instructions: &'a AssessmentInstructions,
    entries: Vec<EncodedEntry<'a>>,
    defaults: &'a BlueprintAssessmentDefaults,
}

#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum EncodedEntry<'a> {
    Fixed {
        reference: &'a QuestionRevisionReference,
        points_possible: AssessmentPointValue,
        scoring_rule: AssessmentEntryScoringRule,
        question_attempt_limit: &'a QuestionAttemptLimit,
        question_attempt_time_limit: &'a QuestionAttemptTimeLimit,
    },
    Pool {
        question_pool_revision: &'a crate::QuestionPoolRevisionReference,
        selection_count: std::num::NonZeroU32,
        points_per_item: AssessmentPointValue,
        scoring_rule: AssessmentEntryScoringRule,
        selection_rule: crate::QuestionPoolSelectionRule,
        question_attempt_limit: &'a QuestionAttemptLimit,
        question_attempt_time_limit: &'a QuestionAttemptTimeLimit,
    },
}

fn deterministic_encoded_bytes(payload: &BlueprintRevisionContent) -> Vec<u8> {
    let meaning = match payload {
        BlueprintRevisionContent::Assessment(assessment) => EncodedMeaning::Assessment {
            content: encode_assessment(assessment),
        },
        BlueprintRevisionContent::Course(course) => EncodedMeaning::Course {
            modules: course
                .modules()
                .iter()
                .map(|module| EncodedModule {
                    blueprint_module_reference: module.blueprint_module_reference(),
                    label: module.label(),
                    assessments: module.assessments().iter().map(encode_assessment).collect(),
                })
                .collect(),
        },
    };
    let json = serde_json::to_vec(&EncodedPayload { meaning })
        .expect("validated private Blueprint Revision Content serializes");
    let mut bytes = Vec::with_capacity(DOMAIN.len() + json.len());
    bytes.extend_from_slice(DOMAIN);
    bytes.extend_from_slice(&json);
    bytes
}

fn encode_assessment(assessment: &BlueprintAssessmentContent) -> EncodedAssessment<'_> {
    EncodedAssessment {
        blueprint_assessment_reference: assessment.blueprint_assessment_reference(),
        assessment_type: assessment.assessment_type(),
        title: assessment.title(),
        instructions: assessment.instructions(),
        entries: assessment
            .entries()
            .iter()
            .map(|entry| match entry {
                BlueprintAssessmentEntryContent::Fixed {
                    reference,
                    points_possible,
                    scoring_rule,
                    question_attempt_limit,
                    question_attempt_time_limit,
                } => EncodedEntry::Fixed {
                    reference,
                    points_possible: *points_possible,
                    scoring_rule: *scoring_rule,
                    question_attempt_limit,
                    question_attempt_time_limit,
                },
                BlueprintAssessmentEntryContent::Pool(pool) => EncodedEntry::Pool {
                    question_pool_revision: &pool.question_pool_revision,
                    selection_count: pool.selection_count,
                    points_per_item: pool.points_per_item,
                    scoring_rule: pool.scoring_rule,
                    selection_rule: pool.selection_rule,
                    question_attempt_limit: &pool.question_attempt_limit,
                    question_attempt_time_limit: &pool.question_attempt_time_limit,
                },
            })
            .collect(),
        defaults: assessment.defaults(),
    }
}

#[cfg(test)]
mod wire_tests {
    use super::*;

    #[test]
    fn question_pool_keeps_the_exact_pool_revision() {
        let pool_revision = crate::QuestionPoolRevisionReference {
            question_pool_id: "7K3M-X9QX".parse().expect("Pool ID"),
            revision_number: crate::QuestionPoolRevisionNumber::new(1).expect("Pool Revision"),
        };
        let pool = BlueprintQuestionPoolContent::new(
            pool_revision.clone(),
            std::num::NonZeroU32::new(1).expect("positive count"),
            AssessmentPointValue::from_whole(1),
            AssessmentEntryScoringRule::Normal,
            crate::QuestionPoolSelectionRule {
                selected_question_order:
                    crate::QuestionPoolSelectedQuestionOrder::QuestionPoolOrder,
            },
            QuestionAttemptLimit { max_attempts: None },
            QuestionAttemptTimeLimit::Unlimited,
        )
        .expect("one exact Pool Revision is valid");

        assert_eq!(pool.question_pool_revision(), &pool_revision);
    }
}
