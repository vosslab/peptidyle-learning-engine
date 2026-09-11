-- Return a current Assignment Workspace save unchanged when the reviewed
-- authored values and ordered Question selection already match the draft.
DO $$
BEGIN
    IF current_user <> 'ple_migrator' THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'migration 2026091022 must run as ple_migrator';
    END IF;
END
$$;

SET LOCAL ROLE ple_api_owner;

CREATE OR REPLACE FUNCTION ple_api.save_live_demo_assignment(
    p_course_reference_number bigint,
    p_assignment_reference_number bigint,
    p_expected_edit_number bigint,
    p_title text,
    p_instructions text,
    p_question_ids text[],
    p_due_at_millis bigint,
    p_late_work_rule text,
    p_time_limit integer,
    p_attempt_limit integer,
    p_completion text,
    p_grade text,
    p_continuation text,
    p_pool_reuse text,
    p_variation text,
    p_resume text,
    p_display text,
    p_navigation text,
    p_question_order text,
    p_threshold double precision,
    p_max_additional integer,
    p_feedback_score text,
    p_feedback_correctness text,
    p_feedback_response text,
    p_feedback_question text,
    p_feedback_answer text,
    p_feedback_explanation text,
    p_feedback_statistics text
) RETURNS TABLE (
    reference_number bigint,
    assignment_edit_number bigint,
    assignment_status text,
    assignment_title text,
    assignment_instructions text
) LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
DECLARE
    a ple_data.assignment%ROWTYPE;
    v_selection_changed boolean;
    v_content_changed boolean;
    i integer;
    q text;
