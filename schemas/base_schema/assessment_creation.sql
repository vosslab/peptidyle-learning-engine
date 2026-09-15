-- Manual Course Assessment creation is a direct-origin operation. Blueprint
-- adoption owns the only trusted path that records exact Blueprint provenance.

SET LOCAL ROLE ple_data_owner;

CREATE FUNCTION ple_data.create_assessment(
    p_assessment_id uuid,
    p_course_reference_number bigint,
    p_assessment_type text,
    p_title text,
    p_instructions text
) RETURNS TABLE (
    assessment_public_reference text, assessment_edit_number bigint,
    assessment_status text, assessment_title text, assessment_instructions text
)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data AS $$
DECLARE course_row ple_data.course_instance%ROWTYPE;
BEGIN
    -- ASVS 2.2.1-2.2.3 and 8.2.1-8.2.2: validate the closed creation shape
    -- and derive both Course authorization and direct origin in this trusted boundary.
    IF p_assessment_id IS NULL
       OR p_assessment_type IS NULL OR p_assessment_type NOT IN (
           'regular_assignment', 'practice_question_assignment', 'bonus_assignment', 'quiz', 'exam'
       )
       OR p_course_reference_number NOT BETWEEN 1 AND 2147483647
       OR p_title IS NULL OR p_instructions IS NULL THEN
        RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Assessment creation is invalid';
    END IF;
    SELECT * INTO course_row FROM ple_data.course_instance
     WHERE reference_number = p_course_reference_number;
    IF NOT FOUND OR NOT ple_api.current_session_account_is_course_instructor(course_row.course_id) THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Assessment is unavailable';
    END IF;
    INSERT INTO ple_data.assessment AS inserted (
        assessment_id, course_id, origin_kind,
        source_blueprint_course_reference_number, source_blueprint_revision_number,
        source_blueprint_assessment_reference,
        created_at, updated_at, assessment_type, assessment_title, assessment_instructions,
        assessment_attempt_limit, late_work_rule, assessment_attempt_grade_rule,
        question_pool_reuse_rule, question_variation_rule,
        assessment_attempt_resume_rule, assessment_question_display_rule,
        assessment_navigation_rule, assessment_question_order_rule, feedback_score,
        feedback_per_item_correctness, feedback_submitted_response,
        feedback_question_feedback, feedback_question_answer,
        feedback_question_answer_explanation, feedback_class_statistics
    ) VALUES (
        p_assessment_id, course_row.course_id, 'direct', NULL, NULL, NULL,
        clock_timestamp(), clock_timestamp(), p_assessment_type, p_title, p_instructions,
        CASE WHEN p_assessment_type IN ('quiz', 'exam') THEN 1 ELSE NULL END,
        'reject', 'highest', 'reuse_selection', 'new_variation',
        'resumable', 'one_question_at_a_time', 'free_navigation', 'shuffled',
        'after_submit', 'after_submit', 'after_submit', 'never',
        CASE WHEN p_assessment_type IN ('practice_question_assignment', 'quiz', 'exam')
             THEN 'after_submit' ELSE 'never' END,
        CASE WHEN p_assessment_type IN ('quiz', 'exam')
             THEN 'after_submit' ELSE 'never' END,
        'never'
    ) RETURNING inserted.public_reference, inserted.assessment_edit_number,
        inserted.assessment_status, inserted.assessment_title, inserted.assessment_instructions
      INTO assessment_public_reference, assessment_edit_number, assessment_status,
           assessment_title, assessment_instructions;
    RETURN NEXT;
END
$$;

REVOKE ALL ON FUNCTION ple_data.create_assessment(uuid, bigint, text, text, text) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_data.create_assessment(uuid, bigint, text, text, text)
    TO ple_api_owner;

RESET ROLE;
