-- assessment_attempt tables. CREATE TABLE and COMMENT ON only.
-- Functions, policies, and privileges live in later layers.

SET LOCAL ROLE ple_private_owner;

CREATE TABLE ple_private.student_assessment_accommodation (
    accommodation_id uuid PRIMARY KEY,
    student_record_id uuid NOT NULL REFERENCES ple_data.student_record(student_record_id),
    assessment_id uuid NOT NULL REFERENCES ple_data.assessment(assessment_id),
    available_at timestamptz,
    due_at timestamptz,
    closes_at timestamptz,
    time_multiplier numeric,
    assessment_attempt_limit integer,
    created_at timestamptz NOT NULL,
    accommodation_edit_number bigint NOT NULL DEFAULT 1
        CHECK (accommodation_edit_number > 0),
    CHECK ((available_at IS NULL OR due_at IS NULL OR available_at <= due_at)
       AND (due_at IS NULL OR closes_at IS NULL OR due_at <= closes_at)),
    CHECK (time_multiplier IS NULL
       OR (time_multiplier >= 1 AND time_multiplier < 'Infinity'::numeric)),
    CHECK (assessment_attempt_limit IS NULL OR assessment_attempt_limit > 0),
    UNIQUE (accommodation_id, student_record_id, assessment_id),
    UNIQUE (student_record_id, assessment_id)
);

CREATE TABLE ple_private.assessment_attempt (
    assessment_attempt_id uuid PRIMARY KEY,
    reference_number bigint GENERATED ALWAYS AS IDENTITY UNIQUE
        CHECK (reference_number BETWEEN 1 AND 2147483647),
    student_record_id uuid NOT NULL REFERENCES ple_data.student_record(student_record_id),
    assessment_id uuid NOT NULL REFERENCES ple_data.assessment(assessment_id),
    assessment_attempt_number integer NOT NULL CHECK (assessment_attempt_number > 0),
    started_at timestamptz NOT NULL,
    expires_at timestamptz,
    assessment_title text NOT NULL CHECK (assessment_title ~ '[^[:space:]]'),
    assessment_instructions text NOT NULL CHECK (assessment_instructions !~ E'\\x00'),
    available_at timestamptz,
    due_at timestamptz,
    closes_at timestamptz,
    assessment_attempt_time_limit_seconds integer,
    assessment_attempt_limit integer,
    late_work_rule text NOT NULL CHECK (late_work_rule IN ('accept', 'mark_late', 'reject')),
    question_variation_rule text NOT NULL CHECK (question_variation_rule IN ('reuse_variation', 'new_variation')),
    assessment_question_order_rule text NOT NULL CHECK (assessment_question_order_rule IN ('authored_order', 'shuffled')),
    feedback_score text NOT NULL CHECK (feedback_score IN ('during_attempt', 'after_submit', 'after_due', 'after_close', 'never')),
    feedback_per_item_correctness text NOT NULL CHECK (feedback_per_item_correctness IN ('during_attempt', 'after_submit', 'after_due', 'after_close', 'never')),
    feedback_submitted_response text NOT NULL CHECK (feedback_submitted_response IN ('during_attempt', 'after_submit', 'after_due', 'after_close', 'never')),
    feedback_question_answer text NOT NULL CHECK (feedback_question_answer IN ('during_attempt', 'after_submit', 'after_due', 'after_close', 'never')),
    feedback_question_answer_explanation text NOT NULL CHECK (feedback_question_answer_explanation IN ('during_attempt', 'after_submit', 'after_due', 'after_close', 'never')),
    feedback_class_statistics text NOT NULL CHECK (feedback_class_statistics IN ('during_attempt', 'after_submit', 'after_due', 'after_close', 'never')),
    schedule_accommodation_id uuid,
    schedule_accommodation_edit_number bigint CHECK (schedule_accommodation_edit_number > 0),
    time_limit_accommodation_id uuid,
    time_limit_accommodation_edit_number bigint CHECK (time_limit_accommodation_edit_number > 0),
    assessment_attempt_limit_accommodation_id uuid,
    assessment_attempt_limit_accommodation_edit_number bigint CHECK (assessment_attempt_limit_accommodation_edit_number > 0),
    UNIQUE (student_record_id, assessment_id, assessment_attempt_number),
    CHECK (expires_at IS NULL OR expires_at >= started_at),
    CHECK ((available_at IS NULL OR due_at IS NULL OR available_at <= due_at)
       AND (due_at IS NULL OR closes_at IS NULL OR due_at <= closes_at)),
    CHECK (assessment_attempt_time_limit_seconds IS NULL OR assessment_attempt_time_limit_seconds > 0),
    CHECK (assessment_attempt_limit IS NULL OR assessment_attempt_limit > 0),
    FOREIGN KEY (schedule_accommodation_id, student_record_id, assessment_id)
        REFERENCES ple_private.student_assessment_accommodation(accommodation_id, student_record_id, assessment_id),
    FOREIGN KEY (time_limit_accommodation_id, student_record_id, assessment_id)
        REFERENCES ple_private.student_assessment_accommodation(accommodation_id, student_record_id, assessment_id),
    FOREIGN KEY (assessment_attempt_limit_accommodation_id, student_record_id, assessment_id)
        REFERENCES ple_private.student_assessment_accommodation(accommodation_id, student_record_id, assessment_id),
    CHECK ((schedule_accommodation_id IS NULL) = (schedule_accommodation_edit_number IS NULL)),
    CHECK ((time_limit_accommodation_id IS NULL) = (time_limit_accommodation_edit_number IS NULL)),
    CHECK ((assessment_attempt_limit_accommodation_id IS NULL) = (assessment_attempt_limit_accommodation_edit_number IS NULL))
);

