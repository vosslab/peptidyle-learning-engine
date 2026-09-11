-- Apply current-Assignment retime behavior to native issuance. Released
-- revision content remains fixed; an active Attempt retains its captured
-- delivery title and due instant, including an explicit no-deadline value.
DO $$
BEGIN
    IF current_user <> 'ple_migrator' THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'migration 2026091024 must run as ple_migrator';
    END IF;
END
$$;

SET LOCAL ROLE ple_api_owner;

CREATE OR REPLACE FUNCTION ple_api.prepare_live_demo_native_ple_issuance(
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
    v_active_attempt ple_private.assignment_attempt%ROWTYPE;
    v_due_at timestamp with time zone;
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
    SELECT attempt.* INTO v_active_attempt
      FROM ple_private.assignment_attempt AS attempt
     WHERE attempt.student_record_id = v_student_record_id
       AND attempt.assignment_id = v_assignment.assignment_id
       AND attempt.completed_at IS NULL
     ORDER BY attempt.attempt_number DESC
     LIMIT 1;
    SELECT revision.* INTO v_revision FROM ple_data.assignment_revision AS revision
     WHERE revision.assignment_revision_id = CASE
         WHEN v_active_attempt.assignment_attempt_id IS NULL
             THEN v_assignment.released_assignment_revision_id
         ELSE v_active_attempt.assignment_revision_id
     END;
    IF v_assignment.assignment_status <> 'released' OR NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Assignment cannot start at this time';
    END IF;
    v_due_at := CASE
        WHEN v_active_attempt.assignment_attempt_id IS NULL THEN v_assignment.due_at
        WHEN v_active_attempt.delivery_assignment_title IS NOT NULL
            THEN v_active_attempt.delivery_due_at
        ELSE v_revision.due_at
    END;
    IF (v_revision.available_at IS NOT NULL AND v_now < v_revision.available_at)
       OR (v_revision.closes_at IS NOT NULL AND v_now >= v_revision.closes_at)
       OR (v_due_at IS NOT NULL AND v_now > v_due_at AND v_revision.late_work_rule = 'reject') THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Assignment cannot start at this time';
    END IF;
    RETURN QUERY
    SELECT source.assignment_entry_id,
           COALESCE(issued.issued_position, source.issued_position),
           source.question_id, source.revision_number, source.source_object_id,
           source.source_object_address, source.source_object_checksum,
           existing.assignment_attempt_id IS NOT NULL,
           attempt.question_seed, binding.presentation_nonce, binding.presentation_checksum
      FROM ple_private.live_demo_native_ple_issuance_source_binding(
          v_revision.assignment_revision_id
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
       AND issued.question_id = source.question_id
       AND issued.revision_number = source.revision_number
      LEFT JOIN ple_private.question_attempt AS attempt
        ON attempt.issued_question_id = issued.issued_question_id
      LEFT JOIN ple_private.question_attempt_presentation_binding AS binding
        ON binding.question_attempt_id = attempt.question_attempt_id
     ORDER BY COALESCE(issued.issued_position, source.issued_position);
END
$$;

CREATE OR REPLACE FUNCTION ple_api.start_live_demo_native_ple_assignment(
    p_course_reference_number bigint,
    p_assignment_reference_number bigint,
    p_presentations jsonb
)
RETURNS TABLE (
    attempt_number integer,
    resumed boolean,
    assignment_title text,
    assignment_instructions text,
    assignment_entry_id uuid,
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
    v_active_attempt ple_private.assignment_attempt%ROWTYPE;
    v_started record;
    v_due_at timestamp with time zone;
    v_now timestamp with time zone;
BEGIN
    IF jsonb_typeof(p_presentations) <> 'array' THEN
        RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Native PLE issuance requires a presentation array';
    END IF;
    SELECT assignment.* INTO v_assignment
      FROM ple_data.course_instance AS course
      JOIN ple_data.assignment AS assignment ON assignment.course_id = course.course_id
     WHERE course.reference_number = p_course_reference_number
       AND assignment.reference_number = p_assignment_reference_number;
    IF NOT FOUND THEN RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Assignment is unavailable'; END IF;
    SELECT student_record.student_record_id INTO v_student_record_id
      FROM ple_data.student_record AS student_record
     WHERE student_record.course_id = v_assignment.course_id
       AND student_record.student_account_id = ple_api.current_session_account_id();
    IF NOT FOUND OR NOT ple_api.current_session_account_owns_student_record(v_assignment.course_id, v_student_record_id) THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Assignment is unavailable';
    END IF;
    PERFORM pg_catalog.pg_advisory_xact_lock(pg_catalog.hashtextextended(
        pg_catalog.format('%s:%s:%s', v_assignment.course_id,
            ple_api.current_session_account_id(), 'student'), 0
    ));
    IF NOT ple_api.current_session_account_owns_student_record(v_assignment.course_id, v_student_record_id) THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Assignment is unavailable';
    END IF;
    SELECT assignment.* INTO v_assignment FROM ple_data.assignment AS assignment
     WHERE assignment.assignment_id = v_assignment.assignment_id FOR UPDATE;
    SELECT attempt.* INTO v_active_attempt
      FROM ple_private.assignment_attempt AS attempt
     WHERE attempt.student_record_id = v_student_record_id
       AND attempt.assignment_id = v_assignment.assignment_id
       AND attempt.completed_at IS NULL
     ORDER BY attempt.attempt_number DESC
     LIMIT 1;
    SELECT revision.* INTO v_revision FROM ple_data.assignment_revision AS revision
     WHERE revision.assignment_revision_id = CASE
         WHEN v_active_attempt.assignment_attempt_id IS NULL
             THEN v_assignment.released_assignment_revision_id
         ELSE v_active_attempt.assignment_revision_id
     END;
    IF v_assignment.assignment_status <> 'released' OR NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Assignment cannot start at this time';
    END IF;
    v_now := pg_catalog.clock_timestamp();
    v_due_at := CASE
        WHEN v_active_attempt.assignment_attempt_id IS NULL THEN v_assignment.due_at
        WHEN v_active_attempt.delivery_assignment_title IS NOT NULL
            THEN v_active_attempt.delivery_due_at
        ELSE v_revision.due_at
    END;
    IF (v_revision.available_at IS NOT NULL AND v_now < v_revision.available_at)
       OR (v_revision.closes_at IS NOT NULL AND v_now >= v_revision.closes_at)
       OR (v_due_at IS NOT NULL AND v_now > v_due_at
           AND v_revision.late_work_rule = 'reject') THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Assignment cannot start at this time';
    END IF;
    IF NOT EXISTS (
        SELECT 1 FROM ple_private.assignment_attempt
         WHERE student_record_id = v_student_record_id
           AND assignment_id = v_assignment.assignment_id AND completed_at IS NULL
    ) AND (
        jsonb_array_length(p_presentations) = 0 OR EXISTS (
            WITH supplied AS (
                SELECT item.assignment_entry_id, item.question_id, item.revision_number
                  FROM jsonb_to_recordset(p_presentations) AS item(
                    issued_question_id uuid, assignment_entry_id uuid, issued_position integer,
                    question_id text, revision_number integer, question_seed numeric,
                    parameter_hash text, reproduction_details jsonb, presentation_nonce text,
                    presentation_checksum text)
            ), expected AS (
                SELECT fixed.assignment_entry_id, fixed.question_id, fixed.revision_number
                  FROM ple_data.assignment_revision_fixed_question AS fixed
                 WHERE fixed.assignment_revision_id = v_revision.assignment_revision_id
            )
            SELECT 1 FROM (
                (SELECT * FROM expected EXCEPT ALL SELECT * FROM supplied)
                UNION ALL (SELECT * FROM supplied EXCEPT ALL SELECT * FROM expected)
            ) AS difference
        )
    ) THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Native PLE presentations must cover each exact Released fixed entry once';
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
    SELECT v_started.attempt_number, v_started.resumed,
           CASE WHEN assignment_attempt.delivery_assignment_title IS NOT NULL
                THEN assignment_attempt.delivery_assignment_title ELSE revision.assignment_title END,
           revision.assignment_instructions, issued.assignment_entry_id, issued.question_id,
           issued.revision_number, issued.issued_position, attempt.question_seed,
           binding.presentation_nonce, binding.presentation_checksum
      FROM ple_private.issued_question AS issued
      JOIN ple_private.question_attempt AS attempt ON attempt.issued_question_id = issued.issued_question_id
      JOIN ple_private.question_attempt_presentation_binding AS binding ON binding.question_attempt_id = attempt.question_attempt_id
      JOIN ple_private.assignment_attempt AS assignment_attempt
        ON assignment_attempt.assignment_attempt_id = v_started.assignment_attempt_id
      JOIN ple_data.assignment_revision AS revision
        ON revision.assignment_revision_id = assignment_attempt.assignment_revision_id
     WHERE issued.assignment_attempt_id = v_started.assignment_attempt_id
     ORDER BY issued.issued_position;
END
$$;

REVOKE ALL PRIVILEGES ON FUNCTION ple_api.prepare_live_demo_native_ple_issuance(bigint, bigint),
    ple_api.start_live_demo_native_ple_assignment(bigint, bigint, jsonb) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_api.prepare_live_demo_native_ple_issuance(bigint, bigint),
    ple_api.start_live_demo_native_ple_assignment(bigint, bigint, jsonb) TO ple_app;

RESET ROLE;
