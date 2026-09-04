-- Bind current immutable source-byte evidence against one mutable Draft
-- Question. Source bytes are recorded first as an Object Record; this path
-- only binds those bytes and exact closed Question Backend facts.

SET LOCAL ROLE ple_private_owner;

CREATE POLICY draft_question_private_owner_source_lookup ON ple_private.draft_question
    FOR SELECT TO ple_private_owner USING (true);
CREATE POLICY draft_question_private_owner_source_update ON ple_private.draft_question
    FOR UPDATE TO ple_private_owner USING (true) WITH CHECK (true);
CREATE POLICY workspace_import_private_owner_source_lookup ON ple_private.workspace_import
    FOR SELECT TO ple_private_owner USING (true);

-- ASVS 2.1.1, 2.2.1, and 2.3.1: this trusted transaction locks the exact
-- authorized Draft Question and uses its positive Edit Number as a CAS token.
CREATE FUNCTION ple_private.bind_draft_question_source(
    p_draft_question_uuid uuid,
    p_expected_draft_question_edit_number bigint,
    p_workspace_id uuid,
    p_backend text,
    p_question_format text,
    p_webwork_pg_path text,
    p_imathas_deployment_reference text,
    p_imathas_item_reference text,
    p_imathas_profile text,
    p_source_object_id uuid,
    p_source_object_checksum text
)
RETURNS void LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private AS $$
DECLARE
    current_edit_number bigint;
    current_binding ple_private.draft_question_source_binding%ROWTYPE;
    has_binding boolean;
    facts_match boolean;
BEGIN
    IF p_expected_draft_question_edit_number IS NULL
       OR p_expected_draft_question_edit_number <= 0 THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Draft Question Source Binding requires a positive Draft Question Edit Number';
    END IF;
    IF NOT ple_api.current_session_account_can_access_authoring_workspace(p_workspace_id) THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Draft Question Source Binding requires current Authoring Workspace access';
    END IF;
    SELECT question.draft_question_edit_number
      INTO current_edit_number
      FROM ple_private.draft_question AS question
     WHERE question.draft_question_uuid = p_draft_question_uuid
       AND question.workspace_id = p_workspace_id
     FOR UPDATE;
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Draft Question Source Binding must use its exact Draft Question and workspace';
    END IF;
    SELECT binding.*
      INTO current_binding
     FROM ple_private.draft_question_source_binding AS binding
     WHERE binding.draft_question_uuid = p_draft_question_uuid
     FOR UPDATE;
    has_binding := FOUND;
    IF has_binding THEN
        facts_match := current_binding.backend = p_backend
        AND current_binding.question_format = p_question_format
        AND current_binding.webwork_pg_path IS NOT DISTINCT FROM p_webwork_pg_path
        AND current_binding.imathas_deployment_reference IS NOT DISTINCT FROM p_imathas_deployment_reference
        AND current_binding.imathas_item_reference IS NOT DISTINCT FROM p_imathas_item_reference
        AND current_binding.imathas_profile IS NOT DISTINCT FROM p_imathas_profile
        AND current_binding.source_object_id = p_source_object_id
        AND current_binding.source_object_checksum = p_source_object_checksum;
    ELSE
        facts_match := false;
    END IF;
    IF facts_match THEN
        IF current_edit_number = p_expected_draft_question_edit_number THEN
            RETURN;
        END IF;
        IF current_edit_number - p_expected_draft_question_edit_number = 1 THEN
            RETURN;
        END IF;
    END IF;
    IF current_edit_number <> p_expected_draft_question_edit_number THEN
        RAISE EXCEPTION USING ERRCODE = '40001',
            MESSAGE = 'Draft Question Edit Number is stale or Source Binding facts do not match';
    END IF;
    IF has_binding THEN
        UPDATE ple_private.draft_question_source_binding
           SET backend = p_backend, question_format = p_question_format,
               webwork_pg_path = p_webwork_pg_path,
               imathas_deployment_reference = p_imathas_deployment_reference,
               imathas_item_reference = p_imathas_item_reference,
               imathas_profile = p_imathas_profile, source_object_id = p_source_object_id,
               source_object_checksum = p_source_object_checksum,
               updated_at = pg_catalog.clock_timestamp()
         WHERE draft_question_uuid = p_draft_question_uuid;
    ELSE
        INSERT INTO ple_private.draft_question_source_binding (
            draft_question_uuid, backend, question_format,
            webwork_pg_path,
            imathas_deployment_reference, imathas_item_reference, imathas_profile,
            source_object_id, source_object_checksum,
            created_at, updated_at
        ) VALUES (
            p_draft_question_uuid, p_backend, p_question_format,
            p_webwork_pg_path,
            p_imathas_deployment_reference, p_imathas_item_reference, p_imathas_profile,
            p_source_object_id, p_source_object_checksum,
            pg_catalog.clock_timestamp(), pg_catalog.clock_timestamp()
        );
    END IF;
    UPDATE ple_private.draft_question
       SET draft_question_edit_number = draft_question_edit_number + 1,
           updated_at = pg_catalog.clock_timestamp()
     WHERE draft_question_uuid = p_draft_question_uuid;
END
$$;
REVOKE ALL PRIVILEGES ON FUNCTION ple_private.bind_draft_question_source(
    uuid, bigint, uuid, text, text, text, text, text, text, uuid, text
) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_private.bind_draft_question_source(
    uuid, bigint, uuid, text, text, text, text, text, text, uuid, text
) TO ple_api_owner;

SET LOCAL ROLE ple_api_owner;
CREATE FUNCTION ple_api.bind_draft_question_source(
    p_draft_question_uuid uuid,
    p_expected_draft_question_edit_number bigint,
    p_workspace_id uuid,
    p_backend text,
    p_question_format text,
    p_webwork_pg_path text,
    p_imathas_deployment_reference text,
    p_imathas_item_reference text,
    p_imathas_profile text,
    p_source_object_id uuid,
    p_source_object_checksum text
)
RETURNS void LANGUAGE sql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private AS $$
    SELECT ple_private.bind_draft_question_source(
        p_draft_question_uuid, p_expected_draft_question_edit_number, p_workspace_id,
        p_backend, p_question_format, p_webwork_pg_path, p_imathas_deployment_reference,
        p_imathas_item_reference, p_imathas_profile, p_source_object_id,
        p_source_object_checksum
    )
$$;
REVOKE ALL PRIVILEGES ON FUNCTION ple_api.bind_draft_question_source(
    uuid, bigint, uuid, text, text, text, text, text, text, uuid, text
) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_api.bind_draft_question_source(
    uuid, bigint, uuid, text, text, text, text, text, text, uuid, text
) TO ple_app;
SET LOCAL ROLE ple_api_owner;
COMMENT ON FUNCTION ple_api.bind_draft_question_source(
    uuid, bigint, uuid, text, text, text, text, text, text, uuid, text
) IS 'Locks one authorized mutable Draft Question; a matching Edit Number applies current Source Binding facts once, identical facts preserve the existing Binding, and one exact post-increment retry is accepted.';
RESET ROLE;