CREATE TABLE ple_private.question_pool_selection (
    question_pool_selection_id uuid PRIMARY KEY,
    assessment_attempt_id uuid NOT NULL REFERENCES ple_private.assessment_attempt(assessment_attempt_id) ON DELETE CASCADE,
    -- This is the stable authored identity copied at issue time, not a foreign
    -- key to the mutable current Assessment Entry.  Released Assessment saves
    -- may replace current entries while this Student Work remains interpretable.
    assessment_entry_id uuid NOT NULL,
    question_pool_id uuid NOT NULL,
    question_pool_revision_number bigint NOT NULL,
    created_at timestamptz NOT NULL,
    selected_question_count integer NOT NULL CHECK (selected_question_count > 0),
    UNIQUE (question_pool_selection_id, assessment_attempt_id, assessment_entry_id),
    UNIQUE (assessment_attempt_id, assessment_entry_id),
    FOREIGN KEY (question_pool_id, question_pool_revision_number)
        REFERENCES ple_data.question_pool_revision(question_pool_id, revision_number)
);

CREATE TABLE ple_private.question_pool_selected_item (
    question_pool_selection_id uuid NOT NULL REFERENCES ple_private.question_pool_selection(question_pool_selection_id) ON DELETE CASCADE,
    member_position integer NOT NULL CHECK (member_position > 0),
    selection_position integer NOT NULL CHECK (selection_position >= 0),
    question_id text NOT NULL,
    revision_number integer NOT NULL,
    PRIMARY KEY (question_pool_selection_id, selection_position),
    UNIQUE (question_pool_selection_id, member_position),
    UNIQUE (question_pool_selection_id, member_position, question_id, revision_number),
    FOREIGN KEY (question_id, revision_number) REFERENCES ple_data.question_revision(question_id, revision_number)
);

CREATE TABLE ple_private.issued_question (
    issued_question_id uuid PRIMARY KEY,
    assessment_attempt_id uuid NOT NULL REFERENCES ple_private.assessment_attempt(assessment_attempt_id) ON DELETE CASCADE,
    assessment_entry_id uuid NOT NULL,
    assessment_content_entry_index integer NOT NULL CHECK (assessment_content_entry_index >= 0),
    issued_position integer NOT NULL CHECK (issued_position >= 0),
    question_id text NOT NULL,
    revision_number integer NOT NULL,
    -- This pre-render source-selection record is not completed reproduction
    -- evidence. Native PLE JSON retains no seed; renderer-backed Questions
    -- retain only the seed needed to obtain their later genuine hash.
    question_seed numeric(20, 0) CHECK (question_seed >= 0 AND question_seed <= 18446744073709551615),
    point_value numeric NOT NULL CHECK (point_value >= 0),
    scoring_rule text NOT NULL CHECK (scoring_rule IN ('normal', 'full_credit', 'extra_credit', 'excluded')),
    question_statistics_eligibility boolean NOT NULL,
    question_attempt_limit integer,
    question_attempt_time_limit_seconds integer,
    question_attempt_grace_seconds integer,
    question_pool_selection_id uuid,
    question_pool_member_position integer,
    UNIQUE (assessment_attempt_id, issued_position),
    FOREIGN KEY (question_id, revision_number) REFERENCES ple_data.question_revision(question_id, revision_number),
    FOREIGN KEY (question_pool_selection_id, assessment_attempt_id, assessment_entry_id)
        REFERENCES ple_private.question_pool_selection(question_pool_selection_id, assessment_attempt_id, assessment_entry_id),
    FOREIGN KEY (question_pool_selection_id, question_pool_member_position, question_id, revision_number)
        REFERENCES ple_private.question_pool_selected_item(question_pool_selection_id, member_position, question_id, revision_number),
    CHECK ((question_pool_selection_id IS NULL) = (question_pool_member_position IS NULL)),
    CHECK (question_attempt_limit IS NULL OR question_attempt_limit > 0),
    CHECK ((question_attempt_time_limit_seconds IS NULL AND question_attempt_grace_seconds IS NULL)
        OR (question_attempt_time_limit_seconds > 0 AND question_attempt_grace_seconds >= 0))
);

