-- MOD-WORKER: a small durable queue. Tenant application reads and enqueues
-- under forced RLS; cross-tenant leasing is available only through the narrow
-- SECURITY DEFINER broker functions below.

DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'ple_queue_broker') THEN
        -- This role owns no application login and receives no table grants
        -- outside this queue. BYPASSRLS is needed solely because the broker
        -- claims work across tenant rows while worker code never can.
        CREATE ROLE ple_queue_broker NOLOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE
            NOINHERIT BYPASSRLS;
    END IF;
END
$$;

CREATE TABLE worker_job (
    job_id uuid PRIMARY KEY,
    tenant_id uuid NOT NULL,
    payload jsonb NOT NULL
        CHECK (jsonb_typeof(payload) = 'object')
        CHECK (
            (
                payload->>'kind' = 'render'
                AND payload ?& ARRAY['kind', 'reference', 'seed']
                AND payload - ARRAY['kind', 'reference', 'seed'] = '{}'::jsonb
                AND jsonb_typeof(payload->'reference') = 'object'
                AND (payload->'reference') ?& ARRAY['problem', 'version']
                AND (payload->'reference') - ARRAY['problem', 'version'] = '{}'::jsonb
                AND jsonb_typeof(payload->'reference'->'problem') = 'string'
                AND jsonb_typeof(payload->'reference'->'version') = 'string'
                AND payload->'reference'->>'problem' ~* '^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$'
                AND payload->'reference'->>'version' ~* '^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$'
                AND jsonb_typeof(payload->'seed') = 'number'
                AND payload->>'seed' ~ '^(0|[1-9][0-9]{0,19})$'
                AND (payload->>'seed')::numeric <= 18446744073709551615
            )
            OR (
                payload->>'kind' = 'export'
                AND payload ?& ARRAY['kind', 'deliveryObject']
                AND payload - ARRAY['kind', 'deliveryObject'] = '{}'::jsonb
                AND jsonb_typeof(payload->'deliveryObject') = 'string'
                AND payload->>'deliveryObject' ~* '^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$'
            )
            OR (
                payload->>'kind' = 'import'
                AND payload ?& ARRAY['kind', 'sourceObject']
                AND payload - ARRAY['kind', 'sourceObject'] = '{}'::jsonb
                AND jsonb_typeof(payload->'sourceObject') = 'string'
                AND payload->>'sourceObject' ~* '^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$'
            )
        ),
    state text NOT NULL
        CHECK (state IN ('ready', 'leased', 'completed', 'dead')),
    available_at timestamptz NOT NULL DEFAULT transaction_timestamp(),
    lease_token uuid,
    lease_expires_at timestamptz,
    attempt_count integer NOT NULL DEFAULT 0
        CHECK (attempt_count >= 0),
    max_attempts integer NOT NULL
        CHECK (max_attempts BETWEEN 1 AND 20),
    last_error text
        CHECK (last_error IS NULL OR last_error IN ('transient', 'permanent', 'timed_out')),
    created_at timestamptz NOT NULL DEFAULT transaction_timestamp(),
    completed_at timestamptz,
    CHECK (attempt_count <= max_attempts),
    CHECK (
        (state = 'leased' AND lease_token IS NOT NULL AND lease_expires_at IS NOT NULL
            AND completed_at IS NULL)
        OR
        (state = 'ready' AND lease_token IS NULL AND lease_expires_at IS NULL
            AND completed_at IS NULL)
        OR
        (state = 'completed' AND lease_token IS NULL AND lease_expires_at IS NULL
            AND completed_at IS NOT NULL)
        OR
        (state = 'dead' AND lease_token IS NULL AND lease_expires_at IS NULL
            AND completed_at IS NOT NULL AND last_error IS NOT NULL)
    )
);

CREATE INDEX worker_job_claim_ready_idx
    ON worker_job (available_at, job_id)
    WHERE state = 'ready';
CREATE INDEX worker_job_expired_lease_idx
    ON worker_job (lease_expires_at, job_id)
    WHERE state = 'leased';
CREATE INDEX worker_job_tenant_lookup_idx
    ON worker_job (tenant_id, job_id);

ALTER TABLE worker_job ENABLE ROW LEVEL SECURITY;
ALTER TABLE worker_job FORCE ROW LEVEL SECURITY;
CREATE POLICY worker_job_tenant_select ON worker_job
    FOR SELECT TO ple_app
    USING (tenant_id = ple_current_tenant());
CREATE POLICY worker_job_tenant_insert ON worker_job
    FOR INSERT TO ple_app
    WITH CHECK (tenant_id = ple_current_tenant() AND state = 'ready');

