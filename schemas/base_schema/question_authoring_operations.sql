-- A Draft source is mutable current authoring state.  The operation is a
-- compare-and-swap and accepts a precise retry only when all source facts are
-- unchanged; it never accepts inline source bytes.
SET LOCAL ROLE ple_private_owner;
CREATE FUNCTION ple_private.bind_draft_question_source(
    p_draft_question_uuid uuid, p_expected_edit_number bigint, p_workspace_id uuid,
    p_backend text, p_question_format text, p_webwork_pg_path text,
    p_imathas_deployment_reference text, p_imathas_item_reference text,
    p_imathas_profile text, p_source_object_id uuid, p_source_object_checksum text
) RETURNS void LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private AS $$
DECLARE
    current_edit bigint;
    existing ple_private.draft_question_source_binding%ROWTYPE;
BEGIN
    IF p_expected_edit_number IS NULL OR p_expected_edit_number <= 0
       OR NOT ple_api.current_session_account_can_access_authoring_workspace(p_workspace_id) THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'authorized Draft Question Edit Number is required';
    END IF;
    SELECT draft_question_edit_number INTO current_edit FROM ple_private.draft_question
     WHERE draft_question_uuid = p_draft_question_uuid AND workspace_id = p_workspace_id
     FOR UPDATE;
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '23514', MESSAGE = 'Draft Question does not belong to workspace';
    END IF;
    SELECT * INTO existing FROM ple_private.draft_question_source_binding
     WHERE draft_question_uuid = p_draft_question_uuid FOR UPDATE;
    IF FOUND AND existing.backend = p_backend AND existing.question_format = p_question_format
       AND existing.webwork_pg_path IS NOT DISTINCT FROM p_webwork_pg_path
       AND existing.imathas_deployment_reference IS NOT DISTINCT FROM p_imathas_deployment_reference
       AND existing.imathas_item_reference IS NOT DISTINCT FROM p_imathas_item_reference
       AND existing.imathas_profile IS NOT DISTINCT FROM p_imathas_profile
       AND existing.source_object_id = p_source_object_id
       AND existing.source_object_checksum = p_source_object_checksum
       AND current_edit IN (p_expected_edit_number, p_expected_edit_number + 1) THEN
        RETURN;
    END IF;
    IF current_edit <> p_expected_edit_number THEN
        RAISE EXCEPTION USING ERRCODE = '40001', MESSAGE = 'Draft Question Edit Number is stale';
    END IF;
    INSERT INTO ple_private.draft_question_source_binding AS binding (
        draft_question_uuid, backend, question_format, webwork_pg_path,
        imathas_deployment_reference, imathas_item_reference, imathas_profile,
        source_object_id, source_object_checksum, created_at, updated_at
    ) VALUES (
        p_draft_question_uuid, p_backend, p_question_format, p_webwork_pg_path,
        p_imathas_deployment_reference, p_imathas_item_reference, p_imathas_profile,
        p_source_object_id, p_source_object_checksum, pg_catalog.clock_timestamp(), pg_catalog.clock_timestamp()
    ) ON CONFLICT (draft_question_uuid) DO UPDATE SET
        backend = EXCLUDED.backend, question_format = EXCLUDED.question_format,
        webwork_pg_path = EXCLUDED.webwork_pg_path,
        imathas_deployment_reference = EXCLUDED.imathas_deployment_reference,
        imathas_item_reference = EXCLUDED.imathas_item_reference,
        imathas_profile = EXCLUDED.imathas_profile,
        source_object_id = EXCLUDED.source_object_id,
        source_object_checksum = EXCLUDED.source_object_checksum,
        updated_at = EXCLUDED.updated_at;
    UPDATE ple_private.draft_question
       SET draft_question_edit_number = draft_question_edit_number + 1,
           updated_at = pg_catalog.clock_timestamp()
     WHERE draft_question_uuid = p_draft_question_uuid;
END
$$;
REVOKE ALL ON FUNCTION ple_private.bind_draft_question_source(
    uuid, bigint, uuid, text, text, text, text, text, text, uuid, text) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_private.bind_draft_question_source(
    uuid, bigint, uuid, text, text, text, text, text, text, uuid, text) TO ple_api_owner;
RESET ROLE;

-- A later publication creates another immutable version of an existing stable
-- Question lineage.  Locking that lineage serializes its revision numbers;
-- availability remains lineage state and is not changed here.
-- ASVS 1.2.4, 2.2.1-2.2.3, 2.3.1-2.3.4, and 8.2.1-8.3.1: the trusted
-- database operation validates the complete aggregate, rechecks current
-- workspace and Question Owner authority, and commits it atomically.
SET LOCAL ROLE ple_private_owner;
CREATE FUNCTION ple_private.publish_question_revision(
    p_draft_question_uuid uuid, p_expected_edit_number bigint, p_workspace_id uuid,
    p_question_id text, p_expected_parent_revision_number integer,
    p_target_object_id uuid, p_target_object_address jsonb,
    p_target_sha256 bytea, p_target_size_bytes bigint, p_target_media_type text,
    p_target_created_at_millis bigint, p_reason_for_edit text, p_publication_event_id uuid
) RETURNS integer LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
DECLARE
    actor_id uuid;
    current_edit bigint;
    next_revision_number integer;
    metadata ple_private.draft_question_metadata%ROWTYPE;
    binding ple_private.draft_question_source_binding%ROWTYPE;
    source_record ple_private.object_record%ROWTYPE;
    expected_address jsonb;
    published_at timestamptz := clock_timestamp();
