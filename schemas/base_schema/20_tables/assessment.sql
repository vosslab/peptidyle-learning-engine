-- assessment tables. CREATE TABLE and COMMENT ON only.
-- Functions, policies, and privileges live in later layers.

SET LOCAL ROLE ple_data_owner;

-- Frozen Assessment policy is stored once and referenced. Two Assessments
-- with equal policy share one immutable snapshot row.
CREATE TABLE ple_data.assessment_policy_snapshot (
    assessment_policy_snapshot_id ple_data.sha256_digest PRIMARY KEY,
    assessment_title text NOT NULL CHECK (
        assessment_title ~ '[^[:space:]]' AND char_length(assessment_title) <= 200
    ),
    assessment_instructions text NOT NULL CHECK (
        assessment_instructions !~ E'\\x00' AND char_length(assessment_instructions) <= 50000
    ),
    available_at timestamptz,
    due_at timestamptz,
    closes_at timestamptz,
    assessment_attempt_time_limit_seconds integer CHECK (
        assessment_attempt_time_limit_seconds IS NULL
        OR assessment_attempt_time_limit_seconds BETWEEN 1 AND 43200
    ),
    assessment_attempt_limit integer CHECK (
        assessment_attempt_limit IS NULL OR assessment_attempt_limit > 0
    ),
    late_work_rule ple_data.late_work_rule NOT NULL,
    question_variation_rule ple_data.question_variation_rule NOT NULL,
    assessment_question_order_rule ple_data.question_order_rule NOT NULL,
    feedback_score ple_data.feedback_release NOT NULL,
    feedback_per_item_correctness ple_data.feedback_release NOT NULL,
    feedback_submitted_response ple_data.feedback_release NOT NULL,
    feedback_question_answer ple_data.feedback_release NOT NULL,
    feedback_question_answer_explanation ple_data.feedback_release NOT NULL,
    feedback_class_statistics ple_data.feedback_release NOT NULL,
    created_at timestamptz NOT NULL,
    CHECK (
        (available_at IS NULL OR due_at IS NULL OR available_at <= due_at)
        AND (due_at IS NULL OR closes_at IS NULL OR due_at <= closes_at)
    )
);

-- Current Course Assessment aggregates.  These relations describe the content
-- used for future Assessment Attempts; student_work.sql retains the facts used to
-- interpret an Assessment Attempt after a later Assessment edit.
CREATE TABLE ple_data.assessment (
    assessment_id ple_data.assessment_id PRIMARY KEY,
    course_instance_id ple_data.course_instance_id NOT NULL REFERENCES ple_data.course_instance(course_instance_id),
    origin_kind ple_data.assessment_origin_kind NOT NULL,
    source_blueprint_course_id ple_data.blueprint_course_id,
    source_blueprint_revision_number integer CHECK (source_blueprint_revision_number > 0),
    -- BlueprintAssessmentSource: an exact immutable Blueprint Revision plus
    -- the stable Assessment member selected from that Revision.
    source_blueprint_assessment_reference uuid,
    created_at timestamptz NOT NULL,
    updated_at timestamptz NOT NULL,
    assessment_edit_number bigint NOT NULL DEFAULT 1 CHECK (assessment_edit_number > 0),
    assessment_type ple_data.assessment_type NOT NULL,
    assessment_policy_snapshot_id ple_data.sha256_digest NOT NULL
        REFERENCES ple_data.assessment_policy_snapshot(assessment_policy_snapshot_id),
    assessment_status ple_data.assessment_status NOT NULL DEFAULT 'unreleased',
    UNIQUE (assessment_id, course_instance_id),
    FOREIGN KEY (
        source_blueprint_course_id,
        source_blueprint_revision_number,
        source_blueprint_assessment_reference
    ) REFERENCES ple_data.blueprint_revision_assessment (
        blueprint_course_id,
        blueprint_revision_number,
        blueprint_assessment_reference
    ),
    CHECK (
        (origin_kind = 'direct'
            AND source_blueprint_course_id IS NULL
            AND source_blueprint_revision_number IS NULL
            AND source_blueprint_assessment_reference IS NULL)
        OR (origin_kind = 'adopted'
            AND source_blueprint_course_id IS NOT NULL
            AND source_blueprint_revision_number IS NOT NULL
            AND source_blueprint_assessment_reference IS NOT NULL)
    ),
    CHECK (updated_at >= created_at)
);



