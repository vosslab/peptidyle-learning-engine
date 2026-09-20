//! Instructor-owned reusable Assessment settings.
//!
//! An Assessment Template is copied by value when an Instructor creates a
//! Course Instance Assessment. It contains no Course, Blueprint, schedule,
//! lifecycle, Question, Question Pool, or Assessment Entry state.

use std::num::{NonZeroU32, NonZeroU64};
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    AssessmentActivityRules, AssessmentInstructions, AssessmentType, BaseAssessmentPolicy,
    LateWorkRule, MAX_ASSESSMENT_ATTEMPT_LIMIT, MAX_ASSESSMENT_ATTEMPT_TIME_LIMIT_SECONDS,
    MAX_ASSESSMENT_TITLE_UNICODE_SCALARS, StudentFeedbackReleaseRule,
};

/// Largest accepted Assessment Template Name, measured in Unicode scalars.
pub const MAX_ASSESSMENT_TEMPLATE_NAME_UNICODE_SCALARS: usize =
    MAX_ASSESSMENT_TITLE_UNICODE_SCALARS;

/// Private server-generated identity for one Assessment Template.
///
/// This UUID is an authenticated private-API identity, not a human-facing
/// public ID.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct AssessmentTemplateId(Uuid);

impl AssessmentTemplateId {
    /// Rebuilds an identity read from trusted storage.
    pub const fn from_uuid(value: Uuid) -> Self {
        Self(value)
    }

    /// Returns the UUID used by trusted storage and private APIs.
    pub const fn as_uuid(self) -> Uuid {
        self.0
    }

    /// Mints a fresh server-owned Assessment Template identity.
    #[cfg(feature = "generate")]
    pub fn generate() -> Self {
        Self(Uuid::now_v7())
    }
}

impl std::fmt::Display for AssessmentTemplateId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(formatter)
    }
}

/// Validation failure for one Assessment Template Name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssessmentTemplateNameError {
    /// A name must contain visible text after trimming.
    Blank,
    /// A name must not include leading or trailing whitespace.
    NotTrimmed,
    /// A name exceeds the shared Assessment title bound.
    TooLong,
}

impl std::fmt::Display for AssessmentTemplateNameError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Blank => {
                formatter.write_str("assessment template name must contain non-whitespace text")
            }
            Self::NotTrimmed => {
                formatter.write_str("assessment template name must not have surrounding whitespace")
            }
            Self::TooLong => {
                formatter.write_str("assessment template name exceeds the maximum length")
            }
        }
    }
}

impl std::error::Error for AssessmentTemplateNameError {}

/// Validated Instructor-facing Assessment Template Name.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(transparent)]
pub struct AssessmentTemplateName(String);

impl AssessmentTemplateName {
    /// Validates a bounded, trimmed Assessment Template Name.
    pub fn try_new(value: String) -> Result<Self, AssessmentTemplateNameError> {
        if value.trim().is_empty() {
            return Err(AssessmentTemplateNameError::Blank);
        }
        if value != value.trim() {
            return Err(AssessmentTemplateNameError::NotTrimmed);
        }
        if value.chars().count() > MAX_ASSESSMENT_TEMPLATE_NAME_UNICODE_SCALARS {
            return Err(AssessmentTemplateNameError::TooLong);
        }
        Ok(Self(value))
    }

    /// Returns the exact validated display name.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for AssessmentTemplateName {
    type Error = AssessmentTemplateNameError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::try_new(value)
    }
}

impl From<AssessmentTemplateName> for String {
    fn from(value: AssessmentTemplateName) -> Self {
        value.0
    }
}

impl<'de> Deserialize<'de> for AssessmentTemplateName {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        String::deserialize(deserializer)
            .and_then(|value| Self::try_new(value).map_err(serde::de::Error::custom))
    }
}

/// Positive compare-and-swap number for one replaceable Assessment Template.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct AssessmentTemplateEditNumber(NonZeroU64);

impl AssessmentTemplateEditNumber {
    /// First Edit Number for a newly created Assessment Template.
    pub const INITIAL: Self = Self(NonZeroU64::MIN);

    /// Rebuilds a positive Edit Number that fits PostgreSQL `BIGINT`.
    pub fn new(value: u64) -> Option<Self> {
        (value > 0 && value <= i64::MAX as u64).then_some(Self(NonZeroU64::new(value)?))
    }

    /// Returns the exact positive persistence value.
    pub const fn value(self) -> u64 {
        self.0.get()
    }

