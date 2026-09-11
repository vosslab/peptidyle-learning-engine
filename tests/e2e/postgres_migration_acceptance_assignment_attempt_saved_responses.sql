-- M5 durable saved-response and whole Assignment Attempt submission oracle.
\set ON_ERROR_STOP on

DO $$
BEGIN
    IF to_regclass('ple_private.assignment_attempt_saved_response') IS NULL
       OR NOT (SELECT relrowsecurity AND relforcerowsecurity
                 FROM pg_class
                WHERE oid = 'ple_private.assignment_attempt_saved_response'::regclass)
       OR has_table_privilege('ple_app', 'ple_private.assignment_attempt_saved_response', 'SELECT')
       OR has_table_privilege('ple_app', 'ple_private.assignment_attempt_saved_response', 'INSERT')
       OR has_function_privilege(
           'public',
           'ple_api.save_student_assignment_attempt_response(bigint,integer,jsonb)',
           'EXECUTE'
       )
       OR has_function_privilege(
           'public', 'ple_api.finalize_student_assignment_attempt(bigint)', 'EXECUTE'
       )
       OR has_function_privilege(
           'public',
           'ple_api.read_student_assignment_attempt_saved_response(bigint,integer)',
           'EXECUTE'
       )
       OR NOT has_function_privilege(
           'ple_app',
           'ple_api.save_student_assignment_attempt_response(bigint,integer,jsonb)',
           'EXECUTE'
       )
       OR NOT has_function_privilege(
           'ple_app', 'ple_api.finalize_student_assignment_attempt(bigint)', 'EXECUTE'
       )
       OR NOT has_function_privilege(
           'ple_app',
           'ple_api.read_student_assignment_attempt_saved_response(bigint,integer)',
           'EXECUTE'
       ) THEN
        RAISE EXCEPTION 'saved-response storage or procedure authority is not exact';
    END IF;
END
$$;

-- The earlier navigation oracle completed its fixture attempt.  Build a new
-- two-question owned attempt from the same released issue evidence.
SELECT gen_random_uuid() AS m5_saved_attempt_id \gset
SELECT gen_random_uuid() AS m5_saved_pool_selection_id \gset
SET ROLE ple_private_owner;
INSERT INTO ple_private.assignment_attempt (
    assignment_attempt_id, student_record_id, assignment_id, assignment_revision_id,
    started_at, completed_at, attempt_number, question_pool_reuse_rule,
    question_variation_rule
)
SELECT :'m5_saved_attempt_id'::uuid, student_record_id, assignment_id,
       assignment_revision_id, clock_timestamp(), NULL, attempt_number + 1,
       question_pool_reuse_rule, question_variation_rule
  FROM ple_private.assignment_attempt
 WHERE assignment_attempt_id = '00000000-0000-0000-0000-000000000114'::uuid;

BEGIN;
INSERT INTO ple_private.question_pool_selection (
    question_pool_selection_id, assignment_attempt_id, assignment_entry_id,
    created_at, selected_question_count, reused_from_question_pool_selection_id
)
SELECT :'m5_saved_pool_selection_id'::uuid, :'m5_saved_attempt_id'::uuid,
       selection.assignment_entry_id, clock_timestamp(),
       selection.selected_question_count, NULL
  FROM ple_private.question_pool_selection AS selection
 WHERE selection.assignment_attempt_id =
       '00000000-0000-0000-0000-000000000114'::uuid;

INSERT INTO ple_private.question_pool_selected_item (
    question_pool_selection_id, question_pool_item_id, selection_position,
    question_id, revision_number
)
SELECT :'m5_saved_pool_selection_id'::uuid, item.question_pool_item_id,
       item.selection_position, item.question_id, item.revision_number
  FROM ple_private.question_pool_selected_item AS item
  JOIN ple_private.question_pool_selection AS selection
    ON selection.question_pool_selection_id = item.question_pool_selection_id
 WHERE selection.assignment_attempt_id =
       '00000000-0000-0000-0000-000000000114'::uuid;

