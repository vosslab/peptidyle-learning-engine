//! Closed browser envelopes for the CurriculumAdoptionStore lifecycle.

use serde::{Deserialize, Serialize};

use super::{
    AdoptBlueprintAssignmentCompleted, AdoptBlueprintAssignmentPreviewRequest,
    AdoptBlueprintAssignmentPreviewView, ControlledUpdateBlueprintAssignmentCompleted,
    ControlledUpdateBlueprintAssignmentPreview, ControlledUpdateBlueprintAssignmentPreviewRequest,
    CreateSelectedBlueprintAssignmentCompleted, CreateSelectedBlueprintAssignmentPreview,
    CreateSelectedBlueprintAssignmentPreviewRequest, CurriculumAdoptionIdempotencyKey,
    ForkBlueprintCourseCompleted, ForkBlueprintCoursePreviewRequest,
    ForkBlueprintCoursePreviewView, InstantiateBlueprintCourseCompleted,
    InstantiateBlueprintCoursePreviewRequest, InstantiateBlueprintCoursePreviewView,
    RolloverCourseInstanceCompleted, RolloverCourseInstancePreview,
    RolloverCourseInstancePreviewRequest, ShiftCourseInstanceTermCompleted,
    ShiftCourseInstanceTermPreview, ShiftCourseInstanceTermPreviewRequest,
};

/// One closed, answer-free browser request for a current curriculum adoption preview.
///
/// The operation tag selects exactly one Store-owned operation. Each request is only a locator and
/// correction intent; the Store resolves current authorization and mutable facts before previewing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "operation", rename_all = "snake_case", deny_unknown_fields)]
pub enum CurriculumAdoptionPreviewRequest {
    ForkBlueprintCourse {
        request: ForkBlueprintCoursePreviewRequest,
    },
    AdoptBlueprintAssignment {
        request: AdoptBlueprintAssignmentPreviewRequest,
    },
    InstantiateBlueprintCourse {
        request: InstantiateBlueprintCoursePreviewRequest,
    },
    RolloverCourseInstance {
        request: RolloverCourseInstancePreviewRequest,
    },
    ShiftCourseInstanceTerm {
        request: ShiftCourseInstanceTermPreviewRequest,
    },
    ControlledUpdateBlueprintAssignment {
        request: ControlledUpdateBlueprintAssignmentPreviewRequest,
    },
    CreateSelectedBlueprintAssignment {
        request: CreateSelectedBlueprintAssignmentPreviewRequest,
    },
}

/// One closed, answer-free Store preview for a curriculum adoption operation.
///
/// A preview explains the facts resolved at preview time. It grants no apply authority and can be
/// stale as soon as it is returned.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "operation", rename_all = "snake_case", deny_unknown_fields)]
pub enum CurriculumAdoptionPreview {
    ForkBlueprintCourse {
        preview: ForkBlueprintCoursePreviewView,
    },
    AdoptBlueprintAssignment {
        preview: AdoptBlueprintAssignmentPreviewView,
    },
    InstantiateBlueprintCourse {
        preview: InstantiateBlueprintCoursePreviewView,
    },
    RolloverCourseInstance {
        preview: RolloverCourseInstancePreview,
    },
    ShiftCourseInstanceTerm {
        preview: ShiftCourseInstanceTermPreview,
    },
    ControlledUpdateBlueprintAssignment {
        preview: ControlledUpdateBlueprintAssignmentPreview,
    },
    CreateSelectedBlueprintAssignment {
        preview: CreateSelectedBlueprintAssignmentPreview,
    },
}

/// One closed, answer-free completion emitted from an atomic curriculum adoption apply.
///
/// The Store owns issuance and consumption of the non-Serde apply record that produces this
/// completion. The browser receives the result, never the record or immutable receipt evidence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "operation", rename_all = "snake_case", deny_unknown_fields)]
pub enum CurriculumAdoptionCompleted {
    ForkBlueprintCourse {
        completed: ForkBlueprintCourseCompleted,
    },
    AdoptBlueprintAssignment {
        completed: AdoptBlueprintAssignmentCompleted,
    },
    InstantiateBlueprintCourse {
        completed: InstantiateBlueprintCourseCompleted,
    },
    RolloverCourseInstance {
        completed: RolloverCourseInstanceCompleted,
    },
    ShiftCourseInstanceTerm {
        completed: ShiftCourseInstanceTermCompleted,
    },
    ControlledUpdateBlueprintAssignment {
        completed: ControlledUpdateBlueprintAssignmentCompleted,
    },
    CreateSelectedBlueprintAssignment {
        completed: CreateSelectedBlueprintAssignmentCompleted,
    },
}

/// Browser apply intent for one atomic Store-owned curriculum adoption operation.
///
/// The request repeats only browser-safe intent from preview. The idempotency key binds retries to
/// the Store's canonical request digest; previews, commands, records, and receipts stay
/// server-held.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct CurriculumAdoptionApplyIntent {
    pub request: CurriculumAdoptionPreviewRequest,
    pub idempotency_key: CurriculumAdoptionIdempotencyKey,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{BlueprintReference, BlueprintRevision};

    fn fork_request() -> CurriculumAdoptionPreviewRequest {
        CurriculumAdoptionPreviewRequest::ForkBlueprintCourse {
            request: ForkBlueprintCoursePreviewRequest {
                source: super::super::ObservedBlueprintSource {
                    reference: BlueprintReference::new(7).expect("blueprint"),
                    revision: BlueprintRevision::new(2).expect("revision"),
                },
                replacements: super::super::CurriculumPinReplacements::default(),
            },
        }
    }

    #[test]
    fn apply_intent_is_strict_snake_case_and_carries_only_a_request() {
        let intent = CurriculumAdoptionApplyIntent {
            request: fork_request(),
            idempotency_key: CurriculumAdoptionIdempotencyKey::parse("fork-apply")
                .expect("idempotency key"),
        };
        let wire = serde_json::to_value(&intent).expect("intent serializes");
        assert_eq!(wire["request"]["operation"], "fork_blueprint_course");
        assert!(wire.get("preview").is_none());
        assert!(serde_json::from_value::<CurriculumAdoptionApplyIntent>(wire.clone()).is_ok());

        let mut forged = wire;
        forged["command"] = serde_json::json!(true);
        assert!(serde_json::from_value::<CurriculumAdoptionApplyIntent>(forged).is_err());
    }

    #[test]
    fn preview_request_rejects_unknown_operation_and_variant_fields() {
        assert!(
            serde_json::from_value::<CurriculumAdoptionPreviewRequest>(serde_json::json!({
                "operation": "issue_apply_record",
            }))
            .is_err()
        );

        let mut wire = serde_json::to_value(fork_request()).expect("request serializes");
        wire["authority"] = serde_json::json!("instructor");
        assert!(serde_json::from_value::<CurriculumAdoptionPreviewRequest>(wire).is_err());
    }
}
