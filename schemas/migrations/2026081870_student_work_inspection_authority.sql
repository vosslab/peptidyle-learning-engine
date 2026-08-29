-- WP-INST-G2 / G2-W3B: dedicated authority for one audited Student-work read.
--
-- The application role receives only the callable broker installed in 1872.
-- This owner has no login, inherited authority, or role memberships.

BEGIN;

DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_catalog.pg_roles
                   WHERE rolname = 'ple_student_work_inspection_broker') THEN
        CREATE ROLE ple_student_work_inspection_broker
            NOLOGIN NOINHERIT NOSUPERUSER NOCREATEDB NOCREATEROLE
            NOREPLICATION NOBYPASSRLS;
    END IF;
    IF EXISTS (
        SELECT 1 FROM pg_catalog.pg_auth_members AS membership
        WHERE membership.roleid = 'ple_student_work_inspection_broker'::regrole
           OR membership.member = 'ple_student_work_inspection_broker'::regrole
    ) THEN
        RAISE EXCEPTION 'student-work inspection broker must not have role memberships';
    END IF;
END;
$$;

ALTER ROLE ple_student_work_inspection_broker
    NOLOGIN NOINHERIT NOSUPERUSER NOCREATEDB NOCREATEROLE
    NOREPLICATION NOBYPASSRLS;
ALTER ROLE ple_student_work_inspection_broker
    SET search_path TO pg_catalog, public, pg_temp;
REVOKE ALL ON SCHEMA public FROM ple_student_work_inspection_broker;
GRANT USAGE ON SCHEMA public TO ple_student_work_inspection_broker;
REVOKE ALL ON ALL TABLES IN SCHEMA public FROM ple_student_work_inspection_broker;
REVOKE ALL ON ALL SEQUENCES IN SCHEMA public FROM ple_student_work_inspection_broker;
REVOKE ALL ON ALL FUNCTIONS IN SCHEMA public FROM ple_student_work_inspection_broker;

DO $$
DECLARE
    v_broker oid := 'ple_student_work_inspection_broker'::regrole::oid;
BEGIN
    IF EXISTS (
        SELECT 1 FROM pg_catalog.pg_roles
        WHERE rolname = 'ple_student_work_inspection_broker'
          AND (rolcanlogin OR rolinherit OR rolbypassrls OR rolsuper
               OR rolcreatedb OR rolcreaterole OR rolreplication)
    ) THEN
        RAISE EXCEPTION 'student-work inspection broker role attributes are unsafe';
    END IF;
    IF NOT EXISTS (
        SELECT 1 FROM pg_catalog.pg_roles
        WHERE oid = v_broker
          AND rolconfig IS NOT DISTINCT FROM ARRAY[
              'search_path=pg_catalog, public, pg_temp'
          ]::text[]
    ) THEN
        RAISE EXCEPTION 'student-work inspection broker role configuration is unsafe';
    END IF;
    IF (
        SELECT count(*)
        FROM pg_catalog.pg_namespace AS namespace
        CROSS JOIN LATERAL pg_catalog.aclexplode(namespace.nspacl) AS privilege
        WHERE namespace.oid = 'public'::regnamespace
          AND privilege.grantee = v_broker
          AND privilege.privilege_type = 'USAGE'
          AND NOT privilege.is_grantable
    ) <> 1 OR has_schema_privilege(
        'ple_student_work_inspection_broker', 'public', 'CREATE'
    ) THEN
        RAISE EXCEPTION 'student-work inspection broker schema authority is unsafe';
    END IF;
    IF EXISTS (
        SELECT 1 FROM pg_catalog.pg_class AS relation
        CROSS JOIN LATERAL pg_catalog.aclexplode(relation.relacl) AS privilege
        WHERE relation.relnamespace = 'public'::regnamespace
          AND privilege.grantee = v_broker
    ) OR EXISTS (
        SELECT 1 FROM pg_catalog.pg_proc AS procedure_row
        CROSS JOIN LATERAL pg_catalog.aclexplode(procedure_row.proacl) AS privilege
        WHERE procedure_row.pronamespace = 'public'::regnamespace
          AND privilege.grantee = v_broker
    ) THEN
        RAISE EXCEPTION 'student-work inspection broker baseline ACL is unsafe';
    END IF;
END;
$$;

COMMIT;
