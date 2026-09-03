-- SD1 role-qualified Authentication Email, passwordless challenge, and rate-limit roots.

DO $$
BEGIN
    IF current_user <> 'ple_migrator'
       OR NOT pg_catalog.pg_has_role('ple_migrator', 'ple_private_owner', 'SET') THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'migration 2026082903 requires the SD1 private migration principal';
    END IF;
    IF EXISTS (
        SELECT 1 FROM pg_catalog.pg_class AS relations
        JOIN pg_catalog.pg_namespace AS namespaces ON namespaces.oid = relations.relnamespace
        WHERE namespaces.nspname = 'ple_private'
          AND relations.relname IN (
              'account_authentication_email',
              'email_authentication_challenge',
              'authentication_rate_limit'
          )
    ) THEN
        RAISE EXCEPTION USING ERRCODE = '42P07',
            MESSAGE = 'a reserved migration 2026082903 relation already exists';
    END IF;
END
$$;

SET LOCAL ROLE ple_private_owner;

-- An Authentication Email is private credential data for one Account. Student
-- bindings are immutable, Instructor replacement requires a later verified
-- operation, and Sysadmin binding has no current writer; it is never an
-- Account identity, role grant, course relationship, or browser DTO.
CREATE TABLE ple_private.account_authentication_email (
    account_id uuid PRIMARY KEY REFERENCES ple_private.account (account_id),
    normalized_email text NOT NULL UNIQUE,
    delivery_email text NOT NULL,
    verified_at timestamp with time zone NOT NULL,
    updated_at timestamp with time zone NOT NULL,
    CONSTRAINT account_authentication_email_normalized_shape CHECK (
        char_length(normalized_email) BETWEEN 3 AND 320
        AND normalized_email = lower(btrim(normalized_email))
    ),
    CONSTRAINT account_authentication_email_delivery_shape CHECK (
        char_length(btrim(delivery_email)) BETWEEN 3 AND 320
    ),
    CONSTRAINT account_authentication_email_update_is_ordered CHECK (
        updated_at >= verified_at
    )
);

-- An Authentication Email has role-derived integrity rules. Student bindings
-- are immutable after creation, Instructor replacement awaits its dedicated
-- verified operation, and Sysadmin bindings have no current write path.
CREATE FUNCTION ple_private.enforce_account_authentication_email_role()
RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_private
AS $$
DECLARE
    v_new_role text;
    v_old_role text;
BEGIN
    SELECT account.role INTO v_new_role
      FROM ple_private.account AS account
     WHERE account.account_id = NEW.account_id;
    IF v_new_role IS NULL THEN
        RAISE EXCEPTION USING ERRCODE = '23503',
            MESSAGE = 'Authentication Email requires an existing Account';
    END IF;
    IF v_new_role NOT IN ('student', 'instructor') THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Authentication Email requires a supported Account role';
    END IF;
    IF TG_OP = 'UPDATE' THEN
        IF NEW.account_id IS DISTINCT FROM OLD.account_id THEN
            RAISE EXCEPTION USING ERRCODE = '23514',
                MESSAGE = 'Authentication Email Account binding is immutable';
        END IF;
        SELECT account.role INTO v_old_role
          FROM ple_private.account AS account
         WHERE account.account_id = OLD.account_id;
        IF v_old_role IS DISTINCT FROM 'instructor'
           OR v_new_role IS DISTINCT FROM 'instructor' THEN
            RAISE EXCEPTION USING ERRCODE = '23514',
                MESSAGE = 'Student Authentication Email is immutable';
        END IF;
    END IF;
    RETURN NEW;
END
$$;

CREATE TRIGGER account_authentication_email_role_is_enforced
BEFORE INSERT OR UPDATE ON ple_private.account_authentication_email
FOR EACH ROW
EXECUTE FUNCTION ple_private.enforce_account_authentication_email_role();

