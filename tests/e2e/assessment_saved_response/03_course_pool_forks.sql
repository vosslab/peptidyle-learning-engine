BEGIN;
SET LOCAL ROLE ple_data_owner;
INSERT INTO ple_data.content_subject (content_subject_id, name)
VALUES ('73000000-0000-0000-0000-00000000cc02', 'Saved response subject');
INSERT INTO ple_data.content_subject_discipline (content_subject_id, content_discipline_id)
VALUES (
    '73000000-0000-0000-0000-00000000cc02',
    '73000000-0000-0000-0000-00000000cc01'
);
INSERT INTO ple_data.published_question_metadata (
    published_question_id, question_title, question_description, language,
    tags, content_discipline_id, content_subject_id, created_at, updated_at
) VALUES (
    :'question_id', 'Saved response question', 'One available published question',
    'en', ARRAY[]::text[],
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
    published_question_id, revision_number, backend, question_type, published_at
) VALUES (
    :'archived_question_id', 1, 'ple', 'multipleChoice', pg_catalog.transaction_timestamp()
);
INSERT INTO ple_data.question_revision (
    published_question_id, revision_number, backend, question_type, published_at
) VALUES (
    :'replacement_question_id', 1, 'ple', 'multipleChoice', pg_catalog.transaction_timestamp()
);
INSERT INTO ple_data.published_question_metadata (
    published_question_id, question_title, question_description, language,
    tags, content_discipline_id, content_subject_id, created_at, updated_at
) VALUES (
    :'replacement_question_id', 'Replacement Pool question', 'Same-count replacement member',
    'en', ARRAY[]::text[],
    '73000000-0000-0000-0000-00000000cc01',
    '73000000-0000-0000-0000-00000000cc02',
    pg_catalog.transaction_timestamp(), pg_catalog.transaction_timestamp()
);
SELECT 'SVP1-' || ple_private.crockford_checksum_character('SVP1ABC') || 'ABC' AS published_pool_id \gset
SELECT 'SVF1-' || ple_private.crockford_checksum_character('SVF1ABC') || 'ABC' AS fork_pool_id \gset
SELECT 'SVX1-' || ple_private.crockford_checksum_character('SVX1ABC') || 'ABC' AS missing_pool_id \gset
SELECT 'SVY1-' || ple_private.crockford_checksum_character('SVY1ABC') || 'ABC' AS rejected_pool_id \gset
SELECT 'ASVR0002' || ple_private.crockford_checksum_character('ASVR0002') AS course_pool_assessment_id \gset

RESET ROLE;
SET LOCAL ROLE ple_app;
SELECT set_config('ple.session_account_id', :'instructor_id', true);
SELECT set_config('ple.course_pool_archived_question_id', :'archived_question_id', true);
SELECT set_config('ple.course_pool_rejected_pool_id', :'rejected_pool_id', true);
DO $$
BEGIN
    BEGIN
        PERFORM * FROM ple_api.create_question_pool(
            current_setting('ple.course_pool_rejected_pool_id'),
            ARRAY[current_setting('ple.course_pool_archived_question_id')],
            ARRAY[1], true, 'Unavailable pool', 'Unavailable pool description', ARRAY[]::text[]
        );
        RAISE EXCEPTION 'archived Question was accepted as a Question Pool member';
    EXCEPTION WHEN foreign_key_violation THEN
        IF SQLERRM <> 'Question Pool member is unavailable' THEN RAISE; END IF;
    END;
END $$;
SELECT question_pool_id AS created_pool_id, question_pool_edit_number AS created_pool_edit
  FROM ple_api.create_question_pool(
    :'published_pool_id', ARRAY[:'question_id'], ARRAY[1], true,
    'Published pool', 'Published pool of one available question', ARRAY[]::text[]
) \gset
SELECT pool.question_pool_edit_number AS source_inspected_edit
  FROM ple_api.resolve_current_published_question_pool(:'created_pool_id') AS pool \gset
RESET ROLE;
SET LOCAL ROLE ple_api_owner;
SELECT question_pool_edit_number AS source_updated_edit
  FROM ple_data.save_question_pool_members(
      :'created_pool_id', :'source_inspected_edit',
      ARRAY[:'replacement_question_id'], ARRAY[1], true
  ) \gset
