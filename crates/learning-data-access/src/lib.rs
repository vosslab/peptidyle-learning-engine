//! Backend-neutral persistence contract (WP-C4, MOD-STO).
//!
//! Globally public content needs no tenant context. Institution-visible
//! catalog content goes through [`CatalogStore`], whose reads require
//! [`TenantContext`]. Every educational-record operation also requires that
//! non-defaultable context. Lists require a bounded [`PageRequest`]; the trait
//! has no unbounded or positional paging method. No SQL type appears in this
//! contract.

use async_trait::async_trait;
use base64::Engine;
use domain::run::RunModelError;
use objects::{ObjectRecord, Sha256Digest};
use question_model::FeedbackContent;
use question_model::taxonomy::TaxonomyTerm;
use question_model::{
    ActivityTimestamp, AssignmentDeliveryState, AssignmentEnrollment, AssignmentId, AssignmentItem,
    AssignmentItemId, AssignmentPolicyExceptionId, AssignmentRun, AssignmentRunItem,
    AssignmentSelectionGroup, AssignmentSummary, AssignmentTimingPolicy, AttemptProvenance,
    AttemptResult, AttemptStatus, BackendCapabilities, CatalogLifecycle, CatalogProblemDetail,
    CatalogProblemSummary, CatalogSearchPage, CatalogSearchQuery, CourseGroupId, CourseId,
    CourseMembership, CourseRole, CourseSummary, DraftQuestionDefinition, EnrollmentId,
    GradePolicy, GradebookSummaryRow, ProblemId, PublicationScope, QuestionAttempt,
    QuestionAttemptId, QuestionBackend, QuestionDefinition, QuestionStatisticsDisclosure, RunId,
    RunPolicies, ScoringGeneration, ScoringStatus, SelectionOrdering, StudentAssignmentSummary,
    StudentId, StudentResponse, TenantId, UserId, VersionId, WorkspaceDraftSummary, WorkspaceId,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

mod activity_policy;
mod asset_delivery;
mod external_tool;
mod feedback;
mod flat_import_provenance;
mod flat_question;
mod gradebook_cursor;
pub mod in_memory;
mod item_analysis;
/// In-memory backend used by tests and lanes waiting for PostgreSQL.
pub mod jobs;
mod manual_grading;
/// Cursor and bounded-page types shared by every list method.
pub mod pagination;
mod policy;
/// PostgreSQL health and future backend implementation.
pub mod postgres;
mod publication_validation;
mod qti;
mod qti_ingress;
/// Pure retention lifecycle policy; persistence and worker execution land in MOD-RETENTION R2+.
pub mod retention;
/// Explicit tenant context used by every educational-record operation.
pub mod rls;
mod run_summary_cursor;
mod score_precision;
/// Provider-neutral, replica-safe authentication session contract.
pub mod session;
mod statistics;

pub use crate::asset_delivery::{
    AssetAccessEvent, AssetDeliveryId, AssetDeliveryRecord, AssetDeliveryScope, AssetStore,
    AuthorizedAssetDelivery, CatalogAssetBinding,
};
pub(crate) use crate::external_tool::fresh_external_tool_launch_id;
pub use crate::external_tool::{
    BeginExternalToolGradeCommand, CommitExternalToolSubmissionCommand,
    CommitVerifiedExternalToolSubmissionCommand, CreateExternalToolLaunchSessionCommand,
    CreatedExternalToolLaunchSession, ExternalToolBegin, ExternalToolBinding,
    ExternalToolBrokerStore, ExternalToolLaunchProof, ExternalToolLaunchSessionStore,
    ExternalToolLaunchToken, ExternalToolLease, ExternalToolLeaseToken,
    ExternalToolVerifiedPending, PersistedCorrelation, ResolvedExternalToolLaunchSession,
    StageExternalToolVerificationCommand,
};
pub use crate::feedback::{
    AttemptFeedbackRecord, FeedbackReleaseRecord, ReleaseAttemptFeedbackCommand,
    RunSummaryOutcomeInput, RunSummaryPageInput, private_feedback_record,
};
pub use crate::flat_import_provenance::{
    FlatImportChoiceMapPayload, FlatImportConversionVersion, FlatImportIntegrityDigests,
    FlatImportProvenanceStore, FlatImportPublicationPromotion, PersistedFlatImportProfile,
    PublishedFlatImportOrigin, QTI_PROFILE_ARCHIVE_MEDIA_TYPE, QtiProfileFlatConversionCommand,
    QtiProfileImportEvidence, WorkspaceFlatImportOrigin, WorkspaceFlatImportOriginIdentity,
};
pub use crate::flat_question::{
    FlatQuestionGradingPayload, FlatQuestionGradingStore, FlatQuestionPublicationPromotion,
    FlatQuestionStore, UpsertFlatQuestionCommand, WorkspaceFlatQuestionSource,
};
pub use crate::item_analysis::{
    CourseItemAnalysisCommitOutcome, CourseItemAnalysisStore, CourseItemAnalysisWorkerCommand,
    CourseItemAnalysisWorkerStore,
};
pub use crate::jobs::{
    ClaimedJob, CreateAssignmentExport, EnqueueJob, ExportArtifactKind, ExportArtifactRecord,
    ExportCommitDisposition, ExportId, ExportJobCommit, ExportJobStore, JobClaimFilter,
    JobFailureDisposition, JobFailureKind, JobId, JobKind, JobLeaseDuration, JobLeaseToken,
    JobPayload, JobState, JobStore, QueueDepth, StudentExportArtifactView, StudentExportJob,
    StudentExportState, StudentExportView, TenantJobView,
};
pub use crate::manual_grading::{
    EvaluationRevision, ManualCredit, ManualEvaluationRecord, ManualEvaluationStatus,
    ManualGradeActionId, ManualGradeReceipt, ManualGradingStore, SetManualGradeCommand,
    SubmitPendingManualQuestionAttemptCommand,
};
pub use crate::pagination::{Cursor, Page, PageRequest, PageSize, PaginationError};
pub use crate::policy::{
    AssignmentExceptionLimit, AssignmentExceptionTimestamp, AssignmentPolicyException,
    AssignmentPolicyExceptionTarget, CourseGroupRecord, CourseGroupRevision,
    DeleteAssignmentPolicyExceptionCommand, PutCourseGroupCommand, ResolvedAssignmentTiming,
    ResolvedAttemptTiming, SetAssignmentPolicyExceptionCommand, StoredAssignmentPolicyException,
    StoredCourseGroup,
};
pub(crate) use crate::policy::{
    ResolvedAssignmentTimingPolicy, resolve_assignment_policy, validate_assignment_policy_exception,
};
pub use crate::qti::{
    CommitPreparedQtiImport, CommitPreparedQtiImportOutcome, CreateQtiImportCommand,
    QtiGradingStore, QtiImportGradingPayload, QtiImportItem, QtiImportItemRegistration,
    QtiImportItemResult, QtiImportItemStatus, QtiImportProfileSummary, QtiImportRef,
    QtiImportRegistry, QtiImportStore, QtiPublicationPromotion, QtiUnsupportedFeature,
};
pub(crate) use crate::qti_ingress::validate_queue_qti_import;
pub use crate::qti_ingress::{
    QtiImportApiState, QtiImportApiStore, QtiImportApiView, QueueQtiImportCommand,
    qti_import_job_id,
};
pub use crate::retention::{
    AssignmentDefinitionDisposition, CourseRetentionRecord, CourseRetentionSnapshot,
    CourseRetentionState, CourseRetentionStatus, CourseRetentionView,
    DEFAULT_RETENTION_ARCHIVE_DAYS, DEFAULT_RETENTION_DELETE_DAYS, DEFAULT_RETENTION_NOTIFY_DAYS,
    InstitutionRetentionPolicy, MAX_RETENTION_DAYS, MAX_RETENTION_DISPATCH_BATCH,
    RETENTION_ARCHIVE_NOTIFICATION_COPY, RETENTION_JOB_MAX_ATTEMPTS, RetentionCleanupManifest,
    RetentionDays, RetentionDispatchBatch, RetentionNotificationIntent, RetentionNotificationView,
    RetentionPolicyError, RetentionRequestOutcome, RetentionRequestResult, RetentionRevision,
    RetentionStage, RetentionWork, RetentionWorkerCommand,
};
pub use crate::rls::TenantContext;
pub use crate::session::{
    SessionLifetime, SessionRecord, SessionStore, SessionSubject, SessionSubjectError,
    SessionTokenHash, SessionTokenHashParseError,
};
#[cfg(test)]
pub(crate) use activity_policy::CurrentRunQuestion;
pub(crate) use activity_policy::{
    completed_run_score, current_run_questions, ensure_tenant, grade_policy,
    project_enrollment_completion, summary_transition, validate_asset_delivery,
    validate_assignment, validate_assignment_timing, validate_attempt_result, validate_course,
    validate_course_group,
};
#[cfg(feature = "postgres")]
pub(crate) use publication_validation::validate_source_artifact_identity;
pub(crate) use publication_validation::{
    validate_draft, validate_flat_question_publication, validate_publication_source,
    validate_published, validate_qti_import, validate_qti_publication_promotion,
    validate_source_artifact_for_publication,
};

/// The storage authority for security-sensitive expiry calculations. Server
/// routes must not consult a process wall clock because replicas can disagree.
#[async_trait]
pub trait AuthoritativeTimeStore: Send + Sync {
    async fn authoritative_time(
        &self,
        context: TenantContext,
    ) -> Result<ActivityTimestamp, StoreError>;
}

/// Retention persistence boundary. Every mutator authenticates the supplied
/// stored session and derives administrator or course-instructor authority
/// from persisted session/course data; no request supplies a role or tenant.
#[async_trait]
pub trait RetentionStore: Send + Sync {
    /// Updates this tenant's future-course policy after stored-admin validation.
    async fn configure_retention_policy(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        policy: InstitutionRetentionPolicy,
    ) -> Result<(), StoreError>;

    /// Ends a course at the backend authoritative time and snapshots its policy.
    async fn end_course_retention(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        course: CourseId,
    ) -> Result<CourseRetentionRecord, StoreError>;

    /// Reads an ended course's retention record after stored-session/course authorization.
    async fn course_retention(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        course: CourseId,
    ) -> Result<Option<CourseRetentionRecord>, StoreError>;
}

/// Private schedule-control boundary for the retention service.
///
/// The dispatcher accepts only a validated batch size: no caller can name a
/// tenant, course, stage, timestamp, generation, job, or object key. The two
/// authenticated mutators derive all authority from the persisted session.
#[async_trait]
pub trait RetentionScheduleStore: Send + Sync {
    /// Dispatches due current-generation stages into bound closed queue jobs.
    async fn dispatch_due_retention_stages(
        &self,
        batch: RetentionDispatchBatch,
    ) -> Result<u16, StoreError>;

    /// Extends only unstarted current stages after stored-administrator validation.
    async fn extend_course_retention(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        course: CourseId,
        additional_days: RetentionDays,
    ) -> Result<CourseRetentionRecord, StoreError>;

    /// Records the explicit archive-time assignment-definition choice.
    async fn set_archive_disposition(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        course: CourseId,
        disposition: AssignmentDefinitionDisposition,
    ) -> Result<CourseRetentionRecord, StoreError>;
}

/// Browser-facing retention control boundary.
///
/// It exposes only a safe lifecycle projection and conditional instructor
/// requests. Archive and delete requests enter the existing broker-owned
/// dispatch contract; they never execute cleanup in a request transaction.
#[async_trait]
pub trait RetentionApiStore: Send + Sync {
    /// Reads the key-free retention projection for an authorized course actor.
    async fn retention_view(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        course: CourseId,
    ) -> Result<Option<CourseRetentionView>, StoreError>;

    /// Reads the current durable in-app notification intent, if one exists.
    async fn retention_notification(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        course: CourseId,
    ) -> Result<Option<RetentionNotificationView>, StoreError>;

    /// Conditionally extends an active schedule after an exact revision match.
    async fn extend_retention_if_revision(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        course: CourseId,
        expected: RetentionRevision,
        additional_days: RetentionDays,
    ) -> Result<CourseRetentionView, StoreError>;

    /// Conditionally requests immediate archive work through the closed broker.
    async fn request_retention_archive_if_revision(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        course: CourseId,
        expected: RetentionRevision,
        disposition: AssignmentDefinitionDisposition,
    ) -> Result<RetentionRequestResult, StoreError>;

    /// Conditionally requests immediate permanent-delete work through the broker.
    async fn request_retention_delete_if_revision(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        course: CourseId,
        expected: RetentionRevision,
    ) -> Result<RetentionRequestResult, StoreError>;
}

/// Server-worker retention boundary. The only input is a current queue lease
/// plus the closed course/stage/generation identity; the Store resolves every
/// educational-record object and performs all access revocation itself.
#[async_trait]
pub trait RetentionWorkerStore: Send + Sync {
    /// Claims a current scheduled stage and returns private exact work. A
    /// stale generation or lease is rejected before any object key is exposed.
    async fn prepare_retention_work(
        &self,
        command: RetentionWorkerCommand,
    ) -> Result<RetentionWork, StoreError>;

    /// Finalizes the exact claimed stage only after its worker completed every
    /// external idempotent effect. The implementation also completes `job` in
    /// the same conditional transition.
    async fn commit_retention_work(
        &self,
        command: RetentionWorkerCommand,
    ) -> Result<(), StoreError>;
}

/// Lease- and generation-fenced scoring rebuild command used only by workers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AssignmentScoringWorkerCommand {
    pub job: JobId,
    pub lease: JobLeaseToken,
    pub assignment: AssignmentId,
    pub generation: ScoringGeneration,
}

