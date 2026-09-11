-- M7 Student Assignment access facts acceptance oracle.
--
-- The M5 saved-response fixture has an active claimed Student, a completed
-- submission, and an older completed Attempt.  This oracle keeps that real
-- roster-and-invitation authority path and verifies only the widened access
-- projection; selected completed-Attempt history has its own oracle.
\set ON_ERROR_STOP on
\set VERBOSITY verbose

DO $$
BEGIN
    IF to_regprocedure('ple_api.live_demo_assignment_access(bigint,bigint)') IS NULL
       OR NOT has_function_privilege(
           'ple_app', 'ple_api.live_demo_assignment_access(bigint,bigint)', 'EXECUTE'
       )
       OR has_function_privilege(
           'public', 'ple_api.live_demo_assignment_access(bigint,bigint)', 'EXECUTE'
       )
       OR (SELECT pg_get_userbyid(proowner)
             FROM pg_proc
            WHERE oid = 'ple_api.live_demo_assignment_access(bigint,bigint)'::regprocedure)
          <> 'ple_private_owner'
       OR has_table_privilege('ple_app', 'ple_private.assignment_submission', 'SELECT')
       OR has_function_privilege(
           'ple_app', 'ple_api.has_automated_grading_receipt(uuid,uuid)', 'EXECUTE'
       ) THEN
        RAISE EXCEPTION 'M7 Student Assignment access procedure authority is not exact';
    END IF;
END
$$;

-- Each oracle runs in a fresh psql process.  Re-resolve the durable M11 and
-- M5 fixture rows rather than relying on a prior oracle's session settings.
SELECT course.reference_number AS m7_m11_course_reference,
       assignment.reference_number AS m7_m11_assignment_reference
  FROM ple_private.assignment_attempt AS attempt
  JOIN ple_data.assignment AS assignment ON assignment.assignment_id = attempt.assignment_id
  JOIN ple_data.course_instance AS course ON course.course_id = assignment.course_id
 WHERE attempt.completed_at IS NULL
   AND attempt.delivery_assignment_title = 'M11 delivery title after retime'
   AND attempt.delivery_due_at IS NULL
 ORDER BY attempt.started_at DESC
 LIMIT 1
\gset
SELECT attempt.assignment_attempt_id AS m7_saved_attempt_id
  FROM ple_private.assignment_attempt AS attempt
 WHERE attempt.completed_at IS NOT NULL
   AND EXISTS (
       SELECT 1 FROM ple_private.assignment_submission AS submission
        WHERE submission.assignment_attempt_id = attempt.assignment_attempt_id
   )
   AND 2 = (
       SELECT count(*) FROM ple_private.question_submission_grading AS grading
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
SELECT set_config('ple_e2e.m11_delivery_course_reference', :'m7_m11_course_reference', false);
SELECT set_config('ple_e2e.m11_delivery_assignment_reference', :'m7_m11_assignment_reference', false);
SELECT set_config('ple_e2e.m5_saved_attempt_id', :'m7_saved_attempt_id', false);
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM ple_private.authenticated_session AS session
        JOIN ple_data.student_record AS student
          ON student.student_account_id = session.account_id
        JOIN ple_private.assignment_attempt AS attempt
          ON attempt.student_record_id = student.student_record_id
         WHERE session.token_hash = decode(repeat('ab', 32), 'hex')
           AND attempt.assignment_attempt_id =
               current_setting('ple_e2e.m5_saved_attempt_id')::uuid
    ) OR NOT EXISTS (
        SELECT 1 FROM ple_private.authenticated_session AS session
         WHERE session.token_hash = decode(repeat('fa', 32), 'hex')
           AND session.product_role = 'student'
    ) OR NOT EXISTS (
        SELECT 1 FROM ple_data.course_membership AS membership
        JOIN ple_private.assignment_attempt AS attempt
          ON attempt.student_record_id = membership.student_record_id
         WHERE membership.membership_id = '00000000-0000-0000-0000-000000000108'::uuid
           AND attempt.assignment_attempt_id =
               current_setting('ple_e2e.m5_saved_attempt_id')::uuid
           AND ple_data.course_membership_is_active(membership.membership_id)
    ) THEN
        RAISE EXCEPTION 'M7 access oracle fixture authority is not the expected claimed Student boundary';
    END IF;
END
$$;