RESET ROLE;
SET LOCAL ROLE ple_app;
SELECT set_config('ple.course_pool_metadata_id', :'created_pool_id', true);
DO $$
DECLARE
    pool_id text := current_setting('ple.course_pool_metadata_id');
    token bigint;
    saved_token bigint;
    stale_rejected boolean := false;
    member_edit bigint;
    saved_hint text;
    provenance_token bigint;
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

    SELECT result.question_pool_metadata_edit_number INTO saved_token
      FROM ple_api.bulk_replace_question_pool_search_metadata(
          jsonb_build_array(jsonb_build_object(
              'questionPoolId', pool_id,
              'questionPoolMetadataEditNumber', saved_token
          )),
          jsonb_build_object('tags', jsonb_build_array('current-tag'))
      ) AS result;
    IF saved_token <> 3 THEN RAISE EXCEPTION 'search metadata replacement did not advance token'; END IF;
    stale_rejected := false;
    BEGIN
        PERFORM * FROM ple_api.bulk_replace_question_pool_search_metadata(
            jsonb_build_array(jsonb_build_object(
                'questionPoolId', pool_id,
                'questionPoolMetadataEditNumber', 2
            )),
            jsonb_build_object('tags', jsonb_build_array('stale-tag'))
        );
        RAISE EXCEPTION 'stale search metadata replacement was accepted';
    EXCEPTION WHEN serialization_failure THEN
        stale_rejected := true;
    END;
    IF NOT stale_rejected THEN RAISE EXCEPTION 'stale search metadata replacement was not rejected'; END IF;

    SELECT ple_api.save_question_pool_provenance(
        pool_id, saved_token, ARRAY['Oracle author'], NULL, 'CC-BY-4.0', NULL, NULL
    ) INTO provenance_token;
    IF provenance_token <> 4 THEN RAISE EXCEPTION 'provenance replacement did not advance token'; END IF;
    stale_rejected := false;
    BEGIN
        PERFORM ple_api.save_question_pool_provenance(
            pool_id, 3, ARRAY['Stale author'], NULL, 'CC0-1.0', NULL, NULL
        );
        RAISE EXCEPTION 'stale provenance replacement was accepted';
    EXCEPTION WHEN serialization_failure THEN
        stale_rejected := true;
    END;
    IF NOT stale_rejected THEN RAISE EXCEPTION 'stale provenance replacement was not rejected'; END IF;

    SELECT support.hint, support.question_pool_metadata_edit_number
      INTO saved_hint, token
      FROM ple_api.read_question_pool_ple_managed_support(pool_id) AS support;
    SELECT pool.question_pool_edit_number INTO member_edit
      FROM ple_api.resolve_current_published_question_pool(pool_id) AS pool;
    SELECT provenance.question_pool_metadata_edit_number INTO provenance_token
      FROM ple_api.read_question_pool_provenance(pool_id) AS provenance;
    IF token <> 4 OR provenance_token <> 4 OR saved_hint <> 'current hint' OR member_edit <> 2 THEN
        RAISE EXCEPTION 'Pool metadata replacements changed membership or lost current state';
    END IF;
    RAISE NOTICE 'pool_metadata_replacements_use_separate_advancing_token';
END $$;
SELECT assessment_id AS created_assessment_id, assessment_edit_number AS created_assessment_edit
  FROM ple_api.create_assessment(
    :'course_pool_assessment_id', :'course_id', 'regular_assignment',
    'Course pool assessment', 'Instructions for the course pool assessment'
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
    'question_variation_rule', snapshot.question_variation_rule::text,
    'assessment_question_order_rule', snapshot.assessment_question_order_rule::text,
    'feedback_per_item_correctness', snapshot.feedback_per_item_correctness::text,
    'feedback_submitted_response', snapshot.feedback_submitted_response::text,
    'feedback_question_answer', snapshot.feedback_question_answer::text,
    'feedback_question_answer_explanation', snapshot.feedback_question_answer_explanation::text,
    'feedback_class_statistics', snapshot.feedback_class_statistics::text,
    'feedback_hints', snapshot.feedback_hints::text,
    'feedback_worked_solutions', snapshot.feedback_worked_solutions::text
)::text AS course_pool_values
  FROM ple_data.assessment AS assessment
  JOIN ple_data.assessment_policy_snapshot AS snapshot
    ON snapshot.assessment_policy_snapshot_id = assessment.assessment_policy_snapshot_id
 WHERE assessment.assessment_id = :'created_assessment_id' \gset

