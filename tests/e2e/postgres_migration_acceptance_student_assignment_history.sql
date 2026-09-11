-- M7 selected completed Student Assignment Attempt history acceptance oracle.
--
-- This runs after the access oracle.  It re-resolves that oracle's committed
-- M5 and M11 fixtures because each psql file has a fresh session.
\set ON_ERROR_STOP on
\set VERBOSITY verbose

DO $$
BEGIN
    IF to_regprocedure('ple_api.read_student_assignment_attempt_history(bigint)') IS NULL
       OR NOT has_function_privilege(
           'ple_app', 'ple_api.read_student_assignment_attempt_history(bigint)', 'EXECUTE'
       )
       OR has_function_privilege(
           'public', 'ple_api.read_student_assignment_attempt_history(bigint)', 'EXECUTE'
       )
       OR (SELECT pg_get_userbyid(proowner)
             FROM pg_proc
            WHERE oid = 'ple_api.read_student_assignment_attempt_history(bigint)'::regprocedure)
          <> 'ple_private_owner' THEN
        RAISE EXCEPTION 'M7 Student Assignment Attempt history procedure authority is not exact';
    END IF;
    IF to_regprocedure('ple_api.read_student_assignment_attempt_history_response_sources(bigint)') IS NULL
       OR NOT has_function_privilege(
           'ple_app', 'ple_api.read_student_assignment_attempt_history_response_sources(bigint)', 'EXECUTE'
       )
       OR has_function_privilege(
           'public', 'ple_api.read_student_assignment_attempt_history_response_sources(bigint)', 'EXECUTE'
       )
       OR (SELECT pg_get_userbyid(proowner)
             FROM pg_proc
            WHERE oid = 'ple_api.read_student_assignment_attempt_history_response_sources(bigint)'::regprocedure)
          <> 'ple_private_owner' THEN
        RAISE EXCEPTION 'M7 Student Assignment Attempt response-source procedure authority is not exact';
    END IF;
END
$$;

-- The access oracle builds the fully graded numeric-zero fixture from the
-- M5 saved-response evidence.  The original saved submission remains pending.
SELECT attempt.reference_number AS m7_history_pending_reference
  FROM ple_private.assignment_attempt AS attempt
  JOIN ple_data.assignment AS assignment ON assignment.assignment_id = attempt.assignment_id
  JOIN ple_private.assignment_submission AS assignment_submission
    ON assignment_submission.assignment_attempt_id = attempt.assignment_attempt_id
 WHERE attempt.completed_at IS NOT NULL
   AND 2 = (
       SELECT count(*)
         FROM ple_private.question_submission_grading AS grading
         JOIN ple_private.question_submission AS submission
           ON submission.submission_id = grading.submission_id
         JOIN ple_private.question_attempt AS question_attempt
           ON question_attempt.question_attempt_id = submission.question_attempt_id
         JOIN ple_private.issued_question AS issued
           ON issued.issued_question_id = question_attempt.issued_question_id
        WHERE issued.assignment_attempt_id = attempt.assignment_attempt_id
          AND grading.grading_state = 'pending'
   )
 ORDER BY attempt.completed_at DESC
 LIMIT 1
\gset
SELECT attempt.reference_number AS m7_history_zero_reference
  FROM ple_private.assignment_attempt AS attempt
  JOIN ple_private.assignment_submission AS assignment_submission
    ON assignment_submission.assignment_attempt_id = attempt.assignment_attempt_id
 WHERE assignment_submission.receipt ->> 'submissionBoundary' = 'm7-access'
 ORDER BY assignment_submission.submitted_at DESC
 LIMIT 1
\gset
SELECT attempt.reference_number AS m7_history_active_m11_reference
  FROM ple_private.assignment_attempt AS attempt
  JOIN ple_data.student_record AS student ON student.student_record_id = attempt.student_record_id
  JOIN ple_private.authenticated_session AS session ON session.account_id = student.student_account_id
 WHERE attempt.completed_at IS NULL
   AND attempt.delivery_assignment_title = 'M11 delivery title after retime'
   AND session.token_hash = decode(repeat('d2', 32), 'hex')
 ORDER BY attempt.started_at DESC
 LIMIT 1
\gset
SELECT attempt.reference_number AS m7_history_source_reference
  FROM ple_private.assignment_attempt AS attempt
 WHERE attempt.assignment_attempt_id = '00000000-0000-0000-0000-000000000114'::uuid
   AND attempt.completed_at IS NOT NULL
