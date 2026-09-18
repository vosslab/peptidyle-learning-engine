-- question_authoring tables. CREATE TABLE and COMMENT ON only.
-- Functions, policies, and privileges live in later layers.

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
    workspace_id uuid NOT NULL REFERENCES ple_private.authoring_workspace(workspace_id),
    draft_question_edit_number bigint NOT NULL DEFAULT 1
        CHECK (draft_question_edit_number > 0),
    created_at timestamptz NOT NULL,
    updated_at timestamptz NOT NULL,
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
    -- PLE-managed general feedback is authored metadata, distinct from
    -- backend-generated interaction feedback.  Publication copies it into
    -- the immutable Question Revision.
    general_feedback text CHECK (
        general_feedback = btrim(general_feedback)
        AND char_length(general_feedback) BETWEEN 1 AND 4000
        AND general_feedback !~ '[[:cntrl:]]'
    ),
    language text NOT NULL CHECK (language = btrim(language) AND char_length(language) BETWEEN 2 AND 35),
    created_at timestamptz NOT NULL,
    updated_at timestamptz NOT NULL CHECK (updated_at >= created_at)
);

CREATE TABLE ple_private.draft_question_source_binding (
    draft_question_uuid uuid PRIMARY KEY
        REFERENCES ple_private.draft_question(draft_question_uuid) ON DELETE CASCADE,
    backend text NOT NULL CHECK (backend IN ('ple', 'webwork', 'imathas')),
    question_format text NOT NULL CHECK (question_format IN ('pleQuestionJson', 'webworkPg', 'webworkPgml', 'imathas')),
    question_type text NOT NULL CHECK (question_type IN (
        'multipleChoice', 'multipleAnswer', 'fillInBlank', 'multipleFillInBlank',
        'numeric', 'matching', 'ordering', 'hotspot'
    )),
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
    question_format text NOT NULL CHECK (question_format IN ('pleQuestionJson', 'webworkPg', 'webworkPgml', 'imathas')),
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
        'pleQuestionJson', 'webworkPg', 'webworkPgml', 'qti', 'imathas'
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

CREATE TABLE ple_private.draft_question_fork_source (
    draft_question_uuid uuid PRIMARY KEY
        REFERENCES ple_private.draft_question(draft_question_uuid) ON DELETE CASCADE,
    actor_account_id uuid NOT NULL REFERENCES ple_private.account(account_id),
    idempotency_key uuid NOT NULL,
    source_question_id text NOT NULL,
    source_revision_number integer NOT NULL,
    created_at timestamptz NOT NULL,
    UNIQUE (actor_account_id, idempotency_key),
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

-- Immutable private raster staging for a real Draft; no catalog or mutable selection.
CREATE TABLE ple_private.draft_question_asset (
    draft_question_uuid uuid NOT NULL,
    workspace_id uuid NOT NULL,
    asset_id uuid NOT NULL,
    source_object_id uuid NOT NULL UNIQUE REFERENCES ple_private.object_record,
    intrinsic_width integer NOT NULL CHECK (intrinsic_width > 0),
    intrinsic_height integer NOT NULL CHECK (intrinsic_height > 0),
    PRIMARY KEY (draft_question_uuid, asset_id),
    FOREIGN KEY (draft_question_uuid, workspace_id)
        REFERENCES ple_private.draft_question(draft_question_uuid, workspace_id) ON DELETE CASCADE,
    CHECK (intrinsic_width::bigint * intrinsic_height <= 20000000)
);

COMMENT ON TABLE ple_private.authoring_workspace IS 'role: current state, deleted by workspace delete and publication. HUMAN_GUIDANCE.md Question authoring.';

COMMENT ON TABLE ple_private.authoring_workspace_collaborator_event IS 'role: event, deleted by workspace delete and publication. HUMAN_GUIDANCE.md Question authoring.';

COMMENT ON TABLE ple_private.draft_question IS 'role: current state, deleted by workspace delete and publication. HUMAN_GUIDANCE.md Question authoring.';

COMMENT ON TABLE ple_private.draft_question_metadata IS 'role: current state, deleted by workspace delete and publication. HUMAN_GUIDANCE.md Question authoring.';

COMMENT ON TABLE ple_private.draft_question_source_binding IS 'role: current state, deleted by workspace delete and publication. HUMAN_GUIDANCE.md Question authoring.';

COMMENT ON TABLE ple_private.question_revision_source_binding IS 'role: revision, deleted by workspace delete and publication. HUMAN_GUIDANCE.md Question authoring.';

COMMENT ON TABLE ple_private.workspace_import IS 'role: event, deleted by workspace delete and publication. HUMAN_GUIDANCE.md Question authoring.';

COMMENT ON TABLE ple_private.workspace_import_item_result IS 'role: event, deleted by workspace delete and publication. HUMAN_GUIDANCE.md Question authoring.';

COMMENT ON TABLE ple_private.draft_question_fork_source IS 'role: event, deleted by workspace delete and publication. HUMAN_GUIDANCE.md Question authoring.';

COMMENT ON TABLE ple_private.question_folder IS 'role: current state, deleted by workspace delete and publication. HUMAN_GUIDANCE.md Question authoring.';

COMMENT ON TABLE ple_private.question_folder_entry IS 'role: current state, deleted by workspace delete and publication. HUMAN_GUIDANCE.md Question authoring.';

COMMENT ON TABLE ple_private.saved_question_search IS 'role: current state, deleted by workspace delete and publication. HUMAN_GUIDANCE.md Question authoring.';

COMMENT ON TABLE ple_private.draft_question_asset IS 'role: current state, deleted by workspace delete and publication. HUMAN_GUIDANCE.md Question authoring.';

