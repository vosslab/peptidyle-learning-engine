-- M11 durable native static PLE Question Presentation issuance.
--
-- A presentation binding is deliberately one-to-one with a Question Attempt:
-- it stores only the nonce and full descriptor checksum needed to reproduce the
-- answer-free public descriptor.  Source and reproduction facts remain in the
-- existing private Question Attempt payload.

SET LOCAL ROLE ple_private_owner;

CREATE TABLE ple_private.question_attempt_presentation_binding (
    question_attempt_id uuid PRIMARY KEY
        REFERENCES ple_private.question_attempt (question_attempt_id),
    descriptor_version smallint NOT NULL CHECK (descriptor_version = 1),
    presentation_nonce text NOT NULL CHECK (presentation_nonce ~ '^[0-9a-f]{32}$'),
    presentation_checksum text NOT NULL CHECK (presentation_checksum ~ '^[0-9a-f]{64}$')
);

CREATE FUNCTION ple_private.reject_question_attempt_presentation_binding_change()
RETURNS trigger LANGUAGE plpgsql
SET search_path = pg_catalog, ple_private AS $$
BEGIN
    RAISE EXCEPTION USING ERRCODE = '23514',
        MESSAGE = 'Question Attempt Presentation Binding is immutable';
END
$$;
CREATE TRIGGER question_attempt_presentation_binding_is_immutable
BEFORE UPDATE OR DELETE ON ple_private.question_attempt_presentation_binding
FOR EACH ROW EXECUTE FUNCTION ple_private.reject_question_attempt_presentation_binding_change();

ALTER TABLE ple_private.question_attempt_presentation_binding ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.question_attempt_presentation_binding FORCE ROW LEVEL SECURITY;
CREATE POLICY question_attempt_presentation_binding_private_owner_access
    ON ple_private.question_attempt_presentation_binding
    FOR ALL TO ple_private_owner USING (true) WITH CHECK (true);
CREATE POLICY question_attempt_presentation_binding_api_owner_issue
    ON ple_private.question_attempt_presentation_binding
    FOR SELECT TO ple_api_owner USING (true);
CREATE POLICY question_attempt_presentation_binding_api_owner_insert
    ON ple_private.question_attempt_presentation_binding
    FOR INSERT TO ple_api_owner WITH CHECK (true);
GRANT SELECT, INSERT ON TABLE ple_private.question_attempt_presentation_binding TO ple_api_owner;
-- The API owner can add only the private Question Attempt evidence required by
-- its non-public SECURITY DEFINER issue procedure; ple_app remains procedure-only.
CREATE POLICY question_attempt_api_owner_native_ple_issue
    ON ple_private.question_attempt FOR INSERT TO ple_api_owner WITH CHECK (true);
CREATE POLICY question_attempt_api_owner_native_ple_read
    ON ple_private.question_attempt FOR SELECT TO ple_api_owner USING (true);
CREATE POLICY issued_question_api_owner_native_ple_read
    ON ple_private.issued_question FOR SELECT TO ple_api_owner USING (true);
GRANT SELECT, INSERT ON TABLE ple_private.question_attempt TO ple_api_owner;
GRANT SELECT ON TABLE ple_private.issued_question TO ple_api_owner;

-- A Question Attempt's reproduction evidence is write-once.  Later grading
-- transitions may change only question_attempt_state (the established 0102
-- path); neither API-owner privilege nor a future routine may rewrite what
-- the Student was issued.
CREATE FUNCTION ple_private.reject_question_attempt_reproduction_change()
RETURNS trigger LANGUAGE plpgsql
SET search_path = pg_catalog, ple_private AS $$
BEGIN
    IF NEW.issued_question_id IS DISTINCT FROM OLD.issued_question_id
       OR NEW.question_seed IS DISTINCT FROM OLD.question_seed
       OR NEW.generated_parameter_sha256 IS DISTINCT FROM OLD.generated_parameter_sha256
       OR NEW.issued_at IS DISTINCT FROM OLD.issued_at
       OR NEW.deadline_at IS DISTINCT FROM OLD.deadline_at
       OR NEW.reproduction_details IS DISTINCT FROM OLD.reproduction_details THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Question Attempt reproduction evidence is immutable';
    END IF;
    RETURN NEW;
