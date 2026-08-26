use objects::Sha256Digest;
use question_model::curriculum_adoption::{
    CurriculumSemanticAssignment, CurriculumSemanticCourse, CurriculumSemanticDigest,
};
use question_model::{
    ActivityTimestamp, AlphaCourseReference, AssignmentDefinitionSourceView, AssignmentId,
    AssignmentReference, CourseReference, CourseScheduleRevision, CourseTerm,
    CurriculumAdoptionIdempotencyKey, CurriculumImportRevision, CurriculumSourceView,
    ObservedAlphaSource, ObservedAssignmentRevision, UserId,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CurriculumAdoptionOperation {
    ForkAlpha,
    InstantiateBlueprint,
    InstantiateAlpha,
    RolloverCourse,
    ShiftCourseTerm,
    FastForwardAssignment,
    CreateSourceDerivedAssignment,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum MemoryCurriculumAdoptionOutcome {
    ForkAlpha {
        source: ObservedAlphaSource,
        alpha: AlphaCourseReference,
    },
    InstantiateBlueprint {
        course: CourseReference,
        assignment: AssignmentReference,
    },
    InstantiateAlpha {
        source: ObservedAlphaSource,
        course: CourseReference,
    },
    RolloverCourse {
        source_course: CourseReference,
        course: CourseReference,
    },
    ShiftCourseTerm {
        course: CourseReference,
        term: CourseTerm,
    },
    FastForwardAssignment {
        course: CourseReference,
        assignment: AssignmentReference,
        import_revision: CurriculumImportRevision,
    },
    CreateSourceDerivedAssignment {
        course: CourseReference,
        assignment: AssignmentReference,
    },
}

/// Minimal private completed-operation evidence; semantic/source contents stay
/// in their own immutable records and never enter a browser receipt.
#[derive(Debug, Clone)]
pub(crate) struct MemoryCurriculumAdoptionReceipt {
    pub(super) operation: CurriculumAdoptionOperation,
    pub(super) actor: UserId,
    pub(super) request_sha256: Sha256Digest,
    pub(super) completed: MemoryCurriculumAdoptionOutcome,
    pub(super) occurred_at: ActivityTimestamp,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct StoredCurriculumBaseline {
    pub(super) payload: CurriculumSemanticAssignment,
    pub(super) digest: CurriculumSemanticDigest,
    pub(super) revision: CurriculumImportRevision,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum StoredCurriculumSource {
    Assignment(AssignmentDefinitionSourceView),
    WholeReusable(CurriculumSourceView),
    Rollover {
        source_course: CourseReference,
        source_schedule_revision: CourseScheduleRevision,
    },
    /// Exact source-assignment witness for one destination assignment created
    /// by a rollover. This stays distinct from the course-level envelope.
    RolloverAssignment {
        source_course: CourseReference,
        source_schedule_revision: CourseScheduleRevision,
        source_assignment: ObservedAssignmentRevision,
    },
}

/// Immutable provenance separate from normalized reusable meaning.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StoredCurriculumEnvelope {
    pub(super) source: StoredCurriculumSource,
    pub(super) actor: UserId,
    pub(super) occurred_at: ActivityTimestamp,
    pub(super) receipt: CurriculumAdoptionIdempotencyKey,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct StoredAssignmentAdoptionEvidence {
    pub(super) baseline: StoredCurriculumBaseline,
    pub(super) envelope: StoredCurriculumEnvelope,
}

/// Exact immutable assignment set created by one whole-course adoption. The
/// receipt-keyed evidence rows remain stable if an import is later advanced or
/// an unrelated assignment is added to the destination course.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct StoredCourseAdoptionRecord {
    pub(super) assignments: Vec<AssignmentId>,
    pub(super) payload: CurriculumSemanticCourse,
    pub(super) digest: CurriculumSemanticDigest,
    pub(super) receipt: CurriculumAdoptionIdempotencyKey,
}

/// Immutable normalized fork meaning plus its separate source/provenance binding.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct StoredAlphaForkLineage {
    pub(super) payload: CurriculumSemanticCourse,
    pub(super) digest: CurriculumSemanticDigest,
    pub(super) source: ObservedAlphaSource,
    pub(super) actor: UserId,
    pub(super) occurred_at: ActivityTimestamp,
    pub(super) receipt: CurriculumAdoptionIdempotencyKey,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StoredCourseImportEnvelope {
    pub(super) source: ObservedAlphaSource,
    pub(super) assignments: Vec<AssignmentId>,
    pub(super) actor: UserId,
    pub(super) occurred_at: ActivityTimestamp,
    pub(super) receipt: CurriculumAdoptionIdempotencyKey,
}
