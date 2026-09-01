//! [`QuestionRevision`], the backend-neutral representation every engine
//! maps into (WP-C1).
//!
//! One shared shape is what lets a WeBWorK question, a QTI item, an H5P
//! activity, and a first-party algorithmic question flow through the same
//! attempt loop, gradebook, and export path. Each backend adapter translates
//! into this type, and everything downstream reads only this type.
//!
//! The definition describes a question. It carries no Answer Key, Question
//! Feedback, Question Answer Explanation, or Question Grading Input:
//! [`QuestionGradingRule`] states *how* a response is judged, while the private
//! values it is judged against live in `crates/grading`, server-side.

use serde::{Deserialize, Serialize};

use crate::assignment_activity_rules::{QuestionAttemptLimit, QuestionAttemptTimeLimit};
use crate::classification::{License, QuestionClassification, Tag};
use crate::envelope::QuestionContentBlock;
use crate::generation::QuestionVariationDefinition;
use crate::identity::{WorkspaceId, WorkspaceImportId};
use crate::response::{QuestionResponseFormat, QuestionType};
use crate::{QuestionId, QuestionRevisionNumber};

/// Maximum Unicode scalar values permitted in a student-facing question title.
///
/// Browser decoders use `Array.from(title).length`, which has the same count
/// for valid UTF-8 JSON strings. Titles are never silently trimmed or
/// normalized at either boundary.
pub const MAX_QUESTION_TITLE_UNICODE_SCALARS: usize = 512;

/// The authored or imported representation of a Question.
///
/// Question Format describes source representation and interchange. It is
/// independent of the educational Question Type, the server-side Question
/// Backend, and any Question Generator used to make a Question Variation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum QuestionFormat {
    /// Version 2 of PLE's canonical static flat-question JSON.
    PleFlatQuestionV2,
    /// A first-party generated Question authored as a PLE Question Implementation.
    PleAlgorithmic,
    /// A WeBWorK PG source.
    WebworkPg,
    /// An imported QTI item.
    Qti,
    /// An imported H5P activity.
    H5p,
    /// An archived iMathAS source snapshot.
    Imathas,
}

/// Why a student-facing question title cannot be safely delivered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuestionTitleError {
    /// The trimmed title contains no visible content.
    Blank,
    /// The title exceeds [`MAX_QUESTION_TITLE_UNICODE_SCALARS`].
    TooLong,
}

impl std::fmt::Display for QuestionTitleError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Blank => formatter.write_str("question title must not be blank"),
            Self::TooLong => write!(
                formatter,
                "question title must contain at most {MAX_QUESTION_TITLE_UNICODE_SCALARS} Unicode scalar values"
            ),
        }
    }
}

impl std::error::Error for QuestionTitleError {}

/// Validates the durable student-facing title without mutating it.
///
/// Whitespace-only titles are refused after trimming for the emptiness check;
/// leading and trailing whitespace are otherwise preserved exactly rather than
/// being silently changed before storage or delivery.
pub fn validate_question_title(title: &str) -> Result<(), QuestionTitleError> {
    if title.trim().is_empty() {
        return Err(QuestionTitleError::Blank);
    }
    if title.chars().count() > MAX_QUESTION_TITLE_UNICODE_SCALARS {
        return Err(QuestionTitleError::TooLong);
    }
    Ok(())
}

/// Backend-specific location information for a Question Source.
///
/// This identifies the Question Backend and the backend's own location fields.
/// It deliberately carries neither Question Source data nor a Source Object
/// Reference; the private Question Source record owns those exact bytes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "backend",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum QuestionBackendLocator {
    /// A first-party PLE Question.
    Ple,
    /// A WeBWorK PG problem, rendered by the renderer service.
    Webwork {
        /// Path within the problem library, for example an OPL path.
        pg_path: String,
    },
    /// An item from an imported QTI package.
    Qti {
        /// Item identifier within the package.
        item_id: String,
    },
    /// An imported H5P activity, which evaluates in the browser.
    H5p {
        /// H5P content type, for example `H5P.MultiChoice`.
        content_type: String,
    },
    /// An iMathAS item resolved through a configured integration profile.
    ///
    /// The provider name is an opaque deployment-configured key. The private
    /// Question Source record holds the immutable snapshot bytes separately.
    Imathas {
        /// Opaque configured provider key, never a URL or credential.
        provider: String,
        /// Provider-local item reference captured in the immutable snapshot.
        item_ref: String,
        /// Supported integration profile pinned at publication time.
        integration_profile: String,
    },
}

