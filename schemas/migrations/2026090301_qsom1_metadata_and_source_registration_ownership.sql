-- QSOM1 schema-foundation ownership split. Draft and Published Question
-- metadata have distinct owners and lifecycles; source registrations do too.
-- This is deliberately schema-only: publication remains an unmounted future
-- transaction that must create the complete published aggregate atomically.

SET LOCAL ROLE ple_private_owner;

CREATE FUNCTION ple_private.question_metadata_fields_are_valid(
    p_question_title text,
    p_question_description text
)
RETURNS boolean LANGUAGE sql IMMUTABLE
SET search_path = pg_catalog AS $$
    SELECT char_length(btrim(p_question_title)) BETWEEN 1 AND 500
       AND char_length(btrim(p_question_description)) BETWEEN 1 AND 4000
$$;

CREATE FUNCTION ple_private.question_source_binding_backend_fields_are_valid(
    p_backend text,
    p_webwork_pg_path text,
    p_imathas_deployment_reference text,
    p_imathas_item_reference text,
    p_imathas_profile text,
    p_requires_imathas_profile boolean
)
RETURNS boolean LANGUAGE sql IMMUTABLE
SET search_path = pg_catalog AS $$
    SELECT COALESCE(
        (p_backend = 'ple'
            AND p_webwork_pg_path IS NULL
            AND p_imathas_deployment_reference IS NULL
            AND p_imathas_item_reference IS NULL
            AND p_imathas_profile IS NULL)
        OR (p_backend = 'webwork'
            AND p_webwork_pg_path IS NOT NULL
            AND p_imathas_deployment_reference IS NULL
            AND p_imathas_item_reference IS NULL
            AND p_imathas_profile IS NULL)
        OR (p_backend = 'imathas'
            AND p_webwork_pg_path IS NULL
            AND p_imathas_deployment_reference IS NOT NULL
            AND p_imathas_item_reference IS NOT NULL
            AND p_requires_imathas_profile = (p_imathas_profile IS NOT NULL))
    , false)
$$;

CREATE TABLE ple_private.draft_question_metadata (
    draft_question_uuid uuid PRIMARY KEY
        REFERENCES ple_private.draft_question (draft_question_uuid) ON DELETE CASCADE,
    question_title text NOT NULL,
    question_description text NOT NULL,
    created_at timestamp with time zone NOT NULL,
    updated_at timestamp with time zone NOT NULL,
    CONSTRAINT draft_question_metadata_fields_are_valid CHECK (
        ple_private.question_metadata_fields_are_valid(question_title, question_description)
    ),
    CONSTRAINT draft_question_metadata_timestamps_are_ordered CHECK (updated_at >= created_at)
);

SET LOCAL ROLE ple_data_owner;
CREATE TABLE ple_data.published_question_metadata (
    question_id text PRIMARY KEY
        REFERENCES ple_data.published_question (question_id) ON DELETE CASCADE,
    question_title text NOT NULL,
    question_description text NOT NULL,
    created_at timestamp with time zone NOT NULL,
    updated_at timestamp with time zone NOT NULL,
    CONSTRAINT published_question_metadata_fields_are_valid CHECK (
        ple_private.question_metadata_fields_are_valid(question_title, question_description)
    ),
    CONSTRAINT published_question_metadata_timestamps_are_ordered CHECK (updated_at >= created_at)
);

-- The old baseline had no independent Draft Question Description or Published
-- Question Title. These values only bridge an empty pre-production baseline;
-- future publication supplies validated values explicitly.
INSERT INTO ple_private.draft_question_metadata (
    draft_question_uuid, question_title, question_description, created_at, updated_at
)
SELECT draft_question_uuid, title, title, created_at, updated_at
  FROM ple_private.draft_question;
INSERT INTO ple_data.published_question_metadata (
    question_id, question_title, question_description, created_at, updated_at
)
SELECT revision.question_id, revision.question_id, revision.question_description,
       revision.published_at, revision.published_at
  FROM ple_data.question_revision AS revision
 WHERE NOT EXISTS (
     SELECT 1 FROM ple_data.published_question_metadata AS metadata
      WHERE metadata.question_id = revision.question_id
 );

