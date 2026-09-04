-- Global Account and opaque Authenticated Session foundation.
--
-- The role strings deliberately match question_model::ProductRole's lower-camel
-- wire names.  PostgreSQL service roles such as ple_student are not human
-- Product Roles and are never stored in these columns.

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
           AND relations.relname IN ('account', 'authenticated_session')
    ) THEN
        RAISE EXCEPTION USING
            ERRCODE = '42P07',
            MESSAGE = 'a reserved migration 2026082902 relation already exists';
    END IF;
END
$$;

SET LOCAL ROLE ple_private_owner;

-- ASVS 2.3.3: a global account has exactly one closed Product Role for its lifetime.
CREATE TABLE ple_private.account (
    account_id uuid PRIMARY KEY,
    product_role text NOT NULL,
    created_at timestamp with time zone NOT NULL,
    CONSTRAINT account_product_role_is_closed CHECK (
        product_role IN ('student', 'instructor', 'sysadmin')
    ),
    CONSTRAINT account_product_role_is_unique UNIQUE (account_id, product_role)
);

-- ASVS 2.3.3: a database trigger protects the invariant even if a future
-- privileged write path is accidentally granted UPDATE access.
CREATE FUNCTION ple_private.reject_account_identity_change()
RETURNS trigger
LANGUAGE plpgsql
SET search_path = pg_catalog, ple_private
AS $$
BEGIN
    IF NEW.account_id IS DISTINCT FROM OLD.account_id
       OR NEW.product_role IS DISTINCT FROM OLD.product_role
       OR NEW.created_at IS DISTINCT FROM OLD.created_at THEN
        RAISE EXCEPTION USING
            ERRCODE = '23514',
            MESSAGE = 'an account identity and Product Role are immutable';
    END IF;
    RETURN NEW;
END
$$;

CREATE TRIGGER account_identity_is_immutable
BEFORE UPDATE ON ple_private.account
FOR EACH ROW
EXECUTE FUNCTION ple_private.reject_account_identity_change();

-- ASVS 8.2.2 and 8.3.1: Account State is append-only and independently
-- governs authentication; it never rewrites the immutable Product Role.
CREATE TABLE ple_private.account_state_event (
    event_id uuid PRIMARY KEY,
    account_id uuid NOT NULL REFERENCES ple_private.account (account_id),
    state text NOT NULL CHECK (state IN ('active', 'deactivated', 'closed')),
    occurred_at timestamp with time zone NOT NULL,
    reason text,
    CONSTRAINT account_state_event_reason_is_present_for_nonactive_state CHECK (
        state = 'active' OR char_length(btrim(reason)) BETWEEN 1 AND 1000
    )
);
CREATE FUNCTION ple_private.record_initial_account_state()
RETURNS trigger LANGUAGE plpgsql SET search_path = pg_catalog, ple_private AS $$
BEGIN
    INSERT INTO ple_private.account_state_event (event_id, account_id, state, occurred_at)
    VALUES (NEW.account_id, NEW.account_id, 'active', NEW.created_at);
    RETURN NEW;
END
$$;
CREATE TRIGGER account_creation_records_active_state
AFTER INSERT ON ple_private.account
FOR EACH ROW EXECUTE FUNCTION ple_private.record_initial_account_state();

-- The opaque browser credential is never stored or selected.  Its fixed-size
-- Session Token Hash and the composite foreign key bind every session to the account's
-- sole immutable Product Role.  ASVS 2.3.3, 8.2.2, and 8.3.1.
CREATE TABLE ple_private.authenticated_session (
    session_id uuid PRIMARY KEY,
    account_id uuid NOT NULL,
    product_role text NOT NULL,
    token_hash bytea NOT NULL,
    created_at timestamp with time zone NOT NULL,
    expires_at timestamp with time zone NOT NULL,
    revoked_at timestamp with time zone,
    CONSTRAINT authenticated_session_account_product_role_matches
        FOREIGN KEY (account_id, product_role)
        REFERENCES ple_private.account (account_id, product_role),
    CONSTRAINT authenticated_session_product_role_is_closed CHECK (
        product_role IN ('student', 'instructor', 'sysadmin')
    ),
    CONSTRAINT authenticated_session_token_hash_is_sha256 CHECK (
        pg_catalog.octet_length(token_hash) = 32
    ),
    CONSTRAINT authenticated_session_expiry_is_after_creation CHECK (
        expires_at > created_at
    ),
    CONSTRAINT authenticated_session_revocation_is_after_creation CHECK (
        revoked_at IS NULL OR revoked_at >= created_at
    ),
    CONSTRAINT authenticated_session_token_hash_is_unique UNIQUE (token_hash)
);

