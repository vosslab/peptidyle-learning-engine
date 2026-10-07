BEGIN;
SELECT 'BPVP00000' || ple_private.crockford_checksum_character('BPVP00000') AS blueprint_candidate_id \gset
SELECT 'BPVF00000' || ple_private.crockford_checksum_character('BPVF00000') AS blueprint_fork_id \gset
SELECT 'SVB1-' || ple_private.crockford_checksum_character('SVB1ABC') || 'ABC' AS blueprint_fork_pool_id \gset
SET LOCAL ROLE ple_app;
SELECT set_config('ple.session_account_id', :'instructor_id', true);
SELECT set_config('ple.blueprint_candidate_id', :'blueprint_candidate_id', true);
SELECT set_config('ple.blueprint_fork_id', :'blueprint_fork_id', true);
SELECT set_config('ple.blueprint_question_id', :'question_id', true);
SELECT set_config('ple.blueprint_archived_question_id', :'archived_question_id', true);
SELECT set_config('ple.blueprint_pool_id', :'published_pool_id', true);
SELECT set_config('ple.blueprint_expected_pool_member_id', :'replacement_question_id', true);
SELECT set_config('ple.blueprint_fork_pool_id', :'blueprint_fork_pool_id', true);
DO $$
DECLARE
    defaults_json jsonb := jsonb_build_object(
        'assessment_attempt_time_limit_seconds', NULL,
        'assessment_attempt_limit', NULL,
        'late_work_rule', 'reject',
        'activity_rules', jsonb_build_object(
            'partialCreditEnabled', true,
            'questionVariationRule', 'newVariation',
            'assessmentQuestionOrderRule', 'shuffled'
        ),
        'student_feedback_release_rule', jsonb_build_object(
            'per_item_correctness', 'after_submit',
            'submitted_response', 'after_submit',
            'question_answer', 'never',
            'question_answer_explanation', 'never',
            'class_statistics', 'never',
            'hints', 'never',
            'worked_solutions', 'never'
        )
    );
    template jsonb;
    empty_content jsonb;
    created record;
    pool_metadata_edit bigint;
    saved record;
    forked record;
    saved_content jsonb;
    fork_content jsonb;