CREATE TABLE ple_data.assessment_entry (
    assessment_entry_id uuid PRIMARY KEY,
    assessment_id ple_data.assessment_id NOT NULL REFERENCES ple_data.assessment(assessment_id),
    authored_position integer NOT NULL CHECK (authored_position >= 0),
    entry_kind ple_data.entry_kind NOT NULL,
    availability ple_data.entry_availability NOT NULL DEFAULT 'available',
    active_authored_position integer GENERATED ALWAYS AS (
        CASE WHEN availability = 'available' THEN authored_position END
    ) STORED,
    -- Kind-bound child keys. MATCH SIMPLE deferred FKs require the matching
    -- child and skip the other kind without a nullable pairing CHECK.
    assessment_entry_question_id uuid GENERATED ALWAYS AS (
        CASE WHEN entry_kind = 'fixed_question' THEN assessment_entry_id END
    ) STORED,
    assessment_entry_pool_id uuid GENERATED ALWAYS AS (
        CASE WHEN entry_kind = 'question_pool' THEN assessment_entry_id END
    ) STORED,
    scoring_rule ple_data.scoring_rule NOT NULL,
    question_attempt_limit integer,
    question_attempt_time_limit_seconds integer,
    question_attempt_grace_seconds integer,
    -- ASVS 2.3.3: validate current positions after the atomic complete-content
    -- save, allowing swaps while retired Entries retain their historical positions.
    CONSTRAINT assessment_entry_active_authored_position_key
        UNIQUE (assessment_id, active_authored_position) DEFERRABLE INITIALLY DEFERRED,
    UNIQUE (assessment_entry_id, assessment_id),
    UNIQUE (assessment_entry_question_id),
    UNIQUE (assessment_entry_pool_id),
    CHECK (question_attempt_limit IS NULL OR question_attempt_limit > 0),
    CHECK (
        (question_attempt_time_limit_seconds IS NULL AND question_attempt_grace_seconds IS NULL)
        OR (question_attempt_time_limit_seconds > 0
            AND question_attempt_grace_seconds >= 0)
    ),
    created_at timestamptz NOT NULL DEFAULT pg_catalog.transaction_timestamp(),
    updated_at timestamptz NOT NULL DEFAULT pg_catalog.transaction_timestamp(),
    CHECK (updated_at >= created_at)
);

CREATE TABLE ple_data.assessment_entry_question (
    assessment_entry_id uuid PRIMARY KEY
        REFERENCES ple_data.assessment_entry(assessment_entry_question_id),
    assessment_id ple_data.assessment_id NOT NULL,
    published_question_id ple_data.question_family_id NOT NULL,
    question_revision_number integer NOT NULL,
    points_possible numeric NOT NULL CHECK (
        points_possible BETWEEN 0 AND 1000000000.9999 AND scale(points_possible) <= 4
    ),
    UNIQUE (assessment_entry_id, assessment_id),
    FOREIGN KEY (assessment_entry_id, assessment_id)
        REFERENCES ple_data.assessment_entry(assessment_entry_id, assessment_id),
    FOREIGN KEY (published_question_id, question_revision_number)
        REFERENCES ple_data.question_revision(published_question_id, revision_number),
    created_at timestamptz NOT NULL DEFAULT pg_catalog.transaction_timestamp(),
    updated_at timestamptz NOT NULL DEFAULT pg_catalog.transaction_timestamp(),
    CHECK (updated_at >= created_at)
);

