-- Connected PostgreSQL oracle for ordinary Assessment Attempt finalization.
--
-- This uses the ordinary Student API over the committed fixture from the
-- existing assessment_access_postgres connected test, run before this oracle
-- by the baseline owner. The security catalog does not create that fixture.
-- It protects immutable backend credit, current-point scoring,
-- deadline closure, and submission-attempt lifecycle; it deliberately does
-- not preserve internal Jobs or public grading-state projections.
\set ON_ERROR_STOP on

BEGIN;
SET CONSTRAINTS ALL DEFERRED;

-- The Assessment Access fixture supplies Student Records eb02/eb06, Entry
-- ed02, and native Question Revision 1. Course Instance and Assessment IDs
-- are minted public IDs; resolve them from that live catalog.
SET LOCAL ROLE ple_data_owner;
DO $$
DECLARE
    course_id_value text;
    assessment_id_value text;
    entry_id_value uuid;
    question_id_value text;
    expiry_student_id text;
    other_student_id text;
    instructor_id_value text;
BEGIN
    SELECT record.course_instance_id, record.student_account_id
      INTO STRICT course_id_value, expiry_student_id
      FROM ple_data.student_record AS record
     WHERE record.student_record_id = '00000000-0000-0000-0000-00000000eb02';
    SELECT record.student_account_id
      INTO STRICT other_student_id
      FROM ple_data.student_record AS record
     WHERE record.student_record_id = '00000000-0000-0000-0000-00000000eb06'
       AND record.course_instance_id = course_id_value;
    SELECT membership.account_id
      INTO STRICT instructor_id_value
      FROM ple_data.course_membership AS membership
     WHERE membership.course_instance_id = course_id_value
       AND membership.role = 'instructor'
     ORDER BY membership.joined_at, membership.course_membership_id
     LIMIT 1;
    SELECT question.assessment_entry_id, question.published_question_id, entry.assessment_id
      INTO entry_id_value, question_id_value, assessment_id_value
      FROM ple_data.assessment_entry_question AS question
      JOIN ple_data.assessment_entry AS entry
        ON entry.assessment_entry_id = question.assessment_entry_id
      JOIN ple_data.assessment AS assessment
        ON assessment.assessment_id = entry.assessment_id
     WHERE question.assessment_entry_id = '00000000-0000-0000-0000-00000000ed02'
       AND assessment.course_instance_id = course_id_value;
    IF assessment_id_value IS NULL THEN
        SELECT assessment.assessment_id, question.assessment_entry_id, question.published_question_id
          INTO STRICT assessment_id_value, entry_id_value, question_id_value
          FROM ple_data.assessment AS assessment
          JOIN ple_data.assessment_policy_snapshot AS policy
            ON policy.assessment_policy_snapshot_id = assessment.assessment_policy_snapshot_id
          JOIN ple_data.assessment_entry AS entry
            ON entry.assessment_id = assessment.assessment_id
          JOIN ple_data.assessment_entry_question AS question
            ON question.assessment_entry_id = entry.assessment_entry_id
         WHERE assessment.course_instance_id = course_id_value
           AND policy.assessment_title = 'Server-owned Assessment Access';
    END IF;
    PERFORM set_config('ple.test_direct_course_id', course_id_value, true);
    PERFORM set_config('ple.test_direct_assessment_id', assessment_id_value, true);
    PERFORM set_config('ple.test_direct_entry_id', entry_id_value::text, true);
    PERFORM set_config('ple.test_direct_question_id', question_id_value, true);
    PERFORM set_config('ple.test_direct_expiry_student_id', expiry_student_id, true);
    PERFORM set_config('ple.test_direct_other_student_id', other_student_id, true);
    PERFORM set_config('ple.test_direct_instructor_id', instructor_id_value, true);
END
$$;

