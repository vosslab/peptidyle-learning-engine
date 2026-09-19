//! Reusable Blueprint Assessment input and answer-free view contracts.

use std::collections::BTreeSet;
use std::num::NonZeroU32;

use serde::{Deserialize, Serialize};

use super::{BlueprintCourseValidationError, validate_blueprint_course_title};
use crate::{
    AssessmentActivityRules, AssessmentEntryScoringRule, AssessmentInstructions,
    AssessmentPointValue, LateWorkRule, MAX_ASSESSMENT_ATTEMPT_LIMIT,
    MAX_ASSESSMENT_ATTEMPT_TIME_LIMIT_SECONDS, MAX_ASSESSMENT_ORDERED_ENTRIES,
    QuestionAttemptLimit, QuestionAttemptTimeLimit, QuestionId, QuestionPoolEditNumber,
    QuestionPoolSelectionRule, QuestionRevisionReference, QuestionSearchResult,
    StudentFeedbackReleaseRule,
};

/// Blueprint Assessment policy defaults copied into a future teaching course.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct BlueprintAssessmentDefaults {
    /// Explicit duration override; None calculates the default from delivered Questions.
    pub assessment_attempt_time_limit_seconds: Option<NonZeroU32>,
    /// Number of Assessment Attempts, if the reusable content establishes one.
    #[serde(rename = "assessment_attempt_limit")]
    pub attempt_limit: Option<NonZeroU32>,
    /// Late-work treatment copied into the future assessment policy.
    pub late_work_rule: LateWorkRule,
    /// Independent Assessment Attempt behavior copied into the future assessment policy.
    pub activity_rules: AssessmentActivityRules,
    /// Student-release policy copied into the future assessment policy.
    #[serde(rename = "student_feedback_release_rule")]
    pub student_feedback_release_rule: StudentFeedbackReleaseRule,
}

impl BlueprintAssessmentDefaults {
    /// Validates reusable limits against the ordinary teaching-policy bounds.
    pub fn validate(&self) -> Result<(), BlueprintCourseValidationError> {
        if self
            .assessment_attempt_time_limit_seconds
            .is_some_and(|limit| limit.get() > MAX_ASSESSMENT_ATTEMPT_TIME_LIMIT_SECONDS)
        {
            return Err(BlueprintCourseValidationError::AssessmentAttemptTimeLimitOutOfRange);
        }
        if self
            .attempt_limit
            .is_some_and(|limit| limit.get() > MAX_ASSESSMENT_ATTEMPT_LIMIT)
        {
            return Err(BlueprintCourseValidationError::AttemptLimitOutOfRange);
        }
        Ok(())
    }
}

/// One Fixed Question Assessment Entry submitted in authored order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ReusableFixedQuestionInput {
    /// Exact published Question Revision checked under destination authority.
    pub published_question: QuestionRevisionReference,
    /// Points copied into the future Fixed Question Assessment Entry.
    pub points_possible: AssessmentPointValue,
    /// Score treatment copied into the future Fixed Question Assessment Entry.
    pub scoring_rule: AssessmentEntryScoringRule,
    /// Question Attempt retry bound copied into the future Fixed Question Assessment Entry.
    pub question_attempt_limit: QuestionAttemptLimit,
    /// Question Attempt timing copied into the future Fixed Question Assessment Entry.
    pub question_attempt_time_limit: QuestionAttemptTimeLimit,
}

/// Explicit source import or retained Assessment-owned Pool edit.
/// ASVS 1.5.2: only the reviewed input alternatives and fields are accepted.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum BlueprintPoolInputChoice {
    /// Copy the current source Pool membership into a fresh Assessment-owned Pool.
    Import {
        question_pool_id: QuestionId,
        question_pool_edit_number: QuestionPoolEditNumber,
    },
    /// Retain a Pool already owned by the Assessment being replaced.
    Retained {
        question_pool_id: QuestionId,
        question_pool_edit_number: QuestionPoolEditNumber,
        /// Null preserves members; an ordered list replaces current Pool membership.
        #[serde(deserialize_with = "deserialize_blueprint_pool_members")]
        members: Option<Vec<QuestionRevisionReference>>,
        /// Explicit interchangeability review for newly submitted member content.
        #[serde(rename = "interchangeabilityAttested")]
        interchangeability_attested: bool,
    },
}

