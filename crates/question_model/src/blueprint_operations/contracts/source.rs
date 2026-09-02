//! Revision-bound Blueprint Course reference.

use serde::{Deserialize, Serialize};

use crate::{BlueprintCourseReference, BlueprintRevision};

/// One exact Blueprint Course and immutable Blueprint Revision pair.
///
/// This is browser-safe Blueprint Revision Reference evidence only. The Store owns draft
/// visibility, published Instructor visibility, and exact server pin resolution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct BlueprintRevisionReference {
    /// The Blueprint Course identity.
    pub reference: BlueprintCourseReference,
    /// The immutable Blueprint Revision selected for review or one operation.
    pub revision: BlueprintRevision,
}