/// Result of atomically publishing one prepared scoring generation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AssignmentScoringCommitOutcome {
    /// This generation replaced every current computed score and summary.
    Committed,
    /// A newer generation superseded this work; its staging rows were discarded.
    Superseded,
    /// The queue lease expired or was reclaimed before publication.
    ClaimNoLongerActive,
}

/// Private staging and atomic-publication boundary for assignment rescoring.
#[async_trait]
pub trait AssignmentScoringWorkerStore: Send + Sync {
    /// Rebuilds private staging rows without changing learner-visible scores.
    async fn prepare_assignment_scoring(
        &self,
        context: TenantContext,
        command: AssignmentScoringWorkerCommand,
    ) -> Result<(), StoreError>;

    /// Conditionally replaces current rows and completes the exact queue lease.
    async fn commit_assignment_scoring(
        &self,
        context: TenantContext,
        command: AssignmentScoringWorkerCommand,
    ) -> Result<AssignmentScoringCommitOutcome, StoreError>;
}

/// Lease- and generation-fenced command for one scheduled auto-submit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AttemptAutoSubmitWorkerCommand {
    pub job: JobId,
    pub lease: JobLeaseToken,
    pub attempt: QuestionAttemptId,
    pub timing_generation: u64,
}

/// Result of atomically resolving one scheduled attempt deadline.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AttemptAutoSubmitCommitOutcome {
    /// This invocation changed active work to `auto_submitted`.
    AutoSubmitted,
    /// A newer timing policy or terminal attempt made this job obsolete.
    Superseded,
    /// An extension moved the effective deadline and the same job was rescheduled.
    Rescheduled,
    /// The queue lease expired or was reclaimed before the transaction committed.
    ClaimNoLongerActive,
}

/// Durable deadline-finalization boundary used by the stateless worker.
#[async_trait]
pub trait AttemptAutoSubmitWorkerStore: Send + Sync {
    /// Re-resolves the current timing row and either submits, supersedes, or
    /// reschedules the exact leased job in one transaction.
    async fn commit_attempt_auto_submit(
        &self,
        context: TenantContext,
        command: AttemptAutoSubmitWorkerCommand,
    ) -> Result<AttemptAutoSubmitCommitOutcome, StoreError>;
}

/// Database-backed visibility boundary for ordinary learner course records.
///
/// This deliberately returns only a boolean: lifecycle stages and retention
/// schedule details remain Store-private. Implementations must evaluate the
/// course existence and current retention fence in the same backend security
/// context as the later record operation.
#[async_trait]
pub trait CourseRecordsAccessStore: Send + Sync {
    /// Returns false for a missing course, a foreign tenant, an archived or
    /// deleted course, or a current-generation archive stage already started.
    async fn course_records_accessible(
        &self,
        context: TenantContext,
        course: CourseId,
    ) -> Result<bool, StoreError>;
}

/// Encodes a catalog keyset continuation without exposing UUID text in URLs.
/// The fixed binary layout binds the continuation to the normalized-query
/// digest; callers still verify it before using it as a SQL cursor.
pub(crate) fn encode_catalog_search_cursor(
    fingerprint: &str,
    problem: Uuid,
    version: Uuid,
) -> String {
    debug_assert_eq!(fingerprint.len(), 64);
    let mut bytes = Vec::with_capacity(129);
    bytes.push(1);
    bytes.extend_from_slice(fingerprint.as_bytes());
    bytes.extend_from_slice(problem.as_bytes());
    bytes.extend_from_slice(version.as_bytes());
    let integrity = objects::Sha256Digest::compute(&bytes);
    bytes.extend_from_slice(integrity.as_bytes());
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}

/// Decodes a canonical bounded catalog continuation and rejects a different
/// normalized query before a storage key can be used.
pub(crate) fn decode_catalog_search_cursor(
    cursor: &str,
    fingerprint: &str,
) -> Result<(Uuid, Uuid), StoreError> {
    if cursor.len() > 200 {
        return Err(StoreError::InvalidRecord(
            "catalog cursor is malformed".to_string(),
        ));
    }
    let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(cursor)
        .map_err(|_| StoreError::InvalidRecord("catalog cursor is malformed".to_string()))?;
    if bytes.len() != 129
        || bytes[0] != 1
        || bytes[1..65] != *fingerprint.as_bytes()
        || objects::Sha256Digest::compute(&bytes[..97]).as_bytes() != &bytes[97..129]
    {
        return Err(StoreError::InvalidRecord(
            "catalog cursor does not belong to this normalized query".to_string(),
        ));
    }
    let problem = Uuid::from_slice(&bytes[65..81])
        .map_err(|_| StoreError::InvalidRecord("catalog cursor is malformed".to_string()))?;
    let version = Uuid::from_slice(&bytes[81..97])
        .map_err(|_| StoreError::InvalidRecord("catalog cursor is malformed".to_string()))?;
    if encode_catalog_search_cursor(fingerprint, problem, version) != cursor {
        return Err(StoreError::InvalidRecord(
            "catalog cursor is malformed".to_string(),
        ));
    }
    Ok((problem, version))
}

/// Encodes a tenant-bound opaque continuation for workspace-draft listing.
///
/// The stable workspace UUID never appears directly in an API cursor. Binding
/// it to the tenant prevents a continuation issued to one tenant from being
/// replayed against another tenant's private workspace list.
pub(crate) fn encode_workspace_draft_cursor(tenant: TenantId, workspace: WorkspaceId) -> String {
    let mut bytes = Vec::with_capacity(65);
    bytes.push(1);
    bytes.extend_from_slice(tenant.as_uuid().as_bytes());
    bytes.extend_from_slice(workspace.as_uuid().as_bytes());
    let integrity = Sha256Digest::compute(&bytes);
    bytes.extend_from_slice(integrity.as_bytes());
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}

/// Decodes a workspace-draft continuation only for the tenant that received it.
pub(crate) fn decode_workspace_draft_cursor(
    cursor: &str,
    tenant: TenantId,
) -> Result<WorkspaceId, StoreError> {
    if cursor.len() > 128 {
        return Err(StoreError::InvalidRecord(
            "workspace cursor is malformed".to_string(),
        ));
    }
    let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(cursor)
        .map_err(|_| StoreError::InvalidRecord("workspace cursor is malformed".to_string()))?;
    let tenant_id = tenant.as_uuid();
    let tenant_bytes = tenant_id.as_bytes();
    if bytes.len() != 65
        || bytes[0] != 1
        || bytes[1..17] != *tenant_bytes
        || Sha256Digest::compute(&bytes[..33]).as_bytes() != &bytes[33..65]
    {
        return Err(StoreError::InvalidRecord(
            "workspace cursor does not belong to this tenant".to_string(),
        ));
    }
    let workspace = Uuid::from_slice(&bytes[17..33])
        .map_err(|_| StoreError::InvalidRecord("workspace cursor is malformed".to_string()))?;
    let workspace = WorkspaceId::from_uuid(workspace);
    if encode_workspace_draft_cursor(tenant, workspace) != cursor {
        return Err(StoreError::InvalidRecord(
            "workspace cursor is malformed".to_string(),
        ));
    }
    Ok(workspace)
}