-- This source binding enables direct evaluation of the fixture Question.
SET LOCAL ROLE ple_private_owner;
INSERT INTO ple_private.object_record (
    object_record_id, object_address, object_storage_area, object_data_class, sha256,
    size_bytes, media_type, created_at
) VALUES (
    'e3000000-0000-0000-0000-000000000001',
    jsonb_build_object(
        'kind', 'questionSource',
        'questionRevision', jsonb_build_object(
            'questionId', current_setting('ple.test_direct_question_id'),
            'revisionNumber', 1
        ),
        'object', 'e3000000-0000-0000-0000-000000000001'::uuid
    ),
    'private-content', 'question-source', decode(repeat('e3', 32), 'hex'), 1,
    'application/json', pg_catalog.transaction_timestamp()
);
INSERT INTO ple_private.question_revision_source_binding (
    published_question_id, revision_number, backend, question_format,
    source_object_record_id, source_object_checksum, created_at
) VALUES (
    current_setting('ple.test_direct_question_id'), 1, 'ple', 'pleQuestionJson',
    'e3000000-0000-0000-0000-000000000001', repeat('e3', 32), pg_catalog.transaction_timestamp()
);

-- Save A, prepare it, then save B at the same millisecond. Response bytes are
-- part of the immutable snapshot fence, so stale backend work leaves no
-- partial submission, result, receipt, or completion behind.
SET LOCAL ROLE ple_api_owner;
SELECT set_config('ple.session_account_id', current_setting('ple.test_direct_other_student_id'), true);
SELECT * FROM ple_api.start_assessment_attempt(
    'e3000000-0000-0000-0000-000000000010',
    '00000000-0000-0000-0000-00000000eb06',
    current_setting('ple.test_direct_assessment_id'),
    '[]'::jsonb,
    jsonb_build_array(jsonb_build_object(
        'issued_question_id', 'e3000000-0000-0000-0000-000000000011',
        'assessment_entry_id', current_setting('ple.test_direct_entry_id')::uuid,
        'issued_position', 0,
        'published_question_id', current_setting('ple.test_direct_question_id'),
        'revision_number', 1
    ))
);
SET LOCAL ROLE ple_private_owner;
INSERT INTO ple_private.question_attempt (
    course_instance_id, question_attempt_id, issued_question_id, issued_at,
    delivery_toolchain_id, rendered_question_sha256
)
SELECT issued.course_instance_id,
       'e3000000-0000-0000-0000-000000000031',
       'e3000000-0000-0000-0000-000000000011',
       clock_timestamp(),
       ple_private.ensure_delivery_toolchain('ple', '1', NULL, NULL, 'ple', '1', 'not_applicable'),
       decode(repeat('31', 32), 'hex')
  FROM ple_private.issued_question AS issued
 WHERE issued.issued_question_id = 'e3000000-0000-0000-0000-000000000011';
SET LOCAL ROLE ple_api_owner;
SELECT * FROM ple_api.save_student_assessment_attempt_response(
    (SELECT assessment_attempt_id
       FROM ple_api.read_started_student_assessment_attempt('e3000000-0000-0000-0000-000000000010')),
    1, '{"kind":"shortText","text":"response A"}'::jsonb
);
SELECT set_config('ple.test_direct_attempt_id', assessment_attempt_id::text, true)
  FROM ple_api.read_started_student_assessment_attempt('e3000000-0000-0000-0000-000000000010');

DO $$
DECLARE saved_millis bigint;
DECLARE stale_snapshot_rejected boolean := false;
BEGIN
    SELECT saved_at_millis INTO saved_millis
      FROM ple_api.prepare_student_assessment_attempt_finalization(
          current_setting('ple.test_direct_attempt_id')::uuid
      )
     WHERE preparation_state = 'ready';
    IF saved_millis IS NULL THEN
        RAISE EXCEPTION 'ordinary Student finalization did not prepare saved response A';
    END IF;
    SET LOCAL ROLE ple_private_owner;
    UPDATE ple_private.assessment_attempt_saved_response
       SET student_response = '{"kind":"shortText","text":"response B"}'::jsonb,
           saved_at = to_timestamp(saved_millis::double precision / 1000.0)
     WHERE question_attempt_id = 'e3000000-0000-0000-0000-000000000031';
    SET LOCAL ROLE ple_api_owner;
    BEGIN
        PERFORM * FROM ple_api.commit_student_assessment_attempt_finalization(
            current_setting('ple.test_direct_attempt_id')::uuid,
            'student',
            jsonb_build_array(jsonb_build_object(
                'question_attempt_id', 'e3000000-0000-0000-0000-000000000031',
                'saved_at_millis', saved_millis,
                'student_response', '{"kind":"shortText","text":"response A"}'::jsonb,
                'normalized_credit', 0.67
            ))
        );
    EXCEPTION WHEN insufficient_privilege THEN
        stale_snapshot_rejected := true;
    WHEN OTHERS THEN
        RAISE EXCEPTION 'stale snapshot unexpected SQLSTATE %, message %', SQLSTATE, SQLERRM;
    END;
    IF NOT stale_snapshot_rejected THEN
        RAISE EXCEPTION 'same-millisecond stale finalization unexpectedly committed';
    END IF;
