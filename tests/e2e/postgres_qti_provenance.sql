\set ON_ERROR_STOP on

-- Disposable WP-QTI-7 oracle. Its 1,024-character identifier is multibyte so
-- `char_length`, rather than byte length or accidental truncation, is proven.
CREATE TEMP TABLE qti_provenance_probe (
    tenant_id uuid NOT NULL,
    workspace_id uuid NOT NULL,
    actor_id uuid NOT NULL,
    import_id uuid NOT NULL,
    source_archive_object_id uuid NOT NULL,
    source_archive_sha256 character(64) NOT NULL,
    source_archive_size_bytes bigint NOT NULL,
    source_archive_media_type text NOT NULL,
    source_archive_license text NOT NULL,
    source_archive_provenance text NOT NULL,
    source_archive_created_at timestamptz NOT NULL,
    flat_source_object_id uuid NOT NULL,
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
    acknowledged_at timestamptz NOT NULL,
    choice_map_payload bytea NOT NULL,
    problem_id uuid NOT NULL,
    version_id uuid NOT NULL,
    published_archive_object_id uuid NOT NULL,
    published_archive_sha256 character(64) NOT NULL,
    published_archive_size_bytes bigint NOT NULL,
    published_archive_media_type text NOT NULL,
    published_archive_license text NOT NULL,
    published_archive_provenance text NOT NULL,
    published_archive_created_at timestamptz NOT NULL
) ON COMMIT PRESERVE ROWS;

CREATE FUNCTION pg_temp.qti_profile_defaults() RETURNS jsonb
    LANGUAGE sql IMMUTABLE
    RETURN jsonb_build_array(
        jsonb_build_object('code', 'policy', 'location', 'item',
            'detail', 'PLE default applied: unlimited attempts.'),
        jsonb_build_object('code', 'policy', 'location', 'item',
            'detail', 'PLE default applied: immediate full feedback.'),
        jsonb_build_object('code', 'policy', 'location', 'item',
            'detail', 'PLE default applied: untimed.'),
        jsonb_build_object('code', 'policy', 'location', 'item',
            'detail', 'PLE default applied: en-US.'),
        jsonb_build_object('code', 'policy', 'location', 'item',
            'detail', 'PLE default applied: allRightsReserved.'),
        jsonb_build_object('code', 'policy', 'location', 'item',
            'detail', 'PLE default applied: empty tags.'),
        jsonb_build_object('code', 'policy', 'location', 'item',
            'detail', 'PLE default applied: empty taxonomy.'),
        jsonb_build_object('code', 'policy', 'location', 'item',
            'detail', 'PLE default applied: no feedback.')
    );

-- Keep the trusted registry source descriptor in one local constructor. The
-- capability and trigger each verify this exact persisted shape; negative
-- provenance cases below remain inline so their divergent values stay visible.
CREATE FUNCTION pg_temp.qti_workspace_source_descriptor(
    p_tenant_id uuid,
    p_workspace_id uuid,
    p_import_id uuid,
    p_object_id uuid,
    p_sha256 text,
    p_size_bytes bigint,
    p_media_type text,
    p_license text,
    p_provenance text,
    p_created_at timestamptz
) RETURNS jsonb
    LANGUAGE sql
    RETURN jsonb_build_object(
        'id', p_object_id::text,
        'bucket', 'private-content',
        'key', jsonb_build_object(
            'kind', 'workspaceSource',
            'tenant', p_tenant_id::text,
            'workspace', p_workspace_id::text,
            'import', p_import_id::text,
            'object', p_object_id::text
        ),
        'category', 'source',
        'version', NULL,
        'sha256', p_sha256,
        'sizeBytes', p_size_bytes,
        'mediaType', p_media_type,
        'license', p_license,
        'provenance', p_provenance,
        'createdAt', floor(extract(epoch FROM p_created_at) * 1000)::bigint
    );

INSERT INTO qti_provenance_probe
SELECT
    '11111111-1111-4111-8111-0000000000a1'::uuid,
    '11111111-1111-4111-8111-0000000000a2'::uuid,
    '11111111-1111-4111-8111-0000000000a3'::uuid,
    '11111111-1111-4111-8111-0000000000a4'::uuid,
    '11111111-1111-4111-8111-0000000000a5'::uuid,
    repeat('a', 64), 42, 'application/zip', 'CC0-1.0', 'e2e qti archive',
    '2026-08-08 00:00:00+00'::timestamptz,
    '11111111-1111-4111-8111-0000000000a6'::uuid,
    repeat(U&'\4F60', 1024),
    'canvas-qti-1.2-static-single-choice/v1', 'v1', 'v1', 'native-v1',
    repeat('b', 64), repeat('c', 64), repeat('d', 64), repeat('e', 64),
    repeat('f', 64), repeat('1', 64),
    encode(pg_catalog.sha256(decode('0102', 'hex')), 'hex'), repeat('3', 64),
    '2026-08-09 00:00:00+00'::timestamptz, decode('0102', 'hex'),
    '11111111-1111-4111-8111-0000000000a7'::uuid,
    '11111111-1111-4111-8111-0000000000a8'::uuid,
    '11111111-1111-4111-8111-0000000000a9'::uuid,
    repeat('a', 64), 42, 'application/zip', 'CC0-1.0', 'e2e published archive',
    '2026-08-09 01:00:00+00'::timestamptz;
GRANT SELECT ON TABLE qti_provenance_probe TO ple_app;

BEGIN;
INSERT INTO public.workspace_draft (tenant_id, workspace_id, payload, payload_sha256)
SELECT tenant_id, workspace_id, '{}'::jsonb, repeat('4', 64)
  FROM qti_provenance_probe;
INSERT INTO public.workspace_draft_access (tenant_id, workspace_id, user_id, role)
SELECT tenant_id, workspace_id, actor_id, 'owner'
  FROM qti_provenance_probe;
INSERT INTO public.workspace_flat_question_source
    (tenant_id, workspace_id, draft_revision, draft_payload_sha256, source_object_id,
     source_payload, source_payload_sha256, canonical_source_sha256, public_binding_sha256)
SELECT tenant_id, workspace_id, 1, repeat('4', 64), flat_source_object_id,
       '{}'::jsonb, repeat('5', 64), mapped_canonical_source_sha256, repeat('6', 64)
  FROM qti_provenance_probe;
