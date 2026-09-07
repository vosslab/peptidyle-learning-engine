-- M13 native PLE Question Submission acceptance.
--
-- One protected transaction accepts the already format-validated Student
-- Response for one exact issued native PLE Question Attempt, records its
-- pending grading state, and prepares the typed grading Job.  It returns only
-- the Student-visible grading-progress fact.

-- The private owner needs this migration-scoped capability to create the
-- externally named procedure under its final definer identity.  The capability
-- is revoked immediately after the procedure is closed.
SET LOCAL ROLE ple_api_owner;
GRANT CREATE ON SCHEMA ple_api TO ple_private_owner;
RESET ROLE;

SET LOCAL ROLE ple_private_owner;
-- Forced RLS remains in effect for the definer.  These policies exist only
-- for this private procedure; API and browser capability roles receive no
-- table grants or policies for the accepted-response write path.
CREATE POLICY question_attempt_private_owner_native_ple_submission_update
    ON ple_private.question_attempt
    FOR UPDATE TO ple_private_owner USING (true) WITH CHECK (true);
CREATE POLICY question_submission_private_owner_native_ple_submission_insert
    ON ple_private.question_submission
    FOR INSERT TO ple_private_owner WITH CHECK (true);
CREATE POLICY question_submission_grading_private_owner_native_ple_submission_insert
    ON ple_private.question_submission_grading
    FOR INSERT TO ple_private_owner WITH CHECK (true);
CREATE POLICY job_private_owner_native_ple_submission_insert
    ON ple_private.job
    FOR INSERT TO ple_private_owner WITH CHECK (true);

-- The API process, but never ple_app's browser caller, needs the immutable
-- source facts to reproduce the response format before it accepts a response.
-- The procedure repeats all Student/course/assignment/nonce authorization and
-- returns no answer, submission, or grading data. ASVS 8.2.1/8.2.2/8.3.1.
CREATE FUNCTION ple_api.resolve_live_demo_native_ple_submission(
    p_course_reference_number bigint,
    p_assignment_reference_number bigint,
    p_presentation_nonce text
)
RETURNS TABLE (
    question_attempt_id uuid,
    question_id text,
    revision_number integer,
    source_object_id uuid,
    source_object_checksum text,
    question_seed numeric,
    presentation_nonce text,
    presentation_checksum text
)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
BEGIN
    IF p_presentation_nonce IS NULL
       OR p_presentation_nonce !~ '^[0-9a-f]{32}$' THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Question Submission is unavailable';
    END IF;

    RETURN QUERY
    SELECT attempt.question_attempt_id,
           issued.question_id,
           issued.revision_number,
           source_binding.source_object_id,
           source_binding.source_object_checksum,
           attempt.question_seed,
           presentation.presentation_nonce,
           presentation.presentation_checksum
      FROM ple_data.course_instance AS course
      JOIN ple_data.assignment AS assignment
        ON assignment.course_id = course.course_id
      JOIN ple_private.assignment_attempt AS assignment_attempt
        ON assignment_attempt.assignment_id = assignment.assignment_id
       AND assignment_attempt.completed_at IS NULL
      JOIN ple_data.student_record AS student_record
        ON student_record.student_record_id = assignment_attempt.student_record_id
      JOIN ple_private.issued_question AS issued
        ON issued.assignment_attempt_id = assignment_attempt.assignment_attempt_id
      JOIN ple_private.question_attempt AS attempt
        ON attempt.issued_question_id = issued.issued_question_id
       AND attempt.question_attempt_state = 'open'
      JOIN ple_private.question_attempt_presentation_binding AS presentation
        ON presentation.question_attempt_id = attempt.question_attempt_id
      JOIN ple_private.question_revision_source_binding AS source_binding
        ON source_binding.question_id = issued.question_id
       AND source_binding.revision_number = issued.revision_number
     WHERE course.reference_number = p_course_reference_number
       AND assignment.reference_number = p_assignment_reference_number
       AND student_record.course_id = assignment.course_id
       AND student_record.student_account_id = ple_api.current_session_account_id()
       AND ple_api.current_session_account_owns_student_record(
           assignment.course_id, student_record.student_record_id
       )
       AND presentation.presentation_nonce = p_presentation_nonce
       AND source_binding.backend = 'ple'
       AND source_binding.question_format = 'pleQuestionJson';
END
$$;

CREATE FUNCTION ple_api.accept_live_demo_native_ple_submission(
    p_question_attempt_id uuid,
    p_student_response jsonb,
    p_submission_id uuid,
    p_question_submission_grading_id uuid,
    p_job_id uuid
)
RETURNS text
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
DECLARE
    v_question_attempt ple_private.question_attempt%ROWTYPE;
    v_now timestamp with time zone;
