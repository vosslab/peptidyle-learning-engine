use std::collections::{BTreeMap, BTreeSet};

use question_model::envelope::{QuestionContentBlock, QuestionVariationPresentation};
use question_model::generation::QuestionSeed;
use question_model::{
    ObjectId, QuestionAssetId, QuestionAttemptReproductionDetails, QuestionRevision,
};

use crate::{
    PleQuestionBackend, PleQuestionBackendError, PreparedPleQuestion, QuestionAssetObjectReference,
};

impl PleQuestionBackend {
    /// Reproduces an issued browser-safe envelope and verifies its record.
    ///
    /// Question Asset Object References come from trusted server storage for the immutable published
    /// version. No answer key is returned.
    pub fn reproduce(
        &self,
        question: &QuestionRevision,
        seed: QuestionSeed,
        recorded_parameter_hash: &str,
        recorded_reproduction_details: &QuestionAttemptReproductionDetails,
        question_asset_object_references: &[QuestionAssetObjectReference],
    ) -> Result<QuestionVariationPresentation, PleQuestionBackendError> {
        let execution = self.backend_execution_for(&recorded_reproduction_details.backend)?;
        let prepared = self.prepare_with_execution(question, seed, execution)?;
        verify_record(
            &prepared,
            recorded_parameter_hash,
            recorded_reproduction_details,
            &resolve_question_asset_objects(&prepared.envelope, question_asset_object_references)?,
        )?;
        Ok(prepared.envelope)
    }
}

pub(super) fn verify_record(
    prepared: &PreparedPleQuestion,
    recorded_parameter_hash: &str,
    recorded: &QuestionAttemptReproductionDetails,
    expected_asset_objects: &[ObjectId],
) -> Result<(), PleQuestionBackendError> {
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
    envelope: &QuestionVariationPresentation,
    question_asset_object_references: &[QuestionAssetObjectReference],
) -> Result<Vec<ObjectId>, PleQuestionBackendError> {
    let referenced_assets = envelope_asset_ids(envelope);
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

fn envelope_asset_ids(envelope: &QuestionVariationPresentation) -> BTreeSet<QuestionAssetId> {
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