/// Shared immutable problem/version reference used by catalog lineage.
pub use question_model::ProblemVersionRef;
/// Compatibility name used by the existing assignment contract.
pub type PublishedVersionRef = ProblemVersionRef;

/// Server-only immutable source-object binding for one published version.
///
/// This is deliberately separate from browser catalog payloads.  Backends use
/// it to resolve the exact bytes that were prepared before publication; an
/// adapter must never reconstruct an object key from a browser value.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PublishedSourceArtifact {
    /// Published version that owns the immutable source bytes.
    pub reference: ProblemVersionRef,
    /// Backend whose source preparation produced this object.
    pub backend: QuestionBackend,
    /// Verified content-bucket object record.
    pub object: ObjectRecord,
}

/// Tenant-owned editable question draft.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DraftRecord {
    /// Direct RLS boundary.
    pub tenant: TenantId,
    /// Editable content with no published identifiers.
    pub question: DraftQuestionDefinition,
    /// Earlier version in the same owned linear chain, for a new revision.
    pub revises: Option<ProblemVersionRef>,
    /// Source version when creating a new attributed fork.
    pub derived_from: Option<ProblemVersionRef>,
}

/// Persisted authority for one authenticated person in a private workspace.
///
/// An owner is established atomically with the first draft write. A
/// collaborator can inspect and revise the workspace, but cannot transfer
/// access, delete it, or publish it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum WorkspaceDraftRole {
    Owner,
    Collaborator,
}

/// Server-issued optimistic-concurrency value for an editable workspace.
///
/// The value is stored as a positive PostgreSQL `bigint`; callers obtain it
/// only from a successful read or write and must echo it on an update.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct WorkspaceDraftRevision(u64);

impl WorkspaceDraftRevision {
    pub(crate) const INITIAL: Self = Self(1);
    const MAX: u64 = i64::MAX as u64;

    /// Returns the value for browser-safe request/response serialization.
    pub fn value(self) -> u64 {
        self.0
    }

    fn next(self) -> Result<Self, StoreError> {
        let next = self.0.checked_add(1).filter(|value| *value <= Self::MAX);
        next.map(Self).ok_or_else(|| {
            StoreError::Unavailable("workspace draft revision limit reached".to_string())
        })
    }

    #[cfg(feature = "postgres")]
    pub(crate) fn from_stored(value: i64) -> Result<Self, StoreError> {
        let value = u64::try_from(value).map_err(|_| {
            StoreError::Unavailable("stored workspace draft revision is invalid".to_string())
        })?;
        if value == 0 {
            return Err(StoreError::Unavailable(
                "stored workspace draft revision is invalid".to_string(),
            ));
        }
        Ok(Self(value))
    }
}

/// Server-issued optimistic-concurrency value for one editable assignment.
///
/// Assignment definitions are tenant-owned course artifacts.  Their selected
/// published versions stay immutable, while the ordered selection and policies
/// change only through this compare-and-swap token.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AssignmentRevision(u64);

impl AssignmentRevision {
    pub(crate) const INITIAL: Self = Self(1);
    const MAX: u64 = i64::MAX as u64;

    pub fn value(self) -> u64 {
        self.0
    }

    pub(crate) fn next(self) -> Result<Self, StoreError> {
        self.0
            .checked_add(1)
            .filter(|value| *value <= Self::MAX)
            .map(Self)
            .ok_or_else(|| StoreError::Unavailable("assignment revision limit reached".to_string()))
    }

    #[cfg(feature = "postgres")]
    pub(crate) fn from_stored(value: i64) -> Result<Self, StoreError> {
        let value = u64::try_from(value).map_err(|_| {
            StoreError::Unavailable("stored assignment revision is invalid".to_string())
        })?;
        if value == 0 {
            return Err(StoreError::Unavailable(
                "stored assignment revision is invalid".to_string(),
            ));
        }
        Ok(Self(value))
    }
}

/// Editable draft plus its server-managed revision token.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceDraft {
    pub record: DraftRecord,
    pub revision: WorkspaceDraftRevision,
}

/// Shared immutable published content.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PublishedProblemRecord {
    /// Stable published problem.
    pub problem: ProblemId,
    /// Copyable human-facing identity of the stable problem.
    pub public_id: question_model::ProblemPublicId,
    /// Exact immutable version.
    pub version: VersionId,
    /// One-based human-facing version within the stable problem.
    pub version_number: question_model::ProblemVersionNumber,
    /// Browser-safe definition whose IDs match this record.
    pub question: QuestionDefinition,
    /// Capabilities declared by the owning adapter at publication time.
    pub capabilities: BackendCapabilities,
    /// Institution-only or cross-tenant catalog visibility.
    pub scope: PublicationScope,
    /// Discoverability and new-assignment state.
    pub lifecycle: CatalogLifecycle,
    /// Ordered, nonempty owners of this problem's linear version chain.
    pub authors: Vec<UserId>,
    /// Earlier version in the same problem chain.
    pub previous_version: Option<VersionId>,
    /// Original source when this problem is a fork.
    pub derived_from: Option<ProblemVersionRef>,
    /// Backend-authoritative time at which this version became immutable.
    pub published_at: ActivityTimestamp,
}

impl PublishedProblemRecord {
    /// Builds the hot browse projection without loading another representation.
    pub fn summary(&self) -> CatalogProblemSummary {
        CatalogProblemSummary {
            problem: self.problem,
            public_id: self.public_id,
            version: self.version,
            version_number: self.version_number,
            backend: QuestionBackend::from(&self.question.source),
            capabilities: self.capabilities.clone(),
            metadata: self.question.metadata.clone(),
            scope: self.scope,
            lifecycle: self.lifecycle.clone(),
            authors: self.authors.clone(),
            previous_version: self.previous_version,
            derived_from: self.derived_from,
            published_at: self.published_at,
        }
    }
}

/// Atomic publication of the exact draft that passed API validation.
#[derive(Debug, Clone, PartialEq)]
pub struct PublishDraftCommand {
    /// Exact draft value validated before entering storage.
    pub expected_draft: DraftRecord,
    /// Exact saved workspace revision the author reviewed before publication.
    ///
    /// Storage compares this under the same transaction lock that consumes the
    /// draft, so equivalent content saved through another tab cannot make an
    /// old review valid again.
    pub expected_revision: WorkspaceDraftRevision,
    /// Complete durable identity minted after the draft is validated.
    pub publication: ProblemVersionRef,
    /// Server-prepared immutable source. iMathAS reaches this field only after
    /// the source snapshot and supported integration profile are verified.
    pub published_source: question_model::QuestionSource,
    /// Server-prepared immutable original or snapshot for source-backed
    /// backends. Native questions intentionally have no source artifact.
    pub source_artifact: Option<PublishedSourceArtifact>,
    /// Present only for a server-prepared QTI publication.
    pub qti_promotion: Option<QtiPublicationPromotion>,
    /// Present only for a server-prepared flat-question publication.
    pub flat_question_promotion: Option<FlatQuestionPublicationPromotion>,
    /// Authenticated author performing the transition.
    pub publisher: UserId,
    /// Institution-only or public target.
    pub scope: PublicationScope,
    /// Trusted capabilities resolved from the server adapter registry.
    pub capabilities: BackendCapabilities,
}

/// Allowed post-publication lifecycle changes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CatalogTransition {
    /// Hide a version from browsing and new assignments.
    Deprecate {
        /// Required author explanation.
        reason: String,
    },
    /// Move an already deprecated version to historical status.
    Archive,
}

/// Tenant-owned course, including its explicit course-local access list.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CourseRecord {
    /// Durable course identity.
    pub id: CourseId,
    /// Direct RLS boundary.
    pub tenant: TenantId,
    /// Human-facing course or section title.
    pub title: String,
    /// Explicit course-local permissions keyed by authenticated user identity.
    pub members: Vec<CourseMembership>,
}

impl CourseRecord {
    /// Returns the browser projection for one direct member or tenant administrator.
    pub fn summary(&self, role: CourseRole) -> CourseSummary {
        CourseSummary {
            id: self.id,
            tenant: self.tenant,
            title: self.title.clone(),
            role,
        }
    }

    /// Resolves one authenticated user's explicit course role.
    pub fn role_for(&self, user: UserId) -> Option<CourseRole> {
        self.members
            .iter()
            .find(|membership| membership.user == user)
            .map(|membership| membership.role.into())
    }
}

/// Explicit scope for course-list authorization.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CourseListScope {
    /// Return only courses carrying a direct membership for this user.
    Member(UserId),
    /// Return every course in the tenant under coarse administrator authority.
    TenantAdministrator,
}

/// Tenant-owned assignment that references shared immutable content.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssignmentRecord {
    /// Durable assignment identity.
    pub id: AssignmentId,
    /// Direct RLS boundary.
    pub tenant: TenantId,
    /// Tenant-owned course containing the assignment.
    pub course_id: CourseId,
    /// Human-facing assignment title.
    pub title: String,
    /// Stable ordered fixed items selected for the assignment.
    pub items: Vec<question_model::AssignmentItem>,
    /// Random-selection groups with pinned immutable candidates.
    pub selection_groups: Vec<question_model::AssignmentSelectionGroup>,
    /// Four independent run policies.
    pub policies: RunPolicies,
}

/// Editable assignment definition together with its server-managed revision.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoredAssignment {
    pub record: AssignmentRecord,
    pub revision: AssignmentRevision,
    /// Generation matched by current computed score rows.
    pub scoring_generation: ScoringGeneration,
    /// Whether scores for this generation may be presented.
    pub scoring_status: ScoringStatus,
}

/// Editable assignment fields supplied after the server has bound identity and
/// course ownership from the authenticated route.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssignmentUpdate {
    pub title: String,
    pub items: Vec<question_model::AssignmentItem>,
    pub selection_groups: Vec<question_model::AssignmentSelectionGroup>,
    pub policies: RunPolicies,
}

/// Current timing policy paired with the assignment's shared revision token.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StoredAssignmentTiming {
    pub tenant: TenantId,
    pub course: CourseId,
    pub assignment: AssignmentId,
    pub policy: AssignmentTimingPolicy,
    pub revision: AssignmentRevision,
}

