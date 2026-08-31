-- SD1 private workspaces and drafts; private work has no Question Library identity.

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
    question_format text NOT NULL CHECK (question_format IN (
        'pleFlatQuestionV2', 'nativeAlgorithmic', 'webworkPg', 'qti', 'h5p', 'imathas'
    )),
    question_type text NOT NULL CHECK (question_type IN (
        'multipleChoice', 'multipleAnswer', 'fillInBlank', 'multipleFillInBlank',
        'numeric', 'matching', 'ordering', 'hotspot'
    )),
    question_generator_id text,
    question_generator_version text,
    source_record jsonb NOT NULL CHECK (jsonb_typeof(source_record) = 'object'),
    canonical_source_sha256 text NOT NULL CHECK (
        canonical_source_sha256 ~ '^[0-9a-f]{64}$'
    ),
    public_binding_sha256 text NOT NULL CHECK (
        public_binding_sha256 ~ '^[0-9a-f]{64}$'
    ),
    created_at timestamp with time zone NOT NULL,
    updated_at timestamp with time zone NOT NULL,
    CONSTRAINT draft_question_source_timestamps_are_ordered CHECK (updated_at >= created_at),
    CONSTRAINT draft_question_source_generator_is_complete CHECK (
        (question_generator_id IS NULL AND question_generator_version IS NULL)
        OR (
            char_length(btrim(question_generator_id)) BETWEEN 1 AND 200
            AND char_length(btrim(question_generator_version)) BETWEEN 1 AND 200
        )
    )
);
CREATE TABLE ple_private.draft_question_answer_key (
    draft_question_revision_id uuid PRIMARY KEY REFERENCES ple_private.draft_question_revision (draft_question_revision_id),
    workspace_id uuid NOT NULL REFERENCES ple_private.authoring_workspace (workspace_id),
    public_binding_sha256 text NOT NULL CHECK (
        public_binding_sha256 ~ '^[0-9a-f]{64}$'
    ),
    answer_key jsonb NOT NULL CHECK (jsonb_typeof(answer_key) = 'object'),
    answer_key_sha256 text NOT NULL CHECK (answer_key_sha256 ~ '^[0-9a-f]{64}$'),
    created_at timestamp with time zone NOT NULL
);
CREATE TABLE ple_private.draft_question_feedback (
    draft_question_revision_id uuid PRIMARY KEY REFERENCES ple_private.draft_question_revision (draft_question_revision_id),
    workspace_id uuid NOT NULL REFERENCES ple_private.authoring_workspace (workspace_id),
    question_feedback jsonb NOT NULL CHECK (jsonb_typeof(question_feedback) = 'object'),
    question_feedback_sha256 text NOT NULL CHECK (question_feedback_sha256 ~ '^[0-9a-f]{64}$'),
    created_at timestamp with time zone NOT NULL
);
CREATE TABLE ple_private.draft_question_answer_explanation (
    draft_question_revision_id uuid PRIMARY KEY REFERENCES ple_private.draft_question_revision (draft_question_revision_id),
    workspace_id uuid NOT NULL REFERENCES ple_private.authoring_workspace (workspace_id),
    answer_explanation jsonb NOT NULL CHECK (jsonb_typeof(answer_explanation) = 'array'),
    answer_explanation_sha256 text NOT NULL CHECK (answer_explanation_sha256 ~ '^[0-9a-f]{64}$'),
    created_at timestamp with time zone NOT NULL
);
CREATE TABLE ple_private.draft_question_grading_input (
    draft_question_revision_id uuid PRIMARY KEY REFERENCES ple_private.draft_question_revision (draft_question_revision_id),
    workspace_id uuid NOT NULL REFERENCES ple_private.authoring_workspace (workspace_id),
    question_format text NOT NULL CHECK (question_format IN (
        'pleFlatQuestionV2', 'nativeAlgorithmic', 'webworkPg', 'qti', 'h5p', 'imathas'
    )),
    grading_input bytea NOT NULL CHECK (pg_catalog.octet_length(grading_input) BETWEEN 1 AND 262144),
    grading_input_sha256 text NOT NULL CHECK (grading_input_sha256 ~ '^[0-9a-f]{64}$'),
    created_at timestamp with time zone NOT NULL
);
CREATE FUNCTION ple_private.validate_draft_question_revision_workspace()
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
            MESSAGE = 'Draft Question private record must use its revision parent Authoring Workspace';
    END IF;
    RETURN NEW;
