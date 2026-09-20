-- Functions, triggers, and views from question_authoring_operations.sql.

SET LOCAL ROLE ple_private_owner;

-- A Draft is current private authoring state.  These operations retain the
-- source-derived language with its title and description from the first save;
-- publication later copies that complete metadata into its immutable Revision.
CREATE FUNCTION ple_private.ensure_own_authoring_workspace(p_proposed_workspace_id uuid)
RETURNS uuid LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private AS $$
DECLARE
    account_id text;
    authoring_workspace_id uuid;
BEGIN
    IF p_proposed_workspace_id IS NULL
       OR NOT ple_api.current_session_account_is_instructor() THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Authoring Workspace requires a current Instructor Account';
    END IF;
    account_id := ple_api.current_session_account_id();
    PERFORM pg_catalog.pg_advisory_xact_lock(
        pg_catalog.hashtextextended(account_id::text, 0));
    SELECT workspace.authoring_workspace_id INTO authoring_workspace_id
      FROM ple_private.authoring_workspace AS workspace
     WHERE workspace.owner_account_id = account_id AND workspace.revoked_at IS NULL
     ORDER BY workspace.created_at, workspace.authoring_workspace_id
     LIMIT 1;
    IF authoring_workspace_id IS NULL THEN
        INSERT INTO ple_private.authoring_workspace(authoring_workspace_id, owner_account_id, created_at)
        VALUES (p_proposed_workspace_id, account_id, pg_catalog.clock_timestamp());
        authoring_workspace_id := p_proposed_workspace_id;
    END IF;
    RETURN authoring_workspace_id;
END
$$;