RESET ROLE;
SET LOCAL ROLE ple_app;
SELECT set_config('ple.session_account_id', :'instructor_id', true);
SELECT set_config('ple.course_pool_course_id', :'course_id', true);
SELECT set_config('ple.course_pool_assessment_id', :'created_assessment_id', true);
SELECT set_config('ple.course_pool_question_id', :'question_id', true);
SELECT set_config('ple.course_pool_archived_question_id', :'archived_question_id', true);
SELECT set_config('ple.course_pool_pool_id', :'created_pool_id', true);
SELECT set_config('ple.course_pool_inspected_source_edit', :'source_inspected_edit', true);
SELECT set_config('ple.course_pool_updated_source_edit', :'source_updated_edit', true);
SELECT set_config('ple.course_pool_replacement_question_id', :'replacement_question_id', true);
SELECT set_config('ple.course_pool_fork_id', :'fork_pool_id', true);
SELECT set_config('ple.course_pool_missing_pool_id', :'missing_pool_id', true);
SELECT set_config('ple.course_pool_values', :'course_pool_values', true);
SELECT set_config('ple.course_pool_created_edit', :'created_assessment_edit', true);
SELECT set_config('ple.released_assessment_id', :'assessment_id', true);
DO $$
DECLARE
    course_id text := current_setting('ple.course_pool_course_id');
    assessment_id text := current_setting('ple.course_pool_assessment_id');
    values_json jsonb := current_setting('ple.course_pool_values')::jsonb;
    denial_count integer := 0;
    source_stale_rejected boolean := false;
    saved_edit bigint;
    imported_pool text;
    imported_edit bigint;
