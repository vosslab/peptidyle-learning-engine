-- assessment tables. CREATE TABLE and COMMENT ON only.
-- Functions, policies, and privileges live in later layers.

SET LOCAL ROLE ple_data_owner;

-- Current Course Assessment aggregates.  These relations describe the content
-- used for future Assessment Attempts; student_work.sql retains the facts used to
-- interpret an Assessment Attempt after a later Assessment edit.
CREATE TABLE ple_data.assessment (
    assessment_id uuid PRIMARY KEY,
    course_instance_id uuid NOT NULL REFERENCES ple_data.course_instance(course_instance_id),
    reference_number bigint GENERATED ALWAYS AS IDENTITY UNIQUE NOT NULL
        CHECK (reference_number BETWEEN 1 AND 2147483647),
    public_reference text NOT NULL UNIQUE CHECK (
        ple_private.is_canonical_prefixed_public_id(public_reference, 'A')
    ),
    origin_kind ple_data.assessment_origin_kind NOT NULL,
    source_blueprint_course_reference_number bigint,
    source_blueprint_revision_number bigint CHECK (source_blueprint_revision_number > 0),
    -- BlueprintAssessmentSource: an exact immutable Blueprint Revision plus
    -- the stable Assessment member selected from that Revision.
    source_blueprint_assessment_reference uuid,
    created_at timestamptz NOT NULL,
    updated_at timestamptz NOT NULL,
    assessment_edit_number bigint NOT NULL DEFAULT 1 CHECK (assessment_edit_number > 0),
    assessment_title text NOT NULL CHECK (
        assessment_title ~ '[^[:space:]]' AND char_length(assessment_title) <= 200
    ),
    assessment_type ple_data.assessment_type NOT NULL,
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
    assessment_attempt_limit integer CHECK (assessment_attempt_limit IS NULL OR assessment_attempt_limit > 0),
    late_work_rule ple_data.late_work_rule NOT NULL,
    question_variation_rule ple_data.question_variation_rule NOT NULL,
    assessment_question_order_rule ple_data.question_order_rule NOT NULL,
    feedback_score ple_data.feedback_release NOT NULL,
    feedback_per_item_correctness ple_data.feedback_release NOT NULL,
    feedback_submitted_response ple_data.feedback_release NOT NULL,
    feedback_question_answer ple_data.feedback_release NOT NULL,
    feedback_question_answer_explanation ple_data.feedback_release NOT NULL,
    feedback_class_statistics ple_data.feedback_release NOT NULL,
    assessment_status ple_data.assessment_status NOT NULL DEFAULT 'unreleased',
    UNIQUE (course_instance_id, reference_number),
    UNIQUE (assessment_id, course_instance_id),
    FOREIGN KEY (
        source_blueprint_course_reference_number,
        source_blueprint_revision_number,
        source_blueprint_assessment_reference
    ) REFERENCES ple_data.blueprint_revision_assessment (
        blueprint_course_id,
        blueprint_revision_number,
        blueprint_assessment_reference
    ),
    CHECK (
        (origin_kind = 'direct'
            AND source_blueprint_course_reference_number IS NULL
            AND source_blueprint_revision_number IS NULL
            AND source_blueprint_assessment_reference IS NULL)
        OR (origin_kind = 'adopted'
            AND source_blueprint_course_reference_number IS NOT NULL
            AND source_blueprint_revision_number IS NOT NULL
            AND source_blueprint_assessment_reference IS NOT NULL)
    ),
    CHECK (updated_at >= created_at),
    -- ASVS 2.2.1-2.2.3: mandatory variant fields cannot pass CHECK as unknown.
    CHECK (
        assessment_type NOT IN ('quiz', 'exam')
        OR (assessment_attempt_limit IS NOT NULL AND assessment_attempt_limit = 1)
    )
);



