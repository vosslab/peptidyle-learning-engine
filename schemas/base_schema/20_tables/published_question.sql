-- published_question tables. CREATE TABLE and COMMENT ON only.
-- Functions, policies, and privileges live in later layers.

SET LOCAL ROLE ple_data_owner;

CREATE TABLE ple_data.published_question (
    published_question_id ple_data.question_family_id PRIMARY KEY,
    parent_published_question_id ple_data.question_family_id,
    parent_revision_number integer CHECK (
        parent_revision_number IS NULL OR parent_revision_number > 0
    ),
    CHECK (
        (parent_published_question_id IS NULL) = (parent_revision_number IS NULL)
    ),
    CHECK (
        parent_published_question_id IS NULL
        OR parent_published_question_id <> published_question_id
    ),
    availability ple_data.question_availability NOT NULL DEFAULT 'available',
    availability_edit_number bigint NOT NULL DEFAULT 1
        CHECK (availability_edit_number > 0),
    created_at timestamptz NOT NULL,

    updated_on date NOT NULL DEFAULT CURRENT_DATE
);


CREATE TABLE ple_data.question_revision (
    published_question_id ple_data.question_family_id NOT NULL REFERENCES ple_data.published_question(published_question_id),
    revision_number integer NOT NULL CHECK (revision_number > 0),
    backend ple_data.question_backend NOT NULL,
    -- Deliberately authored, backend-independent general feedback.  It is
    -- immutable with this Question Revision; dynamic backend feedback is not
    -- captured here.
    general_feedback text CHECK (
        general_feedback = btrim(general_feedback)
        AND char_length(general_feedback) BETWEEN 1 AND 4000
        AND general_feedback !~ '[[:cntrl:]]'
    ),
    -- ASVS 2.2.1 and 2.2.2: optional PLE-managed Hint text. NULL is absent.
    -- This column is not WeBWorK source text and is not backend feedback.
    hint text CHECK (
        hint = btrim(hint)
        AND char_length(hint) BETWEEN 1 AND 4000
        AND hint !~ '[[:cntrl:]]'
    ),
    -- ASVS 2.2.1 and 2.2.2: optional PLE-managed Worked Solution text.
    -- NULL is absent. This column is not WeBWorK source text.
    worked_solution text CHECK (
        worked_solution = btrim(worked_solution)
        AND char_length(worked_solution) BETWEEN 1 AND 4000
        AND worked_solution !~ '[[:cntrl:]]'
    ),
    published_at timestamptz NOT NULL,
    PRIMARY KEY (published_question_id, revision_number)
);


CREATE TABLE ple_data.question_revision_metadata (
    published_question_id ple_data.question_family_id NOT NULL,
    revision_number integer NOT NULL CHECK (revision_number > 0),
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
    language text CHECK (
        language = btrim(language) AND char_length(language) BETWEEN 2 AND 35
    ),
    question_type ple_data.question_type NOT NULL,
    -- Permitted metadata corrections update this complete Revision in place.
    -- The edit number detects conflicts without becoming another identity.
    metadata_edit_number bigint NOT NULL DEFAULT 1 CHECK (metadata_edit_number > 0),
    tags text[] NOT NULL DEFAULT ARRAY[]::text[]
        CHECK (ple_data.question_metadata_tags_are_valid(tags)),
    -- ASVS 2.2.2/2.3.3: real vocabulary references preserve the hierarchy
    -- even during concurrent vocabulary repairs; no free-text bridge exists.
    content_discipline_id uuid NOT NULL,
    content_subject_id uuid NOT NULL,
    content_topic_id uuid,
    content_subtopic_id uuid,
    bloom_cognitive_process ple_data.bloom_cognitive_process,
    bloom_knowledge_dimension ple_data.bloom_knowledge_dimension,
    PRIMARY KEY (published_question_id, revision_number),
    FOREIGN KEY (published_question_id, revision_number)
        REFERENCES ple_data.question_revision(published_question_id, revision_number),
    FOREIGN KEY (content_subject_id, content_discipline_id)
        REFERENCES ple_data.content_subject_discipline(content_subject_id, content_discipline_id),
    FOREIGN KEY (content_subject_id, content_topic_id)
        REFERENCES ple_data.content_topic(content_subject_id, content_topic_id),
    FOREIGN KEY (content_topic_id, content_subtopic_id)
        REFERENCES ple_data.content_subtopic(content_topic_id, content_subtopic_id),
    CHECK (content_subtopic_id IS NULL OR content_topic_id IS NOT NULL),
    created_at timestamptz NOT NULL,
    updated_at timestamptz NOT NULL CHECK (updated_at >= created_at)
);

