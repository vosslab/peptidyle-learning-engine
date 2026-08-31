-- SD1 passwordless email challenge and authentication-rate-limit roots.

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

-- An Authentication Email is private mutable credential data.  It identifies
-- the Account that a completed email ceremony may authenticate, but it is not
-- an Account identity, role grant, course relationship, or browser DTO.
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

-- Email is private ceremony state, never a catalog, course, or browser DTO.
-- Every challenge authenticates one existing Account. Account Creation is
-- a distinct Sysadmin-owned workflow. A completed challenge remains as a
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

COMMENT ON TABLE ple_private.account_authentication_email IS
    'Private verified mutable email credential for one existing global Account; never an authorization grant or browser DTO.';
COMMENT ON TABLE ple_private.email_authentication_challenge IS
    'Private, browser-bound, single-use passwordless email ceremony state; raw code is never stored.';
COMMENT ON TABLE ple_private.authentication_rate_limit IS
    'Private fixed-window authentication quotas keyed only by non-reversible server-derived hashes.';

RESET ROLE;
