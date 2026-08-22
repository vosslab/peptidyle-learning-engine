//! Durable, tenant-attributed background-work queue contract.
//!
//! A job is deliberately not a closure or a generic JSON command.  Its
//! payload contains only immutable identifiers that a server-side handler can
//! resolve under the returned tenant context.  Credentials, answer keys,
//! arbitrary URLs, and raw bytes never enter this durable queue.

use crate::RetentionStage;
use async_trait::async_trait;
use objects::ObjectRecord;
use question_model::{
    AssignmentId, CourseId, ObjectId, ProblemVersionRef, QuestionAttemptId, ScoringGeneration,
    TenantId, UserId, WorkspaceId, WorkspaceImportId,
};
use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{StoreError, TenantContext};

/// Opaque tenant-owned identifier for one assignment-export request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ExportId(Uuid);

impl ExportId {
    /// Mints a server-side opaque export identity.
    pub fn generate() -> Result<Self, StoreError> {
        crate::random_uuid::random_uuid_v4(|error| {
            StoreError::Unavailable(format!("export ID randomness unavailable: {error}"))
        })
        .map(Self)
    }

    /// Reconstitutes an ID read from storage or a validated route parameter.
    pub fn from_uuid(value: Uuid) -> Self {
        Self(value)
    }

    /// Returns the database representation.
    pub fn as_uuid(self) -> Uuid {
        self.0
    }
}

impl std::fmt::Display for ExportId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}", self.0)
    }
}

/// The closed, fixed output set for one export job.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ExportArtifactKind {
    /// Standard DOCX exam.
    Docx,
    /// Standard PDF exam.
    Pdf,
    /// Accessible DOCX exam.
    AccessibleDocx,
    /// Accessible PDF exam.
    AccessiblePdf,
}

impl ExportArtifactKind {
    /// The four required outputs in canonical display and validation order.
    pub const ALL: [Self; 4] = [
        Self::Docx,
        Self::Pdf,
        Self::AccessibleDocx,
        Self::AccessiblePdf,
    ];

    /// The only media type accepted for this deterministic output kind.
    pub const fn media_type(self) -> &'static str {
        match self {
            Self::Docx | Self::AccessibleDocx => {
                "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
            }
            Self::Pdf | Self::AccessiblePdf => "application/pdf",
        }
    }
}

/// A server-only request to freeze an assignment and enqueue one bundle job.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CreateAssignmentExport {
    /// Tenant-owned assignment selected by the authenticated route.
    pub assignment: AssignmentId,
    /// Bounded queue retry budget.
    pub max_attempts: u16,
}

/// Browser-safe lifecycle state for an export request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum StudentExportState {
    /// The bundle is pending or currently leased; neither distinction leaks worker state.
    Queued,
    /// All four artifacts and their delivery records committed atomically.
    Ready,
    /// The worker permanently refused the frozen request.
    Failed,
}

/// Browser-safe descriptor for one ready protected download.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StudentExportArtifactView {
    /// Closed rendition kind.
    pub kind: ExportArtifactKind,
    /// Server-selected stable download filename.
    pub filename: String,
    /// Verified output media type.
    pub media_type: String,
    /// Existing protected asset-delivery route identifier.
    pub delivery: crate::AssetDeliveryId,
}

/// Browser-safe export status. It never contains object keys, manifests, leases, or frozen source refs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StudentExportView {
    /// Route identity for this export request.
    pub id: ExportId,
    /// Owning assignment, useful for route-side authorization.
    pub assignment: AssignmentId,
    /// Coarse status safe for browser display.
    pub state: StudentExportState,
    /// Present only after all four artifacts are ready.
    pub artifacts: Option<Vec<StudentExportArtifactView>>,
}

