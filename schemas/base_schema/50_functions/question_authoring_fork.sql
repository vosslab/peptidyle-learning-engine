-- Exact Published Question Revision forks into private Drafts.

SET LOCAL ROLE ple_private_owner;

-- A fork starts a distinct, private Draft from one server-resolved exact
-- source Revision. Ordinary Draft fields hold the copied state and immediate
-- parent tuple; the ordinary creation receipt holds only retry identity. The
-- actor/key advisory lock makes concurrent retries return one Draft instead
-- of racing a second one.
CREATE FUNCTION ple_private.fork_published_question_to_draft(
    p_authoring_workspace_id uuid,
    p_draft_question_uuid uuid,
    p_source_question_id text,
    p_source_question_revision_number integer,
    p_idempotency_key uuid,
    p_target_object_id uuid,
    p_target_object_address jsonb,
    p_target_sha256 bytea,
    p_target_size_bytes bigint,
    p_target_media_type text,
    p_target_created_at_millis bigint,
    p_question_images jsonb
) RETURNS TABLE (
    draft_question_id uuid,
    authoring_workspace_id uuid,
    created_new boolean
) LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
DECLARE
    actor_id text;
    existing_request_fingerprint bytea;
    request_fingerprint bytea;
    random_identifier text;
    reserved_published_question_id text;
    fork_created_at timestamptz := pg_catalog.clock_timestamp();
    source_revision ple_data.question_revision%ROWTYPE;
    source_metadata ple_data.question_revision_metadata%ROWTYPE;
    source_license text;
    source_citation_text text;
    source_binding ple_private.question_revision_source_binding%ROWTYPE;
    source_record ple_private.object_record%ROWTYPE;
    expected_source_address jsonb;
    asset_count bigint;
    target_image jsonb;
    source_asset_relative_name text;
    target_asset_relative_name text;
    source_question_image_asset_id uuid;
    source_asset_checksum bytea;
    source_asset_media_type text;
    source_asset_size_bytes bigint;
    source_asset_width integer;
    source_asset_height integer;
    target_question_image_asset_id uuid;
    target_question_image_object_id uuid;
    target_asset_address jsonb;
    target_asset_checksum bytea;
    target_asset_size_bytes bigint;
    target_asset_media_type text;
    target_asset_created_at_millis bigint;
    target_asset_width integer;
    target_asset_height integer;
