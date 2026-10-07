-- question_authoring tables. CREATE TABLE and COMMENT ON only.
-- Functions, policies, and privileges live in later layers.

SET LOCAL ROLE ple_private_owner;

CREATE TABLE ple_private.authoring_workspace (
    authoring_workspace_id uuid PRIMARY KEY,
    owner_account_id ple_data.account_id NOT NULL REFERENCES ple_private.account(account_id),
    created_at timestamptz NOT NULL,
    revoked_at timestamptz,
    CHECK (revoked_at IS NULL OR revoked_at >= created_at),
    updated_at timestamptz NOT NULL DEFAULT pg_catalog.transaction_timestamp(),
    CHECK (updated_at >= created_at)
);


CREATE TABLE ple_private.authoring_workspace_collaborator_event (
    event_id uuid PRIMARY KEY,
    authoring_workspace_id uuid NOT NULL REFERENCES ple_private.authoring_workspace(authoring_workspace_id),
    collaborator_account_id ple_data.account_id NOT NULL REFERENCES ple_private.account(account_id),
    actor_account_id ple_data.account_id NOT NULL REFERENCES ple_private.account(account_id),
    event_kind ple_data.membership_event_kind NOT NULL,
    occurred_at timestamptz NOT NULL,
    UNIQUE (authoring_workspace_id, collaborator_account_id, event_kind),
    CHECK (event_kind <> 'started' OR collaborator_account_id <> actor_account_id)
);


CREATE TABLE ple_private.draft_question (
    draft_question_id uuid PRIMARY KEY,
    authoring_workspace_id uuid NOT NULL REFERENCES ple_private.authoring_workspace(authoring_workspace_id),
    parent_published_question_id ple_data.question_family_id,
    public_id_reservation_id text UNIQUE
        REFERENCES ple_private.public_id_reservation(canonical_public_id),
    parent_revision_number integer CHECK (
        parent_revision_number IS NULL OR parent_revision_number > 0
    ),
    CHECK (
        (parent_published_question_id IS NULL) = (parent_revision_number IS NULL)
    ),
    CHECK (
        (parent_published_question_id IS NULL) = (public_id_reservation_id IS NULL)
    ),
    CHECK (
        public_id_reservation_id IS NULL
        OR public_id_reservation_id <> parent_published_question_id
    ),
    CHECK (
        public_id_reservation_id IS NULL
        OR ple_private.is_canonical_question_family_id(public_id_reservation_id)
    ),
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
        AND char_length(question_title) BETWEEN 0 AND 512
        AND question_title !~ '[[:cntrl:]]'
    ),
    question_description text NOT NULL CHECK (
        question_description = btrim(question_description)
        AND char_length(question_description) BETWEEN 0 AND 4000
        AND question_description !~ '[[:cntrl:]]'
    ),
    tags text[] NOT NULL DEFAULT ARRAY[]::text[]
        CHECK (ple_data.question_metadata_tags_are_valid(tags)),
    question_license ple_data.license_spdx,
    citation_text text,
    content_discipline_id uuid,
    content_subject_id uuid,
    content_topic_id uuid,
    content_subtopic_id uuid,
    -- PLE-managed general feedback is authored metadata, distinct from
    -- backend-generated interaction feedback.  Publication copies it into
    -- the immutable Question Revision.
    general_feedback text CHECK (
        general_feedback = btrim(general_feedback)
        AND char_length(general_feedback) BETWEEN 1 AND 4000
        AND general_feedback !~ '[[:cntrl:]]'
    ),
    -- ASVS 2.2.1 and 2.2.2: optional PLE-managed Hint. NULL is absent.
    -- Publication copies it onto the Question Revision. It is not backend source.
    hint text CHECK (
        hint = btrim(hint)
        AND char_length(hint) BETWEEN 1 AND 4000
        AND hint !~ '[[:cntrl:]]'
    ),
    -- ASVS 2.2.1 and 2.2.2: optional PLE-managed Worked Solution. NULL is absent.
    worked_solution text CHECK (
        worked_solution = btrim(worked_solution)
        AND char_length(worked_solution) BETWEEN 1 AND 4000
        AND worked_solution !~ '[[:cntrl:]]'
    ),
    language text CHECK (language = btrim(language) AND char_length(language) BETWEEN 2 AND 35),
    bloom_cognitive_process ple_data.bloom_cognitive_process,
    bloom_knowledge_dimension ple_data.bloom_knowledge_dimension,
    CHECK ((content_discipline_id IS NULL) = (content_subject_id IS NULL)),
    CHECK (content_topic_id IS NULL OR content_subject_id IS NOT NULL),
    CHECK (content_subtopic_id IS NULL OR content_topic_id IS NOT NULL),
    created_at timestamptz NOT NULL,
    updated_at timestamptz NOT NULL CHECK (updated_at >= created_at)
);

