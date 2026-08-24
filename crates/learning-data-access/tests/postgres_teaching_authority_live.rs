#![cfg(feature = "postgres")]

//! Disposable PostgreSQL 17 authority and concurrency oracle for T2.
//!
//! This is intentionally ignored: it needs the disposable migrated database
//! named by `PLE_TEST_DATABASE_URL`. Store calls create normal accounts,
//! sessions, courses, and memberships; the small SQL probes cover catalog and
//! RLS facts that the Store API cannot observe.

use learning_data_access::postgres::{PostgresStore, lazy_pool, verify_application_schema};
use learning_data_access::{
    AccountIdentityStore, ApproveInstructorAccount, AuthenticationEmail,
    AuthenticationRateLimitKey, BeginEmailAuthentication, BrowserBindingHash,
    CoInstructorInvitationRevision, CompleteEmailAuthentication, CourseCreationAuthority,
    CourseListScope, CourseRecord, CourseRosterStore, CreateCoInstructorInvitation,
    CreateCourseCommand, Cursor, EmailAuthenticationPurpose, EmailChallengeId,
    EmailChallengeLifetime, EmailChallengeSecretHash, InstructorApprovalRevision, PageRequest,
    PageSize, RemoveDirectInstructorMembership, RespondToCoInstructorInvitation,
    RevokeCoInstructorInvitation, RevokeInstructorApproval, SessionLifetime, SessionStore,
    SessionSubject, SessionTokenHash, Store, StoreError, TeachingAuthorityReferenceStore,
    TeachingAuthorityStore, TenantContext, UpsertCourseMember,
};
use question_model::{CourseId, CourseTerm, TenantId, UserId, UserRole};
use sqlx::{Row, postgres::PgPoolOptions};
use uuid::Uuid;

#[path = "postgres_teaching_authority_live/invitation_authority.rs"]
mod invitation_authority;
#[path = "postgres_teaching_authority_live/public_reference.rs"]
mod public_reference;
#[path = "postgres_teaching_authority_live/race_expiry.rs"]
mod race_expiry;
#[path = "postgres_teaching_authority_live/sysadmin_candidate.rs"]
mod sysadmin_candidate;

use invitation_authority::{
    approval_target_exists_for_app, approved_for_invitation, insert_approved_invitation_for_app,
    target_search_count_for_app, target_session_subject_for_app,
};

fn id() -> Uuid {
    let mut bytes = [0_u8; 16];
    getrandom::fill(&mut bytes).expect("live fixture UUID randomness");
    Uuid::from_bytes(bytes)
}

async fn session(
    store: &PostgresStore,
    tenant: TenantId,
    user: UserId,
    roles: Vec<UserRole>,
) -> SessionTokenHash {
    let token = SessionTokenHash::compute(id().as_bytes());
    store
        .create_session(
            token,
            SessionSubject::new(tenant, user, "T2 live fixture", roles)
                .expect("fixture session subject"),
            SessionLifetime::from_seconds(3_600).expect("fixture session lifetime"),
        )
        .await
        .expect("fixture session persists");
    token
}

async fn approval(
    store: &PostgresStore,
    context: TenantContext,
    sysadmin_session: SessionTokenHash,
    target: UserId,
) -> InstructorApprovalRevision {
    store
        .approve_instructor_account(
            context,
            ApproveInstructorAccount {
                session: sysadmin_session,
                target,
                expected_revision: None,
            },
        )
        .await
        .expect("fixture target approval")
        .revision
}

/// Creates a persisted PLE account through the public passwordless flow.
///
/// Each call has independent random challenge, binding, rate-limit, and email
/// values, while the caller-provided user ID remains the account identity used
/// by the authority APIs under test.
async fn create_account(store: &PostgresStore, user: UserId) {
    create_account_named(store, user, "T2 live co-instructor candidate").await;
}

