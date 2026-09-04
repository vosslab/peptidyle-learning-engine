-- Private Authoring Workspaces and Draft Questions; private work has no Question Library identity.

SET LOCAL ROLE ple_data_owner;
GRANT USAGE ON SCHEMA ple_data TO ple_private_owner;
GRANT REFERENCES ON TABLE ple_data.question_revision TO ple_private_owner;
RESET ROLE;

SET LOCAL ROLE ple_private_owner;
CREATE TABLE ple_private.authoring_workspace (
    workspace_id uuid PRIMARY KEY,
    reference_number bigint GENERATED ALWAYS AS IDENTITY UNIQUE,
    authoring_workspace_owner_account_id uuid NOT NULL
        REFERENCES ple_private.account (account_id),
    created_at timestamp with time zone NOT NULL,
    revoked_at timestamp with time zone,
    CONSTRAINT authoring_workspace_revocation_is_ordered CHECK (revoked_at IS NULL OR revoked_at >= created_at),
    CONSTRAINT authoring_workspace_reference_is_bounded CHECK (
        reference_number > 0 AND reference_number <= 2147483647
    )
);
CREATE TABLE ple_private.authoring_workspace_collaborator_event (
    workspace_collaborator_event_id uuid PRIMARY KEY,
    workspace_id uuid NOT NULL REFERENCES ple_private.authoring_workspace (workspace_id),
    collaborator_account_id uuid NOT NULL REFERENCES ple_private.account (account_id),
    recorded_by_account_id uuid NOT NULL REFERENCES ple_private.account (account_id),
    event_kind text NOT NULL CHECK (event_kind IN ('started', 'ended')),
    occurred_at timestamp with time zone NOT NULL,
    CONSTRAINT authoring_workspace_collaborator_event_kind_is_unique
        UNIQUE (workspace_id, collaborator_account_id, event_kind),
    CONSTRAINT authoring_workspace_collaborator_start_has_distinct_accounts
        CHECK (event_kind <> 'started' OR collaborator_account_id <> recorded_by_account_id)
);
CREATE FUNCTION ple_private.validate_authoring_workspace_collaborator_event()
RETURNS trigger LANGUAGE plpgsql SET search_path = pg_catalog, ple_private AS $$
DECLARE
    authoring_workspace_owner_account_id uuid;
BEGIN
    PERFORM pg_catalog.pg_advisory_xact_lock(
        pg_catalog.hashtextextended(
            NEW.workspace_id::text || ':' || NEW.collaborator_account_id::text,
            0
        )
    );
    SELECT workspace.authoring_workspace_owner_account_id
      INTO authoring_workspace_owner_account_id
      FROM ple_private.authoring_workspace AS workspace
     WHERE workspace.workspace_id = NEW.workspace_id
       AND workspace.revoked_at IS NULL;
    IF authoring_workspace_owner_account_id IS NULL THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Workspace Collaborator Events require an active Authoring Workspace';
    END IF;
    IF NEW.event_kind = 'started' THEN
        IF NEW.recorded_by_account_id <> authoring_workspace_owner_account_id
           OR NEW.collaborator_account_id = authoring_workspace_owner_account_id
           OR NOT EXISTS (
               SELECT 1
                 FROM ple_private.account AS account
                WHERE account.account_id = NEW.collaborator_account_id
                  AND account.product_role = 'instructor'
           ) THEN
            RAISE EXCEPTION USING ERRCODE = '23514',
                MESSAGE = 'only the Authoring Workspace Owner may start a Workspace Collaborator relationship for an Instructor Account';
        END IF;
    ELSIF NOT EXISTS (
        SELECT 1
          FROM ple_private.authoring_workspace_collaborator_event AS start_event
         WHERE start_event.workspace_id = NEW.workspace_id
           AND start_event.collaborator_account_id = NEW.collaborator_account_id
           AND start_event.event_kind = 'started'
           AND start_event.occurred_at <= NEW.occurred_at
    ) OR NEW.recorded_by_account_id NOT IN (
        authoring_workspace_owner_account_id,
        NEW.collaborator_account_id
    ) THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'a Workspace Collaborator relationship can end only after its start and by its Authoring Workspace Owner or Workspace Collaborator';
    END IF;
    RETURN NEW;
END
$$;
CREATE TRIGGER authoring_workspace_collaborator_event_has_valid_transition
BEFORE INSERT ON ple_private.authoring_workspace_collaborator_event
FOR EACH ROW EXECUTE FUNCTION ple_private.validate_authoring_workspace_collaborator_event();
CREATE FUNCTION ple_private.reject_authoring_workspace_collaborator_event_change()
RETURNS trigger LANGUAGE plpgsql SET search_path = pg_catalog, ple_private AS $$
BEGIN
    RAISE EXCEPTION USING ERRCODE = '55000',
        MESSAGE = 'Workspace Collaborator Events are immutable';
