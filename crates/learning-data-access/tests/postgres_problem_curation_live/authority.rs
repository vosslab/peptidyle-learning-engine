use std::fs;
use std::str::FromStr;

use learning_data_access::{
    PageRequest, PageSize, ProblemCollectionReplacementTarget, ProblemCurationCapability,
    ProblemCurationStore, ReplaceProblemCollectionCommand, StoreError,
};
use question_model::{ProblemCollectionAccess, ProblemCollectionVisibility};
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use sqlx::{AssertSqlSafe, PgPool, Row};
use uuid::Uuid;

use crate::fixture::Fixture;

#[derive(Clone, Copy, Debug)]
pub(super) enum BrokerAuthorityStage {
    D2,
    FullyMigrated,
}

impl BrokerAuthorityStage {
    fn expected_function_capabilities(self) -> Vec<&'static str> {
        let mut capabilities = vec![
            "public.ple_current_tenant()",
            "public.ple_catalog_discovery_actor(character,uuid)",
            "public.ple_instructor_approval_eligible(uuid)",
            "public.ple_saved_problem_search_filter_v1_is_valid(jsonb)",
            "public.ple_problem_curation_actor(character,uuid)",
            "public.ple_problem_curation_instructor_actor(character,uuid)",
            "public.ple_problem_curation_preflight_v1(uuid,character,text)",
            "public.ple_problem_collection_readable(uuid,uuid,integer)",
            "public.ple_ensure_problem_favorites_v1(uuid,character)",
            "public.ple_list_problem_collections_v1(uuid,character,integer,integer)",
            "public.ple_problem_collection_summary_v1(uuid,character,integer)",
            "public.ple_problem_collection_members_v1(uuid,character,integer,integer,integer)",
            "public.ple_replace_problem_collection_v1(uuid,character,integer,bigint,text,text,text[])",
            "public.ple_replace_saved_problem_search_v1(uuid,character,integer,bigint,text,jsonb)",
            "public.ple_list_saved_problem_searches_v1(uuid,character,integer,integer)",
            "public.ple_saved_problem_search_v1(uuid,character,integer)",
            "public.ple_delete_problem_collection_v1(uuid,character,integer,bigint)",
            "public.ple_delete_saved_problem_search_v1(uuid,character,integer,bigint)",
        ];
        if matches!(self, Self::FullyMigrated) {
            capabilities.push("public.digest(bytea,text)");
        }
        capabilities
    }
}

fn fresh_id() -> Uuid {
    let mut bytes = [0_u8; 16];
    getrandom::fill(&mut bytes).expect("D2 migration fixture randomness");
    Uuid::from_bytes(bytes)
}

fn migration_copy(maximum_version: Option<i64>) -> std::path::PathBuf {
    let source = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../schemas/migrations");
    let destination = std::env::temp_dir().join(format!("ple-d2-migrations-{}", fresh_id()));
    fs::create_dir_all(&destination).expect("temporary D2 migration directory");
    for entry in fs::read_dir(source).expect("migration directory") {
        let entry = entry.expect("migration entry");
        let name = entry.file_name();
        let version = name
            .to_string_lossy()
            .split('_')
            .next()
            .and_then(|value| value.parse::<i64>().ok())
            .expect("migration filename begins with a numeric version");
        if maximum_version.is_none_or(|maximum| version <= maximum) {
            fs::copy(entry.path(), destination.join(name)).expect("copy D2 migration input");
        }
    }
    destination
}

async fn migration_admin_pool(url: &str) -> PgPool {
    let options = PgConnectOptions::from_str(url)
        .expect("acceptance PostgreSQL URL")
        .database("postgres");
    PgPoolOptions::new()
        .max_connections(2)
        .connect_with(options)
        .await
        .expect("D2 migration admin connection")
}

