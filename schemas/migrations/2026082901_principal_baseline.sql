-- Principal and schema privilege baseline with default-deny access.

DO $$
DECLARE
    migrator pg_catalog.pg_roles%ROWTYPE;
    database_owner name;
    bootstrap_superuser_oid oid;
BEGIN
    IF pg_catalog.current_setting('server_version_num')::integer / 10000 <> 17 THEN
        RAISE EXCEPTION USING
            ERRCODE = '55000',
            MESSAGE = 'migration 2026082901 requires PostgreSQL major version 17';
    END IF;

    IF current_user <> 'ple_migrator' THEN
        RAISE EXCEPTION USING
            ERRCODE = '42501',
            MESSAGE = 'migration 2026082901 must run as ple_migrator';
    END IF;

    IF NOT pg_catalog.has_schema_privilege('ple_migrator', 'pg_catalog', 'USAGE') THEN
        RAISE EXCEPTION USING
            ERRCODE = '55000',
            MESSAGE = 'ple_migrator must have PostgreSQL system catalog usage before the principal baseline runs';
    END IF;

    SELECT roles.*
      INTO migrator
      FROM pg_catalog.pg_roles AS roles
     WHERE roles.rolname = 'ple_migrator';

    IF NOT FOUND
       OR NOT migrator.rolcanlogin
       OR migrator.rolinherit
       OR migrator.rolsuper
       OR migrator.rolcreatedb
       OR NOT migrator.rolcreaterole
       OR migrator.rolreplication
       OR migrator.rolbypassrls
       OR migrator.rolconnlimit <> 2 THEN
        RAISE EXCEPTION USING
            ERRCODE = '55000',
            MESSAGE = 'ple_migrator attributes do not match the baseline bootstrap contract';
    END IF;

    SELECT owner_role.rolname
      INTO database_owner
      FROM pg_catalog.pg_database AS databases
      JOIN pg_catalog.pg_roles AS owner_role
        ON owner_role.oid = databases.datdba
     WHERE databases.datname = current_database();

    IF database_owner IS DISTINCT FROM 'ple_database_owner' THEN
        RAISE EXCEPTION USING
            ERRCODE = '55000',
            MESSAGE = 'the target database must be owned by ple_database_owner';
    END IF;

    IF NOT EXISTS (
        SELECT 1
          FROM pg_catalog.pg_roles AS roles
         WHERE roles.rolname = 'ple_database_owner'
           AND NOT roles.rolcanlogin
           AND NOT roles.rolinherit
           AND NOT roles.rolsuper
           AND NOT roles.rolcreatedb
           AND NOT roles.rolcreaterole
           AND NOT roles.rolreplication
           AND NOT roles.rolbypassrls
           AND roles.rolconnlimit = -1
    ) THEN
        RAISE EXCEPTION USING
            ERRCODE = '55000',
            MESSAGE = 'ple_database_owner does not match the bootstrap contract';
    END IF;

    SELECT relations.relowner
      INTO bootstrap_superuser_oid
      FROM pg_catalog.pg_class AS relations
      JOIN pg_catalog.pg_namespace AS namespaces
        ON namespaces.oid = relations.relnamespace
      JOIN pg_catalog.pg_roles AS owners
        ON owners.oid = relations.relowner
     WHERE namespaces.nspname = 'pg_catalog'
       AND relations.relname = 'pg_authid'
       AND relations.relkind = 'r'
       AND owners.rolsuper;

    IF bootstrap_superuser_oid IS NULL THEN
        RAISE EXCEPTION USING
            ERRCODE = '55000',
            MESSAGE = 'PostgreSQL bootstrap superuser could not be resolved';
    END IF;

    IF EXISTS (
        SELECT 1
          FROM pg_catalog.pg_roles AS roles
         WHERE roles.rolname IN (
             'ple_data_owner',
             'ple_private_owner',
             'ple_audit_owner',
             'ple_api_owner',
             'ple_app',
             'ple_auth',
             'ple_student'
         )
    ) THEN
        RAISE EXCEPTION USING
            ERRCODE = '42710',
            MESSAGE = 'a reserved migration 2026082901 role already exists';
    END IF;

    IF EXISTS (
        SELECT 1
          FROM pg_catalog.pg_namespace AS namespaces
         WHERE namespaces.nspname IN (
             'ple_data',
             'ple_private',
             'ple_audit',
             'ple_api'
         )
    ) THEN
        RAISE EXCEPTION USING
            ERRCODE = '42P06',
            MESSAGE = 'a reserved migration 2026082901 schema already exists';
    END IF;

    IF (
        SELECT pg_catalog.count(*)
          FROM pg_catalog.pg_auth_members AS memberships
          JOIN pg_catalog.pg_roles AS granted_roles
            ON granted_roles.oid = memberships.roleid
          JOIN pg_catalog.pg_roles AS members
            ON members.oid = memberships.member
          JOIN pg_catalog.pg_roles AS grantors
            ON grantors.oid = memberships.grantor
         WHERE granted_roles.rolname = 'ple_database_owner'
           AND members.rolname = 'ple_migrator'
           AND grantors.oid = bootstrap_superuser_oid
           AND grantors.rolsuper
           AND NOT memberships.admin_option
           AND NOT memberships.inherit_option
           AND memberships.set_option
    ) <> 1 OR (
        SELECT pg_catalog.count(*)
          FROM pg_catalog.pg_auth_members AS memberships
         WHERE memberships.roleid = migrator.oid
            OR memberships.member = migrator.oid
    ) <> 1 THEN
        RAISE EXCEPTION USING
            ERRCODE = '55000',
            MESSAGE = 'bootstrap role membership does not match the baseline bootstrap contract';
    END IF;

    IF EXISTS (
        SELECT 1
          FROM pg_catalog.pg_database AS databases
          CROSS JOIN LATERAL pg_catalog.aclexplode(
              COALESCE(
                  databases.datacl,
                  pg_catalog.acldefault('d', databases.datdba)
              )
          ) AS privileges
         WHERE databases.datname = current_database()
           AND privileges.grantee = 0
    ) OR (
        SELECT pg_catalog.count(*)
          FROM pg_catalog.pg_database AS databases
          CROSS JOIN LATERAL pg_catalog.aclexplode(
              COALESCE(
                  databases.datacl,
                  pg_catalog.acldefault('d', databases.datdba)
              )
          ) AS privileges
         WHERE databases.datname = current_database()
           AND privileges.grantee = migrator.oid
           AND privileges.privilege_type = 'CONNECT'
    ) <> 1 OR EXISTS (
        SELECT 1
          FROM pg_catalog.pg_database AS databases
          CROSS JOIN LATERAL pg_catalog.aclexplode(
              COALESCE(
                  databases.datacl,
                  pg_catalog.acldefault('d', databases.datdba)
              )
          ) AS privileges
         WHERE databases.datname = current_database()
           AND privileges.grantee = migrator.oid
           AND privileges.privilege_type <> 'CONNECT'
    ) THEN
        RAISE EXCEPTION USING
            ERRCODE = '55000',
            MESSAGE = 'target database ACLs do not match the bootstrap contract';
    END IF;

    IF EXISTS (
        SELECT 1
          FROM pg_catalog.pg_namespace AS namespaces
          CROSS JOIN LATERAL pg_catalog.aclexplode(
              COALESCE(
                  namespaces.nspacl,
                  pg_catalog.acldefault('n', namespaces.nspowner)
              )
          ) AS privileges
         WHERE namespaces.nspname = 'public'
           AND privileges.grantee = 0
    ) OR (
        SELECT pg_catalog.count(*)
          FROM pg_catalog.pg_namespace AS namespaces
          CROSS JOIN LATERAL pg_catalog.aclexplode(
              COALESCE(
                  namespaces.nspacl,
                  pg_catalog.acldefault('n', namespaces.nspowner)
              )
          ) AS privileges
         WHERE namespaces.nspname = 'public'
           AND privileges.grantee = migrator.oid
           AND privileges.privilege_type IN ('USAGE', 'CREATE')
    ) <> 2 THEN
        RAISE EXCEPTION USING
            ERRCODE = '55000',
            MESSAGE = 'public schema ACLs do not match the SQLx bootstrap contract';
    END IF;

    IF pg_catalog.to_regclass('public._sqlx_migrations') IS NULL THEN
        RAISE EXCEPTION USING
            ERRCODE = '42P01',
            MESSAGE = 'the SQLx migration ledger is absent';
    END IF;

    IF NOT EXISTS (
        SELECT 1
          FROM pg_catalog.pg_class AS relations
          JOIN pg_catalog.pg_namespace AS namespaces
            ON namespaces.oid = relations.relnamespace
          JOIN pg_catalog.pg_roles AS owners
            ON owners.oid = relations.relowner
         WHERE namespaces.nspname = 'public'
           AND relations.relname = '_sqlx_migrations'
           AND relations.relkind = 'r'
           AND owners.rolname = 'ple_migrator'
    ) THEN
        RAISE EXCEPTION USING
            ERRCODE = '55000',
            MESSAGE = 'the SQLx migration ledger must be owned by ple_migrator';
    END IF;
