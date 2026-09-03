//! Static first-party Question Implementation contract.
//!
//! The adapter owns orchestration, reproducibility, and grading delegation.
//! An implementation owns only the small piece that differs between PLE Question
//! Questions: validating static PLE Question JSON and deriving a server-only
//! answer key.

use grading::AnswerKey;
use question_model::QuestionContentBlock;
use question_model::QuestionVariationPresentation;
use question_model::capability::QuestionBackendCapabilities;
use question_model::question_content::DraftQuestionContent;
use question_model::{
    GradingResult, QuestionAnswer, QuestionAnswerExplanation, QuestionFeedback, QuestionFormat,
    QuestionHint, QuestionRevision, QuestionType, StudentResponse,
};

use crate::PleQuestionBackendError;

/// Display-ready Question Answer and Question Answer Explanation for one
/// deterministic draft variant.
///
/// This is deliberately a display-ready generated Question rather than an Answer Key:
/// choice identifiers, numeric expectations, accepted-response sets, and
/// grading rules remain inside the PLE Question Backend.
#[derive(Clone, PartialEq)]
pub struct AuthorPresentationContent {
    /// Display-ready accepted response for the exact generated variation.
    pub question_answer: Vec<QuestionContentBlock>,
    /// Optional display-ready explanation of how or why the answer is reached.
    pub question_answer_explanation: Option<Vec<QuestionContentBlock>>,
}

/// One static first-party Question Implementation.
///
/// Implement this trait to add a PLE Question Implementation without editing backend
/// dispatch, persistence, API routes, or the browser. Implementations must be
/// deterministic functions of immutable Question Content.
pub trait PleQuestionImplementation: Send + Sync {
    /// Authored representation this implementation accepts.
    fn question_format(&self) -> QuestionFormat;

    /// Educational interaction this implementation accepts.
    fn question_type(&self) -> QuestionType;

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
    ) -> Result<Option<AnswerKey>, PleQuestionBackendError>;

    /// Builds separate private Question Feedback, Question Answer, and Question
    /// Answer Explanation values
    /// after the exact issued instance has been reproduced and graded. The
    /// answer key never leaves this trusted adapter boundary; implementations
    /// return only rendered public blocks, never answer identifiers or the Answer Key.
    fn derive_question_feedback_answer_and_explanation(
        &self,
        question: &QuestionRevision,
        presentation: &QuestionVariationPresentation,
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
        let _ = (question, presentation, answer_key, result, response);
        Ok((QuestionFeedback::default(), None, None))
    }

    /// Builds one authorized pre-response hint for an exact issued Question.
    ///
    /// The caller owns the separate during-attempt hint request and its
    /// disclosure policy. A hint is never merged into post-grade feedback.
    fn derive_hint(
        &self,
        question: &QuestionRevision,
        presentation: &QuestionVariationPresentation,
        answer_key: Option<&AnswerKey>,
    ) -> Result<Option<QuestionHint>, PleQuestionBackendError> {
        let _ = (question, presentation, answer_key);
        Ok(None)
    }

    /// Produces an instructor-only, display-ready answer presentation for an
    /// editable draft. The adapter has already constructed `prompt` for the
    /// supplied deterministic variant.  Returning `None` is an honest
    /// declaration that this implementation does not yet provide a safe author view;
    /// callers must surface it as unavailable rather than exposing an answer
    /// key or fabricating a Question Answer or Question Answer Explanation.
    fn derive_author_presentation(
        &self,
        question: &DraftQuestionContent,
        prompt: &[QuestionContentBlock],
    ) -> Result<Option<AuthorPresentationContent>, PleQuestionBackendError> {
        let _ = (question, prompt);
        Ok(None)
    }
}
