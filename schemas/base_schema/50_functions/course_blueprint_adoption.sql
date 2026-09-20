-- Functions, triggers, and views from course_blueprint_adoption.sql.

SET LOCAL ROLE ple_api_owner;

-- Atomic initial teaching content, materialized by the Store from the exact Blueprint.
CREATE FUNCTION ple_api.load_course_instance_blueprint(p_blueprint_course_id text, p_revision_number bigint)
RETURNS TABLE(content jsonb, content_checksum bytea)
LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data AS $$
    SELECT revision.content, revision.content_checksum
      FROM ple_data.blueprint_course AS blueprint
      JOIN ple_data.blueprint_course_revision AS revision
        ON revision.blueprint_course_id = blueprint.blueprint_course_id
       AND revision.blueprint_revision_number = p_revision_number
     WHERE blueprint.blueprint_course_id = p_blueprint_course_id
       AND blueprint.availability = 'public'
       AND (ple_api.current_session_account_is_instructor()
            OR ple_api.current_session_account_is_sysadmin())
$$;





-- The automatic append is authorized by Blueprint ownership, not daughter
-- teaching membership or current Course Instance activity.
CREATE FUNCTION ple_api.load_blueprint_assessment_copy_source(p_blueprint_course_id text, p_revision_number bigint)
RETURNS TABLE(content jsonb, content_checksum bytea)
LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data AS $$
    SELECT revision.content, revision.content_checksum
      FROM ple_data.blueprint_course AS blueprint
      JOIN ple_data.blueprint_course_revision AS revision
        ON revision.blueprint_course_id = blueprint.blueprint_course_id
       AND revision.blueprint_revision_number = p_revision_number
     WHERE blueprint.blueprint_course_id = p_blueprint_course_id
       AND ((blueprint.availability IN ('public', 'archived')
             AND (ple_api.current_session_account_is_instructor()
                  OR ple_api.current_session_account_is_sysadmin()))
            OR (blueprint.owner_account_id = ple_api.current_session_account_id()
                AND ple_api.current_session_account_is_instructor()))
$$;

SET LOCAL ROLE ple_data_owner;






