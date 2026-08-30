-- SD1 atomic passwordless-credential completion brokers.
--
-- These brokers establish only an existing Account's immutable role. The
-- route subsequently creates the one Authenticated Session through its
-- separate broker; no credential ceremony provisions an Account or grants a
-- course relationship.

DO $$
BEGIN
    IF current_user <> 'ple_migrator'
       OR NOT pg_catalog.pg_has_role('ple_migrator', 'ple_private_owner', 'SET') THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'migration 2026082933 requires the SD1 private migration principal';
    END IF;
END
$$;

SET LOCAL ROLE ple_private_owner;

-- ASVS 3.3.1 and 3.3.3: successful lookup consumes the browser-bound email
-- proof in the same statement that returns its pre-provisioned Account.
CREATE FUNCTION ple_private.consume_email_authentication_challenge(
    p_challenge_id uuid,
    p_proof_hash bytea,
    p_browser_binding_hash bytea
)
RETURNS TABLE (account_id uuid, role text)
LANGUAGE sql SECURITY DEFINER
SET search_path = pg_catalog, ple_private
AS $$
    WITH consumed AS (
        UPDATE ple_private.email_authentication_challenge AS challenge
           SET consumed_at = pg_catalog.transaction_timestamp()
         WHERE challenge.challenge_id = p_challenge_id
           AND challenge.token_hash = p_proof_hash
           AND challenge.browser_binding_hash = p_browser_binding_hash
           AND challenge.consumed_at IS NULL
           AND challenge.expires_at > pg_catalog.clock_timestamp()
           AND pg_catalog.octet_length(p_proof_hash) = 32
           AND pg_catalog.octet_length(p_browser_binding_hash) = 32
         RETURNING challenge.target_account_id
    )
    SELECT account.account_id, account.role
      FROM consumed
      JOIN ple_private.account AS account ON account.account_id = consumed.target_account_id
$$;

-- The WebAuthn adapter verifies the assertion before calling this broker. The
-- broker then atomically consumes its browser-bound ceremony and records use
-- of only a non-reversible credential-ID hash.
CREATE FUNCTION ple_private.consume_passkey_authentication(
    p_ceremony_id uuid,
    p_credential_id_hash bytea,
    p_browser_binding_hash bytea
)
RETURNS TABLE (account_id uuid, role text)
LANGUAGE sql SECURITY DEFINER
SET search_path = pg_catalog, ple_private
AS $$
    WITH consumed_ceremony AS (
        UPDATE ple_private.webauthn_ceremony AS ceremony
           SET consumed_at = pg_catalog.transaction_timestamp()
         WHERE ceremony.ceremony_id = p_ceremony_id
           AND ceremony.kind = 'authentication'
           AND ceremony.browser_binding_hash = p_browser_binding_hash
           AND ceremony.consumed_at IS NULL
           AND ceremony.expires_at > pg_catalog.clock_timestamp()
           AND pg_catalog.octet_length(p_browser_binding_hash) = 32
         RETURNING ceremony.ceremony_id
    ), used_passkey AS (
        UPDATE ple_private.passkey AS passkey
           SET last_used_at = pg_catalog.transaction_timestamp()
          FROM consumed_ceremony
         WHERE passkey.credential_id_hash = p_credential_id_hash
           AND passkey.revoked_at IS NULL
           AND pg_catalog.octet_length(p_credential_id_hash) = 32
         RETURNING passkey.account_id
    )
    SELECT account.account_id, account.role
      FROM used_passkey
      JOIN ple_private.account AS account ON account.account_id = used_passkey.account_id
$$;

REVOKE ALL PRIVILEGES ON FUNCTION ple_private.consume_email_authentication_challenge(uuid, bytea, bytea) FROM PUBLIC;
REVOKE ALL PRIVILEGES ON FUNCTION ple_private.consume_passkey_authentication(uuid, bytea, bytea) FROM PUBLIC;
GRANT USAGE ON SCHEMA ple_private TO ple_auth;
GRANT EXECUTE ON FUNCTION ple_private.consume_email_authentication_challenge(uuid, bytea, bytea) TO ple_auth;
GRANT EXECUTE ON FUNCTION ple_private.consume_passkey_authentication(uuid, bytea, bytea) TO ple_auth;

COMMENT ON FUNCTION ple_private.consume_email_authentication_challenge(uuid, bytea, bytea) IS
    'Atomically consumes one eligible browser-bound email challenge and returns its existing Account role.';
COMMENT ON FUNCTION ple_private.consume_passkey_authentication(uuid, bytea, bytea) IS
    'Consumes one validated browser-bound WebAuthn ceremony and records active passkey use.';

RESET ROLE;