BEGIN
    IF p_authoring_workspace_id IS NULL OR p_draft_question_uuid IS NULL
       OR p_source_question_id IS NULL
       OR p_source_question_id !~ '^[0-9A-HJKMNP-TV-Z]{4}-[0-9A-HJKMNP-TV-Z]{4}$'
       OR substr(p_source_question_id, 6, 1) <> ple_private.crockford_checksum_character(
           substr(p_source_question_id, 1, 4) || substr(p_source_question_id, 7, 3)
       )
       OR p_source_question_revision_number IS NULL OR p_source_question_revision_number <= 0
       OR p_idempotency_key IS NULL
       OR NOT (ple_api.current_session_account_is_instructor()
               OR ple_api.current_session_account_is_sysadmin()) THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Question Fork requires an active Instructor and server-issued inputs';
    END IF;
    actor_id := ple_api.current_session_account_id();
    IF actor_id IS NULL THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Question Fork requires an active Instructor and server-issued inputs';
    END IF;
    -- The actor/key receipt is the first business-state read. A successful
    -- replay remains recoverable after archive and does not inspect fresh
    -- candidate objects that the server will delete as unregistered.
    PERFORM pg_catalog.pg_advisory_xact_lock(
        pg_catalog.hashtextextended(actor_id::text || ':' || p_idempotency_key::text, 0));
    request_fingerprint := pg_catalog.sha256(pg_catalog.convert_to(
        pg_catalog.jsonb_build_object(
            'operation', 'forkPublishedQuestionToDraft',
            'sourcePublishedQuestionId', p_source_question_id,
            'sourceRevisionNumber', p_source_question_revision_number
        )::text,
        'UTF8'));
    SELECT receipt.request_fingerprint
      INTO existing_request_fingerprint
      FROM ple_private.draft_question_creation_receipt AS receipt
     WHERE receipt.actor_account_id = actor_id
       AND receipt.request_key = p_idempotency_key;
    IF FOUND THEN
        IF existing_request_fingerprint IS DISTINCT FROM request_fingerprint THEN
            RAISE EXCEPTION USING ERRCODE = '22023',
                MESSAGE = 'Question creation request key belongs to a different request';
        END IF;
        RETURN QUERY
        SELECT question.draft_question_id, question.authoring_workspace_id, false
          FROM ple_private.draft_question_creation_receipt AS receipt
          JOIN ple_private.draft_question AS question
            ON question.draft_question_id = receipt.draft_question_id
         WHERE receipt.actor_account_id = actor_id
           AND receipt.request_key = p_idempotency_key;
        RETURN;
    END IF;
    IF p_target_object_id IS NULL OR p_target_object_address IS NULL
       OR p_target_sha256 IS NULL OR pg_catalog.octet_length(p_target_sha256) <> 32
       OR p_target_size_bytes IS NULL OR p_target_size_bytes < 0
       OR p_target_media_type IS NULL
       OR pg_catalog.char_length(pg_catalog.btrim(p_target_media_type)) NOT BETWEEN 1 AND 255
       OR p_target_created_at_millis IS NULL THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Question Fork target source arguments are invalid';
    END IF;
    IF NOT ple_api.current_session_account_is_authoring_workspace_owner(p_authoring_workspace_id)
       OR NOT EXISTS (
           SELECT 1 FROM ple_private.authoring_workspace AS workspace
            WHERE workspace.authoring_workspace_id = p_authoring_workspace_id
              AND workspace.owner_account_id = actor_id
              AND workspace.revoked_at IS NULL
            FOR KEY SHARE) THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Question Fork requires the actor owner workspace';
    END IF;
    -- Only available or archived exact Revisions can be forked. Archive is the
    -- sole availability exception; keep the stable parent row locked through
    -- this copy so the exact Revision and its source remain immutable.
    PERFORM 1
      FROM ple_data.published_question AS lineage
      JOIN ple_data.question_revision AS revision
        ON revision.published_question_id = lineage.published_question_id
       AND revision.revision_number = p_source_question_revision_number
     WHERE lineage.published_question_id = p_source_question_id
       AND lineage.availability IN ('available', 'archived')
       FOR KEY SHARE OF lineage;
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = 'QF002',
            MESSAGE = 'Question Fork requires an available or archived exact Published Question Revision';
    END IF;
    LOOP
        random_identifier := ple_private.crockford_id_suffix();
        reserved_published_question_id :=
            pg_catalog.substr(random_identifier, 1, 4) || '-' ||
            ple_private.crockford_checksum_character(random_identifier) ||
            pg_catalog.substr(random_identifier, 5, 3);
        BEGIN
            PERFORM ple_private.reserve_public_id(
                reserved_published_question_id, 'published_question');
            EXIT;
        EXCEPTION WHEN SQLSTATE 'QP001' THEN
            -- The global registry owns collisions across all public object kinds.
            NULL;
        END;
    END LOOP;
    SELECT * INTO STRICT source_revision
      FROM ple_data.question_revision AS revision
     WHERE revision.published_question_id = p_source_question_id
       AND revision.revision_number = p_source_question_revision_number;
    SELECT * INTO STRICT source_metadata
      FROM ple_data.question_revision_metadata AS metadata
     WHERE metadata.published_question_id = p_source_question_id
       AND metadata.revision_number = p_source_question_revision_number
     FOR SHARE;
    SELECT license.spdx_expression::text INTO STRICT source_license
     FROM ple_data.question_revision_license AS license
     WHERE license.published_question_id = p_source_question_id
       AND license.revision_number = p_source_question_revision_number;
    SELECT citation.citation_text
      INTO source_citation_text
     FROM ple_data.question_revision_citation AS citation
     WHERE citation.published_question_id = p_source_question_id
       AND citation.revision_number = p_source_question_revision_number;
    SELECT * INTO STRICT source_binding
      FROM ple_private.question_revision_source_binding AS binding
     WHERE binding.published_question_id = p_source_question_id
       AND binding.revision_number = p_source_question_revision_number
     FOR KEY SHARE;
    IF NOT ple_private.question_backend_is_supported_for_production(source_binding.backend) THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Question Fork source backend is unavailable for new production work';
    END IF;
    SELECT * INTO STRICT source_record
      FROM ple_private.object_record AS record
     WHERE record.object_record_id = source_binding.source_object_record_id;
    expected_source_address := pg_catalog.jsonb_build_object(
        'kind', 'workspaceQuestionSource', 'workspaceId', p_authoring_workspace_id,
        'objectId', p_target_object_id);
    IF p_target_object_address IS DISTINCT FROM expected_source_address
       OR p_target_sha256 IS DISTINCT FROM source_record.sha256
       OR pg_catalog.encode(p_target_sha256, 'hex') IS DISTINCT FROM source_binding.source_object_checksum
       OR p_target_size_bytes IS DISTINCT FROM source_record.size_bytes
       OR p_target_media_type IS DISTINCT FROM source_record.media_type
       OR source_record.object_address IS DISTINCT FROM pg_catalog.jsonb_build_object(
           'kind', 'questionSource',
           'publishedQuestionRevisionTuple', pg_catalog.jsonb_build_object(
               'publishedQuestionId', p_source_question_id,
               'revisionNumber', p_source_question_revision_number),
           'objectId', source_binding.source_object_record_id)
       OR source_record.object_storage_area <> 'private-content'
       OR source_record.object_data_class <> 'question-source' THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Question Fork target must preserve the exact source Revision bytes';
    END IF;
    SELECT pg_catalog.count(*) INTO asset_count
      FROM ple_private.question_image_publication AS publication
     WHERE publication.published_question_id = p_source_question_id
       AND publication.revision_number = p_source_question_revision_number;
    IF pg_catalog.jsonb_typeof(p_question_images) IS DISTINCT FROM 'array' THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Question Fork image copies must be an array';
    END IF;
    IF pg_catalog.jsonb_array_length(p_question_images) <> asset_count
       OR (asset_count > 0 AND source_revision.backend <> 'ple')
       OR EXISTS (
           SELECT 1 FROM pg_catalog.jsonb_array_elements(p_question_images) AS image(value)
           GROUP BY image.value ->> 'questionImageAssetId' HAVING pg_catalog.count(*) <> 1) THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Question Fork must copy every exact source image once';
    END IF;
    INSERT INTO ple_private.draft_question(
        draft_question_id, authoring_workspace_id, parent_published_question_id,
        public_id_reservation_id, parent_revision_number, created_at, updated_at
    ) VALUES (
        p_draft_question_uuid, p_authoring_workspace_id,
        p_source_question_id::ple_data.question_family_id,
        reserved_published_question_id,
        p_source_question_revision_number, fork_created_at, fork_created_at
    );
    INSERT INTO ple_private.draft_question_metadata(
        draft_question_id, question_title, question_description, tags, question_license,
        citation_text, general_feedback, hint, worked_solution, language,
        content_discipline_id, content_subject_id, content_topic_id, content_subtopic_id,
        bloom_cognitive_process, bloom_knowledge_dimension, created_at, updated_at
    ) VALUES (
        p_draft_question_uuid, source_metadata.question_title,
        source_metadata.question_description, source_metadata.tags, source_license::ple_data.license_spdx,
        source_citation_text, source_revision.general_feedback,
        source_revision.hint, source_revision.worked_solution,
        source_metadata.language, source_metadata.content_discipline_id,
        source_metadata.content_subject_id, source_metadata.content_topic_id,
        source_metadata.content_subtopic_id, source_metadata.bloom_cognitive_process,
        source_metadata.bloom_knowledge_dimension, fork_created_at, fork_created_at);
    INSERT INTO ple_private.draft_question_authorship(
        draft_question_id, author_position, author_display_name, author_account_id,
        created_at, updated_at
    ) SELECT p_draft_question_uuid, author.author_position,
             author.author_display_name, author.author_account_id, fork_created_at, fork_created_at
        FROM ple_data.question_revision_authorship AS author
       WHERE author.published_question_id = p_source_question_id
         AND author.revision_number = p_source_question_revision_number
       ORDER BY author.author_position;
    INSERT INTO ple_private.object_record(
        object_record_id, object_address, object_storage_area, object_data_class,
        sha256, size_bytes, media_type, created_at
    ) VALUES (
        p_target_object_id, expected_source_address, 'private-content',
        'authoring-content', p_target_sha256, p_target_size_bytes,
        p_target_media_type,
        pg_catalog.to_timestamp(p_target_created_at_millis::double precision / 1000.0));
    INSERT INTO ple_private.draft_question_source_binding(
        draft_question_id, backend, question_format, question_type,
        webwork_pg_path, source_object_record_id, source_object_checksum,
        created_at, updated_at
    ) VALUES (
        p_draft_question_uuid, source_revision.backend, source_binding.question_format,
        source_metadata.question_type, source_binding.webwork_pg_path,
        p_target_object_id,
        pg_catalog.encode(p_target_sha256, 'hex'), fork_created_at, fork_created_at);
    -- ASVS 8.3.1: trusted persistence verifies each image against the locked source Revision.
    FOR target_image IN SELECT value FROM pg_catalog.jsonb_array_elements(p_question_images) LOOP
        target_question_image_asset_id := (target_image ->> 'questionImageAssetId')::uuid;
        target_question_image_object_id := (target_image ->> 'objectId')::uuid;
        target_asset_relative_name := target_image ->> 'sourceRelativeName';
        target_asset_address := target_image -> 'objectAddress';
        target_asset_checksum := pg_catalog.decode(target_image ->> 'checksum', 'hex');
        target_asset_size_bytes := (target_image ->> 'byteLength')::bigint;
        target_asset_media_type := target_image ->> 'mediaType';
        target_asset_created_at_millis := (target_image ->> 'createdAtMillis')::bigint;
        target_asset_width := (target_image ->> 'intrinsicWidth')::integer;
        target_asset_height := (target_image ->> 'intrinsicHeight')::integer;
        SELECT publication.question_image_asset_id, publication.source_relative_name,
               publication.source_object_checksum, publication.verified_media_type,
               record.size_bytes, publication.intrinsic_width, publication.intrinsic_height
          INTO source_question_image_asset_id, source_asset_relative_name, source_asset_checksum,
               source_asset_media_type, source_asset_size_bytes, source_asset_width, source_asset_height
          FROM ple_private.question_image_publication AS publication
          JOIN ple_private.object_record AS record
            ON record.object_record_id = publication.source_object_record_id
         WHERE publication.published_question_id = p_source_question_id
           AND publication.revision_number = p_source_question_revision_number
           AND publication.question_image_asset_id = target_question_image_asset_id
           AND record.object_address = pg_catalog.jsonb_build_object(
               'kind', 'restrictedQuestionImage',
               'publishedQuestionRevisionTuple', pg_catalog.jsonb_build_object(
                   'publishedQuestionId', p_source_question_id,
                   'revisionNumber', p_source_question_revision_number),
               'questionImageAssetId', publication.question_image_asset_id,
               'objectId', publication.source_object_record_id)
           AND record.object_storage_area = 'private-content'
           AND record.object_data_class = 'question-image'
           AND record.sha256 = publication.source_object_checksum
           AND record.media_type = publication.verified_media_type::text
           FOR SHARE OF publication, record;
        IF NOT FOUND
           OR target_question_image_object_id IS NULL
           OR target_question_image_asset_id IS DISTINCT FROM source_question_image_asset_id
           OR target_asset_relative_name IS DISTINCT FROM source_asset_relative_name
           OR target_asset_checksum IS DISTINCT FROM source_asset_checksum
           OR target_asset_size_bytes IS DISTINCT FROM source_asset_size_bytes
           OR target_asset_media_type IS DISTINCT FROM source_asset_media_type
           OR target_asset_width IS DISTINCT FROM source_asset_width
           OR target_asset_height IS DISTINCT FROM source_asset_height
           OR target_asset_created_at_millis IS NULL
           OR target_asset_address IS DISTINCT FROM pg_catalog.jsonb_build_object(
               'kind', 'draftQuestionImage', 'workspaceId', p_authoring_workspace_id,
               'draftQuestionId', p_draft_question_uuid,
               'questionImageAssetId', target_question_image_asset_id,
               'objectId', target_question_image_object_id) THEN
            RAISE EXCEPTION USING ERRCODE = '23514',
                MESSAGE = 'Question Fork target image must preserve exact source evidence';
        END IF;
        INSERT INTO ple_private.object_record(
            object_record_id, object_address, object_storage_area, object_data_class,
            sha256, size_bytes, media_type, created_at
        ) VALUES (
            target_question_image_object_id, target_asset_address, 'private-content',
            'authoring-content', target_asset_checksum, target_asset_size_bytes,
            target_asset_media_type,
            pg_catalog.to_timestamp(target_asset_created_at_millis::double precision / 1000.0));
        INSERT INTO ple_private.draft_question_image(
            draft_question_id, authoring_workspace_id, question_image_asset_id, source_object_record_id,
            intrinsic_width, intrinsic_height, source_relative_name
        ) VALUES (
            p_draft_question_uuid, p_authoring_workspace_id, target_question_image_asset_id,
            target_question_image_object_id, target_asset_width, target_asset_height,
            target_asset_relative_name);
    END LOOP;
    INSERT INTO ple_private.draft_question_creation_receipt(
        draft_question_id, actor_account_id, request_key, request_fingerprint, created_at
    ) VALUES (
        p_draft_question_uuid, actor_id, p_idempotency_key, request_fingerprint, fork_created_at
    );
    draft_question_id := p_draft_question_uuid;
    authoring_workspace_id := p_authoring_workspace_id;
    created_new := true;
    RETURN NEXT;
END
$$;

SET LOCAL ROLE ple_api_owner;

CREATE FUNCTION ple_api.fork_published_question_to_draft(
    p_authoring_workspace_id uuid,
    p_draft_question_uuid uuid,
    p_source_question_id text,
    p_source_question_revision_number integer,
    p_idempotency_key uuid,
    p_target_object_id uuid,
    p_target_object_address jsonb,
    p_target_sha256 bytea,
    p_target_size_bytes bigint,
    p_target_media_type text,
    p_target_created_at_millis bigint,
    p_question_images jsonb
) RETURNS TABLE (
    draft_question_id uuid,
    authoring_workspace_id uuid,
    created_new boolean
) LANGUAGE sql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private AS $$
    SELECT * FROM ple_private.fork_published_question_to_draft(
        p_authoring_workspace_id, p_draft_question_uuid, p_source_question_id,
        p_source_question_revision_number, p_idempotency_key,
        p_target_object_id, p_target_object_address, p_target_sha256,
        p_target_size_bytes, p_target_media_type, p_target_created_at_millis,
        p_question_images)
$$;