\gset
SELECT course.reference_number AS m7_history_parent_course_reference,
       course.course_short_name AS m7_history_parent_course_short_name,
       course.course_long_name AS m7_history_parent_course_long_name,
       course.course_theme AS m7_history_parent_course_theme
  FROM ple_private.assignment_attempt AS attempt
  JOIN ple_data.assignment AS assignment ON assignment.assignment_id = attempt.assignment_id
  JOIN ple_data.course_instance AS course ON course.course_id = assignment.course_id
 WHERE attempt.reference_number = :'m7_history_pending_reference'::bigint
\gset

SELECT set_config('ple_e2e.m7_history_pending_reference', :'m7_history_pending_reference', false);
SELECT set_config('ple_e2e.m7_history_zero_reference', :'m7_history_zero_reference', false);
SELECT set_config('ple_e2e.m7_history_active_m11_reference', :'m7_history_active_m11_reference', false);
SELECT set_config('ple_e2e.m7_history_source_reference', :'m7_history_source_reference', false);
SELECT set_config(
    'ple_e2e.m7_history_parent_course_reference',
    :'m7_history_parent_course_reference', false
);
SELECT set_config(
    'ple_e2e.m7_history_parent_course_short_name',
    :'m7_history_parent_course_short_name', false
);
SELECT set_config(
    'ple_e2e.m7_history_parent_course_long_name',
    :'m7_history_parent_course_long_name', false
);
SELECT set_config(
    'ple_e2e.m7_history_parent_course_theme',
    :'m7_history_parent_course_theme', false
);

-- The acceptance bootstrap computes private fixture expectations before the
-- Student session begins.  ple_app is procedure-only: the session below must
-- exercise only the public reader, never direct private relations.
DO $$
DECLARE
    v_expected_pending_questions jsonb;
    v_expected_zero_results jsonb;
    v_expected_pending_feedback jsonb;
    v_expected_pending_due_millis bigint;
    v_expected_pending_close_millis bigint;
    v_expected_pending_submitted_millis bigint;
    v_expected_sources jsonb;
