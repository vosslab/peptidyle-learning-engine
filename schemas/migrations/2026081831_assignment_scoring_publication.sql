-- Publish one still-current assignment scoring generation and enqueue its
-- course-item-analysis projection through a single server-owned capability.

BEGIN;

DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_catalog.pg_roles WHERE rolname = 'ple_scoring_commit_broker') THEN
        CREATE ROLE ple_scoring_commit_broker
            NOLOGIN NOINHERIT NOSUPERUSER NOCREATEDB NOCREATEROLE NOREPLICATION NOBYPASSRLS;
    END IF;
    IF EXISTS (
        SELECT 1
          FROM pg_catalog.pg_auth_members AS membership
         WHERE membership.roleid = 'ple_scoring_commit_broker'::regrole
            OR membership.member = 'ple_scoring_commit_broker'::regrole
    ) THEN
        RAISE EXCEPTION 'ple_scoring_commit_broker must not have role memberships';
    END IF;
END
$$;

ALTER ROLE ple_scoring_commit_broker
    NOLOGIN NOINHERIT NOSUPERUSER NOCREATEDB NOCREATEROLE NOREPLICATION NOBYPASSRLS;

REVOKE ALL ON SCHEMA public FROM ple_scoring_commit_broker;
GRANT USAGE ON SCHEMA public TO ple_scoring_commit_broker;

CREATE POLICY scoring_commit_assignment_tenant
    ON public.assignment
    TO ple_scoring_commit_broker
    USING (tenant_id = public.ple_current_tenant())
    WITH CHECK (tenant_id = public.ple_current_tenant());

CREATE POLICY scoring_commit_worker_job_tenant
    ON public.worker_job
    TO ple_scoring_commit_broker
    USING (tenant_id = public.ple_current_tenant())
    WITH CHECK (tenant_id = public.ple_current_tenant());

GRANT SELECT (tenant_id, assignment_id, scoring_generation, scoring_status)
    ON public.assignment TO ple_scoring_commit_broker;
GRANT UPDATE (scoring_status, updated_at)
    ON public.assignment TO ple_scoring_commit_broker;
GRANT SELECT (job_id, tenant_id, payload, state, lease_token, lease_expires_at)
    ON public.worker_job TO ple_scoring_commit_broker;
GRANT UPDATE (state)
    ON public.worker_job TO ple_scoring_commit_broker;
GRANT INSERT (job_id, tenant_id, payload, state, max_attempts)
    ON public.worker_job TO ple_scoring_commit_broker;
GRANT EXECUTE ON FUNCTION public.ple_current_tenant()
    TO ple_scoring_commit_broker;

CREATE FUNCTION public.ple_publish_assignment_scoring_generation(
    p_tenant uuid,
    p_scoring_job uuid,
    p_scoring_lease uuid,
    p_assignment uuid,
    p_generation bigint,
    p_analysis_job uuid
) RETURNS boolean
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public', pg_temp
AS $$
DECLARE
    analysis_payload jsonb;
BEGIN
    IF p_tenant IS NULL OR p_scoring_job IS NULL OR p_scoring_lease IS NULL
       OR p_assignment IS NULL OR p_generation IS NULL OR p_generation < 1
       OR p_analysis_job IS NULL
       OR p_tenant IS DISTINCT FROM public.ple_current_tenant() THEN
        RAISE EXCEPTION 'assignment scoring publication request is invalid'
            USING ERRCODE = '22023';
    END IF;

    -- Bind publication to the exact live worker claim that prepared this
    -- generation. A caller cannot publish staged rows through another job.
    PERFORM 1
      FROM public.worker_job AS scoring_job
     WHERE scoring_job.tenant_id = p_tenant
       AND scoring_job.job_id = p_scoring_job
       AND scoring_job.state = 'leased'
       AND scoring_job.lease_token = p_scoring_lease
       AND scoring_job.lease_expires_at > transaction_timestamp()
       AND scoring_job.payload = jsonb_build_object(
           'kind', 'recalculateAssignment',
           'assignment', p_assignment::text,
           'generation', p_generation
       )
     FOR UPDATE;
    IF NOT FOUND THEN
        RETURN false;
    END IF;

    UPDATE public.assignment AS assignment
       SET scoring_status = 'current',
           updated_at = transaction_timestamp()
     WHERE assignment.tenant_id = p_tenant
       AND assignment.assignment_id = p_assignment
       AND assignment.scoring_generation = p_generation
       AND assignment.scoring_status = 'recalculating';
    IF NOT FOUND THEN
        RETURN false;
    END IF;

    analysis_payload := jsonb_build_object(
        'kind', 'recalculateCourseItemAnalysis',
        'assignment', p_assignment::text,
        'generation', p_generation
    );
    INSERT INTO public.worker_job (job_id, tenant_id, payload, state, max_attempts)
    VALUES (p_analysis_job, p_tenant, analysis_payload, 'ready', 10)
    ON CONFLICT DO NOTHING;

    IF NOT FOUND AND NOT EXISTS (
        SELECT 1
          FROM public.worker_job AS analysis_job
         WHERE analysis_job.tenant_id = p_tenant
           AND analysis_job.payload = analysis_payload
    ) THEN
        RAISE EXCEPTION 'course item analysis handoff conflicts' USING ERRCODE = '23505';
    END IF;

    RETURN true;
END
$$;

ALTER FUNCTION public.ple_publish_assignment_scoring_generation(
    uuid, uuid, uuid, uuid, bigint, uuid
) OWNER TO ple_scoring_commit_broker;
REVOKE ALL ON FUNCTION public.ple_publish_assignment_scoring_generation(
    uuid, uuid, uuid, uuid, bigint, uuid
) FROM PUBLIC, ple_app;
GRANT EXECUTE ON FUNCTION public.ple_publish_assignment_scoring_generation(
    uuid, uuid, uuid, uuid, bigint, uuid
) TO ple_app;

DO $$
BEGIN
    IF NOT has_function_privilege(
        'ple_app',
        'public.ple_publish_assignment_scoring_generation(uuid,uuid,uuid,uuid,bigint,uuid)',
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
               'public.ple_publish_assignment_scoring_generation(uuid,uuid,uuid,uuid,bigint,uuid)'::regprocedure
           AND privilege_row.grantee = 0
           AND privilege_row.privilege_type = 'EXECUTE'
    ) OR has_table_privilege('ple_app', 'public.assignment', 'UPDATE')
       OR has_table_privilege('ple_scoring_commit_broker', 'public.assignment', 'INSERT')
       OR has_table_privilege('ple_scoring_commit_broker', 'public.assignment', 'DELETE')
       OR NOT has_column_privilege(
           'ple_scoring_commit_broker', 'public.worker_job', 'state', 'UPDATE'
       )
       OR has_column_privilege(
           'ple_scoring_commit_broker', 'public.worker_job', 'payload', 'UPDATE'
       )
       OR has_table_privilege('ple_scoring_commit_broker', 'public.worker_job', 'DELETE')
       OR EXISTS (
           SELECT 1
             FROM pg_catalog.pg_roles AS role_row
            WHERE role_row.rolname = 'ple_scoring_commit_broker'
              AND (
                  role_row.rolcanlogin OR role_row.rolinherit OR role_row.rolsuper
                  OR role_row.rolcreatedb OR role_row.rolcreaterole
                  OR role_row.rolreplication OR role_row.rolbypassrls
              )
       ) THEN
        RAISE EXCEPTION 'assignment scoring publication privilege matrix is unsafe';
    END IF;
END
$$;

COMMIT;
