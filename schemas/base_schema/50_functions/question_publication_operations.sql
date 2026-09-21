-- Functions, triggers, and views from question_publication_operations.sql.

SET LOCAL ROLE ple_private_owner;

-- A Draft source is mutable current authoring state.  The operation is a
-- compare-and-swap and accepts a no-op only at the exact current Edit Number
-- when all source facts are unchanged; it returns the committed Edit Number
-- for publication and never accepts inline source bytes.
CREATE FUNCTION ple_private.bind_draft_question_source(
    p_draft_question_uuid uuid, p_expected_draft_question_edit_number bigint, p_authoring_workspace_id uuid,
    p_backend text, p_question_format text, p_question_type text, p_webwork_pg_path text,
    p_source_object_id uuid, p_source_object_checksum text
) RETURNS bigint LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private AS $$
DECLARE
    v_current_draft_question_edit_number bigint;
    existing ple_private.draft_question_source_binding%ROWTYPE;
BEGIN
    IF p_expected_draft_question_edit_number IS NULL OR p_expected_draft_question_edit_number <= 0
       OR NOT ple_api.current_session_account_can_access_authoring_workspace(p_authoring_workspace_id) THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'authorized Draft Question Edit Number is required';
    END IF;
    IF NOT ple_private.question_backend_is_supported_for_production(
        p_backend::ple_data.question_backend
    ) THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Question Backend is unavailable for new production work';
    END IF;
    SELECT draft_question_edit_number INTO v_current_draft_question_edit_number FROM ple_private.draft_question
     WHERE draft_question_id = p_draft_question_uuid AND authoring_workspace_id = p_authoring_workspace_id
     FOR UPDATE;
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '23514', MESSAGE = 'Draft Question does not belong to workspace';
    END IF;
    IF v_current_draft_question_edit_number <> p_expected_draft_question_edit_number THEN
        RAISE EXCEPTION USING ERRCODE = '40001', MESSAGE = 'Draft Question Edit Number is stale';
    END IF;
    SELECT * INTO existing FROM ple_private.draft_question_source_binding
     WHERE draft_question_id = p_draft_question_uuid FOR UPDATE;
    IF FOUND AND existing.backend = p_backend::ple_data.question_backend
       AND existing.question_format = p_question_format::ple_data.question_format
       AND existing.question_type = p_question_type::ple_data.question_type
       AND existing.webwork_pg_path IS NOT DISTINCT FROM p_webwork_pg_path
       AND existing.source_object_record_id = p_source_object_id
       AND existing.source_object_checksum = p_source_object_checksum THEN
        RETURN v_current_draft_question_edit_number;
    END IF;
    INSERT INTO ple_private.draft_question_source_binding AS binding (
        draft_question_id, backend, question_format, question_type, webwork_pg_path,
        source_object_record_id, source_object_checksum, created_at, updated_at
    ) VALUES (
        p_draft_question_uuid, p_backend::ple_data.question_backend,
        p_question_format::ple_data.question_format, p_question_type::ple_data.question_type,
        p_webwork_pg_path,
        p_source_object_id, p_source_object_checksum, pg_catalog.clock_timestamp(), pg_catalog.clock_timestamp()
    ) ON CONFLICT (draft_question_id) DO UPDATE SET
        backend = EXCLUDED.backend, question_format = EXCLUDED.question_format,
        question_type = EXCLUDED.question_type,
        webwork_pg_path = EXCLUDED.webwork_pg_path,
        source_object_record_id = EXCLUDED.source_object_record_id,
        source_object_checksum = EXCLUDED.source_object_checksum,
        updated_at = EXCLUDED.updated_at;
    UPDATE ple_private.draft_question
       SET draft_question_edit_number = draft_question_edit_number + 1,
           updated_at = pg_catalog.clock_timestamp()
     WHERE draft_question_id = p_draft_question_uuid
     RETURNING draft_question_edit_number INTO v_current_draft_question_edit_number;
    RETURN v_current_draft_question_edit_number;
END
$$;




