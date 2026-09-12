//! Browser-safe response projections for immutable Blueprint publication.

use question_model::{BlueprintModuleView, BlueprintRevisionReference};
use serde::{Deserialize, Serialize};

/// Answer-free immutable content resolved by its exact Blueprint Revision Reference.
///
/// The reference identifies one published Revision.  It is independent of the
/// mutable Blueprint Draft and stable-lineage availability state.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BlueprintRevisionView {
    pub blueprint_revision: BlueprintRevisionReference,
    pub title: String,
    pub modules: Vec<BlueprintModuleView>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use question_model::{BlueprintCourseReference, BlueprintRevision};

    #[test]
    fn revision_view_keeps_the_exact_immutable_reference() {
        let view = BlueprintRevisionView {
            blueprint_revision: BlueprintRevisionReference {
                reference: "BP-12"
                    .parse::<BlueprintCourseReference>()
                    .expect("reference"),
                revision: BlueprintRevision::new(3).expect("revision"),
            },
            title: "Biochemistry Blueprint".to_string(),
            modules: Vec::new(),
        };

        let wire = serde_json::to_value(view).expect("view serializes");
        assert_eq!(wire["blueprintRevision"]["revision"], "3");
    }
}