BEGIN
    template := jsonb_build_object(
        'modules', jsonb_build_array(jsonb_build_object(
            'blueprint_module_id', '73000000-0000-0000-0000-000000000091',
            'label', 'Module 1',
            'assessments', jsonb_build_array(jsonb_build_object(
                'blueprint_assessment_id', '73000000-0000-0000-0000-000000000092',
                'content', jsonb_build_object(
                    'assessment_type', 'regular_assignment',
                    'title', 'Blueprint pool assessment',
                    'instructions', '',
                    'entries', '[]'::jsonb,
                    'defaults', defaults_json
                )
            ))
        ))
    );
    empty_content := jsonb_set(template, '{modules,0,assessments,0,content,entries}', '[]'::jsonb);
    SELECT * INTO created FROM ple_api.create_blueprint_course(
        current_setting('ple.blueprint_candidate_id'),
        sha256(convert_to('blueprint-published-create', 'UTF8')),
        'BPPOOL', 'Blueprint published content', 'ocean', empty_content,
        sha256(convert_to(empty_content::text, 'UTF8')),
        '73000000-0000-0000-0000-00000000cc01',
        '73000000-0000-0000-0000-00000000cc02',
        NULL, NULL, ARRAY[]::text[]
    );
    IF created.blueprint_course_id <> current_setting('ple.blueprint_candidate_id')
       OR created.blueprint_revision_number <> 1 THEN
        RAISE EXCEPTION 'Blueprint creation did not keep the supplied id at Revision 1';
    END IF;

    BEGIN
        PERFORM * FROM ple_api.save_blueprint_course(
            created.blueprint_course_id, 1,
            sha256(convert_to('blueprint-published-archived-pin', 'UTF8')),
            jsonb_set(template, '{modules,0,assessments,0,content,entries}', jsonb_build_array(jsonb_build_object(
                'kind', 'fixed',
                'published_question_revision_tuple', jsonb_build_object(
                    'publishedQuestionId', current_setting('ple.blueprint_archived_question_id'),
                    'revisionNumber', 1
                ),
                'points_possible', '1',
                'scoring_rule', 'normal',
                'question_attempt_limit', jsonb_build_object('maxAttempts', NULL),
                'question_attempt_time_limit', jsonb_build_object('kind', 'unlimited')
            ))),
            sha256(convert_to('blueprint-published-archived-content', 'UTF8')),
            '[]'::jsonb
        );
        RAISE EXCEPTION 'archived Question pin was accepted on the Blueprint';
    EXCEPTION WHEN invalid_parameter_value THEN
        IF SQLERRM <> 'New Blueprint Question pins require an Available Question' THEN RAISE; END IF;
    END;

    SELECT metadata.question_pool_metadata_edit_number INTO pool_metadata_edit
      FROM ple_api.read_current_question_pool_metadata(
          current_setting('ple.blueprint_pool_id')
      ) AS metadata;
    PERFORM * FROM ple_api.replace_question_pool_metadata(
        current_setting('ple.blueprint_pool_id'), pool_metadata_edit,
        'M11 source pool', 'Fork metadata source', NULL, NULL, ARRAY['m11', 'pool-copy'],
        'Analyze', 'Conceptual Knowledge'
    );

    saved_content := jsonb_set(template, '{modules,0,assessments,0,content,entries}', jsonb_build_array(
        jsonb_build_object(
            'kind', 'fixed',
            'published_question_revision_tuple', jsonb_build_object(
                'publishedQuestionId', current_setting('ple.blueprint_question_id'),
                'revisionNumber', 1
            ),
            'points_possible', '1',
            'scoring_rule', 'normal',
            'question_attempt_limit', jsonb_build_object('maxAttempts', NULL),
            'question_attempt_time_limit', jsonb_build_object('kind', 'unlimited')
        ),
        jsonb_build_object(
            'kind', 'pool',
            'question_pool_id', current_setting('ple.blueprint_pool_id'),
            'selection_count', 3,
            'points_per_item', '1',
            'scoring_rule', 'normal',
            'question_attempt_limit', jsonb_build_object('maxAttempts', NULL),
            'question_attempt_time_limit', jsonb_build_object('kind', 'unlimited')
        )
    ));
    SELECT * INTO saved FROM ple_api.save_blueprint_course(
        created.blueprint_course_id, 1,
        sha256(convert_to('blueprint-published-save', 'UTF8')),
        saved_content, sha256(convert_to(saved_content::text, 'UTF8')),
        '[]'::jsonb
    );
    IF NOT saved.changed OR saved.resulting_blueprint_revision_number <> 2 THEN
        RAISE EXCEPTION 'Blueprint Save did not record the published Question and ordinary Pool ID';
    END IF;
    PERFORM set_config('ple.blueprint_saved_revision', saved.resulting_blueprint_revision_number::text, true);

    PERFORM * FROM ple_api.set_blueprint_availability(created.blueprint_course_id, 1, 'public', NULL);
    fork_content := jsonb_set(saved_content,
        '{modules,0,blueprint_module_id}', to_jsonb('73000000-0000-0000-0000-000000000093'::text));
    fork_content := jsonb_set(fork_content,
        '{modules,0,assessments,0,blueprint_assessment_id}',
        to_jsonb('73000000-0000-0000-0000-000000000094'::text));
    SELECT * INTO forked FROM ple_api.fork_blueprint_course(
        current_setting('ple.blueprint_fork_id'), created.blueprint_course_id, 2,
        sha256(convert_to('blueprint-pool-preserving-fork', 'UTF8')),
        fork_content, sha256(convert_to(fork_content::text, 'UTF8'))
    );
    IF forked.blueprint_course_id <> current_setting('ple.blueprint_fork_id')
       OR forked.blueprint_revision_number <> 1 THEN
        RAISE EXCEPTION 'Blueprint fork did not create the supplied child at Revision 1';
    END IF;
END $$;

RESET ROLE;
SET LOCAL ROLE ple_api_owner;
DO $$
DECLARE
    blueprint_id text := current_setting('ple.blueprint_candidate_id');
    question_id text := current_setting('ple.blueprint_question_id');
    archived_question_id text := current_setting('ple.blueprint_archived_question_id');
    pool_id text := current_setting('ple.blueprint_pool_id');
    stored_pool text;
    published_source text;
    first_entries jsonb;
    fork_entries jsonb;
