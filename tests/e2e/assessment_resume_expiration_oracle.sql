-- Disposable proof that a second start_assessment_attempt resumes the open
-- Attempt and leaves its stored expires_at unchanged. The calls are the
-- shipped function. This file does not restate that function.
\set ON_ERROR_STOP on

BEGIN;
SET CONSTRAINTS ALL DEFERRED;

SET LOCAL ROLE ple_private_owner;
SELECT 'URSR0000' || ple_private.crockford_checksum_character('URSR0000') AS instructor_id \gset
SELECT 'USRS0000' || ple_private.crockford_checksum_character('USRS0000') AS student_id \gset
INSERT INTO ple_private.account (account_id, user_role, created_at)
VALUES
    (:'instructor_id', 'instructor', pg_catalog.transaction_timestamp()),
    (:'student_id', 'student', pg_catalog.transaction_timestamp());

SET LOCAL ROLE ple_data_owner;
INSERT INTO ple_data.content_discipline (content_discipline_id, name)
VALUES ('72000000-0000-0000-0000-00000000cc01', 'Resume discipline');
SELECT 'RSM1-' || ple_private.crockford_checksum_character('RSM1XYZ') || 'XYZ' AS question_id \gset
INSERT INTO ple_data.published_question (published_question_id, created_at)
VALUES (:'question_id', pg_catalog.transaction_timestamp());
INSERT INTO ple_data.question_revision (
    published_question_id, revision_number, backend, published_at
) VALUES (:'question_id', 1, 'ple', pg_catalog.transaction_timestamp());
SELECT encode(ple_private.ensure_assessment_policy_snapshot(
    'Resume quiz', '', NULL,
    pg_catalog.transaction_timestamp() + interval '2 days',
    NULL, 1800, NULL,
    'accept', 'reuse_variation', 'authored_order',
    'after_submit', 'after_submit', 'after_submit', 'never', 'never',
    'regular_assignment'
), 'hex') AS snapshot_id \gset

SET LOCAL ROLE ple_private_owner;
INSERT INTO ple_private.object_record (
    object_record_id, object_address, object_storage_area, object_data_class, sha256,
    size_bytes, media_type, created_at
) VALUES (
    '72000000-0000-0000-0000-000000000010',
    jsonb_build_object(
        'kind', 'questionSource',
        'publishedQuestionRevisionTuple', jsonb_build_object(
            'publishedQuestionId', :'question_id',
            'revisionNumber', 1
        ),
        'objectId', '72000000-0000-0000-0000-000000000010'::uuid
    ),
    'private-content', 'question-source', decode(repeat('20', 32), 'hex'), 1,
    'application/json', pg_catalog.transaction_timestamp()
);
INSERT INTO ple_private.question_revision_source_binding (
    published_question_id, revision_number, backend, question_format, native_question_type,
    source_object_record_id, source_object_checksum, created_at
) VALUES (
    :'question_id', 1, 'ple', 'pleQuestionJson', 'multipleChoice',
    '72000000-0000-0000-0000-000000000010', repeat('20', 32), pg_catalog.transaction_timestamp()
);

SET LOCAL ROLE ple_api_owner;
INSERT INTO ple_data.course_instance (
    course_instance_id, source_kind, course_short_name, course_long_name,
    content_discipline_id, tags, term_starts_on, term_ends_on, created_at
) VALUES (
    'CIRS00000' || ple_private.crockford_checksum_character('CIRS00000'),
    'empty', 'RSM-1', 'Resume Course',
    '72000000-0000-0000-0000-00000000cc01',
    ARRAY[]::text[], current_date, current_date + 1,
    pg_catalog.transaction_timestamp()
) RETURNING course_instance_id AS course_id \gset
INSERT INTO ple_data.course_membership (
    course_membership_id, course_instance_id, account_id, role, joined_at
) VALUES (
    '72000000-0000-0000-0000-000000000301',
    :'course_id', :'instructor_id', 'instructor', pg_catalog.transaction_timestamp()
);
INSERT INTO ple_data.student_record (
    student_record_id, course_instance_id, student_account_id, created_at
) VALUES (
    '72000000-0000-0000-0000-000000000201',
    :'course_id', :'student_id', pg_catalog.transaction_timestamp()
);
INSERT INTO ple_data.course_membership (
    course_membership_id, course_instance_id, account_id, role, student_record_id, joined_at
) VALUES (
    '72000000-0000-0000-0000-000000000302',
    :'course_id', :'student_id', 'student',
    '72000000-0000-0000-0000-000000000201', pg_catalog.transaction_timestamp()
);

