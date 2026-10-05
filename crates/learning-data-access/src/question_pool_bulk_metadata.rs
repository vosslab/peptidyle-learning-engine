//! Atomic Topic, Subtopic, and Tag edits for many Question Pools.
//!
//! Discipline and Subject stay the values established by each Pool's first
//! member. Search metadata uses a mutable-metadata token separate from membership.

use std::collections::BTreeSet;

use async_trait::async_trait;
use question_model::{
    MAX_BULK_QUESTION_METADATA_ITEMS, QuestionPoolId, QuestionPoolMetadataEditNumber,
};

use crate::{SessionTokenHash, StoreError};

/// One Pool and the metadata token the caller last read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BulkQuestionPoolSearchMetadataSelection {
    /// Stable Question Pool identity.
    pub question_pool_id: QuestionPoolId,
    /// Current mutable metadata token.
    pub question_pool_metadata_edit_number: QuestionPoolMetadataEditNumber,
}

/// Closed replacement-or-clear patch for Pool-owned search metadata.
///
/// `None` leaves a field unchanged. `Some(None)` clears Topic or Subtopic.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct BulkQuestionPoolSearchMetadataPatch {
    /// Complete replacement tags when present. An empty list clears tags.
    pub tags: Option<Vec<String>>,
    /// Optional Topic replacement or an explicit clear.
    pub topic_uuid: Option<Option<uuid::Uuid>>,
    /// Optional Subtopic replacement or an explicit clear.
    pub subtopic_uuid: Option<Option<uuid::Uuid>>,
}

impl BulkQuestionPoolSearchMetadataPatch {
    /// Refuses an empty patch or an invalid tag before a database transaction opens.
    pub fn validate_fields(&self) -> Result<(), StoreError> {
        if self.tags.is_none() && self.topic_uuid.is_none() && self.subtopic_uuid.is_none() {
            return Err(invalid("Question Pool search metadata patch is empty"));
        }
        if let Some(tags) = &self.tags {
            let mut distinct = BTreeSet::new();
            if tags
                .iter()
                .any(|tag| !valid_text(tag, 120) || !distinct.insert(tag))
            {
                return Err(invalid("Question Pool search metadata tags are invalid"));
            }
        }
        Ok(())
    }
}

/// Complete all-or-none Question Pool search-metadata command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BulkQuestionPoolSearchMetadataInput {
    /// Distinct selected Question Pools and their current Edit Numbers.
    pub selection: Vec<BulkQuestionPoolSearchMetadataSelection>,
    /// Topic, Subtopic, and Tag replacements.
    pub patch: BulkQuestionPoolSearchMetadataPatch,
}

impl BulkQuestionPoolSearchMetadataInput {
    /// Validates the closed command shape without resolving Pool existence.
    pub fn validate(&self) -> Result<(), StoreError> {
        if self.selection.is_empty() || self.selection.len() > MAX_BULK_QUESTION_METADATA_ITEMS {
            return Err(invalid(
                "Question Pool search metadata selection is invalid",
            ));
        }
        let mut distinct = BTreeSet::new();
        if self.selection.iter().any(|selected| {
            selected.question_pool_metadata_edit_number.get() > i64::MAX as u64
                || !distinct.insert(selected.question_pool_id.clone())
        }) {
            return Err(invalid(
                "Question Pool search metadata selection is invalid",
            ));
        }
        self.patch.validate_fields()
    }
}

/// One Pool after an accepted search-metadata command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BulkQuestionPoolSearchMetadataResult {
    /// Stable Question Pool identity.
    pub question_pool_id: QuestionPoolId,
    /// Metadata token after the accepted replacement.
    pub question_pool_metadata_edit_number: QuestionPoolMetadataEditNumber,
}

/// One active-Instructor command for many Question Pools.
#[async_trait]
pub trait BulkQuestionPoolSearchMetadataStore: Send + Sync {
    /// Replaces Topic, Subtopic, and Tags, advancing each selected Pool metadata token once.
    async fn bulk_replace_question_pool_search_metadata(
        &self,
        session_token_hash: SessionTokenHash,
        input: BulkQuestionPoolSearchMetadataInput,
    ) -> Result<Vec<BulkQuestionPoolSearchMetadataResult>, StoreError>;
}

fn valid_text(value: &str, maximum: usize) -> bool {
    value == value.trim()
        && (1..=maximum).contains(&value.chars().count())
        && !value.chars().any(char::is_control)
}

fn invalid(message: &str) -> StoreError {
    StoreError::InvalidRecord(message.to_owned())
}
