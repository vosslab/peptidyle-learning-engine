-- Disposable proof that commit_expired_student_assessment_attempt_finalization
-- submits an Attempt after its time limit elapses and finalizes the saved
-- response. The credit passed to commit is the already-returned evaluation.
-- This file does not call a Question Backend.
\set ON_ERROR_STOP on

BEGIN;
SET CONSTRAINTS ALL DEFERRED;

SET LOCAL ROLE ple_private_owner;
SELECT 'UEXP0000' || ple_private.crockford_checksum_character('UEXP0000') AS instructor_id \gset
SELECT 'USEX0000' || ple_private.crockford_checksum_character('USEX0000') AS student_id \gset
INSERT INTO ple_private.account (account_id, user_role, created_at)
VALUES
    (:'instructor_id', 'instructor', pg_catalog.transaction_timestamp()),
    (:'student_id', 'student', pg_catalog.transaction_timestamp());

SET LOCAL ROLE ple_data_owner;
INSERT INTO ple_data.content_discipline (content_discipline_id, name)
VALUES ('74000000-0000-0000-0000-00000000cc01', 'Expiry discipline');
SELECT 'EXR1-' || ple_private.crockford_checksum_character('EXR1XYZ') || 'XYZ' AS question_id \gset
INSERT INTO ple_data.published_question (published_question_id, created_at)
VALUES (:'question_id', pg_catalog.transaction_timestamp());
INSERT INTO ple_data.question_revision (
    published_question_id, revision_number, backend, question_type, published_at
) VALUES (:'question_id', 1, 'ple', 'multipleChoice', pg_catalog.transaction_timestamp());
SELECT encode(ple_private.ensure_assessment_policy_snapshot(
    'Expiry submit', '', NULL,
    pg_catalog.transaction_timestamp() + interval '2 days',
    NULL, 5, NULL,
    'accept', 'reuse_variation', 'authored_order',
    'after_submit', 'after_submit', 'after_submit', 'never', 'never', 'never',
    'regular_assignment'
), 'hex') AS snapshot_id \gset

SET LOCAL ROLE ple_private_owner;
INSERT INTO ple_private.object_record (
    object_record_id, object_address, object_storage_area, object_data_class, sha256,
    size_bytes, media_type, created_at
) VALUES (
    '74000000-0000-0000-0000-000000000010',
    jsonb_build_object(
        'kind', 'questionSource',
        'publishedQuestionRevisionTuple', jsonb_build_object(
            'publishedQuestionId', :'question_id',
            'revisionNumber', 1
        ),
        'objectId', '74000000-0000-0000-0000-000000000010'::uuid
    ),
    'private-content', 'question-source', decode(repeat('30', 32), 'hex'), 1,
    'application/json', pg_catalog.transaction_timestamp()
);
INSERT INTO ple_private.question_revision_source_binding (
    published_question_id, revision_number, backend, question_format,
    source_object_record_id, source_object_checksum, created_at
) VALUES (
    :'question_id', 1, 'ple', 'pleQuestionJson',
    '74000000-0000-0000-0000-000000000010', repeat('30', 32), pg_catalog.transaction_timestamp()
);

SET LOCAL ROLE ple_api_owner;
INSERT INTO ple_data.course_instance (
    course_instance_id, source_kind, course_short_name, course_long_name,
    content_discipline_id, tags, term_starts_on, term_ends_on, created_at
) VALUES (
    'CIEX00000' || ple_private.crockford_checksum_character('CIEX00000'),
    'empty', 'EXP-1', 'Expiry Course',
    '74000000-0000-0000-0000-00000000cc01',
    ARRAY[]::text[], current_date, current_date + 1,
    pg_catalog.transaction_timestamp()
) RETURNING course_instance_id AS course_id \gset
INSERT INTO ple_data.course_membership (
    course_membership_id, course_instance_id, account_id, role, joined_at
) VALUES (
    '74000000-0000-0000-0000-000000000301',
    :'course_id', :'instructor_id', 'instructor', pg_catalog.transaction_timestamp()
);
INSERT INTO ple_data.student_record (
    student_record_id, course_instance_id, student_account_id, created_at
) VALUES (
    '74000000-0000-0000-0000-000000000201',
    :'course_id', :'student_id', pg_catalog.transaction_timestamp()
);
INSERT INTO ple_data.course_membership (
    course_membership_id, course_instance_id, account_id, role, student_record_id, joined_at
) VALUES (
    '74000000-0000-0000-0000-000000000302',
    :'course_id', :'student_id', 'student',
    '74000000-0000-0000-0000-000000000201', pg_catalog.transaction_timestamp()
);

