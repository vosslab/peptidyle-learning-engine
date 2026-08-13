//! In-memory Store backend (WP-C4, MOD-STO).

mod account_identity;
mod activity;
mod assets;
mod authoring;
mod catalog;
#[cfg(test)]
mod catalog_search_tests;
mod course_appearance;
mod course_assignments;
mod course_policy;
mod course_roster;
mod courses;
mod exports;
mod external_tool;
mod feedback;
mod flat_import_provenance;
mod flat_question;
mod flat_question_assets;
mod item_analysis;
mod manual_grade_export;
mod qti;
mod qti_ingress;
mod queue;
mod retention;
mod runs;
mod sessions;
mod state;
mod statistics;

use activity::{
    add_seconds, apply_memory_attempt_support, complete_memory_attempt_timing_job, issued_timer,
    projected_attempt, require_attempt_course_records_accessible, require_attempt_owner,
    timing_policy_grace_seconds,
};
use catalog::{catalog_record_visible, page_records};
use course_assignments::{assignment_record, enrollment_record};
use course_policy::{
    apply_memory_assignment_timing_update, memory_assignment_has_results,
    memory_resolved_assignment_policy, resolved_memory_attempt_timing,
    validate_assignment_position, validate_memory_assignment_content_lock,
    validate_memory_assignment_references,
};
use runs::submit_question_attempt_locked;
use statistics::stage_statistics_contributions;

use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, RwLock};
use uuid::Uuid;

use async_trait::async_trait;
use domain::run::continued_practice_allows_run;
use domain::scoring::project_summary;
#[cfg(test)]
use domain::statistics::CollapsedQuestionObservation;
use domain::statistics::QuestionStatisticsAggregate;
use domain::timing::{TimerEvaluation, TimerVerdict, timer_verdict};
use question_model::run_policy::{FeedbackDisclosure, TimingPolicy};
use question_model::taxonomy::TaxonomyTerm;
use question_model::{
    ActivityTimestamp, AssignmentEnrollment, AssignmentId, AssignmentPolicyExceptionId,
    AssignmentRun, AssignmentRunItem, AssignmentTimingPolicy, AttemptStatus, AttemptTimerRecord,
    CatalogCapabilityFacet, CatalogLicenseFacet, CatalogLicenseValue, CatalogLifecycle,
    CatalogProblemDetail, CatalogProblemSummary, CatalogSearchFacets, CatalogSearchPage,
    CatalogSearchQuery, CatalogStatisticsAvailability, CatalogStatisticsFacet,
    CatalogTaxonomyFacet, CourseGroupId, CourseId, CourseRole, CourseSummary, EnrollmentId,
    EnrollmentStatus, MAX_CATALOG_TAXONOMY_FACETS, PresentationBindingV1, ProblemId,
    ProblemPublicId, ProblemVersionNumber, ProblemVersionRef, PublicationScope, QuestionAttempt,
    QuestionAttemptId, QuestionEnvelope, QuestionStatisticsDisclosure, RunId, RunMode,
    ScoringGeneration, ScoringStatus, StatisticsDisclosurePolicy, StudentAssignmentSummary,
    StudentId, StudentResponse, TenantId, UserId, VersionId, WorkspaceId, WorkspaceImportId,
};