-- The claim caller supplies a fresh opaque token. The database supplies every
-- timestamp and atomically selects one ready/expired lease with SKIP LOCKED.
CREATE FUNCTION ple_claim_worker_job(p_token uuid, p_lease_seconds integer)
RETURNS TABLE (
    job_id uuid,
    tenant_id uuid,
    payload jsonb,
    lease_token uuid,
    attempt_count integer
)
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = pg_catalog, public
AS $$
BEGIN
    IF p_token IS NULL OR p_lease_seconds NOT BETWEEN 1 AND 900 THEN
        RAISE EXCEPTION 'invalid queue claim arguments' USING ERRCODE = '22023';
    END IF;

    -- A process that dies on its final lease cannot strand an operational row.
    UPDATE public.worker_job
       SET state = 'dead',
           lease_token = NULL,
           lease_expires_at = NULL,
           last_error = 'timed_out',
           completed_at = transaction_timestamp()
     WHERE state = 'leased'
       AND lease_expires_at <= transaction_timestamp()
       AND attempt_count >= max_attempts;

    RETURN QUERY
    WITH candidate AS (
        SELECT queued.job_id
          FROM public.worker_job AS queued
         WHERE (
                queued.state = 'ready'
            AND queued.available_at <= transaction_timestamp()
         ) OR (
                queued.state = 'leased'
            AND queued.lease_expires_at <= transaction_timestamp()
            AND queued.attempt_count < queued.max_attempts
         )
         ORDER BY queued.available_at, queued.job_id
         FOR UPDATE SKIP LOCKED
         LIMIT 1
    ), claimed AS (
        UPDATE public.worker_job AS queued
           SET state = 'leased',
               lease_token = p_token,
               lease_expires_at = transaction_timestamp()
                    + make_interval(secs => p_lease_seconds),
               attempt_count = queued.attempt_count + 1,
               last_error = NULL,
               completed_at = NULL
          FROM candidate
         WHERE queued.job_id = candidate.job_id
        RETURNING queued.job_id, queued.tenant_id, queued.payload,
                  queued.lease_token, queued.attempt_count
    )
    SELECT * FROM claimed;
END
$$;

CREATE FUNCTION ple_complete_worker_job(p_job_id uuid, p_token uuid)
RETURNS boolean
LANGUAGE sql
SECURITY DEFINER
SET search_path = pg_catalog, public
AS $$
    WITH completed AS (
        UPDATE public.worker_job
           SET state = 'completed',
               lease_token = NULL,
               lease_expires_at = NULL,
               completed_at = transaction_timestamp()
         WHERE job_id = p_job_id
           AND state = 'leased'
           AND lease_token = p_token
           AND lease_expires_at > transaction_timestamp()
        RETURNING 1
    )
    SELECT EXISTS(SELECT 1 FROM completed)
$$;

CREATE FUNCTION ple_fail_worker_job(p_job_id uuid, p_token uuid, p_failure text)
RETURNS text
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = pg_catalog, public
AS $$
DECLARE
    next_state text;
BEGIN
    IF p_failure NOT IN ('transient', 'permanent', 'timed_out') THEN
        RAISE EXCEPTION 'invalid queue failure kind' USING ERRCODE = '22023';
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
     WHERE job_id = p_job_id
       AND state = 'leased'
       AND lease_token = p_token
       AND lease_expires_at > transaction_timestamp()
    RETURNING state INTO next_state;

    IF next_state IS NULL THEN
        RETURN NULL;
    END IF;
    RETURN CASE WHEN next_state = 'dead' THEN 'dead' ELSE 'retrying' END;
END
$$;

CREATE FUNCTION ple_ready_worker_queue_depth()
RETURNS bigint
LANGUAGE sql
SECURITY DEFINER
SET search_path = pg_catalog, public
AS $$
    SELECT count(*)::bigint
      FROM public.worker_job
     WHERE state = 'ready'
       AND available_at <= transaction_timestamp()
$$;

ALTER FUNCTION ple_claim_worker_job(uuid, integer) OWNER TO ple_queue_broker;
ALTER FUNCTION ple_complete_worker_job(uuid, uuid) OWNER TO ple_queue_broker;
ALTER FUNCTION ple_fail_worker_job(uuid, uuid, text) OWNER TO ple_queue_broker;
ALTER FUNCTION ple_ready_worker_queue_depth() OWNER TO ple_queue_broker;

GRANT USAGE ON SCHEMA public TO ple_queue_broker;
REVOKE ALL ON worker_job FROM PUBLIC, ple_app, ple_student, ple_grader;
REVOKE ALL ON FUNCTION ple_claim_worker_job(uuid, integer) FROM PUBLIC;
REVOKE ALL ON FUNCTION ple_complete_worker_job(uuid, uuid) FROM PUBLIC;
REVOKE ALL ON FUNCTION ple_fail_worker_job(uuid, uuid, text) FROM PUBLIC;
REVOKE ALL ON FUNCTION ple_ready_worker_queue_depth() FROM PUBLIC;
GRANT SELECT, INSERT ON worker_job TO ple_app;
-- The no-login function owner is the only principal allowed to bypass the
-- tenant filter, and it receives just the two table privileges its broker SQL
-- uses. Runtime workers get only EXECUTE on these functions through ple_app.
GRANT SELECT, UPDATE ON worker_job TO ple_queue_broker;
GRANT EXECUTE ON FUNCTION ple_claim_worker_job(uuid, integer) TO ple_app;
GRANT EXECUTE ON FUNCTION ple_complete_worker_job(uuid, uuid) TO ple_app;
GRANT EXECUTE ON FUNCTION ple_fail_worker_job(uuid, uuid, text) TO ple_app;
GRANT EXECUTE ON FUNCTION ple_ready_worker_queue_depth() TO ple_app;
