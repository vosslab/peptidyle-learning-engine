-- SD1 resolves an opaque active session and installs transaction-local session facts.

SET LOCAL ROLE ple_private_owner;
CREATE FUNCTION ple_private.resolve_active_authenticated_session(p_token_hash bytea)
RETURNS TABLE (
    account_id uuid,
    session_id uuid,
    role text,
    token_hash bytea,
    created_at timestamp with time zone,
    expires_at timestamp with time zone
)
LANGUAGE sql SECURITY DEFINER
SET search_path = pg_catalog, ple_private
AS $$
    SELECT sessions.account_id, sessions.session_id, sessions.role,
        sessions.token_hash, sessions.created_at, sessions.expires_at
      FROM ple_private.authenticated_session AS sessions
     WHERE sessions.token_hash = p_token_hash
       AND pg_catalog.octet_length(p_token_hash) = 32
       AND sessions.revoked_at IS NULL
       AND sessions.expires_at > pg_catalog.clock_timestamp()
$$;
REVOKE ALL PRIVILEGES ON FUNCTION ple_private.resolve_active_authenticated_session(bytea) FROM PUBLIC;
GRANT USAGE ON SCHEMA ple_private TO ple_api_owner;
GRANT EXECUTE ON FUNCTION ple_private.resolve_active_authenticated_session(bytea) TO ple_api_owner;
RESET ROLE;

SET LOCAL ROLE ple_api_owner;
CREATE FUNCTION ple_api.resolve_and_install_session(p_token_hash bytea)
RETURNS TABLE (
    account_id uuid,
    session_id uuid,
    role text,
    token_hash bytea,
    created_at timestamp with time zone,
    expires_at timestamp with time zone
)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private
AS $$
DECLARE
    resolved_account_id uuid;
    resolved_session_id uuid;
    resolved_role text;
    resolved_token_hash bytea;
    resolved_created_at timestamp with time zone;
    resolved_expires_at timestamp with time zone;
BEGIN
    SELECT resolved.account_id, resolved.session_id, resolved.role,
        resolved.token_hash, resolved.created_at, resolved.expires_at
      INTO resolved_account_id, resolved_session_id, resolved_role,
        resolved_token_hash, resolved_created_at, resolved_expires_at
      FROM ple_private.resolve_active_authenticated_session(p_token_hash) AS resolved;
    IF resolved_account_id IS NULL THEN
        RAISE EXCEPTION USING ERRCODE = '28000', MESSAGE = 'active session not found';
    END IF;
    PERFORM pg_catalog.set_config('ple.session_account_id', resolved_account_id::text, true);
    PERFORM pg_catalog.set_config('ple.session_id', resolved_session_id::text, true);
    RETURN QUERY SELECT resolved_account_id, resolved_session_id, resolved_role,
        resolved_token_hash, resolved_created_at, resolved_expires_at;
END
$$;
REVOKE ALL PRIVILEGES ON FUNCTION ple_api.resolve_and_install_session(bytea) FROM PUBLIC;
GRANT USAGE ON SCHEMA ple_private, ple_api TO ple_auth;
GRANT EXECUTE ON FUNCTION ple_api.resolve_and_install_session(bytea) TO ple_auth;
RESET ROLE;

SET LOCAL ROLE ple_private_owner;
CREATE FUNCTION ple_private.create_authenticated_session(
    p_session_id uuid,
    p_account_id uuid,
    p_role text,
    p_token_hash bytea,
    p_lifetime_seconds bigint
)
RETURNS TABLE (
    session_id uuid,
    token_hash bytea,
    account_id uuid,
    role text,
    created_at timestamp with time zone,
    expires_at timestamp with time zone
)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_private
AS $$
BEGIN
    IF p_session_id IS NULL
       OR p_account_id IS NULL
       OR p_role NOT IN ('student', 'instructor', 'sysadmin')
       OR pg_catalog.octet_length(p_token_hash) <> 32
       OR p_lifetime_seconds IS NULL
       OR p_lifetime_seconds < 1 THEN
        RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'invalid authenticated session input';
    END IF;
    RETURN QUERY
    INSERT INTO ple_private.authenticated_session (
        session_id, account_id, role, token_hash, created_at, expires_at
    )
    VALUES (
        p_session_id, p_account_id, p_role, p_token_hash,
        pg_catalog.transaction_timestamp(),
        pg_catalog.transaction_timestamp() + (p_lifetime_seconds * interval '1 second')
    )
    RETURNING authenticated_session.session_id, authenticated_session.token_hash,
        authenticated_session.account_id, authenticated_session.role,
        authenticated_session.created_at, authenticated_session.expires_at;
END
$$;

CREATE FUNCTION ple_private.revoke_authenticated_session(p_token_hash bytea)
RETURNS void
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_private
AS $$
BEGIN
    IF pg_catalog.octet_length(p_token_hash) <> 32 THEN
        RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'invalid authenticated session hash';
    END IF;
    UPDATE ple_private.authenticated_session
       SET revoked_at = pg_catalog.transaction_timestamp()
     WHERE token_hash = p_token_hash AND revoked_at IS NULL;
END
$$;
REVOKE ALL PRIVILEGES ON FUNCTION ple_private.create_authenticated_session(uuid, uuid, text, bytea, bigint) FROM PUBLIC;
REVOKE ALL PRIVILEGES ON FUNCTION ple_private.revoke_authenticated_session(bytea) FROM PUBLIC;
GRANT USAGE ON SCHEMA ple_private TO ple_auth;
GRANT EXECUTE ON FUNCTION ple_private.create_authenticated_session(uuid, uuid, text, bytea, bigint) TO ple_auth;
GRANT EXECUTE ON FUNCTION ple_private.revoke_authenticated_session(bytea) TO ple_auth;
RESET ROLE;
