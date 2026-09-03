//! First-party PLE Question JSON backend.
//!
//! PLE executes the exact immutable JSON source registered for one Question
//! Revision Reference. It never reconstructs a generic Question Revision or
//! Draft Question Content from that source.

use question_model::{
    ObjectId, QuestionAttemptReproductionDetails, QuestionBackendVersion, QuestionGraderVersion,
    QuestionVariationPresentation,
};

#[path = "lib/question_json_source.rs"]
mod question_json_source;

pub use question_json_source::ResolvedPleQuestionJsonSource;

/// Strict, versioned PLE Question JSON source for first-party static Questions.
pub mod question_json;

#[cfg(test)]
#[path = "lib/question_json_source_tests.rs"]
mod question_json_source_tests;
#[cfg(test)]
mod test_support;

/// Stable Question Backend identifier persisted with every PLE Question Attempt.
pub const ADAPTER_ID: &str = "ple-question-backend";
/// Current Question Backend Version for issuance and replay.
pub const ADAPTER_VERSION: &str = "1";
/// Stable Question Grader identifier persisted with every PLE Question Attempt.
pub const GRADING_ID: &str = "generic-grader";
/// Current Question Grader Version for issuance and replay.
pub const GRADING_VERSION: &str = "1";

/// Trusted server-side relationship between one PLE asset and immutable object bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QuestionAssetObjectReference {
    pub question_asset: question_model::QuestionAssetId,
    pub object_reference: ObjectId,
}

/// Key-free PLE Issued Question returned at Question Attempt issue time.
#[derive(Debug, Clone, PartialEq)]
pub struct PleIssuedQuestion {
    pub presentation: QuestionVariationPresentation,
    pub reproduction_details: QuestionAttemptReproductionDetails,
}

/// PLE source-only backend failure.
#[derive(Debug)]
pub enum PleQuestionBackendError {
    QuestionSourceResolution(objects::QuestionSourceResolutionError),
    UnexpectedQuestionSourceMediaType { media_type: String },
    QuestionSourceDocument(question_json::PleQuestionJsonError),
    UnknownQuestionBackendVersion { version: QuestionBackendVersion },
    UnknownQuestionGraderVersion { version: QuestionGraderVersion },
    Serialization(String),
    ReproductionMismatch { field: &'static str },
}

impl std::fmt::Display for PleQuestionBackendError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::QuestionSourceResolution(error) => error.fmt(formatter),
            Self::UnexpectedQuestionSourceMediaType { media_type } => write!(
                formatter,
                "unexpected PLE Question JSON media type: {media_type}"
            ),
            Self::QuestionSourceDocument(error) => error.fmt(formatter),
            Self::UnknownQuestionBackendVersion { version } => write!(
                formatter,
                "unknown PLE Question Backend Version: {}:{}",
                version.name, version.version
            ),
            Self::UnknownQuestionGraderVersion { version } => write!(
                formatter,
                "unknown PLE Question Grader Version: {}:{}",
                version.name, version.version
            ),
            Self::Serialization(message) => {
                write!(formatter, "PLE serialization failed: {message}")
            }
            Self::ReproductionMismatch { field } => {
                write!(formatter, "PLE reproduction mismatch: {field}")
            }
        }
    }
}
impl std::error::Error for PleQuestionBackendError {}

/// Stateless PLE source executor.
#[derive(Debug, Default)]
pub struct PleQuestionBackend;

impl PleQuestionBackend {
    pub fn new() -> Self {
        Self
    }
}
