-- assessment_attempt tables. CREATE TABLE and COMMENT ON only.
-- Functions, policies, and privileges live in later layers.

SET LOCAL ROLE ple_private_owner;

CREATE TABLE ple_private.student_assessment_accommodation (
    accommodation_id uuid PRIMARY KEY,
    course_instance_id ple_data.course_instance_id NOT NULL,
    student_record_id uuid NOT NULL,
    assessment_id ple_data.assessment_id NOT NULL,
    available_at timestamptz,
    due_at timestamptz,
    closes_at timestamptz,
    time_multiplier numeric,
    assessment_attempt_limit integer,
    created_at timestamptz NOT NULL,
    updated_at timestamptz NOT NULL DEFAULT pg_catalog.transaction_timestamp(),
    CHECK (updated_at >= created_at),
    accommodation_edit_number bigint NOT NULL DEFAULT 1
        CHECK (accommodation_edit_number > 0),
    CHECK ((available_at IS NULL OR due_at IS NULL OR available_at <= due_at)
       AND (due_at IS NULL OR closes_at IS NULL OR due_at <= closes_at)),
    CHECK (time_multiplier IS NULL
       OR (time_multiplier >= 1 AND time_multiplier < 'Infinity'::numeric)),
    CHECK (assessment_attempt_limit IS NULL OR assessment_attempt_limit > 0),
    UNIQUE (accommodation_id, student_record_id, assessment_id, course_instance_id),
    UNIQUE (student_record_id, assessment_id),
    FOREIGN KEY (student_record_id, course_instance_id)
        REFERENCES ple_data.student_record(student_record_id, course_instance_id),
    FOREIGN KEY (assessment_id, course_instance_id)
        REFERENCES ple_data.assessment(assessment_id, course_instance_id)
);

CREATE TABLE ple_private.assessment_attempt (
    course_instance_id ple_data.course_instance_id NOT NULL,
    assessment_attempt_id uuid NOT NULL,
    student_record_id uuid NOT NULL,
    assessment_id ple_data.assessment_id NOT NULL,
    assessment_attempt_number integer NOT NULL CHECK (assessment_attempt_number > 0),
    started_at timestamptz NOT NULL,
    expires_at timestamptz,
    assessment_policy_snapshot_id ple_data.sha256_digest NOT NULL
        REFERENCES ple_data.assessment_policy_snapshot(assessment_policy_snapshot_id),
    schedule_accommodation_id uuid,
    schedule_accommodation_edit_number bigint CHECK (schedule_accommodation_edit_number > 0),
    time_limit_accommodation_id uuid,
    time_limit_accommodation_edit_number bigint CHECK (time_limit_accommodation_edit_number > 0),
    assessment_attempt_limit_accommodation_id uuid,
    assessment_attempt_limit_accommodation_edit_number bigint CHECK (assessment_attempt_limit_accommodation_edit_number > 0),
    PRIMARY KEY (course_instance_id, assessment_attempt_id),
    UNIQUE (course_instance_id, student_record_id, assessment_id, assessment_attempt_number),
    CHECK (expires_at IS NULL OR expires_at >= started_at),
    FOREIGN KEY (student_record_id, course_instance_id)
        REFERENCES ple_data.student_record(student_record_id, course_instance_id),
    FOREIGN KEY (assessment_id, course_instance_id)
        REFERENCES ple_data.assessment(assessment_id, course_instance_id),
    FOREIGN KEY (schedule_accommodation_id, student_record_id, assessment_id, course_instance_id)
        REFERENCES ple_private.student_assessment_accommodation(
            accommodation_id, student_record_id, assessment_id, course_instance_id),
    FOREIGN KEY (time_limit_accommodation_id, student_record_id, assessment_id, course_instance_id)
        REFERENCES ple_private.student_assessment_accommodation(
            accommodation_id, student_record_id, assessment_id, course_instance_id),
    FOREIGN KEY (assessment_attempt_limit_accommodation_id, student_record_id, assessment_id, course_instance_id)
        REFERENCES ple_private.student_assessment_accommodation(
            accommodation_id, student_record_id, assessment_id, course_instance_id),
    CHECK ((schedule_accommodation_id IS NULL) = (schedule_accommodation_edit_number IS NULL)),
    CHECK ((time_limit_accommodation_id IS NULL) = (time_limit_accommodation_edit_number IS NULL)),
    CHECK ((assessment_attempt_limit_accommodation_id IS NULL) = (assessment_attempt_limit_accommodation_edit_number IS NULL))
);

