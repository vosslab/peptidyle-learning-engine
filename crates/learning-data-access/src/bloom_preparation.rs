//! Trusted preparation contract for one-use Bloom Classification receipts.
//!
//! Candidate facts remain typed and unhashed here. PostgreSQL owns the exact
//! fingerprint serialization used both to prepare and consume a receipt.

use async_trait::async_trait;
use objects::Sha256Checksum;
use question_model::{BloomClassification, QuestionRevisionReference};
use uuid::Uuid;

use crate::StoreError;

/// Internal identity for one target-kind-and-content-bound, one-use receipt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BloomPreparationReceiptId(Uuid);

impl BloomPreparationReceiptId {
    /// Wraps an identity read from trusted persistence.
    pub const fn from_uuid(value: Uuid) -> Self {
        Self(value)
    }

    /// Returns the UUID used by trusted persistence and publication wiring.
    pub const fn as_uuid(self) -> Uuid {
        self.0
    }
}

impl std::fmt::Display for BloomPreparationReceiptId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(formatter)
    }
}

/// Closed SQL target kinds for a prepared Bloom Classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum BloomPreparationTargetKind {
    QuestionRevision,
    QuestionPoolRevision,
}

impl BloomPreparationTargetKind {
    /// Returns the exact PostgreSQL target-kind value.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::QuestionRevision => "question_revision",
            Self::QuestionPoolRevision => "question_pool_revision",
        }
    }
}

/// Exact immutable candidate facts from which PostgreSQL derives its fingerprint.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BloomPreparationCandidate {
    /// One exact immutable Published Question source.
    Question {
        /// SHA-256 of the immutable source bytes for that Revision.
        source_checksum: Sha256Checksum,
    },
    /// One exact intended Pool Revision before its atomic publication.
    Pool {
        /// Exact candidate Pool Title.
        title: String,
        /// Exact candidate Pool Description.
        description: String,
        /// Ordered exact Published Question Revision members.
        members: Vec<QuestionRevisionReference>,
    },
}

impl BloomPreparationCandidate {
    /// Returns the SQL target kind implied by these candidate facts.
    pub const fn target_kind(&self) -> BloomPreparationTargetKind {
        match self {
            Self::Question { .. } => BloomPreparationTargetKind::QuestionRevision,
            Self::Pool { .. } => BloomPreparationTargetKind::QuestionPoolRevision,
        }
    }
}

/// Complete trusted input for preparing one semantic-content-bound receipt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrepareBloomClassificationInput {
    /// Candidate facts that SQL fingerprints and binds to the receipt.
    pub candidate: BloomPreparationCandidate,
    /// Complete trusted classification prepared for that exact candidate.
    pub classification: BloomClassification,
}

/// Private persistence boundary used after trusted classification completes.
#[async_trait]
pub trait BloomClassificationPreparationStore: Send + Sync {
    /// Persists a one-use receipt bound to the target kind and exact candidate digest.
    async fn prepare_bloom_classification(
        &self,
        input: PrepareBloomClassificationInput,
    ) -> Result<BloomPreparationReceiptId, StoreError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn target_kind_values_match_the_sql_contract() {
        assert_eq!(
            BloomPreparationTargetKind::QuestionRevision.as_str(),
            "question_revision"
        );
        assert_eq!(
            BloomPreparationTargetKind::QuestionPoolRevision.as_str(),
            "question_pool_revision"
        );
    }
}
