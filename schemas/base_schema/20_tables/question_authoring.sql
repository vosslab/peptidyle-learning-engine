-- question_authoring tables. CREATE TABLE and COMMENT ON only.
-- Functions, policies, and privileges live in later layers.

SET LOCAL ROLE ple_private_owner;

CREATE TABLE ple_private.authoring_workspace (
    authoring_workspace_id uuid PRIMARY KEY,
    reference_number bigint GENERATED ALWAYS AS IDENTITY UNIQUE,
    owner_account_id uuid NOT NULL REFERENCES ple_private.account(account_id),
    created_at timestamptz NOT NULL,
    revoked_at timestamptz,
    CHECK (reference_number > 0 AND reference_number <= 2147483647),
    CHECK (revoked_at IS NULL OR revoked_at >= created_at),
    updated_at timestamptz NOT NULL DEFAULT pg_catalog.transaction_timestamp(),
    CHECK (updated_at >= created_at)
);


CREATE TABLE ple_private.authoring_workspace_collaborator_event (
    event_id uuid PRIMARY KEY,
    authoring_workspace_id uuid NOT NULL REFERENCES ple_private.authoring_workspace(authoring_workspace_id),
    collaborator_account_id uuid NOT NULL REFERENCES ple_private.account(account_id),
    actor_account_id uuid NOT NULL REFERENCES ple_private.account(account_id),
    event_kind ple_data.membership_event_kind NOT NULL,
    occurred_at timestamptz NOT NULL,
    UNIQUE (authoring_workspace_id, collaborator_account_id, event_kind),
    CHECK (event_kind <> 'started' OR collaborator_account_id <> actor_account_id)
);


CREATE TABLE ple_private.draft_question (
    draft_question_id uuid PRIMARY KEY,
    authoring_workspace_id uuid NOT NULL REFERENCES ple_private.authoring_workspace(authoring_workspace_id),
    draft_question_edit_number bigint NOT NULL DEFAULT 1
        CHECK (draft_question_edit_number > 0),
    created_at timestamptz NOT NULL,
    updated_at timestamptz NOT NULL,
    CHECK (updated_at >= created_at),
    UNIQUE (draft_question_id, authoring_workspace_id)
);

CREATE TABLE ple_private.draft_question_metadata (
    draft_question_id uuid PRIMARY KEY
        REFERENCES ple_private.draft_question(draft_question_id) ON DELETE CASCADE,
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
    draft_question_id uuid PRIMARY KEY
        REFERENCES ple_private.draft_question(draft_question_id) ON DELETE CASCADE,
    backend ple_data.question_backend NOT NULL,
    question_format ple_data.question_format NOT NULL,
    question_type ple_data.question_type NOT NULL,
    webwork_pg_path text,
    imathas_deployment_reference text,
    imathas_item_reference text,
    imathas_profile text,
    source_object_record_id uuid NOT NULL,
    source_object_checksum text NOT NULL CHECK (source_object_checksum ~ '^[0-9a-f]{64}$'),
    created_at timestamptz NOT NULL,
    updated_at timestamptz NOT NULL CHECK (updated_at >= created_at),
    CHECK (ple_private.question_source_binding_fields_are_valid(
        backend, question_format, webwork_pg_path, imathas_deployment_reference,
        imathas_item_reference, imathas_profile, false))
);


CREATE TABLE ple_private.question_revision_source_binding (
    published_question_id text NOT NULL,
    revision_number integer NOT NULL,
    backend ple_data.question_backend NOT NULL,
    question_format ple_data.question_format NOT NULL,
    webwork_pg_path text,
    imathas_deployment_reference text,
    imathas_item_reference text,
    imathas_profile text,
    source_object_record_id uuid NOT NULL,
    source_object_checksum text NOT NULL CHECK (source_object_checksum ~ '^[0-9a-f]{64}$'),
    created_at timestamptz NOT NULL,
    PRIMARY KEY (published_question_id, revision_number),
    FOREIGN KEY (published_question_id, revision_number)
        REFERENCES ple_data.question_revision(published_question_id, revision_number),
    CHECK (ple_private.question_source_binding_fields_are_valid(
        backend, question_format, webwork_pg_path, imathas_deployment_reference,
        imathas_item_reference, imathas_profile, false))
);


