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
#[path = "conformance/drivers.rs"]
mod drivers;
#[path = "conformance/export_cases.rs"]
mod export_cases;
#[path = "conformance/publication.rs"]
mod publication;
use publication::*;
#[path = "conformance/assignment_workspace.rs"]
mod assignment_workspace;
#[path = "conformance/assignments.rs"]
mod assignments;
use assignments::*;
#[path = "conformance/core_store.rs"]
mod core_store;
use core_store::*;
#[path = "conformance/navigation_references.rs"]
mod navigation_references;
use navigation_references::*;
#[path = "conformance/pagination_scale.rs"]
mod pagination_scale;
use pagination_scale::*;
#[path = "conformance/catalog.rs"]
mod catalog;
#[path = "conformance/co_instructor_memory.rs"]
mod co_instructor_memory;
#[path = "conformance/course_appearance.rs"]
mod course_appearance;
#[path = "conformance/course_gradebook.rs"]
mod course_gradebook;
#[path = "conformance/effective_policy.rs"]
mod effective_policy;
#[path = "conformance/effective_policy_parity.rs"]
mod effective_policy_parity;
#[path = "conformance/enrollment.rs"]
mod enrollment;
#[path = "conformance/entitlement.rs"]
mod entitlement;
#[path = "conformance/external_tool.rs"]
mod external_tool;
#[path = "conformance/flat_import_provenance.rs"]
mod flat_import_provenance;
#[path = "conformance/flat_question.rs"]
mod flat_question;
#[path = "conformance/flat_question_assets.rs"]
mod flat_question_assets;
#[path = "conformance/group_store_memory.rs"]
mod group_store_memory;
#[path = "conformance/item_analysis.rs"]
mod item_analysis;
#[path = "conformance/jobs.rs"]
mod jobs;
#[path = "conformance/preview_plane_memory.rs"]
mod preview_plane_memory;
#[path = "conformance/qti.rs"]
mod qti;
#[path = "conformance/qti_ingress.rs"]
mod qti_ingress;
#[path = "conformance/sessions.rs"]
mod sessions;
#[path = "conformance/student_work_inspection.rs"]
mod student_work_inspection;

use assets::source_artifact;