BEGIN
    IF current_setting('ple.blueprint_saved_revision') <> '2' THEN
        RAISE EXCEPTION 'Blueprint Revision 2 was not recorded';
    END IF;
    IF NOT EXISTS (
        SELECT 1 FROM ple_data.blueprint_course AS blueprint
         WHERE blueprint.blueprint_course_id = blueprint_id
           AND blueprint.availability = 'public'
           AND blueprint.current_blueprint_revision_number = 2
    ) THEN
        RAISE EXCEPTION 'Blueprint did not remain Public at Revision 2 after its fork';
    END IF;
    SELECT revision.content #> '{modules,0,assessments,0,content,entries}'
      INTO first_entries
      FROM ple_data.blueprint_course_revision AS revision
     WHERE revision.blueprint_course_id = blueprint_id
       AND revision.blueprint_revision_number = 1;
    IF first_entries <> '[]'::jsonb THEN
        RAISE EXCEPTION 'refused Blueprint Save changed Revision 1';
    END IF;
    IF NOT EXISTS (
        SELECT 1 FROM ple_data.blueprint_revision_question_pin AS pin
         WHERE pin.blueprint_course_id = blueprint_id
           AND pin.blueprint_revision_number = 2
           AND pin.published_question_id = question_id
           AND pin.question_revision_number = 1
    ) OR EXISTS (
        SELECT 1 FROM ple_data.blueprint_revision_question_pin AS pin
         WHERE pin.blueprint_course_id = blueprint_id
           AND pin.published_question_id = archived_question_id
    ) THEN
        RAISE EXCEPTION 'Blueprint pin was not the available published Question';
    END IF;
    SELECT entry ->> 'question_pool_id' INTO stored_pool
      FROM ple_data.blueprint_course_revision AS revision,
           jsonb_array_elements(revision.content #> '{modules,0,assessments,0,content,entries}') AS entry
     WHERE revision.blueprint_course_id = blueprint_id
       AND revision.blueprint_revision_number = 2
       AND entry ->> 'kind' = 'pool';
    IF stored_pool IS DISTINCT FROM pool_id THEN
        RAISE EXCEPTION 'Blueprint did not preserve its ordinary Pool ID';
    END IF;
    IF (SELECT (entry ->> 'selection_count')::integer
          FROM ple_data.blueprint_course_revision AS revision,
               jsonb_array_elements(revision.content #> '{modules,0,assessments,0,content,entries}') AS entry
         WHERE revision.blueprint_course_id = blueprint_id
           AND revision.blueprint_revision_number = 2
           AND entry ->> 'kind' = 'pool') <> 3 THEN
        RAISE EXCEPTION 'Blueprint save rejected or changed its Assessment-local short-Pool request';
    END IF;
    IF EXISTS (
        SELECT 1
          FROM ple_data.blueprint_course_revision AS revision,
               jsonb_array_elements(revision.content #> '{modules,0,assessments,0,content,entries}') AS entry
         WHERE revision.blueprint_course_id = blueprint_id
           AND revision.blueprint_revision_number = 2
           AND entry ->> 'kind' = 'pool'
           AND entry ? 'question_pool_edit_number'
    ) THEN
        RAISE EXCEPTION 'Blueprint persisted Pool Edit Number as historical state';
    END IF;
    SELECT pool.source_question_pool_id INTO published_source
      FROM ple_data.question_pool AS pool
     WHERE pool.question_pool_id = pool_id;
    IF published_source IS NOT NULL THEN
        RAISE EXCEPTION 'source Question Pool was not an ordinary Pool';
    END IF;
    SELECT revision.content #> '{modules,0,assessments,0,content,entries}'
      INTO fork_entries
      FROM ple_data.blueprint_course_revision AS revision
     WHERE revision.blueprint_course_id = current_setting('ple.blueprint_fork_id')
       AND revision.blueprint_revision_number = 1;
    IF NOT EXISTS (
        SELECT 1 FROM jsonb_array_elements(fork_entries) AS entry
         WHERE entry ->> 'kind' = 'pool'
           AND entry ->> 'question_pool_id' = pool_id
           AND (entry ? 'question_pool_edit_number') IS FALSE
    ) THEN
        RAISE EXCEPTION 'Blueprint fork did not preserve the existing Pool ID without an Edit Number';
    END IF;
    RAISE NOTICE 'blueprint_courses_contain_only_published_questions_and_published_pools';
END $$;
DO $$
DECLARE
    source_id text := current_setting('ple.blueprint_pool_id');
    child_id text := current_setting('ple.blueprint_fork_pool_id');
    owner_id text := current_setting('ple.session_account_id');
    source_pool ple_data.question_pool%ROWTYPE;
    child_pool ple_data.question_pool%ROWTYPE;
    fork_result record;
    source_edit bigint;
BEGIN
    SELECT * INTO fork_result FROM ple_data.fork_question_pool(child_id, source_id);
    IF fork_result.question_pool_id <> child_id OR fork_result.question_pool_edit_number <> 1 THEN
        RAISE EXCEPTION 'ordinary Pool fork did not start at the supplied ID and Edit 1';
    END IF;
    SELECT * INTO source_pool FROM ple_data.question_pool WHERE question_pool_id = source_id;
    SELECT * INTO child_pool FROM ple_data.question_pool WHERE question_pool_id = child_id;
    IF child_pool.owner_account_id::text <> owner_id
       OR child_pool.owner_user_role <> 'instructor'
       OR child_pool.question_pool_edit_number <> 1
       OR child_pool.source_question_pool_id::text <> source_id THEN
        RAISE EXCEPTION 'ordinary Pool fork did not keep acting owner, Edit 1, and source pointer';
    END IF;
    IF ROW(
        child_pool.question_type, child_pool.backend, child_pool.license,
        child_pool.title, child_pool.description, child_pool.content_discipline_id,
        child_pool.content_subject_id, child_pool.content_topic_id, child_pool.content_subtopic_id,
        child_pool.tags, child_pool.bloom_cognitive_process, child_pool.bloom_knowledge_dimension,
        child_pool.hint, child_pool.general_feedback, child_pool.worked_solution
    ) IS DISTINCT FROM ROW(
        source_pool.question_type, source_pool.backend, source_pool.license,
        source_pool.title, source_pool.description, source_pool.content_discipline_id,
        source_pool.content_subject_id, source_pool.content_topic_id, source_pool.content_subtopic_id,
        source_pool.tags, source_pool.bloom_cognitive_process, source_pool.bloom_knowledge_dimension,
        source_pool.hint, source_pool.general_feedback, source_pool.worked_solution
    ) THEN
        RAISE EXCEPTION 'ordinary Pool fork did not copy all source metadata and Bloom dimensions';
    END IF;
    IF EXISTS (
        (SELECT published_question_id, question_revision_number
           FROM ple_data.question_pool_member WHERE question_pool_id = child_id)
        EXCEPT ALL
        (SELECT published_question_id, question_revision_number
           FROM ple_data.question_pool_member WHERE question_pool_id = source_id)
    ) OR EXISTS (
        (SELECT published_question_id, question_revision_number
           FROM ple_data.question_pool_member WHERE question_pool_id = source_id)
        EXCEPT ALL
        (SELECT published_question_id, question_revision_number
           FROM ple_data.question_pool_member WHERE question_pool_id = child_id)
    ) THEN
        RAISE EXCEPTION 'ordinary Pool fork did not copy the exact current Revision Tuples';
    END IF;

    PERFORM * FROM ple_data.save_question_pool_members(
        child_id, 1, ARRAY[current_setting('ple.blueprint_expected_pool_member_id')], ARRAY[1]
    );
    IF (SELECT count(*) FROM ple_data.question_pool_member WHERE question_pool_id = source_id) <> 2 THEN
        RAISE EXCEPTION 'editing the ordinary Pool fork changed its parent membership';
    END IF;
    SELECT question_pool_edit_number INTO source_edit
      FROM ple_data.question_pool WHERE question_pool_id = source_id;
    PERFORM * FROM ple_data.save_question_pool_members(
        source_id, source_edit,
        ARRAY[current_setting('ple.blueprint_question_id')], ARRAY[1]
    );
    IF (SELECT count(*) FROM ple_data.question_pool_member WHERE question_pool_id = child_id) <> 1
       OR NOT EXISTS (
           SELECT 1 FROM ple_data.question_pool_member
            WHERE question_pool_id = child_id
              AND published_question_id = current_setting('ple.blueprint_expected_pool_member_id')
              AND question_revision_number = 1
       ) THEN
        RAISE EXCEPTION 'editing the parent ordinary Pool changed its fork membership';
    END IF;
    RAISE NOTICE 'ordinary_pool_fork_copies_metadata_and_current_tuples_independently';
END $$;
COMMIT;
