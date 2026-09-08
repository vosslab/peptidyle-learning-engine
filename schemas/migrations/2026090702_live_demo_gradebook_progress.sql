-- Include every active Student in the Instructor Gradebook progress projection.

SET LOCAL ROLE ple_api_owner;
GRANT CREATE ON SCHEMA ple_api TO ple_private_owner;
RESET ROLE;

SET LOCAL ROLE ple_private_owner;
DROP FUNCTION ple_api.read_live_demo_gradebook(bigint);

-- ASVS 4.1.3, 8.2.1, and 8.3.1: exact-Course Instructor authorization
-- remains inside this SECURITY DEFINER boundary. The expanded projection
-- carries only course-local identity and answer-free progress aggregates.
CREATE FUNCTION ple_api.read_live_demo_gradebook(p_course_reference_number bigint)
RETURNS TABLE (
    course_reference_number bigint,
    roster_id text,
    assignment_reference_number bigint,
    assignment_attempt_completion text,
    graded_question_count bigint,
    question_count bigint,
    points_earned double precision,
    points_possible double precision
)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private, ple_audit AS $$
DECLARE
    v_course_id uuid;
    v_instructor_account_id uuid;
BEGIN
    IF p_course_reference_number NOT BETWEEN 1 AND 2147483647 THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Gradebook Reference is invalid';
    END IF;
    SELECT course.course_id INTO v_course_id
      FROM ple_data.course_instance AS course
     WHERE course.reference_number = p_course_reference_number;
    IF NOT FOUND THEN
        RETURN;
    END IF;
    v_instructor_account_id :=
        ple_private.require_live_demo_current_course_instructor(v_course_id);
    PERFORM ple_audit.record_live_demo_gradebook_read(
        v_course_id, v_instructor_account_id
    );

    RETURN QUERY
    WITH released_assignment AS (
        SELECT assignment.assignment_id,
               assignment.reference_number,
               COALESCE(question_total.question_count, 0)::bigint AS question_count
          FROM ple_data.assignment AS assignment
          LEFT JOIN LATERAL (
              SELECT sum(
                         CASE entry.entry_kind
                             WHEN 'fixed_question' THEN 1
                             ELSE pool.selection_count
                         END
                     ) AS question_count
                FROM ple_data.assignment_revision_entry AS entry
                LEFT JOIN ple_data.assignment_revision_question_pool AS pool
                  ON pool.assignment_revision_id = entry.assignment_revision_id
                 AND pool.assignment_entry_id = entry.assignment_entry_id
               WHERE entry.assignment_revision_id =
                     assignment.released_assignment_revision_id
          ) AS question_total ON true
         WHERE assignment.course_id = v_course_id
           AND assignment.assignment_status = 'released'
    ), active_student AS (
        SELECT profile.roster_id, record.student_record_id
          FROM ple_private.course_roster_profile AS profile
          JOIN ple_data.student_record AS record
            ON record.course_id = profile.course_id
           AND record.student_account_id = profile.student_account_id
          JOIN ple_data.course_membership AS membership
            ON membership.course_id = record.course_id
           AND membership.student_record_id = record.student_record_id
           AND membership.role = 'student'
         WHERE profile.course_id = v_course_id
           AND ple_data.course_membership_is_active(membership.membership_id)
    ), student_work AS (
        SELECT student.roster_id,
               assignment.reference_number AS assignment_reference_number,
               CASE
                   WHEN attempt.assignment_attempt_id IS NULL THEN NULL
                   WHEN attempt.completed_at IS NOT NULL THEN 'completed'
                   ELSE 'in_progress'
               END AS assignment_attempt_completion,
               count(DISTINCT result.grading_result_id)::bigint AS graded_question_count,
               assignment.question_count,
               COALESCE(sum(result.points_earned), 0)::double precision AS points_earned,
               COALESCE(sum(result.points_possible), 0)::double precision AS points_possible
          FROM active_student AS student
         CROSS JOIN released_assignment AS assignment
          LEFT JOIN LATERAL (
              SELECT candidate.assignment_attempt_id, candidate.completed_at
                FROM ple_private.assignment_attempt AS candidate
               WHERE candidate.student_record_id = student.student_record_id
                 AND candidate.assignment_id = assignment.assignment_id
               ORDER BY candidate.started_at DESC,
                        candidate.assignment_attempt_id DESC
               LIMIT 1
          ) AS attempt ON true
          LEFT JOIN ple_private.issued_question AS issued
            ON issued.assignment_attempt_id = attempt.assignment_attempt_id
          LEFT JOIN ple_private.question_attempt AS question_attempt
            ON question_attempt.issued_question_id = issued.issued_question_id
          LEFT JOIN ple_private.grading_result AS result
            ON result.question_attempt_id = question_attempt.question_attempt_id
         GROUP BY student.roster_id, assignment.reference_number,
                  assignment.question_count, attempt.assignment_attempt_id,
                  attempt.completed_at
    )
    SELECT p_course_reference_number, work.roster_id,
           work.assignment_reference_number,
           work.assignment_attempt_completion,
           work.graded_question_count, work.question_count,
           work.points_earned, work.points_possible
      FROM student_work AS work
    UNION ALL
    SELECT p_course_reference_number, NULL::text, NULL::bigint, NULL::text,
           NULL::bigint, NULL::bigint, NULL::double precision,
           NULL::double precision
     WHERE NOT EXISTS (SELECT 1 FROM student_work)
     ORDER BY roster_id NULLS FIRST, assignment_reference_number NULLS FIRST;
END
$$;

REVOKE ALL PRIVILEGES ON FUNCTION ple_api.read_live_demo_gradebook(bigint)
    FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_api.read_live_demo_gradebook(bigint) TO ple_app;

RESET ROLE;

SET LOCAL ROLE ple_api_owner;
REVOKE CREATE ON SCHEMA ple_api FROM ple_private_owner;
RESET ROLE;
