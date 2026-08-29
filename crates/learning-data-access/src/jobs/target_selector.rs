//! Closed target evidence derived from durable background-job payloads.
//!
//! Selectors name bounded evidence for trusted storage to resolve into a
//! locked worker manifest. They do not authorize worker access themselves.

use question_model::{
    AssignmentId, CourseId, ObjectId, ProblemVersionRef, QuestionAttemptId, ScoringGeneration,
    WorkspaceId, WorkspaceImportId,
};

use super::{AcceptedSubmissionId, GradingExecutionGeneration, JobPayload, RetentionStage};

/// Closed server-side evidence naming the durable target implied by a job payload.
///
/// A selector is intentionally not authority: trusted storage resolves it to an
/// exact locked manifest before a worker may act. It carries only the bounded
/// identifiers and stale-work fences already present in [`JobPayload`]. It is
/// neither serialized nor a browser contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobTargetSelector {
    /// One accepted automated submission execution.
    AcceptedSubmission {
        attempt: QuestionAttemptId,
        submission: AcceptedSubmissionId,
        execution_generation: GradingExecutionGeneration,
    },
    /// One assignment score rebuild generation.
    AssignmentScores {
        assignment: AssignmentId,
        scoring_generation: ScoringGeneration,
    },
    /// One assignment item-analysis rebuild generation.
    AssignmentItemAnalysis {
        assignment: AssignmentId,
        scoring_generation: ScoringGeneration,
    },
    /// One active-attempt timing fence.
    AttemptTiming {
        attempt: QuestionAttemptId,
        timing_generation: u64,
    },
    /// One course-retention stage and schedule generation.
    CourseRetention {
        course: CourseId,
        stage: RetentionStage,
        generation: u64,
    },
    /// One deterministic global catalog render.
    CatalogRender {
        reference: ProblemVersionRef,
        seed: u64,
    },
    /// One existing export delivery object.
    ExportDeliveryObject { delivery_object: ObjectId },
    /// One immutable import source object.
    ImportSourceObject { source_object: ObjectId },
    /// One workspace-owned QTI import source.
    WorkspaceImport {
        workspace: WorkspaceId,
        import: WorkspaceImportId,
        source_object: ObjectId,
    },
    /// One immutable catalog asset-publication target.
    CatalogAssetPublication { reference: ProblemVersionRef },
}