SET LOCAL ROLE ple_private_owner;
CREATE TABLE ple_private.draft_question_source_binding (
    draft_question_uuid uuid PRIMARY KEY
        REFERENCES ple_private.draft_question (draft_question_uuid) ON DELETE CASCADE,
    backend text NOT NULL CHECK (backend IN ('ple', 'webwork', 'imathas')),
    question_format text NOT NULL CHECK (question_format IN (
        'pleQuestionJson', 'webworkPg', 'imathas'
    )),
    webwork_pg_path text,
    imathas_deployment_reference text,
    imathas_item_reference text,
    imathas_profile text,
    source_object_id uuid NOT NULL REFERENCES ple_private.object_record (object_id),
    source_object_checksum text NOT NULL CHECK (source_object_checksum ~ '^[0-9a-f]{64}$'),
    created_at timestamp with time zone NOT NULL,
    updated_at timestamp with time zone NOT NULL,
    CONSTRAINT draft_question_source_binding_webwork_pg_path_is_bounded CHECK (
        webwork_pg_path IS NULL OR char_length(btrim(webwork_pg_path)) BETWEEN 1 AND 1000
    ),
    CONSTRAINT draft_question_source_binding_imathas_deployment_reference_is_bounded CHECK (
        imathas_deployment_reference IS NULL
        OR char_length(btrim(imathas_deployment_reference)) BETWEEN 1 AND 255
    ),
    CONSTRAINT draft_question_source_binding_imathas_item_reference_is_bounded CHECK (
        imathas_item_reference IS NULL
        OR char_length(btrim(imathas_item_reference)) BETWEEN 1 AND 255
    ),
    CONSTRAINT draft_question_source_binding_imathas_profile_is_bounded CHECK (
        imathas_profile IS NULL OR imathas_profile ~ '^[A-Za-z0-9._-]{1,160}$'
    ),
    CONSTRAINT draft_question_source_binding_backend_fields_are_closed CHECK (
        ple_private.question_source_binding_backend_fields_are_valid(
            backend, webwork_pg_path, imathas_deployment_reference,
            imathas_item_reference, imathas_profile, false
        )
    ),
    CONSTRAINT draft_question_source_binding_timestamps_are_ordered CHECK (updated_at >= created_at)
);

CREATE TABLE ple_private.question_revision_source_binding (
    question_id text NOT NULL,
    revision_number integer NOT NULL CHECK (revision_number > 0),
    backend text NOT NULL CHECK (backend IN ('ple', 'webwork', 'imathas')),
    question_format text NOT NULL CHECK (question_format IN (
        'pleQuestionJson', 'webworkPg', 'imathas'
    )),
    webwork_pg_path text,
    imathas_deployment_reference text,
    imathas_item_reference text,
    imathas_profile text,
    source_object_id uuid NOT NULL REFERENCES ple_private.object_record (object_id),
    source_object_checksum text NOT NULL CHECK (source_object_checksum ~ '^[0-9a-f]{64}$'),
    created_at timestamp with time zone NOT NULL,
    PRIMARY KEY (question_id, revision_number),
    CONSTRAINT question_revision_source_binding_revision_matches FOREIGN KEY (question_id, revision_number)
        REFERENCES ple_data.question_revision (question_id, revision_number),
    CONSTRAINT question_revision_source_binding_webwork_pg_path_is_bounded CHECK (
        webwork_pg_path IS NULL OR char_length(btrim(webwork_pg_path)) BETWEEN 1 AND 1000
    ),
    CONSTRAINT question_revision_source_binding_imathas_deployment_reference_is_bounded CHECK (
        imathas_deployment_reference IS NULL
        OR char_length(btrim(imathas_deployment_reference)) BETWEEN 1 AND 255
    ),
    CONSTRAINT question_revision_source_binding_imathas_item_reference_is_bounded CHECK (
        imathas_item_reference IS NULL
        OR char_length(btrim(imathas_item_reference)) BETWEEN 1 AND 255
    ),
    CONSTRAINT question_revision_source_binding_imathas_profile_is_bounded CHECK (
        imathas_profile IS NULL OR imathas_profile ~ '^[A-Za-z0-9._-]{1,160}$'
    ),
    CONSTRAINT question_revision_source_binding_backend_fields_are_closed CHECK (
        ple_private.question_source_binding_backend_fields_are_valid(
            backend, webwork_pg_path, imathas_deployment_reference,
            imathas_item_reference, imathas_profile, true
        )
    )
);

