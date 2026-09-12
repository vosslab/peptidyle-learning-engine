-- Passwordless email and passkey authentication plus opaque sessions.

SET LOCAL ROLE ple_private_owner;

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

CREATE FUNCTION ple_private.enforce_account_authentication_email_role()
RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_private
AS $$
DECLARE v_role text;
BEGIN
    SELECT product_role INTO v_role FROM ple_private.account WHERE account_id = NEW.account_id;
    IF v_role NOT IN ('student', 'instructor') THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Authentication Email requires a Student or Instructor Account';
    END IF;
    IF TG_OP = 'UPDATE' AND (NEW.account_id IS DISTINCT FROM OLD.account_id OR v_role <> 'instructor') THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Student Authentication Email is immutable';
    END IF;
    RETURN NEW;
END
$$;

CREATE TRIGGER account_authentication_email_role_is_enforced
BEFORE INSERT OR UPDATE ON ple_private.account_authentication_email
FOR EACH ROW EXECUTE FUNCTION ple_private.enforce_account_authentication_email_role();

CREATE TABLE ple_private.authentication_rate_limit (
    scope text NOT NULL CHECK (scope IN ('email', 'network', 'principal', 'service')),
    key_hash bytea NOT NULL CHECK (pg_catalog.octet_length(key_hash) = 32),
    window_started_at timestamp with time zone NOT NULL,
    consumed_attempts integer NOT NULL CHECK (consumed_attempts BETWEEN 1 AND 10000),
    PRIMARY KEY (scope, key_hash, window_started_at)
);

CREATE TABLE ple_private.email_authentication_challenge (
    challenge_id uuid PRIMARY KEY,
    token_hash bytea NOT NULL UNIQUE CHECK (pg_catalog.octet_length(token_hash) = 32),
    browser_binding_hash bytea NOT NULL CHECK (pg_catalog.octet_length(browser_binding_hash) = 32),
    email_rate_limit_key_hash bytea NOT NULL CHECK (pg_catalog.octet_length(email_rate_limit_key_hash) = 32),
    email text NOT NULL CHECK (char_length(email) BETWEEN 3 AND 320 AND email = lower(btrim(email))),
    purpose text NOT NULL CHECK (purpose IN ('sign_in', 'change_email')),
    target_account_id uuid NOT NULL REFERENCES ple_private.account (account_id),
    created_at timestamp with time zone NOT NULL,
    expires_at timestamp with time zone NOT NULL,
    consumed_at timestamp with time zone,
    CHECK (expires_at > created_at AND expires_at <= created_at + interval '10 minutes'),
    CHECK (consumed_at IS NULL OR consumed_at >= created_at)
);

CREATE INDEX email_authentication_challenge_active_token_idx
ON ple_private.email_authentication_challenge (token_hash, expires_at) WHERE consumed_at IS NULL;

CREATE TABLE ple_private.passkey_ceremony (
    ceremony_id uuid PRIMARY KEY,
    kind text NOT NULL CHECK (kind IN ('registration', 'authentication')),
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
    CHECK (revoked_at IS NULL OR revoked_at >= created_at)
);

CREATE INDEX passkey_active_account_idx ON ple_private.passkey (account_id, created_at)
WHERE revoked_at IS NULL;

CREATE TABLE ple_private.authenticated_session (
    session_id uuid PRIMARY KEY,
    account_id uuid NOT NULL REFERENCES ple_private.account (account_id),
    product_role text NOT NULL,
    token_hash bytea NOT NULL UNIQUE CHECK (pg_catalog.octet_length(token_hash) = 32),
    created_at timestamp with time zone NOT NULL,
    expires_at timestamp with time zone NOT NULL,
    revoked_at timestamp with time zone,
    CHECK (expires_at > created_at),
    CHECK (revoked_at IS NULL OR revoked_at >= created_at),
    FOREIGN KEY (account_id, product_role) REFERENCES ple_private.account (account_id, product_role)
);

CREATE INDEX authenticated_session_active_account_idx ON ple_private.authenticated_session (account_id, expires_at)
WHERE revoked_at IS NULL;

