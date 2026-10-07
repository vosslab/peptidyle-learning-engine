-- One connected PostgreSQL acceptance for Assessment Unrelease.  The leased
-- baseline owner invokes this after it has installed the canonical schema.
-- Fixture writes use schema-owner roles solely to construct otherwise-valid
-- Student Work; the operation under test is always invoked as ple_app with a
-- current-session account identity.
\set ON_ERROR_STOP on

BEGIN;
SET CONSTRAINTS ALL DEFERRED;

SET LOCAL ROLE ple_data_owner;
SELECT encode(ple_private.ensure_assessment_policy_snapshot(
    'Unrelease target', '',
    NULL, pg_catalog.transaction_timestamp() + INTERVAL '2 days', NULL, 5400, NULL,
    'accept', 'reuse_variation', 'authored_order',
    'after_submit', 'after_submit', 'after_submit', 'after_submit', 'after_submit',
    'regular_assignment', 'never', 'never', false
), 'hex') AS target_snapshot_id \gset
SELECT encode(ple_private.ensure_assessment_policy_snapshot(
    'Statistics survivor', '',
    NULL, pg_catalog.transaction_timestamp() + INTERVAL '2 days', NULL, 5400, NULL,
    'accept', 'reuse_variation', 'authored_order',
    'after_submit', 'after_submit', 'after_submit', 'after_submit', 'after_submit',
    'regular_assignment'
), 'hex') AS survivor_snapshot_id \gset
SELECT encode(ple_private.ensure_assessment_policy_snapshot(
    'Unrelease lock race', '',
    NULL, pg_catalog.transaction_timestamp() + INTERVAL '2 days', NULL, 5400, NULL,
    'accept', 'reuse_variation', 'authored_order',
    'after_submit', 'after_submit', 'after_submit', 'after_submit', 'after_submit',
    'regular_assignment'
), 'hex') AS race_snapshot_id \gset

-- Capture minted Account identities so earlier acceptance fixtures cannot
-- collide with the permanent public-ID registry.
SET LOCAL ROLE ple_private_owner;
INSERT INTO ple_private.account (account_id, user_role, created_at)
VALUES ('U00000009', 'instructor', pg_catalog.transaction_timestamp())
RETURNING account_id AS instructor_id \gset
SELECT set_config('ple.test_unrelease_instructor_id', :'instructor_id', false);
INSERT INTO ple_private.account (account_id, user_role, created_at)
VALUES ('U00000009', 'student', pg_catalog.transaction_timestamp())
RETURNING account_id AS student_id \gset
SELECT set_config('ple.test_unrelease_student_id', :'student_id', false);

SELECT 'ABCD-' || ple_private.crockford_checksum_character('ABCDEFG') || 'EFG' AS question_id \gset
SELECT set_config('ple.test_unrelease_question_id', :'question_id', false);

SET LOCAL ROLE ple_data_owner;
INSERT INTO ple_data.content_discipline (content_discipline_id, name)
VALUES ('20000000-0000-0000-0000-00000000cc01', 'Unrelease fixture discipline')
ON CONFLICT (content_discipline_id) DO NOTHING;
SELECT content_discipline_id AS discipline_id
  FROM ple_data.content_discipline
 WHERE content_discipline_id = '20000000-0000-0000-0000-00000000cc01' \gset
INSERT INTO ple_data.published_question (published_question_id, created_at)
VALUES (:'question_id', pg_catalog.transaction_timestamp());
INSERT INTO ple_data.question_revision (
    published_question_id, revision_number, backend, published_at
) VALUES (:'question_id', 1, 'ple', pg_catalog.transaction_timestamp());

SET LOCAL ROLE ple_private_owner;
INSERT INTO ple_private.object_record (
    object_record_id, object_address, object_storage_area, object_data_class, sha256,
    size_bytes, media_type, created_at
) VALUES (
    '20000000-0000-0000-0000-000000000010',
    jsonb_build_object(
        'kind', 'questionSource',
        'publishedQuestionRevisionTuple', jsonb_build_object(
            'publishedQuestionId', :'question_id',
            'revisionNumber', 1
        ),
        'objectId', '20000000-0000-0000-0000-000000000010'::uuid
    ),
    'private-content', 'question-source', decode(repeat('10', 32), 'hex'), 1,
    'application/json', pg_catalog.transaction_timestamp()
);
INSERT INTO ple_private.question_revision_source_binding (
    published_question_id, revision_number, backend, question_format, native_question_type,
    source_object_record_id, source_object_checksum, created_at
) VALUES (
    :'question_id', 1, 'ple', 'pleQuestionJson', 'multipleChoice',
    '20000000-0000-0000-0000-000000000010', repeat('10', 32), pg_catalog.transaction_timestamp()
);

