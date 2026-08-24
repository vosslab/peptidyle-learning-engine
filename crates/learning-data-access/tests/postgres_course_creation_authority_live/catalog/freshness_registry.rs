//! Live authority oracle for the closed Base Course freshness registry.

use super::*;

const BROKER: &str = "ple_base_course_freshness_broker";
const WITNESS: &str = "ple_rehearsal_freshness_witness";
const REGISTRY_OWNER: &str = "ple_base_course_freshness_registry_owner";
const SEARCH_PATH: &str = "search_path=pg_catalog, public, pg_temp";
const FUNCTION: &str = "public.ple_verify_sealed_rehearsal_freshness_empty()";

pub(super) async fn catalog(pool: &PgPool) {
    registry_covers_the_public_relation_universe(pool).await;
    freshness_roles_are_closed(pool).await;
    registry_acl_and_rls_matrix_is_exact(pool).await;
    metadata_authority_is_read_only(pool).await;
    sealed_verifier_capability_is_exact(pool).await;
    freshness_function_locks_its_registry(pool).await;
    freshness_rejects_registry_drift_before_lifecycle_mutation(pool).await;
}

async fn metadata_authority_is_read_only(pool: &PgPool) {
    let domains: Vec<(String, String, String)> = sqlx::query_as(
        "SELECT domain,inspection_role::text,verifier_name FROM public.ple_base_course_freshness_domain ORDER BY domain",
    ).fetch_all(pool).await.expect("freshness domain registry");
    assert_eq!(
        domains,
        vec![
            (
                "raw".to_owned(),
                BROKER.to_owned(),
                "direct_raw_relation_empty".to_owned()
            ),
            (
                "sealed_rehearsal".to_owned(),
                WITNESS.to_owned(),
                FUNCTION.to_owned()
            ),
        ]
    );
    for relation in [
        "public.ple_base_course_freshness_domain",
        "public.ple_base_course_freshness_relation",
    ] {
        let owner: String = sqlx::query_scalar("SELECT r.rolname FROM pg_class c JOIN pg_roles r ON r.oid=c.relowner WHERE c.oid=$1::regclass")
            .bind(relation).fetch_one(pool).await.expect("freshness registry owner");
        assert_eq!(owner, REGISTRY_OWNER, "inert owner for {relation}");
    }
    for (role, relation, privilege, expected) in [
        (
            BROKER,
            "public.ple_base_course_freshness_domain",
            "SELECT",
            true,
        ),
        (
            BROKER,
            "public.ple_base_course_freshness_domain",
            "MAINTAIN",
            true,
        ),
        (
            WITNESS,
            "public.ple_base_course_freshness_domain",
            "SELECT",
            true,
        ),
        (
            WITNESS,
            "public.ple_base_course_freshness_domain",
            "MAINTAIN",
            true,
        ),
        (
            BROKER,
            "public.ple_base_course_freshness_relation",
            "SELECT",
            true,
        ),
        (
            BROKER,
            "public.ple_base_course_freshness_relation",
            "MAINTAIN",
            true,
        ),
        (
            WITNESS,
            "public.ple_base_course_freshness_relation",
            "SELECT",
            true,
        ),
        (
            WITNESS,
            "public.ple_base_course_freshness_relation",
            "MAINTAIN",
            true,
        ),
        (BROKER, "public.question_id_namespace", "SELECT", true),
        (BROKER, "public.question_id_namespace", "MAINTAIN", true),
        (WITNESS, "public.question_id_namespace", "SELECT", false),
    ] {
        let granted: bool = sqlx::query_scalar("SELECT has_table_privilege($1,$2,$3)")
            .bind(role)
            .bind(relation)
            .bind(privilege)
            .fetch_one(pool)
            .await
            .expect("freshness metadata privilege");
        assert_eq!(granted, expected, "{role} {privilege} on {relation}");
    }
    for relation in [
        "public.ple_base_course_freshness_domain",
        "public.ple_base_course_freshness_relation",
    ] {
        for role in [BROKER, WITNESS] {
            for privilege in [
                "INSERT",
                "UPDATE",
                "DELETE",
                "TRUNCATE",
                "REFERENCES",
                "TRIGGER",
            ] {
                let granted: bool = sqlx::query_scalar("SELECT has_table_privilege($1,$2,$3)")
                    .bind(role)
                    .bind(relation)
                    .bind(privilege)
                    .fetch_one(pool)
                    .await
                    .expect("registry mutation privilege");
                assert!(!granted, "{role} cannot {privilege} {relation}");
            }
        }
    }
}

