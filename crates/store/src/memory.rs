//! In-memory Store backend (WP-C4, MOD-STO).

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
    ActivityTimestamp, AssignmentEnrollment, AssignmentId, AssignmentRun, AttemptResult,
    AttemptTimerRecord, CatalogCapabilityFacet, CatalogLicenseFacet, CatalogLicenseValue,
    CatalogLifecycle, CatalogProblemDetail, CatalogProblemSummary, CatalogSearchFacets,
    CatalogSearchPage, CatalogSearchQuery, CatalogStatisticsAvailability, CatalogStatisticsFacet,
    CatalogTaxonomyFacet, CourseId, CourseRole, CourseSummary, EnrollmentId, EnrollmentStatus,
    MAX_CATALOG_TAXONOMY_FACETS, ProblemId, ProblemVersionRef, PublicationScope, QuestionAttempt,
    QuestionAttemptId, QuestionStatisticsDisclosure, RunId, RunMode, StatisticsDisclosurePolicy,
    StudentAssignmentSummary, StudentResponse, TenantId, UserId, UserRole, VersionId, WorkspaceId,
    WorkspaceImportId,
};

use crate::gradebook_cursor::GradebookCursor;
use crate::retention::RetentionApiAction;
use crate::run_summary_cursor::RunSummaryCursor;
use crate::statistics::{StatisticsContribution, derive_statistics_contributions};
use crate::{
    ActivityTransition, AssetAccessEvent, AssetDeliveryId, AssetDeliveryRecord, AssetDeliveryScope,
    AssetStore, AssignmentDefinitionDisposition, AssignmentRecord, AssignmentRevision,
    AssignmentUpdate, AuthorizedAssetDelivery, CatalogAssetBinding, CatalogSourceStore,
    CatalogStore, CatalogTransition, CourseListScope, CourseRecord, CourseRecordsAccessStore,
    CourseRetentionRecord, CourseRetentionSnapshot, CourseRetentionState, CourseRetentionView,
    Cursor, DraftRecord, FeedbackReleaseRecord, InstitutionRetentionPolicy,
    IssueQuestionAttemptCommand, Page, PageRequest, PageSize, PrefetchedQuestion,
    PublishDraftCommand, PublishedProblemRecord, PublishedSourceArtifact,
    RETENTION_JOB_MAX_ATTEMPTS, ReleaseAttemptFeedbackCommand, ReservePrefetchedQuestionCommand,
    RetentionApiStore, RetentionCleanupManifest, RetentionDays, RetentionDispatchBatch,
    RetentionRevision, RetentionScheduleStore, RetentionStore, RetentionWork,
    RetentionWorkerCommand, RetentionWorkerStore, RunSummaryOutcomeInput, RunSummaryPageInput,
    SessionLifetime, SessionRecord, SessionStore, SessionSubject, SessionTokenHash, Store,
    StoreError, StoredAssignment, SubmissionIdempotencyKey, SubmissionNextAttempt,
    SubmissionRecord, SubmitQuestionAttemptCommand, TenantContext, WorkspaceDraft,
    WorkspaceDraftRevision, WorkspaceDraftRole, completed_run_score, decode_catalog_search_cursor,
    encode_catalog_search_cursor, ensure_tenant, grade_policy, private_feedback_record,
    project_enrollment_completion, summary_transition, validate_asset_delivery,
    validate_assignment, validate_course, validate_draft, validate_published,
    validate_qti_publication_promotion,
};

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
    state
        .retention_stages
        .get(&(
            tenant,
            course,
            crate::RetentionStage::ArchiveStudentRecords,
            generation,
        ))
        .is_none_or(|stage| stage.state != RetentionStageWorkState::Started)
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
    BeginExternalToolGradeCommand, CommitExternalToolSubmissionCommand,
    CommitVerifiedExternalToolSubmissionCommand, CreateExternalToolLaunchSessionCommand,
    CreatedExternalToolLaunchSession, ExternalToolBegin, ExternalToolBinding,
    ExternalToolBrokerStore, ExternalToolLaunchProof, ExternalToolLaunchSessionStore,
    ExternalToolLaunchToken, ExternalToolLease, ExternalToolLeaseToken,
    ExternalToolVerifiedPending, PersistedCorrelation, ResolvedExternalToolLaunchSession,
    StageExternalToolVerificationCommand, fresh_external_tool_launch_id,
};
use crate::{
    ClaimedJob, CreateAssignmentExport, EnqueueJob, ExportArtifactKind, ExportArtifactRecord,
    ExportCommitDisposition, ExportId, ExportJobCommit, ExportJobStore, JobFailureDisposition,
    JobFailureKind, JobId, JobLeaseDuration, JobLeaseToken, JobPayload, JobState, JobStore,
    QueueDepth, StudentExportArtifactView, StudentExportJob, StudentExportState, StudentExportView,
    TenantJobView,
};
use crate::{
    CommitPreparedQtiImport, CommitPreparedQtiImportOutcome, CreateQtiImportCommand,
    QtiGradingStore, QtiImportGradingPayload, QtiImportRegistry, QtiImportStore,
    validate_qti_import,
};
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
}

/// All maps use tenant ID in their key for tenant-owned records.
#[derive(Debug, Default)]
struct State {
    authoritative_time: ActivityTimestamp,
    sessions: BTreeMap<SessionTokenHash, StoredSession>,
    catalog_grants: BTreeSet<(TenantId, ProblemId, VersionId)>,
    drafts: BTreeMap<(TenantId, WorkspaceId), DraftRecord>,
    draft_revisions: BTreeMap<(TenantId, WorkspaceId), WorkspaceDraftRevision>,
    draft_access: BTreeMap<(TenantId, WorkspaceId, UserId), WorkspaceDraftRole>,
    published: BTreeMap<(ProblemId, VersionId), PublishedProblemRecord>,
    source_artifacts: BTreeMap<(ProblemId, VersionId), PublishedSourceArtifact>,
    qti_imports: BTreeMap<(TenantId, WorkspaceId, WorkspaceImportId), QtiImportRegistry>,
    prepared_qti_imports: BTreeMap<(TenantId, WorkspaceId, WorkspaceImportId), QtiImportRegistry>,
    qti_grading:
        BTreeMap<(TenantId, WorkspaceId, WorkspaceImportId, String), QtiImportGradingPayload>,
    published_qti_grading: BTreeMap<(ProblemId, VersionId, String), QtiImportGradingPayload>,
    prepared_qti_grading:
        BTreeMap<(TenantId, WorkspaceId, WorkspaceImportId, String), QtiImportGradingPayload>,
    courses: BTreeMap<(TenantId, CourseId), CourseRecord>,
    assignments: BTreeMap<(TenantId, AssignmentId), AssignmentRecord>,
    assignment_revisions: BTreeMap<(TenantId, AssignmentId), AssignmentRevision>,
    enrollments: BTreeMap<(TenantId, EnrollmentId), AssignmentEnrollment>,
    runs: BTreeMap<(TenantId, RunId), AssignmentRun>,
    attempts: BTreeMap<(TenantId, QuestionAttemptId), QuestionAttempt>,
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

#[derive(Debug)]
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
impl JobStore for MemoryStore {
    async fn enqueue_job(
        &self,
        context: TenantContext,
        job: EnqueueJob,
    ) -> Result<JobId, StoreError> {
        ensure_tenant(context, job.tenant)?;
        job.validate()?;
        let id = JobId::generate()?;
        let mut state = self.write_state()?;
        let available_at = state.authoritative_time;
        state.jobs.insert(
            id,
            StoredJob {
                tenant: job.tenant,
                payload: job.payload,
                state: JobState::Ready,
                available_at,
                lease_token: None,
                lease_expires_at: None,
                attempt_count: 0,
                max_attempts: job.max_attempts,
                failure: None,
            },
        );
        Ok(id)
    }

    async fn claim_next_job(
        &self,
        lease: JobLeaseDuration,
    ) -> Result<Option<ClaimedJob>, StoreError> {
        let token = JobLeaseToken::generate()?;
        let mut state = self.write_state()?;
        let now = state.authoritative_time;

        // A worker killed after its last allowed claim cannot leave a permanent
        // leased row. Mark it dead before selecting eligible work.
        let mut expired_export_jobs = Vec::new();
        for (id, job) in &mut state.jobs {
            if job.state == JobState::Leased
                && job.lease_expires_at.is_some_and(|expiry| expiry <= now)
                && job.attempt_count >= job.max_attempts
            {
                job.state = JobState::Dead;
                job.lease_token = None;
                job.lease_expires_at = None;
                job.failure = Some(JobFailureKind::TimedOut);
                expired_export_jobs.push(*id);
            }
        }
        for id in expired_export_jobs {
            mark_export_failed(&mut state, id);
        }

        let id = state.jobs.iter().find_map(|(id, job)| {
            let ready = job.state == JobState::Ready && job.available_at <= now;
            let expired = job.state == JobState::Leased
                && job.lease_expires_at.is_some_and(|expiry| expiry <= now)
                && job.attempt_count < job.max_attempts;
            (ready || expired).then_some(*id)
        });
        let Some(id) = id else {
            return Ok(None);
        };
        let job = state
            .jobs
            .get_mut(&id)
            .expect("selected job remains present");
        job.state = JobState::Leased;
        job.attempt_count = job
            .attempt_count
            .checked_add(1)
            .ok_or_else(|| StoreError::InvalidRecord("job attempts overflow".to_string()))?;
        job.lease_token = Some(token);
        job.lease_expires_at = Some(add_job_seconds(now, lease.seconds())?);
        job.failure = None;
        Ok(Some(ClaimedJob {
            id,
            tenant: job.tenant,
            payload: job.payload.clone(),
            lease_token: token,
            attempt_count: job.attempt_count,
        }))
    }

    async fn complete_job(&self, id: JobId, token: JobLeaseToken) -> Result<(), StoreError> {
        let mut state = self.write_state()?;
        let now = state.authoritative_time;
        let job = state.jobs.get_mut(&id).ok_or(StoreError::NotFound)?;
        if job.state != JobState::Leased
            || job.lease_token != Some(token)
            || !job.lease_expires_at.is_some_and(|expiry| expiry > now)
        {
            return Err(StoreError::Conflict);
        }
        job.state = JobState::Completed;
        job.lease_token = None;
        job.lease_expires_at = None;
        Ok(())
    }

    async fn fail_job(
        &self,
        id: JobId,
        token: JobLeaseToken,
        failure: JobFailureKind,
    ) -> Result<JobFailureDisposition, StoreError> {
        let mut state = self.write_state()?;
        let now = state.authoritative_time;
        let job = state.jobs.get_mut(&id).ok_or(StoreError::NotFound)?;
        if job.state != JobState::Leased
            || job.lease_token != Some(token)
            || !job.lease_expires_at.is_some_and(|expiry| expiry > now)
        {
            return Err(StoreError::Conflict);
        }
        job.lease_token = None;
        job.lease_expires_at = None;
        job.failure = Some(failure);
        if failure == JobFailureKind::Permanent || job.attempt_count >= job.max_attempts {
            job.state = JobState::Dead;
            mark_export_failed(&mut state, id);
            return Ok(JobFailureDisposition::Dead);
        }
        let delay_seconds = retry_delay_seconds(job.attempt_count);
        job.state = JobState::Ready;
        job.available_at = add_job_seconds(now, delay_seconds)?;
        Ok(JobFailureDisposition::Retrying)
    }

    async fn get_job(
        &self,
        context: TenantContext,
        id: JobId,
    ) -> Result<Option<TenantJobView>, StoreError> {
        let state = self.read_state()?;
        Ok(state
            .jobs
            .get(&id)
            .filter(|job| job.tenant == context.tenant_id())
            .map(|job| TenantJobView {
                id,
                payload: job.payload.clone(),
                state: job.state,
                attempt_count: job.attempt_count,
            }))
    }

    async fn ready_queue_depth(&self) -> Result<QueueDepth, StoreError> {
        let state = self.read_state()?;
        let ready = state
            .jobs
            .values()
            .filter(|job| {
                job.state == JobState::Ready && job.available_at <= state.authoritative_time
            })
            .count();
        Ok(QueueDepth {
            ready: u64::try_from(ready).expect("queue length fits u64"),
        })
    }
}

#[async_trait]
impl ExportJobStore for MemoryStore {
    async fn create_assignment_export(
        &self,
        context: TenantContext,
        request: CreateAssignmentExport,
    ) -> Result<StudentExportView, StoreError> {
        if !(1..=20).contains(&request.max_attempts) {
            return Err(StoreError::InvalidRecord(
                "job max attempts must be between 1 and 20".to_string(),
            ));
        }
        let export = ExportId::generate()?;
        let manifest = fresh_export_object_id()?;
        let job = JobId::generate()?;
        let mut expected = BTreeMap::new();
        for kind in ExportArtifactKind::ALL {
            expected.insert(kind, fresh_export_object_id()?);
        }
        let mut state = self.write_state()?;
        let assignment = state
            .assignments
            .get(&(context.tenant_id(), request.assignment))
            .ok_or(StoreError::NotFound)?;
        if assignment.tenant != context.tenant_id() {
            return Err(StoreError::NotFound);
        }
        let record = StoredExport {
            course: assignment.course_id,
            assignment: assignment.id,
            title: assignment.title.clone(),
            requested_by: request.requested_by,
            manifest,
            problems: assignment
                .problems
                .iter()
                .map(|reference| ProblemVersionRef {
                    problem: reference.problem,
                    version: reference.version,
                })
                .collect(),
            job,
            state: StudentExportState::Queued,
            expected,
            artifacts: None,
        };
        state.exports.insert((context.tenant_id(), export), record);
        let available_at = state.authoritative_time;
        state.jobs.insert(
            job,
            StoredJob {
                tenant: context.tenant_id(),
                payload: JobPayload::Export {
                    delivery_object: manifest,
                },
                state: JobState::Ready,
                available_at,
                lease_token: None,
                lease_expires_at: None,
                attempt_count: 0,
                max_attempts: request.max_attempts,
                failure: None,
            },
        );
        Ok(StudentExportView {
            id: export,
            assignment: request.assignment,
            state: StudentExportState::Queued,
            artifacts: None,
        })
    }

    async fn get_assignment_export(
        &self,
        context: TenantContext,
        export: ExportId,
    ) -> Result<Option<StudentExportView>, StoreError> {
        let state = self.read_state()?;
        Ok(state
            .exports
            .get(&(context.tenant_id(), export))
            .map(|stored| export_view(export, stored)))
    }

    async fn get_assignment_export_for_requester(
        &self,
        context: TenantContext,
        export: ExportId,
        requester: UserId,
    ) -> Result<Option<StudentExportView>, StoreError> {
        let state = self.read_state()?;
        Ok(state
            .exports
            .get(&(context.tenant_id(), export))
            .filter(|stored| stored.requested_by == requester)
            .map(|stored| export_view(export, stored)))
    }

    async fn load_export_job(
        &self,
        context: TenantContext,
        manifest: question_model::ObjectId,
    ) -> Result<Option<StudentExportJob>, StoreError> {
        let state = self.read_state()?;
        Ok(state
            .exports
            .iter()
            .find(|((tenant, _), stored)| {
                *tenant == context.tenant_id() && stored.manifest == manifest
            })
            .map(|((tenant, export), stored)| StudentExportJob {
                id: *export,
                tenant: *tenant,
                assignment: stored.assignment,
                course: stored.course,
                title: stored.title.clone(),
                requested_by: stored.requested_by,
                manifest: stored.manifest,
                problems: stored.problems.clone(),
                expected_artifacts: stored
                    .expected
                    .iter()
                    .map(|(kind, object)| (*kind, *object))
                    .collect(),
            }))
    }

    async fn commit_export_effect(
        &self,
        context: TenantContext,
        commit: ExportJobCommit,
    ) -> Result<ExportCommitDisposition, StoreError> {
        validate_export_artifacts(context.tenant_id(), &commit.artifacts)?;
        let mut state = self.write_state()?;
        let (export, stored) = state
            .exports
            .iter()
            .find(|((tenant, _), stored)| {
                *tenant == context.tenant_id() && stored.manifest == commit.manifest
            })
            .map(|((_, export), stored)| (*export, stored.clone()))
            .ok_or(StoreError::NotFound)?;
        if stored.job != commit.job {
            return Err(StoreError::Conflict);
        }
        if stored.state == StudentExportState::Ready {
            return if stored.artifacts.as_ref() == Some(&commit.artifacts) {
                Ok(ExportCommitDisposition::AlreadyCommitted)
            } else {
                Err(StoreError::Conflict)
            };
        }
        validate_expected_export_artifacts(&stored.expected, &commit.artifacts)?;
        let now = state.authoritative_time;
        let job = state.jobs.get(&commit.job).ok_or(StoreError::NotFound)?;
        if job.tenant != context.tenant_id()
            || job.payload
                != (JobPayload::Export {
                    delivery_object: commit.manifest,
                })
            || job.state != JobState::Leased
            || job.lease_token != Some(commit.lease)
            || !job.lease_expires_at.is_some_and(|expiry| expiry > now)
        {
            return Err(StoreError::Conflict);
        }
        for artifact in &commit.artifacts {
            let delivery = crate::AssetDeliveryRecord {
                id: crate::AssetDeliveryId::from_object(artifact.object.id),
                object: artifact.object.clone(),
                scope: crate::AssetDeliveryScope::StudentRecord {
                    tenant: context.tenant_id(),
                    authorized_users: vec![stored.requested_by],
                },
            };
            crate::validate_asset_delivery(&delivery)?;
            if state.asset_deliveries.contains_key(&delivery.id) {
                return Err(StoreError::Conflict);
            }
        }
        for artifact in &commit.artifacts {
            let id = crate::AssetDeliveryId::from_object(artifact.object.id);
            state.asset_deliveries.insert(
                id,
                crate::AssetDeliveryRecord {
                    id,
                    object: artifact.object.clone(),
                    scope: crate::AssetDeliveryScope::StudentRecord {
                        tenant: context.tenant_id(),
                        authorized_users: vec![stored.requested_by],
                    },
                },
            );
        }
        let stored = state
            .exports
            .get_mut(&(context.tenant_id(), export))
            .expect("export selected from this state remains present");
        stored.state = StudentExportState::Ready;
        stored.artifacts = Some(commit.artifacts);
        let job = state
            .jobs
            .get_mut(&commit.job)
            .expect("job selected from this state remains present");
        job.state = JobState::Completed;
        job.lease_token = None;
        job.lease_expires_at = None;
        Ok(ExportCommitDisposition::Committed)
    }
}

fn fresh_export_object_id() -> Result<question_model::ObjectId, StoreError> {
    let mut bytes = [0_u8; 16];
    getrandom::fill(&mut bytes).map_err(|error| {
        StoreError::Unavailable(format!("export object ID randomness unavailable: {error}"))
    })?;
    Ok(question_model::ObjectId::from_uuid(Uuid::from_bytes(bytes)))
}

fn export_view(export: ExportId, stored: &StoredExport) -> StudentExportView {
    StudentExportView {
        id: export,
        assignment: stored.assignment,
        state: stored.state,
        artifacts: stored.artifacts.as_ref().map(|artifacts| {
            artifacts
                .iter()
                .map(|artifact| StudentExportArtifactView {
                    kind: artifact.kind,
                    filename: artifact.filename.clone(),
                    media_type: artifact.object.media_type.clone(),
                    delivery: crate::AssetDeliveryId::from_object(artifact.object.id),
                })
                .collect()
        }),
    }
}

fn validate_export_artifacts(
    tenant: TenantId,
    artifacts: &[ExportArtifactRecord],
) -> Result<(), StoreError> {
    if artifacts.len() != ExportArtifactKind::ALL.len() {
        return Err(StoreError::InvalidRecord(
            "an export effect must contain exactly four artifacts".to_string(),
        ));
    }
    let mut kinds = BTreeSet::new();
    let mut objects = BTreeSet::new();
    for artifact in artifacts {
        if !kinds.insert(artifact.kind) || !objects.insert(artifact.object.id) {
            return Err(StoreError::InvalidRecord(
                "export artifact kinds and objects must be unique".to_string(),
            ));
        }
        let expected_name = match artifact.kind {
            ExportArtifactKind::Docx => "exam.docx",
            ExportArtifactKind::Pdf => "exam.pdf",
            ExportArtifactKind::AccessibleDocx => "exam-accessible.docx",
            ExportArtifactKind::AccessiblePdf => "exam-accessible.pdf",
        };
        if artifact.filename != expected_name
            || artifact.object.media_type != artifact.kind.media_type()
            || !matches!(
                artifact.object.key,
                objects::ObjectKey::StudentRecord { tenant: key_tenant, object }
                    if key_tenant == tenant && object == artifact.object.id
            )
        {
            return Err(StoreError::InvalidRecord(
                "export artifact does not match its closed private output contract".to_string(),
            ));
        }
    }
    Ok(())
}

fn validate_expected_export_artifacts(
    expected: &BTreeMap<ExportArtifactKind, question_model::ObjectId>,
    artifacts: &[ExportArtifactRecord],
) -> Result<(), StoreError> {
    if artifacts.iter().all(|artifact| {
        expected
            .get(&artifact.kind)
            .is_some_and(|object| *object == artifact.object.id)
    }) {
        Ok(())
    } else {
        Err(StoreError::Conflict)
    }
}

fn mark_export_failed(state: &mut State, job: JobId) {
    for export in state.exports.values_mut() {
        if export.job == job && export.state == StudentExportState::Queued {
            export.state = StudentExportState::Failed;
        }
    }
}

fn retry_delay_seconds(attempt_count: u16) -> i32 {
    // 1, 2, 4, ... capped at five minutes. This is intentionally deterministic
    // so a later worker cannot turn retry timing into a second policy engine.
    let shift = u32::from(attempt_count.saturating_sub(1)).min(8);
    i32::try_from(1_u32 << shift).expect("bounded retry delay fits i32")
}

fn add_job_seconds(now: ActivityTimestamp, seconds: i32) -> Result<ActivityTimestamp, StoreError> {
    now.as_unix_millis()
        .checked_add(i64::from(seconds) * 1_000)
        .map(ActivityTimestamp::from_unix_millis)
        .ok_or_else(|| StoreError::InvalidRecord("job timestamp overflow".to_string()))
}

#[async_trait]
impl AssetStore for MemoryStore {
    async fn register_asset_delivery(
        &self,
        context: TenantContext,
        record: AssetDeliveryRecord,
    ) -> Result<(), StoreError> {
        validate_asset_delivery(&record)?;
        let mut state = self.write_state()?;
        match &record.scope {
            AssetDeliveryScope::Catalog { reference, .. } => {
                let published = state
                    .published
                    .get(&(reference.problem, reference.version))
                    .ok_or(StoreError::NotFound)?;
                if !catalog_record_visible(&state, context.tenant_id(), published) {
                    return Err(StoreError::NotFound);
                }
            }
            AssetDeliveryScope::StudentRecord { tenant, .. } => {
                ensure_tenant(context, *tenant)?;
            }
        }
        if state.asset_deliveries.contains_key(&record.id) {
            return Err(StoreError::AlreadyExists);
        }
        state.asset_deliveries.insert(record.id, record);
        Ok(())
    }