END
$$;
CREATE TRIGGER question_attempt_reproduction_evidence_is_immutable
BEFORE UPDATE ON ple_private.question_attempt
FOR EACH ROW EXECUTE FUNCTION ple_private.reject_question_attempt_reproduction_change();
REVOKE ALL PRIVILEGES ON TABLE ple_private.question_attempt_presentation_binding FROM PUBLIC;
REVOKE ALL PRIVILEGES ON FUNCTION ple_private.reject_question_attempt_presentation_binding_change()
    FROM PUBLIC;
REVOKE ALL PRIVILEGES ON FUNCTION ple_private.reject_question_attempt_reproduction_change()
    FROM PUBLIC;

CREATE FUNCTION ple_private.live_demo_native_ple_issuance_source_binding(
    p_assignment_revision_id uuid
)
RETURNS TABLE (
    assignment_entry_id uuid,
    issued_position integer,
    question_id text,
    revision_number integer,
    source_object_id uuid,
    source_object_address jsonb,
    source_object_checksum text
)
LANGUAGE sql SECURITY DEFINER
SET search_path = pg_catalog, ple_data, ple_private AS $$
    SELECT entry.assignment_entry_id, entry.assignment_content_entry_index,
           fixed.question_id, fixed.revision_number,
           binding.source_object_id, object_record.object_address, binding.source_object_checksum
      FROM ple_data.assignment_revision_entry AS entry
      JOIN ple_data.assignment_revision_fixed_question AS fixed
        ON fixed.assignment_revision_id = entry.assignment_revision_id
       AND fixed.assignment_entry_id = entry.assignment_entry_id
      JOIN ple_private.question_revision_source_binding AS binding
        ON binding.question_id = fixed.question_id AND binding.revision_number = fixed.revision_number
      JOIN ple_private.object_record AS object_record ON object_record.object_id = binding.source_object_id
     WHERE entry.assignment_revision_id = p_assignment_revision_id
       AND binding.backend = 'ple' AND binding.question_format = 'pleQuestionJson'
     ORDER BY entry.assignment_content_entry_index
$$;
REVOKE ALL PRIVILEGES ON FUNCTION ple_private.live_demo_native_ple_issuance_source_binding(uuid)
    FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_private.live_demo_native_ple_issuance_source_binding(uuid)
    TO ple_api_owner;
RESET ROLE;

SET LOCAL ROLE ple_api_owner;

-- This authorization/read seam is intentionally private to the API owner.  It
-- returns no source data to ple_app or to a browser; the Rust server resolves
-- the exact immutable source through its own S3 ObjectStore before commit.
CREATE FUNCTION ple_api.prepare_live_demo_native_ple_issuance(
    p_course_reference_number bigint,
    p_assignment_reference_number bigint
)
RETURNS TABLE (
    assignment_entry_id uuid,
    issued_position integer,
    question_id text,
    revision_number integer,
    source_object_id uuid,
    source_object_address jsonb,
    source_object_checksum text,
    resumed boolean,
    question_seed numeric,
    presentation_nonce text,
    presentation_checksum text
)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
DECLARE
    v_course_id uuid;
    v_assignment ple_data.assignment%ROWTYPE;
    v_student_record_id uuid;
    v_revision ple_data.assignment_revision%ROWTYPE;
    v_now timestamp with time zone := pg_catalog.clock_timestamp();
