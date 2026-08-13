//! Validation for native flat-question publication and private grading promotion.

use super::{validate_flat_import_publication_promotion, validate_source_artifact_for_publication};
use crate::{
    AssetDeliveryScope, PublishDraftCommand, StoreError, TenantContext, validate_asset_delivery,
};
use question_model::{DraftQuestionSource, QuestionSource};

/// Validates a flat-question-specific promotion path before publication.
pub(crate) fn validate_flat_question_publication(
    context: TenantContext,
    command: &PublishDraftCommand,
    staged: &crate::WorkspaceFlatQuestionSource,
) -> Result<(), StoreError> {
    validate_source_artifact_for_publication(
        command.publication,
        &command.published_source,
        command.source_artifact.as_ref(),
        command.flat_question_promotion.is_some(),
    )?;
    let Some(promotion) = command.flat_question_promotion.as_ref() else {
        return Err(StoreError::InvalidRecord(
            "flat-question publication requires a flat-question promotion".to_string(),
        ));
    };
    validate_flat_import_publication_promotion(
        context,
        command.publication,
        promotion.import_origin.as_ref(),
    )?;
    grading::flat_question::validate_for_draft(&command.expected_draft.question).map_err(
        |error| StoreError::InvalidRecord(format!("flat-question draft is invalid: {error}")),
    )?;
    match (
        &command.expected_draft.question.source,
        &command.published_source,
    ) {
        (
            DraftQuestionSource::Native {
                family: draft_family,
            },
            QuestionSource::Native {
                family: published_family,
            },
        ) if draft_family == published_family
            && grading::flat_question::is_flat_question_family(draft_family) => {}
        _ => {
            return Err(StoreError::InvalidRecord(
                "flat-question promotion requires matching supported native flat sources"
                    .to_string(),
            ));
        }
    }
    if staged.tenant != context.tenant_id()
        || staged.workspace != command.expected_draft.question.workspace
    {
        return Err(StoreError::InvalidRecord(
            "flat-question promotion is not staged for this tenant or workspace".to_string(),
        ));
    }
    if staged.workspace_revision != command.expected_revision {
        return Err(StoreError::Conflict);
    }
    if promotion.source != *staged {
        return Err(StoreError::Conflict);
    }
    match &command.published_source {
        QuestionSource::Native { family } => {
            if family != &staged.source_family {
                return Err(StoreError::InvalidRecord(
                    "flat-question source family must match the staged draft family".to_string(),
                ));
            }
        }
        _ => {
            return Err(StoreError::InvalidRecord(
                "flat-question promotion requires native source data".to_string(),
            ));
        }
    };
    let artifact = command.source_artifact.as_ref().ok_or_else(|| {
        StoreError::InvalidRecord("native promotion requires source artifact".to_string())
    })?;
    let source_artifact_matches_staging = staged.source_record.sha256 == artifact.object.sha256
        && staged.source_record.size_bytes == artifact.object.size_bytes
        && staged.source_record.media_type == artifact.object.media_type;
    let hotspot_retargeted = matches!(
        (&command.expected_draft.question.response, &promotion.published_question.response),
        (
            question_model::ResponseDefinition::Hotspot { surface: staged_surface, .. },
            question_model::ResponseDefinition::Hotspot { surface: published_surface, .. },
        ) if staged_surface.asset != published_surface.asset
            && staged_surface.checksum == published_surface.checksum
    );
    if !source_artifact_matches_staging && !hotspot_retargeted {
        return Err(StoreError::Conflict);
    }
    let expected_hotspot = match &command.expected_draft.question.response {
        question_model::ResponseDefinition::Hotspot { surface, .. } => Some(surface),
        _ => None,
    };
    match expected_hotspot {
        None if promotion.assets.is_empty()
            && promotion.published_question == command.expected_draft.question => {}
        None => return Err(StoreError::Conflict),
        Some(surface) if promotion.assets.len() == 1 => {
            let delivery = &promotion.assets[0];
            validate_asset_delivery(delivery)?;
            crate::validate_catalog_asset_delivery_scope(delivery, command.scope)?;
            let AssetDeliveryScope::Catalog { asset, reference } = delivery.scope else {
                return Err(StoreError::InvalidRecord(
                    "flat-question hotspot asset must be a catalog asset".to_string(),
                ));
            };
            let mut expected_published_question = command.expected_draft.question.clone();
            let question_model::ResponseDefinition::Hotspot {
                surface: published_surface,
                ..
            } = &mut expected_published_question.response
            else {
                return Err(StoreError::Conflict);
            };
            let staged_asset = published_surface.asset;
            published_surface.asset = asset;
            for block in &mut expected_published_question.prompt {
                if let question_model::envelope::ContentBlock::Image {
                    asset: prompt_asset,
                    ..
                } = block
                    && prompt_asset.asset == staged_asset
                {
                    prompt_asset.asset = asset;
                }
            }
            if asset == surface.asset
                || reference != command.publication
                || delivery.object.sha256.to_string() != surface.checksum
                || promotion.published_question != expected_published_question
            {
                return Err(StoreError::Conflict);
            }
        }
        Some(_) => return Err(StoreError::Conflict),
    }
    Ok(())
}

/// Validates the Memory backend's locked current grading payload without
/// widening PostgreSQL application-role access to private grader bytes.
/// PostgreSQL performs the equivalent stored-only binding and copy inside its
/// grader-owned promotion capability.
pub(crate) fn validate_flat_question_publication_grading(
    command: &PublishDraftCommand,
    staged: &crate::WorkspaceFlatQuestionSource,
    stored_grading: &crate::FlatQuestionGradingPayload,
) -> Result<crate::FlatQuestionGradingPayload, StoreError> {
    let private = stored_grading.decode_private()?;
    private
        .validate_for_draft(&command.expected_draft.question)
        .map_err(|error| {
            StoreError::InvalidRecord(format!(
                "flat-question grading material does not match the staged draft: {error}"
            ))
        })?;
    let published_question = &command
        .flat_question_promotion
        .as_ref()
        .expect("flat-question promotion was validated before grading")
        .published_question;
    let rebound = match stored_grading.rebind_to_draft(published_question) {
        Ok(value) => value,
        Err(error) => {
            return Err(StoreError::InvalidRecord(error.to_string()));
        }
    };
    rebound
        .decode_private()?
        .validate_for_draft(published_question)
        .map_err(|error| {
            StoreError::InvalidRecord(format!(
                "flat-question grading material does not match the published public definition: {error}"
            ))
        })?;
    if stored_grading.public_binding_sha256() != staged.public_binding_sha256
        || private.public_binding_sha256() != staged.public_binding_sha256
    {
        return Err(StoreError::Conflict);
    }
    Ok(rebound)
}