END
$$;
CREATE TRIGGER draft_question_source_workspace_matches_revision
BEFORE INSERT OR UPDATE ON ple_private.draft_question_source
FOR EACH ROW EXECUTE FUNCTION ple_private.validate_draft_question_revision_workspace();
CREATE TRIGGER draft_question_answer_key_workspace_matches_revision
BEFORE INSERT OR UPDATE ON ple_private.draft_question_answer_key
FOR EACH ROW EXECUTE FUNCTION ple_private.validate_draft_question_revision_workspace();
CREATE TRIGGER draft_question_feedback_workspace_matches_revision
BEFORE INSERT OR UPDATE ON ple_private.draft_question_feedback
FOR EACH ROW EXECUTE FUNCTION ple_private.validate_draft_question_revision_workspace();
CREATE TRIGGER draft_question_answer_explanation_workspace_matches_revision
BEFORE INSERT OR UPDATE ON ple_private.draft_question_answer_explanation
FOR EACH ROW EXECUTE FUNCTION ple_private.validate_draft_question_revision_workspace();
CREATE TRIGGER draft_question_grading_input_workspace_matches_revision
BEFORE INSERT OR UPDATE ON ple_private.draft_question_grading_input
FOR EACH ROW EXECUTE FUNCTION ple_private.validate_draft_question_revision_workspace();
CREATE FUNCTION ple_private.reject_question_private_record_change()
RETURNS trigger LANGUAGE plpgsql SET search_path = pg_catalog, ple_private AS $$
BEGIN RAISE EXCEPTION USING ERRCODE = '55000', MESSAGE = 'Question private record is immutable'; END
$$;
CREATE TRIGGER draft_question_source_is_immutable
BEFORE UPDATE OR DELETE ON ple_private.draft_question_source
FOR EACH ROW EXECUTE FUNCTION ple_private.reject_question_private_record_change();
CREATE TRIGGER draft_question_answer_key_is_immutable
BEFORE UPDATE OR DELETE ON ple_private.draft_question_answer_key
FOR EACH ROW EXECUTE FUNCTION ple_private.reject_question_private_record_change();
CREATE TRIGGER draft_question_feedback_is_immutable
BEFORE UPDATE OR DELETE ON ple_private.draft_question_feedback
FOR EACH ROW EXECUTE FUNCTION ple_private.reject_question_private_record_change();
CREATE TRIGGER draft_question_answer_explanation_is_immutable
BEFORE UPDATE OR DELETE ON ple_private.draft_question_answer_explanation
FOR EACH ROW EXECUTE FUNCTION ple_private.reject_question_private_record_change();
CREATE TRIGGER draft_question_grading_input_is_immutable
BEFORE UPDATE OR DELETE ON ple_private.draft_question_grading_input
FOR EACH ROW EXECUTE FUNCTION ple_private.reject_question_private_record_change();
CREATE TABLE ple_private.question_revision_answer_key (
    question_id text NOT NULL,
    version_number integer NOT NULL,
    answer_key jsonb NOT NULL CHECK (jsonb_typeof(answer_key) = 'object'),
    answer_key_sha256 text NOT NULL CHECK (answer_key_sha256 ~ '^[0-9a-f]{64}$'),
    created_at timestamp with time zone NOT NULL,
    PRIMARY KEY (question_id, version_number),
    CONSTRAINT question_revision_answer_key_version_matches FOREIGN KEY (question_id, version_number)
        REFERENCES ple_data.published_question_version (question_id, version_number)
);
CREATE TABLE ple_private.question_revision_feedback (
    question_id text NOT NULL,
    version_number integer NOT NULL,
    question_feedback jsonb NOT NULL CHECK (jsonb_typeof(question_feedback) = 'object'),
    question_feedback_sha256 text NOT NULL CHECK (question_feedback_sha256 ~ '^[0-9a-f]{64}$'),
    created_at timestamp with time zone NOT NULL,
    PRIMARY KEY (question_id, version_number),
    CONSTRAINT question_revision_feedback_version_matches FOREIGN KEY (question_id, version_number)
        REFERENCES ple_data.published_question_version (question_id, version_number)
);
CREATE TABLE ple_private.question_revision_answer_explanation (
    question_id text NOT NULL,
    version_number integer NOT NULL,
    answer_explanation jsonb NOT NULL CHECK (jsonb_typeof(answer_explanation) = 'array'),
    answer_explanation_sha256 text NOT NULL CHECK (answer_explanation_sha256 ~ '^[0-9a-f]{64}$'),
    created_at timestamp with time zone NOT NULL,
    PRIMARY KEY (question_id, version_number),
    CONSTRAINT question_revision_answer_explanation_version_matches FOREIGN KEY (question_id, version_number)
        REFERENCES ple_data.published_question_version (question_id, version_number)
);
CREATE TABLE ple_private.question_revision_grading_input (
    question_id text NOT NULL,
    version_number integer NOT NULL,
    question_format text NOT NULL CHECK (question_format IN (
        'pleFlatQuestionV2', 'nativeAlgorithmic', 'webworkPg', 'qti', 'h5p', 'imathas'
    )),
    grading_input bytea NOT NULL CHECK (pg_catalog.octet_length(grading_input) BETWEEN 1 AND 262144),
    grading_input_sha256 text NOT NULL CHECK (grading_input_sha256 ~ '^[0-9a-f]{64}$'),
    created_at timestamp with time zone NOT NULL,
    PRIMARY KEY (question_id, version_number),
    CONSTRAINT question_revision_grading_input_version_matches FOREIGN KEY (question_id, version_number)
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
CREATE FUNCTION ple_private.reject_question_revision_private_record_change()
RETURNS trigger LANGUAGE plpgsql SET search_path = pg_catalog, ple_private AS $$
BEGIN RAISE EXCEPTION USING ERRCODE = '55000', MESSAGE = 'Question Revision private record is immutable'; END
$$;
CREATE TRIGGER question_revision_answer_key_is_immutable
BEFORE UPDATE OR DELETE ON ple_private.question_revision_answer_key
FOR EACH ROW EXECUTE FUNCTION ple_private.reject_question_revision_private_record_change();
CREATE TRIGGER question_revision_feedback_is_immutable
BEFORE UPDATE OR DELETE ON ple_private.question_revision_feedback
FOR EACH ROW EXECUTE FUNCTION ple_private.reject_question_revision_private_record_change();
CREATE TRIGGER question_revision_answer_explanation_is_immutable
BEFORE UPDATE OR DELETE ON ple_private.question_revision_answer_explanation
FOR EACH ROW EXECUTE FUNCTION ple_private.reject_question_revision_private_record_change();
CREATE TRIGGER question_revision_grading_input_is_immutable
BEFORE UPDATE OR DELETE ON ple_private.question_revision_grading_input
FOR EACH ROW EXECUTE FUNCTION ple_private.reject_question_revision_private_record_change();
CREATE TABLE ple_private.workspace_import_grading_input (
    workspace_id uuid NOT NULL,
    import_id uuid NOT NULL,
    item_id text NOT NULL CHECK (char_length(btrim(item_id)) BETWEEN 1 AND 500),
    grading_input bytea NOT NULL CHECK (pg_catalog.octet_length(grading_input) BETWEEN 1 AND 262144),
    grading_input_sha256 text NOT NULL CHECK (grading_input_sha256 ~ '^[0-9a-f]{64}$'),
    created_at timestamp with time zone NOT NULL,
    PRIMARY KEY (workspace_id, import_id, item_id),
    CONSTRAINT workspace_import_grading_input_parent_matches FOREIGN KEY (workspace_id, import_id)
        REFERENCES ple_private.workspace_qti_import (workspace_id, import_id) ON DELETE CASCADE
);
CREATE FUNCTION ple_private.reject_committed_workspace_import_grading_input_change()
RETURNS trigger LANGUAGE plpgsql SET search_path = pg_catalog, ple_private AS $$
BEGIN
    IF EXISTS (
        SELECT 1 FROM ple_private.workspace_qti_import
        WHERE workspace_id = OLD.workspace_id AND import_id = OLD.import_id AND state = 'committed'
    ) THEN
        RAISE EXCEPTION USING ERRCODE = '55000',
            MESSAGE = 'Committed Workspace Import Question Grading Input is immutable';
    END IF;
    RETURN NEW;
END
$$;
CREATE TRIGGER workspace_import_grading_input_is_immutable_after_commit
BEFORE UPDATE OR DELETE ON ple_private.workspace_import_grading_input
FOR EACH ROW EXECUTE FUNCTION ple_private.reject_committed_workspace_import_grading_input_change();
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
ALTER TABLE ple_private.draft_question_answer_key ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.draft_question_answer_key FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_private.draft_question_feedback ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.draft_question_feedback FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_private.draft_question_answer_explanation ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.draft_question_answer_explanation FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_private.draft_question_grading_input ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.draft_question_grading_input FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_private.question_revision_answer_key ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.question_revision_answer_key FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_private.question_revision_feedback ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.question_revision_feedback FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_private.question_revision_answer_explanation ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.question_revision_answer_explanation FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_private.question_revision_grading_input ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.question_revision_grading_input FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_private.workspace_qti_import ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.workspace_qti_import FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_private.workspace_import_grading_input ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.workspace_import_grading_input FORCE ROW LEVEL SECURITY;
REVOKE ALL PRIVILEGES ON TABLE ple_private.authoring_workspace,
    ple_private.authoring_workspace_collaborator_event,
    ple_private.draft_question, ple_private.draft_question_revision, ple_private.draft_question_source,
    ple_private.draft_question_answer_key, ple_private.draft_question_feedback,
    ple_private.draft_question_answer_explanation, ple_private.draft_question_grading_input,
    ple_private.question_revision_answer_key, ple_private.question_revision_feedback,
    ple_private.question_revision_answer_explanation, ple_private.question_revision_grading_input,
    ple_private.workspace_qti_import,
    ple_private.workspace_import_grading_input FROM PUBLIC;
COMMENT ON TABLE ple_private.authoring_workspace IS 'Private owner-scoped draft-authoring root; no Question Library visibility.';
COMMENT ON TABLE ple_private.authoring_workspace_collaborator_event IS
    'Immutable start or end evidence for one Approved Instructor Workspace Collaborator relationship.';
COMMENT ON TABLE ple_private.draft_question IS 'Private Draft Question lineage inside one Authoring Workspace.';
COMMENT ON TABLE ple_private.draft_question_revision IS 'Immutable complete private Draft Question Revision with no published Question identity.';
COMMENT ON TABLE ple_private.draft_question_source IS 'Private Question Source bound to one exact Draft Question Revision.';
COMMENT ON TABLE ple_private.draft_question_answer_key IS 'Private Answer Key bound to one exact Draft Question Revision.';
COMMENT ON TABLE ple_private.draft_question_feedback IS 'Private Question Feedback bound to one exact Draft Question Revision.';
COMMENT ON TABLE ple_private.draft_question_answer_explanation IS 'Private Question Answer Explanation bound to one exact Draft Question Revision.';
COMMENT ON TABLE ple_private.draft_question_grading_input IS 'Private format-specific Question Grading Input bound to one exact Draft Question Revision.';
COMMENT ON TABLE ple_private.question_revision_answer_key IS 'Private immutable Answer Key keyed by one exact Question Revision.';
COMMENT ON TABLE ple_private.question_revision_feedback IS 'Private immutable Question Feedback keyed by one exact Question Revision.';
COMMENT ON TABLE ple_private.question_revision_answer_explanation IS 'Private immutable Question Answer Explanation keyed by one exact Question Revision.';
COMMENT ON TABLE ple_private.question_revision_grading_input IS 'Private immutable format-specific Question Grading Input keyed by one exact Question Revision.';
COMMENT ON TABLE ple_private.workspace_qti_import IS 'Private QTI registry keyed by the authoring workspace and immutable import identity.';
COMMENT ON TABLE ple_private.workspace_import_grading_input IS 'Private QTI Question Grading Input bound to one exact Workspace Import item.';
RESET ROLE;
