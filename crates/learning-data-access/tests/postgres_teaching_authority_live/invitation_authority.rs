//! App-role invitation probes for the live teaching-authority oracle.

use learning_data_access::SessionTokenHash;
use question_model::{CourseId, TenantId, UserId};
use sqlx::PgPool;
use uuid::Uuid;

fn id() -> Uuid {
    let mut bytes = [0_u8; 16];
    getrandom::fill(&mut bytes).expect("live fixture UUID randomness");
    Uuid::from_bytes(bytes)
}

pub async fn approved_for_invitation(pool: &PgPool, target: UserId) -> bool {
    let mut tx = pool
        .begin()
        .await
        .expect("approval eligibility transaction");
    sqlx::query("SET LOCAL ROLE ple_app")
        .execute(&mut *tx)
        .await
        .expect("ple_app role");
    let eligible: bool = sqlx::query_scalar("SELECT public.ple_instructor_approval_eligible($1)")
        .bind(target.as_uuid())
        .fetch_one(&mut *tx)
        .await
        .expect("brokered approval eligibility");
    tx.rollback().await.expect("approval eligibility rollback");
    eligible
}

pub async fn approval_target_exists_for_app(pool: &PgPool, target: UserId) -> bool {
    let mut tx = pool.begin().await.expect("approval target transaction");
    sqlx::query("SET LOCAL ROLE ple_app")
        .execute(&mut *tx)
        .await
        .expect("ple_app role");
    let exists: bool =
        sqlx::query_scalar("SELECT public.ple_instructor_approval_target_exists($1)")
            .bind(target.as_uuid())
            .fetch_one(&mut *tx)
            .await
            .expect("brokered approval target existence");
    tx.rollback().await.expect("approval target rollback");
    exists
}

pub async fn target_session_subject_for_app(
    pool: &PgPool,
    tenant: TenantId,
    session: SessionTokenHash,
) -> Option<Uuid> {
    let mut tx = pool
        .begin()
        .await
        .expect("target-session subject transaction");
    sqlx::query("SET LOCAL ROLE ple_app")
        .execute(&mut *tx)
        .await
        .expect("ple_app role");
    sqlx::query("SELECT set_config('ple.tenant_id', $1, true)")
        .bind(tenant.to_string())
        .execute(&mut *tx)
        .await
        .expect("target-session tenant context");
    sqlx::query("SELECT set_config('ple.session_hash', $1, true)")
        .bind(session.to_string())
        .execute(&mut *tx)
        .await
        .expect("presented target-session context");
    let subject =
        sqlx::query_scalar("SELECT user_id FROM public.ple_target_session_subject($1, $2)")
            .bind(session.to_string())
            .bind(tenant.as_uuid())
            .fetch_optional(&mut *tx)
            .await
            .expect("target-session broker call");
    tx.rollback()
        .await
        .expect("target-session subject rollback");
    subject
}

pub async fn direct_invitation_insert_is_denied_for_app(
    pool: &PgPool,
    tenant: TenantId,
    course: CourseId,
    target: UserId,
    inviter: UserId,
) {
    let mut tx = pool
        .begin()
        .await
        .expect("direct invitation insert transaction");
    sqlx::query("SET LOCAL ROLE ple_app")
        .execute(&mut *tx)
        .await
        .expect("ple_app role");
    sqlx::query("SELECT set_config('ple.tenant_id', $1, true)")
        .bind(tenant.to_string())
        .execute(&mut *tx)
        .await
        .expect("direct invitation tenant context");
    let error = sqlx::query(concat!(
        "INSERT INTO public.course_instructor_invitation (tenant_id, course_id, invitation_id, ",
        "target_user_id, invited_by_membership_id) SELECT $1, $2, $3, $4, ",
        "course_membership_id FROM public.course_member WHERE tenant_id=$1 AND course_id=$2 ",
        "AND user_id=$5 AND role='instructor' AND status='active'",
    ))
    .bind(tenant.as_uuid())
    .bind(course.as_uuid())
    .bind(id())
    .bind(target.as_uuid())
    .bind(inviter.as_uuid())
    .execute(&mut *tx)
    .await
    .expect_err("ple_app cannot insert an invitation outside the session-derived broker");
    assert_eq!(
        error
            .as_database_error()
            .and_then(|value| value.code())
            .as_deref(),
        Some("42501")
    );
    tx.rollback()
        .await
        .expect("direct invitation insert rollback");
}

pub async fn target_search_count_for_app(
    pool: &PgPool,
    tenant: TenantId,
    actor: UserId,
    course: CourseId,
    query: &str,
) -> usize {
    let mut tx = pool.begin().await.expect("target-search transaction");
    sqlx::query("SET LOCAL ROLE ple_app")
        .execute(&mut *tx)
        .await
        .expect("ple_app role");
    sqlx::query("SELECT set_config('ple.tenant_id', $1, true)")
        .bind(tenant.to_string())
        .execute(&mut *tx)
        .await
        .expect("target-search tenant context");
    let rows = sqlx::query(
        "SELECT * FROM public.ple_course_co_instructor_target_search($1, $2, $3, $4, $5)",
    )
    .bind(actor.as_uuid())
    .bind(course.as_uuid())
    .bind(query)
    .bind(None::<i32>)
    .bind(11_i32)
    .fetch_all(&mut *tx)
    .await
    .expect("ple_app target-search broker call");
    tx.rollback().await.expect("target-search rollback");
    rows.len()
}
