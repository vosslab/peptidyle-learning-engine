BEGIN;

-- The application role presents a session capability; the broker derives the
-- actor and owns every lock and co-instructor invitation mutation.
ALTER ROLE ple_teaching_authority_broker
    NOLOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE NOINHERIT NOREPLICATION NOBYPASSRLS;

CREATE POLICY teaching_authority_course_lock
    ON public.course FOR UPDATE TO ple_teaching_authority_broker
    USING (tenant_id = public.ple_current_tenant())
    WITH CHECK (tenant_id = public.ple_current_tenant());
CREATE POLICY teaching_authority_member_lock
    ON public.course_member FOR UPDATE TO ple_teaching_authority_broker
    USING (tenant_id = public.ple_current_tenant())
    WITH CHECK (tenant_id = public.ple_current_tenant());
CREATE POLICY teaching_authority_invitation_insert
    ON public.course_instructor_invitation FOR INSERT TO ple_teaching_authority_broker
    WITH CHECK (tenant_id = public.ple_current_tenant());

GRANT SELECT, UPDATE (course_id) ON public.course TO ple_teaching_authority_broker;
GRANT SELECT, UPDATE (course_membership_id) ON public.course_member
    TO ple_teaching_authority_broker;
GRANT UPDATE (status, revoked_at) ON public.course_member
    TO ple_teaching_authority_broker;
GRANT SELECT, UPDATE (session_hash) ON public.auth_session TO ple_teaching_authority_broker;
GRANT SELECT, INSERT,
    UPDATE (status, accepted_at, declined_at, revoked_at, accepted_membership_id, revision)
    ON public.course_instructor_invitation TO ple_teaching_authority_broker;
GRANT USAGE, SELECT ON SEQUENCE public.course_instructor_invitation_public_id_seq
    TO ple_teaching_authority_broker;

CREATE OR REPLACE FUNCTION public.ple_accept_co_instructor_invitation_v1(
    p_tenant uuid,
    p_session character(64),
    p_invitation uuid,
    p_expected_revision bigint
) RETURNS TABLE (
    tenant_id uuid,
    actor_id uuid,
    course_id uuid,
    course_membership_id uuid,
    roster_revision bigint
)
LANGUAGE plpgsql VOLATILE SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public', pg_temp
AS $$
#variable_conflict use_column
DECLARE
    v_actor uuid;
    v_course uuid;
    v_target uuid;
    v_status text;
    v_revision bigint;
    v_invitation_revision bigint;
    v_membership uuid;
    v_role text;
