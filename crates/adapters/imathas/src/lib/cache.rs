//! Immutable render-cache identity, validation, and compact encodings.

use std::fmt::Write as _;

use objects::ObjectKey;
use question_model::generation::Seed;
use question_model::{
    ImplementationVersion, ObjectId, ProblemId, QuestionDefinition, QuestionEnvelope,
    QuestionSource, SourceArtifact, VersionId,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::{GradeBinding, ImathasAdapterError, ImathasSource};

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct CachedRender {
    pub(super) schema: u8,
    pub(super) source: SourceArtifact,
    pub(super) provider: String,
    pub(super) profile: String,
    pub(super) envelope: QuestionEnvelope,
}

pub(super) fn decode_cache(bytes: &[u8]) -> Result<CachedRender, ImathasAdapterError> {
    serde_json::from_slice(bytes).map_err(|_| ImathasAdapterError::InvalidCache)
}

pub(super) fn validate_cache(
    cached: &CachedRender,
    question: &QuestionDefinition,
    seed: Seed,
    source: &ImathasSource,
) -> Result<(), ImathasAdapterError> {
    if cached.schema != 1
        || cached.source != source.artifact
        || cached.provider != source.provider
        || cached.profile != source.profile
        || cached.envelope.version != question.version
        || cached.envelope.seed != seed
        || question_model::validate_question_title(&cached.envelope.title).is_err()
        || !matches!(
            cached.envelope.response,
            question_model::ResponseDefinition::ExternalTool {}
        )
    {
        return Err(ImathasAdapterError::InvalidCache);
    }
    Ok(())
}

pub(super) fn verify_binding(
    question: &QuestionDefinition,
    source: &ImathasSource,
) -> Result<(), ImathasAdapterError> {
    if question.problem != source.problem || question.version != source.version {
        return Err(ImathasAdapterError::SourceDoesNotMatchQuestion);
    }
    match &question.source {
        QuestionSource::Imathas {
            provider,
            item_ref,
            snapshot,
            snapshot_sha256,
            integration_profile,
        } if provider == &source.provider
            && item_ref == &source.item_ref
            && snapshot == &source.artifact.object
            && snapshot_sha256 == &source.artifact.sha256
            && integration_profile == &source.profile =>
        {
            Ok(())
        }
        _ => Err(ImathasAdapterError::SourceDoesNotMatchQuestion),
    }
}

pub(super) fn render_key(problem: ProblemId, version: VersionId, seed: Seed) -> ObjectKey {
    ObjectKey::ProblemRender {
        problem,
        version,
        seed,
        object: deterministic_id(version, seed),
    }
}

fn deterministic_id(version: VersionId, seed: Seed) -> ObjectId {
    let mut hash = Sha256::new();
    hash.update(b"peptidyle:imathas:render-cache:v1");
    hash.update(version.as_uuid().as_bytes());
    hash.update(seed.value().to_be_bytes());
    let digest = hash.finalize();
    let mut bytes = [0; 16];
    bytes.copy_from_slice(&digest[..16]);
    ObjectId::from_uuid(Uuid::from_bytes(bytes))
}

pub(super) fn parameter_hash(seed: Seed) -> String {
    let mut hash = Sha256::new();
    hash.update(b"peptidyle:imathas:parameters:v1");
    hash.update(seed.value().to_be_bytes());
    hex(hash.finalize().as_slice())
}

pub(super) fn implementation(id: &str, version: &str) -> ImplementationVersion {
    ImplementationVersion {
        id: id.into(),
        version: version.into(),
    }
}

pub(super) fn valid_opaque_key(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
}

/// Supported iMathAS item identifiers are deliberately identifier-shaped,
/// rather than URLs or arbitrary provider path fragments. Numeric item IDs and
/// provider opaque IDs share this bounded grammar.
pub(super) fn valid_item_ref(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
        && !value.contains("..")
}

pub(super) fn binding_payload(binding: &GradeBinding) -> Vec<u8> {
    let mut value = Vec::with_capacity(16 * 3 + 8);
    value.extend_from_slice(binding.attempt.as_uuid().as_bytes());
    value.extend_from_slice(binding.problem.as_uuid().as_bytes());
    value.extend_from_slice(binding.version.as_uuid().as_bytes());
    value.extend_from_slice(&binding.seed.value().to_be_bytes());
    value
}

pub(super) fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    left.iter()
        .zip(right)
        .fold(0_u8, |difference, (a, b)| difference | (a ^ b))
        == 0
}

pub(super) fn hex(bytes: &[u8]) -> String {
    let mut value = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        let _ = write!(&mut value, "{byte:02x}");
    }
    value
}