/// Private frozen worker input resolved only from a claimed manifest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StudentExportJob {
    /// Immutable request identity.
    pub id: ExportId,
    /// Tenant inherited from the active queue lease.
    pub tenant: TenantId,
    /// Assignment selected at request creation.
    pub assignment: AssignmentId,
    /// Course that owned the assignment at request creation.
    pub course: CourseId,
    /// Human-facing assignment title frozen with the selected problem versions.
    pub title: String,
    /// User authorized for the initial protected downloads.
    pub requested_by: UserId,
    /// Closed queue payload value, never sent to the browser.
    pub manifest: ObjectId,
    /// Exact ordered published versions frozen before queue insertion.
    pub problems: Vec<ProblemVersionRef>,
    /// The exact four private object identities expected from the renderer.
    pub expected_artifacts: Vec<(ExportArtifactKind, ObjectId)>,
}

/// One immutable artifact written by the worker before the database commit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ExportArtifactRecord {
    /// Closed rendition kind.
    pub kind: ExportArtifactKind,
    /// Server-selected file name, checked against the closed kind.
    pub filename: String,
    /// Verified object metadata; bytes remain in object storage.
    pub object: ObjectRecord,
}

/// Private atomically committed effect for one current export lease.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportJobCommit {
    /// Claimed durable job.
    pub job: JobId,
    /// Current opaque lease capability.
    pub lease: JobLeaseToken,
    /// Exact closed export manifest from the queue payload.
    pub manifest: ObjectId,
    /// Exactly the four closed artifacts.
    pub artifacts: Vec<ExportArtifactRecord>,
}

/// Result of a commit request. Same-job exact replay is deliberately harmless.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportCommitDisposition {
    /// This invocation made the export visible and completed its job.
    Committed,
    /// The same job had already committed exactly this immutable effect.
    AlreadyCommitted,
}

/// Opaque durable identity for one queued unit of work.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct JobId(Uuid);

impl JobId {
    /// Mints a server-side opaque queue identity.
    pub fn generate() -> Result<Self, StoreError> {
        crate::random_uuid::random_uuid_v4(|error| {
            StoreError::Unavailable(format!("job ID randomness unavailable: {error}"))
        })
        .map(Self)
    }

    /// Reconstitutes an ID read from storage.
    pub fn from_uuid(value: Uuid) -> Self {
        Self(value)
    }

    /// Returns the database representation.
    pub fn as_uuid(self) -> Uuid {
        self.0
    }
}

impl std::fmt::Display for JobId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}", self.0)
    }
}

/// Per-claim capability required to complete or fail a leased job.
///
/// This value is never a browser value and is replaced when an expired lease
/// is reclaimed.  It intentionally has no string parser or serializer for an
/// HTTP boundary.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct JobLeaseToken(Uuid);

impl JobLeaseToken {
    /// Creates an unpredictable worker-local claim token.
    pub fn generate() -> Result<Self, StoreError> {
        crate::random_uuid::random_128_bits(|error| {
            StoreError::Unavailable(format!("job lease randomness unavailable: {error}"))
        })
        .map(crate::random_uuid::uuid_storage_from_128_random_bits)
        .map(Self)
    }

    /// Returns the database representation for the private broker call.
    #[cfg(feature = "postgres")]
    pub(crate) fn as_uuid(self) -> Uuid {
        self.0
    }

    #[cfg(feature = "postgres")]
    pub(crate) fn from_uuid(value: Uuid) -> Self {
        Self(value)
    }
}

impl std::fmt::Debug for JobLeaseToken {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("JobLeaseToken(<redacted>)")
    }
}

