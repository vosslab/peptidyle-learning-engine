-- M7 Student Course landing score-disclosure acceptance oracle.
--
-- This follows the M7 access fixture. It uses its completed, receipt-backed
-- Attempt and rolls each ordinary fixture insert back after the assertion.
\set ON_ERROR_STOP on
\set VERBOSITY verbose

DO $$
BEGIN
    IF to_regprocedure('ple_api.list_released_live_student_assignments(bigint)') IS NULL
       OR NOT has_function_privilege(
           'ple_app', 'ple_api.list_released_live_student_assignments(bigint)', 'EXECUTE'
       )
       OR has_function_privilege(
           'public', 'ple_api.list_released_live_student_assignments(bigint)', 'EXECUTE'
       )
       OR (SELECT pg_get_userbyid(proowner)
             FROM pg_proc
            WHERE oid = 'ple_api.list_released_live_student_assignments(bigint)'::regprocedure)
          <> 'ple_private_owner' THEN
        RAISE EXCEPTION 'M7 Student landing score procedure authority is not exact';
    END IF;
END
$$;

-- The preceding access oracle creates this latest completed Attempt for the
-- claimed Student session (ab...). Its receipt boundary is fixture-only.
SELECT course.reference_number AS m7_score_course_reference,
       assignment.reference_number AS m7_score_assignment_reference,
       attempt.assignment_attempt_id AS m7_score_attempt_id,
       attempt.attempt_number AS m7_score_attempt_number
  FROM ple_private.assignment_attempt AS attempt
  JOIN ple_private.assignment_submission AS assignment_submission
    ON assignment_submission.assignment_attempt_id = attempt.assignment_attempt_id
  JOIN ple_data.assignment AS assignment ON assignment.assignment_id = attempt.assignment_id
  JOIN ple_data.course_instance AS course ON course.course_id = assignment.course_id
  JOIN ple_data.assignment_revision AS revision
    ON revision.assignment_revision_id = attempt.assignment_revision_id
 WHERE assignment_submission.receipt ->> 'submissionBoundary' = 'm7-access'
   AND attempt.completed_at IS NOT NULL
   AND revision.feedback_score = 'after_submit'
   AND NOT EXISTS (
       SELECT 1
         FROM ple_private.assignment_attempt AS newer
        WHERE newer.student_record_id = attempt.student_record_id
          AND newer.assignment_id = attempt.assignment_id
          AND (newer.started_at, newer.assignment_attempt_id)
              > (attempt.started_at, attempt.assignment_attempt_id)
   )
 ORDER BY attempt.started_at DESC, attempt.assignment_attempt_id DESC
 LIMIT 1
\gset
SELECT set_config('ple_e2e.m7_score_course_reference', :'m7_score_course_reference', false);
SELECT set_config(
    'ple_e2e.m7_score_assignment_reference', :'m7_score_assignment_reference', false
);
SELECT set_config('ple_e2e.m7_score_attempt_id', :'m7_score_attempt_id', false);
SELECT set_config('ple_e2e.m7_score_attempt_number', :'m7_score_attempt_number', false);

-- Every issued Question in this fixture has precisely one complete chain.
DO $$
BEGIN
    IF EXISTS (
        SELECT 1
          FROM ple_private.issued_question AS issued
          LEFT JOIN LATERAL (
              SELECT count(DISTINCT question_attempt.question_attempt_id) AS attempt_count,
                     count(DISTINCT submission.submission_id) AS submission_count,
                     count(DISTINCT grading.question_submission_grading_id) AS grading_count,
                     count(DISTINCT result.grading_result_id) AS result_count,
                     count(DISTINCT receipt.automated_grading_receipt_id) AS receipt_count,
                     COALESCE(bool_and(grading.grading_state = 'graded'), false) AS is_graded
                FROM ple_private.question_attempt AS question_attempt
                LEFT JOIN ple_private.question_submission AS submission
                  ON submission.question_attempt_id = question_attempt.question_attempt_id
                LEFT JOIN ple_private.question_submission_grading AS grading
                  ON grading.submission_id = submission.submission_id
                LEFT JOIN ple_private.grading_result AS result
                  ON result.question_submission_grading_id = grading.question_submission_grading_id
                 AND result.submission_id = submission.submission_id
                 AND result.question_attempt_id = question_attempt.question_attempt_id
                LEFT JOIN ple_audit.automated_grading_receipt AS receipt
                  ON receipt.question_submission_grading_id = grading.question_submission_grading_id
                 AND receipt.grading_result_id = result.grading_result_id
               WHERE question_attempt.issued_question_id = issued.issued_question_id
          ) AS chain ON true
         WHERE issued.assignment_attempt_id = current_setting('ple_e2e.m7_score_attempt_id')::uuid
           AND (chain.attempt_count, chain.submission_count, chain.grading_count,
                chain.result_count, chain.receipt_count, chain.is_graded)
               IS DISTINCT FROM (1::bigint, 1::bigint, 1::bigint, 1::bigint,
                                 1::bigint, true)
    ) OR NOT EXISTS (
        SELECT 1
          FROM ple_private.issued_question AS issued
         WHERE issued.assignment_attempt_id = current_setting('ple_e2e.m7_score_attempt_id')::uuid
    ) THEN
        RAISE EXCEPTION 'M7 landing score fixture lacks complete issued Question chains';
    END IF;