BEGIN
    SELECT assignment.* INTO v_assignment
      FROM ple_data.course_instance AS course
      JOIN ple_data.assignment AS assignment ON assignment.course_id = course.course_id
     WHERE course.reference_number = p_course_reference_number
       AND assignment.reference_number = p_assignment_reference_number;
    v_course_id := v_assignment.course_id;
    IF NOT FOUND THEN RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Assignment is unavailable'; END IF;
    SELECT student_record.student_record_id INTO v_student_record_id
      FROM ple_data.student_record AS student_record
     WHERE student_record.course_id = v_course_id
       AND student_record.student_account_id = ple_api.current_session_account_id();
    IF NOT FOUND OR NOT ple_api.current_session_account_owns_student_record(v_course_id, v_student_record_id) THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Assignment is unavailable';
    END IF;
    SELECT revision.* INTO v_revision FROM ple_data.assignment_revision AS revision
     WHERE revision.assignment_revision_id = v_assignment.released_assignment_revision_id;
    IF v_assignment.assignment_status <> 'released' OR NOT FOUND
       OR (v_revision.available_at IS NOT NULL AND v_now < v_revision.available_at)
       OR (v_revision.closes_at IS NOT NULL AND v_now >= v_revision.closes_at)
       OR (v_revision.due_at IS NOT NULL AND v_now > v_revision.due_at AND v_revision.late_work_rule = 'reject') THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Assignment cannot start at this time';
    END IF;
    RETURN QUERY
    SELECT source.*, existing.assignment_attempt_id IS NOT NULL,
           attempt.question_seed, binding.presentation_nonce, binding.presentation_checksum
      FROM ple_private.live_demo_native_ple_issuance_source_binding(
          v_assignment.released_assignment_revision_id
      ) AS source
      LEFT JOIN LATERAL (
          SELECT assignment_attempt_id
            FROM ple_private.assignment_attempt
           WHERE student_record_id = v_student_record_id
             AND assignment_id = v_assignment.assignment_id
             AND completed_at IS NULL
           ORDER BY attempt_number DESC LIMIT 1
      ) AS existing ON true
      LEFT JOIN ple_private.issued_question AS issued
        ON issued.assignment_attempt_id = existing.assignment_attempt_id
       AND issued.assignment_entry_id = source.assignment_entry_id
       AND issued.issued_position = source.issued_position
      LEFT JOIN ple_private.question_attempt AS attempt
        ON attempt.issued_question_id = issued.issued_question_id
      LEFT JOIN ple_private.question_attempt_presentation_binding AS binding
        ON binding.question_attempt_id = attempt.question_attempt_id
     ORDER BY source.issued_position;
END
$$;

-- ASVS 2.3.1/2.3.3: preparation is not authority to issue.  This transaction
-- invokes the established locked start procedure, then stores every native
-- Question Attempt and its binding before it returns.  Its resume path reads
-- the established private facts and never mints or mutates them.
CREATE FUNCTION ple_api.start_live_demo_native_ple_assignment(
    p_course_reference_number bigint,
    p_assignment_reference_number bigint,
    p_presentations jsonb
)
RETURNS TABLE (
    attempt_number integer,
    resumed boolean,
    assignment_title text,
    assignment_instructions text,
    question_id text,
    revision_number integer,
    issued_position integer,
    question_seed numeric,
    presentation_nonce text,
    presentation_checksum text
)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
DECLARE
    v_assignment ple_data.assignment%ROWTYPE;
    v_revision ple_data.assignment_revision%ROWTYPE;
    v_student_record_id uuid;
    v_started record;
    v_now timestamp with time zone;
