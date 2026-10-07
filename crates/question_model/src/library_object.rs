//! Shared identity vocabulary for the Published Question Library.

use serde::{Deserialize, Serialize};

/// The kind of reusable object identified by a Library public ID.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LibraryObjectKind {
    /// A stable Published Question lineage.
    Question,
    /// A stable published Question Pool lineage.
    QuestionPool,
}