INSERT INTO public.workspace_qti_import
    (tenant_id, workspace_id, import_id, source_object_id, payload, payload_sha256, state)
SELECT tenant_id, workspace_id, import_id, source_archive_object_id,
       jsonb_build_object('source', pg_temp.qti_workspace_source_descriptor(
           tenant_id, workspace_id, import_id, source_archive_object_id,
           source_archive_sha256, source_archive_size_bytes, source_archive_media_type,
           source_archive_license, source_archive_provenance, source_archive_created_at
       ), 'profileSummary', jsonb_build_object(
           'profileId', profile_id, 'profileVersion', profile_version,
           'mappingVersion', mapping_version,
           'profileReportSha256', profile_report_sha256,
           'defaults', pg_temp.qti_profile_defaults()
       )), repeat('7', 64), 'prepared'
  FROM qti_provenance_probe;
INSERT INTO public.workspace_qti_import_item
    (tenant_id, workspace_id, import_id, item_id, payload, payload_sha256)
SELECT tenant_id, workspace_id, import_id, source_item_identifier,
       '{}'::jsonb, repeat('8', 64)
  FROM qti_provenance_probe;
INSERT INTO public.workspace_qti_import_result
    (tenant_id, workspace_id, import_id, ordinal, source_identifier, status,
     normalized_sha256, payload, payload_sha256)
SELECT tenant_id, workspace_id, import_id, 0, source_item_identifier, 'accepted',
       normalized_item_sha256,
       jsonb_build_object('itemId', source_item_identifier), repeat('9', 64)
  FROM qti_provenance_probe;
INSERT INTO public.workspace_qti_import_grading
    (tenant_id, workspace_id, import_id, item_id, payload, payload_sha256)
SELECT tenant_id, workspace_id, import_id, source_item_identifier,
       decode('01', 'hex'), repeat('a', 64)
  FROM qti_provenance_probe;
INSERT INTO public.problem
    (problem_id, question_id, owner_tenant_id, owner_user_id, visibility, license)
SELECT problem_id, 'M4Y9K21', tenant_id, actor_id, 'institution', 'CC0-1.0'
  FROM qti_provenance_probe;
INSERT INTO public.problem_version
    (problem_id, version_id, content_sha256, workspace_id, title,
     backend, publication_scope, authors)
SELECT problem_id, version_id, repeat('a', 64), workspace_id,
       'QTI provenance Unicode boundary', 'native', 'institution', '["E2E"]'::jsonb
  FROM qti_provenance_probe;
INSERT INTO public.published_qti_grading
    (problem_id, version_id, item_id, payload, payload_sha256)
SELECT problem_id, version_id, source_item_identifier, decode('01', 'hex'), repeat('a', 64)
  FROM qti_provenance_probe;
COMMIT;

-- Valid Rust identifiers are 1,024 Unicode scalars, not 1,024 bytes. All
-- linked import/grading surfaces round-trip the exact same value.
DO $$
DECLARE
    expected_identifier text := repeat(U&'\4F60', 1024);
    actual_identifier text;
BEGIN
    SELECT item_id INTO actual_identifier
      FROM public.workspace_qti_import_item
     WHERE import_id = '11111111-1111-4111-8111-0000000000a4'::uuid;
    IF actual_identifier <> expected_identifier OR char_length(actual_identifier) <> 1024 THEN
        RAISE EXCEPTION 'workspace_qti_import_item did not preserve 1,024 Unicode scalars';
    END IF;
    SELECT source_identifier INTO actual_identifier
      FROM public.workspace_qti_import_result
     WHERE import_id = '11111111-1111-4111-8111-0000000000a4'::uuid;
    IF actual_identifier <> expected_identifier OR char_length(actual_identifier) <> 1024 THEN
        RAISE EXCEPTION 'workspace_qti_import_result did not preserve 1,024 Unicode scalars';
    END IF;
    SELECT item_id INTO actual_identifier
      FROM public.workspace_qti_import_grading
     WHERE import_id = '11111111-1111-4111-8111-0000000000a4'::uuid;
    IF actual_identifier <> expected_identifier OR char_length(actual_identifier) <> 1024 THEN
        RAISE EXCEPTION 'workspace_qti_import_grading did not preserve 1,024 Unicode scalars';
    END IF;
    SELECT item_id INTO actual_identifier
      FROM public.published_qti_grading
     WHERE problem_id = '11111111-1111-4111-8111-0000000000a7'::uuid;
    IF actual_identifier <> expected_identifier OR char_length(actual_identifier) <> 1024 THEN
        RAISE EXCEPTION 'published_qti_grading did not preserve 1,024 Unicode scalars';
    END IF;
END
$$;

INSERT INTO public.workspace_qti_import
    (tenant_id, workspace_id, import_id, source_object_id, payload, payload_sha256, state)
VALUES
    ('11111111-1111-4111-8111-0000000000a1',
     '11111111-1111-4111-8111-0000000000a2',
     '11111111-1111-4111-8111-0000000000ac',
     '11111111-1111-4111-8111-0000000000ad', '{}'::jsonb, repeat('7', 64), 'prepared');

