-- Immutable Object Records and the exact Question Source Object Reference relationship.
--
-- Bytes are written to object storage first. This relation is the database's
-- authoritative existence record and binds a Question Source to those exact bytes.

SET LOCAL ROLE ple_private_owner;

CREATE TABLE ple_private.object_record (
    object_id uuid PRIMARY KEY,
    object_address jsonb NOT NULL CHECK (jsonb_typeof(object_address) = 'object'),
    object_storage_area text NOT NULL CHECK (object_storage_area IN (
        'public-assets', 'private-content', 'student-records', 'temp-processing'
    )),
    object_data_class text NOT NULL CHECK (object_data_class IN (
        'authoring-content', 'question-source', 'question-asset', 'question-render',
        'course-appearance', 'student-record', 'temporary-processing'
    )),
    sha256 bytea NOT NULL CHECK (pg_catalog.octet_length(sha256) = 32),
    size_bytes bigint NOT NULL CHECK (size_bytes >= 0),
    media_type text NOT NULL CHECK (char_length(btrim(media_type)) BETWEEN 1 AND 255),
    created_at timestamp with time zone NOT NULL,
    CONSTRAINT object_record_address_is_unique UNIQUE (object_address)
);

CREATE FUNCTION ple_private.reject_object_record_change()
RETURNS trigger LANGUAGE plpgsql SET search_path = pg_catalog, ple_private AS $$
BEGIN
    RAISE EXCEPTION USING ERRCODE = '55000', MESSAGE = 'Object Records are immutable';
END
$$;

CREATE TRIGGER object_record_is_immutable
BEFORE UPDATE OR DELETE ON ple_private.object_record
FOR EACH ROW EXECUTE FUNCTION ple_private.reject_object_record_change();

ALTER TABLE ple_private.object_record ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.object_record FORCE ROW LEVEL SECURITY;
REVOKE ALL PRIVILEGES ON TABLE ple_private.object_record FROM PUBLIC;
CREATE POLICY object_record_private_owner_access ON ple_private.object_record
    FOR ALL TO ple_private_owner USING (true) WITH CHECK (true);

ALTER TABLE ple_private.question_source_registration
    ADD CONSTRAINT question_source_registration_object_record_exists
    FOREIGN KEY (source_object_id)
    REFERENCES ple_private.object_record (object_id);

CREATE FUNCTION ple_private.validate_question_source_registration_object_record()
RETURNS trigger LANGUAGE plpgsql SET search_path = pg_catalog, ple_private AS $$
DECLARE
    owner_workspace_id uuid;
    expected_address jsonb;
    expected_data_class text;
BEGIN
    IF NEW.source_object_id IS NULL THEN
        RETURN NEW;
    END IF;

    IF NEW.draft_question_uuid IS NOT NULL THEN
        SELECT question.workspace_id
          INTO owner_workspace_id
          FROM ple_private.draft_question AS question
         WHERE question.draft_question_uuid = NEW.draft_question_uuid;
        expected_address := jsonb_build_object(
            'kind', 'workspaceQuestionSource',
            'workspace', owner_workspace_id,
            'object', NEW.source_object_id
        );
        expected_data_class := 'authoring-content';
    ELSE
        expected_address := jsonb_build_object(
            'kind', 'questionSource',
            'questionRevision', jsonb_build_object(
                'questionId', NEW.question_id,
                'revisionNumber', NEW.revision_number
            ),
            'object', NEW.source_object_id
        );
        expected_data_class := 'question-source';
    END IF;

    IF NOT EXISTS (
        SELECT 1
          FROM ple_private.object_record AS record
         WHERE record.object_id = NEW.source_object_id
           AND record.object_storage_area = 'private-content'
           AND record.object_data_class = expected_data_class
           AND encode(record.sha256, 'hex') = NEW.source_object_checksum
           AND record.object_address @> expected_address
    ) THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Question Source Object Reference must name matching private Question Source Object Record bytes';
    END IF;
    RETURN NEW;
END
$$;

CREATE TRIGGER question_source_registration_object_record_matches_owner
BEFORE INSERT OR UPDATE ON ple_private.question_source_registration
FOR EACH ROW EXECUTE FUNCTION ple_private.validate_question_source_registration_object_record();