SET LOCAL ROLE ple_api_owner;
INSERT INTO ple_data.course_instance (
    course_instance_id, source_kind, course_short_name, course_long_name,
    content_discipline_id, tags, term_starts_on, term_ends_on, created_at
) VALUES (
    'CINR00000' || ple_private.crockford_checksum_character('CINR00000'),
    'empty', 'UNR-1', 'Unrelease acceptance Course',
    :'discipline_id',
    ARRAY[]::text[], current_date, current_date + 1,
    pg_catalog.transaction_timestamp()
)
RETURNING course_instance_id AS course_id \gset
SELECT set_config('ple.test_unrelease_course_id', :'course_id', false);

INSERT INTO ple_data.course_membership (
    course_membership_id, course_instance_id, account_id, role, joined_at
) VALUES (
    '30000000-0000-0000-0000-000000000003',
    :'course_id', :'instructor_id', 'instructor', pg_catalog.transaction_timestamp()
);
INSERT INTO ple_data.student_record (
    student_record_id, course_instance_id, student_account_id, created_at
) VALUES (
    '30000000-0000-0000-0000-000000000002',
    :'course_id', :'student_id', pg_catalog.transaction_timestamp()
)
RETURNING student_record_id AS record_id \gset
INSERT INTO ple_data.course_membership (
    course_membership_id, course_instance_id, account_id, role, student_record_id, joined_at
) VALUES (
    '30000000-0000-0000-0000-000000000004',
    :'course_id', :'student_id', 'student', :'record_id', pg_catalog.transaction_timestamp()
);

-- This Pool is deliberately emptied after its Question is issued. The retained
-- Pool aggregate must continue to describe the issued Question's origin.
SET LOCAL ROLE ple_data_owner;
INSERT INTO ple_data.content_subject (content_subject_id, name)
VALUES ('20000000-0000-0000-0000-00000000cc02', 'Unrelease fixture subject')
ON CONFLICT (content_subject_id) DO NOTHING;
INSERT INTO ple_data.content_subject_discipline (content_subject_id, content_discipline_id)
VALUES (
    '20000000-0000-0000-0000-00000000cc02',
    '20000000-0000-0000-0000-00000000cc01'
)
ON CONFLICT (content_subject_id, content_discipline_id) DO NOTHING;
INSERT INTO ple_data.question_revision_metadata (
    published_question_id, revision_number, question_title, question_description,
    language, question_type, tags, content_discipline_id, content_subject_id,
    created_at, updated_at
) VALUES (
    :'question_id', 1, 'Unrelease statistics Question', 'Question used for statistics attribution',
    'en', 'multipleChoice', ARRAY[]::text[],
    '20000000-0000-0000-0000-00000000cc01',
    '20000000-0000-0000-0000-00000000cc02',
    pg_catalog.transaction_timestamp(), pg_catalog.transaction_timestamp()
);
INSERT INTO ple_data.question_revision_license (
    published_question_id, revision_number, spdx_expression
) VALUES (:'question_id', 1, 'CC-BY-4.0');
SELECT 'P001-' || ple_private.crockford_checksum_character('P001ABC') || 'ABC' AS pool_id \gset
SELECT set_config('ple.test_unrelease_pool_id', :'pool_id', false);
INSERT INTO ple_data.question_pool (
    question_pool_id, owner_account_id, question_pool_edit_number,
    question_pool_metadata_edit_number, question_type, backend, license,
    title, description, content_discipline_id, content_subject_id, created_at
) VALUES (
    :'pool_id', :'instructor_id', 1, 1, 'multipleChoice', 'ple', 'CC-BY-4.0',
    'Unrelease statistics Pool', 'Pool for origin attribution',
    '20000000-0000-0000-0000-00000000cc01',
    '20000000-0000-0000-0000-00000000cc02', pg_catalog.transaction_timestamp()
);
INSERT INTO ple_data.question_pool_member (
    question_pool_id, published_question_id, question_revision_number, created_at
) VALUES (:'pool_id', :'question_id', 1, pg_catalog.transaction_timestamp());
SELECT 'QRST-' || ple_private.crockford_checksum_character('QRSTABC') || 'ABC'
    AS replacement_question_id \gset
SELECT set_config('ple.test_unrelease_replacement_question_id', :'replacement_question_id', false);
INSERT INTO ple_data.published_question (published_question_id, created_at)
VALUES (:'replacement_question_id', pg_catalog.transaction_timestamp());
INSERT INTO ple_data.question_revision (
    published_question_id, revision_number, backend, published_at
) VALUES (
    :'replacement_question_id', 1, 'ple', pg_catalog.transaction_timestamp()
);
INSERT INTO ple_data.question_revision_metadata (
    published_question_id, revision_number, question_title, question_description,
    language, question_type, tags, content_discipline_id, content_subject_id,
    created_at, updated_at
) VALUES (
    :'replacement_question_id', 1, 'Replacement Pool member', 'Replacement member for edit',
    'en', 'multipleChoice', ARRAY[]::text[],
    '20000000-0000-0000-0000-00000000cc01',
    '20000000-0000-0000-0000-00000000cc02',
    pg_catalog.transaction_timestamp(), pg_catalog.transaction_timestamp()
);
INSERT INTO ple_data.question_revision_license (
    published_question_id, revision_number, spdx_expression
) VALUES (:'replacement_question_id', 1, 'CC-BY-4.0');