CREATE INDEX authenticated_session_active_account_idx
    ON ple_private.authenticated_session (account_id, expires_at)
    WHERE revoked_at IS NULL;

-- A session may be revoked, but it cannot be repointed, recredentialed, or
-- assigned a different Product Role after creation.  ASVS 2.3.3.
CREATE FUNCTION ple_private.reject_authenticated_session_identity_change()
RETURNS trigger
LANGUAGE plpgsql
SET search_path = pg_catalog, ple_private
AS $$
BEGIN
    IF NEW.session_id IS DISTINCT FROM OLD.session_id
       OR NEW.account_id IS DISTINCT FROM OLD.account_id
       OR NEW.product_role IS DISTINCT FROM OLD.product_role
       OR NEW.token_hash IS DISTINCT FROM OLD.token_hash
       OR NEW.created_at IS DISTINCT FROM OLD.created_at
       OR NEW.expires_at IS DISTINCT FROM OLD.expires_at THEN
        RAISE EXCEPTION USING
            ERRCODE = '23514',
            MESSAGE = 'an authenticated session identity and Product Role are immutable';
    END IF;
    IF OLD.revoked_at IS NOT NULL
       AND NEW.revoked_at IS DISTINCT FROM OLD.revoked_at THEN
        RAISE EXCEPTION USING
            ERRCODE = '23514',
            MESSAGE = 'a revoked authenticated session cannot be changed or restored';
    END IF;
    RETURN NEW;
END
$$;

CREATE TRIGGER authenticated_session_identity_is_immutable
BEFORE UPDATE ON ple_private.authenticated_session
FOR EACH ROW
EXECUTE FUNCTION ple_private.reject_authenticated_session_identity_change();

CREATE FUNCTION ple_private.revoke_sessions_after_account_deactivation_or_closure()
RETURNS trigger LANGUAGE plpgsql SET search_path = pg_catalog, ple_private AS $$
BEGIN
    IF NEW.state IN ('deactivated', 'closed') THEN
        UPDATE ple_private.authenticated_session
           SET revoked_at = NEW.occurred_at
         WHERE account_id = NEW.account_id AND revoked_at IS NULL;
    END IF;
    RETURN NEW;
END
$$;
CREATE TRIGGER account_deactivation_or_closure_revokes_sessions
AFTER INSERT ON ple_private.account_state_event
FOR EACH ROW EXECUTE FUNCTION ple_private.revoke_sessions_after_account_deactivation_or_closure();

ALTER TABLE ple_private.account ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.account FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_private.authenticated_session ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.authenticated_session FORCE ROW LEVEL SECURITY;

REVOKE ALL PRIVILEGES ON TABLE ple_private.account FROM PUBLIC;
REVOKE ALL PRIVILEGES ON TABLE ple_private.authenticated_session FROM PUBLIC;
REVOKE ALL PRIVILEGES ON FUNCTION ple_private.reject_account_identity_change()
    FROM PUBLIC;
REVOKE ALL PRIVILEGES ON FUNCTION ple_private.reject_authenticated_session_identity_change()
    FROM PUBLIC;

COMMENT ON TABLE ple_private.account IS
    'Global Account identity and its one immutable human Product Role; no email or credential data.';
COMMENT ON COLUMN ple_private.account.product_role IS
    'Closed lower-camel ProductRole value: student, instructor, or sysadmin.';
COMMENT ON TABLE ple_private.authenticated_session IS
    'Server-only Session Token Hash, expiry, revocation, and Account Product Role binding.';
COMMENT ON COLUMN ple_private.authenticated_session.token_hash IS
    'Fixed-size Session Token Hash for an opaque browser credential; raw credentials are never stored.';

RESET ROLE;
