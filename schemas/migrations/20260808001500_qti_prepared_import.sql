-- QTI worker preparation remains private until its queue lease commits it.
ALTER TABLE workspace_qti_import
    ADD COLUMN state text NOT NULL DEFAULT 'committed'
        CHECK (state IN ('prepared', 'committed'));

CREATE INDEX workspace_qti_import_committed_idx
    ON workspace_qti_import (tenant_id, workspace_id, import_id)
    WHERE state = 'committed';

-- The staging capability is the only no-login principal that can inspect a
-- hidden registry or join it to answer-bearing grading rows.  Neither the
-- app nor the dedicated grader receives those table reads directly.
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'ple_qti_staging_broker') THEN
        CREATE ROLE ple_qti_staging_broker NOLOGIN NOSUPERUSER NOCREATEDB
            NOCREATEROLE NOINHERIT BYPASSRLS;
    END IF;
END
$$;
GRANT USAGE ON SCHEMA public TO ple_qti_staging_broker;
GRANT SELECT, UPDATE ON worker_job TO ple_qti_staging_broker;
GRANT SELECT, UPDATE ON workspace_qti_import TO ple_qti_staging_broker;
GRANT SELECT ON workspace_qti_import_grading TO ple_qti_staging_broker;

CREATE FUNCTION ple_qti_import_is_prepared(
    p_tenant uuid,
    p_workspace uuid,
    p_import uuid
)
RETURNS boolean
LANGUAGE sql
STABLE
SECURITY DEFINER
SET search_path = pg_catalog, public
AS $$
    SELECT p_tenant = public.ple_current_tenant() AND EXISTS(
        SELECT 1 FROM public.workspace_qti_import AS registry
         WHERE registry.tenant_id = p_tenant
           AND registry.workspace_id = p_workspace
           AND registry.import_id = p_import
           AND registry.state = 'prepared'
    )
$$;
ALTER FUNCTION ple_qti_import_is_prepared(uuid, uuid, uuid)
    OWNER TO ple_qti_staging_broker;
REVOKE ALL ON FUNCTION ple_qti_import_is_prepared(uuid, uuid, uuid) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_qti_import_is_prepared(uuid, uuid, uuid) TO ple_app;

-- Application writes remain tenant-scoped, but a prepared row is not a
-- generally readable educational record. Committed projection and prepared
-- retry comparison are provided by the narrow functions below.
DROP POLICY workspace_qti_import_tenant ON workspace_qti_import;
CREATE POLICY workspace_qti_import_app_insert ON workspace_qti_import
    FOR INSERT TO ple_app
    WITH CHECK (tenant_id = ple_current_tenant() AND state = 'prepared');
REVOKE SELECT ON workspace_qti_import, workspace_qti_import_item,
    workspace_qti_import_asset, workspace_qti_import_unsupported FROM ple_app;

-- Child rows can be written only while their exact parent is hidden. Once a
-- lease commits it, the immutable set cannot be appended through the app role.
DROP POLICY workspace_qti_import_item_tenant ON workspace_qti_import_item;
CREATE POLICY workspace_qti_import_item_app_prepared_insert
    ON workspace_qti_import_item FOR INSERT TO ple_app
    WITH CHECK (
        tenant_id = ple_current_tenant()
        AND ple_qti_import_is_prepared(tenant_id, workspace_id, import_id)
    );
DROP POLICY workspace_qti_import_asset_tenant ON workspace_qti_import_asset;
CREATE POLICY workspace_qti_import_asset_app_prepared_insert
    ON workspace_qti_import_asset FOR INSERT TO ple_app
    WITH CHECK (
        tenant_id = ple_current_tenant()
        AND ple_qti_import_is_prepared(tenant_id, workspace_id, import_id)
    );
DROP POLICY workspace_qti_import_unsupported_tenant ON workspace_qti_import_unsupported;
CREATE POLICY workspace_qti_import_unsupported_app_prepared_insert
    ON workspace_qti_import_unsupported FOR INSERT TO ple_app
    WITH CHECK (
        tenant_id = ple_current_tenant()
        AND ple_qti_import_is_prepared(tenant_id, workspace_id, import_id)
    );
DROP POLICY workspace_qti_import_grading_app_insert ON workspace_qti_import_grading;
CREATE POLICY workspace_qti_import_grading_app_prepared_insert
    ON workspace_qti_import_grading FOR INSERT TO ple_app
    WITH CHECK (
        tenant_id = ple_current_tenant()
        AND ple_qti_import_is_prepared(tenant_id, workspace_id, import_id)
    );

-- A grader must not bypass the committed-state predicate by selecting the
-- table directly, even when a prepared record shares its tenant setting.
DROP POLICY workspace_qti_import_grading_grader_select
    ON workspace_qti_import_grading;