END
$$;

CREATE ROLE ple_data_owner
    NOLOGIN NOINHERIT NOSUPERUSER NOCREATEDB NOCREATEROLE
    NOREPLICATION NOBYPASSRLS;
CREATE ROLE ple_private_owner
    NOLOGIN NOINHERIT NOSUPERUSER NOCREATEDB NOCREATEROLE
    NOREPLICATION NOBYPASSRLS;
CREATE ROLE ple_audit_owner
    NOLOGIN NOINHERIT NOSUPERUSER NOCREATEDB NOCREATEROLE
    NOREPLICATION NOBYPASSRLS;
CREATE ROLE ple_api_owner
    NOLOGIN NOINHERIT NOSUPERUSER NOCREATEDB NOCREATEROLE
    NOREPLICATION NOBYPASSRLS;

CREATE ROLE ple_app
    NOLOGIN NOINHERIT NOSUPERUSER NOCREATEDB NOCREATEROLE
    NOREPLICATION NOBYPASSRLS;
CREATE ROLE ple_auth
    NOLOGIN NOINHERIT NOSUPERUSER NOCREATEDB NOCREATEROLE
    NOREPLICATION NOBYPASSRLS;
CREATE ROLE ple_student
    NOLOGIN NOINHERIT NOSUPERUSER NOCREATEDB NOCREATEROLE
    NOREPLICATION NOBYPASSRLS;

