-- M6 live Authoring Workspace operations. Browser requests name only an
-- authorized Draft Question Reference; all UUIDs and Object Addresses stay in
-- the server and are rechecked by these session-bound procedures.

SET LOCAL ROLE ple_private_owner;

-- The definer procedure below owns only first-workspace discovery and creation.
-- Its private owner remains subject to forced RLS, so grant no broader runtime
-- identity and no update/delete path for Authoring Workspaces.
CREATE POLICY authoring_workspace_private_owner_authoring_lookup
    ON ple_private.authoring_workspace
    FOR SELECT TO ple_private_owner USING (true);
CREATE POLICY authoring_workspace_private_owner_authoring_create
    ON ple_private.authoring_workspace
    FOR INSERT TO ple_private_owner WITH CHECK (true);
-- Existing private publication/source procedures can read and update a Draft
-- Question. First authoring also needs exactly the INSERT half of that path.
CREATE POLICY draft_question_private_owner_authoring_create
    ON ple_private.draft_question
    FOR INSERT TO ple_private_owner WITH CHECK (true);

CREATE FUNCTION ple_private.ensure_own_authoring_workspace(p_proposed_workspace_id uuid)
RETURNS uuid LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private AS $$
DECLARE
    v_account_id uuid;
    v_workspace_id uuid;
BEGIN
    IF p_proposed_workspace_id IS NULL
       OR NOT ple_api.current_session_account_is_instructor() THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Authoring Workspace requires a current Instructor Account';
    END IF;
    v_account_id := ple_api.current_session_account_id();
    PERFORM pg_catalog.pg_advisory_xact_lock(
        pg_catalog.hashtextextended(v_account_id::text, 0)
    );
    SELECT workspace.workspace_id
      INTO v_workspace_id
      FROM ple_private.authoring_workspace AS workspace
     WHERE workspace.authoring_workspace_owner_account_id = v_account_id
       AND workspace.revoked_at IS NULL
     ORDER BY workspace.created_at, workspace.workspace_id
     LIMIT 1;
    IF v_workspace_id IS NULL THEN
        INSERT INTO ple_private.authoring_workspace (
            workspace_id, authoring_workspace_owner_account_id, created_at
        ) VALUES (p_proposed_workspace_id, v_account_id, pg_catalog.clock_timestamp());
        v_workspace_id := p_proposed_workspace_id;
    END IF;
    RETURN v_workspace_id;
END
$$;

CREATE FUNCTION ple_private.list_authoring_drafts()
RETURNS TABLE (
    reference_number bigint,
    draft_question_edit_number bigint,
    question_title text,
    question_description text
)
LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private AS $$
    SELECT question.reference_number,
           question.draft_question_edit_number,
           metadata.question_title,
           metadata.question_description
      FROM ple_private.draft_question AS question
      JOIN ple_private.draft_question_metadata AS metadata
        ON metadata.draft_question_uuid = question.draft_question_uuid
     WHERE ple_api.current_session_account_is_instructor()
       AND ple_api.current_session_account_can_access_authoring_workspace(question.workspace_id)
     ORDER BY question.updated_at DESC, question.reference_number DESC
$$;

CREATE FUNCTION ple_private.load_authoring_draft(p_reference_number bigint)
RETURNS TABLE (
    draft_question_uuid uuid,
    workspace_id uuid,
    reference_number bigint,
    draft_question_edit_number bigint,
    question_title text,
    question_description text,
    object_id uuid,
    object_address jsonb,
    sha256 bytea,
    size_bytes bigint,
    media_type text,
    created_at_millis bigint
)
LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private AS $$
    SELECT question.draft_question_uuid,
           question.workspace_id,
           question.reference_number,
           question.draft_question_edit_number,
           metadata.question_title,
           metadata.question_description,
           record.object_id,
           record.object_address,
           record.sha256,
           record.size_bytes,
           record.media_type,
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
           'kind', 'workspaceQuestionSource',
           'workspace', question.workspace_id,
           'object', record.object_id
       )