COMMENT ON TABLE ple_private.assessment_attempt IS 'role: student work, Immutable effective Assessment evidence for one Student Work occurrence; its immutable Assessment Submission is the sole completion authority.';

COMMENT ON TABLE ple_private.issued_question IS 'role: student work, Pre-render source-selection record: exact Assessment Entry identity, Question Revision, optional renderer seed, per-question policy, scoring, statistics, and pool-selection evidence for one issued position.';

-- Question-Assessment Attempt interaction and submission evidence below an Assessment Attempt.
CREATE TABLE ple_private.question_attempt (
    question_attempt_id uuid PRIMARY KEY,
    issued_question_id uuid NOT NULL UNIQUE REFERENCES ple_private.issued_question(issued_question_id) ON DELETE CASCADE,
    question_seed numeric(20, 0) CHECK (question_seed >= 0 AND question_seed <= 18446744073709551615),
    generated_parameter_sha256 text CHECK (generated_parameter_sha256 ~ '^[0-9a-f]{64}$'),
    issued_at timestamptz NOT NULL,
    deadline_at timestamptz,
    finalized_at timestamptz,
    question_attempt_state text NOT NULL CHECK (question_attempt_state IN ('open', 'response_finalized', 'closed_unanswered')),
    backend_name text NOT NULL CHECK (char_length(btrim(backend_name)) BETWEEN 1 AND 100),
    backend_version text NOT NULL CHECK (char_length(btrim(backend_version)) BETWEEN 1 AND 100),
    renderer_name text,
    renderer_version text,
    source_object_id uuid REFERENCES ple_private.object_record(object_id),
    source_object_checksum bytea CHECK (source_object_checksum IS NULL OR octet_length(source_object_checksum) = 32),
    grader_name text NOT NULL CHECK (char_length(btrim(grader_name)) BETWEEN 1 AND 100),
    grader_version text NOT NULL CHECK (char_length(btrim(grader_version)) BETWEEN 1 AND 100),
    rendered_question_sha256 bytea NOT NULL CHECK (octet_length(rendered_question_sha256) = 32),
    issued_capability text NOT NULL CHECK (issued_capability IN ('question_presentation', 'ple_question_json_presentation', 'webwork_presentation', 'not_applicable')),
    CHECK (deadline_at IS NULL OR deadline_at >= issued_at),
    -- Static PLE JSON has no generator-derived reproduction values.  The two
    -- fields are an atomic pair for renderer-backed Questions (ASVS 2.2.3).
    CHECK ((question_seed IS NULL) = (generated_parameter_sha256 IS NULL)),
    CHECK ((backend_name = 'ple'
            AND question_seed IS NULL AND generated_parameter_sha256 IS NULL)
        OR (backend_name IN ('webwork', 'imathas')
            AND question_seed IS NOT NULL AND generated_parameter_sha256 IS NOT NULL)),
    CHECK ((renderer_name IS NULL) = (renderer_version IS NULL)),
    CHECK ((source_object_id IS NULL) = (source_object_checksum IS NULL)),
    CHECK ((question_attempt_state = 'response_finalized') = (finalized_at IS NOT NULL)),
    CHECK (finalized_at IS NULL OR finalized_at >= issued_at)
);

CREATE TABLE ple_private.assessment_attempt_saved_response (
    question_attempt_id uuid PRIMARY KEY REFERENCES ple_private.question_attempt(question_attempt_id) ON DELETE CASCADE,
    student_response jsonb NOT NULL CHECK (jsonb_typeof(student_response) = 'object'),
    saved_at timestamptz NOT NULL
);

CREATE TABLE ple_private.question_response (
    question_response_id uuid PRIMARY KEY,
    assessment_submission_id uuid NOT NULL,
    question_attempt_id uuid NOT NULL UNIQUE REFERENCES ple_private.question_attempt(question_attempt_id) ON DELETE CASCADE,
    finalized_at timestamptz NOT NULL,
    student_response jsonb NOT NULL CHECK (jsonb_typeof(student_response) = 'object'),
    UNIQUE (question_response_id, question_attempt_id)
);

