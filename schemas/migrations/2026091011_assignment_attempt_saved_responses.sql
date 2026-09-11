-- Persists Student working responses and whole Assignment Attempt submission.
--
-- Working responses remain private mutable state until one explicit Assignment
-- Attempt submission turns them into immutable Question Submission evidence.

SET LOCAL ROLE ple_private_owner;

CREATE TABLE ple_private.assignment_attempt_saved_response (
    question_attempt_id uuid PRIMARY KEY
        REFERENCES ple_private.question_attempt (question_attempt_id),
    student_response jsonb NOT NULL
        CONSTRAINT assignment_attempt_saved_response_is_object
        CHECK (pg_catalog.jsonb_typeof(student_response) = 'object'),
    saved_at timestamp with time zone NOT NULL
);

ALTER TABLE ple_private.assignment_attempt_saved_response ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.assignment_attempt_saved_response FORCE ROW LEVEL SECURITY;
CREATE POLICY assignment_attempt_saved_response_private_owner_access
    ON ple_private.assignment_attempt_saved_response
    FOR ALL TO ple_private_owner USING (true) WITH CHECK (true);
CREATE POLICY assignment_submission_private_owner_access
    ON ple_private.assignment_submission
    FOR ALL TO ple_private_owner USING (true) WITH CHECK (true);
REVOKE ALL PRIVILEGES ON TABLE ple_private.assignment_attempt_saved_response FROM PUBLIC;
COMMENT ON TABLE ple_private.assignment_attempt_saved_response IS
    'Private current Student Response saved while an Assignment Attempt remains active.';

-- Assignment Attempt submission, rather than grading completion, is the
-- terminal Student action.  Grading continues asynchronously after that
-- durable submission boundary.
DROP TRIGGER grading_result_completes_assignment_attempt
    ON ple_private.grading_result;

RESET ROLE;

SET LOCAL ROLE ple_api_owner;
GRANT CREATE ON SCHEMA ple_api TO ple_private_owner;
RESET ROLE;

SET LOCAL ROLE ple_private_owner;

CREATE FUNCTION ple_api.save_student_assignment_attempt_response(
    p_assignment_attempt_reference_number bigint,
    p_position integer,
    p_student_response jsonb
) RETURNS TABLE (
    assignment_attempt_reference_number bigint,
    issued_position integer,
    response_state text,
    saved_at timestamp with time zone
)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
DECLARE
    v_attempt ple_private.assignment_attempt%ROWTYPE;
    v_question_attempt ple_private.question_attempt%ROWTYPE;
    v_attempt_time_limit_seconds integer;
    v_now timestamp with time zone;