-- Target and survivor use the same immutable Question Revision.  The survivor
-- makes retained anonymous statistics distinguishable from remaining private
-- Student Work after the target is Unreleased.
INSERT INTO ple_data.assessment (
    assessment_id, course_instance_id, origin_kind, created_at, updated_at,
    assessment_type, assessment_policy_snapshot_id, assessment_status
) VALUES (
    'ANR00001' || ple_private.crockford_checksum_character('ANR00001'),
    :'course_id', 'direct',
    pg_catalog.transaction_timestamp(), pg_catalog.transaction_timestamp(),
    'regular_assignment', decode(:'target_snapshot_id', 'hex'), 'released'
)
RETURNING assessment_id AS target_assessment_id \gset
SELECT set_config('ple.test_unrelease_target_assessment_id', :'target_assessment_id', false);
INSERT INTO ple_data.assessment (
    assessment_id, course_instance_id, origin_kind, created_at, updated_at,
    assessment_type, assessment_policy_snapshot_id, assessment_status
) VALUES (
    'ANR00002' || ple_private.crockford_checksum_character('ANR00002'),
    :'course_id', 'direct',
    pg_catalog.transaction_timestamp(), pg_catalog.transaction_timestamp(),
    'regular_assignment', decode(:'survivor_snapshot_id', 'hex'), 'released'
)
RETURNING assessment_id AS survivor_assessment_id \gset
SELECT set_config('ple.test_unrelease_survivor_assessment_id', :'survivor_assessment_id', false);
INSERT INTO ple_data.assessment (
    assessment_id, course_instance_id, origin_kind, created_at, updated_at,
    assessment_type, assessment_policy_snapshot_id, assessment_status
) VALUES (
    'ANR00003' || ple_private.crockford_checksum_character('ANR00003'),
    :'course_id', 'direct',
    pg_catalog.transaction_timestamp(), pg_catalog.transaction_timestamp(),
    'regular_assignment', decode(:'race_snapshot_id', 'hex'), 'released'
)
RETURNING assessment_id AS race_assessment_id \gset

INSERT INTO ple_data.assessment_entry (
    assessment_entry_id, assessment_id, authored_position, entry_kind,
    availability, scoring_rule
) VALUES
    ('40000000-0000-0000-0000-000000000011', :'target_assessment_id',
     0, 'fixed_question', 'available', 'normal'),
    ('40000000-0000-0000-0000-000000000012', :'survivor_assessment_id',
     0, 'question_pool', 'available', 'normal'),
    ('40000000-0000-0000-0000-000000000013', :'race_assessment_id',
     0, 'fixed_question', 'available', 'normal');
INSERT INTO ple_data.assessment_entry_question (
    assessment_entry_id, assessment_id, published_question_id,
    question_revision_number, points_possible
) VALUES
    ('40000000-0000-0000-0000-000000000011', :'target_assessment_id', :'question_id', 1, 1),
    ('40000000-0000-0000-0000-000000000013', :'race_assessment_id', :'question_id', 1, 1);
INSERT INTO ple_data.assessment_entry_pool (
    assessment_entry_id, assessment_id, question_pool_id, selection_count, points_per_item
) VALUES (
    '40000000-0000-0000-0000-000000000012', :'survivor_assessment_id', :'pool_id', 1, 1
);

