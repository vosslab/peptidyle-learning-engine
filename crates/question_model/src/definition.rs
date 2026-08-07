//! [`QuestionDefinition`], the backend-neutral representation every engine
//! maps into (WP-C1).
//!
//! One shared shape is what lets a WeBWorK question, a QTI item, an H5P
//! activity, and a first-party algorithmic question flow through the same
//! attempt loop, gradebook, and export path. Each backend adapter translates
//! into this type, and everything downstream reads only this type.
//!
//! The definition describes a question. It carries no answer key and no
//! grading material: [`GradingDefinition`] states *how* a response is judged,
//! while the values it is judged against live in `crates/grading`, server-side.

use serde::{Deserialize, Serialize};

use crate::envelope::ContentBlock;
use crate::generation::RandomizationDefinition;
use crate::identity::{ProblemId, VersionId, WorkspaceId};
use crate::response::ResponseDefinition;
use crate::run_policy::{AttemptPolicy, TimingPolicy};
use crate::taxonomy::{License, Tag, TaxonomyTerm};

/// Which engine a question came from, and how to find it there.
///
/// The reference stays with the question so an import can be repeated and an
/// export can point back at the original. Each variant carries exactly what its
/// backend needs to locate the source.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "backend",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum QuestionSource {
    /// A first-party algorithmic question.
    Native {
        /// Question family the native adapter dispatches on.
        family: String,
    },
    /// A WeBWorK PG problem, rendered by the renderer service.
    Webwork {
        /// Path within the problem library, for example an OPL path.
        pg_path: String,
    },
    /// An item from an imported QTI package.
    Qti {
        /// Item identifier within the package.
        item_id: String,
        /// Identifier of the archived original package.
        package_asset: String,
    },
    /// An imported H5P activity, which evaluates in the browser.
    H5p {
        /// H5P content type, for example `H5P.MultiChoice`.
        content_type: String,
    },
}

/// How a response is judged, without stating what the answer is.
///
/// Safe to send to a browser: a student can see that partial credit applies and
/// how a question is weighted, and still learn nothing about the answer.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "mode",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum GradingDefinition {
    /// Correct or incorrect, with no middle ground.
    AllOrNothing {
        /// Points awarded for a correct response.
        points: f64,
    },
    /// Credit proportional to how much of the response is correct.
    ///
    /// Requires the `partialCredit` capability; capability validation refuses
    /// an assignment whose backend lacks it, before publication.
    PartialCredit {
        /// Points awarded for a fully correct response.
        points: f64,
    },
    /// Practice with no recorded score.
    ///
    /// The honest declaration for an H5P activity, which evaluates in the
    /// browser and therefore cannot carry a graded assignment.
    Ungraded,
}

/// Descriptive information used for search, attribution, and reuse.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuestionMetadata {
    /// Title shown in the catalog and in printed exams.
    pub title: String,
    /// Free-form labels for search.
    pub tags: Vec<Tag>,
    /// Controlled-vocabulary terms that survive export.
    pub taxonomy: Vec<TaxonomyTerm>,
    /// Terms under which the content may be reused.
    pub license: License,
    /// BCP 47 language tag for the prompt, for example `en-US`.
    pub language: String,
}

/// A question, in the platform's own representation.
///
/// The `problem` field is what separates a draft from published content: a
/// draft lives in a workspace and has no [`ProblemId`], and publishing is the
/// transition that assigns one. Reading `problem.is_none()` is therefore the
/// same question as "is this a draft", with no separate flag to fall out of
/// sync.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuestionDefinition {
    /// This immutable version of the question.
    pub version: VersionId,
    /// The published problem this version belongs to, once published.
    pub problem: Option<ProblemId>,
    /// The workspace that authored it.
    pub workspace: WorkspaceId,
    /// Which engine it came from.
    pub source: QuestionSource,
    /// The prompt, in render order.
    pub prompt: Vec<ContentBlock>,
    /// The shape of response expected.
    pub response: ResponseDefinition,
    /// How many attempts, and when feedback appears.
    pub attempt_policy: AttemptPolicy,
    /// Time limits, if any.
    pub timing_policy: TimingPolicy,
    /// How content varies between students and runs.
    pub randomization: RandomizationDefinition,
    /// How a response is judged.
    pub grading: GradingDefinition,
    /// Title, tags, taxonomy, license, language.
    pub metadata: QuestionMetadata,
}

impl QuestionDefinition {
    /// Whether this question is still a draft.
    ///
    /// Derived from the absence of a [`ProblemId`] rather than stored, so it
    /// cannot disagree with the identifier it describes.
    pub fn is_draft(&self) -> bool {
        self.problem.is_none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::answer::NumericTolerance;
    use crate::generation::RandomizationDefinition;
    use crate::run_policy::FeedbackDisclosure;
    use uuid::Uuid;

    fn sample_question(problem: Option<ProblemId>) -> QuestionDefinition {
        QuestionDefinition {
            version: VersionId::from_uuid(Uuid::from_u128(2)),
            problem,
            workspace: WorkspaceId::from_uuid(Uuid::from_u128(3)),
            source: QuestionSource::Native {
                family: "molar_mass".to_string(),
            },
            prompt: vec![ContentBlock::Text {
                markdown: "What is the molar mass?".to_string(),
            }],
            response: ResponseDefinition::Numeric {
                tolerance: NumericTolerance::Relative { fraction: 0.01 },
                unit: Some("g/mol".to_string()),
            },
            attempt_policy: AttemptPolicy {
                max_attempts: None,
                feedback: FeedbackDisclosure::Immediate,
            },
            timing_policy: TimingPolicy::Untimed,
            randomization: RandomizationDefinition::Static,
            grading: GradingDefinition::AllOrNothing { points: 1.0 },
            metadata: QuestionMetadata {
                title: "Molar mass".to_string(),
                tags: vec![Tag::new("stoichiometry")],
                taxonomy: Vec::new(),
                license: License::CcBySa,
                language: "en-US".to_string(),
            },
        }
    }

    #[test]
    fn a_question_without_a_problem_id_is_a_draft() {
        assert!(sample_question(None).is_draft());
    }

    #[test]
    fn a_published_question_carries_a_problem_id() {
        let published = sample_question(Some(ProblemId::from_uuid(Uuid::from_u128(9))));
        assert!(!published.is_draft());
    }

    #[test]
    fn a_question_survives_a_json_round_trip() {
        let question = sample_question(None);
        let json = serde_json::to_string(&question).expect("serialization should succeed");
        let restored: QuestionDefinition =
            serde_json::from_str(&json).expect("deserialization should succeed");
        assert_eq!(restored, question);
    }
}