pub(super) async fn pre_d2_broker_drift_converges(url: &str) {
    let admin = migration_admin_pool(url).await;
    let database = format!("ple_d2_acl_{:x}", fresh_id().as_u128());
    assert!(
        database.len() < 64,
        "generated D2 database identifier is bounded"
    );
    sqlx::query(AssertSqlSafe(format!("CREATE DATABASE {database}")))
        .execute(&admin)
        .await
        .expect("create isolated D2 migration database");
    let cleanup_database = database.clone();
    let source_url = url.to_owned();
    let result = tokio::spawn(async move {
        let options = PgConnectOptions::from_str(&source_url)
            .expect("acceptance PostgreSQL URL")
            .database(&database);
        let pool = PgPoolOptions::new()
            .max_connections(2)
            .connect_with(options)
            .await
            .expect("isolated D2 migration database connection");
        let pre_d2 = migration_copy(Some(2026081835));
        sqlx::migrate::Migrator::new(pre_d2.clone())
            .await
            .expect("pre-D2 migration source")
            .run(&pool)
            .await
            .expect("migrate through 1835");
        sqlx::raw_sql(
            "DO $$ BEGIN IF NOT EXISTS (SELECT 1 FROM pg_roles \
               WHERE rolname='ple_problem_curation_broker') THEN \
               CREATE ROLE ple_problem_curation_broker; END IF; END $$; \
             ALTER ROLE ple_problem_curation_broker LOGIN SUPERUSER CREATEDB CREATEROLE \
               INHERIT REPLICATION BYPASSRLS; \
             GRANT ple_app TO ple_problem_curation_broker; \
             GRANT ple_problem_curation_broker TO ple_student; \
             GRANT ALL PRIVILEGES ON TABLE public.problem_collection,public.course \
               TO ple_problem_curation_broker; \
             GRANT UPDATE (title) ON TABLE public.problem_collection \
               TO ple_problem_curation_broker; \
             GRANT SELECT (version) ON TABLE public.ple_migration_state \
               TO ple_problem_curation_broker; \
             GRANT USAGE,SELECT,UPDATE ON SEQUENCE public.catalog_search_publication_sequence \
               TO ple_problem_curation_broker; \
             GRANT EXECUTE ON FUNCTION public.ple_course_records_accessible(uuid,uuid) \
               TO ple_problem_curation_broker; \
             ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT SELECT ON TABLES \
               TO ple_problem_curation_broker; \
             ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT USAGE ON SEQUENCES \
               TO ple_problem_curation_broker; \
             ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT EXECUTE ON FUNCTIONS \
               TO ple_problem_curation_broker;",
        )
        .execute(&pool)
        .await
        .expect("inject representative pre-D2 broker drift");
        let through_d2 = migration_copy(Some(2026081836));
        sqlx::migrate::Migrator::new(through_d2.clone())
            .await
            .expect("D2 migration source")
            .run(&pool)
            .await
            .expect("1836 converges arbitrary broker drift");
        broker_role_and_forced_rls_are_sealed(&pool, BrokerAuthorityStage::D2).await;
        let applied: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM public._sqlx_migrations WHERE success AND version=2026081836",
        )
        .fetch_one(&pool)
        .await
        .expect("D2 migration ledger row");
        assert_eq!(applied, 1, "canonical D2 migration applies exactly once");
        pool.close().await;
        fs::remove_dir_all(pre_d2).expect("remove pre-D2 migration copy");
        fs::remove_dir_all(through_d2).expect("remove D2 migration copy");
    })
    .await;
    let _ = sqlx::query("SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE datname=$1")
        .bind(&cleanup_database)
        .execute(&admin)
        .await;
    sqlx::raw_sql(
        "ALTER ROLE ple_problem_curation_broker NOLOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE \
           NOINHERIT NOREPLICATION NOBYPASSRLS; \
         REVOKE ple_app FROM ple_problem_curation_broker; \
         REVOKE ple_problem_curation_broker FROM ple_student;",
    )
    .execute(&admin)
    .await
    .expect("restore shared PostgreSQL D2 role posture");
    sqlx::query(AssertSqlSafe(format!(
        "DROP DATABASE IF EXISTS {cleanup_database}"
    )))
    .execute(&admin)
    .await
    .expect("drop isolated D2 migration database");
    result.expect("pre-D2 drift fixture task");
}