CREATE TABLE ple_data.question_publication_event (
    event_id uuid PRIMARY KEY,
    published_question_id ple_data.question_family_id NOT NULL,
    revision_number integer NOT NULL CHECK (revision_number > 0),
    actor_account_id ple_data.account_id NOT NULL REFERENCES ple_private.account(account_id),
    occurred_at timestamptz NOT NULL,
    UNIQUE (published_question_id, revision_number),
    FOREIGN KEY (published_question_id, revision_number)
        REFERENCES ple_data.question_revision(published_question_id, revision_number)
);

CREATE TABLE ple_data.question_availability_event (
    event_id uuid PRIMARY KEY,
    published_question_id ple_data.question_family_id NOT NULL REFERENCES ple_data.published_question(published_question_id),
    actor_account_id ple_data.account_id NOT NULL REFERENCES ple_private.account(account_id),
    availability ple_data.question_availability NOT NULL,
    edit_number bigint NOT NULL CHECK (edit_number > 0),
    reason text,
    occurred_at timestamptz NOT NULL,
    UNIQUE (published_question_id, edit_number),
    CHECK (
        (availability = 'available' AND reason IS NULL)
        OR (availability = 'archived' AND reason = btrim(reason)
            AND char_length(reason) BETWEEN 1 AND 1000)
    )
);


SET LOCAL ROLE ple_data_owner;
COMMENT ON TABLE ple_data.published_question IS 'role: current state, Stable Question lineage with current availability and its qualified edit number.';

COMMENT ON TABLE ple_data.question_revision IS 'role: revision, Immutable exact published Question content identity; archive never removes this provenance.';

COMMENT ON TABLE ple_data.question_availability_event IS 'role: event, Append-only actor-attributed current-lineage availability transitions.';

CREATE TABLE ple_data.question_revision_acceptance (
    published_question_id ple_data.question_family_id NOT NULL,
    revision_number integer NOT NULL CHECK (revision_number > 0),
    parent_revision_number integer,
    editor_account_id ple_data.account_id NOT NULL REFERENCES ple_private.account(account_id),
    accepted_by_account_id ple_data.account_id NOT NULL REFERENCES ple_private.account(account_id),
    accepted_at timestamptz NOT NULL,
    reason_for_edit text NOT NULL CHECK (
        reason_for_edit = btrim(reason_for_edit)
        AND char_length(reason_for_edit) BETWEEN 1 AND 2000
        AND reason_for_edit !~ '[[:cntrl:]]'
    ),
    PRIMARY KEY (published_question_id, revision_number),
    FOREIGN KEY (published_question_id, revision_number)
        REFERENCES ple_data.question_revision(published_question_id, revision_number),
    FOREIGN KEY (published_question_id, parent_revision_number)
        REFERENCES ple_data.question_revision(published_question_id, revision_number),
    CHECK ((revision_number = 1 AND parent_revision_number IS NULL)
        OR (revision_number > 1 AND parent_revision_number BETWEEN 1 AND revision_number - 1))
);

CREATE TABLE ple_data.question_revision_authorship (
    published_question_id ple_data.question_family_id NOT NULL,
    revision_number integer NOT NULL,
    author_position integer NOT NULL CHECK (author_position BETWEEN 1 AND 16),
    author_display_name text NOT NULL CHECK (
        author_display_name = btrim(author_display_name)
        AND char_length(author_display_name) BETWEEN 1 AND 120
        AND author_display_name !~ '[[:cntrl:]]'
    ),
    author_account_id ple_data.account_id REFERENCES ple_private.account(account_id),
    PRIMARY KEY (published_question_id, revision_number, author_position),
    UNIQUE (published_question_id, revision_number, author_display_name),
    FOREIGN KEY (published_question_id, revision_number)
        REFERENCES ple_data.question_revision(published_question_id, revision_number),
    created_at timestamptz NOT NULL DEFAULT pg_catalog.transaction_timestamp(),
    updated_at timestamptz NOT NULL DEFAULT pg_catalog.transaction_timestamp(),
    CHECK (updated_at >= created_at)
);