CREATE TABLE ple_data.assessment_entry_pool (
    assessment_entry_id uuid PRIMARY KEY
        REFERENCES ple_data.assessment_entry(assessment_entry_pool_id),
    assessment_id ple_data.assessment_id NOT NULL,
    question_pool_id ple_data.question_family_id NOT NULL
        REFERENCES ple_data.question_pool(question_pool_id),
    selection_count integer NOT NULL CHECK (selection_count > 0),
    points_per_item numeric NOT NULL CHECK (
        points_per_item BETWEEN 0 AND 1000000000.9999 AND scale(points_per_item) <= 4
    ),
    selected_question_order ple_data.selected_question_order NOT NULL,
    UNIQUE (assessment_entry_id, assessment_id),
    UNIQUE (assessment_entry_id, assessment_id, question_pool_id),
    FOREIGN KEY (assessment_entry_id, assessment_id)
        REFERENCES ple_data.assessment_entry(assessment_entry_id, assessment_id),
    created_at timestamptz NOT NULL DEFAULT pg_catalog.transaction_timestamp(),
    updated_at timestamptz NOT NULL DEFAULT pg_catalog.transaction_timestamp(),
    CHECK (updated_at >= created_at)
);

ALTER TABLE ple_data.assessment_entry
    ADD CONSTRAINT assessment_entry_has_question_child
    FOREIGN KEY (assessment_entry_question_id)
        REFERENCES ple_data.assessment_entry_question(assessment_entry_id)
        DEFERRABLE INITIALLY DEFERRED,
    ADD CONSTRAINT assessment_entry_has_pool_child
    FOREIGN KEY (assessment_entry_pool_id)
        REFERENCES ple_data.assessment_entry_pool(assessment_entry_id)
        DEFERRABLE INITIALLY DEFERRED;





-- A Pool imported into an Assessment is owned by exactly one Assessment Entry.
-- This cyclic, deferred association deliberately makes it impossible for the
-- ordinary current-content save to attach a published Pool, another fork, or
-- an arbitrary browser-provided lineage.  The trusted import boundary creates
-- the entry, copies current source members once, and this association atomically.
CREATE TABLE ple_data.assessment_question_pool_fork (
    assessment_entry_id uuid NOT NULL,
    assessment_id ple_data.assessment_id NOT NULL,
    question_pool_id ple_data.question_family_id NOT NULL UNIQUE
        REFERENCES ple_data.question_pool(question_pool_id),
    PRIMARY KEY (assessment_entry_id),
    UNIQUE (assessment_entry_id, assessment_id, question_pool_id),
    FOREIGN KEY (assessment_entry_id, assessment_id)
        REFERENCES ple_data.assessment_entry(assessment_entry_id, assessment_id)
        DEFERRABLE INITIALLY DEFERRED,
    created_at timestamptz NOT NULL DEFAULT pg_catalog.transaction_timestamp(),
    updated_at timestamptz NOT NULL DEFAULT pg_catalog.transaction_timestamp(),
    CHECK (updated_at >= created_at)
);


SET LOCAL ROLE ple_private_owner;

-- Private durable state for external Question-delivery backends.  The backend
-- keeps opaque state; an Issued Question and its Question Attempt remain the
-- authoritative reproducibility and Student Work evidence.
CREATE TABLE ple_private.imathas_render_cache_entry (
    imathas_render_cache_entry_id uuid PRIMARY KEY,
    imathas_deployment_reference text NOT NULL CHECK (imathas_deployment_reference ~ '^[A-Za-z0-9._-]{1,160}$'),
    published_question_id ple_data.question_family_id NOT NULL,
    revision_number integer NOT NULL,
    imathas_normalized_question_seed integer NOT NULL CHECK (imathas_normalized_question_seed BETWEEN 1 AND 9999),
    imathas_profile text NOT NULL CHECK (imathas_profile ~ '^[A-Za-z0-9._-]{1,160}$'),
    source_payload_digest bytea NOT NULL CHECK (octet_length(source_payload_digest) = 32),
    encrypted_render_data bytea NOT NULL CHECK (octet_length(encrypted_render_data) BETWEEN 1 AND 1048576),
    fetched_at timestamptz NOT NULL,
    expires_at timestamptz NOT NULL CHECK (expires_at > fetched_at),
    FOREIGN KEY (published_question_id, revision_number) REFERENCES ple_data.question_revision(published_question_id, revision_number),
    UNIQUE (imathas_deployment_reference, published_question_id, revision_number, imathas_normalized_question_seed, imathas_profile, source_payload_digest),
    updated_at timestamptz NOT NULL DEFAULT pg_catalog.transaction_timestamp()
);