    async fn get_public_asset_delivery(
        &self,
        delivery: AssetDeliveryId,
    ) -> Result<Option<AssetDeliveryRecord>, StoreError> {
        let state = self.read_state()?;
        let Some(record) = state.asset_deliveries.get(&delivery) else {
            return Ok(None);
        };
        let AssetDeliveryScope::Catalog { reference, .. } = record.scope else {
            return Ok(None);
        };
        Ok(state
            .published
            .get(&(reference.problem, reference.version))
            .filter(|published| published.scope == PublicationScope::Public)
            .map(|_| record.clone()))
    }

    async fn catalog_asset_bindings(
        &self,
        context: TenantContext,
        reference: ProblemVersionRef,
    ) -> Result<Vec<CatalogAssetBinding>, StoreError> {
        let state = self.read_state()?;
        let Some(published) = state.published.get(&(reference.problem, reference.version)) else {
            return Ok(Vec::new());
        };
        if !catalog_record_visible(&state, context.tenant_id(), published) {
            return Ok(Vec::new());
        }

        let mut bindings = state
            .asset_deliveries
            .values()
            .filter_map(|record| match record.scope {
                AssetDeliveryScope::Catalog {
                    asset,
                    reference: asset_reference,
                } if asset_reference == reference => Some(CatalogAssetBinding {
                    asset,
                    object: record.object.id,
                }),
                AssetDeliveryScope::Catalog { .. } | AssetDeliveryScope::StudentRecord { .. } => {
                    None
                }
            })
            .collect::<Vec<_>>();
        bindings.sort_unstable_by_key(|binding| binding.asset);
        Ok(bindings)
    }

    async fn authorize_asset_delivery(
        &self,
        context: TenantContext,
        actor: UserId,
        delivery: AssetDeliveryId,
    ) -> Result<AuthorizedAssetDelivery, StoreError> {
        let mut state = self.write_state()?;
        let record = state
            .asset_deliveries
            .get(&delivery)
            .cloned()
            .ok_or(StoreError::NotFound)?;
        let authorized = match &record.scope {
            AssetDeliveryScope::Catalog { reference, .. } => state
                .published
                .get(&(reference.problem, reference.version))
                .is_some_and(|published| {
                    catalog_record_visible(&state, context.tenant_id(), published)
                }),
            AssetDeliveryScope::StudentRecord {
                tenant,
                authorized_users,
            } => *tenant == context.tenant_id() && authorized_users.contains(&actor),
        };
        if !authorized {
            return Err(StoreError::NotFound);
        }
        let authorized_at = state.authoritative_time;
        state.asset_access_events.push(AssetAccessEvent {
            tenant: context.tenant_id(),
            actor,
            delivery,
            object: record.object.id,
            bucket: record.object.bucket,
            occurred_at: authorized_at,
        });
        Ok(AuthorizedAssetDelivery {
            record,
            authorized_at,
        })
    }
}

#[async_trait]
impl CatalogStore for MemoryStore {
    async fn publish_draft(
        &self,
        context: TenantContext,
        actor: UserId,
        command: PublishDraftCommand,
    ) -> Result<PublishedProblemRecord, StoreError> {
        ensure_tenant(context, command.expected_draft.tenant)?;
        validate_draft(&command.expected_draft)?;
        crate::validate_publication_source(&command.expected_draft, &command.published_source)?;
        crate::validate_source_artifact(
            command.publication,
            &command.published_source,
            command.source_artifact.as_ref(),
        )?;
        let qti_promotion = match (
            &command.expected_draft.question.source,
            command.qti_promotion.as_ref(),
        ) {
            (question_model::DraftQuestionSource::Qti { .. }, Some(promotion)) => Some(promotion),
            (question_model::DraftQuestionSource::Qti { .. }, None) | (_, Some(_)) => {
                return Err(StoreError::InvalidRecord(
                    "QTI publication requires dedicated committed staging evidence".to_string(),
                ));
            }
            (_, None) => None,
        };
        if qti_promotion.is_some() && command.publisher != actor {
            return Err(StoreError::InvalidRecord(
                "QTI promotion actor must be the authenticated publisher".to_string(),
            ));
        }
        let mut state = self.write_state()?;
        let draft_key = (
            context.tenant_id(),
            command.expected_draft.question.workspace,
        );
        if command.publisher != actor
            || state.draft_access.get(&(
                context.tenant_id(),
                command.expected_draft.question.workspace,
                actor,
            )) != Some(&WorkspaceDraftRole::Owner)
        {
            return Err(StoreError::Forbidden);
        }
        match state.drafts.get(&draft_key) {
            Some(stored) if stored == &command.expected_draft => {}
            Some(_) => return Err(StoreError::Conflict),
            None => return Err(StoreError::NotFound),
        }
        if state.draft_revisions.get(&draft_key).copied() != Some(command.expected_revision) {
            return Err(StoreError::Conflict);
        }
        let qti_grading = if let Some(promotion) = qti_promotion {
            let registry = state
                .qti_imports
                .get(&(
                    promotion.staging.tenant,
                    promotion.staging.workspace,
                    promotion.staging.import,
                ))
                .ok_or(StoreError::NotFound)?;
            validate_qti_publication_promotion(context, &command, promotion, registry)?;
            let question_model::DraftQuestionSource::Qti { item_id, .. } =
                &command.expected_draft.question.source
            else {
                unreachable!("QTI promotion was matched against a QTI draft");
            };
            let material = state
                .qti_grading
                .get(&(
                    promotion.staging.tenant,
                    promotion.staging.workspace,
                    promotion.staging.import,
                    item_id.clone(),
                ))
                .cloned()
                .ok_or(StoreError::Conflict)?;
            for asset in &promotion.assets {
                if state.asset_deliveries.contains_key(&asset.id)
                    || state
                        .asset_deliveries
                        .values()
                        .any(|existing| existing.object.id == asset.object.id)
                {
                    return Err(StoreError::AlreadyExists);
                }
            }
            Some((item_id.clone(), material))
        } else {
            None
        };
        let publication = command.publication;
        if state
            .published
            .contains_key(&(publication.problem, publication.version))
        {
            return Err(StoreError::AlreadyExists);
        }

        let (authors, previous_version, derived_from) =
            if let Some(revises) = command.expected_draft.revises {
                if publication.problem != revises.problem {
                    return Err(StoreError::InvalidRecord(
                        "revision must remain in its existing problem chain".to_string(),
                    ));
                }
                let base = state
                    .published
                    .get(&(revises.problem, revises.version))
                    .ok_or(StoreError::NotFound)?;
                if !catalog_record_visible(&state, context.tenant_id(), base) {
                    return Err(StoreError::NotFound);
                }
                if !base.authors.contains(&command.publisher) {
                    return Err(StoreError::Forbidden);
                }
                if state.published.values().any(|record| {
                    record.problem == revises.problem
                        && record.previous_version == Some(revises.version)
                }) {
                    return Err(StoreError::Conflict);
                }
                (
                    base.authors.clone(),
                    Some(revises.version),
                    base.derived_from,
                )
            } else {
                if state
                    .published
                    .keys()
                    .any(|(problem, _)| *problem == publication.problem)
                {
                    return Err(StoreError::AlreadyExists);
                }
                if let Some(source) = command.expected_draft.derived_from {
                    let source_record = state
                        .published
                        .get(&(source.problem, source.version))
                        .ok_or(StoreError::NotFound)?;
                    if !catalog_record_visible(&state, context.tenant_id(), source_record) {
                        return Err(StoreError::NotFound);
                    }
                }
                (
                    vec![command.publisher],
                    None,
                    command.expected_draft.derived_from,
                )
            };

        let question = question_model::QuestionDefinition::from_draft(
            command.expected_draft.question.clone(),
            publication.problem,
            publication.version,
            command.published_source.clone(),
        );
        let record = PublishedProblemRecord {
            problem: publication.problem,
            version: publication.version,
            question,
            capabilities: command.capabilities,
            scope: command.scope,
            lifecycle: CatalogLifecycle::Published,
            authors,
            previous_version,
            derived_from,
            published_at: state.authoritative_time,
        };
        validate_published(&record)?;
        if record.scope == PublicationScope::Institution {
            state
                .catalog_grants
                .insert((context.tenant_id(), record.problem, record.version));
        }
        state
            .published
            .insert((record.problem, record.version), record.clone());
        if let Some(artifact) = command.source_artifact {
            state
                .source_artifacts
                .insert((publication.problem, publication.version), artifact);
        }
        if let Some(promotion) = command.qti_promotion {
            for asset in promotion.assets {
                state.asset_deliveries.insert(asset.id, asset);
            }
        }
        if let Some((item_id, material)) = qti_grading {
            state.published_qti_grading.insert(
                (publication.problem, publication.version, item_id),
                material,
            );
        }
        state.drafts.remove(&draft_key);
        state.draft_revisions.remove(&draft_key);
        state
            .draft_access
            .retain(|(tenant, workspace, _), _| (*tenant, *workspace) != draft_key);
        Ok(record)
    }

    async fn get_catalog_problem(
        &self,
        context: TenantContext,
        reference: ProblemVersionRef,
    ) -> Result<Option<PublishedProblemRecord>, StoreError> {
        let state = self.read_state()?;
        Ok(state
            .published
            .get(&(reference.problem, reference.version))
            .filter(|record| catalog_record_visible(&state, context.tenant_id(), record))
            .cloned())
    }

    async fn list_catalog(
        &self,
        context: TenantContext,
        page: PageRequest,
    ) -> Result<Page<CatalogProblemSummary>, StoreError> {
        let state = self.read_state()?;
        let records = state
            .published
            .iter()
            .filter(|(_, record)| {
                record.lifecycle.is_discoverable()
                    && catalog_record_visible(&state, context.tenant_id(), record)
            })
            .map(|((problem, version), record)| (format!("{problem}/{version}"), record.summary()))
            .collect();
        Ok(page_records(records, &page))
    }

    async fn list_catalog_taxonomy(
        &self,
        context: TenantContext,
        page: PageRequest,
    ) -> Result<Page<TaxonomyTerm>, StoreError> {
        let state = self.read_state()?;
        let mut distinct = BTreeMap::new();
        for record in state.published.values().filter(|record| {
            record.lifecycle.is_discoverable()
                && catalog_record_visible(&state, context.tenant_id(), record)
        }) {
            for term in &record.question.metadata.taxonomy {
                distinct
                    .entry(taxonomy_cursor_key(term))
                    .or_insert_with(|| term.clone());
            }
        }
        Ok(page_records(distinct.into_iter().collect(), &page))
    }

    async fn search_catalog(
        &self,
        context: TenantContext,
        query: CatalogSearchQuery,
    ) -> Result<CatalogSearchPage, StoreError> {
        let query = query
            .normalized()
            .map_err(|error| StoreError::InvalidRecord(error.to_string()))?;
        let page = search_page_request(&query)?;
        let fingerprint = catalog_search_fingerprint(&query);
        let after = page
            .after
            .as_ref()
            .map(|cursor| decode_catalog_search_cursor(cursor.as_str(), &fingerprint))
            .transpose()?
            .map(|(problem, version)| format!("{problem}/{version}"));
        let state = self.read_state()?;
        let matching = state
            .published
            .iter()
            .filter_map(|((problem, version), record)| {
                if !record.lifecycle.is_discoverable()
                    || !catalog_record_visible(&state, context.tenant_id(), record)
                {
                    return None;
                }
                let statistics_available = state
                    .question_statistics
                    .get(&(*problem, *version))
                    .is_some_and(|aggregate| {
                        matches!(
                            aggregate.disclose(StatisticsDisclosurePolicy::default()),
                            QuestionStatisticsDisclosure::Available(_)
                        )
                    });
                catalog_search_matches(record, &query, statistics_available)
                    .then(|| (format!("{problem}/{version}"), record, statistics_available))
            })
            .collect::<Vec<_>>();
        let facets = catalog_search_facets(
            matching
                .iter()
                .map(|(_, record, available)| (*record, *available)),
        );
        let mut selected = matching
            .into_iter()
            .filter(|(key, _, _)| after.as_ref().is_none_or(|cursor| key > cursor))
            .take(usize::from(page.size.get()) + 1)
            .collect::<Vec<_>>();
        let has_more = selected.len() > usize::from(page.size.get());
        if has_more {
            selected.pop();
        }
        let next_cursor = if has_more {
            selected.last().map(|(_, record, _)| {
                encode_catalog_search_cursor(
                    &fingerprint,
                    record.problem.as_uuid(),
                    record.version.as_uuid(),
                )
            })
        } else {
            None
        };
        Ok(CatalogSearchPage {
            items: selected
                .into_iter()
                .map(|(_, record, _)| record.summary())
                .collect(),
            next_cursor,
            facets,
        })
    }

    async fn get_catalog_detail(
        &self,
        context: TenantContext,
        reference: ProblemVersionRef,
    ) -> Result<Option<CatalogProblemDetail>, StoreError> {
        let state = self.read_state()?;
        Ok(state
            .published
            .get(&(reference.problem, reference.version))
            .filter(|record| catalog_record_visible(&state, context.tenant_id(), record))
            .map(|record| {
                let statistics = state
                    .question_statistics
                    .get(&(reference.problem, reference.version))
                    .map(|aggregate| aggregate.disclose(StatisticsDisclosurePolicy::default()))
                    .unwrap_or(QuestionStatisticsDisclosure::Suppressed);
                CatalogProblemDetail {
                    summary: record.summary(),
                    prompt: record.question.prompt.clone(),
                    statistics: match statistics {
                        QuestionStatisticsDisclosure::Suppressed => {
                            question_model::CatalogStatisticsStatus::Unavailable
                        }
                        QuestionStatisticsDisclosure::Available(view) => {
                            question_model::CatalogStatisticsStatus::Available(view)
                        }
                    },
                }
            }))
    }

    async fn transition_catalog_problem(
        &self,
        context: TenantContext,
        actor: UserId,
        reference: ProblemVersionRef,
        transition: CatalogTransition,
    ) -> Result<PublishedProblemRecord, StoreError> {
        let mut state = self.write_state()?;
        let key = (reference.problem, reference.version);
        let visible = state
            .published
            .get(&key)
            .is_some_and(|record| catalog_record_visible(&state, context.tenant_id(), record));
        if !visible {
            return Err(StoreError::NotFound);
        }
        let record = state.published.get_mut(&key).ok_or(StoreError::NotFound)?;
        if !record.authors.contains(&actor) {
            return Err(StoreError::Forbidden);
        }
        record.lifecycle = match (&record.lifecycle, transition) {
            (CatalogLifecycle::Published, CatalogTransition::Deprecate { reason }) => {
                let reason = validated_deprecation_reason(reason)?;
                CatalogLifecycle::Deprecated { reason }
            }
            (CatalogLifecycle::Deprecated { reason }, CatalogTransition::Archive) => {
                CatalogLifecycle::Archived {
                    reason: reason.clone(),
                }
            }
            _ => {
                return Err(StoreError::InvalidRecord(
                    "catalog lifecycle transition is not allowed".to_string(),
                ));
            }
        };
        Ok(record.clone())
    }
}

#[async_trait]
impl CatalogSourceStore for MemoryStore {
    async fn catalog_source_artifact(
        &self,
        context: TenantContext,
        reference: ProblemVersionRef,
    ) -> Result<Option<PublishedSourceArtifact>, StoreError> {
        let state = self.read_state()?;
        let Some(published) = state.published.get(&(reference.problem, reference.version)) else {
            return Ok(None);
        };
        if !catalog_record_visible(&state, context.tenant_id(), published) {
            return Ok(None);
        }
        Ok(state
            .source_artifacts
            .get(&(reference.problem, reference.version))
            .cloned())
    }
}

#[derive(Debug)]
struct StoredSession {
    record: SessionRecord,
    revoked: bool,
}

#[async_trait]
impl SessionStore for MemoryStore {
    async fn create_session(
        &self,
        token_hash: SessionTokenHash,
        subject: SessionSubject,
        lifetime: SessionLifetime,
    ) -> Result<SessionRecord, StoreError> {
        let mut state = self.write_state()?;
        if state.sessions.contains_key(&token_hash) {
            return Err(StoreError::AlreadyExists);
        }
        let created_at = state.authoritative_time;
        let lifetime_millis = i64::from(lifetime.as_seconds()) * 1_000;
        let expires_at = ActivityTimestamp::from_unix_millis(
            created_at
                .as_unix_millis()
                .checked_add(lifetime_millis)
                .ok_or_else(|| StoreError::InvalidRecord("session expiry overflow".to_string()))?,
        );
        let record = SessionRecord {
            token_hash,
            subject,
            created_at,
            expires_at,
        };
        state.sessions.insert(
            token_hash,
            StoredSession {
                record: record.clone(),
                revoked: false,
            },
        );
        Ok(record)
    }

