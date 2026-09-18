-- authentication tables. CREATE TABLE and COMMENT ON only.
-- Functions, policies, and privileges live in later layers.

SET LOCAL ROLE ple_private_owner;

-- Passwordless email and passkey authentication plus opaque sessions.
CREATE TABLE ple_private.account_authentication_email (
    account_id uuid PRIMARY KEY REFERENCES ple_private.account (account_id),
    normalized_email text NOT NULL UNIQUE,
    delivery_email text NOT NULL,
    verified_at timestamp with time zone NOT NULL,
    updated_at timestamp with time zone NOT NULL,
    CHECK (char_length(normalized_email) BETWEEN 3 AND 320
           AND normalized_email = lower(btrim(normalized_email))),
    CHECK (char_length(btrim(delivery_email)) BETWEEN 3 AND 320),
    CHECK (updated_at >= verified_at)
);

CREATE TABLE ple_private.authentication_rate_limit (
    scope ple_data.auth_rate_scope NOT NULL,
    key_hash bytea NOT NULL CHECK (pg_catalog.octet_length(key_hash) = 32),
    window_started_at timestamp with time zone NOT NULL,
    consumed_attempts integer NOT NULL CHECK (consumed_attempts BETWEEN 1 AND 10000),
    PRIMARY KEY (scope, key_hash, window_started_at),
    updated_at timestamptz NOT NULL DEFAULT pg_catalog.transaction_timestamp()
);



CREATE TABLE ple_private.email_authentication_challenge (
    challenge_id uuid PRIMARY KEY,
    token_hash bytea NOT NULL UNIQUE CHECK (pg_catalog.octet_length(token_hash) = 32),
    browser_binding_hash bytea NOT NULL CHECK (pg_catalog.octet_length(browser_binding_hash) = 32),
    email_rate_limit_key_hash bytea NOT NULL CHECK (pg_catalog.octet_length(email_rate_limit_key_hash) = 32),
    email text NOT NULL CHECK (char_length(email) BETWEEN 3 AND 320 AND email = lower(btrim(email))),
    purpose ple_data.email_challenge_purpose NOT NULL,
    target_account_id uuid NOT NULL REFERENCES ple_private.account (account_id),
    created_at timestamp with time zone NOT NULL,
    expires_at timestamp with time zone NOT NULL,
    consumed_at timestamp with time zone,
    CHECK (expires_at > created_at AND expires_at <= created_at + interval '10 minutes'),
    CHECK (consumed_at IS NULL OR consumed_at >= created_at)
);


CREATE TABLE ple_private.passkey_ceremony (
    ceremony_id uuid PRIMARY KEY,
    kind ple_data.passkey_ceremony_kind NOT NULL,
    target_account_id uuid REFERENCES ple_private.account (account_id),
    browser_binding_hash bytea NOT NULL CHECK (pg_catalog.octet_length(browser_binding_hash) = 32),
    state bytea NOT NULL CHECK (pg_catalog.octet_length(state) > 0),
    created_at timestamp with time zone NOT NULL,
    expires_at timestamp with time zone NOT NULL,
    consumed_at timestamp with time zone,
    CHECK ((kind = 'registration' AND target_account_id IS NOT NULL) OR kind = 'authentication'),
    CHECK (expires_at > created_at AND expires_at <= created_at + interval '10 minutes'),
    CHECK (consumed_at IS NULL OR consumed_at >= created_at)
);


CREATE TABLE ple_private.passkey (
    passkey_id uuid PRIMARY KEY,
    account_id uuid NOT NULL REFERENCES ple_private.account (account_id),
    credential_id_hash bytea NOT NULL UNIQUE CHECK (pg_catalog.octet_length(credential_id_hash) = 32),
    label text NOT NULL CHECK (char_length(btrim(label)) BETWEEN 1 AND 200),
    credential_state bytea NOT NULL CHECK (pg_catalog.octet_length(credential_state) > 0),
    created_at timestamp with time zone NOT NULL,
    last_used_at timestamp with time zone,
    revoked_at timestamp with time zone,
    CHECK (last_used_at IS NULL OR last_used_at >= created_at),
    CHECK (revoked_at IS NULL OR revoked_at >= created_at),
    updated_at timestamptz NOT NULL DEFAULT pg_catalog.transaction_timestamp(),
    CHECK (updated_at >= created_at)
);




