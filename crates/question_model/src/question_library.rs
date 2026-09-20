//! Browser-safe shared Question Library metadata.

use std::num::NonZeroU64;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::question_license::QuestionLicense;
use crate::question_tag::Tag;
use crate::{
    CourseInstanceId, QuestionBackendCapabilities, QuestionMetadata, QuestionRevisionNumber,
    Timestamp,
};

pub use crate::question_search::{
    MAX_QUESTION_SEARCH_AUTHOR_NAME_FACETS, MAX_QUESTION_SEARCH_AUTHOR_NAME_FILTERS,
    MAX_QUESTION_SEARCH_BACKEND_FACETS, MAX_QUESTION_SEARCH_CURSOR_ENCODED_BYTES,
    MAX_QUESTION_SEARCH_QUESTION_TYPE_FACETS, MAX_QUESTION_SEARCH_QUESTION_TYPE_FILTERS,
    MAX_QUESTION_SEARCH_TAG_FACETS, MAX_QUESTION_SEARCH_TAG_FILTERS, QuestionSearchAuthorFacet,
    QuestionSearchAuthorship, QuestionSearchBackendFacet, QuestionSearchBloomCognitiveProcessFacet,
    QuestionSearchBloomKnowledgeDimensionFacet, QuestionSearchCapabilityFacet,
    QuestionSearchCourseUse, QuestionSearchCourseUseFacet, QuestionSearchFacets,
    QuestionSearchFilter, QuestionSearchQuestionLicenseFacet, QuestionSearchRequest,
    QuestionSearchRequestError, QuestionSearchSort, QuestionSearchSubjectFacet,
    QuestionSearchTagFacet, QuestionSearchTopicFacet, QuestionTypeFacet,
    normalized_question_search_group_value,
};
pub use crate::response::QuestionType;

/// Maximum own-course rows included with one exact Question Library usage detail.
///
/// The aggregate summary remains complete while the named course list stays a
/// compact, visible decision aid rather than an unbounded course inventory.
pub const MAX_QUESTION_SEARCH_OWN_COURSE_USAGES: usize = 100;

/// Crockford Base32 alphabet used by public identifiers.
pub const QUESTION_ID_ALPHABET: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";

/// Number of server-random Crockford characters in one Question or Pool ID.
pub const QUESTION_ID_IDENTIFIER_LENGTH: usize = 7;

/// Byte length of the exact canonical `XXXX-ZXXX` Question or Pool ID.
pub const QUESTION_ID_CANONICAL_LENGTH: usize = 9;

/// Zero-based byte position of the embedded checksum character.
pub const QUESTION_ID_CHECK_CHARACTER_INDEX: usize = 5;

/// Zero-based byte position of the required Question or Pool hyphen.
pub const QUESTION_ID_HYPHEN_INDEX: usize = 4;

/// Product limit kept independent of the larger encoded namespace.
pub const MAX_QUESTION_ID_COUNT: u64 = 100_000_000;

/// Maximum number of Published Questions one shared-metadata command changes.
pub const MAX_BULK_QUESTION_METADATA_ITEMS: usize = 1000;

/// One stable, non-sequential human-facing identity for a Published Question
/// lineage. [`QuestionRevisionTuple`] pairs it with a positive revision
/// number to identify one immutable Question Revision.
///
/// `XXXX-ZXXX` is the one canonical value at every boundary. The hyphen and
/// public checksum character are both part of the ID, not presentation.
/// Parsing rejects every alternate spelling, including lowercase,
/// unhyphenated, alias, and whitespace-padded input.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct QuestionId(String);

