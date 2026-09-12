-- Database-wide PLE foundation: ordinary capability roles, schemas, default
-- deny ACLs, and SQLx's isolated forward-migration ledger.
--
-- Platform bootstrap creates every PLE role. PostgreSQL 17 gives a CREATEROLE
-- principal an unremovable ADMIN membership for roles it creates, so the
-- restricted migrator does not create ordinary capabilities. This base
-- validates the bootstrap boundary and configures those roles for its modules.

DO $$
BEGIN
    IF pg_catalog.current_setting('server_version_num')::integer / 10000 <> 17 THEN
        RAISE EXCEPTION USING
            ERRCODE = '55000',
            MESSAGE = 'the PLE base schema requires PostgreSQL major version 17';
    END IF;

    IF current_user <> 'ple_migrator' THEN
        RAISE EXCEPTION USING
            ERRCODE = '42501',
            MESSAGE = 'the PLE base schema must run as ple_migrator';
    END IF;

    -- ASVS 2.3.3: reject a non-dedicated target before this manifest creates
    -- PLE objects, so the enclosing single transaction leaves no partial base.
    IF EXISTS (
        SELECT 1
          FROM pg_catalog.pg_namespace AS namespace
         WHERE namespace.nspname <> ALL (ARRAY['public', 'information_schema'])
           AND namespace.nspname !~ '^pg_'
    ) OR EXISTS (
        SELECT 1
          FROM pg_catalog.pg_class AS relation
          JOIN pg_catalog.pg_namespace AS namespace
            ON namespace.oid = relation.relnamespace
         WHERE namespace.nspname = 'public'
           AND relation.relpersistence = 'p'
           AND relation.relkind IN ('r', 'p', 'v', 'm', 'f')
    ) THEN
        RAISE EXCEPTION USING
            ERRCODE = '55000',
            MESSAGE = 'the PLE base schema requires an otherwise empty dedicated database';
    END IF;

    IF (
        SELECT count(*)
          FROM pg_catalog.pg_auth_members AS membership
         WHERE membership.roleid = (
                   SELECT oid
                     FROM pg_catalog.pg_roles
                    WHERE rolname = 'ple_unrelease_executor'
               )
    ) <> 1 OR NOT EXISTS (
        SELECT 1
          FROM pg_catalog.pg_roles AS role
         WHERE role.rolname = 'ple_migrator'
           AND role.rolcanlogin
           AND NOT role.rolinherit
           AND NOT role.rolsuper
           AND NOT role.rolcreatedb
           AND NOT role.rolcreaterole
           AND NOT role.rolreplication
           AND NOT role.rolbypassrls
    ) THEN
        RAISE EXCEPTION USING
            ERRCODE = '55000',
            MESSAGE = 'the migration login does not satisfy the PLE bootstrap contract';
    END IF;

    IF NOT EXISTS (
        SELECT 1
          FROM pg_catalog.pg_roles AS role
         WHERE role.rolname = 'ple_database_owner'
           AND NOT role.rolcanlogin
           AND NOT role.rolsuper
           AND NOT role.rolcreatedb
           AND NOT role.rolcreaterole
           AND NOT role.rolbypassrls
    ) THEN
        RAISE EXCEPTION USING
            ERRCODE = '55000',
            MESSAGE = 'the platform database owner does not satisfy the PLE bootstrap contract';
    END IF;

    IF EXISTS (
        SELECT 1
          FROM pg_catalog.pg_roles AS role
         WHERE role.rolname IN (
             'ple_public_asset_publisher',
             'ple_native_ple_grading_worker',
             'ple_webwork_grading_worker',
             'ple_imathas_question_backend_grading_worker',
             'ple_unrelease_executor',
             'ple_data_owner',
             'ple_private_owner',
             'ple_audit_owner',
             'ple_api_owner',
             'ple_app',
             'ple_auth',
             'ple_student'
         )
           AND (
               role.rolcanlogin
               OR role.rolinherit
               OR role.rolsuper
               OR role.rolcreatedb
               OR role.rolcreaterole
               OR role.rolreplication
               OR role.rolbypassrls
               OR role.rolconnlimit <> -1
           )
    ) OR (
        SELECT count(*)
          FROM pg_catalog.pg_roles AS role
         WHERE role.rolname IN (
             'ple_public_asset_publisher',
             'ple_native_ple_grading_worker',
             'ple_webwork_grading_worker',
             'ple_imathas_question_backend_grading_worker',
             'ple_unrelease_executor',
             'ple_data_owner',
             'ple_private_owner',
             'ple_audit_owner',
             'ple_api_owner',
             'ple_app',
             'ple_auth',
             'ple_student'
         )
    ) <> 12 THEN
        RAISE EXCEPTION USING
            ERRCODE = '55000',
            MESSAGE = 'the PLE capability roles do not satisfy the bootstrap contract';
    END IF;

    IF (
        SELECT count(*)
          FROM pg_catalog.pg_auth_members AS membership
         WHERE membership.member = (
                   SELECT oid
                     FROM pg_catalog.pg_roles
                    WHERE rolname = 'ple_migrator'
               )
    ) <> 6 OR (
        SELECT count(*)
          FROM pg_catalog.pg_auth_members AS membership
          JOIN pg_catalog.pg_roles AS granted_role
            ON granted_role.oid = membership.roleid
          JOIN pg_catalog.pg_roles AS member_role
            ON member_role.oid = membership.member
         WHERE member_role.rolname = 'ple_migrator'
           AND granted_role.rolname IN (
               'ple_database_owner',
               'ple_unrelease_executor',
               'ple_data_owner',
               'ple_private_owner',
               'ple_audit_owner',
               'ple_api_owner'
           )
           AND NOT membership.admin_option
           AND NOT membership.inherit_option
           AND membership.set_option
    ) <> 6 THEN
        RAISE EXCEPTION USING
            ERRCODE = '42501',
            MESSAGE = 'the migrator memberships do not satisfy the PLE bootstrap contract';
    END IF;

    IF EXISTS (
        SELECT 1
          FROM pg_catalog.pg_namespace AS namespace
         WHERE namespace.nspname IN (
             'ple_data',
             'ple_private',
             'ple_audit',
             'ple_api',
             'ple_migration'
         )
    ) THEN
        RAISE EXCEPTION USING
            ERRCODE = '42P06',
            MESSAGE = 'the PLE base namespace already exists';
    END IF;
