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
use question_model::run_policy::TimingPolicy;
#[cfg(feature = "postgres")]
use question_model::taxonomy::TaxonomyTerm;
#[cfg(feature = "postgres")]
use question_model::{
    ActivityTimestamp, AssignmentEnrollment, AssignmentId, AssignmentRun, AttemptResult,
    AttemptTimerRecord, BackendCapabilities, CatalogLifecycle, CatalogProblemSummary, CourseId,
    CourseMembership, CourseMembershipRole, CourseRole, CourseSummary, EnrollmentId,
    EnrollmentStatus, ProblemId, ProblemVersionRef, PublicationScope, QuestionAttempt,
    QuestionAttemptId, QuestionBackend, QuestionMetadata, RunId, RunMode, StudentAssignmentSummary,
    StudentResponse, TenantId, UserId, UserRole, VersionId, WorkspaceId,
};
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
use crate::{
    ActivityTransition, AssetAccessEvent, AssetDeliveryId, AssetDeliveryRecord, AssetDeliveryScope,
    AssetStore, AssignmentRecord, AuthorizedAssetDelivery, CatalogStore, CatalogTransition,
    CourseListScope, CourseRecord, Cursor, DraftRecord, IssueQuestionAttemptCommand, Page,
    PageRequest, PublishDraftCommand, PublishedProblemRecord, SessionLifetime, SessionRecord,
    SessionStore, SessionSubject, SessionTokenHash, Store, StoreError, SubmissionIdempotencyKey,
    SubmissionRecord, SubmitQuestionAttemptCommand, TenantContext, completed_run_score,
    ensure_tenant, grade_policy, project_enrollment_completion, summary_transition,
    validate_asset_delivery, validate_assignment, validate_course, validate_draft,
    validate_published,
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
];

/// The connection pool type, re-exported so callers do not need `sqlx`.
#[cfg(feature = "postgres")]
pub type Pool = PgPool;

