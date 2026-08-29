-- WP-INST-G2 / G2-W3A: retain claimed-job tenancy through failure finalization.

BEGIN;

CREATE FUNCTION public.ple_fail_worker_job(
    p_tenant uuid,
    p_job_id uuid,
    p_token uuid,
    p_failure text
) RETURNS text
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public', pg_temp
AS $$
DECLARE
    next_state text;
BEGIN
    IF p_tenant IS NULL OR p_job_id IS NULL OR p_token IS NULL
       OR p_failure NOT IN ('transient', 'permanent', 'timed_out')
       OR p_tenant IS DISTINCT FROM public.ple_current_tenant()
    THEN
        RAISE EXCEPTION 'tenant-bound queue failure arguments are invalid'
            USING ERRCODE = '22023';
    END IF;

    UPDATE public.worker_job
       SET state = CASE
               WHEN p_failure = 'permanent' OR attempt_count >= max_attempts THEN 'dead'
               ELSE 'ready'
           END,
           available_at = CASE
               WHEN p_failure = 'permanent' OR attempt_count >= max_attempts
                   THEN available_at
               ELSE transaction_timestamp() + make_interval(
                   secs => (1 << LEAST(GREATEST(attempt_count - 1, 0), 8))
               )
           END,
           lease_token = NULL,
           lease_expires_at = NULL,
           last_error = p_failure,
           completed_at = CASE
               WHEN p_failure = 'permanent' OR attempt_count >= max_attempts
                   THEN transaction_timestamp()
               ELSE NULL
           END
     WHERE tenant_id = p_tenant
       AND job_id = p_job_id
       AND state = 'leased'
       AND lease_token = p_token
       AND lease_expires_at > transaction_timestamp()
       AND payload ->> 'kind' <> 'gradeAcceptedSubmission'
    RETURNING state INTO next_state;

    IF next_state IS NULL THEN
        RETURN NULL;
    END IF;
    RETURN CASE WHEN next_state = 'dead' THEN 'dead' ELSE 'retrying' END;
END;
$$;

ALTER FUNCTION public.ple_fail_worker_job(uuid, uuid, uuid, text)
    OWNER TO ple_queue_broker;
REVOKE ALL ON FUNCTION public.ple_fail_worker_job(uuid, uuid, uuid, text)
    FROM PUBLIC;
GRANT EXECUTE ON FUNCTION public.ple_fail_worker_job(uuid, uuid, uuid, text)
    TO ple_app;

CREATE FUNCTION public.ple_fail_public_asset_publication_job(
    p_tenant uuid,
    p_job uuid,
    p_token uuid,
    p_failure text
) RETURNS text
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public', pg_temp
AS $$
BEGIN
    IF p_tenant IS NULL OR p_job IS NULL OR p_token IS NULL
       OR p_failure NOT IN ('transient', 'permanent', 'timed_out')
    THEN
        RAISE EXCEPTION 'tenant-bound public asset failure arguments are invalid'
            USING ERRCODE = '22023';
    END IF;

    PERFORM 1
      FROM public.worker_job AS job
     WHERE job.tenant_id = p_tenant
       AND job.job_id = p_job
       AND job.state = 'leased'
       AND job.lease_token = p_token
       AND job.lease_expires_at > transaction_timestamp()
       AND job.payload ->> 'kind' = 'publishPublicAssets';
    IF NOT FOUND THEN
        RETURN NULL;
    END IF;

    PERFORM set_config('ple.tenant_id', p_tenant::text, true);
    RETURN public.ple_fail_worker_job(p_tenant, p_job, p_token, p_failure);
END;
$$;

ALTER FUNCTION public.ple_fail_public_asset_publication_job(uuid, uuid, uuid, text)
    OWNER TO ple_queue_broker;
REVOKE ALL ON FUNCTION public.ple_fail_public_asset_publication_job(uuid, uuid, uuid, text)
    FROM PUBLIC;
