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
use domain::completion::{
    RequiredQuestionState, WithinRunCompletion, derive_within_run_completion,
};
use domain::run::RunModelError;
use domain::scoring::RunTransition;
use objects::{Bucket, ObjectCategory, ObjectKey, ObjectRecord, Sha256Digest};
use question_model::taxonomy::TaxonomyTerm;
use question_model::{
    ActivityTimestamp, AssetId, AssignmentDeliveryState, AssignmentEnrollment, AssignmentId,
    AssignmentItem, AssignmentItemId, AssignmentPolicyExceptionId, AssignmentRun,
    AssignmentRunItem, AssignmentSelectionGroup, AssignmentSummary, AssignmentTimingPolicy,
    AttemptProvenance, AttemptResult, AttemptStatus, BackendCapabilities, CatalogLifecycle,
    CatalogProblemDetail, CatalogProblemSummary, CatalogSearchPage, CatalogSearchQuery,
    CourseGroupId, CourseId, CourseMembership, CourseMembershipRole, CourseRole, CourseSummary,
    DraftQuestionDefinition, DraftQuestionSource, EnrollmentId, GradePolicy, GradebookSummaryRow,
    ObjectId, ProblemId, PublicationScope, QuestionAttempt, QuestionAttemptId, QuestionBackend,
    QuestionDefinition, QuestionSource, QuestionStatisticsDisclosure, RunId, RunPolicies,
    ScoringGeneration, ScoringStatus, SelectionOrdering, StudentAssignmentSummary, StudentId,
    StudentResponse, TenantId, UserId, VersionId, WorkspaceDraftSummary, WorkspaceId,
    WorkspaceImportId,
};
use question_model::{FeedbackContent, envelope::ContentBlock};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

mod gradebook_cursor;
/// In-memory backend used by tests and lanes waiting for PostgreSQL.
pub mod jobs;
pub mod memory;
/// Cursor and bounded-page types shared by every list method.
pub mod pagination;
/// PostgreSQL health and future backend implementation.
pub mod postgres;
/// Pure retention lifecycle policy; persistence and worker execution land in MOD-RETENTION R2+.
pub mod retention;
/// Explicit tenant context used by every educational-record operation.
pub mod rls;
mod run_summary_cursor;
/// Provider-neutral, replica-safe authentication session contract.
pub mod session;
mod statistics;

pub use crate::jobs::{
    ClaimedJob, CreateAssignmentExport, EnqueueJob, ExportArtifactKind, ExportArtifactRecord,
    ExportCommitDisposition, ExportId, ExportJobCommit, ExportJobStore, JobFailureDisposition,
    JobFailureKind, JobId, JobLeaseDuration, JobLeaseToken, JobPayload, JobState, JobStore,
    QueueDepth, StudentExportArtifactView, StudentExportJob, StudentExportState, StudentExportView,
    TenantJobView,
};
pub use crate::pagination::{Cursor, Page, PageRequest, PageSize, PaginationError};
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

/// Opaque route identifier for either a logical catalog asset or a tenant object.
///
/// The identifier is never minted independently: public content reuses its
/// [`AssetId`], and a student-record artifact reuses its [`ObjectId`]. That
/// lets one stable `/api/assets/{id}` route serve both classes without
/// collapsing their distinct model identities.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct AssetDeliveryId(Uuid);

impl AssetDeliveryId {
    /// Builds the route identifier for a logical catalog asset.
    pub fn from_asset(asset: AssetId) -> Self {
        Self(asset.as_uuid())
    }

    /// Builds the route identifier for a tenant-owned physical artifact.
    pub fn from_object(object: ObjectId) -> Self {
        Self(object.as_uuid())
    }

    /// Returns the storage UUID.
    pub fn as_uuid(&self) -> Uuid {
        self.0
    }
}

impl std::fmt::Display for AssetDeliveryId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}", self.0)
    }
}

impl std::str::FromStr for AssetDeliveryId {
    type Err = uuid::Error;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Uuid::parse_str(value).map(Self)
    }
}

/// Authorization linkage stored beside one immutable object record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum AssetDeliveryScope {
    /// Content asset whose current visibility comes from its published version.
    Catalog {
        /// Logical asset embedded in browser-safe question markup.
        asset: AssetId,
        /// Exact immutable version owning the asset.
        reference: ProblemVersionRef,
    },
    /// Educational-record artifact visible only to explicitly named users.
    StudentRecord {
        /// Direct RLS boundary owning the artifact.
        tenant: TenantId,
        /// Exact course whose retention lifecycle governs this record.
        course: CourseId,
        /// Authenticated users allowed to request a short-lived URL.
        authorized_users: Vec<UserId>,
    },
}

/// Database-authoritative mapping from a route ID to exact stored bytes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetDeliveryRecord {
    /// Stable identifier accepted by `/api/assets/{id}`.
    pub id: AssetDeliveryId,
    /// Immutable metadata returned after object bytes were written.
    pub object: ObjectRecord,
    /// Visibility and ownership linkage checked on every protected request.
    pub scope: AssetDeliveryScope,
}

/// Immutable logical-to-physical asset mapping for one published catalog version.
///
/// This is an internal storage result used while reproducing a server-issued
/// question attempt. It deliberately omits object metadata and delivery
/// authorization because neither belongs in browser question delivery.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CatalogAssetBinding {
    /// Browser-safe logical asset referenced by immutable authored content.
    pub asset: AssetId,
    /// Exact immutable object selected when this version was published.
    pub object: ObjectId,
}

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

/// Private QTI staging evidence selected before any published identity exists.
///
/// The server copies the referenced bytes to candidate published object keys
/// first. Store promotion validates this exact committed staging import and
/// atomically records its catalog asset bindings and grader-owned material.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QtiPublicationPromotion {
    pub staging: QtiImportRef,
    pub assets: Vec<AssetDeliveryRecord>,
}

/// Tenant/workspace/import address for a private, immutable QTI staging record.
///
/// This is deliberately not a browser DTO and contains no published identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QtiImportRef {
    pub tenant: TenantId,
    pub workspace: WorkspaceId,
    pub import: WorkspaceImportId,
}

/// Browser-safe identity and integrity record for one item in a QTI package.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QtiImportItem {
    pub item_id: String,
    /// Digest of the canonical, server-sanitized item representation.
    pub model_sha256: Sha256Digest,
    /// Logical assets the item references. The registry verifies each exists.
    pub assets: Vec<AssetId>,
}

/// A supported package feature retained for author-facing diagnostics.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QtiUnsupportedFeature {
    pub code: String,
    pub location: String,
}

/// Complete safe metadata for a private QTI import.
///
/// Grading choices and archive bytes are intentionally absent. This type is
/// persistence-only; it is never included in question-model serialization.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QtiImportRegistry {
    pub reference: QtiImportRef,
    pub source: ObjectRecord,
    pub parse_schema: String,
    pub adapter_version: String,
    pub items: Vec<QtiImportItem>,
    pub assets: Vec<ObjectRecord>,
    pub unsupported_features: Vec<QtiUnsupportedFeature>,
}

/// Opaque answer-bearing material stored only in the grader table.
///
/// It has no serialization or display implementation and redacts itself when
/// diagnostics inspect a registration. The import writer can persist it; only
/// [`QtiGradingStore`] can read it back.
#[derive(Clone, PartialEq, Eq)]
pub struct QtiImportGradingPayload(Vec<u8>);

impl QtiImportGradingPayload {
    pub fn new(bytes: Vec<u8>) -> Result<Self, StoreError> {
        if bytes.is_empty() || bytes.len() > 256 * 1024 {
            return Err(StoreError::InvalidRecord(
                "QTI grading payload must contain 1 to 262144 bytes".to_string(),
            ));
        }
        Ok(Self(bytes))
    }

    /// Returns an integrity digest without disclosing answer-bearing bytes.
    pub fn sha256(&self) -> Sha256Digest {
        Sha256Digest::compute(&self.0)
    }

    /// Decodes the one correct choice stored by the bounded QTI importer.
    ///
    /// The bytes remain private to the dedicated grader capability.  This
    /// method is intentionally the narrowest server-side handoff: callers can
    /// construct an ordinary grading key, but cannot inspect, serialize, or
    /// log the archived grading payload itself.
    pub fn server_correct_choice(&self) -> Result<question_model::response::ChoiceId, StoreError> {
        serde_json::from_slice(&self.0).map_err(|_| {
            StoreError::InvalidRecord("stored QTI grading payload is invalid".to_string())
        })
    }

    #[cfg(feature = "postgres")]
    pub(crate) fn bytes(&self) -> &[u8] {
        &self.0
    }
}

impl std::fmt::Debug for QtiImportGradingPayload {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("QtiImportGradingPayload([redacted])")
    }
}

/// One item plus its answer-bearing, server-only grading binding.
#[derive(Clone, PartialEq, Eq)]
pub struct QtiImportItemRegistration {
    pub item: QtiImportItem,
    pub grading: QtiImportGradingPayload,
}

impl std::fmt::Debug for QtiImportItemRegistration {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("QtiImportItemRegistration")
            .field("item", &self.item)
            .field("grading", &"[redacted]")
            .finish()
    }
}

/// All metadata supplied after object bytes have already been written and verified.
#[derive(Clone)]
pub struct CreateQtiImportCommand {
    pub registry: QtiImportRegistry,
    pub item_bindings: Vec<QtiImportItemRegistration>,
}

/// Exact private worker claim allowed to expose a prepared QTI import.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommitPreparedQtiImport {
    pub job: JobId,
    pub lease: JobLeaseToken,
    pub reference: QtiImportRef,
    pub source_object: ObjectId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommitPreparedQtiImportOutcome {
    Committed,
    ClaimNoLongerActive,
}

impl std::fmt::Debug for CreateQtiImportCommand {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("CreateQtiImportCommand")
            .field("registry", &self.registry)
            .field("item_binding_count", &self.item_bindings.len())
            .finish()
    }
}

