-- Session-authorized binding of immutable Question Source bytes to one Draft
-- Question Revision. Source bytes are registered first as an Object Record.

SET LOCAL ROLE ple_private_owner;

CREATE POLICY draft_question_private_owner_source_lookup ON ple_private.draft_question
    FOR SELECT TO ple_private_owner USING (true);
CREATE POLICY draft_question_revision_private_owner_source_lookup ON ple_private.draft_question_revision
    FOR SELECT TO ple_private_owner USING (true);
CREATE POLICY question_source_private_owner_source_lookup ON ple_private.question_source
    FOR SELECT TO ple_private_owner USING (true);
CREATE POLICY question_source_private_owner_source_registration ON ple_private.question_source
    FOR INSERT TO ple_private_owner WITH CHECK (true);
CREATE POLICY workspace_import_private_owner_source_lookup ON ple_private.workspace_import
    FOR SELECT TO ple_private_owner USING (true);

-- ASVS 1.2.4, 2.1.1, 2.2.1, 2.2.2, 2.3.1, 8.1.1, 8.2.1, and 8.3.1:
-- this is the sole session-authorized path that creates an immutable private
-- Question Source. It validates direct backend fields, rechecks the exact
-- Draft Question Revision workspace, and permits only an identical retry.
CREATE FUNCTION ple_private.register_draft_question_source(
    p_question_source_uuid uuid,
    p_draft_question_uuid uuid,
    p_draft_question_revision_number integer,
    p_workspace_id uuid,
    p_backend text,
    p_question_format text,
    p_question_type text,
    p_webwork_pg_path text,
    p_qti_package_item_identifier text,
    p_workspace_import_id uuid,
    p_imathas_deployment_reference text,
    p_imathas_item_reference text,
    p_imathas_profile text,
    p_source_object_id uuid,
    p_source_object_checksum text,
    p_public_content_checksum text
)
RETURNS uuid LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private AS $$
DECLARE
    resolved_draft_question_revision_uuid uuid;
    resolved_question_source_uuid uuid;
BEGIN
    IF NOT ple_api.current_session_account_can_access_workspace(p_workspace_id) THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Draft Question Source registration requires current workspace access';
    END IF;
    SELECT revision.draft_question_revision_uuid
      INTO resolved_draft_question_revision_uuid
      FROM ple_private.draft_question_revision AS revision
      JOIN ple_private.draft_question AS question
        ON question.draft_question_uuid = revision.draft_question_uuid
     WHERE revision.draft_question_uuid = p_draft_question_uuid
       AND revision.revision_number = p_draft_question_revision_number
       AND question.workspace_id = p_workspace_id;
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Draft Question Source must use its exact Draft Question Revision and workspace';
    END IF;
    IF p_source_object_checksum !~ '^[0-9a-f]{64}$'
       OR p_public_content_checksum !~ '^[0-9a-f]{64}$' THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Question Source checksums must be canonical lowercase SHA-256 values';
    END IF;
    IF p_question_type NOT IN (
        'multipleChoice', 'multipleAnswer', 'fillInBlank', 'multipleFillInBlank',
        'numeric', 'matching', 'ordering', 'hotspot'
    ) THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Question Source must use a supported Question Type';
    END IF;
    IF (p_backend = 'ple' AND p_question_format NOT IN ('pleQuestionJson', 'pleAlgorithmic'))
       OR (p_backend = 'webwork' AND p_question_format <> 'webworkPg')
       OR (p_backend = 'qti' AND p_question_format <> 'qti')
       OR (p_backend = 'imathas' AND p_question_format <> 'imathas')
       OR p_backend NOT IN ('ple', 'webwork', 'qti', 'imathas') THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Question Source Format must be supported by its Question Backend';
    END IF;
    IF NOT COALESCE((
        (p_backend = 'ple'
            AND p_webwork_pg_path IS NULL AND p_qti_package_item_identifier IS NULL
            AND p_workspace_import_id IS NULL AND p_imathas_deployment_reference IS NULL
            AND p_imathas_item_reference IS NULL AND p_imathas_profile IS NULL)
        OR (p_backend = 'webwork'
            AND char_length(btrim(p_webwork_pg_path)) BETWEEN 1 AND 1000
            AND p_qti_package_item_identifier IS NULL AND p_workspace_import_id IS NULL
            AND p_imathas_deployment_reference IS NULL AND p_imathas_item_reference IS NULL
            AND p_imathas_profile IS NULL)
        OR (p_backend = 'qti'
            AND p_webwork_pg_path IS NULL
            AND char_length(btrim(p_qti_package_item_identifier)) BETWEEN 1 AND 1000
            AND p_workspace_import_id IS NOT NULL
            AND p_imathas_deployment_reference IS NULL AND p_imathas_item_reference IS NULL
            AND p_imathas_profile IS NULL
            AND EXISTS (
                SELECT 1 FROM ple_private.workspace_import AS workspace_import
                 WHERE workspace_import.workspace_id = p_workspace_id
                   AND workspace_import.import_id = p_workspace_import_id
            ))
        OR (p_backend = 'imathas'
            AND p_webwork_pg_path IS NULL AND p_qti_package_item_identifier IS NULL
            AND p_workspace_import_id IS NULL
            AND char_length(btrim(p_imathas_deployment_reference)) BETWEEN 1 AND 255
            AND char_length(btrim(p_imathas_item_reference)) BETWEEN 1 AND 255
            AND p_imathas_profile IS NULL)
    ), false) THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Question Source must use exactly the fields for its Question Backend';
    END IF;

    INSERT INTO ple_private.question_source (
        question_source_uuid, draft_question_revision_uuid, backend, question_format,
        question_type, webwork_pg_path, qti_package_item_identifier, workspace_import_id,
        imathas_deployment_reference, imathas_item_reference, imathas_profile,
        source_object_id, source_object_checksum, public_content_checksum, created_at, updated_at
    ) VALUES (
        p_question_source_uuid, resolved_draft_question_revision_uuid, p_backend, p_question_format,
        p_question_type, p_webwork_pg_path, p_qti_package_item_identifier, p_workspace_import_id,
        p_imathas_deployment_reference, p_imathas_item_reference, p_imathas_profile,
        p_source_object_id, p_source_object_checksum, p_public_content_checksum,
        CURRENT_TIMESTAMP, CURRENT_TIMESTAMP
    ) ON CONFLICT DO NOTHING
    RETURNING question_source_uuid INTO resolved_question_source_uuid;
    IF FOUND THEN
        RETURN resolved_question_source_uuid;
    END IF;

    SELECT source.question_source_uuid
      INTO resolved_question_source_uuid
      FROM ple_private.question_source AS source
     WHERE source.draft_question_revision_uuid = resolved_draft_question_revision_uuid
       AND source.backend = p_backend
       AND source.question_format = p_question_format
       AND source.question_type = p_question_type
       AND source.webwork_pg_path IS NOT DISTINCT FROM p_webwork_pg_path
       AND source.qti_package_item_identifier IS NOT DISTINCT FROM p_qti_package_item_identifier
       AND source.workspace_import_id IS NOT DISTINCT FROM p_workspace_import_id
       AND source.imathas_deployment_reference IS NOT DISTINCT FROM p_imathas_deployment_reference
       AND source.imathas_item_reference IS NOT DISTINCT FROM p_imathas_item_reference
       AND source.imathas_profile IS NOT DISTINCT FROM p_imathas_profile
       AND source.source_object_id = p_source_object_id
       AND source.source_object_checksum = p_source_object_checksum
       AND source.public_content_checksum = p_public_content_checksum;
    IF FOUND THEN
        RETURN resolved_question_source_uuid;
    END IF;
    RAISE EXCEPTION USING ERRCODE = '23505',
        MESSAGE = 'Draft Question Revision or Question Source identity already names different immutable facts';