impl QuestionId {
    /// The exact canonical public value for every storage and transport boundary.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Mints the exact public value from seven server-random Crockford characters.
    ///
    /// ASVS V2.1.1 and V2.2.1: this trusted server-generation boundary checks its
    /// constrained input before creating an identifier. Callers must obtain the
    /// seven random characters from a cryptographically secure source.
    pub fn from_random_identifier(identifier: impl AsRef<str>) -> Result<Self, &'static str> {
        let identifier = identifier.as_ref();
        if identifier.len() != QUESTION_ID_IDENTIFIER_LENGTH
            || !identifier
                .bytes()
                .all(|character| QUESTION_ID_ALPHABET.contains(&character))
        {
            return Err("Question ID random characters must be exact uppercase Crockford Base32");
        }
        let checksum = public_id_checksum_character(identifier.as_bytes());
        Ok(Self(format!(
            "{}-{checksum}{}",
            &identifier[..QUESTION_ID_HYPHEN_INDEX],
            &identifier[QUESTION_ID_HYPHEN_INDEX..]
        )))
    }
}

impl std::fmt::Display for QuestionId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::str::FromStr for QuestionId {
    type Err = &'static str;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        // ASVS V2.2.1 and V2.2.2: an untrusted boundary accepts only this
        // one exact value; it never trims, aliases, or reformats an ID.
        if value.len() != QUESTION_ID_CANONICAL_LENGTH
            || !value.is_ascii()
            || value.as_bytes()[QUESTION_ID_HYPHEN_INDEX] != b'-'
            || !value[..QUESTION_ID_HYPHEN_INDEX]
                .bytes()
                .chain(value[QUESTION_ID_CHECK_CHARACTER_INDEX..].bytes())
                .all(|character| QUESTION_ID_ALPHABET.contains(&character))
        {
            return Err("Question ID must use exact canonical XXXX-ZXXX syntax");
        }
        let checksum_input = format!(
            "{}{}",
            &value[..QUESTION_ID_HYPHEN_INDEX],
            &value[QUESTION_ID_CHECK_CHARACTER_INDEX + 1..]
        );
        if value.as_bytes()[QUESTION_ID_CHECK_CHARACTER_INDEX]
            != public_id_checksum_character(checksum_input.as_bytes()) as u8
        {
            return Err("Question ID checksum does not match its canonical characters");
        }
        Ok(Self(value.to_owned()))
    }
}

/// Calculates the public checksum character for canonical-ID characters after
/// separators and checksum positions have been excluded by the caller.
///
/// This is intentionally public and unsalted so the same exact ID can be
/// validated outside PLE; it is an entry-integrity check, never authority.
pub fn public_id_checksum_character(canonical_characters: &[u8]) -> char {
    let digest = Sha256::digest(canonical_characters);
    QUESTION_ID_ALPHABET[(digest[0] >> 3) as usize] as char
}

impl TryFrom<String> for QuestionId {
    type Error = &'static str;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        value.parse()
    }
}

impl From<QuestionId> for String {
    fn from(value: QuestionId) -> Self {
        value.to_string()
    }
}

/// Exact immutable Question Revision identity used by storage, delivery,
/// grading, replay, and audit.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QuestionRevisionTuple {
    /// Stable Question lineage.
    pub question_id: QuestionId,
    /// Exact immutable version within that Question lineage.
    pub revision_number: QuestionRevisionNumber,
}

/// Browser-safe current shared metadata for one Published Question.
///
/// This is current search metadata, not a Question Revision or metadata
/// history record. It includes only the fields the bulk metadata workflow can
/// read and replace.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PublishedQuestionSharedMetadata {
    /// Stable Published Question identity.
    pub question_id: QuestionId,
    /// Current compare-and-swap number for the shared metadata.
    pub metadata_edit_number: u64,
    /// Current complete tag set.
    pub tags: Vec<Tag>,
    /// Current canonical Discipline identity.
    pub discipline_uuid: uuid::Uuid,
    /// Current canonical Subject identity.
    pub subject_uuid: uuid::Uuid,
    /// Current optional Topic identity.
    pub topic_uuid: Option<uuid::Uuid>,
    /// Current optional Subtopic identity.
    pub subtopic_uuid: Option<uuid::Uuid>,
}

/// Current selection availability for a stable Published Question lineage.
/// Exact Question Revisions remain resolvable after an archive transition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "availability",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum QuestionAvailability {
    /// Discoverable and eligible for ordinary new selection.
    Available,
    /// Hidden from ordinary browsing and new selection. Existing exact pins
    /// still resolve under their normal authorization path.
    Archived,
}