CREATE TABLE ple_private.question_pool_selection (
    course_instance_id ple_data.course_instance_id NOT NULL,
    question_pool_selection_id uuid NOT NULL,
    assessment_attempt_id uuid NOT NULL,
    -- This is the stable authored identity copied at issue time, not a foreign
    -- key to the mutable current Assessment Entry.  Released Assessment saves
    -- may replace current entries while this Student Work remains interpretable.
    assessment_entry_id uuid NOT NULL,
    question_pool_id ple_data.question_family_id NOT NULL
        REFERENCES ple_data.question_pool(question_pool_id),
    question_pool_edit_number bigint NOT NULL CHECK (question_pool_edit_number > 0),
    created_at timestamptz NOT NULL,
    PRIMARY KEY (course_instance_id, question_pool_selection_id),
    UNIQUE (course_instance_id, question_pool_selection_id, assessment_attempt_id, assessment_entry_id),
    UNIQUE (course_instance_id, assessment_attempt_id, assessment_entry_id),
    FOREIGN KEY (course_instance_id, assessment_attempt_id)
        REFERENCES ple_private.assessment_attempt(course_instance_id, assessment_attempt_id)
        ON DELETE CASCADE
);

CREATE TABLE ple_private.question_pool_selected_item (
    course_instance_id ple_data.course_instance_id NOT NULL,
    question_pool_selection_id uuid NOT NULL,
    member_position integer NOT NULL CHECK (member_position > 0),
    selection_position integer NOT NULL CHECK (selection_position >= 0),
    published_question_id ple_data.question_family_id NOT NULL,
    revision_number integer NOT NULL,
    PRIMARY KEY (course_instance_id, question_pool_selection_id, selection_position),
    UNIQUE (course_instance_id, question_pool_selection_id, member_position),
    UNIQUE (course_instance_id, question_pool_selection_id, member_position, published_question_id, revision_number),
    FOREIGN KEY (course_instance_id, question_pool_selection_id)
        REFERENCES ple_private.question_pool_selection(course_instance_id, question_pool_selection_id)
        ON DELETE CASCADE,
    FOREIGN KEY (published_question_id, revision_number)
        REFERENCES ple_data.question_revision(published_question_id, revision_number),
    created_at timestamptz NOT NULL DEFAULT pg_catalog.transaction_timestamp()
);

-- Frozen Assessment Entry facts needed to score or interpret an Issued
-- Question. Two equal Entry configurations share one immutable row.
CREATE TABLE ple_private.assessment_entry_snapshot (
    assessment_entry_snapshot_id ple_data.sha256_digest PRIMARY KEY,
    entry_kind ple_data.entry_kind NOT NULL,
    scoring_rule ple_data.scoring_rule NOT NULL,
    points numeric NOT NULL CHECK (
        points BETWEEN 0 AND 1000000000.9999 AND scale(points) <= 4
    ),
    published_question_id ple_data.question_family_id,
    question_revision_number integer,
    question_pool_id ple_data.question_family_id
        REFERENCES ple_data.question_pool(question_pool_id),
    question_attempt_limit integer,
    question_attempt_time_limit_seconds integer,
    question_attempt_grace_seconds integer,
    created_at timestamptz NOT NULL,
    FOREIGN KEY (published_question_id, question_revision_number)
        REFERENCES ple_data.question_revision(published_question_id, revision_number),
    CHECK (
        (entry_kind = 'fixed_question'
            AND published_question_id IS NOT NULL
            AND question_revision_number IS NOT NULL
            AND question_pool_id IS NULL)
        OR (entry_kind = 'question_pool'
            AND published_question_id IS NULL
            AND question_revision_number IS NULL
            AND question_pool_id IS NOT NULL)
    ),
    CHECK (question_attempt_limit IS NULL OR question_attempt_limit > 0),
    CHECK (
        (question_attempt_time_limit_seconds IS NULL AND question_attempt_grace_seconds IS NULL)
        OR (question_attempt_time_limit_seconds > 0
            AND question_attempt_grace_seconds >= 0)
    )
);

