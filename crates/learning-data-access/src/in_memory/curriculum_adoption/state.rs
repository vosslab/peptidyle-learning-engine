use question_model::curriculum_adoption::{
    CurriculumSemanticAssignment, CurriculumSemanticCourse, CurriculumSemanticDigest,
};
use question_model::{
    ActivityTimestamp, AlphaCourseReference, AssignmentDefinitionSourceView, AssignmentId,
    AssignmentReference, CourseReference, CourseScheduleRevision, CourseTerm,
    CurriculumAdoptionIdempotencyKey, CurriculumImportRevision, ObservedAlphaSource,
    ObservedAssignmentRevision, TenantId, UserId,
};
use std::collections::BTreeMap;

use crate::curriculum_adoption::{CurriculumAdoptionOperation, CurriculumAdoptionRequestDigest};

/// Private state owned by the curriculum-adoption capability.
///
/// `State` retains the single outer lock and operation-level rollback snapshot;
/// grouping these immutable import records only gives this capability one
/// coherent ownership boundary.
#[derive(Debug, Default, Clone, PartialEq)]
pub(in crate::in_memory) struct CurriculumAdoptionState {
    /// Current, repairable projection of the newest immutable import evidence.
    pub(super) import_records: BTreeMap<(TenantId, AssignmentId), StoredAssignmentImport>,
    pub(super) assignment_evidence: BTreeMap<
        (TenantId, CurriculumAdoptionIdempotencyKey, AssignmentId),
        StoredAssignmentAdoptionEvidence,
    >,
    /// Immutable aggregate evidence for courses created from Alpha or rollover.
    pub(super) whole_course_adoptions:
        BTreeMap<(TenantId, question_model::CourseId), StoredWholeCourseAdoption>,
    pub(super) alpha_fork_lineage: BTreeMap<AlphaCourseReference, StoredAlphaForkLineage>,
    pub(super) receipts:
        BTreeMap<(TenantId, CurriculumAdoptionIdempotencyKey), MemoryCurriculumAdoptionReceipt>,
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
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct MemoryCurriculumAdoptionReceipt {
    pub(super) operation: CurriculumAdoptionOperation,
    pub(super) actor: UserId,
    pub(super) request_sha256: CurriculumAdoptionRequestDigest,
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
pub(crate) enum StoredAssignmentImportSource {
    Reusable(AssignmentDefinitionSourceView),
    Rollover(RolloverAssignmentProvenance),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RolloverAssignmentProvenance {
    pub(super) source_course: CourseReference,
    pub(super) source_schedule_revision: CourseScheduleRevision,
    pub(super) source_assignment: ObservedAssignmentRevision,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum StoredWholeCourseOrigin {
    Alpha {
        source: ObservedAlphaSource,
    },
    Rollover {
        source_course: CourseReference,
        source_schedule_revision: CourseScheduleRevision,
    },
}

/// Immutable provenance separate from normalized reusable meaning.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StoredAssignmentImportProvenance {
    pub(super) source: StoredAssignmentImportSource,
    pub(super) actor: UserId,
    pub(super) occurred_at: ActivityTimestamp,
    pub(super) receipt: CurriculumAdoptionIdempotencyKey,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct StoredAssignmentAdoptionEvidence {
    pub(super) baseline: StoredCurriculumBaseline,
    pub(super) provenance: StoredAssignmentImportProvenance,
}

/// Current derived import projection. Immutable receipt-keyed evidence is the
/// sole authority and can rebuild this record during reconciliation.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct StoredAssignmentImport {
    pub(super) baseline: StoredCurriculumBaseline,
    pub(super) provenance: StoredAssignmentImportProvenance,
}

/// Exact immutable assignment set created by one whole-course adoption. The
/// receipt-keyed evidence rows remain stable if an import is later advanced or
/// an unrelated assignment is added to the destination course.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct StoredWholeCourseAdoption {
    pub(super) destination_assignments: Vec<AssignmentId>,
    pub(super) payload: CurriculumSemanticCourse,
    pub(super) digest: CurriculumSemanticDigest,
    pub(super) origin: StoredWholeCourseOrigin,
    pub(super) actor: UserId,
    pub(super) occurred_at: ActivityTimestamp,
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