REVOKE SELECT ON workspace_qti_import_grading FROM ple_qti_grader;

-- The original queue shape check predates the closed QTI payload family.
-- Replace only that check, preserving strict exact-key validation for every
-- existing payload rather than widening the queue to arbitrary JSON.
ALTER TABLE worker_job DROP CONSTRAINT worker_job_payload_check;
ALTER TABLE worker_job ADD CONSTRAINT worker_job_payload_check CHECK (
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
    ) OR (
        payload->>'kind' = 'export'
        AND payload ?& ARRAY['kind', 'deliveryObject']
        AND payload - ARRAY['kind', 'deliveryObject'] = '{}'::jsonb
        AND jsonb_typeof(payload->'deliveryObject') = 'string'
        AND payload->>'deliveryObject' ~* '^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$'
    ) OR (
        payload->>'kind' = 'import'
        AND payload ?& ARRAY['kind', 'sourceObject']
        AND payload - ARRAY['kind', 'sourceObject'] = '{}'::jsonb
        AND jsonb_typeof(payload->'sourceObject') = 'string'
        AND payload->>'sourceObject' ~* '^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$'
    ) OR (
        payload->>'kind' = 'qtiImport'
        AND payload ?& ARRAY['kind', 'workspace', 'import', 'sourceObject']
        AND payload - ARRAY['kind', 'workspace', 'import', 'sourceObject'] = '{}'::jsonb
        AND jsonb_typeof(payload->'workspace') = 'string'
        AND jsonb_typeof(payload->'import') = 'string'
        AND jsonb_typeof(payload->'sourceObject') = 'string'
        AND payload->>'workspace' ~* '^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$'
        AND payload->>'import' ~* '^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$'
        AND payload->>'sourceObject' ~* '^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$'
    )
);

-- No application role receives UPDATE on worker_job or the QTI registry.
-- This capability accepts exactly one active QTI lease, promotes exactly its
-- hidden registry, and completes exactly that job in one transaction.
CREATE FUNCTION ple_commit_prepared_qti_import(
    p_tenant uuid,
    p_job uuid,
    p_lease_token uuid,
    p_workspace uuid,
    p_import uuid,
    p_source_object uuid
)
RETURNS boolean
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = pg_catalog, public
AS $$
DECLARE
    committed boolean;
BEGIN
    IF p_tenant IS NULL OR p_job IS NULL OR p_lease_token IS NULL
       OR p_workspace IS NULL OR p_import IS NULL OR p_source_object IS NULL
       OR p_tenant <> public.ple_current_tenant() THEN
        RAISE EXCEPTION 'invalid QTI prepared-import commit capability' USING ERRCODE = '22023';
    END IF;
    WITH eligible AS (
        SELECT job_id
          FROM public.worker_job
         WHERE job_id = p_job
           AND tenant_id = p_tenant
           AND state = 'leased'
           AND lease_token = p_lease_token
           AND lease_expires_at > transaction_timestamp()
           AND payload = jsonb_build_object(
                'kind', 'qtiImport',
                'workspace', p_workspace::text,
                'import', p_import::text,
                'sourceObject', p_source_object::text)
         FOR UPDATE
    ), promoted AS (
        UPDATE public.workspace_qti_import AS registry
           SET state = 'committed'
          FROM eligible
         WHERE registry.tenant_id = p_tenant
           AND registry.workspace_id = p_workspace
           AND registry.import_id = p_import
           AND registry.source_object_id = p_source_object
           AND registry.state = 'prepared'
        RETURNING registry.import_id
    ), completed AS (
        UPDATE public.worker_job AS job
           SET state = 'completed',
               lease_token = NULL,
               lease_expires_at = NULL,
               completed_at = transaction_timestamp()
          FROM promoted
         WHERE job.job_id = p_job
           AND job.tenant_id = p_tenant
           AND job.state = 'leased'
           AND job.lease_token = p_lease_token
           AND job.lease_expires_at > transaction_timestamp()
        RETURNING 1
    )
    SELECT EXISTS(SELECT 1 FROM completed) INTO committed;
    RETURN committed;
END
$$;

ALTER FUNCTION ple_commit_prepared_qti_import(uuid, uuid, uuid, uuid, uuid, uuid)
    OWNER TO ple_qti_staging_broker;

REVOKE ALL ON FUNCTION ple_commit_prepared_qti_import(uuid, uuid, uuid, uuid, uuid, uuid)
    FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_commit_prepared_qti_import(uuid, uuid, uuid, uuid, uuid, uuid)
    TO ple_app;

