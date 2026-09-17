//! Participant-only frozen Change Proposal review and receiving-owner decisions.

use crate::blueprint_course::{
    BlueprintComparisonAssessment, BlueprintComparisonAssessmentRelationship,
    BlueprintComparisonModule, BlueprintComparisonNames,
};
use question_model::{BlueprintMetadataEtag, BlueprintRevisionReference, Timestamp};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Exact saved content and independently recorded metadata pins; never editor JSON.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BlueprintChangeProposalCreateRequest {
    pub source: BlueprintRevisionReference,
    pub source_metadata_etag: BlueprintMetadataEtag,
    pub target: BlueprintRevisionReference,
    pub target_metadata_etag: BlueprintMetadataEtag,
}

/// Whole-Assessment granularity, explicit layout, independent names and classification.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum BlueprintChangeProposalDecisionView {
    Entire,
    Selected {
        selection: question_model::blueprint_course::BlueprintForkApplySelection,
        source_short_name: bool,
        source_long_name: bool,
        source_classification: bool,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BlueprintChangeProposalAcceptanceRequest {
    pub expected_target: BlueprintRevisionReference,
    pub expected_target_metadata_etag: BlueprintMetadataEtag,
    pub decision: BlueprintChangeProposalDecisionView,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BlueprintChangeProposalAcceptedSummaryView {
    pub accepted_at: Timestamp,
    pub target: BlueprintRevisionReference,
    pub target_metadata_etag: BlueprintMetadataEtag,
}

/// Opaque UUID handle conveys no authorization, identity or global sequence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BlueprintChangeProposalSummaryView {
    pub proposal_id: String,
    pub created_at: Timestamp,
    pub source: BlueprintRevisionReference,
    pub source_metadata_etag: BlueprintMetadataEtag,
    pub source_names: BlueprintComparisonNames,
    pub target: BlueprintRevisionReference,
    pub target_metadata_etag: BlueprintMetadataEtag,
    pub target_names: BlueprintComparisonNames,
    pub target_is_stale: bool,
    pub accepted: Option<BlueprintChangeProposalAcceptedSummaryView>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BlueprintChangeProposalPageView {
    pub items: Vec<BlueprintChangeProposalSummaryView>,
    pub next_cursor: Option<String>,
}

/// Original ID-bearing inventory, labelled with its exact pinned basis.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BlueprintChangeProposalSideView {
    pub revision: BlueprintRevisionReference,
    pub metadata_etag: BlueprintMetadataEtag,
    pub names: BlueprintComparisonNames,
    pub classification: question_model::CourseClassification,
    pub modules: Vec<BlueprintComparisonModule>,
    pub assessments: Vec<BlueprintComparisonAssessment>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BlueprintChangeProposalComparisonView {
    pub source: BlueprintChangeProposalSideView,
    pub target: BlueprintChangeProposalSideView,
    /// Existing relationship DTO: left means frozen source, right means frozen target.
    pub assessment_relationships: Vec<BlueprintComparisonAssessmentRelationship>,
    pub shared_question_ids: Vec<question_model::QuestionId>,
    pub source_only_question_ids: Vec<question_model::QuestionId>,
    pub target_only_question_ids: Vec<question_model::QuestionId>,
}

/// Committed immutable outcome, never a later ordinary target-head reload.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BlueprintChangeProposalAcceptedView {
    pub accepted_at: Timestamp,
    pub target: BlueprintRevisionReference,
    pub target_metadata_etag: BlueprintMetadataEtag,
    pub decision: BlueprintChangeProposalDecisionView,
    pub applied_selection: question_model::blueprint_course::BlueprintForkApplySelection,
    pub new_modules: BTreeMap<
        question_model::BlueprintModuleReference,
        question_model::BlueprintModuleReference,
    >,
    pub new_assessments: BTreeMap<
        question_model::BlueprintAssessmentReference,
        question_model::BlueprintAssessmentReference,
    >,
    pub resulting_json: question_model::CanonicalBlueprintCourse,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BlueprintChangeProposalDetailView {
    pub proposal: BlueprintChangeProposalSummaryView,
    /// Presentation only. SQL independently authorizes and guards final acceptance.
    pub can_accept: bool,
    pub comparison: BlueprintChangeProposalComparisonView,
    pub accepted: Option<BlueprintChangeProposalAcceptedView>,
}