async fn create_account_named(store: &PostgresStore, user: UserId, display_name: &str) {
    let challenge_secret = id();
    let binding_secret = id();
    let rate_limit_secret = id();
    let email = format!("t2-live-{}@example.edu", user.as_uuid().simple());
    let token_hash = EmailChallengeSecretHash::compute(challenge_secret.as_bytes());
    let browser_binding = BrowserBindingHash::compute(binding_secret.as_bytes());
    store
        .begin_email_authentication(BeginEmailAuthentication {
            id: EmailChallengeId::from_uuid(id()),
            token_hash,
            browser_binding,
            email_rate_limit_key: AuthenticationRateLimitKey::compute(rate_limit_secret.as_bytes()),
            email: AuthenticationEmail::parse(&email).expect("fixture email"),
            purpose: EmailAuthenticationPurpose::SignInOrRegister,
            lifetime: EmailChallengeLifetime::from_seconds(600).expect("fixture lifetime"),
        })
        .await
        .expect("fixture email-authentication challenge");
    store
        .complete_email_authentication(CompleteEmailAuthentication {
            token_hash,
            browser_binding,
            proposed_user: user,
            proposed_display_name: display_name.to_string(),
        })
        .await
        .expect("fixture account");
}

async fn invitation_count_for_app(pool: &sqlx::PgPool, tenant: Option<TenantId>) -> i64 {
    let mut tx = pool.begin().await.expect("RLS probe transaction");
    sqlx::query("SET LOCAL ROLE ple_app")
        .execute(&mut *tx)
        .await
        .expect("ple_app role");
    if let Some(tenant) = tenant {
        sqlx::query("SELECT set_config('ple.tenant_id', $1, true)")
            .bind(tenant.to_string())
            .execute(&mut *tx)
            .await
            .expect("tenant RLS context");
    }
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM public.course_instructor_invitation")
        .fetch_one(&mut *tx)
        .await
        .expect("RLS count");
    tx.rollback().await.expect("RLS probe rollback");
    count
}

async fn approval_function_guards(pool: &sqlx::PgPool) {
    let rows = sqlx::query(concat!(
        "SELECT p.proname, p.prosecdef, p.provolatile::text AS provolatile, ",
        "p.proconfig, r.rolname, ",
        "has_function_privilege('public', p.oid, 'EXECUTE') AS public_execute, ",
        "has_function_privilege('ple_app', p.oid, 'EXECUTE') AS app_execute ",
        "FROM pg_proc p JOIN pg_roles r ON r.oid=p.proowner WHERE p.proname IN ",
        "('ple_instructor_approval_eligible', 'ple_target_session_subject', ",
        "'ple_instructor_approval_target_exists', ",
        "'ple_own_account_reference', 'ple_sysadmin_account_reference', ",
        "'ple_approved_account_reference', ",
        "'ple_lock_instructor_approval_eligibility', 'ple_sysadmin_instructor_approval', ",
        "'ple_sysadmin_revoke_instructor_approval', ",
        "'ple_course_instructor_invitation_reference_list', ",
        "'ple_pending_instructor_invitation_reference_list', ",
        "'ple_course_membership_reference_list', ",
        "'ple_course_active_student_membership_reference_list', ",
        "'ple_course_active_student_membership_reference', ",
        "'ple_course_instructor_membership_reference_list', ",
        "'ple_course_co_instructor_target_search', ",
        "'ple_course_instructor_roster_revision')",
    ))
    .fetch_all(pool)
    .await
    .expect("function catalog query");
    assert_eq!(
        rows.len(),
        17,
        "the seventeen T2 broker functions are installed"
    );
    for row in rows {
        let name: String = row.try_get("proname").expect("function name");
        let owner: String = row.try_get("rolname").expect("function owner");
        let config: Option<Vec<String>> = row.try_get("proconfig").expect("function config");
        assert!(
            row.try_get::<bool, _>("prosecdef")
                .expect("security definer")
        );
        assert_eq!(owner, "ple_teaching_authority_broker");
        assert!(
            !row.try_get::<bool, _>("public_execute")
                .expect("public grant")
        );
        assert!(
            row.try_get::<bool, _>("app_execute")
                .expect("ple_app grant"),
            "{name} remains callable only through the application capability"
        );
        assert!(
            config.unwrap_or_default().iter().any(|value| {
                value == "search_path=pg_catalog, public"
                    || value == "search_path=pg_catalog,public"
            }),
            "{name} has a fixed search path"
        );
        if name == "ple_lock_instructor_approval_eligibility" {
            assert_eq!(
                row.try_get::<String, _>("provolatile").expect("volatility"),
                "v"
            );
        }
    }
    let app_can_read_sessions: bool = sqlx::query_scalar(
        "SELECT has_table_privilege('ple_app', 'public.auth_session', 'SELECT')",
    )
    .fetch_one(pool)
    .await
    .expect("auth-session privilege catalog query");
    assert!(
        !app_can_read_sessions,
        "the pending-invitation surface receives no direct auth-session read grant"
    );
}