CREATE TABLE ple_data.assessment_entry (
    assessment_entry_id uuid PRIMARY KEY,
    assessment_id uuid NOT NULL REFERENCES ple_data.assessment(assessment_id),
    authored_position integer NOT NULL CHECK (authored_position >= 0),
    entry_kind ple_data.entry_kind NOT NULL,
    availability ple_data.entry_availability NOT NULL DEFAULT 'available',
    active_authored_position integer GENERATED ALWAYS AS (
        CASE WHEN availability = 'available' THEN authored_position END
    ) STORED,
    scoring_rule ple_data.scoring_rule NOT NULL,
    published_question_id text,
    question_revision_number integer,
    question_pool_id uuid,
    question_pool_revision_number bigint,
    points_possible numeric CHECK (points_possible BETWEEN 0 AND 1000000000.9999 AND scale(points_possible) <= 4),
    selection_count integer,
    points_per_item numeric CHECK (points_per_item BETWEEN 0 AND 1000000000.9999 AND scale(points_per_item) <= 4),
    selected_question_order text,
    question_attempt_limit integer,
    question_attempt_time_limit_seconds integer,
    question_attempt_grace_seconds integer,
    -- ASVS 2.3.3: validate current positions after the atomic complete-content
    -- save, allowing swaps while retired Entries retain their historical positions.
    CONSTRAINT assessment_entry_active_authored_position_key
        UNIQUE (assessment_id, active_authored_position) DEFERRABLE INITIALLY DEFERRED,
    UNIQUE (assessment_entry_id, assessment_id),
    FOREIGN KEY (published_question_id, question_revision_number)
        REFERENCES ple_data.question_revision(published_question_id, revision_number),
    FOREIGN KEY (question_pool_id, question_pool_revision_number)
        REFERENCES ple_data.question_pool_revision(question_pool_id, revision_number),
    CHECK (
        (entry_kind = 'fixed_question'
            AND published_question_id IS NOT NULL
            AND question_revision_number IS NOT NULL
            AND question_pool_id IS NULL
            AND question_pool_revision_number IS NULL
            AND points_possible IS NOT NULL
            AND points_possible >= 0
            AND selection_count IS NULL
            AND points_per_item IS NULL
            AND selected_question_order IS NULL)
        OR (entry_kind = 'question_pool'
            AND published_question_id IS NULL
            AND question_revision_number IS NULL
            AND question_pool_id IS NOT NULL
            AND question_pool_revision_number IS NOT NULL
            AND points_possible IS NULL
            AND selection_count IS NOT NULL
            AND selection_count > 0
            AND points_per_item IS NOT NULL
            AND points_per_item >= 0
            AND selected_question_order IS NOT NULL
            AND selected_question_order IN ('question_pool_order', 'random_order'))
    ),
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





-- A Pool imported into an Assessment is owned by exactly one Assessment Entry.
-- This cyclic, deferred association deliberately makes it impossible for the
-- ordinary current-content save to attach a published Pool, another fork, or
-- an arbitrary browser-provided lineage.  The trusted import boundary creates
-- the entry, fork revision 1, and this association atomically.
CREATE TABLE ple_data.assessment_question_pool_fork (
    assessment_entry_id uuid NOT NULL,
    assessment_id uuid NOT NULL,
    question_pool_id uuid NOT NULL UNIQUE,
    origin_question_pool_revision_number bigint NOT NULL CHECK (origin_question_pool_revision_number = 1),
    PRIMARY KEY (assessment_entry_id),
    UNIQUE (assessment_entry_id, assessment_id, question_pool_id),
    FOREIGN KEY (assessment_entry_id, assessment_id)
        REFERENCES ple_data.assessment_entry(assessment_entry_id, assessment_id)
        DEFERRABLE INITIALLY DEFERRED,
    FOREIGN KEY (question_pool_id, origin_question_pool_revision_number)
        REFERENCES ple_data.question_pool_revision(question_pool_id, revision_number),
    created_at timestamptz NOT NULL DEFAULT pg_catalog.transaction_timestamp(),
    updated_at timestamptz NOT NULL DEFAULT pg_catalog.transaction_timestamp(),
    CHECK (updated_at >= created_at)
);


SET LOCAL ROLE ple_data_owner;
COMMENT ON TABLE ple_data.assessment IS 'role: current state, Current Course Assessment aggregate with qualified Edit Number; released saves govern future Assessment Attempts.';

COMMENT ON TABLE ple_data.assessment_entry IS 'role: current state, Stable current Assessment Entry identity and exact fixed Question Revision pin or Question Pool policy.';

COMMENT ON TABLE ple_data.assessment_question_pool_fork IS 'role: current state, One Assessment Entry-owned fork Pool lineage; origin revision 1 is retained while the Entry pins later immutable fork revisions.';

SET LOCAL ROLE ple_private_owner;

-- Private durable state for external Question-delivery backends.  The backend
-- keeps opaque state; an Issued Question and its Question Attempt remain the
-- authoritative reproducibility and Student Work evidence.
CREATE TABLE ple_private.imathas_render_cache_entry (
    imathas_render_cache_entry_id uuid PRIMARY KEY,
    imathas_deployment_reference text NOT NULL CHECK (imathas_deployment_reference ~ '^[A-Za-z0-9._-]{1,160}$'),
    published_question_id text NOT NULL,
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


SET LOCAL ROLE ple_private_owner;
COMMENT ON TABLE ple_private.imathas_render_cache_entry IS 'role: current state, deleted by Unrelease of Student Work; Assessment rows remain with the Course. HUMAN_GUIDANCE.md Assessments.';

SET LOCAL ROLE ple_data_owner;
COMMENT ON TABLE ple_data.assessment IS 'role: current state, Current Course Assessment aggregate with qualified Edit Number; released saves govern future Assessment Attempts.';

COMMENT ON TABLE ple_data.assessment_entry IS 'role: current state, Stable current Assessment Entry identity and exact fixed Question Revision pin or Question Pool policy.';

COMMENT ON TABLE ple_data.assessment_question_pool_fork IS 'role: current state, One Assessment Entry-owned fork Pool lineage; origin revision 1 is retained while the Entry pins later immutable fork revisions.';

SET LOCAL ROLE ple_private_owner;

SET LOCAL ROLE ple_private_owner;
COMMENT ON TABLE ple_private.imathas_render_cache_entry IS 'role: current state, deleted by Unrelease of Student Work; Assessment rows remain with the Course. HUMAN_GUIDANCE.md Assessments.';

SET LOCAL ROLE ple_data_owner;
COMMENT ON TABLE ple_data.assessment IS 'role: current state, Current Course Assessment aggregate with qualified Edit Number; released saves govern future Assessment Attempts.';

COMMENT ON TABLE ple_data.assessment_entry IS 'role: current state, Stable current Assessment Entry identity and exact fixed Question Revision pin or Question Pool policy.';

COMMENT ON TABLE ple_data.assessment_question_pool_fork IS 'role: current state, One Assessment Entry-owned fork Pool lineage; origin revision 1 is retained while the Entry pins later immutable fork revisions.';

SET LOCAL ROLE ple_private_owner;

SET LOCAL ROLE ple_private_owner;
COMMENT ON TABLE ple_private.imathas_render_cache_entry IS 'role: current state, deleted by Unrelease of Student Work; Assessment rows remain with the Course. HUMAN_GUIDANCE.md Assessments.';



COMMENT ON TABLE ple_private.imathas_render_cache_entry IS 'role: current state, deleted by Unrelease of Student Work; Assessment rows remain with the Course. HUMAN_GUIDANCE.md Assessments.';



COMMENT ON TABLE ple_private.imathas_render_cache_entry IS 'role: current state, deleted by Unrelease of Student Work; Assessment rows remain with the Course. HUMAN_GUIDANCE.md Assessments.';

SET LOCAL ROLE ple_data_owner;
COMMENT ON TABLE ple_data.assessment IS 'role: current state, Current Course Assessment aggregate with qualified Edit Number; released saves govern future Assessment Attempts.';

COMMENT ON TABLE ple_data.assessment_entry IS 'role: current state, Stable current Assessment Entry identity and exact fixed Question Revision pin or Question Pool policy.';

COMMENT ON TABLE ple_data.assessment_question_pool_fork IS 'role: current state, One Assessment Entry-owned fork Pool lineage; origin revision 1 is retained while the Entry pins later immutable fork revisions.';

SET LOCAL ROLE ple_private_owner;

SET LOCAL ROLE ple_private_owner;
COMMENT ON TABLE ple_private.imathas_render_cache_entry IS 'role: current state, deleted by Unrelease of Student Work; Assessment rows remain with the Course. HUMAN_GUIDANCE.md Assessments.';

SET LOCAL ROLE ple_data_owner;
COMMENT ON TABLE ple_data.assessment IS 'role: current state, Current Course Assessment aggregate with qualified Edit Number; released saves govern future Assessment Attempts.';

COMMENT ON TABLE ple_data.assessment_entry IS 'role: current state, Stable current Assessment Entry identity and exact fixed Question Revision pin or Question Pool policy.';

COMMENT ON TABLE ple_data.assessment_question_pool_fork IS 'role: current state, One Assessment Entry-owned fork Pool lineage; origin revision 1 is retained while the Entry pins later immutable fork revisions.';

SET LOCAL ROLE ple_private_owner;

SET LOCAL ROLE ple_private_owner;
COMMENT ON TABLE ple_private.imathas_render_cache_entry IS 'role: current state, deleted by Unrelease of Student Work; Assessment rows remain with the Course. HUMAN_GUIDANCE.md Assessments.';



COMMENT ON TABLE ple_private.imathas_render_cache_entry IS 'role: current state, deleted by Unrelease of Student Work; Assessment rows remain with the Course. HUMAN_GUIDANCE.md Assessments.';