use crate::gradebook_cursor::GradebookCursor;
use crate::retention::RetentionApiAction;
use crate::retention::{RetentionCleanupManifestState, StoredRetentionCleanupManifest};
use crate::run_summary_cursor::RunSummaryCursor;
use crate::statistics::{StatisticsContribution, derive_statistics_contributions};
use crate::{
    AccountRecord, AccountSessionRecord, AccountSessionTokenHash, ActivityTransition,
    AssetAccessEvent, AssetDeliveryId, AssetDeliveryRecord, AssignmentDefinitionDisposition,
    AssignmentEditorUpdate, AssignmentPolicyException, AssignmentPolicyExceptionTarget,
    AssignmentRecord, AssignmentRevision, AttemptSupportAction, AttemptSupportActionId,
    AttemptSupportRecord, AuthenticationRateLimitKey, AuthenticationRateLimitScope,
    CatalogSourceStore, CatalogStore, CatalogTransition, ClearAttemptCommand,
    CourseEnrollmentPolicy, CourseGroupRecord, CourseGroupRevision, CourseInvitationId,
    CourseInvitationSecretHash, CourseListScope, CourseMemberId, CourseRecord,
    CourseRecordsAccessStore, CourseRetentionRecord, CourseRetentionSnapshot, CourseRetentionState,
    CourseRetentionView, CourseRosterId, CourseRosterMember, CredentialIdHash, Cursor,
    DeleteAndRegradeAssignmentItemCommand, DeleteAssignmentPolicyExceptionCommand, DraftRecord,
    EmailChallengeSecretHash, FeedbackReleaseRecord, FlatQuestionGradingPayload,
    ForceSubmitAttemptCommand, InstitutionRetentionPolicy, IssueQuestionAttemptCommand, Page,
    PageRequest, PageSize, PasskeyId, PasskeyRecord, PrefetchedQuestion, PublishDraftCommand,
    PublishedProblemRecord, PublishedSourceArtifact, PutCourseGroupCommand,
    RETENTION_JOB_MAX_ATTEMPTS, ReleaseAttemptFeedbackCommand, ReservePrefetchedQuestionCommand,
    ResolvedAssignmentTiming, ResolvedAttemptTiming, RetentionApiStore, RetentionCleanupManifest,
    RetentionDays, RetentionDispatchBatch, RetentionRevision, RetentionScheduleStore,
    RetentionStore, RetentionWork, RetentionWorkerCommand, RetentionWorkerStore,
    RosterIdempotencyKey, RunSummaryOutcomeInput, RunSummaryPageInput, SessionTokenHash,
    SetAssignmentPolicyExceptionCommand, StoreError, StoredAssignment,
    StoredAssignmentPolicyException, StoredAssignmentTiming, StoredCourseGroup,
    SubmissionIdempotencyKey, SubmissionNextAttempt, SubmissionRecord,
    SubmitQuestionAttemptCommand, TenantContext, UpdateAssignmentTimingCommand, WebauthnCeremony,
    WebauthnCeremonyId, WebworkGradeReplayStateV1, WorkspaceDraft, WorkspaceDraftRevision,
    WorkspaceDraftRole, WorkspaceFlatQuestionSource, assignment_item_is_retired,
    assignment_scoring_changed, completed_run_score, current_run_questions,
    decode_catalog_search_cursor, delete_and_regrade_update, encode_catalog_search_cursor,
    ensure_tenant, grade_policy, private_feedback_record, project_enrollment_completion,
    resolve_assignment_policy, select_assignment_run_items, summary_transition,
    validate_assignment, validate_assignment_policy_exception, validate_assignment_timing,
    validate_course, validate_course_group, validate_draft, validate_published,
    validate_qti_publication_promotion,
};

mod manual_grading;

#[async_trait]
impl crate::AuthoritativeTimeStore for MemoryStore {
    async fn authoritative_time(
        &self,
        _context: crate::TenantContext,
    ) -> Result<ActivityTimestamp, StoreError> {
        Ok(self.read_state()?.authoritative_time)
    }
}
#[async_trait]
impl CourseRecordsAccessStore for MemoryStore {
    async fn course_records_accessible(
        &self,
        context: TenantContext,
        course: CourseId,
    ) -> Result<bool, StoreError> {
        let state = self.read_state()?;
        Ok(course_records_accessible(
            &state,
            context.tenant_id(),
            course,
        ))
    }
}

