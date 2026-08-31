//! Answer *shapes*: what a valid response looks like (WP-C1).
//!
//! Read this before adding a type here. The answer format belongs in this
//! crate: how close a number must be, how text is compared, how many choices
//! may be selected. Those are shared content, and the browser needs them to
//! validate a response's shape before submitting it.
//!
//! The answer *value* is an answer key and belongs in `crates/grading`, which
//! runs server-side. A useful test for where a type belongs: if it would let a
//! caller learn the correct response, it belongs in `grading`.
//!
//! Everything here is safe to ship to a browser.

use serde::{Deserialize, Serialize};

/// How close a numeric response must be to count as correct.
///
/// The tolerance is shared content because the browser shows it to the student
/// ("within 1%") and validates the entry format against it. The correct value
/// stays server-side.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum NumericResponseTolerance {
    /// The response must match exactly, digit for digit.
    Exact,
    /// The response must fall within a fixed distance of the expected value.
    Absolute {
        /// Maximum permitted difference.
        epsilon: f64,
    },
    /// The response must fall within a fraction of the expected value.
    Relative {
        /// Permitted fraction, where 0.01 means one percent.
        fraction: f64,
    },
    /// The response must agree to a number of significant figures.
    SignificantFigures {
        /// How many significant figures are compared.
        digits: u8,
    },
}

/// How a text response is compared.
///
/// Normalization rules are shared content so the browser can show a student
/// what will be ignored, which prevents the "I typed the right answer" dispute
/// that comes from invisible whitespace.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum TextResponseMatchRule {
    /// Character-for-character comparison.
    Exact,
    /// Comparison that treats upper and lower case as equal.
    CaseInsensitive,
    /// Comparison after trimming and collapsing whitespace, and folding case.
    Normalized,
}

/// How many choices a student may select.
///
/// Encoded as a type so a widget knows whether to render radio buttons or
/// checkboxes, and so a response carrying the wrong count is rejected by shape
/// rather than by a grading round trip.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum ResponseSelectionRule {
    /// Exactly one choice: radio buttons.
    ExactlyOne,
    /// A fixed number of choices.
    Exactly {
        /// How many choices the student selects.
        count: u32,
    },
    /// Any number of choices, including none: checkboxes.
    AnyNumber,
    /// At least one choice, with no upper limit.
    AtLeastOne,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tolerances_serialize_with_their_parameters() {
        let tolerance = NumericResponseTolerance::Relative { fraction: 0.01 };
        let json = serde_json::to_string(&tolerance).expect("serialization should succeed");
        assert_eq!(json, r#"{"kind":"relative","fraction":0.01}"#);
    }

    #[test]
    fn text_match_modes_use_camel_case_names() {
        let json = serde_json::to_string(&TextResponseMatchRule::CaseInsensitive)
            .expect("serialization works");
        assert_eq!(json, r#""caseInsensitive""#);
    }
}
