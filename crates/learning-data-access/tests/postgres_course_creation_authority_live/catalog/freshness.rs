//! Catalog oracle for the catalog-derived Base Course freshness capability.

use super::*;
use learning_data_access::postgres::SchemaCompatibilityError;

const ROLE: &str = "ple_base_course_freshness_broker";

pub(super) async fn catalog(pool: &PgPool) {
    reconciliation_and_drift_detection(pool).await;
    relation_privileges(pool).await;
    policies(pool).await;
    sealed_closure(pool).await;
}

async fn reconciliation_and_drift_detection(pool: &PgPool) {
    sqlx::raw_sql(
        "DROP TABLE IF EXISTS public.ple_base_course_freshness_oracle; \
         CREATE TABLE public.ple_base_course_freshness_oracle (id bigint PRIMARY KEY, payload text); \
         ALTER TABLE public.ple_base_course_freshness_oracle ENABLE ROW LEVEL SECURITY;",
    )
    .execute(pool)
    .await
    .expect("freshness drift oracle relation");

    let registration = apply_migrations(pool).await;
    if let Err(error) = registration {
        drop_oracle_relation(pool).await;
        panic!("freshness registration covers a later public relation: {error}");
    }

    sqlx::raw_sql(
        "DROP POLICY ple_base_course_freshness_select \
             ON public.ple_base_course_freshness_oracle; \
         CREATE POLICY ple_base_course_freshness_select \
             ON public.ple_base_course_freshness_oracle FOR SELECT TO PUBLIC USING (false); \
         CREATE POLICY ple_base_course_freshness_extra \
             ON public.ple_base_course_freshness_oracle FOR SELECT \
             TO ple_base_course_freshness_broker USING (false);",
    )
    .execute(pool)
    .await
    .expect("reserved-name and broker-policy collisions");
    let collision_reconciliation = apply_migrations(pool).await;
    let collision_verification = verify_base_course_freshness_capability(pool).await;

    sqlx::query("GRANT INSERT ON TABLE public.ple_base_course_freshness_oracle TO PUBLIC")
        .execute(pool)
        .await
        .expect("effective PUBLIC table-write drift");
    let public_table_write = verify_base_course_freshness_capability(pool).await;
    sqlx::query("REVOKE INSERT ON TABLE public.ple_base_course_freshness_oracle FROM PUBLIC")
        .execute(pool)
        .await
        .expect("repair effective PUBLIC table-write drift");

    sqlx::query(
        "GRANT UPDATE (payload) ON TABLE public.ple_base_course_freshness_oracle TO PUBLIC",
    )
    .execute(pool)
    .await
    .expect("effective PUBLIC column-write drift");
    let public_column_write = verify_base_course_freshness_capability(pool).await;
    sqlx::query(
        "REVOKE UPDATE (payload) ON TABLE public.ple_base_course_freshness_oracle FROM PUBLIC",
    )
    .execute(pool)
    .await
    .expect("repair effective PUBLIC column-write drift");

    sqlx::query(
        "ALTER TABLE public.ple_base_course_freshness_oracle \
         OWNER TO ple_base_course_freshness_broker",
    )
    .execute(pool)
    .await
    .expect("freshness-broker ownership drift");
    let broker_ownership = verify_base_course_freshness_capability(pool).await;
    sqlx::query("ALTER TABLE public.ple_base_course_freshness_oracle OWNER TO CURRENT_USER")
        .execute(pool)
        .await
        .expect("repair freshness-broker ownership drift");
    let ownership_reconciliation = apply_migrations(pool).await;
    let repaired_verification = verify_base_course_freshness_capability(pool).await;
    drop_oracle_relation(pool).await;

    collision_reconciliation.expect("reserved and broker policy collisions reconcile");
    collision_verification.expect("collision reconciliation restores the exact capability");
    assert!(
        matches!(
            public_table_write,
            Err(SchemaCompatibilityError::Incompatible(_))
        ),
        "effective PUBLIC table writes are incompatible"
    );
    assert!(
        matches!(
            public_column_write,
            Err(SchemaCompatibilityError::Incompatible(_))
        ),
        "effective PUBLIC column writes are incompatible"
    );
    assert!(
        matches!(
            broker_ownership,
            Err(SchemaCompatibilityError::Incompatible(_))
        ),
        "freshness broker relation ownership is incompatible"
    );
    ownership_reconciliation.expect("ownership repair restores exact direct privileges");
    repaired_verification.expect("all injected freshness drift is repaired");
}

