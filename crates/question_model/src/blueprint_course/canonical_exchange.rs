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
    CreateBlueprintCourseInput, CreateBlueprintModuleInput, PublishedQuestionRevisionTuple,
    QuestionAttemptLimit, QuestionAttemptTimeLimit, ReusableFixedQuestionInput, ReusablePoolInput,
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
        Self::export_with_theme(
            short_name,
            long_name,
            classification,
            crate::Theme::default(),
            content,
        )
    }

    /// Projects reusable structure and its persisted visual identity.
    pub fn export_with_theme(
        short_name: impl Into<String>,
        long_name: impl Into<String>,
        classification: crate::CourseClassification,
        theme: crate::Theme,
        content: &BlueprintCourseContent,
    ) -> Self {
        Self {
            metadata: CanonicalBlueprintMetadata {
                short_name: short_name.into(),
                long_name: long_name.into(),
                classification,
                theme,
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
            theme: self.metadata.theme,
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
    theme: crate::Theme,
}

impl CanonicalBlueprintMetadata {
    /// Explicit current metadata; UUID identities are installation-local.
    pub fn classification(&self) -> &crate::CourseClassification {
        &self.classification
    }

    /// Current visual identity carried into a fork or import.
    pub fn theme(&self) -> crate::Theme {
        self.theme
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
        #[serde(deserialize_with = "deserialize_published_question_revision_tuple")]
        published_question_revision_tuple: PublishedQuestionRevisionTuple,
        points_possible: AssessmentPointValue,
        scoring_rule: AssessmentEntryScoringRule,
        question_attempt_limit: QuestionAttemptLimit,
        #[serde(deserialize_with = "deserialize_question_attempt_time_limit")]
        question_attempt_time_limit: QuestionAttemptTimeLimit,
    },
    /// One current Question Pool with reusable selection settings.
    Pool {
        question_pool_id: crate::QuestionPoolId,
        selection_count: std::num::NonZeroU32,
        points_per_item: AssessmentPointValue,
        scoring_rule: AssessmentEntryScoringRule,
        question_attempt_limit: QuestionAttemptLimit,
        #[serde(deserialize_with = "deserialize_question_attempt_time_limit")]
        question_attempt_time_limit: QuestionAttemptTimeLimit,
    },
}

fn deserialize_published_question_revision_tuple<'de, D>(
    deserializer: D,
) -> Result<PublishedQuestionRevisionTuple, D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase", deny_unknown_fields)]
    struct StrictPublishedQuestionRevisionTuple {
        published_question_id: crate::PublishedQuestionId,
        revision_number: crate::QuestionRevisionNumber,
    }

    let tuple = StrictPublishedQuestionRevisionTuple::deserialize(deserializer)?;
    Ok(PublishedQuestionRevisionTuple {
        published_question_id: tuple.published_question_id,
        revision_number: tuple.revision_number,
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
                published_question_revision_tuple,
                points_possible,
                scoring_rule,
                question_attempt_limit,
                question_attempt_time_limit,
            } => BlueprintAssessmentEntryInput::Fixed(ReusableFixedQuestionInput {
                published_question_revision_tuple,
                points_possible,
                scoring_rule,
                question_attempt_limit,
                question_attempt_time_limit,
            }),
            Self::Pool {
                question_pool_id,
                selection_count,
                points_per_item,
                scoring_rule,
                question_attempt_limit,
                question_attempt_time_limit,
            } => BlueprintAssessmentEntryInput::Pool(ReusablePoolInput {
                question_pool_id,
                selection_count,
                points_per_item,
                scoring_rule,
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
                published_question_revision_tuple,
                points_possible,
                scoring_rule,
                question_attempt_limit,
                question_attempt_time_limit,
            } => Self::Fixed {
                published_question_revision_tuple: published_question_revision_tuple.clone(),
                points_possible: *points_possible,
                scoring_rule: *scoring_rule,
                question_attempt_limit: *question_attempt_limit,
                question_attempt_time_limit: *question_attempt_time_limit,
            },
            BlueprintAssessmentEntryContent::Pool(pool) => Self::Pool {
                question_pool_id: pool.question_pool_id().clone(),
                selection_count: pool.selection_count(),
                points_per_item: pool.points_per_item(),
                scoring_rule: pool.scoring_rule(),
                question_attempt_limit: *pool.question_attempt_limit(),
                question_attempt_time_limit: *pool.question_attempt_time_limit(),
            },
        }
    }
}
