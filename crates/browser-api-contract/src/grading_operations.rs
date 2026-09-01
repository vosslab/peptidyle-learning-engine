//! Route-only accepted receipts for Instructor grading actions.

use serde::{Deserialize, Serialize};

/// Browser-safe accepted Receipt for one Instructor Grading Operation action.
///
/// The `retry_token` is the direct PLE field. HTTP `idempotency-key` remains
/// protocol framing at the route boundary and is never a DTO field name.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum InstructorGradingOperationActionReceipt {
    Retry {
        retry_token: String,
        operation: String,
        resulting_operation_revision: u64,
        occurred_at: i64,
    },
    Recalculation {
        retry_token: String,
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
    fn receipt_uses_direct_snake_retry_token() {
        let receipt = InstructorGradingOperationActionReceipt::Retry {
            retry_token: "00000000-0000-0000-0000-000000000001".to_string(),
            operation: "GO-7".to_string(),
            resulting_operation_revision: 4,
            occurred_at: 1,
        };
        let wire = serde_json::to_value(receipt).expect("receipt serializes");
        assert!(wire.get("retry_token").is_some());
        assert!(wire.get("idempotency_key").is_none());
    }
}