CREATE TABLE ple_private.issued_question (
    course_instance_id ple_data.course_instance_id NOT NULL,
    issued_question_id uuid NOT NULL,
    assessment_attempt_id uuid NOT NULL,
    assessment_entry_id uuid NOT NULL,
    assessment_entry_snapshot_id ple_data.sha256_digest NOT NULL
        REFERENCES ple_private.assessment_entry_snapshot(assessment_entry_snapshot_id),
    assessment_content_entry_index integer NOT NULL CHECK (assessment_content_entry_index >= 0),
    issued_position integer NOT NULL CHECK (issued_position >= 0),
    published_question_id ple_data.question_family_id NOT NULL,
    revision_number integer NOT NULL,
    -- This pre-render source-selection record is not completed reproduction
    -- evidence. Native PLE JSON retains no seed; renderer-backed Questions
    -- retain only the seed needed to obtain their later genuine hash.
    question_seed numeric(20, 0) CHECK (question_seed >= 0 AND question_seed <= 18446744073709551615),
    question_statistics_eligibility boolean NOT NULL,
    question_pool_selection_id uuid,
    question_pool_member_position integer,
    PRIMARY KEY (course_instance_id, issued_question_id),
    UNIQUE (course_instance_id, assessment_attempt_id, issued_position),
    FOREIGN KEY (course_instance_id, assessment_attempt_id)
        REFERENCES ple_private.assessment_attempt(course_instance_id, assessment_attempt_id)
        ON DELETE CASCADE,
    FOREIGN KEY (published_question_id, revision_number)
        REFERENCES ple_data.question_revision(published_question_id, revision_number),
    FOREIGN KEY (course_instance_id, question_pool_selection_id, assessment_attempt_id, assessment_entry_id)
        REFERENCES ple_private.question_pool_selection(
            course_instance_id, question_pool_selection_id, assessment_attempt_id, assessment_entry_id),
    FOREIGN KEY (course_instance_id, question_pool_selection_id, question_pool_member_position, published_question_id, revision_number)
        REFERENCES ple_private.question_pool_selected_item(
            course_instance_id, question_pool_selection_id, member_position, published_question_id, revision_number),
    CHECK ((question_pool_selection_id IS NULL) = (question_pool_member_position IS NULL)),
    created_at timestamptz NOT NULL DEFAULT pg_catalog.transaction_timestamp()
);

-- Shared immutable delivery toolchain. Two Question Attempts with the same
-- seven values reuse one row. Open vs finalized is question_attempt.finalized_at.
CREATE TABLE ple_private.delivery_toolchain (
    delivery_toolchain_id uuid PRIMARY KEY,
    backend_name ple_data.question_backend NOT NULL,
    backend_version text NOT NULL CHECK (char_length(btrim(backend_version)) BETWEEN 1 AND 100),
    renderer_name text CHECK (
        renderer_name IS NULL
        OR char_length(btrim(renderer_name)) BETWEEN 1 AND 100
    ),
    renderer_version text CHECK (
        renderer_version IS NULL
        OR char_length(btrim(renderer_version)) BETWEEN 1 AND 100
    ),
    grader_name text NOT NULL CHECK (char_length(btrim(grader_name)) BETWEEN 1 AND 100),
    grader_version text NOT NULL CHECK (char_length(btrim(grader_version)) BETWEEN 1 AND 100),
    issued_capability ple_data.issued_capability NOT NULL,
    created_at timestamptz NOT NULL,
    CHECK ((renderer_name IS NULL) = (renderer_version IS NULL)),
    CONSTRAINT delivery_toolchain_values_key UNIQUE NULLS NOT DISTINCT (
        backend_name, backend_version, renderer_name, renderer_version,
        grader_name, grader_version, issued_capability
    )
);