BEGIN
    IF jsonb_typeof(p_presentations) <> 'array' THEN
        RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Native PLE issuance requires a presentation array';
    END IF;
    SELECT assignment.* INTO v_assignment
      FROM ple_data.course_instance AS course JOIN ple_data.assignment AS assignment ON assignment.course_id = course.course_id
     WHERE course.reference_number = p_course_reference_number AND assignment.reference_number = p_assignment_reference_number;
    IF NOT FOUND THEN RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Assignment is unavailable'; END IF;
    SELECT student_record.student_record_id INTO v_student_record_id FROM ple_data.student_record AS student_record
     WHERE student_record.course_id = v_assignment.course_id AND student_record.student_account_id = ple_api.current_session_account_id();
    IF NOT FOUND OR NOT ple_api.current_session_account_owns_student_record(v_assignment.course_id, v_student_record_id) THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Assignment is unavailable';
    END IF;
    -- ASVS 2.3.1/2.3.3: serialize against the course-membership transition
    -- lock, then recheck the exact Student Record before durable issue.
    PERFORM pg_catalog.pg_advisory_xact_lock(pg_catalog.hashtextextended(
        pg_catalog.format('%s:%s:%s', v_assignment.course_id,
            ple_api.current_session_account_id(), 'student'), 0
    ));
    IF NOT ple_api.current_session_account_owns_student_record(
        v_assignment.course_id, v_student_record_id
    ) THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Assignment is unavailable';
    END IF;
    -- Recheck under the same assignment-row lock as durable creation.  The
    -- generic start checks availability/closes; this M11 boundary also owns
    -- the released due/reject rule and must not bridge preparation to issue.
    SELECT assignment.* INTO v_assignment FROM ple_data.assignment AS assignment
     WHERE assignment.assignment_id = v_assignment.assignment_id FOR UPDATE;
    SELECT revision.* INTO v_revision FROM ple_data.assignment_revision AS revision
     WHERE revision.assignment_revision_id = v_assignment.released_assignment_revision_id;
    v_now := pg_catalog.clock_timestamp();
    IF v_assignment.assignment_status <> 'released' OR NOT FOUND
       OR (v_revision.available_at IS NOT NULL AND v_now < v_revision.available_at)
       OR (v_revision.closes_at IS NOT NULL AND v_now >= v_revision.closes_at)
       OR (v_revision.due_at IS NOT NULL AND v_now > v_revision.due_at
           AND v_revision.late_work_rule = 'reject') THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Assignment cannot start at this time';
    END IF;
    -- The supplied evidence must cover each exact immutable fixed entry once,
    -- including its released display position.  The shared start primitive
    -- validates identity pins but intentionally has no position field in its
    -- set comparison, so this M11 boundary closes that gap before insertion.
    IF NOT EXISTS (
        SELECT 1 FROM ple_private.assignment_attempt
         WHERE student_record_id = v_student_record_id
           AND assignment_id = v_assignment.assignment_id AND completed_at IS NULL
    ) AND (
        jsonb_array_length(p_presentations) = 0 OR EXISTS (
            WITH supplied AS (
                SELECT item.assignment_entry_id, item.issued_position, item.question_id,
                       item.revision_number
                  FROM jsonb_to_recordset(p_presentations) AS item(
                    issued_question_id uuid, assignment_entry_id uuid, issued_position integer,
                    question_id text, revision_number integer, question_seed numeric,
                    parameter_hash text, reproduction_details jsonb, presentation_nonce text,
                    presentation_checksum text)
            ), expected AS (
                SELECT fixed.assignment_entry_id, entry.assignment_content_entry_index AS issued_position,
                       fixed.question_id, fixed.revision_number
                  FROM ple_data.assignment_revision_entry AS entry
                  JOIN ple_data.assignment_revision_fixed_question AS fixed
                    ON fixed.assignment_revision_id = entry.assignment_revision_id
                   AND fixed.assignment_entry_id = entry.assignment_entry_id
                 WHERE entry.assignment_revision_id = v_assignment.released_assignment_revision_id
            )
            SELECT 1 FROM (
                (SELECT * FROM expected EXCEPT ALL SELECT * FROM supplied)
                UNION ALL (SELECT * FROM supplied EXCEPT ALL SELECT * FROM expected)
            ) AS difference
        )
    ) THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Native PLE presentations must cover each exact Released fixed entry and position once';
    END IF;
    SELECT * INTO v_started FROM ple_private.start_assignment_attempt(
        pg_catalog.gen_random_uuid(), v_student_record_id, v_assignment.assignment_id, '[]'::jsonb,
        (SELECT jsonb_agg(jsonb_build_object(
            'issued_question_id', item.issued_question_id, 'assignment_entry_id', item.assignment_entry_id,
            'issued_position', item.issued_position, 'question_id', item.question_id,
            'revision_number', item.revision_number, 'question_pool_selection_id', NULL,
            'question_pool_item_id', NULL) ORDER BY item.issued_position)
         FROM jsonb_to_recordset(p_presentations) AS item(
            issued_question_id uuid, assignment_entry_id uuid, issued_position integer,
            question_id text, revision_number integer, question_seed numeric, parameter_hash text,
            reproduction_details jsonb, presentation_nonce text, presentation_checksum text))
    );
    IF NOT v_started.resumed THEN
        INSERT INTO ple_private.question_attempt (
            question_attempt_id, issued_question_id, question_seed, generated_parameter_sha256,
            issued_at, deadline_at, question_attempt_state, reproduction_details
        )
        SELECT pg_catalog.gen_random_uuid(), issued.issued_question_id, item.question_seed,
               item.parameter_hash, pg_catalog.clock_timestamp(), NULL, 'open', item.reproduction_details
          FROM jsonb_to_recordset(p_presentations) AS item(
            issued_question_id uuid, assignment_entry_id uuid, issued_position integer,
            question_id text, revision_number integer, question_seed numeric, parameter_hash text,
            reproduction_details jsonb, presentation_nonce text, presentation_checksum text)
          JOIN ple_private.issued_question AS issued ON issued.issued_question_id = item.issued_question_id
         WHERE issued.assignment_attempt_id = v_started.assignment_attempt_id;
        INSERT INTO ple_private.question_attempt_presentation_binding (
            question_attempt_id, descriptor_version, presentation_nonce, presentation_checksum
        )
        SELECT attempt.question_attempt_id, 1, item.presentation_nonce, item.presentation_checksum
          FROM jsonb_to_recordset(p_presentations) AS item(
            issued_question_id uuid, assignment_entry_id uuid, issued_position integer,
            question_id text, revision_number integer, question_seed numeric, parameter_hash text,
            reproduction_details jsonb, presentation_nonce text, presentation_checksum text)
          JOIN ple_private.question_attempt AS attempt ON attempt.issued_question_id = item.issued_question_id;
    END IF;
    RETURN QUERY
    SELECT v_started.attempt_number, v_started.resumed, revision.assignment_title, revision.assignment_instructions,
           issued.question_id, issued.revision_number, issued.issued_position, attempt.question_seed,
           binding.presentation_nonce, binding.presentation_checksum
      FROM ple_private.issued_question AS issued
      JOIN ple_private.question_attempt AS attempt ON attempt.issued_question_id = issued.issued_question_id
      JOIN ple_private.question_attempt_presentation_binding AS binding ON binding.question_attempt_id = attempt.question_attempt_id
      JOIN ple_data.assignment_revision AS revision ON revision.assignment_revision_id = v_assignment.released_assignment_revision_id
     WHERE issued.assignment_attempt_id = v_started.assignment_attempt_id ORDER BY issued.issued_position;
END
$$;

-- ASVS 8.2.1/8.2.2: only ple_app may invoke these pinned SECURITY DEFINER
-- procedures; no caller receives source bindings or direct private-table access.
REVOKE ALL PRIVILEGES ON FUNCTION ple_api.prepare_live_demo_native_ple_issuance(bigint, bigint),
    ple_api.start_live_demo_native_ple_assignment(bigint, bigint, jsonb) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_api.prepare_live_demo_native_ple_issuance(bigint, bigint),
    ple_api.start_live_demo_native_ple_assignment(bigint, bigint, jsonb) TO ple_app;
RESET ROLE;