BEGIN
    IF p_tenant IS NULL OR p_session IS NULL OR p_invitation IS NULL
       OR p_expected_revision IS NULL OR p_expected_revision < 1
       OR p_tenant IS DISTINCT FROM public.ple_current_tenant() THEN
        RAISE EXCEPTION 'co-instructor acceptance arguments are invalid' USING ERRCODE = '22023';
    END IF;
    PERFORM set_config('ple.session_hash', p_session, true);
    SELECT session_row.user_id INTO v_actor
      FROM public.auth_session AS session_row
     WHERE session_row.session_hash = p_session
       AND session_row.tenant_id = p_tenant
       AND session_row.revoked_at IS NULL
       AND session_row.expires_at > transaction_timestamp()
     FOR UPDATE;
    IF NOT FOUND THEN RETURN; END IF;
    SELECT course_row.course_id INTO v_course
      FROM public.course AS course_row
      JOIN public.course_instructor_invitation AS invitation
        ON invitation.tenant_id = course_row.tenant_id
       AND invitation.course_id = course_row.course_id
     WHERE course_row.tenant_id = p_tenant
       AND invitation.invitation_id = p_invitation
     FOR UPDATE OF course_row;
    IF NOT FOUND THEN RETURN; END IF;
    SELECT roster.revision INTO v_revision
      FROM public.course_roster_state AS roster
     WHERE roster.tenant_id = p_tenant AND roster.course_id = v_course
     FOR UPDATE;
    IF NOT FOUND OR v_revision < 1 THEN
        RAISE EXCEPTION 'course roster aggregate is invalid' USING ERRCODE = '55000';
    END IF;
    SELECT invitation.target_user_id, invitation.status, invitation.revision,
           invitation.accepted_membership_id
      INTO v_target, v_status, v_invitation_revision, v_membership
      FROM public.course_instructor_invitation AS invitation
     WHERE invitation.tenant_id = p_tenant
       AND invitation.invitation_id = p_invitation
     FOR UPDATE;
    IF v_target IS DISTINCT FROM v_actor THEN RETURN; END IF;
    IF v_status = 'accepted' THEN
        IF v_membership IS NULL THEN
            RAISE EXCEPTION 'co-instructor acceptance aggregate is invalid'
                USING ERRCODE = '55000';
        END IF;
        tenant_id := p_tenant;
        actor_id := v_actor;
        course_id := v_course;
        course_membership_id := v_membership;
        roster_revision := v_revision;
        RETURN NEXT;
        RETURN;
    END IF;
    IF v_status <> 'pending' OR v_invitation_revision <> p_expected_revision
       OR NOT EXISTS (
            SELECT 1 FROM public.course_instructor_invitation AS invitation
             WHERE invitation.tenant_id = p_tenant
               AND invitation.invitation_id = p_invitation
               AND invitation.expires_at > transaction_timestamp()
       ) OR NOT public.ple_lock_instructor_approval_eligibility(v_actor) THEN
        RAISE EXCEPTION 'co-instructor acceptance is unavailable' USING ERRCODE = '55000';
    END IF;
    SELECT member.course_membership_id, member.role INTO v_membership, v_role
      FROM public.course_member AS member
     WHERE member.tenant_id = p_tenant AND member.course_id = v_course
       AND member.user_id = v_actor AND member.status = 'active'
     FOR UPDATE;
    IF FOUND AND v_role <> 'instructor' THEN
        RAISE EXCEPTION 'co-instructor membership conflicts' USING ERRCODE = '55000';
    END IF;
    IF NOT FOUND THEN
        v_membership := gen_random_uuid();
        INSERT INTO public.course_member (
            tenant_id, course_id, course_membership_id, user_id,
            role, student_id, status, joined_at
        ) VALUES (
            p_tenant, v_course, v_membership, v_actor,
            'instructor', NULL, 'active', transaction_timestamp()
        );
    END IF;
    UPDATE public.course_instructor_invitation AS invitation
       SET status = 'accepted', accepted_at = transaction_timestamp(),
           accepted_membership_id = v_membership, revision = invitation.revision + 1
     WHERE invitation.tenant_id = p_tenant
       AND invitation.invitation_id = p_invitation
       AND invitation.status = 'pending'
       AND invitation.revision = p_expected_revision;
    IF NOT FOUND THEN
        RAISE EXCEPTION 'co-instructor acceptance is unavailable' USING ERRCODE = '55000';
    END IF;
    UPDATE public.course_roster_state AS roster
       SET revision = roster.revision + 1, updated_at = transaction_timestamp()
     WHERE roster.tenant_id = p_tenant AND roster.course_id = v_course
       AND roster.revision = v_revision
     RETURNING roster.revision INTO roster_revision;
    IF NOT FOUND THEN
        RAISE EXCEPTION 'course roster revision is unavailable' USING ERRCODE = '55000';
    END IF;
    tenant_id := p_tenant;
    actor_id := v_actor;
    course_id := v_course;
    course_membership_id := v_membership;
    RETURN NEXT;
END
$$;

CREATE FUNCTION public.ple_create_co_instructor_invitation_v1(
    p_tenant uuid,
    p_session character(64),
    p_course uuid,
    p_target uuid
) RETURNS TABLE (
    tenant_id uuid,
    actor_id uuid,
    invitation_id uuid,
    course_id uuid,
    target_user_id uuid,
    invited_by_membership_id uuid,
    created_at_millis bigint,
    expires_at_millis bigint,
    accepted_at_millis bigint,
    declined_at_millis bigint,
    revoked_at_millis bigint,
    status text,
    revision bigint
)
LANGUAGE plpgsql VOLATILE SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public', pg_temp
AS $$
#variable_conflict use_column
DECLARE
    v_actor uuid;
    v_membership uuid;
    v_invitation public.course_instructor_invitation%ROWTYPE;