INSERT INTO ple_private.issued_question (
    issued_question_id, assignment_attempt_id, assignment_entry_id, question_id,
    revision_number, issued_position, point_value, scoring_rule,
    question_statistics_eligibility, question_pool_selection_id,
    question_pool_item_id
)
SELECT gen_random_uuid(), :'m5_saved_attempt_id'::uuid, assignment_entry_id,
       question_id, revision_number, issued_position, point_value, scoring_rule,
       question_statistics_eligibility,
       CASE WHEN question_pool_selection_id IS NULL THEN NULL
            ELSE :'m5_saved_pool_selection_id'::uuid END,
       question_pool_item_id
  FROM ple_private.issued_question
 WHERE assignment_attempt_id = '00000000-0000-0000-0000-000000000114'::uuid
 ORDER BY issued_position;
COMMIT;

RESET ROLE;
SET ROLE ple_api_owner;
INSERT INTO ple_private.question_attempt (
    question_attempt_id, issued_question_id, question_seed,
    generated_parameter_sha256, issued_at, deadline_at,
    question_attempt_state, reproduction_details
)
SELECT gen_random_uuid(), issued.issued_question_id, issued.issued_position + 10,
       repeat('b', 64), clock_timestamp(), NULL, 'open', '{}'::jsonb
  FROM ple_private.issued_question AS issued
 WHERE issued.assignment_attempt_id = :'m5_saved_attempt_id'::uuid
 ORDER BY issued.issued_position;
RESET ROLE;
SET ROLE ple_private_owner;

SELECT reference_number AS m5_saved_attempt_reference
  FROM ple_private.assignment_attempt
 WHERE assignment_attempt_id = :'m5_saved_attempt_id'::uuid
\gset
RESET ROLE;

-- An Assignment Attempt derives its time boundary from the immutable released
-- Assignment Revision it already pins.  A null limit above remains usable;
-- this separately pinned one-second revision rejects both save and finalize
-- after that boundary without creating any Student-work evidence.
SELECT gen_random_uuid() AS m5_timed_assignment_id \gset
SELECT gen_random_uuid() AS m5_timed_revision_id \gset
SELECT gen_random_uuid() AS m5_timed_attempt_id \gset
SELECT gen_random_uuid() AS m5_timed_issued_question_id \gset
SELECT gen_random_uuid() AS m5_timed_question_attempt_id \gset

INSERT INTO ple_data.assignment (
    assignment_id, course_id, source_blueprint_course_reference_number,
    source_blueprint_revision_number, created_at, updated_at,
    assignment_edit_number, assignment_title, assignment_instructions,
    available_at, due_at, closes_at, assignment_attempt_time_limit_seconds,
    attempt_limit, late_work_rule, assignment_deadline_rule,
    assignment_completion_rule, assignment_completion_score_threshold,
    assignment_attempt_grade_rule, assignment_attempt_continuation_rule,
    max_additional_assignment_attempts, question_pool_reuse_rule,
    question_variation_rule, assignment_attempt_resume_rule,
    assignment_question_display_rule, assignment_navigation_rule,
    assignment_question_order_rule, assignment_status, released_assignment_revision_id
)
SELECT :'m5_timed_assignment_id'::uuid, assignment.course_id,
       assignment.source_blueprint_course_reference_number,
       assignment.source_blueprint_revision_number, clock_timestamp(), clock_timestamp(),
       1, 'M5 expired Attempt fixture', '', NULL, NULL, NULL, 1, NULL,
       'accept', 'auto_submit', 'answer_all', NULL, 'highest', 'unlimited',
       NULL, 'reuse_selection', 'new_variation', 'resumable', 'all_questions',
       'free_navigation', 'authored_order', 'unreleased', NULL
  FROM ple_data.assignment AS assignment
 WHERE assignment.assignment_id = '00000000-0000-0000-0000-000000000110'::uuid;