SET LOCAL ROLE ple_data_owner;
INSERT INTO ple_data.assessment (
    assessment_id, course_instance_id, origin_kind, created_at, updated_at,
    assessment_type, assessment_policy_snapshot_id
) VALUES (
    'AEXP0000' || ple_private.crockford_checksum_character('AEXP0000'),
    :'course_id', 'direct',
    pg_catalog.transaction_timestamp(), pg_catalog.transaction_timestamp(),
    'regular_assignment', decode(:'snapshot_id', 'hex')
) RETURNING assessment_id AS assessment_id \gset
INSERT INTO ple_data.assessment_entry (
    assessment_entry_id, assessment_id, authored_position, entry_kind,
    availability, scoring_rule
) VALUES (
    '74000000-0000-0000-0000-000000000081', :'assessment_id', 0, 'fixed_question', 'available', 'normal'
);
INSERT INTO ple_data.assessment_entry_question (
    assessment_entry_id, assessment_id, published_question_id,
    question_revision_number, points_possible
) VALUES (
    '74000000-0000-0000-0000-000000000081', :'assessment_id', :'question_id', 1, 1
);

SET LOCAL ROLE ple_app;
SELECT set_config('ple.session_account_id', :'instructor_id', true);
SELECT assessment_status AS released_status
  FROM ple_api.release_assessment(:'course_id', :'assessment_id', 1) \gset
