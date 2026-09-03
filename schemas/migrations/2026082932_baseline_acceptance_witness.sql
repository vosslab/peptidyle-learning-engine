-- Schema acceptance witness for forced RLS and default-deny ACLs.

SET LOCAL ROLE ple_api_owner;
CREATE FUNCTION ple_api.assert_baseline_security_audit()
RETURNS void LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api
AS $$
BEGIN
    IF EXISTS (
        SELECT 1
        FROM pg_catalog.pg_class AS relation
        JOIN pg_catalog.pg_namespace AS namespace ON namespace.oid = relation.relnamespace
        WHERE namespace.nspname IN ('ple_data', 'ple_private', 'ple_audit')
          AND relation.relkind IN ('r', 'p')
          AND (NOT relation.relrowsecurity OR NOT relation.relforcerowsecurity)
    ) THEN
        RAISE EXCEPTION USING ERRCODE = '55000', MESSAGE = 'baseline relation lacks forced row security';
    END IF;
    IF EXISTS (
        SELECT 1
        FROM pg_catalog.pg_namespace AS namespace
        WHERE namespace.nspname IN ('ple_data', 'ple_private', 'ple_audit', 'ple_api')
          AND pg_catalog.has_schema_privilege('public', namespace.oid, 'USAGE')
    ) THEN
        RAISE EXCEPTION USING ERRCODE = '55000', MESSAGE = 'PUBLIC retains protected schema usage';
    END IF;
    IF EXISTS (
        SELECT 1
        FROM pg_catalog.pg_class AS relation
        JOIN pg_catalog.pg_namespace AS namespace ON namespace.oid = relation.relnamespace
        WHERE namespace.nspname IN ('ple_data', 'ple_private', 'ple_audit', 'ple_api')
          AND (
              (relation.relkind IN ('r', 'p') AND (
              pg_catalog.has_table_privilege('public', relation.oid, 'SELECT')
              OR pg_catalog.has_table_privilege('public', relation.oid, 'INSERT')
              OR pg_catalog.has_table_privilege('public', relation.oid, 'UPDATE')
              OR pg_catalog.has_table_privilege('public', relation.oid, 'DELETE')
              ))
              OR (relation.relkind = 'S' AND (
                  pg_catalog.has_sequence_privilege('public', relation.oid, 'SELECT')
                  OR pg_catalog.has_sequence_privilege('public', relation.oid, 'USAGE')
                  OR pg_catalog.has_sequence_privilege('public', relation.oid, 'UPDATE')
              ))
          )
    ) THEN
        RAISE EXCEPTION USING ERRCODE = '55000', MESSAGE = 'PUBLIC retains protected relation privilege';
    END IF;
END
$$;
REVOKE ALL PRIVILEGES ON FUNCTION ple_api.assert_baseline_security_audit() FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_api.assert_baseline_security_audit() TO ple_auth;
RESET ROLE;
