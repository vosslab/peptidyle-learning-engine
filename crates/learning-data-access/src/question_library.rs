//! Instructor-authorized Published Question discovery records.
//!
//! The Store returns immutable revision and private-source binding evidence to
//! server code only.  A Server Route resolves that evidence into an
//! answer-free browser view; neither this type nor its source locator is a
//! browser DTO.

use async_trait::async_trait;
use question_model::{
    BloomClassificationEditNumber, BloomClassificationView, BloomCognitiveProcess,
    BloomKnowledgeDimension, ObjectId, PublishedQuestionId, PublishedQuestionRevisionTuple,
    PublishedQuestionSharedMetadata, QuestionAuthorship, QuestionAvailability,
    QuestionAvailabilityEditNumber, QuestionBackend, QuestionFormat, QuestionLicense,
    QuestionSearchAuthorFacet, QuestionSearchBackendFacet,
    QuestionSearchBloomCognitiveProcessFacet, QuestionSearchBloomKnowledgeDimensionFacet,
    QuestionSearchCourseUseFacet, QuestionSearchQuestionLicenseFacet, QuestionSearchSubjectFacet,
    QuestionSearchTagFacet, QuestionSearchTopicFacet, QuestionType, QuestionTypeFacet,
    SourceObjectChecksum, Timestamp,
};
use uuid::Uuid;

use crate::{SessionTokenHash, StoreError};

/// One Instructor-authorized Published Question plus its exact private source binding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublishedQuestionLibraryEntry {
    /// Stable Published Question and latest accepted Question Revision.
    pub published_question_revision_tuple: PublishedQuestionRevisionTuple,
    /// The exact backend that must interpret the immutable source.
    pub backend: QuestionBackend,
    /// Immutable reviewed source representation. This is browser-safe metadata,
    /// not a source path or backend-private configuration.
    pub question_format: QuestionFormat,
    /// Immutable author-declared educational type of this revision.
    pub question_type: QuestionType,
    /// Database-authoritative publication time.
    pub published_at: Timestamp,
    /// Exact Bloom Classification for `published_question_revision_tuple`, when assigned.
    pub bloom: Option<BloomClassificationView>,
    /// Current shared Published Question title.
    pub question_title: String,
    /// Current shared Published Question description.
    pub question_description: String,
    /// Current shared search metadata and its independent edit number.
    pub shared_metadata: PublishedQuestionSharedMetadata,
    /// Vocabulary-resolved names used by existing human-readable search facets.
    pub subject_name: String,
    pub topic_name: Option<String>,
    pub discipline_name: String,
    /// Whether this existing classification reference is retired. Retired
    /// names remain resolvable for historical/current reads but cannot be
    /// selected for new classification.
    pub discipline_is_retired: bool,
    pub subtopic_name: Option<String>,
    /// Whether a current available entry in one of the viewer's Courses uses
    /// this stable Question lineage.
    pub used_in_current_account_courses: bool,
    /// Immutable reviewed public credit for this Question Revision.
    pub authorship: QuestionAuthorship,
    /// Whether the authenticated Account is an immutable Question Author.
    pub authored_by_current_account: bool,
    /// Whether the authenticated current Question Owner may invoke Archive.
    /// The server still reauthorizes the command at mutation time.
    pub viewer_may_archive: bool,
    /// Immutable Question License for this Question Revision.
    pub question_license: QuestionLicense,
    /// Current selection availability of the stable Question lineage.
    pub availability: QuestionAvailability,
    /// Qualified current availability state of the stable Question lineage.
    pub availability_edit_number: QuestionAvailabilityEditNumber,
    /// Private immutable Object Record locator; server use only.
    pub source_object_id: ObjectId,
    /// Checksum required before the server accepts private source bytes.
    pub source_object_checksum: SourceObjectChecksum,
    /// Source media type required before backend parsing.
    pub source_media_type: String,
    /// Registered backend-private WeBWorK path for this exact Revision.
    /// Absent for Questions that do not use the WeBWorK backend.
    pub webwork_pg_path: Option<String>,
}