CREATE TABLE ple_private.workspace_import (
    authoring_workspace_id uuid NOT NULL REFERENCES ple_private.authoring_workspace(authoring_workspace_id),
    import_id uuid NOT NULL,
    import_format ple_data.import_format NOT NULL,
    format_import_data jsonb NOT NULL CHECK (jsonb_typeof(format_import_data) = 'object'),
    format_import_data_sha256 text NOT NULL CHECK (format_import_data_sha256 ~ '^[0-9a-f]{64}$'),
    item_registry jsonb NOT NULL CHECK (jsonb_typeof(item_registry) = 'object'),
    item_registry_sha256 text NOT NULL CHECK (item_registry_sha256 ~ '^[0-9a-f]{64}$'),
    state ple_data.import_state NOT NULL,
    staged_at timestamptz NOT NULL,
    committed_at timestamptz,
    PRIMARY KEY (authoring_workspace_id, import_id),
    CHECK ((state = 'staged' AND committed_at IS NULL)
        OR (state = 'committed' AND committed_at >= staged_at))
);


CREATE TABLE ple_private.workspace_import_item_result (
    authoring_workspace_id uuid NOT NULL,
    import_id uuid NOT NULL,
    source_item_reference text NOT NULL CHECK (
        char_length(btrim(source_item_reference)) BETWEEN 1 AND 500
    ),
    item_result ple_data.import_item_result NOT NULL,
    format_item_data jsonb NOT NULL CHECK (jsonb_typeof(format_item_data) = 'object'),
    format_item_data_sha256 text NOT NULL CHECK (format_item_data_sha256 ~ '^[0-9a-f]{64}$'),
    recorded_at timestamptz NOT NULL,
    PRIMARY KEY (authoring_workspace_id, import_id, source_item_reference),
    FOREIGN KEY (authoring_workspace_id, import_id)
        REFERENCES ple_private.workspace_import(authoring_workspace_id, import_id) ON DELETE CASCADE
);


CREATE TABLE ple_private.draft_question_fork_source (
    draft_question_id uuid PRIMARY KEY
        REFERENCES ple_private.draft_question(draft_question_id) ON DELETE CASCADE,
    actor_account_id uuid NOT NULL REFERENCES ple_private.account(account_id),
    idempotency_key uuid NOT NULL,
    source_question_id text NOT NULL,
    source_revision_number integer NOT NULL,
    created_at timestamptz NOT NULL,
    UNIQUE (actor_account_id, idempotency_key),
    FOREIGN KEY (source_question_id, source_revision_number)
        REFERENCES ple_data.question_revision(published_question_id, revision_number)
);

CREATE TABLE ple_private.question_folder (
    question_folder_id uuid PRIMARY KEY,
    owner_account_id uuid NOT NULL REFERENCES ple_private.account(account_id),
    title text NOT NULL CHECK (title = btrim(title) AND char_length(title) BETWEEN 1 AND 300),
    edit_number bigint NOT NULL DEFAULT 1 CHECK (edit_number > 0),
    created_at timestamptz NOT NULL,
    updated_at timestamptz NOT NULL CHECK (updated_at >= created_at)
);

CREATE TABLE ple_private.question_folder_entry (
    question_folder_id uuid NOT NULL REFERENCES ple_private.question_folder(question_folder_id) ON DELETE CASCADE,
    published_question_id text NOT NULL REFERENCES ple_data.published_question(published_question_id),
    added_at timestamptz NOT NULL,
    PRIMARY KEY (question_folder_id, published_question_id),
    updated_at timestamptz NOT NULL DEFAULT pg_catalog.transaction_timestamp()
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
    draft_question_id uuid NOT NULL,
    authoring_workspace_id uuid NOT NULL,
    asset_id uuid NOT NULL,
    source_object_record_id uuid NOT NULL UNIQUE REFERENCES ple_private.object_record,
    intrinsic_width integer NOT NULL CHECK (intrinsic_width > 0),
    intrinsic_height integer NOT NULL CHECK (intrinsic_height > 0),
    PRIMARY KEY (draft_question_id, asset_id),
    FOREIGN KEY (draft_question_id, authoring_workspace_id)
        REFERENCES ple_private.draft_question(draft_question_id, authoring_workspace_id) ON DELETE CASCADE,
    CHECK (intrinsic_width::bigint * intrinsic_height <= 20000000),
    created_at timestamptz NOT NULL DEFAULT pg_catalog.transaction_timestamp(),
    updated_at timestamptz NOT NULL DEFAULT pg_catalog.transaction_timestamp(),
    CHECK (updated_at >= created_at)
);


SET LOCAL ROLE ple_private_owner;
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