INSERT INTO ple_private.draft_question_source_binding (
    draft_question_uuid, backend, question_format, webwork_pg_path,
    imathas_deployment_reference, imathas_item_reference, imathas_profile,
    source_object_id, source_object_checksum, created_at, updated_at
)
SELECT draft_question_uuid, backend, question_format, webwork_pg_path,
       imathas_deployment_reference, imathas_item_reference, imathas_profile,
       source_object_id, source_object_checksum, created_at, updated_at
  FROM ple_private.question_source_registration
 WHERE draft_question_uuid IS NOT NULL;
INSERT INTO ple_private.question_revision_source_binding (
    question_id, revision_number, backend, question_format, webwork_pg_path,
    imathas_deployment_reference, imathas_item_reference, imathas_profile,
    source_object_id, source_object_checksum, created_at
)
SELECT question_id, revision_number, backend, question_format, webwork_pg_path,
       imathas_deployment_reference, imathas_item_reference, imathas_profile,
       source_object_id, source_object_checksum, created_at
  FROM ple_private.question_source_registration
 WHERE question_id IS NOT NULL;

CREATE FUNCTION ple_private.validate_question_revision_source_binding_backend()
RETURNS trigger LANGUAGE plpgsql
SET search_path = pg_catalog, ple_data, ple_private AS $$
BEGIN
    IF NOT EXISTS (
        SELECT 1
          FROM ple_data.question_revision AS revision
         WHERE revision.question_id = NEW.question_id
           AND revision.revision_number = NEW.revision_number
           AND revision.backend = NEW.backend
    ) THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Question Source Backend must match its Question Revision Backend';
    END IF;
    RETURN NEW;
END
$$;

CREATE FUNCTION ple_private.validate_draft_question_source_binding_object_record()
RETURNS trigger LANGUAGE plpgsql
SET search_path = pg_catalog, ple_private AS $$
DECLARE
    owner_workspace_id uuid;
    expected_address jsonb;
BEGIN
    SELECT question.workspace_id
      INTO owner_workspace_id
      FROM ple_private.draft_question AS question
     WHERE question.draft_question_uuid = NEW.draft_question_uuid;
    expected_address := jsonb_build_object(
        'kind', 'workspaceQuestionSource',
        'workspace', owner_workspace_id,
        'object', NEW.source_object_id
    );
    IF NOT EXISTS (
        SELECT 1
          FROM ple_private.object_record AS record
         WHERE record.object_id = NEW.source_object_id
           AND record.object_storage_area = 'private-content'
           AND record.object_data_class = 'authoring-content'
           AND encode(record.sha256, 'hex') = NEW.source_object_checksum
           AND record.object_address = expected_address
    ) THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Draft Question Source Binding requires its exact private Object Address';
    END IF;
    RETURN NEW;
END
$$;

CREATE FUNCTION ple_private.validate_question_revision_source_binding_object_record()
RETURNS trigger LANGUAGE plpgsql
SET search_path = pg_catalog, ple_private AS $$
DECLARE
    expected_address jsonb := jsonb_build_object(
        'kind', 'questionSource',
        'questionRevision', jsonb_build_object(
            'questionId', NEW.question_id,
            'revisionNumber', NEW.revision_number
        ),
        'object', NEW.source_object_id
    );