END
$$;
SET LOCAL ROLE ple_private_owner;
DO $$
BEGIN
    IF EXISTS (
            SELECT 1 FROM ple_private.assessment_submission
             WHERE assessment_attempt_id = 'e3000000-0000-0000-0000-000000000010'
        )
       OR EXISTS (
            SELECT 1 FROM ple_private.assessment_attempt_saved_response
             WHERE question_attempt_id = 'e3000000-0000-0000-0000-000000000031'
               AND finalized_at IS NOT NULL
        )
       OR EXISTS (
            SELECT 1 FROM ple_private.grading_result
             WHERE question_attempt_id = 'e3000000-0000-0000-0000-000000000031'
        )
       OR EXISTS (
            SELECT 1
              FROM ple_audit.automated_grading_receipt AS receipt
              JOIN ple_private.grading_result AS result
                ON result.grading_result_id = receipt.grading_result_id
             WHERE result.question_attempt_id = 'e3000000-0000-0000-0000-000000000031'
        ) THEN
        RAISE EXCEPTION 'stale finalization left partial immutable evidence';
    END IF;
END
$$;

-- Commit B and replay the lost response. Changing the Entry from two to three
-- points recalculates 0.67 credit as 2.01/3 without another backend action.
SET LOCAL ROLE ple_api_owner;
DO $$
DECLARE prepared record;
DECLARE first_score record;
DECLARE replay_score record;
DECLARE receipt_id uuid;
BEGIN
    SELECT * INTO prepared
      FROM ple_api.prepare_student_assessment_attempt_finalization(
          current_setting('ple.test_direct_attempt_id')::uuid
      )
     WHERE preparation_state = 'ready';
    SELECT * INTO first_score
      FROM ple_api.commit_student_assessment_attempt_finalization(
          current_setting('ple.test_direct_attempt_id')::uuid,
          'student',
          jsonb_build_array(jsonb_build_object(
              'question_attempt_id', prepared.question_attempt_id,
              'saved_at_millis', prepared.saved_at_millis,
              'student_response', prepared.student_response,
              'normalized_credit', 0.67
          ))
      );
    IF first_score.points_earned <> 1.34 OR first_score.points_possible <> 2 THEN
        RAISE EXCEPTION 'direct credit did not score at the issued two-point Entry';
    END IF;
    SET LOCAL ROLE ple_private_owner;
    SELECT receipt.automated_grading_receipt_id INTO receipt_id
      FROM ple_audit.automated_grading_receipt AS receipt
      JOIN ple_private.grading_result AS result
        ON result.grading_result_id = receipt.grading_result_id
     WHERE result.question_attempt_id = prepared.question_attempt_id;
    SET LOCAL ROLE ple_data_owner;
    UPDATE ple_data.assessment_entry_question
       SET points_possible = 3
     WHERE assessment_entry_id = current_setting('ple.test_direct_entry_id')::uuid;
    SET LOCAL ROLE ple_api_owner;
    SELECT * INTO replay_score
      FROM ple_api.commit_student_assessment_attempt_finalization(
          current_setting('ple.test_direct_attempt_id')::uuid,
          'student',
          '[]'::jsonb
      );
    SET LOCAL ROLE ple_private_owner;
    IF replay_score.points_earned <> 2.01 OR replay_score.points_possible <> 3
       OR receipt_id IS DISTINCT FROM (
            SELECT receipt.automated_grading_receipt_id
              FROM ple_audit.automated_grading_receipt AS receipt
              JOIN ple_private.grading_result AS result
                ON result.grading_result_id = receipt.grading_result_id
             WHERE result.question_attempt_id = prepared.question_attempt_id
       )
       OR (SELECT normalized_credit FROM ple_private.grading_result
            WHERE question_attempt_id = prepared.question_attempt_id) <> 0.67
       OR (SELECT count(*) FROM ple_private.assessment_submission
            WHERE assessment_attempt_id = 'e3000000-0000-0000-0000-000000000010') <> 1 THEN
        RAISE EXCEPTION 'direct finalization replay changed immutable credit or receipt';
    END IF;