/// Authorized current-state replacement for assignment timing and access.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UpdateAssignmentTimingCommand {
    pub actor: UserId,
    pub course: CourseId,
    pub assignment: AssignmentId,
    pub expected_revision: AssignmentRevision,
    pub policy: AssignmentTimingPolicy,
}

/// Revision-checked instructor command behind the Delete and Regrade action.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeleteAndRegradeAssignmentItemCommand {
    pub course: CourseId,
    pub assignment: AssignmentId,
    pub item: AssignmentItemId,
    pub expected_revision: AssignmentRevision,
}

/// Bounded client-generated key for replaying one submission safely.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SubmissionIdempotencyKey(String);

impl SubmissionIdempotencyKey {
    /// Parses one visible ASCII key without accepting whitespace or controls.
    pub fn parse(value: impl Into<String>) -> Result<Self, StoreError> {
        const MAX_KEY_BYTES: usize = 200;
        let value = value.into();
        if value.is_empty()
            || value.len() > MAX_KEY_BYTES
            || !value.is_ascii()
            || value
                .bytes()
                .any(|byte| byte.is_ascii_whitespace() || byte.is_ascii_control())
        {
            return Err(StoreError::InvalidRecord(
                "idempotency key must contain 1 to 200 visible ASCII characters".to_string(),
            ));
        }
        Ok(Self(value))
    }

    /// Returns the validated header value.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Server-owned data needed to issue or resume one question instance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IssueQuestionAttemptCommand {
    /// Authenticated enrollment owner.
    pub actor: UserId,
    /// Fresh proposed identity; ignored when an active instance already exists.
    pub attempt: QuestionAttemptId,
    /// Active run receiving the question.
    pub run: RunId,
    /// Zero-based logical assignment position.
    pub assignment_position: u32,
    /// Exact immutable content identity at that position.
    pub problem: ProblemId,
    /// Exact immutable version at that position.
    pub question_version: VersionId,
    /// Fresh operating-system-random seed for the proposed instance.
    pub seed: u64,
    /// Hash of the generated parameters.
    pub parameter_hash: String,
    /// Adapter, generator, renderer, source, asset, and grading provenance.
    pub provenance: AttemptProvenance,
    /// Server-owned candidate prepared while the preceding attempt was active.
    /// It is verified and consumed atomically with issuance; browser input can
    /// never create this internal command.
    pub prefetched: Option<PrefetchedQuestion>,
    /// Committed predecessor whose immutable receipt is finalized by this
    /// issuance. This link is written in the same transaction as the attempt.
    pub predecessor_submission: Option<QuestionAttemptId>,
}

/// Immutable successor state for one committed submission.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubmissionNextAttempt {
    /// Finalization has not yet run (for crash healing of older writes).
    Pending,
    /// This submission completed or exhausted the run without another attempt.
    None,
    /// Exact next attempt issued from this submission.
    Issued(QuestionAttemptId),
}

/// Key-free, tenant-owned preparation for a possible next question.
///
/// This intentionally has neither an attempt identity nor a timer. It cannot
/// receive a response, grade, or summary transition; only matching post-submit
/// issuance may consume it into a real [`QuestionAttempt`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PrefetchedQuestion {
    pub tenant: TenantId,
    pub run: RunId,
    pub predecessor: QuestionAttemptId,
    pub assignment_position: u32,
    pub problem: ProblemId,
    pub question_version: VersionId,
    pub seed: u64,
    pub parameter_hash: String,
    pub provenance: AttemptProvenance,
}

/// Trusted server request to create or resume a prefetch reservation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReservePrefetchedQuestionCommand {
    pub actor: UserId,
    pub reservation: PrefetchedQuestion,
}

/// Trusted server result to persist for one student response.
#[derive(Clone, PartialEq)]
pub struct SubmitQuestionAttemptCommand {
    /// Authenticated enrollment owner.
    pub actor: UserId,
    /// Issued question being answered.
    pub attempt: QuestionAttemptId,
    /// Student-controlled response already validated and server-graded.
    pub response: StudentResponse,
    /// Key-free grading result produced inside the server boundary.
    pub result: AttemptResult,
    /// Trusted, sanitized teaching material captured with the first grade.
    ///
    /// This remains server-only: it is not a response DTO and is never
    /// serialized by the public model generator.
    pub feedback: FeedbackContent,
    /// Stable key reused by browser retries of this exact response.
    pub idempotency_key: SubmissionIdempotencyKey,
}

impl std::fmt::Debug for SubmitQuestionAttemptCommand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SubmitQuestionAttemptCommand")
            .field("actor", &self.actor)
            .field("attempt", &self.attempt)
            .field("response", &self.response)
            .field("result", &self.result)
            .field("idempotency_key", &self.idempotency_key)
            .field("feedback", &"[redacted]")
            .finish()
    }
}

/// Stable idempotency and audit identity for one instructor support action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AttemptSupportActionId(Uuid);

impl AttemptSupportActionId {
    /// Wraps an identity read from storage or a trusted server boundary.
    pub fn from_uuid(value: Uuid) -> Self {
        Self(value)
    }

    /// Returns the UUID persisted with the audit event.
    pub fn as_uuid(self) -> Uuid {
        self.0
    }

    /// Mints one server-owned action identity.
    pub fn generate() -> Result<Self, StoreError> {
        let mut bytes = [0_u8; 16];
        getrandom::fill(&mut bytes).map_err(|error| {
            StoreError::Unavailable(format!(
                "attempt support action ID randomness unavailable: {error}"
            ))
        })?;
        Ok(Self(Uuid::from_bytes(bytes)))
    }
}

/// Closed set of sensitive attempt-support mutations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AttemptSupportAction {
    /// Close an active question without fabricating a response or grade.
    ForceSubmit,
    /// Exclude an attempt from current scoring while retaining its evidence.
    Clear,
}

impl AttemptSupportAction {
    #[cfg(feature = "postgres")]
    fn audit_name(self) -> &'static str {
        match self {
            Self::ForceSubmit => "attempt.force_submit",
            Self::Clear => "attempt.clear",
        }
    }
}

/// Idempotent instructor request to close one still-active question attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ForceSubmitAttemptCommand {
    pub action: AttemptSupportActionId,
    pub actor: UserId,
    pub attempt: QuestionAttemptId,
}

/// Idempotent instructor request to remove one attempt from current scoring.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClearAttemptCommand {
    pub action: AttemptSupportActionId,
    pub actor: UserId,
    pub attempt: QuestionAttemptId,
}

/// Minimal retained evidence for one instructor attempt-support action.
///
/// No response, evaluation, score, student identity, or obsolete grade is
/// copied into this record. The protected attempt remains the evidence owner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AttemptSupportRecord {
    pub tenant: TenantId,
    pub action: AttemptSupportActionId,
    pub actor: UserId,
    pub attempt: QuestionAttemptId,
    pub kind: AttemptSupportAction,
    pub previous_status: AttemptStatus,
    pub resulting_status: AttemptStatus,
    pub occurred_at: ActivityTimestamp,
}

/// First committed submission result or an exact idempotent replay of it.
#[derive(Clone, PartialEq)]
pub struct SubmissionRecord {
    /// Browser-safe attempt projection with response and disclosed result data.
    pub attempt: QuestionAttempt,
    /// Run after any completion derived by this submission.
    pub run: AssignmentRun,
    /// Compact projection updated in the same transaction as the submission.
    pub summary: StudentAssignmentSummary,
    /// Private, immutable teaching content retained for policy-controlled
    /// disclosure. This is intentionally not browser-safe data.
    pub feedback: AttemptFeedbackRecord,
}

impl std::fmt::Debug for SubmissionRecord {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SubmissionRecord")
            .field("attempt", &self.attempt)
            .field("run", &self.run)
            .field("summary", &self.summary)
            .field("feedback", &"[redacted]")
            .finish()
    }
}

impl AssignmentRecord {
    /// Every pinned immutable reference in the current assignment definition.
    pub fn references(&self) -> impl Iterator<Item = ProblemVersionRef> + '_ {
        self.items.iter().map(|item| item.reference).chain(
            self.selection_groups
                .iter()
                .flat_map(|group| group.candidates.iter().map(|candidate| candidate.reference)),
        )
    }

    /// Active fixed items in current future-run order.
    pub fn active_items(&self) -> impl Iterator<Item = &question_model::AssignmentItem> {
        self.items
            .iter()
            .filter(|item| item.delivery_state == question_model::AssignmentDeliveryState::Active)
    }

    /// Immutable content that may be delivered by a future run.
    pub fn active_references(&self) -> impl Iterator<Item = ProblemVersionRef> + '_ {
        self.active_items()
            .map(|item| item.reference)
            .chain(self.selection_groups.iter().flat_map(|group| {
                group.candidates.iter().filter_map(|candidate| {
                    (candidate.delivery_state == question_model::AssignmentDeliveryState::Active)
                        .then_some(candidate.reference)
                })
            }))
    }

    /// Resolves one active fixed item by its future-run position.
    pub fn active_item_at(&self, position: u32) -> Option<&question_model::AssignmentItem> {
        self.active_items().find(|item| item.position == position)
    }

    /// Builds the browser-safe assignment projection.
    pub fn summary(&self) -> AssignmentSummary {
        AssignmentSummary {
            id: self.id,
            tenant: self.tenant,
            course_id: self.course_id,
            title: self.title.clone(),
            items: self.items.clone(),
            selection_groups: self.selection_groups.clone(),
            policies: self.policies,
        }
    }
}

