-- Cluster roles, schemas, and default-deny ACLs.

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
    -- ASVS 8.2.1: execution capabilities are explicit non-login roles; only
    -- the restricted migration principal may SET the installation capability.
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

    IF EXISTS (
        SELECT 1
          FROM pg_catalog.pg_roles AS role
         WHERE role.rolname IN (
             'ple_unrelease_executor',
             'ple_course_retention_executor',
             'ple_course_retention_notifier',
             'ple_course_retention_notification_owner'
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
    ) OR EXISTS (
        SELECT 1
          FROM pg_catalog.pg_roles AS role
         WHERE role.rolname IN (
             'ple_unrelease_executor',
             'ple_course_retention_executor',
             'ple_course_retention_notifier',
             'ple_course_retention_notification_owner'
         )
           AND (
               SELECT count(*)
                 FROM pg_catalog.pg_auth_members AS membership
                WHERE membership.roleid = role.oid
           ) <> 1
    ) OR NOT EXISTS (
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
             'ple_assessment_attempt_expiry_worker',
             'ple_unrelease_executor',
             'ple_course_retention_executor',
             'ple_course_retention_notifier',
             'ple_course_retention_notification_owner',
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
             'ple_assessment_attempt_expiry_worker',
             'ple_unrelease_executor',
             'ple_course_retention_executor',
             'ple_course_retention_notifier',
             'ple_course_retention_notification_owner',
             'ple_data_owner',
             'ple_private_owner',
             'ple_audit_owner',
             'ple_api_owner',
             'ple_app',
             'ple_auth',
             'ple_student'
         )
    ) <> 13 THEN
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
    ) <> 9 OR (
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
               'ple_course_retention_executor',
               'ple_course_retention_notifier',
               'ple_course_retention_notification_owner',
               'ple_data_owner',
               'ple_private_owner',
               'ple_audit_owner',
               'ple_api_owner'
           )
           AND NOT membership.admin_option
           AND NOT membership.inherit_option
           AND membership.set_option
    ) <> 9 THEN
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
    ple_audit_owner, ple_api_owner, ple_course_retention_notification_owner,
    ple_app, ple_auth, ple_student;

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

GRANT USAGE ON SCHEMA ple_migration TO ple_api_owner;

SET LOCAL ROLE ple_data_owner;

CREATE SCHEMA ple_data AUTHORIZATION ple_data_owner;

REVOKE ALL PRIVILEGES ON SCHEMA ple_data FROM PUBLIC;

ALTER DEFAULT PRIVILEGES REVOKE ALL PRIVILEGES ON SCHEMAS FROM PUBLIC;

ALTER DEFAULT PRIVILEGES REVOKE ALL PRIVILEGES ON TABLES FROM PUBLIC;

ALTER DEFAULT PRIVILEGES GRANT REFERENCES ON TABLES TO
    ple_private_owner, ple_audit_owner, ple_api_owner;

ALTER DEFAULT PRIVILEGES REVOKE ALL PRIVILEGES ON SEQUENCES FROM PUBLIC;

ALTER DEFAULT PRIVILEGES REVOKE ALL PRIVILEGES ON FUNCTIONS FROM PUBLIC;

ALTER DEFAULT PRIVILEGES GRANT EXECUTE ON FUNCTIONS TO
    ple_private_owner, ple_audit_owner, ple_api_owner;

ALTER DEFAULT PRIVILEGES REVOKE ALL PRIVILEGES ON TYPES FROM PUBLIC;

SET LOCAL ROLE ple_private_owner;

CREATE SCHEMA ple_private AUTHORIZATION ple_private_owner;

REVOKE ALL PRIVILEGES ON SCHEMA ple_private FROM PUBLIC;

ALTER DEFAULT PRIVILEGES REVOKE ALL PRIVILEGES ON SCHEMAS FROM PUBLIC;

ALTER DEFAULT PRIVILEGES REVOKE ALL PRIVILEGES ON TABLES FROM PUBLIC;

ALTER DEFAULT PRIVILEGES GRANT REFERENCES ON TABLES TO
    ple_data_owner, ple_audit_owner, ple_api_owner;