-- Start direct and Pool-selected Attempts through the ordinary restricted
-- path. The fixture saves Student responses, then finalizes them through the
-- production API before exercising destructive closure and retained statistics.
SET LOCAL ROLE ple_app;
SELECT set_config('ple.session_account_id', :'student_id', true);
SELECT assessment_attempt_id, assessment_attempt_number, resumed
  FROM ple_api.start_assessment_attempt(
    '50000000-0000-0000-0000-000000000001',
    :'record_id',
    :'target_assessment_id',
    '[]'::jsonb,
    jsonb_build_array(jsonb_build_object(
        'issued_question_id', '50000000-0000-0000-0000-000000000011',
        'assessment_entry_id', '40000000-0000-0000-0000-000000000011',
        'issued_position', 0,
        'published_question_id', :'question_id',
        'revision_number', 1
    ))
);
SELECT assessment_attempt_id, assessment_attempt_number, resumed
  FROM ple_api.start_assessment_attempt(
    '50000000-0000-0000-0000-000000000002',
    :'record_id',
    :'survivor_assessment_id',
    jsonb_build_array(jsonb_build_object(
        'question_pool_selection_id', '50000000-0000-0000-0000-000000000013',
        'assessment_entry_id', '40000000-0000-0000-0000-000000000012',
        'question_pool_id', :'pool_id',
        'question_pool_edit_number', 1,
        'selected_items', jsonb_build_array(jsonb_build_object(
            'published_question_id', :'question_id', 'revision_number', 1
        ))
    )),
    jsonb_build_array(jsonb_build_object(
        'issued_question_id', '50000000-0000-0000-0000-000000000012',
        'assessment_entry_id', '40000000-0000-0000-0000-000000000012',
        'issued_position', 0,
        'published_question_id', :'question_id',
        'revision_number', 1,
        'question_pool_selection_id', '50000000-0000-0000-0000-000000000013'
    ))
);
RESET ROLE;
SET LOCAL ROLE ple_data_owner;
DO $$
BEGIN
    IF EXISTS (
        SELECT 1 FROM ple_data.question_revision_statistics
         WHERE published_question_id = current_setting('ple.test_unrelease_question_id')
           AND revision_number = 1 AND issued_count > 0
    ) OR EXISTS (
        SELECT 1 FROM ple_data.question_pool_statistics
         WHERE question_pool_id = current_setting('ple.test_unrelease_pool_id')
           AND issued_count > 0
    ) THEN
        RAISE EXCEPTION 'starting attempts counted Questions before committed presentation';
    END IF;
END $$;
SET LOCAL ROLE ple_app;
-- Commit real Student presentation records. Repeating the first presentation
-- exercises the production retry path and must not recount its delivery.
SELECT *
  FROM ple_api.commit_student_assessment_attempt_presentation(
    '50000000-0000-0000-0000-000000000001',
    jsonb_build_array(jsonb_build_object(
        'issued_question_id', '50000000-0000-0000-0000-000000000011',
        'question_attempt_id', '50000000-0000-0000-0000-000000000021',
        'generated_parameter_sha256', NULL,
        'backend_version', '1',
        'renderer_name', NULL,
        'renderer_version', NULL,
        'grader_name', 'ple',
        'grader_version', '1',
        'rendered_question_sha256', repeat('b', 64),
        'issued_capability', 'ple_question_json_presentation',
        'presentation_nonce', repeat('1', 32),
        'presentation_checksum', repeat('c', 64),
        'presentation', jsonb_build_object('question', 'Unrelease fixture'),
        'backend_document', NULL,
        'response_item_bindings', '[]'::jsonb
    ))
);
SELECT *
  FROM ple_api.commit_student_assessment_attempt_presentation(
    '50000000-0000-0000-0000-000000000001',
    jsonb_build_array(jsonb_build_object(
        'issued_question_id', '50000000-0000-0000-0000-000000000011',
        'question_attempt_id', '50000000-0000-0000-0000-000000000021',
        'generated_parameter_sha256', NULL,
        'backend_version', '1',
        'renderer_name', NULL,
        'renderer_version', NULL,
        'grader_name', 'ple',
        'grader_version', '1',
        'rendered_question_sha256', repeat('b', 64),
        'issued_capability', 'ple_question_json_presentation',
        'presentation_nonce', repeat('1', 32),
        'presentation_checksum', repeat('c', 64),
        'presentation', jsonb_build_object('question', 'Unrelease fixture'),
        'backend_document', NULL,
        'response_item_bindings', '[]'::jsonb
    ))
);
-- The Rust service opens a transaction per issuance; this oracle shares one,
-- so restore its deferred constraint mode after the preceding API call.
SET CONSTRAINTS ALL DEFERRED;
SELECT *
  FROM ple_api.commit_student_assessment_attempt_presentation(
    '50000000-0000-0000-0000-000000000002',
    jsonb_build_array(jsonb_build_object(
        'issued_question_id', '50000000-0000-0000-0000-000000000012',
        'question_attempt_id', '50000000-0000-0000-0000-000000000022',
        'generated_parameter_sha256', NULL,
        'backend_version', '1',
        'renderer_name', NULL,
        'renderer_version', NULL,
        'grader_name', 'ple',
        'grader_version', '1',
        'rendered_question_sha256', repeat('d', 64),
        'issued_capability', 'ple_question_json_presentation',
        'presentation_nonce', repeat('2', 32),
        'presentation_checksum', repeat('e', 64),
        'presentation', jsonb_build_object('question', 'Unrelease fixture'),
        'backend_document', NULL,
        'response_item_bindings', '[]'::jsonb
    ))
);
RESET ROLE;
SET LOCAL ROLE ple_data_owner;
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM ple_data.question_revision_statistics
         WHERE published_question_id = current_setting('ple.test_unrelease_question_id')
           AND revision_number = 1 AND issued_count = 2 AND answered_count = 0
    ) THEN
        RAISE EXCEPTION 'production presentations did not count each delivery exactly once before submission';
    END IF;