-- A later publication creates another immutable version of an existing stable
-- Question lineage.  Locking that lineage serializes its revision numbers;
-- availability remains lineage state and is not changed here.
-- ASVS 1.2.4, 2.2.1-2.2.3, 2.3.1-2.3.4, and 8.2.1-8.3.1: the trusted
-- database operation validates the complete aggregate, rechecks current
-- workspace and Question Owner authority, and commits it atomically.
CREATE FUNCTION ple_private.publish_question_revision(
    p_draft_question_uuid uuid, p_expected_draft_question_edit_number bigint, p_authoring_workspace_id uuid,
    p_published_question_id text, p_expected_parent_question_revision_number integer,
    p_target_object_id uuid, p_target_object_address jsonb,
    p_target_sha256 bytea, p_target_size_bytes bigint, p_target_media_type text,
    p_target_created_at_millis bigint, p_reason_for_edit text, p_publication_event_id uuid,
    p_hotspot_question_image jsonb
) RETURNS integer LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
DECLARE
    actor_id text;
    v_current_draft_question_edit_number bigint;
    v_next_question_revision_number integer;
    metadata ple_private.draft_question_metadata%ROWTYPE;
    binding ple_private.draft_question_source_binding%ROWTYPE;
    parent_binding ple_private.question_revision_source_binding%ROWTYPE;
    parent_revision ple_data.question_revision%ROWTYPE;
    source_record ple_private.object_record%ROWTYPE;
    expected_address jsonb;
    published_at timestamptz := clock_timestamp();
