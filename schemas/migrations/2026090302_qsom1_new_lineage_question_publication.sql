-- WP-SD1-A-QSOM1-P1: publish one exact Draft Question as the first
-- immutable Question Revision of a new Published Question lineage.
--
-- Object storage is bytes-first. The trusted server copies the verified Draft
-- Question Source bytes to the typed Question Revision Object Address before
-- calling this transaction. A failed transaction can therefore leave only an
-- unregistered private object, which the later cleanup package must reclaim.

SET LOCAL ROLE ple_data_owner;
GRANT USAGE ON SCHEMA ple_data TO ple_private_owner;
ALTER FUNCTION ple_data.validate_question_publication_credit() SECURITY DEFINER;
GRANT INSERT, SELECT ON TABLE
    ple_data.published_question,
    ple_data.published_question_metadata,
    ple_data.question_revision,
    ple_data.question_revision_acceptance,
    ple_data.question_revision_authorship,
    ple_data.question_revision_license,
    ple_data.question_ownership_event,
    ple_data.question_publication_event,
    ple_data.question_revision_availability_event
TO ple_private_owner;

CREATE POLICY published_question_private_publication_insert
    ON ple_data.published_question FOR INSERT TO ple_private_owner WITH CHECK (true);
CREATE POLICY published_question_metadata_private_publication_insert
    ON ple_data.published_question_metadata FOR INSERT TO ple_private_owner WITH CHECK (true);
CREATE POLICY question_revision_private_publication_insert
    ON ple_data.question_revision FOR INSERT TO ple_private_owner WITH CHECK (true);
CREATE POLICY question_revision_private_publication_read
    ON ple_data.question_revision FOR SELECT TO ple_private_owner USING (true);
CREATE POLICY question_revision_acceptance_private_publication_insert
    ON ple_data.question_revision_acceptance FOR INSERT TO ple_private_owner WITH CHECK (true);
CREATE POLICY question_revision_authorship_private_publication_insert
    ON ple_data.question_revision_authorship FOR INSERT TO ple_private_owner WITH CHECK (true);
CREATE POLICY question_revision_license_private_publication_insert
    ON ple_data.question_revision_license FOR INSERT TO ple_private_owner WITH CHECK (true);
CREATE POLICY question_ownership_event_private_publication_insert
    ON ple_data.question_ownership_event FOR INSERT TO ple_private_owner WITH CHECK (true);
CREATE POLICY question_publication_event_private_publication_insert
    ON ple_data.question_publication_event FOR INSERT TO ple_private_owner WITH CHECK (true);
CREATE POLICY question_revision_availability_event_private_publication_insert
    ON ple_data.question_revision_availability_event
    FOR INSERT TO ple_private_owner WITH CHECK (true);
CREATE POLICY question_revision_acceptance_data_owner_validation_read
    ON ple_data.question_revision_acceptance FOR SELECT TO ple_data_owner USING (true);
CREATE POLICY question_revision_authorship_data_owner_validation_read
    ON ple_data.question_revision_authorship FOR SELECT TO ple_data_owner USING (true);
CREATE POLICY question_revision_license_data_owner_validation_read
    ON ple_data.question_revision_license FOR SELECT TO ple_data_owner USING (true);
CREATE POLICY question_ownership_event_data_owner_validation_read
    ON ple_data.question_ownership_event FOR SELECT TO ple_data_owner USING (true);
COMMENT ON FUNCTION ple_data.validate_question_publication_credit() IS
    'Trusted deferred completeness check for Question Authorship, acceptance, Question License, and current Question Owner at publication commit.';
RESET ROLE;

SET LOCAL ROLE ple_api_owner;
GRANT EXECUTE ON FUNCTION ple_api.current_session_account_is_instructor()
    TO ple_private_owner;
RESET ROLE;

SET LOCAL ROLE ple_private_owner;

