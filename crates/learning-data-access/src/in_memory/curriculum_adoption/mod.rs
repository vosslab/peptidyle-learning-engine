//! Deterministic one-lock Memory parity for B2 curriculum adoption.

mod authorization;
mod course_structure;
mod destination;
mod operations;
mod pin_replacements;
mod receipt_evidence;
mod state;
#[cfg(test)]
mod tests;

pub(super) use crate::curriculum_adoption::{
    CurriculumAdoptionOperation, CurriculumAdoptionRequestDigest,
};
pub(super) use authorization::{
    advance_course_schedule_revision, assignment_has_run, authorized_actor, course_has_any_run,
    course_instance_blueprint_application, course_witness, require_course_instructor,
    require_exact_witness, resolve_course,
};
pub(super) use course_structure::{
    course_assignment_ids, current_with_projected_teaching_schedule, rollover_input,
};
pub(super) use pin_replacements::{
    assignment_source_snapshot_with_replacements, pin_correction,
    source_snapshot_with_replacements, unavailable_destination_pin, validate_destination_pins,
};
pub(super) use receipt_evidence::{
    completed_response, lookup_replay_or_conflict, rebuild_current_projection,
    resolve_reconciliation_target, store_completed_receipt, validate_receipt_evidence,
};
pub(super) use state::{
    AssignmentAdoptionEvidenceDetail, MemoryCurriculumAdoptionEvidence,
    MemoryCurriculumAdoptionReceipt, StoredAssignmentAdoptionEvidence, StoredAssignmentImport,
    StoredWholeCourseAdoption,
};
pub(super) use state::{CurriculumAdoptionState, MemoryCurriculumAdoptionOutcome};
