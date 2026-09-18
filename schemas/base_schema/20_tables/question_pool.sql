-- question_pool tables. CREATE TABLE and COMMENT ON only.
-- Functions, policies, and privileges live in later layers.

SET LOCAL ROLE ple_data_owner;

CREATE TABLE ple_data.question_pool (
    question_pool_id uuid PRIMARY KEY,
    -- The stored `XXXX-ZXXX` value is the public Pool ID. Its seven random
    -- Crockford characters are checked by the sixth checksum character; the
    -- database remains the final collision authority and owns no secret.
    public_question_pool_id text NOT NULL UNIQUE CHECK (
        public_question_pool_id ~ '^[0-9A-HJKMNP-TV-Z]{4}-[0-9A-HJKMNP-TV-Z]{4}$'
        AND substr(public_question_pool_id, 6, 1) = ple_private.crockford_checksum_character(
            substr(public_question_pool_id, 1, 4) || substr(public_question_pool_id, 7, 3)
        )
    ),
    metadata_etag uuid NOT NULL,
    title text NOT NULL CHECK (
        title = btrim(title) AND char_length(title) BETWEEN 1 AND 512
        AND title !~ '[[:cntrl:]]'
    ),
    description text NOT NULL CHECK (
        description = btrim(description) AND char_length(description) BETWEEN 1 AND 4000
        AND description !~ '[[:cntrl:]]'
    ),
    -- Pool search metadata is its own lineage state, not live member metadata.
    content_discipline_id uuid NOT NULL,
    content_subject_id uuid NOT NULL,
    content_topic_id uuid,
    content_subtopic_id uuid,
    tags text[] NOT NULL DEFAULT ARRAY[]::text[]
        CHECK (ple_data.question_metadata_tags_are_valid(tags)),
    FOREIGN KEY (content_subject_id, content_discipline_id)
        REFERENCES ple_data.content_subject_discipline(content_subject_id, content_discipline_id),
    FOREIGN KEY (content_subject_id, content_topic_id)
        REFERENCES ple_data.content_topic(content_subject_id, content_topic_id),
    FOREIGN KEY (content_topic_id, content_subtopic_id)
        REFERENCES ple_data.content_subtopic(content_topic_id, content_subtopic_id),
    CHECK (content_subtopic_id IS NULL OR content_topic_id IS NOT NULL),
    current_revision_number bigint NOT NULL DEFAULT 1 CHECK (current_revision_number > 0),
    -- A fork is a new immutable lineage.  Its source names one exact immutable
    -- published Pool Revision; original published Pools have no source pair.
    source_question_pool_id uuid,
    source_question_pool_revision_number bigint,
    created_at timestamptz NOT NULL,
    CHECK ((source_question_pool_id IS NULL) = (source_question_pool_revision_number IS NULL)),
    updated_on date NOT NULL DEFAULT CURRENT_DATE
);




-- A Question Pool is a stable lineage. This narrow schema makes an exact
-- immutable revision reference representable without inventing Pool contents,
-- current state, ownership, or a draft lifecycle.
CREATE TABLE ple_data.question_pool_revision (
    question_pool_id uuid NOT NULL
        REFERENCES ple_data.question_pool(question_pool_id),
    revision_number integer NOT NULL CHECK (revision_number > 0),
    member_count integer NOT NULL CHECK (member_count > 0),
    interchangeability_attested_by_account_id uuid NOT NULL
        REFERENCES ple_private.account(account_id),
    interchangeability_attested_at timestamptz NOT NULL,
    created_at timestamptz NOT NULL,
    -- ASVS 2.3.3: private immutable provenance for transaction-local child attachment.
    -- Full top-level XIDs survive savepoint release without xmin's frozen-row aliasing.
    created_in_transaction xid8 NOT NULL DEFAULT pg_current_xact_id(),
    PRIMARY KEY (question_pool_id, revision_number)
);