END
$$;
CREATE TRIGGER authoring_workspace_collaborator_event_is_immutable
BEFORE UPDATE OR DELETE ON ple_private.authoring_workspace_collaborator_event
FOR EACH ROW EXECUTE FUNCTION ple_private.reject_authoring_workspace_collaborator_event_change();
CREATE TABLE ple_private.draft_question (
    draft_question_uuid uuid PRIMARY KEY,
    reference_number bigint GENERATED ALWAYS AS IDENTITY UNIQUE,
    workspace_id uuid NOT NULL REFERENCES ple_private.authoring_workspace (workspace_id),
    draft_question_edit_number bigint NOT NULL DEFAULT 1
        CHECK (draft_question_edit_number > 0),
    title text NOT NULL CHECK (char_length(btrim(title)) BETWEEN 1 AND 500),
    question_content jsonb NOT NULL CHECK (jsonb_typeof(question_content) = 'object'),
    created_at timestamp with time zone NOT NULL,
    updated_at timestamp with time zone NOT NULL,
    CONSTRAINT draft_question_reference_is_bounded
        CHECK (reference_number > 0 AND reference_number <= 2147483647),
    CONSTRAINT draft_question_timestamps_are_ordered CHECK (updated_at >= created_at),
    CONSTRAINT draft_question_workspace_identity_is_unique UNIQUE (draft_question_uuid, workspace_id)
);
CREATE FUNCTION ple_private.question_source_binding_backend_fields_are_valid(
    p_backend text,
    p_question_format text,
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
            AND p_question_format = 'pleQuestionJson'
            AND p_webwork_pg_path IS NULL
            AND p_imathas_deployment_reference IS NULL
            AND p_imathas_item_reference IS NULL
            AND p_imathas_profile IS NULL)
        OR (p_backend = 'webwork'
            AND p_question_format = 'webworkPg'
            AND p_webwork_pg_path IS NOT NULL
            AND p_imathas_deployment_reference IS NULL
            AND p_imathas_item_reference IS NULL
            AND p_imathas_profile IS NULL)
        OR (p_backend = 'imathas'
            AND p_question_format = 'imathas'
            AND p_webwork_pg_path IS NULL
            AND p_imathas_deployment_reference IS NOT NULL
            AND p_imathas_item_reference IS NOT NULL
            AND p_requires_imathas_profile = (p_imathas_profile IS NOT NULL))
    , false)
$$;
GRANT EXECUTE ON FUNCTION ple_private.question_source_binding_backend_fields_are_valid(
    text, text, text, text, text, text, boolean
) TO ple_data_owner;

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
    source_object_id uuid NOT NULL,
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
            backend, question_format, webwork_pg_path, imathas_deployment_reference,
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
    source_object_id uuid NOT NULL,
    source_object_checksum text NOT NULL CHECK (source_object_checksum ~ '^[0-9a-f]{64}$'),
    created_at timestamp with time zone NOT NULL,
    PRIMARY KEY (question_id, revision_number),
    CONSTRAINT question_revision_source_binding_revision_matches
        FOREIGN KEY (question_id, revision_number)
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
            backend, question_format, webwork_pg_path, imathas_deployment_reference,
            imathas_item_reference, imathas_profile, true
        )
    )
);
CREATE FUNCTION ple_private.validate_question_revision_source_binding_backend()
RETURNS trigger LANGUAGE plpgsql SET search_path = pg_catalog, ple_data, ple_private AS $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM ple_data.question_revision AS revision
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
CREATE TRIGGER question_revision_source_binding_backend_matches_question_revision
BEFORE INSERT OR UPDATE ON ple_private.question_revision_source_binding
FOR EACH ROW EXECUTE FUNCTION ple_private.validate_question_revision_source_binding_backend();
CREATE FUNCTION ple_private.reject_question_revision_source_binding_change()
RETURNS trigger LANGUAGE plpgsql SET search_path = pg_catalog, ple_private AS $$
BEGIN
    RAISE EXCEPTION USING ERRCODE = '55000',
        MESSAGE = 'Question Revision Source Binding is immutable';
    IF TG_OP = 'DELETE' THEN
        RETURN OLD;
    END IF;
    RETURN NEW;
