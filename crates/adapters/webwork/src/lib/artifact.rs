//! Trusted immutable PG-source resolution and binding checks.

use objects::{ObjectKey, ObjectStore};
use question_model::{
    QuestionDefinition, QuestionSource, QuestionVersionReference, SourceArtifact,
};

use super::WebworkAdapterError;

/// Immutable PG source resolved from trusted object storage before adapter use.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WebworkSource {
    pub(super) question_version: QuestionVersionReference,
    pub(super) artifact: SourceArtifact,
    pub(super) pg_source: Vec<u8>,
}

impl WebworkSource {
    /// Resolves PG source only from its exact immutable published object key.
    pub async fn resolve<S: ObjectStore>(
        store: &S,
        question_version: QuestionVersionReference,
        artifact: SourceArtifact,
    ) -> Result<Self, WebworkAdapterError> {
        let expected_key = ObjectKey::QuestionSource {
            question_version: question_version.clone(),
            object: artifact.object,
        };
        let stored = store
            .get(&expected_key)
            .await
            .map_err(WebworkAdapterError::ObjectStore)?;
        if stored.record.key != expected_key
            || stored.record.id != artifact.object
            || stored.record.category != objects::ObjectCategory::Source
            || stored.record.question_version != Some(question_version.clone())
            || stored.record.sha256.to_string() != artifact.sha256
        {
            return Err(WebworkAdapterError::UntrustedSource);
        }
        Ok(Self {
            question_version,
            artifact,
            pg_source: stored.bytes,
        })
    }

    /// Immutable source artifact carried into attempt provenance.
    pub fn artifact(&self) -> &SourceArtifact {
        &self.artifact
    }
}

pub(super) fn webwork_identity(
    question: &QuestionDefinition,
) -> Result<(QuestionVersionReference, &str), WebworkAdapterError> {
    match &question.source {
        QuestionSource::Webwork { pg_path } => Ok((
            QuestionVersionReference {
                question_id: question.question_id.clone(),
                version_number: question.version_number,
            },
            pg_path,
        )),
        _ => Err(WebworkAdapterError::UnsupportedSource),
    }
}

pub(super) fn verify_source(source: &WebworkSource) -> Result<(), WebworkAdapterError> {
    let actual = objects::Sha256Digest::compute(&source.pg_source).to_string();
    if actual == source.artifact.sha256 {
        Ok(())
    } else {
        Err(WebworkAdapterError::SourceChecksumMismatch)
    }
}

pub(super) fn verify_source_binding(
    source: &WebworkSource,
    question_version: &QuestionVersionReference,
) -> Result<(), WebworkAdapterError> {
    if &source.question_version == question_version {
        Ok(())
    } else {
        Err(WebworkAdapterError::SourceDoesNotMatchQuestion)
    }
}