/// Audit payload appended before a protected signed URL is requested.
///
/// It deliberately contains neither the signed URL nor session credentials.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetAccessEvent {
    /// Tenant in whose security context the request was authorized.
    pub tenant: TenantId,
    /// Authenticated person requesting the protected object.
    pub actor: UserId,
    /// Stable route identifier requested by the actor.
    pub delivery: AssetDeliveryId,
    /// Exact physical object whose URL may be issued.
    pub object: ObjectId,
    /// Bucket whose fixed delivery lifetime applies.
    pub bucket: Bucket,
    /// Course that authorized this delivery access, when visible in learner records.
    pub course: Option<CourseId>,
    /// Database-authoritative authorization time.
    pub occurred_at: ActivityTimestamp,
}

/// Protected object record and the timestamp used to bound its signed URL.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthorizedAssetDelivery {
    /// Exact immutable object record selected by the database registry.
    pub record: AssetDeliveryRecord,
    /// Database-authoritative time already captured in the access audit.
    pub authorized_at: ActivityTimestamp,
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

/// Server-issued optimistic revision for one current course group.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CourseGroupRevision(u64);

impl CourseGroupRevision {
    pub(crate) const INITIAL: Self = Self(1);
    const MAX: u64 = i64::MAX as u64;

    /// Returns the positive stored revision number.
    pub fn value(self) -> u64 {
        self.0
    }

    pub(crate) fn next(self) -> Result<Self, StoreError> {
        self.0
            .checked_add(1)
            .filter(|value| *value <= Self::MAX)
            .map(Self)
            .ok_or_else(|| {
                StoreError::Unavailable("course group revision limit reached".to_string())
            })
    }

    #[cfg(feature = "postgres")]
    pub(crate) fn from_stored(value: i64) -> Result<Self, StoreError> {
        let value = u64::try_from(value).map_err(|_| {
            StoreError::Unavailable("stored course group revision is invalid".to_string())
        })?;
        if value == 0 {
            return Err(StoreError::Unavailable(
                "stored course group revision is invalid".to_string(),
            ));
        }
        Ok(Self(value))
    }
}

/// Current course group whose members may share an assignment accommodation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CourseGroupRecord {
    pub id: CourseGroupId,
    pub tenant: TenantId,
    pub course: CourseId,
    pub title: String,
    pub members: Vec<UserId>,
}

/// One course group together with its exact compare-and-swap revision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredCourseGroup {
    pub record: CourseGroupRecord,
    pub revision: CourseGroupRevision,
}

/// Instructor-authenticated create or replacement of a course group.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PutCourseGroupCommand {
    pub actor: UserId,
    pub expected_revision: Option<CourseGroupRevision>,
    pub record: CourseGroupRecord,
}

/// One exception target. A student target is assignment-enrollment identity;
/// a group target is current course membership identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AssignmentPolicyExceptionTarget {
    Student(StudentId),
    CourseGroup(CourseGroupId),
}

/// Explicit availability endpoint override. `Unrestricted` is distinct from
/// an absent field, which means this exception does not address that endpoint.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AssignmentExceptionTimestamp {
    Unrestricted,
    At(ActivityTimestamp),
}

/// Explicit attempt/timer override. `Unlimited` is distinct from inheritance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AssignmentExceptionLimit {
    Unlimited,
    Value(u32),
}

/// Mutable current accommodation for one student or course group.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssignmentPolicyException {
    pub id: AssignmentPolicyExceptionId,
    pub target: AssignmentPolicyExceptionTarget,
    pub available_at: Option<AssignmentExceptionTimestamp>,
    pub closes_at: Option<AssignmentExceptionTimestamp>,
    pub time_limit_seconds: Option<AssignmentExceptionLimit>,
    pub attempt_limit: Option<AssignmentExceptionLimit>,
}

/// One stored exception paired with the assignment's shared revision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredAssignmentPolicyException {
    pub exception: AssignmentPolicyException,
    pub assignment_revision: AssignmentRevision,
}

/// Effective learner policy and the exceptions that actually expanded it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolvedAssignmentTiming {
    pub tenant: TenantId,
    pub course: CourseId,
    pub assignment: AssignmentId,
    pub student: StudentId,
    pub policy: AssignmentTimingPolicy,
    pub contributors: Vec<AssignmentPolicyExceptionTarget>,
    pub revision: AssignmentRevision,
}

/// Policy explanation recorded for one issued attempt. Terminal work retains
/// the last policy that governed it instead of being rewritten by later edits.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolvedAttemptTiming {
    pub attempt: QuestionAttemptId,
    pub policy: AssignmentTimingPolicy,
    pub contributors: Vec<AssignmentPolicyExceptionTarget>,
}

/// Revision-checked replacement for one target's current accommodation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetAssignmentPolicyExceptionCommand {
    pub actor: UserId,
    pub course: CourseId,
    pub assignment: AssignmentId,
    pub expected_revision: AssignmentRevision,
    pub exception: AssignmentPolicyException,
}

/// Revision-checked removal of one current accommodation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeleteAssignmentPolicyExceptionCommand {
    pub actor: UserId,
    pub course: CourseId,
    pub assignment: AssignmentId,
    pub expected_revision: AssignmentRevision,
    pub exception: AssignmentPolicyExceptionId,
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

/// Exact immutable binding for one server-mediated external-tool exchange.
///
/// This is deliberately a store-private/server-core contract, rather than a
/// catalog or browser DTO.  In particular, `provider` is a configured opaque
/// deployment key, never an endpoint.
#[derive(Clone, PartialEq, Eq)]
pub struct ExternalToolBinding {
    pub provider: String,
    pub problem: ProblemId,
    pub version: VersionId,
    pub seed: u64,
    pub source_object: ObjectId,
    pub source_sha256: String,
    pub integration_profile: String,
    pub response_sha256: Sha256Digest,
}

impl std::fmt::Debug for ExternalToolBinding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ExternalToolBinding([redacted])")
    }
}

impl ExternalToolBinding {
    /// Rejects unbounded opaque deployment values before they reach storage.
    pub fn validate(&self) -> Result<(), StoreError> {
        for (name, value, max) in [
            ("provider", self.provider.as_str(), 160usize),
            ("source checksum", self.source_sha256.as_str(), 64),
            (
                "integration profile",
                self.integration_profile.as_str(),
                160,
            ),
        ] {
            if value.is_empty()
                || value.len() > max
                || !value
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
            {
                return Err(StoreError::InvalidRecord(format!(
                    "external tool {name} is invalid"
                )));
            }
        }
        if self.source_sha256.len() != 64
            || !self
                .source_sha256
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit())
        {
            return Err(StoreError::InvalidRecord(
                "external tool source checksum is invalid".to_string(),
            ));
        }
        Ok(())
    }
}

/// Opaque persisted provider correlation. It cannot be serialized or logged.
#[derive(Clone, PartialEq, Eq)]
pub struct PersistedCorrelation(Vec<u8>);

impl PersistedCorrelation {
    pub fn new(value: Vec<u8>) -> Result<Self, StoreError> {
        if value.is_empty() || value.len() > 512 {
            return Err(StoreError::InvalidRecord(
                "external-tool correlation must be 1 to 512 bytes".to_string(),
            ));
        }
        Ok(Self(value))
    }
    /// Returns an owned copy for a server-only adapter codec.
    ///
    /// This type remains non-serde and redacts diagnostics; callers must
    /// validate their own authenticated correlation format before sending
    /// anything to an external provider.
    pub fn to_storage_bytes(&self) -> Vec<u8> {
        self.0.clone()
    }
    #[cfg(feature = "postgres")]
    pub(crate) fn bytes(&self) -> &[u8] {
        &self.0
    }
    #[cfg(feature = "postgres")]
    pub(crate) fn from_stored(value: Vec<u8>) -> Result<Self, StoreError> {
        Self::new(value)
    }
}

impl std::fmt::Debug for PersistedCorrelation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("PersistedCorrelation([redacted])")
    }
}

/// Opaque short-lived lease proving that a replica owns verification work.
#[derive(Clone, PartialEq, Eq)]
pub struct ExternalToolLeaseToken([u8; 32]);

impl ExternalToolLeaseToken {
    fn generate() -> Result<Self, StoreError> {
        let mut bytes = [0_u8; 32];
        getrandom::fill(&mut bytes).map_err(|error| {
            StoreError::Unavailable(format!("external-tool lease entropy unavailable: {error}"))
        })?;
        Ok(Self(bytes))
    }
    #[cfg(feature = "postgres")]
    pub(crate) fn bytes(&self) -> &[u8; 32] {
        &self.0
    }

    pub(crate) fn hash(&self) -> Sha256Digest {
        Sha256Digest::compute(&self.0)
    }
}

impl std::fmt::Debug for ExternalToolLeaseToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ExternalToolLeaseToken([redacted])")
    }
}

/// Work a single replica may send to a configured external provider.
#[derive(Clone)]
pub struct ExternalToolLease {
    pub binding: ExternalToolBinding,
    pub correlation: PersistedCorrelation,
    pub token: ExternalToolLeaseToken,
    pub expires_at: ActivityTimestamp,
}

impl std::fmt::Debug for ExternalToolLease {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ExternalToolLease")
            .field("binding", &self.binding)
            .field("expires_at", &self.expires_at)
            .finish_non_exhaustive()
    }
}

/// A server-verified grade retained until its ordinary attempt submission commits.
#[derive(Clone, PartialEq)]
pub struct ExternalToolVerifiedPending {
    /// Immutable server-only binding restored after a verifier crash.
    pub binding: ExternalToolBinding,
    /// Opaque persisted correlation reused for the exact recovery commit.
    pub correlation: PersistedCorrelation,
    pub result: AttemptResult,
    pub result_sha256: Sha256Digest,
}

impl std::fmt::Debug for ExternalToolVerifiedPending {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ExternalToolVerifiedPending([redacted])")
    }
}

/// Result of atomically claiming or resuming a provider exchange.
#[derive(Clone)]
pub enum ExternalToolBegin {
    Committed(Box<SubmissionRecord>),
    VerifiedPending(ExternalToolVerifiedPending),
    Lease(ExternalToolLease),
    InProgress,
}

impl std::fmt::Debug for ExternalToolBegin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let state = match self {
            Self::Committed(_) => "Committed",
            Self::VerifiedPending(_) => "VerifiedPending",
            Self::Lease(_) => "Lease",
            Self::InProgress => "InProgress",
        };
        write!(f, "ExternalToolBegin::{state}([redacted])")
    }
}

