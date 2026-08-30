-- SD1 global accounts and opaque primary sessions.
--
-- The role strings deliberately match question_model::UserRole's lower-camel
-- wire names.  PostgreSQL service roles such as ple_student are not human
-- product roles and are never stored in these columns.

DO $$
BEGIN
    IF current_user <> 'ple_migrator' THEN
        RAISE EXCEPTION USING
            ERRCODE = '42501',
            MESSAGE = 'migration 2026082902 must run as ple_migrator';
    END IF;

    IF pg_catalog.to_regnamespace('ple_private') IS NULL
       OR NOT pg_catalog.pg_has_role(
           'ple_migrator', 'ple_private_owner', 'SET'
       ) THEN
        RAISE EXCEPTION USING
            ERRCODE = '55000',
            MESSAGE = 'migration 2026082902 requires the 2026082901 private baseline';
    END IF;

    IF EXISTS (
        SELECT 1
          FROM pg_catalog.pg_class AS relations
          JOIN pg_catalog.pg_namespace AS namespaces
            ON namespaces.oid = relations.relnamespace
         WHERE namespaces.nspname = 'ple_private'
           AND relations.relname IN ('account', 'primary_session')
    ) THEN
        RAISE EXCEPTION USING
            ERRCODE = '42P07',
            MESSAGE = 'a reserved migration 2026082902 relation already exists';
    END IF;
END
$$;

SET LOCAL ROLE ple_private_owner;

-- ASVS 2.3.3: a global account has exactly one closed role for its lifetime.
CREATE TABLE ple_private.account (
    user_id uuid PRIMARY KEY,
    role text NOT NULL,
    provisioned_at timestamp with time zone NOT NULL,
    CONSTRAINT account_role_is_closed CHECK (
        role IN ('student', 'instructor', 'sysadmin')
    ),
    CONSTRAINT account_user_role_is_unique UNIQUE (user_id, role)
);

-- ASVS 2.3.3: a database trigger protects the invariant even if a future
-- privileged write path is accidentally granted UPDATE access.
CREATE FUNCTION ple_private.reject_account_identity_change()
RETURNS trigger
LANGUAGE plpgsql
SET search_path = pg_catalog, ple_private
AS $$
BEGIN
    IF NEW.user_id IS DISTINCT FROM OLD.user_id
       OR NEW.role IS DISTINCT FROM OLD.role
       OR NEW.provisioned_at IS DISTINCT FROM OLD.provisioned_at THEN
        RAISE EXCEPTION USING
            ERRCODE = '23514',
            MESSAGE = 'an account identity and role are immutable';
    END IF;
    RETURN NEW;
END
$$;

CREATE TRIGGER account_identity_is_immutable
BEFORE UPDATE ON ple_private.account
FOR EACH ROW
EXECUTE FUNCTION ple_private.reject_account_identity_change();

-- The opaque browser credential is never stored or selected.  Its fixed-size
-- digest and the composite foreign key bind every session to the account's
-- sole immutable role.  ASVS 2.3.3, 8.2.2, and 8.3.1.
CREATE TABLE ple_private.primary_session (
    session_id uuid PRIMARY KEY,
    user_id uuid NOT NULL,
    role text NOT NULL,
    token_hash bytea NOT NULL,
    created_at timestamp with time zone NOT NULL,
    expires_at timestamp with time zone NOT NULL,
    revoked_at timestamp with time zone,
    CONSTRAINT primary_session_account_role_matches
        FOREIGN KEY (user_id, role)
        REFERENCES ple_private.account (user_id, role),
    CONSTRAINT primary_session_role_is_closed CHECK (
        role IN ('student', 'instructor', 'sysadmin')
    ),
    CONSTRAINT primary_session_token_hash_is_sha256 CHECK (
        pg_catalog.octet_length(token_hash) = 32
    ),
    CONSTRAINT primary_session_expiry_is_after_creation CHECK (
        expires_at > created_at
    ),
    CONSTRAINT primary_session_revocation_is_after_creation CHECK (
        revoked_at IS NULL OR revoked_at >= created_at
    ),
    CONSTRAINT primary_session_token_hash_is_unique UNIQUE (token_hash)
);

CREATE INDEX primary_session_active_user_idx
    ON ple_private.primary_session (user_id, expires_at)
    WHERE revoked_at IS NULL;

-- A session may be revoked, but it cannot be repointed, recredentialed, or
-- assigned a different role after creation.  ASVS 2.3.3.
CREATE FUNCTION ple_private.reject_primary_session_identity_change()
RETURNS trigger
LANGUAGE plpgsql
SET search_path = pg_catalog, ple_private
AS $$
BEGIN
    IF NEW.session_id IS DISTINCT FROM OLD.session_id
       OR NEW.user_id IS DISTINCT FROM OLD.user_id
       OR NEW.role IS DISTINCT FROM OLD.role
       OR NEW.token_hash IS DISTINCT FROM OLD.token_hash
       OR NEW.created_at IS DISTINCT FROM OLD.created_at
       OR NEW.expires_at IS DISTINCT FROM OLD.expires_at THEN
        RAISE EXCEPTION USING
            ERRCODE = '23514',
            MESSAGE = 'a primary session identity and role are immutable';
    END IF;
    IF OLD.revoked_at IS NOT NULL
       AND NEW.revoked_at IS DISTINCT FROM OLD.revoked_at THEN
        RAISE EXCEPTION USING
            ERRCODE = '23514',
            MESSAGE = 'a revoked primary session cannot be changed or restored';
    END IF;
    RETURN NEW;
END
$$;

CREATE TRIGGER primary_session_identity_is_immutable
BEFORE UPDATE ON ple_private.primary_session
FOR EACH ROW
EXECUTE FUNCTION ple_private.reject_primary_session_identity_change();

ALTER TABLE ple_private.account ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.account FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_private.primary_session ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.primary_session FORCE ROW LEVEL SECURITY;

REVOKE ALL PRIVILEGES ON TABLE ple_private.account FROM PUBLIC;
REVOKE ALL PRIVILEGES ON TABLE ple_private.primary_session FROM PUBLIC;
REVOKE ALL PRIVILEGES ON FUNCTION ple_private.reject_account_identity_change()
    FROM PUBLIC;
REVOKE ALL PRIVILEGES ON FUNCTION ple_private.reject_primary_session_identity_change()
    FROM PUBLIC;

COMMENT ON TABLE ple_private.account IS
    'Global account identity and its one immutable human role; no email or credential data.';
COMMENT ON COLUMN ple_private.account.role IS
    'Closed lower-camel UserRole value: student, instructor, or sysadmin.';
COMMENT ON TABLE ple_private.primary_session IS
    'Server-only opaque-session digest, expiry, revocation, and account-role binding.';
COMMENT ON COLUMN ple_private.primary_session.token_hash IS
    'Fixed-size digest of an opaque browser credential; raw credentials are never stored.';

RESET ROLE;