async fn drop_oracle_relation(pool: &PgPool) {
    sqlx::query("DROP TABLE IF EXISTS public.ple_base_course_freshness_oracle")
        .execute(pool)
        .await
        .expect("freshness drift oracle cleanup");
}

async fn relation_privileges(pool: &PgPool) {
    let expected: Vec<(String, String)> = sqlx::query_as(
        "SELECT format('%I.%I',n.nspname,c.relname),privilege.privilege_type \
         FROM pg_class c JOIN pg_namespace n ON n.oid=c.relnamespace \
         CROSS JOIN (VALUES ('SELECT'),('MAINTAIN')) privilege(privilege_type) \
         WHERE n.nspname='public' AND c.relkind IN ('r','p') \
           AND c.relname<>'_sqlx_migrations' \
         ORDER BY 1,2",
    )
    .fetch_all(pool)
    .await
    .expect("expected Base Course freshness relation privileges");
    let actual: Vec<(String, String)> = sqlx::query_as(
        "SELECT format('%I.%I',n.nspname,c.relname),acl.privilege_type \
         FROM pg_class c JOIN pg_namespace n ON n.oid=c.relnamespace \
         CROSS JOIN LATERAL aclexplode(coalesce(c.relacl,acldefault('r',c.relowner))) acl \
         WHERE n.nspname='public' AND c.relkind IN ('r','p') \
           AND acl.grantee=$1::regrole AND acl.grantee<>c.relowner \
         ORDER BY 1,2",
    )
    .bind(ROLE)
    .fetch_all(pool)
    .await
    .expect("actual Base Course freshness relation privileges");
    assert_eq!(
        actual, expected,
        "exact catalog-derived freshness relation graph"
    );
}

async fn policies(pool: &PgPool) {
    let expected: Vec<PolicyCatalogRow> = sqlx::query_as(
        "SELECT 'ple_base_course_freshness_select',c.relname,'r',true,ARRAY[$1::text], \
         'true'::text,NULL::text FROM pg_class c JOIN pg_namespace n ON n.oid=c.relnamespace \
         WHERE n.nspname='public' AND c.relkind IN ('r','p') AND c.relrowsecurity \
           AND c.relname<>'_sqlx_migrations' ORDER BY c.relname",
    )
    .bind(ROLE)
    .fetch_all(pool)
    .await
    .expect("expected Base Course freshness policies");
    let actual: Vec<PolicyCatalogRow> = sqlx::query_as(
        "SELECT p.polname,c.relname,p.polcmd::text,p.polpermissive, \
         array(SELECT coalesce(r.rolname::text,'PUBLIC') FROM unnest(p.polroles) role_oid \
               LEFT JOIN pg_roles r ON r.oid=role_oid \
               ORDER BY coalesce(r.rolname::text,'PUBLIC')), \
         pg_get_expr(p.polqual,p.polrelid),pg_get_expr(p.polwithcheck,p.polrelid) \
         FROM pg_policy p JOIN pg_class c ON c.oid=p.polrelid \
         JOIN pg_namespace n ON n.oid=c.relnamespace \
         WHERE n.nspname='public' AND c.relkind IN ('r','p') \
           AND (p.polname='ple_base_course_freshness_select' \
                OR $1::regrole::oid=ANY(p.polroles)) ORDER BY c.relname,p.polname",
    )
    .bind(ROLE)
    .fetch_all(pool)
    .await
    .expect("actual Base Course freshness policies");
    assert_eq!(
        actual, expected,
        "one exact freshness SELECT policy per RLS relation"
    );
}

