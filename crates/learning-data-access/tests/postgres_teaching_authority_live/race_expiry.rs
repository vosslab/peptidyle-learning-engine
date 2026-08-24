//! Live expiry-boundary and approval-lock serialization evidence.

use std::time::Duration;

use super::*;

const LIVE_TIMEOUT: Duration = Duration::from_secs(10);
const LOCK_POLL_INTERVAL: Duration = Duration::from_millis(20);

async fn wait_for_eligibility_lock(
    pool: &sqlx::PgPool,
    blocker_pid: i32,
    expected_accept_pid: i32,
) -> Result<(), String> {
    let deadline = tokio::time::Instant::now() + LIVE_TIMEOUT;
    loop {
        let waiters: i64 = sqlx::query_scalar(concat!(
            "SELECT count(*) FROM pg_stat_activity AS activity ",
            "JOIN pg_locks AS blocked ON blocked.pid=activity.pid ",
            "JOIN pg_locks AS held ON held.locktype=blocked.locktype ",
            "AND held.transactionid=blocked.transactionid ",
            "WHERE activity.pid=$2 ",
            "AND activity.query LIKE '%ple_lock_instructor_approval_eligibility%' ",
            "AND activity.wait_event_type='Lock' AND activity.wait_event='transactionid' ",
            "AND blocked.locktype='transactionid' AND NOT blocked.granted ",
            "AND held.pid=$1 AND held.granted",
        ))
        .bind(blocker_pid)
        .bind(expected_accept_pid)
        .fetch_one(pool)
        .await
        .expect("approval-lock wait probe");
        if waiters == 1 {
            return Ok(());
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!(
                "acceptance backend {expected_accept_pid} did not wait for blocker {blocker_pid} \\
                 in ple_lock_instructor_approval_eligibility"
            ));
        }
        tokio::time::sleep(LOCK_POLL_INTERVAL).await;
    }
}