-- The M11 post-retime Student owns an active Attempt whose captured title and
-- NULL deadline survive a later current-Assignment past deadline.  Access
-- therefore uses issued facts and the captured title, rather than mutable
-- current schedule facts.
BEGIN;
SET LOCAL ROLE ple_auth;
SELECT ple_api.resolve_and_install_session(decode(repeat('d2', 32), 'hex'));
SET LOCAL ROLE ple_app;
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1
          FROM ple_api.live_demo_assignment_access(
              current_setting('ple_e2e.m11_delivery_course_reference')::bigint,
              current_setting('ple_e2e.m11_delivery_assignment_reference')::bigint
          ) AS access
         WHERE access.start_decision = 'may_start'
           AND access.assignment_title = 'M11 delivery title after retime'
           AND access.question_count > 0
           AND access.points_possible > 0
           AND access.assignment_attempt_time_limit_seconds = 600
           AND access.previous_attempts = '[]'::jsonb
    ) THEN
        RAISE EXCEPTION 'active Student access did not expose captured Assignment facts';
    END IF;
END
$$;
COMMIT;

-- Finalized M5 saved responses are complete but initially ungraded.  Keep
-- that Attempt incomplete, complete its earlier predecessor without a
-- submission, and build one later zero-credit completed Attempt from the
-- existing immutable issued evidence.  A numeric zero must remain a score;
-- incomplete grading and a closed prior Attempt must remain absent.
SELECT gen_random_uuid() AS m7_scored_attempt_id \gset
SELECT gen_random_uuid() AS m7_scored_pool_selection_id \gset
BEGIN;
SET ROLE ple_private_owner;
INSERT INTO ple_private.assignment_attempt (
    assignment_attempt_id, student_record_id, assignment_id, assignment_revision_id,
    started_at, completed_at, attempt_number, question_pool_reuse_rule,
    question_variation_rule
)
SELECT :'m7_scored_attempt_id'::uuid, attempt.student_record_id,
       attempt.assignment_id, attempt.assignment_revision_id,
       clock_timestamp() - interval '1 second', clock_timestamp(),
       (SELECT max(other.attempt_number) + 1
          FROM ple_private.assignment_attempt AS other
         WHERE other.student_record_id = attempt.student_record_id
           AND other.assignment_id = attempt.assignment_id),
       attempt.question_pool_reuse_rule, attempt.question_variation_rule
  FROM ple_private.assignment_attempt AS attempt
 WHERE attempt.assignment_attempt_id = current_setting('ple_e2e.m5_saved_attempt_id')::uuid;
INSERT INTO ple_private.question_pool_selection (
    question_pool_selection_id, assignment_attempt_id, assignment_entry_id,
    created_at, selected_question_count, reused_from_question_pool_selection_id
)
SELECT :'m7_scored_pool_selection_id'::uuid, :'m7_scored_attempt_id'::uuid,
       selection.assignment_entry_id, clock_timestamp(),
       selection.selected_question_count, NULL
  FROM ple_private.question_pool_selection AS selection
 WHERE selection.assignment_attempt_id = current_setting('ple_e2e.m5_saved_attempt_id')::uuid;
INSERT INTO ple_private.question_pool_selected_item (
    question_pool_selection_id, question_pool_item_id, selection_position,
    question_id, revision_number
)
SELECT :'m7_scored_pool_selection_id'::uuid, item.question_pool_item_id,
       item.selection_position, item.question_id, item.revision_number
  FROM ple_private.question_pool_selected_item AS item
  JOIN ple_private.question_pool_selection AS selection
    ON selection.question_pool_selection_id = item.question_pool_selection_id
 WHERE selection.assignment_attempt_id = current_setting('ple_e2e.m5_saved_attempt_id')::uuid;
INSERT INTO ple_private.issued_question (
    issued_question_id, assignment_attempt_id, assignment_entry_id, question_id,
    revision_number, issued_position, point_value, scoring_rule,
    question_statistics_eligibility, question_pool_selection_id, question_pool_item_id
)
SELECT gen_random_uuid(), :'m7_scored_attempt_id'::uuid,
       issued.assignment_entry_id, issued.question_id, issued.revision_number,
       issued.issued_position, issued.point_value, issued.scoring_rule,
       issued.question_statistics_eligibility,
       CASE WHEN issued.question_pool_selection_id IS NULL THEN NULL
            ELSE :'m7_scored_pool_selection_id'::uuid END,
       issued.question_pool_item_id
 FROM ple_private.issued_question AS issued
 WHERE issued.assignment_attempt_id = current_setting('ple_e2e.m5_saved_attempt_id')::uuid
 ORDER BY issued.issued_position;
RESET ROLE;
SET ROLE ple_api_owner;
INSERT INTO ple_private.question_attempt (
    question_attempt_id, issued_question_id, question_seed,
    generated_parameter_sha256, issued_at, deadline_at,
    question_attempt_state, reproduction_details
)
SELECT gen_random_uuid(), copied.issued_question_id, original.question_seed,
       original.generated_parameter_sha256, clock_timestamp() - interval '1 second',
       NULL, 'submission_accepted', original.reproduction_details
  FROM ple_private.question_attempt AS original
  JOIN ple_private.issued_question AS original_issued
    ON original_issued.issued_question_id = original.issued_question_id
  JOIN ple_private.issued_question AS copied
    ON copied.assignment_attempt_id = :'m7_scored_attempt_id'::uuid
   AND copied.issued_position = original_issued.issued_position
 WHERE original_issued.assignment_attempt_id = current_setting('ple_e2e.m5_saved_attempt_id')::uuid;
