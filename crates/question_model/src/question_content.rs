//! [`QuestionRevision`], the backend-neutral representation every engine
//! maps into (WP-C1).
//!
//! One shared shape is what lets a WeBWorK question, a QTI item, and a
//! first-party algorithmic question flow through the same attempt loop,
//! gradebook, and export path. Each Question Backend translates into this
//! type, and everything downstream reads only this type.
//!
//! Question Content describes a Question. It carries no Answer Key, Question
//! Feedback, Question Answer Explanation, or Question Grading Input:
//! [`QuestionGradingRule`] states *how* a response is judged, while the private
//! values it is judged against live in `crates/grading`, server-side.

use serde::{Deserialize, Serialize};

use crate::assignment_activity_rules::{QuestionAttemptLimit, QuestionAttemptTimeLimit};
use crate::classification::{QuestionClassification, QuestionLicense, Tag};
use crate::envelope::QuestionContentBlock;
use crate::generation::QuestionVariationRule;
use crate::identity::{WorkspaceId, WorkspaceImportId};
use crate::question_citation::QuestionCitation;
use crate::response::{QuestionResponseFormat, QuestionType};
use crate::{QuestionId, QuestionRevisionNumber};

/// Maximum Unicode scalar values permitted in a student-facing question title.
///
/// Browser decoders use `Array.from(title).length`, which has the same count
/// for valid UTF-8 JSON strings. Titles are never silently trimmed or
/// normalized at either boundary.
pub const MAX_QUESTION_TITLE_UNICODE_SCALARS: usize = 512;

/// Maximum Unicode scalar values permitted in an Instructor-facing Question Description.
pub const MAX_QUESTION_DESCRIPTION_UNICODE_SCALARS: usize = 4_000;

/// Maximum bytes in an opaque iMathAS deployment, item, or profile identifier.
///
/// These identifiers are configuration and source-location keys, not URLs,
/// credentials, or arbitrary path fragments.  Keeping their grammar in the
/// Question Model makes the authored, published, adapter, and storage
/// boundaries agree before a draft can reach an adapter.
pub const MAX_IMATHAS_IDENTIFIER_BYTES: usize = 128;

/// Why an iMathAS deployment, item, or profile identifier is not safe to
/// retain in a Question Backend binding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImathasQuestionBackendBindingError {
    InvalidDeploymentReference,
    InvalidItemReference,
    InvalidProfile,
}

impl std::fmt::Display for ImathasQuestionBackendBindingError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidDeploymentReference => {
                formatter.write_str("iMathAS deployment reference is invalid")
            }
            Self::InvalidItemReference => formatter.write_str("iMathAS item reference is invalid"),
            Self::InvalidProfile => formatter.write_str("iMathAS profile is invalid"),
        }
    }
}

impl std::error::Error for ImathasQuestionBackendBindingError {}

fn has_imathas_identifier_grammar(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_IMATHAS_IDENTIFIER_BYTES
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
}

/// Opaque configured iMathAS deployment selector.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct ImathasDeploymentReference(String);

