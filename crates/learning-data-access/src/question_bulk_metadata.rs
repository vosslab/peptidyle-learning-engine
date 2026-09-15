//! Atomic, closed-field shared metadata edits for Published Questions.
//!
//! This is deliberately not a generic patch model.  The command names the
//! only three mutable shared search fields and carries a per-lineage metadata
//! precondition for every selected Question.

use std::collections::BTreeSet;

use async_trait::async_trait;
use question_model::QuestionId;
use uuid::Uuid;

use crate::{SessionTokenHash, StoreError};

/// Maximum number of Published Questions one shared-metadata command changes.
pub const MAX_BULK_QUESTION_METADATA_ITEMS: usize = 1000;

/// One exact metadata precondition supplied for a selected Published Question.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BulkPublishedQuestionMetadataSelection {
    /// Stable Published Question identity, never a Revision or source identity.
    pub question_id: QuestionId,
    /// Current independent metadata edit number for this Question lineage.
    pub metadata_edit_number: u64,
}

/// Closed replacement-or-clear patch for shared Published Question metadata.
///
/// `None` leaves a field unchanged.  `Some(None)` clears `subject` or `topic`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct BulkPublishedQuestionMetadataPatch {
    /// Complete replacement tags when present; an empty list intentionally clears tags.
    pub tags: Option<Vec<String>>,
    /// Complete replacement or explicit clear for the optional subject.
    pub subject: Option<Option<String>>,
    /// Complete replacement or explicit clear for the optional topic.
    pub topic: Option<Option<String>>,
}

impl BulkPublishedQuestionMetadataPatch {
    /// Refuses an empty or malformed patch before a database transaction opens.
    pub fn validate(&self) -> Result<(), StoreError> {
        if self.tags.is_none() && self.subject.is_none() && self.topic.is_none() {
            return Err(invalid("Bulk Published Question metadata patch is empty"));
        }
        if let Some(tags) = &self.tags {
            let mut distinct = BTreeSet::new();
            if tags
                .iter()
                .any(|tag| !valid_text(tag) || !distinct.insert(tag))
            {
                return Err(invalid("Bulk Published Question metadata tags are invalid"));
            }
        }
        for value in [
            self.subject.as_ref().and_then(|value| value.as_ref()),
            self.topic.as_ref().and_then(|value| value.as_ref()),
        ] {
            if value.is_some_and(|value| !valid_text(value)) {
                return Err(invalid("Bulk Published Question metadata text is invalid"));
            }
        }
        Ok(())
    }
}

/// Complete all-or-none metadata command input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BulkPublishedQuestionMetadataInput {
    /// Distinct selected Published Questions and their current metadata state.
    pub selection: Vec<BulkPublishedQuestionMetadataSelection>,
    /// Closed shared metadata replacement-or-clear patch.
    pub patch: BulkPublishedQuestionMetadataPatch,
    /// Browser-provided retry key; PostgreSQL binds it to the authenticated actor and request.
    pub idempotency_key: Uuid,
}

impl BulkPublishedQuestionMetadataInput {
    /// Validates the closed command shape without resolving Question existence or authority.
    pub fn validate(&self) -> Result<(), StoreError> {
        if self.selection.is_empty() || self.selection.len() > MAX_BULK_QUESTION_METADATA_ITEMS {
            return Err(invalid(
                "Bulk Published Question metadata selection is invalid",
            ));
        }
        let mut distinct = BTreeSet::new();
        if self.selection.iter().any(|selected| {
            selected.metadata_edit_number == 0
                || selected.metadata_edit_number > i64::MAX as u64
                || !distinct.insert(selected.question_id.clone())
        }) {
            return Err(invalid(
                "Bulk Published Question metadata selection is invalid",
            ));
        }
        self.patch.validate()
    }
}

/// One ordered current metadata receipt returned for a successful whole command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BulkPublishedQuestionMetadataResult {
    /// Stable Published Question identity.
    pub question_id: QuestionId,
    /// Metadata edit number after the accepted command.
    pub metadata_edit_number: u64,
}

/// The only special command failure: a retry key was reused for another request.
#[derive(Debug, Clone, PartialEq)]
pub enum BulkPublishedQuestionMetadataError {
    /// An existing actor-bound idempotency key has a different request digest.
    IdempotencyConflict,
    /// All remaining failures preserve normal Store semantics.
    Store(StoreError),
}

/// One active-vetted-Instructor atomic metadata command.
#[async_trait]
pub trait BulkPublishedQuestionMetadataStore: Send + Sync {
    /// Replaces or clears only the closed shared metadata fields for every selected Question.
    async fn bulk_replace_published_question_metadata(
        &self,
        session_token_hash: SessionTokenHash,
        input: BulkPublishedQuestionMetadataInput,
    ) -> Result<Vec<BulkPublishedQuestionMetadataResult>, BulkPublishedQuestionMetadataError>;
}

fn valid_text(value: &str) -> bool {
    value == value.trim()
        && (1..=120).contains(&value.chars().count())
        && !value.chars().any(char::is_control)
}

fn invalid(message: &str) -> StoreError {
    StoreError::InvalidRecord(message.to_owned())
}
