//! Instructor-authorized Published Question discovery records.
//!
//! The Store returns immutable revision and private-source binding evidence to
//! server code only.  A Server Route resolves that evidence into an
//! answer-free browser view; neither this type nor its source locator is a
//! browser DTO.

use async_trait::async_trait;
use question_model::{
    PublishedQuestionSharedMetadata, QuestionAuthorship, QuestionAvailability,
    QuestionAvailabilityEditNumber, QuestionBackend, QuestionFormat, QuestionId, QuestionLicense,
    QuestionRevisionReference, QuestionType, SourceObjectChecksum, SourceObjectReference,
    Timestamp,
};

use crate::{SessionTokenHash, StoreError};

/// One Instructor-authorized Published Question plus its exact private source binding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublishedQuestionLibraryEntry {
    /// Stable Published Question and latest accepted Question Revision.
    pub question_revision: QuestionRevisionReference,
    /// The exact backend that must interpret the immutable source.
    pub backend: QuestionBackend,
    /// Immutable reviewed source representation. This is browser-safe metadata,
    /// not a source path or backend-private configuration.
    pub question_format: QuestionFormat,
    /// Immutable author-declared educational type of this revision.
    pub question_type: QuestionType,
    /// Database-authoritative publication time.
    pub published_at: Timestamp,
    /// Current shared Published Question title.
    pub question_title: String,
    /// Current shared Published Question description.
    pub question_description: String,
    /// Current shared search metadata and its independent edit number.
    pub shared_metadata: PublishedQuestionSharedMetadata,
    /// Vocabulary-resolved names used by existing human-readable search facets.
    pub subject_name: String,
    pub topic_name: Option<String>,
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
    pub source_object_reference: SourceObjectReference,
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

/// Store for the authenticated Instructor Question Library read boundary.
#[async_trait]
pub trait QuestionLibraryStore: Send + Sync {
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
        question_id: &QuestionId,
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
        question_revision: &QuestionRevisionReference,
    ) -> Result<PublishedQuestionLibraryEntry, StoreError>;

    /// Loads one bounded, all-or-none current shared-metadata snapshot.
    ///
    /// The database returns only available accepted Published Question
    /// lineages for an active vetted Instructor, in canonical Question-ID
    /// order. Any unavailable target conceals the whole selection.
    async fn load_current_published_question_shared_metadata(
        &self,
        session_token_hash: SessionTokenHash,
        question_ids: &[QuestionId],
    ) -> Result<Vec<PublishedQuestionSharedMetadata>, StoreError>;

    /// Archives one owned stable Question lineage at its exact availability
    /// edit number after the caller has provided the exact current title.
    ///
    /// PostgreSQL locks the lineage, verifies authority and confirmation, and
    /// records the immutable availability event atomically.
    async fn archive_published_question(
        &self,
        session_token_hash: SessionTokenHash,
        question_id: &QuestionId,
        expected_edit_number: QuestionAvailabilityEditNumber,
        confirmation_title: &str,
    ) -> Result<PublishedQuestionAvailability, StoreError>;

    /// Restores one owned stable Question lineage at its exact availability
    /// edit number. PostgreSQL records the immutable restore event atomically.
    async fn restore_published_question(
        &self,
        session_token_hash: SessionTokenHash,
        question_id: &QuestionId,
        expected_edit_number: QuestionAvailabilityEditNumber,
    ) -> Result<PublishedQuestionAvailability, StoreError>;
}
