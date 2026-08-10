//! Reusable Store conformance suite, first run against memory in WP-C4.

#[path = "conformance/assets.rs"]
mod assets;
#[path = "conformance/run_api.rs"]
mod run_api;
use run_api::*;
#[path = "conformance/run_scale.rs"]
mod run_scale;
use run_scale::*;
#[path = "conformance/run_support.rs"]
mod run_support;
use run_support::*;
#[path = "conformance/run_delete_regrade.rs"]
mod run_delete_regrade;
use run_delete_regrade::*;
#[path = "conformance/run_rescoring.rs"]
mod run_rescoring;
use run_rescoring::*;
#[path = "conformance/run_receipts.rs"]
mod run_receipts;
use run_receipts::*;
#[path = "conformance/assignment_timing_cases.rs"]
mod assignment_timing_cases;
#[path = "conformance/course_policy_cases.rs"]
mod course_policy_cases;
#[path = "conformance/drivers.rs"]
mod drivers;
#[path = "conformance/export_cases.rs"]
mod export_cases;
#[path = "conformance/publication.rs"]
mod publication;
use publication::*;
#[path = "conformance/assignments.rs"]
mod assignments;
use assignments::*;
#[path = "conformance/core_store.rs"]
mod core_store;
use core_store::*;
#[path = "conformance/catalog.rs"]
mod catalog;
#[path = "conformance/course_appearance.rs"]
mod course_appearance;
#[path = "conformance/external_tool.rs"]
mod external_tool;
#[path = "conformance/flat_import_provenance.rs"]
mod flat_import_provenance;
#[path = "conformance/flat_question.rs"]
mod flat_question;
#[path = "conformance/item_analysis.rs"]
mod item_analysis;
#[path = "conformance/jobs.rs"]
mod jobs;
#[path = "conformance/manual_grading.rs"]
mod manual_grading;
#[path = "conformance/qti.rs"]
mod qti;
#[path = "conformance/qti_ingress.rs"]
mod qti_ingress;
#[path = "conformance/sessions.rs"]
mod sessions;

use assets::source_artifact;

use learning_data_access::in_memory::MemoryStore;
use learning_data_access::{
    ActivityTransition, AssetDeliveryId, AssetDeliveryRecord, AssetDeliveryScope, AssetStore,
    AssignmentExceptionLimit, AssignmentExceptionTimestamp, AssignmentPolicyException,
    AssignmentPolicyExceptionTarget, AssignmentRecord, AssignmentScoringCommitOutcome,
    AssignmentScoringWorkerCommand, AssignmentScoringWorkerStore, AssignmentUpdate,
    AttemptAutoSubmitCommitOutcome, AttemptAutoSubmitWorkerCommand, AttemptAutoSubmitWorkerStore,
    AttemptSupportAction, AttemptSupportActionId, CatalogSourceStore, CatalogStore,
    CatalogTransition, ClearAttemptCommand, CourseGroupRecord, CourseListScope, CourseRecord,
    Cursor, DeleteAndRegradeAssignmentItemCommand, DeleteAssignmentPolicyExceptionCommand,
    DraftRecord, EvaluationRevision, ForceSubmitAttemptCommand, IssueQuestionAttemptCommand,
    ManualCredit, ManualGradeActionId, ManualGradingStore, PageRequest, PageSize,
    PrefetchedQuestion, PublishDraftCommand, PublishedSourceArtifact, PublishedVersionRef,
    PutCourseGroupCommand, ReleaseAttemptFeedbackCommand, ReservePrefetchedQuestionCommand,
    SessionLifetime, SessionStore, SessionSubject, SessionTokenHash,
    SetAssignmentPolicyExceptionCommand, SetManualGradeCommand, Store, StoreError,
    SubmissionIdempotencyKey, SubmitPendingManualQuestionAttemptCommand,
    SubmitQuestionAttemptCommand, TenantContext, UpdateAssignmentTimingCommand,
};
use learning_data_access::{
    BeginExternalToolGradeCommand, CommitVerifiedExternalToolSubmissionCommand,
    CreateExternalToolLaunchSessionCommand, ExternalToolBegin, ExternalToolBrokerStore,
    ExternalToolLaunchProof, ExternalToolLaunchSessionStore, PersistedCorrelation,
    StageExternalToolVerificationCommand,
};
use learning_data_access::{
    CommitPreparedQtiImport, CommitPreparedQtiImportOutcome, CreateQtiImportCommand,
    QtiGradingStore, QtiImportGradingPayload, QtiImportItem, QtiImportItemRegistration,
    QtiImportItemResult, QtiImportItemStatus, QtiImportRef, QtiImportRegistry, QtiImportStore,
    QtiPublicationPromotion, QtiUnsupportedFeature,
};
use learning_data_access::{
    CourseItemAnalysisCommitOutcome, CourseItemAnalysisStore, CourseItemAnalysisWorkerCommand,
    CourseItemAnalysisWorkerStore,
};
use learning_data_access::{
    CreateAssignmentExport, EnqueueJob, ExportArtifactKind, ExportArtifactRecord,
    ExportCommitDisposition, ExportJobCommit, ExportJobStore, JobClaimFilter,
    JobFailureDisposition, JobFailureKind, JobKind, JobLeaseDuration, JobLeaseToken, JobPayload,
    JobState, JobStore,
};
use objects::{ObjectCategory, ObjectKey, ObjectRecord, Sha256Digest};
use question_model::answer::NumericTolerance;
use question_model::envelope::ContentBlock;
use question_model::generation::RandomizationDefinition;
use question_model::response::StudentResponse;
use question_model::run_policy::{AttemptPolicy, FeedbackDisclosure, TimingPolicy};
use question_model::taxonomy::{License, Tag, TaxonomyTerm};
use question_model::{
    ActivityTimestamp, AssetId, AssignmentDeliveryState, AssignmentEnrollment, AssignmentId,
    AssignmentItem, AssignmentItemId, AssignmentPolicyExceptionId, AssignmentRun,
    AssignmentScoringMode, AssignmentTimingPolicy, AttemptProvenance, AttemptResult, AttemptStatus,
    AttemptTimerRecord, BackendCapabilities, Capability, CatalogLifecycle, CompletionRequirement,
    ContinuedPractice, CourseGroupId, CourseId, CourseMembership, CourseMembershipRole, CourseRole,
    DraftQuestionDefinition, DraftQuestionSource, EnrollmentId, FeedbackContent,
    GeneratorReference, GradePolicy, GradingDefinition, ImplementationVersion,
    LateSubmissionPolicy, ObjectId, PointValue, PresentationBindingV1, PresentationDigestV1,
    PresentationNonceV1, ProblemId, ProblemVersionRef, PublicationScope, QuestionAttempt,
    QuestionAttemptId, QuestionBackend, QuestionMetadata, QuestionSource, ResponseDefinition,
    RunId, RunMode, RunPolicies, SourceArtifact, StudentId, TenantId, UserId, UserRole,
    VariationPolicy, VersionId, WorkspaceId, WorkspaceImportId,
};
use std::sync::atomic::{AtomicU64, Ordering};
use uuid::Uuid;