#[tokio::test]
#[ignore = "requires the disposable PostgreSQL 17 database baseline"]
async fn postgres_teaching_authority_exact_expiry_boundary_oracle() {
    let url = std::env::var("PLE_TEST_DATABASE_URL")
        .expect("PLE_TEST_DATABASE_URL must name the disposable acceptance database");
    let pool = lazy_pool(&url).expect("disposable PostgreSQL URL");
    verify_application_schema(&pool)
        .await
        .expect("migrated schema");
    let store = PostgresStore::with_question_id_secret(pool.clone(), [0x56; 32]);
    let tenant = TenantId::from_uuid(id());
    let context = TenantContext::from_authenticated_session(tenant);
    let sysadmin = UserId::from_uuid(id());
    let instructor = UserId::from_uuid(id());
    let target = UserId::from_uuid(id());
    create_account(&store, sysadmin).await;
    create_account(&store, instructor).await;
    create_account(&store, target).await;
    let sysadmin_session = session(&store, tenant, sysadmin, vec![UserRole::Sysadmin]).await;
    let instructor_session = session(&store, tenant, instructor, vec![UserRole::Instructor]).await;
    let target_session = session(&store, tenant, target, vec![UserRole::Instructor]).await;
    let course = CourseId::from_uuid(id());
    store
        .create_course(
            context,
            CreateCourseCommand {
                course: CourseRecord {
                    id: course,
                    tenant,
                    title: "T2 exact expiry live course".into(),
                    term: CourseTerm::from_parts("2026-08-24", "2026-12-18", "America/Chicago")
                        .expect("fixture term"),
                },
                authority: CourseCreationAuthority::ApprovedInstructor {
                    actor: instructor,
                    session: instructor_session,
                },
            },
        )
        .await
        .expect("fixture course");
    approval(&store, context, sysadmin_session, target).await;
    let expired = store
        .create_co_instructor_invitation(
            context,
            CreateCoInstructorInvitation {
                session: instructor_session,
                actor: instructor,
                course,
                target,
            },
        )
        .await
        .expect("pending invitation");

    let mut owner = pool
        .begin()
        .await
        .expect("database-owner fixture transaction");
    sqlx::query("SET LOCAL session_replication_role = replica")
        .execute(&mut *owner)
        .await
        .expect("temporarily suppress immutable-row trigger for exact expiry fixture");
    sqlx::query(concat!(
        "WITH boundary AS (SELECT transaction_timestamp() AS instant) ",
        "UPDATE course_instructor_invitation AS invitation ",
        "SET created_at=boundary.instant - interval '30 days', expires_at=boundary.instant ",
        "FROM boundary WHERE invitation.tenant_id=$1 AND invitation.invitation_id=$2",
    ))
    .bind(tenant.as_uuid())
    .bind(expired.invitation.id.as_uuid())
    .execute(&mut *owner)
    .await
    .expect("exact 30-day expiry fixture update");
    sqlx::query("SET LOCAL session_replication_role = DEFAULT")
        .execute(&mut *owner)
        .await
        .expect("restore normal trigger role before fixture commit");
    owner.commit().await.expect("expiry fixture commit");

    let pending = store
        .list_pending_co_instructor_invitations(
            context,
            target_session,
            PageRequest::first(PageSize::new(10).expect("page size")),
        )
        .await
        .expect("expired target pending list");
    assert!(
        pending.items.is_empty(),
        "the exact boundary row is not pending"
    );
    assert_eq!(
        store
            .accept_co_instructor_invitation(
                context,
                RespondToCoInstructorInvitation {
                    session: target_session,
                    actor: target,
                    invitation: expired.invitation.id,
                    expected_revision: expired.revision,
                },
            )
            .await,
        Err(StoreError::Conflict),
        "acceptance at exact expiry conflicts"
    );
    let before_reinvite: (String, i64, bool, bool) = sqlx::query_as(concat!(
        "SELECT status, revision, expires_at - created_at = interval '30 days', ",
        "expires_at <= transaction_timestamp() FROM course_instructor_invitation ",
        "WHERE tenant_id=$1 AND invitation_id=$2",
    ))
    .bind(tenant.as_uuid())
    .bind(expired.invitation.id.as_uuid())
    .fetch_one(&pool)
    .await
    .expect("immutable pending boundary row");
    assert_eq!(before_reinvite, ("pending".into(), 1, true, true));

    let replacement = store
        .create_co_instructor_invitation(
            context,
            CreateCoInstructorInvitation {
                session: instructor_session,
                actor: instructor,
                course,
                target,
            },
        )
        .await
        .expect("normal reinvite expires the old pending row");
    let transitioned: (String, i64, bool) = sqlx::query_as(concat!(
        "SELECT status, revision, accepted_at IS NULL AND declined_at IS NULL ",
        "AND revoked_at IS NULL ",
        "FROM course_instructor_invitation WHERE tenant_id=$1 AND invitation_id=$2",
    ))
    .bind(tenant.as_uuid())
    .bind(expired.invitation.id.as_uuid())
    .fetch_one(&pool)
    .await
    .expect("normal expiry transition receipt");
    assert_eq!(transitioned, ("expired".into(), 2, true));
    assert_ne!(replacement.invitation.id, expired.invitation.id);
}

