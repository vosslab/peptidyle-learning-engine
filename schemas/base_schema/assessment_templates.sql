-- Instructor-owned reusable Assessment settings. Templates are private current
-- state outside Courses and Blueprints; they contain no schedule, lifecycle,
-- Question, Pool, point, or source fields.

SET LOCAL ROLE ple_private_owner;

-- Rust str::trim follows Unicode's White_Space property. PostgreSQL's POSIX
-- space class is locale-dependent and does not recognize every one of those
-- code points, so keep this exact model-bound set explicit and ASCII-encoded.
CREATE FUNCTION ple_private.assessment_template_name_is_valid(p_name text)
RETURNS boolean LANGUAGE sql IMMUTABLE
SET search_path = pg_catalog, ple_private AS $$
    SELECT p_name IS NOT NULL
       AND char_length(p_name) <= 200
       AND p_name = btrim(
           p_name,
           U&'\0009\000A\000B\000C\000D\0020\0085\00A0\1680\2000\2001\2002\2003'
               || U&'\2004\2005\2006\2007\2008\2009\200A\2028\2029\202F\205F\3000'
       )
       AND char_length(p_name) > 0
$$;

CREATE TABLE ple_private.assessment_template (
    assessment_template_id uuid PRIMARY KEY,
    owner_account_id uuid NOT NULL REFERENCES ple_private.account(account_id),
    assessment_template_edit_number bigint NOT NULL DEFAULT 1
        CHECK (assessment_template_edit_number > 0),
    template_name text NOT NULL CHECK (
        ple_private.assessment_template_name_is_valid(template_name)
    ),
    assessment_type text NOT NULL CHECK (assessment_type IN (
        'regular_assignment', 'practice_question_assignment', 'bonus_assignment', 'quiz', 'exam'
    )),
    instructions text NOT NULL CHECK (
        instructions !~ E'\\x00' AND char_length(instructions) <= 50000
    ),
    assessment_attempt_time_limit_seconds integer CHECK (
        assessment_attempt_time_limit_seconds IS NULL
        OR assessment_attempt_time_limit_seconds BETWEEN 1 AND 43200
    ),
    assessment_attempt_limit integer CHECK (
        assessment_attempt_limit IS NULL OR assessment_attempt_limit > 0
    ),
    late_work_rule text NOT NULL CHECK (late_work_rule IN ('accept', 'mark_late', 'reject')),
    question_variation_rule text NOT NULL CHECK (
        question_variation_rule IN ('reuse_variation', 'new_variation')
    ),
    assessment_question_order_rule text NOT NULL CHECK (
        assessment_question_order_rule IN ('authored_order', 'shuffled')
    ),
    feedback_score text NOT NULL CHECK (
        feedback_score IN ('during_attempt', 'after_submit', 'after_due', 'after_close', 'never')
    ),
    feedback_per_item_correctness text NOT NULL CHECK (
        feedback_per_item_correctness IN (
            'during_attempt', 'after_submit', 'after_due', 'after_close', 'never'
        )
    ),
    feedback_submitted_response text NOT NULL CHECK (
        feedback_submitted_response IN (
            'during_attempt', 'after_submit', 'after_due', 'after_close', 'never'
        )
    ),
    feedback_question_answer text NOT NULL CHECK (
        feedback_question_answer IN (
            'during_attempt', 'after_submit', 'after_due', 'after_close', 'never'
        )
    ),
    feedback_question_answer_explanation text NOT NULL CHECK (
        feedback_question_answer_explanation IN (
            'during_attempt', 'after_submit', 'after_due', 'after_close', 'never'
        )
    ),
    feedback_class_statistics text NOT NULL CHECK (
        feedback_class_statistics IN (
            'during_attempt', 'after_submit', 'after_due', 'after_close', 'never'
        )
    ),
    -- ASVS 2.2.1-2.2.3: mandatory variant fields cannot pass CHECK as unknown.
    CHECK (
        assessment_type NOT IN ('quiz', 'exam')
        OR (assessment_attempt_limit IS NOT NULL AND assessment_attempt_limit = 1)
    )
);

