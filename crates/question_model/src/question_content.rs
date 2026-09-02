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
use crate::generation::QuestionVariationRule;
use crate::identity::{QuestionAssetId, WorkspaceId, WorkspaceImportId};
use crate::question_citation::QuestionCitation;
use crate::response::{QuestionResponseFormat, QuestionType};
use crate::{QuestionBackend, QuestionId, QuestionRevisionNumber};

/// A reference to a stored asset used inside Question Content.
///
/// The checksum travels with the reference so a client can verify that the
/// bytes it received are the bytes the Question was authored against, which is
/// what makes a cached render trustworthy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuestionAssetReference {
    /// Identifier of the stored object.
    pub asset: QuestionAssetId,
    /// Hex-encoded checksum computed when the asset was written.
    pub checksum: String,
}

/// One renderable piece of a Question prompt.
///
/// Each variant that carries visual content also carries text describing it.
/// That text is required rather than optional: a Question whose figure has no
/// description is unusable with a screen reader, and the renderer surfaces a
/// missing description as an authoring error rather than rendering a gap.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum QuestionContentBlock {
    /// Prose, in a restricted Markdown subset that the renderer sanitizes.
    Text {
        /// Markdown source.
        markdown: String,
    },
    /// A mathematical expression.
    Math {
        /// LaTeX source.
        latex: String,
        /// Spoken-form description for assistive technology.
        description: String,
    },
    /// An image or figure.
    Image {
        /// The stored asset.
        asset: QuestionAssetReference,
        /// Description of what the image conveys.
        description: String,
    },
    /// A code listing.
    Code {
        /// Language name for highlighting, for example `python`.
        language: String,
        /// The listing itself.
        source: String,
    },
    /// A data table.
    Table {
        /// Column headings, left to right.
        headers: Vec<String>,
        /// Rows, each holding one cell per heading.
        rows: Vec<Vec<String>>,
        /// Description of what the table shows.
        description: String,
    },
}

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

/// Why direct Question Backend fields do not describe one permitted backend record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuestionBackendFieldsError {
    /// The record omits the exact field required by its Question Backend.
    MissingRequiredField,
    /// The record carries a field owned by another Question Backend or stage.
    UnexpectedField,
    /// Publication changed the backend instead of preparing the same backend's exact facts.
    BackendMismatch,
}

impl std::fmt::Display for QuestionBackendFieldsError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingRequiredField => formatter.write_str("Question Backend field is required"),
            Self::UnexpectedField => {
                formatter.write_str("Question Backend record carries an inapplicable field")
            }
            Self::BackendMismatch => formatter
                .write_str("published Question Backend differs from the Draft Question Backend"),
        }
    }
}

impl std::error::Error for QuestionBackendFieldsError {}

#[derive(Debug, Clone, Copy)]
struct QuestionBackendFieldPresence {
    webwork_pg_path: bool,
    qti_package_item_identifier: bool,
    workspace_import_id: bool,
    imathas_question_backend_binding: bool,
    draft_imathas_question_backend_binding: bool,
}

