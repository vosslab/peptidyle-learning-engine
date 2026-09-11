-- New Assignments present Questions one at a time and shuffle their Question
-- order by default. Existing authored order remains an explicit saved choice.
DO $$
BEGIN
    IF current_user <> 'ple_migrator' THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'migration 2026091020 must run as ple_migrator';
    END IF;
END
$$;

SET LOCAL ROLE ple_api_owner;

CREATE OR REPLACE FUNCTION ple_api.create_live_demo_assignment(
    p_assignment_id uuid,
    p_course_reference_number bigint,
    p_title text,
    p_instructions text
) RETURNS TABLE (
    reference_number bigint,
    assignment_edit_number bigint,
    assignment_status text,
    assignment_title text,
    assignment_instructions text
) LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data AS $$
DECLARE
    c ple_data.course_instance%ROWTYPE;
    o ple_data.course_origin%ROWTYPE;
BEGIN
    IF p_assignment_id IS NULL OR p_title IS NULL OR p_instructions IS NULL THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Assignment Workspace arguments are invalid';
    END IF;
    SELECT * INTO c FROM ple_data.course_instance AS course
     WHERE course.reference_number = p_course_reference_number;
    IF NOT FOUND OR NOT ple_api.current_session_account_is_course_instructor(c.course_id) THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Assignment Workspace requires a current Instructor Course Membership';
    END IF;
    SELECT * INTO o FROM ple_data.course_origin AS origin WHERE origin.course_id = c.course_id;
    INSERT INTO ple_data.assignment(
        assignment_id, course_id, source_blueprint_course_reference_number,
        source_blueprint_revision_number, created_at, updated_at, assignment_edit_number,
        assignment_title, assignment_instructions, available_at, due_at, closes_at,
        assignment_attempt_time_limit_seconds, attempt_limit, late_work_rule,
        assignment_deadline_rule, assignment_completion_rule,
        assignment_completion_score_threshold, assignment_attempt_grade_rule,
        assignment_attempt_continuation_rule, max_additional_assignment_attempts,
        question_pool_reuse_rule, question_variation_rule, assignment_attempt_resume_rule,
        assignment_question_display_rule, assignment_navigation_rule,
        assignment_question_order_rule, assignment_status
    ) VALUES (
        p_assignment_id, c.course_id, o.blueprint_course_reference_number,
        o.blueprint_revision_number, transaction_timestamp(), transaction_timestamp(), 1,
        p_title, p_instructions, NULL, NULL, NULL, NULL, NULL, 'reject', 'auto_submit',
        'answer_all', NULL, 'highest', 'unlimited', NULL, 'reuse_selection', 'new_variation',
        'resumable', 'one_question_at_a_time', 'free_navigation', 'shuffled', 'unreleased'
    ) RETURNING assignment.reference_number, assignment.assignment_edit_number,
        assignment.assignment_status, assignment.assignment_title, assignment.assignment_instructions
        INTO reference_number, assignment_edit_number, assignment_status, assignment_title,
            assignment_instructions;
    RETURN NEXT;
END
$$;

REVOKE ALL PRIVILEGES ON FUNCTION ple_api.create_live_demo_assignment(uuid, bigint, text, text)
    FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_api.create_live_demo_assignment(uuid, bigint, text, text) TO ple_app;

RESET ROLE;