impl JobPayload {
    /// Returns the closed target evidence carried by this payload.
    ///
    /// Later trusted queue storage resolves this selector atomically to the
    /// locked worker manifest. The selector itself grants no access.
    pub const fn target_selector(&self) -> JobTargetSelector {
        match *self {
            Self::GradeAcceptedSubmission {
                attempt,
                submission,
                execution_generation,
            } => JobTargetSelector::AcceptedSubmission {
                attempt,
                submission,
                execution_generation,
            },
            Self::RecalculateAssignment {
                assignment,
                generation,
            } => JobTargetSelector::AssignmentScores {
                assignment,
                scoring_generation: generation,
            },
            Self::RecalculateCourseItemAnalysis {
                assignment,
                generation,
            } => JobTargetSelector::AssignmentItemAnalysis {
                assignment,
                scoring_generation: generation,
            },
            Self::AutoSubmitAttempt {
                attempt,
                timing_generation,
            } => JobTargetSelector::AttemptTiming {
                attempt,
                timing_generation,
            },
            Self::Retention {
                course,
                stage,
                generation,
            } => JobTargetSelector::CourseRetention {
                course,
                stage,
                generation,
            },
            Self::Render { reference, seed } => {
                JobTargetSelector::CatalogRender { reference, seed }
            }
            Self::Export { delivery_object } => {
                JobTargetSelector::ExportDeliveryObject { delivery_object }
            }
            Self::Import { source_object } => {
                JobTargetSelector::ImportSourceObject { source_object }
            }
            Self::QtiImport {
                workspace,
                import,
                source_object,
            } => JobTargetSelector::WorkspaceImport {
                workspace,
                import,
                source_object,
            },
            Self::PublishPublicAssets { reference } => {
                JobTargetSelector::CatalogAssetPublication { reference }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use question_model::{ProblemId, VersionId};
    use uuid::Uuid;

    use super::*;

    #[test]
    fn every_payload_family_preserves_its_exact_target_selector_evidence() {
        let reference = ProblemVersionRef {
            problem: ProblemId::from_uuid(Uuid::from_u128(1)),
            version: VersionId::from_uuid(Uuid::from_u128(2)),
        };
        let scoring_generation = ScoringGeneration::new(3).expect("positive scoring generation");
        let execution_generation =
            GradingExecutionGeneration::from_u64(4).expect("positive execution generation");
        let cases = [
            (
                JobPayload::GradeAcceptedSubmission {
                    attempt: QuestionAttemptId::from_uuid(Uuid::from_u128(5)),
                    submission: AcceptedSubmissionId::from_uuid(Uuid::from_u128(6)),
                    execution_generation,
                },
                JobTargetSelector::AcceptedSubmission {
                    attempt: QuestionAttemptId::from_uuid(Uuid::from_u128(5)),
                    submission: AcceptedSubmissionId::from_uuid(Uuid::from_u128(6)),
                    execution_generation,
                },
            ),
            (
                JobPayload::RecalculateAssignment {
                    assignment: AssignmentId::from_uuid(Uuid::from_u128(7)),
                    generation: scoring_generation,
                },
                JobTargetSelector::AssignmentScores {
                    assignment: AssignmentId::from_uuid(Uuid::from_u128(7)),
                    scoring_generation,
                },
            ),
            (
                JobPayload::RecalculateCourseItemAnalysis {
                    assignment: AssignmentId::from_uuid(Uuid::from_u128(8)),
                    generation: scoring_generation,
                },
                JobTargetSelector::AssignmentItemAnalysis {
                    assignment: AssignmentId::from_uuid(Uuid::from_u128(8)),
                    scoring_generation,
                },
            ),
            (
                JobPayload::AutoSubmitAttempt {
                    attempt: QuestionAttemptId::from_uuid(Uuid::from_u128(9)),
                    timing_generation: 10,
                },
                JobTargetSelector::AttemptTiming {
                    attempt: QuestionAttemptId::from_uuid(Uuid::from_u128(9)),
                    timing_generation: 10,
                },
            ),
            (
                JobPayload::Retention {
                    course: CourseId::from_uuid(Uuid::from_u128(11)),
                    stage: RetentionStage::ArchiveStudentRecords,
                    generation: 12,
                },
                JobTargetSelector::CourseRetention {
                    course: CourseId::from_uuid(Uuid::from_u128(11)),
                    stage: RetentionStage::ArchiveStudentRecords,
                    generation: 12,
                },
            ),
            (
                JobPayload::Render {
                    reference,
                    seed: 13,
                },
                JobTargetSelector::CatalogRender {
                    reference,
                    seed: 13,
                },
            ),
            (
                JobPayload::Export {
                    delivery_object: ObjectId::from_uuid(Uuid::from_u128(14)),
                },
                JobTargetSelector::ExportDeliveryObject {
                    delivery_object: ObjectId::from_uuid(Uuid::from_u128(14)),
                },
            ),
            (
                JobPayload::Import {
                    source_object: ObjectId::from_uuid(Uuid::from_u128(15)),
                },
                JobTargetSelector::ImportSourceObject {
                    source_object: ObjectId::from_uuid(Uuid::from_u128(15)),
                },
            ),
            (
                JobPayload::QtiImport {
                    workspace: WorkspaceId::from_uuid(Uuid::from_u128(16)),
                    import: WorkspaceImportId::from_uuid(Uuid::from_u128(17)),
                    source_object: ObjectId::from_uuid(Uuid::from_u128(18)),
                },
                JobTargetSelector::WorkspaceImport {
                    workspace: WorkspaceId::from_uuid(Uuid::from_u128(16)),
                    import: WorkspaceImportId::from_uuid(Uuid::from_u128(17)),
                    source_object: ObjectId::from_uuid(Uuid::from_u128(18)),
                },
            ),
            (
                JobPayload::PublishPublicAssets { reference },
                JobTargetSelector::CatalogAssetPublication { reference },
            ),
        ];

        for (payload, expected) in cases {
            assert_eq!(payload.target_selector(), expected);
        }
    }
}
