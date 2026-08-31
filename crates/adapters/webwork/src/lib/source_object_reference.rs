//! Trusted immutable PG-source resolution and binding checks.

use objects::{ObjectAddress, ObjectStore};
use question_model::{
    QuestionSource, QuestionVersion, QuestionVersionReference, SourceObjectReference,
};

use super::WebworkAdapterError;

/// Immutable PG source resolved from trusted object storage before adapter use.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WebworkSource {
    pub(super) question_version: QuestionVersionReference,
    pub(super) source_object_reference: SourceObjectReference,
    pub(super) pg_source: Vec<u8>,
}

impl WebworkSource {
    /// Resolves PG source only from its exact immutable published object key.
    pub async fn resolve<S: ObjectStore>(
        store: &S,
        question_version: QuestionVersionReference,
        source_object_reference: SourceObjectReference,
    ) -> Result<Self, WebworkAdapterError> {
        let expected_key = ObjectAddress::QuestionSource {
            question_version: question_version.clone(),
            object: source_object_reference.object,
        };
        let stored = store
            .get(&expected_key)
            .await
            .map_err(WebworkAdapterError::ObjectStore)?;
        if stored.record.address != expected_key
            || stored.record.id != source_object_reference.object
            || stored.record.question_version != Some(question_version.clone())
            || stored.record.sha256.to_string() != source_object_reference.sha256
        {
            return Err(WebworkAdapterError::UntrustedSource);
        }
        Ok(Self {
            question_version,
            source_object_reference,
            pg_source: stored.bytes,
        })
    }

    /// Immutable Source Object Reference carried into Question Attempt Reproduction Details.
    pub fn source_object_reference(&self) -> &SourceObjectReference {
        &self.source_object_reference
    }
}

pub(super) fn webwork_identity(
    question: &QuestionVersion,
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
    if actual == source.source_object_reference.sha256 {
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