-- Every fixed migration principal resolves PostgreSQL built-ins and operators through
-- the server-owned catalog. This is not a capability to PLE data or private
-- records; PLE schemas remain explicitly default-deny below.
-- ASVS 8.2.1, 8.2.2: application-object access is granted only by the
-- owning PLE schema and remains separate from system-name resolution.
GRANT USAGE ON SCHEMA pg_catalog TO ple_database_owner, ple_data_owner,
    ple_private_owner, ple_audit_owner, ple_api_owner, ple_app, ple_auth,
    ple_student;

GRANT ple_data_owner TO ple_migrator
    WITH ADMIN FALSE, INHERIT FALSE, SET TRUE
    GRANTED BY ple_migrator;
GRANT ple_private_owner TO ple_migrator
    WITH ADMIN FALSE, INHERIT FALSE, SET TRUE
    GRANTED BY ple_migrator;
GRANT ple_audit_owner TO ple_migrator
    WITH ADMIN FALSE, INHERIT FALSE, SET TRUE
    GRANTED BY ple_migrator;
GRANT ple_api_owner TO ple_migrator
    WITH ADMIN FALSE, INHERIT FALSE, SET TRUE
    GRANTED BY ple_migrator;

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
    EXECUTE pg_catalog.format(
        'GRANT CREATE ON DATABASE %I TO ple_data_owner, ple_private_owner, '
        'ple_audit_owner, ple_api_owner',
        database_name
    );
    EXECUTE pg_catalog.format(
        'COMMENT ON DATABASE %I IS %L',
        database_name,
        'Owned PLE application namespace with default-deny runtime privileges.'
    );
END
$$;

REVOKE ALL PRIVILEGES ON SCHEMA public FROM PUBLIC;
REVOKE CREATE ON SCHEMA public FROM ple_migrator;
GRANT USAGE ON SCHEMA public TO ple_migrator, ple_api_owner;

RESET ROLE;

