-- Finalize one course-item-analysis generation through a single server-owned
-- capability. The exact worker claim is locked before its assignment fence.

BEGIN;

DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_catalog.pg_roles WHERE rolname = 'ple_item_analysis_commit_broker') THEN
        CREATE ROLE ple_item_analysis_commit_broker
            NOLOGIN NOINHERIT NOSUPERUSER NOCREATEDB NOCREATEROLE NOREPLICATION NOBYPASSRLS;
    END IF;
    IF EXISTS (
        SELECT 1
          FROM pg_catalog.pg_auth_members AS membership
         WHERE membership.roleid = 'ple_item_analysis_commit_broker'::regrole
            OR membership.member = 'ple_item_analysis_commit_broker'::regrole
    ) THEN
        RAISE EXCEPTION 'ple_item_analysis_commit_broker must not have role memberships';
    END IF;
END
$$;

ALTER ROLE ple_item_analysis_commit_broker
    NOLOGIN NOINHERIT NOSUPERUSER NOCREATEDB NOCREATEROLE NOREPLICATION NOBYPASSRLS;

REVOKE ALL ON SCHEMA public FROM ple_item_analysis_commit_broker;
GRANT USAGE ON SCHEMA public TO ple_item_analysis_commit_broker;

CREATE POLICY item_analysis_commit_assignment_tenant
    ON public.assignment
    TO ple_item_analysis_commit_broker
    USING (tenant_id = public.ple_current_tenant());
CREATE POLICY item_analysis_commit_job_tenant
    ON public.worker_job
    TO ple_item_analysis_commit_broker
    USING (tenant_id = public.ple_current_tenant());
CREATE POLICY item_analysis_commit_staging_tenant
    ON public.course_item_analysis_staging
    TO ple_item_analysis_commit_broker
    USING (tenant_id = public.ple_current_tenant());
CREATE POLICY item_analysis_commit_current_tenant
    ON public.course_item_analysis_current
    TO ple_item_analysis_commit_broker
    USING (tenant_id = public.ple_current_tenant())
    WITH CHECK (tenant_id = public.ple_current_tenant());

GRANT SELECT (tenant_id, course_id, assignment_id, scoring_generation, scoring_status)
    ON public.assignment TO ple_item_analysis_commit_broker;
-- PostgreSQL requires a narrow UPDATE grant for a locking SELECT. The
-- execute-only function never changes this column.
GRANT UPDATE (scoring_status)
    ON public.assignment TO ple_item_analysis_commit_broker;
GRANT SELECT (job_id, tenant_id, payload, state, lease_token, lease_expires_at)
    ON public.worker_job TO ple_item_analysis_commit_broker;
GRANT UPDATE (state)
    ON public.worker_job TO ple_item_analysis_commit_broker;
GRANT SELECT, DELETE
    ON public.course_item_analysis_staging TO ple_item_analysis_commit_broker;
GRANT SELECT, INSERT, DELETE
    ON public.course_item_analysis_current TO ple_item_analysis_commit_broker;
GRANT EXECUTE ON FUNCTION public.ple_current_tenant(), public.ple_complete_worker_job(uuid, uuid)
    TO ple_item_analysis_commit_broker;

CREATE FUNCTION public.ple_commit_course_item_analysis_generation(
    p_tenant uuid,
    p_job uuid,
    p_lease uuid,
    p_assignment uuid,
    p_generation bigint
) RETURNS text
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public', pg_temp
AS $$
DECLARE
    assignment_course uuid;
    assignment_generation bigint;
    assignment_status text;
    promoted_count bigint;