END
$$;

-- The public landing assertions retain their private fixture oracle as a
-- scalar expectation, computed before entering the Student application role.
SELECT set_config(
    'ple_e2e.m7_score_expected_issued_count', count(*)::text, false
)
  FROM ple_private.issued_question
 WHERE assignment_attempt_id = current_setting('ple_e2e.m7_score_attempt_id')::uuid;

BEGIN;

-- Each procedure call uses the production Student session boundary.
SET LOCAL ROLE ple_auth;
SELECT ple_api.resolve_and_install_session(decode(repeat('ab', 32), 'hex'));
SET LOCAL ROLE ple_app;
DO $$
DECLARE
    v_landing record;
BEGIN
    SELECT * INTO v_landing
      FROM ple_api.list_released_live_student_assignments(
          current_setting('ple_e2e.m7_score_course_reference')::bigint
      ) AS landing
     WHERE landing.assignment_reference_number =
           current_setting('ple_e2e.m7_score_assignment_reference')::bigint;

    IF NOT FOUND
       OR v_landing.assignment_attempt_number <>
          current_setting('ple_e2e.m7_score_attempt_number')::integer
       OR v_landing.question_count <>
          current_setting('ple_e2e.m7_score_expected_issued_count')::bigint
       OR v_landing.graded_question_count <>
          current_setting('ple_e2e.m7_score_expected_issued_count')::bigint
       OR (v_landing.points_earned IS NULL) <> (v_landing.points_possible IS NULL)
       OR v_landing.points_earned IS NULL THEN
        RAISE EXCEPTION
            'latest complete receipt-backed Attempt did not release an all-or-none score';
    END IF;
END
$$;

-- One additional issued Question lacks a Question Attempt. It widens the
-- denominator but makes the score pair unavailable.
SAVEPOINT m7_missing_question_attempt;
RESET ROLE;
SET LOCAL ROLE ple_private_owner;
INSERT INTO ple_private.issued_question (
    issued_question_id, assignment_attempt_id, assignment_entry_id, question_id,
    revision_number, issued_position, point_value, scoring_rule,
    question_statistics_eligibility, question_pool_selection_id, question_pool_item_id
)
SELECT gen_random_uuid(), issued.assignment_attempt_id, issued.assignment_entry_id,
       issued.question_id, issued.revision_number, issued.issued_position + 100,
       issued.point_value, issued.scoring_rule, issued.question_statistics_eligibility,
       issued.question_pool_selection_id, issued.question_pool_item_id
  FROM ple_private.issued_question AS issued
 WHERE issued.assignment_attempt_id = current_setting('ple_e2e.m7_score_attempt_id')::uuid
 ORDER BY issued.issued_position
 LIMIT 1;
SELECT set_config(
    'ple_e2e.m7_score_expected_missing_issued_count', count(*)::text, true
)
  FROM ple_private.issued_question
 WHERE assignment_attempt_id = current_setting('ple_e2e.m7_score_attempt_id')::uuid;
RESET ROLE;
SET LOCAL ROLE ple_app;
DO $$
DECLARE
    v_landing record;