BEGIN
    IF p_expected_edit_number IS NULL OR p_expected_edit_number <= 0
       OR p_question_id IS NULL OR p_question_id !~ '^[0-9A-HJKMNP-TV-Z]{7}$'
       OR p_expected_parent_revision_number IS NULL OR p_expected_parent_revision_number <= 0
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
       OR NOT ple_api.current_session_account_can_access_authoring_workspace(p_workspace_id) THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Question Revision Publication requires current Authoring Workspace access';
    END IF;
    actor_id := ple_api.current_session_account_id();
    SELECT draft_question_edit_number INTO current_edit FROM ple_private.draft_question
     WHERE draft_question_uuid = p_draft_question_uuid AND workspace_id = p_workspace_id FOR UPDATE;
    IF NOT FOUND OR current_edit <> p_expected_edit_number THEN
        RAISE EXCEPTION USING ERRCODE = 'PQR01',
            MESSAGE = 'Question Revision Publication Draft Question Edit Number is stale or not in its workspace';
    END IF;
    PERFORM 1 FROM ple_data.published_question WHERE question_id = p_question_id FOR UPDATE;
    IF NOT FOUND OR NOT EXISTS (
        SELECT 1 FROM ple_data.question_current_owner
         WHERE question_id = p_question_id AND owner_account_id = actor_id
    ) THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Question Revision Publication requires current Question Owner authority';
    END IF;
    SELECT COALESCE(max(revision_number), 0) + 1 INTO next_revision_number
      FROM ple_data.question_revision WHERE question_id = p_question_id;
    IF next_revision_number <= 1 OR next_revision_number - 1 <> p_expected_parent_revision_number THEN
        RAISE EXCEPTION USING ERRCODE = 'PQR01',
            MESSAGE = 'Question Revision Publication parent Revision is stale';
    END IF;
    SELECT * INTO STRICT metadata FROM ple_private.draft_question_metadata
     WHERE draft_question_uuid = p_draft_question_uuid FOR UPDATE;
    SELECT * INTO STRICT binding FROM ple_private.draft_question_source_binding
     WHERE draft_question_uuid = p_draft_question_uuid FOR UPDATE;
    SELECT * INTO STRICT source_record FROM ple_private.object_record
     WHERE object_id = binding.source_object_id;
    expected_address := jsonb_build_object('kind', 'questionSource',
        'questionRevision', jsonb_build_object('questionId', p_question_id,
            'revisionNumber', next_revision_number), 'object', p_target_object_id);
    IF p_target_object_address IS DISTINCT FROM expected_address
       OR p_target_sha256 IS DISTINCT FROM source_record.sha256
       OR encode(p_target_sha256, 'hex') IS DISTINCT FROM binding.source_object_checksum
       OR p_target_size_bytes IS DISTINCT FROM source_record.size_bytes
       OR p_target_media_type IS DISTINCT FROM source_record.media_type THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Question Revision Publication target must preserve the exact Draft Question Source bytes';
    END IF;
    INSERT INTO ple_data.question_revision(question_id, revision_number, backend, published_at)
    VALUES (p_question_id, next_revision_number, binding.backend, published_at);
    INSERT INTO ple_private.object_record(
        object_id, object_address, object_storage_area, object_data_class, sha256, size_bytes, media_type, created_at
    ) VALUES (p_target_object_id, expected_address, 'private-content', 'question-source',
        p_target_sha256, p_target_size_bytes, p_target_media_type,
        to_timestamp(p_target_created_at_millis::double precision / 1000.0));
    INSERT INTO ple_private.question_revision_source_binding(
        question_id, revision_number, backend, question_format, webwork_pg_path,
        imathas_deployment_reference, imathas_item_reference, imathas_profile,
        source_object_id, source_object_checksum, created_at
    ) VALUES (p_question_id, next_revision_number, binding.backend, binding.question_format,
        binding.webwork_pg_path, binding.imathas_deployment_reference,
        binding.imathas_item_reference, binding.imathas_profile, p_target_object_id,
        encode(p_target_sha256, 'hex'), published_at);
    INSERT INTO ple_data.question_revision_acceptance(
        question_id, revision_number, parent_revision_number, editor_account_id,
        accepted_by_account_id, accepted_at, reason_for_edit
    ) VALUES (p_question_id, next_revision_number, next_revision_number - 1, actor_id, actor_id,
        published_at, p_reason_for_edit);
    INSERT INTO ple_data.question_revision_authorship(
        question_id, revision_number, author_position, author_display_name, author_account_id
    ) SELECT p_question_id, next_revision_number, author.author_position,
        author.author_display_name, author.author_account_id
        -- Moderate edits retain the parent revision's immutable credit. A
        -- fork is the separate path that creates a new authorship record.
        FROM ple_data.question_revision_authorship AS author
       WHERE author.question_id = p_question_id
         AND author.revision_number = p_expected_parent_revision_number;
    INSERT INTO ple_data.question_revision_license(question_id, revision_number, spdx_expression)
    SELECT p_question_id, next_revision_number, license.spdx_expression
      -- The compatible CC license is immutable lineage evidence as well.
      FROM ple_data.question_revision_license AS license
     WHERE license.question_id = p_question_id
       AND license.revision_number = p_expected_parent_revision_number;
    UPDATE ple_data.published_question_metadata
       SET question_title = metadata.question_title,
           question_description = metadata.question_description,
           language = metadata.language,
           updated_at = published_at
     WHERE question_id = p_question_id;
    INSERT INTO ple_data.question_publication_event(event_id, question_id, revision_number, actor_account_id, occurred_at)
    VALUES (p_publication_event_id, p_question_id, next_revision_number, actor_id, published_at);
    RETURN next_revision_number;
END
$$;
REVOKE ALL ON FUNCTION ple_private.publish_question_revision(
    uuid, bigint, uuid, text, integer, uuid, jsonb, bytea, bigint, text, bigint, text, uuid) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_private.publish_question_revision(
    uuid, bigint, uuid, text, integer, uuid, jsonb, bytea, bigint, text, bigint, text, uuid) TO ple_api_owner;
RESET ROLE;

SET LOCAL ROLE ple_api_owner;
-- Authoring owns these API predicates because only authoring owns the
-- workspace relations.  The API owner receives no table privilege or RLS
-- bypass; it delegates to the narrowly scoped private-owner predicates.
CREATE FUNCTION ple_api.current_session_account_is_authoring_workspace_owner(
    p_workspace_id uuid
) RETURNS boolean LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private AS $$
    SELECT ple_private.current_session_is_authoring_workspace_owner(p_workspace_id)
$$;
CREATE FUNCTION ple_api.current_session_account_can_access_authoring_workspace(
    p_workspace_id uuid
) RETURNS boolean LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private AS $$
    SELECT ple_private.current_session_can_access_authoring_workspace(p_workspace_id)