BEGIN
    IF p_expected_draft_question_edit_number IS NULL OR p_expected_draft_question_edit_number <= 0
       OR p_published_question_id IS NULL
       OR p_published_question_id !~ '^[0-9A-HJKMNP-TV-Z]{4}-[0-9A-HJKMNP-TV-Z]{4}$'
       OR substr(p_published_question_id, 6, 1) <> ple_private.crockford_checksum_character(
           substr(p_published_question_id, 1, 4) || substr(p_published_question_id, 7, 3)
       )
       OR p_expected_parent_question_revision_number IS NULL OR p_expected_parent_question_revision_number <= 0
       OR p_target_object_id IS NULL OR p_target_sha256 IS NULL OR octet_length(p_target_sha256) <> 32
       OR p_target_size_bytes IS NULL OR p_target_size_bytes < 0
       OR p_target_media_type IS NULL OR char_length(btrim(p_target_media_type)) NOT BETWEEN 1 AND 255
       OR p_target_created_at_millis IS NULL OR p_reason_for_edit IS NULL OR p_reason_for_edit <> btrim(p_reason_for_edit)
       OR char_length(p_reason_for_edit) NOT BETWEEN 1 AND 2000 OR p_reason_for_edit ~ '[[:cntrl:]]'
       OR p_publication_event_id IS NULL THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Question Revision Publication arguments are invalid';
    END IF;
    IF NOT ple_api.current_session_account_is_instructor()
       OR NOT ple_api.current_session_account_can_access_authoring_workspace(p_authoring_workspace_id) THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Question Revision Publication requires current Authoring Workspace access';
    END IF;
    actor_id := ple_api.current_session_account_id();
    SELECT draft_question_edit_number INTO v_current_draft_question_edit_number FROM ple_private.draft_question
     WHERE draft_question_id = p_draft_question_uuid AND authoring_workspace_id = p_authoring_workspace_id FOR UPDATE;
    IF NOT FOUND OR v_current_draft_question_edit_number <> p_expected_draft_question_edit_number THEN
        RAISE EXCEPTION USING ERRCODE = 'PQR01',
            MESSAGE = 'Question Revision Publication Draft Question Edit Number is stale or not in its workspace';
    END IF;
    IF EXISTS (
        SELECT 1 FROM ple_private.draft_question_fork_source AS fork
         WHERE fork.draft_question_id = p_draft_question_uuid
    ) THEN
        RAISE EXCEPTION USING ERRCODE = '55000',
            MESSAGE = 'Question Fork Draft must publish through its reserved new lineage';
    END IF;
    PERFORM 1 FROM ple_data.published_question WHERE published_question_id = p_published_question_id FOR UPDATE;
    IF NOT FOUND OR NOT EXISTS (
        SELECT 1 FROM ple_data.question_current_owner
         WHERE published_question_id = p_published_question_id AND owner_account_id = actor_id
    ) THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Question Revision Publication requires current Question Owner authority';
    END IF;
    SELECT COALESCE(max(revision_number), 0) + 1 INTO v_next_question_revision_number
      FROM ple_data.question_revision WHERE published_question_id = p_published_question_id;
    IF v_next_question_revision_number <= 1 OR v_next_question_revision_number - 1 <> p_expected_parent_question_revision_number THEN
        RAISE EXCEPTION USING ERRCODE = 'PQR01',
            MESSAGE = 'Question Revision Publication parent Revision is stale';
    END IF;
    SELECT * INTO STRICT metadata FROM ple_private.draft_question_metadata
     WHERE draft_question_id = p_draft_question_uuid FOR UPDATE;
    SELECT * INTO STRICT binding FROM ple_private.draft_question_source_binding
     WHERE draft_question_id = p_draft_question_uuid FOR UPDATE;
    IF NOT ple_private.question_backend_is_supported_for_production(binding.backend) THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Question Backend is unavailable for new production work';
    END IF;
    SELECT * INTO STRICT parent_binding FROM ple_private.question_revision_source_binding
     WHERE published_question_id = p_published_question_id
       AND revision_number = p_expected_parent_question_revision_number
     FOR UPDATE;
    SELECT * INTO STRICT parent_revision FROM ple_data.question_revision
     WHERE published_question_id = p_published_question_id
       AND revision_number = p_expected_parent_question_revision_number;
    SELECT * INTO STRICT source_record FROM ple_private.object_record
     WHERE object_record_id = binding.source_object_record_id;
    -- A Question Source is the immutable, backend-owned package that carries
    -- content, answer, grading, backend interaction feedback, and asset
    -- references. PLE-managed general feedback is separate immutable
    -- Question Revision content. Title, description, and other search
    -- metadata are lineage state in published_question_metadata, so they
    -- cannot justify a successor Question Revision. The trusted publication
    -- boundary compares exact authoritative content rather than trusting a
    -- browser-side edit label.
    -- ASVS 2.2.1-2.2.3 and 2.3.1: enforce the revision business rule after
    -- locking the Draft and its immediate parent, before any successor facts.
    IF binding.backend = parent_binding.backend
       AND binding.question_format = parent_binding.question_format
       AND binding.question_type = parent_revision.question_type
       AND binding.webwork_pg_path IS NOT DISTINCT FROM parent_binding.webwork_pg_path
       AND binding.source_object_checksum = parent_binding.source_object_checksum
       AND metadata.general_feedback IS NOT DISTINCT FROM parent_revision.general_feedback THEN
        RAISE EXCEPTION USING ERRCODE = 'PQR01',
            MESSAGE = 'Question Revision Publication content does not differ from its parent Revision';
    END IF;
    expected_address := jsonb_build_object('kind', 'questionSource',
        'publishedQuestionRevisionTuple', jsonb_build_object('publishedQuestionId', p_published_question_id,
            'revisionNumber', v_next_question_revision_number), 'objectId', p_target_object_id);
    IF p_target_object_address IS DISTINCT FROM expected_address
       OR p_target_sha256 IS DISTINCT FROM source_record.sha256
       OR encode(p_target_sha256, 'hex') IS DISTINCT FROM binding.source_object_checksum
       OR p_target_size_bytes IS DISTINCT FROM source_record.size_bytes
       OR p_target_media_type IS DISTINCT FROM source_record.media_type THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Question Revision Publication target must preserve the exact Draft Question Source bytes';
    END IF;
    INSERT INTO ple_data.question_revision(
        published_question_id, revision_number, backend, question_type, general_feedback, published_at
    ) VALUES (
        p_published_question_id, v_next_question_revision_number, binding.backend, binding.question_type,
        metadata.general_feedback, published_at
    );
    INSERT INTO ple_private.object_record(
        object_record_id, object_address, object_storage_area, object_data_class, sha256, size_bytes, media_type, created_at
    ) VALUES (p_target_object_id, expected_address, 'private-content', 'question-source',
        p_target_sha256, p_target_size_bytes, p_target_media_type,
        to_timestamp(p_target_created_at_millis::double precision / 1000.0));
    INSERT INTO ple_private.question_revision_source_binding(
        published_question_id, revision_number, backend, question_format, webwork_pg_path,
        source_object_record_id, source_object_checksum, created_at
    ) VALUES (p_published_question_id, v_next_question_revision_number, binding.backend, binding.question_format,
        binding.webwork_pg_path, p_target_object_id,
        encode(p_target_sha256, 'hex'), published_at);
    INSERT INTO ple_data.question_revision_acceptance(
        published_question_id, revision_number, parent_revision_number, editor_account_id,
        accepted_by_account_id, accepted_at, reason_for_edit
    ) VALUES (p_published_question_id, v_next_question_revision_number, v_next_question_revision_number - 1, actor_id, actor_id,
        published_at, p_reason_for_edit);
    INSERT INTO ple_data.question_revision_authorship(
        published_question_id, revision_number, author_position, author_display_name, author_account_id
    ) SELECT p_published_question_id, v_next_question_revision_number, author.author_position,
        author.author_display_name, author.author_account_id
        -- Moderate edits retain the parent revision's immutable credit. A
        -- fork is the separate path that creates a new authorship record.
        FROM ple_data.question_revision_authorship AS author
       WHERE author.published_question_id = p_published_question_id
         AND author.revision_number = p_expected_parent_question_revision_number;
    INSERT INTO ple_data.question_revision_license(published_question_id, revision_number, spdx_expression)
    SELECT p_published_question_id, v_next_question_revision_number, license.spdx_expression
      -- The compatible CC license is immutable lineage evidence as well.
      FROM ple_data.question_revision_license AS license
     WHERE license.published_question_id = p_published_question_id
       AND license.revision_number = p_expected_parent_question_revision_number;
    UPDATE ple_data.published_question_metadata
       SET question_title = metadata.question_title,
           question_description = metadata.question_description,
           language = metadata.language,
           updated_at = published_at
     WHERE published_question_id = p_published_question_id;
    INSERT INTO ple_data.question_publication_event(event_id, published_question_id, revision_number, actor_account_id, occurred_at)
    VALUES (p_publication_event_id, p_published_question_id, v_next_question_revision_number, actor_id, published_at);
    PERFORM ple_private.bind_draft_question_image_publication(p_draft_question_uuid, p_authoring_workspace_id,
        p_published_question_id, v_next_question_revision_number, binding.backend, binding.question_type, p_hotspot_question_image, published_at);
    RETURN v_next_question_revision_number;
