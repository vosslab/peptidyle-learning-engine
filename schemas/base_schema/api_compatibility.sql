-- Application-safe schema compatibility state. The projection has no mutable
-- API surface and is intentionally the only runtime route to forward state.

SET LOCAL ROLE ple_api_owner;

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

REVOKE ALL PRIVILEGES ON TABLE ple_api.ple_schema_state FROM PUBLIC;
GRANT USAGE ON SCHEMA ple_api TO ple_app;
GRANT SELECT ON TABLE ple_api.ple_schema_state TO ple_app;
-- The coordinator verifies its own completed install through this deliberately
-- read-only projection.  It remains a separate capability from ple_app,
-- SQLx's ledger, and every API procedure (ASVS 8.2.1).
GRANT USAGE ON SCHEMA ple_api TO ple_migrator;
GRANT SELECT ON TABLE ple_api.ple_schema_state TO ple_migrator;

COMMENT ON VIEW ple_api.ple_schema_state IS
    'Application-readable base release and read-only SQLx forward state.';

RESET ROLE;
