use serde::{Deserialize, Serialize};

use super::TeachingOperationRevision;

/// Browser-safe strong revision returned after an accepted M2/M3/M4 mutation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TeachingOperationRevisionResponse {
    pub revision: TeachingOperationRevision,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn response_serializes_the_canonical_decimal_revision() {
        let response = TeachingOperationRevisionResponse {
            revision: "42".parse().unwrap(),
        };
        assert_eq!(
            serde_json::to_value(response).unwrap(),
            serde_json::json!({"revision":"42"})
        );
        assert!(
            serde_json::from_str::<TeachingOperationRevisionResponse>(
                r#"{"revision":"42","extra":true}"#
            )
            .is_err()
        );
    }
}