/// The one in-memory equivalent of the database learner-record predicate.
/// Keep this helper free of actor/session authority: route authorization and
/// ordinary record visibility are separate concerns.
fn course_records_accessible(state: &State, tenant: TenantId, course: CourseId) -> bool {
    if !state.courses.contains_key(&(tenant, course)) {
        return false;
    }
    let Some(retention) = state.course_retention.get(&(tenant, course)) else {
        return true;
    };
    if matches!(
        retention.status.state,
        CourseRetentionState::StudentRecordsArchived | CourseRetentionState::StudentRecordsDeleted
    ) {
        return false;
    }
    let generation = retention.snapshot.generation();
    if let Some(stage) = state.retention_stages.get(&(
        tenant,
        course,
        crate::RetentionStage::ArchiveStudentRecords,
        generation,
    )) && stage.state == RetentionStageWorkState::Started
    {
        return false;
    }
    if let Some(stage) = state.retention_stages.get(&(
        tenant,
        course,
        crate::RetentionStage::DeleteStudentRecords,
        generation,
    )) && stage.state == RetentionStageWorkState::Started
    {
        return false;
    }
    true
}

fn require_course_records_accessible(
    state: &State,
    tenant: TenantId,
    course: CourseId,
) -> Result<(), StoreError> {
    course_records_accessible(state, tenant, course)
        .then_some(())
        .ok_or(StoreError::NotFound)
}
use crate::{
    ClaimedJob, CreateAssignmentExport, EnqueueJob, ExportArtifactKind, ExportArtifactRecord,
    ExportCommitDisposition, ExportId, ExportJobCommit, ExportJobStore, JobFailureDisposition,
    JobFailureKind, JobId, JobLeaseDuration, JobLeaseToken, JobPayload, JobState, JobStore,
    QueueDepth, StudentExportArtifactView, StudentExportJob, StudentExportState, StudentExportView,
    TenantJobView,
};
use crate::{
    ExternalToolBinding, ExternalToolLeaseToken, ExternalToolVerifiedPending, PersistedCorrelation,
};
use crate::{QtiImportGradingPayload, QtiImportRegistry};
use objects::Sha256Digest;

/// Memory backend used by conformance tests and pre-PostgreSQL lanes.
#[derive(Debug, Clone, Default)]
pub struct MemoryStore {
    state: Arc<RwLock<State>>,
}

/// Injected test/local grader capability. It is intentionally a different
/// handle from [`MemoryStore`], so application persistence cannot read QTI
/// answer material merely by having a tenant context.
#[derive(Clone)]
pub struct MemoryQtiGraderStore {
    state: Arc<RwLock<State>>,
}

/// Injected test/local grader capability for private flat-question grading material.
#[derive(Clone)]
pub struct MemoryFlatQuestionGraderStore {
    state: Arc<RwLock<State>>,
}

impl MemoryStore {
    /// Builds separately injected application and grader handles for a local
    /// composition. Production code uses the PostgreSQL grader handle instead.
    pub fn with_qti_grader() -> (Self, MemoryQtiGraderStore) {
        let state = Arc::new(RwLock::new(State::default()));
        (
            Self {
                state: Arc::clone(&state),
            },
            MemoryQtiGraderStore { state },
        )
    }

    /// Builds separate application and flat-question grader handles sharing state.
    pub fn with_flat_question_grader() -> (Self, MemoryFlatQuestionGraderStore) {
        let state = Arc::new(RwLock::new(State::default()));
        (
            Self {
                state: Arc::clone(&state),
            },
            MemoryFlatQuestionGraderStore { state },
        )
    }
}