BEGIN
    IF p_question_attempt_id IS NULL OR p_submission_id IS NULL
       OR p_question_submission_grading_id IS NULL OR p_job_id IS NULL
       OR pg_catalog.jsonb_typeof(p_student_response) IS DISTINCT FROM 'object' THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Question Submission acceptance facts are invalid';
    END IF;

    -- ASVS 2.3.1/2.3.3: lock the exact Question Attempt before rechecking the
    -- current Student, Course, Assignment, native-PLE source, issued
    -- presentation, and open lifecycle state.  No caller-selected Question,
    -- correctness, or reproduction identity participates in this authority.
    SELECT question_attempt.* INTO v_question_attempt
      FROM ple_private.question_attempt AS question_attempt
      JOIN ple_private.issued_question AS issued
        ON issued.issued_question_id = question_attempt.issued_question_id
      JOIN ple_private.assignment_attempt AS assignment_attempt
        ON assignment_attempt.assignment_attempt_id = issued.assignment_attempt_id
      JOIN ple_data.assignment AS assignment
        ON assignment.assignment_id = assignment_attempt.assignment_id
      JOIN ple_data.student_record AS student_record
        ON student_record.student_record_id = assignment_attempt.student_record_id
      JOIN ple_private.question_attempt_presentation_binding AS presentation
        ON presentation.question_attempt_id = question_attempt.question_attempt_id
      JOIN ple_private.question_revision_source_binding AS source_binding
        ON source_binding.question_id = issued.question_id
       AND source_binding.revision_number = issued.revision_number
     WHERE question_attempt.question_attempt_id = p_question_attempt_id
       AND assignment_attempt.completed_at IS NULL
       AND student_record.course_id = assignment.course_id
       AND student_record.student_account_id = ple_api.current_session_account_id()
       AND ple_api.current_session_account_owns_student_record(
           assignment.course_id, student_record.student_record_id
       )
       AND source_binding.backend = 'ple'
       AND source_binding.question_format = 'pleQuestionJson'
     FOR UPDATE OF question_attempt;
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Question Submission is unavailable';
    END IF;

    v_now := pg_catalog.clock_timestamp();
    IF v_question_attempt.question_attempt_state <> 'open'
       OR (v_question_attempt.deadline_at IS NOT NULL
           AND v_question_attempt.deadline_at <= v_now)
       OR EXISTS (
           SELECT 1 FROM ple_private.question_submission AS submission
            WHERE submission.question_attempt_id = v_question_attempt.question_attempt_id
       ) THEN
        RAISE EXCEPTION USING ERRCODE = '55000',
            MESSAGE = 'Question Submission cannot be accepted';
    END IF;

    INSERT INTO ple_private.question_submission (
        submission_id, question_attempt_id, submitted_at, student_response
    ) VALUES (
        p_submission_id, v_question_attempt.question_attempt_id, v_now, p_student_response
    );
    UPDATE ple_private.question_attempt
       SET question_attempt_state = 'submission_accepted'
     WHERE question_attempt_id = v_question_attempt.question_attempt_id
       AND question_attempt_state = 'open';
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '55000',
            MESSAGE = 'Question Submission cannot be accepted';
    END IF;
    INSERT INTO ple_private.job (
        job_id, job_kind, job_target_kind, question_submission_id, generation,
        payload, state, available_at, max_attempts, created_at
    ) VALUES (
        p_job_id, 'grade_accepted_submission', 'question_submission', p_submission_id, 1,
        '{}'::jsonb, 'ready', v_now, 3, v_now
    );
    INSERT INTO ple_private.question_submission_grading (
        question_submission_grading_id, submission_id, job_id, grading_state, created_at
    ) VALUES (
        p_question_submission_grading_id, p_submission_id, p_job_id, 'pending', v_now
    );
    RETURN 'pending';
END
$$;

-- ASVS 8.2.1/8.2.2/8.3.1: ple_app may invoke this narrow procedure only.  It
-- has no direct privilege on accepted Student Responses, grading records, or
-- Jobs, and the procedure returns no answer or correctness-bearing record.
REVOKE ALL PRIVILEGES ON FUNCTION ple_api.accept_live_demo_native_ple_submission(
    uuid, jsonb, uuid, uuid, uuid
) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_api.accept_live_demo_native_ple_submission(
    uuid, jsonb, uuid, uuid, uuid
) TO ple_app;
REVOKE ALL PRIVILEGES ON FUNCTION ple_api.resolve_live_demo_native_ple_submission(
    bigint, bigint, text
) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_api.resolve_live_demo_native_ple_submission(
    bigint, bigint, text
) TO ple_app;
RESET ROLE;

SET LOCAL ROLE ple_api_owner;
REVOKE CREATE ON SCHEMA ple_api FROM ple_private_owner;
RESET ROLE;
