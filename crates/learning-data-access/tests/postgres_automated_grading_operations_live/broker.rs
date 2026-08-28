//! Test-only transaction and public-broker helpers for the connected oracle.

use question_model::{AssignmentId, CourseId, TenantId};
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

pub(super) async fn app_transaction<'pool>(
    pool: &'pool PgPool,
    tenant: TenantId,
    session: &str,
) -> Transaction<'pool, Postgres> {
    let mut transaction = pool.begin().await.expect("begin app transaction");
    sqlx::query("SET LOCAL ROLE ple_app")
        .execute(&mut *transaction)
        .await
        .expect("select application role");
    sqlx::query("SELECT set_config('ple.tenant_id', $1, true)")
        .bind(tenant.to_string())
        .execute(&mut *transaction)
        .await
        .expect("bind tenant context");
    sqlx::query("SELECT set_config('ple.session_hash', $1, true)")
        .bind(session)
        .execute(&mut *transaction)
        .await
        .expect("bind session context");
    transaction
}

pub(super) async fn admin_tenant_transaction<'pool>(
    pool: &'pool PgPool,
    tenant: TenantId,
) -> Transaction<'pool, Postgres> {
    let mut transaction = pool.begin().await.expect("begin admin tenant transaction");
    sqlx::query("SELECT set_config('ple.tenant_id', $1, true)")
        .bind(tenant.to_string())
        .execute(&mut *transaction)
        .await
        .expect("bind admin tenant context");
    transaction
}

pub(super) struct InstructorBroker<'a> {
    pub(super) pool: &'a PgPool,
    pub(super) tenant: TenantId,
    pub(super) session: &'a str,
    pub(super) course: CourseId,
    pub(super) assignment: AssignmentId,
}

impl InstructorBroker<'_> {
    pub(super) async fn list(
        &self,
        group_by: &str,
        after: Option<(&str, i32)>,
    ) -> Vec<sqlx::postgres::PgRow> {
        let mut transaction = app_transaction(self.pool, self.tenant, self.session).await;
        let rows = sqlx::query(
            "SELECT * FROM public.ple_list_instructor_grading_operations_v1(\
             $1,$2,$3,$4,$5,$6,$7,$8)",
        )
        .bind(self.tenant.as_uuid())
        .bind(self.session)
        .bind(self.course.as_uuid())
        .bind(self.assignment.as_uuid())
        .bind(group_by)
        .bind(after.map(|value| value.0))
        .bind(after.map(|value| value.1))
        .bind(20_i32)
        .fetch_all(&mut *transaction)
        .await
        .expect("list grading operations through public broker");
        transaction.commit().await.expect("commit list transaction");
        rows
    }

    pub(super) async fn retry(
        &self,
        operation: i32,
        revision: i64,
        action: Uuid,
    ) -> Result<sqlx::postgres::PgRow, sqlx::Error> {
        let mut transaction = app_transaction(self.pool, self.tenant, self.session).await;
        let row = sqlx::query(
            "SELECT * FROM public.ple_retry_instructor_grading_operation_v1(\
             $1,$2,$3,$4,$5,$6,$7)",
        )
        .bind(self.tenant.as_uuid())
        .bind(self.session)
        .bind(self.course.as_uuid())
        .bind(self.assignment.as_uuid())
        .bind(operation)
        .bind(revision)
        .bind(action)
        .fetch_optional(&mut *transaction)
        .await?;
        transaction.commit().await?;
        row.ok_or(sqlx::Error::RowNotFound)
    }

    pub(super) async fn recalculate(
        &self,
        revision: i64,
        action: Uuid,
    ) -> Result<sqlx::postgres::PgRow, sqlx::Error> {
        let mut transaction = app_transaction(self.pool, self.tenant, self.session).await;
        let row = sqlx::query(
            "SELECT * FROM public.ple_recalculate_instructor_assignment_v1(\
             $1,$2,$3,$4,$5,$6)",
        )
        .bind(self.tenant.as_uuid())
        .bind(self.session)
        .bind(self.course.as_uuid())
        .bind(self.assignment.as_uuid())
        .bind(revision)
        .bind(action)
        .fetch_optional(&mut *transaction)
        .await?;
        transaction.commit().await?;
        row.ok_or(sqlx::Error::RowNotFound)
    }
}
