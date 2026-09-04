//! Closed browser records for the exact Blueprint-operation lifecycle.

use serde::{Deserialize, Serialize};

use super::{
    ApplyBlueprintUpdateCompleted, ApplyBlueprintUpdatePreview, ApplyBlueprintUpdatePreviewRequest,
    CopyAssignmentFromBlueprintCompleted, CopyAssignmentFromBlueprintPreview,
    CopyAssignmentFromBlueprintPreviewRequest, CopyCourseForNewTermCompleted,
    CopyCourseForNewTermPreview, CopyCourseForNewTermPreviewRequest,
    CreateCourseFromBlueprintCompleted, CreateCourseFromBlueprintPreviewRequest,
    CreateCourseFromBlueprintPreviewView, ForkBlueprintCourseCompleted,
    ForkBlueprintCoursePreviewRequest, ForkBlueprintCoursePreviewView, ShiftCourseDatesCompleted,
    ShiftCourseDatesPreview, ShiftCourseDatesPreviewRequest,
};

/// One closed, answer-free browser request for a Blueprint-operation preview.
///
/// The operation tag selects exactly one Store-owned operation. Each request carries only opaque action References and
/// correction intent; the Store resolves current authorization and mutable facts before previewing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "operation", rename_all = "snake_case", deny_unknown_fields)]
pub enum BlueprintOperationPreviewRequest {
    ForkBlueprintCourse {
        request: ForkBlueprintCoursePreviewRequest,
    },
    CreateCourseFromBlueprint {
        request: CreateCourseFromBlueprintPreviewRequest,
    },
    CopyCourseForNewTerm {
        request: CopyCourseForNewTermPreviewRequest,
    },
    ShiftCourseDates {
        request: ShiftCourseDatesPreviewRequest,
    },
    ApplyBlueprintUpdate {
        request: ApplyBlueprintUpdatePreviewRequest,
    },
    CopyAssignmentFromBlueprint {
        request: CopyAssignmentFromBlueprintPreviewRequest,
    },
}

/// One closed, answer-free Store preview for a Blueprint operation.
///
/// A preview explains the facts resolved at preview time. It grants no apply authority and can be
/// stale as soon as it is returned.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "operation", rename_all = "snake_case", deny_unknown_fields)]
pub enum BlueprintOperationPreview {
    ForkBlueprintCourse {
        preview: ForkBlueprintCoursePreviewView,
    },
    CreateCourseFromBlueprint {
        preview: CreateCourseFromBlueprintPreviewView,
    },
    CopyCourseForNewTerm {
        preview: CopyCourseForNewTermPreview,
    },
    ShiftCourseDates {
        preview: ShiftCourseDatesPreview,
    },
    ApplyBlueprintUpdate {
        preview: ApplyBlueprintUpdatePreview,
    },
    CopyAssignmentFromBlueprint {
        preview: CopyAssignmentFromBlueprintPreview,
    },
}

/// One closed, answer-free completion emitted from an atomic Blueprint operation.
///
/// The Store owns issuance and consumption of the non-Serde apply record that produces this
/// completion. The browser receives the result, never the record or immutable receipt evidence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "operation", rename_all = "snake_case", deny_unknown_fields)]
pub enum BlueprintOperationCompleted {
    ForkBlueprintCourse {
        completed: ForkBlueprintCourseCompleted,
    },
    CreateCourseFromBlueprint {
        completed: CreateCourseFromBlueprintCompleted,
    },
    CopyCourseForNewTerm {
        completed: CopyCourseForNewTermCompleted,
    },
    ShiftCourseDates {
        completed: ShiftCourseDatesCompleted,
    },
    ApplyBlueprintUpdate {
        completed: ApplyBlueprintUpdateCompleted,
    },
    CopyAssignmentFromBlueprint {
        completed: CopyAssignmentFromBlueprintCompleted,
    },
}

/// Browser apply intent for one future Store-owned Blueprint operation.
///
/// It carries only browser-safe request facts. A future Store and Server Route decide whether
/// the exact operation identity, reservation, revision, and Receipt suffice for repetition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct BlueprintOperationApplyIntent {
    pub request: BlueprintOperationPreviewRequest,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{BlueprintCourseReference, BlueprintRevision};

    fn fork_request() -> BlueprintOperationPreviewRequest {
        BlueprintOperationPreviewRequest::ForkBlueprintCourse {
            request: ForkBlueprintCoursePreviewRequest {
                source: super::super::BlueprintRevisionReference {
                    reference: BlueprintCourseReference::new(7).expect("blueprint"),
                    revision: BlueprintRevision::new(2).expect("revision"),
                },
                replacements: super::super::QuestionRevisionSubstitutions::default(),
            },
        }
    }

    #[test]
    fn apply_intent_is_strict_snake_case_and_carries_only_a_request() {
        let intent = BlueprintOperationApplyIntent {
            request: fork_request(),
        };
        let wire = serde_json::to_value(&intent).expect("intent serializes");
        assert_eq!(wire["request"]["operation"], "fork_blueprint_course");
        assert!(wire.get("preview").is_none());
        assert!(serde_json::from_value::<BlueprintOperationApplyIntent>(wire.clone()).is_ok());

        let mut forged = wire;
        forged["command"] = serde_json::json!(true);
        assert!(serde_json::from_value::<BlueprintOperationApplyIntent>(forged).is_err());
    }

    #[test]
    fn preview_request_rejects_unknown_operation_and_variant_fields() {
        assert!(
            serde_json::from_value::<BlueprintOperationPreviewRequest>(serde_json::json!({
                "operation": "issue_apply_record",
            }))
            .is_err()
        );

        let mut wire = serde_json::to_value(fork_request()).expect("request serializes");
        wire["authority"] = serde_json::json!("instructor");
        assert!(serde_json::from_value::<BlueprintOperationPreviewRequest>(wire).is_err());
    }
}
