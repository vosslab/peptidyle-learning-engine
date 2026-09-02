//! Typed browser-safe previews and server-owned commands for exact Blueprint operations.
//!
//! This module is the stable public front door. Focused children keep scalar,
//! source, preview, pin, recovery, receipt, inspection, and repair
//! responsibilities independently readable and evolvable.

mod assignment_source;
mod bounded;
mod course_instance;
mod course_instance_commands;
mod course_instance_receipts;
mod envelope;
mod operations;
mod pins;
mod scalars;
mod server_records;
mod source;

pub use assignment_source::BlueprintAssignmentRevisionReference;
pub use course_instance::*;
pub use course_instance_commands::*;
pub use course_instance_receipts::*;
pub use envelope::{
    BlueprintOperationApplyIntent, BlueprintOperationCompleted, BlueprintOperationPreview,
    BlueprintOperationPreviewRequest,
};
pub use operations::*;
pub use pins::{
    BlueprintQuestionPosition, BlueprintQuestionPositionError, QuestionRevisionSubstitution,
    QuestionRevisionSubstitutions, QuestionRevisionSubstitutionsError,
    ReplacementQuestionRevisionChoices, ReplacementQuestionRevisionChoicesError,
};
pub use scalars::{
    CurriculumImportRevision, CurriculumImportRevisionError, RequestChecksum, RequestRetryToken,
    RequestRetryTokenError,
};
pub use server_records::{
    ApplyBlueprintUpdateApplyRecord, AssignmentImportRepairApplyRecord,
    CopyAssignmentFromBlueprintApplyRecord, CopyCourseForNewTermApplyRecord,
    CreateCourseFromBlueprintApplyRecord, ForkBlueprintCourseApplyRecord, RequestRetryBinding,
    ShiftCourseDatesApplyRecord,
};
pub use source::BlueprintRevisionReference;