BEGIN
    IF NOT EXISTS (
        SELECT 1
          FROM ple_private.object_record AS record
         WHERE record.object_id = NEW.source_object_id
           AND record.object_storage_area = 'private-content'
           AND record.object_data_class = 'question-source'
           AND encode(record.sha256, 'hex') = NEW.source_object_checksum
           AND record.object_address = expected_address
    ) THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Question Revision Source Binding requires its exact private Object Address';
    END IF;
    RETURN NEW;
END
$$;

CREATE FUNCTION ple_private.reject_question_revision_source_binding_change()
RETURNS trigger LANGUAGE plpgsql
SET search_path = pg_catalog, ple_private AS $$
BEGIN
    RAISE EXCEPTION USING ERRCODE = '55000',
        MESSAGE = 'Question Revision Source Binding is immutable';
END
$$;

CREATE TRIGGER question_revision_source_binding_backend_matches_question_revision
BEFORE INSERT OR UPDATE ON ple_private.question_revision_source_binding
FOR EACH ROW EXECUTE FUNCTION ple_private.validate_question_revision_source_binding_backend();
CREATE TRIGGER draft_question_source_binding_object_record_matches_owner
BEFORE INSERT OR UPDATE ON ple_private.draft_question_source_binding
FOR EACH ROW EXECUTE FUNCTION ple_private.validate_draft_question_source_binding_object_record();
CREATE TRIGGER question_revision_source_binding_object_record_matches_owner
BEFORE INSERT ON ple_private.question_revision_source_binding
FOR EACH ROW EXECUTE FUNCTION ple_private.validate_question_revision_source_binding_object_record();
CREATE TRIGGER question_revision_source_binding_is_immutable
BEFORE UPDATE OR DELETE ON ple_private.question_revision_source_binding
FOR EACH ROW EXECUTE FUNCTION ple_private.reject_question_revision_source_binding_change();

ALTER TABLE ple_private.draft_question_metadata ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.draft_question_metadata FORCE ROW LEVEL SECURITY;
SET LOCAL ROLE ple_data_owner;
ALTER TABLE ple_data.published_question_metadata ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_data.published_question_metadata FORCE ROW LEVEL SECURITY;
SET LOCAL ROLE ple_private_owner;
ALTER TABLE ple_private.draft_question_source_binding ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.draft_question_source_binding FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_private.question_revision_source_binding ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.question_revision_source_binding FORCE ROW LEVEL SECURITY;
CREATE POLICY draft_question_metadata_private_owner_access ON ple_private.draft_question_metadata
    FOR ALL TO ple_private_owner USING (true) WITH CHECK (true);
CREATE POLICY draft_question_metadata_workspace_access ON ple_private.draft_question_metadata
    FOR ALL TO ple_app
    USING (EXISTS (
        SELECT 1 FROM ple_private.draft_question AS question
         WHERE question.draft_question_uuid = draft_question_metadata.draft_question_uuid
           AND ple_api.current_session_account_can_access_workspace(question.workspace_id)
    ))
    WITH CHECK (EXISTS (
        SELECT 1 FROM ple_private.draft_question AS question
         WHERE question.draft_question_uuid = draft_question_metadata.draft_question_uuid
           AND ple_api.current_session_account_can_access_workspace(question.workspace_id)
    ));
SET LOCAL ROLE ple_data_owner;
CREATE POLICY published_question_metadata_data_owner_access ON ple_data.published_question_metadata
    FOR ALL TO ple_data_owner USING (true) WITH CHECK (true);
CREATE POLICY published_question_metadata_api_owner_read ON ple_data.published_question_metadata
    FOR SELECT TO ple_api_owner USING (true);
REVOKE ALL PRIVILEGES ON TABLE ple_data.published_question_metadata FROM PUBLIC;
GRANT SELECT ON TABLE ple_data.published_question_metadata TO ple_api_owner;

SET LOCAL ROLE ple_private_owner;
CREATE POLICY draft_question_source_binding_private_owner_access ON ple_private.draft_question_source_binding
    FOR ALL TO ple_private_owner USING (true) WITH CHECK (true);
