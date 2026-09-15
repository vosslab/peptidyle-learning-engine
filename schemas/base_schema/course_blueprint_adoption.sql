-- Atomic initial teaching content, materialized by the Store from the exact Blueprint.
SET LOCAL ROLE ple_api_owner;

CREATE FUNCTION ple_api.load_course_instance_blueprint(p_reference bigint, p_revision bigint)
RETURNS TABLE(content jsonb, content_checksum bytea)
LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data AS $$
    SELECT revision.content, revision.content_checksum
      FROM ple_data.blueprint_course AS blueprint
      JOIN ple_data.blueprint_course_revision AS revision
        ON revision.blueprint_course_reference_number = blueprint.reference_number
       AND revision.blueprint_revision_number = p_revision
     WHERE blueprint.reference_number = p_reference
       -- C49 defines Public as the only reusable state. C73 owns the
       -- separate adoption-default structure below.
       AND blueprint.availability = 'public'
       AND (ple_api.current_session_account_is_instructor()
            OR ple_api.current_session_account_is_sysadmin())
$$;

REVOKE ALL ON FUNCTION ple_api.load_course_instance_blueprint(bigint, bigint) FROM PUBLIC;
-- The data-owner adoption validator has no broad read policy on forced-RLS
-- Blueprint relations. It may invoke this already lifecycle-gated exact
-- Revision reader while materializing its child transaction.
GRANT EXECUTE ON FUNCTION ple_api.load_course_instance_blueprint(bigint, bigint) TO ple_data_owner;

-- Only the typed Store receives human-facing Blueprint references.  The
-- data-owner adoption validator above retains its internal numeric reader.
CREATE FUNCTION ple_api.load_course_instance_blueprint(p_reference text, p_revision bigint)
RETURNS TABLE(content jsonb, content_checksum bytea)
LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data AS $$
    SELECT * FROM ple_api.load_course_instance_blueprint(
        (SELECT reference_number FROM ple_data.blueprint_course WHERE public_reference = $1),
        $2
    )
$$;
REVOKE ALL ON FUNCTION ple_api.load_course_instance_blueprint(text, bigint) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_api.load_course_instance_blueprint(text, bigint) TO ple_app;
RESET ROLE;

SET LOCAL ROLE ple_data_owner;

-- ASVS 2.3.1 and 2.3.3: course adoption is one atomic business operation.
-- The Store may mint fresh Course-side identities, but it must not choose any
-- persisted curriculum or policy value.  Compare that proposed materialization
-- to the sealed Blueprint Revision here, before any Course child exists.
CREATE FUNCTION ple_data.validate_course_blueprint_adoption(
    p_blueprint_reference bigint, p_blueprint_revision bigint, p_assessments jsonb
)
RETURNS void LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_data AS $$
DECLARE
    member jsonb;
    source_content jsonb;
    source_entry jsonb;
    proposed_entry jsonb;
    entry_index integer;