ALTER DEFAULT PRIVILEGES REVOKE ALL PRIVILEGES ON SEQUENCES FROM PUBLIC;

ALTER DEFAULT PRIVILEGES REVOKE ALL PRIVILEGES ON FUNCTIONS FROM PUBLIC;

ALTER DEFAULT PRIVILEGES GRANT EXECUTE ON FUNCTIONS TO
    ple_data_owner, ple_audit_owner, ple_api_owner;

ALTER DEFAULT PRIVILEGES REVOKE ALL PRIVILEGES ON TYPES FROM PUBLIC;

SET LOCAL ROLE ple_audit_owner;

CREATE SCHEMA ple_audit AUTHORIZATION ple_audit_owner;

REVOKE ALL PRIVILEGES ON SCHEMA ple_audit FROM PUBLIC;

ALTER DEFAULT PRIVILEGES REVOKE ALL PRIVILEGES ON SCHEMAS FROM PUBLIC;

ALTER DEFAULT PRIVILEGES REVOKE ALL PRIVILEGES ON TABLES FROM PUBLIC;

ALTER DEFAULT PRIVILEGES GRANT REFERENCES ON TABLES TO
    ple_data_owner, ple_private_owner, ple_api_owner;

ALTER DEFAULT PRIVILEGES REVOKE ALL PRIVILEGES ON SEQUENCES FROM PUBLIC;

ALTER DEFAULT PRIVILEGES REVOKE ALL PRIVILEGES ON FUNCTIONS FROM PUBLIC;

ALTER DEFAULT PRIVILEGES GRANT EXECUTE ON FUNCTIONS TO
    ple_data_owner, ple_private_owner, ple_api_owner;

ALTER DEFAULT PRIVILEGES REVOKE ALL PRIVILEGES ON TYPES FROM PUBLIC;

SET LOCAL ROLE ple_api_owner;

CREATE SCHEMA ple_api AUTHORIZATION ple_api_owner;

REVOKE ALL PRIVILEGES ON SCHEMA ple_api FROM PUBLIC;

ALTER DEFAULT PRIVILEGES REVOKE ALL PRIVILEGES ON SCHEMAS FROM PUBLIC;

ALTER DEFAULT PRIVILEGES REVOKE ALL PRIVILEGES ON TABLES FROM PUBLIC;

ALTER DEFAULT PRIVILEGES REVOKE ALL PRIVILEGES ON SEQUENCES FROM PUBLIC;

ALTER DEFAULT PRIVILEGES REVOKE ALL PRIVILEGES ON FUNCTIONS FROM PUBLIC;

ALTER DEFAULT PRIVILEGES REVOKE ALL PRIVILEGES ON TYPES FROM PUBLIC;

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

SET LOCAL ROLE ple_private_owner;

GRANT USAGE ON SCHEMA ple_private TO ple_data_owner, ple_api_owner;

GRANT USAGE ON SCHEMA ple_private TO ple_api_owner;

GRANT USAGE ON SCHEMA ple_private TO ple_data_owner;

GRANT USAGE ON SCHEMA ple_private TO ple_audit_owner;

SET LOCAL ROLE ple_audit_owner;

GRANT USAGE ON SCHEMA ple_audit TO ple_private_owner;

SET LOCAL ROLE ple_api_owner;

GRANT USAGE ON SCHEMA ple_api TO ple_app;

SET LOCAL ROLE ple_private_owner;

GRANT USAGE ON SCHEMA ple_private TO ple_api_owner;

SET LOCAL ROLE ple_api_owner;

GRANT USAGE ON SCHEMA ple_api TO ple_auth;

GRANT USAGE ON SCHEMA ple_api TO ple_app, ple_auth, ple_student, ple_data_owner, ple_private_owner;

SET LOCAL ROLE ple_data_owner;

-- Authenticated global vocabulary commands; no content references or lifecycle states.
-- ASVS 8.2.1/8.2.2: the installed session, never caller-supplied roles, authorizes
-- private commands. API wrappers alone are executable by ple_app.
GRANT USAGE ON SCHEMA ple_data TO ple_private_owner;

SET LOCAL ROLE ple_private_owner;

-- Shared Question Library lineages.  Availability is current lineage state;
-- exact immutable revisions remain available to authorized historical readers.

