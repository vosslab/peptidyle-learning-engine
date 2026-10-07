BEGIN;
SET LOCAL ROLE ple_data_owner;
INSERT INTO ple_data.content_subject (content_subject_id, name)
VALUES ('73000000-0000-0000-0000-00000000cc02', 'Saved response subject');
INSERT INTO ple_data.content_subject_discipline (content_subject_id, content_discipline_id)
VALUES (
    '73000000-0000-0000-0000-00000000cc02',
    '73000000-0000-0000-0000-00000000cc01'
);
INSERT INTO ple_data.question_revision_metadata (
    published_question_id, revision_number, question_title, question_description, language, question_type,
    tags, content_discipline_id, content_subject_id, created_at, updated_at
) VALUES (
    :'question_id', 1, 'Saved response question', 'One available published question',
    'en', 'multipleChoice', ARRAY[]::text[],
    '73000000-0000-0000-0000-00000000cc01',
    '73000000-0000-0000-0000-00000000cc02',
    pg_catalog.transaction_timestamp(), pg_catalog.transaction_timestamp()
);
SELECT 'SVQ1-' || ple_private.crockford_checksum_character('SVQ1ABC') || 'ABC' AS archived_question_id \gset
SELECT 'SVQ2-' || ple_private.crockford_checksum_character('SVQ2ABC') || 'ABC' AS replacement_question_id \gset
INSERT INTO ple_data.published_question (published_question_id, availability, created_at)
VALUES (:'archived_question_id', 'archived', pg_catalog.transaction_timestamp());
INSERT INTO ple_data.published_question (published_question_id, availability, created_at)
VALUES (:'replacement_question_id', 'available', pg_catalog.transaction_timestamp());
INSERT INTO ple_data.question_revision (
    published_question_id, revision_number, backend, published_at
) VALUES (
    :'archived_question_id', 1, 'ple', pg_catalog.transaction_timestamp()
);
INSERT INTO ple_data.question_revision (
    published_question_id, revision_number, backend, published_at
) VALUES (
    :'replacement_question_id', 1, 'ple', pg_catalog.transaction_timestamp()
);
INSERT INTO ple_data.question_revision_metadata (
    published_question_id, revision_number, question_title, question_description, language, question_type,
    tags, content_discipline_id, content_subject_id, created_at, updated_at
) VALUES (
    :'replacement_question_id', 1, 'Replacement Pool question', 'Same-count replacement member',
    'en', 'multipleChoice', ARRAY[]::text[],
    '73000000-0000-0000-0000-00000000cc01',
    '73000000-0000-0000-0000-00000000cc02',
    pg_catalog.transaction_timestamp(), pg_catalog.transaction_timestamp()
);
SELECT 'SVD2-' || ple_private.crockford_checksum_character('SVD2ABC') || 'ABC' AS duplicate_revisions_pool_id \gset
SELECT 'SVT2-' || ple_private.crockford_checksum_character('SVT2ABC') || 'ABC' AS wrong_type_pool_id \gset
SELECT 'SVB2-' || ple_private.crockford_checksum_character('SVB2ABC') || 'ABC' AS wrong_backend_pool_id \gset
SELECT 'SVT3-' || ple_private.crockford_checksum_character('SVT3ABC') || 'ABC' AS wrong_type_question_id \gset
SELECT 'SVB3-' || ple_private.crockford_checksum_character('SVB3ABC') || 'ABC' AS wrong_backend_question_id \gset
INSERT INTO ple_data.published_question (published_question_id, availability, created_at)
VALUES
    (:'wrong_type_question_id', 'available', pg_catalog.transaction_timestamp()),
    (:'wrong_backend_question_id', 'available', pg_catalog.transaction_timestamp());