$$;
REVOKE ALL ON FUNCTION ple_api.current_session_account_is_authoring_workspace_owner(uuid),
    ple_api.current_session_account_can_access_authoring_workspace(uuid) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_api.current_session_account_is_authoring_workspace_owner(uuid),
    ple_api.current_session_account_can_access_authoring_workspace(uuid)
    TO ple_app, ple_auth, ple_student, ple_data_owner, ple_private_owner;

CREATE FUNCTION ple_api.bind_draft_question_source(
    p_draft_question_uuid uuid, p_expected_edit_number bigint, p_workspace_id uuid,
    p_backend text, p_question_format text, p_webwork_pg_path text,
    p_imathas_deployment_reference text, p_imathas_item_reference text,
    p_imathas_profile text, p_source_object_id uuid, p_source_object_checksum text
) RETURNS void LANGUAGE sql SECURITY DEFINER
SET search_path = pg_catalog, ple_private, ple_api AS $$
    SELECT ple_private.bind_draft_question_source(
        p_draft_question_uuid, p_expected_edit_number, p_workspace_id, p_backend,
        p_question_format, p_webwork_pg_path, p_imathas_deployment_reference,
        p_imathas_item_reference, p_imathas_profile, p_source_object_id, p_source_object_checksum)
$$;
CREATE FUNCTION ple_api.publish_question_revision(
    p_draft_question_uuid uuid, p_expected_edit_number bigint, p_workspace_id uuid,
    p_question_id text, p_expected_parent_revision_number integer,
    p_target_object_id uuid, p_target_object_address jsonb,
    p_target_sha256 bytea, p_target_size_bytes bigint, p_target_media_type text,
    p_target_created_at_millis bigint, p_reason_for_edit text, p_publication_event_id uuid
) RETURNS integer LANGUAGE sql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private AS $$
    SELECT ple_private.publish_question_revision(
        p_draft_question_uuid, p_expected_edit_number, p_workspace_id, p_question_id,
        p_expected_parent_revision_number, p_target_object_id, p_target_object_address,
        p_target_sha256, p_target_size_bytes,
        p_target_media_type, p_target_created_at_millis, p_reason_for_edit, p_publication_event_id)
$$;
REVOKE ALL ON FUNCTION ple_api.bind_draft_question_source(
    uuid, bigint, uuid, text, text, text, text, text, text, uuid, text) FROM PUBLIC;
REVOKE ALL ON FUNCTION ple_api.publish_question_revision(
    uuid, bigint, uuid, text, integer, uuid, jsonb, bytea, bigint, text, bigint, text, uuid) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_api.bind_draft_question_source(
    uuid, bigint, uuid, text, text, text, text, text, text, uuid, text) TO ple_app;
GRANT EXECUTE ON FUNCTION ple_api.publish_question_revision(
    uuid, bigint, uuid, text, integer, uuid, jsonb, bytea, bigint, text, bigint, text, uuid) TO ple_app;
RESET ROLE;

-- Publication reads one current Draft source record, then the server copies its
-- verified bytes to the typed immutable Question Revision address before this
-- transaction records the complete Question aggregate.
SET LOCAL ROLE ple_private_owner;
CREATE FUNCTION ple_private.load_draft_question_publication_source(
    p_draft_question_uuid uuid, p_expected_edit_number bigint, p_workspace_id uuid
) RETURNS TABLE (
    object_id uuid, object_address jsonb, sha256 bytea, size_bytes bigint,
    media_type text, created_at_millis bigint
) LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private AS $$
DECLARE current_edit bigint; row_count bigint;
BEGIN
    IF p_draft_question_uuid IS NULL OR p_workspace_id IS NULL
       OR p_expected_edit_number IS NULL OR p_expected_edit_number <= 0 THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Draft Question Publication Source arguments are invalid';
    END IF;
    IF NOT ple_api.current_session_account_is_instructor()
       OR NOT ple_api.current_session_account_can_access_authoring_workspace(p_workspace_id) THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Draft Question Publication Source requires current Authoring Workspace access';
    END IF;
    SELECT draft_question_edit_number INTO current_edit FROM ple_private.draft_question
     WHERE draft_question_uuid = p_draft_question_uuid AND workspace_id = p_workspace_id;
    IF NOT FOUND OR current_edit <> p_expected_edit_number THEN
        RAISE EXCEPTION USING ERRCODE = '40001',
            MESSAGE = 'Draft Question Publication Source Edit Number is stale or not in its workspace';
    END IF;
    RETURN QUERY SELECT record.object_id, record.object_address, record.sha256,
        record.size_bytes, record.media_type,
        round(extract(epoch FROM record.created_at) * 1000)::bigint
      FROM ple_private.draft_question_source_binding AS binding
      JOIN ple_private.object_record AS record ON record.object_id = binding.source_object_id
     WHERE binding.draft_question_uuid = p_draft_question_uuid
       AND binding.source_object_checksum = encode(record.sha256, 'hex')
       AND record.object_storage_area = 'private-content'
       AND record.object_data_class = 'authoring-content'
       AND record.object_address = jsonb_build_object('kind', 'workspaceQuestionSource',
           'workspace', p_workspace_id, 'object', record.object_id);
    GET DIAGNOSTICS row_count = ROW_COUNT;
    IF row_count <> 1 THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Draft Question Publication Source Binding is incomplete';
    END IF;
END
$$;

CREATE FUNCTION ple_private.publish_new_question_lineage(
    p_draft_question_uuid uuid, p_expected_edit_number bigint, p_workspace_id uuid,
    p_question_id text, p_target_object_id uuid, p_target_object_address jsonb,
    p_target_sha256 bytea, p_target_size_bytes bigint, p_target_media_type text,
    p_target_created_at_millis bigint, p_authorship jsonb, p_license text,
    p_reason_for_edit text, p_ownership_event_id uuid, p_publication_event_id uuid,
    p_availability_event_id uuid
) RETURNS void LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
DECLARE
    actor_id uuid; current_edit bigint; metadata ple_private.draft_question_metadata%ROWTYPE;
    binding ple_private.draft_question_source_binding%ROWTYPE;
    source_record ple_private.object_record%ROWTYPE; expected_address jsonb;
    published_at timestamptz := clock_timestamp(); author_count integer; valid_count integer;