CREATE TABLE ple_private.question_attempt (
    course_instance_id ple_data.course_instance_id NOT NULL,
    question_attempt_id uuid NOT NULL,
    issued_question_id uuid NOT NULL,
    question_seed numeric(20, 0) CHECK (question_seed >= 0 AND question_seed <= 18446744073709551615),
    generated_parameter_sha256 text CHECK (generated_parameter_sha256 ~ '^[0-9a-f]{64}$'),
    issued_at timestamptz NOT NULL,
    deadline_at timestamptz,
    finalized_at timestamptz,
    delivery_toolchain_id uuid NOT NULL
        REFERENCES ple_private.delivery_toolchain(delivery_toolchain_id),
    source_object_record_id uuid REFERENCES ple_private.object_record(object_record_id),
    source_object_checksum bytea CHECK (source_object_checksum IS NULL OR octet_length(source_object_checksum) = 32),
    rendered_question_sha256 bytea NOT NULL CHECK (octet_length(rendered_question_sha256) = 32),
    PRIMARY KEY (course_instance_id, question_attempt_id),
    UNIQUE (course_instance_id, issued_question_id),
    FOREIGN KEY (course_instance_id, issued_question_id)
        REFERENCES ple_private.issued_question(course_instance_id, issued_question_id)
        ON DELETE CASCADE,
    CHECK (deadline_at IS NULL OR deadline_at >= issued_at),
    -- Static PLE JSON has no generator-derived reproduction values.  The two
    -- fields are an atomic pair for renderer-backed Questions (ASVS 2.2.3).
    -- Backend pairing is enforced against delivery_toolchain at issue time.
    CHECK ((question_seed IS NULL) = (generated_parameter_sha256 IS NULL)),
    CHECK ((source_object_record_id IS NULL) = (source_object_checksum IS NULL)),
    CHECK (finalized_at IS NULL OR finalized_at >= issued_at)
);

CREATE TABLE ple_private.assessment_submission (
    course_instance_id ple_data.course_instance_id NOT NULL,
    assessment_submission_id uuid NOT NULL,
    assessment_attempt_id uuid NOT NULL,
    submitted_at timestamptz NOT NULL,
    authorized_by_account_id ple_data.account_id REFERENCES ple_private.account(account_id),
    PRIMARY KEY (course_instance_id, assessment_submission_id),
    UNIQUE (course_instance_id, assessment_attempt_id),
    FOREIGN KEY (course_instance_id, assessment_attempt_id)
        REFERENCES ple_private.assessment_attempt(course_instance_id, assessment_attempt_id)
        ON DELETE CASCADE
);

CREATE TABLE ple_private.assessment_attempt_saved_response (
    course_instance_id ple_data.course_instance_id NOT NULL,
    question_attempt_id uuid NOT NULL,
    student_response jsonb NOT NULL CHECK (jsonb_typeof(student_response) = 'object'),
    saved_at timestamptz NOT NULL,
    finalized_at timestamptz,
    assessment_submission_id uuid,
    PRIMARY KEY (course_instance_id, question_attempt_id),
    FOREIGN KEY (course_instance_id, question_attempt_id)
        REFERENCES ple_private.question_attempt(course_instance_id, question_attempt_id)
        ON DELETE CASCADE,
    FOREIGN KEY (course_instance_id, assessment_submission_id)
        REFERENCES ple_private.assessment_submission(course_instance_id, assessment_submission_id)
        ON DELETE CASCADE,
    CHECK ((finalized_at IS NULL) = (assessment_submission_id IS NULL)),
    CHECK (finalized_at IS NULL OR finalized_at >= saved_at)
);

CREATE TABLE ple_private.question_attempt_presentation_binding (
    course_instance_id ple_data.course_instance_id NOT NULL,
    question_attempt_id uuid NOT NULL,
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
    PRIMARY KEY (course_instance_id, question_attempt_id),
    UNIQUE (course_instance_id, question_attempt_id, presentation_nonce),
    FOREIGN KEY (course_instance_id, question_attempt_id)
        REFERENCES ple_private.question_attempt(course_instance_id, question_attempt_id)
        ON DELETE CASCADE,
    created_at timestamptz NOT NULL DEFAULT pg_catalog.transaction_timestamp()
);

