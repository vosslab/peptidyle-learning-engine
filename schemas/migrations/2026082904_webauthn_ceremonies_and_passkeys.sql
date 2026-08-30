-- SD1 private WebAuthn ceremony and passkey roots; PLE stores no passwords.

DO $$
BEGIN
    IF current_user <> 'ple_migrator'
       OR NOT pg_catalog.pg_has_role('ple_migrator', 'ple_private_owner', 'SET') THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'migration 2026082904 requires the SD1 private migration principal';
    END IF;
    IF EXISTS (
        SELECT 1 FROM pg_catalog.pg_class AS relations
        JOIN pg_catalog.pg_namespace AS namespaces ON namespaces.oid = relations.relnamespace
        WHERE namespaces.nspname = 'ple_private'
          AND relations.relname IN ('webauthn_ceremony', 'passkey')
    ) THEN
        RAISE EXCEPTION USING ERRCODE = '42P07',
            MESSAGE = 'a reserved migration 2026082904 relation already exists';
    END IF;
END
$$;

SET LOCAL ROLE ple_private_owner;

CREATE TABLE ple_private.webauthn_ceremony (
    ceremony_id uuid PRIMARY KEY,
    kind text NOT NULL,
    target_account_id uuid,
    browser_binding_hash bytea NOT NULL,
    state bytea NOT NULL,
    created_at timestamp with time zone NOT NULL,
    expires_at timestamp with time zone NOT NULL,
    consumed_at timestamp with time zone,
    CONSTRAINT webauthn_ceremony_kind_is_closed CHECK (kind IN ('registration', 'authentication')),
    CONSTRAINT webauthn_ceremony_target_is_valid CHECK (
        (kind = 'registration' AND target_account_id IS NOT NULL)
        OR kind = 'authentication'
    ),
    CONSTRAINT webauthn_ceremony_target_exists FOREIGN KEY (target_account_id)
        REFERENCES ple_private.account (account_id),
    CONSTRAINT webauthn_ceremony_binding_is_sha256 CHECK (pg_catalog.octet_length(browser_binding_hash) = 32),
    CONSTRAINT webauthn_ceremony_state_is_present CHECK (pg_catalog.octet_length(state) > 0),
    CONSTRAINT webauthn_ceremony_lifetime_is_bounded CHECK (
        expires_at > created_at AND expires_at <= created_at + interval '10 minutes'
    ),
    CONSTRAINT webauthn_ceremony_consumption_is_ordered CHECK (consumed_at IS NULL OR consumed_at >= created_at)
);

CREATE TABLE ple_private.passkey (
    passkey_id uuid PRIMARY KEY,
    account_id uuid NOT NULL REFERENCES ple_private.account (account_id),
    credential_id_hash bytea NOT NULL UNIQUE,
    label text NOT NULL,
    credential_state bytea NOT NULL,
    created_at timestamp with time zone NOT NULL,
    last_used_at timestamp with time zone,
    revoked_at timestamp with time zone,
    CONSTRAINT passkey_credential_id_hash_is_sha256 CHECK (pg_catalog.octet_length(credential_id_hash) = 32),
    CONSTRAINT passkey_label_is_bounded CHECK (char_length(btrim(label)) BETWEEN 1 AND 200),
    CONSTRAINT passkey_credential_state_is_present CHECK (pg_catalog.octet_length(credential_state) > 0),
    CONSTRAINT passkey_usage_is_ordered CHECK (last_used_at IS NULL OR last_used_at >= created_at),
    CONSTRAINT passkey_revocation_is_ordered CHECK (revoked_at IS NULL OR revoked_at >= created_at)
);

CREATE INDEX passkey_active_account_idx ON ple_private.passkey (account_id, created_at) WHERE revoked_at IS NULL;
ALTER TABLE ple_private.webauthn_ceremony ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.webauthn_ceremony FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_private.passkey ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.passkey FORCE ROW LEVEL SECURITY;
REVOKE ALL PRIVILEGES ON TABLE ple_private.webauthn_ceremony FROM PUBLIC;
REVOKE ALL PRIVILEGES ON TABLE ple_private.passkey FROM PUBLIC;
COMMENT ON TABLE ple_private.webauthn_ceremony IS
    'Private browser-bound, single-use WebAuthn ceremony state; no password verifier.';
COMMENT ON TABLE ple_private.passkey IS
    'Private serialized WebAuthn credential state for one global account.';

RESET ROLE;
