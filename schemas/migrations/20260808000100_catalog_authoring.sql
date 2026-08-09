-- Initial database epoch: catalog authoring.

SET check_function_bodies = false;

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

ALTER FUNCTION public.ple_prepared_qti_import_matches(uuid, uuid, uuid, jsonb, character, jsonb) OWNER TO ple_qti_staging_broker;

ALTER FUNCTION public.ple_qti_import_is_prepared(uuid, uuid, uuid) OWNER TO ple_qti_staging_broker;

ALTER FUNCTION public.ple_read_committed_qti_grading(uuid, uuid, uuid, text) OWNER TO ple_qti_staging_broker;

ALTER FUNCTION public.ple_read_committed_qti_import(uuid, uuid, uuid) OWNER TO ple_qti_staging_broker;

ALTER FUNCTION public.ple_read_published_qti_grading(uuid, uuid, uuid, text) OWNER TO ple_qti_staging_broker;

CREATE TABLE public.answer_key (
    problem_id uuid NOT NULL,
    version_id uuid NOT NULL,
    key_payload jsonb NOT NULL,
    key_sha256 character(64) NOT NULL
);

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
    CONSTRAINT published_qti_grading_item_id_check CHECK (((length(item_id) >= 1) AND (length(item_id) <= 512))),
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
    CONSTRAINT published_source_artifact_backend_check CHECK ((backend = ANY (ARRAY['webwork'::text, 'qti'::text, 'h5p'::text, 'imathas'::text]))),
    CONSTRAINT published_source_artifact_payload_check CHECK ((jsonb_typeof(payload) = 'object'::text))
);

ALTER TABLE ONLY public.published_source_artifact FORCE ROW LEVEL SECURITY;

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
    CONSTRAINT workspace_qti_import_payload_check CHECK ((jsonb_typeof(payload) = 'object'::text)),
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
    CONSTRAINT workspace_qti_import_asset_payload_check CHECK ((jsonb_typeof(payload) = 'object'::text))
);

ALTER TABLE ONLY public.workspace_qti_import_asset FORCE ROW LEVEL SECURITY;

CREATE TABLE public.workspace_qti_import_grading (
    tenant_id uuid NOT NULL,
    workspace_id uuid NOT NULL,
    import_id uuid NOT NULL,
    item_id text NOT NULL,
    payload bytea NOT NULL,
    payload_sha256 character(64) NOT NULL,
    CONSTRAINT workspace_qti_import_grading_item_id_check CHECK (((length(item_id) >= 1) AND (length(item_id) <= 512))),
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
    CONSTRAINT workspace_qti_import_item_item_id_check CHECK (((length(item_id) >= 1) AND (length(item_id) <= 512))),
    CONSTRAINT workspace_qti_import_item_payload_check CHECK ((jsonb_typeof(payload) = 'object'::text))
);

ALTER TABLE ONLY public.workspace_qti_import_item FORCE ROW LEVEL SECURITY;

CREATE TABLE public.workspace_qti_import_unsupported (
    tenant_id uuid NOT NULL,
    workspace_id uuid NOT NULL,
    import_id uuid NOT NULL,
    ordinal integer NOT NULL,
    payload jsonb NOT NULL,
    payload_sha256 character(64) NOT NULL,
    CONSTRAINT workspace_qti_import_unsupported_ordinal_check CHECK ((ordinal >= 0)),
    CONSTRAINT workspace_qti_import_unsupported_payload_check CHECK ((jsonb_typeof(payload) = 'object'::text))
);

ALTER TABLE ONLY public.workspace_qti_import_unsupported FORCE ROW LEVEL SECURITY;

ALTER TABLE ONLY public.answer_key
    ADD CONSTRAINT answer_key_pkey PRIMARY KEY (problem_id, version_id);

ALTER TABLE ONLY public.catalog_tenant_grant
    ADD CONSTRAINT catalog_tenant_grant_pkey PRIMARY KEY (tenant_id, problem_id, version_id);

ALTER TABLE ONLY public.problem
    ADD CONSTRAINT problem_pkey PRIMARY KEY (problem_id);

ALTER TABLE ONLY public.problem
    ADD CONSTRAINT problem_public_id_key UNIQUE (public_id);

ALTER TABLE public.problem_version
    ADD CONSTRAINT problem_version_authors_nonempty_check CHECK ((jsonb_array_length(authors) > 0)) NOT VALID;

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

ALTER TABLE ONLY public.published_source_artifact
    ADD CONSTRAINT published_source_artifact_object_id_key UNIQUE (object_id);

