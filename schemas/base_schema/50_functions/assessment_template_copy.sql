-- Functions, triggers, and views from assessment_template_copy.sql.

SET LOCAL ROLE ple_data_owner;

-- Create a direct Course Assessment from one owned Assessment Template.
-- The Template is read once and its portable settings are copied by value;
-- neither aggregate retains the other's identity.
CREATE FUNCTION ple_data.create_assessment_from_template_values(
    p_assessment_id text,
    p_course_reference_number text,
    p_assessment_type text,
    p_title text,
    p_instructions text,
    p_assessment_attempt_time_limit_seconds integer,
    p_assessment_attempt_limit integer,
    p_late_work_rule text,
    p_question_variation_rule text,
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
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
DECLARE
    created record;
    snapshot_id ple_data.sha256_digest;
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

    snapshot_id := ple_private.ensure_assessment_policy_snapshot(
        p_title, p_instructions, NULL, NULL, NULL,
        p_assessment_attempt_time_limit_seconds, p_assessment_attempt_limit,
        p_late_work_rule::ple_data.late_work_rule,
        p_question_variation_rule::ple_data.question_variation_rule,
        p_assessment_question_order_rule::ple_data.question_order_rule,
        p_feedback_score::ple_data.feedback_release,
        p_feedback_per_item_correctness::ple_data.feedback_release,
        p_feedback_submitted_response::ple_data.feedback_release,
        p_feedback_question_answer::ple_data.feedback_release,
        p_feedback_question_answer_explanation::ple_data.feedback_release,
        p_feedback_class_statistics::ple_data.feedback_release,
        p_assessment_type::ple_data.assessment_type
    );
    UPDATE ple_data.assessment AS assessment
       SET assessment_policy_snapshot_id = snapshot_id
     WHERE assessment.assessment_id = p_assessment_id;

    assessment_reference_number := created.assessment_public_reference;
    assessment_edit_number := created.assessment_edit_number;
    assessment_status := created.assessment_status;
    assessment_title := created.assessment_title;
    assessment_instructions := created.assessment_instructions;
    RETURN NEXT;
END
$$;

SET LOCAL ROLE ple_api_owner;

CREATE FUNCTION ple_api.create_assessment_from_template(
    p_assessment_id text,
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
    course_reference_number text;
    template ple_private.assessment_template%ROWTYPE;
    policy ple_data.assessment_policy_snapshot%ROWTYPE;
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
    SELECT * INTO policy
      FROM ple_data.assessment_policy_snapshot
     WHERE assessment_policy_snapshot_id = template.assessment_policy_snapshot_id;

    SELECT course.course_instance_id INTO course_reference_number
      FROM ple_data.course_instance AS course
     WHERE course.course_instance_id = p_course_reference_number;

    -- ASVS 1.2.4 and 2.2.2: every input is a typed procedure parameter.
    -- Canonical direct creation repeats destination Course authorization.
    RETURN QUERY
    SELECT created.*
      FROM ple_data.create_assessment_from_template_values(
        p_assessment_id,
        course_reference_number,
        template.assessment_type,
        p_title,
        policy.assessment_instructions,
        policy.assessment_attempt_time_limit_seconds,
        policy.assessment_attempt_limit,
        policy.late_work_rule,
        policy.question_variation_rule,
        policy.assessment_question_order_rule,
        policy.feedback_score,
        policy.feedback_per_item_correctness,
        policy.feedback_submitted_response,
        policy.feedback_question_answer,
        policy.feedback_question_answer_explanation,
        policy.feedback_class_statistics
    ) AS created;
END
$$;