CREATE FUNCTION ple_private.reject_authenticated_session_identity_change()
RETURNS trigger LANGUAGE plpgsql
SET search_path = pg_catalog, ple_private
AS $$
BEGIN
    IF NEW.session_id IS DISTINCT FROM OLD.session_id OR NEW.account_id IS DISTINCT FROM OLD.account_id
       OR NEW.product_role IS DISTINCT FROM OLD.product_role OR NEW.token_hash IS DISTINCT FROM OLD.token_hash
       OR NEW.created_at IS DISTINCT FROM OLD.created_at OR NEW.expires_at IS DISTINCT FROM OLD.expires_at
       OR (OLD.revoked_at IS NOT NULL AND NEW.revoked_at IS DISTINCT FROM OLD.revoked_at) THEN
        RAISE EXCEPTION USING ERRCODE = '23514', MESSAGE = 'Session identity is immutable';
    END IF;
    RETURN NEW;
END
$$;

CREATE TRIGGER authenticated_session_identity_is_immutable
BEFORE UPDATE ON ple_private.authenticated_session
FOR EACH ROW EXECUTE FUNCTION ple_private.reject_authenticated_session_identity_change();

CREATE FUNCTION ple_private.revoke_sessions_after_account_deactivation_or_closure()
RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_private
AS $$
BEGIN
    IF NEW.state IN ('deactivated', 'closed') THEN
        UPDATE ple_private.authenticated_session SET revoked_at = pg_catalog.transaction_timestamp()
        WHERE account_id = NEW.account_id AND revoked_at IS NULL;
    END IF;
    RETURN NEW;
END
$$;

CREATE TRIGGER account_deactivation_or_closure_revokes_sessions
AFTER INSERT ON ple_private.account_state_event
FOR EACH ROW EXECUTE FUNCTION ple_private.revoke_sessions_after_account_deactivation_or_closure();

ALTER TABLE ple_private.account_authentication_email ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.account_authentication_email FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_private.authentication_rate_limit ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.authentication_rate_limit FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_private.email_authentication_challenge ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.email_authentication_challenge FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_private.passkey_ceremony ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.passkey_ceremony FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_private.passkey ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.passkey FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_private.authenticated_session ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.authenticated_session FORCE ROW LEVEL SECURITY;

CREATE POLICY account_authentication_email_private_owner_access ON ple_private.account_authentication_email
    FOR ALL TO ple_private_owner USING (true) WITH CHECK (true);
CREATE POLICY authentication_rate_limit_private_owner_access ON ple_private.authentication_rate_limit
    FOR ALL TO ple_private_owner USING (true) WITH CHECK (true);
CREATE POLICY email_authentication_challenge_private_owner_access ON ple_private.email_authentication_challenge
    FOR ALL TO ple_private_owner USING (true) WITH CHECK (true);
CREATE POLICY passkey_ceremony_private_owner_access ON ple_private.passkey_ceremony
    FOR ALL TO ple_private_owner USING (true) WITH CHECK (true);
CREATE POLICY passkey_private_owner_access ON ple_private.passkey
    FOR ALL TO ple_private_owner USING (true) WITH CHECK (true);
CREATE POLICY authenticated_session_private_owner_access ON ple_private.authenticated_session
    FOR ALL TO ple_private_owner USING (true) WITH CHECK (true);

REVOKE ALL PRIVILEGES ON TABLE ple_private.account_authentication_email,
    ple_private.authentication_rate_limit, ple_private.email_authentication_challenge,
    ple_private.passkey_ceremony, ple_private.passkey, ple_private.authenticated_session FROM PUBLIC;

CREATE FUNCTION ple_private.resolve_active_authenticated_session(p_token_hash bytea)
RETURNS TABLE (account_id uuid, session_id uuid, product_role text, token_hash bytea,
               created_at timestamp with time zone, expires_at timestamp with time zone)
LANGUAGE sql SECURITY DEFINER
SET search_path = pg_catalog, ple_private
AS $$
    SELECT session.account_id, session.session_id, session.product_role, session.token_hash,
           session.created_at, session.expires_at
    FROM ple_private.authenticated_session AS session
    JOIN LATERAL (
        SELECT event.state FROM ple_private.account_state_event AS event
         WHERE event.account_id = session.account_id
         ORDER BY event.occurred_at DESC, event.event_id DESC LIMIT 1
    ) AS account_state ON account_state.state = 'active'
    WHERE session.token_hash = p_token_hash AND pg_catalog.octet_length(p_token_hash) = 32
      AND session.revoked_at IS NULL AND session.expires_at > pg_catalog.clock_timestamp()
$$;

CREATE FUNCTION ple_private.create_authenticated_session(
    p_session_id uuid, p_account_id uuid, p_token_hash bytea, p_lifetime_seconds bigint
)
RETURNS TABLE (session_id uuid, token_hash bytea, account_id uuid, product_role text,
               created_at timestamp with time zone, expires_at timestamp with time zone)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_private