/// Database-confirmed current availability after one lineage transition.
///
/// The database records the corresponding actor-attributed immutable event in
/// the same transaction.  The Store returns current state because it is the
/// value needed for the next qualified transition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublishedQuestionAvailability {
    /// New current availability of the stable Published Question lineage.
    pub availability: QuestionAvailability,
    /// New qualified edit number of that lineage.
    pub edit_number: QuestionAvailabilityEditNumber,
}

/// Server-normalized predicate for one Question Library text token.
///
/// The HTTP grammar remains server-owned.  This store value only records its
/// already-parsed meaning, so PostgreSQL receives no search-language syntax.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuestionLibraryTextTerm {
    pub field: QuestionLibraryTextField,
    pub value: String,
    pub excluded: bool,
}

/// Closed database field vocabulary for parsed Question Library text terms.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuestionLibraryTextField {
    Any,
    Discipline,
    Subtopic,
    Subject,
    Topic,
    Tags,
    QuestionType,
    Author,
}

/// Backend eligibility derived by the server from adapter capability declarations.
///
/// `Only(Vec::new())` deliberately represents an impossible capability
/// combination; it differs from an unrestricted `Any` query.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum QuestionLibraryBackendRestriction {
    #[default]
    Any,
    Only(Vec<QuestionBackend>),
}

/// Store-only order choice for Question Library keyset discovery.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuestionLibrarySearchSort {
    TitleAscending,
    PublishedNewest,
}

/// Typed keyset position decoded and authenticated by the server route.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QuestionLibrarySearchCursorPosition {
    TitleAscending {
        title: String,
        question_id: PublishedQuestionId,
    },
    PublishedNewest {
        published_at_millis: i64,
        question_id: PublishedQuestionId,
    },
}

/// Complete normalized filter plus bounded page selection for one store query.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuestionLibrarySearchRequest {
    pub exact_question_id: Option<PublishedQuestionId>,
    pub text_terms: Vec<QuestionLibraryTextTerm>,
    pub author_names: Vec<String>,
    pub backends: QuestionLibraryBackendRestriction,
    pub tags: Vec<String>,
    pub subjects: Vec<String>,
    pub topics: Vec<String>,
    pub discipline_uuid: Option<Uuid>,
    pub subject_uuid: Option<Uuid>,
    pub topic_uuid: Option<Uuid>,
    pub subtopic_uuid: Option<Uuid>,
    pub cross_discipline: bool,
    pub bloom_cognitive_process: Option<BloomCognitiveProcess>,
    pub bloom_knowledge_dimension: Option<BloomKnowledgeDimension>,
    pub question_types: Vec<QuestionType>,
    pub question_licenses: Vec<QuestionLicense>,
    pub used_in_current_account_courses: bool,
    pub authored_by_current_account: bool,
    pub sort: QuestionLibrarySearchSort,
    pub page_size: u16,
    pub after: Option<QuestionLibrarySearchCursorPosition>,
}

/// One selected bounded page and a typed continuation position for its final row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuestionLibrarySearchPage {
    pub items: Vec<PublishedQuestionLibraryEntry>,
    pub next_position: Option<QuestionLibrarySearchCursorPosition>,
    /// Complete authorized-query aggregates, evaluated before cursor paging.
    pub facets: QuestionLibrarySearchFacets,
}

/// Database-owned Question Library facets before the server folds existing
/// backend capability declarations into the public capability facets.
///
/// The database counts the complete metadata predicate intersection. It never
/// reads Question source or learns a second backend capability model.
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QuestionLibrarySearchFacets {
    pub author_names: Vec<QuestionSearchAuthorFacet>,
    pub author_names_truncated: bool,
    pub backends: Vec<QuestionSearchBackendFacet>,
    pub tags: Vec<QuestionSearchTagFacet>,
    pub tags_truncated: bool,
    pub subjects: Vec<QuestionSearchSubjectFacet>,
    pub subjects_truncated: bool,
    pub topics: Vec<QuestionSearchTopicFacet>,
    pub topics_truncated: bool,
    pub question_types: Vec<QuestionTypeFacet>,
    pub question_licenses: Vec<QuestionSearchQuestionLicenseFacet>,
    pub used_in_my_courses: QuestionSearchCourseUseFacet,
    pub bloom_cognitive_processes: Vec<QuestionSearchBloomCognitiveProcessFacet>,
    pub bloom_knowledge_dimensions: Vec<QuestionSearchBloomKnowledgeDimensionFacet>,
}

