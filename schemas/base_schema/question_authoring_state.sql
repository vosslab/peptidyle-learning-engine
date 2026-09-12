-- Private Question authoring state.  Publication copies validated values into
-- the shared lineage; draft rows never become library rows in place.

SET LOCAL ROLE ple_data_owner;
GRANT USAGE ON SCHEMA ple_data TO ple_private_owner;
GRANT REFERENCES ON TABLE ple_data.question_revision TO ple_private_owner;
GRANT REFERENCES ON TABLE ple_data.published_question TO ple_private_owner;
RESET ROLE;

SET LOCAL ROLE ple_private_owner;

CREATE TABLE ple_private.authoring_workspace (
    workspace_id uuid PRIMARY KEY,
    reference_number bigint GENERATED ALWAYS AS IDENTITY UNIQUE,
    owner_account_id uuid NOT NULL REFERENCES ple_private.account(account_id),
    created_at timestamptz NOT NULL,
    revoked_at timestamptz,
    CHECK (reference_number > 0 AND reference_number <= 2147483647),
    CHECK (revoked_at IS NULL OR revoked_at >= created_at)
);

CREATE TABLE ple_private.authoring_workspace_collaborator_event (
    event_id uuid PRIMARY KEY,
    workspace_id uuid NOT NULL REFERENCES ple_private.authoring_workspace(workspace_id),
    collaborator_account_id uuid NOT NULL REFERENCES ple_private.account(account_id),
    actor_account_id uuid NOT NULL REFERENCES ple_private.account(account_id),
    event_kind text NOT NULL CHECK (event_kind IN ('started', 'ended')),
    occurred_at timestamptz NOT NULL,
    UNIQUE (workspace_id, collaborator_account_id, event_kind),
    CHECK (event_kind <> 'started' OR collaborator_account_id <> actor_account_id)
);

CREATE TABLE ple_private.draft_question (
    draft_question_uuid uuid PRIMARY KEY,
    reference_number bigint GENERATED ALWAYS AS IDENTITY UNIQUE,
    workspace_id uuid NOT NULL REFERENCES ple_private.authoring_workspace(workspace_id),
    draft_question_edit_number bigint NOT NULL DEFAULT 1
        CHECK (draft_question_edit_number > 0),
    created_at timestamptz NOT NULL,
    updated_at timestamptz NOT NULL,
    CHECK (reference_number > 0 AND reference_number <= 2147483647),
    CHECK (updated_at >= created_at),
    UNIQUE (draft_question_uuid, workspace_id)
);

CREATE TABLE ple_private.draft_question_metadata (
    draft_question_uuid uuid PRIMARY KEY
        REFERENCES ple_private.draft_question(draft_question_uuid) ON DELETE CASCADE,
    question_title text NOT NULL CHECK (
        question_title = btrim(question_title)
        AND char_length(question_title) BETWEEN 1 AND 512
        AND question_title !~ '[[:cntrl:]]'
    ),
    question_description text NOT NULL CHECK (
        question_description = btrim(question_description)
        AND char_length(question_description) BETWEEN 1 AND 4000
        AND question_description !~ '[[:cntrl:]]'
    ),
    language text NOT NULL CHECK (language = btrim(language) AND char_length(language) BETWEEN 2 AND 35),
    created_at timestamptz NOT NULL,
    updated_at timestamptz NOT NULL CHECK (updated_at >= created_at)
);

CREATE FUNCTION ple_private.question_source_binding_fields_are_valid(
    p_backend text, p_question_format text, p_webwork_pg_path text,
    p_imathas_deployment_reference text, p_imathas_item_reference text,
    p_imathas_profile text, p_requires_profile boolean
) RETURNS boolean LANGUAGE sql IMMUTABLE SET search_path = pg_catalog AS $$
    SELECT COALESCE(
        (p_backend = 'ple' AND p_question_format = 'pleQuestionJson'
            AND p_webwork_pg_path IS NULL AND p_imathas_deployment_reference IS NULL
            AND p_imathas_item_reference IS NULL AND p_imathas_profile IS NULL)
        OR (p_backend = 'webwork' AND p_question_format = 'webworkPg'
            AND p_webwork_pg_path IS NOT NULL AND p_imathas_deployment_reference IS NULL
            AND p_imathas_item_reference IS NULL AND p_imathas_profile IS NULL)
        OR (p_backend = 'imathas' AND p_question_format = 'imathas'
            AND p_webwork_pg_path IS NULL AND p_imathas_deployment_reference IS NOT NULL
            AND p_imathas_item_reference IS NOT NULL
            AND (p_imathas_profile IS NOT NULL) = p_requires_profile), false)