DO $$
DECLARE failed_constraint text;
BEGIN
    BEGIN
        INSERT INTO public.workspace_qti_import_item
            (tenant_id, workspace_id, import_id, item_id, payload, payload_sha256)
        VALUES ('11111111-1111-4111-8111-0000000000a1',
                '11111111-1111-4111-8111-0000000000a2',
                '11111111-1111-4111-8111-0000000000ac', repeat(U&'\4F60', 1025),
                '{}'::jsonb, repeat('8', 64));
        RAISE EXCEPTION '1,025-scalar QTI item identifier was accepted';
    EXCEPTION WHEN check_violation THEN
        GET STACKED DIAGNOSTICS failed_constraint = CONSTRAINT_NAME;
        IF failed_constraint <> 'workspace_qti_import_item_item_id_check' THEN
            RAISE;
        END IF;
    END;
    BEGIN
        INSERT INTO public.workspace_qti_import_result
            (tenant_id, workspace_id, import_id, ordinal, source_identifier, status,
             normalized_sha256, payload, payload_sha256)
        VALUES ('11111111-1111-4111-8111-0000000000a1',
                '11111111-1111-4111-8111-0000000000a2',
                '11111111-1111-4111-8111-0000000000ac', 0, repeat(U&'\4F60', 1025),
                'accepted', repeat('b', 64), '{}'::jsonb, repeat('9', 64));
        RAISE EXCEPTION '1,025-scalar QTI result identifier was accepted';
    EXCEPTION WHEN check_violation THEN
        GET STACKED DIAGNOSTICS failed_constraint = CONSTRAINT_NAME;
        IF failed_constraint <> 'workspace_qti_import_result_source_identifier_check' THEN
            RAISE;
        END IF;
    END;
    BEGIN
        INSERT INTO public.workspace_qti_import_grading
            (tenant_id, workspace_id, import_id, item_id, payload, payload_sha256)
        VALUES ('11111111-1111-4111-8111-0000000000a1',
                '11111111-1111-4111-8111-0000000000a2',
                '11111111-1111-4111-8111-0000000000ac', repeat(U&'\4F60', 1025),
                decode('01', 'hex'), repeat('a', 64));
        RAISE EXCEPTION '1,025-scalar QTI grading identifier was accepted';
    EXCEPTION WHEN check_violation THEN
        GET STACKED DIAGNOSTICS failed_constraint = CONSTRAINT_NAME;
        IF failed_constraint <> 'workspace_qti_import_grading_item_id_check' THEN
            RAISE;
        END IF;
    END;
    BEGIN
        INSERT INTO public.published_qti_grading
            (problem_id, version_id, item_id, payload, payload_sha256)
        VALUES ('11111111-1111-4111-8111-0000000000a7',
                '11111111-1111-4111-8111-0000000000a8', repeat(U&'\4F60', 1025),
                decode('01', 'hex'), repeat('a', 64));
        RAISE EXCEPTION '1,025-scalar published QTI grading identifier was accepted';
    EXCEPTION WHEN check_violation THEN
        GET STACKED DIAGNOSTICS failed_constraint = CONSTRAINT_NAME;
        IF failed_constraint <> 'published_qti_grading_item_id_check' THEN
            RAISE;
        END IF;
    END;
END
$$;

-- The accepted-result payload is part of the prepared proof, independently
-- of its indexed source identifier. A swapped itemId must refuse before any
-- profile or item evidence is written.
UPDATE public.workspace_qti_import_result
   SET payload = jsonb_build_object('itemId', 'swapped-item')
 WHERE import_id = '11111111-1111-4111-8111-0000000000a4'::uuid;

BEGIN;
SET LOCAL ROLE ple_app;
SELECT set_config('ple.tenant_id', '11111111-1111-4111-8111-0000000000a1', true);
DO $$
DECLARE q pg_temp.qti_provenance_probe%ROWTYPE;
BEGIN
    SELECT * INTO q FROM pg_temp.qti_provenance_probe;
    IF public.ple_stage_qti_profile_evidence(
        q.tenant_id, q.workspace_id, q.import_id, q.source_item_identifier,
        q.source_item_identifier, q.profile_id, q.profile_version, q.mapping_version,
        q.profile_report_sha256, q.normalized_item_sha256, q.public_mapping_sha256,
        q.private_mapping_sha256, q.mapping_sha256, q.warning_sha256, q.choice_map_sha256
    ) THEN
        RAISE EXCEPTION 'profile evidence accepted a swapped result payload itemId';
    END IF;
END
$$;
COMMIT;

DO $$
BEGIN
    IF EXISTS (
        SELECT 1 FROM public.workspace_qti_profile_import_evidence
         WHERE import_id = '11111111-1111-4111-8111-0000000000a4'::uuid
    ) OR EXISTS (
        SELECT 1 FROM public.workspace_qti_profile_item_evidence
         WHERE import_id = '11111111-1111-4111-8111-0000000000a4'::uuid
    ) THEN
        RAISE EXCEPTION 'swapped result payload mutated profile evidence';
    END IF;
END
$$;

UPDATE public.workspace_qti_import_result
   SET payload = jsonb_build_object(
       'itemId', (SELECT source_item_identifier FROM qti_provenance_probe)
   )
 WHERE import_id = '11111111-1111-4111-8111-0000000000a4'::uuid;

BEGIN;
SET LOCAL ROLE ple_app;
SELECT set_config('ple.tenant_id', '11111111-1111-4111-8111-0000000000a1', true);
DO $$
DECLARE q pg_temp.qti_provenance_probe%ROWTYPE;
BEGIN
    SELECT * INTO q FROM pg_temp.qti_provenance_probe;
    IF public.ple_stage_qti_profile_evidence(
        q.tenant_id, q.workspace_id, q.import_id, q.source_item_identifier,
        q.source_item_identifier, q.profile_id, q.profile_version, q.mapping_version,
        q.profile_report_sha256, repeat('0', 64), q.public_mapping_sha256,
        q.private_mapping_sha256, q.mapping_sha256, q.warning_sha256, q.choice_map_sha256
    ) THEN
        RAISE EXCEPTION 'profile evidence accepted a normalized digest that disagrees with result';
    END IF;
    IF NOT public.ple_stage_qti_profile_evidence(
        q.tenant_id, q.workspace_id, q.import_id, q.source_item_identifier,
        q.source_item_identifier, q.profile_id, q.profile_version, q.mapping_version,
        q.profile_report_sha256, q.normalized_item_sha256, q.public_mapping_sha256,
        q.private_mapping_sha256, q.mapping_sha256, q.warning_sha256, q.choice_map_sha256
    ) THEN
        RAISE EXCEPTION 'prepared profile evidence capability failed';
    END IF;
    IF public.ple_replace_workspace_flat_import_origin(
        q.tenant_id, q.workspace_id, q.actor_id, q.import_id, q.source_archive_object_id,
        q.source_archive_sha256, q.source_archive_size_bytes, q.source_archive_media_type,
        q.source_archive_license, q.source_archive_provenance, q.source_archive_created_at,
        q.source_item_identifier, q.profile_id, q.profile_version, q.mapping_version,
        q.conversion_version, q.normalized_item_sha256, q.profile_report_sha256,
        q.public_mapping_sha256, q.private_mapping_sha256, q.mapping_sha256,
        q.warning_sha256, q.choice_map_sha256, q.mapped_canonical_source_sha256,
        q.actor_id, q.acknowledged_at, q.choice_map_payload
    ) THEN
        RAISE EXCEPTION 'current origin accepted a prepared QTI import';
    END IF;
