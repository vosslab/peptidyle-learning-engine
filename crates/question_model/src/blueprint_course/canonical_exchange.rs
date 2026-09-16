//! Canonical, answer-free reusable Blueprint Course exchange projection.
//!
//! This is an export representation, not persistence and not an import
//! command. It contains current lineage metadata and reusable teaching content:
//! names, classification, ordered module/Assessment structure, reusable settings,
//! and exact Published Question Revision pins.  It has no owner, lineage or
//! revision identifier, availability, Stars, Watches, Course Instance,
//! Student, delivery, or operational state.  A future import boundary must
//! authenticate and validate decoded JSON before creating a new private
//! Blueprint Course; it must not treat this projection as authority.

use serde::Serialize;

use crate::{
    AssessmentEntryScoringRule, AssessmentInstructions, AssessmentPointValue,
    BlueprintAssessmentDefaults, BlueprintAssessmentEntryContent, BlueprintCourseContent,
    QuestionAttemptLimit, QuestionAttemptTimeLimit, QuestionPoolSelectionRule,
    QuestionRevisionReference,
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
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct CanonicalBlueprintCourse {
    metadata: CanonicalBlueprintMetadata,
    modules: Vec<CanonicalBlueprintModule>,
}

impl CanonicalBlueprintCourse {
    /// Projects validated reusable content without any operational identity.
    ///
    /// The caller supplies current metadata from the selected Blueprint lineage. The
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
}

/// Explicit current Blueprint metadata accompanying reusable structure.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
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
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
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
}

/// One reusable Blueprint Assessment and its complete reusable settings.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
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
}

/// One fixed Published Question or one published Question Pool.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CanonicalBlueprintAssessmentEntry {
    /// One exact immutable Published Question Revision.
    Fixed {
        published_question: QuestionRevisionReference,
        points_possible: AssessmentPointValue,
        scoring_rule: AssessmentEntryScoringRule,
        question_attempt_limit: QuestionAttemptLimit,
        question_attempt_time_limit: QuestionAttemptTimeLimit,
    },
    /// One exact immutable Question Pool Revision with reusable selection settings.
    Pool {
        question_pool_revision: crate::QuestionPoolRevisionReference,
        selection_count: std::num::NonZeroU32,
        points_per_item: AssessmentPointValue,
        scoring_rule: AssessmentEntryScoringRule,
        selection_rule: QuestionPoolSelectionRule,
        question_attempt_limit: QuestionAttemptLimit,
        question_attempt_time_limit: QuestionAttemptTimeLimit,
    },
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
                reference,
                points_possible,
                scoring_rule,
                question_attempt_limit,
                question_attempt_time_limit,
            } => Self::Fixed {
                published_question: reference.clone(),
                points_possible: *points_possible,
                scoring_rule: *scoring_rule,
                question_attempt_limit: *question_attempt_limit,
                question_attempt_time_limit: *question_attempt_time_limit,
            },
            BlueprintAssessmentEntryContent::Pool(pool) => Self::Pool {
                question_pool_revision: pool.question_pool_revision().clone(),
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