END
$$;

ALTER DEFAULT PRIVILEGES FOR ROLE ple_migrator
    REVOKE ALL PRIVILEGES ON TABLES FROM PUBLIC;
ALTER DEFAULT PRIVILEGES FOR ROLE ple_migrator
    REVOKE ALL PRIVILEGES ON SEQUENCES FROM PUBLIC;
ALTER DEFAULT PRIVILEGES FOR ROLE ple_migrator
    REVOKE ALL PRIVILEGES ON FUNCTIONS FROM PUBLIC;
ALTER DEFAULT PRIVILEGES FOR ROLE ple_migrator
    REVOKE ALL PRIVILEGES ON TYPES FROM PUBLIC;

-- System-catalog name resolution is server-owned, distinct from PLE object
-- access. ASVS 8.2.1 and 13.2.2: PLE data access remains explicitly granted.
GRANT USAGE ON SCHEMA pg_catalog TO ple_data_owner, ple_private_owner,
    ple_audit_owner, ple_api_owner, ple_app, ple_auth, ple_student;

SET LOCAL ROLE ple_database_owner;

DO $$
DECLARE
    database_name name := current_database();
BEGIN
    EXECUTE pg_catalog.format(
        'REVOKE CONNECT, CREATE, TEMPORARY ON DATABASE %I FROM PUBLIC',
        database_name
    );
    EXECUTE pg_catalog.format(
        'GRANT CONNECT ON DATABASE %I TO ple_migrator',
        database_name
    );
    EXECUTE pg_catalog.format(
        'REVOKE CREATE ON DATABASE %I FROM ple_migrator',
        database_name
    );
END
$$;

REVOKE ALL PRIVILEGES ON SCHEMA public FROM PUBLIC;
REVOKE ALL PRIVILEGES ON SCHEMA public FROM ple_migrator;

ALTER DEFAULT PRIVILEGES REVOKE ALL PRIVILEGES ON SCHEMAS FROM PUBLIC;
ALTER DEFAULT PRIVILEGES REVOKE ALL PRIVILEGES ON TABLES FROM PUBLIC;
ALTER DEFAULT PRIVILEGES REVOKE ALL PRIVILEGES ON SEQUENCES FROM PUBLIC;
ALTER DEFAULT PRIVILEGES REVOKE ALL PRIVILEGES ON FUNCTIONS FROM PUBLIC;
ALTER DEFAULT PRIVILEGES REVOKE ALL PRIVILEGES ON TYPES FROM PUBLIC;

DO $$
DECLARE
    database_name name := current_database();
BEGIN
    EXECUTE pg_catalog.format(
        'GRANT CREATE ON DATABASE %I TO ple_data_owner, ple_private_owner, '
        'ple_audit_owner, ple_api_owner',
        database_name
    );
END
$$;

-- `ple_migrator` deliberately has no database-level CREATE.  Keep the
-- platform owner active through this owned-schema boundary; the following
-- qualified schema grant is all SQLx needs for its own ledger table.
CREATE SCHEMA ple_migration AUTHORIZATION ple_database_owner;
REVOKE ALL PRIVILEGES ON SCHEMA ple_migration FROM PUBLIC;
GRANT USAGE, CREATE ON SCHEMA ple_migration TO ple_migrator;

RESET ROLE;

