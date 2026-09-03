//! Browser-safe contracts for an Instructor's non-mutating Question Pool Preview.
//!
//! A preview describes a saved Question Pool. It deliberately contains no
//! selected Question Pool Item identity, seed, Answer Key, student identity,
//! or issued-work record. The server owns the temporary preview nonce and discards it after the
//! response is produced.

use serde::{Deserialize, Serialize};

use crate::{
    AssignmentEditNumber, AssignmentEntryId, AssignmentReference, QuestionId,
    QuestionPoolSelectionRule,
};

/// Strict request body for an Instructor's one-off sample of a saved pool.
/// The route owns course and assignment identity; the revision comes from the
/// `If-Match` header, so the browser can select only one saved Assignment Entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QuestionPoolPreviewRequest {
    pub assignment_entry_id: AssignmentEntryId,
}

/// The public Question Library identity and title that are safe in an Instructor
/// preview.  A Question ID remains the sole human-facing question identity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QuestionPoolPreviewItem {
    pub question_id: QuestionId,
    pub title: String,
}

/// One no-store preview result for a saved assignment Question Pool.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QuestionPoolPreview {
    pub assignment: AssignmentReference,
    pub edit_number: AssignmentEditNumber,
    pub assignment_entry_id: AssignmentEntryId,
    /// Stable presentation label derived from the saved Question Pool
    /// Assignment Entry order. Question Pool Assignment Entries have no
    /// Instructor-authored label in v1.
    pub question_pool_label: String,
    pub selection_count: u32,
    pub selection_rule: QuestionPoolSelectionRule,
    pub items: Vec<QuestionPoolPreviewItem>,
    pub selected_items: Vec<QuestionPoolPreviewItem>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_is_strict_and_result_serializes_only_public_question_identity() {
        assert!(
            serde_json::from_value::<QuestionPoolPreviewRequest>(
                serde_json::json!({"assignmentEntryId": 2, "nonce": "browser-controlled"})
            )
            .is_err()
        );
        let question_id: QuestionId = "ABC-DEF1".parse().expect("canonical question ID");
        let result = QuestionPoolPreview {
            assignment: "A-4".parse().expect("assignment reference"),
            edit_number: "3".parse().expect("edit number"),
            assignment_entry_id: serde_json::from_value(serde_json::json!(
                "0198e000-0000-7000-8000-000000000017"
            ))
            .expect("entry ID"),
            question_pool_label: "Pool 3".to_string(),
            selection_count: 1,
            selection_rule: QuestionPoolSelectionRule {
                selected_question_order: crate::QuestionPoolSelectedQuestionOrder::RandomOrder,
            },
            items: vec![QuestionPoolPreviewItem {
                question_id: question_id.clone(),
                title: "Question Pool Item".to_string(),
            }],
            selected_items: vec![QuestionPoolPreviewItem {
                question_id,
                title: "Question Pool Item".to_string(),
            }],
        };
        assert_eq!(
            serde_json::to_value(result).expect("serializes"),
            serde_json::json!({
                "assignment":"A-4", "editNumber":"3", "assignmentEntryId":"0198e000-0000-7000-8000-000000000017", "questionPoolLabel":"Pool 3",
                "selectionCount":1, "selectionRule":{"selectedQuestionOrder":"randomOrder"},
                "items":[{"questionId":"ABC-DEF1", "title":"Question Pool Item"}],
                "selectedItems":[{"questionId":"ABC-DEF1", "title":"Question Pool Item"}]
            })
        );
    }
}
