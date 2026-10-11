//! Browser-safe projections for an active Student Assessment Attempt.

use question_model::{
    PublishedQuestionRevisionTuple, QuestionContentBlock, QuestionPresentationResponseFormat,
};
use serde::{Deserialize, Serialize};

/// One selected, answer-free Question presentation owned by the active Student.
///
/// `published_question_revision_tuple` is the exact immutable revision retained by the Issued
/// Question.  It lets the browser request only assets authorized for that
/// revision; it is not resolved from mutable Assessment or Question state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StudentQuestionPresentation {
    pub published_question_revision_tuple: PublishedQuestionRevisionTuple,
    /// Optional integrity binding for separately served isolated author content.
    /// This is never raw source, a URL, or a browser capability.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author_content_digest: Option<String>,
    pub prompt: Vec<QuestionContentBlock>,
    pub response: QuestionPresentationResponseFormat,
}