BEGIN
    IF p_assignment_attempt_reference_number NOT BETWEEN 1 AND 2147483647
       OR p_position NOT BETWEEN 1 AND 2147483647
       OR pg_catalog.jsonb_typeof(p_student_response) IS DISTINCT FROM 'object' THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Student response save facts are invalid';
    END IF;

    -- Lock the owned Assignment Attempt before its selected Question Attempt.
    SELECT attempt.* INTO v_attempt
      FROM ple_private.assignment_attempt AS attempt
      JOIN ple_data.student_record AS student
        ON student.student_record_id = attempt.student_record_id
      JOIN ple_data.assignment AS assignment
        ON assignment.assignment_id = attempt.assignment_id
      JOIN ple_private.account AS account
        ON account.account_id = student.student_account_id
     WHERE attempt.reference_number = p_assignment_attempt_reference_number
       AND attempt.completed_at IS NULL
       AND account.product_role = 'student'
       AND student.course_id = assignment.course_id
       AND student.student_account_id = ple_api.current_session_account_id()
       AND ple_api.current_session_account_owns_student_record(
             assignment.course_id, student.student_record_id
       )
       AND EXISTS (
           SELECT 1 FROM ple_data.course_membership AS membership
            WHERE membership.course_id = assignment.course_id
              AND membership.account_id = student.student_account_id
              AND membership.role = 'student'
              AND ple_data.course_membership_is_active(membership.membership_id)
       )
     FOR UPDATE OF attempt;
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Student response save is unavailable';
    END IF;

    SELECT revision.assignment_attempt_time_limit_seconds
      INTO v_attempt_time_limit_seconds
      FROM ple_data.assignment_revision AS revision
     WHERE revision.assignment_id = v_attempt.assignment_id
       AND revision.assignment_revision_id = v_attempt.assignment_revision_id;

    SELECT question_attempt.* INTO v_question_attempt
      FROM ple_private.issued_question AS issued
      JOIN ple_private.question_attempt AS question_attempt
        ON question_attempt.issued_question_id = issued.issued_question_id
     WHERE issued.assignment_attempt_id = v_attempt.assignment_attempt_id
       AND issued.issued_position = p_position - 1
     FOR UPDATE OF question_attempt;
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Student response save is unavailable';
    END IF;

    v_now := pg_catalog.clock_timestamp();
    IF v_question_attempt.question_attempt_state <> 'open'
       OR (v_attempt_time_limit_seconds IS NOT NULL
           AND v_attempt.started_at
                 + v_attempt_time_limit_seconds * interval '1 second' <= v_now)
       OR (v_question_attempt.deadline_at IS NOT NULL
           AND v_question_attempt.deadline_at <= v_now) THEN
        RAISE EXCEPTION USING ERRCODE = '55000',
            MESSAGE = 'Student response cannot be saved';
    END IF;

    INSERT INTO ple_private.assignment_attempt_saved_response (
        question_attempt_id, student_response, saved_at
    ) VALUES (
        v_question_attempt.question_attempt_id, p_student_response, v_now
    ) ON CONFLICT (question_attempt_id) DO UPDATE
        SET student_response = EXCLUDED.student_response,
            saved_at = EXCLUDED.saved_at
      WHERE ple_private.assignment_attempt_saved_response.student_response
            IS DISTINCT FROM EXCLUDED.student_response;

    RETURN QUERY
    SELECT v_attempt.reference_number, p_position, 'saved'::text,
           COALESCE(
               (SELECT saved_response.saved_at
                  FROM ple_private.assignment_attempt_saved_response AS saved_response
                 WHERE saved_response.question_attempt_id =
                       v_question_attempt.question_attempt_id),
               v_now
           );
END
$$;

CREATE FUNCTION ple_api.finalize_student_assignment_attempt(
    p_assignment_attempt_reference_number bigint
) RETURNS TABLE (submission_state text, missing_positions integer[])
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
DECLARE
    v_attempt ple_private.assignment_attempt%ROWTYPE;
    v_attempt_time_limit_seconds integer;
    v_now timestamp with time zone;
    v_missing_positions integer[];