fn deserialize_blueprint_pool_members<'de, D>(
    deserializer: D,
) -> Result<Option<Vec<QuestionRevisionReference>>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Option::<Vec<QuestionRevisionReference>>::deserialize(deserializer)
}

/// One Question Pool Assessment Entry with an explicit ownership operation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ReusablePoolInput {
    /// Exact source import or explicit retained Assessment-owned Pool edit.
    pub pool: BlueprintPoolInputChoice,
    /// Positive number of Pool members selected for each future Assessment Attempt.
    pub selection_count: NonZeroU32,
    /// Points copied for every selected Question Pool Item.
    pub points_per_item: AssessmentPointValue,
    /// Scoring rule copied for every selected Question Pool Item.
    pub scoring_rule: AssessmentEntryScoringRule,
    /// Complete reviewed selection behavior.
    pub selection_rule: QuestionPoolSelectionRule,
    /// Uniform Question Attempt retry bound copied for every selected Question Pool Item.
    pub question_attempt_limit: QuestionAttemptLimit,
    /// Uniform Question Attempt timing copied for every selected Question Pool Item.
    pub question_attempt_time_limit: QuestionAttemptTimeLimit,
}

impl ReusablePoolInput {
    fn validate(&self) -> Result<(), BlueprintCourseValidationError> {
        if let BlueprintPoolInputChoice::Retained {
            members: Some(members),
            interchangeability_attested,
            ..
        } = &self.pool
        {
            // ASVS 2.2.1, 2.2.3: bound related authored members and selection together.
            if members.is_empty()
                || members.len() > crate::MAX_QUESTION_POOL_ITEMS_PER_ASSESSMENT_ENTRY
            {
                return Err(BlueprintCourseValidationError::InvalidQuestionPoolItems);
            }
            if self.selection_count.get() as usize > members.len() {
                return Err(BlueprintCourseValidationError::InvalidPoolSelectionCount);
            }
            let mut questions = BTreeSet::new();
            if members
                .iter()
                .any(|member| !questions.insert(member.question_id.clone()))
            {
                return Err(BlueprintCourseValidationError::DuplicateQuestionPoolItem);
            }
            if !interchangeability_attested {
                return Err(
                    BlueprintCourseValidationError::QuestionPoolInterchangeabilityNotAttested,
                );
            }
        }
        Ok(())
    }
}

/// One ordered reusable content entry. Vector order is the only position.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum BlueprintAssessmentEntryInput {
    /// One Fixed Question Assessment Entry in content order.
    Fixed(ReusableFixedQuestionInput),
    /// One Question Pool Assessment Entry in content order.
    Pool(ReusablePoolInput),
}

/// Complete submitted Blueprint Assessment meaning.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct BlueprintAssessmentContentInput {
    /// Fixed pedagogical purpose shared with Course Instance Assessments.
    pub assessment_type: crate::AssessmentType,
    /// Instructor-facing title copied into future assessment assessments.
    pub title: String,
    /// Student-facing instructions copied into future assessment assessments.
    pub instructions: AssessmentInstructions,
    /// Fixed Question Assessment Entries and Question Pool Assessment Entries in authored order.
    pub entries: Vec<BlueprintAssessmentEntryInput>,
    /// Reusable delivery and Assessment Attempt defaults.
    pub defaults: BlueprintAssessmentDefaults,
}

