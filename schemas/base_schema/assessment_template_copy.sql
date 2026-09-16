-- Create a direct Course Assessment from one owned Assessment Template.
-- The Template is read once and its portable settings are copied by value;
-- neither aggregate retains the other's identity.

SET LOCAL ROLE ple_data_owner;

CREATE FUNCTION ple_data.create_assessment_from_template_values(
    p_assessment_id uuid,
    p_course_reference_number bigint,
    p_assessment_type text,
    p_title text,
    p_instructions text,
    p_assessment_attempt_time_limit_seconds integer,
    p_assessment_attempt_limit integer,
    p_late_work_rule text,
    p_assessment_attempt_grade_rule text,
    p_question_variation_rule text,
    p_assessment_attempt_resume_rule text,
    p_assessment_question_display_rule text,
    p_assessment_navigation_rule text,
    p_assessment_question_order_rule text,
    p_feedback_score text,
    p_feedback_per_item_correctness text,
    p_feedback_submitted_response text,
    p_feedback_question_answer text,
    p_feedback_question_answer_explanation text,
    p_feedback_class_statistics text
) RETURNS TABLE (
    assessment_reference_number text,
    assessment_edit_number bigint,
    assessment_status text,
    assessment_title text,
    assessment_instructions text
)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data AS $$
DECLARE
    created record;
BEGIN
    -- ASVS 2.3.3: canonical direct creation and the initial by-value copy are
    -- one transaction. A failure leaves neither a partial Assessment nor links.
    SELECT * INTO created
      FROM ple_data.create_assessment(
        p_assessment_id,
        p_course_reference_number,
        p_assessment_type,
        p_title,
        p_instructions
    );

    UPDATE ple_data.assessment AS assessment
       SET assessment_attempt_time_limit_seconds =
               p_assessment_attempt_time_limit_seconds,
           assessment_attempt_limit = p_assessment_attempt_limit,
           late_work_rule = p_late_work_rule,
           assessment_attempt_grade_rule = p_assessment_attempt_grade_rule,
           question_variation_rule = p_question_variation_rule,
           assessment_attempt_resume_rule = p_assessment_attempt_resume_rule,
           assessment_question_display_rule = p_assessment_question_display_rule,
           assessment_navigation_rule = p_assessment_navigation_rule,
           assessment_question_order_rule = p_assessment_question_order_rule,
           feedback_score = p_feedback_score,
           feedback_per_item_correctness = p_feedback_per_item_correctness,
           feedback_submitted_response = p_feedback_submitted_response,
           feedback_question_answer = p_feedback_question_answer,
           feedback_question_answer_explanation = p_feedback_question_answer_explanation,
           feedback_class_statistics = p_feedback_class_statistics
     WHERE assessment.assessment_id = p_assessment_id;

    assessment_reference_number := created.assessment_public_reference;
    assessment_edit_number := created.assessment_edit_number;
    assessment_status := created.assessment_status;
    assessment_title := created.assessment_title;
    assessment_instructions := created.assessment_instructions;
    RETURN NEXT;
END
$$;

REVOKE ALL ON FUNCTION ple_data.create_assessment_from_template_values(
    uuid, bigint, text, text, text, integer, integer, text, text, text,
    text, text, text, text, text, text, text, text, text, text
) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_data.create_assessment_from_template_values(
    uuid, bigint, text, text, text, integer, integer, text, text, text,
    text, text, text, text, text, text, text, text, text, text
) TO ple_api_owner;

RESET ROLE;

SET LOCAL ROLE ple_api_owner;

CREATE FUNCTION ple_api.create_assessment_from_template(
    p_assessment_id uuid,
    p_course_reference_number text,
    p_assessment_template_id uuid,
    p_title text
) RETURNS TABLE (
    assessment_reference_number text,
    assessment_edit_number bigint,
    assessment_status text,
    assessment_title text,
    assessment_instructions text
)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
DECLARE
    course_reference_number bigint;
    template ple_private.assessment_template%ROWTYPE;
BEGIN
    IF p_assessment_id IS NULL OR p_assessment_template_id IS NULL OR p_title IS NULL
       OR NOT ple_api.current_session_account_is_instructor() THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Assessment Template is unavailable';
    END IF;

    -- ASVS 8.2.1-8.3.1: both object authorizations are derived from the
    -- installed server session. The private Template UUID is only a locator.
    SELECT owned.* INTO template
      FROM ple_private.assessment_template AS owned
     WHERE owned.assessment_template_id = p_assessment_template_id
       AND owned.owner_account_id = ple_api.current_session_account_id()
     FOR SHARE;
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Assessment Template is unavailable';
    END IF;

    SELECT course.reference_number INTO course_reference_number
      FROM ple_data.course_instance AS course
     WHERE course.public_reference = p_course_reference_number;

    -- ASVS 1.2.4 and 2.2.2: every input is a typed procedure parameter.
    -- Canonical direct creation repeats destination Course authorization.
    RETURN QUERY
    SELECT created.*
      FROM ple_data.create_assessment_from_template_values(
        p_assessment_id,
        course_reference_number,
        template.assessment_type,
        p_title,
        template.instructions,
        template.assessment_attempt_time_limit_seconds,
        template.assessment_attempt_limit,
        template.late_work_rule,
        template.assessment_attempt_grade_rule,
        template.question_variation_rule,
        template.assessment_attempt_resume_rule,
        template.assessment_question_display_rule,
        template.assessment_navigation_rule,
        template.assessment_question_order_rule,
        template.feedback_score,
        template.feedback_per_item_correctness,
        template.feedback_submitted_response,
        template.feedback_question_answer,
        template.feedback_question_answer_explanation,
        template.feedback_class_statistics
    ) AS created;
END
$$;

REVOKE ALL ON FUNCTION ple_api.create_assessment_from_template(uuid, text, uuid, text)
    FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_api.create_assessment_from_template(uuid, text, uuid, text)
    TO ple_app;

RESET ROLE;
