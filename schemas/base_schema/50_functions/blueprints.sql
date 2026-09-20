-- Functions, triggers, and views from blueprints.sql.

SET LOCAL ROLE ple_data_owner;



CREATE TRIGGER blueprint_course_public_id_is_minted
BEFORE INSERT ON ple_data.blueprint_course
FOR EACH ROW EXECUTE FUNCTION ple_private.assign_public_id('BP');

CREATE FUNCTION ple_data.blueprint_content_question_pins(p_content jsonb)
RETURNS TABLE (content_path text, published_question_id text, question_revision_number bigint)
LANGUAGE sql IMMUTABLE STRICT
SET search_path = pg_catalog, ple_data
AS $$
    WITH entries AS (
        SELECT module_ordinality, assessment_ordinality, entry_ordinality, entry
          FROM pg_catalog.jsonb_array_elements(p_content -> 'modules')
                 WITH ORDINALITY AS module_row(module, module_ordinality)
          CROSS JOIN LATERAL pg_catalog.jsonb_array_elements(module_row.module -> 'assessments')
                 WITH ORDINALITY AS assessment_row(assessment, assessment_ordinality)
          CROSS JOIN LATERAL pg_catalog.jsonb_array_elements(
              assessment_row.assessment -> 'content' -> 'entries'
          ) WITH ORDINALITY AS entry_row(entry, entry_ordinality)
    ), pins AS (
        SELECT module_ordinality, assessment_ordinality, entry_ordinality,
               entry -> 'question_revision_tuple' AS pin
          FROM entries WHERE entry ->> 'kind' = 'fixed'
    )
    SELECT pg_catalog.format('m%s.a%s.e%s.p%s', module_ordinality,
               assessment_ordinality, entry_ordinality, 0),
           pin ->> 'questionId',
           (pin ->> 'revisionNumber')::bigint
      FROM pins
$$;



-- A Pool entry names one current Question Pool by Pool ID. Ordered Question
-- Revision member pins live in current Pool membership, rather than being
-- copied into Blueprint JSON as a second representation.
CREATE FUNCTION ple_data.blueprint_content_pools(p_content jsonb)
RETURNS TABLE (content_path text, question_pool_id text)
LANGUAGE sql IMMUTABLE STRICT
SET search_path = pg_catalog, ple_data
AS $$
    SELECT pg_catalog.format('m%s.a%s.e%s', module_ordinality,
               assessment_ordinality, entry_ordinality),
           entry ->> 'question_pool_id'
      FROM pg_catalog.jsonb_array_elements(p_content -> 'modules')
             WITH ORDINALITY AS module_row(module, module_ordinality)
      CROSS JOIN LATERAL pg_catalog.jsonb_array_elements(module_row.module -> 'assessments')
             WITH ORDINALITY AS assessment_row(assessment, assessment_ordinality)
      CROSS JOIN LATERAL pg_catalog.jsonb_array_elements(
          assessment_row.assessment -> 'content' -> 'entries'
      ) WITH ORDINALITY AS entry_row(entry, entry_ordinality)
     WHERE entry ->> 'kind' = 'pool'
$$;

CREATE FUNCTION ple_data.blueprint_content_assessments(p_content jsonb)
RETURNS TABLE (
    blueprint_module_id uuid, blueprint_assessment_id uuid,
    module_position integer, assessment_position integer
)
LANGUAGE sql IMMUTABLE STRICT
SET search_path = pg_catalog, ple_data
AS $$
    SELECT (module_row.module ->> 'blueprint_module_id')::uuid,
           (assessment_row.assessment ->> 'blueprint_assessment_id')::uuid,
           module_row.module_ordinality::integer,
           assessment_row.assessment_ordinality::integer
      FROM pg_catalog.jsonb_array_elements(p_content -> 'modules')
             WITH ORDINALITY AS module_row(module, module_ordinality)
      CROSS JOIN LATERAL pg_catalog.jsonb_array_elements(module_row.module -> 'assessments')
             WITH ORDINALITY AS assessment_row(assessment, assessment_ordinality)
$$;

CREATE FUNCTION ple_data.blueprint_content_modules(p_content jsonb)
RETURNS TABLE (blueprint_module_id uuid, module_position integer)
LANGUAGE sql IMMUTABLE STRICT
SET search_path = pg_catalog, ple_data
AS $$
    SELECT (module_row.module ->> 'blueprint_module_id')::uuid,
           module_row.module_ordinality::integer
      FROM pg_catalog.jsonb_array_elements(p_content -> 'modules')
             WITH ORDINALITY AS module_row(module, module_ordinality)
$$;