/// Backend-specific location information permitted while content is a private
/// Draft Question Revision.
///
/// Unlike [`QuestionBackendLocator::Imathas`], its iMathAS variant intentionally has
/// no snapshot or profile: those are fetched and frozen by the server before
/// publication can mint a durable version.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "backend",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum DraftQuestionBackendLocator {
    /// A first-party PLE Question.
    Ple,
    /// A WeBWorK PG problem.
    Webwork { pg_path: String },
    /// An imported QTI item staged in this draft's private workspace.
    ///
    /// The import record, rather than the browser, resolves private archive,
    /// asset and Question Grading Input. It cannot be used as a published locator.
    Qti {
        item_id: String,
        import_id: WorkspaceImportId,
    },
    /// An H5P activity.
    H5p { content_type: String },
    /// Private iMathAS sandbox locator, never an endpoint or credential.
    Imathas { provider: String, item_ref: String },
}

impl TryFrom<DraftQuestionBackendLocator> for QuestionBackendLocator {
    type Error = QuestionBackendLocatorPreparationError;

    fn try_from(backend_locator: DraftQuestionBackendLocator) -> Result<Self, Self::Error> {
        match backend_locator {
            DraftQuestionBackendLocator::Ple => Ok(Self::Ple),
            DraftQuestionBackendLocator::Webwork { pg_path } => Ok(Self::Webwork { pg_path }),
            DraftQuestionBackendLocator::Qti { .. } => {
                Err(QuestionBackendLocatorPreparationError::QtiImportRequired)
            }
            DraftQuestionBackendLocator::H5p { content_type } => Ok(Self::H5p { content_type }),
            DraftQuestionBackendLocator::Imathas { .. } => {
                Err(QuestionBackendLocatorPreparationError::SnapshotRequired)
            }
        }
    }
}

/// Why a draft backend locator cannot become a published immutable source automatically.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuestionBackendLocatorPreparationError {
    /// iMathAS requires the server to create the Question Source snapshot and pin its profile.
    SnapshotRequired,
    /// QTI publication must resolve an authorized staged import into an exact
    /// immutable Question Source before identifiers are minted.
    QtiImportRequired,
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
pub enum QuestionGradingRule {
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
    /// Title shown in the Question Library and in printed exams.
    pub title: String,
    /// Free-form labels for search.
    pub tags: Vec<Tag>,
    /// Exact mappings to external or institutional classification systems.
    pub classifications: Vec<QuestionClassification>,
    /// Terms under which the content may be reused.
    pub license: License,
    /// BCP 47 language tag for the prompt, for example `en-US`.
    pub language: String,
}

impl QuestionMetadata {
    /// Validates the one piece of metadata delivered in every student envelope.
    pub fn validate_title(&self) -> Result<(), QuestionTitleError> {
        validate_question_title(&self.title)
    }
}

/// Editable workspace content before publication.
///
/// A draft deliberately has neither a [`QuestionId`] nor a [`QuestionRevisionNumber`].
/// Publication is the one boundary that mints both durable identifiers.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DraftQuestionRevision {
    /// The workspace that authored it.
    pub workspace: WorkspaceId,
    /// The Question Backend and any backend-specific location facts.
    pub backend_locator: DraftQuestionBackendLocator,
    /// The authored or imported representation of this Question.
    pub question_format: QuestionFormat,
    /// The prompt, in render order.
    pub prompt: Vec<QuestionContentBlock>,
    /// The shape of response expected.
    pub response: QuestionResponseFormat,
    /// The educational interaction this Question assesses.
    pub question_type: QuestionType,
    /// How many Question Attempts a Student may make for one Issued Question.
    pub question_attempt_limit: QuestionAttemptLimit,
    /// Time limits, if any.
    pub question_attempt_time_limit: QuestionAttemptTimeLimit,
    /// How content varies between students and runs.
    pub question_variation_definition: QuestionVariationDefinition,
    /// How a response is judged.
    pub grading: QuestionGradingRule,
    /// Title, tags, Question Classifications, license, language.
    pub metadata: QuestionMetadata,
}

