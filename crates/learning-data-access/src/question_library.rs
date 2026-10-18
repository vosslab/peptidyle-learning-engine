//! Instructor-authorized Published Question discovery records.
//!
//! The Store returns immutable revision and private-source binding evidence to
//! server code only.  A Server Route resolves that evidence into an
//! answer-free browser view; neither this type nor its source locator is a
//! browser DTO.

use async_trait::async_trait;
use objects::ObjectRecord;
use question_model::{
    AccountId, BloomClassificationView, BloomCognitiveProcess, BloomKnowledgeDimension,
    LibrarySearchCategoryCounts, LibrarySearchKind, ObjectId, PublishedQuestionFilter,
    PublishedQuestionId, PublishedQuestionRevisionTuple, PublishedQuestionSharedMetadata,
    QuestionAuthorship, QuestionAvailability, QuestionAvailabilityEditNumber, QuestionBackend,
    QuestionFormat, QuestionLicense, QuestionPoolLibrarySummary, QuestionSearchAuthorFacet,
    QuestionSearchBackendFacet, QuestionSearchBloomCognitiveProcessFacet,
    QuestionSearchBloomKnowledgeDimensionFacet, QuestionSearchQuestionLicenseFacet,
    QuestionSearchSubjectFacet, QuestionSearchTagFacet, QuestionSearchTopicFacet, QuestionType,
    QuestionTypeFacet, SourceObjectChecksum, Timestamp,
};
use question_model::{LibraryFacetChoice, LibraryFacetField};
use uuid::Uuid;

use crate::{
    DiscoveryPageSize, NumberedDiscoveryPageRequest, NumberedPage, SessionTokenHash, StoreError,
};

/// One Instructor-authorized Published Question plus its exact private source binding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublishedQuestionLibraryEntry {
    /// Stable Published Question and latest accepted Question Revision.
    pub published_question_revision_tuple: PublishedQuestionRevisionTuple,
    /// Whether an immutable thumbnail relation exists for this exact authorized Revision.
    /// This is registration metadata, not a check of stored artifact health.
    pub has_question_thumbnail: bool,
    /// Exact immediate source Revision for a forked Question lineage.
    pub parent_published_question_revision_tuple: Option<PublishedQuestionRevisionTuple>,
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
    /// Exact Revision language, when supplied.
    pub language: Option<String>,
    /// Exact optional Revision citation.
    pub question_citation: Option<String>,
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
    /// Immutable reviewed public credit for this Question Revision.
    pub authorship: QuestionAuthorship,
    /// Whether the authenticated Account is an immutable Question Author.
    pub authored_by_current_account: bool,
    /// Whether the authenticated current Question Owner may invoke Archive.
    /// The server still reauthorizes the command at mutation time.
    pub viewer_may_archive: bool,
    /// Whether the authenticated viewer may edit metadata on the current available Revision.
    pub viewer_may_edit_metadata: bool,
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
    /// Immutable exact-revision source registry for Native compilation, independent
    /// of public image delivery readiness. Server use only.
    pub question_image_bindings: Vec<question_model::QuestionHtmlImageBinding>,
    /// Registered backend-private WeBWorK path for this exact Revision.
    /// Absent for Questions that do not use the WeBWorK backend.
    pub webwork_pg_path: Option<String>,
    /// Separate immutable support values used only when opening a correction Draft.
    pub revision_general_feedback: Option<String>,
    pub revision_hint: Option<String>,
    pub revision_worked_solution: Option<String>,
}

/// One exact private thumbnail Object Record authorized through the Question
/// Library's existing active-Instructor/Sysadmin visibility relation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublishedQuestionThumbnail {
    /// Immutable stored-object metadata. The caller derives the sole typed
    /// address from this value and revalidates the returned bytes.
    pub object_record: ObjectRecord,
    /// Stored thumbnail dimensions, independently constrained by the relation.
    pub intrinsic_width: u32,
    pub intrinsic_height: u32,
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

/// Store-only order choice for Question Library discovery.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuestionLibrarySearchSort {
    TitleAscending,
    TitleDescending,
    KindAscending,
    KindDescending,
    DisciplineAscending,
    DisciplineDescending,
    QuestionTypeAscending,
    QuestionTypeDescending,
    PublishedNewest,
    StarredDateAscending,
    StarredDateDescending,
}

/// Server-selected collection scope. Browser Library requests always use Public.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum QuestionLibrarySearchScope {
    #[default]
    Public,
    CurrentStarred,
}

