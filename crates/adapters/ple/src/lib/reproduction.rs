use std::collections::{BTreeMap, BTreeSet};

use question_model::generation::QuestionSeed;
use question_model::{
    ObjectId, QuestionAssetId, QuestionAttemptReproductionDetails, QuestionRevision,
};
use question_model::{QuestionContentBlock, QuestionVariationPresentation};

use crate::{
    PleQuestionBackend, PleQuestionBackendError, PreparedPleQuestion, QuestionAssetObjectReference,
};

impl PleQuestionBackend {
    /// Reproduces an issued browser-safe Question Presentation and verifies its record.
    ///
    /// Question Asset Object References come from trusted server storage for the immutable published
    /// version. No answer key is returned.
    pub fn reproduce(
        &self,
        question: &QuestionRevision,
        seed: QuestionSeed,
        recorded_reproduction_details: &QuestionAttemptReproductionDetails,
        question_asset_object_references: &[QuestionAssetObjectReference],
    ) -> Result<QuestionVariationPresentation, PleQuestionBackendError> {
        self.require_backend_version(&recorded_reproduction_details.backend)?;
        let prepared = self.prepare_with_execution(question, seed)?;
        verify_record(
            &prepared,
            recorded_reproduction_details,
            &resolve_question_asset_objects(
                &prepared.presentation,
                question_asset_object_references,
            )?,
        )?;
        Ok(prepared.presentation)
    }
}

pub(super) fn verify_record(
    prepared: &PreparedPleQuestion,
    recorded: &QuestionAttemptReproductionDetails,
    expected_asset_objects: &[ObjectId],
) -> Result<(), PleQuestionBackendError> {
    verify_equal(recorded.renderer_version.is_none(), "rendererVersion")?;
    verify_equal(
        recorded.source_object_reference.is_some(),
        "sourceObjectReference",
    )?;
    verify_equal(
        recorded.source_object_checksum.is_some(),
        "sourceObjectChecksum",
    )?;
    verify_equal(
        recorded.asset_objects.as_slice() == expected_asset_objects,
        "assetObjects",
    )?;
    verify_equal(
        recorded.rendered_question_sha256 == prepared.rendered_question_sha256,
        "renderedQuestionSha256",
    )
}

fn verify_equal(matches: bool, field: &'static str) -> Result<(), PleQuestionBackendError> {
    if matches {
        Ok(())
    } else {
        Err(PleQuestionBackendError::ReproductionMismatch { field })
    }
}

pub(super) fn resolve_question_asset_objects(
    presentation: &QuestionVariationPresentation,
    question_asset_object_references: &[QuestionAssetObjectReference],
) -> Result<Vec<ObjectId>, PleQuestionBackendError> {
    let referenced_assets = presentation_asset_ids(presentation);
    let mut bindings = BTreeMap::new();
    for reference in question_asset_object_references {
        if bindings
            .insert(reference.question_asset, reference.object_reference)
            .is_some()
        {
            return Err(PleQuestionBackendError::ConflictingAssetBinding(
                reference.question_asset,
            ));
        }
    }
    for asset in &referenced_assets {
        if !bindings.contains_key(asset) {
            return Err(PleQuestionBackendError::MissingAssetBinding(*asset));
        }
    }
    for asset in bindings.keys() {
        if !referenced_assets.contains(asset) {
            return Err(PleQuestionBackendError::UnrelatedAssetBinding(*asset));
        }
    }
    Ok(referenced_assets
        .iter()
        .map(|asset| {
            *bindings
                .get(asset)
                .expect("all referenced assets were verified as bound")
        })
        .collect())
}

fn presentation_asset_ids(
    presentation: &QuestionVariationPresentation,
) -> BTreeSet<QuestionAssetId> {
    let mut assets = BTreeSet::new();
    collect_content_assets(&presentation.prompt, &mut assets);
    match &presentation.response {
        question_model::response::QuestionResponseFormat::MultipleChoice { choices, .. } => {
            for choice in choices {
                collect_content_assets(&choice.body, &mut assets);
            }
        }
        question_model::response::QuestionResponseFormat::Ordering { items } => {
            for item in items {
                collect_content_assets(&item.body, &mut assets);
            }
        }
        question_model::response::QuestionResponseFormat::Numeric { .. }
        | question_model::response::QuestionResponseFormat::ShortText { .. }
        | question_model::response::QuestionResponseFormat::MultiBlank { .. }
        | question_model::response::QuestionResponseFormat::Matching { .. }
        | question_model::response::QuestionResponseFormat::ImathasQuestionBackend {} => {}
        question_model::response::QuestionResponseFormat::Hotspot { surface, .. } => {
            assets.insert(surface.asset);
        }
    }
    assets
}

fn collect_content_assets(blocks: &[QuestionContentBlock], assets: &mut BTreeSet<QuestionAssetId>) {
    for block in blocks {
        if let QuestionContentBlock::Image { asset, .. } = block {
            assets.insert(asset.asset);
        }
    }
}