INSERT INTO ple_data.assignment_revision (
    assignment_revision_id, assignment_id, course_id, course_schedule_revision_id,
    revision_number, assignment_title, assignment_instructions, available_at,
    due_at, closes_at, assignment_attempt_time_limit_seconds, attempt_limit,
    late_work_rule, assignment_deadline_rule, assignment_completion_rule,
    assignment_completion_score_threshold, assignment_attempt_grade_rule,
    assignment_attempt_continuation_rule, max_additional_assignment_attempts,
    question_pool_reuse_rule, question_variation_rule, assignment_attempt_resume_rule,
    assignment_question_display_rule, assignment_navigation_rule,
    assignment_question_order_rule, created_at
)
SELECT :'m5_timed_revision_id'::uuid, :'m5_timed_assignment_id'::uuid,
       revision.course_id, revision.course_schedule_revision_id, 1,
       'M5 expired Attempt fixture', '', NULL, NULL, NULL, 1, NULL,
       'accept', 'auto_submit', 'answer_all', NULL, 'highest', 'unlimited',
       NULL, 'reuse_selection', 'new_variation', 'resumable', 'all_questions',
       'free_navigation', 'authored_order', clock_timestamp()
  FROM ple_data.assignment_revision AS revision
 WHERE revision.assignment_revision_id = '00000000-0000-0000-0000-000000000111'::uuid;

UPDATE ple_data.assignment
   SET assignment_status = 'released',
       released_assignment_revision_id = :'m5_timed_revision_id'::uuid,
       updated_at = clock_timestamp()
 WHERE assignment_id = :'m5_timed_assignment_id'::uuid;

-- The timed fixture pins the same fixed Question through an Entry belonging
-- to its own released Revision.  Keep the immutable issue evidence exact.
BEGIN;
SET ROLE ple_data_owner;
INSERT INTO ple_data.assignment_revision_entry (
    assignment_revision_id, assignment_entry_id, assignment_content_entry_index,
    entry_kind, availability, scoring_rule, point_value, question_attempt_limit,
    question_attempt_time_limit_seconds, question_attempt_time_limit_grace_seconds
)
SELECT :'m5_timed_revision_id'::uuid, entry.assignment_entry_id,
       entry.assignment_content_entry_index, entry.entry_kind, entry.availability,
       entry.scoring_rule, entry.point_value, entry.question_attempt_limit,
       entry.question_attempt_time_limit_seconds,
       entry.question_attempt_time_limit_grace_seconds
  FROM ple_data.assignment_revision_entry AS entry
 WHERE entry.assignment_revision_id =
       '00000000-0000-0000-0000-000000000111'::uuid
   AND entry.assignment_entry_id =
       '00000000-0000-0000-0000-000000000112'::uuid;

INSERT INTO ple_data.assignment_revision_fixed_question (
    assignment_revision_id, assignment_entry_id, question_id, revision_number
)
SELECT :'m5_timed_revision_id'::uuid, fixed_question.assignment_entry_id,
       fixed_question.question_id, fixed_question.revision_number
  FROM ple_data.assignment_revision_fixed_question AS fixed_question
 WHERE fixed_question.assignment_revision_id =
       '00000000-0000-0000-0000-000000000111'::uuid
   AND fixed_question.assignment_entry_id =
       '00000000-0000-0000-0000-000000000112'::uuid;
COMMIT;
RESET ROLE;

SET ROLE ple_private_owner;
INSERT INTO ple_private.assignment_attempt (
    assignment_attempt_id, student_record_id, assignment_id, assignment_revision_id,
    started_at, completed_at, attempt_number, question_pool_reuse_rule,
    question_variation_rule
)
SELECT :'m5_timed_attempt_id'::uuid, student_record_id,
       :'m5_timed_assignment_id'::uuid, :'m5_timed_revision_id'::uuid,
       clock_timestamp() - interval '2 seconds', NULL, attempt_number + 2,
       question_pool_reuse_rule, question_variation_rule
  FROM ple_private.assignment_attempt
 WHERE assignment_attempt_id = '00000000-0000-0000-0000-000000000114'::uuid;