ALTER TABLE ONLY public.published_source_artifact
    ADD CONSTRAINT published_source_artifact_pkey PRIMARY KEY (problem_id, version_id);

ALTER TABLE ONLY public.workspace_draft_access
    ADD CONSTRAINT workspace_draft_access_pkey PRIMARY KEY (tenant_id, workspace_id, user_id);

ALTER TABLE ONLY public.workspace_draft
    ADD CONSTRAINT workspace_draft_pkey PRIMARY KEY (tenant_id, workspace_id);

ALTER TABLE ONLY public.workspace_qti_import_asset
    ADD CONSTRAINT workspace_qti_import_asset_object_id_key UNIQUE (object_id);

ALTER TABLE ONLY public.workspace_qti_import_asset
    ADD CONSTRAINT workspace_qti_import_asset_pkey PRIMARY KEY (tenant_id, workspace_id, import_id, asset_id);

ALTER TABLE ONLY public.workspace_qti_import_grading
    ADD CONSTRAINT workspace_qti_import_grading_pkey PRIMARY KEY (tenant_id, workspace_id, import_id, item_id);

ALTER TABLE ONLY public.workspace_qti_import_item
    ADD CONSTRAINT workspace_qti_import_item_pkey PRIMARY KEY (tenant_id, workspace_id, import_id, item_id);

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

ALTER TABLE ONLY public.published_source_artifact
    ADD CONSTRAINT published_source_artifact_problem_id_version_id_fkey FOREIGN KEY (problem_id, version_id) REFERENCES public.problem_version(problem_id, version_id) ON DELETE RESTRICT DEFERRABLE INITIALLY DEFERRED;

ALTER TABLE ONLY public.workspace_draft_access
    ADD CONSTRAINT workspace_draft_access_tenant_id_workspace_id_fkey FOREIGN KEY (tenant_id, workspace_id) REFERENCES public.workspace_draft(tenant_id, workspace_id) ON DELETE CASCADE;

ALTER TABLE ONLY public.workspace_qti_import_asset
    ADD CONSTRAINT workspace_qti_import_asset_tenant_id_workspace_id_import_i_fkey FOREIGN KEY (tenant_id, workspace_id, import_id) REFERENCES public.workspace_qti_import(tenant_id, workspace_id, import_id) ON DELETE RESTRICT DEFERRABLE INITIALLY DEFERRED;

ALTER TABLE ONLY public.workspace_qti_import_grading
    ADD CONSTRAINT workspace_qti_import_grading_tenant_id_workspace_id_import_fkey FOREIGN KEY (tenant_id, workspace_id, import_id, item_id) REFERENCES public.workspace_qti_import_item(tenant_id, workspace_id, import_id, item_id) ON DELETE RESTRICT DEFERRABLE INITIALLY DEFERRED;

ALTER TABLE ONLY public.workspace_qti_import_item
    ADD CONSTRAINT workspace_qti_import_item_tenant_id_workspace_id_import_id_fkey FOREIGN KEY (tenant_id, workspace_id, import_id) REFERENCES public.workspace_qti_import(tenant_id, workspace_id, import_id) ON DELETE RESTRICT DEFERRABLE INITIALLY DEFERRED;

ALTER TABLE ONLY public.workspace_qti_import_unsupported
    ADD CONSTRAINT workspace_qti_import_unsuppor_tenant_id_workspace_id_impor_fkey FOREIGN KEY (tenant_id, workspace_id, import_id) REFERENCES public.workspace_qti_import(tenant_id, workspace_id, import_id) ON DELETE RESTRICT DEFERRABLE INITIALLY DEFERRED;

ALTER TABLE public.catalog_tenant_grant ENABLE ROW LEVEL SECURITY;

CREATE POLICY catalog_tenant_grant_statistics_visible_select ON public.catalog_tenant_grant FOR SELECT TO ple_statistics_broker USING ((tenant_id = public.ple_current_tenant()));

CREATE POLICY catalog_tenant_grant_tenant ON public.catalog_tenant_grant USING ((tenant_id = public.ple_current_tenant())) WITH CHECK ((tenant_id = public.ple_current_tenant()));

ALTER TABLE public.problem_version ENABLE ROW LEVEL SECURITY;

CREATE POLICY problem_version_app_insert ON public.problem_version FOR INSERT TO ple_app WITH CHECK (((publication_scope = 'public'::text) OR (EXISTS ( SELECT 1
   FROM public.catalog_tenant_grant grant_row
  WHERE ((grant_row.problem_id = problem_version.problem_id) AND (grant_row.version_id = problem_version.version_id) AND (grant_row.tenant_id = public.ple_current_tenant()))))));

