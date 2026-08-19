-- A RETURNS TABLE function exposes its output names as PL/pgSQL variables.
-- Qualify every account/session column so the SQL capability reaches its
-- deliberate conflict branch instead of failing on ambiguous identifiers.

CREATE OR REPLACE FUNCTION public.ple_complete_email_change_and_revoke_sessions(
    p_token_hash bytea,
    p_browser_binding_hash bytea,
    p_user_id uuid,
    p_session_token_hash bytea,
    p_session_seconds bigint
) RETURNS TABLE (
    user_id uuid,
    normalized_email text,
    delivery_email text,
    display_name text,
    platform_roles jsonb,
    created_at_millis bigint,
    updated_at_millis bigint,
    session_created_at_millis bigint,
    session_expires_at_millis bigint
)
    LANGUAGE plpgsql
    SECURITY DEFINER
    SET search_path TO pg_catalog, public
    AS $$
DECLARE
    v_normalized_email text;
    v_delivery_email text;
    v_rate_limit_key_hash bytea;
    v_account public.ple_account%ROWTYPE;
    v_session_created_at timestamp with time zone;
    v_session_expires_at timestamp with time zone;
BEGIN
    IF octet_length(p_token_hash) <> 32
       OR octet_length(p_browser_binding_hash) <> 32
       OR octet_length(p_session_token_hash) <> 32
       OR p_session_seconds NOT BETWEEN 1 AND 900 THEN
        RETURN;
    END IF;

    DELETE FROM public.email_authentication_challenge AS challenge
     WHERE challenge.token_hash = p_token_hash
       AND challenge.browser_binding_hash = p_browser_binding_hash
       AND challenge.expires_at > transaction_timestamp()
       AND challenge.purpose = 'change_email'
       AND challenge.purpose_user_id = p_user_id
     RETURNING challenge.normalized_email, challenge.delivery_email, challenge.rate_limit_key_hash
      INTO v_normalized_email, v_delivery_email, v_rate_limit_key_hash;
    IF NOT FOUND THEN
        RETURN;
    END IF;

    BEGIN
        UPDATE public.ple_account AS account
           SET normalized_email = v_normalized_email,
               delivery_email = v_delivery_email,
               updated_at = transaction_timestamp()
         WHERE account.user_id = p_user_id
         RETURNING account.* INTO v_account;
    EXCEPTION WHEN unique_violation THEN
        RAISE EXCEPTION 'email address is already assigned' USING ERRCODE = '55000';
    END;
    IF NOT FOUND THEN
        RAISE EXCEPTION 'email challenge references a missing account'
            USING ERRCODE = '23503';
    END IF;

    DELETE FROM public.account_authentication_session AS session
     WHERE session.user_id = p_user_id;
    UPDATE public.auth_session AS session
       SET revoked_at = transaction_timestamp()
     WHERE session.user_id = p_user_id AND session.revoked_at IS NULL;

    INSERT INTO public.account_authentication_session (token_hash, user_id, expires_at)
         VALUES (
             p_session_token_hash,
             p_user_id,
             transaction_timestamp() + (p_session_seconds * interval '1 second')
         )
      RETURNING created_at, expires_at
           INTO v_session_created_at, v_session_expires_at;

    DELETE FROM public.authentication_rate_limit AS rate_limit
     WHERE rate_limit.limit_scope = 'email' AND rate_limit.key_hash = v_rate_limit_key_hash;

    RETURN QUERY
    SELECT
        v_account.user_id,
        v_account.normalized_email,
        v_account.delivery_email,
        v_account.display_name,
        v_account.platform_roles,
        floor(extract(epoch FROM v_account.created_at) * 1000)::bigint,
        floor(extract(epoch FROM v_account.updated_at) * 1000)::bigint,
        floor(extract(epoch FROM v_session_created_at) * 1000)::bigint,
        floor(extract(epoch FROM v_session_expires_at) * 1000)::bigint;
END
$$;

ALTER FUNCTION public.ple_complete_email_change_and_revoke_sessions(
    bytea, bytea, uuid, bytea, bigint
) OWNER TO ple_enrollment_broker;
