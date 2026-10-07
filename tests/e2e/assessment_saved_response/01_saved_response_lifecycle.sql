BEGIN;
SET CONSTRAINTS ALL DEFERRED;

SET LOCAL ROLE ple_private_owner;
SELECT 'UVSV0000' || ple_private.crockford_checksum_character('UVSV0000') AS instructor_id \gset
SELECT 'USSV0000' || ple_private.crockford_checksum_character('USSV0000') AS student_id \gset
SELECT 'UVSV0002' || ple_private.crockford_checksum_character('UVSV0002') AS sysadmin_id \gset
INSERT INTO ple_private.account (account_id, user_role, created_at)
VALUES
    (:'instructor_id', 'instructor', pg_catalog.transaction_timestamp()),
    (:'student_id', 'student', pg_catalog.transaction_timestamp()),
    (:'sysadmin_id', 'sysadmin', pg_catalog.transaction_timestamp());
INSERT INTO ple_private.instructor_profile (
    account_id, first_name, last_name, affiliation, created_at, updated_at
) VALUES (
    :'instructor_id', 'Saved', 'Instructor', 'Test University',
    pg_catalog.transaction_timestamp(), pg_catalog.transaction_timestamp()
);

SET LOCAL ROLE ple_data_owner;
INSERT INTO ple_data.content_discipline (content_discipline_id, name)
VALUES ('73000000-0000-0000-0000-00000000cc01', 'Saved response discipline');
SELECT 'SVR1-' || ple_private.crockford_checksum_character('SVR1XYZ') || 'XYZ' AS question_id \gset
INSERT INTO ple_data.published_question (published_question_id, created_at)
VALUES (:'question_id', pg_catalog.transaction_timestamp());
INSERT INTO ple_data.question_revision (
    published_question_id, revision_number, backend, published_at
) VALUES (:'question_id', 1, 'ple', pg_catalog.transaction_timestamp());
INSERT INTO ple_data.question_revision_license (
    published_question_id, revision_number, spdx_expression
) VALUES (:'question_id', 1, 'CC0-1.0');
SELECT encode(ple_private.ensure_assessment_policy_snapshot(
    'Saved response', '', NULL,
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
    '73000000-0000-0000-0000-000000000010',
    jsonb_build_object(
        'kind', 'questionSource',
        'publishedQuestionRevisionTuple', jsonb_build_object(
            'publishedQuestionId', :'question_id',
            'revisionNumber', 1
        ),
        'objectId', '73000000-0000-0000-0000-000000000010'::uuid
    ),
    'private-content', 'question-source', decode(repeat('30', 32), 'hex'), 1,
    'application/json', pg_catalog.transaction_timestamp()
);
INSERT INTO ple_private.question_revision_source_binding (
    published_question_id, revision_number, backend, question_format, native_question_type,
    source_object_record_id, source_object_checksum, created_at
) VALUES (
    :'question_id', 1, 'ple', 'pleQuestionJson', 'multipleChoice',
    '73000000-0000-0000-0000-000000000010', repeat('30', 32), pg_catalog.transaction_timestamp()
);

SET LOCAL ROLE ple_api_owner;
INSERT INTO ple_data.course_instance (
    course_instance_id, source_kind, course_short_name, course_long_name,
    content_discipline_id, tags, term_starts_on, term_ends_on, created_at
) VALUES (
    'CISV00000' || ple_private.crockford_checksum_character('CISV00000'),
    'empty', 'SVR-1', 'Saved response Course',
    '73000000-0000-0000-0000-00000000cc01',
    ARRAY[]::text[], current_date, current_date + 1,
    pg_catalog.transaction_timestamp()
) RETURNING course_instance_id AS course_id \gset
INSERT INTO ple_data.course_membership (
    course_membership_id, course_instance_id, account_id, role, joined_at
) VALUES (
    '73000000-0000-0000-0000-000000000301',
    :'course_id', :'instructor_id', 'instructor', pg_catalog.transaction_timestamp()
);
INSERT INTO ple_data.student_record (
    student_record_id, course_instance_id, student_account_id, created_at
) VALUES (
    '73000000-0000-0000-0000-000000000201',
    :'course_id', :'student_id', pg_catalog.transaction_timestamp()
);
INSERT INTO ple_data.course_membership (
    course_membership_id, course_instance_id, account_id, role, student_record_id, joined_at
) VALUES (
    '73000000-0000-0000-0000-000000000302',
    :'course_id', :'student_id', 'student',
    '73000000-0000-0000-0000-000000000201', pg_catalog.transaction_timestamp()
);

SET LOCAL ROLE ple_data_owner;
INSERT INTO ple_data.assessment (
    assessment_id, course_instance_id, origin_kind, created_at, updated_at,
    assessment_type, assessment_policy_snapshot_id
) VALUES (
    'ASVR0000' || ple_private.crockford_checksum_character('ASVR0000'),
    :'course_id', 'direct',
    pg_catalog.transaction_timestamp(), pg_catalog.transaction_timestamp(),
    'regular_assignment', decode(:'snapshot_id', 'hex')
) RETURNING assessment_id AS assessment_id \gset
INSERT INTO ple_data.assessment_entry (
    assessment_entry_id, assessment_id, authored_position, entry_kind,
    availability, scoring_rule
) VALUES (
    '73000000-0000-0000-0000-000000000081', :'assessment_id', 0, 'fixed_question', 'available', 'normal'
);
INSERT INTO ple_data.assessment_entry_question (
    assessment_entry_id, assessment_id, published_question_id,
    question_revision_number, points_possible
) VALUES (
    '73000000-0000-0000-0000-000000000081', :'assessment_id', :'question_id', 1, 1
);

SET LOCAL ROLE ple_app;
SELECT set_config('ple.session_account_id', :'instructor_id', true);
SELECT assessment_status AS released_status
  FROM ple_api.release_assessment(:'course_id', :'assessment_id', 1) \gset
SELECT set_config('ple.session_account_id', :'student_id', true);
SELECT assessment_attempt_id AS attempt_id
  FROM ple_api.start_assessment_attempt(
    '73000000-0000-0000-0000-000000000001',
    '73000000-0000-0000-0000-000000000201',
    :'assessment_id',
    '[]'::jsonb,
    jsonb_build_array(jsonb_build_object(
        'issued_question_id', '73000000-0000-0000-0000-000000000011',
        'assessment_entry_id', '73000000-0000-0000-0000-000000000081',
        'issued_position', 0,
        'published_question_id', :'question_id',
        'revision_number', 1
    ))
) \gset

RESET ROLE;
SET LOCAL ROLE ple_private_owner;
INSERT INTO ple_private.question_attempt (
    course_instance_id, question_attempt_id, issued_question_id, issued_at,
    delivery_toolchain_id, rendered_question_sha256
)
SELECT issued.course_instance_id,
       '73000000-0000-0000-0000-000000000021'::uuid,
       issued.issued_question_id,
       pg_catalog.transaction_timestamp(),
       ple_private.ensure_delivery_toolchain('ple', '1', NULL, NULL, 'ple', '1', 'not_applicable'),
       decode(repeat('ab', 32), 'hex')
  FROM ple_private.issued_question AS issued
 WHERE issued.issued_question_id = '73000000-0000-0000-0000-000000000011';

SET LOCAL ROLE ple_app;
SELECT set_config('ple.session_account_id', :'student_id', true);
SELECT '{"choice":"bilayer"}'::jsonb::text AS first_response \gset
SELECT '{"choice":"cholesterol"}'::jsonb::text AS replacement_response \gset
SELECT response_state AS first_state
  FROM ple_api.save_student_assessment_attempt_response(
    :'attempt_id'::uuid, 1, :'first_response'::jsonb
) \gset
SELECT student_response::text AS read_first
  FROM ple_api.read_student_assessment_attempt_saved_response(:'attempt_id'::uuid, 1) \gset
SELECT response_state AS replacement_state
  FROM ple_api.save_student_assessment_attempt_response(
    :'attempt_id'::uuid, 1, :'replacement_response'::jsonb
) \gset
SELECT student_response::text AS read_replacement
  FROM ple_api.read_student_assessment_attempt_saved_response(:'attempt_id'::uuid, 1) \gset
SELECT (points_earned = 1::double precision AND points_possible = 1::double precision)::text AS graded_one
  FROM ple_api.commit_student_assessment_attempt_finalization(
    :'attempt_id'::uuid,
    'student',
    (
      SELECT jsonb_agg(jsonb_build_object(
          'question_attempt_id', prepared.question_attempt_id,
          'saved_at_millis', prepared.saved_at_millis,
          'student_response', prepared.student_response,
          'normalized_credit', 1
      ))
        FROM ple_api.prepare_student_assessment_attempt_finalization(:'attempt_id'::uuid) AS prepared
       WHERE prepared.preparation_state = 'ready'
         AND prepared.question_attempt_id IS NOT NULL
    )
) \gset
SELECT response_state AS frozen_state
  FROM ple_api.save_student_assessment_attempt_response(
    :'attempt_id'::uuid, 1, '{"choice":"ignored"}'::jsonb
) \gset
SELECT student_response::text AS read_after_submit
  FROM ple_api.read_student_assessment_attempt_saved_response(:'attempt_id'::uuid, 1) \gset

RESET ROLE;
SELECT count(*)::text AS saved_rows
  FROM ple_private.assessment_attempt_saved_response AS saved
  JOIN ple_private.question_attempt AS question_attempt
    ON question_attempt.question_attempt_id = saved.question_attempt_id
  JOIN ple_private.issued_question AS issued
    ON issued.issued_question_id = question_attempt.issued_question_id
 WHERE issued.assessment_attempt_id = :'attempt_id'::uuid \gset
SELECT count(*)::text AS submission_rows,
       bool_and(submission.authorized_by_account_id = :'student_id')::text AS student_submitted
  FROM ple_private.assessment_submission AS submission
 WHERE submission.assessment_attempt_id = :'attempt_id'::uuid \gset
SELECT count(*)::text AS grading_rows
  FROM ple_private.grading_result AS result
  JOIN ple_private.question_attempt AS question_attempt
    ON question_attempt.question_attempt_id = result.question_attempt_id
  JOIN ple_private.issued_question AS issued
    ON issued.issued_question_id = question_attempt.issued_question_id
 WHERE issued.assessment_attempt_id = :'attempt_id'::uuid \gset
SELECT set_config('ple.saved_proof_released', :'released_status', true);
SELECT set_config('ple.saved_proof_first_state', :'first_state', true);
SELECT set_config('ple.saved_proof_read_first', :'read_first', true);
SELECT set_config('ple.saved_proof_first_response', :'first_response', true);
SELECT set_config('ple.saved_proof_replacement_state', :'replacement_state', true);
SELECT set_config('ple.saved_proof_read_replacement', :'read_replacement', true);
SELECT set_config('ple.saved_proof_replacement_response', :'replacement_response', true);
SELECT set_config('ple.saved_proof_graded_one', :'graded_one', true);
SELECT set_config('ple.saved_proof_frozen_state', :'frozen_state', true);
SELECT set_config('ple.saved_proof_read_after', :'read_after_submit', true);
SELECT set_config('ple.saved_proof_saved_rows', :'saved_rows', true);
SELECT set_config('ple.saved_proof_submission_rows', :'submission_rows', true);
SELECT set_config('ple.saved_proof_student_submitted', :'student_submitted', true);
SELECT set_config('ple.saved_proof_grading_rows', :'grading_rows', true);
SELECT set_config('ple.saved_proof_session', :'student_id', true);
SELECT set_config('ple.saved_proof_instructor', :'instructor_id', true);

DO $$
BEGIN
    IF current_setting('ple.saved_proof_released') <> 'released'
       OR current_setting('ple.saved_proof_first_state') <> 'saved'
       OR current_setting('ple.saved_proof_read_first')
          <> current_setting('ple.saved_proof_first_response')
       OR current_setting('ple.saved_proof_replacement_state') <> 'saved'
       OR current_setting('ple.saved_proof_read_replacement')
          <> current_setting('ple.saved_proof_replacement_response')
       OR current_setting('ple.saved_proof_graded_one') <> 'true'
       OR current_setting('ple.saved_proof_frozen_state') <> 'expired'
       OR current_setting('ple.saved_proof_read_after')
          <> current_setting('ple.saved_proof_replacement_response')
       OR current_setting('ple.saved_proof_saved_rows') <> '1'
       OR current_setting('ple.saved_proof_submission_rows') <> '1'
       OR current_setting('ple.saved_proof_student_submitted') <> 'true'
       OR current_setting('ple.saved_proof_grading_rows') <> '1'
       OR current_setting('ple.saved_proof_session') = current_setting('ple.saved_proof_instructor') THEN
        RAISE EXCEPTION 'saved response proof failed released=% first=% read_first=% replacement=% read_replacement=% graded=% frozen=% read_after=% saved_rows=% submissions=% student_submitted=% grades=%',
            current_setting('ple.saved_proof_released'),
            current_setting('ple.saved_proof_first_state'),
            current_setting('ple.saved_proof_read_first'),
            current_setting('ple.saved_proof_replacement_state'),
            current_setting('ple.saved_proof_read_replacement'),
            current_setting('ple.saved_proof_graded_one'),
            current_setting('ple.saved_proof_frozen_state'),
            current_setting('ple.saved_proof_read_after'),
            current_setting('ple.saved_proof_saved_rows'),
            current_setting('ple.saved_proof_submission_rows'),
            current_setting('ple.saved_proof_student_submitted'),
            current_setting('ple.saved_proof_grading_rows');
    END IF;
    RAISE NOTICE 'save_student_assessment_attempt_response replaced_open_response commit_student_assessment_attempt_finalization graded_without_instructor';
END $$;

-- A second Attempt issues two Questions and saves only the first. Preparation
-- for the Question Backend returns that saved Question Attempt and omits the
-- unanswered one. This block does not grade either Question.
SET LOCAL ROLE ple_data_owner;
INSERT INTO ple_data.assessment (
    assessment_id, course_instance_id, origin_kind, created_at, updated_at,
    assessment_type, assessment_policy_snapshot_id
) VALUES (
    'ASVR0001' || ple_private.crockford_checksum_character('ASVR0001'),
    :'course_id', 'direct',
    pg_catalog.transaction_timestamp(), pg_catalog.transaction_timestamp(),
    'regular_assignment', decode(:'snapshot_id', 'hex')
) RETURNING assessment_id AS unanswered_assessment_id \gset
INSERT INTO ple_data.assessment_entry (
    assessment_entry_id, assessment_id, authored_position, entry_kind,
    availability, scoring_rule
) VALUES
    ('73000000-0000-0000-0000-000000000082', :'unanswered_assessment_id', 0, 'fixed_question', 'available', 'normal'),
    ('73000000-0000-0000-0000-000000000083', :'unanswered_assessment_id', 1, 'fixed_question', 'available', 'normal');
INSERT INTO ple_data.assessment_entry_question (
    assessment_entry_id, assessment_id, published_question_id,
    question_revision_number, points_possible
) VALUES
    ('73000000-0000-0000-0000-000000000082', :'unanswered_assessment_id', :'question_id', 1, 1),
    ('73000000-0000-0000-0000-000000000083', :'unanswered_assessment_id', :'question_id', 1, 1);

SET LOCAL ROLE ple_app;
SELECT set_config('ple.session_account_id', :'instructor_id', true);
SELECT assessment_status AS second_released
  FROM ple_api.release_assessment(:'course_id', :'unanswered_assessment_id', 1) \gset
SELECT set_config('ple.session_account_id', :'student_id', true);
SELECT assessment_attempt_id AS unanswered_attempt_id
  FROM ple_api.start_assessment_attempt(
    '73000000-0000-0000-0000-000000000002',
    '73000000-0000-0000-0000-000000000201',
    :'unanswered_assessment_id',
    '[]'::jsonb,
    jsonb_build_array(
        jsonb_build_object(
            'issued_question_id', '73000000-0000-0000-0000-000000000012',
            'assessment_entry_id', '73000000-0000-0000-0000-000000000082',
            'issued_position', 0,
            'published_question_id', :'question_id',
            'revision_number', 1
        ),
        jsonb_build_object(
            'issued_question_id', '73000000-0000-0000-0000-000000000013',
            'assessment_entry_id', '73000000-0000-0000-0000-000000000083',
            'issued_position', 1,
            'published_question_id', :'question_id',
            'revision_number', 1
        )
    )
) \gset

RESET ROLE;
SET LOCAL ROLE ple_private_owner;
INSERT INTO ple_private.question_attempt (
    course_instance_id, question_attempt_id, issued_question_id, issued_at,
    delivery_toolchain_id, rendered_question_sha256
)
SELECT issued.course_instance_id,
       '73000000-0000-0000-0000-000000000022'::uuid,
       issued.issued_question_id,
       pg_catalog.transaction_timestamp(),
       ple_private.ensure_delivery_toolchain('ple', '1', NULL, NULL, 'ple', '1', 'not_applicable'),
       decode(repeat('ab', 32), 'hex')
  FROM ple_private.issued_question AS issued
 WHERE issued.issued_question_id = '73000000-0000-0000-0000-000000000012';
INSERT INTO ple_private.question_attempt (
    course_instance_id, question_attempt_id, issued_question_id, issued_at,
    delivery_toolchain_id, rendered_question_sha256
)
SELECT issued.course_instance_id,
       '73000000-0000-0000-0000-000000000023'::uuid,
       issued.issued_question_id,
       pg_catalog.transaction_timestamp(),
       ple_private.ensure_delivery_toolchain('ple', '1', NULL, NULL, 'ple', '1', 'not_applicable'),
       decode(repeat('cd', 32), 'hex')
  FROM ple_private.issued_question AS issued
 WHERE issued.issued_question_id = '73000000-0000-0000-0000-000000000013';

SET LOCAL ROLE ple_app;
SELECT set_config('ple.session_account_id', :'student_id', true);
SELECT response_state AS unanswered_pair_saved
  FROM ple_api.save_student_assessment_attempt_response(
    :'unanswered_attempt_id'::uuid, 1, '{"choice":"bilayer"}'::jsonb
) \gset
SELECT count(*)::text AS ready_rows,
       count(*) FILTER (WHERE prepared.question_attempt_id IS NOT NULL)::text AS backend_work_rows,
       count(*) FILTER (
           WHERE prepared.question_attempt_id = '73000000-0000-0000-0000-000000000022'::uuid
       )::text AS saved_sent_rows,
       count(*) FILTER (
           WHERE prepared.question_attempt_id = '73000000-0000-0000-0000-000000000023'::uuid
       )::text AS unanswered_sent_rows
  FROM ple_api.prepare_student_assessment_attempt_finalization(:'unanswered_attempt_id'::uuid) AS prepared
 WHERE prepared.preparation_state = 'ready' \gset
RESET ROLE;
SELECT count(*)::text AS issued_question_rows
  FROM ple_private.issued_question AS issued
 WHERE issued.assessment_attempt_id = :'unanswered_attempt_id'::uuid \gset

SELECT set_config('ple.unanswered_pair_saved', :'unanswered_pair_saved', true);
SELECT set_config('ple.unanswered_ready_rows', :'ready_rows', true);
SELECT set_config('ple.unanswered_backend_work_rows', :'backend_work_rows', true);
SELECT set_config('ple.unanswered_saved_sent_rows', :'saved_sent_rows', true);
SELECT set_config('ple.unanswered_sent_rows', :'unanswered_sent_rows', true);
SELECT set_config('ple.unanswered_issued_rows', :'issued_question_rows', true);
DO $$
BEGIN
    IF current_setting('ple.unanswered_pair_saved') <> 'saved'
       OR current_setting('ple.unanswered_issued_rows') <> '2'
       OR current_setting('ple.unanswered_ready_rows') <> '1'
       OR current_setting('ple.unanswered_backend_work_rows') <> '1'
       OR current_setting('ple.unanswered_saved_sent_rows') <> '1'
       OR current_setting('ple.unanswered_sent_rows') <> '0' THEN
        RAISE EXCEPTION 'unanswered backend work proof failed saved=% issued=% ready=% backend=% saved_sent=% unanswered_sent=%',
            current_setting('ple.unanswered_pair_saved'),
            current_setting('ple.unanswered_issued_rows'),
            current_setting('ple.unanswered_ready_rows'),
            current_setting('ple.unanswered_backend_work_rows'),
            current_setting('ple.unanswered_saved_sent_rows'),
            current_setting('ple.unanswered_sent_rows');
    END IF;
    RAISE NOTICE 'unanswered_question_omitted_from_backend_work';
END $$;
COMMIT;