$$;

CREATE FUNCTION ple_private.create_authoring_draft(
    p_workspace_id uuid,
    p_draft_question_uuid uuid,
    p_object_id uuid,
    p_object_address jsonb,
    p_sha256 bytea,
    p_size_bytes bigint,
    p_media_type text,
    p_created_at_millis bigint,
    p_question_title text,
    p_question_description text
)
RETURNS bigint LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private AS $$
DECLARE
    v_reference_number bigint;
    v_created_at timestamp with time zone;
    v_expected_address jsonb;
BEGIN
    IF p_workspace_id IS NULL OR p_draft_question_uuid IS NULL OR p_object_id IS NULL
       OR p_sha256 IS NULL OR pg_catalog.octet_length(p_sha256) <> 32
       OR p_size_bytes IS NULL OR p_size_bytes < 0
       OR p_media_type <> 'application/vnd.peptidyle.question+json'
       OR p_created_at_millis IS NULL
       OR NOT ple_private.question_metadata_fields_are_valid(
           p_question_title, p_question_description
       )
       OR NOT ple_api.current_session_account_is_instructor()
       OR NOT ple_api.current_session_account_is_authoring_workspace_owner(p_workspace_id) THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Draft Question creation arguments are invalid';
    END IF;
    v_expected_address := pg_catalog.jsonb_build_object(
        'kind', 'workspaceQuestionSource', 'workspace', p_workspace_id, 'object', p_object_id
    );
    IF p_object_address IS DISTINCT FROM v_expected_address THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Draft Question source must use its exact Workspace Question Source Object Address';
    END IF;
    v_created_at := pg_catalog.to_timestamp(p_created_at_millis::double precision / 1000.0);
    INSERT INTO ple_private.draft_question (
        draft_question_uuid, workspace_id, created_at, updated_at
    ) VALUES (
        p_draft_question_uuid, p_workspace_id,
        pg_catalog.clock_timestamp(), pg_catalog.clock_timestamp()
    ) RETURNING reference_number INTO v_reference_number;
    INSERT INTO ple_private.draft_question_metadata (
        draft_question_uuid, question_title, question_description, created_at, updated_at
    ) VALUES (
        p_draft_question_uuid, p_question_title, p_question_description,
        pg_catalog.clock_timestamp(), pg_catalog.clock_timestamp()
    );
    INSERT INTO ple_private.object_record (
        object_id, object_address, object_storage_area, object_data_class,
        sha256, size_bytes, media_type, created_at
    ) VALUES (
        p_object_id, v_expected_address, 'private-content', 'authoring-content',
        p_sha256, p_size_bytes, p_media_type, v_created_at
    );
    INSERT INTO ple_private.draft_question_source_binding (
        draft_question_uuid, backend, question_format, source_object_id,
        source_object_checksum, created_at, updated_at
    ) VALUES (
        p_draft_question_uuid, 'ple', 'pleQuestionJson', p_object_id,
        pg_catalog.encode(p_sha256, 'hex'), pg_catalog.clock_timestamp(), pg_catalog.clock_timestamp()
    );
    RETURN v_reference_number;
END
$$;

CREATE FUNCTION ple_private.save_authoring_draft(
    p_reference_number bigint,
    p_expected_edit_number bigint,
    p_object_id uuid,
    p_object_address jsonb,
    p_sha256 bytea,
    p_size_bytes bigint,
    p_media_type text,
    p_created_at_millis bigint,
    p_question_title text,
    p_question_description text
)
RETURNS void LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private AS $$
DECLARE
    v_draft_question_uuid uuid;
    v_workspace_id uuid;
    v_current_edit_number bigint;
    v_expected_address jsonb;
