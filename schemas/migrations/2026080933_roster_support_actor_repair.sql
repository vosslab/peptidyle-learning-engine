-- Forward repair for the roster-support capability introduced in 0928.
--
-- The applied migration is immutable. Recreate its function with PostgreSQL's
-- built-in SHA-256 primitive, and move the SECURITY DEFINER capability from
-- the migration owner to a dedicated least-authority, RLS-obeying broker.

DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'ple_roster_support_broker') THEN
        CREATE ROLE ple_roster_support_broker NOLOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE
            NOINHERIT NOBYPASSRLS;
    END IF;
END
$$;

ALTER ROLE ple_roster_support_broker NOLOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE
    NOINHERIT NOBYPASSRLS;

REVOKE ALL ON SCHEMA public FROM ple_roster_support_broker;
GRANT USAGE ON SCHEMA public TO ple_roster_support_broker;

-- The pre-lock probe has no action parameter and cannot produce a roster
-- mutation or audit event. The audited actor below is the only Sysadmin roster
-- support capability.
CREATE FUNCTION public.ple_course_roster_support_precheck(
    p_session character,
    p_course uuid
) RETURNS uuid
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'pg_catalog', 'public'
    AS $$
DECLARE
    actor uuid;
    roles jsonb;
    member_role text;
BEGIN
    PERFORM set_config('ple.session_hash', p_session, true);
    SELECT user_id, auth_session.roles INTO actor, roles
      FROM public.auth_session
     WHERE session_hash = p_session
       AND tenant_id = public.ple_current_tenant()
       AND revoked_at IS NULL
       AND expires_at > transaction_timestamp();
    IF actor IS NULL
       OR NOT public.ple_course_records_accessible(public.ple_current_tenant(), p_course) THEN
        RETURN NULL;
    END IF;
    SELECT role INTO member_role
      FROM public.course_member
     WHERE tenant_id = public.ple_current_tenant()
       AND course_id = p_course
       AND user_id = actor;
    IF member_role = 'instructor' THEN
        RETURN actor;
    END IF;
    IF NOT roles @> '["sysadmin"]'::jsonb THEN
        RETURN NULL;
    END IF;
    RETURN actor;
END
$$;

CREATE FUNCTION public.ple_course_roster_support_actor(
    p_session character,
    p_course uuid,
    p_action text
) RETURNS uuid
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'pg_catalog', 'public'
    AS $$
DECLARE
    actor uuid;
    roles jsonb;
    member_role text;
    audit_payload jsonb;
BEGIN
    IF p_action NOT IN (
        'listRoster',
        'createInvitation',
        'replaceEnrollmentPolicy',
        'revokeMember',
        'revokeInvitation',
        'stageImport',
        'commitImport'
    ) THEN
        RETURN NULL;
    END IF;
    PERFORM set_config('ple.session_hash', p_session, true);
    SELECT user_id, auth_session.roles INTO actor, roles
      FROM public.auth_session
     WHERE session_hash = p_session
       AND tenant_id = public.ple_current_tenant()
       AND revoked_at IS NULL
       AND expires_at > transaction_timestamp();
    IF actor IS NULL
       OR NOT public.ple_course_records_accessible(public.ple_current_tenant(), p_course) THEN
        RETURN NULL;
    END IF;
    SELECT role INTO member_role
      FROM public.course_member
     WHERE tenant_id = public.ple_current_tenant()
       AND course_id = p_course
       AND user_id = actor
     FOR KEY SHARE;
    IF member_role = 'instructor' THEN
        RETURN actor;
    END IF;
    IF NOT roles @> '["sysadmin"]'::jsonb THEN
        RETURN NULL;
    END IF;
    audit_payload := jsonb_build_object('supportAction', p_action);
    INSERT INTO public.audit_event (
            tenant_id,
            audit_event_id,
            occurred_at,
            actor_id,
            course_id,
            action,
            target_kind,
            target_id,
            payload,
            payload_sha256
        ) VALUES (
            public.ple_current_tenant(),
            gen_random_uuid(),
            transaction_timestamp(),
            actor,
            p_course,
            'sysadmin.rosterSupport',
            'courseRoster',
            p_course,
            audit_payload,
            encode(pg_catalog.sha256(convert_to(audit_payload::text,'UTF8')),'hex')
        );
    RETURN actor;
END
$$;

ALTER FUNCTION public.ple_course_roster_support_precheck(character, uuid)
    OWNER TO ple_roster_support_broker;
ALTER FUNCTION public.ple_course_roster_support_actor(character, uuid, text)
    OWNER TO ple_roster_support_broker;

-- The owner has no broad application role. Its direct privileges exactly cover
-- the session, course-membership, lifecycle, and audit work in the function.
GRANT SELECT ON TABLE public.auth_session, public.course, public.course_member
    TO ple_roster_support_broker;
-- PostgreSQL requires UPDATE privilege for SELECT ... FOR KEY SHARE. The
-- broker receives no general update capability.
GRANT UPDATE (tenant_id) ON TABLE public.course_member TO ple_roster_support_broker;
GRANT INSERT ON TABLE public.audit_event TO ple_roster_support_broker;
GRANT EXECUTE ON FUNCTION public.ple_current_tenant() TO ple_roster_support_broker;
GRANT EXECUTE ON FUNCTION public.ple_course_records_accessible(uuid, uuid)
    TO ple_roster_support_broker;

REVOKE ALL ON FUNCTION public.ple_course_roster_support_actor(character, uuid, text, boolean)
    FROM PUBLIC;
REVOKE ALL ON FUNCTION public.ple_course_roster_support_actor(character, uuid, text, boolean)
    FROM ple_app;
DROP FUNCTION public.ple_course_roster_support_actor(character, uuid, text, boolean);

REVOKE ALL ON FUNCTION public.ple_course_roster_support_precheck(character, uuid)
    FROM PUBLIC;
GRANT EXECUTE ON FUNCTION public.ple_course_roster_support_precheck(character, uuid)
    TO ple_app;
REVOKE ALL ON FUNCTION public.ple_course_roster_support_actor(character, uuid, text)
    FROM PUBLIC;
GRANT EXECUTE ON FUNCTION public.ple_course_roster_support_actor(character, uuid, text)
    TO ple_app;