impl QuestionAvailability {
    /// Whether Question Library browsing should include the immutable publication.
    pub fn is_discoverable(&self) -> bool {
        matches!(self, Self::Available)
    }

    /// Whether this publication can create a new reference through ordinary selection.
    ///
    /// This does not govern resolution of an existing exact immutable pin.
    pub fn is_eligible_for_ordinary_new_selection(&self) -> bool {
        matches!(self, Self::Available)
    }

    /// Whether an existing exact Question Revision Tuple remains resolvable
    /// for an authorized read.
    ///
    /// Availability controls discovery and new selection; it does not erase
    /// immutable revision provenance.
    pub fn permits_existing_exact_revision_resolution(&self) -> bool {
        true
    }
}

/// Positive compare-and-swap number for one Published Question lineage's
/// availability transition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct QuestionAvailabilityEditNumber(NonZeroU64);

impl QuestionAvailabilityEditNumber {
    pub const INITIAL: Self = Self(NonZeroU64::MIN);

    pub fn new(value: u64) -> Option<Self> {
        (value > 0 && value <= i64::MAX as u64).then_some(Self(NonZeroU64::new(value)?))
    }

    pub const fn value(self) -> u64 {
        self.0.get()
    }

    pub fn checked_next(self) -> Option<Self> {
        Self::new(self.value().checked_add(1)?)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QuestionAvailabilityEditNumberError;

impl std::fmt::Display for QuestionAvailabilityEditNumberError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .write_str("question availability edit number must be a canonical positive decimal")
    }
}

impl std::error::Error for QuestionAvailabilityEditNumberError {}

impl FromStr for QuestionAvailabilityEditNumber {
    type Err = QuestionAvailabilityEditNumberError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        if value.is_empty()
            || value.starts_with('0')
            || !value.bytes().all(|byte| byte.is_ascii_digit())
        {
            return Err(QuestionAvailabilityEditNumberError);
        }
        value
            .parse::<u64>()
            .ok()
            .and_then(Self::new)
            .ok_or(QuestionAvailabilityEditNumberError)
    }
}

impl TryFrom<String> for QuestionAvailabilityEditNumber {
    type Error = QuestionAvailabilityEditNumberError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        value.parse()
    }
}

impl From<QuestionAvailabilityEditNumber> for String {
    fn from(value: QuestionAvailabilityEditNumber) -> Self {
        value.to_string()
    }
}

impl std::fmt::Display for QuestionAvailabilityEditNumber {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.value().fmt(formatter)
    }
}

/// Immutable availability-transition evidence for a stable Published Question.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuestionAvailabilityEvent {
    pub question_id: QuestionId,
    pub actor: crate::AccountId,
    pub availability: QuestionAvailability,
    pub edit_number: QuestionAvailabilityEditNumber,
    pub recorded_at: Timestamp,
}

/// Question Backend without source paths or package identifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum QuestionBackend {
    /// First-party Rust/WASM question.
    Ple,
    /// WeBWorK PG question.
    Webwork,
    /// iMathAS item served through the verified iMathAS Question Backend Transport.
    Imathas,
}

/// Which layer owns a Question Backend's interaction semantics.
///
/// This is deliberately a model-only policy seam, not a browser wire shape or
/// a generic response model. It gives the common backend interface one
/// explicit boundary: PLE implements its own native interaction, while an
/// external backend keeps rendering, response interpretation, grading,
/// feedback, and backend state inside its adapter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuestionBackendInteractionPolicy {
    /// PLE is the Question Backend and owns the native interaction.
    PleOwned,
    /// The selected adapter owns its opaque interaction semantics.
    BackendOwned,
}

impl QuestionBackend {
    /// Every browser-safe Question Backend supported by this release.
    ///
    /// This is the common adapter vocabulary. It identifies the adapter that
    /// owns backend-specific behavior; it does not make PLE interpret that
    /// behavior.
    pub const ALL: [Self; 3] = [Self::Ple, Self::Webwork, Self::Imathas];

