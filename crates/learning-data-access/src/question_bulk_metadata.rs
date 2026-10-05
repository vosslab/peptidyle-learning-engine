//! Atomic, closed-field shared metadata edits for Published Questions.
//!
//! This is deliberately not a generic patch model.  The command names the
//! mutable shared search fields and carries a per-lineage metadata
//! precondition for every selected Question.

use std::collections::BTreeSet;

use async_trait::async_trait;
use question_model::{MAX_BULK_QUESTION_METADATA_ITEMS, PublishedQuestionId};

use crate::{SessionTokenHash, StoreError};

/// One exact metadata precondition supplied for a selected Published Question.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BulkPublishedQuestionMetadataSelection {
    /// Stable Published Question identity, never a Revision or source identity.
    pub question_id: PublishedQuestionId,
    /// Current independent metadata edit number for this Question lineage.
    pub metadata_edit_number: u64,
    /// Replacement Title for this Question. Absent leaves the stored Title unchanged.
    pub question_title: Option<String>,
    /// Replacement Description for this Question. Absent leaves the stored Description unchanged.
    pub question_description: Option<String>,
}

/// Closed replacement-or-clear patch for shared Published Question metadata.
///
/// `None` leaves a field unchanged. `Some(None)` clears an optional classification.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct BulkPublishedQuestionMetadataPatch {
    /// Complete replacement tags when present; an empty list intentionally clears tags.
    pub tags: Option<Vec<String>>,
    /// Required classification replacements cannot be cleared.
    pub discipline_uuid: Option<uuid::Uuid>,
    pub subject_uuid: Option<uuid::Uuid>,
    /// Optional classification replacements permit an explicit clear.
    pub topic_uuid: Option<Option<uuid::Uuid>>,
    pub subtopic_uuid: Option<Option<uuid::Uuid>>,
}

impl BulkPublishedQuestionMetadataPatch {
    /// Refuses a malformed shared patch before a database transaction opens.
    ///
    /// An empty patch is valid here. The command requires either this patch or a
    /// per-Question Title or Description.
    pub fn validate_fields(&self) -> Result<(), StoreError> {
        if let Some(tags) = &self.tags {
            let mut distinct = BTreeSet::new();
            if tags
                .iter()
                .any(|tag| !valid_text(tag, 120) || !distinct.insert(tag))
            {
                return Err(invalid("Bulk Published Question metadata tags are invalid"));
            }
        }
        Ok(())
    }

    fn is_empty(&self) -> bool {
        self.tags.is_none()
            && self.discipline_uuid.is_none()
            && self.subject_uuid.is_none()
            && self.topic_uuid.is_none()
            && self.subtopic_uuid.is_none()
    }
}

/// Complete all-or-none metadata command input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BulkPublishedQuestionMetadataInput {
    /// Distinct selected Published Questions and their current metadata state.
    pub selection: Vec<BulkPublishedQuestionMetadataSelection>,
    /// Closed shared metadata replacement-or-clear patch.
    pub patch: BulkPublishedQuestionMetadataPatch,
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
                || selected
                    .question_title
                    .as_deref()
                    .is_some_and(|title| !valid_text(title, 512))
                || selected
                    .question_description
                    .as_deref()
                    .is_some_and(|description| !valid_text(description, 4_000))
        }) {
            return Err(invalid(
                "Bulk Published Question metadata selection is invalid",
            ));
        }
        self.patch.validate_fields()?;
        // ASVS 2.2.1: a command must change shared metadata or one Question's own text.
        if self.patch.is_empty()
            && self.selection.iter().all(|selected| {
                selected.question_title.is_none() && selected.question_description.is_none()
            })
        {
            return Err(invalid("Bulk Published Question metadata patch is empty"));
        }
        Ok(())
    }
}

/// One ordered current metadata result returned for a successful whole command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BulkPublishedQuestionMetadataResult {
    /// Stable Published Question identity.
    pub question_id: PublishedQuestionId,
    /// Metadata edit number after the accepted command.
    pub metadata_edit_number: u64,
}

/// One active-Instructor atomic metadata command.
#[async_trait]
pub trait BulkPublishedQuestionMetadataStore: Send + Sync {
    /// Replaces or clears only the closed shared metadata fields for every selected Question.
    async fn bulk_replace_published_question_metadata(
        &self,
        session_token_hash: SessionTokenHash,
        input: BulkPublishedQuestionMetadataInput,
    ) -> Result<Vec<BulkPublishedQuestionMetadataResult>, StoreError>;
}

fn valid_text(value: &str, maximum: usize) -> bool {
    value == value.trim()
        && (1..=maximum).contains(&value.chars().count())
        && !value.chars().any(char::is_control)
}

fn invalid(message: &str) -> StoreError {
    StoreError::InvalidRecord(message.to_owned())
}