#[tokio::test]
#[ignore = "requires the disposable PostgreSQL 17 database baseline"]
async fn postgres_teaching_authority_acceptance_precedes_queued_approval_revoke() {
    let url = std::env::var("PLE_TEST_DATABASE_URL")
        .expect("PLE_TEST_DATABASE_URL must name the disposable acceptance database");
    let pool = lazy_pool(&url).expect("disposable PostgreSQL URL");
    verify_application_schema(&pool)
        .await
        .expect("migrated schema");
    let store = PostgresStore::with_question_id_secret(pool.clone(), [0x57; 32]);
    let tenant = TenantId::from_uuid(id());
    let context = TenantContext::from_authenticated_session(tenant);
    let sysadmin = UserId::from_uuid(id());
    let instructor = UserId::from_uuid(id());
    let target = UserId::from_uuid(id());
    create_account(&store, sysadmin).await;
    create_account(&store, instructor).await;
    create_account(&store, target).await;
    let sysadmin_session = session(&store, tenant, sysadmin, vec![UserRole::Sysadmin]).await;
    let instructor_session = session(&store, tenant, instructor, vec![UserRole::Instructor]).await;
    let target_session = session(&store, tenant, target, vec![UserRole::Instructor]).await;
    let course = CourseId::from_uuid(id());
    store
        .create_course(
            context,
            CreateCourseCommand {
                course: CourseRecord {
                    id: course,
                    tenant,
                    title: "T2 approval race live course".into(),
                    term: CourseTerm::from_parts("2026-08-24", "2026-12-18", "America/Chicago")
                        .expect("fixture term"),
                },
                authority: CourseCreationAuthority::ApprovedInstructor {
                    actor: instructor,
                    session: instructor_session,
                },
            },
        )
        .await
        .expect("fixture course");
    let approval_revision = approval(&store, context, sysadmin_session, target).await;
    let invitation = store
        .create_co_instructor_invitation(
            context,
            CreateCoInstructorInvitation {
                session: instructor_session,
                actor: instructor,
                course,
                target,
            },
        )
        .await
        .expect("target invitation");

    let mut blocker = pool.begin().await.expect("approval blocker transaction");
    let blocker_pid: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
        .fetch_one(&mut *blocker)
        .await
        .expect("blocker backend PID");
    sqlx::query("SELECT 1 FROM public.instructor_approval WHERE user_id=$1 FOR UPDATE")
        .bind(target.as_uuid())
        .execute(&mut *blocker)
        .await
        .expect("block exact instructor approval row");

    let acceptance_pool = PgPoolOptions::new()
        .max_connections(1)
        .connect(&url)
        .await
        .expect("single-connection acceptance pool");
    let expected_accept_pid: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
        .fetch_one(&acceptance_pool)
        .await
        .expect("acceptance backend PID before spawn");
    let accept_store = PostgresStore::with_question_id_secret(acceptance_pool.clone(), [0x57; 32]);
    let mut accept = tokio::spawn(async move {
        accept_store
            .accept_co_instructor_invitation(
                context,
                RespondToCoInstructorInvitation {
                    session: target_session,
                    actor: target,
                    invitation: invitation.invitation.id,
                    expected_revision: invitation.revision,
                },
            )
            .await
    });
    if let Err(error) = wait_for_eligibility_lock(&pool, blocker_pid, expected_accept_pid).await {
        accept.abort();
        blocker
            .rollback()
            .await
            .expect("release failed approval blocker");
        let _ = accept.await;
        acceptance_pool.close().await;
        panic!("{error}");
    }

    let revoke_store = store.clone();
    let mut revoke = tokio::spawn(async move {
        revoke_store
            .revoke_instructor_approval(
                context,
                RevokeInstructorApproval {
                    session: sysadmin_session,
                    target,
                    expected_revision: approval_revision,
                },
            )
            .await
    });
    blocker.commit().await.expect("release approval blocker");
    let outcomes = tokio::time::timeout(LIVE_TIMEOUT, async {
        tokio::join!(&mut accept, &mut revoke)
    })
    .await;
    let (accepted, revoked) = match outcomes {
        Ok(outcomes) => outcomes,
        Err(_) => {
            accept.abort();
            revoke.abort();
            let _ = accept.await;
            let _ = revoke.await;
            acceptance_pool.close().await;
            panic!("acceptance and queued revoke must not deadlock");
        }
    };
    acceptance_pool.close().await;
    let accepted = accepted
        .expect("acceptance task joins")
        .expect("first queued approval lock holder accepts");
    let revoked = revoked
        .expect("revoke task joins")
        .expect("queued approval revocation commits after acceptance");
    assert_eq!(accepted.user, target);
    assert!(revoked.approval.revoked_at.is_some());

    let poststate: (i64, String, bool) = sqlx::query_as(concat!(
        "SELECT (SELECT count(*) FROM course_member WHERE tenant_id=$1 AND course_id=$2 ",
        "AND user_id=$3 AND role='instructor' AND status='active' AND student_id IS NULL), ",
        "(SELECT status FROM course_instructor_invitation WHERE tenant_id=$1 ",
        "AND invitation_id=$4), ",
        "(SELECT revoked_at IS NOT NULL FROM instructor_approval WHERE user_id=$3)",
    ))
    .bind(tenant.as_uuid())
    .bind(course.as_uuid())
    .bind(target.as_uuid())
    .bind(invitation.invitation.id.as_uuid())
    .fetch_one(&pool)
    .await
    .expect("linearizable acceptance then revocation poststate");
    assert_eq!(poststate, (1, "accepted".into(), true));
}