$$;

CREATE TABLE ple_private.draft_question_source_binding (
    draft_question_uuid uuid PRIMARY KEY
        REFERENCES ple_private.draft_question(draft_question_uuid) ON DELETE CASCADE,
    backend text NOT NULL CHECK (backend IN ('ple', 'webwork', 'imathas')),
    question_format text NOT NULL CHECK (question_format IN ('pleQuestionJson', 'webworkPg', 'imathas')),
    webwork_pg_path text,
    imathas_deployment_reference text,
    imathas_item_reference text,
    imathas_profile text,
    source_object_id uuid NOT NULL,
    source_object_checksum text NOT NULL CHECK (source_object_checksum ~ '^[0-9a-f]{64}$'),
    created_at timestamptz NOT NULL,
    updated_at timestamptz NOT NULL CHECK (updated_at >= created_at),
    CHECK (ple_private.question_source_binding_fields_are_valid(
        backend, question_format, webwork_pg_path, imathas_deployment_reference,
        imathas_item_reference, imathas_profile, false))
);

CREATE TABLE ple_private.question_revision_source_binding (
    question_id text NOT NULL,
    revision_number integer NOT NULL,
    backend text NOT NULL CHECK (backend IN ('ple', 'webwork', 'imathas')),
    question_format text NOT NULL CHECK (question_format IN ('pleQuestionJson', 'webworkPg', 'imathas')),
    webwork_pg_path text,
    imathas_deployment_reference text,
    imathas_item_reference text,
    imathas_profile text,
    source_object_id uuid NOT NULL,
    source_object_checksum text NOT NULL CHECK (source_object_checksum ~ '^[0-9a-f]{64}$'),
    created_at timestamptz NOT NULL,
    PRIMARY KEY (question_id, revision_number),
    FOREIGN KEY (question_id, revision_number)
        REFERENCES ple_data.question_revision(question_id, revision_number),
    CHECK (ple_private.question_source_binding_fields_are_valid(
        backend, question_format, webwork_pg_path, imathas_deployment_reference,
        imathas_item_reference, imathas_profile, false))
);

CREATE TABLE ple_private.workspace_import (
    workspace_id uuid NOT NULL REFERENCES ple_private.authoring_workspace(workspace_id),
    import_id uuid NOT NULL,
    import_format text NOT NULL CHECK (import_format IN (
        'pleQuestionJson', 'webworkPg', 'qti', 'h5p', 'imathas'
    )),
    format_import_data jsonb NOT NULL CHECK (jsonb_typeof(format_import_data) = 'object'),
    format_import_data_sha256 text NOT NULL CHECK (format_import_data_sha256 ~ '^[0-9a-f]{64}$'),
    item_registry jsonb NOT NULL CHECK (jsonb_typeof(item_registry) = 'object'),
    item_registry_sha256 text NOT NULL CHECK (item_registry_sha256 ~ '^[0-9a-f]{64}$'),
    state text NOT NULL CHECK (state IN ('staged', 'committed')),
    staged_at timestamptz NOT NULL,
    committed_at timestamptz,
    PRIMARY KEY (workspace_id, import_id),
    CHECK ((state = 'staged' AND committed_at IS NULL)
        OR (state = 'committed' AND committed_at >= staged_at))
);

CREATE TABLE ple_private.workspace_import_item_result (
    workspace_id uuid NOT NULL,
    import_id uuid NOT NULL,
    source_item_reference text NOT NULL CHECK (
        char_length(btrim(source_item_reference)) BETWEEN 1 AND 500
    ),
    item_result text NOT NULL CHECK (item_result IN ('accepted', 'rejected')),
    format_item_data jsonb NOT NULL CHECK (jsonb_typeof(format_item_data) = 'object'),
    format_item_data_sha256 text NOT NULL CHECK (format_item_data_sha256 ~ '^[0-9a-f]{64}$'),
    recorded_at timestamptz NOT NULL,
    PRIMARY KEY (workspace_id, import_id, source_item_reference),
    FOREIGN KEY (workspace_id, import_id)
        REFERENCES ple_private.workspace_import(workspace_id, import_id) ON DELETE CASCADE
);