-- ASVS 1.2.4, 2.2.1-2.2.3, 2.3.1, 2.3.3-2.3.4, 5.3.2,
-- 8.2.1-8.2.3, and 8.3.1: parameters carry values only; the trusted
-- transaction validates the exact current Draft Question, locks its Edit
-- Number, derives object classification and lineage facts, and either commits
-- the complete publication or rolls it all back.
CREATE FUNCTION ple_private.publish_new_question_lineage(
    p_draft_question_uuid uuid,
    p_expected_draft_question_edit_number bigint,
    p_workspace_id uuid,
    p_question_id text,
    p_question_source_object_id uuid,
    p_question_source_object_address jsonb,
    p_question_source_sha256 bytea,
    p_question_source_size_bytes bigint,
    p_question_source_media_type text,
    p_question_source_created_at_millis bigint,
    p_question_authorship jsonb,
    p_question_license text,
    p_reason_for_edit text,
    p_question_ownership_event_id uuid,
    p_question_publication_event_id uuid,
    p_question_availability_event_id uuid
)
RETURNS void LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
DECLARE
    v_account_id uuid;
    v_current_edit_number bigint;
    v_metadata ple_private.draft_question_metadata%ROWTYPE;
    v_binding ple_private.draft_question_source_binding%ROWTYPE;
    v_source_record ple_private.object_record%ROWTYPE;
    v_expected_object_address jsonb;
    v_published_at timestamp with time zone := pg_catalog.clock_timestamp();
    v_question_author_count integer;
    v_valid_question_author_count integer;