END
$$;
COMMIT;

UPDATE public.workspace_qti_import
   SET state = 'committed'
 WHERE import_id = '11111111-1111-4111-8111-0000000000a4'::uuid;

-- The committed reader rechecks the accepted-result itemId rather than
-- trusting previously staged evidence after the result relation changes.
UPDATE public.workspace_qti_import_result
   SET payload = jsonb_build_object('itemId', 'swapped-item')
 WHERE import_id = '11111111-1111-4111-8111-0000000000a4'::uuid;

BEGIN;
SET LOCAL ROLE ple_app;
SELECT set_config('ple.tenant_id', '11111111-1111-4111-8111-0000000000a1', true);
DO $$
DECLARE q pg_temp.qti_provenance_probe%ROWTYPE;
BEGIN
    SELECT * INTO q FROM pg_temp.qti_provenance_probe;
    IF (SELECT count(*) FROM public.ple_read_committed_qti_profile_evidence(
            q.tenant_id, q.workspace_id, q.import_id, q.source_item_identifier
        )) <> 0 THEN
        RAISE EXCEPTION 'committed profile reader accepted a swapped result payload itemId';
    END IF;
    IF public.ple_replace_workspace_flat_import_origin(
        q.tenant_id, q.workspace_id, q.actor_id, q.import_id,
        q.source_archive_object_id, q.source_archive_sha256,
        q.source_archive_size_bytes, q.source_archive_media_type,
        q.source_archive_license, q.source_archive_provenance,
        q.source_archive_created_at, q.source_item_identifier,
        q.profile_id, q.profile_version, q.mapping_version, q.conversion_version,
        q.normalized_item_sha256, q.profile_report_sha256,
        q.public_mapping_sha256, q.private_mapping_sha256, q.mapping_sha256,
        q.warning_sha256, q.choice_map_sha256, q.mapped_canonical_source_sha256,
        q.actor_id, q.acknowledged_at, q.choice_map_payload
    ) THEN
        RAISE EXCEPTION 'current origin accepted a swapped result payload itemId';
    END IF;
END
$$;
COMMIT;

DO $$
BEGIN
    IF EXISTS (
        SELECT 1 FROM public.workspace_flat_import_origin
         WHERE workspace_id = '11111111-1111-4111-8111-0000000000a2'::uuid
    ) OR EXISTS (
        SELECT 1 FROM public.workspace_flat_import_choice_map
         WHERE workspace_id = '11111111-1111-4111-8111-0000000000a2'::uuid
    ) OR NOT EXISTS (
        SELECT 1 FROM public.workspace_draft
         WHERE workspace_id = '11111111-1111-4111-8111-0000000000a2'::uuid
           AND revision = 1
           AND payload_sha256 = repeat('4', 64)
    ) OR NOT EXISTS (
        SELECT 1 FROM public.workspace_flat_question_source
         WHERE workspace_id = '11111111-1111-4111-8111-0000000000a2'::uuid
           AND source_payload_sha256 = repeat('5', 64)
           AND canonical_source_sha256 = repeat('3', 64)
    ) THEN
        RAISE EXCEPTION 'swapped result replacement mutated origin, map, draft, or source';
    END IF;
END
$$;

UPDATE public.workspace_qti_import_result
   SET payload = jsonb_build_object(
       'itemId', (SELECT source_item_identifier FROM qti_provenance_probe)
   )
 WHERE import_id = '11111111-1111-4111-8111-0000000000a4'::uuid;