    /// Advances one successful Assessment Template replacement.
    pub fn checked_next(self) -> Option<Self> {
        Self::new(self.value().checked_add(1)?)
    }
}

impl FromStr for AssessmentTemplateEditNumber {
    type Err = AssessmentTemplateEditNumberError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        if value.is_empty()
            || value.starts_with('0')
            || !value.bytes().all(|byte| byte.is_ascii_digit())
        {
            return Err(AssessmentTemplateEditNumberError);
        }
        value
            .parse::<u64>()
            .ok()
            .and_then(Self::new)
            .ok_or(AssessmentTemplateEditNumberError)
    }
}

impl TryFrom<String> for AssessmentTemplateEditNumber {
    type Error = AssessmentTemplateEditNumberError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        value.parse()
    }
}

impl From<AssessmentTemplateEditNumber> for String {
    fn from(value: AssessmentTemplateEditNumber) -> Self {
        value.to_string()
    }
}

impl std::fmt::Display for AssessmentTemplateEditNumber {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.value().fmt(formatter)
    }
}

/// An Edit Number was not one canonical positive PostgreSQL-`BIGINT` decimal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AssessmentTemplateEditNumberError;

impl std::fmt::Display for AssessmentTemplateEditNumberError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("assessment template edit number must be a canonical positive decimal")
    }
}

impl std::error::Error for AssessmentTemplateEditNumberError {}

/// The closed reusable settings copied into a new Course Instance Assessment.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AssessmentTemplateSettings {
    /// Validated student-facing plain-text instructions.
    pub instructions: AssessmentInstructions,
    /// Explicit duration override; None copies the content-based default intent.
    #[serde(deserialize_with = "deserialize_assessment_attempt_time_limit_seconds")]
    pub assessment_attempt_time_limit_seconds: Option<NonZeroU32>,
    /// Maximum number of Assessment Attempts when one applies.
    #[serde(deserialize_with = "deserialize_attempt_limit")]
    pub attempt_limit: Option<NonZeroU32>,
    /// Treatment of work after the ordinary due instant.
    pub late_work_rule: LateWorkRule,
    /// Complete independent Assessment activity behavior.
    pub activity_rules: AssessmentActivityRules,
    /// Independent Student-facing feedback release behavior.
    pub student_feedback_release_rule: StudentFeedbackReleaseRule,
}

/// A Template's settings disagree with its fixed Assessment Type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssessmentTemplateSettingsError {
    /// Quiz and Exam Templates permit exactly one Assessment Attempt.
    QuizExamAttemptLimitMustBeOne,
}

impl std::fmt::Display for AssessmentTemplateSettingsError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::QuizExamAttemptLimitMustBeOne => {
                formatter.write_str("Quiz and Exam templates must permit exactly one attempt")
            }
        }
    }
}

impl std::error::Error for AssessmentTemplateSettingsError {}

fn deserialize_assessment_attempt_time_limit_seconds<'de, D>(
    deserializer: D,
) -> Result<Option<NonZeroU32>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    deserialize_optional_bounded_non_zero_u32(
        deserializer,
        MAX_ASSESSMENT_ATTEMPT_TIME_LIMIT_SECONDS,
    )
}

fn deserialize_attempt_limit<'de, D>(deserializer: D) -> Result<Option<NonZeroU32>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    deserialize_optional_bounded_non_zero_u32(deserializer, MAX_ASSESSMENT_ATTEMPT_LIMIT)
}

fn deserialize_optional_bounded_non_zero_u32<'de, D>(
    deserializer: D,
    maximum: u32,
) -> Result<Option<NonZeroU32>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    // ASVS 1.5.2 and 2.2.1: accept only values that fit the persisted INTEGER contract.
    Option::<NonZeroU32>::deserialize(deserializer).and_then(|value| match value {
        None => Ok(None),
        Some(limit) if limit.get() <= maximum => Ok(Some(limit)),
        Some(_) => Err(serde::de::Error::custom(
            "assessment attempt limit exceeds the supported range",
        )),
    })
}

impl AssessmentTemplateSettings {
    /// Creates the canonical new-Assessment settings for one Assessment Type.
    pub fn for_assessment_type(assessment_type: AssessmentType) -> Self {
        let base_policy = BaseAssessmentPolicy::default();
        Self {
            instructions: AssessmentInstructions::default(),
            assessment_attempt_time_limit_seconds: base_policy
                .assessment_attempt_time_limit_seconds,
            attempt_limit: match assessment_type {
                AssessmentType::Quiz | AssessmentType::Exam => Some(NonZeroU32::MIN),
                AssessmentType::RegularAssignment
                | AssessmentType::PracticeQuestionAssignment
                | AssessmentType::BonusAssignment => base_policy.attempt_limit,
            },
            late_work_rule: base_policy.late_work_rule,
            activity_rules: AssessmentActivityRules::default(),
            student_feedback_release_rule: StudentFeedbackReleaseRule::for_assessment_type(
                assessment_type,
            ),
        }
    }

