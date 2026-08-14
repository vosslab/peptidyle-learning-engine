-- Initial database epoch: catalog authoring.

-- These catalog capabilities are declared before their protected tables so
-- ownership and grants can be established as one dependency-ordered epoch.
-- PostgreSQL validates the SQL bodies on first invocation after every target
-- relation exists; this narrow migration-local exception is required only for
-- those genuine forward references.
SET check_function_bodies = false;

-- `problem` visibility depends on visible versions, so a problem-version RLS
-- policy cannot query `problem` directly without recursing. This closed
-- predicate runs as the dedicated ownership broker and returns only whether
-- the current tenant owns the named problem.
CREATE FUNCTION public.ple_problem_owned_by_current_tenant(p_problem uuid) RETURNS boolean
    LANGUAGE sql STABLE SECURITY DEFINER
    SET search_path TO 'pg_catalog', 'public'
    AS $$
    SELECT p_problem IS NOT NULL
       AND public.ple_current_tenant() IS NOT NULL
       AND EXISTS (
            SELECT 1
              FROM public.problem AS owner_problem
             WHERE owner_problem.problem_id = p_problem
               AND owner_problem.owner_tenant_id = public.ple_current_tenant()
       )
$$;

ALTER FUNCTION public.ple_problem_owned_by_current_tenant(uuid)
    OWNER TO ple_catalog_ownership_broker;

CREATE FUNCTION public.ple_prepared_qti_import_matches(p_tenant uuid, p_workspace uuid, p_import uuid, p_registry_payload jsonb, p_registry_sha256 character, p_grading_sha256 jsonb) RETURNS boolean
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'pg_catalog', 'public'
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

CREATE FUNCTION public.ple_qti_import_is_prepared(p_tenant uuid, p_workspace uuid, p_import uuid) RETURNS boolean
    LANGUAGE sql STABLE SECURITY DEFINER
    SET search_path TO 'pg_catalog', 'public'
    AS $$
    SELECT p_tenant = public.ple_current_tenant() AND EXISTS(
        SELECT 1 FROM public.workspace_qti_import AS registry
         WHERE registry.tenant_id = p_tenant
           AND registry.workspace_id = p_workspace
           AND registry.import_id = p_import
           AND registry.state = 'prepared'
    )
$$;

CREATE FUNCTION public.ple_read_committed_qti_grading(p_tenant uuid, p_workspace uuid, p_import uuid, p_item_id text) RETURNS TABLE(payload bytea, payload_sha256 character)
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'pg_catalog', 'public'
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

CREATE FUNCTION public.ple_read_committed_qti_import(p_tenant uuid, p_workspace uuid, p_import uuid) RETURNS TABLE(payload jsonb, payload_sha256 character)
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'pg_catalog', 'public'
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

CREATE FUNCTION public.ple_read_published_qti_grading(p_tenant uuid, p_problem uuid, p_version uuid, p_item_id text) RETURNS TABLE(payload bytea, payload_sha256 character)
    LANGUAGE plpgsql STABLE SECURITY DEFINER
    SET search_path TO 'pg_catalog', 'public'
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

-- The grader connection is intentionally the only caller that can retrieve
-- answer-bearing flat-question material.  The public problem payload is used
-- solely to prove that this is the closed native flat-question family; it is
-- never returned from this capability.
CREATE FUNCTION public.ple_flat_question_grading_material(
    p_tenant uuid,
    p_problem uuid,
    p_version uuid
) RETURNS TABLE(key_payload jsonb, key_sha256 character(64))
    LANGUAGE plpgsql STABLE SECURITY DEFINER
    SET search_path TO 'pg_catalog', 'public'
    AS $$
