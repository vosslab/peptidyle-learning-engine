//! Immutable source-copy Change Proposals and one receiving-owner acceptance.

use async_trait::async_trait;
use question_model::{
    AccountId, BlueprintEditNumber, BlueprintRevisionReference, CanonicalBlueprintCourse, Timestamp,
};

use crate::{Page, PageRequest, SessionTokenHash, StoreError, StoredBlueprintRevision};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Explicit whole-content or whole-Assessment/layout acceptance, not a JSON importer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum BlueprintChangeProposalDecision {
    Entire,
    Selected {
        selection: question_model::blueprint_course::BlueprintForkApplySelection,
        source_short_name: bool,
        source_long_name: bool,
        source_classification: bool,
    },
}

/// Both current target guards must equal the immutable Proposal comparison basis.
#[derive(Debug, Clone)]
pub struct AcceptBlueprintChangeProposalInput {
    pub proposal_id: uuid::Uuid,
    pub expected_target: BlueprintRevisionReference,
    pub expected_target_blueprint_edit_number: BlueprintEditNumber,
    pub decision: BlueprintChangeProposalDecision,
}

/// Immutable exact decision and trusted new-copy destination mappings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct BlueprintChangeProposalAcceptedDecision {
    pub decision: BlueprintChangeProposalDecision,
    pub applied_selection: question_model::blueprint_course::BlueprintForkApplySelection,
    pub new_modules: BTreeMap<
        question_model::BlueprintModuleReference,
        question_model::BlueprintModuleReference,
    >,
    pub new_assessments:
        BTreeMap<question_model::BlueprintAssessmentId, question_model::BlueprintAssessmentId>,
}

/// Exact accepted result, reconstructed from its immutable Revision/metadata pins.
#[derive(Debug, Clone, PartialEq)]
pub struct AcceptedBlueprintChangeProposal {
    pub proposal_id: uuid::Uuid,
    pub actor: AccountId,
    pub accepted_at: Timestamp,
    pub target: BlueprintRevisionReference,
    pub target_blueprint_edit_number: BlueprintEditNumber,
    pub decision: BlueprintChangeProposalAcceptedDecision,
    pub resulting_json: CanonicalBlueprintCourse,
}

/// Exact reviewed content and independent lineage metadata; no client JSON input.
#[derive(Debug, Clone)]
pub struct CreateBlueprintChangeProposalInput {
    pub source: BlueprintRevisionReference,
    pub source_blueprint_edit_number: BlueprintEditNumber,
    pub target: BlueprintRevisionReference,
    pub target_blueprint_edit_number: BlueprintEditNumber,
}

/// Frozen Proposal evidence reconstructed from existing immutable fact sequences.
#[derive(Debug, Clone, PartialEq)]
pub struct StoredBlueprintChangeProposal {
    pub proposal_id: uuid::Uuid,
    pub proposer: AccountId,
    pub created_at: Timestamp,
    pub source: BlueprintRevisionReference,
    pub source_blueprint_edit_number: BlueprintEditNumber,
    pub target: BlueprintRevisionReference,
    pub target_blueprint_edit_number: BlueprintEditNumber,
    pub proposed_json: CanonicalBlueprintCourse,
    pub target_comparison_json: CanonicalBlueprintCourse,
    /// Current target content OR metadata differs from the frozen comparison basis.
    pub target_is_stale: bool,
}

/// Explicit participant discovery scope; there is no installation-wide listing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BlueprintChangeProposalListScope {
    Mine,
    Target(question_model::BlueprintCourseId),
}

/// Exact metadata-event names and accepted pins; no actor identity is projected.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlueprintChangeProposalSummary {
    pub proposal_id: uuid::Uuid,
    pub created_at: Timestamp,
    pub source: BlueprintRevisionReference,
    pub source_blueprint_edit_number: BlueprintEditNumber,
    pub source_short_name: String,
    pub source_long_name: String,
    pub target: BlueprintRevisionReference,
    pub target_blueprint_edit_number: BlueprintEditNumber,
    pub target_short_name: String,
    pub target_long_name: String,
    pub target_is_stale: bool,
    pub accepted: Option<BlueprintChangeProposalAcceptedSummary>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlueprintChangeProposalAcceptedSummary {
    pub accepted_at: Timestamp,
    pub target: BlueprintRevisionReference,
    pub target_blueprint_edit_number: BlueprintEditNumber,
}

/// Participant-authorized ID-bearing trees and exact answer-free Pool memberships.
#[derive(Debug, Clone, PartialEq)]
pub struct BlueprintChangeProposalReview {
    pub proposal: StoredBlueprintChangeProposal,
    pub source: StoredBlueprintRevision,
    pub target: StoredBlueprintRevision,
    pub pool_memberships: BTreeMap<
        (
            question_model::QuestionId,
            question_model::QuestionPoolEditNumber,
        ),
        Vec<question_model::QuestionRevisionReference>,
    >,
    pub can_accept: bool,
    pub accepted: Option<AcceptedBlueprintChangeProposal>,
}

/// Participant-only access. Submission shares exact proposed evidence with the
/// receiving owner, without granting general Private source/history visibility.
#[async_trait]
pub trait BlueprintChangeProposalStore: Send + Sync {
    async fn list_blueprint_change_proposals(
        &self,
        session: SessionTokenHash,
        scope: BlueprintChangeProposalListScope,
        page: PageRequest,
    ) -> Result<Page<BlueprintChangeProposalSummary>, StoreError>;

    /// Loads original review and any accepted evidence under one authenticated transaction.
    async fn read_blueprint_change_proposal_review(
        &self,
        session: SessionTokenHash,
        proposal_id: uuid::Uuid,
    ) -> Result<Option<BlueprintChangeProposalReview>, StoreError>;

    /// Creates a source-copy Proposal without changing either Blueprint or daughters.
    async fn create_blueprint_change_proposal(
        &self,
        session: SessionTokenHash,
        input: CreateBlueprintChangeProposalInput,
    ) -> Result<StoredBlueprintChangeProposal, StoreError>;

    async fn read_blueprint_change_proposal(
        &self,
        session: SessionTokenHash,
        proposal_id: uuid::Uuid,
    ) -> Result<Option<StoredBlueprintChangeProposal>, StoreError>;

    /// Final acceptance: one successor, no direct daughter changes, no replay workflow.
    async fn accept_blueprint_change_proposal(
        &self,
        session: SessionTokenHash,
        input: AcceptBlueprintChangeProposalInput,
        bloom_receipts: crate::PoolBloomPreparationReceipts,
    ) -> Result<AcceptedBlueprintChangeProposal, StoreError>;

    async fn read_accepted_blueprint_change_proposal(
        &self,
        session: SessionTokenHash,
        proposal_id: uuid::Uuid,
    ) -> Result<Option<AcceptedBlueprintChangeProposal>, StoreError>;
}