INSERT INTO ple_data.question_revision (published_question_id, revision_number, backend, published_at)
VALUES
    (:'question_id', 2, 'ple', pg_catalog.transaction_timestamp()),
    (:'wrong_type_question_id', 1, 'ple', pg_catalog.transaction_timestamp()),
    (:'wrong_backend_question_id', 1, 'webwork', pg_catalog.transaction_timestamp());
INSERT INTO ple_data.question_revision_metadata (
    published_question_id, revision_number, question_title, question_description, language, question_type,
    tags, content_discipline_id, content_subject_id, created_at, updated_at
)
SELECT published_question_id, 2, question_title, question_description, language, question_type,
       tags, content_discipline_id, content_subject_id,
       pg_catalog.transaction_timestamp(), pg_catalog.transaction_timestamp()
  FROM ple_data.question_revision_metadata
 WHERE published_question_id = :'question_id'
   AND revision_number = 1;
INSERT INTO ple_data.question_revision_license (
    published_question_id, revision_number, spdx_expression
) VALUES
    (:'archived_question_id', 1, 'CC0-1.0'),
    (:'replacement_question_id', 1, 'CC-BY-4.0'),
    (:'question_id', 2, 'CC-BY-SA-4.0'),
    (:'wrong_type_question_id', 1, 'CC0-1.0'),
    (:'wrong_backend_question_id', 1, 'CC0-1.0');
INSERT INTO ple_data.question_revision_metadata (
    published_question_id, revision_number, question_title, question_description, language, question_type,
    tags, content_discipline_id, content_subject_id, created_at, updated_at
) VALUES
    (:'wrong_type_question_id', 1, 'Wrong type Pool question', 'A distinct type fixture member', 'en', 'fillInBlank', ARRAY[]::text[],
     '73000000-0000-0000-0000-00000000cc01', '73000000-0000-0000-0000-00000000cc02', pg_catalog.transaction_timestamp(), pg_catalog.transaction_timestamp()),
    (:'wrong_backend_question_id', 1, 'Wrong backend Pool question', 'A distinct backend fixture member', 'en', 'multipleChoice', ARRAY[]::text[],
     '73000000-0000-0000-0000-00000000cc01', '73000000-0000-0000-0000-00000000cc02', pg_catalog.transaction_timestamp(), pg_catalog.transaction_timestamp());
SELECT 'SVP1-' || ple_private.crockford_checksum_character('SVP1ABC') || 'ABC' AS published_pool_id \gset
SELECT 'SVX1-' || ple_private.crockford_checksum_character('SVX1ABC') || 'ABC' AS missing_pool_id \gset
SELECT 'SVY1-' || ple_private.crockford_checksum_character('SVY1ABC') || 'ABC' AS rejected_pool_id \gset
SELECT 'ASVR0002' || ple_private.crockford_checksum_character('ASVR0002') AS course_pool_assessment_id \gset

RESET ROLE;
SET LOCAL ROLE ple_app;
SELECT set_config('ple.session_account_id', :'instructor_id', true);
SELECT set_config('ple.course_pool_archived_question_id', :'archived_question_id', true);
SELECT set_config('ple.course_pool_rejected_pool_id', :'rejected_pool_id', true);
SELECT set_config('ple.course_pool_question_id', :'question_id', true);
SELECT set_config('ple.course_pool_replacement_question_id', :'replacement_question_id', true);
SELECT set_config('ple.course_pool_duplicate_revisions_pool_id', :'duplicate_revisions_pool_id', true);
SELECT set_config('ple.course_pool_wrong_type_pool_id', :'wrong_type_pool_id', true);
SELECT set_config('ple.course_pool_wrong_backend_pool_id', :'wrong_backend_pool_id', true);
SELECT set_config('ple.course_pool_wrong_type_question_id', :'wrong_type_question_id', true);
SELECT set_config('ple.course_pool_wrong_backend_question_id', :'wrong_backend_question_id', true);
RESET ROLE;
SET LOCAL ROLE ple_data_owner;
DO $$
BEGIN
    IF has_function_privilege('ple_app',
           'ple_data.calculate_question_pool_license(text[],integer[])', 'EXECUTE')
       OR has_function_privilege('ple_app',
           'ple_data.current_question_pool_license(text)', 'EXECUTE') THEN
        RAISE EXCEPTION 'Internal Pool license calculation must remain behind authorized operations';
    END IF;