/// Input to the atomic exchange claim.
#[derive(Clone)]
pub struct BeginExternalToolGradeCommand {
    pub actor: UserId,
    pub attempt: QuestionAttemptId,
    pub response: StudentResponse,
    pub idempotency_key: SubmissionIdempotencyKey,
    pub binding: ExternalToolBinding,
    pub proposed_correlation: PersistedCorrelation,
    pub lease_millis: u32,
}

/// Authenticated result staged by the lease holder before final commit.
#[derive(Clone)]
pub struct StageExternalToolVerificationCommand {
    pub actor: UserId,
    pub attempt: QuestionAttemptId,
    pub response: StudentResponse,
    pub idempotency_key: SubmissionIdempotencyKey,
    pub binding: ExternalToolBinding,
    pub correlation: PersistedCorrelation,
    pub lease_token: ExternalToolLeaseToken,
    pub result: AttemptResult,
}

/// Commits the staged verified grade through the ordinary attempt transition.
#[derive(Clone)]
pub struct CommitExternalToolSubmissionCommand {
    pub actor: UserId,
    pub attempt: QuestionAttemptId,
    pub response: StudentResponse,
    pub idempotency_key: SubmissionIdempotencyKey,
    pub binding: ExternalToolBinding,
    pub correlation: PersistedCorrelation,
    pub lease_token: ExternalToolLeaseToken,
    /// One-time same-origin frame capability consumed with the grade commit.
    pub launch_proof: ExternalToolLaunchProof,
}

/// Commits a previously staged provider verdict after the original verifier
/// lost its lease or process. This is server-only and intentionally carries no
/// lease token: exact binding, response, key, and correlation select the one
/// immutable `verified_pending` record.
#[derive(Clone)]
pub struct CommitVerifiedExternalToolSubmissionCommand {
    pub actor: UserId,
    pub attempt: QuestionAttemptId,
    pub response: StudentResponse,
    pub idempotency_key: SubmissionIdempotencyKey,
    pub binding: ExternalToolBinding,
    pub correlation: PersistedCorrelation,
    /// One-time same-origin frame capability consumed with the recovered commit.
    pub launch_proof: ExternalToolLaunchProof,
}

/// Durable external-grade state machine. This is intentionally not serde.
#[async_trait]
pub trait ExternalToolBrokerStore: Send + Sync {
    async fn begin_or_resume_external_grade(
        &self,
        context: TenantContext,
        command: BeginExternalToolGradeCommand,
    ) -> Result<ExternalToolBegin, StoreError>;
    async fn stage_external_tool_verification(
        &self,
        context: TenantContext,
        command: StageExternalToolVerificationCommand,
    ) -> Result<(), StoreError>;
    async fn commit_external_tool_submission(
        &self,
        context: TenantContext,
        command: CommitExternalToolSubmissionCommand,
    ) -> Result<SubmissionRecord, StoreError>;
    async fn commit_verified_external_tool_submission(
        &self,
        context: TenantContext,
        command: CommitVerifiedExternalToolSubmissionCommand,
    ) -> Result<SubmissionRecord, StoreError>;
}

/// Browser cookie material minted once for a short-lived same-origin launch.
/// It is intentionally non-serde and redacts itself in diagnostics.
#[derive(Clone, PartialEq, Eq)]
pub struct ExternalToolLaunchToken([u8; 32]);

impl ExternalToolLaunchToken {
    fn generate() -> Result<Self, StoreError> {
        let mut bytes = [0_u8; 32];
        getrandom::fill(&mut bytes).map_err(|error| {
            StoreError::Unavailable(format!("external-tool launch entropy unavailable: {error}"))
        })?;
        Ok(Self(bytes))
    }
    pub(crate) fn hash(&self) -> Sha256Digest {
        Sha256Digest::compute(&self.0)
    }
    /// Canonical cookie representation for the server-owned launch route.
    /// This remains opaque, non-serde, and never appears in a DTO.
    pub fn encode_cookie_value(&self) -> String {
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(self.0)
    }
    /// Parses only the exact 32-byte unpadded base64url cookie value.
    pub fn parse_cookie_value(value: &str) -> Result<Self, StoreError> {
        if value.len() != 43
            || !value
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
        {
            return Err(StoreError::InvalidRecord(
                "external-tool launch token is invalid".into(),
            ));
        }
        let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(value)
            .map_err(|_| {
                StoreError::InvalidRecord("external-tool launch token is invalid".into())
            })?;
        let array: [u8; 32] = bytes.try_into().map_err(|_| {
            StoreError::InvalidRecord("external-tool launch token is invalid".into())
        })?;
        if base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(array) != value {
            return Err(StoreError::InvalidRecord(
                "external-tool launch token is invalid".into(),
            ));
        }
        Ok(Self(array))
    }
}

pub(crate) fn fresh_external_tool_launch_id() -> Result<Uuid, StoreError> {
    let mut bytes = [0_u8; 16];
    getrandom::fill(&mut bytes).map_err(|error| {
        StoreError::Unavailable(format!("external-tool launch entropy unavailable: {error}"))
    })?;
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    Ok(Uuid::from_bytes(bytes))
}
impl std::fmt::Debug for ExternalToolLaunchToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ExternalToolLaunchToken([redacted])")
    }
}

/// Server-only proof that the learner still owns the exact same-origin launch
/// session used for this external-tool submission. It is non-serde and redacts
/// cookie material in diagnostics.
#[derive(Clone)]
pub struct ExternalToolLaunchProof {
    pub session_id: Uuid,
    pub token: ExternalToolLaunchToken,
}

impl std::fmt::Debug for ExternalToolLaunchProof {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ExternalToolLaunchProof([redacted])")
    }
}

/// Input to a server-created frame launch session. Provider state is already
/// encrypted by server configuration; it is never a browser token or URL.
#[derive(Clone)]
pub struct CreateExternalToolLaunchSessionCommand {
    pub actor: UserId,
    pub attempt: QuestionAttemptId,
    pub binding: ExternalToolBinding,
    pub encrypted_provider_state: Option<Vec<u8>>,
    pub lifetime_millis: u32,
}

/// The only time raw cookie bytes leave the Store boundary.
#[derive(Clone)]
pub struct CreatedExternalToolLaunchSession {
    pub id: Uuid,
    pub token: ExternalToolLaunchToken,
    pub expires_at: ActivityTimestamp,
}
impl std::fmt::Debug for CreatedExternalToolLaunchSession {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CreatedExternalToolLaunchSession")
            .field("id", &self.id)
            .field("expires_at", &self.expires_at)
            .finish()
    }
}

/// Server-only resolved launch state. It is never serialized into a route.
#[derive(Clone)]
pub struct ResolvedExternalToolLaunchSession {
    pub binding: ExternalToolBinding,
    pub encrypted_provider_state: Option<Vec<u8>>,
}
impl std::fmt::Debug for ResolvedExternalToolLaunchSession {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ResolvedExternalToolLaunchSession")
            .field("binding", &self.binding)
            .field(
                "encrypted_provider_state",
                &self.encrypted_provider_state.as_ref().map(|_| "[redacted]"),
            )
            .finish()
    }
}

#[async_trait]
pub trait ExternalToolLaunchSessionStore: Send + Sync {
    async fn create_external_tool_launch_session(
        &self,
        context: TenantContext,
        command: CreateExternalToolLaunchSessionCommand,
    ) -> Result<CreatedExternalToolLaunchSession, StoreError>;
    async fn resolve_external_tool_launch_session(
        &self,
        context: TenantContext,
        actor: UserId,
        attempt: QuestionAttemptId,
        id: Uuid,
        token: &ExternalToolLaunchToken,
    ) -> Result<Option<ResolvedExternalToolLaunchSession>, StoreError>;
    async fn revoke_external_tool_launch_session(
        &self,
        context: TenantContext,
        actor: UserId,
        attempt: QuestionAttemptId,
        id: Uuid,
    ) -> Result<(), StoreError>;
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

/// Tenant-owned private feedback retained beside the first grade.
///
/// The content is deliberately neither serde nor debug printable. Store
/// backends encode its closed `ContentBlock` representation only into their
/// private persistence table and must return the original stored content on a
/// matching idempotent replay.
#[derive(Clone, PartialEq, Eq)]
pub struct AttemptFeedbackRecord {
    content: FeedbackContent,
    content_sha256: Sha256Digest,
}

/// Immutable tenant-owned decision to unlock one first-grade feedback record.
///
/// This carries no feedback content and is never an attempt-list projection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FeedbackReleaseRecord {
    pub tenant: TenantId,
    pub attempt: QuestionAttemptId,
    pub released_by: UserId,
    pub released_at: ActivityTimestamp,
}

/// Private, bounded input for server-side feedback redaction on a run summary.
///
/// This is intentionally neither serializable nor debug printable. The route
/// turns it into a public DTO only after applying the current disclosure and
/// release policy.
#[derive(Clone, PartialEq)]
pub struct RunSummaryOutcomeInput {
    pub attempt: QuestionAttemptId,
    pub assignment_position: u32,
    pub submitted_at: Option<ActivityTimestamp>,
    pub response: Option<StudentResponse>,
    pub result: Option<AttemptResult>,
    pub feedback_policy: question_model::run_policy::FeedbackDisclosure,
    pub feedback: Option<AttemptFeedbackRecord>,
    pub release: Option<FeedbackReleaseRecord>,
}

/// Private run-summary material returned in one tenant-authorized store read.
///
/// It deliberately carries no question definition, source, provenance, key,
/// envelope, or provider data. `practice_allowed` is advisory presentation
/// state; `start_or_resume_run` remains the authoritative transition.
#[derive(Clone, PartialEq)]
pub struct RunSummaryPageInput {
    pub run: AssignmentRun,
    pub assignment: AssignmentRecord,
    pub summary: StudentAssignmentSummary,
    pub practice_allowed: bool,
    pub outcomes: Page<RunSummaryOutcomeInput>,
}

/// Trusted command for an instructor-initiated feedback disclosure release.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReleaseAttemptFeedbackCommand {
    pub actor: UserId,
    pub attempt: QuestionAttemptId,
}

