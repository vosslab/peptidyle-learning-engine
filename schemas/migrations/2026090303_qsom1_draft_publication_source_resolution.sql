-- WP-SD1-A-QSOM1-P2: resolve one exact current Draft Question Source Object
-- Record for the server-only Question Publication copy coordinator.

SET LOCAL ROLE ple_private_owner;

-- ASVS 1.2.4, 2.2.1-2.2.3, 2.3.1, 8.2.1-8.2.3, and 8.3.1:
-- all values remain parameters, and the trusted database boundary rechecks
-- current Instructor/workspace authority plus the Draft Question Edit Number
-- before returning only the source fields required for the copy.
CREATE FUNCTION ple_private.load_draft_question_publication_source(
    p_draft_question_uuid uuid,
    p_expected_draft_question_edit_number bigint,
    p_workspace_id uuid
)
RETURNS TABLE (
    object_id uuid,
    object_address jsonb,
    sha256 bytea,
    size_bytes bigint,
    media_type text,
    created_at_millis bigint
)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private AS $$
DECLARE
    v_current_edit_number bigint;
    v_row_count bigint;
BEGIN
    IF p_draft_question_uuid IS NULL
       OR p_expected_draft_question_edit_number IS NULL
       OR p_expected_draft_question_edit_number <= 0
       OR p_workspace_id IS NULL THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Draft Question Publication Source arguments are invalid';
    END IF;
    IF NOT ple_api.current_session_account_is_instructor()
       OR NOT ple_api.current_session_account_can_access_authoring_workspace(p_workspace_id) THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Draft Question Publication Source requires current Authoring Workspace access';
    END IF;

    SELECT question.draft_question_edit_number
      INTO v_current_edit_number
      FROM ple_private.draft_question AS question
     WHERE question.draft_question_uuid = p_draft_question_uuid
       AND question.workspace_id = p_workspace_id;
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Draft Question Publication Source requires its exact Draft Question and workspace';
    END IF;
    IF v_current_edit_number <> p_expected_draft_question_edit_number THEN
        RAISE EXCEPTION USING ERRCODE = '40001',
            MESSAGE = 'Draft Question Publication Source Edit Number is stale';
    END IF;

    RETURN QUERY
    SELECT record.object_id,
           record.object_address,
           record.sha256,
           record.size_bytes,
           record.media_type,
           pg_catalog.round(
               extract(epoch FROM record.created_at) * 1000
           )::bigint
      FROM ple_private.draft_question_source_binding AS binding
      JOIN ple_private.object_record AS record
        ON record.object_id = binding.source_object_id
     WHERE binding.draft_question_uuid = p_draft_question_uuid
       AND binding.source_object_checksum = pg_catalog.encode(record.sha256, 'hex')
       AND record.object_storage_area = 'private-content'
       AND record.object_data_class = 'authoring-content'
       AND record.object_address = pg_catalog.jsonb_build_object(
           'kind', 'workspaceQuestionSource',
           'workspace', p_workspace_id,
           'object', record.object_id
       );
    GET DIAGNOSTICS v_row_count = ROW_COUNT;
    IF v_row_count <> 1 THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Draft Question Publication Source Binding is incomplete';
    END IF;
END
$$;

REVOKE ALL PRIVILEGES ON FUNCTION
    ple_private.load_draft_question_publication_source(uuid, bigint, uuid)
FROM PUBLIC;
GRANT EXECUTE ON FUNCTION
    ple_private.load_draft_question_publication_source(uuid, bigint, uuid)
TO ple_api_owner;

SET LOCAL ROLE ple_api_owner;

CREATE FUNCTION ple_api.load_draft_question_publication_source(
    p_draft_question_uuid uuid,
    p_expected_draft_question_edit_number bigint,
    p_workspace_id uuid
)
RETURNS TABLE (
    object_id uuid,
    object_address jsonb,
    sha256 bytea,
    size_bytes bigint,
    media_type text,
    created_at_millis bigint
)
LANGUAGE sql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private AS $$
    SELECT *
      FROM ple_private.load_draft_question_publication_source(
          p_draft_question_uuid,
          p_expected_draft_question_edit_number,
          p_workspace_id
      )
$$;

REVOKE ALL PRIVILEGES ON FUNCTION
    ple_api.load_draft_question_publication_source(uuid, bigint, uuid)
FROM PUBLIC;
GRANT EXECUTE ON FUNCTION
    ple_api.load_draft_question_publication_source(uuid, bigint, uuid)
TO ple_app;

COMMENT ON FUNCTION
    ple_api.load_draft_question_publication_source(uuid, bigint, uuid)
IS 'Returns only the exact current Workspace Question Source Object Record required by the server-only Question Publication copy coordinator.';

RESET ROLE;
