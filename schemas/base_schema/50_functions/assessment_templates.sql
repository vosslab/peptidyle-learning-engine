-- Functions, triggers, and views from assessment_templates.sql.

SET LOCAL ROLE ple_private_owner;

-- Instructor-owned reusable Assessment settings. Templates are private current
-- state outside Courses and Blueprints; they contain no schedule, lifecycle,
-- Question, Pool, point, or source fields.



-- Rust str::trim follows Unicode's White_Space property. PostgreSQL's POSIX
-- space class is locale-dependent and does not recognize every one of those
-- code points, so keep this exact model-bound set explicit and ASCII-encoded.
-- ASVS 1.5.2 and 2.1.1-2.2.3: the Store accepts the same closed JSON
-- representation as the Rust domain model. No SQL Type-default registry is
-- present; create receives settings constructed by the trusted server model.
CREATE FUNCTION ple_private.validate_assessment_template_settings(p_settings jsonb)
RETURNS void LANGUAGE plpgsql IMMUTABLE
SET search_path = pg_catalog, ple_private AS $$
DECLARE
    activity_rules jsonb;
    feedback_rules jsonb;
    integer_text text;
BEGIN
    IF p_settings IS NULL OR jsonb_typeof(p_settings) <> 'object'
       OR ARRAY(SELECT key FROM jsonb_object_keys(p_settings) AS key ORDER BY key)
            <> ARRAY[
                'activityRules', 'assessmentAttemptTimeLimitSeconds', 'attemptLimit',
                'instructions', 'lateWorkRule', 'studentFeedbackReleaseRule'
            ]::text[]
       OR jsonb_typeof(p_settings -> 'instructions') <> 'string'
       OR char_length(p_settings ->> 'instructions') > 50000
       OR jsonb_typeof(p_settings -> 'lateWorkRule') <> 'string'
       OR p_settings ->> 'lateWorkRule' NOT IN ('accept', 'mark_late', 'reject') THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Assessment Template settings are invalid';
    END IF;

    IF p_settings -> 'assessmentAttemptTimeLimitSeconds' <> 'null'::jsonb THEN
        IF jsonb_typeof(p_settings -> 'assessmentAttemptTimeLimitSeconds') <> 'number' THEN
            RAISE EXCEPTION USING ERRCODE = '22023',
                MESSAGE = 'Assessment Template settings are invalid';
        END IF;
        integer_text := p_settings ->> 'assessmentAttemptTimeLimitSeconds';
        IF integer_text !~ '^[1-9][0-9]*$'
           OR integer_text::numeric > 2147483647 THEN
            RAISE EXCEPTION USING ERRCODE = '22023',
                MESSAGE = 'Assessment Template settings are invalid';
        END IF;
    END IF;

    IF p_settings -> 'attemptLimit' <> 'null'::jsonb THEN
        IF jsonb_typeof(p_settings -> 'attemptLimit') <> 'number' THEN
            RAISE EXCEPTION USING ERRCODE = '22023',
                MESSAGE = 'Assessment Template settings are invalid';
        END IF;
        integer_text := p_settings ->> 'attemptLimit';
        IF integer_text !~ '^[1-9][0-9]*$'
           OR integer_text::numeric > 2147483647 THEN
            RAISE EXCEPTION USING ERRCODE = '22023',
                MESSAGE = 'Assessment Template settings are invalid';
        END IF;
    END IF;

    activity_rules := p_settings -> 'activityRules';
    IF jsonb_typeof(activity_rules) <> 'object'
       OR ARRAY(SELECT key FROM jsonb_object_keys(activity_rules) AS key ORDER BY key)
            <> ARRAY[
                'assessmentQuestionOrderRule', 'questionVariationRule'
            ]::text[] THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Assessment Template settings are invalid';
    END IF;

    -- ASVS 2.2.1: only the two configurable activity rules are accepted.
    IF jsonb_typeof(activity_rules -> 'questionVariationRule') <> 'string'
       OR activity_rules ->> 'questionVariationRule' NOT IN ('reuseVariation', 'newVariation')
       OR jsonb_typeof(activity_rules -> 'assessmentQuestionOrderRule') <> 'string'
       OR activity_rules ->> 'assessmentQuestionOrderRule' NOT IN ('authoredOrder', 'shuffled') THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Assessment Template settings are invalid';
    END IF;

    feedback_rules := p_settings -> 'studentFeedbackReleaseRule';
    IF jsonb_typeof(feedback_rules) <> 'object'
       OR ARRAY(SELECT key FROM jsonb_object_keys(feedback_rules) AS key ORDER BY key)
            <> ARRAY[
                'class_statistics', 'per_item_correctness', 'question_answer',
                'question_answer_explanation', 'score',
                'submitted_response'
            ]::text[]
       OR EXISTS (
            SELECT 1
              FROM jsonb_each(feedback_rules) AS feedback(key, value)
             WHERE jsonb_typeof(value) <> 'string'
                OR value #>> '{}' NOT IN (
                    'during_attempt', 'after_submit', 'after_due', 'after_close', 'never'
                )
       ) THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Assessment Template settings are invalid';
    END IF;