-- Sysadmin TOTP is a second factor, never a selector or demo-only shortcut.
-- The encrypted seed is AEAD-wrapped by the server-held key ring before it
-- reaches this table; plaintext seed material has no database column.
CREATE TABLE ple_private.sysadmin_totp_credential (
    account_id uuid PRIMARY KEY REFERENCES ple_private.account (account_id),
    encryption_key_id text NOT NULL CHECK (char_length(encryption_key_id) BETWEEN 1 AND 128),
    seed_nonce bytea NOT NULL CHECK (pg_catalog.octet_length(seed_nonce) = 24),
    encrypted_seed bytea NOT NULL CHECK (pg_catalog.octet_length(encrypted_seed) BETWEEN 36 AND 80),
    provisioned_at timestamp with time zone NOT NULL,
    updated_at timestamptz NOT NULL DEFAULT pg_catalog.transaction_timestamp()
);


CREATE TABLE ple_private.sysadmin_totp_attestation (
    sysadmin_totp_attestation_id uuid PRIMARY KEY,
    account_id uuid NOT NULL REFERENCES ple_private.account (account_id),
    browser_binding_hash bytea NOT NULL CHECK (pg_catalog.octet_length(browser_binding_hash) = 32),
    created_at timestamp with time zone NOT NULL,
    expires_at timestamp with time zone NOT NULL,
    consumed_at timestamp with time zone,
    CHECK (expires_at > created_at AND expires_at <= created_at + interval '10 minutes'),
    CHECK (consumed_at IS NULL OR consumed_at >= created_at)
);



-- An Account may use a verified 30-second TOTP counter once only.  It is
-- intentionally retained after the pending attestation expires so replay is
-- impossible across browser-bound ceremonies (ASVS 6.5.1).
CREATE TABLE ple_private.sysadmin_totp_used_counter (
    account_id uuid NOT NULL REFERENCES ple_private.account (account_id),
    totp_counter bigint NOT NULL CHECK (totp_counter >= 0),
    used_at timestamp with time zone NOT NULL,
    PRIMARY KEY (account_id, totp_counter)
);



-- A reservation exists only after PostgreSQL has resolved a live, matching
-- pending attestation. It therefore cannot become an attacker-controlled UUID
-- map. Five charged attempts in one ten-minute window lock that attestation
-- through its own expiry; these operational bounds remain implementation
-- details, not a browser or product contract.
CREATE TABLE ple_private.sysadmin_totp_verification_attempt (
    sysadmin_totp_attestation_id uuid PRIMARY KEY REFERENCES ple_private.sysadmin_totp_attestation (sysadmin_totp_attestation_id),
    account_id uuid NOT NULL REFERENCES ple_private.account (account_id),
    browser_binding_hash bytea NOT NULL CHECK (pg_catalog.octet_length(browser_binding_hash) = 32),
    window_started_at timestamp with time zone NOT NULL,
    charged_attempt_count integer NOT NULL CHECK (charged_attempt_count BETWEEN 1 AND 5),
    locked_until timestamp with time zone
);

CREATE TABLE ple_private.authenticated_session (
    session_id uuid PRIMARY KEY,
    account_id uuid NOT NULL REFERENCES ple_private.account (account_id),
    product_role ple_data.product_role NOT NULL,
    token_hash bytea NOT NULL UNIQUE CHECK (pg_catalog.octet_length(token_hash) = 32),
    created_at timestamp with time zone NOT NULL,
    expires_at timestamp with time zone NOT NULL,
    revoked_at timestamp with time zone,
    CHECK (expires_at > created_at),
    CHECK (revoked_at IS NULL OR revoked_at >= created_at),
    FOREIGN KEY (account_id, product_role) REFERENCES ple_private.account (account_id, product_role),
    updated_at timestamptz NOT NULL DEFAULT pg_catalog.transaction_timestamp(),
    CHECK (updated_at >= created_at)
);