-- Only non-reversible server-derived quota keys are retained.  The mutable
-- counter is scoped to an explicit fixed window and never identifies a person.
CREATE TABLE ple_private.authentication_rate_limit (
    scope text NOT NULL,
    key_hash bytea NOT NULL,
    window_started_at timestamp with time zone NOT NULL,
    consumed_attempts integer NOT NULL,
    CONSTRAINT authentication_rate_limit_scope_is_closed CHECK (
        scope IN ('email', 'network', 'principal', 'service')
    ),
    CONSTRAINT authentication_rate_limit_key_is_sha256 CHECK (
        pg_catalog.octet_length(key_hash) = 32
    ),
    CONSTRAINT authentication_rate_limit_attempts_are_positive CHECK (
        consumed_attempts > 0 AND consumed_attempts <= 10000
    ),
    PRIMARY KEY (scope, key_hash, window_started_at)
);

-- Email is private ceremony state, never a Question Library, course, or browser DTO.
-- Every challenge authenticates one existing Account. Create Instructor Account
-- is a distinct Active Sysadmin-owned workflow. A completed challenge remains as a
-- minimal single-use receipt until the future retention owner deletes it;
-- neither raw code nor password is stored.
CREATE TABLE ple_private.email_authentication_challenge (
    challenge_id uuid PRIMARY KEY,
    token_hash bytea NOT NULL UNIQUE,
    browser_binding_hash bytea NOT NULL,
    email_rate_limit_key_hash bytea NOT NULL,
    email text NOT NULL,
    purpose text NOT NULL,
    target_account_id uuid NOT NULL,
    created_at timestamp with time zone NOT NULL,
    expires_at timestamp with time zone NOT NULL,
    consumed_at timestamp with time zone,
    CONSTRAINT email_challenge_target_account_exists
        FOREIGN KEY (target_account_id) REFERENCES ple_private.account (account_id),
    CONSTRAINT email_challenge_token_hash_is_sha256 CHECK (pg_catalog.octet_length(token_hash) = 32),
    CONSTRAINT email_challenge_browser_binding_is_sha256 CHECK (pg_catalog.octet_length(browser_binding_hash) = 32),
    CONSTRAINT email_challenge_rate_limit_key_is_sha256 CHECK (pg_catalog.octet_length(email_rate_limit_key_hash) = 32),
    CONSTRAINT email_challenge_email_is_bounded CHECK (
        char_length(email) BETWEEN 3 AND 320 AND email = lower(btrim(email))
    ),
    CONSTRAINT email_challenge_purpose_is_closed CHECK (
        purpose IN ('sign_in', 'change_email')
    ),
    CONSTRAINT email_challenge_lifetime_is_bounded CHECK (
        expires_at > created_at AND expires_at <= created_at + interval '10 minutes'
    ),
    CONSTRAINT email_challenge_consumption_is_ordered CHECK (
        consumed_at IS NULL OR consumed_at >= created_at
    )
);

CREATE INDEX email_authentication_challenge_active_token_idx
    ON ple_private.email_authentication_challenge (token_hash, expires_at)
    WHERE consumed_at IS NULL;

ALTER TABLE ple_private.authentication_rate_limit ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.authentication_rate_limit FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_private.account_authentication_email ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.account_authentication_email FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_private.email_authentication_challenge ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.email_authentication_challenge FORCE ROW LEVEL SECURITY;
REVOKE ALL PRIVILEGES ON TABLE ple_private.authentication_rate_limit FROM PUBLIC;
REVOKE ALL PRIVILEGES ON TABLE ple_private.account_authentication_email FROM PUBLIC;
REVOKE ALL PRIVILEGES ON TABLE ple_private.email_authentication_challenge FROM PUBLIC;
REVOKE ALL PRIVILEGES ON FUNCTION ple_private.enforce_account_authentication_email_role()
    FROM PUBLIC;

COMMENT ON TABLE ple_private.account_authentication_email IS
    'Private role-qualified Authentication Email for one existing global Account; never an authorization grant or browser DTO.';
COMMENT ON TABLE ple_private.email_authentication_challenge IS
    'Private, browser-bound, single-use passwordless email ceremony state; raw code is never stored.';
COMMENT ON TABLE ple_private.authentication_rate_limit IS
    'Private fixed-window authentication quotas keyed only by non-reversible server-derived hashes.';

RESET ROLE;
