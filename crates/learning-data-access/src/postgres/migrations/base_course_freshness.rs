//! Catalog-derived Base Course freshness registration and verification.
//!
//! The migration runner owns the epoch and lock lifecycle. This private module
//! keeps the large, catalog-sensitive SQL projection beside its executor helper
//! so the runner remains focused on migration state and transaction policy.

use sqlx::{Executor, Postgres};

pub(super) const RECONCILIATION_SQL: &str = include_str!(
    "../../../../../schemas/migrations/2026081835_base_course_freshness_registration.sql"
);

pub(super) const VERIFICATION_SQL: &str = r#"
WITH public_relations AS (
    SELECT relation_row.oid, format('%I.%I', namespace.nspname, relation_row.relname) AS relation_name
      FROM pg_catalog.pg_class AS relation_row
      JOIN pg_catalog.pg_namespace AS namespace ON namespace.oid = relation_row.relnamespace
     WHERE namespace.nspname = 'public'
       AND relation_row.relkind IN ('r', 'p')
), expected_relations AS (
    SELECT oid, relation_name
      FROM public_relations
     WHERE relation_name <> 'public._sqlx_migrations'
), expected_relation_privileges AS (
    SELECT relation_name, privilege_type
      FROM expected_relations
      CROSS JOIN (VALUES ('SELECT'), ('MAINTAIN')) AS privilege(privilege_type)
), actual_relation_privileges AS (
    SELECT format('%I.%I', namespace.nspname, relation_row.relname) AS relation_name,
           privilege.privilege_type
      FROM pg_catalog.pg_class AS relation_row
      JOIN pg_catalog.pg_namespace AS namespace ON namespace.oid = relation_row.relnamespace
      CROSS JOIN LATERAL aclexplode(
          COALESCE(relation_row.relacl, acldefault('r', relation_row.relowner))
      ) AS privilege
     WHERE namespace.nspname = 'public'
       AND relation_row.relkind IN ('r', 'p')
       AND privilege.grantee = 'ple_base_course_freshness_broker'::regrole
       AND privilege.grantee <> relation_row.relowner
), expected_policies AS (
    SELECT format('%I.%I', namespace.nspname, relation_row.relname) AS relation_name,
           'ple_base_course_freshness_select'::name AS policy_name,
           'r'::"char" AS command,
           true AS permissive,
           'true'::text AS using_expression,
           NULL::text AS check_expression,
           ARRAY['ple_base_course_freshness_broker']::text[] AS role_names
      FROM pg_catalog.pg_class AS relation_row
      JOIN pg_catalog.pg_namespace AS namespace ON namespace.oid = relation_row.relnamespace
     WHERE namespace.nspname = 'public'
       AND relation_row.relkind IN ('r', 'p')
       AND relation_row.relrowsecurity
       AND relation_row.relname <> '_sqlx_migrations'
), actual_policies AS (
    SELECT format('%I.%I', namespace.nspname, relation_row.relname) AS relation_name,
           policy_row.polname AS policy_name,
           policy_row.polcmd AS command,
           policy_row.polpermissive AS permissive,
           pg_catalog.pg_get_expr(policy_row.polqual, policy_row.polrelid) AS using_expression,
           pg_catalog.pg_get_expr(policy_row.polwithcheck, policy_row.polrelid) AS check_expression,
           ARRAY(
               SELECT COALESCE(role_row.rolname::text, 'PUBLIC')
                 FROM unnest(policy_row.polroles) AS policy_role(role_oid)
               LEFT JOIN pg_catalog.pg_roles AS role_row ON role_row.oid = policy_role.role_oid
                ORDER BY COALESCE(role_row.rolname::text, 'PUBLIC')
           ) AS role_names
      FROM pg_catalog.pg_policy AS policy_row
      JOIN pg_catalog.pg_class AS relation_row ON relation_row.oid = policy_row.polrelid
      JOIN pg_catalog.pg_namespace AS namespace ON namespace.oid = relation_row.relnamespace
     WHERE namespace.nspname = 'public'
       AND relation_row.relkind IN ('r', 'p')
       AND (
           policy_row.polname = 'ple_base_course_freshness_select'
           OR 'ple_base_course_freshness_broker'::regrole::oid = ANY(policy_row.polroles)
       )
)
SELECT NOT EXISTS (
    SELECT 1
      FROM (
          SELECT 1 AS drift
            FROM (SELECT * FROM expected_relation_privileges EXCEPT SELECT * FROM actual_relation_privileges)
          UNION ALL
          SELECT 1 AS drift
            FROM (SELECT * FROM actual_relation_privileges EXCEPT SELECT * FROM expected_relation_privileges)
          UNION ALL
          SELECT 1 AS drift
            FROM (SELECT * FROM expected_policies EXCEPT SELECT * FROM actual_policies)
          UNION ALL
          SELECT 1 AS drift
            FROM (SELECT * FROM actual_policies EXCEPT SELECT * FROM expected_policies)
          UNION ALL
          SELECT 1 AS drift
            FROM pg_catalog.pg_roles AS role_row
           WHERE role_row.rolname = 'ple_base_course_freshness_broker'
             AND (
                 role_row.rolcanlogin OR role_row.rolsuper OR role_row.rolcreatedb
                 OR role_row.rolcreaterole OR role_row.rolinherit OR role_row.rolreplication
                 OR role_row.rolbypassrls
             )
          UNION ALL
          SELECT 1 AS drift
            FROM pg_catalog.pg_auth_members AS membership
           WHERE membership.member = 'ple_base_course_freshness_broker'::regrole
              OR membership.roleid = 'ple_base_course_freshness_broker'::regrole
          UNION ALL
          SELECT 1 AS drift
            FROM pg_catalog.pg_attribute AS attribute
            CROSS JOIN LATERAL aclexplode(attribute.attacl) AS privilege
           WHERE privilege.grantee = 'ple_base_course_freshness_broker'::regrole
          UNION ALL
          SELECT 1 AS drift
            FROM public_relations AS relation_row
           WHERE relation_row.oid IN (
               SELECT owned_relation.oid
                 FROM pg_catalog.pg_class AS owned_relation
                WHERE owned_relation.relowner = 'ple_base_course_freshness_broker'::regrole
           )
          UNION ALL
          SELECT 1 AS drift
            FROM public_relations AS relation_row
           WHERE has_table_privilege(
                     'ple_base_course_freshness_broker', relation_row.oid, 'INSERT'
                 )
              OR has_table_privilege(
                     'ple_base_course_freshness_broker', relation_row.oid, 'UPDATE'
                 )
              OR has_table_privilege(
                     'ple_base_course_freshness_broker', relation_row.oid, 'DELETE'
                 )
              OR has_table_privilege(
                     'ple_base_course_freshness_broker', relation_row.oid, 'TRUNCATE'
                 )
              OR has_table_privilege(
                     'ple_base_course_freshness_broker', relation_row.oid, 'REFERENCES'
                 )
              OR has_table_privilege(
                     'ple_base_course_freshness_broker', relation_row.oid, 'TRIGGER'
                 )
          UNION ALL
          SELECT 1 AS drift
            FROM public_relations AS relation_row
            JOIN pg_catalog.pg_attribute AS attribute
              ON attribute.attrelid = relation_row.oid
           WHERE attribute.attnum > 0
             AND NOT attribute.attisdropped
             AND (
                 has_column_privilege(
                     'ple_base_course_freshness_broker', relation_row.oid,
                     attribute.attnum, 'INSERT'
                 )
                 OR has_column_privilege(
                     'ple_base_course_freshness_broker', relation_row.oid,
                     attribute.attnum, 'UPDATE'
                 )
                 OR has_column_privilege(
                     'ple_base_course_freshness_broker', relation_row.oid,
                     attribute.attnum, 'REFERENCES'
                 )
             )
          UNION ALL
          SELECT 1 AS drift
            FROM pg_catalog.pg_class AS sequence_row
            JOIN pg_catalog.pg_namespace AS namespace ON namespace.oid = sequence_row.relnamespace
           WHERE namespace.nspname = 'public'
             AND sequence_row.relkind = 'S'
             AND (
                 has_sequence_privilege('ple_base_course_freshness_broker', sequence_row.oid, 'USAGE')
                 OR has_sequence_privilege('ple_base_course_freshness_broker', sequence_row.oid, 'SELECT')
                 OR has_sequence_privilege('ple_base_course_freshness_broker', sequence_row.oid, 'UPDATE')
             )
      ) AS capability_drift
) AS compatible
"#;

pub(super) async fn is_compatible<'executor, E>(executor: E) -> Result<bool, sqlx::Error>
where
    E: Executor<'executor, Database = Postgres>,
{
    sqlx::query_scalar(VERIFICATION_SQL)
        .fetch_one(executor)
        .await
}