-- A fork starts a distinct, private, unversioned Draft without copying any
-- browser-provided content. The server has resolved the exact source Revision.
-- Publication mints the new public Question ID; this immutable private receipt
-- keeps only the source provenance. The actor/key advisory lock
-- makes concurrent retries return one Draft instead of racing a second one.
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
    p_hotspot_question_image jsonb
) RETURNS TABLE (
    draft_question_id uuid,
    authoring_workspace_id uuid,
    created_new boolean
) LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
DECLARE
    actor_id text;
    existing_source_question_id text;
    v_existing_source_question_revision_number integer;
    created_at timestamptz := pg_catalog.clock_timestamp();
    source_revision ple_data.question_revision%ROWTYPE;
    source_metadata ple_data.published_question_metadata%ROWTYPE;
    source_binding ple_private.question_revision_source_binding%ROWTYPE;
    source_record ple_private.object_record%ROWTYPE;
    expected_source_address jsonb;
    asset_count bigint;
    source_question_image_asset_id uuid;
    source_asset_object_id uuid;
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
       OR NOT ple_api.current_session_account_is_instructor() THEN
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
    SELECT fork.source_question_id, fork.source_revision_number
      INTO existing_source_question_id, v_existing_source_question_revision_number
      FROM ple_private.draft_question_fork_source AS fork
     WHERE fork.actor_account_id = actor_id
       AND fork.idempotency_key = p_idempotency_key;
    IF FOUND THEN
        IF existing_source_question_id <> p_source_question_id
           OR v_existing_source_question_revision_number <> p_source_question_revision_number THEN
            RAISE EXCEPTION USING ERRCODE = '22023',
                MESSAGE = 'Question Fork idempotency key belongs to a different source Revision';
        END IF;
        RETURN QUERY
        SELECT question.draft_question_id, question.authoring_workspace_id, false
          FROM ple_private.draft_question_fork_source AS fork
          JOIN ple_private.draft_question AS question
            ON question.draft_question_id = fork.draft_question_id
         WHERE fork.actor_account_id = actor_id
           AND fork.idempotency_key = p_idempotency_key;
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
    -- Recheck C876's reader predicate at the write boundary: a source
    -- archived between resolution and creation cannot become a new fork.
    PERFORM 1
      FROM ple_data.published_question AS lineage
      JOIN ple_data.question_revision AS revision
        ON revision.published_question_id = lineage.published_question_id
       AND revision.revision_number = p_source_question_revision_number
     WHERE lineage.published_question_id = p_source_question_id
       AND lineage.availability = 'available'
       -- `set_question_availability` takes FOR UPDATE on this same lineage.
       -- Hold a compatible key-share lock through this transaction so archive
       -- and fork are ordered: this operation cannot commit a fork after an
       -- archive that it observed only before the transition.
       FOR KEY SHARE OF lineage;
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = 'QF002',
            MESSAGE = 'Question Fork requires an available exact Published Question Revision';
    END IF;
    SELECT * INTO STRICT source_revision
      FROM ple_data.question_revision AS revision
     WHERE revision.published_question_id = p_source_question_id
       AND revision.revision_number = p_source_question_revision_number;
    SELECT * INTO STRICT source_metadata
      FROM ple_data.published_question_metadata AS metadata
     WHERE metadata.published_question_id = p_source_question_id
     FOR KEY SHARE;
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
           'questionRevisionTuple', pg_catalog.jsonb_build_object(
               'questionId', p_source_question_id,
               'revisionNumber', p_source_question_revision_number),
           'objectId', source_binding.source_object_record_id)
       OR source_record.object_storage_area <> 'private-content'
       OR source_record.object_data_class <> 'question-source' THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Question Fork target must preserve the exact source Revision bytes';
    END IF;
    IF source_revision.question_type = 'hotspot' THEN
        SELECT pg_catalog.count(*) INTO asset_count
          FROM ple_private.question_image_publication AS publication
         WHERE publication.published_question_id = p_source_question_id
           AND publication.revision_number = p_source_question_revision_number;
        IF source_revision.backend <> 'ple' OR asset_count <> 1 OR p_hotspot_question_image IS NULL THEN
            RAISE EXCEPTION USING ERRCODE = '23514',
                MESSAGE = 'Native HOTSPOT fork requires one exact source raster';
        END IF;
        SELECT publication.question_image_asset_id, publication.source_object_record_id,
               publication.source_object_checksum, publication.verified_media_type,
               record.size_bytes, publication.intrinsic_width, publication.intrinsic_height
          INTO STRICT source_question_image_asset_id, source_asset_object_id, source_asset_checksum,
               source_asset_media_type, source_asset_size_bytes,
               source_asset_width, source_asset_height
          FROM ple_private.question_image_publication AS publication
          JOIN ple_private.object_record AS record
            ON record.object_record_id = publication.source_object_record_id
         WHERE publication.published_question_id = p_source_question_id
           AND publication.revision_number = p_source_question_revision_number
           AND record.object_address = pg_catalog.jsonb_build_object(
               'kind', 'restrictedQuestionImage',
               'questionRevisionTuple', pg_catalog.jsonb_build_object(
                   'questionId', p_source_question_id,
                   'revisionNumber', p_source_question_revision_number),
               'questionImageAssetId', publication.question_image_asset_id,
               'objectId', publication.source_object_record_id)
           AND record.object_storage_area = 'private-content'
           AND record.object_data_class = 'question-image'
           AND record.sha256 = publication.source_object_checksum
           AND record.media_type = publication.verified_media_type;
        target_question_image_asset_id := (p_hotspot_question_image ->> 'questionImageAssetId')::uuid;
        target_question_image_object_id := (p_hotspot_question_image ->> 'objectId')::uuid;
        target_asset_address := p_hotspot_question_image -> 'objectAddress';
        target_asset_checksum := pg_catalog.decode(p_hotspot_question_image ->> 'checksum', 'hex');
        target_asset_size_bytes := (p_hotspot_question_image ->> 'byteLength')::bigint;
        target_asset_media_type := p_hotspot_question_image ->> 'mediaType';
        target_asset_created_at_millis := (p_hotspot_question_image ->> 'createdAtMillis')::bigint;
        target_asset_width := (p_hotspot_question_image ->> 'intrinsicWidth')::integer;
        target_asset_height := (p_hotspot_question_image ->> 'intrinsicHeight')::integer;
        IF target_question_image_asset_id IS DISTINCT FROM source_question_image_asset_id
           OR target_asset_checksum IS DISTINCT FROM source_asset_checksum
           OR target_asset_size_bytes IS DISTINCT FROM source_asset_size_bytes
           OR target_asset_media_type IS DISTINCT FROM source_asset_media_type
           OR target_asset_width IS DISTINCT FROM source_asset_width
           OR target_asset_height IS DISTINCT FROM source_asset_height
           OR target_asset_created_at_millis IS NULL
           OR target_asset_address IS DISTINCT FROM pg_catalog.jsonb_build_object(
               'kind', 'draftQuestionImage', 'workspaceId', p_authoring_workspace_id,
               'draftQuestionId', p_draft_question_uuid,
               'questionImageAssetId', target_question_image_asset_id, 'objectId', target_question_image_object_id) THEN
            RAISE EXCEPTION USING ERRCODE = '23514',
                MESSAGE = 'Question Fork target raster must preserve exact HOTSPOT evidence';
        END IF;
    ELSIF p_hotspot_question_image IS NOT NULL THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Only a native HOTSPOT fork may carry a Draft raster';
    END IF;
    INSERT INTO ple_private.draft_question(
        draft_question_id, authoring_workspace_id, created_at, updated_at
    ) VALUES (
        p_draft_question_uuid, p_authoring_workspace_id, created_at, created_at
    );
    INSERT INTO ple_private.draft_question_metadata(
        draft_question_id, question_title, question_description,
        general_feedback, language, created_at, updated_at
    ) VALUES (
        p_draft_question_uuid, source_metadata.question_title,
        source_metadata.question_description, source_revision.general_feedback,
        source_metadata.language, created_at, created_at);
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
        source_revision.question_type, source_binding.webwork_pg_path,
        p_target_object_id,
        pg_catalog.encode(p_target_sha256, 'hex'), created_at, created_at);
    IF source_revision.question_type = 'hotspot' THEN
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
            intrinsic_width, intrinsic_height
        ) VALUES (
            p_draft_question_uuid, p_authoring_workspace_id, target_question_image_asset_id,
            target_question_image_object_id, target_asset_width, target_asset_height);
    END IF;
    INSERT INTO ple_private.draft_question_fork_source(
        draft_question_id, actor_account_id, idempotency_key,
        source_question_id, source_revision_number, created_at
    ) VALUES (
        p_draft_question_uuid, actor_id, p_idempotency_key,
        p_source_question_id, p_source_question_revision_number, created_at
    );
    draft_question_id := p_draft_question_uuid;
    authoring_workspace_id := p_authoring_workspace_id;
    created_new := true;
    RETURN NEXT;