BEGIN
    SELECT COALESCE(jsonb_agg(jsonb_build_object(
               'position', issued.issued_position + 1,
               'responseState', CASE WHEN submission.question_attempt_id IS NULL
                                     THEN 'closed' ELSE 'submitted' END
           ) ORDER BY issued.issued_position), '[]'::jsonb),
           jsonb_build_object(
               'score', revision.feedback_score,
               'per_item_correctness', revision.feedback_per_item_correctness,
               'submitted_response', revision.feedback_submitted_response,
               'question_feedback', revision.feedback_question_feedback,
               'question_answer', revision.feedback_question_answer,
               'question_answer_explanation', revision.feedback_question_answer_explanation,
               'class_statistics', revision.feedback_class_statistics
           ),
           floor(extract(epoch FROM CASE WHEN attempt.delivery_assignment_title IS NOT NULL
                                         THEN attempt.delivery_due_at ELSE revision.due_at END) * 1000)::bigint,
           floor(extract(epoch FROM revision.closes_at) * 1000)::bigint,
           floor(extract(epoch FROM assignment_submission.submitted_at) * 1000)::bigint
      INTO v_expected_pending_questions, v_expected_pending_feedback,
           v_expected_pending_due_millis, v_expected_pending_close_millis,
           v_expected_pending_submitted_millis
      FROM ple_private.assignment_attempt AS attempt
      JOIN ple_data.assignment_revision AS revision
        ON revision.assignment_revision_id = attempt.assignment_revision_id
      JOIN ple_private.assignment_submission AS assignment_submission
        ON assignment_submission.assignment_attempt_id = attempt.assignment_attempt_id
      JOIN ple_private.issued_question AS issued
        ON issued.assignment_attempt_id = attempt.assignment_attempt_id
      JOIN ple_private.question_attempt AS question_attempt
        ON question_attempt.issued_question_id = issued.issued_question_id
      LEFT JOIN ple_private.question_submission AS submission
        ON submission.question_attempt_id = question_attempt.question_attempt_id
     WHERE attempt.reference_number = current_setting('ple_e2e.m7_history_pending_reference')::bigint
     GROUP BY attempt.assignment_attempt_id, revision.feedback_score,
              revision.feedback_per_item_correctness, revision.feedback_submitted_response,
              revision.feedback_question_feedback, revision.feedback_question_answer,
              revision.feedback_question_answer_explanation, revision.feedback_class_statistics,
              attempt.delivery_assignment_title, attempt.delivery_due_at, revision.due_at,
              revision.closes_at, assignment_submission.submitted_at;

    IF NOT FOUND OR v_expected_pending_questions IS NULL THEN
        RAISE EXCEPTION 'pending completed Attempt history fixture did not produce owned expectations';
    END IF;

    SELECT jsonb_agg(jsonb_build_object(
               'position', issued.issued_position + 1,
               'correct', result.correct,
               'pointsEarned', result.points_earned,
               'pointsPossible', result.points_possible
           ) ORDER BY issued.issued_position)
      INTO v_expected_zero_results
      FROM ple_private.issued_question AS issued
      JOIN ple_private.question_attempt AS question_attempt
        ON question_attempt.issued_question_id = issued.issued_question_id
      JOIN ple_private.question_submission AS submission
        ON submission.question_attempt_id = question_attempt.question_attempt_id
      JOIN ple_private.question_submission_grading AS grading
        ON grading.submission_id = submission.submission_id
      JOIN ple_private.grading_result AS result
        ON result.question_submission_grading_id = grading.question_submission_grading_id
       AND result.submission_id = submission.submission_id
       AND result.question_attempt_id = question_attempt.question_attempt_id
      JOIN ple_audit.automated_grading_receipt AS receipt
        ON receipt.question_submission_grading_id = grading.question_submission_grading_id
       AND receipt.grading_result_id = result.grading_result_id
     WHERE issued.assignment_attempt_id = (
         SELECT assignment_attempt_id
           FROM ple_private.assignment_attempt
          WHERE reference_number = current_setting('ple_e2e.m7_history_zero_reference')::bigint
     );
    IF v_expected_zero_results IS NULL OR v_expected_zero_results = '[]'::jsonb THEN
        RAISE EXCEPTION 'zero-credit Attempt history fixture did not produce graded expectations';
    END IF;

    IF NOT EXISTS (
        SELECT 1
          FROM ple_private.assignment_attempt AS attempt
          JOIN ple_data.assignment AS assignment ON assignment.assignment_id = attempt.assignment_id
         WHERE attempt.reference_number = current_setting('ple_e2e.m7_history_active_m11_reference')::bigint
           AND attempt.delivery_assignment_title = 'M11 delivery title after retime'
           AND attempt.delivery_due_at IS NULL
           AND assignment.due_at <= statement_timestamp()
    ) THEN
        RAISE EXCEPTION 'M11 no-deadline delivery discriminator fixture is invalid';
    END IF;

    SELECT jsonb_agg(jsonb_build_object(
               'position', issued.issued_position + 1,
               'student_response', submission.student_response,
               'backend', binding.backend,
               'question_attempt_id', question_attempt.question_attempt_id,
               'question_id', issued.question_id,
               'revision_number', issued.revision_number,
               'source_object_id', binding.source_object_id::text,
               'source_object_address', source_object.object_address,
               'source_object_checksum', binding.source_object_checksum,
               'webwork_pg_path', binding.webwork_pg_path,
               'question_seed', question_attempt.question_seed::text,
               'presentation_nonce', presentation.presentation_nonce,
               'presentation_checksum', presentation.presentation_checksum,
               'question_asset_renditions', '[]'::jsonb
           ) ORDER BY issued.issued_position)
      INTO v_expected_sources
      FROM ple_private.assignment_attempt AS attempt
      JOIN ple_private.issued_question AS issued
        ON issued.assignment_attempt_id = attempt.assignment_attempt_id
      JOIN ple_private.question_attempt AS question_attempt
        ON question_attempt.issued_question_id = issued.issued_question_id
      LEFT JOIN ple_private.question_submission AS submission
        ON submission.question_attempt_id = question_attempt.question_attempt_id
      JOIN ple_private.question_attempt_presentation_binding AS presentation
        ON presentation.question_attempt_id = question_attempt.question_attempt_id
      JOIN ple_private.question_revision_source_binding AS binding
        ON binding.question_id = issued.question_id AND binding.revision_number = issued.revision_number
      JOIN ple_private.object_record AS source_object ON source_object.object_id = binding.source_object_id
     WHERE attempt.reference_number = current_setting('ple_e2e.m7_history_source_reference')::bigint;
    IF v_expected_sources IS NULL
       OR jsonb_array_length(v_expected_sources) = 0
       OR EXISTS (
           SELECT 1 FROM jsonb_array_elements(v_expected_sources) AS source(value)
            WHERE source.value ->> 'backend' <> 'ple'
               OR source.value ->> 'student_response' IS NOT NULL
       ) THEN
        RAISE EXCEPTION 'completed source-bound Attempt fixture did not retain NULL recorded responses';
    END IF;

    PERFORM set_config(
        'ple_e2e.m7_history_pending_expected',
        jsonb_build_object(
            'questions', v_expected_pending_questions,
            'feedbackRule', v_expected_pending_feedback,
            'dueAtMillis', v_expected_pending_due_millis,
            'closesAtMillis', v_expected_pending_close_millis,
            'submittedAtMillis', v_expected_pending_submitted_millis
        )::text,
        false
    );
    PERFORM set_config('ple_e2e.m7_history_zero_expected', v_expected_zero_results::text, false);
    PERFORM set_config('ple_e2e.m7_history_source_expected', v_expected_sources::text, false);