BEGIN
    IF jsonb_typeof(p_assessments) IS DISTINCT FROM 'array' THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Course initial assessments are invalid';
    END IF;
    FOR member IN SELECT value FROM jsonb_array_elements(p_assessments) LOOP
        SELECT assessment_row.assessment -> 'content' INTO source_content
          FROM ple_api.load_course_instance_blueprint(
              p_blueprint_reference, p_blueprint_revision
          ) AS revision
          CROSS JOIN LATERAL jsonb_array_elements(revision.content -> 'modules')
              AS module_row(module)
          CROSS JOIN LATERAL jsonb_array_elements(module_row.module -> 'assessments')
              AS assessment_row(assessment)
         WHERE assessment_row.assessment ->> 'blueprint_assessment_reference' = member ->> 'source';
        IF source_content IS NULL
           OR jsonb_typeof(member -> 'values') <> 'object'
           OR jsonb_typeof(member -> 'entries') <> 'array' THEN
            RAISE EXCEPTION USING ERRCODE = '22023',
                MESSAGE = 'Blueprint adoption content is invalid';
        END IF;
        IF member -> 'values' IS DISTINCT FROM jsonb_build_object(
            'assessment_title', source_content -> 'title',
            'assessment_instructions', source_content -> 'instructions',
            'available_at', NULL, 'due_at', NULL, 'closes_at', NULL,
            'assessment_attempt_time_limit_seconds',
                source_content #> '{defaults,assessment_attempt_time_limit_seconds}',
            'assessment_attempt_limit', source_content #> '{defaults,assessment_attempt_limit}',
            'late_work_rule', source_content #> '{defaults,late_work_rule}',
            'assessment_completion_rule', CASE (source_content #>> '{defaults,activity_rules,assessmentCompletionRule,kind}')
                WHEN 'answerAll' THEN 'answer_all' WHEN 'allCorrect' THEN 'all_correct'
                WHEN 'scoreAtLeast' THEN 'score_at_least' END,
            'assessment_completion_score_threshold',
                source_content #> '{defaults,activity_rules,assessmentCompletionRule,fraction}',
            'assessment_attempt_grade_rule', CASE (source_content #>> '{defaults,activity_rules,assessmentAttemptGradeRule}')
                WHEN 'first' THEN 'first' WHEN 'latest' THEN 'latest' WHEN 'highest' THEN 'highest'
                WHEN 'instructorSelected' THEN 'instructor_selected' END,
            'assessment_attempt_continuation_rule', CASE (source_content #>> '{defaults,activity_rules,assessmentAttemptContinuationRule,kind}')
                WHEN 'unlimited' THEN 'unlimited' WHEN 'capped' THEN 'capped' WHEN 'closed' THEN 'closed' END,
            'max_additional_assessment_attempts',
                source_content #> '{defaults,activity_rules,assessmentAttemptContinuationRule,maxAdditionalAssessmentAttempts}',
            'question_pool_reuse_rule', CASE (source_content #>> '{defaults,activity_rules,questionPoolReuseRule}')
                WHEN 'reuseSelection' THEN 'reuse_selection' WHEN 'selectAgain' THEN 'select_again' END,
            'question_variation_rule', CASE (source_content #>> '{defaults,activity_rules,questionVariationRule}')
                WHEN 'reuseVariation' THEN 'reuse_variation' WHEN 'newVariation' THEN 'new_variation' END,
            'assessment_attempt_resume_rule', CASE (source_content #>> '{defaults,activity_rules,assessmentAttemptResumeRule}')
                WHEN 'resumable' THEN 'resumable' WHEN 'singleSession' THEN 'single_session' END,
            'assessment_question_display_rule', CASE (source_content #>> '{defaults,activity_rules,assessmentQuestionDisplayRule}')
                WHEN 'allQuestions' THEN 'all_questions' WHEN 'oneQuestionAtATime' THEN 'one_question_at_a_time' END,
            'assessment_navigation_rule', CASE (source_content #>> '{defaults,activity_rules,assessmentNavigationRule}')
                WHEN 'freeNavigation' THEN 'free_navigation' WHEN 'forwardOnly' THEN 'forward_only' END,
            'assessment_question_order_rule', CASE (source_content #>> '{defaults,activity_rules,assessmentQuestionOrderRule}')
                WHEN 'authoredOrder' THEN 'authored_order' WHEN 'shuffled' THEN 'shuffled' END,
            'feedback_score', source_content #> '{defaults,student_feedback_release_rule,score}',
            'feedback_per_item_correctness', source_content #> '{defaults,student_feedback_release_rule,per_item_correctness}',
            'feedback_submitted_response', source_content #> '{defaults,student_feedback_release_rule,submitted_response}',
            'feedback_question_feedback', source_content #> '{defaults,student_feedback_release_rule,question_feedback}',
            'feedback_question_answer', source_content #> '{defaults,student_feedback_release_rule,question_answer}',
            'feedback_question_answer_explanation', source_content #> '{defaults,student_feedback_release_rule,question_answer_explanation}',
            'feedback_class_statistics', source_content #> '{defaults,student_feedback_release_rule,class_statistics}'
        ) THEN
            RAISE EXCEPTION USING ERRCODE = '22023',
                MESSAGE = 'Blueprint adoption policy differs from its exact Revision';
        END IF;
        IF jsonb_array_length(member -> 'entries') <> jsonb_array_length(source_content -> 'entries') THEN
            RAISE EXCEPTION USING ERRCODE = '22023',
                MESSAGE = 'Blueprint adoption Entries differ from its exact Revision';
        END IF;
        FOR source_entry, entry_index IN
            SELECT value, ordinality::integer - 1
              FROM jsonb_array_elements(source_content -> 'entries') WITH ORDINALITY
        LOOP
            proposed_entry := member -> 'entries' -> entry_index;
            IF source_entry ->> 'kind' = 'fixed' THEN
                IF proposed_entry ->> 'kind' <> 'fixed_question'
                   OR proposed_entry ->> 'availability' <> 'available'
                   OR proposed_entry ->> 'authoredPosition' <> entry_index::text
                   -- C842/C843 store Question IDs compactly. The sealed JSON
                   -- uses the model's display serialization (AAAA-ZBBB), so
                   -- remove only that presentation separator at this trusted
                   -- persistence seam; this is not a legacy input parser.
                   OR proposed_entry ->> 'questionId' IS DISTINCT FROM
                        replace(source_entry #>> '{question_revision,questionId}', '-', '')
                   OR proposed_entry ->> 'revisionNumber' IS DISTINCT FROM source_entry #>> '{question_revision,revisionNumber}'
                   OR proposed_entry ->> 'pointsPossible' IS DISTINCT FROM source_entry ->> 'points_possible'
                   OR proposed_entry ->> 'scoringRule' IS DISTINCT FROM (CASE (source_entry ->> 'scoring_rule')
                        WHEN 'normal' THEN 'normal' WHEN 'fullCredit' THEN 'full_credit'
                        WHEN 'extraCredit' THEN 'extra_credit' WHEN 'excluded' THEN 'excluded' END)
                   OR proposed_entry ->> 'questionAttemptLimit' IS DISTINCT FROM source_entry #>> '{question_attempt_limit,maxAttempts}'
                   OR proposed_entry ->> 'questionAttemptTimeLimitSeconds' IS DISTINCT FROM
                        source_entry #>> '{question_attempt_time_limit,seconds}'
                   OR proposed_entry ->> 'questionAttemptGraceSeconds' IS DISTINCT FROM
                        source_entry #>> '{question_attempt_time_limit,graceSeconds}' THEN
                    RAISE EXCEPTION USING ERRCODE = '22023',
                        MESSAGE = 'Blueprint Fixed Question differs from its exact Revision';
                END IF;
            ELSIF source_entry ->> 'kind' = 'pool' THEN
                IF proposed_entry ->> 'kind' <> 'question_pool'
                   OR proposed_entry ->> 'availability' <> 'available'
                   OR proposed_entry ->> 'authoredPosition' <> entry_index::text
                   OR proposed_entry ->> 'sourceQuestionPoolId' IS DISTINCT FROM
                        replace(source_entry #>> '{question_pool_revision,questionPoolId}', '-', '')
                   OR proposed_entry ->> 'sourceQuestionPoolRevisionNumber' IS DISTINCT FROM
                        source_entry #>> '{question_pool_revision,revisionNumber}'
                   OR proposed_entry ->> 'forkQuestionPoolId' !~*
                        '^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$'
                   OR proposed_entry ->> 'forkPublicQuestionPoolId' !~
                        '^[0-9A-HJKMNP-TV-Z]{8}$'
                   OR proposed_entry ->> 'selectionCount' IS DISTINCT FROM source_entry ->> 'selection_count'
                   OR proposed_entry ->> 'pointsPerItem' IS DISTINCT FROM source_entry ->> 'points_per_item'
                   OR proposed_entry ->> 'scoringRule' IS DISTINCT FROM (CASE (source_entry ->> 'scoring_rule')
                        WHEN 'normal' THEN 'normal' WHEN 'fullCredit' THEN 'full_credit'
                        WHEN 'extraCredit' THEN 'extra_credit' WHEN 'excluded' THEN 'excluded' END)
                   OR proposed_entry ->> 'selectedQuestionOrder' IS DISTINCT FROM (CASE
                        WHEN (source_entry #>> '{selection_rule,selectedQuestionOrder}') = 'questionPoolOrder'
                        THEN 'question_pool_order' WHEN (source_entry #>> '{selection_rule,selectedQuestionOrder}') = 'randomOrder'
                        THEN 'random_order' END)
                   OR proposed_entry ->> 'questionAttemptLimit' IS DISTINCT FROM source_entry #>> '{question_attempt_limit,maxAttempts}'
                   OR proposed_entry ->> 'questionAttemptTimeLimitSeconds' IS DISTINCT FROM
                        source_entry #>> '{question_attempt_time_limit,seconds}'
                   OR proposed_entry ->> 'questionAttemptGraceSeconds' IS DISTINCT FROM
                        source_entry #>> '{question_attempt_time_limit,graceSeconds}' THEN
                    RAISE EXCEPTION USING ERRCODE = '22023',
                        MESSAGE = 'Blueprint Question Pool differs from its exact Revision';
                END IF;
            ELSE
                RAISE EXCEPTION USING ERRCODE = '22023',
                    MESSAGE = 'Blueprint Assessment Entry is invalid';
            END IF;
        END LOOP;
    END LOOP;
END;
$$;

CREATE FUNCTION ple_data.initialize_course_assessments(
    p_course_id uuid, p_blueprint_reference bigint, p_blueprint_revision bigint, p_assessments jsonb
)
RETURNS void LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data AS $$
DECLARE
    member jsonb;
    entry_json jsonb;
    candidate ple_data.assessment%ROWTYPE;
    new_assessment_id uuid;
    source_question_pool_id uuid;
    forked record;
BEGIN
    IF EXISTS (SELECT 1 FROM ple_data.assessment WHERE course_id = p_course_id)
       OR jsonb_typeof(p_assessments) IS DISTINCT FROM 'array' THEN
        RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Course initial assessments are invalid';
    END IF;
    PERFORM ple_data.validate_course_blueprint_adoption(
        p_blueprint_reference, p_blueprint_revision, p_assessments
    );
    FOR member IN SELECT value FROM jsonb_array_elements(p_assessments) LOOP
        IF jsonb_typeof(member -> 'entries') IS DISTINCT FROM 'array'
           OR jsonb_array_length(member -> 'entries') = 0 THEN
            RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Blueprint Assessment requires its Questions';
        END IF;
        -- Only content/policy columns below are admitted. Identity, provenance,
        -- initial Edit Number, and unreleased state are always database-owned.
        SELECT * INTO candidate FROM jsonb_populate_record(NULL::ple_data.assessment,
            member -> 'values');
        new_assessment_id := gen_random_uuid();
        INSERT INTO ple_data.assessment (
            assessment_id, course_id, source_blueprint_course_reference_number,
            source_blueprint_revision_number, source_blueprint_assessment_reference,
            created_at, updated_at,
            assessment_title,
            assessment_instructions,
            available_at,
            due_at,
            closes_at,
            assessment_attempt_time_limit_seconds,
            assessment_attempt_limit,
            late_work_rule,
            assessment_completion_rule,
            assessment_completion_score_threshold,
            assessment_attempt_grade_rule,
            assessment_attempt_continuation_rule,
            max_additional_assessment_attempts,
            question_pool_reuse_rule,
            question_variation_rule,
            assessment_attempt_resume_rule,
            assessment_question_display_rule,
            assessment_navigation_rule,
            assessment_question_order_rule,
            feedback_score,
            feedback_per_item_correctness,
            feedback_submitted_response,
            feedback_question_feedback,
            feedback_question_answer,
            feedback_question_answer_explanation,
            feedback_class_statistics
        ) VALUES (
            new_assessment_id, p_course_id, p_blueprint_reference,
            p_blueprint_revision, (member ->> 'source')::uuid,
            transaction_timestamp(), transaction_timestamp(),
            candidate.assessment_title,
            candidate.assessment_instructions,
            NULL,
            NULL,
            NULL,
            candidate.assessment_attempt_time_limit_seconds,
            candidate.assessment_attempt_limit,
            candidate.late_work_rule,
            candidate.assessment_completion_rule,
            candidate.assessment_completion_score_threshold,
            candidate.assessment_attempt_grade_rule,
            candidate.assessment_attempt_continuation_rule,
            candidate.max_additional_assessment_attempts,
            candidate.question_pool_reuse_rule,
            candidate.question_variation_rule,
            candidate.assessment_attempt_resume_rule,
            candidate.assessment_question_display_rule,
            candidate.assessment_navigation_rule,
            candidate.assessment_question_order_rule,
            candidate.feedback_score,
            candidate.feedback_per_item_correctness,
            candidate.feedback_submitted_response,
            candidate.feedback_question_feedback,
            candidate.feedback_question_answer,
            candidate.feedback_question_answer_explanation,
            candidate.feedback_class_statistics
        );
        -- Fixed entries have no child lineage and can use the ordinary guarded
        -- entry writer. Pool entries are created below through their distinct
        -- immutable fork boundary; a normal save may never attach a published Pool.
        PERFORM ple_data.replace_assessment_entries(
            new_assessment_id,
            COALESCE(
                (
                    SELECT jsonb_agg(value ORDER BY ordinality)
                      FROM jsonb_array_elements(member -> 'entries') WITH ORDINALITY
                     WHERE value ->> 'kind' = 'fixed_question'
                ),
                '[]'::jsonb
            )
        );
        FOR entry_json IN
            SELECT value FROM jsonb_array_elements(member -> 'entries')
             WHERE value ->> 'kind' = 'question_pool'
        LOOP
            SELECT pool.question_pool_id INTO source_question_pool_id
             FROM ple_data.question_pool AS pool
             WHERE pool.public_question_pool_id = entry_json ->> 'sourceQuestionPoolId';
            IF NOT FOUND THEN
                RAISE EXCEPTION USING ERRCODE = '22023',
                    MESSAGE = 'Blueprint Question Pool source is unavailable';
            END IF;
            SELECT * INTO forked FROM ple_data.fork_question_pool_revision_for_course_adoption(
                (entry_json ->> 'forkQuestionPoolId')::uuid,
                entry_json ->> 'forkPublicQuestionPoolId',
                source_question_pool_id,
                (entry_json ->> 'sourceQuestionPoolRevisionNumber')::bigint
            );
            IF (entry_json ->> 'selectionCount')::integer > (
                SELECT member_count FROM ple_data.question_pool_revision
                 WHERE question_pool_id = forked.question_pool_id AND revision_number = 1
            ) THEN
                RAISE EXCEPTION USING ERRCODE = '22023',
                    MESSAGE = 'Blueprint Question Pool selection exceeds fork member count';
            END IF;
            INSERT INTO ple_data.assessment_entry (
                assessment_entry_id, assessment_id, authored_position, entry_kind, availability,
                scoring_rule, question_pool_id, question_pool_revision_number, selection_count,
                points_per_item, selected_question_order, question_attempt_limit,
                question_attempt_time_limit_seconds, question_attempt_grace_seconds
            ) VALUES (
                (entry_json ->> 'assessmentEntryId')::uuid, new_assessment_id,
                (entry_json ->> 'authoredPosition')::integer, 'question_pool', 'available',
                entry_json ->> 'scoringRule', forked.question_pool_id, 1,
                (entry_json ->> 'selectionCount')::integer,
                (entry_json ->> 'pointsPerItem')::numeric,
                entry_json ->> 'selectedQuestionOrder',
                NULLIF(entry_json ->> 'questionAttemptLimit', '')::integer,
                NULLIF(entry_json ->> 'questionAttemptTimeLimitSeconds', '')::integer,
                NULLIF(entry_json ->> 'questionAttemptGraceSeconds', '')::integer
            );
            INSERT INTO ple_data.assessment_question_pool_fork (
                assessment_entry_id, assessment_id, question_pool_id,
                origin_question_pool_revision_number
            ) VALUES (
                (entry_json ->> 'assessmentEntryId')::uuid, new_assessment_id,
                forked.question_pool_id, 1
            );
        END LOOP;
    END LOOP;
END
$$;

REVOKE ALL ON FUNCTION ple_data.initialize_course_assessments(uuid, bigint, bigint, jsonb) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_data.initialize_course_assessments(uuid, bigint, bigint, jsonb) TO ple_api_owner;
RESET ROLE;