BEGIN
    -- ASVS 2.2.1 and 8.3.1: trusted persistence validates the closed timing
    -- vocabulary before changing the Instructor-authorized Assignment record.
    IF p_expected_edit_number <= 0 OR p_question_ids IS NULL OR cardinality(p_question_ids) > 25
       OR cardinality(p_question_ids) <> cardinality(ARRAY(
           SELECT DISTINCT value FROM unnest(p_question_ids) AS value
       ))
       OR p_late_work_rule NOT IN ('accept', 'mark_late', 'reject')
       OR p_time_limit <= 0 OR p_attempt_limit <= 0
       OR p_completion NOT IN ('answer_all', 'all_correct', 'score_at_least')
       OR p_grade NOT IN ('first', 'latest', 'highest', 'instructor_selected')
       OR p_continuation NOT IN ('unlimited', 'capped', 'closed')
       OR p_pool_reuse NOT IN ('reuse_selection', 'select_again')
       OR p_variation NOT IN ('reuse_variation', 'new_variation')
       OR p_resume NOT IN ('resumable', 'single_session')
       OR p_display NOT IN ('all_questions', 'one_question_at_a_time')
       OR p_navigation NOT IN ('free_navigation', 'forward_only')
       OR p_question_order NOT IN ('authored_order', 'shuffled')
       OR p_threshold IS NOT NULL AND (p_threshold < 0 OR p_threshold > 1)
       OR p_completion = 'score_at_least' AND p_threshold IS NULL
       OR p_continuation = 'capped' AND (p_max_additional IS NULL OR p_max_additional < 0)
       OR p_feedback_score NOT IN ('during_attempt', 'after_submit', 'after_due', 'after_close', 'never')
       OR p_feedback_correctness NOT IN ('during_attempt', 'after_submit', 'after_due', 'after_close', 'never')
       OR p_feedback_response NOT IN ('during_attempt', 'after_submit', 'after_due', 'after_close', 'never')
       OR p_feedback_question NOT IN ('during_attempt', 'after_submit', 'after_due', 'after_close', 'never')
       OR p_feedback_answer NOT IN ('during_attempt', 'after_submit', 'after_due', 'after_close', 'never')
       OR p_feedback_explanation NOT IN ('during_attempt', 'after_submit', 'after_due', 'after_close', 'never')
       OR p_feedback_statistics NOT IN ('during_attempt', 'after_submit', 'after_due', 'after_close', 'never') THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Assignment Workspace save arguments are invalid';
    END IF;

    SELECT assignment.* INTO a
      FROM ple_data.course_instance AS c
      JOIN ple_data.assignment AS assignment ON assignment.course_id = c.course_id
     WHERE c.reference_number = p_course_reference_number
       AND assignment.reference_number = p_assignment_reference_number
     FOR UPDATE OF assignment;
    IF NOT FOUND OR NOT ple_api.current_session_account_is_course_instructor(a.course_id) THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Assignment Workspace requires a current Instructor Course Membership';
    END IF;
    IF a.assignment_edit_number <> p_expected_edit_number OR a.assignment_status <> 'unreleased' THEN
        RAISE EXCEPTION USING ERRCODE = '40001', MESSAGE = 'Assignment Workspace changed';
    END IF;

    FOREACH q IN ARRAY p_question_ids LOOP
        IF NOT EXISTS (
            SELECT 1 FROM ple_api.published_question_summary AS s
            JOIN LATERAL (
                SELECT e.availability FROM ple_data.question_revision_availability_event AS e
                 WHERE e.question_id = s.question_id
                   AND e.revision_number = s.latest_question_revision_number
                 ORDER BY e.occurred_at DESC, e.event_id DESC LIMIT 1
            ) AS x ON x.availability = 'available'
            WHERE s.question_id = q
        ) THEN
            RAISE EXCEPTION USING ERRCODE = '22023',
                MESSAGE = 'Assignment Workspace selects an unavailable Published Question';
        END IF;
    END LOOP;

    SELECT COALESCE(array_agg(question_id ORDER BY question_index), ARRAY[]::text[])
           IS DISTINCT FROM p_question_ids
      INTO v_selection_changed
      FROM ple_private.live_demo_assignment_question
     WHERE assignment_id = a.assignment_id;
    v_content_changed := ROW(
        a.assignment_title, a.assignment_instructions, a.due_at, a.late_work_rule,
        a.assignment_attempt_time_limit_seconds, a.attempt_limit, a.assignment_completion_rule,
        a.assignment_completion_score_threshold, a.assignment_attempt_grade_rule,
        a.assignment_attempt_continuation_rule, a.max_additional_assignment_attempts,
        a.question_pool_reuse_rule, a.question_variation_rule,
        a.assignment_attempt_resume_rule, a.assignment_question_display_rule,
        a.assignment_navigation_rule, a.assignment_question_order_rule, a.feedback_score,
        a.feedback_per_item_correctness, a.feedback_submitted_response,
        a.feedback_question_feedback, a.feedback_question_answer,
        a.feedback_question_answer_explanation, a.feedback_class_statistics
    ) IS DISTINCT FROM ROW(
        p_title, p_instructions,
        CASE WHEN p_due_at_millis IS NULL THEN NULL
             ELSE to_timestamp(p_due_at_millis::double precision / 1000) END,
        p_late_work_rule, p_time_limit, p_attempt_limit, p_completion, p_threshold, p_grade,
        p_continuation, p_max_additional, p_pool_reuse, p_variation, p_resume, p_display,
        p_navigation, p_question_order, p_feedback_score, p_feedback_correctness,
        p_feedback_response, p_feedback_question, p_feedback_answer, p_feedback_explanation,
        p_feedback_statistics
    );

    IF NOT v_selection_changed AND NOT v_content_changed THEN
        reference_number := a.reference_number;
        assignment_edit_number := a.assignment_edit_number;
        assignment_status := a.assignment_status;
        assignment_title := a.assignment_title;
        assignment_instructions := a.assignment_instructions;
        RETURN NEXT;
        RETURN;
    END IF;

    IF v_selection_changed THEN
        DELETE FROM ple_private.live_demo_assignment_question WHERE assignment_id = a.assignment_id;
        FOR i IN 1..COALESCE(cardinality(p_question_ids), 0) LOOP
            INSERT INTO ple_private.live_demo_assignment_question
            VALUES (a.assignment_id, p_question_ids[i], i - 1);
        END LOOP;
    END IF;

    UPDATE ple_data.assignment AS updated_assignment
       SET assignment_title = p_title,
           assignment_instructions = p_instructions,
           due_at = CASE WHEN p_due_at_millis IS NULL THEN NULL
                         ELSE to_timestamp(p_due_at_millis::double precision / 1000) END,
           late_work_rule = p_late_work_rule,
           assignment_attempt_time_limit_seconds = p_time_limit,
           attempt_limit = p_attempt_limit,
           assignment_completion_rule = p_completion,
           assignment_completion_score_threshold = p_threshold,
           assignment_attempt_grade_rule = p_grade,
           assignment_attempt_continuation_rule = p_continuation,
           max_additional_assignment_attempts = p_max_additional,
           question_pool_reuse_rule = p_pool_reuse,
           question_variation_rule = p_variation,
           assignment_attempt_resume_rule = p_resume,
           assignment_question_display_rule = p_display,
           assignment_navigation_rule = p_navigation,
           assignment_question_order_rule = p_question_order,
           feedback_score = p_feedback_score,
           feedback_per_item_correctness = p_feedback_correctness,
           feedback_submitted_response = p_feedback_response,
           feedback_question_feedback = p_feedback_question,
           feedback_question_answer = p_feedback_answer,
           feedback_question_answer_explanation = p_feedback_explanation,
           feedback_class_statistics = p_feedback_statistics,
           assignment_edit_number = updated_assignment.assignment_edit_number + 1,
           live_demo_question_selection_version = updated_assignment.live_demo_question_selection_version
               + CASE WHEN v_selection_changed THEN 1 ELSE 0 END,
           updated_at = transaction_timestamp()
     WHERE updated_assignment.assignment_id = a.assignment_id
     RETURNING updated_assignment.reference_number,
               updated_assignment.assignment_edit_number,
               updated_assignment.assignment_status,
               updated_assignment.assignment_title,
               updated_assignment.assignment_instructions
          INTO reference_number,
               assignment_edit_number,
               assignment_status,
               assignment_title,
               assignment_instructions;
    RETURN NEXT;
END
$$;

REVOKE ALL PRIVILEGES ON FUNCTION ple_api.save_live_demo_assignment(
    bigint, bigint, bigint, text, text, text[], bigint, text, integer, integer,
    text, text, text, text, text, text, text, text, text, double precision, integer,
    text, text, text, text, text, text, text
) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_api.save_live_demo_assignment(
    bigint, bigint, bigint, text, text, text[], bigint, text, integer, integer,
    text, text, text, text, text, text, text, text, text, double precision, integer,
    text, text, text, text, text, text, text
) TO ple_app;

RESET ROLE;