-- SQLx 0.9 performs CREATE TABLE IF NOT EXISTS on every forward run. Its
-- narrow CREATE grant is limited to this otherwise isolated schema (ASVS 13.2.2).
CREATE TABLE ple_migration._sqlx_migrations (
    version bigint PRIMARY KEY,
    description text NOT NULL,
    installed_on timestamp with time zone NOT NULL DEFAULT now(),
    success boolean NOT NULL,
    checksum bytea NOT NULL,
    execution_time bigint NOT NULL
);
ALTER TABLE ple_migration._sqlx_migrations OWNER TO ple_migrator;
REVOKE ALL PRIVILEGES ON TABLE ple_migration._sqlx_migrations FROM PUBLIC;
GRANT SELECT ON TABLE ple_migration._sqlx_migrations TO ple_api_owner;

SET LOCAL ROLE ple_database_owner;
GRANT USAGE ON SCHEMA ple_migration TO ple_api_owner;
RESET ROLE;

SET LOCAL ROLE ple_data_owner;
CREATE SCHEMA ple_data AUTHORIZATION ple_data_owner;
REVOKE ALL PRIVILEGES ON SCHEMA ple_data FROM PUBLIC;
ALTER DEFAULT PRIVILEGES REVOKE ALL PRIVILEGES ON SCHEMAS FROM PUBLIC;
ALTER DEFAULT PRIVILEGES REVOKE ALL PRIVILEGES ON TABLES FROM PUBLIC;
ALTER DEFAULT PRIVILEGES REVOKE ALL PRIVILEGES ON SEQUENCES FROM PUBLIC;
ALTER DEFAULT PRIVILEGES REVOKE ALL PRIVILEGES ON FUNCTIONS FROM PUBLIC;
ALTER DEFAULT PRIVILEGES REVOKE ALL PRIVILEGES ON TYPES FROM PUBLIC;
RESET ROLE;

SET LOCAL ROLE ple_private_owner;
CREATE SCHEMA ple_private AUTHORIZATION ple_private_owner;
REVOKE ALL PRIVILEGES ON SCHEMA ple_private FROM PUBLIC;
ALTER DEFAULT PRIVILEGES REVOKE ALL PRIVILEGES ON SCHEMAS FROM PUBLIC;
ALTER DEFAULT PRIVILEGES REVOKE ALL PRIVILEGES ON TABLES FROM PUBLIC;
ALTER DEFAULT PRIVILEGES REVOKE ALL PRIVILEGES ON SEQUENCES FROM PUBLIC;
ALTER DEFAULT PRIVILEGES REVOKE ALL PRIVILEGES ON FUNCTIONS FROM PUBLIC;
ALTER DEFAULT PRIVILEGES REVOKE ALL PRIVILEGES ON TYPES FROM PUBLIC;
RESET ROLE;

SET LOCAL ROLE ple_audit_owner;
CREATE SCHEMA ple_audit AUTHORIZATION ple_audit_owner;
REVOKE ALL PRIVILEGES ON SCHEMA ple_audit FROM PUBLIC;
ALTER DEFAULT PRIVILEGES REVOKE ALL PRIVILEGES ON SCHEMAS FROM PUBLIC;
ALTER DEFAULT PRIVILEGES REVOKE ALL PRIVILEGES ON TABLES FROM PUBLIC;
ALTER DEFAULT PRIVILEGES REVOKE ALL PRIVILEGES ON SEQUENCES FROM PUBLIC;
ALTER DEFAULT PRIVILEGES REVOKE ALL PRIVILEGES ON FUNCTIONS FROM PUBLIC;
ALTER DEFAULT PRIVILEGES REVOKE ALL PRIVILEGES ON TYPES FROM PUBLIC;
RESET ROLE;

SET LOCAL ROLE ple_api_owner;
CREATE SCHEMA ple_api AUTHORIZATION ple_api_owner;
REVOKE ALL PRIVILEGES ON SCHEMA ple_api FROM PUBLIC;
ALTER DEFAULT PRIVILEGES REVOKE ALL PRIVILEGES ON SCHEMAS FROM PUBLIC;
ALTER DEFAULT PRIVILEGES REVOKE ALL PRIVILEGES ON TABLES FROM PUBLIC;
ALTER DEFAULT PRIVILEGES REVOKE ALL PRIVILEGES ON SEQUENCES FROM PUBLIC;
ALTER DEFAULT PRIVILEGES REVOKE ALL PRIVILEGES ON FUNCTIONS FROM PUBLIC;
ALTER DEFAULT PRIVILEGES REVOKE ALL PRIVILEGES ON TYPES FROM PUBLIC;
RESET ROLE;

SET LOCAL ROLE ple_database_owner;
DO $$
DECLARE
    database_name name := current_database();
BEGIN
    EXECUTE pg_catalog.format(
        'REVOKE CREATE ON DATABASE %I FROM ple_data_owner, ple_private_owner, '
        'ple_audit_owner, ple_api_owner',
        database_name
    );
END
$$;
RESET ROLE;