CREATE POLICY draft_question_source_binding_workspace_access ON ple_private.draft_question_source_binding
    FOR ALL TO ple_app
    USING (EXISTS (
        SELECT 1 FROM ple_private.draft_question AS question
         WHERE question.draft_question_uuid = draft_question_source_binding.draft_question_uuid
           AND ple_api.current_session_account_can_access_workspace(question.workspace_id)
    ))
    WITH CHECK (EXISTS (
        SELECT 1 FROM ple_private.draft_question AS question
         WHERE question.draft_question_uuid = draft_question_source_binding.draft_question_uuid
           AND ple_api.current_session_account_can_access_workspace(question.workspace_id)
    ));
CREATE POLICY question_revision_source_binding_private_owner_access ON ple_private.question_revision_source_binding
    FOR ALL TO ple_private_owner USING (true) WITH CHECK (true);
CREATE POLICY question_revision_source_binding_api_owner_read ON ple_private.question_revision_source_binding
    FOR SELECT TO ple_api_owner USING (true);
REVOKE ALL PRIVILEGES ON TABLE ple_private.draft_question_metadata,
    ple_private.draft_question_source_binding,
    ple_private.question_revision_source_binding FROM PUBLIC;
GRANT SELECT ON TABLE ple_private.question_revision_source_binding TO ple_api_owner;

CREATE INDEX draft_question_metadata_title_idx
    ON ple_private.draft_question_metadata (question_title);
SET LOCAL ROLE ple_data_owner;
CREATE INDEX published_question_metadata_search_idx
    ON ple_data.published_question_metadata
    USING gin (to_tsvector('simple', question_title || ' ' || question_description));

-- Rebind the existing public Draft registration capability to its exact table.
SET LOCAL ROLE ple_private_owner;
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
    has_registration boolean;
    facts_match boolean;
