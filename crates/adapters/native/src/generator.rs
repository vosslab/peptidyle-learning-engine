//! Extensible first-party Question Implementation contract (MOD-ADP-NAT).
//!
//! The adapter owns orchestration, reproducibility, and grading delegation.
//! An implementation owns only the small piece that differs between native
//! Questions: turning generated parameters into prompt blocks and a
//! server-only answer key. Adding an implementation therefore does not change
//! the engine, API, or browser contracts.

use domain::generator::QuestionVariationParameters;
use grading::AnswerKey;
use question_model::capability::QuestionBackendCapabilities;
use question_model::definition::DraftQuestionDefinition;
use question_model::envelope::ContentBlock;
use question_model::envelope::QuestionPresentation;
use question_model::generation::QuestionGeneratorReference;
use question_model::{
    GradingResult, QuestionFormat, QuestionHint, QuestionPostGradingContent, QuestionType,
    QuestionVersion, StudentResponse,
};

use crate::NativeAdapterError;

/// Rendered instructor teaching material for one deterministic draft variant.
///
/// This is deliberately display-ready rather than an answer-key projection:
/// choice identifiers, numeric expectations, accepted-response sets, and
/// grading rules remain inside the native adapter.
#[derive(Clone, PartialEq)]
pub struct AuthorPresentationContent {
    /// Display-ready accepted response for the exact generated variation.
    pub question_answer: Vec<ContentBlock>,
    /// Optional display-ready explanation of how or why the answer is reached.
    pub question_answer_explanation: Option<Vec<ContentBlock>>,
}

/// Exact release of one Native Question Implementation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeQuestionImplementationRelease {
    /// Stable Native Question Implementation identifier.
    pub id: String,
    /// Implementation compatibility release.
    pub version: String,
}

/// One versioned first-party Question Implementation.
///
/// Implement this trait to add a native implementation without editing adapter
/// dispatch, persistence, API routes, or the browser. Implementations must be
/// deterministic functions of the immutable definition and generated Question Variation Parameters.
pub trait NativeQuestionImplementation: Send + Sync {
    /// Authored representation this implementation accepts.
    fn question_format(&self) -> QuestionFormat;

    /// Educational interaction this implementation accepts.
    fn question_type(&self) -> QuestionType;

    /// Stable native implementation name and exact release.
    fn implementation_release(&self) -> NativeQuestionImplementationRelease;

    /// Exact additive Question Generator supported by this implementation.
    ///
    /// Static Question Implementations return `None`. A behavior change requires a new
    /// generator version and a new Question Implementation kept alongside the
    /// old one while published content references it.
    fn generator(&self) -> Option<QuestionGeneratorReference>;

    /// Capabilities this implementation can honestly provide now.
    fn capabilities(&self) -> QuestionBackendCapabilities;

    /// Derives the server-only Answer Key after shared prompt materialization.
    ///
    /// # Errors
    ///
    /// Returns [`NativeAdapterError::IncompatibleQuestionImplementation`] when
    /// the authored Question does not satisfy this implementation's contract.
    fn derive_answer_key(
        &self,
        question: &QuestionVersion,
        generated: &QuestionVariationParameters,
    ) -> Result<Option<AnswerKey>, NativeAdapterError>;

    /// Builds sanitized teaching material after the exact issued instance has
    /// been reproduced and graded. The answer key never leaves this trusted
    /// adapter boundary; implementations must return rendered public blocks,
    /// not answer identifiers or key material.
    fn derive_post_grading_content(
        &self,
        question: &QuestionVersion,
        generated: &QuestionVariationParameters,
        envelope: &QuestionPresentation,
        answer_key: Option<&AnswerKey>,
        result: &GradingResult,
        response: &StudentResponse,
    ) -> Result<QuestionPostGradingContent, NativeAdapterError> {
        let _ = (question, generated, envelope, answer_key, result, response);
        Ok(QuestionPostGradingContent::default())
    }

    /// Builds one authorized pre-response hint for an exact issued Question.
    ///
    /// The caller owns the separate during-attempt hint request and its
    /// disclosure policy. A hint is never merged into post-grade feedback.
    fn derive_hint(
        &self,
        question: &QuestionVersion,
        generated: &QuestionVariationParameters,
        envelope: &QuestionPresentation,
        answer_key: Option<&AnswerKey>,
    ) -> Result<Option<QuestionHint>, NativeAdapterError> {
        let _ = (question, generated, envelope, answer_key);
        Ok(None)
    }

    /// Produces an instructor-only, display-ready answer presentation for an
    /// editable draft.  The adapter has already materialized `prompt` for the
    /// supplied deterministic variant.  Returning `None` is an honest
    /// declaration that this implementation does not yet provide a safe author view;
    /// callers must surface it as unavailable rather than exposing an answer
    /// key or fabricating teaching material.
    fn derive_author_presentation(
        &self,
        question: &DraftQuestionDefinition,
        generated: &QuestionVariationParameters,
        prompt: &[ContentBlock],
    ) -> Result<Option<AuthorPresentationContent>, NativeAdapterError> {
        let _ = (question, generated, prompt);
        Ok(None)
    }
}
