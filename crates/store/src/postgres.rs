//! PostgreSQL backend, embedded migrations, and connection handling.
//!
//! Every operation runs as the non-bypassing `ple_app` role. Tenant-owned
//! operations also set `ple.tenant_id` locally inside their transaction, so a
//! pooled connection cannot retain another request's tenant context.

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
use question_model::run_policy::{FeedbackDisclosure, TimingPolicy};
#[cfg(feature = "postgres")]
use question_model::taxonomy::TaxonomyTerm;
#[cfg(feature = "postgres")]
use question_model::{
    ActivityTimestamp, AssetId, AssignmentEnrollment, AssignmentId, AssignmentRun, AttemptResult,
    AttemptTimerRecord, BackendCapabilities, CatalogCapabilityFacet, CatalogLicenseFacet,
    CatalogLifecycle, CatalogProblemDetail, CatalogProblemSummary, CatalogSearchFacets,
    CatalogSearchPage, CatalogSearchQuery, CatalogStatisticsAvailability, CatalogStatisticsFacet,
    CatalogTaxonomyFacet, CourseId, CourseMembership, CourseMembershipRole, CourseRole,
    CourseSummary, EnrollmentId, EnrollmentStatus, ObjectId, ProblemId, ProblemVersionRef,
    PublicationScope, QuestionAttempt, QuestionAttemptId, QuestionBackend, QuestionDefinition,
    QuestionMetadata, QuestionStatisticsDisclosure, QuestionStatisticsView, RunId, RunMode,
    StudentAssignmentSummary, StudentResponse, TenantId, UserId, UserRole, VersionId,
    WorkspaceDraftSummary, WorkspaceId, WorkspaceImportId,
};
#[cfg(feature = "postgres")]
use question_model::{FeedbackContent, envelope::ContentBlock};
#[cfg(feature = "postgres")]
use serde::Serialize;
#[cfg(feature = "postgres")]
use serde::de::DeserializeOwned;
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
    AssetStore, AssignmentDefinitionDisposition, AssignmentRecord, AssignmentRevision,
    AssignmentUpdate, AttemptFeedbackRecord, AuthorizedAssetDelivery, CatalogAssetBinding,
    CatalogSourceStore, CatalogStore, CatalogTransition, CourseListScope, CourseRecord,
    CourseRecordsAccessStore, CourseRetentionRecord, CourseRetentionSnapshot, CourseRetentionState,
    CourseRetentionView, Cursor, DraftRecord, FeedbackReleaseRecord, InstitutionRetentionPolicy,
    IssueQuestionAttemptCommand, Page, PageRequest, PageSize, PrefetchedQuestion,
    PublishDraftCommand, PublishedProblemRecord, PublishedSourceArtifact,
    ReleaseAttemptFeedbackCommand, ReservePrefetchedQuestionCommand, RetentionApiStore,
    RetentionCleanupManifest, RetentionDays, RetentionDispatchBatch, RetentionRevision,
    RetentionScheduleStore, RetentionStore, RetentionWork, RetentionWorkerCommand,
    RetentionWorkerStore, RunSummaryOutcomeInput, RunSummaryPageInput, SessionLifetime,
    SessionRecord, SessionStore, SessionSubject, SessionTokenHash, Store, StoreError,
    StoredAssignment, SubmissionIdempotencyKey, SubmissionNextAttempt, SubmissionRecord,
    SubmitQuestionAttemptCommand, TenantContext, WorkspaceDraft, WorkspaceDraftRevision,
    completed_run_score, decode_catalog_search_cursor, decode_workspace_draft_cursor,
    encode_catalog_search_cursor, encode_workspace_draft_cursor, ensure_tenant, grade_policy,
    private_feedback_record, project_enrollment_completion, summary_transition,
    validate_asset_delivery, validate_assignment, validate_course, validate_draft,
    validate_publication_source, validate_published, validate_qti_import,
    validate_qti_publication_promotion, validate_source_artifact,
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
const MIGRATIONS: &[(i64, &str)] = &[
    (
        20260807000000,
        include_str!("../../../schemas/migrations/20260807000000_initial.sql"),
    ),
    (
        20260807000100,
        include_str!("../../../schemas/migrations/20260807000100_auth_sessions.sql"),
    ),
    (
        20260807000200,
        include_str!("../../../schemas/migrations/20260807000200_catalog.sql"),
    ),
    (
        20260807000300,
        include_str!("../../../schemas/migrations/20260807000300_courses.sql"),
    ),
    (
        20260807000400,
        include_str!("../../../schemas/migrations/20260807000400_run_api.sql"),
    ),
    (
        20260807000500,
        include_str!("../../../schemas/migrations/20260807000500_asset_delivery.sql"),
    ),
    (
        20260808000000,
        include_str!("../../../schemas/migrations/20260808000000_imathas_catalog_backend.sql"),
    ),
    (
        20260808000100,
        include_str!("../../../schemas/migrations/20260808000100_published_source_artifact.sql"),
    ),
    (
        20260808000200,
        include_str!("../../../schemas/migrations/20260808000200_worker_jobs.sql"),
    ),
    (
        20260808000300,
        include_str!("../../../schemas/migrations/20260808000300_gradebook_summary.sql"),
    ),
    (
        20260808000400,
        include_str!("../../../schemas/migrations/20260808000400_imathas_broker.sql"),
    ),
    (
        20260808000500,
        include_str!(
            "../../../schemas/migrations/20260808000500_imathas_broker_verification_token.sql"
        ),
    ),
    (
        20260808000600,
        include_str!("../../../schemas/migrations/20260808000600_workspace_qti_import.sql"),
    ),
    (
        20260808000700,
        include_str!("../../../schemas/migrations/20260808000700_attempt_feedback.sql"),
    ),
    (
        20260808000800,
        include_str!("../../../schemas/migrations/20260808000800_qti_grader_principal.sql"),
    ),
    (
        20260808000900,
        include_str!("../../../schemas/migrations/20260808000900_submission_receipt_snapshot.sql"),
    ),
    (
        20260808001000,
        include_str!("../../../schemas/migrations/20260808001000_catalog_search.sql"),
    ),
    (
        20260808001100,
        include_str!("../../../schemas/migrations/20260808001100_feedback_release.sql"),
    ),
    (
        20260808001200,
        include_str!("../../../schemas/migrations/20260808001200_run_summary_cursor.sql"),
    ),
    (
        20260808001300,
        include_str!("../../../schemas/migrations/20260808001300_workspace_draft_privileges.sql"),
    ),
    (
        20260808001400,
        include_str!("../../../schemas/migrations/20260808001400_workspace_draft_access.sql"),
    ),
    (
        20260808001500,
        include_str!("../../../schemas/migrations/20260808001500_qti_prepared_import.sql"),
    ),
    (
        20260808001600,
        include_str!("../../../schemas/migrations/20260808001600_published_qti_grading.sql"),
    ),
    (
        20260808001700,
        include_str!("../../../schemas/migrations/20260808001700_assignment_revision.sql"),
    ),
    (
        20260808001800,
        include_str!("../../../schemas/migrations/20260808001800_question_prefetch.sql"),
    ),
    (
        20260808001900,
        include_str!("../../../schemas/migrations/20260808001900_student_exports.sql"),
    ),
    (
        20260808002000,
        include_str!("../../../schemas/migrations/20260808002000_question_statistics.sql"),
    ),
    (
        20260808002100,
        include_str!("../../../schemas/migrations/20260808002100_retention_foundation.sql"),
    ),
    (
        20260808002200,
        include_str!("../../../schemas/migrations/20260808002200_retention_worker.sql"),
    ),
    (
        20260808002300,
        include_str!("../../../schemas/migrations/20260808002300_retention_lifecycle.sql"),
    ),
    (
        20260808002400,
        include_str!("../../../schemas/migrations/20260808002400_retention_api.sql"),
    ),
    (
        20260808002500,
        include_str!("../../../schemas/migrations/20260808002500_retention_archive_access.sql"),
    ),
];

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
        let row = sqlx::query("SELECT payload FROM assignment WHERE assignment_id = $1 FOR SHARE")
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
            problems: assignment
                .problems
                .iter()
                .map(|reference| ProblemVersionRef {
                    problem: reference.problem,
                    version: reference.version,
                })
                .collect(),
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
        let requester: UserId = sqlx::query_scalar(
            "SELECT requester_id FROM student_export_request WHERE job_id = $1 AND manifest_object_id = $2",
        )
        .bind(commit.job.as_uuid())
        .bind(commit.manifest.as_uuid())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?
        .map(UserId::from_uuid)
        .ok_or(StoreError::NotFound)?;
        let mut artifacts = Vec::with_capacity(commit.artifacts.len());
        for artifact in &commit.artifacts {
            let delivery = AssetDeliveryRecord {
                id: AssetDeliveryId::from_object(artifact.object.id),
                object: artifact.object.clone(),
                scope: AssetDeliveryScope::StudentRecord {
                    tenant: context.tenant_id(),
                    authorized_users: vec![requester],
                },
            };
            // The broker replaces this empty list with the frozen requestor. Validate object shape
            // directly here because the public asset helper rightfully refuses an empty ACL.
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
        let (kind, tenant, problem, version, asset) = match &record.scope {
            AssetDeliveryScope::Catalog { asset, reference } => (
                "catalog",
                None,
                Some(reference.problem),
                Some(reference.version),
                Some(*asset),
            ),
            AssetDeliveryScope::StudentRecord { tenant, .. } => {
                ensure_tenant(context, *tenant)?;
                ("student_record", Some(*tenant), None, None, None)
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
        sqlx::query(
            "INSERT INTO asset_delivery \
             (delivery_id, delivery_kind, tenant_id, object_id, problem_id, version_id, \
              asset_id, payload, payload_sha256) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
        )
        .bind(record.id.as_uuid())
        .bind(kind)
        .bind(tenant.map(|value| value.as_uuid()))
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
            "SELECT payload, payload_sha256 FROM asset_delivery \
             WHERE delivery_id = $1",
        )
        .bind(delivery.as_uuid())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?
        .ok_or(StoreError::NotFound)?;
        let record = decode_asset_delivery_row(&row)?;
        if let AssetDeliveryScope::StudentRecord {
            tenant,
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
            occurred_at: authorized_at,
        };
        let (payload, checksum) = encode_payload(&event)?;
        sqlx::query(
            "INSERT INTO audit_event \
             (tenant_id, audit_event_id, occurred_at, payload, payload_sha256) \
             VALUES ($1, gen_random_uuid(), transaction_timestamp(), $2, $3)",
        )
        .bind(context.tenant_id().as_uuid())
        .bind(payload)
        .bind(checksum)
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

/// Applies every embedded, checksummed schema migration in version order.
///
/// # Errors
///
/// Returns a database or migration-integrity failure.
#[cfg(feature = "postgres")]
pub async fn apply_migrations(pool: &PgPool) -> Result<(), sqlx::Error> {
    let mut transaction = pool.begin().await?;
    sqlx::query("SELECT pg_advisory_xact_lock(731_026_808)")
        .execute(&mut *transaction)
        .await?;
    sqlx::raw_sql(
        "CREATE TABLE IF NOT EXISTS ple_schema_migration (\
         version bigint PRIMARY KEY, \
         checksum character(64) NOT NULL, \
         applied_at timestamptz NOT NULL DEFAULT transaction_timestamp()\
         )",
    )
    .execute(&mut *transaction)
    .await?;
    for (version, migration) in MIGRATIONS {
        let checksum = Sha256Digest::compute(migration.as_bytes()).to_string();
        let existing: Option<String> =
            sqlx::query_scalar("SELECT checksum FROM ple_schema_migration WHERE version = $1")
                .bind(version)
                .fetch_optional(&mut *transaction)
                .await?;
        if let Some(existing) = existing {
            if existing != checksum {
                return Err(sqlx::Error::Protocol(format!(
                    "migration {version} checksum changed after application"
                )));
            }
            continue;
        }
        sqlx::raw_sql(*migration).execute(&mut *transaction).await?;
        sqlx::query("INSERT INTO ple_schema_migration (version, checksum) VALUES ($1, $2)")
            .bind(version)
            .bind(checksum)
            .execute(&mut *transaction)
            .await?;
    }
    transaction.commit().await
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
            "SELECT pv.problem_id, pv.version_id, pvp.payload, pvp.payload_sha256, \
                    pv.lifecycle, pv.lifecycle_reason \
             FROM problem_version AS pv \
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
        let mut transaction = self.begin_tenant(context).await?;
        sqlx::query(
            "INSERT INTO course (tenant_id, course_id, title) VALUES ($1, $2, $3) \
             ON CONFLICT (tenant_id, course_id) DO UPDATE SET \
             title = EXCLUDED.title, updated_at = transaction_timestamp()",
        )
        .bind(course.tenant.as_uuid())
        .bind(course.id.as_uuid())
        .bind(&course.title)
        .execute(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        sqlx::query("DELETE FROM course_member WHERE tenant_id = $1 AND course_id = $2")
            .bind(course.tenant.as_uuid())
            .bind(course.id.as_uuid())
            .execute(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        for membership in &course.members {
            sqlx::query(
                "INSERT INTO course_member (tenant_id, course_id, user_id, role) \
                 VALUES ($1, $2, $3, $4)",
            )
            .bind(course.tenant.as_uuid())
            .bind(course.id.as_uuid())
            .bind(membership.user.as_uuid())
            .bind(course_membership_role_name(membership.role))
            .execute(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
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
            CourseListScope::Member(user) => sqlx::query(
                "SELECT c.course_id::text AS stable_key, c.course_id, c.title, cm.role \
                 FROM course AS c JOIN course_member AS cm \
                   ON cm.tenant_id = c.tenant_id AND cm.course_id = c.course_id \
                 WHERE c.tenant_id = $1 AND cm.user_id = $2 \
                   AND ($3::text IS NULL OR c.course_id::text > $3) \
                 ORDER BY c.course_id::text LIMIT $4",
            )
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

    async fn create_assignment(
        &self,
        context: TenantContext,
        assignment: AssignmentRecord,
    ) -> Result<StoredAssignment, StoreError> {
        ensure_tenant(context, assignment.tenant)?;
        validate_assignment(&assignment)?;
        let (payload, checksum) = encode_payload(&assignment)?;
        let mut transaction = self.begin_tenant(context).await?;
        validate_postgres_assignment_references(&mut transaction, context, &assignment).await?;
        let inserted = sqlx::query(
            "INSERT INTO assignment \
             (tenant_id, assignment_id, course_id, title, payload, payload_sha256, revision) \
             VALUES ($1, $2, $3, $4, $5, $6, 1) \
             ON CONFLICT (tenant_id, assignment_id) DO NOTHING \
             RETURNING revision",
        )
        .bind(assignment.tenant.as_uuid())
        .bind(assignment.id.as_uuid())
        .bind(assignment.course_id.as_uuid())
        .bind(&assignment.title)
        .bind(payload)
        .bind(checksum)
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let Some(row) = inserted else {
            return Err(StoreError::AlreadyExists);
        };
        insert_postgres_assignment_problems(&mut transaction, &assignment).await?;
        let revision =
            AssignmentRevision::from_stored(row.try_get("revision").map_err(map_sqlx_error)?)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(StoredAssignment {
            record: assignment,
            revision,
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
            problems: update.problems,
            policies: update.policies,
        };
        validate_assignment(&assignment)?;
        let (payload, checksum) = encode_payload(&assignment)?;
        let mut transaction = self.begin_tenant(context).await?;
        validate_postgres_assignment_references(&mut transaction, context, &assignment).await?;
        let revision = sqlx::query_scalar::<_, i64>(
            "UPDATE assignment SET title = $4, payload = $5, payload_sha256 = $6, \
                    revision = revision + 1, updated_at = transaction_timestamp() \
             WHERE tenant_id = $1 AND assignment_id = $2 AND course_id = $3 AND revision = $7 \
             RETURNING revision",
        )
        .bind(assignment.tenant.as_uuid())
        .bind(assignment.id.as_uuid())
        .bind(assignment.course_id.as_uuid())
        .bind(&assignment.title)
        .bind(payload)
        .bind(checksum)
        .bind(i64::try_from(expected_revision.value()).map_err(|_| StoreError::Conflict)?)
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let Some(revision) = revision else {
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
        sqlx::query("DELETE FROM assignment_problem WHERE tenant_id = $1 AND assignment_id = $2")
            .bind(assignment.tenant.as_uuid())
            .bind(assignment.id.as_uuid())
            .execute(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        insert_postgres_assignment_problems(&mut transaction, &assignment).await?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(StoredAssignment {
            record: assignment,
            revision: AssignmentRevision::from_stored(revision)?,
        })
    }

    async fn get_assignment_for_edit(
        &self,
        context: TenantContext,
        assignment: AssignmentId,
    ) -> Result<Option<StoredAssignment>, StoreError> {
        let mut transaction = self.begin_tenant(context).await?;
        let row = sqlx::query(
            "SELECT payload, payload_sha256, revision FROM assignment \
             WHERE tenant_id = $1 AND assignment_id = $2",
        )
        .bind(context.tenant_id().as_uuid())
        .bind(assignment.as_uuid())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let result = row
            .as_ref()
            .map(|row| {
                Ok::<_, StoreError>(StoredAssignment {
                    record: decode_payload_row(row)?,
                    revision: AssignmentRevision::from_stored(
                        row.try_get("revision").map_err(map_sqlx_error)?,
                    )?,
                })
            })
            .transpose()?;
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
            "SELECT payload, payload_sha256 FROM assignment \
             WHERE tenant_id = $1 AND assignment_id = $2",
        )
        .bind(context.tenant_id().as_uuid())
        .bind(assignment.as_uuid())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let record = row.as_ref().map(decode_payload_row).transpose()?;
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
            "SELECT assignment_id::text AS stable_key, payload, payload_sha256 \
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
        let result = page_from_rows(rows, page.size.get())?;
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
                    COALESCE(si.payload_sha256, qa.payload_sha256) AS payload_sha256 \
             FROM question_attempt AS qa \
             LEFT JOIN submission_idempotency AS si \
               ON si.tenant_id = qa.tenant_id AND si.attempt_id = qa.attempt_id \
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
        let result = page_from_rows(rows, page.size.get())?;
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
                    pvp.payload #>> '{question,attemptPolicy,feedback}' AS feedback_policy, \
                    af.hint, af.correct_response, af.rationale, af.content_sha256, \
                    fr.released_by, floor(extract(epoch FROM fr.released_at) * 1000)::bigint AS released_at \
             FROM question_attempt AS qa \
             LEFT JOIN submission_idempotency AS si \
               ON si.tenant_id = qa.tenant_id AND si.attempt_id = qa.attempt_id \
             JOIN problem_version_payload AS pvp \
               ON pvp.problem_id = qa.problem_id AND pvp.version_id = qa.version_id \
             LEFT JOIN attempt_feedback AS af \
               ON af.tenant_id = qa.tenant_id AND af.attempt_id = qa.attempt_id \
             LEFT JOIN feedback_release AS fr \
               ON fr.tenant_id = qa.tenant_id AND fr.attempt_id = qa.attempt_id \
             WHERE qa.tenant_id = $1 AND qa.run_id = $2 \
               AND ($3::integer IS NULL OR (qa.assignment_position, qa.attempt_id) > ($3, $4::uuid)) \
             ORDER BY qa.assignment_position, qa.attempt_id LIMIT $5",
        )
        .bind(tenant.as_uuid())
        .bind(run.id.as_uuid())
        .bind(after.map(|cursor| i32::try_from(cursor.assignment_position)).transpose().map_err(|_| StoreError::InvalidRecord("run summary cursor position is invalid".to_string()))?)
        .bind(after.map(|cursor| cursor.attempt))
        .bind(limit)
        .fetch_all(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let has_more = rows.len() > usize::from(page.size.get());
        let mut outcomes = Vec::with_capacity(rows.len().min(usize::from(page.size.get())));
        for row in rows.into_iter().take(usize::from(page.size.get())) {
            let attempt: QuestionAttempt =
                decode_payload_row_named(&row, "attempt_payload", "attempt_sha256")?;
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
                    COALESCE(si.payload_sha256, qa.payload_sha256) AS payload_sha256 \
             FROM question_attempt AS qa \
             LEFT JOIN submission_idempotency AS si \
               ON si.tenant_id = qa.tenant_id AND si.attempt_id = qa.attempt_id \
             WHERE qa.tenant_id = $1 AND qa.attempt_id = $2 \
             ORDER BY qa.occurred_at LIMIT 1",
        )
        .bind(context.tenant_id().as_uuid())
        .bind(attempt.as_uuid())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let record = row.as_ref().map(decode_payload_row).transpose()?;
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
        let (authors, previous_version, derived_from, is_new_problem) =
            if let Some(revises) = command.expected_draft.revises {
                if publication.problem != revises.problem {
                    return Err(StoreError::InvalidRecord(
                        "revision must remain in its existing problem chain".to_string(),
                    ));
                }
                let base_row = sqlx::query(
                    "SELECT pv.problem_id, pv.version_id, pvp.payload, pvp.payload_sha256, \
                            pv.lifecycle, pv.lifecycle_reason \
                     FROM problem_version AS pv \
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
                    false,
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
                    true,
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
            version: publication.version,
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

        if is_new_problem {
            sqlx::query("INSERT INTO problem (problem_id) VALUES ($1)")
                .bind(record.problem.as_uuid())
                .execute(&mut *transaction)
                .await
                .map_err(map_sqlx_error)?;
        }
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
        insert_problem_version(&mut transaction, &record).await?;
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
            "SELECT pv.problem_id, pv.version_id, pvp.payload, pvp.payload_sha256, \
                    pv.lifecycle, pv.lifecycle_reason \
             FROM problem_version AS pv \
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

    async fn list_catalog(
        &self,
        context: TenantContext,
        page: PageRequest,
    ) -> Result<Page<CatalogProblemSummary>, StoreError> {
        let cursor = page.after.as_ref().map(|value| value.as_str().to_string());
        let limit = i64::from(page.size.get()) + 1;
        let mut transaction = self.begin_tenant(context).await?;
        let rows = sqlx::query(
            "SELECT pv.problem_id::text || '/' || pv.version_id::text AS stable_key, \
                    pv.problem_id, pv.version_id, pv.backend, pv.capabilities, pv.metadata, \
                    pv.publication_scope, pv.lifecycle, pv.lifecycle_reason, pv.authors, \
                    pv.previous_version_id, pv.derived_from_problem_id, \
                    pv.derived_from_version_id, \
                    floor(extract(epoch FROM pv.created_at) * 1000)::bigint \
                        AS published_at_millis \
             FROM problem_version AS pv \
             WHERE pv.lifecycle = 'published' \
               AND ($1::text IS NULL \
                    OR pv.problem_id::text || '/' || pv.version_id::text > $1) \
             ORDER BY pv.problem_id::text, pv.version_id::text LIMIT $2",
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
                     SELECT pv.problem_id, pv.version_id, \
                            encode(convert_to(term->>'scheme', 'UTF8'), 'hex') || '/' || \
                            encode(convert_to(term->>'code', 'UTF8'), 'hex') AS stable_key, \
                            term AS taxonomy_term \
                     FROM problem_version AS pv \
                     CROSS JOIN LATERAL jsonb_array_elements( \
                         CASE WHEN jsonb_typeof(pv.metadata->'taxonomy') = 'array' \
                              THEN pv.metadata->'taxonomy' ELSE '[]'::jsonb END \
                     ) AS term \
                     WHERE pv.lifecycle = 'published' \
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
            "SELECT pv.problem_id::text || '/' || pv.version_id::text AS stable_key, \
                    pv.problem_id, pv.version_id, pv.backend, pv.capabilities, pv.metadata, \
                    pv.publication_scope, pv.lifecycle, pv.lifecycle_reason, pv.authors, \
                    pv.previous_version_id, pv.derived_from_problem_id, \
                    pv.derived_from_version_id, \
                    floor(extract(epoch FROM pv.created_at) * 1000)::bigint \
                        AS published_at_millis \
             FROM problem_version AS pv \
             LEFT JOIN LATERAL ple_question_statistics_view(pv.problem_id, pv.version_id) AS statistics \
               ON TRUE \
             WHERE pv.lifecycle = 'published' \
               AND ($1::text IS NULL OR to_tsvector('simple', pv.title || ' ' || pv.metadata::text) \
                    @@ websearch_to_tsquery('simple', $1)) \
               AND NOT EXISTS ( \
                   SELECT 1 FROM jsonb_array_elements($2::jsonb) AS wanted \
                   WHERE NOT EXISTS ( \
                       SELECT 1 FROM jsonb_array_elements( \
                           CASE WHEN jsonb_typeof(pv.metadata->'taxonomy') = 'array' \
                                THEN pv.metadata->'taxonomy' ELSE '[]'::jsonb END \
                       ) AS stored \
                       WHERE stored->>'scheme' = wanted->>'scheme' \
                         AND stored->>'code' = wanted->>'code' \
                   ) \
               ) \
               AND pv.capabilities @> $3::jsonb \
               AND (jsonb_array_length($4::jsonb) = 0 OR (pv.metadata->'license'->>'kind') \
                    IN (SELECT jsonb_array_elements_text($4::jsonb))) \
               AND ($5::smallint <> 1 OR statistics.cohort_size IS NOT NULL) \
               AND ($5::smallint <> 2 OR statistics.cohort_size IS NULL) \
               AND ($6::uuid IS NULL OR (pv.problem_id, pv.version_id) > ($6, $7)) \
             ORDER BY pv.problem_id, pv.version_id LIMIT $8",
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
                 SELECT pv.metadata FROM problem_version AS pv \
                 LEFT JOIN LATERAL ple_question_statistics_view(pv.problem_id, pv.version_id) AS statistics ON TRUE \
                 WHERE pv.lifecycle = 'published' \
                   AND ($1::text IS NULL OR to_tsvector('simple', pv.title || ' ' || pv.metadata::text) \
                        @@ websearch_to_tsquery('simple', $1)) \
                   AND NOT EXISTS (SELECT 1 FROM jsonb_array_elements($2::jsonb) AS wanted \
                       WHERE NOT EXISTS (SELECT 1 FROM jsonb_array_elements( \
                           CASE WHEN jsonb_typeof(pv.metadata->'taxonomy') = 'array' \
                                THEN pv.metadata->'taxonomy' ELSE '[]'::jsonb END) AS stored \
                           WHERE stored->>'scheme' = wanted->>'scheme' AND stored->>'code' = wanted->>'code')) \
                   AND pv.capabilities @> $3::jsonb \
                   AND (jsonb_array_length($4::jsonb) = 0 OR (pv.metadata->'license'->>'kind') \
                        IN (SELECT jsonb_array_elements_text($4::jsonb))) \
                   AND ($5::smallint <> 1 OR statistics.cohort_size IS NOT NULL) \
                   AND ($5::smallint <> 2 OR statistics.cohort_size IS NULL) \
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
            "WITH filtered AS (SELECT pv.capabilities FROM problem_version AS pv \
               LEFT JOIN LATERAL ple_question_statistics_view(pv.problem_id, pv.version_id) AS statistics ON TRUE \
               WHERE pv.lifecycle = 'published' \
               AND ($1::text IS NULL OR to_tsvector('simple', pv.title || ' ' || pv.metadata::text) @@ websearch_to_tsquery('simple', $1)) \
               AND NOT EXISTS (SELECT 1 FROM jsonb_array_elements($2::jsonb) AS wanted WHERE NOT EXISTS (SELECT 1 FROM jsonb_array_elements(CASE WHEN jsonb_typeof(pv.metadata->'taxonomy') = 'array' THEN pv.metadata->'taxonomy' ELSE '[]'::jsonb END) AS stored WHERE stored->>'scheme' = wanted->>'scheme' AND stored->>'code' = wanted->>'code')) \
               AND pv.capabilities @> $3::jsonb AND (jsonb_array_length($4::jsonb) = 0 OR (pv.metadata->'license'->>'kind') IN (SELECT jsonb_array_elements_text($4::jsonb))) \
               AND ($5::smallint <> 1 OR statistics.cohort_size IS NOT NULL) \
               AND ($5::smallint <> 2 OR statistics.cohort_size IS NULL)) \
             SELECT capability, count(*)::bigint AS facet_count FROM filtered CROSS JOIN LATERAL jsonb_array_elements_text(capabilities) AS capability GROUP BY capability ORDER BY capability",
        ).bind(text.clone()).bind(taxonomy.clone()).bind(capabilities.clone()).bind(licenses.clone()).bind(statistics).fetch_all(&mut *transaction).await.map_err(map_sqlx_error)?;
        let license_rows = sqlx::query(
            "WITH filtered AS (SELECT pv.metadata FROM problem_version AS pv \
               LEFT JOIN LATERAL ple_question_statistics_view(pv.problem_id, pv.version_id) AS statistics ON TRUE \
               WHERE pv.lifecycle = 'published' \
               AND ($1::text IS NULL OR to_tsvector('simple', pv.title || ' ' || pv.metadata::text) @@ websearch_to_tsquery('simple', $1)) \
               AND NOT EXISTS (SELECT 1 FROM jsonb_array_elements($2::jsonb) AS wanted WHERE NOT EXISTS (SELECT 1 FROM jsonb_array_elements(CASE WHEN jsonb_typeof(pv.metadata->'taxonomy') = 'array' THEN pv.metadata->'taxonomy' ELSE '[]'::jsonb END) AS stored WHERE stored->>'scheme' = wanted->>'scheme' AND stored->>'code' = wanted->>'code')) \
               AND pv.capabilities @> $3::jsonb AND (jsonb_array_length($4::jsonb) = 0 OR (pv.metadata->'license'->>'kind') IN (SELECT jsonb_array_elements_text($4::jsonb))) \
               AND ($5::smallint <> 1 OR statistics.cohort_size IS NOT NULL) \
               AND ($5::smallint <> 2 OR statistics.cohort_size IS NULL)) \
             SELECT metadata->'license'->>'kind' AS license, count(*)::bigint AS facet_count FROM filtered GROUP BY license ORDER BY license",
        ).bind(text.clone()).bind(taxonomy.clone()).bind(capabilities.clone()).bind(licenses.clone()).bind(statistics).fetch_all(&mut *transaction).await.map_err(map_sqlx_error)?;
        let statistics_facet = sqlx::query(
            "SELECT count(*) FILTER (WHERE statistics.cohort_size IS NOT NULL)::bigint AS available, \
                    count(*) FILTER (WHERE statistics.cohort_size IS NULL)::bigint AS unavailable \
             FROM problem_version AS pv \
             LEFT JOIN LATERAL ple_question_statistics_view(pv.problem_id, pv.version_id) AS statistics ON TRUE \
             WHERE pv.lifecycle = 'published' \
             AND ($1::text IS NULL OR to_tsvector('simple', pv.title || ' ' || pv.metadata::text) @@ websearch_to_tsquery('simple', $1)) \
             AND NOT EXISTS (SELECT 1 FROM jsonb_array_elements($2::jsonb) AS wanted WHERE NOT EXISTS (SELECT 1 FROM jsonb_array_elements(CASE WHEN jsonb_typeof(pv.metadata->'taxonomy') = 'array' THEN pv.metadata->'taxonomy' ELSE '[]'::jsonb END) AS stored WHERE stored->>'scheme' = wanted->>'scheme' AND stored->>'code' = wanted->>'code')) \
             AND pv.capabilities @> $3::jsonb AND (jsonb_array_length($4::jsonb) = 0 OR (pv.metadata->'license'->>'kind') IN (SELECT jsonb_array_elements_text($4::jsonb))) \
             AND ($5::smallint <> 1 OR statistics.cohort_size IS NOT NULL) \
             AND ($5::smallint <> 2 OR statistics.cohort_size IS NULL)",
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
            "SELECT pv.problem_id, pv.version_id, pvp.payload, pvp.payload_sha256, \
                    pv.lifecycle, pv.lifecycle_reason \
             FROM problem_version AS pv \
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
            "SELECT pv.problem_id, pv.version_id, pvp.payload, pvp.payload_sha256, \
                    pv.lifecycle, pv.lifecycle_reason \
             FROM problem_version AS pv \
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

    let assignment = load_assignment(transaction, tenant, assignment_id).await?;
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
    let run_number = u32::try_from(max_run_number)
        .map_err(|_| StoreError::InvalidRecord("run number overflow".to_string()))?
        .checked_add(1)
        .ok_or_else(|| StoreError::InvalidRecord("run number overflow".to_string()))?;
    let now = database_timestamp(transaction).await?;
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
    store_summary(transaction, &next).await?;
    Ok(run)
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
    let assignment = load_assignment(transaction, tenant, enrollment.assignment).await?;
    validate_postgres_assignment_position(&assignment, &command)?;
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
        "SELECT qa.payload, qa.payload_sha256 FROM question_attempt AS qa \
         LEFT JOIN submission_idempotency AS si \
           ON si.tenant_id = qa.tenant_id AND si.attempt_id = qa.attempt_id \
         WHERE qa.tenant_id = $1 AND qa.run_id = $2 AND si.attempt_id IS NULL \
         ORDER BY qa.occurred_at DESC, qa.attempt_id::text DESC LIMIT 1",
    )
    .bind(tenant.as_uuid())
    .bind(run.id.as_uuid())
    .fetch_optional(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    if let Some(row) = unresolved {
        let active: QuestionAttempt = decode_payload_row(&row)?;
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
    let timer = issued_timer(issued_at, &run, question.question.timing_policy)?;
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
         (tenant_id, attempt_id, run_id, assignment_position, occurred_at, payload, payload_sha256) \
         VALUES ($1, $2, $3, $4, transaction_timestamp(), $5, $6)",
    )
    .bind(tenant.as_uuid())
    .bind(attempt.id.as_uuid())
    .bind(attempt.run.as_uuid())
    .bind(assignment_position)
    .bind(payload)
    .bind(checksum)
    .execute(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
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
    let row = sqlx::query("SELECT payload, payload_sha256 FROM question_attempt WHERE tenant_id = $1 AND attempt_id = $2 ORDER BY occurred_at LIMIT 1 FOR UPDATE")
        .bind(tenant.as_uuid()).bind(attempt.as_uuid()).fetch_optional(&mut **transaction).await.map_err(map_sqlx_error)?.ok_or(StoreError::NotFound)?;
    decode_payload_row(&row)
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
        "SELECT payload, payload_sha256 FROM question_attempt \
         WHERE tenant_id = $1 AND attempt_id = $2 ORDER BY occurred_at LIMIT 1 FOR UPDATE",
    )
    .bind(tenant.as_uuid())
    .bind(command.attempt.as_uuid())
    .fetch_optional(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?
    .ok_or(StoreError::NotFound)?;
    let base: QuestionAttempt = decode_payload_row(&attempt_row)?;
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
    let feedback = private_feedback_record(command.feedback.clone())?;

    let mut run = load_run_for_update(transaction, tenant, base.run).await?;
    if run.completed_at.is_some() || run.score.is_some() {
        return Err(StoreError::Conflict);
    }
    let mut enrollment = load_enrollment_for_update(transaction, tenant, run.enrollment).await?;
    let assignment = load_assignment(transaction, tenant, enrollment.assignment).await?;
    let question = load_published_record(transaction, base.problem, base.question_version).await?;
    crate::validate_attempt_result(command.result)?;
    let submitted_at = database_timestamp(transaction).await?;
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

    let previous = load_summary_for_update(transaction, tenant, enrollment.id).await?;
    let mut next = project_summary(
        &previous,
        domain::scoring::RunTransition::QuestionAttemptRecorded { at: submitted_at },
        grade_policy(&assignment),
    )?;
    let rows = sqlx::query(
        "SELECT COALESCE(si.payload, qa.payload) AS payload, \
                COALESCE(si.payload_sha256, qa.payload_sha256) AS payload_sha256 \
         FROM question_attempt AS qa \
         LEFT JOIN submission_idempotency AS si \
           ON si.tenant_id = qa.tenant_id AND si.attempt_id = qa.attempt_id \
         WHERE qa.tenant_id = $1 AND qa.run_id = $2",
    )
    .bind(tenant.as_uuid())
    .bind(run.id.as_uuid())
    .fetch_all(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    let attempts = rows
        .iter()
        .map(decode_payload_row)
        .collect::<Result<Vec<QuestionAttempt>, StoreError>>()?;
    let results = postgres_current_results(&attempts, &assignment, &submitted);
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
                &assignment,
                &results,
                &attempts,
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
    sqlx::query(
        "INSERT INTO grade_event \
         (tenant_id, grade_event_id, attempt_id, occurred_at, payload, payload_sha256) \
         VALUES ($1, $2, $2, transaction_timestamp(), $3, $4)",
    )
    .bind(tenant.as_uuid())
    .bind(submitted.id.as_uuid())
    .bind(grade_payload)
    .bind(grade_checksum)
    .execute(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;

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
fn validate_postgres_assignment_position(
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
fn postgres_current_results(
    attempts: &[QuestionAttempt],
    assignment: &AssignmentRecord,
    current: &QuestionAttempt,
) -> Vec<Option<AttemptResult>> {
    let mut latest: Vec<Option<(ActivityTimestamp, QuestionAttemptId, AttemptResult)>> =
        vec![None; assignment.problems.len()];
    for stored in attempts {
        let attempt = if stored.id == current.id {
            current
        } else {
            stored
        };
        let (Some(submitted_at), Some(result)) = (attempt.timer.submitted_at, attempt.result)
        else {
            continue;
        };
        let Ok(position) = usize::try_from(attempt.assignment_position) else {
            continue;
        };
        let Some(slot) = latest.get_mut(position) else {
            continue;
        };
        if slot
            .as_ref()
            .is_none_or(|(at, id, _)| (submitted_at, attempt.id) > (*at, *id))
        {
            *slot = Some((submitted_at, attempt.id, result));
        }
    }
    latest
        .into_iter()
        .map(|entry| entry.map(|(_, _, result)| result))
        .collect()
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
    store_summary(transaction, &next).await?;
    Ok(next)
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
    if !assignment.problems.iter().any(|reference| {
        reference.problem == attempt.problem && reference.version == attempt.question_version
    }) {
        return Err(StoreError::InvalidRecord(
            "question attempt must reference a version in its assignment".to_string(),
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
         (tenant_id, attempt_id, run_id, assignment_position, occurred_at, payload, payload_sha256) \
         VALUES ($1, $2, $3, $4, to_timestamp($5::double precision / 1000.0), $6, $7)",
    )
    .bind(tenant.as_uuid())
    .bind(attempt.id.as_uuid())
    .bind(attempt.run.as_uuid())
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
    for reference in &assignment.problems {
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
async fn insert_postgres_assignment_problems(
    transaction: &mut Transaction<'_, Postgres>,
    assignment: &AssignmentRecord,
) -> Result<(), StoreError> {
    for (position, reference) in assignment.problems.iter().enumerate() {
        let position = i32::try_from(position)
            .map_err(|_| StoreError::InvalidRecord("too many assignment problems".to_string()))?;
        sqlx::query(
            "INSERT INTO assignment_problem \
             (tenant_id, assignment_id, position, problem_id, version_id) \
             VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(assignment.tenant.as_uuid())
        .bind(assignment.id.as_uuid())
        .bind(position)
        .bind(reference.problem.as_uuid())
        .bind(reference.version.as_uuid())
        .execute(&mut **transaction)
        .await
        .map_err(map_sqlx_error)?;
    }
    Ok(())
}

#[cfg(feature = "postgres")]
async fn load_assignment(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    assignment: AssignmentId,
) -> Result<AssignmentRecord, StoreError> {
    let row = sqlx::query(
        "SELECT payload, payload_sha256 FROM assignment \
         WHERE tenant_id = $1 AND assignment_id = $2",
    )
    .bind(tenant.as_uuid())
    .bind(assignment.as_uuid())
    .fetch_optional(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?
    .ok_or(StoreError::NotFound)?;
    decode_payload_row(&row)
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
) -> Result<(), StoreError> {
    let backend = question_backend_name(QuestionBackend::from(&record.question.source));
    let (lifecycle, lifecycle_reason) = catalog_lifecycle_parts(&record.lifecycle);
    let derived_from_problem = record.derived_from.map(|source| source.problem.as_uuid());
    let derived_from_version = record.derived_from.map(|source| source.version.as_uuid());
    sqlx::query(
        "INSERT INTO problem_version \
         (problem_id, version_id, workspace_id, title, backend, capabilities, metadata, \
          publication_scope, lifecycle, lifecycle_reason, authors, previous_version_id, \
          derived_from_problem_id, derived_from_version_id) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)",
    )
    .bind(record.problem.as_uuid())
    .bind(record.version.as_uuid())
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
    if record.problem != stored_problem || record.version != stored_version {
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
    let version = VersionId::from_uuid(row.try_get("version_id").map_err(map_sqlx_error)?);
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
        version,
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
    let mut records = rows
        .iter()
        .map(|row| {
            let key = row
                .try_get::<String, _>("stable_key")
                .map_err(map_sqlx_error)?;
            let record = decode_payload_row(row)?;
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

#[cfg(all(test, feature = "postgres"))]
mod tests {
    use super::GRADEBOOK_SUMMARY_PAGE_SQL;

    const ASSIGNMENT_REVISION_MIGRATION: &str =
        include_str!("../../../schemas/migrations/20260808001700_assignment_revision.sql");

    const GRADEBOOK_MIGRATION: &str =
        include_str!("../../../schemas/migrations/20260808000300_gradebook_summary.sql");
    const PREFETCH_MIGRATION: &str =
        include_str!("../../../schemas/migrations/20260808001800_question_prefetch.sql");
    const STATISTICS_MIGRATION: &str =
        include_str!("../../../schemas/migrations/20260808002000_question_statistics.sql");
    const RETENTION_MIGRATION: &str =
        include_str!("../../../schemas/migrations/20260808002100_retention_foundation.sql");
    const RETENTION_WORKER_MIGRATION: &str =
        include_str!("../../../schemas/migrations/20260808002200_retention_worker.sql");
    const RETENTION_LIFECYCLE_MIGRATION: &str =
        include_str!("../../../schemas/migrations/20260808002300_retention_lifecycle.sql");
    const RETENTION_API_MIGRATION: &str =
        include_str!("../../../schemas/migrations/20260808002400_retention_api.sql");
    const RETENTION_ARCHIVE_ACCESS_MIGRATION: &str =
        include_str!("../../../schemas/migrations/20260808002500_retention_archive_access.sql");

    #[test]
    fn retention_archive_access_migration_is_registered_and_fenced() {
        assert!(
            super::MIGRATIONS
                .iter()
                .any(|(version, _)| *version == 20260808002500)
        );
        let migration = RETENTION_ARCHIVE_ACCESS_MIGRATION.to_ascii_lowercase();
        for required in [
            "create function ple_course_records_accessible",
            "p_tenant is distinct from public.ple_current_tenant()",
            "from public.course",
            "lifecycle in ('archived', 'deleted')",
            "generation = current_generation",
            "stage = 'archivestudentrecords'",
            "state = 'started'",
            "security definer",
            "set search_path = pg_catalog, public",
            "grant execute on function ple_course_records_accessible",
            "drop policy if exists assignment_tenant",
            "student_assignment_summary",
            "assignment_run",
            "question_attempt",
            "submission",
            "attempt_feedback",
            "question_prefetch",
            "external_tool_exchange",
            "student_export_request",
            "asset_delivery has no course column",
            "join public.course_retention_dispatch",
            "s.state='started'",
            "s.job_id=p_job",
            "s.lease_token=p_token",
            "set lifecycle='archived'",
        ] {
            assert!(
                migration.contains(required),
                "missing R4.3 guard: {required}"
            );
        }
        assert!(GRADEBOOK_SUMMARY_PAGE_SQL.contains("ple_course_records_accessible"));
        for forbidden in [
            "delete from public.question_attempt",
            "delete from public.submission",
            "grant delete on",
        ] {
            assert!(
                !migration.contains(forbidden),
                "R4.3 must not broaden destructive scope: {forbidden}"
            );
        }
    }

    #[test]
    fn retention_lifecycle_migration_dispatches_only_bound_due_current_stages() {
        assert!(
            super::MIGRATIONS
                .iter()
                .any(|(version, _)| *version == 20260808002300)
        );
        let migration = RETENTION_LIFECYCLE_MIGRATION.to_ascii_lowercase();
        for required in [
            "create table course_retention_dispatch",
            "foreign key (tenant_id, course_id, stage, generation)",
            "foreign key (job_id) references worker_job(job_id) deferrable initially deferred",
            "enable row level security",
            "force row level security",
            "create function ple_dispatch_due_retention_stages",
            "for update of s, r skip locked",
            "s.due_at <= transaction_timestamp()",
            "r.generation=s.generation",
            "jsonb_build_object('kind','retention'",
            "create or replace function ple_prepare_retention_work",
            "join public.course_retention_dispatch d",
            "create or replace function ple_commit_retention_work",
            "create function ple_extend_course_retention",
            "create function ple_set_archive_disposition",
            "w.state in ('ready','leased')",
            "stage='archivestudentrecords'",
            "errcode = '42501'",
            "security definer set search_path = pg_catalog, public",
            "revoke all on course_retention_dispatch from public, ple_app",
        ] {
            assert!(
                migration.contains(required),
                "missing R4.1 guard: {required}"
            );
        }
        for forbidden in [
            "question_statistics_aggregate",
            "problem_version_payload",
            "bucket-prefix",
            "payload->>'objectkey'",
        ] {
            assert!(
                !migration.contains(forbidden),
                "R4.1 must not broaden dispatch into {forbidden}"
            );
        }
    }

    #[test]
    fn retention_api_migration_is_cas_safe_and_reuses_closed_dispatch() {
        assert!(
            super::MIGRATIONS
                .iter()
                .any(|(version, _)| *version == 20260808002400)
        );
        let migration = RETENTION_API_MIGRATION.to_ascii_lowercase();
        for required in [
            "create function ple_read_retention_notification",
            "create function ple_apply_retention_api_action",
            "create table course_retention_api_receipt",
            "primary key (tenant_id, course_id, expected_generation)",
            "replay.actor_id<>actor or replay.action<>p_action",
            "replay.assignment_disposition is distinct from p_disposition",
            "resulting_generation",
            "p_expected_generation",
            "lifecycle='active' for update",
            "if current.generation<>p_expected_generation then return null",
            "order by n.generation desc, n.created_at desc",
            "returns table (intent text, created_at_millis bigint)",
            "return 'inprogress'",
            "return 'completed'",
            "from public.course_retention_dispatch d",
            "return 'scheduled'",
            "current.assignment_disposition is distinct from p_disposition",
            "return case when immediate_stage is null then 'changed' else 'scheduled' end",
            "ple_retention_authorize(p_session, p_course, true)",
            "course_retention_dispatch",
            "jsonb_build_object('kind','retention'",
            "security definer set search_path = pg_catalog, public",
            "errcode = '42501'",
            "revoke all on function ple_read_retention_notification",
        ] {
            assert!(
                migration.contains(required),
                "missing R4.2 guard: {required}"
            );
        }
        for forbidden in ["object_payload", "student_export_artifact", "delete from"] {
            assert!(
                !migration.contains(forbidden),
                "R4.2 request boundary must not perform purge work: {forbidden}"
            );
        }
        assert!(
            migration
                .find("lifecycle='active' for update")
                .expect("course lock")
                < migration
                    .find("select * into replay")
                    .expect("receipt lookup"),
            "the course lock must serialize concurrent retries before receipt lookup"
        );
    }

    #[test]
    fn retention_worker_migration_is_registered_and_broker_only() {
        assert!(
            super::MIGRATIONS
                .iter()
                .any(|(version, _)| *version == 20260808002200)
        );
        let migration = RETENTION_WORKER_MIGRATION.to_ascii_lowercase();
        for required in [
            "create table course_retention_notification",
            "drop constraint worker_job_payload_check",
            "payload->>'kind'='retention'",
            "alter table course_retention_stage",
            "create function ple_prepare_retention_work",
            "create function ple_commit_retention_work",
            "security definer set search_path = pg_catalog, public",
            "delete from public.asset_delivery",
            "invalid student-record retention manifest",
            "a.object_payload->>'bucket' <> 'student-records'",
            "worker_job w",
            "lease_token=p_token",
            "s.due_at <= transaction_timestamp()",
            "for update of w",
            "s.state='started' and s.job_id=p_job",
            "to ple_retention_broker",
            "retention_broker_worker_job",
            "retention_broker_export_request",
            "retention_broker_export_artifact",
            "retention_broker_asset_delivery",
            "grant select, delete on asset_delivery to ple_retention_broker",
            "revoke all on course_retention_notification",
        ] {
            assert!(
                migration.contains(required),
                "missing retention worker guard: {required}"
            );
        }
        for forbidden in [
            "question_statistics_aggregate",
            "question_attempt",
            "submission",
            "bucket-prefix",
        ] {
            assert!(
                !migration.contains(forbidden),
                "worker migration must not broaden retention scope: {forbidden}"
            );
        }
    }

    #[test]
    fn retention_migration_is_registered_tenant_scoped_and_non_destructive() {
        assert!(
            super::MIGRATIONS
                .iter()
                .any(|(version, _)| *version == 20260808002100)
        );
        let migration = RETENTION_MIGRATION.to_ascii_lowercase();
        for required in [
            "create table institution_retention_policy",
            "create table course_retention",
            "create table course_retention_stage",
            "generation bigint not null check (generation > 0)",
            "course_retention_stage_due_idx",
            "enable row level security",
            "force row level security",
            "create function ple_retention_authorize",
            "perform set_config('ple.session_hash', p_session, true)",
            "language sql volatile security definer set search_path = pg_catalog, public",
            "where public.ple_retention_authorize(p_session, p_course, false)",
            "revoke all on institution_retention_policy, course_retention, course_retention_stage",
        ] {
            assert!(
                migration.contains(required),
                "missing retention guard: {required}"
            );
        }
        for forbidden in [
            "question_statistics_aggregate",
            "question_attempt",
            "submission",
            "grade_event",
            "grant delete",
        ] {
            assert!(
                !migration.contains(forbidden),
                "retention foundation must not expose destructive history authority: {forbidden}"
            );
        }
    }

    #[test]
    fn statistics_migration_is_registered_private_and_retention_safe() {
        let versions: Vec<_> = super::MIGRATIONS
            .iter()
            .map(|(version, _)| *version)
            .collect();
        assert!(versions.windows(2).all(|pair| pair[0] < pair[1]));
        assert!(versions.contains(&20260808002000));
        let normalized = STATISTICS_MIGRATION.to_ascii_lowercase();
        for required in [
            "create role ple_statistics_broker",
            "nologin",
            "nobypassrls",
            "create table question_statistics_aggregate",
            "primary key (problem_id, version_id)",
            "create table question_statistics_contribution_receipt",
            "observation_sha256 bytea not null",
            "on delete cascade",
            "enable row level security",
            "force row level security",
            "question_statistics_contribution_receipt_tenant",
            "question_statistics_contribution_receipt_broker",
            "security definer set search_path = pg_catalog, public",
            "ple_record_question_statistics",
            "ple_question_statistics_view",
            "aggregate.cohort_size >= 5",
            "problem_version_statistics_visible_select",
            "assignment_run_statistics_broker",
            "question_attempt_statistics_broker",
            "earlier_run.run_number < run.run_number",
            "run.payload->>'mode' = 'assigned'",
            "first_completed_run_id",
            "ple_statistics_aggregate_valid",
            "not public.ple_statistics_canonical_float(p_score)",
            "not public.ple_statistics_canonical_float(p_rest_score)",
            "paired_score_sum",
        ] {
            assert!(normalized.contains(required), "missing {required}");
        }
        for forbidden in [
            "grant select on question_statistics_aggregate to ple_app",
            "grant select on question_statistics_aggregate to ple_student",
            "grant select on question_statistics_aggregate to ple_grader",
            "grant select on question_statistics_aggregate to ple_qti_grader",
            "grant select on question_statistics_aggregate to ple_queue_broker",
            "unique (tenant_id, attempt_id)",
        ] {
            assert!(
                !normalized.contains(forbidden),
                "statistics migration must not expose or over-constrain: {forbidden}"
            );
        }
    }

    #[test]
    fn prefetch_migration_is_registered_ordered_and_tenant_hardened() {
        let versions: Vec<_> = super::MIGRATIONS
            .iter()
            .map(|(version, _)| *version)
            .collect();
        assert!(versions.windows(2).all(|pair| pair[0] < pair[1]));
        assert!(versions.contains(&20260808001800));
        let normalized = PREFETCH_MIGRATION.to_ascii_lowercase();
        for required in [
            "create table question_prefetch",
            "create table submission_next_attempt",
            "enable row level security",
            "force row level security",
            "question_prefetch_tenant",
            "submission_next_attempt_tenant",
            "predecessor_occurred_at",
            "next_attempt_occurred_at",
            "references question_attempt",
            "references submission_idempotency",
            "grant select, insert, delete on question_prefetch to ple_app",
            "grant select, insert on submission_next_attempt to ple_app",
        ] {
            assert!(normalized.contains(required), "missing {required}");
        }
        assert!(!normalized.contains("update on question_prefetch"));
    }

    #[test]
    fn gradebook_page_query_stays_on_compact_projection_tables() {
        let normalized = GRADEBOOK_SUMMARY_PAGE_SQL.to_ascii_lowercase();
        for forbidden in [
            "assignment_run",
            "question_attempt",
            "submission",
            "grade_event",
            "count(",
            "sum(",
            "avg(",
        ] {
            assert!(
                !normalized.contains(forbidden),
                "gradebook query must not scan history or aggregate it: {forbidden}"
            );
        }
        for required in [
            "assignment as a",
            "enrollment as e",
            "student_assignment_summary as sas",
            "(a.assignment_id, e.enrollment_id) > ($3, $4)",
            "order by a.assignment_id, e.enrollment_id",
            "limit $5",
        ] {
            assert!(normalized.contains(required));
        }
        for forbidden in [
            "assignment_id::text",
            "enrollment_id::text",
            "|| '/' ||",
            "order by a.assignment_id::text",
        ] {
            assert!(
                !normalized.contains(forbidden),
                "gradebook cursor and order must stay index-aligned: {forbidden}"
            );
        }
    }

    #[test]
    fn catalog_statistics_queries_read_only_visible_catalog_and_safe_aggregates() {
        let source = include_str!("postgres.rs");
        let start = source
            .find("    async fn search_catalog(")
            .expect("catalog search implementation");
        let end = source[start..]
            .find("    async fn get_catalog_detail(")
            .map(|offset| start + offset)
            .expect("catalog detail follows search");
        let search = source[start..end].to_ascii_lowercase();
        for required in [
            "left join lateral ple_question_statistics_view",
            "statistics.cohort_size is not null",
            "statistics.cohort_size is null",
        ] {
            assert!(
                search.contains(required),
                "missing safe catalog statistics shape: {required}"
            );
        }
        assert_eq!(
            search
                .matches("left join lateral ple_question_statistics_view")
                .count(),
            search
                .matches("$5::smallint <> 1 or statistics.cohort_size is not null")
                .count(),
            "every availability-filtered catalog SQL block must use the safe lateral reader"
        );
        for forbidden in [
            "question_attempt",
            "submission",
            "grade_event",
            "assignment_run",
            "feedback",
            "problem_version_payload",
            " from snapshot",
            " join snapshot",
        ] {
            assert!(
                !search.contains(forbidden),
                "catalog statistics must not scan private history: {forbidden}"
            );
        }
    }

    #[test]
    fn gradebook_page_index_matches_the_native_cursor_tuple() {
        let normalized = GRADEBOOK_MIGRATION.to_ascii_lowercase();
        assert!(normalized.contains("enrollment_gradebook_summary_page_idx"));
        assert!(normalized.contains("on enrollment (tenant_id, assignment_id, enrollment_id)"));
    }

    #[test]
    fn assignment_revision_migration_is_forward_only_and_positive() {
        let normalized = ASSIGNMENT_REVISION_MIGRATION.to_ascii_lowercase();
        assert!(normalized.contains("add column revision bigint not null default 1"));
        assert!(normalized.contains("check (revision > 0)"));
    }
}
