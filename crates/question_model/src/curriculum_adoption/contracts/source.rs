//! Revision-bound BlueprintCourse source evidence.

use serde::{Deserialize, Serialize};

use crate::{BlueprintReference, BlueprintRevision};

/// A revision-bound BlueprintCourse observed through an authorized read.
///
/// This is browser-safe locator/revision evidence only. The Store owns draft
/// visibility, published Instructor visibility, and exact server pin resolution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ObservedBlueprintSource {
    /// The sole reusable-course locator.
    pub reference: BlueprintReference,
    /// Complete ordered-tree revision selected for preview or apply.
    pub revision: BlueprintRevision,
}