/// Store for the authenticated Instructor Question Library read boundary.
#[async_trait]
pub trait QuestionLibraryStore: Send + Sync {
    /// Selects one globally filtered and ordered Question Library page.
    ///
    /// The server parses text grammar and validates opaque cursor binding; the
    /// store applies only this typed request under current session authority.
    async fn search_published_question_library_entries(
        &self,
        session_token_hash: SessionTokenHash,
        request: QuestionLibrarySearchRequest,
    ) -> Result<QuestionLibrarySearchPage, StoreError>;

    /// Returns current Instructor-discoverable entries with their exact
    /// immutable source bindings. Archived lineages are excluded before the
    /// caller applies its bounded browser query; the private binding is never
    /// serialized.
    async fn list_published_question_library_entries(
        &self,
        session_token_hash: SessionTokenHash,
    ) -> Result<Vec<PublishedQuestionLibraryEntry>, StoreError>;

    /// Resolves one stable Question ID for ordinary current browsing.
    ///
    /// Archived lineages are not resolvable through this discovery path.
    async fn load_published_question_library_entry(
        &self,
        session_token_hash: SessionTokenHash,
        question_id: &PublishedQuestionId,
    ) -> Result<PublishedQuestionLibraryEntry, StoreError>;

    /// Resolves one existing exact immutable Question Revision after the same
    /// Instructor authorization.
    ///
    /// This historical-resolution path intentionally remains available after
    /// the stable lineage is archived. It is not an ordinary new-selection
    /// path.
    async fn load_published_question_revision_library_entry(
        &self,
        session_token_hash: SessionTokenHash,
        published_question_revision_tuple: &PublishedQuestionRevisionTuple,
    ) -> Result<PublishedQuestionLibraryEntry, StoreError>;

    /// Corrects both Bloom dimensions for one exact Revision through the
    /// classification-owned compare-and-swap number. PostgreSQL authorizes the
    /// active vetted Instructor and checks stale state before accepting a no-op.
    async fn correct_question_revision_bloom(
        &self,
        session_token_hash: SessionTokenHash,
        published_question_revision_tuple: &PublishedQuestionRevisionTuple,
        expected_edit_number: BloomClassificationEditNumber,
        cognitive_process: BloomCognitiveProcess,
        knowledge_dimension: BloomKnowledgeDimension,
    ) -> Result<BloomClassificationView, StoreError>;

    /// Loads one bounded, all-or-none current shared-metadata snapshot.
    ///
    /// The database returns only available accepted Published Question
    /// lineages for an active vetted Instructor, in canonical Question-ID
    /// order. Any unavailable target conceals the whole selection.
    async fn load_current_published_question_shared_metadata(
        &self,
        session_token_hash: SessionTokenHash,
        question_ids: &[PublishedQuestionId],
    ) -> Result<Vec<PublishedQuestionSharedMetadata>, StoreError>;

    /// Archives one owned stable Question lineage at its exact availability
    /// edit number after the caller has provided the exact current title.
    ///
    /// PostgreSQL locks the lineage, verifies authority and confirmation, and
    /// records the immutable availability event atomically.
    async fn archive_published_question(
        &self,
        session_token_hash: SessionTokenHash,
        question_id: &PublishedQuestionId,
        expected_edit_number: QuestionAvailabilityEditNumber,
        confirmation_title: &str,
    ) -> Result<PublishedQuestionAvailability, StoreError>;

    /// Restores one owned stable Question lineage at its exact availability
    /// edit number. PostgreSQL records the immutable restore event atomically.
    async fn restore_published_question(
        &self,
        session_token_hash: SessionTokenHash,
        question_id: &PublishedQuestionId,
        expected_edit_number: QuestionAvailabilityEditNumber,
    ) -> Result<PublishedQuestionAvailability, StoreError>;
}