END
$$;
CREATE TRIGGER question_revision_source_binding_is_immutable
BEFORE UPDATE OR DELETE ON ple_private.question_revision_source_binding
FOR EACH ROW EXECUTE FUNCTION ple_private.reject_question_revision_source_binding_change();
CREATE TABLE ple_private.workspace_import (
    workspace_id uuid NOT NULL REFERENCES ple_private.authoring_workspace (workspace_id),
    import_id uuid NOT NULL,
    import_format text NOT NULL CHECK (import_format IN (
        'pleQuestionJson', 'webworkPg', 'qti', 'h5p', 'imathas'
    )),
    format_import_data jsonb NOT NULL CHECK (jsonb_typeof(format_import_data) = 'object'),
    format_import_data_sha256 text NOT NULL CHECK (format_import_data_sha256 ~ '^[0-9a-f]{64}$'),
    item_registry jsonb NOT NULL CHECK (jsonb_typeof(item_registry) = 'object'),
    item_registry_sha256 text NOT NULL CHECK (item_registry_sha256 ~ '^[0-9a-f]{64}$'),
    state text NOT NULL CHECK (state IN ('staged', 'committed')),
    staged_at timestamp with time zone NOT NULL,
    committed_at timestamp with time zone,
    PRIMARY KEY (workspace_id, import_id),
    CONSTRAINT workspace_import_commit_is_ordered CHECK (
        (state = 'staged' AND committed_at IS NULL)
        OR (state = 'committed' AND committed_at >= staged_at)
    )
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
    recorded_at timestamp with time zone NOT NULL,
    PRIMARY KEY (workspace_id, import_id, source_item_reference),
    CONSTRAINT workspace_import_item_result_parent_matches FOREIGN KEY (workspace_id, import_id)
        REFERENCES ple_private.workspace_import (workspace_id, import_id) ON DELETE CASCADE
);
CREATE FUNCTION ple_private.reject_committed_workspace_import_item_result_change()
RETURNS trigger LANGUAGE plpgsql SET search_path = pg_catalog, ple_private AS $$
BEGIN
    IF EXISTS (
        SELECT 1 FROM ple_private.workspace_import
        WHERE workspace_id = OLD.workspace_id AND import_id = OLD.import_id AND state = 'committed'
    ) THEN
        RAISE EXCEPTION USING ERRCODE = '55000',
            MESSAGE = 'Committed Workspace Import Item Result is immutable';
    END IF;
    IF TG_OP = 'DELETE' THEN
        RETURN OLD;
    END IF;
    RETURN NEW;
END
$$;
CREATE TRIGGER workspace_import_item_result_is_immutable_after_commit
BEFORE UPDATE OR DELETE ON ple_private.workspace_import_item_result
FOR EACH ROW EXECUTE FUNCTION ple_private.reject_committed_workspace_import_item_result_change();
ALTER TABLE ple_private.authoring_workspace ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.authoring_workspace FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_private.authoring_workspace_collaborator_event ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.authoring_workspace_collaborator_event FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_private.draft_question ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.draft_question FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_private.draft_question_source_binding ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.draft_question_source_binding FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_private.question_revision_source_binding ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.question_revision_source_binding FORCE ROW LEVEL SECURITY;
CREATE POLICY draft_question_source_binding_private_owner_access
    ON ple_private.draft_question_source_binding
    FOR ALL TO ple_private_owner USING (true) WITH CHECK (true);
CREATE POLICY question_revision_source_binding_private_owner_access
    ON ple_private.question_revision_source_binding
    FOR ALL TO ple_private_owner USING (true) WITH CHECK (true);
ALTER TABLE ple_private.workspace_import ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.workspace_import FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_private.workspace_import_item_result ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.workspace_import_item_result FORCE ROW LEVEL SECURITY;
REVOKE ALL PRIVILEGES ON TABLE ple_private.authoring_workspace,
    ple_private.authoring_workspace_collaborator_event,
    ple_private.draft_question, ple_private.draft_question_source_binding,
    ple_private.question_revision_source_binding,
    ple_private.workspace_import, ple_private.workspace_import_item_result FROM PUBLIC;
COMMENT ON TABLE ple_private.authoring_workspace IS 'Private draft-authoring root with one Authoring Workspace Owner; no Question Library visibility.';
COMMENT ON TABLE ple_private.authoring_workspace_collaborator_event IS
    'Immutable start or end evidence for one Instructor Account Workspace Collaborator relationship.';
COMMENT ON TABLE ple_private.draft_question IS 'Private Draft Question lineage inside one Authoring Workspace.';
COMMENT ON TABLE ple_private.draft_question_source_binding IS
    'Current mutable Source Binding for one exact Draft Question.';
COMMENT ON TABLE ple_private.question_revision_source_binding IS
    'Immutable Source Binding for one exact Question Revision.';
COMMENT ON TABLE ple_private.workspace_import IS
    'Private staged Workspace Import with an exact Question Format, format-owned data, and item registry.';
COMMENT ON TABLE ple_private.workspace_import_item_result IS
    'Private accepted or rejected Workspace Import item result with format-owned item data.';
RESET ROLE;