/// Closed immutable references accepted by the initial queue.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum JobPayload {
    /// Rebuild all current computed scores for one assignment generation.
    /// Student, attempt, response, and grade data remain out of the queue.
    RecalculateAssignment {
        /// Tenant-owned assignment resolved by the worker under its claim context.
        assignment: AssignmentId,
        /// Stale-work fence; only this still-current generation may commit.
        generation: ScoringGeneration,
    },
    /// Rebuild the current tenant-owned course item analysis after scoring
    /// publication. The worker derives every learner record under its lease.
    RecalculateCourseItemAnalysis {
        assignment: AssignmentId,
        generation: ScoringGeneration,
    },
    /// Close one active attempt after its current effective deadline.
    /// The worker rechecks the current timing generation and never carries a
    /// response, grade, student identity, or authored answer material.
    AutoSubmitAttempt {
        /// Tenant-owned attempt resolved under the queue claim context.
        attempt: QuestionAttemptId,
        /// Stale-work fence advanced by every active timing-policy change.
        timing_generation: u64,
    },
    /// Process one private retention lifecycle stage. The Store resolves any
    /// notification recipients and exact object manifest; no learner or
    /// object identity is durable queue data.
    Retention {
        /// Ended course whose current schedule owns this work.
        course: CourseId,
        /// Closed lifecycle stage; this is not an HTTP input.
        stage: RetentionStage,
        /// Positive stale-work fence from the course schedule snapshot.
        generation: u64,
    },
    /// Populate a deterministic server-side render cache for one issued seed.
    Render {
        /// Exact immutable published content.
        reference: ProblemVersionRef,
        /// Server-generated instance seed.
        seed: u64,
    },
    /// Produce a tenant-authorized export whose target record already exists.
    Export {
        /// Immutable tenant-owned target object identity.
        delivery_object: ObjectId,
    },
    /// Import a previously stored, immutable source object.
    Import {
        /// Exact immutable source archive or provider snapshot.
        source_object: ObjectId,
    },
    /// Parse a private QTI archive already stored under a typed workspace key.
    QtiImport {
        workspace: WorkspaceId,
        import: WorkspaceImportId,
        source_object: ObjectId,
    },
    /// Materialize committed public catalog assets. The worker re-resolves
    /// every typed key and private source from the durable registry; neither
    /// bucket paths nor bytes are queue payload data.
    PublishPublicAssets {
        /// Exact immutable version whose public assets remain pending.
        reference: ProblemVersionRef,
    },
}

/// Closed queue family used to bind a worker process to only the work it can finish.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum JobKind {
    RecalculateAssignment,
    RecalculateCourseItemAnalysis,
    AutoSubmitAttempt,
    Retention,
    Render,
    Export,
    Import,
    QtiImport,
    PublishPublicAssets,
}

impl JobKind {
    pub const ALL: [Self; 9] = [
        Self::RecalculateAssignment,
        Self::RecalculateCourseItemAnalysis,
        Self::AutoSubmitAttempt,
        Self::Retention,
        Self::Render,
        Self::Export,
        Self::Import,
        Self::QtiImport,
        Self::PublishPublicAssets,
    ];

    /// Exact tagged-enum discriminant persisted in `worker_job.payload`.
    pub const fn database_name(self) -> &'static str {
        match self {
            Self::RecalculateAssignment => "recalculateAssignment",
            Self::RecalculateCourseItemAnalysis => "recalculateCourseItemAnalysis",
            Self::AutoSubmitAttempt => "autoSubmitAttempt",
            Self::Retention => "retention",
            Self::Render => "render",
            Self::Export => "export",
            Self::Import => "import",
            Self::QtiImport => "qtiImport",
            Self::PublishPublicAssets => "publishPublicAssets",
        }
    }
}

impl JobPayload {
    pub const fn kind(&self) -> JobKind {
        match self {
            Self::RecalculateAssignment { .. } => JobKind::RecalculateAssignment,
            Self::RecalculateCourseItemAnalysis { .. } => JobKind::RecalculateCourseItemAnalysis,
            Self::AutoSubmitAttempt { .. } => JobKind::AutoSubmitAttempt,
            Self::Retention { .. } => JobKind::Retention,
            Self::Render { .. } => JobKind::Render,
            Self::Export { .. } => JobKind::Export,
            Self::Import { .. } => JobKind::Import,
            Self::QtiImport { .. } => JobKind::QtiImport,
            Self::PublishPublicAssets { .. } => JobKind::PublishPublicAssets,
        }
    }
}

/// Nonempty, duplicate-free family allowlist for one queue claimant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JobClaimFilter {
    kinds: BTreeSet<JobKind>,
}

