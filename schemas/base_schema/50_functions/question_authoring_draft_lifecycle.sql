-- Draft workspace, creation, saves, and deletion, with their API entry points.

SET LOCAL ROLE ple_private_owner;

-- A Draft is current private authoring state. Record metadata belongs to the
-- Draft aggregate and is copied into its immutable Revision at publication.
CREATE FUNCTION ple_private.ensure_own_authoring_workspace(p_proposed_workspace_id uuid)
RETURNS uuid LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private AS $$
DECLARE
    account_id text;
    authoring_workspace_id uuid;
BEGIN
    IF p_proposed_workspace_id IS NULL
       OR NOT (ple_api.current_session_account_is_instructor()
               OR ple_api.current_session_account_is_sysadmin()) THEN
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
        VALUES (p_proposed_workspace_id, account_id, pg_catalog.transaction_timestamp());
        authoring_workspace_id := p_proposed_workspace_id;
    END IF;
    RETURN authoring_workspace_id;
END
$$;

CREATE FUNCTION ple_private.create_authoring_draft(
    p_authoring_workspace_id uuid, p_draft_question_uuid uuid, p_object_record_id uuid,
    p_object_address jsonb, p_sha256 bytea, p_size_bytes bigint, p_media_type text,
    p_created_at_millis bigint, p_question_title text, p_question_description text,
    p_tags text[], p_question_license text, p_citation_text text,
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
       OR char_length(p_question_title) NOT BETWEEN 0 AND 512 OR p_question_title ~ '[[:cntrl:]]'
       OR p_question_description IS NULL OR p_question_description <> btrim(p_question_description)
       OR char_length(p_question_description) NOT BETWEEN 0 AND 4000 OR p_question_description ~ '[[:cntrl:]]'
       OR p_tags IS NULL OR NOT ple_data.question_metadata_tags_are_valid(p_tags)
       OR (p_question_license IS NOT NULL AND p_question_license NOT IN ('CC0-1.0', 'CC-BY-4.0', 'CC-BY-SA-4.0'))
       OR (p_language IS NOT NULL AND (p_language <> btrim(p_language)
           OR char_length(p_language) NOT BETWEEN 2 AND 35))
       OR (p_question_type IS NOT NULL AND p_question_type NOT IN ('multipleChoice', 'multipleAnswer', 'fillInBlank', 'multipleFillInBlank',
           'numeric', 'matching', 'ordering', 'hotspot')
       )
       OR NOT (ple_api.current_session_account_is_instructor()
               OR ple_api.current_session_account_is_sysadmin())
       OR NOT ple_api.current_session_account_is_authoring_workspace_owner(p_authoring_workspace_id) THEN
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
        draft_question_id, question_title, question_description, tags, question_license,
        citation_text, language, created_at, updated_at
    ) VALUES (
        p_draft_question_uuid, p_question_title, p_question_description, p_tags,
        p_question_license::ple_data.license_spdx, p_citation_text, p_language,
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
    p_created_at_millis bigint, p_question_type text
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
       OR (p_question_type IS NOT NULL AND p_question_type NOT IN ('multipleChoice', 'multipleAnswer', 'fillInBlank', 'multipleFillInBlank',
           'numeric', 'matching', 'ordering', 'hotspot')
       )
       OR NOT (ple_api.current_session_account_is_instructor()
               OR ple_api.current_session_account_is_sysadmin()) THEN
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
    -- ASVS 1.2.4, 2.2.2: bind raw source bytes to the registered media type;
    -- source syntax and image references are validated during preview/publication.
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
           question_type = p_question_type::ple_data.question_type,
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
-- dynamic backend feedback.  Hint and Worked Solution use the same boundary.
-- p_replace_support false leaves those two columns unchanged, so an ordinary
-- general-feedback save cannot wipe them.  A Draft edit advances the ordinary
-- Draft CAS; publication then records the exact text on a new immutable Revision.
-- p_replace_question_type plus nullable p_question_type means preserve, clear, or set.
-- ASVS 2.2.1 and 2.2.2: each text is trimmed, 1-4000 characters, and free of controls.
-- ASVS 8.2.3: this function does not copy WeBWorK source into these columns.
CREATE FUNCTION ple_private.save_authoring_draft_general_feedback(
    p_draft_question_uuid uuid,
    p_expected_draft_question_edit_number bigint,
    p_general_feedback text,
    p_hint text,
    p_worked_solution text,
    p_replace_support boolean,
    p_question_title text,
    p_question_description text,
    p_tags text[],
    p_question_license text,
    p_citation_text text,
    p_language text,
    p_replace_question_type boolean,
    p_question_type text
) RETURNS TABLE (draft_question_edit_number bigint, general_feedback text)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private AS $$
DECLARE
    v_draft_question_uuid uuid;
    v_current_draft_question_edit_number bigint;
    v_question_backend ple_data.question_backend;
BEGIN
    IF p_draft_question_uuid IS NULL
       OR p_expected_draft_question_edit_number IS NULL OR p_expected_draft_question_edit_number <= 0
       OR p_replace_support IS NULL
       OR p_replace_question_type IS NULL
       OR (p_question_type IS NOT NULL AND p_question_type NOT IN (
           'multipleChoice', 'multipleAnswer', 'fillInBlank', 'multipleFillInBlank',
           'numeric', 'matching', 'ordering', 'hotspot'))
       OR (NOT p_replace_question_type AND p_question_type IS NOT NULL)
       OR (p_general_feedback IS NOT NULL AND (
           p_general_feedback <> btrim(p_general_feedback)
           OR char_length(p_general_feedback) NOT BETWEEN 1 AND 4000
           OR p_general_feedback ~ '[[:cntrl:]]'
       ))
       OR (p_hint IS NOT NULL AND (
           p_hint <> btrim(p_hint)
           OR char_length(p_hint) NOT BETWEEN 1 AND 4000
           OR p_hint ~ '[[:cntrl:]]'
       ))
       OR (p_worked_solution IS NOT NULL AND (
           p_worked_solution <> btrim(p_worked_solution)
           OR char_length(p_worked_solution) NOT BETWEEN 1 AND 4000
           OR p_worked_solution ~ '[[:cntrl:]]'
       ))
       OR p_question_title IS NULL OR p_question_title <> btrim(p_question_title)
       OR char_length(p_question_title) NOT BETWEEN 0 AND 512 OR p_question_title ~ '[[:cntrl:]]'
       OR p_question_description IS NULL OR p_question_description <> btrim(p_question_description)
       OR char_length(p_question_description) NOT BETWEEN 0 AND 4000 OR p_question_description ~ '[[:cntrl:]]'
       OR p_tags IS NULL OR NOT ple_data.question_metadata_tags_are_valid(p_tags)
       OR (p_question_license IS NOT NULL AND p_question_license NOT IN ('CC0-1.0', 'CC-BY-4.0', 'CC-BY-SA-4.0'))
       OR (p_language IS NOT NULL AND (p_language <> btrim(p_language)
           OR char_length(p_language) NOT BETWEEN 2 AND 35))
       OR NOT (ple_api.current_session_account_is_instructor()
               OR ple_api.current_session_account_is_sysadmin()) THEN
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
    IF p_replace_question_type THEN
        SELECT backend INTO v_question_backend
          FROM ple_private.draft_question_source_binding
         WHERE draft_question_id = v_draft_question_uuid
         FOR UPDATE;
        IF NOT FOUND OR v_question_backend <> 'webwork'::ple_data.question_backend THEN
            RAISE EXCEPTION USING ERRCODE = '22023',
                MESSAGE = 'Manual Question Type updates require a WebWork Draft';
        END IF;
    END IF;
    UPDATE ple_private.draft_question_metadata
       SET question_title = p_question_title,
           question_description = p_question_description,
           tags = p_tags,
           question_license = p_question_license::ple_data.license_spdx,
           citation_text = p_citation_text,
           language = p_language,
           general_feedback = p_general_feedback,
           hint = CASE WHEN p_replace_support THEN p_hint ELSE hint END,
           worked_solution = CASE WHEN p_replace_support THEN p_worked_solution ELSE worked_solution END,
           updated_at = pg_catalog.clock_timestamp()
     WHERE draft_question_id = v_draft_question_uuid;
    IF p_replace_question_type THEN
        UPDATE ple_private.draft_question_source_binding
           SET question_type = p_question_type::ple_data.question_type,
               updated_at = pg_catalog.clock_timestamp()
         WHERE draft_question_id = v_draft_question_uuid;
    END IF;
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
    IF NOT (ple_api.current_session_account_is_instructor()
            OR ple_api.current_session_account_is_sysadmin()) THEN
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

CREATE FUNCTION ple_api.create_authoring_draft(
    p_authoring_workspace_id uuid, p_draft_question_uuid uuid, p_object_record_id uuid,
    p_object_address jsonb, p_sha256 bytea, p_size_bytes bigint, p_media_type text,
    p_created_at_millis bigint, p_question_title text, p_question_description text,
    p_tags text[], p_question_license text, p_citation_text text,
    p_language text, p_webwork_pg_path text, p_question_type text, p_question_format text
) RETURNS uuid LANGUAGE sql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private AS $$
    SELECT ple_private.create_authoring_draft(
        p_authoring_workspace_id, p_draft_question_uuid, p_object_record_id, p_object_address, p_sha256,
        p_size_bytes, p_media_type, p_created_at_millis, p_question_title,
        p_question_description, p_tags, p_question_license, p_citation_text,
        p_language, p_webwork_pg_path, p_question_type, p_question_format)
$$;

CREATE FUNCTION ple_api.save_authoring_draft(
    p_draft_question_uuid uuid, p_expected_draft_question_edit_number bigint, p_object_record_id uuid,
    p_object_address jsonb, p_sha256 bytea, p_size_bytes bigint, p_media_type text,
    p_created_at_millis bigint, p_question_type text
) RETURNS void LANGUAGE sql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private AS $$
    SELECT ple_private.save_authoring_draft(
        p_draft_question_uuid, p_expected_draft_question_edit_number, p_object_record_id, p_object_address, p_sha256,
        p_size_bytes, p_media_type, p_created_at_millis, p_question_type)
$$;

CREATE FUNCTION ple_api.save_authoring_draft_general_feedback(
    p_draft_question_uuid uuid,
    p_expected_draft_question_edit_number bigint,
    p_general_feedback text,
    p_hint text,
    p_worked_solution text,
    p_replace_support boolean,
    p_question_title text,
    p_question_description text,
    p_tags text[],
    p_question_license text,
    p_citation_text text,
    p_language text,
    p_replace_question_type boolean,
    p_question_type text
) RETURNS TABLE (draft_question_edit_number bigint, general_feedback text)
LANGUAGE sql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private AS $$
    SELECT * FROM ple_private.save_authoring_draft_general_feedback(
        p_draft_question_uuid, p_expected_draft_question_edit_number, p_general_feedback,
        p_hint, p_worked_solution, p_replace_support, p_question_title, p_question_description,
        p_tags, p_question_license, p_citation_text, p_language,
        p_replace_question_type, p_question_type)
$$;

CREATE FUNCTION ple_api.delete_draft_question(
    p_draft_question_uuid uuid,
    p_expected_draft_question_edit_number bigint
) RETURNS void LANGUAGE sql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private AS $$
    SELECT ple_private.delete_draft_question(p_draft_question_uuid, p_expected_draft_question_edit_number)
$$;