/// Freezes current fixed items and deterministic group selections for one new run.
pub(crate) fn select_assignment_run_items(
    assignment: &AssignmentRecord,
    run: RunId,
) -> Result<Vec<AssignmentRunItem>, StoreError> {
    enum Source<'a> {
        Fixed(&'a AssignmentItem),
        Group(&'a AssignmentSelectionGroup),
    }
    let mut sources = assignment
        .active_items()
        .map(|item| (item.position, Source::Fixed(item)))
        .chain(
            assignment
                .selection_groups
                .iter()
                .map(|group| (group.position, Source::Group(group))),
        )
        .collect::<Vec<_>>();
    sources.sort_by_key(|(position, _)| *position);
    let mut selected = Vec::new();
    for (source_position, source) in sources {
        match source {
            Source::Fixed(item) => {
                selected.push((item.id, source_position, item.reference, None, None))
            }
            Source::Group(group) => {
                let seed = assignment_selection_seed(run, group);
                let mut candidates = group
                    .candidates
                    .iter()
                    .filter(|candidate| candidate.delivery_state == AssignmentDeliveryState::Active)
                    .map(|candidate| (assignment_selection_rank(seed, candidate.id), candidate))
                    .collect::<Vec<_>>();
                candidates.sort_by_key(|(rank, candidate)| (*rank, candidate.id));
                candidates.truncate(usize::try_from(group.draw_count).map_err(|_| {
                    StoreError::InvalidRecord("selection draw count is too large".to_string())
                })?);
                if group.ordering == SelectionOrdering::CandidateOrder {
                    candidates.sort_by_key(|(_, candidate)| (candidate.position, candidate.id));
                }
                for (_, candidate) in candidates {
                    selected.push((
                        candidate.id,
                        source_position,
                        candidate.reference,
                        Some(group.id),
                        Some(seed),
                    ));
                }
            }
        }
    }
    selected
        .into_iter()
        .enumerate()
        .map(
            |(
                issued_position,
                (assignment_item, source_position, reference, selection_group, selection_seed),
            )| {
                Ok(AssignmentRunItem {
                    run,
                    assignment_item,
                    source_position,
                    issued_position: u32::try_from(issued_position).map_err(|_| {
                        StoreError::InvalidRecord("too many selected run items".to_string())
                    })?,
                    reference,
                    selection_group,
                    selection_seed,
                })
            },
        )
        .collect()
}

fn assignment_selection_seed(run: RunId, group: &AssignmentSelectionGroup) -> u64 {
    let mut bytes = Vec::with_capacity(34);
    bytes.extend_from_slice(run.as_uuid().as_bytes());
    bytes.extend_from_slice(group.id.as_uuid().as_bytes());
    bytes.extend_from_slice(&group.algorithm_version.to_be_bytes());
    let digest = Sha256Digest::compute(&bytes);
    let mut seed = [0_u8; 8];
    seed.copy_from_slice(&digest.as_bytes()[..8]);
    u64::from_be_bytes(seed) & 9_007_199_254_740_991
}

fn assignment_selection_rank(seed: u64, candidate: AssignmentItemId) -> u64 {
    let mut bytes = Vec::with_capacity(24);
    bytes.extend_from_slice(&seed.to_be_bytes());
    bytes.extend_from_slice(candidate.as_uuid().as_bytes());
    let digest = Sha256Digest::compute(&bytes);
    let mut rank = [0_u8; 8];
    rank.copy_from_slice(&digest.as_bytes()[..8]);
    u64::from_be_bytes(rank)
}

fn delete_and_regrade_update(
    stored: &StoredAssignment,
    target: AssignmentItemId,
) -> Result<Option<AssignmentUpdate>, StoreError> {
    let mut update = AssignmentUpdate {
        title: stored.record.title.clone(),
        items: stored.record.items.clone(),
        selection_groups: stored.record.selection_groups.clone(),
        policies: stored.record.policies,
    };
    if let Some(item) = update.items.iter_mut().find(|item| item.id == target) {
        if item.delivery_state == AssignmentDeliveryState::Retired
            && item.scoring_mode == question_model::AssignmentScoringMode::Excluded
        {
            return Ok(None);
        }
        item.delivery_state = AssignmentDeliveryState::Retired;
        item.scoring_mode = question_model::AssignmentScoringMode::Excluded;
        return Ok(Some(update));
    }
    if let Some(candidate) = update
        .selection_groups
        .iter_mut()
        .flat_map(|group| group.candidates.iter_mut())
        .find(|candidate| candidate.id == target)
    {
        if candidate.delivery_state == AssignmentDeliveryState::Retired {
            return Ok(None);
        }
        candidate.delivery_state = AssignmentDeliveryState::Retired;
        return Ok(Some(update));
    }
    Err(StoreError::NotFound)
}

pub(crate) fn assignment_scoring_changed(
    previous: &AssignmentRecord,
    replacement: &AssignmentRecord,
) -> bool {
    let fixed = |assignment: &AssignmentRecord| {
        assignment
            .items
            .iter()
            .map(|item| {
                (
                    item.id,
                    (item.points_possible, item.delivery_state, item.scoring_mode),
                )
            })
            .collect::<std::collections::BTreeMap<_, _>>()
    };
    let groups = |assignment: &AssignmentRecord| {
        assignment
            .selection_groups
            .iter()
            .map(|group| {
                (
                    group.id,
                    (
                        group.points_per_item,
                        group
                            .candidates
                            .iter()
                            .map(|candidate| (candidate.id, candidate.delivery_state))
                            .collect::<std::collections::BTreeMap<_, _>>(),
                    ),
                )
            })
            .collect::<std::collections::BTreeMap<_, _>>()
    };
    previous.policies.completion != replacement.policies.completion
        || previous.policies.grade != replacement.policies.grade
        || fixed(previous) != fixed(replacement)
        || groups(previous) != groups(replacement)
}

/// Applies current assignment scoring to one normalized backend result.
pub(crate) fn current_attempt_points(
    assignment: &AssignmentRecord,
    assignment_item: AssignmentItemId,
    status: AttemptStatus,
    result: AttemptResult,
) -> Result<(f64, f64), StoreError> {
    validate_attempt_result(result)?;
    if matches!(status, AttemptStatus::Cleared | AttemptStatus::Exempt) {
        return Ok((0.0, 0.0));
    }
    let (points, mode) = if let Some(item) = assignment
        .items
        .iter()
        .find(|item| item.id == assignment_item)
    {
        (item.points_possible, item.scoring_mode)
    } else if let Some(group) = assignment.selection_groups.iter().find(|group| {
        group
            .candidates
            .iter()
            .any(|candidate| candidate.id == assignment_item)
    }) {
        let candidate = group
            .candidates
            .iter()
            .find(|candidate| candidate.id == assignment_item)
            .expect("selection group was found through this candidate");
        (
            group.points_per_item,
            if candidate.delivery_state == AssignmentDeliveryState::Retired {
                question_model::AssignmentScoringMode::Excluded
            } else {
                question_model::AssignmentScoringMode::Normal
            },
        )
    } else {
        return Err(StoreError::InvalidRecord(
            "run item no longer resolves to a current scoring definition".to_string(),
        ));
    };
    let credit = result.points_earned / result.points_possible;
    let possible_points = points.scaled() as f64 / 10_000.0;
    let (earned, possible) = match mode {
        question_model::AssignmentScoringMode::Normal => {
            (credit * possible_points, possible_points)
        }
        question_model::AssignmentScoringMode::FullCredit => (possible_points, possible_points),
        question_model::AssignmentScoringMode::ExtraCredit => (credit * possible_points, 0.0),
        question_model::AssignmentScoringMode::Excluded => (0.0, 0.0),
    };
    Ok((
        score_precision::round_for_persistence(earned),
        score_precision::round_for_persistence(possible),
    ))
}

pub(crate) fn assignment_item_is_retired(
    assignment: &AssignmentRecord,
    assignment_item: AssignmentItemId,
) -> Option<bool> {
    assignment
        .items
        .iter()
        .find(|item| item.id == assignment_item)
        .map(|item| item.delivery_state == AssignmentDeliveryState::Retired)
        .or_else(|| {
            assignment
                .selection_groups
                .iter()
                .flat_map(|group| group.candidates.iter())
                .find(|candidate| candidate.id == assignment_item)
                .map(|candidate| candidate.delivery_state == AssignmentDeliveryState::Retired)
        })
}

/// Publishes the first completion and replaces computed score fields and run pointers.
pub(crate) fn recalculated_enrollment_projection(
    mut enrollment: AssignmentEnrollment,
    mut summary: StudentAssignmentSummary,
    grade_policy: GradePolicy,
    mut completed_runs: Vec<domain::scoring::CompletedRunScore>,
    first_completed_at: Option<question_model::ActivityTimestamp>,
) -> Result<(AssignmentEnrollment, StudentAssignmentSummary), StoreError> {
    completed_runs.sort_by_key(|run| run.run_number);
    let selected = domain::scoring::score(
        &completed_runs,
        grade_policy,
        (grade_policy == GradePolicy::InstructorSelected)
            .then_some(enrollment.current_grade_run)
            .flatten(),
    )
    .map_err(|error| StoreError::InvalidRecord(error.to_string()))?;
    let best = domain::scoring::score(&completed_runs, GradePolicy::Highest, None)
        .map_err(|error| StoreError::InvalidRecord(error.to_string()))?;
    summary.completed_run_count = u32::try_from(completed_runs.len())
        .map_err(|_| StoreError::InvalidRecord("too many completed runs".to_string()))?;
    summary.latest_score = completed_runs.last().map(|run| run.score);
    summary.best_score = best.map(|selection| selection.score);
    summary.current_score = selected.map(|selection| selection.score);
    if enrollment.first_completed_at.is_none() {
        enrollment.first_completed_at = first_completed_at;
    }
    enrollment.best_grade_run = best.map(|selection| selection.run);
    enrollment.current_grade_run = selected.map(|selection| selection.run);
    Ok((enrollment, summary))
}

#[cfg(test)]
mod assignment_selection_tests {
    use super::*;
    use question_model::{
        AssignmentScoringMode, AssignmentSelectionCandidate, AttemptTimerRecord,
        ImplementationVersion, PointValue, ProblemVersionRef,
    };

    fn id(value: u128) -> Uuid {
        Uuid::from_u128(value)
    }

    #[test]
    fn run_selection_is_reproducible_and_freezes_expanded_order() {
        let reference = |value| ProblemVersionRef {
            problem: ProblemId::from_uuid(id(10 + value)),
            version: VersionId::from_uuid(id(20 + value)),
        };
        let assignment = AssignmentRecord {
            id: AssignmentId::from_uuid(id(1)),
            tenant: TenantId::from_uuid(id(2)),
            course_id: CourseId::from_uuid(id(3)),
            title: "Selection fixture".to_string(),
            items: vec![AssignmentItem {
                id: AssignmentItemId::from_uuid(id(30)),
                reference: reference(0),
                position: 0,
                points_possible: PointValue::from_whole(1),
                delivery_state: AssignmentDeliveryState::Active,
                scoring_mode: AssignmentScoringMode::Normal,
            }],
            selection_groups: vec![AssignmentSelectionGroup {
                id: question_model::AssignmentSelectionGroupId::from_uuid(id(31)),
                position: 1,
                draw_count: 2,
                points_per_item: PointValue::from_whole(2),
                ordering: SelectionOrdering::Randomized,
                algorithm_version: 1,
                candidates: (1..=4)
                    .map(|value| AssignmentSelectionCandidate {
                        id: AssignmentItemId::from_uuid(id(40 + value)),
                        position: u32::try_from(value - 1).expect("fixture position"),
                        reference: reference(value),
                        delivery_state: if value == 4 {
                            AssignmentDeliveryState::Retired
                        } else {
                            AssignmentDeliveryState::Active
                        },
                    })
                    .collect(),
            }],
            policies: RunPolicies {
                completion: question_model::CompletionRequirement::AnswerAll,
                grade: GradePolicy::Highest,
                continued_practice: question_model::ContinuedPractice::Unlimited,
                variation: question_model::VariationPolicy::NewSeeds,
            },
        };
        let run = RunId::from_uuid(id(100));
        let first = select_assignment_run_items(&assignment, run).expect("valid selection");
        let replay = select_assignment_run_items(&assignment, run).expect("repeat selection");

        assert_eq!(first, replay);
        assert_eq!(first.len(), 3);
        assert_eq!(
            first
                .iter()
                .map(|item| item.issued_position)
                .collect::<Vec<_>>(),
            vec![0, 1, 2]
        );
        assert!(first[0].selection_group.is_none());
        assert!(first[1..].iter().all(|item| item.selection_seed.is_some()));
        assert!(
            first
                .iter()
                .all(|item| item.assignment_item != AssignmentItemId::from_uuid(id(44)))
        );
        let next = select_assignment_run_items(&assignment, RunId::from_uuid(id(101)))
            .expect("next run selection");
        assert_ne!(first[1].selection_seed, next[1].selection_seed);
    }

    #[test]
    fn current_attempt_points_apply_every_scoring_mode_and_attempt_exclusion() {
        let reference = ProblemVersionRef {
            problem: ProblemId::from_uuid(id(200)),
            version: VersionId::from_uuid(id(201)),
        };
        let modes = [
            AssignmentScoringMode::Normal,
            AssignmentScoringMode::FullCredit,
            AssignmentScoringMode::ExtraCredit,
            AssignmentScoringMode::Excluded,
        ];
        let assignment = AssignmentRecord {
            id: AssignmentId::from_uuid(id(202)),
            tenant: TenantId::from_uuid(id(203)),
            course_id: CourseId::from_uuid(id(204)),
            title: "Scoring modes".to_string(),
            items: modes
                .into_iter()
                .enumerate()
                .map(|(position, scoring_mode)| AssignmentItem {
                    id: AssignmentItemId::from_uuid(id(210 + position as u128)),
                    reference,
                    position: u32::try_from(position).expect("fixture position"),
                    points_possible: PointValue::from_whole(2),
                    delivery_state: AssignmentDeliveryState::Active,
                    scoring_mode,
                })
                .collect(),
            selection_groups: Vec::new(),
            policies: RunPolicies {
                completion: question_model::CompletionRequirement::AnswerAll,
                grade: GradePolicy::Highest,
                continued_practice: question_model::ContinuedPractice::Unlimited,
                variation: question_model::VariationPolicy::NewSeeds,
            },
        };
        let result = |credit: f64| AttemptResult {
            correct: credit == 1.0,
            points_earned: credit,
            points_possible: 1.0,
        };

        assert_eq!(
            current_attempt_points(
                &assignment,
                assignment.items[0].id,
                AttemptStatus::Submitted,
                result(-0.5),
            ),
            Ok((-1.0, 2.0)),
            "normal scoring retains negative credit"
        );
        assert_eq!(
            current_attempt_points(
                &assignment,
                assignment.items[1].id,
                AttemptStatus::Submitted,
                result(-0.5),
            ),
            Ok((2.0, 2.0)),
            "full credit ignores the normalized result"
        );
        assert_eq!(
            current_attempt_points(
                &assignment,
                assignment.items[2].id,
                AttemptStatus::Submitted,
                result(1.25),
            ),
            Ok((2.5, 0.0)),
            "extra credit changes only the numerator"
        );
        assert_eq!(
            current_attempt_points(
                &assignment,
                assignment.items[3].id,
                AttemptStatus::Submitted,
                result(1.0),
            ),
            Ok((0.0, 0.0)),
            "excluded items change neither numerator nor denominator"
        );
        assert_eq!(
            current_attempt_points(
                &assignment,
                assignment.items[0].id,
                AttemptStatus::Cleared,
                result(1.0),
            ),
            Ok((0.0, 0.0)),
            "cleared attempts are absent from current scoring"
        );
        assert_eq!(
            current_attempt_points(
                &assignment,
                assignment.items[0].id,
                AttemptStatus::Submitted,
                result(4.000_000_000_000_3),
            ),
            Ok((8.0, 2.0)),
            "computed points are rounded before persistence"
        );
    }

    #[test]
    fn completed_run_score_is_rounded_before_persistence() {
        let questions = vec![Some(CurrentRunQuestion {
            assignment_item: AssignmentItemId::from_uuid(id(250)),
            result: AttemptResult {
                correct: false,
                points_earned: 1.0,
                points_possible: 3.0,
            },
            earned_points: 1.0,
            possible_points: 3.0,
        })];

        assert_eq!(
            completed_run_score(&questions, question_model::CompletionRequirement::AnswerAll),
            Ok(Some(0.3333))
        );
    }

    #[test]
    fn selected_group_items_complete_from_the_immutable_delivered_order() {
        let tenant = TenantId::from_uuid(id(300));
        let run = RunId::from_uuid(id(301));
        let reference = |value| ProblemVersionRef {
            problem: ProblemId::from_uuid(id(310 + value)),
            version: VersionId::from_uuid(id(320 + value)),
        };
        let assignment = AssignmentRecord {
            id: AssignmentId::from_uuid(id(302)),
            tenant,
            course_id: CourseId::from_uuid(id(303)),
            title: "Selected completion".to_string(),
            items: Vec::new(),
            selection_groups: vec![AssignmentSelectionGroup {
                id: question_model::AssignmentSelectionGroupId::from_uuid(id(304)),
                position: 0,
                draw_count: 2,
                points_per_item: PointValue::from_whole(2),
                ordering: SelectionOrdering::CandidateOrder,
                algorithm_version: 1,
                candidates: (0..2)
                    .map(|position| AssignmentSelectionCandidate {
                        id: AssignmentItemId::from_uuid(id(330 + u128::from(position))),
                        position,
                        reference: reference(u128::from(position)),
                        delivery_state: AssignmentDeliveryState::Active,
                    })
                    .collect(),
            }],
            policies: RunPolicies {
                completion: question_model::CompletionRequirement::AnswerAll,
                grade: GradePolicy::Highest,
                continued_practice: question_model::ContinuedPractice::Unlimited,
                variation: question_model::VariationPolicy::NewSeeds,
            },
        };
        let run_items = select_assignment_run_items(&assignment, run).expect("selected run items");
        let attempts = run_items
            .iter()
            .enumerate()
            .map(|(index, item)| QuestionAttempt {
                id: QuestionAttemptId::from_uuid(id(340 + index as u128)),
                tenant,
                run,
                problem: item.reference.problem,
                question_version: item.reference.version,
                assignment_position: item.issued_position,
                seed: u64::try_from(index).expect("fixture seed"),
                parameter_hash: format!("selected-{index}"),
                response: Some(StudentResponse::Numeric { value: 1.0 }),
                status: AttemptStatus::Submitted,
                result: Some(AttemptResult {
                    correct: true,
                    points_earned: 1.0,
                    points_possible: 1.0,
                }),
                timer: AttemptTimerRecord {
                    issued_at: ActivityTimestamp::from_unix_millis(index as i64),
                    deadline: None,
                    submitted_at: Some(ActivityTimestamp::from_unix_millis(index as i64 + 1)),
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
                        id: "native".to_string(),
                        version: "1".to_string(),
                    },
                    rendered_question_sha256: format!("selected-render-{index}"),
                },
            })
            .collect::<Vec<_>>();
        let questions = current_run_questions(
            &assignment,
            &run_items,
            &attempts,
            attempts.last().expect("selected current attempt"),
        )
        .expect("selected questions resolve");

        assert_eq!(questions.len(), 2);
        assert_eq!(
            completed_run_score(&questions, question_model::CompletionRequirement::AnswerAll),
            Ok(Some(1.0))
        );
        assert!(questions.iter().all(|question| {
            question.is_some_and(|question| {
                question.earned_points == 2.0 && question.possible_points == 2.0
            })
        }));
    }
}

