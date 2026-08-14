-- PLE is pre-production.  Account presentation is authorized from the opaque
-- short-lived account-session hash, never a user ID supplied by a route.
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_roles WHERE rolname = 'ple_account_presentation_broker'
    ) THEN
        CREATE ROLE ple_account_presentation_broker
            NOLOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE NOINHERIT
            NOREPLICATION NOBYPASSRLS;
    END IF;
END
$$;
ALTER ROLE ple_account_presentation_broker
    NOLOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE NOINHERIT
    NOREPLICATION NOBYPASSRLS;
DO $$
BEGIN
    IF EXISTS (
        SELECT 1 FROM pg_auth_members AS membership
         WHERE membership.roleid = 'ple_account_presentation_broker'::regrole
            OR membership.member = 'ple_account_presentation_broker'::regrole
    ) THEN
        RAISE EXCEPTION 'ple_account_presentation_broker must not have role memberships';
    END IF;
END
$$;
REVOKE ALL ON SCHEMA public FROM ple_account_presentation_broker;
GRANT USAGE ON SCHEMA public TO ple_account_presentation_broker;

CREATE TABLE public.account_presentation_preference (
    user_id uuid PRIMARY KEY REFERENCES ple_account(user_id) ON DELETE CASCADE,
    contrast text NOT NULL DEFAULT 'standard'
        CHECK (contrast IN ('standard', 'increased')),
    updated_at timestamptz NOT NULL DEFAULT transaction_timestamp()
);

ALTER TABLE public.account_presentation_preference ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.account_presentation_preference FORCE ROW LEVEL SECURITY;

CREATE POLICY account_presentation_preference_broker
    ON public.account_presentation_preference
    TO ple_account_presentation_broker
    USING (true) WITH CHECK (true);

CREATE POLICY account_authentication_session_presentation_broker
    ON public.account_authentication_session
    FOR SELECT TO ple_account_presentation_broker
    USING (true);

GRANT SELECT, INSERT, UPDATE ON public.account_presentation_preference
    TO ple_account_presentation_broker;
GRANT SELECT ON public.account_authentication_session
    TO ple_account_presentation_broker;

CREATE FUNCTION public.ple_account_presentation_get(
    p_session_token_hash bytea
) RETURNS text
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'pg_catalog', 'public'
    AS $$
DECLARE
    account_user uuid;
    preference text;
BEGIN
    IF p_session_token_hash IS NULL OR octet_length(p_session_token_hash) <> 32 THEN
        RETURN NULL;
    END IF;
    SELECT session_row.user_id INTO account_user
      FROM public.account_authentication_session AS session_row
     WHERE session_row.token_hash = p_session_token_hash
       AND session_row.expires_at > transaction_timestamp();
    IF account_user IS NULL THEN
        RETURN NULL;
    END IF;
    SELECT preference_row.contrast INTO preference
      FROM public.account_presentation_preference AS preference_row
     WHERE preference_row.user_id = account_user;
    RETURN COALESCE(preference, 'standard');
END
$$;

CREATE FUNCTION public.ple_account_presentation_save(
    p_session_token_hash bytea,
    p_contrast text
) RETURNS text
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'pg_catalog', 'public'
    AS $$
DECLARE
    account_user uuid;
BEGIN
    IF p_session_token_hash IS NULL
       OR octet_length(p_session_token_hash) <> 32
       OR p_contrast IS NULL
       OR p_contrast NOT IN ('standard', 'increased') THEN
        RETURN NULL;
    END IF;
    SELECT session_row.user_id INTO account_user
      FROM public.account_authentication_session AS session_row
     WHERE session_row.token_hash = p_session_token_hash
       AND session_row.expires_at > transaction_timestamp();
    IF account_user IS NULL THEN
        RETURN NULL;
    END IF;
    INSERT INTO public.account_presentation_preference (user_id, contrast, updated_at)
    VALUES (account_user, p_contrast, transaction_timestamp())
    ON CONFLICT (user_id) DO UPDATE SET
        contrast = EXCLUDED.contrast,
        updated_at = transaction_timestamp();
    RETURN p_contrast;
END
$$;

ALTER FUNCTION public.ple_account_presentation_get(bytea)
    OWNER TO ple_account_presentation_broker;
ALTER FUNCTION public.ple_account_presentation_save(bytea, text)
    OWNER TO ple_account_presentation_broker;
REVOKE ALL ON FUNCTION public.ple_account_presentation_get(bytea) FROM PUBLIC;
REVOKE ALL ON FUNCTION public.ple_account_presentation_save(bytea, text) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION public.ple_account_presentation_get(bytea) TO ple_auth;
GRANT EXECUTE ON FUNCTION public.ple_account_presentation_save(bytea, text) TO ple_auth;

REVOKE ALL ON public.account_presentation_preference FROM ple_auth;
