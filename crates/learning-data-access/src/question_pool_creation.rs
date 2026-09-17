//! Atomic creation of one reusable Published Question Pool Revision.
//!
//! Selection-count policy deliberately does not cross this boundary. It belongs
//! to the Assessment Entry, separate from immutable Pool membership and its
//! Instructor attestation.

use std::collections::BTreeSet;

use async_trait::async_trait;
use question_model::{
    MAX_QUESTION_POOL_ITEMS_PER_ASSESSMENT_ENTRY, QuestionId, QuestionRevisionReference,
};
use uuid::Uuid;

use crate::{SessionTokenHash, StoreError};

/// Complete server-owned create input for the first immutable Pool Revision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateQuestionPoolInput {
    /// Deliberate Pool-specific lineage Title and Description.
    pub title: String,
    pub description: String,
    /// Private stable storage identity minted by the trusted server operation.
    pub question_pool_id: Uuid,
    /// Fresh checksum-valid public Pool identity minted by the server issuer.
    pub public_question_pool_id: QuestionId,
    /// Ordered exact Published Question Revision pins.
    pub members: Vec<QuestionRevisionReference>,
    /// The Instructor affirms that these Questions are interchangeable.
    pub interchangeability_attested: bool,
}

impl CreateQuestionPoolInput {
    /// Refuses malformed or speculative Pool content before opening a transaction.
    pub fn validate(&self) -> Result<(), StoreError> {
        // ASVS 2.2.1/2.2.2: enforce canonical bounded text at the trusted boundary.
        if !(1..=question_model::MAX_QUESTION_TITLE_UNICODE_SCALARS)
            .contains(&self.title.chars().count())
            || !(1..=question_model::MAX_QUESTION_DESCRIPTION_UNICODE_SCALARS)
                .contains(&self.description.chars().count())
            || [&self.title, &self.description]
                .iter()
                // SQL btrim removes ASCII spaces, not all Unicode whitespace.
                .any(|text| {
                    text.as_str() != text.trim_matches(' ') || text.chars().any(char::is_control)
                })
        {
            return Err(StoreError::InvalidRecord(
                "Question Pool Title and Description must be canonical bounded nonempty text"
                    .to_owned(),
            ));
        }
        if self.members.is_empty()
            || self.members.len() > MAX_QUESTION_POOL_ITEMS_PER_ASSESSMENT_ENTRY
            || !self.interchangeability_attested
        {
            return Err(StoreError::InvalidRecord(
                "Question Pool creation requires bounded nonempty members and interchangeability attestation"
                    .to_owned(),
            ));
        }
        let mut distinct = BTreeSet::new();
        if self
            .members
            .iter()
            .any(|member| !distinct.insert(member.clone()))
        {
            return Err(StoreError::InvalidRecord(
                "Question Pool members must be distinct exact Published Question Revisions"
                    .to_owned(),
            ));
        }
        Ok(())
    }
}

/// The answer-free receipt for one newly-created first Pool Revision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreatedQuestionPool {
    /// Canonical public Question Pool identity.
    pub public_question_pool_id: QuestionId,
    /// The immutable first Pool Revision.
    pub revision_number: u64,
}

/// The only conclusive creation race for a freshly issued Pool ID.
#[derive(Debug, Clone, PartialEq)]
pub enum CreateQuestionPoolError {
    /// The fresh public Pool ID collided with an existing Pool identity.
    IdentityCollision,
    /// Any other result has normal persistence semantics.
    Store(StoreError),
}

/// Session-authorized atomic first-Revision persistence.
#[async_trait]
pub trait QuestionPoolCreationStore: Send + Sync {
    /// Creates one Pool lineage and its complete immutable Revision 1 together.
    async fn create_question_pool(
        &self,
        session_token_hash: SessionTokenHash,
        input: CreateQuestionPoolInput,
    ) -> Result<CreatedQuestionPool, CreateQuestionPoolError>;
}