BEGIN;
SET LOCAL ROLE ple_app;
SELECT set_config('ple.tenant_id', '11111111-1111-4111-8111-0000000000a1', true);
DO $$
DECLARE q pg_temp.qti_provenance_probe%ROWTYPE;
BEGIN
    SELECT * INTO q FROM pg_temp.qti_provenance_probe;
    IF (SELECT count(*) FROM public.ple_read_committed_qti_profile_evidence(
            q.tenant_id, q.workspace_id, q.import_id, q.source_item_identifier
        )) <> 1 THEN
        RAISE EXCEPTION 'committed profile-evidence reader did not return one locked record';
    END IF;
    IF public.ple_replace_workspace_flat_import_origin(
        q.tenant_id, q.workspace_id, q.actor_id, q.import_id,
        '11111111-1111-4111-8111-0000000000ae'::uuid, q.source_archive_sha256,
        q.source_archive_size_bytes, q.source_archive_media_type, q.source_archive_license,
        q.source_archive_provenance, q.source_archive_created_at, q.source_item_identifier,
        q.profile_id, q.profile_version, q.mapping_version, q.conversion_version,
        q.normalized_item_sha256, q.profile_report_sha256, q.public_mapping_sha256,
        q.private_mapping_sha256, q.mapping_sha256, q.warning_sha256, q.choice_map_sha256,
        q.mapped_canonical_source_sha256, q.actor_id, q.acknowledged_at, q.choice_map_payload
    ) THEN
        RAISE EXCEPTION 'current origin accepted a wrong archive object';
    END IF;
    IF public.ple_replace_workspace_flat_import_origin(
        q.tenant_id, q.workspace_id, q.actor_id, q.import_id, q.source_archive_object_id,
        repeat('0', 64), q.source_archive_size_bytes, q.source_archive_media_type,
        q.source_archive_license, q.source_archive_provenance, q.source_archive_created_at,
        q.source_item_identifier, q.profile_id, q.profile_version, q.mapping_version,
        q.conversion_version, q.normalized_item_sha256, q.profile_report_sha256,
        q.public_mapping_sha256, q.private_mapping_sha256, q.mapping_sha256,
        q.warning_sha256, q.choice_map_sha256, q.mapped_canonical_source_sha256,
        q.actor_id, q.acknowledged_at, q.choice_map_payload
    ) THEN
        RAISE EXCEPTION 'current origin accepted archive metadata that disagrees with registry';
    END IF;
    BEGIN
        PERFORM public.ple_replace_workspace_flat_import_origin(
            q.tenant_id, q.workspace_id, q.actor_id, q.import_id, q.source_archive_object_id,
            q.source_archive_sha256, q.source_archive_size_bytes, q.source_archive_media_type,
            q.source_archive_license, q.source_archive_provenance, q.source_archive_created_at,
            q.source_item_identifier, q.profile_id, q.profile_version, q.mapping_version,
            q.conversion_version, q.normalized_item_sha256, q.profile_report_sha256,
            q.public_mapping_sha256, q.private_mapping_sha256, q.mapping_sha256,
            q.warning_sha256, q.choice_map_sha256, q.mapped_canonical_source_sha256,
            q.actor_id, q.acknowledged_at, decode('ffff', 'hex')
        );
        RAISE EXCEPTION 'current origin accepted a mismatched choice-map digest';
    EXCEPTION WHEN SQLSTATE '22023' THEN NULL;
    END;
    IF NOT public.ple_replace_workspace_flat_import_origin(
        q.tenant_id, q.workspace_id, q.actor_id, q.import_id, q.source_archive_object_id,
        q.source_archive_sha256, q.source_archive_size_bytes, q.source_archive_media_type,
        q.source_archive_license, q.source_archive_provenance, q.source_archive_created_at,
        q.source_item_identifier, q.profile_id, q.profile_version, q.mapping_version,
        q.conversion_version, q.normalized_item_sha256, q.profile_report_sha256,
        q.public_mapping_sha256, q.private_mapping_sha256, q.mapping_sha256,
        q.warning_sha256, q.choice_map_sha256, q.mapped_canonical_source_sha256,
        q.actor_id, q.acknowledged_at, q.choice_map_payload
    ) THEN
        RAISE EXCEPTION 'current origin replacement failed';
    END IF;
    IF (SELECT count(*) FROM public.ple_read_workspace_flat_import_origin(
            q.tenant_id, q.workspace_id, q.actor_id
        )) <> 1 THEN
        RAISE EXCEPTION 'authorized current-origin reader did not return choice map';
    END IF;
    IF NOT public.ple_promote_flat_import_origin(
        q.tenant_id, q.workspace_id, q.actor_id, q.problem_id, q.version_id, q.import_id,
        q.source_archive_object_id, q.source_archive_sha256, q.source_item_identifier,
        q.profile_id, q.profile_version, q.mapping_version, q.conversion_version,
        q.normalized_item_sha256, q.profile_report_sha256, q.public_mapping_sha256,
        q.private_mapping_sha256, q.mapping_sha256, q.warning_sha256, q.choice_map_sha256,
        q.mapped_canonical_source_sha256, q.actor_id, q.acknowledged_at,
        q.published_archive_object_id, q.published_archive_sha256,
        q.published_archive_size_bytes, q.published_archive_media_type,
        q.published_archive_license, q.published_archive_provenance,
        q.published_archive_created_at
    ) THEN
        RAISE EXCEPTION 'publication origin promotion failed';
    END IF;
    BEGIN
        PERFORM 1 FROM public.workspace_flat_import_choice_map;
        RAISE EXCEPTION 'ple_app directly read current private choice maps';
    EXCEPTION WHEN insufficient_privilege THEN NULL;
    END;
    BEGIN
        PERFORM 1 FROM public.workspace_qti_profile_item_evidence;
        RAISE EXCEPTION 'ple_app directly read protected profile evidence';
    EXCEPTION WHEN insufficient_privilege THEN NULL;
    END;
    BEGIN
        PERFORM 1 FROM public.ple_read_committed_qti_profile_evidence(
            '22222222-2222-4222-8222-0000000000b1'::uuid,
            q.workspace_id, q.import_id, q.source_item_identifier
        );
        RAISE EXCEPTION 'cross-tenant committed-evidence call was accepted';
    EXCEPTION WHEN SQLSTATE '22023' THEN NULL;
    END;
END
$$;
COMMIT;

-- The normal capability is not the only fence: its NOBYPASSRLS owner can
-- write origin rows directly, so the trigger also rejects divergent registry
-- metadata before a duplicate-row constraint could mask the result.
BEGIN;
SET LOCAL ROLE ple_qti_provenance_broker;
SELECT set_config('ple.tenant_id', '11111111-1111-4111-8111-0000000000a1', true);
DO $$
BEGIN
    BEGIN
        INSERT INTO public.workspace_flat_import_origin
            (tenant_id, workspace_id, import_id, source_archive_object_id,
             source_archive_sha256, source_archive_size_bytes, source_archive_media_type,
             source_archive_license, source_archive_provenance, source_archive_created_at,
             source_item_identifier, profile_id, profile_version, mapping_version,
             conversion_version, normalized_item_sha256, profile_report_sha256,
             public_mapping_sha256, private_mapping_sha256, mapping_sha256, warning_sha256,
             choice_map_sha256, mapped_canonical_source_sha256, acknowledged_by, acknowledged_at)
        VALUES ('11111111-1111-4111-8111-0000000000a1',
                '11111111-1111-4111-8111-0000000000a2',
                '11111111-1111-4111-8111-0000000000a4',
                '11111111-1111-4111-8111-0000000000a5', repeat('0', 64), 42,
                'application/zip', 'CC0-1.0', 'e2e qti archive', '2026-08-08 00:00:00+00',
                repeat(U&'\4F60', 1024), 'canvas-qti-1.2-static-single-choice/v1', 'v1', 'v1',
                'native-v1', repeat('b', 64), repeat('c', 64), repeat('d', 64),
                repeat('e', 64), repeat('f', 64), repeat('1', 64),
                encode(pg_catalog.sha256(decode('0102', 'hex')), 'hex'),
                repeat('3', 64), '11111111-1111-4111-8111-0000000000a3',
                '2026-08-09 00:00:00+00');
        RAISE EXCEPTION 'raw provenance origin accepted divergent registry metadata';
    EXCEPTION WHEN foreign_key_violation THEN NULL;
    END;
    BEGIN
        INSERT INTO public.workspace_flat_import_choice_map
            (tenant_id, workspace_id, choice_map_sha256, payload)
        VALUES ('11111111-1111-4111-8111-0000000000a1',
                '11111111-1111-4111-8111-0000000000a2',
                encode(pg_catalog.sha256(decode('0102', 'hex')), 'hex'), decode('ff', 'hex'));
        RAISE EXCEPTION 'raw broker insert accepted a mismatched current choice-map digest';
    EXCEPTION WHEN check_violation THEN NULL;
    END;
    BEGIN
        INSERT INTO public.published_flat_import_choice_map
            (owner_tenant_id, problem_id, version_id, choice_map_sha256, payload)
        VALUES ('11111111-1111-4111-8111-0000000000a1',
                '11111111-1111-4111-8111-0000000000a7',
                '11111111-1111-4111-8111-0000000000a8',
                encode(pg_catalog.sha256(decode('0102', 'hex')), 'hex'), decode('ff', 'hex'));
        RAISE EXCEPTION 'raw broker insert accepted a mismatched published choice-map digest';
    EXCEPTION WHEN check_violation THEN NULL;
    END;