ALTER TABLE ple_private.draft_question_source_binding
    ADD CONSTRAINT draft_question_source_binding_object_record_exists
    FOREIGN KEY (source_object_id) REFERENCES ple_private.object_record(object_id);
ALTER TABLE ple_private.question_revision_source_binding
    ADD CONSTRAINT question_revision_source_binding_object_record_exists
    FOREIGN KEY (source_object_id) REFERENCES ple_private.object_record(object_id);

CREATE TABLE ple_private.draft_question_fork_source (
    draft_question_uuid uuid PRIMARY KEY
        REFERENCES ple_private.draft_question(draft_question_uuid),
    source_question_id text NOT NULL,
    source_revision_number integer NOT NULL,
    created_at timestamptz NOT NULL,
    FOREIGN KEY (source_question_id, source_revision_number)
        REFERENCES ple_data.question_revision(question_id, revision_number)
);

CREATE TABLE ple_private.question_folder (
    folder_id uuid PRIMARY KEY,
    owner_account_id uuid NOT NULL REFERENCES ple_private.account(account_id),
    title text NOT NULL CHECK (title = btrim(title) AND char_length(title) BETWEEN 1 AND 300),
    edit_number bigint NOT NULL DEFAULT 1 CHECK (edit_number > 0),
    created_at timestamptz NOT NULL,
    updated_at timestamptz NOT NULL CHECK (updated_at >= created_at)
);
CREATE TABLE ple_private.question_folder_entry (
    folder_id uuid NOT NULL REFERENCES ple_private.question_folder(folder_id) ON DELETE CASCADE,
    question_id text NOT NULL REFERENCES ple_data.published_question(question_id),
    added_at timestamptz NOT NULL,
    PRIMARY KEY (folder_id, question_id)
);
CREATE TABLE ple_private.saved_question_search (
    search_id uuid PRIMARY KEY,
    owner_account_id uuid NOT NULL REFERENCES ple_private.account(account_id),
    edit_number bigint NOT NULL DEFAULT 1 CHECK (edit_number > 0),
    filter jsonb NOT NULL CHECK (jsonb_typeof(filter) = 'object'),
    created_at timestamptz NOT NULL,
    updated_at timestamptz NOT NULL CHECK (updated_at >= created_at)
);

CREATE FUNCTION ple_private.reject_immutable_question_source_change()
RETURNS trigger LANGUAGE plpgsql SET search_path = pg_catalog, ple_private AS $$
BEGIN
    RAISE EXCEPTION USING ERRCODE = '55000',
        MESSAGE = 'Question Revision source and Question Fork Source are immutable';
END
$$;

CREATE FUNCTION ple_private.validate_authoring_workspace_collaborator_event()
RETURNS trigger LANGUAGE plpgsql
SET search_path = pg_catalog, ple_private AS $$
DECLARE
    workspace_owner uuid;
BEGIN
    PERFORM pg_catalog.pg_advisory_xact_lock(
        pg_catalog.hashtextextended(
            NEW.workspace_id::text || ':' || NEW.collaborator_account_id::text, 0));
    SELECT owner_account_id INTO workspace_owner
      FROM ple_private.authoring_workspace
     WHERE workspace_id = NEW.workspace_id AND revoked_at IS NULL;
    IF workspace_owner IS NULL THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Workspace Collaborator Events require an active Authoring Workspace';
    END IF;
    IF NEW.event_kind = 'started' THEN
        IF NEW.actor_account_id <> workspace_owner
           OR NEW.collaborator_account_id = workspace_owner
           OR NOT EXISTS (SELECT 1 FROM ple_private.account
               WHERE account_id = NEW.collaborator_account_id AND product_role = 'instructor') THEN
            RAISE EXCEPTION USING ERRCODE = '23514',
                MESSAGE = 'only the Authoring Workspace Owner starts an Instructor collaborator relationship';
        END IF;
    ELSIF NOT EXISTS (SELECT 1 FROM ple_private.authoring_workspace_collaborator_event AS started
        WHERE started.workspace_id = NEW.workspace_id
          AND started.collaborator_account_id = NEW.collaborator_account_id
          AND started.event_kind = 'started' AND started.occurred_at <= NEW.occurred_at)
       OR NEW.actor_account_id NOT IN (workspace_owner, NEW.collaborator_account_id) THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'a Workspace Collaborator relationship ends after its start by its owner or collaborator';
    END IF;
    RETURN NEW;
