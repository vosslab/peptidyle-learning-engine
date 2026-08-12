//! PostgreSQL backend, embedded migrations, and connection handling.
//!
//! Every operation runs as the non-bypassing `ple_app` role. Tenant-owned
//! operations also set `ple.tenant_id` locally inside their transaction, so a
//! pooled connection cannot retain another request's tenant context.

#[cfg(feature = "postgres")]
use std::collections::{BTreeMap, BTreeSet};

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
    ActivityTimestamp, AssignmentDeliveryState, AssignmentEnrollment, AssignmentId, AssignmentItem,
    AssignmentItemId, AssignmentPolicyExceptionId, AssignmentRun, AssignmentRunItem,
    AssignmentScoringMode, AssignmentSelectionCandidate, AssignmentSelectionGroup,
    AssignmentSelectionGroupId, AssignmentTimingPolicy, AttemptResult, AttemptStatus,
    BackendCapabilities, CatalogCapabilityFacet, CatalogLicenseFacet, CatalogLifecycle,
    CatalogProblemSummary, CatalogSearchQuery, CatalogTaxonomyFacet, CourseGroupId, CourseId,
    CourseMembership, CourseMembershipRole, CourseRole, CourseSummary, EnrollmentId,
    EnrollmentStatus, LateSubmissionPolicy, PointValue, PresentationBindingV1,
    PresentationDigestV1, PresentationNonceV1, ProblemId, ProblemPublicId, ProblemVersionNumber,
    ProblemVersionRef, PublicationScope, QuestionAttempt, QuestionAttemptId, QuestionBackend,
    QuestionMetadata, QuestionStatisticsDisclosure, QuestionStatisticsView, RunId, RunMode,
    ScoringGeneration, ScoringStatus, SelectionOrdering, StudentAssignmentSummary, StudentId,
    StudentResponse, TenantId, UserId, VersionId, WorkspaceDraftSummary, WorkspaceId,
    WorkspaceImportId,
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
use sqlx::postgres::{PgPool, PgRow};
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
    ActivityTransition, AssetDeliveryRecord, AssetDeliveryScope, AssignmentDefinitionDisposition,
    AssignmentEditorUpdate, AssignmentPolicyExceptionTarget, AssignmentRecord, AssignmentRevision,
    AttemptFeedbackRecord, AttemptSupportAction, AttemptSupportActionId, AttemptSupportRecord,
    ClearAttemptCommand, CourseGroupRecord, CourseGroupRevision, CourseListScope, CourseRecord,
    CourseRecordsAccessStore, CourseRetentionRecord, CourseRetentionSnapshot, CourseRetentionState,
    CourseRetentionView, Cursor, DeleteAndRegradeAssignmentItemCommand,
    DeleteAssignmentPolicyExceptionCommand, DraftRecord, FeedbackReleaseRecord,
    ForceSubmitAttemptCommand, InstitutionRetentionPolicy, IssueQuestionAttemptCommand, Page,
    PageRequest, PageSize, PrefetchedQuestion, PublishedProblemRecord, PublishedSourceArtifact,
    PutCourseGroupCommand, ReleaseAttemptFeedbackCommand, ReservePrefetchedQuestionCommand,
    ResolvedAssignmentTiming, ResolvedAttemptTiming, RetentionApiStore, RetentionCleanupManifest,
    RetentionDays, RetentionDispatchBatch, RetentionRevision, RetentionScheduleStore,
    RetentionStore, RetentionWork, RetentionWorkerCommand, RetentionWorkerStore,
    RunSummaryOutcomeInput, RunSummaryPageInput, SetAssignmentPolicyExceptionCommand, Store,
    StoreError, StoredAssignment, StoredAssignmentPolicyException, StoredAssignmentTiming,
    StoredCourseGroup, SubmissionIdempotencyKey, SubmissionNextAttempt, SubmissionRecord,
    SubmitQuestionAttemptCommand, TenantContext, UpdateAssignmentTimingCommand, WorkspaceDraft,
    WorkspaceDraftRevision, assignment_scoring_changed, completed_run_score, current_run_questions,
    decode_workspace_draft_cursor, delete_and_regrade_update, encode_workspace_draft_cursor,
    ensure_tenant, grade_policy, private_feedback_record, project_enrollment_completion,
    select_assignment_run_items, summary_transition, validate_asset_delivery, validate_assignment,
    validate_assignment_policy_exception, validate_assignment_timing, validate_course,
    validate_course_group, validate_draft, validate_published, validate_qti_import,
};