END
$$;

BEGIN;
SET LOCAL ROLE ple_auth;
SELECT ple_api.resolve_and_install_session(decode(repeat('ab', 32), 'hex'));
SET LOCAL ROLE ple_app;
DO $$
DECLARE
    v_pending record;
    v_zero record;
    v_closed record;
    v_expected_pending jsonb := current_setting('ple_e2e.m7_history_pending_expected')::jsonb;
    v_expected_zero_results jsonb := current_setting('ple_e2e.m7_history_zero_expected')::jsonb;
    v_expected_sources jsonb := current_setting('ple_e2e.m7_history_source_expected')::jsonb;
    v_actual_sources jsonb;
BEGIN
    SELECT * INTO v_pending
      FROM ple_api.read_student_assignment_attempt_history(
          current_setting('ple_e2e.m7_history_pending_reference')::bigint
      );
    IF NOT FOUND
       OR v_pending.assignment_reference_number IS NULL
       OR v_pending.course_reference_number IS NULL
       OR v_pending.course_short_name IS NULL
       OR v_pending.course_long_name IS NULL
       OR v_pending.course_theme IS NULL
       OR v_pending.course_reference_number IS DISTINCT FROM
          current_setting('ple_e2e.m7_history_parent_course_reference')::bigint
       OR v_pending.course_short_name IS DISTINCT FROM
          current_setting('ple_e2e.m7_history_parent_course_short_name')
       OR v_pending.course_long_name IS DISTINCT FROM
          current_setting('ple_e2e.m7_history_parent_course_long_name')
       OR v_pending.course_theme IS DISTINCT FROM
          current_setting('ple_e2e.m7_history_parent_course_theme')
       OR v_pending.state <> 'submitted'
       OR v_pending.questions <> (v_expected_pending -> 'questions')
       OR v_pending.feedback_rule <> (v_expected_pending -> 'feedbackRule')
       OR v_pending.due_at_millis IS DISTINCT FROM (v_expected_pending ->> 'dueAtMillis')::bigint
       OR v_pending.closes_at_millis IS DISTINCT FROM (v_expected_pending ->> 'closesAtMillis')::bigint
       OR v_pending.submitted_at_millis IS DISTINCT FROM (v_expected_pending ->> 'submittedAtMillis')::bigint
       OR v_pending.evaluated_at_millis IS NULL
       OR v_pending.grading_is_current
       OR v_pending.grading_results <> '[]'::jsonb THEN
        RAISE EXCEPTION 'pending completed Attempt history did not preserve owned metadata or conceal grading evidence';
    END IF;

    SELECT * INTO v_zero
      FROM ple_api.read_student_assignment_attempt_history(
          current_setting('ple_e2e.m7_history_zero_reference')::bigint
      );
    IF NOT FOUND
       OR v_zero.state <> 'submitted'
       OR NOT v_zero.grading_is_current
       OR v_zero.grading_results <> v_expected_zero_results
       OR NOT EXISTS (
           SELECT 1 FROM jsonb_array_elements(v_zero.grading_results) AS result(value)
            WHERE result.value ->> 'pointsEarned' = '0'
       ) THEN
        RAISE EXCEPTION 'fully graded zero-credit Attempt history lost current private results';
    END IF;

    SELECT * INTO v_closed
      FROM ple_api.read_student_assignment_attempt_history(
          current_setting('ple_e2e.m7_history_source_reference')::bigint
      );
    IF NOT FOUND
       OR v_closed.state <> 'closed'
       OR v_closed.course_reference_number IS NULL
       OR v_closed.course_short_name IS NULL
       OR v_closed.course_long_name IS NULL
       OR v_closed.course_theme IS NULL
       OR v_closed.course_reference_number IS DISTINCT FROM
          current_setting('ple_e2e.m7_history_parent_course_reference')::bigint
       OR v_closed.course_short_name IS DISTINCT FROM
          current_setting('ple_e2e.m7_history_parent_course_short_name')
       OR v_closed.course_long_name IS DISTINCT FROM
          current_setting('ple_e2e.m7_history_parent_course_long_name')
       OR v_closed.course_theme IS DISTINCT FROM
          current_setting('ple_e2e.m7_history_parent_course_theme') THEN
        RAISE EXCEPTION 'closed Attempt history did not preserve its owned Course metadata';
    END IF;

    SELECT jsonb_agg(to_jsonb(source) ORDER BY source."position") INTO v_actual_sources
      FROM ple_api.read_student_assignment_attempt_history_response_sources(
          current_setting('ple_e2e.m7_history_source_reference')::bigint
      ) AS source;
    IF v_actual_sources IS DISTINCT FROM v_expected_sources
       OR EXISTS (
           SELECT 1 FROM jsonb_array_elements(v_actual_sources) AS source(value)
            WHERE source.value ->> 'student_response' IS NOT NULL
       ) THEN
        RAISE EXCEPTION 'owned completed source reader did not reproduce the pinned NULL-response presentation';
    END IF;
