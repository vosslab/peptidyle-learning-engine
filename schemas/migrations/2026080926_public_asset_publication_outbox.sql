-- Durable post-commit publication of immutable public catalog assets.
--
-- Catalog publication and an object-store copy cannot share a transaction.
-- The catalog transaction therefore commits a pending registry record and a
-- closed queue payload.  This broker function is the sole state transition
-- from pending to publicly deliverable: it checks the exact active lease,
-- accepts only the mechanical pending -> ready JSON transformation, and
-- completes that same lease atomically.

ALTER TABLE public.worker_job
    DROP CONSTRAINT worker_job_payload_kind_check;

ALTER TABLE public.worker_job
    ADD CONSTRAINT worker_job_payload_kind_check CHECK (
        CASE payload ->> 'kind'
            WHEN 'render' THEN
                payload ?& ARRAY['kind', 'reference', 'seed']
                AND payload - ARRAY['kind', 'reference', 'seed'] = '{}'::jsonb
                AND jsonb_typeof(payload -> 'reference') = 'object'
                AND (payload -> 'reference') ?& ARRAY['problem', 'version']
                AND (payload -> 'reference') - ARRAY['problem', 'version'] = '{}'::jsonb
                AND (payload #>> '{reference,problem}') ~* '^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$'
                AND (payload #>> '{reference,version}') ~* '^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$'
                AND jsonb_typeof(payload -> 'seed') = 'number'
                AND (payload ->> 'seed') ~ '^(0|[1-9][0-9]{0,19})$'
                AND (payload ->> 'seed')::numeric <= 18446744073709551615
            WHEN 'export' THEN
                payload ?& ARRAY['kind', 'delivery_object']
                AND payload - ARRAY['kind', 'delivery_object'] = '{}'::jsonb
                AND (payload ->> 'delivery_object') ~* '^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$'
            WHEN 'import' THEN
                payload ?& ARRAY['kind', 'source_object']
                AND payload - ARRAY['kind', 'source_object'] = '{}'::jsonb
                AND (payload ->> 'source_object') ~* '^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$'
            WHEN 'qtiImport' THEN
                payload ?& ARRAY['kind', 'workspace', 'import', 'source_object']
                AND payload - ARRAY['kind', 'workspace', 'import', 'source_object'] = '{}'::jsonb
                AND (payload ->> 'workspace') ~* '^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$'
                AND (payload ->> 'import') ~* '^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$'
                AND (payload ->> 'source_object') ~* '^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$'
            WHEN 'retention' THEN
                payload ?& ARRAY['kind', 'course', 'stage', 'generation']
                AND payload - ARRAY['kind', 'course', 'stage', 'generation'] = '{}'::jsonb
                AND (payload ->> 'course') ~* '^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$'
                AND (payload ->> 'stage') = ANY (ARRAY['notify', 'archiveStudentRecords', 'deleteStudentRecords'])
                AND jsonb_typeof(payload -> 'generation') = 'number'
                AND (payload ->> 'generation') ~ '^[1-9][0-9]{0,18}$'
                AND (payload ->> 'generation')::numeric <= 9223372036854775807
            WHEN 'recalculateAssignment' THEN
                payload ?& ARRAY['kind', 'assignment', 'generation']
                AND payload - ARRAY['kind', 'assignment', 'generation'] = '{}'::jsonb
                AND (payload ->> 'assignment') ~* '^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$'
                AND jsonb_typeof(payload -> 'generation') = 'number'
                AND (payload ->> 'generation') ~ '^[1-9][0-9]{0,18}$'
                AND (payload ->> 'generation')::numeric <= 9223372036854775807
            WHEN 'recalculateCourseItemAnalysis' THEN
                payload ?& ARRAY['kind', 'assignment', 'generation']
                AND payload - ARRAY['kind', 'assignment', 'generation'] = '{}'::jsonb
                AND (payload ->> 'assignment') ~* '^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$'
                AND jsonb_typeof(payload -> 'generation') = 'number'
                AND (payload ->> 'generation') ~ '^[1-9][0-9]{0,18}$'
                AND (payload ->> 'generation')::numeric <= 9223372036854775807
            WHEN 'autoSubmitAttempt' THEN
                payload ?& ARRAY['kind', 'attempt', 'timing_generation']
                AND payload - ARRAY['kind', 'attempt', 'timing_generation'] = '{}'::jsonb
                AND (payload ->> 'attempt') ~* '^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$'
                AND jsonb_typeof(payload -> 'timing_generation') = 'number'
                AND (payload ->> 'timing_generation') ~ '^[1-9][0-9]{0,18}$'
                AND (payload ->> 'timing_generation')::numeric <= 9223372036854775807
            WHEN 'publishPublicAssets' THEN
                payload ?& ARRAY['kind', 'reference']
                AND payload - ARRAY['kind', 'reference'] = '{}'::jsonb
                AND jsonb_typeof(payload -> 'reference') = 'object'
                AND (payload -> 'reference') ?& ARRAY['problem', 'version']
                AND (payload -> 'reference') - ARRAY['problem', 'version'] = '{}'::jsonb
                AND (payload #>> '{reference,problem}') ~* '^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$'
                AND (payload #>> '{reference,version}') ~* '^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$'
            ELSE false
        END
    );

CREATE OR REPLACE FUNCTION public.ple_claim_worker_job(
    p_token uuid,
    p_lease_seconds integer,
    p_kinds text[]
) RETURNS TABLE(job_id uuid, tenant_id uuid, payload jsonb, lease_token uuid, attempt_count integer)
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'pg_catalog', 'public'
    AS $$
BEGIN
    IF p_token IS NULL OR p_lease_seconds NOT BETWEEN 1 AND 900
       OR p_kinds IS NULL OR cardinality(p_kinds) NOT BETWEEN 1 AND 9
       OR NOT (p_kinds <@ ARRAY[
            'recalculateAssignment', 'recalculateCourseItemAnalysis',
            'autoSubmitAttempt', 'retention', 'render', 'export', 'import', 'qtiImport',
            'publishPublicAssets'
       ]::text[]) THEN
        RAISE EXCEPTION 'invalid queue claim arguments' USING ERRCODE = '22023';
    END IF;

    UPDATE public.worker_job AS expired
       SET state = 'dead', lease_token = NULL, lease_expires_at = NULL,
           last_error = 'timed_out', completed_at = transaction_timestamp()
     WHERE expired.state = 'leased'
       AND expired.payload ->> 'kind' = ANY(p_kinds)
       AND expired.lease_expires_at <= transaction_timestamp()
       AND expired.attempt_count >= expired.max_attempts;

    RETURN QUERY
    WITH candidate AS (
        SELECT queued.job_id
          FROM public.worker_job AS queued
         WHERE queued.payload ->> 'kind' = ANY(p_kinds)
           AND ((queued.state = 'ready' AND queued.available_at <= transaction_timestamp())
             OR (queued.state = 'leased' AND queued.lease_expires_at <= transaction_timestamp()
                 AND queued.attempt_count < queued.max_attempts))
         ORDER BY CASE WHEN queued.payload->>'kind' = 'recalculateCourseItemAnalysis' THEN 1 ELSE 0 END,
                  queued.available_at, queued.job_id
         FOR UPDATE SKIP LOCKED
         LIMIT 1
    ), claimed AS (
        UPDATE public.worker_job AS queued
           SET state = 'leased', lease_token = p_token,
               lease_expires_at = transaction_timestamp() + make_interval(secs => p_lease_seconds),
               attempt_count = queued.attempt_count + 1, last_error = NULL, completed_at = NULL
          FROM candidate
         WHERE queued.job_id = candidate.job_id
        RETURNING queued.job_id, queued.tenant_id, queued.payload, queued.lease_token,
                  queued.attempt_count
    )
    SELECT * FROM claimed;
END
$$;

CREATE OR REPLACE FUNCTION public.ple_ready_worker_queue_depth(p_kinds text[]) RETURNS bigint
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'pg_catalog', 'public'
    AS $$
BEGIN
    IF p_kinds IS NULL OR cardinality(p_kinds) NOT BETWEEN 1 AND 9
       OR NOT (p_kinds <@ ARRAY[
            'recalculateAssignment', 'recalculateCourseItemAnalysis',
            'autoSubmitAttempt', 'retention', 'render', 'export', 'import', 'qtiImport',
            'publishPublicAssets'
       ]::text[]) THEN
        RAISE EXCEPTION 'invalid queue depth arguments' USING ERRCODE = '22023';
    END IF;
    RETURN (
        SELECT count(*)::bigint
          FROM public.worker_job
         WHERE state = 'ready'
           AND available_at <= transaction_timestamp()
           AND payload ->> 'kind' = ANY(p_kinds)
    );
END
$$;

CREATE FUNCTION public.ple_activate_public_asset_publication(
    p_job uuid,
    p_token uuid,
    p_problem uuid,
    p_version uuid,
    p_transitions jsonb
) RETURNS boolean
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'pg_catalog', 'public'
    AS $$
DECLARE
    expected_payload jsonb;
BEGIN
    IF p_job IS NULL OR p_token IS NULL OR p_problem IS NULL OR p_version IS NULL
       OR p_transitions IS NULL
       OR jsonb_typeof(p_transitions) <> 'object' THEN
        RAISE EXCEPTION 'invalid public asset publication activation arguments'
            USING ERRCODE = '22023';
    END IF;
    expected_payload := jsonb_build_object(
        'kind', 'publishPublicAssets',
        'reference', jsonb_build_object('problem', p_problem::text, 'version', p_version::text)
    );
    PERFORM 1
      FROM public.problem_version AS version_row
     WHERE version_row.problem_id = p_problem
       AND version_row.version_id = p_version
       AND version_row.publication_scope = 'public';
    IF NOT FOUND THEN
        RAISE EXCEPTION 'public asset publisher may activate only a public version'
            USING ERRCODE = '22023';
    END IF;
    PERFORM 1
      FROM public.worker_job AS job
     WHERE job.job_id = p_job
       AND job.state = 'leased'
       AND job.lease_token = p_token
       AND job.lease_expires_at > transaction_timestamp()
       AND job.payload = expected_payload
     FOR UPDATE;
    IF NOT FOUND THEN
        RETURN false;
    END IF;

    -- A publisher can provide neither an extra row nor a partial batch. The
    -- supplied payload is restricted to exactly the registry's mechanical
    -- transition, so this capability cannot alter a published asset binding.
    IF EXISTS (
        SELECT 1
          FROM public.asset_delivery AS ad
         WHERE ad.delivery_kind = 'catalog'
           AND ad.problem_id = p_problem AND ad.version_id = p_version
           AND ad.payload ->> 'publication' = 'pending'
           AND NOT (p_transitions ? ad.delivery_id::text)
    ) OR EXISTS (
        SELECT 1
          FROM jsonb_each(p_transitions) AS supplied(delivery_text, transition)
          LEFT JOIN public.asset_delivery AS ad
            ON ad.delivery_id::text = supplied.delivery_text
           AND ad.delivery_kind = 'catalog'
           AND ad.problem_id = p_problem AND ad.version_id = p_version
           AND ad.payload ->> 'publication' = 'pending'
         WHERE ad.delivery_id IS NULL
            OR jsonb_typeof(supplied.transition) <> 'object'
            OR NOT (supplied.transition ?& ARRAY['payload', 'payloadSha256'])
            OR supplied.transition - ARRAY['payload', 'payloadSha256'] <> '{}'::jsonb
            OR jsonb_typeof(supplied.transition -> 'payload') <> 'object'
            OR supplied.transition -> 'payload' <>
                ((ad.payload - 'pendingSource') || jsonb_build_object('publication', 'ready'))
            OR (supplied.transition ->> 'payloadSha256') !~ '^[0-9a-f]{64}$'
    ) THEN
        RAISE EXCEPTION 'public asset publication transition does not match pending registry'
            USING ERRCODE = '22023';
    END IF;

    UPDATE public.asset_delivery AS ad
       SET payload = supplied.transition -> 'payload',
           payload_sha256 = supplied.transition ->> 'payloadSha256'
      FROM jsonb_each(p_transitions) AS supplied(delivery_text, transition)
     WHERE ad.delivery_id::text = supplied.delivery_text
       AND ad.delivery_kind = 'catalog'
       AND ad.problem_id = p_problem AND ad.version_id = p_version
       AND ad.payload ->> 'publication' = 'pending';

    UPDATE public.worker_job
       SET state = 'completed', lease_token = NULL, lease_expires_at = NULL,
           completed_at = transaction_timestamp()
     WHERE job_id = p_job AND state = 'leased' AND lease_token = p_token;
    IF NOT FOUND THEN
        RAISE EXCEPTION 'public asset publication lease changed while activating'
            USING ERRCODE = '40001';
    END IF;
    RETURN true;
END
$$;

ALTER FUNCTION public.ple_claim_worker_job(uuid, integer, text[]) OWNER TO ple_queue_broker;
ALTER FUNCTION public.ple_ready_worker_queue_depth(text[]) OWNER TO ple_queue_broker;
ALTER FUNCTION public.ple_activate_public_asset_publication(uuid, uuid, uuid, uuid, jsonb)
    OWNER TO ple_queue_broker;

REVOKE ALL ON FUNCTION public.ple_activate_public_asset_publication(uuid, uuid, uuid, uuid, jsonb)
    FROM PUBLIC;
GRANT EXECUTE ON FUNCTION public.ple_activate_public_asset_publication(uuid, uuid, uuid, uuid, jsonb)
    TO ple_app;