CREATE TABLE ple_private.question_attempt_response_item_binding (
    course_instance_id ple_data.course_instance_id NOT NULL,
    question_attempt_id uuid NOT NULL,
    presentation_response_item_id text NOT NULL CHECK (presentation_response_item_id ~ '^[0-9a-f]{4}$'),
    response_item_id text NOT NULL CHECK (char_length(btrim(response_item_id)) > 0),
    PRIMARY KEY (course_instance_id, question_attempt_id, presentation_response_item_id),
    UNIQUE (course_instance_id, question_attempt_id, response_item_id),
    FOREIGN KEY (course_instance_id, question_attempt_id)
        REFERENCES ple_private.question_attempt_presentation_binding(course_instance_id, question_attempt_id)
        ON DELETE CASCADE,
    created_at timestamptz NOT NULL DEFAULT pg_catalog.transaction_timestamp()
);

CREATE TABLE ple_private.question_attempt_presentation_asset_binding (
    course_instance_id ple_data.course_instance_id NOT NULL,
    question_attempt_id uuid NOT NULL,
    PRIMARY KEY (course_instance_id, question_attempt_id),
    FOREIGN KEY (course_instance_id, question_attempt_id)
        REFERENCES ple_private.question_attempt_presentation_binding(course_instance_id, question_attempt_id)
        ON DELETE CASCADE,
    created_at timestamptz NOT NULL DEFAULT pg_catalog.transaction_timestamp()
);

CREATE TABLE ple_private.question_attempt_presentation_image_rendition (
    course_instance_id ple_data.course_instance_id NOT NULL,
    question_attempt_id uuid NOT NULL,
    question_image_asset_id uuid NOT NULL,
    question_image_checksum bytea NOT NULL CHECK (octet_length(question_image_checksum) = 32),
    rendition_checksum bytea NOT NULL CHECK (octet_length(rendition_checksum) = 32),
    intrinsic_width integer NOT NULL CHECK (intrinsic_width > 0),
    intrinsic_height integer NOT NULL CHECK (intrinsic_height > 0),
    PRIMARY KEY (course_instance_id, question_attempt_id, question_image_asset_id),
    FOREIGN KEY (course_instance_id, question_attempt_id)
        REFERENCES ple_private.question_attempt_presentation_asset_binding(course_instance_id, question_attempt_id)
        ON DELETE CASCADE,
    created_at timestamptz NOT NULL DEFAULT pg_catalog.transaction_timestamp()
);

CREATE TABLE ple_private.grading_result (
    course_instance_id ple_data.course_instance_id NOT NULL,
    grading_result_id uuid NOT NULL,
    question_attempt_id uuid NOT NULL,
    -- The Question Backend owns response interpretation.  PLE retains only
    -- its normalized immutable outcome; point values remain Assessment
    -- configuration and are applied by score readers.
    normalized_credit numeric NOT NULL CHECK (
        normalized_credit >= 0 AND normalized_credit <= 1
    ),
    recorded_at timestamptz NOT NULL,
    PRIMARY KEY (course_instance_id, grading_result_id),
    UNIQUE (course_instance_id, question_attempt_id),
    FOREIGN KEY (course_instance_id, question_attempt_id)
        REFERENCES ple_private.question_attempt(course_instance_id, question_attempt_id)
        ON DELETE CASCADE
);

SET LOCAL ROLE ple_audit_owner;

CREATE TABLE ple_audit.automated_grading_receipt (
    automated_grading_receipt_id uuid PRIMARY KEY,
    course_instance_id ple_data.course_instance_id NOT NULL,
    grading_result_id uuid NOT NULL,
    committed_at timestamptz NOT NULL,
    automated_grading_receipt_checksum bytea NOT NULL
        CHECK (octet_length(automated_grading_receipt_checksum) = 32),
    UNIQUE (course_instance_id, grading_result_id),
    FOREIGN KEY (course_instance_id, grading_result_id)
        REFERENCES ple_private.grading_result(course_instance_id, grading_result_id)
        ON DELETE CASCADE
);