impl BlueprintAssessmentContentInput {
    /// Validates bounded ordered entries and their reusable assessment meaning.
    pub fn validate(&self) -> Result<(), BlueprintCourseValidationError> {
        validate_blueprint_course_title(&self.title)
            .map_err(|_| BlueprintCourseValidationError::InvalidContentTitle)?;
        if self.entries.is_empty() || self.entries.len() > MAX_ASSESSMENT_ORDERED_ENTRIES {
            return Err(BlueprintCourseValidationError::InvalidEntryCount);
        }
        self.defaults.validate()?;
        // ASVS 2.2.1, 2.2.2: bound delivered Questions, not whole Pool membership.
        let question_count: u64 = self
            .entries
            .iter()
            .map(|entry| match entry {
                BlueprintAssessmentEntryInput::Fixed(_) => 1,
                BlueprintAssessmentEntryInput::Pool(pool) => u64::from(pool.selection_count.get()),
            })
            .sum();
        if question_count > 250 {
            return Err(BlueprintCourseValidationError::InvalidEntryCount);
        }
        for entry in &self.entries {
            if let BlueprintAssessmentEntryInput::Pool(pool) = entry {
                pool.validate()?;
            }
        }
        Ok(())
    }
}

/// Current selection status for an exact retained reusable question member.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReusableSelectionAvailability {
    /// The current publication remains selectable for a new content.
    Available,
    /// The pinned member remains inspectable but cannot be selected anew.
    Retained,
}

/// Answer-free view of one exact published Question Revision.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ReusableQuestionView {
    /// Exact stored published Question Revision; never inferred from a library head.
    pub reference: QuestionRevisionReference,
    /// Public Question Library metadata and disclosed evidence for the stored Revision.
    pub question_library: QuestionSearchResult,
    /// Whether the stored exact member remains selectable for a new copy.
    pub selection_availability: ReusableSelectionAvailability,
}

/// Current answer-free Reusable Pool View.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ReusablePoolView {
    pub question_pool_id: QuestionId,
    pub question_pool_edit_number: QuestionPoolEditNumber,
    /// Positive number of Pool members selected for each future Assessment Attempt.
    pub selection_count: NonZeroU32,
    /// Points copied for every selected Question Pool Item.
    pub points_per_item: AssessmentPointValue,
    /// Scoring rule copied for every selected Question Pool Item.
    pub scoring_rule: AssessmentEntryScoringRule,
    /// Complete reviewed selection behavior.
    pub selection_rule: QuestionPoolSelectionRule,
    /// Uniform Question Attempt retry bound copied for every selected Question Pool Item.
    pub question_attempt_limit: QuestionAttemptLimit,
    /// Uniform Question Attempt timing copied for every selected Question Pool Item.
    pub question_attempt_time_limit: QuestionAttemptTimeLimit,
}

/// Current answer-free reusable-content entry. Vector order is its position.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum BlueprintAssessmentEntryView {
    /// One Fixed Question Assessment Entry in content order.
    Fixed {
        /// Current answer-free Reusable Question View.
        question: Box<ReusableQuestionView>,
        /// Points copied into the future Fixed Question Assessment Entry.
        points_possible: AssessmentPointValue,
        /// Score treatment copied into the future Fixed Question Assessment Entry.
        scoring_rule: AssessmentEntryScoringRule,
        /// Question Attempt retry bound copied into the future Fixed Question Assessment Entry.
        question_attempt_limit: QuestionAttemptLimit,
        /// Question Attempt timing copied into the future Fixed Question Assessment Entry.
        question_attempt_time_limit: QuestionAttemptTimeLimit,
    },
    /// One Question Pool Assessment Entry in content order.
    Pool(ReusablePoolView),
}

/// Current answer-free Blueprint Assessment content.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct BlueprintAssessmentContentView {
    /// Fixed pedagogical purpose shared with Course Instance Assessments.
    pub assessment_type: crate::AssessmentType,
    /// Instructor-facing title copied into future assessment assessments.
    pub title: String,
    /// Student-facing instructions copied into future assessment assessments.
    pub instructions: AssessmentInstructions,
    /// Fixed Question Assessment Entries and Question Pool Assessment Entries in retained authored order.
    pub entries: Vec<BlueprintAssessmentEntryView>,
    /// Reusable delivery and Assessment Attempt defaults.
    pub defaults: BlueprintAssessmentDefaults,
}
