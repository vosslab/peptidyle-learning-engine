//! Browser-safe contracts for an Instructor's non-mutating item-pool sample.
//!
//! A preview describes a saved Question Pool. It deliberately contains no
//! candidate identity, seed, answer material, student identity, or issued-work
//! record.  The server owns the temporary draw nonce and discards it after the
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
pub struct PoolDrawPreviewRequest {
    pub assignment_entry_id: AssignmentEntryId,
}

/// The public Question Library identity and title that are safe in an Instructor
/// preview.  A Question ID remains the sole human-facing question identity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PoolDrawPreviewQuestion {
    pub question_id: QuestionId,
    pub title: String,
}

/// One no-store preview result for a saved assignment Question Pool.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PoolDrawPreview {
    pub assignment: AssignmentReference,
    pub revision: TeachingOperationRevision,
    pub assignment_entry_id: AssignmentEntryId,
    /// Stable presentation label derived from the saved entry order. Pool
    /// definitions have no user-authored label in v1.
    pub question_pool_label: String,
    pub draw_count: u32,
    pub selection_rule: QuestionPoolSelectionRule,
    pub candidates: Vec<PoolDrawPreviewQuestion>,
    pub sampled: Vec<PoolDrawPreviewQuestion>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_is_strict_and_result_serializes_only_public_question_identity() {
        assert!(
            serde_json::from_value::<PoolDrawPreviewRequest>(
                serde_json::json!({"assignmentEntryId": 2, "nonce": "browser-controlled"})
            )
            .is_err()
        );
        let question_id: QuestionId = "ABC-DEF1".parse().expect("canonical question ID");
        let result = PoolDrawPreview {
            assignment: "A-4".parse().expect("assignment reference"),
            revision: TeachingOperationRevision::new(3).expect("revision"),
            assignment_entry_id: serde_json::from_value(serde_json::json!(
                "0198e000-0000-7000-8000-000000000017"
            ))
            .expect("entry ID"),
            question_pool_label: "Pool 3".to_string(),
            draw_count: 1,
            selection_rule: QuestionPoolSelectionRule {
                ordering: crate::SelectionOrdering::Randomized,
                algorithm: crate::PoolDrawAlgorithm::V1,
            },
            candidates: vec![PoolDrawPreviewQuestion {
                question_id: question_id.clone(),
                title: "Pool candidate".to_string(),
            }],
            sampled: vec![PoolDrawPreviewQuestion {
                question_id,
                title: "Pool candidate".to_string(),
            }],
        };
        assert_eq!(
            serde_json::to_value(result).expect("serializes"),
            serde_json::json!({
                "assignment":"A-4", "revision":"3", "assignmentEntryId":"0198e000-0000-7000-8000-000000000017", "questionPoolLabel":"Pool 3",
                "drawCount":1, "selectionRule":{"ordering":"randomized", "algorithm":"v1"},
                "candidates":[{"questionId":"ABC-DEF1", "title":"Pool candidate"}],
                "sampled":[{"questionId":"ABC-DEF1", "title":"Pool candidate"}]
            })
        );
    }
}