INSERT INTO ple_private.issued_question (
    issued_question_id, assignment_attempt_id, assignment_entry_id, question_id,
    revision_number, issued_position, point_value, scoring_rule,
    question_statistics_eligibility
)
SELECT :'m5_timed_issued_question_id'::uuid, :'m5_timed_attempt_id'::uuid,
       assignment_entry_id, question_id, revision_number, 0, point_value,
       scoring_rule, question_statistics_eligibility
  FROM ple_private.issued_question
 WHERE assignment_attempt_id = '00000000-0000-0000-0000-000000000114'::uuid
 ORDER BY issued_position
 LIMIT 1;

RESET ROLE;
SET ROLE ple_api_owner;
INSERT INTO ple_private.question_attempt (
    question_attempt_id, issued_question_id, question_seed,
    generated_parameter_sha256, issued_at, deadline_at,
    question_attempt_state, reproduction_details
) VALUES (
    :'m5_timed_question_attempt_id'::uuid, :'m5_timed_issued_question_id'::uuid,
    99, repeat('c', 64), clock_timestamp() - interval '2 seconds', NULL,
    'open', '{}'::jsonb
);
RESET ROLE;
SET ROLE ple_private_owner;
SELECT reference_number AS m5_timed_attempt_reference
  FROM ple_private.assignment_attempt
 WHERE assignment_attempt_id = :'m5_timed_attempt_id'::uuid
\gset
RESET ROLE;
SELECT set_config(
    'ple_e2e.m5_timed_attempt_reference', :'m5_timed_attempt_reference', false
);
SELECT set_config('ple_e2e.m5_timed_attempt_id', :'m5_timed_attempt_id', false);
SELECT set_config(
    'ple_e2e.m5_timed_question_attempt_id', :'m5_timed_question_attempt_id', false
);

BEGIN;
SET ROLE ple_auth;
SELECT ple_api.resolve_and_install_session(decode(repeat('ab', 32), 'hex'));
SET ROLE ple_app;
DO $$
BEGIN
    PERFORM 1 FROM ple_api.save_student_assignment_attempt_response(
        current_setting('ple_e2e.m5_timed_attempt_reference')::bigint,
        1, '{}'::jsonb
    );
    RAISE EXCEPTION 'expired Assignment Attempt accepted a saved response';
EXCEPTION WHEN object_not_in_prerequisite_state THEN NULL;
END
$$;
DO $$
BEGIN
    PERFORM 1 FROM ple_api.finalize_student_assignment_attempt(
        current_setting('ple_e2e.m5_timed_attempt_reference')::bigint
    );
    RAISE EXCEPTION 'expired Assignment Attempt finalized';
EXCEPTION WHEN object_not_in_prerequisite_state THEN NULL;
END
$$;
COMMIT;

SET ROLE ple_private_owner;
DO $$
BEGIN
    IF EXISTS (
        SELECT 1 FROM ple_private.assignment_attempt_saved_response AS saved_response
         WHERE saved_response.question_attempt_id =
               current_setting('ple_e2e.m5_timed_question_attempt_id', true)::uuid
    ) OR EXISTS (
        SELECT 1 FROM ple_private.assignment_submission AS submission
         WHERE submission.assignment_attempt_id =
               current_setting('ple_e2e.m5_timed_attempt_id')::uuid
    ) THEN
        RAISE EXCEPTION 'expired Assignment Attempt wrote Student-work evidence';
    END IF;
END
$$;
RESET ROLE;
SELECT set_config(
    'ple_e2e.m5_saved_attempt_reference', :'m5_saved_attempt_reference', false
);
SELECT set_config('ple_e2e.m5_saved_attempt_id', :'m5_saved_attempt_id', false);

-- Browser capability roles use procedures only; direct private-table access
-- remains unavailable even after an authenticated Student session is installed.
BEGIN;
SET ROLE ple_auth;
SELECT ple_api.resolve_and_install_session(decode(repeat('ab', 32), 'hex'));
SET ROLE ple_app;
DO $$
BEGIN
    PERFORM 1 FROM ple_private.assignment_attempt_saved_response;
    RAISE EXCEPTION 'Student browser role read saved responses directly';