BEGIN
    IF p_assignment_attempt_reference_number NOT BETWEEN 1 AND 2147483647 THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Assignment Attempt submission facts are invalid';
    END IF;

    -- This first lock serializes saves and finalization for the one owned
    -- Assignment Attempt.  Question Attempts are locked below in issue order.
    SELECT attempt.* INTO v_attempt
      FROM ple_private.assignment_attempt AS attempt
      JOIN ple_data.student_record AS student
        ON student.student_record_id = attempt.student_record_id
      JOIN ple_data.assignment AS assignment
        ON assignment.assignment_id = attempt.assignment_id
         JOIN ple_private.account AS account
            ON account.account_id = student.student_account_id
         WHERE attempt.reference_number = p_assignment_attempt_reference_number
           AND (
               attempt.completed_at IS NULL
               OR EXISTS (
                   SELECT 1 FROM ple_private.assignment_submission AS submission
                    WHERE submission.assignment_attempt_id =
                          attempt.assignment_attempt_id
               )
           )
           AND account.product_role = 'student'
       AND student.course_id = assignment.course_id
       AND student.student_account_id = ple_api.current_session_account_id()
       AND ple_api.current_session_account_owns_student_record(
             assignment.course_id, student.student_record_id
       )
       AND EXISTS (
           SELECT 1 FROM ple_data.course_membership AS membership
            WHERE membership.course_id = assignment.course_id
              AND membership.account_id = student.student_account_id
              AND membership.role = 'student'
              AND ple_data.course_membership_is_active(membership.membership_id)
       )
     FOR UPDATE OF attempt;
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Assignment Attempt submission is unavailable';
    END IF;

    SELECT revision.assignment_attempt_time_limit_seconds
      INTO v_attempt_time_limit_seconds
      FROM ple_data.assignment_revision AS revision
     WHERE revision.assignment_id = v_attempt.assignment_id
       AND revision.assignment_revision_id = v_attempt.assignment_revision_id;

    IF EXISTS (
        SELECT 1 FROM ple_private.assignment_submission AS submission
         WHERE submission.assignment_attempt_id = v_attempt.assignment_attempt_id
    ) THEN
        RETURN QUERY SELECT 'submitted'::text, ARRAY[]::integer[];
        RETURN;
    END IF;

    v_now := pg_catalog.clock_timestamp();
    IF v_attempt_time_limit_seconds IS NOT NULL
       AND v_attempt.started_at
             + v_attempt_time_limit_seconds * interval '1 second' <= v_now THEN
        RAISE EXCEPTION USING ERRCODE = '55000',
            MESSAGE = 'Assignment Attempt submission time has expired';
    END IF;

    -- Acquire every issued Question Attempt in stable delivery order before
    -- checking state or copying saved working responses.
    PERFORM question_attempt.question_attempt_id
      FROM ple_private.issued_question AS issued
      JOIN ple_private.question_attempt AS question_attempt
        ON question_attempt.issued_question_id = issued.issued_question_id
     WHERE issued.assignment_attempt_id = v_attempt.assignment_attempt_id
     ORDER BY issued.issued_position
     FOR UPDATE OF question_attempt;

    SELECT array_agg(issued.issued_position + 1 ORDER BY issued.issued_position)
      INTO v_missing_positions
      FROM ple_private.issued_question AS issued
      JOIN ple_private.question_attempt AS question_attempt
        ON question_attempt.issued_question_id = issued.issued_question_id
      LEFT JOIN ple_private.assignment_attempt_saved_response AS saved_response
        ON saved_response.question_attempt_id = question_attempt.question_attempt_id
     WHERE issued.assignment_attempt_id = v_attempt.assignment_attempt_id
       AND (
           saved_response.question_attempt_id IS NULL
           OR question_attempt.question_attempt_state <> 'open'
           OR (question_attempt.deadline_at IS NOT NULL
               AND question_attempt.deadline_at <= v_now)
       );
    IF v_missing_positions IS NOT NULL THEN
        RETURN QUERY SELECT 'missing_responses'::text, v_missing_positions;
        RETURN;
    END IF;

    INSERT INTO ple_private.assignment_submission (
        assignment_submission_id, assignment_attempt_id, submitted_at,
        authorized_by_account_id, receipt
    ) VALUES (
        pg_catalog.gen_random_uuid(), v_attempt.assignment_attempt_id, v_now,
        ple_api.current_session_account_id(),
        jsonb_build_object('submissionBoundary', 'assignmentAttempt')
    );

    INSERT INTO ple_private.question_submission (
        submission_id, question_attempt_id, submitted_at, student_response
    )
    SELECT pg_catalog.gen_random_uuid(), question_attempt.question_attempt_id,
           v_now, saved_response.student_response
      FROM ple_private.issued_question AS issued
      JOIN ple_private.question_attempt AS question_attempt
        ON question_attempt.issued_question_id = issued.issued_question_id
      JOIN ple_private.assignment_attempt_saved_response AS saved_response
        ON saved_response.question_attempt_id = question_attempt.question_attempt_id
     WHERE issued.assignment_attempt_id = v_attempt.assignment_attempt_id
     ORDER BY issued.issued_position;

    UPDATE ple_private.question_attempt AS question_attempt
       SET question_attempt_state = 'submission_accepted'
      FROM ple_private.issued_question AS issued
     WHERE issued.issued_question_id = question_attempt.issued_question_id
       AND issued.assignment_attempt_id = v_attempt.assignment_attempt_id
       AND question_attempt.question_attempt_state = 'open';

    INSERT INTO ple_private.job (
        job_id, job_kind, job_target_kind, question_submission_id, generation,
        payload, state, available_at, max_attempts, created_at
    )
    SELECT pg_catalog.gen_random_uuid(), 'grade_accepted_submission',
           'question_submission', submission.submission_id, 1, '{}'::jsonb,
           'ready', v_now, 3, v_now
      FROM ple_private.question_submission AS submission
      JOIN ple_private.question_attempt AS question_attempt
        ON question_attempt.question_attempt_id = submission.question_attempt_id
      JOIN ple_private.issued_question AS issued
        ON issued.issued_question_id = question_attempt.issued_question_id
     WHERE issued.assignment_attempt_id = v_attempt.assignment_attempt_id
     ORDER BY issued.issued_position;

    INSERT INTO ple_private.question_submission_grading (
        question_submission_grading_id, submission_id, job_id, grading_state,
        created_at
    )
    SELECT pg_catalog.gen_random_uuid(), submission.submission_id, job.job_id,
           'pending', v_now
      FROM ple_private.question_submission AS submission
      JOIN ple_private.question_attempt AS question_attempt
        ON question_attempt.question_attempt_id = submission.question_attempt_id
      JOIN ple_private.issued_question AS issued
        ON issued.issued_question_id = question_attempt.issued_question_id
      JOIN ple_private.job AS job
        ON job.question_submission_id = submission.submission_id
       AND job.job_kind = 'grade_accepted_submission'
       AND job.job_target_kind = 'question_submission'
     WHERE issued.assignment_attempt_id = v_attempt.assignment_attempt_id
     ORDER BY issued.issued_position;

    UPDATE ple_private.assignment_attempt
       SET completed_at = GREATEST(started_at, v_now)
     WHERE assignment_attempt_id = v_attempt.assignment_attempt_id
       AND completed_at IS NULL;

    RETURN QUERY SELECT 'submitted'::text, ARRAY[]::integer[];