impl AttemptFeedbackRecord {
    pub fn content(&self) -> &FeedbackContent {
        &self.content
    }

    pub fn content_sha256(&self) -> &Sha256Digest {
        &self.content_sha256
    }
}

/// Validates and canonicalizes private feedback before any submission state is
/// changed. The closed block model admits only structural content that the
/// renderer can safely escape or resolve through its own positive allowlist.
pub fn private_feedback_record(
    content: FeedbackContent,
) -> Result<AttemptFeedbackRecord, StoreError> {
    let mut budget = FeedbackBudget::default();
    validate_feedback_blocks(content.hint.as_deref(), "hint", &mut budget)?;
    validate_feedback_blocks(
        content.correct_response.as_deref(),
        "correct_response",
        &mut budget,
    )?;
    validate_feedback_blocks(content.rationale.as_deref(), "rationale", &mut budget)?;
    let encoded = canonical_feedback_bytes(&content)?;
    Ok(AttemptFeedbackRecord {
        content,
        content_sha256: Sha256Digest::compute(&encoded),
    })
}

pub(crate) fn canonical_feedback_bytes(content: &FeedbackContent) -> Result<Vec<u8>, StoreError> {
    // This local shape is intentionally private: FeedbackContent itself never
    // gains serde derives just to make database storage convenient.
    serde_json::to_vec(&(
        content.hint.as_deref(),
        content.correct_response.as_deref(),
        content.rationale.as_deref(),
    ))
    .map_err(|error| StoreError::InvalidRecord(format!("feedback encoding failed: {error}")))
}

#[derive(Default)]
struct FeedbackBudget {
    blocks: usize,
    bytes: usize,
}

fn validate_feedback_blocks(
    blocks: Option<&[ContentBlock]>,
    field: &str,
    budget: &mut FeedbackBudget,
) -> Result<(), StoreError> {
    const MAX_TOTAL_BLOCKS: usize = 64;
    const MAX_TOTAL_BYTES: usize = 64 * 1024;
    const MAX_TABLE_COLUMNS: usize = 64;
    const MAX_TABLE_ROWS: usize = 256;
    let Some(blocks) = blocks else {
        return Ok(());
    };
    let encoded = serde_json::to_vec(blocks).map_err(|error| {
        StoreError::InvalidRecord(format!("feedback {field} encoding failed: {error}"))
    })?;
    budget.blocks = budget.blocks.saturating_add(blocks.len());
    budget.bytes = budget.bytes.saturating_add(encoded.len());
    if budget.blocks > MAX_TOTAL_BLOCKS {
        return Err(StoreError::InvalidRecord(
            "feedback has too many blocks".to_string(),
        ));
    }
    if budget.bytes > MAX_TOTAL_BYTES {
        return Err(StoreError::InvalidRecord(
            "feedback is too large".to_string(),
        ));
    }
    for block in blocks {
        match block {
            // Literal text, code, and table cells are inert data. The
            // renderer owns escaping/sanitization; Store must not impose a
            // brittle content blacklist.
            ContentBlock::Text { .. } | ContentBlock::Math { .. } => {}
            ContentBlock::Image { asset, .. } => validate_feedback_asset_checksum(&asset.checksum)?,
            ContentBlock::Code { language, .. } => validate_feedback_language(language)?,
            ContentBlock::Table { headers, rows, .. } => {
                if headers.is_empty() || headers.len() > MAX_TABLE_COLUMNS {
                    return Err(StoreError::InvalidRecord(format!(
                        "feedback {field} table has an invalid column count"
                    )));
                }
                if rows.len() > MAX_TABLE_ROWS || rows.iter().any(|row| row.len() != headers.len())
                {
                    return Err(StoreError::InvalidRecord(format!(
                        "feedback {field} table rows do not match its headers"
                    )));
                }
            }
        }
    }
    Ok(())
}

fn validate_feedback_asset_checksum(checksum: &str) -> Result<(), StoreError> {
    if checksum.len() != 64
        || !checksum
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
    {
        return Err(StoreError::InvalidRecord(
            "feedback image has an invalid asset checksum".to_string(),
        ));
    }
    Ok(())
}