use learning_data_access::in_memory::MemoryStore;
use learning_data_access::{
    AcceptedSubmissionCommand, AcceptedSubmissionExecutionDisposition,
    AcceptedSubmissionExecutionOutcome, AcceptedSubmissionExecutionRecoveryClaimStore,
    AcceptedSubmissionExecutionStore, AcceptedSubmissionGrade, AccountIdentityStore,
    AccountSessionLifetime, AccountSessionStore, AccountSessionTokenHash, AuthenticationEmail,
    AuthenticationRateLimitDecision, AuthenticationRateLimitKey, AuthenticationRateLimitPolicy,
    AuthenticationRateLimitScope, AutomatedGradingStore, BeginEmailAuthentication,
    BrowserBindingHash, ClaimCourseInvitation, CommitCourseRosterImport,
    CompleteCourseInvitationDelivery, CompleteEmailAuthentication,
    CompleteEmailAuthenticationAndCreateSession, CompleteEmailChangeAndRevokeUserSessions,
    CompletePasskeyAuthenticationAndCreateSession, ConsumeAuthenticationRateLimit,
    CourseInvitationDeliveryState, CourseInvitationDeliveryStore,
    CourseInvitationDeliveryWorkerStore, CourseInvitationLifetime, CourseInvitationSecretHash,
    CourseRosterId, CourseRosterImportLifetime, CourseRosterImportRowInput, CourseRosterStore,
    CourseRosterSupportAction, CreateCourseInvitation, CredentialIdHash,
    EmailAuthenticationPurpose, EmailChallengeId, EmailChallengeLifetime, EmailChallengeSecretHash,
    InvitationDeliveryReissuance, PasskeyId, PasskeyRecord, RegisterPasskey,
    RevokeCourseInvitation, RevokeCourseMember, RosterIdempotencyKey, RosterImportInvitation,
    RosterImportRowStatus, RosterRevision, StageCourseRosterImport, UpsertCourseMember,
    WebauthnState, validated_passkey_label,
};
use learning_data_access::{
    ActivityTransition, AddAssignmentFixedItemCommand, AssetDeliveryId, AssetDeliveryRecord,
    AssetDeliveryScope, AssetPublication, AssetStore, AssignmentContentUpdate,
    AssignmentPoliciesUpdate, AssignmentRecord, AssignmentScoringCommitOutcome,
    AssignmentScoringWorkerCommand, AssignmentScoringWorkerStore, AssignmentUpdate,
    AttemptSupportAction, AttemptSupportActionId, CatalogSourceStore, CatalogStore,
    CatalogTransition, ClearAttemptCommand, CourseCreationAuthority, CourseGroupRecord,
    CourseListScope, CourseRecord, CreateAssignmentCommand, CreateAssignmentDraftCommand,
    CreateCourseCommand, Cursor, DeleteAndRegradeAssignmentItemCommand, DraftRecord,
    FlatGradingCapability, ForceSubmitAttemptCommand, IssueQuestionAttemptCommand,
    MaterializeAssignmentEntitlementCommand, NativeExecutionEnvelopeCapability,
    NavigationReferenceStore, PageRequest, PageSize, PrefetchedPrivateExecutionV1,
    PrefetchedQuestionDescriptorV1, PresentationCapability, PublishDraftCommand,
    PublishedSourceArtifact, PutCourseGroupCommand, QtiGradingCapability,
    ReleaseAttemptFeedbackCommand, RemoveAssignmentFixedItemCommand, ReplaceAssignmentCommand,
    ReplaceAssignmentContentCommand, ReplaceAssignmentContentOutcome,
    ReplaceAssignmentFixedItemCommand, ReplaceAssignmentPoliciesCommand,
    ReplaceAssignmentPoliciesOutcome, ReservePrefetchedQuestionCommand, RunRouteIdentity,
    SessionLifetime, SessionStore, SessionSubject, SessionTokenHash, Store, StoreError,
    StudentWorkRoutingBinding, SubmissionIdempotencyKey, SubmitQuestionAttemptCommand,
    TenantContext, WebworkGradingCapability, WebworkReplayControlV1, WebworkReplayMappingV1,
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
    JobFailureDisposition, JobFailureKind, JobId, JobKind, JobLeaseDuration, JobLeaseToken,
    JobPayload, JobState, JobStore, WorkerId, canonical_attempt_result_json,
};
use objects::{ObjectCategory, ObjectKey, ObjectRecord, Sha256Digest};
use question_model::answer::NumericTolerance;
use question_model::envelope::ContentBlock;
use question_model::generation::RandomizationDefinition;
use question_model::response::StudentResponse;
use question_model::run_policy::{AttemptPolicy, TimingPolicy};
use question_model::taxonomy::{License, Tag};
use question_model::{
    ActivityTimestamp, AssetId, AssignmentDeliveryState, AssignmentId, AssignmentItem,
    AssignmentItemId, AssignmentPolicyExceptionId, AssignmentRun, AssignmentScoringMode,
    AssignmentSelectionCandidate, AssignmentSelectionGroup, AssignmentSelectionGroupId,
    AttemptProvenance, AttemptResult, AttemptStatus, AttemptTimerRecord, BackendCapabilities,
    Capability, CompletionRequirement, ContinuedPractice, CourseGroupId, CourseId,
    CourseMembershipRole, DraftQuestionDefinition, DraftQuestionSource, EnrollmentId,
    EntitlementPurpose, FeedbackContent, GeneratorReference, GradePolicy, GradingDefinition,
    ImplementationVersion, ObjectId, PointValue, ProblemDisplayRef, ProblemId, ProblemVersionRef,
    PublicAuthorName, PublicByline, PublicationScope, QuestionAttempt, QuestionAttemptId,
    QuestionBackend, QuestionDefinition, QuestionMetadata, QuestionSource, RenderedItemIdV1,
    ResponseDefinition, RunId, RunPolicies, SelectionOrdering, SourceArtifact,
    StudentDisclosurePolicy, StudentDisclosureTiming, TenantId, UserId, UserRole, VariationPolicy,
    VersionId, WorkspaceId, WorkspaceImportId,
};
use std::sync::atomic::{AtomicU64, Ordering};
use uuid::Uuid;

fn uuid(value: u128) -> Uuid {
    Uuid::from_u128(value)
}

