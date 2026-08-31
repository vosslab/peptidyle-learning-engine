//! Deterministic browser-safe render-cache identity and validation.

use objects::ObjectKey;
use question_model::generation::Seed;
use question_model::{ObjectId, QuestionEnvelope, QuestionVersionReference, SourceArtifact};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use super::WebworkAdapterError;
use crate::WebworkSource;

pub(super) const CACHE_SCHEMA_VERSION: u8 = 1;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct SafeRenderedWebworkQuestion {
    pub(super) envelope: QuestionEnvelope,
    pub(super) sanitized_html: String,
    pub(super) renderer: crate::renderer_contract::RendererIdentity,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct CachedWebworkRender {
    pub(super) schema_version: u8,
    pub(super) source_artifact: SourceArtifact,
    pub(super) rendered: SafeRenderedWebworkQuestion,
}

pub(super) fn render_key(question_version: &QuestionVersionReference, seed: Seed) -> ObjectKey {
    ObjectKey::QuestionRender {
        question_version: question_version.clone(),
        seed,
        object: deterministic_render_object_id(question_version, seed),
    }
}

fn deterministic_render_object_id(
    question_version: &QuestionVersionReference,
    seed: Seed,
) -> ObjectId {
    let mut hash = Sha256::new();
    hash.update(b"peptidyle:webwork-render-cache:v1");
    hash.update(question_version.question_id.to_string().as_bytes());
    hash.update(question_version.version_number.get().to_be_bytes());
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
    question_version: &QuestionVersionReference,
    seed: Seed,
    source: &WebworkSource,
    title: &str,
    active_renderer: &crate::renderer_contract::RendererIdentity,
) -> Result<(), WebworkAdapterError> {
    if cached.schema_version != CACHE_SCHEMA_VERSION
        || cached.source_artifact != source.artifact
        || cached.rendered.renderer.id.is_empty()
        || cached.rendered.renderer.version.is_empty()
    {
        return Err(WebworkAdapterError::InvalidCache(
            "cache provenance is incomplete or does not match the published source".to_string(),
        ));
    }
    if &cached.rendered.renderer != active_renderer {
        return Err(WebworkAdapterError::InvalidCache(
            "cache renderer identity does not match the configured renderer".to_string(),
        ));
    }
    validate_envelope(&cached.rendered.envelope, question_version, seed)?;
    if cached.rendered.envelope.title != title {
        return Err(WebworkAdapterError::InvalidCache(
            "cache title does not match immutable published metadata".to_string(),
        ));
    }
    Ok(())
}

pub(super) fn validate_envelope(
    envelope: &QuestionEnvelope,
    question_version: &QuestionVersionReference,
    seed: Seed,
) -> Result<(), WebworkAdapterError> {
    if &envelope.question_version != question_version {
        return Err(WebworkAdapterError::InvalidRendererEnvelope(
            "renderer returned a different immutable version".to_string(),
        ));
    }
    if envelope.seed != seed {
        return Err(WebworkAdapterError::InvalidRendererEnvelope(
            "renderer returned a different deterministic seed".to_string(),
        ));
    }
    Ok(())
}

pub(super) fn rendered_hash(rendered: &CachedWebworkRender) -> Result<String, WebworkAdapterError> {
    let bytes = serde_json::to_vec(rendered)
        .map_err(|error| WebworkAdapterError::InvalidRendererEnvelope(error.to_string()))?;
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