BEGIN
    IF current_setting('ple.course_pool_created_edit') <> '1' THEN
        RAISE EXCEPTION 'new Course Assessment did not start at Edit Number 1';
    END IF;

    BEGIN
        PERFORM * FROM ple_api.save_assessment(
            course_id, assessment_id, 1, values_json,
            jsonb_build_array(jsonb_build_object(
                'assessmentEntryId', '73000000-0000-0000-0000-000000000084',
                'kind', 'fixed_question',
                'availability', 'available',
                'scoringRule', 'normal',
                'authoredPosition', 0,
                'questionId', current_setting('ple.course_pool_archived_question_id'),
                'revisionNumber', 1,
                'pointsPossible', 1
            ))
        );
        RAISE EXCEPTION 'archived Question pin was accepted';
    EXCEPTION WHEN invalid_parameter_value THEN
        IF SQLERRM <> 'New Assessment Question pins require an Available Question' THEN RAISE; END IF;
        denial_count := denial_count + 1;
    END;

    BEGIN
        PERFORM * FROM ple_api.save_assessment(
            course_id, assessment_id, 1, values_json,
            jsonb_build_array(jsonb_build_object(
                'assessmentEntryId', '73000000-0000-0000-0000-000000000085',
                'kind', 'question_pool',
                'availability', 'available',
                'scoringRule', 'normal',
                'authoredPosition', 0,
                'selectionCount', 1,
                'pointsPerItem', 1,
                'selectedQuestionOrder', 'question_pool_order',
                'questionPoolId', current_setting('ple.course_pool_pool_id')
            ))
        );
        RAISE EXCEPTION 'published Question Pool was attached without a fork';
    EXCEPTION WHEN invalid_parameter_value THEN
        IF SQLERRM <> 'Assessment Question Pool membership requires an immutable fork command' THEN
            RAISE;
        END IF;
        denial_count := denial_count + 1;
    END;

    BEGIN
        PERFORM * FROM ple_api.import_assessment_question_pool_fork_for_ids(
            course_id, assessment_id, '73000000-0000-0000-0000-000000000086',
            1, current_setting('ple.course_pool_fork_id'),
            current_setting('ple.course_pool_missing_pool_id'),
            1,
            1, 1, 1, 'question_pool_order', 'normal'
        );
        RAISE EXCEPTION 'missing Question Pool source was accepted';
    EXCEPTION WHEN foreign_key_violation THEN
        IF SQLERRM <> 'Question Pool source does not exist' THEN RAISE; END IF;
        denial_count := denial_count + 1;
    END;

    IF denial_count <> 3 THEN
        RAISE EXCEPTION 'course published-content denial count %', denial_count;
    END IF;

    SELECT result.assessment_edit_number INTO saved_edit
      FROM ple_api.save_assessment(
        course_id, assessment_id, 1, values_json,
        jsonb_build_array(jsonb_build_object(
            'assessmentEntryId', '73000000-0000-0000-0000-000000000087',
            'kind', 'fixed_question',
            'availability', 'available',
            'scoringRule', 'normal',
            'authoredPosition', 0,
            'questionId', current_setting('ple.course_pool_question_id'),
            'revisionNumber', 1,
            'pointsPossible', 1
        ))
    ) AS result;
    IF saved_edit <> 2 THEN
        RAISE EXCEPTION 'available Question pin did not advance the Assessment Edit Number';
    END IF;

    BEGIN
        PERFORM * FROM ple_api.import_assessment_question_pool_fork_for_ids(
            course_id, assessment_id, '73000000-0000-0000-0000-000000000088',
            saved_edit, current_setting('ple.course_pool_fork_id'),
            current_setting('ple.course_pool_pool_id'),
            current_setting('ple.course_pool_inspected_source_edit')::bigint,
            1, 1, 1, 'question_pool_order', 'normal'
        );
        RAISE EXCEPTION 'stale inspected Pool membership was accepted';
    EXCEPTION WHEN serialization_failure THEN
        source_stale_rejected := true;
    END;
    IF NOT source_stale_rejected THEN
        RAISE EXCEPTION 'stale inspected Pool membership was not rejected';
    END IF;

    SELECT result.question_pool_id, result.assessment_edit_number
      INTO imported_pool, imported_edit
      FROM ple_api.import_assessment_question_pool_fork_for_ids(
        course_id, assessment_id, '73000000-0000-0000-0000-000000000088',
        saved_edit, current_setting('ple.course_pool_fork_id'),
        current_setting('ple.course_pool_pool_id'),
        current_setting('ple.course_pool_updated_source_edit')::bigint,
        1, 1, 1, 'question_pool_order', 'normal'
    ) AS result;
    IF imported_pool <> current_setting('ple.course_pool_fork_id') OR imported_edit <> 3 THEN
        RAISE EXCEPTION 'Course Assessment import did not store the fork';
    END IF;
    PERFORM set_config('ple.course_pool_saved_edit', saved_edit::text, true);
    PERFORM set_config('ple.course_pool_imported_edit', imported_edit::text, true);
END $$;

RESET ROLE;
SET LOCAL ROLE ple_data_owner;
DO $$
DECLARE
    course_assessment_id text := current_setting('ple.course_pool_assessment_id');
    question_id text := current_setting('ple.course_pool_question_id');
    replacement_question_id text := current_setting('ple.course_pool_replacement_question_id');
    archived_question_id text := current_setting('ple.course_pool_archived_question_id');
    pool_id text := current_setting('ple.course_pool_pool_id');
    fork_id text := current_setting('ple.course_pool_fork_id');
    stored_pool text;
    source_pool text;
    published_source text;
    member_question text;
    member_revision integer;
    member_count integer;
    source_member_count integer;
    source_member_question text;
    source_member_edit bigint;