async fn registry_covers_the_public_relation_universe(pool: &PgPool) {
    let expected: Vec<i64> = sqlx::query_scalar(
        "SELECT c.oid::int8 FROM pg_class c \
         JOIN pg_namespace n ON n.oid=c.relnamespace WHERE n.nspname='public' \
         AND c.relkind IN ('r','p') AND c.relname NOT IN \
         ('_sqlx_migrations','question_id_namespace','ple_base_course_freshness_relation','ple_base_course_freshness_domain') \
         ORDER BY c.oid",
    )
    .fetch_all(pool)
    .await
    .expect("public freshness universe");
    let actual: Vec<i64> = sqlx::query_scalar(
        "SELECT relation_oid::int8 FROM public.ple_base_course_freshness_relation \
         ORDER BY relation_oid",
    )
    .fetch_all(pool)
    .await
    .expect("freshness registry");
    assert_eq!(
        actual, expected,
        "the explicit registry covers the exact public r/p universe"
    );
}

async fn freshness_roles_are_closed(pool: &PgPool) {
    for role in [BROKER, WITNESS, REGISTRY_OWNER] {
        let flags: (bool, bool, bool, bool, bool, bool, bool) = sqlx::query_as(
        "SELECT rolcanlogin,rolsuper,rolcreatedb,rolcreaterole,rolinherit,rolreplication,rolbypassrls \
         FROM pg_roles WHERE rolname=$1",
    )
    .bind(role)
    .fetch_one(pool)
    .await
    .expect("closed freshness role");
        assert_eq!(
            flags,
            (false, false, false, false, false, false, false),
            "closed flags for {role}"
        );
        let memberships: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM pg_auth_members WHERE roleid=$1::regrole OR member=$1::regrole",
        )
        .bind(role)
        .fetch_one(pool)
        .await
        .expect("freshness role memberships");
        assert_eq!(memberships, 0, "{role} has no membership path");
        let schema_privileges: (bool, bool) = sqlx::query_as(
            "SELECT has_schema_privilege($1,'public','USAGE'),has_schema_privilege($1,'public','CREATE')",
        )
        .bind(role)
        .fetch_one(pool)
        .await
        .expect("freshness public-schema authority");
        assert_eq!(
            schema_privileges,
            (true, false),
            "exact public schema authority for {role}"
        );
    }
    let column_acls: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM pg_attribute a CROSS JOIN LATERAL aclexplode(a.attacl) acl \
         WHERE acl.grantee IN($1::regrole,$2::regrole)",
    )
    .bind(BROKER)
    .bind(WITNESS)
    .fetch_one(pool)
    .await
    .expect("freshness direct column ACLs");
    assert_eq!(column_acls, 0, "freshness roles have no direct column ACLs");
    let sequence_privileges: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM pg_class c JOIN pg_namespace n ON n.oid=c.relnamespace \
         WHERE n.nspname='public' AND c.relkind='S' AND (has_sequence_privilege($1,c.oid,'USAGE') \
         OR has_sequence_privilege($1,c.oid,'SELECT') OR has_sequence_privilege($1,c.oid,'UPDATE') \
         OR has_sequence_privilege($2,c.oid,'USAGE') OR has_sequence_privilege($2,c.oid,'SELECT') \
         OR has_sequence_privilege($2,c.oid,'UPDATE'))",
    )
    .bind(BROKER)
    .bind(WITNESS)
    .fetch_one(pool)
    .await
    .expect("freshness public-sequence authority");
    assert_eq!(
        sequence_privileges, 0,
        "freshness roles have no public-sequence privilege"
    );
}