END $$;
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM ple_data.question_pool_statistics
         WHERE question_pool_id = current_setting('ple.test_unrelease_pool_id')
           AND issued_count = 1 AND answered_count = 0
    ) THEN
        RAISE EXCEPTION 'Pool delivery count was not recorded once';
    END IF;
END $$;
SET LOCAL ROLE ple_private_owner;
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM ple_private.question_statistics_observation_receipt
         WHERE issued_question_id = '50000000-0000-0000-0000-000000000012'
           AND question_pool_id = current_setting('ple.test_unrelease_pool_id')
    ) OR NOT EXISTS (
        SELECT 1 FROM ple_private.question_pool_selection
         WHERE question_pool_selection_id = '50000000-0000-0000-0000-000000000013'
           AND question_pool_id = current_setting('ple.test_unrelease_pool_id')
           AND question_pool_edit_number = 1
    ) THEN
        RAISE EXCEPTION 'Pool delivery evidence did not retain its originating Pool and Edit Number';
    END IF;
END $$;
INSERT INTO ple_private.assessment_attempt_saved_response (
    course_instance_id, question_attempt_id, student_response, saved_at
)
SELECT attempt.course_instance_id,
       attempt.question_attempt_id,
       '{}'::jsonb AS student_response,
       pg_catalog.transaction_timestamp() AS saved_at
  FROM ple_private.question_attempt AS attempt
 WHERE attempt.question_attempt_id IN (
     '50000000-0000-0000-0000-000000000021',
     '50000000-0000-0000-0000-000000000022'
 );
SELECT floor(extract(epoch FROM saved_at) * 1000)::bigint AS target_saved_at_millis
  FROM ple_private.assessment_attempt_saved_response
 WHERE question_attempt_id = '50000000-0000-0000-0000-000000000021' \gset
SELECT floor(extract(epoch FROM saved_at) * 1000)::bigint AS survivor_saved_at_millis
  FROM ple_private.assessment_attempt_saved_response
 WHERE question_attempt_id = '50000000-0000-0000-0000-000000000022' \gset
RESET ROLE;
SET LOCAL ROLE ple_app;
SELECT set_config('ple.session_account_id', :'student_id', true);
SELECT * FROM ple_api.commit_student_assessment_attempt_finalization(
    '50000000-0000-0000-0000-000000000001', 'student',
    jsonb_build_array(jsonb_build_object(
        'question_attempt_id', '50000000-0000-0000-0000-000000000021',
        'saved_at_millis', :'target_saved_at_millis'::bigint,
        'student_response', '{}'::jsonb,
        'normalized_credit', 0.5
    ))
);
SELECT * FROM ple_api.commit_student_assessment_attempt_finalization(
    '50000000-0000-0000-0000-000000000002', 'student',
    jsonb_build_array(jsonb_build_object(
        'question_attempt_id', '50000000-0000-0000-0000-000000000022',
        'saved_at_millis', :'survivor_saved_at_millis'::bigint,
        'student_response', '{}'::jsonb,
        'normalized_credit', 1
    ))
);
-- A repeated submission finalization is idempotent; the stored Backend
-- fractions remain 0.5 and 1 regardless of the awarded-point projection.
DO $$
DECLARE earned double precision;
DECLARE possible double precision;
BEGIN
    SELECT points_earned, points_possible INTO earned, possible
      FROM ple_api.commit_student_assessment_attempt_finalization(
          '50000000-0000-0000-0000-000000000001', 'student', '[]'::jsonb
      );
    IF earned <> 0 OR possible <> 1 THEN
        RAISE EXCEPTION 'disabled partial credit did not affect awarded points as expected';
    END IF;
END $$;
RESET ROLE;
SET LOCAL ROLE ple_data_owner;
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM ple_data.question_revision_statistics
         WHERE published_question_id = current_setting('ple.test_unrelease_question_id') AND revision_number = 1
           AND issued_count = 2 AND answered_count = 2 AND correct_count = 1
           AND partial_count = 1 AND credit_sum = 1.5 AND credit_sum_sq = 1.25
           AND blank_count = 0
    ) THEN
        RAISE EXCEPTION 'production statistics capture did not count each accepted grade exactly once';
    END IF;
END $$;
-- Retire the survivor's Pool entry after both issued Attempts are finalized.
-- Derive the Assessment values from its current policy snapshot so this edit
-- preserves the existing policy exactly.
SELECT jsonb_build_object(
        'assessment_title', snapshot.assessment_title,
        'assessment_instructions', snapshot.assessment_instructions,
        'available_at', snapshot.available_at, 'due_at', snapshot.due_at,
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
    )::text AS survivor_values
  FROM ple_data.assessment AS assessment
  JOIN ple_data.assessment_policy_snapshot AS snapshot
    ON snapshot.assessment_policy_snapshot_id = assessment.assessment_policy_snapshot_id
 WHERE assessment.assessment_id = :'survivor_assessment_id' \gset