BEGIN
    IF p_expected_edit_number IS NULL OR p_expected_edit_number <= 0
       OR p_question_id IS NULL OR p_question_id !~ '^[0-9A-HJKMNP-TV-Z]{7}$'
       OR p_target_object_id IS NULL OR p_target_sha256 IS NULL OR octet_length(p_target_sha256) <> 32
       OR p_target_size_bytes IS NULL OR p_target_size_bytes < 0
       OR p_target_media_type IS NULL OR char_length(btrim(p_target_media_type)) NOT BETWEEN 1 AND 255
       OR p_target_created_at_millis IS NULL OR p_authorship IS NULL
       OR jsonb_typeof(p_authorship) <> 'array' OR jsonb_array_length(p_authorship) NOT BETWEEN 1 AND 16
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
       OR NOT ple_api.current_session_account_can_access_authoring_workspace(p_workspace_id) THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Question Publication requires current Authoring Workspace access';
    END IF;
    actor_id := ple_api.current_session_account_id();
    SELECT draft_question_edit_number INTO current_edit FROM ple_private.draft_question
     WHERE draft_question_uuid = p_draft_question_uuid AND workspace_id = p_workspace_id FOR UPDATE;
    IF NOT FOUND OR current_edit <> p_expected_edit_number THEN
        RAISE EXCEPTION USING ERRCODE = '40001',
            MESSAGE = 'Question Publication Draft Question Edit Number is stale or not in its workspace';
    END IF;
    SELECT * INTO STRICT metadata FROM ple_private.draft_question_metadata
     WHERE draft_question_uuid = p_draft_question_uuid FOR UPDATE;
    SELECT * INTO STRICT binding FROM ple_private.draft_question_source_binding
     WHERE draft_question_uuid = p_draft_question_uuid FOR UPDATE;
    SELECT * INTO STRICT source_record FROM ple_private.object_record
     WHERE object_id = binding.source_object_id;
    expected_address := jsonb_build_object('kind', 'questionSource',
        'questionRevision', jsonb_build_object('questionId', p_question_id, 'revisionNumber', 1),
        'object', p_target_object_id);
    IF p_target_object_address IS DISTINCT FROM expected_address
       OR p_target_sha256 IS DISTINCT FROM source_record.sha256
       OR encode(p_target_sha256, 'hex') IS DISTINCT FROM binding.source_object_checksum
       OR p_target_size_bytes IS DISTINCT FROM source_record.size_bytes
       OR p_target_media_type IS DISTINCT FROM source_record.media_type THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Question Publication target must preserve the exact Draft Question Source bytes';
    END IF;
    INSERT INTO ple_data.published_question(question_id, created_at) VALUES (p_question_id, published_at);
    INSERT INTO ple_data.published_question_metadata(
        question_id, question_title, question_description, language, created_at, updated_at
    ) VALUES (p_question_id, metadata.question_title, metadata.question_description, metadata.language,
        published_at, published_at);
    INSERT INTO ple_data.question_revision(question_id, revision_number, backend, published_at)
    VALUES (p_question_id, 1, binding.backend, published_at);
    INSERT INTO ple_private.object_record(
        object_id, object_address, object_storage_area, object_data_class, sha256, size_bytes, media_type, created_at
    ) VALUES (p_target_object_id, expected_address, 'private-content', 'question-source',
        p_target_sha256, p_target_size_bytes, p_target_media_type,
        to_timestamp(p_target_created_at_millis::double precision / 1000.0));
    INSERT INTO ple_private.question_revision_source_binding(
        question_id, revision_number, backend, question_format, webwork_pg_path,
        imathas_deployment_reference, imathas_item_reference, imathas_profile,
        source_object_id, source_object_checksum, created_at
    ) VALUES (p_question_id, 1, binding.backend, binding.question_format, binding.webwork_pg_path,
        binding.imathas_deployment_reference, binding.imathas_item_reference, binding.imathas_profile,
        p_target_object_id, encode(p_target_sha256, 'hex'), published_at);
    INSERT INTO ple_data.question_revision_acceptance(
        question_id, revision_number, parent_revision_number, editor_account_id,
        accepted_by_account_id, accepted_at, reason_for_edit
    ) VALUES (p_question_id, 1, NULL, actor_id, actor_id, published_at, p_reason_for_edit);
    INSERT INTO ple_data.question_revision_authorship(
        question_id, revision_number, author_position, author_display_name, author_account_id
    ) SELECT p_question_id, 1, author.ordinality::integer, author.value #>> '{}', NULL::uuid
        FROM jsonb_array_elements(p_authorship) WITH ORDINALITY AS author(value, ordinality);
    INSERT INTO ple_data.question_revision_license(question_id, revision_number, spdx_expression)
    VALUES (p_question_id, 1, p_license);
    INSERT INTO ple_data.question_ownership_event(
        question_ownership_event_id, question_id, owner_account_id, recorded_by_account_id, event_kind, occurred_at
    ) VALUES (p_ownership_event_id, p_question_id, actor_id, actor_id, 'initial', published_at);
    INSERT INTO ple_data.question_fork_source(
        forked_question_id, source_question_id, source_revision_number, recorded_at
    ) SELECT p_question_id, source_question_id, source_revision_number, published_at
      FROM ple_private.draft_question_fork_source WHERE draft_question_uuid = p_draft_question_uuid;
    INSERT INTO ple_data.question_publication_event(event_id, question_id, revision_number, actor_account_id, occurred_at)
    VALUES (p_publication_event_id, p_question_id, 1, actor_id, published_at);
    INSERT INTO ple_data.question_availability_event(
        event_id, question_id, actor_account_id, availability, edit_number, reason, occurred_at
    ) VALUES (p_availability_event_id, p_question_id, actor_id, 'available', 1, NULL, published_at);
END
$$;

REVOKE ALL ON FUNCTION ple_private.load_draft_question_publication_source(uuid, bigint, uuid),
    ple_private.publish_new_question_lineage(uuid, bigint, uuid, text, uuid, jsonb, bytea, bigint,
        text, bigint, jsonb, text, text, uuid, uuid, uuid) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_private.load_draft_question_publication_source(uuid, bigint, uuid),
    ple_private.publish_new_question_lineage(uuid, bigint, uuid, text, uuid, jsonb, bytea, bigint,
        text, bigint, jsonb, text, text, uuid, uuid, uuid) TO ple_api_owner;
