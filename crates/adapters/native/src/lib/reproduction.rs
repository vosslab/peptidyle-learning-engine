use std::collections::{BTreeMap, BTreeSet};

use question_model::envelope::{ContentBlock, QuestionPresentation};
use question_model::generation::QuestionSeed;
use question_model::{AssetId, ObjectId, QuestionAttemptReproductionDetails, QuestionVersion};

use crate::{AssetObjectBinding, NativeAdapter, NativeAdapterError, PreparedNativeQuestion};

impl NativeAdapter {
    /// Reproduces an issued browser-safe envelope and verifies its record.
    ///
    /// Bindings come from trusted server storage for the immutable published
    /// version. No answer key is returned.
    pub fn reproduce(
        &self,
        question: &QuestionVersion,
        seed: QuestionSeed,
        recorded_parameter_hash: &str,
        recorded_reproduction_details: &QuestionAttemptReproductionDetails,
        asset_bindings: &[AssetObjectBinding],
    ) -> Result<QuestionPresentation, NativeAdapterError> {
        let execution = self.backend_execution_for(&recorded_reproduction_details.backend)?;
        let prepared = self.prepare_with_execution(question, seed, execution)?;
        verify_record(
            &prepared,
            recorded_parameter_hash,
            recorded_reproduction_details,
            &resolve_asset_objects(&prepared.envelope, asset_bindings)?,
        )?;
        Ok(prepared.envelope)
    }
}

pub(super) fn verify_record(
    prepared: &PreparedNativeQuestion,
    recorded_parameter_hash: &str,
    recorded: &QuestionAttemptReproductionDetails,
    expected_asset_objects: &[ObjectId],
) -> Result<(), NativeAdapterError> {
    verify_equal(
        prepared.parameter_hash == recorded_parameter_hash,
        "parameterHash",
    )?;
    verify_equal(recorded.renderer_version.is_none(), "rendererVersion")?;
    verify_equal(
        recorded.generator == prepared.generated.generator,
        "generator",
    )?;
    verify_equal(
        recorded.source_object_reference.is_none(),
        "sourceObjectReference",
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

fn verify_equal(matches: bool, field: &'static str) -> Result<(), NativeAdapterError> {
    if matches {
        Ok(())
    } else {
        Err(NativeAdapterError::ReproductionMismatch { field })
    }
}

pub(super) fn resolve_asset_objects(
    envelope: &QuestionPresentation,
    asset_bindings: &[AssetObjectBinding],
) -> Result<Vec<ObjectId>, NativeAdapterError> {
    let referenced_assets = envelope_asset_ids(envelope);
    let mut bindings = BTreeMap::new();
    for binding in asset_bindings {
        if bindings.insert(binding.asset, binding.object).is_some() {
            return Err(NativeAdapterError::ConflictingAssetBinding(binding.asset));
        }
    }
    for asset in &referenced_assets {
        if !bindings.contains_key(asset) {
            return Err(NativeAdapterError::MissingAssetBinding(*asset));
        }
    }
    for asset in bindings.keys() {
        if !referenced_assets.contains(asset) {
            return Err(NativeAdapterError::UnrelatedAssetBinding(*asset));
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

fn envelope_asset_ids(envelope: &QuestionPresentation) -> BTreeSet<AssetId> {
    let mut assets = BTreeSet::new();
    collect_content_assets(&envelope.prompt, &mut assets);
    match &envelope.response {
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
        | question_model::response::QuestionResponseFormat::ExternalTool {} => {}
        question_model::response::QuestionResponseFormat::Hotspot { surface, .. } => {
            assets.insert(surface.asset);
        }
    }
    assets
}

fn collect_content_assets(blocks: &[ContentBlock], assets: &mut BTreeSet<AssetId>) {
    for block in blocks {
        if let ContentBlock::Image { asset, .. } = block {
            assets.insert(asset.asset);
        }
    }
}