async fn registry_acl_and_rls_matrix_is_exact(pool: &PgPool) {
    let expected_acl: Vec<(i64, String, String)> = sqlx::query_as(
        "SELECT relation_oid::int8,CASE WHEN domain='raw' THEN $1 ELSE $2 END,privilege \
         FROM public.ple_base_course_freshness_relation \
         CROSS JOIN (VALUES('MAINTAIN'),('SELECT')) wanted(privilege) ORDER BY 1,2,3",
    )
    .bind(BROKER)
    .bind(WITNESS)
    .fetch_all(pool)
    .await
    .expect("expected freshness ACL matrix");
    let actual_acl: Vec<(i64, String, String)> = sqlx::query_as(
        "SELECT c.oid::int8,r.rolname,acl.privilege_type FROM pg_class c \
         CROSS JOIN LATERAL aclexplode(coalesce(c.relacl,acldefault('r',c.relowner))) acl \
         JOIN pg_roles r ON r.oid=acl.grantee WHERE c.oid IN(SELECT relation_oid FROM public.ple_base_course_freshness_relation) \
         AND r.rolname IN ($1,$2) \
         AND acl.grantee<>c.relowner ORDER BY 1,2,3",
    )
    .bind(BROKER)
    .bind(WITNESS)
    .fetch_all(pool)
    .await
    .expect("actual freshness ACL matrix");
    assert_eq!(
        actual_acl, expected_acl,
        "each registry domain owns only SELECT and MAINTAIN"
    );
    let unsafe_relation_authority: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM pg_class c \
         WHERE c.oid IN(SELECT relation_oid FROM public.ple_base_course_freshness_relation) \
         AND (c.relowner IN($1::regrole,$2::regrole) \
              OR EXISTS(SELECT 1 FROM aclexplode(coalesce(c.relacl,acldefault('r',c.relowner))) acl \
                        WHERE acl.grantee=0) \
              OR (c.relrowsecurity AND NOT c.relforcerowsecurity))",
    )
    .bind(BROKER)
    .bind(WITNESS)
    .fetch_one(pool)
    .await
    .expect("unsafe freshness relation authority");
    assert_eq!(
        unsafe_relation_authority, 0,
        "registered relations have inert owners, no PUBLIC ACL, and forced RLS"
    );

    let expected_policies: Vec<(i64, String, String)> = sqlx::query_as(
        "SELECT registry.relation_oid::int8,CASE WHEN registry.domain='raw' \
         THEN 'ple_base_course_freshness_select' ELSE 'ple_rehearsal_freshness_witness_select' END, \
         CASE WHEN registry.domain='raw' THEN $1 ELSE $2 END \
         FROM public.ple_base_course_freshness_relation registry JOIN pg_class c ON c.oid=registry.relation_oid \
         WHERE c.relrowsecurity ORDER BY 1,2,3",
    )
    .bind(BROKER)
    .bind(WITNESS)
    .fetch_all(pool)
    .await
    .expect("expected freshness RLS matrix");
    let actual_policies: Vec<(i64, String, String)> = sqlx::query_as(
        "SELECT p.polrelid::int8,p.polname,r.rolname FROM pg_policy p \
         JOIN pg_roles r ON r.oid=ANY(p.polroles) WHERE p.polrelid IN(SELECT relation_oid FROM public.ple_base_course_freshness_relation) \
         AND r.rolname IN ($1,$2) \
         AND p.polcmd='r' AND p.polpermissive AND pg_get_expr(p.polqual,p.polrelid)='true' \
         AND p.polwithcheck IS NULL ORDER BY 1,2,3",
    )
    .bind(BROKER)
    .bind(WITNESS)
    .fetch_all(pool)
    .await
    .expect("actual freshness RLS matrix");
    assert_eq!(
        actual_policies, expected_policies,
        "only true SELECT policies cross the registry boundary"
    );
    let public_policies: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM pg_policy p \
         WHERE p.polrelid IN(SELECT relation_oid FROM public.ple_base_course_freshness_relation) \
         AND 0=ANY(p.polroles)",
    )
    .fetch_one(pool)
    .await
    .expect("PUBLIC freshness policies");
    assert_eq!(public_policies, 0, "PUBLIC has no registered-table policy");

    let sealed_selects: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM public.ple_base_course_freshness_relation \
         WHERE domain='sealed_rehearsal' AND has_table_privilege($1,relation_oid,'SELECT')",
    )
    .bind(BROKER)
    .fetch_one(pool)
    .await
    .expect("broker sealed SELECT denial");
    assert_eq!(
        sealed_selects, 0,
        "Base Course broker cannot read sealed rehearsal rows"
    );
    let raw_selects: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM public.ple_base_course_freshness_relation \
         WHERE domain='raw' AND has_table_privilege($1,relation_oid,'SELECT')",
    )
    .bind(WITNESS)
    .fetch_one(pool)
    .await
    .expect("witness raw SELECT denial");
    assert_eq!(
        raw_selects, 0,
        "sealed rehearsal witness cannot read raw application rows"
    );
}

