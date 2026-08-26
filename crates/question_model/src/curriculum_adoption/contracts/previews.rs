//! Closed, answer-free B2 preview requests and outcomes.

use serde::{Deserialize, Serialize};

use super::{
    AssignmentDefinitionSourceView, CourseScheduleWitness, CurriculumAdoptionTitle,
    CurriculumImportRevision, CurriculumPinReplacements, ObservedAlphaSource,
    ObservedAssignmentRevision, ObservedBlueprintSource, UnavailablePinRecoveryAction,
};
use crate::{
    AssignmentReference, AssignmentRevision, AssignmentTeachingSettingsFailureCode,
    AssignmentTeachingSettingsLocalError, AssignmentTeachingSettingsValidationFailure,
    CourseReference, CourseTerm, ResolvedRelativeAssignmentSchedule,
};

/// Field-specific schedule correction derived from existing local-time domain errors.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CurriculumScheduleCorrection {
    /// Existing bounded, answer-free correction details.
    pub correction: AssignmentTeachingSettingsValidationFailure,
}

impl From<AssignmentTeachingSettingsLocalError> for CurriculumScheduleCorrection {
    fn from(value: AssignmentTeachingSettingsLocalError) -> Self {
        Self {
            correction: AssignmentTeachingSettingsValidationFailure {
                error: AssignmentTeachingSettingsFailureCode::AssignmentTeachingSettingsInvalid,
                field: value.field(),
                reason: value.reason(),
                message: value.to_string(),
            },
        }
    }
}

/// Preview request for an independent Alpha fork. It has no teaching-course fields.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ForkAlphaPreviewRequest {
    /// Public source observed under approved-Instructor authority.
    pub source: ObservedAlphaSource,
    /// Explicit public-question substitutions accumulated through preview corrections.
    pub replacements: CurriculumPinReplacements,
}

/// Preview request for one Blueprint instantiation into an existing teaching course.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BlueprintInstantiationPreviewRequest {
    /// Owner-scoped source and exact revision.
    pub source: ObservedBlueprintSource,
    /// Existing teaching-course destination.
    pub course: CourseReference,
    /// Target term used to resolve reusable schedule defaults.
    pub target_term: CourseTerm,
    /// Explicit public-question substitutions accumulated through preview corrections.
    pub replacements: CurriculumPinReplacements,
}

/// Preview request for one Alpha instantiation into a new teaching course.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AlphaInstantiationPreviewRequest {
    /// Public source and exact revision.
    pub source: ObservedAlphaSource,
    /// Validated title proposed for the new ordinary teaching course.
    pub title: CurriculumAdoptionTitle,
    /// Explicit target term for every new course schedule.
    pub target_term: CourseTerm,
    /// Explicit public-question substitutions accumulated through preview corrections.
    pub replacements: CurriculumPinReplacements,
}

/// Preview request for an ordinary teaching-course rollover.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CourseRolloverPreviewRequest {
    /// Source-course schedule and assignment revision witness.
    pub witness: CourseScheduleWitness,
    /// Validated title proposed for the new ordinary teaching course.
    pub title: CurriculumAdoptionTitle,
    /// Explicit target term for the new course.
    pub target_term: CourseTerm,
    /// Explicit public-question substitutions accumulated through preview corrections.
    pub replacements: CurriculumPinReplacements,
}

/// Preview request for an atomic whole-course term shift.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CourseTermShiftPreviewRequest {
    /// Current course and assignment revisions that apply must repeat.
    pub witness: CourseScheduleWitness,
    /// Replacement term for the existing unissued course.
    pub target_term: CourseTerm,
}

/// Preview request for an imported assignment fast-forward decision.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AssignmentFastForwardPreviewRequest {
    /// Destination course that owns the import.
    pub course: CourseReference,
    /// Destination assignment revision observed by the Instructor.
    pub assignment: ObservedAssignmentRevision,
    /// Durable import baseline revision observed by the Instructor.
    pub import_revision: CurriculumImportRevision,
    /// Re-readable source revision selected for comparison.
    pub source: AssignmentDefinitionSourceView,
}

/// Preview request for a separate source-derived assignment after divergence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceDerivedAssignmentPreviewRequest {
    /// Existing teaching-course destination.
    pub course: CourseReference,
    /// Source definition selected for the new independent draft.
    pub source: AssignmentDefinitionSourceView,
    /// Explicit public-question substitutions accumulated through preview corrections.
    pub replacements: CurriculumPinReplacements,
}

/// One answer-free assignment schedule row for an already existing destination assignment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CurriculumAssignmentView {
    /// Assignment route locator scoped by the surrounding course.
    pub reference: AssignmentReference,
    /// Instructor-visible assignment title.
    pub title: CurriculumAdoptionTitle,
    /// Exact assignment-definition revision shown to the Instructor.
    pub revision: AssignmentRevision,
    /// Server-resolved target-term schedule projection.
    pub schedule: ResolvedRelativeAssignmentSchedule,
}

/// One answer-free row prepared before a destination assignment has a route reference.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PreparedCurriculumAssignmentView {
    /// Instructor-visible title that will be copied into the new assignment.
    pub title: CurriculumAdoptionTitle,
    /// Server-resolved target-term schedule projection.
    pub schedule: ResolvedRelativeAssignmentSchedule,
}

/// One answer-free course preview prepared before a destination course has a route reference.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PreparedCurriculumCourseView {
    /// Instructor-visible course title that will be copied into the new course.
    pub title: CurriculumAdoptionTitle,
    /// New ordinary assignment definitions in source order.
    pub assignments: Vec<PreparedCurriculumAssignmentView>,
}