END
$$;

SET LOCAL ROLE ple_api_owner;





-- Authoring owns these API predicates because only authoring owns the
-- workspace relations.  The API owner receives no table privilege or RLS
-- bypass; it delegates to the narrowly scoped private-owner predicates.
CREATE FUNCTION ple_api.current_session_account_is_authoring_workspace_owner(
    p_authoring_workspace_id uuid
) RETURNS boolean LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private AS $$
    SELECT ple_private.current_session_is_authoring_workspace_owner(p_authoring_workspace_id)
$$;

CREATE FUNCTION ple_api.current_session_account_can_access_authoring_workspace(
    p_authoring_workspace_id uuid
) RETURNS boolean LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private AS $$
    SELECT ple_private.current_session_can_access_authoring_workspace(p_authoring_workspace_id)
$$;

CREATE FUNCTION ple_api.bind_draft_question_source(
    p_draft_question_uuid uuid, p_expected_draft_question_edit_number bigint, p_authoring_workspace_id uuid,
    p_backend text, p_question_format text, p_question_type text, p_webwork_pg_path text,
    p_source_object_id uuid, p_source_object_checksum text
) RETURNS bigint LANGUAGE sql SECURITY DEFINER
SET search_path = pg_catalog, ple_private, ple_api AS $$
    SELECT ple_private.bind_draft_question_source(
        p_draft_question_uuid, p_expected_draft_question_edit_number, p_authoring_workspace_id, p_backend,
        p_question_format, p_question_type, p_webwork_pg_path, p_source_object_id,
        p_source_object_checksum)
$$;