/// One atomic activity write applied with its compact summary projection.
#[derive(Debug, Clone, PartialEq)]
pub enum ActivityTransition {
    /// Creates a new incomplete run and records its start activity.
    StartRun {
        /// New run with server-supplied start time.
        run: AssignmentRun,
    },
    /// Appends one immutable question-attempt record.
    RecordQuestionAttempt {
        /// Attempt carrying response, result, timing, and reproducibility data.
        attempt: Box<QuestionAttempt>,
    },
    /// Completes an existing run and projects its score.
    CompleteRun {
        /// Existing run to complete.
        run: RunId,
        /// Final score fraction.
        score: f64,
        /// Authoritative PostgreSQL timestamp.
        at: ActivityTimestamp,
    },
}

/// Portable persistence failure with no SQL type in its variants.
#[derive(Debug, Clone, PartialEq)]
pub enum StoreError {
    /// Requested record is absent in the active tenant or shared catalog.
    NotFound,
    /// Immutable identity already exists.
    AlreadyExists,
    /// A tenant-owned record disagrees with authenticated context.
    TenantMismatch,
    /// Stored state changed after a caller validated its expected value.
    Conflict,
    /// PostgreSQL aborted the whole transaction due to a serialization or deadlock conflict.
    RetryableTransaction,
    /// Authenticated identity lacks ownership or role for the operation.
    Forbidden,
    /// Record shape violates a model invariant.
    InvalidRecord(String),
    /// Pure activity projection rejected the transition.
    RunModel(RunModelError),
    /// The database-authoritative timer no longer accepts this response.
    TimedOut,
    /// Backend state is temporarily unavailable.
    Unavailable(String),
}

