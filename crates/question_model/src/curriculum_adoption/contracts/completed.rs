//! Operation-specific completed adoption results and immutable receipt bindings.

use serde::{Deserialize, Serialize};

use super::{CurriculumAdoptionIdempotencyKey, CurriculumImportRevision, ObservedAlphaSource};
use crate::{AlphaCourseReference, AssignmentReference, CourseReference, CourseTerm};

/// Browser-safe immutable receipt binding for one completed operation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CurriculumAdoptionReceiptBinding {
    /// Client key bound to the stored request digest and completed receipt.
    pub idempotency_key: CurriculumAdoptionIdempotencyKey,
}

/// Whether a completed write was newly performed or loaded from a matching receipt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CurriculumReplayStatus {
    /// The Store performed and persisted the operation now.
    Applied,
    /// The Store returned the matching completed receipt without another mutation.
    Replayed,
}

/// Completed Alpha fork result. The source and resulting Alpha are explicit and distinct.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ForkAlphaCompleted {
    /// Source Alpha selected for the immutable fork lineage.
    pub source: ObservedAlphaSource,
    /// New independently editable Alpha route reference.
    pub alpha: AlphaCourseReference,
    /// Whether the fork was applied now or loaded from its receipt.
    pub replay: CurriculumReplayStatus,
    /// Immutable receipt binding for this completed fork.
    pub receipt: CurriculumAdoptionReceiptBinding,
}

/// Completed Blueprint-instantiation result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BlueprintInstantiationCompleted {
    /// Existing teaching-course destination.
    pub course: CourseReference,
    /// New ordinary assignment definition.
    pub assignment: AssignmentReference,
    /// Whether the write was applied now or replayed.
    pub replay: CurriculumReplayStatus,
    /// Immutable receipt binding for this completed instantiation.
    pub receipt: CurriculumAdoptionReceiptBinding,
}

/// Completed Alpha-instantiation result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AlphaInstantiationCompleted {
    /// Source Alpha used for the new teaching course.
    pub source: ObservedAlphaSource,
    /// New ordinary teaching-course destination.
    pub course: CourseReference,
    /// Whether the write was applied now or replayed.
    pub replay: CurriculumReplayStatus,
    /// Immutable receipt binding for this completed instantiation.
    pub receipt: CurriculumAdoptionReceiptBinding,
}

/// Completed course-rollover result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CourseRolloverCompleted {
    /// Source ordinary teaching course.
    pub source_course: CourseReference,
    /// New ordinary teaching-course destination.
    pub course: CourseReference,
    /// Whether the write was applied now or replayed.
    pub replay: CurriculumReplayStatus,
    /// Immutable receipt binding for this completed rollover.
    pub receipt: CurriculumAdoptionReceiptBinding,
}

/// Completed whole-course term-shift result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CourseTermShiftCompleted {
    /// Existing ordinary teaching course updated atomically.
    pub course: CourseReference,
    /// Target term now owned by the course.
    pub term: CourseTerm,
    /// Whether the write was applied now or replayed.
    pub replay: CurriculumReplayStatus,
    /// Immutable receipt binding for this completed term shift.
    pub receipt: CurriculumAdoptionReceiptBinding,
}

/// Completed eligible assignment fast-forward result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AssignmentFastForwardCompleted {
    /// Existing course that owns the updated import.
    pub course: CourseReference,
    /// Existing assignment that received new reusable meaning.
    pub assignment: AssignmentReference,
    /// Advanced import revision after the new baseline/envelope was stored.
    pub import_revision: CurriculumImportRevision,
    /// Whether the write was applied now or replayed.
    pub replay: CurriculumReplayStatus,
    /// Immutable receipt binding for this completed fast-forward.
    pub receipt: CurriculumAdoptionReceiptBinding,
}

/// Completed source-derived assignment creation result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceDerivedAssignmentCompleted {
    /// Existing teaching-course destination.
    pub course: CourseReference,
    /// New independent ordinary assignment definition.
    pub assignment: AssignmentReference,
    /// Whether the result was applied now or replayed from durable evidence.
    pub replay: CurriculumReplayStatus,
    /// Immutable receipt binding for this completed source-derived creation.
    pub receipt: CurriculumAdoptionReceiptBinding,
}