/// Compact browser-safe identity for one private workspace draft.
///
/// The full backend locator, prompt, Question Response Format, grading policy, and
/// asset references remain on the detail record.  In particular, this list
/// projection deliberately has no published Question ID or Question Revision Number.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceDraftSummary {
    /// Private workspace identity used to retrieve the editable draft.
    pub workspace: WorkspaceId,
    /// Stable typed locator used in application navigation.
    pub reference: crate::AuthoringWorkspaceReference,
    /// Human-facing draft title.
    pub title: String,
    /// Question Backend without its private source locator.
    pub question_backend: crate::question_library::QuestionBackend,
}

impl DraftQuestionRevision {
    /// Builds the intentionally compact list projection for this draft.
    pub fn workspace_summary(
        &self,
        reference: crate::AuthoringWorkspaceReference,
    ) -> WorkspaceDraftSummary {
        WorkspaceDraftSummary {
            workspace: self.workspace,
            reference,
            title: self.metadata.title.clone(),
            question_backend: crate::question_library::QuestionBackend::from(&self.backend_locator),
        }
    }
}

/// Immutable published question content.
///
/// Every assignment, attempt, envelope, cache key, and grading operation uses
/// this type, so a draft can never enter a published-only path by omission.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuestionRevision {
    /// Stable Question lineage established at publication.
    pub question_id: QuestionId,
    /// Exact immutable version within the Question lineage.
    pub revision_number: QuestionRevisionNumber,
    /// The workspace that authored it.
    pub workspace: WorkspaceId,
    /// Which engine it came from.
    pub backend_locator: QuestionBackendLocator,
    /// The authored or imported representation of this Question.
    pub question_format: QuestionFormat,
    /// The prompt, in render order.
    pub prompt: Vec<QuestionContentBlock>,
    /// The shape of response expected.
    pub response: QuestionResponseFormat,
    /// The educational interaction this Question assesses.
    pub question_type: QuestionType,
    /// How many Question Attempts a Student may make for one Issued Question.
    pub question_attempt_limit: QuestionAttemptLimit,
    /// Time limits, if any.
    pub question_attempt_time_limit: QuestionAttemptTimeLimit,
    /// How content varies between students and runs.
    pub question_variation_definition: QuestionVariationDefinition,
    /// How a response is judged.
    pub grading: QuestionGradingRule,
    /// Title, tags, Question Classifications, license, language.
    pub metadata: QuestionMetadata,
}