impl std::fmt::Display for StoreError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound => write!(formatter, "record not found"),
            Self::AlreadyExists => write!(formatter, "immutable record already exists"),
            Self::TenantMismatch => write!(formatter, "record tenant does not match context"),
            Self::Conflict => write!(formatter, "record changed before the operation committed"),
            Self::RetryableTransaction => write!(formatter, "transaction must be retried"),
            Self::Forbidden => write!(formatter, "operation is not authorized"),
            Self::InvalidRecord(message) => write!(formatter, "invalid record: {message}"),
            Self::RunModel(error) => write!(formatter, "activity transition rejected: {error}"),
            Self::TimedOut => write!(formatter, "question attempt timed out"),
            Self::Unavailable(message) => write!(formatter, "store unavailable: {message}"),
        }
    }
}

impl std::error::Error for StoreError {}

impl From<RunModelError> for StoreError {
    fn from(error: RunModelError) -> Self {
        Self::RunModel(error)
    }
}

/// Catalog operations that require visibility, ownership, and atomic publish.
#[async_trait]
pub trait CatalogStore: Send + Sync {
    /// Validates the stored draft expectation and atomically publishes it.
    async fn publish_draft(
        &self,
        context: TenantContext,
        actor: UserId,
        command: PublishDraftCommand,
    ) -> Result<PublishedProblemRecord, StoreError>;

    /// Resolves an exact visible version, including deprecated or archived ones.
    async fn get_catalog_problem(
        &self,
        context: TenantContext,
        reference: ProblemVersionRef,
    ) -> Result<Option<PublishedProblemRecord>, StoreError>;

    /// Resolves a copyable catalog reference under the caller's visibility.
    /// A stable reference selects the latest assignable version; an exact
    /// reference never silently upgrades to another version.
    async fn resolve_catalog_problem(
        &self,
        context: TenantContext,
        reference: question_model::ProblemDisplayRef,
    ) -> Result<Option<PublishedProblemRecord>, StoreError> {
        let _ = (context, reference);
        Err(StoreError::Unavailable(
            "human catalog lookup is not implemented by this store".to_string(),
        ))
    }

    /// Lists discoverable hot metadata in stable cursor order.
    async fn list_catalog(
        &self,
        context: TenantContext,
        page: PageRequest,
    ) -> Result<Page<CatalogProblemSummary>, StoreError>;

    /// Lists distinct controlled taxonomy terms in stable cursor order.
    async fn list_catalog_taxonomy(
        &self,
        context: TenantContext,
        page: PageRequest,
    ) -> Result<Page<TaxonomyTerm>, StoreError>;

    /// Searches hot discoverable metadata and returns rows plus server-side
    /// facets from one normalized-query snapshot. Implementations must reject
    /// a cursor issued for a different normalized query and must never load
    /// `problem_version_payload` merely to browse or aggregate.
    async fn search_catalog(
        &self,
        context: TenantContext,
        query: CatalogSearchQuery,
    ) -> Result<CatalogSearchPage, StoreError> {
        let _ = (context, query);
        Err(StoreError::Unavailable(
            "catalog search is not implemented by this store".to_string(),
        ))
    }

    /// Returns a safe exact immutable catalog-detail projection. This default
    /// retains compatibility for focused test stores while production stores
    /// may use a hot metadata projection instead of loading source bindings.
    async fn get_catalog_detail(
        &self,
        context: TenantContext,
        reference: ProblemVersionRef,
    ) -> Result<Option<CatalogProblemDetail>, StoreError> {
        Ok(self
            .get_catalog_problem(context, reference)
            .await?
            .map(|record| CatalogProblemDetail {
                summary: record.summary(),
                prompt: record.question.prompt,
                statistics: question_model::CatalogStatisticsStatus::Unavailable,
            }))
    }

    /// Applies an author-owned, one-way post-publication transition.
    async fn transition_catalog_problem(
        &self,
        context: TenantContext,
        actor: UserId,
        reference: ProblemVersionRef,
        transition: CatalogTransition,
    ) -> Result<PublishedProblemRecord, StoreError>;
}

/// Private catalog bridge from an exact visible version to its source bytes.
///
/// This trait is intentionally not part of any browser DTO or public asset
/// delivery API. A foreign tenant receives `None` before an object store is
/// consulted, which keeps source-object existence tenant-isolated.
#[async_trait]
pub trait CatalogSourceStore: Send + Sync {
    /// Resolves the exact source binding for one visible immutable version.
    async fn catalog_source_artifact(
        &self,
        context: TenantContext,
        reference: ProblemVersionRef,
    ) -> Result<Option<PublishedSourceArtifact>, StoreError>;
}

/// Persistence operations consumed by catalog, course, run, and worker lanes.
#[async_trait]
pub trait Store: Send + Sync {
    /// Returns only the globally k-anonymous metrics visible for one catalog
    /// version.  This has no contribution-write counterpart: submission
    /// completion owns that server-only capability.
    async fn question_statistics(
        &self,
        _context: TenantContext,
        _reference: ProblemVersionRef,
    ) -> Result<QuestionStatisticsDisclosure, StoreError> {
        Ok(QuestionStatisticsDisclosure::Suppressed)
    }

    /// Creates or replaces a tenant-owned editable draft for an authorized actor.
    ///
    /// The first save must pass `None` and atomically establishes `actor` as
    /// owner. Later saves require the exact revision returned by a prior read
    /// or save, so a stale browser tab cannot overwrite newer author work.
    async fn upsert_draft(
        &self,
        context: TenantContext,
        actor: UserId,
        expected_revision: Option<WorkspaceDraftRevision>,
        draft: DraftRecord,
    ) -> Result<WorkspaceDraft, StoreError>;

    /// Reads a draft only when `actor` has an explicit workspace binding.
    async fn get_draft(
        &self,
        context: TenantContext,
        actor: UserId,
        workspace: WorkspaceId,
    ) -> Result<Option<WorkspaceDraft>, StoreError>;

    /// Lists compact private workspace-draft summaries visible to `actor` in
    /// tenant-bound cursor order.
    async fn list_drafts(
        &self,
        context: TenantContext,
        actor: UserId,
        page: PageRequest,
    ) -> Result<Page<WorkspaceDraftSummary>, StoreError>;

    /// Removes only one unversioned draft in the active tenant.
    ///
    /// The caller supplies the revision obtained from a successful read or
    /// save.  The implementation compares that revision and verifies owner
    /// authority in the same removal operation, so a stale tab cannot delete
    /// newer author work.
    ///
    /// `false` deliberately covers an absent or foreign-tenant workspace, so
    /// callers do not gain an existence oracle through deletion.
    async fn delete_draft(
        &self,
        context: TenantContext,
        actor: UserId,
        workspace: WorkspaceId,
        expected_revision: WorkspaceDraftRevision,
    ) -> Result<bool, StoreError>;

    /// Adds a collaborator to an existing workspace.
    ///
    /// Only the persisted owner may grant this access. Repeating the same
    /// grant is idempotent so an interrupted invitation retry is harmless.
    async fn grant_draft_collaborator(
        &self,
        context: TenantContext,
        actor: UserId,
        workspace: WorkspaceId,
        collaborator: UserId,
    ) -> Result<(), StoreError>;

    /// Resolves one exact globally public version.
    ///
    /// Tenant-visible institution content is intentionally absent; use
    /// [`CatalogStore::get_catalog_problem`] when a session tenant is known.
    async fn get_published_problem(
        &self,
        problem: ProblemId,
        version: VersionId,
    ) -> Result<Option<PublishedProblemRecord>, StoreError>;

    /// Lists globally public, discoverable versions in stable cursor order.
    ///
    /// Tenant-visible institution content is intentionally absent; use
    /// [`CatalogStore::list_catalog`] when a session tenant is known.
    async fn list_published_problems(
        &self,
        page: PageRequest,
    ) -> Result<Page<PublishedProblemRecord>, StoreError>;

    /// Creates or replaces a tenant-owned course and its explicit memberships.
    async fn upsert_course(
        &self,
        context: TenantContext,
        course: CourseRecord,
    ) -> Result<(), StoreError>;

    /// Reads one course inside the active tenant for authorization checks.
    async fn get_course(
        &self,
        context: TenantContext,
        course: CourseId,
    ) -> Result<Option<CourseRecord>, StoreError>;

    /// Lists courses visible to a member or tenant administrator.
    async fn list_courses(
        &self,
        context: TenantContext,
        scope: CourseListScope,
        page: PageRequest,
    ) -> Result<Page<CourseSummary>, StoreError>;

    /// Creates or conditionally replaces one instructor-owned course group.
    /// Membership edits immediately re-resolve active attempts for every
    /// assignment exception that targets this group.
    async fn put_course_group(
        &self,
        context: TenantContext,
        command: PutCourseGroupCommand,
    ) -> Result<StoredCourseGroup, StoreError>;

    /// Reads one current course group inside the active tenant.
    async fn get_course_group(
        &self,
        context: TenantContext,
        group: CourseGroupId,
    ) -> Result<Option<StoredCourseGroup>, StoreError>;

    /// Creates a new assignment with an initial strong revision token.
    async fn create_assignment(
        &self,
        context: TenantContext,
        assignment: AssignmentRecord,
    ) -> Result<StoredAssignment, StoreError>;

    /// Replaces one assignment only when the caller holds its exact revision.
    async fn replace_assignment(
        &self,
        context: TenantContext,
        course: CourseId,
        assignment: AssignmentId,
        expected_revision: AssignmentRevision,
        update: AssignmentUpdate,
    ) -> Result<StoredAssignment, StoreError>;

    /// Reads the mutable access/time-limit policy and shared assignment revision.
    async fn get_assignment_timing(
        &self,
        context: TenantContext,
        assignment: AssignmentId,
    ) -> Result<Option<StoredAssignmentTiming>, StoreError>;

    /// Replaces current timing under instructor authority and immediately
    /// re-resolves every active attempt. A newly elapsed deadline is submitted
    /// in this transaction; an extension reschedules its durable job.
    async fn update_assignment_timing(
        &self,
        context: TenantContext,
        command: UpdateAssignmentTimingCommand,
    ) -> Result<StoredAssignmentTiming, StoreError>;