    /// Whether this backend may create new production work in this release.
    ///
    /// `ALL` remains the closed wire vocabulary so retained historical records
    /// can still be decoded and inspected. iMathAS is deliberately deferred.
    pub const fn is_supported_for_production(self) -> bool {
        matches!(self, Self::Ple | Self::Webwork)
    }

    /// Canonical public wire value for this closed backend vocabulary.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Ple => "ple",
            Self::Webwork => "webwork",
            Self::Imathas => "imathas",
        }
    }

    /// Returns the ownership boundary for the selected backend's interaction.
    ///
    /// A caller may use this to select the common lifecycle path, but must not
    /// infer or parse an external backend's controls, response shape, or
    /// state.
    pub const fn interaction_policy(self) -> QuestionBackendInteractionPolicy {
        match self {
            Self::Ple => QuestionBackendInteractionPolicy::PleOwned,
            Self::Webwork | Self::Imathas => QuestionBackendInteractionPolicy::BackendOwned,
        }
    }
}

/// The stable browser-safe surface shared by every Question Backend adapter.
///
/// This is deliberately a data contract, not an adapter trait. Every adapter
/// identifies itself through the closed [`QuestionBackend`] vocabulary and
/// declares only the capabilities it supports. Rendering, interaction,
/// response interpretation, grading, feedback, runtime state, and scoring
/// remain adapter-owned and therefore have no field in this shared surface.
///
/// `QuestionBackendInterface` may travel to the browser as part of Question
/// Library metadata. It contains no source path, package identifier, control
/// vocabulary, response shape, answer key, lifecycle state, or runtime data.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QuestionBackendInterface {
    /// The adapter identity, without backend-private configuration.
    pub backend: QuestionBackend,
    /// Capabilities declared by the adapter that owns its internal details.
    pub capabilities: QuestionBackendCapabilities,
}

impl QuestionBackendInterface {
    /// Creates the complete common surface for one adapter declaration.
    pub fn new(backend: QuestionBackend, capabilities: QuestionBackendCapabilities) -> Self {
        Self {
            backend,
            capabilities,
        }
    }
}

/// Question Library summary metadata returned by browse endpoints without loading payloads.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QuestionSummary {
    /// Sole human-facing identity of this stable Published Question lineage.
    pub question_id: QuestionId,
    /// Current accepted Revision for ordinary routes, or the exact resolved
    /// Revision for an exact-detail route. Independent of selection availability.
    pub question_revision_tuple: QuestionRevisionTuple,
    /// Question Backend, without private backend fields or Question Source data.
    pub backend: QuestionBackend,
    /// Immutable reviewed source representation, for Instructor identification
    /// only. It never exposes a source path or backend-private configuration.
    pub question_format: crate::QuestionFormat,
    /// Immutable author-declared educational Question Type copied to this Published Question
    /// Revision at publication time. It is never inferred from backend controls.
    pub question_type: QuestionType,
    /// Capabilities declared by the owning adapter at publication time.
    pub capabilities: QuestionBackendCapabilities,
    /// Shared metadata used for Question Title, Question Description,
    /// Question License, and language facets.
    pub metadata: QuestionMetadata,
    /// Immutable reviewed Question Authorship display snapshot; never Question Owner authority.
    pub authorship: crate::QuestionAuthorship,
    /// Current availability for ordinary new selection; publication itself is
    /// separate immutable history.
    pub availability: QuestionAvailability,
    /// Database-authoritative publication time.
    pub published_at: Timestamp,
    /// Exact Bloom Classification and its independent correction Edit Number, when assigned.
    pub bloom: Option<crate::BloomClassificationView>,
}

/// Current Question lineage plus the session-derived Archive affordance.
///
/// This projection carries no owner identity. The mutation independently
/// reauthorizes the current session, Question, ownership, title, and Edit Number.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QuestionLineageView {
    /// Browser-safe current Published Question summary.
    pub summary: QuestionSummary,
    /// Whether the current viewer is the current owner allowed to request Archive.
    pub viewer_may_archive: bool,
}

