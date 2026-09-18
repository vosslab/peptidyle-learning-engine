-- Functions, triggers, and views from authentication.sql.

SET LOCAL ROLE ple_private_owner;

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

CREATE FUNCTION ple_private.resolve_active_authenticated_session(p_token_hash bytea)
RETURNS TABLE (account_id text, session_id uuid, product_role text, token_hash bytea,
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
    p_session_id uuid, p_account_id text, p_token_hash bytea, p_lifetime_seconds bigint
)
RETURNS TABLE (session_id uuid, token_hash bytea, account_id text, product_role text,
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
    -- ASVS 2.3.1 and 6.3.4: every primary-authentication path must leave
    -- Sysadmin session creation to the one-use, browser-bound TOTP transition.
    IF v_role = 'sysadmin' THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Sysadmin TOTP completion required for a session';
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
RETURNS TABLE (account_id text, product_role text)
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
RETURNS TABLE (account_id text, product_role text)
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

CREATE FUNCTION ple_private.provision_sysadmin_totp_credential(
    p_account_id text, p_encryption_key_id text, p_seed_nonce bytea, p_encrypted_seed bytea
)
RETURNS void LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_private
AS $$
DECLARE v_now timestamptz := pg_catalog.transaction_timestamp();
BEGIN
    -- ASVS 2.2.1: reject malformed cipher parts before any security decision.
    IF p_account_id IS NULL OR char_length(p_encryption_key_id) NOT BETWEEN 1 AND 128
       OR pg_catalog.octet_length(p_seed_nonce) <> 24
       OR pg_catalog.octet_length(p_encrypted_seed) NOT BETWEEN 36 AND 80 THEN
        RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Invalid Sysadmin TOTP credential input';
    END IF;
    IF NOT EXISTS (
        SELECT 1 FROM ple_private.account AS account
        JOIN LATERAL (
            SELECT event.state FROM ple_private.account_state_event AS event
            WHERE event.account_id = account.account_id
            ORDER BY event.occurred_at DESC, event.event_id DESC LIMIT 1
        ) AS state ON state.state = 'active'
        WHERE account.account_id = p_account_id AND account.product_role = 'sysadmin'
    ) THEN
        RAISE EXCEPTION USING ERRCODE = '23503', MESSAGE = 'Active Sysadmin Account required';
    END IF;
    INSERT INTO ple_private.sysadmin_totp_credential (
        account_id, encryption_key_id, seed_nonce, encrypted_seed, provisioned_at
    ) VALUES (p_account_id, p_encryption_key_id, p_seed_nonce, p_encrypted_seed, v_now)
    ON CONFLICT (account_id) DO UPDATE SET
        encryption_key_id = EXCLUDED.encryption_key_id,
        seed_nonce = EXCLUDED.seed_nonce,
        encrypted_seed = EXCLUDED.encrypted_seed,
        provisioned_at = EXCLUDED.provisioned_at;
END
$$;

CREATE FUNCTION ple_private.create_pending_sysadmin_totp_attestation(
    p_attestation_id uuid, p_account_id text, p_browser_binding_hash bytea,
    p_lifetime_seconds bigint
)
RETURNS TABLE (sysadmin_totp_attestation_id uuid)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_private
AS $$
DECLARE v_now timestamptz := pg_catalog.transaction_timestamp();
BEGIN
    IF p_attestation_id IS NULL OR p_account_id IS NULL
       OR pg_catalog.octet_length(p_browser_binding_hash) <> 32
       OR p_lifetime_seconds NOT BETWEEN 1 AND 600 THEN
        RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Invalid Sysadmin TOTP attestation input';
    END IF;
    -- ASVS 2.3.1: role and active state come only from the database.  A
    -- non-Sysadmin receives the same empty result as an unknown Account.
    RETURN QUERY
    INSERT INTO ple_private.sysadmin_totp_attestation (
        sysadmin_totp_attestation_id, account_id, browser_binding_hash, created_at, expires_at
    ) SELECT p_attestation_id, account.account_id, p_browser_binding_hash, v_now,
             v_now + p_lifetime_seconds * interval '1 second'
      FROM ple_private.account AS account
      JOIN ple_private.sysadmin_totp_credential AS credential
        ON credential.account_id = account.account_id
      JOIN LATERAL (
          SELECT event.state FROM ple_private.account_state_event AS event
          WHERE event.account_id = account.account_id
          ORDER BY event.occurred_at DESC, event.event_id DESC LIMIT 1
      ) AS state ON state.state = 'active'
     WHERE account.account_id = p_account_id AND account.product_role = 'sysadmin'
    RETURNING sysadmin_totp_attestation.sysadmin_totp_attestation_id;
END
$$;

CREATE FUNCTION ple_private.load_pending_sysadmin_totp_attestation(
    p_attestation_id uuid, p_browser_binding_hash bytea
)
RETURNS TABLE (
    account_id text, encryption_key_id text, seed_nonce bytea, encrypted_seed bytea
)
LANGUAGE sql SECURITY DEFINER
SET search_path = pg_catalog, ple_private
AS $$
    SELECT attestation.account_id, credential.encryption_key_id, credential.seed_nonce,
           credential.encrypted_seed
      FROM ple_private.sysadmin_totp_attestation AS attestation
      JOIN ple_private.sysadmin_totp_credential AS credential
        ON credential.account_id = attestation.account_id
      JOIN ple_private.account AS account ON account.account_id = attestation.account_id
      JOIN LATERAL (
          SELECT event.state FROM ple_private.account_state_event AS event
          WHERE event.account_id = account.account_id
          ORDER BY event.occurred_at DESC, event.event_id DESC LIMIT 1
      ) AS state ON state.state = 'active'
     WHERE attestation.sysadmin_totp_attestation_id = p_attestation_id
       AND attestation.browser_binding_hash = p_browser_binding_hash
       AND attestation.consumed_at IS NULL
       AND attestation.expires_at > pg_catalog.clock_timestamp()
       AND account.product_role = 'sysadmin'
       AND pg_catalog.octet_length(p_browser_binding_hash) = 32
$$;

CREATE FUNCTION ple_private.reserve_sysadmin_totp_verification_attempt(
    p_attestation_id uuid, p_browser_binding_hash bytea
)
RETURNS TABLE (account_id text)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_private
AS $$
DECLARE
    v_now timestamptz := pg_catalog.transaction_timestamp();
    v_candidate record;
    v_attempt record;
    v_next_count integer;
BEGIN
    IF p_attestation_id IS NULL OR pg_catalog.octet_length(p_browser_binding_hash) <> 32 THEN
        RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Invalid Sysadmin TOTP verification attempt input';
    END IF;
    -- ASVS 2.3.1 and 2.3.3: lock the single server-resolved pending
    -- attestation first. Unknown, expired, consumed, inactive, wrong-browser,
    -- and non-Sysadmin requests allocate no mutable attempt state.
    SELECT attestation.account_id, attestation.expires_at
      INTO v_candidate
      FROM ple_private.sysadmin_totp_attestation AS attestation
      JOIN ple_private.account AS account ON account.account_id = attestation.account_id
      JOIN LATERAL (
          SELECT event.state FROM ple_private.account_state_event AS event
          WHERE event.account_id = account.account_id
          ORDER BY event.occurred_at DESC, event.event_id DESC LIMIT 1
      ) AS state ON state.state = 'active'
     WHERE attestation.sysadmin_totp_attestation_id = p_attestation_id
       AND attestation.browser_binding_hash = p_browser_binding_hash
       AND attestation.consumed_at IS NULL
       AND attestation.expires_at > pg_catalog.clock_timestamp()
       AND account.product_role = 'sysadmin'
     FOR UPDATE OF attestation;
    IF NOT FOUND THEN RETURN; END IF;

    SELECT attempt.account_id, attempt.browser_binding_hash, attempt.window_started_at,
           attempt.charged_attempt_count, attempt.locked_until
      INTO v_attempt
      FROM ple_private.sysadmin_totp_verification_attempt AS attempt
     WHERE attempt.sysadmin_totp_attestation_id = p_attestation_id
     FOR UPDATE;
    IF FOUND AND (v_attempt.account_id IS DISTINCT FROM v_candidate.account_id
                  OR v_attempt.browser_binding_hash IS DISTINCT FROM p_browser_binding_hash) THEN
        RAISE EXCEPTION USING ERRCODE = '55000', MESSAGE = 'Sysadmin TOTP attempt binding is invalid';
    END IF;
    IF FOUND AND v_attempt.locked_until IS NOT NULL AND v_attempt.locked_until > v_now THEN
        RETURN;
    END IF;
    IF NOT FOUND OR v_attempt.window_started_at + interval '10 minutes' <= v_now THEN
        INSERT INTO ple_private.sysadmin_totp_verification_attempt (
            sysadmin_totp_attestation_id, account_id, browser_binding_hash, window_started_at,
            charged_attempt_count, locked_until
        ) VALUES (
            p_attestation_id, v_candidate.account_id, p_browser_binding_hash, v_now, 1, NULL
        ) ON CONFLICT (sysadmin_totp_attestation_id) DO UPDATE SET
            account_id = EXCLUDED.account_id,
            browser_binding_hash = EXCLUDED.browser_binding_hash,
            window_started_at = EXCLUDED.window_started_at,
            charged_attempt_count = EXCLUDED.charged_attempt_count,
            locked_until = EXCLUDED.locked_until;
    ELSE
        v_next_count := v_attempt.charged_attempt_count + 1;
        IF v_next_count > 5 THEN RETURN; END IF;
        UPDATE ple_private.sysadmin_totp_verification_attempt AS attempt
           SET charged_attempt_count = v_next_count,
               locked_until = CASE WHEN v_next_count = 5 THEN v_candidate.expires_at ELSE NULL END
         WHERE attempt.sysadmin_totp_attestation_id = p_attestation_id;
    END IF;
    RETURN QUERY SELECT v_candidate.account_id;
END
$$;

CREATE FUNCTION ple_private.consume_sysadmin_totp_attestation_into_session(
    p_attestation_id uuid, p_browser_binding_hash bytea, p_totp_counter bigint,
    p_session_id uuid, p_token_hash bytea, p_lifetime_seconds bigint
)
RETURNS TABLE (session_id uuid, token_hash bytea, account_id text, product_role text,
               created_at timestamp with time zone, expires_at timestamp with time zone)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_private
AS $$
DECLARE v_now timestamptz := pg_catalog.transaction_timestamp();
BEGIN
    IF p_attestation_id IS NULL OR p_session_id IS NULL
       OR pg_catalog.octet_length(p_browser_binding_hash) <> 32
       OR p_totp_counter < 0 OR pg_catalog.octet_length(p_token_hash) <> 32
       OR p_lifetime_seconds NOT BETWEEN 1 AND 31536000 THEN
        RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Invalid Sysadmin TOTP completion input';
    END IF;
    -- ASVS 2.3.1, 2.3.3, and 6.5.1: lock the pending browser-bound ceremony,
    -- reserve the Account/counter once, create the session, then mark the
    -- ceremony consumed.  Any failed later step rolls the transaction back.
    RETURN QUERY
    WITH candidate AS MATERIALIZED (
        SELECT attestation.sysadmin_totp_attestation_id, attestation.account_id
          FROM ple_private.sysadmin_totp_attestation AS attestation
          JOIN ple_private.account AS account ON account.account_id = attestation.account_id
          JOIN LATERAL (
              SELECT event.state FROM ple_private.account_state_event AS event
              WHERE event.account_id = account.account_id
              ORDER BY event.occurred_at DESC, event.event_id DESC LIMIT 1
          ) AS state ON state.state = 'active'
         WHERE attestation.sysadmin_totp_attestation_id = p_attestation_id
           AND attestation.browser_binding_hash = p_browser_binding_hash
           AND attestation.consumed_at IS NULL
           AND attestation.expires_at > pg_catalog.clock_timestamp()
           AND account.product_role = 'sysadmin'
         FOR UPDATE OF attestation
    ), counter_once AS (
        INSERT INTO ple_private.sysadmin_totp_used_counter (account_id, totp_counter, used_at)
        SELECT candidate.account_id, p_totp_counter, v_now FROM candidate
        ON CONFLICT ON CONSTRAINT sysadmin_totp_used_counter_pkey DO NOTHING
        RETURNING sysadmin_totp_used_counter.account_id
    ), created AS (
        INSERT INTO ple_private.authenticated_session (
            session_id, account_id, product_role, token_hash, created_at, expires_at
        ) SELECT p_session_id, candidate.account_id, account.product_role, p_token_hash, v_now,
                 v_now + p_lifetime_seconds * interval '1 second'
            FROM candidate
            JOIN counter_once ON counter_once.account_id = candidate.account_id
            JOIN ple_private.account AS account ON account.account_id = candidate.account_id
        RETURNING authenticated_session.session_id, authenticated_session.token_hash,
                  authenticated_session.account_id, authenticated_session.product_role,
                  authenticated_session.created_at, authenticated_session.expires_at
    ), consumed AS (
        UPDATE ple_private.sysadmin_totp_attestation AS attestation SET consumed_at = v_now
          FROM created
         WHERE attestation.sysadmin_totp_attestation_id = p_attestation_id
           AND attestation.account_id = created.account_id
        RETURNING attestation.sysadmin_totp_attestation_id
    ) SELECT created.session_id, created.token_hash, created.account_id, created.product_role,
             created.created_at, created.expires_at
        FROM created CROSS JOIN consumed;
END
$$;

SET LOCAL ROLE ple_api_owner;

CREATE FUNCTION ple_api.resolve_and_install_session(p_token_hash bytea)
RETURNS TABLE (account_id text, session_id uuid, product_role text, token_hash bytea,
               created_at timestamp with time zone, expires_at timestamp with time zone)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private
AS $$
DECLARE resolved record;
BEGIN
    SELECT * INTO resolved FROM ple_private.resolve_active_authenticated_session(p_token_hash);
    -- An expired, revoked, deactivated, or unknown session is an ordinary
    -- unauthenticated outcome, not database infrastructure failure. Returning
    -- no row lets the HTTP boundary apply its nonenumerating route policy.
    IF NOT FOUND THEN RETURN; END IF;
    PERFORM pg_catalog.set_config('ple.session_account_id', resolved.account_id::text, true);
    PERFORM pg_catalog.set_config('ple.session_id', resolved.session_id::text, true);
    RETURN QUERY SELECT resolved.account_id, resolved.session_id, resolved.product_role,
        resolved.token_hash, resolved.created_at, resolved.expires_at;
END
$$;

CREATE FUNCTION ple_api.create_authenticated_session(
    p_session_id uuid, p_account_id text, p_token_hash bytea, p_lifetime_seconds bigint
)
RETURNS TABLE (session_id uuid, token_hash bytea, account_id text, product_role text,
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
RETURNS TABLE (account_id text, product_role text)
LANGUAGE sql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private
AS $$ SELECT * FROM ple_private.consume_email_authentication_challenge(
    p_challenge_id, p_proof_hash, p_browser_binding_hash
) $$;

CREATE FUNCTION ple_api.consume_passkey_authentication(
    p_ceremony_id uuid, p_credential_id_hash bytea, p_browser_binding_hash bytea
)
RETURNS TABLE (account_id text, product_role text)
LANGUAGE sql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private
AS $$ SELECT * FROM ple_private.consume_passkey_authentication(
    p_ceremony_id, p_credential_id_hash, p_browser_binding_hash
) $$;

CREATE FUNCTION ple_api.provision_sysadmin_totp_credential(
    p_account_id text, p_encryption_key_id text, p_seed_nonce bytea, p_encrypted_seed bytea
)
RETURNS void LANGUAGE sql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private
AS $$ SELECT ple_private.provision_sysadmin_totp_credential(
    p_account_id, p_encryption_key_id, p_seed_nonce, p_encrypted_seed
) $$;

CREATE FUNCTION ple_api.create_pending_sysadmin_totp_attestation(
    p_attestation_id uuid, p_account_id text, p_browser_binding_hash bytea,
    p_lifetime_seconds bigint
)
RETURNS TABLE (sysadmin_totp_attestation_id uuid)
LANGUAGE sql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private
AS $$ SELECT * FROM ple_private.create_pending_sysadmin_totp_attestation(
    p_attestation_id, p_account_id, p_browser_binding_hash, p_lifetime_seconds
) $$;

CREATE FUNCTION ple_api.load_pending_sysadmin_totp_attestation(
    p_attestation_id uuid, p_browser_binding_hash bytea
)
RETURNS TABLE (
    account_id text, encryption_key_id text, seed_nonce bytea, encrypted_seed bytea
)
LANGUAGE sql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private
AS $$ SELECT * FROM ple_private.load_pending_sysadmin_totp_attestation(
    p_attestation_id, p_browser_binding_hash
) $$;

CREATE FUNCTION ple_api.reserve_sysadmin_totp_verification_attempt(
    p_attestation_id uuid, p_browser_binding_hash bytea
)
RETURNS TABLE (account_id text)
LANGUAGE sql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private
AS $$ SELECT * FROM ple_private.reserve_sysadmin_totp_verification_attempt(
    p_attestation_id, p_browser_binding_hash
) $$;

CREATE FUNCTION ple_api.consume_sysadmin_totp_attestation_into_session(
    p_attestation_id uuid, p_browser_binding_hash bytea, p_totp_counter bigint,
    p_session_id uuid, p_token_hash bytea, p_lifetime_seconds bigint
)
RETURNS TABLE (session_id uuid, token_hash bytea, account_id text, product_role text,
               created_at timestamp with time zone, expires_at timestamp with time zone)
LANGUAGE sql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private
AS $$ SELECT * FROM ple_private.consume_sysadmin_totp_attestation_into_session(
    p_attestation_id, p_browser_binding_hash, p_totp_counter, p_session_id,
    p_token_hash, p_lifetime_seconds
) $$;