CREATE TABLE ple_data.question_revision_license (
    published_question_id ple_data.question_family_id NOT NULL,
    revision_number integer NOT NULL,
    spdx_expression ple_data.license_spdx NOT NULL,
    PRIMARY KEY (published_question_id, revision_number),
    FOREIGN KEY (published_question_id, revision_number)
        REFERENCES ple_data.question_revision(published_question_id, revision_number),
    created_at timestamptz NOT NULL DEFAULT pg_catalog.transaction_timestamp(),
    updated_at timestamptz NOT NULL DEFAULT pg_catalog.transaction_timestamp(),
    CHECK (updated_at >= created_at)
);


CREATE TABLE ple_data.question_revision_citation (
    published_question_id ple_data.question_family_id NOT NULL,
    revision_number integer NOT NULL,
    citation_text text,
    PRIMARY KEY (published_question_id, revision_number),
    FOREIGN KEY (published_question_id, revision_number)
        REFERENCES ple_data.question_revision(published_question_id, revision_number),
    created_at timestamptz NOT NULL DEFAULT pg_catalog.transaction_timestamp(),
    updated_at timestamptz NOT NULL DEFAULT pg_catalog.transaction_timestamp(),
    CHECK (updated_at >= created_at)
);


CREATE TABLE ple_data.question_ownership_event (
    question_ownership_event_id uuid PRIMARY KEY,
    published_question_id ple_data.question_family_id NOT NULL REFERENCES ple_data.published_question(published_question_id),
    owner_account_id ple_data.account_id NOT NULL REFERENCES ple_private.account(account_id),
    recorded_by_account_id ple_data.account_id NOT NULL REFERENCES ple_private.account(account_id),
    event_kind ple_data.ownership_event_kind NOT NULL,
    occurred_at timestamptz NOT NULL
);


-- A Star is a present, public-in-principle endorsement of a Published
-- Question lineage.  It intentionally has no revision key: the endorsement
-- belongs to the Question across its immutable Revisions.  C370 owns the
-- application persistence adapter and C371 owns the active-Instructor count
-- and identity projection; this table is not itself a browser projection.
CREATE TABLE ple_data.question_star (
    published_question_id ple_data.question_family_id NOT NULL REFERENCES ple_data.published_question(published_question_id),
    instructor_account_id ple_data.account_id NOT NULL REFERENCES ple_private.account(account_id),
    starred_at timestamptz NOT NULL,
    PRIMARY KEY (published_question_id, instructor_account_id),
    updated_at timestamptz NOT NULL DEFAULT pg_catalog.transaction_timestamp()
);


-- A Watch is a private subscription to a Published Question lineage.  It has
-- no revision key because it follows the Question across immutable Revisions.
-- C372 owns the application persistence adapter and C373 owns the private
-- browser projection.  This store intentionally has no watcher count,
-- identity projection, notification delivery, or public read path.
CREATE TABLE ple_data.question_watch (
    published_question_id ple_data.question_family_id NOT NULL REFERENCES ple_data.published_question(published_question_id),
    instructor_account_id ple_data.account_id NOT NULL REFERENCES ple_private.account(account_id),
    watched_at timestamptz NOT NULL,
    PRIMARY KEY (published_question_id, instructor_account_id),
    updated_at timestamptz NOT NULL DEFAULT pg_catalog.transaction_timestamp()
);


SET LOCAL ROLE ple_data_owner;

COMMENT ON TABLE ple_data.question_revision_metadata IS 'role: revision, ordinary metadata corrections preserve the exact Question Revision including independently nullable Bloom dimensions; HUMAN_GUIDANCE.md Published Question revisions and edits.';

COMMENT ON COLUMN ple_data.question_revision_metadata.question_type IS 'Question Type for this exact Revision; WeBWorK edits use metadata CAS, while Native Type must match its source interaction.';

COMMENT ON TABLE ple_data.question_publication_event IS 'role: event, deleted by none for published lineage; Draft rows follow workspace delete. HUMAN_GUIDANCE.md Published Questions.';