END
$$;



-- Drafts belong to the Account that owns their Authoring Workspace.  Workspace
-- collaborators may read and edit a Draft through the separate access
-- predicate, but never acquire this ownership fact.  C351 consumes this
-- narrow predicate for its destructive operation.
CREATE FUNCTION ple_private.current_session_account_owns_draft_question(
    p_draft_question_uuid uuid
) RETURNS boolean LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private AS $$
    SELECT p_draft_question_uuid IS NOT NULL
       AND ple_api.current_session_account_is_instructor()
       AND EXISTS (
           SELECT 1
             FROM ple_private.draft_question AS question
             JOIN ple_private.authoring_workspace AS workspace
               ON workspace.authoring_workspace_id = question.authoring_workspace_id
            WHERE question.draft_question_id = p_draft_question_uuid
              AND workspace.owner_account_id = ple_api.current_session_account_id()
              AND workspace.revoked_at IS NULL
       )
$$;

CREATE FUNCTION ple_private.list_authoring_drafts()
RETURNS TABLE (
    draft_question_id uuid, draft_question_edit_number bigint,
    question_title text, question_description text
) LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private AS $$
    SELECT question.draft_question_id, question.draft_question_edit_number,
           metadata.question_title, metadata.question_description
      FROM ple_private.draft_question AS question
      JOIN ple_private.draft_question_metadata AS metadata
        ON metadata.draft_question_id = question.draft_question_id
     WHERE ple_api.current_session_account_is_instructor()
       AND ple_api.current_session_account_can_access_authoring_workspace(question.authoring_workspace_id)
     ORDER BY question.updated_at DESC, question.draft_question_id DESC
$$;