END
$$;
ROLLBACK;

-- A second tenant has a real current and published origin/map pair so the
-- dedicated NOBYPASSRLS broker sees a meaningful foreign-row denial.
BEGIN;
INSERT INTO public.workspace_draft (tenant_id, workspace_id, payload, payload_sha256)
VALUES ('22222222-2222-4222-8222-0000000000b1',
        '22222222-2222-4222-8222-0000000000b2', '{}'::jsonb, repeat('4', 64));
INSERT INTO public.workspace_qti_import
    (tenant_id, workspace_id, import_id, source_object_id, payload, payload_sha256, state)
VALUES ('22222222-2222-4222-8222-0000000000b1',
        '22222222-2222-4222-8222-0000000000b2',
        '22222222-2222-4222-8222-0000000000b3',
        '22222222-2222-4222-8222-0000000000b4',
        jsonb_build_object('source', pg_temp.qti_workspace_source_descriptor(
            '22222222-2222-4222-8222-0000000000b1',
            '22222222-2222-4222-8222-0000000000b2',
            '22222222-2222-4222-8222-0000000000b3',
            '22222222-2222-4222-8222-0000000000b4',
            repeat('a', 64), 42, 'application/zip', 'CC0-1.0', 'e2e qti archive',
            '2026-08-08 00:00:00+00'::timestamptz
        ), 'profileSummary', jsonb_build_object(
            'profileId', 'canvas-qti-1.2-static-single-choice/v1',
            'profileVersion', 'v1', 'mappingVersion', 'v1',
            'profileReportSha256', repeat('c', 64),
            'defaults', pg_temp.qti_profile_defaults()
        )), repeat('7', 64), 'committed');
INSERT INTO public.workspace_flat_import_origin
    (tenant_id, workspace_id, import_id, source_archive_object_id, source_archive_sha256,
     source_archive_size_bytes, source_archive_media_type, source_archive_license,
     source_archive_provenance, source_archive_created_at, source_item_identifier,
     profile_id, profile_version, mapping_version, conversion_version,
     normalized_item_sha256, profile_report_sha256, public_mapping_sha256,
     private_mapping_sha256, mapping_sha256, warning_sha256, choice_map_sha256,
     mapped_canonical_source_sha256, acknowledged_by, acknowledged_at)
VALUES ('22222222-2222-4222-8222-0000000000b1',
        '22222222-2222-4222-8222-0000000000b2',
        '22222222-2222-4222-8222-0000000000b3',
        '22222222-2222-4222-8222-0000000000b4', repeat('a', 64), 42,
        'application/zip', 'CC0-1.0', 'e2e qti archive', '2026-08-08 00:00:00+00',
        'tenant-b-item', 'canvas-qti-1.2-static-single-choice/v1', 'v1', 'v1', 'native-v1',
        repeat('b', 64), repeat('c', 64), repeat('d', 64), repeat('e', 64),
        repeat('f', 64), repeat('1', 64),
        encode(pg_catalog.sha256(decode('0304', 'hex')), 'hex'), repeat('3', 64),
        '22222222-2222-4222-8222-0000000000b5', '2026-08-09 00:00:00+00');
INSERT INTO public.workspace_flat_import_choice_map
    (tenant_id, workspace_id, choice_map_sha256, payload)
VALUES ('22222222-2222-4222-8222-0000000000b1',
        '22222222-2222-4222-8222-0000000000b2',
        encode(pg_catalog.sha256(decode('0304', 'hex')), 'hex'), decode('0304', 'hex'));
INSERT INTO public.problem
    (problem_id, question_id, owner_tenant_id, owner_user_id, visibility, license)
VALUES ('22222222-2222-4222-8222-0000000000b6',
        'T6X3W85',
        '22222222-2222-4222-8222-0000000000b1',
        '22222222-2222-4222-8222-0000000000b5', 'institution', 'CC0-1.0');
INSERT INTO public.problem_version
    (problem_id, version_id, content_sha256, workspace_id, title,
     backend, publication_scope, authors)
VALUES ('22222222-2222-4222-8222-0000000000b6',
        '22222222-2222-4222-8222-0000000000b7', repeat('a', 64),
        '22222222-2222-4222-8222-0000000000b2', 'tenant B provenance',
        'native', 'institution', '["E2E"]'::jsonb);
INSERT INTO public.published_flat_import_origin
    (owner_tenant_id, problem_id, version_id, source_import_id, source_archive_object_id,
     source_archive_sha256, source_item_identifier, profile_id, profile_version,
     mapping_version, conversion_version, normalized_item_sha256, profile_report_sha256,
     public_mapping_sha256, private_mapping_sha256, mapping_sha256, warning_sha256,
     choice_map_sha256, mapped_canonical_source_sha256, acknowledged_by, acknowledged_at,
     published_archive_object_id, published_archive_sha256, published_archive_size_bytes,
     published_archive_media_type, published_archive_license, published_archive_provenance,
     published_archive_created_at)
VALUES ('22222222-2222-4222-8222-0000000000b1',
        '22222222-2222-4222-8222-0000000000b6',
        '22222222-2222-4222-8222-0000000000b7',
        '22222222-2222-4222-8222-0000000000b3',
        '22222222-2222-4222-8222-0000000000b4', repeat('a', 64), 'tenant-b-item',
        'canvas-qti-1.2-static-single-choice/v1', 'v1', 'v1', 'native-v1',
        repeat('b', 64), repeat('c', 64), repeat('d', 64), repeat('e', 64),
        repeat('f', 64), repeat('1', 64),
        encode(pg_catalog.sha256(decode('0304', 'hex')), 'hex'), repeat('3', 64),
        '22222222-2222-4222-8222-0000000000b5', '2026-08-09 00:00:00+00',
        '22222222-2222-4222-8222-0000000000b8', repeat('a', 64), 42,
        'application/zip', 'CC0-1.0', 'e2e published archive', '2026-08-09 01:00:00+00');