impl QuestionRevision {
    /// Attaches the IDs minted at successful publication to draft content.
    pub fn from_draft(
        draft: DraftQuestionRevision,
        question_id: QuestionId,
        revision_number: QuestionRevisionNumber,
        backend_locator: QuestionBackendLocator,
    ) -> Self {
        Self {
            question_id,
            revision_number,
            workspace: draft.workspace,
            backend_locator,
            question_format: draft.question_format,
            prompt: draft.prompt,
            response: draft.response,
            question_type: draft.question_type,
            question_attempt_limit: draft.question_attempt_limit,
            question_attempt_time_limit: draft.question_attempt_time_limit,
            question_variation_definition: draft.question_variation_definition,
            grading: draft.grading,
            metadata: draft.metadata,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::answer::NumericResponseTolerance;
    use crate::generation::QuestionVariationDefinition;
    use uuid::Uuid;

    fn sample_draft() -> DraftQuestionRevision {
        DraftQuestionRevision {
            workspace: WorkspaceId::from_uuid(Uuid::from_u128(3)),
            backend_locator: DraftQuestionBackendLocator::Ple,
            question_format: QuestionFormat::PleAlgorithmic,
            prompt: vec![QuestionContentBlock::Text {
                markdown: "What is the molar mass?".to_string(),
            }],
            response: QuestionResponseFormat::Numeric {
                tolerance: NumericResponseTolerance::Relative { fraction: 0.01 },
                unit: Some("g/mol".to_string()),
            },
            question_type: QuestionType::Numeric,
            question_attempt_limit: QuestionAttemptLimit { max_attempts: None },
            question_attempt_time_limit: QuestionAttemptTimeLimit::Unlimited,
            question_variation_definition: QuestionVariationDefinition::Static,
            grading: QuestionGradingRule::AllOrNothing { points: 1.0 },
            metadata: QuestionMetadata {
                title: "Molar mass".to_string(),
                tags: vec![Tag::new("stoichiometry")],
                classifications: Vec::new(),
                license: License::CcBySa,
                language: "en-US".to_string(),
            },
        }
    }

    #[test]
    fn a_draft_serializes_without_published_identifiers() {
        let json = serde_json::to_value(sample_draft()).expect("draft serializes");
        assert!(json.get("questionId").is_none());
        assert!(json.get("revisionNumber").is_none());
    }

    #[test]
    fn a_published_question_carries_its_question_revision_reference() {
        let published = QuestionRevision::from_draft(
            sample_draft(),
            "123-4567".parse().expect("valid Question ID"),
            QuestionRevisionNumber::new(1).expect("positive version"),
            QuestionBackendLocator::Ple,
        );
        assert_eq!(published.question_id.to_string(), "123-4567");
    }

    #[test]
    fn a_question_survives_a_json_round_trip() {
        let question = sample_draft();
        let json = serde_json::to_string(&question).expect("serialization should succeed");
        let restored: DraftQuestionRevision =
            serde_json::from_str(&json).expect("deserialization should succeed");
        assert_eq!(restored, question);
    }

    #[test]
    fn imathas_sandbox_source_cannot_become_published_without_a_snapshot() {
        assert_eq!(
            QuestionBackendLocator::try_from(DraftQuestionBackendLocator::Imathas {
                provider: "myopenmath".to_string(),
                item_ref: "12345".to_string(),
            }),
            Err(QuestionBackendLocatorPreparationError::SnapshotRequired)
        );
    }

    #[test]
    fn qti_draft_uses_only_an_opaque_workspace_import_and_requires_preparation() {
        let import_id = WorkspaceImportId::from_uuid(Uuid::from_u128(44));
        let draft = DraftQuestionBackendLocator::Qti {
            item_id: "choice-1".to_string(),
            import_id,
        };
        let json = serde_json::to_value(&draft).expect("draft serializes");
        assert_eq!(json["backend"], "qti");
        assert_eq!(json["importId"], import_id.to_string());
        assert!(json.get("packageObject").is_none());
        assert_eq!(
            QuestionBackendLocator::try_from(draft),
            Err(QuestionBackendLocatorPreparationError::QtiImportRequired)
        );
    }

    #[test]
    fn qti_backend_locator_carries_only_its_item_location() {
        let source = QuestionBackendLocator::Qti {
            item_id: "choice-1".to_string(),
        };
        let json = serde_json::to_string(&source).expect("locator serializes");
        let restored: QuestionBackendLocator =
            serde_json::from_str(&json).expect("locator round trips");
        assert_eq!(restored, source);
        assert!(serde_json::from_str::<QuestionBackendLocator>(
            r#"{\"backend\":\"qti\",\"itemId\":\"choice-1\",\"sourceObjectReference\":{\"object\":\"00000000-0000-0000-0000-00000000002d\",\"sha256\":\"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\"}}"#
        )
        .is_err());
    }

    #[test]
    fn learner_title_policy_uses_trimmed_content_and_unicode_scalar_count() {
        assert_eq!(
            validate_question_title(" \t\n "),
            Err(QuestionTitleError::Blank)
        );
        assert!(validate_question_title("A title").is_ok());
        assert!(
            validate_question_title(&"\u{1F9EC}".repeat(MAX_QUESTION_TITLE_UNICODE_SCALARS))
                .is_ok()
        );
        assert_eq!(
            validate_question_title(&"\u{1F9EC}".repeat(MAX_QUESTION_TITLE_UNICODE_SCALARS + 1)),
            Err(QuestionTitleError::TooLong)
        );
    }
}