-- The owner predicate and stable list ordering are the complete collection
-- workload; this index avoids scanning other Instructors' private Templates.
CREATE INDEX assessment_template_owner_list_idx
    ON ple_private.assessment_template (
        owner_account_id, template_name, assessment_template_id
    );

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

ALTER TABLE ple_private.assessment_template ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.assessment_template FORCE ROW LEVEL SECURITY;
CREATE POLICY assessment_template_private_owner_access
    ON ple_private.assessment_template FOR ALL TO ple_private_owner
    USING (true) WITH CHECK (true);
CREATE POLICY assessment_template_api_owner_access
    ON ple_private.assessment_template FOR ALL TO ple_api_owner
    USING (
        owner_account_id = ple_api.current_session_account_id()
        AND ple_api.current_session_account_is_instructor()
    )
    WITH CHECK (
        owner_account_id = ple_api.current_session_account_id()
        AND ple_api.current_session_account_is_instructor()
    );

REVOKE ALL ON TABLE ple_private.assessment_template FROM PUBLIC;
REVOKE ALL ON FUNCTION ple_private.assessment_template_name_is_valid(text),
    ple_private.validate_assessment_template_settings(jsonb) FROM PUBLIC;
GRANT SELECT, INSERT ON TABLE ple_private.assessment_template TO ple_api_owner;
GRANT UPDATE (
    assessment_template_edit_number, template_name, assessment_type, instructions,
    assessment_attempt_time_limit_seconds, assessment_attempt_limit, late_work_rule,
    question_variation_rule,
    assessment_question_order_rule, feedback_score,
    feedback_per_item_correctness, feedback_submitted_response,
    feedback_question_answer, feedback_question_answer_explanation, feedback_class_statistics
) ON TABLE ple_private.assessment_template TO ple_api_owner;
GRANT EXECUTE ON FUNCTION ple_private.assessment_template_name_is_valid(text),
    ple_private.validate_assessment_template_settings(jsonb)
    TO ple_api_owner;

RESET ROLE;

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
SET search_path = pg_catalog, ple_api, ple_private AS $$
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
               'instructions', template.instructions,
               'assessmentAttemptTimeLimitSeconds',
                    template.assessment_attempt_time_limit_seconds,
               'attemptLimit', template.assessment_attempt_limit,
               'lateWorkRule', template.late_work_rule,
               'activityRules', jsonb_build_object(
                   'questionVariationRule', CASE template.question_variation_rule
                       WHEN 'reuse_variation' THEN 'reuseVariation' ELSE 'newVariation' END,
                   'assessmentQuestionOrderRule', CASE template.assessment_question_order_rule
                       WHEN 'authored_order' THEN 'authoredOrder' ELSE 'shuffled' END
               ),
               'studentFeedbackReleaseRule', jsonb_build_object(
                   'score', template.feedback_score,
                   'per_item_correctness', template.feedback_per_item_correctness,
                   'submitted_response', template.feedback_submitted_response,
                   'question_answer', template.feedback_question_answer,
                   'question_answer_explanation',
                        template.feedback_question_answer_explanation,
                   'class_statistics', template.feedback_class_statistics
               )
           ),
           template.assessment_template_edit_number
      FROM ple_private.assessment_template AS template
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
SET search_path = pg_catalog, ple_api, ple_private AS $$
DECLARE
    actor_id uuid;
    activity_rules jsonb;
    feedback_rules jsonb;