BEGIN
    SELECT * INTO v_landing
      FROM ple_api.list_released_live_student_assignments(
          current_setting('ple_e2e.m7_score_course_reference')::bigint
      ) AS landing
     WHERE landing.assignment_reference_number =
           current_setting('ple_e2e.m7_score_assignment_reference')::bigint;
    IF NOT FOUND
       OR v_landing.question_count <>
          current_setting('ple_e2e.m7_score_expected_missing_issued_count')::bigint
       OR v_landing.graded_question_count <>
          current_setting('ple_e2e.m7_score_expected_missing_issued_count')::bigint - 1
       OR v_landing.points_earned IS NOT NULL
       OR v_landing.points_possible IS NOT NULL THEN
        RAISE EXCEPTION 'unattempted issued Question did not conceal the widened score';
    END IF;
END
$$;
ROLLBACK TO SAVEPOINT m7_missing_question_attempt;

-- A second Question Attempt on one issued Question must conceal the score
-- without changing the assignment denominator or counted graded Question.
SAVEPOINT m7_duplicate_question_attempt;
RESET ROLE;
SET LOCAL ROLE ple_api_owner;
INSERT INTO ple_private.question_attempt (
    question_attempt_id, issued_question_id, question_seed,
    generated_parameter_sha256, issued_at, deadline_at,
    question_attempt_state, reproduction_details
)
SELECT gen_random_uuid(), question_attempt.issued_question_id, question_attempt.question_seed,
       question_attempt.generated_parameter_sha256, question_attempt.issued_at,
       question_attempt.deadline_at, question_attempt.question_attempt_state,
       question_attempt.reproduction_details
  FROM ple_private.question_attempt AS question_attempt
  JOIN ple_private.issued_question AS issued
    ON issued.issued_question_id = question_attempt.issued_question_id
 WHERE issued.assignment_attempt_id = current_setting('ple_e2e.m7_score_attempt_id')::uuid
 ORDER BY issued.issued_position
 LIMIT 1;
RESET ROLE;
SET LOCAL ROLE ple_app;
DO $$
DECLARE
    v_landing record;
BEGIN
    SELECT * INTO v_landing
      FROM ple_api.list_released_live_student_assignments(
          current_setting('ple_e2e.m7_score_course_reference')::bigint
      ) AS landing
     WHERE landing.assignment_reference_number =
           current_setting('ple_e2e.m7_score_assignment_reference')::bigint;
    IF NOT FOUND
       OR v_landing.question_count <>
          current_setting('ple_e2e.m7_score_expected_issued_count')::bigint
       OR v_landing.graded_question_count <>
          current_setting('ple_e2e.m7_score_expected_issued_count')::bigint
       OR v_landing.points_earned IS NOT NULL
       OR v_landing.points_possible IS NOT NULL THEN
        RAISE EXCEPTION 'duplicate Question Attempt widened or released a score';
    END IF;
END
$$;
ROLLBACK TO SAVEPOINT m7_duplicate_question_attempt;

-- Finish the claimed Student fixture transaction before checking callers that
-- have no claim on its Course. Session installation is transaction-scoped.
ROLLBACK;

-- The procedure conceals the same Course for an anonymous caller.
BEGIN;
RESET ROLE;
SET LOCAL ROLE ple_app;
DO $$
BEGIN
    PERFORM 1 FROM ple_api.list_released_live_student_assignments(
        current_setting('ple_e2e.m7_score_course_reference')::bigint
    );
    RAISE EXCEPTION 'anonymous caller read Student landing assignments';
EXCEPTION WHEN insufficient_privilege THEN NULL;
END
$$;
ROLLBACK;

-- A separate foreign Student session cannot read the claimed Student's Course.
BEGIN;
RESET ROLE;
SET LOCAL ROLE ple_auth;
SELECT ple_api.resolve_and_install_session(decode(repeat('fa', 32), 'hex'));
SET LOCAL ROLE ple_app;
DO $$
BEGIN
    PERFORM 1 FROM ple_api.list_released_live_student_assignments(
        current_setting('ple_e2e.m7_score_course_reference')::bigint
    );
    RAISE EXCEPTION 'foreign Student read Student landing assignments';
EXCEPTION WHEN insufficient_privilege THEN NULL;
END
$$;

ROLLBACK;