/// Exercises the cases that require genuinely independent Store transactions.
/// The ignored PostgreSQL lane is the authority oracle; this remains compile-only
/// in ordinary developer checks.
#[tokio::test]
#[ignore = "requires the disposable PostgreSQL 17 database baseline"]
async fn postgres_teaching_authority_concurrent_lifecycle_oracle() {
    let url = std::env::var("PLE_TEST_DATABASE_URL")
        .expect("PLE_TEST_DATABASE_URL must name the disposable acceptance database");
    let pool = lazy_pool(&url).expect("disposable PostgreSQL URL");
    verify_application_schema(&pool)
        .await
        .expect("migrated schema");
    let store = PostgresStore::with_question_id_secret(pool.clone(), [0x55; 32]);
    let tenant = TenantId::from_uuid(id());
    let context = TenantContext::from_authenticated_session(tenant);
    let sysadmin = UserId::from_uuid(id());
    let instructor = UserId::from_uuid(id());
    let target = UserId::from_uuid(id());
    let alternate = UserId::from_uuid(id());
    create_account(&store, sysadmin).await;
    create_account(&store, target).await;
    create_account(&store, alternate).await;
    let sysadmin_session = session(&store, tenant, sysadmin, vec![UserRole::Sysadmin]).await;
    let instructor_session = session(&store, tenant, instructor, vec![UserRole::Instructor]).await;
    let target_session = session(&store, tenant, target, vec![UserRole::Instructor]).await;
    let alternate_session = session(&store, tenant, alternate, vec![UserRole::Instructor]).await;
    assert_eq!(
        sysadmin_session.to_string().len(),
        64,
        "stored hashes are exact SHA-256 hex"
    );
    assert_eq!(
        SessionTokenHash::from_hex(&sysadmin_session.to_string()).expect("exact hash parses"),
        sysadmin_session,
        "the complete 64-character session hash survives the Store boundary"
    );
    let course = CourseId::from_uuid(id());
    store
        .create_course(
            context,
            CreateCourseCommand {
                course: CourseRecord {
                    id: course,
                    tenant,
                    title: "T2 concurrency live course".into(),
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
    approval(&store, context, sysadmin_session, alternate).await;

    let first_create = store.create_co_instructor_invitation(
        context,
        CreateCoInstructorInvitation {
            actor: instructor,
            course,
            target,
        },
    );
    let second_create = store.create_co_instructor_invitation(
        context,
        CreateCoInstructorInvitation {
            actor: instructor,
            course,
            target,
        },
    );
    let (first_create, second_create) = tokio::join!(first_create, second_create);
    let first_create = first_create.expect("first concurrent invite");
    let second_create = second_create.expect("second concurrent invite");
    assert_eq!(first_create.invitation.id, second_create.invitation.id);
    assert_eq!(first_create.revision, second_create.revision);
    let pending_rows: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM course_instructor_invitation \
         WHERE tenant_id=$1 AND course_id=$2 AND target_user_id=$3 AND status='pending'",
    )
    .bind(tenant.as_uuid())
    .bind(course.as_uuid())
    .bind(target.as_uuid())
    .fetch_one(&pool)
    .await
    .expect("one pending row count");
    assert_eq!(
        pending_rows, 1,
        "two Store transactions persist one pending invitation"
    );

    assert_eq!(
        store
            .decline_co_instructor_invitation(
                context,
                RespondToCoInstructorInvitation {
                    session: alternate_session,
                    actor: alternate,
                    invitation: first_create.invitation.id,
                    expected_revision: first_create.revision,
                },
            )
            .await,
        Err(StoreError::NotFound),
        "a non-target cannot decline another account's invitation"
    );
    store
        .decline_co_instructor_invitation(
            context,
            RespondToCoInstructorInvitation {
                session: target_session,
                actor: target,
                invitation: first_create.invitation.id,
                expected_revision: first_create.revision,
            },
        )
        .await
        .expect("target declines its own pending invitation");
    assert_eq!(
        store
            .decline_co_instructor_invitation(
                context,
                RespondToCoInstructorInvitation {
                    session: target_session,
                    actor: target,
                    invitation: first_create.invitation.id,
                    expected_revision: first_create.revision,
                },
            )
            .await,
        Err(StoreError::Conflict),
        "a stale terminal transition conflicts"
    );

    let revocable = store
        .create_co_instructor_invitation(
            context,
            CreateCoInstructorInvitation {
                actor: instructor,
                course,
                target: alternate,
            },
        )
        .await
        .expect("second target invitation");
    store
        .revoke_co_instructor_invitation(
            context,
            RevokeCoInstructorInvitation {
                actor: instructor,
                course,
                invitation: revocable.invitation.id,
                expected_revision: revocable.revision,
            },
        )
        .await
        .expect("direct instructor revokes a pending invitation");
    assert_eq!(
        store
            .revoke_co_instructor_invitation(
                context,
                RevokeCoInstructorInvitation {
                    actor: instructor,
                    course,
                    invitation: revocable.invitation.id,
                    expected_revision: revocable.revision,
                },
            )
            .await,
        Err(StoreError::Conflict),
        "a stale direct-instructor revoke conflicts"
    );

    let accepted_invitation = store
        .create_co_instructor_invitation(
            context,
            CreateCoInstructorInvitation {
                actor: instructor,
                course,
                target,
            },
        )
        .await
        .expect("fresh invitation after decline");
    let accepted = store
        .accept_co_instructor_invitation(
            context,
            RespondToCoInstructorInvitation {
                session: target_session,
                actor: target,
                invitation: accepted_invitation.invitation.id,
                expected_revision: accepted_invitation.revision,
            },
        )
        .await
        .expect("accepted invitation");
    let active_instructor_rows: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM course_member WHERE tenant_id=$1 AND course_id=$2 \
         AND user_id=$3 AND role='instructor' AND status='active' AND student_id IS NULL",
    )
    .bind(tenant.as_uuid())
    .bind(course.as_uuid())
    .bind(target.as_uuid())
    .fetch_one(&pool)
    .await
    .expect("accepted physical membership");
    assert_eq!(
        active_instructor_rows, 1,
        "acceptance creates exactly one non-student Instructor row"
    );
    let accepted_receipt: (String, i64, Option<Uuid>) = sqlx::query_as(
        "SELECT status, revision, accepted_membership_id FROM course_instructor_invitation \
         WHERE tenant_id=$1 AND invitation_id=$2",
    )
    .bind(tenant.as_uuid())
    .bind(accepted_invitation.invitation.id.as_uuid())
    .fetch_one(&pool)
    .await
    .expect("accepted invitation receipt");
    assert_eq!(accepted_receipt.0, "accepted");
    assert_eq!(accepted_receipt.1, 2);
    assert_eq!(accepted_receipt.2, Some(accepted.membership.as_uuid()));
    let replay = store
        .accept_co_instructor_invitation(
            context,
            RespondToCoInstructorInvitation {
                session: target_session,
                actor: target,
                invitation: accepted_invitation.invitation.id,
                expected_revision: accepted_invitation.revision,
            },
        )
        .await
        .expect("accepted replay");
    assert_eq!(
        replay, accepted,
        "replay has neither a duplicate row nor a second roster bump"
    );

    let before_removal_revision: i64 = sqlx::query_scalar(
        "SELECT revision FROM course_roster_state WHERE tenant_id=$1 AND course_id=$2",
    )
    .bind(tenant.as_uuid())
    .bind(course.as_uuid())
    .fetch_one(&pool)
    .await
    .expect("physical roster revision before concurrent removal");
    assert_eq!(
        before_removal_revision,
        i64::try_from(accepted.roster_revision.value()).expect("roster revision fits bigint"),
        "both concurrent removals begin from the shared accepted roster revision"
    );

    let initial = store
        .get_current_course_membership(context, course, instructor)
        .await
        .expect("initial membership lookup")
        .expect("initial direct instructor");
    let remove_initial = store.remove_direct_instructor_membership(
        context,
        RemoveDirectInstructorMembership {
            actor: instructor,
            course,
            membership: initial.id,
            expected_roster_revision: accepted.roster_revision,
        },
    );
    let remove_target = store.remove_direct_instructor_membership(
        context,
        RemoveDirectInstructorMembership {
            actor: target,
            course,
            membership: accepted.membership,
            expected_roster_revision: accepted.roster_revision,
        },
    );
    let removals = tokio::time::timeout(std::time::Duration::from_secs(10), async {
        tokio::join!(remove_initial, remove_target)
    })
    .await
    .expect("concurrent final-instructor protection must not hang");
    assert!(
        matches!(
            (&removals.0, &removals.1),
            (Ok(()), Err(StoreError::Conflict)) | (Err(StoreError::Conflict), Ok(()))
        ),
        "exactly one of two concurrent direct-instructor removals succeeds"
    );
    let remaining_instructors: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM course_member WHERE tenant_id=$1 AND course_id=$2 \
         AND role='instructor' AND status='active'",
    )
    .bind(tenant.as_uuid())
    .bind(course.as_uuid())
    .fetch_one(&pool)
    .await
    .expect("remaining instructor count");
    assert_eq!(
        remaining_instructors, 1,
        "a course retains one final active Instructor"
    );
    let after_removal_revision: i64 = sqlx::query_scalar(
        "SELECT revision FROM course_roster_state WHERE tenant_id=$1 AND course_id=$2",
    )
    .bind(tenant.as_uuid())
    .bind(course.as_uuid())
    .fetch_one(&pool)
    .await
    .expect("physical roster revision after concurrent removal");
    assert_eq!(
        after_removal_revision,
        before_removal_revision + 1,
        "exactly one successful removal advances the physical roster CAS revision"
    );
    let (revoked_membership, active_membership) = match (&removals.0, &removals.1) {
        (Ok(()), Err(StoreError::Conflict)) => (initial.id, accepted.membership),
        (Err(StoreError::Conflict), Ok(())) => (accepted.membership, initial.id),
        _ => unreachable!("the outcome assertion above accepts exactly one removal"),
    };
    let removal_statuses: (String, String) = sqlx::query_as(concat!(
        "SELECT (SELECT status FROM course_member WHERE tenant_id=$1 AND course_id=$2 ",
        "AND course_membership_id=$3), ",
        "(SELECT status FROM course_member WHERE tenant_id=$1 AND course_id=$2 ",
        "AND course_membership_id=$4)",
    ))
    .bind(tenant.as_uuid())
    .bind(course.as_uuid())
    .bind(revoked_membership.as_uuid())
    .bind(active_membership.as_uuid())
    .fetch_one(&pool)
    .await
    .expect("physical final-instructor removal statuses");
    assert_eq!(
        removal_statuses,
        ("revoked".into(), "active".into()),
        "the one successful removal is revoked and the remaining Instructor stays active"
    );

    store
        .revoke_session(alternate_session)
        .await
        .expect("fixture session revoke");
    assert!(
        store
            .list_pending_co_instructor_invitations(
                context,
                alternate_session,
                PageRequest::first(PageSize::new(10).expect("page size")),
            )
            .await
            .is_err(),
        "a revoked session is rejected"
    );
    let expired_session = session(&store, tenant, alternate, vec![UserRole::Instructor]).await;
    sqlx::query("UPDATE auth_session SET expires_at=transaction_timestamp() WHERE session_hash=$1")
        .bind(expired_session.to_string())
        .execute(&pool)
        .await
        .expect("bounded expired-session fixture update");
    assert!(
        store
            .list_pending_co_instructor_invitations(
                context,
                expired_session,
                PageRequest::first(PageSize::new(10).expect("page size")),
            )
            .await
            .is_err(),
        "an expired session is rejected"
    );
    let email_columns: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM information_schema.columns WHERE table_schema='public' \
         AND table_name='course_instructor_invitation' AND column_name ILIKE '%email%'",
    )
    .fetch_one(&pool)
    .await
    .expect("invitation column inspection");
    assert_eq!(
        email_columns, 0,
        "teaching-authority invitation storage contains no email column"
    );
    assert_eq!(
        target_session.to_string().len(),
        64,
        "ordinary sessions use the complete hash path"
    );
    approval_function_guards(&pool).await;
}