END $$;
RESET ROLE;
SET LOCAL ROLE ple_app;
DO $$
BEGIN
    BEGIN
        PERFORM * FROM ple_api.create_question_pool(
            current_setting('ple.course_pool_rejected_pool_id'),
            ARRAY[current_setting('ple.course_pool_archived_question_id')],
            ARRAY[1], 'Unavailable pool', 'Unavailable pool description', ARRAY[]::text[]
        );
        RAISE EXCEPTION 'archived Question was accepted as a Question Pool member';
    EXCEPTION WHEN foreign_key_violation THEN
        IF SQLERRM <> 'Question Pool member is unavailable' THEN RAISE; END IF;
    END;
END $$;
SELECT question_pool_id AS created_pool_id, question_pool_edit_number AS created_pool_edit
  FROM ple_api.create_question_pool(
    :'published_pool_id',
    ARRAY[current_setting('ple.course_pool_question_id'), current_setting('ple.course_pool_replacement_question_id')],
    ARRAY[1, 1],
    'Published pool', 'Published pool of two available questions', ARRAY[]::text[]
) \gset
SELECT set_config('ple.course_pool_created_pool_id', :'created_pool_id', true);
DO $$
BEGIN
    IF (SELECT license FROM ple_api.read_current_published_question_pool(
            current_setting('ple.course_pool_created_pool_id')
        ) LIMIT 1) <> 'CC-BY-4.0' THEN
        RAISE EXCEPTION 'Question Pool creation did not calculate the exact member license';
    END IF;
END $$;
DO $$
BEGIN
    BEGIN
        PERFORM * FROM ple_api.create_question_pool(current_setting('ple.course_pool_duplicate_revisions_pool_id'),
            ARRAY[current_setting('ple.course_pool_question_id'), current_setting('ple.course_pool_question_id')], ARRAY[1, 2],
            'Duplicate revision pool', 'Duplicate revision Pool description', ARRAY[]::text[]);
        RAISE EXCEPTION 'two Revisions of one Question were accepted';
    EXCEPTION WHEN invalid_parameter_value THEN NULL;
    END;
END $$;
RESET ROLE;
SET LOCAL ROLE ple_api_owner;
DO $$
BEGIN
    IF (SELECT question_pool_edit_number
          FROM ple_data.save_question_pool_members(
              current_setting('ple.course_pool_created_pool_id'), 1,
              ARRAY[current_setting('ple.course_pool_replacement_question_id'), current_setting('ple.course_pool_question_id')],
              ARRAY[1, 1]
          )) <> 1 THEN
        RAISE EXCEPTION 'tuple-set reorder advanced the Question Pool Edit Number';
    END IF;
    BEGIN
        PERFORM * FROM ple_data.save_question_pool_members(current_setting('ple.course_pool_created_pool_id'), 1,
            ARRAY[current_setting('ple.course_pool_question_id'), current_setting('ple.course_pool_wrong_type_question_id')], ARRAY[1, 1]);
        RAISE EXCEPTION 'member save accepted a mismatched Question Type';
    EXCEPTION WHEN invalid_parameter_value THEN
        IF SQLERRM <> 'Question Pool members must share the established Type and Backend' THEN RAISE; END IF;
    END;
    BEGIN
        PERFORM * FROM ple_data.save_question_pool_members(current_setting('ple.course_pool_created_pool_id'), 1,
            ARRAY[current_setting('ple.course_pool_question_id'), current_setting('ple.course_pool_wrong_backend_question_id')], ARRAY[1, 1]);
        RAISE EXCEPTION 'member save accepted a mismatched Question Backend';
    EXCEPTION WHEN invalid_parameter_value THEN
        IF SQLERRM <> 'Question Pool members must share the established Type and Backend' THEN RAISE; END IF;
    END;
    BEGIN
        PERFORM * FROM ple_data.save_question_pool_members(current_setting('ple.course_pool_created_pool_id'), 1,
            ARRAY[current_setting('ple.course_pool_question_id'), current_setting('ple.course_pool_question_id')], ARRAY[1, 2]);
        RAISE EXCEPTION 'member save accepted two Revisions of one Question';
    EXCEPTION WHEN invalid_parameter_value THEN NULL;
    END;