CREATE FUNCTION ple_private.load_authoring_draft(p_draft_question_uuid uuid)
RETURNS TABLE (
    draft_question_id uuid, authoring_workspace_id uuid,
    draft_question_edit_number bigint, question_title text, question_description text,
    general_feedback text, language text, question_type text, object_record_id uuid, object_address jsonb, sha256 bytea,
    size_bytes bigint, media_type text, created_at_millis bigint
) LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private AS $$
    SELECT question.draft_question_id, question.authoring_workspace_id,
           question.draft_question_edit_number, metadata.question_title,
           metadata.question_description, metadata.general_feedback, metadata.language,
           binding.question_type, record.object_record_id,
           record.object_address, record.sha256, record.size_bytes, record.media_type,
           pg_catalog.round(extract(epoch FROM record.created_at) * 1000)::bigint
      FROM ple_private.draft_question AS question
      JOIN ple_private.draft_question_metadata AS metadata
        ON metadata.draft_question_id = question.draft_question_id
      JOIN ple_private.draft_question_source_binding AS binding
        ON binding.draft_question_id = question.draft_question_id
      JOIN ple_private.object_record AS record
        ON record.object_record_id = binding.source_object_record_id
       AND binding.source_object_checksum = pg_catalog.encode(record.sha256, 'hex')
     WHERE p_draft_question_uuid IS NOT NULL
       AND ple_api.current_session_account_is_instructor()
       AND ple_api.current_session_account_can_access_authoring_workspace(question.authoring_workspace_id)
       AND question.draft_question_id = p_draft_question_uuid
       AND record.object_storage_area = 'private-content'
       AND record.object_data_class = 'authoring-content'
       AND record.object_address = pg_catalog.jsonb_build_object(
           'kind', 'workspaceQuestionSource', 'workspaceId', question.authoring_workspace_id,
           'objectId', record.object_record_id)
$$;

CREATE FUNCTION ple_private.create_authoring_draft(
    p_authoring_workspace_id uuid, p_draft_question_uuid uuid, p_object_record_id uuid,
    p_object_address jsonb, p_sha256 bytea, p_size_bytes bigint, p_media_type text,
    p_created_at_millis bigint, p_question_title text, p_question_description text,
    p_language text, p_webwork_pg_path text, p_question_type text, p_question_format text
) RETURNS uuid LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private AS $$
DECLARE
    created_at timestamptz;
    expected_address jsonb;
BEGIN
    IF p_authoring_workspace_id IS NULL OR p_draft_question_uuid IS NULL OR p_object_record_id IS NULL
       OR p_sha256 IS NULL OR pg_catalog.octet_length(p_sha256) <> 32
       OR p_size_bytes IS NULL OR p_size_bytes < 0
       OR NOT (
           (p_media_type = 'application/vnd.peptidyle.question+json'
                AND p_webwork_pg_path IS NULL)
           OR (p_media_type = 'text/x-wework-pg'
                AND p_question_format IN ('webworkPg', 'webworkPgml')
                AND p_webwork_pg_path IS NOT NULL
                AND char_length(p_webwork_pg_path) BETWEEN 1 AND 1024
                AND left(p_webwork_pg_path, 1) <> '/'
                AND position(E'\\' in p_webwork_pg_path) = 0
                AND NOT EXISTS (
                    SELECT 1 FROM unnest(string_to_array(p_webwork_pg_path, '/')) AS segment(value)
                     WHERE segment.value IN ('', '.', '..')
                ))
       )
       OR (p_media_type = 'application/vnd.peptidyle.question+json'
           AND p_question_format <> 'pleQuestionJson')
       OR p_created_at_millis IS NULL
       OR p_question_title IS NULL OR p_question_title <> btrim(p_question_title)
       OR char_length(p_question_title) NOT BETWEEN 1 AND 512 OR p_question_title ~ '[[:cntrl:]]'
       OR p_question_description IS NULL OR p_question_description <> btrim(p_question_description)
       OR char_length(p_question_description) NOT BETWEEN 1 AND 4000 OR p_question_description ~ '[[:cntrl:]]'
       OR p_language IS NULL OR p_language <> btrim(p_language)
       OR char_length(p_language) NOT BETWEEN 2 AND 35
       OR p_question_type NOT IN ('multipleChoice', 'multipleAnswer', 'fillInBlank', 'multipleFillInBlank',
           'numeric', 'matching', 'ordering', 'hotspot')
       OR NOT ple_api.current_session_account_is_instructor()
       OR NOT ple_api.current_session_account_is_authoring_workspace_owner(p_authoring_workspace_id)
       OR (p_question_format = 'pleQuestionJson' AND p_question_type = 'hotspot') THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Draft Question creation arguments are invalid';
    END IF;
    expected_address := pg_catalog.jsonb_build_object(
        'kind', 'workspaceQuestionSource', 'workspaceId', p_authoring_workspace_id, 'objectId', p_object_record_id);
    IF p_object_address IS DISTINCT FROM expected_address THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Draft Question source must use its exact Workspace Question Source Object Address';
    END IF;
    created_at := pg_catalog.to_timestamp(p_created_at_millis::double precision / 1000.0);
    INSERT INTO ple_private.draft_question(
        draft_question_id, authoring_workspace_id, created_at, updated_at
    ) VALUES (
        p_draft_question_uuid, p_authoring_workspace_id, pg_catalog.clock_timestamp(), pg_catalog.clock_timestamp()
    );
    INSERT INTO ple_private.draft_question_metadata(
        draft_question_id, question_title, question_description, language, created_at, updated_at
    ) VALUES (
        p_draft_question_uuid, p_question_title, p_question_description, p_language,
        pg_catalog.clock_timestamp(), pg_catalog.clock_timestamp());
    INSERT INTO ple_private.object_record(
        object_record_id, object_address, object_storage_area, object_data_class,
        sha256, size_bytes, media_type, created_at
    ) VALUES (
        p_object_record_id, expected_address, 'private-content', 'authoring-content',
        p_sha256, p_size_bytes, p_media_type, created_at);
    INSERT INTO ple_private.draft_question_source_binding(
        draft_question_id, backend, question_format, question_type, webwork_pg_path, source_object_record_id,
        source_object_checksum, created_at, updated_at
    ) VALUES (
        p_draft_question_uuid,
        (CASE p_media_type WHEN 'application/vnd.peptidyle.question+json' THEN 'ple' ELSE 'webwork' END)::ple_data.question_backend,
        p_question_format::ple_data.question_format,
        p_question_type::ple_data.question_type, p_webwork_pg_path, p_object_record_id,
        pg_catalog.encode(p_sha256, 'hex'), pg_catalog.clock_timestamp(), pg_catalog.clock_timestamp());
    RETURN p_draft_question_uuid;