SET LOCAL ROLE ple_app;
SELECT set_config('ple.session_account_id', :'instructor_id', true);
SELECT assessment_edit_number AS survivor_edit
  FROM ple_api.save_assessment(
      :'course_id', :'survivor_assessment_id', 1, :'survivor_values'::jsonb,
      jsonb_build_array(jsonb_build_object(
          'assessmentEntryId', '40000000-0000-0000-0000-000000000012',
          'kind', 'question_pool', 'availability', 'retired', 'scoringRule', 'normal',
          'authoredPosition', 0, 'selectionCount', 1, 'pointsPerItem', 1,
          'questionPoolId', :'pool_id'
      ))
  ) \gset
RESET ROLE;
-- Presentation commits make constraints immediate in this shared test
-- transaction; the Rust Pool save gets a fresh transaction with deferred checks.
SET CONSTRAINTS ALL DEFERRED;
SET LOCAL ROLE ple_api_owner;
SELECT question_pool_edit_number
  FROM ple_data.save_question_pool_members(
      current_setting('ple.test_unrelease_pool_id'), 1,
      ARRAY[:'replacement_question_id'], ARRAY[1]
  );
SET CONSTRAINTS ALL IMMEDIATE;
RESET ROLE;
SET LOCAL ROLE ple_data_owner;
DO $$
BEGIN
    IF EXISTS (
        SELECT 1 FROM ple_data.question_pool_member
         WHERE question_pool_id = current_setting('ple.test_unrelease_pool_id')
           AND published_question_id = current_setting('ple.test_unrelease_question_id')
    ) OR NOT EXISTS (
        SELECT 1 FROM ple_data.question_pool_member
         WHERE question_pool_id = current_setting('ple.test_unrelease_pool_id')
           AND published_question_id = current_setting('ple.test_unrelease_replacement_question_id')
    ) OR NOT EXISTS (
        SELECT 1 FROM ple_data.question_pool
         WHERE question_pool_id = current_setting('ple.test_unrelease_pool_id')
           AND question_pool_edit_number = 2
    ) OR NOT EXISTS (
        SELECT 1 FROM ple_data.question_pool_statistics
         WHERE question_pool_id = current_setting('ple.test_unrelease_pool_id')
           AND issued_count = 1 AND answered_count = 1
    ) OR NOT EXISTS (
        SELECT 1 FROM ple_data.assessment_entry
         WHERE assessment_entry_id = '40000000-0000-0000-0000-000000000012'
           AND assessment_id = current_setting('ple.test_unrelease_survivor_assessment_id')
           AND availability = 'retired'
    ) THEN
        RAISE EXCEPTION 'editing current Pool membership changed prior delivery statistics or failed to replace its member';
    END IF;
END $$;
RESET ROLE;
SET LOCAL ROLE ple_private_owner;
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM ple_private.question_statistics_observation_receipt
         WHERE issued_question_id = '50000000-0000-0000-0000-000000000012'
           AND question_pool_id = current_setting('ple.test_unrelease_pool_id')
    ) OR NOT EXISTS (
        SELECT 1 FROM ple_private.question_pool_selection
         WHERE question_pool_selection_id = '50000000-0000-0000-0000-000000000013'
           AND question_pool_id = current_setting('ple.test_unrelease_pool_id')
           AND question_pool_edit_number = 1
    ) THEN
        RAISE EXCEPTION 'Pool delivery evidence did not retain its originating Pool and Edit Number';
    END IF;
END $$;
RESET ROLE;
SET LOCAL ROLE ple_data_owner;
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM ple_data.question_pool_statistics
         WHERE question_pool_id = current_setting('ple.test_unrelease_pool_id')
           AND issued_count = 1 AND answered_count = 1 AND correct_count = 1
           AND partial_count = 0 AND incorrect_count = 0
           AND credit_sum = 1 AND credit_sum_sq = 1
    ) OR EXISTS (
        SELECT 1 FROM ple_data.question_revision_statistics
         WHERE published_question_id = current_setting('ple.test_unrelease_replacement_question_id')
           AND issued_count > 0
    ) THEN
        RAISE EXCEPTION 'Pool outcomes did not remain attributed to the originating Pool after a member edit';
    END IF;
END $$;
COMMIT;

-- Error cases are real public calls.  Each verifies the rejected transaction
-- leaves current state, retained evidence, statistics, and audit unchanged.
BEGIN;
SET LOCAL ROLE ple_app;
SELECT set_config('ple.session_account_id', :'student_id', true);
DO $$
BEGIN
    PERFORM * FROM ple_api.read_assessment_unrelease_impact(
        current_setting('ple.test_unrelease_course_id'), current_setting('ple.test_unrelease_target_assessment_id')
    );
    RAISE EXCEPTION 'foreign unrelease impact was enumerated';