SET LOCAL ROLE ple_data_owner;
INSERT INTO ple_data.assessment (
    assessment_id, course_instance_id, origin_kind, created_at, updated_at,
    assessment_type, assessment_policy_snapshot_id
) VALUES (
    'ARSR0000' || ple_private.crockford_checksum_character('ARSR0000'),
    :'course_id', 'direct',
    pg_catalog.transaction_timestamp(), pg_catalog.transaction_timestamp(),
    'regular_assignment', decode(:'snapshot_id', 'hex')
) RETURNING assessment_id AS assessment_id \gset
INSERT INTO ple_data.assessment_entry (
    assessment_entry_id, assessment_id, authored_position, entry_kind,
    availability, scoring_rule
) VALUES (
    '72000000-0000-0000-0000-000000000081', :'assessment_id', 0, 'fixed_question', 'available', 'normal'
);
INSERT INTO ple_data.assessment_entry_question (
    assessment_entry_id, assessment_id, published_question_id,
    question_revision_number, points_possible
) VALUES (
    '72000000-0000-0000-0000-000000000081', :'assessment_id', :'question_id', 1, 1
);

SET LOCAL ROLE ple_app;
SELECT set_config('ple.session_account_id', :'instructor_id', true);
SELECT assessment_status AS released_status
  FROM ple_api.release_assessment(:'course_id', :'assessment_id', 1) \gset

SELECT set_config('ple.session_account_id', :'student_id', true);
SELECT assessment_attempt_id AS first_attempt_id,
       resumed::text AS first_resumed
  FROM ple_api.start_assessment_attempt(
    '72000000-0000-0000-0000-000000000001',
    '72000000-0000-0000-0000-000000000201',
    :'assessment_id',
    '[]'::jsonb,
    jsonb_build_array(jsonb_build_object(
        'issued_question_id', '72000000-0000-0000-0000-000000000011',
        'assessment_entry_id', '72000000-0000-0000-0000-000000000081',
        'issued_position', 0,
        'published_question_id', :'question_id',
        'revision_number', 1
    ))
) \gset

RESET ROLE;
SELECT expires_at::text AS first_expires_at
  FROM ple_private.assessment_attempt
 WHERE assessment_attempt_id = :'first_attempt_id'::uuid \gset

SET LOCAL ROLE ple_app;
SELECT set_config('ple.session_account_id', :'student_id', true);
SELECT assessment_attempt_id AS second_attempt_id,
       resumed::text AS second_resumed
  FROM ple_api.start_assessment_attempt(
    '72000000-0000-0000-0000-000000000002',
    '72000000-0000-0000-0000-000000000201',
    :'assessment_id',
    '[]'::jsonb,
    jsonb_build_array(jsonb_build_object(
        'issued_question_id', '72000000-0000-0000-0000-000000000012',
        'assessment_entry_id', '72000000-0000-0000-0000-000000000081',
        'issued_position', 0,
        'published_question_id', :'question_id',
        'revision_number', 1
    ))
) \gset

RESET ROLE;
SELECT expires_at::text AS second_expires_at
  FROM ple_private.assessment_attempt
 WHERE assessment_attempt_id = :'first_attempt_id'::uuid \gset
SELECT count(*)::text AS attempt_count
  FROM ple_private.assessment_attempt
 WHERE assessment_id = :'assessment_id' \gset
SELECT count(*)::text AS issued_count
  FROM ple_private.issued_question
 WHERE assessment_attempt_id = :'first_attempt_id'::uuid \gset