    async fn resolve_session(
        &self,
        token_hash: SessionTokenHash,
    ) -> Result<Option<SessionRecord>, StoreError> {
        let state = self.read_state()?;
        let now = state.authoritative_time;
        Ok(state.sessions.get(&token_hash).and_then(|stored| {
            (!stored.revoked && stored.record.expires_at > now).then(|| stored.record.clone())
        }))
    }

    async fn revoke_session(&self, token_hash: SessionTokenHash) -> Result<(), StoreError> {
        let mut state = self.write_state()?;
        if let Some(stored) = state.sessions.get_mut(&token_hash) {
            stored.revoked = true;
        }
        Ok(())
    }
}

#[async_trait]
impl RetentionStore for MemoryStore {
    async fn configure_retention_policy(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        policy: InstitutionRetentionPolicy,
    ) -> Result<(), StoreError> {
        let mut state = self.write_state()?;
        let subject = active_retention_session(&state, context, session)?;
        if !subject.roles().contains(&UserRole::Administrator) {
            return Err(StoreError::Forbidden);
        }
        state.retention_policies.insert(context.tenant_id(), policy);
        Ok(())
    }

    async fn end_course_retention(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        course: CourseId,
    ) -> Result<CourseRetentionRecord, StoreError> {
        let mut state = self.write_state()?;
        let subject = active_retention_session(&state, context, session)?;
        ensure_retention_course_authority(
            &state,
            context,
            subject.user(),
            subject.roles(),
            course,
        )?;
        let key = (context.tenant_id(), course);
        if let Some(existing) = state.course_retention.get(&key).copied() {
            return Ok(existing);
        }
        let policy = state
            .retention_policies
            .get(&context.tenant_id())
            .copied()
            .unwrap_or_default();
        let snapshot = CourseRetentionSnapshot::new(
            state.authoritative_time,
            policy,
            AssignmentDefinitionDisposition::Retain,
            1,
        )
        .map_err(|error| StoreError::InvalidRecord(error.to_string()))?;
        let record = CourseRetentionRecord {
            snapshot,
            status: crate::CourseRetentionStatus::from_persisted(
                CourseRetentionState::Active,
                AssignmentDefinitionDisposition::Retain,
            ),
        };
        state.course_retention.insert(key, record);
        for stage in [
            crate::RetentionStage::Notify,
            crate::RetentionStage::ArchiveStudentRecords,
            crate::RetentionStage::DeleteStudentRecords,
        ] {
            let due_at = policy
                .due_at(state.authoritative_time, stage)
                .map_err(|error| StoreError::InvalidRecord(error.to_string()))?;
            state.retention_stages.insert(
                (context.tenant_id(), course, stage, 1),
                StoredRetentionStage {
                    due_at,
                    state: RetentionStageWorkState::Scheduled,
                    job: None,
                    lease: None,
                },
            );
        }
        Ok(record)
    }

    async fn course_retention(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        course: CourseId,
    ) -> Result<Option<CourseRetentionRecord>, StoreError> {
        let state = self.read_state()?;
        let subject = active_retention_session(&state, context, session)?;
        if ensure_retention_course_authority(
            &state,
            context,
            subject.user(),
            subject.roles(),
            course,
        )
        .is_err()
        {
            return Ok(None);
        }
        Ok(state
            .course_retention
            .get(&(context.tenant_id(), course))
            .copied())
    }
}

#[async_trait]
impl RetentionScheduleStore for MemoryStore {
    async fn dispatch_due_retention_stages(
        &self,
        batch: RetentionDispatchBatch,
    ) -> Result<u16, StoreError> {
        let mut state = self.write_state()?;
        let now = state.authoritative_time;
        let mut candidates = Vec::new();
        for (key @ (tenant, course, stage, generation), stored) in &state.retention_stages {
            if candidates.len() >= usize::from(batch.get())
                || stored.state != RetentionStageWorkState::Scheduled
                || stored.due_at > now
                || state.retention_dispatches.contains_key(key)
            {
                continue;
            }
            let Some(record) = state.course_retention.get(&(*tenant, *course)) else {
                continue;
            };
            if record.snapshot.generation() != *generation
                || (record.status.state != CourseRetentionState::Active
                    && !(record.status.state == CourseRetentionState::StudentRecordsArchived
                        && *stage == crate::RetentionStage::DeleteStudentRecords))
            {
                continue;
            }
            candidates.push((*key, *stage));
        }
        let mut jobs = Vec::with_capacity(candidates.len());
        for (key, stage) in &candidates {
            jobs.push((
                *key,
                crate::JobId::generate()?,
                JobPayload::Retention {
                    course: key.1,
                    stage: *stage,
                    generation: key.3,
                },
            ));
        }
        for (key, id, payload) in &jobs {
            state.jobs.insert(
                *id,
                StoredJob {
                    tenant: key.0,
                    payload: payload.clone(),
                    state: JobState::Ready,
                    available_at: now,
                    lease_token: None,
                    lease_expires_at: None,
                    attempt_count: 0,
                    max_attempts: RETENTION_JOB_MAX_ATTEMPTS,
                    failure: None,
                },
            );
            state.retention_dispatches.insert(*key, *id);
        }
        u16::try_from(jobs.len()).map_err(|_| {
            StoreError::Unavailable("retention dispatch count exceeds u16".to_string())
        })
    }

    async fn extend_course_retention(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        course: CourseId,
        additional_days: RetentionDays,
    ) -> Result<CourseRetentionRecord, StoreError> {
        let mut state = self.write_state()?;
        let subject = active_retention_session(&state, context, session)?;
        if !subject.roles().contains(&UserRole::Administrator) {
            return Err(StoreError::Forbidden);
        }
        let key = (context.tenant_id(), course);
        if !state.courses.contains_key(&key) {
            return Err(StoreError::Forbidden);
        }
        let record = state
            .course_retention
            .get(&key)
            .copied()
            // An existing course with no ended schedule is a lifecycle conflict,
            // while the preceding existence guard keeps a missing course
            // nonenumerating. PostgreSQL's broker uses the same distinction.
            .ok_or(StoreError::Conflict)?;
        if record.status.state != CourseRetentionState::Active {
            return Err(StoreError::Conflict);
        }
        let old_generation = record.snapshot.generation();
        let new_generation = old_generation.checked_add(1).ok_or_else(|| {
            StoreError::InvalidRecord("retention generation overflow".to_string())
        })?;
        let stages = [
            crate::RetentionStage::Notify,
            crate::RetentionStage::ArchiveStudentRecords,
            crate::RetentionStage::DeleteStudentRecords,
        ];
        let old = stages
            .iter()
            .map(|stage| {
                state
                    .retention_stages
                    .get(&(key.0, key.1, *stage, old_generation))
                    .copied()
                    .ok_or(StoreError::Conflict)
                    .map(|stored| (*stage, stored))
            })
            .collect::<Result<Vec<_>, _>>()?;
        if old
            .iter()
            .any(|(_, stored)| stored.state == RetentionStageWorkState::Started)
        {
            return Err(StoreError::Conflict);
        }
        let shift_millis = i64::from(additional_days.get())
            .checked_mul(86_400_000)
            .ok_or_else(|| {
                StoreError::InvalidRecord("retention extension overflows".to_string())
            })?;
        let mut replacement = Vec::with_capacity(old.len());
        for (stage, stored) in &old {
            let next = match stored.state {
                RetentionStageWorkState::Completed => StoredRetentionStage {
                    due_at: stored.due_at,
                    state: RetentionStageWorkState::Completed,
                    job: None,
                    lease: None,
                },
                RetentionStageWorkState::Scheduled => StoredRetentionStage {
                    due_at: ActivityTimestamp::from_unix_millis(
                        stored
                            .due_at
                            .as_unix_millis()
                            .checked_add(shift_millis)
                            .ok_or_else(|| {
                                StoreError::InvalidRecord(
                                    "retention extension timestamp overflows".to_string(),
                                )
                            })?,
                    ),
                    state: RetentionStageWorkState::Scheduled,
                    job: None,
                    lease: None,
                },
                RetentionStageWorkState::Started | RetentionStageWorkState::Superseded => {
                    return Err(StoreError::Conflict);
                }
            };
            replacement.push((*stage, next));
        }
        for (stage, stored) in &old {
            let old_key = (key.0, key.1, *stage, old_generation);
            if stored.state == RetentionStageWorkState::Scheduled {
                if let Some(job) = state.retention_dispatches.get(&old_key).copied()
                    && let Some(job) = state.jobs.get_mut(&job)
                    && matches!(job.state, JobState::Ready | JobState::Leased)
                {
                    job.state = JobState::Dead;
                    job.lease_token = None;
                    job.lease_expires_at = None;
                    job.failure = Some(JobFailureKind::Permanent);
                }
                state.retention_stages.insert(
                    old_key,
                    StoredRetentionStage {
                        state: RetentionStageWorkState::Superseded,
                        ..*stored
                    },
                );
            }
        }
        let snapshot = record
            .snapshot
            .with_generation_and_disposition(new_generation, record.status.assignment_definitions)
            .map_err(|error| StoreError::InvalidRecord(error.to_string()))?;
        let updated = CourseRetentionRecord {
            snapshot,
            status: crate::CourseRetentionStatus::from_persisted(
                CourseRetentionState::Active,
                record.status.assignment_definitions,
            ),
        };
        state.course_retention.insert(key, updated);
        for (stage, stored) in replacement {
            state
                .retention_stages
                .insert((key.0, key.1, stage, new_generation), stored);
        }
        Ok(updated)
    }

    async fn set_archive_disposition(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        course: CourseId,
        disposition: AssignmentDefinitionDisposition,
    ) -> Result<CourseRetentionRecord, StoreError> {
        let mut state = self.write_state()?;
        let subject = active_retention_session(&state, context, session)?;
        ensure_retention_course_authority(
            &state,
            context,
            subject.user(),
            subject.roles(),
            course,
        )?;
        let key = (context.tenant_id(), course);
        let record = state
            .course_retention
            .get(&key)
            .copied()
            .ok_or(StoreError::Conflict)?;
        let archive_key = (
            key.0,
            key.1,
            crate::RetentionStage::ArchiveStudentRecords,
            record.snapshot.generation(),
        );
        if record.status.state != CourseRetentionState::Active
            || state
                .retention_stages
                .get(&archive_key)
                .is_none_or(|stage| stage.state != RetentionStageWorkState::Scheduled)
        {
            return Err(StoreError::Conflict);
        }
        let snapshot = record
            .snapshot
            .with_generation_and_disposition(record.snapshot.generation(), disposition)
            .map_err(|error| StoreError::InvalidRecord(error.to_string()))?;
        let updated = CourseRetentionRecord {
            snapshot,
            status: crate::CourseRetentionStatus::from_persisted(
                CourseRetentionState::Active,
                disposition,
            ),
        };
        state.course_retention.insert(key, updated);
        Ok(updated)
    }
}

#[async_trait]
impl RetentionApiStore for MemoryStore {
    async fn retention_view(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        course: CourseId,
    ) -> Result<Option<CourseRetentionView>, StoreError> {
        let state = self.read_state()?;
        let subject = active_retention_session(&state, context, session)?;
        if ensure_retention_course_authority(
            &state,
            context,
            subject.user(),
            subject.roles(),
            course,
        )
        .is_err()
        {
            return Ok(None);
        }
        state
            .course_retention
            .get(&(context.tenant_id(), course))
            .copied()
            .map(|record| {
                record
                    .safe_view()
                    .map_err(|error| StoreError::InvalidRecord(error.to_string()))
            })
            .transpose()
    }

    async fn retention_notification(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        course: CourseId,
    ) -> Result<Option<crate::RetentionNotificationView>, StoreError> {
        let state = self.read_state()?;
        let subject = active_retention_session(&state, context, session)?;
        if ensure_retention_course_authority(
            &state,
            context,
            subject.user(),
            subject.roles(),
            course,
        )
        .is_err()
        {
            return Ok(None);
        }
        Ok(state
            .retention_notifications
            .iter()
            .filter(|((tenant, notification_course, _), _)| {
                *tenant == context.tenant_id() && *notification_course == course
            })
            .max_by_key(|((_, _, generation), notification)| (*generation, notification.created_at))
            .map(|(_, notification)| *notification))
    }

    async fn extend_retention_if_revision(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        course: CourseId,
        expected: RetentionRevision,
        additional_days: RetentionDays,
    ) -> Result<CourseRetentionView, StoreError> {
        Ok(self
            .mutate_retention_api(
                context,
                session,
                course,
                expected,
                RetentionApiAction::Extend(additional_days),
            )?
            .retention)
    }

    async fn request_retention_archive_if_revision(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        course: CourseId,
        expected: RetentionRevision,
        disposition: AssignmentDefinitionDisposition,
    ) -> Result<crate::RetentionRequestResult, StoreError> {
        let mutation = self.mutate_retention_api(
            context,
            session,
            course,
            expected,
            RetentionApiAction::Archive(disposition),
        )?;
        Ok(crate::RetentionRequestResult {
            retention: mutation.retention,
            outcome: mutation.manual_outcome.ok_or(StoreError::Conflict)?,
        })
    }

