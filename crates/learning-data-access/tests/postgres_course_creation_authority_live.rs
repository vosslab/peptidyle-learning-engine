#![cfg(feature = "postgres")]

//! Disposable PostgreSQL 17 oracle for the closed course-provisioning authority.
//! This database-facing test proves that the `1818` capability boundary remains closed when a
//! future Rust caller changes or migration privilege is accidentally widened.

use learning_data_access::postgres::{
    apply_migrations, lazy_pool, migration_status, verify_base_course_freshness_capability,
};
use serde_json::{Value, json};
use sqlx::{PgPool, Postgres, Row, Transaction};
use uuid::Uuid;

macro_rules! expect_rejection {
    ($transaction:ident, $operation:expr, $message:literal) => {{
        sqlx::query("SAVEPOINT expected_rejection")
            .execute(&mut *$transaction)
            .await
            .expect("expected-rejection savepoint");
        assert!($operation.await.is_err(), $message);
        sqlx::query("ROLLBACK TO SAVEPOINT expected_rejection")
            .execute(&mut *$transaction)
            .await
            .expect("expected-rejection rollback");
        sqlx::query("RELEASE SAVEPOINT expected_rejection")
            .execute(&mut *$transaction)
            .await
            .expect("expected-rejection release");
    }};
}

#[path = "postgres_course_creation_authority_live/catalog.rs"]
mod catalog;
#[path = "postgres_course_creation_authority_live/prefix_support.rs"]
mod prefix_support;
#[path = "postgres_course_creation_authority_live/recipe_guards.rs"]
mod recipe_guards;
#[path = "postgres_course_creation_authority_live/roster_mutator.rs"]
mod roster_mutator;
use prefix_support::*;

const COURSE_CREATION_MIGRATION: i64 = 2_026_081_818;
const BASE_COURSE_FRESHNESS_REGISTRATION_MIGRATION: i64 = 2_026_081_835;
const RECEIPT: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
static MIGRATION_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

fn id() -> Uuid {
    let mut bytes = [0_u8; 16];
    getrandom::fill(&mut bytes).expect("fixture UUID randomness");
    Uuid::from_bytes(bytes)
}

fn session_hash(seed: u8) -> String {
    let mut hash = format!("{}{}", id().simple(), id().simple());
    hash.replace_range(..2, &format!("{seed:02x}"));
    hash
}

async fn pool() -> PgPool {
    let _migration_guard = MIGRATION_LOCK.lock().await;
    let runtime = load_acceptance_runtime();
    let url = runtime.admin_url().expose();
    let pool = lazy_pool(url).expect("valid disposable PostgreSQL URL");
    apply_migrations(&pool)
        .await
        .expect("full migration epoch applies");
    apply_migrations(&pool)
        .await
        .expect("migration epoch converges");
    verify_base_course_freshness_capability(&pool)
        .await
        .expect("Base Course freshness capability is catalog-complete");
    let version: i32 = sqlx::query_scalar("SELECT current_setting('server_version_num')::int4")
        .fetch_one(&pool)
        .await
        .expect("PostgreSQL version is readable");
    assert!(
        (170_000..180_000).contains(&version),
        "the authority oracle requires PostgreSQL 17, found {version}"
    );
    let status = migration_status(&pool)
        .await
        .expect("migration ledger is readable");
    assert!(
        status.is_compatible(),
        "complete migration epoch is compatible"
    );
    assert!(
        status
            .entries()
            .iter()
            .any(|entry| entry.version() == COURSE_CREATION_MIGRATION),
        "course-creation authority migration is installed"
    );
    assert!(
        status
            .entries()
            .iter()
            .any(|entry| entry.version() == BASE_COURSE_FRESHNESS_REGISTRATION_MIGRATION),
        "Base Course freshness registration migration is installed"
    );
    pool
}

#[derive(Clone, Copy)]
struct People {
    tenant: Uuid,
    instructor: Uuid,
    sysadmin: Uuid,
    unapproved: Uuid,
    foreign: Uuid,
}

#[derive(Clone, Copy)]
struct BaseCoursePeople {
    avery: Uuid,
    elena: Uuid,
    jack: Uuid,
    mary: Uuid,
    morgan: Uuid,
}