SELECT set_config('ple.resume_proof_first_resumed', :'first_resumed', true);
SELECT set_config('ple.resume_proof_second_resumed', :'second_resumed', true);
SELECT set_config('ple.resume_proof_first_attempt', :'first_attempt_id', true);
SELECT set_config('ple.resume_proof_second_attempt', :'second_attempt_id', true);
SELECT set_config('ple.resume_proof_first_expires', :'first_expires_at', true);
SELECT set_config('ple.resume_proof_second_expires', :'second_expires_at', true);
SELECT set_config('ple.resume_proof_attempt_count', :'attempt_count', true);
SELECT set_config('ple.resume_proof_issued_count', :'issued_count', true);
SELECT set_config('ple.resume_proof_released', :'released_status', true);

DO $$
BEGIN
    IF current_setting('ple.resume_proof_released') <> 'released'
       OR current_setting('ple.resume_proof_first_resumed') <> 'false'
       OR current_setting('ple.resume_proof_second_resumed') <> 'true'
       OR current_setting('ple.resume_proof_first_attempt')
          <> current_setting('ple.resume_proof_second_attempt')
       OR current_setting('ple.resume_proof_first_expires')
          <> current_setting('ple.resume_proof_second_expires')
       OR current_setting('ple.resume_proof_attempt_count') <> '1'
       OR current_setting('ple.resume_proof_issued_count') <> '1' THEN
        RAISE EXCEPTION 'resume proof failed released=% first_resumed=% second_resumed=% first=% second=% first_expires=% second_expires=% attempts=% issued=%',
            current_setting('ple.resume_proof_released'),
            current_setting('ple.resume_proof_first_resumed'),
            current_setting('ple.resume_proof_second_resumed'),
            current_setting('ple.resume_proof_first_attempt'),
            current_setting('ple.resume_proof_second_attempt'),
            current_setting('ple.resume_proof_first_expires'),
            current_setting('ple.resume_proof_second_expires'),
            current_setting('ple.resume_proof_attempt_count'),
            current_setting('ple.resume_proof_issued_count');
    END IF;
    RAISE NOTICE 'start_assessment_attempt resume same_attempt kept_expires_at';
END $$;

-- Two authenticated browser sessions for the same Student. Each session is
-- installed by the shipped session function, then start resumes the open Attempt.
SET LOCAL ROLE ple_auth;
SELECT session_id AS first_browser_session_id
  FROM ple_api.create_authenticated_session(
    '72000000-0000-0000-0000-0000000000a1',
    :'student_id',
    decode(repeat('a1', 32), 'hex'),
    3600
  ) \gset
SELECT session_id AS second_browser_session_id
  FROM ple_api.create_authenticated_session(
    '72000000-0000-0000-0000-0000000000a2',
    :'student_id',
    decode(repeat('b2', 32), 'hex'),
    3600
  ) \gset
SELECT session_id AS installed_first_session_id
  FROM ple_api.resolve_and_install_session(decode(repeat('a1', 32), 'hex')) \gset
SET LOCAL ROLE ple_app;
SELECT assessment_attempt_id AS third_attempt_id,
       resumed::text AS third_resumed
  FROM ple_api.start_assessment_attempt(
    '72000000-0000-0000-0000-000000000003',
    '72000000-0000-0000-0000-000000000201',
    :'assessment_id',
    '[]'::jsonb,
    jsonb_build_array(jsonb_build_object(
        'issued_question_id', '72000000-0000-0000-0000-000000000013',
        'assessment_entry_id', '72000000-0000-0000-0000-000000000081',
        'issued_position', 0,
        'published_question_id', :'question_id',
        'revision_number', 1
    ))
) \gset
RESET ROLE;
SELECT expires_at::text AS third_expires_at
  FROM ple_private.assessment_attempt
 WHERE assessment_attempt_id = :'first_attempt_id'::uuid \gset

SET LOCAL ROLE ple_auth;
SELECT session_id AS installed_second_session_id
  FROM ple_api.resolve_and_install_session(decode(repeat('b2', 32), 'hex')) \gset
SET LOCAL ROLE ple_app;
SELECT assessment_attempt_id AS fourth_attempt_id,
       resumed::text AS fourth_resumed
  FROM ple_api.start_assessment_attempt(
    '72000000-0000-0000-0000-000000000004',
    '72000000-0000-0000-0000-000000000201',
    :'assessment_id',
    '[]'::jsonb,
    jsonb_build_array(jsonb_build_object(
        'issued_question_id', '72000000-0000-0000-0000-000000000014',
        'assessment_entry_id', '72000000-0000-0000-0000-000000000081',
        'issued_position', 0,
        'published_question_id', :'question_id',
        'revision_number', 1
    ))
) \gset
RESET ROLE;
SELECT expires_at::text AS fourth_expires_at
  FROM ple_private.assessment_attempt
 WHERE assessment_attempt_id = :'first_attempt_id'::uuid \gset
