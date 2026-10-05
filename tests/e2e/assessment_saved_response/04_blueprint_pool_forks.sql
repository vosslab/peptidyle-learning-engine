BEGIN;
SELECT 'BPVP00000' || ple_private.crockford_checksum_character('BPVP00000') AS blueprint_candidate_id \gset
SELECT 'SVB1-' || ple_private.crockford_checksum_character('SVB1ABC') || 'ABC' AS blueprint_fork_pool_id \gset
SET LOCAL ROLE ple_app;
SELECT set_config('ple.session_account_id', :'instructor_id', true);
SELECT set_config('ple.blueprint_candidate_id', :'blueprint_candidate_id', true);
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
    denial_count integer := 0;
    fork_edit bigint;
    saved record;
    saved_content jsonb;
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
        'BPPOOL', 'Blueprint published content', empty_content,
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
        denial_count := denial_count + 1;
    END;

    BEGIN
        PERFORM * FROM ple_api.save_blueprint_course(
            created.blueprint_course_id, 1,
            sha256(convert_to('blueprint-published-raw-pool', 'UTF8')),
            jsonb_set(template, '{modules,0,assessments,0,content,entries}', jsonb_build_array(jsonb_build_object(
                'kind', 'pool',
                'question_pool_id', current_setting('ple.blueprint_pool_id'),
                'question_pool_edit_number', 1,
                'selection_count', 1,
                'points_per_item', '1',
                'scoring_rule', 'normal',
                'selection_rule', jsonb_build_object('selectedQuestionOrder', 'questionPoolOrder'),
                'question_attempt_limit', jsonb_build_object('maxAttempts', NULL),
                'question_attempt_time_limit', jsonb_build_object('kind', 'unlimited')
            ))),
            sha256(convert_to('blueprint-published-raw-pool-content', 'UTF8')),
            '[]'::jsonb
        );
        RAISE EXCEPTION 'published Question Pool was stored on the Blueprint';
    EXCEPTION WHEN insufficient_privilege THEN
        IF SQLERRM <> 'Blueprint Pool ownership is unavailable' THEN RAISE; END IF;
        denial_count := denial_count + 1;
    END;
    IF denial_count <> 2 THEN
        RAISE EXCEPTION 'Blueprint published-content denial count %', denial_count;
    END IF;

    SELECT ple_api.fork_blueprint_question_pool(
        current_setting('ple.blueprint_pool_id'),
        current_setting('ple.blueprint_fork_pool_id')
    ) INTO fork_edit;
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
            'question_pool_id', current_setting('ple.blueprint_fork_pool_id'),
            'question_pool_edit_number', fork_edit,
            'selection_count', 1,
            'points_per_item', '1',
            'scoring_rule', 'normal',
            'selection_rule', jsonb_build_object('selectedQuestionOrder', 'questionPoolOrder'),
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
        RAISE EXCEPTION 'Blueprint Save did not record the published Question and Pool fork';
    END IF;
    PERFORM set_config('ple.blueprint_saved_revision', saved.resulting_blueprint_revision_number::text, true);
    PERFORM set_config('ple.blueprint_fork_edit', fork_edit::text, true);
END $$;

RESET ROLE;
SET LOCAL ROLE ple_api_owner;
DO $$
DECLARE
    blueprint_id text := current_setting('ple.blueprint_candidate_id');
    question_id text := current_setting('ple.blueprint_question_id');
    expected_pool_member_id text := current_setting('ple.blueprint_expected_pool_member_id');
    archived_question_id text := current_setting('ple.blueprint_archived_question_id');
    pool_id text := current_setting('ple.blueprint_pool_id');
    fork_id text := current_setting('ple.blueprint_fork_pool_id');
    stored_pool text;
    source_pool text;
    published_source text;
    member_count integer;
    member_question text;
    member_revision integer;
    first_entries jsonb;
BEGIN
    IF current_setting('ple.blueprint_saved_revision') <> '2' THEN
        RAISE EXCEPTION 'Blueprint Revision 2 was not recorded';
    END IF;
    IF NOT EXISTS (
        SELECT 1 FROM ple_data.blueprint_course AS blueprint
         WHERE blueprint.blueprint_course_id = blueprint_id
           AND blueprint.availability = 'private'
           AND blueprint.current_blueprint_revision_number = 2
    ) THEN
        RAISE EXCEPTION 'Blueprint did not stay Private at Revision 2';
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
    SELECT pool.source_question_pool_id INTO source_pool
      FROM ple_data.question_pool AS pool
     WHERE pool.question_pool_id = stored_pool;
    IF stored_pool IS DISTINCT FROM fork_id OR source_pool IS DISTINCT FROM pool_id THEN
        RAISE EXCEPTION 'Blueprint stored a Pool other than the published Pool fork';
    END IF;
    SELECT published.source_question_pool_id INTO published_source
      FROM ple_data.question_pool AS published
     WHERE published.question_pool_id = pool_id;
    IF published_source IS NOT NULL THEN
        RAISE EXCEPTION 'source Question Pool was not an original published Pool';
    END IF;
    SELECT count(*), min(member.published_question_id), min(member.question_revision_number)
      INTO member_count, member_question, member_revision
      FROM ple_data.question_pool_member AS member
     WHERE member.question_pool_id = fork_id;
    IF member_count <> 1 OR member_question IS DISTINCT FROM expected_pool_member_id OR member_revision <> 1 THEN
        RAISE EXCEPTION 'Blueprint fork membership was not the available published Question revision';
    END IF;
    RAISE NOTICE 'blueprint_courses_contain_only_published_questions_and_published_pools';
END $$;
COMMIT;
