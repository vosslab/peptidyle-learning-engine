//! Browser-safe projections for an active Student Assignment Attempt.

use question_model::{
    QuestionContentBlock, QuestionPresentationResponseFormat, QuestionRevisionReference,
};
use serde::{Deserialize, Serialize};

/// One selected, answer-free Question presentation owned by the active Student.
///
/// `question_revision` is the exact immutable revision retained by the Issued
/// Question.  It lets the browser request only assets authorized for that
/// revision; it is not resolved from mutable Assignment or Question state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StudentQuestionPresentation {
    pub question_revision: QuestionRevisionReference,
    pub prompt: Vec<QuestionContentBlock>,
    pub response: QuestionPresentationResponseFormat,
}

#[cfg(test)]
mod tests {
    use super::*;
    use question_model::{QuestionId, QuestionRevisionNumber};

    #[test]
    fn selected_presentation_keeps_the_exact_revision_for_asset_urls() {
        let question_id: QuestionId = "000-000N".parse().expect("question ID");
        let presentation = StudentQuestionPresentation {
            question_revision: QuestionRevisionReference {
                question_id: question_id.clone(),
                revision_number: QuestionRevisionNumber::new(3).expect("revision"),
            },
            prompt: Vec::new(),
            response: QuestionPresentationResponseFormat::ImathasQuestionBackend {},
        };

        let wire = serde_json::to_value(presentation).expect("presentation serializes");
        assert_eq!(
            wire["questionRevision"]["questionId"],
            question_id.to_string()
        );
        assert_eq!(wire["questionRevision"]["revisionNumber"], 3);
    }
}
