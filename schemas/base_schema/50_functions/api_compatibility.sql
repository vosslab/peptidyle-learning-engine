-- Functions, triggers, and views from api_compatibility.sql.

SET LOCAL ROLE ple_api_owner;

-- Application-safe schema compatibility state. The projection has no mutable
-- API surface and is intentionally the only runtime route to forward state.
CREATE VIEW ple_api.ple_schema_state
WITH (security_barrier = true, security_invoker = false) AS
SELECT 'pre-production'::text AS base_release,
       migration.version,
       migration.success,
       migration.checksum
  FROM ple_migration._sqlx_migrations AS migration
UNION ALL
SELECT 'pre-production'::text AS base_release,
       NULL::bigint AS version,
       NULL::boolean AS success,
       NULL::bytea AS checksum
 WHERE NOT EXISTS (
     SELECT 1
       FROM ple_migration._sqlx_migrations
 );

COMMENT ON VIEW ple_api.ple_schema_state IS
    'Application-readable base release and read-only SQLx forward state.';