CREATE POLICY problem_version_app_update ON public.problem_version FOR UPDATE TO ple_app USING (((publication_scope = 'public'::text) OR (EXISTS ( SELECT 1
   FROM public.catalog_tenant_grant grant_row
  WHERE ((grant_row.problem_id = problem_version.problem_id) AND (grant_row.version_id = problem_version.version_id) AND (grant_row.tenant_id = public.ple_current_tenant())))))) WITH CHECK (((publication_scope = 'public'::text) OR (EXISTS ( SELECT 1
   FROM public.catalog_tenant_grant grant_row
  WHERE ((grant_row.problem_id = problem_version.problem_id) AND (grant_row.version_id = problem_version.version_id) AND (grant_row.tenant_id = public.ple_current_tenant()))))));

ALTER TABLE public.problem_version_payload ENABLE ROW LEVEL SECURITY;

CREATE POLICY problem_version_payload_app_insert ON public.problem_version_payload FOR INSERT TO ple_app WITH CHECK ((EXISTS ( SELECT 1
   FROM public.problem_version visible_version
  WHERE ((visible_version.problem_id = problem_version_payload.problem_id) AND (visible_version.version_id = problem_version_payload.version_id)))));

CREATE POLICY problem_version_payload_visible_select ON public.problem_version_payload FOR SELECT TO ple_app, ple_student USING ((EXISTS ( SELECT 1
   FROM public.problem_version visible_version
  WHERE ((visible_version.problem_id = problem_version_payload.problem_id) AND (visible_version.version_id = problem_version_payload.version_id)))));

CREATE POLICY problem_version_statistics_visible_select ON public.problem_version FOR SELECT TO ple_statistics_broker USING (((publication_scope = 'public'::text) OR (EXISTS ( SELECT 1
   FROM public.catalog_tenant_grant grant_row
  WHERE ((grant_row.problem_id = problem_version.problem_id) AND (grant_row.version_id = problem_version.version_id) AND (grant_row.tenant_id = public.ple_current_tenant()))))));

CREATE POLICY problem_version_visible_select ON public.problem_version FOR SELECT TO ple_app, ple_student USING (((publication_scope = 'public'::text) OR (EXISTS ( SELECT 1
   FROM public.catalog_tenant_grant grant_row
  WHERE ((grant_row.problem_id = problem_version.problem_id) AND (grant_row.version_id = problem_version.version_id) AND (grant_row.tenant_id = public.ple_current_tenant()))))));

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

ALTER TABLE public.published_source_artifact ENABLE ROW LEVEL SECURITY;

CREATE POLICY published_source_artifact_app_insert ON public.published_source_artifact FOR INSERT TO ple_app WITH CHECK ((EXISTS ( SELECT 1
   FROM public.problem_version visible_version
  WHERE ((visible_version.problem_id = published_source_artifact.problem_id) AND (visible_version.version_id = published_source_artifact.version_id)))));

CREATE POLICY published_source_artifact_visible_select ON public.published_source_artifact FOR SELECT TO ple_app USING ((EXISTS ( SELECT 1
   FROM public.problem_version visible_version
  WHERE ((visible_version.problem_id = published_source_artifact.problem_id) AND (visible_version.version_id = published_source_artifact.version_id)))));

ALTER TABLE public.workspace_draft ENABLE ROW LEVEL SECURITY;

ALTER TABLE public.workspace_draft_access ENABLE ROW LEVEL SECURITY;

CREATE POLICY workspace_draft_access_tenant ON public.workspace_draft_access USING ((tenant_id = public.ple_current_tenant())) WITH CHECK ((tenant_id = public.ple_current_tenant()));

CREATE POLICY workspace_draft_tenant ON public.workspace_draft USING ((tenant_id = public.ple_current_tenant())) WITH CHECK ((tenant_id = public.ple_current_tenant()));

ALTER TABLE public.workspace_qti_import ENABLE ROW LEVEL SECURITY;

CREATE POLICY workspace_qti_import_app_insert ON public.workspace_qti_import FOR INSERT TO ple_app WITH CHECK (((tenant_id = public.ple_current_tenant()) AND (state = 'prepared'::text)));

ALTER TABLE public.workspace_qti_import_asset ENABLE ROW LEVEL SECURITY;

CREATE POLICY workspace_qti_import_asset_app_prepared_insert ON public.workspace_qti_import_asset FOR INSERT TO ple_app WITH CHECK (((tenant_id = public.ple_current_tenant()) AND public.ple_qti_import_is_prepared(tenant_id, workspace_id, import_id)));