BEGIN
    IF p_tenant IS NULL OR p_session IS NULL OR p_course IS NULL OR p_target IS NULL
       OR p_tenant IS DISTINCT FROM public.ple_current_tenant() THEN
        RAISE EXCEPTION 'co-instructor invitation arguments are invalid' USING ERRCODE = '22023';
    END IF;
    PERFORM set_config('ple.session_hash', p_session, true);
    SELECT session_row.user_id INTO v_actor
      FROM public.auth_session AS session_row
     WHERE session_row.session_hash = p_session
       AND session_row.tenant_id = p_tenant
       AND session_row.revoked_at IS NULL
       AND session_row.expires_at > transaction_timestamp()
     FOR UPDATE;
    IF NOT FOUND THEN RETURN; END IF;
    PERFORM 1 FROM public.course AS course_row
     WHERE course_row.tenant_id = p_tenant AND course_row.course_id = p_course
     FOR UPDATE;
    IF NOT FOUND THEN RETURN; END IF;
    SELECT member.course_membership_id INTO v_membership
      FROM public.course_member AS member
     WHERE member.tenant_id = p_tenant AND member.course_id = p_course
       AND member.user_id = v_actor AND member.role = 'instructor'
       AND member.status = 'active'
     FOR UPDATE;
    IF NOT FOUND THEN RETURN; END IF;
    IF NOT public.ple_lock_instructor_approval_eligibility(p_target)
       OR EXISTS (
            SELECT 1 FROM public.course_member AS target_member
             WHERE target_member.tenant_id = p_tenant
               AND target_member.course_id = p_course
               AND target_member.user_id = p_target
               AND target_member.status = 'active'
       ) THEN
        RETURN;
    END IF;
    INSERT INTO public.course_instructor_invitation (
        tenant_id, course_id, invitation_id, target_user_id, invited_by_membership_id
    ) VALUES (p_tenant, p_course, gen_random_uuid(), p_target, v_membership)
    ON CONFLICT (tenant_id, course_id, target_user_id) WHERE status = 'pending'
    DO NOTHING
    RETURNING * INTO v_invitation;
    IF NOT FOUND THEN
        SELECT invitation.* INTO v_invitation
          FROM public.course_instructor_invitation AS invitation
         WHERE invitation.tenant_id = p_tenant
           AND invitation.course_id = p_course
           AND invitation.target_user_id = p_target
           AND invitation.status = 'pending'
           AND invitation.expires_at > transaction_timestamp()
         FOR UPDATE;
        IF NOT FOUND THEN
            RAISE EXCEPTION 'co-instructor invitation aggregate is unavailable'
                USING ERRCODE = '55000';
        END IF;
    END IF;
    RETURN QUERY SELECT
        p_tenant, v_actor, v_invitation.invitation_id, v_invitation.course_id,
        v_invitation.target_user_id, v_invitation.invited_by_membership_id,
        floor(extract(epoch FROM v_invitation.created_at) * 1000)::bigint,
        floor(extract(epoch FROM v_invitation.expires_at) * 1000)::bigint,
        CASE WHEN v_invitation.accepted_at IS NULL THEN NULL ELSE
            floor(extract(epoch FROM v_invitation.accepted_at) * 1000)::bigint END,
        CASE WHEN v_invitation.declined_at IS NULL THEN NULL ELSE
            floor(extract(epoch FROM v_invitation.declined_at) * 1000)::bigint END,
        CASE WHEN v_invitation.revoked_at IS NULL THEN NULL ELSE
            floor(extract(epoch FROM v_invitation.revoked_at) * 1000)::bigint END,
        v_invitation.status, v_invitation.revision;
END
$$;