RESET ROLE;
SET ROLE ple_private_owner;
INSERT INTO ple_private.assignment_submission (
    assignment_submission_id, assignment_attempt_id, submitted_at,
    authorized_by_account_id, receipt
)
SELECT gen_random_uuid(), :'m7_scored_attempt_id'::uuid, clock_timestamp(),
       student.student_account_id, jsonb_build_object('submissionBoundary', 'm7-access')
  FROM ple_private.assignment_attempt AS attempt
  JOIN ple_data.student_record AS student ON student.student_record_id = attempt.student_record_id
 WHERE attempt.assignment_attempt_id = :'m7_scored_attempt_id'::uuid;
INSERT INTO ple_private.question_submission (
    submission_id, question_attempt_id, submitted_at, student_response
)
SELECT gen_random_uuid(), copied_attempt.question_attempt_id, clock_timestamp(),
       original_submission.student_response
  FROM ple_private.question_submission AS original_submission
  JOIN ple_private.question_attempt AS original_attempt
    ON original_attempt.question_attempt_id = original_submission.question_attempt_id
  JOIN ple_private.issued_question AS original_issued
    ON original_issued.issued_question_id = original_attempt.issued_question_id
  JOIN ple_private.issued_question AS copied_issued
    ON copied_issued.assignment_attempt_id = :'m7_scored_attempt_id'::uuid
   AND copied_issued.issued_position = original_issued.issued_position
  JOIN ple_private.question_attempt AS copied_attempt
    ON copied_attempt.issued_question_id = copied_issued.issued_question_id
 WHERE original_issued.assignment_attempt_id = current_setting('ple_e2e.m5_saved_attempt_id')::uuid;
INSERT INTO ple_private.job (
    job_id, job_kind, job_target_kind, question_submission_id, generation,
    payload, state, available_at, max_attempts, created_at
)
SELECT gen_random_uuid(), 'grade_accepted_submission', 'question_submission',
       submission.submission_id, 1, '{}'::jsonb, 'ready', clock_timestamp(),
       3, clock_timestamp()
  FROM ple_private.question_submission AS submission
  JOIN ple_private.question_attempt AS question_attempt
    ON question_attempt.question_attempt_id = submission.question_attempt_id
  JOIN ple_private.issued_question AS issued
    ON issued.issued_question_id = question_attempt.issued_question_id
 WHERE issued.assignment_attempt_id = :'m7_scored_attempt_id'::uuid;
INSERT INTO ple_private.question_submission_grading (
    question_submission_grading_id, submission_id, job_id, grading_state, created_at, completed_at
)
SELECT gen_random_uuid(), submission.submission_id, job.job_id, 'graded',
       clock_timestamp(), clock_timestamp()
  FROM ple_private.question_submission AS submission
  JOIN ple_private.job AS job ON job.question_submission_id = submission.submission_id
  JOIN ple_private.question_attempt AS question_attempt
    ON question_attempt.question_attempt_id = submission.question_attempt_id
  JOIN ple_private.issued_question AS issued
    ON issued.issued_question_id = question_attempt.issued_question_id
 WHERE issued.assignment_attempt_id = :'m7_scored_attempt_id'::uuid;
INSERT INTO ple_private.grading_result (
    grading_result_id, submission_id, question_submission_grading_id,
    question_attempt_id, correct, points_earned, points_possible, recorded_at
)
SELECT gen_random_uuid(), submission.submission_id,
       grading.question_submission_grading_id, question_attempt.question_attempt_id,
       false, 0, issued.point_value, clock_timestamp()
  FROM ple_private.question_submission AS submission
  JOIN ple_private.question_submission_grading AS grading
    ON grading.submission_id = submission.submission_id
  JOIN ple_private.question_attempt AS question_attempt
    ON question_attempt.question_attempt_id = submission.question_attempt_id
  JOIN ple_private.issued_question AS issued
    ON issued.issued_question_id = question_attempt.issued_question_id
 WHERE issued.assignment_attempt_id = :'m7_scored_attempt_id'::uuid;
UPDATE ple_private.assignment_attempt
   SET completed_at = clock_timestamp()
 WHERE assignment_attempt_id = '00000000-0000-0000-0000-000000000114'::uuid;