END
$$;

SET LOCAL ROLE ple_api_owner;







-- ASVS 1.2.4, 2.3.3, and 8.2.1-8.3.3: these fixed-shape functions are the
-- only runtime seam. Every operation derives the active Instructor from the
-- trusted session and owner filters conceal foreign Template existence.
CREATE FUNCTION ple_api.list_assessment_templates()
RETURNS TABLE (
    assessment_template_id uuid,
    template_name text,
    assessment_type text,
    settings jsonb,
    assessment_template_edit_number bigint
)
LANGUAGE plpgsql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private, ple_data AS $$
BEGIN
    IF NOT ple_api.current_session_account_is_instructor() THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Assessment Templates are unavailable';
    END IF;

    RETURN QUERY
    SELECT template.assessment_template_id,
           template.template_name,
           template.assessment_type,
           jsonb_build_object(
               'instructions', policy.assessment_instructions,
               'assessmentAttemptTimeLimitSeconds',
                    policy.assessment_attempt_time_limit_seconds,
               'attemptLimit', policy.assessment_attempt_limit,
               'lateWorkRule', policy.late_work_rule,
               'activityRules', jsonb_build_object(
                   'questionVariationRule', CASE policy.question_variation_rule
                       WHEN 'reuse_variation' THEN 'reuseVariation' ELSE 'newVariation' END,
                   'assessmentQuestionOrderRule', CASE policy.assessment_question_order_rule
                       WHEN 'authored_order' THEN 'authoredOrder' ELSE 'shuffled' END
               ),
               'studentFeedbackReleaseRule', jsonb_build_object(
                   'score', policy.feedback_score,
                   'per_item_correctness', policy.feedback_per_item_correctness,
                   'submitted_response', policy.feedback_submitted_response,
                   'question_answer', policy.feedback_question_answer,
                   'question_answer_explanation',
                        policy.feedback_question_answer_explanation,
                   'class_statistics', policy.feedback_class_statistics
               )
           ),
           template.assessment_template_edit_number
      FROM ple_private.assessment_template AS template
      JOIN ple_data.assessment_policy_snapshot AS policy
        ON policy.assessment_policy_snapshot_id = template.assessment_policy_snapshot_id
     WHERE template.owner_account_id = ple_api.current_session_account_id()
     ORDER BY template.template_name, template.assessment_template_id;
END
$$;

CREATE FUNCTION ple_api.read_assessment_template(p_assessment_template_id uuid)
RETURNS TABLE (
    assessment_template_id uuid,
    template_name text,
    assessment_type text,
    settings jsonb,
    assessment_template_edit_number bigint
)
LANGUAGE plpgsql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private AS $$
BEGIN
    IF NOT ple_api.current_session_account_is_instructor() THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Assessment Template is unavailable';
    END IF;

    RETURN QUERY
    SELECT listed.*
      FROM ple_api.list_assessment_templates() AS listed
     WHERE listed.assessment_template_id = p_assessment_template_id;
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Assessment Template is unavailable';
    END IF;
END
$$;

CREATE FUNCTION ple_api.create_assessment_template(
    p_assessment_template_id uuid,
    p_template_name text,
    p_assessment_type text,
    p_settings jsonb
)
RETURNS TABLE (
    assessment_template_id uuid,
    template_name text,
    assessment_type text,
    settings jsonb,
    assessment_template_edit_number bigint
)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private, ple_data AS $$
DECLARE
    actor_id text;
    activity_rules jsonb;
    feedback_rules jsonb;
    snapshot_id ple_data.sha256_digest;
