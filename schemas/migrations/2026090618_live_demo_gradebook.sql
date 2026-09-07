-- M15 Instructor Gradebook projection from immutable Grading Results.
--
-- This read model is intentionally not a new grade-calculation engine. It
-- groups only M13's completed immutable Grading Results and omits Student
-- Responses, Answer Keys, source, private IDs, and grader internals.

SET LOCAL ROLE ple_private_owner;
CREATE POLICY course_roster_profile_private_owner_gradebook_read
    ON ple_private.course_roster_profile FOR SELECT TO ple_private_owner USING (true);
CREATE POLICY grading_result_private_owner_gradebook_read
    ON ple_private.grading_result FOR SELECT TO ple_private_owner USING (true);
RESET ROLE;

SET LOCAL ROLE ple_audit_owner;
CREATE TABLE ple_audit.live_demo_gradebook_read_event (
    event_id uuid PRIMARY KEY,
    course_id uuid NOT NULL,
    instructor_account_id uuid NOT NULL,
    occurred_at timestamp with time zone NOT NULL
);
ALTER TABLE ple_audit.live_demo_gradebook_read_event ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_audit.live_demo_gradebook_read_event FORCE ROW LEVEL SECURITY;
CREATE POLICY live_demo_gradebook_read_event_audit_owner_create
    ON ple_audit.live_demo_gradebook_read_event
    FOR INSERT TO ple_audit_owner WITH CHECK (true);
CREATE FUNCTION ple_audit.reject_live_demo_gradebook_read_event_change()
RETURNS trigger LANGUAGE plpgsql SET search_path = pg_catalog, ple_audit AS $$
BEGIN
    RAISE EXCEPTION USING ERRCODE = '55000',
        MESSAGE = 'Gradebook read audit events are immutable';
END
$$;
CREATE TRIGGER live_demo_gradebook_read_event_is_immutable
BEFORE UPDATE OR DELETE ON ple_audit.live_demo_gradebook_read_event
FOR EACH ROW EXECUTE FUNCTION ple_audit.reject_live_demo_gradebook_read_event_change();
CREATE FUNCTION ple_audit.record_live_demo_gradebook_read(
    p_course_id uuid, p_instructor_account_id uuid
) RETURNS void LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_audit AS $$
BEGIN
    IF p_course_id IS NULL OR p_instructor_account_id IS NULL THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Gradebook audit arguments are invalid';
    END IF;
    INSERT INTO ple_audit.live_demo_gradebook_read_event (
        event_id, course_id, instructor_account_id, occurred_at
    ) VALUES (
        pg_catalog.gen_random_uuid(), p_course_id, p_instructor_account_id,
        pg_catalog.transaction_timestamp()
    );
END
$$;
REVOKE ALL PRIVILEGES ON TABLE ple_audit.live_demo_gradebook_read_event FROM PUBLIC;
REVOKE ALL PRIVILEGES ON FUNCTION ple_audit.reject_live_demo_gradebook_read_event_change() FROM PUBLIC;
REVOKE ALL PRIVILEGES ON FUNCTION ple_audit.record_live_demo_gradebook_read(uuid, uuid) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_audit.record_live_demo_gradebook_read(uuid, uuid)
    TO ple_private_owner;
RESET ROLE;

SET LOCAL ROLE ple_api_owner;
GRANT CREATE ON SCHEMA ple_api TO ple_private_owner;
RESET ROLE;

SET LOCAL ROLE ple_private_owner;
-- ASVS 4.1.3, 8.2.1: this definer resolves the Course by its public
-- reference and repeats direct current Instructor authority before touching
-- any Student Work. The API role receives execute-only access below.
CREATE FUNCTION ple_api.read_live_demo_gradebook(p_course_reference_number bigint)
RETURNS TABLE (
    course_reference_number bigint,
    roster_id text,
    assignment_reference_number bigint,
    graded_question_count bigint,
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
            MESSAGE = 'Gradebook arguments are invalid';
    END IF;
    SELECT course.course_id INTO v_course_id
      FROM ple_data.course_instance AS course
     WHERE course.reference_number = p_course_reference_number;
    IF NOT FOUND THEN
        RETURN;
    END IF;
    v_instructor_account_id := ple_private.require_live_demo_current_course_instructor(v_course_id);
    PERFORM ple_audit.record_live_demo_gradebook_read(v_course_id, v_instructor_account_id);

    RETURN QUERY
    WITH graded AS (
        SELECT profile.roster_id,
               assignment.reference_number AS assignment_reference_number,
               pg_catalog.count(result.grading_result_id)::bigint AS graded_question_count,
               pg_catalog.sum(result.points_earned)::double precision AS points_earned,
               pg_catalog.sum(result.points_possible)::double precision AS points_possible
          FROM ple_private.course_roster_profile AS profile
          JOIN ple_data.student_record AS student_record
            ON student_record.course_id = profile.course_id
           AND student_record.student_account_id = profile.student_account_id
          JOIN ple_private.assignment_attempt AS assignment_attempt
            ON assignment_attempt.student_record_id = student_record.student_record_id
          JOIN ple_data.assignment AS assignment
            ON assignment.assignment_id = assignment_attempt.assignment_id
           AND assignment.course_id = profile.course_id
          JOIN ple_private.issued_question AS issued
            ON issued.assignment_attempt_id = assignment_attempt.assignment_attempt_id
          JOIN ple_private.question_attempt AS question_attempt
            ON question_attempt.issued_question_id = issued.issued_question_id
          JOIN ple_private.grading_result AS result
            ON result.question_attempt_id = question_attempt.question_attempt_id
         WHERE profile.course_id = v_course_id
         GROUP BY profile.roster_id, assignment.reference_number
    )
    SELECT p_course_reference_number, graded.roster_id,
           graded.assignment_reference_number, graded.graded_question_count,
           graded.points_earned, graded.points_possible
      FROM graded
    UNION ALL
    SELECT p_course_reference_number, NULL::text, NULL::bigint, NULL::bigint,
           NULL::double precision, NULL::double precision
     WHERE NOT EXISTS (SELECT 1 FROM graded)
     ORDER BY roster_id NULLS FIRST, assignment_reference_number NULLS FIRST;
END
$$;
REVOKE ALL PRIVILEGES ON FUNCTION ple_api.read_live_demo_gradebook(bigint) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_api.read_live_demo_gradebook(bigint) TO ple_app;
RESET ROLE;

SET LOCAL ROLE ple_api_owner;
REVOKE CREATE ON SCHEMA ple_api FROM ple_private_owner;
RESET ROLE;
