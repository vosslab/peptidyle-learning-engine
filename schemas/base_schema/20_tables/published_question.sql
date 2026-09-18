-- published_question tables. CREATE TABLE and COMMENT ON only.
-- Functions, policies, and privileges live in later layers.

SET LOCAL ROLE ple_data_owner;

CREATE TABLE ple_data.published_question (
    question_id text PRIMARY KEY,
    availability text NOT NULL DEFAULT 'available'
        CHECK (availability IN ('available', 'archived')),
    availability_edit_number bigint NOT NULL DEFAULT 1
        CHECK (availability_edit_number > 0),
    created_at timestamptz NOT NULL,
    CONSTRAINT published_question_id_is_crockford_shape CHECK (
        question_id ~ '^[0-9A-HJKMNP-TV-Z]{4}-[0-9A-HJKMNP-TV-Z]{4}$'
        AND substr(question_id, 6, 1) = ple_private.crockford_checksum_character(
            substr(question_id, 1, 4) || substr(question_id, 7, 3)
        )
    )
);

CREATE TABLE ple_data.question_revision (
    question_id text NOT NULL REFERENCES ple_data.published_question(question_id),
    revision_number integer NOT NULL CHECK (revision_number > 0),
    backend text NOT NULL CHECK (backend IN ('ple', 'webwork', 'imathas')),
    question_type text NOT NULL CHECK (question_type IN (
        'multipleChoice', 'multipleAnswer', 'fillInBlank', 'multipleFillInBlank',
        'numeric', 'matching', 'ordering', 'hotspot'
    )),
    -- Deliberately authored, backend-independent general feedback.  It is
    -- immutable with this Question Revision; dynamic backend feedback is not
    -- captured here.
    general_feedback text CHECK (
        general_feedback = btrim(general_feedback)
        AND char_length(general_feedback) BETWEEN 1 AND 4000
        AND general_feedback !~ '[[:cntrl:]]'
    ),
    published_at timestamptz NOT NULL,
    PRIMARY KEY (question_id, revision_number)
);

CREATE TABLE ple_data.published_question_metadata (
    question_id text PRIMARY KEY REFERENCES ple_data.published_question(question_id),
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
    language text NOT NULL CHECK (
        language = btrim(language) AND char_length(language) BETWEEN 2 AND 35
    ),
    -- Search metadata belongs to the Published Question lineage, not to an
    -- immutable Revision.  C365 is the only bulk writer and advances this
    -- independent optimistic-concurrency value.
    metadata_edit_number bigint NOT NULL DEFAULT 1 CHECK (metadata_edit_number > 0),
    tags text[] NOT NULL DEFAULT ARRAY[]::text[]
        CHECK (ple_data.question_metadata_tags_are_valid(tags)),
    -- ASVS 2.2.2/2.3.3: real vocabulary references preserve the hierarchy
    -- even during concurrent vocabulary repairs; no free-text bridge exists.
    discipline_uuid uuid NOT NULL,
    subject_uuid uuid NOT NULL,
    topic_uuid uuid,
    subtopic_uuid uuid,
    FOREIGN KEY (subject_uuid, discipline_uuid)
        REFERENCES ple_data.content_subject_discipline(subject_uuid, discipline_uuid),
    FOREIGN KEY (subject_uuid, topic_uuid)
        REFERENCES ple_data.content_topic(subject_uuid, topic_uuid),
    FOREIGN KEY (topic_uuid, subtopic_uuid)
        REFERENCES ple_data.content_subtopic(topic_uuid, subtopic_uuid),
    CHECK (subtopic_uuid IS NULL OR topic_uuid IS NOT NULL),
    created_at timestamptz NOT NULL,
    updated_at timestamptz NOT NULL CHECK (updated_at >= created_at)
);

CREATE TABLE ple_data.question_publication_event (
    event_id uuid PRIMARY KEY,
    question_id text NOT NULL,
    revision_number integer NOT NULL CHECK (revision_number > 0),
    actor_account_id uuid NOT NULL REFERENCES ple_private.account(account_id),
    occurred_at timestamptz NOT NULL,
    UNIQUE (question_id, revision_number),
    FOREIGN KEY (question_id, revision_number)
        REFERENCES ple_data.question_revision(question_id, revision_number)
);

CREATE TABLE ple_data.question_availability_event (
    event_id uuid PRIMARY KEY,
    question_id text NOT NULL REFERENCES ple_data.published_question(question_id),
    actor_account_id uuid NOT NULL REFERENCES ple_private.account(account_id),
    availability text NOT NULL CHECK (availability IN ('available', 'archived')),
    edit_number bigint NOT NULL CHECK (edit_number > 0),
    reason text,
    occurred_at timestamptz NOT NULL,
    UNIQUE (question_id, edit_number),
    CHECK (
        (availability = 'available' AND reason IS NULL)
        OR (availability = 'archived' AND reason = btrim(reason)
            AND char_length(reason) BETWEEN 1 AND 1000)
    )
);

