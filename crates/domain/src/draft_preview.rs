//! Key-free deterministic Workspace Draft Question presentation.
//!
//! This is intentionally an adapter-independent static presentation transform.
//! For PLE, it compiles the exact source-derived Draft Question Content prompt
//! into static PLE Question JSON; it accepts no Question Seed or authored
//! parameter map, never chooses an answer, and never evaluates a response.

use question_model::QuestionContentBlock;
use question_model::capability::Capability;
use question_model::question_library::QuestionBackend;
use question_model::{QuestionResponseFormat, WorkspaceId};
use serde::{Deserialize, Serialize};

/// Browser-safe inputs needed to preview one editable workspace draft.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DraftPreviewRequest {
    /// Private workspace owning the unversioned draft.
    pub workspace: WorkspaceId,
    /// Question Backend selected by the draft.
    pub question_backend: QuestionBackend,
    /// Student-facing draft title.
    pub title: String,
    /// Authored prompt blocks.
    pub prompt: Vec<QuestionContentBlock>,
    /// Browser-safe response shape.
    pub response: QuestionResponseFormat,
}

/// Identity-free prompt presentation for one draft.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DraftQuestionPreview {
    /// Private workspace owning the draft.
    pub workspace: WorkspaceId,
    /// Student-facing title.
    pub title: String,
    /// Fully constructed prompt for the deterministic Question Variation.
    pub prompt: Vec<QuestionContentBlock>,
    /// Browser-safe response shape.
    pub response: QuestionResponseFormat,
}

/// The explicit result of a local draft-preview request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum DraftPreviewResult {
    /// PLE Question presentation is ready locally.
    Ready { preview: DraftQuestionPreview },
    /// This source needs a backend path rather than a synthetic browser preview.
    Unavailable {
        /// Question Backend selected by the draft source.
        backend: QuestionBackend,
        /// Capability the source cannot provide locally.
        capability: Capability,
    },
}

/// Produces a local preview when the Question Source is PLE.
///
/// All other draft Question Backends deliberately return an explicit capability result;
/// they do not fall back to an invented browser evaluator.
pub fn preview_ple_draft(request: &DraftPreviewRequest) -> DraftPreviewResult {
    if request.question_backend != QuestionBackend::Ple {
        return DraftPreviewResult::Unavailable {
            backend: request.question_backend,
            capability: Capability::OfflinePreview,
        };
    }
    let prompt = build_question_prompt(&request.prompt);
    DraftPreviewResult::Ready {
        preview: DraftQuestionPreview {
            workspace: request.workspace,
            title: request.title.clone(),
            prompt,
            response: request.response.clone(),
        },
    }
}

/// Returns the exact source-derived static PLE Question JSON prompt.
///
/// This helper accepts authored prompt blocks only. It accepts no Question Seed
/// or authored parameter map.
pub fn build_question_prompt(prompt: &[QuestionContentBlock]) -> Vec<QuestionContentBlock> {
    prompt.to_vec()
}