END $$;
SELECT question_pool_edit_number AS changed_pool_edit
  FROM ple_data.save_question_pool_members(
      current_setting('ple.course_pool_created_pool_id'), 1,
      ARRAY[current_setting('ple.course_pool_replacement_question_id')], ARRAY[1]
  ) \gset
SELECT set_config('ple.course_pool_changed_pool_edit', :'changed_pool_edit', true);
DO $$
BEGIN
    IF current_setting('ple.course_pool_changed_pool_edit')::bigint <> 2 THEN
        RAISE EXCEPTION 'changed tuple set did not advance the Question Pool Edit Number';
    END IF;
END $$;
RESET ROLE;
SET LOCAL ROLE ple_app;
SELECT pool.question_pool_edit_number AS source_inspected_edit
  FROM ple_api.resolve_current_published_question_pool(current_setting('ple.course_pool_created_pool_id')) AS pool \gset
RESET ROLE;
SET LOCAL ROLE ple_api_owner;
SELECT question_pool_edit_number AS source_updated_edit
  FROM ple_data.save_question_pool_members(
      current_setting('ple.course_pool_created_pool_id'), :'source_inspected_edit',
      ARRAY[:'replacement_question_id'], ARRAY[1]
  ) \gset
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM ple_data.question_pool
         WHERE question_pool_id = current_setting('ple.course_pool_created_pool_id')
           AND license = 'CC-BY-4.0'
    ) THEN
        RAISE EXCEPTION 'Question Pool member replacement did not refresh the calculated license';
    END IF;
END $$;
RESET ROLE;
SET LOCAL ROLE ple_app;
SELECT set_config('ple.course_pool_metadata_id', current_setting('ple.course_pool_created_pool_id'), true);
DO $$
DECLARE
    pool_id text := current_setting('ple.course_pool_metadata_id');
    token bigint;
    saved_token bigint;
    stale_rejected boolean := false;
    member_edit bigint;
    saved_hint text;
    pool_metadata record;
