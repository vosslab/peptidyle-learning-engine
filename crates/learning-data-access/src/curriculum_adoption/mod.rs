//! Backend-neutral semantic planning for B2 curriculum adoption.

mod inspection;
mod pins;
mod planner;
mod preview;
mod request_digest;
#[cfg(any(test, feature = "test-support"))]
mod rollover;
mod semantic_snapshot;

pub(crate) use inspection::{
    CurrentTeachingImportInput, CurriculumImportInspectionInput, ObservedSemanticEnvelope,
    project_current_teaching_import, project_curriculum_import_inspection,
    validate_semantic_evidence,
};
pub(crate) use pins::PositionedPin;
pub(crate) use pins::{ResolvedPinReplacement, substitute_resolved_pins};
#[cfg(any(test, feature = "test-support"))]
pub(crate) use pins::{first_unavailable_pin, positioned_pins};
#[cfg(any(test, feature = "test-support"))]
pub(crate) use planner::{
    AssignmentMaterializationEntry, plan_assignment_entries, same_source_locator,
};
pub(crate) use planner::{AssignmentMaterializationPlan, plan_assignment_materialization};
pub(crate) use planner::{FastForwardProjectionInput, project_fast_forward_decision};
pub(crate) use preview::{preview_assignment, preview_course};
pub(crate) use request_digest::{
    CanonicalCurriculumAdoptionIntentV1, CurriculumAdoptionOperation,
    CurriculumAdoptionRequestDigest, reconciliation_target_digest,
};
#[cfg(any(test, feature = "test-support"))]
pub(crate) use rollover::RolloverInput;
#[cfg(any(test, feature = "test-support"))]
pub(crate) use semantic_snapshot::{
    SemanticAssignmentEntryInputV1, SemanticAssignmentInputV1, SemanticModuleInputV1,
    SemanticPlannerError, SemanticPoolInputV1,
};
pub(crate) use semantic_snapshot::{
    SemanticPayloadInputV1, TeachingAssignmentInputV1, normalize_payload,
    normalize_teaching_assignment, semantic_payload_input,
};
