-- Functions, triggers, and views from assessment_policy_snapshot.sql.

SET LOCAL ROLE ple_private_owner;

-- Content-addressed Assessment policy. Two equal policies share one immutable
-- row. Quiz and Exam attempt-limit = 1 is enforced here when the caller
-- supplies an Assessment type; table CHECKs cannot join that type.
CREATE FUNCTION ple_private.ensure_assessment_policy_snapshot(
    p_assessment_title text,
    p_assessment_instructions text,
    p_available_at timestamptz,
    p_due_at timestamptz,
    p_closes_at timestamptz,
    p_assessment_attempt_time_limit_seconds integer,
    p_assessment_attempt_limit integer,
    p_late_work_rule ple_data.late_work_rule,
    p_question_variation_rule ple_data.question_variation_rule,
    p_assessment_question_order_rule ple_data.question_order_rule,
    p_feedback_score ple_data.feedback_release,
    p_feedback_per_item_correctness ple_data.feedback_release,
    p_feedback_submitted_response ple_data.feedback_release,
    p_feedback_question_answer ple_data.feedback_release,
    p_feedback_question_answer_explanation ple_data.feedback_release,
    p_feedback_class_statistics ple_data.feedback_release,
    p_assessment_type ple_data.assessment_type
) RETURNS ple_data.sha256_digest
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_data, ple_private AS $$
DECLARE
    snapshot_id ple_data.sha256_digest;
    canonical_jsonb jsonb;
BEGIN
    IF p_assessment_type IN ('quiz', 'exam')
       AND p_assessment_attempt_limit IS DISTINCT FROM 1 THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Quiz and Exam assessments require exactly one Assessment Attempt';
    END IF;
    canonical_jsonb := jsonb_build_object(
        'assessment_title', p_assessment_title,
        'assessment_instructions', p_assessment_instructions,
        'available_at', to_jsonb(p_available_at),
        'due_at', to_jsonb(p_due_at),
        'closes_at', to_jsonb(p_closes_at),
        'assessment_attempt_time_limit_seconds', p_assessment_attempt_time_limit_seconds,
        'assessment_attempt_limit', p_assessment_attempt_limit,
        'late_work_rule', p_late_work_rule,
        'question_variation_rule', p_question_variation_rule,
        'assessment_question_order_rule', p_assessment_question_order_rule,
        'feedback_score', p_feedback_score,
        'feedback_per_item_correctness', p_feedback_per_item_correctness,
        'feedback_submitted_response', p_feedback_submitted_response,
        'feedback_question_answer', p_feedback_question_answer,
        'feedback_question_answer_explanation', p_feedback_question_answer_explanation,
        'feedback_class_statistics', p_feedback_class_statistics
    );
    snapshot_id := sha256(convert_to(canonical_jsonb::text, 'UTF8'));
    INSERT INTO ple_data.assessment_policy_snapshot (
        assessment_policy_snapshot_id,
        assessment_title,
        assessment_instructions,
        available_at,
        due_at,
        closes_at,
        assessment_attempt_time_limit_seconds,
        assessment_attempt_limit,
        late_work_rule,
        question_variation_rule,
        assessment_question_order_rule,
        feedback_score,
        feedback_per_item_correctness,
        feedback_submitted_response,
        feedback_question_answer,
        feedback_question_answer_explanation,
        feedback_class_statistics,
        created_at
    ) VALUES (
        snapshot_id,
        p_assessment_title,
        p_assessment_instructions,
        p_available_at,
        p_due_at,
        p_closes_at,
        p_assessment_attempt_time_limit_seconds,
        p_assessment_attempt_limit,
        p_late_work_rule,
        p_question_variation_rule,
        p_assessment_question_order_rule,
        p_feedback_score,
        p_feedback_per_item_correctness,
        p_feedback_submitted_response,
        p_feedback_question_answer,
        p_feedback_question_answer_explanation,
        p_feedback_class_statistics,
        clock_timestamp()
    ) ON CONFLICT (assessment_policy_snapshot_id) DO NOTHING;
    RETURN snapshot_id;
END
$$;
