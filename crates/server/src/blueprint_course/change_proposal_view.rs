//! Pure projection of authorized frozen Proposal evidence, with no ordinary head reads.

use browser_api_contract::blueprint_change_proposal::{
    BlueprintChangeProposalAcceptedSummaryView, BlueprintChangeProposalAcceptedView,
    BlueprintChangeProposalComparisonView, BlueprintChangeProposalDecisionView,
    BlueprintChangeProposalDetailView, BlueprintChangeProposalSideView,
    BlueprintChangeProposalSummaryView,
};
use browser_api_contract::blueprint_course::{
    BlueprintComparisonAssessmentRelationship, BlueprintComparisonNames,
};
use learning_data_access::{
    AcceptedBlueprintChangeProposal, BlueprintChangeProposalDecision,
    BlueprintChangeProposalReview, BlueprintChangeProposalSummary, StoreError,
};
use question_model::blueprint_course::{BlueprintComparisonInventory, compare_blueprint_courses};

pub(super) fn summary(
    record: BlueprintChangeProposalSummary,
) -> BlueprintChangeProposalSummaryView {
    BlueprintChangeProposalSummaryView {
        proposal_id: record.proposal_id.to_string(),
        created_at: record.created_at,
        source: record.source,
        source_metadata_etag: record.source_metadata_etag,
        source_names: BlueprintComparisonNames {
            short_name: record.source_short_name,
            long_name: record.source_long_name,
        },
        target: record.target,
        target_metadata_etag: record.target_metadata_etag,
        target_names: BlueprintComparisonNames {
            short_name: record.target_short_name,
            long_name: record.target_long_name,
        },
        target_is_stale: record.target_is_stale,
        accepted: record
            .accepted
            .map(|row| BlueprintChangeProposalAcceptedSummaryView {
                accepted_at: row.accepted_at,
                target: row.target,
                target_metadata_etag: row.target_metadata_etag,
            }),
    }
}

pub(super) fn detail(
    review: BlueprintChangeProposalReview,
) -> Result<BlueprintChangeProposalDetailView, StoreError> {
    let comparison = compare_blueprint_courses(
        &review.source.content.to_domain()?,
        &review.target.content.to_domain()?,
        &review.pool_memberships,
    )
    .map_err(|_| StoreError::InvalidRecord("Blueprint Proposal comparison is invalid".into()))?;
    let proposal = review.proposal;
    let source_metadata = proposal.proposed_json.metadata();
    let target_metadata = proposal.target_comparison_json.metadata();
    let source_names = BlueprintComparisonNames {
        short_name: source_metadata.short_name().to_owned(),
        long_name: source_metadata.long_name().to_owned(),
    };
    let target_names = BlueprintComparisonNames {
        short_name: target_metadata.short_name().to_owned(),
        long_name: target_metadata.long_name().to_owned(),
    };
    Ok(BlueprintChangeProposalDetailView {
        proposal: BlueprintChangeProposalSummaryView {
            proposal_id: proposal.proposal_id.to_string(),
            created_at: proposal.created_at,
            source: proposal.source,
            source_metadata_etag: proposal.source_metadata_etag,
            source_names: source_names.clone(),
            target: proposal.target,
            target_metadata_etag: proposal.target_metadata_etag,
            target_names: target_names.clone(),
            target_is_stale: proposal.target_is_stale,
            accepted: review.accepted.as_ref().map(|row| {
                BlueprintChangeProposalAcceptedSummaryView {
                    accepted_at: row.accepted_at,
                    target: row.target,
                    target_metadata_etag: row.target_metadata_etag,
                }
            }),
        },
        can_accept: review.can_accept,
        comparison: BlueprintChangeProposalComparisonView {
            source: side(
                comparison.left,
                proposal.source,
                proposal.source_metadata_etag,
                source_names,
                source_metadata.classification().clone(),
            ),
            target: side(
                comparison.right,
                proposal.target,
                proposal.target_metadata_etag,
                target_names,
                target_metadata.classification().clone(),
            ),
            assessment_relationships: comparison
                .relationships
                .into_iter()
                .map(|row| BlueprintComparisonAssessmentRelationship {
                    left_assessment_reference: row.left_assessment_reference,
                    right_assessment_reference: row.right_assessment_reference,
                    shared_question_ids: row.shared_question_ids,
                })
                .collect(),
            shared_question_ids: comparison.shared_question_ids,
            source_only_question_ids: comparison.left_only_question_ids,
            target_only_question_ids: comparison.right_only_question_ids,
        },
        accepted: review.accepted.map(accepted),
    })
}

fn side(
    inventory: BlueprintComparisonInventory,
    revision: question_model::BlueprintRevisionReference,
    metadata_etag: question_model::BlueprintMetadataEtag,
    names: BlueprintComparisonNames,
    classification: question_model::CourseClassification,
) -> BlueprintChangeProposalSideView {
    let projected = super::fork_review::comparison_side(
        inventory,
        revision,
        names.short_name,
        names.long_name,
        metadata_etag,
    );
    BlueprintChangeProposalSideView {
        revision,
        metadata_etag,
        names: projected.names,
        classification,
        modules: projected.modules,
        assessments: projected.assessments,
    }
}

pub(super) fn accepted(
    record: AcceptedBlueprintChangeProposal,
) -> BlueprintChangeProposalAcceptedView {
    BlueprintChangeProposalAcceptedView {
        accepted_at: record.accepted_at,
        target: record.target,
        target_metadata_etag: record.target_metadata_etag,
        decision: decision_view(record.decision.decision),
        applied_selection: record.decision.applied_selection,
        new_modules: record.decision.new_modules,
        new_assessments: record.decision.new_assessments,
        resulting_json: record.resulting_json,
    }
}

fn decision_view(decision: BlueprintChangeProposalDecision) -> BlueprintChangeProposalDecisionView {
    match decision {
        BlueprintChangeProposalDecision::Entire => BlueprintChangeProposalDecisionView::Entire,
        BlueprintChangeProposalDecision::Selected {
            selection,
            source_short_name,
            source_long_name,
            source_classification,
        } => BlueprintChangeProposalDecisionView::Selected {
            selection,
            source_short_name,
            source_long_name,
            source_classification,
        },
    }
}

pub(super) fn store_decision(
    decision: BlueprintChangeProposalDecisionView,
) -> BlueprintChangeProposalDecision {
    match decision {
        BlueprintChangeProposalDecisionView::Entire => BlueprintChangeProposalDecision::Entire,
        BlueprintChangeProposalDecisionView::Selected {
            selection,
            source_short_name,
            source_long_name,
            source_classification,
        } => BlueprintChangeProposalDecision::Selected {
            selection,
            source_short_name,
            source_long_name,
            source_classification,
        },
    }
}