/// Replica-safe PostgreSQL implementation of the backend-neutral store.
#[cfg(feature = "postgres")]
#[derive(Clone)]
pub struct PostgresStore {
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
impl Store for PostgresStore {
    async fn upsert_draft(
        &self,
        context: TenantContext,
        draft: DraftRecord,
    ) -> Result<(), StoreError> {
        ensure_tenant(context, draft.tenant)?;
        validate_draft(&draft)?;
        let (payload, checksum) = encode_payload(&draft)?;
        let mut transaction = self.begin_tenant(context).await?;
        sqlx::query(
            "INSERT INTO workspace_draft \
             (tenant_id, workspace_id, payload, payload_sha256) \
             VALUES ($1, $2, $3, $4) \
             ON CONFLICT (tenant_id, workspace_id) DO UPDATE SET \
             payload = EXCLUDED.payload, payload_sha256 = EXCLUDED.payload_sha256, \
             updated_at = transaction_timestamp()",
        )
        .bind(draft.tenant.as_uuid())
        .bind(draft.question.workspace.as_uuid())
        .bind(payload)
        .bind(checksum)
        .execute(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        transaction.commit().await.map_err(map_sqlx_error)
    }

    async fn get_draft(
        &self,
        context: TenantContext,
        workspace: WorkspaceId,
    ) -> Result<Option<DraftRecord>, StoreError> {
        let mut transaction = self.begin_tenant(context).await?;
        let row = sqlx::query(
            "SELECT payload, payload_sha256 FROM workspace_draft \
             WHERE tenant_id = $1 AND workspace_id = $2",
        )
        .bind(context.tenant_id().as_uuid())
        .bind(workspace.as_uuid())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let record = row.as_ref().map(decode_payload_row).transpose()?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(record)
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

    async fn upsert_assignment(
        &self,
        context: TenantContext,
        assignment: AssignmentRecord,
    ) -> Result<(), StoreError> {
        ensure_tenant(context, assignment.tenant)?;
        validate_assignment(&assignment)?;
        let (payload, checksum) = encode_payload(&assignment)?;
        let mut transaction = self.begin_tenant(context).await?;
        let course_exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM course WHERE tenant_id = $1 AND course_id = $2)",
        )
        .bind(assignment.tenant.as_uuid())
        .bind(assignment.course_id.as_uuid())
        .fetch_one(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        if !course_exists {
            return Err(StoreError::InvalidRecord(
                "assignment references a missing course".to_string(),
            ));
        }
        for reference in &assignment.problems {
            let exists: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM problem_version \
                 WHERE problem_id = $1 AND version_id = $2 \
                   AND lifecycle IN ('published', 'deprecated'))",
            )
            .bind(reference.problem.as_uuid())
            .bind(reference.version.as_uuid())
            .fetch_one(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
            if !exists {
                return Err(StoreError::InvalidRecord(format!(
                    "assignment references a missing, hidden, or inactive published version {}/{}",
                    reference.problem, reference.version
                )));
            }
        }
        sqlx::query(
            "INSERT INTO assignment \
             (tenant_id, assignment_id, course_id, title, payload, payload_sha256) \
             VALUES ($1, $2, $3, $4, $5, $6) \
             ON CONFLICT (tenant_id, assignment_id) DO UPDATE SET \
             course_id = EXCLUDED.course_id, title = EXCLUDED.title, \
             payload = EXCLUDED.payload, payload_sha256 = EXCLUDED.payload_sha256, \
             updated_at = transaction_timestamp()",
        )
        .bind(assignment.tenant.as_uuid())
        .bind(assignment.id.as_uuid())
        .bind(assignment.course_id.as_uuid())
        .bind(&assignment.title)
        .bind(payload)
        .bind(checksum)
        .execute(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        sqlx::query("DELETE FROM assignment_problem WHERE tenant_id = $1 AND assignment_id = $2")
            .bind(assignment.tenant.as_uuid())
            .bind(assignment.id.as_uuid())
            .execute(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        for (position, reference) in assignment.problems.iter().enumerate() {
            let position = i32::try_from(position).map_err(|_| {
                StoreError::InvalidRecord("too many assignment problems".to_string())
            })?;
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
            .execute(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        }
        transaction.commit().await.map_err(map_sqlx_error)
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
}

#[cfg(feature = "postgres")]
#[async_trait]
impl CatalogStore for PostgresStore {
    async fn publish_draft(
        &self,
        context: TenantContext,
        command: PublishDraftCommand,
    ) -> Result<PublishedProblemRecord, StoreError> {
        ensure_tenant(context, command.expected_draft.tenant)?;
        validate_draft(&command.expected_draft)?;

        let mut transaction = self.begin_tenant(context).await?;
        let draft_row = sqlx::query(
            "SELECT payload, payload_sha256 FROM workspace_draft \
             WHERE tenant_id = $1 AND workspace_id = $2 FOR UPDATE",
        )
        .bind(context.tenant_id().as_uuid())
        .bind(command.expected_draft.question.workspace.as_uuid())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?
        .ok_or(StoreError::NotFound)?;
        let stored_draft: DraftRecord = decode_payload_row(&draft_row)?;
        if stored_draft != command.expected_draft {
            return Err(StoreError::Conflict);
        }

        let version = command.expected_draft.question.version;
        let (authors, previous_version, derived_from, is_new_problem) =
            if let Some(revises) = command.expected_draft.revises {
                if command.problem != revises.problem {
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
        .bind(command.problem.as_uuid())
        .bind(version.as_uuid())
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
        let mut question = command.expected_draft.question.clone();
        question.problem = Some(command.problem);
        let record = PublishedProblemRecord {
            problem: command.problem,
            version,
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
            return Ok(active);
        }
        return Err(StoreError::InvalidRecord(
            "another question attempt is already active in this run".to_string(),
        ));
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
    if command.parameter_hash.trim().is_empty()
        || command
            .provenance
            .rendered_question_sha256
            .trim()
            .is_empty()
    {
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
        seed: command.seed,
        parameter_hash: command.parameter_hash,
        response: None,
        result: None,
        timer,
        provenance: command.provenance,
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
    Ok(attempt)
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
    }
    let (attempt_payload, attempt_checksum) = encode_payload(&submitted)?;
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
    Ok(SubmissionRecord {
        attempt: submitted,
        run,
        summary: next,
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
    let run = load_run_for_update(transaction, tenant, submitted.run).await?;
    let enrollment = load_enrollment_for_update(transaction, tenant, run.enrollment).await?;
    let summary = load_summary_for_update(transaction, tenant, enrollment.id).await?;
    Ok(Some(SubmissionRecord {
        attempt: submitted,
        run,
        summary,
    }))
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

#[cfg(feature = "postgres")]
fn question_backend_name(backend: QuestionBackend) -> &'static str {
    match backend {
        QuestionBackend::Native => "native",
        QuestionBackend::Webwork => "webwork",
        QuestionBackend::Qti => "qti",
        QuestionBackend::H5p => "h5p",
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
            _ => {}
        }
    }
    StoreError::Unavailable(error.to_string())
}