pub(super) async fn broker_role_and_forced_rls_are_sealed(
    pool: &PgPool,
    stage: BrokerAuthorityStage,
) {
    let author_ids_index: String = sqlx::query_scalar(
        "SELECT pg_get_indexdef('public.problem_version_author_ids_gin_idx'::regclass)",
    )
    .fetch_one(pool)
    .await
    .expect("authorship GIN index exists");
    assert!(
        author_ids_index.contains("USING gin (author_ids jsonb_path_ops)"),
        "authorship scope uses the immutable-author index"
    );
    let role = sqlx::query(
        "SELECT rolcanlogin, rolsuper, rolcreatedb, rolcreaterole, rolinherit, rolreplication, rolbypassrls \
         FROM pg_roles WHERE rolname='ple_problem_curation_broker'",
    )
    .fetch_one(pool)
    .await
    .expect("D2 broker role exists");
    for column in [
        "rolcanlogin",
        "rolsuper",
        "rolcreatedb",
        "rolcreaterole",
        "rolinherit",
        "rolreplication",
        "rolbypassrls",
    ] {
        assert!(
            !role
                .try_get::<bool, _>(column)
                .expect("closed D2 broker flag"),
            "D2 broker {column} is closed"
        );
    }
    let memberships: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM pg_auth_members WHERE member='ple_problem_curation_broker'::regrole \
         OR roleid='ple_problem_curation_broker'::regrole",
    )
    .fetch_one(pool)
    .await
    .expect("D2 broker memberships");
    assert_eq!(memberships, 0, "D2 broker has no role membership edges");
    let schema_acl: (bool, bool) = sqlx::query_as(
        "SELECT has_schema_privilege('ple_problem_curation_broker','public','USAGE'), \
                has_schema_privilege('ple_problem_curation_broker','public','CREATE')",
    )
    .fetch_one(pool)
    .await
    .expect("D2 broker schema capability");
    assert_eq!(schema_acl, (true, false), "D2 broker has schema usage only");
    let default_acls: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM pg_default_acl AS defaults \
         CROSS JOIN LATERAL aclexplode(defaults.defaclacl) AS privilege \
         WHERE privilege.grantee='ple_problem_curation_broker'::regrole \
           AND defaults.defaclobjtype IN ('r','S','f')",
    )
    .fetch_one(pool)
    .await
    .expect("D2 broker default ACL closure");
    assert_eq!(default_acls, 0, "D2 broker has no ambient default grants");
    let table_acls: Vec<(String, String)> = sqlx::query_as(
        "SELECT relation_row.relname::text,privilege.privilege_type \
         FROM pg_class AS relation_row JOIN pg_namespace AS namespace \
           ON namespace.oid=relation_row.relnamespace \
         CROSS JOIN LATERAL aclexplode(coalesce( \
           relation_row.relacl,acldefault('r',relation_row.relowner))) AS privilege \
         WHERE namespace.nspname='public' AND relation_row.relkind IN ('r','p') \
           AND privilege.grantee='ple_problem_curation_broker'::regrole \
         ORDER BY relation_row.relname,privilege.privilege_type",
    )
    .fetch_all(pool)
    .await
    .expect("D2 broker table ACLs");
    let mut expected_table_acls = vec![
        ("auth_session", "SELECT"),
        ("catalog_search_document", "SELECT"),
        ("catalog_tenant_grant", "SELECT"),
        ("problem_collection", "DELETE"),
        ("problem_collection", "INSERT"),
        ("problem_collection", "SELECT"),
        ("problem_collection", "UPDATE"),
        ("problem_collection_member", "DELETE"),
        ("problem_collection_member", "INSERT"),
        ("problem_collection_member", "SELECT"),
        ("saved_problem_search", "DELETE"),
        ("saved_problem_search", "INSERT"),
        ("saved_problem_search", "SELECT"),
        ("saved_problem_search", "UPDATE"),
    ]
    .into_iter()
    .map(|(relation, privilege)| (relation.to_string(), privilege.to_string()))
    .collect::<Vec<_>>();
    expected_table_acls.sort();
    assert_eq!(table_acls, expected_table_acls, "D2 table ACLs are exact");
    let column_acls: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM pg_attribute AS attribute \
         CROSS JOIN LATERAL aclexplode(attribute.attacl) AS privilege \
         WHERE privilege.grantee='ple_problem_curation_broker'::regrole",
    )
    .fetch_one(pool)
    .await
    .expect("D2 broker column ACLs");
    assert_eq!(column_acls, 0, "D2 broker has no column privilege drift");
    let sequence_acls: Vec<(String, String)> = sqlx::query_as(
        "SELECT sequence_row.relname::text,privilege.privilege_type \
         FROM pg_class AS sequence_row JOIN pg_namespace AS namespace \
           ON namespace.oid=sequence_row.relnamespace \
         CROSS JOIN LATERAL aclexplode(coalesce( \
           sequence_row.relacl,acldefault('S',sequence_row.relowner))) AS privilege \
         WHERE namespace.nspname='public' AND sequence_row.relkind='S' \
           AND privilege.grantee='ple_problem_curation_broker'::regrole \
         ORDER BY sequence_row.relname,privilege.privilege_type",
    )
    .fetch_all(pool)
    .await
    .expect("D2 broker sequence ACLs");
    assert_eq!(
        sequence_acls,
        vec![
            ("problem_collection_reference_seq".into(), "USAGE".into()),
            ("saved_problem_search_reference_seq".into(), "USAGE".into()),
        ],
        "D2 sequence ACLs are exact"
    );
    let expected_function_capabilities = stage
        .expected_function_capabilities()
        .into_iter()
        .map(str::to_owned)
        .collect::<Vec<_>>();
    let expected_function_capability_count = expected_function_capabilities.len() as i64;
    let function_acl: (i64, i64) = sqlx::query_as(
        "WITH expected_capability AS ( \
           SELECT to_regprocedure(capability)::oid AS oid \
             FROM unnest($1::text[]) AS capability \
         ) \
         SELECT count(*),count(*) FILTER (WHERE NOT EXISTS ( \
           SELECT 1 FROM expected_capability \
            WHERE expected_capability.oid=function_row.oid \
         )) \
         FROM pg_proc AS function_row JOIN pg_namespace AS namespace \
           ON namespace.oid=function_row.pronamespace \
         CROSS JOIN LATERAL aclexplode(coalesce( \
           function_row.proacl,acldefault('f',function_row.proowner))) AS privilege \
         WHERE namespace.nspname='public' \
           AND privilege.grantee='ple_problem_curation_broker'::regrole",
    )
    .bind(expected_function_capabilities)
    .fetch_one(pool)
    .await
    .expect("D2 broker function ACLs");
    assert_eq!(
        function_acl,
        (expected_function_capability_count, 0),
        "{stage:?} function ACLs are exact"
    );
    for relation in [
        "problem_collection",
        "problem_collection_member",
        "saved_problem_search",
    ] {
        let forced: (bool, bool) = sqlx::query_as(
            "SELECT relrowsecurity, relforcerowsecurity FROM pg_class \
             WHERE oid=format('public.%I',$1)::regclass",
        )
        .bind(relation)
        .fetch_one(pool)
        .await
        .expect("curation RLS relation");
        assert_eq!(forced, (true, true), "{relation} uses forced RLS");
        for privilege in ["SELECT", "INSERT", "UPDATE", "DELETE"] {
            let direct: bool = sqlx::query_scalar(
                "SELECT has_table_privilege('ple_app',format('public.%I',$1),$2)",
            )
            .bind(relation)
            .bind(privilege)
            .fetch_one(pool)
            .await
            .expect("app direct privilege probe");
            assert!(
                !direct,
                "ple_app reaches {relation} only through the broker"
            );
        }
    }
    let functions: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM pg_proc AS p JOIN pg_roles AS r ON r.oid=p.proowner \
         WHERE p.proname IN ('ple_problem_curation_preflight_v1', \
           'ple_ensure_problem_favorites_v1','ple_list_problem_collections_v1', \
           'ple_problem_collection_summary_v1','ple_problem_collection_members_v1', \
           'ple_replace_problem_collection_v1','ple_replace_saved_problem_search_v1', \
           'ple_list_saved_problem_searches_v1','ple_saved_problem_search_v1', \
           'ple_delete_problem_collection_v1','ple_delete_saved_problem_search_v1') \
           AND p.prosecdef AND r.rolname='ple_problem_curation_broker' \
           AND NOT has_function_privilege('public',p.oid,'EXECUTE')",
    )
    .fetch_one(pool)
    .await
    .expect("D2 broker procedure catalog");
    assert_eq!(
        functions, 11,
        "every D2 public procedure is definer-owned and closed to PUBLIC"
    );
    let executable: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM pg_proc AS p WHERE p.proname IN ( \
          'ple_problem_curation_preflight_v1','ple_ensure_problem_favorites_v1', \
          'ple_list_problem_collections_v1', \
          'ple_problem_collection_summary_v1','ple_problem_collection_members_v1', \
          'ple_replace_problem_collection_v1','ple_replace_saved_problem_search_v1', \
          'ple_list_saved_problem_searches_v1','ple_saved_problem_search_v1', \
          'ple_delete_problem_collection_v1','ple_delete_saved_problem_search_v1') \
          AND has_function_privilege('ple_app',p.oid,'EXECUTE') \
          AND NOT has_function_privilege('public',p.oid,'EXECUTE')",
    )
    .fetch_one(pool)
    .await
    .expect("D2 application procedure grants");
    assert_eq!(
        executable, 11,
        "the application reaches every D2 capability only by procedure"
    );
    let mut invalid_preflight = pool
        .begin()
        .await
        .expect("invalid D2 preflight transaction");
    sqlx::query("SET LOCAL ROLE ple_app")
        .execute(&mut *invalid_preflight)
        .await
        .expect("D2 app role for preflight");
    let error = sqlx::query_scalar::<_, bool>(
        "SELECT public.ple_problem_curation_preflight_v1(NULL,NULL,'ambientAuthority')",
    )
    .fetch_one(&mut *invalid_preflight)
    .await
    .expect_err("unknown D2 preflight capability is rejected");
    assert_eq!(
        error
            .as_database_error()
            .and_then(|value| value.code())
            .as_deref(),
        Some("22023")
    );
    invalid_preflight
        .rollback()
        .await
        .expect("invalid D2 preflight rollback");
    let mut direct = pool.begin().await.expect("D2 direct DML transaction");
    sqlx::query("SET LOCAL ROLE ple_app")
        .execute(&mut *direct)
        .await
        .expect("D2 app role");
    let error = sqlx::query("INSERT INTO public.saved_problem_search DEFAULT VALUES")
        .execute(&mut *direct)
        .await
        .expect_err("direct curation DML is denied");
    assert_eq!(
        error
            .as_database_error()
            .and_then(|value| value.code())
            .as_deref(),
        Some("42501")
    );
    direct.rollback().await.expect("D2 direct DML rollback");
}

