//! Extensible first-party question-family contract (MOD-ADP-NAT).
//!
//! The adapter owns orchestration, reproducibility, and grading delegation.
//! A family owns only the small piece that differs between native question
//! kinds: turning generated parameters into prompt blocks and a server-only
//! answer key. Adding a family therefore does not change the engine, API, or
//! browser contracts.

use domain::generator::GeneratedVariant;
use grading::AnswerKey;
use question_model::capability::BackendCapabilities;
use question_model::definition::DraftQuestionDefinition;
use question_model::envelope::ContentBlock;
use question_model::envelope::QuestionEnvelope;
use question_model::generation::GeneratorReference;
use question_model::{AttemptResult, FeedbackContent, QuestionDefinition, StudentResponse};

use crate::NativeAdapterError;

/// Rendered instructor teaching material for one deterministic draft variant.
///
/// This is deliberately display-ready rather than an answer-key projection:
/// choice identifiers, numeric expectations, accepted-response sets, and
/// grading rules remain inside the native adapter.
#[derive(Clone, PartialEq)]
pub struct AuthorPresentationContent {
    /// Accessible blocks that explain the correct response.
    pub correct_response: Vec<ContentBlock>,
    /// Optional teaching explanation for why that response is correct.
    pub rationale: Option<Vec<ContentBlock>>,
}

/// One versioned first-party question family.
///
/// Implement this trait to add a native family without editing adapter
/// dispatch, persistence, API routes, or the browser. Implementations must be
/// deterministic functions of the immutable definition and generated variant.
pub trait NativeQuestionFamily: Send + Sync {
    /// Stable value stored in [`question_model::QuestionSource::Native`].
    fn family(&self) -> &'static str;

    /// Exact additive generator implementation supported by this family.
    ///
    /// Static families return `None`. A behavior change requires a new
    /// generator version and a new family implementation kept alongside the
    /// old one while published content references it.
    fn generator(&self) -> Option<GeneratorReference>;

    /// Capabilities this implementation can honestly provide now.
    fn capabilities(&self) -> BackendCapabilities;

    /// Derives server-only grading material after shared prompt materialization.
    ///
    /// # Errors
    ///
    /// Returns [`NativeAdapterError::InvalidFamilyDefinition`] when the
    /// authored question does not satisfy this family's versioned contract.
    fn derive_answer_key(
        &self,
        question: &QuestionDefinition,
        generated: &GeneratedVariant,
    ) -> Result<Option<AnswerKey>, NativeAdapterError>;

    /// Builds sanitized teaching material after the exact issued instance has
    /// been reproduced and graded. The answer key never leaves this trusted
    /// adapter boundary; implementations must return rendered public blocks,
    /// not answer identifiers or key material.
    fn derive_feedback(
        &self,
        question: &QuestionDefinition,
        generated: &GeneratedVariant,
        envelope: &QuestionEnvelope,
        answer_key: Option<&AnswerKey>,
        result: &AttemptResult,
        response: &StudentResponse,
    ) -> Result<FeedbackContent, NativeAdapterError> {
        let _ = (question, generated, envelope, answer_key, result, response);
        Ok(FeedbackContent::default())
    }

    /// Produces an instructor-only, display-ready answer presentation for an
    /// editable draft.  The adapter has already materialized `prompt` for the
    /// supplied deterministic variant.  Returning `None` is an honest
    /// declaration that this family does not yet provide a safe author view;
    /// callers must surface it as unavailable rather than exposing an answer
    /// key or fabricating teaching material.
    fn derive_author_presentation(
        &self,
        question: &DraftQuestionDefinition,
        generated: &GeneratedVariant,
        prompt: &[ContentBlock],
    ) -> Result<Option<AuthorPresentationContent>, NativeAdapterError> {
        let _ = (question, generated, prompt);
        Ok(None)
    }
}