CREATE FUNCTION public.ple_revoke_co_instructor_invitation_v1(
    p_tenant uuid,
    p_session character(64),
    p_course uuid,
    p_invitation uuid,
    p_expected_revision bigint
) RETURNS TABLE (
    tenant_id uuid,
    actor_id uuid,
    course_id uuid,
    invitation_id uuid,
    revision bigint
)
LANGUAGE plpgsql VOLATILE SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public', pg_temp
AS $$
#variable_conflict use_column
DECLARE
    v_actor uuid;
    v_invitation public.course_instructor_invitation%ROWTYPE;
BEGIN
    IF p_tenant IS NULL OR p_session IS NULL OR p_course IS NULL
       OR p_invitation IS NULL OR p_expected_revision IS NULL OR p_expected_revision < 1
       OR p_tenant IS DISTINCT FROM public.ple_current_tenant() THEN
        RAISE EXCEPTION 'co-instructor revocation arguments are invalid' USING ERRCODE = '22023';
    END IF;
    PERFORM set_config('ple.session_hash', p_session, true);
    SELECT session_row.user_id INTO v_actor
      FROM public.auth_session AS session_row
     WHERE session_row.session_hash = p_session
       AND session_row.tenant_id = p_tenant
       AND session_row.revoked_at IS NULL
       AND session_row.expires_at > transaction_timestamp()
     FOR UPDATE;
    IF NOT FOUND THEN RETURN; END IF;
    PERFORM 1 FROM public.course AS course_row
     WHERE course_row.tenant_id = p_tenant AND course_row.course_id = p_course
     FOR UPDATE;
    IF NOT FOUND THEN RETURN; END IF;
    PERFORM 1 FROM public.course_member AS member
     WHERE member.tenant_id = p_tenant AND member.course_id = p_course
       AND member.user_id = v_actor AND member.role = 'instructor'
       AND member.status = 'active'
     FOR UPDATE;
    IF NOT FOUND THEN RETURN; END IF;
    SELECT invitation.* INTO v_invitation
      FROM public.course_instructor_invitation AS invitation
     WHERE invitation.tenant_id = p_tenant
       AND invitation.invitation_id = p_invitation
     FOR UPDATE;
    IF NOT FOUND OR v_invitation.course_id IS DISTINCT FROM p_course THEN RETURN; END IF;
    IF v_invitation.status <> 'pending'
       OR v_invitation.expires_at <= transaction_timestamp()
       OR v_invitation.revision <> p_expected_revision THEN
        RAISE EXCEPTION 'co-instructor invitation revision conflicts' USING ERRCODE = '23505';
    END IF;
    UPDATE public.course_instructor_invitation AS invitation
       SET status = 'revoked', revoked_at = transaction_timestamp(),
           revision = invitation.revision + 1
     WHERE invitation.tenant_id = p_tenant
       AND invitation.invitation_id = p_invitation
     RETURNING invitation.revision INTO revision;
    tenant_id := p_tenant;
    actor_id := v_actor;
    course_id := p_course;
    invitation_id := p_invitation;
    RETURN NEXT;
END
$$;

CREATE FUNCTION public.ple_decline_co_instructor_invitation_v1(
    p_tenant uuid,
    p_session character(64),
    p_invitation uuid,
    p_expected_revision bigint
) RETURNS TABLE (
    tenant_id uuid,
    actor_id uuid,
    course_id uuid,
    invitation_id uuid,
    revision bigint
)
LANGUAGE plpgsql VOLATILE SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public', pg_temp
AS $$
#variable_conflict use_column
DECLARE
    v_actor uuid;
    v_course uuid;
    v_invitation public.course_instructor_invitation%ROWTYPE;