/// Fork preview naming the source and the resulting independent Alpha exactly.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ForkAlphaPreviewView {
    /// Public Alpha source selected for the fork.
    pub source: ObservedAlphaSource,
    /// Resulting independent Alpha title copied exactly from the source.
    pub resulting_alpha_title: CurriculumAdoptionTitle,
    /// Validated substitutions already applied to the prepared semantic snapshot.
    pub replacements: CurriculumPinReplacements,
    /// First exact source pin requiring an explicit authorized replacement.
    pub pin_correction: Option<UnavailablePinRecoveryAction>,
}

/// Blueprint-instantiation preview for one existing teaching course.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BlueprintInstantiationPreviewView {
    /// Selected source and observed revision.
    pub source: ObservedBlueprintSource,
    /// Existing teaching-course destination.
    pub course: CourseReference,
    /// Target term that resolved the prepared schedule.
    pub target_term: CourseTerm,
    /// Current course/assignment revisions required by apply.
    pub witness: CourseScheduleWitness,
    /// Prepared assignment before it has a destination reference.
    pub assignment: PreparedCurriculumAssignmentView,
    /// Validated substitutions already applied to the prepared semantic snapshot.
    pub replacements: CurriculumPinReplacements,
    /// Field-specific schedule corrections, if any.
    pub corrections: Vec<CurriculumScheduleCorrection>,
    /// First exact source pin requiring an explicit authorized replacement.
    pub pin_correction: Option<UnavailablePinRecoveryAction>,
}

/// Alpha-instantiation preview for one new teaching course.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AlphaInstantiationPreviewView {
    /// Selected public source and observed revision.
    pub source: ObservedAlphaSource,
    /// Explicit target term.
    pub target_term: CourseTerm,
    /// Prepared course before it has a destination reference.
    pub course: PreparedCurriculumCourseView,
    /// Validated substitutions already applied to the prepared semantic snapshot.
    pub replacements: CurriculumPinReplacements,
    /// Field-specific schedule corrections, if any.
    pub corrections: Vec<CurriculumScheduleCorrection>,
    /// First exact source pin requiring an explicit authorized replacement.
    pub pin_correction: Option<UnavailablePinRecoveryAction>,
}

/// Rollover preview for a new ordinary course.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CourseRolloverPreviewView {
    /// Source course and all revisions that apply must repeat.
    pub witness: CourseScheduleWitness,
    /// Explicit target term.
    pub target_term: CourseTerm,
    /// Prepared new course before it has a destination reference.
    pub course: PreparedCurriculumCourseView,
    /// Validated substitutions already applied to the prepared semantic snapshot.
    pub replacements: CurriculumPinReplacements,
    /// Field-specific schedule corrections, if any.
    pub corrections: Vec<CurriculumScheduleCorrection>,
    /// First exact source pin requiring an explicit authorized replacement.
    pub pin_correction: Option<UnavailablePinRecoveryAction>,
}

/// Whole-course term-shift preview for an eligible existing destination course.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CourseTermShiftPreviewView {
    /// Course and assignment revisions required by apply.
    pub witness: CourseScheduleWitness,
    /// Target term selected for the existing course.
    pub target_term: CourseTerm,
    /// Existing assignments with resolved target-term schedules.
    pub assignments: Vec<CurriculumAssignmentView>,
    /// Field-specific schedule corrections, if any.
    pub corrections: Vec<CurriculumScheduleCorrection>,
}

/// Typed reason an existing course has no whole-course term-shift action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CourseTermShiftIneligibility {
    /// Learner work has issued and retains its original course-term context.
    IssuedWork,
}

/// Explicit next action when an existing course cannot shift its term in place.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CourseTermShiftRecoveryAction {
    /// Create a new ordinary course through the revision-bound course-rollover flow.
    RolloverCourse,
}

/// Closed result of a whole-course term-shift preview.
///
/// Only the eligible branch carries the exact schedule witness accepted by apply.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum CourseTermShiftPreviewOutcome {
    /// The course has no issued learner work and may proceed with the returned preview.
    Eligible {
        /// Exact preview, including the revision witness that apply must repeat.
        preview: CourseTermShiftPreviewView,
    },
    /// The course retains its original term context because learner work already issued.
    Ineligible {
        /// Existing course selected for this preview.
        course: CourseReference,
        /// Typed non-actionable reason returned by the Store.
        reason: CourseTermShiftIneligibility,
        /// Explicit follow-on workflow that preserves the issued course unchanged.
        recovery: CourseTermShiftRecoveryAction,
    },
}

/// Preview for a new independent source-derived assignment before it has a reference.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceDerivedAssignmentPreviewView {
    /// Existing teaching-course destination.
    pub course: CourseReference,
    /// Selected re-readable source.
    pub source: AssignmentDefinitionSourceView,
    /// Current course and assignment revisions required by apply.
    pub witness: CourseScheduleWitness,
    /// Prepared independent assignment before it has a destination reference.
    pub assignment: PreparedCurriculumAssignmentView,
    /// Validated substitutions already applied to the prepared semantic snapshot.
    pub replacements: CurriculumPinReplacements,
    /// Field-specific schedule corrections, if any.
    pub corrections: Vec<CurriculumScheduleCorrection>,
    /// First exact source pin requiring an explicit authorized replacement.
    pub pin_correction: Option<UnavailablePinRecoveryAction>,
}
