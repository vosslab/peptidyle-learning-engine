//! Trusted immutable PG-source resolution and binding checks.

use objects::{ObjectStore, QuestionSourceResolutionError, ResolvedQuestionSource};
use question_model::{QuestionRevisionReference, SourceObjectChecksum, SourceObjectReference};

use super::WebworkAdapterError;

/// Immutable PG source resolved from trusted object storage before adapter use.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedWebworkQuestionSource {
    resolved: ResolvedQuestionSource,
    binding: WebworkQuestionSourceBinding,
}

/// Registered WeBWorK routing facts for one immutable Question Source.
///
/// Publication owns this binding.  The adapter accepts it only together with
/// the exact immutable source object it names, never a generic Question
/// Revision record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WebworkQuestionSourceBinding {
    question_revision: QuestionRevisionReference,
    pg_path: String,
}

impl WebworkQuestionSourceBinding {
    /// Creates the registered PG-path binding for one published source.
    pub fn new(
        question_revision: QuestionRevisionReference,
        pg_path: String,
    ) -> Result<Self, WebworkAdapterError> {
        if pg_path.is_empty()
            || pg_path.len() > 1_024
            || pg_path.starts_with('/')
            || pg_path.contains(['\\', '\0'])
            || pg_path
                .split('/')
                .any(|part| part.is_empty() || matches!(part, "." | ".."))
        {
            return Err(WebworkAdapterError::InvalidPgPath);
        }
        Ok(Self {
            question_revision,
            pg_path,
        })
    }

    /// Exact immutable revision that registered this source.
    pub fn question_revision(&self) -> &QuestionRevisionReference {
        &self.question_revision
    }

    /// Registered OPL-style PG location used only by the private renderer.
    pub fn pg_path(&self) -> &str {
        &self.pg_path
    }
}

impl ResolvedWebworkQuestionSource {
    /// Resolves PG source only from its exact immutable published Object Address.
    pub async fn resolve<S: ObjectStore>(
        store: &S,
        binding: WebworkQuestionSourceBinding,
        source_object_reference: SourceObjectReference,
        source_object_checksum: SourceObjectChecksum,
    ) -> Result<Self, WebworkAdapterError> {
        let resolved = ResolvedQuestionSource::resolve(
            store,
            binding.question_revision().clone(),
            source_object_reference,
            source_object_checksum,
        )
        .await
        .map_err(|error| match error {
            QuestionSourceResolutionError::ObjectStore(error) => {
                WebworkAdapterError::ObjectStore(error)
            }
            QuestionSourceResolutionError::UntrustedObjectRecord => {
                WebworkAdapterError::UntrustedSource
            }
        })?;
        Ok(Self { resolved, binding })
    }

    /// Immutable Source Object Reference carried into Question Attempt Reproduction Details.
    pub fn source_object_reference(&self) -> &SourceObjectReference {
        self.resolved.source_object_reference()
    }

    /// SHA-256 evidence for the immutable source object bytes.
    pub fn source_object_checksum(&self) -> &SourceObjectChecksum {
        self.resolved.source_object_checksum()
    }

    /// Exact immutable revision verified through the Question Source Object Address.
    pub fn question_revision(&self) -> &QuestionRevisionReference {
        self.binding.question_revision()
    }

    /// Registered private PG path for this exact immutable source.
    pub fn pg_path(&self) -> &str {
        self.binding.pg_path()
    }

    /// Verified immutable PG source bytes for the renderer.
    pub(super) fn pg_source(&self) -> &[u8] {
        self.resolved.bytes()
    }
}

pub(super) fn verify_source(
    source: &ResolvedWebworkQuestionSource,
) -> Result<(), WebworkAdapterError> {
    let actual = objects::Sha256Checksum::compute(source.pg_source()).to_string();
    if actual == source.source_object_checksum().as_str() {
        Ok(())
    } else {
        Err(WebworkAdapterError::SourceChecksumMismatch)
    }
}