impl JobClaimFilter {
    pub fn new(kinds: impl IntoIterator<Item = JobKind>) -> Result<Self, StoreError> {
        let kinds = kinds.into_iter().collect::<BTreeSet<_>>();
        if kinds.is_empty() {
            return Err(StoreError::InvalidRecord(
                "worker claim filter must contain at least one job family".to_string(),
            ));
        }
        Ok(Self { kinds })
    }

    pub fn all() -> Self {
        Self {
            kinds: JobKind::ALL.into_iter().collect(),
        }
    }

    pub fn contains(&self, kind: JobKind) -> bool {
        self.kinds.contains(&kind)
    }

    pub fn kinds(&self) -> impl ExactSizeIterator<Item = JobKind> + '_ {
        self.kinds.iter().copied()
    }

    #[cfg(feature = "postgres")]
    pub(crate) fn database_names(&self) -> Vec<&'static str> {
        self.kinds().map(JobKind::database_name).collect::<Vec<_>>()
    }
}

/// Validated request to enqueue one job under the current tenant.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EnqueueJob {
    /// Tenant is explicit so a producer cannot accidentally enqueue globally.
    pub tenant: TenantId,
    /// Closed, key-free immutable work reference.
    pub payload: JobPayload,
    /// Total claim attempts allowed before a job becomes dead.
    pub max_attempts: u16,
}

impl EnqueueJob {
    /// The queue remains intentionally bounded rather than becoming a retry DSL.
    pub fn validate(&self) -> Result<(), StoreError> {
        if !(1..=20).contains(&self.max_attempts) {
            return Err(StoreError::InvalidRecord(
                "job max attempts must be between 1 and 20".to_string(),
            ));
        }
        Ok(())
    }
}

/// A bounded lease duration. Initial workers do not renew leases.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JobLeaseDuration(u32);

impl JobLeaseDuration {
    /// Parses a bounded lease shorter than an initial handler deadline.
    pub fn from_seconds(seconds: u32) -> Result<Self, StoreError> {
        if !(1..=900).contains(&seconds) {
            return Err(StoreError::InvalidRecord(
                "job lease duration must be between 1 and 900 seconds".to_string(),
            ));
        }
        Ok(Self(seconds))
    }

    pub(crate) fn seconds(self) -> i32 {
        i32::try_from(self.0).expect("validated job lease fits PostgreSQL integer")
    }
}

/// A job atomically leased to exactly one worker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClaimedJob {
    /// Durable queue identity.
    pub id: JobId,
    /// Required context for every handler-side storage operation.
    pub tenant: TenantId,
    /// Safe immutable work reference.
    pub payload: JobPayload,
    /// Opaque capability for this lease only.
    pub lease_token: JobLeaseToken,
    /// Claim count after this lease was acquired.
    pub attempt_count: u16,
}

/// Redacted classification retained for an exhausted job.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobFailureKind {
    /// A bounded dependency outage or worker crash.
    Transient,
    /// Input was valid enough to queue but cannot be processed.
    Permanent,
    /// Handler exceeded its bounded execution deadline.
    TimedOut,
}

impl JobFailureKind {
    #[cfg(feature = "postgres")]
    pub(crate) const fn as_db(self) -> &'static str {
        match self {
            Self::Transient => "transient",
            Self::Permanent => "permanent",
            Self::TimedOut => "timed_out",
        }
    }
}

/// Result of an accepted failure report.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobFailureDisposition {
    /// The job is delayed for a bounded retry.
    Retrying,
    /// Attempts are exhausted or the failure is permanent.
    Dead,
}

/// Claimable backlog only; delayed, leased, completed, and dead jobs are absent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QueueDepth {
    /// Jobs ready at the database transaction timestamp.
    pub ready: u64,
}

/// Tenant-safe inspection projection. It excludes claim tokens and failure text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TenantJobView {
    /// Durable queue identity.
    pub id: JobId,
    /// Immutable safe work reference.
    pub payload: JobPayload,
    /// Current queue state for diagnostics; no global listing is provided.
    pub state: JobState,
    /// Number of completed claim attempts.
    pub attempt_count: u16,
}