CREATE TABLE ple_data.question_pool_revision_member (
    question_pool_id uuid NOT NULL,
    revision_number integer NOT NULL,
    member_position integer NOT NULL CHECK (member_position > 0),
    published_question_id text NOT NULL,
    question_revision_number integer NOT NULL CHECK (question_revision_number > 0),
    PRIMARY KEY (question_pool_id, revision_number, member_position),
    UNIQUE (question_pool_id, revision_number, published_question_id, question_revision_number),
    FOREIGN KEY (question_pool_id, revision_number)
        REFERENCES ple_data.question_pool_revision(question_pool_id, revision_number),
    FOREIGN KEY (published_question_id, question_revision_number)
        REFERENCES ple_data.question_revision(published_question_id, revision_number),
    created_at timestamptz NOT NULL DEFAULT pg_catalog.transaction_timestamp()
);


SET LOCAL ROLE ple_data_owner;
COMMENT ON TABLE ple_data.question_pool IS 'role: current state, Stable Question Pool lineage, immutable exact source-Pool provenance for forks, server-issued canonical public Crockford ID, and current metadata ETag.';

COMMENT ON TABLE ple_data.question_pool_revision IS 'role: revision, Append-only sequential immutable Pool Revision with creating Instructor attestation and exact member count.';

COMMENT ON TABLE ple_data.question_pool_revision_member IS 'role: revision, Ordered distinct exact Published Question Revision pins; backend-neutral and intentionally no selected count.';

CREATE TABLE ple_data.question_pool_star (
    public_question_pool_id text NOT NULL REFERENCES ple_data.question_pool(public_question_pool_id),
    instructor_account_id uuid NOT NULL REFERENCES ple_private.account(account_id),
    starred_at timestamptz NOT NULL,
    PRIMARY KEY (public_question_pool_id, instructor_account_id),
    updated_at timestamptz NOT NULL DEFAULT pg_catalog.transaction_timestamp()
);


CREATE TABLE ple_data.question_pool_watch (
    public_question_pool_id text NOT NULL REFERENCES ple_data.question_pool(public_question_pool_id),
    instructor_account_id uuid NOT NULL REFERENCES ple_private.account(account_id),
    watched_at timestamptz NOT NULL,
    PRIMARY KEY (public_question_pool_id, instructor_account_id),
    updated_at timestamptz NOT NULL DEFAULT pg_catalog.transaction_timestamp()
);


CREATE TABLE ple_data.question_pool_revision_bloom (
    question_pool_id uuid NOT NULL,
    revision_number integer NOT NULL,
    cognitive_process ple_data.bloom_cognitive_process NOT NULL,
    knowledge_dimension ple_data.bloom_knowledge_dimension NOT NULL,
    classification_edit_number bigint NOT NULL CHECK (classification_edit_number > 0),
    PRIMARY KEY (question_pool_id, revision_number),
    FOREIGN KEY (question_pool_id, revision_number)
        REFERENCES ple_data.question_pool_revision (question_pool_id, revision_number),
    created_at timestamptz NOT NULL DEFAULT pg_catalog.transaction_timestamp()
);


COMMENT ON TABLE ple_data.question_pool_star IS 'role: current state, deleted by none for published Pools. HUMAN_GUIDANCE.md Question Pool specifications.';

COMMENT ON TABLE ple_data.question_pool_watch IS 'role: current state, deleted by none for published Pools. HUMAN_GUIDANCE.md Question Pool specifications.';

COMMENT ON TABLE ple_data.question_pool_revision_bloom IS 'role: revision, deleted by none for published Pools. HUMAN_GUIDANCE.md Question Pool specifications.';



COMMENT ON TABLE ple_data.question_pool_star IS 'role: current state, deleted by none for published Pools. HUMAN_GUIDANCE.md Question Pool specifications.';

COMMENT ON TABLE ple_data.question_pool_watch IS 'role: current state, deleted by none for published Pools. HUMAN_GUIDANCE.md Question Pool specifications.';

COMMENT ON TABLE ple_data.question_pool_revision_bloom IS 'role: revision, deleted by none for published Pools. HUMAN_GUIDANCE.md Question Pool specifications.';

