-- assessment tables. CREATE TABLE and COMMENT ON only.
-- Functions, policies, and privileges live in later layers.

SET LOCAL ROLE ple_data_owner;

-- Current Course Assessment aggregates.  These relations describe the content
-- used for future Assessment Attempts; student_work.sql retains the facts used to
-- interpret an Assessment Attempt after a later Assessment edit.
CREATE TABLE ple_data.assessment (
    assessment_id uuid PRIMARY KEY,
    course_id uuid NOT NULL REFERENCES ple_data.course_instance(course_id),
    reference_number bigint GENERATED ALWAYS AS IDENTITY UNIQUE NOT NULL
        CHECK (reference_number BETWEEN 1 AND 2147483647),
    public_reference text NOT NULL UNIQUE CHECK (
        ple_private.is_canonical_prefixed_public_id(public_reference, 'A')
    ),
    origin_kind text NOT NULL CHECK (origin_kind IN ('direct', 'adopted')),
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
    assessment_type text NOT NULL CHECK (assessment_type IN (
        'regular_assignment', 'practice_question_assignment', 'bonus_assignment', 'quiz', 'exam'
    )),
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
    late_work_rule text NOT NULL CHECK (late_work_rule IN ('accept', 'mark_late', 'reject')),
    question_variation_rule text NOT NULL CHECK (
        question_variation_rule IN ('reuse_variation', 'new_variation')
    ),
    assessment_question_order_rule text NOT NULL CHECK (
        assessment_question_order_rule IN ('authored_order', 'shuffled')
    ),
    feedback_score text NOT NULL CHECK (
        feedback_score IN ('during_attempt', 'after_submit', 'after_due', 'after_close', 'never')
    ),
    feedback_per_item_correctness text NOT NULL CHECK (
        feedback_per_item_correctness IN ('during_attempt', 'after_submit', 'after_due', 'after_close', 'never')
    ),
    feedback_submitted_response text NOT NULL CHECK (
        feedback_submitted_response IN ('during_attempt', 'after_submit', 'after_due', 'after_close', 'never')
    ),
    feedback_question_answer text NOT NULL CHECK (
        feedback_question_answer IN ('during_attempt', 'after_submit', 'after_due', 'after_close', 'never')
    ),
    feedback_question_answer_explanation text NOT NULL CHECK (
        feedback_question_answer_explanation IN ('during_attempt', 'after_submit', 'after_due', 'after_close', 'never')
    ),
    feedback_class_statistics text NOT NULL CHECK (
        feedback_class_statistics IN ('during_attempt', 'after_submit', 'after_due', 'after_close', 'never')
    ),
    assessment_status text NOT NULL DEFAULT 'unreleased' CHECK (
        assessment_status IN ('unreleased', 'released', 'closed', 'archived')
    ),
    UNIQUE (course_id, reference_number),
    UNIQUE (assessment_id, course_id),
    FOREIGN KEY (
        source_blueprint_course_reference_number,
        source_blueprint_revision_number,
        source_blueprint_assessment_reference
    ) REFERENCES ple_data.blueprint_revision_assessment (
        blueprint_course_reference_number,
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
    entry_kind text NOT NULL CHECK (entry_kind IN ('fixed_question', 'question_pool')),
    availability text NOT NULL DEFAULT 'available' CHECK (availability IN ('available', 'retired')),
    active_authored_position integer GENERATED ALWAYS AS (
        CASE WHEN availability = 'available' THEN authored_position END
    ) STORED,
    scoring_rule text NOT NULL CHECK (scoring_rule IN ('normal', 'full_credit', 'extra_credit', 'excluded')),
    question_id text,
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
    FOREIGN KEY (question_id, question_revision_number)
        REFERENCES ple_data.question_revision(question_id, revision_number),
    FOREIGN KEY (question_pool_id, question_pool_revision_number)
        REFERENCES ple_data.question_pool_revision(question_pool_id, revision_number),
    CHECK (
        (entry_kind = 'fixed_question'
            AND question_id IS NOT NULL
            AND question_revision_number IS NOT NULL
            AND question_pool_id IS NULL
            AND question_pool_revision_number IS NULL
            AND points_possible IS NOT NULL
            AND points_possible >= 0
            AND selection_count IS NULL
            AND points_per_item IS NULL
            AND selected_question_order IS NULL)
        OR (entry_kind = 'question_pool'
            AND question_id IS NULL
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
    )
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
        REFERENCES ple_data.question_pool_revision(question_pool_id, revision_number)
);

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
    question_id text NOT NULL,
    revision_number integer NOT NULL,
    imathas_normalized_question_seed integer NOT NULL CHECK (imathas_normalized_question_seed BETWEEN 1 AND 9999),
    imathas_profile text NOT NULL CHECK (imathas_profile ~ '^[A-Za-z0-9._-]{1,160}$'),
    source_payload_digest bytea NOT NULL CHECK (octet_length(source_payload_digest) = 32),
    encrypted_render_data bytea NOT NULL CHECK (octet_length(encrypted_render_data) BETWEEN 1 AND 1048576),
    fetched_at timestamptz NOT NULL,
    expires_at timestamptz NOT NULL CHECK (expires_at > fetched_at),
    FOREIGN KEY (question_id, revision_number) REFERENCES ple_data.question_revision(question_id, revision_number),
    UNIQUE (imathas_deployment_reference, question_id, revision_number, imathas_normalized_question_seed, imathas_profile, source_payload_digest)
);

CREATE TABLE ple_private.imathas_question_backend_session (
    imathas_question_backend_session_id uuid PRIMARY KEY,
    course_id uuid NOT NULL,
    assessment_id uuid NOT NULL,
    question_attempt_id uuid NOT NULL UNIQUE REFERENCES ple_private.question_attempt(question_attempt_id) ON DELETE CASCADE,
    account_id uuid NOT NULL REFERENCES ple_private.account(account_id),
    imathas_deployment_reference text NOT NULL CHECK (imathas_deployment_reference ~ '^[A-Za-z0-9._-]{1,160}$'),
    imathas_item_reference text NOT NULL CHECK (octet_length(imathas_item_reference) BETWEEN 1 AND 128 AND imathas_item_reference ~ '^[A-Za-z0-9._-]+$'),
    question_id text NOT NULL,
    revision_number integer NOT NULL,
    source_object_id uuid NOT NULL,
    source_object_checksum bytea NOT NULL CHECK (octet_length(source_object_checksum) = 32),
    imathas_profile text NOT NULL CHECK (imathas_profile ~ '^[A-Za-z0-9._-]{1,160}$'),
    question_seed numeric(20, 0) NOT NULL CHECK (question_seed BETWEEN 0 AND 18446744073709551615),
    imathas_launch_binding_checksum text NOT NULL CHECK (imathas_launch_binding_checksum ~ '^[0-9a-f]{64}$'),
    imathas_response_sha256 bytea NOT NULL CHECK (octet_length(imathas_response_sha256) = 32),
    imathas_question_backend_session_challenge bytea NOT NULL CHECK (octet_length(imathas_question_backend_session_challenge) = 32 AND imathas_question_backend_session_challenge <> decode(repeat('00', 32), 'hex')),
    imathas_question_backend_session_authentication bytea NOT NULL CHECK (octet_length(imathas_question_backend_session_authentication) BETWEEN 3 AND 512 AND convert_from(imathas_question_backend_session_authentication, 'UTF8') ~ '^([0-9a-f]{2})+[.][0-9a-f]{64}$'),
    issued_at timestamptz NOT NULL,
    expires_at timestamptz NOT NULL CHECK (expires_at > issued_at),
    revoked_at timestamptz,
    consumed_at timestamptz,
    activity_lease_token_sha256 bytea CHECK (activity_lease_token_sha256 IS NULL OR octet_length(activity_lease_token_sha256) = 32),
    activity_lease_expires_at timestamptz,
    imathas_question_backend_state_key_id text NOT NULL CHECK (imathas_question_backend_state_key_id ~ '^[A-Za-z0-9._:-]{1,160}$'),
    imathas_question_backend_state_nonce bytea NOT NULL CHECK (octet_length(imathas_question_backend_state_nonce) = 24),
    imathas_question_backend_state_ciphertext bytea NOT NULL CHECK (octet_length(imathas_question_backend_state_ciphertext) BETWEEN 17 AND 65536),
    FOREIGN KEY (course_id, assessment_id) REFERENCES ple_data.assessment(course_id, assessment_id),
    FOREIGN KEY (question_id, revision_number) REFERENCES ple_data.question_revision(question_id, revision_number),
    CHECK (revoked_at IS NULL OR revoked_at >= issued_at),
    CHECK (consumed_at IS NULL OR consumed_at >= issued_at),
    CHECK ((activity_lease_token_sha256 IS NULL) = (activity_lease_expires_at IS NULL)),
    CHECK (activity_lease_expires_at IS NULL OR activity_lease_expires_at > issued_at AND activity_lease_expires_at <= expires_at),
    CHECK (revoked_at IS NULL OR consumed_at IS NULL),
    UNIQUE (imathas_question_backend_state_key_id, imathas_question_backend_state_nonce)
);

COMMENT ON TABLE ple_private.imathas_render_cache_entry IS 'role: current state, deleted by Unrelease of Student Work; Assessment rows remain with the Course. HUMAN_GUIDANCE.md Assessments.';

COMMENT ON TABLE ple_private.imathas_question_backend_session IS 'role: event, deleted by Unrelease of Student Work; Assessment rows remain with the Course. HUMAN_GUIDANCE.md Assessments.';

