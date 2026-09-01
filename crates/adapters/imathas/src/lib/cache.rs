//! Immutable render-cache identity, validation, and compact encodings.

use std::fmt::Write as _;

use objects::ObjectAddress;
use question_model::generation::QuestionSeed;
use question_model::{
    ObjectId, QuestionBackendLocator, QuestionBackendVersion, QuestionGraderVersion,
    QuestionRevision, QuestionRevisionReference, QuestionVariationPresentation,
    SourceObjectChecksum, SourceObjectReference,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::{ExternalToolGradingContext, ImathasAdapterError, ResolvedImathasQuestionSource};

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct CachedRender {
    pub(super) schema: u8,
    pub(super) source: SourceObjectReference,
    pub(super) source_object_checksum: SourceObjectChecksum,
    pub(super) provider: String,
    pub(super) profile: String,
    pub(super) envelope: QuestionVariationPresentation,
}

pub(super) fn decode_cache(bytes: &[u8]) -> Result<CachedRender, ImathasAdapterError> {
    serde_json::from_slice(bytes).map_err(|_| ImathasAdapterError::InvalidCache)
}

pub(super) fn validate_cache(
    cached: &CachedRender,
    question: &QuestionRevision,
    seed: QuestionSeed,
    source: &ResolvedImathasQuestionSource,
) -> Result<(), ImathasAdapterError> {
    if cached.schema != 1
        || cached.source != *source.artifact()
        || cached.source_object_checksum != *source.source_object_checksum()
        || cached.provider != source.provider
        || cached.profile != source.profile
        || cached.envelope.variation.question_revision
            != (QuestionRevisionReference {
                question_id: question.question_id.clone(),
                revision_number: question.revision_number,
            })
        || cached.envelope.variation.seed != seed
        || question_model::validate_question_title(&cached.envelope.title).is_err()
        || !matches!(
            cached.envelope.response,
            question_model::QuestionResponseFormat::ExternalTool {}
        )
    {
        return Err(ImathasAdapterError::InvalidCache);
    }
    Ok(())
}

pub(super) fn verify_binding(
    question: &QuestionRevision,
    source: &ResolvedImathasQuestionSource,
) -> Result<(), ImathasAdapterError> {
    if *source.question_revision()
        != (QuestionRevisionReference {
            question_id: question.question_id.clone(),
            revision_number: question.revision_number,
        })
    {
        return Err(ImathasAdapterError::SourceDoesNotMatchQuestion);
    }
    match &question.backend_locator {
        QuestionBackendLocator::Imathas {
            provider,
            item_ref,
            integration_profile,
        } if provider.as_str() == source.provider.as_str()
            && item_ref.as_str() == source.item_ref.as_str()
            && integration_profile.as_str() == source.profile.as_str() =>
        {
            Ok(())
        }
        _ => Err(ImathasAdapterError::SourceDoesNotMatchQuestion),
    }
}

pub(super) fn render_key(
    question_revision: &QuestionRevisionReference,
    seed: QuestionSeed,
) -> ObjectAddress {
    ObjectAddress::QuestionRender {
        question_revision: question_revision.clone(),
        seed,
        object: deterministic_id(question_revision, seed),
    }
}

fn deterministic_id(question_revision: &QuestionRevisionReference, seed: QuestionSeed) -> ObjectId {
    let mut hash = Sha256::new();
    hash.update(b"peptidyle:imathas:render-cache:v1");
    hash.update(question_revision.question_id.to_string().as_bytes());
    hash.update(question_revision.revision_number.get().to_be_bytes());
    hash.update(seed.value().to_be_bytes());
    let digest = hash.finalize();
    let mut bytes = [0; 16];
    bytes.copy_from_slice(&digest[..16]);
    ObjectId::from_uuid(Uuid::from_bytes(bytes))
}

pub(super) fn parameter_hash(seed: QuestionSeed) -> String {
    let mut hash = Sha256::new();
    hash.update(b"peptidyle:imathas:parameters:v1");
    hash.update(seed.value().to_be_bytes());
    hex(hash.finalize().as_slice())
}

pub(super) fn backend_version(name: &str, version: &str) -> QuestionBackendVersion {
    QuestionBackendVersion {
        name: name.into(),
        version: version.into(),
    }
}

pub(super) fn grader_version(name: &str, version: &str) -> QuestionGraderVersion {
    QuestionGraderVersion {
        name: name.into(),
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

pub(super) fn external_tool_grading_context_payload(
    binding: &ExternalToolGradingContext,
) -> Vec<u8> {
    let mut value = Vec::with_capacity(16 + 8 + 4 + 8);
    value.extend_from_slice(binding.attempt.as_uuid().as_bytes());
    value.extend_from_slice(binding.question_revision.question_id.to_string().as_bytes());
    value.extend_from_slice(
        &binding
            .question_revision
            .revision_number
            .get()
            .to_be_bytes(),
    );
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