BEGIN
    IF p_reference_number IS NULL OR p_reference_number NOT BETWEEN 1 AND 2147483647
       OR p_expected_edit_number IS NULL OR p_expected_edit_number <= 0
       OR p_object_id IS NULL OR p_sha256 IS NULL OR pg_catalog.octet_length(p_sha256) <> 32
       OR p_size_bytes IS NULL OR p_size_bytes < 0
       OR p_media_type <> 'application/vnd.peptidyle.question+json'
       OR p_created_at_millis IS NULL
       OR NOT ple_private.question_metadata_fields_are_valid(
           p_question_title, p_question_description
       )
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
        RAISE EXCEPTION USING ERRCODE = '40001',
            MESSAGE = 'Draft Question Edit Number is stale';
    END IF;
    v_expected_address := pg_catalog.jsonb_build_object(
        'kind', 'workspaceQuestionSource', 'workspace', v_workspace_id, 'object', p_object_id
    );
    IF p_object_address IS DISTINCT FROM v_expected_address THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Draft Question source must use its exact Workspace Question Source Object Address';
    END IF;
    INSERT INTO ple_private.object_record (
        object_id, object_address, object_storage_area, object_data_class,
        sha256, size_bytes, media_type, created_at
    ) VALUES (
        p_object_id, v_expected_address, 'private-content', 'authoring-content', p_sha256,
        p_size_bytes, p_media_type,
        pg_catalog.to_timestamp(p_created_at_millis::double precision / 1000.0)
    );
    UPDATE ple_private.draft_question_source_binding
       SET source_object_id = p_object_id,
           source_object_checksum = pg_catalog.encode(p_sha256, 'hex'),
           updated_at = pg_catalog.clock_timestamp()
     WHERE draft_question_uuid = v_draft_question_uuid;
    UPDATE ple_private.draft_question_metadata
       SET question_title = p_question_title,
           question_description = p_question_description,
           updated_at = pg_catalog.clock_timestamp()
     WHERE draft_question_uuid = v_draft_question_uuid;
    UPDATE ple_private.draft_question
       SET draft_question_edit_number = draft_question_edit_number + 1,
           updated_at = pg_catalog.clock_timestamp()
     WHERE draft_question_uuid = v_draft_question_uuid;
END
$$;

REVOKE ALL PRIVILEGES ON FUNCTION ple_private.ensure_own_authoring_workspace(uuid) FROM PUBLIC;
REVOKE ALL PRIVILEGES ON FUNCTION ple_private.list_authoring_drafts() FROM PUBLIC;
REVOKE ALL PRIVILEGES ON FUNCTION ple_private.load_authoring_draft(bigint) FROM PUBLIC;
REVOKE ALL PRIVILEGES ON FUNCTION ple_private.create_authoring_draft(
    uuid, uuid, uuid, jsonb, bytea, bigint, text, bigint, text, text
) FROM PUBLIC;
REVOKE ALL PRIVILEGES ON FUNCTION ple_private.save_authoring_draft(
    bigint, bigint, uuid, jsonb, bytea, bigint, text, bigint, text, text
) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_private.ensure_own_authoring_workspace(uuid) TO ple_api_owner;
GRANT EXECUTE ON FUNCTION ple_private.list_authoring_drafts() TO ple_api_owner;
GRANT EXECUTE ON FUNCTION ple_private.load_authoring_draft(bigint) TO ple_api_owner;
GRANT EXECUTE ON FUNCTION ple_private.create_authoring_draft(
    uuid, uuid, uuid, jsonb, bytea, bigint, text, bigint, text, text
) TO ple_api_owner;
GRANT EXECUTE ON FUNCTION ple_private.save_authoring_draft(
    bigint, bigint, uuid, jsonb, bytea, bigint, text, bigint, text, text
) TO ple_api_owner;

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
)
LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private AS $$
    SELECT * FROM ple_private.list_authoring_drafts()