async fn sealed_verifier_capability_is_exact(pool: &PgPool) {
    let row = sqlx::query(
        "SELECT r.rolname,p.prosecdef,p.proconfig FROM pg_proc p JOIN pg_roles r ON r.oid=p.proowner \
         WHERE p.oid=to_regprocedure($1)",
    )
    .bind(FUNCTION)
    .fetch_one(pool)
    .await
    .expect("sealed verifier function");
    assert_eq!(row.try_get::<String, _>(0).expect("owner"), WITNESS);
    assert!(row.try_get::<bool, _>(1).expect("security definer"));
    assert_eq!(
        row.try_get::<Vec<String>, _>(2).expect("search path"),
        vec![SEARCH_PATH.to_owned()]
    );
    for (role, expected) in [
        (BROKER, true),
        (WITNESS, true),
        ("ple_base_course_install_broker", false),
        ("ple_app", false),
    ] {
        let granted: bool = sqlx::query_scalar("SELECT has_function_privilege($1,$2,'EXECUTE')")
            .bind(role)
            .bind(FUNCTION)
            .fetch_one(pool)
            .await
            .expect("verifier execute matrix");
        assert_eq!(granted, expected, "{role} verifier execute authority");
    }
    let public_execute: bool =
        sqlx::query_scalar("SELECT has_function_privilege('public',$1,'EXECUTE')")
            .bind(FUNCTION)
            .fetch_one(pool)
            .await
            .expect("PUBLIC verifier execute");
    assert!(!public_execute, "PUBLIC cannot execute the sealed verifier");
}

async fn freshness_function_locks_its_registry(pool: &PgPool) {
    let definition: String = sqlx::query_scalar(
        "SELECT pg_get_functiondef(to_regprocedure( \
         'public.ple_require_fresh_base_course_install_internal()'))",
    )
    .fetch_one(pool)
    .await
    .expect("Base Course freshness function definition");
    assert!(
        definition
            .contains("LOCK TABLE ONLY public.ple_base_course_freshness_relation IN SHARE MODE"),
        "freshness locks the registry before using its coverage and authority decisions"
    );
    assert!(
        definition
            .contains("LOCK TABLE ONLY public.ple_base_course_freshness_domain IN SHARE MODE"),
        "freshness locks the domain registry before using its verifier decisions"
    );
}

async fn freshness_rejects_registry_drift_before_lifecycle_mutation(pool: &PgPool) {
    let mut tx = pool.begin().await.expect("freshness drift transaction");
    sqlx::query(
        "DELETE FROM public.ple_base_course_freshness_relation WHERE relation_oid=( \
         SELECT relation_oid FROM public.ple_base_course_freshness_relation WHERE domain='raw' LIMIT 1)",
    )
    .execute(&mut *tx)
    .await
    .expect("transactional registry drift");
    sqlx::query("SET LOCAL ROLE ple_base_course_freshness_broker")
        .execute(&mut *tx)
        .await
        .expect("freshness broker role");
    let result =
        sqlx::query("SELECT * FROM public.ple_require_fresh_base_course_install_internal()")
            .fetch_all(&mut *tx)
            .await;
    assert!(
        result.is_err(),
        "registry drift fails closed before a Base Course lifecycle marker"
    );
    tx.rollback().await.expect("discard drift fixture");
}
