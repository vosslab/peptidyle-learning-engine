//! Closed-shape recipe negatives for the 1818 PostgreSQL authority oracle.

use std::str::FromStr;

use learning_data_access::postgres::apply_migrations;
use sqlx::AssertSqlSafe;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};

use super::*;

// Isolated databases are created concurrently by the authority oracle.  Keep
// their migration setup serialized while the shared live schema is validated
// read-only by the parent fixture.
static ISOLATED_MIGRATION_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

pub(super) struct IsolatedDatabase {
    pub(super) pool: PgPool,
    admin: PgPool,
    name: String,
}

impl IsolatedDatabase {
    pub(super) async fn provision(label: &str) -> Self {
        let runtime = load_acceptance_runtime();
        let url = runtime.admin_url().expose();
        let admin_options = PgConnectOptions::from_str(url)
            .expect("disposable PostgreSQL URL")
            .database("postgres");
        let admin = PgPoolOptions::new()
            .max_connections(2)
            .connect_with(admin_options)
            .await
            .expect("isolated database admin connection");
        let name = format!("ple_1818_{label}_{:x}", id().as_u128());
        assert!(name.len() < 64);
        sqlx::query(AssertSqlSafe(format!("CREATE DATABASE {name}")))
            .execute(&admin)
            .await
            .expect("create isolated authority database");
        let product_options = PgConnectOptions::from_str(url)
            .expect("disposable PostgreSQL URL")
            .database(&name);
        let pool = PgPoolOptions::new()
            .max_connections(4)
            .connect_with(product_options)
            .await
            .expect("isolated authority database connection");
        let _migration_guard = ISOLATED_MIGRATION_LOCK.lock().await;
        apply_migrations(&pool)
            .await
            .expect("migrate isolated authority database");
        Self { pool, admin, name }
    }

    pub(super) async fn cleanup(self) {
        self.pool.close().await;
        let _ =
            sqlx::query("SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE datname=$1")
                .bind(&self.name)
                .execute(&self.admin)
                .await;
        sqlx::query(AssertSqlSafe(format!(
            "DROP DATABASE IF EXISTS {}",
            self.name
        )))
        .execute(&self.admin)
        .await
        .expect("drop isolated authority database");
    }
}

pub(super) fn unseeded_people() -> People {
    People {
        tenant: id(),
        instructor: id(),
        sysadmin: id(),
        unapproved: id(),
        foreign: id(),
    }
}

async fn assert_rejected(
    installer: &mut Transaction<'_, Postgres>,
    tenant: Uuid,
    recipe: Value,
    reason: &str,
) {
    assert!(
        prepare(installer, tenant, recipe).await.is_err(),
        "{reason} is rejected before a recipe is retained"
    );
    assert_eq!(
        installer_state(installer).await,
        None,
        "{reason} leaves no retained recipe"
    );
}

pub async fn assert_preparation_rejections(
    installer: &mut Transaction<'_, Postgres>,
    tenant: Uuid,
    canonical: &Value,
    people: BaseCoursePeople,
) {
    for version in [json!("1"), json!(2)] {
        let mut candidate = canonical.clone();
        candidate["schemaVersion"] = version;
        assert_rejected(installer, tenant, candidate, "non-canonical schema version").await;
    }
    let mut non_string_participant = canonical.clone();
    non_string_participant["participants"]["jack"] = json!(7);
    assert_rejected(
        installer,
        tenant,
        non_string_participant,
        "non-string participant scalar",
    )
    .await;
    let mut malformed_participant = canonical.clone();
    malformed_participant["participants"]["mary"] = json!("not-a-uuid");
    assert_rejected(
        installer,
        tenant,
        malformed_participant,
        "malformed participant UUID",
    )
    .await;
    let mut nested_unknown = canonical.clone();
    nested_unknown["courses"]["baseCourse"]["unreviewed"] = json!(true);
    assert_rejected(
        installer,
        tenant,
        nested_unknown,
        "nested unknown course key",
    )
    .await;
    let mut wrong_course_scalar = canonical.clone();
    wrong_course_scalar["courses"]["geneticsPractice"]["timeZone"] = json!("UTC");
    assert_rejected(
        installer,
        tenant,
        wrong_course_scalar,
        "wrong fixed course scalar",
    )
    .await;
    let mut participant_collision = canonical.clone();
    participant_collision["participants"]["avery"] = json!(people.elena);
    assert_rejected(
        installer,
        tenant,
        participant_collision,
        "participant UUID collision",
    )
    .await;
    let mut course_collision = canonical.clone();
    course_collision["courses"]["geneticsPractice"]["id"] = json!(people.morgan);
    assert_rejected(
        installer,
        tenant,
        course_collision,
        "course and participant UUID collision",
    )
    .await;
    {
        let installer = &mut **installer;
        expect_rejection!(
            installer,
            sqlx::query("SELECT public.ple_base_course_install_validate_recipe_internal($1)")
                .bind(canonical)
                .fetch_one(&mut *installer),
            "installer cannot execute the private recipe validator"
        );
    }
}

pub async fn assert_retained_recipe_substitutions(
    installer: &mut Transaction<'_, Postgres>,
    tenant: Uuid,
    canonical: &Value,
) {
    let mut participant = canonical.clone();
    participant["participants"]["elena"] = json!(id());
    let mut approval = canonical.clone();
    approval["courses"]["baseCourse"]["initialInstructor"] = json!(id());
    let mut course = canonical.clone();
    course["courses"]["baseCourse"]["id"] = json!(id());
    for candidate in [participant, approval, course] {
        assert!(
            prepare(installer, tenant, candidate).await.is_err(),
            "a retained participant, approval binding, or course identity cannot be substituted"
        );
    }
}

pub async fn assert_non_elena_approvals_rejected(
    installer: &mut Transaction<'_, Postgres>,
    generation: Uuid,
    people: BaseCoursePeople,
) {
    for target in [people.mary, people.jack, people.morgan] {
        owner_role(installer).await;
        sqlx::query("INSERT INTO public.instructor_approval(user_id,approved_by,approved_at,revision) VALUES($1,$2,transaction_timestamp(),1)")
            .bind(target)
            .bind(people.morgan)
            .execute(&mut **installer)
            .await
            .expect("wrong active approval fixture");
        become_installer(installer).await;
        {
            let installer = &mut **installer;
            expect_rejection!(
                installer,
                sqlx::query("SELECT public.ple_base_course_install_seed_accounts_v2($1)")
                    .bind(generation)
                    .execute(&mut *installer),
                "active approval for any non-Elena identity conflicts"
            );
        }
        owner_role(installer).await;
        sqlx::query("DELETE FROM public.instructor_approval WHERE user_id=$1")
            .bind(target)
            .execute(&mut **installer)
            .await
            .expect("remove wrong approval fixture");
        become_installer(installer).await;
    }
}
