-- MOD-ADP-QTI / MOD-STO: immutable published QTI answer bindings.
--
-- A publication copies exactly one selected item from a committed private
-- import. The browser never receives this table or either function below.

CREATE TABLE published_qti_grading (
    problem_id uuid NOT NULL,
    version_id uuid NOT NULL,
    item_id text NOT NULL CHECK (length(item_id) BETWEEN 1 AND 512),
    payload bytea NOT NULL CHECK (octet_length(payload) BETWEEN 1 AND 262144),
    payload_sha256 character(64) NOT NULL,
    created_at timestamptz NOT NULL DEFAULT transaction_timestamp(),
    PRIMARY KEY (problem_id, version_id, item_id),
    FOREIGN KEY (problem_id, version_id)
        REFERENCES problem_version(problem_id, version_id)
        ON DELETE RESTRICT
        DEFERRABLE INITIALLY DEFERRED
);

ALTER TABLE published_qti_grading ENABLE ROW LEVEL SECURITY;
ALTER TABLE published_qti_grading FORCE ROW LEVEL SECURITY;

-- The existing broker is a narrowly provisioned SECURITY DEFINER owner. It
-- alone reads staging grading bytes and inserts their immutable destination.
GRANT SELECT ON workspace_qti_import_item TO ple_qti_staging_broker;
GRANT INSERT, SELECT ON published_qti_grading TO ple_qti_staging_broker;
CREATE POLICY published_qti_grading_broker ON published_qti_grading
    FOR ALL TO ple_qti_staging_broker
    USING (true) WITH CHECK (true);

CREATE FUNCTION ple_promote_qti_grading(
    p_tenant uuid,
    p_workspace uuid,
    p_import uuid,
    p_problem uuid,
    p_version uuid,
    p_item_id text
)
RETURNS boolean
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = pg_catalog, public
AS $$
DECLARE
    copied_count bigint;
BEGIN
    IF p_tenant IS NULL OR p_workspace IS NULL OR p_import IS NULL
       OR p_problem IS NULL OR p_version IS NULL OR p_item_id IS NULL
       OR p_tenant <> public.ple_current_tenant() THEN
        RAISE EXCEPTION 'invalid QTI publication promotion capability'
            USING ERRCODE = '22023';
    END IF;

    -- Lock the committed registry and selected safe item before copying its
    -- hidden grading bytes. A stale or foreign staging reference simply does
    -- not promote anything, allowing the application transaction to roll back.
    PERFORM 1
      FROM public.workspace_qti_import AS registry
      JOIN public.workspace_qti_import_item AS item
        ON item.tenant_id = registry.tenant_id
       AND item.workspace_id = registry.workspace_id
       AND item.import_id = registry.import_id
     WHERE registry.tenant_id = p_tenant
       AND registry.workspace_id = p_workspace
       AND registry.import_id = p_import
       AND registry.state = 'committed'
       AND item.item_id = p_item_id
     FOR KEY SHARE OF registry, item;
    IF NOT FOUND THEN
        RETURN false;
    END IF;

    INSERT INTO public.published_qti_grading
        (problem_id, version_id, item_id, payload, payload_sha256)
    SELECT p_problem, p_version, grading.item_id, grading.payload,
           grading.payload_sha256
      FROM public.workspace_qti_import_grading AS grading
     WHERE grading.tenant_id = p_tenant
       AND grading.workspace_id = p_workspace
       AND grading.import_id = p_import
       AND grading.item_id = p_item_id
    ON CONFLICT (problem_id, version_id, item_id) DO NOTHING;
    GET DIAGNOSTICS copied_count = ROW_COUNT;
    RETURN copied_count = 1;
END
$$;

ALTER FUNCTION ple_promote_qti_grading(uuid, uuid, uuid, uuid, uuid, text)
    OWNER TO ple_qti_staging_broker;
REVOKE ALL ON FUNCTION ple_promote_qti_grading(uuid, uuid, uuid, uuid, uuid, text)
    FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_promote_qti_grading(uuid, uuid, uuid, uuid, uuid, text)
    TO ple_app;

-- The grader invokes a narrow reader with its tenant setting. Shared catalog
-- content remains reusable across tenants, while an unauthenticated or
-- browser connection cannot call into the answer-bearing table.
CREATE FUNCTION ple_read_published_qti_grading(
    p_tenant uuid,
    p_problem uuid,
    p_version uuid,
    p_item_id text
)
RETURNS TABLE(payload bytea, payload_sha256 character(64))
LANGUAGE plpgsql
STABLE
SECURITY DEFINER
SET search_path = pg_catalog, public
AS $$
BEGIN
    IF p_tenant IS NULL OR p_problem IS NULL OR p_version IS NULL
       OR p_item_id IS NULL OR p_tenant <> public.ple_current_tenant() THEN
        RAISE EXCEPTION 'invalid published QTI grading read capability'
            USING ERRCODE = '22023';
    END IF;
    RETURN QUERY
    SELECT grading.payload, grading.payload_sha256
      FROM public.published_qti_grading AS grading
     JOIN public.problem_version AS version_row
        ON version_row.problem_id = grading.problem_id
       AND version_row.version_id = grading.version_id
     WHERE grading.problem_id = p_problem
       AND grading.version_id = p_version
       AND grading.item_id = p_item_id
       AND version_row.backend = 'qti'
       AND (
            version_row.publication_scope = 'public'
            OR EXISTS (
                SELECT 1
                  FROM public.catalog_tenant_grant AS grant_row
                 WHERE grant_row.tenant_id = p_tenant
                   AND grant_row.problem_id = version_row.problem_id
                   AND grant_row.version_id = version_row.version_id
            )
       );
END
$$;

ALTER FUNCTION ple_read_published_qti_grading(uuid, uuid, uuid, text)
    OWNER TO ple_qti_staging_broker;
REVOKE ALL ON FUNCTION ple_read_published_qti_grading(uuid, uuid, uuid, text)
    FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_read_published_qti_grading(uuid, uuid, uuid, text)
    TO ple_qti_grader;
