//! Immutable render-cache identity, validation, and compact encodings.

use std::fmt::Write as _;

use objects::ObjectAddress;
use question_model::generation::QuestionSeed;
use question_model::{
    ImathasQuestionBackendBinding, ObjectId, QuestionBackend, QuestionBackendVersion,
    QuestionGraderVersion, QuestionRevision, QuestionRevisionReference,
    QuestionVariationPresentation, SourceObjectChecksum, SourceObjectReference,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::{ImathasAdapterError, ResolvedImathasQuestionSource};

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct CachedRender {
    pub(super) schema: u8,
    pub(super) source: SourceObjectReference,
    pub(super) source_object_checksum: SourceObjectChecksum,
    pub(super) binding: ImathasQuestionBackendBinding,
    pub(super) presentation: QuestionVariationPresentation,
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
        || cached.source != *source.source_object_reference()
        || cached.source_object_checksum != *source.source_object_checksum()
        || cached.binding != source.binding
        || cached.presentation.variation.question_revision
            != (QuestionRevisionReference {
                question_id: question.question_id.clone(),
                revision_number: question.revision_number,
            })
        || cached.presentation.variation.question_seed != seed
        || question_model::validate_question_title(&cached.presentation.title).is_err()
        || !matches!(
            cached.presentation.response,
            question_model::QuestionResponseFormat::ImathasQuestionBackend {}
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
    if question.question_backend == QuestionBackend::Imathas
        && question.imathas_question_backend_binding.as_ref() == Some(&source.binding)
    {
        Ok(())
    } else {
        Err(ImathasAdapterError::SourceDoesNotMatchQuestion)
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