EXCEPTION WHEN insufficient_privilege THEN NULL;
END
$$;
COMMIT;

-- First save creates private mutable state.  An identical retry preserves its
-- timestamp, while a changed response replaces only the current saved value.
BEGIN;
SET ROLE ple_auth;
SELECT ple_api.resolve_and_install_session(decode(repeat('ab', 32), 'hex'));
SET ROLE ple_app;
DO $$
DECLARE
    v_first_saved_at timestamp with time zone;
    v_retry_saved_at timestamp with time zone;
BEGIN
    SELECT saved_at INTO v_first_saved_at
      FROM ple_api.save_student_assignment_attempt_response(
          current_setting('ple_e2e.m5_saved_attempt_reference')::bigint,
          1, '{"choice":"first"}'::jsonb
      );
    SELECT saved_at INTO v_retry_saved_at
      FROM ple_api.save_student_assignment_attempt_response(
          current_setting('ple_e2e.m5_saved_attempt_reference')::bigint,
          1, '{"choice":"first"}'::jsonb
      );
    IF v_first_saved_at IS NULL OR v_retry_saved_at IS DISTINCT FROM v_first_saved_at THEN
        RAISE EXCEPTION 'identical saved-response retry changed saved_at';
    END IF;
    PERFORM 1 FROM ple_api.save_student_assignment_attempt_response(
        current_setting('ple_e2e.m5_saved_attempt_reference')::bigint,
        1, '{"choice":"changed"}'::jsonb
    );
    IF NOT EXISTS (
        SELECT 1
          FROM ple_api.read_student_assignment_attempt_saved_response(
              current_setting('ple_e2e.m5_saved_attempt_reference')::bigint, 1
          ) AS saved_response
         WHERE saved_response.issued_position = 1
           AND saved_response.student_response = '{"choice":"changed"}'::jsonb
           AND saved_response.saved_at IS NOT NULL
    ) THEN
        RAISE EXCEPTION 'Student reload did not restore the current saved response';
    END IF;
END
$$;
COMMIT;

SET ROLE ple_private_owner;
DO $$
BEGIN
    IF (SELECT student_response
          FROM ple_private.assignment_attempt_saved_response AS saved_response
          JOIN ple_private.question_attempt AS question_attempt
            ON question_attempt.question_attempt_id = saved_response.question_attempt_id
          JOIN ple_private.issued_question AS issued
            ON issued.issued_question_id = question_attempt.issued_question_id
          JOIN ple_private.assignment_attempt AS attempt
            ON attempt.assignment_attempt_id = issued.assignment_attempt_id
         WHERE attempt.reference_number =
               current_setting('ple_e2e.m5_saved_attempt_reference')::bigint
           AND issued.issued_position = 0)
       IS DISTINCT FROM '{"choice":"changed"}'::jsonb THEN
        RAISE EXCEPTION 'changed saved response did not replace current working state';
    END IF;
END
$$;
RESET ROLE;

-- A foreign Student has a valid Student session but neither save nor finalize
-- authority for this Attempt.
SELECT gen_random_uuid() AS m5_saved_foreign_account_id \gset
SELECT gen_random_uuid() AS m5_saved_foreign_session_id \gset
SET ROLE ple_private_owner;
INSERT INTO ple_private.account (account_id, product_role, created_at)
VALUES (:'m5_saved_foreign_account_id'::uuid, 'student', clock_timestamp());
INSERT INTO ple_private.authenticated_session (
    session_id, account_id, product_role, token_hash, created_at, expires_at,
    revoked_at
) VALUES (
    :'m5_saved_foreign_session_id'::uuid, :'m5_saved_foreign_account_id'::uuid,
    'student', decode(repeat('fa', 32), 'hex'), clock_timestamp(),
    clock_timestamp() + interval '1 hour', NULL
);
RESET ROLE;
BEGIN;
SET ROLE ple_auth;
SELECT ple_api.resolve_and_install_session(decode(repeat('fa', 32), 'hex'));
SET ROLE ple_app;
DO $$
BEGIN
    PERFORM 1 FROM ple_api.save_student_assignment_attempt_response(
        current_setting('ple_e2e.m5_saved_attempt_reference')::bigint,
        1, '{}'::jsonb
    );
    RAISE EXCEPTION 'foreign Student saved another Student response';