-- PostgreSQL's system catalogs are part of the server trust boundary, not PLE
-- application namespaces. Preserve their ordinary catalog visibility while
-- each PLE-owned schema below establishes explicit default-deny privileges.
-- ASVS 8.2.1, 8.2.2: only PLE application objects receive explicit runtime
-- capabilities; removing catalog visibility cannot strengthen that boundary.
ALTER DEFAULT PRIVILEGES FOR ROLE ple_migrator
    REVOKE ALL PRIVILEGES ON TABLES FROM PUBLIC;
ALTER DEFAULT PRIVILEGES FOR ROLE ple_migrator
    REVOKE ALL PRIVILEGES ON SEQUENCES FROM PUBLIC;
ALTER DEFAULT PRIVILEGES FOR ROLE ple_migrator
    REVOKE ALL PRIVILEGES ON ROUTINES FROM PUBLIC;
ALTER DEFAULT PRIVILEGES FOR ROLE ple_migrator
    REVOKE ALL PRIVILEGES ON TYPES FROM PUBLIC;

SET LOCAL ROLE ple_database_owner;
ALTER DEFAULT PRIVILEGES REVOKE ALL PRIVILEGES ON SCHEMAS FROM PUBLIC;
ALTER DEFAULT PRIVILEGES REVOKE ALL PRIVILEGES ON TABLES FROM PUBLIC;
ALTER DEFAULT PRIVILEGES REVOKE ALL PRIVILEGES ON SEQUENCES FROM PUBLIC;
ALTER DEFAULT PRIVILEGES REVOKE ALL PRIVILEGES ON ROUTINES FROM PUBLIC;
ALTER DEFAULT PRIVILEGES REVOKE ALL PRIVILEGES ON TYPES FROM PUBLIC;
RESET ROLE;

SET LOCAL ROLE ple_data_owner;
CREATE SCHEMA ple_data AUTHORIZATION ple_data_owner;
REVOKE ALL PRIVILEGES ON SCHEMA ple_data FROM PUBLIC;
ALTER DEFAULT PRIVILEGES REVOKE ALL PRIVILEGES ON SCHEMAS FROM PUBLIC;
ALTER DEFAULT PRIVILEGES REVOKE ALL PRIVILEGES ON TABLES FROM PUBLIC;
ALTER DEFAULT PRIVILEGES REVOKE ALL PRIVILEGES ON SEQUENCES FROM PUBLIC;
ALTER DEFAULT PRIVILEGES REVOKE ALL PRIVILEGES ON ROUTINES FROM PUBLIC;
ALTER DEFAULT PRIVILEGES REVOKE ALL PRIVILEGES ON TYPES FROM PUBLIC;
RESET ROLE;

SET LOCAL ROLE ple_private_owner;
CREATE SCHEMA ple_private AUTHORIZATION ple_private_owner;
REVOKE ALL PRIVILEGES ON SCHEMA ple_private FROM PUBLIC;
ALTER DEFAULT PRIVILEGES REVOKE ALL PRIVILEGES ON SCHEMAS FROM PUBLIC;
ALTER DEFAULT PRIVILEGES REVOKE ALL PRIVILEGES ON TABLES FROM PUBLIC;
ALTER DEFAULT PRIVILEGES REVOKE ALL PRIVILEGES ON SEQUENCES FROM PUBLIC;
ALTER DEFAULT PRIVILEGES REVOKE ALL PRIVILEGES ON ROUTINES FROM PUBLIC;
ALTER DEFAULT PRIVILEGES REVOKE ALL PRIVILEGES ON TYPES FROM PUBLIC;
RESET ROLE;

SET LOCAL ROLE ple_audit_owner;
CREATE SCHEMA ple_audit AUTHORIZATION ple_audit_owner;
REVOKE ALL PRIVILEGES ON SCHEMA ple_audit FROM PUBLIC;
ALTER DEFAULT PRIVILEGES REVOKE ALL PRIVILEGES ON SCHEMAS FROM PUBLIC;
ALTER DEFAULT PRIVILEGES REVOKE ALL PRIVILEGES ON TABLES FROM PUBLIC;
ALTER DEFAULT PRIVILEGES REVOKE ALL PRIVILEGES ON SEQUENCES FROM PUBLIC;
ALTER DEFAULT PRIVILEGES REVOKE ALL PRIVILEGES ON ROUTINES FROM PUBLIC;
ALTER DEFAULT PRIVILEGES REVOKE ALL PRIVILEGES ON TYPES FROM PUBLIC;
RESET ROLE;