END
$$;

-- This is the narrowly self-owned working-response read used to restore a
-- Student's own active Question after navigation or reload.  It is separate
-- from the answer-free navigation projection and never traverses answer,
-- correctness, or grading evidence.
CREATE FUNCTION ple_api.read_student_assignment_attempt_saved_response(
    p_assignment_attempt_reference_number bigint,
    p_position integer
) RETURNS TABLE (
    issued_position integer,
    student_response jsonb,
    saved_at timestamp with time zone
)
LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
    SELECT issued.issued_position + 1, saved_response.student_response,
           saved_response.saved_at
      FROM ple_private.assignment_attempt AS attempt
      JOIN ple_data.student_record AS student
        ON student.student_record_id = attempt.student_record_id
      JOIN ple_data.assignment AS assignment
        ON assignment.assignment_id = attempt.assignment_id
      JOIN ple_private.account AS account
        ON account.account_id = student.student_account_id
      JOIN ple_private.issued_question AS issued
        ON issued.assignment_attempt_id = attempt.assignment_attempt_id
      JOIN ple_private.question_attempt AS question_attempt
        ON question_attempt.issued_question_id = issued.issued_question_id
      LEFT JOIN ple_private.assignment_attempt_saved_response AS saved_response
        ON saved_response.question_attempt_id = question_attempt.question_attempt_id
     WHERE p_assignment_attempt_reference_number BETWEEN 1 AND 2147483647
       AND p_position BETWEEN 1 AND 2147483647
       AND attempt.reference_number = p_assignment_attempt_reference_number
       AND attempt.completed_at IS NULL
       AND issued.issued_position = p_position - 1
       AND account.product_role = 'student'
       AND student.course_id = assignment.course_id
       AND student.student_account_id = ple_api.current_session_account_id()
       AND ple_api.current_session_account_owns_student_record(
             assignment.course_id, student.student_record_id
       )
       AND EXISTS (
           SELECT 1 FROM ple_data.course_membership AS membership
            WHERE membership.course_id = assignment.course_id
              AND membership.account_id = student.student_account_id
              AND membership.role = 'student'
              AND ple_data.course_membership_is_active(membership.membership_id)
       )
