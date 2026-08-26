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

pub(super) use crate::curriculum_adoption::{CurriculumAdoptionOperation, request_digest};
pub(super) use authorization::{
    advance_course_schedule_revision, assignment_has_run, authorized_actor, course_has_any_run,
    course_witness, require_course_instructor, require_exact_witness, resolve_course,
};
pub(super) use course_structure::{
    course_assignment_ids, current_with_projected_teaching_schedule, rollover_input,
};
pub(super) use pin_replacements::{
    apply_pin_replacements, assignment_source_snapshot_with_replacements, pin_correction,
    replacement_question_choices, source_snapshot_with_replacements, unavailable_destination_pin,
    validate_destination_pins,
};
pub(super) use receipt_evidence::{
    completed_outcome_assignment_ids, ensure_completed_outcome_binding,
    ensure_completed_outcome_contains_assignment, matching_receipt,
    refuse_detached_whole_course_receipt, store_receipt,
    validate_current_assignment_import_evidence, validate_whole_course_adoption,
};
pub(super) use state::{CurriculumAdoptionState, MemoryCurriculumAdoptionOutcome};
pub(super) use state::{
    MemoryCurriculumAdoptionReceipt, RolloverAssignmentProvenance, StoredAlphaForkLineage,
    StoredAssignmentAdoptionEvidence, StoredAssignmentImport, StoredAssignmentImportProvenance,
    StoredAssignmentImportSource, StoredCurriculumBaseline, StoredWholeCourseAdoption,
    StoredWholeCourseOrigin,
};
