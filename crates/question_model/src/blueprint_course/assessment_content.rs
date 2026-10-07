//! Reusable Blueprint Assessment input and answer-free view contracts.

use std::num::NonZeroU32;

use serde::{Deserialize, Serialize};

use super::{BlueprintCourseValidationError, validate_blueprint_course_title};
use crate::{
    AssessmentActivityRules, AssessmentEntryScoringRule, AssessmentInstructions,
    AssessmentPointValue, LateWorkRule, MAX_ASSESSMENT_ATTEMPT_LIMIT,
    MAX_ASSESSMENT_ORDERED_ENTRIES, PublishedQuestionRevisionTuple, QuestionAttemptLimit,
    QuestionAttemptTimeLimit, QuestionPoolId, QuestionSearchResult, StudentFeedbackReleaseRule,
    is_valid_base_assessment_attempt_time_limit_seconds,
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
            .is_some_and(|limit| !is_valid_base_assessment_attempt_time_limit_seconds(limit.get()))
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
    pub published_question_revision_tuple: PublishedQuestionRevisionTuple,
    /// Points copied into the future Fixed Question Assessment Entry.
    pub points_possible: AssessmentPointValue,
    /// Score treatment copied into the future Fixed Question Assessment Entry.
    pub scoring_rule: AssessmentEntryScoringRule,
    /// Question Attempt retry bound copied into the future Fixed Question Assessment Entry.
    pub question_attempt_limit: QuestionAttemptLimit,
    /// Question Attempt timing copied into the future Fixed Question Assessment Entry.
    pub question_attempt_time_limit: QuestionAttemptTimeLimit,
}

/// One Question Pool reference with Assessment-local selection and scoring policy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ReusablePoolInput {
    /// Existing ordinary Pool used by this Blueprint Assessment.
    pub question_pool_id: QuestionPoolId,
    /// Positive number of Pool members selected for each future Assessment Attempt.
    pub selection_count: NonZeroU32,
    /// Points copied for every selected Question Pool Item.
    pub points_per_item: AssessmentPointValue,
    /// Scoring rule copied for every selected Question Pool Item.
    pub scoring_rule: AssessmentEntryScoringRule,
    /// Uniform Question Attempt retry bound copied for every selected Question Pool Item.
    pub question_attempt_limit: QuestionAttemptLimit,
    /// Uniform Question Attempt timing copied for every selected Question Pool Item.
    pub question_attempt_time_limit: QuestionAttemptTimeLimit,
}

impl ReusablePoolInput {
    fn validate(&self) -> Result<(), BlueprintCourseValidationError> {
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
    pub published_question_revision_tuple: PublishedQuestionRevisionTuple,
    /// Public Question Library metadata and disclosed evidence for the stored Revision.
    pub question_library: QuestionSearchResult,
    /// Whether the stored exact member remains selectable for a new copy.
    pub selection_availability: ReusableSelectionAvailability,
}

/// Current answer-free Reusable Pool View.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ReusablePoolView {
    pub question_pool_id: QuestionPoolId,
    /// Positive number of Pool members selected for each future Assessment Attempt.
    pub selection_count: NonZeroU32,
    /// Points copied for every selected Question Pool Item.
    pub points_per_item: AssessmentPointValue,
    /// Scoring rule copied for every selected Question Pool Item.
    pub scoring_rule: AssessmentEntryScoringRule,
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