async fn seed_people(pool: &PgPool) -> People {
    let people = People {
        tenant: id(),
        instructor: id(),
        sysadmin: id(),
        unapproved: id(),
        foreign: id(),
    };
    for (user, label, name, roles) in [
        (
            people.instructor,
            "instructor",
            "Approved Instructor",
            json!([]),
        ),
        (
            people.sysadmin,
            "sysadmin",
            "System Administrator",
            json!(["sysadmin"]),
        ),
        (
            people.unapproved,
            "unapproved",
            "Unapproved Instructor",
            json!([]),
        ),
        (people.foreign, "foreign", "Foreign Actor", json!([])),
    ] {
        let email = format!("authority-{label}-{}@example.edu", user.simple());
        sqlx::query("INSERT INTO public.ple_account(user_id,normalized_email,delivery_email,display_name,platform_roles) VALUES($1,$2,$2,$3,$4)")
            .bind(user).bind(email).bind(name).bind(roles).execute(pool).await.expect("account fixture");
    }
    sqlx::query("INSERT INTO public.instructor_approval(user_id,approved_by,approved_at,revision) VALUES($1,$2,transaction_timestamp(),1)")
        .bind(people.instructor).bind(people.sysadmin).execute(pool).await.expect("approved instructor fixture");
    people
}

async fn seed_session(
    pool: &PgPool,
    tenant: Uuid,
    actor: Uuid,
    roles: Value,
    hash: &str,
    active: bool,
) {
    sqlx::query("INSERT INTO public.auth_session(session_hash,tenant_id,user_id,display_name,roles,created_at,expires_at,revoked_at) VALUES($1,$2,$3,'Authority fixture',$4,transaction_timestamp()-interval '2 hours',transaction_timestamp()+CASE WHEN $5 THEN interval '1 hour' ELSE interval '-1 hour' END,NULL)")
        .bind(hash).bind(tenant).bind(actor).bind(roles).bind(active).execute(pool).await.expect("session fixture");
}

async fn app<'a>(pool: &'a PgPool, tenant: Uuid) -> Transaction<'a, Postgres> {
    let mut tx = pool.begin().await.expect("application transaction");
    sqlx::query("SET LOCAL ROLE ple_app")
        .execute(&mut *tx)
        .await
        .expect("application role");
    sqlx::query("SELECT set_config('ple.tenant_id',$1,true)")
        .bind(tenant.to_string())
        .execute(&mut *tx)
        .await
        .expect("tenant context");
    tx
}

async fn sysadmin_contender(
    pool: PgPool,
    tenant: Uuid,
    course: Uuid,
    actor: Uuid,
    session: String,
) -> bool {
    let mut tx = app(&pool, tenant).await;
    let created = create_as_sysadmin(&mut tx, tenant, course, actor, &session)
        .await
        .is_ok();
    if created {
        tx.commit().await.expect("winning contender commit");
    } else {
        tx.rollback().await.expect("losing contender rollback");
    }
    created
}