SET LOCAL ROLE ple_data_owner;
COMMENT ON TABLE ple_data.assessment_policy_snapshot IS 'role: snapshot, deleted by nothing (shared, immutable). HUMAN_GUIDANCE.md Assessment Attempt snapshots.';
COMMENT ON TABLE ple_data.assessment IS 'role: current state, Current Course Assessment aggregate with qualified Edit Number; released saves govern future Assessment Attempts.';
COMMENT ON TABLE ple_data.assessment_entry IS 'role: current state, Stable current Assessment Entry identity, kind, scoring, availability, and per-question attempt limits.';
COMMENT ON TABLE ple_data.assessment_entry_question IS 'role: current state, Fixed-question Assessment Entry pin: exact Published Question Revision and points possible.';
COMMENT ON TABLE ple_data.assessment_entry_pool IS 'role: current state, Question Pool Assessment Entry policy: Pool ID pin, selection count, points per item, and selected-question order.';
COMMENT ON TABLE ple_data.assessment_question_pool_fork IS 'role: current state, One Assessment Entry-owned fork Pool lineage; import copies current source members once.';
COMMENT ON COLUMN ple_data.assessment_policy_snapshot.available_at IS 'NULL means this optional fact is absent.';
COMMENT ON COLUMN ple_data.assessment_policy_snapshot.due_at IS 'NULL means this optional fact is absent.';
COMMENT ON COLUMN ple_data.assessment_policy_snapshot.closes_at IS 'NULL means this optional fact is absent.';
COMMENT ON COLUMN ple_data.assessment_policy_snapshot.assessment_attempt_time_limit_seconds IS 'NULL means this optional fact is absent.';
COMMENT ON COLUMN ple_data.assessment_policy_snapshot.assessment_attempt_limit IS 'NULL means this optional fact is absent.';
COMMENT ON COLUMN ple_data.assessment.source_blueprint_course_id IS 'NULL means this optional fact is absent.';
COMMENT ON COLUMN ple_data.assessment.source_blueprint_revision_number IS 'NULL means this optional fact is absent.';
COMMENT ON COLUMN ple_data.assessment.source_blueprint_assessment_reference IS 'NULL means this optional fact is absent.';
COMMENT ON COLUMN ple_data.assessment_entry.active_authored_position IS 'NULL means this optional fact is absent.';
COMMENT ON COLUMN ple_data.assessment_entry.assessment_entry_question_id IS 'NULL means this Entry is a Question Pool child, not a fixed Question.';
COMMENT ON COLUMN ple_data.assessment_entry.assessment_entry_pool_id IS 'NULL means this Entry is a fixed Question child, not a Question Pool.';
COMMENT ON COLUMN ple_data.assessment_entry.question_attempt_limit IS 'NULL means this optional fact is absent.';
COMMENT ON COLUMN ple_data.assessment_entry.question_attempt_time_limit_seconds IS 'NULL means this optional fact is absent.';
COMMENT ON COLUMN ple_data.assessment_entry.question_attempt_grace_seconds IS 'NULL means this optional fact is absent.';

SET LOCAL ROLE ple_private_owner;
COMMENT ON TABLE ple_private.imathas_render_cache_entry IS 'role: current state, deleted by Unrelease of Student Work; Assessment rows remain with the Course. HUMAN_GUIDANCE.md Assessments.';