EXCEPTION WHEN insufficient_privilege THEN NULL;
END
$$;
DO $$
BEGIN
    PERFORM 1 FROM ple_api.finalize_student_assignment_attempt(
        current_setting('ple_e2e.m5_saved_attempt_reference')::bigint
    );
    RAISE EXCEPTION 'foreign Student finalized another Student attempt';
EXCEPTION WHEN insufficient_privilege THEN NULL;
END
$$;
DO $$
BEGIN
    IF EXISTS (
        SELECT 1
          FROM ple_api.read_student_assignment_attempt_saved_response(
              current_setting('ple_e2e.m5_saved_attempt_reference')::bigint, 1
          )
    ) THEN
        RAISE EXCEPTION 'foreign Student read another Student saved response';
    END IF;
END
$$;
COMMIT;

-- Missing saved positions return an answer-free conflict result and leave no
-- immutable submission, grading, or Job records behind.
BEGIN;
SET ROLE ple_auth;
SELECT ple_api.resolve_and_install_session(decode(repeat('ab', 32), 'hex'));
SET ROLE ple_app;
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM ple_api.finalize_student_assignment_attempt(
            current_setting('ple_e2e.m5_saved_attempt_reference')::bigint
        ) AS result
         WHERE result.submission_state = 'missing_responses'
           AND result.missing_positions = ARRAY[2]
    ) THEN
        RAISE EXCEPTION 'missing saved response did not return its bounded position';
    END IF;
END
$$;
COMMIT;

SET ROLE ple_private_owner;
DO $$
BEGIN
    IF EXISTS (
        SELECT 1
          FROM ple_private.assignment_submission AS submission
         WHERE submission.assignment_attempt_id =
               current_setting('ple_e2e.m5_saved_attempt_id')::uuid
    ) OR EXISTS (
        SELECT 1
          FROM ple_private.question_submission AS submission
          JOIN ple_private.question_attempt AS question_attempt
            ON question_attempt.question_attempt_id = submission.question_attempt_id
          JOIN ple_private.issued_question AS issued
            ON issued.issued_question_id = question_attempt.issued_question_id
         WHERE issued.assignment_attempt_id =
               current_setting('ple_e2e.m5_saved_attempt_id')::uuid
    ) THEN
        RAISE EXCEPTION 'missing-save finalization wrote immutable evidence';
    END IF;
END
$$;
RESET ROLE;

-- Saving the remaining Question finalizes exactly once.  The public progress
-- projection remains answer-free and exposes only submitted position state.
BEGIN;
SET ROLE ple_auth;
SELECT ple_api.resolve_and_install_session(decode(repeat('ab', 32), 'hex'));
SET ROLE ple_app;
SELECT 1 FROM ple_api.save_student_assignment_attempt_response(
    current_setting('ple_e2e.m5_saved_attempt_reference')::bigint,
    2, '{"choice":"second"}'::jsonb
);
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM ple_api.finalize_student_assignment_attempt(
            current_setting('ple_e2e.m5_saved_attempt_reference')::bigint
        ) AS result
         WHERE result.submission_state = 'submitted'
           AND result.missing_positions = ARRAY[]::integer[]
    ) THEN
        RAISE EXCEPTION 'Assignment Attempt finalization did not succeed';
    END IF;
    IF (SELECT count(*)
          FROM ple_api.read_student_assignment_attempt_progress(
              current_setting('ple_e2e.m5_saved_attempt_reference')::bigint
          ) AS progress
         WHERE progress.response_state = 'submitted') <> 2
       OR EXISTS (
           SELECT 1
             FROM ple_api.read_student_assignment_attempt_progress(
                 current_setting('ple_e2e.m5_saved_attempt_reference')::bigint
             ) AS progress,
             LATERAL jsonb_object_keys(to_jsonb(progress)) AS key(name)
            WHERE key.name ~* '(uuid|id|nonce|checksum|source|key|answer|score|correct)'
       ) THEN
        RAISE EXCEPTION 'finalized Student progress is not answer-free submitted state';
    END IF;
    IF EXISTS (
        SELECT 1 FROM ple_api.read_student_assignment_attempt_position(
            current_setting('ple_e2e.m5_saved_attempt_reference')::bigint, 1
        )
    ) THEN
        RAISE EXCEPTION 'finalized Attempt retained active selected-position delivery';
    END IF;
    IF EXISTS (
        SELECT 1
          FROM ple_api.read_student_assignment_attempt_saved_response(
              current_setting('ple_e2e.m5_saved_attempt_reference')::bigint, 1
          )
    ) THEN
        RAISE EXCEPTION 'finalized Attempt retained saved working-response delivery';
    END IF;
    IF NOT EXISTS (
        SELECT 1 FROM ple_api.finalize_student_assignment_attempt(
            current_setting('ple_e2e.m5_saved_attempt_reference')::bigint
        ) AS result
         WHERE result.submission_state = 'submitted'
           AND result.missing_positions = ARRAY[]::integer[]
    ) THEN
        RAISE EXCEPTION 'repeat Assignment Attempt finalization was not idempotent';
    END IF;