    async fn request_retention_delete_if_revision(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        course: CourseId,
        expected: RetentionRevision,
    ) -> Result<crate::RetentionRequestResult, StoreError> {
        let mutation = self.mutate_retention_api(
            context,
            session,
            course,
            expected,
            RetentionApiAction::Delete,
        )?;
        Ok(crate::RetentionRequestResult {
            retention: mutation.retention,
            outcome: mutation.manual_outcome.ok_or(StoreError::Conflict)?,
        })
    }
}

impl MemoryStore {
    fn mutate_retention_api(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        course: CourseId,
        expected: RetentionRevision,
        action: RetentionApiAction,
    ) -> Result<RetentionApiMutation, StoreError> {
        let mut state = self.write_state()?;
        let subject = active_retention_session(&state, context, session)?;
        let actor = subject.user();
        let administrator = subject.roles().contains(&UserRole::Administrator);
        ensure_retention_course_authority(&state, context, actor, subject.roles(), course)?;
        if matches!(action, RetentionApiAction::Extend(_)) && !administrator {
            return Err(StoreError::Forbidden);
        }
        let key = (context.tenant_id(), course);
        if let Some(receipt) = state
            .retention_api_receipts
            .get(&(key.0, key.1, expected.value()))
            .copied()
        {
            if receipt.actor != actor || receipt.action != action {
                return Err(StoreError::Conflict);
            }
            let stage = state
                .retention_stages
                .get(&(key.0, key.1, receipt.stage, receipt.resulting_generation))
                .copied()
                .ok_or(StoreError::Conflict)?;
            let outcome = match stage.state {
                RetentionStageWorkState::Scheduled => crate::RetentionRequestOutcome::Scheduled,
                RetentionStageWorkState::Started => crate::RetentionRequestOutcome::InProgress,
                RetentionStageWorkState::Completed => crate::RetentionRequestOutcome::Completed,
                RetentionStageWorkState::Superseded => return Err(StoreError::Conflict),
            };
            let retention = state
                .course_retention
                .get(&key)
                .copied()
                .ok_or(StoreError::Conflict)?
                .safe_view()
                .map_err(|error| StoreError::InvalidRecord(error.to_string()))?;
            return Ok(RetentionApiMutation {
                retention,
                manual_outcome: Some(outcome),
            });
        }
        let record = state
            .course_retention
            .get(&key)
            .copied()
            .ok_or(StoreError::Conflict)?;
        if record.status.state != CourseRetentionState::Active
            || record.snapshot.generation() != expected.value()
        {
            return Err(StoreError::Conflict);
        }
        let old_generation = record.snapshot.generation();
        let new_generation = old_generation.checked_add(1).ok_or_else(|| {
            StoreError::InvalidRecord("retention generation overflow".to_string())
        })?;
        let stages = [
            crate::RetentionStage::Notify,
            crate::RetentionStage::ArchiveStudentRecords,
            crate::RetentionStage::DeleteStudentRecords,
        ];
        let old = stages
            .iter()
            .map(|stage| {
                state
                    .retention_stages
                    .get(&(key.0, key.1, *stage, old_generation))
                    .copied()
                    .ok_or(StoreError::Conflict)
                    .map(|stored| (*stage, stored))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let immediate_stage_for_replay = match action {
            RetentionApiAction::Archive(_) => Some(crate::RetentionStage::ArchiveStudentRecords),
            RetentionApiAction::Delete => Some(crate::RetentionStage::DeleteStudentRecords),
            RetentionApiAction::Extend(_) => None,
        };
        if let Some(stage) = immediate_stage_for_replay
            && let Some((_, stored)) = old.iter().find(|(candidate, _)| *candidate == stage)
        {
            let outcome = match stored.state {
                RetentionStageWorkState::Started => {
                    Some(crate::RetentionRequestOutcome::InProgress)
                }
                RetentionStageWorkState::Completed => {
                    Some(crate::RetentionRequestOutcome::Completed)
                }
                RetentionStageWorkState::Scheduled | RetentionStageWorkState::Superseded => None,
            };
            if let Some(outcome) = outcome {
                return Ok(RetentionApiMutation {
                    retention: record
                        .safe_view()
                        .map_err(|error| StoreError::InvalidRecord(error.to_string()))?,
                    manual_outcome: Some(outcome),
                });
            }
            if stored.state == RetentionStageWorkState::Scheduled
                && state
                    .retention_dispatches
                    .contains_key(&(key.0, key.1, stage, old_generation))
            {
                if matches!(action, RetentionApiAction::Archive(disposition) if disposition != record.status.assignment_definitions)
                {
                    return Err(StoreError::Conflict);
                }
                return Ok(RetentionApiMutation {
                    retention: record
                        .safe_view()
                        .map_err(|error| StoreError::InvalidRecord(error.to_string()))?,
                    manual_outcome: Some(crate::RetentionRequestOutcome::Scheduled),
                });
            }
        }
        if old
            .iter()
            .any(|(_, stored)| stored.state == RetentionStageWorkState::Started)
        {
            return Err(StoreError::Conflict);
        }
        let (extension, immediate_stage, next_disposition) = match action {
            RetentionApiAction::Extend(days) => {
                (Some(days), None, record.status.assignment_definitions)
            }
            RetentionApiAction::Archive(disposition) => (
                None,
                Some(crate::RetentionStage::ArchiveStudentRecords),
                disposition,
            ),
            RetentionApiAction::Delete => (
                None,
                Some(crate::RetentionStage::DeleteStudentRecords),
                record.status.assignment_definitions,
            ),
        };
        let shift_millis = match extension {
            Some(days) => i64::from(days.get())
                .checked_mul(86_400_000)
                .ok_or_else(|| {
                    StoreError::InvalidRecord("retention extension overflows".to_string())
                })?,
            None => 0,
        };
        let mut replacements = Vec::with_capacity(old.len());
        for (stage, stored) in &old {
            let due_at = if Some(*stage) == immediate_stage {
                state.authoritative_time
            } else if stored.state == RetentionStageWorkState::Completed {
                stored.due_at
            } else {
                ActivityTimestamp::from_unix_millis(
                    stored
                        .due_at
                        .as_unix_millis()
                        .checked_add(shift_millis)
                        .ok_or_else(|| {
                            StoreError::InvalidRecord(
                                "retention extension timestamp overflows".to_string(),
                            )
                        })?,
                )
            };
            let next_state = match stored.state {
                RetentionStageWorkState::Completed => RetentionStageWorkState::Completed,
                RetentionStageWorkState::Scheduled => RetentionStageWorkState::Scheduled,
                RetentionStageWorkState::Started | RetentionStageWorkState::Superseded => {
                    return Err(StoreError::Conflict);
                }
            };
            if Some(*stage) == immediate_stage && next_state != RetentionStageWorkState::Scheduled {
                return Err(StoreError::Conflict);
            }
            replacements.push((
                *stage,
                StoredRetentionStage {
                    due_at,
                    state: next_state,
                    job: None,
                    lease: None,
                },
            ));
        }
        for (stage, stored) in old {
            let old_key = (key.0, key.1, stage, old_generation);
            if stored.state == RetentionStageWorkState::Scheduled {
                if let Some(job_id) = state.retention_dispatches.get(&old_key).copied()
                    && let Some(job) = state.jobs.get_mut(&job_id)
                    && matches!(job.state, JobState::Ready | JobState::Leased)
                {
                    job.state = JobState::Dead;
                    job.lease_token = None;
                    job.lease_expires_at = None;
                    job.failure = Some(JobFailureKind::Permanent);
                }
                state.retention_stages.insert(
                    old_key,
                    StoredRetentionStage {
                        state: RetentionStageWorkState::Superseded,
                        ..stored
                    },
                );
            }
        }
        let snapshot = record
            .snapshot
            .with_generation_and_disposition(new_generation, next_disposition)
            .map_err(|error| StoreError::InvalidRecord(error.to_string()))?;
        let updated = CourseRetentionRecord {
            snapshot,
            status: crate::CourseRetentionStatus::from_persisted(
                CourseRetentionState::Active,
                next_disposition,
            ),
        };
        state.course_retention.insert(key, updated);
        for (stage, stored) in replacements {
            state
                .retention_stages
                .insert((key.0, key.1, stage, new_generation), stored);
        }
        if let Some(stage) = immediate_stage {
            let dispatch_key = (key.0, key.1, stage, new_generation);
            let job_id = crate::JobId::generate()?;
            let available_at = state.authoritative_time;
            state.jobs.insert(
                job_id,
                StoredJob {
                    tenant: key.0,
                    payload: JobPayload::Retention {
                        course,
                        stage,
                        generation: new_generation,
                    },
                    state: JobState::Ready,
                    available_at,
                    lease_token: None,
                    lease_expires_at: None,
                    attempt_count: 0,
                    max_attempts: RETENTION_JOB_MAX_ATTEMPTS,
                    failure: None,
                },
            );
            state.retention_dispatches.insert(dispatch_key, job_id);
            state.retention_api_receipts.insert(
                (key.0, key.1, expected.value()),
                RetentionApiReceipt {
                    actor,
                    action,
                    resulting_generation: new_generation,
                    stage,
                },
            );
        }
        Ok(RetentionApiMutation {
            retention: updated
                .safe_view()
                .map_err(|error| StoreError::InvalidRecord(error.to_string()))?,
            manual_outcome: immediate_stage.map(|_| crate::RetentionRequestOutcome::Scheduled),
        })
    }
}

#[async_trait]
impl RetentionWorkerStore for MemoryStore {
    async fn prepare_retention_work(
        &self,
        command: RetentionWorkerCommand,
    ) -> Result<RetentionWork, StoreError> {
        let mut state = self.write_state()?;
        let key = (command.tenant, command.course);
        let current = state
            .course_retention
            .get(&key)
            .copied()
            .ok_or(StoreError::NotFound)?;
        if current.snapshot.generation() != command.generation {
            return Err(StoreError::Conflict);
        }
        let stage_key = (
            command.tenant,
            command.course,
            command.stage,
            command.generation,
        );
        let stage = state
            .retention_stages
            .get(&stage_key)
            .copied()
            .ok_or(StoreError::Conflict)?;
        if state.authoritative_time < stage.due_at
            || !matches!(
                stage.state,
                RetentionStageWorkState::Scheduled | RetentionStageWorkState::Started
            )
            || (stage.state == RetentionStageWorkState::Started && stage.job != Some(command.job))
            || state.retention_dispatches.get(&stage_key) != Some(&command.job)
        {
            return Err(StoreError::Conflict);
        }
        let job = state.jobs.get(&command.job).ok_or(StoreError::NotFound)?;
        if job.tenant != command.tenant
            || job.state != crate::JobState::Leased
            || job.lease_token != Some(command.lease)
            || job.lease_expires_at <= Some(state.authoritative_time)
            || job.payload
                != (crate::JobPayload::Retention {
                    course: command.course,
                    stage: command.stage,
                    generation: command.generation,
                })
        {
            return Err(StoreError::Conflict);
        }
        state.retention_stages.insert(
            stage_key,
            StoredRetentionStage {
                due_at: stage.due_at,
                state: RetentionStageWorkState::Started,
                job: Some(command.job),
                lease: Some(command.lease),
            },
        );
        match command.stage {
            crate::RetentionStage::Notify => Ok(RetentionWork::Notify),
            crate::RetentionStage::ArchiveStudentRecords
            | crate::RetentionStage::DeleteStudentRecords => {
                let mut records = BTreeSet::new();
                let mut deliveries = Vec::new();
                let mut terminalize = Vec::new();
                for ((tenant, export_id), export) in &state.exports {
                    if *tenant != command.tenant || export.course != command.course {
                        continue;
                    }
                    if let Some(artifacts) = &export.artifacts {
                        for artifact in artifacts {
                            let objects::ObjectKey::StudentRecord { tenant, .. } =
                                &artifact.object.key
                            else {
                                return Err(StoreError::InvalidRecord(
                                    "retention manifest contains a non-student object".to_string(),
                                ));
                            };
                            if *tenant != command.tenant {
                                return Err(StoreError::TenantMismatch);
                            }
                            deliveries
                                .push(crate::AssetDeliveryId::from_object(artifact.object.id));
                            records.insert(artifact.object.key.clone());
                        }
                    } else {
                        for object in export.expected.values() {
                            records.insert(objects::ObjectKey::StudentRecord {
                                tenant: command.tenant,
                                object: *object,
                            });
                        }
                        terminalize.push((*tenant, *export_id, export.job));
                    }
                }
                for (tenant, export, job) in terminalize {
                    if let Some(export) = state.exports.get_mut(&(tenant, export)) {
                        export.state = crate::StudentExportState::Failed;
                    }
                    if let Some(job) = state.jobs.get_mut(&job) {
                        job.state = crate::JobState::Dead;
                        job.lease_token = None;
                        job.lease_expires_at = None;
                    }
                }
                // Revocation occurs before the external object delete. Repeating this
                // preparation is harmless and makes a partial delete retry safe.
                for delivery in deliveries {
                    state.asset_deliveries.remove(&delivery);
                }
                Ok(RetentionWork::Cleanup(RetentionCleanupManifest {
                    objects: records.into_iter().collect(),
                }))
            }
        }
    }

    async fn commit_retention_work(
        &self,
        command: RetentionWorkerCommand,
    ) -> Result<(), StoreError> {
        let mut state = self.write_state()?;
        let now = state.authoritative_time;
        let job = state.jobs.get(&command.job).ok_or(StoreError::NotFound)?;
        if job.tenant != command.tenant
            || job.state != crate::JobState::Leased
            || job.lease_token != Some(command.lease)
            || job.lease_expires_at <= Some(now)
            || job.payload
                != (crate::JobPayload::Retention {
                    course: command.course,
                    stage: command.stage,
                    generation: command.generation,
                })
        {
            return Err(StoreError::Conflict);
        }
        let record = state
            .course_retention
            .get(&(command.tenant, command.course))
            .copied()
            .ok_or(StoreError::NotFound)?;
        if record.snapshot.generation() != command.generation {
            return Err(StoreError::Conflict);
        }
        let stage_key = (
            command.tenant,
            command.course,
            command.stage,
            command.generation,
        );
        let stage = state
            .retention_stages
            .get(&stage_key)
            .copied()
            .ok_or(StoreError::Conflict)?;
        if stage.state != RetentionStageWorkState::Started
            || stage.job != Some(command.job)
            || stage.lease != Some(command.lease)
            || state.retention_dispatches.get(&stage_key) != Some(&command.job)
        {
            return Err(StoreError::Conflict);
        }
        if command.stage == crate::RetentionStage::ArchiveStudentRecords
            && record.status.state != CourseRetentionState::Active
        {
            return Err(StoreError::Conflict);
        }
        if command.stage == crate::RetentionStage::Notify {
            let created_at = state.authoritative_time;
            state.retention_notifications.insert(
                (command.tenant, command.course, command.generation),
                crate::RetentionNotificationView {
                    intent: crate::RetentionNotificationIntent::Archive,
                    created_at,
                },
            );
        }
        state.retention_stages.insert(
            stage_key,
            StoredRetentionStage {
                due_at: stage.due_at,
                state: RetentionStageWorkState::Completed,
                job: Some(command.job),
                lease: Some(command.lease),
            },
        );
        if command.stage == crate::RetentionStage::ArchiveStudentRecords {
            let record = state
                .course_retention
                .get_mut(&(command.tenant, command.course))
                .ok_or(StoreError::NotFound)?;
            record.status = crate::CourseRetentionStatus::from_persisted(
                CourseRetentionState::StudentRecordsArchived,
                record.snapshot.assignment_definitions(),
            );
        }
        let job = state
            .jobs
            .get_mut(&command.job)
            .ok_or(StoreError::NotFound)?;
        job.state = crate::JobState::Completed;
        job.lease_token = None;
        job.lease_expires_at = None;
        Ok(())
    }
}

fn active_retention_session(
    state: &State,
    context: TenantContext,
    session: SessionTokenHash,
) -> Result<&SessionSubject, StoreError> {
    let stored = state.sessions.get(&session).ok_or(StoreError::Forbidden)?;
    if stored.revoked
        || stored.record.expires_at <= state.authoritative_time
        || stored.record.subject.tenant() != context.tenant_id()
    {
        return Err(StoreError::Forbidden);
    }
    Ok(&stored.record.subject)
}

fn ensure_retention_course_authority(
    state: &State,
    context: TenantContext,
    user: UserId,
    roles: &[UserRole],
    course: CourseId,
) -> Result<(), StoreError> {
    let course_record = state
        .courses
        .get(&(context.tenant_id(), course))
        .ok_or(StoreError::Forbidden)?;
    if roles.contains(&UserRole::Administrator)
        || course_record.role_for(user) == Some(CourseRole::Instructor)
    {
        Ok(())
    } else {
        Err(StoreError::Forbidden)
    }
}

#[async_trait]
impl QtiImportStore for MemoryStore {
    async fn prepare_qti_import(
        &self,
        context: TenantContext,
        command: CreateQtiImportCommand,
    ) -> Result<(), StoreError> {
        ensure_tenant(context, command.registry.reference.tenant)?;
        validate_qti_import(&command)?;
        let reference = command.registry.reference;
        let key = (reference.tenant, reference.workspace, reference.import);
        let mut state = self.write_state()?;
        if state.qti_imports.contains_key(&key) {
            return Err(StoreError::Conflict);
        }
        if let Some(existing) = state.prepared_qti_imports.get(&key) {
            let exact_grading = command.item_bindings.len() == existing.items.len()
                && command.item_bindings.iter().all(|binding| {
                    state.prepared_qti_grading.get(&(
                        key.0,
                        key.1,
                        key.2,
                        binding.item.item_id.clone(),
                    )) == Some(&binding.grading)
                });
            return if existing == &command.registry && exact_grading {
                Ok(())
            } else {
                Err(StoreError::Conflict)
            };
        }
        for binding in &command.item_bindings {
            state.prepared_qti_grading.insert(
                (key.0, key.1, key.2, binding.item.item_id.clone()),
                binding.grading.clone(),
            );
        }
        state.prepared_qti_imports.insert(key, command.registry);
        Ok(())
    }

    async fn commit_prepared_qti_import(
        &self,
        context: TenantContext,
        command: CommitPreparedQtiImport,
    ) -> Result<CommitPreparedQtiImportOutcome, StoreError> {
        ensure_tenant(context, command.reference.tenant)?;
        let key = (
            command.reference.tenant,
            command.reference.workspace,
            command.reference.import,
        );
        let mut state = self.write_state()?;
        let now = state.authoritative_time;
        let active = state.jobs.get(&command.job).is_some_and(|job| {
            job.tenant == context.tenant_id()
                && job.state == JobState::Leased
                && job.lease_token == Some(command.lease)
                && job.lease_expires_at.is_some_and(|expiry| expiry > now)
                && job.payload
                    == JobPayload::QtiImport {
                        workspace: key.1,
                        import: key.2,
                        source_object: command.source_object,
                    }
        });
        if !active {
            return Ok(CommitPreparedQtiImportOutcome::ClaimNoLongerActive);
        }
        let registry = state
            .prepared_qti_imports
            .get(&key)
            .cloned()
            .ok_or(StoreError::NotFound)?;
        if registry.source.id != command.source_object {
            return Err(StoreError::Conflict);
        }
        for item in &registry.items {
            let grade_key = (key.0, key.1, key.2, item.item_id.clone());
            let material = state
                .prepared_qti_grading
                .remove(&grade_key)
                .ok_or(StoreError::Conflict)?;
            state.qti_grading.insert(grade_key, material);
        }
        state.prepared_qti_imports.remove(&key);
        state.qti_imports.insert(key, registry);
        let job = state
            .jobs
            .get_mut(&command.job)
            .ok_or(StoreError::NotFound)?;
        job.state = JobState::Completed;
        job.lease_token = None;
        job.lease_expires_at = None;
        Ok(CommitPreparedQtiImportOutcome::Committed)
    }

    async fn get_qti_import(
        &self,
        context: TenantContext,
        workspace: WorkspaceId,
        import: WorkspaceImportId,
    ) -> Result<Option<QtiImportRegistry>, StoreError> {
        Ok(self
            .read_state()?
            .qti_imports
            .get(&(context.tenant_id(), workspace, import))
            .cloned())
    }
}

#[async_trait]
impl QtiGradingStore for MemoryQtiGraderStore {
    async fn qti_import_grading(
        &self,
        context: TenantContext,
        workspace: WorkspaceId,
        import: WorkspaceImportId,
        item_id: &str,
    ) -> Result<Option<QtiImportGradingPayload>, StoreError> {
        // Require the public registry under the same tenant/workspace scope so
        // a guessed item key cannot enumerate private grading records.
        let state = self
            .state
            .read()
            .map_err(|error| StoreError::Unavailable(error.to_string()))?;
        if !state
            .qti_imports
            .contains_key(&(context.tenant_id(), workspace, import))
        {
            return Ok(None);
        }
        Ok(state
            .qti_grading
            .get(&(context.tenant_id(), workspace, import, item_id.to_string()))
            .cloned())
    }

    async fn qti_published_grading(
        &self,
        context: TenantContext,
        reference: ProblemVersionRef,
        item_id: &str,
    ) -> Result<Option<QtiImportGradingPayload>, StoreError> {
        let state = self
            .state
            .read()
            .map_err(|error| StoreError::Unavailable(error.to_string()))?;
        let Some(published) = state.published.get(&(reference.problem, reference.version)) else {
            return Ok(None);
        };
        if !catalog_record_visible(&state, context.tenant_id(), published) {
            return Ok(None);
        }
        Ok(state
            .published_qti_grading
            .get(&(reference.problem, reference.version, item_id.to_string()))
            .cloned())
    }
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
        state.drafts.remove(&key);
        state.draft_revisions.remove(&key);
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
        self.write_state()?
            .courses
            .insert((course.tenant, course.id), course);
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
                Some((course_id.to_string(), record.summary(role)))
            })
            .collect();
        Ok(page_records(records, &page))
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
        };
        state.assignments.insert(key, stored.record.clone());
        state.assignment_revisions.insert(key, stored.revision);
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
            problems: update.problems,
            policies: update.policies,
        };
        validate_assignment(&assignment)?;
        validate_memory_assignment_references(&state, context, &assignment)?;
        let stored = StoredAssignment {
            record: assignment,
            revision: current.next()?,
        };
        state.assignments.insert(key, stored.record.clone());
        state.assignment_revisions.insert(key, stored.revision);
        Ok(stored)
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
        Ok(Some(StoredAssignment { record, revision }))
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
        state.runs.insert((tenant, run.id), run.clone());
        state.summaries.insert((tenant, enrollment.id), next);
        Ok(run)
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
        validate_assignment_position(&assignment, &command)?;

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
                    && !state.submissions.contains_key(&(tenant, attempt.id))
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
                return Ok(active);
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
            })
            .max_by_key(|attempt| (attempt.timer.issued_at, attempt.id));
        if latest_for_position.is_some_and(|latest| {
            state
                .submissions
                .get(&(tenant, latest.id))
                .and_then(|submission| submission.record.attempt.result)
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
        let timer = issued_timer(
            state.authoritative_time,
            &run,
            question.question.timing_policy,
        )?;
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
            .problems
            .get(
                usize::try_from(reservation.assignment_position).map_err(|_| {
                    StoreError::InvalidRecord("prefetch position is too large".to_string())
                })?,
            )
            .ok_or_else(|| {
                StoreError::InvalidRecord("prefetch position is outside the assignment".to_string())
            })?;
        if expected.problem != reservation.problem
            || expected.version != reservation.question_version
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
            let current = submitted.map(|record| &record.attempt).unwrap_or(attempt);
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
                if !assignment.problems.iter().any(|reference| {
                    reference.problem == attempt.problem
                        && reference.version == attempt.question_version
                }) {
                    return Err(StoreError::InvalidRecord(
                        "question attempt must reference a version in its assignment".to_string(),
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

#[async_trait]
impl ExternalToolBrokerStore for MemoryStore {
    async fn begin_or_resume_external_grade(
        &self,
        context: TenantContext,
        command: BeginExternalToolGradeCommand,
    ) -> Result<ExternalToolBegin, StoreError> {
        validate_external_command(&command.response, &command.binding, command.lease_millis)?;
        let mut state = self.write_state()?;
        let tenant = context.tenant_id();
        let attempt = state
            .attempts
            .get(&(tenant, command.attempt))
            .cloned()
            .ok_or(StoreError::NotFound)?;
        require_attempt_owner(&state, tenant, &attempt, command.actor)?;
        let published = state
            .published
            .get(&(attempt.problem, attempt.question_version))
            .ok_or(StoreError::NotFound)?;
        validate_external_binding(&attempt, &published.question.source, &command.binding)?;
        if let Some(submission) = state.submissions.get(&(tenant, command.attempt)) {
            if submission.key == command.idempotency_key && submission.response == command.response
            {
                return Ok(ExternalToolBegin::Committed(Box::new(
                    submission.record.clone(),
                )));
            }
            return Err(StoreError::Conflict);
        }
        let now = state.authoritative_time;
        if let Some(exchange) = state
            .external_tool_exchanges
            .get_mut(&(tenant, command.attempt))
        {
            validate_exchange(exchange, &command)?;
            if let Some(verified) = &exchange.verified {
                return Ok(ExternalToolBegin::VerifiedPending(verified.clone()));
            }
            if exchange.lease_expires_at.is_some_and(|expiry| expiry > now) {
                return Ok(ExternalToolBegin::InProgress);
            }
            let token = ExternalToolLeaseToken::generate()?;
            let expires_at = add_external_millis(now, command.lease_millis)?;
            exchange.lease = Some(token.clone());
            exchange.lease_expires_at = Some(expires_at);
            return Ok(ExternalToolBegin::Lease(ExternalToolLease {
                binding: exchange.binding.clone(),
                correlation: exchange.correlation.clone(),
                token,
                expires_at,
            }));
        }
        let token = ExternalToolLeaseToken::generate()?;
        let expires_at = add_external_millis(now, command.lease_millis)?;
        let exchange = StoredExternalToolExchange {
            actor: command.actor,
            binding: command.binding.clone(),
            response: command.response,
            key: command.idempotency_key,
            correlation: command.proposed_correlation.clone(),
            lease: Some(token.clone()),
            lease_expires_at: Some(expires_at),
            verified_lease_hash: None,
            verified: None,
        };
        state
            .external_tool_exchanges
            .insert((tenant, command.attempt), exchange);
        Ok(ExternalToolBegin::Lease(ExternalToolLease {
            binding: command.binding,
            correlation: command.proposed_correlation,
            token,
            expires_at,
        }))
    }

    async fn stage_external_tool_verification(
        &self,
        context: TenantContext,
        command: StageExternalToolVerificationCommand,
    ) -> Result<(), StoreError> {
        validate_external_response(&command.response, &command.binding)?;
        crate::validate_attempt_result(command.result)?;
        let mut state = self.write_state()?;
        let tenant = context.tenant_id();
        let now = state.authoritative_time;
        let attempt = state
            .attempts
            .get(&(tenant, command.attempt))
            .cloned()
            .ok_or(StoreError::NotFound)?;
        require_attempt_owner(&state, tenant, &attempt, command.actor)?;
        let published = state
            .published
            .get(&(attempt.problem, attempt.question_version))
            .ok_or(StoreError::NotFound)?;
        validate_external_binding(&attempt, &published.question.source, &command.binding)?;
        let exchange = state
            .external_tool_exchanges
            .get_mut(&(tenant, command.attempt))
            .ok_or(StoreError::NotFound)?;
        if exchange.actor != command.actor
            || exchange.binding != command.binding
            || exchange.response != command.response
            || exchange.key != command.idempotency_key
            || exchange.correlation != command.correlation
            || exchange.lease.as_ref() != Some(&command.lease_token)
            || !exchange.lease_expires_at.is_some_and(|expiry| expiry > now)
        {
            return Err(StoreError::Conflict);
        }
        let bytes = serde_json::to_vec(&command.result).map_err(|error| {
            StoreError::InvalidRecord(format!("external result encoding failed: {error}"))
        })?;
        exchange.verified = Some(ExternalToolVerifiedPending {
            binding: command.binding.clone(),
            correlation: command.correlation.clone(),
            result: command.result,
            result_sha256: Sha256Digest::compute(&bytes),
        });
        exchange.verified_lease_hash = Some(command.lease_token.hash());
        exchange.lease = None;
        exchange.lease_expires_at = None;
        Ok(())
    }

    async fn commit_external_tool_submission(
        &self,
        context: TenantContext,
        command: CommitExternalToolSubmissionCommand,
    ) -> Result<SubmissionRecord, StoreError> {
        let mut state = self.write_state()?;
        let tenant = context.tenant_id();
        let attempt = state
            .attempts
            .get(&(tenant, command.attempt))
            .cloned()
            .ok_or(StoreError::NotFound)?;
        require_attempt_owner(&state, tenant, &attempt, command.actor)?;
        validate_external_response(&command.response, &command.binding)?;
        let published = state
            .published
            .get(&(attempt.problem, attempt.question_version))
            .ok_or(StoreError::NotFound)?;
        validate_external_binding(&attempt, &published.question.source, &command.binding)?;
        if let Some(record) = state.submissions.get(&(tenant, command.attempt)) {
            return if record.key == command.idempotency_key && record.response == command.response {
                Ok(record.record.clone())
            } else {
                Err(StoreError::Conflict)
            };
        }
        validate_active_external_launch(
            &state,
            tenant,
            command.actor,
            command.attempt,
            &command.binding,
            &command.launch_proof,
        )?;
        let result = {
            let exchange = state
                .external_tool_exchanges
                .get(&(tenant, command.attempt))
                .ok_or(StoreError::NotFound)?;
            if exchange.actor != command.actor
                || exchange.binding != command.binding
                || exchange.response != command.response
                || exchange.key != command.idempotency_key
                || exchange.correlation != command.correlation
                || exchange.verified_lease_hash != Some(command.lease_token.hash())
            {
                return Err(StoreError::Conflict);
            }
            exchange
                .verified
                .as_ref()
                .ok_or(StoreError::Conflict)?
                .result
        };
        // Every fallible check in the generic transition precedes its first
        // mutation; no separate lock or visible half-submission is possible.
        let record = submit_question_attempt_locked(
            &mut state,
            context,
            SubmitQuestionAttemptCommand {
                actor: command.actor,
                attempt: command.attempt,
                response: command.response,
                result,
                feedback: question_model::FeedbackContent::default(),
                idempotency_key: command.idempotency_key,
            },
        )?;
        state
            .external_tool_exchanges
            .remove(&(tenant, command.attempt));
        revoke_external_launch(&mut state, tenant, command.launch_proof.session_id)?;
        Ok(record)
    }

    async fn commit_verified_external_tool_submission(
        &self,
        context: TenantContext,
        command: CommitVerifiedExternalToolSubmissionCommand,
    ) -> Result<SubmissionRecord, StoreError> {
        validate_external_response(&command.response, &command.binding)?;
        let mut state = self.write_state()?;
        let tenant = context.tenant_id();
        let attempt = state
            .attempts
            .get(&(tenant, command.attempt))
            .cloned()
            .ok_or(StoreError::NotFound)?;
        require_attempt_owner(&state, tenant, &attempt, command.actor)?;
        let published = state
            .published
            .get(&(attempt.problem, attempt.question_version))
            .ok_or(StoreError::NotFound)?;
        validate_external_binding(&attempt, &published.question.source, &command.binding)?;
        if let Some(record) = state.submissions.get(&(tenant, command.attempt)) {
            return if record.key == command.idempotency_key && record.response == command.response {
                Ok(record.record.clone())
            } else {
                Err(StoreError::Conflict)
            };
        }
        validate_active_external_launch(
            &state,
            tenant,
            command.actor,
            command.attempt,
            &command.binding,
            &command.launch_proof,
        )?;
        let result = {
            let exchange = state
                .external_tool_exchanges
                .get(&(tenant, command.attempt))
                .ok_or(StoreError::NotFound)?;
            if exchange.actor != command.actor
                || exchange.binding != command.binding
                || exchange.response != command.response
                || exchange.key != command.idempotency_key
                || exchange.correlation != command.correlation
            {
                return Err(StoreError::Conflict);
            }
            exchange
                .verified
                .as_ref()
                .ok_or(StoreError::Conflict)?
                .result
        };
        let record = submit_question_attempt_locked(
            &mut state,
            context,
            SubmitQuestionAttemptCommand {
                actor: command.actor,
                attempt: command.attempt,
                response: command.response,
                result,
                feedback: question_model::FeedbackContent::default(),
                idempotency_key: command.idempotency_key,
            },
        )?;
        state
            .external_tool_exchanges
            .remove(&(tenant, command.attempt));
        revoke_external_launch(&mut state, tenant, command.launch_proof.session_id)?;
        Ok(record)
    }
}

#[async_trait]
impl ExternalToolLaunchSessionStore for MemoryStore {
    async fn create_external_tool_launch_session(
        &self,
        context: TenantContext,
        command: CreateExternalToolLaunchSessionCommand,
    ) -> Result<CreatedExternalToolLaunchSession, StoreError> {
        validate_external_response(&StudentResponse::ExternalTool {}, &command.binding)?;
        if command.lifetime_millis == 0
            || command.lifetime_millis > 900_000
            || command
                .encrypted_provider_state
                .as_ref()
                .is_some_and(|bytes| bytes.len() > 65_536)
        {
            return Err(StoreError::InvalidRecord(
                "external-tool launch session is invalid".to_string(),
            ));
        }
        let mut state = self.write_state()?;
        let tenant = context.tenant_id();
        let attempt = state
            .attempts
            .get(&(tenant, command.attempt))
            .cloned()
            .ok_or(StoreError::NotFound)?;
        require_attempt_owner(&state, tenant, &attempt, command.actor)?;
        let published = state
            .published
            .get(&(attempt.problem, attempt.question_version))
            .ok_or(StoreError::NotFound)?;
        validate_external_binding(&attempt, &published.question.source, &command.binding)?;
        let token = ExternalToolLaunchToken::generate()?;
        let expires_at =
            add_external_launch_millis(state.authoritative_time, command.lifetime_millis)?;
        let id = fresh_external_tool_launch_id()?;
        state.external_tool_launch_sessions.insert(
            (tenant, id),
            StoredExternalToolLaunchSession {
                actor: command.actor,
                attempt: command.attempt,
                binding: command.binding,
                token_hash: token.hash(),
                encrypted_provider_state: command.encrypted_provider_state,
                expires_at,
                revoked: false,
            },
        );
        Ok(CreatedExternalToolLaunchSession {
            id,
            token,
            expires_at,
        })
    }
    async fn resolve_external_tool_launch_session(
        &self,
        context: TenantContext,
        actor: UserId,
        attempt: QuestionAttemptId,
        id: Uuid,
        token: &ExternalToolLaunchToken,
    ) -> Result<Option<ResolvedExternalToolLaunchSession>, StoreError> {
        let state = self.read_state()?;
        let tenant = context.tenant_id();
        let Some(session) = state.external_tool_launch_sessions.get(&(tenant, id)) else {
            return Ok(None);
        };
        let record = state
            .attempts
            .get(&(tenant, attempt))
            .ok_or(StoreError::NotFound)?;
        require_attempt_owner(&state, tenant, record, actor)?;
        if session.actor != actor || session.attempt != attempt {
            return Ok(None);
        }
        if session.revoked
            || session.expires_at <= state.authoritative_time
            || session.token_hash != token.hash()
        {
            return Ok(None);
        }
        Ok(Some(ResolvedExternalToolLaunchSession {
            binding: session.binding.clone(),
            encrypted_provider_state: session.encrypted_provider_state.clone(),
        }))
    }
    async fn revoke_external_tool_launch_session(
        &self,
        context: TenantContext,
        actor: UserId,
        attempt: QuestionAttemptId,
        id: Uuid,
    ) -> Result<(), StoreError> {
        let mut state = self.write_state()?;
        let tenant = context.tenant_id();
        let session = state
            .external_tool_launch_sessions
            .get_mut(&(tenant, id))
            .ok_or(StoreError::NotFound)?;
        if session.actor != actor || session.attempt != attempt {
            return Err(StoreError::NotFound);
        }
        session.revoked = true;
        Ok(())
    }
}

fn add_external_millis(
    now: ActivityTimestamp,
    millis: u32,
) -> Result<ActivityTimestamp, StoreError> {
    if millis == 0 || millis > 300_000 {
        return Err(StoreError::InvalidRecord(
            "external-tool lease must be 1 to 300000 milliseconds".to_string(),
        ));
    }
    now.as_unix_millis()
        .checked_add(i64::from(millis))
        .map(ActivityTimestamp::from_unix_millis)
        .ok_or_else(|| {
            StoreError::InvalidRecord("external-tool lease timestamp overflow".to_string())
        })
}

fn add_external_launch_millis(
    now: ActivityTimestamp,
    millis: u32,
) -> Result<ActivityTimestamp, StoreError> {
    if millis == 0 || millis > 900_000 {
        return Err(StoreError::InvalidRecord(
            "external-tool launch session is invalid".to_string(),
        ));
    }
    now.as_unix_millis()
        .checked_add(i64::from(millis))
        .map(ActivityTimestamp::from_unix_millis)
        .ok_or_else(|| {
            StoreError::InvalidRecord("external-tool launch timestamp overflow".to_string())
        })
}

fn validate_external_command(
    response: &StudentResponse,
    binding: &ExternalToolBinding,
    lease_millis: u32,
) -> Result<(), StoreError> {
    validate_external_response(response, binding)?;
    let _ = add_external_millis(ActivityTimestamp::from_unix_millis(0), lease_millis)?;
    Ok(())
}

fn validate_external_binding(
    attempt: &QuestionAttempt,
    source: &question_model::QuestionSource,
    binding: &ExternalToolBinding,
) -> Result<(), StoreError> {
    if attempt.problem != binding.problem
        || attempt.question_version != binding.version
        || attempt.seed != binding.seed
    {
        return Err(StoreError::Conflict);
    }
    let provenance_source = attempt
        .provenance
        .source_artifact
        .as_ref()
        .ok_or(StoreError::Conflict)?;
    if provenance_source.object != binding.source_object
        || provenance_source.sha256 != binding.source_sha256
    {
        return Err(StoreError::Conflict);
    }
    let question_model::QuestionSource::Imathas {
        provider,
        snapshot,
        snapshot_sha256,
        integration_profile,
        ..
    } = source
    else {
        return Err(StoreError::Conflict);
    };
    if provider != &binding.provider
        || snapshot != &binding.source_object
        || snapshot_sha256 != &binding.source_sha256
        || integration_profile != &binding.integration_profile
    {
        return Err(StoreError::Conflict);
    }
    Ok(())
}

fn validate_external_response(
    response: &StudentResponse,
    binding: &ExternalToolBinding,
) -> Result<(), StoreError> {
    if !matches!(response, StudentResponse::ExternalTool {}) {
        return Err(StoreError::InvalidRecord(
            "external-tool exchange requires the external marker response".to_string(),
        ));
    }
    binding.validate()?;
    let canonical = serde_json::to_vec(response).map_err(|error| {
        StoreError::InvalidRecord(format!("external response encoding failed: {error}"))
    })?;
    if Sha256Digest::compute(&canonical) != binding.response_sha256 {
        return Err(StoreError::Conflict);
    }
    Ok(())
}

fn validate_active_external_launch(
    state: &State,
    tenant: TenantId,
    actor: UserId,
    attempt: QuestionAttemptId,
    binding: &ExternalToolBinding,
    proof: &ExternalToolLaunchProof,
) -> Result<(), StoreError> {
    let session = state
        .external_tool_launch_sessions
        .get(&(tenant, proof.session_id))
        .ok_or(StoreError::Conflict)?;
    if session.actor != actor
        || session.attempt != attempt
        || session.binding != *binding
        || session.revoked
        || session.expires_at <= state.authoritative_time
        || session.token_hash != proof.token.hash()
    {
        return Err(StoreError::Conflict);
    }
    Ok(())
}

fn revoke_external_launch(
    state: &mut State,
    tenant: TenantId,
    session_id: Uuid,
) -> Result<(), StoreError> {
    let session = state
        .external_tool_launch_sessions
        .get_mut(&(tenant, session_id))
        .ok_or(StoreError::Conflict)?;
    if session.revoked {
        return Err(StoreError::Conflict);
    }
    session.revoked = true;
    Ok(())
}

fn validate_exchange(
    exchange: &StoredExternalToolExchange,
    command: &BeginExternalToolGradeCommand,
) -> Result<(), StoreError> {
    if exchange.actor != command.actor
        || exchange.binding != command.binding
        || exchange.response != command.response
        || exchange.key != command.idempotency_key
    {
        return Err(StoreError::Conflict);
    }
    Ok(())
}

/// Ordinary submission transition while the caller already holds the one
/// MemoryStore write lock. Keeping this separate prevents an external-tool
/// verification from becoming a check-then-submit race in the test backend.
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
    let question = state
        .published
        .get(&(base.problem, base.question_version))
        .ok_or(StoreError::NotFound)?;
    crate::validate_attempt_result(command.result)?;
    let submitted_at = state.authoritative_time;
    let mut submitted = base;
    submitted.response = Some(command.response.clone());
    submitted.result = Some(command.result);
    submitted.timer.submitted_at = Some(submitted_at);
    let verdict = timer_verdict(&TimerEvaluation {
        policy: question.question.timing_policy,
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
    let results = current_results_by_position(state, tenant, &run, &assignment, &submitted);
    let mut statistics_contributions = None;
    if let Some(score) = completed_run_score(&results, assignment.policies.completion)? {
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
            statistics_contributions = Some(derive_statistics_contributions(
                &assignment,
                &results,
                &attempts,
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
    state.runs.insert((tenant, run.id), run);
    state
        .enrollments
        .insert((tenant, enrollment.id), enrollment);
    state.summaries.insert((tenant, next.enrollment), next);
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
    assignment: &AssignmentRecord,
    command: &IssueQuestionAttemptCommand,
) -> Result<(), StoreError> {
    let position = usize::try_from(command.assignment_position)
        .map_err(|_| StoreError::InvalidRecord("assignment position is too large".to_string()))?;
    let expected = assignment.problems.get(position).ok_or_else(|| {
        StoreError::InvalidRecord("question position is outside the assignment".to_string())
    })?;
    if expected.problem != command.problem || expected.version != command.question_version {
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
    for reference in &assignment.problems {
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

fn projected_attempt(
    state: &State,
    tenant: TenantId,
    attempt: &QuestionAttempt,
) -> QuestionAttempt {
    state
        .submissions
        .get(&(tenant, attempt.id))
        .map_or_else(|| attempt.clone(), |stored| stored.record.attempt.clone())
}

fn current_results_by_position(
    state: &State,
    tenant: TenantId,
    run: &AssignmentRun,
    assignment: &AssignmentRecord,
    current: &QuestionAttempt,
) -> Vec<Option<AttemptResult>> {
    let mut latest: Vec<Option<(ActivityTimestamp, QuestionAttemptId, AttemptResult)>> =
        vec![None; assignment.problems.len()];
    for base in state
        .attempts
        .values()
        .filter(|attempt| attempt.tenant == tenant && attempt.run == run.id)
    {
        let projected = if base.id == current.id {
            current.clone()
        } else {
            projected_attempt(state, tenant, base)
        };
        let (Some(submitted_at), Some(result)) = (projected.timer.submitted_at, projected.result)
        else {
            continue;
        };
        let Ok(position) = usize::try_from(projected.assignment_position) else {
            continue;
        };
        let Some(slot) = latest.get_mut(position) else {
            continue;
        };
        if slot
            .as_ref()
            .is_none_or(|(at, id, _)| (submitted_at, projected.id) > (*at, *id))
        {
            *slot = Some((submitted_at, projected.id, result));
        }
    }
    latest
        .into_iter()
        .map(|entry| entry.map(|(_, _, result)| result))
        .collect()
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
        let mut state = self.write_state()?;
        let now = state.authoritative_time;
        let job = crate::JobId::generate()?;
        let export_job = crate::JobId::generate()?;
        let export = crate::ExportId::generate()?;
        let snapshot = CourseRetentionSnapshot::new(
            now,
            InstitutionRetentionPolicy::default(),
            AssignmentDefinitionDisposition::Retain,
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
                    AssignmentDefinitionDisposition::Retain,
                ),
            },
        );
        state.retention_stages.insert(
            (
                tenant,
                course,
                crate::RetentionStage::ArchiveStudentRecords,
                1,
            ),
            StoredRetentionStage {
                due_at: now,
                state: RetentionStageWorkState::Scheduled,
                job: None,
                lease: None,
            },
        );
        state.retention_dispatches.insert(
            (
                tenant,
                course,
                crate::RetentionStage::ArchiveStudentRecords,
                1,
            ),
            job,
        );
        state.jobs.insert(
            job,
            StoredJob {
                tenant,
                payload: crate::JobPayload::Retention {
                    course,
                    stage: crate::RetentionStage::ArchiveStudentRecords,
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

#[cfg(test)]
mod catalog_search_tests {
    use super::*;
    use question_model::answer::NumericTolerance;
    use question_model::generation::RandomizationDefinition;
    use question_model::run_policy::{AttemptPolicy, FeedbackDisclosure};
    use question_model::taxonomy::{License, Tag};
    use question_model::{
        AssignmentRun, AttemptProvenance, AttemptTimerRecord, BackendCapabilities, Capability,
        DraftQuestionDefinition, DraftQuestionSource, GradingDefinition, ImplementationVersion,
        QuestionDefinition, QuestionMetadata, ResponseDefinition, StudentId,
    };

    fn record(number: u128) -> PublishedProblemRecord {
        let problem = ProblemId::from_uuid(Uuid::from_u128(number));
        let version = VersionId::from_uuid(Uuid::from_u128(20_000 + number));
        let question = QuestionDefinition::from_draft(
            DraftQuestionDefinition {
                workspace: WorkspaceId::from_uuid(Uuid::from_u128(30_000 + number)),
                source: DraftQuestionSource::Native {
                    family: "catalog_fixture".to_string(),
                },
                prompt: Vec::new(),
                response: ResponseDefinition::Numeric {
                    tolerance: NumericTolerance::Absolute { epsilon: 0.1 },
                    unit: None,
                },
                attempt_policy: AttemptPolicy {
                    max_attempts: None,
                    feedback: FeedbackDisclosure::ImmediateCorrectness,
                },
                timing_policy: TimingPolicy::Untimed,
                randomization: RandomizationDefinition::Static,
                grading: GradingDefinition::AllOrNothing { points: 1.0 },
                metadata: QuestionMetadata {
                    title: format!("Peptide catalog item {number}"),
                    tags: vec![Tag::new("peptide")],
                    taxonomy: vec![TaxonomyTerm {
                        scheme: "discipline".to_string(),
                        code: "biochemistry".to_string(),
                        label: "Biochemistry".to_string(),
                    }],
                    license: License::CcBy,
                    language: "en".to_string(),
                },
            },
            problem,
            version,
            question_model::QuestionSource::Native {
                family: "catalog_fixture".to_string(),
            },
        );
        PublishedProblemRecord {
            problem,
            version,
            question,
            capabilities: BackendCapabilities::from_iter([Capability::ServerGrading]),
            scope: PublicationScope::Public,
            lifecycle: CatalogLifecycle::Published,
            authors: vec![UserId::from_uuid(Uuid::from_u128(40_000))],
            previous_version: None,
            derived_from: None,
            published_at: ActivityTimestamp::from_unix_millis(0),
        }
    }

    fn statistics_attempt(
        number: u128,
        tenant: TenantId,
        run: RunId,
        reference: ProblemVersionRef,
        position: u32,
        issued_at: i64,
    ) -> QuestionAttempt {
        QuestionAttempt {
            id: QuestionAttemptId::from_uuid(Uuid::from_u128(number)),
            tenant,
            run,
            problem: reference.problem,
            question_version: reference.version,
            assignment_position: position,
            seed: number as u64,
            parameter_hash: format!("statistics-parameters-{number}"),
            response: None,
            result: None,
            timer: AttemptTimerRecord {
                issued_at: ActivityTimestamp::from_unix_millis(issued_at),
                deadline: None,
                submitted_at: None,
            },
            provenance: AttemptProvenance {
                adapter: ImplementationVersion {
                    id: "native".to_string(),
                    version: "1".to_string(),
                },
                renderer: None,
                generator: None,
                source_artifact: None,
                asset_objects: Vec::new(),
                grading: ImplementationVersion {
                    id: "numeric".to_string(),
                    version: "1".to_string(),
                },
                rendered_question_sha256: format!("statistics-render-{number}"),
            },
        }
    }

    fn submit_statistics_attempt(
        store: &MemoryStore,
        context: TenantContext,
        actor: UserId,
        attempt: QuestionAttempt,
        submitted_at: i64,
        earned: f64,
        possible: f64,
    ) -> (SubmissionRecord, SubmitQuestionAttemptCommand) {
        let command = SubmitQuestionAttemptCommand {
            actor,
            attempt: attempt.id,
            response: StudentResponse::Numeric { value: earned },
            result: AttemptResult {
                correct: earned == possible,
                points_earned: earned,
                points_possible: possible,
            },
            feedback: question_model::FeedbackContent::default(),
            idempotency_key: SubmissionIdempotencyKey::parse(format!(
                "statistics-submission-{}",
                attempt.id
            ))
            .expect("valid fixture idempotency key"),
        };
        let mut state = store.write_state().expect("statistics fixture state");
        state.authoritative_time = ActivityTimestamp::from_unix_millis(submitted_at);
        state.attempts.insert((attempt.tenant, attempt.id), attempt);
        let record = submit_question_attempt_locked(&mut state, context, command.clone())
            .expect("statistics fixture submission");
        (record, command)
    }

    #[test]
    fn first_assigned_completion_records_collapsed_statistics_once() {
        let store = MemoryStore::default();
        let tenant = TenantId::from_uuid(Uuid::from_u128(72_001));
        let context = TenantContext::from_authenticated_session(tenant);
        let actor = UserId::from_uuid(Uuid::from_u128(72_002));
        let assignment_id = AssignmentId::from_uuid(Uuid::from_u128(72_003));
        let enrollment_id = EnrollmentId::from_uuid(Uuid::from_u128(72_004));
        let assigned_run = RunId::from_uuid(Uuid::from_u128(72_005));
        let mut published_a = record(72_010);
        let mut published_b = record(72_011);
        published_a.scope = PublicationScope::Public;
        published_b.scope = PublicationScope::Public;
        let a = ProblemVersionRef {
            problem: published_a.problem,
            version: published_a.version,
        };
        let b = ProblemVersionRef {
            problem: published_b.problem,
            version: published_b.version,
        };
        let assignment = AssignmentRecord {
            id: assignment_id,
            tenant,
            course_id: CourseId::from_uuid(Uuid::from_u128(72_006)),
            title: "Statistics completion fixture".to_string(),
            problems: vec![a, b, a],
            policies: question_model::RunPolicies {
                completion: question_model::CompletionRequirement::AnswerAll,
                grade: question_model::GradePolicy::First,
                continued_practice: question_model::ContinuedPractice::Unlimited,
                variation: question_model::VariationPolicy::NewSeeds,
            },
        };
        let enrollment = AssignmentEnrollment {
            id: enrollment_id,
            tenant,
            assignment: assignment_id,
            user: actor,
            student: StudentId::from_uuid(Uuid::from_u128(72_007)),
            first_completed_at: None,
            current_grade_run: None,
            best_grade_run: None,
        };
        let run = AssignmentRun {
            id: assigned_run,
            tenant,
            enrollment: enrollment_id,
            run_number: 1,
            started_at: ActivityTimestamp::from_unix_millis(0),
            completed_at: None,
            score: None,
            mode: RunMode::Assigned,
            variation: question_model::VariationPolicy::NewSeeds,
        };
        {
            let mut state = store.write_state().expect("statistics fixture state");
            state.courses.insert(
                (tenant, assignment.course_id),
                CourseRecord {
                    id: assignment.course_id,
                    tenant,
                    title: "Statistics fixture course".to_string(),
                    members: Vec::new(),
                },
            );
            state
                .published
                .insert((published_a.problem, published_a.version), published_a);
            state
                .published
                .insert((published_b.problem, published_b.version), published_b);
            state
                .assignments
                .insert((tenant, assignment_id), assignment);
            state
                .enrollments
                .insert((tenant, enrollment_id), enrollment);
            state.runs.insert((tenant, assigned_run), run);
            state.summaries.insert(
                (tenant, enrollment_id),
                StudentAssignmentSummary::empty(tenant, enrollment_id),
            );
        }

        let regressive = statistics_attempt(72_099, tenant, assigned_run, a, 0, 2_000);
        let regressive_command = SubmitQuestionAttemptCommand {
            actor,
            attempt: regressive.id,
            response: StudentResponse::Numeric { value: 0.0 },
            result: AttemptResult {
                correct: false,
                points_earned: 0.0,
                points_possible: 2.0,
            },
            feedback: question_model::FeedbackContent::default(),
            idempotency_key: SubmissionIdempotencyKey::parse("statistics-regressive-time")
                .expect("valid fixture idempotency key"),
        };
        {
            let mut state = store.write_state().expect("regressive statistics state");
            state.authoritative_time = ActivityTimestamp::from_unix_millis(1_500);
            state
                .attempts
                .insert((tenant, regressive.id), regressive.clone());
            assert!(matches!(
                submit_question_attempt_locked(&mut state, context, regressive_command),
                Err(StoreError::InvalidRecord(_))
            ));
            assert!(!state.submissions.contains_key(&(tenant, regressive.id)));
            assert_eq!(
                state.summaries[&(tenant, enrollment_id)],
                StudentAssignmentSummary::empty(tenant, enrollment_id)
            );
            assert_eq!(state.runs[&(tenant, assigned_run)].completed_at, None);
            assert!(state.question_statistics.is_empty());
            assert!(state.question_statistics_receipts.is_empty());
        }

        submit_statistics_attempt(
            &store,
            context,
            actor,
            statistics_attempt(72_100, tenant, assigned_run, a, 0, 0),
            1_500,
            0.0,
            2.0,
        );
        submit_statistics_attempt(
            &store,
            context,
            actor,
            statistics_attempt(72_101, tenant, assigned_run, a, 0, 2_000),
            4_500,
            1.0,
            2.0,
        );
        submit_statistics_attempt(
            &store,
            context,
            actor,
            statistics_attempt(72_102, tenant, assigned_run, b, 1, 5_000),
            6_000,
            1.0,
            4.0,
        );
        let (_, final_command) = submit_statistics_attempt(
            &store,
            context,
            actor,
            statistics_attempt(72_103, tenant, assigned_run, a, 2, 7_000),
            100_007_000,
            2.0,
            2.0,
        );

        let completed_statistics = {
            let state = store.read_state().expect("completed statistics state");
            assert_eq!(state.question_statistics_receipts.len(), 2);
            let a_snapshot = state.question_statistics[&(a.problem, a.version)].snapshot();
            assert_eq!(a_snapshot.cohort_size, 1);
            assert_eq!(a_snapshot.score_sum, 0.75);
            assert_eq!(a_snapshot.attempts_sum, 3);
            assert_eq!(a_snapshot.durations.bins[9], 1);
            assert_eq!(a_snapshot.discrimination.count, 1);
            assert_eq!(a_snapshot.discrimination.mean_x, 0.75);
            assert_eq!(a_snapshot.discrimination.mean_y, 0.25);
            let b_snapshot = state.question_statistics[&(b.problem, b.version)].snapshot();
            assert_eq!(b_snapshot.cohort_size, 1);
            assert_eq!(b_snapshot.score_sum, 0.25);
            assert_eq!(b_snapshot.attempts_sum, 1);
            assert_eq!(b_snapshot.durations.bins[0], 1);
            assert_eq!(b_snapshot.discrimination.mean_x, 0.25);
            assert_eq!(b_snapshot.discrimination.mean_y, 0.75);
            (
                state.question_statistics.clone(),
                state.question_statistics_receipts.clone(),
            )
        };

        {
            let mut state = store.write_state().expect("replay statistics state");
            let replay = submit_question_attempt_locked(&mut state, context, final_command)
                .expect("exact completed submission replay");
            assert_eq!(replay.run.id, assigned_run);
            assert_eq!(state.question_statistics, completed_statistics.0);
            assert_eq!(state.question_statistics_receipts.len(), 2);
        }

        let practice_run = RunId::from_uuid(Uuid::from_u128(72_200));
        {
            let mut state = store.write_state().expect("practice statistics state");
            state.runs.insert(
                (tenant, practice_run),
                AssignmentRun {
                    id: practice_run,
                    tenant,
                    enrollment: enrollment_id,
                    run_number: 2,
                    started_at: ActivityTimestamp::from_unix_millis(200_000_000),
                    completed_at: None,
                    score: None,
                    mode: RunMode::Practice,
                    variation: question_model::VariationPolicy::NewSeeds,
                },
            );
        }
        for (number, reference, position, earned, possible) in [
            (72_201, a, 0, 1.0, 2.0),
            (72_202, b, 1, 1.0, 4.0),
            (72_203, a, 2, 2.0, 2.0),
        ] {
            submit_statistics_attempt(
                &store,
                context,
                actor,
                statistics_attempt(
                    number,
                    tenant,
                    practice_run,
                    reference,
                    position,
                    200_000_000,
                ),
                200_001_000 + i64::from(position),
                earned,
                possible,
            );
        }
        let state = store.read_state().expect("practice completion state");
        assert_eq!(state.question_statistics, completed_statistics.0);
        assert_eq!(
            state.question_statistics_receipts.len(),
            completed_statistics.1.len()
        );
    }

    #[tokio::test]
    async fn statistics_receipts_are_exactly_once_and_disclose_only_at_k_five() {
        let store = MemoryStore::default();
        let mut record = record(71_000);
        record.scope = PublicationScope::Institution;
        let reference = ProblemVersionRef {
            problem: record.problem,
            version: record.version,
        };
        let tenant = TenantId::from_uuid(Uuid::from_u128(71_001));
        let context = TenantContext::from_authenticated_session(tenant);
        store
            .write_state()
            .expect("test state")
            .published
            .insert((reference.problem, reference.version), record);
        store
            .write_state()
            .expect("test state")
            .catalog_grants
            .insert((tenant, reference.problem, reference.version));

        let first = CollapsedQuestionObservation::new(0.5, 2, 30, Some(0.4))
            .expect("valid collapsed observation");
        assert!(
            store
                .record_question_statistics_contribution(
                    tenant,
                    EnrollmentId::from_uuid(Uuid::from_u128(71_010)),
                    RunId::from_uuid(Uuid::from_u128(71_020)),
                    QuestionAttemptId::from_uuid(Uuid::from_u128(71_030)),
                    reference,
                    first,
                )
                .expect("first receipt records")
        );
        assert!(
            !store
                .record_question_statistics_contribution(
                    tenant,
                    EnrollmentId::from_uuid(Uuid::from_u128(71_010)),
                    RunId::from_uuid(Uuid::from_u128(71_020)),
                    QuestionAttemptId::from_uuid(Uuid::from_u128(71_030)),
                    reference,
                    first,
                )
                .expect("exact replay is harmless")
        );
        let before_conflict = store.read_state().expect("test state").question_statistics
            [&(reference.problem, reference.version)]
            .snapshot();
        assert_eq!(
            store.record_question_statistics_contribution(
                tenant,
                EnrollmentId::from_uuid(Uuid::from_u128(71_010)),
                RunId::from_uuid(Uuid::from_u128(71_020)),
                QuestionAttemptId::from_uuid(Uuid::from_u128(71_030)),
                reference,
                CollapsedQuestionObservation::new(0.6, 2, 30, Some(0.4))
                    .expect("valid conflicting observation"),
            ),
            Err(StoreError::Conflict)
        );
        assert_eq!(
            store.read_state().expect("test state").question_statistics
                [&(reference.problem, reference.version)]
                .snapshot(),
            before_conflict
        );
        for number in 1..4_u128 {
            assert!(
                store
                    .record_question_statistics_contribution(
                        tenant,
                        EnrollmentId::from_uuid(Uuid::from_u128(71_010 + number)),
                        RunId::from_uuid(Uuid::from_u128(71_020 + number)),
                        QuestionAttemptId::from_uuid(Uuid::from_u128(71_030 + number)),
                        reference,
                        CollapsedQuestionObservation::new(0.5, 1, 30, Some(0.4))
                            .expect("valid contribution"),
                    )
                    .expect("distinct receipt records")
            );
        }
        assert_eq!(
            store
                .question_statistics(context, reference)
                .await
                .expect("safe statistics read at four"),
            QuestionStatisticsDisclosure::Suppressed
        );
        assert!(
            store
                .record_question_statistics_contribution(
                    tenant,
                    EnrollmentId::from_uuid(Uuid::from_u128(71_014)),
                    RunId::from_uuid(Uuid::from_u128(71_024)),
                    QuestionAttemptId::from_uuid(Uuid::from_u128(71_034)),
                    reference,
                    CollapsedQuestionObservation::new(0.5, 1, 30, Some(0.4))
                        .expect("valid fifth contribution"),
                )
                .expect("fifth receipt records")
        );
        let disclosure = store
            .question_statistics(context, reference)
            .await
            .expect("safe statistics read");
        assert!(matches!(
            disclosure,
            QuestionStatisticsDisclosure::Available(view) if view.cohort_size == 5
        ));
        {
            let state = store.read_state().expect("test state");
            assert_eq!(state.question_statistics_receipts.len(), 5);
            assert_eq!(
                state.question_statistics[&(reference.problem, reference.version)].cohort_size(),
                5
            );
        }
        let second_reference = ProblemVersionRef {
            problem: ProblemId::from_uuid(Uuid::from_u128(71_100)),
            version: VersionId::from_uuid(Uuid::from_u128(71_101)),
        };
        assert!(
            store
                .record_question_statistics_contribution(
                    tenant,
                    EnrollmentId::from_uuid(Uuid::from_u128(71_100)),
                    RunId::from_uuid(Uuid::from_u128(71_020)),
                    QuestionAttemptId::from_uuid(Uuid::from_u128(71_030)),
                    second_reference,
                    first,
                )
                .expect("one completion trigger can contribute another version")
        );
        let foreign_context =
            TenantContext::from_authenticated_session(TenantId::from_uuid(Uuid::from_u128(71_999)));
        assert_eq!(
            store
                .question_statistics(foreign_context, reference)
                .await
                .expect("foreign safe statistics read"),
            QuestionStatisticsDisclosure::Suppressed
        );
    }

    #[tokio::test]
    async fn catalog_statistics_filter_facets_and_detail_use_only_k_gated_aggregates() {
        let store = MemoryStore::default();
        let tenant = TenantId::from_uuid(Uuid::from_u128(73_001));
        let context = TenantContext::from_authenticated_session(tenant);
        let published = record(73_002);
        let reference = ProblemVersionRef {
            problem: published.problem,
            version: published.version,
        };
        store
            .write_state()
            .expect("catalog statistics state")
            .published
            .insert((reference.problem, reference.version), published);

        for number in 0..4_u128 {
            store
                .record_question_statistics_contribution(
                    tenant,
                    EnrollmentId::from_uuid(Uuid::from_u128(73_100 + number)),
                    RunId::from_uuid(Uuid::from_u128(73_200 + number)),
                    QuestionAttemptId::from_uuid(Uuid::from_u128(73_300 + number)),
                    reference,
                    CollapsedQuestionObservation::new(0.5, 1, 30, Some(0.5))
                        .expect("valid observation"),
                )
                .expect("statistics receipt");
        }
        let below_k = store
            .search_catalog(context, CatalogSearchQuery::default())
            .await
            .expect("below-k catalog search");
        assert_eq!(below_k.facets.statistics.available, 0);
        assert_eq!(below_k.facets.statistics.unavailable, 1);
        assert!(
            store
                .search_catalog(
                    context,
                    CatalogSearchQuery {
                        statistics: CatalogStatisticsAvailability::Available,
                        ..CatalogSearchQuery::default()
                    },
                )
                .await
                .expect("below-k available filter")
                .items
                .is_empty()
        );
        assert!(matches!(
            store
                .get_catalog_detail(context, reference)
                .await
                .expect("below-k detail")
                .expect("visible detail")
                .statistics,
            question_model::CatalogStatisticsStatus::Unavailable
        ));

        store
            .record_question_statistics_contribution(
                tenant,
                EnrollmentId::from_uuid(Uuid::from_u128(73_104)),
                RunId::from_uuid(Uuid::from_u128(73_204)),
                QuestionAttemptId::from_uuid(Uuid::from_u128(73_304)),
                reference,
                CollapsedQuestionObservation::new(0.5, 1, 30, Some(0.5))
                    .expect("valid fifth observation"),
            )
            .expect("fifth statistics receipt");
        let at_k = store
            .search_catalog(context, CatalogSearchQuery::default())
            .await
            .expect("at-k catalog search");
        assert_eq!(at_k.facets.statistics.available, 1);
        assert_eq!(at_k.facets.statistics.unavailable, 0);
        assert_eq!(
            store
                .search_catalog(
                    context,
                    CatalogSearchQuery {
                        statistics: CatalogStatisticsAvailability::Available,
                        ..CatalogSearchQuery::default()
                    },
                )
                .await
                .expect("at-k available filter")
                .items
                .len(),
            1
        );
        assert!(matches!(
            store
                .get_catalog_detail(context, reference)
                .await
                .expect("at-k detail")
                .expect("visible detail")
                .statistics,
            question_model::CatalogStatisticsStatus::Available(view) if view.cohort_size == 5
        ));
    }

    #[tokio::test]
    async fn ten_thousand_catalog_rows_return_one_bounded_page_with_server_facets() {
        let store = MemoryStore::default();
        {
            let mut state = store.write_state().expect("test state");
            for number in 1..=10_000 {
                let record = record(number);
                state
                    .published
                    .insert((record.problem, record.version), record);
            }
            let mut institution_only = record(10_001);
            institution_only.scope = PublicationScope::Institution;
            state.catalog_grants.insert((
                TenantId::from_uuid(Uuid::from_u128(50_001)),
                institution_only.problem,
                institution_only.version,
            ));
            state.published.insert(
                (institution_only.problem, institution_only.version),
                institution_only,
            );
            for number in 0..65_u128 {
                let mut distinct = record(11_000 + number);
                distinct.question.metadata.taxonomy = vec![TaxonomyTerm {
                    scheme: "extra".to_string(),
                    code: format!("{number:02}"),
                    label: if number == 0 { "Zulu" } else { "Term" }.to_string(),
                }];
                state
                    .published
                    .insert((distinct.problem, distinct.version), distinct);
            }
            let mut duplicate_label = record(12_000);
            duplicate_label.question.metadata.taxonomy = vec![TaxonomyTerm {
                scheme: "extra".to_string(),
                code: "00".to_string(),
                label: "Alpha".to_string(),
            }];
            state.published.insert(
                (duplicate_label.problem, duplicate_label.version),
                duplicate_label,
            );
        }
        let context =
            TenantContext::from_authenticated_session(TenantId::from_uuid(Uuid::from_u128(50_000)));
        let first = store
            .search_catalog(
                context,
                CatalogSearchQuery {
                    text: Some(" peptide   catalog ".to_string()),
                    page_size: Some(37),
                    ..CatalogSearchQuery::default()
                },
            )
            .await
            .expect("bounded search");
        assert_eq!(first.items.len(), 37);
        assert_eq!(first.facets.statistics.available, 0);
        assert_eq!(first.facets.statistics.unavailable, 10_066);
        assert_eq!(first.facets.taxonomy[0].count, 10_000);
        assert_eq!(first.facets.taxonomy.len(), MAX_CATALOG_TAXONOMY_FACETS);
        assert_eq!(first.facets.taxonomy[1].term.code, "00");
        assert_eq!(first.facets.taxonomy[1].term.label, "Alpha");
        assert_eq!(first.facets.taxonomy[1].count, 2);
        assert_eq!(
            first.facets.taxonomy[1..]
                .iter()
                .map(|facet| facet.term.code.clone())
                .collect::<Vec<_>>(),
            (0..=62)
                .map(|number| format!("{number:02}"))
                .collect::<Vec<_>>(),
        );
        assert!(
            first
                .facets
                .taxonomy
                .iter()
                .all(|facet| facet.term.code != "63" && facet.term.code != "64")
        );
        assert_eq!(first.facets.capabilities[0].count, 10_066);
        assert_eq!(first.facets.licenses[0].count, 10_066);
        assert!(
            first
                .items
                .iter()
                .all(|item| item.scope == PublicationScope::Public)
        );
        let cursor = first.next_cursor.clone().expect("next cursor");
        assert!(cursor.len() <= 200);
        assert!(
            cursor.chars().all(
                |character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_')
            )
        );
        let mut tampered = cursor.clone().into_bytes();
        let last = tampered.len() - 1;
        tampered[last] = if tampered[last] == b'A' { b'B' } else { b'A' };
        assert!(matches!(
            store
                .search_catalog(
                    context,
                    CatalogSearchQuery {
                        text: Some("peptide catalog".to_string()),
                        cursor: Some(String::from_utf8(tampered).expect("url-safe cursor")),
                        ..CatalogSearchQuery::default()
                    },
                )
                .await,
            Err(StoreError::InvalidRecord(_))
        ));
        let second = store
            .search_catalog(
                context,
                CatalogSearchQuery {
                    text: Some("peptide catalog".to_string()),
                    cursor: Some(cursor),
                    page_size: Some(37),
                    ..CatalogSearchQuery::default()
                },
            )
            .await
            .expect("second bounded search");
        assert_eq!(second.items.len(), 37);
        assert!(
            first
                .items
                .iter()
                .all(|left| second
                    .items
                    .iter()
                    .all(|right| (left.problem, left.version) != (right.problem, right.version)))
        );
        assert!(matches!(
            store
                .search_catalog(
                    context,
                    CatalogSearchQuery {
                        text: Some("different query".to_string()),
                        cursor: first.next_cursor,
                        ..CatalogSearchQuery::default()
                    },
                )
                .await,
            Err(StoreError::InvalidRecord(_))
        ));
    }
}

#[cfg(test)]
mod retention_tests {
    use super::*;
    use crate::{
        JobLeaseDuration, JobPayload, JobStore, RetentionApiStore, RetentionDays,
        RetentionDispatchBatch, RetentionScheduleStore, RetentionWorkerCommand,
        RetentionWorkerStore,
    };
    use question_model::{CourseMembership, CourseMembershipRole};

    fn session(number: u8) -> SessionTokenHash {
        SessionTokenHash::compute(&[number; 32])
    }

    async fn establish_session(
        store: &MemoryStore,
        token: SessionTokenHash,
        tenant: TenantId,
        user: UserId,
        roles: Vec<UserRole>,
    ) {
        store
            .create_session(
                token,
                SessionSubject::new(tenant, user, "Retention fixture", roles)
                    .expect("valid session subject"),
                SessionLifetime::from_seconds(3_600).expect("valid lifetime"),
            )
            .await
            .expect("store session");
    }

    #[tokio::test]
    async fn retention_policy_and_course_end_are_session_authorized_and_idempotent() {
        let store = MemoryStore::default();
        let tenant = TenantId::from_uuid(Uuid::from_u128(81_001));
        let context = TenantContext::from_authenticated_session(tenant);
        let instructor = UserId::from_uuid(Uuid::from_u128(81_002));
        let student = UserId::from_uuid(Uuid::from_u128(81_003));
        let administrator = UserId::from_uuid(Uuid::from_u128(81_004));
        let course = CourseId::from_uuid(Uuid::from_u128(81_005));
        {
            let mut state = store.write_state().expect("retention state");
            state.authoritative_time = ActivityTimestamp::from_unix_millis(1_000_000);
            state.courses.insert(
                (tenant, course),
                CourseRecord {
                    id: course,
                    tenant,
                    title: "Retention course".to_string(),
                    members: vec![
                        CourseMembership {
                            user: instructor,
                            role: CourseMembershipRole::Instructor,
                        },
                        CourseMembership {
                            user: student,
                            role: CourseMembershipRole::Student,
                        },
                    ],
                },
            );
        }
        establish_session(
            &store,
            session(1),
            tenant,
            instructor,
            vec![UserRole::Instructor],
        )
        .await;
        establish_session(&store, session(2), tenant, student, vec![UserRole::Student]).await;
        establish_session(
            &store,
            session(3),
            tenant,
            administrator,
            vec![UserRole::Administrator],
        )
        .await;

        assert_eq!(
            store
                .configure_retention_policy(
                    context,
                    session(1),
                    InstitutionRetentionPolicy::default()
                )
                .await,
            Err(StoreError::Forbidden)
        );
        let custom = InstitutionRetentionPolicy::new(
            RetentionDays::new(31).unwrap(),
            RetentionDays::new(101).unwrap(),
            RetentionDays::new(366).unwrap(),
        )
        .unwrap();
        store
            .configure_retention_policy(context, session(3), custom)
            .await
            .expect("admin policy");
        let first = store
            .end_course_retention(context, session(1), course)
            .await
            .expect("instructor ends course");
        assert_eq!(
            first.snapshot.ended_at(),
            ActivityTimestamp::from_unix_millis(1_000_000)
        );
        assert_eq!(first.snapshot.policy(), custom);
        assert_eq!(first.snapshot.generation(), 1);
        assert_eq!(first.status.state, CourseRetentionState::Active);
        assert_eq!(
            store
                .end_course_retention(context, session(1), course)
                .await
                .expect("exact replay"),
            first
        );
        assert_eq!(
            store.course_retention(context, session(2), course).await,
            Ok(None)
        );
        assert_eq!(
            store
                .course_retention(context, session(3), course)
                .await
                .expect("admin view"),
            Some(first)
        );
    }

    #[tokio::test]
    async fn scheduler_dispatches_each_due_current_stage_once_and_binds_worker_execution() {
        let store = MemoryStore::default();
        let tenant = TenantId::from_uuid(Uuid::from_u128(81_100));
        let context = TenantContext::from_authenticated_session(tenant);
        let instructor = UserId::from_uuid(Uuid::from_u128(81_101));
        let administrator = UserId::from_uuid(Uuid::from_u128(81_102));
        let course = CourseId::from_uuid(Uuid::from_u128(81_103));
        {
            let mut state = store.write_state().expect("state");
            state.authoritative_time = ActivityTimestamp::from_unix_millis(1_000_000);
            state.courses.insert(
                (tenant, course),
                CourseRecord {
                    id: course,
                    tenant,
                    title: "Dispatch course".to_string(),
                    members: vec![CourseMembership {
                        user: instructor,
                        role: CourseMembershipRole::Instructor,
                    }],
                },
            );
        }
        establish_session(
            &store,
            session(10),
            tenant,
            instructor,
            vec![UserRole::Instructor],
        )
        .await;
        establish_session(
            &store,
            session(11),
            tenant,
            administrator,
            vec![UserRole::Administrator],
        )
        .await;
        let policy = InstitutionRetentionPolicy::new(
            RetentionDays::new(2).unwrap(),
            RetentionDays::new(4).unwrap(),
            RetentionDays::new(6).unwrap(),
        )
        .unwrap();
        store
            .configure_retention_policy(context, session(11), policy)
            .await
            .expect("admin policy");
        let record = store
            .end_course_retention(context, session(10), course)
            .await
            .expect("course end");
        let batch = RetentionDispatchBatch::new(3).expect("batch");
        for stage in [
            crate::RetentionStage::Notify,
            crate::RetentionStage::ArchiveStudentRecords,
            crate::RetentionStage::DeleteStudentRecords,
        ] {
            let due = record
                .snapshot
                .policy()
                .due_at(record.snapshot.ended_at(), stage)
                .unwrap();
            {
                let mut state = store.write_state().expect("state");
                state.authoritative_time =
                    ActivityTimestamp::from_unix_millis(due.as_unix_millis() - 1);
            }
            assert_eq!(store.dispatch_due_retention_stages(batch).await, Ok(0));
            {
                let mut state = store.write_state().expect("state");
                state.authoritative_time = due;
            }
            assert_eq!(store.dispatch_due_retention_stages(batch).await, Ok(1));
            assert_eq!(store.dispatch_due_retention_stages(batch).await, Ok(0));
            let claimed = store
                .claim_next_job(JobLeaseDuration::from_seconds(30).unwrap())
                .await
                .expect("claim")
                .expect("bound job");
            assert_eq!(
                claimed.payload,
                JobPayload::Retention {
                    course,
                    stage,
                    generation: 1,
                }
            );
            let command = RetentionWorkerCommand {
                tenant,
                course,
                stage,
                generation: 1,
                job: claimed.id,
                lease: claimed.lease_token,
            };
            store
                .prepare_retention_work(command)
                .await
                .expect("bound preparation");
            store
                .commit_retention_work(command)
                .await
                .expect("exact completion");
        }
        // A valid-looking but unbound job cannot execute under R3's worker API.
        let forged = store
            .enqueue_job(
                context,
                crate::EnqueueJob {
                    tenant,
                    payload: JobPayload::Retention {
                        course,
                        stage: crate::RetentionStage::Notify,
                        generation: 1,
                    },
                    max_attempts: 1,
                },
            )
            .await
            .expect("raw-looking job is queue-valid but not retention-bound");
        let claimed = store
            .claim_next_job(JobLeaseDuration::from_seconds(30).unwrap())
            .await
            .expect("claim forged")
            .expect("forged job");
        assert_eq!(claimed.id, forged);
        assert_eq!(
            store
                .prepare_retention_work(RetentionWorkerCommand {
                    tenant,
                    course,
                    stage: crate::RetentionStage::Notify,
                    generation: 1,
                    job: forged,
                    lease: claimed.lease_token,
                })
                .await,
            Err(StoreError::Conflict)
        );

        // The same trusted scheduler path honors an institution without an
        // explicit policy row: R1's 30-day default is still a real dispatch
        // deadline, not only a pure-policy calculation.
        let default_tenant = TenantId::from_uuid(Uuid::from_u128(81_104));
        let default_context = TenantContext::from_authenticated_session(default_tenant);
        let default_course = CourseId::from_uuid(Uuid::from_u128(81_105));
        {
            let mut state = store.write_state().expect("state");
            state.courses.insert(
                (default_tenant, default_course),
                CourseRecord {
                    id: default_course,
                    tenant: default_tenant,
                    title: "Default dispatch course".to_string(),
                    members: vec![CourseMembership {
                        user: instructor,
                        role: CourseMembershipRole::Instructor,
                    }],
                },
            );
        }
        establish_session(
            &store,
            session(12),
            default_tenant,
            instructor,
            vec![UserRole::Instructor],
        )
        .await;
        let default_record = store
            .end_course_retention(default_context, session(12), default_course)
            .await
            .expect("default-policy end");
        let default_due = default_record
            .snapshot
            .policy()
            .due_at(
                default_record.snapshot.ended_at(),
                crate::RetentionStage::Notify,
            )
            .expect("default due");
        {
            let mut state = store.write_state().expect("state");
            state.authoritative_time = default_due;
        }
        assert_eq!(store.dispatch_due_retention_stages(batch).await, Ok(1));
        let default_job = store
            .claim_next_job(JobLeaseDuration::from_seconds(30).unwrap())
            .await
            .expect("claim default")
            .expect("default job");
        assert_eq!(
            default_job.payload,
            JobPayload::Retention {
                course: default_course,
                stage: crate::RetentionStage::Notify,
                generation: 1,
            }
        );
    }

    #[tokio::test]
    async fn extension_and_disposition_are_authorized_and_generation_fenced() {
        let store = MemoryStore::default();
        let tenant = TenantId::from_uuid(Uuid::from_u128(81_200));
        let context = TenantContext::from_authenticated_session(tenant);
        let instructor = UserId::from_uuid(Uuid::from_u128(81_201));
        let student = UserId::from_uuid(Uuid::from_u128(81_202));
        let administrator = UserId::from_uuid(Uuid::from_u128(81_203));
        let course = CourseId::from_uuid(Uuid::from_u128(81_204));
        {
            let mut state = store.write_state().expect("state");
            state.authoritative_time = ActivityTimestamp::from_unix_millis(2_000_000);
            state.courses.insert(
                (tenant, course),
                CourseRecord {
                    id: course,
                    tenant,
                    title: "Extension course".to_string(),
                    members: vec![
                        CourseMembership {
                            user: instructor,
                            role: CourseMembershipRole::Instructor,
                        },
                        CourseMembership {
                            user: student,
                            role: CourseMembershipRole::Student,
                        },
                    ],
                },
            );
        }
        establish_session(
            &store,
            session(20),
            tenant,
            instructor,
            vec![UserRole::Instructor],
        )
        .await;
        establish_session(
            &store,
            session(21),
            tenant,
            student,
            vec![UserRole::Student],
        )
        .await;
        establish_session(
            &store,
            session(22),
            tenant,
            administrator,
            vec![UserRole::Administrator],
        )
        .await;
        assert_eq!(
            store
                .extend_course_retention(
                    context,
                    session(22),
                    course,
                    RetentionDays::new(1).unwrap(),
                )
                .await,
            Err(StoreError::Conflict),
            "an existing but unended course has no schedule to extend"
        );
        assert_eq!(
            store
                .extend_course_retention(
                    context,
                    session(22),
                    CourseId::from_uuid(Uuid::from_u128(81_299)),
                    RetentionDays::new(1).unwrap(),
                )
                .await,
            Err(StoreError::Forbidden),
            "missing courses remain nonenumerating"
        );
        let original = store
            .end_course_retention(context, session(20), course)
            .await
            .expect("end");
        assert_eq!(
            store
                .extend_course_retention(
                    context,
                    session(20),
                    course,
                    RetentionDays::new(1).unwrap()
                )
                .await,
            Err(StoreError::Forbidden)
        );
        assert_eq!(
            store
                .extend_course_retention(
                    context,
                    session(21),
                    course,
                    RetentionDays::new(1).unwrap()
                )
                .await,
            Err(StoreError::Forbidden)
        );
        let chosen = store
            .set_archive_disposition(
                context,
                session(20),
                course,
                AssignmentDefinitionDisposition::Delete,
            )
            .await
            .expect("instructor disposition");
        assert_eq!(
            chosen.status.assignment_definitions,
            AssignmentDefinitionDisposition::Delete
        );
        // A completed notification is historical: an extension copies it without
        // shifting or redelivering it, while future stages move to the new
        // generation.
        {
            let mut state = store.write_state().expect("state");
            let notify_key = (tenant, course, crate::RetentionStage::Notify, 1);
            let stored = state.retention_stages[&notify_key];
            state.retention_stages.insert(
                notify_key,
                StoredRetentionStage {
                    state: RetentionStageWorkState::Completed,
                    ..stored
                },
            );
            let notification_created_at = state.authoritative_time;
            state.retention_notifications.insert(
                (tenant, course, 1),
                crate::RetentionNotificationView {
                    intent: crate::RetentionNotificationIntent::Archive,
                    created_at: notification_created_at,
                },
            );
            // A scheduler may have handed a still-unstarted future stage to a
            // worker. Extension fences that lease by killing its exact dispatch
            // job before the generation changes.
            let leased_job = crate::JobId::from_uuid(Uuid::from_u128(81_205));
            let now = state.authoritative_time;
            state.jobs.insert(
                leased_job,
                StoredJob {
                    tenant,
                    payload: JobPayload::Retention {
                        course,
                        stage: crate::RetentionStage::ArchiveStudentRecords,
                        generation: 1,
                    },
                    state: JobState::Leased,
                    available_at: now,
                    lease_token: Some(JobLeaseToken::generate().expect("lease")),
                    lease_expires_at: Some(ActivityTimestamp::from_unix_millis(
                        now.as_unix_millis() + 10_000,
                    )),
                    attempt_count: 1,
                    max_attempts: RETENTION_JOB_MAX_ATTEMPTS,
                    failure: None,
                },
            );
            state.retention_dispatches.insert(
                (
                    tenant,
                    course,
                    crate::RetentionStage::ArchiveStudentRecords,
                    1,
                ),
                leased_job,
            );
        }
        let extended = store
            .extend_course_retention(context, session(22), course, RetentionDays::new(7).unwrap())
            .await
            .expect("admin extension");
        assert_eq!(extended.snapshot.generation(), 2);
        assert_eq!(
            extended.status.assignment_definitions,
            AssignmentDefinitionDisposition::Delete
        );
        let latest_notification = store
            .retention_notification(context, session(20), course)
            .await
            .expect("authorized notification read")
            .expect("completed notification remains readable after extension");
        assert_eq!(
            latest_notification.intent,
            crate::RetentionNotificationIntent::Archive
        );
        assert_eq!(
            latest_notification.created_at,
            ActivityTimestamp::from_unix_millis(2_000_000)
        );
        {
            let state = store.read_state().expect("state");
            for stage in [
                crate::RetentionStage::Notify,
                crate::RetentionStage::ArchiveStudentRecords,
                crate::RetentionStage::DeleteStudentRecords,
            ] {
                let old = state.retention_stages[&(tenant, course, stage, 1)];
                let new = state.retention_stages[&(tenant, course, stage, 2)];
                if stage == crate::RetentionStage::Notify {
                    assert_eq!(old.state, RetentionStageWorkState::Completed);
                    assert_eq!(new.state, RetentionStageWorkState::Completed);
                    assert_eq!(new.due_at, old.due_at);
                } else {
                    assert_eq!(old.state, RetentionStageWorkState::Superseded);
                    assert_eq!(
                        new.due_at.as_unix_millis(),
                        old.due_at.as_unix_millis() + 7 * 86_400_000
                    );
                }
            }
            assert_eq!(original.snapshot.ended_at(), extended.snapshot.ended_at());
            assert_eq!(
                state.jobs[&crate::JobId::from_uuid(Uuid::from_u128(81_205))].state,
                JobState::Dead,
                "extension must revoke a leased but unstarted dispatched stage"
            );
        }

        // The archive-time disposition freezes as soon as its own stage starts;
        // an administrator also cannot extend an in-progress generation.
        {
            let mut state = store.write_state().expect("state");
            let archive_key = (
                tenant,
                course,
                crate::RetentionStage::ArchiveStudentRecords,
                2,
            );
            let stored = state.retention_stages[&archive_key];
            state.retention_stages.insert(
                archive_key,
                StoredRetentionStage {
                    state: RetentionStageWorkState::Started,
                    ..stored
                },
            );
        }
        assert_eq!(
            store
                .set_archive_disposition(
                    context,
                    session(20),
                    course,
                    AssignmentDefinitionDisposition::Retain,
                )
                .await,
            Err(StoreError::Conflict)
        );
        assert_eq!(
            store
                .extend_course_retention(
                    context,
                    session(22),
                    course,
                    RetentionDays::new(1).unwrap(),
                )
                .await,
            Err(StoreError::Conflict)
        );
    }

    #[tokio::test]
    async fn retention_api_uses_safe_revision_cas_and_closed_manual_dispatch() {
        let store = MemoryStore::default();
        let tenant = TenantId::from_uuid(Uuid::from_u128(81_300));
        let context = TenantContext::from_authenticated_session(tenant);
        let instructor = UserId::from_uuid(Uuid::from_u128(81_301));
        let course = CourseId::from_uuid(Uuid::from_u128(81_302));
        {
            let mut state = store.write_state().expect("state");
            state.authoritative_time = ActivityTimestamp::from_unix_millis(3_000_000);
            state.courses.insert(
                (tenant, course),
                CourseRecord {
                    id: course,
                    tenant,
                    title: "Retention API course".to_string(),
                    members: vec![CourseMembership {
                        user: instructor,
                        role: CourseMembershipRole::Instructor,
                    }],
                },
            );
        }
        establish_session(
            &store,
            session(30),
            tenant,
            instructor,
            vec![UserRole::Instructor],
        )
        .await;
        store
            .end_course_retention(context, session(30), course)
            .await
            .expect("end course");
        let view = store
            .retention_view(context, session(30), course)
            .await
            .expect("safe view")
            .expect("ended view");
        assert_eq!(view.revision.value(), 1);
        let queued = store
            .request_retention_archive_if_revision(
                context,
                session(30),
                course,
                view.revision,
                AssignmentDefinitionDisposition::Delete,
            )
            .await
            .expect("queue archive");
        assert_eq!(queued.outcome, crate::RetentionRequestOutcome::Scheduled);
        assert_eq!(queued.retention.revision.value(), 2);
        assert_eq!(
            queued.retention.assignment_definitions,
            AssignmentDefinitionDisposition::Delete
        );
        let scheduled_replay = store
            .request_retention_archive_if_revision(
                context,
                session(30),
                course,
                view.revision,
                AssignmentDefinitionDisposition::Delete,
            )
            .await
            .expect("exact scheduled replay");
        assert_eq!(
            scheduled_replay.outcome,
            crate::RetentionRequestOutcome::Scheduled
        );
        assert_eq!(scheduled_replay.retention.revision.value(), 2);
        let current_revision_replay = store
            .request_retention_archive_if_revision(
                context,
                session(30),
                course,
                queued.retention.revision,
                AssignmentDefinitionDisposition::Delete,
            )
            .await
            .expect("current-revision scheduled replay");
        assert_eq!(
            current_revision_replay.outcome,
            crate::RetentionRequestOutcome::Scheduled
        );
        assert_eq!(current_revision_replay.retention.revision.value(), 2);
        assert_eq!(
            store
                .read_state()
                .expect("state")
                .jobs
                .values()
                .filter(|job| matches!(job.payload, JobPayload::Retention { course: candidate, stage: crate::RetentionStage::ArchiveStudentRecords, generation: 2 } if candidate == course))
                .count(),
            1,
            "exact scheduled replay creates no second job"
        );
        assert_eq!(
            store
                .request_retention_archive_if_revision(
                    context,
                    session(30),
                    course,
                    view.revision,
                    AssignmentDefinitionDisposition::Retain,
                )
                .await,
            Err(StoreError::Conflict),
            "a replay cannot silently change the requested disposition"
        );
        assert_eq!(
            store
                .request_retention_delete_if_revision(context, session(30), course, view.revision)
                .await,
            Err(StoreError::Conflict),
            "stale tabs cannot replace a queued request"
        );
        let job = store
            .claim_next_job(JobLeaseDuration::from_seconds(30).unwrap())
            .await
            .expect("claim")
            .expect("bound archive work");
        assert_eq!(
            job.payload,
            JobPayload::Retention {
                course,
                stage: crate::RetentionStage::ArchiveStudentRecords,
                generation: 2,
            }
        );
        let command = RetentionWorkerCommand {
            tenant,
            course,
            stage: crate::RetentionStage::ArchiveStudentRecords,
            generation: 2,
            job: job.id,
            lease: job.lease_token,
        };
        store
            .prepare_retention_work(command)
            .await
            .expect("start archive");
        let in_progress = store
            .request_retention_archive_if_revision(
                context,
                session(30),
                course,
                view.revision,
                AssignmentDefinitionDisposition::Delete,
            )
            .await
            .expect("exact in-progress replay");
        assert_eq!(
            in_progress.outcome,
            crate::RetentionRequestOutcome::InProgress
        );
        assert_eq!(in_progress.retention.revision, queued.retention.revision);
        store
            .commit_retention_work(command)
            .await
            .expect("complete archive");
        assert_eq!(
            store
                .request_retention_archive_if_revision(
                    context,
                    session(30),
                    course,
                    view.revision,
                    AssignmentDefinitionDisposition::Delete,
                )
                .await
                .expect("exact completed replay")
                .outcome,
            crate::RetentionRequestOutcome::Completed
        );
    }
}

#[cfg(test)]
mod retention_worker_tests {
    use super::*;
    use crate::{JobLeaseToken, RetentionWorkerCommand, RetentionWorkerStore};

    fn id(value: u128) -> Uuid {
        Uuid::from_u128(value)
    }
    fn fixture() -> (
        MemoryStore,
        TenantContext,
        CourseId,
        crate::JobId,
        JobLeaseToken,
        ActivityTimestamp,
    ) {
        let store = MemoryStore::default();
        let tenant = TenantId::from_uuid(id(82_001));
        let context = TenantContext::from_authenticated_session(tenant);
        let course = CourseId::from_uuid(id(82_002));
        let job = crate::JobId::from_uuid(id(82_003));
        let lease = JobLeaseToken::generate().expect("lease");
        let now = ActivityTimestamp::from_unix_millis(2_000_000);
        let snapshot = CourseRetentionSnapshot::new(
            now,
            InstitutionRetentionPolicy::default(),
            AssignmentDefinitionDisposition::Retain,
            1,
        )
        .expect("snapshot");
        let due = snapshot
            .policy()
            .due_at(now, crate::RetentionStage::Notify)
            .expect("due");
        let mut state = store.write_state().expect("state");
        state.authoritative_time = ActivityTimestamp::from_unix_millis(due.as_unix_millis() - 1);
        state.courses.insert(
            (tenant, course),
            CourseRecord {
                id: course,
                tenant,
                title: "Retention worker test course".to_string(),
                members: Vec::new(),
            },
        );
        state.course_retention.insert(
            (tenant, course),
            CourseRetentionRecord {
                snapshot,
                status: crate::CourseRetentionStatus::from_persisted(
                    CourseRetentionState::Active,
                    AssignmentDefinitionDisposition::Retain,
                ),
            },
        );
        state.retention_stages.insert(
            (tenant, course, crate::RetentionStage::Notify, 1),
            StoredRetentionStage {
                due_at: due,
                state: RetentionStageWorkState::Scheduled,
                job: None,
                lease: None,
            },
        );
        state.jobs.insert(
            job,
            StoredJob {
                tenant,
                payload: crate::JobPayload::Retention {
                    course,
                    stage: crate::RetentionStage::Notify,
                    generation: 1,
                },
                state: crate::JobState::Leased,
                available_at: now,
                lease_token: Some(lease),
                lease_expires_at: Some(ActivityTimestamp::from_unix_millis(
                    due.as_unix_millis() + 10_000,
                )),
                attempt_count: 1,
                max_attempts: 2,
                failure: None,
            },
        );
        state
            .retention_dispatches
            .insert((tenant, course, crate::RetentionStage::Notify, 1), job);
        drop(state);
        (store, context, course, job, lease, due)
    }

    #[tokio::test]
    async fn retention_worker_requires_due_time_exact_preparation_and_current_lease() {
        let (store, context, course, job, lease, due) = fixture();
        let command = RetentionWorkerCommand {
            tenant: context.tenant_id(),
            course,
            stage: crate::RetentionStage::Notify,
            generation: 1,
            job,
            lease,
        };
        assert_eq!(
            store.prepare_retention_work(command).await,
            Err(StoreError::Conflict),
            "due minus one millisecond must not run"
        );
        {
            let mut state = store.write_state().expect("state");
            state.authoritative_time = due;
        }
        assert!(matches!(
            store.prepare_retention_work(command).await,
            Ok(RetentionWork::Notify)
        ));
        let other_job = crate::JobId::from_uuid(id(82_004));
        let other_lease = JobLeaseToken::generate().expect("lease");
        {
            let mut state = store.write_state().expect("state");
            state.jobs.insert(
                other_job,
                StoredJob {
                    tenant: context.tenant_id(),
                    payload: crate::JobPayload::Retention {
                        course,
                        stage: crate::RetentionStage::Notify,
                        generation: 1,
                    },
                    state: crate::JobState::Leased,
                    available_at: due,
                    lease_token: Some(other_lease),
                    lease_expires_at: Some(ActivityTimestamp::from_unix_millis(
                        due.as_unix_millis() + 10_000,
                    )),
                    attempt_count: 1,
                    max_attempts: 2,
                    failure: None,
                },
            );
        }
        let different = RetentionWorkerCommand {
            job: other_job,
            lease: other_lease,
            ..command
        };
        assert_eq!(
            store.prepare_retention_work(different).await,
            Err(StoreError::Conflict)
        );
        assert_eq!(
            store.commit_retention_work(different).await,
            Err(StoreError::Conflict)
        );
        assert!(store.commit_retention_work(command).await.is_ok());
        let record = store
            .course_retention(context, SessionTokenHash::compute(&[9; 32]), course)
            .await;
        assert_eq!(record, Err(StoreError::Forbidden));
        let state = store.read_state().expect("state");
        assert_eq!(
            state
                .retention_notifications
                .get(&(context.tenant_id(), course, 1)),
            Some(&crate::RetentionNotificationView {
                intent: crate::RetentionNotificationIntent::Archive,
                created_at: due,
            })
        );
        assert_eq!(
            state.course_retention[&(context.tenant_id(), course)]
                .status
                .state,
            CourseRetentionState::Active
        );
    }

    #[tokio::test]
    async fn archive_cleanup_terminalizes_uncommitted_export_and_returns_every_expected_typed_key()
    {
        let (store, context, course, job, lease, due) = fixture();
        let export_job = crate::JobId::from_uuid(id(82_010));
        let export = crate::ExportId::from_uuid(id(82_011));
        let expected = [
            (
                crate::ExportArtifactKind::Docx,
                question_model::ObjectId::from_uuid(id(82_012)),
            ),
            (
                crate::ExportArtifactKind::Pdf,
                question_model::ObjectId::from_uuid(id(82_013)),
            ),
            (
                crate::ExportArtifactKind::AccessibleDocx,
                question_model::ObjectId::from_uuid(id(82_014)),
            ),
            (
                crate::ExportArtifactKind::AccessiblePdf,
                question_model::ObjectId::from_uuid(id(82_015)),
            ),
        ]
        .into_iter()
        .collect::<BTreeMap<_, _>>();
        {
            let mut state = store.write_state().expect("state");
            state.authoritative_time = due;
            state.jobs.get_mut(&job).expect("retention job").payload =
                crate::JobPayload::Retention {
                    course,
                    stage: crate::RetentionStage::ArchiveStudentRecords,
                    generation: 1,
                };
            state.retention_dispatches.remove(&(
                context.tenant_id(),
                course,
                crate::RetentionStage::Notify,
                1,
            ));
            state.retention_dispatches.insert(
                (
                    context.tenant_id(),
                    course,
                    crate::RetentionStage::ArchiveStudentRecords,
                    1,
                ),
                job,
            );
            state.retention_stages.insert(
                (
                    context.tenant_id(),
                    course,
                    crate::RetentionStage::ArchiveStudentRecords,
                    1,
                ),
                StoredRetentionStage {
                    due_at: due,
                    state: RetentionStageWorkState::Scheduled,
                    job: None,
                    lease: None,
                },
            );
            state.jobs.insert(
                export_job,
                StoredJob {
                    tenant: context.tenant_id(),
                    payload: crate::JobPayload::Export {
                        delivery_object: question_model::ObjectId::from_uuid(id(82_016)),
                    },
                    state: crate::JobState::Leased,
                    available_at: due,
                    lease_token: Some(JobLeaseToken::generate().expect("export lease")),
                    lease_expires_at: Some(ActivityTimestamp::from_unix_millis(
                        due.as_unix_millis() + 1_000,
                    )),
                    attempt_count: 1,
                    max_attempts: 2,
                    failure: None,
                },
            );
            state.exports.insert(
                (context.tenant_id(), export),
                StoredExport {
                    course,
                    assignment: AssignmentId::from_uuid(id(82_017)),
                    title: "retention fixture".to_string(),
                    requested_by: UserId::from_uuid(id(82_018)),
                    manifest: question_model::ObjectId::from_uuid(id(82_016)),
                    problems: Vec::new(),
                    job: export_job,
                    state: crate::StudentExportState::Queued,
                    expected: expected.clone(),
                    artifacts: None,
                },
            );
        }
        let command = RetentionWorkerCommand {
            tenant: context.tenant_id(),
            course,
            stage: crate::RetentionStage::ArchiveStudentRecords,
            generation: 1,
            job,
            lease,
        };
        let RetentionWork::Cleanup(manifest) = store
            .prepare_retention_work(command)
            .await
            .expect("due cleanup")
        else {
            panic!("cleanup work")
        };
        assert_eq!(manifest.objects().len(), 4);
        assert!(manifest.objects().iter().all(|key| matches!(key, objects::ObjectKey::StudentRecord { tenant, .. } if *tenant == context.tenant_id())));
        let state = store.read_state().expect("state");
        assert_eq!(
            state.exports[&(context.tenant_id(), export)].state,
            crate::StudentExportState::Failed
        );
        assert_eq!(state.jobs[&export_job].state, crate::JobState::Dead);
        assert_eq!(
            state.course_retention[&(context.tenant_id(), course)]
                .status
                .state,
            CourseRetentionState::Active
        );
    }

    #[tokio::test]
    async fn archive_prepare_fences_access_and_exact_commit_archives_atomically() {
        let (store, context, course, job, lease, due) = fixture();
        {
            let mut state = store.write_state().expect("state");
            state.authoritative_time = due;
            state.retention_stages.remove(&(
                context.tenant_id(),
                course,
                crate::RetentionStage::Notify,
                1,
            ));
            state.retention_dispatches.remove(&(
                context.tenant_id(),
                course,
                crate::RetentionStage::Notify,
                1,
            ));
            state.retention_stages.insert(
                (
                    context.tenant_id(),
                    course,
                    crate::RetentionStage::ArchiveStudentRecords,
                    1,
                ),
                StoredRetentionStage {
                    due_at: due,
                    state: RetentionStageWorkState::Scheduled,
                    job: None,
                    lease: None,
                },
            );
            state.retention_dispatches.insert(
                (
                    context.tenant_id(),
                    course,
                    crate::RetentionStage::ArchiveStudentRecords,
                    1,
                ),
                job,
            );
            state.jobs.get_mut(&job).expect("job").payload = crate::JobPayload::Retention {
                course,
                stage: crate::RetentionStage::ArchiveStudentRecords,
                generation: 1,
            };
        }
        assert!(
            store
                .course_records_accessible(context, course)
                .await
                .expect("ordinary access")
        );
        let command = RetentionWorkerCommand {
            tenant: context.tenant_id(),
            course,
            stage: crate::RetentionStage::ArchiveStudentRecords,
            generation: 1,
            job,
            lease,
        };
        assert!(matches!(
            store
                .prepare_retention_work(command)
                .await
                .expect("prepare"),
            RetentionWork::Cleanup(_)
        ));
        assert!(
            !store
                .course_records_accessible(context, course)
                .await
                .expect("prepare fence")
        );
        assert_eq!(
            store.read_state().expect("state").course_retention[&(context.tenant_id(), course)]
                .status
                .state,
            CourseRetentionState::Active
        );
        store.commit_retention_work(command).await.expect("commit");
        let state = store.read_state().expect("state");
        assert_eq!(
            state.course_retention[&(context.tenant_id(), course)]
                .status
                .state,
            CourseRetentionState::StudentRecordsArchived
        );
        assert_eq!(
            state.retention_stages[&(
                context.tenant_id(),
                course,
                crate::RetentionStage::ArchiveStudentRecords,
                1,
            )]
                .state,
            RetentionStageWorkState::Completed
        );
        assert_eq!(state.jobs[&job].state, crate::JobState::Completed);
    }
}