EXCEPTION WHEN insufficient_privilege THEN NULL;
END $$;
SELECT set_config('ple.session_account_id', :'instructor_id', true);
DO $$
BEGIN
    PERFORM * FROM ple_api.unrelease_assessment(
        current_setting('ple.test_unrelease_course_id'), current_setting('ple.test_unrelease_target_assessment_id'),
        2, 'Unrelease target'
    );
    RAISE EXCEPTION 'stale Unrelease Edit Number was accepted';
EXCEPTION WHEN serialization_failure THEN NULL;
END $$;
DO $$
BEGIN
    PERFORM * FROM ple_api.unrelease_assessment(
        current_setting('ple.test_unrelease_course_id'), current_setting('ple.test_unrelease_target_assessment_id'),
        1, 'wrong title'
    );
    RAISE EXCEPTION 'wrong Unrelease confirmation title was accepted';
EXCEPTION WHEN invalid_parameter_value THEN NULL;
END $$;
RESET ROLE;
SET LOCAL ROLE ple_data_owner;
DO $$
BEGIN
    IF (SELECT assessment_status FROM ple_data.assessment
         WHERE assessment_id = current_setting('ple.test_unrelease_target_assessment_id')) <> 'released'
       OR (SELECT assessment_edit_number FROM ple_data.assessment
           WHERE assessment_id = current_setting('ple.test_unrelease_target_assessment_id')) <> 1 THEN
        RAISE EXCEPTION 'a rejected Unrelease changed current Assessment state';
    END IF;
END $$;
SET LOCAL ROLE ple_private_owner;
DO $$
BEGIN
    IF (SELECT count(*) FROM ple_private.assessment_attempt
         WHERE assessment_id = current_setting('ple.test_unrelease_target_assessment_id')) <> 1 THEN
        RAISE EXCEPTION 'a rejected Unrelease changed Student Work';
    END IF;
END $$;
SET LOCAL ROLE ple_audit_owner;
DO $$
BEGIN
    IF (SELECT count(*) FROM ple_audit.assessment_unrelease_event) <> 0 THEN
        RAISE EXCEPTION 'a rejected Unrelease wrote audit evidence';
    END IF;
END $$;
COMMIT;

-- The accepted public transition returns aggregate impact only, removes the
-- rooted closure, retains the current Assessment/entry and shared Revision,
-- retains both anonymous observations despite private evidence deletion, and records a
-- redacted aggregate audit receipt.
BEGIN;
SET LOCAL ROLE ple_app;
SELECT set_config('ple.session_account_id', :'instructor_id', true);
DO $$
DECLARE result record;
BEGIN
    SELECT * INTO result FROM ple_api.unrelease_assessment(
        current_setting('ple.test_unrelease_course_id'), current_setting('ple.test_unrelease_target_assessment_id'),
        1, 'Unrelease target'
    );
    IF result.assessment_status <> 'unreleased' OR result.assessment_edit_number <> 2
       OR result.assessment_attempt_count <> 1 OR result.finalized_saved_response_count <> 1
       OR result.assessment_submission_count <> 1 OR result.grading_result_count <> 1 THEN
        RAISE EXCEPTION 'accepted Unrelease returned an incorrect aggregate receipt';
    END IF;
END $$;
RESET ROLE;
SET LOCAL ROLE ple_data_owner;
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM ple_data.assessment
                   WHERE assessment_id = current_setting('ple.test_unrelease_target_assessment_id')
                     AND assessment_status = 'unreleased'
                     AND assessment_edit_number = 2)
       OR NOT EXISTS (SELECT 1 FROM ple_data.assessment_entry
                       WHERE assessment_id = current_setting('ple.test_unrelease_target_assessment_id'))
       OR NOT EXISTS (SELECT 1 FROM ple_data.question_revision
                       WHERE published_question_id = current_setting('ple.test_unrelease_question_id')
                         AND revision_number = 1) THEN
        RAISE EXCEPTION 'Unrelease did not preserve current Assessment or shared Question state';
    END IF;