SET LOCAL ROLE ple_private_owner;
COMMENT ON TABLE ple_private.student_assessment_accommodation IS 'role: current state, Course-scoped per-Student Assessment schedule and time configuration copied onto Assessment Attempts at start.';
COMMENT ON TABLE ple_private.assessment_attempt IS 'role: student work, Immutable effective Assessment evidence for one Student Work occurrence; its immutable Assessment Submission is the sole completion authority.';
COMMENT ON TABLE ple_private.question_pool_selection IS 'role: student work, deleted by Unrelease and FERPA purge of the Course Instance. HUMAN_GUIDANCE.md Student Work.';
COMMENT ON TABLE ple_private.question_pool_selected_item IS 'role: student work, deleted by Unrelease and FERPA purge of the Course Instance. HUMAN_GUIDANCE.md Student Work.';
COMMENT ON TABLE ple_private.assessment_entry_snapshot IS 'role: snapshot, deleted by nothing (shared, immutable). HUMAN_GUIDANCE.md Assessment Attempt snapshots.';
COMMENT ON TABLE ple_private.issued_question IS 'role: student work, Pre-render source-selection record: exact Assessment Entry identity, Entry snapshot, Question Revision, optional renderer seed, statistics, and pool-selection evidence for one issued position.';
COMMENT ON TABLE ple_private.delivery_toolchain IS 'role: snapshot, Shared immutable Question delivery toolchain; deleted by nothing. HUMAN_GUIDANCE.md Student Work.';
COMMENT ON TABLE ple_private.question_attempt IS 'role: student work, Exact reproduction and operational evidence for one Issued Question; mutable current Question content is not an interpretation source.';
COMMENT ON TABLE ple_private.assessment_attempt_saved_response IS 'role: student work, Private response bytes for one Question Attempt, finalized in place on submit; unanswered Issued Questions have no row.';
COMMENT ON TABLE ple_private.assessment_submission IS 'role: student work, deleted by Unrelease and FERPA purge of the Course Instance. HUMAN_GUIDANCE.md Student Work.';
COMMENT ON TABLE ple_private.question_attempt_presentation_binding IS 'role: student work, Checksummed issued presentation and, for backend-owned Questions, immutable document retained for one Question Attempt.';
COMMENT ON TABLE ple_private.question_attempt_response_item_binding IS 'role: student work, deleted by Unrelease and FERPA purge of the Course Instance. HUMAN_GUIDANCE.md Student Work.';
COMMENT ON TABLE ple_private.question_attempt_presentation_asset_binding IS 'role: student work, deleted by Unrelease and FERPA purge of the Course Instance. HUMAN_GUIDANCE.md Student Work.';
COMMENT ON TABLE ple_private.question_attempt_presentation_image_rendition IS 'role: student work, deleted by Unrelease and FERPA purge of the Course Instance. HUMAN_GUIDANCE.md Student Work.';
COMMENT ON TABLE ple_private.grading_result IS 'role: student work, One immutable normalized-credit outcome for an accepted Submission; current Assessment Entry points calculate scores on read.';

SET LOCAL ROLE ple_audit_owner;
COMMENT ON TABLE ple_audit.automated_grading_receipt IS 'role: event, Immutable receipt for one automated grading commit; deleted only with its exclusive Student Work root.';