CREATE TABLE ple_private.assessment_submission (
    assessment_submission_id uuid PRIMARY KEY,
    assessment_attempt_id uuid NOT NULL UNIQUE REFERENCES ple_private.assessment_attempt(assessment_attempt_id) ON DELETE CASCADE,
    submitted_at timestamptz NOT NULL,
    finalization_kind text NOT NULL CHECK (finalization_kind IN ('student', 'deadline')),
    authorized_by_account_id uuid REFERENCES ple_private.account(account_id),
    receipt jsonb NOT NULL CHECK (jsonb_typeof(receipt) = 'object'),
    CHECK ((finalization_kind = 'student') = (authorized_by_account_id IS NOT NULL))
);

COMMENT ON TABLE ple_private.question_attempt IS 'role: student work, Exact reproduction and operational evidence for one Issued Question; mutable current Question content is not an interpretation source.';

COMMENT ON TABLE ple_private.assessment_attempt_saved_response IS 'role: student work, Private current response state before submission; root-owned by its Question Attempt.';

-- Immutable presentation bindings.  Backend-owned documents remain below a
-- Question Attempt so Unrelease can remove them with Student Work.
CREATE TABLE ple_private.question_attempt_presentation_binding (
    question_attempt_id uuid PRIMARY KEY REFERENCES ple_private.question_attempt(question_attempt_id) ON DELETE CASCADE,
    -- Preproduction direct cutover: the author-content digest descriptor is v3.
    -- Do not accept v1/v2 compatibility rows or synthesize a sentinel pair.
    descriptor_version smallint NOT NULL CHECK (descriptor_version = 3),
    presentation_nonce text NOT NULL CHECK (presentation_nonce ~ '^[0-9a-f]{32,128}$'),
    presentation_checksum bytea NOT NULL CHECK (octet_length(presentation_checksum) = 32),
    presentation jsonb NOT NULL CHECK (jsonb_typeof(presentation) = 'object'),
    -- Answer-free source for the separately isolated author-content document.
    -- It is never merged into the generic browser presentation JSON.
    author_content jsonb,
    -- The backend-owned document is retained with this immutable Question
    -- Assessment Attempt.  Only the WeBWorK capability supplies it.
    backend_document text,
    UNIQUE (question_attempt_id, presentation_nonce)
);



-- The public descriptor determines presentation references; this normalized
-- child retains only the durable response identity needed to interpret saved
-- Student Work after source content is gone.
CREATE TABLE ple_private.question_attempt_response_item_binding (
    question_attempt_id uuid NOT NULL REFERENCES ple_private.question_attempt_presentation_binding(question_attempt_id) ON DELETE CASCADE,
    presentation_response_item_reference text NOT NULL CHECK (presentation_response_item_reference ~ '^[0-9a-f]{4}$'),
    response_item_reference text NOT NULL CHECK (char_length(btrim(response_item_reference)) > 0),
    PRIMARY KEY (question_attempt_id, presentation_response_item_reference),
    UNIQUE (question_attempt_id, response_item_reference)
);

CREATE TABLE ple_private.question_attempt_presentation_asset_binding (
    question_attempt_id uuid PRIMARY KEY REFERENCES ple_private.question_attempt_presentation_binding(question_attempt_id) ON DELETE CASCADE
);

CREATE TABLE ple_private.question_attempt_presentation_asset_rendition (
    question_attempt_id uuid NOT NULL REFERENCES ple_private.question_attempt_presentation_asset_binding(question_attempt_id) ON DELETE CASCADE,
    asset_id uuid NOT NULL,
    question_asset_checksum bytea NOT NULL CHECK (octet_length(question_asset_checksum) = 32),
    rendition_checksum bytea NOT NULL CHECK (octet_length(rendition_checksum) = 32),
    intrinsic_width integer NOT NULL CHECK (intrinsic_width > 0),
    intrinsic_height integer NOT NULL CHECK (intrinsic_height > 0),
    PRIMARY KEY (question_attempt_id, asset_id)
);

COMMENT ON TABLE ple_private.question_attempt_presentation_binding IS 'role: student work, Checksummed issued presentation and, for backend-owned Questions, immutable document retained for one Question Attempt.';

