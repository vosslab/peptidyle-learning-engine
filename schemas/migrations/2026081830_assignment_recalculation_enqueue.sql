-- Route grade-driven scoring invalidation through one server-owned capability.
-- The operation advances the generation and enqueues its matching worker job
-- atomically; application code retains no direct assignment mutation grant.

BEGIN;

DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_catalog.pg_roles WHERE rolname = 'ple_scoring_invalidation_broker') THEN
        CREATE ROLE ple_scoring_invalidation_broker
            NOLOGIN NOINHERIT NOSUPERUSER NOCREATEDB NOCREATEROLE NOREPLICATION NOBYPASSRLS;
    END IF;
    IF EXISTS (
        SELECT 1
          FROM pg_catalog.pg_auth_members AS membership
         WHERE membership.roleid = 'ple_scoring_invalidation_broker'::regrole
            OR membership.member = 'ple_scoring_invalidation_broker'::regrole
    ) THEN
        RAISE EXCEPTION 'ple_scoring_invalidation_broker must not have role memberships';
    END IF;
END
$$;

ALTER ROLE ple_scoring_invalidation_broker
    NOLOGIN NOINHERIT NOSUPERUSER NOCREATEDB NOCREATEROLE NOREPLICATION NOBYPASSRLS;

REVOKE ALL ON SCHEMA public FROM ple_scoring_invalidation_broker;
GRANT USAGE ON SCHEMA public TO ple_scoring_invalidation_broker;

CREATE POLICY scoring_invalidation_assignment_tenant
    ON public.assignment
    TO ple_scoring_invalidation_broker
    USING (tenant_id = public.ple_current_tenant())
    WITH CHECK (tenant_id = public.ple_current_tenant());

CREATE POLICY scoring_invalidation_worker_job_tenant
    ON public.worker_job
    TO ple_scoring_invalidation_broker
    WITH CHECK (tenant_id = public.ple_current_tenant());

GRANT SELECT (tenant_id, assignment_id, scoring_generation)
    ON public.assignment TO ple_scoring_invalidation_broker;
GRANT UPDATE (scoring_generation, scoring_status, updated_at)
    ON public.assignment TO ple_scoring_invalidation_broker;
GRANT INSERT (job_id, tenant_id, payload, state, max_attempts)
    ON public.worker_job TO ple_scoring_invalidation_broker;
GRANT EXECUTE ON FUNCTION public.ple_current_tenant()
    TO ple_scoring_invalidation_broker;

CREATE FUNCTION public.ple_enqueue_assignment_recalculation(
    p_tenant uuid,
    p_assignment uuid,
    p_job uuid,
    p_max_attempts integer
) RETURNS bigint
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public', pg_temp
AS $$
DECLARE
    next_generation bigint;
BEGIN
    IF p_tenant IS NULL OR p_assignment IS NULL OR p_job IS NULL
       OR p_max_attempts NOT BETWEEN 1 AND 20
       OR p_tenant IS DISTINCT FROM public.ple_current_tenant() THEN
        RAISE EXCEPTION 'assignment recalculation request is invalid' USING ERRCODE = '22023';
    END IF;

    UPDATE public.assignment AS assignment
       SET scoring_generation = assignment.scoring_generation + 1,
           scoring_status = 'recalculating',
           updated_at = transaction_timestamp()
     WHERE assignment.tenant_id = p_tenant
       AND assignment.assignment_id = p_assignment
    RETURNING assignment.scoring_generation INTO next_generation;

    IF NOT FOUND THEN
        RAISE EXCEPTION 'assignment recalculation target is unavailable' USING ERRCODE = '42501';
    END IF;

    INSERT INTO public.worker_job (job_id, tenant_id, payload, state, max_attempts)
    VALUES (
        p_job,
        p_tenant,
        jsonb_build_object(
            'kind', 'recalculateAssignment',
            'assignment', p_assignment::text,
            'generation', next_generation
        ),
        'ready',
        p_max_attempts
    );

    RETURN next_generation;
END
$$;

ALTER FUNCTION public.ple_enqueue_assignment_recalculation(uuid, uuid, uuid, integer)
    OWNER TO ple_scoring_invalidation_broker;
REVOKE ALL ON FUNCTION public.ple_enqueue_assignment_recalculation(uuid, uuid, uuid, integer)
    FROM PUBLIC, ple_app;
GRANT EXECUTE ON FUNCTION public.ple_enqueue_assignment_recalculation(uuid, uuid, uuid, integer)
    TO ple_app;

DO $$
BEGIN
    IF NOT has_function_privilege(
        'ple_app',
        'public.ple_enqueue_assignment_recalculation(uuid,uuid,uuid,integer)',
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
               'public.ple_enqueue_assignment_recalculation(uuid,uuid,uuid,integer)'::regprocedure
           AND privilege_row.grantee = 0
           AND privilege_row.privilege_type = 'EXECUTE'
    ) OR has_table_privilege('ple_scoring_invalidation_broker', 'public.assignment', 'INSERT')
       OR has_table_privilege('ple_scoring_invalidation_broker', 'public.assignment', 'DELETE')
       OR has_table_privilege('ple_scoring_invalidation_broker', 'public.assignment', 'TRUNCATE')
       OR has_table_privilege('ple_scoring_invalidation_broker', 'public.assignment', 'TRIGGER')
       OR has_table_privilege('ple_scoring_invalidation_broker', 'public.worker_job', 'SELECT')
       OR has_table_privilege('ple_scoring_invalidation_broker', 'public.worker_job', 'UPDATE')
       OR has_table_privilege('ple_scoring_invalidation_broker', 'public.worker_job', 'DELETE')
       OR EXISTS (
           SELECT 1
             FROM pg_catalog.pg_roles AS role_row
            WHERE role_row.rolname = 'ple_scoring_invalidation_broker'
              AND (
                  role_row.rolcanlogin OR role_row.rolinherit OR role_row.rolsuper
                  OR role_row.rolcreatedb OR role_row.rolcreaterole
                  OR role_row.rolreplication OR role_row.rolbypassrls
              )
       ) THEN
        RAISE EXCEPTION 'assignment recalculation capability privilege matrix is unsafe';
    END IF;
END
$$;

COMMIT;