async fn sealed_closure(pool: &PgPool) {
    let role_drift: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM pg_roles WHERE rolname=$1 AND (rolcanlogin OR rolsuper OR \
         rolcreatedb OR rolcreaterole OR rolinherit OR rolreplication OR rolbypassrls)",
    )
    .bind(ROLE)
    .fetch_one(pool)
    .await
    .expect("freshness role attributes");
    assert_eq!(
        role_drift, 0,
        "freshness broker is sealed NOLOGIN/NOBYPASSRLS"
    );
    let membership_edges: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM pg_auth_members WHERE member=$1::regrole OR roleid=$1::regrole",
    )
    .bind(ROLE)
    .fetch_one(pool)
    .await
    .expect("freshness membership graph");
    assert_eq!(
        membership_edges, 0,
        "freshness broker has no membership edge"
    );
    let sequence_privileges: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM pg_class c JOIN pg_namespace n ON n.oid=c.relnamespace \
         WHERE n.nspname='public' AND c.relkind='S' AND (has_sequence_privilege($1,c.oid,'USAGE') \
         OR has_sequence_privilege($1,c.oid,'SELECT') OR has_sequence_privilege($1,c.oid,'UPDATE'))",
    )
    .bind(ROLE)
    .fetch_one(pool)
    .await
    .expect("freshness sequence privileges");
    assert_eq!(
        sequence_privileges, 0,
        "freshness broker has no sequence authority"
    );
    let column_privileges: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM pg_attribute a CROSS JOIN LATERAL aclexplode(a.attacl) acl \
         WHERE acl.grantee=$1::regrole",
    )
    .bind(ROLE)
    .fetch_one(pool)
    .await
    .expect("freshness column privileges");
    assert_eq!(
        column_privileges, 0,
        "freshness broker has no column write authority"
    );
    let owned_relations: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM pg_class c JOIN pg_namespace n ON n.oid=c.relnamespace \
         WHERE n.nspname='public' AND c.relkind IN ('r','p') AND c.relowner=$1::regrole",
    )
    .bind(ROLE)
    .fetch_one(pool)
    .await
    .expect("freshness-owned public relations");
    assert_eq!(
        owned_relations, 0,
        "freshness broker owns no public relation"
    );
    let effective_table_writes: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM pg_class c JOIN pg_namespace n ON n.oid=c.relnamespace \
         WHERE n.nspname='public' AND c.relkind IN ('r','p') AND ( \
           has_table_privilege($1,c.oid,'INSERT') OR has_table_privilege($1,c.oid,'UPDATE') OR \
           has_table_privilege($1,c.oid,'DELETE') OR has_table_privilege($1,c.oid,'TRUNCATE') OR \
           has_table_privilege($1,c.oid,'REFERENCES') OR has_table_privilege($1,c.oid,'TRIGGER'))",
    )
    .bind(ROLE)
    .fetch_one(pool)
    .await
    .expect("freshness effective table writes");
    assert_eq!(
        effective_table_writes, 0,
        "freshness broker has no effective table write authority"
    );
    let effective_column_writes: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM pg_class c JOIN pg_namespace n ON n.oid=c.relnamespace \
         JOIN pg_attribute a ON a.attrelid=c.oid \
         WHERE n.nspname='public' AND c.relkind IN ('r','p') \
           AND a.attnum>0 AND NOT a.attisdropped AND ( \
             has_column_privilege($1,c.oid,a.attnum,'INSERT') OR \
             has_column_privilege($1,c.oid,a.attnum,'UPDATE') OR \
             has_column_privilege($1,c.oid,a.attnum,'REFERENCES'))",
    )
    .bind(ROLE)
    .fetch_one(pool)
    .await
    .expect("freshness effective column writes");
    assert_eq!(
        effective_column_writes, 0,
        "freshness broker has no effective column write authority"
    );
}