BEGIN
    SELECT support.question_pool_metadata_edit_number INTO token
      FROM ple_api.read_question_pool_ple_managed_support(pool_id) AS support;
    IF token <> 1 THEN RAISE EXCEPTION 'Pool metadata token did not start at 1'; END IF;
    SELECT support.question_pool_metadata_edit_number INTO saved_token
      FROM ple_api.save_question_pool_ple_managed_support(
          pool_id, token, 'current hint', NULL, NULL
      ) AS support;
    IF saved_token <> 2 THEN RAISE EXCEPTION 'support replacement did not advance metadata token'; END IF;
    BEGIN
        PERFORM * FROM ple_api.save_question_pool_ple_managed_support(
            pool_id, token, 'stale hint', NULL, NULL
        );
        RAISE EXCEPTION 'stale support replacement was accepted';
    EXCEPTION WHEN serialization_failure THEN
        stale_rejected := true;
    END;
    IF NOT stale_rejected THEN RAISE EXCEPTION 'stale support replacement was not rejected'; END IF;

    SELECT * INTO pool_metadata
      FROM ple_api.read_current_question_pool_metadata(pool_id);
    SELECT result.question_pool_metadata_edit_number INTO saved_token
      FROM ple_api.replace_question_pool_metadata(
          pool_id, saved_token, pool_metadata.title, pool_metadata.description,
          pool_metadata.topic_uuid, pool_metadata.subtopic_uuid,
          ARRAY['current-tag'],
          pool_metadata.bloom_cognitive_process,
          pool_metadata.bloom_knowledge_dimension
      ) AS result;
    IF saved_token <> 3 THEN RAISE EXCEPTION 'Pool metadata replacement did not advance token'; END IF;
    stale_rejected := false;
    BEGIN
        PERFORM * FROM ple_api.replace_question_pool_metadata(
            pool_id, 2, pool_metadata.title, pool_metadata.description,
            pool_metadata.topic_uuid, pool_metadata.subtopic_uuid,
            ARRAY['stale-tag'],
            pool_metadata.bloom_cognitive_process,
            pool_metadata.bloom_knowledge_dimension
        );
        RAISE EXCEPTION 'stale Pool metadata replacement was accepted';
    EXCEPTION WHEN serialization_failure THEN
        stale_rejected := true;
    END;
    IF NOT stale_rejected THEN RAISE EXCEPTION 'stale search metadata replacement was not rejected'; END IF;

    SELECT result.question_pool_metadata_edit_number INTO saved_token
      FROM ple_api.bulk_replace_question_pool_search_metadata(
          jsonb_build_array(jsonb_build_object(
              'questionPoolId', pool_id,
              'questionPoolMetadataEditNumber', saved_token
          )),
          jsonb_build_object('tags', jsonb_build_array('bulk-current-tag'))
      ) AS result;
    IF saved_token <> 4 THEN RAISE EXCEPTION 'bulk search metadata replacement did not advance token'; END IF;
    stale_rejected := false;
    BEGIN
        PERFORM * FROM ple_api.bulk_replace_question_pool_search_metadata(
            jsonb_build_array(jsonb_build_object(
                'questionPoolId', pool_id,
                'questionPoolMetadataEditNumber', 3
            )),
            jsonb_build_object('tags', jsonb_build_array('stale-tag'))
        );
        RAISE EXCEPTION 'stale bulk search metadata replacement was accepted';
    EXCEPTION WHEN serialization_failure THEN
        stale_rejected := true;
    END;
    IF NOT stale_rejected THEN RAISE EXCEPTION 'stale bulk search metadata token was not rejected'; END IF;
    SELECT * INTO pool_metadata
      FROM ple_api.read_current_question_pool_metadata(pool_id);
    IF pool_metadata.tags IS DISTINCT FROM ARRAY['bulk-current-tag']::text[] THEN
        RAISE EXCEPTION 'bulk search metadata replacement did not retain the submitted tag';
    END IF;

    SELECT support.hint, support.question_pool_metadata_edit_number
      INTO saved_hint, token
      FROM ple_api.read_question_pool_ple_managed_support(pool_id) AS support;
    SELECT pool.question_pool_edit_number INTO member_edit
      FROM ple_api.resolve_current_published_question_pool(pool_id) AS pool;
    IF token <> 4 OR saved_hint <> 'current hint' OR member_edit <> 2 THEN
        RAISE EXCEPTION 'Pool metadata replacements changed membership or lost current state';
    END IF;
    RAISE NOTICE 'pool_metadata_replacements_use_separate_advancing_token';
END $$;
RESET ROLE;
SET LOCAL ROLE ple_api_owner;
SELECT question_pool_edit_number AS shared_pool_members_edit
  FROM ple_data.save_question_pool_members(
      current_setting('ple.course_pool_created_pool_id'), 2,
      ARRAY[current_setting('ple.course_pool_replacement_question_id'),
            current_setting('ple.course_pool_question_id')], ARRAY[1, 1]
  ) \gset
