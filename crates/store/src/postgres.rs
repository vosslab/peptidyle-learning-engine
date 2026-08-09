//! PostgreSQL backend, embedded migrations, and connection handling.
//!
//! Every operation runs as the non-bypassing `ple_app` role. Tenant-owned
//! operations also set `ple.tenant_id` locally inside their transaction, so a
//! pooled connection cannot retain another request's tenant context.

#[cfg(feature = "postgres")]
use std::collections::{BTreeMap, BTreeSet};
#[cfg(feature = "postgres")]
use std::fmt;

#[cfg(feature = "postgres")]
use async_trait::async_trait;
#[cfg(feature = "postgres")]
use domain::run::continued_practice_allows_run;
#[cfg(feature = "postgres")]
use domain::scoring::project_summary;
#[cfg(feature = "postgres")]
use domain::timing::{TimerEvaluation, TimerVerdict, timer_verdict};
#[cfg(feature = "postgres")]
use objects::Sha256Digest;
#[cfg(feature = "postgres")]
use question_model::run_policy::{
    CompletionRequirement, ContinuedPractice, FeedbackDisclosure, GradePolicy, RunPolicies,
    TimingPolicy, VariationPolicy,
};
#[cfg(feature = "postgres")]
use question_model::taxonomy::TaxonomyTerm;
#[cfg(feature = "postgres")]
use question_model::{
    ActivityTimestamp, AssetId, AssignmentDeadlineBehavior, AssignmentDeliveryState,
    AssignmentEnrollment, AssignmentId, AssignmentItem, AssignmentItemId,
    AssignmentPolicyExceptionId, AssignmentRun, AssignmentRunItem, AssignmentScoringMode,
    AssignmentSelectionCandidate, AssignmentSelectionGroup, AssignmentSelectionGroupId,
    AssignmentTimingPolicy, AttemptResult, AttemptStatus, AttemptTimerRecord, BackendCapabilities,
    CatalogCapabilityFacet, CatalogLicenseFacet, CatalogLifecycle, CatalogProblemDetail,
    CatalogProblemSummary, CatalogSearchFacets, CatalogSearchPage, CatalogSearchQuery,
    CatalogStatisticsAvailability, CatalogStatisticsFacet, CatalogTaxonomyFacet, CourseGroupId,
    CourseId, CourseMembership, CourseMembershipRole, CourseRole, CourseSummary, EnrollmentId,
    EnrollmentStatus, LateSubmissionPolicy, ObjectId, PointValue, ProblemId, ProblemPublicId,
    ProblemVersionNumber, ProblemVersionRef, PublicationScope, QuestionAttempt, QuestionAttemptId,
    QuestionBackend, QuestionDefinition, QuestionMetadata, QuestionStatisticsDisclosure,
    QuestionStatisticsView, RunId, RunMode, ScoringGeneration, ScoringStatus, SelectionOrdering,
    StudentAssignmentSummary, StudentId, StudentResponse, TenantId, UserId, UserRole, VersionId,
    WorkspaceDraftSummary, WorkspaceId, WorkspaceImportId,
};
#[cfg(feature = "postgres")]
use question_model::{FeedbackContent, envelope::ContentBlock};
#[cfg(feature = "postgres")]
use serde::de::DeserializeOwned;
#[cfg(feature = "postgres")]
use serde::{Deserialize, Serialize};
#[cfg(feature = "postgres")]
use serde_json::Value;
#[cfg(feature = "postgres")]
use sqlx::postgres::{PgPool, PgPoolOptions, PgRow};
#[cfg(feature = "postgres")]
use sqlx::types::{Json, Uuid};
#[cfg(feature = "postgres")]
use sqlx::{Postgres, Row, Transaction};

#[cfg(feature = "postgres")]
use crate::gradebook_cursor::GradebookCursor;
#[cfg(feature = "postgres")]
use crate::retention::RetentionApiAction;
#[cfg(feature = "postgres")]
use crate::run_summary_cursor::RunSummaryCursor;
#[cfg(feature = "postgres")]
use crate::statistics::derive_statistics_contributions;
#[cfg(feature = "postgres")]
use crate::{
    ActivityTransition, AssetAccessEvent, AssetDeliveryId, AssetDeliveryRecord, AssetDeliveryScope,
    AssetStore, AssignmentDefinitionDisposition, AssignmentExceptionLimit,
    AssignmentExceptionTimestamp, AssignmentPolicyException, AssignmentPolicyExceptionTarget,
    AssignmentRecord, AssignmentRevision, AssignmentUpdate, AttemptFeedbackRecord,
    AttemptSupportAction, AttemptSupportActionId, AttemptSupportRecord, AuthorizedAssetDelivery,
    CatalogAssetBinding, CatalogSourceStore, CatalogStore, CatalogTransition, ClearAttemptCommand,
    CourseGroupRecord, CourseGroupRevision, CourseListScope, CourseRecord,
    CourseRecordsAccessStore, CourseRetentionRecord, CourseRetentionSnapshot, CourseRetentionState,
    CourseRetentionView, Cursor, DeleteAndRegradeAssignmentItemCommand,
    DeleteAssignmentPolicyExceptionCommand, DraftRecord, FeedbackReleaseRecord,
    ForceSubmitAttemptCommand, InstitutionRetentionPolicy, IssueQuestionAttemptCommand, Page,
    PageRequest, PageSize, PrefetchedQuestion, PublishDraftCommand, PublishedProblemRecord,
    PublishedSourceArtifact, PutCourseGroupCommand, ReleaseAttemptFeedbackCommand,
    ReservePrefetchedQuestionCommand, ResolvedAssignmentTiming, ResolvedAttemptTiming,
    RetentionApiStore, RetentionCleanupManifest, RetentionDays, RetentionDispatchBatch,
    RetentionRevision, RetentionScheduleStore, RetentionStore, RetentionWork,
    RetentionWorkerCommand, RetentionWorkerStore, RunSummaryOutcomeInput, RunSummaryPageInput,
    SessionLifetime, SessionRecord, SessionStore, SessionSubject, SessionTokenHash,
    SetAssignmentPolicyExceptionCommand, Store, StoreError, StoredAssignment,
    StoredAssignmentPolicyException, StoredAssignmentTiming, StoredCourseGroup,
    SubmissionIdempotencyKey, SubmissionNextAttempt, SubmissionRecord,
    SubmitQuestionAttemptCommand, TenantContext, UpdateAssignmentTimingCommand, WorkspaceDraft,
    WorkspaceDraftRevision, assignment_scoring_changed, completed_run_score, current_run_questions,
    decode_catalog_search_cursor, decode_workspace_draft_cursor, delete_and_regrade_update,
    encode_catalog_search_cursor, encode_workspace_draft_cursor, ensure_tenant, grade_policy,
    private_feedback_record, project_enrollment_completion, resolve_assignment_policy,
    select_assignment_run_items, summary_transition, validate_asset_delivery, validate_assignment,
    validate_assignment_policy_exception, validate_assignment_timing, validate_course,
    validate_course_group, validate_draft, validate_publication_source, validate_published,
    validate_qti_import, validate_qti_publication_promotion, validate_source_artifact,
    validate_source_artifact_identity,
};
#[cfg(feature = "postgres")]
use crate::{
    BeginExternalToolGradeCommand, CommitExternalToolSubmissionCommand,
    CommitVerifiedExternalToolSubmissionCommand, CreateExternalToolLaunchSessionCommand,
    CreatedExternalToolLaunchSession, ExternalToolBegin, ExternalToolBinding,
    ExternalToolBrokerStore, ExternalToolLaunchProof, ExternalToolLaunchSessionStore,
    ExternalToolLaunchToken, ExternalToolLease, ExternalToolLeaseToken,
    ExternalToolVerifiedPending, ResolvedExternalToolLaunchSession,
    StageExternalToolVerificationCommand, fresh_external_tool_launch_id,
};
#[cfg(feature = "postgres")]
use crate::{
    ClaimedJob, CreateAssignmentExport, EnqueueJob, ExportArtifactKind, ExportArtifactRecord,
    ExportCommitDisposition, ExportId, ExportJobCommit, ExportJobStore, JobFailureDisposition,
    JobFailureKind, JobId, JobLeaseDuration, JobLeaseToken, JobPayload, JobState, JobStore,
    QueueDepth, StudentExportArtifactView, StudentExportJob, StudentExportState, StudentExportView,
    TenantJobView,
};
#[cfg(feature = "postgres")]
use crate::{
    CommitPreparedQtiImport, CommitPreparedQtiImportOutcome, CreateQtiImportCommand,
    QtiGradingStore, QtiImportGradingPayload, QtiImportRegistry, QtiImportStore,
};

#[cfg(feature = "postgres")]
static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("../../schemas/migrations");

#[cfg(feature = "postgres")]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
struct AttemptSupportAuditPayload {
    previous_status: AttemptStatus,
    resulting_status: AttemptStatus,
}

/// Read-only state of one embedded migration relative to a database.
#[cfg(feature = "postgres")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MigrationDisposition {
    /// The exact embedded checksum is recorded as successful.
    Applied,
    /// The migration is known to the application but absent from the ledger.
    Pending,
    /// The recorded checksum differs from the immutable embedded migration.
    Modified,
    /// SQLx recorded a failed, partially applied migration.
    Dirty,
}

/// Status of one migration in the initial database epoch.
#[cfg(feature = "postgres")]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MigrationStatusEntry {
    version: i64,
    description: String,
    disposition: MigrationDisposition,
}

#[cfg(feature = "postgres")]
impl MigrationStatusEntry {
    /// Returns the ordered SQLx migration version.
    pub fn version(&self) -> i64 {
        self.version
    }

    /// Returns the filename-derived migration description.
    pub fn description(&self) -> &str {
        &self.description
    }

    /// Returns the database disposition for this migration.
    pub fn disposition(&self) -> MigrationDisposition {
        self.disposition
    }
}

/// Read-only comparison of the embedded epoch with the SQLx ledger.
#[cfg(feature = "postgres")]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MigrationStatus {
    ledger_present: bool,
    entries: Vec<MigrationStatusEntry>,
    unexpected_applied_versions: Vec<i64>,
}

#[cfg(feature = "postgres")]
impl MigrationStatus {
    /// Returns whether SQLx has created its authoritative ledger.
    pub fn ledger_present(&self) -> bool {
        self.ledger_present
    }

    /// Returns every known migration in version order.
    pub fn entries(&self) -> &[MigrationStatusEntry] {
        &self.entries
    }

    /// Returns applied versions absent from the embedded immutable epoch.
    pub fn unexpected_applied_versions(&self) -> &[i64] {
        &self.unexpected_applied_versions
    }

    /// Returns true only for an exact, successful, complete epoch.
    pub fn is_compatible(&self) -> bool {
        self.ledger_present
            && self.unexpected_applied_versions.is_empty()
            && self
                .entries
                .iter()
                .all(|entry| entry.disposition == MigrationDisposition::Applied)
    }

    fn incompatibility_reason(&self) -> String {
        if !self.ledger_present {
            return "the SQLx migration ledger is absent".to_string();
        }
        if let Some(version) = self.unexpected_applied_versions.first() {
            return format!("applied migration {version} is absent from the embedded epoch");
        }
        if let Some(entry) = self
            .entries
            .iter()
            .find(|entry| entry.disposition != MigrationDisposition::Applied)
        {
            let state = match entry.disposition {
                MigrationDisposition::Applied => "applied",
                MigrationDisposition::Pending => "pending",
                MigrationDisposition::Modified => "modified",
                MigrationDisposition::Dirty => "dirty",
            };
            return format!("migration {} is {state}", entry.version);
        }
        "the database migration state is incompatible".to_string()
    }
}

/// Startup migration verification failure with credential-safe diagnostics.
#[cfg(feature = "postgres")]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SchemaCompatibilityError {
    /// PostgreSQL could not be reached, so the stateless API may start degraded.
    Unavailable,
    /// PostgreSQL was reachable but its schema was not the exact embedded epoch.
    Incompatible(String),
}

#[cfg(feature = "postgres")]
impl fmt::Display for SchemaCompatibilityError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unavailable => formatter.write_str("database schema state is unavailable"),
            Self::Incompatible(reason) => {
                write!(formatter, "database schema is incompatible: {reason}")
            }
        }
    }
}

#[cfg(feature = "postgres")]
impl std::error::Error for SchemaCompatibilityError {}

#[cfg(feature = "postgres")]
#[derive(Clone, Debug)]
struct AppliedMigrationState {
    version: i64,
    success: bool,
    checksum: Vec<u8>,
}

/// Gradebook pages are bounded joins over compact tenant-owned rows only.
///
/// This must stay free of `assignment_run`, `question_attempt`, and other
/// append-only history tables. The migration adds the matching enrollment
/// lookup index; this query itself is intentionally the complete page path.
#[cfg(feature = "postgres")]
const GRADEBOOK_SUMMARY_PAGE_SQL: &str = "SELECT \
    e.enrollment_id, e.student_id, a.assignment_id, a.title AS assignment_title, \
    sas.payload, sas.payload_sha256 \
 FROM assignment AS a \
 JOIN enrollment AS e \
   ON e.tenant_id = a.tenant_id AND e.assignment_id = a.assignment_id \
 JOIN student_assignment_summary AS sas \
   ON sas.tenant_id = e.tenant_id AND sas.enrollment_id = e.enrollment_id \
 WHERE a.tenant_id = $1 AND a.course_id = $2 \
   AND public.ple_course_records_accessible(a.tenant_id, a.course_id) \
   AND ($3::uuid IS NULL \
        OR (a.assignment_id, e.enrollment_id) > ($3, $4)) \
 ORDER BY a.assignment_id, e.enrollment_id LIMIT $5";

/// Member course pagination preserves manager definition access while hiding
/// an archived course from its learners at the database query boundary.
#[cfg(feature = "postgres")]
const MEMBER_COURSE_PAGE_SQL: &str = "SELECT \
    c.course_id::text AS stable_key, c.course_id, c.title, cm.role \
 FROM course AS c JOIN course_member AS cm \
   ON cm.tenant_id = c.tenant_id AND cm.course_id = c.course_id \
 WHERE c.tenant_id = $1 AND cm.user_id = $2 \
   AND (cm.role <> 'student' OR \
        public.ple_course_records_accessible(c.tenant_id, c.course_id)) \
   AND ($3::text IS NULL OR c.course_id::text > $3) \
 ORDER BY c.course_id::text LIMIT $4";

/// The connection pool type, re-exported so callers do not need `sqlx`.
#[cfg(feature = "postgres")]
pub type Pool = PgPool;

/// Replica-safe PostgreSQL implementation of the backend-neutral store.
#[cfg(feature = "postgres")]
#[derive(Clone)]
pub struct PostgresStore {
    pool: PgPool,
}

/// Injected grader-only database handle. Server composition supplies this only
/// to the grading boundary; [`PostgresStore`] never implements the matching
/// read trait or assumes the grader role.
#[cfg(feature = "postgres")]
#[derive(Clone)]
pub struct PostgresQtiGraderStore {
    pool: PgPool,
}

#[cfg(feature = "postgres")]
impl PostgresStore {
    /// Wraps a pool whose login can assume the migration-owned `ple_app` role.
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    async fn begin_app(&self) -> Result<Transaction<'_, Postgres>, StoreError> {
        let mut transaction = self.pool.begin().await.map_err(map_sqlx_error)?;
        sqlx::query("SET LOCAL ROLE ple_app")
            .execute(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        Ok(transaction)
    }

    async fn begin_tenant(
        &self,
        context: TenantContext,
    ) -> Result<Transaction<'_, Postgres>, StoreError> {
        let mut transaction = self.begin_app().await?;
        sqlx::query("SELECT set_config('ple.tenant_id', $1, true)")
            .bind(context.tenant_id().to_string())
            .execute(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        Ok(transaction)
    }

    /// Starts a read-only tenant transaction whose page and aggregate queries
    /// observe one PostgreSQL snapshot.  `SET TRANSACTION` must be the first
    /// statement, so this cannot delegate to [`Self::begin_tenant`].
    async fn begin_tenant_snapshot(
        &self,
        context: TenantContext,
    ) -> Result<Transaction<'_, Postgres>, StoreError> {
        let mut transaction = self.pool.begin().await.map_err(map_sqlx_error)?;
        sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ READ ONLY")
            .execute(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        sqlx::query("SET LOCAL ROLE ple_app")
            .execute(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        sqlx::query("SELECT set_config('ple.tenant_id', $1, true)")
            .bind(context.tenant_id().to_string())
            .execute(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        Ok(transaction)
    }

    async fn begin_session(
        &self,
        token_hash: SessionTokenHash,
    ) -> Result<Transaction<'_, Postgres>, StoreError> {
        let mut transaction = self.pool.begin().await.map_err(map_sqlx_error)?;
        sqlx::query("SET LOCAL ROLE ple_auth")
            .execute(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        sqlx::query("SELECT set_config('ple.session_hash', $1, true)")
            .bind(token_hash.to_string())
            .execute(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        Ok(transaction)
    }
}

#[cfg(feature = "postgres")]
impl PostgresQtiGraderStore {
    /// Connects using the dedicated, least-privilege QTI grader credential.
    ///
    /// The application pool is deliberately not accepted here. Deployment
    /// provisions the password and provides this URL only to server grading
    /// composition when QTI grading is enabled.
    pub async fn connect(database_url: &str) -> Result<Self, StoreError> {
        let pool = PgPoolOptions::new()
            .max_connections(4)
            .connect(database_url)
            .await
            .map_err(map_sqlx_error)?;
        let row = sqlx::query(
            "SELECT current_user AS current_user, session_user AS session_user, \
             r.rolsuper, r.rolbypassrls, \
             pg_has_role(current_user, 'ple_app', 'member') AS can_assume_app \
             FROM pg_roles AS r WHERE r.rolname = current_user",
        )
        .fetch_one(&pool)
        .await
        .map_err(map_sqlx_error)?;
        let current_user: String = row.try_get("current_user").map_err(map_sqlx_error)?;
        let session_user: String = row.try_get("session_user").map_err(map_sqlx_error)?;
        let superuser: bool = row.try_get("rolsuper").map_err(map_sqlx_error)?;
        let bypass_rls: bool = row.try_get("rolbypassrls").map_err(map_sqlx_error)?;
        let can_assume_app: bool = row.try_get("can_assume_app").map_err(map_sqlx_error)?;
        if current_user != "ple_qti_grader"
            || session_user != "ple_qti_grader"
            || superuser
            || bypass_rls
            || can_assume_app
        {
            pool.close().await;
            return Err(StoreError::Forbidden);
        }
        Ok(Self { pool })
    }

    async fn begin_grader_tenant(
        &self,
        context: TenantContext,
    ) -> Result<Transaction<'_, Postgres>, StoreError> {
        let mut transaction = self.pool.begin().await.map_err(map_sqlx_error)?;
        sqlx::query("SELECT set_config('ple.tenant_id', $1, true)")
            .bind(context.tenant_id().to_string())
            .execute(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        Ok(transaction)
    }
}

#[cfg(feature = "postgres")]
#[async_trait]
impl JobStore for PostgresStore {
    async fn enqueue_job(
        &self,
        context: TenantContext,
        job: EnqueueJob,
    ) -> Result<JobId, StoreError> {
        ensure_tenant(context, job.tenant)?;
        job.validate()?;
        let id = JobId::generate()?;
        let payload = serde_json::to_value(&job.payload).map_err(|error| {
            StoreError::InvalidRecord(format!("job payload serialization failed: {error}"))
        })?;
        let mut transaction = self.begin_tenant(context).await?;
        sqlx::query(
            "INSERT INTO worker_job (job_id, tenant_id, payload, state, max_attempts) \
             VALUES ($1, $2, $3, 'ready', $4)",
        )
        .bind(id.as_uuid())
        .bind(job.tenant.as_uuid())
        .bind(payload)
        .bind(i32::from(job.max_attempts))
        .execute(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(id)
    }

    async fn claim_next_job(
        &self,
        lease: JobLeaseDuration,
    ) -> Result<Option<ClaimedJob>, StoreError> {
        let token = JobLeaseToken::generate()?;
        let mut transaction = self.begin_app().await?;
        let row = sqlx::query(
            "SELECT job_id, tenant_id, payload, lease_token, attempt_count \
             FROM ple_claim_worker_job($1, $2)",
        )
        .bind(token.as_uuid())
        .bind(lease.seconds())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let claimed = row
            .as_ref()
            .map(|row| decode_claimed_job(row, token))
            .transpose()?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(claimed)
    }

    async fn complete_job(&self, id: JobId, token: JobLeaseToken) -> Result<(), StoreError> {
        let mut transaction = self.begin_app().await?;
        let completed: bool = sqlx::query_scalar("SELECT ple_complete_worker_job($1, $2)")
            .bind(id.as_uuid())
            .bind(token.as_uuid())
            .fetch_one(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        if !completed {
            return Err(StoreError::Conflict);
        }
        transaction.commit().await.map_err(map_sqlx_error)
    }

    async fn fail_job(
        &self,
        id: JobId,
        token: JobLeaseToken,
        failure: JobFailureKind,
    ) -> Result<JobFailureDisposition, StoreError> {
        let mut transaction = self.begin_app().await?;
        let row = sqlx::query("SELECT ple_fail_worker_job($1, $2, $3) AS disposition")
            .bind(id.as_uuid())
            .bind(token.as_uuid())
            .bind(failure.as_db())
            .fetch_one(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        let disposition: Option<String> = row.try_get("disposition").map_err(map_sqlx_error)?;
        let result = match disposition.as_deref() {
            Some("retrying") => JobFailureDisposition::Retrying,
            Some("dead") => JobFailureDisposition::Dead,
            None => return Err(StoreError::Conflict),
            Some(_) => {
                return Err(StoreError::Unavailable(
                    "queue broker returned an unknown failure disposition".to_string(),
                ));
            }
        };
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(result)
    }

    async fn get_job(
        &self,
        context: TenantContext,
        id: JobId,
    ) -> Result<Option<TenantJobView>, StoreError> {
        let mut transaction = self.begin_tenant(context).await?;
        let row =
            sqlx::query("SELECT payload, state, attempt_count FROM worker_job WHERE job_id = $1")
                .bind(id.as_uuid())
                .fetch_optional(&mut *transaction)
                .await
                .map_err(map_sqlx_error)?;
        let view = row
            .as_ref()
            .map(|row| decode_tenant_job_view(row, id))
            .transpose()?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(view)
    }

    async fn ready_queue_depth(&self) -> Result<QueueDepth, StoreError> {
        let mut transaction = self.begin_app().await?;
        let ready: i64 = sqlx::query_scalar("SELECT ple_ready_worker_queue_depth()")
            .fetch_one(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(QueueDepth {
            ready: u64::try_from(ready).map_err(|_| {
                StoreError::Unavailable("queue broker returned a negative depth".to_string())
            })?,
        })
    }
}

#[cfg(feature = "postgres")]
#[async_trait]
impl crate::AttemptAutoSubmitWorkerStore for PostgresStore {
    async fn commit_attempt_auto_submit(
        &self,
        context: TenantContext,
        command: crate::AttemptAutoSubmitWorkerCommand,
    ) -> Result<crate::AttemptAutoSubmitCommitOutcome, StoreError> {
        let tenant = context.tenant_id();
        let expected_payload = serde_json::to_value(JobPayload::AutoSubmitAttempt {
            attempt: command.attempt,
            timing_generation: command.timing_generation,
        })
        .map_err(|error| StoreError::InvalidRecord(error.to_string()))?;
        let mut transaction = self.begin_tenant(context).await?;
        let claim_active: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM worker_job \
             WHERE job_id = $1 AND tenant_id = $2 AND state = 'leased' \
               AND lease_token = $3 AND lease_expires_at > transaction_timestamp() \
               AND payload = $4)",
        )
        .bind(command.job.as_uuid())
        .bind(tenant.as_uuid())
        .bind(command.lease.as_uuid())
        .bind(expected_payload)
        .fetch_one(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        if !claim_active {
            transaction.commit().await.map_err(map_sqlx_error)?;
            return Ok(crate::AttemptAutoSubmitCommitOutcome::ClaimNoLongerActive);
        }
        let attempt =
            load_attempt_for_external_update(&mut transaction, tenant, command.attempt).await?;
        let timing_row = sqlx::query(
            "SELECT timing_generation, job_id, effective_grace_seconds, \
                    floor(extract(epoch FROM effective_deadline) * 1000)::bigint \
                        AS effective_deadline_millis, \
                    floor(extract(epoch FROM auto_submit_at) * 1000)::bigint \
                        AS auto_submit_at_millis \
             FROM attempt_timing_current \
             WHERE tenant_id = $1 AND attempt_id = $2 FOR UPDATE",
        )
        .bind(tenant.as_uuid())
        .bind(command.attempt.as_uuid())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let mapped_job = timing_row
            .as_ref()
            .map(|row| row.try_get::<Option<Uuid>, _>("job_id"))
            .transpose()
            .map_err(map_sqlx_error)?
            .flatten();
        if attempt.status != AttemptStatus::InProgress
            || timing_row.is_none()
            || mapped_job != Some(command.job.as_uuid())
        {
            complete_postgres_claimed_job(&mut transaction, command.job, command.lease).await?;
            transaction.commit().await.map_err(map_sqlx_error)?;
            return Ok(crate::AttemptAutoSubmitCommitOutcome::Superseded);
        }
        let timing_row = timing_row.expect("current mapping has a timing row");
        let generation = u64::try_from(
            timing_row
                .try_get::<i64, _>("timing_generation")
                .map_err(map_sqlx_error)?,
        )
        .map_err(|_| StoreError::Unavailable("stored timing generation is invalid".to_string()))?;
        let auto_submit_at = timing_row
            .try_get::<Option<i64>, _>("auto_submit_at_millis")
            .map_err(map_sqlx_error)?
            .map(ActivityTimestamp::from_unix_millis);
        let Some(auto_submit_at) = auto_submit_at else {
            complete_postgres_claimed_job(&mut transaction, command.job, command.lease).await?;
            sqlx::query(
                "UPDATE attempt_timing_current SET job_id = NULL, \
                    updated_at = transaction_timestamp() \
                 WHERE tenant_id = $1 AND attempt_id = $2",
            )
            .bind(tenant.as_uuid())
            .bind(command.attempt.as_uuid())
            .execute(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
            transaction.commit().await.map_err(map_sqlx_error)?;
            return Ok(crate::AttemptAutoSubmitCommitOutcome::Superseded);
        };
        let now = database_timestamp(&mut transaction).await?;
        if now < auto_submit_at {
            let payload = serde_json::to_value(JobPayload::AutoSubmitAttempt {
                attempt: command.attempt,
                timing_generation: generation,
            })
            .map_err(|error| StoreError::InvalidRecord(error.to_string()))?;
            let changed: bool = sqlx::query_scalar(
                "SELECT ple_reschedule_attempt_timing_job($1, $2, $3, $4, \
                    TIMESTAMPTZ 'epoch' + $5::bigint * INTERVAL '1 millisecond')",
            )
            .bind(tenant.as_uuid())
            .bind(command.job.as_uuid())
            .bind(command.lease.as_uuid())
            .bind(payload)
            .bind(auto_submit_at.as_unix_millis())
            .fetch_one(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
            if !changed {
                return Err(StoreError::Conflict);
            }
            transaction.commit().await.map_err(map_sqlx_error)?;
            return Ok(crate::AttemptAutoSubmitCommitOutcome::Rescheduled);
        }

        complete_postgres_claimed_job(&mut transaction, command.job, command.lease).await?;
        let updated = sqlx::query(
            "UPDATE question_attempt SET attempt_status = 'auto_submitted', \
                    submitted_at = transaction_timestamp() \
             WHERE tenant_id = $1 AND attempt_id = $2 AND attempt_status = 'in_progress'",
        )
        .bind(tenant.as_uuid())
        .bind(command.attempt.as_uuid())
        .execute(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        if updated.rows_affected() != 1 {
            return Err(StoreError::Conflict);
        }
        sqlx::query(
            "UPDATE attempt_timing_current SET job_id = NULL, \
                updated_at = transaction_timestamp() \
             WHERE tenant_id = $1 AND attempt_id = $2",
        )
        .bind(tenant.as_uuid())
        .bind(command.attempt.as_uuid())
        .execute(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(crate::AttemptAutoSubmitCommitOutcome::AutoSubmitted)
    }
}

#[cfg(feature = "postgres")]
async fn complete_postgres_claimed_job(
    transaction: &mut Transaction<'_, Postgres>,
    job: JobId,
    lease: JobLeaseToken,
) -> Result<(), StoreError> {
    let completed: bool = sqlx::query_scalar("SELECT ple_complete_worker_job($1, $2)")
        .bind(job.as_uuid())
        .bind(lease.as_uuid())
        .fetch_one(&mut **transaction)
        .await
        .map_err(map_sqlx_error)?;
    if !completed {
        return Err(StoreError::Conflict);
    }
    Ok(())
}

#[cfg(feature = "postgres")]
#[async_trait]
impl crate::AssignmentScoringWorkerStore for PostgresStore {
    async fn prepare_assignment_scoring(
        &self,
        context: TenantContext,
        command: crate::AssignmentScoringWorkerCommand,
    ) -> Result<(), StoreError> {
        let tenant = context.tenant_id();
        let expected_payload = serde_json::to_value(JobPayload::RecalculateAssignment {
            assignment: command.assignment,
            generation: command.generation,
        })
        .map_err(|error| StoreError::InvalidRecord(error.to_string()))?;
        let mut transaction = self.begin_tenant(context).await?;
        let claim_active: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM worker_job \
             WHERE job_id = $1 AND tenant_id = $2 AND state = 'leased' \
               AND lease_token = $3 AND lease_expires_at > transaction_timestamp() \
               AND payload = $4)",
        )
        .bind(command.job.as_uuid())
        .bind(tenant.as_uuid())
        .bind(command.lease.as_uuid())
        .bind(expected_payload)
        .fetch_one(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        if !claim_active {
            return Err(StoreError::Conflict);
        }
        let assignment = load_assignment(&mut transaction, tenant, command.assignment).await?;
        let generation = i64::try_from(command.generation.value()).map_err(|_| {
            StoreError::InvalidRecord("scoring generation is too large".to_string())
        })?;
        let current: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM assignment \
             WHERE tenant_id = $1 AND assignment_id = $2 \
               AND scoring_generation = $3 AND scoring_status = 'recalculating')",
        )
        .bind(tenant.as_uuid())
        .bind(command.assignment.as_uuid())
        .bind(generation)
        .fetch_one(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        if !current {
            return Err(StoreError::Conflict);
        }
        sqlx::query("DELETE FROM assignment_scoring_staging WHERE tenant_id = $1 AND job_id = $2")
            .bind(tenant.as_uuid())
            .bind(command.job.as_uuid())
            .execute(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        sqlx::query(
            "DELETE FROM assignment_attempt_score_staging \
             WHERE tenant_id = $1 AND job_id = $2",
        )
        .bind(tenant.as_uuid())
        .bind(command.job.as_uuid())
        .execute(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        sqlx::query(
            "DELETE FROM assignment_summary_staging \
             WHERE tenant_id = $1 AND job_id = $2",
        )
        .bind(tenant.as_uuid())
        .bind(command.job.as_uuid())
        .execute(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let attempt_count = sqlx::query(
            "WITH current_definition AS ( \
                 SELECT se.attempt_id, ri.assignment_item_id, a.course_id, \
                        se.credit_fraction, se.grading_status, \
                        COALESCE(ai.points_possible, sg.points_per_item) AS points_possible, \
                        COALESCE(ai.scoring_mode, CASE WHEN sc.delivery_state = 'retired' \
                            THEN 'excluded' ELSE 'normal' END) AS scoring_mode \
                   FROM submission_evaluation se \
                   JOIN question_attempt qa ON qa.tenant_id = se.tenant_id \
                        AND qa.attempt_id = se.attempt_id \
                   JOIN assignment_run ar ON ar.tenant_id = qa.tenant_id \
                        AND ar.run_id = qa.run_id \
                   JOIN enrollment e ON e.tenant_id = ar.tenant_id \
                        AND e.enrollment_id = ar.enrollment_id \
                   JOIN assignment a ON a.tenant_id = e.tenant_id \
                        AND a.assignment_id = e.assignment_id \
                   JOIN assignment_run_item ri ON ri.tenant_id = qa.tenant_id \
                        AND ri.run_id = qa.run_id \
                        AND ri.issued_position = qa.assignment_position \
              LEFT JOIN assignment_item ai ON ai.tenant_id = a.tenant_id \
                        AND ai.assignment_id = a.assignment_id \
                        AND ai.assignment_item_id = ri.assignment_item_id \
              LEFT JOIN assignment_selection_candidate sc ON sc.tenant_id = a.tenant_id \
                        AND sc.assignment_id = a.assignment_id \
                        AND sc.candidate_id = ri.assignment_item_id \
              LEFT JOIN assignment_selection_group sg ON sg.tenant_id = sc.tenant_id \
                        AND sg.assignment_id = sc.assignment_id \
                        AND sg.selection_group_id = sc.selection_group_id \
                  WHERE se.tenant_id = $1 AND a.assignment_id = $2 \
                    AND se.grading_status = 'graded' \
                    AND qa.attempt_status NOT IN ('cleared', 'exempt') \
                    AND (ai.assignment_item_id IS NOT NULL OR sc.candidate_id IS NOT NULL) \
             ) \
             INSERT INTO assignment_attempt_score_staging \
                 (tenant_id, job_id, assignment_id, scoring_generation, attempt_id, \
                  assignment_item_id, earned_points, possible_points, course_id) \
             SELECT $1, $3, $2, $4, attempt_id, assignment_item_id, \
                    CASE \
                      WHEN scoring_mode = 'excluded' THEN 0 \
                      WHEN scoring_mode = 'full_credit' THEN points_possible \
                      ELSE round(credit_fraction * points_possible, 4) \
                    END, \
                    CASE \
                      WHEN scoring_mode IN ('excluded', 'extra_credit') THEN 0 \
                      ELSE points_possible \
                    END, course_id \
               FROM current_definition",
        )
        .bind(tenant.as_uuid())
        .bind(command.assignment.as_uuid())
        .bind(command.job.as_uuid())
        .bind(generation)
        .execute(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?
        .rows_affected();

        let score_rows = sqlx::query(
            "WITH latest AS ( \
                 SELECT DISTINCT ON (qa.run_id, qa.assignment_position) \
                        qa.run_id, staged.earned_points, staged.possible_points \
                   FROM assignment_attempt_score_staging staged \
                   JOIN question_attempt qa ON qa.tenant_id = staged.tenant_id \
                        AND qa.attempt_id = staged.attempt_id \
                   JOIN submission_evaluation se ON se.tenant_id = staged.tenant_id \
                        AND se.attempt_id = staged.attempt_id \
                  WHERE staged.tenant_id = $1 AND staged.job_id = $2 \
                  ORDER BY qa.run_id, qa.assignment_position, se.evaluated_at DESC, qa.attempt_id DESC \
             ) \
             SELECT ar.enrollment_id, ar.run_id, ar.run_number, \
                    COALESCE(sum(latest.earned_points), 0)::text AS earned_points, \
                    COALESCE(sum(latest.possible_points), 0)::text AS possible_points \
               FROM assignment_run ar \
               JOIN enrollment e ON e.tenant_id = ar.tenant_id \
                    AND e.enrollment_id = ar.enrollment_id \
               JOIN latest ON latest.run_id = ar.run_id \
              WHERE ar.tenant_id = $1 AND e.assignment_id = $3 \
                AND ar.completed_at IS NOT NULL \
                AND NOT EXISTS ( \
                    SELECT 1 FROM question_attempt pending \
                    JOIN submission_evaluation evaluation \
                      ON evaluation.tenant_id = pending.tenant_id \
                     AND evaluation.attempt_id = pending.attempt_id \
                   WHERE pending.tenant_id = ar.tenant_id \
                     AND pending.run_id = ar.run_id \
                     AND pending.attempt_status NOT IN ('cleared', 'exempt') \
                     AND evaluation.grading_status = 'needs_manual_grading' \
                ) \
              GROUP BY ar.enrollment_id, ar.run_id, ar.run_number",
        )
        .bind(tenant.as_uuid())
        .bind(command.job.as_uuid())
        .bind(command.assignment.as_uuid())
        .fetch_all(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let mut completed_by_enrollment: BTreeMap<
            EnrollmentId,
            Vec<domain::scoring::CompletedRunScore>,
        > = BTreeMap::new();
        for row in score_rows {
            let earned = row
                .try_get::<String, _>("earned_points")
                .map_err(map_sqlx_error)?
                .parse::<f64>()
                .map_err(|_| {
                    StoreError::Unavailable("stored earned points are invalid".to_string())
                })?;
            let possible = row
                .try_get::<String, _>("possible_points")
                .map_err(map_sqlx_error)?
                .parse::<f64>()
                .map_err(|_| {
                    StoreError::Unavailable("stored possible points are invalid".to_string())
                })?;
            let score = if possible > 0.0 {
                earned / possible
            } else {
                earned
            };
            let enrollment =
                EnrollmentId::from_uuid(row.try_get("enrollment_id").map_err(map_sqlx_error)?);
            completed_by_enrollment.entry(enrollment).or_default().push(
                domain::scoring::CompletedRunScore {
                    run: RunId::from_uuid(row.try_get("run_id").map_err(map_sqlx_error)?),
                    run_number: u32::try_from(
                        row.try_get::<i64, _>("run_number")
                            .map_err(map_sqlx_error)?,
                    )
                    .map_err(|_| {
                        StoreError::Unavailable("stored run number is invalid".to_string())
                    })?,
                    score,
                },
            );
        }
        let enrollment_rows = sqlx::query(
            "SELECT e.enrollment_id, e.payload AS enrollment_payload, \
                    e.payload_sha256 AS enrollment_payload_sha256, \
                    sas.payload AS summary_payload, sas.payload_sha256 AS summary_payload_sha256 \
               FROM enrollment e \
               JOIN student_assignment_summary sas ON sas.tenant_id = e.tenant_id \
                    AND sas.enrollment_id = e.enrollment_id \
              WHERE e.tenant_id = $1 AND e.assignment_id = $2 \
              ORDER BY e.enrollment_id FOR SHARE OF e, sas",
        )
        .bind(tenant.as_uuid())
        .bind(command.assignment.as_uuid())
        .fetch_all(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let enrollment_count = enrollment_rows.len();
        for row in enrollment_rows {
            let enrollment: AssignmentEnrollment =
                decode_payload_row_named(&row, "enrollment_payload", "enrollment_payload_sha256")?;
            let summary: StudentAssignmentSummary =
                decode_payload_row_named(&row, "summary_payload", "summary_payload_sha256")?;
            let completed = completed_by_enrollment
                .remove(&enrollment.id)
                .unwrap_or_default();
            let (enrollment, summary) = crate::recalculated_enrollment_projection(
                enrollment,
                summary,
                assignment.policies.grade,
                completed,
            )?;
            let (summary_payload, summary_checksum) = encode_payload(&summary)?;
            let (enrollment_payload, enrollment_checksum) = encode_payload(&enrollment)?;
            sqlx::query(
                "INSERT INTO assignment_summary_staging \
                 (tenant_id, job_id, assignment_id, scoring_generation, enrollment_id, \
                  summary_payload, summary_payload_sha256, enrollment_payload, \
                  enrollment_payload_sha256) \
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
            )
            .bind(tenant.as_uuid())
            .bind(command.job.as_uuid())
            .bind(command.assignment.as_uuid())
            .bind(generation)
            .bind(enrollment.id.as_uuid())
            .bind(summary_payload)
            .bind(summary_checksum)
            .bind(enrollment_payload)
            .bind(enrollment_checksum)
            .execute(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        }
        if !completed_by_enrollment.is_empty() {
            return Err(StoreError::Unavailable(
                "completed run has no assignment enrollment".to_string(),
            ));
        }
        sqlx::query(
            "INSERT INTO assignment_scoring_staging \
             (tenant_id, job_id, assignment_id, scoring_generation, attempt_count, enrollment_count) \
             VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(tenant.as_uuid())
        .bind(command.job.as_uuid())
        .bind(command.assignment.as_uuid())
        .bind(generation)
        .bind(i64::try_from(attempt_count).map_err(|_| StoreError::Conflict)?)
        .bind(i64::try_from(enrollment_count).map_err(|_| StoreError::Conflict)?)
        .execute(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        transaction.commit().await.map_err(map_sqlx_error)
    }

    async fn commit_assignment_scoring(
        &self,
        context: TenantContext,
        command: crate::AssignmentScoringWorkerCommand,
    ) -> Result<crate::AssignmentScoringCommitOutcome, StoreError> {
        let tenant = context.tenant_id();
        let expected_payload = serde_json::to_value(JobPayload::RecalculateAssignment {
            assignment: command.assignment,
            generation: command.generation,
        })
        .map_err(|error| StoreError::InvalidRecord(error.to_string()))?;
        let generation = i64::try_from(command.generation.value()).map_err(|_| {
            StoreError::InvalidRecord("scoring generation is too large".to_string())
        })?;
        let mut transaction = self.begin_tenant(context).await?;
        let claim_active: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM worker_job \
             WHERE job_id = $1 AND tenant_id = $2 AND state = 'leased' \
               AND lease_token = $3 AND lease_expires_at > transaction_timestamp() \
               AND payload = $4)",
        )
        .bind(command.job.as_uuid())
        .bind(tenant.as_uuid())
        .bind(command.lease.as_uuid())
        .bind(expected_payload)
        .fetch_one(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        if !claim_active {
            transaction.rollback().await.map_err(map_sqlx_error)?;
            return Ok(crate::AssignmentScoringCommitOutcome::ClaimNoLongerActive);
        }
        let current_generation: i64 = sqlx::query_scalar(
            "SELECT scoring_generation FROM assignment \
             WHERE tenant_id = $1 AND assignment_id = $2 FOR UPDATE",
        )
        .bind(tenant.as_uuid())
        .bind(command.assignment.as_uuid())
        .fetch_one(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let superseded = current_generation != generation;
        let current_attempt_count: i64 = sqlx::query_scalar(
            "SELECT count(*) \
               FROM submission_evaluation se \
               JOIN question_attempt qa ON qa.tenant_id = se.tenant_id \
                    AND qa.attempt_id = se.attempt_id \
               JOIN assignment_run ar ON ar.tenant_id = qa.tenant_id \
                    AND ar.run_id = qa.run_id \
               JOIN enrollment e ON e.tenant_id = ar.tenant_id \
                    AND e.enrollment_id = ar.enrollment_id \
               JOIN assignment a ON a.tenant_id = e.tenant_id \
                    AND a.assignment_id = e.assignment_id \
               JOIN assignment_run_item ri ON ri.tenant_id = qa.tenant_id \
                    AND ri.run_id = qa.run_id \
                    AND ri.issued_position = qa.assignment_position \
          LEFT JOIN assignment_item ai ON ai.tenant_id = a.tenant_id \
                    AND ai.assignment_id = a.assignment_id \
                    AND ai.assignment_item_id = ri.assignment_item_id \
          LEFT JOIN assignment_selection_candidate sc ON sc.tenant_id = a.tenant_id \
                    AND sc.assignment_id = a.assignment_id \
                    AND sc.candidate_id = ri.assignment_item_id \
              WHERE se.tenant_id = $1 AND a.assignment_id = $2 \
                AND se.grading_status = 'graded' \
                AND qa.attempt_status NOT IN ('cleared', 'exempt') \
                AND (ai.assignment_item_id IS NOT NULL OR sc.candidate_id IS NOT NULL)",
        )
        .bind(tenant.as_uuid())
        .bind(command.assignment.as_uuid())
        .fetch_one(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let prepared: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM assignment_scoring_staging \
             WHERE tenant_id = $1 AND job_id = $2 AND assignment_id = $3 \
               AND scoring_generation = $4 \
               AND ($6 OR attempt_count = $5) \
               AND attempt_count = (SELECT count(*) FROM assignment_attempt_score_staging \
                    WHERE tenant_id = $1 AND job_id = $2) \
               AND enrollment_count = (SELECT count(*) FROM assignment_summary_staging \
                    WHERE tenant_id = $1 AND job_id = $2))",
        )
        .bind(tenant.as_uuid())
        .bind(command.job.as_uuid())
        .bind(command.assignment.as_uuid())
        .bind(generation)
        .bind(current_attempt_count)
        .bind(superseded)
        .fetch_one(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        if !prepared {
            return Err(StoreError::Conflict);
        }
        if !superseded {
            sqlx::query(
                "DELETE FROM attempt_score_current \
                 WHERE tenant_id = $1 AND assignment_id = $2",
            )
            .bind(tenant.as_uuid())
            .bind(command.assignment.as_uuid())
            .execute(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
            sqlx::query(
                "INSERT INTO attempt_score_current \
                 (tenant_id, attempt_id, assignment_id, assignment_item_id, scoring_generation, \
                  earned_points, possible_points, course_id) \
                 SELECT tenant_id, attempt_id, assignment_id, assignment_item_id, \
                        scoring_generation, earned_points, possible_points, course_id \
                   FROM assignment_attempt_score_staging \
                  WHERE tenant_id = $1 AND job_id = $2",
            )
            .bind(tenant.as_uuid())
            .bind(command.job.as_uuid())
            .execute(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
            sqlx::query(
                "UPDATE student_assignment_summary sas \
                    SET payload = staged.summary_payload, \
                        payload_sha256 = staged.summary_payload_sha256, \
                        updated_at = transaction_timestamp() \
                   FROM assignment_summary_staging staged \
                  WHERE staged.tenant_id = $1 AND staged.job_id = $2 \
                    AND sas.tenant_id = staged.tenant_id \
                    AND sas.enrollment_id = staged.enrollment_id",
            )
            .bind(tenant.as_uuid())
            .bind(command.job.as_uuid())
            .execute(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
            sqlx::query(
                "UPDATE enrollment e SET payload = staged.enrollment_payload, \
                        payload_sha256 = staged.enrollment_payload_sha256 \
                   FROM assignment_summary_staging staged \
                  WHERE staged.tenant_id = $1 AND staged.job_id = $2 \
                    AND e.tenant_id = staged.tenant_id \
                    AND e.enrollment_id = staged.enrollment_id",
            )
            .bind(tenant.as_uuid())
            .bind(command.job.as_uuid())
            .execute(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
            let updated = sqlx::query(
                "UPDATE assignment SET scoring_status = 'current', \
                        updated_at = transaction_timestamp() \
                 WHERE tenant_id = $1 AND assignment_id = $2 \
                   AND scoring_generation = $3 AND scoring_status = 'recalculating'",
            )
            .bind(tenant.as_uuid())
            .bind(command.assignment.as_uuid())
            .bind(generation)
            .execute(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
            if updated.rows_affected() != 1 {
                return Err(StoreError::Conflict);
            }
        }
        sqlx::query("DELETE FROM assignment_scoring_staging WHERE tenant_id = $1 AND job_id = $2")
            .bind(tenant.as_uuid())
            .bind(command.job.as_uuid())
            .execute(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        sqlx::query(
            "DELETE FROM assignment_attempt_score_staging WHERE tenant_id = $1 AND job_id = $2",
        )
        .bind(tenant.as_uuid())
        .bind(command.job.as_uuid())
        .execute(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        sqlx::query("DELETE FROM assignment_summary_staging WHERE tenant_id = $1 AND job_id = $2")
            .bind(tenant.as_uuid())
            .bind(command.job.as_uuid())
            .execute(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        let completed: bool = sqlx::query_scalar("SELECT ple_complete_worker_job($1, $2)")
            .bind(command.job.as_uuid())
            .bind(command.lease.as_uuid())
            .fetch_one(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        if !completed {
            return Err(StoreError::Conflict);
        }
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(if superseded {
            crate::AssignmentScoringCommitOutcome::Superseded
        } else {
            crate::AssignmentScoringCommitOutcome::Committed
        })
    }
}

#[cfg(feature = "postgres")]
#[async_trait]
impl ExportJobStore for PostgresStore {
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
        let mut transaction = self.begin_tenant(context).await?;
        let row = sqlx::query(
            "SELECT payload FROM assignment \
             WHERE assignment_id = $1 \
               AND public.ple_course_records_accessible(tenant_id, course_id) \
             FOR SHARE",
        )
        .bind(request.assignment.as_uuid())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?
        .ok_or(StoreError::NotFound)?;
        let assignment: AssignmentRecord = decode_payload_row(&row)?;
        let mut expected = Vec::new();
        for kind in ExportArtifactKind::ALL {
            expected.push((kind, fresh_export_object_id()?));
        }
        let frozen = StudentExportJob {
            id: export,
            tenant: context.tenant_id(),
            assignment: assignment.id,
            course: assignment.course_id,
            title: assignment.title.clone(),
            requested_by: request.requested_by,
            manifest,
            problems: assignment.active_references().collect(),
            expected_artifacts: expected.clone(),
        };
        let (frozen_payload, frozen_checksum) = encode_payload(&frozen)?;
        let payload = serde_json::to_value(JobPayload::Export {
            delivery_object: manifest,
        })
        .map_err(|error| {
            StoreError::InvalidRecord(format!("job payload serialization failed: {error}"))
        })?;
        sqlx::query(
            "INSERT INTO worker_job (job_id, tenant_id, payload, state, max_attempts) \
             VALUES ($1, $2, $3, 'ready', $4)",
        )
        .bind(job.as_uuid())
        .bind(context.tenant_id().as_uuid())
        .bind(payload)
        .bind(i32::from(request.max_attempts))
        .execute(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        sqlx::query(
            "INSERT INTO student_export_request \
             (export_id, tenant_id, course_id, assignment_id, requester_id, job_id, manifest_object_id, \
              frozen_payload, frozen_payload_sha256, state) \
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,decode($9,'hex'),'queued')",
        )
        .bind(export.as_uuid())
        .bind(context.tenant_id().as_uuid())
        .bind(assignment.course_id.as_uuid())
        .bind(assignment.id.as_uuid())
        .bind(request.requested_by.as_uuid())
        .bind(job.as_uuid())
        .bind(manifest.as_uuid())
        .bind(frozen_payload)
        .bind(frozen_checksum)
        .execute(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        for (kind, object) in expected {
            sqlx::query(
                "INSERT INTO student_export_artifact (export_id, kind, object_id) VALUES ($1,$2,$3)",
            )
            .bind(export.as_uuid())
            .bind(export_artifact_kind_db(kind))
            .bind(object.as_uuid())
            .execute(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        }
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(StudentExportView {
            id: export,
            assignment: assignment.id,
            state: StudentExportState::Queued,
            artifacts: None,
        })
    }

    async fn get_assignment_export(
        &self,
        context: TenantContext,
        export: ExportId,
    ) -> Result<Option<StudentExportView>, StoreError> {
        get_postgres_export_view(self, context, export, None).await
    }

    async fn get_assignment_export_for_requester(
        &self,
        context: TenantContext,
        export: ExportId,
        requester: UserId,
    ) -> Result<Option<StudentExportView>, StoreError> {
        get_postgres_export_view(self, context, export, Some(requester)).await
    }

    async fn load_export_job(
        &self,
        context: TenantContext,
        manifest: ObjectId,
    ) -> Result<Option<StudentExportJob>, StoreError> {
        let mut transaction = self.begin_tenant(context).await?;
        let row = sqlx::query(
            "SELECT frozen_payload, frozen_payload_sha256 FROM student_export_request \
             WHERE manifest_object_id = $1",
        )
        .bind(manifest.as_uuid())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let result = row.as_ref().map(decode_payload_row).transpose()?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(result)
    }

    async fn commit_export_effect(
        &self,
        context: TenantContext,
        commit: ExportJobCommit,
    ) -> Result<ExportCommitDisposition, StoreError> {
        validate_export_artifacts(context.tenant_id(), &commit.artifacts)?;
        let mut transaction = self.begin_tenant(context).await?;
        let request_row = sqlx::query(
            "SELECT requester_id, course_id FROM student_export_request \
             WHERE job_id = $1 AND manifest_object_id = $2",
        )
        .bind(commit.job.as_uuid())
        .bind(commit.manifest.as_uuid())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?
        .ok_or(StoreError::NotFound)?;
        let requester = UserId::from_uuid(
            request_row
                .try_get("requester_id")
                .map_err(map_sqlx_error)?,
        );
        let course = CourseId::from_uuid(request_row.try_get("course_id").map_err(map_sqlx_error)?);
        let mut artifacts = Vec::with_capacity(commit.artifacts.len());
        for artifact in &commit.artifacts {
            let delivery = AssetDeliveryRecord {
                id: AssetDeliveryId::from_object(artifact.object.id),
                object: artifact.object.clone(),
                scope: AssetDeliveryScope::StudentRecord {
                    tenant: context.tenant_id(),
                    course,
                    authorized_users: vec![requester],
                },
            };
            // The requester and course come only from the frozen export row.
            // The broker verifies the exact typed delivery while committing
            // the closed four-artifact bundle under the active lease.
            let object = serde_json::to_value(&artifact.object).map_err(|error| {
                StoreError::InvalidRecord(format!("export object serialization failed: {error}"))
            })?;
            let (delivery_payload, delivery_sha256) = encode_payload(&delivery)?;
            artifacts.push(serde_json::json!({
                "kind": export_artifact_kind_db(artifact.kind),
                "object": artifact.object.id.as_uuid().to_string(),
                "filename": artifact.filename,
                "mediaType": artifact.object.media_type,
                "objectRecord": object,
                "delivery": delivery_payload.0,
                "deliverySha256": delivery_sha256,
            }));
        }
        let disposition: Option<String> =
            sqlx::query_scalar("SELECT ple_commit_export_job($1,$2,$3,$4,$5)")
                .bind(context.tenant_id().as_uuid())
                .bind(commit.job.as_uuid())
                .bind(commit.lease.as_uuid())
                .bind(commit.manifest.as_uuid())
                .bind(serde_json::Value::Array(artifacts))
                .fetch_one(&mut *transaction)
                .await
                .map_err(map_sqlx_error)?;
        let result = match disposition.as_deref() {
            Some("committed") => ExportCommitDisposition::Committed,
            Some("already_committed") => ExportCommitDisposition::AlreadyCommitted,
            _ => return Err(StoreError::Conflict),
        };
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(result)
    }
}

#[cfg(feature = "postgres")]
fn fresh_export_object_id() -> Result<ObjectId, StoreError> {
    let mut bytes = [0_u8; 16];
    getrandom::fill(&mut bytes).map_err(|error| {
        StoreError::Unavailable(format!("export object ID randomness unavailable: {error}"))
    })?;
    Ok(ObjectId::from_uuid(Uuid::from_bytes(bytes)))
}

#[cfg(feature = "postgres")]
fn export_artifact_kind_db(kind: ExportArtifactKind) -> &'static str {
    match kind {
        ExportArtifactKind::Docx => "docx",
        ExportArtifactKind::Pdf => "pdf",
        ExportArtifactKind::AccessibleDocx => "accessibleDocx",
        ExportArtifactKind::AccessiblePdf => "accessiblePdf",
    }
}

#[cfg(feature = "postgres")]
fn export_artifact_kind_from_db(value: &str) -> Result<ExportArtifactKind, StoreError> {
    match value {
        "docx" => Ok(ExportArtifactKind::Docx),
        "pdf" => Ok(ExportArtifactKind::Pdf),
        "accessibleDocx" => Ok(ExportArtifactKind::AccessibleDocx),
        "accessiblePdf" => Ok(ExportArtifactKind::AccessiblePdf),
        _ => Err(StoreError::Unavailable(
            "unknown stored export artifact kind".to_string(),
        )),
    }
}

#[cfg(feature = "postgres")]
fn validate_export_artifacts(
    tenant: TenantId,
    artifacts: &[ExportArtifactRecord],
) -> Result<(), StoreError> {
    if artifacts.len() != 4 {
        return Err(StoreError::InvalidRecord(
            "an export effect must contain exactly four artifacts".to_string(),
        ));
    }
    let mut kinds = std::collections::BTreeSet::new();
    let mut objects = std::collections::BTreeSet::new();
    for artifact in artifacts {
        let expected_filename = match artifact.kind {
            ExportArtifactKind::Docx => "exam.docx",
            ExportArtifactKind::Pdf => "exam.pdf",
            ExportArtifactKind::AccessibleDocx => "exam-accessible.docx",
            ExportArtifactKind::AccessiblePdf => "exam-accessible.pdf",
        };
        if !kinds.insert(artifact.kind)
            || !objects.insert(artifact.object.id)
            || artifact.filename != expected_filename
            || artifact.object.media_type != artifact.kind.media_type()
            || !matches!(&artifact.object.key, objects::ObjectKey::StudentRecord { tenant: key_tenant, object }
                if *key_tenant == tenant && *object == artifact.object.id)
        {
            return Err(StoreError::InvalidRecord(
                "export artifact does not match its closed private output contract".to_string(),
            ));
        }
    }
    Ok(())
}

#[cfg(feature = "postgres")]
async fn get_postgres_export_view(
    store: &PostgresStore,
    context: TenantContext,
    export: ExportId,
    requester: Option<UserId>,
) -> Result<Option<StudentExportView>, StoreError> {
    let mut transaction = store.begin_tenant(context).await?;
    let row = sqlx::query(
        "SELECT assignment_id, state FROM student_export_request WHERE export_id = $1 \
         AND ($2::uuid IS NULL OR requester_id = $2)",
    )
    .bind(export.as_uuid())
    .bind(requester.map(|value| value.as_uuid()))
    .fetch_optional(&mut *transaction)
    .await
    .map_err(map_sqlx_error)?;
    let view = if let Some(row) = row {
        let state = match row
            .try_get::<String, _>("state")
            .map_err(map_sqlx_error)?
            .as_str()
        {
            "queued" => StudentExportState::Queued,
            "ready" => StudentExportState::Ready,
            "failed" => StudentExportState::Failed,
            _ => {
                return Err(StoreError::Unavailable(
                    "unknown stored export state".to_string(),
                ));
            }
        };
        let artifacts = if state == StudentExportState::Ready {
            let rows = sqlx::query(
                "SELECT kind, filename, media_type, delivery_id FROM student_export_artifact \
                 WHERE export_id = $1 ORDER BY kind",
            )
            .bind(export.as_uuid())
            .fetch_all(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
            Some(
                rows.into_iter()
                    .map(|artifact| {
                        Ok(StudentExportArtifactView {
                            kind: export_artifact_kind_from_db(
                                &artifact
                                    .try_get::<String, _>("kind")
                                    .map_err(map_sqlx_error)?,
                            )?,
                            filename: artifact.try_get("filename").map_err(map_sqlx_error)?,
                            media_type: artifact.try_get("media_type").map_err(map_sqlx_error)?,
                            delivery: AssetDeliveryId::from_object(ObjectId::from_uuid(
                                artifact.try_get("delivery_id").map_err(map_sqlx_error)?,
                            )),
                        })
                    })
                    .collect::<Result<Vec<_>, StoreError>>()?,
            )
        } else {
            None
        };
        Some(StudentExportView {
            id: export,
            assignment: AssignmentId::from_uuid(
                row.try_get("assignment_id").map_err(map_sqlx_error)?,
            ),
            state,
            artifacts,
        })
    } else {
        None
    };
    transaction.commit().await.map_err(map_sqlx_error)?;
    Ok(view)
}

#[cfg(feature = "postgres")]
#[async_trait]
impl AssetStore for PostgresStore {
    async fn register_asset_delivery(
        &self,
        context: TenantContext,
        record: AssetDeliveryRecord,
    ) -> Result<(), StoreError> {
        validate_asset_delivery(&record)?;
        let (payload, checksum) = encode_payload(&record)?;
        let (kind, tenant, course, problem, version, asset) = match &record.scope {
            AssetDeliveryScope::Catalog { asset, reference } => (
                "catalog",
                None,
                None,
                Some(reference.problem),
                Some(reference.version),
                Some(*asset),
            ),
            AssetDeliveryScope::StudentRecord { tenant, course, .. } => {
                ensure_tenant(context, *tenant)?;
                (
                    "student_record",
                    Some(*tenant),
                    Some(*course),
                    None,
                    None,
                    None,
                )
            }
        };
        let mut transaction = self.begin_tenant(context).await?;
        if let (Some(problem), Some(version)) = (problem, version) {
            let visible: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM problem_version \
                 WHERE problem_id = $1 AND version_id = $2)",
            )
            .bind(problem.as_uuid())
            .bind(version.as_uuid())
            .fetch_one(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
            if !visible {
                return Err(StoreError::NotFound);
            }
        }
        if let Some(course) = course {
            let accessible: bool =
                sqlx::query_scalar("SELECT public.ple_course_records_accessible($1, $2)")
                    .bind(context.tenant_id().as_uuid())
                    .bind(course.as_uuid())
                    .fetch_one(&mut *transaction)
                    .await
                    .map_err(map_sqlx_error)?;
            if !accessible {
                return Err(StoreError::NotFound);
            }
        }
        sqlx::query(
            "INSERT INTO asset_delivery \
             (delivery_id, delivery_kind, tenant_id, course_id, object_id, problem_id, version_id, \
              asset_id, payload, payload_sha256) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
        )
        .bind(record.id.as_uuid())
        .bind(kind)
        .bind(tenant.map(|value| value.as_uuid()))
        .bind(course.map(|value| value.as_uuid()))
        .bind(record.object.id.as_uuid())
        .bind(problem.map(|value| value.as_uuid()))
        .bind(version.map(|value| value.as_uuid()))
        .bind(asset.map(|value| value.as_uuid()))
        .bind(payload)
        .bind(checksum)
        .execute(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        transaction.commit().await.map_err(map_sqlx_error)
    }

    async fn get_public_asset_delivery(
        &self,
        delivery: AssetDeliveryId,
    ) -> Result<Option<AssetDeliveryRecord>, StoreError> {
        let mut transaction = self.begin_app().await?;
        let row = sqlx::query(
            "SELECT ad.payload, ad.payload_sha256 FROM asset_delivery AS ad \
             JOIN problem_version AS pv \
               ON pv.problem_id = ad.problem_id AND pv.version_id = ad.version_id \
             WHERE ad.delivery_id = $1 AND ad.delivery_kind = 'catalog' \
               AND pv.publication_scope = 'public'",
        )
        .bind(delivery.as_uuid())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let record = row.as_ref().map(decode_asset_delivery_row).transpose()?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(record)
    }

    async fn catalog_asset_bindings(
        &self,
        context: TenantContext,
        reference: ProblemVersionRef,
    ) -> Result<Vec<CatalogAssetBinding>, StoreError> {
        let mut transaction = self.begin_tenant(context).await?;
        let rows = sqlx::query(
            "SELECT asset_id, object_id FROM asset_delivery \
             WHERE delivery_kind = 'catalog' \
               AND problem_id = $1 AND version_id = $2 \
             ORDER BY asset_id ASC",
        )
        .bind(reference.problem.as_uuid())
        .bind(reference.version.as_uuid())
        .fetch_all(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let bindings = rows
            .iter()
            .map(|row| CatalogAssetBinding {
                asset: AssetId::from_uuid(row.get::<Uuid, _>("asset_id")),
                object: ObjectId::from_uuid(row.get::<Uuid, _>("object_id")),
            })
            .collect();
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(bindings)
    }

    async fn authorize_asset_delivery(
        &self,
        context: TenantContext,
        actor: UserId,
        delivery: AssetDeliveryId,
    ) -> Result<AuthorizedAssetDelivery, StoreError> {
        let mut transaction = self.begin_tenant(context).await?;
        let row = sqlx::query(
            "SELECT payload, payload_sha256, course_id FROM asset_delivery \
             WHERE delivery_id = $1",
        )
        .bind(delivery.as_uuid())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?
        .ok_or(StoreError::NotFound)?;
        let record = decode_asset_delivery_row(&row)?;
        let (scope_text, delivery_id, scope_course_id): (&str, Uuid, Option<Uuid>) =
            match &record.scope {
                AssetDeliveryScope::Catalog { .. } => ("catalog", record.id.as_uuid(), None),
                AssetDeliveryScope::StudentRecord {
                    tenant: scope_tenant,
                    course,
                    ..
                } => {
                    if *scope_tenant != context.tenant_id() {
                        return Err(StoreError::NotFound);
                    }
                    let object_course = row
                        .try_get::<Option<Uuid>, _>("course_id")
                        .map_err(map_sqlx_error)?;
                    if object_course != Some(course.as_uuid()) {
                        return Err(StoreError::NotFound);
                    }
                    (
                        "student_record",
                        record.id.as_uuid(),
                        Some(course.as_uuid()),
                    )
                }
            };
        if let AssetDeliveryScope::StudentRecord {
            tenant,
            course: _,
            authorized_users,
        } = &record.scope
            && (*tenant != context.tenant_id() || !authorized_users.contains(&actor))
        {
            return Err(StoreError::NotFound);
        }
        let authorized_at = database_timestamp(&mut transaction).await?;
        let event = AssetAccessEvent {
            tenant: context.tenant_id(),
            actor,
            delivery,
            object: record.object.id,
            bucket: record.object.bucket,
            course: scope_course_id.map(CourseId::from_uuid),
            occurred_at: authorized_at,
        };
        let (payload, checksum) = encode_payload(&event)?;
        sqlx::query(
            "INSERT INTO record_access_log \
             (tenant_id, access_log_id, occurred_at, payload, payload_sha256, \
              delivery_scope, delivery_id, course_id) \
             VALUES ($1, gen_random_uuid(), transaction_timestamp(), $2, $3, $4, $5, $6)",
        )
        .bind(context.tenant_id().as_uuid())
        .bind(payload)
        .bind(checksum)
        .bind(scope_text)
        .bind(delivery_id)
        .bind(scope_course_id)
        .execute(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(AuthorizedAssetDelivery {
            record,
            authorized_at,
        })
    }
}

#[cfg(feature = "postgres")]
#[async_trait]
impl SessionStore for PostgresStore {
    async fn create_session(
        &self,
        token_hash: SessionTokenHash,
        subject: SessionSubject,
        lifetime: SessionLifetime,
    ) -> Result<SessionRecord, StoreError> {
        let mut transaction = self.begin_session(token_hash).await?;
        let row = sqlx::query(
            "INSERT INTO auth_session \
             (session_hash, tenant_id, user_id, display_name, roles, expires_at) \
             VALUES ($1, $2, $3, $4, $5, \
                     transaction_timestamp() + ($6::bigint * interval '1 second')) \
             RETURNING session_hash, tenant_id, user_id, display_name, roles, \
                       floor(extract(epoch FROM created_at) * 1000)::bigint AS created_at_millis, \
                       floor(extract(epoch FROM expires_at) * 1000)::bigint AS expires_at_millis",
        )
        .bind(token_hash.to_string())
        .bind(subject.tenant().as_uuid())
        .bind(subject.user().as_uuid())
        .bind(subject.display_name())
        .bind(Json(subject.roles().to_vec()))
        .bind(i64::from(lifetime.as_seconds()))
        .fetch_one(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let record = decode_session_row(&row)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(record)
    }

    async fn resolve_session(
        &self,
        token_hash: SessionTokenHash,
    ) -> Result<Option<SessionRecord>, StoreError> {
        let mut transaction = self.begin_session(token_hash).await?;
        let row = sqlx::query(
            "SELECT session_hash, tenant_id, user_id, display_name, roles, \
                    floor(extract(epoch FROM created_at) * 1000)::bigint AS created_at_millis, \
                    floor(extract(epoch FROM expires_at) * 1000)::bigint AS expires_at_millis \
             FROM auth_session \
             WHERE session_hash = $1 AND revoked_at IS NULL \
                   AND expires_at > transaction_timestamp()",
        )
        .bind(token_hash.to_string())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let record = row.as_ref().map(decode_session_row).transpose()?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(record)
    }

    async fn revoke_session(&self, token_hash: SessionTokenHash) -> Result<(), StoreError> {
        let mut transaction = self.begin_session(token_hash).await?;
        sqlx::query(
            "UPDATE auth_session SET revoked_at = transaction_timestamp() \
             WHERE session_hash = $1 AND revoked_at IS NULL",
        )
        .bind(token_hash.to_string())
        .execute(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        transaction.commit().await.map_err(map_sqlx_error)
    }
}

#[cfg(feature = "postgres")]
#[async_trait]
impl RetentionStore for PostgresStore {
    async fn configure_retention_policy(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        policy: InstitutionRetentionPolicy,
    ) -> Result<(), StoreError> {
        let mut transaction = self.begin_tenant(context).await?;
        let changed: bool =
            sqlx::query_scalar("SELECT ple_configure_retention_policy($1, $2, $3, $4)")
                .bind(session.to_string())
                .bind(i32::from(policy.notify_after().get()))
                .bind(i32::from(policy.archive_after().get()))
                .bind(i32::from(policy.delete_after().get()))
                .fetch_one(&mut *transaction)
                .await
                .map_err(map_sqlx_error)?;
        if !changed {
            return Err(StoreError::Forbidden);
        }
        transaction.commit().await.map_err(map_sqlx_error)
    }

    async fn end_course_retention(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        course: CourseId,
    ) -> Result<CourseRetentionRecord, StoreError> {
        let mut transaction = self.begin_tenant(context).await?;
        let ended: bool = sqlx::query_scalar("SELECT ple_end_course_retention($1, $2)")
            .bind(session.to_string())
            .bind(course.as_uuid())
            .fetch_one(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        if !ended {
            return Err(StoreError::Forbidden);
        }
        let row = sqlx::query("SELECT * FROM ple_read_course_retention($1, $2)")
            .bind(session.to_string())
            .bind(course.as_uuid())
            .fetch_optional(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?
            .ok_or(StoreError::NotFound)?;
        let record = decode_retention_record(&row)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(record)
    }

    async fn course_retention(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        course: CourseId,
    ) -> Result<Option<CourseRetentionRecord>, StoreError> {
        let mut transaction = self.begin_tenant(context).await?;
        let row = sqlx::query("SELECT * FROM ple_read_course_retention($1, $2)")
            .bind(session.to_string())
            .bind(course.as_uuid())
            .fetch_optional(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        let record = row.as_ref().map(decode_retention_record).transpose()?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(record)
    }
}

#[cfg(feature = "postgres")]
#[async_trait]
impl RetentionScheduleStore for PostgresStore {
    async fn dispatch_due_retention_stages(
        &self,
        batch: RetentionDispatchBatch,
    ) -> Result<u16, StoreError> {
        let mut transaction = self.begin_app().await?;
        let count: i64 = sqlx::query_scalar("SELECT ple_dispatch_due_retention_stages($1)")
            .bind(i32::from(batch.get()))
            .fetch_one(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        let count = u16::try_from(count).map_err(|_| {
            StoreError::Unavailable("retention broker returned invalid dispatch count".to_string())
        })?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(count)
    }

    async fn extend_course_retention(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        course: CourseId,
        additional_days: RetentionDays,
    ) -> Result<CourseRetentionRecord, StoreError> {
        let mut transaction = self.begin_tenant(context).await?;
        let changed: bool = sqlx::query_scalar("SELECT ple_extend_course_retention($1, $2, $3)")
            .bind(session.to_string())
            .bind(course.as_uuid())
            .bind(i32::from(additional_days.get()))
            .fetch_one(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        if !changed {
            return Err(StoreError::Conflict);
        }
        let row = sqlx::query("SELECT * FROM ple_read_course_retention($1, $2)")
            .bind(session.to_string())
            .bind(course.as_uuid())
            .fetch_optional(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?
            .ok_or(StoreError::NotFound)?;
        let record = decode_retention_record(&row)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(record)
    }

    async fn set_archive_disposition(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        course: CourseId,
        disposition: AssignmentDefinitionDisposition,
    ) -> Result<CourseRetentionRecord, StoreError> {
        let mut transaction = self.begin_tenant(context).await?;
        let disposition = match disposition {
            AssignmentDefinitionDisposition::Retain => "retain",
            AssignmentDefinitionDisposition::Delete => "delete",
        };
        let changed: bool = sqlx::query_scalar("SELECT ple_set_archive_disposition($1, $2, $3)")
            .bind(session.to_string())
            .bind(course.as_uuid())
            .bind(disposition)
            .fetch_one(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        if !changed {
            return Err(StoreError::Conflict);
        }
        let row = sqlx::query("SELECT * FROM ple_read_course_retention($1, $2)")
            .bind(session.to_string())
            .bind(course.as_uuid())
            .fetch_optional(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?
            .ok_or(StoreError::NotFound)?;
        let record = decode_retention_record(&row)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(record)
    }
}

#[cfg(feature = "postgres")]
#[async_trait]
impl RetentionApiStore for PostgresStore {
    async fn retention_view(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        course: CourseId,
    ) -> Result<Option<CourseRetentionView>, StoreError> {
        let mut transaction = self.begin_tenant(context).await?;
        let row = sqlx::query("SELECT * FROM ple_read_course_retention($1, $2)")
            .bind(session.to_string())
            .bind(course.as_uuid())
            .fetch_optional(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        let view = row
            .as_ref()
            .map(decode_retention_record)
            .transpose()?
            .map(|record| {
                record
                    .safe_view()
                    .map_err(|error| StoreError::InvalidRecord(error.to_string()))
            })
            .transpose()?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(view)
    }

    async fn retention_notification(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        course: CourseId,
    ) -> Result<Option<crate::RetentionNotificationView>, StoreError> {
        let mut transaction = self.begin_tenant(context).await?;
        let row = sqlx::query("SELECT * FROM ple_read_retention_notification($1, $2)")
            .bind(session.to_string())
            .bind(course.as_uuid())
            .fetch_optional(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        let Some(row) = row else {
            return Ok(None);
        };
        let intent: String = row.try_get("intent").map_err(map_sqlx_error)?;
        let created_at_millis: i64 = row.try_get("created_at_millis").map_err(map_sqlx_error)?;
        match intent.as_str() {
            "archive" => Ok(Some(crate::RetentionNotificationView {
                intent: crate::RetentionNotificationIntent::Archive,
                created_at: ActivityTimestamp::from_unix_millis(created_at_millis),
            })),
            "delete" => Ok(Some(crate::RetentionNotificationView {
                intent: crate::RetentionNotificationIntent::Delete,
                created_at: ActivityTimestamp::from_unix_millis(created_at_millis),
            })),
            "extend" => Ok(Some(crate::RetentionNotificationView {
                intent: crate::RetentionNotificationIntent::Extend,
                created_at: ActivityTimestamp::from_unix_millis(created_at_millis),
            })),
            _ => Err(StoreError::InvalidRecord(
                "invalid retention notification intent".to_string(),
            )),
        }
    }

    async fn extend_retention_if_revision(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        course: CourseId,
        expected: RetentionRevision,
        additional_days: RetentionDays,
    ) -> Result<CourseRetentionView, StoreError> {
        let (retention, outcome) = self
            .apply_retention_api_action(
                context,
                session,
                course,
                expected,
                RetentionApiAction::Extend(additional_days),
            )
            .await?;
        let _ = outcome;
        Ok(retention)
    }

    async fn request_retention_archive_if_revision(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        course: CourseId,
        expected: RetentionRevision,
        disposition: AssignmentDefinitionDisposition,
    ) -> Result<crate::RetentionRequestResult, StoreError> {
        let (retention, outcome) = self
            .apply_retention_api_action(
                context,
                session,
                course,
                expected,
                RetentionApiAction::Archive(disposition),
            )
            .await?;
        Ok(crate::RetentionRequestResult { retention, outcome })
    }

    async fn request_retention_delete_if_revision(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        course: CourseId,
        expected: RetentionRevision,
    ) -> Result<crate::RetentionRequestResult, StoreError> {
        let (retention, outcome) = self
            .apply_retention_api_action(
                context,
                session,
                course,
                expected,
                RetentionApiAction::Delete,
            )
            .await?;
        Ok(crate::RetentionRequestResult { retention, outcome })
    }
}

#[cfg(feature = "postgres")]
impl PostgresStore {
    async fn apply_retention_api_action(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        course: CourseId,
        expected: RetentionRevision,
        action: RetentionApiAction,
    ) -> Result<(CourseRetentionView, crate::RetentionRequestOutcome), StoreError> {
        let mut transaction = self.begin_tenant(context).await?;
        let (action, additional_days, disposition) = match action {
            RetentionApiAction::Extend(days) => ("extend", Some(i32::from(days.get())), None),
            RetentionApiAction::Archive(AssignmentDefinitionDisposition::Retain) => {
                ("archive", None, Some("retain"))
            }
            RetentionApiAction::Archive(AssignmentDefinitionDisposition::Delete) => {
                ("archive", None, Some("delete"))
            }
            RetentionApiAction::Delete => ("delete", None, None),
        };
        let outcome: Option<String> =
            sqlx::query_scalar("SELECT ple_apply_retention_api_action($1, $2, $3, $4, $5, $6)")
                .bind(session.to_string())
                .bind(course.as_uuid())
                .bind(i64::try_from(expected.value()).map_err(|_| StoreError::Conflict)?)
                .bind(action)
                .bind(additional_days)
                .bind(disposition)
                .fetch_one(&mut *transaction)
                .await
                .map_err(map_sqlx_error)?;
        let Some(outcome) = outcome else {
            return Err(StoreError::Conflict);
        };
        let row = sqlx::query("SELECT * FROM ple_read_course_retention($1, $2)")
            .bind(session.to_string())
            .bind(course.as_uuid())
            .fetch_optional(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?
            .ok_or(StoreError::NotFound)?;
        let view = decode_retention_record(&row)?
            .safe_view()
            .map_err(|error| StoreError::InvalidRecord(error.to_string()))?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        let outcome = match outcome.as_str() {
            "scheduled" | "changed" => crate::RetentionRequestOutcome::Scheduled,
            "inProgress" => crate::RetentionRequestOutcome::InProgress,
            "completed" => crate::RetentionRequestOutcome::Completed,
            _ => {
                return Err(StoreError::InvalidRecord(
                    "invalid retention API outcome".to_string(),
                ));
            }
        };
        Ok((view, outcome))
    }
}

#[cfg(feature = "postgres")]
#[async_trait]
impl RetentionWorkerStore for PostgresStore {
    async fn prepare_retention_work(
        &self,
        command: RetentionWorkerCommand,
    ) -> Result<RetentionWork, StoreError> {
        let mut transaction = self
            .begin_tenant(TenantContext::from_authenticated_session(command.tenant))
            .await?;
        let value: Option<Value> =
            sqlx::query_scalar("SELECT ple_prepare_retention_work($1,$2,$3,$4,$5,$6)")
                .bind(command.tenant.as_uuid())
                .bind(command.job.as_uuid())
                .bind(command.lease.as_uuid())
                .bind(command.course.as_uuid())
                .bind(retention_stage_db(command.stage))
                .bind(i64::try_from(command.generation).map_err(|_| {
                    StoreError::InvalidRecord(
                        "retention generation exceeds database range".to_string(),
                    )
                })?)
                .fetch_one(&mut *transaction)
                .await
                .map_err(map_sqlx_error)?;
        let Some(value) = value else {
            return Err(StoreError::Conflict);
        };
        let kind = value.get("kind").and_then(Value::as_str).ok_or_else(|| {
            StoreError::Unavailable("stored retention work is invalid".to_string())
        })?;
        let work = match kind {
            "notify" => RetentionWork::Notify,
            "cleanup" => {
                let values = value
                    .get("objects")
                    .and_then(Value::as_array)
                    .ok_or_else(|| {
                        StoreError::Unavailable(
                            "stored retention object manifest is invalid".to_string(),
                        )
                    })?;
                let mut objects = Vec::with_capacity(values.len());
                for value in values {
                    let raw = value.as_str().ok_or_else(|| {
                        StoreError::Unavailable(
                            "stored retention object manifest is invalid".to_string(),
                        )
                    })?;
                    let object = uuid::Uuid::parse_str(raw).map_err(|_| {
                        StoreError::Unavailable(
                            "stored retention object manifest is invalid".to_string(),
                        )
                    })?;
                    objects.push(objects::ObjectKey::StudentRecord {
                        tenant: command.tenant,
                        object: ObjectId::from_uuid(object),
                    });
                }
                RetentionWork::Cleanup(RetentionCleanupManifest { objects })
            }
            _ => {
                return Err(StoreError::Unavailable(
                    "stored retention work kind is invalid".to_string(),
                ));
            }
        };
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(work)
    }

    async fn commit_retention_work(
        &self,
        command: RetentionWorkerCommand,
    ) -> Result<(), StoreError> {
        let mut transaction = self
            .begin_tenant(TenantContext::from_authenticated_session(command.tenant))
            .await?;
        let committed: bool =
            sqlx::query_scalar("SELECT ple_commit_retention_work($1,$2,$3,$4,$5,$6)")
                .bind(command.tenant.as_uuid())
                .bind(command.job.as_uuid())
                .bind(command.lease.as_uuid())
                .bind(command.course.as_uuid())
                .bind(retention_stage_db(command.stage))
                .bind(i64::try_from(command.generation).map_err(|_| {
                    StoreError::InvalidRecord(
                        "retention generation exceeds database range".to_string(),
                    )
                })?)
                .fetch_one(&mut *transaction)
                .await
                .map_err(map_sqlx_error)?;
        if !committed {
            return Err(StoreError::Conflict);
        }
        transaction.commit().await.map_err(map_sqlx_error)
    }
}

#[cfg(feature = "postgres")]
fn retention_stage_db(stage: crate::RetentionStage) -> &'static str {
    match stage {
        crate::RetentionStage::Notify => "notify",
        crate::RetentionStage::ArchiveStudentRecords => "archiveStudentRecords",
        crate::RetentionStage::DeleteStudentRecords => "deleteStudentRecords",
    }
}

#[cfg(feature = "postgres")]
fn decode_retention_policy(row: &PgRow) -> Result<InstitutionRetentionPolicy, StoreError> {
    let notify: i32 = row.try_get("notify_days").map_err(map_sqlx_error)?;
    let archive: i32 = row.try_get("archive_days").map_err(map_sqlx_error)?;
    let delete: i32 = row.try_get("delete_days").map_err(map_sqlx_error)?;
    let days = |value| {
        RetentionDays::new(u16::try_from(value).map_err(|_| {
            StoreError::Unavailable("stored retention policy is invalid".to_string())
        })?)
        .map_err(|error| StoreError::Unavailable(error.to_string()))
    };
    InstitutionRetentionPolicy::new(days(notify)?, days(archive)?, days(delete)?)
        .map_err(|error| StoreError::Unavailable(error.to_string()))
}

#[cfg(feature = "postgres")]
fn decode_retention_record(row: &PgRow) -> Result<CourseRetentionRecord, StoreError> {
    let ended_at: i64 = row.try_get("ended_at_millis").map_err(map_sqlx_error)?;
    let generation: i64 = row.try_get("generation").map_err(map_sqlx_error)?;
    let lifecycle: String = row.try_get("lifecycle").map_err(map_sqlx_error)?;
    let disposition: String = row
        .try_get("assignment_disposition")
        .map_err(map_sqlx_error)?;
    let disposition = match disposition.as_str() {
        "retain" => AssignmentDefinitionDisposition::Retain,
        "delete" => AssignmentDefinitionDisposition::Delete,
        _ => {
            return Err(StoreError::Unavailable(
                "stored retention disposition is invalid".to_string(),
            ));
        }
    };
    let state = match lifecycle.as_str() {
        "active" => CourseRetentionState::Active,
        "archived" => CourseRetentionState::StudentRecordsArchived,
        "deleted" => CourseRetentionState::StudentRecordsDeleted,
        _ => {
            return Err(StoreError::Unavailable(
                "stored retention lifecycle is invalid".to_string(),
            ));
        }
    };
    Ok(CourseRetentionRecord {
        snapshot: CourseRetentionSnapshot::new(
            ActivityTimestamp::from_unix_millis(ended_at),
            decode_retention_policy(row)?,
            disposition,
            u64::try_from(generation).map_err(|_| {
                StoreError::Unavailable("stored retention generation is invalid".to_string())
            })?,
        )
        .map_err(|error| StoreError::Unavailable(error.to_string()))?,
        status: crate::CourseRetentionStatus::from_persisted(state, disposition),
    })
}

/// Builds a lazy connection pool.
///
/// Lazy on purpose: the API can start and report degraded health while the
/// database is unavailable instead of disappearing from the orchestrator.
///
/// # Errors
///
/// Returns an error when `database_url` is not a valid connection string.
#[cfg(feature = "postgres")]
pub fn lazy_pool(database_url: &str) -> Result<PgPool, sqlx::Error> {
    PgPoolOptions::new()
        .max_connections(8)
        .connect_lazy(database_url)
}

#[cfg(feature = "postgres")]
fn evaluate_migration_status(
    ledger_present: bool,
    applied: Vec<AppliedMigrationState>,
) -> MigrationStatus {
    let mut applied_by_version = applied
        .into_iter()
        .map(|migration| (migration.version, migration))
        .collect::<BTreeMap<_, _>>();
    let entries = MIGRATOR
        .iter()
        .filter(|migration| !migration.migration_type.is_down_migration())
        .map(|migration| {
            let disposition = match applied_by_version.remove(&migration.version) {
                None => MigrationDisposition::Pending,
                Some(applied) if !applied.success => MigrationDisposition::Dirty,
                Some(applied) if applied.checksum.as_slice() != migration.checksum.as_ref() => {
                    MigrationDisposition::Modified
                }
                Some(_) => MigrationDisposition::Applied,
            };
            MigrationStatusEntry {
                version: migration.version,
                description: migration.description.to_string(),
                disposition,
            }
        })
        .collect();
    MigrationStatus {
        ledger_present,
        entries,
        unexpected_applied_versions: applied_by_version.into_keys().collect(),
    }
}

#[cfg(feature = "postgres")]
fn undefined_relation(error: &sqlx::Error) -> bool {
    matches!(
        error,
        sqlx::Error::Database(database_error)
            if database_error.code().as_deref() == Some("42P01")
    )
}

#[cfg(feature = "postgres")]
async fn read_migration_rows(
    pool: &PgPool,
) -> Result<(bool, Vec<AppliedMigrationState>), sqlx::Error> {
    let rows = match sqlx::query(
        "SELECT version, success, checksum FROM public._sqlx_migrations ORDER BY version",
    )
    .fetch_all(pool)
    .await
    {
        Ok(rows) => rows,
        Err(error) if undefined_relation(&error) => return Ok((false, Vec::new())),
        Err(error) => return Err(error),
    };
    let applied = rows
        .into_iter()
        .map(|row| {
            Ok(AppliedMigrationState {
                version: row.try_get("version")?,
                success: row.try_get("success")?,
                checksum: row.try_get("checksum")?,
            })
        })
        .collect::<Result<Vec<_>, sqlx::Error>>()?;
    Ok((true, applied))
}

/// Reports known, pending, dirty, modified, and unexpected migrations without mutation.
///
/// A database with no SQLx ledger is reported as a clean pending epoch so the
/// migration command can explain what it will apply.
///
/// # Errors
///
/// Returns a database error when PostgreSQL is unreachable or the ledger cannot
/// be read safely.
#[cfg(feature = "postgres")]
pub async fn migration_status(pool: &PgPool) -> Result<MigrationStatus, sqlx::Error> {
    let (ledger_present, applied) = read_migration_rows(pool).await?;
    Ok(evaluate_migration_status(ledger_present, applied))
}

/// Verifies the exact application-visible schema epoch through a read-only transaction.
///
/// This deliberately queries the narrow `ple_migration_state` projection as
/// `ple_app`; application startup never creates the SQLx ledger or applies DDL.
///
/// # Errors
///
/// Returns [`SchemaCompatibilityError::Unavailable`] when PostgreSQL cannot be
/// reached. A reachable database with a missing projection, rejected app role,
/// unknown version, dirty row, pending migration, or checksum mismatch returns
/// [`SchemaCompatibilityError::Incompatible`].
#[cfg(feature = "postgres")]
pub async fn verify_application_schema(pool: &PgPool) -> Result<(), SchemaCompatibilityError> {
    let mut transaction = pool
        .begin()
        .await
        .map_err(|_| SchemaCompatibilityError::Unavailable)?;
    sqlx::query("SET TRANSACTION READ ONLY")
        .execute(&mut *transaction)
        .await
        .map_err(|_| SchemaCompatibilityError::Unavailable)?;
    sqlx::query("SET LOCAL ROLE ple_app")
        .execute(&mut *transaction)
        .await
        .map_err(|_| {
            SchemaCompatibilityError::Incompatible(
                "the application principal is unavailable".to_string(),
            )
        })?;
    let rows = sqlx::query(
        "SELECT version, success, checksum FROM public.ple_migration_state ORDER BY version",
    )
    .fetch_all(&mut *transaction)
    .await
    .map_err(|_| {
        SchemaCompatibilityError::Incompatible(
            "the migration-state projection is unavailable".to_string(),
        )
    })?;
    let applied = rows
        .into_iter()
        .map(|row| {
            let version = row.try_get("version").map_err(|_| {
                SchemaCompatibilityError::Incompatible(
                    "the migration-state projection has an invalid version".to_string(),
                )
            })?;
            let success = row.try_get("success").map_err(|_| {
                SchemaCompatibilityError::Incompatible(
                    "the migration-state projection has an invalid state".to_string(),
                )
            })?;
            let checksum = row.try_get("checksum").map_err(|_| {
                SchemaCompatibilityError::Incompatible(
                    "the migration-state projection has an invalid checksum".to_string(),
                )
            })?;
            Ok(AppliedMigrationState {
                version,
                success,
                checksum,
            })
        })
        .collect::<Result<Vec<_>, SchemaCompatibilityError>>()?;
    let status = evaluate_migration_status(true, applied);
    if !status.is_compatible() {
        return Err(SchemaCompatibilityError::Incompatible(
            status.incompatibility_reason(),
        ));
    }
    transaction
        .commit()
        .await
        .map_err(|_| SchemaCompatibilityError::Unavailable)?;
    Ok(())
}

/// Applies every embedded, checksummed schema migration in version order.
///
/// # Errors
///
/// Returns a database or migration-integrity failure.
#[cfg(feature = "postgres")]
pub async fn apply_migrations(pool: &PgPool) -> Result<(), sqlx::Error> {
    MIGRATOR.run(pool).await?;
    Ok(())
}

/// Runs a real query against PostgreSQL.
///
/// # Errors
///
/// Returns an error when the database is unreachable or rejects the query.
#[cfg(feature = "postgres")]
pub async fn ping(pool: &PgPool) -> Result<(), sqlx::Error> {
    sqlx::query("SELECT 1").execute(pool).await?;
    Ok(())
}

#[cfg(all(test, feature = "postgres"))]
mod migration_tests {
    use super::*;

    fn exact_applied_epoch() -> Vec<AppliedMigrationState> {
        MIGRATOR
            .iter()
            .filter(|migration| !migration.migration_type.is_down_migration())
            .map(|migration| AppliedMigrationState {
                version: migration.version,
                success: true,
                checksum: migration.checksum.to_vec(),
            })
            .collect()
    }

    #[test]
    fn exact_successful_epoch_is_compatible() {
        let status = evaluate_migration_status(true, exact_applied_epoch());

        assert!(status.is_compatible());
        assert!(
            status
                .entries()
                .iter()
                .all(|entry| entry.disposition() == MigrationDisposition::Applied)
        );
    }

    #[test]
    fn absent_known_migration_is_pending() {
        let mut applied = exact_applied_epoch();
        let missing = applied.remove(0).version;

        let status = evaluate_migration_status(true, applied);

        assert!(!status.is_compatible());
        assert!(status.entries().iter().any(|entry| {
            entry.version() == missing && entry.disposition() == MigrationDisposition::Pending
        }));
    }

    #[test]
    fn checksum_change_is_modified() {
        let mut applied = exact_applied_epoch();
        let modified = applied
            .first_mut()
            .expect("embedded database epoch has a first migration");
        modified.checksum[0] ^= 0xff;
        let version = modified.version;

        let status = evaluate_migration_status(true, applied);

        assert!(status.entries().iter().any(|entry| {
            entry.version() == version && entry.disposition() == MigrationDisposition::Modified
        }));
    }

    #[test]
    fn failed_and_unknown_versions_are_incompatible() {
        let mut applied = exact_applied_epoch();
        applied
            .first_mut()
            .expect("embedded database epoch has a first migration")
            .success = false;
        applied.push(AppliedMigrationState {
            version: i64::MAX,
            success: true,
            checksum: vec![0; 48],
        });

        let status = evaluate_migration_status(true, applied);

        assert!(!status.is_compatible());
        assert_eq!(status.unexpected_applied_versions(), &[i64::MAX]);
        assert!(
            status
                .entries()
                .iter()
                .any(|entry| entry.disposition() == MigrationDisposition::Dirty)
        );
    }
}

#[cfg(feature = "postgres")]
#[async_trait]
impl QtiImportStore for PostgresStore {
    async fn prepare_qti_import(
        &self,
        context: TenantContext,
        command: CreateQtiImportCommand,
    ) -> Result<(), StoreError> {
        // This is deliberately not implemented as `create` followed by an
        // UPDATE.  A committed row, however briefly present, is observable by
        // a concurrent request and would leak an incomplete import.
        ensure_tenant(context, command.registry.reference.tenant)?;
        validate_qti_import(&command)?;
        let reference = command.registry.reference;
        let (registry_payload, registry_checksum) = encode_payload(&command.registry)?;
        let grading_checksums = Json(Value::Object(
            command
                .item_bindings
                .iter()
                .map(|binding| {
                    (
                        binding.item.item_id.clone(),
                        Value::String(binding.grading.sha256().to_string()),
                    )
                })
                .collect(),
        ));
        let mut transaction = self.begin_tenant(context).await?;
        // Serialize same-import preparation. Hash collisions only add waiting;
        // equality is still checked against the complete typed reference.
        let preparation_lock = format!(
            "{}:{}:{}",
            reference.tenant, reference.workspace, reference.import
        );
        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1, 0))")
            .bind(preparation_lock)
            .execute(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        let matches_prepared: bool =
            sqlx::query_scalar("SELECT ple_prepared_qti_import_matches($1, $2, $3, $4, $5, $6)")
                .bind(reference.tenant.as_uuid())
                .bind(reference.workspace.as_uuid())
                .bind(reference.import.as_uuid())
                .bind(registry_payload.clone())
                .bind(&registry_checksum)
                .bind(grading_checksums)
                .fetch_one(&mut *transaction)
                .await
                .map_err(map_sqlx_error)?;
        if matches_prepared {
            transaction.commit().await.map_err(map_sqlx_error)?;
            return Ok(());
        }
        sqlx::query(
            "INSERT INTO workspace_qti_import \
             (tenant_id, workspace_id, import_id, source_object_id, payload, payload_sha256, state) \
             VALUES ($1, $2, $3, $4, $5, $6, 'prepared')",
        )
        .bind(reference.tenant.as_uuid())
        .bind(reference.workspace.as_uuid())
        .bind(reference.import.as_uuid())
        .bind(command.registry.source.id.as_uuid())
        .bind(registry_payload)
        .bind(registry_checksum)
        .execute(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        for binding in &command.item_bindings {
            let (item_payload, item_checksum) = encode_payload(&binding.item)?;
            sqlx::query(
                "INSERT INTO workspace_qti_import_item \
                 (tenant_id, workspace_id, import_id, item_id, payload, payload_sha256) \
                 VALUES ($1, $2, $3, $4, $5, $6)",
            )
            .bind(reference.tenant.as_uuid())
            .bind(reference.workspace.as_uuid())
            .bind(reference.import.as_uuid())
            .bind(&binding.item.item_id)
            .bind(item_payload)
            .bind(item_checksum)
            .execute(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
            sqlx::query(
                "INSERT INTO workspace_qti_import_grading \
                 (tenant_id, workspace_id, import_id, item_id, payload, payload_sha256) \
                 VALUES ($1, $2, $3, $4, $5, $6)",
            )
            .bind(reference.tenant.as_uuid())
            .bind(reference.workspace.as_uuid())
            .bind(reference.import.as_uuid())
            .bind(&binding.item.item_id)
            .bind(binding.grading.bytes())
            .bind(Sha256Digest::compute(binding.grading.bytes()).to_string())
            .execute(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        }
        for asset in &command.registry.assets {
            let objects::ObjectKey::WorkspaceAsset {
                asset: logical_asset,
                ..
            } = &asset.key
            else {
                return Err(StoreError::InvalidRecord(
                    "validated QTI asset lost its logical identity".to_string(),
                ));
            };
            let (payload, checksum) = encode_payload(asset)?;
            sqlx::query(
                "INSERT INTO workspace_qti_import_asset \
                 (tenant_id, workspace_id, import_id, asset_id, object_id, payload, payload_sha256) \
                 VALUES ($1, $2, $3, $4, $5, $6, $7)",
            )
            .bind(reference.tenant.as_uuid())
            .bind(reference.workspace.as_uuid())
            .bind(reference.import.as_uuid())
            .bind(logical_asset.as_uuid())
            .bind(asset.id.as_uuid())
            .bind(payload)
            .bind(checksum)
            .execute(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        }
        for (ordinal, feature) in command.registry.unsupported_features.iter().enumerate() {
            let (payload, checksum) = encode_payload(feature)?;
            sqlx::query(
                "INSERT INTO workspace_qti_import_unsupported \
                 (tenant_id, workspace_id, import_id, ordinal, payload, payload_sha256) \
                 VALUES ($1, $2, $3, $4, $5, $6)",
            )
            .bind(reference.tenant.as_uuid())
            .bind(reference.workspace.as_uuid())
            .bind(reference.import.as_uuid())
            .bind(i32::try_from(ordinal).map_err(|_| {
                StoreError::InvalidRecord("too many QTI unsupported features".to_string())
            })?)
            .bind(payload)
            .bind(checksum)
            .execute(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        }
        transaction.commit().await.map_err(map_sqlx_error)
    }

    async fn commit_prepared_qti_import(
        &self,
        context: TenantContext,
        command: CommitPreparedQtiImport,
    ) -> Result<CommitPreparedQtiImportOutcome, StoreError> {
        ensure_tenant(context, command.reference.tenant)?;
        let mut transaction = self.begin_tenant(context).await?;
        let committed: bool =
            sqlx::query_scalar("SELECT ple_commit_prepared_qti_import($1, $2, $3, $4, $5, $6)")
                .bind(context.tenant_id().as_uuid())
                .bind(command.job.as_uuid())
                .bind(command.lease.as_uuid())
                .bind(command.reference.workspace.as_uuid())
                .bind(command.reference.import.as_uuid())
                .bind(command.source_object.as_uuid())
                .fetch_one(&mut *transaction)
                .await
                .map_err(map_sqlx_error)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(if committed {
            CommitPreparedQtiImportOutcome::Committed
        } else {
            CommitPreparedQtiImportOutcome::ClaimNoLongerActive
        })
    }

    async fn get_qti_import(
        &self,
        context: TenantContext,
        workspace: WorkspaceId,
        import: WorkspaceImportId,
    ) -> Result<Option<QtiImportRegistry>, StoreError> {
        let mut transaction = self.begin_tenant(context).await?;
        let row = sqlx::query(
            "SELECT payload, payload_sha256 FROM ple_read_committed_qti_import($1, $2, $3)",
        )
        .bind(context.tenant_id().as_uuid())
        .bind(workspace.as_uuid())
        .bind(import.as_uuid())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let record = row.as_ref().map(decode_payload_row).transpose()?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(record)
    }
}

#[cfg(feature = "postgres")]
#[async_trait]
impl QtiGradingStore for PostgresQtiGraderStore {
    async fn qti_import_grading(
        &self,
        context: TenantContext,
        workspace: WorkspaceId,
        import: WorkspaceImportId,
        item_id: &str,
    ) -> Result<Option<QtiImportGradingPayload>, StoreError> {
        let mut transaction = self.begin_grader_tenant(context).await?;
        let row = sqlx::query(
            "SELECT payload, payload_sha256 FROM ple_read_committed_qti_grading($1, $2, $3, $4)",
        )
        .bind(context.tenant_id().as_uuid())
        .bind(workspace.as_uuid())
        .bind(import.as_uuid())
        .bind(item_id)
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let material = row
            .as_ref()
            .map(|row| {
                let bytes: Vec<u8> = row.try_get("payload").map_err(map_sqlx_error)?;
                let expected: String = row.try_get("payload_sha256").map_err(map_sqlx_error)?;
                if Sha256Digest::compute(&bytes).to_string() != expected {
                    return Err(StoreError::Unavailable(
                        "stored QTI grading payload checksum mismatch".to_string(),
                    ));
                }
                QtiImportGradingPayload::new(bytes)
            })
            .transpose()?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(material)
    }

    async fn qti_published_grading(
        &self,
        context: TenantContext,
        reference: ProblemVersionRef,
        item_id: &str,
    ) -> Result<Option<QtiImportGradingPayload>, StoreError> {
        let mut transaction = self.begin_grader_tenant(context).await?;
        let row = sqlx::query(
            "SELECT payload, payload_sha256 FROM ple_read_published_qti_grading($1, $2, $3, $4)",
        )
        .bind(context.tenant_id().as_uuid())
        .bind(reference.problem.as_uuid())
        .bind(reference.version.as_uuid())
        .bind(item_id)
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let material = decode_qti_grading_row(row.as_ref())?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(material)
    }
}

#[cfg(feature = "postgres")]
fn question_statistics_disclosure_from_row(
    row: Option<&PgRow>,
) -> Result<QuestionStatisticsDisclosure, StoreError> {
    let Some(row) = row else {
        return Ok(QuestionStatisticsDisclosure::Suppressed);
    };
    let cohort_size: i64 = row.try_get("cohort_size").map_err(map_sqlx_error)?;
    let time_median_seconds_estimate: i64 = row
        .try_get("time_median_seconds_estimate")
        .map_err(map_sqlx_error)?;
    let view = QuestionStatisticsView {
        cohort_size: u64::try_from(cohort_size).map_err(|_| {
            StoreError::Unavailable("stored statistics cohort is invalid".to_string())
        })?,
        difficulty_index: row.try_get("difficulty_index").map_err(map_sqlx_error)?,
        attempts_mean: row.try_get("attempts_mean").map_err(map_sqlx_error)?,
        time_median_seconds_estimate: u64::try_from(time_median_seconds_estimate).map_err(
            |_| StoreError::Unavailable("stored statistics duration is invalid".to_string()),
        )?,
        discrimination_index: row
            .try_get("discrimination_index")
            .map_err(map_sqlx_error)?,
    };
    Ok(QuestionStatisticsDisclosure::Available(view))
}

#[cfg(feature = "postgres")]
#[async_trait]
impl Store for PostgresStore {
    async fn question_statistics(
        &self,
        context: TenantContext,
        reference: ProblemVersionRef,
    ) -> Result<QuestionStatisticsDisclosure, StoreError> {
        let mut transaction = self.begin_tenant(context).await?;
        let row = sqlx::query(
            "SELECT cohort_size, difficulty_index, attempts_mean, time_median_seconds_estimate, \
                    discrimination_index \
             FROM ple_question_statistics_view($1, $2)",
        )
        .bind(reference.problem.as_uuid())
        .bind(reference.version.as_uuid())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        question_statistics_disclosure_from_row(row.as_ref())
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
        let (payload, checksum) = encode_payload(&draft)?;
        let mut transaction = self.begin_tenant(context).await?;
        let current: Option<i64> = sqlx::query_scalar(
            "SELECT revision FROM workspace_draft \
             WHERE tenant_id = $1 AND workspace_id = $2 FOR UPDATE",
        )
        .bind(draft.tenant.as_uuid())
        .bind(draft.question.workspace.as_uuid())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let revision = match current {
            Some(value) => {
                let current = WorkspaceDraftRevision::from_stored(value)?;
                let role: Option<String> = sqlx::query_scalar(
                    "SELECT role FROM workspace_draft_access \
                     WHERE tenant_id = $1 AND workspace_id = $2 AND user_id = $3",
                )
                .bind(draft.tenant.as_uuid())
                .bind(draft.question.workspace.as_uuid())
                .bind(actor.as_uuid())
                .fetch_optional(&mut *transaction)
                .await
                .map_err(map_sqlx_error)?;
                if !matches!(role.as_deref(), Some("owner" | "collaborator")) {
                    return Err(StoreError::Forbidden);
                }
                if expected_revision != Some(current) {
                    return Err(StoreError::Conflict);
                }
                let next = current.next()?;
                sqlx::query(
                    "UPDATE workspace_draft SET payload = $3, payload_sha256 = $4, \
                     revision = $5, updated_at = transaction_timestamp() \
                     WHERE tenant_id = $1 AND workspace_id = $2",
                )
                .bind(draft.tenant.as_uuid())
                .bind(draft.question.workspace.as_uuid())
                .bind(payload)
                .bind(checksum)
                .bind(i64::try_from(next.value()).map_err(|_| {
                    StoreError::Unavailable("workspace draft revision limit reached".to_string())
                })?)
                .execute(&mut *transaction)
                .await
                .map_err(map_sqlx_error)?;
                next
            }
            None => {
                if expected_revision.is_some() {
                    return Err(StoreError::Conflict);
                }
                sqlx::query(
                    "INSERT INTO workspace_draft \
                     (tenant_id, workspace_id, payload, payload_sha256, revision) \
                     VALUES ($1, $2, $3, $4, $5)",
                )
                .bind(draft.tenant.as_uuid())
                .bind(draft.question.workspace.as_uuid())
                .bind(payload)
                .bind(checksum)
                .bind(
                    i64::try_from(WorkspaceDraftRevision::INITIAL.value()).map_err(|_| {
                        StoreError::Unavailable(
                            "workspace draft revision limit reached".to_string(),
                        )
                    })?,
                )
                .execute(&mut *transaction)
                .await
                .map_err(map_sqlx_error)?;
                sqlx::query(
                    "INSERT INTO workspace_draft_access \
                     (tenant_id, workspace_id, user_id, role) VALUES ($1, $2, $3, 'owner')",
                )
                .bind(draft.tenant.as_uuid())
                .bind(draft.question.workspace.as_uuid())
                .bind(actor.as_uuid())
                .execute(&mut *transaction)
                .await
                .map_err(map_sqlx_error)?;
                WorkspaceDraftRevision::INITIAL
            }
        };
        transaction.commit().await.map_err(map_sqlx_error)?;
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
        let mut transaction = self.begin_tenant(context).await?;
        let row = sqlx::query(
            "SELECT d.payload, d.payload_sha256, d.revision FROM workspace_draft AS d \
             JOIN workspace_draft_access AS a \
               ON a.tenant_id = d.tenant_id AND a.workspace_id = d.workspace_id \
             WHERE d.tenant_id = $1 AND d.workspace_id = $2 AND a.user_id = $3",
        )
        .bind(context.tenant_id().as_uuid())
        .bind(workspace.as_uuid())
        .bind(actor.as_uuid())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let record = row
            .as_ref()
            .map(|row| {
                let record = decode_payload_row(row)?;
                let revision = WorkspaceDraftRevision::from_stored(
                    row.try_get("revision").map_err(map_sqlx_error)?,
                )?;
                Ok::<WorkspaceDraft, StoreError>(WorkspaceDraft { record, revision })
            })
            .transpose()?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(record)
    }

    async fn list_drafts(
        &self,
        context: TenantContext,
        actor: UserId,
        page: PageRequest,
    ) -> Result<Page<WorkspaceDraftSummary>, StoreError> {
        let after = page
            .after
            .as_ref()
            .map(|cursor| decode_workspace_draft_cursor(cursor.as_str(), context.tenant_id()))
            .transpose()?;
        let limit = i64::from(page.size.get()) + 1;
        let mut transaction = self.begin_tenant(context).await?;
        let rows = sqlx::query(
            "SELECT d.workspace_id, d.payload, d.payload_sha256 FROM workspace_draft AS d \
             JOIN workspace_draft_access AS a \
               ON a.tenant_id = d.tenant_id AND a.workspace_id = d.workspace_id \
             WHERE d.tenant_id = $1 AND a.user_id = $2 \
               AND ($3::uuid IS NULL OR d.workspace_id > $3) \
             ORDER BY d.workspace_id LIMIT $4",
        )
        .bind(context.tenant_id().as_uuid())
        .bind(actor.as_uuid())
        .bind(after.map(|workspace| workspace.as_uuid()))
        .bind(limit)
        .fetch_all(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let mut drafts = rows
            .iter()
            .map(|row| {
                let workspace: Uuid = row.try_get("workspace_id").map_err(map_sqlx_error)?;
                let draft: DraftRecord = decode_payload_row(row)?;
                if draft.tenant != context.tenant_id()
                    || draft.question.workspace.as_uuid() != workspace
                {
                    return Err(StoreError::Unavailable(
                        "stored workspace draft identity does not match its row".to_string(),
                    ));
                }
                Ok((
                    WorkspaceId::from_uuid(workspace),
                    draft.question.workspace_summary(),
                ))
            })
            .collect::<Result<Vec<_>, StoreError>>()?;
        let has_more = drafts.len() > usize::from(page.size.get());
        if has_more {
            drafts.pop();
        }
        let next_cursor = if has_more {
            drafts.last().map(|(workspace, _)| {
                Cursor::from_stable_key(encode_workspace_draft_cursor(
                    context.tenant_id(),
                    *workspace,
                ))
            })
        } else {
            None
        };
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(Page {
            items: drafts.into_iter().map(|(_, summary)| summary).collect(),
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
        let mut transaction = self.begin_tenant(context).await?;
        let deleted = sqlx::query_scalar::<_, Uuid>(
            "DELETE FROM workspace_draft AS d USING workspace_draft_access AS a \
             WHERE d.tenant_id = $1 AND d.workspace_id = $2 AND d.revision = $4 \
               AND a.tenant_id = d.tenant_id AND a.workspace_id = d.workspace_id \
               AND a.user_id = $3 AND a.role = 'owner' \
             RETURNING d.workspace_id",
        )
        .bind(context.tenant_id().as_uuid())
        .bind(workspace.as_uuid())
        .bind(actor.as_uuid())
        .bind(i64::try_from(expected_revision.value()).map_err(|_| {
            StoreError::Unavailable("workspace draft revision limit reached".to_string())
        })?)
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        if deleted.is_some() {
            transaction.commit().await.map_err(map_sqlx_error)?;
            return Ok(true);
        }

        // The delete predicate above is the authoritative atomic decision.
        // This follow-up only classifies its safe non-mutating failure for the
        // caller while preserving absent/foreign-tenant non-enumeration.
        let row = sqlx::query(
            "SELECT d.revision, a.role FROM workspace_draft AS d \
             LEFT JOIN workspace_draft_access AS a \
               ON a.tenant_id = d.tenant_id AND a.workspace_id = d.workspace_id AND a.user_id = $3 \
             WHERE d.tenant_id = $1 AND d.workspace_id = $2 \
             FOR UPDATE OF d",
        )
        .bind(context.tenant_id().as_uuid())
        .bind(workspace.as_uuid())
        .bind(actor.as_uuid())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let Some(row) = row else {
            transaction.commit().await.map_err(map_sqlx_error)?;
            return Ok(false);
        };
        let role: Option<String> = row.try_get("role").map_err(map_sqlx_error)?;
        if role.as_deref() != Some("owner") {
            return Err(StoreError::Forbidden);
        }
        let current =
            WorkspaceDraftRevision::from_stored(row.try_get("revision").map_err(map_sqlx_error)?)?;
        if current != expected_revision {
            return Err(StoreError::Conflict);
        }
        Err(StoreError::Conflict)
    }

    async fn grant_draft_collaborator(
        &self,
        context: TenantContext,
        actor: UserId,
        workspace: WorkspaceId,
        collaborator: UserId,
    ) -> Result<(), StoreError> {
        let mut transaction = self.begin_tenant(context).await?;
        let role: Option<String> = sqlx::query_scalar(
            "SELECT role FROM workspace_draft_access \
             WHERE tenant_id = $1 AND workspace_id = $2 AND user_id = $3",
        )
        .bind(context.tenant_id().as_uuid())
        .bind(workspace.as_uuid())
        .bind(actor.as_uuid())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        if role.as_deref() != Some("owner") {
            let exists: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM workspace_draft \
                 WHERE tenant_id = $1 AND workspace_id = $2)",
            )
            .bind(context.tenant_id().as_uuid())
            .bind(workspace.as_uuid())
            .fetch_one(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
            return Err(if exists {
                StoreError::Forbidden
            } else {
                StoreError::NotFound
            });
        }
        if collaborator != actor {
            sqlx::query(
                "INSERT INTO workspace_draft_access \
                 (tenant_id, workspace_id, user_id, role) VALUES ($1, $2, $3, 'collaborator') \
                 ON CONFLICT (tenant_id, workspace_id, user_id) DO NOTHING",
            )
            .bind(context.tenant_id().as_uuid())
            .bind(workspace.as_uuid())
            .bind(collaborator.as_uuid())
            .execute(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        }
        transaction.commit().await.map_err(map_sqlx_error)
    }

    async fn get_published_problem(
        &self,
        problem: question_model::ProblemId,
        version: VersionId,
    ) -> Result<Option<PublishedProblemRecord>, StoreError> {
        let mut transaction = self.begin_app().await?;
        let row = sqlx::query(
            "SELECT pv.problem_id, p.public_id, pv.version_id, pv.version_number, \
                    pvp.payload, pvp.payload_sha256, pv.lifecycle, pv.lifecycle_reason \
             FROM problem_version AS pv \
             JOIN problem AS p USING (problem_id) \
             JOIN problem_version_payload AS pvp USING (problem_id, version_id) \
             WHERE pv.problem_id = $1 AND pv.version_id = $2 \
               AND pv.publication_scope = 'public'",
        )
        .bind(problem.as_uuid())
        .bind(version.as_uuid())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let record = row.as_ref().map(decode_catalog_payload_row).transpose()?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(record)
    }

    async fn list_published_problems(
        &self,
        page: PageRequest,
    ) -> Result<Page<PublishedProblemRecord>, StoreError> {
        let cursor = page.after.as_ref().map(|value| value.as_str().to_string());
        let limit = i64::from(page.size.get()) + 1;
        let mut transaction = self.begin_app().await?;
        let rows = sqlx::query(
            "SELECT pv.problem_id::text || '/' || pv.version_id::text AS stable_key, \
                    payload, payload_sha256 \
             FROM problem_version AS pv \
             JOIN problem_version_payload AS pvp \
               USING (problem_id, version_id) \
             WHERE pv.publication_scope = 'public' \
               AND pv.lifecycle = 'published' \
               AND ($1::text IS NULL \
                    OR pv.problem_id::text || '/' || pv.version_id::text > $1) \
             ORDER BY pv.problem_id::text, pv.version_id::text \
             LIMIT $2",
        )
        .bind(cursor)
        .bind(limit)
        .fetch_all(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let result = page_from_rows(rows, page.size.get())?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(result)
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
        let mut transaction = self.begin_tenant(context).await?;
        sqlx::query(
            "INSERT INTO course (tenant_id, course_id, title) VALUES ($1, $2, $3) \
             ON CONFLICT (tenant_id, course_id) DO UPDATE SET \
             title = EXCLUDED.title, updated_at = transaction_timestamp()",
        )
        .bind(tenant.as_uuid())
        .bind(course_id.as_uuid())
        .bind(&course.title)
        .execute(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let affected = sqlx::query_scalar::<_, Uuid>(
            "SELECT DISTINCT assignment_id FROM assignment_policy_exception \
             WHERE tenant_id = $1 AND course_id = $2 AND course_group_id IS NOT NULL \
             ORDER BY assignment_id",
        )
        .bind(tenant.as_uuid())
        .bind(course_id.as_uuid())
        .fetch_all(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?
        .into_iter()
        .map(AssignmentId::from_uuid)
        .collect::<Vec<_>>();
        // The sorted query fixes the multi-assignment lock order. Each policy
        // advisory lock precedes its active attempt/timing row locks.
        let mut locked = Vec::with_capacity(affected.len());
        for assignment in affected {
            lock_postgres_assignment_policy(&mut transaction, tenant, assignment).await?;
            locked.push((
                assignment,
                lock_postgres_active_timing_rows(&mut transaction, tenant, assignment).await?,
            ));
        }
        for membership in &course.members {
            sqlx::query(
                "INSERT INTO course_member (tenant_id, course_id, user_id, role) \
                 VALUES ($1, $2, $3, $4) \
                 ON CONFLICT (tenant_id, course_id, user_id) DO UPDATE SET role = EXCLUDED.role",
            )
            .bind(tenant.as_uuid())
            .bind(course_id.as_uuid())
            .bind(membership.user.as_uuid())
            .bind(course_membership_role_name(membership.role))
            .execute(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        }
        let member_ids = course
            .members
            .iter()
            .map(|membership| membership.user.as_uuid())
            .collect::<Vec<_>>();
        sqlx::query(
            "DELETE FROM course_member WHERE tenant_id = $1 AND course_id = $2 \
             AND NOT (user_id = ANY($3::uuid[]))",
        )
        .bind(tenant.as_uuid())
        .bind(course_id.as_uuid())
        .bind(&member_ids)
        .execute(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        sqlx::query(
            "DELETE FROM course_group_member AS grouped USING course_member AS member \
             WHERE grouped.tenant_id = $1 AND grouped.course_id = $2 \
               AND member.tenant_id = grouped.tenant_id AND member.course_id = grouped.course_id \
               AND member.user_id = grouped.user_id AND member.role <> 'student'",
        )
        .bind(tenant.as_uuid())
        .bind(course_id.as_uuid())
        .execute(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let now = database_timestamp(&mut transaction).await?;
        for (assignment, rows) in locked {
            apply_postgres_locked_timing_rows(
                &mut transaction,
                tenant,
                assignment,
                None,
                now,
                rows,
            )
            .await?;
        }
        transaction.commit().await.map_err(map_sqlx_error)
    }

    async fn get_course(
        &self,
        context: TenantContext,
        course: CourseId,
    ) -> Result<Option<CourseRecord>, StoreError> {
        let mut transaction = self.begin_tenant(context).await?;
        let row = sqlx::query("SELECT title FROM course WHERE tenant_id = $1 AND course_id = $2")
            .bind(context.tenant_id().as_uuid())
            .bind(course.as_uuid())
            .fetch_optional(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        let Some(row) = row else {
            transaction.commit().await.map_err(map_sqlx_error)?;
            return Ok(None);
        };
        let member_rows = sqlx::query(
            "SELECT user_id, role FROM course_member \
             WHERE tenant_id = $1 AND course_id = $2 ORDER BY user_id",
        )
        .bind(context.tenant_id().as_uuid())
        .bind(course.as_uuid())
        .fetch_all(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let members = member_rows
            .iter()
            .map(|member| {
                let user = member.try_get("user_id").map_err(map_sqlx_error)?;
                let role: String = member.try_get("role").map_err(map_sqlx_error)?;
                Ok(CourseMembership {
                    user: UserId::from_uuid(user),
                    role: parse_course_membership_role(&role)?,
                })
            })
            .collect::<Result<Vec<_>, StoreError>>()?;
        let record = CourseRecord {
            id: course,
            tenant: context.tenant_id(),
            title: row.try_get("title").map_err(map_sqlx_error)?,
            members,
        };
        validate_course(&record).map_err(|error| {
            StoreError::Unavailable(format!("stored course is invalid: {error}"))
        })?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(Some(record))
    }

    async fn list_courses(
        &self,
        context: TenantContext,
        scope: CourseListScope,
        page: PageRequest,
    ) -> Result<Page<CourseSummary>, StoreError> {
        let cursor = page.after.as_ref().map(|value| value.as_str().to_string());
        let limit = i64::from(page.size.get()) + 1;
        let mut transaction = self.begin_tenant(context).await?;
        let rows = match scope {
            CourseListScope::Member(user) => sqlx::query(MEMBER_COURSE_PAGE_SQL)
                .bind(context.tenant_id().as_uuid())
                .bind(user.as_uuid())
                .bind(cursor)
                .bind(limit)
                .fetch_all(&mut *transaction)
                .await
                .map_err(map_sqlx_error)?,
            CourseListScope::TenantAdministrator => sqlx::query(
                "SELECT course_id::text AS stable_key, course_id, title, \
                        'administrator'::text AS role \
                 FROM course WHERE tenant_id = $1 \
                   AND ($2::text IS NULL OR course_id::text > $2) \
                 ORDER BY course_id::text LIMIT $3",
            )
            .bind(context.tenant_id().as_uuid())
            .bind(cursor)
            .bind(limit)
            .fetch_all(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?,
        };
        let mut records = rows
            .iter()
            .map(|row| {
                let key: String = row.try_get("stable_key").map_err(map_sqlx_error)?;
                let id = CourseId::from_uuid(row.try_get("course_id").map_err(map_sqlx_error)?);
                let title = row.try_get("title").map_err(map_sqlx_error)?;
                let role: String = row.try_get("role").map_err(map_sqlx_error)?;
                Ok((
                    key,
                    CourseSummary {
                        id,
                        tenant: context.tenant_id(),
                        title,
                        role: parse_course_role(&role)?,
                    },
                ))
            })
            .collect::<Result<Vec<_>, StoreError>>()?;
        let result = page_from_keyed_records(&mut records, page.size.get())?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(result)
    }

    async fn put_course_group(
        &self,
        context: TenantContext,
        command: PutCourseGroupCommand,
    ) -> Result<StoredCourseGroup, StoreError> {
        ensure_tenant(context, command.record.tenant)?;
        validate_course_group(&command.record)?;
        let tenant = context.tenant_id();
        let mut transaction = self.begin_tenant(context).await?;
        let authorized = postgres_is_course_instructor(
            &mut transaction,
            tenant,
            command.record.course,
            command.actor,
        )
        .await?;
        let accessible: bool =
            sqlx::query_scalar("SELECT public.ple_course_records_accessible($1, $2)")
                .bind(tenant.as_uuid())
                .bind(command.record.course.as_uuid())
                .fetch_one(&mut *transaction)
                .await
                .map_err(map_sqlx_error)?;
        if !authorized || !accessible {
            return Err(StoreError::NotFound);
        }
        let member_ids = command
            .record
            .members
            .iter()
            .map(UserId::as_uuid)
            .collect::<Vec<_>>();
        let valid_members: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM course_member WHERE tenant_id = $1 AND course_id = $2 \
             AND role = 'student' AND user_id = ANY($3::uuid[])",
        )
        .bind(tenant.as_uuid())
        .bind(command.record.course.as_uuid())
        .bind(&member_ids)
        .fetch_one(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        if valid_members
            != i64::try_from(member_ids.len()).map_err(|_| {
                StoreError::InvalidRecord("course group has too many members".to_string())
            })?
        {
            return Err(StoreError::NotFound);
        }

        let row = sqlx::query(
            "SELECT course_id, title, revision FROM course_group \
             WHERE tenant_id = $1 AND course_group_id = $2 FOR UPDATE",
        )
        .bind(tenant.as_uuid())
        .bind(command.record.id.as_uuid())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let existing = if let Some(row) = &row {
            let members =
                load_postgres_course_group_members(&mut transaction, tenant, command.record.id)
                    .await?;
            Some(StoredCourseGroup {
                record: CourseGroupRecord {
                    id: command.record.id,
                    tenant,
                    course: CourseId::from_uuid(row.try_get("course_id").map_err(map_sqlx_error)?),
                    title: row.try_get("title").map_err(map_sqlx_error)?,
                    members,
                },
                revision: CourseGroupRevision::from_stored(
                    row.try_get("revision").map_err(map_sqlx_error)?,
                )?,
            })
        } else {
            None
        };
        if let Some(existing) = &existing
            && existing.record == command.record
        {
            transaction.commit().await.map_err(map_sqlx_error)?;
            return Ok(existing.clone());
        }
        let revision = match &existing {
            Some(existing) if command.expected_revision == Some(existing.revision) => {
                existing.revision.next()?
            }
            Some(_) => return Err(StoreError::Conflict),
            None if command.expected_revision.is_none() => CourseGroupRevision::INITIAL,
            None => return Err(StoreError::Conflict),
        };
        if existing
            .as_ref()
            .is_some_and(|record| record.record.course != command.record.course)
        {
            return Err(StoreError::Conflict);
        }
        let affected = sqlx::query_scalar::<_, Uuid>(
            "SELECT assignment_id FROM assignment_policy_exception \
             WHERE tenant_id = $1 AND course_group_id = $2 ORDER BY assignment_id",
        )
        .bind(tenant.as_uuid())
        .bind(command.record.id.as_uuid())
        .fetch_all(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?
        .into_iter()
        .map(AssignmentId::from_uuid)
        .collect::<BTreeSet<_>>();
        // BTreeSet iteration gives every concurrent group edit the same
        // assignment lock order before any active attempt/timing row lock.
        let mut locked = Vec::with_capacity(affected.len());
        for assignment in &affected {
            lock_postgres_assignment_policy(&mut transaction, tenant, *assignment).await?;
            locked.push((
                *assignment,
                lock_postgres_active_timing_rows(&mut transaction, tenant, *assignment).await?,
            ));
        }
        let revision_i64 = i64::try_from(revision.value()).map_err(|_| StoreError::Conflict)?;
        if existing.is_some() {
            let updated = sqlx::query(
                "UPDATE course_group SET title = $3, revision = $4, \
                 updated_at = transaction_timestamp() \
                 WHERE tenant_id = $1 AND course_group_id = $2",
            )
            .bind(tenant.as_uuid())
            .bind(command.record.id.as_uuid())
            .bind(&command.record.title)
            .bind(revision_i64)
            .execute(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
            if updated.rows_affected() != 1 {
                return Err(StoreError::Conflict);
            }
        } else {
            sqlx::query(
                "INSERT INTO course_group \
                 (tenant_id, course_id, course_group_id, title, revision) \
                 VALUES ($1, $2, $3, $4, $5)",
            )
            .bind(tenant.as_uuid())
            .bind(command.record.course.as_uuid())
            .bind(command.record.id.as_uuid())
            .bind(&command.record.title)
            .bind(revision_i64)
            .execute(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        }
        sqlx::query(
            "DELETE FROM course_group_member WHERE tenant_id = $1 AND course_group_id = $2",
        )
        .bind(tenant.as_uuid())
        .bind(command.record.id.as_uuid())
        .execute(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        for user in &command.record.members {
            sqlx::query(
                "INSERT INTO course_group_member \
                 (tenant_id, course_id, course_group_id, user_id) VALUES ($1, $2, $3, $4)",
            )
            .bind(tenant.as_uuid())
            .bind(command.record.course.as_uuid())
            .bind(command.record.id.as_uuid())
            .bind(user.as_uuid())
            .execute(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        }
        let now = database_timestamp(&mut transaction).await?;
        for (assignment, rows) in locked {
            apply_postgres_locked_timing_rows(
                &mut transaction,
                tenant,
                assignment,
                None,
                now,
                rows,
            )
            .await?;
        }
        transaction.commit().await.map_err(map_sqlx_error)?;
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
        let tenant = context.tenant_id();
        let mut transaction = self.begin_tenant(context).await?;
        let row = sqlx::query(
            "SELECT course_id, title, revision FROM course_group \
             WHERE tenant_id = $1 AND course_group_id = $2",
        )
        .bind(tenant.as_uuid())
        .bind(group.as_uuid())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let result = if let Some(row) = row {
            Some(StoredCourseGroup {
                record: CourseGroupRecord {
                    id: group,
                    tenant,
                    course: CourseId::from_uuid(row.try_get("course_id").map_err(map_sqlx_error)?),
                    title: row.try_get("title").map_err(map_sqlx_error)?,
                    members: load_postgres_course_group_members(&mut transaction, tenant, group)
                        .await?,
                },
                revision: CourseGroupRevision::from_stored(
                    row.try_get("revision").map_err(map_sqlx_error)?,
                )?,
            })
        } else {
            None
        };
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(result)
    }

    async fn create_assignment(
        &self,
        context: TenantContext,
        assignment: AssignmentRecord,
    ) -> Result<StoredAssignment, StoreError> {
        ensure_tenant(context, assignment.tenant)?;
        validate_assignment(&assignment)?;
        let (completion_policy, completion_threshold) =
            completion_policy_columns(assignment.policies.completion);
        let (practice_policy, practice_limit) =
            continued_practice_columns(assignment.policies.continued_practice)?;
        let mut transaction = self.begin_tenant(context).await?;
        validate_postgres_assignment_references(&mut transaction, context, &assignment).await?;
        let inserted = sqlx::query(
            "INSERT INTO assignment \
             (tenant_id, assignment_id, course_id, title, completion_policy, \
              completion_threshold, attempt_selection_policy, continued_practice_policy, \
              practice_max_additional_runs, variation_policy, lifecycle, visible, \
              auto_submit, revision) \
             VALUES ($1, $2, $3, $4, $5, $6::numeric, $7, $8, $9, $10, \
                     'published', true, true, 1) \
             ON CONFLICT (tenant_id, assignment_id) DO NOTHING \
             RETURNING revision, scoring_generation, scoring_status",
        )
        .bind(assignment.tenant.as_uuid())
        .bind(assignment.id.as_uuid())
        .bind(assignment.course_id.as_uuid())
        .bind(&assignment.title)
        .bind(completion_policy)
        .bind(completion_threshold)
        .bind(grade_policy_name(assignment.policies.grade))
        .bind(practice_policy)
        .bind(practice_limit)
        .bind(variation_policy_name(assignment.policies.variation))
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let Some(row) = inserted else {
            return Err(StoreError::AlreadyExists);
        };
        insert_postgres_assignment_items(&mut transaction, &assignment).await?;
        let revision =
            AssignmentRevision::from_stored(row.try_get("revision").map_err(map_sqlx_error)?)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(StoredAssignment {
            record: assignment,
            revision,
            scoring_generation: decode_scoring_generation(&row)?,
            scoring_status: decode_scoring_status(&row)?,
        })
    }

    async fn replace_assignment(
        &self,
        context: TenantContext,
        course: CourseId,
        assignment: AssignmentId,
        expected_revision: AssignmentRevision,
        update: AssignmentUpdate,
    ) -> Result<StoredAssignment, StoreError> {
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
        let (completion_policy, completion_threshold) =
            completion_policy_columns(assignment.policies.completion);
        let (practice_policy, practice_limit) =
            continued_practice_columns(assignment.policies.continued_practice)?;
        let mut transaction = self.begin_tenant(context).await?;
        validate_postgres_assignment_references(&mut transaction, context, &assignment).await?;
        let previous = load_assignment(&mut transaction, assignment.tenant, assignment.id).await?;
        let scoring_changed = assignment_scoring_changed(&previous, &assignment);
        let has_scores: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM attempt_score_current \
             WHERE tenant_id = $1 AND assignment_id = $2)",
        )
        .bind(assignment.tenant.as_uuid())
        .bind(assignment.id.as_uuid())
        .fetch_one(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let row = sqlx::query(
            "UPDATE assignment SET title = $4, completion_policy = $5, \
                    completion_threshold = $6::numeric, attempt_selection_policy = $7, \
                    continued_practice_policy = $8, practice_max_additional_runs = $9, \
                    variation_policy = $10, \
                    scoring_generation = scoring_generation + CASE WHEN $11 THEN 1 ELSE 0 END, \
                    scoring_status = CASE WHEN $11 \
                        THEN CASE WHEN $12 THEN 'recalculating' ELSE 'current' END \
                        ELSE scoring_status END, \
                    revision = revision + 1, updated_at = transaction_timestamp() \
             WHERE tenant_id = $1 AND assignment_id = $2 AND course_id = $3 AND revision = $13 \
             RETURNING revision, scoring_generation, scoring_status",
        )
        .bind(assignment.tenant.as_uuid())
        .bind(assignment.id.as_uuid())
        .bind(assignment.course_id.as_uuid())
        .bind(&assignment.title)
        .bind(completion_policy)
        .bind(completion_threshold)
        .bind(grade_policy_name(assignment.policies.grade))
        .bind(practice_policy)
        .bind(practice_limit)
        .bind(variation_policy_name(assignment.policies.variation))
        .bind(scoring_changed)
        .bind(has_scores)
        .bind(i64::try_from(expected_revision.value()).map_err(|_| StoreError::Conflict)?)
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let Some(row) = row else {
            let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM assignment WHERE tenant_id = $1 AND assignment_id = $2 AND course_id = $3)",
            )
            .bind(assignment.tenant.as_uuid())
            .bind(assignment.id.as_uuid())
            .bind(assignment.course_id.as_uuid())
            .fetch_one(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
            return Err(if exists {
                StoreError::Conflict
            } else {
                StoreError::NotFound
            });
        };
        let scoring_generation = decode_scoring_generation(&row)?;
        let scoring_status = decode_scoring_status(&row)?;
        replace_postgres_assignment_items(&mut transaction, &assignment).await?;
        if scoring_status == ScoringStatus::Recalculating {
            let job = JobId::generate()?;
            let payload = serde_json::to_value(JobPayload::RecalculateAssignment {
                assignment: assignment.id,
                generation: scoring_generation,
            })
            .map_err(|error| {
                StoreError::InvalidRecord(format!(
                    "assignment scoring job serialization failed: {error}"
                ))
            })?;
            sqlx::query(
                "INSERT INTO worker_job (job_id, tenant_id, payload, state, max_attempts) \
                 VALUES ($1, $2, $3, 'ready', 10)",
            )
            .bind(job.as_uuid())
            .bind(assignment.tenant.as_uuid())
            .bind(payload)
            .execute(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        }
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(StoredAssignment {
            record: assignment,
            revision: AssignmentRevision::from_stored(
                row.try_get("revision").map_err(map_sqlx_error)?,
            )?,
            scoring_generation,
            scoring_status,
        })
    }

    async fn get_assignment_timing(
        &self,
        context: TenantContext,
        assignment: AssignmentId,
    ) -> Result<Option<StoredAssignmentTiming>, StoreError> {
        let mut transaction = self.begin_tenant(context).await?;
        let row = sqlx::query(
            "SELECT assignment_id, course_id, visible, \
                    floor(extract(epoch FROM available_at) * 1000)::bigint AS available_at_millis, \
                    floor(extract(epoch FROM due_at) * 1000)::bigint AS due_at_millis, \
                    floor(extract(epoch FROM closes_at) * 1000)::bigint AS closes_at_millis, \
                    late_submission_policy, time_limit_seconds, auto_submit, attempt_limit, revision \
             FROM assignment WHERE tenant_id = $1 AND assignment_id = $2",
        )
        .bind(context.tenant_id().as_uuid())
        .bind(assignment.as_uuid())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let result = row
            .as_ref()
            .map(|row| decode_stored_assignment_timing(row, context.tenant_id()))
            .transpose()?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(result)
    }

    async fn update_assignment_timing(
        &self,
        context: TenantContext,
        command: UpdateAssignmentTimingCommand,
    ) -> Result<StoredAssignmentTiming, StoreError> {
        validate_assignment_timing(command.policy)?;
        let tenant = context.tenant_id();
        let mut transaction = self.begin_tenant(context).await?;
        let row = sqlx::query(
            "SELECT assignment_id, course_id, visible, \
                    floor(extract(epoch FROM available_at) * 1000)::bigint AS available_at_millis, \
                    floor(extract(epoch FROM due_at) * 1000)::bigint AS due_at_millis, \
                    floor(extract(epoch FROM closes_at) * 1000)::bigint AS closes_at_millis, \
                    late_submission_policy, time_limit_seconds, auto_submit, attempt_limit, revision \
             FROM assignment WHERE tenant_id = $1 AND assignment_id = $2",
        )
        .bind(tenant.as_uuid())
        .bind(command.assignment.as_uuid())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?
        .ok_or(StoreError::NotFound)?;
        let current = decode_stored_assignment_timing(&row, tenant)?;
        if current.course != command.course
            || !postgres_is_course_instructor(
                &mut transaction,
                tenant,
                command.course,
                command.actor,
            )
            .await?
        {
            return Err(StoreError::NotFound);
        }
        if current.policy == command.policy {
            let locked = sqlx::query(
                "SELECT assignment_id, course_id, visible, \
                        floor(extract(epoch FROM available_at) * 1000)::bigint AS available_at_millis, \
                        floor(extract(epoch FROM due_at) * 1000)::bigint AS due_at_millis, \
                        floor(extract(epoch FROM closes_at) * 1000)::bigint AS closes_at_millis, \
                        late_submission_policy, time_limit_seconds, auto_submit, attempt_limit, revision \
                 FROM assignment WHERE tenant_id = $1 AND assignment_id = $2 FOR UPDATE",
            )
            .bind(tenant.as_uuid())
            .bind(command.assignment.as_uuid())
            .fetch_optional(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?
            .ok_or(StoreError::NotFound)?;
            let locked = decode_stored_assignment_timing(&locked, tenant)?;
            if locked.policy != command.policy {
                return Err(StoreError::Conflict);
            }
            transaction.commit().await.map_err(map_sqlx_error)?;
            return Ok(locked);
        }
        if current.revision != command.expected_revision {
            return Err(StoreError::Conflict);
        }
        lock_postgres_assignment_policy(&mut transaction, tenant, command.assignment).await?;
        let active_rows =
            lock_postgres_active_timing_rows(&mut transaction, tenant, command.assignment).await?;
        let locked =
            load_postgres_assignment_timing(&mut transaction, tenant, command.assignment, true)
                .await?
                .ok_or(StoreError::NotFound)?;
        if locked.policy == command.policy {
            transaction.commit().await.map_err(map_sqlx_error)?;
            return Ok(locked);
        }
        if locked.revision != command.expected_revision || locked.course != command.course {
            return Err(StoreError::Conflict);
        }
        let now = database_timestamp(&mut transaction).await?;
        apply_postgres_locked_timing_rows(
            &mut transaction,
            tenant,
            command.assignment,
            Some(command.policy),
            now,
            active_rows,
        )
        .await?;
        let revision = locked.revision.next()?;
        let updated = sqlx::query(
            "UPDATE assignment SET visible = $3, \
                    available_at = TIMESTAMPTZ 'epoch' + $4::bigint * INTERVAL '1 millisecond', \
                    due_at = TIMESTAMPTZ 'epoch' + $5::bigint * INTERVAL '1 millisecond', \
                    closes_at = TIMESTAMPTZ 'epoch' + $6::bigint * INTERVAL '1 millisecond', \
                    late_submission_policy = $7, time_limit_seconds = $8, \
                    auto_submit = true, attempt_limit = $9, revision = $10, \
                    updated_at = transaction_timestamp() \
             WHERE tenant_id = $1 AND assignment_id = $2 AND revision = $11",
        )
        .bind(tenant.as_uuid())
        .bind(command.assignment.as_uuid())
        .bind(command.policy.visible)
        .bind(
            command
                .policy
                .available_at
                .map(|value| value.as_unix_millis()),
        )
        .bind(command.policy.due_at.map(|value| value.as_unix_millis()))
        .bind(command.policy.closes_at.map(|value| value.as_unix_millis()))
        .bind(late_submission_policy_name(command.policy.late_submission))
        .bind(command.policy.time_limit_seconds.map(i64::from))
        .bind(command.policy.attempt_limit.map(i64::from))
        .bind(i64::try_from(revision.value()).map_err(|_| StoreError::Conflict)?)
        .bind(i64::try_from(locked.revision.value()).map_err(|_| StoreError::Conflict)?)
        .execute(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        if updated.rows_affected() != 1 {
            return Err(StoreError::Conflict);
        }
        transaction.commit().await.map_err(map_sqlx_error)?;
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
        let mut transaction = self.begin_tenant(context).await?;
        if let AssignmentPolicyExceptionTarget::CourseGroup(group) = command.exception.target {
            let course: Option<Uuid> = sqlx::query_scalar(
                "SELECT course_id FROM course_group WHERE tenant_id = $1 \
                 AND course_group_id = $2 FOR UPDATE",
            )
            .bind(tenant.as_uuid())
            .bind(group.as_uuid())
            .fetch_optional(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
            if course != Some(command.course.as_uuid()) {
                return Err(StoreError::NotFound);
            }
        }
        lock_postgres_assignment_policy(&mut transaction, tenant, command.assignment).await?;
        let current =
            load_postgres_assignment_timing(&mut transaction, tenant, command.assignment, false)
                .await?
                .ok_or(StoreError::NotFound)?;
        if current.course != command.course
            || !postgres_is_course_instructor(
                &mut transaction,
                tenant,
                command.course,
                command.actor,
            )
            .await?
        {
            return Err(StoreError::NotFound);
        }
        let accessible: bool =
            sqlx::query_scalar("SELECT public.ple_course_records_accessible($1, $2)")
                .bind(tenant.as_uuid())
                .bind(command.course.as_uuid())
                .fetch_one(&mut *transaction)
                .await
                .map_err(map_sqlx_error)?;
        if !accessible {
            return Err(StoreError::NotFound);
        }
        if let AssignmentPolicyExceptionTarget::Student(student) = command.exception.target {
            let exists: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM enrollment WHERE tenant_id = $1 \
                 AND assignment_id = $2 AND student_id = $3)",
            )
            .bind(tenant.as_uuid())
            .bind(command.assignment.as_uuid())
            .bind(student.as_uuid())
            .fetch_one(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
            if !exists {
                return Err(StoreError::NotFound);
            }
        }
        let rows = load_postgres_policy_exception_identity_rows(
            &mut transaction,
            tenant,
            command.assignment,
            command.exception.id,
            command.exception.target,
        )
        .await?;
        if rows.len() > 1 {
            return Err(StoreError::Conflict);
        }
        let existing = rows
            .first()
            .map(decode_postgres_policy_exception)
            .transpose()?;
        if let Some(existing) = &existing {
            if existing.id != command.exception.id || existing.target != command.exception.target {
                return Err(StoreError::Conflict);
            }
            if existing == &command.exception {
                transaction.commit().await.map_err(map_sqlx_error)?;
                return Ok(StoredAssignmentPolicyException {
                    exception: existing.clone(),
                    assignment_revision: current.revision,
                });
            }
        }
        if current.revision != command.expected_revision {
            return Err(StoreError::Conflict);
        }
        let active_rows =
            lock_postgres_active_timing_rows(&mut transaction, tenant, command.assignment).await?;
        let locked =
            load_postgres_assignment_timing(&mut transaction, tenant, command.assignment, true)
                .await?
                .ok_or(StoreError::NotFound)?;
        if locked.revision != command.expected_revision || locked.course != command.course {
            return Err(StoreError::Conflict);
        }
        let (available_mode, available_at) =
            postgres_exception_timestamp_columns(command.exception.available_at);
        let (closes_mode, closes_at) =
            postgres_exception_timestamp_columns(command.exception.closes_at);
        let (time_limit_mode, time_limit_seconds) =
            postgres_exception_limit_columns(command.exception.time_limit_seconds);
        let (attempt_limit_mode, attempt_limit) =
            postgres_exception_limit_columns(command.exception.attempt_limit);
        let (student_id, course_group_id) = match command.exception.target {
            AssignmentPolicyExceptionTarget::Student(student) => (Some(student.as_uuid()), None),
            AssignmentPolicyExceptionTarget::CourseGroup(group) => (None, Some(group.as_uuid())),
        };
        if existing.is_some() {
            let updated = sqlx::query(
                "UPDATE assignment_policy_exception SET available_mode = $3, \
                 available_at = TIMESTAMPTZ 'epoch' + $4::bigint * INTERVAL '1 millisecond', \
                 closes_mode = $5, \
                 closes_at = TIMESTAMPTZ 'epoch' + $6::bigint * INTERVAL '1 millisecond', \
                 time_limit_mode = $7, time_limit_seconds = $8, \
                 attempt_limit_mode = $9, attempt_limit = $10, revision = revision + 1, \
                 updated_at = transaction_timestamp() \
                 WHERE tenant_id = $1 AND assignment_policy_exception_id = $2",
            )
            .bind(tenant.as_uuid())
            .bind(command.exception.id.as_uuid())
            .bind(available_mode)
            .bind(available_at)
            .bind(closes_mode)
            .bind(closes_at)
            .bind(time_limit_mode)
            .bind(time_limit_seconds)
            .bind(attempt_limit_mode)
            .bind(attempt_limit)
            .execute(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
            if updated.rows_affected() != 1 {
                return Err(StoreError::Conflict);
            }
        } else {
            sqlx::query(
                "INSERT INTO assignment_policy_exception \
                 (tenant_id, assignment_policy_exception_id, course_id, assignment_id, \
                  student_id, course_group_id, available_mode, available_at, closes_mode, \
                  closes_at, time_limit_mode, time_limit_seconds, attempt_limit_mode, \
                  attempt_limit) VALUES ($1, $2, $3, $4, $5, $6, $7, \
                  TIMESTAMPTZ 'epoch' + $8::bigint * INTERVAL '1 millisecond', $9, \
                  TIMESTAMPTZ 'epoch' + $10::bigint * INTERVAL '1 millisecond', \
                  $11, $12, $13, $14)",
            )
            .bind(tenant.as_uuid())
            .bind(command.exception.id.as_uuid())
            .bind(command.course.as_uuid())
            .bind(command.assignment.as_uuid())
            .bind(student_id)
            .bind(course_group_id)
            .bind(available_mode)
            .bind(available_at)
            .bind(closes_mode)
            .bind(closes_at)
            .bind(time_limit_mode)
            .bind(time_limit_seconds)
            .bind(attempt_limit_mode)
            .bind(attempt_limit)
            .execute(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        }
        let revision = locked.revision.next()?;
        update_postgres_assignment_revision(
            &mut transaction,
            tenant,
            command.assignment,
            locked.revision,
            revision,
        )
        .await?;
        let now = database_timestamp(&mut transaction).await?;
        apply_postgres_locked_timing_rows(
            &mut transaction,
            tenant,
            command.assignment,
            None,
            now,
            active_rows,
        )
        .await?;
        transaction.commit().await.map_err(map_sqlx_error)?;
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
        let mut transaction = self.begin_tenant(context).await?;
        let initial_row = sqlx::query(
            "SELECT assignment_policy_exception_id, student_id, course_group_id, \
                    available_mode, \
                    floor(extract(epoch FROM available_at) * 1000)::bigint AS available_at_millis, \
                    closes_mode, \
                    floor(extract(epoch FROM closes_at) * 1000)::bigint AS closes_at_millis, \
                    time_limit_mode, time_limit_seconds, attempt_limit_mode, attempt_limit \
             FROM assignment_policy_exception WHERE tenant_id = $1 AND assignment_id = $2 \
               AND assignment_policy_exception_id = $3",
        )
        .bind(tenant.as_uuid())
        .bind(command.assignment.as_uuid())
        .bind(command.exception.as_uuid())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?
        .ok_or(StoreError::NotFound)?;
        let initial = decode_postgres_policy_exception(&initial_row)?;
        if let AssignmentPolicyExceptionTarget::CourseGroup(group) = initial.target {
            let course: Option<Uuid> = sqlx::query_scalar(
                "SELECT course_id FROM course_group WHERE tenant_id = $1 \
                 AND course_group_id = $2 FOR UPDATE",
            )
            .bind(tenant.as_uuid())
            .bind(group.as_uuid())
            .fetch_optional(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
            if course != Some(command.course.as_uuid()) {
                return Err(StoreError::NotFound);
            }
        }
        lock_postgres_assignment_policy(&mut transaction, tenant, command.assignment).await?;
        let current =
            load_postgres_assignment_timing(&mut transaction, tenant, command.assignment, false)
                .await?
                .ok_or(StoreError::NotFound)?;
        if current.course != command.course
            || !postgres_is_course_instructor(
                &mut transaction,
                tenant,
                command.course,
                command.actor,
            )
            .await?
        {
            return Err(StoreError::NotFound);
        }
        let accessible: bool =
            sqlx::query_scalar("SELECT public.ple_course_records_accessible($1, $2)")
                .bind(tenant.as_uuid())
                .bind(command.course.as_uuid())
                .fetch_one(&mut *transaction)
                .await
                .map_err(map_sqlx_error)?;
        if !accessible {
            return Err(StoreError::NotFound);
        }
        if current.revision != command.expected_revision {
            return Err(StoreError::Conflict);
        }
        let active_rows =
            lock_postgres_active_timing_rows(&mut transaction, tenant, command.assignment).await?;
        let locked =
            load_postgres_assignment_timing(&mut transaction, tenant, command.assignment, true)
                .await?
                .ok_or(StoreError::NotFound)?;
        if locked.revision != command.expected_revision || locked.course != command.course {
            return Err(StoreError::Conflict);
        }
        let deleted = sqlx::query(
            "DELETE FROM assignment_policy_exception WHERE tenant_id = $1 \
             AND assignment_id = $2 AND assignment_policy_exception_id = $3",
        )
        .bind(tenant.as_uuid())
        .bind(command.assignment.as_uuid())
        .bind(command.exception.as_uuid())
        .execute(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        if deleted.rows_affected() != 1 {
            return Err(StoreError::Conflict);
        }
        let revision = locked.revision.next()?;
        update_postgres_assignment_revision(
            &mut transaction,
            tenant,
            command.assignment,
            locked.revision,
            revision,
        )
        .await?;
        let now = database_timestamp(&mut transaction).await?;
        apply_postgres_locked_timing_rows(
            &mut transaction,
            tenant,
            command.assignment,
            None,
            now,
            active_rows,
        )
        .await?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(revision)
    }

    async fn get_assignment_policy_exception(
        &self,
        context: TenantContext,
        assignment: AssignmentId,
        exception: AssignmentPolicyExceptionId,
    ) -> Result<Option<StoredAssignmentPolicyException>, StoreError> {
        let tenant = context.tenant_id();
        let mut transaction = self.begin_tenant(context).await?;
        let row = sqlx::query(
            "SELECT assignment_policy_exception_id, student_id, course_group_id, \
                    available_mode, \
                    floor(extract(epoch FROM available_at) * 1000)::bigint AS available_at_millis, \
                    closes_mode, \
                    floor(extract(epoch FROM closes_at) * 1000)::bigint AS closes_at_millis, \
                    time_limit_mode, time_limit_seconds, attempt_limit_mode, attempt_limit \
             FROM assignment_policy_exception WHERE tenant_id = $1 AND assignment_id = $2 \
               AND assignment_policy_exception_id = $3",
        )
        .bind(tenant.as_uuid())
        .bind(assignment.as_uuid())
        .bind(exception.as_uuid())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let result = if let Some(row) = row {
            let timing =
                load_postgres_assignment_timing(&mut transaction, tenant, assignment, false)
                    .await?
                    .ok_or(StoreError::NotFound)?;
            Some(StoredAssignmentPolicyException {
                exception: decode_postgres_policy_exception(&row)?,
                assignment_revision: timing.revision,
            })
        } else {
            None
        };
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(result)
    }

    async fn resolve_assignment_timing(
        &self,
        context: TenantContext,
        assignment: AssignmentId,
        student: StudentId,
    ) -> Result<Option<ResolvedAssignmentTiming>, StoreError> {
        let tenant = context.tenant_id();
        let mut transaction = self.begin_tenant(context).await?;
        let enrollment =
            load_postgres_enrollment_by_student(&mut transaction, tenant, assignment, student)
                .await?;
        let Some(enrollment) = enrollment else {
            transaction.commit().await.map_err(map_sqlx_error)?;
            return Ok(None);
        };
        let timing = load_postgres_assignment_timing(&mut transaction, tenant, assignment, false)
            .await?
            .ok_or(StoreError::NotFound)?;
        let resolved = load_postgres_resolved_assignment_policy(
            &mut transaction,
            tenant,
            assignment,
            &enrollment,
            Some(timing.policy),
        )
        .await?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(Some(ResolvedAssignmentTiming {
            tenant,
            course: timing.course,
            assignment,
            student,
            policy: resolved.policy,
            contributors: resolved.contributors,
            revision: timing.revision,
        }))
    }

    async fn get_attempt_resolved_timing(
        &self,
        context: TenantContext,
        attempt: QuestionAttemptId,
    ) -> Result<Option<ResolvedAttemptTiming>, StoreError> {
        let mut transaction = self.begin_tenant(context).await?;
        let row = sqlx::query(
            "SELECT resolved_visible, \
                    floor(extract(epoch FROM resolved_available_at) * 1000)::bigint AS available_at_millis, \
                    floor(extract(epoch FROM resolved_due_at) * 1000)::bigint AS due_at_millis, \
                    floor(extract(epoch FROM resolved_closes_at) * 1000)::bigint AS closes_at_millis, \
                    resolved_late_submission_policy, resolved_time_limit_seconds, \
                    resolved_attempt_limit, resolution_sources \
             FROM attempt_timing_current WHERE tenant_id = $1 AND attempt_id = $2",
        )
        .bind(context.tenant_id().as_uuid())
        .bind(attempt.as_uuid())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let result = row
            .as_ref()
            .map(|row| decode_postgres_resolved_attempt_timing(row, attempt))
            .transpose()?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(result)
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
        let mut transaction = self.begin_tenant(context).await?;
        let row = sqlx::query(
            "SELECT assignment_id, course_id, title, completion_policy, \
                    completion_threshold::text AS completion_threshold, \
                    attempt_selection_policy, continued_practice_policy, \
                    practice_max_additional_runs, variation_policy, revision, \
                    scoring_generation, scoring_status \
             FROM assignment \
             WHERE tenant_id = $1 AND assignment_id = $2",
        )
        .bind(context.tenant_id().as_uuid())
        .bind(assignment.as_uuid())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let result = match row.as_ref() {
            Some(row) => Some(StoredAssignment {
                record: load_assignment_relations(
                    &mut transaction,
                    decode_assignment_header(row, context.tenant_id())?,
                )
                .await?,
                revision: AssignmentRevision::from_stored(
                    row.try_get("revision").map_err(map_sqlx_error)?,
                )?,
                scoring_generation: decode_scoring_generation(row)?,
                scoring_status: decode_scoring_status(row)?,
            }),
            None => None,
        };
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(result)
    }

    async fn get_assignment(
        &self,
        context: TenantContext,
        assignment: AssignmentId,
    ) -> Result<Option<AssignmentRecord>, StoreError> {
        let mut transaction = self.begin_tenant(context).await?;
        let row = sqlx::query(
            "SELECT assignment_id, course_id, title, completion_policy, \
                    completion_threshold::text AS completion_threshold, \
                    attempt_selection_policy, continued_practice_policy, \
                    practice_max_additional_runs, variation_policy \
             FROM assignment \
             WHERE tenant_id = $1 AND assignment_id = $2",
        )
        .bind(context.tenant_id().as_uuid())
        .bind(assignment.as_uuid())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let record = match row.as_ref() {
            Some(row) => Some(
                load_assignment_relations(
                    &mut transaction,
                    decode_assignment_header(row, context.tenant_id())?,
                )
                .await?,
            ),
            None => None,
        };
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(record)
    }

    async fn list_assignments(
        &self,
        context: TenantContext,
        course: CourseId,
        page: PageRequest,
    ) -> Result<Page<AssignmentRecord>, StoreError> {
        let cursor = page.after.as_ref().map(|value| value.as_str().to_string());
        let limit = i64::from(page.size.get()) + 1;
        let mut transaction = self.begin_tenant(context).await?;
        let course_exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM course WHERE tenant_id = $1 AND course_id = $2)",
        )
        .bind(context.tenant_id().as_uuid())
        .bind(course.as_uuid())
        .fetch_one(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        if !course_exists {
            return Err(StoreError::NotFound);
        }
        let rows = sqlx::query(
            "SELECT assignment_id::text AS stable_key, assignment_id, course_id, title, \
                    completion_policy, completion_threshold::text AS completion_threshold, \
                    attempt_selection_policy, continued_practice_policy, \
                    practice_max_additional_runs, variation_policy \
             FROM assignment \
             WHERE tenant_id = $1 AND course_id = $2 \
               AND ($3::text IS NULL OR assignment_id::text > $3) \
             ORDER BY assignment_id::text LIMIT $4",
        )
        .bind(context.tenant_id().as_uuid())
        .bind(course.as_uuid())
        .bind(cursor)
        .bind(limit)
        .fetch_all(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let mut records = Vec::with_capacity(rows.len());
        for row in &rows {
            let key: String = row.try_get("stable_key").map_err(map_sqlx_error)?;
            let header = decode_assignment_header(row, context.tenant_id())?;
            records.push((
                key,
                load_assignment_relations(&mut transaction, header).await?,
            ));
        }
        let result = page_from_keyed_records(&mut records, page.size.get())?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(result)
    }

    async fn create_enrollment(
        &self,
        context: TenantContext,
        enrollment: AssignmentEnrollment,
    ) -> Result<(), StoreError> {
        ensure_tenant(context, enrollment.tenant)?;
        let summary = StudentAssignmentSummary::empty(enrollment.tenant, enrollment.id);
        let (enrollment_payload, enrollment_checksum) = encode_payload(&enrollment)?;
        let (summary_payload, summary_checksum) = encode_payload(&summary)?;
        let mut transaction = self.begin_tenant(context).await?;
        let eligible_assignment: bool = sqlx::query_scalar(
            "SELECT EXISTS( \
                 SELECT 1 FROM assignment AS a \
                 JOIN course_member AS cm \
                   ON cm.tenant_id = a.tenant_id AND cm.course_id = a.course_id \
                 WHERE a.tenant_id = $1 AND a.assignment_id = $2 \
                   AND cm.user_id = $3 AND cm.role = 'student' \
             )",
        )
        .bind(enrollment.tenant.as_uuid())
        .bind(enrollment.assignment.as_uuid())
        .bind(enrollment.user.as_uuid())
        .fetch_one(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        if !eligible_assignment {
            return Err(StoreError::InvalidRecord(
                "enrollment user must be a student member of the assignment course".to_string(),
            ));
        }
        sqlx::query(
            "INSERT INTO enrollment \
             (tenant_id, enrollment_id, assignment_id, user_id, student_id, payload, payload_sha256) \
             VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(enrollment.tenant.as_uuid())
        .bind(enrollment.id.as_uuid())
        .bind(enrollment.assignment.as_uuid())
        .bind(enrollment.user.as_uuid())
        .bind(enrollment.student.as_uuid())
        .bind(enrollment_payload)
        .bind(enrollment_checksum)
        .execute(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        sqlx::query(
            "INSERT INTO student_assignment_summary \
             (tenant_id, enrollment_id, payload, payload_sha256) VALUES ($1, $2, $3, $4)",
        )
        .bind(enrollment.tenant.as_uuid())
        .bind(enrollment.id.as_uuid())
        .bind(summary_payload)
        .bind(summary_checksum)
        .execute(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        transaction.commit().await.map_err(map_sqlx_error)
    }

    async fn get_enrollment(
        &self,
        context: TenantContext,
        enrollment: EnrollmentId,
    ) -> Result<Option<AssignmentEnrollment>, StoreError> {
        let mut transaction = self.begin_tenant(context).await?;
        let row = sqlx::query(
            "SELECT payload, payload_sha256 FROM enrollment \
             WHERE tenant_id = $1 AND enrollment_id = $2",
        )
        .bind(context.tenant_id().as_uuid())
        .bind(enrollment.as_uuid())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let record = row.as_ref().map(decode_payload_row).transpose()?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(record)
    }

    async fn start_or_resume_run(
        &self,
        context: TenantContext,
        actor: UserId,
        assignment: AssignmentId,
        proposed_run: RunId,
    ) -> Result<AssignmentRun, StoreError> {
        let mut transaction = self.begin_tenant(context).await?;
        let run =
            start_or_resume_run(&mut transaction, context, actor, assignment, proposed_run).await?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(run)
    }

    async fn assignment_run_items(
        &self,
        context: TenantContext,
        run: RunId,
    ) -> Result<Vec<AssignmentRunItem>, StoreError> {
        let mut transaction = self.begin_tenant(context).await?;
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM assignment_run WHERE tenant_id = $1 AND run_id = $2)",
        )
        .bind(context.tenant_id().as_uuid())
        .bind(run.as_uuid())
        .fetch_one(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        if !exists {
            return Err(StoreError::NotFound);
        }
        let items = load_assignment_run_items(&mut transaction, context.tenant_id(), run).await?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(items)
    }

    async fn issue_or_resume_question_attempt(
        &self,
        context: TenantContext,
        command: IssueQuestionAttemptCommand,
    ) -> Result<QuestionAttempt, StoreError> {
        let mut transaction = self.begin_tenant(context).await?;
        let attempt = issue_or_resume_question_attempt(&mut transaction, context, command).await?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(attempt)
    }

    async fn reserve_or_resume_prefetched_question(
        &self,
        context: TenantContext,
        command: ReservePrefetchedQuestionCommand,
    ) -> Result<PrefetchedQuestion, StoreError> {
        let reservation = command.reservation;
        if reservation.tenant != context.tenant_id()
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
        let mut transaction = self.begin_tenant(context).await?;
        let run =
            load_run_for_update(&mut transaction, context.tenant_id(), reservation.run).await?;
        if run.completed_at.is_some() || run.score.is_some() {
            return Err(StoreError::Conflict);
        }
        let enrollment =
            load_enrollment_for_update(&mut transaction, context.tenant_id(), run.enrollment)
                .await?;
        if enrollment.user != command.actor {
            return Err(StoreError::Forbidden);
        }
        let predecessor = load_attempt_for_external_update(
            &mut transaction,
            context.tenant_id(),
            reservation.predecessor,
        )
        .await?;
        let submitted: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM submission_idempotency WHERE tenant_id = $1 AND attempt_id = $2)")
            .bind(context.tenant_id().as_uuid()).bind(reservation.predecessor.as_uuid())
            .fetch_one(&mut *transaction).await.map_err(map_sqlx_error)?;
        if predecessor.run != reservation.run || submitted {
            return Err(StoreError::Conflict);
        }
        let assignment =
            load_assignment(&mut transaction, context.tenant_id(), enrollment.assignment).await?;
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
        let target_already_attempted: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM question_attempt WHERE tenant_id = $1 AND run_id = $2 AND assignment_position = $3)",
        )
        .bind(context.tenant_id().as_uuid())
        .bind(reservation.run.as_uuid())
        .bind(i32::try_from(reservation.assignment_position).map_err(|_| StoreError::InvalidRecord("prefetch position is too large".to_string()))?)
        .fetch_one(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        if target_already_attempted {
            return Err(StoreError::Conflict);
        }
        let existing = sqlx::query("SELECT payload, payload_sha256 FROM question_prefetch WHERE tenant_id = $1 AND run_id = $2 AND predecessor_attempt_id = $3 AND assignment_position = $4 FOR UPDATE")
            .bind(context.tenant_id().as_uuid()).bind(reservation.run.as_uuid()).bind(reservation.predecessor.as_uuid()).bind(i32::try_from(reservation.assignment_position).map_err(|_| StoreError::InvalidRecord("prefetch position is too large".to_string()))?)
            .fetch_optional(&mut *transaction).await.map_err(map_sqlx_error)?;
        if let Some(row) = existing {
            let existing: PrefetchedQuestion = decode_payload_row(&row)?;
            transaction.commit().await.map_err(map_sqlx_error)?;
            return if existing == reservation {
                Ok(existing)
            } else {
                Err(StoreError::Conflict)
            };
        }
        let (payload, checksum) = encode_payload(&reservation)?;
        let inserted = sqlx::query("INSERT INTO question_prefetch (tenant_id, run_id, predecessor_attempt_id, predecessor_occurred_at, assignment_position, created_at, payload, payload_sha256) SELECT $1, $2, $3, qa.occurred_at, $4, transaction_timestamp(), $5, $6 FROM question_attempt qa WHERE qa.tenant_id = $1 AND qa.attempt_id = $3 AND qa.run_id = $2")
            .bind(context.tenant_id().as_uuid()).bind(reservation.run.as_uuid()).bind(reservation.predecessor.as_uuid()).bind(i32::try_from(reservation.assignment_position).map_err(|_| StoreError::InvalidRecord("prefetch position is too large".to_string()))?).bind(payload).bind(checksum)
            .execute(&mut *transaction).await.map_err(map_sqlx_error)?;
        if inserted.rows_affected() != 1 {
            return Err(StoreError::Conflict);
        }
        transaction.commit().await.map_err(map_sqlx_error)?;
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
        let mut transaction = self.begin_tenant(context).await?;
        let run_record = load_run_for_update(&mut transaction, context.tenant_id(), run).await?;
        let enrollment = load_enrollment_for_update(
            &mut transaction,
            context.tenant_id(),
            run_record.enrollment,
        )
        .await?;
        if enrollment.user != actor {
            return Err(StoreError::Forbidden);
        }
        let row = sqlx::query("SELECT payload, payload_sha256 FROM question_prefetch WHERE tenant_id = $1 AND run_id = $2 AND predecessor_attempt_id = $3 AND assignment_position = $4")
            .bind(context.tenant_id().as_uuid()).bind(run.as_uuid()).bind(predecessor.as_uuid()).bind(i32::try_from(assignment_position).map_err(|_| StoreError::InvalidRecord("prefetch position is too large".to_string()))?)
            .fetch_optional(&mut *transaction).await.map_err(map_sqlx_error)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        row.as_ref().map(decode_payload_row).transpose()
    }

    async fn submission_next_attempt(
        &self,
        context: TenantContext,
        actor: UserId,
        predecessor: QuestionAttemptId,
    ) -> Result<SubmissionNextAttempt, StoreError> {
        let mut transaction = self.begin_tenant(context).await?;
        require_attempt_owner(&mut transaction, context.tenant_id(), predecessor, actor).await?;
        let submitted: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM submission_idempotency WHERE tenant_id = $1 AND attempt_id = $2)")
            .bind(context.tenant_id().as_uuid()).bind(predecessor.as_uuid()).fetch_one(&mut *transaction).await.map_err(map_sqlx_error)?;
        if !submitted {
            return Err(StoreError::Conflict);
        }
        let next: Option<Option<Uuid>> = sqlx::query_scalar("SELECT next_attempt_id FROM submission_next_attempt WHERE tenant_id = $1 AND predecessor_attempt_id = $2")
            .bind(context.tenant_id().as_uuid()).bind(predecessor.as_uuid()).fetch_optional(&mut *transaction).await.map_err(map_sqlx_error)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(match next {
            None => SubmissionNextAttempt::Pending,
            Some(None) => SubmissionNextAttempt::None,
            Some(Some(id)) => SubmissionNextAttempt::Issued(QuestionAttemptId::from_uuid(id)),
        })
    }

    async fn pending_submission_for_run(
        &self,
        context: TenantContext,
        actor: UserId,
        run: RunId,
    ) -> Result<Option<QuestionAttemptId>, StoreError> {
        let mut transaction = self.begin_tenant(context).await?;
        let record = load_run_for_update(&mut transaction, context.tenant_id(), run).await?;
        if load_enrollment_for_update(&mut transaction, context.tenant_id(), record.enrollment)
            .await?
            .user
            != actor
        {
            return Err(StoreError::Forbidden);
        }
        let ids: Vec<Uuid> = sqlx::query_scalar("SELECT qa.attempt_id FROM question_attempt qa JOIN submission_idempotency si ON si.tenant_id = qa.tenant_id AND si.attempt_id = qa.attempt_id LEFT JOIN submission_next_attempt sna ON sna.tenant_id = qa.tenant_id AND sna.predecessor_attempt_id = qa.attempt_id WHERE qa.tenant_id = $1 AND qa.run_id = $2 AND sna.predecessor_attempt_id IS NULL ORDER BY qa.occurred_at DESC LIMIT 2")
            .bind(context.tenant_id().as_uuid()).bind(run.as_uuid()).fetch_all(&mut *transaction).await.map_err(map_sqlx_error)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        match ids.as_slice() {
            [] => Ok(None),
            [id] => Ok(Some(QuestionAttemptId::from_uuid(*id))),
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
        let mut transaction = self.begin_tenant(context).await?;
        require_attempt_owner(&mut transaction, context.tenant_id(), predecessor, actor).await?;
        let submitted: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM submission_idempotency WHERE tenant_id = $1 AND attempt_id = $2)")
            .bind(context.tenant_id().as_uuid()).bind(predecessor.as_uuid()).fetch_one(&mut *transaction).await.map_err(map_sqlx_error)?;
        if !submitted {
            return Err(StoreError::Conflict);
        }
        let existing: Option<Option<Uuid>> = sqlx::query_scalar("SELECT next_attempt_id FROM submission_next_attempt WHERE tenant_id = $1 AND predecessor_attempt_id = $2 FOR UPDATE")
            .bind(context.tenant_id().as_uuid()).bind(predecessor.as_uuid()).fetch_optional(&mut *transaction).await.map_err(map_sqlx_error)?;
        if let Some(existing) = existing {
            transaction.commit().await.map_err(map_sqlx_error)?;
            return if existing == next.map(|value| value.as_uuid()) {
                Ok(())
            } else {
                Err(StoreError::Conflict)
            };
        }
        match next {
            Some(next) => {
                let inserted = sqlx::query("INSERT INTO submission_next_attempt (tenant_id, predecessor_attempt_id, next_attempt_id, next_attempt_occurred_at) SELECT $1, $2, $3, next_attempt.occurred_at FROM question_attempt next_attempt JOIN question_attempt predecessor_attempt ON predecessor_attempt.tenant_id = next_attempt.tenant_id AND predecessor_attempt.run_id = next_attempt.run_id WHERE next_attempt.tenant_id = $1 AND next_attempt.attempt_id = $3 AND predecessor_attempt.attempt_id = $2")
                    .bind(context.tenant_id().as_uuid()).bind(predecessor.as_uuid()).bind(next.as_uuid()).execute(&mut *transaction).await.map_err(map_sqlx_error)?;
                if inserted.rows_affected() != 1 {
                    return Err(StoreError::Conflict);
                }
            }
            None => {
                sqlx::query("INSERT INTO submission_next_attempt (tenant_id, predecessor_attempt_id, next_attempt_id, next_attempt_occurred_at) VALUES ($1, $2, NULL, NULL)")
                .bind(context.tenant_id().as_uuid()).bind(predecessor.as_uuid()).execute(&mut *transaction).await.map_err(map_sqlx_error)?;
            }
        }
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(())
    }

    async fn list_question_attempts(
        &self,
        context: TenantContext,
        run: RunId,
        page: PageRequest,
    ) -> Result<Page<QuestionAttempt>, StoreError> {
        let cursor = page.after.as_ref().map(|value| value.as_str().to_string());
        let limit = i64::from(page.size.get()) + 1;
        let mut transaction = self.begin_tenant(context).await?;
        let run_exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM assignment_run \
             WHERE tenant_id = $1 AND run_id = $2)",
        )
        .bind(context.tenant_id().as_uuid())
        .bind(run.as_uuid())
        .fetch_one(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        if !run_exists {
            return Err(StoreError::NotFound);
        }
        let rows = sqlx::query(
            "SELECT lpad(qa.assignment_position::text, 10, '0') || '/' || \
                    lpad((extract(epoch FROM qa.occurred_at) * 1000)::bigint::text, 20, '0') \
                    || '/' || qa.attempt_id::text AS stable_key, \
                    COALESCE(si.payload, qa.payload) AS payload, \
                    COALESCE(si.payload_sha256, qa.payload_sha256) AS payload_sha256, \
                    qa.attempt_status AS current_attempt_status, \
                    floor(extract(epoch FROM qa.submitted_at) * 1000)::bigint \
                        AS current_submitted_at, \
                    floor(extract(epoch FROM timing.effective_deadline) * 1000)::bigint \
                        AS current_deadline_at \
             FROM question_attempt AS qa \
             LEFT JOIN submission_idempotency AS si \
               ON si.tenant_id = qa.tenant_id AND si.attempt_id = qa.attempt_id \
             LEFT JOIN attempt_timing_current AS timing \
               ON timing.tenant_id = qa.tenant_id AND timing.attempt_id = qa.attempt_id \
             WHERE qa.tenant_id = $1 AND qa.run_id = $2 \
               AND ($3::text IS NULL OR \
                    lpad(qa.assignment_position::text, 10, '0') || '/' || \
                    lpad((extract(epoch FROM qa.occurred_at) * 1000)::bigint::text, 20, '0') \
                    || '/' || qa.attempt_id::text > $3) \
             ORDER BY qa.assignment_position, qa.occurred_at, qa.attempt_id::text LIMIT $4",
        )
        .bind(context.tenant_id().as_uuid())
        .bind(run.as_uuid())
        .bind(cursor)
        .bind(limit)
        .fetch_all(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let result = page_from_rows_with(rows, page.size.get(), decode_current_attempt_row)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(result)
    }

    async fn replay_submission(
        &self,
        context: TenantContext,
        actor: UserId,
        attempt: QuestionAttemptId,
        response: &StudentResponse,
        idempotency_key: &SubmissionIdempotencyKey,
    ) -> Result<Option<SubmissionRecord>, StoreError> {
        let mut transaction = self.begin_tenant(context).await?;
        require_attempt_owner(&mut transaction, context.tenant_id(), attempt, actor).await?;
        let record = load_submission_replay(
            &mut transaction,
            context.tenant_id(),
            attempt,
            response,
            idempotency_key,
        )
        .await?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(record)
    }

    async fn submit_question_attempt(
        &self,
        context: TenantContext,
        command: SubmitQuestionAttemptCommand,
    ) -> Result<SubmissionRecord, StoreError> {
        let mut transaction = self.begin_tenant(context).await?;
        let record = submit_question_attempt(&mut transaction, context, command).await?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(record)
    }

    async fn force_submit_attempt(
        &self,
        context: TenantContext,
        command: ForceSubmitAttemptCommand,
    ) -> Result<AttemptSupportRecord, StoreError> {
        let mut transaction = self.begin_tenant(context).await?;
        let record = apply_postgres_attempt_support(
            &mut transaction,
            context,
            command.action,
            command.actor,
            command.attempt,
            AttemptSupportAction::ForceSubmit,
        )
        .await?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(record)
    }

    async fn clear_attempt(
        &self,
        context: TenantContext,
        command: ClearAttemptCommand,
    ) -> Result<AttemptSupportRecord, StoreError> {
        let mut transaction = self.begin_tenant(context).await?;
        let record = apply_postgres_attempt_support(
            &mut transaction,
            context,
            command.action,
            command.actor,
            command.attempt,
            AttemptSupportAction::Clear,
        )
        .await?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(record)
    }

    async fn release_attempt_feedback(
        &self,
        context: TenantContext,
        command: ReleaseAttemptFeedbackCommand,
    ) -> Result<FeedbackReleaseRecord, StoreError> {
        let tenant = context.tenant_id();
        let mut transaction = self.begin_tenant(context).await?;
        let attempt =
            load_attempt_for_external_update(&mut transaction, tenant, command.attempt).await?;
        let run = load_run_for_update(&mut transaction, tenant, attempt.run).await?;
        let enrollment =
            load_enrollment_for_update(&mut transaction, tenant, run.enrollment).await?;
        let assignment = load_assignment(&mut transaction, tenant, enrollment.assignment).await?;
        if !postgres_is_course_instructor(
            &mut transaction,
            tenant,
            assignment.course_id,
            command.actor,
        )
        .await?
        {
            return Err(StoreError::NotFound);
        }
        let has_feedback: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM attempt_feedback WHERE tenant_id = $1 AND attempt_id = $2)",
        )
        .bind(tenant.as_uuid())
        .bind(command.attempt.as_uuid())
        .fetch_one(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        if !has_feedback {
            return Err(StoreError::NotFound);
        }
        let question =
            load_published_record(&mut transaction, attempt.problem, attempt.question_version)
                .await?;
        if question.question.attempt_policy.feedback
            != question_model::run_policy::FeedbackDisclosure::OnRelease
        {
            return Err(StoreError::InvalidRecord(
                "feedback release requires an on-release question policy".to_string(),
            ));
        }
        let inserted = sqlx::query(
            "INSERT INTO feedback_release (tenant_id, attempt_id, released_by, released_at) \
             VALUES ($1, $2, $3, transaction_timestamp()) \
             ON CONFLICT (tenant_id, attempt_id) DO NOTHING \
             RETURNING released_by, floor(extract(epoch FROM released_at) * 1000)::bigint AS released_at",
        )
        .bind(tenant.as_uuid())
        .bind(command.attempt.as_uuid())
        .bind(command.actor.as_uuid())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let row = match inserted {
            Some(row) => row,
            None => sqlx::query(
                "SELECT released_by, floor(extract(epoch FROM released_at) * 1000)::bigint AS released_at \
                 FROM feedback_release WHERE tenant_id = $1 AND attempt_id = $2",
            )
            .bind(tenant.as_uuid())
            .bind(command.attempt.as_uuid())
            .fetch_one(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?,
        };
        let released_by = UserId::from_uuid(row.try_get("released_by").map_err(map_sqlx_error)?);
        if released_by != command.actor {
            return Err(StoreError::Conflict);
        }
        let released_at: i64 = row.try_get("released_at").map_err(map_sqlx_error)?;
        let record = FeedbackReleaseRecord {
            tenant,
            attempt: command.attempt,
            released_by,
            released_at: ActivityTimestamp::from_unix_millis(released_at),
        };
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(record)
    }

    async fn get_attempt_feedback_release(
        &self,
        context: TenantContext,
        actor: UserId,
        attempt_id: QuestionAttemptId,
    ) -> Result<Option<FeedbackReleaseRecord>, StoreError> {
        let tenant = context.tenant_id();
        let mut transaction = self.begin_tenant(context).await?;
        let attempt =
            load_attempt_for_external_update(&mut transaction, tenant, attempt_id).await?;
        let run = load_run_for_update(&mut transaction, tenant, attempt.run).await?;
        let enrollment =
            load_enrollment_for_update(&mut transaction, tenant, run.enrollment).await?;
        let assignment = load_assignment(&mut transaction, tenant, enrollment.assignment).await?;
        if actor != enrollment.user
            && !postgres_is_course_instructor(&mut transaction, tenant, assignment.course_id, actor)
                .await?
        {
            return Err(StoreError::NotFound);
        }
        let row = sqlx::query(
            "SELECT released_by, floor(extract(epoch FROM released_at) * 1000)::bigint AS released_at \
             FROM feedback_release WHERE tenant_id = $1 AND attempt_id = $2",
        )
        .bind(tenant.as_uuid())
        .bind(attempt_id.as_uuid())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let record = row
            .map(|row| {
                Ok::<FeedbackReleaseRecord, StoreError>(FeedbackReleaseRecord {
                    tenant,
                    attempt: attempt_id,
                    released_by: UserId::from_uuid(
                        row.try_get("released_by").map_err(map_sqlx_error)?,
                    ),
                    released_at: ActivityTimestamp::from_unix_millis(
                        row.try_get("released_at").map_err(map_sqlx_error)?,
                    ),
                })
            })
            .transpose()?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(record)
    }

    async fn get_run_summary_page(
        &self,
        context: TenantContext,
        actor: UserId,
        run_id: RunId,
        page: PageRequest,
    ) -> Result<RunSummaryPageInput, StoreError> {
        let tenant = context.tenant_id();
        let after = page
            .after
            .as_ref()
            .map(|cursor| RunSummaryCursor::decode(cursor, tenant.as_uuid(), run_id.as_uuid()))
            .transpose()?;
        let limit = i64::from(page.size.get()) + 1;
        let mut transaction = self.begin_tenant(context).await?;
        let run_row = sqlx::query(
            "SELECT payload, payload_sha256 FROM assignment_run \
             WHERE tenant_id = $1 AND run_id = $2",
        )
        .bind(tenant.as_uuid())
        .bind(run_id.as_uuid())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?
        .ok_or(StoreError::NotFound)?;
        let run: AssignmentRun = decode_payload_row(&run_row)?;
        let enrollment_row = sqlx::query(
            "SELECT payload, payload_sha256 FROM enrollment \
             WHERE tenant_id = $1 AND enrollment_id = $2",
        )
        .bind(tenant.as_uuid())
        .bind(run.enrollment.as_uuid())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?
        .ok_or(StoreError::NotFound)?;
        let enrollment: AssignmentEnrollment = decode_payload_row(&enrollment_row)?;
        let assignment = load_assignment(&mut transaction, tenant, enrollment.assignment).await?;
        if actor != enrollment.user
            && !postgres_is_course_instructor(&mut transaction, tenant, assignment.course_id, actor)
                .await?
        {
            return Err(StoreError::NotFound);
        }
        let summary_row = sqlx::query(
            "SELECT payload, payload_sha256 FROM student_assignment_summary \
             WHERE tenant_id = $1 AND enrollment_id = $2",
        )
        .bind(tenant.as_uuid())
        .bind(enrollment.id.as_uuid())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?
        .ok_or(StoreError::NotFound)?;
        let summary: StudentAssignmentSummary = decode_payload_row(&summary_row)?;

        // This is deliberately the sole bounded outcome query. It joins the
        // immutable question policy and private feedback/release rows so the
        // caller never performs one lookup per outcome.
        let rows = sqlx::query(
            "SELECT COALESCE(si.payload, qa.payload) AS attempt_payload, \
                    COALESCE(si.payload_sha256, qa.payload_sha256) AS attempt_sha256, \
                    qa.attempt_status AS current_attempt_status, \
                    floor(extract(epoch FROM qa.submitted_at) * 1000)::bigint \
                        AS current_submitted_at, \
                    floor(extract(epoch FROM timing.effective_deadline) * 1000)::bigint \
                        AS current_deadline_at, \
                    pvp.payload #>> '{question,attemptPolicy,feedback}' AS feedback_policy, \
                    af.hint, af.correct_response, af.rationale, af.content_sha256, \
                    fr.released_by, floor(extract(epoch FROM fr.released_at) * 1000)::bigint AS released_at \
             FROM question_attempt AS qa \
             LEFT JOIN submission_idempotency AS si \
               ON si.tenant_id = qa.tenant_id AND si.attempt_id = qa.attempt_id \
             LEFT JOIN attempt_timing_current AS timing \
               ON timing.tenant_id = qa.tenant_id AND timing.attempt_id = qa.attempt_id \
             JOIN assignment_run_item AS ri \
               ON ri.tenant_id = qa.tenant_id AND ri.run_id = qa.run_id \
              AND ri.issued_position = qa.assignment_position \
             LEFT JOIN assignment_item AS ai \
               ON ai.tenant_id = ri.tenant_id AND ai.assignment_id = $6 \
              AND ai.assignment_item_id = ri.assignment_item_id \
             LEFT JOIN assignment_selection_candidate AS sc \
               ON sc.tenant_id = ri.tenant_id AND sc.assignment_id = $6 \
              AND sc.candidate_id = ri.assignment_item_id \
             JOIN problem_version_payload AS pvp \
               ON pvp.problem_id = qa.problem_id AND pvp.version_id = qa.version_id \
             LEFT JOIN attempt_feedback AS af \
               ON af.tenant_id = qa.tenant_id AND af.attempt_id = qa.attempt_id \
             LEFT JOIN feedback_release AS fr \
               ON fr.tenant_id = qa.tenant_id AND fr.attempt_id = qa.attempt_id \
             WHERE qa.tenant_id = $1 AND qa.run_id = $2 \
               AND ($3::integer IS NULL OR (qa.assignment_position, qa.attempt_id) > ($3, $4::uuid)) \
               AND (NOT $7::boolean OR COALESCE(ai.delivery_state, sc.delivery_state) <> 'retired') \
               AND (NOT $7::boolean OR qa.attempt_status <> 'cleared') \
             ORDER BY qa.assignment_position, qa.attempt_id LIMIT $5",
        )
        .bind(tenant.as_uuid())
        .bind(run.id.as_uuid())
        .bind(after.map(|cursor| i32::try_from(cursor.assignment_position)).transpose().map_err(|_| StoreError::InvalidRecord("run summary cursor position is invalid".to_string()))?)
        .bind(after.map(|cursor| cursor.attempt))
        .bind(limit)
        .bind(assignment.id.as_uuid())
        .bind(actor == enrollment.user)
        .fetch_all(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let has_more = rows.len() > usize::from(page.size.get());
        let mut outcomes = Vec::with_capacity(rows.len().min(usize::from(page.size.get())));
        for row in rows.into_iter().take(usize::from(page.size.get())) {
            let attempt =
                decode_current_attempt_row_named(&row, "attempt_payload", "attempt_sha256")?;
            let feedback = feedback_from_summary_row(&row)?;
            let release = row
                .try_get::<Option<Uuid>, _>("released_by")
                .map_err(map_sqlx_error)?
                .zip(
                    row.try_get::<Option<i64>, _>("released_at")
                        .map_err(map_sqlx_error)?,
                )
                .map(|(released_by, released_at)| FeedbackReleaseRecord {
                    tenant,
                    attempt: attempt.id,
                    released_by: UserId::from_uuid(released_by),
                    released_at: ActivityTimestamp::from_unix_millis(released_at),
                });
            outcomes.push((
                RunSummaryCursor {
                    assignment_position: attempt.assignment_position,
                    attempt: attempt.id.as_uuid(),
                },
                RunSummaryOutcomeInput {
                    attempt: attempt.id,
                    assignment_position: attempt.assignment_position,
                    submitted_at: attempt.timer.submitted_at,
                    response: attempt.response,
                    result: attempt.result,
                    feedback_policy: feedback_policy_from_summary_row(&row)?,
                    feedback,
                    release,
                },
            ));
        }
        let next_cursor = has_more
            .then(|| {
                outcomes
                    .last()
                    .map(|(cursor, _)| cursor.encode(tenant.as_uuid(), run.id.as_uuid()))
            })
            .flatten();
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(RunSummaryPageInput {
            practice_allowed: continued_practice_allows_run(
                &summary,
                assignment.policies.continued_practice,
            ),
            run,
            assignment,
            summary,
            outcomes: Page {
                items: outcomes.into_iter().map(|(_, item)| item).collect(),
                next_cursor,
            },
        })
    }

    async fn apply_activity_transition(
        &self,
        context: TenantContext,
        transition: ActivityTransition,
    ) -> Result<StudentAssignmentSummary, StoreError> {
        let mut transaction = self.begin_tenant(context).await?;
        let next = match transition {
            ActivityTransition::StartRun { run } => {
                apply_start_run(&mut transaction, context, run).await?
            }
            ActivityTransition::RecordQuestionAttempt { attempt } => {
                apply_question_attempt(&mut transaction, context, *attempt).await?
            }
            ActivityTransition::CompleteRun { run, score, at } => {
                apply_complete_run(&mut transaction, context, run, score, at).await?
            }
        };
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(next)
    }

    async fn get_run(
        &self,
        context: TenantContext,
        run: RunId,
    ) -> Result<Option<AssignmentRun>, StoreError> {
        let mut transaction = self.begin_tenant(context).await?;
        let row = sqlx::query(
            "SELECT payload, payload_sha256 FROM assignment_run \
             WHERE tenant_id = $1 AND run_id = $2",
        )
        .bind(context.tenant_id().as_uuid())
        .bind(run.as_uuid())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let record = row.as_ref().map(decode_payload_row).transpose()?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(record)
    }

    async fn list_runs(
        &self,
        context: TenantContext,
        enrollment: EnrollmentId,
        page: PageRequest,
    ) -> Result<Page<AssignmentRun>, StoreError> {
        let cursor = page.after.as_ref().map(|value| value.as_str().to_string());
        let limit = i64::from(page.size.get()) + 1;
        let mut transaction = self.begin_tenant(context).await?;
        let enrollment_exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM enrollment \
             WHERE tenant_id = $1 AND enrollment_id = $2)",
        )
        .bind(context.tenant_id().as_uuid())
        .bind(enrollment.as_uuid())
        .fetch_one(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        if !enrollment_exists {
            return Err(StoreError::NotFound);
        }
        let rows = sqlx::query(
            "SELECT lpad(run_number::text, 10, '0') || '/' || run_id::text AS stable_key, \
                    payload, payload_sha256 \
             FROM assignment_run \
             WHERE tenant_id = $1 AND enrollment_id = $2 \
               AND ($3::text IS NULL \
                    OR lpad(run_number::text, 10, '0') || '/' || run_id::text > $3) \
             ORDER BY run_number, run_id::text LIMIT $4",
        )
        .bind(context.tenant_id().as_uuid())
        .bind(enrollment.as_uuid())
        .bind(cursor)
        .bind(limit)
        .fetch_all(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let result = page_from_rows(rows, page.size.get())?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(result)
    }

    async fn get_question_attempt(
        &self,
        context: TenantContext,
        attempt: QuestionAttemptId,
    ) -> Result<Option<QuestionAttempt>, StoreError> {
        let mut transaction = self.begin_tenant(context).await?;
        let row = sqlx::query(
            "SELECT COALESCE(si.payload, qa.payload) AS payload, \
                    COALESCE(si.payload_sha256, qa.payload_sha256) AS payload_sha256, \
                    qa.attempt_status AS current_attempt_status, \
                    floor(extract(epoch FROM qa.submitted_at) * 1000)::bigint \
                        AS current_submitted_at, \
                    floor(extract(epoch FROM timing.effective_deadline) * 1000)::bigint \
                        AS current_deadline_at \
             FROM question_attempt AS qa \
             LEFT JOIN submission_idempotency AS si \
               ON si.tenant_id = qa.tenant_id AND si.attempt_id = qa.attempt_id \
             LEFT JOIN attempt_timing_current AS timing \
               ON timing.tenant_id = qa.tenant_id AND timing.attempt_id = qa.attempt_id \
             WHERE qa.tenant_id = $1 AND qa.attempt_id = $2 \
             ORDER BY qa.occurred_at LIMIT 1",
        )
        .bind(context.tenant_id().as_uuid())
        .bind(attempt.as_uuid())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let record = row.as_ref().map(decode_current_attempt_row).transpose()?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(record)
    }

    async fn get_summary(
        &self,
        context: TenantContext,
        enrollment: EnrollmentId,
    ) -> Result<Option<StudentAssignmentSummary>, StoreError> {
        let mut transaction = self.begin_tenant(context).await?;
        let row = sqlx::query(
            "SELECT payload, payload_sha256 FROM student_assignment_summary \
             WHERE tenant_id = $1 AND enrollment_id = $2",
        )
        .bind(context.tenant_id().as_uuid())
        .bind(enrollment.as_uuid())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let record = row.as_ref().map(decode_payload_row).transpose()?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(record)
    }

    async fn list_gradebook_rows(
        &self,
        context: TenantContext,
        course: CourseId,
        page: PageRequest,
    ) -> Result<Page<question_model::GradebookSummaryRow>, StoreError> {
        let cursor = page
            .after
            .as_ref()
            .map(GradebookCursor::decode)
            .transpose()?;
        let limit = i64::from(page.size.get()) + 1;
        let mut transaction = self.begin_tenant(context).await?;
        let course_exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM course WHERE tenant_id = $1 AND course_id = $2)",
        )
        .bind(context.tenant_id().as_uuid())
        .bind(course.as_uuid())
        .fetch_one(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        if !course_exists {
            return Err(StoreError::NotFound);
        }
        let rows = sqlx::query(GRADEBOOK_SUMMARY_PAGE_SQL)
            .bind(context.tenant_id().as_uuid())
            .bind(course.as_uuid())
            .bind(cursor.map(|value| value.assignment))
            .bind(cursor.map(|value| value.enrollment))
            .bind(limit)
            .fetch_all(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        let mut records = rows
            .iter()
            .map(|row| {
                let assignment_id =
                    AssignmentId::from_uuid(row.try_get("assignment_id").map_err(map_sqlx_error)?);
                let enrollment_id =
                    EnrollmentId::from_uuid(row.try_get("enrollment_id").map_err(map_sqlx_error)?);
                let summary: StudentAssignmentSummary = decode_payload_row(row)?;
                Ok((
                    GradebookCursor {
                        assignment: assignment_id.as_uuid(),
                        enrollment: enrollment_id.as_uuid(),
                    },
                    question_model::GradebookSummaryRow {
                        tenant: context.tenant_id(),
                        course_id: course,
                        enrollment_id,
                        student_id: question_model::StudentId::from_uuid(
                            row.try_get("student_id").map_err(map_sqlx_error)?,
                        ),
                        assignment_id,
                        assignment_title: row
                            .try_get("assignment_title")
                            .map_err(map_sqlx_error)?,
                        summary,
                    },
                ))
            })
            .collect::<Result<Vec<_>, StoreError>>()?;
        let result = gradebook_page_from_records(&mut records, page.size.get());
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(result)
    }
}

#[cfg(feature = "postgres")]
#[async_trait]
impl ExternalToolBrokerStore for PostgresStore {
    async fn begin_or_resume_external_grade(
        &self,
        context: TenantContext,
        command: BeginExternalToolGradeCommand,
    ) -> Result<ExternalToolBegin, StoreError> {
        postgres_validate_external_command(&command)?;
        let tenant = context.tenant_id();
        let mut transaction = self.begin_tenant(context).await?;
        let base =
            load_attempt_for_external_update(&mut transaction, tenant, command.attempt).await?;
        require_attempt_owner(&mut transaction, tenant, base.id, command.actor).await?;
        let published =
            load_published_record(&mut transaction, base.problem, base.question_version).await?;
        postgres_validate_external_binding(&base, &published.question.source, &command.binding)?;
        if let Some(replay) = load_submission_replay(
            &mut transaction,
            tenant,
            base.id,
            &command.response,
            &command.idempotency_key,
        )
        .await?
        {
            transaction.commit().await.map_err(map_sqlx_error)?;
            return Ok(ExternalToolBegin::Committed(Box::new(replay)));
        }
        let row = sqlx::query(
            "SELECT actor_id, provider, problem_id, version_id, seed, source_object_id, source_sha256, integration_profile, response_sha256, idempotency_key, correlation, state, lease_token, EXTRACT(EPOCH FROM lease_expires_at) * 1000 AS lease_millis, result_payload, result_sha256 FROM external_tool_exchange WHERE tenant_id = $1 AND attempt_id = $2 FOR UPDATE",
        ).bind(tenant.as_uuid()).bind(command.attempt.as_uuid()).fetch_optional(&mut *transaction).await.map_err(map_sqlx_error)?;
        if let Some(row) = row {
            let stored = postgres_external_binding(&row)?;
            let actor: Uuid = row.try_get("actor_id").map_err(map_sqlx_error)?;
            let response_hash: Vec<u8> = row.try_get("response_sha256").map_err(map_sqlx_error)?;
            let key: String = row.try_get("idempotency_key").map_err(map_sqlx_error)?;
            if actor != command.actor.as_uuid()
                || !postgres_binding_matches(&stored, &command.binding)
                || response_hash.as_slice() != command.binding.response_sha256.as_bytes()
                || key != command.idempotency_key.as_str()
            {
                return Err(StoreError::Conflict);
            }
            let state: String = row.try_get("state").map_err(map_sqlx_error)?;
            if state == "committed" {
                return Err(StoreError::Conflict);
            }
            if state == "verified_pending" {
                let payload: Value = row.try_get("result_payload").map_err(map_sqlx_error)?;
                let result: AttemptResult = serde_json::from_value(payload).map_err(|error| {
                    StoreError::InvalidRecord(format!("external result decode failed: {error}"))
                })?;
                let raw: String = row.try_get("result_sha256").map_err(map_sqlx_error)?;
                let bytes = serde_json::to_vec(&result)
                    .map_err(|error| StoreError::InvalidRecord(error.to_string()))?;
                if raw != Sha256Digest::compute(&bytes).to_string() {
                    return Err(StoreError::Conflict);
                }
                transaction.commit().await.map_err(map_sqlx_error)?;
                return Ok(ExternalToolBegin::VerifiedPending(
                    ExternalToolVerifiedPending {
                        binding: stored,
                        correlation: crate::PersistedCorrelation::from_stored(
                            row.try_get("correlation").map_err(map_sqlx_error)?,
                        )?,
                        result,
                        result_sha256: Sha256Digest::compute(&bytes),
                    },
                ));
            }
            let token = ExternalToolLeaseToken::generate()?;
            let correlation = crate::PersistedCorrelation::from_stored(
                row.try_get("correlation").map_err(map_sqlx_error)?,
            )?;
            let changed = sqlx::query("UPDATE external_tool_exchange SET lease_token = $3, lease_expires_at = transaction_timestamp() + ($4::bigint * interval '1 millisecond'), updated_at = transaction_timestamp() WHERE tenant_id = $1 AND attempt_id = $2 AND state = 'verifying' AND lease_expires_at <= transaction_timestamp()")
                .bind(tenant.as_uuid()).bind(command.attempt.as_uuid()).bind(token.bytes().as_slice()).bind(i64::from(command.lease_millis)).execute(&mut *transaction).await.map_err(map_sqlx_error)?;
            if changed.rows_affected() == 0 {
                transaction.commit().await.map_err(map_sqlx_error)?;
                return Ok(ExternalToolBegin::InProgress);
            }
            let now = database_timestamp(&mut transaction).await?;
            let expires_at = ActivityTimestamp::from_unix_millis(
                now.as_unix_millis() + i64::from(command.lease_millis),
            );
            transaction.commit().await.map_err(map_sqlx_error)?;
            return Ok(ExternalToolBegin::Lease(ExternalToolLease {
                binding: command.binding,
                correlation,
                token,
                expires_at,
            }));
        }
        let token = ExternalToolLeaseToken::generate()?;
        sqlx::query("INSERT INTO external_tool_exchange (tenant_id, attempt_id, actor_id, provider, problem_id, version_id, seed, source_object_id, source_sha256, integration_profile, response_sha256, idempotency_key, correlation, state, lease_token, lease_expires_at) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,'verifying',$14, transaction_timestamp() + ($15::bigint * interval '1 millisecond'))")
            .bind(tenant.as_uuid()).bind(command.attempt.as_uuid()).bind(command.actor.as_uuid()).bind(&command.binding.provider).bind(command.binding.problem.as_uuid()).bind(command.binding.version.as_uuid()).bind(command.binding.seed as i64).bind(command.binding.source_object.as_uuid()).bind(&command.binding.source_sha256).bind(&command.binding.integration_profile).bind(command.binding.response_sha256.as_bytes().as_slice()).bind(command.idempotency_key.as_str()).bind(command.proposed_correlation.bytes()).bind(token.bytes().as_slice()).bind(i64::from(command.lease_millis)).execute(&mut *transaction).await.map_err(map_sqlx_error)?;
        let now = database_timestamp(&mut transaction).await?;
        let expires_at = ActivityTimestamp::from_unix_millis(
            now.as_unix_millis() + i64::from(command.lease_millis),
        );
        transaction.commit().await.map_err(map_sqlx_error)?;
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
        postgres_validate_external_response(&command.response, &command.binding)?;
        crate::validate_attempt_result(command.result)?;
        let tenant = context.tenant_id();
        let mut transaction = self.begin_tenant(context).await?;
        let base =
            load_attempt_for_external_update(&mut transaction, tenant, command.attempt).await?;
        require_attempt_owner(&mut transaction, tenant, base.id, command.actor).await?;
        let published =
            load_published_record(&mut transaction, base.problem, base.question_version).await?;
        postgres_validate_external_binding(&base, &published.question.source, &command.binding)?;
        let payload = serde_json::to_value(command.result)
            .map_err(|error| StoreError::InvalidRecord(error.to_string()))?;
        let raw = serde_json::to_vec(&command.result)
            .map_err(|error| StoreError::InvalidRecord(error.to_string()))?;
        let changed = sqlx::query("UPDATE external_tool_exchange SET state = 'verified_pending', lease_token = NULL, lease_expires_at = NULL, verification_token_sha256 = $17, result_payload = $9, result_sha256 = $10, updated_at = transaction_timestamp() WHERE tenant_id = $1 AND attempt_id = $2 AND actor_id = $3 AND provider = $4 AND problem_id = $5 AND version_id = $6 AND seed = $7 AND source_object_id = $8 AND source_sha256 = $11 AND integration_profile = $12 AND response_sha256 = $13 AND idempotency_key = $14 AND correlation = $15 AND state = 'verifying' AND lease_token = $16 AND lease_expires_at > transaction_timestamp()")
            .bind(tenant.as_uuid()).bind(command.attempt.as_uuid()).bind(command.actor.as_uuid()).bind(&command.binding.provider).bind(command.binding.problem.as_uuid()).bind(command.binding.version.as_uuid()).bind(command.binding.seed as i64).bind(command.binding.source_object.as_uuid()).bind(payload).bind(Sha256Digest::compute(&raw).to_string()).bind(&command.binding.source_sha256).bind(&command.binding.integration_profile).bind(command.binding.response_sha256.as_bytes().as_slice()).bind(command.idempotency_key.as_str()).bind(command.correlation.bytes()).bind(command.lease_token.bytes().as_slice()).bind(command.lease_token.hash().as_bytes().as_slice()).execute(&mut *transaction).await.map_err(map_sqlx_error)?;
        if changed.rows_affected() != 1 {
            return Err(StoreError::Conflict);
        }
        transaction.commit().await.map_err(map_sqlx_error)
    }

    async fn commit_external_tool_submission(
        &self,
        context: TenantContext,
        command: CommitExternalToolSubmissionCommand,
    ) -> Result<SubmissionRecord, StoreError> {
        postgres_validate_external_response(&command.response, &command.binding)?;
        let tenant = context.tenant_id();
        let mut transaction = self.begin_tenant(context).await?;
        let base =
            load_attempt_for_external_update(&mut transaction, tenant, command.attempt).await?;
        require_attempt_owner(&mut transaction, tenant, base.id, command.actor).await?;
        let published =
            load_published_record(&mut transaction, base.problem, base.question_version).await?;
        postgres_validate_external_binding(&base, &published.question.source, &command.binding)?;
        if let Some(replay) = load_submission_replay(
            &mut transaction,
            tenant,
            base.id,
            &command.response,
            &command.idempotency_key,
        )
        .await?
        {
            transaction.commit().await.map_err(map_sqlx_error)?;
            return Ok(replay);
        }
        validate_and_lock_external_launch(
            &mut transaction,
            tenant,
            command.actor,
            command.attempt,
            &command.binding,
            &command.launch_proof,
        )
        .await?;
        let row = sqlx::query("SELECT result_payload, result_sha256, verification_token_sha256 FROM external_tool_exchange WHERE tenant_id = $1 AND attempt_id = $2 AND actor_id = $3 AND provider = $4 AND problem_id = $5 AND version_id = $6 AND seed = $7 AND source_object_id = $8 AND source_sha256 = $9 AND integration_profile = $10 AND response_sha256 = $11 AND idempotency_key = $12 AND correlation = $13 AND state = 'verified_pending' FOR UPDATE")
            .bind(tenant.as_uuid()).bind(command.attempt.as_uuid()).bind(command.actor.as_uuid()).bind(&command.binding.provider).bind(command.binding.problem.as_uuid()).bind(command.binding.version.as_uuid()).bind(command.binding.seed as i64).bind(command.binding.source_object.as_uuid()).bind(&command.binding.source_sha256).bind(&command.binding.integration_profile).bind(command.binding.response_sha256.as_bytes().as_slice()).bind(command.idempotency_key.as_str()).bind(command.correlation.bytes()).fetch_optional(&mut *transaction).await.map_err(map_sqlx_error)?.ok_or(StoreError::Conflict)?;
        let payload: Value = row.try_get("result_payload").map_err(map_sqlx_error)?;
        let result: AttemptResult = serde_json::from_value(payload).map_err(|error| {
            StoreError::InvalidRecord(format!("external result decode failed: {error}"))
        })?;
        let expected_hash: Vec<u8> = row
            .try_get("verification_token_sha256")
            .map_err(map_sqlx_error)?;
        if expected_hash.as_slice() != command.lease_token.hash().as_bytes() {
            return Err(StoreError::Conflict);
        }
        let stored_result_hash: String = row.try_get("result_sha256").map_err(map_sqlx_error)?;
        let encoded_result = serde_json::to_vec(&result)
            .map_err(|error| StoreError::InvalidRecord(error.to_string()))?;
        if stored_result_hash != Sha256Digest::compute(&encoded_result).to_string() {
            return Err(StoreError::Conflict);
        }
        let record = submit_question_attempt(
            &mut transaction,
            context,
            SubmitQuestionAttemptCommand {
                actor: command.actor,
                attempt: command.attempt,
                response: command.response,
                result,
                feedback: FeedbackContent::default(),
                idempotency_key: command.idempotency_key,
            },
        )
        .await?;
        sqlx::query("UPDATE external_tool_exchange SET state = 'committed', verification_token_sha256 = NULL, updated_at = transaction_timestamp() WHERE tenant_id = $1 AND attempt_id = $2").bind(tenant.as_uuid()).bind(command.attempt.as_uuid()).execute(&mut *transaction).await.map_err(map_sqlx_error)?;
        revoke_locked_external_launch(&mut transaction, tenant, command.launch_proof.session_id)
            .await?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(record)
    }

    async fn commit_verified_external_tool_submission(
        &self,
        context: TenantContext,
        command: CommitVerifiedExternalToolSubmissionCommand,
    ) -> Result<SubmissionRecord, StoreError> {
        postgres_validate_external_response(&command.response, &command.binding)?;
        let tenant = context.tenant_id();
        let mut transaction = self.begin_tenant(context).await?;
        let base =
            load_attempt_for_external_update(&mut transaction, tenant, command.attempt).await?;
        require_attempt_owner(&mut transaction, tenant, base.id, command.actor).await?;
        let published =
            load_published_record(&mut transaction, base.problem, base.question_version).await?;
        postgres_validate_external_binding(&base, &published.question.source, &command.binding)?;
        if let Some(replay) = load_submission_replay(
            &mut transaction,
            tenant,
            base.id,
            &command.response,
            &command.idempotency_key,
        )
        .await?
        {
            transaction.commit().await.map_err(map_sqlx_error)?;
            return Ok(replay);
        }
        validate_and_lock_external_launch(
            &mut transaction,
            tenant,
            command.actor,
            command.attempt,
            &command.binding,
            &command.launch_proof,
        )
        .await?;
        let row = sqlx::query("SELECT result_payload, result_sha256 FROM external_tool_exchange WHERE tenant_id = $1 AND attempt_id = $2 AND actor_id = $3 AND provider = $4 AND problem_id = $5 AND version_id = $6 AND seed = $7 AND source_object_id = $8 AND source_sha256 = $9 AND integration_profile = $10 AND response_sha256 = $11 AND idempotency_key = $12 AND correlation = $13 AND state = 'verified_pending' FOR UPDATE")
            .bind(tenant.as_uuid()).bind(command.attempt.as_uuid()).bind(command.actor.as_uuid()).bind(&command.binding.provider).bind(command.binding.problem.as_uuid()).bind(command.binding.version.as_uuid()).bind(command.binding.seed as i64).bind(command.binding.source_object.as_uuid()).bind(&command.binding.source_sha256).bind(&command.binding.integration_profile).bind(command.binding.response_sha256.as_bytes().as_slice()).bind(command.idempotency_key.as_str()).bind(command.correlation.bytes()).fetch_optional(&mut *transaction).await.map_err(map_sqlx_error)?.ok_or(StoreError::Conflict)?;
        let payload: Value = row.try_get("result_payload").map_err(map_sqlx_error)?;
        let result: AttemptResult = serde_json::from_value(payload).map_err(|error| {
            StoreError::InvalidRecord(format!("external result decode failed: {error}"))
        })?;
        let stored_result_hash: String = row.try_get("result_sha256").map_err(map_sqlx_error)?;
        let encoded_result = serde_json::to_vec(&result)
            .map_err(|error| StoreError::InvalidRecord(error.to_string()))?;
        if stored_result_hash != Sha256Digest::compute(&encoded_result).to_string() {
            return Err(StoreError::Conflict);
        }
        let record = submit_question_attempt(
            &mut transaction,
            context,
            SubmitQuestionAttemptCommand {
                actor: command.actor,
                attempt: command.attempt,
                response: command.response,
                result,
                feedback: FeedbackContent::default(),
                idempotency_key: command.idempotency_key,
            },
        )
        .await?;
        sqlx::query("UPDATE external_tool_exchange SET state = 'committed', verification_token_sha256 = NULL, updated_at = transaction_timestamp() WHERE tenant_id = $1 AND attempt_id = $2")
            .bind(tenant.as_uuid()).bind(command.attempt.as_uuid()).execute(&mut *transaction).await.map_err(map_sqlx_error)?;
        revoke_locked_external_launch(&mut transaction, tenant, command.launch_proof.session_id)
            .await?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(record)
    }
}

#[cfg(feature = "postgres")]
#[async_trait]
impl crate::AuthoritativeTimeStore for PostgresStore {
    async fn authoritative_time(
        &self,
        context: TenantContext,
    ) -> Result<ActivityTimestamp, StoreError> {
        let mut transaction = self.begin_tenant(context).await?;
        let now = database_timestamp(&mut transaction).await?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(now)
    }
}

#[cfg(feature = "postgres")]
#[async_trait]
impl CourseRecordsAccessStore for PostgresStore {
    async fn course_records_accessible(
        &self,
        context: TenantContext,
        course: CourseId,
    ) -> Result<bool, StoreError> {
        let mut transaction = self.begin_tenant(context).await?;
        // The function checks the tenant setting before consulting any course
        // or retention row, so a foreign tenant cannot become an existence
        // oracle through this no-backend precheck.
        let accessible: bool =
            sqlx::query_scalar("SELECT public.ple_course_records_accessible($1, $2)")
                .bind(context.tenant_id().as_uuid())
                .bind(course.as_uuid())
                .fetch_one(&mut *transaction)
                .await
                .map_err(map_sqlx_error)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(accessible)
    }
}

#[cfg(feature = "postgres")]
#[async_trait]
impl ExternalToolLaunchSessionStore for PostgresStore {
    async fn create_external_tool_launch_session(
        &self,
        context: TenantContext,
        command: CreateExternalToolLaunchSessionCommand,
    ) -> Result<CreatedExternalToolLaunchSession, StoreError> {
        postgres_validate_external_response(&StudentResponse::ExternalTool {}, &command.binding)?;
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
        let tenant = context.tenant_id();
        let mut transaction = self.begin_tenant(context).await?;
        let attempt =
            load_attempt_for_external_update(&mut transaction, tenant, command.attempt).await?;
        require_attempt_owner(&mut transaction, tenant, attempt.id, command.actor).await?;
        let published =
            load_published_record(&mut transaction, attempt.problem, attempt.question_version)
                .await?;
        postgres_validate_external_binding(&attempt, &published.question.source, &command.binding)?;
        let id = fresh_external_tool_launch_id()?;
        let token = ExternalToolLaunchToken::generate()?;
        let row = sqlx::query("INSERT INTO external_tool_launch_session (launch_session_id, tenant_id, attempt_id, actor_id, provider, problem_id, version_id, seed, source_object_id, source_sha256, integration_profile, response_sha256, token_sha256, encrypted_provider_state, expires_at) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,transaction_timestamp() + ($15::bigint * interval '1 millisecond')) RETURNING EXTRACT(EPOCH FROM expires_at) * 1000 AS expires_millis")
            .bind(id).bind(tenant.as_uuid()).bind(command.attempt.as_uuid()).bind(command.actor.as_uuid()).bind(&command.binding.provider).bind(command.binding.problem.as_uuid()).bind(command.binding.version.as_uuid()).bind(command.binding.seed as i64).bind(command.binding.source_object.as_uuid()).bind(&command.binding.source_sha256).bind(&command.binding.integration_profile).bind(command.binding.response_sha256.as_bytes().as_slice()).bind(token.hash().as_bytes().as_slice()).bind(command.encrypted_provider_state).bind(i64::from(command.lifetime_millis)).fetch_one(&mut *transaction).await.map_err(map_sqlx_error)?;
        let expires: f64 = row.try_get("expires_millis").map_err(map_sqlx_error)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(CreatedExternalToolLaunchSession {
            id,
            token,
            expires_at: ActivityTimestamp::from_unix_millis(expires as i64),
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
        let tenant = context.tenant_id();
        let mut transaction = self.begin_tenant(context).await?;
        let base = load_attempt_for_external_update(&mut transaction, tenant, attempt).await?;
        require_attempt_owner(&mut transaction, tenant, base.id, actor).await?;
        let row = sqlx::query("SELECT provider, problem_id, version_id, seed, source_object_id, source_sha256, integration_profile, response_sha256, token_sha256, encrypted_provider_state FROM external_tool_launch_session WHERE tenant_id = $1 AND launch_session_id = $2 AND attempt_id = $3 AND actor_id = $4 AND revoked_at IS NULL AND expires_at > transaction_timestamp()")
            .bind(tenant.as_uuid()).bind(id).bind(attempt.as_uuid()).bind(actor.as_uuid()).fetch_optional(&mut *transaction).await.map_err(map_sqlx_error)?;
        let Some(row) = row else {
            transaction.commit().await.map_err(map_sqlx_error)?;
            return Ok(None);
        };
        let hash: Vec<u8> = row.try_get("token_sha256").map_err(map_sqlx_error)?;
        if hash.as_slice() != token.hash().as_bytes() {
            transaction.commit().await.map_err(map_sqlx_error)?;
            return Ok(None);
        }
        // Exact binding is re-derived from the attempt and caller's prevalidated source;
        // the session is not itself a catalog authority. Provider state is opaque.
        let binding = postgres_external_binding(&row)?;
        let published =
            load_published_record(&mut transaction, base.problem, base.question_version).await?;
        postgres_validate_external_binding(&base, &published.question.source, &binding)?;
        let encrypted_provider_state = row
            .try_get("encrypted_provider_state")
            .map_err(map_sqlx_error)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(Some(ResolvedExternalToolLaunchSession {
            binding,
            encrypted_provider_state,
        }))
    }
    async fn revoke_external_tool_launch_session(
        &self,
        context: TenantContext,
        actor: UserId,
        attempt: QuestionAttemptId,
        id: Uuid,
    ) -> Result<(), StoreError> {
        let tenant = context.tenant_id();
        let mut transaction = self.begin_tenant(context).await?;
        let changed = sqlx::query("UPDATE external_tool_launch_session SET revoked_at = transaction_timestamp() WHERE tenant_id = $1 AND launch_session_id = $2 AND attempt_id = $3 AND actor_id = $4 AND revoked_at IS NULL").bind(tenant.as_uuid()).bind(id).bind(attempt.as_uuid()).bind(actor.as_uuid()).execute(&mut *transaction).await.map_err(map_sqlx_error)?;
        if changed.rows_affected() != 1 {
            return Err(StoreError::NotFound);
        }
        transaction.commit().await.map_err(map_sqlx_error)
    }
}

#[cfg(feature = "postgres")]
#[async_trait]
impl CatalogStore for PostgresStore {
    async fn publish_draft(
        &self,
        context: TenantContext,
        actor: UserId,
        command: PublishDraftCommand,
    ) -> Result<PublishedProblemRecord, StoreError> {
        ensure_tenant(context, command.expected_draft.tenant)?;
        validate_draft(&command.expected_draft)?;
        validate_publication_source(&command.expected_draft, &command.published_source)?;
        validate_source_artifact(
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

        let mut transaction = self.begin_tenant(context).await?;
        if command.publisher != actor {
            return Err(StoreError::Forbidden);
        }
        let workspace_role: Option<String> = sqlx::query_scalar(
            "SELECT role FROM workspace_draft_access \
             WHERE tenant_id = $1 AND workspace_id = $2 AND user_id = $3",
        )
        .bind(context.tenant_id().as_uuid())
        .bind(command.expected_draft.question.workspace.as_uuid())
        .bind(actor.as_uuid())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        if workspace_role.as_deref() != Some("owner") {
            return Err(StoreError::Forbidden);
        }
        let draft_row = sqlx::query(
            "SELECT payload, payload_sha256, revision FROM workspace_draft \
             WHERE tenant_id = $1 AND workspace_id = $2 FOR UPDATE",
        )
        .bind(context.tenant_id().as_uuid())
        .bind(command.expected_draft.question.workspace.as_uuid())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?
        .ok_or(StoreError::NotFound)?;
        let stored_draft: DraftRecord = decode_payload_row(&draft_row)?;
        let stored_revision = WorkspaceDraftRevision::from_stored(
            draft_row.try_get("revision").map_err(map_sqlx_error)?,
        )?;
        if stored_draft != command.expected_draft || stored_revision != command.expected_revision {
            return Err(StoreError::Conflict);
        }
        let qti_item = if let Some(promotion) = qti_promotion {
            let row = sqlx::query(
                "SELECT payload, payload_sha256 FROM ple_read_committed_qti_import($1, $2, $3)",
            )
            .bind(context.tenant_id().as_uuid())
            .bind(promotion.staging.workspace.as_uuid())
            .bind(promotion.staging.import.as_uuid())
            .fetch_optional(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?
            .ok_or(StoreError::NotFound)?;
            let registry: QtiImportRegistry = decode_payload_row(&row)?;
            validate_qti_publication_promotion(context, &command, promotion, &registry)?;
            let question_model::DraftQuestionSource::Qti { item_id, .. } =
                &command.expected_draft.question.source
            else {
                unreachable!("QTI promotion was matched against a QTI draft");
            };
            Some(item_id.clone())
        } else {
            None
        };

        let publication = command.publication;
        let (authors, previous_version, derived_from, existing_display_identity) =
            if let Some(revises) = command.expected_draft.revises {
                if publication.problem != revises.problem {
                    return Err(StoreError::InvalidRecord(
                        "revision must remain in its existing problem chain".to_string(),
                    ));
                }
                let base_row = sqlx::query(
                    "SELECT pv.problem_id, p.public_id, pv.version_id, pv.version_number, \
                            pvp.payload, pvp.payload_sha256, pv.lifecycle, pv.lifecycle_reason \
                     FROM problem_version AS pv \
                     JOIN problem AS p USING (problem_id) \
                     JOIN problem_version_payload AS pvp USING (problem_id, version_id) \
                     WHERE pv.problem_id = $1 AND pv.version_id = $2 \
                     FOR UPDATE OF pv",
                )
                .bind(revises.problem.as_uuid())
                .bind(revises.version.as_uuid())
                .fetch_optional(&mut *transaction)
                .await
                .map_err(map_sqlx_error)?
                .ok_or(StoreError::NotFound)?;
                let base = decode_catalog_payload_row(&base_row)?;
                if !base.authors.contains(&command.publisher) {
                    return Err(StoreError::Forbidden);
                }
                let has_successor: bool = sqlx::query_scalar(
                    "SELECT EXISTS(SELECT 1 FROM problem_version \
                     WHERE problem_id = $1 AND previous_version_id = $2)",
                )
                .bind(revises.problem.as_uuid())
                .bind(revises.version.as_uuid())
                .fetch_one(&mut *transaction)
                .await
                .map_err(map_sqlx_error)?;
                if has_successor {
                    return Err(StoreError::Conflict);
                }
                (
                    base.authors,
                    Some(revises.version),
                    base.derived_from,
                    Some((
                        base.public_id,
                        ProblemVersionNumber::new(
                            base.version_number.value().checked_add(1).ok_or_else(|| {
                                StoreError::Unavailable(
                                    "problem version number limit reached".to_string(),
                                )
                            })?,
                        )
                        .expect("incremented version remains positive"),
                    )),
                )
            } else {
                if let Some(source) = command.expected_draft.derived_from {
                    let source_visible: bool = sqlx::query_scalar(
                        "SELECT EXISTS(SELECT 1 FROM problem_version \
                         WHERE problem_id = $1 AND version_id = $2)",
                    )
                    .bind(source.problem.as_uuid())
                    .bind(source.version.as_uuid())
                    .fetch_one(&mut *transaction)
                    .await
                    .map_err(map_sqlx_error)?;
                    if !source_visible {
                        return Err(StoreError::NotFound);
                    }
                }
                (
                    vec![command.publisher],
                    None,
                    command.expected_draft.derived_from,
                    None,
                )
            };

        let duplicate_version: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM problem_version \
             WHERE problem_id = $1 AND version_id = $2)",
        )
        .bind(publication.problem.as_uuid())
        .bind(publication.version.as_uuid())
        .fetch_one(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        if duplicate_version {
            return Err(StoreError::AlreadyExists);
        }

        let (public_id, version_number) = match existing_display_identity {
            Some(identity) => identity,
            None => {
                let license =
                    serde_json::to_value(&command.expected_draft.question.metadata.license)
                        .map_err(|error| StoreError::InvalidRecord(error.to_string()))?;
                let license = license
                    .get("kind")
                    .and_then(Value::as_str)
                    .unwrap_or("other");
                let value: i64 = sqlx::query_scalar(
                    "INSERT INTO problem \
                     (problem_id, owner_tenant_id, owner_user_id, visibility, license) \
                     VALUES ($1, $2, $3, $4, $5) RETURNING public_id",
                )
                .bind(publication.problem.as_uuid())
                .bind(context.tenant_id().as_uuid())
                .bind(command.publisher.as_uuid())
                .bind(publication_scope_name(command.scope))
                .bind(license)
                .fetch_one(&mut *transaction)
                .await
                .map_err(map_sqlx_error)?;
                let value = u64::try_from(value).map_err(|_| {
                    StoreError::Unavailable("stored problem public ID is invalid".to_string())
                })?;
                (
                    ProblemPublicId::new(value).ok_or_else(|| {
                        StoreError::Unavailable("stored problem public ID is invalid".to_string())
                    })?,
                    ProblemVersionNumber::new(1).expect("one is positive"),
                )
            }
        };

        let published_at_millis: i64 = sqlx::query_scalar(
            "SELECT floor(extract(epoch FROM transaction_timestamp()) * 1000)::bigint",
        )
        .fetch_one(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let question = QuestionDefinition::from_draft(
            command.expected_draft.question.clone(),
            publication.problem,
            publication.version,
            command.published_source.clone(),
        );
        let record = PublishedProblemRecord {
            problem: publication.problem,
            public_id,
            version: publication.version,
            version_number,
            question,
            capabilities: command.capabilities,
            scope: command.scope,
            lifecycle: CatalogLifecycle::Published,
            authors,
            previous_version,
            derived_from,
            published_at: ActivityTimestamp::from_unix_millis(published_at_millis),
        };
        validate_published(&record)?;
        let (payload, checksum) = encode_payload(&record)?;

        if record.scope == PublicationScope::Institution {
            sqlx::query(
                "INSERT INTO catalog_tenant_grant (tenant_id, problem_id, version_id) \
                 VALUES ($1, $2, $3)",
            )
            .bind(context.tenant_id().as_uuid())
            .bind(record.problem.as_uuid())
            .bind(record.version.as_uuid())
            .execute(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        }
        insert_problem_version(&mut transaction, &record, &checksum).await?;
        sqlx::query(
            "INSERT INTO problem_version_payload \
             (problem_id, version_id, payload, payload_sha256) VALUES ($1, $2, $3, $4)",
        )
        .bind(record.problem.as_uuid())
        .bind(record.version.as_uuid())
        .bind(payload)
        .bind(checksum)
        .execute(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        if let Some(artifact) = command.source_artifact {
            insert_published_source_artifact(&mut transaction, &artifact).await?;
        }
        if let Some(promotion) = command.qti_promotion {
            for asset in &promotion.assets {
                insert_catalog_asset_delivery(&mut transaction, asset).await?;
            }
            let item_id = qti_item.expect("QTI promotion has an exact staged item");
            let promoted: bool =
                sqlx::query_scalar("SELECT ple_promote_qti_grading($1, $2, $3, $4, $5, $6)")
                    .bind(context.tenant_id().as_uuid())
                    .bind(promotion.staging.workspace.as_uuid())
                    .bind(promotion.staging.import.as_uuid())
                    .bind(record.problem.as_uuid())
                    .bind(record.version.as_uuid())
                    .bind(item_id)
                    .fetch_one(&mut *transaction)
                    .await
                    .map_err(map_sqlx_error)?;
            if !promoted {
                return Err(StoreError::Conflict);
            }
        }
        sqlx::query("DELETE FROM workspace_draft WHERE tenant_id = $1 AND workspace_id = $2")
            .bind(context.tenant_id().as_uuid())
            .bind(record.question.workspace.as_uuid())
            .execute(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(record)
    }

    async fn get_catalog_problem(
        &self,
        context: TenantContext,
        reference: ProblemVersionRef,
    ) -> Result<Option<PublishedProblemRecord>, StoreError> {
        let mut transaction = self.begin_tenant(context).await?;
        let row = sqlx::query(
            "SELECT pv.problem_id, p.public_id, pv.version_id, pv.version_number, \
                    pvp.payload, pvp.payload_sha256, pv.lifecycle, pv.lifecycle_reason \
             FROM problem_version AS pv \
             JOIN problem AS p USING (problem_id) \
             JOIN problem_version_payload AS pvp USING (problem_id, version_id) \
             WHERE pv.problem_id = $1 AND pv.version_id = $2",
        )
        .bind(reference.problem.as_uuid())
        .bind(reference.version.as_uuid())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let record = row.as_ref().map(decode_catalog_payload_row).transpose()?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(record)
    }

    async fn resolve_catalog_problem(
        &self,
        context: TenantContext,
        reference: question_model::ProblemDisplayRef,
    ) -> Result<Option<PublishedProblemRecord>, StoreError> {
        let requested_version = reference
            .version
            .map(|version| i64::try_from(version.value()))
            .transpose()
            .map_err(|_| StoreError::InvalidRecord("problem version is too large".to_string()))?;
        let public_id = i64::try_from(reference.problem.value())
            .map_err(|_| StoreError::InvalidRecord("problem ID is too large".to_string()))?;
        let mut transaction = self.begin_tenant(context).await?;
        let row = sqlx::query(
            "SELECT pv.problem_id, p.public_id, pv.version_id, pv.version_number, \
                    pvp.payload, pvp.payload_sha256, pv.lifecycle, pv.lifecycle_reason \
             FROM problem AS p \
             JOIN problem_version AS pv USING (problem_id) \
             JOIN problem_version_payload AS pvp USING (problem_id, version_id) \
             WHERE p.public_id = $1 \
               AND ($2::bigint IS NULL OR pv.version_number = $2) \
               AND pv.lifecycle IN ('published', 'deprecated') \
             ORDER BY pv.version_number DESC LIMIT 1",
        )
        .bind(public_id)
        .bind(requested_version)
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let record = row.as_ref().map(decode_catalog_payload_row).transpose()?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(record)
    }

    async fn list_catalog(
        &self,
        context: TenantContext,
        page: PageRequest,
    ) -> Result<Page<CatalogProblemSummary>, StoreError> {
        let cursor = page.after.as_ref().map(|value| value.as_str().to_string());
        let limit = i64::from(page.size.get()) + 1;
        let mut transaction = self.begin_tenant(context).await?;
        let rows = sqlx::query(
            "SELECT document.problem_id::text || '/' || document.version_id::text AS stable_key, \
                    document.problem_id, document.public_id, document.version_id, \
                    document.version_number, document.backend, document.capabilities, \
                    document.metadata, document.publication_scope, document.lifecycle, \
                    document.lifecycle_reason, document.authors, document.previous_version_id, \
                    document.derived_from_problem_id, document.derived_from_version_id, \
                    floor(extract(epoch FROM document.published_at) * 1000)::bigint \
                        AS published_at_millis \
             FROM catalog_search_document AS document \
             WHERE document.lifecycle = 'published' \
               AND ($1::text IS NULL \
                    OR document.problem_id::text || '/' || document.version_id::text > $1) \
             ORDER BY document.problem_id::text, document.version_id::text LIMIT $2",
        )
        .bind(cursor)
        .bind(limit)
        .fetch_all(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let result = catalog_summary_page_from_rows(rows, page.size.get())?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(result)
    }

    async fn list_catalog_taxonomy(
        &self,
        context: TenantContext,
        page: PageRequest,
    ) -> Result<Page<TaxonomyTerm>, StoreError> {
        let cursor = page.after.as_ref().map(|value| value.as_str().to_string());
        let limit = i64::from(page.size.get()) + 1;
        let mut transaction = self.begin_tenant(context).await?;
        let rows = sqlx::query(
            "SELECT stable_key, taxonomy_term \
             FROM ( \
                 SELECT DISTINCT ON (term_row.stable_key) \
                        term_row.stable_key, term_row.taxonomy_term \
                 FROM ( \
                     SELECT document.problem_id, document.version_id, \
                            encode(convert_to(term->>'scheme', 'UTF8'), 'hex') || '/' || \
                            encode(convert_to(term->>'code', 'UTF8'), 'hex') AS stable_key, \
                            term AS taxonomy_term \
                     FROM catalog_search_document AS document \
                     CROSS JOIN LATERAL jsonb_array_elements(document.taxonomy) AS term \
                     WHERE document.lifecycle = 'published' \
                 ) AS term_row \
                 ORDER BY term_row.stable_key, term_row.problem_id::text, \
                          term_row.version_id::text \
             ) AS distinct_term \
             WHERE $1::text IS NULL OR stable_key > $1 \
             ORDER BY stable_key LIMIT $2",
        )
        .bind(cursor)
        .bind(limit)
        .fetch_all(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let result = taxonomy_page_from_rows(rows, page.size.get())?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(result)
    }

    async fn search_catalog(
        &self,
        context: TenantContext,
        query: CatalogSearchQuery,
    ) -> Result<CatalogSearchPage, StoreError> {
        let query = query
            .normalized()
            .map_err(|error| StoreError::InvalidRecord(error.to_string()))?;
        let page = postgres_search_page_request(&query)?;
        let fingerprint = postgres_catalog_search_fingerprint(&query);
        let after = page
            .after
            .as_ref()
            .map(|cursor| decode_catalog_search_cursor(cursor.as_str(), &fingerprint))
            .transpose()?;
        let (after_problem, after_version) = after
            .map(|(problem, version)| (Some(problem), Some(version)))
            .unwrap_or((None, None));
        let text = query.text.clone();
        let taxonomy = Json(query.taxonomy.clone());
        let capabilities = Json(query.capabilities.clone());
        let licenses = Json(query.licenses.clone());
        let statistics = match query.statistics {
            CatalogStatisticsAvailability::Any => 0_i16,
            CatalogStatisticsAvailability::Available => 1_i16,
            CatalogStatisticsAvailability::Unavailable => 2_i16,
        };
        let limit = i64::from(page.size.get()) + 1;
        let mut transaction = self.begin_tenant_snapshot(context).await?;
        // All statements below remain in this one tenant-scoped transaction.
        // PostgreSQL's RLS visibility applies before these predicates; no
        // caller-provided tenant ID or payload join can widen the result.
        let rows = sqlx::query(
            "SELECT document.problem_id::text || '/' || document.version_id::text AS stable_key, \
                    document.problem_id, document.public_id, document.version_id, \
                    document.version_number, document.backend, document.capabilities, \
                    document.metadata, document.publication_scope, document.lifecycle, \
                    document.lifecycle_reason, document.authors, document.previous_version_id, \
                    document.derived_from_problem_id, document.derived_from_version_id, \
                    floor(extract(epoch FROM document.published_at) * 1000)::bigint \
                        AS published_at_millis \
             FROM catalog_search_view AS document \
             WHERE document.lifecycle = 'published' \
               AND ($1::text IS NULL OR document.search_text @@ websearch_to_tsquery('simple', $1)) \
               AND NOT EXISTS ( \
                   SELECT 1 FROM jsonb_array_elements($2::jsonb) AS wanted \
                   WHERE NOT EXISTS ( \
                       SELECT 1 FROM jsonb_array_elements(document.taxonomy) AS stored \
                       WHERE stored->>'scheme' = wanted->>'scheme' \
                         AND stored->>'code' = wanted->>'code' \
                   ) \
               ) \
               AND document.capabilities @> $3::jsonb \
               AND (jsonb_array_length($4::jsonb) = 0 OR document.license \
                    IN (SELECT jsonb_array_elements_text($4::jsonb))) \
               AND ($5::smallint <> 1 OR document.statistics_available) \
               AND ($5::smallint <> 2 OR NOT document.statistics_available) \
               AND ($6::uuid IS NULL OR (document.problem_id, document.version_id) > ($6, $7)) \
             ORDER BY document.problem_id, document.version_id LIMIT $8",
        )
        .bind(text.clone())
        .bind(taxonomy.clone())
        .bind(capabilities.clone())
        .bind(licenses.clone())
        .bind(statistics)
        .bind(after_problem)
        .bind(after_version)
        .bind(limit)
        .fetch_all(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let page_result = catalog_summary_page_from_rows(rows, page.size.get())?;
        let taxonomy_rows = sqlx::query(
            "WITH filtered AS ( \
                 SELECT document.metadata FROM catalog_search_view AS document \
                 WHERE document.lifecycle = 'published' \
                   AND ($1::text IS NULL OR document.search_text \
                        @@ websearch_to_tsquery('simple', $1)) \
                   AND NOT EXISTS (SELECT 1 FROM jsonb_array_elements($2::jsonb) AS wanted \
                       WHERE NOT EXISTS (SELECT 1 FROM jsonb_array_elements( \
                           document.taxonomy) AS stored \
                           WHERE stored->>'scheme' = wanted->>'scheme' AND stored->>'code' = wanted->>'code')) \
                   AND document.capabilities @> $3::jsonb \
                   AND (jsonb_array_length($4::jsonb) = 0 OR document.license \
                        IN (SELECT jsonb_array_elements_text($4::jsonb))) \
                   AND ($5::smallint <> 1 OR document.statistics_available) \
                   AND ($5::smallint <> 2 OR NOT document.statistics_available) \
             ) SELECT jsonb_build_object('scheme', term->>'scheme', 'code', term->>'code', \
                         'label', min(term->>'label')) AS taxonomy_term, count(*)::bigint AS facet_count \
               FROM filtered CROSS JOIN LATERAL jsonb_array_elements( \
                   CASE WHEN jsonb_typeof(metadata->'taxonomy') = 'array' \
                        THEN metadata->'taxonomy' ELSE '[]'::jsonb END) AS term \
               GROUP BY term->>'scheme', term->>'code' \
               ORDER BY count(*) DESC, term->>'scheme', term->>'code' LIMIT 64",
        )
        .bind(text.clone()).bind(taxonomy.clone()).bind(capabilities.clone()).bind(licenses.clone())
        .bind(statistics).fetch_all(&mut *transaction).await.map_err(map_sqlx_error)?;
        let capability_rows = sqlx::query(
            "WITH filtered AS (SELECT document.capabilities FROM catalog_search_view AS document \
               WHERE document.lifecycle = 'published' \
               AND ($1::text IS NULL OR document.search_text @@ websearch_to_tsquery('simple', $1)) \
               AND NOT EXISTS (SELECT 1 FROM jsonb_array_elements($2::jsonb) AS wanted WHERE NOT EXISTS (SELECT 1 FROM jsonb_array_elements(document.taxonomy) AS stored WHERE stored->>'scheme' = wanted->>'scheme' AND stored->>'code' = wanted->>'code')) \
               AND document.capabilities @> $3::jsonb AND (jsonb_array_length($4::jsonb) = 0 OR document.license IN (SELECT jsonb_array_elements_text($4::jsonb))) \
               AND ($5::smallint <> 1 OR document.statistics_available) \
               AND ($5::smallint <> 2 OR NOT document.statistics_available)) \
             SELECT capability, count(*)::bigint AS facet_count FROM filtered CROSS JOIN LATERAL jsonb_array_elements_text(capabilities) AS capability GROUP BY capability ORDER BY capability",
        ).bind(text.clone()).bind(taxonomy.clone()).bind(capabilities.clone()).bind(licenses.clone()).bind(statistics).fetch_all(&mut *transaction).await.map_err(map_sqlx_error)?;
        let license_rows = sqlx::query(
            "WITH filtered AS (SELECT document.license FROM catalog_search_view AS document \
               WHERE document.lifecycle = 'published' \
               AND ($1::text IS NULL OR document.search_text @@ websearch_to_tsquery('simple', $1)) \
               AND NOT EXISTS (SELECT 1 FROM jsonb_array_elements($2::jsonb) AS wanted WHERE NOT EXISTS (SELECT 1 FROM jsonb_array_elements(document.taxonomy) AS stored WHERE stored->>'scheme' = wanted->>'scheme' AND stored->>'code' = wanted->>'code')) \
               AND document.capabilities @> $3::jsonb AND (jsonb_array_length($4::jsonb) = 0 OR document.license IN (SELECT jsonb_array_elements_text($4::jsonb))) \
               AND ($5::smallint <> 1 OR document.statistics_available) \
               AND ($5::smallint <> 2 OR NOT document.statistics_available)) \
             SELECT license, count(*)::bigint AS facet_count FROM filtered GROUP BY license ORDER BY license",
        ).bind(text.clone()).bind(taxonomy.clone()).bind(capabilities.clone()).bind(licenses.clone()).bind(statistics).fetch_all(&mut *transaction).await.map_err(map_sqlx_error)?;
        let statistics_facet = sqlx::query(
            "SELECT count(*) FILTER (WHERE document.statistics_available)::bigint AS available, \
                    count(*) FILTER (WHERE NOT document.statistics_available)::bigint AS unavailable \
             FROM catalog_search_view AS document \
             WHERE document.lifecycle = 'published' \
             AND ($1::text IS NULL OR document.search_text @@ websearch_to_tsquery('simple', $1)) \
             AND NOT EXISTS (SELECT 1 FROM jsonb_array_elements($2::jsonb) AS wanted WHERE NOT EXISTS (SELECT 1 FROM jsonb_array_elements(document.taxonomy) AS stored WHERE stored->>'scheme' = wanted->>'scheme' AND stored->>'code' = wanted->>'code')) \
             AND document.capabilities @> $3::jsonb AND (jsonb_array_length($4::jsonb) = 0 OR document.license IN (SELECT jsonb_array_elements_text($4::jsonb))) \
             AND ($5::smallint <> 1 OR document.statistics_available) \
             AND ($5::smallint <> 2 OR NOT document.statistics_available)",
        ).bind(text).bind(taxonomy).bind(capabilities).bind(licenses).bind(statistics).fetch_one(&mut *transaction).await.map_err(map_sqlx_error)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(CatalogSearchPage {
            items: page_result.items,
            next_cursor: page_result.next_cursor.map(|cursor| {
                let (problem, version) =
                    cursor.as_str().split_once('/').expect("catalog stable key");
                encode_catalog_search_cursor(
                    &fingerprint,
                    problem.parse().expect("catalog problem UUID"),
                    version.parse().expect("catalog version UUID"),
                )
            }),
            facets: CatalogSearchFacets {
                taxonomy: taxonomy_rows
                    .into_iter()
                    .map(decode_catalog_taxonomy_facet)
                    .collect::<Result<_, _>>()?,
                capabilities: capability_rows
                    .into_iter()
                    .map(decode_catalog_capability_facet)
                    .collect::<Result<_, _>>()?,
                licenses: license_rows
                    .into_iter()
                    .map(decode_catalog_license_facet)
                    .collect::<Result<_, _>>()?,
                statistics: CatalogStatisticsFacet {
                    available: u64::try_from(
                        statistics_facet
                            .try_get::<i64, _>("available")
                            .map_err(map_sqlx_error)?,
                    )
                    .map_err(|_| StoreError::Unavailable("catalog count overflow".to_string()))?,
                    unavailable: u64::try_from(
                        statistics_facet
                            .try_get::<i64, _>("unavailable")
                            .map_err(map_sqlx_error)?,
                    )
                    .map_err(|_| StoreError::Unavailable("catalog count overflow".to_string()))?,
                },
            },
        })
    }

    async fn get_catalog_detail(
        &self,
        context: TenantContext,
        reference: ProblemVersionRef,
    ) -> Result<Option<CatalogProblemDetail>, StoreError> {
        // Keep the authored prompt and its safe aggregate projection in one
        // tenant-scoped snapshot. The statistics statement calls only the
        // k-gated reader; it never joins catalog payload or learner history.
        let mut transaction = self.begin_tenant_snapshot(context).await?;
        let row = sqlx::query(
            "SELECT pv.problem_id, p.public_id, pv.version_id, pv.version_number, \
                    pvp.payload, pvp.payload_sha256, pv.lifecycle, pv.lifecycle_reason \
             FROM problem_version AS pv \
             JOIN problem AS p USING (problem_id) \
             JOIN problem_version_payload AS pvp USING (problem_id, version_id) \
             WHERE pv.problem_id = $1 AND pv.version_id = $2",
        )
        .bind(reference.problem.as_uuid())
        .bind(reference.version.as_uuid())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let record = row.as_ref().map(decode_catalog_payload_row).transpose()?;
        let statistics = if record.is_some() {
            let row = sqlx::query(
                "SELECT cohort_size, difficulty_index, attempts_mean, time_median_seconds_estimate, \
                        discrimination_index \
                 FROM ple_question_statistics_view($1, $2)",
            )
            .bind(reference.problem.as_uuid())
            .bind(reference.version.as_uuid())
            .fetch_optional(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
            question_statistics_disclosure_from_row(row.as_ref())?
        } else {
            QuestionStatisticsDisclosure::Suppressed
        };
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(record.map(|record| CatalogProblemDetail {
            summary: record.summary(),
            prompt: record.question.prompt,
            statistics: match statistics {
                QuestionStatisticsDisclosure::Suppressed => {
                    question_model::CatalogStatisticsStatus::Unavailable
                }
                QuestionStatisticsDisclosure::Available(view) => {
                    question_model::CatalogStatisticsStatus::Available(view)
                }
            },
        }))
    }

    async fn transition_catalog_problem(
        &self,
        context: TenantContext,
        actor: UserId,
        reference: ProblemVersionRef,
        transition: CatalogTransition,
    ) -> Result<PublishedProblemRecord, StoreError> {
        let mut transaction = self.begin_tenant(context).await?;
        let row = sqlx::query(
            "SELECT pv.problem_id, p.public_id, pv.version_id, pv.version_number, \
                    pvp.payload, pvp.payload_sha256, pv.lifecycle, pv.lifecycle_reason \
             FROM problem_version AS pv \
             JOIN problem AS p USING (problem_id) \
             JOIN problem_version_payload AS pvp USING (problem_id, version_id) \
             WHERE pv.problem_id = $1 AND pv.version_id = $2 FOR UPDATE OF pv",
        )
        .bind(reference.problem.as_uuid())
        .bind(reference.version.as_uuid())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?
        .ok_or(StoreError::NotFound)?;
        let mut record = decode_catalog_payload_row(&row)?;
        if !record.authors.contains(&actor) {
            return Err(StoreError::Forbidden);
        }
        record.lifecycle = match (&record.lifecycle, transition) {
            (CatalogLifecycle::Published, CatalogTransition::Deprecate { reason }) => {
                CatalogLifecycle::Deprecated {
                    reason: validated_deprecation_reason(reason)?,
                }
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
        let (lifecycle, lifecycle_reason) = catalog_lifecycle_parts(&record.lifecycle);
        sqlx::query(
            "UPDATE problem_version SET lifecycle = $3, lifecycle_reason = $4 \
             WHERE problem_id = $1 AND version_id = $2",
        )
        .bind(reference.problem.as_uuid())
        .bind(reference.version.as_uuid())
        .bind(lifecycle)
        .bind(lifecycle_reason)
        .execute(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(record)
    }
}

#[cfg(feature = "postgres")]
#[async_trait]
impl CatalogSourceStore for PostgresStore {
    async fn catalog_source_artifact(
        &self,
        context: TenantContext,
        reference: ProblemVersionRef,
    ) -> Result<Option<PublishedSourceArtifact>, StoreError> {
        let mut transaction = self.begin_tenant(context).await?;
        let row = sqlx::query(
            "SELECT backend, payload, payload_sha256 FROM published_source_artifact \
             WHERE problem_id = $1 AND version_id = $2",
        )
        .bind(reference.problem.as_uuid())
        .bind(reference.version.as_uuid())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let artifact: Option<PublishedSourceArtifact> =
            row.as_ref().map(decode_payload_row).transpose()?;
        if let Some(ref artifact) = artifact {
            let stored_backend: String = row
                .as_ref()
                .expect("artifact row exists when payload decoded")
                .get("backend");
            if stored_backend != question_backend_name(artifact.backend) {
                return Err(StoreError::InvalidRecord(
                    "stored source artifact backend does not match its payload".to_string(),
                ));
            }
            validate_source_artifact_identity(reference, artifact.backend, artifact)?;
        }
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(artifact)
    }
}

#[cfg(feature = "postgres")]
fn decode_stored_assignment_timing(
    row: &PgRow,
    tenant: TenantId,
) -> Result<StoredAssignmentTiming, StoreError> {
    let auto_submit: bool = row.try_get("auto_submit").map_err(map_sqlx_error)?;
    if !auto_submit {
        return Err(StoreError::Unavailable(
            "stored assignment uses unsupported overtime behavior".to_string(),
        ));
    }
    let late_submission: String = row
        .try_get("late_submission_policy")
        .map_err(map_sqlx_error)?;
    let timestamp = |name| -> Result<Option<ActivityTimestamp>, StoreError> {
        Ok(row
            .try_get::<Option<i64>, _>(name)
            .map_err(map_sqlx_error)?
            .map(ActivityTimestamp::from_unix_millis))
    };
    let time_limit_seconds: Option<i32> =
        row.try_get("time_limit_seconds").map_err(map_sqlx_error)?;
    let attempt_limit: Option<i32> = row.try_get("attempt_limit").map_err(map_sqlx_error)?;
    Ok(StoredAssignmentTiming {
        tenant,
        course: CourseId::from_uuid(row.try_get("course_id").map_err(map_sqlx_error)?),
        assignment: AssignmentId::from_uuid(row.try_get("assignment_id").map_err(map_sqlx_error)?),
        policy: AssignmentTimingPolicy {
            visible: row.try_get("visible").map_err(map_sqlx_error)?,
            available_at: timestamp("available_at_millis")?,
            due_at: timestamp("due_at_millis")?,
            closes_at: timestamp("closes_at_millis")?,
            late_submission: parse_late_submission_policy(&late_submission)?,
            time_limit_seconds: time_limit_seconds
                .map(|value| {
                    u32::try_from(value).map_err(|_| {
                        StoreError::Unavailable(
                            "stored assignment time limit is invalid".to_string(),
                        )
                    })
                })
                .transpose()?,
            attempt_limit: attempt_limit
                .map(|value| {
                    u32::try_from(value).map_err(|_| {
                        StoreError::Unavailable(
                            "stored assignment attempt limit is invalid".to_string(),
                        )
                    })
                })
                .transpose()?,
            deadline_behavior: AssignmentDeadlineBehavior::AutoSubmit,
        },
        revision: AssignmentRevision::from_stored(
            row.try_get("revision").map_err(map_sqlx_error)?,
        )?,
    })
}

#[cfg(feature = "postgres")]
fn parse_late_submission_policy(value: &str) -> Result<LateSubmissionPolicy, StoreError> {
    match value {
        "accept" => Ok(LateSubmissionPolicy::Accept),
        "mark_late" => Ok(LateSubmissionPolicy::MarkLate),
        "reject" => Ok(LateSubmissionPolicy::Reject),
        _ => Err(StoreError::Unavailable(
            "stored late-submission policy is invalid".to_string(),
        )),
    }
}

#[cfg(feature = "postgres")]
fn late_submission_policy_name(value: LateSubmissionPolicy) -> &'static str {
    match value {
        LateSubmissionPolicy::Accept => "accept",
        LateSubmissionPolicy::MarkLate => "mark_late",
        LateSubmissionPolicy::Reject => "reject",
    }
}

#[cfg(feature = "postgres")]
async fn load_postgres_course_group_members(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    group: CourseGroupId,
) -> Result<Vec<UserId>, StoreError> {
    sqlx::query_scalar::<_, Uuid>(
        "SELECT user_id FROM course_group_member WHERE tenant_id = $1 \
         AND course_group_id = $2 ORDER BY user_id",
    )
    .bind(tenant.as_uuid())
    .bind(group.as_uuid())
    .fetch_all(&mut **transaction)
    .await
    .map_err(map_sqlx_error)
    .map(|members| members.into_iter().map(UserId::from_uuid).collect())
}

#[cfg(feature = "postgres")]
async fn lock_postgres_assignment_policy(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    assignment: AssignmentId,
) -> Result<(), StoreError> {
    // Serialize attempt issue/start with base timing, exception, and group
    // membership changes. Callers take this advisory lock before assignment
    // and active attempt/timing row locks; multi-assignment callers sort IDs.
    sqlx::query(
        "SELECT pg_advisory_xact_lock(hashtextextended($1::uuid::text || ':' || $2::uuid::text, 0))",
    )
    .bind(tenant.as_uuid())
    .bind(assignment.as_uuid())
    .execute(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    Ok(())
}

#[cfg(feature = "postgres")]
async fn load_postgres_assignment_timing(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    assignment: AssignmentId,
    lock: bool,
) -> Result<Option<StoredAssignmentTiming>, StoreError> {
    let row = if lock {
        sqlx::query(
            "SELECT assignment_id, course_id, visible, \
                    floor(extract(epoch FROM available_at) * 1000)::bigint AS available_at_millis, \
                    floor(extract(epoch FROM due_at) * 1000)::bigint AS due_at_millis, \
                    floor(extract(epoch FROM closes_at) * 1000)::bigint AS closes_at_millis, \
                    late_submission_policy, time_limit_seconds, auto_submit, attempt_limit, revision \
             FROM assignment WHERE tenant_id = $1 AND assignment_id = $2 FOR UPDATE",
        )
        .bind(tenant.as_uuid())
        .bind(assignment.as_uuid())
        .fetch_optional(&mut **transaction)
        .await
        .map_err(map_sqlx_error)?
    } else {
        sqlx::query(
            "SELECT assignment_id, course_id, visible, \
                    floor(extract(epoch FROM available_at) * 1000)::bigint AS available_at_millis, \
                    floor(extract(epoch FROM due_at) * 1000)::bigint AS due_at_millis, \
                    floor(extract(epoch FROM closes_at) * 1000)::bigint AS closes_at_millis, \
                    late_submission_policy, time_limit_seconds, auto_submit, attempt_limit, revision \
             FROM assignment WHERE tenant_id = $1 AND assignment_id = $2",
        )
        .bind(tenant.as_uuid())
        .bind(assignment.as_uuid())
        .fetch_optional(&mut **transaction)
        .await
        .map_err(map_sqlx_error)?
    };
    row.as_ref()
        .map(|row| decode_stored_assignment_timing(row, tenant))
        .transpose()
}

#[cfg(feature = "postgres")]
async fn load_postgres_enrollment_by_student(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    assignment: AssignmentId,
    student: StudentId,
) -> Result<Option<AssignmentEnrollment>, StoreError> {
    let row = sqlx::query(
        "SELECT payload, payload_sha256 FROM enrollment WHERE tenant_id = $1 \
         AND assignment_id = $2 AND student_id = $3",
    )
    .bind(tenant.as_uuid())
    .bind(assignment.as_uuid())
    .bind(student.as_uuid())
    .fetch_optional(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    row.as_ref().map(decode_payload_row).transpose()
}

#[cfg(feature = "postgres")]
fn postgres_exception_timestamp_columns(
    value: Option<AssignmentExceptionTimestamp>,
) -> (Option<&'static str>, Option<i64>) {
    match value {
        None => (None, None),
        Some(AssignmentExceptionTimestamp::Unrestricted) => (Some("unrestricted"), None),
        Some(AssignmentExceptionTimestamp::At(value)) => (Some("at"), Some(value.as_unix_millis())),
    }
}

#[cfg(feature = "postgres")]
fn postgres_exception_limit_columns(
    value: Option<AssignmentExceptionLimit>,
) -> (Option<&'static str>, Option<i64>) {
    match value {
        None => (None, None),
        Some(AssignmentExceptionLimit::Unlimited) => (Some("unlimited"), None),
        Some(AssignmentExceptionLimit::Value(value)) => (Some("value"), Some(i64::from(value))),
    }
}

#[cfg(feature = "postgres")]
fn decode_postgres_exception_timestamp(
    mode: Option<String>,
    millis: Option<i64>,
) -> Result<Option<AssignmentExceptionTimestamp>, StoreError> {
    match (mode.as_deref(), millis) {
        (None, None) => Ok(None),
        (Some("unrestricted"), None) => Ok(Some(AssignmentExceptionTimestamp::Unrestricted)),
        (Some("at"), Some(value)) => Ok(Some(AssignmentExceptionTimestamp::At(
            ActivityTimestamp::from_unix_millis(value),
        ))),
        _ => Err(StoreError::Unavailable(
            "stored assignment exception timestamp is invalid".to_string(),
        )),
    }
}

#[cfg(feature = "postgres")]
fn decode_postgres_exception_limit(
    mode: Option<String>,
    value: Option<i32>,
) -> Result<Option<AssignmentExceptionLimit>, StoreError> {
    match (mode.as_deref(), value) {
        (None, None) => Ok(None),
        (Some("unlimited"), None) => Ok(Some(AssignmentExceptionLimit::Unlimited)),
        (Some("value"), Some(value)) => Ok(Some(AssignmentExceptionLimit::Value(
            u32::try_from(value).map_err(|_| {
                StoreError::Unavailable("stored assignment exception limit is invalid".to_string())
            })?,
        ))),
        _ => Err(StoreError::Unavailable(
            "stored assignment exception limit is invalid".to_string(),
        )),
    }
}

#[cfg(feature = "postgres")]
fn decode_postgres_policy_exception(row: &PgRow) -> Result<AssignmentPolicyException, StoreError> {
    let student: Option<Uuid> = row.try_get("student_id").map_err(map_sqlx_error)?;
    let group: Option<Uuid> = row.try_get("course_group_id").map_err(map_sqlx_error)?;
    let target = match (student, group) {
        (Some(student), None) => {
            AssignmentPolicyExceptionTarget::Student(StudentId::from_uuid(student))
        }
        (None, Some(group)) => {
            AssignmentPolicyExceptionTarget::CourseGroup(CourseGroupId::from_uuid(group))
        }
        _ => {
            return Err(StoreError::Unavailable(
                "stored assignment exception target is invalid".to_string(),
            ));
        }
    };
    let exception = AssignmentPolicyException {
        id: AssignmentPolicyExceptionId::from_uuid(
            row.try_get("assignment_policy_exception_id")
                .map_err(map_sqlx_error)?,
        ),
        target,
        available_at: decode_postgres_exception_timestamp(
            row.try_get("available_mode").map_err(map_sqlx_error)?,
            row.try_get("available_at_millis").map_err(map_sqlx_error)?,
        )?,
        closes_at: decode_postgres_exception_timestamp(
            row.try_get("closes_mode").map_err(map_sqlx_error)?,
            row.try_get("closes_at_millis").map_err(map_sqlx_error)?,
        )?,
        time_limit_seconds: decode_postgres_exception_limit(
            row.try_get("time_limit_mode").map_err(map_sqlx_error)?,
            row.try_get("time_limit_seconds").map_err(map_sqlx_error)?,
        )?,
        attempt_limit: decode_postgres_exception_limit(
            row.try_get("attempt_limit_mode").map_err(map_sqlx_error)?,
            row.try_get("attempt_limit").map_err(map_sqlx_error)?,
        )?,
    };
    validate_assignment_policy_exception(&exception).map_err(|error| {
        StoreError::Unavailable(format!("stored assignment exception is invalid: {error}"))
    })?;
    Ok(exception)
}

#[cfg(feature = "postgres")]
async fn load_postgres_policy_exception_identity_rows(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    assignment: AssignmentId,
    exception: AssignmentPolicyExceptionId,
    target: AssignmentPolicyExceptionTarget,
) -> Result<Vec<PgRow>, StoreError> {
    match target {
        AssignmentPolicyExceptionTarget::Student(student) => sqlx::query(
            "SELECT assignment_policy_exception_id, student_id, course_group_id, \
                    available_mode, \
                    floor(extract(epoch FROM available_at) * 1000)::bigint AS available_at_millis, \
                    closes_mode, \
                    floor(extract(epoch FROM closes_at) * 1000)::bigint AS closes_at_millis, \
                    time_limit_mode, time_limit_seconds, attempt_limit_mode, attempt_limit \
             FROM assignment_policy_exception WHERE tenant_id = $1 AND assignment_id = $2 \
               AND (assignment_policy_exception_id = $3 OR student_id = $4) FOR UPDATE",
        )
        .bind(tenant.as_uuid())
        .bind(assignment.as_uuid())
        .bind(exception.as_uuid())
        .bind(student.as_uuid())
        .fetch_all(&mut **transaction)
        .await
        .map_err(map_sqlx_error),
        AssignmentPolicyExceptionTarget::CourseGroup(group) => sqlx::query(
            "SELECT assignment_policy_exception_id, student_id, course_group_id, \
                    available_mode, \
                    floor(extract(epoch FROM available_at) * 1000)::bigint AS available_at_millis, \
                    closes_mode, \
                    floor(extract(epoch FROM closes_at) * 1000)::bigint AS closes_at_millis, \
                    time_limit_mode, time_limit_seconds, attempt_limit_mode, attempt_limit \
             FROM assignment_policy_exception WHERE tenant_id = $1 AND assignment_id = $2 \
               AND (assignment_policy_exception_id = $3 OR course_group_id = $4) FOR UPDATE",
        )
        .bind(tenant.as_uuid())
        .bind(assignment.as_uuid())
        .bind(exception.as_uuid())
        .bind(group.as_uuid())
        .fetch_all(&mut **transaction)
        .await
        .map_err(map_sqlx_error),
    }
}

#[cfg(feature = "postgres")]
async fn load_postgres_resolved_assignment_policy(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    assignment: AssignmentId,
    enrollment: &AssignmentEnrollment,
    base_override: Option<AssignmentTimingPolicy>,
) -> Result<crate::ResolvedAssignmentTimingPolicy, StoreError> {
    let base = match base_override {
        Some(policy) => policy,
        None => load_postgres_assignment_timing_policy(transaction, tenant, assignment).await?,
    };
    let rows = sqlx::query(
        "SELECT exception.assignment_policy_exception_id, exception.student_id, \
                exception.course_group_id, exception.available_mode, \
                floor(extract(epoch FROM exception.available_at) * 1000)::bigint AS available_at_millis, \
                exception.closes_mode, \
                floor(extract(epoch FROM exception.closes_at) * 1000)::bigint AS closes_at_millis, \
                exception.time_limit_mode, exception.time_limit_seconds, \
                exception.attempt_limit_mode, exception.attempt_limit \
         FROM assignment_policy_exception AS exception \
         WHERE exception.tenant_id = $1 AND exception.assignment_id = $2 \
           AND (exception.student_id = $3 OR EXISTS ( \
                SELECT 1 FROM course_group_member AS member \
                 WHERE member.tenant_id = exception.tenant_id \
                   AND member.course_group_id = exception.course_group_id \
                   AND member.user_id = $4)) \
         ORDER BY exception.student_id NULLS LAST, exception.course_group_id NULLS LAST",
    )
    .bind(tenant.as_uuid())
    .bind(assignment.as_uuid())
    .bind(enrollment.student.as_uuid())
    .bind(enrollment.user.as_uuid())
    .fetch_all(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    let exceptions = rows
        .iter()
        .map(decode_postgres_policy_exception)
        .collect::<Result<Vec<_>, StoreError>>()?;
    resolve_assignment_policy(base, &exceptions)
}

#[cfg(feature = "postgres")]
async fn lock_postgres_active_timing_rows(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    assignment: AssignmentId,
) -> Result<Vec<PgRow>, StoreError> {
    let active_attempts = sqlx::query_scalar::<_, Uuid>(
        "SELECT attempt.attempt_id FROM question_attempt AS attempt \
         JOIN assignment_run AS run ON run.tenant_id = attempt.tenant_id \
            AND run.run_id = attempt.run_id \
         JOIN enrollment ON enrollment.tenant_id = run.tenant_id \
            AND enrollment.enrollment_id = run.enrollment_id \
         WHERE attempt.tenant_id = $1 AND enrollment.assignment_id = $2 \
           AND attempt.attempt_status = 'in_progress' \
         ORDER BY attempt.attempt_id FOR UPDATE OF attempt",
    )
    .bind(tenant.as_uuid())
    .bind(assignment.as_uuid())
    .fetch_all(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    let rows = sqlx::query(
        "SELECT timing.attempt_id, timing.authored_grace_seconds, timing.timing_generation, \
                timing.job_id, job.state AS job_state, run.payload AS run_payload, \
                run.payload_sha256 AS run_payload_sha256, run.enrollment_id, \
                floor(extract(epoch FROM timing.authored_deadline) * 1000)::bigint \
                    AS authored_deadline_millis \
         FROM attempt_timing_current AS timing \
         JOIN question_attempt AS attempt ON attempt.tenant_id = timing.tenant_id \
            AND attempt.attempt_id = timing.attempt_id \
            AND attempt.occurred_at = timing.attempt_occurred_at \
         JOIN assignment_run AS run ON run.tenant_id = attempt.tenant_id \
            AND run.run_id = attempt.run_id \
         LEFT JOIN worker_job AS job ON job.job_id = timing.job_id \
         WHERE timing.tenant_id = $1 AND timing.assignment_id = $2 \
           AND attempt.attempt_status = 'in_progress' \
         ORDER BY timing.attempt_id FOR UPDATE OF timing",
    )
    .bind(tenant.as_uuid())
    .bind(assignment.as_uuid())
    .fetch_all(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    if rows.len() != active_attempts.len() {
        return Err(StoreError::Unavailable(
            "an active attempt is missing its current timing row".to_string(),
        ));
    }
    Ok(rows)
}

#[cfg(feature = "postgres")]
async fn apply_postgres_locked_timing_rows(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    assignment: AssignmentId,
    base_override: Option<AssignmentTimingPolicy>,
    now: ActivityTimestamp,
    rows: Vec<PgRow>,
) -> Result<(), StoreError> {
    for row in rows {
        let enrollment_id =
            EnrollmentId::from_uuid(row.try_get("enrollment_id").map_err(map_sqlx_error)?);
        let enrollment_row = sqlx::query(
            "SELECT payload, payload_sha256 FROM enrollment WHERE tenant_id = $1 \
             AND enrollment_id = $2",
        )
        .bind(tenant.as_uuid())
        .bind(enrollment_id.as_uuid())
        .fetch_optional(&mut **transaction)
        .await
        .map_err(map_sqlx_error)?
        .ok_or(StoreError::NotFound)?;
        let enrollment: AssignmentEnrollment = decode_payload_row(&enrollment_row)?;
        let resolution = load_postgres_resolved_assignment_policy(
            transaction,
            tenant,
            assignment,
            &enrollment,
            base_override,
        )
        .await?;
        apply_postgres_active_timing_update(transaction, tenant, &resolution, now, &row).await?;
    }
    Ok(())
}

#[cfg(feature = "postgres")]
async fn update_postgres_assignment_revision(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    assignment: AssignmentId,
    previous: AssignmentRevision,
    next: AssignmentRevision,
) -> Result<(), StoreError> {
    let updated = sqlx::query(
        "UPDATE assignment SET revision = $3, updated_at = transaction_timestamp() \
         WHERE tenant_id = $1 AND assignment_id = $2 AND revision = $4",
    )
    .bind(tenant.as_uuid())
    .bind(assignment.as_uuid())
    .bind(i64::try_from(next.value()).map_err(|_| StoreError::Conflict)?)
    .bind(i64::try_from(previous.value()).map_err(|_| StoreError::Conflict)?)
    .execute(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    if updated.rows_affected() != 1 {
        return Err(StoreError::Conflict);
    }
    Ok(())
}

#[cfg(feature = "postgres")]
fn decode_postgres_resolved_attempt_timing(
    row: &PgRow,
    attempt: QuestionAttemptId,
) -> Result<ResolvedAttemptTiming, StoreError> {
    let timestamp = |column| {
        row.try_get::<Option<i64>, _>(column)
            .map_err(map_sqlx_error)
            .map(|value| value.map(ActivityTimestamp::from_unix_millis))
    };
    let time_limit: Option<i32> = row
        .try_get("resolved_time_limit_seconds")
        .map_err(map_sqlx_error)?;
    let attempt_limit: Option<i32> = row
        .try_get("resolved_attempt_limit")
        .map_err(map_sqlx_error)?;
    let sources: Json<Vec<AssignmentPolicyExceptionTarget>> =
        row.try_get("resolution_sources").map_err(map_sqlx_error)?;
    let policy = AssignmentTimingPolicy {
        visible: row.try_get("resolved_visible").map_err(map_sqlx_error)?,
        available_at: timestamp("available_at_millis")?,
        due_at: timestamp("due_at_millis")?,
        closes_at: timestamp("closes_at_millis")?,
        late_submission: parse_late_submission_policy(
            &row.try_get::<String, _>("resolved_late_submission_policy")
                .map_err(map_sqlx_error)?,
        )?,
        time_limit_seconds: time_limit
            .map(|value| {
                u32::try_from(value).map_err(|_| {
                    StoreError::Unavailable("stored resolved time limit is invalid".to_string())
                })
            })
            .transpose()?,
        attempt_limit: attempt_limit
            .map(|value| {
                u32::try_from(value).map_err(|_| {
                    StoreError::Unavailable("stored resolved attempt limit is invalid".to_string())
                })
            })
            .transpose()?,
        deadline_behavior: AssignmentDeadlineBehavior::AutoSubmit,
    };
    validate_assignment_timing(policy)?;
    Ok(ResolvedAttemptTiming {
        attempt,
        policy,
        contributors: sources.0,
    })
}

#[cfg(feature = "postgres")]
struct ResolvedPostgresAttemptTiming {
    effective_deadline: Option<ActivityTimestamp>,
    effective_grace_seconds: u32,
    auto_submit_at: Option<ActivityTimestamp>,
    resolution_kind: &'static str,
}

#[cfg(feature = "postgres")]
fn resolved_postgres_attempt_timing(
    policy: AssignmentTimingPolicy,
    run: &AssignmentRun,
    authored_deadline: Option<ActivityTimestamp>,
    authored_grace_seconds: u32,
) -> Result<ResolvedPostgresAttemptTiming, StoreError> {
    let mut resolved = authored_deadline
        .map(|deadline| (deadline, authored_grace_seconds, 4_u8, "authored_question"));
    let mut consider =
        |deadline: ActivityTimestamp, grace_seconds: u32, priority: u8, source: &'static str| {
            if resolved.is_none_or(|(current_deadline, current_grace, current_priority, _)| {
                (deadline, grace_seconds, priority)
                    < (current_deadline, current_grace, current_priority)
            }) {
                resolved = Some((deadline, grace_seconds, priority, source));
            }
        };
    if let Some(seconds) = policy.time_limit_seconds {
        consider(
            add_seconds(run.started_at, seconds, "assignment time limit")?,
            0,
            3,
            "assignment_time_limit",
        );
    }
    if policy.late_submission == LateSubmissionPolicy::Reject
        && let Some(due_at) = policy.due_at
    {
        consider(due_at, 0, 2, "due_at");
    }
    if let Some(closes_at) = policy.closes_at {
        consider(closes_at, 0, 1, "closes_at");
    }
    let auto_submit_at = resolved
        .map(|(deadline, grace_seconds, _, _)| {
            add_seconds(deadline, grace_seconds, "attempt auto-submit deadline")
        })
        .transpose()?;
    let (effective_deadline, effective_grace_seconds, resolution_kind) = match resolved {
        Some((deadline, grace_seconds, _, source)) => (Some(deadline), grace_seconds, source),
        None => (None, 0, "untimed"),
    };
    Ok(ResolvedPostgresAttemptTiming {
        effective_deadline,
        effective_grace_seconds,
        auto_submit_at,
        resolution_kind,
    })
}

#[cfg(feature = "postgres")]
async fn apply_postgres_active_timing_update(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    resolution: &crate::ResolvedAssignmentTimingPolicy,
    now: ActivityTimestamp,
    row: &PgRow,
) -> Result<(), StoreError> {
    let attempt = QuestionAttemptId::from_uuid(row.try_get("attempt_id").map_err(map_sqlx_error)?);
    let authored_deadline = row
        .try_get::<Option<i64>, _>("authored_deadline_millis")
        .map_err(map_sqlx_error)?
        .map(ActivityTimestamp::from_unix_millis);
    let authored_grace = u32::try_from(
        row.try_get::<i32, _>("authored_grace_seconds")
            .map_err(map_sqlx_error)?,
    )
    .map_err(|_| StoreError::Unavailable("stored authored grace is invalid".to_string()))?;
    let generation = u64::try_from(
        row.try_get::<i64, _>("timing_generation")
            .map_err(map_sqlx_error)?,
    )
    .map_err(|_| StoreError::Unavailable("stored timing generation is invalid".to_string()))?
    .checked_add(1)
    .ok_or(StoreError::Conflict)?;
    let run: AssignmentRun = decode_payload_row_named(row, "run_payload", "run_payload_sha256")?;
    let ResolvedPostgresAttemptTiming {
        effective_deadline,
        effective_grace_seconds: effective_grace,
        auto_submit_at,
        resolution_kind,
    } = resolved_postgres_attempt_timing(
        resolution.policy,
        &run,
        authored_deadline,
        authored_grace,
    )?;
    let previous_job = row
        .try_get::<Option<Uuid>, _>("job_id")
        .map_err(map_sqlx_error)?
        .map(JobId::from_uuid);
    let job_state: Option<String> = row.try_get("job_state").map_err(map_sqlx_error)?;
    let immediate = auto_submit_at.is_some_and(|deadline| deadline <= now);

    if immediate || auto_submit_at.is_none() {
        if let (Some(job), Some(state)) = (previous_job, job_state.as_deref())
            && matches!(state, "ready" | "leased")
        {
            let canceled: bool = sqlx::query_scalar("SELECT ple_cancel_attempt_timing_job($1, $2)")
                .bind(tenant.as_uuid())
                .bind(job.as_uuid())
                .fetch_one(&mut **transaction)
                .await
                .map_err(map_sqlx_error)?;
            if !canceled {
                return Err(StoreError::Conflict);
            }
        }
        update_postgres_attempt_timing_row(
            transaction,
            tenant,
            attempt,
            effective_deadline,
            effective_grace,
            auto_submit_at,
            resolution_kind,
            resolution,
            generation,
            None,
        )
        .await?;
        if immediate {
            let updated = sqlx::query(
                "UPDATE question_attempt SET attempt_status = 'auto_submitted', \
                        submitted_at = transaction_timestamp() \
                 WHERE tenant_id = $1 AND attempt_id = $2 AND attempt_status = 'in_progress'",
            )
            .bind(tenant.as_uuid())
            .bind(attempt.as_uuid())
            .execute(&mut **transaction)
            .await
            .map_err(map_sqlx_error)?;
            if updated.rows_affected() != 1 {
                return Err(StoreError::Conflict);
            }
        }
        return Ok(());
    }

    let available_at = auto_submit_at.expect("timed attempt has an auto-submit time");
    let payload = serde_json::to_value(JobPayload::AutoSubmitAttempt {
        attempt,
        timing_generation: generation,
    })
    .map_err(|error| StoreError::InvalidRecord(error.to_string()))?;
    match (previous_job, job_state.as_deref()) {
        (Some(job), Some("ready")) => {
            update_postgres_attempt_timing_row(
                transaction,
                tenant,
                attempt,
                effective_deadline,
                effective_grace,
                auto_submit_at,
                resolution_kind,
                resolution,
                generation,
                Some(job),
            )
            .await?;
            let changed: bool = sqlx::query_scalar(
                "SELECT ple_reschedule_attempt_timing_job($1, $2, $3, $4, \
                    TIMESTAMPTZ 'epoch' + $5::bigint * INTERVAL '1 millisecond')",
            )
            .bind(tenant.as_uuid())
            .bind(job.as_uuid())
            .bind(Option::<Uuid>::None)
            .bind(payload)
            .bind(available_at.as_unix_millis())
            .fetch_one(&mut **transaction)
            .await
            .map_err(map_sqlx_error)?;
            if !changed {
                return Err(StoreError::Conflict);
            }
        }
        (Some(job), Some("leased")) => {
            update_postgres_attempt_timing_row(
                transaction,
                tenant,
                attempt,
                effective_deadline,
                effective_grace,
                auto_submit_at,
                resolution_kind,
                resolution,
                generation,
                Some(job),
            )
            .await?;
        }
        _ => {
            let job = JobId::generate()?;
            sqlx::query(
                "INSERT INTO worker_job \
                 (job_id, tenant_id, payload, state, available_at, max_attempts) \
                 VALUES ($1, $2, $3, 'ready', \
                    TIMESTAMPTZ 'epoch' + $4::bigint * INTERVAL '1 millisecond', 10)",
            )
            .bind(job.as_uuid())
            .bind(tenant.as_uuid())
            .bind(payload)
            .bind(available_at.as_unix_millis())
            .execute(&mut **transaction)
            .await
            .map_err(map_sqlx_error)?;
            update_postgres_attempt_timing_row(
                transaction,
                tenant,
                attempt,
                effective_deadline,
                effective_grace,
                auto_submit_at,
                resolution_kind,
                resolution,
                generation,
                Some(job),
            )
            .await?;
        }
    }
    Ok(())
}

#[cfg(feature = "postgres")]
async fn load_postgres_assignment_timing_policy(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    assignment: AssignmentId,
) -> Result<AssignmentTimingPolicy, StoreError> {
    let row = sqlx::query(
        "SELECT assignment_id, course_id, visible, \
                floor(extract(epoch FROM available_at) * 1000)::bigint AS available_at_millis, \
                floor(extract(epoch FROM due_at) * 1000)::bigint AS due_at_millis, \
                floor(extract(epoch FROM closes_at) * 1000)::bigint AS closes_at_millis, \
                late_submission_policy, time_limit_seconds, auto_submit, attempt_limit, revision \
         FROM assignment WHERE tenant_id = $1 AND assignment_id = $2",
    )
    .bind(tenant.as_uuid())
    .bind(assignment.as_uuid())
    .fetch_optional(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?
    .ok_or(StoreError::NotFound)?;
    Ok(decode_stored_assignment_timing(&row, tenant)?.policy)
}

#[cfg(feature = "postgres")]
async fn cancel_postgres_attempt_timing_job(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    attempt: QuestionAttemptId,
) -> Result<(), StoreError> {
    let row = sqlx::query(
        "SELECT timing.job_id, job.state AS job_state \
         FROM attempt_timing_current AS timing \
         LEFT JOIN worker_job AS job ON job.job_id = timing.job_id \
         WHERE timing.tenant_id = $1 AND timing.attempt_id = $2 FOR UPDATE OF timing",
    )
    .bind(tenant.as_uuid())
    .bind(attempt.as_uuid())
    .fetch_optional(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?
    .ok_or_else(|| {
        StoreError::Unavailable("attempt is missing its current timing row".to_string())
    })?;
    let job = row
        .try_get::<Option<Uuid>, _>("job_id")
        .map_err(map_sqlx_error)?
        .map(JobId::from_uuid);
    let state: Option<String> = row.try_get("job_state").map_err(map_sqlx_error)?;
    if let (Some(job), Some(state)) = (job, state.as_deref())
        && matches!(state, "ready" | "leased")
    {
        let canceled: bool = sqlx::query_scalar("SELECT ple_cancel_attempt_timing_job($1, $2)")
            .bind(tenant.as_uuid())
            .bind(job.as_uuid())
            .fetch_one(&mut **transaction)
            .await
            .map_err(map_sqlx_error)?;
        if !canceled {
            return Err(StoreError::Conflict);
        }
    }
    sqlx::query(
        "UPDATE attempt_timing_current SET job_id = NULL, updated_at = transaction_timestamp() \
         WHERE tenant_id = $1 AND attempt_id = $2",
    )
    .bind(tenant.as_uuid())
    .bind(attempt.as_uuid())
    .execute(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    Ok(())
}

#[cfg(feature = "postgres")]
fn timing_policy_grace_seconds(policy: TimingPolicy) -> u32 {
    match policy {
        TimingPolicy::Untimed => 0,
        TimingPolicy::PerQuestion { grace_seconds, .. }
        | TimingPolicy::PerAttempt { grace_seconds, .. } => grace_seconds,
    }
}

#[cfg(feature = "postgres")]
#[allow(clippy::too_many_arguments)]
async fn update_postgres_attempt_timing_row(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    attempt: QuestionAttemptId,
    effective_deadline: Option<ActivityTimestamp>,
    effective_grace_seconds: u32,
    auto_submit_at: Option<ActivityTimestamp>,
    resolution_kind: &str,
    resolution: &crate::ResolvedAssignmentTimingPolicy,
    timing_generation: u64,
    job: Option<JobId>,
) -> Result<(), StoreError> {
    let updated = sqlx::query(
        "UPDATE attempt_timing_current \
         SET effective_deadline = TIMESTAMPTZ 'epoch' + $3::bigint * INTERVAL '1 millisecond', \
             effective_grace_seconds = $4, \
             auto_submit_at = TIMESTAMPTZ 'epoch' + $5::bigint * INTERVAL '1 millisecond', \
             resolution_kind = $6, resolved_visible = $7, \
             resolved_available_at = TIMESTAMPTZ 'epoch' + $8::bigint * INTERVAL '1 millisecond', \
             resolved_due_at = TIMESTAMPTZ 'epoch' + $9::bigint * INTERVAL '1 millisecond', \
             resolved_closes_at = TIMESTAMPTZ 'epoch' + $10::bigint * INTERVAL '1 millisecond', \
             resolved_late_submission_policy = $11, resolved_time_limit_seconds = $12, \
             resolved_attempt_limit = $13, resolution_sources = $14, \
             timing_generation = $15, job_id = $16, \
             updated_at = transaction_timestamp() \
         WHERE tenant_id = $1 AND attempt_id = $2",
    )
    .bind(tenant.as_uuid())
    .bind(attempt.as_uuid())
    .bind(effective_deadline.map(|value| value.as_unix_millis()))
    .bind(i64::from(effective_grace_seconds))
    .bind(auto_submit_at.map(|value| value.as_unix_millis()))
    .bind(resolution_kind)
    .bind(resolution.policy.visible)
    .bind(
        resolution
            .policy
            .available_at
            .map(|value| value.as_unix_millis()),
    )
    .bind(resolution.policy.due_at.map(|value| value.as_unix_millis()))
    .bind(
        resolution
            .policy
            .closes_at
            .map(|value| value.as_unix_millis()),
    )
    .bind(late_submission_policy_name(
        resolution.policy.late_submission,
    ))
    .bind(resolution.policy.time_limit_seconds.map(i64::from))
    .bind(resolution.policy.attempt_limit.map(i64::from))
    .bind(
        serde_json::to_value(&resolution.contributors)
            .map_err(|error| StoreError::InvalidRecord(error.to_string()))?,
    )
    .bind(i64::try_from(timing_generation).map_err(|_| StoreError::Conflict)?)
    .bind(job.map(JobId::as_uuid))
    .execute(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    if updated.rows_affected() != 1 {
        return Err(StoreError::Conflict);
    }
    Ok(())
}

#[cfg(feature = "postgres")]
async fn start_or_resume_run(
    transaction: &mut Transaction<'_, Postgres>,
    context: TenantContext,
    actor: UserId,
    assignment_id: AssignmentId,
    proposed_run: RunId,
) -> Result<AssignmentRun, StoreError> {
    let tenant = context.tenant_id();
    let enrollment_row = sqlx::query(
        "SELECT payload, payload_sha256 FROM enrollment \
         WHERE tenant_id = $1 AND assignment_id = $2 AND user_id = $3 FOR UPDATE",
    )
    .bind(tenant.as_uuid())
    .bind(assignment_id.as_uuid())
    .bind(actor.as_uuid())
    .fetch_optional(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?
    .ok_or(StoreError::NotFound)?;
    let enrollment: AssignmentEnrollment = decode_payload_row(&enrollment_row)?;
    let active_row = sqlx::query(
        "SELECT payload, payload_sha256 FROM assignment_run \
         WHERE tenant_id = $1 AND enrollment_id = $2 AND completed_at IS NULL \
         ORDER BY run_number DESC LIMIT 1",
    )
    .bind(tenant.as_uuid())
    .bind(enrollment.id.as_uuid())
    .fetch_optional(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    if let Some(row) = active_row {
        return decode_payload_row(&row);
    }

    lock_postgres_assignment_policy(transaction, tenant, assignment_id).await?;
    let assignment = load_assignment_for_share(transaction, tenant, assignment_id).await?;
    let timing = load_postgres_resolved_assignment_policy(
        transaction,
        tenant,
        assignment_id,
        &enrollment,
        None,
    )
    .await?
    .policy;
    let now = database_timestamp(transaction).await?;
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
    if timing.late_submission == LateSubmissionPolicy::Reject
        && timing.due_at.is_some_and(|due_at| now > due_at)
    {
        return Err(StoreError::InvalidRecord(
            "assignment due date has passed".to_string(),
        ));
    }
    let previous = load_summary_for_update(transaction, tenant, enrollment.id).await?;
    if !continued_practice_allows_run(&previous, assignment.policies.continued_practice) {
        return Err(StoreError::InvalidRecord(
            "continued-practice policy does not permit another run".to_string(),
        ));
    }
    let max_run_number: i64 = sqlx::query_scalar(
        "SELECT COALESCE(MAX(run_number), 0) FROM assignment_run \
         WHERE tenant_id = $1 AND enrollment_id = $2",
    )
    .bind(tenant.as_uuid())
    .bind(enrollment.id.as_uuid())
    .fetch_one(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    if timing
        .attempt_limit
        .is_some_and(|limit| max_run_number >= i64::from(limit))
    {
        return Err(StoreError::InvalidRecord(
            "assignment attempt limit has been reached".to_string(),
        ));
    }
    let run_number = u32::try_from(max_run_number)
        .map_err(|_| StoreError::InvalidRecord("run number overflow".to_string()))?
        .checked_add(1)
        .ok_or_else(|| StoreError::InvalidRecord("run number overflow".to_string()))?;
    let run = AssignmentRun {
        id: proposed_run,
        tenant,
        enrollment: enrollment.id,
        run_number,
        started_at: now,
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
        domain::scoring::RunTransition::Started { at: now },
        grade_policy(&assignment),
    )?;
    let (payload, checksum) = encode_payload(&run)?;
    sqlx::query(
        "INSERT INTO assignment_run \
         (tenant_id, run_id, enrollment_id, run_number, started_at, payload, payload_sha256) \
         VALUES ($1, $2, $3, $4, transaction_timestamp(), $5, $6)",
    )
    .bind(tenant.as_uuid())
    .bind(run.id.as_uuid())
    .bind(run.enrollment.as_uuid())
    .bind(i64::from(run.run_number))
    .bind(payload)
    .bind(checksum)
    .execute(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    insert_assignment_run_items(transaction, &assignment, run.id).await?;
    store_summary(transaction, &next).await?;
    Ok(run)
}

#[cfg(feature = "postgres")]
async fn apply_postgres_attempt_support(
    transaction: &mut Transaction<'_, Postgres>,
    context: TenantContext,
    action_id: AttemptSupportActionId,
    actor: UserId,
    attempt_id: QuestionAttemptId,
    action: AttemptSupportAction,
) -> Result<AttemptSupportRecord, StoreError> {
    let tenant = context.tenant_id();
    let previous = load_attempt_for_external_update(transaction, tenant, attempt_id).await?;
    let run = load_run_for_update(transaction, tenant, previous.run).await?;
    let enrollment = load_enrollment_for_update(transaction, tenant, run.enrollment).await?;
    let assignment = load_assignment(transaction, tenant, enrollment.assignment).await?;
    if !postgres_is_course_instructor(transaction, tenant, assignment.course_id, actor).await? {
        return Err(StoreError::NotFound);
    }

    // The audit table is time-partitioned, so its primary key necessarily
    // includes occurred_at. Serialize this application-owned identity before
    // querying it to preserve cross-partition, cross-attempt retry safety.
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1::uuid::text, 0))")
        .bind(action_id.as_uuid())
        .execute(&mut **transaction)
        .await
        .map_err(map_sqlx_error)?;
    let prior_rows = sqlx::query(
        "SELECT actor_id, course_id, action, target_kind, target_id, payload, \
                payload_sha256, \
                floor(extract(epoch FROM occurred_at) * 1000)::bigint \
                    AS occurred_at_millis \
         FROM audit_event \
         WHERE tenant_id = $1 AND audit_event_id = $2 \
         ORDER BY occurred_at LIMIT 2",
    )
    .bind(tenant.as_uuid())
    .bind(action_id.as_uuid())
    .fetch_all(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    if prior_rows.len() > 1 {
        return Err(StoreError::Unavailable(
            "attempt support action identity is duplicated".to_string(),
        ));
    }
    if let Some(row) = prior_rows.first() {
        let payload: AttemptSupportAuditPayload = decode_payload_row(row)?;
        let prior_actor: Uuid = row.try_get("actor_id").map_err(map_sqlx_error)?;
        let prior_course: Option<Uuid> = row.try_get("course_id").map_err(map_sqlx_error)?;
        let prior_action: String = row.try_get("action").map_err(map_sqlx_error)?;
        let target_kind: String = row.try_get("target_kind").map_err(map_sqlx_error)?;
        let target_id: Uuid = row.try_get("target_id").map_err(map_sqlx_error)?;
        if prior_actor != actor.as_uuid()
            || prior_course != Some(assignment.course_id.as_uuid())
            || prior_action != action.audit_name()
            || target_kind != "question_attempt"
            || target_id != attempt_id.as_uuid()
        {
            return Err(StoreError::Conflict);
        }
        return Ok(AttemptSupportRecord {
            tenant,
            action: action_id,
            actor,
            attempt: attempt_id,
            kind: action,
            previous_status: payload.previous_status,
            resulting_status: payload.resulting_status,
            occurred_at: ActivityTimestamp::from_unix_millis(
                row.try_get("occurred_at_millis").map_err(map_sqlx_error)?,
            ),
        });
    }

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
    let updated = sqlx::query(
        "UPDATE question_attempt \
         SET attempt_status = $3, \
             submitted_at = CASE WHEN $4 THEN transaction_timestamp() ELSE submitted_at END \
         WHERE tenant_id = $1 AND attempt_id = $2",
    )
    .bind(tenant.as_uuid())
    .bind(attempt_id.as_uuid())
    .bind(attempt_status_name(resulting_status))
    .bind(action == AttemptSupportAction::ForceSubmit)
    .execute(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    if updated.rows_affected() != 1 {
        return Err(StoreError::Conflict);
    }
    cancel_postgres_attempt_timing_job(transaction, tenant, attempt_id).await?;

    let has_evaluation: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM submission_evaluation \
         WHERE tenant_id = $1 AND attempt_id = $2)",
    )
    .bind(tenant.as_uuid())
    .bind(attempt_id.as_uuid())
    .fetch_one(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    if action == AttemptSupportAction::Clear && has_evaluation {
        let row = sqlx::query(
            "UPDATE assignment \
             SET scoring_generation = scoring_generation + 1, \
                 scoring_status = 'recalculating', updated_at = transaction_timestamp() \
             WHERE tenant_id = $1 AND assignment_id = $2 \
             RETURNING scoring_generation",
        )
        .bind(tenant.as_uuid())
        .bind(assignment.id.as_uuid())
        .fetch_optional(&mut **transaction)
        .await
        .map_err(map_sqlx_error)?
        .ok_or(StoreError::NotFound)?;
        let generation = decode_scoring_generation(&row)?;
        let job = JobId::generate()?;
        let payload = serde_json::to_value(JobPayload::RecalculateAssignment {
            assignment: assignment.id,
            generation,
        })
        .map_err(|error| {
            StoreError::InvalidRecord(format!(
                "attempt clear scoring job serialization failed: {error}"
            ))
        })?;
        sqlx::query(
            "INSERT INTO worker_job (job_id, tenant_id, payload, state, max_attempts) \
             VALUES ($1, $2, $3, 'ready', 10)",
        )
        .bind(job.as_uuid())
        .bind(tenant.as_uuid())
        .bind(payload)
        .execute(&mut **transaction)
        .await
        .map_err(map_sqlx_error)?;
    }

    let occurred_at = database_timestamp(transaction).await?;
    let audit_payload = AttemptSupportAuditPayload {
        previous_status: previous.status,
        resulting_status,
    };
    let (payload, checksum) = encode_payload(&audit_payload)?;
    sqlx::query(
        "INSERT INTO audit_event \
         (tenant_id, audit_event_id, occurred_at, actor_id, course_id, action, \
          target_kind, target_id, payload, payload_sha256) \
         VALUES ($1, $2, transaction_timestamp(), $3, $4, $5, \
                 'question_attempt', $6, $7, $8)",
    )
    .bind(tenant.as_uuid())
    .bind(action_id.as_uuid())
    .bind(actor.as_uuid())
    .bind(assignment.course_id.as_uuid())
    .bind(action.audit_name())
    .bind(attempt_id.as_uuid())
    .bind(payload)
    .bind(checksum)
    .execute(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    Ok(AttemptSupportRecord {
        tenant,
        action: action_id,
        actor,
        attempt: attempt_id,
        kind: action,
        previous_status: previous.status,
        resulting_status,
        occurred_at,
    })
}

#[cfg(feature = "postgres")]
async fn issue_or_resume_question_attempt(
    transaction: &mut Transaction<'_, Postgres>,
    context: TenantContext,
    command: IssueQuestionAttemptCommand,
) -> Result<QuestionAttempt, StoreError> {
    let tenant = context.tenant_id();
    let run = load_run_for_update(transaction, tenant, command.run).await?;
    if run.completed_at.is_some() || run.score.is_some() {
        return Err(StoreError::InvalidRecord(
            "a completed run cannot issue another question".to_string(),
        ));
    }
    let enrollment = load_enrollment_for_update(transaction, tenant, run.enrollment).await?;
    if enrollment.user != command.actor {
        return Err(StoreError::Forbidden);
    }
    lock_postgres_assignment_policy(transaction, tenant, enrollment.assignment).await?;
    let assignment_guard =
        load_assignment_for_share(transaction, tenant, enrollment.assignment).await?;
    let resolved_assignment_timing = load_postgres_resolved_assignment_policy(
        transaction,
        tenant,
        enrollment.assignment,
        &enrollment,
        None,
    )
    .await?;
    validate_postgres_assignment_position(transaction, tenant, &command).await?;
    let assignment_position = i32::try_from(command.assignment_position)
        .map_err(|_| StoreError::InvalidRecord("assignment position is too large".to_string()))?;
    if let Some(prefetched) = command.prefetched.as_ref()
        && (prefetched.tenant != tenant
            || prefetched.run != command.run
            || command.predecessor_submission != Some(prefetched.predecessor)
            || prefetched.assignment_position != command.assignment_position
            || prefetched.problem != command.problem
            || prefetched.question_version != command.question_version)
    {
        return Err(StoreError::Conflict);
    }

    let unresolved = sqlx::query(
        "SELECT qa.payload, qa.payload_sha256, \
                qa.attempt_status AS current_attempt_status, \
                floor(extract(epoch FROM qa.submitted_at) * 1000)::bigint AS current_submitted_at, \
                floor(extract(epoch FROM timing.effective_deadline) * 1000)::bigint \
                    AS current_deadline_at \
         FROM question_attempt AS qa \
         LEFT JOIN submission_idempotency AS si \
           ON si.tenant_id = qa.tenant_id AND si.attempt_id = qa.attempt_id \
         LEFT JOIN attempt_timing_current AS timing \
           ON timing.tenant_id = qa.tenant_id AND timing.attempt_id = qa.attempt_id \
         WHERE qa.tenant_id = $1 AND qa.run_id = $2 \
           AND qa.attempt_status = 'in_progress' AND si.attempt_id IS NULL \
         ORDER BY qa.occurred_at DESC, qa.attempt_id::text DESC LIMIT 1",
    )
    .bind(tenant.as_uuid())
    .bind(run.id.as_uuid())
    .fetch_optional(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    if let Some(row) = unresolved {
        let active = decode_current_attempt_row(&row)?;
        if active.assignment_position == command.assignment_position {
            if let Some(predecessor) = command.predecessor_submission {
                // Converging healers must attach the already-issued active
                // attempt to the durable predecessor receipt before return.
                // Select the persisted timestamp rather than the public
                // millisecond timer value so the partitioned FK is exact.
                let inserted = sqlx::query(
                    "INSERT INTO submission_next_attempt \
                     (tenant_id, predecessor_attempt_id, next_attempt_id, next_attempt_occurred_at) \
                     SELECT $1, $2, $3, active.occurred_at \
                     FROM question_attempt AS active \
                     JOIN question_attempt AS predecessor_attempt \
                       ON predecessor_attempt.tenant_id = active.tenant_id \
                      AND predecessor_attempt.run_id = active.run_id \
                     JOIN submission_idempotency AS submitted \
                       ON submitted.tenant_id = predecessor_attempt.tenant_id \
                      AND submitted.attempt_id = predecessor_attempt.attempt_id \
                     WHERE active.tenant_id = $1 AND active.attempt_id = $3 \
                       AND predecessor_attempt.attempt_id = $2 \
                     ON CONFLICT (tenant_id, predecessor_attempt_id) DO NOTHING",
                )
                .bind(tenant.as_uuid())
                .bind(predecessor.as_uuid())
                .bind(active.id.as_uuid())
                .execute(&mut **transaction)
                .await
                .map_err(map_sqlx_error)?;
                if inserted.rows_affected() == 0 {
                    let existing: Option<Option<Uuid>> = sqlx::query_scalar(
                        "SELECT next_attempt_id FROM submission_next_attempt \
                         WHERE tenant_id = $1 AND predecessor_attempt_id = $2 FOR UPDATE",
                    )
                    .bind(tenant.as_uuid())
                    .bind(predecessor.as_uuid())
                    .fetch_optional(&mut **transaction)
                    .await
                    .map_err(map_sqlx_error)?
                    .flatten();
                    if existing != Some(Some(active.id.as_uuid())) {
                        return Err(StoreError::Conflict);
                    }
                }
            }
            return Ok(active);
        }
        return Err(StoreError::InvalidRecord(
            "another question attempt is already active in this run".to_string(),
        ));
    }
    let prefetched = command.prefetched.as_ref();
    if let Some(prefetched) = prefetched {
        if prefetched.tenant != tenant
            || prefetched.run != command.run
            || command.predecessor_submission != Some(prefetched.predecessor)
            || prefetched.assignment_position != command.assignment_position
            || prefetched.problem != command.problem
            || prefetched.question_version != command.question_version
        {
            return Err(StoreError::Conflict);
        }
        let row = sqlx::query(
            "SELECT payload, payload_sha256 FROM question_prefetch \
             WHERE tenant_id = $1 AND run_id = $2 AND predecessor_attempt_id = $3 AND assignment_position = $4 FOR UPDATE",
        )
        .bind(tenant.as_uuid())
        .bind(command.run.as_uuid())
        .bind(prefetched.predecessor.as_uuid())
        .bind(assignment_position)
        .fetch_optional(&mut **transaction)
        .await
        .map_err(map_sqlx_error)?
        .ok_or(StoreError::Conflict)?;
        let stored: PrefetchedQuestion = decode_payload_row(&row)?;
        let predecessor_submitted: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM submission_idempotency WHERE tenant_id = $1 AND attempt_id = $2)",
        )
        .bind(tenant.as_uuid())
        .bind(prefetched.predecessor.as_uuid())
        .fetch_one(&mut **transaction)
        .await
        .map_err(map_sqlx_error)?;
        if stored != *prefetched || !predecessor_submitted {
            return Err(StoreError::Conflict);
        }
    }
    let latest_submission = sqlx::query(
        "SELECT si.payload, si.payload_sha256 FROM question_attempt AS qa \
         JOIN submission_idempotency AS si \
           ON si.tenant_id = qa.tenant_id AND si.attempt_id = qa.attempt_id \
         WHERE qa.tenant_id = $1 AND qa.run_id = $2 AND qa.assignment_position = $3 \
           AND qa.attempt_status NOT IN ('cleared', 'exempt') \
         ORDER BY si.submitted_at DESC, qa.attempt_id::text DESC LIMIT 1",
    )
    .bind(tenant.as_uuid())
    .bind(run.id.as_uuid())
    .bind(assignment_position)
    .fetch_optional(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    if let Some(row) = latest_submission {
        let latest: QuestionAttempt = decode_payload_row(&row)?;
        if latest.result.is_some_and(|result| result.correct) {
            return Err(StoreError::InvalidRecord(
                "a correct question position cannot be retried".to_string(),
            ));
        }
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
    if parameter_hash.trim().is_empty() || provenance.rendered_question_sha256.trim().is_empty() {
        return Err(StoreError::InvalidRecord(
            "issued attempt hashes must not be empty".to_string(),
        ));
    }
    let question =
        load_published_record(transaction, command.problem, command.question_version).await?;
    let issued_at = database_timestamp(transaction).await?;
    let authored_timer = issued_timer(issued_at, &run, question.question.timing_policy)?;
    let authored_grace_seconds = timing_policy_grace_seconds(question.question.timing_policy);
    let ResolvedPostgresAttemptTiming {
        effective_deadline,
        effective_grace_seconds,
        auto_submit_at,
        resolution_kind,
    } = resolved_postgres_attempt_timing(
        resolved_assignment_timing.policy,
        &run,
        authored_timer.deadline,
        authored_grace_seconds,
    )?;
    if effective_deadline.is_some_and(|deadline| deadline < issued_at)
        || auto_submit_at.is_some_and(|deadline| deadline <= issued_at)
    {
        return Err(StoreError::TimedOut);
    }
    let timer = AttemptTimerRecord {
        deadline: effective_deadline,
        ..authored_timer
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
    let duplicate: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM question_attempt \
         WHERE tenant_id = $1 AND attempt_id = $2)",
    )
    .bind(tenant.as_uuid())
    .bind(attempt.id.as_uuid())
    .fetch_one(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    if duplicate {
        return Err(StoreError::AlreadyExists);
    }
    let (payload, checksum) = encode_payload(&attempt)?;
    sqlx::query(
        "INSERT INTO question_attempt \
         (tenant_id, attempt_id, run_id, problem_id, version_id, assignment_position, \
          occurred_at, payload, payload_sha256) \
         VALUES ($1, $2, $3, $4, $5, $6, transaction_timestamp(), $7, $8)",
    )
    .bind(tenant.as_uuid())
    .bind(attempt.id.as_uuid())
    .bind(attempt.run.as_uuid())
    .bind(attempt.problem.as_uuid())
    .bind(attempt.question_version.as_uuid())
    .bind(assignment_position)
    .bind(payload)
    .bind(checksum)
    .execute(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    let timing_generation = 1_u64;
    let timing_job = if let Some(available_at) = auto_submit_at {
        let job = JobId::generate()?;
        let payload = serde_json::to_value(JobPayload::AutoSubmitAttempt {
            attempt: attempt.id,
            timing_generation,
        })
        .map_err(|error| StoreError::InvalidRecord(error.to_string()))?;
        sqlx::query(
            "INSERT INTO worker_job \
             (job_id, tenant_id, payload, state, available_at, max_attempts) \
             VALUES ($1, $2, $3, 'ready', \
                TIMESTAMPTZ 'epoch' + $4::bigint * INTERVAL '1 millisecond', 10)",
        )
        .bind(job.as_uuid())
        .bind(tenant.as_uuid())
        .bind(payload)
        .bind(available_at.as_unix_millis())
        .execute(&mut **transaction)
        .await
        .map_err(map_sqlx_error)?;
        Some(job)
    } else {
        None
    };
    let timing_inserted = sqlx::query(
        "INSERT INTO attempt_timing_current \
         (tenant_id, attempt_id, attempt_occurred_at, assignment_id, course_id, \
          authored_deadline, authored_grace_seconds, effective_deadline, \
          effective_grace_seconds, auto_submit_at, resolution_kind, resolved_visible, \
          resolved_available_at, resolved_due_at, resolved_closes_at, \
          resolved_late_submission_policy, resolved_time_limit_seconds, \
          resolved_attempt_limit, resolution_sources, timing_generation, job_id) \
         SELECT $1, $2, attempt.occurred_at, $3, $4, \
                TIMESTAMPTZ 'epoch' + $5::bigint * INTERVAL '1 millisecond', $6, \
                TIMESTAMPTZ 'epoch' + $7::bigint * INTERVAL '1 millisecond', $8, \
                TIMESTAMPTZ 'epoch' + $9::bigint * INTERVAL '1 millisecond', $10, $11, \
                TIMESTAMPTZ 'epoch' + $12::bigint * INTERVAL '1 millisecond', \
                TIMESTAMPTZ 'epoch' + $13::bigint * INTERVAL '1 millisecond', \
                TIMESTAMPTZ 'epoch' + $14::bigint * INTERVAL '1 millisecond', \
                $15, $16, $17, $18, $19, $20 \
           FROM question_attempt AS attempt \
          WHERE attempt.tenant_id = $1 AND attempt.attempt_id = $2",
    )
    .bind(tenant.as_uuid())
    .bind(attempt.id.as_uuid())
    .bind(assignment_guard.id.as_uuid())
    .bind(assignment_guard.course_id.as_uuid())
    .bind(authored_timer.deadline.map(|value| value.as_unix_millis()))
    .bind(i64::from(authored_grace_seconds))
    .bind(effective_deadline.map(|value| value.as_unix_millis()))
    .bind(i64::from(effective_grace_seconds))
    .bind(auto_submit_at.map(|value| value.as_unix_millis()))
    .bind(resolution_kind)
    .bind(resolved_assignment_timing.policy.visible)
    .bind(
        resolved_assignment_timing
            .policy
            .available_at
            .map(|value| value.as_unix_millis()),
    )
    .bind(
        resolved_assignment_timing
            .policy
            .due_at
            .map(|value| value.as_unix_millis()),
    )
    .bind(
        resolved_assignment_timing
            .policy
            .closes_at
            .map(|value| value.as_unix_millis()),
    )
    .bind(late_submission_policy_name(
        resolved_assignment_timing.policy.late_submission,
    ))
    .bind(
        resolved_assignment_timing
            .policy
            .time_limit_seconds
            .map(i64::from),
    )
    .bind(
        resolved_assignment_timing
            .policy
            .attempt_limit
            .map(i64::from),
    )
    .bind(
        serde_json::to_value(&resolved_assignment_timing.contributors)
            .map_err(|error| StoreError::InvalidRecord(error.to_string()))?,
    )
    .bind(i64::try_from(timing_generation).expect("initial generation fits"))
    .bind(timing_job.map(JobId::as_uuid))
    .execute(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    if timing_inserted.rows_affected() != 1 {
        return Err(StoreError::Conflict);
    }
    if let Some(prefetched) = prefetched {
        sqlx::query(
            "DELETE FROM question_prefetch WHERE tenant_id = $1 AND run_id = $2 AND predecessor_attempt_id = $3 AND assignment_position = $4",
        )
        .bind(tenant.as_uuid())
        .bind(command.run.as_uuid())
        .bind(prefetched.predecessor.as_uuid())
        .bind(assignment_position)
        .execute(&mut **transaction)
        .await
        .map_err(map_sqlx_error)?;
    }
    if let Some(predecessor) = command.predecessor_submission {
        if load_attempt_for_external_update(transaction, tenant, predecessor)
            .await?
            .run
            != command.run
        {
            return Err(StoreError::Conflict);
        }
        let submitted: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM submission_idempotency WHERE tenant_id = $1 AND attempt_id = $2)",
        )
        .bind(tenant.as_uuid())
        .bind(predecessor.as_uuid())
        .fetch_one(&mut **transaction)
        .await
        .map_err(map_sqlx_error)?;
        if !submitted {
            return Err(StoreError::Conflict);
        }
        let existing: Option<Option<Uuid>> = sqlx::query_scalar(
            "SELECT next_attempt_id FROM submission_next_attempt WHERE tenant_id = $1 AND predecessor_attempt_id = $2 FOR UPDATE",
        )
        .bind(tenant.as_uuid())
        .bind(predecessor.as_uuid())
        .fetch_optional(&mut **transaction)
        .await
        .map_err(map_sqlx_error)?;
        match existing {
            Some(Some(existing)) if existing != attempt.id.as_uuid() => {
                return Err(StoreError::Conflict);
            }
            Some(None) => return Err(StoreError::Conflict),
            Some(Some(_)) => {}
            None => {
                sqlx::query(
                    "INSERT INTO submission_next_attempt (tenant_id, predecessor_attempt_id, next_attempt_id, next_attempt_occurred_at) VALUES ($1, $2, $3, transaction_timestamp())",
                )
                .bind(tenant.as_uuid())
                .bind(predecessor.as_uuid())
                .bind(attempt.id.as_uuid())
                .execute(&mut **transaction)
                .await
                .map_err(map_sqlx_error)?;
            }
        }
    }
    Ok(attempt)
}

#[cfg(feature = "postgres")]
async fn load_attempt_for_external_update(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    attempt: QuestionAttemptId,
) -> Result<QuestionAttempt, StoreError> {
    let row = sqlx::query("SELECT attempt.payload, attempt.payload_sha256, \
            attempt.attempt_status AS current_attempt_status, \
            floor(extract(epoch FROM attempt.submitted_at) * 1000)::bigint AS current_submitted_at, \
            floor(extract(epoch FROM timing.effective_deadline) * 1000)::bigint AS current_deadline_at \
        FROM question_attempt AS attempt \
        LEFT JOIN attempt_timing_current AS timing \
          ON timing.tenant_id = attempt.tenant_id AND timing.attempt_id = attempt.attempt_id \
        WHERE attempt.tenant_id = $1 AND attempt.attempt_id = $2 \
        ORDER BY attempt.occurred_at LIMIT 1 FOR UPDATE OF attempt")
        .bind(tenant.as_uuid()).bind(attempt.as_uuid()).fetch_optional(&mut **transaction).await.map_err(map_sqlx_error)?.ok_or(StoreError::NotFound)?;
    decode_current_attempt_row(&row)
}

#[cfg(feature = "postgres")]
fn postgres_validate_external_command(
    command: &BeginExternalToolGradeCommand,
) -> Result<(), StoreError> {
    postgres_validate_external_response(&command.response, &command.binding)?;
    if command.lease_millis == 0 || command.lease_millis > 300_000 {
        return Err(StoreError::InvalidRecord(
            "external-tool lease must be 1 to 300000 milliseconds".to_string(),
        ));
    }
    Ok(())
}

#[cfg(feature = "postgres")]
fn postgres_validate_external_binding(
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

#[cfg(feature = "postgres")]
fn postgres_validate_external_response(
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

#[cfg(feature = "postgres")]
async fn validate_and_lock_external_launch(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    actor: UserId,
    attempt: QuestionAttemptId,
    binding: &ExternalToolBinding,
    proof: &ExternalToolLaunchProof,
) -> Result<(), StoreError> {
    let row = sqlx::query(
        "SELECT provider, problem_id, version_id, seed, source_object_id, source_sha256, integration_profile, response_sha256, token_sha256 \
         FROM external_tool_launch_session \
         WHERE tenant_id = $1 AND launch_session_id = $2 AND attempt_id = $3 AND actor_id = $4 \
           AND revoked_at IS NULL AND expires_at > transaction_timestamp() FOR UPDATE",
    )
    .bind(tenant.as_uuid())
    .bind(proof.session_id)
    .bind(attempt.as_uuid())
    .bind(actor.as_uuid())
    .fetch_optional(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?
    .ok_or(StoreError::Conflict)?;
    let stored = postgres_external_binding(&row)?;
    let token_hash: Vec<u8> = row.try_get("token_sha256").map_err(map_sqlx_error)?;
    if !postgres_binding_matches(&stored, binding)
        || stored.response_sha256 != binding.response_sha256
        || token_hash.as_slice() != proof.token.hash().as_bytes()
    {
        return Err(StoreError::Conflict);
    }
    Ok(())
}

#[cfg(feature = "postgres")]
async fn revoke_locked_external_launch(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    session_id: Uuid,
) -> Result<(), StoreError> {
    let changed = sqlx::query(
        "UPDATE external_tool_launch_session SET revoked_at = transaction_timestamp() \
         WHERE tenant_id = $1 AND launch_session_id = $2 AND revoked_at IS NULL",
    )
    .bind(tenant.as_uuid())
    .bind(session_id)
    .execute(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    if changed.rows_affected() != 1 {
        return Err(StoreError::Conflict);
    }
    Ok(())
}

#[cfg(feature = "postgres")]
fn postgres_external_binding(row: &PgRow) -> Result<ExternalToolBinding, StoreError> {
    let response: Vec<u8> = row.try_get("response_sha256").map_err(map_sqlx_error)?;
    let response: [u8; 32] = response.try_into().map_err(|_| {
        StoreError::InvalidRecord("stored external response checksum is malformed".to_string())
    })?;
    Ok(ExternalToolBinding {
        provider: row.try_get("provider").map_err(map_sqlx_error)?,
        problem: ProblemId::from_uuid(row.try_get("problem_id").map_err(map_sqlx_error)?),
        version: VersionId::from_uuid(row.try_get("version_id").map_err(map_sqlx_error)?),
        seed: row.try_get::<i64, _>("seed").map_err(map_sqlx_error)? as u64,
        source_object: ObjectId::from_uuid(
            row.try_get("source_object_id").map_err(map_sqlx_error)?,
        ),
        source_sha256: row.try_get("source_sha256").map_err(map_sqlx_error)?,
        integration_profile: row.try_get("integration_profile").map_err(map_sqlx_error)?,
        response_sha256: Sha256Digest::from_bytes(response),
    })
}

#[cfg(feature = "postgres")]
fn postgres_binding_matches(stored: &ExternalToolBinding, requested: &ExternalToolBinding) -> bool {
    stored.provider == requested.provider
        && stored.problem == requested.problem
        && stored.version == requested.version
        && stored.seed == requested.seed
        && stored.source_object == requested.source_object
        && stored.source_sha256 == requested.source_sha256
        && stored.integration_profile == requested.integration_profile
}

#[cfg(feature = "postgres")]
async fn submit_question_attempt(
    transaction: &mut Transaction<'_, Postgres>,
    context: TenantContext,
    command: SubmitQuestionAttemptCommand,
) -> Result<SubmissionRecord, StoreError> {
    let tenant = context.tenant_id();
    let attempt_row = sqlx::query(
        "SELECT attempt.payload, attempt.payload_sha256, \
                attempt.attempt_status AS current_attempt_status, \
                floor(extract(epoch FROM attempt.submitted_at) * 1000)::bigint \
                    AS current_submitted_at, \
                floor(extract(epoch FROM timing.effective_deadline) * 1000)::bigint \
                    AS current_deadline_at, timing.effective_grace_seconds \
         FROM question_attempt AS attempt \
         LEFT JOIN attempt_timing_current AS timing \
           ON timing.tenant_id = attempt.tenant_id AND timing.attempt_id = attempt.attempt_id \
         WHERE attempt.tenant_id = $1 AND attempt.attempt_id = $2 \
         ORDER BY attempt.occurred_at LIMIT 1 FOR UPDATE OF attempt",
    )
    .bind(tenant.as_uuid())
    .bind(command.attempt.as_uuid())
    .fetch_optional(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?
    .ok_or(StoreError::NotFound)?;
    let base = decode_current_attempt_row(&attempt_row)?;
    require_attempt_owner(transaction, tenant, base.id, command.actor).await?;
    if let Some(replay) = load_submission_replay(
        transaction,
        tenant,
        base.id,
        &command.response,
        &command.idempotency_key,
    )
    .await?
    {
        return Ok(replay);
    }
    if base.status != AttemptStatus::InProgress {
        return Err(StoreError::Conflict);
    }
    let feedback = private_feedback_record(command.feedback.clone())?;

    let mut run = load_run_for_update(transaction, tenant, base.run).await?;
    if run.completed_at.is_some() || run.score.is_some() {
        return Err(StoreError::Conflict);
    }
    let mut enrollment = load_enrollment_for_update(transaction, tenant, run.enrollment).await?;
    let assignment = load_assignment_for_share(transaction, tenant, enrollment.assignment).await?;
    let question = load_published_record(transaction, base.problem, base.question_version).await?;
    crate::validate_attempt_result(command.result)?;
    let submitted_at = database_timestamp(transaction).await?;
    let mut submitted = base;
    submitted.response = Some(command.response.clone());
    submitted.status = AttemptStatus::Submitted;
    submitted.result = Some(command.result);
    submitted.timer.submitted_at = Some(submitted_at);
    let effective_grace = attempt_row
        .try_get::<Option<i32>, _>("effective_grace_seconds")
        .map_err(map_sqlx_error)?;
    let effective_policy = match effective_grace {
        Some(grace_seconds) if submitted.timer.deadline.is_some() => TimingPolicy::PerQuestion {
            seconds: 1,
            grace_seconds: u32::try_from(grace_seconds).map_err(|_| {
                StoreError::Unavailable("stored effective grace is invalid".to_string())
            })?,
        },
        Some(_) => TimingPolicy::Untimed,
        None => question.question.timing_policy,
    };
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

    let previous = load_summary_for_update(transaction, tenant, enrollment.id).await?;
    let mut next = project_summary(
        &previous,
        domain::scoring::RunTransition::QuestionAttemptRecorded { at: submitted_at },
        grade_policy(&assignment),
    )?;
    let rows = sqlx::query(
        "SELECT COALESCE(si.payload, qa.payload) AS payload, \
                COALESCE(si.payload_sha256, qa.payload_sha256) AS payload_sha256, \
                qa.attempt_status AS current_attempt_status, \
                floor(extract(epoch FROM qa.submitted_at) * 1000)::bigint \
                    AS current_submitted_at, \
                floor(extract(epoch FROM timing.effective_deadline) * 1000)::bigint \
                    AS current_deadline_at \
         FROM question_attempt AS qa \
         LEFT JOIN submission_idempotency AS si \
           ON si.tenant_id = qa.tenant_id AND si.attempt_id = qa.attempt_id \
         LEFT JOIN attempt_timing_current AS timing \
           ON timing.tenant_id = qa.tenant_id AND timing.attempt_id = qa.attempt_id \
         WHERE qa.tenant_id = $1 AND qa.run_id = $2",
    )
    .bind(tenant.as_uuid())
    .bind(run.id.as_uuid())
    .fetch_all(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    let attempts = rows
        .iter()
        .map(decode_current_attempt_row)
        .collect::<Result<Vec<QuestionAttempt>, StoreError>>()?;
    let run_items = load_assignment_run_items(transaction, tenant, run.id).await?;
    let questions = current_run_questions(&assignment, &run_items, &attempts, &submitted)?;
    let results = questions
        .iter()
        .map(|question| question.map(|question| question.result))
        .collect::<Vec<_>>();
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
            let attempts = attempts
                .iter()
                .map(|attempt| {
                    if attempt.id == submitted.id {
                        submitted.clone()
                    } else {
                        attempt.clone()
                    }
                })
                .collect::<Vec<_>>();
            statistics_contributions = Some(derive_statistics_contributions(
                &run_items, &results, &attempts,
            )?);
        }
    }
    let (attempt_payload, attempt_checksum) = encode_payload(&submitted)?;
    let feedback_columns = encode_feedback_columns(feedback.content())?;
    sqlx::query(
        "INSERT INTO attempt_feedback \
         (tenant_id, attempt_id, hint, correct_response, rationale, content_sha256) \
         VALUES ($1, $2, $3, $4, $5, $6)",
    )
    .bind(tenant.as_uuid())
    .bind(submitted.id.as_uuid())
    .bind(feedback_columns.hint)
    .bind(feedback_columns.correct_response)
    .bind(feedback_columns.rationale)
    .bind(feedback.content_sha256().to_string())
    .execute(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    let (_, response_checksum) = encode_payload(&command.response)?;
    sqlx::query(
        "INSERT INTO submission_idempotency \
         (tenant_id, attempt_id, idempotency_key, response_sha256, submitted_at, \
          payload, payload_sha256) \
         VALUES ($1, $2, $3, $4, transaction_timestamp(), $5, $6)",
    )
    .bind(tenant.as_uuid())
    .bind(submitted.id.as_uuid())
    .bind(command.idempotency_key.as_str())
    .bind(response_checksum)
    .bind(attempt_payload.clone())
    .bind(attempt_checksum.clone())
    .execute(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    sqlx::query(
        "UPDATE question_attempt SET attempt_status = 'submitted', \
             submitted_at = transaction_timestamp() \
         WHERE tenant_id = $1 AND attempt_id = $2",
    )
    .bind(tenant.as_uuid())
    .bind(submitted.id.as_uuid())
    .execute(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    cancel_postgres_attempt_timing_job(transaction, tenant, submitted.id).await?;
    let (response_payload, response_checksum) = encode_payload(&command.response)?;
    sqlx::query(
        "INSERT INTO submission \
         (tenant_id, submission_id, attempt_id, idempotency_key, occurred_at, payload, payload_sha256) \
         VALUES ($1, $2, $2, $3, transaction_timestamp(), $4, $5)",
    )
    .bind(tenant.as_uuid())
    .bind(submitted.id.as_uuid())
    .bind(command.idempotency_key.as_str())
    .bind(response_payload)
    .bind(response_checksum)
    .execute(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    let (grade_payload, grade_checksum) = encode_payload(&command.result)?;
    let (assignment_item, credit_fraction, earned_points, possible_points) =
        current_attempt_score(transaction, &assignment, &submitted, command.result).await?;
    sqlx::query(
        "INSERT INTO submission_evaluation \
         (tenant_id, attempt_id, submission_id, credit_fraction, correct, grading_status, \
          payload, payload_sha256) \
         VALUES ($1, $2, $2, $3::numeric, $4, 'graded', $5, $6) \
         ON CONFLICT (tenant_id, attempt_id) DO UPDATE \
         SET submission_id = EXCLUDED.submission_id, \
             credit_fraction = EXCLUDED.credit_fraction, correct = EXCLUDED.correct, \
             grading_status = EXCLUDED.grading_status, payload = EXCLUDED.payload, \
             payload_sha256 = EXCLUDED.payload_sha256, \
             evaluated_at = transaction_timestamp()",
    )
    .bind(tenant.as_uuid())
    .bind(submitted.id.as_uuid())
    .bind(credit_fraction)
    .bind(command.result.correct)
    .bind(grade_payload)
    .bind(grade_checksum)
    .execute(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    let scored = sqlx::query(
        "INSERT INTO attempt_score_current \
         (tenant_id, attempt_id, assignment_id, assignment_item_id, scoring_generation, \
          earned_points, possible_points, course_id) \
         SELECT $1, $2, a.assignment_id, $3, a.scoring_generation, $4::numeric, $5::numeric, \
                a.course_id \
           FROM assignment a WHERE a.tenant_id = $1 AND a.assignment_id = $6 \
         ON CONFLICT (tenant_id, attempt_id) DO UPDATE \
         SET assignment_id = EXCLUDED.assignment_id, \
             assignment_item_id = EXCLUDED.assignment_item_id, \
             scoring_generation = EXCLUDED.scoring_generation, \
             earned_points = EXCLUDED.earned_points, \
             possible_points = EXCLUDED.possible_points, \
             course_id = EXCLUDED.course_id, calculated_at = transaction_timestamp()",
    )
    .bind(tenant.as_uuid())
    .bind(submitted.id.as_uuid())
    .bind(assignment_item.as_uuid())
    .bind(earned_points)
    .bind(possible_points)
    .bind(assignment.id.as_uuid())
    .execute(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    if scored.rows_affected() != 1 {
        return Err(StoreError::Conflict);
    }

    if run.completed_at.is_some() {
        let (run_payload, run_checksum) = encode_payload(&run)?;
        let (enrollment_payload, enrollment_checksum) = encode_payload(&enrollment)?;
        sqlx::query(
            "UPDATE assignment_run SET completed_at = transaction_timestamp(), \
             payload = $3, payload_sha256 = $4 WHERE tenant_id = $1 AND run_id = $2",
        )
        .bind(tenant.as_uuid())
        .bind(run.id.as_uuid())
        .bind(run_payload)
        .bind(run_checksum)
        .execute(&mut **transaction)
        .await
        .map_err(map_sqlx_error)?;
        sqlx::query(
            "UPDATE enrollment SET payload = $3, payload_sha256 = $4 \
             WHERE tenant_id = $1 AND enrollment_id = $2",
        )
        .bind(tenant.as_uuid())
        .bind(enrollment.id.as_uuid())
        .bind(enrollment_payload)
        .bind(enrollment_checksum)
        .execute(&mut **transaction)
        .await
        .map_err(map_sqlx_error)?;
    }
    store_summary(transaction, &next).await?;
    if let Some(contributions) = &statistics_contributions {
        for contribution in contributions {
            let recorded: bool = sqlx::query_scalar(
                "SELECT ple_record_question_statistics( \
                    $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)",
            )
            .bind(tenant.as_uuid())
            .bind(enrollment.id.as_uuid())
            .bind(run.id.as_uuid())
            .bind(submitted.id.as_uuid())
            .bind(contribution.reference.problem.as_uuid())
            .bind(contribution.reference.version.as_uuid())
            .bind(contribution.observation.normalized_score())
            .bind(
                i64::try_from(contribution.observation.attempts()).map_err(|_| {
                    StoreError::InvalidRecord("statistics attempt count is too large".to_string())
                })?,
            )
            .bind(
                i64::try_from(contribution.observation.duration_seconds()).map_err(|_| {
                    StoreError::InvalidRecord("statistics duration is too large".to_string())
                })?,
            )
            .bind(contribution.observation.rest_score())
            .bind(contribution.checksum.as_bytes().to_vec())
            .fetch_one(&mut **transaction)
            .await
            .map_err(map_sqlx_error)?;
            if !recorded {
                return Err(StoreError::Conflict);
            }
        }
    }
    let (receipt_run, receipt_run_sha256) = encode_payload(&run)?;
    let (receipt_summary, receipt_summary_sha256) = encode_payload(&next)?;
    sqlx::query(
        "INSERT INTO submission_receipt_snapshot \
         (tenant_id, attempt_id, run_payload, run_payload_sha256, summary_payload, summary_payload_sha256) \
         VALUES ($1, $2, $3, $4, $5, $6)",
    )
    .bind(tenant.as_uuid())
    .bind(submitted.id.as_uuid())
    .bind(receipt_run)
    .bind(receipt_run_sha256)
    .bind(receipt_summary)
    .bind(receipt_summary_sha256)
    .execute(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    Ok(SubmissionRecord {
        attempt: submitted,
        run,
        summary: next,
        feedback,
    })
}

#[cfg(feature = "postgres")]
async fn load_submission_replay(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    attempt: QuestionAttemptId,
    response: &StudentResponse,
    idempotency_key: &SubmissionIdempotencyKey,
) -> Result<Option<SubmissionRecord>, StoreError> {
    let row = sqlx::query(
        "SELECT idempotency_key, response_sha256, payload, payload_sha256 \
         FROM submission_idempotency WHERE tenant_id = $1 AND attempt_id = $2",
    )
    .bind(tenant.as_uuid())
    .bind(attempt.as_uuid())
    .fetch_optional(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    let Some(row) = row else {
        return Ok(None);
    };
    let stored_key: String = row.try_get("idempotency_key").map_err(map_sqlx_error)?;
    let stored_response_checksum: String =
        row.try_get("response_sha256").map_err(map_sqlx_error)?;
    let (_, response_checksum) = encode_payload(response)?;
    if stored_key != idempotency_key.as_str() || stored_response_checksum != response_checksum {
        return Err(StoreError::Conflict);
    }
    let submitted: QuestionAttempt = decode_payload_row(&row)?;
    let feedback = load_attempt_feedback(transaction, tenant, attempt).await?;
    let Some((run, summary)) =
        load_submission_receipt_snapshot(transaction, tenant, attempt).await?
    else {
        // A pre-snapshot row predates this migration. There is no honest way
        // to recreate its receipt-time state, so retain the old current-state
        // fallback only for that legacy data; new writes never take this path.
        let run = load_run_for_update(transaction, tenant, submitted.run).await?;
        let enrollment = load_enrollment_for_update(transaction, tenant, run.enrollment).await?;
        let summary = load_summary_for_update(transaction, tenant, enrollment.id).await?;
        return Ok(Some(SubmissionRecord {
            attempt: submitted,
            run,
            summary,
            feedback,
        }));
    };
    Ok(Some(SubmissionRecord {
        attempt: submitted,
        run,
        summary,
        feedback,
    }))
}

#[cfg(feature = "postgres")]
async fn load_submission_receipt_snapshot(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    attempt: QuestionAttemptId,
) -> Result<Option<(AssignmentRun, StudentAssignmentSummary)>, StoreError> {
    let row = sqlx::query(
        "SELECT run_payload AS payload, run_payload_sha256 AS payload_sha256, \
                summary_payload, summary_payload_sha256 \
         FROM submission_receipt_snapshot WHERE tenant_id = $1 AND attempt_id = $2",
    )
    .bind(tenant.as_uuid())
    .bind(attempt.as_uuid())
    .fetch_optional(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    let Some(row) = row else {
        return Ok(None);
    };
    let run: AssignmentRun = decode_payload_row(&row)?;
    // `decode_payload_row` uses payload/payload_sha256 names, so decode the
    // distinct summary columns explicitly to keep checksum verification exact.
    let summary_payload: Value = row.try_get("summary_payload").map_err(map_sqlx_error)?;
    let summary_sha256: String = row
        .try_get("summary_payload_sha256")
        .map_err(map_sqlx_error)?;
    let summary_bytes = serde_json::to_vec(&summary_payload).map_err(|error| {
        StoreError::InvalidRecord(format!("receipt summary encode failed: {error}"))
    })?;
    if Sha256Digest::compute(&summary_bytes).to_string() != summary_sha256 {
        return Err(StoreError::InvalidRecord(
            "receipt summary checksum mismatch".to_string(),
        ));
    }
    let summary = serde_json::from_value(summary_payload).map_err(|error| {
        StoreError::InvalidRecord(format!("receipt summary decode failed: {error}"))
    })?;
    Ok(Some((run, summary)))
}

#[cfg(feature = "postgres")]
struct FeedbackColumns {
    hint: Option<Value>,
    correct_response: Option<Value>,
    rationale: Option<Value>,
}

#[cfg(feature = "postgres")]
fn encode_feedback_columns(content: &FeedbackContent) -> Result<FeedbackColumns, StoreError> {
    fn field(value: Option<&Vec<ContentBlock>>) -> Result<Option<Value>, StoreError> {
        value
            .map(|blocks| {
                serde_json::to_value(blocks).map_err(|error| {
                    StoreError::InvalidRecord(format!("feedback encoding failed: {error}"))
                })
            })
            .transpose()
    }
    Ok(FeedbackColumns {
        hint: field(content.hint.as_ref())?,
        correct_response: field(content.correct_response.as_ref())?,
        rationale: field(content.rationale.as_ref())?,
    })
}

#[cfg(feature = "postgres")]
async fn load_attempt_feedback(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    attempt: QuestionAttemptId,
) -> Result<AttemptFeedbackRecord, StoreError> {
    let row = sqlx::query(
        "SELECT hint, correct_response, rationale, content_sha256 \
         FROM attempt_feedback WHERE tenant_id = $1 AND attempt_id = $2",
    )
    .bind(tenant.as_uuid())
    .bind(attempt.as_uuid())
    .fetch_optional(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?
    .ok_or_else(|| {
        StoreError::InvalidRecord("submission is missing private feedback".to_string())
    })?;
    fn field(row: &PgRow, name: &str) -> Result<Option<Vec<ContentBlock>>, StoreError> {
        let value: Option<Value> = row.try_get(name).map_err(map_sqlx_error)?;
        value
            .map(|value| {
                serde_json::from_value(value).map_err(|error| {
                    StoreError::InvalidRecord(format!("stored feedback decode failed: {error}"))
                })
            })
            .transpose()
    }
    let content = FeedbackContent {
        hint: field(&row, "hint")?,
        correct_response: field(&row, "correct_response")?,
        rationale: field(&row, "rationale")?,
    };
    let feedback = private_feedback_record(content)?;
    let stored_digest: String = row.try_get("content_sha256").map_err(map_sqlx_error)?;
    if stored_digest != feedback.content_sha256().to_string() {
        return Err(StoreError::InvalidRecord(
            "stored feedback digest mismatch".to_string(),
        ));
    }
    Ok(feedback)
}

#[cfg(feature = "postgres")]
async fn require_attempt_owner(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    attempt: QuestionAttemptId,
    actor: UserId,
) -> Result<(), StoreError> {
    let owns_attempt: bool = sqlx::query_scalar(
        "SELECT EXISTS( \
             SELECT 1 FROM question_attempt AS qa \
             JOIN assignment_run AS ar \
               ON ar.tenant_id = qa.tenant_id AND ar.run_id = qa.run_id \
             JOIN enrollment AS e \
               ON e.tenant_id = ar.tenant_id AND e.enrollment_id = ar.enrollment_id \
             WHERE qa.tenant_id = $1 AND qa.attempt_id = $2 AND e.user_id = $3 \
         )",
    )
    .bind(tenant.as_uuid())
    .bind(attempt.as_uuid())
    .bind(actor.as_uuid())
    .fetch_one(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    if owns_attempt {
        Ok(())
    } else {
        Err(StoreError::NotFound)
    }
}

#[cfg(feature = "postgres")]
async fn postgres_is_course_instructor(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    course: CourseId,
    actor: UserId,
) -> Result<bool, StoreError> {
    sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM course_member \
         WHERE tenant_id = $1 AND course_id = $2 AND user_id = $3 AND role = 'instructor')",
    )
    .bind(tenant.as_uuid())
    .bind(course.as_uuid())
    .bind(actor.as_uuid())
    .fetch_one(&mut **transaction)
    .await
    .map_err(map_sqlx_error)
}

#[cfg(feature = "postgres")]
async fn database_timestamp(
    transaction: &mut Transaction<'_, Postgres>,
) -> Result<ActivityTimestamp, StoreError> {
    let milliseconds: i64 = sqlx::query_scalar(
        "SELECT floor(extract(epoch FROM transaction_timestamp()) * 1000)::bigint",
    )
    .fetch_one(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    Ok(ActivityTimestamp::from_unix_millis(milliseconds))
}

#[cfg(feature = "postgres")]
async fn load_published_record(
    transaction: &mut Transaction<'_, Postgres>,
    problem: ProblemId,
    version: VersionId,
) -> Result<PublishedProblemRecord, StoreError> {
    let row = sqlx::query(
        "SELECT payload, payload_sha256 FROM problem_version_payload \
         WHERE problem_id = $1 AND version_id = $2",
    )
    .bind(problem.as_uuid())
    .bind(version.as_uuid())
    .fetch_optional(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?
    .ok_or(StoreError::NotFound)?;
    decode_payload_row(&row)
}

#[cfg(feature = "postgres")]
async fn validate_postgres_assignment_position(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    command: &IssueQuestionAttemptCommand,
) -> Result<(), StoreError> {
    let position = i32::try_from(command.assignment_position)
        .map_err(|_| StoreError::InvalidRecord("assignment position is too large".to_string()))?;
    let row = sqlx::query(
        "SELECT problem_id, version_id FROM assignment_run_item \
         WHERE tenant_id = $1 AND run_id = $2 AND issued_position = $3",
    )
    .bind(tenant.as_uuid())
    .bind(command.run.as_uuid())
    .bind(position)
    .fetch_optional(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?
    .ok_or_else(|| StoreError::InvalidRecord("question position is outside the run".to_string()))?;
    let problem: Uuid = row.try_get("problem_id").map_err(map_sqlx_error)?;
    let version: Uuid = row.try_get("version_id").map_err(map_sqlx_error)?;
    if problem != command.problem.as_uuid() || version != command.question_version.as_uuid() {
        return Err(StoreError::InvalidRecord(
            "question identity does not match its run position".to_string(),
        ));
    }
    Ok(())
}

#[cfg(feature = "postgres")]
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

#[cfg(feature = "postgres")]
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

#[cfg(feature = "postgres")]
async fn current_attempt_score(
    transaction: &mut Transaction<'_, Postgres>,
    assignment: &AssignmentRecord,
    attempt: &QuestionAttempt,
    result: AttemptResult,
) -> Result<(AssignmentItemId, String, String, String), StoreError> {
    let assignment_item =
        sqlx::query_scalar::<_, Uuid>(
            "SELECT assignment_item_id FROM assignment_run_item \
         WHERE tenant_id = $1 AND run_id = $2 AND issued_position = $3",
        )
        .bind(assignment.tenant.as_uuid())
        .bind(attempt.run.as_uuid())
        .bind(i32::try_from(attempt.assignment_position).map_err(|_| {
            StoreError::InvalidRecord("assignment position is too large".to_string())
        })?)
        .fetch_optional(&mut **transaction)
        .await
        .map_err(map_sqlx_error)?
        .map(AssignmentItemId::from_uuid)
        .ok_or_else(|| {
            StoreError::InvalidRecord(
                "submitted attempt does not resolve to an immutable run item".to_string(),
            )
        })?;
    let credit = result.points_earned / result.points_possible;
    let (earned, possible) =
        crate::current_attempt_points(assignment, assignment_item, attempt.status, result)?;
    Ok((
        assignment_item,
        format!("{credit:.12}"),
        format!("{earned:.4}"),
        format!("{possible:.4}"),
    ))
}

#[cfg(feature = "postgres")]
async fn apply_start_run(
    transaction: &mut Transaction<'_, Postgres>,
    context: TenantContext,
    run: AssignmentRun,
) -> Result<StudentAssignmentSummary, StoreError> {
    ensure_tenant(context, run.tenant)?;
    if run.run_number == 0 || run.completed_at.is_some() || run.score.is_some() {
        return Err(StoreError::InvalidRecord(
            "new run must be one-based and incomplete".to_string(),
        ));
    }
    let tenant = context.tenant_id();
    let duplicate: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM assignment_run WHERE tenant_id = $1 AND run_id = $2)",
    )
    .bind(tenant.as_uuid())
    .bind(run.id.as_uuid())
    .fetch_one(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    if duplicate {
        return Err(StoreError::AlreadyExists);
    }
    let enrollment = load_enrollment_for_update(transaction, tenant, run.enrollment).await?;
    let assignment = load_assignment(transaction, tenant, enrollment.assignment).await?;
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
    let active_exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM assignment_run \
         WHERE tenant_id = $1 AND enrollment_id = $2 AND completed_at IS NULL)",
    )
    .bind(tenant.as_uuid())
    .bind(run.enrollment.as_uuid())
    .fetch_one(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    if active_exists {
        return Err(StoreError::InvalidRecord(
            "an enrollment cannot have two in-progress runs".to_string(),
        ));
    }
    let max_run_number: i64 = sqlx::query_scalar(
        "SELECT COALESCE(MAX(run_number), 0) FROM assignment_run \
         WHERE tenant_id = $1 AND enrollment_id = $2",
    )
    .bind(tenant.as_uuid())
    .bind(run.enrollment.as_uuid())
    .fetch_one(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    let expected_run_number = u32::try_from(max_run_number)
        .map_err(|_| StoreError::InvalidRecord("run number overflow".to_string()))?
        .checked_add(1)
        .ok_or_else(|| StoreError::InvalidRecord("run number overflow".to_string()))?;
    if run.run_number != expected_run_number {
        return Err(StoreError::InvalidRecord(format!(
            "run number must be the next one-based value {expected_run_number}"
        )));
    }
    let previous = load_summary_for_update(transaction, tenant, enrollment.id).await?;
    if !continued_practice_allows_run(&previous, assignment.policies.continued_practice) {
        return Err(StoreError::InvalidRecord(
            "continued-practice policy does not permit another run".to_string(),
        ));
    }
    let transition = ActivityTransition::StartRun { run: run.clone() };
    let next = project_summary(
        &previous,
        summary_transition(&transition),
        grade_policy(&assignment),
    )?;
    let (run_payload, run_checksum) = encode_payload(&run)?;
    sqlx::query(
        "INSERT INTO assignment_run \
         (tenant_id, run_id, enrollment_id, run_number, started_at, \
          payload, payload_sha256) \
         VALUES ($1, $2, $3, $4, to_timestamp($5::double precision / 1000.0), $6, $7)",
    )
    .bind(tenant.as_uuid())
    .bind(run.id.as_uuid())
    .bind(run.enrollment.as_uuid())
    .bind(i64::from(run.run_number))
    .bind(run.started_at.as_unix_millis())
    .bind(run_payload)
    .bind(run_checksum)
    .execute(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    insert_assignment_run_items(transaction, &assignment, run.id).await?;
    store_summary(transaction, &next).await?;
    Ok(next)
}

#[cfg(feature = "postgres")]
async fn insert_assignment_run_items(
    transaction: &mut Transaction<'_, Postgres>,
    assignment: &AssignmentRecord,
    run: RunId,
) -> Result<(), StoreError> {
    for item in select_assignment_run_items(assignment, run)? {
        sqlx::query(
            "INSERT INTO assignment_run_item \
             (tenant_id, run_id, assignment_item_id, source_position, issued_position, \
              problem_id, version_id, selection_group_id, selection_seed) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
        )
        .bind(assignment.tenant.as_uuid())
        .bind(run.as_uuid())
        .bind(item.assignment_item.as_uuid())
        .bind(i32::try_from(item.source_position).map_err(|_| {
            StoreError::InvalidRecord("run source position is too large".to_string())
        })?)
        .bind(i32::try_from(item.issued_position).map_err(|_| {
            StoreError::InvalidRecord("run issued position is too large".to_string())
        })?)
        .bind(item.reference.problem.as_uuid())
        .bind(item.reference.version.as_uuid())
        .bind(item.selection_group.map(|group| group.as_uuid()))
        .bind(
            item.selection_seed
                .map(i64::try_from)
                .transpose()
                .map_err(|_| {
                    StoreError::InvalidRecord("selection seed is too large".to_string())
                })?,
        )
        .execute(&mut **transaction)
        .await
        .map_err(map_sqlx_error)?;
    }
    Ok(())
}

#[cfg(feature = "postgres")]
async fn load_assignment_run_items(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    run: RunId,
) -> Result<Vec<AssignmentRunItem>, StoreError> {
    let rows = sqlx::query(
        "SELECT assignment_item_id, source_position, issued_position, problem_id, \
                version_id, selection_group_id, selection_seed \
         FROM assignment_run_item WHERE tenant_id = $1 AND run_id = $2 \
         ORDER BY issued_position",
    )
    .bind(tenant.as_uuid())
    .bind(run.as_uuid())
    .fetch_all(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    rows.iter()
        .map(|row| {
            let selection_seed: Option<i64> =
                row.try_get("selection_seed").map_err(map_sqlx_error)?;
            Ok(AssignmentRunItem {
                run,
                assignment_item: AssignmentItemId::from_uuid(
                    row.try_get("assignment_item_id").map_err(map_sqlx_error)?,
                ),
                source_position: stored_u32(row, "source_position", "run source position")?,
                issued_position: stored_u32(row, "issued_position", "run issued position")?,
                reference: ProblemVersionRef {
                    problem: ProblemId::from_uuid(
                        row.try_get("problem_id").map_err(map_sqlx_error)?,
                    ),
                    version: VersionId::from_uuid(
                        row.try_get("version_id").map_err(map_sqlx_error)?,
                    ),
                },
                selection_group: row
                    .try_get::<Option<Uuid>, _>("selection_group_id")
                    .map_err(map_sqlx_error)?
                    .map(AssignmentSelectionGroupId::from_uuid),
                selection_seed: selection_seed.map(|seed| seed as u64),
            })
        })
        .collect()
}

#[cfg(feature = "postgres")]
async fn apply_question_attempt(
    transaction: &mut Transaction<'_, Postgres>,
    context: TenantContext,
    attempt: QuestionAttempt,
) -> Result<StudentAssignmentSummary, StoreError> {
    ensure_tenant(context, attempt.tenant)?;
    let tenant = context.tenant_id();
    let duplicate: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM question_attempt \
         WHERE tenant_id = $1 AND attempt_id = $2)",
    )
    .bind(tenant.as_uuid())
    .bind(attempt.id.as_uuid())
    .fetch_one(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    if duplicate {
        return Err(StoreError::AlreadyExists);
    }
    let run = load_run_for_update(transaction, tenant, attempt.run).await?;
    if run.completed_at.is_some() || run.score.is_some() {
        return Err(StoreError::InvalidRecord(
            "question attempts cannot be added to a completed run".to_string(),
        ));
    }
    let enrollment = load_enrollment_for_update(transaction, tenant, run.enrollment).await?;
    let assignment = load_assignment(transaction, tenant, enrollment.assignment).await?;
    let matches_run_item: bool =
        sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM assignment_run_item \
         WHERE tenant_id = $1 AND run_id = $2 AND issued_position = $3 \
           AND problem_id = $4 AND version_id = $5)",
        )
        .bind(tenant.as_uuid())
        .bind(attempt.run.as_uuid())
        .bind(i32::try_from(attempt.assignment_position).map_err(|_| {
            StoreError::InvalidRecord("assignment position is too large".to_string())
        })?)
        .bind(attempt.problem.as_uuid())
        .bind(attempt.question_version.as_uuid())
        .fetch_one(&mut **transaction)
        .await
        .map_err(map_sqlx_error)?;
    if !matches_run_item {
        return Err(StoreError::InvalidRecord(
            "question attempt must match an immutable run item".to_string(),
        ));
    }
    let previous = load_summary_for_update(transaction, tenant, enrollment.id).await?;
    let transition = ActivityTransition::RecordQuestionAttempt {
        attempt: Box::new(attempt.clone()),
    };
    let next = project_summary(
        &previous,
        summary_transition(&transition),
        grade_policy(&assignment),
    )?;
    let occurred_at = attempt
        .timer
        .submitted_at
        .unwrap_or(attempt.timer.issued_at)
        .as_unix_millis();
    let (payload, checksum) = encode_payload(&attempt)?;
    sqlx::query(
        "INSERT INTO question_attempt \
         (tenant_id, attempt_id, run_id, problem_id, version_id, assignment_position, \
          occurred_at, payload, payload_sha256) \
         VALUES ($1, $2, $3, $4, $5, $6, to_timestamp($7::double precision / 1000.0), $8, $9)",
    )
    .bind(tenant.as_uuid())
    .bind(attempt.id.as_uuid())
    .bind(attempt.run.as_uuid())
    .bind(attempt.problem.as_uuid())
    .bind(attempt.question_version.as_uuid())
    .bind(i64::from(attempt.assignment_position))
    .bind(occurred_at)
    .bind(payload)
    .bind(checksum)
    .execute(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    store_summary(transaction, &next).await?;
    Ok(next)
}

#[cfg(feature = "postgres")]
async fn apply_complete_run(
    transaction: &mut Transaction<'_, Postgres>,
    context: TenantContext,
    run_id: RunId,
    score: f64,
    at: question_model::ActivityTimestamp,
) -> Result<StudentAssignmentSummary, StoreError> {
    let tenant = context.tenant_id();
    let mut run = load_run_for_update(transaction, tenant, run_id).await?;
    if run.completed_at.is_some() || run.score.is_some() {
        return Err(StoreError::InvalidRecord(
            "completed run cannot be completed again".to_string(),
        ));
    }
    let mut enrollment = load_enrollment_for_update(transaction, tenant, run.enrollment).await?;
    let assignment = load_assignment(transaction, tenant, enrollment.assignment).await?;
    let previous = load_summary_for_update(transaction, tenant, enrollment.id).await?;
    let transition = ActivityTransition::CompleteRun {
        run: run_id,
        score,
        at,
    };
    let grade = grade_policy(&assignment);
    let next = project_summary(&previous, summary_transition(&transition), grade)?;
    run.completed_at = Some(at);
    run.score = Some(score);
    project_enrollment_completion(&mut enrollment, &previous, grade, run_id, score, at);
    let (run_payload, run_checksum) = encode_payload(&run)?;
    let (enrollment_payload, enrollment_checksum) = encode_payload(&enrollment)?;
    sqlx::query(
        "UPDATE assignment_run SET completed_at = to_timestamp($3::double precision / 1000.0), \
         payload = $4, payload_sha256 = $5 WHERE tenant_id = $1 AND run_id = $2",
    )
    .bind(tenant.as_uuid())
    .bind(run_id.as_uuid())
    .bind(at.as_unix_millis())
    .bind(run_payload)
    .bind(run_checksum)
    .execute(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    sqlx::query(
        "UPDATE enrollment SET payload = $3, payload_sha256 = $4 \
         WHERE tenant_id = $1 AND enrollment_id = $2",
    )
    .bind(tenant.as_uuid())
    .bind(enrollment.id.as_uuid())
    .bind(enrollment_payload)
    .bind(enrollment_checksum)
    .execute(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    store_summary(transaction, &next).await?;
    Ok(next)
}

#[cfg(feature = "postgres")]
async fn validate_postgres_assignment_references(
    transaction: &mut Transaction<'_, Postgres>,
    context: TenantContext,
    assignment: &AssignmentRecord,
) -> Result<(), StoreError> {
    let course_exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM course WHERE tenant_id = $1 AND course_id = $2)",
    )
    .bind(assignment.tenant.as_uuid())
    .bind(assignment.course_id.as_uuid())
    .fetch_one(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    if !course_exists {
        return Err(StoreError::InvalidRecord(
            "assignment references a missing course".to_string(),
        ));
    }
    for reference in assignment.references() {
        // The RLS-protected version table resolves visibility under the active
        // tenant. Deprecated immutable versions remain assignable; archived
        // versions are historical-only.
        let lifecycle: Option<String> = sqlx::query_scalar(
            "SELECT lifecycle FROM problem_version \
             WHERE problem_id = $1 AND version_id = $2 FOR SHARE",
        )
        .bind(reference.problem.as_uuid())
        .bind(reference.version.as_uuid())
        .fetch_optional(&mut **transaction)
        .await
        .map_err(map_sqlx_error)?;
        if !matches!(lifecycle.as_deref(), Some("published" | "deprecated")) {
            return Err(StoreError::InvalidRecord(format!(
                "assignment references a missing, hidden, or inactive published version {}/{}",
                reference.problem, reference.version
            )));
        }
    }
    let _ = context;
    Ok(())
}

#[cfg(feature = "postgres")]
async fn insert_postgres_assignment_items(
    transaction: &mut Transaction<'_, Postgres>,
    assignment: &AssignmentRecord,
) -> Result<(), StoreError> {
    for item in &assignment.items {
        insert_postgres_assignment_item(transaction, assignment, item).await?;
    }
    for group in &assignment.selection_groups {
        insert_postgres_assignment_group(transaction, assignment, group).await?;
    }
    Ok(())
}

#[cfg(feature = "postgres")]
async fn replace_postgres_assignment_items(
    transaction: &mut Transaction<'_, Postgres>,
    assignment: &AssignmentRecord,
) -> Result<(), StoreError> {
    let existing_item_rows = sqlx::query(
        "SELECT assignment_item_id, problem_id, version_id FROM assignment_item \
         WHERE tenant_id = $1 AND assignment_id = $2 FOR UPDATE",
    )
    .bind(assignment.tenant.as_uuid())
    .bind(assignment.id.as_uuid())
    .fetch_all(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    let mut existing_items = BTreeMap::new();
    for row in &existing_item_rows {
        existing_items.insert(
            AssignmentItemId::from_uuid(row.try_get("assignment_item_id").map_err(map_sqlx_error)?),
            ProblemVersionRef {
                problem: ProblemId::from_uuid(row.try_get("problem_id").map_err(map_sqlx_error)?),
                version: VersionId::from_uuid(row.try_get("version_id").map_err(map_sqlx_error)?),
            },
        );
    }
    for item in &assignment.items {
        if existing_items
            .get(&item.id)
            .is_some_and(|reference| *reference != item.reference)
        {
            return Err(StoreError::InvalidRecord(
                "replacing pinned content requires a new assignment item identity".to_string(),
            ));
        }
    }

    let existing_candidate_rows = sqlx::query(
        "SELECT candidate_id, selection_group_id, problem_id, version_id \
         FROM assignment_selection_candidate \
         WHERE tenant_id = $1 AND assignment_id = $2 FOR UPDATE",
    )
    .bind(assignment.tenant.as_uuid())
    .bind(assignment.id.as_uuid())
    .fetch_all(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    let mut existing_candidates = BTreeMap::new();
    for row in &existing_candidate_rows {
        existing_candidates.insert(
            AssignmentItemId::from_uuid(row.try_get("candidate_id").map_err(map_sqlx_error)?),
            (
                AssignmentSelectionGroupId::from_uuid(
                    row.try_get("selection_group_id").map_err(map_sqlx_error)?,
                ),
                ProblemVersionRef {
                    problem: ProblemId::from_uuid(
                        row.try_get("problem_id").map_err(map_sqlx_error)?,
                    ),
                    version: VersionId::from_uuid(
                        row.try_get("version_id").map_err(map_sqlx_error)?,
                    ),
                },
            ),
        );
    }
    for group in &assignment.selection_groups {
        for candidate in &group.candidates {
            if existing_candidates
                .get(&candidate.id)
                .is_some_and(|stored| *stored != (group.id, candidate.reference))
            {
                return Err(StoreError::InvalidRecord(
                    "moving or replacing a selection candidate requires a new identity".to_string(),
                ));
            }
        }
    }

    let item_ids = assignment
        .items
        .iter()
        .map(|item| item.id.as_uuid())
        .collect::<Vec<_>>();
    sqlx::query(
        "DELETE FROM assignment_item WHERE tenant_id = $1 AND assignment_id = $2 \
           AND NOT (assignment_item_id = ANY($3::uuid[]))",
    )
    .bind(assignment.tenant.as_uuid())
    .bind(assignment.id.as_uuid())
    .bind(&item_ids)
    .execute(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;

    let group_ids = assignment
        .selection_groups
        .iter()
        .map(|group| group.id.as_uuid())
        .collect::<Vec<_>>();
    sqlx::query(
        "DELETE FROM assignment_selection_group \
         WHERE tenant_id = $1 AND assignment_id = $2 \
           AND NOT (selection_group_id = ANY($3::uuid[]))",
    )
    .bind(assignment.tenant.as_uuid())
    .bind(assignment.id.as_uuid())
    .bind(&group_ids)
    .execute(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;

    let candidate_ids = assignment
        .selection_groups
        .iter()
        .flat_map(|group| group.candidates.iter())
        .map(|candidate| candidate.id.as_uuid())
        .collect::<Vec<_>>();
    sqlx::query(
        "DELETE FROM assignment_selection_candidate \
         WHERE tenant_id = $1 AND assignment_id = $2 \
           AND NOT (candidate_id = ANY($3::uuid[]))",
    )
    .bind(assignment.tenant.as_uuid())
    .bind(assignment.id.as_uuid())
    .bind(&candidate_ids)
    .execute(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;

    const POSITION_STAGING_OFFSET: i32 = 1_000_000;
    sqlx::query(
        "UPDATE assignment_item SET position = position + $3 \
         WHERE tenant_id = $1 AND assignment_id = $2",
    )
    .bind(assignment.tenant.as_uuid())
    .bind(assignment.id.as_uuid())
    .bind(POSITION_STAGING_OFFSET)
    .execute(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    sqlx::query(
        "UPDATE assignment_selection_group SET position = position + $3 \
         WHERE tenant_id = $1 AND assignment_id = $2",
    )
    .bind(assignment.tenant.as_uuid())
    .bind(assignment.id.as_uuid())
    .bind(POSITION_STAGING_OFFSET)
    .execute(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    sqlx::query(
        "UPDATE assignment_selection_candidate SET position = position + $3 \
         WHERE tenant_id = $1 AND assignment_id = $2",
    )
    .bind(assignment.tenant.as_uuid())
    .bind(assignment.id.as_uuid())
    .bind(POSITION_STAGING_OFFSET)
    .execute(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;

    for item in &assignment.items {
        let position = i32::try_from(item.position).map_err(|_| {
            StoreError::InvalidRecord("assignment item position is too large".to_string())
        })?;
        if existing_items.contains_key(&item.id) {
            sqlx::query(
                "UPDATE assignment_item SET position = $4, points_possible = $5::numeric, \
                        delivery_state = $6, scoring_mode = $7, revision = revision + 1, \
                        updated_at = transaction_timestamp() \
                 WHERE tenant_id = $1 AND assignment_id = $2 AND assignment_item_id = $3",
            )
            .bind(assignment.tenant.as_uuid())
            .bind(assignment.id.as_uuid())
            .bind(item.id.as_uuid())
            .bind(position)
            .bind(item.points_possible.to_string())
            .bind(assignment_delivery_state_name(item.delivery_state))
            .bind(assignment_scoring_mode_name(item.scoring_mode))
            .execute(&mut **transaction)
            .await
            .map_err(map_sqlx_error)?;
        } else {
            insert_postgres_assignment_item(transaction, assignment, item).await?;
        }
    }

    for group in &assignment.selection_groups {
        let position = i32::try_from(group.position).map_err(|_| {
            StoreError::InvalidRecord("selection group position is too large".to_string())
        })?;
        let updated = sqlx::query(
            "UPDATE assignment_selection_group \
             SET position = $4, draw_count = $5, points_per_item = $6::numeric, \
                 ordering_policy = $7, algorithm_version = $8, revision = revision + 1, \
                 updated_at = transaction_timestamp() \
             WHERE tenant_id = $1 AND assignment_id = $2 AND selection_group_id = $3",
        )
        .bind(assignment.tenant.as_uuid())
        .bind(assignment.id.as_uuid())
        .bind(group.id.as_uuid())
        .bind(position)
        .bind(i32::try_from(group.draw_count).map_err(|_| {
            StoreError::InvalidRecord("selection group draw count is too large".to_string())
        })?)
        .bind(group.points_per_item.to_string())
        .bind(selection_ordering_name(group.ordering))
        .bind(i32::from(group.algorithm_version))
        .execute(&mut **transaction)
        .await
        .map_err(map_sqlx_error)?;
        if updated.rows_affected() == 0 {
            insert_postgres_assignment_group(transaction, assignment, group).await?;
        } else {
            for candidate in &group.candidates {
                let updated = sqlx::query(
                    "UPDATE assignment_selection_candidate SET position = $5, delivery_state = $6, \
                            updated_at = transaction_timestamp() \
                     WHERE tenant_id = $1 AND assignment_id = $2 \
                       AND selection_group_id = $3 AND candidate_id = $4",
                )
                .bind(assignment.tenant.as_uuid())
                .bind(assignment.id.as_uuid())
                .bind(group.id.as_uuid())
                .bind(candidate.id.as_uuid())
                .bind(i32::try_from(candidate.position).map_err(|_| {
                    StoreError::InvalidRecord(
                        "selection candidate position is too large".to_string(),
                    )
                })?)
                .bind(assignment_delivery_state_name(candidate.delivery_state))
                .execute(&mut **transaction)
                .await
                .map_err(map_sqlx_error)?;
                if updated.rows_affected() == 0 {
                    insert_postgres_assignment_candidate(
                        transaction,
                        assignment,
                        group.id,
                        candidate,
                    )
                    .await?;
                }
            }
        }
    }
    Ok(())
}

#[cfg(feature = "postgres")]
async fn insert_postgres_assignment_item(
    transaction: &mut Transaction<'_, Postgres>,
    assignment: &AssignmentRecord,
    item: &AssignmentItem,
) -> Result<(), StoreError> {
    sqlx::query(
        "INSERT INTO assignment_item \
         (tenant_id, assignment_id, assignment_item_id, position, problem_id, version_id, \
          points_possible, delivery_state, scoring_mode) \
         VALUES ($1, $2, $3, $4, $5, $6, $7::numeric, $8, $9)",
    )
    .bind(assignment.tenant.as_uuid())
    .bind(assignment.id.as_uuid())
    .bind(item.id.as_uuid())
    .bind(i32::try_from(item.position).map_err(|_| {
        StoreError::InvalidRecord("assignment item position is too large".to_string())
    })?)
    .bind(item.reference.problem.as_uuid())
    .bind(item.reference.version.as_uuid())
    .bind(item.points_possible.to_string())
    .bind(assignment_delivery_state_name(item.delivery_state))
    .bind(assignment_scoring_mode_name(item.scoring_mode))
    .execute(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    Ok(())
}

#[cfg(feature = "postgres")]
async fn insert_postgres_assignment_group(
    transaction: &mut Transaction<'_, Postgres>,
    assignment: &AssignmentRecord,
    group: &AssignmentSelectionGroup,
) -> Result<(), StoreError> {
    sqlx::query(
        "INSERT INTO assignment_selection_group \
         (tenant_id, assignment_id, selection_group_id, position, draw_count, \
          points_per_item, ordering_policy, algorithm_version) \
         VALUES ($1, $2, $3, $4, $5, $6::numeric, $7, $8)",
    )
    .bind(assignment.tenant.as_uuid())
    .bind(assignment.id.as_uuid())
    .bind(group.id.as_uuid())
    .bind(i32::try_from(group.position).map_err(|_| {
        StoreError::InvalidRecord("selection group position is too large".to_string())
    })?)
    .bind(i32::try_from(group.draw_count).map_err(|_| {
        StoreError::InvalidRecord("selection group draw count is too large".to_string())
    })?)
    .bind(group.points_per_item.to_string())
    .bind(selection_ordering_name(group.ordering))
    .bind(i32::from(group.algorithm_version))
    .execute(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    for candidate in &group.candidates {
        insert_postgres_assignment_candidate(transaction, assignment, group.id, candidate).await?;
    }
    Ok(())
}

#[cfg(feature = "postgres")]
async fn insert_postgres_assignment_candidate(
    transaction: &mut Transaction<'_, Postgres>,
    assignment: &AssignmentRecord,
    group: AssignmentSelectionGroupId,
    candidate: &AssignmentSelectionCandidate,
) -> Result<(), StoreError> {
    sqlx::query(
        "INSERT INTO assignment_selection_candidate \
         (tenant_id, assignment_id, selection_group_id, candidate_id, position, problem_id, \
          version_id, delivery_state) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
    )
    .bind(assignment.tenant.as_uuid())
    .bind(assignment.id.as_uuid())
    .bind(group.as_uuid())
    .bind(candidate.id.as_uuid())
    .bind(i32::try_from(candidate.position).map_err(|_| {
        StoreError::InvalidRecord("selection candidate position is too large".to_string())
    })?)
    .bind(candidate.reference.problem.as_uuid())
    .bind(candidate.reference.version.as_uuid())
    .bind(assignment_delivery_state_name(candidate.delivery_state))
    .execute(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    Ok(())
}

#[cfg(feature = "postgres")]
async fn load_assignment(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    assignment: AssignmentId,
) -> Result<AssignmentRecord, StoreError> {
    let row = sqlx::query(
        "SELECT assignment_id, course_id, title, completion_policy, \
                completion_threshold::text AS completion_threshold, \
                attempt_selection_policy, continued_practice_policy, \
                practice_max_additional_runs, variation_policy \
         FROM assignment \
         WHERE tenant_id = $1 AND assignment_id = $2",
    )
    .bind(tenant.as_uuid())
    .bind(assignment.as_uuid())
    .fetch_optional(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?
    .ok_or(StoreError::NotFound)?;
    let header = decode_assignment_header(&row, tenant)?;
    load_assignment_relations(transaction, header).await
}

#[cfg(feature = "postgres")]
async fn load_assignment_for_share(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    assignment: AssignmentId,
) -> Result<AssignmentRecord, StoreError> {
    let row = sqlx::query(
        "SELECT assignment_id, course_id, title, completion_policy, \
                completion_threshold::text AS completion_threshold, \
                attempt_selection_policy, continued_practice_policy, \
                practice_max_additional_runs, variation_policy \
         FROM assignment \
         WHERE tenant_id = $1 AND assignment_id = $2 FOR SHARE",
    )
    .bind(tenant.as_uuid())
    .bind(assignment.as_uuid())
    .fetch_optional(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?
    .ok_or(StoreError::NotFound)?;
    let header = decode_assignment_header(&row, tenant)?;
    load_assignment_relations(transaction, header).await
}

#[cfg(feature = "postgres")]
fn decode_assignment_header(row: &PgRow, tenant: TenantId) -> Result<AssignmentRecord, StoreError> {
    let completion_policy: String = row.try_get("completion_policy").map_err(map_sqlx_error)?;
    let completion_threshold: Option<String> = row
        .try_get("completion_threshold")
        .map_err(map_sqlx_error)?;
    let grade_policy: String = row
        .try_get("attempt_selection_policy")
        .map_err(map_sqlx_error)?;
    let practice_policy: String = row
        .try_get("continued_practice_policy")
        .map_err(map_sqlx_error)?;
    let practice_limit: Option<i32> = row
        .try_get("practice_max_additional_runs")
        .map_err(map_sqlx_error)?;
    let variation_policy: String = row.try_get("variation_policy").map_err(map_sqlx_error)?;
    Ok(AssignmentRecord {
        id: AssignmentId::from_uuid(row.try_get("assignment_id").map_err(map_sqlx_error)?),
        tenant,
        course_id: CourseId::from_uuid(row.try_get("course_id").map_err(map_sqlx_error)?),
        title: row.try_get("title").map_err(map_sqlx_error)?,
        items: Vec::new(),
        selection_groups: Vec::new(),
        policies: RunPolicies {
            completion: parse_completion_policy(&completion_policy, completion_threshold)?,
            grade: parse_grade_policy(&grade_policy)?,
            continued_practice: parse_continued_practice(&practice_policy, practice_limit)?,
            variation: parse_variation_policy(&variation_policy)?,
        },
    })
}

#[cfg(feature = "postgres")]
async fn load_assignment_relations(
    transaction: &mut Transaction<'_, Postgres>,
    mut assignment: AssignmentRecord,
) -> Result<AssignmentRecord, StoreError> {
    let item_rows = sqlx::query(
        "SELECT assignment_item_id, position, problem_id, version_id, \
                points_possible::text AS points_possible, delivery_state, scoring_mode \
         FROM assignment_item WHERE tenant_id = $1 AND assignment_id = $2 \
         ORDER BY position, assignment_item_id",
    )
    .bind(assignment.tenant.as_uuid())
    .bind(assignment.id.as_uuid())
    .fetch_all(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    assignment.items = item_rows
        .iter()
        .map(decode_assignment_item)
        .collect::<Result<Vec<_>, _>>()?;

    let candidate_rows = sqlx::query(
        "SELECT selection_group_id, candidate_id, position, problem_id, version_id, delivery_state \
         FROM assignment_selection_candidate \
         WHERE tenant_id = $1 AND assignment_id = $2 \
         ORDER BY selection_group_id, position, candidate_id",
    )
    .bind(assignment.tenant.as_uuid())
    .bind(assignment.id.as_uuid())
    .fetch_all(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    let mut candidates: BTreeMap<AssignmentSelectionGroupId, Vec<AssignmentSelectionCandidate>> =
        BTreeMap::new();
    for row in &candidate_rows {
        let group = AssignmentSelectionGroupId::from_uuid(
            row.try_get("selection_group_id").map_err(map_sqlx_error)?,
        );
        candidates
            .entry(group)
            .or_default()
            .push(decode_assignment_candidate(row)?);
    }

    let group_rows = sqlx::query(
        "SELECT selection_group_id, position, draw_count, \
                points_per_item::text AS points_per_item, ordering_policy, algorithm_version \
         FROM assignment_selection_group WHERE tenant_id = $1 AND assignment_id = $2 \
         ORDER BY position, selection_group_id",
    )
    .bind(assignment.tenant.as_uuid())
    .bind(assignment.id.as_uuid())
    .fetch_all(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    assignment.selection_groups = group_rows
        .iter()
        .map(|row| {
            let id = AssignmentSelectionGroupId::from_uuid(
                row.try_get("selection_group_id").map_err(map_sqlx_error)?,
            );
            Ok(AssignmentSelectionGroup {
                id,
                position: stored_u32(row, "position", "selection group position")?,
                draw_count: stored_u32(row, "draw_count", "selection group draw count")?,
                points_per_item: stored_points(row, "points_per_item")?,
                ordering: parse_selection_ordering(
                    &row.try_get::<String, _>("ordering_policy")
                        .map_err(map_sqlx_error)?,
                )?,
                algorithm_version: stored_u16(
                    row,
                    "algorithm_version",
                    "selection algorithm version",
                )?,
                candidates: candidates.remove(&id).unwrap_or_default(),
            })
        })
        .collect::<Result<Vec<_>, StoreError>>()?;
    if !candidates.is_empty() {
        return Err(StoreError::Unavailable(
            "stored selection candidate has no assignment group".to_string(),
        ));
    }
    validate_assignment(&assignment).map_err(|error| {
        StoreError::Unavailable(format!("stored assignment is invalid: {error}"))
    })?;
    Ok(assignment)
}

#[cfg(feature = "postgres")]
fn decode_assignment_item(row: &PgRow) -> Result<AssignmentItem, StoreError> {
    Ok(AssignmentItem {
        id: AssignmentItemId::from_uuid(row.try_get("assignment_item_id").map_err(map_sqlx_error)?),
        reference: ProblemVersionRef {
            problem: ProblemId::from_uuid(row.try_get("problem_id").map_err(map_sqlx_error)?),
            version: VersionId::from_uuid(row.try_get("version_id").map_err(map_sqlx_error)?),
        },
        position: stored_u32(row, "position", "assignment item position")?,
        points_possible: stored_points(row, "points_possible")?,
        delivery_state: parse_assignment_delivery_state(
            &row.try_get::<String, _>("delivery_state")
                .map_err(map_sqlx_error)?,
        )?,
        scoring_mode: parse_assignment_scoring_mode(
            &row.try_get::<String, _>("scoring_mode")
                .map_err(map_sqlx_error)?,
        )?,
    })
}

#[cfg(feature = "postgres")]
fn decode_assignment_candidate(row: &PgRow) -> Result<AssignmentSelectionCandidate, StoreError> {
    Ok(AssignmentSelectionCandidate {
        id: AssignmentItemId::from_uuid(row.try_get("candidate_id").map_err(map_sqlx_error)?),
        position: stored_u32(row, "position", "selection candidate position")?,
        reference: ProblemVersionRef {
            problem: ProblemId::from_uuid(row.try_get("problem_id").map_err(map_sqlx_error)?),
            version: VersionId::from_uuid(row.try_get("version_id").map_err(map_sqlx_error)?),
        },
        delivery_state: parse_assignment_delivery_state(
            &row.try_get::<String, _>("delivery_state")
                .map_err(map_sqlx_error)?,
        )?,
    })
}

#[cfg(feature = "postgres")]
fn stored_points(row: &PgRow, column: &str) -> Result<PointValue, StoreError> {
    row.try_get::<String, _>(column)
        .map_err(map_sqlx_error)?
        .parse()
        .map_err(|error| StoreError::Unavailable(format!("stored points are invalid: {error}")))
}

#[cfg(feature = "postgres")]
fn stored_u32(row: &PgRow, column: &str, description: &str) -> Result<u32, StoreError> {
    let value: i32 = row.try_get(column).map_err(map_sqlx_error)?;
    u32::try_from(value)
        .map_err(|_| StoreError::Unavailable(format!("stored {description} is invalid")))
}

#[cfg(feature = "postgres")]
fn stored_u16(row: &PgRow, column: &str, description: &str) -> Result<u16, StoreError> {
    let value: i32 = row.try_get(column).map_err(map_sqlx_error)?;
    u16::try_from(value)
        .map_err(|_| StoreError::Unavailable(format!("stored {description} is invalid")))
}

#[cfg(feature = "postgres")]
async fn load_enrollment_for_update(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    enrollment: EnrollmentId,
) -> Result<AssignmentEnrollment, StoreError> {
    let row = sqlx::query(
        "SELECT payload, payload_sha256 FROM enrollment \
         WHERE tenant_id = $1 AND enrollment_id = $2 FOR UPDATE",
    )
    .bind(tenant.as_uuid())
    .bind(enrollment.as_uuid())
    .fetch_optional(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?
    .ok_or(StoreError::NotFound)?;
    decode_payload_row(&row)
}

#[cfg(feature = "postgres")]
async fn load_run_for_update(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    run: RunId,
) -> Result<AssignmentRun, StoreError> {
    let row = sqlx::query(
        "SELECT payload, payload_sha256 FROM assignment_run \
         WHERE tenant_id = $1 AND run_id = $2 FOR UPDATE",
    )
    .bind(tenant.as_uuid())
    .bind(run.as_uuid())
    .fetch_optional(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?
    .ok_or(StoreError::NotFound)?;
    decode_payload_row(&row)
}

#[cfg(feature = "postgres")]
async fn load_summary_for_update(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    enrollment: EnrollmentId,
) -> Result<StudentAssignmentSummary, StoreError> {
    let row = sqlx::query(
        "SELECT payload, payload_sha256 FROM student_assignment_summary \
         WHERE tenant_id = $1 AND enrollment_id = $2 FOR UPDATE",
    )
    .bind(tenant.as_uuid())
    .bind(enrollment.as_uuid())
    .fetch_optional(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?
    .ok_or(StoreError::NotFound)?;
    decode_payload_row(&row)
}

#[cfg(feature = "postgres")]
async fn store_summary(
    transaction: &mut Transaction<'_, Postgres>,
    summary: &StudentAssignmentSummary,
) -> Result<(), StoreError> {
    let (payload, checksum) = encode_payload(summary)?;
    sqlx::query(
        "UPDATE student_assignment_summary SET payload = $3, payload_sha256 = $4, \
         updated_at = transaction_timestamp() WHERE tenant_id = $1 AND enrollment_id = $2",
    )
    .bind(summary.tenant.as_uuid())
    .bind(summary.enrollment.as_uuid())
    .bind(payload)
    .bind(checksum)
    .execute(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    Ok(())
}

#[cfg(feature = "postgres")]
async fn insert_problem_version(
    transaction: &mut Transaction<'_, Postgres>,
    record: &PublishedProblemRecord,
    content_sha256: &str,
) -> Result<(), StoreError> {
    let backend = question_backend_name(QuestionBackend::from(&record.question.source));
    let (lifecycle, lifecycle_reason) = catalog_lifecycle_parts(&record.lifecycle);
    let derived_from_problem = record.derived_from.map(|source| source.problem.as_uuid());
    let derived_from_version = record.derived_from.map(|source| source.version.as_uuid());
    sqlx::query(
        "INSERT INTO problem_version \
         (problem_id, version_id, version_number, content_sha256, workspace_id, title, \
          backend, capabilities, metadata, \
          publication_scope, lifecycle, lifecycle_reason, authors, previous_version_id, \
          derived_from_problem_id, derived_from_version_id) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16)",
    )
    .bind(record.problem.as_uuid())
    .bind(record.version.as_uuid())
    .bind(i64::try_from(record.version_number.value()).map_err(|_| {
        StoreError::InvalidRecord("problem version number is too large".to_string())
    })?)
    .bind(content_sha256)
    .bind(record.question.workspace.as_uuid())
    .bind(&record.question.metadata.title)
    .bind(backend)
    .bind(Json(record.capabilities.clone()))
    .bind(Json(record.question.metadata.clone()))
    .bind(publication_scope_name(record.scope))
    .bind(lifecycle)
    .bind(lifecycle_reason)
    .bind(Json(record.authors.clone()))
    .bind(record.previous_version.map(|version| version.as_uuid()))
    .bind(derived_from_problem)
    .bind(derived_from_version)
    .execute(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    Ok(())
}

/// Persists a server-only source binding in the same transaction that makes
/// its immutable version visible.
#[cfg(feature = "postgres")]
async fn insert_published_source_artifact(
    transaction: &mut Transaction<'_, Postgres>,
    artifact: &PublishedSourceArtifact,
) -> Result<(), StoreError> {
    let (payload, checksum) = encode_payload(artifact)?;
    sqlx::query(
        "INSERT INTO published_source_artifact \
         (problem_id, version_id, backend, object_id, payload, payload_sha256) \
         VALUES ($1, $2, $3, $4, $5, $6)",
    )
    .bind(artifact.reference.problem.as_uuid())
    .bind(artifact.reference.version.as_uuid())
    .bind(question_backend_name(artifact.backend))
    .bind(artifact.object.id.as_uuid())
    .bind(payload)
    .bind(checksum)
    .execute(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    Ok(())
}

/// Inserts a QTI candidate asset while the containing publication transaction
/// is still private. The ordinary asset API deliberately cannot do this: it
/// only accepts assets for an already visible version.
#[cfg(feature = "postgres")]
async fn insert_catalog_asset_delivery(
    transaction: &mut Transaction<'_, Postgres>,
    record: &AssetDeliveryRecord,
) -> Result<(), StoreError> {
    validate_asset_delivery(record)?;
    let AssetDeliveryScope::Catalog { asset, reference } = record.scope else {
        return Err(StoreError::InvalidRecord(
            "QTI promotion assets must be catalog assets".to_string(),
        ));
    };
    let (payload, checksum) = encode_payload(record)?;
    sqlx::query(
        "INSERT INTO asset_delivery \
         (delivery_id, delivery_kind, tenant_id, object_id, problem_id, version_id, \
          asset_id, payload, payload_sha256) \
         VALUES ($1, 'catalog', NULL, $2, $3, $4, $5, $6, $7)",
    )
    .bind(record.id.as_uuid())
    .bind(record.object.id.as_uuid())
    .bind(reference.problem.as_uuid())
    .bind(reference.version.as_uuid())
    .bind(asset.as_uuid())
    .bind(payload)
    .bind(checksum)
    .execute(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    Ok(())
}

#[cfg(feature = "postgres")]
fn decode_qti_grading_row(
    row: Option<&PgRow>,
) -> Result<Option<QtiImportGradingPayload>, StoreError> {
    row.map(|row| {
        let bytes: Vec<u8> = row.try_get("payload").map_err(map_sqlx_error)?;
        let expected: String = row.try_get("payload_sha256").map_err(map_sqlx_error)?;
        if Sha256Digest::compute(&bytes).to_string() != expected {
            return Err(StoreError::Unavailable(
                "stored QTI grading payload checksum mismatch".to_string(),
            ));
        }
        QtiImportGradingPayload::new(bytes)
    })
    .transpose()
}

#[cfg(feature = "postgres")]
fn question_backend_name(backend: QuestionBackend) -> &'static str {
    match backend {
        QuestionBackend::Native => "native",
        QuestionBackend::Webwork => "webwork",
        QuestionBackend::Qti => "qti",
        QuestionBackend::H5p => "h5p",
        QuestionBackend::Imathas => "imathas",
    }
}

#[cfg(feature = "postgres")]
fn course_membership_role_name(role: CourseMembershipRole) -> &'static str {
    match role {
        CourseMembershipRole::Student => "student",
        CourseMembershipRole::Instructor => "instructor",
    }
}

#[cfg(feature = "postgres")]
fn assignment_delivery_state_name(state: question_model::AssignmentDeliveryState) -> &'static str {
    match state {
        question_model::AssignmentDeliveryState::Active => "active",
        question_model::AssignmentDeliveryState::Retired => "retired",
    }
}

#[cfg(feature = "postgres")]
fn completion_policy_columns(policy: CompletionRequirement) -> (&'static str, Option<String>) {
    match policy {
        CompletionRequirement::AnswerAll => ("answer_all", None),
        CompletionRequirement::AllCorrect => ("all_correct", None),
        CompletionRequirement::ScoreAtLeast { fraction } => {
            ("score_at_least", Some(fraction.to_string()))
        }
    }
}

#[cfg(feature = "postgres")]
fn grade_policy_name(policy: GradePolicy) -> &'static str {
    match policy {
        GradePolicy::First => "first",
        GradePolicy::Latest => "last",
        GradePolicy::Highest => "highest",
        GradePolicy::InstructorSelected => "instructor_selected",
    }
}

#[cfg(feature = "postgres")]
fn continued_practice_columns(
    policy: ContinuedPractice,
) -> Result<(&'static str, Option<i32>), StoreError> {
    match policy {
        ContinuedPractice::Unlimited => Ok(("unlimited", None)),
        ContinuedPractice::Closed => Ok(("closed", None)),
        ContinuedPractice::Capped {
            max_additional_runs,
        } => Ok((
            "capped",
            Some(i32::try_from(max_additional_runs).map_err(|_| {
                StoreError::InvalidRecord("continued-practice limit is too large".to_string())
            })?),
        )),
    }
}

#[cfg(feature = "postgres")]
fn variation_policy_name(policy: VariationPolicy) -> &'static str {
    match policy {
        VariationPolicy::NewSeeds => "new_seeds",
        VariationPolicy::SelectedProblemVariants => "selected_problem_variants",
        VariationPolicy::FullRegeneration => "full_regeneration",
    }
}

#[cfg(feature = "postgres")]
fn assignment_scoring_mode_name(mode: question_model::AssignmentScoringMode) -> &'static str {
    match mode {
        question_model::AssignmentScoringMode::Normal => "normal",
        question_model::AssignmentScoringMode::FullCredit => "full_credit",
        question_model::AssignmentScoringMode::ExtraCredit => "extra_credit",
        question_model::AssignmentScoringMode::Excluded => "excluded",
    }
}

#[cfg(feature = "postgres")]
fn selection_ordering_name(ordering: question_model::SelectionOrdering) -> &'static str {
    match ordering {
        question_model::SelectionOrdering::CandidateOrder => "candidate_order",
        question_model::SelectionOrdering::Randomized => "randomized",
    }
}

#[cfg(feature = "postgres")]
fn parse_assignment_delivery_state(value: &str) -> Result<AssignmentDeliveryState, StoreError> {
    match value {
        "active" => Ok(AssignmentDeliveryState::Active),
        "retired" => Ok(AssignmentDeliveryState::Retired),
        _ => Err(invalid_stored_assignment_value("delivery state", value)),
    }
}

#[cfg(feature = "postgres")]
fn parse_assignment_scoring_mode(value: &str) -> Result<AssignmentScoringMode, StoreError> {
    match value {
        "normal" => Ok(AssignmentScoringMode::Normal),
        "full_credit" => Ok(AssignmentScoringMode::FullCredit),
        "extra_credit" => Ok(AssignmentScoringMode::ExtraCredit),
        "excluded" => Ok(AssignmentScoringMode::Excluded),
        _ => Err(invalid_stored_assignment_value("scoring mode", value)),
    }
}

#[cfg(feature = "postgres")]
fn parse_selection_ordering(value: &str) -> Result<SelectionOrdering, StoreError> {
    match value {
        "candidate_order" => Ok(SelectionOrdering::CandidateOrder),
        "randomized" => Ok(SelectionOrdering::Randomized),
        _ => Err(invalid_stored_assignment_value("selection ordering", value)),
    }
}

#[cfg(feature = "postgres")]
fn parse_completion_policy(
    policy: &str,
    threshold: Option<String>,
) -> Result<CompletionRequirement, StoreError> {
    match (policy, threshold) {
        ("answer_all", None) => Ok(CompletionRequirement::AnswerAll),
        ("all_correct", None) => Ok(CompletionRequirement::AllCorrect),
        ("score_at_least", Some(value)) => {
            let fraction = value
                .parse::<f64>()
                .map_err(|_| invalid_stored_assignment_value("completion threshold", &value))?;
            Ok(CompletionRequirement::ScoreAtLeast { fraction })
        }
        _ => Err(invalid_stored_assignment_value("completion policy", policy)),
    }
}

#[cfg(feature = "postgres")]
fn parse_grade_policy(value: &str) -> Result<GradePolicy, StoreError> {
    match value {
        "first" => Ok(GradePolicy::First),
        "last" => Ok(GradePolicy::Latest),
        "highest" => Ok(GradePolicy::Highest),
        "instructor_selected" => Ok(GradePolicy::InstructorSelected),
        _ => Err(invalid_stored_assignment_value(
            "attempt selection policy",
            value,
        )),
    }
}

#[cfg(feature = "postgres")]
fn parse_continued_practice(
    policy: &str,
    limit: Option<i32>,
) -> Result<ContinuedPractice, StoreError> {
    match (policy, limit) {
        ("unlimited", None) => Ok(ContinuedPractice::Unlimited),
        ("closed", None) => Ok(ContinuedPractice::Closed),
        ("capped", Some(limit)) => Ok(ContinuedPractice::Capped {
            max_additional_runs: u32::try_from(limit).map_err(|_| {
                invalid_stored_assignment_value("continued-practice limit", &limit.to_string())
            })?,
        }),
        _ => Err(invalid_stored_assignment_value(
            "continued-practice policy",
            policy,
        )),
    }
}

#[cfg(feature = "postgres")]
fn parse_variation_policy(value: &str) -> Result<VariationPolicy, StoreError> {
    match value {
        "new_seeds" => Ok(VariationPolicy::NewSeeds),
        "selected_problem_variants" => Ok(VariationPolicy::SelectedProblemVariants),
        "full_regeneration" => Ok(VariationPolicy::FullRegeneration),
        _ => Err(invalid_stored_assignment_value("variation policy", value)),
    }
}

#[cfg(feature = "postgres")]
fn invalid_stored_assignment_value(field: &str, value: &str) -> StoreError {
    StoreError::Unavailable(format!("stored assignment {field} is invalid: {value}"))
}

#[cfg(feature = "postgres")]
fn decode_scoring_generation(row: &PgRow) -> Result<ScoringGeneration, StoreError> {
    let value: i64 = row.try_get("scoring_generation").map_err(map_sqlx_error)?;
    u64::try_from(value)
        .ok()
        .and_then(ScoringGeneration::new)
        .ok_or_else(|| invalid_stored_assignment_value("scoring generation", &value.to_string()))
}

#[cfg(feature = "postgres")]
fn decode_scoring_status(row: &PgRow) -> Result<ScoringStatus, StoreError> {
    let value: String = row.try_get("scoring_status").map_err(map_sqlx_error)?;
    match value.as_str() {
        "current" => Ok(ScoringStatus::Current),
        "recalculating" => Ok(ScoringStatus::Recalculating),
        "failed" => Ok(ScoringStatus::Failed),
        _ => Err(invalid_stored_assignment_value("scoring status", &value)),
    }
}

#[cfg(feature = "postgres")]
fn parse_course_membership_role(value: &str) -> Result<CourseMembershipRole, StoreError> {
    match value {
        "student" => Ok(CourseMembershipRole::Student),
        "instructor" => Ok(CourseMembershipRole::Instructor),
        _ => Err(StoreError::Unavailable(format!(
            "stored course membership role is invalid: {value}"
        ))),
    }
}

#[cfg(feature = "postgres")]
fn parse_course_role(value: &str) -> Result<CourseRole, StoreError> {
    match value {
        "student" => Ok(CourseRole::Student),
        "instructor" => Ok(CourseRole::Instructor),
        "administrator" => Ok(CourseRole::Administrator),
        _ => Err(StoreError::Unavailable(format!(
            "stored effective course role is invalid: {value}"
        ))),
    }
}

#[cfg(feature = "postgres")]
fn parse_question_backend(value: &str) -> Result<QuestionBackend, StoreError> {
    match value {
        "native" => Ok(QuestionBackend::Native),
        "webwork" => Ok(QuestionBackend::Webwork),
        "qti" => Ok(QuestionBackend::Qti),
        "h5p" => Ok(QuestionBackend::H5p),
        "imathas" => Ok(QuestionBackend::Imathas),
        _ => Err(StoreError::Unavailable(format!(
            "stored question backend is invalid: {value}"
        ))),
    }
}

#[cfg(feature = "postgres")]
fn publication_scope_name(scope: PublicationScope) -> &'static str {
    match scope {
        PublicationScope::Institution => "institution",
        PublicationScope::Public => "public",
    }
}

#[cfg(feature = "postgres")]
fn parse_publication_scope(value: &str) -> Result<PublicationScope, StoreError> {
    match value {
        "institution" => Ok(PublicationScope::Institution),
        "public" => Ok(PublicationScope::Public),
        _ => Err(StoreError::Unavailable(format!(
            "stored publication scope is invalid: {value}"
        ))),
    }
}

#[cfg(feature = "postgres")]
fn catalog_lifecycle_parts(lifecycle: &CatalogLifecycle) -> (&'static str, Option<&str>) {
    match lifecycle {
        CatalogLifecycle::Published => ("published", None),
        CatalogLifecycle::Deprecated { reason } => ("deprecated", Some(reason.as_str())),
        CatalogLifecycle::Archived { reason } => ("archived", Some(reason.as_str())),
    }
}

#[cfg(feature = "postgres")]
fn parse_catalog_lifecycle(
    lifecycle: &str,
    reason: Option<String>,
) -> Result<CatalogLifecycle, StoreError> {
    match (lifecycle, reason) {
        ("published", None) => Ok(CatalogLifecycle::Published),
        ("deprecated", Some(reason)) => Ok(CatalogLifecycle::Deprecated {
            reason: validated_deprecation_reason(reason)?,
        }),
        ("archived", Some(reason)) => Ok(CatalogLifecycle::Archived {
            reason: validated_deprecation_reason(reason)?,
        }),
        _ => Err(StoreError::Unavailable(
            "stored catalog lifecycle and reason disagree".to_string(),
        )),
    }
}

#[cfg(feature = "postgres")]
fn decode_catalog_payload_row(row: &PgRow) -> Result<PublishedProblemRecord, StoreError> {
    let mut record: PublishedProblemRecord = decode_payload_row(row)?;
    let stored_problem = ProblemId::from_uuid(row.try_get("problem_id").map_err(map_sqlx_error)?);
    let stored_version = VersionId::from_uuid(row.try_get("version_id").map_err(map_sqlx_error)?);
    let stored_public_id =
        decode_problem_public_id(row.try_get("public_id").map_err(map_sqlx_error)?)?;
    let stored_version_number =
        decode_problem_version_number(row.try_get("version_number").map_err(map_sqlx_error)?)?;
    if record.problem != stored_problem
        || record.public_id != stored_public_id
        || record.version != stored_version
        || record.version_number != stored_version_number
    {
        return Err(StoreError::Unavailable(
            "stored catalog payload identity disagrees with its row".to_string(),
        ));
    }
    let lifecycle: String = row.try_get("lifecycle").map_err(map_sqlx_error)?;
    let reason: Option<String> = row.try_get("lifecycle_reason").map_err(map_sqlx_error)?;
    record.lifecycle = parse_catalog_lifecycle(&lifecycle, reason)?;
    validate_published(&record).map_err(|error| {
        StoreError::Unavailable(format!("stored catalog payload is invalid: {error}"))
    })?;
    Ok(record)
}

#[cfg(feature = "postgres")]
fn decode_catalog_summary_row(row: &PgRow) -> Result<CatalogProblemSummary, StoreError> {
    let problem = ProblemId::from_uuid(row.try_get("problem_id").map_err(map_sqlx_error)?);
    let public_id = decode_problem_public_id(row.try_get("public_id").map_err(map_sqlx_error)?)?;
    let version = VersionId::from_uuid(row.try_get("version_id").map_err(map_sqlx_error)?);
    let version_number =
        decode_problem_version_number(row.try_get("version_number").map_err(map_sqlx_error)?)?;
    let backend: String = row.try_get("backend").map_err(map_sqlx_error)?;
    let Json(capabilities): Json<BackendCapabilities> =
        row.try_get("capabilities").map_err(map_sqlx_error)?;
    let Json(metadata): Json<QuestionMetadata> = row.try_get("metadata").map_err(map_sqlx_error)?;
    let publication_scope: String = row.try_get("publication_scope").map_err(map_sqlx_error)?;
    let lifecycle: String = row.try_get("lifecycle").map_err(map_sqlx_error)?;
    let lifecycle_reason: Option<String> =
        row.try_get("lifecycle_reason").map_err(map_sqlx_error)?;
    let Json(authors): Json<Vec<UserId>> = row.try_get("authors").map_err(map_sqlx_error)?;
    if authors.is_empty() {
        return Err(StoreError::Unavailable(
            "stored catalog authors must not be empty".to_string(),
        ));
    }
    let previous_version = row
        .try_get::<Option<Uuid>, _>("previous_version_id")
        .map_err(map_sqlx_error)?
        .map(VersionId::from_uuid);
    let derived_problem = row
        .try_get::<Option<Uuid>, _>("derived_from_problem_id")
        .map_err(map_sqlx_error)?;
    let derived_version = row
        .try_get::<Option<Uuid>, _>("derived_from_version_id")
        .map_err(map_sqlx_error)?;
    let derived_from = match (derived_problem, derived_version) {
        (Some(problem), Some(version)) => Some(ProblemVersionRef {
            problem: ProblemId::from_uuid(problem),
            version: VersionId::from_uuid(version),
        }),
        (None, None) => None,
        _ => {
            return Err(StoreError::Unavailable(
                "stored catalog fork lineage is incomplete".to_string(),
            ));
        }
    };
    let published_at_millis: i64 = row.try_get("published_at_millis").map_err(map_sqlx_error)?;
    Ok(CatalogProblemSummary {
        problem,
        public_id,
        version,
        version_number,
        backend: parse_question_backend(&backend)?,
        capabilities,
        metadata,
        scope: parse_publication_scope(&publication_scope)?,
        lifecycle: parse_catalog_lifecycle(&lifecycle, lifecycle_reason)?,
        authors,
        previous_version,
        derived_from,
        published_at: ActivityTimestamp::from_unix_millis(published_at_millis),
    })
}

#[cfg(feature = "postgres")]
fn decode_problem_public_id(value: i64) -> Result<ProblemPublicId, StoreError> {
    u64::try_from(value)
        .ok()
        .and_then(ProblemPublicId::new)
        .ok_or_else(|| StoreError::Unavailable("stored problem public ID is invalid".to_string()))
}

#[cfg(feature = "postgres")]
fn decode_problem_version_number(value: i64) -> Result<ProblemVersionNumber, StoreError> {
    u64::try_from(value)
        .ok()
        .and_then(ProblemVersionNumber::new)
        .ok_or_else(|| {
            StoreError::Unavailable("stored problem version number is invalid".to_string())
        })
}

#[cfg(feature = "postgres")]
fn postgres_search_page_request(query: &CatalogSearchQuery) -> Result<PageRequest, StoreError> {
    let size = PageSize::new(query.page_size.unwrap_or(50))
        .map_err(|error| StoreError::InvalidRecord(error.to_string()))?;
    match query.cursor.clone() {
        Some(cursor) => Cursor::parse(cursor)
            .map(|cursor| PageRequest::after(cursor, size))
            .map_err(|error| StoreError::InvalidRecord(error.to_string())),
        None => Ok(PageRequest::first(size)),
    }
}

#[cfg(feature = "postgres")]
fn postgres_catalog_search_fingerprint(query: &CatalogSearchQuery) -> String {
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

#[cfg(feature = "postgres")]
fn decode_catalog_taxonomy_facet(row: PgRow) -> Result<CatalogTaxonomyFacet, StoreError> {
    let Json(term): Json<TaxonomyTerm> = row.try_get("taxonomy_term").map_err(map_sqlx_error)?;
    let count: i64 = row.try_get("facet_count").map_err(map_sqlx_error)?;
    Ok(CatalogTaxonomyFacet {
        term,
        count: u64::try_from(count)
            .map_err(|_| StoreError::Unavailable("catalog count overflow".to_string()))?,
    })
}

#[cfg(feature = "postgres")]
fn decode_catalog_capability_facet(row: PgRow) -> Result<CatalogCapabilityFacet, StoreError> {
    let capability: String = row.try_get("capability").map_err(map_sqlx_error)?;
    let count: i64 = row.try_get("facet_count").map_err(map_sqlx_error)?;
    let capability = serde_json::from_value(Value::String(capability)).map_err(|_| {
        StoreError::Unavailable("stored catalog capability facet is invalid".to_string())
    })?;
    Ok(CatalogCapabilityFacet {
        capability,
        count: u64::try_from(count)
            .map_err(|_| StoreError::Unavailable("catalog count overflow".to_string()))?,
    })
}

#[cfg(feature = "postgres")]
fn decode_catalog_license_facet(row: PgRow) -> Result<CatalogLicenseFacet, StoreError> {
    let license: String = row.try_get("license").map_err(map_sqlx_error)?;
    let count: i64 = row.try_get("facet_count").map_err(map_sqlx_error)?;
    let license = serde_json::from_value(Value::String(license)).map_err(|_| {
        StoreError::Unavailable("stored catalog license facet is invalid".to_string())
    })?;
    Ok(CatalogLicenseFacet {
        license,
        count: u64::try_from(count)
            .map_err(|_| StoreError::Unavailable("catalog count overflow".to_string()))?,
    })
}

#[cfg(feature = "postgres")]
fn catalog_summary_page_from_rows(
    rows: Vec<PgRow>,
    page_size: u16,
) -> Result<Page<CatalogProblemSummary>, StoreError> {
    let mut records = rows
        .iter()
        .map(|row| {
            let key = row
                .try_get::<String, _>("stable_key")
                .map_err(map_sqlx_error)?;
            Ok((key, decode_catalog_summary_row(row)?))
        })
        .collect::<Result<Vec<_>, StoreError>>()?;
    page_from_keyed_records(&mut records, page_size)
}

#[cfg(feature = "postgres")]
fn taxonomy_page_from_rows(
    rows: Vec<PgRow>,
    page_size: u16,
) -> Result<Page<TaxonomyTerm>, StoreError> {
    let mut records = rows
        .iter()
        .map(|row| {
            let key = row
                .try_get::<String, _>("stable_key")
                .map_err(map_sqlx_error)?;
            let Json(term): Json<TaxonomyTerm> =
                row.try_get("taxonomy_term").map_err(map_sqlx_error)?;
            Ok((key, term))
        })
        .collect::<Result<Vec<_>, StoreError>>()?;
    page_from_keyed_records(&mut records, page_size)
}

#[cfg(feature = "postgres")]
fn page_from_keyed_records<T>(
    records: &mut Vec<(String, T)>,
    page_size: u16,
) -> Result<Page<T>, StoreError> {
    let has_more = records.len() > usize::from(page_size);
    if has_more {
        records.pop();
    }
    let next_cursor = if has_more {
        records
            .last()
            .map(|(key, _)| Cursor::from_stable_key(key.clone()))
    } else {
        None
    };
    Ok(Page {
        items: records.drain(..).map(|(_, record)| record).collect(),
        next_cursor,
    })
}

/// Converts the `LIMIT page_size + 1` native UUID tuple result into one page.
///
/// The SQL order and continuation key deliberately use the same tuple, unlike
/// the generic string-key pages above. That alignment keeps the gradebook
/// query eligible for its assignment/enrollment page indexes.
#[cfg(feature = "postgres")]
fn gradebook_page_from_records<T>(
    records: &mut Vec<(GradebookCursor, T)>,
    page_size: u16,
) -> Page<T> {
    let has_more = records.len() > usize::from(page_size);
    if has_more {
        records.pop();
    }
    let next_cursor = has_more.then(|| {
        records
            .last()
            .map(|(key, _)| key.encode())
            .expect("a nonempty page precedes a following page")
    });
    Page {
        items: records.drain(..).map(|(_, record)| record).collect(),
        next_cursor,
    }
}

#[cfg(feature = "postgres")]
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

#[cfg(feature = "postgres")]
fn encode_payload<T: Serialize>(record: &T) -> Result<(Json<Value>, String), StoreError> {
    let value = serde_json::to_value(record)
        .map_err(|error| StoreError::InvalidRecord(error.to_string()))?;
    let bytes =
        serde_json::to_vec(&value).map_err(|error| StoreError::InvalidRecord(error.to_string()))?;
    let checksum = Sha256Digest::compute(&bytes).to_string();
    Ok((Json(value), checksum))
}

#[cfg(feature = "postgres")]
fn decode_payload_row<T: DeserializeOwned>(row: &PgRow) -> Result<T, StoreError> {
    let Json(value): Json<Value> = row.try_get("payload").map_err(map_sqlx_error)?;
    let expected: String = row.try_get("payload_sha256").map_err(map_sqlx_error)?;
    let bytes =
        serde_json::to_vec(&value).map_err(|error| StoreError::Unavailable(error.to_string()))?;
    if Sha256Digest::compute(&bytes).to_string() != expected {
        return Err(StoreError::Unavailable(
            "stored JSON payload checksum mismatch".to_string(),
        ));
    }
    serde_json::from_value(value).map_err(|error| StoreError::Unavailable(error.to_string()))
}

#[cfg(feature = "postgres")]
fn decode_payload_row_named<T: DeserializeOwned>(
    row: &PgRow,
    payload_name: &str,
    checksum_name: &str,
) -> Result<T, StoreError> {
    let Json(value): Json<Value> = row.try_get(payload_name).map_err(map_sqlx_error)?;
    let expected: String = row.try_get(checksum_name).map_err(map_sqlx_error)?;
    let bytes =
        serde_json::to_vec(&value).map_err(|error| StoreError::Unavailable(error.to_string()))?;
    if Sha256Digest::compute(&bytes).to_string() != expected {
        return Err(StoreError::Unavailable(
            "stored JSON payload checksum mismatch".to_string(),
        ));
    }
    serde_json::from_value(value).map_err(|error| StoreError::Unavailable(error.to_string()))
}

#[cfg(feature = "postgres")]
fn attempt_status_name(status: AttemptStatus) -> &'static str {
    match status {
        AttemptStatus::InProgress => "in_progress",
        AttemptStatus::Submitted => "submitted",
        AttemptStatus::AutoSubmitted => "auto_submitted",
        AttemptStatus::NeedsManualGrading => "needs_manual_grading",
        AttemptStatus::Cleared => "cleared",
        AttemptStatus::Exempt => "exempt",
    }
}

#[cfg(feature = "postgres")]
fn decode_attempt_status(value: &str) -> Result<AttemptStatus, StoreError> {
    match value {
        "in_progress" => Ok(AttemptStatus::InProgress),
        "submitted" => Ok(AttemptStatus::Submitted),
        "auto_submitted" => Ok(AttemptStatus::AutoSubmitted),
        "needs_manual_grading" => Ok(AttemptStatus::NeedsManualGrading),
        "cleared" => Ok(AttemptStatus::Cleared),
        "exempt" => Ok(AttemptStatus::Exempt),
        _ => Err(StoreError::Unavailable(
            "stored attempt status is invalid".to_string(),
        )),
    }
}

#[cfg(feature = "postgres")]
fn decode_current_attempt_row_named(
    row: &PgRow,
    payload_name: &str,
    checksum_name: &str,
) -> Result<QuestionAttempt, StoreError> {
    let mut attempt: QuestionAttempt = decode_payload_row_named(row, payload_name, checksum_name)?;
    let status: String = row
        .try_get("current_attempt_status")
        .map_err(map_sqlx_error)?;
    attempt.status = decode_attempt_status(&status)?;
    if let Some(submitted_at) = row
        .try_get::<Option<i64>, _>("current_submitted_at")
        .map_err(map_sqlx_error)?
    {
        attempt.timer.submitted_at = Some(ActivityTimestamp::from_unix_millis(submitted_at));
    } else if attempt.status == AttemptStatus::InProgress {
        attempt.timer.submitted_at = None;
    }
    match row.try_get::<Option<i64>, _>("current_deadline_at") {
        Ok(deadline) => {
            attempt.timer.deadline = deadline.map(ActivityTimestamp::from_unix_millis);
        }
        Err(sqlx::Error::ColumnNotFound(_)) => {}
        Err(error) => return Err(map_sqlx_error(error)),
    }
    Ok(attempt)
}

#[cfg(feature = "postgres")]
fn decode_current_attempt_row(row: &PgRow) -> Result<QuestionAttempt, StoreError> {
    decode_current_attempt_row_named(row, "payload", "payload_sha256")
}

#[cfg(feature = "postgres")]
fn feedback_from_summary_row(row: &PgRow) -> Result<Option<AttemptFeedbackRecord>, StoreError> {
    let digest: Option<String> = row.try_get("content_sha256").map_err(map_sqlx_error)?;
    let Some(digest) = digest else {
        return Ok(None);
    };
    fn field(row: &PgRow, name: &str) -> Result<Option<Vec<ContentBlock>>, StoreError> {
        let value: Option<Value> = row.try_get(name).map_err(map_sqlx_error)?;
        value
            .map(|value| {
                serde_json::from_value(value).map_err(|error| {
                    StoreError::InvalidRecord(format!("stored feedback decode failed: {error}"))
                })
            })
            .transpose()
    }
    let feedback = private_feedback_record(FeedbackContent {
        hint: field(row, "hint")?,
        correct_response: field(row, "correct_response")?,
        rationale: field(row, "rationale")?,
    })?;
    if feedback.content_sha256().to_string() != digest {
        return Err(StoreError::InvalidRecord(
            "stored feedback digest mismatch".to_string(),
        ));
    }
    Ok(Some(feedback))
}

#[cfg(feature = "postgres")]
fn feedback_policy_from_summary_row(row: &PgRow) -> Result<FeedbackDisclosure, StoreError> {
    let value: String = row.try_get("feedback_policy").map_err(map_sqlx_error)?;
    serde_json::from_value(Value::String(value))
        .map_err(|_| StoreError::Unavailable("stored feedback policy is invalid".to_string()))
}

#[cfg(feature = "postgres")]
fn decode_asset_delivery_row(row: &PgRow) -> Result<AssetDeliveryRecord, StoreError> {
    let record: AssetDeliveryRecord = decode_payload_row(row)?;
    validate_asset_delivery(&record).map_err(|error| {
        StoreError::Unavailable(format!("stored asset delivery is invalid: {error}"))
    })?;
    Ok(record)
}

#[cfg(feature = "postgres")]
fn decode_session_row(row: &PgRow) -> Result<SessionRecord, StoreError> {
    let token_hash: String = row.try_get("session_hash").map_err(map_sqlx_error)?;
    let tenant = row.try_get("tenant_id").map_err(map_sqlx_error)?;
    let user = row.try_get("user_id").map_err(map_sqlx_error)?;
    let display_name: String = row.try_get("display_name").map_err(map_sqlx_error)?;
    let Json(roles): Json<Vec<UserRole>> = row.try_get("roles").map_err(map_sqlx_error)?;
    let created_at_millis: i64 = row.try_get("created_at_millis").map_err(map_sqlx_error)?;
    let expires_at_millis: i64 = row.try_get("expires_at_millis").map_err(map_sqlx_error)?;
    let token_hash = SessionTokenHash::from_hex(token_hash.trim_end()).map_err(|error| {
        StoreError::Unavailable(format!("stored session hash is invalid: {error}"))
    })?;
    let subject = SessionSubject::new(
        TenantId::from_uuid(tenant),
        UserId::from_uuid(user),
        display_name,
        roles,
    )
    .map_err(|error| {
        StoreError::Unavailable(format!("stored session subject is invalid: {error}"))
    })?;
    Ok(SessionRecord {
        token_hash,
        subject,
        created_at: ActivityTimestamp::from_unix_millis(created_at_millis),
        expires_at: ActivityTimestamp::from_unix_millis(expires_at_millis),
    })
}

#[cfg(feature = "postgres")]
fn page_from_rows<T: DeserializeOwned>(
    rows: Vec<PgRow>,
    page_size: u16,
) -> Result<Page<T>, StoreError> {
    page_from_rows_with(rows, page_size, decode_payload_row)
}

#[cfg(feature = "postgres")]
fn page_from_rows_with<T>(
    rows: Vec<PgRow>,
    page_size: u16,
    decode: impl Fn(&PgRow) -> Result<T, StoreError>,
) -> Result<Page<T>, StoreError> {
    let mut records = rows
        .iter()
        .map(|row| {
            let key = row
                .try_get::<String, _>("stable_key")
                .map_err(map_sqlx_error)?;
            let record = decode(row)?;
            Ok((key, record))
        })
        .collect::<Result<Vec<_>, StoreError>>()?;
    let has_more = records.len() > usize::from(page_size);
    if has_more {
        records.pop();
    }
    let next_cursor = if has_more {
        records
            .last()
            .map(|(key, _)| Cursor::from_stable_key(key.clone()))
    } else {
        None
    };
    Ok(Page {
        items: records.into_iter().map(|(_, record)| record).collect(),
        next_cursor,
    })
}

#[cfg(feature = "postgres")]
fn map_sqlx_error(error: sqlx::Error) -> StoreError {
    if let sqlx::Error::Database(database_error) = &error {
        match database_error.code().as_deref() {
            Some("23505")
                if database_error.constraint() == Some("problem_version_linear_chain_idx") =>
            {
                return StoreError::Conflict;
            }
            Some("23505") => return StoreError::AlreadyExists,
            Some("23503") | Some("23514") => {
                return StoreError::InvalidRecord(database_error.message().to_string());
            }
            Some("55000") => return StoreError::Conflict,
            // Retention broker functions deliberately raise this SQLSTATE after
            // validating a stored session and course authority. It keeps missing
            // and unauthorized courses nonenumerating while matching Memory's
            // authority-first Store boundary.
            Some("42501") => return StoreError::Forbidden,
            _ => {}
        }
    }
    StoreError::Unavailable(error.to_string())
}

#[cfg(feature = "postgres")]
fn decode_job_payload(value: Value) -> Result<JobPayload, StoreError> {
    serde_json::from_value(value).map_err(|error| {
        StoreError::Unavailable(format!("stored queue payload is invalid: {error}"))
    })
}

#[cfg(feature = "postgres")]
fn decode_claimed_job(
    row: &PgRow,
    expected_token: JobLeaseToken,
) -> Result<ClaimedJob, StoreError> {
    let Json(payload): Json<Value> = row.try_get("payload").map_err(map_sqlx_error)?;
    let stored_token: Uuid = row.try_get("lease_token").map_err(map_sqlx_error)?;
    if stored_token != expected_token.as_uuid() {
        return Err(StoreError::Unavailable(
            "queue broker returned a mismatched lease token".to_string(),
        ));
    }
    let attempt_count: i32 = row.try_get("attempt_count").map_err(map_sqlx_error)?;
    Ok(ClaimedJob {
        id: JobId::from_uuid(row.try_get("job_id").map_err(map_sqlx_error)?),
        tenant: TenantId::from_uuid(row.try_get("tenant_id").map_err(map_sqlx_error)?),
        payload: decode_job_payload(payload)?,
        lease_token: JobLeaseToken::from_uuid(stored_token),
        attempt_count: u16::try_from(attempt_count).map_err(|_| {
            StoreError::Unavailable("stored queue attempt count is invalid".to_string())
        })?,
    })
}

#[cfg(feature = "postgres")]
fn decode_tenant_job_view(row: &PgRow, id: JobId) -> Result<TenantJobView, StoreError> {
    let Json(payload): Json<Value> = row.try_get("payload").map_err(map_sqlx_error)?;
    let state: String = row.try_get("state").map_err(map_sqlx_error)?;
    let state = match state.as_str() {
        "ready" => JobState::Ready,
        "leased" => JobState::Leased,
        "completed" => JobState::Completed,
        "dead" => JobState::Dead,
        _ => {
            return Err(StoreError::Unavailable(
                "stored queue state is invalid".to_string(),
            ));
        }
    };
    let attempt_count: i32 = row.try_get("attempt_count").map_err(map_sqlx_error)?;
    Ok(TenantJobView {
        id,
        payload: decode_job_payload(payload)?,
        state,
        attempt_count: u16::try_from(attempt_count).map_err(|_| {
            StoreError::Unavailable("stored queue attempt count is invalid".to_string())
        })?,
    })
}