SET LOCAL ROLE ple_private_owner;
COMMENT ON TABLE ple_private.account_authentication_email IS 'role: current state, deleted by Account closure and the authentication retention sweep. HUMAN_GUIDANCE.md Authentication.';

COMMENT ON TABLE ple_private.authentication_rate_limit IS 'role: current state, deleted by Account closure and the authentication retention sweep. HUMAN_GUIDANCE.md Authentication.';

COMMENT ON TABLE ple_private.email_authentication_challenge IS 'role: event, deleted by Account closure and the authentication retention sweep. HUMAN_GUIDANCE.md Authentication.';

COMMENT ON TABLE ple_private.passkey_ceremony IS 'role: event, deleted by Account closure and the authentication retention sweep. HUMAN_GUIDANCE.md Authentication.';

COMMENT ON TABLE ple_private.passkey IS 'role: current state, deleted by Account closure and the authentication retention sweep. HUMAN_GUIDANCE.md Authentication.';

COMMENT ON TABLE ple_private.sysadmin_totp_credential IS 'role: current state, deleted by Account closure and the authentication retention sweep. HUMAN_GUIDANCE.md Authentication.';

COMMENT ON TABLE ple_private.sysadmin_totp_attestation IS 'role: event, deleted by Account closure and the authentication retention sweep. HUMAN_GUIDANCE.md Authentication.';

COMMENT ON TABLE ple_private.sysadmin_totp_used_counter IS 'role: event, deleted by Account closure and the authentication retention sweep. HUMAN_GUIDANCE.md Authentication.';

COMMENT ON TABLE ple_private.sysadmin_totp_verification_attempt IS 'role: event, deleted by Account closure and the authentication retention sweep. HUMAN_GUIDANCE.md Authentication.';

COMMENT ON TABLE ple_private.authenticated_session IS 'role: current state, deleted by Account closure and the authentication retention sweep. HUMAN_GUIDANCE.md Authentication.';

COMMENT ON TABLE ple_private.account_authentication_email IS 'role: current state, deleted by Account closure and the authentication retention sweep. HUMAN_GUIDANCE.md Authentication.';

COMMENT ON TABLE ple_private.authentication_rate_limit IS 'role: current state, deleted by Account closure and the authentication retention sweep. HUMAN_GUIDANCE.md Authentication.';

COMMENT ON TABLE ple_private.email_authentication_challenge IS 'role: event, deleted by Account closure and the authentication retention sweep. HUMAN_GUIDANCE.md Authentication.';

COMMENT ON TABLE ple_private.passkey_ceremony IS 'role: event, deleted by Account closure and the authentication retention sweep. HUMAN_GUIDANCE.md Authentication.';

COMMENT ON TABLE ple_private.passkey IS 'role: current state, deleted by Account closure and the authentication retention sweep. HUMAN_GUIDANCE.md Authentication.';

COMMENT ON TABLE ple_private.sysadmin_totp_credential IS 'role: current state, deleted by Account closure and the authentication retention sweep. HUMAN_GUIDANCE.md Authentication.';

COMMENT ON TABLE ple_private.sysadmin_totp_attestation IS 'role: event, deleted by Account closure and the authentication retention sweep. HUMAN_GUIDANCE.md Authentication.';

COMMENT ON TABLE ple_private.sysadmin_totp_used_counter IS 'role: event, deleted by Account closure and the authentication retention sweep. HUMAN_GUIDANCE.md Authentication.';

