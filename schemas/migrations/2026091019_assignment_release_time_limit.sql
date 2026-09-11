-- Require a configured whole-Assignment Attempt limit before an Instructor
-- releases current authored work. Drafts remain nullable so the Instructor can
-- select the duration in Assignment Policies before release.

DO $$
BEGIN
    IF current_user <> 'ple_migrator' THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'migration 2026091019 must run as ple_migrator';
    END IF;
END
$$;

SET LOCAL ROLE ple_api_owner;

CREATE OR REPLACE FUNCTION ple_api.validate_live_demo_assignment_release(
    p_course_reference_number bigint,
    p_assignment_reference_number bigint
) RETURNS TABLE (issue text)
LANGUAGE plpgsql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
DECLARE
    v_assignment_id uuid;
    v_course_id uuid;
    v_attempt_time_limit_seconds integer;
BEGIN
    -- ASVS 2.2.1, 2.3.1, and 8.3.1: the trusted release boundary validates
    -- the selected duration before an immutable Assignment Revision exists.
    SELECT assignment.assignment_id,
           course.course_id,
           assignment.assignment_attempt_time_limit_seconds
      INTO v_assignment_id, v_course_id, v_attempt_time_limit_seconds
      FROM ple_data.course_instance AS course
      JOIN ple_data.assignment AS assignment ON assignment.course_id = course.course_id
     WHERE course.reference_number = p_course_reference_number
       AND assignment.reference_number = p_assignment_reference_number;

    IF NOT FOUND OR NOT ple_api.current_session_account_is_course_instructor(v_course_id) THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Assignment release requires a current Instructor Course Membership';
    END IF;

    IF v_attempt_time_limit_seconds IS NULL OR v_attempt_time_limit_seconds <= 0 THEN
        issue := 'time_limit_required';
        RETURN NEXT;
    END IF;
    IF NOT EXISTS (
        SELECT 1
          FROM ple_private.live_demo_assignment_question
         WHERE assignment_id = v_assignment_id
    ) THEN
        issue := 'no_published_questions';
        RETURN NEXT;
    END IF;
    IF EXISTS (
        SELECT 1
          FROM ple_private.live_demo_assignment_question AS selected
          LEFT JOIN ple_api.published_question_summary AS summary
            ON summary.question_id = selected.question_id
          LEFT JOIN LATERAL (
              SELECT event.availability
                FROM ple_data.question_revision_availability_event AS event
               WHERE event.question_id = summary.question_id
                 AND event.revision_number = summary.latest_question_revision_number
               ORDER BY event.occurred_at DESC, event.event_id DESC
               LIMIT 1
          ) AS availability ON true
         WHERE selected.assignment_id = v_assignment_id
           AND (summary.question_id IS NULL OR availability.availability IS DISTINCT FROM 'available')
    ) THEN
        issue := 'question_unavailable';
        RETURN NEXT;
    END IF;
END
$$;

REVOKE ALL PRIVILEGES ON FUNCTION ple_api.validate_live_demo_assignment_release(bigint, bigint)
    FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_api.validate_live_demo_assignment_release(bigint, bigint)
    TO ple_app;

RESET ROLE;