-- The lineage rows record their publishing actor.  These grants must precede
-- the cross-schema foreign keys below: question_stewardship is intentionally
-- later because it depends on the lineage tables.
GRANT USAGE ON SCHEMA ple_private TO ple_data_owner;

GRANT USAGE ON SCHEMA ple_private TO ple_api_owner;

SET LOCAL ROLE ple_data_owner;

GRANT USAGE ON SCHEMA ple_data TO ple_private_owner;


-- The later security-definer source reader needs namespace resolution only;
-- its own fixed query and RLS policies remain the data boundary.
GRANT USAGE ON SCHEMA ple_data TO ple_api_owner;

SET LOCAL ROLE ple_api_owner;




-- ASVS 1.2.4, 2.2-2.3, 8.2-8.3, and 15.4: this capability is the only
-- transition path.  It locks the lineage, rechecks ownership, and records a
-- redacted, actor-attributed event in the same transaction.
GRANT USAGE ON SCHEMA ple_api TO ple_data_owner;

SET LOCAL ROLE ple_data_owner;

GRANT USAGE ON SCHEMA ple_data TO ple_api_owner;

SET LOCAL ROLE ple_private_owner;

-- Stable published Question Pool identity and immutable member pins. This
-- foundation deliberately stores no selected-count setting: selection is
-- Assessment Entry policy, never Pool lineage state.
GRANT USAGE ON SCHEMA ple_private TO ple_data_owner;

SET LOCAL ROLE ple_data_owner;

GRANT USAGE ON SCHEMA ple_data TO ple_api_owner;

GRANT USAGE ON SCHEMA ple_private TO ple_data_owner;

SET LOCAL ROLE ple_private_owner;

-- Retained text-only stewardship discussions for stable Question Library lineages.
--
-- A thread records the exact immutable Revision current when it was created,
-- while its target remains the stable Question or Pool lineage.  The later
-- operation module is the only browser-facing mutation boundary.
GRANT USAGE ON SCHEMA ple_private TO ple_data_owner;

SET LOCAL ROLE ple_api_owner;

GRANT USAGE, CREATE ON SCHEMA ple_api TO ple_data_owner;

SET LOCAL ROLE ple_private_owner;

GRANT USAGE ON SCHEMA ple_private TO ple_api_owner;

SET LOCAL ROLE ple_data_owner;

GRANT USAGE ON SCHEMA ple_data TO ple_private_owner, ple_audit_owner;

SET LOCAL ROLE ple_private_owner;

GRANT USAGE ON SCHEMA ple_private TO ple_audit_owner;

SET LOCAL ROLE ple_data_owner;

-- Private Question authoring state.  Publication copies validated values into
-- the shared lineage; draft rows never become library rows in place.
GRANT USAGE ON SCHEMA ple_data TO ple_private_owner;

SET LOCAL ROLE ple_private_owner;

GRANT USAGE ON SCHEMA ple_private TO ple_api_owner, ple_public_asset_publisher;

SET LOCAL ROLE ple_data_owner;

-- Reusable Blueprint Course lineages and immutable save-created Revisions.
GRANT USAGE ON SCHEMA ple_data TO ple_api_owner;

SET LOCAL ROLE ple_private_owner;

GRANT CREATE ON SCHEMA ple_private TO ple_api_owner;

SET LOCAL ROLE ple_data_owner;

GRANT USAGE ON SCHEMA ple_data TO ple_api_owner;

GRANT USAGE ON SCHEMA ple_data TO ple_private_owner, ple_audit_owner;

SET LOCAL ROLE ple_private_owner;

GRANT USAGE ON SCHEMA ple_private TO ple_audit_owner;

SET LOCAL ROLE ple_api_owner;

GRANT USAGE ON SCHEMA ple_api TO ple_assessment_attempt_expiry_worker;

SET LOCAL ROLE ple_private_owner;

GRANT USAGE ON SCHEMA ple_private TO ple_api_owner, ple_public_asset_publisher;

GRANT USAGE ON SCHEMA ple_private TO ple_api_owner, ple_public_asset_publisher;



-- The audit owner owns the receipt relation but its receipt is an exclusive
-- child of private Student Work.  It needs only the FK/trigger capability.
GRANT USAGE ON SCHEMA ple_private TO ple_audit_owner;