#[cfg(feature = "postgres")]
mod manual_grading;
#[cfg(feature = "postgres")]
use crate::{
    ClaimedJob, EnqueueJob, JobFailureDisposition, JobFailureKind, JobId, JobLeaseDuration,
    JobLeaseToken, JobPayload, JobState, JobStore, QueueDepth, TenantJobView,
};
#[cfg(feature = "postgres")]
use crate::{
    CommitPreparedQtiImport, CommitPreparedQtiImportOutcome, CreateQtiImportCommand,
    QtiGradingStore, QtiImportGradingPayload, QtiImportRegistry, QtiImportStore,
};

#[cfg(feature = "postgres")]
mod activity;
#[cfg(feature = "postgres")]
mod row_decode;
#[cfg(feature = "postgres")]
use row_decode::*;
#[cfg(feature = "postgres")]
mod assignment_values;
#[cfg(feature = "postgres")]
use assignment_values::*;
#[cfg(feature = "postgres")]
mod assignment_records;
#[cfg(feature = "postgres")]
use assignment_records::*;
#[cfg(feature = "postgres")]
mod transaction_context;
#[cfg(feature = "postgres")]
use transaction_context::*;
#[cfg(feature = "postgres")]
mod feedback_data;
#[cfg(feature = "postgres")]
use feedback_data::*;
#[cfg(feature = "postgres")]
mod submission;
#[cfg(feature = "postgres")]
use submission::*;
#[cfg(feature = "postgres")]
mod run_lifecycle;
#[cfg(feature = "postgres")]
use run_lifecycle::*;
#[cfg(feature = "postgres")]
mod account_identity;
#[cfg(feature = "postgres")]
mod assets;
#[cfg(feature = "postgres")]
mod assignment_timing;
#[cfg(feature = "postgres")]
mod authoring;
#[cfg(feature = "postgres")]
mod catalog;
#[cfg(feature = "postgres")]
mod connection;
#[cfg(feature = "postgres")]
mod course_appearance;
#[cfg(feature = "postgres")]
mod course_assignments;
#[cfg(feature = "postgres")]
mod course_policy;
#[cfg(feature = "postgres")]
mod course_roster;
#[cfg(feature = "postgres")]
mod course_roster_decode;
#[cfg(feature = "postgres")]
mod courses;
#[cfg(feature = "postgres")]
mod exports;
#[cfg(feature = "postgres")]
mod external_tool;
#[cfg(feature = "postgres")]
mod feedback;
#[cfg(feature = "postgres")]
mod flat_import_provenance;
#[cfg(feature = "postgres")]
mod flat_question;
#[cfg(feature = "postgres")]
mod item_analysis;
#[cfg(feature = "postgres")]
mod jobs;
#[cfg(feature = "postgres")]
mod manual_grade_export;
#[cfg(feature = "postgres")]
mod migrations;
#[cfg(feature = "postgres")]
mod qti;
#[cfg(feature = "postgres")]
mod qti_ingress;
#[cfg(feature = "postgres")]
mod retention;
#[cfg(feature = "postgres")]
mod runs;
#[cfg(feature = "postgres")]
mod sessions;
#[cfg(feature = "postgres")]
mod statistics;
#[cfg(feature = "postgres")]
pub use connection::lazy_pool;
#[cfg(feature = "postgres")]
use connection::{connect_grader_pool, map_sqlx_error, retry_transaction};
#[cfg(feature = "postgres")]
pub use migrations::{
    MigrationDisposition, MigrationStatus, MigrationStatusEntry, SchemaCompatibilityError,
    apply_migrations, migration_principal, migration_status, migration_status_from_directory,
    verify_application_schema,
};

#[cfg(feature = "postgres")]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
struct AttemptSupportAuditPayload {
    previous_status: AttemptStatus,
    resulting_status: AttemptStatus,
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
pub struct PostgresGraderStore {
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
}

#[cfg(feature = "postgres")]
impl PostgresGraderStore {
    /// Connects using the dedicated, least-privilege QTI grader credential.
    ///
    /// The application pool is deliberately not accepted here. Deployment
    /// provisions the password and provides this URL only to server grading
    /// composition when QTI grading is enabled.
    pub async fn connect(database_url: &str) -> Result<Self, StoreError> {
        let pool = connect_grader_pool(database_url)
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
        if current_user != "ple_grading_reader"
            || session_user != "ple_grading_reader"
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
pub(super) fn question_statistics_disclosure_from_row(
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
