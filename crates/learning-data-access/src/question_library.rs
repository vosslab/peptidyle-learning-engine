//! Instructor-authorized Published Question discovery records.
//!
//! The Store returns immutable revision and private-source binding evidence to
//! server code only.  A Server Route resolves that evidence into an
//! answer-free browser view; neither this type nor its source locator is a
//! browser DTO.

use async_trait::async_trait;
use question_model::{
    QuestionAuthorship, QuestionBackend, QuestionId, QuestionLicense, QuestionRevisionAvailability,
    QuestionRevisionReference, SourceObjectChecksum, SourceObjectReference, Timestamp,
};

use crate::{SessionTokenHash, StoreError};

/// One Instructor-authorized Published Question plus its exact private source binding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublishedQuestionLibraryEntry {
    /// Stable Published Question and latest accepted Question Revision.
    pub question_revision: QuestionRevisionReference,
    /// The exact backend that must interpret the immutable source.
    pub backend: QuestionBackend,
    /// Database-authoritative publication time.
    pub published_at: Timestamp,
    /// Current shared Published Question title.
    pub question_title: String,
    /// Current shared Published Question description.
    pub question_description: String,
    /// Immutable reviewed public credit for this Question Revision.
    pub authorship: QuestionAuthorship,
    /// Whether the authenticated Account is an immutable Question Author.
    pub authored_by_current_account: bool,
    /// Immutable Question License for this Question Revision.
    pub question_license: QuestionLicense,
    /// Current selection availability for the exact revision.
    pub availability: QuestionRevisionAvailability,
    /// Private immutable Object Record locator; server use only.
    pub source_object_reference: SourceObjectReference,
    /// Checksum required before the server accepts private source bytes.
    pub source_object_checksum: SourceObjectChecksum,
    /// Source media type required before backend parsing.
    pub source_media_type: String,
}

/// Store for the authenticated Instructor Question Library read boundary.
#[async_trait]
pub trait QuestionLibraryStore: Send + Sync {
    /// Returns all current Instructor-discoverable entries with their exact
    /// immutable source bindings. The caller applies the bounded browser query
    /// and never serializes the private binding.
    async fn list_published_question_library_entries(
        &self,
        session_token_hash: SessionTokenHash,
    ) -> Result<Vec<PublishedQuestionLibraryEntry>, StoreError>;

    /// Resolves one stable Question ID after the same Instructor authorization.
    async fn load_published_question_library_entry(
        &self,
        session_token_hash: SessionTokenHash,
        question_id: &QuestionId,
    ) -> Result<PublishedQuestionLibraryEntry, StoreError>;
}
