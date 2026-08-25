-- The session-bound purpose-policy broker returns names shared by the stored
-- aggregate. Keep PL/pgSQL's strict ambiguity policy and bind every stored
-- column through an explicit table alias.

BEGIN;

CREATE OR REPLACE FUNCTION public.ple_replace_course_group_purpose_policy_v1(
    p_tenant uuid,
    p_session character(64),
    p_course uuid,
    p_purpose text,
    p_multiple_membership text,
    p_expected_revision bigint
) RETURNS TABLE(
    tenant_id uuid,
    actor_id uuid,
    course_id uuid,
    purpose text,
    multiple_membership text,
    revision bigint
) LANGUAGE plpgsql VOLATILE SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public', pg_temp AS $$
#variable_conflict error
DECLARE
    v_actor uuid;
    v_roles jsonb;
    v_policy_count bigint;
    v_stored_revision bigint;
    v_next_revision bigint;
BEGIN
    IF p_tenant IS NULL OR p_session IS NULL OR p_course IS NULL
       OR p_purpose NOT IN ('section', 'lab', 'cohort', 'accommodation', 'work')
       OR p_multiple_membership NOT IN ('allow', 'warn')
       OR p_expected_revision IS NULL OR p_expected_revision < 1
       OR p_tenant IS DISTINCT FROM public.ple_current_tenant() THEN
        RAISE EXCEPTION 'course group purpose policy arguments are invalid' USING ERRCODE='22023';
    END IF;

    PERFORM set_config('ple.session_hash', p_session, true);
    SELECT session_row.user_id, session_row.roles INTO v_actor, v_roles
      FROM public.auth_session AS session_row
     WHERE session_row.session_hash = p_session
       AND session_row.tenant_id = p_tenant
       AND session_row.revoked_at IS NULL
       AND session_row.expires_at > transaction_timestamp()
     FOR UPDATE;
    IF NOT FOUND OR v_actor IS NULL OR NOT v_roles @> '["instructor"]'::jsonb THEN
        RAISE EXCEPTION 'course group purpose policy is unavailable' USING ERRCODE='42501';
    END IF;

    -- Canonical aggregate order: course, direct-Instructor membership, then
    -- all five purpose rows in their closed lexical order.
    PERFORM 1
      FROM public.course AS course_row
     WHERE course_row.tenant_id = p_tenant
       AND course_row.course_id = p_course
     FOR UPDATE;
    IF NOT FOUND THEN
        RAISE EXCEPTION 'course group purpose policy is unavailable' USING ERRCODE='42501';
    END IF;
    PERFORM 1
      FROM public.course_member AS member_row
     WHERE member_row.tenant_id = p_tenant
       AND member_row.course_id = p_course
       AND member_row.user_id = v_actor
       AND member_row.role = 'instructor'
       AND member_row.status = 'active'
     ORDER BY member_row.course_membership_id
     FOR UPDATE;
    IF NOT FOUND OR NOT public.ple_course_records_accessible(p_tenant, p_course) THEN
        RAISE EXCEPTION 'course group purpose policy is unavailable' USING ERRCODE='42501';
    END IF;
    PERFORM 1
      FROM public.course_group_membership_policy AS policy_row
     WHERE policy_row.tenant_id = p_tenant
       AND policy_row.course_id = p_course
     ORDER BY policy_row.purpose
     FOR UPDATE;
    SELECT count(*) INTO v_policy_count
      FROM public.course_group_membership_policy AS policy_row
     WHERE policy_row.tenant_id = p_tenant
       AND policy_row.course_id = p_course;
    IF v_policy_count <> 5
       OR (SELECT count(*)
             FROM public.course_group_membership_policy AS policy_row
            WHERE policy_row.tenant_id = p_tenant
              AND policy_row.course_id = p_course
              AND policy_row.purpose IN (
                  'section', 'lab', 'cohort', 'accommodation', 'work'
              )) <> 5 THEN
        RAISE EXCEPTION 'course group purpose policy aggregate is invalid' USING ERRCODE='55000';
    END IF;
    SELECT policy_row.revision INTO v_stored_revision
      FROM public.course_group_membership_policy AS policy_row
     WHERE policy_row.tenant_id = p_tenant
       AND policy_row.course_id = p_course
       AND policy_row.purpose = p_purpose;
    IF NOT FOUND THEN
        RAISE EXCEPTION 'course group purpose policy aggregate is invalid' USING ERRCODE='55000';
    END IF;
    IF v_stored_revision <> p_expected_revision THEN
        RAISE EXCEPTION 'course group purpose policy revision conflict' USING ERRCODE='55000';
    END IF;

    UPDATE public.course_group_membership_policy AS policy_row
       SET multiple_membership = p_multiple_membership,
           revision = policy_row.revision + 1,
           updated_at = transaction_timestamp()
     WHERE policy_row.tenant_id = p_tenant
       AND policy_row.course_id = p_course
       AND policy_row.purpose = p_purpose
       AND policy_row.revision = p_expected_revision
     RETURNING policy_row.revision INTO v_next_revision;
    IF NOT FOUND OR v_next_revision < 1 THEN
        RAISE EXCEPTION 'course group purpose policy revision conflict' USING ERRCODE='55000';
    END IF;

    tenant_id := p_tenant;
    actor_id := v_actor;
    course_id := p_course;
    purpose := p_purpose;
    multiple_membership := p_multiple_membership;
    revision := v_next_revision;
    RETURN NEXT;
END
$$;

ALTER FUNCTION public.ple_replace_course_group_purpose_policy_v1(
    uuid, character(64), uuid, text, text, bigint
) OWNER TO ple_course_group_mutator_broker;
REVOKE ALL ON FUNCTION public.ple_replace_course_group_purpose_policy_v1(
    uuid, character(64), uuid, text, text, bigint
) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION public.ple_replace_course_group_purpose_policy_v1(
    uuid, character(64), uuid, text, text, bigint
) TO ple_app;

DO $$
BEGIN
    IF EXISTS (
        SELECT 1
          FROM pg_proc AS procedure_row
         WHERE procedure_row.oid =
               'public.ple_replace_course_group_purpose_policy_v1(uuid,character(64),uuid,text,text,bigint)'::regprocedure
           AND (
               procedure_row.proowner <> 'ple_course_group_mutator_broker'::regrole
               OR NOT procedure_row.prosecdef
               OR NOT coalesce(procedure_row.proconfig, ARRAY[]::text[])
                      @> ARRAY['search_path=pg_catalog, public, pg_temp']
           )
    )
       OR NOT has_function_privilege(
           'ple_app',
           'public.ple_replace_course_group_purpose_policy_v1(uuid,character(64),uuid,text,text,bigint)'::regprocedure,
           'EXECUTE'
       )
       OR has_function_privilege(
           'public',
           'public.ple_replace_course_group_purpose_policy_v1(uuid,character(64),uuid,text,text,bigint)'::regprocedure,
           'EXECUTE'
       ) THEN
        RAISE EXCEPTION 'course group purpose policy broker catalog is unsafe';
    END IF;
END
$$;

COMMIT;