/// Small stable lifecycle vocabulary for tenant inspection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobState {
    /// Eligible when its availability timestamp is reached.
    Ready,
    /// Exclusively held by a worker until lease expiry.
    Leased,
    /// Successfully finalized.
    Completed,
    /// Exhausted without exposing error text.
    Dead,
}

/// Queue storage kept separate from [`crate::Store`].
#[async_trait]
pub trait JobStore: Send + Sync {
    /// Atomically inserts a tenant-attributed ready job.
    async fn enqueue_job(
        &self,
        context: TenantContext,
        job: EnqueueJob,
    ) -> Result<JobId, StoreError>;

    /// Claims one eligible job across tenants through the dedicated queue broker.
    async fn claim_next_job(
        &self,
        filter: &JobClaimFilter,
        lease: JobLeaseDuration,
    ) -> Result<Option<ClaimedJob>, StoreError>;

    /// Completes the current claim only; stale tokens are rejected.
    async fn complete_job(&self, id: JobId, token: JobLeaseToken) -> Result<(), StoreError>;

    /// Applies bounded backoff or marks the current claim dead.
    async fn fail_job(
        &self,
        id: JobId,
        token: JobLeaseToken,
        failure: JobFailureKind,
    ) -> Result<JobFailureDisposition, StoreError>;

    /// Reads only an active tenant's own job, never a queue listing.
    async fn get_job(
        &self,
        context: TenantContext,
        id: JobId,
    ) -> Result<Option<TenantJobView>, StoreError>;

    /// Returns operational backlog through the narrow queue broker.
    async fn ready_queue_depth(&self, filter: &JobClaimFilter) -> Result<QueueDepth, StoreError>;
}

/// Atomic persistence boundary for the four-artifact assignment export producer.
///
/// Object bytes are deliberately written before [`commit_export_effect`](Self::commit_export_effect).
/// This trait then makes all four protected delivery rows, the ready result, and job completion
/// visible together; a failed or stale lease can leave only unreachable orphan objects.
#[async_trait]
pub trait ExportJobStore: Send + Sync {
    /// Freezes the current assignment's ordered published versions, allocates the manifest and all
    /// four object identities, and atomically creates its one queue job.
    async fn create_assignment_export(
        &self,
        context: TenantContext,
        // Resolved at the same storage boundary as assignment authority.
        session: crate::SessionTokenHash,
        request: CreateAssignmentExport,
    ) -> Result<StudentExportView, StoreError>;

    /// Reads a tenant-owned browser-safe status projection.
    async fn get_assignment_export(
        &self,
        context: TenantContext,
        export: ExportId,
    ) -> Result<Option<StudentExportView>, StoreError>;

    /// Reads a status projection only when it belongs to the authenticated requestor. Course-wide
    /// instructor or sysadmin sharing is deliberately a separate policy, never an accidental
    /// consequence of knowing an export ID.
    async fn get_assignment_export_for_requester(
        &self,
        context: TenantContext,
        export: ExportId,
        requester: UserId,
    ) -> Result<Option<StudentExportView>, StoreError>;

    /// Resolves private frozen worker input for one claimed tenant and manifest.
    async fn load_export_job(
        &self,
        context: TenantContext,
        manifest: ObjectId,
    ) -> Result<Option<StudentExportJob>, StoreError>;

    /// Atomically registers exactly four protected artifacts, records the ready result, and
    /// completes the matching active lease. Same-job exact replay returns `AlreadyCommitted`.
    async fn commit_export_effect(
        &self,
        context: TenantContext,
        commit: ExportJobCommit,
    ) -> Result<ExportCommitDisposition, StoreError>;
}

#[cfg(test)]
mod tests {
    use question_model::{ObjectId, QuestionAttemptId, WorkspaceId, WorkspaceImportId};
    use serde_json::json;
    use uuid::Uuid;