END
$$;

CREATE FUNCTION ple_private.save_authoring_draft(
    p_draft_question_uuid uuid, p_expected_draft_question_edit_number bigint, p_object_record_id uuid,
    p_object_address jsonb, p_sha256 bytea, p_size_bytes bigint, p_media_type text,
    p_created_at_millis bigint, p_question_title text, p_question_description text,
    p_language text, p_question_type text, p_hotspot_question_image_asset_id uuid, p_hotspot_checksum text
) RETURNS void LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private AS $$
DECLARE
    v_draft_question_uuid uuid;
    v_workspace_id uuid;
    v_current_draft_question_edit_number bigint;
    v_binding ple_private.draft_question_source_binding%ROWTYPE;
    expected_address jsonb;
BEGIN
    IF p_draft_question_uuid IS NULL
       OR p_expected_draft_question_edit_number IS NULL OR p_expected_draft_question_edit_number <= 0
       OR p_object_record_id IS NULL OR p_sha256 IS NULL OR pg_catalog.octet_length(p_sha256) <> 32
       OR p_size_bytes IS NULL OR p_size_bytes < 0
       OR p_created_at_millis IS NULL
       OR p_question_title IS NULL OR p_question_title <> btrim(p_question_title)
       OR char_length(p_question_title) NOT BETWEEN 1 AND 512 OR p_question_title ~ '[[:cntrl:]]'
       OR p_question_description IS NULL OR p_question_description <> btrim(p_question_description)
       OR char_length(p_question_description) NOT BETWEEN 1 AND 4000 OR p_question_description ~ '[[:cntrl:]]'
       OR p_language IS NULL OR p_language <> btrim(p_language)
       OR char_length(p_language) NOT BETWEEN 2 AND 35
       OR p_question_type NOT IN ('multipleChoice', 'multipleAnswer', 'fillInBlank', 'multipleFillInBlank',
           'numeric', 'matching', 'ordering', 'hotspot')
       OR NOT ple_api.current_session_account_is_instructor() THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Draft Question save arguments are invalid';
    END IF;
    SELECT question.draft_question_id, question.authoring_workspace_id, question.draft_question_edit_number
      INTO v_draft_question_uuid, v_workspace_id, v_current_draft_question_edit_number
      FROM ple_private.draft_question AS question
     WHERE question.draft_question_id = p_draft_question_uuid
       AND ple_api.current_session_account_can_access_authoring_workspace(question.authoring_workspace_id)
     FOR UPDATE;
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Draft Question is not available in the current Authoring Workspace';
    END IF;
    IF v_current_draft_question_edit_number <> p_expected_draft_question_edit_number THEN
        RAISE EXCEPTION USING ERRCODE = '40001', MESSAGE = 'Draft Question Edit Number is stale';
    END IF;
    SELECT * INTO STRICT v_binding
      FROM ple_private.draft_question_source_binding
     WHERE draft_question_id = v_draft_question_uuid
     FOR UPDATE;
    -- ASVS 8.2.2, 15.4.2: source binding and exact Draft-owned image are accepted under one CAS.
    IF v_binding.backend = 'ple' AND p_question_type = 'hotspot' THEN
        IF p_hotspot_question_image_asset_id IS NULL OR p_hotspot_checksum IS NULL OR NOT EXISTS (
            SELECT 1 FROM ple_private.draft_question_image AS image
            JOIN ple_private.object_record AS record ON record.object_record_id = image.source_object_record_id
            WHERE image.draft_question_id = v_draft_question_uuid AND image.authoring_workspace_id = v_workspace_id
              AND image.question_image_asset_id = p_hotspot_question_image_asset_id AND encode(record.sha256, 'hex') = p_hotspot_checksum
              AND ple_private.current_session_is_authoring_workspace_owner(v_workspace_id)
        ) THEN
            RAISE EXCEPTION USING ERRCODE = '23514', MESSAGE = 'Native HOTSPOT requires its exact Draft-owned surface';
        END IF;
    ELSIF p_hotspot_question_image_asset_id IS NOT NULL OR p_hotspot_checksum IS NOT NULL THEN
        RAISE EXCEPTION USING ERRCODE = '23514', MESSAGE = 'Only native HOTSPOT accepts a Draft image surface';
    END IF;
    IF NOT (
        (v_binding.backend = 'ple' AND v_binding.question_format = 'pleQuestionJson'
            AND v_binding.webwork_pg_path IS NULL
            AND p_media_type = 'application/vnd.peptidyle.question+json')
        OR (v_binding.backend = 'webwork' AND v_binding.question_format IN ('webworkPg', 'webworkPgml')
            AND v_binding.webwork_pg_path IS NOT NULL
            AND p_media_type = 'text/x-wework-pg')
    ) THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Draft Question source media type must match its registered source binding';
    END IF;
    expected_address := pg_catalog.jsonb_build_object(
        'kind', 'workspaceQuestionSource', 'workspaceId', v_workspace_id, 'objectId', p_object_record_id);
    IF p_object_address IS DISTINCT FROM expected_address THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Draft Question source must use its exact Workspace Question Source Object Address';
    END IF;
    INSERT INTO ple_private.object_record(
        object_record_id, object_address, object_storage_area, object_data_class,
        sha256, size_bytes, media_type, created_at
    ) VALUES (
        p_object_record_id, expected_address, 'private-content', 'authoring-content', p_sha256,
        p_size_bytes, p_media_type,
        pg_catalog.to_timestamp(p_created_at_millis::double precision / 1000.0));
    UPDATE ple_private.draft_question_source_binding
       SET source_object_record_id = p_object_record_id,
           source_object_checksum = pg_catalog.encode(p_sha256, 'hex'),
           question_type = p_question_type,
           updated_at = pg_catalog.clock_timestamp()
     WHERE draft_question_id = v_draft_question_uuid;
    UPDATE ple_private.draft_question_metadata
       SET question_title = p_question_title,
           question_description = p_question_description,
           language = p_language,
           updated_at = pg_catalog.clock_timestamp()
     WHERE draft_question_id = v_draft_question_uuid;
    UPDATE ple_private.draft_question
       SET draft_question_edit_number = draft_question_edit_number + 1,
           updated_at = pg_catalog.clock_timestamp()
     WHERE draft_question_id = v_draft_question_uuid;