SET LOCAL ROLE ple_audit_owner;

GRANT USAGE ON SCHEMA ple_audit TO ple_api_owner;

GRANT USAGE ON SCHEMA ple_audit TO ple_private_owner;

SET LOCAL ROLE ple_data_owner;

GRANT USAGE ON SCHEMA ple_data TO ple_private_owner;

SET LOCAL ROLE ple_audit_owner;

GRANT USAGE ON SCHEMA ple_audit TO ple_private_owner;

SET LOCAL ROLE ple_data_owner;

-- Executor-owned, one-way Course inactivity and retention transitions. The preceding
-- course_retention.sql owns policy/due calculation; this late module owns the
-- exact destructive capability after every Course Student-Work child exists.
GRANT USAGE ON SCHEMA ple_data TO ple_course_retention_executor;

SET LOCAL ROLE ple_private_owner;

GRANT USAGE ON SCHEMA ple_private TO ple_course_retention_executor;

SET LOCAL ROLE ple_audit_owner;

GRANT USAGE ON SCHEMA ple_audit TO ple_course_retention_executor;

SET LOCAL ROLE ple_api_owner;

GRANT USAGE, CREATE ON SCHEMA ple_api TO ple_course_retention_executor;

SET LOCAL ROLE ple_private_owner;

REVOKE USAGE ON SCHEMA ple_private FROM ple_course_retention_notifier;

SET LOCAL ROLE ple_data_owner;







-- A dedicated non-login owner, not the notifier capability, owns the trusted
-- lookup/receipt operation.  The notifier receives only typed functions.
GRANT USAGE ON SCHEMA ple_data TO ple_course_retention_notification_owner;

REVOKE USAGE ON SCHEMA ple_data FROM ple_course_retention_notifier;

SET LOCAL ROLE ple_api_owner;





-- The dedicated notification owner owns the SECURITY DEFINER entry points.
-- It may create them during this migration only; the notifier has no table ACLs.
GRANT USAGE, CREATE ON SCHEMA ple_api TO ple_course_retention_notification_owner;

SET LOCAL ROLE ple_private_owner;

GRANT USAGE ON SCHEMA ple_private TO ple_course_retention_notification_owner;

SET LOCAL ROLE ple_api_owner;

GRANT USAGE ON SCHEMA ple_api TO ple_course_retention_notifier;

SET LOCAL ROLE ple_private_owner;

-- Forced Question Correction evidence.  A correction is a narrow, immutable
-- Sysadmin record for a critical exact Question Revision replacement.  It does
-- not create a Question Change Proposal or an alternate revision lifecycle.
GRANT USAGE ON SCHEMA ple_private TO ple_data_owner;

SET LOCAL ROLE ple_data_owner;

GRANT USAGE ON SCHEMA ple_data TO ple_audit_owner;

SET LOCAL ROLE ple_private_owner;

GRANT USAGE ON SCHEMA ple_private TO ple_audit_owner;

SET LOCAL ROLE ple_audit_owner;

GRANT USAGE ON SCHEMA ple_audit TO ple_unrelease_executor;

SET LOCAL ROLE ple_data_owner;





-- The executor has no login.  These narrowly scoped grants make its single
-- SECURITY DEFINER entry point capable of reading confirmation counts and
-- deleting only the Assessment Attempt root; dependent evidence follows the
-- owning FK cascades and remains unavailable to runtime roles.
GRANT USAGE ON SCHEMA ple_data TO ple_unrelease_executor;

SET LOCAL ROLE ple_private_owner;

GRANT USAGE ON SCHEMA ple_private TO ple_unrelease_executor;

SET LOCAL ROLE ple_api_owner;

GRANT USAGE, CREATE ON SCHEMA ple_api TO ple_unrelease_executor;

GRANT USAGE ON SCHEMA ple_api TO ple_app;


-- The coordinator verifies its own completed install through this deliberately
-- read-only projection.  It remains a separate capability from ple_app,
-- SQLx's ledger, and every API procedure (ASVS 8.2.1).
GRANT USAGE ON SCHEMA ple_api TO ple_migrator;

