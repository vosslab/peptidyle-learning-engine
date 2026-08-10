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
    AssignmentSelectionGroupId, AttemptResult, AttemptStatus, AttemptTimerRecord,
    BackendCapabilities, CatalogCapabilityFacet, CatalogLicenseFacet, CatalogLifecycle,
    CatalogProblemSummary, CatalogSearchQuery, CatalogTaxonomyFacet, CourseGroupId, CourseId,
    CourseMembership, CourseMembershipRole, CourseRole, CourseSummary, EnrollmentId,
    EnrollmentStatus, LateSubmissionPolicy, PointValue, ProblemId, ProblemPublicId,
    ProblemVersionNumber, ProblemVersionRef, PublicationScope, QuestionAttempt, QuestionAttemptId,
    QuestionBackend, QuestionMetadata, QuestionStatisticsDisclosure, QuestionStatisticsView, RunId,
    RunMode, ScoringGeneration, ScoringStatus, SelectionOrdering, StudentAssignmentSummary,
    StudentId, StudentResponse, TenantId, UserId, VersionId, WorkspaceDraftSummary, WorkspaceId,
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
    AssignmentPolicyExceptionTarget, AssignmentRecord, AssignmentRevision, AssignmentUpdate,
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
mod assets;
#[cfg(feature = "postgres")]
mod assignment_timing;
#[cfg(feature = "postgres")]
mod catalog;
#[cfg(feature = "postgres")]
mod connection;
#[cfg(feature = "postgres")]
mod course_appearance;
#[cfg(feature = "postgres")]
mod exports;
#[cfg(feature = "postgres")]
mod external_tool;
#[cfg(feature = "postgres")]
mod flat_import_provenance;
#[cfg(feature = "postgres")]
mod flat_question;
#[cfg(feature = "postgres")]
mod item_analysis;
#[cfg(feature = "postgres")]
mod jobs;
#[cfg(feature = "postgres")]
mod migrations;
#[cfg(feature = "postgres")]
mod qti;
#[cfg(feature = "postgres")]
mod qti_ingress;
#[cfg(feature = "postgres")]
mod retention;
#[cfg(feature = "postgres")]
mod sessions;
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
        let expected_revision_value = i64::try_from(expected_revision.value()).map_err(|_| {
            StoreError::Unavailable("workspace draft revision limit reached".to_string())
        })?;
        let authorized: bool =
            sqlx::query_scalar("SELECT ple_delete_draft_qti_jobs($1, $2, $3, $4)")
                .bind(context.tenant_id().as_uuid())
                .bind(workspace.as_uuid())
                .bind(actor.as_uuid())
                .bind(expected_revision_value)
                .fetch_one(&mut *transaction)
                .await
                .map_err(map_sqlx_error)?;
        let deleted = if authorized {
            sqlx::query_scalar::<_, Uuid>(
                "DELETE FROM workspace_draft AS d USING workspace_draft_access AS a \
                 WHERE d.tenant_id = $1 AND d.workspace_id = $2 AND d.revision = $4 \
                   AND a.tenant_id = d.tenant_id AND a.workspace_id = d.workspace_id \
                   AND a.user_id = $3 AND a.role = 'owner' \
                 RETURNING d.workspace_id",
            )
            .bind(context.tenant_id().as_uuid())
            .bind(workspace.as_uuid())
            .bind(actor.as_uuid())
            .bind(expected_revision_value)
            .fetch_optional(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?
        } else {
            None
        };
        if deleted.is_some() {
            transaction.commit().await.map_err(map_sqlx_error)?;
            return Ok(true);
        }

        // The capability and delete predicates above are the authoritative
        // atomic decision. This follow-up only classifies its safe
        // non-mutating failure while preserving absent/foreign non-enumeration.
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
        retry_transaction(|| {
            let course = course.clone();
            async move {
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
                    assignment_timing::lock_postgres_assignment_policy(
                        &mut transaction,
                        tenant,
                        assignment,
                    )
                    .await?;
                    locked.push((
                        assignment,
                        assignment_timing::lock_postgres_active_timing_rows(
                            &mut transaction,
                            tenant,
                            assignment,
                        )
                        .await?,
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
                    assignment_timing::apply_postgres_locked_timing_rows(
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
        })
        .await
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
        retry_transaction(|| {
            let command = command.clone();
            async move {
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
                    let members = assignment_timing::load_postgres_course_group_members(
                        &mut transaction,
                        tenant,
                        command.record.id,
                    )
                    .await?;
                    Some(StoredCourseGroup {
                        record: CourseGroupRecord {
                            id: command.record.id,
                            tenant,
                            course: CourseId::from_uuid(
                                row.try_get("course_id").map_err(map_sqlx_error)?,
                            ),
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
                    assignment_timing::lock_postgres_assignment_policy(
                        &mut transaction,
                        tenant,
                        *assignment,
                    )
                    .await?;
                    locked.push((
                        *assignment,
                        assignment_timing::lock_postgres_active_timing_rows(
                            &mut transaction,
                            tenant,
                            *assignment,
                        )
                        .await?,
                    ));
                }
                let revision_i64 =
                    i64::try_from(revision.value()).map_err(|_| StoreError::Conflict)?;
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
                    assignment_timing::apply_postgres_locked_timing_rows(
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
        })
        .await
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
                    members: assignment_timing::load_postgres_course_group_members(
                        &mut transaction,
                        tenant,
                        group,
                    )
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
            .map(|row| assignment_timing::decode_stored_assignment_timing(row, context.tenant_id()))
            .transpose()?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(result)
    }

    async fn update_assignment_timing(
        &self,
        context: TenantContext,
        command: UpdateAssignmentTimingCommand,
    ) -> Result<StoredAssignmentTiming, StoreError> {
        retry_transaction(|| async move {
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
        let current = assignment_timing::decode_stored_assignment_timing(&row, tenant)?;
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
            let locked = assignment_timing::decode_stored_assignment_timing(&locked, tenant)?;
            if locked.policy != command.policy {
                return Err(StoreError::Conflict);
            }
            transaction.commit().await.map_err(map_sqlx_error)?;
            return Ok(locked);
        }
        if current.revision != command.expected_revision {
            return Err(StoreError::Conflict);
        }
        assignment_timing::lock_postgres_assignment_policy(&mut transaction, tenant, command.assignment).await?;
        let active_rows =
            assignment_timing::lock_postgres_active_timing_rows(&mut transaction, tenant, command.assignment).await?;
        let locked =
            assignment_timing::load_postgres_assignment_timing(&mut transaction, tenant, command.assignment, true)
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
        assignment_timing::apply_postgres_locked_timing_rows(
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
        .bind(assignment_timing::late_submission_policy_name(command.policy.late_submission))
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
        })
        .await
    }

    async fn set_assignment_policy_exception(
        &self,
        context: TenantContext,
        command: SetAssignmentPolicyExceptionCommand,
    ) -> Result<StoredAssignmentPolicyException, StoreError> {
        retry_transaction(|| {
            let command = command.clone();
            async move {
                validate_assignment_policy_exception(&command.exception)?;
                let tenant = context.tenant_id();
                let mut transaction = self.begin_tenant(context).await?;
                if let AssignmentPolicyExceptionTarget::CourseGroup(group) =
                    command.exception.target
                {
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
                assignment_timing::lock_postgres_assignment_policy(
                    &mut transaction,
                    tenant,
                    command.assignment,
                )
                .await?;
                let current = assignment_timing::load_postgres_assignment_timing(
                    &mut transaction,
                    tenant,
                    command.assignment,
                    false,
                )
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
                if let AssignmentPolicyExceptionTarget::Student(student) = command.exception.target
                {
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
                let rows = assignment_timing::load_postgres_policy_exception_identity_rows(
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
                    .map(assignment_timing::decode_postgres_policy_exception)
                    .transpose()?;
                if let Some(existing) = &existing {
                    if existing.id != command.exception.id
                        || existing.target != command.exception.target
                    {
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
                let active_rows = assignment_timing::lock_postgres_active_timing_rows(
                    &mut transaction,
                    tenant,
                    command.assignment,
                )
                .await?;
                let locked = assignment_timing::load_postgres_assignment_timing(
                    &mut transaction,
                    tenant,
                    command.assignment,
                    true,
                )
                .await?
                .ok_or(StoreError::NotFound)?;
                if locked.revision != command.expected_revision || locked.course != command.course {
                    return Err(StoreError::Conflict);
                }
                let (available_mode, available_at) =
                    assignment_timing::postgres_exception_timestamp_columns(
                        command.exception.available_at,
                    );
                let (closes_mode, closes_at) =
                    assignment_timing::postgres_exception_timestamp_columns(
                        command.exception.closes_at,
                    );
                let (time_limit_mode, time_limit_seconds) =
                    assignment_timing::postgres_exception_limit_columns(
                        command.exception.time_limit_seconds,
                    );
                let (attempt_limit_mode, attempt_limit) =
                    assignment_timing::postgres_exception_limit_columns(
                        command.exception.attempt_limit,
                    );
                let (student_id, course_group_id) = match command.exception.target {
                    AssignmentPolicyExceptionTarget::Student(student) => {
                        (Some(student.as_uuid()), None)
                    }
                    AssignmentPolicyExceptionTarget::CourseGroup(group) => {
                        (None, Some(group.as_uuid()))
                    }
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
                assignment_timing::update_postgres_assignment_revision(
                    &mut transaction,
                    tenant,
                    command.assignment,
                    locked.revision,
                    revision,
                )
                .await?;
                let now = database_timestamp(&mut transaction).await?;
                assignment_timing::apply_postgres_locked_timing_rows(
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
        })
        .await
    }

    async fn delete_assignment_policy_exception(
        &self,
        context: TenantContext,
        command: DeleteAssignmentPolicyExceptionCommand,
    ) -> Result<AssignmentRevision, StoreError> {
        retry_transaction(|| async move {
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
            let initial = assignment_timing::decode_postgres_policy_exception(&initial_row)?;
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
            assignment_timing::lock_postgres_assignment_policy(
                &mut transaction,
                tenant,
                command.assignment,
            )
            .await?;
            let current = assignment_timing::load_postgres_assignment_timing(
                &mut transaction,
                tenant,
                command.assignment,
                false,
            )
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
            let active_rows = assignment_timing::lock_postgres_active_timing_rows(
                &mut transaction,
                tenant,
                command.assignment,
            )
            .await?;
            let locked = assignment_timing::load_postgres_assignment_timing(
                &mut transaction,
                tenant,
                command.assignment,
                true,
            )
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
            assignment_timing::update_postgres_assignment_revision(
                &mut transaction,
                tenant,
                command.assignment,
                locked.revision,
                revision,
            )
            .await?;
            let now = database_timestamp(&mut transaction).await?;
            assignment_timing::apply_postgres_locked_timing_rows(
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
        })
        .await
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
            let timing = assignment_timing::load_postgres_assignment_timing(
                &mut transaction,
                tenant,
                assignment,
                false,
            )
            .await?
            .ok_or(StoreError::NotFound)?;
            Some(StoredAssignmentPolicyException {
                exception: assignment_timing::decode_postgres_policy_exception(&row)?,
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
        let enrollment = assignment_timing::load_postgres_enrollment_by_student(
            &mut transaction,
            tenant,
            assignment,
            student,
        )
        .await?;
        let Some(enrollment) = enrollment else {
            transaction.commit().await.map_err(map_sqlx_error)?;
            return Ok(None);
        };
        let timing = assignment_timing::load_postgres_assignment_timing(
            &mut transaction,
            tenant,
            assignment,
            false,
        )
        .await?
        .ok_or(StoreError::NotFound)?;
        let resolved = assignment_timing::load_postgres_resolved_assignment_policy(
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
            .map(|row| assignment_timing::decode_postgres_resolved_attempt_timing(row, attempt))
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
        retry_transaction(|| async move {
            let mut transaction = self.begin_tenant(context).await?;
            let run =
                start_or_resume_run(&mut transaction, context, actor, assignment, proposed_run)
                    .await?;
            transaction.commit().await.map_err(map_sqlx_error)?;
            Ok(run)
        })
        .await
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
        retry_transaction(|| {
            let command = command.clone();
            async move {
                let mut transaction = self.begin_tenant(context).await?;
                let attempt =
                    issue_or_resume_question_attempt(&mut transaction, context, command).await?;
                transaction.commit().await.map_err(map_sqlx_error)?;
                Ok(attempt)
            }
        })
        .await
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
        // Successor receipts are immutable. The primary key serializes
        // concurrent finalizers without requiring an UPDATE grant solely for
        // SELECT FOR UPDATE; a losing insert accepts only the exact receipt.
        let inserted = match next {
            Some(next) => {
                sqlx::query("INSERT INTO submission_next_attempt (tenant_id, predecessor_attempt_id, next_attempt_id, next_attempt_occurred_at) SELECT $1, $2, $3, next_attempt.occurred_at FROM question_attempt next_attempt JOIN question_attempt predecessor_attempt ON predecessor_attempt.tenant_id = next_attempt.tenant_id AND predecessor_attempt.run_id = next_attempt.run_id WHERE next_attempt.tenant_id = $1 AND next_attempt.attempt_id = $3 AND predecessor_attempt.attempt_id = $2 ON CONFLICT (tenant_id, predecessor_attempt_id) DO NOTHING")
                    .bind(context.tenant_id().as_uuid()).bind(predecessor.as_uuid()).bind(next.as_uuid()).execute(&mut *transaction).await.map_err(map_sqlx_error)?
            }
            None => {
                sqlx::query("INSERT INTO submission_next_attempt (tenant_id, predecessor_attempt_id, next_attempt_id, next_attempt_occurred_at) VALUES ($1, $2, NULL, NULL) ON CONFLICT (tenant_id, predecessor_attempt_id) DO NOTHING")
                .bind(context.tenant_id().as_uuid()).bind(predecessor.as_uuid()).execute(&mut *transaction).await.map_err(map_sqlx_error)?
            }
        };
        if inserted.rows_affected() == 0 {
            let existing: Option<Option<Uuid>> = sqlx::query_scalar("SELECT next_attempt_id FROM submission_next_attempt WHERE tenant_id = $1 AND predecessor_attempt_id = $2")
                .bind(context.tenant_id().as_uuid()).bind(predecessor.as_uuid()).fetch_optional(&mut *transaction).await.map_err(map_sqlx_error)?;
            if existing != Some(next.map(|value| value.as_uuid())) {
                return Err(StoreError::Conflict);
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
                    evaluation.payload AS evaluation_payload, \
                    evaluation.payload_sha256 AS evaluation_payload_sha256, \
                    evaluation.grading_status AS evaluation_grading_status, \
                    qa.attempt_status AS current_attempt_status, \
                    floor(extract(epoch FROM qa.submitted_at) * 1000)::bigint \
                        AS current_submitted_at, \
                    floor(extract(epoch FROM timing.effective_deadline) * 1000)::bigint \
                        AS current_deadline_at \
             FROM question_attempt AS qa \
             LEFT JOIN submission_idempotency AS si \
               ON si.tenant_id = qa.tenant_id AND si.attempt_id = qa.attempt_id \
             LEFT JOIN submission_evaluation AS evaluation \
               ON evaluation.tenant_id = qa.tenant_id AND evaluation.attempt_id = qa.attempt_id \
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
        let result = page_from_rows_with(
            rows,
            page.size.get(),
            decode_current_attempt_with_evaluation_row,
        )?;
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
        retry_transaction(|| async move {
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
        })
        .await
    }

    async fn clear_attempt(
        &self,
        context: TenantContext,
        command: ClearAttemptCommand,
    ) -> Result<AttemptSupportRecord, StoreError> {
        retry_transaction(|| async move {
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
        })
        .await
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
                    evaluation.payload AS evaluation_payload, \
                    evaluation.payload_sha256 AS evaluation_payload_sha256, \
                    evaluation.grading_status AS evaluation_grading_status, \
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
             LEFT JOIN submission_evaluation AS evaluation \
               ON evaluation.tenant_id = qa.tenant_id AND evaluation.attempt_id = qa.attempt_id \
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
            let attempt = decode_current_attempt_with_evaluation_row_named(
                &row,
                "attempt_payload",
                "attempt_sha256",
            )?;
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
                    evaluation.payload AS evaluation_payload, \
                    evaluation.payload_sha256 AS evaluation_payload_sha256, \
                    evaluation.grading_status AS evaluation_grading_status, \
                    qa.attempt_status AS current_attempt_status, \
                    floor(extract(epoch FROM qa.submitted_at) * 1000)::bigint \
                        AS current_submitted_at, \
                    floor(extract(epoch FROM timing.effective_deadline) * 1000)::bigint \
                        AS current_deadline_at \
             FROM question_attempt AS qa \
             LEFT JOIN submission_idempotency AS si \
               ON si.tenant_id = qa.tenant_id AND si.attempt_id = qa.attempt_id \
             LEFT JOIN submission_evaluation AS evaluation \
               ON evaluation.tenant_id = qa.tenant_id AND evaluation.attempt_id = qa.attempt_id \
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
        let record = row
            .as_ref()
            .map(decode_current_attempt_with_evaluation_row)
            .transpose()?;
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

    assignment_timing::lock_postgres_assignment_policy(transaction, tenant, assignment_id).await?;
    let assignment = load_assignment_for_share(transaction, tenant, assignment_id).await?;
    let timing = assignment_timing::load_postgres_resolved_assignment_policy(
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
    assignment_timing::cancel_postgres_attempt_timing_job(transaction, tenant, attempt_id).await?;

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
    assignment_timing::lock_postgres_assignment_policy(transaction, tenant, enrollment.assignment)
        .await?;
    let assignment_guard =
        load_assignment_for_share(transaction, tenant, enrollment.assignment).await?;
    let resolved_assignment_timing = assignment_timing::load_postgres_resolved_assignment_policy(
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
                         WHERE tenant_id = $1 AND predecessor_attempt_id = $2",
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
    let authored_grace_seconds =
        assignment_timing::timing_policy_grace_seconds(question.question.timing_policy);
    let assignment_timing::ResolvedPostgresAttemptTiming {
        effective_deadline,
        effective_grace_seconds,
        auto_submit_at,
        resolution_kind,
    } = assignment_timing::resolved_postgres_attempt_timing(
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
    .bind(assignment_timing::late_submission_policy_name(
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
        // `SELECT .. FOR UPDATE` would require a table-wide UPDATE grant even
        // though successor links are immutable. The primary key serializes
        // concurrent insertions; a loser reads and accepts only the exact link.
        let inserted = sqlx::query(
            "INSERT INTO submission_next_attempt \
             (tenant_id, predecessor_attempt_id, next_attempt_id, next_attempt_occurred_at) \
             VALUES ($1, $2, $3, transaction_timestamp()) \
             ON CONFLICT (tenant_id, predecessor_attempt_id) DO NOTHING",
        )
        .bind(tenant.as_uuid())
        .bind(predecessor.as_uuid())
        .bind(attempt.id.as_uuid())
        .execute(&mut **transaction)
        .await
        .map_err(map_sqlx_error)?;
        if inserted.rows_affected() == 0 {
            let existing: Option<Option<Uuid>> = sqlx::query_scalar(
                "SELECT next_attempt_id FROM submission_next_attempt \
                 WHERE tenant_id = $1 AND predecessor_attempt_id = $2",
            )
            .bind(tenant.as_uuid())
            .bind(predecessor.as_uuid())
            .fetch_optional(&mut **transaction)
            .await
            .map_err(map_sqlx_error)?;
            if existing != Some(Some(attempt.id.as_uuid())) {
                return Err(StoreError::Conflict);
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
                evaluation.payload AS evaluation_payload, \
                evaluation.payload_sha256 AS evaluation_payload_sha256, \
                evaluation.grading_status AS evaluation_grading_status, \
                qa.attempt_status AS current_attempt_status, \
                floor(extract(epoch FROM qa.submitted_at) * 1000)::bigint \
                    AS current_submitted_at, \
                floor(extract(epoch FROM timing.effective_deadline) * 1000)::bigint \
                    AS current_deadline_at \
         FROM question_attempt AS qa \
         LEFT JOIN submission_idempotency AS si \
           ON si.tenant_id = qa.tenant_id AND si.attempt_id = qa.attempt_id \
         LEFT JOIN submission_evaluation AS evaluation \
           ON evaluation.tenant_id = qa.tenant_id AND evaluation.attempt_id = qa.attempt_id \
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
        .map(decode_current_attempt_with_evaluation_row)
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
    assignment_timing::cancel_postgres_attempt_timing_job(transaction, tenant, submitted.id)
        .await?;
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
pub(super) fn add_seconds(
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
pub(super) async fn insert_problem_version(
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
pub(super) async fn insert_published_source_artifact(
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
pub(super) async fn insert_catalog_asset_delivery(
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
pub(super) fn question_backend_name(backend: QuestionBackend) -> &'static str {
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
pub(super) fn publication_scope_name(scope: PublicationScope) -> &'static str {
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
pub(super) fn catalog_lifecycle_parts(
    lifecycle: &CatalogLifecycle,
) -> (&'static str, Option<&str>) {
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
pub(super) fn decode_catalog_payload_row(
    row: &PgRow,
) -> Result<PublishedProblemRecord, StoreError> {
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
pub(super) fn decode_catalog_summary_row(row: &PgRow) -> Result<CatalogProblemSummary, StoreError> {
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
pub(super) fn decode_problem_public_id(value: i64) -> Result<ProblemPublicId, StoreError> {
    u64::try_from(value)
        .ok()
        .and_then(ProblemPublicId::new)
        .ok_or_else(|| StoreError::Unavailable("stored problem public ID is invalid".to_string()))
}

#[cfg(feature = "postgres")]
pub(super) fn decode_problem_version_number(
    value: i64,
) -> Result<ProblemVersionNumber, StoreError> {
    u64::try_from(value)
        .ok()
        .and_then(ProblemVersionNumber::new)
        .ok_or_else(|| {
            StoreError::Unavailable("stored problem version number is invalid".to_string())
        })
}

#[cfg(feature = "postgres")]
pub(super) fn postgres_search_page_request(
    query: &CatalogSearchQuery,
) -> Result<PageRequest, StoreError> {
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
pub(super) fn postgres_catalog_search_fingerprint(query: &CatalogSearchQuery) -> String {
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
pub(super) fn decode_catalog_taxonomy_facet(
    row: PgRow,
) -> Result<CatalogTaxonomyFacet, StoreError> {
    let Json(term): Json<TaxonomyTerm> = row.try_get("taxonomy_term").map_err(map_sqlx_error)?;
    let count: i64 = row.try_get("facet_count").map_err(map_sqlx_error)?;
    Ok(CatalogTaxonomyFacet {
        term,
        count: u64::try_from(count)
            .map_err(|_| StoreError::Unavailable("catalog count overflow".to_string()))?,
    })
}

#[cfg(feature = "postgres")]
pub(super) fn decode_catalog_capability_facet(
    row: PgRow,
) -> Result<CatalogCapabilityFacet, StoreError> {
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
pub(super) fn decode_catalog_license_facet(row: PgRow) -> Result<CatalogLicenseFacet, StoreError> {
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
pub(super) fn catalog_summary_page_from_rows(
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
pub(super) fn taxonomy_page_from_rows(
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
pub(super) fn page_from_keyed_records<T>(
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
pub(super) fn validated_deprecation_reason(reason: String) -> Result<String, StoreError> {
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
pub(super) fn encode_payload<T: Serialize>(
    record: &T,
) -> Result<(Json<Value>, String), StoreError> {
    let value = serde_json::to_value(record)
        .map_err(|error| StoreError::InvalidRecord(error.to_string()))?;
    let bytes =
        serde_json::to_vec(&value).map_err(|error| StoreError::InvalidRecord(error.to_string()))?;
    let checksum = Sha256Digest::compute(&bytes).to_string();
    Ok((Json(value), checksum))
}

#[cfg(feature = "postgres")]
pub(super) fn decode_payload_row<T: DeserializeOwned>(row: &PgRow) -> Result<T, StoreError> {
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
pub(super) fn decode_payload_row_named<T: DeserializeOwned>(
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
fn decode_current_attempt_with_evaluation_row_named(
    row: &PgRow,
    payload_name: &str,
    checksum_name: &str,
) -> Result<QuestionAttempt, StoreError> {
    let mut attempt = decode_current_attempt_row_named(row, payload_name, checksum_name)?;
    let status: Option<String> = row
        .try_get("evaluation_grading_status")
        .map_err(map_sqlx_error)?;
    let Some(status) = status else {
        return Ok(attempt);
    };
    let Json(payload): Json<Value> = row.try_get("evaluation_payload").map_err(map_sqlx_error)?;
    let checksum: String = row
        .try_get("evaluation_payload_sha256")
        .map_err(map_sqlx_error)?;
    let bytes =
        serde_json::to_vec(&payload).map_err(|error| StoreError::Unavailable(error.to_string()))?;
    if Sha256Digest::compute(&bytes).to_string() != checksum {
        return Err(StoreError::Unavailable(
            "stored evaluation payload checksum mismatch".to_string(),
        ));
    }
    match status.as_str() {
        "needs_manual_grading" => {
            attempt.result = None;
            Ok(attempt)
        }
        "graded" | "exempt" => {
            let result = serde_json::from_value(payload)
                .map_err(|error| StoreError::Unavailable(error.to_string()))?;
            crate::validate_attempt_result(result)?;
            attempt.result = Some(result);
            Ok(attempt)
        }
        _ => Err(StoreError::Unavailable(
            "stored evaluation grading status is invalid".to_string(),
        )),
    }
}

#[cfg(feature = "postgres")]
fn decode_current_attempt_with_evaluation_row(row: &PgRow) -> Result<QuestionAttempt, StoreError> {
    decode_current_attempt_with_evaluation_row_named(row, "payload", "payload_sha256")
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