impl QuestionSummary {
    /// Returns the exact browser-safe common backend surface for this Question.
    ///
    /// This projection intentionally leaves all backend-specific runtime and
    /// interaction details with the adapter.
    pub fn backend_interface(&self) -> QuestionBackendInterface {
        QuestionBackendInterface::new(self.backend, self.capabilities.clone())
    }

    /// Free-form tags for filtering without loading the question payload.
    pub fn tags(&self) -> &[Tag] {
        &self.metadata.tags
    }

    /// Question License facet for reuse decisions.
    pub fn question_license(&self) -> Option<&QuestionLicense> {
        self.metadata.question_license.as_ref()
    }
}

pub use crate::question_library_statistics::{
    QuestionRevisionUsageStatistics, QuestionStatistics, QuestionUsageTotals,
};

/// One context-free search item. Search results contain immutable Question Library
/// metadata and anonymous discovery evidence only.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QuestionSearchResult {
    /// Exact immutable Question Library summary metadata.
    pub summary: QuestionSummary,
    /// Current readable Discipline name for this Question's existing classification reference.
    pub discipline_name: String,
    /// Whether `discipline_name` is retired. Retired classifications remain readable on existing
    /// references and are not new-selection choices.
    pub discipline_is_retired: bool,
    /// Decomposed anonymous evidence suitable for search discovery.
    pub evidence: QuestionStatistics,
}

/// Global and Account-owned usage counts for one exact publication.
///
/// Global totals do not identify a course. Own counts describe only the
/// requesting instructor's visible course references and are expanded by
/// [`QuestionUseDetails`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QuestionUseSummary {
    /// Installation-wide number of distinct courses with an assessment use.
    pub global_course_count: u64,
    /// Installation-wide number of assessment uses.
    pub global_assessment_count: u64,
    /// Current Account's distinct courses that use this publication.
    pub own_course_count: u64,
    /// Current Account's assessment uses across their visible courses.
    pub own_assessment_count: u64,
}

/// One current Account-visible course using an exact publication.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CourseQuestionUse {
    /// Authorized Course Instance ID; it is never authority by itself.
    pub course_instance_id: CourseInstanceId,
    /// Current course title visible to the requesting instructor.
    pub title: String,
    /// Number of current assessment uses in this course.
    pub assessment_count: u64,
}

/// Bounded Question Use Details View for a requesting instructor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QuestionUseDetails {
    /// Scope-explicit aggregate counts for this exact publication.
    pub summary: QuestionUseSummary,
    /// At most [`MAX_QUESTION_SEARCH_OWN_COURSE_USAGES`] Account-visible course rows.
    pub own_courses: Vec<CourseQuestionUse>,
    /// Whether additional Account-visible course rows remain beyond this bounded list.
    pub own_courses_truncated: bool,
}

/// Bounded search page with aggregates from the same Question Search snapshot.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QuestionSearchPage {
    /// At most the request's validated page size of context-free discovery rows.
    pub items: Vec<QuestionSearchResult>,
    /// Opaque continuation token, bound to the normalized query.
    pub next_cursor: Option<String>,
    /// Server-side facet counts; clients must not infer them from `items`.
    pub facets: QuestionSearchFacets,
}

/// Answer-free prompt view embedded in Question Details.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum QuestionDetailsPromptView {
    /// The immutable publication contains one fixed prompt.
    Static {
        /// Browser-safe prompt blocks in authored order.
        blocks: Vec<crate::question_content::QuestionContentBlock>,
    },
    /// One deterministic, server-generated example of a variable prompt.
    GeneratedExample {
        /// Browser-safe prompt blocks with every authored parameter resolved.
        blocks: Vec<crate::question_content::QuestionContentBlock>,
    },
}