RESET ROLE;

SET LOCAL ROLE ple_api_owner;
CREATE FUNCTION ple_api.load_draft_question_publication_source(
    p_draft_question_uuid uuid, p_expected_edit_number bigint, p_workspace_id uuid
) RETURNS TABLE (object_id uuid, object_address jsonb, sha256 bytea, size_bytes bigint,
    media_type text, created_at_millis bigint)
LANGUAGE sql SECURITY DEFINER SET search_path = pg_catalog, ple_api, ple_private AS $$
    SELECT * FROM ple_private.load_draft_question_publication_source(
        p_draft_question_uuid, p_expected_edit_number, p_workspace_id)
$$;
CREATE FUNCTION ple_api.publish_new_question_lineage(
    p_draft_question_uuid uuid, p_expected_edit_number bigint, p_workspace_id uuid,
    p_question_id text, p_target_object_id uuid, p_target_object_address jsonb,
    p_target_sha256 bytea, p_target_size_bytes bigint, p_target_media_type text,
    p_target_created_at_millis bigint, p_authorship jsonb, p_license text,
    p_reason_for_edit text, p_ownership_event_id uuid, p_publication_event_id uuid,
    p_availability_event_id uuid
) RETURNS void LANGUAGE sql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private AS $$
    SELECT ple_private.publish_new_question_lineage(p_draft_question_uuid, p_expected_edit_number,
        p_workspace_id, p_question_id, p_target_object_id, p_target_object_address, p_target_sha256,
        p_target_size_bytes, p_target_media_type, p_target_created_at_millis, p_authorship, p_license,
        p_reason_for_edit, p_ownership_event_id, p_publication_event_id, p_availability_event_id)
$$;
REVOKE ALL ON FUNCTION ple_api.load_draft_question_publication_source(uuid, bigint, uuid),
    ple_api.publish_new_question_lineage(uuid, bigint, uuid, text, uuid, jsonb, bytea, bigint,
        text, bigint, jsonb, text, text, uuid, uuid, uuid) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_api.load_draft_question_publication_source(uuid, bigint, uuid),
    ple_api.publish_new_question_lineage(uuid, bigint, uuid, text, uuid, jsonb, bytea, bigint,
        text, bigint, jsonb, text, text, uuid, uuid, uuid) TO ple_app;
RESET ROLE;

SET LOCAL ROLE ple_private_owner;
CREATE FUNCTION ple_private.question_library_entries(
    p_question_id text DEFAULT NULL, p_revision_number integer DEFAULT NULL,
    p_require_available boolean DEFAULT true
) RETURNS TABLE (
    question_id text, revision_number integer, backend text, published_at_millis bigint,
    question_title text, question_description text, author_names text[],
    authored_by_current_account boolean, question_license text, availability text,
    availability_edit_number bigint, source_object_id uuid, source_object_checksum text,
    source_media_type text
) LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
BEGIN
    IF NOT ple_api.current_session_account_is_instructor() THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Question Library requires an active Instructor Account';
    END IF;
    RETURN QUERY
    SELECT revision.question_id, revision.revision_number, revision.backend,
           floor(extract(epoch FROM revision.published_at) * 1000)::bigint,
           metadata.question_title, metadata.question_description,
           ARRAY(SELECT authorship.author_display_name
               FROM ple_data.question_revision_authorship AS authorship
              WHERE authorship.question_id = revision.question_id
                AND authorship.revision_number = revision.revision_number
              ORDER BY authorship.author_position),
           EXISTS (SELECT 1 FROM ple_data.question_revision_authorship AS authorship
              WHERE authorship.question_id = revision.question_id
                AND authorship.revision_number = revision.revision_number
                AND authorship.author_account_id = ple_api.current_session_account_id()),
           license.spdx_expression, lineage.availability, lineage.availability_edit_number,
           binding.source_object_id, binding.source_object_checksum, record.media_type
      FROM ple_data.question_revision AS revision
      JOIN ple_data.published_question AS lineage ON lineage.question_id = revision.question_id
      JOIN ple_data.published_question_metadata AS metadata ON metadata.question_id = revision.question_id
      JOIN ple_data.question_revision_license AS license
        ON license.question_id = revision.question_id AND license.revision_number = revision.revision_number
      JOIN ple_private.question_revision_source_binding AS binding
        ON binding.question_id = revision.question_id AND binding.revision_number = revision.revision_number
      JOIN ple_private.object_record AS record ON record.object_id = binding.source_object_id
     WHERE (p_question_id IS NULL OR revision.question_id = p_question_id)
       AND (p_revision_number IS NULL OR revision.revision_number = p_revision_number)
       AND (p_revision_number IS NOT NULL OR revision.revision_number = (
           SELECT max(accepted.revision_number)
             FROM ple_data.question_revision_acceptance AS accepted
            WHERE accepted.question_id = revision.question_id
       ))
       AND (NOT p_require_available OR lineage.availability = 'available')
     ORDER BY metadata.question_title, revision.question_id, revision.revision_number;
END
$$;
REVOKE ALL ON FUNCTION ple_private.question_library_entries(text, integer, boolean) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_private.question_library_entries(text, integer, boolean) TO ple_api_owner;
RESET ROLE;

SET LOCAL ROLE ple_api_owner;
CREATE VIEW ple_api.published_question_summary
WITH (security_barrier = true, security_invoker = false) AS
SELECT lineage.question_id, revision.revision_number AS latest_question_revision_number,
       revision.backend, revision.published_at, metadata.question_title,
       metadata.question_description, metadata.language, lineage.availability,
       lineage.availability_edit_number
  FROM ple_data.published_question AS lineage
  JOIN ple_data.published_question_metadata AS metadata ON metadata.question_id = lineage.question_id
  JOIN LATERAL (
      SELECT accepted.revision_number FROM ple_data.question_revision_acceptance AS accepted
       WHERE accepted.question_id = lineage.question_id
       ORDER BY accepted.revision_number DESC LIMIT 1
  ) AS latest ON true
  JOIN ple_data.question_revision AS revision
    ON revision.question_id = lineage.question_id AND revision.revision_number = latest.revision_number;