fn uuid(value: u128) -> Uuid {
    Uuid::from_u128(value)
}

fn presentation_binding(marker: u8) -> PresentationBindingV1 {
    PresentationBindingV1::new(
        PresentationNonceV1::from_bytes([marker; 16]),
        PresentationDigestV1::compute(&[marker]),
    )
}

fn fixed_items(references: Vec<ProblemVersionRef>) -> Vec<AssignmentItem> {
    static NEXT_ITEM_ID: AtomicU64 = AtomicU64::new(900_000);
    references
        .into_iter()
        .enumerate()
        .map(|(position, reference)| AssignmentItem {
            id: AssignmentItemId::from_uuid(uuid(u128::from(
                NEXT_ITEM_ID.fetch_add(1, Ordering::Relaxed),
            ))),
            reference,
            position: u32::try_from(position).expect("fixture position fits"),
            points_possible: PointValue::from_whole(1),
            delivery_state: AssignmentDeliveryState::Active,
            scoring_mode: AssignmentScoringMode::Normal,
        })
        .collect()
}

fn draft_question(workspace: WorkspaceId) -> DraftQuestionDefinition {
    DraftQuestionDefinition {
        workspace,
        source: DraftQuestionSource::Native {
            family: "molar_mass".to_string(),
        },
        prompt: vec![ContentBlock::Text {
            markdown: "What is the molar mass?".to_string(),
        }],
        response: ResponseDefinition::Numeric {
            tolerance: NumericTolerance::Relative { fraction: 0.01 },
            unit: Some("g/mol".to_string()),
        },
        attempt_policy: AttemptPolicy {
            max_attempts: None,
            feedback: FeedbackDisclosure::ImmediateFull,
        },
        timing_policy: TimingPolicy::Untimed,
        randomization: RandomizationDefinition::Static,
        grading: GradingDefinition::AllOrNothing { points: 1.0 },
        metadata: QuestionMetadata {
            title: "Molar mass".to_string(),
            tags: vec![Tag::new("biochemistry")],
            taxonomy: Vec::new(),
            license: License::CcBySa,
            language: "en-US".to_string(),
        },
    }
}

fn published_source() -> QuestionSource {
    QuestionSource::Native {
        family: "molar_mass".to_string(),
    }
}

fn policies() -> RunPolicies {
    RunPolicies {
        completion: CompletionRequirement::AllCorrect,
        grade: GradePolicy::Highest,
        continued_practice: ContinuedPractice::Unlimited,
        variation: VariationPolicy::NewSeeds,
    }
}

fn implementation(id: &str) -> ImplementationVersion {
    ImplementationVersion {
        id: id.to_string(),
        version: "1".to_string(),
    }
}

fn generator(id: &str) -> GeneratorReference {
    GeneratorReference {
        id: id.to_string(),
        version: "1".to_string(),
    }
}

fn object_record(key: ObjectKey, bytes: &[u8], at: i64) -> ObjectRecord {
    ObjectRecord {
        id: key.object_id(),
        bucket: key.bucket(),
        sha256: Sha256Digest::compute(bytes),
        size_bytes: u64::try_from(bytes.len()).expect("fixture size should fit"),
        media_type: "image/svg+xml".to_string(),
        category: key.category(),
        version: key.version_id(),
        license: "CC BY-SA 4.0".to_string(),
        provenance: "asset delivery conformance fixture".to_string(),
        created_at: ActivityTimestamp::from_unix_millis(at),
        key,
    }
}

struct RunApiFixture {
    fixture_offset: u128,
    tenant: TenantId,
    context: TenantContext,
    publisher: UserId,
    student_user: UserId,
    problem: ProblemId,
    version: VersionId,
    course: CourseId,
    assignment: AssignmentId,
    reservation: PrefetchedQuestion,
    response: StudentResponse,
    run: AssignmentRun,
}
