-- WP-PROF-LD2: bounded account discovery for normal Sysadmin approval.
--
-- The existing teaching-authority broker owns this projection. It returns
-- only an opaque account reference, a safe display label, and the closed
-- approval state needed by the approval UI; it never returns email, UUID,
-- tenant facts, or course relationships.

CREATE FUNCTION public.ple_sysadmin_instructor_candidate_search(
    p_session character(64), p_query text, p_after integer, p_limit integer
) RETURNS TABLE(
    account_public_id integer,
    account_display_name text,
    approval_state text,
    approval_revision bigint
)
LANGUAGE plpgsql STABLE SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public'
AS $$
DECLARE actor uuid;
DECLARE roles jsonb;
BEGIN
    SELECT session.user_id, session.roles INTO actor, roles
      FROM public.auth_session AS session
     WHERE session.session_hash = p_session
       AND session.tenant_id = public.ple_current_tenant()
       AND session.revoked_at IS NULL
       AND session.expires_at > transaction_timestamp();
    IF actor IS NULL OR NOT roles @> '["sysadmin"]'::jsonb THEN
        RAISE EXCEPTION 'sysadmin instructor candidate discovery is not authorized'
            USING ERRCODE = '42501';
    END IF;
    IF p_query IS NULL OR p_query <> btrim(p_query)
       OR char_length(p_query) NOT BETWEEN 2 AND 100
       OR p_after IS NOT NULL AND p_after <= 0
       OR p_limit NOT BETWEEN 2 AND 101 THEN
        RAISE EXCEPTION 'sysadmin instructor candidate discovery is invalid'
            USING ERRCODE = '22023';
    END IF;
    RETURN QUERY
    SELECT account.public_id,
           account.display_name,
           CASE
               WHEN approval.user_id IS NULL THEN 'unapproved'
               WHEN approval.revoked_at IS NULL THEN 'approved'
               ELSE 'revoked'
           END,
           approval.revision
      FROM public.ple_account AS account
      LEFT JOIN public.instructor_approval AS approval ON approval.user_id = account.user_id
     WHERE position(lower(p_query) IN lower(account.display_name)) > 0
       AND (p_after IS NULL OR account.public_id > p_after)
     ORDER BY account.public_id
     LIMIT p_limit;
END
$$;

ALTER FUNCTION public.ple_sysadmin_instructor_candidate_search(character, text, integer, integer)
    OWNER TO ple_teaching_authority_broker;
REVOKE ALL ON FUNCTION public.ple_sysadmin_instructor_candidate_search(
    character, text, integer, integer
) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION public.ple_sysadmin_instructor_candidate_search(
    character, text, integer, integer
) TO ple_app;

-- The auth role may read the completed baseline generation without receiving
-- lifecycle writes or broader lifecycle state.
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_roles WHERE rolname = 'ple_live_demo_installation_broker'
    ) THEN
        CREATE ROLE ple_live_demo_installation_broker
            NOLOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE
            NOINHERIT NOREPLICATION NOBYPASSRLS;
    END IF;
END
$$;

ALTER ROLE ple_live_demo_installation_broker
    NOLOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE
    NOINHERIT NOREPLICATION NOBYPASSRLS;

REVOKE ALL ON SCHEMA public FROM ple_live_demo_installation_broker;
GRANT USAGE ON SCHEMA public TO ple_live_demo_installation_broker;
GRANT SELECT ON TABLE public.live_demo_install_state TO ple_live_demo_installation_broker;

CREATE FUNCTION public.ple_completed_live_demo_installation_generation()
RETURNS uuid
LANGUAGE sql STABLE SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public'
AS $$
    SELECT installation_generation
      FROM public.live_demo_install_state
     WHERE singleton = true AND state = 'complete'
$$;

ALTER FUNCTION public.ple_completed_live_demo_installation_generation()
    OWNER TO ple_live_demo_installation_broker;
REVOKE ALL ON FUNCTION public.ple_completed_live_demo_installation_generation() FROM PUBLIC;
GRANT EXECUTE ON FUNCTION public.ple_completed_live_demo_installation_generation() TO ple_auth;
