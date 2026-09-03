-- Register current immutable source-byte evidence against one mutable Draft
-- Question. Source bytes are registered first as an Object Record; this path
-- only binds those bytes and exact closed Question Backend facts.

SET LOCAL ROLE ple_private_owner;

CREATE POLICY draft_question_private_owner_source_lookup ON ple_private.draft_question
    FOR SELECT TO ple_private_owner USING (true);
CREATE POLICY draft_question_private_owner_source_update ON ple_private.draft_question
    FOR UPDATE TO ple_private_owner USING (true) WITH CHECK (true);
CREATE POLICY question_source_registration_private_owner_source_lookup ON ple_private.question_source_registration
    FOR SELECT TO ple_private_owner USING (true);
CREATE POLICY question_source_registration_private_owner_source_registration ON ple_private.question_source_registration
    FOR INSERT TO ple_private_owner WITH CHECK (true);
CREATE POLICY question_source_registration_private_owner_source_replacement ON ple_private.question_source_registration
    FOR UPDATE TO ple_private_owner USING (true) WITH CHECK (true);
CREATE POLICY workspace_import_private_owner_source_lookup ON ple_private.workspace_import
    FOR SELECT TO ple_private_owner USING (true);

-- ASVS 2.1.1, 2.2.1, and 2.3.1: this trusted transaction locks the exact
-- authorized Draft Question and uses its positive Edit Number as a CAS token.
CREATE FUNCTION ple_private.register_draft_question_source_registration(
    p_draft_question_uuid uuid,
    p_expected_draft_question_edit_number bigint,
    p_workspace_id uuid,
    p_backend text,
    p_question_format text,
    p_webwork_pg_path text,
    p_qti_package_item_identifier text,
    p_workspace_import_id uuid,
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
    current_registration ple_private.question_source_registration%ROWTYPE;
    has_registration boolean;
    facts_match boolean;
BEGIN
    IF p_expected_draft_question_edit_number IS NULL
       OR p_expected_draft_question_edit_number <= 0 THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Draft Question Source Registration requires a positive Draft Question Edit Number';
    END IF;
    IF NOT ple_api.current_session_account_can_access_workspace(p_workspace_id) THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Draft Question Source Registration requires current workspace access';
    END IF;
    SELECT question.draft_question_edit_number
      INTO current_edit_number
      FROM ple_private.draft_question AS question
     WHERE question.draft_question_uuid = p_draft_question_uuid
       AND question.workspace_id = p_workspace_id
     FOR UPDATE;
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Draft Question Source Registration must use its exact Draft Question and workspace';
    END IF;
    SELECT registration.*
      INTO current_registration
     FROM ple_private.question_source_registration AS registration
     WHERE registration.draft_question_uuid = p_draft_question_uuid
     FOR UPDATE;
    has_registration := FOUND;
    IF has_registration THEN
        facts_match := current_registration.backend = p_backend
        AND current_registration.question_format = p_question_format
        AND current_registration.webwork_pg_path IS NOT DISTINCT FROM p_webwork_pg_path
        AND current_registration.qti_package_item_identifier IS NOT DISTINCT FROM p_qti_package_item_identifier
        AND current_registration.workspace_import_id IS NOT DISTINCT FROM p_workspace_import_id
        AND current_registration.imathas_deployment_reference IS NOT DISTINCT FROM p_imathas_deployment_reference
        AND current_registration.imathas_item_reference IS NOT DISTINCT FROM p_imathas_item_reference
        AND current_registration.imathas_profile IS NOT DISTINCT FROM p_imathas_profile
        AND current_registration.source_object_id = p_source_object_id
        AND current_registration.source_object_checksum = p_source_object_checksum;
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
            MESSAGE = 'Draft Question Edit Number is stale or Source Registration facts do not match';
    END IF;
    IF has_registration THEN
        UPDATE ple_private.question_source_registration
           SET backend = p_backend, question_format = p_question_format,
               webwork_pg_path = p_webwork_pg_path,
               qti_package_item_identifier = p_qti_package_item_identifier,
               workspace_import_id = p_workspace_import_id,
               imathas_deployment_reference = p_imathas_deployment_reference,
               imathas_item_reference = p_imathas_item_reference,
               imathas_profile = p_imathas_profile, source_object_id = p_source_object_id,
               source_object_checksum = p_source_object_checksum,
               updated_at = pg_catalog.clock_timestamp()
         WHERE draft_question_uuid = p_draft_question_uuid;
    ELSE
        INSERT INTO ple_private.question_source_registration (
            draft_question_uuid, backend, question_format,
            webwork_pg_path, qti_package_item_identifier, workspace_import_id,
            imathas_deployment_reference, imathas_item_reference, imathas_profile,
            source_object_id, source_object_checksum,
            created_at, updated_at
        ) VALUES (
            p_draft_question_uuid, p_backend, p_question_format,
            p_webwork_pg_path, p_qti_package_item_identifier, p_workspace_import_id,
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
REVOKE ALL PRIVILEGES ON FUNCTION ple_private.register_draft_question_source_registration(
    uuid, bigint, uuid, text, text, text, text, uuid, text, text, text, uuid, text
) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_private.register_draft_question_source_registration(
    uuid, bigint, uuid, text, text, text, text, uuid, text, text, text, uuid, text
) TO ple_api_owner;

SET LOCAL ROLE ple_api_owner;
CREATE FUNCTION ple_api.register_draft_question_source_registration(
    p_draft_question_uuid uuid,
    p_expected_draft_question_edit_number bigint,
    p_workspace_id uuid,
    p_backend text,
    p_question_format text,
    p_webwork_pg_path text,
    p_qti_package_item_identifier text,
    p_workspace_import_id uuid,
    p_imathas_deployment_reference text,
    p_imathas_item_reference text,
    p_imathas_profile text,
    p_source_object_id uuid,
    p_source_object_checksum text
)
RETURNS void LANGUAGE sql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private AS $$
    SELECT ple_private.register_draft_question_source_registration(
        p_draft_question_uuid, p_expected_draft_question_edit_number, p_workspace_id,
        p_backend, p_question_format, p_webwork_pg_path,
        p_qti_package_item_identifier, p_workspace_import_id, p_imathas_deployment_reference,
        p_imathas_item_reference, p_imathas_profile, p_source_object_id,
        p_source_object_checksum
    )
$$;
REVOKE ALL PRIVILEGES ON FUNCTION ple_api.register_draft_question_source_registration(
    uuid, bigint, uuid, text, text, text, text, uuid, text, text, text, uuid, text
) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_api.register_draft_question_source_registration(
    uuid, bigint, uuid, text, text, text, text, uuid, text, text, text, uuid, text
) TO ple_app;
SET LOCAL ROLE ple_api_owner;
COMMENT ON FUNCTION ple_api.register_draft_question_source_registration(
    uuid, bigint, uuid, text, text, text, text, uuid, text, text, text, uuid, text
) IS 'Locks one authorized mutable Draft Question; a matching Edit Number applies current Source Registration facts once, identical facts are idempotent, and one exact post-increment retry is accepted.';
RESET ROLE;