END
$$;

SET LOCAL ROLE ple_api_owner;
-- Submitted Attempts never resume and consume the one-Attempt limit.
DO $$
DECLARE attempt_limit_rejected boolean := false;
BEGIN
    BEGIN
        PERFORM * FROM ple_api.start_assessment_attempt(
            'e3000000-0000-0000-0000-000000000012',
            '00000000-0000-0000-0000-00000000eb06',
            current_setting('ple.test_direct_assessment_id'),
            '[]'::jsonb,
            '[]'::jsonb
        );
    EXCEPTION WHEN check_violation THEN
        attempt_limit_rejected := true;
    WHEN OTHERS THEN
        RAISE EXCEPTION 'attempt limit unexpected SQLSTATE %, message %', SQLSTATE, SQLERRM;
    END;
    IF NOT attempt_limit_rejected THEN
        RAISE EXCEPTION 'submitted incorrect Attempt resumed or bypassed its limit';
    END IF;
END
$$;

-- The Student landing repeats the same limit truth as Start. A submitted
-- Attempt counts immediately, even though its score is read from immutable
-- credit and current Entry points.
DO $$
BEGIN
    IF (SELECT count(*)
          FROM ple_api.list_released_live_student_assessments(
              current_setting('ple.test_direct_course_id')
          ) AS landing
         WHERE landing.assessment_id = current_setting('ple.test_direct_assessment_id')
           AND landing.start_decision = 'attempt_limit_reached') <> 1 THEN
        RAISE EXCEPTION 'Student landing did not report the submitted Attempt limit';
    END IF;
END
$$;

-- An expired all-unanswered Attempt has no invented backend result, but every
-- issued position remains in the possible-score denominator at zero credit.
SET LOCAL ROLE ple_api_owner;
INSERT INTO ple_private.course_roster_profile (
    course_roster_profile_id, course_instance_id, student_account_id, roster_id, roster_name, created_at
) VALUES (
    'e3000000-0000-0000-0000-000000000003',
    current_setting('ple.test_direct_course_id'),
    current_setting('ple.test_direct_expiry_student_id'),
    'expiry-student-920001', 'Synthetic Expiry Student', pg_catalog.transaction_timestamp()
)
ON CONFLICT DO NOTHING;
SET LOCAL ROLE ple_private_owner;
INSERT INTO ple_private.student_assessment_accommodation (
    accommodation_id, course_instance_id, student_record_id, assessment_id, available_at, due_at,
    closes_at, time_multiplier, assessment_attempt_limit, created_at
) VALUES (
    'e3000000-0000-0000-0000-000000000002',
    current_setting('ple.test_direct_course_id'),
    '00000000-0000-0000-0000-00000000eb02',
    current_setting('ple.test_direct_assessment_id'),
    clock_timestamp() - interval '1 hour', clock_timestamp() + interval '1 hour',
    clock_timestamp() + interval '2 hours', 1, 1, pg_catalog.transaction_timestamp()
)
ON CONFLICT (student_record_id, assessment_id) DO UPDATE
   SET available_at = EXCLUDED.available_at,
       due_at = EXCLUDED.due_at,
       closes_at = EXCLUDED.closes_at,
       time_multiplier = EXCLUDED.time_multiplier,
       assessment_attempt_limit = EXCLUDED.assessment_attempt_limit,
       accommodation_edit_number = ple_private.student_assessment_accommodation.accommodation_edit_number + 1;
SET LOCAL ROLE ple_api_owner;
-- The generic baseline maps Student record eb02 to the minted Student Account.
SELECT set_config('ple.session_account_id', current_setting('ple.test_direct_expiry_student_id'), true);
SELECT * FROM ple_api.start_assessment_attempt(
    'e3000000-0000-0000-0000-000000000020',
    '00000000-0000-0000-0000-00000000eb02',
    current_setting('ple.test_direct_assessment_id'),
    '[]'::jsonb,
    jsonb_build_array(jsonb_build_object(
        'issued_question_id', 'e3000000-0000-0000-0000-000000000021',
        'assessment_entry_id', current_setting('ple.test_direct_entry_id')::uuid,
        'issued_position', 0,
        'published_question_id', current_setting('ple.test_direct_question_id'),
        'revision_number', 1
    ))
);
SET LOCAL ROLE ple_private_owner;
INSERT INTO ple_private.question_attempt (
    course_instance_id, question_attempt_id, issued_question_id, issued_at,
    delivery_toolchain_id, rendered_question_sha256
)
SELECT issued.course_instance_id,
       'e3000000-0000-0000-0000-000000000041',
       'e3000000-0000-0000-0000-000000000021',
       clock_timestamp(),
       ple_private.ensure_delivery_toolchain('ple', '1', NULL, NULL, 'ple', '1', 'not_applicable'),
       decode(repeat('41', 32), 'hex')
  FROM ple_private.issued_question AS issued
 WHERE issued.issued_question_id = 'e3000000-0000-0000-0000-000000000021';