BEGIN
    IF p_tenant IS NULL OR p_session IS NULL OR p_invitation IS NULL
       OR p_expected_revision IS NULL OR p_expected_revision < 1
       OR p_tenant IS DISTINCT FROM public.ple_current_tenant() THEN
        RAISE EXCEPTION 'co-instructor decline arguments are invalid' USING ERRCODE = '22023';
    END IF;
    PERFORM set_config('ple.session_hash', p_session, true);
    SELECT session_row.user_id INTO v_actor
      FROM public.auth_session AS session_row
     WHERE session_row.session_hash = p_session
       AND session_row.tenant_id = p_tenant
       AND session_row.revoked_at IS NULL
       AND session_row.expires_at > transaction_timestamp()
     FOR UPDATE;
    IF NOT FOUND THEN RETURN; END IF;
    SELECT invitation.course_id INTO v_course
      FROM public.course_instructor_invitation AS invitation
     WHERE invitation.tenant_id = p_tenant
       AND invitation.invitation_id = p_invitation;
    IF NOT FOUND THEN RETURN; END IF;
    PERFORM 1 FROM public.course AS course_row
     WHERE course_row.tenant_id = p_tenant AND course_row.course_id = v_course
     FOR UPDATE;
    IF NOT FOUND THEN RETURN; END IF;
    SELECT invitation.* INTO v_invitation
      FROM public.course_instructor_invitation AS invitation
     WHERE invitation.tenant_id = p_tenant
       AND invitation.invitation_id = p_invitation
     FOR UPDATE;
    IF NOT FOUND OR v_invitation.course_id IS DISTINCT FROM v_course
       OR v_invitation.target_user_id IS DISTINCT FROM v_actor THEN RETURN; END IF;
    IF v_invitation.status <> 'pending'
       OR v_invitation.expires_at <= transaction_timestamp()
       OR v_invitation.revision <> p_expected_revision THEN
        RAISE EXCEPTION 'co-instructor invitation revision conflicts' USING ERRCODE = '23505';
    END IF;
    UPDATE public.course_instructor_invitation AS invitation
       SET status = 'declined', declined_at = transaction_timestamp(),
           revision = invitation.revision + 1
     WHERE invitation.tenant_id = p_tenant
       AND invitation.invitation_id = p_invitation
     RETURNING invitation.revision INTO revision;
    tenant_id := p_tenant;
    actor_id := v_actor;
    course_id := v_course;
    invitation_id := p_invitation;
    RETURN NEXT;
END
$$;

-- Direct-instructor removal is a live course roster transition.  The broker
-- derives the actor from its active session and locks the whole instructor
-- set in membership-ID order before it counts and changes that set.  This
-- gives concurrent removals one stable serial order and keeps a final active
-- Instructor in the course.
CREATE FUNCTION public.ple_remove_direct_instructor_membership_v1(
    p_tenant uuid,
    p_session character(64),
    p_course uuid,
    p_membership uuid,
    p_expected_roster_revision bigint
) RETURNS TABLE (
    tenant_id uuid,
    actor_id uuid,
    course_id uuid,
    course_membership_id uuid,
    roster_revision bigint
)
LANGUAGE plpgsql VOLATILE SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public', pg_temp
AS $$
#variable_conflict use_column
DECLARE
    v_actor uuid;
    v_revision bigint;
    v_instructor_count bigint;
    v_target_user uuid;
