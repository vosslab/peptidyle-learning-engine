//! Server-only answer-key types (WP-C6, MOD-GRD).
//!
//! These values decide deterministic correctness. They therefore stay in
//! `grading`, outside the generated browser types and the WASM dependency
//! closure.

use std::collections::{BTreeMap, BTreeSet};

use question_model::response::ResponseItemReference;
use serde::{Deserialize, Serialize};

/// Correct response material for one question revision.
///
/// An ungraded question has no `AnswerKey`; it does not use a non-secret
/// placeholder variant. Keeping every value here server-only makes the crate
/// boundary easy to audit.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum AnswerKey {
    /// Expected numeric value before the public tolerance is applied.
    Numeric {
        /// Correct numeric value.
        expected: f64,
    },
    /// Correct selection identifiers.
    MultipleChoice {
        /// Exact correct set; order is irrelevant.
        correct: BTreeSet<ResponseItemReference>,
    },
    /// Accepted short-text values before the public match mode is applied.
    ShortText {
        /// Accepted answers in authoring order.
        accepted: Vec<String>,
    },
    /// Accepted values for every named multi-blank slot.
    MultiBlank {
        /// Slot identifier to accepted answer forms.
        accepted: BTreeMap<ResponseItemReference, Vec<String>>,
    },
    /// Correct prompt-to-choice matching.
    Matching {
        /// Prompt identifier to correct choice identifier.
        correct: BTreeMap<ResponseItemReference, ResponseItemReference>,
    },
    /// Correct ordering of item identifiers.
    Ordering {
        /// Expected identifiers from first to last.
        correct: Vec<ResponseItemReference>,
    },
    /// Correct candidate regions on a hotspot surface.
    Hotspot {
        /// Exact correct region set; geometry remains in the public definition.
        correct: BTreeSet<ResponseItemReference>,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn answer_values_serialize_only_inside_the_server_crate() {
        let key = AnswerKey::MultipleChoice {
            correct: BTreeSet::from([ResponseItemReference::new("peptide-bond")]),
        };

        assert_eq!(
            serde_json::to_string(&key).expect("answer key should serialize"),
            r#"{"kind":"multipleChoice","correct":["peptide-bond"]}"#
        );
    }
}
