//! Route-only accepted receipts for Instructor grading actions.

use serde::{Deserialize, Serialize};

/// Browser-safe accepted Receipt for one Instructor Grading Operation action.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum InstructorGradingOperationActionReceipt {
    Retry {
        operation: String,
        resulting_operation_revision: u64,
        occurred_at: i64,
    },
    Recalculation {
        operation: String,
        resulting_operation_revision: u64,
        assignment_revision: u64,
        scoring_generation: u64,
        occurred_at: i64,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn receipt_uses_exact_operation_and_revision_facts() {
        let receipt = InstructorGradingOperationActionReceipt::Retry {
            operation: "GO-7".to_string(),
            resulting_operation_revision: 4,
            occurred_at: 1,
        };
        let wire = serde_json::to_value(receipt).expect("receipt serializes");
        assert_eq!(wire["operation"], "GO-7");
        assert_eq!(wire["resulting_operation_revision"], 4);
        assert!(wire.get("retry_token").is_none());
    }
}