-- Blueprint content is a closed, bounded reusable-curriculum shape.  Course
-- Instance delivery, schedule, roster, response, and grade state have no
-- extension point here: the Store's typed codec and this persistence boundary
-- agree on every object node rather than relying on a growing deny-list.
CREATE FUNCTION ple_data.blueprint_content_has_exact_keys(p_value jsonb, p_keys text[])
RETURNS boolean LANGUAGE sql IMMUTABLE STRICT
SET search_path = pg_catalog, ple_data
AS $$
    SELECT CASE WHEN jsonb_typeof(p_value) <> 'object' THEN false ELSE
        NOT EXISTS (
            SELECT 1 FROM jsonb_object_keys(p_value) AS actual WHERE actual <> ALL (p_keys)
        )
        AND NOT EXISTS (SELECT 1 FROM unnest(p_keys) AS required WHERE NOT p_value ? required)
    END
$$;

CREATE FUNCTION ple_data.blueprint_content_is_closed(p_content jsonb)
RETURNS boolean LANGUAGE plpgsql IMMUTABLE STRICT
SET search_path = pg_catalog, ple_data
AS $$
DECLARE
    module_value jsonb;
    assessment_value jsonb;
    content_value jsonb;
    entry_value jsonb;
    pin_value jsonb;
    defaults_value jsonb;
    activity_value jsonb;
    feedback_value jsonb;
    limit_value jsonb;
    time_limit_value jsonb;
    selection_value jsonb;
    delivered_question_count bigint;