INSERT INTO public.published_flat_import_choice_map
    (owner_tenant_id, problem_id, version_id, choice_map_sha256, payload)
VALUES ('22222222-2222-4222-8222-0000000000b1',
        '22222222-2222-4222-8222-0000000000b6',
        '22222222-2222-4222-8222-0000000000b7',
        encode(pg_catalog.sha256(decode('0304', 'hex')), 'hex'), decode('0304', 'hex'));
COMMIT;

BEGIN;
SET LOCAL ROLE ple_qti_provenance_broker;
SELECT set_config('ple.tenant_id', '11111111-1111-4111-8111-0000000000a1', true);
DO $$
BEGIN
    IF (SELECT count(*) FROM public.workspace_flat_import_origin) <> 1
       OR (SELECT count(*) FROM public.workspace_flat_import_choice_map) <> 1
       OR (SELECT count(*) FROM public.published_flat_import_origin) <> 1
       OR (SELECT count(*) FROM public.published_flat_import_choice_map) <> 1 THEN
        RAISE EXCEPTION 'provenance broker did not read exactly its tenant-A lineage';
    END IF;
    IF EXISTS (SELECT 1 FROM public.workspace_flat_import_origin
               WHERE tenant_id = '22222222-2222-4222-8222-0000000000b1')
       OR EXISTS (SELECT 1 FROM public.published_flat_import_origin
                  WHERE owner_tenant_id = '22222222-2222-4222-8222-0000000000b1') THEN
        RAISE EXCEPTION 'provenance broker leaked tenant-B lineage';
    END IF;
END
$$;
ROLLBACK;

-- Ordinary app, learner, and grader identities cannot enumerate either the
-- immutable provenance record or its protected vendor choice map.
BEGIN;
SET LOCAL ROLE ple_app;
SELECT set_config('ple.tenant_id', '11111111-1111-4111-8111-0000000000a1', true);
DO $$
BEGIN
    BEGIN PERFORM 1 FROM public.published_flat_import_origin;
        RAISE EXCEPTION 'ple_app directly read published provenance';
    EXCEPTION WHEN insufficient_privilege THEN NULL; END;
    BEGIN PERFORM 1 FROM public.published_flat_import_choice_map;
        RAISE EXCEPTION 'ple_app directly read published choice maps';
    EXCEPTION WHEN insufficient_privilege THEN NULL; END;
END
$$;
ROLLBACK;
BEGIN;
SET LOCAL ROLE ple_student;
SELECT set_config('ple.tenant_id', '11111111-1111-4111-8111-0000000000a1', true);
DO $$
BEGIN
    BEGIN PERFORM 1 FROM public.published_flat_import_origin;
        RAISE EXCEPTION 'ple_student directly read published provenance';
    EXCEPTION WHEN insufficient_privilege THEN NULL; END;
    BEGIN PERFORM 1 FROM public.published_flat_import_choice_map;
        RAISE EXCEPTION 'ple_student directly read published choice maps';
    EXCEPTION WHEN insufficient_privilege THEN NULL; END;
END
$$;
ROLLBACK;
BEGIN;
SET LOCAL ROLE ple_grader;
SELECT set_config('ple.tenant_id', '11111111-1111-4111-8111-0000000000a1', true);
DO $$
BEGIN
    BEGIN PERFORM 1 FROM public.published_flat_import_origin;
        RAISE EXCEPTION 'ple_grader directly read published provenance';
    EXCEPTION WHEN insufficient_privilege THEN NULL; END;
    BEGIN PERFORM 1 FROM public.published_flat_import_choice_map;
        RAISE EXCEPTION 'ple_grader directly read published choice maps';
    EXCEPTION WHEN insufficient_privilege THEN NULL; END;
END
$$;
ROLLBACK;

-- The reverse pin is evaluated through the real BYPASSRLS staging owner; the
-- guard itself runs as the tenant-scoped NOBYPASSRLS provenance owner.
BEGIN;
SET LOCAL ROLE ple_qti_staging_broker;
SELECT set_config('ple.tenant_id', '11111111-1111-4111-8111-0000000000a1', true);
DO $$
BEGIN
    BEGIN
        UPDATE public.workspace_qti_import
           SET state = 'prepared'
         WHERE import_id = '11111111-1111-4111-8111-0000000000a4'::uuid;
        RAISE EXCEPTION 'staging broker regressed an origin-pinned import';
    EXCEPTION WHEN SQLSTATE '55000' THEN NULL;
    END;
END
$$;
ROLLBACK;

DO $$
BEGIN
    BEGIN
        DELETE FROM public.workspace_qti_import
         WHERE import_id = '11111111-1111-4111-8111-0000000000a4'::uuid;
        RAISE EXCEPTION 'origin-pinned import was deleted';
    EXCEPTION WHEN SQLSTATE '23001' OR SQLSTATE '23503' THEN NULL;
    END;
    BEGIN
        UPDATE public.published_flat_import_origin
           SET conversion_version = 'native-v2'
         WHERE owner_tenant_id = '11111111-1111-4111-8111-0000000000a1'::uuid;
        RAISE EXCEPTION 'published origin was mutable';
    EXCEPTION WHEN SQLSTATE '55000' THEN NULL;
    END;
    BEGIN
        DELETE FROM public.published_flat_import_origin
         WHERE owner_tenant_id = '11111111-1111-4111-8111-0000000000a1'::uuid;
        RAISE EXCEPTION 'published origin deleted despite immutable cascade guard';
    EXCEPTION WHEN SQLSTATE '55000' THEN NULL;
    END;
    BEGIN
        UPDATE public.published_flat_import_choice_map
           SET payload = decode('ff', 'hex')
         WHERE owner_tenant_id = '11111111-1111-4111-8111-0000000000a1'::uuid;
        RAISE EXCEPTION 'published protected choice map was mutable';
    EXCEPTION WHEN SQLSTATE '55000' THEN NULL;
    END;