BEGIN
    IF p_expected_draft_question_edit_number IS NULL
       OR p_expected_draft_question_edit_number <= 0 THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Bind Draft Question Source requires a positive Draft Question Edit Number';
    END IF;
    IF NOT ple_api.current_session_account_can_access_workspace(p_workspace_id) THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Bind Draft Question Source requires current workspace access';
    END IF;
    SELECT question.draft_question_edit_number
      INTO current_edit_number
      FROM ple_private.draft_question AS question
     WHERE question.draft_question_uuid = p_draft_question_uuid
       AND question.workspace_id = p_workspace_id
     FOR UPDATE;
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Bind Draft Question Source must use its exact Draft Question and workspace';
    END IF;
    SELECT binding.* INTO current_binding
      FROM ple_private.draft_question_source_binding AS binding
     WHERE binding.draft_question_uuid = p_draft_question_uuid
     FOR UPDATE;
    has_registration := FOUND;
    IF has_registration THEN
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
    IF facts_match AND current_edit_number IN (
        p_expected_draft_question_edit_number, p_expected_draft_question_edit_number + 1
    ) THEN
        RETURN;
    END IF;
    IF current_edit_number <> p_expected_draft_question_edit_number THEN
        RAISE EXCEPTION USING ERRCODE = '40001',
            MESSAGE = 'Draft Question Edit Number is stale or Source Binding facts do not match';
    END IF;
    IF has_registration THEN
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
            draft_question_uuid, backend, question_format, webwork_pg_path,
            imathas_deployment_reference, imathas_item_reference, imathas_profile,
            source_object_id, source_object_checksum, created_at, updated_at
        ) VALUES (
            p_draft_question_uuid, p_backend, p_question_format, p_webwork_pg_path,
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
DROP FUNCTION ple_api.register_draft_question_source_registration(
    uuid, bigint, uuid, text, text, text, text, text, text, uuid, text
);
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

CREATE OR REPLACE FUNCTION ple_api.create_imathas_question_backend_session(
    p_imathas_question_backend_session_id uuid, p_course_id uuid, p_assignment_id uuid, p_question_attempt_id uuid,
    p_imathas_deployment_reference text, p_imathas_item_reference text, p_question_id text,
    p_revision_number integer, p_source_object_id uuid, p_source_object_checksum bytea,
    p_imathas_profile text, p_question_seed numeric, p_imathas_launch_binding_checksum text,
    p_imathas_response_sha256 bytea, p_imathas_question_backend_session_challenge bytea,
    p_imathas_question_backend_session_authentication bytea, p_issued_at timestamp with time zone,
    p_expires_at timestamp with time zone, p_imathas_question_backend_state_key_id text,
    p_imathas_question_backend_state_nonce bytea, p_imathas_question_backend_state_ciphertext bytea
) RETURNS uuid LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
DECLARE v_account_id uuid;
BEGIN
    v_account_id := ple_api.current_session_account_id();
    IF p_imathas_question_backend_session_id IS NULL OR p_issued_at IS NULL
       OR p_issued_at > pg_catalog.clock_timestamp()
       OR p_expires_at <= pg_catalog.clock_timestamp() OR p_expires_at <= p_issued_at
       OR p_imathas_question_backend_state_key_id IS NULL
       OR p_imathas_question_backend_state_nonce IS NULL
       OR p_imathas_question_backend_state_ciphertext IS NULL
       OR p_imathas_item_reference IS NULL OR p_question_seed IS NULL
       OR p_imathas_launch_binding_checksum IS NULL THEN
        RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'iMathAS Question Backend Session requires a future expiry';
    END IF;
    IF NOT EXISTS (
        SELECT 1 FROM ple_private.question_attempt qa
        JOIN ple_private.issued_question iq ON iq.issued_question_id = qa.issued_question_id
        JOIN ple_private.assignment_attempt aa ON aa.assignment_attempt_id = iq.assignment_attempt_id
        JOIN ple_data.student_record sr ON sr.student_record_id = aa.student_record_id
        JOIN ple_data.assignment a ON a.assignment_id = aa.assignment_id
        JOIN ple_private.question_revision_source_binding binding
          ON binding.question_id = iq.question_id
         AND binding.revision_number = iq.revision_number
         AND binding.source_object_id = p_source_object_id
         AND binding.source_object_checksum = pg_catalog.encode(p_source_object_checksum, 'hex')
        WHERE qa.question_attempt_id = p_question_attempt_id AND sr.student_account_id = v_account_id
          AND sr.course_id = p_course_id AND a.course_id = p_course_id
          AND aa.assignment_id = p_assignment_id AND iq.question_id = p_question_id
          AND iq.revision_number = p_revision_number AND qa.question_seed = p_question_seed
          AND ple_api.current_session_account_owns_student_record(p_course_id, aa.student_record_id)
    ) THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'iMathAS Question Backend Session context is not owned by the installed Account';
    END IF;
    INSERT INTO ple_private.imathas_question_backend_session (
        imathas_question_backend_session_id, course_id, assignment_id, question_attempt_id, account_id, imathas_deployment_reference,
        imathas_item_reference, question_id, revision_number, source_object_id,
        source_object_checksum, imathas_profile, question_seed, imathas_launch_binding_checksum,
        imathas_response_sha256, imathas_question_backend_session_challenge,
        imathas_question_backend_session_authentication, issued_at, expires_at,
        imathas_question_backend_state_key_id, imathas_question_backend_state_nonce,
        imathas_question_backend_state_ciphertext
    ) VALUES (
        p_imathas_question_backend_session_id, p_course_id, p_assignment_id, p_question_attempt_id, v_account_id,
        p_imathas_deployment_reference, p_imathas_item_reference, p_question_id, p_revision_number,
        p_source_object_id, p_source_object_checksum, p_imathas_profile, p_question_seed,
        p_imathas_launch_binding_checksum, p_imathas_response_sha256,
        p_imathas_question_backend_session_challenge, p_imathas_question_backend_session_authentication,
        p_issued_at, p_expires_at, p_imathas_question_backend_state_key_id,
        p_imathas_question_backend_state_nonce, p_imathas_question_backend_state_ciphertext
    );
    RETURN p_imathas_question_backend_session_id;
END
$$;

SET LOCAL ROLE ple_private_owner;
CREATE FUNCTION ple_private.question_revision_has_question_source_binding(
    p_question_id text,
    p_revision_number integer
)
RETURNS boolean LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_private AS $$
    SELECT EXISTS (
        SELECT 1
          FROM ple_private.question_revision_source_binding AS binding
         WHERE binding.question_id = p_question_id
           AND binding.revision_number = p_revision_number
    )
$$;

DROP TABLE ple_private.question_source_registration;
ALTER TABLE ple_private.draft_question DROP COLUMN title;
ALTER TABLE ple_private.draft_question DROP COLUMN question_content;
DROP INDEX IF EXISTS ple_data.question_revision_question_description_search_idx;
ALTER TABLE ple_data.question_revision DROP COLUMN public_metadata;

-- Published reads use only stable Published Question Metadata, never a Draft
-- Question or mutable fields on an immutable Question Revision.
SET LOCAL ROLE ple_data_owner;
DROP TRIGGER question_publication_event_has_question_source_registration
    ON ple_data.question_publication_event;
DROP FUNCTION ple_data.validate_question_publication_has_question_source_registration();
CREATE FUNCTION ple_data.validate_question_publication_has_question_source_binding()
RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_data, ple_private AS $$
BEGIN
    IF NOT ple_private.question_revision_has_question_source_binding(NEW.question_id, NEW.revision_number) THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Question Publication requires an exact Question Revision Source Binding';
    END IF;
    RETURN NEW;
END
$$;
CREATE CONSTRAINT TRIGGER question_publication_event_has_question_source_binding
AFTER INSERT ON ple_data.question_publication_event
DEFERRABLE INITIALLY DEFERRED
FOR EACH ROW EXECUTE FUNCTION ple_data.validate_question_publication_has_question_source_binding();

SET LOCAL ROLE ple_api_owner;
DROP VIEW ple_api.published_question_summary;
CREATE VIEW ple_api.published_question_summary
WITH (security_barrier = true) AS
SELECT questions.question_id,
       latest_acceptance.revision_number AS latest_question_revision_number,
       versions.backend,
       versions.published_at,
       metadata.question_title,
       metadata.question_description
  FROM ple_data.published_question AS questions
  JOIN ple_data.published_question_metadata AS metadata
    ON metadata.question_id = questions.question_id
  JOIN LATERAL (
      SELECT acceptance.revision_number
        FROM ple_data.question_revision_acceptance AS acceptance
       WHERE acceptance.question_id = questions.question_id
       ORDER BY acceptance.revision_number DESC
       LIMIT 1
  ) AS latest_acceptance ON true
  JOIN ple_data.question_revision AS versions
    ON versions.question_id = questions.question_id
   AND versions.revision_number = latest_acceptance.revision_number
 WHERE ple_api.current_session_account_is_instructor();
REVOKE ALL PRIVILEGES ON TABLE ple_api.published_question_summary FROM PUBLIC;
GRANT SELECT ON TABLE ple_api.published_question_summary TO ple_app;
RESET ROLE;

SET LOCAL ROLE ple_private_owner;
COMMENT ON TABLE ple_private.draft_question_metadata IS
    'Mutable private Draft Question Title and Question Description owned by one Draft Question.';
COMMENT ON TABLE ple_private.draft_question_source_binding IS
    'Current mutable Source Binding for one exact Draft Question; its Object Address is exact.';
COMMENT ON TABLE ple_private.question_revision_source_binding IS
    'Immutable Source Binding for one exact Question Revision; its Object Address is exact.';
RESET ROLE;
SET LOCAL ROLE ple_data_owner;
COMMENT ON TABLE ple_data.published_question_metadata IS
    'Mutable Published Question Title and Question Description owned by one stable Question lineage.';
RESET ROLE;