GRANT EXECUTE ON FUNCTION public.ple_fail_public_asset_publication_job(
    uuid, uuid, uuid, text
) TO ple_public_asset_publisher;

REVOKE ALL ON FUNCTION public.ple_fail_public_asset_publication_job(uuid, uuid, text)
    FROM PUBLIC, ple_public_asset_publisher;
DROP FUNCTION public.ple_fail_public_asset_publication_job(uuid, uuid, text) RESTRICT;
REVOKE ALL ON FUNCTION public.ple_fail_worker_job(uuid, uuid, text)
    FROM PUBLIC, ple_app;
DROP FUNCTION public.ple_fail_worker_job(uuid, uuid, text) RESTRICT;

DO $$
DECLARE
    generic_function regprocedure :=
        'public.ple_fail_worker_job(uuid,uuid,uuid,text)'::regprocedure;
    publisher_function regprocedure :=
        'public.ple_fail_public_asset_publication_job(uuid,uuid,uuid,text)'::regprocedure;
BEGIN
    IF EXISTS (
        SELECT 1
          FROM pg_catalog.pg_proc AS procedure_row
         WHERE procedure_row.oid = generic_function
           AND (
               procedure_row.proowner <> 'ple_queue_broker'::regrole
               OR NOT procedure_row.prosecdef
               OR procedure_row.proconfig IS DISTINCT FROM
                   ARRAY['search_path=pg_catalog, public, pg_temp']
           )
    ) OR EXISTS (
        SELECT 1
          FROM pg_catalog.pg_proc AS procedure_row
          CROSS JOIN LATERAL pg_catalog.aclexplode(
              COALESCE(
                  procedure_row.proacl,
                  pg_catalog.acldefault('f', procedure_row.proowner)
              )
          ) AS privilege_row
         WHERE procedure_row.oid = generic_function
           AND privilege_row.grantee <> procedure_row.proowner
           AND (
               privilege_row.grantee <> 'ple_app'::regrole
               OR privilege_row.privilege_type <> 'EXECUTE'
               OR privilege_row.is_grantable
           )
    ) OR NOT pg_catalog.has_function_privilege(
        'ple_app', generic_function, 'EXECUTE'
    ) THEN
        RAISE EXCEPTION 'tenant-bound worker failure authority is unsafe';
    END IF;

    IF EXISTS (
        SELECT 1
          FROM pg_catalog.pg_proc AS procedure_row
         WHERE procedure_row.oid = publisher_function
           AND (
               procedure_row.proowner <> 'ple_queue_broker'::regrole
               OR NOT procedure_row.prosecdef
               OR procedure_row.proconfig IS DISTINCT FROM
                   ARRAY['search_path=pg_catalog, public, pg_temp']
           )
    ) OR EXISTS (
        SELECT 1
          FROM pg_catalog.pg_proc AS procedure_row
          CROSS JOIN LATERAL pg_catalog.aclexplode(
              COALESCE(
                  procedure_row.proacl,
                  pg_catalog.acldefault('f', procedure_row.proowner)
              )
          ) AS privilege_row
         WHERE procedure_row.oid = publisher_function
           AND privilege_row.grantee <> procedure_row.proowner
           AND (
               privilege_row.grantee <> 'ple_public_asset_publisher'::regrole
               OR privilege_row.privilege_type <> 'EXECUTE'
               OR privilege_row.is_grantable
           )
    ) OR NOT pg_catalog.has_function_privilege(
        'ple_public_asset_publisher', publisher_function, 'EXECUTE'
    ) OR pg_catalog.to_regprocedure(
        'public.ple_fail_worker_job(uuid,uuid,text)'
    ) IS NOT NULL OR pg_catalog.to_regprocedure(
        'public.ple_fail_public_asset_publication_job(uuid,uuid,text)'
    ) IS NOT NULL THEN
        RAISE EXCEPTION 'tenant-bound publisher failure authority is unsafe';
    END IF;
END;
$$;

COMMIT;