END
$$;



-- General feedback is deliberately authored PLE metadata.  This is the one
-- metadata editor boundary: it neither reads nor parses backend source or
-- dynamic backend feedback.  A Draft edit advances the ordinary Draft CAS;
-- publication then records the exact text on a new immutable Revision.
CREATE FUNCTION ple_private.save_authoring_draft_general_feedback(
    p_draft_question_uuid uuid, p_expected_draft_question_edit_number bigint, p_general_feedback text
) RETURNS TABLE (draft_question_edit_number bigint, general_feedback text)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private AS $$
DECLARE
    v_draft_question_uuid uuid;
    v_current_draft_question_edit_number bigint;
BEGIN
    IF p_draft_question_uuid IS NULL
       OR p_expected_draft_question_edit_number IS NULL OR p_expected_draft_question_edit_number <= 0
       OR (p_general_feedback IS NOT NULL AND (
           p_general_feedback <> btrim(p_general_feedback)
           OR char_length(p_general_feedback) NOT BETWEEN 1 AND 4000
       ))
       OR NOT ple_api.current_session_account_is_instructor() THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Draft Question general feedback arguments are invalid';
    END IF;
    IF p_general_feedback IS NOT NULL AND p_general_feedback ~ '[[:cntrl:]]' THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Draft Question general feedback arguments are invalid';
    END IF;
    SELECT question.draft_question_id, question.draft_question_edit_number
      INTO v_draft_question_uuid, v_current_draft_question_edit_number
      FROM ple_private.draft_question AS question
     WHERE question.draft_question_id = p_draft_question_uuid
       AND ple_api.current_session_account_can_access_authoring_workspace(question.authoring_workspace_id)
     FOR UPDATE;
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Draft Question is not available in the current Authoring Workspace';
    END IF;
    IF v_current_draft_question_edit_number <> p_expected_draft_question_edit_number THEN
        RAISE EXCEPTION USING ERRCODE = '40001',
            MESSAGE = 'Draft Question Edit Number is stale';
    END IF;
    UPDATE ple_private.draft_question_metadata
       SET general_feedback = p_general_feedback,
           updated_at = pg_catalog.clock_timestamp()
     WHERE draft_question_id = v_draft_question_uuid;
    UPDATE ple_private.draft_question AS question
       SET draft_question_edit_number = question.draft_question_edit_number + 1,
           updated_at = pg_catalog.clock_timestamp()
     WHERE question.draft_question_id = v_draft_question_uuid
     RETURNING question.draft_question_edit_number INTO v_current_draft_question_edit_number;
    RETURN QUERY SELECT v_current_draft_question_edit_number, p_general_feedback;