-- Immutable grading evidence.  Every result is rooted at one accepted
-- Submission and retained Assessment Attempt evidence; Assessment Revision is not an
-- interpretation source.
CREATE TABLE ple_private.question_response_grading (
    question_response_grading_id uuid PRIMARY KEY,
    question_response_id uuid NOT NULL UNIQUE REFERENCES ple_private.question_response(question_response_id)
        ON DELETE CASCADE,
    grading_state text NOT NULL DEFAULT 'graded' CHECK (grading_state = 'graded'),
    created_at timestamptz NOT NULL,
    completed_at timestamptz,
    UNIQUE (question_response_grading_id, question_response_id),
    CHECK (completed_at IS NOT NULL),
    CHECK (completed_at IS NULL OR completed_at >= created_at)
);

CREATE TABLE ple_private.grading_result (
    grading_result_id uuid PRIMARY KEY,
    question_response_id uuid NOT NULL UNIQUE REFERENCES ple_private.question_response(question_response_id)
        ON DELETE CASCADE,
    question_response_grading_id uuid NOT NULL UNIQUE,
    question_attempt_id uuid NOT NULL UNIQUE REFERENCES ple_private.question_attempt(question_attempt_id)
        ON DELETE CASCADE,
    -- The Question Backend owns response interpretation.  PLE retains only
    -- its normalized immutable outcome; point values remain Assessment
    -- configuration and are applied by score readers.
    normalized_credit numeric NOT NULL CHECK (
        normalized_credit >= 0 AND normalized_credit <= 1
    ),
    recorded_at timestamptz NOT NULL,
    FOREIGN KEY (question_response_id, question_attempt_id)
        REFERENCES ple_private.question_response(question_response_id, question_attempt_id)
        ON DELETE CASCADE,
    FOREIGN KEY (question_response_grading_id, question_response_id)
        REFERENCES ple_private.question_response_grading(question_response_grading_id, question_response_id)
        ON DELETE CASCADE,
    UNIQUE (question_response_grading_id, grading_result_id)
);

SET LOCAL ROLE ple_audit_owner;

CREATE TABLE ple_audit.automated_grading_receipt (
    automated_grading_receipt_id uuid PRIMARY KEY,
    question_response_grading_id uuid NOT NULL,
    grading_result_id uuid NOT NULL UNIQUE,
    committed_at timestamptz NOT NULL,
    automated_grading_receipt_checksum bytea NOT NULL
        CHECK (octet_length(automated_grading_receipt_checksum) = 32),
    FOREIGN KEY (question_response_grading_id, grading_result_id)
        REFERENCES ple_private.grading_result(question_response_grading_id, grading_result_id)
        ON DELETE CASCADE
);

SET LOCAL ROLE ple_private_owner;

COMMENT ON TABLE ple_private.grading_result IS 'role: student work, One immutable normalized-credit outcome for an accepted Submission; current Assessment Entry points calculate scores on read.';

SET LOCAL ROLE ple_audit_owner;

COMMENT ON TABLE ple_audit.automated_grading_receipt IS 'role: event, Immutable receipt for one automated grading commit; deleted only with its exclusive Student Work root.';

SET LOCAL ROLE ple_private_owner;

COMMENT ON TABLE ple_private.student_assessment_accommodation IS 'role: student work, deleted by Unrelease and FERPA purge of the Course Instance. HUMAN_GUIDANCE.md Student Work.';

COMMENT ON TABLE ple_private.question_pool_selection IS 'role: student work, deleted by Unrelease and FERPA purge of the Course Instance. HUMAN_GUIDANCE.md Student Work.';

COMMENT ON TABLE ple_private.question_pool_selected_item IS 'role: student work, deleted by Unrelease and FERPA purge of the Course Instance. HUMAN_GUIDANCE.md Student Work.';

COMMENT ON TABLE ple_private.question_response IS 'role: student work, deleted by Unrelease and FERPA purge of the Course Instance. HUMAN_GUIDANCE.md Student Work.';

COMMENT ON TABLE ple_private.assessment_submission IS 'role: student work, deleted by Unrelease and FERPA purge of the Course Instance. HUMAN_GUIDANCE.md Student Work.';

COMMENT ON TABLE ple_private.question_attempt_response_item_binding IS 'role: student work, deleted by Unrelease and FERPA purge of the Course Instance. HUMAN_GUIDANCE.md Student Work.';

COMMENT ON TABLE ple_private.question_attempt_presentation_asset_binding IS 'role: student work, deleted by Unrelease and FERPA purge of the Course Instance. HUMAN_GUIDANCE.md Student Work.';

COMMENT ON TABLE ple_private.question_attempt_presentation_asset_rendition IS 'role: student work, deleted by Unrelease and FERPA purge of the Course Instance. HUMAN_GUIDANCE.md Student Work.';

COMMENT ON TABLE ple_private.question_response_grading IS 'role: student work, deleted by Unrelease and FERPA purge of the Course Instance. HUMAN_GUIDANCE.md Student Work.';