BEGIN
    IF NOT ple_api.current_session_account_is_instructor() THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Assessment Template is unavailable';
    END IF;
    actor_id := ple_api.current_session_account_id();
    PERFORM ple_private.validate_assessment_template_settings(p_settings);
    activity_rules := p_settings -> 'activityRules';
    feedback_rules := p_settings -> 'studentFeedbackReleaseRule';

    INSERT INTO ple_private.assessment_template (
        assessment_template_id, owner_account_id, template_name, assessment_type, instructions,
        assessment_attempt_time_limit_seconds, assessment_attempt_limit, late_work_rule,
        question_variation_rule,
        assessment_question_order_rule, feedback_score,
        feedback_per_item_correctness, feedback_submitted_response,
        feedback_question_answer, feedback_question_answer_explanation, feedback_class_statistics
    ) VALUES (
        p_assessment_template_id, actor_id, p_template_name, p_assessment_type,
        p_settings ->> 'instructions',
        (p_settings ->> 'assessmentAttemptTimeLimitSeconds')::integer,
        (p_settings ->> 'attemptLimit')::integer, p_settings ->> 'lateWorkRule',
        CASE activity_rules ->> 'questionVariationRule'
            WHEN 'reuseVariation' THEN 'reuse_variation' ELSE 'new_variation' END,
        CASE activity_rules ->> 'assessmentQuestionOrderRule'
            WHEN 'authoredOrder' THEN 'authored_order' ELSE 'shuffled' END,
        feedback_rules ->> 'score', feedback_rules ->> 'per_item_correctness',
        feedback_rules ->> 'submitted_response',
        feedback_rules ->> 'question_answer', feedback_rules ->> 'question_answer_explanation',
        feedback_rules ->> 'class_statistics'
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
SET search_path = pg_catalog, ple_api, ple_private AS $$
DECLARE
    activity_rules jsonb;
    current_edit_number bigint;
    feedback_rules jsonb;
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

    UPDATE ple_private.assessment_template AS template
       SET assessment_template_edit_number = template.assessment_template_edit_number + 1,
           template_name = p_template_name,
           assessment_type = p_assessment_type,
           instructions = p_settings ->> 'instructions',
           assessment_attempt_time_limit_seconds =
                (p_settings ->> 'assessmentAttemptTimeLimitSeconds')::integer,
           assessment_attempt_limit = (p_settings ->> 'attemptLimit')::integer,
           late_work_rule = p_settings ->> 'lateWorkRule',
           question_variation_rule = CASE activity_rules ->> 'questionVariationRule'
               WHEN 'reuseVariation' THEN 'reuse_variation' ELSE 'new_variation' END,
           assessment_question_order_rule = CASE activity_rules ->> 'assessmentQuestionOrderRule'
               WHEN 'authoredOrder' THEN 'authored_order' ELSE 'shuffled' END,
           feedback_score = feedback_rules ->> 'score',
           feedback_per_item_correctness = feedback_rules ->> 'per_item_correctness',
           feedback_submitted_response = feedback_rules ->> 'submitted_response',
           feedback_question_answer = feedback_rules ->> 'question_answer',
           feedback_question_answer_explanation =
                feedback_rules ->> 'question_answer_explanation',
           feedback_class_statistics = feedback_rules ->> 'class_statistics'
     WHERE template.assessment_template_id = p_assessment_template_id
       AND template.owner_account_id = ple_api.current_session_account_id();

    RETURN QUERY SELECT saved.*
      FROM ple_api.read_assessment_template(p_assessment_template_id) AS saved;
END
$$;

REVOKE ALL ON FUNCTION ple_api.list_assessment_templates(),
    ple_api.read_assessment_template(uuid),
    ple_api.create_assessment_template(uuid, text, text, jsonb),
    ple_api.save_assessment_template(uuid, bigint, text, text, jsonb)
    FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_api.list_assessment_templates(),
    ple_api.read_assessment_template(uuid),
    ple_api.create_assessment_template(uuid, text, text, jsonb),
    ple_api.save_assessment_template(uuid, bigint, text, text, jsonb)
    TO ple_app;

RESET ROLE;