/// Complete normalized filter plus numbered page selection for one store query.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuestionLibrarySearchRequest {
    pub scope: QuestionLibrarySearchScope,
    pub kind: LibrarySearchKind,
    pub questions: PublishedQuestionFilter,
    pub owner_account_id: Option<AccountId>,
    pub exact_question_id: Option<PublishedQuestionId>,
    pub text_terms: Vec<QuestionLibraryTextTerm>,
    pub author_names: Vec<String>,
    pub backends: QuestionLibraryBackendRestriction,
    pub tags: Vec<String>,
    pub subjects: Vec<String>,
    pub topics: Vec<String>,
    pub discipline_uuids: Vec<Uuid>,
    pub subject_uuids: Vec<Uuid>,
    pub topic_uuids: Vec<Uuid>,
    pub subtopic_uuids: Vec<Uuid>,
    pub cross_discipline: bool,
    pub bloom_cognitive_processes: Vec<BloomCognitiveProcess>,
    pub bloom_knowledge_dimensions: Vec<BloomKnowledgeDimension>,
    pub question_types: Vec<QuestionType>,
    pub question_licenses: Vec<QuestionLicense>,
    pub authored_by_current_account: bool,
    pub sort: QuestionLibrarySearchSort,
    pub page: NumberedDiscoveryPageRequest,
    /// Excludes exact Question identities before the complete result set is counted and paged.
    /// Normal Library requests leave this empty; picker callers use it only for fixed members.
    pub excluded_question_ids: Vec<PublishedQuestionId>,
}

/// One selected bounded page and a typed continuation position for its final row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LibrarySearchEntry {
    Question {
        entry: Box<PublishedQuestionLibraryEntry>,
        owner_account_id: AccountId,
        starred_at_micros: Option<i64>,
    },
    Pool {
        summary: Box<QuestionPoolLibrarySummary>,
        created_at: Timestamp,
        subject_name: String,
        topic_name: Option<String>,
        subtopic_name: Option<String>,
    },
}

impl LibrarySearchEntry {
    pub fn public_id(&self) -> &str {
        match self {
            Self::Question { entry, .. } => entry
                .published_question_revision_tuple
                .published_question_id
                .as_str(),
            Self::Pool { summary, .. } => summary.question_pool_id.as_str(),
        }
    }

    pub fn title(&self) -> &str {
        match self {
            Self::Question { entry, .. } => &entry.question_title,
            Self::Pool { summary, .. } => &summary.metadata.title,
        }
    }

    pub fn sort_time(&self) -> Timestamp {
        match self {
            Self::Question { entry, .. } => entry.published_at,
            Self::Pool { created_at, .. } => *created_at,
        }
    }

    pub fn starred_at_micros(&self) -> Option<i64> {
        match self {
            Self::Question {
                starred_at_micros, ..
            } => *starred_at_micros,
            Self::Pool { .. } => None,
        }
    }
}

/// One globally filtered and ordered numbered page with Question Library facets.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuestionLibrarySearchPage {
    pub page: NumberedPage<LibrarySearchEntry, DiscoveryPageSize>,
    /// Complete authorized-query aggregates, evaluated before numbered paging.
    pub facets: QuestionLibrarySearchFacets,
}

/// One choice enumeration over the complete authorized matching collection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuestionLibraryFacetChoiceRequest {
    pub search: QuestionLibrarySearchRequest,
    pub field: LibraryFacetField,
    pub choice_search: Option<String>,
    pub after_key: Option<String>,
    pub selected_keys: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuestionLibraryFacetChoiceResult {
    pub choices: Vec<LibraryFacetChoice>,
    pub selected: Vec<LibraryFacetChoice>,
    pub next_key: Option<String>,
}

/// Database-owned Question Library facets before the server folds existing
/// backend capability declarations into the public capability facets.
///
/// The database counts the complete metadata predicate intersection. It never
/// reads Question source or learns a second backend capability model.
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QuestionLibrarySearchFacets {
    pub categories: LibrarySearchCategoryCounts,
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
    pub bloom_cognitive_processes: Vec<QuestionSearchBloomCognitiveProcessFacet>,
    pub bloom_knowledge_dimensions: Vec<QuestionSearchBloomKnowledgeDimensionFacet>,
}

/// Store for the authenticated Instructor Question Library read boundary.
#[async_trait]
pub trait QuestionLibraryStore: Send + Sync {
    /// Resolves one exact published Revision's private thumbnail only after
    /// the ordinary Question Library visibility relation authorizes it.
    async fn resolve_question_thumbnail(
        &self,
        session_token_hash: SessionTokenHash,
        published_question_revision_tuple: &PublishedQuestionRevisionTuple,
    ) -> Result<PublishedQuestionThumbnail, StoreError>;

    async fn list_question_library_facet_choices(
        &self,
        session_token_hash: SessionTokenHash,
        request: QuestionLibraryFacetChoiceRequest,
    ) -> Result<QuestionLibraryFacetChoiceResult, StoreError>;

    /// Selects one globally filtered and ordered Question Library page.
    ///
    /// The server parses text grammar and validates the numbered request; the
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

    /// Loads one bounded, all-or-none current shared-metadata snapshot.
    ///
    /// The database returns only available accepted Published Question
    /// lineages for an active Instructor, in canonical Question-ID
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