-- The grader remains blind to archive/registry metadata.  This narrow reader
-- makes answer material available only after the registry was atomically
-- committed, under its own tenant setting.
CREATE FUNCTION ple_read_committed_qti_grading(
    p_tenant uuid,
    p_workspace uuid,
    p_import uuid,
    p_item_id text
)
RETURNS TABLE(payload bytea, payload_sha256 character(64))
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = pg_catalog, public
AS $$
BEGIN
    IF p_tenant IS NULL OR p_workspace IS NULL OR p_import IS NULL
       OR p_item_id IS NULL OR p_tenant <> public.ple_current_tenant() THEN
        RAISE EXCEPTION 'invalid QTI grading read capability' USING ERRCODE = '22023';
    END IF;
    RETURN QUERY
    SELECT grading.payload, grading.payload_sha256
      FROM public.workspace_qti_import_grading AS grading
      JOIN public.workspace_qti_import AS registry
        ON registry.tenant_id = grading.tenant_id
       AND registry.workspace_id = grading.workspace_id
       AND registry.import_id = grading.import_id
     WHERE grading.tenant_id = p_tenant
       AND grading.workspace_id = p_workspace
       AND grading.import_id = p_import
       AND grading.item_id = p_item_id
       AND registry.state = 'committed';
END
$$;

ALTER FUNCTION ple_read_committed_qti_grading(uuid, uuid, uuid, text)
    OWNER TO ple_qti_staging_broker;

REVOKE ALL ON FUNCTION ple_read_committed_qti_grading(uuid, uuid, uuid, text) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_read_committed_qti_grading(uuid, uuid, uuid, text)
    TO ple_qti_grader;

CREATE FUNCTION ple_read_committed_qti_import(
    p_tenant uuid,
    p_workspace uuid,
    p_import uuid
)
RETURNS TABLE(payload jsonb, payload_sha256 character(64))
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = pg_catalog, public
AS $$
BEGIN
    IF p_tenant IS NULL OR p_workspace IS NULL OR p_import IS NULL
       OR p_tenant <> public.ple_current_tenant() THEN
        RAISE EXCEPTION 'invalid QTI registry read capability' USING ERRCODE = '22023';
    END IF;
    RETURN QUERY
    SELECT registry.payload, registry.payload_sha256
      FROM public.workspace_qti_import AS registry
     WHERE registry.tenant_id = p_tenant
       AND registry.workspace_id = p_workspace
       AND registry.import_id = p_import
       AND registry.state = 'committed';
END
$$;
ALTER FUNCTION ple_read_committed_qti_import(uuid, uuid, uuid)
    OWNER TO ple_qti_staging_broker;
REVOKE ALL ON FUNCTION ple_read_committed_qti_import(uuid, uuid, uuid) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_read_committed_qti_import(uuid, uuid, uuid) TO ple_app;

-- Retry comparison sees only equality, never a hidden row or its answers.
-- The caller supplies canonical registry and per-item grading digests; a
-- divergent replay remains a conflict rather than silently retaining a key.
CREATE FUNCTION ple_prepared_qti_import_matches(
    p_tenant uuid,
    p_workspace uuid,
    p_import uuid,
    p_registry_payload jsonb,
    p_registry_sha256 character(64),
    p_grading_sha256 jsonb
)
RETURNS boolean
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = pg_catalog, public
AS $$
DECLARE
    matches boolean;
BEGIN
    IF p_tenant IS NULL OR p_workspace IS NULL OR p_import IS NULL
       OR p_registry_payload IS NULL OR p_registry_sha256 IS NULL
       OR p_grading_sha256 IS NULL OR p_tenant <> public.ple_current_tenant() THEN
        RAISE EXCEPTION 'invalid QTI prepared-import retry capability' USING ERRCODE = '22023';
    END IF;
    SELECT registry.payload = p_registry_payload
           AND registry.payload_sha256 = p_registry_sha256
           AND COALESCE((
                SELECT jsonb_object_agg(grading.item_id, grading.payload_sha256 ORDER BY grading.item_id)
                  FROM public.workspace_qti_import_grading AS grading
                 WHERE grading.tenant_id = registry.tenant_id
                   AND grading.workspace_id = registry.workspace_id
                   AND grading.import_id = registry.import_id
           ), '{}'::jsonb) = p_grading_sha256
      INTO matches
      FROM public.workspace_qti_import AS registry
     WHERE registry.tenant_id = p_tenant
       AND registry.workspace_id = p_workspace
       AND registry.import_id = p_import
       AND registry.state = 'prepared';
    RETURN COALESCE(matches, false);
END
$$;
ALTER FUNCTION ple_prepared_qti_import_matches(uuid, uuid, uuid, jsonb, character(64), jsonb)
    OWNER TO ple_qti_staging_broker;
REVOKE ALL ON FUNCTION ple_prepared_qti_import_matches(uuid, uuid, uuid, jsonb, character(64), jsonb)
    FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_prepared_qti_import_matches(uuid, uuid, uuid, jsonb, character(64), jsonb)
    TO ple_app;
