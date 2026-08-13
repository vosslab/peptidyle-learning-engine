-- Canonical human-role epoch: student, instructor, and sysadmin only.
--
-- Instructor authority is approved by provisioning a direct instructor
-- course membership. Sysadmin is an operator-approved account attribute and
-- never substitutes for course membership when reading FERPA records.

ALTER TABLE public.ple_account
    ADD COLUMN platform_roles jsonb NOT NULL DEFAULT '[]'::jsonb,
    ADD CONSTRAINT ple_account_platform_roles_check CHECK (
        platform_roles = '[]'::jsonb OR platform_roles = '["sysadmin"]'::jsonb
    );

-- The application auth role may read the operator decision and issue a
-- session, but compromised application SQL cannot grant that decision.
REVOKE INSERT, UPDATE ON TABLE public.ple_account FROM ple_auth;
GRANT INSERT (user_id, normalized_email, delivery_email, display_name)
    ON TABLE public.ple_account TO ple_auth;
GRANT UPDATE (normalized_email, delivery_email, display_name, updated_at)
    ON TABLE public.ple_account TO ple_auth;

-- This project has no production users. Existing development sessions are
-- intentionally discarded instead of retaining obsolete wire roles.
DELETE FROM public.auth_session;
ALTER TABLE public.auth_session DROP CONSTRAINT auth_session_roles_check;
ALTER TABLE public.auth_session
    ADD CONSTRAINT auth_session_roles_check CHECK (
        jsonb_typeof(roles) = 'array'
        AND jsonb_array_length(roles) > 0
        AND roles <@ '["student", "instructor", "sysadmin"]'::jsonb
    );

CREATE OR REPLACE FUNCTION public.ple_retention_authorize(
    p_session character,
    p_course uuid DEFAULT NULL::uuid,
    p_admin_only boolean DEFAULT false
) RETURNS boolean
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'pg_catalog', 'public'
    AS $$
DECLARE
    actor uuid;
    roles jsonb;
BEGIN
    PERFORM set_config('ple.session_hash', p_session, true);
    SELECT user_id, auth_session.roles INTO actor, roles
      FROM public.auth_session
     WHERE session_hash = p_session
       AND tenant_id = public.ple_current_tenant()
       AND revoked_at IS NULL
       AND expires_at > transaction_timestamp();
    IF actor IS NULL THEN
        RETURN false;
    END IF;
    IF p_course IS NOT NULL AND NOT EXISTS (
        SELECT 1 FROM public.course
         WHERE tenant_id = public.ple_current_tenant() AND course_id = p_course
    ) THEN
        RETURN false;
    END IF;
    IF roles @> '["sysadmin"]'::jsonb THEN
        RETURN true;
    END IF;
    IF p_admin_only OR p_course IS NULL THEN
        RETURN false;
    END IF;
    RETURN EXISTS (
        SELECT 1 FROM public.course_member
         WHERE tenant_id = public.ple_current_tenant()
           AND course_id = p_course
           AND user_id = actor
           AND role = 'instructor'
    );
END
$$;

CREATE OR REPLACE FUNCTION public.ple_course_appearance_actor(
    p_session character,
    p_course uuid,
    p_manager_only boolean DEFAULT false
) RETURNS uuid
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'pg_catalog', 'public'
    AS $$
DECLARE
    actor uuid;
    member_role text;
BEGIN
    PERFORM set_config('ple.session_hash', p_session, true);
    SELECT user_id INTO actor
      FROM public.auth_session
     WHERE session_hash = p_session
       AND tenant_id = public.ple_current_tenant()
       AND revoked_at IS NULL
       AND expires_at > transaction_timestamp();
    IF actor IS NULL OR NOT EXISTS (
        SELECT 1 FROM public.course
         WHERE tenant_id = public.ple_current_tenant() AND course_id = p_course
    ) THEN
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
    IF NOT p_manager_only
       AND member_role = 'student'
       AND public.ple_course_records_accessible(public.ple_current_tenant(), p_course) THEN
        RETURN actor;
    END IF;
    RETURN NULL;
END
$$;

-- Roster, export, and FERPA-bearing analytics require direct instructor
-- membership. Sysadmin status deliberately does not satisfy this function.
CREATE OR REPLACE FUNCTION public.ple_course_roster_actor(
    p_session character,
    p_course uuid,
    p_manager_only boolean DEFAULT true
) RETURNS uuid
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'pg_catalog', 'public'
    AS $$
DECLARE
    actor uuid;
    member_role text;
BEGIN
    PERFORM set_config('ple.session_hash', p_session, true);
    SELECT user_id INTO actor
      FROM public.auth_session
     WHERE session_hash = p_session
       AND tenant_id = public.ple_current_tenant()
       AND revoked_at IS NULL
       AND expires_at > transaction_timestamp();
    IF actor IS NULL OR NOT EXISTS (
        SELECT 1 FROM public.course
         WHERE tenant_id = public.ple_current_tenant() AND course_id = p_course
    ) THEN
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
    IF NOT p_manager_only
       AND member_role = 'student'
       AND public.ple_course_records_accessible(public.ple_current_tenant(), p_course) THEN
        RETURN actor;
    END IF;
    RETURN NULL;
END
$$;

-- A Sysadmin may cross only the narrow roster-support boundary needed to help
-- an Instructor. This does not confer course membership, gradebook, response,
-- run, export, item-analysis, or general course authority. Every successful
-- Sysadmin support operation is written to tenant audit evidence in the same
-- transaction before protected roster data is returned or changed.
CREATE FUNCTION public.ple_course_roster_support_actor(
    p_session character,
    p_course uuid,
    p_action text,
    p_audit boolean
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
    IF p_audit THEN
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
            encode(digest(convert_to(audit_payload::text, 'UTF8'), 'sha256'), 'hex')
        );
    END IF;
    RETURN actor;
END
$$;

REVOKE ALL ON FUNCTION public.ple_course_roster_support_actor(
    character, uuid, text, boolean
) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION public.ple_course_roster_support_actor(
    character, uuid, text, boolean
) TO ple_app;
