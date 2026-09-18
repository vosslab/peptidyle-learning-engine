-- question_pool tables. CREATE TABLE and COMMENT ON only.
-- Functions, policies, and privileges live in later layers.

SET LOCAL ROLE ple_data_owner;

CREATE TABLE ple_data.question_pool (
    -- The stored `XXXX-ZXXX` value is the public Pool ID. Its seven random
    -- Crockford characters are checked by the sixth checksum character; the
    -- database remains the final collision authority and owns no secret.
    question_pool_id ple_data.question_family_id PRIMARY KEY,
    question_pool_edit_number bigint NOT NULL CHECK (question_pool_edit_number > 0),
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
    interchangeability_attested_by_account_id ple_data.account_id NOT NULL
        REFERENCES ple_private.account(account_id),
    interchangeability_attested_at timestamptz NOT NULL,
    -- A fork is a new lineage. Its source names one published Pool; original
    -- published Pools have no source.
    source_question_pool_id ple_data.question_family_id
        REFERENCES ple_data.question_pool(question_pool_id),
    created_at timestamptz NOT NULL,
    -- ASVS 2.3.3: private provenance for transaction-local fork attachment.
    created_in_transaction xid8 NOT NULL DEFAULT pg_current_xact_id(),
    updated_on date NOT NULL DEFAULT CURRENT_DATE
);

CREATE TABLE ple_data.question_pool_member (
    question_pool_id ple_data.question_family_id NOT NULL
        REFERENCES ple_data.question_pool(question_pool_id),
    member_position integer NOT NULL CHECK (member_position > 0),
    published_question_id ple_data.question_family_id NOT NULL,
    question_revision_number integer NOT NULL CHECK (question_revision_number > 0),
    PRIMARY KEY (question_pool_id, member_position),
    UNIQUE (question_pool_id, published_question_id, question_revision_number),
    FOREIGN KEY (published_question_id, question_revision_number)
        REFERENCES ple_data.question_revision(published_question_id, revision_number),
    created_at timestamptz NOT NULL,
    updated_at timestamptz NOT NULL DEFAULT pg_catalog.transaction_timestamp(),
    CHECK (updated_at >= created_at)
);

CREATE TABLE ple_data.question_pool_star (
    question_pool_id ple_data.question_family_id NOT NULL REFERENCES ple_data.question_pool(question_pool_id),
    instructor_account_id ple_data.account_id NOT NULL REFERENCES ple_private.account(account_id),
    starred_at timestamptz NOT NULL,
    PRIMARY KEY (question_pool_id, instructor_account_id),
    updated_at timestamptz NOT NULL DEFAULT pg_catalog.transaction_timestamp()
);

CREATE TABLE ple_data.question_pool_watch (
    question_pool_id ple_data.question_family_id NOT NULL REFERENCES ple_data.question_pool(question_pool_id),
    instructor_account_id ple_data.account_id NOT NULL REFERENCES ple_private.account(account_id),
    watched_at timestamptz NOT NULL,
    PRIMARY KEY (question_pool_id, instructor_account_id),
    updated_at timestamptz NOT NULL DEFAULT pg_catalog.transaction_timestamp()
);

CREATE TABLE ple_data.question_pool_bloom (
    question_pool_id ple_data.question_family_id NOT NULL
        REFERENCES ple_data.question_pool(question_pool_id),
    cognitive_process ple_data.bloom_cognitive_process NOT NULL,
    knowledge_dimension ple_data.bloom_knowledge_dimension NOT NULL,
    classification_edit_number bigint NOT NULL CHECK (classification_edit_number > 0),
    PRIMARY KEY (question_pool_id),
    created_at timestamptz NOT NULL DEFAULT pg_catalog.transaction_timestamp(),
    updated_at timestamptz NOT NULL DEFAULT pg_catalog.transaction_timestamp(),
    CHECK (updated_at >= created_at)
);

SET LOCAL ROLE ple_data_owner;
COMMENT ON TABLE ple_data.question_pool IS 'role: current state, Stable Question Pool lineage, current member-list Edit Number, interchangeability attestation, immutable exact source-Pool provenance for forks, and server-issued canonical public Crockford ID.';
COMMENT ON TABLE ple_data.question_pool_member IS 'role: current state, deleted by Pool delete or member-list replace. Ordered distinct exact Published Question Revision pins; backend-neutral and intentionally no selected count.';
COMMENT ON TABLE ple_data.question_pool_star IS 'role: current state, deleted by none for published Pools. HUMAN_GUIDANCE.md Question Pool specifications.';
COMMENT ON TABLE ple_data.question_pool_watch IS 'role: current state, deleted by none for published Pools. HUMAN_GUIDANCE.md Question Pool specifications.';
COMMENT ON TABLE ple_data.question_pool_bloom IS 'role: current state, Current Pool Bloom classification and classification Edit Number. HUMAN_GUIDANCE.md Question Pool specifications.';
COMMENT ON COLUMN ple_data.question_pool.content_topic_id IS 'NULL means this optional fact is absent.';
COMMENT ON COLUMN ple_data.question_pool.content_subtopic_id IS 'NULL means this optional fact is absent.';
COMMENT ON COLUMN ple_data.question_pool.source_question_pool_id IS 'NULL means this optional fact is absent.';