    /// Validates Type-owned invariants before a Template is stored or copied.
    ///
    /// # Errors
    ///
    /// Returns [`AssessmentTemplateSettingsError::QuizExamAttemptLimitMustBeOne`]
    /// when a Quiz or Exam Template does not permit exactly one Attempt.
    pub fn validate_for_assessment_type(
        &self,
        assessment_type: AssessmentType,
    ) -> Result<(), AssessmentTemplateSettingsError> {
        if matches!(assessment_type, AssessmentType::Quiz | AssessmentType::Exam)
            && self.attempt_limit != Some(NonZeroU32::MIN)
        {
            return Err(AssessmentTemplateSettingsError::QuizExamAttemptLimitMustBeOne);
        }
        Ok(())
    }
}

/// One current Instructor-owned reusable Assessment Template.
///
/// The owning Account remains an authorization and persistence concern; it is
/// deliberately absent from this private-API aggregate.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AssessmentTemplate {
    /// Private server-generated Template identity.
    pub id: AssessmentTemplateId,
    /// Instructor-facing Template Name.
    pub name: AssessmentTemplateName,
    /// Fixed pedagogical purpose selected for this Template.
    pub assessment_type: AssessmentType,
    /// Reusable settings copied by value into a new Assessment.
    pub settings: AssessmentTemplateSettings,
    /// Current compare-and-swap value for replacement.
    pub assessment_template_edit_number: AssessmentTemplateEditNumber,
}

impl AssessmentTemplate {
    /// Creates a new Template from the canonical defaults for its Assessment Type.
    pub fn new(
        id: AssessmentTemplateId,
        name: AssessmentTemplateName,
        assessment_type: AssessmentType,
    ) -> Self {
        Self {
            id,
            name,
            assessment_type,
            settings: AssessmentTemplateSettings::for_assessment_type(assessment_type),
            assessment_template_edit_number: AssessmentTemplateEditNumber::INITIAL,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quiz_and_exam_templates_permit_exactly_one_attempt() {
        for assessment_type in [AssessmentType::Quiz, AssessmentType::Exam] {
            let mut settings = AssessmentTemplateSettings::for_assessment_type(assessment_type);
            assert_eq!(settings.attempt_limit, Some(NonZeroU32::MIN));
            assert_eq!(
                settings.validate_for_assessment_type(assessment_type),
                Ok(())
            );

            settings.attempt_limit = None;
            assert_eq!(
                settings.validate_for_assessment_type(assessment_type),
                Err(AssessmentTemplateSettingsError::QuizExamAttemptLimitMustBeOne)
            );
            settings.attempt_limit = NonZeroU32::new(2);
            assert_eq!(
                settings.validate_for_assessment_type(assessment_type),
                Err(AssessmentTemplateSettingsError::QuizExamAttemptLimitMustBeOne)
            );
        }
    }

    #[test]
    fn regular_template_defaults_to_unlimited_attempts() {
        let settings =
            AssessmentTemplateSettings::for_assessment_type(AssessmentType::RegularAssignment);
        assert_eq!(settings.attempt_limit, None);
        assert_eq!(
            settings.validate_for_assessment_type(AssessmentType::RegularAssignment),
            Ok(())
        );
    }

    #[test]
    fn assessment_template_json_uses_assessment_template_edit_number() {
        let template = AssessmentTemplate::new(
            AssessmentTemplateId::from_uuid(Uuid::from_u128(1)),
            AssessmentTemplateName::try_from("Timed quiz".to_string()).expect("name"),
            AssessmentType::Quiz,
        );
        let wire = serde_json::to_value(&template).expect("template serializes");
        assert_eq!(wire["assessmentTemplateEditNumber"], "1");
        assert!(wire.get("editNumber").is_none());
        let mut leftover = wire.clone();
        leftover
            .as_object_mut()
            .expect("object")
            .insert("editNumber".to_string(), serde_json::json!("1"));
        leftover
            .as_object_mut()
            .expect("object")
            .remove("assessmentTemplateEditNumber");
        assert!(serde_json::from_value::<AssessmentTemplate>(leftover).is_err());
    }
}