SELECT 'ASVR0002' || ple_private.crockford_checksum_character('ASVR0002') AS shared_pool_assessment_one_id \gset
SELECT 'ASVR0003' || ple_private.crockford_checksum_character('ASVR0003') AS shared_pool_assessment_two_id \gset
RESET ROLE;
SET LOCAL ROLE ple_app;
SELECT assessment_id AS created_assessment_one_id, assessment_edit_number AS created_assessment_one_edit
  FROM ple_api.create_assessment(
    :'shared_pool_assessment_one_id', :'course_id', 'regular_assignment',
    'Shared Pool assessment one', 'First Assessment sharing the reusable Pool'
) \gset
SELECT assessment_id AS created_assessment_two_id, assessment_edit_number AS created_assessment_two_edit
  FROM ple_api.create_assessment(
    :'shared_pool_assessment_two_id', :'course_id', 'regular_assignment',
    'Shared Pool assessment two', 'Second Assessment sharing the reusable Pool'
) \gset

RESET ROLE;
SET LOCAL ROLE ple_data_owner;
SELECT jsonb_build_object(
    'assessment_title', snapshot.assessment_title,
    'assessment_instructions', snapshot.assessment_instructions,
    'available_at', snapshot.available_at,
    'due_at', snapshot.due_at,
    'closes_at', snapshot.closes_at,
    'assessment_attempt_time_limit_seconds', snapshot.assessment_attempt_time_limit_seconds,
    'assessment_attempt_limit', snapshot.assessment_attempt_limit,
    'late_work_rule', snapshot.late_work_rule::text,
    'partial_credit_enabled', snapshot.partial_credit_enabled,
    'question_variation_rule', snapshot.question_variation_rule::text,
    'assessment_question_order_rule', snapshot.assessment_question_order_rule::text,
    'feedback_per_item_correctness', snapshot.feedback_per_item_correctness::text,
    'feedback_submitted_response', snapshot.feedback_submitted_response::text,
    'feedback_question_answer', snapshot.feedback_question_answer::text,
    'feedback_question_answer_explanation', snapshot.feedback_question_answer_explanation::text,
    'feedback_class_statistics', snapshot.feedback_class_statistics::text,
    'feedback_hints', snapshot.feedback_hints::text,
    'feedback_worked_solutions', snapshot.feedback_worked_solutions::text
)::text AS shared_pool_assessment_values
  FROM ple_data.assessment AS assessment
  JOIN ple_data.assessment_policy_snapshot AS snapshot
    ON snapshot.assessment_policy_snapshot_id = assessment.assessment_policy_snapshot_id
 WHERE assessment.assessment_id = :'created_assessment_one_id' \gset

RESET ROLE;
SET LOCAL ROLE ple_app;
SELECT set_config('ple.session_account_id', :'instructor_id', true);
SELECT set_config('ple.shared_pool_assessment_one_id', :'created_assessment_one_id', true);
SELECT set_config('ple.course_pool_created_assessment_one_edit', :'created_assessment_one_edit', true);
SELECT set_config('ple.course_pool_created_assessment_two_edit', :'created_assessment_two_edit', true);
SELECT set_config('ple.course_id', :'course_id', true);
SELECT set_config('ple.shared_pool_assessment_two_id', :'created_assessment_two_id', true);
SELECT set_config('ple.shared_pool_assessment_values', :'shared_pool_assessment_values', true);
SELECT set_config('ple.shared_pool_id', current_setting('ple.course_pool_created_pool_id'), true);
DO $$
DECLARE
    assessment_one text := current_setting('ple.shared_pool_assessment_one_id');
    assessment_two text := current_setting('ple.shared_pool_assessment_two_id');
    values_json jsonb := current_setting('ple.shared_pool_assessment_values')::jsonb;
    pool_id text := current_setting('ple.shared_pool_id');
    saved_one bigint;
    saved_two bigint;