/// Validates the closed Question Backend field matrix shared by editable and
/// published Question records.
///
/// This deliberately relates backend-owned location facts only. It carries no
/// Question Source bytes, Source Object Reference, or Source Object Checksum.
fn validate_question_backend_field_matrix(
    question_backend: QuestionBackend,
    fields: QuestionBackendFieldPresence,
    is_draft: bool,
) -> Result<(), QuestionBackendFieldsError> {
    let QuestionBackendFieldPresence {
        webwork_pg_path,
        qti_package_item_identifier,
        workspace_import_id,
        imathas_question_backend_binding,
        draft_imathas_question_backend_binding,
    } = fields;

    let no_location_fields = !webwork_pg_path
        && !qti_package_item_identifier
        && !workspace_import_id
        && !imathas_question_backend_binding
        && !draft_imathas_question_backend_binding;
    match question_backend {
        QuestionBackend::Ple if no_location_fields => Ok(()),
        QuestionBackend::Ple => Err(QuestionBackendFieldsError::UnexpectedField),
        QuestionBackend::Webwork
            if webwork_pg_path
                && !qti_package_item_identifier
                && !workspace_import_id
                && !imathas_question_backend_binding
                && !draft_imathas_question_backend_binding =>
        {
            Ok(())
        }
        QuestionBackend::Webwork if !webwork_pg_path => {
            Err(QuestionBackendFieldsError::MissingRequiredField)
        }
        QuestionBackend::Webwork => Err(QuestionBackendFieldsError::UnexpectedField),
        QuestionBackend::Qti
            if qti_package_item_identifier
                && (is_draft == workspace_import_id)
                && !webwork_pg_path
                && !imathas_question_backend_binding
                && !draft_imathas_question_backend_binding =>
        {
            Ok(())
        }
        QuestionBackend::Qti
            if !qti_package_item_identifier || (is_draft && !workspace_import_id) =>
        {
            Err(QuestionBackendFieldsError::MissingRequiredField)
        }
        QuestionBackend::Qti => Err(QuestionBackendFieldsError::UnexpectedField),
        QuestionBackend::Imathas
            if !webwork_pg_path
                && !qti_package_item_identifier
                && !workspace_import_id
                && ((is_draft
                    && draft_imathas_question_backend_binding
                    && !imathas_question_backend_binding)
                    || (!is_draft
                        && imathas_question_backend_binding
                        && !draft_imathas_question_backend_binding)) =>
        {
            Ok(())
        }
        QuestionBackend::Imathas
            if (is_draft && !draft_imathas_question_backend_binding)
                || (!is_draft && !imathas_question_backend_binding) =>
        {
            Err(QuestionBackendFieldsError::MissingRequiredField)
        }
        QuestionBackend::Imathas => Err(QuestionBackendFieldsError::UnexpectedField),
    }
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
    /// Validates the one piece of metadata delivered in every Student Question Presentation.
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
    /// The Question Backend that owns the exact optional fields below.
    pub question_backend: QuestionBackend,
    /// WeBWorK PG Path for a WeBWorK Question Backend only.
    pub webwork_pg_path: Option<String>,
    /// QTI package item identifier for a QTI Question Backend only.
    pub qti_package_item_identifier: Option<String>,
    /// Private workspace import identity for a draft QTI Question only.
    pub workspace_import_id: Option<WorkspaceImportId>,
    /// iMathAS Deployment and Item References before publication pins a profile.
    pub draft_imathas_question_backend_binding: Option<DraftImathasQuestionBackendBinding>,
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

impl DraftQuestionContent {
    /// Validates the exact fields permitted for this editable Question Backend.
    pub fn validate_question_backend_fields(&self) -> Result<(), QuestionBackendFieldsError> {
        validate_question_backend_field_matrix(
            self.question_backend,
            QuestionBackendFieldPresence {
                webwork_pg_path: self.webwork_pg_path.is_some(),
                qti_package_item_identifier: self.qti_package_item_identifier.is_some(),
                workspace_import_id: self.workspace_import_id.is_some(),
                imathas_question_backend_binding: false,
                draft_imathas_question_backend_binding: self
                    .draft_imathas_question_backend_binding
                    .is_some(),
            },
            true,
        )
    }
}

/// Compact browser-safe identity for one private Draft Question.
///
/// The exact backend fields, prompt, Question Response Format, grading policy, and
/// asset references remain on the detail record.  In particular, this list
/// Draft Question Summary deliberately has no published Question ID or Question Revision Number.
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
    /// Question Backend without its private backend-specific fields.
    pub question_backend: crate::question_library::QuestionBackend,
}

