//! Browser-safe Student Response format validation.
//!
//! This module can inspect Question Response Formats and Student input, but it has
//! no answer key and makes no correctness decision. The server repeats the
//! same validation before calling the server-only `grading` crate.

mod authored_response;
mod presentation_response;

pub use authored_response::validate_response_format;
pub use presentation_response::validate_presentation_response_format;

use std::collections::BTreeSet;

use question_model::answer::ResponseSelectionRule;
use question_model::response::ResponseItemId;
use serde::{Deserialize, Serialize};

/// One reason a student response cannot be submitted in its current form.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum StudentResponseFormatIssue {
    /// The response kind does not match the Question Response Format.
    ResponseKindMismatch,
    /// A numeric response is NaN or infinite.
    NumericNotFinite,
    /// The number of selected response items violates the declared Response Selection Rule.
    SelectionCount {
        /// Response Selection Rule declared by the question.
        expected: ResponseSelectionRule,
        /// Number of selected Response Item IDs in the submitted response.
        actual: u64,
    },
    /// A selected Response Item ID appears more than once in one response.
    DuplicateChoice {
        /// Repeated Response Item ID.
        choice: ResponseItemId,
    },
    /// A submitted Response Item ID does not occur in the Question Revision.
    UnknownChoice {
        /// Unrecognized Response Item ID.
        choice: ResponseItemId,
    },
    /// Short text exceeds the question's character limit.
    TextTooLong {
        /// Maximum allowed Unicode scalar values.
        max_length: u32,
        /// Submitted Unicode scalar values.
        actual_length: u64,
    },
    /// A multi-blank response repeats a slot or names a slot absent from the Question.
    BlankSlotsMismatch,
    /// A matching response repeats or names a prompt absent from the Question Response Format.
    MatchingPromptsMismatch,
    /// A matching response repeats a choice where the Question Response Format requires a permutation.
    DuplicateMatchChoice {
        /// Reused Matching Choice ID.
        choice: ResponseItemId,
    },
    /// A matching response names a choice absent from the Question Response Format.
    UnknownMatchChoice {
        /// Unrecognized Matching Choice ID.
        choice: ResponseItemId,
    },
    /// An ordering response is not an exact permutation of the defined items.
    OrderingItemsMismatch,
    /// A Hotspot Region appears more than once in one response.
    DuplicateHotspotRegion {
        /// Repeated Hotspot Region ID.
        region: ResponseItemId,
    },
    /// A Student Hotspot Selection names a region absent from the Question Response Format.
    UnknownHotspotRegion {
        /// Unrecognized Hotspot Region ID.
        region: ResponseItemId,
    },
}

/// Complete browser-safe format verdict for one student response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StudentResponseFormatCheck {
    /// Every format issue found, in stable validation order.
    pub issues: Vec<StudentResponseFormatIssue>,
}

impl StudentResponseFormatCheck {
    /// Whether the response is structurally ready for server submission.
    pub fn is_valid(&self) -> bool {
        self.issues.is_empty()
    }
}

fn validate_text_length(max_length: u32, text: &str, issues: &mut Vec<StudentResponseFormatIssue>) {
    let actual_length = count(text.chars());
    if actual_length > u64::from(max_length) {
        issues.push(StudentResponseFormatIssue::TextTooLong {
            max_length,
            actual_length,
        });
    }
}

fn validate_hotspot_region_ids(
    selections: &[question_model::response::StudentHotspotSelection],
    available: &BTreeSet<ResponseItemId>,
    issues: &mut Vec<StudentResponseFormatIssue>,
) {
    let mut observed = BTreeSet::new();
    for selection in selections {
        if !available.contains(&selection.region) {
            issues.push(StudentResponseFormatIssue::UnknownHotspotRegion {
                region: selection.region.clone(),
            });
        }
        if !observed.insert(selection.region.clone()) {
            issues.push(StudentResponseFormatIssue::DuplicateHotspotRegion {
                region: selection.region.clone(),
            });
        }
    }
}

/// Converts an iterator count without target-width differences in the check.
fn count(values: impl Iterator) -> u64 {
    u64::try_from(values.count()).expect("supported usize values fit u64")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn issue_json_uses_the_browser_camel_case_contract() {
        let check = StudentResponseFormatCheck {
            issues: vec![StudentResponseFormatIssue::TextTooLong {
                max_length: 2,
                actual_length: 3,
            }],
        };

        assert_eq!(
            serde_json::to_value(check).expect("Student Response Format Check should serialize"),
            serde_json::json!({
                "issues": [{
                    "kind": "textTooLong",
                    "maxLength": 2,
                    "actualLength": 3
                }]
            })
        );
    }
}