CREATE FUNCTION ple_api.list_question_library_entries()
RETURNS TABLE (
    question_id text, revision_number integer, backend text, published_at_millis bigint,
    question_title text, question_description text, author_names text[],
    authored_by_current_account boolean, question_license text, availability text,
    availability_edit_number bigint, source_object_id uuid, source_object_checksum text,
    source_media_type text
) LANGUAGE sql SECURITY DEFINER SET search_path = pg_catalog, ple_api, ple_private AS $$
    SELECT * FROM ple_private.question_library_entries(NULL, NULL, true)
$$;
CREATE FUNCTION ple_api.load_question_library_revision(p_question_id text, p_revision_number integer)
RETURNS TABLE (
    question_id text, revision_number integer, backend text, published_at_millis bigint,
    question_title text, question_description text, author_names text[],
    authored_by_current_account boolean, question_license text, availability text,
    availability_edit_number bigint, source_object_id uuid, source_object_checksum text,
    source_media_type text
) LANGUAGE sql SECURITY DEFINER SET search_path = pg_catalog, ple_api, ple_private AS $$
    SELECT * FROM ple_private.question_library_entries(p_question_id, p_revision_number, false)
$$;
REVOKE ALL ON TABLE ple_api.published_question_summary FROM PUBLIC;
REVOKE ALL ON FUNCTION ple_api.list_question_library_entries(),
    ple_api.load_question_library_revision(text, integer) FROM PUBLIC;
GRANT SELECT ON TABLE ple_api.published_question_summary TO ple_app;
GRANT EXECUTE ON FUNCTION ple_api.list_question_library_entries(),
    ple_api.load_question_library_revision(text, integer) TO ple_app;
RESET ROLE;

-- A Draft is current private authoring state.  These operations retain the
-- source-derived language with its title and description from the first save;
-- publication later copies that complete metadata into its immutable Revision.
SET LOCAL ROLE ple_private_owner;
CREATE FUNCTION ple_private.ensure_own_authoring_workspace(p_proposed_workspace_id uuid)
RETURNS uuid LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private AS $$
DECLARE
    account_id uuid;
    workspace_id uuid;
BEGIN
    IF p_proposed_workspace_id IS NULL
       OR NOT ple_api.current_session_account_is_instructor() THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Authoring Workspace requires a current Instructor Account';
    END IF;
    account_id := ple_api.current_session_account_id();
    PERFORM pg_catalog.pg_advisory_xact_lock(
        pg_catalog.hashtextextended(account_id::text, 0));
    SELECT workspace.workspace_id INTO workspace_id
      FROM ple_private.authoring_workspace AS workspace
     WHERE workspace.owner_account_id = account_id AND workspace.revoked_at IS NULL
     ORDER BY workspace.created_at, workspace.workspace_id
     LIMIT 1;
    IF workspace_id IS NULL THEN
        INSERT INTO ple_private.authoring_workspace(workspace_id, owner_account_id, created_at)
        VALUES (p_proposed_workspace_id, account_id, pg_catalog.clock_timestamp());
        workspace_id := p_proposed_workspace_id;
    END IF;
    RETURN workspace_id;
END
$$;

CREATE FUNCTION ple_private.list_authoring_drafts()
RETURNS TABLE (
    reference_number bigint, draft_question_edit_number bigint,
    question_title text, question_description text
) LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private AS $$
    SELECT question.reference_number, question.draft_question_edit_number,
           metadata.question_title, metadata.question_description
      FROM ple_private.draft_question AS question
      JOIN ple_private.draft_question_metadata AS metadata
        ON metadata.draft_question_uuid = question.draft_question_uuid
     WHERE ple_api.current_session_account_is_instructor()
       AND ple_api.current_session_account_can_access_authoring_workspace(question.workspace_id)
     ORDER BY question.updated_at DESC, question.reference_number DESC
$$;

CREATE FUNCTION ple_private.load_authoring_draft(p_reference_number bigint)
RETURNS TABLE (
    draft_question_uuid uuid, workspace_id uuid, reference_number bigint,
    draft_question_edit_number bigint, question_title text, question_description text,
    language text, object_id uuid, object_address jsonb, sha256 bytea,
    size_bytes bigint, media_type text, created_at_millis bigint
) LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private AS $$
    SELECT question.draft_question_uuid, question.workspace_id, question.reference_number,
           question.draft_question_edit_number, metadata.question_title,
           metadata.question_description, metadata.language, record.object_id,
           record.object_address, record.sha256, record.size_bytes, record.media_type,
           pg_catalog.round(extract(epoch FROM record.created_at) * 1000)::bigint
      FROM ple_private.draft_question AS question
      JOIN ple_private.draft_question_metadata AS metadata
        ON metadata.draft_question_uuid = question.draft_question_uuid
      JOIN ple_private.draft_question_source_binding AS binding
        ON binding.draft_question_uuid = question.draft_question_uuid
      JOIN ple_private.object_record AS record
        ON record.object_id = binding.source_object_id
       AND binding.source_object_checksum = pg_catalog.encode(record.sha256, 'hex')
     WHERE p_reference_number BETWEEN 1 AND 2147483647
       AND ple_api.current_session_account_is_instructor()
       AND ple_api.current_session_account_can_access_authoring_workspace(question.workspace_id)
       AND question.reference_number = p_reference_number
       AND record.object_storage_area = 'private-content'
       AND record.object_data_class = 'authoring-content'
       AND record.object_address = pg_catalog.jsonb_build_object(
           'kind', 'workspaceQuestionSource', 'workspace', question.workspace_id,
           'object', record.object_id)
$$;

CREATE FUNCTION ple_private.create_authoring_draft(
    p_workspace_id uuid, p_draft_question_uuid uuid, p_object_id uuid,
    p_object_address jsonb, p_sha256 bytea, p_size_bytes bigint, p_media_type text,
    p_created_at_millis bigint, p_question_title text, p_question_description text,
    p_language text, p_webwork_pg_path text
) RETURNS bigint LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private AS $$
DECLARE
    reference_number bigint;
    created_at timestamptz;
    expected_address jsonb;