SET LOCAL ROLE ple_private_owner;
COMMENT ON COLUMN ple_private.student_assessment_accommodation.available_at IS 'NULL means this optional fact is absent.';
COMMENT ON COLUMN ple_private.student_assessment_accommodation.due_at IS 'NULL means this optional fact is absent.';
COMMENT ON COLUMN ple_private.student_assessment_accommodation.closes_at IS 'NULL means this optional fact is absent.';
COMMENT ON COLUMN ple_private.student_assessment_accommodation.time_multiplier IS 'NULL means this optional fact is absent.';
COMMENT ON COLUMN ple_private.student_assessment_accommodation.assessment_attempt_limit IS 'NULL means this optional fact is absent.';
COMMENT ON COLUMN ple_private.assessment_attempt.expires_at IS 'NULL means this optional fact is absent.';
COMMENT ON COLUMN ple_private.assessment_attempt.schedule_accommodation_id IS 'NULL means this optional fact is absent.';
COMMENT ON COLUMN ple_private.assessment_attempt.schedule_accommodation_edit_number IS 'NULL means this optional fact is absent.';
COMMENT ON COLUMN ple_private.assessment_attempt.time_limit_accommodation_id IS 'NULL means this optional fact is absent.';
COMMENT ON COLUMN ple_private.assessment_attempt.time_limit_accommodation_edit_number IS 'NULL means this optional fact is absent.';
COMMENT ON COLUMN ple_private.assessment_attempt.assessment_attempt_limit_accommodation_id IS 'NULL means this optional fact is absent.';
COMMENT ON COLUMN ple_private.assessment_attempt.assessment_attempt_limit_accommodation_edit_number IS 'NULL means this optional fact is absent.';
COMMENT ON COLUMN ple_private.assessment_entry_snapshot.published_question_id IS 'NULL means this snapshot is a Question Pool Entry.';
COMMENT ON COLUMN ple_private.assessment_entry_snapshot.question_revision_number IS 'NULL means this snapshot is a Question Pool Entry.';
COMMENT ON COLUMN ple_private.assessment_entry_snapshot.question_pool_id IS 'NULL means this snapshot is a fixed Question Entry.';
COMMENT ON COLUMN ple_private.assessment_entry_snapshot.question_attempt_limit IS 'NULL means this optional fact is absent.';
COMMENT ON COLUMN ple_private.assessment_entry_snapshot.question_attempt_time_limit_seconds IS 'NULL means this optional fact is absent.';
COMMENT ON COLUMN ple_private.assessment_entry_snapshot.question_attempt_grace_seconds IS 'NULL means this optional fact is absent.';
COMMENT ON COLUMN ple_private.issued_question.question_seed IS 'NULL means this optional fact is absent.';
COMMENT ON COLUMN ple_private.issued_question.question_pool_selection_id IS 'NULL means this optional fact is absent.';
COMMENT ON COLUMN ple_private.issued_question.question_pool_member_position IS 'NULL means this optional fact is absent.';
COMMENT ON COLUMN ple_private.delivery_toolchain.renderer_name IS 'NULL means this toolchain has no renderer.';
COMMENT ON COLUMN ple_private.delivery_toolchain.renderer_version IS 'NULL means this toolchain has no renderer.';
COMMENT ON COLUMN ple_private.question_attempt.question_seed IS 'NULL means this optional fact is absent.';
COMMENT ON COLUMN ple_private.question_attempt.generated_parameter_sha256 IS 'NULL means this optional fact is absent.';
COMMENT ON COLUMN ple_private.question_attempt.deadline_at IS 'NULL means this optional fact is absent.';
COMMENT ON COLUMN ple_private.question_attempt.finalized_at IS 'NULL means the Question Attempt is still open.';
COMMENT ON COLUMN ple_private.question_attempt.source_object_record_id IS 'NULL means this optional fact is absent.';
COMMENT ON COLUMN ple_private.question_attempt.source_object_checksum IS 'NULL means this optional fact is absent.';
COMMENT ON COLUMN ple_private.assessment_submission.authorized_by_account_id IS 'NULL means deadline finalization; non-NULL means the Student submitted.';
COMMENT ON COLUMN ple_private.assessment_attempt_saved_response.finalized_at IS 'NULL means the owning Assessment Attempt is still open.';
COMMENT ON COLUMN ple_private.assessment_attempt_saved_response.assessment_submission_id IS 'NULL means the owning Assessment Attempt is still open.';
COMMENT ON COLUMN ple_private.question_attempt_presentation_binding.author_content IS 'NULL means this optional fact is absent.';
COMMENT ON COLUMN ple_private.question_attempt_presentation_binding.backend_document IS 'NULL means this optional fact is absent.';