BEGIN
    IF NOT ple_api.current_session_account_is_instructor() THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Assessment Template is unavailable';
    END IF;
    actor_id := ple_api.current_session_account_id();
    PERFORM ple_private.validate_assessment_template_settings(p_settings);
    activity_rules := p_settings -> 'activityRules';
    feedback_rules := p_settings -> 'studentFeedbackReleaseRule';
    snapshot_id := ple_private.ensure_assessment_policy_snapshot(
        p_template_name,
        p_settings ->> 'instructions',
        NULL, NULL, NULL,
        (p_settings ->> 'assessmentAttemptTimeLimitSeconds')::integer,
        (p_settings ->> 'attemptLimit')::integer,
        (p_settings ->> 'lateWorkRule')::ple_data.late_work_rule,
        CASE activity_rules ->> 'questionVariationRule'
            WHEN 'reuseVariation' THEN 'reuse_variation' ELSE 'new_variation' END,
        CASE activity_rules ->> 'assessmentQuestionOrderRule'
            WHEN 'authoredOrder' THEN 'authored_order' ELSE 'shuffled' END,
        (feedback_rules ->> 'score')::ple_data.feedback_release,
        (feedback_rules ->> 'per_item_correctness')::ple_data.feedback_release,
        (feedback_rules ->> 'submitted_response')::ple_data.feedback_release,
        (feedback_rules ->> 'question_answer')::ple_data.feedback_release,
        (feedback_rules ->> 'question_answer_explanation')::ple_data.feedback_release,
        (feedback_rules ->> 'class_statistics')::ple_data.feedback_release,
        p_assessment_type::ple_data.assessment_type
    );

    INSERT INTO ple_private.assessment_template (
        assessment_template_id, owner_account_id, template_name, assessment_type,
        assessment_policy_snapshot_id
    ) VALUES (
        p_assessment_template_id, actor_id, p_template_name, p_assessment_type, snapshot_id
    );

    RETURN QUERY SELECT created.*
      FROM ple_api.read_assessment_template(p_assessment_template_id) AS created;
END
$$;

CREATE FUNCTION ple_api.save_assessment_template(
    p_assessment_template_id uuid,
    p_expected_edit_number bigint,
    p_template_name text,
    p_assessment_type text,
    p_settings jsonb
)
RETURNS TABLE (
    assessment_template_id uuid,
    template_name text,
    assessment_type text,
    settings jsonb,
    assessment_template_edit_number bigint
)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private, ple_data AS $$
DECLARE
    activity_rules jsonb;
    current_edit_number bigint;
    feedback_rules jsonb;
    snapshot_id ple_data.sha256_digest;
BEGIN
    IF NOT ple_api.current_session_account_is_instructor() THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Assessment Template is unavailable';
    END IF;

    SELECT template.assessment_template_edit_number INTO current_edit_number
      FROM ple_private.assessment_template AS template
     WHERE template.assessment_template_id = p_assessment_template_id
       AND template.owner_account_id = ple_api.current_session_account_id()
     FOR UPDATE;
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Assessment Template is unavailable';
    END IF;
    IF p_expected_edit_number IS NULL OR p_expected_edit_number <= 0 THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Assessment Template Edit Number is invalid';
    END IF;
    IF current_edit_number <> p_expected_edit_number THEN
        RAISE EXCEPTION USING ERRCODE = '40001',
            MESSAGE = 'Assessment Template changed before this save';
    END IF;

    PERFORM ple_private.validate_assessment_template_settings(p_settings);
    activity_rules := p_settings -> 'activityRules';
    feedback_rules := p_settings -> 'studentFeedbackReleaseRule';
    snapshot_id := ple_private.ensure_assessment_policy_snapshot(
        p_template_name,
        p_settings ->> 'instructions',
        NULL, NULL, NULL,
        (p_settings ->> 'assessmentAttemptTimeLimitSeconds')::integer,
        (p_settings ->> 'attemptLimit')::integer,
        (p_settings ->> 'lateWorkRule')::ple_data.late_work_rule,
        CASE activity_rules ->> 'questionVariationRule'
            WHEN 'reuseVariation' THEN 'reuse_variation' ELSE 'new_variation' END,
        CASE activity_rules ->> 'assessmentQuestionOrderRule'
            WHEN 'authoredOrder' THEN 'authored_order' ELSE 'shuffled' END,
        (feedback_rules ->> 'score')::ple_data.feedback_release,
        (feedback_rules ->> 'per_item_correctness')::ple_data.feedback_release,
        (feedback_rules ->> 'submitted_response')::ple_data.feedback_release,
        (feedback_rules ->> 'question_answer')::ple_data.feedback_release,
        (feedback_rules ->> 'question_answer_explanation')::ple_data.feedback_release,
        (feedback_rules ->> 'class_statistics')::ple_data.feedback_release,
        p_assessment_type::ple_data.assessment_type
    );

    UPDATE ple_private.assessment_template AS template
       SET assessment_template_edit_number = template.assessment_template_edit_number + 1,
           template_name = p_template_name,
           assessment_type = p_assessment_type,
           assessment_policy_snapshot_id = snapshot_id
     WHERE template.assessment_template_id = p_assessment_template_id
       AND template.owner_account_id = ple_api.current_session_account_id();

    RETURN QUERY SELECT saved.*
      FROM ple_api.read_assessment_template(p_assessment_template_id) AS saved;
END
$$;