SELECT count(*)::text AS session_attempt_count
  FROM ple_private.assessment_attempt
 WHERE assessment_id = :'assessment_id' \gset
SELECT count(*)::text AS session_issued_count
  FROM ple_private.issued_question
 WHERE assessment_attempt_id = :'first_attempt_id'::uuid \gset
SELECT set_config('ple.resume_proof_first_browser_session', :'first_browser_session_id', true);
SELECT set_config('ple.resume_proof_second_browser_session', :'second_browser_session_id', true);
SELECT set_config('ple.resume_proof_installed_first_session', :'installed_first_session_id', true);
SELECT set_config('ple.resume_proof_installed_second_session', :'installed_second_session_id', true);
SELECT set_config('ple.resume_proof_third_attempt', :'third_attempt_id', true);
SELECT set_config('ple.resume_proof_fourth_attempt', :'fourth_attempt_id', true);
SELECT set_config('ple.resume_proof_third_resumed', :'third_resumed', true);
SELECT set_config('ple.resume_proof_fourth_resumed', :'fourth_resumed', true);
SELECT set_config('ple.resume_proof_third_expires', :'third_expires_at', true);
SELECT set_config('ple.resume_proof_fourth_expires', :'fourth_expires_at', true);
SELECT set_config('ple.resume_proof_session_attempt_count', :'session_attempt_count', true);
SELECT set_config('ple.resume_proof_session_issued_count', :'session_issued_count', true);

DO $$
BEGIN
    IF current_setting('ple.resume_proof_first_browser_session')
          = current_setting('ple.resume_proof_second_browser_session')
       OR current_setting('ple.resume_proof_installed_first_session')
          <> current_setting('ple.resume_proof_first_browser_session')
       OR current_setting('ple.resume_proof_installed_second_session')
          <> current_setting('ple.resume_proof_second_browser_session')
       OR current_setting('ple.resume_proof_third_resumed') <> 'true'
       OR current_setting('ple.resume_proof_fourth_resumed') <> 'true'
       OR current_setting('ple.resume_proof_third_attempt')
          <> current_setting('ple.resume_proof_first_attempt')
       OR current_setting('ple.resume_proof_fourth_attempt')
          <> current_setting('ple.resume_proof_first_attempt')
       OR current_setting('ple.resume_proof_third_expires')
          <> current_setting('ple.resume_proof_first_expires')
       OR current_setting('ple.resume_proof_fourth_expires')
          <> current_setting('ple.resume_proof_first_expires')
       OR current_setting('ple.resume_proof_session_attempt_count') <> '1'
       OR current_setting('ple.resume_proof_session_issued_count') <> '1' THEN
        RAISE EXCEPTION 'distinct browser session resume failed first_session=% second_session=% installed_first=% installed_second=% third_resumed=% fourth_resumed=% third=% fourth=% first=% third_expires=% fourth_expires=% first_expires=% attempts=% issued=%',
            current_setting('ple.resume_proof_first_browser_session'),
            current_setting('ple.resume_proof_second_browser_session'),
            current_setting('ple.resume_proof_installed_first_session'),
            current_setting('ple.resume_proof_installed_second_session'),
            current_setting('ple.resume_proof_third_resumed'),
            current_setting('ple.resume_proof_fourth_resumed'),
            current_setting('ple.resume_proof_third_attempt'),
            current_setting('ple.resume_proof_fourth_attempt'),
            current_setting('ple.resume_proof_first_attempt'),
            current_setting('ple.resume_proof_third_expires'),
            current_setting('ple.resume_proof_fourth_expires'),
            current_setting('ple.resume_proof_first_expires'),
            current_setting('ple.resume_proof_session_attempt_count'),
            current_setting('ple.resume_proof_session_issued_count');
    END IF;
    RAISE NOTICE 'start_assessment_attempt resume same_attempt from_distinct_browser_sessions';
END $$;
COMMIT;