ALTER TABLE public.workspace_qti_import_grading ENABLE ROW LEVEL SECURITY;

CREATE POLICY workspace_qti_import_grading_app_prepared_insert ON public.workspace_qti_import_grading FOR INSERT TO ple_app WITH CHECK (((tenant_id = public.ple_current_tenant()) AND public.ple_qti_import_is_prepared(tenant_id, workspace_id, import_id)));

ALTER TABLE public.workspace_qti_import_item ENABLE ROW LEVEL SECURITY;

CREATE POLICY workspace_qti_import_item_app_prepared_insert ON public.workspace_qti_import_item FOR INSERT TO ple_app WITH CHECK (((tenant_id = public.ple_current_tenant()) AND public.ple_qti_import_is_prepared(tenant_id, workspace_id, import_id)));

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

GRANT SELECT,INSERT ON TABLE public.answer_key TO ple_grader;

GRANT SELECT,INSERT ON TABLE public.catalog_tenant_grant TO ple_app;
GRANT SELECT ON TABLE public.catalog_tenant_grant TO ple_student;
GRANT SELECT ON TABLE public.catalog_tenant_grant TO ple_statistics_broker;

GRANT SELECT,INSERT ON TABLE public.problem TO ple_app;
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

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.workspace_draft TO ple_app;

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.workspace_draft_access TO ple_app;

GRANT INSERT ON TABLE public.workspace_qti_import TO ple_app;
GRANT SELECT,UPDATE ON TABLE public.workspace_qti_import TO ple_qti_staging_broker;

GRANT INSERT ON TABLE public.workspace_qti_import_asset TO ple_app;

GRANT INSERT ON TABLE public.workspace_qti_import_grading TO ple_app;
GRANT SELECT ON TABLE public.workspace_qti_import_grading TO ple_qti_staging_broker;

GRANT INSERT ON TABLE public.workspace_qti_import_item TO ple_app;
GRANT SELECT ON TABLE public.workspace_qti_import_item TO ple_qti_staging_broker;

GRANT INSERT ON TABLE public.workspace_qti_import_unsupported TO ple_app;

REVOKE ALL ON FUNCTION public.ple_prepared_qti_import_matches(p_tenant uuid, p_workspace uuid, p_import uuid, p_registry_payload jsonb, p_registry_sha256 character, p_grading_sha256 jsonb) FROM PUBLIC;
GRANT ALL ON FUNCTION public.ple_prepared_qti_import_matches(p_tenant uuid, p_workspace uuid, p_import uuid, p_registry_payload jsonb, p_registry_sha256 character, p_grading_sha256 jsonb) TO ple_app;

REVOKE ALL ON FUNCTION public.ple_qti_import_is_prepared(p_tenant uuid, p_workspace uuid, p_import uuid) FROM PUBLIC;
GRANT ALL ON FUNCTION public.ple_qti_import_is_prepared(p_tenant uuid, p_workspace uuid, p_import uuid) TO ple_app;

REVOKE ALL ON FUNCTION public.ple_read_committed_qti_grading(p_tenant uuid, p_workspace uuid, p_import uuid, p_item_id text) FROM PUBLIC;
GRANT ALL ON FUNCTION public.ple_read_committed_qti_grading(p_tenant uuid, p_workspace uuid, p_import uuid, p_item_id text) TO ple_qti_grader;

REVOKE ALL ON FUNCTION public.ple_read_committed_qti_import(p_tenant uuid, p_workspace uuid, p_import uuid) FROM PUBLIC;
GRANT ALL ON FUNCTION public.ple_read_committed_qti_import(p_tenant uuid, p_workspace uuid, p_import uuid) TO ple_app;

REVOKE ALL ON FUNCTION public.ple_read_published_qti_grading(p_tenant uuid, p_problem uuid, p_version uuid, p_item_id text) FROM PUBLIC;
GRANT ALL ON FUNCTION public.ple_read_published_qti_grading(p_tenant uuid, p_problem uuid, p_version uuid, p_item_id text) TO ple_qti_grader;

DO $$
BEGIN
    FOR partition_number IN 0..15 LOOP
        EXECUTE format(
            'CREATE TABLE public.%I PARTITION OF public.problem_version_payload '
            'FOR VALUES WITH (MODULUS 16, REMAINDER %s)',
            'problem_version_payload_p' || partition_number,
            partition_number
        );
    END LOOP;
END
$$;