COMMENT ON TABLE ple_private.sysadmin_totp_verification_attempt IS 'role: event, deleted by Account closure and the authentication retention sweep. HUMAN_GUIDANCE.md Authentication.';

COMMENT ON TABLE ple_private.authenticated_session IS 'role: current state, deleted by Account closure and the authentication retention sweep. HUMAN_GUIDANCE.md Authentication.';



COMMENT ON TABLE ple_private.account_authentication_email IS 'role: current state, deleted by Account closure and the authentication retention sweep. HUMAN_GUIDANCE.md Authentication.';

COMMENT ON TABLE ple_private.authentication_rate_limit IS 'role: current state, deleted by Account closure and the authentication retention sweep. HUMAN_GUIDANCE.md Authentication.';

COMMENT ON TABLE ple_private.email_authentication_challenge IS 'role: event, deleted by Account closure and the authentication retention sweep. HUMAN_GUIDANCE.md Authentication.';

COMMENT ON TABLE ple_private.passkey_ceremony IS 'role: event, deleted by Account closure and the authentication retention sweep. HUMAN_GUIDANCE.md Authentication.';

COMMENT ON TABLE ple_private.passkey IS 'role: current state, deleted by Account closure and the authentication retention sweep. HUMAN_GUIDANCE.md Authentication.';

COMMENT ON TABLE ple_private.sysadmin_totp_credential IS 'role: current state, deleted by Account closure and the authentication retention sweep. HUMAN_GUIDANCE.md Authentication.';

COMMENT ON TABLE ple_private.sysadmin_totp_attestation IS 'role: event, deleted by Account closure and the authentication retention sweep. HUMAN_GUIDANCE.md Authentication.';

COMMENT ON TABLE ple_private.sysadmin_totp_used_counter IS 'role: event, deleted by Account closure and the authentication retention sweep. HUMAN_GUIDANCE.md Authentication.';

COMMENT ON TABLE ple_private.sysadmin_totp_verification_attempt IS 'role: event, deleted by Account closure and the authentication retention sweep. HUMAN_GUIDANCE.md Authentication.';

COMMENT ON TABLE ple_private.authenticated_session IS 'role: current state, deleted by Account closure and the authentication retention sweep. HUMAN_GUIDANCE.md Authentication.';



COMMENT ON TABLE ple_private.account_authentication_email IS 'role: current state, deleted by Account closure and the authentication retention sweep. HUMAN_GUIDANCE.md Authentication.';

COMMENT ON TABLE ple_private.authentication_rate_limit IS 'role: current state, deleted by Account closure and the authentication retention sweep. HUMAN_GUIDANCE.md Authentication.';

COMMENT ON TABLE ple_private.email_authentication_challenge IS 'role: event, deleted by Account closure and the authentication retention sweep. HUMAN_GUIDANCE.md Authentication.';

COMMENT ON TABLE ple_private.passkey_ceremony IS 'role: event, deleted by Account closure and the authentication retention sweep. HUMAN_GUIDANCE.md Authentication.';

COMMENT ON TABLE ple_private.passkey IS 'role: current state, deleted by Account closure and the authentication retention sweep. HUMAN_GUIDANCE.md Authentication.';

COMMENT ON TABLE ple_private.sysadmin_totp_credential IS 'role: current state, deleted by Account closure and the authentication retention sweep. HUMAN_GUIDANCE.md Authentication.';

COMMENT ON TABLE ple_private.sysadmin_totp_attestation IS 'role: event, deleted by Account closure and the authentication retention sweep. HUMAN_GUIDANCE.md Authentication.';

COMMENT ON TABLE ple_private.sysadmin_totp_used_counter IS 'role: event, deleted by Account closure and the authentication retention sweep. HUMAN_GUIDANCE.md Authentication.';

COMMENT ON TABLE ple_private.sysadmin_totp_verification_attempt IS 'role: event, deleted by Account closure and the authentication retention sweep. HUMAN_GUIDANCE.md Authentication.';

