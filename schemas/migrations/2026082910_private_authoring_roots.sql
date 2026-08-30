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
CREATE TABLE ple_private.authoring_workspace_collaborator (
    workspace_id uuid NOT NULL REFERENCES ple_private.authoring_workspace (workspace_id),
    collaborator_account_id uuid NOT NULL REFERENCES ple_private.account (account_id),
    granted_by_account_id uuid NOT NULL REFERENCES ple_private.account (account_id),
    granted_at timestamp with time zone NOT NULL,
    PRIMARY KEY (workspace_id, collaborator_account_id),
    CONSTRAINT authoring_workspace_collaborator_is_not_grantor CHECK (collaborator_account_id <> granted_by_account_id)
);
CREATE TABLE ple_private.workspace_draft_question (
    draft_id uuid PRIMARY KEY,
    workspace_id uuid NOT NULL REFERENCES ple_private.authoring_workspace (workspace_id),
    revision integer NOT NULL CHECK (revision > 0),
    title text NOT NULL CHECK (char_length(btrim(title)) BETWEEN 1 AND 500),
    definition jsonb NOT NULL CHECK (jsonb_typeof(definition) = 'object'),
    created_at timestamp with time zone NOT NULL,
    updated_at timestamp with time zone NOT NULL,
    CONSTRAINT workspace_draft_question_timestamps_are_ordered CHECK (updated_at >= created_at),
    CONSTRAINT workspace_draft_question_revision_is_unique UNIQUE (draft_id, revision),
    CONSTRAINT workspace_draft_question_has_one_current_draft UNIQUE (workspace_id)
);
CREATE TABLE ple_private.workspace_flat_question_source (
    workspace_id uuid PRIMARY KEY REFERENCES ple_private.authoring_workspace (workspace_id),
    draft_revision integer NOT NULL CHECK (draft_revision > 0),
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
    CONSTRAINT workspace_flat_question_source_timestamps_are_ordered CHECK (updated_at >= created_at)
);
CREATE TABLE ple_private.workspace_flat_question_grading (
    workspace_id uuid PRIMARY KEY REFERENCES ple_private.authoring_workspace (workspace_id),
    draft_revision integer NOT NULL CHECK (draft_revision > 0),
    public_binding_sha256 text NOT NULL CHECK (
        public_binding_sha256 ~ '^[0-9a-f]{64}$'
    ),
    payload jsonb NOT NULL CHECK (jsonb_typeof(payload) = 'object'),
    payload_sha256 text NOT NULL CHECK (payload_sha256 ~ '^[0-9a-f]{64}$'),
    created_at timestamp with time zone NOT NULL,
    updated_at timestamp with time zone NOT NULL,
    CONSTRAINT workspace_flat_question_grading_timestamps_are_ordered CHECK (updated_at >= created_at)
);
CREATE TABLE ple_private.published_flat_question_grading (
    problem_id uuid NOT NULL,
    version_id uuid NOT NULL,
    payload jsonb NOT NULL CHECK (jsonb_typeof(payload) = 'object'),
    payload_sha256 text NOT NULL CHECK (payload_sha256 ~ '^[0-9a-f]{64}$'),
    created_at timestamp with time zone NOT NULL,
    PRIMARY KEY (problem_id, version_id),
    CONSTRAINT published_flat_question_grading_version_matches FOREIGN KEY (problem_id, version_id)
        REFERENCES ple_data.published_question_version (problem_id, version_id)
);
CREATE TABLE ple_private.published_qti_question_grading (
    problem_id uuid NOT NULL,
    version_id uuid NOT NULL,
    item_id text NOT NULL CHECK (char_length(btrim(item_id)) BETWEEN 1 AND 500),
    payload bytea NOT NULL CHECK (pg_catalog.octet_length(payload) BETWEEN 1 AND 262144),
    payload_sha256 text NOT NULL CHECK (payload_sha256 ~ '^[0-9a-f]{64}$'),
    created_at timestamp with time zone NOT NULL,
    PRIMARY KEY (problem_id, version_id, item_id),
    CONSTRAINT published_qti_question_grading_version_matches FOREIGN KEY (problem_id, version_id)
        REFERENCES ple_data.published_question_version (problem_id, version_id)
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
ALTER TABLE ple_private.authoring_workspace_collaborator ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.authoring_workspace_collaborator FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_private.workspace_draft_question ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.workspace_draft_question FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_private.workspace_flat_question_source ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.workspace_flat_question_source FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_private.workspace_flat_question_grading ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.workspace_flat_question_grading FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_private.published_flat_question_grading ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.published_flat_question_grading FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_private.published_qti_question_grading ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.published_qti_question_grading FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_private.workspace_qti_import ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.workspace_qti_import FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_private.workspace_qti_import_grading ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.workspace_qti_import_grading FORCE ROW LEVEL SECURITY;
REVOKE ALL PRIVILEGES ON TABLE ple_private.authoring_workspace,
    ple_private.authoring_workspace_collaborator,
    ple_private.workspace_draft_question, ple_private.workspace_flat_question_source,
    ple_private.workspace_flat_question_grading,
    ple_private.published_flat_question_grading, ple_private.published_qti_question_grading,
    ple_private.workspace_qti_import,
    ple_private.workspace_qti_import_grading FROM PUBLIC;
COMMENT ON TABLE ple_private.authoring_workspace IS 'Private owner-scoped draft-authoring root; no shared catalog visibility.';
COMMENT ON TABLE ple_private.workspace_draft_question IS 'Private mutable draft content with no published QuestionId or ProblemId.';
COMMENT ON TABLE ple_private.workspace_flat_question_source IS 'Private current flat-question source binding keyed by its authoring workspace.';
COMMENT ON TABLE ple_private.workspace_flat_question_grading IS 'Private answer-bearing flat-question grading material keyed by its authoring workspace.';
COMMENT ON TABLE ple_private.published_flat_question_grading IS 'Private immutable flat-question grading material keyed by an exact published problem version.';
COMMENT ON TABLE ple_private.published_qti_question_grading IS 'Private immutable QTI item grading material keyed by an exact published problem version.';
COMMENT ON TABLE ple_private.workspace_qti_import IS 'Private QTI registry keyed by the authoring workspace and immutable import identity.';
COMMENT ON TABLE ple_private.workspace_qti_import_grading IS 'Private answer-bearing QTI item material accessible only through a grader lease.';
RESET ROLE;
