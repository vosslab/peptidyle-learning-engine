-- Immutable private image descriptors for native flat-question authoring.
--
-- Object bytes remain in the object store.  This registry owns only the
-- verified descriptor that a private workspace may reference.  Rows never
-- change: callers may retry an identical registration, but replacement uses a
-- new logical asset identity and an explicit future promotion workflow.

CREATE TABLE public.workspace_flat_question_asset (
    tenant_id uuid NOT NULL,
    workspace_id uuid NOT NULL,
    asset_id uuid NOT NULL,
    object_id uuid NOT NULL,
    payload jsonb NOT NULL,
    payload_sha256 character(64) NOT NULL,
    intrinsic_width integer NOT NULL,
    intrinsic_height integer NOT NULL,
    media_type text NOT NULL,
    created_at timestamp with time zone DEFAULT transaction_timestamp() NOT NULL,
    CONSTRAINT workspace_flat_question_asset_pkey
        PRIMARY KEY (tenant_id, workspace_id, asset_id),
    CONSTRAINT workspace_flat_question_asset_object_id_key UNIQUE (object_id),
    CONSTRAINT workspace_flat_question_asset_payload_check
        CHECK (jsonb_typeof(payload) = 'object'),
    CONSTRAINT workspace_flat_question_asset_payload_size_check
        CHECK (octet_length(payload::text) <= 16384),
    CONSTRAINT workspace_flat_question_asset_payload_sha256_check
        CHECK (payload_sha256 ~ '^[0-9a-f]{64}$'),
    CONSTRAINT workspace_flat_question_asset_dimensions_check
        CHECK (intrinsic_width BETWEEN 1 AND 2147483647
            AND intrinsic_height BETWEEN 1 AND 2147483647),
    CONSTRAINT workspace_flat_question_asset_media_type_check
        CHECK (media_type IN ('image/jpeg', 'image/png', 'image/webp')),
    CONSTRAINT workspace_flat_question_asset_descriptor_identity_check
        CHECK (payload->>'tenant' = tenant_id::text
            AND payload->>'workspace' = workspace_id::text
            AND payload->>'asset' = asset_id::text
            AND payload #>> '{object,id}' = object_id::text
            AND payload #>> '{object,key,kind}' = 'workspaceQuestionAsset'
            AND payload #>> '{object,key,tenant}' = tenant_id::text
            AND payload #>> '{object,key,workspace}' = workspace_id::text
            AND payload #>> '{object,key,asset}' = asset_id::text
            AND payload #>> '{object,key,object}' = object_id::text
            AND payload #>> '{object,bucket}' = 'content'
            AND payload #>> '{object,category}' = 'asset'
            AND payload #> '{object,version}' = 'null'::jsonb
            AND payload->>'intrinsicWidth' = intrinsic_width::text
            AND payload->>'intrinsicHeight' = intrinsic_height::text
            AND payload #>> '{object,mediaType}' = media_type),
    CONSTRAINT workspace_flat_question_asset_descriptor_object_check
        CHECK (payload #>> '{object,sha256}' ~ '^[0-9a-f]{64}$'
            AND jsonb_typeof(payload #> '{object,sizeBytes}') = 'number'
            AND (payload #>> '{object,sizeBytes}')::numeric > 0
            AND char_length(btrim(payload #>> '{object,license}')) BETWEEN 1 AND 512
            AND char_length(btrim(payload #>> '{object,provenance}')) BETWEEN 1 AND 1024
            AND char_length(btrim(payload->>'displayLabel')) BETWEEN 1 AND 160)
);

ALTER TABLE ONLY public.workspace_flat_question_asset FORCE ROW LEVEL SECURITY;
ALTER TABLE public.workspace_flat_question_asset ENABLE ROW LEVEL SECURITY;

CREATE POLICY workspace_flat_question_asset_app_select
    ON public.workspace_flat_question_asset FOR SELECT TO ple_app
    USING (tenant_id = public.ple_current_tenant());

CREATE POLICY workspace_flat_question_asset_app_insert
    ON public.workspace_flat_question_asset FOR INSERT TO ple_app
    WITH CHECK (tenant_id = public.ple_current_tenant());

-- No UPDATE or DELETE grant/policy: this registry is append-only.  The
-- primary key makes an exact application retry inspectable without allowing a
-- caller to overwrite a logical asset descriptor.
GRANT SELECT, INSERT ON TABLE public.workspace_flat_question_asset TO ple_app;
