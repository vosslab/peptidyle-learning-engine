-- WP-INST-G2 / G2-W4A: exact grading-operation selection for Gradebook navigation.
--
-- The Gradebook resolves a public operation reference through the existing
-- Instructor grading-operation broker. The application keeps execute-only
-- authority and receives only the operation family plus public route IDs.

BEGIN;

CREATE FUNCTION public.ple_resolve_instructor_grading_operation_v1(
    p_tenant_id uuid,
    p_session character(64),
    p_course_id uuid,
    p_operation_reference integer
) RETURNS TABLE (
    target_kind text,
    assignment_reference integer,
    membership_reference integer
)
LANGUAGE plpgsql
VOLATILE
SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public', pg_temp
AS $$
DECLARE
    v_assignment_id uuid;
    v_target_kind text;
    v_assignment_reference integer;
    v_membership_reference integer;
BEGIN
    IF p_tenant_id IS NULL OR p_session IS NULL OR p_course_id IS NULL
       OR p_operation_reference IS NULL OR p_operation_reference <= 0
       OR p_tenant_id IS DISTINCT FROM public.ple_current_tenant()
    THEN
        RETURN;
    END IF;

    SELECT operation_row.assignment_id,
           operation_row.target_kind,
           assignment_row.public_id,
           member.public_id
      INTO v_assignment_id,
           v_target_kind,
           v_assignment_reference,
           v_membership_reference
      FROM public.grading_operation AS operation_row
      JOIN public.assignment AS assignment_row
        ON assignment_row.tenant_id = operation_row.tenant_id
       AND assignment_row.course_id = operation_row.course_id
       AND assignment_row.assignment_id = operation_row.assignment_id
      LEFT JOIN public.question_attempt AS attempt
        ON attempt.tenant_id = operation_row.tenant_id
       AND attempt.attempt_id = operation_row.attempt_id
      LEFT JOIN public.assignment_run AS run
        ON run.tenant_id = attempt.tenant_id
       AND run.run_id = attempt.run_id
      LEFT JOIN public.enrollment AS enrollment
        ON enrollment.tenant_id = run.tenant_id
       AND enrollment.enrollment_id = run.enrollment_id
      LEFT JOIN public.course_member AS member
        ON member.tenant_id = enrollment.tenant_id
       AND member.course_id = operation_row.course_id
       AND member.course_membership_id = enrollment.course_membership_id
       AND member.role = 'student'
       AND member.status = 'active'
     WHERE operation_row.tenant_id = p_tenant_id
       AND operation_row.course_id = p_course_id
       AND operation_row.grading_operation_id = p_operation_reference;

    IF NOT FOUND OR public.ple_instructor_grading_operation_actor_v1(
        p_tenant_id, p_session, p_course_id, v_assignment_id
    ) IS NULL THEN
        RETURN;
    END IF;
    IF v_target_kind = 'submission' AND v_membership_reference IS NULL THEN
        RETURN;
    END IF;
    IF v_target_kind NOT IN ('submission', 'assignment_scoring_generation') THEN
        RETURN;
    END IF;

    target_kind := v_target_kind;
    assignment_reference := v_assignment_reference;
    membership_reference := v_membership_reference;
    RETURN NEXT;
END;
$$;

ALTER FUNCTION public.ple_resolve_instructor_grading_operation_v1(
    uuid, character, uuid, integer
) OWNER TO ple_instructor_grading_operation_broker;
REVOKE ALL ON FUNCTION public.ple_resolve_instructor_grading_operation_v1(
    uuid, character, uuid, integer
) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION public.ple_resolve_instructor_grading_operation_v1(
    uuid, character, uuid, integer
) TO ple_app;

DO $$
DECLARE
    v_function regprocedure := pg_catalog.to_regprocedure(
        'public.ple_resolve_instructor_grading_operation_v1'
        '(uuid,character,uuid,integer)'
    );
BEGIN
    IF NOT EXISTS (
        SELECT 1
          FROM pg_catalog.pg_proc AS procedure_row
         WHERE procedure_row.oid = v_function
           AND procedure_row.proowner = 'ple_instructor_grading_operation_broker'::regrole
           AND procedure_row.prosecdef
           AND procedure_row.proconfig IS NOT DISTINCT FROM ARRAY[
               'search_path=pg_catalog, public, pg_temp'
           ]::text[]
    ) OR EXISTS (
        SELECT 1
          FROM pg_catalog.pg_proc AS procedure_row
          CROSS JOIN LATERAL pg_catalog.aclexplode(COALESCE(
              procedure_row.proacl,
              pg_catalog.acldefault('f', procedure_row.proowner)
          )) AS privilege
         WHERE procedure_row.oid = v_function
           AND privilege.grantee <> procedure_row.proowner
           AND (
               privilege.grantee <> 'ple_app'::regrole::oid
               OR privilege.privilege_type <> 'EXECUTE'
               OR privilege.is_grantable
           )
    ) OR NOT pg_catalog.has_function_privilege('ple_app', v_function, 'EXECUTE')
       OR pg_catalog.has_table_privilege(
           'ple_app', 'public.grading_operation', 'SELECT'
       )
    THEN
        RAISE EXCEPTION 'Gradebook operation-selection capability is unsafe';
    END IF;
END;
$$;

COMMIT;