/// Returns the deterministic Sysadmin authority used by ordinary course
/// fixtures.  The session is persisted through the real SessionStore so the
/// course write still exercises the backend's authority check.
pub(crate) async fn sysadmin_course_creation_authority<S>(
    store: &S,
    tenant: TenantId,
    course: CourseId,
    actor: UserId,
) -> CourseCreationAuthority
where
    S: SessionStore + ?Sized,
{
    let mut token_material = b"conformance-course-creation-sysadmin-v1".to_vec();
    token_material.extend_from_slice(tenant.as_uuid().as_bytes());
    token_material.extend_from_slice(course.as_uuid().as_bytes());
    token_material.extend_from_slice(actor.as_uuid().as_bytes());
    let session = SessionTokenHash::compute(&token_material);
    let subject = SessionSubject::new(
        tenant,
        actor,
        "Conformance Sysadmin",
        vec![UserRole::Sysadmin],
    )
    .expect("conformance Sysadmin subject");

    let record = match store
        .resolve_session(session)
        .await
        .expect("resolve deterministic course-creation session")
    {
        Some(record) => record,
        None => {
            match store
                .create_session(
                    session,
                    subject.clone(),
                    SessionLifetime::from_seconds(3_600).expect("positive session lifetime"),
                )
                .await
            {
                Ok(_) | Err(StoreError::AlreadyExists) => {}
                Err(error) => panic!("create deterministic course-creation session: {error:?}"),
            }
            store
                .resolve_session(session)
                .await
                .expect("resolve created course-creation session")
                .expect("created course-creation session remains active")
        }
    };

    assert_eq!(record.token_hash, session);
    assert_eq!(record.subject, subject);
    assert!(record.expires_at > record.created_at);
    CourseCreationAuthority::Sysadmin { actor, session }
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

/// Conformance fixtures intentionally name the teaching defaults at the one
/// creation boundary.  Individual tests that exercise lifecycle or schedule
/// behavior construct their own explicit settings instead.
trait ConformanceAssignmentStore: Store {
    async fn create_assignment_with_default_policy(
        &self,
        context: TenantContext,
        actor: UserId,
        mut assignment: AssignmentRecord,
    ) -> Result<learning_data_access::StoredAssignment, StoreError> {
        let settings = question_model::AssignmentTeachingSettings {
            lifecycle: assignment.lifecycle,
            instructions: assignment.instructions.clone(),
            base_policy: question_model::BaseAssignmentPolicy::default(),
        };
        assignment.lifecycle = question_model::AssignmentLifecycle::Draft;
        let created = self
            .create_assignment(
                context,
                CreateAssignmentCommand {
                    actor,
                    assignment,
                    base_policy: question_model::BaseAssignmentPolicy::default(),
                },
            )
            .await?;
        self.put_assignment_teaching_settings(
            context,
            learning_data_access::PutAssignmentTeachingSettingsCommand {
                actor,
                course: created.record.course_id,
                assignment: created.record.id,
                expected_revision: created.revision,
                settings,
            },
        )
        .await?;
        self.get_assignment_for_edit(context, created.record.id)
            .await?
            .ok_or(StoreError::NotFound)
    }
}

impl<T: Store + ?Sized> ConformanceAssignmentStore for T {}

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
        attempt_policy: AttemptPolicy { max_attempts: None },
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

fn native_issued_question_snapshot(
    workspace: WorkspaceId,
    problem: ProblemId,
    version: VersionId,
) -> learning_data_access::IssuedQuestionSnapshotV1 {
    learning_data_access::IssuedQuestionSnapshotV1::new(
        QuestionDefinition::from_draft(
            draft_question(workspace),
            problem,
            version,
            published_source(),
        ),
        learning_data_access::IssuedQuestionFamilyWitnessV1::Native {
            physical_asset_bindings: Vec::new(),
        },
    )
    .expect("construct exact native issued-question snapshot")
}

fn reviewed_byline() -> PublicByline {
    PublicByline::new(vec![
        PublicAuthorName::new("Peptidyle Test Author".to_string()).expect("valid test byline"),
    ])
    .expect("valid test byline")
}

fn policies() -> RunPolicies {
    RunPolicies {
        completion: CompletionRequirement::AllCorrect,
        grade: GradePolicy::Highest,
        continued_practice: ContinuedPractice::Unlimited,
        variation: VariationPolicy::NewSeeds,
    }
}

/// Run the focused immutable-publication and assignment-revision contract
/// against a durable Store implementation.
///
/// The memory test remains the ordinary owner; an ignored disposable database
/// oracle calls this explicit wrapper to prove the same behavior through
/// PostgreSQL transactions, RLS, and broker functions.
pub(crate) async fn exercise_durable_publication_assignment_contract<S>(store: &S)
where
    S: Store + CatalogStore + CourseRosterStore + SessionStore,
{
    exercise_publication_identity_boundary(store).await;
    exercise_assignment_cas(store).await;
    assignments::exercise_assignment_workspace_slices(store).await;
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
    workspace: WorkspaceId,
    problem: ProblemId,
    version: VersionId,
    course: CourseId,
    assignment: AssignmentId,
    reservation: PrefetchedQuestionDescriptorV1,
    response: StudentResponse,
    run: AssignmentRun,
}