END
$$;

CREATE FUNCTION ple_private.reject_authoring_workspace_collaborator_event_change()
RETURNS trigger LANGUAGE plpgsql SET search_path = pg_catalog, ple_private AS $$
BEGIN
    RAISE EXCEPTION USING ERRCODE = '55000', MESSAGE = 'Workspace Collaborator Events are immutable';
END
$$;

CREATE FUNCTION ple_private.reject_committed_workspace_import_item_result_change()
RETURNS trigger LANGUAGE plpgsql SET search_path = pg_catalog, ple_private AS $$
BEGIN
    IF EXISTS (SELECT 1 FROM ple_private.workspace_import
        WHERE workspace_id = OLD.workspace_id AND import_id = OLD.import_id AND state = 'committed') THEN
        RAISE EXCEPTION USING ERRCODE = '55000',
            MESSAGE = 'Committed Workspace Import Item Result is immutable';
    END IF;
    RETURN COALESCE(NEW, OLD);
END
$$;

CREATE FUNCTION ple_private.validate_draft_question_source_binding_object_record()
RETURNS trigger LANGUAGE plpgsql
SET search_path = pg_catalog, ple_private AS $$
DECLARE expected_address jsonb; workspace uuid;
BEGIN
    SELECT workspace_id INTO workspace FROM ple_private.draft_question
     WHERE draft_question_uuid = NEW.draft_question_uuid;
    expected_address := jsonb_build_object('kind', 'workspaceQuestionSource',
        'workspace', workspace, 'object', NEW.source_object_id);
    IF NOT EXISTS (SELECT 1 FROM ple_private.object_record AS record
        WHERE record.object_id = NEW.source_object_id
          AND record.object_storage_area = 'private-content'
          AND record.object_data_class = 'authoring-content'
          AND encode(record.sha256, 'hex') = NEW.source_object_checksum
          AND record.object_address = expected_address) THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Draft Question Source Binding requires its exact private Object Address';
    END IF;
    RETURN NEW;
END
$$;

CREATE FUNCTION ple_private.validate_question_revision_source_binding_object_record()
RETURNS trigger LANGUAGE plpgsql
SET search_path = pg_catalog, ple_private AS $$
DECLARE expected_address jsonb;
BEGIN
    expected_address := jsonb_build_object('kind', 'questionSource',
        'questionRevision', jsonb_build_object('questionId', NEW.question_id,
            'revisionNumber', NEW.revision_number), 'object', NEW.source_object_id);
    IF NOT EXISTS (SELECT 1 FROM ple_private.object_record AS record
        WHERE record.object_id = NEW.source_object_id
          AND record.object_storage_area = 'private-content'
          AND record.object_data_class = 'question-source'
          AND encode(record.sha256, 'hex') = NEW.source_object_checksum
          AND record.object_address = expected_address) THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Question Revision Source Binding requires its exact private Object Address';
    END IF;
    RETURN NEW;
END
$$;

CREATE FUNCTION ple_private.validate_question_revision_source_binding()
RETURNS trigger LANGUAGE plpgsql
SET search_path = pg_catalog, ple_data, ple_private AS $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM ple_data.question_revision
        WHERE question_id = NEW.question_id AND revision_number = NEW.revision_number
          AND backend = NEW.backend) THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Question Revision Source Binding backend must match its Question Revision';
    END IF;
    RETURN NEW;
END
$$;