END
$$;

DELETE FROM public.workspace_draft
 WHERE tenant_id = '11111111-1111-4111-8111-0000000000a1'::uuid;
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM public.workspace_flat_import_origin
               WHERE tenant_id = '11111111-1111-4111-8111-0000000000a1')
       OR EXISTS (SELECT 1 FROM public.workspace_flat_import_choice_map
                  WHERE tenant_id = '11111111-1111-4111-8111-0000000000a1') THEN
        RAISE EXCEPTION 'draft cleanup retained current-only provenance';
    END IF;
    IF NOT EXISTS (SELECT 1 FROM public.published_flat_import_origin
                   WHERE owner_tenant_id = '11111111-1111-4111-8111-0000000000a1')
       OR NOT EXISTS (SELECT 1 FROM public.published_flat_import_choice_map
                      WHERE owner_tenant_id = '11111111-1111-4111-8111-0000000000a1') THEN
        RAISE EXCEPTION 'draft cleanup erased published provenance';
    END IF;
END
$$;

-- Once current provenance is released with its workspace, durable import
-- children can be cleaned in child-first order and the now-unpinned registry
-- can be removed. Published provenance remains above as the retention hold.
DELETE FROM public.workspace_qti_profile_item_evidence
 WHERE import_id = '11111111-1111-4111-8111-0000000000a4'::uuid;
DELETE FROM public.workspace_qti_profile_import_evidence
 WHERE import_id = '11111111-1111-4111-8111-0000000000a4'::uuid;
DELETE FROM public.workspace_qti_import_grading
 WHERE import_id = '11111111-1111-4111-8111-0000000000a4'::uuid;
DELETE FROM public.workspace_qti_import_result
 WHERE import_id = '11111111-1111-4111-8111-0000000000a4'::uuid;
DELETE FROM public.workspace_qti_import_item
 WHERE import_id = '11111111-1111-4111-8111-0000000000a4'::uuid;
DELETE FROM public.workspace_qti_import
 WHERE import_id = '11111111-1111-4111-8111-0000000000a4'::uuid;
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM public.workspace_qti_import
               WHERE import_id = '11111111-1111-4111-8111-0000000000a4'::uuid) THEN
        RAISE EXCEPTION 'released QTI import could not be cleaned';
    END IF;
END
$$;

DO $$
DECLARE relation_name text;
BEGIN
    IF EXISTS (
        SELECT 1 FROM pg_roles
         WHERE rolname = 'ple_qti_provenance_broker'
           AND (rolcanlogin OR rolsuper OR rolcreatedb OR rolcreaterole OR rolinherit OR rolbypassrls)
    ) THEN
        RAISE EXCEPTION 'provenance broker role is broader than NOLOGIN NOBYPASSRLS';
    END IF;
    FOR relation_name IN SELECT unnest(ARRAY[
        'published_flat_import_origin', 'published_flat_import_choice_map',
        'workspace_flat_import_origin', 'workspace_flat_import_choice_map',
        'workspace_qti_profile_import_evidence', 'workspace_qti_profile_item_evidence'
    ]) LOOP
        IF NOT EXISTS (
            SELECT 1 FROM pg_class AS relation
             WHERE relation.oid = ('public.' || relation_name)::regclass
               AND relation.relrowsecurity AND relation.relforcerowsecurity
        ) THEN
            RAISE EXCEPTION 'provenance relation % is not forced-RLS', relation_name;
        END IF;
        IF has_table_privilege('ple_retention_broker', 'public.' || relation_name, 'SELECT')
           OR has_table_privilege('ple_retention_broker', 'public.' || relation_name, 'DELETE') THEN
            RAISE EXCEPTION 'retention broker can access author provenance relation %', relation_name;
        END IF;
    END LOOP;
    IF has_table_privilege('ple_app', 'public.workspace_flat_import_choice_map', 'SELECT')
       OR has_table_privilege('ple_app', 'public.workspace_qti_profile_item_evidence', 'SELECT')
       OR has_table_privilege('ple_app', 'public.published_flat_import_origin', 'SELECT')
       OR has_table_privilege('ple_student', 'public.published_flat_import_origin', 'SELECT')
       OR has_table_privilege('ple_grader', 'public.published_flat_import_choice_map', 'SELECT')
       OR has_column_privilege('ple_qti_provenance_broker',
           'public.workspace_qti_import_result', 'source_identifier', 'UPDATE')
       OR has_column_privilege('ple_qti_provenance_broker',
           'public.workspace_qti_profile_item_evidence', 'normalized_item_sha256', 'UPDATE') THEN
        RAISE EXCEPTION 'provenance grants are broader than the protected capability contract';
    END IF;
    IF EXISTS (
        SELECT 1
          FROM pg_proc AS procedure
          CROSS JOIN LATERAL aclexplode(coalesce(procedure.proacl,
              acldefault('f', procedure.proowner))) AS acl
         WHERE procedure.proname IN (
             'ple_stage_qti_profile_evidence', 'ple_read_committed_qti_profile_evidence',
             'ple_read_workspace_flat_import_origin', 'ple_replace_workspace_flat_import_origin',
             'ple_promote_flat_import_origin', 'ple_guard_pinned_workspace_qti_import',
             'ple_validate_flat_import_choice_map_digest'
         )
           AND acl.grantee = 0
           AND acl.privilege_type = 'EXECUTE'
    ) THEN
        RAISE EXCEPTION 'a protected QTI provenance function is PUBLIC-executable';
    END IF;
    IF EXISTS (
        SELECT 1 FROM pg_proc AS procedure
         WHERE procedure.proname IN (
             'ple_stage_qti_profile_evidence', 'ple_read_committed_qti_profile_evidence',
             'ple_read_workspace_flat_import_origin', 'ple_replace_workspace_flat_import_origin',
             'ple_promote_flat_import_origin', 'ple_guard_pinned_workspace_qti_import'
         )
           AND NOT (procedure.proconfig @> ARRAY['search_path=pg_catalog, public, pg_temp'])
    ) THEN
        RAISE EXCEPTION 'a protected QTI provenance function lacks safe explicit search_path';
    END IF;
END
$$;