BEGIN
    IF p_tenant IS NULL OR p_problem IS NULL OR p_version IS NULL
       OR p_tenant <> public.ple_current_tenant() THEN
        RAISE EXCEPTION 'invalid flat-question grading read capability'
            USING ERRCODE = '22023';
    END IF;

    RETURN QUERY
    SELECT answer.key_payload, answer.key_sha256
      FROM public.answer_key AS answer
      JOIN public.problem_version AS version_row
        ON version_row.problem_id = answer.problem_id
       AND version_row.version_id = answer.version_id
      JOIN public.problem_version_payload AS version_payload
        ON version_payload.problem_id = version_row.problem_id
       AND version_payload.version_id = version_row.version_id
     WHERE answer.problem_id = p_problem
       AND answer.version_id = p_version
       AND version_row.backend = 'native'::text
       AND version_payload.payload #>> '{question,source,backend}' = 'native'::text
       AND version_payload.payload #>> '{question,source,family}' = 'flat_single_choice_v1'::text
       AND (
            version_row.publication_scope = 'public'::text
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

ALTER FUNCTION public.ple_prepared_qti_import_matches(uuid, uuid, uuid, jsonb, character, jsonb) OWNER TO ple_qti_staging_broker;

ALTER FUNCTION public.ple_qti_import_is_prepared(uuid, uuid, uuid) OWNER TO ple_qti_staging_broker;

ALTER FUNCTION public.ple_read_committed_qti_grading(uuid, uuid, uuid, text) OWNER TO ple_qti_staging_broker;

ALTER FUNCTION public.ple_read_committed_qti_import(uuid, uuid, uuid) OWNER TO ple_qti_staging_broker;

ALTER FUNCTION public.ple_read_published_qti_grading(uuid, uuid, uuid, text) OWNER TO ple_qti_staging_broker;

ALTER FUNCTION public.ple_flat_question_grading_material(uuid, uuid, uuid) OWNER TO ple_grader;

CREATE TABLE public.answer_key (
    problem_id uuid NOT NULL,
    version_id uuid NOT NULL,
    key_payload jsonb NOT NULL,
    key_sha256 character(64) NOT NULL
);

ALTER TABLE ONLY public.answer_key FORCE ROW LEVEL SECURITY;

CREATE TABLE public.catalog_tenant_grant (
    tenant_id uuid NOT NULL,
    problem_id uuid NOT NULL,
    version_id uuid NOT NULL
);

ALTER TABLE ONLY public.catalog_tenant_grant FORCE ROW LEVEL SECURITY;

CREATE TABLE public.problem (
    problem_id uuid NOT NULL,
    public_id bigint GENERATED ALWAYS AS IDENTITY,
    owner_tenant_id uuid NOT NULL,
    owner_user_id uuid NOT NULL,
    visibility text NOT NULL,
    license text NOT NULL,
    lifecycle text DEFAULT 'published'::text NOT NULL,
    created_at timestamp with time zone DEFAULT transaction_timestamp() NOT NULL
    ,CONSTRAINT problem_public_id_check CHECK ((public_id > 0))
    ,CONSTRAINT problem_visibility_check CHECK ((visibility = ANY (ARRAY['institution'::text, 'public'::text])))
    ,CONSTRAINT problem_lifecycle_check CHECK ((lifecycle = ANY (ARRAY['published'::text, 'deprecated'::text, 'archived'::text])))
);

ALTER TABLE ONLY public.problem FORCE ROW LEVEL SECURITY;

CREATE TABLE public.problem_version (
    problem_id uuid NOT NULL,
    version_id uuid NOT NULL,
    version_number bigint NOT NULL,
    model_schema_version integer DEFAULT 1 NOT NULL,
    content_sha256 character(64) NOT NULL,
    workspace_id uuid NOT NULL,
    title text NOT NULL,
    lifecycle text DEFAULT 'published'::text NOT NULL,
    created_at timestamp with time zone DEFAULT transaction_timestamp() NOT NULL,
    backend text DEFAULT 'native'::text NOT NULL,
    capabilities jsonb DEFAULT '[]'::jsonb NOT NULL,
    metadata jsonb DEFAULT '{}'::jsonb NOT NULL,
    publication_scope text DEFAULT 'public'::text NOT NULL,
    lifecycle_reason text,
    authors jsonb DEFAULT '[]'::jsonb NOT NULL,
    previous_version_id uuid,
    derived_from_problem_id uuid,
    derived_from_version_id uuid,
    CONSTRAINT problem_version_authors_check CHECK ((jsonb_typeof(authors) = 'array'::text)),
    CONSTRAINT problem_version_backend_check CHECK ((backend = ANY (ARRAY['native'::text, 'webwork'::text, 'qti'::text, 'h5p'::text, 'imathas'::text]))),
    CONSTRAINT problem_version_capabilities_check CHECK ((jsonb_typeof(capabilities) = 'array'::text)),
    CONSTRAINT problem_version_derived_pair_check CHECK (((derived_from_problem_id IS NULL) = (derived_from_version_id IS NULL))),
    CONSTRAINT problem_version_lifecycle_check CHECK ((lifecycle = ANY (ARRAY['published'::text, 'deprecated'::text, 'archived'::text]))),
    CONSTRAINT problem_version_lifecycle_reason_check CHECK ((((lifecycle = 'published'::text) AND (lifecycle_reason IS NULL)) OR ((lifecycle = ANY (ARRAY['deprecated'::text, 'archived'::text])) AND ((char_length(btrim(lifecycle_reason)) >= 1) AND (char_length(btrim(lifecycle_reason)) <= 1000))))),
    CONSTRAINT problem_version_metadata_check CHECK ((jsonb_typeof(metadata) = 'object'::text)),
    CONSTRAINT problem_version_publication_scope_check CHECK ((publication_scope = ANY (ARRAY['institution'::text, 'public'::text]))),
    CONSTRAINT problem_version_number_check CHECK ((version_number > 0)),
    CONSTRAINT problem_version_model_schema_version_check CHECK ((model_schema_version > 0)),
    CONSTRAINT problem_version_content_sha256_check CHECK ((content_sha256 ~ '^[0-9a-f]{64}$'::text))
);

ALTER TABLE ONLY public.problem_version FORCE ROW LEVEL SECURITY;

CREATE TABLE public.problem_version_payload (
    problem_id uuid NOT NULL,
    version_id uuid NOT NULL,
    payload jsonb NOT NULL,
    payload_sha256 character(64) NOT NULL
)
PARTITION BY HASH (problem_id);

ALTER TABLE ONLY public.problem_version_payload FORCE ROW LEVEL SECURITY;

CREATE TABLE public.published_qti_grading (
    problem_id uuid NOT NULL,
    version_id uuid NOT NULL,
    item_id text NOT NULL,
    payload bytea NOT NULL,
    payload_sha256 character(64) NOT NULL,
    created_at timestamp with time zone DEFAULT transaction_timestamp() NOT NULL,
    CONSTRAINT published_qti_grading_item_id_check CHECK (((char_length(item_id) >= 1) AND (char_length(item_id) <= 1024))),
    CONSTRAINT published_qti_grading_payload_check CHECK (((octet_length(payload) >= 1) AND (octet_length(payload) <= 262144)))
);

CREATE TABLE public.problem_collection (
    collection_id uuid NOT NULL,
    owner_tenant_id uuid NOT NULL,
    owner_user_id uuid NOT NULL,
    title text NOT NULL,
    visibility text NOT NULL,
    created_at timestamp with time zone DEFAULT transaction_timestamp() NOT NULL,
    updated_at timestamp with time zone DEFAULT transaction_timestamp() NOT NULL,
    revision bigint DEFAULT 1 NOT NULL,
    CONSTRAINT problem_collection_title_check CHECK ((char_length(btrim(title)) BETWEEN 1 AND 200)),
    CONSTRAINT problem_collection_visibility_check CHECK ((visibility = ANY (ARRAY['private'::text, 'institution'::text, 'public'::text]))),
    CONSTRAINT problem_collection_revision_check CHECK ((revision > 0))
);

ALTER TABLE ONLY public.problem_collection FORCE ROW LEVEL SECURITY;

CREATE TABLE public.problem_collection_member (
    owner_tenant_id uuid NOT NULL,
    collection_id uuid NOT NULL,
    problem_id uuid NOT NULL,
    version_id uuid NOT NULL,
    position integer NOT NULL,
    created_at timestamp with time zone DEFAULT transaction_timestamp() NOT NULL,
    CONSTRAINT problem_collection_member_position_check CHECK ((position >= 0))
);

ALTER TABLE ONLY public.problem_collection_member FORCE ROW LEVEL SECURITY;

CREATE TABLE public.catalog_search_document (
    problem_id uuid NOT NULL,
    version_id uuid NOT NULL,
    public_id bigint NOT NULL,
    version_number bigint NOT NULL,
    title text NOT NULL,
    backend text NOT NULL,
    metadata jsonb NOT NULL,
    publication_scope text NOT NULL,
    lifecycle text NOT NULL,
    lifecycle_reason text,
    authors jsonb NOT NULL,
    previous_version_id uuid,
    derived_from_problem_id uuid,
    derived_from_version_id uuid,
    published_at timestamp with time zone NOT NULL,
    authors_text text NOT NULL,
    question_type text NOT NULL,
    language text NOT NULL,
    license text NOT NULL,
    taxonomy jsonb DEFAULT '[]'::jsonb NOT NULL,
    keywords jsonb DEFAULT '[]'::jsonb NOT NULL,
    capabilities jsonb DEFAULT '[]'::jsonb NOT NULL,
    search_text tsvector NOT NULL,
    quality_signal numeric(12,6),
    updated_at timestamp with time zone DEFAULT transaction_timestamp() NOT NULL,
    CONSTRAINT catalog_search_document_metadata_check CHECK ((jsonb_typeof(metadata) = 'object'::text)),
    CONSTRAINT catalog_search_document_authors_check CHECK ((jsonb_typeof(authors) = 'array'::text AND jsonb_array_length(authors) > 0)),
    CONSTRAINT catalog_search_document_taxonomy_check CHECK ((jsonb_typeof(taxonomy) = 'array'::text)),
    CONSTRAINT catalog_search_document_keywords_check CHECK ((jsonb_typeof(keywords) = 'array'::text)),
    CONSTRAINT catalog_search_document_capabilities_check CHECK ((jsonb_typeof(capabilities) = 'array'::text))
);

ALTER TABLE ONLY public.published_qti_grading FORCE ROW LEVEL SECURITY;

CREATE TABLE public.published_source_artifact (
    problem_id uuid NOT NULL,
    version_id uuid NOT NULL,
    backend text NOT NULL,
    object_id uuid NOT NULL,
    payload jsonb NOT NULL,
    payload_sha256 character(64) NOT NULL,
    created_at timestamp with time zone DEFAULT transaction_timestamp() NOT NULL,
    CONSTRAINT published_source_artifact_backend_check CHECK ((backend = ANY (ARRAY['native'::text, 'webwork'::text, 'qti'::text, 'h5p'::text, 'imathas'::text]))),
    CONSTRAINT published_source_artifact_payload_check CHECK ((jsonb_typeof(payload) = 'object'::text))
);

ALTER TABLE ONLY public.published_source_artifact FORCE ROW LEVEL SECURITY;

-- Imported flat questions retain tenant-owned lineage outside the public
-- catalog payload. Object keys are deliberately absent: both source and
-- published keys are reconstructed only by their typed Rust owners.
CREATE TABLE public.published_flat_import_origin (
    owner_tenant_id uuid NOT NULL,
    problem_id uuid NOT NULL,
    version_id uuid NOT NULL,
    source_import_id uuid NOT NULL,
    source_archive_object_id uuid NOT NULL,
    source_archive_sha256 character(64) NOT NULL,
    source_item_identifier text NOT NULL,
    profile_id text NOT NULL,
    profile_version text NOT NULL,
    mapping_version text NOT NULL,
    conversion_version text NOT NULL,
    normalized_item_sha256 character(64) NOT NULL,
    profile_report_sha256 character(64) NOT NULL,
    public_mapping_sha256 character(64) NOT NULL,
    private_mapping_sha256 character(64) NOT NULL,
    mapping_sha256 character(64) NOT NULL,
    warning_sha256 character(64) NOT NULL,
    choice_map_sha256 character(64) NOT NULL,
    mapped_canonical_source_sha256 character(64) NOT NULL,
    acknowledged_by uuid NOT NULL,
    acknowledged_at timestamp with time zone NOT NULL,
    published_archive_object_id uuid NOT NULL,
    published_archive_sha256 character(64) NOT NULL,
    published_archive_size_bytes bigint NOT NULL,
    published_archive_media_type text NOT NULL,
    published_archive_license text NOT NULL,
    published_archive_provenance text NOT NULL,
    published_archive_created_at timestamp with time zone NOT NULL,
    created_at timestamp with time zone DEFAULT transaction_timestamp() NOT NULL,
    CONSTRAINT published_flat_import_origin_source_item_identifier_check CHECK ((char_length(source_item_identifier) BETWEEN 1 AND 1024) AND (btrim(source_item_identifier) <> ''::text)),
    CONSTRAINT published_flat_import_origin_profile_check CHECK (((profile_id = 'canvas-qti-1.2-static-single-choice/v1'::text) AND (profile_version = 'v1'::text) AND (mapping_version = 'v1'::text)) OR ((profile_id = 'blackboard-qti-2.1-static-single-choice-pool/v1'::text) AND (profile_version = 'v1'::text) AND (mapping_version = 'v1'::text))),
    CONSTRAINT published_flat_import_origin_conversion_version_check CHECK ((octet_length(conversion_version) BETWEEN 1 AND 128) AND (conversion_version ~ '^[a-z0-9_/-]+$'::text)),
    CONSTRAINT published_flat_import_origin_source_archive_sha256_check CHECK ((source_archive_sha256 ~ '^[0-9a-f]{64}$'::text)),
    CONSTRAINT published_flat_import_origin_normalized_item_sha256_check CHECK ((normalized_item_sha256 ~ '^[0-9a-f]{64}$'::text)),
    CONSTRAINT published_flat_import_origin_profile_report_sha256_check CHECK ((profile_report_sha256 ~ '^[0-9a-f]{64}$'::text)),
    CONSTRAINT published_flat_import_origin_public_mapping_sha256_check CHECK ((public_mapping_sha256 ~ '^[0-9a-f]{64}$'::text)),
    CONSTRAINT published_flat_import_origin_private_mapping_sha256_check CHECK ((private_mapping_sha256 ~ '^[0-9a-f]{64}$'::text)),
    CONSTRAINT published_flat_import_origin_mapping_sha256_check CHECK ((mapping_sha256 ~ '^[0-9a-f]{64}$'::text)),
    CONSTRAINT published_flat_import_origin_warning_sha256_check CHECK ((warning_sha256 ~ '^[0-9a-f]{64}$'::text)),
    CONSTRAINT published_flat_import_origin_choice_map_sha256_check CHECK ((choice_map_sha256 ~ '^[0-9a-f]{64}$'::text)),
    CONSTRAINT published_flat_import_origin_mapped_source_sha256_check CHECK ((mapped_canonical_source_sha256 ~ '^[0-9a-f]{64}$'::text)),
    CONSTRAINT published_flat_import_origin_archive_sha256_check CHECK ((published_archive_sha256 ~ '^[0-9a-f]{64}$'::text)),
    CONSTRAINT published_flat_import_origin_archive_size_check CHECK ((published_archive_size_bytes BETWEEN 1 AND 33554432)),
    CONSTRAINT published_flat_import_origin_archive_media_type_check CHECK ((published_archive_media_type = 'application/zip'::text)),
    CONSTRAINT published_flat_import_origin_archive_license_check CHECK ((char_length(btrim(published_archive_license)) BETWEEN 1 AND 512)),
    CONSTRAINT published_flat_import_origin_archive_provenance_check CHECK ((char_length(btrim(published_archive_provenance)) BETWEEN 1 AND 2048))
);

ALTER TABLE ONLY public.published_flat_import_origin FORCE ROW LEVEL SECURITY;

CREATE TABLE public.published_flat_import_choice_map (
    owner_tenant_id uuid NOT NULL,
    problem_id uuid NOT NULL,
    version_id uuid NOT NULL,
    choice_map_sha256 character(64) NOT NULL,
    payload bytea NOT NULL,
    created_at timestamp with time zone DEFAULT transaction_timestamp() NOT NULL,
    CONSTRAINT published_flat_import_choice_map_sha256_check CHECK ((choice_map_sha256 ~ '^[0-9a-f]{64}$'::text)),
    CONSTRAINT published_flat_import_choice_map_payload_check CHECK ((octet_length(payload) BETWEEN 1 AND 2097152))
);

ALTER TABLE ONLY public.published_flat_import_choice_map FORCE ROW LEVEL SECURITY;

CREATE TABLE public.workspace_draft (
    tenant_id uuid NOT NULL,
    workspace_id uuid NOT NULL,
    payload jsonb NOT NULL,
    payload_sha256 character(64) NOT NULL,
    updated_at timestamp with time zone DEFAULT transaction_timestamp() NOT NULL,
    revision bigint DEFAULT 1 NOT NULL,
    CONSTRAINT workspace_draft_revision_check CHECK ((revision > 0))
);

ALTER TABLE ONLY public.workspace_draft FORCE ROW LEVEL SECURITY;

CREATE TABLE public.workspace_flat_question_source (
    tenant_id uuid NOT NULL,
    workspace_id uuid NOT NULL,
    draft_revision bigint NOT NULL,
    draft_payload_sha256 character(64) NOT NULL,
    source_object_id uuid NOT NULL,
    source_payload jsonb NOT NULL,
    source_payload_sha256 character(64) NOT NULL,
    canonical_source_sha256 character(64) NOT NULL,
    public_binding_sha256 character(64) NOT NULL,
    created_at timestamp with time zone DEFAULT transaction_timestamp() NOT NULL,
    CONSTRAINT workspace_flat_question_source_payload_revision_check CHECK ((draft_revision > 0)),
    CONSTRAINT workspace_flat_question_source_payload_check CHECK ((jsonb_typeof(source_payload) = 'object'::text)),
    CONSTRAINT workspace_flat_question_source_payload_size_check CHECK ((octet_length((source_payload)::text) <= 65536)),
    CONSTRAINT workspace_flat_question_source_draft_payload_sha256_check CHECK ((draft_payload_sha256 ~ '^[0-9a-f]{64}$'::text)),
    CONSTRAINT workspace_flat_question_source_source_payload_sha256_check CHECK ((source_payload_sha256 ~ '^[0-9a-f]{64}$'::text)),
    CONSTRAINT workspace_flat_question_source_canonical_source_sha256_check CHECK ((canonical_source_sha256 ~ '^[0-9a-f]{64}$'::text)),
    CONSTRAINT workspace_flat_question_source_public_binding_sha256_check CHECK ((public_binding_sha256 ~ '^[0-9a-f]{64}$'::text))
);

ALTER TABLE ONLY public.workspace_flat_question_source FORCE ROW LEVEL SECURITY;

-- Private compiler material is current workspace state, not publication
-- input. It remains bound to the exact draft and staged source that produced
-- it until either source is replaced or the workspace is published.
CREATE TABLE public.workspace_flat_question_grading (
    tenant_id uuid NOT NULL,
    workspace_id uuid NOT NULL,
    draft_revision bigint NOT NULL,
    draft_payload_sha256 character(64) NOT NULL,
    source_object_id uuid NOT NULL,
    source_payload_sha256 character(64) NOT NULL,
    canonical_source_sha256 character(64) NOT NULL,
    public_binding_sha256 character(64) NOT NULL,
    key_payload jsonb NOT NULL,
    key_sha256 character(64) NOT NULL,
    created_at timestamp with time zone DEFAULT transaction_timestamp() NOT NULL,
    CONSTRAINT workspace_flat_question_grading_draft_revision_check
        CHECK ((draft_revision > 0)),
    CONSTRAINT workspace_flat_question_grading_draft_payload_sha256_check
        CHECK ((draft_payload_sha256 ~ '^[0-9a-f]{64}$'::text)),
    CONSTRAINT workspace_flat_question_grading_source_payload_sha256_check
        CHECK ((source_payload_sha256 ~ '^[0-9a-f]{64}$'::text)),
    CONSTRAINT workspace_flat_question_grading_canonical_source_sha256_check
        CHECK ((canonical_source_sha256 ~ '^[0-9a-f]{64}$'::text)),
    CONSTRAINT workspace_flat_question_grading_public_binding_sha256_check
        CHECK ((public_binding_sha256 ~ '^[0-9a-f]{64}$'::text)),
    CONSTRAINT workspace_flat_question_grading_key_sha256_check
        CHECK ((key_sha256 ~ '^[0-9a-f]{64}$'::text)),
    CONSTRAINT workspace_flat_question_grading_key_payload_check
        CHECK ((jsonb_typeof(key_payload) = 'object'::text)),
    CONSTRAINT workspace_flat_question_grading_key_payload_size_check
        CHECK ((octet_length((key_payload)::text) <= 350000))
);

ALTER TABLE ONLY public.workspace_flat_question_grading FORCE ROW LEVEL SECURITY;

CREATE TABLE public.workspace_draft_access (
    tenant_id uuid NOT NULL,
    workspace_id uuid NOT NULL,
    user_id uuid NOT NULL,
    role text NOT NULL,
    created_at timestamp with time zone DEFAULT transaction_timestamp() NOT NULL,
    CONSTRAINT workspace_draft_access_role_check CHECK ((role = ANY (ARRAY['owner'::text, 'collaborator'::text])))
);

ALTER TABLE ONLY public.workspace_draft_access FORCE ROW LEVEL SECURITY;

CREATE TABLE public.workspace_qti_import (
    tenant_id uuid NOT NULL,
    workspace_id uuid NOT NULL,
    import_id uuid NOT NULL,
    source_object_id uuid NOT NULL,
    payload jsonb NOT NULL,
    payload_sha256 character(64) NOT NULL,
    created_at timestamp with time zone DEFAULT transaction_timestamp() NOT NULL,
    state text DEFAULT 'committed'::text NOT NULL,
    CONSTRAINT workspace_qti_import_payload_check CHECK (((jsonb_typeof(payload) = 'object'::text) AND (octet_length((payload)::text) <= 16777216))),
    CONSTRAINT workspace_qti_import_state_check CHECK ((state = ANY (ARRAY['prepared'::text, 'committed'::text])))
);

ALTER TABLE ONLY public.workspace_qti_import FORCE ROW LEVEL SECURITY;

CREATE TABLE public.workspace_qti_import_asset (
    tenant_id uuid NOT NULL,
    workspace_id uuid NOT NULL,
    import_id uuid NOT NULL,
    asset_id uuid NOT NULL,
    object_id uuid NOT NULL,
    payload jsonb NOT NULL,
    payload_sha256 character(64) NOT NULL,
    CONSTRAINT workspace_qti_import_asset_payload_check CHECK (((jsonb_typeof(payload) = 'object'::text) AND (octet_length((payload)::text) <= 65536)))
);

ALTER TABLE ONLY public.workspace_qti_import_asset FORCE ROW LEVEL SECURITY;

CREATE TABLE public.workspace_qti_import_grading (
    tenant_id uuid NOT NULL,
    workspace_id uuid NOT NULL,
    import_id uuid NOT NULL,
    item_id text NOT NULL,
    payload bytea NOT NULL,
    payload_sha256 character(64) NOT NULL,
    CONSTRAINT workspace_qti_import_grading_item_id_check CHECK (((char_length(item_id) >= 1) AND (char_length(item_id) <= 1024))),
    CONSTRAINT workspace_qti_import_grading_payload_check CHECK (((octet_length(payload) >= 1) AND (octet_length(payload) <= 262144)))
);

ALTER TABLE ONLY public.workspace_qti_import_grading FORCE ROW LEVEL SECURITY;

CREATE TABLE public.workspace_qti_import_item (
    tenant_id uuid NOT NULL,
    workspace_id uuid NOT NULL,
    import_id uuid NOT NULL,
    item_id text NOT NULL,
    payload jsonb NOT NULL,
    payload_sha256 character(64) NOT NULL,
    CONSTRAINT workspace_qti_import_item_item_id_check CHECK (((char_length(item_id) >= 1) AND (char_length(item_id) <= 1024))),
    CONSTRAINT workspace_qti_import_item_payload_check CHECK (((jsonb_typeof(payload) = 'object'::text) AND (octet_length((payload)::text) <= 65536)))
);

ALTER TABLE ONLY public.workspace_qti_import_item FORCE ROW LEVEL SECURITY;

CREATE TABLE public.workspace_qti_import_result (
    tenant_id uuid NOT NULL,
    workspace_id uuid NOT NULL,
    import_id uuid NOT NULL,
    ordinal integer NOT NULL,
    source_identifier text NOT NULL,
    status text NOT NULL,
    normalized_sha256 character(64),
    payload jsonb NOT NULL,
    payload_sha256 character(64) NOT NULL,
    CONSTRAINT workspace_qti_import_result_ordinal_check CHECK ((ordinal >= 0)),
    CONSTRAINT workspace_qti_import_result_source_identifier_check CHECK (((char_length(source_identifier) >= 1) AND (char_length(source_identifier) <= 1024))),
    CONSTRAINT workspace_qti_import_result_status_check CHECK ((status = ANY (ARRAY['accepted'::text, 'rejected'::text]))),
    CONSTRAINT workspace_qti_import_result_shape_check CHECK ((((status = 'accepted'::text) AND (normalized_sha256 IS NOT NULL)) OR ((status = 'rejected'::text) AND (normalized_sha256 IS NULL)))),
    CONSTRAINT workspace_qti_import_result_payload_check CHECK (((jsonb_typeof(payload) = 'object'::text) AND (octet_length((payload)::text) <= 65536)))
);

ALTER TABLE ONLY public.workspace_qti_import_result FORCE ROW LEVEL SECURITY;

-- Closed, private profile evidence is prepared with the import and becomes
-- immutable when its registry commits. It gives conversion a database-side
-- revalidation boundary without exposing the vendor choice map itself.
CREATE TABLE public.workspace_qti_profile_import_evidence (
    tenant_id uuid NOT NULL,
    workspace_id uuid NOT NULL,
    import_id uuid NOT NULL,
    profile_id text NOT NULL,
    profile_version text NOT NULL,
    mapping_version text NOT NULL,
    profile_report_sha256 character(64) NOT NULL,
    created_at timestamp with time zone DEFAULT transaction_timestamp() NOT NULL,
    CONSTRAINT workspace_qti_profile_import_evidence_profile_check CHECK (((profile_id = 'canvas-qti-1.2-static-single-choice/v1'::text) AND (profile_version = 'v1'::text) AND (mapping_version = 'v1'::text)) OR ((profile_id = 'blackboard-qti-2.1-static-single-choice-pool/v1'::text) AND (profile_version = 'v1'::text) AND (mapping_version = 'v1'::text))),
    CONSTRAINT workspace_qti_profile_import_evidence_report_sha256_check CHECK ((profile_report_sha256 ~ '^[0-9a-f]{64}$'::text))
);

ALTER TABLE ONLY public.workspace_qti_profile_import_evidence FORCE ROW LEVEL SECURITY;

CREATE TABLE public.workspace_qti_profile_item_evidence (
    tenant_id uuid NOT NULL,
    workspace_id uuid NOT NULL,
    import_id uuid NOT NULL,
    item_id text NOT NULL,
    source_item_identifier text NOT NULL,
    normalized_item_sha256 character(64) NOT NULL,
    public_mapping_sha256 character(64) NOT NULL,
    private_mapping_sha256 character(64) NOT NULL,
    mapping_sha256 character(64) NOT NULL,
    warning_sha256 character(64) NOT NULL,
    choice_map_sha256 character(64) NOT NULL,
    created_at timestamp with time zone DEFAULT transaction_timestamp() NOT NULL,
    CONSTRAINT workspace_qti_profile_item_evidence_item_id_check CHECK ((char_length(item_id) BETWEEN 1 AND 1024)),
    CONSTRAINT workspace_qti_profile_item_evidence_source_item_identifier_check CHECK ((char_length(source_item_identifier) BETWEEN 1 AND 1024) AND (btrim(source_item_identifier) <> ''::text)),
    CONSTRAINT workspace_qti_profile_item_evidence_item_binding_check CHECK ((item_id = source_item_identifier)),
    CONSTRAINT workspace_qti_profile_item_evidence_normalized_item_sha256_check CHECK ((normalized_item_sha256 ~ '^[0-9a-f]{64}$'::text)),
    CONSTRAINT workspace_qti_profile_item_evidence_public_mapping_sha256_check CHECK ((public_mapping_sha256 ~ '^[0-9a-f]{64}$'::text)),
    CONSTRAINT workspace_qti_profile_item_evidence_private_mapping_sha256_check CHECK ((private_mapping_sha256 ~ '^[0-9a-f]{64}$'::text)),
    CONSTRAINT workspace_qti_profile_item_evidence_mapping_sha256_check CHECK ((mapping_sha256 ~ '^[0-9a-f]{64}$'::text)),
    CONSTRAINT workspace_qti_profile_item_evidence_warning_sha256_check CHECK ((warning_sha256 ~ '^[0-9a-f]{64}$'::text)),
    CONSTRAINT workspace_qti_profile_item_evidence_choice_map_sha256_check CHECK ((choice_map_sha256 ~ '^[0-9a-f]{64}$'::text))
);

ALTER TABLE ONLY public.workspace_qti_profile_item_evidence FORCE ROW LEVEL SECURITY;

CREATE TABLE public.workspace_qti_import_unsupported (
    tenant_id uuid NOT NULL,
    workspace_id uuid NOT NULL,
    import_id uuid NOT NULL,
    ordinal integer NOT NULL,
    payload jsonb NOT NULL,
    payload_sha256 character(64) NOT NULL,
    CONSTRAINT workspace_qti_import_unsupported_ordinal_check CHECK ((ordinal >= 0)),
    CONSTRAINT workspace_qti_import_unsupported_payload_check CHECK (((jsonb_typeof(payload) = 'object'::text) AND (octet_length((payload)::text) <= 8192)))
);

ALTER TABLE ONLY public.workspace_qti_import_unsupported FORCE ROW LEVEL SECURITY;

CREATE TABLE public.workspace_flat_import_origin (
    tenant_id uuid NOT NULL,
    workspace_id uuid NOT NULL,
    import_id uuid NOT NULL,
    source_archive_object_id uuid NOT NULL,
    source_archive_sha256 character(64) NOT NULL,
    source_archive_size_bytes bigint NOT NULL,
    source_archive_media_type text NOT NULL,
    source_archive_license text NOT NULL,
    source_archive_provenance text NOT NULL,
    source_archive_created_at timestamp with time zone NOT NULL,
    source_item_identifier text NOT NULL,
    profile_id text NOT NULL,
    profile_version text NOT NULL,
    mapping_version text NOT NULL,
    conversion_version text NOT NULL,
    normalized_item_sha256 character(64) NOT NULL,
    profile_report_sha256 character(64) NOT NULL,
    public_mapping_sha256 character(64) NOT NULL,
    private_mapping_sha256 character(64) NOT NULL,
    mapping_sha256 character(64) NOT NULL,
    warning_sha256 character(64) NOT NULL,
    choice_map_sha256 character(64) NOT NULL,
    mapped_canonical_source_sha256 character(64) NOT NULL,
    acknowledged_by uuid NOT NULL,
    acknowledged_at timestamp with time zone NOT NULL,
    created_at timestamp with time zone DEFAULT transaction_timestamp() NOT NULL,
    CONSTRAINT workspace_flat_import_origin_source_item_identifier_check CHECK ((char_length(source_item_identifier) BETWEEN 1 AND 1024) AND (btrim(source_item_identifier) <> ''::text)),
    CONSTRAINT workspace_flat_import_origin_profile_check CHECK (((profile_id = 'canvas-qti-1.2-static-single-choice/v1'::text) AND (profile_version = 'v1'::text) AND (mapping_version = 'v1'::text)) OR ((profile_id = 'blackboard-qti-2.1-static-single-choice-pool/v1'::text) AND (profile_version = 'v1'::text) AND (mapping_version = 'v1'::text))),
    CONSTRAINT workspace_flat_import_origin_conversion_version_check CHECK ((octet_length(conversion_version) BETWEEN 1 AND 128) AND (conversion_version ~ '^[a-z0-9_/-]+$'::text)),
    CONSTRAINT workspace_flat_import_origin_source_archive_sha256_check CHECK ((source_archive_sha256 ~ '^[0-9a-f]{64}$'::text)),
    CONSTRAINT workspace_flat_import_origin_source_archive_size_check CHECK ((source_archive_size_bytes BETWEEN 1 AND 33554432)),
    CONSTRAINT workspace_flat_import_origin_source_archive_media_type_check CHECK ((source_archive_media_type = 'application/zip'::text)),
    CONSTRAINT workspace_flat_import_origin_source_archive_license_check CHECK ((char_length(btrim(source_archive_license)) BETWEEN 1 AND 512)),
    CONSTRAINT workspace_flat_import_origin_source_archive_provenance_check CHECK ((char_length(btrim(source_archive_provenance)) BETWEEN 1 AND 2048)),
    CONSTRAINT workspace_flat_import_origin_normalized_item_sha256_check CHECK ((normalized_item_sha256 ~ '^[0-9a-f]{64}$'::text)),
    CONSTRAINT workspace_flat_import_origin_profile_report_sha256_check CHECK ((profile_report_sha256 ~ '^[0-9a-f]{64}$'::text)),
    CONSTRAINT workspace_flat_import_origin_public_mapping_sha256_check CHECK ((public_mapping_sha256 ~ '^[0-9a-f]{64}$'::text)),
    CONSTRAINT workspace_flat_import_origin_private_mapping_sha256_check CHECK ((private_mapping_sha256 ~ '^[0-9a-f]{64}$'::text)),
    CONSTRAINT workspace_flat_import_origin_mapping_sha256_check CHECK ((mapping_sha256 ~ '^[0-9a-f]{64}$'::text)),
    CONSTRAINT workspace_flat_import_origin_warning_sha256_check CHECK ((warning_sha256 ~ '^[0-9a-f]{64}$'::text)),
    CONSTRAINT workspace_flat_import_origin_choice_map_sha256_check CHECK ((choice_map_sha256 ~ '^[0-9a-f]{64}$'::text)),
    CONSTRAINT workspace_flat_import_origin_mapped_source_sha256_check CHECK ((mapped_canonical_source_sha256 ~ '^[0-9a-f]{64}$'::text))
);

ALTER TABLE ONLY public.workspace_flat_import_origin FORCE ROW LEVEL SECURITY;

CREATE TABLE public.workspace_flat_import_choice_map (
    tenant_id uuid NOT NULL,
    workspace_id uuid NOT NULL,
    choice_map_sha256 character(64) NOT NULL,
    payload bytea NOT NULL,
    created_at timestamp with time zone DEFAULT transaction_timestamp() NOT NULL,
    CONSTRAINT workspace_flat_import_choice_map_sha256_check CHECK ((choice_map_sha256 ~ '^[0-9a-f]{64}$'::text)),
    CONSTRAINT workspace_flat_import_choice_map_payload_check CHECK ((octet_length(payload) BETWEEN 1 AND 2097152))
);

ALTER TABLE ONLY public.workspace_flat_import_choice_map FORCE ROW LEVEL SECURITY;

ALTER TABLE ONLY public.answer_key
    ADD CONSTRAINT answer_key_pkey PRIMARY KEY (problem_id, version_id);

ALTER TABLE ONLY public.catalog_tenant_grant
    ADD CONSTRAINT catalog_tenant_grant_pkey PRIMARY KEY (tenant_id, problem_id, version_id);

ALTER TABLE ONLY public.problem
    ADD CONSTRAINT problem_pkey PRIMARY KEY (problem_id);

ALTER TABLE ONLY public.problem
    ADD CONSTRAINT problem_public_id_key UNIQUE (public_id);

ALTER TABLE public.problem_version
    ADD CONSTRAINT problem_version_authors_nonempty_check CHECK ((jsonb_array_length(authors) > 0));

ALTER TABLE ONLY public.problem_version_payload
    ADD CONSTRAINT problem_version_payload_pkey PRIMARY KEY (problem_id, version_id);

ALTER TABLE ONLY public.problem_version
    ADD CONSTRAINT problem_version_pkey PRIMARY KEY (problem_id, version_id);

ALTER TABLE ONLY public.problem_version
    ADD CONSTRAINT problem_version_number_key UNIQUE (problem_id, version_number);

ALTER TABLE ONLY public.problem_collection
    ADD CONSTRAINT problem_collection_pkey PRIMARY KEY (owner_tenant_id, collection_id);

ALTER TABLE ONLY public.problem_collection_member
    ADD CONSTRAINT problem_collection_member_pkey PRIMARY KEY (owner_tenant_id, collection_id, position);

ALTER TABLE ONLY public.problem_collection_member
    ADD CONSTRAINT problem_collection_member_version_key UNIQUE (owner_tenant_id, collection_id, problem_id, version_id);

ALTER TABLE ONLY public.catalog_search_document
    ADD CONSTRAINT catalog_search_document_pkey PRIMARY KEY (problem_id, version_id);

ALTER TABLE ONLY public.published_qti_grading
    ADD CONSTRAINT published_qti_grading_pkey PRIMARY KEY (problem_id, version_id, item_id);

ALTER TABLE ONLY public.published_flat_import_origin
    ADD CONSTRAINT published_flat_import_origin_pkey PRIMARY KEY (owner_tenant_id, problem_id, version_id);

ALTER TABLE ONLY public.published_flat_import_origin
    ADD CONSTRAINT published_flat_import_origin_choice_map_key
    UNIQUE (owner_tenant_id, problem_id, version_id, choice_map_sha256);

ALTER TABLE ONLY public.published_flat_import_origin
    ADD CONSTRAINT published_flat_import_origin_archive_object_id_key UNIQUE (published_archive_object_id);

ALTER TABLE ONLY public.published_flat_import_choice_map
    ADD CONSTRAINT published_flat_import_choice_map_pkey PRIMARY KEY (owner_tenant_id, problem_id, version_id);

ALTER TABLE ONLY public.published_source_artifact
    ADD CONSTRAINT published_source_artifact_object_id_key UNIQUE (object_id);

ALTER TABLE ONLY public.published_source_artifact
    ADD CONSTRAINT published_source_artifact_pkey PRIMARY KEY (problem_id, version_id);

ALTER TABLE ONLY public.workspace_draft_access
    ADD CONSTRAINT workspace_draft_access_pkey PRIMARY KEY (tenant_id, workspace_id, user_id);

ALTER TABLE ONLY public.workspace_draft
    ADD CONSTRAINT workspace_draft_pkey PRIMARY KEY (tenant_id, workspace_id);

ALTER TABLE ONLY public.workspace_flat_question_source
    ADD CONSTRAINT workspace_flat_question_source_pkey PRIMARY KEY (tenant_id, workspace_id);

ALTER TABLE ONLY public.workspace_flat_question_source
    ADD CONSTRAINT workspace_flat_question_source_source_object_id_key UNIQUE (source_object_id);

ALTER TABLE ONLY public.workspace_flat_question_grading
    ADD CONSTRAINT workspace_flat_question_grading_pkey PRIMARY KEY (tenant_id, workspace_id);

ALTER TABLE ONLY public.workspace_flat_import_origin
    ADD CONSTRAINT workspace_flat_import_origin_pkey PRIMARY KEY (tenant_id, workspace_id);

ALTER TABLE ONLY public.workspace_flat_import_origin
    ADD CONSTRAINT workspace_flat_import_origin_choice_map_key
    UNIQUE (tenant_id, workspace_id, choice_map_sha256);

ALTER TABLE ONLY public.workspace_flat_import_choice_map
    ADD CONSTRAINT workspace_flat_import_choice_map_pkey PRIMARY KEY (tenant_id, workspace_id);

ALTER TABLE ONLY public.workspace_qti_import_asset
    ADD CONSTRAINT workspace_qti_import_asset_object_id_key UNIQUE (object_id);

ALTER TABLE ONLY public.workspace_qti_import_asset
    ADD CONSTRAINT workspace_qti_import_asset_pkey PRIMARY KEY (tenant_id, workspace_id, import_id, asset_id);

ALTER TABLE ONLY public.workspace_qti_import_grading
    ADD CONSTRAINT workspace_qti_import_grading_pkey PRIMARY KEY (tenant_id, workspace_id, import_id, item_id);

ALTER TABLE ONLY public.workspace_qti_import_item
    ADD CONSTRAINT workspace_qti_import_item_pkey PRIMARY KEY (tenant_id, workspace_id, import_id, item_id);

ALTER TABLE ONLY public.workspace_qti_import_result
    ADD CONSTRAINT workspace_qti_import_result_pkey PRIMARY KEY (tenant_id, workspace_id, import_id, ordinal);

ALTER TABLE ONLY public.workspace_qti_profile_import_evidence
    ADD CONSTRAINT workspace_qti_profile_import_evidence_pkey
    PRIMARY KEY (tenant_id, workspace_id, import_id);

ALTER TABLE ONLY public.workspace_qti_profile_item_evidence
    ADD CONSTRAINT workspace_qti_profile_item_evidence_pkey
    PRIMARY KEY (tenant_id, workspace_id, import_id, item_id);

ALTER TABLE ONLY public.workspace_qti_import
    ADD CONSTRAINT workspace_qti_import_pkey PRIMARY KEY (tenant_id, workspace_id, import_id);

ALTER TABLE ONLY public.workspace_qti_import
    ADD CONSTRAINT workspace_qti_import_source_object_id_key UNIQUE (source_object_id);

ALTER TABLE ONLY public.workspace_qti_import_unsupported
    ADD CONSTRAINT workspace_qti_import_unsupported_pkey PRIMARY KEY (tenant_id, workspace_id, import_id, ordinal);

CREATE INDEX problem_version_capabilities_idx ON public.problem_version USING gin (capabilities jsonb_path_ops);

CREATE INDEX catalog_search_document_search_idx ON public.catalog_search_document USING gin (search_text);

CREATE INDEX catalog_search_document_public_id_idx ON public.catalog_search_document USING btree (public_id, version_number);

CREATE INDEX problem_version_catalog_idx ON public.problem_version USING btree (lifecycle, title, problem_id, version_id);

CREATE INDEX problem_version_catalog_search_key_idx ON public.problem_version USING btree (problem_id, version_id) WHERE (lifecycle = 'published'::text);

CREATE INDEX problem_version_catalog_search_text_idx ON public.problem_version USING gin (to_tsvector('simple'::regconfig, ((title || ' '::text) || (metadata)::text)));

CREATE UNIQUE INDEX problem_version_linear_chain_idx ON public.problem_version USING btree (problem_id, previous_version_id) WHERE (previous_version_id IS NOT NULL);

CREATE INDEX problem_version_metadata_idx ON public.problem_version USING gin (metadata jsonb_path_ops);

CREATE INDEX workspace_draft_access_user_idx ON public.workspace_draft_access USING btree (tenant_id, user_id, workspace_id);

CREATE INDEX workspace_qti_import_committed_idx ON public.workspace_qti_import USING btree (tenant_id, workspace_id, import_id) WHERE (state = 'committed'::text);

-- Required support for the restrictive import-pin foreign key. Other origin
-- indexes wait for measured read workloads.
CREATE INDEX workspace_flat_import_origin_import_pin_idx ON public.workspace_flat_import_origin
    USING btree (tenant_id, workspace_id, import_id);

-- The published origin is tenant-leading for RLS; the global catalog FK needs
-- this inverse support index for restrictive version deletion checks.
CREATE INDEX published_flat_import_origin_problem_version_idx ON public.published_flat_import_origin
    USING btree (problem_id, version_id);

-- This FK targets globally-owned immutable catalog versions, so tenant_id cannot lead.
CREATE INDEX catalog_tenant_grant_problem_version_idx ON public.catalog_tenant_grant
    USING btree (problem_id, version_id);

ALTER TABLE ONLY public.answer_key
    ADD CONSTRAINT answer_key_problem_id_version_id_fkey FOREIGN KEY (problem_id, version_id) REFERENCES public.problem_version(problem_id, version_id);

ALTER TABLE ONLY public.catalog_tenant_grant
    ADD CONSTRAINT catalog_tenant_grant_problem_id_version_id_fkey FOREIGN KEY (problem_id, version_id) REFERENCES public.problem_version(problem_id, version_id) ON DELETE CASCADE DEFERRABLE INITIALLY DEFERRED;

ALTER TABLE ONLY public.problem_version
    ADD CONSTRAINT problem_version_derived_from_fk FOREIGN KEY (derived_from_problem_id, derived_from_version_id) REFERENCES public.problem_version(problem_id, version_id) DEFERRABLE INITIALLY DEFERRED;

ALTER TABLE public.problem_version_payload
    ADD CONSTRAINT problem_version_payload_problem_id_version_id_fkey FOREIGN KEY (problem_id, version_id) REFERENCES public.problem_version(problem_id, version_id);

ALTER TABLE ONLY public.problem_version
    ADD CONSTRAINT problem_version_previous_fk FOREIGN KEY (problem_id, previous_version_id) REFERENCES public.problem_version(problem_id, version_id) DEFERRABLE INITIALLY DEFERRED;

ALTER TABLE ONLY public.problem_version
    ADD CONSTRAINT problem_version_problem_id_fkey FOREIGN KEY (problem_id) REFERENCES public.problem(problem_id);

ALTER TABLE ONLY public.problem_collection_member
    ADD CONSTRAINT problem_collection_member_collection_fkey FOREIGN KEY (owner_tenant_id, collection_id) REFERENCES public.problem_collection(owner_tenant_id, collection_id) ON DELETE CASCADE;

ALTER TABLE ONLY public.problem_collection_member
    ADD CONSTRAINT problem_collection_member_version_fkey FOREIGN KEY (problem_id, version_id) REFERENCES public.problem_version(problem_id, version_id) ON DELETE RESTRICT;

ALTER TABLE ONLY public.catalog_search_document
    ADD CONSTRAINT catalog_search_document_version_fkey FOREIGN KEY (problem_id, version_id) REFERENCES public.problem_version(problem_id, version_id) ON DELETE CASCADE;

ALTER TABLE ONLY public.published_qti_grading
    ADD CONSTRAINT published_qti_grading_problem_id_version_id_fkey FOREIGN KEY (problem_id, version_id) REFERENCES public.problem_version(problem_id, version_id) ON DELETE RESTRICT DEFERRABLE INITIALLY DEFERRED;

ALTER TABLE ONLY public.published_flat_import_origin
    ADD CONSTRAINT published_flat_import_origin_problem_version_fkey
    FOREIGN KEY (problem_id, version_id) REFERENCES public.problem_version(problem_id, version_id)
    ON DELETE RESTRICT DEFERRABLE INITIALLY DEFERRED;

ALTER TABLE ONLY public.published_flat_import_choice_map
    ADD CONSTRAINT published_flat_import_choice_map_origin_fkey
    FOREIGN KEY (owner_tenant_id, problem_id, version_id, choice_map_sha256)
    REFERENCES public.published_flat_import_origin(owner_tenant_id, problem_id, version_id, choice_map_sha256)
    ON DELETE CASCADE DEFERRABLE INITIALLY DEFERRED;

ALTER TABLE ONLY public.published_source_artifact
    ADD CONSTRAINT published_source_artifact_problem_id_version_id_fkey FOREIGN KEY (problem_id, version_id) REFERENCES public.problem_version(problem_id, version_id) ON DELETE RESTRICT DEFERRABLE INITIALLY DEFERRED;

ALTER TABLE ONLY public.workspace_draft_access
    ADD CONSTRAINT workspace_draft_access_tenant_id_workspace_id_fkey FOREIGN KEY (tenant_id, workspace_id) REFERENCES public.workspace_draft(tenant_id, workspace_id) ON DELETE CASCADE;

ALTER TABLE ONLY public.workspace_flat_question_source
    ADD CONSTRAINT workspace_flat_question_source_workspace_draft_fkey FOREIGN KEY (tenant_id, workspace_id) REFERENCES public.workspace_draft(tenant_id, workspace_id) ON DELETE CASCADE;

ALTER TABLE ONLY public.workspace_flat_question_grading
    ADD CONSTRAINT workspace_flat_question_grading_source_fkey
    FOREIGN KEY (tenant_id, workspace_id)
    REFERENCES public.workspace_flat_question_source(tenant_id, workspace_id)
    ON DELETE CASCADE;

ALTER TABLE ONLY public.workspace_flat_import_origin
    ADD CONSTRAINT workspace_flat_import_origin_workspace_draft_fkey
    FOREIGN KEY (tenant_id, workspace_id) REFERENCES public.workspace_draft(tenant_id, workspace_id)
    ON DELETE CASCADE;

ALTER TABLE ONLY public.workspace_flat_import_origin
    ADD CONSTRAINT workspace_flat_import_origin_import_pin_fkey
    FOREIGN KEY (tenant_id, workspace_id, import_id)
    REFERENCES public.workspace_qti_import(tenant_id, workspace_id, import_id)
    ON DELETE RESTRICT DEFERRABLE INITIALLY DEFERRED;

ALTER TABLE ONLY public.workspace_flat_import_choice_map
    ADD CONSTRAINT workspace_flat_import_choice_map_origin_fkey
    FOREIGN KEY (tenant_id, workspace_id, choice_map_sha256)
    REFERENCES public.workspace_flat_import_origin(tenant_id, workspace_id, choice_map_sha256)
    ON DELETE CASCADE DEFERRABLE INITIALLY DEFERRED;

ALTER TABLE ONLY public.workspace_qti_import_asset
    ADD CONSTRAINT workspace_qti_import_asset_tenant_id_workspace_id_import_i_fkey FOREIGN KEY (tenant_id, workspace_id, import_id) REFERENCES public.workspace_qti_import(tenant_id, workspace_id, import_id) ON DELETE RESTRICT DEFERRABLE INITIALLY DEFERRED;

ALTER TABLE ONLY public.workspace_qti_import_grading
    ADD CONSTRAINT workspace_qti_import_grading_tenant_id_workspace_id_import_fkey FOREIGN KEY (tenant_id, workspace_id, import_id, item_id) REFERENCES public.workspace_qti_import_item(tenant_id, workspace_id, import_id, item_id) ON DELETE RESTRICT DEFERRABLE INITIALLY DEFERRED;

ALTER TABLE ONLY public.workspace_qti_import_item
    ADD CONSTRAINT workspace_qti_import_item_tenant_id_workspace_id_import_id_fkey FOREIGN KEY (tenant_id, workspace_id, import_id) REFERENCES public.workspace_qti_import(tenant_id, workspace_id, import_id) ON DELETE RESTRICT DEFERRABLE INITIALLY DEFERRED;

ALTER TABLE ONLY public.workspace_qti_import_result
    ADD CONSTRAINT workspace_qti_import_result_tenant_id_workspace_id_import_id_fkey FOREIGN KEY (tenant_id, workspace_id, import_id) REFERENCES public.workspace_qti_import(tenant_id, workspace_id, import_id) ON DELETE RESTRICT DEFERRABLE INITIALLY DEFERRED;

ALTER TABLE ONLY public.workspace_qti_profile_item_evidence
    ADD CONSTRAINT workspace_qti_profile_item_evidence_item_fkey
    FOREIGN KEY (tenant_id, workspace_id, import_id, item_id)
    REFERENCES public.workspace_qti_import_item(tenant_id, workspace_id, import_id, item_id)
    ON DELETE RESTRICT DEFERRABLE INITIALLY DEFERRED;

ALTER TABLE ONLY public.workspace_qti_profile_import_evidence
    ADD CONSTRAINT workspace_qti_profile_import_evidence_import_fkey
    FOREIGN KEY (tenant_id, workspace_id, import_id)
    REFERENCES public.workspace_qti_import(tenant_id, workspace_id, import_id)
    ON DELETE RESTRICT DEFERRABLE INITIALLY DEFERRED;

ALTER TABLE ONLY public.workspace_qti_profile_item_evidence
    ADD CONSTRAINT workspace_qti_profile_item_evidence_import_evidence_fkey
    FOREIGN KEY (tenant_id, workspace_id, import_id)
    REFERENCES public.workspace_qti_profile_import_evidence(tenant_id, workspace_id, import_id)
    ON DELETE RESTRICT DEFERRABLE INITIALLY DEFERRED;

ALTER TABLE ONLY public.workspace_qti_import_unsupported
    ADD CONSTRAINT workspace_qti_import_unsuppor_tenant_id_workspace_id_impor_fkey FOREIGN KEY (tenant_id, workspace_id, import_id) REFERENCES public.workspace_qti_import(tenant_id, workspace_id, import_id) ON DELETE RESTRICT DEFERRABLE INITIALLY DEFERRED;

ALTER TABLE public.catalog_tenant_grant ENABLE ROW LEVEL SECURITY;

CREATE POLICY catalog_tenant_grant_statistics_visible_select ON public.catalog_tenant_grant FOR SELECT TO ple_statistics_broker USING ((tenant_id = public.ple_current_tenant()));

CREATE POLICY catalog_tenant_grant_tenant ON public.catalog_tenant_grant USING ((tenant_id = public.ple_current_tenant())) WITH CHECK ((tenant_id = public.ple_current_tenant()));

ALTER TABLE public.problem ENABLE ROW LEVEL SECURITY;

-- A problem is visible to its owner, or when this tenant can see one of its
-- immutable versions. `visibility` alone is intentionally not a tenant grant.
CREATE POLICY problem_owner_write ON public.problem TO ple_app
    USING ((owner_tenant_id = public.ple_current_tenant()))
    WITH CHECK ((owner_tenant_id = public.ple_current_tenant()));

CREATE POLICY problem_visible_select ON public.problem FOR SELECT TO ple_app, ple_student
    USING ((owner_tenant_id = public.ple_current_tenant()) OR EXISTS (
        SELECT 1
          FROM public.problem_version AS visible_version
         WHERE visible_version.problem_id = problem.problem_id
           AND (visible_version.publication_scope = 'public' OR EXISTS (
                SELECT 1
                  FROM public.catalog_tenant_grant AS grant_row
                 WHERE grant_row.tenant_id = public.ple_current_tenant()
                   AND grant_row.problem_id = visible_version.problem_id
                   AND grant_row.version_id = visible_version.version_id
           ))
    ));

-- The grading broker may validate only catalog rows owned by its current
-- tenant before publication. Published/granted visibility remains on the
-- version policy below for its immutable grading-read capability.
CREATE POLICY problem_grader_owner_select ON public.problem FOR SELECT TO ple_grader
    USING ((owner_tenant_id = public.ple_current_tenant()));

CREATE POLICY problem_qti_provenance_broker_select ON public.problem FOR SELECT
    TO ple_qti_provenance_broker
    USING ((owner_tenant_id = public.ple_current_tenant()));

ALTER TABLE public.answer_key ENABLE ROW LEVEL SECURITY;

-- Answer keys have no tenant column. Graders may read or insert a key only
-- for a version that the current tenant is authorized to use for grading.
CREATE POLICY answer_key_grader ON public.answer_key TO ple_grader
    USING (EXISTS (
        SELECT 1
          FROM public.problem_version AS visible_version
         WHERE visible_version.problem_id = answer_key.problem_id
           AND visible_version.version_id = answer_key.version_id
           AND (visible_version.publication_scope = 'public' OR EXISTS (
                SELECT 1
                  FROM public.catalog_tenant_grant AS grant_row
                 WHERE grant_row.tenant_id = public.ple_current_tenant()
                   AND grant_row.problem_id = visible_version.problem_id
                   AND grant_row.version_id = visible_version.version_id
           ))
    ))
    WITH CHECK (EXISTS (
        SELECT 1
          FROM public.problem_version AS visible_version
         WHERE visible_version.problem_id = answer_key.problem_id
           AND visible_version.version_id = answer_key.version_id
           AND (visible_version.publication_scope = 'public' OR EXISTS (
                SELECT 1
                  FROM public.catalog_tenant_grant AS grant_row
                 WHERE grant_row.tenant_id = public.ple_current_tenant()
                   AND grant_row.problem_id = visible_version.problem_id
                   AND grant_row.version_id = visible_version.version_id
           ))
    ));

ALTER TABLE public.problem_version ENABLE ROW LEVEL SECURITY;

CREATE POLICY problem_version_app_insert ON public.problem_version FOR INSERT TO ple_app
    WITH CHECK (public.ple_problem_owned_by_current_tenant(problem_id));

CREATE POLICY problem_version_app_update ON public.problem_version FOR UPDATE TO ple_app
    USING (public.ple_problem_owned_by_current_tenant(problem_id))
    WITH CHECK (public.ple_problem_owned_by_current_tenant(problem_id));

ALTER TABLE public.problem_version_payload ENABLE ROW LEVEL SECURITY;

CREATE POLICY problem_version_payload_app_insert ON public.problem_version_payload FOR INSERT TO ple_app WITH CHECK ((EXISTS ( SELECT 1
   FROM public.problem_version visible_version
  WHERE ((visible_version.problem_id = problem_version_payload.problem_id) AND (visible_version.version_id = problem_version_payload.version_id)))));

CREATE POLICY problem_version_payload_visible_select ON public.problem_version_payload FOR SELECT TO ple_app, ple_student USING ((EXISTS ( SELECT 1
   FROM public.problem_version visible_version
  WHERE ((visible_version.problem_id = problem_version_payload.problem_id) AND (visible_version.version_id = problem_version_payload.version_id)))));

-- The security-definer flat grader capability needs to inspect only the
-- answer-free published model to prove family eligibility.  Visibility stays
-- scoped to the same published-version tenant policy as all grader reads.
CREATE POLICY problem_version_payload_grader_select ON public.problem_version_payload FOR SELECT TO ple_grader USING ((EXISTS ( SELECT 1
   FROM public.problem_version visible_version
  WHERE ((visible_version.problem_id = problem_version_payload.problem_id) AND (visible_version.version_id = problem_version_payload.version_id)))));

CREATE POLICY problem_version_statistics_visible_select ON public.problem_version FOR SELECT TO ple_statistics_broker USING (((publication_scope = 'public'::text) OR (EXISTS ( SELECT 1
   FROM public.catalog_tenant_grant grant_row
  WHERE ((grant_row.problem_id = problem_version.problem_id) AND (grant_row.version_id = problem_version.version_id) AND (grant_row.tenant_id = public.ple_current_tenant()))))));

CREATE POLICY problem_version_visible_select ON public.problem_version FOR SELECT TO ple_app, ple_student, ple_grader USING (((publication_scope = 'public'::text) OR (EXISTS ( SELECT 1
   FROM public.catalog_tenant_grant grant_row
  WHERE ((grant_row.problem_id = problem_version.problem_id) AND (grant_row.version_id = problem_version.version_id) AND (grant_row.tenant_id = public.ple_current_tenant()))))));

CREATE POLICY problem_version_grader_owner_select ON public.problem_version FOR SELECT
    TO ple_grader
    USING (EXISTS (
        SELECT 1
          FROM public.problem AS owner_problem
         WHERE owner_problem.problem_id = problem_version.problem_id
           AND owner_problem.owner_tenant_id = public.ple_current_tenant()
    ));

CREATE POLICY problem_version_qti_provenance_broker_select ON public.problem_version FOR SELECT
    TO ple_qti_provenance_broker
    USING (EXISTS (
        SELECT 1
          FROM public.problem owner_problem
         WHERE owner_problem.problem_id = problem_version.problem_id
           AND owner_problem.owner_tenant_id = public.ple_current_tenant()
    ));

ALTER TABLE public.problem_collection ENABLE ROW LEVEL SECURITY;

CREATE POLICY problem_collection_tenant_write ON public.problem_collection TO ple_app
    USING ((owner_tenant_id = public.ple_current_tenant()))
    WITH CHECK ((owner_tenant_id = public.ple_current_tenant()));

CREATE POLICY problem_collection_visible_select ON public.problem_collection FOR SELECT TO ple_app
    USING ((visibility = 'public') OR (owner_tenant_id = public.ple_current_tenant()));

ALTER TABLE public.problem_collection_member ENABLE ROW LEVEL SECURITY;

CREATE POLICY problem_collection_member_tenant_write ON public.problem_collection_member TO ple_app
    USING ((owner_tenant_id = public.ple_current_tenant()))
    WITH CHECK ((owner_tenant_id = public.ple_current_tenant()));

CREATE POLICY problem_collection_member_visible_select ON public.problem_collection_member FOR SELECT TO ple_app
    USING (EXISTS (
        SELECT 1
          FROM public.problem_collection collection
         WHERE collection.owner_tenant_id = problem_collection_member.owner_tenant_id
           AND collection.collection_id = problem_collection_member.collection_id
    ));

ALTER TABLE public.catalog_search_document ENABLE ROW LEVEL SECURITY;

CREATE POLICY catalog_search_document_visible_select ON public.catalog_search_document FOR SELECT TO ple_app, ple_student
    USING (EXISTS (
        SELECT 1
          FROM public.problem_version visible_version
         WHERE visible_version.problem_id = catalog_search_document.problem_id
           AND visible_version.version_id = catalog_search_document.version_id
           AND visible_version.lifecycle = 'published'
    ));

ALTER TABLE public.published_qti_grading ENABLE ROW LEVEL SECURITY;

CREATE POLICY published_qti_grading_broker ON public.published_qti_grading TO ple_qti_staging_broker USING (true) WITH CHECK (true);

ALTER TABLE public.published_flat_import_origin ENABLE ROW LEVEL SECURITY;

CREATE POLICY published_flat_import_origin_provenance_broker ON public.published_flat_import_origin
    TO ple_qti_provenance_broker
    USING ((owner_tenant_id = public.ple_current_tenant()))
    WITH CHECK ((owner_tenant_id = public.ple_current_tenant()));

ALTER TABLE public.published_flat_import_choice_map ENABLE ROW LEVEL SECURITY;

CREATE POLICY published_flat_import_choice_map_provenance_broker ON public.published_flat_import_choice_map
    TO ple_qti_provenance_broker
    USING ((owner_tenant_id = public.ple_current_tenant()))
    WITH CHECK ((owner_tenant_id = public.ple_current_tenant()));

ALTER TABLE public.published_source_artifact ENABLE ROW LEVEL SECURITY;

CREATE POLICY published_source_artifact_app_insert ON public.published_source_artifact FOR INSERT TO ple_app WITH CHECK ((EXISTS ( SELECT 1
   FROM public.problem_version visible_version
  WHERE ((visible_version.problem_id = published_source_artifact.problem_id) AND (visible_version.version_id = published_source_artifact.version_id)))));

CREATE POLICY published_source_artifact_visible_select ON public.published_source_artifact FOR SELECT TO ple_app USING ((EXISTS ( SELECT 1
   FROM public.problem_version visible_version
  WHERE ((visible_version.problem_id = published_source_artifact.problem_id) AND (visible_version.version_id = published_source_artifact.version_id)))));

ALTER TABLE public.workspace_draft ENABLE ROW LEVEL SECURITY;

ALTER TABLE public.workspace_draft_access ENABLE ROW LEVEL SECURITY;

ALTER TABLE public.workspace_flat_question_source ENABLE ROW LEVEL SECURITY;

ALTER TABLE public.workspace_flat_question_grading ENABLE ROW LEVEL SECURITY;

CREATE POLICY workspace_draft_access_tenant ON public.workspace_draft_access USING ((tenant_id = public.ple_current_tenant())) WITH CHECK ((tenant_id = public.ple_current_tenant()));

CREATE POLICY workspace_draft_tenant ON public.workspace_draft USING ((tenant_id = public.ple_current_tenant())) WITH CHECK ((tenant_id = public.ple_current_tenant()));

CREATE POLICY workspace_flat_question_source_app_select ON public.workspace_flat_question_source FOR SELECT TO ple_app USING ((tenant_id = public.ple_current_tenant()));

CREATE POLICY workspace_flat_question_source_app_insert ON public.workspace_flat_question_source FOR INSERT TO ple_app WITH CHECK ((tenant_id = public.ple_current_tenant()));

CREATE POLICY workspace_flat_question_source_app_delete ON public.workspace_flat_question_source FOR DELETE TO ple_app USING ((tenant_id = public.ple_current_tenant()));

CREATE POLICY workspace_flat_question_source_grader_select ON public.workspace_flat_question_source FOR SELECT TO ple_grader USING ((tenant_id = public.ple_current_tenant()));

-- PostgreSQL treats SELECT FOR KEY SHARE as an UPDATE-class lock. Scope the
-- required privilege and policy to the source timestamp; the grading broker
-- cannot mutate source identity or content.
CREATE POLICY workspace_flat_question_source_grader_lock ON public.workspace_flat_question_source
    FOR UPDATE TO ple_grader
    USING ((tenant_id = public.ple_current_tenant()))
    WITH CHECK ((tenant_id = public.ple_current_tenant()));

CREATE POLICY workspace_flat_question_grading_grader ON public.workspace_flat_question_grading
    TO ple_grader
    USING ((tenant_id = public.ple_current_tenant()))
    WITH CHECK ((tenant_id = public.ple_current_tenant()));

CREATE POLICY workspace_flat_question_source_provenance_broker_select ON public.workspace_flat_question_source
    FOR SELECT TO ple_qti_provenance_broker
    USING ((tenant_id = public.ple_current_tenant()));

-- PostgreSQL treats SELECT FOR KEY SHARE as an UPDATE-class lock. The broker
-- has only a timestamp UPDATE grant, and this matching policy permits that
-- lock without exposing source-content mutation.
CREATE POLICY workspace_flat_question_source_provenance_broker_lock ON public.workspace_flat_question_source
    FOR UPDATE TO ple_qti_provenance_broker
    USING ((tenant_id = public.ple_current_tenant()))
    WITH CHECK ((tenant_id = public.ple_current_tenant()));

ALTER TABLE public.workspace_flat_import_origin ENABLE ROW LEVEL SECURITY;

CREATE POLICY workspace_flat_import_origin_provenance_broker ON public.workspace_flat_import_origin
    TO ple_qti_provenance_broker
    USING ((tenant_id = public.ple_current_tenant()))
    WITH CHECK ((tenant_id = public.ple_current_tenant()));

ALTER TABLE public.workspace_flat_import_choice_map ENABLE ROW LEVEL SECURITY;

CREATE POLICY workspace_flat_import_choice_map_provenance_broker ON public.workspace_flat_import_choice_map
    TO ple_qti_provenance_broker
    USING ((tenant_id = public.ple_current_tenant()))
    WITH CHECK ((tenant_id = public.ple_current_tenant()));

ALTER TABLE public.workspace_qti_import ENABLE ROW LEVEL SECURITY;

CREATE POLICY workspace_qti_import_app_insert ON public.workspace_qti_import FOR INSERT TO ple_app WITH CHECK (((tenant_id = public.ple_current_tenant()) AND (state = 'prepared'::text)));

CREATE POLICY workspace_qti_import_provenance_broker_select ON public.workspace_qti_import
    FOR SELECT TO ple_qti_provenance_broker
    USING ((tenant_id = public.ple_current_tenant()));

-- The frozen conversion order locks the committed registry before reading
-- immutable children. Scope the required UPDATE-class lock to the existing
-- non-semantic timestamp grant below.
CREATE POLICY workspace_qti_import_provenance_broker_lock ON public.workspace_qti_import
    FOR UPDATE TO ple_qti_provenance_broker
    USING ((tenant_id = public.ple_current_tenant()))
    WITH CHECK ((tenant_id = public.ple_current_tenant()));

ALTER TABLE public.workspace_qti_import_asset ENABLE ROW LEVEL SECURITY;

CREATE POLICY workspace_qti_import_asset_app_prepared_insert ON public.workspace_qti_import_asset FOR INSERT TO ple_app WITH CHECK (((tenant_id = public.ple_current_tenant()) AND public.ple_qti_import_is_prepared(tenant_id, workspace_id, import_id)));

ALTER TABLE public.workspace_qti_import_grading ENABLE ROW LEVEL SECURITY;

CREATE POLICY workspace_qti_import_grading_app_prepared_insert ON public.workspace_qti_import_grading FOR INSERT TO ple_app WITH CHECK (((tenant_id = public.ple_current_tenant()) AND public.ple_qti_import_is_prepared(tenant_id, workspace_id, import_id)));

ALTER TABLE public.workspace_qti_import_item ENABLE ROW LEVEL SECURITY;

CREATE POLICY workspace_qti_import_item_app_prepared_insert ON public.workspace_qti_import_item FOR INSERT TO ple_app WITH CHECK (((tenant_id = public.ple_current_tenant()) AND public.ple_qti_import_is_prepared(tenant_id, workspace_id, import_id)));

CREATE POLICY workspace_qti_import_item_provenance_broker_select ON public.workspace_qti_import_item
    FOR SELECT TO ple_qti_provenance_broker
    USING ((tenant_id = public.ple_current_tenant()));

ALTER TABLE public.workspace_qti_import_result ENABLE ROW LEVEL SECURITY;

CREATE POLICY workspace_qti_import_result_app_prepared_insert ON public.workspace_qti_import_result FOR INSERT TO ple_app WITH CHECK (((tenant_id = public.ple_current_tenant()) AND public.ple_qti_import_is_prepared(tenant_id, workspace_id, import_id)));

CREATE POLICY workspace_qti_import_result_provenance_broker_select ON public.workspace_qti_import_result
    FOR SELECT TO ple_qti_provenance_broker
    USING ((tenant_id = public.ple_current_tenant()));

ALTER TABLE public.workspace_qti_profile_item_evidence ENABLE ROW LEVEL SECURITY;

ALTER TABLE public.workspace_qti_profile_import_evidence ENABLE ROW LEVEL SECURITY;

CREATE POLICY workspace_qti_profile_import_evidence_staging_broker ON public.workspace_qti_profile_import_evidence
    FOR INSERT TO ple_qti_staging_broker
    WITH CHECK ((tenant_id = public.ple_current_tenant())
        AND public.ple_qti_import_is_prepared(tenant_id, workspace_id, import_id));

CREATE POLICY workspace_qti_profile_item_evidence_provenance_broker_select ON public.workspace_qti_profile_item_evidence
    FOR SELECT TO ple_qti_provenance_broker
    USING ((tenant_id = public.ple_current_tenant()));

CREATE POLICY workspace_qti_profile_item_evidence_staging_broker ON public.workspace_qti_profile_item_evidence
    FOR INSERT TO ple_qti_staging_broker
    WITH CHECK ((tenant_id = public.ple_current_tenant())
        AND public.ple_qti_import_is_prepared(tenant_id, workspace_id, import_id));

CREATE POLICY workspace_qti_profile_import_evidence_provenance_broker_select ON public.workspace_qti_profile_import_evidence
    FOR SELECT TO ple_qti_provenance_broker
    USING ((tenant_id = public.ple_current_tenant()));

ALTER TABLE public.workspace_qti_import_unsupported ENABLE ROW LEVEL SECURITY;

CREATE POLICY workspace_qti_import_unsupported_app_prepared_insert ON public.workspace_qti_import_unsupported FOR INSERT TO ple_app WITH CHECK (((tenant_id = public.ple_current_tenant()) AND public.ple_qti_import_is_prepared(tenant_id, workspace_id, import_id)));

CREATE FUNCTION public.ple_validate_problem_version_number() RETURNS trigger
    LANGUAGE plpgsql
    SET search_path TO 'pg_catalog', 'public'
    AS $$
DECLARE expected_version bigint;
BEGIN
    PERFORM pg_advisory_xact_lock(hashtextextended(NEW.problem_id::text, 0));
    SELECT COALESCE(MAX(version_number), 0) + 1
      INTO expected_version
      FROM public.problem_version
     WHERE problem_id = NEW.problem_id;
    IF NEW.version_number <> expected_version THEN
        RAISE EXCEPTION 'problem version number must be the next one-based value'
            USING ERRCODE = '23514';
    END IF;
    RETURN NEW;
END
$$;

CREATE FUNCTION public.ple_protect_published_problem_version() RETURNS trigger
    LANGUAGE plpgsql
    SET search_path TO 'pg_catalog', 'public'
    AS $$
BEGIN
    IF (to_jsonb(NEW) - 'lifecycle' - 'lifecycle_reason')
        IS DISTINCT FROM
       (to_jsonb(OLD) - 'lifecycle' - 'lifecycle_reason') THEN
        RAISE EXCEPTION 'published problem versions are immutable'
            USING ERRCODE = '55000';
    END IF;
    RETURN NEW;
END
$$;

CREATE FUNCTION public.ple_project_catalog_search_document() RETURNS trigger
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'pg_catalog', 'public'
    AS $$
DECLARE stable_public_id bigint;
BEGIN
    SELECT public_id INTO stable_public_id
      FROM public.problem
     WHERE problem_id = NEW.problem_id;
    INSERT INTO public.catalog_search_document (
        problem_id, version_id, public_id, version_number, title, backend,
        metadata, publication_scope, lifecycle, lifecycle_reason, authors,
        previous_version_id, derived_from_problem_id, derived_from_version_id,
        published_at,
        authors_text, question_type, language, license, taxonomy,
        keywords, capabilities, search_text
    ) VALUES (
        NEW.problem_id,
        NEW.version_id,
        stable_public_id,
        NEW.version_number,
        NEW.title,
        NEW.backend,
        NEW.metadata,
        NEW.publication_scope,
        NEW.lifecycle,
        NEW.lifecycle_reason,
        NEW.authors,
        NEW.previous_version_id,
        NEW.derived_from_problem_id,
        NEW.derived_from_version_id,
        NEW.created_at,
        NEW.authors::text,
        NEW.backend,
        COALESCE(NEW.metadata->>'language', 'und'),
        COALESCE(NEW.metadata #>> '{license,kind}', 'unknown'),
        COALESCE(NEW.metadata->'taxonomy', '[]'::jsonb),
        COALESCE(NEW.metadata->'tags', '[]'::jsonb),
        NEW.capabilities,
        to_tsvector('simple', concat_ws(' ', NEW.title, NEW.authors::text, NEW.metadata::text))
    )
    ON CONFLICT (problem_id, version_id) DO UPDATE SET
        lifecycle = EXCLUDED.lifecycle,
        lifecycle_reason = EXCLUDED.lifecycle_reason,
        updated_at = transaction_timestamp();
    RETURN NEW;
END
$$;

CREATE FUNCTION public.ple_reject_immutable_catalog_mutation() RETURNS trigger
    LANGUAGE plpgsql
    SET search_path TO 'pg_catalog', 'public'
    AS $$
BEGIN
    RAISE EXCEPTION 'published catalog content is immutable'
        USING ERRCODE = '55000';
END
$$;

CREATE FUNCTION public.ple_clear_workspace_flat_question_source() RETURNS trigger
    LANGUAGE plpgsql
    SET search_path TO 'pg_catalog', 'public'
    AS $$
BEGIN
    DELETE FROM public.workspace_flat_question_source
      WHERE tenant_id = NEW.tenant_id AND workspace_id = NEW.workspace_id;
    RETURN NEW;
END
$$;

-- The JSONB envelope preserves exact private compiler bytes as bounded base64.
-- Validate its closed shape and recompute the private checksum before either
-- current staging or immutable publication may trust it.
CREATE FUNCTION public.ple_flat_question_grading_envelope_valid(
    p_key_payload jsonb,
    p_key_sha256 character(64),
    p_public_binding_sha256 character(64)
) RETURNS boolean
    LANGUAGE plpgsql IMMUTABLE STRICT
    SET search_path TO 'pg_catalog', 'public', 'pg_temp'
    AS $$
DECLARE
    decoded_payload bytea;
BEGIN
    IF jsonb_typeof(p_key_payload) <> 'object'::text
       OR octet_length((p_key_payload)::text) > 350000
       OR p_key_sha256 !~ '^[0-9a-f]{64}$'::text
       OR p_public_binding_sha256 !~ '^[0-9a-f]{64}$'::text
       OR NOT (p_key_payload ?& ARRAY[
            'publicSha256', 'payloadSha256', 'payloadBase64'
       ])
       OR p_key_payload - ARRAY[
            'publicSha256', 'payloadSha256', 'payloadBase64'
       ] <> '{}'::jsonb
       OR jsonb_typeof(p_key_payload -> 'publicSha256') <> 'string'::text
       OR jsonb_typeof(p_key_payload -> 'payloadSha256') <> 'string'::text
       OR jsonb_typeof(p_key_payload -> 'payloadBase64') <> 'string'::text
       OR p_key_payload ->> 'publicSha256' <> p_public_binding_sha256
       OR p_key_payload ->> 'payloadSha256' <> p_key_sha256
       OR p_key_payload ->> 'payloadSha256' !~ '^[0-9a-f]{64}$'::text
       OR char_length(p_key_payload ->> 'payloadBase64') NOT BETWEEN 4 AND 349528
       OR char_length(p_key_payload ->> 'payloadBase64') % 4 <> 0
       OR p_key_payload ->> 'payloadBase64' !~ '^[A-Za-z0-9+/]*={0,2}$'::text
    THEN
        RETURN false;
    END IF;

    -- Keep PostgreSQL decoder diagnostics behind the generic capability
    -- boundary. Catch only its documented invalid-argument class around the
    -- one decode operation.
    BEGIN
        decoded_payload := decode(p_key_payload ->> 'payloadBase64', 'base64');
    EXCEPTION WHEN invalid_parameter_value THEN
        RETURN false;
    END;
    RETURN octet_length(decoded_payload) BETWEEN 1 AND 262144
       -- PostgreSQL wraps base64 output every 76 characters; the Rust
       -- STANDARD encoder and this envelope require one unwrapped string.
       AND replace(encode(decoded_payload, 'base64'), E'\n', '') =
           p_key_payload ->> 'payloadBase64'
       AND encode(sha256(decoded_payload), 'hex') = p_key_sha256;
END
$$;

-- Retain a database-side fence even though ordinary callers can write current
-- grading only through the protected staging capability. This also keeps a
-- broker-owned direct write bound to the exact locked draft and source.
CREATE FUNCTION public.ple_validate_workspace_flat_question_grading() RETURNS trigger
    LANGUAGE plpgsql
    SET search_path TO 'pg_catalog', 'public', 'pg_temp'
    AS $$
BEGIN
    IF NEW.tenant_id <> public.ple_current_tenant()
       OR NOT public.ple_flat_question_grading_envelope_valid(
            NEW.key_payload, NEW.key_sha256, NEW.public_binding_sha256
       )
    THEN
        RAISE EXCEPTION 'invalid current flat grading envelope'
            USING ERRCODE = '22023';
    END IF;

    PERFORM 1
      FROM public.workspace_draft AS draft
     WHERE draft.tenant_id = NEW.tenant_id
       AND draft.workspace_id = NEW.workspace_id
       AND draft.revision = NEW.draft_revision
       AND draft.payload_sha256 = NEW.draft_payload_sha256
     FOR UPDATE;
    IF NOT FOUND THEN
        RAISE EXCEPTION 'current flat grading draft binding is invalid'
            USING ERRCODE = '23514';
    END IF;

    PERFORM 1
      FROM public.workspace_flat_question_source AS source
     WHERE source.tenant_id = NEW.tenant_id
       AND source.workspace_id = NEW.workspace_id
       AND source.draft_revision = NEW.draft_revision
       AND source.draft_payload_sha256 = NEW.draft_payload_sha256
       AND source.source_object_id = NEW.source_object_id
       AND source.source_payload_sha256 = NEW.source_payload_sha256
       AND source.canonical_source_sha256 = NEW.canonical_source_sha256
       AND source.public_binding_sha256 = NEW.public_binding_sha256
     FOR KEY SHARE;
    IF NOT FOUND THEN
        RAISE EXCEPTION 'current flat grading source binding is invalid'
            USING ERRCODE = '23514';
    END IF;
    RETURN NEW;
END
$$;

ALTER FUNCTION public.ple_flat_question_grading_envelope_valid(
    jsonb, character(64), character(64)
) OWNER TO ple_grader;
REVOKE ALL ON FUNCTION public.ple_flat_question_grading_envelope_valid(
    jsonb, character(64), character(64)
) FROM PUBLIC;

ALTER FUNCTION public.ple_validate_workspace_flat_question_grading() OWNER TO ple_grader;
REVOKE ALL ON FUNCTION public.ple_validate_workspace_flat_question_grading() FROM PUBLIC;

-- A current origin may pin only the immutable, committed import whose source
-- archive it names. The dedicated provenance capability is the only ordinary
-- writer, but retain this trigger as the database-side binding fence.
CREATE FUNCTION public.ple_validate_workspace_flat_import_origin() RETURNS trigger
    LANGUAGE plpgsql
    SET search_path TO 'pg_catalog', 'public'
    AS $$
BEGIN
    IF NOT EXISTS (
        SELECT 1
          FROM public.workspace_qti_import AS import_row
         WHERE import_row.tenant_id = NEW.tenant_id
           AND import_row.workspace_id = NEW.workspace_id
           AND import_row.import_id = NEW.import_id
           AND import_row.source_object_id = NEW.source_archive_object_id
           AND import_row.payload #>> '{source,id}' = NEW.source_archive_object_id::text
           AND import_row.payload #>> '{source,bucket}' = 'private-content'
           AND import_row.payload #>> '{source,key,kind}' = 'workspaceSource'
           AND import_row.payload #>> '{source,key,tenant}' = NEW.tenant_id::text
           AND import_row.payload #>> '{source,key,workspace}' = NEW.workspace_id::text
           AND import_row.payload #>> '{source,key,import}' = NEW.import_id::text
           AND import_row.payload #>> '{source,key,object}' = NEW.source_archive_object_id::text
           AND import_row.payload #>> '{source,category}' = 'source'
           AND import_row.payload #> '{source,version}' = 'null'::jsonb
           AND import_row.payload #>> '{source,sha256}' = NEW.source_archive_sha256
           AND (import_row.payload #>> '{source,sizeBytes}')::bigint = NEW.source_archive_size_bytes
           AND import_row.payload #>> '{source,mediaType}' = NEW.source_archive_media_type
           AND import_row.payload #>> '{source,license}' = NEW.source_archive_license
           AND import_row.payload #>> '{source,provenance}' = NEW.source_archive_provenance
           AND (import_row.payload #>> '{source,createdAt}')::bigint =
               floor(extract(epoch FROM NEW.source_archive_created_at) * 1000)::bigint
           AND import_row.state = 'committed'::text
    ) THEN
        RAISE EXCEPTION 'flat-import origin must pin its committed QTI archive'
            USING ERRCODE = '23503';
    END IF;
    RETURN NEW;
END
$$;

-- Choice-map bytes are private, but their digest is durable provenance. The
-- provenance broker may write the protected tables directly, so derive the
-- digest at the table boundary as well as in the public capability.
CREATE FUNCTION public.ple_validate_flat_import_choice_map_digest() RETURNS trigger
    LANGUAGE plpgsql
    SET search_path TO 'pg_catalog', 'public'
    AS $$
BEGIN
    IF NEW.choice_map_sha256 <> encode(pg_catalog.sha256(NEW.payload), 'hex') THEN
        RAISE EXCEPTION 'flat-import choice-map payload digest mismatch'
            USING ERRCODE = '23514';
    END IF;
    RETURN NEW;
END
$$;

REVOKE ALL ON FUNCTION public.ple_validate_flat_import_choice_map_digest() FROM PUBLIC;
GRANT EXECUTE ON FUNCTION public.ple_validate_flat_import_choice_map_digest() TO ple_qti_provenance_broker;

-- An import that has become a current-origin pin is immutable as staging
-- evidence. This closes the reverse direction of the origin binding: a
-- staging broker cannot regress or replace the committed archive afterward.
CREATE FUNCTION public.ple_guard_pinned_workspace_qti_import() RETURNS trigger
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'pg_catalog', 'public', 'pg_temp'
    AS $$
BEGIN
    IF EXISTS (
        SELECT 1
          FROM public.workspace_flat_import_origin AS origin
         WHERE origin.tenant_id = OLD.tenant_id
           AND origin.workspace_id = OLD.workspace_id
           AND origin.import_id = OLD.import_id
    ) THEN
        RAISE EXCEPTION 'pinned QTI import is immutable'
            USING ERRCODE = '55000';
    END IF;
    RETURN NEW;
END
$$;

ALTER FUNCTION public.ple_guard_pinned_workspace_qti_import() OWNER TO ple_qti_provenance_broker;
REVOKE ALL ON FUNCTION public.ple_guard_pinned_workspace_qti_import() FROM PUBLIC;

CREATE TRIGGER problem_version_number_guard
    BEFORE INSERT ON public.problem_version
    FOR EACH ROW EXECUTE FUNCTION public.ple_validate_problem_version_number();

CREATE TRIGGER problem_version_immutability
    BEFORE UPDATE ON public.problem_version
    FOR EACH ROW EXECUTE FUNCTION public.ple_protect_published_problem_version();

CREATE TRIGGER problem_version_search_projection
    AFTER INSERT OR UPDATE OF lifecycle, lifecycle_reason ON public.problem_version
    FOR EACH ROW EXECUTE FUNCTION public.ple_project_catalog_search_document();

CREATE TRIGGER answer_key_immutability
    BEFORE UPDATE OR DELETE ON public.answer_key
    FOR EACH ROW EXECUTE FUNCTION public.ple_reject_immutable_catalog_mutation();

CREATE TRIGGER problem_version_payload_immutability
    BEFORE UPDATE OR DELETE ON public.problem_version_payload
    FOR EACH ROW EXECUTE FUNCTION public.ple_reject_immutable_catalog_mutation();

CREATE TRIGGER published_qti_grading_immutability
    BEFORE UPDATE OR DELETE ON public.published_qti_grading
    FOR EACH ROW EXECUTE FUNCTION public.ple_reject_immutable_catalog_mutation();

CREATE TRIGGER published_source_artifact_immutability
    BEFORE UPDATE OR DELETE ON public.published_source_artifact
    FOR EACH ROW EXECUTE FUNCTION public.ple_reject_immutable_catalog_mutation();

CREATE TRIGGER published_flat_import_origin_immutability
    BEFORE UPDATE OR DELETE ON public.published_flat_import_origin
    FOR EACH ROW EXECUTE FUNCTION public.ple_reject_immutable_catalog_mutation();

CREATE TRIGGER published_flat_import_choice_map_immutability
    BEFORE UPDATE OR DELETE ON public.published_flat_import_choice_map
    FOR EACH ROW EXECUTE FUNCTION public.ple_reject_immutable_catalog_mutation();

CREATE TRIGGER published_flat_import_choice_map_digest_guard
    BEFORE INSERT ON public.published_flat_import_choice_map
    FOR EACH ROW EXECUTE FUNCTION public.ple_validate_flat_import_choice_map_digest();

CREATE TRIGGER workspace_flat_import_origin_committed_import_guard
    BEFORE INSERT OR UPDATE ON public.workspace_flat_import_origin
    FOR EACH ROW EXECUTE FUNCTION public.ple_validate_workspace_flat_import_origin();

CREATE TRIGGER workspace_flat_import_choice_map_digest_guard
    BEFORE INSERT OR UPDATE ON public.workspace_flat_import_choice_map
    FOR EACH ROW EXECUTE FUNCTION public.ple_validate_flat_import_choice_map_digest();

CREATE TRIGGER workspace_qti_import_pinned_immutability
    BEFORE UPDATE ON public.workspace_qti_import
    FOR EACH ROW EXECUTE FUNCTION public.ple_guard_pinned_workspace_qti_import();

CREATE TRIGGER workspace_flat_question_source_clear_cache
    BEFORE UPDATE OF payload, payload_sha256, revision ON public.workspace_draft
    FOR EACH ROW EXECUTE FUNCTION public.ple_clear_workspace_flat_question_source();

CREATE TRIGGER workspace_flat_question_grading_binding_guard
    BEFORE INSERT OR UPDATE ON public.workspace_flat_question_grading
    FOR EACH ROW EXECUTE FUNCTION public.ple_validate_workspace_flat_question_grading();

GRANT SELECT,INSERT ON TABLE public.answer_key TO ple_grader;
GRANT SELECT ON TABLE public.problem TO ple_grader;
GRANT SELECT ON TABLE public.problem_version TO ple_grader;
GRANT SELECT ON TABLE public.catalog_tenant_grant TO ple_grader;
GRANT SELECT ON TABLE public.problem_version_payload TO ple_grader;

GRANT SELECT,INSERT ON TABLE public.catalog_tenant_grant TO ple_app;
GRANT SELECT ON TABLE public.catalog_tenant_grant TO ple_student;
GRANT SELECT ON TABLE public.catalog_tenant_grant TO ple_statistics_broker;

GRANT SELECT,INSERT ON TABLE public.problem TO ple_app;
GRANT SELECT ON TABLE public.problem TO ple_catalog_ownership_broker;
GRANT USAGE,SELECT ON SEQUENCE public.problem_public_id_seq TO ple_app;

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.problem_collection TO ple_app;
GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.problem_collection_member TO ple_app;
GRANT SELECT ON TABLE public.catalog_search_document TO ple_app, ple_student;

GRANT SELECT,INSERT ON TABLE public.problem_version TO ple_app;
GRANT SELECT ON TABLE public.problem_version TO ple_student;
GRANT SELECT ON TABLE public.problem_version TO ple_statistics_broker;

GRANT UPDATE(lifecycle) ON TABLE public.problem_version TO ple_app;

GRANT UPDATE(lifecycle_reason) ON TABLE public.problem_version TO ple_app;

GRANT SELECT,INSERT ON TABLE public.problem_version_payload TO ple_app;
GRANT SELECT ON TABLE public.problem_version_payload TO ple_student;

GRANT SELECT,INSERT ON TABLE public.published_qti_grading TO ple_qti_staging_broker;

GRANT SELECT,INSERT ON TABLE public.published_source_artifact TO ple_app;

GRANT SELECT ON TABLE public.problem TO ple_qti_provenance_broker;
GRANT SELECT ON TABLE public.problem_version TO ple_qti_provenance_broker;
GRANT SELECT,INSERT ON TABLE public.published_flat_import_origin TO ple_qti_provenance_broker;
GRANT SELECT,INSERT ON TABLE public.published_flat_import_choice_map TO ple_qti_provenance_broker;

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.workspace_draft TO ple_app;

-- `ple_promote_flat_question_grading` uses the draft row as its one
-- serialization lock.  PostgreSQL requires UPDATE privilege for SELECT FOR
-- UPDATE; scope that privilege to the non-semantic timestamp column and keep
-- the tenant RLS policy in force for the security-definer owner.
GRANT SELECT,UPDATE(updated_at) ON TABLE public.workspace_draft TO ple_grader;

GRANT SELECT,UPDATE(updated_at) ON TABLE public.workspace_draft TO ple_qti_provenance_broker;

GRANT SELECT,INSERT,DELETE ON TABLE public.workspace_flat_question_source TO ple_app;

GRANT SELECT,UPDATE(created_at) ON TABLE public.workspace_flat_question_source TO ple_grader;
GRANT SELECT,INSERT ON TABLE public.workspace_flat_question_grading TO ple_grader;
-- Lock-only UPDATE on the timestamp is required by PostgreSQL for the
-- broker's `FOR KEY SHARE` source consistency check; it grants no source
-- content mutation capability.
GRANT SELECT,UPDATE(created_at) ON TABLE public.workspace_flat_question_source TO ple_qti_provenance_broker;

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.workspace_draft_access TO ple_app;
GRANT SELECT ON TABLE public.workspace_draft_access TO ple_qti_provenance_broker;

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.workspace_flat_import_origin TO ple_qti_provenance_broker;
GRANT SELECT,INSERT,DELETE ON TABLE public.workspace_flat_import_choice_map TO ple_qti_provenance_broker;

GRANT INSERT ON TABLE public.workspace_qti_import TO ple_app;
GRANT SELECT,UPDATE ON TABLE public.workspace_qti_import TO ple_qti_staging_broker;
-- The committed registry is the import-tier lock in the frozen conversion
-- order. Scope PostgreSQL's required row-lock privilege to its timestamp.
GRANT SELECT,UPDATE(created_at) ON TABLE public.workspace_qti_import TO ple_qti_provenance_broker;

GRANT INSERT ON TABLE public.workspace_qti_import_asset TO ple_app;

GRANT INSERT ON TABLE public.workspace_qti_import_grading TO ple_app;
GRANT SELECT ON TABLE public.workspace_qti_import_grading TO ple_qti_staging_broker;

GRANT INSERT ON TABLE public.workspace_qti_import_item TO ple_app;
GRANT SELECT ON TABLE public.workspace_qti_import_item TO ple_qti_staging_broker;
GRANT SELECT ON TABLE public.workspace_qti_import_item TO ple_qti_provenance_broker;

GRANT INSERT ON TABLE public.workspace_qti_import_result TO ple_app;
GRANT SELECT ON TABLE public.workspace_qti_import_result TO ple_qti_staging_broker;
GRANT SELECT ON TABLE public.workspace_qti_import_result TO ple_qti_provenance_broker;

GRANT SELECT,INSERT ON TABLE public.workspace_qti_profile_import_evidence TO ple_qti_staging_broker;
GRANT SELECT ON TABLE public.workspace_qti_profile_import_evidence TO ple_qti_provenance_broker;
GRANT SELECT,INSERT ON TABLE public.workspace_qti_profile_item_evidence TO ple_qti_staging_broker;
GRANT SELECT ON TABLE public.workspace_qti_profile_item_evidence TO ple_qti_provenance_broker;

GRANT INSERT ON TABLE public.workspace_qti_import_unsupported TO ple_app;

REVOKE ALL ON FUNCTION public.ple_prepared_qti_import_matches(p_tenant uuid, p_workspace uuid, p_import uuid, p_registry_payload jsonb, p_registry_sha256 character, p_grading_sha256 jsonb) FROM PUBLIC;
GRANT ALL ON FUNCTION public.ple_prepared_qti_import_matches(p_tenant uuid, p_workspace uuid, p_import uuid, p_registry_payload jsonb, p_registry_sha256 character, p_grading_sha256 jsonb) TO ple_app;

REVOKE ALL ON FUNCTION public.ple_problem_owned_by_current_tenant(uuid) FROM PUBLIC;
GRANT ALL ON FUNCTION public.ple_problem_owned_by_current_tenant(uuid) TO ple_app;

REVOKE ALL ON FUNCTION public.ple_qti_import_is_prepared(p_tenant uuid, p_workspace uuid, p_import uuid) FROM PUBLIC;
GRANT ALL ON FUNCTION public.ple_qti_import_is_prepared(p_tenant uuid, p_workspace uuid, p_import uuid) TO ple_app;

REVOKE ALL ON FUNCTION public.ple_read_committed_qti_grading(p_tenant uuid, p_workspace uuid, p_import uuid, p_item_id text) FROM PUBLIC;
GRANT ALL ON FUNCTION public.ple_read_committed_qti_grading(p_tenant uuid, p_workspace uuid, p_import uuid, p_item_id text) TO ple_grading_reader;

REVOKE ALL ON FUNCTION public.ple_read_committed_qti_import(p_tenant uuid, p_workspace uuid, p_import uuid) FROM PUBLIC;
GRANT ALL ON FUNCTION public.ple_read_committed_qti_import(p_tenant uuid, p_workspace uuid, p_import uuid) TO ple_app;

REVOKE ALL ON FUNCTION public.ple_read_published_qti_grading(p_tenant uuid, p_problem uuid, p_version uuid, p_item_id text) FROM PUBLIC;
GRANT ALL ON FUNCTION public.ple_read_published_qti_grading(p_tenant uuid, p_problem uuid, p_version uuid, p_item_id text) TO ple_grading_reader;

REVOKE ALL ON FUNCTION public.ple_flat_question_grading_material(p_tenant uuid, p_problem uuid, p_version uuid) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION public.ple_flat_question_grading_material(p_tenant uuid, p_problem uuid, p_version uuid) TO ple_grading_reader;

DO $$
BEGIN
    FOR partition_number IN 0..15 LOOP
        EXECUTE format(
            'CREATE TABLE IF NOT EXISTS public.%I PARTITION OF public.problem_version_payload '
            'FOR VALUES WITH (MODULUS 16, REMAINDER %s)',
            'problem_version_payload_p' || partition_number,
            partition_number
        );
    END LOOP;
END
$$;