CREATE FUNCTION ple_api.publish_question_revision(
    p_draft_question_uuid uuid, p_expected_draft_question_edit_number bigint, p_authoring_workspace_id uuid,
    p_published_question_id text, p_expected_parent_question_revision_number integer,
    p_target_object_id uuid, p_target_object_address jsonb,
    p_target_sha256 bytea, p_target_size_bytes bigint, p_target_media_type text,
    p_target_created_at_millis bigint, p_reason_for_edit text, p_publication_event_id uuid,
    p_hotspot_question_image jsonb
) RETURNS integer LANGUAGE sql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private AS $$
    SELECT ple_private.publish_question_revision(
        p_draft_question_uuid, p_expected_draft_question_edit_number, p_authoring_workspace_id, p_published_question_id,
        p_expected_parent_question_revision_number, p_target_object_id, p_target_object_address,
        p_target_sha256, p_target_size_bytes,
        p_target_media_type, p_target_created_at_millis, p_reason_for_edit, p_publication_event_id,
        p_hotspot_question_image)
$$;

SET LOCAL ROLE ple_private_owner;




-- Publication reads one current Draft source record, then the server copies its
-- verified bytes to the typed immutable Question Revision address before this
-- transaction records the complete Question aggregate.
CREATE FUNCTION ple_private.load_draft_question_publication_source(
    p_draft_question_uuid uuid, p_expected_draft_question_edit_number bigint, p_authoring_workspace_id uuid
) RETURNS TABLE (
    object_record_id uuid, object_address jsonb, sha256 bytea, size_bytes bigint,
    media_type text, created_at_millis bigint
) LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private AS $$
DECLARE v_current_draft_question_edit_number bigint; row_count bigint;
BEGIN
    IF p_draft_question_uuid IS NULL OR p_authoring_workspace_id IS NULL
       OR p_expected_draft_question_edit_number IS NULL OR p_expected_draft_question_edit_number <= 0 THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Draft Question Publication Source arguments are invalid';
    END IF;
    IF NOT ple_api.current_session_account_is_instructor()
       OR NOT ple_api.current_session_account_can_access_authoring_workspace(p_authoring_workspace_id) THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Draft Question Publication Source requires current Authoring Workspace access';
    END IF;
    SELECT draft_question_edit_number INTO v_current_draft_question_edit_number FROM ple_private.draft_question
     WHERE draft_question_id = p_draft_question_uuid AND authoring_workspace_id = p_authoring_workspace_id;
    IF NOT FOUND OR v_current_draft_question_edit_number <> p_expected_draft_question_edit_number THEN
        RAISE EXCEPTION USING ERRCODE = '40001',
            MESSAGE = 'Draft Question Publication Source Edit Number is stale or not in its workspace';
    END IF;
    RETURN QUERY SELECT record.object_record_id, record.object_address, record.sha256,
        record.size_bytes, record.media_type,
        round(extract(epoch FROM record.created_at) * 1000)::bigint
      FROM ple_private.draft_question_source_binding AS binding
      JOIN ple_private.object_record AS record ON record.object_record_id = binding.source_object_record_id
     WHERE binding.draft_question_id = p_draft_question_uuid
       AND binding.source_object_checksum = encode(record.sha256, 'hex')
       AND record.object_storage_area = 'private-content'
       AND record.object_data_class = 'authoring-content'
       AND record.object_address = jsonb_build_object('kind', 'workspaceQuestionSource',
           'workspaceId', p_authoring_workspace_id, 'objectId', record.object_record_id);
    GET DIAGNOSTICS row_count = ROW_COUNT;
    IF row_count <> 1 THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Draft Question Publication Source Binding is incomplete';
    END IF;
END
$$;

