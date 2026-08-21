-- Student account-course context runs before the authenticated account chooses
-- a tenant. Keep the retention decision in a narrow retention-owned helper.
CREATE FUNCTION public.ple_account_student_context_records_visible(
    p_tenant uuid,
    p_course uuid
) RETURNS boolean
    LANGUAGE sql
    STABLE
    SECURITY DEFINER
    SET search_path TO 'pg_catalog', 'public'
    AS $$
    SELECT p_tenant IS NOT NULL
       AND p_course IS NOT NULL
       AND NOT EXISTS (
           SELECT 1
             FROM public.course_retention AS retention
            WHERE retention.tenant_id = p_tenant
              AND retention.course_id = p_course
              AND retention.lifecycle IN ('archived', 'deleted')
       )
       AND NOT EXISTS (
           SELECT 1
             FROM public.course_retention AS retention
             JOIN public.course_retention_stage AS stage
               ON stage.tenant_id = retention.tenant_id
              AND stage.course_id = retention.course_id
              AND stage.generation = retention.generation
            WHERE retention.tenant_id = p_tenant
              AND retention.course_id = p_course
              AND stage.stage IN ('archiveStudentRecords', 'deleteStudentRecords')
              AND stage.state = 'started'
       )
$$;

ALTER FUNCTION public.ple_account_student_context_records_visible(uuid, uuid)
    OWNER TO ple_retention_broker;
REVOKE ALL ON FUNCTION public.ple_account_student_context_records_visible(uuid, uuid)
    FROM PUBLIC;
GRANT EXECUTE ON FUNCTION public.ple_account_student_context_records_visible(uuid, uuid)
    TO ple_enrollment_broker;

CREATE OR REPLACE FUNCTION public.ple_account_course_context_page(
    p_user uuid,
    p_after_tenant uuid,
    p_after_course uuid,
    p_limit integer
) RETURNS TABLE (tenant_id uuid, course_id uuid, title text, role text)
    LANGUAGE sql STABLE SECURITY DEFINER
    SET search_path TO 'pg_catalog', 'public'
    AS $$
    SELECT membership.tenant_id, membership.course_id, course.title, membership.role
      FROM public.course_member membership
      JOIN public.course course
        ON course.tenant_id = membership.tenant_id AND course.course_id = membership.course_id
     WHERE membership.user_id = p_user AND membership.status = 'active'
       AND (membership.role <> 'student'
            OR public.ple_account_student_context_records_visible(
                membership.tenant_id, membership.course_id))
       AND (p_after_tenant IS NULL
            OR (membership.tenant_id, membership.course_id) > (p_after_tenant, p_after_course))
     ORDER BY membership.tenant_id, membership.course_id
     LIMIT least(greatest(p_limit, 1), 101)
$$;

CREATE OR REPLACE FUNCTION public.ple_account_course_context(
    p_user uuid,
    p_course uuid
) RETURNS TABLE (tenant_id uuid, course_id uuid, title text, role text)
    LANGUAGE sql STABLE SECURITY DEFINER
    SET search_path TO 'pg_catalog', 'public'
    AS $$
    SELECT membership.tenant_id, membership.course_id, course.title, membership.role
      FROM public.course_member membership
      JOIN public.course course
        ON course.tenant_id = membership.tenant_id AND course.course_id = membership.course_id
     WHERE membership.user_id = p_user AND membership.course_id = p_course
       AND membership.status = 'active'
       AND (membership.role <> 'student'
            OR public.ple_account_student_context_records_visible(
                membership.tenant_id, membership.course_id))
     ORDER BY membership.tenant_id LIMIT 2
$$;

ALTER FUNCTION public.ple_account_course_context_page(uuid, uuid, uuid, integer)
    OWNER TO ple_enrollment_broker;
ALTER FUNCTION public.ple_account_course_context(uuid, uuid)
    OWNER TO ple_enrollment_broker;
REVOKE ALL ON FUNCTION public.ple_account_course_context_page(uuid, uuid, uuid, integer)
    FROM PUBLIC;
REVOKE ALL ON FUNCTION public.ple_account_course_context(uuid, uuid) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION public.ple_account_course_context_page(uuid, uuid, uuid, integer)
    TO ple_auth;
GRANT EXECUTE ON FUNCTION public.ple_account_course_context(uuid, uuid) TO ple_auth;