BEGIN
    IF current_setting('ple.course_pool_saved_edit') <> '2'
       OR current_setting('ple.course_pool_imported_edit') <> '3' THEN
        RAISE EXCEPTION 'Course Assessment edit sequence was not recorded';
    END IF;
    IF NOT EXISTS (
        SELECT 1 FROM ple_data.assessment
         WHERE assessment.assessment_id = current_setting('ple.released_assessment_id')
           AND assessment.assessment_status = 'released'
    ) THEN
        RAISE EXCEPTION 'released Assessment changed during the Course Pool proof';
    END IF;
    IF NOT EXISTS (
        SELECT 1 FROM ple_data.assessment
         WHERE assessment.assessment_id = course_assessment_id
           AND assessment.assessment_status = 'unreleased'
           AND assessment.assessment_edit_number = 3
    ) THEN
        RAISE EXCEPTION 'Course Assessment did not remain unreleased at Edit Number 3';
    END IF;
    IF NOT EXISTS (
        SELECT 1
          FROM ple_data.assessment_entry AS entry
          JOIN ple_data.assessment_entry_question AS question
            ON question.assessment_entry_id = entry.assessment_entry_id
         WHERE entry.assessment_id = course_assessment_id
           AND entry.entry_kind = 'fixed_question'
           AND entry.availability = 'available'
           AND question.published_question_id = question_id
           AND question.question_revision_number = 1
    ) THEN
        RAISE EXCEPTION 'Course Assessment did not pin the available published Question';
    END IF;
    IF EXISTS (
        SELECT 1
          FROM ple_data.assessment_entry_question AS question
         WHERE question.assessment_id = course_assessment_id
           AND question.published_question_id = archived_question_id
    ) THEN
        RAISE EXCEPTION 'Course Assessment stored the archived Question';
    END IF;

    SELECT pool_entry.question_pool_id, pool.source_question_pool_id
      INTO stored_pool, source_pool
      FROM ple_data.assessment_entry AS entry
      JOIN ple_data.assessment_entry_pool AS pool_entry
        ON pool_entry.assessment_entry_id = entry.assessment_entry_id
      JOIN ple_data.assessment_question_pool_fork AS owned
        ON owned.assessment_entry_id = entry.assessment_entry_id
       AND owned.question_pool_id = pool_entry.question_pool_id
      JOIN ple_data.question_pool AS pool
        ON pool.question_pool_id = pool_entry.question_pool_id
     WHERE entry.assessment_id = course_assessment_id
       AND entry.entry_kind = 'question_pool'
       AND entry.availability = 'available';
    IF stored_pool IS DISTINCT FROM fork_id OR source_pool IS DISTINCT FROM pool_id THEN
        RAISE EXCEPTION 'Course Assessment stored a Pool other than the published Pool fork';
    END IF;
    SELECT count(*), min(member.published_question_id), pool.question_pool_edit_number
      INTO source_member_count, source_member_question, source_member_edit
      FROM ple_data.question_pool AS pool
      JOIN ple_data.question_pool_member AS member
        ON member.question_pool_id = pool.question_pool_id
     WHERE pool.question_pool_id = pool_id
     GROUP BY pool.question_pool_id;
    IF source_member_count <> 1
       OR source_member_question IS DISTINCT FROM replacement_question_id
       OR source_member_edit <> current_setting('ple.course_pool_updated_source_edit')::bigint
       OR source_member_edit <> current_setting('ple.course_pool_inspected_source_edit')::bigint + 1 THEN
        RAISE EXCEPTION 'source Pool did not retain a same-count membership change at its advanced edit';
    END IF;
    SELECT published.source_question_pool_id INTO published_source
      FROM ple_data.question_pool AS published
     WHERE published.question_pool_id = pool_id;
    IF published_source IS NOT NULL THEN
        RAISE EXCEPTION 'created Question Pool was not an original published Pool';
    END IF;
    SELECT count(*), min(member.published_question_id), min(member.question_revision_number)
      INTO member_count, member_question, member_revision
      FROM ple_data.question_pool_member AS member
     WHERE member.question_pool_id = fork_id;
    IF member_count <> 1 OR member_question IS DISTINCT FROM replacement_question_id OR member_revision <> 1 THEN
        RAISE EXCEPTION 'fork membership was not the inspected-current available Question revision';
    END IF;
    IF EXISTS (
        SELECT 1 FROM ple_data.assessment_entry_pool AS pool_entry
         WHERE pool_entry.assessment_id = course_assessment_id
           AND pool_entry.question_pool_id = pool_id
    ) THEN
        RAISE EXCEPTION 'Course Assessment stored the published Question Pool id';
    END IF;
    RAISE NOTICE 'course_instance_contains_only_published_questions_and_published_pools';
END $$;
COMMIT;