BEGIN
    IF NOT ple_data.blueprint_content_has_exact_keys(p_content, ARRAY['modules'])
       OR jsonb_typeof(p_content -> 'modules') <> 'array'
       OR jsonb_array_length(p_content -> 'modules') > 1024 THEN
        RETURN false;
    END IF;
    FOR module_value IN SELECT value FROM jsonb_array_elements(p_content -> 'modules') LOOP
        IF NOT ple_data.blueprint_content_has_exact_keys(
            module_value, ARRAY['blueprint_module_id', 'label', 'assessments']
        ) OR jsonb_typeof(module_value -> 'blueprint_module_id') <> 'string'
          OR jsonb_typeof(module_value -> 'label') <> 'string'
          OR jsonb_typeof(module_value -> 'assessments') <> 'array'
          OR jsonb_array_length(module_value -> 'assessments') NOT BETWEEN 1 AND 1024 THEN
            RETURN false;
        END IF;
        FOR assessment_value IN
            SELECT value FROM jsonb_array_elements(module_value -> 'assessments') LOOP
            IF NOT ple_data.blueprint_content_has_exact_keys(
                assessment_value, ARRAY['blueprint_assessment_id', 'content']
            ) OR jsonb_typeof(assessment_value -> 'blueprint_assessment_id') <> 'string'
              OR jsonb_typeof(assessment_value -> 'content') <> 'object' THEN
                RETURN false;
            END IF;
            content_value := assessment_value -> 'content';
            IF NOT ple_data.blueprint_content_has_exact_keys(
                content_value, ARRAY['assessment_type', 'title', 'instructions', 'entries', 'defaults']
            ) OR content_value ->> 'assessment_type' NOT IN (
                'regular_assignment', 'practice_question_assignment', 'bonus_assignment', 'quiz', 'exam'
            ) OR jsonb_typeof(content_value -> 'title') <> 'string'
              OR jsonb_typeof(content_value -> 'instructions') <> 'string'
              OR jsonb_typeof(content_value -> 'entries') <> 'array'
              OR jsonb_array_length(content_value -> 'entries') > 1024 THEN
                RETURN false;
            END IF;
            defaults_value := content_value -> 'defaults';
            IF NOT ple_data.blueprint_content_has_exact_keys(defaults_value, ARRAY[
                'assessment_attempt_time_limit_seconds', 'assessment_attempt_limit', 'late_work_rule',
                'activity_rules', 'student_feedback_release_rule'
            ]) OR (
                content_value ->> 'assessment_type' IN ('quiz', 'exam')
                AND defaults_value -> 'assessment_attempt_limit' IS DISTINCT FROM '1'::jsonb
            ) THEN
                RETURN false;
            END IF;
            -- ASVS 2.2.1, 2.2.2: NULL means the calculated default, never Unlimited.
            IF defaults_value -> 'assessment_attempt_time_limit_seconds' <> 'null'::jsonb THEN
                IF jsonb_typeof(defaults_value -> 'assessment_attempt_time_limit_seconds') <> 'number'
                   OR defaults_value ->> 'assessment_attempt_time_limit_seconds' !~ '^[1-9][0-9]*$'
                   OR (defaults_value ->> 'assessment_attempt_time_limit_seconds')::numeric > 43200 THEN
                    RETURN false;
                END IF;
            END IF;
            delivered_question_count := 0;
            activity_value := defaults_value -> 'activity_rules';
            feedback_value := defaults_value -> 'student_feedback_release_rule';
            IF NOT ple_data.blueprint_content_has_exact_keys(activity_value, ARRAY[
                'questionVariationRule',
                'assessmentQuestionOrderRule'
            ]) OR NOT ple_data.blueprint_content_has_exact_keys(feedback_value, ARRAY[
                'score', 'per_item_correctness', 'submitted_response',
                'question_answer', 'question_answer_explanation', 'class_statistics'
            ]) THEN
                RETURN false;
            END IF;
            FOR entry_value IN SELECT value FROM jsonb_array_elements(content_value -> 'entries') LOOP
                IF entry_value ->> 'kind' = 'fixed' THEN
                    delivered_question_count := delivered_question_count + 1;
                    IF NOT ple_data.blueprint_content_has_exact_keys(entry_value, ARRAY[
                        'kind', 'question_revision_tuple', 'points_possible', 'scoring_rule',
                        'question_attempt_limit', 'question_attempt_time_limit'
                    ]) THEN RETURN false; END IF;
                    pin_value := entry_value -> 'question_revision_tuple';
                    IF NOT ple_data.blueprint_content_has_exact_keys(
                        pin_value, ARRAY['questionId', 'revisionNumber']
                    ) THEN RETURN false; END IF;
                ELSIF entry_value ->> 'kind' = 'pool' THEN
                    IF entry_value ->> 'selection_count' !~ '^[1-9][0-9]*$'
                       OR entry_value ->> 'selection_count' IS NULL THEN RETURN false; END IF;
                    IF (entry_value ->> 'selection_count')::numeric > 250 THEN RETURN false; END IF;
                    delivered_question_count := delivered_question_count +
                        (entry_value ->> 'selection_count')::bigint;
                    IF NOT ple_data.blueprint_content_has_exact_keys(entry_value, ARRAY[
                        'kind', 'question_pool_id', 'question_pool_edit_number',
                        'selection_count', 'points_per_item',
                        'scoring_rule', 'selection_rule', 'question_attempt_limit',
                        'question_attempt_time_limit'
                    ]) THEN
                        RETURN false;
                    END IF;
                    IF entry_value ->> 'question_pool_id' IS NULL
                       OR jsonb_typeof(entry_value -> 'question_pool_edit_number')
                            NOT IN ('number', 'string') THEN
                        RETURN false;
                    END IF;
                    selection_value := entry_value -> 'selection_rule';
                    IF NOT ple_data.blueprint_content_has_exact_keys(
                        selection_value, ARRAY['selectedQuestionOrder']
                    ) THEN RETURN false; END IF;
                ELSE
                    RETURN false;
                END IF;
                IF delivered_question_count > 250 THEN RETURN false; END IF;
                limit_value := entry_value -> 'question_attempt_limit';
                time_limit_value := entry_value -> 'question_attempt_time_limit';
                IF NOT ple_data.blueprint_content_has_exact_keys(limit_value, ARRAY['maxAttempts'])
                   OR NOT (
                       ple_data.blueprint_content_has_exact_keys(time_limit_value, ARRAY['kind'])
                       OR ple_data.blueprint_content_has_exact_keys(
                           time_limit_value, ARRAY['kind', 'seconds', 'graceSeconds']
                       )
                   ) THEN
                    RETURN false;
                END IF;
            END LOOP;
        END LOOP;
    END LOOP;
    RETURN true;
END
$$;