/// All maps use tenant ID in their key for tenant-owned records.
#[derive(Debug, Default, Clone)]
struct State {
    authoritative_time: ActivityTimestamp,
    next_problem_public_id: u64,
    accounts: BTreeMap<UserId, AccountRecord>,
    account_by_email: BTreeMap<String, UserId>,
    authentication_rate_limits: BTreeMap<
        (AuthenticationRateLimitScope, AuthenticationRateLimitKey),
        account_identity::StoredAuthenticationRateLimit,
    >,
    email_challenges: BTreeMap<EmailChallengeSecretHash, account_identity::StoredEmailChallenge>,
    account_sessions: BTreeMap<AccountSessionTokenHash, AccountSessionRecord>,
    webauthn_ceremonies: BTreeMap<WebauthnCeremonyId, WebauthnCeremony>,
    passkeys: BTreeMap<PasskeyId, PasskeyRecord>,
    passkey_by_credential: BTreeMap<CredentialIdHash, PasskeyId>,
    sessions: BTreeMap<SessionTokenHash, sessions::StoredSession>,
    catalog_grants: BTreeSet<(TenantId, ProblemId, VersionId)>,
    drafts: BTreeMap<(TenantId, WorkspaceId), DraftRecord>,
    draft_revisions: BTreeMap<(TenantId, WorkspaceId), WorkspaceDraftRevision>,
    draft_access: BTreeMap<(TenantId, WorkspaceId, UserId), WorkspaceDraftRole>,
    problem_owner_tenants: BTreeMap<ProblemId, TenantId>,
    published: BTreeMap<(ProblemId, VersionId), PublishedProblemRecord>,
    source_artifacts: BTreeMap<(ProblemId, VersionId), PublishedSourceArtifact>,
    flat_question_sources: BTreeMap<(TenantId, WorkspaceId), WorkspaceFlatQuestionSource>,
    workspace_flat_question_assets: BTreeMap<
        (TenantId, WorkspaceId, question_model::AssetId),
        crate::WorkspaceFlatQuestionAsset,
    >,
    workspace_flat_question_grading: BTreeMap<(TenantId, WorkspaceId), FlatQuestionGradingPayload>,
    published_flat_question_grading: BTreeMap<(ProblemId, VersionId), FlatQuestionGradingPayload>,
    workspace_flat_import_origins: flat_import_provenance::WorkspaceFlatImportOrigins,
    published_flat_import_origins: flat_import_provenance::PublishedFlatImportOrigins,
    qti_profile_import_evidence: flat_import_provenance::QtiProfileImportEvidences,
    qti_imports: BTreeMap<(TenantId, WorkspaceId, WorkspaceImportId), QtiImportRegistry>,
    prepared_qti_imports: BTreeMap<(TenantId, WorkspaceId, WorkspaceImportId), QtiImportRegistry>,
    qti_grading:
        BTreeMap<(TenantId, WorkspaceId, WorkspaceImportId, String), QtiImportGradingPayload>,
    published_qti_grading: BTreeMap<(ProblemId, VersionId, String), QtiImportGradingPayload>,
    prepared_qti_grading:
        BTreeMap<(TenantId, WorkspaceId, WorkspaceImportId, String), QtiImportGradingPayload>,
    courses: BTreeMap<(TenantId, CourseId), CourseRecord>,
    learner_by_user: BTreeMap<(TenantId, UserId), StudentId>,
    learner_by_student: BTreeSet<(TenantId, StudentId, UserId)>,
    roster_policies: BTreeMap<(TenantId, CourseId), CourseEnrollmentPolicy>,
    roster_members: BTreeMap<(TenantId, CourseId, CourseMemberId), CourseRosterMember>,
    roster_member_by_user: BTreeMap<(TenantId, CourseId, UserId), CourseMemberId>,
    roster_member_by_roster_id: BTreeMap<(TenantId, CourseId, CourseRosterId), CourseMemberId>,
    course_invitations:
        BTreeMap<(TenantId, CourseId, CourseInvitationId), course_roster::StoredCourseInvitation>,
    invitation_by_hash:
        BTreeMap<CourseInvitationSecretHash, (TenantId, CourseId, CourseInvitationId)>,
    invitation_idempotency: BTreeMap<
        (TenantId, CourseId, RosterIdempotencyKey),
        (CourseInvitationId, CourseInvitationSecretHash),
    >,
    roster_imports: BTreeMap<
        (TenantId, CourseId, crate::CourseRosterImportId),
        course_roster::import::StoredCourseRosterImport,
    >,
    roster_import_idempotency:
        BTreeMap<(TenantId, CourseId, RosterIdempotencyKey), crate::CourseRosterImportId>,
    manual_grade_export_audits:
        BTreeMap<crate::ManualGradeExportId, (TenantId, CourseId, AssignmentId, UserId, usize)>,
    course_appearances: BTreeMap<(TenantId, CourseId), question_model::CourseAppearance>,
    course_banner_candidates: BTreeMap<
        (TenantId, CourseId, question_model::CourseBannerCandidateId),
        course_appearance::StoredCourseBannerCandidate,
    >,
    course_groups: BTreeMap<(TenantId, CourseGroupId), CourseGroupRecord>,
    course_group_revisions: BTreeMap<(TenantId, CourseGroupId), CourseGroupRevision>,
    assignments: BTreeMap<(TenantId, AssignmentId), AssignmentRecord>,
    assignment_revisions: BTreeMap<(TenantId, AssignmentId), AssignmentRevision>,
    assignment_timing: BTreeMap<(TenantId, AssignmentId), AssignmentTimingPolicy>,
    assignment_policy_exceptions: BTreeMap<
        (TenantId, AssignmentId, AssignmentPolicyExceptionTarget),
        AssignmentPolicyException,
    >,
    assignment_scoring: BTreeMap<(TenantId, AssignmentId), (ScoringGeneration, ScoringStatus)>,
    assignment_score_staging: BTreeMap<JobId, PreparedAssignmentScoring>,
    item_analysis_staging: BTreeMap<JobId, PreparedCourseItemAnalysis>,
    item_analysis:
        BTreeMap<(TenantId, AssignmentId), domain::item_analysis::CourseItemAnalysisReport>,
    attempt_scores: BTreeMap<(TenantId, QuestionAttemptId), MemoryAttemptScore>,
    enrollments: BTreeMap<(TenantId, EnrollmentId), AssignmentEnrollment>,
    runs: BTreeMap<(TenantId, RunId), AssignmentRun>,
    run_items: BTreeMap<(TenantId, RunId), Vec<AssignmentRunItem>>,
    attempts: BTreeMap<(TenantId, QuestionAttemptId), QuestionAttempt>,
    attempt_presentation_capabilities:
        BTreeMap<(TenantId, QuestionAttemptId), crate::PresentationCapability>,
    attempt_presentations: BTreeMap<(TenantId, QuestionAttemptId), PresentationBindingV1>,
    attempt_presentation_snapshots:
        BTreeMap<(TenantId, QuestionAttemptId), crate::ReceiptPresentationSnapshot>,
    attempt_grading_envelopes: BTreeMap<(TenantId, QuestionAttemptId), QuestionEnvelope>,
    attempt_flat_grading_capabilities:
        BTreeMap<(TenantId, QuestionAttemptId), crate::FlatGradingCapability>,
    attempt_flat_grading: BTreeMap<(TenantId, QuestionAttemptId), crate::IssuedFlatGradingContract>,
    attempt_webwork_grading_capabilities:
        BTreeMap<(TenantId, QuestionAttemptId), crate::WebworkGradingCapability>,
    attempt_webwork_grading:
        BTreeMap<(TenantId, QuestionAttemptId), crate::IssuedWebworkGradingContract>,
    attempt_feedback_disclosures: BTreeMap<(TenantId, QuestionAttemptId), FeedbackDisclosure>,
    webwork_grade_replay: BTreeMap<(TenantId, QuestionAttemptId), WebworkGradeReplayStateV1>,
    attempt_timing: BTreeMap<(TenantId, QuestionAttemptId), MemoryAttemptTiming>,
    attempt_timing_resolution:
        BTreeMap<(TenantId, QuestionAttemptId), crate::ResolvedAssignmentTimingPolicy>,
    attempt_current: BTreeMap<(TenantId, QuestionAttemptId), QuestionAttempt>,
    attempt_support_actions: BTreeMap<(TenantId, AttemptSupportActionId), AttemptSupportRecord>,
    manual_evaluations: BTreeMap<(TenantId, QuestionAttemptId), crate::ManualEvaluationRecord>,
    manual_grade_actions:
        BTreeMap<(TenantId, crate::ManualGradeActionId), manual_grading::MemoryManualGradeReceipt>,
    prefetched_questions: BTreeMap<(TenantId, RunId, QuestionAttemptId, u32), PrefetchedQuestion>,
    submissions: BTreeMap<(TenantId, QuestionAttemptId), StoredSubmission>,
    submission_next_attempts:
        BTreeMap<(TenantId, QuestionAttemptId), Option<crate::ReceiptNextAttempt>>,
    feedback_releases: BTreeMap<(TenantId, QuestionAttemptId), FeedbackReleaseRecord>,
    question_statistics: BTreeMap<(ProblemId, VersionId), QuestionStatisticsAggregate>,
    question_statistics_receipts:
        BTreeMap<(TenantId, EnrollmentId, ProblemId, VersionId), StatisticsContributionReceipt>,
    retention_policies: BTreeMap<TenantId, InstitutionRetentionPolicy>,
    course_retention: BTreeMap<(TenantId, CourseId), CourseRetentionRecord>,
    retention_stages:
        BTreeMap<(TenantId, CourseId, crate::RetentionStage, u64), StoredRetentionStage>,
    retention_cleanup_manifests:
        BTreeMap<(TenantId, CourseId, u64, crate::RetentionStage), StoredRetentionCleanupManifest>,
    /// The only identities permitted to execute retention payloads. A job may
    /// look valid, but without this scheduler-created binding R3 refuses it.
    retention_dispatches: BTreeMap<(TenantId, CourseId, crate::RetentionStage, u64), crate::JobId>,
    /// Durable in-app retention notification identities; no recipient or
    /// learner record is duplicated into the worker/outbox projection.
    retention_notifications: BTreeMap<(TenantId, CourseId, u64), crate::RetentionNotificationView>,
    retention_api_receipts: BTreeMap<(TenantId, CourseId, u64), RetentionApiReceipt>,
    external_tool_exchanges: BTreeMap<(TenantId, QuestionAttemptId), StoredExternalToolExchange>,
    external_tool_launch_sessions: BTreeMap<(TenantId, Uuid), StoredExternalToolLaunchSession>,
    summaries: BTreeMap<(TenantId, EnrollmentId), StudentAssignmentSummary>,
    asset_deliveries: BTreeMap<AssetDeliveryId, AssetDeliveryRecord>,
    asset_access_events: Vec<AssetAccessEvent>,
    jobs: BTreeMap<JobId, StoredJob>,
    exports: BTreeMap<(TenantId, ExportId), StoredExport>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RetentionStageWorkState {
    Scheduled,
    Started,
    Completed,
    Superseded,
}
#[derive(Debug, Clone, Copy)]
struct StoredRetentionStage {
    due_at: ActivityTimestamp,
    state: RetentionStageWorkState,
    job: Option<crate::JobId>,
    lease: Option<crate::JobLeaseToken>,
}

#[derive(Debug, Clone, Copy)]
struct RetentionApiMutation {
    retention: CourseRetentionView,
    manual_outcome: Option<crate::RetentionRequestOutcome>,
}

#[derive(Debug, Clone, Copy)]
struct RetentionApiReceipt {
    actor: UserId,
    action: RetentionApiAction,
    resulting_generation: u64,
    stage: crate::RetentionStage,
}

fn cleanup_manifest_record_to_work(
    stored: &StoredRetentionCleanupManifest,
) -> RetentionCleanupManifest {
    RetentionCleanupManifest {
        objects: stored.objects.iter().cloned().collect(),
    }
}

/// Queue state held under the same mutex as the authoritative test clock.
#[derive(Debug, Clone)]
struct StoredJob {
    tenant: TenantId,
    payload: JobPayload,
    state: JobState,
    available_at: ActivityTimestamp,
    lease_token: Option<JobLeaseToken>,
    lease_expires_at: Option<ActivityTimestamp>,
    attempt_count: u16,
    max_attempts: u16,
    failure: Option<JobFailureKind>,
}

#[derive(Debug, Clone)]
struct MemoryAttemptScore {
    assignment: AssignmentId,
    assignment_item: question_model::AssignmentItemId,
    generation: ScoringGeneration,
    earned_points: f64,
    possible_points: f64,
}

#[derive(Debug, Clone, Copy)]
struct MemoryAttemptTiming {
    assignment: AssignmentId,
    authored_deadline: Option<ActivityTimestamp>,
    authored_grace_seconds: u32,
    effective_deadline: Option<ActivityTimestamp>,
    effective_grace_seconds: u32,
    auto_submit_at: Option<ActivityTimestamp>,
    generation: u64,
    job: Option<JobId>,
}

#[derive(Debug, Clone)]
struct PreparedAssignmentScoring {
    tenant: TenantId,
    assignment: AssignmentId,
    generation: ScoringGeneration,
    attempts: BTreeMap<QuestionAttemptId, MemoryAttemptScore>,
    enrollments: BTreeMap<EnrollmentId, AssignmentEnrollment>,
    summaries: BTreeMap<EnrollmentId, StudentAssignmentSummary>,
}

#[derive(Debug, Clone)]
struct PreparedCourseItemAnalysis {
    tenant: TenantId,
    assignment: AssignmentId,
    generation: ScoringGeneration,
    report: domain::item_analysis::CourseItemAnalysisReport,
}

/// Private request state; only `StudentExportView` crosses an HTTP boundary.
#[derive(Debug, Clone)]
struct StoredExport {
    course: CourseId,
    assignment: AssignmentId,
    title: String,
    requested_by: UserId,
    manifest: question_model::ObjectId,
    problems: Vec<ProblemVersionRef>,
    job: JobId,
    state: StudentExportState,
    expected: BTreeMap<ExportArtifactKind, question_model::ObjectId>,
    artifacts: Option<Vec<ExportArtifactRecord>>,
}

/// Immutable first result retained for exact submission replay.
#[derive(Debug, Clone)]
struct StoredSubmission {
    key: SubmissionIdempotencyKey,
    response: StudentResponse,
    record: SubmissionRecord,
}

/// Tenant-owned idempotency marker for a first-completed-run contribution.
///
/// It is deliberately private and omitted from all catalog projections.  The
/// matching PostgreSQL receipt has cascading tenant-record foreign keys.
#[derive(Debug, Clone, Copy)]
struct StatisticsContributionReceipt {
    first_completed_run: RunId,
    attempt: QuestionAttemptId,
    #[cfg(test)]
    observation: CollapsedQuestionObservation,
    checksum: objects::Sha256Digest,
}

/// Private broker state. No field here is exposed through a browser projection.
#[derive(Debug, Clone)]
struct StoredExternalToolExchange {
    actor: UserId,
    binding: ExternalToolBinding,
    response: StudentResponse,
    key: SubmissionIdempotencyKey,
    correlation: PersistedCorrelation,
    lease: Option<ExternalToolLeaseToken>,
    lease_expires_at: Option<ActivityTimestamp>,
    verified_lease_hash: Option<Sha256Digest>,
    verified: Option<ExternalToolVerifiedPending>,
}

#[derive(Debug, Clone)]
struct StoredExternalToolLaunchSession {
    actor: UserId,
    attempt: QuestionAttemptId,
    binding: ExternalToolBinding,
    token_hash: Sha256Digest,
    encrypted_provider_state: Option<Vec<u8>>,
    expires_at: ActivityTimestamp,
    revoked: bool,
}