#[tokio::test]
#[ignore = "requires the private acceptance runtime workspace"]
async fn ordinary_course_creation_authorizes_sessions_and_preserves_complete_aggregates() {
    let pool = pool().await;
    let people = seed_people(&pool).await;
    let instructor_hash = session_hash(1);
    let sysadmin_hash = session_hash(2);
    seed_session(
        &pool,
        people.tenant,
        people.instructor,
        json!(["instructor"]),
        &instructor_hash,
        true,
    )
    .await;
    seed_session(
        &pool,
        people.tenant,
        people.sysadmin,
        json!(["sysadmin"]),
        &sysadmin_hash,
        true,
    )
    .await;
    let instructor_course = id();
    let sysadmin_course = id();
    let mut tx = app(&pool, people.tenant).await;
    assert_eq!(
        create_as_instructor(
            &mut tx,
            people.tenant,
            instructor_course,
            people.instructor,
            &instructor_hash
        )
        .await
        .expect("approved instructor creates course")
        .0,
        instructor_course
    );
    tx.commit().await.expect("instructor commit");
    let mut tx = app(&pool, people.tenant).await;
    assert_eq!(
        create_as_sysadmin(
            &mut tx,
            people.tenant,
            sysadmin_course,
            people.sysadmin,
            &sysadmin_hash
        )
        .await
        .expect("sysadmin creates course")
        .0,
        sysadmin_course
    );
    tx.commit().await.expect("sysadmin commit");
    assert_complete_aggregate(
        &pool,
        people.tenant,
        instructor_course,
        people.instructor,
        1,
    )
    .await;
    assert_complete_aggregate(&pool, people.tenant, sysadmin_course, people.sysadmin, 1).await;

    let denied = [
        (
            people.tenant,
            people.unapproved,
            json!(["instructor"]),
            true,
            people.unapproved,
            false,
        ),
        (
            people.tenant,
            people.instructor,
            json!(["student"]),
            true,
            people.instructor,
            false,
        ),
        (
            id(),
            people.instructor,
            json!(["instructor"]),
            true,
            people.instructor,
            false,
        ),
        (
            people.tenant,
            people.instructor,
            json!(["instructor"]),
            false,
            people.instructor,
            false,
        ),
    ];
    for (tenant, session_actor, roles, active, call_actor, _) in denied {
        let hash = session_hash((id().as_u128() & 0xff) as u8);
        seed_session(&pool, tenant, session_actor, roles, &hash, active).await;
        let course = id();
        let mut tx = app(&pool, people.tenant).await;
        assert!(
            create_as_instructor(&mut tx, people.tenant, course, call_actor, &hash)
                .await
                .is_err(),
            "invalid authority is denied"
        );
        tx.rollback().await.expect("denied transaction rollback");
        assert_no_course(&pool, people.tenant, course).await;
    }
    let revoked_hash = session_hash(17);
    seed_session(
        &pool,
        people.tenant,
        people.instructor,
        json!(["instructor"]),
        &revoked_hash,
        true,
    )
    .await;
    sqlx::query(
        "UPDATE public.auth_session SET revoked_at=transaction_timestamp() WHERE session_hash=$1",
    )
    .bind(&revoked_hash)
    .execute(&pool)
    .await
    .expect("revoke session");
    let revoked_course = id();
    let mut tx = app(&pool, people.tenant).await;
    assert!(
        create_as_instructor(
            &mut tx,
            people.tenant,
            revoked_course,
            people.instructor,
            &revoked_hash
        )
        .await
        .is_err()
    );
    tx.rollback().await.expect("revoked rollback");
    assert_no_course(&pool, people.tenant, revoked_course).await;
    sqlx::query(
        "UPDATE public.instructor_approval SET revoked_at=transaction_timestamp() WHERE user_id=$1",
    )
    .bind(people.instructor)
    .execute(&pool)
    .await
    .expect("revoke approval");
    let approval_course = id();
    let mut tx = app(&pool, people.tenant).await;
    assert!(
        create_as_instructor(
            &mut tx,
            people.tenant,
            approval_course,
            people.instructor,
            &instructor_hash
        )
        .await
        .is_err()
    );
    tx.rollback().await.expect("approval rollback");
    assert_no_course(&pool, people.tenant, approval_course).await;
}

#[tokio::test]
#[ignore = "requires the private acceptance runtime workspace"]
async fn ordinary_authority_blocks_direct_dml_private_core_and_duplicate_identity() {
    let pool = pool().await;
    let people = seed_people(&pool).await;
    let hash = session_hash(3);
    seed_session(
        &pool,
        people.tenant,
        people.sysadmin,
        json!(["sysadmin"]),
        &hash,
        true,
    )
    .await;
    let direct_course = id();
    let mut tx = app(&pool, people.tenant).await;
    expect_rejection!(tx, sqlx::query("INSERT INTO public.course(tenant_id,course_id,title,term_start_date,term_end_date,time_zone) VALUES($1,$2,'direct',DATE '2026-01-01',DATE '2026-12-31','America/Chicago')").bind(people.tenant).bind(direct_course).execute(&mut *tx), "application direct course DML is denied");
    expect_rejection!(tx, sqlx::query("SELECT * FROM public.ple_create_course_core_internal($1,$2,'private',DATE '2026-01-01',DATE '2026-12-31','America/Chicago',$3)").bind(people.tenant).bind(id()).bind(people.sysadmin).fetch_one(&mut *tx), "application cannot call private core");
    tx.rollback().await.expect("denial rollback");
    assert_no_course(&pool, people.tenant, direct_course).await;
    let course = id();
    let mut tx = app(&pool, people.tenant).await;
    create_as_sysadmin(&mut tx, people.tenant, course, people.sysadmin, &hash)
        .await
        .expect("first course creation");
    tx.commit().await.expect("first course commit");
    let mut tx = app(&pool, people.tenant).await;
    assert!(
        create_as_sysadmin(&mut tx, people.tenant, course, people.sysadmin, &hash)
            .await
            .is_err(),
        "duplicate course identity is rejected"
    );
    tx.rollback().await.expect("duplicate rollback");
    assert_complete_aggregate(&pool, people.tenant, course, people.sysadmin, 1).await;
}

