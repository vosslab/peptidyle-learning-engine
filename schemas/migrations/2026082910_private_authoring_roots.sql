-- SD1 private workspaces and drafts; private work has no shared catalog identity.

SET LOCAL ROLE ple_data_owner;
GRANT USAGE ON SCHEMA ple_data TO ple_private_owner;
GRANT REFERENCES ON TABLE ple_data.published_question_version TO ple_private_owner;
RESET ROLE;

SET LOCAL ROLE ple_private_owner;
CREATE TABLE ple_private.authoring_workspace (
    workspace_id uuid PRIMARY KEY,
    reference_number bigint GENERATED ALWAYS AS IDENTITY UNIQUE,
    owner_account_id uuid NOT NULL REFERENCES ple_private.account (account_id),
    created_at timestamp with time zone NOT NULL,
    revoked_at timestamp with time zone,
    CONSTRAINT authoring_workspace_revocation_is_ordered CHECK (revoked_at IS NULL OR revoked_at >= created_at),
    CONSTRAINT authoring_workspace_reference_is_bounded CHECK (
        reference_number > 0 AND reference_number <= 4294967295
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
    workspace_owner_account_id uuid;
BEGIN
    PERFORM pg_catalog.pg_advisory_xact_lock(
        pg_catalog.hashtextextended(
            NEW.workspace_id::text || ':' || NEW.collaborator_account_id::text,
            0
        )
    );
    SELECT workspace.owner_account_id
      INTO workspace_owner_account_id
      FROM ple_private.authoring_workspace AS workspace
     WHERE workspace.workspace_id = NEW.workspace_id
       AND workspace.revoked_at IS NULL;
    IF workspace_owner_account_id IS NULL THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Workspace Collaborator Events require an active Authoring Workspace';
    END IF;
    IF NEW.event_kind = 'started' THEN
        IF NEW.recorded_by_account_id <> workspace_owner_account_id
           OR NEW.collaborator_account_id = workspace_owner_account_id
           OR NOT EXISTS (
               SELECT 1
                 FROM LATERAL (
                     SELECT approval.event_kind
                       FROM ple_private.instructor_approval_event AS approval
                      WHERE approval.instructor_account_id = NEW.collaborator_account_id
                      ORDER BY approval.occurred_at DESC, approval.instructor_approval_event_id DESC
                      LIMIT 1
                 ) AS latest_approval
                WHERE latest_approval.event_kind = 'approved'
           ) THEN
            RAISE EXCEPTION USING ERRCODE = '23514',
                MESSAGE = 'only the Authoring Workspace Owner may start a Workspace Collaborator relationship for an Approved Instructor';
        END IF;
    ELSIF NOT EXISTS (
        SELECT 1
          FROM ple_private.authoring_workspace_collaborator_event AS start_event
         WHERE start_event.workspace_id = NEW.workspace_id
           AND start_event.collaborator_account_id = NEW.collaborator_account_id
           AND start_event.event_kind = 'started'
           AND start_event.occurred_at <= NEW.occurred_at
    ) OR NEW.recorded_by_account_id NOT IN (workspace_owner_account_id, NEW.collaborator_account_id) THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'a Workspace Collaborator relationship can end only after its start and by its owner or collaborator';
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
    draft_question_id uuid PRIMARY KEY,
    workspace_id uuid NOT NULL REFERENCES ple_private.authoring_workspace (workspace_id),
    created_at timestamp with time zone NOT NULL,
    CONSTRAINT draft_question_workspace_identity_is_unique UNIQUE (draft_question_id, workspace_id)
);
CREATE TABLE ple_private.draft_question_revision (
    draft_question_revision_id uuid PRIMARY KEY,
    draft_question_id uuid NOT NULL REFERENCES ple_private.draft_question (draft_question_id),
    revision_number integer NOT NULL CHECK (revision_number > 0),
    title text NOT NULL CHECK (char_length(btrim(title)) BETWEEN 1 AND 500),
    definition jsonb NOT NULL CHECK (jsonb_typeof(definition) = 'object'),
    created_at timestamp with time zone NOT NULL,
    CONSTRAINT draft_question_revision_number_is_unique UNIQUE (draft_question_id, revision_number)
);
CREATE FUNCTION ple_private.reject_draft_question_revision_change()
RETURNS trigger LANGUAGE plpgsql SET search_path = pg_catalog, ple_private AS $$
BEGIN RAISE EXCEPTION USING ERRCODE = '55000', MESSAGE = 'Draft Question Revisions are immutable'; END
$$;
CREATE TRIGGER draft_question_revision_is_immutable BEFORE UPDATE OR DELETE ON ple_private.draft_question_revision FOR EACH ROW EXECUTE FUNCTION ple_private.reject_draft_question_revision_change();
CREATE TABLE ple_private.draft_question_source (
    draft_question_revision_id uuid PRIMARY KEY REFERENCES ple_private.draft_question_revision (draft_question_revision_id),
    workspace_id uuid NOT NULL REFERENCES ple_private.authoring_workspace (workspace_id),
    source_family text NOT NULL CHECK (char_length(btrim(source_family)) BETWEEN 1 AND 200),
    source_record jsonb NOT NULL CHECK (jsonb_typeof(source_record) = 'object'),
    canonical_source_sha256 text NOT NULL CHECK (
        canonical_source_sha256 ~ '^[0-9a-f]{64}$'
    ),
    public_binding_sha256 text NOT NULL CHECK (
        public_binding_sha256 ~ '^[0-9a-f]{64}$'
    ),
    created_at timestamp with time zone NOT NULL,
    updated_at timestamp with time zone NOT NULL,
    CONSTRAINT draft_question_source_timestamps_are_ordered CHECK (updated_at >= created_at)
);
CREATE TABLE ple_private.draft_question_grading_material (
    draft_question_revision_id uuid PRIMARY KEY REFERENCES ple_private.draft_question_revision (draft_question_revision_id),
    workspace_id uuid NOT NULL REFERENCES ple_private.authoring_workspace (workspace_id),
    public_binding_sha256 text NOT NULL CHECK (
        public_binding_sha256 ~ '^[0-9a-f]{64}$'
    ),
    payload jsonb NOT NULL CHECK (jsonb_typeof(payload) = 'object'),
    payload_sha256 text NOT NULL CHECK (payload_sha256 ~ '^[0-9a-f]{64}$'),
    created_at timestamp with time zone NOT NULL,
    updated_at timestamp with time zone NOT NULL,
    CONSTRAINT draft_question_grading_material_timestamps_are_ordered CHECK (updated_at >= created_at)
);
CREATE FUNCTION ple_private.validate_draft_question_material_workspace()
RETURNS trigger LANGUAGE plpgsql SET search_path = pg_catalog, ple_private AS $$
BEGIN
    IF NOT EXISTS (
        SELECT 1
          FROM ple_private.draft_question_revision AS revision
          JOIN ple_private.draft_question AS question
            ON question.draft_question_id = revision.draft_question_id
         WHERE revision.draft_question_revision_id = NEW.draft_question_revision_id
           AND question.workspace_id = NEW.workspace_id
    ) THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Draft Question material must use its revision parent Authoring Workspace';
    END IF;
    RETURN NEW;
END
$$;
CREATE TRIGGER draft_question_source_workspace_matches_revision
BEFORE INSERT OR UPDATE ON ple_private.draft_question_source
FOR EACH ROW EXECUTE FUNCTION ple_private.validate_draft_question_material_workspace();
CREATE TRIGGER draft_question_grading_material_workspace_matches_revision
BEFORE INSERT OR UPDATE ON ple_private.draft_question_grading_material
FOR EACH ROW EXECUTE FUNCTION ple_private.validate_draft_question_material_workspace();
CREATE FUNCTION ple_private.reject_draft_question_material_change()
RETURNS trigger LANGUAGE plpgsql SET search_path = pg_catalog, ple_private AS $$
BEGIN RAISE EXCEPTION USING ERRCODE = '55000', MESSAGE = 'Draft Question material is immutable'; END
$$;
CREATE TRIGGER draft_question_source_is_immutable
BEFORE UPDATE OR DELETE ON ple_private.draft_question_source
FOR EACH ROW EXECUTE FUNCTION ple_private.reject_draft_question_material_change();
CREATE TRIGGER draft_question_grading_material_is_immutable
BEFORE UPDATE OR DELETE ON ple_private.draft_question_grading_material
FOR EACH ROW EXECUTE FUNCTION ple_private.reject_draft_question_material_change();
CREATE TABLE ple_private.published_flat_question_grading (
    question_id text NOT NULL,
    version_number integer NOT NULL,
    payload jsonb NOT NULL CHECK (jsonb_typeof(payload) = 'object'),
    payload_sha256 text NOT NULL CHECK (payload_sha256 ~ '^[0-9a-f]{64}$'),
    created_at timestamp with time zone NOT NULL,
    PRIMARY KEY (question_id, version_number),
    CONSTRAINT published_flat_question_grading_version_matches FOREIGN KEY (question_id, version_number)
        REFERENCES ple_data.published_question_version (question_id, version_number)
);
CREATE TABLE ple_private.published_qti_question_grading (
    question_id text NOT NULL,
    version_number integer NOT NULL,
    item_id text NOT NULL CHECK (char_length(btrim(item_id)) BETWEEN 1 AND 500),
    payload bytea NOT NULL CHECK (pg_catalog.octet_length(payload) BETWEEN 1 AND 262144),
    payload_sha256 text NOT NULL CHECK (payload_sha256 ~ '^[0-9a-f]{64}$'),
    created_at timestamp with time zone NOT NULL,
    PRIMARY KEY (question_id, version_number, item_id),
    CONSTRAINT published_qti_question_grading_version_matches FOREIGN KEY (question_id, version_number)
        REFERENCES ple_data.published_question_version (question_id, version_number)
);
CREATE TABLE ple_private.workspace_qti_import (
    workspace_id uuid NOT NULL REFERENCES ple_private.authoring_workspace (workspace_id),
    import_id uuid NOT NULL,
    source_record jsonb NOT NULL CHECK (jsonb_typeof(source_record) = 'object'),
    registry jsonb NOT NULL CHECK (jsonb_typeof(registry) = 'object'),
    registry_sha256 text NOT NULL CHECK (registry_sha256 ~ '^[0-9a-f]{64}$'),
    grading_bindings_sha256 text NOT NULL CHECK (grading_bindings_sha256 ~ '^[0-9a-f]{64}$'),
    state text NOT NULL CHECK (state IN ('prepared', 'committed')),
    prepared_at timestamp with time zone NOT NULL,
    committed_at timestamp with time zone,
    PRIMARY KEY (workspace_id, import_id),
    CONSTRAINT workspace_qti_import_commit_is_ordered CHECK (
        (state = 'prepared' AND committed_at IS NULL)
        OR (state = 'committed' AND committed_at >= prepared_at)
    )
);
CREATE TABLE ple_private.workspace_qti_import_grading (
    workspace_id uuid NOT NULL,
    import_id uuid NOT NULL,
    item_id text NOT NULL CHECK (char_length(btrim(item_id)) BETWEEN 1 AND 500),
    payload bytea NOT NULL CHECK (pg_catalog.octet_length(payload) BETWEEN 1 AND 262144),
    payload_sha256 text NOT NULL CHECK (payload_sha256 ~ '^[0-9a-f]{64}$'),
    created_at timestamp with time zone NOT NULL,
    PRIMARY KEY (workspace_id, import_id, item_id),
    CONSTRAINT workspace_qti_import_grading_parent_matches FOREIGN KEY (workspace_id, import_id)
        REFERENCES ple_private.workspace_qti_import (workspace_id, import_id) ON DELETE CASCADE
);
ALTER TABLE ple_private.authoring_workspace ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.authoring_workspace FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_private.authoring_workspace_collaborator_event ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.authoring_workspace_collaborator_event FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_private.draft_question ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.draft_question FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_private.draft_question_revision ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.draft_question_revision FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_private.draft_question_source ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.draft_question_source FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_private.draft_question_grading_material ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.draft_question_grading_material FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_private.published_flat_question_grading ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.published_flat_question_grading FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_private.published_qti_question_grading ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.published_qti_question_grading FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_private.workspace_qti_import ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.workspace_qti_import FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_private.workspace_qti_import_grading ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.workspace_qti_import_grading FORCE ROW LEVEL SECURITY;
REVOKE ALL PRIVILEGES ON TABLE ple_private.authoring_workspace,
    ple_private.authoring_workspace_collaborator_event,
    ple_private.draft_question, ple_private.draft_question_revision, ple_private.draft_question_source,
    ple_private.draft_question_grading_material,
    ple_private.published_flat_question_grading, ple_private.published_qti_question_grading,
    ple_private.workspace_qti_import,
    ple_private.workspace_qti_import_grading FROM PUBLIC;
COMMENT ON TABLE ple_private.authoring_workspace IS 'Private owner-scoped draft-authoring root; no shared catalog visibility.';
COMMENT ON TABLE ple_private.authoring_workspace_collaborator_event IS
    'Immutable start or end evidence for one Approved Instructor Workspace Collaborator relationship.';
COMMENT ON TABLE ple_private.draft_question IS 'Private Draft Question lineage inside one Authoring Workspace.';
COMMENT ON TABLE ple_private.draft_question_revision IS 'Immutable complete private Draft Question Revision with no published Question identity.';
COMMENT ON TABLE ple_private.draft_question_source IS 'Private Question Source bound to one exact Draft Question Revision.';
COMMENT ON TABLE ple_private.draft_question_grading_material IS 'Private Question Grading Material bound to one exact Draft Question Revision.';
COMMENT ON TABLE ple_private.published_flat_question_grading IS 'Private immutable flat-question grading material keyed by an exact published problem version.';
COMMENT ON TABLE ple_private.published_qti_question_grading IS 'Private immutable QTI item grading material keyed by an exact published problem version.';
COMMENT ON TABLE ple_private.workspace_qti_import IS 'Private QTI registry keyed by the authoring workspace and immutable import identity.';
COMMENT ON TABLE ple_private.workspace_qti_import_grading IS 'Private answer-bearing QTI item material accessible only through a grader lease.';
RESET ROLE;