COMMENT ON TABLE ple_private.authenticated_session IS 'role: current state, deleted by Account closure and the authentication retention sweep. HUMAN_GUIDANCE.md Authentication.';

COMMENT ON TABLE ple_private.account_authentication_email IS 'role: current state, deleted by Account closure and the authentication retention sweep. HUMAN_GUIDANCE.md Authentication.';

COMMENT ON TABLE ple_private.authentication_rate_limit IS 'role: current state, deleted by Account closure and the authentication retention sweep. HUMAN_GUIDANCE.md Authentication.';

COMMENT ON TABLE ple_private.email_authentication_challenge IS 'role: event, deleted by Account closure and the authentication retention sweep. HUMAN_GUIDANCE.md Authentication.';

COMMENT ON TABLE ple_private.passkey_ceremony IS 'role: event, deleted by Account closure and the authentication retention sweep. HUMAN_GUIDANCE.md Authentication.';

COMMENT ON TABLE ple_private.passkey IS 'role: current state, deleted by Account closure and the authentication retention sweep. HUMAN_GUIDANCE.md Authentication.';

COMMENT ON TABLE ple_private.sysadmin_totp_credential IS 'role: current state, deleted by Account closure and the authentication retention sweep. HUMAN_GUIDANCE.md Authentication.';

COMMENT ON TABLE ple_private.sysadmin_totp_attestation IS 'role: event, deleted by Account closure and the authentication retention sweep. HUMAN_GUIDANCE.md Authentication.';

COMMENT ON TABLE ple_private.sysadmin_totp_used_counter IS 'role: event, deleted by Account closure and the authentication retention sweep. HUMAN_GUIDANCE.md Authentication.';

COMMENT ON TABLE ple_private.sysadmin_totp_verification_attempt IS 'role: event, deleted by Account closure and the authentication retention sweep. HUMAN_GUIDANCE.md Authentication.';

COMMENT ON TABLE ple_private.authenticated_session IS 'role: current state, deleted by Account closure and the authentication retention sweep. HUMAN_GUIDANCE.md Authentication.';



COMMENT ON TABLE ple_private.account_authentication_email IS 'role: current state, deleted by Account closure and the authentication retention sweep. HUMAN_GUIDANCE.md Authentication.';

COMMENT ON TABLE ple_private.authentication_rate_limit IS 'role: current state, deleted by Account closure and the authentication retention sweep. HUMAN_GUIDANCE.md Authentication.';

COMMENT ON TABLE ple_private.email_authentication_challenge IS 'role: event, deleted by Account closure and the authentication retention sweep. HUMAN_GUIDANCE.md Authentication.';

COMMENT ON TABLE ple_private.passkey_ceremony IS 'role: event, deleted by Account closure and the authentication retention sweep. HUMAN_GUIDANCE.md Authentication.';

COMMENT ON TABLE ple_private.passkey IS 'role: current state, deleted by Account closure and the authentication retention sweep. HUMAN_GUIDANCE.md Authentication.';

COMMENT ON TABLE ple_private.sysadmin_totp_credential IS 'role: current state, deleted by Account closure and the authentication retention sweep. HUMAN_GUIDANCE.md Authentication.';

COMMENT ON TABLE ple_private.sysadmin_totp_attestation IS 'role: event, deleted by Account closure and the authentication retention sweep. HUMAN_GUIDANCE.md Authentication.';

COMMENT ON TABLE ple_private.sysadmin_totp_used_counter IS 'role: event, deleted by Account closure and the authentication retention sweep. HUMAN_GUIDANCE.md Authentication.';

COMMENT ON TABLE ple_private.sysadmin_totp_verification_attempt IS 'role: event, deleted by Account closure and the authentication retention sweep. HUMAN_GUIDANCE.md Authentication.';

COMMENT ON TABLE ple_private.authenticated_session IS 'role: current state, deleted by Account closure and the authentication retention sweep. HUMAN_GUIDANCE.md Authentication.';