SET LOCAL ROLE ple_api_owner;
CREATE SCHEMA ple_api AUTHORIZATION ple_api_owner;
REVOKE ALL PRIVILEGES ON SCHEMA ple_api FROM PUBLIC;
ALTER DEFAULT PRIVILEGES REVOKE ALL PRIVILEGES ON SCHEMAS FROM PUBLIC;
ALTER DEFAULT PRIVILEGES REVOKE ALL PRIVILEGES ON TABLES FROM PUBLIC;
ALTER DEFAULT PRIVILEGES REVOKE ALL PRIVILEGES ON SEQUENCES FROM PUBLIC;
ALTER DEFAULT PRIVILEGES REVOKE ALL PRIVILEGES ON ROUTINES FROM PUBLIC;
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

GRANT SELECT ON TABLE public._sqlx_migrations TO ple_api_owner;

SET LOCAL ROLE ple_api_owner;

CREATE VIEW ple_api.ple_migration_state AS
SELECT migrations.version,
       migrations.success,
       migrations.checksum
  FROM public._sqlx_migrations AS migrations;

REVOKE ALL PRIVILEGES ON TABLE ple_api.ple_migration_state FROM PUBLIC;
GRANT USAGE ON SCHEMA ple_api TO ple_app;
GRANT SELECT ON TABLE ple_api.ple_migration_state TO ple_app;

COMMENT ON SCHEMA ple_api IS
    'Safe API projections and explicitly granted invocation entry points.';

RESET ROLE;

SET LOCAL ROLE ple_data_owner;
COMMENT ON SCHEMA ple_data IS
    'Ordinary relational roots and forced-row-security application data.';
RESET ROLE;

SET LOCAL ROLE ple_private_owner;
COMMENT ON SCHEMA ple_private IS
    'Authentication, answer-bearing, credential, and server-private data.';
RESET ROLE;

SET LOCAL ROLE ple_audit_owner;
COMMENT ON SCHEMA ple_audit IS
    'Immutable receipts, access audit, and protected evidence.';
RESET ROLE;

COMMENT ON ROLE ple_data_owner IS
    'Owns ordinary relational and forced-row-security data objects.';
COMMENT ON ROLE ple_private_owner IS
    'Owns authentication, answer-bearing, credential, and private objects.';
COMMENT ON ROLE ple_audit_owner IS
    'Owns immutable receipt, access-audit, and protected-evidence objects.';
COMMENT ON ROLE ple_api_owner IS
    'Owns safe API projections and ordinary invoker objects.';
COMMENT ON ROLE ple_app IS
    'Invokes ordinary session-authorized API operations.';
COMMENT ON ROLE ple_auth IS
    'Invokes authentication and session-resolution operations.';
COMMENT ON ROLE ple_student IS
    'Invokes Student-safe session-authorized projections.';

DO $$
DECLARE
    reserved_membership_rows bigint;
    migration_view_oid oid;
    projected_columns name[];
    bootstrap_superuser_oid oid;
