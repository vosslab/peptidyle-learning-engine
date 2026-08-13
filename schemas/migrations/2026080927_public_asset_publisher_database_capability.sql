-- Dedicated least-authority database capability for public asset publication.
--
-- The publisher has a distinct deployment-owned LOGIN (`ple_publisher_login`)
-- which may SET LOCAL ROLE only to this NOLOGIN capability. It cannot assume
-- `ple_app`, read tables, enumerate jobs, or invoke a general queue broker.
-- Its four functions bind every registry read, failure transition, and final
-- activation to the exact active `publishPublicAssets` lease.

DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'ple_public_asset_publisher') THEN
        CREATE ROLE ple_public_asset_publisher NOLOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE
            NOINHERIT NOBYPASSRLS;
    END IF;
END
$$;

REVOKE ALL ON SCHEMA public FROM ple_public_asset_publisher;
GRANT USAGE ON SCHEMA public TO ple_public_asset_publisher;

CREATE FUNCTION public.ple_claim_public_asset_publication_job(
    p_token uuid,
    p_lease_seconds integer
) RETURNS TABLE(job_id uuid, tenant_id uuid, payload jsonb, lease_token uuid, attempt_count integer)
    LANGUAGE sql SECURITY DEFINER
    SET search_path TO 'pg_catalog', 'public'
    AS $$
    SELECT *
      FROM public.ple_claim_worker_job(
          p_token,
          p_lease_seconds,
          ARRAY['publishPublicAssets']::text[]
      )
$$;

CREATE FUNCTION public.ple_ready_public_asset_publication_queue_depth() RETURNS bigint
    LANGUAGE sql SECURITY DEFINER
    SET search_path TO 'pg_catalog', 'public'
    AS $$
    SELECT public.ple_ready_worker_queue_depth(ARRAY['publishPublicAssets']::text[])
$$;

CREATE FUNCTION public.ple_fail_public_asset_publication_job(
    p_job uuid,
    p_token uuid,
    p_failure text
) RETURNS text
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'pg_catalog', 'public'
    AS $$
BEGIN
    IF p_job IS NULL OR p_token IS NULL OR p_failure NOT IN ('transient', 'permanent', 'timed_out')
    THEN
        RAISE EXCEPTION 'invalid public asset publication failure arguments'
            USING ERRCODE = '22023';
    END IF;

    PERFORM 1
      FROM public.worker_job AS job
     WHERE job.job_id = p_job
       AND job.state = 'leased'
       AND job.lease_token = p_token
       AND job.lease_expires_at > transaction_timestamp()
       AND job.payload ->> 'kind' = 'publishPublicAssets';
    IF NOT FOUND THEN
        RETURN NULL;
    END IF;

    RETURN public.ple_fail_worker_job(p_job, p_token, p_failure);
END
$$;

CREATE FUNCTION public.ple_read_pending_public_asset_publication(
    p_job uuid,
    p_token uuid,
    p_problem uuid,
    p_version uuid
) RETURNS TABLE(delivery_id uuid, payload jsonb, payload_sha256 character(64))
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'pg_catalog', 'public'
    AS $$
DECLARE
    expected_payload jsonb;
BEGIN
    IF p_job IS NULL OR p_token IS NULL OR p_problem IS NULL OR p_version IS NULL THEN
        RAISE EXCEPTION 'invalid public asset publication read arguments' USING ERRCODE = '22023';
    END IF;
    expected_payload := jsonb_build_object(
        'kind', 'publishPublicAssets',
        'reference', jsonb_build_object('problem', p_problem::text, 'version', p_version::text)
    );
    PERFORM 1
      FROM public.worker_job AS job
     WHERE job.job_id = p_job
       AND job.state = 'leased'
       AND job.lease_token = p_token
       AND job.lease_expires_at > transaction_timestamp()
       AND job.payload = expected_payload;
    IF NOT FOUND THEN
        RAISE EXCEPTION 'public asset publication lease is not active' USING ERRCODE = '22023';
    END IF;

    RETURN QUERY
    SELECT asset.delivery_id, asset.payload, asset.payload_sha256
      FROM public.asset_delivery AS asset
      JOIN public.problem_version AS version_row
        ON version_row.problem_id = asset.problem_id
       AND version_row.version_id = asset.version_id
     WHERE asset.delivery_kind = 'catalog'
       AND asset.problem_id = p_problem
       AND asset.version_id = p_version
       AND version_row.publication_scope = 'public'
       AND asset.payload ->> 'publication' = 'pending'
     ORDER BY asset.asset_id ASC;