END
$$;



-- ASVS 1.2.4, 2.2.1-2.2.2, and 2.3.1-2.3.4: resolve the private Draft UUID
-- under current owner authority, lock the Draft, enforce its exact
-- Edit Number, and delete the private aggregate atomically.  Published
-- Question lineages are separate ple_data state and are never considered.
CREATE FUNCTION ple_private.delete_draft_question(
    p_draft_question_uuid uuid,
    p_expected_draft_question_edit_number bigint
) RETURNS void LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private AS $$
DECLARE
    v_draft_question_uuid uuid;
    v_current_draft_question_edit_number bigint;
BEGIN
    IF p_draft_question_uuid IS NULL
       OR p_expected_draft_question_edit_number IS NULL OR p_expected_draft_question_edit_number <= 0 THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Draft Question deletion arguments are invalid';
    END IF;
    IF NOT ple_api.current_session_account_is_instructor() THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Draft Question deletion requires its current owner';
    END IF;
    SELECT question.draft_question_id, question.draft_question_edit_number
      INTO v_draft_question_uuid, v_current_draft_question_edit_number
      FROM ple_private.draft_question AS question
     WHERE question.draft_question_id = p_draft_question_uuid
       AND ple_private.current_session_account_owns_draft_question(
               question.draft_question_id)
     FOR UPDATE OF question;
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Draft Question deletion requires its current owner';
    END IF;
    IF v_current_draft_question_edit_number <> p_expected_draft_question_edit_number THEN
        RAISE EXCEPTION USING ERRCODE = '40001',
            MESSAGE = 'Draft Question Edit Number is stale';
    END IF;
    PERFORM pg_catalog.set_config(
        'ple.authorized_draft_delete_uuid', v_draft_question_uuid::text, true);
    DELETE FROM ple_private.draft_question
     WHERE draft_question_id = v_draft_question_uuid;
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Draft Question deletion requires its current owner';
    END IF;