    /// Creates or replaces one target's current accommodation under the
    /// assignment revision and immediately re-resolves affected active work.
    async fn set_assignment_policy_exception(
        &self,
        context: TenantContext,
        command: SetAssignmentPolicyExceptionCommand,
    ) -> Result<StoredAssignmentPolicyException, StoreError>;

    /// Removes one current accommodation and immediately re-resolves affected work.
    async fn delete_assignment_policy_exception(
        &self,
        context: TenantContext,
        command: DeleteAssignmentPolicyExceptionCommand,
    ) -> Result<AssignmentRevision, StoreError>;

    /// Reads one exception by its non-authorizing internal identity.
    async fn get_assignment_policy_exception(
        &self,
        context: TenantContext,
        assignment: AssignmentId,
        exception: AssignmentPolicyExceptionId,
    ) -> Result<Option<StoredAssignmentPolicyException>, StoreError>;

    /// Resolves the exact current policy used for one assignment enrollment.
    async fn resolve_assignment_timing(
        &self,
        context: TenantContext,
        assignment: AssignmentId,
        student: StudentId,
    ) -> Result<Option<ResolvedAssignmentTiming>, StoreError>;

    /// Reads the effective policy explanation recorded for one issued attempt.
    async fn get_attempt_resolved_timing(
        &self,
        context: TenantContext,
        attempt: QuestionAttemptId,
    ) -> Result<Option<ResolvedAttemptTiming>, StoreError>;

    /// Retires one fixed item or selection candidate and recalculates all current grades.
    ///
    /// The command is rejected while an affected attempt is in progress. Submitted
    /// evidence remains protected; future runs omit the retired identity.
    async fn delete_and_regrade_assignment_item(
        &self,
        context: TenantContext,
        command: DeleteAndRegradeAssignmentItemCommand,
    ) -> Result<StoredAssignment, StoreError>;

    /// Reads one assignment and its current revision for an authenticated edit.
    async fn get_assignment_for_edit(
        &self,
        context: TenantContext,
        assignment: AssignmentId,
    ) -> Result<Option<StoredAssignment>, StoreError>;

    /// Reads one assignment inside the active tenant.
    async fn get_assignment(
        &self,
        context: TenantContext,
        assignment: AssignmentId,
    ) -> Result<Option<AssignmentRecord>, StoreError>;

    /// Lists assignments inside the active tenant in stable cursor order.
    async fn list_assignments(
        &self,
        context: TenantContext,
        course: CourseId,
        page: PageRequest,
    ) -> Result<Page<AssignmentRecord>, StoreError>;

    /// Creates one enrollment and its empty compact summary.
    async fn create_enrollment(
        &self,
        context: TenantContext,
        enrollment: AssignmentEnrollment,
    ) -> Result<(), StoreError>;

    /// Reads one enrollment inside the active tenant.
    async fn get_enrollment(
        &self,
        context: TenantContext,
        enrollment: EnrollmentId,
    ) -> Result<Option<AssignmentEnrollment>, StoreError>;

    /// Starts the next run or returns the enrollment's existing active run.
    ///
    /// The backend owns the timestamp, one-based run number, mode, policy,
    /// and compact-summary transition. The proposed ID is used only when a new
    /// run is actually inserted.
    async fn start_or_resume_run(
        &self,
        context: TenantContext,
        actor: UserId,
        assignment: AssignmentId,
        proposed_run: RunId,
    ) -> Result<AssignmentRun, StoreError>;

    /// Reads the immutable selected questions and issued order frozen at run start.
    async fn assignment_run_items(
        &self,
        context: TenantContext,
        run: RunId,
    ) -> Result<Vec<AssignmentRunItem>, StoreError>;

    /// Issues a fresh question or returns the run's unresolved instance.
    ///
    /// Storage supplies the authoritative issue time and deadline and permits
    /// at most one unresolved question in a run.
    async fn issue_or_resume_question_attempt(
        &self,
        context: TenantContext,
        command: IssueQuestionAttemptCommand,
    ) -> Result<QuestionAttempt, StoreError>;

    /// Reserves a key-free future variant for an owned unresolved predecessor.
    /// This operation never creates a question attempt or starts a timer.
    async fn reserve_or_resume_prefetched_question(
        &self,
        context: TenantContext,
        command: ReservePrefetchedQuestionCommand,
    ) -> Result<PrefetchedQuestion, StoreError>;

    /// Finds a reservation selected by trusted server sequencing. Promotion
    /// remains atomic in `issue_or_resume_question_attempt`.
    async fn get_prefetched_question(
        &self,
        context: TenantContext,
        actor: UserId,
        run: RunId,
        predecessor: QuestionAttemptId,
        assignment_position: u32,
    ) -> Result<Option<PrefetchedQuestion>, StoreError>;

    /// Reads the immutable next-attempt result for an owned submission.
    async fn submission_next_attempt(
        &self,
        context: TenantContext,
        actor: UserId,
        predecessor: QuestionAttemptId,
    ) -> Result<SubmissionNextAttempt, StoreError>;

    /// Returns the sole owned committed submission in a run whose successor
    /// receipt has not yet been finalized. Ambiguity is a conflict, never a
    /// route-level guess.
    async fn pending_submission_for_run(
        &self,
        context: TenantContext,
        actor: UserId,
        run: RunId,
    ) -> Result<Option<QuestionAttemptId>, StoreError>;

    /// Finalizes a no-successor receipt after the server has checked current
    /// run state. Repeating the exact decision is safe; a different decision
    /// conflicts rather than rewriting an earlier receipt.
    async fn finalize_submission_next_attempt(
        &self,
        context: TenantContext,
        actor: UserId,
        predecessor: QuestionAttemptId,
        next: Option<QuestionAttemptId>,
    ) -> Result<(), StoreError>;

    /// Lists browser-safe attempt projections for one run in stable cursor order.
    async fn list_question_attempts(
        &self,
        context: TenantContext,
        run: RunId,
        page: PageRequest,
    ) -> Result<Page<QuestionAttempt>, StoreError>;

    /// Returns a prior exact submission before invoking a grading backend again.
    ///
    /// A changed response or key for an already submitted attempt is a conflict.
    async fn replay_submission(
        &self,
        context: TenantContext,
        actor: UserId,
        attempt: QuestionAttemptId,
        response: &StudentResponse,
        idempotency_key: &SubmissionIdempotencyKey,
    ) -> Result<Option<SubmissionRecord>, StoreError>;

    /// Atomically records the first response, grade event, run completion, and summary.
    ///
    /// The backend supplies the submission timestamp and applies the authored
    /// timing policy. Exact retries return the first committed record.
    async fn submit_question_attempt(
        &self,
        context: TenantContext,
        command: SubmitQuestionAttemptCommand,
    ) -> Result<SubmissionRecord, StoreError>;

    /// Closes an active question without inventing a response or score.
    ///
    /// Only a persisted direct course instructor may perform this action.
    /// The attempt becomes `needs_manual_grading` and exact action retries
    /// return the original minimal audit record.
    async fn force_submit_attempt(
        &self,
        context: TenantContext,
        command: ForceSubmitAttemptCommand,
    ) -> Result<AttemptSupportRecord, StoreError>;

    /// Excludes an attempt from current scoring while retaining raw evidence.
    ///
    /// A submitted evaluation triggers generation-fenced assignment
    /// recalculation; exact action retries never enqueue a duplicate job.
    async fn clear_attempt(
        &self,
        context: TenantContext,
        command: ClearAttemptCommand,
    ) -> Result<AttemptSupportRecord, StoreError>;

    /// Atomically records an authorized instructor release of an existing
    /// first-grade feedback record. The original receipt is never rewritten.
    async fn release_attempt_feedback(
        &self,
        context: TenantContext,
        command: ReleaseAttemptFeedbackCommand,
    ) -> Result<FeedbackReleaseRecord, StoreError>;

    /// Reads the current release state for one attempt after proving the actor
    /// owns that educational record or directly instructs its course.
    async fn get_attempt_feedback_release(
        &self,
        context: TenantContext,
        actor: UserId,
        attempt: QuestionAttemptId,
    ) -> Result<Option<FeedbackReleaseRecord>, StoreError>;

    /// Reads the bounded, private material for one run-summary projection.
    ///
    /// The actor must own the enrollment or directly instruct its course. A
    /// failed authorization is deliberately indistinguishable from absence.
    /// Implementations use a stable `(assignment_position, attempt_id)` cursor
    /// and never consult question envelopes or re-run an adapter.
    async fn get_run_summary_page(
        &self,
        context: TenantContext,
        actor: UserId,
        run: RunId,
        page: PageRequest,
    ) -> Result<RunSummaryPageInput, StoreError>;

    /// Atomically writes activity and its compact summary projection.
    async fn apply_activity_transition(
        &self,
        context: TenantContext,
        transition: ActivityTransition,
    ) -> Result<StudentAssignmentSummary, StoreError>;

    /// Reads one run inside the active tenant.
    async fn get_run(
        &self,
        context: TenantContext,
        run: RunId,
    ) -> Result<Option<AssignmentRun>, StoreError>;

    /// Lists runs for one enrollment in stable cursor order.
    async fn list_runs(
        &self,
        context: TenantContext,
        enrollment: EnrollmentId,
        page: PageRequest,
    ) -> Result<Page<AssignmentRun>, StoreError>;

    /// Reads one question attempt inside the active tenant.
    async fn get_question_attempt(
        &self,
        context: TenantContext,
        attempt: QuestionAttemptId,
    ) -> Result<Option<QuestionAttempt>, StoreError>;

    /// Reads the transactionally maintained summary for one enrollment.
    async fn get_summary(
        &self,
        context: TenantContext,
        enrollment: EnrollmentId,
    ) -> Result<Option<StudentAssignmentSummary>, StoreError>;

    /// Lists compact gradebook rows for one tenant-owned course.
    ///
    /// The stable cursor is the backend-owned `(assignment_id, enrollment_id)`
    /// key. Implementations read assignment, enrollment, and maintained
    /// summary rows only; they do not scan run or attempt history.
    async fn list_gradebook_rows(
        &self,
        context: TenantContext,
        course: CourseId,
        page: PageRequest,
    ) -> Result<Page<GradebookSummaryRow>, StoreError>;
}