-- ASVS 2.3.1 and 2.3.3: course adoption is one atomic business operation.
-- The Store may mint fresh Course-side identities, but it must not choose any
-- persisted curriculum or policy value.  Compare that proposed materialization
-- to the sealed Blueprint Revision here, before any Course child exists.
CREATE FUNCTION ple_data.validate_course_blueprint_adoption(
    p_blueprint_course_id text, p_blueprint_revision_number bigint, p_assessments jsonb
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
          FROM ple_api.load_blueprint_assessment_copy_source(
              p_blueprint_course_id, p_blueprint_revision_number
          ) AS revision
          CROSS JOIN LATERAL jsonb_array_elements(revision.content -> 'modules')
              AS module_row(module)
          CROSS JOIN LATERAL jsonb_array_elements(module_row.module -> 'assessments')
              AS assessment_row(assessment)
         WHERE assessment_row.assessment ->> 'blueprint_assessment_id' = member ->> 'source';
        IF source_content IS NULL
           OR jsonb_typeof(member -> 'values') <> 'object'
           OR jsonb_typeof(member -> 'entries') <> 'array' THEN
            RAISE EXCEPTION USING ERRCODE = '22023',
                MESSAGE = 'Blueprint adoption content is invalid';
        END IF;
        IF member -> 'values' IS DISTINCT FROM jsonb_build_object(
            'assessment_type', source_content -> 'assessment_type',
            'assessment_title', source_content -> 'title',
            'assessment_instructions', source_content -> 'instructions',
            'available_at', NULL, 'due_at', NULL, 'closes_at', NULL,
            'assessment_attempt_time_limit_seconds',
                source_content #> '{defaults,assessment_attempt_time_limit_seconds}',
            'assessment_attempt_limit', CASE
                WHEN source_content ->> 'assessment_type' IN ('quiz', 'exam') THEN '1'::jsonb
                ELSE source_content #> '{defaults,assessment_attempt_limit}'
            END,
            'late_work_rule', source_content #> '{defaults,late_work_rule}',
            'question_variation_rule', CASE (source_content #>> '{defaults,activity_rules,questionVariationRule}')
                WHEN 'reuseVariation' THEN 'reuse_variation' WHEN 'newVariation' THEN 'new_variation' END,
            'assessment_question_order_rule', CASE (source_content #>> '{defaults,activity_rules,assessmentQuestionOrderRule}')
                WHEN 'authoredOrder' THEN 'authored_order' WHEN 'shuffled' THEN 'shuffled' END,
            'feedback_score', source_content #> '{defaults,student_feedback_release_rule,score}',
            'feedback_per_item_correctness', source_content #> '{defaults,student_feedback_release_rule,per_item_correctness}',
            'feedback_submitted_response', source_content #> '{defaults,student_feedback_release_rule,submitted_response}',
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
                IF proposed_entry ->> 'kind' IS DISTINCT FROM 'fixed_question'
                   OR proposed_entry ->> 'availability' IS DISTINCT FROM 'available'
                   OR proposed_entry ->> 'authoredPosition' IS DISTINCT FROM entry_index::text
                   -- C842/C843 persist the exact canonical Question ID in the
                   -- sealed JSON and relational state; no representation
                   -- translation is permitted at this persistence seam.
                   OR proposed_entry ->> 'questionId' IS DISTINCT FROM
                        source_entry #>> '{question_revision,questionId}'
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
                IF proposed_entry ->> 'kind' IS DISTINCT FROM 'question_pool'
                   OR proposed_entry ->> 'availability' IS DISTINCT FROM 'available'
                   OR proposed_entry ->> 'authoredPosition' IS DISTINCT FROM entry_index::text
                   OR proposed_entry ->> 'sourceQuestionPoolId' IS DISTINCT FROM
                        source_entry ->> 'question_pool_id'
                   OR proposed_entry ->> 'sourceQuestionPoolEditNumber' IS DISTINCT FROM
                        source_entry ->> 'question_pool_edit_number'
                   OR proposed_entry ->> 'forkQuestionPoolId' !~
                        '^[0-9A-HJKMNP-TV-Z]{4}-[0-9A-HJKMNP-TV-Z]{4}$'
                   OR substr(proposed_entry ->> 'forkQuestionPoolId', 6, 1) IS DISTINCT FROM
                        ple_private.crockford_checksum_character(
                            substr(proposed_entry ->> 'forkQuestionPoolId', 1, 4)
                            || substr(proposed_entry ->> 'forkQuestionPoolId', 7, 3)
                        )
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

CREATE FUNCTION ple_data.append_course_assessments(
    p_course_instance_id text, p_blueprint_course_id text, p_blueprint_revision_number bigint, p_assessments jsonb
)
RETURNS void LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
DECLARE
    member jsonb;
    entry_json jsonb;
    candidate ple_data.assessment_policy_snapshot%ROWTYPE;
    assessment_type_value ple_data.assessment_type;
    snapshot_id ple_data.sha256_digest;
    new_assessment_id text;
    source_question_pool_id text;
    forked record;
BEGIN
    IF jsonb_typeof(p_assessments) IS DISTINCT FROM 'array' THEN
        RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Course initial assessments are invalid';
    END IF;
    PERFORM ple_data.validate_course_blueprint_adoption(
        p_blueprint_course_id, p_blueprint_revision_number, p_assessments
    );
    FOR member IN SELECT value FROM jsonb_array_elements(p_assessments) LOOP
        -- Stable provenance makes a repeated append harmless, without touching
        -- the existing daughter copy's policies, release state, or Student Work.
        IF EXISTS (
            SELECT 1 FROM ple_data.assessment
             WHERE course_instance_id = p_course_instance_id
               AND source_blueprint_course_id = p_blueprint_course_id
               AND source_blueprint_assessment_id = (member ->> 'source')::uuid
        ) THEN CONTINUE; END IF;
        IF jsonb_typeof(member -> 'entries') IS DISTINCT FROM 'array'
           OR jsonb_array_length(member -> 'entries') = 0 THEN
            RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Blueprint Assessment requires its Questions';
        END IF;
        -- Only content/policy columns below are admitted. Identity, provenance,
        -- initial Edit Number, and unreleased state are always database-owned.
        assessment_type_value := (member -> 'values' ->> 'assessment_type')::ple_data.assessment_type;
        SELECT * INTO candidate FROM jsonb_populate_record(
            NULL::ple_data.assessment_policy_snapshot, member -> 'values');
        snapshot_id := ple_private.ensure_assessment_policy_snapshot(
            candidate.assessment_title, candidate.assessment_instructions,
            NULL, NULL, NULL,
            candidate.assessment_attempt_time_limit_seconds, candidate.assessment_attempt_limit,
            candidate.late_work_rule, candidate.question_variation_rule,
            candidate.assessment_question_order_rule, candidate.feedback_score,
            candidate.feedback_per_item_correctness, candidate.feedback_submitted_response,
            candidate.feedback_question_answer, candidate.feedback_question_answer_explanation,
            candidate.feedback_class_statistics, assessment_type_value
        );
        new_assessment_id := gen_random_uuid();
        INSERT INTO ple_data.assessment (
            assessment_id, course_instance_id, origin_kind, source_blueprint_course_id,
            source_blueprint_revision_number, source_blueprint_assessment_id,
            created_at, updated_at,
            assessment_type,
            assessment_policy_snapshot_id
        ) VALUES (
            new_assessment_id, p_course_instance_id, 'adopted', p_blueprint_course_id,
            p_blueprint_revision_number, (member ->> 'source')::uuid,
            transaction_timestamp(), transaction_timestamp(),
            assessment_type_value,
            snapshot_id
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
             WHERE pool.question_pool_id = entry_json ->> 'sourceQuestionPoolId';
            IF NOT FOUND THEN
                RAISE EXCEPTION USING ERRCODE = '22023',
                    MESSAGE = 'Blueprint Question Pool source is unavailable';
            END IF;
            SELECT * INTO forked FROM ple_data.fork_question_pool_for_course_adoption(
                entry_json ->> 'forkQuestionPoolId',
                source_question_pool_id
            );
            IF (entry_json ->> 'selectionCount')::integer > (
                SELECT count(*) FROM ple_data.question_pool_member AS member
                 WHERE member.question_pool_id = forked.question_pool_id
            ) THEN
                RAISE EXCEPTION USING ERRCODE = '22023',
                    MESSAGE = 'Blueprint Question Pool selection exceeds fork member count';
            END IF;
            INSERT INTO ple_data.assessment_entry (
                assessment_entry_id, assessment_id, authored_position, entry_kind, availability,
                scoring_rule, question_attempt_limit,
                question_attempt_time_limit_seconds, question_attempt_grace_seconds
            ) VALUES (
                (entry_json ->> 'assessmentEntryId')::uuid, new_assessment_id,
                (entry_json ->> 'authoredPosition')::integer, 'question_pool', 'available',
                entry_json ->> 'scoringRule',
                NULLIF(entry_json ->> 'questionAttemptLimit', '')::integer,
                NULLIF(entry_json ->> 'questionAttemptTimeLimitSeconds', '')::integer,
                NULLIF(entry_json ->> 'questionAttemptGraceSeconds', '')::integer
            );
            INSERT INTO ple_data.assessment_entry_pool (
                assessment_entry_id, assessment_id, question_pool_id,
                selection_count, points_per_item, selected_question_order
            ) VALUES (
                (entry_json ->> 'assessmentEntryId')::uuid, new_assessment_id,
                forked.question_pool_id,
                (entry_json ->> 'selectionCount')::integer,
                (entry_json ->> 'pointsPerItem')::numeric,
                entry_json ->> 'selectedQuestionOrder'
            );
            INSERT INTO ple_data.assessment_question_pool_fork (
                assessment_entry_id, assessment_id, question_pool_id
            ) VALUES (
                (entry_json ->> 'assessmentEntryId')::uuid, new_assessment_id,
                forked.question_pool_id
            );
        END LOOP;
    END LOOP;
END
$$;

CREATE FUNCTION ple_data.initialize_course_assessments(
    p_course_instance_id text, p_blueprint_course_id text, p_blueprint_revision_number bigint, p_assessments jsonb
)
RETURNS void LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_data AS $$
BEGIN
    IF EXISTS (SELECT 1 FROM ple_data.assessment WHERE course_instance_id = p_course_instance_id) THEN
        RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Course initial assessments are invalid';
    END IF;
    PERFORM ple_data.append_course_assessments(
        p_course_instance_id, p_blueprint_course_id, p_blueprint_revision_number, p_assessments
    );
END
$$;

SET LOCAL ROLE ple_api_owner;

CREATE FUNCTION ple_api.list_blueprint_daughter_course_ids(p_blueprint_course_id text)
RETURNS TABLE(course_instance_id text)
LANGUAGE plpgsql VOLATILE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data AS $$
BEGIN
    PERFORM 1 FROM ple_data.blueprint_course
     WHERE blueprint_course_id = p_blueprint_course_id
       AND owner_account_id = ple_api.current_session_account_id()
       AND ple_api.current_session_account_is_instructor()
     FOR UPDATE;
    RETURN QUERY SELECT daughter.course_instance_id
      FROM ple_data.course_instance AS daughter
      JOIN ple_data.blueprint_course AS blueprint
        ON blueprint.blueprint_course_id = daughter.blueprint_course_id
     WHERE blueprint.blueprint_course_id = p_blueprint_course_id
       AND blueprint.owner_account_id = ple_api.current_session_account_id()
       AND ple_api.current_session_account_is_instructor()
     ORDER BY daughter.course_instance_id;
END
$$;

CREATE FUNCTION ple_api.append_new_blueprint_assessments(
    p_blueprint_course_id text, p_prior_revision bigint, p_saved_revision bigint,
    p_course_instance_id text, p_assessments jsonb
)
RETURNS void LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data AS $$
DECLARE
    blueprint ple_data.blueprint_course%ROWTYPE;
    expected_sources jsonb;
    proposed_sources jsonb;
BEGIN
    -- ASVS 8.2.1, 8.2.2: the owner may append only to daughters of this parent.
    -- ASVS 2.3.1, 2.3.3: this runs before the Store commits the Save,
    -- under the same Blueprint lock used by Save and Course adoption.
    SELECT * INTO blueprint FROM ple_data.blueprint_course
     WHERE blueprint_course_id = p_blueprint_course_id
       AND owner_account_id = ple_api.current_session_account_id()
       AND ple_api.current_session_account_is_instructor()
     FOR UPDATE;
    IF NOT FOUND OR p_saved_revision <> p_prior_revision + 1
       OR blueprint.current_blueprint_revision_number < p_saved_revision
       OR jsonb_typeof(p_assessments) IS DISTINCT FROM 'array'
       OR NOT EXISTS (
           SELECT 1 FROM ple_data.course_instance
            WHERE course_instance_id = p_course_instance_id
              AND blueprint_course_id = blueprint.blueprint_course_id
       ) THEN
        RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Blueprint Assessment append is invalid';
    END IF;
    SELECT COALESCE(jsonb_agg(added.blueprint_assessment_id::text
                             ORDER BY added.blueprint_assessment_id::text), '[]'::jsonb)
      INTO expected_sources
      FROM ple_data.blueprint_revision_assessment AS added
     WHERE added.blueprint_course_id = blueprint.blueprint_course_id
       AND added.blueprint_revision_number = p_saved_revision
       AND NOT EXISTS (
           SELECT 1 FROM ple_data.blueprint_revision_assessment AS prior
            WHERE prior.blueprint_course_id = blueprint.blueprint_course_id
              AND prior.blueprint_revision_number = p_prior_revision
              AND prior.blueprint_assessment_id = added.blueprint_assessment_id
       );
    SELECT COALESCE(jsonb_agg(value ->> 'source' ORDER BY value ->> 'source'), '[]'::jsonb)
      INTO proposed_sources FROM jsonb_array_elements(p_assessments);
    IF proposed_sources IS DISTINCT FROM expected_sources THEN
        RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Blueprint append must contain only newly added Assessments';
    END IF;
    PERFORM ple_data.append_course_assessments(
        p_course_instance_id, blueprint.blueprint_course_id, p_saved_revision, p_assessments
    );
END
$$;

CREATE FUNCTION ple_api.save_blueprint_course(
    p_blueprint_course_id text, p_expected_blueprint_revision_number bigint,
    p_request_checksum bytea, p_content jsonb, p_content_checksum bytea,
    p_daughters jsonb
)
RETURNS TABLE(resulting_blueprint_revision_number bigint, changed boolean,
              accepted_at timestamp with time zone)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data AS $$
DECLARE
    receipt record;
    replay boolean;
    daughter jsonb;
    expected_daughters jsonb;
    proposed_daughters jsonb;
BEGIN
    SELECT COALESCE(jsonb_agg(course_instance_id::text ORDER BY course_instance_id::text), '[]'::jsonb)
      INTO expected_daughters FROM ple_api.list_blueprint_daughter_course_ids(p_blueprint_course_id);
    SELECT EXISTS (
        SELECT 1 FROM ple_data.blueprint_course_save_receipt AS prior_receipt
        JOIN ple_data.blueprint_course AS blueprint
          ON blueprint.blueprint_course_id = prior_receipt.blueprint_course_id
        WHERE blueprint.blueprint_course_id = p_blueprint_course_id
          AND prior_receipt.actor_account_id = ple_api.current_session_account_id()
          AND prior_receipt.request_checksum = p_request_checksum
    ) INTO replay;
    SELECT * INTO receipt FROM ple_api.save_blueprint_course(
        p_blueprint_course_id, p_expected_blueprint_revision_number, p_request_checksum,
        p_content, p_content_checksum
    );
    IF receipt.changed AND NOT replay THEN
        IF jsonb_typeof(p_daughters) IS DISTINCT FROM 'array' THEN
            RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Blueprint daughters are invalid';
        END IF;
        SELECT COALESCE(jsonb_agg(value ->> 'course_instance_id' ORDER BY value ->> 'course_instance_id'), '[]'::jsonb)
          INTO proposed_daughters FROM jsonb_array_elements(p_daughters);
        IF proposed_daughters IS DISTINCT FROM expected_daughters THEN
            RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Blueprint Save requires every daughter Course Instance';
        END IF;
        FOR daughter IN SELECT value FROM jsonb_array_elements(p_daughters) LOOP
            PERFORM ple_api.append_new_blueprint_assessments(
                p_blueprint_course_id, p_expected_blueprint_revision_number,
                receipt.resulting_blueprint_revision_number,
                daughter ->> 'course_instance_id', daughter -> 'assessments'
            );
        END LOOP;
    END IF;
    RETURN QUERY SELECT receipt.resulting_blueprint_revision_number, receipt.changed, receipt.accepted_at;
END
$$;

