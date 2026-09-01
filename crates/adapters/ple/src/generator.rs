//! Extensible first-party Question Implementation contract (MOD-ADP-PLE).
//!
//! The adapter owns orchestration, reproducibility, and grading delegation.
//! An implementation owns only the small piece that differs between PLE Question
//! Questions: turning generated parameters into prompt blocks and a
//! server-only answer key. Adding an implementation therefore does not change
//! the engine, API, or browser contracts.

use domain::generator::QuestionVariationParameters;
use grading::AnswerKey;
use question_model::capability::QuestionBackendCapabilities;
use question_model::envelope::QuestionContentBlock;
use question_model::envelope::QuestionVariationPresentation;
use question_model::generation::QuestionGeneratorReference;
use question_model::question_content::DraftQuestionContent;
use question_model::{
    GradingResult, QuestionAnswer, QuestionAnswerExplanation, QuestionFeedback, QuestionFormat,
    QuestionHint, QuestionRevision, QuestionType, StudentResponse,
};

use crate::PleQuestionBackendError;

/// Rendered instructor teaching material for one deterministic draft variant.
///
/// This is deliberately display-ready rather than an answer-key projection:
/// choice identifiers, numeric expectations, accepted-response sets, and
/// grading rules remain inside the PLE Question Backend.
#[derive(Clone, PartialEq)]
pub struct AuthorPresentationContent {
    /// Display-ready accepted response for the exact generated variation.
    pub question_answer: Vec<QuestionContentBlock>,
    /// Optional display-ready explanation of how or why the answer is reached.
    pub question_answer_explanation: Option<Vec<QuestionContentBlock>>,
}

/// Exact release of one PLE Question Implementation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PleQuestionImplementationRelease {
    /// Stable PLE Question Implementation identifier.
    pub id: String,
    /// Implementation compatibility release.
    pub version: String,
}

/// One versioned first-party Question Implementation.
///
/// Implement this trait to add a PLE Question Implementation without editing backend
/// dispatch, persistence, API routes, or the browser. Implementations must be
/// deterministic functions of immutable Question Content and generated Question Variation Parameters.
pub trait PleQuestionImplementation: Send + Sync {
    /// Authored representation this implementation accepts.
    fn question_format(&self) -> QuestionFormat;

    /// Educational interaction this implementation accepts.
    fn question_type(&self) -> QuestionType;

    /// Stable PLE Question Implementation name and exact release.
    fn implementation_release(&self) -> PleQuestionImplementationRelease;

    /// Exact additive Question Generator supported by this implementation.
    ///
    /// Static Question Implementations return `None`. A behavior change requires a new
    /// generator version and a new Question Implementation kept alongside the
    /// old one while published content references it.
    fn generator(&self) -> Option<QuestionGeneratorReference>;

    /// Capabilities this implementation can honestly provide now.
    fn capabilities(&self) -> QuestionBackendCapabilities;

    /// Derives the server-only Answer Key after shared prompt construction.
    ///
    /// # Errors
    ///
    /// Returns [`PleQuestionBackendError::IncompatibleQuestionImplementation`] when
    /// the authored Question does not satisfy this implementation's contract.
    fn derive_answer_key(
        &self,
        question: &QuestionRevision,
        generated: &QuestionVariationParameters,
    ) -> Result<Option<AnswerKey>, PleQuestionBackendError>;

    /// Builds separate private Question Feedback, Question Answer, and Question
    /// Answer Explanation values
    /// after the exact issued instance has been reproduced and graded. The
    /// answer key never leaves this trusted adapter boundary; implementations
    /// return only rendered public blocks, never answer identifiers or key material.
    fn derive_question_feedback_answer_and_explanation(
        &self,
        question: &QuestionRevision,
        generated: &QuestionVariationParameters,
        envelope: &QuestionVariationPresentation,
        answer_key: Option<&AnswerKey>,
        result: &GradingResult,
        response: &StudentResponse,
    ) -> Result<
        (
            QuestionFeedback,
            Option<QuestionAnswer>,
            Option<QuestionAnswerExplanation>,
        ),
        PleQuestionBackendError,
    > {
        let _ = (question, generated, envelope, answer_key, result, response);
        Ok((QuestionFeedback::default(), None, None))
    }

    /// Builds one authorized pre-response hint for an exact issued Question.
    ///
    /// The caller owns the separate during-attempt hint request and its
    /// disclosure policy. A hint is never merged into post-grade feedback.
    fn derive_hint(
        &self,
        question: &QuestionRevision,
        generated: &QuestionVariationParameters,
        envelope: &QuestionVariationPresentation,
        answer_key: Option<&AnswerKey>,
    ) -> Result<Option<QuestionHint>, PleQuestionBackendError> {
        let _ = (question, generated, envelope, answer_key);
        Ok(None)
    }

    /// Produces an instructor-only, display-ready answer presentation for an
    /// editable draft. The adapter has already constructed `prompt` for the
    /// supplied deterministic variant.  Returning `None` is an honest
    /// declaration that this implementation does not yet provide a safe author view;
    /// callers must surface it as unavailable rather than exposing an answer
    /// key or fabricating teaching material.
    fn derive_author_presentation(
        &self,
        question: &DraftQuestionContent,
        generated: &QuestionVariationParameters,
        prompt: &[QuestionContentBlock],
    ) -> Result<Option<AuthorPresentationContent>, PleQuestionBackendError> {
        let _ = (question, generated, prompt);
        Ok(None)
    }
}