BEGIN
    IF p_expected_draft_question_edit_number IS NULL
       OR p_expected_draft_question_edit_number <= 0
       OR p_question_id IS NULL
       OR p_question_id !~ '^[0-9A-HJKMNP-TV-Z]{3}-[0-9A-HJKMNP-TV-Z]{4}$'
       OR p_question_source_object_id IS NULL
       OR p_question_source_sha256 IS NULL
       OR pg_catalog.octet_length(p_question_source_sha256) <> 32
       OR p_question_source_size_bytes IS NULL
       OR p_question_source_size_bytes < 0
       OR p_question_source_media_type IS NULL
       OR char_length(btrim(p_question_source_media_type)) NOT BETWEEN 1 AND 255
       OR p_question_source_created_at_millis IS NULL
       OR p_question_authorship IS NULL
       OR jsonb_typeof(p_question_authorship) <> 'array'
       OR jsonb_array_length(p_question_authorship) NOT BETWEEN 1 AND 16
       OR p_question_license IS NULL
       OR p_question_license NOT IN ('CC0-1.0', 'CC-BY-4.0', 'CC-BY-SA-4.0')
       OR p_reason_for_edit IS NULL
       OR char_length(btrim(p_reason_for_edit)) NOT BETWEEN 1 AND 2000
       OR p_reason_for_edit <> btrim(p_reason_for_edit)
       OR p_reason_for_edit ~ '[[:cntrl:]]'
       OR p_question_ownership_event_id IS NULL
       OR p_question_publication_event_id IS NULL
       OR p_question_availability_event_id IS NULL THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Question Publication arguments violate the new-lineage contract';
    END IF;

    SELECT count(DISTINCT author.value #>> '{}'), count(*)
      INTO STRICT v_question_author_count, v_valid_question_author_count
      FROM jsonb_array_elements(p_question_authorship) AS author(value)
     WHERE jsonb_typeof(author.value) = 'string'
       AND author.value #>> '{}' = btrim(author.value #>> '{}')
       AND char_length(author.value #>> '{}') BETWEEN 1 AND 120
       AND author.value #>> '{}' !~ '[[:cntrl:]]';
    IF v_question_author_count <> jsonb_array_length(p_question_authorship)
       OR v_valid_question_author_count <> jsonb_array_length(p_question_authorship) THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Question Publication requires distinct reviewed Question Authors';
    END IF;

    IF NOT ple_api.current_session_account_is_instructor()
       OR NOT ple_api.current_session_account_can_access_authoring_workspace(p_workspace_id) THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Question Publication requires current Authoring Workspace access';
    END IF;
    v_account_id := ple_api.current_session_account_id();

    SELECT question.draft_question_edit_number
      INTO v_current_edit_number
      FROM ple_private.draft_question AS question
     WHERE question.draft_question_uuid = p_draft_question_uuid
       AND question.workspace_id = p_workspace_id
     FOR UPDATE;
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Question Publication requires its exact Draft Question and workspace';
    END IF;
    IF v_current_edit_number <> p_expected_draft_question_edit_number THEN
        RAISE EXCEPTION USING ERRCODE = '40001',
            MESSAGE = 'Question Publication Draft Question Edit Number is stale';
    END IF;
    SELECT metadata.*
      INTO STRICT v_metadata
      FROM ple_private.draft_question_metadata AS metadata
     WHERE metadata.draft_question_uuid = p_draft_question_uuid
     FOR UPDATE;
    SELECT binding.*
      INTO STRICT v_binding
      FROM ple_private.draft_question_source_binding AS binding
     WHERE binding.draft_question_uuid = p_draft_question_uuid
     FOR UPDATE;
    SELECT record.*
      INTO STRICT v_source_record
      FROM ple_private.object_record AS record
     WHERE record.object_id = v_binding.source_object_id;

    v_expected_object_address := jsonb_build_object(
        'kind', 'questionSource',
        'questionRevision', jsonb_build_object(
            'questionId', p_question_id,
            'revisionNumber', 1
        ),
        'object', p_question_source_object_id
    );
    IF p_question_source_object_address IS DISTINCT FROM v_expected_object_address
       OR p_question_source_sha256 IS DISTINCT FROM v_source_record.sha256
       OR encode(p_question_source_sha256, 'hex') IS DISTINCT FROM v_binding.source_object_checksum
       OR p_question_source_size_bytes IS DISTINCT FROM v_source_record.size_bytes
       OR p_question_source_media_type IS DISTINCT FROM v_source_record.media_type THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Question Publication target must preserve the exact Draft Question Source bytes';
    END IF;

    INSERT INTO ple_data.published_question (question_id, created_at)
    VALUES (p_question_id, v_published_at);
    INSERT INTO ple_data.published_question_metadata (
        question_id, question_title, question_description, created_at, updated_at
    ) VALUES (
        p_question_id, v_metadata.question_title, v_metadata.question_description,
        v_published_at, v_published_at
    );
    INSERT INTO ple_data.question_revision (
        question_id, revision_number, backend, published_at
    ) VALUES (p_question_id, 1, v_binding.backend, v_published_at);
    INSERT INTO ple_private.object_record (
        object_id, object_address, object_storage_area, object_data_class,
        sha256, size_bytes, media_type, created_at
    ) VALUES (
        p_question_source_object_id, v_expected_object_address,
        'private-content', 'question-source', p_question_source_sha256,
        p_question_source_size_bytes, p_question_source_media_type,
        pg_catalog.to_timestamp(p_question_source_created_at_millis::double precision / 1000.0)
    );
    INSERT INTO ple_private.question_revision_source_binding (
        question_id, revision_number, backend, question_format, webwork_pg_path,
        imathas_deployment_reference, imathas_item_reference, imathas_profile,
        source_object_id, source_object_checksum, created_at
    ) VALUES (
        p_question_id, 1, v_binding.backend, v_binding.question_format,
        v_binding.webwork_pg_path, v_binding.imathas_deployment_reference,
        v_binding.imathas_item_reference, v_binding.imathas_profile,
        p_question_source_object_id, encode(p_question_source_sha256, 'hex'), v_published_at
    );
    INSERT INTO ple_data.question_revision_acceptance (
        question_id, revision_number, parent_revision_number, editor_account_id,
        accepted_by_account_id, accepted_at, reason_for_edit
    ) VALUES (
        p_question_id, 1, NULL, v_account_id, v_account_id,
        v_published_at, p_reason_for_edit
    );
    INSERT INTO ple_data.question_revision_authorship (
        question_id, revision_number, author_position,
        author_display_name, author_account_id
    )
    SELECT p_question_id, 1, author.ordinality::integer, author.value #>> '{}',
           NULL::uuid
      FROM jsonb_array_elements(p_question_authorship)
           WITH ORDINALITY AS author(value, ordinality);
    INSERT INTO ple_data.question_revision_license (
        question_id, revision_number, spdx_expression
    ) VALUES (p_question_id, 1, p_question_license);
    INSERT INTO ple_data.question_ownership_event (
        question_ownership_event_id, question_id, owner_account_id,
        recorded_by_account_id, event_kind, occurred_at
    ) VALUES (
        p_question_ownership_event_id, p_question_id, v_account_id,
        v_account_id, 'initial', v_published_at
    );
    PERFORM ple_private.publish_question_fork_source(
        p_draft_question_uuid, p_question_id
    );
    INSERT INTO ple_data.question_publication_event (
        event_id, question_id, revision_number, published_at
    ) VALUES (p_question_publication_event_id, p_question_id, 1, v_published_at);
    INSERT INTO ple_data.question_revision_availability_event (
        event_id, question_id, revision_number, availability, reason, occurred_at
    ) VALUES (
        p_question_availability_event_id, p_question_id, 1,
        'available', NULL, v_published_at
    );
END
$$;

REVOKE ALL PRIVILEGES ON FUNCTION ple_private.publish_new_question_lineage(
    uuid, bigint, uuid, text, uuid, jsonb, bytea, bigint, text, bigint,
    jsonb, text, text, uuid, uuid, uuid
) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_private.publish_new_question_lineage(
    uuid, bigint, uuid, text, uuid, jsonb, bytea, bigint, text, bigint,
    jsonb, text, text, uuid, uuid, uuid
) TO ple_api_owner;

SET LOCAL ROLE ple_api_owner;
CREATE FUNCTION ple_api.publish_new_question_lineage(
    p_draft_question_uuid uuid,
    p_expected_draft_question_edit_number bigint,
    p_workspace_id uuid,
    p_question_id text,
    p_question_source_object_id uuid,
    p_question_source_object_address jsonb,
    p_question_source_sha256 bytea,
    p_question_source_size_bytes bigint,
    p_question_source_media_type text,
    p_question_source_created_at_millis bigint,
    p_question_authorship jsonb,
    p_question_license text,
    p_reason_for_edit text,
    p_question_ownership_event_id uuid,
    p_question_publication_event_id uuid,
    p_question_availability_event_id uuid
)
RETURNS void LANGUAGE sql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private AS $$
    SELECT ple_private.publish_new_question_lineage(
        p_draft_question_uuid, p_expected_draft_question_edit_number,
        p_workspace_id, p_question_id, p_question_source_object_id,
        p_question_source_object_address, p_question_source_sha256,
        p_question_source_size_bytes, p_question_source_media_type,
        p_question_source_created_at_millis, p_question_authorship,
        p_question_license, p_reason_for_edit, p_question_ownership_event_id,
        p_question_publication_event_id, p_question_availability_event_id
    )
$$;
REVOKE ALL PRIVILEGES ON FUNCTION ple_api.publish_new_question_lineage(
    uuid, bigint, uuid, text, uuid, jsonb, bytea, bigint, text, bigint,
    jsonb, text, text, uuid, uuid, uuid
) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_api.publish_new_question_lineage(
    uuid, bigint, uuid, text, uuid, jsonb, bytea, bigint, text, bigint,
    jsonb, text, text, uuid, uuid, uuid
) TO ple_app;

COMMENT ON FUNCTION ple_api.publish_new_question_lineage(
    uuid, bigint, uuid, text, uuid, jsonb, bytea, bigint, text, bigint,
    jsonb, text, text, uuid, uuid, uuid
) IS 'Atomically publishes one exact current Draft Question as revision 1 of a new Published Question lineage after bytes-first Question Source storage.';

RESET ROLE;