END
$$;
COMMIT;

-- The active M11 Attempt belongs to d2, not the M5 Student.  It must stay
-- outside completed history even though the mutable Assignment is past due.
BEGIN;
SET LOCAL ROLE ple_auth;
SELECT ple_api.resolve_and_install_session(decode(repeat('d2', 32), 'hex'));
SET LOCAL ROLE ple_app;
DO $$
BEGIN
    IF EXISTS (
        SELECT 1 FROM ple_api.read_student_assignment_attempt_history(
            current_setting('ple_e2e.m7_history_active_m11_reference')::bigint
        )
    ) THEN
        RAISE EXCEPTION 'active M11 no-deadline Attempt entered completed history';
    END IF;
    IF EXISTS (
        SELECT 1 FROM ple_api.read_student_assignment_attempt_history_response_sources(
            current_setting('ple_e2e.m7_history_active_m11_reference')::bigint
        )
    ) THEN
        RAISE EXCEPTION 'active M11 Attempt exposed response sources';
    END IF;
END
$$;
COMMIT;

-- Concealment must happen before any protected field reaches anonymous,
-- foreign, or revoked callers.  Revocation rolls back with its fixture event.
BEGIN;
SET LOCAL ROLE ple_app;
DO $$
BEGIN
    PERFORM 1 FROM ple_api.read_student_assignment_attempt_history(
        current_setting('ple_e2e.m7_history_pending_reference')::bigint
    );
    IF FOUND THEN
        RAISE EXCEPTION 'anonymous caller read Student Assignment Attempt history';
    END IF;
    PERFORM 1 FROM ple_api.read_student_assignment_attempt_history_response_sources(
        current_setting('ple_e2e.m7_history_source_reference')::bigint
    );
    IF FOUND THEN
        RAISE EXCEPTION 'anonymous caller read Student Assignment Attempt response sources';
    END IF;
EXCEPTION WHEN insufficient_privilege THEN NULL;
END
$$;
COMMIT;

BEGIN;
SET LOCAL ROLE ple_app;
DO $$
BEGIN
    PERFORM 1 FROM ple_api.read_student_assignment_attempt_history_response_sources(
        current_setting('ple_e2e.m7_history_source_reference')::bigint
    );
    IF FOUND THEN
        RAISE EXCEPTION 'anonymous caller read Student Assignment Attempt response sources';
    END IF;
EXCEPTION WHEN insufficient_privilege THEN NULL;
END
$$;
COMMIT;