$$;
CREATE FUNCTION ple_api.load_authoring_draft(p_reference_number bigint)
RETURNS TABLE (
    draft_question_uuid uuid, workspace_id uuid, reference_number bigint,
    draft_question_edit_number bigint, question_title text, question_description text,
    object_id uuid, object_address jsonb, sha256 bytea, size_bytes bigint,
    media_type text, created_at_millis bigint
)
LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private AS $$
    SELECT * FROM ple_private.load_authoring_draft(p_reference_number)
$$;
CREATE FUNCTION ple_api.create_authoring_draft(
    p_workspace_id uuid, p_draft_question_uuid uuid, p_object_id uuid,
    p_object_address jsonb, p_sha256 bytea, p_size_bytes bigint, p_media_type text,
    p_created_at_millis bigint, p_question_title text, p_question_description text
)
RETURNS bigint LANGUAGE sql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private AS $$
    SELECT ple_private.create_authoring_draft(
        p_workspace_id, p_draft_question_uuid, p_object_id, p_object_address, p_sha256,
        p_size_bytes, p_media_type, p_created_at_millis, p_question_title, p_question_description
    )
$$;
CREATE FUNCTION ple_api.save_authoring_draft(
    p_reference_number bigint, p_expected_edit_number bigint, p_object_id uuid,
    p_object_address jsonb, p_sha256 bytea, p_size_bytes bigint, p_media_type text,
    p_created_at_millis bigint, p_question_title text, p_question_description text
)
RETURNS void LANGUAGE sql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private AS $$
    SELECT ple_private.save_authoring_draft(
        p_reference_number, p_expected_edit_number, p_object_id, p_object_address, p_sha256,
        p_size_bytes, p_media_type, p_created_at_millis, p_question_title, p_question_description
    )
$$;

REVOKE ALL PRIVILEGES ON FUNCTION ple_api.ensure_own_authoring_workspace(uuid) FROM PUBLIC;
REVOKE ALL PRIVILEGES ON FUNCTION ple_api.list_authoring_drafts() FROM PUBLIC;
REVOKE ALL PRIVILEGES ON FUNCTION ple_api.load_authoring_draft(bigint) FROM PUBLIC;
REVOKE ALL PRIVILEGES ON FUNCTION ple_api.create_authoring_draft(
    uuid, uuid, uuid, jsonb, bytea, bigint, text, bigint, text, text
) FROM PUBLIC;
REVOKE ALL PRIVILEGES ON FUNCTION ple_api.save_authoring_draft(
    bigint, bigint, uuid, jsonb, bytea, bigint, text, bigint, text, text
) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_api.ensure_own_authoring_workspace(uuid) TO ple_app;
GRANT EXECUTE ON FUNCTION ple_api.list_authoring_drafts() TO ple_app;
GRANT EXECUTE ON FUNCTION ple_api.load_authoring_draft(bigint) TO ple_app;
GRANT EXECUTE ON FUNCTION ple_api.create_authoring_draft(
    uuid, uuid, uuid, jsonb, bytea, bigint, text, bigint, text, text
) TO ple_app;
GRANT EXECUTE ON FUNCTION ple_api.save_authoring_draft(
    bigint, bigint, uuid, jsonb, bytea, bigint, text, bigint, text, text
) TO ple_app;

COMMENT ON FUNCTION ple_api.ensure_own_authoring_workspace(uuid) IS
    'Returns the current Instructor Authoring Workspace, creating exactly one owner workspace when absent.';
COMMENT ON FUNCTION ple_api.load_authoring_draft(bigint) IS
    'Resolves one current private Draft Question only through the active Authoring Workspace relationship.';
COMMENT ON FUNCTION ple_api.save_authoring_draft(bigint, bigint, uuid, jsonb, bytea, bigint, text, bigint, text, text) IS
    'Atomically replaces a private Draft Question Source and metadata at one exact Draft Question Edit Number.';

RESET ROLE;