impl ImathasDeploymentReference {
    pub fn new(value: impl Into<String>) -> Result<Self, ImathasQuestionBackendBindingError> {
        let value = value.into();
        if !has_imathas_identifier_grammar(&value) {
            return Err(ImathasQuestionBackendBindingError::InvalidDeploymentReference);
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for ImathasDeploymentReference {
    type Error = ImathasQuestionBackendBindingError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<ImathasDeploymentReference> for String {
    fn from(value: ImathasDeploymentReference) -> Self {
        value.0
    }
}

/// iMathAS-backend-local item selector.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct ImathasItemReference(String);

impl ImathasItemReference {
    pub fn new(value: impl Into<String>) -> Result<Self, ImathasQuestionBackendBindingError> {
        let value = value.into();
        if !has_imathas_identifier_grammar(&value) || value.contains("..") {
            return Err(ImathasQuestionBackendBindingError::InvalidItemReference);
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for ImathasItemReference {
    type Error = ImathasQuestionBackendBindingError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<ImathasItemReference> for String {
    fn from(value: ImathasItemReference) -> Self {
        value.0
    }
}

/// Pinned iMathAS profile selected at publication.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct ImathasProfile(String);

impl ImathasProfile {
    pub fn new(value: impl Into<String>) -> Result<Self, ImathasQuestionBackendBindingError> {
        let value = value.into();
        if !has_imathas_identifier_grammar(&value) {
            return Err(ImathasQuestionBackendBindingError::InvalidProfile);
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for ImathasProfile {
    type Error = ImathasQuestionBackendBindingError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<ImathasProfile> for String {
    fn from(value: ImathasProfile) -> Self {
        value.0
    }
}

/// Immutable iMathAS backend location and profile pinned by a Question Revision.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ImathasQuestionBackendBinding {
    deployment_reference: ImathasDeploymentReference,
    item_reference: ImathasItemReference,
    profile: ImathasProfile,
}

impl ImathasQuestionBackendBinding {
    pub fn new(
        deployment_reference: ImathasDeploymentReference,
        item_reference: ImathasItemReference,
        profile: ImathasProfile,
    ) -> Self {
        Self {
            deployment_reference,
            item_reference,
            profile,
        }
    }

    pub fn deployment_reference(&self) -> &ImathasDeploymentReference {
        &self.deployment_reference
    }

    pub fn item_reference(&self) -> &ImathasItemReference {
        &self.item_reference
    }

    pub fn profile(&self) -> &ImathasProfile {
        &self.profile
    }
}

/// iMathAS location permitted before source snapshot preparation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DraftImathasQuestionBackendBinding {
    deployment_reference: ImathasDeploymentReference,
    item_reference: ImathasItemReference,
}

impl DraftImathasQuestionBackendBinding {
    pub fn new(
        deployment_reference: ImathasDeploymentReference,
        item_reference: ImathasItemReference,
    ) -> Self {
        Self {
            deployment_reference,
            item_reference,
        }
    }

    pub fn deployment_reference(&self) -> &ImathasDeploymentReference {
        &self.deployment_reference
    }

    pub fn item_reference(&self) -> &ImathasItemReference {
        &self.item_reference
    }
}

/// The authored or imported representation of a Question.
///
/// Question Format describes source representation and interchange. It is
/// independent of the educational Question Type, the server-side Question
/// Backend, and any Question Generator used to make a Question Variation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum QuestionFormat {
    /// Schema version 2 of PLE's canonical static Question JSON.
    PleQuestionJson,
    /// A first-party generated Question authored as a PLE Question Implementation.
    PleAlgorithmic,
    /// A WeBWorK PG source.
    WebworkPg,
    /// An imported QTI item.
    Qti,
    /// An H5P Package Import representation, retained outside Question Backend
    /// and Question Source lifecycles.
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

/// Why an Instructor-facing Question Description cannot be stored safely.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuestionDescriptionError {
    /// The trimmed description contains no visible content.
    Blank,
    /// The description exceeds [`MAX_QUESTION_DESCRIPTION_UNICODE_SCALARS`].
    TooLong,
}

impl std::fmt::Display for QuestionDescriptionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Blank => formatter.write_str("question description must not be blank"),
            Self::TooLong => write!(
                formatter,
                "question description must contain at most {MAX_QUESTION_DESCRIPTION_UNICODE_SCALARS} Unicode scalar values"
            ),
        }
    }
}

impl std::error::Error for QuestionDescriptionError {}

/// Validates the Instructor-facing discovery summary without changing its content.
pub fn validate_question_description(description: &str) -> Result<(), QuestionDescriptionError> {
    if description.trim().is_empty() {
        return Err(QuestionDescriptionError::Blank);
    }
    if description.chars().count() > MAX_QUESTION_DESCRIPTION_UNICODE_SCALARS {
        return Err(QuestionDescriptionError::TooLong);
    }
    Ok(())
}

/// Backend-specific location information for a Question Source.
///
/// This identifies the Question Backend and the backend's own location fields.
/// It deliberately carries neither Question Source data nor a Source Object
/// Reference; the private Question Source owns those exact bytes.
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
    /// An iMathAS item resolved through its configured deployment and profile.
    ///
    /// The deployment reference is an opaque configured key. The private
    /// Question Source holds the immutable snapshot bytes separately.
    Imathas {
        /// The typed iMathAS binding is flattened to its browser contract.
        #[serde(flatten)]
        binding: ImathasQuestionBackendBinding,
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
    /// Private iMathAS sandbox locator, never an endpoint or credential.
    Imathas {
        /// The typed draft binding is flattened to its browser contract.
        #[serde(flatten)]
        binding: DraftImathasQuestionBackendBinding,
    },
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
    /// H5P Package Import uses this for browser-evaluated practice and cannot
    /// carry a graded assignment.
    Ungraded,
}

/// Descriptive information used for search, attribution, and reuse.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuestionMetadata {
    /// Title shown in the Question Library and in printed exams.
    pub title: String,
    /// Instructor-facing answer-free discovery summary; never student-delivered by default.
    pub question_description: String,
    /// Free-form labels for search.
    pub tags: Vec<Tag>,
    /// Exact mappings to external or institutional classification systems.
    pub classifications: Vec<QuestionClassification>,
    /// Exact versioned legal grant under which this Question Revision may be reused.
    ///
    /// A Draft Question Revision may leave this unset until it is ready for
    /// publication; publication requires a compatible value.
    pub question_license: Option<QuestionLicense>,
    /// Optional source-publication credit distinct from Authorship and ownership.
    pub question_citation: Option<QuestionCitation>,
    /// BCP 47 language tag for the prompt, for example `en-US`.
    pub language: String,
}

impl QuestionMetadata {
    /// Validates the one piece of metadata delivered in every student envelope.
    pub fn validate_title(&self) -> Result<(), QuestionTitleError> {
        validate_question_title(&self.title)
    }

    /// Validates the Instructor-facing discovery summary independently of the title.
    pub fn validate_question_description(&self) -> Result<(), QuestionDescriptionError> {
        validate_question_description(&self.question_description)
    }
}

/// Editable Question content before it is accepted into a private persisted revision.
///
/// This transient value carries its owning Authoring Workspace but has no
/// Draft Question identity. The server persistence boundary binds it to an
/// exact Draft Question Revision; publication then mints the distinct
/// [`QuestionId`] and [`QuestionRevisionNumber`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DraftQuestionContent {
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
    /// How content varies between students and Assignment Attempts.
    pub question_variation_rule: QuestionVariationRule,
    /// How a response is judged.
    pub grading: QuestionGradingRule,
    /// Title, Question Description, tags, Question Classifications, Question License, and language.
    pub metadata: QuestionMetadata,
}

/// Compact browser-safe identity for one private Draft Question.
///
/// The full backend locator, prompt, Question Response Format, grading policy, and
/// asset references remain on the detail record.  In particular, this list
/// projection deliberately has no published Question ID or Question Revision Number.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DraftQuestionSummary {
    /// Stable browser-safe Draft Question locator selected by the editor.
    pub draft_question: crate::DraftQuestionReference,
    /// Private Authoring Workspace relationship that authorizes the editor.
    pub workspace: WorkspaceId,
    /// Stable typed locator used in application navigation.
    pub authoring_workspace: crate::AuthoringWorkspaceReference,
    /// Human-facing draft title.
    pub title: String,
    /// Question Backend without its private source locator.
    pub question_backend: crate::question_library::QuestionBackend,
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
    /// How content varies between students and Assignment Attempts.
    pub question_variation_rule: QuestionVariationRule,
    /// How a response is judged.
    pub grading: QuestionGradingRule,
    /// Title, tags, Question Classifications, Question License, language.
    pub metadata: QuestionMetadata,
}

impl QuestionRevision {
    /// Attaches the IDs minted at successful publication to draft content.
    pub fn from_draft(
        draft: DraftQuestionContent,
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
            question_variation_rule: draft.question_variation_rule,
            grading: draft.grading,
            metadata: draft.metadata,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::answer::NumericResponseTolerance;
    use crate::generation::QuestionVariationRule;
    use uuid::Uuid;

    fn sample_draft() -> DraftQuestionContent {
        DraftQuestionContent {
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
            question_variation_rule: QuestionVariationRule::Static,
            grading: QuestionGradingRule::AllOrNothing { points: 1.0 },
            metadata: QuestionMetadata {
                title: "Molar mass".to_string(),
                question_description: "Instructor-facing molar-mass fixture summary.".to_string(),
                tags: vec![Tag::new("stoichiometry")],
                classifications: Vec::new(),
                question_license: Some(QuestionLicense::CcBySa4_0),
                question_citation: None,
                language: "en-US".to_string(),
            },
        }
    }

    #[test]
    fn draft_question_content_serializes_without_published_identifiers() {
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
        let restored: DraftQuestionContent =
            serde_json::from_str(&json).expect("deserialization should succeed");
        assert_eq!(restored, question);
    }

    #[test]
    fn imathas_sandbox_source_cannot_become_published_without_a_snapshot() {
        let binding = DraftImathasQuestionBackendBinding::new(
            ImathasDeploymentReference::new("myopenmath").expect("valid deployment"),
            ImathasItemReference::new("12345").expect("valid item"),
        );
        let draft = DraftQuestionBackendLocator::Imathas {
            binding: binding.clone(),
        };
        assert_eq!(
            serde_json::to_value(&draft).expect("draft serializes"),
            serde_json::json!({
                "backend": "imathas",
                "deploymentReference": "myopenmath",
                "itemReference": "12345",
            })
        );
        assert_eq!(
            QuestionBackendLocator::try_from(draft),
            Err(QuestionBackendLocatorPreparationError::SnapshotRequired)
        );
    }

    #[test]
    fn imathas_bindings_are_typed_but_keep_the_flat_browser_locator_shape() {
        let binding = ImathasQuestionBackendBinding::new(
            ImathasDeploymentReference::new("self-hosted-imathas").expect("valid deployment"),
            ImathasItemReference::new("item-17").expect("valid item"),
            ImathasProfile::new("imathas_remote_grading_v1").expect("valid profile"),
        );
        let locator = QuestionBackendLocator::Imathas { binding };

        let json = serde_json::to_value(&locator).expect("locator serializes");
        assert_eq!(
            json,
            serde_json::json!({
                "backend": "imathas",
                "deploymentReference": "self-hosted-imathas",
                "itemReference": "item-17",
                "profile": "imathas_remote_grading_v1",
            })
        );
        assert!(
            serde_json::from_value::<QuestionBackendLocator>(serde_json::json!({
                "backend": "imathas",
                "deploymentReference": "https://untrusted.example",
                "itemReference": "item-17",
                "profile": "imathas_remote_grading_v1",
            }))
            .is_err()
        );
    }

    #[test]
    fn imathas_item_reference_refuses_path_traversal_segments() {
        assert_eq!(
            ImathasItemReference::new("item..17"),
            Err(ImathasQuestionBackendBindingError::InvalidItemReference)
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

    #[test]
    fn instructor_question_description_is_required_and_bounded_separately_from_title() {
        assert_eq!(
            validate_question_description(" \t\n "),
            Err(QuestionDescriptionError::Blank)
        );
        assert!(validate_question_description("Instructor-facing discovery summary.").is_ok());
        assert!(
            validate_question_description(
                &"\u{1F9EC}".repeat(MAX_QUESTION_DESCRIPTION_UNICODE_SCALARS)
            )
            .is_ok()
        );
        assert_eq!(
            validate_question_description(
                &"\u{1F9EC}".repeat(MAX_QUESTION_DESCRIPTION_UNICODE_SCALARS + 1)
            ),
            Err(QuestionDescriptionError::TooLong)
        );
    }
}