BEGIN
    IF p_tenant IS NULL OR p_job IS NULL OR p_lease IS NULL
       OR p_assignment IS NULL OR p_generation IS NULL OR p_generation < 1
       OR p_tenant IS DISTINCT FROM public.ple_current_tenant() THEN
        RAISE EXCEPTION 'course item analysis commit request is invalid'
            USING ERRCODE = '22023';
    END IF;

    PERFORM 1
      FROM public.worker_job AS analysis_job
     WHERE analysis_job.tenant_id = p_tenant
       AND analysis_job.job_id = p_job
       AND analysis_job.state = 'leased'
       AND analysis_job.lease_token = p_lease
       AND analysis_job.lease_expires_at > transaction_timestamp()
       AND analysis_job.payload = jsonb_build_object(
           'kind', 'recalculateCourseItemAnalysis',
           'assignment', p_assignment::text,
           'generation', p_generation
       )
     FOR UPDATE;
    IF NOT FOUND THEN
        RETURN 'claim_no_longer_active';
    END IF;

    SELECT assignment.course_id,
           assignment.scoring_generation,
           assignment.scoring_status
      INTO assignment_course, assignment_generation, assignment_status
      FROM public.assignment AS assignment
     WHERE assignment.tenant_id = p_tenant
       AND assignment.assignment_id = p_assignment
     FOR UPDATE;

    IF NOT FOUND
       OR assignment_generation IS DISTINCT FROM p_generation
       OR assignment_status IS DISTINCT FROM 'current' THEN
        DELETE FROM public.course_item_analysis_staging AS staging
         WHERE staging.tenant_id = p_tenant AND staging.job_id = p_job;
        IF NOT public.ple_complete_worker_job(p_job, p_lease) THEN
            RAISE EXCEPTION 'course item analysis claim completion conflicts'
                USING ERRCODE = '23505';
        END IF;
        RETURN 'superseded';
    END IF;

    IF NOT EXISTS (
        SELECT 1
          FROM public.course_item_analysis_staging AS staging
         WHERE staging.tenant_id = p_tenant
           AND staging.job_id = p_job
           AND staging.course_id = assignment_course
           AND staging.assignment_id = p_assignment
           AND staging.source_scoring_generation = p_generation
    ) THEN
        RETURN 'staging_unavailable';
    END IF;

    DELETE FROM public.course_item_analysis_current AS current_analysis
     WHERE current_analysis.tenant_id = p_tenant
       AND current_analysis.assignment_id = p_assignment;

    INSERT INTO public.course_item_analysis_current (
        tenant_id,
        course_id,
        assignment_id,
        source_scoring_generation,
        report_schema_version,
        report_payload,
        report_payload_sha256,
        analyzed_at
    )
    SELECT staging.tenant_id,
           staging.course_id,
           staging.assignment_id,
           staging.source_scoring_generation,
           staging.report_schema_version,
           staging.report_payload,
           staging.report_payload_sha256,
           staging.prepared_at
      FROM public.course_item_analysis_staging AS staging
     WHERE staging.tenant_id = p_tenant AND staging.job_id = p_job;
    GET DIAGNOSTICS promoted_count = ROW_COUNT;
    IF promoted_count <> 1 THEN
        RAISE EXCEPTION 'course item analysis promotion conflicts' USING ERRCODE = '23505';
    END IF;

    DELETE FROM public.course_item_analysis_staging AS staging
     WHERE staging.tenant_id = p_tenant AND staging.job_id = p_job;
    IF NOT FOUND OR NOT public.ple_complete_worker_job(p_job, p_lease) THEN
        RAISE EXCEPTION 'course item analysis finalization conflicts' USING ERRCODE = '23505';
    END IF;
    RETURN 'committed';
END
$$;

ALTER FUNCTION public.ple_commit_course_item_analysis_generation(uuid, uuid, uuid, uuid, bigint)
    OWNER TO ple_item_analysis_commit_broker;
REVOKE ALL ON FUNCTION public.ple_commit_course_item_analysis_generation(uuid, uuid, uuid, uuid, bigint)
    FROM PUBLIC, ple_app;
GRANT EXECUTE ON FUNCTION public.ple_commit_course_item_analysis_generation(uuid, uuid, uuid, uuid, bigint)
    TO ple_app;

DO $$
BEGIN
    IF NOT has_function_privilege(
        'ple_app',
        'public.ple_commit_course_item_analysis_generation(uuid,uuid,uuid,uuid,bigint)',
        'EXECUTE'
    ) OR EXISTS (
        SELECT 1
          FROM pg_catalog.pg_proc AS procedure_row
          CROSS JOIN LATERAL pg_catalog.aclexplode(
              COALESCE(
                  procedure_row.proacl,
                  pg_catalog.acldefault('f', procedure_row.proowner)
              )
          ) AS privilege_row
         WHERE procedure_row.oid =
               'public.ple_commit_course_item_analysis_generation(uuid,uuid,uuid,uuid,bigint)'::regprocedure
           AND privilege_row.grantee = 0
           AND privilege_row.privilege_type = 'EXECUTE'
    ) OR has_table_privilege('ple_item_analysis_commit_broker', 'public.assignment', 'INSERT')
       OR has_table_privilege('ple_item_analysis_commit_broker', 'public.assignment', 'DELETE')
       OR has_table_privilege('ple_item_analysis_commit_broker', 'public.worker_job', 'INSERT')
       OR has_table_privilege('ple_item_analysis_commit_broker', 'public.worker_job', 'DELETE')
       OR has_table_privilege('ple_item_analysis_commit_broker', 'public.course_item_analysis_current', 'UPDATE')
       OR has_table_privilege('ple_item_analysis_commit_broker', 'public.course_item_analysis_staging', 'INSERT')
       OR has_table_privilege('ple_item_analysis_commit_broker', 'public.course_item_analysis_staging', 'UPDATE')
       OR EXISTS (
           SELECT 1
             FROM pg_catalog.pg_roles AS role_row
            WHERE role_row.rolname = 'ple_item_analysis_commit_broker'
              AND (
                  role_row.rolcanlogin OR role_row.rolinherit OR role_row.rolsuper
                  OR role_row.rolcreatedb OR role_row.rolcreaterole
                  OR role_row.rolreplication OR role_row.rolbypassrls
              )
       ) THEN
        RAISE EXCEPTION 'course item analysis commit privilege matrix is unsafe';
    END IF;
END
$$;

COMMIT;
