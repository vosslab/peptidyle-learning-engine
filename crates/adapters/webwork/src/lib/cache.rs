//! Deterministic browser-safe render-cache identity and validation.

use objects::ObjectAddress;
use question_model::generation::QuestionSeed;
use question_model::{
    ObjectId, QuestionRendererVersion, QuestionRevisionReference, QuestionVariationPresentation,
    SourceObjectChecksum, SourceObjectReference,
};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use super::WebworkAdapterError;
use crate::ResolvedWebworkQuestionSource;

pub(super) const CACHE_SCHEMA_VERSION: u8 = 2;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct SafeRenderedWebworkQuestion {
    pub(super) presentation: QuestionVariationPresentation,
    pub(super) renderer_version: QuestionRendererVersion,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct CachedWebworkRender {
    pub(super) schema_version: u8,
    pub(super) source_object_reference: SourceObjectReference,
    pub(super) source_object_checksum: SourceObjectChecksum,
    pub(super) rendered: SafeRenderedWebworkQuestion,
}

pub(super) fn render_key(
    question_revision: &QuestionRevisionReference,
    seed: QuestionSeed,
) -> ObjectAddress {
    ObjectAddress::QuestionRender {
        question_revision: question_revision.clone(),
        seed,
        object: deterministic_render_object_id(question_revision, seed),
    }
}

fn deterministic_render_object_id(
    question_revision: &QuestionRevisionReference,
    seed: QuestionSeed,
) -> ObjectId {
    let mut hash = Sha256::new();
    hash.update(b"peptidyle:webwork-render-cache:v2");
    hash.update(question_revision.question_id.to_string().as_bytes());
    hash.update(question_revision.revision_number.get().to_be_bytes());
    hash.update(seed.value().to_be_bytes());
    let digest = hash.finalize();
    let mut bytes = [0_u8; 16];
    bytes.copy_from_slice(&digest[..16]);
    ObjectId::from_uuid(Uuid::from_bytes(bytes))
}

pub(super) fn decode_render(bytes: &[u8]) -> Result<CachedWebworkRender, WebworkAdapterError> {
    serde_json::from_slice(bytes)
        .map_err(|error| WebworkAdapterError::InvalidCache(error.to_string()))
}

pub(super) fn validate_cached(
    cached: &CachedWebworkRender,
    question_revision: &QuestionRevisionReference,
    seed: QuestionSeed,
    source: &ResolvedWebworkQuestionSource,
    title: &str,
    active_renderer_version: &QuestionRendererVersion,
) -> Result<(), WebworkAdapterError> {
    if cached.schema_version != CACHE_SCHEMA_VERSION
        || cached.source_object_reference != *source.source_object_reference()
        || cached.source_object_checksum != *source.source_object_checksum()
        || cached.rendered.renderer_version.name.is_empty()
        || cached.rendered.renderer_version.version.is_empty()
    {
        return Err(WebworkAdapterError::InvalidCache(
            "cached Question Source is incomplete or does not match the published Question Source"
                .to_string(),
        ));
    }
    if &cached.rendered.renderer_version != active_renderer_version {
        return Err(WebworkAdapterError::InvalidCache(
            "cache renderer identity does not match the configured renderer".to_string(),
        ));
    }
    validate_presentation(&cached.rendered.presentation, question_revision, seed)?;
    if cached.rendered.presentation.title != title {
        return Err(WebworkAdapterError::InvalidCache(
            "cache title does not match immutable published metadata".to_string(),
        ));
    }
    Ok(())
}

pub(super) fn validate_presentation(
    presentation: &QuestionVariationPresentation,
    question_revision: &QuestionRevisionReference,
    seed: QuestionSeed,
) -> Result<(), WebworkAdapterError> {
    if &presentation.variation.question_revision != question_revision {
        return Err(WebworkAdapterError::InvalidRendererQuestionPresentation(
            "renderer returned a different immutable version".to_string(),
        ));
    }
    if presentation.variation.question_seed != seed {
        return Err(WebworkAdapterError::InvalidRendererQuestionPresentation(
            "renderer returned a different deterministic seed".to_string(),
        ));
    }
    Ok(())
}

pub(super) fn rendered_hash(rendered: &CachedWebworkRender) -> Result<String, WebworkAdapterError> {
    let bytes = serde_json::to_vec(rendered).map_err(|error| {
        WebworkAdapterError::InvalidRendererQuestionPresentation(error.to_string())
    })?;
    Ok(hex_digest(Sha256::digest(bytes).as_slice()))
}

pub(super) fn hex_digest(bytes: &[u8]) -> String {
    use std::fmt::Write as _;

    let mut value = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        let _ = write!(&mut value, "{byte:02x}");
    }
    value
}
