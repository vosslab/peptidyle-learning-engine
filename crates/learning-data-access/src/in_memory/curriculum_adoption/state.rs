use std::collections::BTreeMap;

use question_model::{
    ActivityTimestamp, AssignmentDefinitionSourceView, AssignmentId, AssignmentReference,
    CourseInstanceReceiptTarget, CourseInstanceWitness, CourseReference,
    CurriculumAdoptionCompleted, CurriculumAdoptionIdempotencyKey, CurriculumImportRevision,
    CurriculumReplayStatus, ForkBlueprintCourseCompleted, ObservedBlueprintSource, UserId,
};

use crate::curriculum_adoption::{CurriculumAdoptionOperation, CurriculumAdoptionRequestDigest};

/// Private immutable and derived state for one Memory curriculum-adoption capability.
#[derive(Debug, Default, Clone, PartialEq)]
pub(in crate::in_memory) struct CurriculumAdoptionState {
    pub(super) receipts:
        BTreeMap<(UserId, CurriculumAdoptionIdempotencyKey), MemoryCurriculumAdoptionReceipt>,
    pub(super) assignment_evidence:
        BTreeMap<(AssignmentId, CurriculumImportRevision), StoredAssignmentAdoptionEvidence>,
    pub(super) import_records: BTreeMap<AssignmentId, StoredAssignmentImport>,
    pub(super) whole_course_adoptions:
        BTreeMap<question_model::CourseId, StoredWholeCourseAdoption>,
    pub(super) receipt_targets:
        BTreeMap<(UserId, CurriculumAdoptionIdempotencyKey), CourseInstanceReceiptTarget>,
}

/// Safe locators sufficient to project an answer-free completed operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum MemoryCurriculumAdoptionOutcome {
    ForkBlueprintCourse {
        source: ObservedBlueprintSource,
        created: ObservedBlueprintSource,
    },
    AdoptBlueprintAssignment {
        course: CourseReference,
        assignment: AssignmentReference,
    },
    InstantiateBlueprintCourse {
        course: CourseReference,
    },
    RolloverCourseInstance {
        course: CourseReference,
    },
    ShiftCourseInstanceTerm {
        course: CourseReference,
    },
    ControlledUpdateBlueprintAssignment {
        course: CourseReference,
        assignment: AssignmentReference,
    },
    CreateSelectedBlueprintAssignment {
        course: CourseReference,
        assignment: AssignmentReference,
    },
    ReconcileCourseInstanceAdoption {
        course: CourseReference,
    },
}

impl MemoryCurriculumAdoptionOutcome {
    pub(super) fn operation(&self) -> CurriculumAdoptionOperation {
        match self {
            Self::ForkBlueprintCourse { .. } => CurriculumAdoptionOperation::ForkBlueprintCourse,
            Self::AdoptBlueprintAssignment { .. } => {
                CurriculumAdoptionOperation::AdoptBlueprintAssignment
            }
            Self::InstantiateBlueprintCourse { .. } => {
                CurriculumAdoptionOperation::InstantiateBlueprintCourse
            }
            Self::RolloverCourseInstance { .. } => {
                CurriculumAdoptionOperation::RolloverCourseInstance
            }
            Self::ShiftCourseInstanceTerm { .. } => {
                CurriculumAdoptionOperation::ShiftCourseInstanceTerm
            }
            Self::ControlledUpdateBlueprintAssignment { .. } => {
                CurriculumAdoptionOperation::ControlledUpdateBlueprintAssignment
            }
            Self::CreateSelectedBlueprintAssignment { .. } => {
                CurriculumAdoptionOperation::CreateSelectedBlueprintAssignment
            }
            Self::ReconcileCourseInstanceAdoption { .. } => {
                CurriculumAdoptionOperation::ReconcileCourseInstanceAdoption
            }
        }
    }

