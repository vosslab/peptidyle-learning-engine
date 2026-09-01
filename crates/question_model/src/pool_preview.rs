//! Browser-safe contracts for an Instructor's non-mutating Question Pool Preview.
//!
//! A preview describes a saved Question Pool. It deliberately contains no
//! candidate identity, seed, answer material, student identity, or issued-work
//! record. The server owns the temporary preview nonce and discards it after the
//! response is produced.

use serde::{Deserialize, Serialize};

use crate::{
    AssignmentEntryId, AssignmentReference, QuestionId, QuestionPoolSelectionRule,
    TeachingOperationRevision,
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
pub struct QuestionPoolPreviewQuestion {
    pub question_id: QuestionId,
    pub title: String,
}

/// One no-store preview result for a saved assignment Question Pool.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QuestionPoolPreview {
    pub assignment: AssignmentReference,
    pub revision: TeachingOperationRevision,
    pub assignment_entry_id: AssignmentEntryId,
    /// Stable presentation label derived from the saved entry order. Pool
    /// definitions have no user-authored label in v1.
    pub question_pool_label: String,
    pub selection_count: u32,
    pub selection_rule: QuestionPoolSelectionRule,
    pub entries: Vec<QuestionPoolPreviewQuestion>,
    pub selected: Vec<QuestionPoolPreviewQuestion>,
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
            revision: TeachingOperationRevision::new(3).expect("revision"),
            assignment_entry_id: serde_json::from_value(serde_json::json!(
                "0198e000-0000-7000-8000-000000000017"
            ))
            .expect("entry ID"),
            question_pool_label: "Pool 3".to_string(),
            selection_count: 1,
            selection_rule: QuestionPoolSelectionRule {
                selected_question_order: crate::QuestionPoolSelectedQuestionOrder::RandomOrder,
            },
            entries: vec![QuestionPoolPreviewQuestion {
                question_id: question_id.clone(),
                title: "Pool entry".to_string(),
            }],
            selected: vec![QuestionPoolPreviewQuestion {
                question_id,
                title: "Pool entry".to_string(),
            }],
        };
        assert_eq!(
            serde_json::to_value(result).expect("serializes"),
            serde_json::json!({
                "assignment":"A-4", "revision":"3", "assignmentEntryId":"0198e000-0000-7000-8000-000000000017", "questionPoolLabel":"Pool 3",
                "selectionCount":1, "selectionRule":{"selectedQuestionOrder":"randomOrder"},
                "entries":[{"questionId":"ABC-DEF1", "title":"Pool entry"}],
                "selected":[{"questionId":"ABC-DEF1", "title":"Pool entry"}]
            })
        );
    }
}
