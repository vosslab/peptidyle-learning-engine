//! In-memory Store backend (WP-C4, MOD-STO).

mod assets;
mod catalog;
#[cfg(test)]
mod catalog_search_tests;
mod exports;
mod external_tool;
mod flat_import_provenance;
mod flat_question;
mod item_analysis;
mod qti;
mod qti_ingress;
mod queue;
mod retention;
mod sessions;

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
use question_model::run_policy::TimingPolicy;
use question_model::taxonomy::TaxonomyTerm;
use question_model::{
    ActivityTimestamp, AssignmentEnrollment, AssignmentId, AssignmentPolicyExceptionId,
    AssignmentRun, AssignmentRunItem, AssignmentTimingPolicy, AttemptStatus, AttemptTimerRecord,
    CatalogCapabilityFacet, CatalogLicenseFacet, CatalogLicenseValue, CatalogLifecycle,
    CatalogProblemDetail, CatalogProblemSummary, CatalogSearchFacets, CatalogSearchPage,
    CatalogSearchQuery, CatalogStatisticsAvailability, CatalogStatisticsFacet,
    CatalogTaxonomyFacet, CourseGroupId, CourseId, CourseRole, CourseSummary, EnrollmentId,
    EnrollmentStatus, MAX_CATALOG_TAXONOMY_FACETS, ProblemId, ProblemPublicId,
    ProblemVersionNumber, ProblemVersionRef, PublicationScope, QuestionAttempt, QuestionAttemptId,
    QuestionStatisticsDisclosure, RunId, RunMode, ScoringGeneration, ScoringStatus,
    StatisticsDisclosurePolicy, StudentAssignmentSummary, StudentId, StudentResponse, TenantId,
    UserId, VersionId, WorkspaceId, WorkspaceImportId,
};

