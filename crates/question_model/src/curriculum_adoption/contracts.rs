//! Typed browser-safe previews and server-owned commands for B2 adoption.
//!
//! This module is the stable public front door. Focused children keep scalar,
//! source, preview, pin, recovery, receipt, inspection, and reconciliation
//! responsibilities independently readable and evolvable.

mod assignment_source;
mod bounded;
mod commands;
mod completed;
mod inspection;
mod pins;
mod previews;
mod reconciliation;
mod recovery;
mod scalars;
mod source;

pub use assignment_source::{
    AssignmentDefinitionSourceView, ObservedAlphaAssignmentSource,
    ObservedAlphaAssignmentSourceError,
};
pub use commands::{
    AlphaInstantiationCommand, AssignmentFastForwardCommand, BlueprintInstantiationCommand,
    CourseRolloverCommand, CourseTermShiftCommand, CreateSourceDerivedAssignmentCommand,
    CurriculumAdoptionCommandError, ForkAlphaCommand,
};
pub use completed::{
    AlphaInstantiationCompleted, AssignmentFastForwardCompleted, BlueprintInstantiationCompleted,
    CourseRolloverCompleted, CourseTermShiftCompleted, CurriculumAdoptionReceiptBinding,
    CurriculumReplayStatus, ForkAlphaCompleted, SourceDerivedAssignmentCompleted,
};
pub use inspection::{
    CurriculumAssignmentImportSourceView, CurriculumCourseImportOriginView,
    CurriculumCourseImportView, CurriculumCourseImportViewError, CurriculumImportView,
    RolloverAssignmentSourceView, RolloverAssignmentSourceViewError,
    RolloverCourseImportOriginView,
};
pub use pins::{
    CurriculumPinPosition, CurriculumPinPositionError, CurriculumPinReplacement,
    CurriculumPinReplacements, CurriculumPinReplacementsError, ReplacementQuestionChoices,
    ReplacementQuestionChoicesError,
};
pub use previews::{
    AlphaInstantiationPreviewRequest, AlphaInstantiationPreviewView,
    AssignmentFastForwardPreviewRequest, BlueprintInstantiationPreviewRequest,
    BlueprintInstantiationPreviewView, CourseRolloverPreviewRequest, CourseRolloverPreviewView,
    CourseTermShiftIneligibility, CourseTermShiftPreviewOutcome, CourseTermShiftPreviewRequest,
    CourseTermShiftPreviewView, CourseTermShiftRecoveryAction, CurriculumAssignmentView,
    CurriculumScheduleCorrection, ForkAlphaPreviewRequest, ForkAlphaPreviewView,
    PreparedCurriculumAssignmentView, PreparedCurriculumCourseView,
    SourceDerivedAssignmentPreviewRequest, SourceDerivedAssignmentPreviewView,
};
pub use reconciliation::{
    CurriculumAdoptionReconciliationResult, CurriculumAdoptionRepairedProjection,
    CurriculumAdoptionRepairedProjections, CurriculumAdoptionRepairedProjectionsError,
    ReconcileCurriculumAdoptionCommand,
};
pub use recovery::{
    AssignmentFastForwardDecision, AssignmentFastForwardPreviewView,
    PreservedAssignmentRecoveryAction, UnavailablePinRecoveryAction,
};
pub use scalars::{
    CurriculumAdoptionIdempotencyKey, CurriculumAdoptionIdempotencyKeyError,
    CurriculumAdoptionTitle, CurriculumAdoptionTitleError, CurriculumImportRevision,
    CurriculumImportRevisionError,
};
pub use source::{
    CourseScheduleWitness, CourseScheduleWitnessError, CurriculumSourceView, ObservedAlphaSource,
    ObservedAssignmentRevision, ObservedBlueprintSource,
};