COMMENT ON TABLE ple_data.question_revision_acceptance IS 'role: event, deleted by none for published lineage; Draft rows follow workspace delete. HUMAN_GUIDANCE.md Published Questions.';

COMMENT ON TABLE ple_data.question_revision_authorship IS 'role: current state, deleted by none for published lineage; Draft rows follow workspace delete. HUMAN_GUIDANCE.md Published Questions.';

COMMENT ON TABLE ple_data.question_revision_license IS 'role: current state, deleted by none for published lineage; Draft rows follow workspace delete. HUMAN_GUIDANCE.md Published Questions.';

COMMENT ON TABLE ple_data.question_revision_citation IS 'role: current state, deleted by none for published lineage; Draft rows follow workspace delete. HUMAN_GUIDANCE.md Published Questions.';

COMMENT ON TABLE ple_data.question_ownership_event IS 'role: event, deleted by none for published lineage; Draft rows follow workspace delete. HUMAN_GUIDANCE.md Published Questions.';

COMMENT ON TABLE ple_data.question_star IS 'role: current state, deleted by none for published lineage; Draft rows follow workspace delete. HUMAN_GUIDANCE.md Published Questions.';

COMMENT ON TABLE ple_data.question_watch IS 'role: current state, deleted by none for published lineage; Draft rows follow workspace delete. HUMAN_GUIDANCE.md Published Questions.';


SET LOCAL ROLE ple_private_owner;

SET LOCAL ROLE ple_private_owner;

SET LOCAL ROLE ple_private_owner;

SET LOCAL ROLE ple_data_owner;

SET LOCAL ROLE ple_data_owner;


SET LOCAL ROLE ple_private_owner;

SET LOCAL ROLE ple_private_owner;


SET LOCAL ROLE ple_data_owner;

SET LOCAL ROLE ple_data_owner;


SET LOCAL ROLE ple_private_owner;

SET LOCAL ROLE ple_private_owner;


SET LOCAL ROLE ple_data_owner;

SET LOCAL ROLE ple_data_owner;


SET LOCAL ROLE ple_private_owner;

SET LOCAL ROLE ple_private_owner;

SET LOCAL ROLE ple_private_owner;

SET LOCAL ROLE ple_data_owner;

SET LOCAL ROLE ple_data_owner;


SET LOCAL ROLE ple_private_owner;

SET LOCAL ROLE ple_private_owner;


SET LOCAL ROLE ple_data_owner;

SET LOCAL ROLE ple_data_owner;


SET LOCAL ROLE ple_private_owner;

SET LOCAL ROLE ple_private_owner;

SET LOCAL ROLE ple_data_owner;
COMMENT ON COLUMN ple_data.question_revision.general_feedback IS 'NULL means this optional fact is absent.';
COMMENT ON COLUMN ple_data.question_revision.hint IS 'NULL means this optional fact is absent.';
COMMENT ON COLUMN ple_data.question_revision.worked_solution IS 'NULL means this optional fact is absent.';
COMMENT ON COLUMN ple_data.question_revision_metadata.content_topic_id IS 'NULL means this optional fact is absent.';
COMMENT ON COLUMN ple_data.question_revision_metadata.content_subtopic_id IS 'NULL means this optional fact is absent.';
COMMENT ON COLUMN ple_data.question_revision_metadata.bloom_cognitive_process IS 'NULL means this optional fact is absent.';
COMMENT ON COLUMN ple_data.question_revision_metadata.bloom_knowledge_dimension IS 'NULL means this optional fact is absent.';
COMMENT ON COLUMN ple_data.question_availability_event.reason IS 'NULL means this optional fact is absent.';
COMMENT ON COLUMN ple_data.question_revision_acceptance.parent_revision_number IS 'NULL means this optional fact is absent.';
COMMENT ON COLUMN ple_data.published_question.parent_published_question_id IS 'NULL means this Published Question was not forked from another Published Question.';
COMMENT ON COLUMN ple_data.published_question.parent_revision_number IS 'NULL means no parent Revision; otherwise this is the exact immediate source Revision.';
COMMENT ON COLUMN ple_data.question_revision_authorship.author_account_id IS 'NULL means this optional fact is absent.';
COMMENT ON COLUMN ple_data.question_revision_citation.citation_text IS 'NULL means this optional fact is absent.';