/// Immutable published question content.
///
/// Every Assignment, Question Attempt, Question Variation, render cache key,
/// and grading operation uses this type, so a draft can never enter a
/// published-only path by omission.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuestionRevision {
    /// Stable Question lineage established at publication.
    pub question_id: QuestionId,
    /// Exact immutable version within the Question lineage.
    pub revision_number: QuestionRevisionNumber,
    /// The workspace that authored it.
    pub workspace: WorkspaceId,
    /// The Question Backend that owns the exact optional fields below.
    pub question_backend: QuestionBackend,
    /// WeBWorK PG Path for a WeBWorK Question Backend only.
    pub webwork_pg_path: Option<String>,
    /// QTI package item identifier for a QTI Question Backend only.
    pub qti_package_item_identifier: Option<String>,
    /// iMathAS Deployment, Item, and pinned Profile for an iMathAS Question Backend only.
    pub imathas_question_backend_binding: Option<ImathasQuestionBackendBinding>,
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
    /// Validates the exact fields permitted for this immutable Question Backend.
    pub fn validate_question_backend_fields(&self) -> Result<(), QuestionBackendFieldsError> {
        validate_question_backend_field_matrix(
            self.question_backend,
            QuestionBackendFieldPresence {
                webwork_pg_path: self.webwork_pg_path.is_some(),
                qti_package_item_identifier: self.qti_package_item_identifier.is_some(),
                workspace_import_id: false,
                imathas_question_backend_binding: self.imathas_question_backend_binding.is_some(),
                draft_imathas_question_backend_binding: false,
            },
            false,
        )
    }

    /// Attaches the IDs minted at successful publication to draft content.
    pub fn from_draft(
        draft: DraftQuestionContent,
        question_id: QuestionId,
        revision_number: QuestionRevisionNumber,
        question_backend: QuestionBackend,
        webwork_pg_path: Option<String>,
        qti_package_item_identifier: Option<String>,
        imathas_question_backend_binding: Option<ImathasQuestionBackendBinding>,
    ) -> Result<Self, QuestionBackendFieldsError> {
        draft.validate_question_backend_fields()?;
        if draft.question_backend != question_backend {
            return Err(QuestionBackendFieldsError::BackendMismatch);
        }
        let question_revision = Self {
            question_id,
            revision_number,
            workspace: draft.workspace,
            question_backend,
            webwork_pg_path,
            qti_package_item_identifier,
            imathas_question_backend_binding,
            question_format: draft.question_format,
            prompt: draft.prompt,
            response: draft.response,
            question_type: draft.question_type,
            question_attempt_limit: draft.question_attempt_limit,
            question_attempt_time_limit: draft.question_attempt_time_limit,
            question_variation_rule: draft.question_variation_rule,
            grading: draft.grading,
            metadata: draft.metadata,
        };
        question_revision.validate_question_backend_fields()?;
        Ok(question_revision)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::answer::NumericResponseTolerance;
    use crate::generation::QuestionVariationRule;
    use uuid::Uuid;

    #[test]
    fn visual_blocks_carry_their_description() {
        let block = QuestionContentBlock::Math {
            latex: r"\frac{1}{2}".to_string(),
            description: "one half".to_string(),
        };
        let json = serde_json::to_string(&block).expect("serialization should succeed");
        assert!(json.contains("one half"));
    }

    #[test]
    fn blocks_serialize_with_a_discriminant() {
        let block = QuestionContentBlock::Text {
            markdown: "Balance the equation.".to_string(),
        };
        let json = serde_json::to_string(&block).expect("serialization should succeed");
        assert!(json.starts_with(r#"{"kind":"text""#));
    }

    fn sample_draft() -> DraftQuestionContent {
        DraftQuestionContent {
            workspace: WorkspaceId::from_uuid(Uuid::from_u128(3)),
            question_backend: QuestionBackend::Ple,
            webwork_pg_path: None,
            qti_package_item_identifier: None,
            workspace_import_id: None,
            draft_imathas_question_backend_binding: None,
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
            QuestionBackend::Ple,
            None,
            None,
            None,
        )
        .expect("PLE draft publishes with no backend-specific fields");
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
    fn draft_question_backend_fields_accept_the_four_permitted_rows() {
        let binding = DraftImathasQuestionBackendBinding::new(
            ImathasDeploymentReference::new("myopenmath").expect("valid deployment"),
            ImathasItemReference::new("12345").expect("valid item"),
        );
        let mut draft = sample_draft();
        assert!(draft.validate_question_backend_fields().is_ok());

        draft.question_backend = QuestionBackend::Webwork;
        draft.webwork_pg_path = Some("Library/Algebra/test.pg".to_string());
        assert!(draft.validate_question_backend_fields().is_ok());

        draft.question_backend = QuestionBackend::Qti;
        draft.webwork_pg_path = None;
        draft.qti_package_item_identifier = Some("choice-1".to_string());
        draft.workspace_import_id = Some(WorkspaceImportId::from_uuid(Uuid::from_u128(44)));
        assert!(draft.validate_question_backend_fields().is_ok());

        draft.question_backend = QuestionBackend::Imathas;
        draft.qti_package_item_identifier = None;
        draft.workspace_import_id = None;
        draft.draft_imathas_question_backend_binding = Some(binding);
        assert!(draft.validate_question_backend_fields().is_ok());
    }

    #[test]
    fn published_question_backend_fields_require_a_pinned_imathas_profile() {
        let draft_binding = DraftImathasQuestionBackendBinding::new(
            ImathasDeploymentReference::new("self-hosted-imathas").expect("valid deployment"),
            ImathasItemReference::new("item-17").expect("valid item"),
        );
        let binding = ImathasQuestionBackendBinding::new(
            ImathasDeploymentReference::new("self-hosted-imathas").expect("valid deployment"),
            ImathasItemReference::new("item-17").expect("valid item"),
            ImathasProfile::new("imathas_remote_grading_v1").expect("valid profile"),
        );
        let mut draft = sample_draft();
        draft.question_backend = QuestionBackend::Imathas;
        draft.draft_imathas_question_backend_binding = Some(draft_binding);
        let published = QuestionRevision::from_draft(
            draft,
            "123-4567".parse().expect("valid Question ID"),
            QuestionRevisionNumber::new(1).expect("positive version"),
            QuestionBackend::Imathas,
            None,
            None,
            Some(binding),
        );
        assert_eq!(
            published
                .expect("pinned iMathAS profile publishes")
                .imathas_question_backend_binding
                .expect("iMathAS binding")
                .profile()
                .as_str(),
            "imathas_remote_grading_v1"
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
    fn question_backend_fields_reject_cross_backend_and_draft_only_values() {
        let mut draft = sample_draft();
        draft.webwork_pg_path = Some("Library/Algebra/test.pg".to_string());
        assert_eq!(
            draft.validate_question_backend_fields(),
            Err(QuestionBackendFieldsError::UnexpectedField)
        );

        let mut draft = sample_draft();
        draft.question_backend = QuestionBackend::Qti;
        draft.qti_package_item_identifier = Some("choice-1".to_string());
        draft.workspace_import_id = Some(WorkspaceImportId::from_uuid(Uuid::from_u128(44)));
        let published = QuestionRevision::from_draft(
            draft,
            "123-4567".parse().expect("valid Question ID"),
            QuestionRevisionNumber::new(1).expect("positive version"),
            QuestionBackend::Qti,
            None,
            Some("choice-1".to_string()),
            None,
        );
        assert!(published.is_ok());
    }

    #[test]
    fn publication_cannot_change_a_draft_question_backend() {
        let published = QuestionRevision::from_draft(
            sample_draft(),
            "123-4567".parse().expect("valid Question ID"),
            QuestionRevisionNumber::new(1).expect("positive version"),
            QuestionBackend::Qti,
            None,
            Some("choice-1".to_string()),
            None,
        );
        assert_eq!(published, Err(QuestionBackendFieldsError::BackendMismatch));
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