BEGIN
    IF p_tenant IS NULL OR p_session IS NULL OR p_course IS NULL
       OR p_membership IS NULL OR p_expected_roster_revision IS NULL
       OR p_expected_roster_revision < 1
       OR p_tenant IS DISTINCT FROM public.ple_current_tenant() THEN
        RAISE EXCEPTION 'direct instructor removal arguments are invalid'
            USING ERRCODE = '22023';
    END IF;

    PERFORM set_config('ple.session_hash', p_session, true);
    SELECT session_row.user_id INTO v_actor
      FROM public.auth_session AS session_row
     WHERE session_row.session_hash = p_session
       AND session_row.tenant_id = p_tenant
       AND session_row.revoked_at IS NULL
       AND session_row.expires_at > transaction_timestamp()
     FOR UPDATE;
    IF NOT FOUND THEN RETURN; END IF;

    PERFORM 1 FROM public.course AS course_row
     WHERE course_row.tenant_id = p_tenant AND course_row.course_id = p_course
     FOR UPDATE;
    IF NOT FOUND THEN RETURN; END IF;

    SELECT roster.revision INTO v_revision
      FROM public.course_roster_state AS roster
     WHERE roster.tenant_id = p_tenant AND roster.course_id = p_course
     FOR UPDATE;
    IF NOT FOUND OR v_revision < 1 THEN
        RAISE EXCEPTION 'course roster aggregate is invalid' USING ERRCODE = '55000';
    END IF;
    IF v_revision <> p_expected_roster_revision THEN
        RAISE EXCEPTION 'course roster revision conflicts' USING ERRCODE = '55000';
    END IF;

    PERFORM 1
      FROM public.course_member AS member
     WHERE member.tenant_id = p_tenant
       AND member.course_id = p_course
       AND member.role = 'instructor'
       AND member.status = 'active'
     ORDER BY member.course_membership_id
     FOR UPDATE;

    PERFORM 1
      FROM public.course_member AS member
     WHERE member.tenant_id = p_tenant
       AND member.course_id = p_course
       AND member.user_id = v_actor
       AND member.role = 'instructor'
       AND member.status = 'active';
    IF NOT FOUND THEN RETURN; END IF;

    SELECT member.user_id INTO v_target_user
      FROM public.course_member AS member
     WHERE member.tenant_id = p_tenant
       AND member.course_id = p_course
       AND member.course_membership_id = p_membership
       AND member.role = 'instructor'
       AND member.status = 'active';
    IF NOT FOUND THEN RETURN; END IF;

    SELECT count(*) INTO v_instructor_count
      FROM public.course_member AS member
     WHERE member.tenant_id = p_tenant
       AND member.course_id = p_course
       AND member.role = 'instructor'
       AND member.status = 'active';
    IF v_instructor_count < 2 THEN
        RAISE EXCEPTION 'the final active instructor cannot be removed'
            USING ERRCODE = '55000';
    END IF;

    UPDATE public.course_member AS member
       SET status = 'revoked', revoked_at = transaction_timestamp()
     WHERE member.tenant_id = p_tenant
       AND member.course_id = p_course
       AND member.course_membership_id = p_membership
       AND member.user_id = v_target_user
       AND member.role = 'instructor'
       AND member.status = 'active';
    IF NOT FOUND THEN
        RAISE EXCEPTION 'direct instructor membership transition is unavailable'
            USING ERRCODE = '55000';
    END IF;

    UPDATE public.course_roster_state AS roster
       SET revision = roster.revision + 1, updated_at = transaction_timestamp()
     WHERE roster.tenant_id = p_tenant
       AND roster.course_id = p_course
       AND roster.revision = v_revision
     RETURNING roster.revision INTO roster_revision;
    IF NOT FOUND THEN
        RAISE EXCEPTION 'course roster revision is unavailable' USING ERRCODE = '55000';
    END IF;

    tenant_id := p_tenant;
    actor_id := v_actor;
    course_id := p_course;
    course_membership_id := p_membership;
    RETURN NEXT;
END
$$;

ALTER FUNCTION public.ple_create_co_instructor_invitation_v1(uuid, character, uuid, uuid)
    OWNER TO ple_teaching_authority_broker;
ALTER FUNCTION public.ple_revoke_co_instructor_invitation_v1(uuid, character, uuid, uuid, bigint)
    OWNER TO ple_teaching_authority_broker;
ALTER FUNCTION public.ple_decline_co_instructor_invitation_v1(uuid, character, uuid, bigint)
    OWNER TO ple_teaching_authority_broker;
