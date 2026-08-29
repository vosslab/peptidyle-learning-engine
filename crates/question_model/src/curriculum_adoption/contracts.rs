//! Typed browser-safe previews and server-owned commands for B2 adoption.
//!
//! This module is the stable public front door. Focused children keep scalar,
//! source, preview, pin, recovery, receipt, inspection, and reconciliation
//! responsibilities independently readable and evolvable.

mod adoption;
mod assignment_source;
mod bounded;
mod course_instance;
mod course_instance_commands;
mod course_instance_receipts;
mod envelope;
mod pins;
mod scalars;
mod server_records;
mod source;

#[cfg(test)]
mod course_instance_record_tests;

pub use adoption::*;
pub use assignment_source::{AssignmentDefinitionSourceView, AssignmentDefinitionSourceViewError};
pub use course_instance::*;
pub use course_instance_commands::*;
pub use course_instance_receipts::*;
pub use envelope::{
    CurriculumAdoptionApplyIntent, CurriculumAdoptionCompleted, CurriculumAdoptionPreview,
    CurriculumAdoptionPreviewRequest,
};
pub use pins::{
    CurriculumPinPosition, CurriculumPinPositionError, CurriculumPinReplacement,
    CurriculumPinReplacements, CurriculumPinReplacementsError, ReplacementQuestionChoices,
    ReplacementQuestionChoicesError,
};
pub use scalars::{
    CurriculumAdoptionIdempotencyKey, CurriculumAdoptionIdempotencyKeyError,
    CurriculumImportRevision, CurriculumImportRevisionError,
};
pub use server_records::{
    AdoptBlueprintAssignmentApplyRecord, ControlledUpdateBlueprintAssignmentApplyRecord,
    CreateSelectedBlueprintAssignmentApplyRecord, ForkBlueprintCourseApplyRecord,
    InstantiateBlueprintCourseApplyRecord, ReconcileCourseInstanceAdoptionApplyRecord,
    RolloverCourseInstanceApplyRecord, ShiftCourseInstanceTermApplyRecord,
};
pub use source::ObservedBlueprintSource;