END $$;
SET LOCAL ROLE ple_private_owner;
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM ple_private.assessment_attempt
                WHERE assessment_id = current_setting('ple.test_unrelease_target_assessment_id'))
       OR EXISTS (SELECT 1 FROM ple_private.assessment_attempt_saved_response
                   WHERE question_attempt_id = '50000000-0000-0000-0000-000000000021')
       OR EXISTS (SELECT 1 FROM ple_private.assessment_submission
                   WHERE assessment_attempt_id = '50000000-0000-0000-0000-000000000001')
       OR EXISTS (SELECT 1 FROM ple_private.grading_result
                   WHERE question_attempt_id = '50000000-0000-0000-0000-000000000021')
       OR EXISTS (SELECT 1 FROM ple_private.question_statistics_observation_receipt
                   WHERE issued_question_id = '50000000-0000-0000-0000-000000000011')
       OR NOT EXISTS (SELECT 1 FROM ple_private.question_statistics_observation_receipt
                       WHERE issued_question_id = '50000000-0000-0000-0000-000000000012')
       OR NOT EXISTS (SELECT 1 FROM ple_private.assessment_attempt
                       WHERE assessment_id = current_setting('ple.test_unrelease_survivor_assessment_id')) THEN
        RAISE EXCEPTION 'Unrelease did not preserve and delete the expected roots';
    END IF;
END $$;
SET LOCAL ROLE ple_data_owner;
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM ple_data.question_revision_statistics
         WHERE published_question_id = current_setting('ple.test_unrelease_question_id') AND revision_number = 1
           AND issued_count = 2 AND answered_count = 2 AND correct_count = 1
           AND partial_count = 1 AND blank_count = 0
           AND credit_sum = 1.5 AND credit_sum_sq = 1.25
    ) OR NOT EXISTS (
        SELECT 1 FROM ple_data.question_pool_statistics
         WHERE question_pool_id = current_setting('ple.test_unrelease_pool_id')
           AND issued_count = 1 AND answered_count = 1 AND correct_count = 1
           AND credit_sum = 1 AND credit_sum_sq = 1
    ) THEN
        RAISE EXCEPTION 'Unrelease changed retained anonymous Question Revision statistics';
    END IF;
END $$;
SET LOCAL ROLE ple_audit_owner;
DO $$
DECLARE event_json jsonb;
BEGIN
    SELECT to_jsonb(event) INTO event_json FROM ple_audit.assessment_unrelease_event AS event;
    IF (SELECT count(*) FROM ple_audit.assessment_unrelease_event) <> 1
       OR event_json ? 'student_record_id' OR event_json ? 'student_response'
       OR event_json ? 'points_earned'
       OR (event_json ->> 'assessment_attempt_count') <> '1' THEN
        RAISE EXCEPTION 'Unrelease audit receipt is not one redacted aggregate event';
    END IF;
END $$;
COMMIT;

-- A repeated lifecycle action remains a conflict and cannot create a second
-- audit record or alter retained statistics.  Recapturing the deleted grade
-- also cannot resurrect private evidence or increment anonymous totals.
BEGIN;
SET LOCAL ROLE ple_app;
SELECT set_config('ple.session_account_id', :'instructor_id', true);
DO $$
BEGIN
    PERFORM * FROM ple_api.unrelease_assessment(
        current_setting('ple.test_unrelease_course_id'), current_setting('ple.test_unrelease_target_assessment_id'),
        2, 'Unrelease target'
    );
    RAISE EXCEPTION 'Unreleased Assessment accepted a second Unrelease';
EXCEPTION WHEN object_not_in_prerequisite_state THEN NULL;
END $$;
RESET ROLE;
SET LOCAL ROLE ple_private_owner;
SELECT ple_private.capture_issued_question_statistics_observation(
    :'course_id', '50000000-0000-0000-0000-000000000011',
    '50000000-0000-0000-0000-000000000041', pg_catalog.transaction_timestamp()
);
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM ple_private.question_statistics_observation_receipt
                WHERE issued_question_id = '50000000-0000-0000-0000-000000000011') THEN
        RAISE EXCEPTION 'recapture resurrected deleted private statistics evidence';
    END IF;
END $$;
SET LOCAL ROLE ple_data_owner;
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM ple_data.question_revision_statistics
         WHERE published_question_id = current_setting('ple.test_unrelease_question_id') AND revision_number = 1
           AND issued_count = 2 AND answered_count = 2 AND correct_count = 1
           AND partial_count = 1 AND blank_count = 0
           AND credit_sum = 1.5 AND credit_sum_sq = 1.25
    ) OR NOT EXISTS (
        SELECT 1 FROM ple_data.question_pool_statistics
         WHERE question_pool_id = current_setting('ple.test_unrelease_pool_id')
           AND issued_count = 1 AND answered_count = 1 AND correct_count = 1
           AND credit_sum = 1 AND credit_sum_sq = 1
    ) THEN
        RAISE EXCEPTION 'rejected repeat or deleted-grade recapture changed retained statistics';
    END IF;
END $$;
SET LOCAL ROLE ple_audit_owner;
DO $$
BEGIN
    IF (SELECT count(*) FROM ple_audit.assessment_unrelease_event) <> 1 THEN
        RAISE EXCEPTION 'rejected lifecycle repeat wrote an audit event';
    END IF;
END $$;
COMMIT;

SELECT 'unrelease_connected_oracle_pass';