ALTER FUNCTION public.ple_remove_direct_instructor_membership_v1(
    uuid, character, uuid, uuid, bigint
) OWNER TO ple_teaching_authority_broker;
REVOKE ALL ON FUNCTION public.ple_create_co_instructor_invitation_v1(
    uuid, character, uuid, uuid
) FROM PUBLIC;
REVOKE ALL ON FUNCTION public.ple_revoke_co_instructor_invitation_v1(
    uuid, character, uuid, uuid, bigint
) FROM PUBLIC;
REVOKE ALL ON FUNCTION public.ple_decline_co_instructor_invitation_v1(
    uuid, character, uuid, bigint
) FROM PUBLIC;
REVOKE ALL ON FUNCTION public.ple_remove_direct_instructor_membership_v1(
    uuid, character, uuid, uuid, bigint
) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION public.ple_create_co_instructor_invitation_v1(
    uuid, character, uuid, uuid
) TO ple_app;
GRANT EXECUTE ON FUNCTION public.ple_revoke_co_instructor_invitation_v1(
    uuid, character, uuid, uuid, bigint
) TO ple_app;
GRANT EXECUTE ON FUNCTION public.ple_decline_co_instructor_invitation_v1(
    uuid, character, uuid, bigint
) TO ple_app;
GRANT EXECUTE ON FUNCTION public.ple_remove_direct_instructor_membership_v1(
    uuid, character, uuid, uuid, bigint
) TO ple_app;

REVOKE INSERT, UPDATE, DELETE ON public.course_instructor_invitation FROM ple_app;
REVOKE USAGE, SELECT, UPDATE ON SEQUENCE public.course_instructor_invitation_public_id_seq
    FROM ple_app;

DO $$
BEGIN
    IF EXISTS (
        SELECT 1 FROM pg_roles
         WHERE rolname = 'ple_teaching_authority_broker'
           AND (rolcanlogin OR rolsuper OR rolcreatedb OR rolcreaterole
                OR rolinherit OR rolreplication OR rolbypassrls)
    ) OR has_table_privilege(
        'ple_app', 'public.course_instructor_invitation', 'INSERT,UPDATE,DELETE'
    ) OR has_sequence_privilege(
        'ple_app', 'public.course_instructor_invitation_public_id_seq', 'USAGE'
    ) OR has_sequence_privilege(
        'ple_app', 'public.course_instructor_invitation_public_id_seq', 'SELECT'
    ) OR has_sequence_privilege(
        'ple_app', 'public.course_instructor_invitation_public_id_seq', 'UPDATE'
    ) OR NOT has_table_privilege(
        'ple_teaching_authority_broker', 'public.course_instructor_invitation', 'SELECT,INSERT'
    ) OR NOT has_column_privilege(
        'ple_teaching_authority_broker', 'public.course', 'course_id', 'UPDATE'
    ) OR NOT has_column_privilege(
        'ple_teaching_authority_broker', 'public.course_member', 'course_membership_id', 'UPDATE'
    ) OR NOT has_column_privilege(
        'ple_teaching_authority_broker', 'public.course_member', 'status', 'UPDATE'
    ) OR NOT has_column_privilege(
        'ple_teaching_authority_broker', 'public.course_member', 'revoked_at', 'UPDATE'
    ) OR NOT has_column_privilege(
        'ple_teaching_authority_broker', 'public.auth_session', 'session_hash', 'UPDATE'
    ) OR NOT has_sequence_privilege(
        'ple_teaching_authority_broker',
        'public.course_instructor_invitation_public_id_seq', 'USAGE'
    ) OR has_function_privilege(
        'public',
        'public.ple_create_co_instructor_invitation_v1(uuid,character,uuid,uuid)'::regprocedure,
        'EXECUTE'
    ) OR has_function_privilege(
        'public',
        'public.ple_revoke_co_instructor_invitation_v1(uuid,character,uuid,uuid,bigint)'::regprocedure,
        'EXECUTE'
    ) OR has_function_privilege(
        'public',
        'public.ple_decline_co_instructor_invitation_v1(uuid,character,uuid,bigint)'::regprocedure,
        'EXECUTE'
    ) OR has_function_privilege(
        'public',
        'public.ple_remove_direct_instructor_membership_v1(uuid,character,uuid,uuid,bigint)'::regprocedure,
        'EXECUTE'
    ) OR NOT has_function_privilege(
        'ple_app',
        'public.ple_remove_direct_instructor_membership_v1(uuid,character,uuid,uuid,bigint)'::regprocedure,
        'EXECUTE'
    ) THEN
        RAISE EXCEPTION 'teaching invitation mutator authority catalog is unsafe';
    END IF;
END
$$;

COMMIT;