$$;

-- Preserve the existing answer-free projection while distinguishing mutable
-- saved work from immutable submissions.  A finalized owner may still read
-- its submitted position states; selected delivery remains active-only.
CREATE OR REPLACE FUNCTION ple_api.read_student_assignment_attempt_progress(
    p_assignment_attempt_reference_number bigint
) RETURNS TABLE (
    assignment_attempt_reference_number bigint, question_count integer,
    recommended_position integer, issued_position integer, response_state text
) LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
    WITH owned_attempt AS (
        SELECT attempt.assignment_attempt_id, attempt.reference_number,
               attempt.completed_at
          FROM ple_private.assignment_attempt AS attempt
          JOIN ple_data.student_record AS student
            ON student.student_record_id = attempt.student_record_id
          JOIN ple_data.assignment AS assignment
            ON assignment.assignment_id = attempt.assignment_id
         JOIN ple_private.account AS account
            ON account.account_id = student.student_account_id
         WHERE attempt.reference_number = p_assignment_attempt_reference_number
           AND (
               attempt.completed_at IS NULL
               OR EXISTS (
                   SELECT 1 FROM ple_private.assignment_submission AS submission
                    WHERE submission.assignment_attempt_id =
                          attempt.assignment_attempt_id
               )
           )
           AND account.product_role = 'student'
           AND student.course_id = assignment.course_id
           AND student.student_account_id = ple_api.current_session_account_id()
           AND ple_api.current_session_account_owns_student_record(
                 assignment.course_id, student.student_record_id
           )
           AND EXISTS (
               SELECT 1 FROM ple_data.course_membership AS membership
                WHERE membership.course_id = assignment.course_id
                  AND membership.account_id = student.student_account_id
                  AND membership.role = 'student'
                  AND ple_data.course_membership_is_active(membership.membership_id)
           )
    ), positions AS (
        SELECT owned.reference_number, issued.issued_position + 1 AS issued_position,
               CASE
                   WHEN submission.question_attempt_id IS NOT NULL THEN 'submitted'
                   WHEN saved_response.question_attempt_id IS NOT NULL THEN 'saved'
                   WHEN question_attempt.question_attempt_state = 'closed_at_deadline' THEN 'closed'
                   ELSE 'unanswered'
               END AS response_state
          FROM owned_attempt AS owned
          JOIN ple_private.issued_question AS issued
            ON issued.assignment_attempt_id = owned.assignment_attempt_id
          JOIN ple_private.question_attempt AS question_attempt
            ON question_attempt.issued_question_id = issued.issued_question_id
          LEFT JOIN ple_private.question_submission AS submission
            ON submission.question_attempt_id = question_attempt.question_attempt_id
          LEFT JOIN ple_private.assignment_attempt_saved_response AS saved_response
            ON saved_response.question_attempt_id = question_attempt.question_attempt_id
    )
    SELECT reference_number,
           count(*) OVER ()::integer,
           min(issued_position) FILTER (WHERE response_state = 'unanswered') OVER (),
           issued_position, response_state
      FROM positions
     ORDER BY issued_position
$$;

REVOKE ALL PRIVILEGES ON FUNCTION
    ple_api.save_student_assignment_attempt_response(bigint, integer, jsonb),
    ple_api.finalize_student_assignment_attempt(bigint),
    ple_api.read_student_assignment_attempt_saved_response(bigint, integer)
    FROM PUBLIC;
GRANT EXECUTE ON FUNCTION
    ple_api.save_student_assignment_attempt_response(bigint, integer, jsonb),
    ple_api.finalize_student_assignment_attempt(bigint),
    ple_api.read_student_assignment_attempt_saved_response(bigint, integer)
    TO ple_app;

RESET ROLE;

SET LOCAL ROLE ple_api_owner;
REVOKE CREATE ON SCHEMA ple_api FROM ple_private_owner;
RESET ROLE;