/// Safe immutable Question Details View.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QuestionDetails {
    /// Exact immutable hot metadata for this publication.
    pub summary: QuestionSummary,
    /// Current readable Discipline name for this Question's existing classification reference.
    pub discipline_name: String,
    /// Current readable Subject name for this Question's existing classification reference.
    pub subject_name: String,
    /// Whether `discipline_name` is retired. Retired classifications remain readable on existing
    /// references and are not new-selection choices.
    pub discipline_is_retired: bool,
    /// Static content or one server-generated example; source, response,
    /// Question Variation Rule, grading, keys, and Question Pool Preview Nonce are excluded.
    pub prompt: QuestionDetailsPromptView,
    /// Answer-free native response controls; backend-owned previews supply their own controls.
    pub response_preview: Option<crate::QuestionResponsePreview>,
    /// Explainable anonymous evidence for this exact publication.
    pub evidence: QuestionStatistics,
    /// Bounded current-Account usage evidence for this exact publication.
    pub usage: QuestionUseDetails,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Capability;

    #[test]
    fn backend_summary_never_carries_private_backend_fields() {
        assert_eq!(
            serde_json::to_string(&QuestionBackend::Webwork).expect("backend serializes"),
            "\"webwork\""
        );
        assert_eq!(
            QuestionBackend::ALL.map(QuestionBackend::as_str),
            ["ple", "webwork", "imathas"]
        );
        assert!(QuestionBackend::Ple.is_supported_for_production());
        assert!(QuestionBackend::Webwork.is_supported_for_production());
        assert!(!QuestionBackend::Imathas.is_supported_for_production());
    }

    #[test]
    fn archive_blocks_new_selection_but_preserves_exact_revision_resolution() {
        assert!(QuestionAvailability::Available.is_discoverable());
        assert!(QuestionAvailability::Available.is_eligible_for_ordinary_new_selection());
        assert!(QuestionAvailability::Available.permits_existing_exact_revision_resolution());
        let archived = QuestionAvailability::Archived;
        assert!(!archived.is_discoverable());
        assert!(!archived.is_eligible_for_ordinary_new_selection());
        assert!(archived.permits_existing_exact_revision_resolution());
    }

    #[test]
    fn question_search_normalizes_equivalent_filters_and_bounds_hostile_input() {
        let query = QuestionSearchRequest {
            text: Some("  Peptide\tBond  ".to_string()),
            author_names: vec![
                "  Dr. Ada  Lovelace ".to_string(),
                "dr. ada lovelace".to_string(),
            ],
            backends: vec![QuestionBackend::Ple, QuestionBackend::Ple],
            tags: vec![
                " Protein   Structure ".to_string(),
                "protein structure".to_string(),
            ],
            question_types: vec![QuestionType::MultipleChoice; 2],
            capabilities: vec![Capability::Hints, Capability::Hints],
            question_licenses: vec![QuestionLicense::CcBy4_0, QuestionLicense::CcBy4_0],
            ..QuestionSearchRequest::default()
        }
        .normalized()
        .expect("equivalent filters normalize");
        assert_eq!(query.text.as_deref(), Some("peptide bond"));
        assert_eq!(query.author_names, vec!["dr. ada lovelace"]);
        assert_eq!(query.backends, vec![QuestionBackend::Ple]);
        assert_eq!(query.tags, vec!["protein structure"]);
        assert_eq!(query.question_types, vec![QuestionType::MultipleChoice]);
        assert_eq!(query.capabilities, vec![Capability::Hints]);
        assert_eq!(query.question_licenses, vec![QuestionLicense::CcBy4_0]);
        assert!(
            QuestionSearchRequest {
                text: Some("x".repeat(257)),
                ..QuestionSearchRequest::default()
            }
            .normalized()
            .is_err()
        );
    }

    #[test]
    fn question_library_detail_wire_shape_has_no_source_or_grading_fields() {
        let detail = QuestionDetails {
            summary: QuestionSummary {
                question_id: "ABCD-XEFG".parse().expect("fixture Question ID parses"),
                question_revision_tuple: QuestionRevisionTuple {
                    question_id: "ABCD-XEFG".parse().expect("fixture Question ID parses"),
                    revision_number: QuestionRevisionNumber::new(1).expect("positive version"),
                },
                backend: QuestionBackend::Ple,
                question_format: crate::QuestionFormat::PleQuestionJson,
                question_type: QuestionType::MultipleChoice,
                capabilities: QuestionBackendCapabilities::none(),
                metadata: QuestionMetadata {
                    question_title: "Safe detail".to_string(),
                    question_description: "Instructor-facing safe detail fixture summary."
                        .to_string(),
                    tags: Vec::new(),
                    question_license: Some(QuestionLicense::Cc0_1_0),
                    question_citation: None,
                    language: "en".to_string(),
                },
                authorship: crate::QuestionAuthorship::new(vec![crate::QuestionAuthor {
                    display_name: crate::QuestionAuthorDisplayName::new(
                        "Fixture Author".to_string(),
                    )
                    .expect("valid Question Author"),
                }])
                .expect("valid Question Authorship"),
                availability: QuestionAvailability::Available,
                published_at: Timestamp::from_unix_millis(0),
                bloom: Some(crate::BloomClassificationView {
                    cognitive_process: crate::BloomCognitiveProcess::Understand,
                    knowledge_dimension: crate::BloomKnowledgeDimension::ConceptualKnowledge,
                    classification_edit_number: crate::BloomClassificationEditNumber::INITIAL,
                }),
            },
            discipline_name: "Biology".to_string(),
            subject_name: "Genetics".to_string(),
            discipline_is_retired: false,
            prompt: QuestionDetailsPromptView::Static { blocks: Vec::new() },
            response_preview: Some(crate::QuestionResponsePreview::ShortText {}),
            evidence: QuestionStatistics::Unavailable,
            usage: QuestionUseDetails {
                summary: QuestionUseSummary {
                    global_course_count: 0,
                    global_assessment_count: 0,
                    own_course_count: 0,
                    own_assessment_count: 0,
                },
                own_courses: Vec::new(),
                own_courses_truncated: false,
            },
        };
        let wire = serde_json::to_value(detail).expect("detail serializes");
        assert_eq!(
            wire.get("subjectName"),
            Some(&serde_json::json!("Genetics"))
        );
        assert!(wire.get("source").is_none());
        assert!(wire.get("response").is_none());
        assert!(wire.get("questionVariationRule").is_none());
        assert!(wire.get("seed").is_none());
        assert!(wire.get("grading").is_none());
        assert!(wire.get("answerKey").is_none());
    }

    #[test]
    fn discovery_evidence_serializes_as_a_closed_explainable_union() {
        assert_eq!(
            serde_json::to_value(QuestionStatistics::Unavailable)
                .expect("unavailable Question Statistics serialize"),
            serde_json::json!({ "state": "unavailable" })
        );
    }

    #[test]
    fn question_revision_tuple_round_trips_camel_case_members() {
        let tuple = QuestionRevisionTuple {
            question_id: "ABCD-XEFG".parse().expect("fixture Question ID parses"),
            revision_number: QuestionRevisionNumber::new(1).expect("positive version"),
        };
        let json = serde_json::to_value(&tuple).expect("tuple serializes");
        assert_eq!(json["questionId"], "ABCD-XEFG");
        assert_eq!(json["revisionNumber"], 1);
        assert!(json.get("reference").is_none());
        let decoded: QuestionRevisionTuple =
            serde_json::from_value(json).expect("tuple deserializes from its members");
        assert_eq!(decoded, tuple);
    }

    #[test]
    fn question_revision_tuple_rejects_legacy_reference_json() {
        assert!(
            serde_json::from_str::<QuestionRevisionTuple>(
                r#"{"reference":{"questionId":"ABCD-XEFG","revisionNumber":1}}"#
            )
            .is_err()
        );
        assert!(
            serde_json::from_str::<QuestionRevisionTuple>(
                r#"{"questionId":"ABCD-XEFG","revisionNumber":1,"reference":"ABCD-XEFG"}"#
            )
            .is_err()
        );
        assert!(
            serde_json::from_value::<QuestionRevisionTuple>(serde_json::json!(1)).is_err(),
            "a lone Revision Number is not a Question Revision Tuple"
        );
    }
}