CREATE FUNCTION ple_private.publish_new_question_lineage(
    p_draft_question_uuid uuid, p_expected_draft_question_edit_number bigint, p_authoring_workspace_id uuid,
    p_published_question_id text, p_target_object_id uuid, p_target_object_address jsonb,
    p_target_sha256 bytea, p_target_size_bytes bigint, p_target_media_type text,
    p_target_created_at_millis bigint, p_authorship jsonb,
    p_initial_shared_tags text[], p_discipline_uuid uuid, p_subject_uuid uuid,
    p_topic_uuid uuid, p_subtopic_uuid uuid, p_license text,
    p_reason_for_edit text, p_ownership_event_id uuid, p_publication_event_id uuid,
    p_availability_event_id uuid, p_hotspot_question_image jsonb
) RETURNS void LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
DECLARE
    actor_id text; v_current_draft_question_edit_number bigint; metadata ple_private.draft_question_metadata%ROWTYPE;
    binding ple_private.draft_question_source_binding%ROWTYPE;
    source_record ple_private.object_record%ROWTYPE; expected_address jsonb;
    published_at timestamptz := clock_timestamp(); author_count integer; valid_count integer;
    recorded_source_question_id text;
    recorded_source_revision_number integer; recorded_source_license text;
BEGIN
    IF p_expected_draft_question_edit_number IS NULL OR p_expected_draft_question_edit_number <= 0
       OR p_published_question_id IS NULL
       OR p_published_question_id !~ '^[0-9A-HJKMNP-TV-Z]{4}-[0-9A-HJKMNP-TV-Z]{4}$'
       OR substr(p_published_question_id, 6, 1) <> ple_private.crockford_checksum_character(
           substr(p_published_question_id, 1, 4) || substr(p_published_question_id, 7, 3)
       )
       OR p_target_object_id IS NULL OR p_target_sha256 IS NULL OR octet_length(p_target_sha256) <> 32
       OR p_target_size_bytes IS NULL OR p_target_size_bytes < 0
       OR p_target_media_type IS NULL OR char_length(btrim(p_target_media_type)) NOT BETWEEN 1 AND 255
       OR p_target_created_at_millis IS NULL OR p_authorship IS NULL
       OR jsonb_typeof(p_authorship) <> 'array' OR jsonb_array_length(p_authorship) NOT BETWEEN 1 AND 16
       OR p_discipline_uuid IS NULL OR p_subject_uuid IS NULL
       OR (p_subtopic_uuid IS NOT NULL AND p_topic_uuid IS NULL)
       OR NOT ple_data.question_metadata_tags_are_valid(p_initial_shared_tags)
       OR p_license NOT IN ('CC0-1.0', 'CC-BY-4.0', 'CC-BY-SA-4.0')
       OR p_reason_for_edit IS NULL OR p_reason_for_edit <> btrim(p_reason_for_edit)
       OR char_length(p_reason_for_edit) NOT BETWEEN 1 AND 2000 OR p_reason_for_edit ~ '[[:cntrl:]]'
       OR p_ownership_event_id IS NULL OR p_publication_event_id IS NULL OR p_availability_event_id IS NULL THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Question Publication arguments violate the new-lineage contract';
    END IF;
    SELECT count(DISTINCT author.value #>> '{}'), count(*) INTO author_count, valid_count
      FROM jsonb_array_elements(p_authorship) AS author(value)
     WHERE jsonb_typeof(author.value) = 'string'
       AND author.value #>> '{}' = btrim(author.value #>> '{}')
       AND char_length(author.value #>> '{}') BETWEEN 1 AND 120
       AND author.value #>> '{}' !~ '[[:cntrl:]]';
    IF author_count <> jsonb_array_length(p_authorship) OR valid_count <> jsonb_array_length(p_authorship) THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Question Publication requires distinct reviewed Question Authors';
    END IF;
    IF NOT ple_api.current_session_account_is_instructor()
       OR NOT ple_api.current_session_account_can_access_authoring_workspace(p_authoring_workspace_id) THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Question Publication requires current Authoring Workspace access';
    END IF;
    actor_id := ple_api.current_session_account_id();
    SELECT draft_question_edit_number INTO v_current_draft_question_edit_number FROM ple_private.draft_question
     WHERE draft_question_id = p_draft_question_uuid AND authoring_workspace_id = p_authoring_workspace_id FOR UPDATE;
    IF NOT FOUND OR v_current_draft_question_edit_number <> p_expected_draft_question_edit_number THEN
        RAISE EXCEPTION USING ERRCODE = '40001',
            MESSAGE = 'Question Publication Draft Question Edit Number is stale or not in its workspace';
    END IF;
    SELECT fork.source_question_id, fork.source_revision_number
      INTO recorded_source_question_id,
           recorded_source_revision_number
      FROM ple_private.draft_question_fork_source AS fork
     WHERE fork.draft_question_id = p_draft_question_uuid
     FOR UPDATE;
    -- ASVS 2.2.2, 2.2.3, and 2.3.3: derive the exact source Revision
    -- license from the immutable server-owned fork pin and preserve that exact
    -- license before this transaction writes publication state.
    IF recorded_source_question_id IS NOT NULL THEN
        SELECT license.spdx_expression INTO STRICT recorded_source_license
          FROM ple_data.question_revision_license AS license
         WHERE license.published_question_id = recorded_source_question_id
           AND license.revision_number = recorded_source_revision_number;
        IF p_license <> recorded_source_license THEN
            RAISE EXCEPTION USING ERRCODE = '23514',
                MESSAGE = 'Question Fork Publication must preserve its exact source Revision license';
        END IF;
    END IF;
    SELECT * INTO STRICT metadata FROM ple_private.draft_question_metadata
     WHERE draft_question_id = p_draft_question_uuid FOR UPDATE;
    SELECT * INTO STRICT binding FROM ple_private.draft_question_source_binding
     WHERE draft_question_id = p_draft_question_uuid FOR UPDATE;
    IF NOT ple_private.question_backend_is_supported_for_production(binding.backend) THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Question Backend is unavailable for new production work';
    END IF;
    SELECT * INTO STRICT source_record FROM ple_private.object_record
     WHERE object_record_id = binding.source_object_record_id;
    expected_address := jsonb_build_object('kind', 'questionSource',
        'publishedQuestionRevisionTuple', jsonb_build_object('publishedQuestionId', p_published_question_id, 'revisionNumber', 1),
        'objectId', p_target_object_id);
    IF p_target_object_address IS DISTINCT FROM expected_address
       OR p_target_sha256 IS DISTINCT FROM source_record.sha256
       OR encode(p_target_sha256, 'hex') IS DISTINCT FROM binding.source_object_checksum
       OR p_target_size_bytes IS DISTINCT FROM source_record.size_bytes
       OR p_target_media_type IS DISTINCT FROM source_record.media_type THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Question Publication target must preserve the exact Draft Question Source bytes';
    END IF;
    INSERT INTO ple_data.published_question(published_question_id, created_at) VALUES (p_published_question_id, published_at);
    PERFORM ple_private.require_active_content_discipline(p_discipline_uuid);
    INSERT INTO ple_data.published_question_metadata(
        published_question_id, question_title, question_description, language, tags,
        content_discipline_id, content_subject_id, content_topic_id, content_subtopic_id, created_at, updated_at
    ) VALUES (p_published_question_id, metadata.question_title, metadata.question_description, metadata.language,
        p_initial_shared_tags, p_discipline_uuid, p_subject_uuid, p_topic_uuid, p_subtopic_uuid,
        published_at, published_at);
    INSERT INTO ple_data.question_revision(
        published_question_id, revision_number, backend, question_type, general_feedback, published_at
    ) VALUES (
        p_published_question_id, 1, binding.backend, binding.question_type,
        metadata.general_feedback, published_at
    );
    INSERT INTO ple_private.object_record(
        object_record_id, object_address, object_storage_area, object_data_class, sha256, size_bytes, media_type, created_at
    ) VALUES (p_target_object_id, expected_address, 'private-content', 'question-source',
        p_target_sha256, p_target_size_bytes, p_target_media_type,
        to_timestamp(p_target_created_at_millis::double precision / 1000.0));
    INSERT INTO ple_private.question_revision_source_binding(
        published_question_id, revision_number, backend, question_format, webwork_pg_path,
        source_object_record_id, source_object_checksum, created_at
    ) VALUES (p_published_question_id, 1, binding.backend, binding.question_format, binding.webwork_pg_path,
        p_target_object_id, encode(p_target_sha256, 'hex'), published_at);
    INSERT INTO ple_data.question_revision_acceptance(
        published_question_id, revision_number, parent_revision_number, editor_account_id,
        accepted_by_account_id, accepted_at, reason_for_edit
    ) VALUES (p_published_question_id, 1, NULL, actor_id, actor_id, published_at, p_reason_for_edit);
    INSERT INTO ple_data.question_revision_authorship(
        published_question_id, revision_number, author_position, author_display_name, author_account_id
    ) SELECT p_published_question_id, 1, author.ordinality::integer, author.value #>> '{}', NULL::uuid
        FROM jsonb_array_elements(p_authorship) WITH ORDINALITY AS author(value, ordinality);
    INSERT INTO ple_data.question_revision_license(published_question_id, revision_number, spdx_expression)
    VALUES (p_published_question_id, 1, p_license::ple_data.license_spdx);
    INSERT INTO ple_data.question_ownership_event(
        question_ownership_event_id, published_question_id, owner_account_id, recorded_by_account_id, event_kind, occurred_at
    ) VALUES (p_ownership_event_id, p_published_question_id, actor_id, actor_id, 'initial', published_at);
    INSERT INTO ple_data.question_fork_source(
        forked_published_question_id, source_question_id, source_revision_number, recorded_at
    ) SELECT p_published_question_id, source_question_id, source_revision_number, published_at
      FROM ple_private.draft_question_fork_source WHERE draft_question_id = p_draft_question_uuid;
    INSERT INTO ple_data.question_publication_event(event_id, published_question_id, revision_number, actor_account_id, occurred_at)
    VALUES (p_publication_event_id, p_published_question_id, 1, actor_id, published_at);
    INSERT INTO ple_data.question_availability_event(
        event_id, published_question_id, actor_account_id, availability, edit_number, reason, occurred_at
    ) VALUES (p_availability_event_id, p_published_question_id, actor_id, 'available'::ple_data.question_availability, 1, NULL, published_at);
    PERFORM ple_private.bind_draft_question_image_publication(p_draft_question_uuid, p_authoring_workspace_id,
        p_published_question_id, 1, binding.backend, binding.question_type, p_hotspot_question_image, published_at);
END
$$;

SET LOCAL ROLE ple_api_owner;

CREATE FUNCTION ple_api.load_draft_question_publication_source(
    p_draft_question_uuid uuid, p_expected_draft_question_edit_number bigint, p_authoring_workspace_id uuid
) RETURNS TABLE (object_record_id uuid, object_address jsonb, sha256 bytea, size_bytes bigint,
    media_type text, created_at_millis bigint)
LANGUAGE sql SECURITY DEFINER SET search_path = pg_catalog, ple_api, ple_private AS $$
    SELECT * FROM ple_private.load_draft_question_publication_source(
        p_draft_question_uuid, p_expected_draft_question_edit_number, p_authoring_workspace_id)
$$;

CREATE FUNCTION ple_api.publish_new_question_lineage(
    p_draft_question_uuid uuid, p_expected_draft_question_edit_number bigint, p_authoring_workspace_id uuid,
    p_published_question_id text, p_target_object_id uuid, p_target_object_address jsonb,
    p_target_sha256 bytea, p_target_size_bytes bigint, p_target_media_type text,
    p_target_created_at_millis bigint, p_authorship jsonb,
    p_initial_shared_tags text[], p_discipline_uuid uuid, p_subject_uuid uuid,
    p_topic_uuid uuid, p_subtopic_uuid uuid, p_license text,
    p_reason_for_edit text, p_ownership_event_id uuid, p_publication_event_id uuid,
    p_availability_event_id uuid, p_hotspot_question_image jsonb
) RETURNS void LANGUAGE sql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private AS $$
    SELECT ple_private.publish_new_question_lineage(p_draft_question_uuid, p_expected_draft_question_edit_number,
        p_authoring_workspace_id, p_published_question_id, p_target_object_id, p_target_object_address, p_target_sha256,
        p_target_size_bytes, p_target_media_type, p_target_created_at_millis, p_authorship,
        p_initial_shared_tags, p_discipline_uuid, p_subject_uuid,
        p_topic_uuid, p_subtopic_uuid, p_license,
        p_reason_for_edit, p_ownership_event_id, p_publication_event_id, p_availability_event_id,
        p_hotspot_question_image)
$$;