END
$$;

REVOKE ALL PRIVILEGES ON FUNCTION ple_private.register_draft_question_source(
    uuid, uuid, integer, uuid, text, text, text, text, text, uuid, text, text, text, uuid, text, text
) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_private.register_draft_question_source(
    uuid, uuid, integer, uuid, text, text, text, text, text, uuid, text, text, text, uuid, text, text
) TO ple_api_owner;

SET LOCAL ROLE ple_api_owner;

CREATE FUNCTION ple_api.register_draft_question_source(
    p_question_source_uuid uuid,
    p_draft_question_uuid uuid,
    p_draft_question_revision_number integer,
    p_workspace_id uuid,
    p_backend text,
    p_question_format text,
    p_question_type text,
    p_webwork_pg_path text,
    p_qti_package_item_identifier text,
    p_workspace_import_id uuid,
    p_imathas_deployment_reference text,
    p_imathas_item_reference text,
    p_imathas_profile text,
    p_source_object_id uuid,
    p_source_object_checksum text,
    p_public_content_checksum text
)
RETURNS uuid LANGUAGE sql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private AS $$
    SELECT ple_private.register_draft_question_source(
        p_question_source_uuid, p_draft_question_uuid, p_draft_question_revision_number, p_workspace_id,
        p_backend, p_question_format, p_question_type, p_webwork_pg_path,
        p_qti_package_item_identifier, p_workspace_import_id, p_imathas_deployment_reference,
        p_imathas_item_reference, p_imathas_profile, p_source_object_id,
        p_source_object_checksum, p_public_content_checksum
    )
$$;

REVOKE ALL PRIVILEGES ON FUNCTION ple_api.register_draft_question_source(
    uuid, uuid, integer, uuid, text, text, text, text, text, uuid, text, text, text, uuid, text, text
) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_api.register_draft_question_source(
    uuid, uuid, integer, uuid, text, text, text, text, text, uuid, text, text, text, uuid, text, text
) TO ple_app;

COMMENT ON FUNCTION ple_api.register_draft_question_source(
    uuid, uuid, integer, uuid, text, text, text, text, text, uuid, text, text, text, uuid, text, text
) IS 'Resolves one exact Draft Question Revision within its authorized Authoring Workspace, then binds it to immutable Question Source bytes and exact Question Backend fields.';

RESET ROLE;