CREATE TABLE ple_private.draft_question_authorship (
    draft_question_id uuid NOT NULL
        REFERENCES ple_private.draft_question(draft_question_id) ON DELETE CASCADE,
    author_position integer NOT NULL CHECK (author_position BETWEEN 1 AND 16),
    author_display_name text NOT NULL CHECK (
        author_display_name = btrim(author_display_name)
        AND char_length(author_display_name) BETWEEN 1 AND 120
        AND author_display_name !~ '[[:cntrl:]]'
    ),
    author_account_id ple_data.account_id REFERENCES ple_private.account(account_id),
    created_at timestamptz NOT NULL,
    updated_at timestamptz NOT NULL CHECK (updated_at >= created_at),
    PRIMARY KEY (draft_question_id, author_position),
    UNIQUE (draft_question_id, author_display_name)
);

CREATE TABLE ple_private.draft_question_creation_receipt (
    draft_question_id uuid NOT NULL UNIQUE
        REFERENCES ple_private.draft_question(draft_question_id) ON DELETE CASCADE,
    actor_account_id ple_data.account_id NOT NULL REFERENCES ple_private.account(account_id),
    request_key uuid NOT NULL,
    request_fingerprint bytea NOT NULL CHECK (octet_length(request_fingerprint) = 32),
    created_at timestamptz NOT NULL,
    PRIMARY KEY (actor_account_id, request_key)
);

CREATE TABLE ple_private.draft_question_source_binding (
    draft_question_id uuid PRIMARY KEY
        REFERENCES ple_private.draft_question(draft_question_id) ON DELETE CASCADE,
    backend ple_data.question_backend NOT NULL,
    question_format ple_data.question_format NOT NULL,
    question_type ple_data.question_type,
    webwork_pg_path text,
    source_object_record_id uuid NOT NULL,
    source_object_checksum text NOT NULL CHECK (source_object_checksum ~ '^[0-9a-f]{64}$'),
    created_at timestamptz NOT NULL,
    updated_at timestamptz NOT NULL CHECK (updated_at >= created_at),
    CHECK (ple_private.question_source_binding_fields_are_valid(
        backend, question_format, webwork_pg_path))
);


CREATE TABLE ple_private.question_revision_source_binding (
    published_question_id ple_data.question_family_id NOT NULL,
    revision_number integer NOT NULL,
    backend ple_data.question_backend NOT NULL,
    -- Native interaction Type is immutable source evidence used to guard its editable projection.
    -- WeBWorK Type is ordinary metadata and is therefore NULL here.
    native_question_type ple_data.question_type,
    question_format ple_data.question_format NOT NULL,
    webwork_pg_path text,
    source_object_record_id uuid NOT NULL,
    source_object_checksum text NOT NULL CHECK (source_object_checksum ~ '^[0-9a-f]{64}$'),
    created_at timestamptz NOT NULL,
    PRIMARY KEY (published_question_id, revision_number),
    FOREIGN KEY (published_question_id, revision_number)
        REFERENCES ple_data.question_revision(published_question_id, revision_number),
    CHECK (ple_private.question_source_binding_fields_are_valid(
        backend, question_format, webwork_pg_path)),
    CHECK ((backend = 'ple' AND native_question_type IS NOT NULL)
        OR (backend = 'webwork' AND native_question_type IS NULL))
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
    source_item_key text NOT NULL CHECK (
        char_length(btrim(source_item_key)) BETWEEN 1 AND 500
    ),
    item_result ple_data.import_item_result NOT NULL,
    format_item_data jsonb NOT NULL CHECK (jsonb_typeof(format_item_data) = 'object'),
    format_item_data_sha256 text NOT NULL CHECK (format_item_data_sha256 ~ '^[0-9a-f]{64}$'),
    recorded_at timestamptz NOT NULL,
    PRIMARY KEY (authoring_workspace_id, import_id, source_item_key),
    FOREIGN KEY (authoring_workspace_id, import_id)
        REFERENCES ple_private.workspace_import(authoring_workspace_id, import_id) ON DELETE CASCADE
);


CREATE TABLE ple_private.question_folder (
    question_folder_id uuid PRIMARY KEY,
    owner_account_id ple_data.account_id NOT NULL REFERENCES ple_private.account(account_id),
    title text NOT NULL CHECK (title = btrim(title) AND char_length(title) BETWEEN 1 AND 300),
    edit_number bigint NOT NULL DEFAULT 1 CHECK (edit_number > 0),
    created_at timestamptz NOT NULL,
    updated_at timestamptz NOT NULL CHECK (updated_at >= created_at)
);

CREATE TABLE ple_private.question_folder_entry (
    question_folder_id uuid NOT NULL REFERENCES ple_private.question_folder(question_folder_id) ON DELETE CASCADE,
    published_question_id ple_data.question_family_id NOT NULL REFERENCES ple_data.published_question(published_question_id),
    added_at timestamptz NOT NULL,
    PRIMARY KEY (question_folder_id, published_question_id),
    updated_at timestamptz NOT NULL DEFAULT pg_catalog.transaction_timestamp()
);