#[tokio::test]
#[ignore = "requires the private acceptance runtime workspace"]
async fn capability_matrix_and_authority_races_remain_serialized() {
    let pool = pool().await;
    let people = seed_people(&pool).await;
    let hash = session_hash(4);
    seed_session(
        &pool,
        people.tenant,
        people.sysadmin,
        json!(["sysadmin"]),
        &hash,
        true,
    )
    .await;
    let matrix = sqlx::query("SELECT has_table_privilege('ple_app','public.course','INSERT'),has_table_privilege('ple_base_course_installer','public.live_demo_install_state','INSERT'),has_table_privilege('ple_course_creation_broker','public.ple_account','SELECT'),has_function_privilege('ple_app','public.ple_create_course_core_internal(uuid,uuid,text,date,date,text,uuid)','EXECUTE'),has_function_privilege('ple_base_course_installer','public.ple_create_course_core_internal(uuid,uuid,text,date,date,text,uuid)','EXECUTE'),has_function_privilege('ple_app','public.ple_create_course_as_sysadmin_v1(uuid,uuid,text,date,date,text,uuid,character)','EXECUTE')")
        .fetch_one(&pool).await.expect("capability matrix");
    assert!(!matrix.try_get::<bool, _>(0).expect("app direct DML"));
    assert!(!matrix.try_get::<bool, _>(1).expect("installer direct DML"));
    assert!(
        !matrix
            .try_get::<bool, _>(2)
            .expect("ordinary broker account read")
    );
    assert!(!matrix.try_get::<bool, _>(3).expect("app private core"));
    assert!(
        !matrix
            .try_get::<bool, _>(4)
            .expect("installer private core")
    );
    assert!(matrix.try_get::<bool, _>(5).expect("app public broker"));

    let course = id();
    let (left, right) = tokio::join!(
        sysadmin_contender(
            pool.clone(),
            people.tenant,
            course,
            people.sysadmin,
            hash.clone()
        ),
        sysadmin_contender(pool.clone(), people.tenant, course, people.sysadmin, hash),
    );
    assert_eq!(
        u8::from(left) + u8::from(right),
        1,
        "one same-course contender commits"
    );
    assert_complete_aggregate(&pool, people.tenant, course, people.sysadmin, 1).await;

    let instructor_hash = session_hash(5);
    seed_session(
        &pool,
        people.tenant,
        people.instructor,
        json!(["instructor"]),
        &instructor_hash,
        true,
    )
    .await;
    let mut revoke = pool.begin().await.expect("revocation transaction");
    sqlx::query(
        "UPDATE public.instructor_approval SET revoked_at=transaction_timestamp() WHERE user_id=$1",
    )
    .bind(people.instructor)
    .execute(&mut *revoke)
    .await
    .expect("lock approval for revocation");
    let race_course = id();
    let contender = tokio::spawn({
        let pool = pool.clone();
        let hash = instructor_hash.clone();
        async move {
            let mut tx = app(&pool, people.tenant).await;
            let denied = create_as_instructor(
                &mut tx,
                people.tenant,
                race_course,
                people.instructor,
                &hash,
            )
            .await
            .is_err();
            tx.rollback().await.expect("revocation contender rollback");
            denied
        }
    });
    revoke
        .commit()
        .await
        .expect("revocation commits before authorization releases");
    assert!(
        contender.await.expect("revocation contender joins"),
        "authorization observes the serialized revocation"
    );
    assert_no_course(&pool, people.tenant, race_course).await;
}