pub(super) async fn actor_authority_privacy_and_tenant_isolation(fixture: &Fixture) {
    for session in [
        fixture.elena_session,
        fixture.ada_session,
        fixture.morgan_session,
    ] {
        fixture
            .store
            .preflight_problem_curation(
                fixture.context,
                session,
                ProblemCurationCapability::CatalogInstitutionRead,
            )
            .await
            .expect("Instructor and Sysadmin catalog readers");
    }
    for session in [fixture.elena_session, fixture.ada_session] {
        fixture
            .store
            .preflight_problem_curation(
                fixture.context,
                session,
                ProblemCurationCapability::PersonalMutation,
            )
            .await
            .expect("approved Instructor personal curation");
    }
    assert_eq!(
        fixture
            .store
            .preflight_problem_curation(
                fixture.context,
                fixture.morgan_session,
                ProblemCurationCapability::PersonalMutation,
            )
            .await,
        Err(StoreError::Forbidden)
    );

    let create_result = fixture
        .store
        .replace_problem_collection(
            fixture.context,
            fixture.elena_session,
            ReplaceProblemCollectionCommand {
                target: ProblemCollectionReplacementTarget::NewNamed,
                expected_revision: None,
                title: Some("D2 shared enzyme set".into()),
                visibility: Some(ProblemCollectionVisibility::Institution),
                question_ids: fixture.public_questions.clone(),
            },
        )
        .await;
    let institution = match create_result {
        Ok(institution) => institution,
        Err(store_error) => {
            let mut transaction = fixture.pool.begin().await.expect("diagnostic transaction");
            sqlx::query("SET LOCAL ROLE ple_app")
                .execute(&mut *transaction)
                .await
                .expect("diagnostic application role");
            sqlx::query("SELECT set_config('ple.tenant_id', $1, true)")
                .bind(fixture.tenant.to_string())
                .execute(&mut *transaction)
                .await
                .expect("diagnostic tenant context");
            sqlx::query("SELECT set_config('ple.session_hash', $1, true)")
                .bind(fixture.elena_session.to_string())
                .execute(&mut *transaction)
                .await
                .expect("diagnostic session context");
            let question_ids: Vec<String> = fixture
                .public_questions
                .iter()
                .map(|question_id| question_id.compact())
                .collect();
            match sqlx::query(
                "SELECT * FROM public.ple_replace_problem_collection_v1($1, $2, NULL, 0, $3, $4, $5)",
            )
            .bind(fixture.tenant.as_uuid())
            .bind(fixture.elena_session.to_string())
            .bind("D2 shared enzyme set")
            .bind("institution")
            .bind(question_ids)
            .fetch_optional(&mut *transaction)
            .await
            {
                Err(database_error) => panic!(
                    "Elena collection creation failed in the Store ({store_error:?}) and broker ({database_error:?})"
                ),
                Ok(None) => panic!(
                    "Elena collection creation failed in the Store ({store_error:?}); broker concealed the request"
                ),
                Ok(Some(_)) => panic!(
                    "Elena collection creation failed in the Store ({store_error:?}); direct broker call succeeded"
                ),
            }
        }
    };
    assert_eq!(
        institution.access,
        ProblemCollectionAccess::Owner,
        "approved dual-role Elena mutates through instructor authority"
    );
    let morgan = fixture
        .store
        .get_problem_collection_summary(
            fixture.context,
            fixture.morgan_session,
            institution.reference,
        )
        .await
        .expect("Morgan institution read")
        .expect("institution collection visible");
    assert_eq!(morgan.access, ProblemCollectionAccess::InstitutionReader);
    assert!(matches!(
        fixture
            .store
            .replace_problem_collection(
                fixture.context,
                fixture.morgan_session,
                ReplaceProblemCollectionCommand {
                    target: ProblemCollectionReplacementTarget::Existing(institution.reference),
                    expected_revision: Some(institution.revision),
                    title: Some("Morgan edit".into()),
                    visibility: Some(ProblemCollectionVisibility::Institution),
                    question_ids: Vec::new(),
                },
            )
            .await,
        Err(StoreError::Forbidden)
    ));
    let private = fixture
        .store
        .replace_problem_collection(
            fixture.context,
            fixture.ada_session,
            ReplaceProblemCollectionCommand {
                target: ProblemCollectionReplacementTarget::NewNamed,
                expected_revision: None,
                title: Some("Ada private set".into()),
                visibility: Some(ProblemCollectionVisibility::Private),
                question_ids: fixture.public_questions[..1].to_vec(),
            },
        )
        .await
        .expect("Ada private collection");
    assert_eq!(
        fixture
            .store
            .get_problem_collection_summary(
                fixture.context,
                fixture.morgan_session,
                private.reference,
            )
            .await
            .expect("Morgan no existence oracle"),
        None
    );
    assert_eq!(
        fixture
            .store
            .get_problem_collection_summary(
                fixture.other_context,
                fixture.elena_session,
                institution.reference,
            )
            .await
            .expect("cross-tenant session fails closed"),
        None
    );
    let listed = fixture
        .store
        .list_problem_collections(
            fixture.context,
            fixture.morgan_session,
            PageRequest::first(PageSize::new(100).expect("page")),
        )
        .await
        .expect("Morgan collection list");
    assert_eq!(
        listed.items,
        vec![morgan],
        "Morgan receives only safe institution metadata"
    );
}