END
$$;
DO $$
BEGIN
    PERFORM 1 FROM ple_api.save_student_assignment_attempt_response(
        current_setting('ple_e2e.m5_saved_attempt_reference')::bigint,
        1, '{}'::jsonb
    );
    RAISE EXCEPTION 'saved response remained writable after finalization';
EXCEPTION WHEN insufficient_privilege OR object_not_in_prerequisite_state THEN NULL;
END
$$;
COMMIT;

SET ROLE ple_private_owner;
DO $$
BEGIN
    IF (SELECT count(*) FROM ple_private.assignment_submission
         WHERE assignment_attempt_id = current_setting('ple_e2e.m5_saved_attempt_id')::uuid) <> 1
       OR (SELECT count(*)
             FROM ple_private.question_submission AS submission
             JOIN ple_private.question_attempt AS question_attempt
               ON question_attempt.question_attempt_id = submission.question_attempt_id
             JOIN ple_private.issued_question AS issued
               ON issued.issued_question_id = question_attempt.issued_question_id
            WHERE issued.assignment_attempt_id = current_setting('ple_e2e.m5_saved_attempt_id')::uuid) <> 2
       OR (SELECT count(*)
             FROM ple_private.question_submission_grading AS grading
             JOIN ple_private.question_submission AS submission
               ON submission.submission_id = grading.submission_id
             JOIN ple_private.question_attempt AS question_attempt
               ON question_attempt.question_attempt_id = submission.question_attempt_id
             JOIN ple_private.issued_question AS issued
               ON issued.issued_question_id = question_attempt.issued_question_id
            WHERE issued.assignment_attempt_id = current_setting('ple_e2e.m5_saved_attempt_id')::uuid
              AND grading.grading_state = 'pending') <> 2
       OR (SELECT count(*)
             FROM ple_private.job AS job
             JOIN ple_private.question_submission AS submission
               ON submission.submission_id = job.question_submission_id
             JOIN ple_private.question_attempt AS question_attempt
               ON question_attempt.question_attempt_id = submission.question_attempt_id
             JOIN ple_private.issued_question AS issued
               ON issued.issued_question_id = question_attempt.issued_question_id
            WHERE issued.assignment_attempt_id = current_setting('ple_e2e.m5_saved_attempt_id')::uuid
              AND job.job_kind = 'grade_accepted_submission'
              AND job.state = 'ready') <> 2
       OR NOT EXISTS (
            SELECT 1 FROM ple_private.assignment_attempt
             WHERE assignment_attempt_id = current_setting('ple_e2e.m5_saved_attempt_id')::uuid
               AND completed_at IS NOT NULL
       ) THEN
        RAISE EXCEPTION 'whole-attempt finalization did not create exact immutable grading work';
    END IF;
END
$$;
RESET ROLE;
