//! Browser-safe contracts for an Instructor's non-mutating item-pool sample.
//!
//! A preview describes a saved selection group.  It deliberately contains no
//! candidate identity, seed, answer material, student identity, or issued-work
//! record.  The server owns the temporary draw nonce and discards it after the
//! response is produced.

use serde::{Deserialize, Serialize};

use crate::{
    AssignmentReference, PoolDrawAlgorithm, QuestionId, SelectionOrdering,
    TeachingOperationRevision,
};

/// Strict request body for an Instructor's one-off sample of a saved pool.
/// The route owns course and assignment identity; the revision comes from the
/// `If-Match` header, so the browser can select only the visible group position.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PoolDrawPreviewRequest {
    pub group_position: u32,
}

/// The public catalog identity and title that are safe in an Instructor
/// preview.  A Question ID remains the sole human-facing question identity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PoolDrawPreviewQuestion {
    pub question_id: QuestionId,
    pub title: String,
}

/// One no-store preview result for a saved assignment selection group.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PoolDrawPreview {
    pub assignment: AssignmentReference,
    pub revision: TeachingOperationRevision,
    pub group_position: u32,
    /// Stable presentation label derived from the saved shared position. Pool
    /// definitions have no user-authored label in v1.
    pub group_label: String,
    pub draw_count: u32,
    pub ordering: SelectionOrdering,
    pub algorithm: PoolDrawAlgorithm,
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
                serde_json::json!({"groupPosition": 2, "nonce": "browser-controlled"})
            )
            .is_err()
        );
        let question_id: QuestionId = "ABC-DEF1".parse().expect("canonical question ID");
        let result = PoolDrawPreview {
            assignment: "A-4".parse().expect("assignment reference"),
            revision: TeachingOperationRevision::new(3).expect("revision"),
            group_position: 2,
            group_label: "Pool 3".to_string(),
            draw_count: 1,
            ordering: SelectionOrdering::Randomized,
            algorithm: PoolDrawAlgorithm::V1,
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
                "assignment":"A-4", "revision":"3", "groupPosition":2, "groupLabel":"Pool 3",
                "drawCount":1, "ordering":"randomized", "algorithm":"v1",
                "candidates":[{"questionId":"ABC-DEF1", "title":"Pool candidate"}],
                "sampled":[{"questionId":"ABC-DEF1", "title":"Pool candidate"}]
            })
        );
    }
}
