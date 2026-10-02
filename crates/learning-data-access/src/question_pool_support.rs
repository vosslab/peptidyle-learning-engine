//! Optional PLE-managed Hint, Question Feedback, and Worked Solution for one Question Pool.
//!
//! The texts are current Pool state. A save advances the metadata token and
//! does not carry member Question identities.

use async_trait::async_trait;
use question_model::{QuestionPoolId, QuestionPoolMetadataEditNumber};

use crate::{SessionTokenHash, StoreError};

/// One Pool's optional support texts and the metadata token last read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuestionPoolPleManagedSupport {
    /// Stable Question Pool identity.
    pub question_pool_id: QuestionPoolId,
    /// Current metadata token. Support replacement advances it once.
    pub question_pool_metadata_edit_number: QuestionPoolMetadataEditNumber,
    /// Optional Pool Hint. Absent when the Instructor clears it.
    pub hint: Option<String>,
    /// Optional Pool Question Feedback. Absent when the Instructor clears it.
    pub general_feedback: Option<String>,
    /// Optional Pool Worked Solution. Absent when the Instructor clears it.
    pub worked_solution: Option<String>,
}

impl QuestionPoolPleManagedSupport {
    /// Refuses an empty Edit Number or text the Pool columns cannot store.
    pub fn validate(&self) -> Result<(), StoreError> {
        if self.question_pool_metadata_edit_number.get() > i64::MAX as u64 {
            return Err(invalid("Question Pool support Edit Number is invalid"));
        }
        for value in [&self.hint, &self.general_feedback, &self.worked_solution]
            .into_iter()
            .flatten()
        {
            if !valid_support_text(value) {
                return Err(invalid("Question Pool support text is invalid"));
            }
        }
        Ok(())
    }
}

/// Active-vetted-Instructor read and replacement of one Pool's support texts.
#[async_trait]
pub trait QuestionPoolSupportStore: Send + Sync {
    /// Reads the Pool texts without member Question or Student Work fields.
    async fn read_question_pool_ple_managed_support(
        &self,
        session_token_hash: SessionTokenHash,
        question_pool_id: QuestionPoolId,
    ) -> Result<QuestionPoolPleManagedSupport, StoreError>;

    /// Replaces the three Pool texts and advances the metadata token once.
    async fn save_question_pool_ple_managed_support(
        &self,
        session_token_hash: SessionTokenHash,
        support: QuestionPoolPleManagedSupport,
    ) -> Result<QuestionPoolPleManagedSupport, StoreError>;
}

fn valid_support_text(value: &str) -> bool {
    value == value.trim()
        && (1..=4000).contains(&value.chars().count())
        && !value.chars().any(char::is_control)
}

fn invalid(message: &str) -> StoreError {
    StoreError::InvalidRecord(message.to_owned())
}