END
$$;

SET LOCAL ROLE ple_api_owner;

CREATE FUNCTION ple_api.ensure_own_authoring_workspace(p_proposed_workspace_id uuid)
RETURNS uuid LANGUAGE sql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private AS $$
    SELECT ple_private.ensure_own_authoring_workspace(p_proposed_workspace_id)
$$;

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
    p_hotspot_question_image jsonb
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
        p_hotspot_question_image)
$$;

CREATE FUNCTION ple_api.current_session_account_owns_draft_question(
    p_draft_question_uuid uuid
) RETURNS boolean LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private AS $$
    SELECT ple_private.current_session_account_owns_draft_question(p_draft_question_uuid)
$$;

CREATE FUNCTION ple_api.list_authoring_drafts()
RETURNS TABLE (
    draft_question_id uuid, draft_question_edit_number bigint,
    question_title text, question_description text
) LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private AS $$
    SELECT * FROM ple_private.list_authoring_drafts()
$$;

CREATE FUNCTION ple_api.load_authoring_draft(p_draft_question_uuid uuid)
RETURNS TABLE (
    draft_question_id uuid, authoring_workspace_id uuid,
    draft_question_edit_number bigint, question_title text, question_description text,
    general_feedback text, language text, question_type text, object_record_id uuid, object_address jsonb, sha256 bytea,
    size_bytes bigint, media_type text, created_at_millis bigint
) LANGUAGE sql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private AS $$
    SELECT * FROM ple_private.load_authoring_draft(p_draft_question_uuid)
$$;

CREATE FUNCTION ple_api.create_authoring_draft(
    p_authoring_workspace_id uuid, p_draft_question_uuid uuid, p_object_record_id uuid,
    p_object_address jsonb, p_sha256 bytea, p_size_bytes bigint, p_media_type text,
    p_created_at_millis bigint, p_question_title text, p_question_description text,
    p_language text, p_webwork_pg_path text, p_question_type text, p_question_format text
) RETURNS uuid LANGUAGE sql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private AS $$
    SELECT ple_private.create_authoring_draft(
        p_authoring_workspace_id, p_draft_question_uuid, p_object_record_id, p_object_address, p_sha256,
        p_size_bytes, p_media_type, p_created_at_millis, p_question_title,
        p_question_description, p_language, p_webwork_pg_path, p_question_type, p_question_format)
$$;

CREATE FUNCTION ple_api.save_authoring_draft(
    p_draft_question_uuid uuid, p_expected_draft_question_edit_number bigint, p_object_record_id uuid,
    p_object_address jsonb, p_sha256 bytea, p_size_bytes bigint, p_media_type text,
    p_created_at_millis bigint, p_question_title text, p_question_description text,
    p_language text, p_question_type text, p_hotspot_question_image_asset_id uuid, p_hotspot_checksum text
) RETURNS void LANGUAGE sql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private AS $$
    SELECT ple_private.save_authoring_draft(
        p_draft_question_uuid, p_expected_draft_question_edit_number, p_object_record_id, p_object_address, p_sha256,
        p_size_bytes, p_media_type, p_created_at_millis, p_question_title,
        p_question_description, p_language, p_question_type, p_hotspot_question_image_asset_id, p_hotspot_checksum)
$$;

CREATE FUNCTION ple_api.save_authoring_draft_general_feedback(
    p_draft_question_uuid uuid, p_expected_draft_question_edit_number bigint, p_general_feedback text
) RETURNS TABLE (draft_question_edit_number bigint, general_feedback text)
LANGUAGE sql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private AS $$
    SELECT * FROM ple_private.save_authoring_draft_general_feedback(
        p_draft_question_uuid, p_expected_draft_question_edit_number, p_general_feedback)
$$;

CREATE FUNCTION ple_api.delete_draft_question(
    p_draft_question_uuid uuid,
    p_expected_draft_question_edit_number bigint
) RETURNS void LANGUAGE sql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private AS $$
    SELECT ple_private.delete_draft_question(p_draft_question_uuid, p_expected_draft_question_edit_number)
$$;