    pub(super) fn completed(
        &self,
        replay: CurriculumReplayStatus,
    ) -> Option<CurriculumAdoptionCompleted> {
        match self {
            Self::ForkBlueprintCourse { created, .. } => {
                Some(CurriculumAdoptionCompleted::ForkBlueprintCourse {
                    completed: ForkBlueprintCourseCompleted {
                        blueprint: created.reference,
                        revision: created.revision,
                        replay,
                    },
                })
            }
            Self::AdoptBlueprintAssignment { course, assignment } => {
                Some(CurriculumAdoptionCompleted::AdoptBlueprintAssignment {
                    completed: question_model::AdoptBlueprintAssignmentCompleted {
                        course: *course,
                        assignment: *assignment,
                        replay,
                    },
                })
            }
            Self::InstantiateBlueprintCourse { course } => {
                Some(CurriculumAdoptionCompleted::InstantiateBlueprintCourse {
                    completed: question_model::InstantiateBlueprintCourseCompleted {
                        course: *course,
                        replay,
                    },
                })
            }
            Self::RolloverCourseInstance { course } => {
                Some(CurriculumAdoptionCompleted::RolloverCourseInstance {
                    completed: question_model::RolloverCourseInstanceCompleted {
                        course: *course,
                        replay,
                    },
                })
            }
            Self::ShiftCourseInstanceTerm { course } => {
                Some(CurriculumAdoptionCompleted::ShiftCourseInstanceTerm {
                    completed: question_model::ShiftCourseInstanceTermCompleted {
                        course: *course,
                        replay,
                    },
                })
            }
            Self::ControlledUpdateBlueprintAssignment { course, assignment } => Some(
                CurriculumAdoptionCompleted::ControlledUpdateBlueprintAssignment {
                    completed: question_model::ControlledUpdateBlueprintAssignmentCompleted {
                        course: *course,
                        assignment: *assignment,
                        replay,
                    },
                },
            ),
            Self::CreateSelectedBlueprintAssignment { course, assignment } => Some(
                CurriculumAdoptionCompleted::CreateSelectedBlueprintAssignment {
                    completed: question_model::CreateSelectedBlueprintAssignmentCompleted {
                        course: *course,
                        assignment: *assignment,
                        replay,
                    },
                },
            ),
            Self::ReconcileCourseInstanceAdoption { .. } => None,
        }
    }

    pub(super) fn reconciliation_completed(
        &self,
        replay: CurriculumReplayStatus,
    ) -> Option<question_model::ReconcileCourseInstanceAdoptionCompleted> {
        let Self::ReconcileCourseInstanceAdoption { course } = self else {
            return None;
        };
        Some(question_model::ReconcileCourseInstanceAdoptionCompleted {
            course: *course,
            replay,
        })
    }
}

/// Immutable receipt inserted only with its immutable evidence and derived projections.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MemoryCurriculumAdoptionReceipt {
    pub(super) operation: CurriculumAdoptionOperation,
    pub(super) actor: UserId,
    pub(super) idempotency_key: CurriculumAdoptionIdempotencyKey,
    pub(super) request_digest: CurriculumAdoptionRequestDigest,
    pub(super) occurred_at: ActivityTimestamp,
    pub(super) outcome: MemoryCurriculumAdoptionOutcome,
    pub(super) evidence: MemoryCurriculumAdoptionEvidence,
}

/// Immutable, answer-free facts required to validate a completed operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum MemoryCurriculumAdoptionEvidence {
    ForkBlueprintCourse {
        source: ObservedBlueprintSource,
        created: ObservedBlueprintSource,
    },
    AdoptBlueprintAssignment {
        source: AssignmentDefinitionSourceView,
        destination: CourseInstanceWitness,
        import_revision: CurriculumImportRevision,
    },
    InstantiateBlueprintCourse {
        source: ObservedBlueprintSource,
        destination: CourseInstanceWitness,
    },
    CourseInstanceReceipt(CourseInstanceReceiptTarget),
}

/// Immutable assignment evidence indexed by its exact destination and import revision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StoredAssignmentAdoptionEvidence {
    pub(super) receipt_actor: UserId,
    pub(super) receipt_key: CurriculumAdoptionIdempotencyKey,
    pub(super) source: AssignmentDefinitionSourceView,
    pub(super) destination: CourseInstanceWitness,
    pub(super) import_revision: CurriculumImportRevision,
}

/// Repairable current projection rebuilt only from immutable assignment evidence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StoredAssignmentImport {
    pub(super) receipt_actor: UserId,
    pub(super) receipt_key: CurriculumAdoptionIdempotencyKey,
    pub(super) import_revision: CurriculumImportRevision,
}

/// Immutable whole-CourseInstance adoption locator retained beside its receipt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StoredWholeCourseAdoption {
    pub(super) receipt_actor: UserId,
    pub(super) receipt_key: CurriculumAdoptionIdempotencyKey,
    pub(super) destination: CourseInstanceWitness,
}