#[tokio::test]
#[ignore = "requires the private acceptance runtime workspace"]
async fn installer_capability_is_locked_recipe_bound_and_converges_exact_course_prefixes() {
    let isolated = recipe_guards::IsolatedDatabase::provision("installer").await;
    let pool = isolated.pool.clone();
    let people = recipe_guards::unseeded_people();
    let base = id();
    let genetics = id();
    let (base_people, retained) = recipe(base, genetics);
    let mut no_lock = installer(&pool).await;
    assert!(
        prepare(&mut no_lock, people.tenant, retained.clone())
            .await
            .is_err(),
        "prepare requires the checked-out advisory-lock connection"
    );
    no_lock.rollback().await.expect("no-lock rollback");
    let mut install = installer(&pool).await;
    acquire(&mut install).await;
    recipe_guards::assert_preparation_rejections(
        &mut install,
        people.tenant,
        &retained,
        base_people,
    )
    .await;
    for invalid in [
        Value::Null,
        json!({}),
        json!({"schemaVersion":1,"participants":{},"courses":{},"unexpected":true}),
    ] {
        assert!(
            prepare(&mut install, people.tenant, invalid).await.is_err(),
            "invalid recipe has no retained state"
        );
        assert_eq!(installer_state(&mut install).await, None);
    }
    let mut null_scalar = retained.clone();
    null_scalar["participants"]["elena"] = Value::Null;
    assert!(
        prepare(&mut install, people.tenant, null_scalar)
            .await
            .is_err(),
        "null participant scalar has no retained state"
    );
    assert_eq!(installer_state(&mut install).await, None);
    let mut non_string_scalar = retained.clone();
    non_string_scalar["courses"]["baseCourse"]["title"] = json!(7);
    assert!(
        prepare(&mut install, people.tenant, non_string_scalar)
            .await
            .is_err(),
        "non-string recipe scalar has no retained state"
    );
    let mut participant_collision = retained.clone();
    participant_collision["participants"]["mary"] = json!(base_people.elena);
    assert!(
        prepare(&mut install, people.tenant, participant_collision)
            .await
            .is_err(),
        "participant UUIDs must be distinct"
    );
    for path in [
        ["courses", "baseCourse", "id"],
        ["courses", "geneticsPractice", "id"],
    ] {
        let mut collision = retained.clone();
        collision[path[0]][path[1]][path[2]] = json!(base_people.elena);
        assert!(
            prepare(&mut install, people.tenant, collision)
                .await
                .is_err(),
            "course UUID cannot collide with a participant UUID"
        );
    }
    let prepared = prepare(&mut install, people.tenant, retained.clone())
        .await
        .expect("valid recipe prepares");
    assert_eq!(prepared.0, "installing");
    let generation = prepared.1;
    expect_rejection!(install, sqlx::query("SELECT public.ple_base_course_install_complete_v2($1,$2,'base-course-v1','[]'::jsonb,$3)").bind(people.tenant).bind(generation).bind(RECEIPT).execute(&mut *install), "completion cannot skip account and course seeds");
    let state = installer_state(&mut install)
        .await
        .expect("retained lifecycle state");
    assert_eq!(
        state, "installing",
        "failed early completion retains the installing generation"
    );
    recipe_guards::assert_retained_recipe_substitutions(&mut install, people.tenant, &retained)
        .await;
    for (user, email) in [
        (base_people.avery, "wrong-avery@example.edu"),
        (base_people.elena, "wrong-elena@example.edu"),
        (base_people.jack, "wrong-jack@example.edu"),
        (base_people.mary, "wrong-mary@example.edu"),
        (base_people.morgan, "wrong-morgan@example.edu"),
    ] {
        owner_role(&mut install).await;
        sqlx::query("INSERT INTO public.ple_account(user_id,normalized_email,delivery_email,display_name,platform_roles) VALUES($1,$2,$2,'Wrong retained field','[]'::jsonb)")
            .bind(user).bind(email).execute(&mut *install).await.expect("wrong account fixture");
        become_installer(&mut install).await;
        expect_rejection!(
            install,
            sqlx::query("SELECT public.ple_base_course_install_seed_accounts_v2($1)")
                .bind(generation)
                .execute(&mut *install),
            "wrong fixed account field for a participant conflicts"
        );
        owner_role(&mut install).await;
        sqlx::query("DELETE FROM public.ple_account WHERE user_id=$1")
            .bind(user)
            .execute(&mut *install)
            .await
            .expect("remove wrong account fixture");
        become_installer(&mut install).await;
    }
    sqlx::query("SELECT public.ple_base_course_install_seed_accounts_v2($1)")
        .bind(generation)
        .execute(&mut *install)
        .await
        .expect("seed exact accounts");
    owner_role(&mut install).await;
    let accounts: Vec<(Uuid, String, String, String, Value)> = sqlx::query_as("SELECT user_id,normalized_email,delivery_email,display_name,platform_roles FROM public.ple_account WHERE user_id=ANY($1) ORDER BY normalized_email")
        .bind(vec![base_people.avery, base_people.elena, base_people.jack, base_people.mary, base_people.morgan])
        .fetch_all(&mut *install).await.expect("seeded accounts");
    assert_eq!(
        accounts,
        vec![
            (
                base_people.avery,
                "avery.singh@live-demo.ple.example".to_owned(),
                "avery.singh@live-demo.ple.example".to_owned(),
                "Avery Singh".to_owned(),
                json!([])
            ),
            (
                base_people.elena,
                "elena.rivera@live-demo.ple.example".to_owned(),
                "elena.rivera@live-demo.ple.example".to_owned(),
                "Dr. Elena Rivera".to_owned(),
                json!([])
            ),
            (
                base_people.jack,
                "jack.chen@live-demo.ple.example".to_owned(),
                "jack.chen@live-demo.ple.example".to_owned(),
                "Jack Chen".to_owned(),
                json!([])
            ),
            (
                base_people.mary,
                "mary.okafor@live-demo.ple.example".to_owned(),
                "mary.okafor@live-demo.ple.example".to_owned(),
                "Mary Okafor".to_owned(),
                json!([])
            ),
            (
                base_people.morgan,
                "morgan.reyes@live-demo.ple.example".to_owned(),
                "morgan.reyes@live-demo.ple.example".to_owned(),
                "Morgan Reyes".to_owned(),
                json!(["sysadmin"])
            ),
        ],
        "every retained Base Course identity has its canonical fields and role"
    );
    let (approved_by, avery_approval): (Uuid, i64) = sqlx::query_as("SELECT (SELECT approved_by FROM public.instructor_approval WHERE user_id=($1->'participants'->>'elena')::uuid), (SELECT count(*) FROM public.instructor_approval WHERE user_id=($1->'participants'->>'avery')::uuid)").bind(&retained).fetch_one(&mut *install).await.expect("approval shape");
    assert_eq!(approved_by, base_people.morgan, "Morgan approves Elena");
    assert_eq!(avery_approval, 0, "Avery remains unapproved");
    become_installer(&mut install).await;
    recipe_guards::assert_non_elena_approvals_rejected(&mut install, generation, base_people).await;
    owner_role(&mut install).await;
    sqlx::query("UPDATE public.ple_account SET display_name='Drifted Elena' WHERE user_id=$1")
        .bind(base_people.elena)
        .execute(&mut *install)
        .await
        .expect("participant drift fixture");
    become_installer(&mut install).await;
    expect_rejection!(
        install,
        sqlx::query(
            "SELECT * FROM public.ple_base_course_install_seed_course_v2($1,'base_course')"
        )
        .bind(generation)
        .fetch_one(&mut *install),
        "participant drift after account seed blocks course seed"
    );
    owner_role(&mut install).await;
    sqlx::query("UPDATE public.ple_account SET display_name='Dr. Elena Rivera' WHERE user_id=$1")
        .bind(base_people.elena)
        .execute(&mut *install)
        .await
        .expect("restore participant fixture");
    sqlx::query("UPDATE public.instructor_approval SET approved_by=$1 WHERE user_id=$2")
        .bind(base_people.mary)
        .bind(base_people.elena)
        .execute(&mut *install)
        .await
        .expect("approval drift fixture");
    become_installer(&mut install).await;
    expect_rejection!(
        install,
        sqlx::query(
            "SELECT * FROM public.ple_base_course_install_seed_course_v2($1,'base_course')"
        )
        .bind(generation)
        .fetch_one(&mut *install),
        "approval drift after account seed blocks course seed"
    );
    owner_role(&mut install).await;
    sqlx::query("UPDATE public.instructor_approval SET approved_by=$1 WHERE user_id=$2")
        .bind(base_people.morgan)
        .bind(base_people.elena)
        .execute(&mut *install)
        .await
        .expect("restore approval fixture");
    become_installer(&mut install).await;
    expect_rejection!(
        install,
        sqlx::query("SELECT * FROM public.ple_base_course_install_seed_course_v2($1,'wrong_slot')")
            .bind(generation)
            .fetch_one(&mut *install),
        "unknown slot is rejected"
    );
    sqlx::query("SELECT * FROM public.ple_base_course_install_seed_course_v2($1,'base_course')")
        .bind(generation)
        .fetch_one(&mut *install)
        .await
        .expect("seed Base Course slot");
    expect_rejection!(install, sqlx::query("SELECT public.ple_base_course_install_complete_v2($1,$2,'base-course-v1','[]'::jsonb,$3)").bind(people.tenant).bind(generation).bind(RECEIPT).execute(&mut *install), "completion rejects a generation missing one course slot");
    sqlx::query(
        "SELECT * FROM public.ple_base_course_install_seed_course_v2($1,'genetics_practice')",
    )
    .bind(generation)
    .fetch_one(&mut *install)
    .await
    .expect("seed Genetics Practice slot");
    owner_role(&mut install).await;
    sqlx::query("SAVEPOINT extra_membership_fixture")
        .execute(&mut *install)
        .await
        .expect("extra membership fixture savepoint");
    let extra_membership = id();
    sqlx::query("INSERT INTO public.course_member(tenant_id,course_id,course_membership_id,user_id,role,student_id,status,joined_at) VALUES($1,$2,$3,$4,'instructor',NULL,'active',transaction_timestamp())")
        .bind(people.tenant).bind(base).bind(extra_membership).bind(base_people.mary).execute(&mut *install).await.expect("extra retained membership fixture");
    become_installer(&mut install).await;
    assert_seed_refused(&mut install, generation, "base_course").await;
    sqlx::query("ROLLBACK TO SAVEPOINT extra_membership_fixture")
        .execute(&mut *install)
        .await
        .expect("remove extra membership through fixture rollback");
    sqlx::query("RELEASE SAVEPOINT extra_membership_fixture")
        .execute(&mut *install)
        .await
        .expect("extra membership fixture release");
    become_installer(&mut install).await;
    owner_role(&mut install).await;
    sqlx::query(
        "UPDATE public.course_appearance SET theme_id='forest' WHERE tenant_id=$1 AND course_id=$2",
    )
    .bind(people.tenant)
    .bind(base)
    .execute(&mut *install)
    .await
    .expect("tamper retained aggregate fixture");
    become_installer(&mut install).await;
    assert_seed_refused(&mut install, generation, "base_course").await;
    owner_role(&mut install).await;
    sqlx::query(
        "UPDATE public.course_appearance SET theme_id='grass' WHERE tenant_id=$1 AND course_id=$2",
    )
    .bind(people.tenant)
    .bind(base)
    .execute(&mut *install)
    .await
    .expect("restore retained aggregate fixture");
    become_installer(&mut install).await;
    assert_seed_exact_prefix(&mut install, generation, "base_course").await;
    assert_seed_exact_prefix(&mut install, generation, "genetics_practice").await;
    owner_role(&mut install).await;
    let assignment = retained["graph"]["assignment"]
        .as_str()
        .unwrap()
        .parse::<Uuid>()
        .unwrap();
    sqlx::query(
        "UPDATE public.course_grade_scheme SET revision=2 WHERE tenant_id=$1 AND course_id=$2",
    )
    .bind(people.tenant)
    .bind(base)
    .execute(&mut *install)
    .await
    .expect("wrong pre-assignment Base revision fixture");
    become_installer(&mut install).await;
    assert_seed_refused(&mut install, generation, "base_course").await;
    owner_role(&mut install).await;
    sqlx::query(
        "UPDATE public.course_grade_scheme SET revision=1 WHERE tenant_id=$1 AND course_id=$2",
    )
    .bind(people.tenant)
    .bind(base)
    .execute(&mut *install)
    .await
    .expect("restore pre-assignment Base revision");
    sqlx::query(
        "UPDATE public.course_grade_scheme SET revision=2 WHERE tenant_id=$1 AND course_id=$2",
    )
    .bind(people.tenant)
    .bind(genetics)
    .execute(&mut *install)
    .await
    .expect("wrong Practice revision fixture");
    become_installer(&mut install).await;
    assert_seed_refused(&mut install, generation, "genetics_practice").await;
    owner_role(&mut install).await;
    sqlx::query(
        "UPDATE public.course_grade_scheme SET revision=1 WHERE tenant_id=$1 AND course_id=$2",
    )
    .bind(people.tenant)
    .bind(genetics)
    .execute(&mut *install)
    .await
    .expect("restore Practice revision");
    sqlx::query("SAVEPOINT wrong_only_assignment_phase")
        .execute(&mut *install)
        .await
        .expect("wrong-only assignment savepoint");
    insert_phase_assignment(
        &mut install,
        people.tenant,
        id(),
        base,
        "Wrong-only phase fixture",
    )
    .await;
    become_installer(&mut install).await;
    assert_seed_refused(&mut install, generation, "base_course").await;
    sqlx::query("ROLLBACK TO SAVEPOINT wrong_only_assignment_phase")
        .execute(&mut *install)
        .await
        .expect("remove wrong-only assignment");
    sqlx::query("RELEASE SAVEPOINT wrong_only_assignment_phase")
        .execute(&mut *install)
        .await
        .expect("release wrong-only assignment savepoint");
    owner_role(&mut install).await;
    insert_phase_assignment(
        &mut install,
        people.tenant,
        assignment,
        base,
        "Exact phase fixture",
    )
    .await;
    sqlx::query(
        "UPDATE public.course_grade_scheme SET revision=2 WHERE tenant_id=$1 AND course_id=$2",
    )
    .bind(people.tenant)
    .bind(base)
    .execute(&mut *install)
    .await
    .expect("post-assignment scheme revision fixture");
    become_installer(&mut install).await;
    assert_seed_exact_prefix(&mut install, generation, "base_course").await;
    assert_seed_exact_prefix(&mut install, generation, "genetics_practice").await;
    owner_role(&mut install).await;
    sqlx::query(
        "UPDATE public.course_grade_scheme SET revision=1 WHERE tenant_id=$1 AND course_id=$2",
    )
    .bind(people.tenant)
    .bind(base)
    .execute(&mut *install)
    .await
    .expect("wrong post-assignment Base revision fixture");
    become_installer(&mut install).await;
    assert_seed_refused(&mut install, generation, "base_course").await;
    owner_role(&mut install).await;
    sqlx::query(
        "UPDATE public.course_grade_scheme SET revision=2 WHERE tenant_id=$1 AND course_id=$2",
    )
    .bind(people.tenant)
    .bind(base)
    .execute(&mut *install)
    .await
    .expect("restore post-assignment Base revision");
    sqlx::query("SAVEPOINT practice_assignment_phase")
        .execute(&mut *install)
        .await
        .expect("Practice assignment savepoint");
    insert_phase_assignment(
        &mut install,
        people.tenant,
        id(),
        genetics,
        "Practice phase fixture",
    )
    .await;
    become_installer(&mut install).await;
    assert_seed_refused(&mut install, generation, "genetics_practice").await;
    sqlx::query("ROLLBACK TO SAVEPOINT practice_assignment_phase")
        .execute(&mut *install)
        .await
        .expect("remove Practice assignment");
    sqlx::query("RELEASE SAVEPOINT practice_assignment_phase")
        .execute(&mut *install)
        .await
        .expect("release Practice assignment savepoint");
    owner_role(&mut install).await;
    sqlx::query("SAVEPOINT extra_base_assignment_phase")
        .execute(&mut *install)
        .await
        .expect("extra Base assignment savepoint");
    insert_phase_assignment(
        &mut install,
        people.tenant,
        id(),
        base,
        "Extra Base phase fixture",
    )
    .await;
    become_installer(&mut install).await;
    assert_seed_refused(&mut install, generation, "base_course").await;
    sqlx::query("ROLLBACK TO SAVEPOINT extra_base_assignment_phase")
        .execute(&mut *install)
        .await
        .expect("remove extra Base assignment");
    sqlx::query("RELEASE SAVEPOINT extra_base_assignment_phase")
        .execute(&mut *install)
        .await
        .expect("release extra Base assignment savepoint");
    owner_role(&mut install).await;
    become_installer(&mut install).await;
    expect_rejection!(install, sqlx::query("SELECT * FROM public.ple_base_course_install_complete_v2($1,$2,'base-course-v1','[]'::jsonb,$3)").bind(people.tenant).bind(generation).bind(RECEIPT).fetch_one(&mut *install), "READ COMMITTED completion is refused before graph verification");
    assert_eq!(
        installer_state(&mut install).await.as_deref(),
        Some("installing")
    );
    sqlx::query("SELECT public.ple_base_course_install_release_lock_v1()")
        .execute(&mut *install)
        .await
        .expect("release lock");
    owner_role(&mut install).await;
    install.commit().await.expect("installer commit");
    assert_complete_aggregate(&pool, people.tenant, base, base_people.elena, 2).await;
    assert_complete_aggregate(&pool, people.tenant, genetics, base_people.morgan, 1).await;
    isolated.cleanup().await;
}
#[path = "support/acceptance_runtime.rs"]
mod acceptance_runtime;
use acceptance_runtime::load as load_acceptance_runtime;