BEGIN
    IF p_workspace_id IS NULL OR p_draft_question_uuid IS NULL OR p_object_id IS NULL
       OR p_sha256 IS NULL OR pg_catalog.octet_length(p_sha256) <> 32
       OR p_size_bytes IS NULL OR p_size_bytes < 0
       OR NOT (
           (p_media_type = 'application/vnd.peptidyle.question+json'
                AND p_webwork_pg_path IS NULL)
           OR (p_media_type = 'text/x-wework-pg'
                AND p_webwork_pg_path IS NOT NULL
                AND char_length(p_webwork_pg_path) BETWEEN 1 AND 1024
                AND left(p_webwork_pg_path, 1) <> '/'
                AND position(E'\\' in p_webwork_pg_path) = 0
                AND NOT EXISTS (
                    SELECT 1 FROM unnest(string_to_array(p_webwork_pg_path, '/')) AS segment(value)
                     WHERE segment.value IN ('', '.', '..')
                ))
       )
       OR p_created_at_millis IS NULL
       OR p_question_title IS NULL OR p_question_title <> btrim(p_question_title)
       OR char_length(p_question_title) NOT BETWEEN 1 AND 512 OR p_question_title ~ '[[:cntrl:]]'
       OR p_question_description IS NULL OR p_question_description <> btrim(p_question_description)
       OR char_length(p_question_description) NOT BETWEEN 1 AND 4000 OR p_question_description ~ '[[:cntrl:]]'
       OR p_language IS NULL OR p_language <> btrim(p_language)
       OR char_length(p_language) NOT BETWEEN 2 AND 35
       OR NOT ple_api.current_session_account_is_instructor()
       OR NOT ple_api.current_session_account_is_authoring_workspace_owner(p_workspace_id) THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Draft Question creation arguments are invalid';
    END IF;
    expected_address := pg_catalog.jsonb_build_object(
        'kind', 'workspaceQuestionSource', 'workspace', p_workspace_id, 'object', p_object_id);
    IF p_object_address IS DISTINCT FROM expected_address THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Draft Question source must use its exact Workspace Question Source Object Address';
    END IF;
    created_at := pg_catalog.to_timestamp(p_created_at_millis::double precision / 1000.0);
    INSERT INTO ple_private.draft_question(
        draft_question_uuid, workspace_id, created_at, updated_at
    ) VALUES (
        p_draft_question_uuid, p_workspace_id, pg_catalog.clock_timestamp(), pg_catalog.clock_timestamp()
    ) RETURNING draft_question.reference_number INTO reference_number;
    INSERT INTO ple_private.draft_question_metadata(
        draft_question_uuid, question_title, question_description, language, created_at, updated_at
    ) VALUES (
        p_draft_question_uuid, p_question_title, p_question_description, p_language,
        pg_catalog.clock_timestamp(), pg_catalog.clock_timestamp());
    INSERT INTO ple_private.object_record(
        object_id, object_address, object_storage_area, object_data_class,
        sha256, size_bytes, media_type, created_at
    ) VALUES (
        p_object_id, expected_address, 'private-content', 'authoring-content',
        p_sha256, p_size_bytes, p_media_type, created_at);
    INSERT INTO ple_private.draft_question_source_binding(
        draft_question_uuid, backend, question_format, webwork_pg_path, source_object_id,
        source_object_checksum, created_at, updated_at
    ) VALUES (
        p_draft_question_uuid,
        CASE p_media_type WHEN 'application/vnd.peptidyle.question+json' THEN 'ple' ELSE 'webwork' END,
        CASE p_media_type WHEN 'application/vnd.peptidyle.question+json' THEN 'pleQuestionJson' ELSE 'webworkPg' END,
        p_webwork_pg_path, p_object_id,
        pg_catalog.encode(p_sha256, 'hex'), pg_catalog.clock_timestamp(), pg_catalog.clock_timestamp());
    RETURN reference_number;
END
$$;

CREATE FUNCTION ple_private.save_authoring_draft(
    p_reference_number bigint, p_expected_edit_number bigint, p_object_id uuid,
    p_object_address jsonb, p_sha256 bytea, p_size_bytes bigint, p_media_type text,
    p_created_at_millis bigint, p_question_title text, p_question_description text,
    p_language text
) RETURNS void LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private AS $$
DECLARE
    v_draft_question_uuid uuid;
    v_workspace_id uuid;
    v_current_edit_number bigint;
    v_binding ple_private.draft_question_source_binding%ROWTYPE;
    expected_address jsonb;
BEGIN
    IF p_reference_number IS NULL OR p_reference_number NOT BETWEEN 1 AND 2147483647
       OR p_expected_edit_number IS NULL OR p_expected_edit_number <= 0
       OR p_object_id IS NULL OR p_sha256 IS NULL OR pg_catalog.octet_length(p_sha256) <> 32
       OR p_size_bytes IS NULL OR p_size_bytes < 0
       OR p_created_at_millis IS NULL
       OR p_question_title IS NULL OR p_question_title <> btrim(p_question_title)
       OR char_length(p_question_title) NOT BETWEEN 1 AND 512 OR p_question_title ~ '[[:cntrl:]]'
       OR p_question_description IS NULL OR p_question_description <> btrim(p_question_description)
       OR char_length(p_question_description) NOT BETWEEN 1 AND 4000 OR p_question_description ~ '[[:cntrl:]]'
       OR p_language IS NULL OR p_language <> btrim(p_language)
       OR char_length(p_language) NOT BETWEEN 2 AND 35
       OR NOT ple_api.current_session_account_is_instructor() THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Draft Question save arguments are invalid';
    END IF;
    SELECT question.draft_question_uuid, question.workspace_id, question.draft_question_edit_number
      INTO v_draft_question_uuid, v_workspace_id, v_current_edit_number
      FROM ple_private.draft_question AS question
     WHERE question.reference_number = p_reference_number
       AND ple_api.current_session_account_can_access_authoring_workspace(question.workspace_id)
     FOR UPDATE;
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Draft Question is not available in the current Authoring Workspace';
    END IF;
    IF v_current_edit_number <> p_expected_edit_number THEN
        RAISE EXCEPTION USING ERRCODE = '40001', MESSAGE = 'Draft Question Edit Number is stale';
    END IF;
    SELECT * INTO STRICT v_binding
      FROM ple_private.draft_question_source_binding
     WHERE draft_question_uuid = v_draft_question_uuid
     FOR UPDATE;
    IF NOT (
        (v_binding.backend = 'ple' AND v_binding.question_format = 'pleQuestionJson'
            AND v_binding.webwork_pg_path IS NULL
            AND p_media_type = 'application/vnd.peptidyle.question+json')
        OR (v_binding.backend = 'webwork' AND v_binding.question_format = 'webworkPg'
            AND v_binding.webwork_pg_path IS NOT NULL
            AND p_media_type = 'text/x-wework-pg')
    ) THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Draft Question source media type must match its registered source binding';
    END IF;
    expected_address := pg_catalog.jsonb_build_object(
        'kind', 'workspaceQuestionSource', 'workspace', v_workspace_id, 'object', p_object_id);
    IF p_object_address IS DISTINCT FROM expected_address THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Draft Question source must use its exact Workspace Question Source Object Address';
    END IF;
    INSERT INTO ple_private.object_record(
        object_id, object_address, object_storage_area, object_data_class,
        sha256, size_bytes, media_type, created_at
    ) VALUES (
        p_object_id, expected_address, 'private-content', 'authoring-content', p_sha256,
        p_size_bytes, p_media_type,
        pg_catalog.to_timestamp(p_created_at_millis::double precision / 1000.0));
    UPDATE ple_private.draft_question_source_binding
       SET source_object_id = p_object_id,
           source_object_checksum = pg_catalog.encode(p_sha256, 'hex'),
           updated_at = pg_catalog.clock_timestamp()
     WHERE draft_question_uuid = v_draft_question_uuid;
    UPDATE ple_private.draft_question_metadata
       SET question_title = p_question_title,
           question_description = p_question_description,
           language = p_language,
           updated_at = pg_catalog.clock_timestamp()
     WHERE draft_question_uuid = v_draft_question_uuid;
    UPDATE ple_private.draft_question
       SET draft_question_edit_number = draft_question_edit_number + 1,
           updated_at = pg_catalog.clock_timestamp()
     WHERE draft_question_uuid = v_draft_question_uuid;