CREATE FUNCTION ple_data.validate_blueprint_content(p_content jsonb)
RETURNS void LANGUAGE plpgsql IMMUTABLE STRICT
SET search_path = pg_catalog, ple_data
AS $$
BEGIN
    IF NOT ple_data.blueprint_content_is_closed(p_content)
       OR p_content ? 'title'
       OR jsonb_typeof(p_content -> 'modules') <> 'array'
       OR jsonb_array_length(p_content -> 'modules') > 1024
       OR EXISTS (
           SELECT 1
             FROM jsonb_array_elements(p_content -> 'modules') AS module_row(module)
            WHERE jsonb_typeof(module_row.module) <> 'object'
               OR module_row.module ->> 'blueprint_module_id' IS NULL
               OR module_row.module ->> 'label' IS NULL
               OR module_row.module ->> 'label' <> btrim(module_row.module ->> 'label')
               OR char_length(module_row.module ->> 'label') NOT BETWEEN 1 AND 500
               OR jsonb_typeof(module_row.module -> 'assessments') <> 'array'
               OR jsonb_array_length(module_row.module -> 'assessments') NOT BETWEEN 1 AND 1024
       )
       OR (SELECT count(*) FROM ple_data.blueprint_content_modules(p_content))
          <> (SELECT count(DISTINCT blueprint_module_id)
                FROM ple_data.blueprint_content_modules(p_content))
       OR EXISTS (
           SELECT 1
             FROM jsonb_array_elements(p_content -> 'modules') AS module_row(module)
             CROSS JOIN LATERAL jsonb_array_elements(module_row.module -> 'assessments')
                 AS assessment_row(assessment)
            WHERE jsonb_typeof(assessment_row.assessment) <> 'object'
               OR assessment_row.assessment ->> 'blueprint_assessment_id' IS NULL
               OR jsonb_typeof(assessment_row.assessment -> 'content') <> 'object'
               OR jsonb_typeof(assessment_row.assessment -> 'content' -> 'entries') <> 'array'
       )
       OR (SELECT count(*) FROM ple_data.blueprint_content_assessments(p_content))
          <> (SELECT count(DISTINCT blueprint_assessment_id)
                FROM ple_data.blueprint_content_assessments(p_content))
       OR EXISTS (
           SELECT 1
             FROM jsonb_array_elements(p_content -> 'modules') AS module_row(module)
             CROSS JOIN LATERAL jsonb_array_elements(module_row.module -> 'assessments')
                 AS assessment_row(assessment)
             CROSS JOIN LATERAL jsonb_array_elements(
                 assessment_row.assessment -> 'content' -> 'entries'
             ) AS entry_row(entry)
            WHERE entry_row.entry ->> 'kind' IS NULL
               OR entry_row.entry ->> 'kind' NOT IN ('fixed', 'pool')
               OR (entry_row.entry ->> 'kind' = 'fixed'
                   AND jsonb_typeof(entry_row.entry -> 'question_revision_tuple') <> 'object')
               OR (entry_row.entry ->> 'kind' = 'pool'
                   AND (entry_row.entry ->> 'question_pool_id' IS NULL
                        OR entry_row.entry -> 'question_pool_edit_number' IS NULL))
       ) THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Blueprint Course content is invalid';
    END IF;
END
$$;



-- New pins must select an Available Published Question. A Save may retain an
-- exact pin already owned by its expected immutable head Revision.
CREATE FUNCTION ple_data.validate_blueprint_question_selection(
    p_existing_blueprint_course_instance_id text,
    p_existing_blueprint_revision_number bigint,
    p_content jsonb
) RETURNS void
LANGUAGE plpgsql VOLATILE SECURITY DEFINER
SET search_path = pg_catalog, ple_data
AS $$
BEGIN
    PERFORM 1
      FROM ple_data.published_question AS question
      JOIN ple_data.blueprint_content_question_pins(p_content) AS pin
        ON pin.published_question_id = question.published_question_id
     FOR KEY SHARE OF question;
    IF EXISTS (
        SELECT 1
          FROM ple_data.blueprint_content_question_pins(p_content) AS pin
          LEFT JOIN ple_data.published_question AS question
            ON question.published_question_id = pin.published_question_id
          LEFT JOIN ple_data.question_revision AS revision
            ON revision.published_question_id = pin.published_question_id
           AND revision.revision_number = pin.question_revision_number
          LEFT JOIN ple_data.blueprint_revision_question_pin AS existing
            ON existing.blueprint_course_id
                 = p_existing_blueprint_course_instance_id
           AND existing.blueprint_revision_number = p_existing_blueprint_revision_number
           AND existing.published_question_id = pin.published_question_id
           AND existing.question_revision_number = pin.question_revision_number
         WHERE (question.availability IS DISTINCT FROM 'available'
                OR revision.published_question_id IS NULL)
           AND existing.blueprint_course_id IS NULL
    ) THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'New Blueprint Question pins require an Available Question';
    END IF;
END
$$;



-- The source Course and source Revision are permanent ancestry facts.  The
-- forked Blueprint's own content remains independently editable through its
-- ordinary immutable Revision sequence.
-- ASVS 8.2.2, 8.3.1: enforce this data-specific boundary in trusted PostgreSQL.
CREATE FUNCTION ple_data.reject_blueprint_course_fork_change()
RETURNS trigger LANGUAGE plpgsql
SET search_path = pg_catalog, ple_data AS $$
BEGIN
    RAISE EXCEPTION USING ERRCODE = '55000',
        MESSAGE = 'Blueprint Course fork origin is immutable';
END
$$;

CREATE TRIGGER blueprint_course_fork_origin_is_immutable
BEFORE UPDATE OR DELETE ON ple_data.blueprint_course_fork
FOR EACH ROW EXECUTE FUNCTION ple_data.reject_blueprint_course_fork_change();