-- Immutable private raster staging for a real Draft; no catalog or mutable selection.
CREATE TABLE ple_private.draft_question_image (
    draft_question_id uuid NOT NULL,
    authoring_workspace_id uuid NOT NULL,
    question_image_asset_id uuid NOT NULL,
    source_object_record_id uuid NOT NULL UNIQUE REFERENCES ple_private.object_record,
    intrinsic_width integer NOT NULL CHECK (intrinsic_width > 0),
    intrinsic_height integer NOT NULL CHECK (intrinsic_height > 0),
    PRIMARY KEY (draft_question_id, question_image_asset_id),
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

COMMENT ON TABLE ple_private.draft_question_authorship IS 'role: current state, copied into a fork Draft and deleted with its Draft or publication. HUMAN_GUIDANCE.md Published Questions.';

COMMENT ON TABLE ple_private.draft_question_creation_receipt IS 'role: event, deleted by Draft workspace deletion and publication. HUMAN_GUIDANCE.md Question authoring.';

COMMENT ON COLUMN ple_private.draft_question.public_id_reservation_id IS 'NULL means this ordinary Draft receives a public Question ID at publication; a fork reserves its new ID when the Draft is created.';

COMMENT ON COLUMN ple_private.draft_question_metadata.content_discipline_id IS 'NULL means no content classification has been assigned.';
COMMENT ON COLUMN ple_private.draft_question_metadata.content_subject_id IS 'NULL means no content classification has been assigned.';
COMMENT ON COLUMN ple_private.draft_question_metadata.content_topic_id IS 'NULL means no narrower content classification has been assigned.';
COMMENT ON COLUMN ple_private.draft_question_metadata.content_subtopic_id IS 'NULL means no narrower content classification has been assigned.';
COMMENT ON COLUMN ple_private.draft_question_authorship.author_account_id IS 'NULL means this author has no linked PLE account.';

COMMENT ON TABLE ple_private.question_folder IS 'role: current state, deleted by workspace delete and publication. HUMAN_GUIDANCE.md Question authoring.';

COMMENT ON TABLE ple_private.question_folder_entry IS 'role: current state, deleted by workspace delete and publication. HUMAN_GUIDANCE.md Question authoring.';

COMMENT ON TABLE ple_private.draft_question_image IS 'role: current state, deleted by workspace delete and publication. HUMAN_GUIDANCE.md Question authoring.';


SET LOCAL ROLE ple_private_owner;
COMMENT ON COLUMN ple_private.authoring_workspace.revoked_at IS 'NULL means this optional fact is absent.';
COMMENT ON COLUMN ple_private.draft_question_metadata.general_feedback IS 'NULL means this optional fact is absent.';
COMMENT ON COLUMN ple_private.draft_question_metadata.hint IS 'NULL means this optional fact is absent.';
COMMENT ON COLUMN ple_private.draft_question_metadata.worked_solution IS 'NULL means this optional fact is absent.';
COMMENT ON COLUMN ple_private.draft_question_metadata.question_license IS 'NULL means this optional fact is absent.';
COMMENT ON COLUMN ple_private.draft_question.parent_published_question_id IS 'NULL means this Draft Question was not forked from another Published Question.';
COMMENT ON COLUMN ple_private.draft_question.parent_revision_number IS 'NULL means no parent Revision; otherwise this is the exact immediate source Revision.';
COMMENT ON COLUMN ple_private.draft_question_metadata.content_discipline_id IS 'NULL means no shared content classification is assigned to this Draft.';
COMMENT ON COLUMN ple_private.draft_question_metadata.content_subject_id IS 'NULL means no shared content classification is assigned to this Draft.';
COMMENT ON COLUMN ple_private.draft_question_metadata.content_topic_id IS 'NULL means this optional classification level is absent.';
COMMENT ON COLUMN ple_private.draft_question_metadata.content_subtopic_id IS 'NULL means this optional classification level is absent.';
COMMENT ON COLUMN ple_private.draft_question_metadata.citation_text IS 'NULL means this optional fact is absent.';
COMMENT ON COLUMN ple_private.draft_question_metadata.language IS 'NULL means this optional fact is absent.';
COMMENT ON COLUMN ple_private.draft_question_metadata.bloom_cognitive_process IS 'NULL means this optional fact is absent.';
COMMENT ON COLUMN ple_private.draft_question_metadata.bloom_knowledge_dimension IS 'NULL means this optional fact is absent.';
COMMENT ON COLUMN ple_private.draft_question_authorship.author_account_id IS 'NULL means this author is not linked to a PLE Account.';
COMMENT ON COLUMN ple_private.draft_question_source_binding.question_type IS 'NULL means no Question Type has been assigned to this Draft; publication requires a supported Question Type.';
COMMENT ON COLUMN ple_private.draft_question_source_binding.webwork_pg_path IS 'NULL means this optional fact is absent.';
COMMENT ON COLUMN ple_private.question_revision_source_binding.native_question_type IS 'NULL means the exact source is WeBWorK; Native Type is immutable source interaction evidence.';
COMMENT ON COLUMN ple_private.question_revision_source_binding.webwork_pg_path IS 'NULL means this optional fact is absent.';
COMMENT ON COLUMN ple_private.workspace_import.committed_at IS 'NULL means this optional fact is absent.';
