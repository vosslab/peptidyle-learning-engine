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
    current_availability text;
    metadata ple_private.draft_question_metadata%ROWTYPE;
    binding ple_private.draft_question_source_binding%ROWTYPE;
    parent_binding ple_private.question_revision_source_binding%ROWTYPE;
    parent_revision ple_data.question_revision%ROWTYPE;
    parent_metadata ple_data.question_revision_metadata%ROWTYPE;
    source_record ple_private.object_record%ROWTYPE;
    expected_address jsonb;
    v_question_image_unchanged boolean;
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
    IF NOT (ple_api.current_session_account_is_instructor()
            OR ple_api.current_session_account_is_sysadmin())
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
        SELECT 1 FROM ple_private.draft_question AS draft
         WHERE draft.draft_question_id = p_draft_question_uuid
           AND draft.parent_published_question_id IS NOT NULL
    ) THEN
        RAISE EXCEPTION USING ERRCODE = '55000',
            MESSAGE = 'Question Fork Draft must publish through its reserved new lineage';
    END IF;
    SELECT availability::text INTO current_availability
      FROM ple_data.published_question
     WHERE published_question_id = p_published_question_id
     FOR UPDATE;
    IF NOT FOUND OR current_availability <> 'available'
       OR (NOT ple_api.current_session_account_is_sysadmin() AND NOT EXISTS (
        SELECT 1 FROM ple_data.question_current_owner
         WHERE published_question_id = p_published_question_id AND owner_account_id = actor_id
       )) THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Question Revision Publication requires an available Question Owner or Sysadmin target';
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
    IF binding.question_type IS NULL THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Question Publication requires a source-derived Question Type';
    END IF;
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
    SELECT * INTO STRICT parent_metadata FROM ple_data.question_revision_metadata
     WHERE published_question_id = p_published_question_id
       AND revision_number = p_expected_parent_question_revision_number
     FOR UPDATE;
    SELECT * INTO STRICT source_record FROM ple_private.object_record
     WHERE object_record_id = binding.source_object_record_id;
    -- A Question Source is the immutable, backend-owned package that carries
    -- content, answer, grading, backend interaction feedback, and asset
    -- references. Changing those bytes changes source_object_checksum.
    -- PLE-managed general feedback, Hint, and Worked Solution text are
    -- separate immutable Question Revision content. A native HOTSPOT Question
    -- Image Asset is the draft-owned raster. A different asset, checksum, or
    -- dimension differs from the parent question_image_publication. Both sides
    -- absent stay equal. Metadata-only corrections update the current record
    -- without a successor. A content Revision takes editable Title, Description,
    -- Tags, Language, and support from this saved Draft. Type uses the locked
    -- binding: Native is source-derived, while WebWork Type is manually saved.
    -- Classification and Bloom remain from the exact parent because this path does not edit them.
    -- The trusted boundary compares exact content rather than trusting a browser label.
    -- ASVS 2.2.1-2.2.3 and 2.3.1: enforce the revision business rule after
    -- locking the Draft, its image, and its immediate parent, before any
    -- successor facts. ASVS 8.2.2: compare the draft-owned raster checksum.
    PERFORM 1 FROM ple_private.draft_question_image AS draft_image
     WHERE draft_image.draft_question_id = p_draft_question_uuid
       AND draft_image.authoring_workspace_id = p_authoring_workspace_id
     FOR UPDATE;
    PERFORM 1 FROM ple_private.question_image_publication AS parent_image
     WHERE parent_image.published_question_id = p_published_question_id
       AND parent_image.revision_number = p_expected_parent_question_revision_number
     FOR UPDATE;
    v_question_image_unchanged := CASE
        WHEN binding.backend IS DISTINCT FROM 'ple'::ple_data.question_backend
          OR binding.question_type IS DISTINCT FROM 'hotspot'::ple_data.question_type THEN
            p_hotspot_question_image IS NULL
            AND NOT EXISTS (
                SELECT 1 FROM ple_private.question_image_publication AS parent_image
                 WHERE parent_image.published_question_id = p_published_question_id
                   AND parent_image.revision_number = p_expected_parent_question_revision_number)
        WHEN p_hotspot_question_image IS NULL THEN
            NOT EXISTS (
                SELECT 1 FROM ple_private.question_image_publication AS parent_image
                 WHERE parent_image.published_question_id = p_published_question_id
                   AND parent_image.revision_number = p_expected_parent_question_revision_number)
        WHEN jsonb_typeof(p_hotspot_question_image) IS DISTINCT FROM 'object'
          OR COALESCE(p_hotspot_question_image->>'checksum', '') !~ '^[0-9a-f]{64}$'
          OR COALESCE(p_hotspot_question_image->>'questionImageAssetId', '')
             !~ '^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$'
          OR COALESCE(p_hotspot_question_image->>'intrinsicWidth', '') !~ '^[1-9][0-9]{0,6}$'
          OR COALESCE(p_hotspot_question_image->>'intrinsicHeight', '') !~ '^[1-9][0-9]{0,6}$' THEN
            false
        ELSE
            EXISTS (
                SELECT 1
                  FROM ple_private.draft_question_image AS draft_image
                  JOIN ple_private.object_record AS draft_image_record
                    ON draft_image_record.object_record_id = draft_image.source_object_record_id
                  JOIN ple_private.question_image_publication AS parent_image
                    ON parent_image.published_question_id = p_published_question_id
                   AND parent_image.revision_number = p_expected_parent_question_revision_number
                   AND parent_image.question_image_asset_id = draft_image.question_image_asset_id
                 WHERE draft_image.draft_question_id = p_draft_question_uuid
                   AND draft_image.authoring_workspace_id = p_authoring_workspace_id
                   AND draft_image.question_image_asset_id
                       = (p_hotspot_question_image->>'questionImageAssetId')::uuid
                   AND encode(draft_image_record.sha256, 'hex') = p_hotspot_question_image->>'checksum'
                   AND encode(parent_image.source_object_checksum, 'hex')
                       = p_hotspot_question_image->>'checksum'
                   AND draft_image.intrinsic_width = (p_hotspot_question_image->>'intrinsicWidth')::integer
                   AND draft_image.intrinsic_height = (p_hotspot_question_image->>'intrinsicHeight')::integer
                   AND parent_image.intrinsic_width = draft_image.intrinsic_width
                   AND parent_image.intrinsic_height = draft_image.intrinsic_height)
            AND NOT EXISTS (
                SELECT 1 FROM ple_private.question_image_publication AS parent_image
                 WHERE parent_image.published_question_id = p_published_question_id
                   AND parent_image.revision_number = p_expected_parent_question_revision_number
                   AND parent_image.question_image_asset_id
                       IS DISTINCT FROM (p_hotspot_question_image->>'questionImageAssetId')::uuid)
    END;
    IF binding.backend = parent_binding.backend
       AND binding.question_format = parent_binding.question_format
       AND binding.webwork_pg_path IS NOT DISTINCT FROM parent_binding.webwork_pg_path
       AND binding.source_object_checksum = parent_binding.source_object_checksum
       AND metadata.general_feedback IS NOT DISTINCT FROM parent_revision.general_feedback
       AND metadata.hint IS NOT DISTINCT FROM parent_revision.hint
       AND metadata.worked_solution IS NOT DISTINCT FROM parent_revision.worked_solution
       AND v_question_image_unchanged THEN
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
        published_question_id, revision_number, backend, general_feedback, hint, worked_solution, published_at
    ) VALUES (
        p_published_question_id, v_next_question_revision_number, binding.backend,
        metadata.general_feedback, metadata.hint, metadata.worked_solution, published_at
    );
    INSERT INTO ple_private.object_record(
        object_record_id, object_address, object_storage_area, object_data_class, sha256, size_bytes, media_type, created_at
    ) VALUES (p_target_object_id, expected_address, 'private-content', 'question-source',
        p_target_sha256, p_target_size_bytes, p_target_media_type,
        to_timestamp(p_target_created_at_millis::double precision / 1000.0));
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
    -- The saved Draft metadata owns the successor's editable license and citation values.
    INSERT INTO ple_data.question_revision_license(published_question_id, revision_number, spdx_expression)
    VALUES (p_published_question_id, v_next_question_revision_number, metadata.question_license);
    INSERT INTO ple_data.question_revision_metadata(
        published_question_id, revision_number, question_title, question_description, language, question_type,
        metadata_edit_number, tags, content_discipline_id, content_subject_id,
        content_topic_id, content_subtopic_id, bloom_cognitive_process, bloom_knowledge_dimension,
        created_at, updated_at
    ) VALUES (
        p_published_question_id, v_next_question_revision_number,
        metadata.question_title, metadata.question_description, metadata.language,
        binding.question_type,
        1, metadata.tags, parent_metadata.content_discipline_id, parent_metadata.content_subject_id,
        parent_metadata.content_topic_id, parent_metadata.content_subtopic_id,
        parent_metadata.bloom_cognitive_process, parent_metadata.bloom_knowledge_dimension,
        published_at, published_at
    );
    INSERT INTO ple_private.question_revision_source_binding(
        published_question_id, revision_number, backend, native_question_type, question_format, webwork_pg_path,
        source_object_record_id, source_object_checksum, created_at
    ) VALUES (p_published_question_id, v_next_question_revision_number, binding.backend,
        CASE WHEN binding.backend = 'ple' THEN binding.question_type END, binding.question_format,
        binding.webwork_pg_path, p_target_object_id,
        encode(p_target_sha256, 'hex'), published_at);
    INSERT INTO ple_data.question_revision_citation(
        published_question_id, revision_number, citation_text, created_at, updated_at
    ) SELECT p_published_question_id, v_next_question_revision_number,
             metadata.citation_text, published_at, published_at
       WHERE metadata.citation_text IS NOT NULL;
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
    media_type text, created_at_millis bigint, reserved_published_question_id text
) LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private AS $$
DECLARE v_current_draft_question_edit_number bigint; row_count bigint;
BEGIN
    IF p_draft_question_uuid IS NULL OR p_authoring_workspace_id IS NULL
       OR p_expected_draft_question_edit_number IS NULL OR p_expected_draft_question_edit_number <= 0 THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Draft Question Publication Source arguments are invalid';
    END IF;
    IF NOT (ple_api.current_session_account_is_instructor()
            OR ple_api.current_session_account_is_sysadmin())
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
    IF EXISTS (
        SELECT 1 FROM ple_private.draft_question AS draft
         WHERE draft.draft_question_id = p_draft_question_uuid
           AND draft.parent_published_question_id IS NOT NULL
           AND NOT EXISTS (
               SELECT 1 FROM ple_private.draft_question_creation_receipt AS receipt
                WHERE receipt.draft_question_id = draft.draft_question_id)
    ) THEN
        RAISE EXCEPTION USING ERRCODE = '55000',
            MESSAGE = 'Published Question Fork Draft cannot be published again';
    END IF;
    RETURN QUERY SELECT record.object_record_id, record.object_address, record.sha256,
        record.size_bytes, record.media_type,
        round(extract(epoch FROM record.created_at) * 1000)::bigint,
        draft.public_id_reservation_id AS reserved_published_question_id
      FROM ple_private.draft_question AS draft
      JOIN ple_private.draft_question_source_binding AS binding
        ON binding.draft_question_id = draft.draft_question_id
      JOIN ple_private.object_record AS record ON record.object_record_id = binding.source_object_record_id
     WHERE draft.draft_question_id = p_draft_question_uuid
       AND draft.authoring_workspace_id = p_authoring_workspace_id
       AND binding.draft_question_id = draft.draft_question_id
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
    actor_id text; v_current_draft_question_edit_number bigint;
    lineage_owner_id ple_data.account_id;
    v_parent_published_question_id ple_data.question_family_id;
    v_parent_revision_number integer;
    v_reserved_published_question_id ple_data.question_family_id;
    metadata ple_private.draft_question_metadata%ROWTYPE;
    binding ple_private.draft_question_source_binding%ROWTYPE;
    source_record ple_private.object_record%ROWTYPE; expected_address jsonb;
    published_at timestamptz := clock_timestamp(); author_count integer; valid_count integer;
    final_author_position integer;
    v_initial_shared_tags text[];
    v_discipline_uuid uuid; v_subject_uuid uuid; v_topic_uuid uuid; v_subtopic_uuid uuid;
    v_license text;
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
    SELECT draft.draft_question_edit_number, draft.parent_published_question_id,
           draft.parent_revision_number, draft.public_id_reservation_id
      INTO v_current_draft_question_edit_number, v_parent_published_question_id,
           v_parent_revision_number, v_reserved_published_question_id
      FROM ple_private.draft_question AS draft
     WHERE draft.draft_question_id = p_draft_question_uuid
       AND draft.authoring_workspace_id = p_authoring_workspace_id
     FOR UPDATE;
    IF NOT FOUND OR v_current_draft_question_edit_number <> p_expected_draft_question_edit_number THEN
        RAISE EXCEPTION USING ERRCODE = '40001',
            MESSAGE = 'Question Publication Draft Question Edit Number is stale or not in its workspace';
    END IF;
    IF (v_parent_published_question_id IS NULL)
            IS DISTINCT FROM (v_reserved_published_question_id IS NULL)
       OR (v_reserved_published_question_id IS NOT NULL
           AND v_reserved_published_question_id::text <> p_published_question_id) THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Question Publication ID must match the Draft lineage identity';
    END IF;
    IF v_parent_published_question_id IS NULL THEN
        lineage_owner_id := actor_id::ple_data.account_id;
    ELSE
        SELECT receipt.actor_account_id INTO STRICT lineage_owner_id
          FROM ple_private.draft_question_creation_receipt AS receipt
         WHERE receipt.draft_question_id = p_draft_question_uuid
         FOR SHARE;
    END IF;
    SELECT * INTO STRICT metadata FROM ple_private.draft_question_metadata
     WHERE draft_question_id = p_draft_question_uuid FOR UPDATE;
    IF btrim(metadata.question_title) = '' OR btrim(metadata.question_description) = ''
       OR metadata.question_license IS NULL
       OR p_license IS DISTINCT FROM metadata.question_license::text THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Question Publication requires complete current Draft record metadata';
    END IF;
    v_license := metadata.question_license::text;
    v_initial_shared_tags := metadata.tags;
    IF v_parent_published_question_id IS NOT NULL THEN
        IF metadata.content_discipline_id IS NULL OR metadata.content_subject_id IS NULL THEN
            RAISE EXCEPTION USING ERRCODE = '22023',
                MESSAGE = 'Question Fork Publication requires its copied Draft classification';
        END IF;
        v_discipline_uuid := metadata.content_discipline_id;
        v_subject_uuid := metadata.content_subject_id;
        v_topic_uuid := metadata.content_topic_id;
        v_subtopic_uuid := metadata.content_subtopic_id;
    ELSE
        IF p_initial_shared_tags IS DISTINCT FROM metadata.tags THEN
            RAISE EXCEPTION USING ERRCODE = '22023',
                MESSAGE = 'Question Publication requires complete current Draft record metadata';
        END IF;
        v_discipline_uuid := p_discipline_uuid;
        v_subject_uuid := p_subject_uuid;
        v_topic_uuid := p_topic_uuid;
        v_subtopic_uuid := p_subtopic_uuid;
    END IF;
    SELECT * INTO STRICT binding FROM ple_private.draft_question_source_binding
     WHERE draft_question_id = p_draft_question_uuid FOR UPDATE;
    IF binding.question_type IS NULL THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Question Publication requires a source-derived Question Type';
    END IF;
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
    IF v_reserved_published_question_id IS NOT NULL THEN
        PERFORM pg_catalog.set_config(
            'ple.question_publication_reserved_draft_uuid', p_draft_question_uuid::text, true);
    END IF;
    INSERT INTO ple_data.published_question(
        published_question_id, parent_published_question_id, parent_revision_number, created_at
    ) VALUES (
        p_published_question_id, v_parent_published_question_id,
        v_parent_revision_number, published_at
    );
    IF v_reserved_published_question_id IS NOT NULL THEN
        PERFORM pg_catalog.set_config('ple.question_publication_reserved_draft_uuid', '', true);
    END IF;
    PERFORM ple_private.require_active_content_discipline(v_discipline_uuid);
    INSERT INTO ple_data.question_revision(
        published_question_id, revision_number, backend, general_feedback, hint, worked_solution, published_at
    ) VALUES (
        p_published_question_id, 1, binding.backend,
        metadata.general_feedback, metadata.hint, metadata.worked_solution, published_at
    );
    INSERT INTO ple_data.question_revision_metadata(
        published_question_id, revision_number, question_title, question_description, language, question_type, tags,
        content_discipline_id, content_subject_id, content_topic_id, content_subtopic_id,
        bloom_cognitive_process, bloom_knowledge_dimension, created_at, updated_at
    ) VALUES (p_published_question_id, 1, metadata.question_title, metadata.question_description,
        metadata.language, binding.question_type, v_initial_shared_tags, v_discipline_uuid,
        v_subject_uuid, v_topic_uuid, v_subtopic_uuid,
        metadata.bloom_cognitive_process, metadata.bloom_knowledge_dimension,
        published_at, published_at);
    INSERT INTO ple_private.object_record(
        object_record_id, object_address, object_storage_area, object_data_class, sha256, size_bytes, media_type, created_at
    ) VALUES (p_target_object_id, expected_address, 'private-content', 'question-source',
        p_target_sha256, p_target_size_bytes, p_target_media_type,
        to_timestamp(p_target_created_at_millis::double precision / 1000.0));
    INSERT INTO ple_private.question_revision_source_binding(
        published_question_id, revision_number, backend, native_question_type, question_format, webwork_pg_path,
        source_object_record_id, source_object_checksum, created_at
    ) VALUES (p_published_question_id, 1, binding.backend,
        CASE WHEN binding.backend = 'ple' THEN binding.question_type END, binding.question_format, binding.webwork_pg_path,
        p_target_object_id, encode(p_target_sha256, 'hex'), published_at);
    INSERT INTO ple_data.question_revision_acceptance(
        published_question_id, revision_number, parent_revision_number, editor_account_id,
        accepted_by_account_id, accepted_at, reason_for_edit
    ) VALUES (p_published_question_id, 1, NULL, actor_id, actor_id, published_at, p_reason_for_edit);
    IF v_parent_published_question_id IS NOT NULL THEN
        SELECT count(*), max(author_position)
          INTO author_count, final_author_position
          FROM ple_private.draft_question_authorship
         WHERE draft_question_id = p_draft_question_uuid;
        IF author_count = 0 OR author_count <> final_author_position THEN
            RAISE EXCEPTION USING ERRCODE = '23514',
                MESSAGE = 'Question Fork Publication requires its copied Draft authorship';
        END IF;
        INSERT INTO ple_data.question_revision_authorship(
            published_question_id, revision_number, author_position,
            author_display_name, author_account_id
        ) SELECT p_published_question_id, 1, author.author_position,
                 author.author_display_name, author.author_account_id
            FROM ple_private.draft_question_authorship AS author
           WHERE author.draft_question_id = p_draft_question_uuid
           ORDER BY author.author_position;
    ELSE
        INSERT INTO ple_data.question_revision_authorship(
            published_question_id, revision_number, author_position, author_display_name, author_account_id
        ) SELECT p_published_question_id, 1, author.ordinality::integer, author.value #>> '{}', NULL::uuid
            FROM jsonb_array_elements(p_authorship) WITH ORDINALITY AS author(value, ordinality);
    END IF;
    INSERT INTO ple_data.question_revision_license(published_question_id, revision_number, spdx_expression)
    VALUES (p_published_question_id, 1, v_license::ple_data.license_spdx);
    INSERT INTO ple_data.question_revision_citation(
        published_question_id, revision_number, citation_text, created_at, updated_at
    ) SELECT p_published_question_id, 1, metadata.citation_text, published_at, published_at
       WHERE metadata.citation_text IS NOT NULL;
    INSERT INTO ple_data.question_ownership_event(
        question_ownership_event_id, published_question_id, owner_account_id, recorded_by_account_id, event_kind, occurred_at
    ) VALUES (p_ownership_event_id, p_published_question_id, lineage_owner_id, actor_id, 'initial', published_at);
    INSERT INTO ple_data.question_publication_event(event_id, published_question_id, revision_number, actor_account_id, occurred_at)
    VALUES (p_publication_event_id, p_published_question_id, 1, actor_id, published_at);
    INSERT INTO ple_data.question_availability_event(
        event_id, published_question_id, actor_account_id, availability, edit_number, reason, occurred_at
    ) VALUES (p_availability_event_id, p_published_question_id, actor_id, 'available'::ple_data.question_availability, 1, NULL, published_at);
    PERFORM ple_private.bind_draft_question_image_publication(p_draft_question_uuid, p_authoring_workspace_id,
        p_published_question_id, 1, binding.backend, binding.question_type, p_hotspot_question_image, published_at);
    IF v_parent_published_question_id IS NOT NULL THEN
        PERFORM pg_catalog.set_config(
            'ple.authorized_draft_publication_receipt_uuid', p_draft_question_uuid::text, true);
        DELETE FROM ple_private.draft_question_creation_receipt AS receipt
         WHERE receipt.draft_question_id = p_draft_question_uuid;
        IF NOT FOUND THEN
            RAISE EXCEPTION USING ERRCODE = '23514',
                MESSAGE = 'Question Fork Publication requires its ordinary creation receipt';
        END IF;
        PERFORM pg_catalog.set_config('ple.authorized_draft_publication_receipt_uuid', '', true);
    END IF;
END
$$;

SET LOCAL ROLE ple_api_owner;

CREATE FUNCTION ple_api.load_draft_question_publication_source(
    p_draft_question_uuid uuid, p_expected_draft_question_edit_number bigint, p_authoring_workspace_id uuid
) RETURNS TABLE (object_record_id uuid, object_address jsonb, sha256 bytea, size_bytes bigint,
    media_type text, created_at_millis bigint, reserved_published_question_id text)
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