fn validate_feedback_language(language: &str) -> Result<(), StoreError> {
    if language.is_empty()
        || language.len() > 64
        || !language
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'+' | b'-' | b'_'))
    {
        return Err(StoreError::InvalidRecord(
            "feedback code has an invalid language tag".to_string(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod private_feedback_tests {
    use super::*;

    #[test]
    fn private_feedback_accepts_inert_teaching_text() {
        let content = FeedbackContent {
            hint: Some(vec![
                ContentBlock::Text {
                    markdown: "For this protein comparison, x < 5 is meaningful.".to_string(),
                },
                ContentBlock::Code {
                    language: "python".to_string(),
                    source: "if score > 0: continue".to_string(),
                },
                ContentBlock::Text {
                    markdown: "Literal reference: https://example.invalid/teaching".to_string(),
                },
            ]),
            ..FeedbackContent::default()
        };
        assert!(private_feedback_record(content).is_ok());
    }

    #[test]
    fn private_feedback_rejects_malformed_structure_and_oversized_content() {
        let malformed_table = FeedbackContent {
            hint: Some(vec![ContentBlock::Table {
                headers: vec!["residue".to_string(), "charge".to_string()],
                rows: vec![vec!["Lys".to_string()]],
                description: "amino-acid comparison".to_string(),
            }]),
            ..FeedbackContent::default()
        };
        assert!(matches!(
            private_feedback_record(malformed_table),
            Err(StoreError::InvalidRecord(_))
        ));

        let bad_image = FeedbackContent {
            hint: Some(vec![ContentBlock::Image {
                asset: question_model::envelope::AssetRef {
                    asset: AssetId::from_uuid(uuid::Uuid::nil()),
                    checksum: "not-a-sha256".to_string(),
                },
                description: "a peptide diagram".to_string(),
            }]),
            ..FeedbackContent::default()
        };
        assert!(matches!(
            private_feedback_record(bad_image),
            Err(StoreError::InvalidRecord(_))
        ));

        let oversized = FeedbackContent {
            hint: Some(vec![ContentBlock::Text {
                markdown: "x".repeat(64 * 1024),
            }]),
            ..FeedbackContent::default()
        };
        assert!(matches!(
            private_feedback_record(oversized),
            Err(StoreError::InvalidRecord(_))
        ));

        let too_many = FeedbackContent {
            hint: Some(
                (0..65)
                    .map(|_| ContentBlock::Text {
                        markdown: "bounded".to_string(),
                    })
                    .collect(),
            ),
            ..FeedbackContent::default()
        };
        assert!(matches!(
            private_feedback_record(too_many),
            Err(StoreError::InvalidRecord(_))
        ));
    }

    #[test]
    fn private_feedback_digest_is_stable_for_exact_content() {
        let content = FeedbackContent {
            hint: Some(vec![ContentBlock::Text {
                markdown: "Check your sign.".to_string(),
            }]),
            ..FeedbackContent::default()
        };
        let left = private_feedback_record(content.clone()).expect("valid private feedback");
        let right = private_feedback_record(content).expect("valid private feedback");
        assert_eq!(left.content_sha256(), right.content_sha256());
        assert!(left == right);
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
    let round_four = |value: f64| (value * 10_000.0).round() / 10_000.0;
    Ok((round_four(earned), round_four(possible)))
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

/// Replaces only the computed score fields and run pointers for one enrollment.
pub(crate) fn recalculated_enrollment_projection(
    mut enrollment: AssignmentEnrollment,
    mut summary: StudentAssignmentSummary,
    grade_policy: GradePolicy,
    mut completed_runs: Vec<domain::scoring::CompletedRunScore>,
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

/// Immutable asset registry and protected-delivery authorization boundary.
#[async_trait]
pub trait AssetStore: Send + Sync {
    /// Records metadata only after the owning workflow has stored object bytes.
    async fn register_asset_delivery(
        &self,
        context: TenantContext,
        record: AssetDeliveryRecord,
    ) -> Result<(), StoreError>;

    /// Resolves only globally public catalog content for direct CDN delivery.
    ///
    /// Institution content and every educational record deliberately look
    /// absent here so callers cannot bypass the authenticated path.
    async fn get_public_asset_delivery(
        &self,
        delivery: AssetDeliveryId,
    ) -> Result<Option<AssetDeliveryRecord>, StoreError>;

    /// Resolves every catalog asset registered for one exact visible version.
    ///
    /// The result is ordered by logical [`AssetId`] and intentionally excludes
    /// tenant-owned educational records. This lookup has no delivery audit or
    /// signed-URL side effect: it is solely the trusted bridge from immutable
    /// catalog content to provenance verification.
    async fn catalog_asset_bindings(
        &self,
        context: TenantContext,
        reference: ProblemVersionRef,
    ) -> Result<Vec<CatalogAssetBinding>, StoreError>;

    /// Authorizes one protected request and appends its audit event atomically.
    async fn authorize_asset_delivery(
        &self,
        context: TenantContext,
        actor: UserId,
        delivery: AssetDeliveryId,
    ) -> Result<AuthorizedAssetDelivery, StoreError>;
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

/// Private QTI staging registry. Object bytes must already exist when this is
/// called; this transaction records either the entire import or none of it.
#[async_trait]
pub trait QtiImportStore: Send + Sync {
    /// Persists fully validated but invisible worker preparation.
    async fn prepare_qti_import(
        &self,
        context: TenantContext,
        command: CreateQtiImportCommand,
    ) -> Result<(), StoreError>;

    /// Makes a prepared import visible and completes the exact active lease.
    async fn commit_prepared_qti_import(
        &self,
        context: TenantContext,
        command: CommitPreparedQtiImport,
    ) -> Result<CommitPreparedQtiImportOutcome, StoreError>;

    /// Resolves safe staging metadata under both tenant and workspace scope.
    /// A foreign tenant or workspace deliberately receives `None`.
    async fn get_qti_import(
        &self,
        context: TenantContext,
        workspace: WorkspaceId,
        import: WorkspaceImportId,
    ) -> Result<Option<QtiImportRegistry>, StoreError>;
}

/// Deliberately narrow answer-bearing QTI capability.
///
/// Browser, catalog, draft, object-delivery, and normal import-registry code
/// never require this trait. Implementations use the database grader role for
/// its read path.
#[async_trait]
pub trait QtiGradingStore: Send + Sync {
    async fn qti_import_grading(
        &self,
        context: TenantContext,
        workspace: WorkspaceId,
        import: WorkspaceImportId,
        item_id: &str,
    ) -> Result<Option<QtiImportGradingPayload>, StoreError>;

    /// Resolves the answer-bearing binding for one immutable published QTI
    /// version. This remains a grader-only capability: catalog and asset
    /// delivery code cannot obtain the material through this trait.
    async fn qti_published_grading(
        &self,
        context: TenantContext,
        reference: ProblemVersionRef,
        item_id: &str,
    ) -> Result<Option<QtiImportGradingPayload>, StoreError>;
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

/// Extracts the grade policy used by an activity projection.
pub(crate) fn grade_policy(assignment: &AssignmentRecord) -> GradePolicy {
    assignment.policies.grade
}

/// Maps a storage activity write to the pure domain transition.
pub(crate) fn summary_transition(transition: &ActivityTransition) -> RunTransition {
    match transition {
        ActivityTransition::StartRun { run } => RunTransition::Started { at: run.started_at },
        ActivityTransition::RecordQuestionAttempt { attempt } => {
            RunTransition::QuestionAttemptRecorded {
                at: attempt
                    .timer
                    .submitted_at
                    .unwrap_or(attempt.timer.issued_at),
            }
        }
        ActivityTransition::CompleteRun { score, at, .. } => RunTransition::Completed {
            score: *score,
            at: *at,
        },
    }
}

/// Refuses a tenant-owned record outside the authenticated context.
pub(crate) fn ensure_tenant(
    context: TenantContext,
    record_tenant: TenantId,
) -> Result<(), StoreError> {
    if context.tenant_id() == record_tenant {
        Ok(())
    } else {
        Err(StoreError::TenantMismatch)
    }
}

/// Validates a course record before either backend persists it.
pub(crate) fn validate_course(course: &CourseRecord) -> Result<(), StoreError> {
    validate_title("course", &course.title)?;
    if course.members.is_empty() {
        return Err(StoreError::InvalidRecord(
            "course must have at least one member".to_string(),
        ));
    }
    if !course
        .members
        .iter()
        .any(|membership| membership.role == CourseMembershipRole::Instructor)
    {
        return Err(StoreError::InvalidRecord(
            "course must have at least one instructor".to_string(),
        ));
    }
    let unique_members: std::collections::BTreeSet<_> = course
        .members
        .iter()
        .map(|membership| membership.user)
        .collect();
    if unique_members.len() != course.members.len() {
        return Err(StoreError::InvalidRecord(
            "course memberships must have unique users".to_string(),
        ));
    }
    Ok(())
}

/// Validates one current course group independently of backend authority.
pub(crate) fn validate_course_group(group: &CourseGroupRecord) -> Result<(), StoreError> {
    validate_title("course group", &group.title)?;
    let unique_members: std::collections::BTreeSet<_> = group.members.iter().copied().collect();
    if unique_members.len() != group.members.len() {
        return Err(StoreError::InvalidRecord(
            "course group members must be unique".to_string(),
        ));
    }
    Ok(())
}

/// Validates assignment fields independent of catalog visibility.
pub(crate) fn validate_assignment(assignment: &AssignmentRecord) -> Result<(), StoreError> {
    validate_title("assignment", &assignment.title)?;
    if assignment.items.is_empty() && assignment.selection_groups.is_empty() {
        return Err(StoreError::InvalidRecord(
            "assignment must reference at least one published problem version".to_string(),
        ));
    }
    let mut item_ids = std::collections::BTreeSet::new();
    let mut positions = std::collections::BTreeSet::new();
    for item in &assignment.items {
        if !item_ids.insert(item.id) {
            return Err(StoreError::InvalidRecord(
                "assignment item identities must be unique".to_string(),
            ));
        }
        if !positions.insert(item.position) {
            return Err(StoreError::InvalidRecord(
                "assignment positions must be unique".to_string(),
            ));
        }
        if item.delivery_state == question_model::AssignmentDeliveryState::Retired
            && item.scoring_mode != question_model::AssignmentScoringMode::Excluded
        {
            return Err(StoreError::InvalidRecord(
                "retired assignment items must be excluded from current scoring".to_string(),
            ));
        }
    }
    let mut group_ids = std::collections::BTreeSet::new();
    for group in &assignment.selection_groups {
        if !group_ids.insert(group.id) || !positions.insert(group.position) {
            return Err(StoreError::InvalidRecord(
                "assignment selection identities and positions must be unique".to_string(),
            ));
        }
        let active_candidates = group
            .candidates
            .iter()
            .filter(|candidate| {
                candidate.delivery_state == question_model::AssignmentDeliveryState::Active
            })
            .count();
        if group.draw_count == 0
            || usize::try_from(group.draw_count)
                .map_or(true, |draw_count| draw_count > active_candidates)
            || group.algorithm_version == 0
        {
            return Err(StoreError::InvalidRecord(
                "selection groups need a positive bounded draw and algorithm version".to_string(),
            ));
        }
        let mut candidate_positions = std::collections::BTreeSet::new();
        for candidate in &group.candidates {
            if !item_ids.insert(candidate.id) {
                return Err(StoreError::InvalidRecord(
                    "assignment item and candidate identities must be unique".to_string(),
                ));
            }
            if !candidate_positions.insert(candidate.position) {
                return Err(StoreError::InvalidRecord(
                    "selection candidate positions must be unique within a group".to_string(),
                ));
            }
        }
        if candidate_positions
            .iter()
            .copied()
            .ne(0..u32::try_from(candidate_positions.len()).map_err(|_| {
                StoreError::InvalidRecord("too many selection candidates".to_string())
            })?)
        {
            return Err(StoreError::InvalidRecord(
                "selection candidate positions must be contiguous from zero".to_string(),
            ));
        }
    }
    if positions
        .iter()
        .copied()
        .ne(0..u32::try_from(positions.len())
            .map_err(|_| StoreError::InvalidRecord("too many assignment positions".to_string()))?)
    {
        return Err(StoreError::InvalidRecord(
            "assignment positions must be contiguous from zero".to_string(),
        ));
    }
    if let question_model::CompletionRequirement::ScoreAtLeast { fraction } =
        assignment.policies.completion
        && (!fraction.is_finite() || !(0.0..=1.0).contains(&fraction))
    {
        return Err(StoreError::InvalidRecord(
            "score-at-least completion fraction must be finite and between 0 and 1".to_string(),
        ));
    }
    Ok(())
}

/// Validates one current assignment access/timing policy before persistence.
pub(crate) fn validate_assignment_timing(policy: AssignmentTimingPolicy) -> Result<(), StoreError> {
    if policy.time_limit_seconds == Some(0) {
        return Err(StoreError::InvalidRecord(
            "assignment time limit must be greater than zero".to_string(),
        ));
    }
    if policy.attempt_limit == Some(0) {
        return Err(StoreError::InvalidRecord(
            "assignment attempt limit must be greater than zero".to_string(),
        ));
    }
    let ordered = policy
        .available_at
        .zip(policy.due_at)
        .is_none_or(|(available, due)| available <= due)
        && policy
            .due_at
            .zip(policy.closes_at)
            .is_none_or(|(due, closes)| due <= closes)
        && policy
            .available_at
            .zip(policy.closes_at)
            .is_none_or(|(available, closes)| available <= closes);
    if !ordered {
        return Err(StoreError::InvalidRecord(
            "assignment availability, due date, and close date must be ordered".to_string(),
        ));
    }
    Ok(())
}

/// Effective policy fields plus only the exception targets that expanded them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ResolvedAssignmentTimingPolicy {
    pub policy: AssignmentTimingPolicy,
    pub contributors: Vec<AssignmentPolicyExceptionTarget>,
}

/// Validates one exception before either backend mutates current state.
pub(crate) fn validate_assignment_policy_exception(
    exception: &AssignmentPolicyException,
) -> Result<(), StoreError> {
    if exception.available_at.is_none()
        && exception.closes_at.is_none()
        && exception.time_limit_seconds.is_none()
        && exception.attempt_limit.is_none()
    {
        return Err(StoreError::InvalidRecord(
            "an assignment policy exception must override at least one field".to_string(),
        ));
    }
    for (label, value) in [
        ("exception time limit", exception.time_limit_seconds),
        ("exception attempt limit", exception.attempt_limit),
    ] {
        if value == Some(AssignmentExceptionLimit::Value(0)) {
            return Err(StoreError::InvalidRecord(format!(
                "{label} must be greater than zero"
            )));
        }
    }
    if let (
        Some(AssignmentExceptionTimestamp::At(available_at)),
        Some(AssignmentExceptionTimestamp::At(closes_at)),
    ) = (exception.available_at, exception.closes_at)
        && available_at > closes_at
    {
        return Err(StoreError::InvalidRecord(
            "exception availability and close date must be ordered".to_string(),
        ));
    }
    Ok(())
}

/// Resolves all applicable accommodations against the assignment policy.
/// Every dimension can only become more permissive; an exception can never
/// shorten another learner's access by accident.
pub(crate) fn resolve_assignment_policy(
    base: AssignmentTimingPolicy,
    exceptions: &[AssignmentPolicyException],
) -> Result<ResolvedAssignmentTimingPolicy, StoreError> {
    validate_assignment_timing(base)?;
    let mut policy = base;
    let mut contributors = std::collections::BTreeSet::new();
    for exception in exceptions {
        validate_assignment_policy_exception(exception)?;
        if exception_expands_policy(base, exception) {
            contributors.insert(exception.target);
        }
        if let Some(value) = exception.available_at {
            expand_start_boundary(&mut policy.available_at, value);
        }
        if let Some(value) = exception.closes_at {
            expand_end_boundary(&mut policy.closes_at, value);
        }
        if let Some(value) = exception.time_limit_seconds {
            expand_numeric_limit(&mut policy.time_limit_seconds, value);
        }
        if let Some(value) = exception.attempt_limit {
            expand_numeric_limit(&mut policy.attempt_limit, value);
        }
    }
    validate_assignment_timing(policy)?;
    Ok(ResolvedAssignmentTimingPolicy {
        policy,
        contributors: contributors.into_iter().collect(),
    })
}

fn exception_expands_policy(
    base: AssignmentTimingPolicy,
    exception: &AssignmentPolicyException,
) -> bool {
    let mut available_at = base.available_at;
    let mut closes_at = base.closes_at;
    let mut time_limit_seconds = base.time_limit_seconds;
    let mut attempt_limit = base.attempt_limit;
    exception
        .available_at
        .is_some_and(|value| expand_start_boundary(&mut available_at, value))
        || exception
            .closes_at
            .is_some_and(|value| expand_end_boundary(&mut closes_at, value))
        || exception
            .time_limit_seconds
            .is_some_and(|value| expand_numeric_limit(&mut time_limit_seconds, value))
        || exception
            .attempt_limit
            .is_some_and(|value| expand_numeric_limit(&mut attempt_limit, value))
}

fn expand_start_boundary(
    current: &mut Option<ActivityTimestamp>,
    exception: AssignmentExceptionTimestamp,
) -> bool {
    match exception {
        AssignmentExceptionTimestamp::Unrestricted if current.is_some() => {
            *current = None;
            true
        }
        AssignmentExceptionTimestamp::At(value)
            if current.is_some_and(|existing| value < existing) =>
        {
            *current = Some(value);
            true
        }
        AssignmentExceptionTimestamp::Unrestricted | AssignmentExceptionTimestamp::At(_) => false,
    }
}

fn expand_end_boundary(
    current: &mut Option<ActivityTimestamp>,
    exception: AssignmentExceptionTimestamp,
) -> bool {
    match exception {
        AssignmentExceptionTimestamp::Unrestricted if current.is_some() => {
            *current = None;
            true
        }
        AssignmentExceptionTimestamp::At(value)
            if current.is_some_and(|existing| value > existing) =>
        {
            *current = Some(value);
            true
        }
        AssignmentExceptionTimestamp::Unrestricted | AssignmentExceptionTimestamp::At(_) => false,
    }
}

fn expand_numeric_limit(current: &mut Option<u32>, exception: AssignmentExceptionLimit) -> bool {
    match exception {
        AssignmentExceptionLimit::Unlimited if current.is_some() => {
            *current = None;
            true
        }
        AssignmentExceptionLimit::Value(value)
            if current.is_some_and(|existing| value > existing) =>
        {
            *current = Some(value);
            true
        }
        AssignmentExceptionLimit::Unlimited | AssignmentExceptionLimit::Value(_) => false,
    }
}

#[cfg(test)]
mod assignment_policy_exception_tests {
    use super::*;

    fn id(value: u128) -> Uuid {
        Uuid::from_u128(value)
    }

    #[test]
    fn applicable_exceptions_resolve_each_dimension_most_permissively() {
        let student = StudentId::from_uuid(id(1));
        let group = CourseGroupId::from_uuid(id(2));
        let base = AssignmentTimingPolicy {
            available_at: Some(ActivityTimestamp::from_unix_millis(100)),
            closes_at: Some(ActivityTimestamp::from_unix_millis(200)),
            time_limit_seconds: Some(10),
            attempt_limit: Some(1),
            ..AssignmentTimingPolicy::default()
        };
        let group_exception = AssignmentPolicyException {
            id: AssignmentPolicyExceptionId::from_uuid(id(3)),
            target: AssignmentPolicyExceptionTarget::CourseGroup(group),
            available_at: Some(AssignmentExceptionTimestamp::At(
                ActivityTimestamp::from_unix_millis(90),
            )),
            closes_at: Some(AssignmentExceptionTimestamp::At(
                ActivityTimestamp::from_unix_millis(300),
            )),
            time_limit_seconds: Some(AssignmentExceptionLimit::Value(20)),
            attempt_limit: Some(AssignmentExceptionLimit::Value(2)),
        };
        let student_exception = AssignmentPolicyException {
            id: AssignmentPolicyExceptionId::from_uuid(id(4)),
            target: AssignmentPolicyExceptionTarget::Student(student),
            available_at: Some(AssignmentExceptionTimestamp::Unrestricted),
            closes_at: Some(AssignmentExceptionTimestamp::At(
                ActivityTimestamp::from_unix_millis(250),
            )),
            time_limit_seconds: Some(AssignmentExceptionLimit::Unlimited),
            attempt_limit: Some(AssignmentExceptionLimit::Value(1)),
        };
        let resolved = resolve_assignment_policy(base, &[group_exception, student_exception])
            .expect("valid accommodations");
        assert_eq!(resolved.policy.available_at, None);
        assert_eq!(
            resolved.policy.closes_at,
            Some(ActivityTimestamp::from_unix_millis(300))
        );
        assert_eq!(resolved.policy.time_limit_seconds, None);
        assert_eq!(resolved.policy.attempt_limit, Some(2));
        assert_eq!(
            resolved.contributors,
            vec![
                AssignmentPolicyExceptionTarget::Student(student),
                AssignmentPolicyExceptionTarget::CourseGroup(group),
            ]
        );
    }

    #[test]
    fn exception_validation_refuses_empty_zero_and_reversed_overrides() {
        let mut exception = AssignmentPolicyException {
            id: AssignmentPolicyExceptionId::from_uuid(id(10)),
            target: AssignmentPolicyExceptionTarget::Student(StudentId::from_uuid(id(11))),
            available_at: None,
            closes_at: None,
            time_limit_seconds: None,
            attempt_limit: None,
        };
        assert!(validate_assignment_policy_exception(&exception).is_err());
        exception.time_limit_seconds = Some(AssignmentExceptionLimit::Value(0));
        assert!(validate_assignment_policy_exception(&exception).is_err());
        exception.time_limit_seconds = None;
        exception.available_at = Some(AssignmentExceptionTimestamp::At(
            ActivityTimestamp::from_unix_millis(2),
        ));
        exception.closes_at = Some(AssignmentExceptionTimestamp::At(
            ActivityTimestamp::from_unix_millis(1),
        ));
        assert!(validate_assignment_policy_exception(&exception).is_err());
    }
}

/// Validates that delivery metadata agrees with the typed immutable object key.
pub(crate) fn validate_asset_delivery(record: &AssetDeliveryRecord) -> Result<(), StoreError> {
    if record.object.id != record.object.key.object_id()
        || record.object.bucket != record.object.key.bucket()
        || record.object.category != record.object.key.category()
        || record.object.version != record.object.key.version_id()
    {
        return Err(StoreError::InvalidRecord(
            "object metadata must agree with its typed key".to_string(),
        ));
    }
    if record.object.media_type.trim().is_empty()
        || record.object.license.trim().is_empty()
        || record.object.provenance.trim().is_empty()
    {
        return Err(StoreError::InvalidRecord(
            "object media type, license, and provenance must not be empty".to_string(),
        ));
    }
    match (&record.scope, &record.object.key) {
        (
            AssetDeliveryScope::Catalog { asset, reference },
            ObjectKey::ProblemAsset {
                problem,
                version,
                asset: key_asset,
                object: _,
            },
        ) if record.id == AssetDeliveryId::from_asset(*asset)
            && *asset == *key_asset
            && reference.problem == *problem
            && reference.version == *version
            && record.object.bucket == Bucket::Content
            && record.object.category == ObjectCategory::Asset => {}
        (
            AssetDeliveryScope::StudentRecord {
                tenant,
                course: _,
                authorized_users,
            },
            ObjectKey::StudentRecord {
                tenant: key_tenant,
                object,
            },
        ) if record.id == AssetDeliveryId::from_object(*object)
            && *tenant == *key_tenant
            && record.object.bucket == Bucket::StudentRecords
            && record.object.category == ObjectCategory::Export =>
        {
            if authorized_users.is_empty() {
                return Err(StoreError::InvalidRecord(
                    "student-record delivery must authorize at least one user".to_string(),
                ));
            }
            let unique: std::collections::BTreeSet<_> = authorized_users.iter().copied().collect();
            if unique.len() != authorized_users.len() {
                return Err(StoreError::InvalidRecord(
                    "student-record authorized users must be unique".to_string(),
                ));
            }
        }
        _ => {
            return Err(StoreError::InvalidRecord(
                "only matching catalog assets and student-record exports may be delivered"
                    .to_string(),
            ));
        }
    }
    Ok(())
}

/// One current submitted result resolved against the current assignment scoring definition.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct CurrentRunQuestion {
    pub(crate) assignment_item: AssignmentItemId,
    pub(crate) result: AttemptResult,
    pub(crate) earned_points: f64,
    pub(crate) possible_points: f64,
}

/// Resolves the latest submitted attempt for every immutable delivered position.
pub(crate) fn current_run_questions(
    assignment: &AssignmentRecord,
    run_items: &[AssignmentRunItem],
    attempts: &[QuestionAttempt],
    current: &QuestionAttempt,
) -> Result<Vec<Option<CurrentRunQuestion>>, StoreError> {
    let mut delivered = run_items.iter().collect::<Vec<_>>();
    delivered.sort_by_key(|item| item.issued_position);
    for (position, item) in delivered.iter().enumerate() {
        if item.run != current.run || usize::try_from(item.issued_position).ok() != Some(position) {
            return Err(StoreError::InvalidRecord(
                "immutable run items must have contiguous issued positions".to_string(),
            ));
        }
    }
    let mut latest: Vec<Option<(ActivityTimestamp, QuestionAttemptId, CurrentRunQuestion)>> =
        vec![None; delivered.len()];
    for attempt in attempts
        .iter()
        .filter(|attempt| attempt.id != current.id)
        .chain(std::iter::once(current))
    {
        if attempt.run != current.run {
            return Err(StoreError::InvalidRecord(
                "attempt does not belong to the completed run".to_string(),
            ));
        }
        let position = usize::try_from(attempt.assignment_position).map_err(|_| {
            StoreError::InvalidRecord("attempt position is outside the delivered run".to_string())
        })?;
        let item = delivered.get(position).ok_or_else(|| {
            StoreError::InvalidRecord("attempt position is outside the delivered run".to_string())
        })?;
        if attempt.problem != item.reference.problem
            || attempt.question_version != item.reference.version
        {
            return Err(StoreError::InvalidRecord(
                "attempt identity disagrees with its immutable run item".to_string(),
            ));
        }
        let (Some(submitted_at), Some(result)) = (attempt.timer.submitted_at, attempt.result)
        else {
            continue;
        };
        let (earned_points, possible_points) =
            current_attempt_points(assignment, item.assignment_item, attempt.status, result)?;
        let question = CurrentRunQuestion {
            assignment_item: item.assignment_item,
            result,
            earned_points,
            possible_points,
        };
        let slot = &mut latest[position];
        if slot
            .as_ref()
            .is_none_or(|(at, id, _)| (submitted_at, attempt.id) > (*at, *id))
        {
            *slot = Some((submitted_at, attempt.id, question));
        }
    }
    Ok(latest
        .into_iter()
        .map(|entry| entry.map(|(_, _, question)| question))
        .collect())
}

/// Derives a completed run score from one current result per delivered position.
pub(crate) fn completed_run_score(
    questions: &[Option<CurrentRunQuestion>],
    requirement: question_model::CompletionRequirement,
) -> Result<Option<f64>, StoreError> {
    if questions.iter().any(Option::is_none) {
        return Ok(None);
    }
    let completion: Vec<_> = questions
        .iter()
        .map(|question| {
            let result = question
                .expect("missing results returned before projection")
                .result;
            RequiredQuestionState {
                answered: true,
                correct: result.correct,
                points_earned: result.points_earned,
                points_possible: result.points_possible,
            }
        })
        .collect();
    if derive_within_run_completion(&completion, requirement)? == WithinRunCompletion::InProgress {
        return Ok(None);
    }
    let earned: f64 = questions
        .iter()
        .map(|question| {
            question
                .expect("missing results returned before projection")
                .earned_points
        })
        .sum();
    let possible: f64 = questions
        .iter()
        .map(|question| {
            question
                .expect("missing results returned before projection")
                .possible_points
        })
        .sum();
    if !earned.is_finite() || !possible.is_finite() || possible < 0.0 {
        return Err(StoreError::RunModel(RunModelError::InvalidQuestionPoints));
    }
    let score = if possible > 0.0 {
        earned / possible
    } else {
        earned
    };
    if !score.is_finite() || !(-1_000.0..=1_000.0).contains(&score) {
        return Err(StoreError::RunModel(RunModelError::InvalidQuestionPoints));
    }
    Ok(Some(score))
}

/// Refuses malformed backend grades before they can enter attempt history.
pub(crate) fn validate_attempt_result(result: AttemptResult) -> Result<(), StoreError> {
    let credit = result.points_earned / result.points_possible;
    if !result.points_earned.is_finite()
        || !result.points_possible.is_finite()
        || result.points_possible <= 0.0
        || !credit.is_finite()
        || !(-1_000.0..=1_000.0).contains(&credit)
    {
        return Err(StoreError::InvalidRecord(
            "attempt result must have positive possible points and normalized credit from -1000 to 1000"
                .to_string(),
        ));
    }
    Ok(())
}

fn validate_title(kind: &str, title: &str) -> Result<(), StoreError> {
    const MAX_TITLE_CHARS: usize = 200;
    let trimmed = title.trim();
    if trimmed.is_empty() {
        return Err(StoreError::InvalidRecord(format!(
            "{kind} title must not be empty"
        )));
    }
    if trimmed.chars().count() > MAX_TITLE_CHARS {
        return Err(StoreError::InvalidRecord(format!(
            "{kind} title must contain at most {MAX_TITLE_CHARS} characters"
        )));
    }
    if trimmed != title {
        return Err(StoreError::InvalidRecord(format!(
            "{kind} title must not have leading or trailing whitespace"
        )));
    }
    Ok(())
}

/// Enforces the draft identity invariant before a backend writes bytes.
pub(crate) fn validate_draft(draft: &DraftRecord) -> Result<(), StoreError> {
    validate_question_policies(&draft.question)?;
    draft
        .question
        .metadata
        .validate_title()
        .map_err(|error| StoreError::InvalidRecord(error.to_string()))?;
    if draft.revises.is_some() && draft.derived_from.is_some() {
        return Err(StoreError::InvalidRecord(
            "draft cannot be both a revision and a new fork".to_string(),
        ));
    }
    Ok(())
}

/// Validates every database-side relationship in a QTI staging registry
/// before its transaction begins. Bytes are object-store authoritative and are
/// intentionally not accepted here.
pub(crate) fn validate_qti_import(command: &CreateQtiImportCommand) -> Result<(), StoreError> {
    const MAX_ITEMS: usize = 1_000;
    const MAX_ASSETS: usize = 10_000;
    const MAX_UNSUPPORTED: usize = 1_000;
    let registry = &command.registry;
    validate_qti_text("parse schema", &registry.parse_schema, 160)?;
    validate_qti_text("adapter version", &registry.adapter_version, 160)?;
    if registry.items.is_empty() || registry.items.len() > MAX_ITEMS {
        return Err(StoreError::InvalidRecord(
            "QTI import has an invalid item count".to_string(),
        ));
    }
    if registry.assets.len() > MAX_ASSETS || registry.unsupported_features.len() > MAX_UNSUPPORTED {
        return Err(StoreError::InvalidRecord(
            "QTI import exceeds bounded registry limits".to_string(),
        ));
    }
    validate_workspace_source(&registry.source, registry.reference)?;
    let mut assets = std::collections::BTreeSet::new();
    for asset in &registry.assets {
        validate_workspace_asset(asset, registry.reference)?;
        let ObjectKey::WorkspaceAsset {
            asset: logical_asset,
            ..
        } = &asset.key
        else {
            return Err(StoreError::InvalidRecord(
                "QTI asset is missing its logical identity".to_string(),
            ));
        };
        if !assets.insert(*logical_asset) {
            return Err(StoreError::InvalidRecord(
                "QTI import repeats a logical asset".to_string(),
            ));
        }
    }
    let mut item_ids = std::collections::BTreeSet::new();
    for item in &registry.items {
        validate_qti_text("item id", &item.item_id, 512)?;
        if !item_ids.insert(item.item_id.as_str()) {
            return Err(StoreError::InvalidRecord(
                "QTI import repeats an item id".to_string(),
            ));
        }
        let mut item_assets = std::collections::BTreeSet::new();
        for asset in &item.assets {
            if !assets.contains(asset) || !item_assets.insert(*asset) {
                return Err(StoreError::InvalidRecord(
                    "QTI item references a missing or repeated staged asset".to_string(),
                ));
            }
        }
    }
    if command.item_bindings.len() != registry.items.len() {
        return Err(StoreError::InvalidRecord(
            "every QTI item requires exactly one server-only grading binding".to_string(),
        ));
    }
    let bound = command
        .item_bindings
        .iter()
        .map(|binding| binding.item.item_id.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    if bound.len() != command.item_bindings.len()
        || bound != item_ids
        || command.item_bindings.iter().any(|binding| {
            registry
                .items
                .iter()
                .find(|item| item.item_id == binding.item.item_id)
                != Some(&binding.item)
        })
    {
        return Err(StoreError::InvalidRecord(
            "QTI grading bindings must exactly match immutable item records".to_string(),
        ));
    }
    for feature in &registry.unsupported_features {
        validate_qti_text("unsupported feature code", &feature.code, 160)?;
        validate_qti_text("unsupported feature location", &feature.location, 1_024)?;
    }
    Ok(())
}

/// Validates the browser-inaccessible evidence supplied by the dedicated QTI
/// publication route. The caller separately loads `registry` from committed
/// private staging while holding its publication transaction open.
pub(crate) fn validate_qti_publication_promotion(
    context: TenantContext,
    command: &PublishDraftCommand,
    promotion: &QtiPublicationPromotion,
    registry: &QtiImportRegistry,
) -> Result<(), StoreError> {
    let (draft_item, draft_import) = match &command.expected_draft.question.source {
        DraftQuestionSource::Qti { item_id, import_id } => (item_id, import_id),
        _ => {
            return Err(StoreError::InvalidRecord(
                "QTI promotion requires a QTI draft".to_string(),
            ));
        }
    };
    let QuestionSource::Qti {
        item_id,
        package_object,
        package_sha256,
    } = &command.published_source
    else {
        return Err(StoreError::InvalidRecord(
            "QTI promotion requires a QTI published source".to_string(),
        ));
    };
    if item_id != draft_item
        || promotion.staging.tenant != context.tenant_id()
        || promotion.staging.workspace != command.expected_draft.question.workspace
        || promotion.staging.import != *draft_import
        || registry.reference != promotion.staging
        || !registry
            .items
            .iter()
            .any(|item| item.item_id == *draft_item)
    {
        return Err(StoreError::Conflict);
    }
    let artifact = command.source_artifact.as_ref().ok_or_else(|| {
        StoreError::InvalidRecord("QTI promotion requires a copied source artifact".to_string())
    })?;
    if *package_object != artifact.object.id
        || package_sha256 != &artifact.object.sha256.to_string()
        || artifact.object.sha256 != registry.source.sha256
        || artifact.object.size_bytes != registry.source.size_bytes
        || artifact.object.media_type != registry.source.media_type
    {
        return Err(StoreError::Conflict);
    }

    let staged_item = registry
        .items
        .iter()
        .find(|item| item.item_id == *draft_item)
        .expect("checked QTI item is present");
    let expected_assets: std::collections::BTreeMap<_, _> = registry
        .assets
        .iter()
        .filter_map(|asset| match &asset.key {
            ObjectKey::WorkspaceAsset { asset: id, .. } if staged_item.assets.contains(id) => {
                Some((*id, asset))
            }
            _ => None,
        })
        .collect();
    if expected_assets.len() != staged_item.assets.len()
        || promotion.assets.len() != expected_assets.len()
    {
        return Err(StoreError::Conflict);
    }
    let mut actual_assets = std::collections::BTreeSet::new();
    for delivery in &promotion.assets {
        validate_asset_delivery(delivery)?;
        let AssetDeliveryScope::Catalog { asset, reference } = delivery.scope else {
            return Err(StoreError::InvalidRecord(
                "QTI promotion assets must be catalog assets".to_string(),
            ));
        };
        if reference != command.publication || !actual_assets.insert(asset) {
            return Err(StoreError::Conflict);
        }
        let staged = expected_assets.get(&asset).ok_or(StoreError::Conflict)?;
        if delivery.object.sha256 != staged.sha256
            || delivery.object.size_bytes != staged.size_bytes
            || delivery.object.media_type != staged.media_type
        {
            return Err(StoreError::Conflict);
        }
    }
    Ok(())
}

fn validate_qti_text(name: &str, value: &str, max: usize) -> Result<(), StoreError> {
    if value.is_empty() || value.len() > max || value.bytes().any(|byte| byte.is_ascii_control()) {
        return Err(StoreError::InvalidRecord(format!("QTI {name} is invalid")));
    }
    Ok(())
}

fn validate_workspace_source(
    record: &ObjectRecord,
    reference: QtiImportRef,
) -> Result<(), StoreError> {
    if record.id != record.key.object_id()
        || record.bucket != Bucket::Content
        || record.key.bucket() != Bucket::Content
        || record.category != ObjectCategory::Source
        || record.key.category() != ObjectCategory::Source
        || record.version.is_some()
        || record.key.version_id().is_some()
        || record.media_type != "application/zip"
        || !matches!(record.key, ObjectKey::WorkspaceSource { tenant, workspace, import, .. }
            if tenant == reference.tenant && workspace == reference.workspace && import == reference.import)
    {
        return Err(StoreError::InvalidRecord(
            "QTI source record is not an exact workspace ZIP".to_string(),
        ));
    }
    validate_object_annotations(record)
}

fn validate_workspace_asset(
    record: &ObjectRecord,
    reference: QtiImportRef,
) -> Result<(), StoreError> {
    if record.id != record.key.object_id()
        || record.bucket != Bucket::Content
        || record.key.bucket() != Bucket::Content
        || record.category != ObjectCategory::Asset
        || record.key.category() != ObjectCategory::Asset
        || record.version.is_some()
        || record.key.version_id().is_some()
        || record.media_type.is_empty()
        || !matches!(record.key, ObjectKey::WorkspaceAsset { tenant, workspace, import, .. }
            if tenant == reference.tenant && workspace == reference.workspace && import == reference.import)
    {
        return Err(StoreError::InvalidRecord(
            "QTI asset record is not an exact workspace asset".to_string(),
        ));
    }
    validate_object_annotations(record)
}

fn validate_object_annotations(record: &ObjectRecord) -> Result<(), StoreError> {
    validate_qti_text("object media type", &record.media_type, 255)?;
    validate_qti_text("object license", &record.license, 512)?;
    validate_qti_text("object provenance", &record.provenance, 2_048)
}

/// Ensures the server-prepared immutable source is for the exact draft being
/// published. This prevents a caller from attaching a snapshot from another
/// backend or iMathAS item while the draft is still tenant-owned.
pub(crate) fn validate_publication_source(
    draft: &DraftRecord,
    source: &question_model::QuestionSource,
) -> Result<(), StoreError> {
    source
        .validate()
        .map_err(|error| StoreError::InvalidRecord(error.to_string()))?;
    if question_model::QuestionBackend::from(&draft.question.source)
        != question_model::QuestionBackend::from(source)
    {
        return Err(StoreError::InvalidRecord(
            "published source backend must match the draft source".to_string(),
        ));
    }
    match (&draft.question.source, source) {
        (
            question_model::DraftQuestionSource::Imathas { provider, item_ref },
            question_model::QuestionSource::Imathas {
                provider: published_provider,
                item_ref: published_item,
                ..
            },
        ) if provider == published_provider && item_ref == published_item => Ok(()),
        (question_model::DraftQuestionSource::Imathas { .. }, _) => Err(StoreError::InvalidRecord(
            "iMathAS publication must pin the draft provider and item in its snapshot".to_string(),
        )),
        (
            question_model::DraftQuestionSource::Qti { item_id, .. },
            question_model::QuestionSource::Qti {
                item_id: published_item,
                ..
            },
        ) if item_id == published_item => Ok(()),
        (question_model::DraftQuestionSource::Qti { .. }, _) => Err(StoreError::InvalidRecord(
            "QTI publication must preserve the staged import item identity".to_string(),
        )),
        _ => Ok(()),
    }
}

/// Validates the server-prepared source object before publication can create
/// any visible immutable identity.
pub(crate) fn validate_source_artifact(
    publication: ProblemVersionRef,
    source: &question_model::QuestionSource,
    artifact: Option<&PublishedSourceArtifact>,
) -> Result<(), StoreError> {
    let backend = QuestionBackend::from(source);
    let requires_artifact = !matches!(backend, QuestionBackend::Native);
    let Some(artifact) = artifact else {
        return if requires_artifact {
            Err(StoreError::InvalidRecord(
                "source-backed publication requires an immutable source artifact".to_string(),
            ))
        } else {
            Ok(())
        };
    };
    if !requires_artifact {
        return Err(StoreError::InvalidRecord(
            "native publication must not attach a source artifact".to_string(),
        ));
    }
    validate_source_artifact_identity(publication, backend, artifact)?;
    if let question_model::QuestionSource::Imathas {
        snapshot,
        snapshot_sha256,
        ..
    } = source
        && (*snapshot != artifact.object.id
            || snapshot_sha256 != &artifact.object.sha256.to_string())
    {
        return Err(StoreError::InvalidRecord(
            "iMathAS snapshot must match the immutable source artifact".to_string(),
        ));
    }
    if let question_model::QuestionSource::Qti {
        package_object,
        package_sha256,
        ..
    } = source
        && (*package_object != artifact.object.id
            || package_sha256 != &artifact.object.sha256.to_string())
    {
        return Err(StoreError::InvalidRecord(
            "QTI package must match the immutable source artifact".to_string(),
        ));
    }
    Ok(())
}

/// Checks the object-record half of a source binding. Store resolvers repeat
/// this before returning a decoded database payload.
pub(crate) fn validate_source_artifact_identity(
    publication: ProblemVersionRef,
    backend: QuestionBackend,
    artifact: &PublishedSourceArtifact,
) -> Result<(), StoreError> {
    if artifact.reference != publication || artifact.backend != backend {
        return Err(StoreError::InvalidRecord(
            "source artifact must bind the exact published backend and version".to_string(),
        ));
    }
    let ObjectKey::ProblemSource {
        problem,
        version,
        object,
    } = artifact.object.key
    else {
        return Err(StoreError::InvalidRecord(
            "source artifact must use a published problem-source key".to_string(),
        ));
    };
    if problem != publication.problem
        || version != publication.version
        || object != artifact.object.id
        || artifact.object.bucket != Bucket::Content
        || artifact.object.category != ObjectCategory::Source
        || artifact.object.version != Some(publication.version)
        || artifact.object.key.bucket() != Bucket::Content
        || artifact.object.key.category() != ObjectCategory::Source
        || artifact.object.size_bytes == 0
        || artifact.object.media_type.trim().is_empty()
        || artifact.object.license.trim().is_empty()
        || artifact.object.provenance.trim().is_empty()
    {
        return Err(StoreError::InvalidRecord(
            "source artifact metadata does not match its immutable object key".to_string(),
        ));
    }
    Ok(())
}

/// Enforces published identity agreement before immutable insertion.
pub(crate) fn validate_published(record: &PublishedProblemRecord) -> Result<(), StoreError> {
    if record.question.problem != record.problem || record.question.version != record.version {
        return Err(StoreError::InvalidRecord(
            "published record IDs must match its question definition".to_string(),
        ));
    }
    if record.authors.is_empty() {
        return Err(StoreError::InvalidRecord(
            "published problem must have at least one author".to_string(),
        ));
    }
    validate_question_policies(&record.question)?;
    record
        .question
        .metadata
        .validate_title()
        .map_err(|error| StoreError::InvalidRecord(error.to_string()))?;
    let mut authors = record.authors.clone();
    authors.sort_unstable();
    authors.dedup();
    if authors.len() != record.authors.len() {
        return Err(StoreError::InvalidRecord(
            "published problem authors must be unique".to_string(),
        ));
    }
    if record
        .previous_version
        .is_some_and(|previous| previous == record.version)
    {
        return Err(StoreError::InvalidRecord(
            "published version cannot revise itself".to_string(),
        ));
    }
    Ok(())
}

trait QuestionPolicyView {
    fn attempt_policy(&self) -> &question_model::run_policy::AttemptPolicy;
}

impl QuestionPolicyView for QuestionDefinition {
    fn attempt_policy(&self) -> &question_model::run_policy::AttemptPolicy {
        &self.attempt_policy
    }
}

impl QuestionPolicyView for DraftQuestionDefinition {
    fn attempt_policy(&self) -> &question_model::run_policy::AttemptPolicy {
        &self.attempt_policy
    }
}

fn validate_question_policies(question: &impl QuestionPolicyView) -> Result<(), StoreError> {
    if question.attempt_policy().max_attempts == Some(0) {
        return Err(StoreError::InvalidRecord(
            "question max attempts must be greater than zero".to_string(),
        ));
    }
    Ok(())
}

/// Maintains enrollment completion and grade-run pointers with a summary.
pub(crate) fn project_enrollment_completion(
    enrollment: &mut AssignmentEnrollment,
    previous: &StudentAssignmentSummary,
    grade: GradePolicy,
    run: RunId,
    score: f64,
    at: ActivityTimestamp,
) {
    let is_first_completion = previous.completed_run_count == 0;
    let is_new_best = previous.best_score.is_none_or(|best| score > best);

    if enrollment.first_completed_at.is_none() {
        enrollment.first_completed_at = Some(at);
    }
    if is_new_best || enrollment.best_grade_run.is_none() {
        enrollment.best_grade_run = Some(run);
    }
    enrollment.current_grade_run = match grade {
        GradePolicy::First if is_first_completion => Some(run),
        GradePolicy::First => enrollment.current_grade_run,
        GradePolicy::Latest => Some(run),
        GradePolicy::Highest if is_new_best => Some(run),
        GradePolicy::Highest => enrollment.current_grade_run,
        GradePolicy::InstructorSelected => enrollment.current_grade_run,
    };
}