INSERT INTO ple_audit.automated_grading_receipt (
    automated_grading_receipt_id, question_submission_grading_id,
    grading_result_id, committed_at, automated_grading_receipt_checksum
)
SELECT gen_random_uuid(), result.question_submission_grading_id,
       result.grading_result_id, clock_timestamp(), decode(repeat('a', 64), 'hex')
  FROM ple_private.grading_result AS result
  JOIN ple_private.question_attempt AS question_attempt
    ON question_attempt.question_attempt_id = result.question_attempt_id
 JOIN ple_private.issued_question AS issued
    ON issued.issued_question_id = question_attempt.issued_question_id
 WHERE issued.assignment_attempt_id = :'m7_scored_attempt_id'::uuid;
RESET ROLE;
COMMIT;

SELECT course.reference_number AS m7_history_course_reference,
       assignment.reference_number AS m7_history_assignment_reference
  FROM ple_private.assignment_attempt AS attempt
  JOIN ple_data.assignment AS assignment ON assignment.assignment_id = attempt.assignment_id
  JOIN ple_data.course_instance AS course ON course.course_id = assignment.course_id
 WHERE attempt.assignment_attempt_id = current_setting('ple_e2e.m5_saved_attempt_id')::uuid
\gset
SELECT set_config('ple_e2e.m7_history_course_reference', :'m7_history_course_reference', false);
SELECT set_config('ple_e2e.m7_history_assignment_reference', :'m7_history_assignment_reference', false);

BEGIN;
SET LOCAL ROLE ple_auth;
SELECT ple_api.resolve_and_install_session(decode(repeat('ab', 32), 'hex'));
SET LOCAL ROLE ple_app;
DO $$
DECLARE
    v_history jsonb;
BEGIN
    SELECT previous_attempts INTO v_history
      FROM ple_api.live_demo_assignment_access(
          current_setting('ple_e2e.m7_history_course_reference')::bigint,
          current_setting('ple_e2e.m7_history_assignment_reference')::bigint
      );
    IF jsonb_array_length(v_history) < 2
       OR EXISTS (
           SELECT 1
             FROM jsonb_array_elements(v_history) AS attempt(value),
                  LATERAL jsonb_object_keys(attempt.value) AS key(name)
            WHERE key.name !~ '^(assignmentAttempt|attemptNumber|state|score)$'
               OR key.name ~* '(answer|response|correct|feedback|source|uuid|id|key)'
       )
       OR NOT EXISTS (
           SELECT 1
             FROM jsonb_array_elements(v_history) AS attempt(value)
            WHERE attempt.value ? 'score'
              AND attempt.value #>> '{score,pointsEarned}' = '0'
       )
       OR NOT EXISTS (
           SELECT 1
             FROM jsonb_array_elements(v_history) AS attempt(value)
            WHERE NOT attempt.value ? 'score'
       )
       OR EXISTS (
           SELECT 1
             FROM generate_series(0, jsonb_array_length(v_history) - 2) AS index(position)
            WHERE (v_history -> index.position ->> 'attemptNumber')::integer
                < (v_history -> (index.position + 1) ->> 'attemptNumber')::integer
       ) THEN
        RAISE EXCEPTION 'Student access history did not keep newest-first answer-free score availability';
    END IF;
END
$$;
COMMIT;

-- Anonymous, foreign, and revoked-membership callers share the concealed
-- refusal boundary.  The final probe is transactional so the fixture remains
-- reusable by later service assertions.
BEGIN;
SET LOCAL ROLE ple_app;
DO $$
BEGIN
    PERFORM 1 FROM ple_api.live_demo_assignment_access(
        current_setting('ple_e2e.m7_history_course_reference')::bigint,
        current_setting('ple_e2e.m7_history_assignment_reference')::bigint
    );
    RAISE EXCEPTION 'anonymous caller read Student Assignment access';
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
    PERFORM 1 FROM ple_api.live_demo_assignment_access(
        current_setting('ple_e2e.m7_history_course_reference')::bigint,
        current_setting('ple_e2e.m7_history_assignment_reference')::bigint
    );
    RAISE EXCEPTION 'foreign Student read Student Assignment access';
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
    'ended', clock_timestamp(), 'M7 Student Assignment access revocation acceptance'
);
RESET ROLE;
SET ROLE ple_auth;
SELECT ple_api.resolve_and_install_session(decode(repeat('ab', 32), 'hex'));
SET ROLE ple_app;
DO $$
BEGIN
    PERFORM 1 FROM ple_api.live_demo_assignment_access(
        current_setting('ple_e2e.m7_history_course_reference')::bigint,
        current_setting('ple_e2e.m7_history_assignment_reference')::bigint
    );
    RAISE EXCEPTION 'revoked Student membership read Student Assignment access';
EXCEPTION WHEN insufficient_privilege THEN NULL;
END
$$;
ROLLBACK;