CREATE TRIGGER question_revision_source_binding_is_valid
BEFORE INSERT OR UPDATE ON ple_private.question_revision_source_binding
FOR EACH ROW EXECUTE FUNCTION ple_private.validate_question_revision_source_binding();
CREATE TRIGGER authoring_workspace_collaborator_event_has_valid_transition
BEFORE INSERT ON ple_private.authoring_workspace_collaborator_event
FOR EACH ROW EXECUTE FUNCTION ple_private.validate_authoring_workspace_collaborator_event();
CREATE TRIGGER authoring_workspace_collaborator_event_is_immutable
BEFORE UPDATE OR DELETE ON ple_private.authoring_workspace_collaborator_event
FOR EACH ROW EXECUTE FUNCTION ple_private.reject_authoring_workspace_collaborator_event_change();
CREATE TRIGGER draft_question_source_binding_object_record_matches_owner
BEFORE INSERT OR UPDATE ON ple_private.draft_question_source_binding
FOR EACH ROW EXECUTE FUNCTION ple_private.validate_draft_question_source_binding_object_record();
CREATE TRIGGER question_revision_source_binding_object_record_matches_owner
BEFORE INSERT ON ple_private.question_revision_source_binding
FOR EACH ROW EXECUTE FUNCTION ple_private.validate_question_revision_source_binding_object_record();
CREATE TRIGGER question_revision_source_binding_is_immutable
BEFORE UPDATE OR DELETE ON ple_private.question_revision_source_binding
FOR EACH ROW EXECUTE FUNCTION ple_private.reject_immutable_question_source_change();
CREATE TRIGGER draft_question_fork_source_is_immutable
BEFORE UPDATE OR DELETE ON ple_private.draft_question_fork_source
FOR EACH ROW EXECUTE FUNCTION ple_private.reject_immutable_question_source_change();
CREATE TRIGGER workspace_import_item_result_is_immutable_after_commit
BEFORE UPDATE OR DELETE ON ple_private.workspace_import_item_result
FOR EACH ROW EXECUTE FUNCTION ple_private.reject_committed_workspace_import_item_result_change();

ALTER TABLE ple_private.authoring_workspace ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.authoring_workspace FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_private.authoring_workspace_collaborator_event ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.authoring_workspace_collaborator_event FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_private.draft_question ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.draft_question FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_private.draft_question_metadata ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.draft_question_metadata FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_private.draft_question_source_binding ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.draft_question_source_binding FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_private.question_revision_source_binding ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.question_revision_source_binding FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_private.draft_question_fork_source ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.draft_question_fork_source FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_private.question_folder ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.question_folder FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_private.question_folder_entry ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.question_folder_entry FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_private.saved_question_search ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.saved_question_search FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_private.workspace_import ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.workspace_import FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_private.workspace_import_item_result ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.workspace_import_item_result FORCE ROW LEVEL SECURITY;

CREATE POLICY authoring_workspace_private_owner_access ON ple_private.authoring_workspace
    FOR ALL TO ple_private_owner USING (true) WITH CHECK (true);
CREATE POLICY authoring_collaborator_private_owner_access ON ple_private.authoring_workspace_collaborator_event
    FOR ALL TO ple_private_owner USING (true) WITH CHECK (true);
CREATE POLICY draft_question_private_owner_access ON ple_private.draft_question
    FOR ALL TO ple_private_owner USING (true) WITH CHECK (true);
CREATE POLICY draft_question_metadata_private_owner_access ON ple_private.draft_question_metadata
    FOR ALL TO ple_private_owner USING (true) WITH CHECK (true);
CREATE POLICY draft_source_private_owner_access ON ple_private.draft_question_source_binding
    FOR ALL TO ple_private_owner USING (true) WITH CHECK (true);
CREATE POLICY revision_source_private_owner_access ON ple_private.question_revision_source_binding
    FOR ALL TO ple_private_owner USING (true) WITH CHECK (true);
CREATE POLICY draft_fork_private_owner_access ON ple_private.draft_question_fork_source
    FOR ALL TO ple_private_owner USING (true) WITH CHECK (true);
CREATE POLICY question_folder_private_owner_access ON ple_private.question_folder
    FOR ALL TO ple_private_owner USING (true) WITH CHECK (true);
CREATE POLICY question_folder_entry_private_owner_access ON ple_private.question_folder_entry
    FOR ALL TO ple_private_owner USING (true) WITH CHECK (true);