END
$$;

REVOKE ALL ON FUNCTION ple_private.ensure_own_authoring_workspace(uuid),
    ple_private.list_authoring_drafts(), ple_private.load_authoring_draft(bigint),
    ple_private.create_authoring_draft(uuid, uuid, uuid, jsonb, bytea, bigint, text, bigint, text, text, text, text),
    ple_private.save_authoring_draft(bigint, bigint, uuid, jsonb, bytea, bigint, text, bigint, text, text, text)
    FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_private.ensure_own_authoring_workspace(uuid),
    ple_private.list_authoring_drafts(), ple_private.load_authoring_draft(bigint),
    ple_private.create_authoring_draft(uuid, uuid, uuid, jsonb, bytea, bigint, text, bigint, text, text, text, text),
    ple_private.save_authoring_draft(bigint, bigint, uuid, jsonb, bytea, bigint, text, bigint, text, text, text)
    TO ple_api_owner;
RESET ROLE;

SET LOCAL ROLE ple_api_owner;
CREATE FUNCTION ple_api.ensure_own_authoring_workspace(p_proposed_workspace_id uuid)
RETURNS uuid LANGUAGE sql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private AS $$
    SELECT ple_private.ensure_own_authoring_workspace(p_proposed_workspace_id)
$$;
CREATE FUNCTION ple_api.list_authoring_drafts()
RETURNS TABLE (
    reference_number bigint, draft_question_edit_number bigint,
    question_title text, question_description text
) LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private AS $$
    SELECT * FROM ple_private.list_authoring_drafts()
$$;
CREATE FUNCTION ple_api.load_authoring_draft(p_reference_number bigint)
RETURNS TABLE (
    draft_question_uuid uuid, workspace_id uuid, reference_number bigint,
    draft_question_edit_number bigint, question_title text, question_description text,
    language text, object_id uuid, object_address jsonb, sha256 bytea,
    size_bytes bigint, media_type text, created_at_millis bigint
) LANGUAGE sql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private AS $$
    SELECT * FROM ple_private.load_authoring_draft(p_reference_number)
$$;
CREATE FUNCTION ple_api.create_authoring_draft(
    p_workspace_id uuid, p_draft_question_uuid uuid, p_object_id uuid,
    p_object_address jsonb, p_sha256 bytea, p_size_bytes bigint, p_media_type text,
    p_created_at_millis bigint, p_question_title text, p_question_description text,
    p_language text, p_webwork_pg_path text
) RETURNS bigint LANGUAGE sql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private AS $$
    SELECT ple_private.create_authoring_draft(
        p_workspace_id, p_draft_question_uuid, p_object_id, p_object_address, p_sha256,
        p_size_bytes, p_media_type, p_created_at_millis, p_question_title,
        p_question_description, p_language, p_webwork_pg_path)
$$;
CREATE FUNCTION ple_api.save_authoring_draft(
    p_reference_number bigint, p_expected_edit_number bigint, p_object_id uuid,
    p_object_address jsonb, p_sha256 bytea, p_size_bytes bigint, p_media_type text,
    p_created_at_millis bigint, p_question_title text, p_question_description text,
    p_language text
) RETURNS void LANGUAGE sql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private AS $$
    SELECT ple_private.save_authoring_draft(
        p_reference_number, p_expected_edit_number, p_object_id, p_object_address, p_sha256,
        p_size_bytes, p_media_type, p_created_at_millis, p_question_title,
        p_question_description, p_language)
$$;
REVOKE ALL ON FUNCTION ple_api.ensure_own_authoring_workspace(uuid),
    ple_api.list_authoring_drafts(), ple_api.load_authoring_draft(bigint),
    ple_api.create_authoring_draft(uuid, uuid, uuid, jsonb, bytea, bigint, text, bigint, text, text, text, text),
    ple_api.save_authoring_draft(bigint, bigint, uuid, jsonb, bytea, bigint, text, bigint, text, text, text)
    FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_api.ensure_own_authoring_workspace(uuid),
    ple_api.list_authoring_drafts(), ple_api.load_authoring_draft(bigint),
    ple_api.create_authoring_draft(uuid, uuid, uuid, jsonb, bytea, bigint, text, bigint, text, text, text, text),
    ple_api.save_authoring_draft(bigint, bigint, uuid, jsonb, bytea, bigint, text, bigint, text, text, text)
    TO ple_app;
RESET ROLE;