BEGIN;
SET LOCAL ROLE ple_auth;
SELECT ple_api.resolve_and_install_session(decode(repeat('fa', 32), 'hex'));
SET LOCAL ROLE ple_app;
DO $$
BEGIN
    PERFORM 1 FROM ple_api.read_student_assignment_attempt_history(
        current_setting('ple_e2e.m7_history_pending_reference')::bigint
    );
    IF FOUND THEN
        RAISE EXCEPTION 'foreign Student read Student Assignment Attempt history';
    END IF;
    PERFORM 1 FROM ple_api.read_student_assignment_attempt_history_response_sources(
        current_setting('ple_e2e.m7_history_source_reference')::bigint
    );
    IF FOUND THEN
        RAISE EXCEPTION 'foreign Student read Student Assignment Attempt response sources';
    END IF;
EXCEPTION WHEN insufficient_privilege THEN NULL;
END
$$;
COMMIT;

BEGIN;
SET LOCAL ROLE ple_auth;
SELECT ple_api.resolve_and_install_session(decode(repeat('fa', 32), 'hex'));
SET LOCAL ROLE ple_app;
DO $$
BEGIN
    PERFORM 1 FROM ple_api.read_student_assignment_attempt_history_response_sources(
        current_setting('ple_e2e.m7_history_source_reference')::bigint
    );
    IF FOUND THEN
        RAISE EXCEPTION 'foreign Student read Student Assignment Attempt response sources';
    END IF;
EXCEPTION WHEN insufficient_privilege THEN NULL;
END
$$;
COMMIT;

-- An authenticated Instructor still cannot read Student response sources.
BEGIN;
SET LOCAL ROLE ple_auth;
SELECT ple_api.resolve_and_install_session(decode(repeat('f0', 32), 'hex'));
SET LOCAL ROLE ple_app;
DO $$
BEGIN
    PERFORM 1 FROM ple_api.read_student_assignment_attempt_history_response_sources(
        current_setting('ple_e2e.m7_history_source_reference')::bigint
    );
    IF FOUND THEN
        RAISE EXCEPTION 'Instructor read Student Assignment Attempt response sources';
    END IF;
EXCEPTION WHEN insufficient_privilege THEN NULL;
END
$$;
COMMIT;

BEGIN;
SET ROLE ple_api_owner;
INSERT INTO ple_data.course_membership_event (
    course_membership_event_id, membership_id, event_kind, occurred_at, reason
) VALUES (
    gen_random_uuid(), '00000000-0000-0000-0000-000000000108'::uuid,
    'ended', clock_timestamp(), 'M7 Student Assignment Attempt history revocation acceptance'
);
RESET ROLE;
SET ROLE ple_auth;
SELECT ple_api.resolve_and_install_session(decode(repeat('ab', 32), 'hex'));
SET ROLE ple_app;
DO $$
BEGIN
    PERFORM 1 FROM ple_api.read_student_assignment_attempt_history(
        current_setting('ple_e2e.m7_history_pending_reference')::bigint
    );
    IF FOUND THEN
        RAISE EXCEPTION 'revoked Student membership read Student Assignment Attempt history';
    END IF;
    PERFORM 1 FROM ple_api.read_student_assignment_attempt_history_response_sources(
        current_setting('ple_e2e.m7_history_source_reference')::bigint
    );
    IF FOUND THEN
        RAISE EXCEPTION 'revoked Student membership read Student Assignment Attempt response sources';
    END IF;
EXCEPTION WHEN insufficient_privilege THEN NULL;
END
$$;
ROLLBACK;

-- Exercise the source reader under the revoked session itself, not only the
-- adjacent answer-free history projection.
BEGIN;
SET ROLE ple_api_owner;
INSERT INTO ple_data.course_membership_event (
    course_membership_event_id, membership_id, event_kind, occurred_at, reason
) VALUES (
    gen_random_uuid(), '00000000-0000-0000-0000-000000000108'::uuid,
    'ended', clock_timestamp(), 'M7 response-source revocation acceptance'
);
RESET ROLE;
SET ROLE ple_auth;
SELECT ple_api.resolve_and_install_session(decode(repeat('ab', 32), 'hex'));
SET ROLE ple_app;
DO $$
BEGIN
    PERFORM 1 FROM ple_api.read_student_assignment_attempt_history_response_sources(
        current_setting('ple_e2e.m7_history_source_reference')::bigint
    );
    IF FOUND THEN
        RAISE EXCEPTION 'revoked Student membership read Student Assignment Attempt response sources';
    END IF;
EXCEPTION WHEN insufficient_privilege THEN NULL;
END
$$;
ROLLBACK;