BEGIN
    SELECT relations.relowner
      INTO bootstrap_superuser_oid
      FROM pg_catalog.pg_class AS relations
      JOIN pg_catalog.pg_namespace AS namespaces
        ON namespaces.oid = relations.relnamespace
      JOIN pg_catalog.pg_roles AS owners
        ON owners.oid = relations.relowner
     WHERE namespaces.nspname = 'pg_catalog'
       AND relations.relname = 'pg_authid'
       AND relations.relkind = 'r'
       AND owners.rolsuper;

    IF bootstrap_superuser_oid IS NULL THEN
        RAISE EXCEPTION USING
            ERRCODE = '55000',
            MESSAGE = 'PostgreSQL bootstrap superuser could not be resolved';
    END IF;

    IF NOT pg_catalog.has_schema_privilege('pg_catalog', 'USAGE')
       OR NOT pg_catalog.has_schema_privilege('ple_data_owner', 'pg_catalog', 'USAGE')
       OR NOT pg_catalog.has_schema_privilege('ple_private_owner', 'pg_catalog', 'USAGE')
       OR NOT pg_catalog.has_schema_privilege('ple_audit_owner', 'pg_catalog', 'USAGE')
       OR NOT pg_catalog.has_schema_privilege('ple_api_owner', 'pg_catalog', 'USAGE')
       OR NOT pg_catalog.has_schema_privilege('ple_app', 'pg_catalog', 'USAGE')
       OR NOT pg_catalog.has_schema_privilege('ple_auth', 'pg_catalog', 'USAGE')
       OR NOT pg_catalog.has_schema_privilege('ple_student', 'pg_catalog', 'USAGE') THEN
        RAISE EXCEPTION USING
            ERRCODE = '55000',
            MESSAGE = 'PostgreSQL system catalog visibility must remain available to baseline migration principals';
    END IF;

    IF (
        SELECT pg_catalog.count(*)
          FROM pg_catalog.pg_roles AS roles
         WHERE roles.rolname IN (
             'ple_database_owner',
             'ple_data_owner',
             'ple_private_owner',
             'ple_audit_owner',
             'ple_api_owner',
             'ple_app',
             'ple_auth',
             'ple_student'
         )
           AND NOT roles.rolcanlogin
           AND NOT roles.rolinherit
           AND NOT roles.rolsuper
           AND NOT roles.rolcreatedb
           AND NOT roles.rolcreaterole
           AND NOT roles.rolreplication
           AND NOT roles.rolbypassrls
           AND roles.rolconnlimit = -1
    ) <> 8 THEN
        RAISE EXCEPTION USING
            ERRCODE = '55000',
            MESSAGE = 'reserved role attributes do not match the baseline role contract';
    END IF;

    IF (
        SELECT pg_catalog.count(*)
          FROM pg_catalog.pg_auth_members AS memberships
          JOIN pg_catalog.pg_roles AS granted_roles
            ON granted_roles.oid = memberships.roleid
          JOIN pg_catalog.pg_roles AS members
            ON members.oid = memberships.member
          JOIN pg_catalog.pg_roles AS grantors
            ON grantors.oid = memberships.grantor
         WHERE granted_roles.rolname IN (
             'ple_database_owner',
             'ple_data_owner',
             'ple_private_owner',
             'ple_audit_owner',
             'ple_api_owner',
             'ple_app',
             'ple_auth',
             'ple_student'
         )
           AND members.rolname = 'ple_migrator'
           AND grantors.oid = bootstrap_superuser_oid
           AND grantors.rolsuper
           AND memberships.admin_option
           AND NOT memberships.inherit_option
           AND NOT memberships.set_option
    ) <> 7 THEN
        RAISE EXCEPTION USING
            ERRCODE = '55000',
            MESSAGE = 'automatic creator memberships do not match PostgreSQL 17';
    END IF;

    IF (
        SELECT pg_catalog.count(*)
          FROM pg_catalog.pg_auth_members AS memberships
          JOIN pg_catalog.pg_roles AS granted_roles
            ON granted_roles.oid = memberships.roleid
          JOIN pg_catalog.pg_roles AS members
            ON members.oid = memberships.member
          JOIN pg_catalog.pg_roles AS grantors
            ON grantors.oid = memberships.grantor
         WHERE granted_roles.rolname IN (
             'ple_data_owner',
             'ple_private_owner',
             'ple_audit_owner',
             'ple_api_owner'
         )
           AND members.rolname = 'ple_migrator'
           AND grantors.rolname = 'ple_migrator'
           AND NOT memberships.admin_option
           AND NOT memberships.inherit_option
           AND memberships.set_option
    ) <> 4 THEN
        RAISE EXCEPTION USING
            ERRCODE = '55000',
            MESSAGE = 'owner SET memberships do not match the baseline role contract';
    END IF;

    IF (
        SELECT pg_catalog.count(*)
          FROM pg_catalog.pg_auth_members AS memberships
          JOIN pg_catalog.pg_roles AS granted_roles
            ON granted_roles.oid = memberships.roleid
          JOIN pg_catalog.pg_roles AS members
            ON members.oid = memberships.member
          JOIN pg_catalog.pg_roles AS grantors
            ON grantors.oid = memberships.grantor
         WHERE granted_roles.rolname = 'ple_database_owner'
           AND members.rolname = 'ple_migrator'
           AND grantors.oid = bootstrap_superuser_oid
           AND grantors.rolsuper
           AND NOT memberships.admin_option
           AND NOT memberships.inherit_option
           AND memberships.set_option
    ) <> 1 THEN
        RAISE EXCEPTION USING
            ERRCODE = '55000',
            MESSAGE = 'database-owner bootstrap membership does not match the baseline role contract';
    END IF;

    SELECT pg_catalog.count(*)
      INTO reserved_membership_rows
      FROM pg_catalog.pg_auth_members AS memberships
     WHERE memberships.roleid IN (
         SELECT roles.oid
           FROM pg_catalog.pg_roles AS roles
          WHERE roles.rolname IN (
              'ple_migrator',
              'ple_database_owner',
              'ple_data_owner',
              'ple_private_owner',
              'ple_audit_owner',
              'ple_api_owner',
              'ple_app',
              'ple_auth',
              'ple_student'
          )
     )
        OR memberships.member IN (
         SELECT roles.oid
           FROM pg_catalog.pg_roles AS roles
          WHERE roles.rolname IN (
              'ple_migrator',
              'ple_database_owner',
              'ple_data_owner',
              'ple_private_owner',
              'ple_audit_owner',
              'ple_api_owner',
              'ple_app',
              'ple_auth',
              'ple_student'
          )
     );

    IF reserved_membership_rows <> 12 THEN
        RAISE EXCEPTION USING
            ERRCODE = '55000',
            MESSAGE = 'unexpected membership involving a reserved baseline role';
    END IF;

    IF NOT EXISTS (
        SELECT 1
          FROM pg_catalog.pg_database AS databases
          JOIN pg_catalog.pg_roles AS owners
            ON owners.oid = databases.datdba
         WHERE databases.datname = current_database()
           AND owners.rolname = 'ple_database_owner'
    ) THEN
        RAISE EXCEPTION USING
            ERRCODE = '55000',
            MESSAGE = 'database ownership does not match the baseline ownership contract';
    END IF;

    IF EXISTS (
        SELECT 1
          FROM pg_catalog.pg_database AS databases
          CROSS JOIN LATERAL pg_catalog.aclexplode(
              COALESCE(
                  databases.datacl,
                  pg_catalog.acldefault('d', databases.datdba)
              )
          ) AS privileges
         WHERE databases.datname = current_database()
           AND privileges.grantee = 0
    ) THEN
        RAISE EXCEPTION USING
            ERRCODE = '55000',
            MESSAGE = 'PUBLIC retains a target-database privilege';
    END IF;

    IF (
        SELECT pg_catalog.count(*)
          FROM pg_catalog.pg_namespace AS namespaces
          JOIN pg_catalog.pg_roles AS owners
            ON owners.oid = namespaces.nspowner
         WHERE (namespaces.nspname, owners.rolname) IN (
             ('ple_data', 'ple_data_owner'),
             ('ple_private', 'ple_private_owner'),
             ('ple_audit', 'ple_audit_owner'),
             ('ple_api', 'ple_api_owner')
         )
    ) <> 4 THEN
        RAISE EXCEPTION USING
            ERRCODE = '55000',
            MESSAGE = 'schema ownership does not match the baseline schema ownership contract';
    END IF;

    IF EXISTS (
        SELECT 1
          FROM pg_catalog.pg_namespace AS namespaces
          CROSS JOIN LATERAL pg_catalog.aclexplode(
              COALESCE(
                  namespaces.nspacl,
                  pg_catalog.acldefault('n', namespaces.nspowner)
              )
          ) AS privileges
         WHERE namespaces.nspname IN (
             'public',
             'ple_data',
             'ple_private',
             'ple_audit',
             'ple_api'
         )
           AND privileges.grantee = 0
    ) THEN
        RAISE EXCEPTION USING
            ERRCODE = '55000',
            MESSAGE = 'PUBLIC retains a baseline schema privilege';
    END IF;

    IF NOT pg_catalog.has_schema_privilege('ple_migrator', 'public', 'USAGE')
       OR pg_catalog.has_schema_privilege('ple_migrator', 'public', 'CREATE')
       OR NOT pg_catalog.has_schema_privilege('ple_api_owner', 'public', 'USAGE')
       OR pg_catalog.has_schema_privilege('ple_api_owner', 'public', 'CREATE') THEN
        RAISE EXCEPTION USING
            ERRCODE = '55000',
            MESSAGE = 'public schema direct privileges do not match the baseline schema privilege contract';
    END IF;

    IF EXISTS (
        SELECT 1
          FROM pg_catalog.pg_default_acl AS defaults
          JOIN pg_catalog.pg_roles AS owners
            ON owners.oid = defaults.defaclrole
          CROSS JOIN LATERAL pg_catalog.aclexplode(defaults.defaclacl) AS privileges
         WHERE owners.rolname IN (
             'ple_migrator',
             'ple_database_owner',
             'ple_data_owner',
             'ple_private_owner',
             'ple_audit_owner',
             'ple_api_owner'
         )
           AND privileges.grantee = 0
    ) THEN
        RAISE EXCEPTION USING
            ERRCODE = '55000',
            MESSAGE = 'PUBLIC retains a default object privilege';
    END IF;

    IF (
        SELECT pg_catalog.count(DISTINCT defaults.defaclrole)
          FROM pg_catalog.pg_default_acl AS defaults
          JOIN pg_catalog.pg_roles AS owners
            ON owners.oid = defaults.defaclrole
         WHERE owners.rolname IN (
             'ple_migrator',
             'ple_database_owner',
             'ple_data_owner',
             'ple_private_owner',
             'ple_audit_owner',
             'ple_api_owner'
         )
           AND defaults.defaclobjtype = 'f'
    ) <> 6 THEN
        RAISE EXCEPTION USING
            ERRCODE = '55000',
            MESSAGE = 'global routine default ACL closure is incomplete';
    END IF;

    IF (
        SELECT pg_catalog.count(DISTINCT defaults.defaclrole)
          FROM pg_catalog.pg_default_acl AS defaults
          JOIN pg_catalog.pg_roles AS owners
            ON owners.oid = defaults.defaclrole
         WHERE owners.rolname IN (
             'ple_migrator',
             'ple_database_owner',
             'ple_data_owner',
             'ple_private_owner',
             'ple_audit_owner',
             'ple_api_owner'
         )
           AND defaults.defaclobjtype = 'T'
    ) <> 6 THEN
        RAISE EXCEPTION USING
            ERRCODE = '55000',
            MESSAGE = 'global type default ACL closure is incomplete';
    END IF;

    SELECT relations.oid,
           pg_catalog.array_agg(attributes.attname ORDER BY attributes.attnum)
      INTO migration_view_oid, projected_columns
      FROM pg_catalog.pg_class AS relations
      JOIN pg_catalog.pg_namespace AS namespaces
        ON namespaces.oid = relations.relnamespace
      JOIN pg_catalog.pg_roles AS owners
        ON owners.oid = relations.relowner
      JOIN pg_catalog.pg_attribute AS attributes
        ON attributes.attrelid = relations.oid
       AND attributes.attnum > 0
       AND NOT attributes.attisdropped
     WHERE namespaces.nspname = 'ple_api'
       AND relations.relname = 'ple_migration_state'
       AND relations.relkind = 'v'
       AND owners.rolname = 'ple_api_owner'
     GROUP BY relations.oid;

    IF projected_columns IS DISTINCT FROM ARRAY['version', 'success', 'checksum']::name[]
       OR NOT pg_catalog.has_schema_privilege('ple_app', 'ple_api', 'USAGE')
       OR NOT pg_catalog.has_table_privilege(
           'ple_app',
           migration_view_oid,
           'SELECT'
       )
       OR pg_catalog.has_table_privilege(
           'ple_app',
           'public._sqlx_migrations',
           'SELECT'
       ) THEN
        RAISE EXCEPTION USING
            ERRCODE = '55000',
            MESSAGE = 'safe migration-state projection does not match the baseline schema privilege contract';
    END IF;
END
$$;

RESET ROLE;
