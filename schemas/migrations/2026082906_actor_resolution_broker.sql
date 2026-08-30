-- SD1 resolves an opaque active session and installs only transaction-local actor identity.

SET LOCAL ROLE ple_private_owner;
CREATE FUNCTION ple_private.resolve_active_primary_session(p_token_hash bytea)
RETURNS TABLE (
    user_id uuid,
    session_id uuid,
    role text,
    token_hash bytea,
    created_at timestamp with time zone,
    expires_at timestamp with time zone
)
LANGUAGE sql SECURITY DEFINER
SET search_path = pg_catalog, ple_private
AS $$
    SELECT sessions.user_id, sessions.session_id, sessions.role,
        sessions.token_hash, sessions.created_at, sessions.expires_at
      FROM ple_private.primary_session AS sessions
     WHERE sessions.token_hash = p_token_hash
       AND pg_catalog.octet_length(p_token_hash) = 32
       AND sessions.revoked_at IS NULL
       AND sessions.expires_at > pg_catalog.clock_timestamp()
$$;
REVOKE ALL PRIVILEGES ON FUNCTION ple_private.resolve_active_primary_session(bytea) FROM PUBLIC;
GRANT USAGE ON SCHEMA ple_private TO ple_api_owner;
GRANT EXECUTE ON FUNCTION ple_private.resolve_active_primary_session(bytea) TO ple_api_owner;
RESET ROLE;

SET LOCAL ROLE ple_api_owner;
CREATE FUNCTION ple_api.resolve_and_install_actor(p_token_hash bytea)
RETURNS TABLE (
    user_id uuid,
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
    resolved_user_id uuid;
    resolved_session_id uuid;
    resolved_role text;
    resolved_token_hash bytea;
    resolved_created_at timestamp with time zone;
    resolved_expires_at timestamp with time zone;
BEGIN
    SELECT resolved.user_id, resolved.session_id, resolved.role,
        resolved.token_hash, resolved.created_at, resolved.expires_at
      INTO resolved_user_id, resolved_session_id, resolved_role,
        resolved_token_hash, resolved_created_at, resolved_expires_at
      FROM ple_private.resolve_active_primary_session(p_token_hash) AS resolved;
    IF resolved_user_id IS NULL THEN
        RAISE EXCEPTION USING ERRCODE = '28000', MESSAGE = 'active session not found';
    END IF;
    PERFORM pg_catalog.set_config('ple.actor_user_id', resolved_user_id::text, true);
    PERFORM pg_catalog.set_config('ple.actor_session_id', resolved_session_id::text, true);
    RETURN QUERY SELECT resolved_user_id, resolved_session_id, resolved_role,
        resolved_token_hash, resolved_created_at, resolved_expires_at;
END
$$;
REVOKE ALL PRIVILEGES ON FUNCTION ple_api.resolve_and_install_actor(bytea) FROM PUBLIC;
GRANT USAGE ON SCHEMA ple_private, ple_api TO ple_auth;
GRANT EXECUTE ON FUNCTION ple_api.resolve_and_install_actor(bytea) TO ple_auth;
RESET ROLE;

SET LOCAL ROLE ple_private_owner;
CREATE FUNCTION ple_private.create_primary_session(
    p_session_id uuid,
    p_user_id uuid,
    p_role text,
    p_token_hash bytea,
    p_lifetime_seconds bigint
)
RETURNS TABLE (
    session_id uuid,
    token_hash bytea,
    user_id uuid,
    role text,
    created_at timestamp with time zone,
    expires_at timestamp with time zone
)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_private
AS $$
BEGIN
    IF p_session_id IS NULL
       OR p_user_id IS NULL
       OR p_role NOT IN ('student', 'instructor', 'sysadmin')
       OR pg_catalog.octet_length(p_token_hash) <> 32
       OR p_lifetime_seconds IS NULL
       OR p_lifetime_seconds < 1 THEN
        RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'invalid primary session input';
    END IF;
    RETURN QUERY
    INSERT INTO ple_private.primary_session (
        session_id, user_id, role, token_hash, created_at, expires_at
    )
    VALUES (
        p_session_id, p_user_id, p_role, p_token_hash,
        pg_catalog.transaction_timestamp(),
        pg_catalog.transaction_timestamp() + (p_lifetime_seconds * interval '1 second')
    )
    RETURNING primary_session.session_id, primary_session.token_hash,
        primary_session.user_id, primary_session.role,
        primary_session.created_at, primary_session.expires_at;
END
$$;

CREATE FUNCTION ple_private.revoke_primary_session(p_token_hash bytea)
RETURNS void
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_private
AS $$
BEGIN
    IF pg_catalog.octet_length(p_token_hash) <> 32 THEN
        RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'invalid primary session hash';
    END IF;
    UPDATE ple_private.primary_session
       SET revoked_at = pg_catalog.transaction_timestamp()
     WHERE token_hash = p_token_hash AND revoked_at IS NULL;
END
$$;
REVOKE ALL PRIVILEGES ON FUNCTION ple_private.create_primary_session(uuid, uuid, text, bytea, bigint) FROM PUBLIC;
REVOKE ALL PRIVILEGES ON FUNCTION ple_private.revoke_primary_session(bytea) FROM PUBLIC;
GRANT USAGE ON SCHEMA ple_private TO ple_auth;
GRANT EXECUTE ON FUNCTION ple_private.create_primary_session(uuid, uuid, text, bytea, bigint) TO ple_auth;
GRANT EXECUTE ON FUNCTION ple_private.revoke_primary_session(bytea) TO ple_auth;
RESET ROLE;