AS $$
DECLARE v_role text; v_now timestamptz := pg_catalog.transaction_timestamp();
BEGIN
    IF p_session_id IS NULL OR p_account_id IS NULL OR pg_catalog.octet_length(p_token_hash) <> 32
       OR p_lifetime_seconds NOT BETWEEN 1 AND 31536000 THEN
        RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Invalid authenticated session input';
    END IF;
    SELECT account.product_role INTO v_role FROM ple_private.account AS account
    JOIN LATERAL (
        SELECT event.state FROM ple_private.account_state_event AS event
         WHERE event.account_id = account.account_id
         ORDER BY event.occurred_at DESC, event.event_id DESC LIMIT 1
    ) AS state ON state.state = 'active' WHERE account.account_id = p_account_id;
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '23503', MESSAGE = 'Active Account required for a session';
    END IF;
    RETURN QUERY INSERT INTO ple_private.authenticated_session (
        session_id, account_id, product_role, token_hash, created_at, expires_at
    ) VALUES (p_session_id, p_account_id, v_role, p_token_hash, v_now,
              v_now + p_lifetime_seconds * interval '1 second')
    RETURNING authenticated_session.session_id, authenticated_session.token_hash,
              authenticated_session.account_id, authenticated_session.product_role,
              authenticated_session.created_at, authenticated_session.expires_at;
END
$$;

CREATE FUNCTION ple_private.revoke_authenticated_session(p_token_hash bytea)
RETURNS void LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_private
AS $$
BEGIN
    IF pg_catalog.octet_length(p_token_hash) <> 32 THEN
        RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Invalid authenticated session hash';
    END IF;
    UPDATE ple_private.authenticated_session SET revoked_at = pg_catalog.transaction_timestamp()
    WHERE token_hash = p_token_hash AND revoked_at IS NULL;
END
$$;

CREATE FUNCTION ple_private.consume_email_authentication_challenge(
    p_challenge_id uuid, p_proof_hash bytea, p_browser_binding_hash bytea
)
RETURNS TABLE (account_id uuid, product_role text)
LANGUAGE sql SECURITY DEFINER
SET search_path = pg_catalog, ple_private
AS $$
    WITH consumed AS (
        UPDATE ple_private.email_authentication_challenge SET consumed_at = pg_catalog.transaction_timestamp()
        WHERE challenge_id = p_challenge_id AND token_hash = p_proof_hash
          AND browser_binding_hash = p_browser_binding_hash AND consumed_at IS NULL
          AND expires_at > pg_catalog.clock_timestamp()
          AND pg_catalog.octet_length(p_proof_hash) = 32
          AND pg_catalog.octet_length(p_browser_binding_hash) = 32
        RETURNING target_account_id
    ) SELECT account.account_id, account.product_role FROM consumed
      JOIN ple_private.account AS account ON account.account_id = consumed.target_account_id
$$;

CREATE FUNCTION ple_private.consume_passkey_authentication(
    p_ceremony_id uuid, p_credential_id_hash bytea, p_browser_binding_hash bytea
)
RETURNS TABLE (account_id uuid, product_role text)
LANGUAGE sql SECURITY DEFINER
SET search_path = pg_catalog, ple_private
AS $$
    WITH consumed AS (
        UPDATE ple_private.passkey_ceremony SET consumed_at = pg_catalog.transaction_timestamp()
        WHERE ceremony_id = p_ceremony_id AND kind = 'authentication'
          AND browser_binding_hash = p_browser_binding_hash AND consumed_at IS NULL
          AND expires_at > pg_catalog.clock_timestamp()
          AND pg_catalog.octet_length(p_browser_binding_hash) = 32
        RETURNING ceremony_id
    ), used AS (
        UPDATE ple_private.passkey SET last_used_at = pg_catalog.transaction_timestamp()
        FROM consumed WHERE credential_id_hash = p_credential_id_hash AND revoked_at IS NULL
          AND pg_catalog.octet_length(p_credential_id_hash) = 32 RETURNING account_id
    ) SELECT account.account_id, account.product_role FROM used
      JOIN ple_private.account AS account ON account.account_id = used.account_id
$$;