BEGIN
    IF current_setting('ple.course_pool_created_assessment_one_edit') <> '1'
       OR current_setting('ple.course_pool_created_assessment_two_edit') <> '1' THEN
        RAISE EXCEPTION 'new Assessments did not start at Edit Number 1';
    END IF;
    SELECT result.assessment_edit_number INTO saved_one
      FROM ple_api.save_assessment(
        current_setting('ple.course_id'), assessment_one, 1, values_json,
        jsonb_build_array(jsonb_build_object(
            'assessmentEntryId', '73000000-0000-0000-0000-000000000091',
            'kind', 'question_pool',
            'availability', 'available',
            'scoringRule', 'normal',
            'authoredPosition', 0,
            'selectionCount', 1,
            'pointsPerItem', 1,
            'questionPoolId', pool_id
        ))
      ) AS result;
    SELECT result.assessment_edit_number INTO saved_two
      FROM ple_api.save_assessment(
        current_setting('ple.course_id'), assessment_two, 1, values_json,
        jsonb_build_array(jsonb_build_object(
            'assessmentEntryId', '73000000-0000-0000-0000-000000000092',
            'kind', 'question_pool',
            'availability', 'available',
            'scoringRule', 'normal',
            'authoredPosition', 0,
            'selectionCount', 2,
            'pointsPerItem', 1,
            'questionPoolId', pool_id
        ))
      ) AS result;
    IF saved_one <> 2 OR saved_two <> 2 THEN
        RAISE EXCEPTION 'direct Pool references did not save both Assessments';
    END IF;
END $$;

DO $$
DECLARE
    pool_id text := current_setting('ple.shared_pool_id');
    read_entries integer;
BEGIN
    SELECT count(DISTINCT workspace.assessment_entry_id) INTO read_entries
      FROM ple_api.load_assessment_workspace_rows(
          current_setting('ple.course_id'), current_setting('ple.shared_pool_assessment_one_id')
      ) AS workspace
     WHERE workspace.question_pool_id = pool_id
       AND workspace.selection_count = 1;
    IF read_entries <> 1 THEN
        RAISE EXCEPTION 'Assessment workspace read did not return the saved Pool ID and requested count';
    END IF;
    SELECT count(DISTINCT workspace.assessment_entry_id) INTO read_entries
      FROM ple_api.load_assessment_workspace_rows(
          current_setting('ple.course_id'), current_setting('ple.shared_pool_assessment_two_id')
      ) AS workspace
     WHERE workspace.question_pool_id = pool_id
       AND workspace.selection_count = 2;
    IF read_entries <> 1 THEN
        RAISE EXCEPTION 'second Assessment workspace read did not return its independent Pool count';
    END IF;
    RAISE NOTICE 'direct_pool_reference_save_and_workspace_read_round_trip';
END $$;

RESET ROLE;
SET LOCAL ROLE ple_data_owner;
DO $$
DECLARE
    pool_id text := current_setting('ple.shared_pool_id');
    actual_entries integer;
BEGIN
    SELECT count(*) INTO actual_entries
      FROM ple_data.assessment_entry_pool AS entry_pool
      JOIN ple_data.assessment AS assessment USING (assessment_id)
     WHERE entry_pool.question_pool_id = pool_id
       AND (assessment.assessment_id, entry_pool.selection_count) IN (
           (current_setting('ple.shared_pool_assessment_one_id'), 1),
           (current_setting('ple.shared_pool_assessment_two_id'), 2)
       );
    IF actual_entries <> 2 THEN
        RAISE EXCEPTION 'two Assessments did not retain independent counts for the same Pool';
    END IF;
    IF (SELECT owner_account_id FROM ple_data.question_pool WHERE question_pool_id = pool_id)
        <> current_setting('ple.session_account_id') THEN
        RAISE EXCEPTION 'shared Pool owner changed while Assessments saved references';
    END IF;
    RAISE NOTICE 'two_assessments_share_pool_with_independent_counts_and_unchanged_owner';
END $$;
COMMIT;