CREATE POLICY saved_question_search_private_owner_access ON ple_private.saved_question_search
    FOR ALL TO ple_private_owner USING (true) WITH CHECK (true);
CREATE POLICY workspace_import_private_owner_access ON ple_private.workspace_import
    FOR ALL TO ple_private_owner USING (true) WITH CHECK (true);
CREATE POLICY workspace_import_item_result_private_owner_access ON ple_private.workspace_import_item_result
    FOR ALL TO ple_private_owner USING (true) WITH CHECK (true);

-- These narrowly scoped predicates keep API authorization and the deferred
-- publication-completeness check table-free.  Their SECURITY DEFINER owners
-- are the only roles that can inspect the underlying private rows.
-- ASVS 2.2.1-2.2.3 and 2.3.1-2.3.4: authorization is derived from the
-- current session and immutable source binding, never caller-supplied facts.
CREATE FUNCTION ple_private.current_session_is_authoring_workspace_owner(
    p_workspace_id uuid
) RETURNS boolean LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private AS $$
    SELECT p_workspace_id IS NOT NULL AND EXISTS (
        SELECT 1
          FROM ple_private.authoring_workspace AS workspace
         WHERE workspace.workspace_id = p_workspace_id
           AND workspace.owner_account_id = ple_api.current_session_account_id()
           AND workspace.revoked_at IS NULL
    )
$$;

CREATE FUNCTION ple_private.current_session_can_access_authoring_workspace(
    p_workspace_id uuid
) RETURNS boolean LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private AS $$
    SELECT p_workspace_id IS NOT NULL AND EXISTS (
        SELECT 1
          FROM ple_private.authoring_workspace AS workspace
         WHERE workspace.workspace_id = p_workspace_id
           AND workspace.revoked_at IS NULL
           AND (
               workspace.owner_account_id = ple_api.current_session_account_id()
               OR EXISTS (
                   SELECT 1
                     FROM ple_private.authoring_workspace_collaborator_event AS collaborator
                    WHERE collaborator.workspace_id = workspace.workspace_id
                      AND collaborator.collaborator_account_id = ple_api.current_session_account_id()
                      AND collaborator.event_kind = 'started'
                      AND NOT EXISTS (
                          SELECT 1
                            FROM ple_private.authoring_workspace_collaborator_event AS ended
                           WHERE ended.workspace_id = collaborator.workspace_id
                             AND ended.collaborator_account_id = collaborator.collaborator_account_id
                             AND ended.event_kind = 'ended'
                      )
               )
           )
    )
$$;

CREATE FUNCTION ple_private.question_revision_has_source_binding(
    p_question_id text, p_revision_number integer
) RETURNS boolean LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_private AS $$
    SELECT p_question_id IS NOT NULL AND p_revision_number IS NOT NULL AND EXISTS (
        SELECT 1
          FROM ple_private.question_revision_source_binding AS binding
         WHERE binding.question_id = p_question_id
           AND binding.revision_number = p_revision_number
    )
$$;

REVOKE ALL ON FUNCTION ple_private.current_session_is_authoring_workspace_owner(uuid),
    ple_private.current_session_can_access_authoring_workspace(uuid),
    ple_private.question_revision_has_source_binding(text, integer) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_private.current_session_is_authoring_workspace_owner(uuid),
    ple_private.current_session_can_access_authoring_workspace(uuid) TO ple_api_owner;
GRANT EXECUTE ON FUNCTION ple_private.question_revision_has_source_binding(text, integer)
    TO ple_data_owner;

REVOKE ALL ON ALL TABLES IN SCHEMA ple_private FROM PUBLIC;
REVOKE ALL ON FUNCTION ple_private.question_source_binding_fields_are_valid(
    text, text, text, text, text, text, boolean),
    ple_private.reject_immutable_question_source_change(),
    ple_private.validate_question_revision_source_binding(),
    ple_private.validate_authoring_workspace_collaborator_event(),
    ple_private.reject_authoring_workspace_collaborator_event_change(),
    ple_private.reject_committed_workspace_import_item_result_change(),
    ple_private.validate_draft_question_source_binding_object_record(),
    ple_private.validate_question_revision_source_binding_object_record() FROM PUBLIC;
RESET ROLE;