REVOKE ALL PRIVILEGES ON FUNCTION ple_private.enforce_account_authentication_email_role(),
    ple_private.reject_authenticated_session_identity_change(),
    ple_private.revoke_sessions_after_account_deactivation_or_closure(),
    ple_private.resolve_active_authenticated_session(bytea),
    ple_private.create_authenticated_session(uuid, uuid, bytea, bigint),
    ple_private.revoke_authenticated_session(bytea),
    ple_private.consume_email_authentication_challenge(uuid, bytea, bytea),
    ple_private.consume_passkey_authentication(uuid, bytea, bytea) FROM PUBLIC;
GRANT USAGE ON SCHEMA ple_private TO ple_api_owner;
GRANT EXECUTE ON FUNCTION ple_private.resolve_active_authenticated_session(bytea),
    ple_private.create_authenticated_session(uuid, uuid, bytea, bigint),
    ple_private.revoke_authenticated_session(bytea),
    ple_private.consume_email_authentication_challenge(uuid, bytea, bytea),
    ple_private.consume_passkey_authentication(uuid, bytea, bytea) TO ple_api_owner;

RESET ROLE;

SET LOCAL ROLE ple_api_owner;

CREATE FUNCTION ple_api.resolve_and_install_session(p_token_hash bytea)
RETURNS TABLE (account_id uuid, session_id uuid, product_role text, token_hash bytea,
               created_at timestamp with time zone, expires_at timestamp with time zone)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private
AS $$
DECLARE resolved record;
BEGIN
    SELECT * INTO resolved FROM ple_private.resolve_active_authenticated_session(p_token_hash);
    IF NOT FOUND THEN RAISE EXCEPTION USING ERRCODE = '28000', MESSAGE = 'Active session not found'; END IF;
    PERFORM pg_catalog.set_config('ple.session_account_id', resolved.account_id::text, true);
    PERFORM pg_catalog.set_config('ple.session_id', resolved.session_id::text, true);
    RETURN QUERY SELECT resolved.account_id, resolved.session_id, resolved.product_role,
        resolved.token_hash, resolved.created_at, resolved.expires_at;
END
$$;

CREATE FUNCTION ple_api.create_authenticated_session(
    p_session_id uuid, p_account_id uuid, p_token_hash bytea, p_lifetime_seconds bigint
)
RETURNS TABLE (session_id uuid, token_hash bytea, account_id uuid, product_role text,
               created_at timestamp with time zone, expires_at timestamp with time zone)
LANGUAGE sql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private
AS $$ SELECT * FROM ple_private.create_authenticated_session(
    p_session_id, p_account_id, p_token_hash, p_lifetime_seconds
) $$;

CREATE FUNCTION ple_api.revoke_authenticated_session(p_token_hash bytea)
RETURNS void LANGUAGE sql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private
AS $$ SELECT ple_private.revoke_authenticated_session(p_token_hash) $$;

CREATE FUNCTION ple_api.consume_email_authentication_challenge(
    p_challenge_id uuid, p_proof_hash bytea, p_browser_binding_hash bytea
)
RETURNS TABLE (account_id uuid, product_role text)
LANGUAGE sql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private
AS $$ SELECT * FROM ple_private.consume_email_authentication_challenge(
    p_challenge_id, p_proof_hash, p_browser_binding_hash
) $$;

CREATE FUNCTION ple_api.consume_passkey_authentication(
    p_ceremony_id uuid, p_credential_id_hash bytea, p_browser_binding_hash bytea
)
RETURNS TABLE (account_id uuid, product_role text)
LANGUAGE sql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private
AS $$ SELECT * FROM ple_private.consume_passkey_authentication(
    p_ceremony_id, p_credential_id_hash, p_browser_binding_hash
) $$;

REVOKE ALL PRIVILEGES ON FUNCTION ple_api.resolve_and_install_session(bytea),
    ple_api.create_authenticated_session(uuid, uuid, bytea, bigint),
    ple_api.revoke_authenticated_session(bytea),
    ple_api.consume_email_authentication_challenge(uuid, bytea, bytea),
    ple_api.consume_passkey_authentication(uuid, bytea, bytea) FROM PUBLIC;
GRANT USAGE ON SCHEMA ple_api TO ple_auth;
GRANT EXECUTE ON FUNCTION ple_api.resolve_and_install_session(bytea),
    ple_api.create_authenticated_session(uuid, uuid, bytea, bigint),
    ple_api.revoke_authenticated_session(bytea),
    ple_api.consume_email_authentication_challenge(uuid, bytea, bytea),
    ple_api.consume_passkey_authentication(uuid, bytea, bytea) TO ple_auth;

RESET ROLE;