SELECT set_config('ple.session_account_id', :'student_id', true);
SELECT assessment_attempt_id AS attempt_id
  FROM ple_api.start_assessment_attempt(
    '74000000-0000-0000-0000-000000000001',
    '74000000-0000-0000-0000-000000000201',
    :'assessment_id',
    '[]'::jsonb,
    jsonb_build_array(jsonb_build_object(
        'issued_question_id', '74000000-0000-0000-0000-000000000011',
        'assessment_entry_id', '74000000-0000-0000-0000-000000000081',
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
       '74000000-0000-0000-0000-000000000021'::uuid,
       issued.issued_question_id,
       pg_catalog.transaction_timestamp(),
       ple_private.ensure_delivery_toolchain('ple', '1', NULL, NULL, 'ple', '1', 'not_applicable'),
       decode(repeat('ab', 32), 'hex')
  FROM ple_private.issued_question AS issued
 WHERE issued.issued_question_id = '74000000-0000-0000-0000-000000000011';

SET LOCAL ROLE ple_app;
SELECT set_config('ple.session_account_id', :'student_id', true);
SELECT '{"choice":"cholesterol"}'::jsonb::text AS saved_response \gset
SELECT response_state AS saved_state
  FROM ple_api.save_student_assessment_attempt_response(
    :'attempt_id'::uuid, 1, :'saved_response'::jsonb
) \gset
SELECT pg_catalog.pg_sleep(6);

RESET ROLE;
SET LOCAL ROLE ple_assessment_attempt_expiry_worker;
SELECT count(*)::text AS prepared_rows
  FROM ple_api.prepare_expired_student_assessment_attempt_finalizations(10) AS prepared
 WHERE prepared.assessment_attempt_id = :'attempt_id'::uuid
   AND prepared.student_response = :'saved_response'::jsonb \gset
SELECT (points_earned = 1::double precision AND points_possible = 1::double precision)::text AS graded_one
  FROM ple_api.commit_expired_student_assessment_attempt_finalization(
    :'attempt_id'::uuid,
    (
      SELECT jsonb_agg(jsonb_build_object(
          'question_attempt_id', prepared.question_attempt_id,
          'saved_at_millis', prepared.saved_at_millis,
          'student_response', prepared.student_response,
          'normalized_credit', 1
      ))
        FROM ple_api.prepare_expired_student_assessment_attempt_finalizations(10) AS prepared
       WHERE prepared.assessment_attempt_id = :'attempt_id'::uuid
         AND prepared.question_attempt_id IS NOT NULL
    )
) \gset

RESET ROLE;
SELECT count(*)::text AS saved_rows,
       bool_and(saved.finalized_at IS NOT NULL)::text AS response_finalized,
       bool_and(saved.student_response = :'saved_response'::jsonb)::text AS response_kept
  FROM ple_private.assessment_attempt_saved_response AS saved
  JOIN ple_private.question_attempt AS question_attempt
    ON question_attempt.question_attempt_id = saved.question_attempt_id
  JOIN ple_private.issued_question AS issued
    ON issued.issued_question_id = question_attempt.issued_question_id
 WHERE issued.assessment_attempt_id = :'attempt_id'::uuid \gset
SELECT count(*)::text AS submission_rows,
       bool_and(submission.authorized_by_account_id IS NULL)::text AS deadline_submitted
  FROM ple_private.assessment_submission AS submission
 WHERE submission.assessment_attempt_id = :'attempt_id'::uuid \gset
SELECT count(*)::text AS grading_rows
  FROM ple_private.grading_result AS result
  JOIN ple_private.question_attempt AS question_attempt
    ON question_attempt.question_attempt_id = result.question_attempt_id
  JOIN ple_private.issued_question AS issued
    ON issued.issued_question_id = question_attempt.issued_question_id
 WHERE issued.assessment_attempt_id = :'attempt_id'::uuid \gset
SELECT set_config('ple.expiry_proof_released', :'released_status', true);
SELECT set_config('ple.expiry_proof_saved_state', :'saved_state', true);
SELECT set_config('ple.expiry_proof_prepared_rows', :'prepared_rows', true);
SELECT set_config('ple.expiry_proof_graded_one', :'graded_one', true);
SELECT set_config('ple.expiry_proof_saved_rows', :'saved_rows', true);
SELECT set_config('ple.expiry_proof_response_finalized', :'response_finalized', true);
SELECT set_config('ple.expiry_proof_response_kept', :'response_kept', true);
SELECT set_config('ple.expiry_proof_submission_rows', :'submission_rows', true);
SELECT set_config('ple.expiry_proof_deadline_submitted', :'deadline_submitted', true);
SELECT set_config('ple.expiry_proof_grading_rows', :'grading_rows', true);

DO $$
BEGIN
    IF current_setting('ple.expiry_proof_released') <> 'released'
       OR current_setting('ple.expiry_proof_saved_state') <> 'saved'
       OR current_setting('ple.expiry_proof_prepared_rows') <> '1'
       OR current_setting('ple.expiry_proof_graded_one') <> 'true'
       OR current_setting('ple.expiry_proof_saved_rows') <> '1'
       OR current_setting('ple.expiry_proof_response_finalized') <> 'true'
       OR current_setting('ple.expiry_proof_response_kept') <> 'true'
       OR current_setting('ple.expiry_proof_submission_rows') <> '1'
       OR current_setting('ple.expiry_proof_deadline_submitted') <> 'true'
       OR current_setting('ple.expiry_proof_grading_rows') <> '1' THEN
        RAISE EXCEPTION 'expiry proof failed released=% saved=% prepared=% graded=% saved_rows=% finalized=% kept=% submissions=% deadline=% grades=%',
            current_setting('ple.expiry_proof_released'),
            current_setting('ple.expiry_proof_saved_state'),
            current_setting('ple.expiry_proof_prepared_rows'),
            current_setting('ple.expiry_proof_graded_one'),
            current_setting('ple.expiry_proof_saved_rows'),
            current_setting('ple.expiry_proof_response_finalized'),
            current_setting('ple.expiry_proof_response_kept'),
            current_setting('ple.expiry_proof_submission_rows'),
            current_setting('ple.expiry_proof_deadline_submitted'),
            current_setting('ple.expiry_proof_grading_rows');
    END IF;
    RAISE NOTICE 'commit_expired_student_assessment_attempt_finalization finalized_saved_response';
END $$;
COMMIT;