use crate::gradebook_cursor::GradebookCursor;
use crate::retention::RetentionApiAction;
use crate::retention::{RetentionCleanupManifestState, StoredRetentionCleanupManifest};
use crate::run_summary_cursor::RunSummaryCursor;
use crate::statistics::{StatisticsContribution, derive_statistics_contributions};
use crate::{
    ActivityTransition, AssetAccessEvent, AssetDeliveryId, AssetDeliveryRecord,
    AssignmentDefinitionDisposition, AssignmentPolicyException, AssignmentPolicyExceptionTarget,
    AssignmentRecord, AssignmentRevision, AssignmentUpdate, AttemptSupportAction,
    AttemptSupportActionId, AttemptSupportRecord, CatalogSourceStore, CatalogStore,
    CatalogTransition, ClearAttemptCommand, CourseGroupRecord, CourseGroupRevision,
    CourseListScope, CourseRecord, CourseRecordsAccessStore, CourseRetentionRecord,
    CourseRetentionSnapshot, CourseRetentionState, CourseRetentionView, Cursor,
    DeleteAndRegradeAssignmentItemCommand, DeleteAssignmentPolicyExceptionCommand, DraftRecord,
    FeedbackReleaseRecord, FlatQuestionGradingPayload, ForceSubmitAttemptCommand,
    InstitutionRetentionPolicy, IssueQuestionAttemptCommand, Page, PageRequest, PageSize,
    PrefetchedQuestion, PublishDraftCommand, PublishedProblemRecord, PublishedSourceArtifact,
    PutCourseGroupCommand, RETENTION_JOB_MAX_ATTEMPTS, ReleaseAttemptFeedbackCommand,
    ReservePrefetchedQuestionCommand, ResolvedAssignmentTiming, ResolvedAttemptTiming,
    RetentionApiStore, RetentionCleanupManifest, RetentionDays, RetentionDispatchBatch,
    RetentionRevision, RetentionScheduleStore, RetentionStore, RetentionWork,
    RetentionWorkerCommand, RetentionWorkerStore, RunSummaryOutcomeInput, RunSummaryPageInput,
    SessionTokenHash, SetAssignmentPolicyExceptionCommand, Store, StoreError, StoredAssignment,
    StoredAssignmentPolicyException, StoredAssignmentTiming, StoredCourseGroup,
    SubmissionIdempotencyKey, SubmissionNextAttempt, SubmissionRecord,
    SubmitQuestionAttemptCommand, TenantContext, UpdateAssignmentTimingCommand, WorkspaceDraft,
    WorkspaceDraftRevision, WorkspaceDraftRole, WorkspaceFlatQuestionSource,
    assignment_item_is_retired, assignment_scoring_changed, completed_run_score,
    current_run_questions, decode_catalog_search_cursor, delete_and_regrade_update,
    encode_catalog_search_cursor, ensure_tenant, grade_policy, private_feedback_record,
    project_enrollment_completion, resolve_assignment_policy, select_assignment_run_items,
    summary_transition, validate_assignment, validate_assignment_policy_exception,
    validate_assignment_timing, validate_course, validate_course_group, validate_draft,
    validate_published, validate_qti_publication_promotion,
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
    sessions: BTreeMap<SessionTokenHash, sessions::StoredSession>,
    catalog_grants: BTreeSet<(TenantId, ProblemId, VersionId)>,
    drafts: BTreeMap<(TenantId, WorkspaceId), DraftRecord>,
    draft_revisions: BTreeMap<(TenantId, WorkspaceId), WorkspaceDraftRevision>,
    draft_access: BTreeMap<(TenantId, WorkspaceId, UserId), WorkspaceDraftRole>,
    problem_owner_tenants: BTreeMap<ProblemId, TenantId>,
    published: BTreeMap<(ProblemId, VersionId), PublishedProblemRecord>,
    source_artifacts: BTreeMap<(ProblemId, VersionId), PublishedSourceArtifact>,
    flat_question_sources: BTreeMap<(TenantId, WorkspaceId), WorkspaceFlatQuestionSource>,
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
    submission_next_attempts: BTreeMap<(TenantId, QuestionAttemptId), Option<QuestionAttemptId>>,
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

#[async_trait]
impl Store for MemoryStore {
    async fn question_statistics(
        &self,
        context: TenantContext,
        reference: ProblemVersionRef,
    ) -> Result<QuestionStatisticsDisclosure, StoreError> {
        let state = self.read_state()?;
        let visible = state
            .published
            .get(&(reference.problem, reference.version))
            .is_some_and(|record| catalog_record_visible(&state, context.tenant_id(), record));
        if !visible {
            return Ok(QuestionStatisticsDisclosure::Suppressed);
        }
        let disclosure = state
            .question_statistics
            .get(&(reference.problem, reference.version))
            .map(|aggregate| aggregate.disclose(StatisticsDisclosurePolicy::default()))
            .unwrap_or(QuestionStatisticsDisclosure::Suppressed);
        Ok(disclosure)
    }

    async fn upsert_draft(
        &self,
        context: TenantContext,
        actor: UserId,
        expected_revision: Option<WorkspaceDraftRevision>,
        draft: DraftRecord,
    ) -> Result<WorkspaceDraft, StoreError> {
        ensure_tenant(context, draft.tenant)?;
        validate_draft(&draft)?;
        let mut state = self.write_state()?;
        let key = (draft.tenant, draft.question.workspace);
        if state.drafts.contains_key(&key) {
            let role = state
                .draft_access
                .get(&(draft.tenant, draft.question.workspace, actor));
            if !matches!(
                role,
                Some(WorkspaceDraftRole::Owner | WorkspaceDraftRole::Collaborator)
            ) {
                return Err(StoreError::Forbidden);
            }
            let current = state
                .draft_revisions
                .get(&key)
                .copied()
                .ok_or(StoreError::Forbidden)?;
            if expected_revision != Some(current) {
                return Err(StoreError::Conflict);
            }
            let revision = current.next()?;
            state.drafts.insert(key, draft.clone());
            state.draft_revisions.insert(key, revision);
            state.flat_question_sources.remove(&key);
            state.workspace_flat_question_grading.remove(&key);
            return Ok(WorkspaceDraft {
                record: draft,
                revision,
            });
        }
        if expected_revision.is_some() {
            return Err(StoreError::Conflict);
        }
        let revision = WorkspaceDraftRevision::INITIAL;
        state.drafts.insert(key, draft.clone());
        state.draft_revisions.insert(key, revision);
        state.draft_access.insert(
            (draft.tenant, draft.question.workspace, actor),
            WorkspaceDraftRole::Owner,
        );
        state.flat_question_sources.remove(&key);
        state.workspace_flat_question_grading.remove(&key);
        Ok(WorkspaceDraft {
            record: draft,
            revision,
        })
    }

    async fn get_draft(
        &self,
        context: TenantContext,
        actor: UserId,
        workspace: WorkspaceId,
    ) -> Result<Option<WorkspaceDraft>, StoreError> {
        let state = self.read_state()?;
        let key = (context.tenant_id(), workspace);
        if !state
            .draft_access
            .contains_key(&(context.tenant_id(), workspace, actor))
        {
            return Ok(None);
        }
        let Some(record) = state.drafts.get(&key).cloned() else {
            return Ok(None);
        };
        let revision = state
            .draft_revisions
            .get(&key)
            .copied()
            .ok_or(StoreError::Unavailable(
                "workspace draft is missing its revision".to_string(),
            ))?;
        Ok(Some(WorkspaceDraft { record, revision }))
    }

    async fn list_drafts(
        &self,
        context: TenantContext,
        actor: UserId,
        page: PageRequest,
    ) -> Result<Page<question_model::WorkspaceDraftSummary>, StoreError> {
        let after = page
            .after
            .as_ref()
            .map(|cursor| {
                crate::decode_workspace_draft_cursor(cursor.as_str(), context.tenant_id())
            })
            .transpose()?;
        let state = self.read_state()?;
        let mut drafts: Vec<_> = state
            .drafts
            .iter()
            .filter(|((tenant, workspace), _)| {
                *tenant == context.tenant_id()
                    && state
                        .draft_access
                        .contains_key(&(context.tenant_id(), *workspace, actor))
            })
            .map(|((_, workspace), draft)| (*workspace, draft.question.workspace_summary()))
            .collect();
        drafts.sort_by_key(|(workspace, _)| workspace.as_uuid());
        let mut selected: Vec<_> = drafts
            .into_iter()
            .filter(|(workspace, _)| {
                after.is_none_or(|cursor| workspace.as_uuid() > cursor.as_uuid())
            })
            .take(usize::from(page.size.get()) + 1)
            .collect();
        let has_more = selected.len() > usize::from(page.size.get());
        if has_more {
            selected.pop();
        }
        let next_cursor = if has_more {
            selected.last().map(|(workspace, _)| {
                Cursor::from_stable_key(crate::encode_workspace_draft_cursor(
                    context.tenant_id(),
                    *workspace,
                ))
            })
        } else {
            None
        };
        Ok(Page {
            items: selected.into_iter().map(|(_, summary)| summary).collect(),
            next_cursor,
        })
    }

    async fn delete_draft(
        &self,
        context: TenantContext,
        actor: UserId,
        workspace: WorkspaceId,
        expected_revision: WorkspaceDraftRevision,
    ) -> Result<bool, StoreError> {
        let mut state = self.write_state()?;
        let key = (context.tenant_id(), workspace);
        if !state.drafts.contains_key(&key) {
            return Ok(false);
        }
        if state
            .draft_access
            .get(&(context.tenant_id(), workspace, actor))
            != Some(&WorkspaceDraftRole::Owner)
        {
            return Err(StoreError::Forbidden);
        }
        let current_revision = state.draft_revisions.get(&key).copied().ok_or_else(|| {
            StoreError::Unavailable("workspace draft is missing its revision".to_string())
        })?;
        if current_revision != expected_revision {
            return Err(StoreError::Conflict);
        }
        let prepared_imports = state
            .prepared_qti_imports
            .keys()
            .filter(|(tenant, workspace, _)| (*tenant, *workspace) == key)
            .copied()
            .collect::<BTreeSet<_>>();
        state
            .prepared_qti_imports
            .retain(|(tenant, workspace, _), _| (*tenant, *workspace) != key);
        state
            .prepared_qti_grading
            .retain(|(tenant, workspace, import, _), _| {
                !prepared_imports.contains(&(*tenant, *workspace, *import))
            });
        state
            .qti_profile_import_evidence
            .remove_prepared_imports(&prepared_imports);
        state.drafts.remove(&key);
        state.draft_revisions.remove(&key);
        state.flat_question_sources.remove(&key);
        state.workspace_flat_question_grading.remove(&key);
        state.workspace_flat_import_origins.remove(&key);
        state.jobs.retain(|_, job| {
            job.tenant != key.0
                || !matches!(
                    job.payload,
                    JobPayload::QtiImport { workspace, .. } if workspace == key.1
                )
        });
        state
            .draft_access
            .retain(|(tenant, candidate, _), _| (*tenant, *candidate) != key);
        Ok(true)
    }

    async fn grant_draft_collaborator(
        &self,
        context: TenantContext,
        actor: UserId,
        workspace: WorkspaceId,
        collaborator: UserId,
    ) -> Result<(), StoreError> {
        let mut state = self.write_state()?;
        let key = (context.tenant_id(), workspace);
        if !state.drafts.contains_key(&key) {
            return Err(StoreError::NotFound);
        }
        if state
            .draft_access
            .get(&(context.tenant_id(), workspace, actor))
            != Some(&WorkspaceDraftRole::Owner)
        {
            return Err(StoreError::Forbidden);
        }
        if collaborator != actor {
            state.draft_access.insert(
                (context.tenant_id(), workspace, collaborator),
                WorkspaceDraftRole::Collaborator,
            );
        }
        Ok(())
    }

    async fn get_published_problem(
        &self,
        problem: ProblemId,
        version: VersionId,
    ) -> Result<Option<PublishedProblemRecord>, StoreError> {
        let state = self.read_state()?;
        Ok(state
            .published
            .get(&(problem, version))
            .filter(|record| record.scope == PublicationScope::Public)
            .cloned())
    }

    async fn list_published_problems(
        &self,
        page: PageRequest,
    ) -> Result<Page<PublishedProblemRecord>, StoreError> {
        let state = self.read_state()?;
        let records = state
            .published
            .iter()
            .filter(|(_, record)| {
                record.scope == PublicationScope::Public && record.lifecycle.is_discoverable()
            })
            .map(|((problem, version), record)| (format!("{problem}/{version}"), record.clone()))
            .collect();
        Ok(page_records(records, &page))
    }

    async fn upsert_course(
        &self,
        context: TenantContext,
        course: CourseRecord,
    ) -> Result<(), StoreError> {
        ensure_tenant(context, course.tenant)?;
        validate_course(&course)?;
        let tenant = course.tenant;
        let course_id = course.id;
        let student_members = course
            .members
            .iter()
            .filter_map(|membership| {
                (membership.role == question_model::CourseMembershipRole::Student)
                    .then_some(membership.user)
            })
            .collect::<BTreeSet<_>>();
        let mut state = self.write_state()?;
        let affected_groups = state
            .course_groups
            .iter()
            .filter_map(|((record_tenant, group), record)| {
                (*record_tenant == tenant && record.course == course_id).then_some(*group)
            })
            .collect::<BTreeSet<_>>();
        let affected_assignments = state
            .assignment_policy_exceptions
            .iter()
            .filter_map(|((record_tenant, assignment, target), _)| {
                (*record_tenant == tenant
                    && matches!(
                        target,
                        AssignmentPolicyExceptionTarget::CourseGroup(group)
                            if affected_groups.contains(group)
                    ))
                .then_some(*assignment)
            })
            .collect::<BTreeSet<_>>();
        let snapshot = state.clone();
        state.courses.insert((tenant, course_id), course);
        for group in &affected_groups {
            if let Some(record) = state.course_groups.get_mut(&(tenant, *group)) {
                record
                    .members
                    .retain(|member| student_members.contains(member));
            }
        }
        for assignment in affected_assignments {
            if let Err(error) =
                apply_memory_assignment_timing_update(&mut state, tenant, assignment, None)
            {
                *state = snapshot;
                return Err(error);
            }
        }
        Ok(())
    }

    async fn get_course(
        &self,
        context: TenantContext,
        course: CourseId,
    ) -> Result<Option<CourseRecord>, StoreError> {
        let state = self.read_state()?;
        Ok(state.courses.get(&(context.tenant_id(), course)).cloned())
    }

    async fn list_courses(
        &self,
        context: TenantContext,
        scope: CourseListScope,
        page: PageRequest,
    ) -> Result<Page<CourseSummary>, StoreError> {
        let state = self.read_state()?;
        let records = state
            .courses
            .iter()
            .filter_map(|((tenant, course_id), record)| {
                if *tenant != context.tenant_id() {
                    return None;
                }
                let role = match scope {
                    CourseListScope::Member(user) => record.role_for(user)?,
                    CourseListScope::TenantAdministrator => CourseRole::Administrator,
                };
                if role == CourseRole::Student
                    && !course_records_accessible(&state, context.tenant_id(), *course_id)
                {
                    return None;
                }
                Some((course_id.to_string(), record.summary(role)))
            })
            .collect();
        Ok(page_records(records, &page))
    }

    async fn put_course_group(
        &self,
        context: TenantContext,
        command: PutCourseGroupCommand,
    ) -> Result<StoredCourseGroup, StoreError> {
        ensure_tenant(context, command.record.tenant)?;
        validate_course_group(&command.record)?;
        let tenant = context.tenant_id();
        let key = (tenant, command.record.id);
        let mut state = self.write_state()?;
        require_course_records_accessible(&state, tenant, command.record.course)?;
        let course = state
            .courses
            .get(&(tenant, command.record.course))
            .ok_or(StoreError::NotFound)?;
        if course.role_for(command.actor) != Some(CourseRole::Instructor)
            || command
                .record
                .members
                .iter()
                .any(|user| course.role_for(*user) != Some(CourseRole::Student))
        {
            return Err(StoreError::NotFound);
        }
        if state
            .course_groups
            .get(&key)
            .is_some_and(|existing| existing.course != command.record.course)
        {
            return Err(StoreError::Conflict);
        }
        if let Some(existing) = state.course_groups.get(&key)
            && existing == &command.record
        {
            return Ok(StoredCourseGroup {
                record: existing.clone(),
                revision: state
                    .course_group_revisions
                    .get(&key)
                    .copied()
                    .ok_or(StoreError::NotFound)?,
            });
        }
        let revision = match state.course_group_revisions.get(&key).copied() {
            Some(current) if command.expected_revision == Some(current) => current.next()?,
            Some(_) => return Err(StoreError::Conflict),
            None if command.expected_revision.is_none() => CourseGroupRevision::INITIAL,
            None => return Err(StoreError::Conflict),
        };
        let affected = state
            .assignment_policy_exceptions
            .iter()
            .filter_map(|((record_tenant, assignment, target), _)| {
                (*record_tenant == tenant
                    && *target == AssignmentPolicyExceptionTarget::CourseGroup(command.record.id))
                .then_some(*assignment)
            })
            .collect::<BTreeSet<_>>();
        let snapshot = state.clone();
        state.course_groups.insert(key, command.record.clone());
        state.course_group_revisions.insert(key, revision);
        for assignment in affected {
            if let Err(error) =
                apply_memory_assignment_timing_update(&mut state, tenant, assignment, None)
            {
                *state = snapshot;
                return Err(error);
            }
        }
        Ok(StoredCourseGroup {
            record: command.record,
            revision,
        })
    }

    async fn get_course_group(
        &self,
        context: TenantContext,
        group: CourseGroupId,
    ) -> Result<Option<StoredCourseGroup>, StoreError> {
        let state = self.read_state()?;
        let key = (context.tenant_id(), group);
        let Some(record) = state.course_groups.get(&key).cloned() else {
            return Ok(None);
        };
        Ok(Some(StoredCourseGroup {
            record,
            revision: state
                .course_group_revisions
                .get(&key)
                .copied()
                .ok_or(StoreError::NotFound)?,
        }))
    }

    async fn create_assignment(
        &self,
        context: TenantContext,
        assignment: AssignmentRecord,
    ) -> Result<StoredAssignment, StoreError> {
        ensure_tenant(context, assignment.tenant)?;
        validate_assignment(&assignment)?;
        let mut state = self.write_state()?;
        let key = (assignment.tenant, assignment.id);
        if state.assignments.contains_key(&key) {
            return Err(StoreError::AlreadyExists);
        }
        validate_memory_assignment_references(&state, context, &assignment)?;
        let stored = StoredAssignment {
            record: assignment,
            revision: AssignmentRevision::INITIAL,
            scoring_generation: ScoringGeneration::INITIAL,
            scoring_status: ScoringStatus::Current,
        };
        state.assignments.insert(key, stored.record.clone());
        state.assignment_revisions.insert(key, stored.revision);
        state
            .assignment_timing
            .insert(key, AssignmentTimingPolicy::default());
        state
            .assignment_scoring
            .insert(key, (stored.scoring_generation, stored.scoring_status));
        Ok(stored)
    }

    async fn replace_assignment(
        &self,
        context: TenantContext,
        course: CourseId,
        assignment: AssignmentId,
        expected_revision: AssignmentRevision,
        update: AssignmentUpdate,
    ) -> Result<StoredAssignment, StoreError> {
        let mut state = self.write_state()?;
        let key = (context.tenant_id(), assignment);
        let existing = state
            .assignments
            .get(&key)
            .cloned()
            .ok_or(StoreError::NotFound)?;
        if existing.course_id != course {
            return Err(StoreError::NotFound);
        }
        let current = state
            .assignment_revisions
            .get(&key)
            .copied()
            .ok_or(StoreError::NotFound)?;
        if current != expected_revision {
            return Err(StoreError::Conflict);
        }
        let assignment = AssignmentRecord {
            id: assignment,
            tenant: context.tenant_id(),
            course_id: course,
            title: update.title,
            items: update.items,
            selection_groups: update.selection_groups,
            policies: update.policies,
        };
        validate_assignment(&assignment)?;
        validate_memory_assignment_references(&state, context, &assignment)?;
        let previous = state.assignments.get(&key).ok_or(StoreError::NotFound)?;
        validate_memory_assignment_content_lock(&state, previous, &assignment)?;
        let scoring_changed = assignment_scoring_changed(previous, &assignment);
        let (generation, _) = state
            .assignment_scoring
            .get(&key)
            .copied()
            .ok_or(StoreError::NotFound)?;
        let scoring_generation = if scoring_changed {
            generation.next().ok_or(StoreError::Conflict)?
        } else {
            generation
        };
        let scoring_status =
            if scoring_changed && memory_assignment_has_results(&state, &assignment) {
                ScoringStatus::Recalculating
            } else {
                ScoringStatus::Current
            };
        if scoring_status == ScoringStatus::Recalculating {
            let job = crate::JobId::generate()?;
            let queued = StoredJob {
                tenant: assignment.tenant,
                payload: crate::JobPayload::RecalculateAssignment {
                    assignment: assignment.id,
                    generation: scoring_generation,
                },
                state: JobState::Ready,
                available_at: state.authoritative_time,
                lease_token: None,
                lease_expires_at: None,
                attempt_count: 0,
                max_attempts: 10,
                failure: None,
            };
            if state.jobs.insert(job, queued).is_some() {
                return Err(StoreError::Conflict);
            }
        }
        let stored = StoredAssignment {
            record: assignment,
            revision: current.next()?,
            scoring_generation,
            scoring_status,
        };
        state.assignments.insert(key, stored.record.clone());
        state.assignment_revisions.insert(key, stored.revision);
        state
            .assignment_scoring
            .insert(key, (stored.scoring_generation, stored.scoring_status));
        Ok(stored)
    }

    async fn get_assignment_timing(
        &self,
        context: TenantContext,
        assignment: AssignmentId,
    ) -> Result<Option<StoredAssignmentTiming>, StoreError> {
        let state = self.read_state()?;
        let key = (context.tenant_id(), assignment);
        let Some(record) = state.assignments.get(&key) else {
            return Ok(None);
        };
        let policy = state.assignment_timing.get(&key).copied().ok_or_else(|| {
            StoreError::Unavailable(
                "assignment timing policy is missing from memory state".to_string(),
            )
        })?;
        let revision = state
            .assignment_revisions
            .get(&key)
            .copied()
            .ok_or(StoreError::NotFound)?;
        Ok(Some(StoredAssignmentTiming {
            tenant: context.tenant_id(),
            course: record.course_id,
            assignment,
            policy,
            revision,
        }))
    }

    async fn update_assignment_timing(
        &self,
        context: TenantContext,
        command: UpdateAssignmentTimingCommand,
    ) -> Result<StoredAssignmentTiming, StoreError> {
        validate_assignment_timing(command.policy)?;
        let tenant = context.tenant_id();
        let key = (tenant, command.assignment);
        let mut state = self.write_state()?;
        let assignment = state
            .assignments
            .get(&key)
            .cloned()
            .ok_or(StoreError::NotFound)?;
        if assignment.course_id != command.course {
            return Err(StoreError::NotFound);
        }
        require_course_records_accessible(&state, tenant, command.course)?;
        let course = state
            .courses
            .get(&(tenant, command.course))
            .ok_or(StoreError::NotFound)?;
        if course.role_for(command.actor) != Some(CourseRole::Instructor) {
            return Err(StoreError::NotFound);
        }
        let current_revision = state
            .assignment_revisions
            .get(&key)
            .copied()
            .ok_or(StoreError::NotFound)?;
        let current_policy = state
            .assignment_timing
            .get(&key)
            .copied()
            .ok_or(StoreError::NotFound)?;
        if current_policy == command.policy {
            return Ok(StoredAssignmentTiming {
                tenant,
                course: command.course,
                assignment: command.assignment,
                policy: current_policy,
                revision: current_revision,
            });
        }
        if current_revision != command.expected_revision {
            return Err(StoreError::Conflict);
        }
        let revision = current_revision.next()?;
        apply_memory_assignment_timing_update(
            &mut state,
            tenant,
            command.assignment,
            Some(command.policy),
        )?;
        state.assignment_timing.insert(key, command.policy);
        state.assignment_revisions.insert(key, revision);
        Ok(StoredAssignmentTiming {
            tenant,
            course: command.course,
            assignment: command.assignment,
            policy: command.policy,
            revision,
        })
    }

    async fn set_assignment_policy_exception(
        &self,
        context: TenantContext,
        command: SetAssignmentPolicyExceptionCommand,
    ) -> Result<StoredAssignmentPolicyException, StoreError> {
        validate_assignment_policy_exception(&command.exception)?;
        let tenant = context.tenant_id();
        let assignment_key = (tenant, command.assignment);
        let exception_key = (tenant, command.assignment, command.exception.target);
        let mut state = self.write_state()?;
        let assignment = state
            .assignments
            .get(&assignment_key)
            .cloned()
            .ok_or(StoreError::NotFound)?;
        if assignment.course_id != command.course {
            return Err(StoreError::NotFound);
        }
        require_course_records_accessible(&state, tenant, command.course)?;
        let course = state
            .courses
            .get(&(tenant, command.course))
            .ok_or(StoreError::NotFound)?;
        if course.role_for(command.actor) != Some(CourseRole::Instructor) {
            return Err(StoreError::NotFound);
        }
        match command.exception.target {
            AssignmentPolicyExceptionTarget::Student(student) => {
                if !state.enrollments.values().any(|enrollment| {
                    enrollment.tenant == tenant
                        && enrollment.assignment == command.assignment
                        && enrollment.student == student
                }) {
                    return Err(StoreError::NotFound);
                }
            }
            AssignmentPolicyExceptionTarget::CourseGroup(group) => {
                if state
                    .course_groups
                    .get(&(tenant, group))
                    .is_none_or(|record| record.course != command.course)
                {
                    return Err(StoreError::NotFound);
                }
            }
        }
        if state.assignment_policy_exceptions.iter().any(
            |((record_tenant, record_assignment, target), exception)| {
                *record_tenant == tenant
                    && *record_assignment == command.assignment
                    && *target != command.exception.target
                    && exception.id == command.exception.id
            },
        ) {
            return Err(StoreError::Conflict);
        }
        let current_revision = state
            .assignment_revisions
            .get(&assignment_key)
            .copied()
            .ok_or(StoreError::NotFound)?;
        if let Some(existing) = state.assignment_policy_exceptions.get(&exception_key)
            && existing == &command.exception
        {
            return Ok(StoredAssignmentPolicyException {
                exception: existing.clone(),
                assignment_revision: current_revision,
            });
        }
        if current_revision != command.expected_revision {
            return Err(StoreError::Conflict);
        }
        if state
            .assignment_policy_exceptions
            .get(&exception_key)
            .is_some_and(|existing| existing.id != command.exception.id)
        {
            return Err(StoreError::Conflict);
        }
        let revision = current_revision.next()?;
        let snapshot = state.clone();
        state
            .assignment_policy_exceptions
            .insert(exception_key, command.exception.clone());
        if let Err(error) =
            apply_memory_assignment_timing_update(&mut state, tenant, command.assignment, None)
        {
            *state = snapshot;
            return Err(error);
        }
        state.assignment_revisions.insert(assignment_key, revision);
        Ok(StoredAssignmentPolicyException {
            exception: command.exception,
            assignment_revision: revision,
        })
    }

    async fn delete_assignment_policy_exception(
        &self,
        context: TenantContext,
        command: DeleteAssignmentPolicyExceptionCommand,
    ) -> Result<AssignmentRevision, StoreError> {
        let tenant = context.tenant_id();
        let assignment_key = (tenant, command.assignment);
        let mut state = self.write_state()?;
        let assignment = state
            .assignments
            .get(&assignment_key)
            .cloned()
            .ok_or(StoreError::NotFound)?;
        if assignment.course_id != command.course {
            return Err(StoreError::NotFound);
        }
        require_course_records_accessible(&state, tenant, command.course)?;
        let course = state
            .courses
            .get(&(tenant, command.course))
            .ok_or(StoreError::NotFound)?;
        if course.role_for(command.actor) != Some(CourseRole::Instructor) {
            return Err(StoreError::NotFound);
        }
        let current_revision = state
            .assignment_revisions
            .get(&assignment_key)
            .copied()
            .ok_or(StoreError::NotFound)?;
        if current_revision != command.expected_revision {
            return Err(StoreError::Conflict);
        }
        let exception_key = state
            .assignment_policy_exceptions
            .iter()
            .find_map(|(key @ (record_tenant, record_assignment, _), exception)| {
                (*record_tenant == tenant
                    && *record_assignment == command.assignment
                    && exception.id == command.exception)
                    .then_some(*key)
            })
            .ok_or(StoreError::NotFound)?;
        let revision = current_revision.next()?;
        let snapshot = state.clone();
        state.assignment_policy_exceptions.remove(&exception_key);
        if let Err(error) =
            apply_memory_assignment_timing_update(&mut state, tenant, command.assignment, None)
        {
            *state = snapshot;
            return Err(error);
        }
        state.assignment_revisions.insert(assignment_key, revision);
        Ok(revision)
    }

    async fn get_assignment_policy_exception(
        &self,
        context: TenantContext,
        assignment: AssignmentId,
        exception: AssignmentPolicyExceptionId,
    ) -> Result<Option<StoredAssignmentPolicyException>, StoreError> {
        let state = self.read_state()?;
        let tenant = context.tenant_id();
        let record = state.assignment_policy_exceptions.iter().find_map(
            |((record_tenant, record_assignment, _), record)| {
                (*record_tenant == tenant
                    && *record_assignment == assignment
                    && record.id == exception)
                    .then_some(record.clone())
            },
        );
        let Some(exception) = record else {
            return Ok(None);
        };
        Ok(Some(StoredAssignmentPolicyException {
            exception,
            assignment_revision: state
                .assignment_revisions
                .get(&(tenant, assignment))
                .copied()
                .ok_or(StoreError::NotFound)?,
        }))
    }

    async fn resolve_assignment_timing(
        &self,
        context: TenantContext,
        assignment: AssignmentId,
        student: StudentId,
    ) -> Result<Option<ResolvedAssignmentTiming>, StoreError> {
        let state = self.read_state()?;
        let tenant = context.tenant_id();
        let Some(record) = state.assignments.get(&(tenant, assignment)) else {
            return Ok(None);
        };
        let Some(enrollment) = state.enrollments.values().find(|enrollment| {
            enrollment.tenant == tenant
                && enrollment.assignment == assignment
                && enrollment.student == student
        }) else {
            return Ok(None);
        };
        let resolved =
            memory_resolved_assignment_policy(&state, tenant, assignment, enrollment, None)?;
        Ok(Some(ResolvedAssignmentTiming {
            tenant,
            course: record.course_id,
            assignment,
            student,
            policy: resolved.policy,
            contributors: resolved.contributors,
            revision: state
                .assignment_revisions
                .get(&(tenant, assignment))
                .copied()
                .ok_or(StoreError::NotFound)?,
        }))
    }

    async fn get_attempt_resolved_timing(
        &self,
        context: TenantContext,
        attempt: QuestionAttemptId,
    ) -> Result<Option<ResolvedAttemptTiming>, StoreError> {
        let state = self.read_state()?;
        let tenant = context.tenant_id();
        let Some(resolution) = state.attempt_timing_resolution.get(&(tenant, attempt)) else {
            return Ok(None);
        };
        Ok(Some(ResolvedAttemptTiming {
            attempt,
            policy: resolution.policy,
            contributors: resolution.contributors.clone(),
        }))
    }

    async fn delete_and_regrade_assignment_item(
        &self,
        context: TenantContext,
        command: DeleteAndRegradeAssignmentItemCommand,
    ) -> Result<StoredAssignment, StoreError> {
        let stored = self
            .get_assignment_for_edit(context, command.assignment)
            .await?
            .ok_or(StoreError::NotFound)?;
        if stored.record.course_id != command.course {
            return Err(StoreError::NotFound);
        }
        if stored.revision != command.expected_revision {
            return Err(StoreError::Conflict);
        }
        let Some(update) = delete_and_regrade_update(&stored, command.item)? else {
            return Ok(stored);
        };
        self.replace_assignment(
            context,
            command.course,
            command.assignment,
            command.expected_revision,
            update,
        )
        .await
    }

    async fn get_assignment_for_edit(
        &self,
        context: TenantContext,
        assignment: AssignmentId,
    ) -> Result<Option<StoredAssignment>, StoreError> {
        let state = self.read_state()?;
        let key = (context.tenant_id(), assignment);
        let Some(record) = state.assignments.get(&key).cloned() else {
            return Ok(None);
        };
        let revision = state
            .assignment_revisions
            .get(&key)
            .copied()
            .ok_or_else(|| {
                StoreError::Unavailable(
                    "assignment revision is missing from memory state".to_string(),
                )
            })?;
        let (scoring_generation, scoring_status) =
            state.assignment_scoring.get(&key).copied().ok_or_else(|| {
                StoreError::Unavailable(
                    "assignment scoring state is missing from memory state".to_string(),
                )
            })?;
        Ok(Some(StoredAssignment {
            record,
            revision,
            scoring_generation,
            scoring_status,
        }))
    }

    async fn get_assignment(
        &self,
        context: TenantContext,
        assignment: AssignmentId,
    ) -> Result<Option<AssignmentRecord>, StoreError> {
        let state = self.read_state()?;
        let Some(record) = state
            .assignments
            .get(&(context.tenant_id(), assignment))
            .cloned()
        else {
            return Ok(None);
        };
        Ok(Some(record))
    }

    async fn list_assignments(
        &self,
        context: TenantContext,
        course: CourseId,
        page: PageRequest,
    ) -> Result<Page<AssignmentRecord>, StoreError> {
        let state = self.read_state()?;
        if !state.courses.contains_key(&(context.tenant_id(), course)) {
            return Err(StoreError::NotFound);
        }
        let records = state
            .assignments
            .iter()
            .filter(|((tenant, _), record)| {
                *tenant == context.tenant_id() && record.course_id == course
            })
            .map(|((_, assignment), record)| (assignment.to_string(), record.clone()))
            .collect();
        Ok(page_records(records, &page))
    }

    async fn create_enrollment(
        &self,
        context: TenantContext,
        enrollment: AssignmentEnrollment,
    ) -> Result<(), StoreError> {
        ensure_tenant(context, enrollment.tenant)?;
        let mut state = self.write_state()?;
        let assignment = state
            .assignments
            .get(&(enrollment.tenant, enrollment.assignment))
            .ok_or_else(|| {
                StoreError::InvalidRecord("enrollment references a missing assignment".to_string())
            })?;
        require_course_records_accessible(&state, context.tenant_id(), assignment.course_id)?;
        let course = state
            .courses
            .get(&(enrollment.tenant, assignment.course_id))
            .ok_or(StoreError::NotFound)?;
        if course.role_for(enrollment.user) != Some(CourseRole::Student) {
            return Err(StoreError::InvalidRecord(
                "enrollment user must be a student member of the assignment course".to_string(),
            ));
        }
        let key = (enrollment.tenant, enrollment.id);
        if state.enrollments.contains_key(&key) {
            return Err(StoreError::AlreadyExists);
        }
        if state.enrollments.values().any(|existing| {
            existing.tenant == enrollment.tenant
                && existing.assignment == enrollment.assignment
                && existing.user == enrollment.user
        }) {
            return Err(StoreError::AlreadyExists);
        }
        state.summaries.insert(
            key,
            StudentAssignmentSummary::empty(enrollment.tenant, enrollment.id),
        );
        state.enrollments.insert(key, enrollment);
        Ok(())
    }

    async fn get_enrollment(
        &self,
        context: TenantContext,
        enrollment: EnrollmentId,
    ) -> Result<Option<AssignmentEnrollment>, StoreError> {
        let state = self.read_state()?;
        let Some(record) = state
            .enrollments
            .get(&(context.tenant_id(), enrollment))
            .cloned()
        else {
            return Ok(None);
        };
        let assignment = assignment_record(&state, context.tenant_id(), record.assignment)?;
        if !course_records_accessible(&state, context.tenant_id(), assignment.course_id) {
            return Ok(None);
        }
        Ok(Some(record))
    }

    async fn start_or_resume_run(
        &self,
        context: TenantContext,
        actor: UserId,
        assignment_id: AssignmentId,
        proposed_run: RunId,
    ) -> Result<AssignmentRun, StoreError> {
        let mut state = self.write_state()?;
        let tenant = context.tenant_id();
        let enrollment = state
            .enrollments
            .values()
            .find(|enrollment| {
                enrollment.tenant == tenant
                    && enrollment.assignment == assignment_id
                    && enrollment.user == actor
            })
            .cloned()
            .ok_or(StoreError::NotFound)?;
        if let Some(active) = state.runs.values().find(|run| {
            run.tenant == tenant && run.enrollment == enrollment.id && run.completed_at.is_none()
        }) {
            return Ok(active.clone());
        }
        if state.runs.contains_key(&(tenant, proposed_run)) {
            return Err(StoreError::AlreadyExists);
        }
        let assignment = assignment_record(&state, tenant, assignment_id)?;
        require_course_records_accessible(&state, tenant, assignment.course_id)?;
        let timing =
            memory_resolved_assignment_policy(&state, tenant, assignment_id, &enrollment, None)?
                .policy;
        let now = state.authoritative_time;
        if !timing.visible {
            return Err(StoreError::NotFound);
        }
        if timing
            .available_at
            .is_some_and(|available_at| now < available_at)
        {
            return Err(StoreError::InvalidRecord(
                "assignment is not yet available".to_string(),
            ));
        }
        if timing.closes_at.is_some_and(|closes_at| now >= closes_at) {
            return Err(StoreError::InvalidRecord(
                "assignment is closed".to_string(),
            ));
        }
        if timing.late_submission == question_model::LateSubmissionPolicy::Reject
            && timing.due_at.is_some_and(|due_at| now > due_at)
        {
            return Err(StoreError::InvalidRecord(
                "assignment due date has passed".to_string(),
            ));
        }
        let existing_run_count = state
            .runs
            .values()
            .filter(|run| run.tenant == tenant && run.enrollment == enrollment.id)
            .count();
        if timing
            .attempt_limit
            .is_some_and(|limit| existing_run_count >= limit as usize)
        {
            return Err(StoreError::InvalidRecord(
                "assignment attempt limit has been reached".to_string(),
            ));
        }
        let previous = state
            .summaries
            .get(&(tenant, enrollment.id))
            .cloned()
            .ok_or(StoreError::NotFound)?;
        if !continued_practice_allows_run(&previous, assignment.policies.continued_practice) {
            return Err(StoreError::InvalidRecord(
                "continued-practice policy does not permit another run".to_string(),
            ));
        }
        let run_number = state
            .runs
            .values()
            .filter(|run| run.tenant == tenant && run.enrollment == enrollment.id)
            .map(|run| run.run_number)
            .max()
            .unwrap_or(0)
            .checked_add(1)
            .ok_or_else(|| StoreError::InvalidRecord("run number overflow".to_string()))?;
        let run = AssignmentRun {
            id: proposed_run,
            tenant,
            enrollment: enrollment.id,
            run_number,
            started_at: state.authoritative_time,
            completed_at: None,
            score: None,
            mode: match enrollment.status() {
                EnrollmentStatus::InProgress => RunMode::Assigned,
                EnrollmentStatus::Completed => RunMode::Practice,
            },
            variation: assignment.policies.variation,
        };
        let next = project_summary(
            &previous,
            summary_transition(&ActivityTransition::StartRun { run: run.clone() }),
            grade_policy(&assignment),
        )?;
        let run_items = select_assignment_run_items(&assignment, run.id)?;
        state.runs.insert((tenant, run.id), run.clone());
        state.run_items.insert((tenant, run.id), run_items);
        state.summaries.insert((tenant, enrollment.id), next);
        Ok(run)
    }

    async fn assignment_run_items(
        &self,
        context: TenantContext,
        run: RunId,
    ) -> Result<Vec<AssignmentRunItem>, StoreError> {
        let state = self.read_state()?;
        if !state.runs.contains_key(&(context.tenant_id(), run)) {
            return Err(StoreError::NotFound);
        }
        Ok(state
            .run_items
            .get(&(context.tenant_id(), run))
            .cloned()
            .unwrap_or_default())
    }

    async fn issue_or_resume_question_attempt(
        &self,
        context: TenantContext,
        command: IssueQuestionAttemptCommand,
    ) -> Result<QuestionAttempt, StoreError> {
        let mut state = self.write_state()?;
        let tenant = context.tenant_id();
        let run = state
            .runs
            .get(&(tenant, command.run))
            .cloned()
            .ok_or(StoreError::NotFound)?;
        if run.completed_at.is_some() || run.score.is_some() {
            return Err(StoreError::InvalidRecord(
                "a completed run cannot issue another question".to_string(),
            ));
        }
        let enrollment = enrollment_record(&state, tenant, run.enrollment)?;
        if enrollment.user != command.actor {
            return Err(StoreError::Forbidden);
        }
        let assignment = assignment_record(&state, tenant, enrollment.assignment)?;
        require_course_records_accessible(&state, tenant, assignment.course_id)?;
        let run_items = state
            .run_items
            .get(&(tenant, command.run))
            .ok_or(StoreError::NotFound)?;
        validate_assignment_position(run_items, &command)?;

        let prefetched = command.prefetched.as_ref();
        if let Some(prefetched) = prefetched {
            let key = (
                tenant,
                command.run,
                prefetched.predecessor,
                command.assignment_position,
            );
            if prefetched.tenant != tenant
                || prefetched.run != command.run
                || command.predecessor_submission != Some(prefetched.predecessor)
                || prefetched.assignment_position != command.assignment_position
                || prefetched.problem != command.problem
                || prefetched.question_version != command.question_version
                || state.prefetched_questions.get(&key) != Some(prefetched)
                || !state
                    .submissions
                    .contains_key(&(tenant, prefetched.predecessor))
            {
                return Err(StoreError::Conflict);
            }
        }

        let unresolved = state
            .attempts
            .values()
            .filter(|attempt| {
                attempt.tenant == tenant
                    && attempt.run == run.id
                    && projected_attempt(&state, tenant, attempt).status
                        == AttemptStatus::InProgress
            })
            .max_by_key(|attempt| (attempt.timer.issued_at, attempt.id));
        if let Some(active) = unresolved.cloned() {
            if active.assignment_position == command.assignment_position {
                if let Some(predecessor) = command.predecessor_submission {
                    if state
                        .attempts
                        .get(&(tenant, predecessor))
                        .is_none_or(|value| value.run != command.run)
                    {
                        return Err(StoreError::Conflict);
                    }
                    if !state.submissions.contains_key(&(tenant, predecessor)) {
                        return Err(StoreError::Conflict);
                    }
                    match state.submission_next_attempts.get(&(tenant, predecessor)) {
                        Some(Some(existing)) if *existing != active.id => {
                            return Err(StoreError::Conflict);
                        }
                        Some(None) => return Err(StoreError::Conflict),
                        _ => {
                            state
                                .submission_next_attempts
                                .insert((tenant, predecessor), Some(active.id));
                        }
                    }
                }
                return Ok(projected_attempt(&state, tenant, &active));
            }
            return Err(StoreError::InvalidRecord(
                "another question attempt is already active in this run".to_string(),
            ));
        }
        let latest_for_position = state
            .attempts
            .values()
            .filter(|attempt| {
                attempt.tenant == tenant
                    && attempt.run == run.id
                    && attempt.assignment_position == command.assignment_position
                    && !matches!(
                        projected_attempt(&state, tenant, attempt).status,
                        AttemptStatus::Cleared | AttemptStatus::Exempt
                    )
            })
            .max_by_key(|attempt| (attempt.timer.issued_at, attempt.id));
        if latest_for_position.is_some_and(|latest| {
            projected_attempt(&state, tenant, latest)
                .result
                .is_some_and(|result| result.correct)
        }) {
            return Err(StoreError::InvalidRecord(
                "a correct question position cannot be retried".to_string(),
            ));
        }
        if state.attempts.contains_key(&(tenant, command.attempt)) {
            return Err(StoreError::AlreadyExists);
        }
        let (seed, parameter_hash, provenance) = match prefetched {
            Some(value) => (
                value.seed,
                value.parameter_hash.clone(),
                value.provenance.clone(),
            ),
            None => (
                command.seed,
                command.parameter_hash.clone(),
                command.provenance.clone(),
            ),
        };
        if parameter_hash.trim().is_empty() || provenance.rendered_question_sha256.trim().is_empty()
        {
            return Err(StoreError::InvalidRecord(
                "issued attempt hashes must not be empty".to_string(),
            ));
        }
        let question = state
            .published
            .get(&(command.problem, command.question_version))
            .ok_or(StoreError::NotFound)?;
        let authored_timer = issued_timer(
            state.authoritative_time,
            &run,
            question.question.timing_policy,
        )?;
        let authored_grace_seconds = timing_policy_grace_seconds(question.question.timing_policy);
        let resolved_assignment_timing =
            memory_resolved_assignment_policy(&state, tenant, assignment.id, &enrollment, None)?;
        let (effective_deadline, effective_grace_seconds, auto_submit_at) =
            resolved_memory_attempt_timing(
                resolved_assignment_timing.policy,
                &run,
                authored_timer.deadline,
                authored_grace_seconds,
            )?;
        if effective_deadline.is_some_and(|deadline| deadline < state.authoritative_time)
            || auto_submit_at.is_some_and(|deadline| deadline <= state.authoritative_time)
        {
            return Err(StoreError::TimedOut);
        }
        let timer = AttemptTimerRecord {
            deadline: effective_deadline,
            ..authored_timer
        };
        let timing_generation = 1;
        let timing_job = if let Some(available_at) = auto_submit_at {
            let job = loop {
                let candidate = JobId::generate()?;
                if !state.jobs.contains_key(&candidate) {
                    break candidate;
                }
            };
            Some((
                job,
                StoredJob {
                    tenant,
                    payload: JobPayload::AutoSubmitAttempt {
                        attempt: command.attempt,
                        timing_generation,
                    },
                    state: JobState::Ready,
                    available_at,
                    lease_token: None,
                    lease_expires_at: None,
                    attempt_count: 0,
                    max_attempts: 10,
                    failure: None,
                },
            ))
        } else {
            None
        };
        let attempt = QuestionAttempt {
            id: command.attempt,
            tenant,
            run: run.id,
            problem: command.problem,
            question_version: command.question_version,
            assignment_position: command.assignment_position,
            seed,
            parameter_hash,
            response: None,
            status: AttemptStatus::InProgress,
            result: None,
            timer,
            provenance,
        };
        if let Some(prefetched) = prefetched {
            state.prefetched_questions.remove(&(
                tenant,
                command.run,
                prefetched.predecessor,
                command.assignment_position,
            ));
        }
        if let Some(predecessor) = command.predecessor_submission {
            if state
                .attempts
                .get(&(tenant, predecessor))
                .is_none_or(|value| value.run != command.run)
            {
                return Err(StoreError::Conflict);
            }
            if !state.submissions.contains_key(&(tenant, predecessor)) {
                return Err(StoreError::Conflict);
            }
            match state.submission_next_attempts.get(&(tenant, predecessor)) {
                Some(Some(existing)) if *existing != attempt.id => {
                    return Err(StoreError::Conflict);
                }
                Some(None) => return Err(StoreError::Conflict),
                _ => {
                    state
                        .submission_next_attempts
                        .insert((tenant, predecessor), Some(attempt.id));
                }
            }
        }
        let timing_job_id = timing_job.as_ref().map(|(job, _)| *job);
        if let Some((job, queued)) = timing_job {
            state.jobs.insert(job, queued);
        }
        state.attempt_timing.insert(
            (tenant, attempt.id),
            MemoryAttemptTiming {
                assignment: assignment.id,
                authored_deadline: authored_timer.deadline,
                authored_grace_seconds,
                effective_deadline,
                effective_grace_seconds,
                auto_submit_at,
                generation: timing_generation,
                job: timing_job_id,
            },
        );
        state
            .attempt_timing_resolution
            .insert((tenant, attempt.id), resolved_assignment_timing);
        state.attempts.insert((tenant, attempt.id), attempt.clone());
        Ok(attempt)
    }

    async fn reserve_or_resume_prefetched_question(
        &self,
        context: TenantContext,
        command: ReservePrefetchedQuestionCommand,
    ) -> Result<PrefetchedQuestion, StoreError> {
        let mut state = self.write_state()?;
        let tenant = context.tenant_id();
        let reservation = command.reservation;
        if reservation.tenant != tenant
            || reservation.parameter_hash.trim().is_empty()
            || reservation
                .provenance
                .rendered_question_sha256
                .trim()
                .is_empty()
        {
            return Err(StoreError::InvalidRecord(
                "invalid prefetch reservation".to_string(),
            ));
        }
        let run = state
            .runs
            .get(&(tenant, reservation.run))
            .ok_or(StoreError::NotFound)?;
        if run.completed_at.is_some() || run.score.is_some() {
            return Err(StoreError::Conflict);
        }
        let enrollment = enrollment_record(&state, tenant, run.enrollment)?;
        if enrollment.user != command.actor {
            return Err(StoreError::Forbidden);
        }
        let predecessor = state
            .attempts
            .get(&(tenant, reservation.predecessor))
            .ok_or(StoreError::NotFound)?;
        if predecessor.run != reservation.run
            || state.submissions.contains_key(&(tenant, predecessor.id))
        {
            return Err(StoreError::Conflict);
        }
        let assignment = assignment_record(&state, tenant, enrollment.assignment)?;
        require_course_records_accessible(&state, tenant, assignment.course_id)?;
        let expected = assignment
            .active_item_at(reservation.assignment_position)
            .ok_or_else(|| {
                StoreError::InvalidRecord("prefetch position is outside the assignment".to_string())
            })?;
        if expected.reference.problem != reservation.problem
            || expected.reference.version != reservation.question_version
        {
            return Err(StoreError::InvalidRecord(
                "prefetch identity does not match assignment position".to_string(),
            ));
        }
        if state.attempts.values().any(|attempt| {
            attempt.tenant == tenant
                && attempt.run == reservation.run
                && attempt.assignment_position == reservation.assignment_position
        }) {
            return Err(StoreError::Conflict);
        }
        let key = (
            tenant,
            reservation.run,
            reservation.predecessor,
            reservation.assignment_position,
        );
        if let Some(existing) = state.prefetched_questions.get(&key) {
            return if existing == &reservation {
                Ok(existing.clone())
            } else {
                Err(StoreError::Conflict)
            };
        }
        state.prefetched_questions.insert(key, reservation.clone());
        Ok(reservation)
    }

    async fn get_prefetched_question(
        &self,
        context: TenantContext,
        actor: UserId,
        run: RunId,
        predecessor: QuestionAttemptId,
        assignment_position: u32,
    ) -> Result<Option<PrefetchedQuestion>, StoreError> {
        let state = self.read_state()?;
        let run_record = state
            .runs
            .get(&(context.tenant_id(), run))
            .ok_or(StoreError::NotFound)?;
        let enrollment = enrollment_record(&state, context.tenant_id(), run_record.enrollment)?;
        if enrollment.user != actor {
            return Err(StoreError::Forbidden);
        }
        let assignment = assignment_record(&state, context.tenant_id(), enrollment.assignment)?;
        require_course_records_accessible(&state, context.tenant_id(), assignment.course_id)?;
        Ok(state
            .prefetched_questions
            .get(&(context.tenant_id(), run, predecessor, assignment_position))
            .cloned())
    }

    async fn submission_next_attempt(
        &self,
        context: TenantContext,
        actor: UserId,
        predecessor: QuestionAttemptId,
    ) -> Result<SubmissionNextAttempt, StoreError> {
        let state = self.read_state()?;
        let attempt = state
            .attempts
            .get(&(context.tenant_id(), predecessor))
            .ok_or(StoreError::NotFound)?;
        let run = state
            .runs
            .get(&(context.tenant_id(), attempt.run))
            .ok_or(StoreError::NotFound)?;
        let enrollment = enrollment_record(&state, context.tenant_id(), run.enrollment)?;
        let assignment = assignment_record(&state, context.tenant_id(), enrollment.assignment)?;
        require_course_records_accessible(&state, context.tenant_id(), assignment.course_id)?;
        require_attempt_owner(&state, context.tenant_id(), attempt, actor)?;
        if !state
            .submissions
            .contains_key(&(context.tenant_id(), predecessor))
        {
            return Err(StoreError::Conflict);
        }
        Ok(
            match state
                .submission_next_attempts
                .get(&(context.tenant_id(), predecessor))
            {
                None => SubmissionNextAttempt::Pending,
                Some(None) => SubmissionNextAttempt::None,
                Some(Some(next)) => SubmissionNextAttempt::Issued(*next),
            },
        )
    }

    async fn pending_submission_for_run(
        &self,
        context: TenantContext,
        actor: UserId,
        run: RunId,
    ) -> Result<Option<QuestionAttemptId>, StoreError> {
        let state = self.read_state()?;
        let run_record = state
            .runs
            .get(&(context.tenant_id(), run))
            .ok_or(StoreError::NotFound)?;
        let enrollment = enrollment_record(&state, context.tenant_id(), run_record.enrollment)?;
        let assignment = assignment_record(&state, context.tenant_id(), enrollment.assignment)?;
        require_course_records_accessible(&state, context.tenant_id(), assignment.course_id)?;
        if enrollment.user != actor {
            return Err(StoreError::Forbidden);
        }
        let pending: Vec<_> = state
            .attempts
            .values()
            .filter(|attempt| {
                attempt.tenant == context.tenant_id()
                    && attempt.run == run
                    && state
                        .submissions
                        .contains_key(&(context.tenant_id(), attempt.id))
                    && !state
                        .submission_next_attempts
                        .contains_key(&(context.tenant_id(), attempt.id))
            })
            .map(|attempt| attempt.id)
            .take(2)
            .collect();
        match pending.as_slice() {
            [] => Ok(None),
            [id] => Ok(Some(*id)),
            _ => Err(StoreError::Conflict),
        }
    }

    async fn finalize_submission_next_attempt(
        &self,
        context: TenantContext,
        actor: UserId,
        predecessor: QuestionAttemptId,
        next: Option<QuestionAttemptId>,
    ) -> Result<(), StoreError> {
        let mut state = self.write_state()?;
        let tenant = context.tenant_id();
        let attempt = state
            .attempts
            .get(&(tenant, predecessor))
            .ok_or(StoreError::NotFound)?
            .clone();
        let run = state
            .runs
            .get(&(tenant, attempt.run))
            .ok_or(StoreError::NotFound)?;
        let enrollment = enrollment_record(&state, tenant, run.enrollment)?;
        let assignment = assignment_record(&state, tenant, enrollment.assignment)?;
        require_course_records_accessible(&state, tenant, assignment.course_id)?;
        require_attempt_owner(&state, tenant, &attempt, actor)?;
        if !state.submissions.contains_key(&(tenant, predecessor)) {
            return Err(StoreError::Conflict);
        }
        if let Some(next) = next {
            let next_attempt = state
                .attempts
                .get(&(tenant, next))
                .ok_or(StoreError::NotFound)?;
            if next_attempt.run != attempt.run {
                return Err(StoreError::Conflict);
            }
        }
        match state.submission_next_attempts.get(&(tenant, predecessor)) {
            Some(existing) if *existing != next => Err(StoreError::Conflict),
            _ => {
                state
                    .submission_next_attempts
                    .insert((tenant, predecessor), next);
                Ok(())
            }
        }
    }

    async fn list_question_attempts(
        &self,
        context: TenantContext,
        run: RunId,
        page: PageRequest,
    ) -> Result<Page<QuestionAttempt>, StoreError> {
        let state = self.read_state()?;
        let run_record = state
            .runs
            .get(&(context.tenant_id(), run))
            .ok_or(StoreError::NotFound)?;
        let enrollment = enrollment_record(&state, context.tenant_id(), run_record.enrollment)?;
        let assignment = assignment_record(&state, context.tenant_id(), enrollment.assignment)?;
        require_course_records_accessible(&state, context.tenant_id(), assignment.course_id)?;
        let records = state
            .attempts
            .values()
            .filter(|attempt| attempt.tenant == context.tenant_id() && attempt.run == run)
            .map(|attempt| {
                let projected = projected_attempt(&state, context.tenant_id(), attempt);
                (
                    format!(
                        "{:010}/{:020}/{}",
                        projected.assignment_position,
                        projected.timer.issued_at.as_unix_millis(),
                        projected.id
                    ),
                    projected,
                )
            })
            .collect();
        Ok(page_records(records, &page))
    }

    async fn replay_submission(
        &self,
        context: TenantContext,
        actor: UserId,
        attempt_id: QuestionAttemptId,
        response: &StudentResponse,
        idempotency_key: &SubmissionIdempotencyKey,
    ) -> Result<Option<SubmissionRecord>, StoreError> {
        let state = self.read_state()?;
        let tenant = context.tenant_id();
        let attempt = state
            .attempts
            .get(&(tenant, attempt_id))
            .ok_or(StoreError::NotFound)?;
        let run = state
            .runs
            .get(&(tenant, attempt.run))
            .ok_or(StoreError::NotFound)?;
        let enrollment = enrollment_record(&state, tenant, run.enrollment)?;
        let assignment = assignment_record(&state, tenant, enrollment.assignment)?;
        require_course_records_accessible(&state, tenant, assignment.course_id)?;
        require_attempt_owner(&state, tenant, attempt, actor)?;
        let Some(stored) = state.submissions.get(&(tenant, attempt_id)) else {
            return Ok(None);
        };
        if &stored.key != idempotency_key || &stored.response != response {
            return Err(StoreError::Conflict);
        }
        Ok(Some(stored.record.clone()))
    }

    async fn submit_question_attempt(
        &self,
        context: TenantContext,
        command: SubmitQuestionAttemptCommand,
    ) -> Result<SubmissionRecord, StoreError> {
        let mut state = self.write_state()?;
        let tenant = context.tenant_id();
        let attempt = state
            .attempts
            .get(&(tenant, command.attempt))
            .ok_or(StoreError::NotFound)?;
        let run = state
            .runs
            .get(&(tenant, attempt.run))
            .ok_or(StoreError::NotFound)?;
        let enrollment = enrollment_record(&state, tenant, run.enrollment)?;
        let assignment = assignment_record(&state, tenant, enrollment.assignment)?;
        require_course_records_accessible(&state, tenant, assignment.course_id)?;
        submit_question_attempt_locked(&mut state, context, command)
    }

    async fn force_submit_attempt(
        &self,
        context: TenantContext,
        command: ForceSubmitAttemptCommand,
    ) -> Result<AttemptSupportRecord, StoreError> {
        let mut state = self.write_state()?;
        apply_memory_attempt_support(
            &mut state,
            context,
            command.action,
            command.actor,
            command.attempt,
            AttemptSupportAction::ForceSubmit,
        )
    }

    async fn clear_attempt(
        &self,
        context: TenantContext,
        command: ClearAttemptCommand,
    ) -> Result<AttemptSupportRecord, StoreError> {
        let mut state = self.write_state()?;
        apply_memory_attempt_support(
            &mut state,
            context,
            command.action,
            command.actor,
            command.attempt,
            AttemptSupportAction::Clear,
        )
    }

    async fn release_attempt_feedback(
        &self,
        context: TenantContext,
        command: ReleaseAttemptFeedbackCommand,
    ) -> Result<FeedbackReleaseRecord, StoreError> {
        let mut state = self.write_state()?;
        let tenant = context.tenant_id();
        let attempt = state
            .attempts
            .get(&(tenant, command.attempt))
            .cloned()
            .ok_or(StoreError::NotFound)?;
        let run = state
            .runs
            .get(&(tenant, attempt.run))
            .ok_or(StoreError::NotFound)?;
        let enrollment = enrollment_record(&state, tenant, run.enrollment)?;
        let assignment = assignment_record(&state, tenant, enrollment.assignment)?;
        require_course_records_accessible(&state, tenant, assignment.course_id)?;
        let course = state
            .courses
            .get(&(tenant, assignment.course_id))
            .ok_or(StoreError::NotFound)?;
        if course.role_for(command.actor) != Some(CourseRole::Instructor) {
            return Err(StoreError::NotFound);
        }
        if !state.submissions.contains_key(&(tenant, command.attempt)) {
            return Err(StoreError::NotFound);
        }
        let question = state
            .published
            .get(&(attempt.problem, attempt.question_version))
            .ok_or(StoreError::NotFound)?;
        if question.question.attempt_policy.feedback
            != question_model::run_policy::FeedbackDisclosure::OnRelease
        {
            return Err(StoreError::InvalidRecord(
                "feedback release requires an on-release question policy".to_string(),
            ));
        }
        if let Some(existing) = state.feedback_releases.get(&(tenant, command.attempt)) {
            return if existing.released_by == command.actor {
                Ok(existing.clone())
            } else {
                Err(StoreError::Conflict)
            };
        }
        let record = FeedbackReleaseRecord {
            tenant,
            attempt: command.attempt,
            released_by: command.actor,
            released_at: state.authoritative_time,
        };
        state
            .feedback_releases
            .insert((tenant, command.attempt), record.clone());
        Ok(record)
    }

    async fn get_attempt_feedback_release(
        &self,
        context: TenantContext,
        actor: UserId,
        attempt_id: QuestionAttemptId,
    ) -> Result<Option<FeedbackReleaseRecord>, StoreError> {
        let state = self.read_state()?;
        let tenant = context.tenant_id();
        let attempt = state
            .attempts
            .get(&(tenant, attempt_id))
            .ok_or(StoreError::NotFound)?;
        let run = state
            .runs
            .get(&(tenant, attempt.run))
            .ok_or(StoreError::NotFound)?;
        let enrollment = enrollment_record(&state, tenant, run.enrollment)?;
        let assignment = assignment_record(&state, tenant, enrollment.assignment)?;
        require_course_records_accessible(&state, tenant, assignment.course_id)?;
        let course = state
            .courses
            .get(&(tenant, assignment.course_id))
            .ok_or(StoreError::NotFound)?;
        if actor != enrollment.user && course.role_for(actor) != Some(CourseRole::Instructor) {
            return Err(StoreError::NotFound);
        }
        Ok(state.feedback_releases.get(&(tenant, attempt_id)).cloned())
    }

    async fn get_run_summary_page(
        &self,
        context: TenantContext,
        actor: UserId,
        run_id: RunId,
        page: PageRequest,
    ) -> Result<RunSummaryPageInput, StoreError> {
        let state = self.read_state()?;
        let tenant = context.tenant_id();
        let run = state
            .runs
            .get(&(tenant, run_id))
            .cloned()
            .ok_or(StoreError::NotFound)?;
        let enrollment = enrollment_record(&state, tenant, run.enrollment)?;
        let assignment = assignment_record(&state, tenant, enrollment.assignment)?;
        require_course_records_accessible(&state, tenant, assignment.course_id)?;
        let course = state
            .courses
            .get(&(tenant, assignment.course_id))
            .ok_or(StoreError::NotFound)?;
        if actor != enrollment.user && course.role_for(actor) != Some(CourseRole::Instructor) {
            return Err(StoreError::NotFound);
        }
        let summary = state
            .summaries
            .get(&(tenant, enrollment.id))
            .cloned()
            .ok_or(StoreError::NotFound)?;
        let after = page
            .after
            .as_ref()
            .map(|cursor| RunSummaryCursor::decode(cursor, tenant.as_uuid(), run.id.as_uuid()))
            .transpose()?;
        let mut rows = Vec::new();
        for attempt in state
            .attempts
            .values()
            .filter(|attempt| attempt.tenant == tenant && attempt.run == run.id)
        {
            let current = projected_attempt(&state, tenant, attempt);
            if actor == enrollment.user && current.status == AttemptStatus::Cleared {
                continue;
            }
            if actor == enrollment.user {
                let assignment_item = state
                    .run_items
                    .get(&(tenant, run.id))
                    .and_then(|items| {
                        items
                            .iter()
                            .find(|item| item.issued_position == attempt.assignment_position)
                    })
                    .map(|item| item.assignment_item)
                    .ok_or_else(|| {
                        StoreError::Unavailable(
                            "summary attempt has no immutable run item".to_string(),
                        )
                    })?;
                if assignment_item_is_retired(&assignment, assignment_item).ok_or_else(|| {
                    StoreError::Unavailable(
                        "summary run item has no current assignment tombstone".to_string(),
                    )
                })? {
                    continue;
                }
            }
            let key = RunSummaryCursor {
                assignment_position: attempt.assignment_position,
                attempt: attempt.id.as_uuid(),
            };
            if after.is_some_and(|cursor| key <= cursor) {
                continue;
            }
            let submitted = state
                .submissions
                .get(&(tenant, attempt.id))
                .map(|stored| &stored.record);
            let published = state
                .published
                .get(&(attempt.problem, attempt.question_version))
                .ok_or(StoreError::NotFound)?;
            rows.push((
                key,
                RunSummaryOutcomeInput {
                    attempt: current.id,
                    assignment_position: current.assignment_position,
                    submitted_at: current.timer.submitted_at,
                    response: current.response.clone(),
                    result: current.result,
                    feedback_policy: published.question.attempt_policy.feedback,
                    feedback: submitted.map(|record| record.feedback.clone()),
                    release: state.feedback_releases.get(&(tenant, current.id)).cloned(),
                },
            ));
        }
        rows.sort_by_key(|(key, _)| *key);
        let take = usize::from(page.size.get());
        let has_more = rows.len() > take;
        rows.truncate(take);
        let next_cursor = has_more
            .then(|| {
                rows.last()
                    .map(|(key, _)| key.encode(tenant.as_uuid(), run.id.as_uuid()))
            })
            .flatten();
        Ok(RunSummaryPageInput {
            run,
            practice_allowed: continued_practice_allows_run(
                &summary,
                assignment.policies.continued_practice,
            ),
            assignment,
            summary,
            outcomes: Page {
                items: rows.into_iter().map(|(_, item)| item).collect(),
                next_cursor,
            },
        })
    }

    async fn apply_activity_transition(
        &self,
        context: TenantContext,
        transition: ActivityTransition,
    ) -> Result<StudentAssignmentSummary, StoreError> {
        let mut state = self.write_state()?;
        let tenant = context.tenant_id();

        let (enrollment_id, assignment, domain_transition) = match &transition {
            ActivityTransition::StartRun { run } => {
                ensure_tenant(context, run.tenant)?;
                if run.run_number == 0 || run.completed_at.is_some() || run.score.is_some() {
                    return Err(StoreError::InvalidRecord(
                        "new run must be one-based and incomplete".to_string(),
                    ));
                }
                if state.runs.contains_key(&(tenant, run.id)) {
                    return Err(StoreError::AlreadyExists);
                }
                let enrollment = enrollment_record(&state, tenant, run.enrollment)?;
                let assignment = assignment_record(&state, tenant, enrollment.assignment)?;
                require_course_records_accessible(&state, tenant, assignment.course_id)?;
                let expected_mode = match enrollment.status() {
                    EnrollmentStatus::InProgress => RunMode::Assigned,
                    EnrollmentStatus::Completed => RunMode::Practice,
                };
                if run.mode != expected_mode {
                    return Err(StoreError::InvalidRecord(format!(
                        "run mode must be {expected_mode:?} for this enrollment"
                    )));
                }
                if run.variation != assignment.policies.variation {
                    return Err(StoreError::InvalidRecord(
                        "run variation must match its assignment policy".to_string(),
                    ));
                }
                if state.runs.values().any(|existing| {
                    existing.tenant == tenant
                        && existing.enrollment == run.enrollment
                        && existing.completed_at.is_none()
                }) {
                    return Err(StoreError::InvalidRecord(
                        "an enrollment cannot have two in-progress runs".to_string(),
                    ));
                }
                let expected_run_number = state
                    .runs
                    .values()
                    .filter(|existing| {
                        existing.tenant == tenant && existing.enrollment == run.enrollment
                    })
                    .map(|existing| existing.run_number)
                    .max()
                    .unwrap_or(0)
                    .checked_add(1)
                    .ok_or_else(|| StoreError::InvalidRecord("run number overflow".to_string()))?;
                if run.run_number != expected_run_number {
                    return Err(StoreError::InvalidRecord(format!(
                        "run number must be the next one-based value {expected_run_number}"
                    )));
                }
                (enrollment.id, assignment, summary_transition(&transition))
            }
            ActivityTransition::RecordQuestionAttempt { attempt } => {
                ensure_tenant(context, attempt.tenant)?;
                if state.attempts.contains_key(&(tenant, attempt.id)) {
                    return Err(StoreError::AlreadyExists);
                }
                let run = state
                    .runs
                    .get(&(tenant, attempt.run))
                    .ok_or(StoreError::NotFound)?;
                if run.completed_at.is_some() || run.score.is_some() {
                    return Err(StoreError::InvalidRecord(
                        "question attempts cannot be added to a completed run".to_string(),
                    ));
                }
                let enrollment = enrollment_record(&state, tenant, run.enrollment)?;
                let assignment = assignment_record(&state, tenant, enrollment.assignment)?;
                require_course_records_accessible(&state, tenant, assignment.course_id)?;
                let matches_run_item =
                    state
                        .run_items
                        .get(&(tenant, attempt.run))
                        .is_some_and(|items| {
                            items.iter().any(|item| {
                                item.issued_position == attempt.assignment_position
                                    && item.reference.problem == attempt.problem
                                    && item.reference.version == attempt.question_version
                            })
                        });
                if !matches_run_item {
                    return Err(StoreError::InvalidRecord(
                        "question attempt must match an immutable run item".to_string(),
                    ));
                }
                (enrollment.id, assignment, summary_transition(&transition))
            }
            ActivityTransition::CompleteRun { run, .. } => {
                let run_record = state
                    .runs
                    .get(&(tenant, *run))
                    .ok_or(StoreError::NotFound)?;
                if run_record.completed_at.is_some() || run_record.score.is_some() {
                    return Err(StoreError::InvalidRecord(
                        "completed run cannot be completed again".to_string(),
                    ));
                }
                let enrollment = enrollment_record(&state, tenant, run_record.enrollment)?;
                (
                    enrollment.id,
                    {
                        let assignment = assignment_record(&state, tenant, enrollment.assignment)?;
                        require_course_records_accessible(&state, tenant, assignment.course_id)?;
                        assignment
                    },
                    summary_transition(&transition),
                )
            }
        };

        let summary_key = (tenant, enrollment_id);
        let previous = state
            .summaries
            .get(&summary_key)
            .cloned()
            .ok_or(StoreError::NotFound)?;
        let grade = grade_policy(&assignment);
        if matches!(&transition, ActivityTransition::StartRun { .. })
            && !continued_practice_allows_run(&previous, assignment.policies.continued_practice)
        {
            return Err(StoreError::InvalidRecord(
                "continued-practice policy does not permit another run".to_string(),
            ));
        }
        let next = project_summary(&previous, domain_transition, grade)?;

        match transition {
            ActivityTransition::StartRun { run } => {
                let run_items = select_assignment_run_items(&assignment, run.id)?;
                state.run_items.insert((tenant, run.id), run_items);
                state.runs.insert((tenant, run.id), run);
            }
            ActivityTransition::RecordQuestionAttempt { attempt } => {
                state.attempts.insert((tenant, attempt.id), *attempt);
            }
            ActivityTransition::CompleteRun { run, score, at } => {
                {
                    let run_record = state
                        .runs
                        .get_mut(&(tenant, run))
                        .ok_or(StoreError::NotFound)?;
                    run_record.completed_at = Some(at);
                    run_record.score = Some(score);
                }
                let enrollment = state
                    .enrollments
                    .get_mut(&summary_key)
                    .ok_or(StoreError::NotFound)?;
                project_enrollment_completion(enrollment, &previous, grade, run, score, at);
            }
        }
        state.summaries.insert(summary_key, next.clone());
        Ok(next)
    }

    async fn get_run(
        &self,
        context: TenantContext,
        run: RunId,
    ) -> Result<Option<AssignmentRun>, StoreError> {
        let state = self.read_state()?;
        let Some(record) = state.runs.get(&(context.tenant_id(), run)).cloned() else {
            return Ok(None);
        };
        let enrollment = enrollment_record(&state, context.tenant_id(), record.enrollment)?;
        let assignment = assignment_record(&state, context.tenant_id(), enrollment.assignment)?;
        if !course_records_accessible(&state, context.tenant_id(), assignment.course_id) {
            return Ok(None);
        }
        Ok(Some(record))
    }

    async fn list_runs(
        &self,
        context: TenantContext,
        enrollment: EnrollmentId,
        page: PageRequest,
    ) -> Result<Page<AssignmentRun>, StoreError> {
        let state = self.read_state()?;
        let enrollment_record = state
            .enrollments
            .get(&(context.tenant_id(), enrollment))
            .ok_or(StoreError::NotFound)?;
        let assignment =
            assignment_record(&state, context.tenant_id(), enrollment_record.assignment)?;
        require_course_records_accessible(&state, context.tenant_id(), assignment.course_id)?;
        let records = state
            .runs
            .iter()
            .filter(|((tenant, _), run)| {
                *tenant == context.tenant_id() && run.enrollment == enrollment
            })
            .map(|((_, run_id), run)| (format!("{:010}/{run_id}", run.run_number), run.clone()))
            .collect();
        Ok(page_records(records, &page))
    }

    async fn get_question_attempt(
        &self,
        context: TenantContext,
        attempt: QuestionAttemptId,
    ) -> Result<Option<QuestionAttempt>, StoreError> {
        let state = self.read_state()?;
        let Some(record) = state.attempts.get(&(context.tenant_id(), attempt)) else {
            return Ok(None);
        };
        let run = state
            .runs
            .get(&(context.tenant_id(), record.run))
            .ok_or(StoreError::NotFound)?;
        let enrollment = enrollment_record(&state, context.tenant_id(), run.enrollment)?;
        let assignment = assignment_record(&state, context.tenant_id(), enrollment.assignment)?;
        if !course_records_accessible(&state, context.tenant_id(), assignment.course_id) {
            return Ok(None);
        }
        Ok(Some(projected_attempt(&state, context.tenant_id(), record)))
    }

    async fn get_summary(
        &self,
        context: TenantContext,
        enrollment: EnrollmentId,
    ) -> Result<Option<StudentAssignmentSummary>, StoreError> {
        let state = self.read_state()?;
        let Some(record) = state
            .summaries
            .get(&(context.tenant_id(), enrollment))
            .cloned()
        else {
            return Ok(None);
        };
        let enrollment_record = state
            .enrollments
            .get(&(context.tenant_id(), enrollment))
            .ok_or(StoreError::NotFound)?;
        let assignment =
            assignment_record(&state, context.tenant_id(), enrollment_record.assignment)?;
        if !course_records_accessible(&state, context.tenant_id(), assignment.course_id) {
            return Ok(None);
        }
        Ok(Some(record))
    }

    async fn list_gradebook_rows(
        &self,
        context: TenantContext,
        course: CourseId,
        page: PageRequest,
    ) -> Result<Page<question_model::GradebookSummaryRow>, StoreError> {
        let state = self.read_state()?;
        let tenant = context.tenant_id();
        require_course_records_accessible(&state, tenant, course)?;
        let mut records = state
            .enrollments
            .iter()
            .filter_map(|((row_tenant, enrollment_id), enrollment)| {
                if *row_tenant != tenant {
                    return None;
                }
                let assignment = state.assignments.get(&(tenant, enrollment.assignment))?;
                if assignment.course_id != course {
                    return None;
                }
                let summary = state.summaries.get(&(tenant, *enrollment_id))?.clone();
                Some((
                    GradebookCursor {
                        assignment: assignment.id.as_uuid(),
                        enrollment: enrollment.id.as_uuid(),
                    },
                    question_model::GradebookSummaryRow {
                        tenant,
                        course_id: course,
                        enrollment_id: enrollment.id,
                        student_id: enrollment.student,
                        assignment_id: assignment.id,
                        assignment_title: assignment.title.clone(),
                        summary,
                    },
                ))
            })
            .collect::<Vec<_>>();
        let cursor = page
            .after
            .as_ref()
            .map(GradebookCursor::decode)
            .transpose()?;
        records.sort_by_key(|(key, _)| *key);
        let mut selected = records
            .into_iter()
            .filter(|(key, _)| cursor.is_none_or(|after| *key > after))
            .take(usize::from(page.size.get()) + 1)
            .collect::<Vec<_>>();
        let has_more = selected.len() > usize::from(page.size.get());
        if has_more {
            selected.pop();
        }
        let next_cursor = has_more.then(|| {
            selected
                .last()
                .map(|(key, _)| key.encode())
                .expect("a nonempty page precedes a following page")
        });
        Ok(Page {
            items: selected.into_iter().map(|(_, row)| row).collect(),
            next_cursor,
        })
    }
}

fn submit_question_attempt_locked(
    state: &mut State,
    context: TenantContext,
    command: SubmitQuestionAttemptCommand,
) -> Result<SubmissionRecord, StoreError> {
    let tenant = context.tenant_id();
    let base = state
        .attempts
        .get(&(tenant, command.attempt))
        .cloned()
        .ok_or(StoreError::NotFound)?;
    require_attempt_owner(state, tenant, &base, command.actor)?;
    if let Some(stored) = state.submissions.get(&(tenant, command.attempt)) {
        return if stored.key == command.idempotency_key && stored.response == command.response {
            Ok(stored.record.clone())
        } else {
            Err(StoreError::Conflict)
        };
    }
    if projected_attempt(state, tenant, &base).status != AttemptStatus::InProgress {
        return Err(StoreError::Conflict);
    }
    let feedback = private_feedback_record(command.feedback.clone())?;
    let mut run = state
        .runs
        .get(&(tenant, base.run))
        .cloned()
        .ok_or(StoreError::NotFound)?;
    if run.completed_at.is_some() || run.score.is_some() {
        return Err(StoreError::Conflict);
    }
    let mut enrollment = enrollment_record(state, tenant, run.enrollment)?;
    let assignment = assignment_record(state, tenant, enrollment.assignment)?;
    let authored_policy = state
        .published
        .get(&(base.problem, base.question_version))
        .ok_or(StoreError::NotFound)?
        .question
        .timing_policy;
    crate::validate_attempt_result(command.result)?;
    let submitted_at = state.authoritative_time;
    let mut submitted = projected_attempt(state, tenant, &base);
    submitted.response = Some(command.response.clone());
    submitted.status = AttemptStatus::Submitted;
    submitted.result = Some(command.result);
    submitted.timer.submitted_at = Some(submitted_at);
    let effective_policy =
        state
            .attempt_timing
            .get(&(tenant, command.attempt))
            .map_or(authored_policy, |timing| {
                timing
                    .effective_deadline
                    .map_or(TimingPolicy::Untimed, |_| TimingPolicy::PerQuestion {
                        seconds: 1,
                        grace_seconds: timing.effective_grace_seconds,
                    })
            });
    let verdict = timer_verdict(&TimerEvaluation {
        policy: effective_policy,
        timer: submitted.timer,
        evaluated_at: submitted_at,
        pause_extension_millis: 0,
    })
    .map_err(|error| StoreError::InvalidRecord(error.to_string()))?;
    if verdict == TimerVerdict::TimedOut {
        return Err(StoreError::TimedOut);
    }
    require_course_records_accessible(state, tenant, assignment.course_id)?;
    let previous = state
        .summaries
        .get(&(tenant, enrollment.id))
        .cloned()
        .ok_or(StoreError::NotFound)?;
    let mut next = project_summary(
        &previous,
        domain::scoring::RunTransition::QuestionAttemptRecorded { at: submitted_at },
        grade_policy(&assignment),
    )?;
    let run_items = state
        .run_items
        .get(&(tenant, run.id))
        .cloned()
        .ok_or_else(|| StoreError::Unavailable("run has no immutable items".to_string()))?;
    let attempts = state
        .attempts
        .values()
        .filter(|attempt| attempt.tenant == tenant && attempt.run == run.id)
        .map(|attempt| {
            if attempt.id == submitted.id {
                submitted.clone()
            } else {
                projected_attempt(state, tenant, attempt)
            }
        })
        .collect::<Vec<_>>();
    let questions = current_run_questions(&assignment, &run_items, &attempts, &submitted)?;
    let results = questions
        .iter()
        .map(|question| question.map(|question| question.result))
        .collect::<Vec<_>>();
    let submitted_item = run_items
        .iter()
        .find(|item| item.issued_position == submitted.assignment_position)
        .ok_or_else(|| {
            StoreError::Unavailable("submitted attempt has no immutable run item".to_string())
        })?;
    let submitted_assignment_item = submitted_item.assignment_item;
    let (earned_points, possible_points) = crate::current_attempt_points(
        &assignment,
        submitted_assignment_item,
        submitted.status,
        command.result,
    )?;
    let (scoring_generation, _) = state
        .assignment_scoring
        .get(&(tenant, assignment.id))
        .copied()
        .ok_or(StoreError::NotFound)?;
    let mut statistics_contributions = None;
    if let Some(score) = completed_run_score(&questions, assignment.policies.completion)? {
        next = project_summary(
            &next,
            domain::scoring::RunTransition::Completed {
                score,
                at: submitted_at,
            },
            grade_policy(&assignment),
        )?;
        run.completed_at = Some(submitted_at);
        run.score = Some(score);
        project_enrollment_completion(
            &mut enrollment,
            &previous,
            grade_policy(&assignment),
            run.id,
            score,
            submitted_at,
        );
        if run.mode == RunMode::Assigned && previous.completed_run_count == 0 {
            statistics_contributions = Some(derive_statistics_contributions(
                &run_items, &results, &attempts,
            )?);
        }
    }
    if let Some(contributions) = &statistics_contributions {
        stage_statistics_contributions(
            state,
            tenant,
            enrollment.id,
            run.id,
            submitted.id,
            contributions,
        )?;
    }
    let record = SubmissionRecord {
        attempt: submitted,
        run: run.clone(),
        summary: next.clone(),
        feedback,
    };
    state.submissions.insert(
        (tenant, command.attempt),
        StoredSubmission {
            key: command.idempotency_key,
            response: command.response,
            record: record.clone(),
        },
    );
    state.attempt_scores.insert(
        (tenant, command.attempt),
        MemoryAttemptScore {
            assignment: assignment.id,
            assignment_item: submitted_assignment_item,
            generation: scoring_generation,
            earned_points,
            possible_points,
        },
    );
    state.runs.insert((tenant, run.id), run);
    state
        .enrollments
        .insert((tenant, enrollment.id), enrollment);
    state.summaries.insert((tenant, next.enrollment), next);
    complete_memory_attempt_timing_job(state, tenant, command.attempt);
    Ok(record)
}

/// Stages all first-completed-run contributions before mutating visible
/// submission state. One rejected aggregate leaves the whole MemoryStore
/// transition unchanged.
fn stage_statistics_contributions(
    state: &mut State,
    tenant: TenantId,
    enrollment: EnrollmentId,
    first_completed_run: RunId,
    trigger_attempt: QuestionAttemptId,
    contributions: &[StatisticsContribution],
) -> Result<(), StoreError> {
    let mut aggregate_updates = BTreeMap::new();
    let mut receipt_updates = BTreeMap::new();
    for contribution in contributions {
        let receipt_key = (
            tenant,
            enrollment,
            contribution.reference.problem,
            contribution.reference.version,
        );
        if let Some(receipt) = state.question_statistics_receipts.get(&receipt_key) {
            if receipt.first_completed_run == first_completed_run
                && receipt.attempt == trigger_attempt
                && receipt.checksum == contribution.checksum
            {
                continue;
            }
            return Err(StoreError::Conflict);
        }
        if receipt_updates.contains_key(&receipt_key) {
            return Err(StoreError::Conflict);
        }
        let aggregate_key = (
            contribution.reference.problem,
            contribution.reference.version,
        );
        let aggregate = aggregate_updates.entry(aggregate_key).or_insert_with(|| {
            state
                .question_statistics
                .get(&aggregate_key)
                .cloned()
                .unwrap_or_default()
        });
        aggregate
            .record(contribution.observation)
            .map_err(|error| StoreError::InvalidRecord(error.to_string()))?;
        receipt_updates.insert(
            receipt_key,
            StatisticsContributionReceipt {
                first_completed_run,
                attempt: trigger_attempt,
                #[cfg(test)]
                observation: contribution.observation,
                checksum: contribution.checksum,
            },
        );
    }
    state.question_statistics.extend(aggregate_updates);
    state.question_statistics_receipts.extend(receipt_updates);
    Ok(())
}

fn validate_assignment_position(
    run_items: &[AssignmentRunItem],
    command: &IssueQuestionAttemptCommand,
) -> Result<(), StoreError> {
    let expected = run_items
        .iter()
        .find(|item| item.issued_position == command.assignment_position)
        .ok_or_else(|| {
            StoreError::InvalidRecord("question position is outside the assignment".to_string())
        })?;
    if expected.reference.problem != command.problem
        || expected.reference.version != command.question_version
    {
        return Err(StoreError::InvalidRecord(
            "question identity does not match its assignment position".to_string(),
        ));
    }
    Ok(())
}

fn validate_memory_assignment_references(
    state: &State,
    context: TenantContext,
    assignment: &AssignmentRecord,
) -> Result<(), StoreError> {
    if !state
        .courses
        .contains_key(&(assignment.tenant, assignment.course_id))
    {
        return Err(StoreError::InvalidRecord(
            "assignment references a missing course".to_string(),
        ));
    }
    for reference in assignment.references() {
        let assignable = state
            .published
            .get(&(reference.problem, reference.version))
            .is_some_and(|record| {
                record.lifecycle.is_assignable()
                    && catalog_record_visible(state, context.tenant_id(), record)
            });
        if !assignable {
            return Err(StoreError::InvalidRecord(format!(
                "assignment references a missing, hidden, or inactive published version {}/{}",
                reference.problem, reference.version
            )));
        }
    }
    Ok(())
}

fn validate_memory_assignment_content_lock(
    state: &State,
    previous: &AssignmentRecord,
    replacement: &AssignmentRecord,
) -> Result<(), StoreError> {
    let has_run = state.runs.values().any(|run| {
        state
            .enrollments
            .get(&(run.tenant, run.enrollment))
            .is_some_and(|enrollment| {
                enrollment.tenant == previous.tenant && enrollment.assignment == previous.id
            })
    });
    if !has_run {
        return Ok(());
    }
    let retirement_blocked = previous.items.iter().any(|item| {
        item.delivery_state == question_model::AssignmentDeliveryState::Active
            && replacement.items.iter().any(|candidate| {
                candidate.id == item.id
                    && candidate.delivery_state == question_model::AssignmentDeliveryState::Retired
            })
            && memory_item_has_active_attempt(state, previous, item.id)
    }) || previous.selection_groups.iter().any(|group| {
        group.candidates.iter().any(|candidate| {
            candidate.delivery_state == question_model::AssignmentDeliveryState::Active
                && replacement
                    .selection_groups
                    .iter()
                    .any(|replacement_group| {
                        replacement_group.candidates.iter().any(|replacement| {
                            replacement.id == candidate.id
                                && replacement.delivery_state
                                    == question_model::AssignmentDeliveryState::Retired
                        })
                    })
                && memory_item_has_active_attempt(state, previous, candidate.id)
        })
    });
    if retirement_blocked {
        return Err(StoreError::Conflict);
    }
    let previous_items = previous
        .items
        .iter()
        .map(|item| (item.id, item.reference))
        .collect::<BTreeMap<_, _>>();
    let replacement_items = replacement
        .items
        .iter()
        .map(|item| (item.id, item.reference))
        .collect::<BTreeMap<_, _>>();
    let previous_groups = previous
        .selection_groups
        .iter()
        .map(|group| {
            (
                group.id,
                group
                    .candidates
                    .iter()
                    .map(|candidate| (candidate.id, candidate.reference))
                    .collect::<BTreeMap<_, _>>(),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let replacement_groups = replacement
        .selection_groups
        .iter()
        .map(|group| {
            (
                group.id,
                group
                    .candidates
                    .iter()
                    .map(|candidate| (candidate.id, candidate.reference))
                    .collect::<BTreeMap<_, _>>(),
            )
        })
        .collect::<BTreeMap<_, _>>();
    if previous_items != replacement_items || previous_groups != replacement_groups {
        return Err(StoreError::Conflict);
    }
    Ok(())
}

fn memory_item_has_active_attempt(
    state: &State,
    assignment: &AssignmentRecord,
    assignment_item: question_model::AssignmentItemId,
) -> bool {
    state.attempts.values().any(|base| {
        if base.tenant != assignment.tenant
            || projected_attempt(state, assignment.tenant, base).status != AttemptStatus::InProgress
        {
            return false;
        }
        let Some(run) = state.runs.get(&(assignment.tenant, base.run)) else {
            return false;
        };
        let belongs_to_assignment = state
            .enrollments
            .get(&(assignment.tenant, run.enrollment))
            .is_some_and(|enrollment| enrollment.assignment == assignment.id);
        belongs_to_assignment
            && state
                .run_items
                .get(&(assignment.tenant, run.id))
                .and_then(|items| {
                    items
                        .iter()
                        .find(|item| item.issued_position == base.assignment_position)
                })
                .is_some_and(|item| item.assignment_item == assignment_item)
    })
}

fn memory_assignment_has_results(state: &State, assignment: &AssignmentRecord) -> bool {
    state.submissions.values().any(|submission| {
        let attempt = &submission.record.attempt;
        attempt.result.is_some()
            && state
                .runs
                .get(&(attempt.tenant, attempt.run))
                .and_then(|run| state.enrollments.get(&(run.tenant, run.enrollment)))
                .is_some_and(|enrollment| enrollment.assignment == assignment.id)
    })
}

fn resolved_memory_attempt_timing(
    policy: AssignmentTimingPolicy,
    run: &AssignmentRun,
    authored_deadline: Option<ActivityTimestamp>,
    authored_grace_seconds: u32,
) -> Result<(Option<ActivityTimestamp>, u32, Option<ActivityTimestamp>), StoreError> {
    let mut resolved = authored_deadline.map(|deadline| (deadline, authored_grace_seconds));
    let mut consider = |deadline: ActivityTimestamp, grace_seconds: u32| {
        if resolved.is_none_or(|current| (deadline, grace_seconds) < current) {
            resolved = Some((deadline, grace_seconds));
        }
    };
    if let Some(seconds) = policy.time_limit_seconds {
        consider(
            add_seconds(run.started_at, seconds, "assignment time limit")?,
            0,
        );
    }
    if policy.late_submission == question_model::LateSubmissionPolicy::Reject
        && let Some(due_at) = policy.due_at
    {
        consider(due_at, 0);
    }
    if let Some(closes_at) = policy.closes_at {
        consider(closes_at, 0);
    }
    let auto_submit_at = resolved
        .map(|(deadline, grace_seconds)| {
            add_seconds(deadline, grace_seconds, "attempt auto-submit deadline")
        })
        .transpose()?;
    Ok((
        resolved.map(|(deadline, _)| deadline),
        resolved.map_or(0, |(_, grace_seconds)| grace_seconds),
        auto_submit_at,
    ))
}

fn memory_resolved_assignment_policy(
    state: &State,
    tenant: TenantId,
    assignment: AssignmentId,
    enrollment: &AssignmentEnrollment,
    base_override: Option<AssignmentTimingPolicy>,
) -> Result<crate::ResolvedAssignmentTimingPolicy, StoreError> {
    let base = base_override
        .or_else(|| state.assignment_timing.get(&(tenant, assignment)).copied())
        .ok_or_else(|| {
            StoreError::Unavailable(
                "assignment timing policy is missing from memory state".to_string(),
            )
        })?;
    let applicable = state
        .assignment_policy_exceptions
        .iter()
        .filter_map(|((record_tenant, record_assignment, target), exception)| {
            if *record_tenant != tenant || *record_assignment != assignment {
                return None;
            }
            let applies = match target {
                AssignmentPolicyExceptionTarget::Student(student) => *student == enrollment.student,
                AssignmentPolicyExceptionTarget::CourseGroup(group) => state
                    .course_groups
                    .get(&(tenant, *group))
                    .is_some_and(|record| record.members.contains(&enrollment.user)),
            };
            applies.then_some(exception.clone())
        })
        .collect::<Vec<_>>();
    resolve_assignment_policy(base, &applicable)
}

fn apply_memory_assignment_timing_update(
    state: &mut State,
    tenant: TenantId,
    assignment: AssignmentId,
    base_override: Option<AssignmentTimingPolicy>,
) -> Result<(), StoreError> {
    #[derive(Debug)]
    enum JobChange {
        Insert(JobId, StoredJob),
        Reschedule(JobId, JobPayload, ActivityTimestamp),
        Complete(JobId),
        None,
    }
    struct Pending {
        attempt: QuestionAttemptId,
        timing: MemoryAttemptTiming,
        resolution: crate::ResolvedAssignmentTimingPolicy,
        current: Option<QuestionAttempt>,
        job_change: JobChange,
    }

    let now = state.authoritative_time;
    let existing = state
        .attempt_timing
        .iter()
        .filter(|((record_tenant, _), timing)| {
            *record_tenant == tenant && timing.assignment == assignment
        })
        .map(|((_, attempt), timing)| (*attempt, *timing))
        .collect::<Vec<_>>();
    let mut reserved_jobs = BTreeSet::new();
    let mut pending = Vec::with_capacity(existing.len());
    for (attempt_id, previous_timing) in existing {
        let base = state
            .attempts
            .get(&(tenant, attempt_id))
            .ok_or(StoreError::NotFound)?;
        if projected_attempt(state, tenant, base).status != AttemptStatus::InProgress {
            continue;
        }
        let run = state
            .runs
            .get(&(tenant, base.run))
            .ok_or(StoreError::NotFound)?;
        let enrollment = state
            .enrollments
            .get(&(tenant, run.enrollment))
            .ok_or(StoreError::NotFound)?;
        let resolution = memory_resolved_assignment_policy(
            state,
            tenant,
            assignment,
            enrollment,
            base_override,
        )?;
        let (effective_deadline, grace_seconds, auto_submit_at) = resolved_memory_attempt_timing(
            resolution.policy,
            run,
            previous_timing.authored_deadline,
            previous_timing.authored_grace_seconds,
        )?;
        let generation = previous_timing
            .generation
            .checked_add(1)
            .ok_or(StoreError::Conflict)?;
        let payload = JobPayload::AutoSubmitAttempt {
            attempt: attempt_id,
            timing_generation: generation,
        };
        let immediate = auto_submit_at.is_some_and(|deadline| deadline <= now);
        let mut current = None;
        let mut job = previous_timing.job;
        let existing_job = job.and_then(|id| state.jobs.get(&id).map(|stored| (id, stored.state)));
        let job_change = if immediate {
            let mut projected = projected_attempt(state, tenant, base);
            projected.status = AttemptStatus::AutoSubmitted;
            projected.timer.deadline = effective_deadline;
            projected.timer.submitted_at = Some(now);
            current = Some(projected);
            let change = match existing_job {
                Some((id, JobState::Ready | JobState::Leased)) => JobChange::Complete(id),
                _ => JobChange::None,
            };
            job = None;
            change
        } else if let Some(available_at) = auto_submit_at {
            match existing_job {
                Some((id, JobState::Ready)) => {
                    JobChange::Reschedule(id, payload.clone(), available_at)
                }
                Some((_id, JobState::Leased)) => JobChange::None,
                Some((_, JobState::Completed | JobState::Dead)) | None => {
                    let id = loop {
                        let candidate = JobId::generate()?;
                        if !state.jobs.contains_key(&candidate) && reserved_jobs.insert(candidate) {
                            break candidate;
                        }
                    };
                    job = Some(id);
                    JobChange::Insert(
                        id,
                        StoredJob {
                            tenant,
                            payload: payload.clone(),
                            state: JobState::Ready,
                            available_at,
                            lease_token: None,
                            lease_expires_at: None,
                            attempt_count: 0,
                            max_attempts: 10,
                            failure: None,
                        },
                    )
                }
            }
        } else {
            let change = match existing_job {
                Some((id, JobState::Ready | JobState::Leased)) => JobChange::Complete(id),
                _ => JobChange::None,
            };
            job = None;
            change
        };
        pending.push(Pending {
            attempt: attempt_id,
            timing: MemoryAttemptTiming {
                assignment,
                authored_deadline: previous_timing.authored_deadline,
                authored_grace_seconds: previous_timing.authored_grace_seconds,
                effective_deadline,
                effective_grace_seconds: grace_seconds,
                auto_submit_at,
                generation,
                job,
            },
            resolution,
            current,
            job_change,
        });
    }
    for update in pending {
        match update.job_change {
            JobChange::Insert(id, job) => {
                state.jobs.insert(id, job);
            }
            JobChange::Reschedule(id, payload, available_at) => {
                let job = state.jobs.get_mut(&id).ok_or(StoreError::NotFound)?;
                job.payload = payload;
                job.available_at = available_at;
                job.failure = None;
            }
            JobChange::Complete(id) => {
                if let Some(job) = state.jobs.get_mut(&id) {
                    job.state = JobState::Completed;
                    job.lease_token = None;
                    job.lease_expires_at = None;
                }
            }
            JobChange::None => {}
        }
        if let Some(current) = update.current {
            state
                .attempt_current
                .insert((tenant, update.attempt), current);
        }
        state
            .attempt_timing
            .insert((tenant, update.attempt), update.timing);
        state
            .attempt_timing_resolution
            .insert((tenant, update.attempt), update.resolution);
    }
    Ok(())
}

fn issued_timer(
    issued_at: ActivityTimestamp,
    run: &AssignmentRun,
    policy: TimingPolicy,
) -> Result<AttemptTimerRecord, StoreError> {
    let deadline = match policy {
        TimingPolicy::Untimed => None,
        TimingPolicy::PerQuestion { seconds, .. } => {
            Some(add_seconds(issued_at, seconds, "question deadline")?)
        }
        TimingPolicy::PerAttempt { seconds, .. } => {
            let deadline = add_seconds(run.started_at, seconds, "run deadline")?;
            if deadline < issued_at {
                return Err(StoreError::TimedOut);
            }
            Some(deadline)
        }
    };
    Ok(AttemptTimerRecord {
        issued_at,
        deadline,
        submitted_at: None,
    })
}

fn timing_policy_grace_seconds(policy: TimingPolicy) -> u32 {
    match policy {
        TimingPolicy::Untimed => 0,
        TimingPolicy::PerQuestion { grace_seconds, .. }
        | TimingPolicy::PerAttempt { grace_seconds, .. } => grace_seconds,
    }
}

fn add_seconds(
    timestamp: ActivityTimestamp,
    seconds: u32,
    description: &str,
) -> Result<ActivityTimestamp, StoreError> {
    timestamp
        .as_unix_millis()
        .checked_add(i64::from(seconds) * 1_000)
        .map(ActivityTimestamp::from_unix_millis)
        .ok_or_else(|| StoreError::InvalidRecord(format!("{description} overflow")))
}

fn require_attempt_owner(
    state: &State,
    tenant: TenantId,
    attempt: &QuestionAttempt,
    actor: UserId,
) -> Result<(), StoreError> {
    let run = state
        .runs
        .get(&(tenant, attempt.run))
        .ok_or(StoreError::NotFound)?;
    let enrollment = enrollment_record(state, tenant, run.enrollment)?;
    if enrollment.user == actor {
        Ok(())
    } else {
        Err(StoreError::NotFound)
    }
}

fn require_attempt_course_records_accessible(
    state: &State,
    tenant: TenantId,
    attempt: &QuestionAttempt,
) -> Result<(), StoreError> {
    let run = state
        .runs
        .get(&(tenant, attempt.run))
        .ok_or(StoreError::NotFound)?;
    let enrollment = enrollment_record(state, tenant, run.enrollment)?;
    let assignment = assignment_record(state, tenant, enrollment.assignment)?;
    require_course_records_accessible(state, tenant, assignment.course_id)
}

fn apply_memory_attempt_support(
    state: &mut State,
    context: TenantContext,
    action_id: AttemptSupportActionId,
    actor: UserId,
    attempt_id: QuestionAttemptId,
    action: AttemptSupportAction,
) -> Result<AttemptSupportRecord, StoreError> {
    let tenant = context.tenant_id();
    let base = state
        .attempts
        .get(&(tenant, attempt_id))
        .cloned()
        .ok_or(StoreError::NotFound)?;
    let run = state
        .runs
        .get(&(tenant, base.run))
        .ok_or(StoreError::NotFound)?;
    let enrollment = enrollment_record(state, tenant, run.enrollment)?;
    let assignment = assignment_record(state, tenant, enrollment.assignment)?;
    require_course_records_accessible(state, tenant, assignment.course_id)?;
    let course = state
        .courses
        .get(&(tenant, assignment.course_id))
        .ok_or(StoreError::NotFound)?;
    if course.role_for(actor) != Some(CourseRole::Instructor) {
        return Err(StoreError::NotFound);
    }
    if let Some(existing) = state.attempt_support_actions.get(&(tenant, action_id)) {
        return if existing.actor == actor
            && existing.attempt == attempt_id
            && existing.kind == action
        {
            Ok(*existing)
        } else {
            Err(StoreError::Conflict)
        };
    }

    let previous = projected_attempt(state, tenant, &base);
    let resulting_status = match action {
        AttemptSupportAction::ForceSubmit if previous.status == AttemptStatus::InProgress => {
            AttemptStatus::NeedsManualGrading
        }
        AttemptSupportAction::Clear
            if matches!(
                previous.status,
                AttemptStatus::InProgress
                    | AttemptStatus::Submitted
                    | AttemptStatus::AutoSubmitted
                    | AttemptStatus::NeedsManualGrading
            ) =>
        {
            AttemptStatus::Cleared
        }
        _ => return Err(StoreError::Conflict),
    };
    let now = state.authoritative_time;
    let mut current = previous.clone();
    current.status = resulting_status;
    if action == AttemptSupportAction::ForceSubmit {
        current.timer.submitted_at = Some(now);
    }

    let scoring_update = if action == AttemptSupportAction::Clear && previous.result.is_some() {
        let key = (tenant, assignment.id);
        let (generation, _) = state
            .assignment_scoring
            .get(&key)
            .copied()
            .ok_or(StoreError::NotFound)?;
        let generation = generation.next().ok_or(StoreError::Conflict)?;
        let job = loop {
            let candidate = crate::JobId::generate()?;
            if !state.jobs.contains_key(&candidate) {
                break candidate;
            }
        };
        Some((key, generation, job))
    } else {
        None
    };
    let record = AttemptSupportRecord {
        tenant,
        action: action_id,
        actor,
        attempt: attempt_id,
        kind: action,
        previous_status: previous.status,
        resulting_status,
        occurred_at: now,
    };

    if let Some((key, generation, job)) = scoring_update {
        let queued = StoredJob {
            tenant,
            payload: crate::JobPayload::RecalculateAssignment {
                assignment: assignment.id,
                generation,
            },
            state: JobState::Ready,
            available_at: now,
            lease_token: None,
            lease_expires_at: None,
            attempt_count: 0,
            max_attempts: 10,
            failure: None,
        };
        state.jobs.insert(job, queued);
        state
            .assignment_scoring
            .insert(key, (generation, ScoringStatus::Recalculating));
    }
    state.attempt_current.insert((tenant, attempt_id), current);
    complete_memory_attempt_timing_job(state, tenant, attempt_id);
    state
        .attempt_support_actions
        .insert((tenant, action_id), record);
    Ok(record)
}

fn complete_memory_attempt_timing_job(
    state: &mut State,
    tenant: TenantId,
    attempt: QuestionAttemptId,
) {
    let job = state
        .attempt_timing
        .get_mut(&(tenant, attempt))
        .and_then(|timing| timing.job.take());
    let Some(job) = job else {
        return;
    };
    if let Some(stored) = state.jobs.get_mut(&job)
        && matches!(stored.state, JobState::Ready | JobState::Leased)
    {
        stored.state = JobState::Completed;
        stored.lease_token = None;
        stored.lease_expires_at = None;
    }
}

fn projected_attempt(
    state: &State,
    tenant: TenantId,
    attempt: &QuestionAttempt,
) -> QuestionAttempt {
    let mut projected = state
        .attempt_current
        .get(&(tenant, attempt.id))
        .cloned()
        .or_else(|| {
            state
                .submissions
                .get(&(tenant, attempt.id))
                .map(|stored| stored.record.attempt.clone())
        })
        .unwrap_or_else(|| attempt.clone());
    if let Some(timing) = state.attempt_timing.get(&(tenant, attempt.id)) {
        projected.timer.deadline = timing.effective_deadline;
    }
    projected
}

impl MemoryStore {
    /// Inserts a pre-validation legacy draft for route-boundary tests only.
    ///
    /// This exists to prove current HTTP handlers fail safely when a database
    /// contains historical corrupt data. It is compiled only with the
    /// `test-support` feature and must not be enabled by production code.
    #[cfg(feature = "test-support")]
    pub fn insert_legacy_draft_for_test(&self, draft: DraftRecord) -> Result<(), StoreError> {
        let mut state = self.write_state()?;
        state
            .drafts
            .insert((draft.tenant, draft.question.workspace), draft);
        Ok(())
    }

    /// Seeds a closed, due archive-cleanup job for server worker tests only.
    #[cfg(feature = "test-support")]
    pub fn seed_retention_cleanup_for_test(
        &self,
        tenant: TenantId,
        course: CourseId,
        objects: Vec<question_model::ObjectId>,
    ) -> Result<Vec<objects::ObjectKey>, StoreError> {
        self.seed_retention_cleanup_stage_for_test(
            tenant,
            course,
            objects,
            crate::RetentionStage::ArchiveStudentRecords,
            AssignmentDefinitionDisposition::Retain,
        )
    }

    #[cfg(feature = "test-support")]
    fn seed_retention_cleanup_stage_for_test(
        &self,
        tenant: TenantId,
        course: CourseId,
        objects: Vec<question_model::ObjectId>,
        stage: crate::RetentionStage,
        disposition: AssignmentDefinitionDisposition,
    ) -> Result<Vec<objects::ObjectKey>, StoreError> {
        let mut state = self.write_state()?;
        let now = state.authoritative_time;
        let job = crate::JobId::generate()?;
        let export_job = crate::JobId::generate()?;
        let export = crate::ExportId::generate()?;
        let snapshot = CourseRetentionSnapshot::new(
            now,
            InstitutionRetentionPolicy::default(),
            disposition,
            1,
        )
        .map_err(|error| StoreError::InvalidRecord(error.to_string()))?;
        state
            .courses
            .entry((tenant, course))
            .or_insert_with(|| CourseRecord {
                id: course,
                tenant,
                title: "Retention test course".to_string(),
                members: Vec::new(),
            });
        state.course_retention.insert(
            (tenant, course),
            CourseRetentionRecord {
                snapshot,
                status: crate::CourseRetentionStatus::from_persisted(
                    CourseRetentionState::Active,
                    disposition,
                ),
            },
        );
        state.retention_stages.insert(
            (tenant, course, stage, 1),
            StoredRetentionStage {
                due_at: now,
                state: RetentionStageWorkState::Scheduled,
                job: None,
                lease: None,
            },
        );
        state
            .retention_dispatches
            .insert((tenant, course, stage, 1), job);
        state.jobs.insert(
            job,
            StoredJob {
                tenant,
                payload: crate::JobPayload::Retention {
                    course,
                    stage,
                    generation: 1,
                },
                state: crate::JobState::Ready,
                available_at: now,
                lease_token: None,
                lease_expires_at: None,
                attempt_count: 0,
                max_attempts: 2,
                failure: None,
            },
        );
        let expected = objects
            .iter()
            .copied()
            .enumerate()
            .map(|(index, object)| (crate::ExportArtifactKind::ALL[index], object))
            .collect();
        let fixture_uuid =
            |suffix| Uuid::from_u128(course.as_uuid().as_u128().wrapping_add(suffix));
        let manifest = question_model::ObjectId::from_uuid(fixture_uuid(1));
        state.jobs.insert(
            export_job,
            StoredJob {
                tenant,
                payload: crate::JobPayload::Export {
                    delivery_object: manifest,
                },
                state: crate::JobState::Ready,
                available_at: ActivityTimestamp::from_unix_millis(now.as_unix_millis() + 1),
                lease_token: None,
                lease_expires_at: None,
                attempt_count: 0,
                max_attempts: 2,
                failure: None,
            },
        );
        state.exports.insert(
            (tenant, export),
            StoredExport {
                course,
                assignment: AssignmentId::from_uuid(fixture_uuid(2)),
                title: "retention test".to_string(),
                requested_by: UserId::from_uuid(fixture_uuid(3)),
                manifest,
                problems: Vec::new(),
                job: export_job,
                state: crate::StudentExportState::Queued,
                expected,
                artifacts: None,
            },
        );
        Ok(objects
            .into_iter()
            .map(|object| objects::ObjectKey::StudentRecord { tenant, object })
            .collect())
    }

    /// Sets the stub backend clock used by session tests and local development.
    pub fn set_authoritative_time(&self, now: ActivityTimestamp) -> Result<(), StoreError> {
        self.write_state()?.authoritative_time = now;
        Ok(())
    }

    /// Returns protected asset access events for conformance assertions.
    pub fn asset_access_events(&self) -> Result<Vec<AssetAccessEvent>, StoreError> {
        Ok(self.read_state()?.asset_access_events.clone())
    }

    /// Test-only equivalent of the later submission-completion capability.
    ///
    /// No public Store trait method accepts a collapsed observation.  Keeping
    /// this seam inside the backend proves receipt idempotency and aggregate
    /// atomicity without creating a route-callable statistics writer.
    #[cfg(test)]
    fn record_question_statistics_contribution(
        &self,
        tenant: TenantId,
        enrollment: EnrollmentId,
        first_completed_run: RunId,
        attempt: QuestionAttemptId,
        reference: ProblemVersionRef,
        observation: CollapsedQuestionObservation,
    ) -> Result<bool, StoreError> {
        let mut state = self.write_state()?;
        let receipt_key = (tenant, enrollment, reference.problem, reference.version);
        if let Some(receipt) = state.question_statistics_receipts.get(&receipt_key) {
            return if receipt.first_completed_run == first_completed_run
                && receipt.attempt == attempt
                && receipt.observation == observation
            {
                Ok(false)
            } else {
                Err(StoreError::Conflict)
            };
        }
        let aggregate_key = (reference.problem, reference.version);
        let mut aggregate = state
            .question_statistics
            .get(&aggregate_key)
            .cloned()
            .unwrap_or_default();
        aggregate
            .record(observation)
            .map_err(|error| StoreError::InvalidRecord(error.to_string()))?;
        state.question_statistics.insert(aggregate_key, aggregate);
        state.question_statistics_receipts.insert(
            receipt_key,
            StatisticsContributionReceipt {
                first_completed_run,
                attempt,
                #[cfg(test)]
                observation,
                checksum: objects::Sha256Digest::compute(b"statistics test contribution"),
            },
        );
        Ok(true)
    }

    /// Acquires immutable backend state.
    fn read_state(&self) -> Result<std::sync::RwLockReadGuard<'_, State>, StoreError> {
        self.state
            .read()
            .map_err(|error| StoreError::Unavailable(error.to_string()))
    }

    /// Acquires mutable backend state for one atomic operation.
    fn write_state(&self) -> Result<std::sync::RwLockWriteGuard<'_, State>, StoreError> {
        self.state
            .write()
            .map_err(|error| StoreError::Unavailable(error.to_string()))
    }
}

/// Loads one enrollment inside an already tenant-scoped state operation.
fn enrollment_record(
    state: &State,
    tenant: TenantId,
    enrollment: EnrollmentId,
) -> Result<AssignmentEnrollment, StoreError> {
    state
        .enrollments
        .get(&(tenant, enrollment))
        .cloned()
        .ok_or(StoreError::NotFound)
}

/// Loads the assignment whose grade policy drives summary projection.
fn assignment_record(
    state: &State,
    tenant: TenantId,
    assignment: AssignmentId,
) -> Result<AssignmentRecord, StoreError> {
    state
        .assignments
        .get(&(tenant, assignment))
        .cloned()
        .ok_or(StoreError::NotFound)
}

fn catalog_record_visible(
    state: &State,
    tenant: TenantId,
    record: &PublishedProblemRecord,
) -> bool {
    // Exact-version reads intentionally retain deprecated and archived content
    // for historical assignments. This is the same scope/grant predicate used
    // by PostgreSQL RLS and the published-QTI grader capability.
    record.scope == PublicationScope::Public
        || state
            .catalog_grants
            .contains(&(tenant, record.problem, record.version))
}

/// Converts the wire-owned bounded search request into the shared pagination
/// primitive.  The opaque token is checked for query binding below rather than
/// treated as an untrusted stable key.
fn search_page_request(query: &CatalogSearchQuery) -> Result<PageRequest, StoreError> {
    let size = PageSize::new(query.page_size.unwrap_or(50))
        .map_err(|error| StoreError::InvalidRecord(error.to_string()))?;
    match query.cursor.clone() {
        Some(cursor) => Cursor::parse(cursor)
            .map(|cursor| PageRequest::after(cursor, size))
            .map_err(|error| StoreError::InvalidRecord(error.to_string())),
        None => Ok(PageRequest::first(size)),
    }
}

/// Stable digest of filters only. The digest avoids exposing title/taxonomy
/// contents through a cursor and makes a cursor from a different filter set a
/// deterministic client error rather than a subtly stale page.
fn catalog_search_fingerprint(query: &CatalogSearchQuery) -> String {
    let mut canonical = String::new();
    canonical.push_str(query.text.as_deref().unwrap_or(""));
    canonical.push('\u{1f}');
    for term in &query.taxonomy {
        canonical.push_str(&term.scheme);
        canonical.push('\u{1e}');
        canonical.push_str(&term.code);
        canonical.push('\u{1f}');
    }
    canonical.push('|');
    for capability in &query.capabilities {
        canonical.push_str(capability.as_str());
        canonical.push('\u{1f}');
    }
    canonical.push('|');
    for license in &query.licenses {
        canonical.push_str(&format!("{license:?}"));
        canonical.push('\u{1f}');
    }
    canonical.push('|');
    canonical.push_str(&format!("{:?}", query.statistics));
    Sha256Digest::compute(canonical.as_bytes()).to_string()
}

fn catalog_search_matches(
    record: &PublishedProblemRecord,
    query: &CatalogSearchQuery,
    statistics_available: bool,
) -> bool {
    if matches!(query.statistics, CatalogStatisticsAvailability::Available) && !statistics_available
    {
        return false;
    }
    if matches!(query.statistics, CatalogStatisticsAvailability::Unavailable)
        && statistics_available
    {
        return false;
    }
    if let Some(text) = &query.text {
        let searchable = std::iter::once(record.question.metadata.title.as_str())
            .chain(record.question.metadata.language.split_whitespace())
            .chain(record.question.metadata.tags.iter().map(|tag| tag.as_str()))
            .chain(record.question.metadata.taxonomy.iter().flat_map(|term| {
                [
                    term.scheme.as_str(),
                    term.code.as_str(),
                    term.label.as_str(),
                ]
            }))
            .any(|value| value.to_lowercase().contains(text));
        if !searchable {
            return false;
        }
    }
    if !query.taxonomy.iter().all(|wanted| {
        record
            .question
            .metadata
            .taxonomy
            .iter()
            .any(|term| term.scheme == wanted.scheme && term.code == wanted.code)
    }) {
        return false;
    }
    if !query
        .capabilities
        .iter()
        .all(|capability| record.capabilities.supports(*capability))
    {
        return false;
    }
    query.licenses.is_empty()
        || query
            .licenses
            .iter()
            .any(|license| license.matches(&record.question.metadata.license))
}

fn catalog_search_facets<'a>(
    records: impl Iterator<Item = (&'a PublishedProblemRecord, bool)>,
) -> CatalogSearchFacets {
    let mut taxonomy = BTreeMap::<String, (TaxonomyTerm, u64)>::new();
    let mut capabilities = BTreeMap::new();
    let mut licenses = BTreeMap::new();
    let mut unavailable = 0_u64;
    let mut available = 0_u64;
    for (record, statistics_available) in records {
        if statistics_available {
            available += 1;
        } else {
            unavailable += 1;
        }
        for term in &record.question.metadata.taxonomy {
            let entry = taxonomy
                .entry(taxonomy_cursor_key(term))
                .or_insert_with(|| (term.clone(), 0));
            entry.1 += 1;
            // A controlled identity is `(scheme, code)`. Legacy imports may
            // disagree on display text; choose the lexicographically smallest
            // label so Memory and PostgreSQL remain deterministic.
            if term.label < entry.0.label {
                entry.0.label = term.label.clone();
            }
        }
        for capability in record.capabilities.declared() {
            *capabilities.entry(capability).or_insert(0_u64) += 1;
        }
        *licenses
            .entry(CatalogLicenseValue::from_license(
                &record.question.metadata.license,
            ))
            .or_insert(0_u64) += 1;
    }
    let mut taxonomy = taxonomy
        .into_values()
        .map(|(term, count)| CatalogTaxonomyFacet { term, count })
        .collect::<Vec<_>>();
    taxonomy.sort_by(|left, right| {
        right
            .count
            .cmp(&left.count)
            .then_with(|| left.term.scheme.cmp(&right.term.scheme))
            .then_with(|| left.term.code.cmp(&right.term.code))
    });
    taxonomy.truncate(MAX_CATALOG_TAXONOMY_FACETS);
    CatalogSearchFacets {
        taxonomy,
        capabilities: capabilities
            .into_iter()
            .map(|(capability, count)| CatalogCapabilityFacet { capability, count })
            .collect(),
        licenses: licenses
            .into_iter()
            .map(|(license, count)| CatalogLicenseFacet { license, count })
            .collect(),
        statistics: CatalogStatisticsFacet {
            available,
            unavailable,
        },
    }
}

fn validated_deprecation_reason(reason: String) -> Result<String, StoreError> {
    const MAX_REASON_CHARS: usize = 1_000;
    let reason = reason.trim();
    if reason.is_empty() {
        return Err(StoreError::InvalidRecord(
            "deprecation requires a nonempty reason".to_string(),
        ));
    }
    if reason.chars().count() > MAX_REASON_CHARS {
        return Err(StoreError::InvalidRecord(format!(
            "deprecation reason must contain at most {MAX_REASON_CHARS} characters"
        )));
    }
    Ok(reason.to_string())
}

fn taxonomy_cursor_key(term: &TaxonomyTerm) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";

    let mut key = String::with_capacity((term.scheme.len() + term.code.len()) * 2 + 1);
    for byte in term.scheme.bytes() {
        key.push(char::from(HEX[usize::from(byte >> 4)]));
        key.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    key.push('/');
    for byte in term.code.bytes() {
        key.push(char::from(HEX[usize::from(byte >> 4)]));
        key.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    key
}

/// Applies stable-key cursor paging without a positional index parameter.
fn page_records<T>(mut records: Vec<(String, T)>, request: &PageRequest) -> Page<T> {
    records.sort_by(|left, right| left.0.cmp(&right.0));
    let after = request.after.as_ref().map(Cursor::as_str);
    let mut selected: Vec<(String, T)> = records
        .into_iter()
        .filter(|(key, _)| after.is_none_or(|cursor| key.as_str() > cursor))
        .take(usize::from(request.size.get()) + 1)
        .collect();
    let has_more = selected.len() > usize::from(request.size.get());
    if has_more {
        selected.pop();
    }
    let next_cursor = if has_more {
        selected
            .last()
            .map(|(key, _)| Cursor::from_stable_key(key.clone()))
    } else {
        None
    };
    Page {
        items: selected.into_iter().map(|(_, item)| item).collect(),
        next_cursor,
    }
}