    use super::{EnqueueJob, JobClaimFilter, JobKind, JobPayload};

    #[test]
    fn claim_filter_is_nonempty_deduplicated_and_uses_exact_database_names() {
        assert!(JobClaimFilter::new([]).is_err());
        let filter = JobClaimFilter::new([JobKind::Export, JobKind::QtiImport, JobKind::Export])
            .expect("nonempty filter");
        assert_eq!(
            filter.kinds().collect::<Vec<_>>(),
            [JobKind::Export, JobKind::QtiImport]
        );
        assert!(filter.contains(JobKind::Export));
        assert!(!filter.contains(JobKind::Render));
        #[cfg(feature = "postgres")]
        assert_eq!(filter.database_names(), ["export", "qtiImport"]);
    }

    #[test]
    fn durable_payload_rejects_unrecognized_or_secret_shaped_fields() {
        let reference = json!({
            "problem": "00000000-0000-0000-0000-000000000001",
            "version": "00000000-0000-0000-0000-000000000002"
        });
        for field in ["url", "token", "rawBytes", "answerKey"] {
            let mut payload = json!({
                "kind": "render",
                "reference": reference,
                "seed": 7
            });
            payload
                .as_object_mut()
                .expect("fixture object")
                .insert(field.to_string(), json!("must not persist"));
            assert!(
                serde_json::from_value::<JobPayload>(payload).is_err(),
                "{field} must not be accepted by the durable payload decoder"
            );
        }
    }

    #[test]
    fn enqueue_command_rejects_unknown_wire_fields() {
        let value = json!({
            "tenant": "00000000-0000-0000-0000-000000000003",
            "payload": {
                "kind": "import",
                "sourceObject": "00000000-0000-0000-0000-000000000004"
            },
            "maxAttempts": 2,
            "url": "https://not-a-job-field.invalid"
        });
        assert!(serde_json::from_value::<EnqueueJob>(value).is_err());
    }

    #[test]
    fn retention_payload_is_closed_and_key_free() {
        let valid = json!({
            "kind": "retention",
            "course": "00000000-0000-0000-0000-000000000001",
            "stage": "archiveStudentRecords",
            "generation": 1
        });
        assert!(serde_json::from_value::<JobPayload>(valid.clone()).is_ok());
        for field in [
            "student",
            "object",
            "objectKey",
            "url",
            "payload",
            "delivery",
        ] {
            let mut invalid = valid.clone();
            invalid
                .as_object_mut()
                .expect("fixture object")
                .insert(field.into(), json!("forbidden"));
            assert!(
                serde_json::from_value::<JobPayload>(invalid).is_err(),
                "{field} must not enter retention queue work"
            );
        }
    }

    #[test]
    fn auto_submit_payload_is_a_minimal_numeric_generation_fence() {
        let payload = JobPayload::AutoSubmitAttempt {
            attempt: QuestionAttemptId::from_uuid(Uuid::from_u128(1)),
            timing_generation: 1,
        };
        assert_eq!(
            serde_json::to_value(payload).expect("payload serialization"),
            json!({
                "kind": "autoSubmitAttempt",
                "attempt": "00000000-0000-0000-0000-000000000001",
                "timing_generation": 1
            })
        );
    }

    #[test]
    fn qti_import_payload_matches_the_database_closed_shape() {
        let payload = JobPayload::QtiImport {
            workspace: WorkspaceId::from_uuid(Uuid::from_u128(1)),
            import: WorkspaceImportId::from_uuid(Uuid::from_u128(2)),
            source_object: ObjectId::from_uuid(Uuid::from_u128(3)),
        };
        assert_eq!(
            serde_json::to_value(payload).expect("payload serialization"),
            json!({
                "kind": "qtiImport",
                "workspace": "00000000-0000-0000-0000-000000000001",
                "import": "00000000-0000-0000-0000-000000000002",
                "source_object": "00000000-0000-0000-0000-000000000003"
            })
        );
    }
}