ALTER TABLE ple_private.assessment_attempt DISABLE TRIGGER assessment_attempt_retains_evidence;
UPDATE ple_private.assessment_attempt
   SET expires_at = started_at
 WHERE assessment_attempt_id = 'e3000000-0000-0000-0000-000000000020';
ALTER TABLE ple_private.assessment_attempt ENABLE TRIGGER assessment_attempt_retains_evidence;
-- This persistence oracle uses the shared private expiry wrapper. The
-- security catalog and worker-login adapter separately prove that only the
-- generic expiry worker receives the public worker capability.
SET LOCAL ROLE ple_private_owner;
DO $$
DECLARE score record;
BEGIN
    SELECT * INTO score
      FROM ple_private.commit_expired_student_assessment_attempt_finalization(
          'e3000000-0000-0000-0000-000000000020', '[]'::jsonb
      );
    IF score.points_earned <> 0 OR score.points_possible <> 3 THEN
        RAISE EXCEPTION 'expired unanswered Attempt omitted its issued position from current possible score';
    END IF;
END
$$;
SET LOCAL ROLE ple_private_owner;
DO $$
BEGIN
    IF (SELECT count(*) FROM ple_private.assessment_submission
         WHERE assessment_attempt_id = 'e3000000-0000-0000-0000-000000000020') <> 1
       OR (SELECT count(*) FROM ple_private.assessment_attempt_saved_response
            WHERE question_attempt_id = 'e3000000-0000-0000-0000-000000000041') <> 0
       OR (SELECT count(*) FROM ple_private.question_attempt
            WHERE question_attempt_id = 'e3000000-0000-0000-0000-000000000041'
              AND finalized_at IS NOT NULL) <> 1 THEN
        RAISE EXCEPTION 'expired unanswered Attempt did not close as immutable zero-credit work';
    END IF;
END
$$;

-- Gradebook reads the retained deadline closure as finished zero-credit work.
-- This is a read-only projection: it must include every issued position in
-- the current possible score without creating a result for an unanswered one.
SET LOCAL ROLE ple_api_owner;
SELECT set_config('ple.session_account_id', current_setting('ple.test_direct_instructor_id'), true);
DO $$
DECLARE observed_rows jsonb;
BEGIN
    IF NOT ple_api.current_session_account_is_course_instructor(
        current_setting('ple.test_direct_course_id')
    ) THEN
        RAISE EXCEPTION 'Gradebook fixture Instructor lacks Course authority';
    END IF;
    SELECT coalesce(jsonb_agg(jsonb_build_object(
               'assessmentId', gradebook.assessment_id,
               'completion', gradebook.assessment_attempt_completion,
               'pointsEarned', gradebook.points_earned,
               'pointsPossible', gradebook.points_possible
           ) ORDER BY gradebook.assessment_id), '[]'::jsonb)
      INTO observed_rows
      FROM ple_api.read_course_gradebook(current_setting('ple.test_direct_course_id')) AS gradebook
     WHERE gradebook.assessment_id = current_setting('ple.test_direct_assessment_id');
    IF (SELECT count(*)
          FROM ple_api.read_course_gradebook(current_setting('ple.test_direct_course_id')) AS gradebook
         WHERE gradebook.assessment_id = current_setting('ple.test_direct_assessment_id')
           AND gradebook.assessment_attempt_completion = 'completed'
           AND gradebook.points_earned = 0
           AND gradebook.points_possible = 3) <> 1 THEN
        RAISE EXCEPTION 'Gradebook expired unanswered observed rows %', observed_rows;
    END IF;
END
$$;

COMMIT;