COMMENT ON TABLE ple_data.published_question IS 'role: current state, Stable Question lineage with current availability and its qualified edit number.';

COMMENT ON TABLE ple_data.question_revision IS 'role: revision, Immutable exact published Question content identity; archive never removes this provenance.';

COMMENT ON TABLE ple_data.question_availability_event IS 'role: event, Append-only actor-attributed current-lineage availability transitions.';

CREATE TABLE ple_data.question_revision_acceptance (
    question_id text NOT NULL,
    revision_number integer NOT NULL CHECK (revision_number > 0),
    parent_revision_number integer,
    editor_account_id uuid NOT NULL REFERENCES ple_private.account(account_id),
    accepted_by_account_id uuid NOT NULL REFERENCES ple_private.account(account_id),
    accepted_at timestamptz NOT NULL,
    reason_for_edit text NOT NULL CHECK (
        reason_for_edit = btrim(reason_for_edit)
        AND char_length(reason_for_edit) BETWEEN 1 AND 2000
        AND reason_for_edit !~ '[[:cntrl:]]'
    ),
    PRIMARY KEY (question_id, revision_number),
    FOREIGN KEY (question_id, revision_number)
        REFERENCES ple_data.question_revision(question_id, revision_number),
    FOREIGN KEY (question_id, parent_revision_number)
        REFERENCES ple_data.question_revision(question_id, revision_number),
    CHECK ((revision_number = 1 AND parent_revision_number IS NULL)
        OR (revision_number > 1 AND parent_revision_number BETWEEN 1 AND revision_number - 1))
);

CREATE TABLE ple_data.question_revision_authorship (
    question_id text NOT NULL,
    revision_number integer NOT NULL,
    author_position integer NOT NULL CHECK (author_position BETWEEN 1 AND 16),
    author_display_name text NOT NULL CHECK (
        author_display_name = btrim(author_display_name)
        AND char_length(author_display_name) BETWEEN 1 AND 120
        AND author_display_name !~ '[[:cntrl:]]'
    ),
    author_account_id uuid REFERENCES ple_private.account(account_id),
    PRIMARY KEY (question_id, revision_number, author_position),
    UNIQUE (question_id, revision_number, author_display_name),
    FOREIGN KEY (question_id, revision_number)
        REFERENCES ple_data.question_revision(question_id, revision_number)
);

CREATE TABLE ple_data.question_revision_license (
    question_id text NOT NULL,
    revision_number integer NOT NULL,
    spdx_expression text NOT NULL CHECK (spdx_expression IN (
        'CC0-1.0', 'CC-BY-4.0', 'CC-BY-SA-4.0'
    )),
    PRIMARY KEY (question_id, revision_number),
    FOREIGN KEY (question_id, revision_number)
        REFERENCES ple_data.question_revision(question_id, revision_number)
);

CREATE TABLE ple_data.question_revision_citation (
    question_id text NOT NULL,
    revision_number integer NOT NULL,
    citation_url text,
    citation_text text,
    PRIMARY KEY (question_id, revision_number),
    FOREIGN KEY (question_id, revision_number)
        REFERENCES ple_data.question_revision(question_id, revision_number),
    CHECK (NULLIF(btrim(citation_url), '') IS NOT NULL
        OR NULLIF(btrim(citation_text), '') IS NOT NULL),
    CHECK (citation_url IS NULL OR char_length(btrim(citation_url)) <= 2048),
    CHECK (citation_text IS NULL OR char_length(btrim(citation_text)) <= 4000)
);

CREATE TABLE ple_data.question_ownership_event (
    question_ownership_event_id uuid PRIMARY KEY,
    question_id text NOT NULL REFERENCES ple_data.published_question(question_id),
    owner_account_id uuid NOT NULL REFERENCES ple_private.account(account_id),
    recorded_by_account_id uuid NOT NULL REFERENCES ple_private.account(account_id),
    event_kind text NOT NULL CHECK (event_kind IN ('initial', 'transferred')),
    occurred_at timestamptz NOT NULL
);

CREATE TABLE ple_data.question_fork_source (
    forked_question_id text PRIMARY KEY REFERENCES ple_data.published_question(question_id),
    source_question_id text NOT NULL,
    source_revision_number integer NOT NULL CHECK (source_revision_number > 0),
    recorded_at timestamptz NOT NULL,
    FOREIGN KEY (source_question_id, source_revision_number)
        REFERENCES ple_data.question_revision(question_id, revision_number),
    CHECK (forked_question_id <> source_question_id)
);