-- ASVS 2.1.1, 2.2.1, 2.3.1, 8.1.1, 8.2.1, and 8.3.1: this is the sole
-- session-authorized registration capability for a private workspace Question
-- Source Object.  It derives every classification field from the exact typed
-- Object Address and permits an exact retry after bytes-first object storage.
CREATE FUNCTION ple_private.register_workspace_question_source_object(
    p_workspace_id uuid,
    p_object_id uuid,
    p_object_address jsonb,
    p_sha256 bytea,
    p_size_bytes bigint,
    p_media_type text,
    p_created_at_millis bigint
)
RETURNS void LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private AS $$
DECLARE
    expected_address jsonb := jsonb_build_object(
        'kind', 'workspaceQuestionSource',
        'workspace', p_workspace_id,
        'object', p_object_id
    );
    expected_created_at timestamp with time zone :=
        pg_catalog.to_timestamp(p_created_at_millis::double precision / 1000.0);
BEGIN
    IF NOT ple_api.current_session_account_can_access_workspace(p_workspace_id) THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Workspace Question Source Object registration requires current workspace access';
    END IF;
    IF jsonb_typeof(p_object_address) <> 'object'
       OR p_object_address <> expected_address THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Workspace Question Source Object registration requires its exact typed Object Address';
    END IF;

    INSERT INTO ple_private.object_record (
        object_id, object_address, object_storage_area, object_data_class,
        sha256, size_bytes, media_type, created_at
    ) VALUES (
        p_object_id, expected_address, 'private-content', 'authoring-content',
        p_sha256, p_size_bytes, p_media_type, expected_created_at
    ) ON CONFLICT DO NOTHING;
    IF FOUND THEN
        RETURN;
    END IF;

    IF EXISTS (
        SELECT 1
          FROM ple_private.object_record AS record
         WHERE record.object_id = p_object_id
           AND record.object_address = expected_address
           AND record.object_storage_area = 'private-content'
           AND record.object_data_class = 'authoring-content'
           AND record.sha256 = p_sha256
           AND record.size_bytes = p_size_bytes
           AND record.media_type = p_media_type
           AND record.created_at = expected_created_at
    ) THEN
        RETURN;
    END IF;

    RAISE EXCEPTION USING ERRCODE = '23505',
        MESSAGE = 'Object Record identity or address already names different immutable bytes';
END
$$;

REVOKE ALL PRIVILEGES ON FUNCTION ple_private.register_workspace_question_source_object(
    uuid, uuid, jsonb, bytea, bigint, text, bigint
) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_private.register_workspace_question_source_object(
    uuid, uuid, jsonb, bytea, bigint, text, bigint
) TO ple_api_owner;
GRANT USAGE ON SCHEMA ple_private TO ple_api_owner;

SET LOCAL ROLE ple_api_owner;

CREATE FUNCTION ple_api.register_workspace_question_source_object(
    p_workspace_id uuid,
    p_object_id uuid,
    p_object_address jsonb,
    p_sha256 bytea,
    p_size_bytes bigint,
    p_media_type text,
    p_created_at_millis bigint
)
RETURNS void LANGUAGE sql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private AS $$
    SELECT ple_private.register_workspace_question_source_object(
        p_workspace_id, p_object_id, p_object_address, p_sha256,
        p_size_bytes, p_media_type, p_created_at_millis
    )
$$;

REVOKE ALL PRIVILEGES ON FUNCTION ple_api.register_workspace_question_source_object(
    uuid, uuid, jsonb, bytea, bigint, text, bigint
) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_api.register_workspace_question_source_object(
    uuid, uuid, jsonb, bytea, bigint, text, bigint
) TO ple_app;

RESET ROLE;

SET LOCAL ROLE ple_private_owner;

COMMENT ON TABLE ple_private.object_record IS
    'Immutable database-authoritative Object Record for one typed Object Address and exact stored bytes.';
COMMENT ON CONSTRAINT question_source_registration_object_record_exists ON ple_private.question_source_registration IS
    'A Source Object Reference must identify an existing immutable Object Record.';
COMMENT ON FUNCTION ple_private.validate_question_source_registration_object_record() IS
    'Requires a Source Object Reference to use an exact private Object Address, Object Data Class, and Object Checksum.';

RESET ROLE;

SET LOCAL ROLE ple_api_owner;

COMMENT ON FUNCTION ple_api.register_workspace_question_source_object(
    uuid, uuid, jsonb, bytea, bigint, text, bigint
) IS
    'Registers one exact private Workspace Question Source Object Record after bytes-first storage and current workspace authorization.';

RESET ROLE;