END
$$;

CREATE FUNCTION public.ple_public_asset_publisher_migration_state()
RETURNS TABLE(version bigint, success boolean, checksum bytea)
    LANGUAGE sql SECURITY DEFINER
    SET search_path TO 'pg_catalog', 'public'
    AS $$
    SELECT version, success, checksum FROM public.ple_migration_state ORDER BY version
$$;

ALTER FUNCTION public.ple_claim_public_asset_publication_job(uuid, integer)
    OWNER TO ple_queue_broker;
ALTER FUNCTION public.ple_ready_public_asset_publication_queue_depth()
    OWNER TO ple_queue_broker;
ALTER FUNCTION public.ple_fail_public_asset_publication_job(uuid, uuid, text)
    OWNER TO ple_queue_broker;
ALTER FUNCTION public.ple_read_pending_public_asset_publication(uuid, uuid, uuid, uuid)
    OWNER TO ple_queue_broker;
ALTER FUNCTION public.ple_public_asset_publisher_migration_state()
    OWNER TO ple_queue_broker;

-- The broker owns SECURITY DEFINER functions, so its ACL needs the narrowly
-- required implementation tables. The publisher capability itself receives
-- no table privileges.
GRANT SELECT, UPDATE ON TABLE public.asset_delivery TO ple_queue_broker;
GRANT SELECT ON TABLE public.problem_version TO ple_queue_broker;
GRANT SELECT ON public.ple_migration_state TO ple_queue_broker;

REVOKE ALL ON FUNCTION public.ple_claim_public_asset_publication_job(uuid, integer) FROM PUBLIC;
REVOKE ALL ON FUNCTION public.ple_ready_public_asset_publication_queue_depth() FROM PUBLIC;
REVOKE ALL ON FUNCTION public.ple_fail_public_asset_publication_job(uuid, uuid, text) FROM PUBLIC;
REVOKE ALL ON FUNCTION public.ple_read_pending_public_asset_publication(uuid, uuid, uuid, uuid)
    FROM PUBLIC;
REVOKE ALL ON FUNCTION public.ple_public_asset_publisher_migration_state() FROM PUBLIC;
REVOKE ALL ON FUNCTION public.ple_activate_public_asset_publication(
    uuid, uuid, uuid, uuid, jsonb
) FROM PUBLIC, ple_app;

GRANT EXECUTE ON FUNCTION public.ple_claim_public_asset_publication_job(uuid, integer)
    TO ple_public_asset_publisher;
GRANT EXECUTE ON FUNCTION public.ple_ready_public_asset_publication_queue_depth()
    TO ple_public_asset_publisher;
GRANT EXECUTE ON FUNCTION public.ple_fail_public_asset_publication_job(uuid, uuid, text)
    TO ple_public_asset_publisher;
GRANT EXECUTE ON FUNCTION public.ple_read_pending_public_asset_publication(uuid, uuid, uuid, uuid)
    TO ple_public_asset_publisher;
GRANT EXECUTE ON FUNCTION public.ple_public_asset_publisher_migration_state()
    TO ple_public_asset_publisher;
GRANT EXECUTE ON FUNCTION public.ple_activate_public_asset_publication(
    uuid, uuid, uuid, uuid, jsonb
) TO ple_public_asset_publisher;