-- A Star is a present, public-in-principle endorsement of a Published
-- Question lineage.  It intentionally has no revision key: the endorsement
-- belongs to the Question across its immutable Revisions.  C370 owns the
-- application persistence adapter and C371 owns the vetted-Instructor count
-- and identity projection; this table is not itself a browser projection.
CREATE TABLE ple_data.question_star (
    question_id text NOT NULL REFERENCES ple_data.published_question(question_id),
    instructor_account_id uuid NOT NULL REFERENCES ple_private.account(account_id),
    starred_at timestamptz NOT NULL,
    PRIMARY KEY (question_id, instructor_account_id)
);



-- A Watch is a private subscription to a Published Question lineage.  It has
-- no revision key because it follows the Question across immutable Revisions.
-- C372 owns the application persistence adapter and C373 owns the private
-- browser projection.  This store intentionally has no watcher count,
-- identity projection, notification delivery, or public read path.
CREATE TABLE ple_data.question_watch (
    question_id text NOT NULL REFERENCES ple_data.published_question(question_id),
    instructor_account_id uuid NOT NULL REFERENCES ple_private.account(account_id),
    watched_at timestamptz NOT NULL,
    PRIMARY KEY (question_id, instructor_account_id)
);

CREATE TABLE ple_data.question_revision_bloom (
    question_id text NOT NULL,
    revision_number integer NOT NULL,
    cognitive_process ple_data.bloom_cognitive_process NOT NULL,
    knowledge_dimension ple_data.bloom_knowledge_dimension NOT NULL,
    classification_edit_number bigint NOT NULL CHECK (classification_edit_number > 0),
    PRIMARY KEY (question_id, revision_number),
    FOREIGN KEY (question_id, revision_number)
        REFERENCES ple_data.question_revision (question_id, revision_number)
);

SET LOCAL ROLE ple_private_owner;

CREATE TABLE ple_private.bloom_preparation_receipt (
    bloom_preparation_receipt_id uuid PRIMARY KEY,
    target_kind ple_private.bloom_preparation_target_kind NOT NULL,
    candidate_fingerprint bytea NOT NULL CHECK (octet_length(candidate_fingerprint) = 32),
    cognitive_process ple_data.bloom_cognitive_process NOT NULL,
    knowledge_dimension ple_data.bloom_knowledge_dimension NOT NULL
);

SET LOCAL ROLE ple_data_owner;

COMMENT ON TABLE ple_data.published_question_metadata IS 'role: current state, deleted by none for published lineage; Draft rows follow workspace delete. HUMAN_GUIDANCE.md Published Questions.';

COMMENT ON TABLE ple_data.question_publication_event IS 'role: event, deleted by none for published lineage; Draft rows follow workspace delete. HUMAN_GUIDANCE.md Published Questions.';

COMMENT ON TABLE ple_data.question_revision_acceptance IS 'role: event, deleted by none for published lineage; Draft rows follow workspace delete. HUMAN_GUIDANCE.md Published Questions.';

COMMENT ON TABLE ple_data.question_revision_authorship IS 'role: current state, deleted by none for published lineage; Draft rows follow workspace delete. HUMAN_GUIDANCE.md Published Questions.';

COMMENT ON TABLE ple_data.question_revision_license IS 'role: current state, deleted by none for published lineage; Draft rows follow workspace delete. HUMAN_GUIDANCE.md Published Questions.';

COMMENT ON TABLE ple_data.question_revision_citation IS 'role: current state, deleted by none for published lineage; Draft rows follow workspace delete. HUMAN_GUIDANCE.md Published Questions.';

COMMENT ON TABLE ple_data.question_ownership_event IS 'role: event, deleted by none for published lineage; Draft rows follow workspace delete. HUMAN_GUIDANCE.md Published Questions.';

COMMENT ON TABLE ple_data.question_fork_source IS 'role: event, deleted by none for published lineage; Draft rows follow workspace delete. HUMAN_GUIDANCE.md Published Questions.';

COMMENT ON TABLE ple_data.question_star IS 'role: current state, deleted by none for published lineage; Draft rows follow workspace delete. HUMAN_GUIDANCE.md Published Questions.';

COMMENT ON TABLE ple_data.question_watch IS 'role: current state, deleted by none for published lineage; Draft rows follow workspace delete. HUMAN_GUIDANCE.md Published Questions.';

COMMENT ON TABLE ple_data.question_revision_bloom IS 'role: current state, deleted by none for published lineage; Draft rows follow workspace delete. HUMAN_GUIDANCE.md Published Questions.';

SET LOCAL ROLE ple_private_owner;

COMMENT ON TABLE ple_private.bloom_preparation_receipt IS 'role: event, deleted by none for published lineage; Draft rows follow workspace delete. HUMAN_GUIDANCE.md Published Questions.';

