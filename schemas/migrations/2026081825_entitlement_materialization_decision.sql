BEGIN;

-- Replace the exception-only entitlement preparation seam with an explicit
-- decision. Expected current-policy denial remains distinct from invalid
-- actor authority and infrastructure privilege failures.
REVOKE ALL ON FUNCTION public.ple_prepare_entitlement_materialization(
    uuid, uuid, uuid, uuid, text, uuid
) FROM PUBLIC, ple_app, ple_grader, ple_grading_reader;
DROP FUNCTION public.ple_prepare_entitlement_materialization(
    uuid, uuid, uuid, uuid, text, uuid
);

CREATE FUNCTION public.ple_prepare_entitlement_materialization(
    p_tenant uuid,
    p_course uuid,
    p_assignment uuid,
    p_learner uuid,
    p_authority_kind text,
    p_actor uuid
) RETURNS TABLE (
    decision_kind text,
    tenant_id uuid,
    course_id uuid,
    assignment_id uuid,
    authority_kind text,
    actor_id uuid,
    authority_membership_id uuid,
    learner_id uuid,
    student_membership_id uuid,
    assignment_revision bigint,
    assignment_lifecycle text,
    audience_kind text,
    locked_audience_count bigint,
    locked_audience_group_ids uuid[],
    locked_current_group_count bigint,
    locked_current_group_ids uuid[],
    existing_enrollment_id uuid
)
LANGUAGE plpgsql VOLATILE SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public', pg_temp
AS $$
#variable_conflict error
DECLARE
    v_student_membership uuid;
BEGIN
    IF p_tenant IS NULL OR p_course IS NULL OR p_assignment IS NULL
       OR p_learner IS NULL OR p_actor IS NULL
       OR p_tenant IS DISTINCT FROM public.ple_current_tenant()
       OR p_authority_kind NOT IN ('student_self_service', 'direct_instructor')
       OR (p_authority_kind = 'student_self_service'
           AND p_actor IS DISTINCT FROM p_learner) THEN
        PERFORM public.ple_learner_work_deny_internal();
    END IF;

    -- Establish actor authority without discovering the target learner.
    PERFORM public.ple_learner_work_probe_authority_internal(
        p_tenant,
        p_course,
        p_actor,
        p_authority_kind
    );

    -- Match the canonical learner-work lock prefix. Course-member mutations
    -- take the course lock, so target absence remains stable through the
    -- decision and cannot race a concurrent roster transition.
    PERFORM 1
      FROM public.course AS course_row
     WHERE course_row.tenant_id = p_tenant
       AND course_row.course_id = p_course
     FOR UPDATE;
    IF NOT FOUND OR NOT public.ple_course_records_accessible(p_tenant, p_course) THEN
        PERFORM public.ple_learner_work_deny_internal();
    END IF;

    PERFORM pg_advisory_xact_lock(
        hashtextextended(p_tenant::text || ':' || p_assignment::text, 0)
    );
    PERFORM 1
      FROM public.assignment AS assignment_row
     WHERE assignment_row.tenant_id = p_tenant
       AND assignment_row.course_id = p_course
       AND assignment_row.assignment_id = p_assignment
     FOR UPDATE;
    IF NOT FOUND THEN
        PERFORM public.ple_learner_work_deny_internal();
    END IF;

    SELECT member.course_membership_id INTO v_student_membership
      FROM public.course_member AS member
     WHERE member.tenant_id = p_tenant
       AND member.course_id = p_course
       AND member.user_id = p_learner
       AND member.role = 'student'
       AND member.status = 'active';
    IF NOT FOUND THEN
        decision_kind := 'learner_not_active_course_student';
        RETURN NEXT;
        RETURN;
    END IF;

    RETURN QUERY
    SELECT 'granted'::text,
           prepared.tenant_id,
           prepared.course_id,
           prepared.assignment_id,
           prepared.authority_kind,
           prepared.actor_id,
           prepared.authority_membership_id,
           prepared.learner_id,
           prepared.student_membership_id,
           prepared.assignment_revision,
           prepared.assignment_lifecycle,
           prepared.audience_kind,
           prepared.locked_audience_count,
           prepared.locked_audience_group_ids,
           prepared.locked_current_group_count,
           prepared.locked_current_group_ids,
           prepared.existing_enrollment_id
      FROM public.ple_learner_work_prepare_internal(
          p_tenant,
          p_course,
          p_assignment,
          p_learner,
          p_actor,
          p_authority_kind,
          NULL,
          NULL,
          NULL
      ) AS prepared;
END
$$;

ALTER FUNCTION public.ple_prepare_entitlement_materialization(
    uuid, uuid, uuid, uuid, text, uuid
) OWNER TO ple_learner_work_broker;

REVOKE ALL ON FUNCTION public.ple_prepare_entitlement_materialization(
    uuid, uuid, uuid, uuid, text, uuid
) FROM PUBLIC, ple_app, ple_grader, ple_grading_reader;
GRANT EXECUTE ON FUNCTION public.ple_prepare_entitlement_materialization(
    uuid, uuid, uuid, uuid, text, uuid
) TO ple_app;

DO $$
DECLARE
    decision_function regprocedure :=
        'public.ple_prepare_entitlement_materialization(uuid,uuid,uuid,uuid,text,uuid)'::regprocedure;
BEGIN
    IF NOT EXISTS (
        SELECT 1
          FROM pg_proc AS procedure
         WHERE procedure.oid = decision_function
           AND procedure.proowner = 'ple_learner_work_broker'::regrole
           AND procedure.prosecdef
           AND procedure.provolatile = 'v'
           AND procedure.proconfig = ARRAY['search_path=pg_catalog, public, pg_temp']
    ) THEN
        RAISE EXCEPTION 'entitlement materialization decision function is unsafe';
    END IF;

    IF has_function_privilege('public', decision_function, 'EXECUTE')
       OR has_function_privilege('ple_grader', decision_function, 'EXECUTE')
       OR has_function_privilege('ple_grading_reader', decision_function, 'EXECUTE')
       OR NOT has_function_privilege('ple_app', decision_function, 'EXECUTE') THEN
        RAISE EXCEPTION 'entitlement materialization decision grants are unsafe';
    END IF;

    IF EXISTS (
        SELECT 1
          FROM pg_auth_members AS membership
         WHERE membership.roleid = 'ple_learner_work_broker'::regrole
            OR membership.member = 'ple_learner_work_broker'::regrole
    ) THEN
        RAISE EXCEPTION 'entitlement materialization broker has a membership edge';
    END IF;
END
$$;

COMMIT;
