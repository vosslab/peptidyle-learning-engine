-- M5 Student Assignment Attempt context acceptance oracle.
--
-- This proves the public route-context projection remains inside the owned
-- Student membership boundary and exposes only route chrome plus a
-- server-evaluated duration.
\set ON_ERROR_STOP on

DO $$
BEGIN
    IF NOT has_function_privilege(
           'ple_app',
           'ple_api.read_student_assignment_attempt_context(bigint)',
           'EXECUTE'
       )
       OR has_function_privilege(
           'public',
           'ple_api.read_student_assignment_attempt_context(bigint)',
           'EXECUTE'
       )
       OR (SELECT count(*)
             FROM pg_proc AS procedure
             JOIN pg_namespace AS namespace ON namespace.oid = procedure.pronamespace
             JOIN pg_roles AS owner_role ON owner_role.oid = procedure.proowner
            WHERE namespace.nspname = 'ple_api'
              AND procedure.proname = 'read_student_assignment_attempt_context'
              AND owner_role.rolname = 'ple_private_owner'
              AND procedure.prosecdef
              AND procedure.provolatile = 's'
              AND array_to_string(procedure.proconfig, ',') =
                    'search_path=pg_catalog, ple_api, ple_data, ple_private'
              AND position(
                    'pg_catalog.statement_timestamp()' IN procedure.prosrc
                  ) > 0
              AND position(
                    'pg_catalog.clock_timestamp()' IN procedure.prosrc
                  ) = 0) <> 1 THEN
        RAISE EXCEPTION 'Student Assignment Attempt context procedure authority is not exact';
    END IF;
END
$$;

-- Make one current untimed attempt, one current timed attempt, one
-- expired timed attempt, and one old completed Attempt that has no Assignment
-- Submission receipt.  The saved-response oracle immediately before this one
-- supplies the due-boundary Assignment Revision and the submitted receipt.
SELECT gen_random_uuid() AS m5_context_untimed_id \gset
SELECT gen_random_uuid() AS m5_context_timed_id \gset
SELECT gen_random_uuid() AS m5_context_expired_id \gset
SELECT gen_random_uuid() AS m5_context_historical_id \gset
SET ROLE ple_private_owner;
INSERT INTO ple_private.assignment_attempt (
    assignment_attempt_id, student_record_id, assignment_id, assignment_revision_id,
    started_at, completed_at, attempt_number, question_pool_reuse_rule,
    question_variation_rule
)
SELECT :'m5_context_untimed_id'::uuid, student_record_id, assignment_id,
       assignment_revision_id, clock_timestamp(), NULL, attempt_number + 20,
       question_pool_reuse_rule, question_variation_rule
  FROM ple_private.assignment_attempt
 WHERE assignment_attempt_id = '00000000-0000-0000-0000-000000000114'::uuid;

INSERT INTO ple_private.assignment_attempt (
    assignment_attempt_id, student_record_id, assignment_id, assignment_revision_id,
    started_at, completed_at, attempt_number, question_pool_reuse_rule,
    question_variation_rule
)
SELECT :'m5_context_timed_id'::uuid, student_record_id, assignment_id,
       assignment_revision_id, clock_timestamp(), NULL, attempt_number + 20,
       question_pool_reuse_rule, question_variation_rule
  FROM ple_private.assignment_attempt
 WHERE assignment_attempt_id = (
    SELECT attempt.assignment_attempt_id
      FROM ple_private.assignment_attempt AS attempt
      JOIN ple_data.assignment_revision AS revision
        ON revision.assignment_revision_id = attempt.assignment_revision_id
     WHERE revision.assignment_title = 'M4 due-boundary fixture'
     ORDER BY attempt.started_at
     LIMIT 1
 );

INSERT INTO ple_private.assignment_attempt (
    assignment_attempt_id, student_record_id, assignment_id, assignment_revision_id,
    started_at, completed_at, attempt_number, question_pool_reuse_rule,
    question_variation_rule
)
SELECT :'m5_context_expired_id'::uuid, student_record_id, assignment_id,
       assignment_revision_id, clock_timestamp() - interval '601 seconds', NULL,
       attempt_number + 21,
       question_pool_reuse_rule, question_variation_rule
  FROM ple_private.assignment_attempt
 WHERE assignment_attempt_id = (
    SELECT attempt.assignment_attempt_id
      FROM ple_private.assignment_attempt AS attempt
      JOIN ple_data.assignment_revision AS revision
        ON revision.assignment_revision_id = attempt.assignment_revision_id
     WHERE revision.assignment_title = 'M4 due-boundary fixture'
     ORDER BY attempt.started_at
     LIMIT 1
 );

INSERT INTO ple_private.assignment_attempt (
    assignment_attempt_id, student_record_id, assignment_id, assignment_revision_id,
    started_at, completed_at, attempt_number, question_pool_reuse_rule,
    question_variation_rule
)
SELECT :'m5_context_historical_id'::uuid, student_record_id, assignment_id,
       assignment_revision_id, clock_timestamp(), clock_timestamp(), attempt_number + 40,
       question_pool_reuse_rule, question_variation_rule
  FROM ple_private.assignment_attempt
 WHERE assignment_attempt_id = '00000000-0000-0000-0000-000000000114'::uuid;
RESET ROLE;

SELECT reference_number AS m5_context_untimed_reference
  FROM ple_private.assignment_attempt
 WHERE assignment_attempt_id = :'m5_context_untimed_id'::uuid
\gset
SELECT reference_number AS m5_context_timed_reference
  FROM ple_private.assignment_attempt
 WHERE assignment_attempt_id = :'m5_context_timed_id'::uuid
\gset
SELECT reference_number AS m5_context_historical_reference
  FROM ple_private.assignment_attempt
 WHERE assignment_attempt_id = :'m5_context_historical_id'::uuid
\gset
SELECT reference_number AS m5_context_expired_reference
  FROM ple_private.assignment_attempt
 WHERE assignment_attempt_id = :'m5_context_expired_id'::uuid
\gset
SELECT attempt.reference_number AS m5_context_submitted_reference
  FROM ple_private.assignment_attempt AS attempt
  JOIN ple_private.assignment_submission AS submission
    ON submission.assignment_attempt_id = attempt.assignment_attempt_id
 ORDER BY submission.submitted_at DESC
 LIMIT 1
\gset
SELECT set_config(
    'ple_e2e.m5_context_untimed_reference', :'m5_context_untimed_reference', false
);
SELECT set_config(
    'ple_e2e.m5_context_timed_reference', :'m5_context_timed_reference', false
);
SELECT set_config(
    'ple_e2e.m5_context_historical_reference', :'m5_context_historical_reference', false
);
SELECT set_config(
    'ple_e2e.m5_context_expired_reference', :'m5_context_expired_reference', false
);
SELECT set_config(
    'ple_e2e.m5_context_submitted_reference', :'m5_context_submitted_reference', false
);

BEGIN;
SET ROLE ple_auth;
SELECT ple_api.resolve_and_install_session(decode(repeat('ab', 32), 'hex'));
SET ROLE ple_app;
DO $$
BEGIN
    IF (SELECT count(*) FROM ple_api.read_student_assignment_attempt_context(
        current_setting('ple_e2e.m5_context_timed_reference')::bigint
    )) <> 1 OR EXISTS (
        SELECT 1
          FROM ple_api.read_student_assignment_attempt_context(
              current_setting('ple_e2e.m5_context_untimed_reference')::bigint
          ) AS context
         WHERE context.timer_remaining_milliseconds IS NOT NULL
            OR context.attempt_number < 1
            OR context.course_reference_number < 1
            OR context.assignment_reference_number < 1
            OR context.course_title = ''
            OR context.assignment_title = ''
            OR context.course_theme NOT IN (
                'tundra', 'forest', 'desert', 'grass', 'arctic', 'ocean',
                'tropical', 'coral-reef', 'swamp', 'underground', 'salt-marsh',
                'wetland', 'sea-floor', 'magma', 'beach'
            )
    ) OR (SELECT count(*) FROM ple_api.read_student_assignment_attempt_context(
        current_setting('ple_e2e.m5_context_untimed_reference')::bigint
    )) <> 1 THEN
        RAISE EXCEPTION 'untimed Student Assignment Attempt context is invalid';
    END IF;

    IF (SELECT count(*) FROM ple_api.read_student_assignment_attempt_context(
        current_setting('ple_e2e.m5_context_expired_reference')::bigint
    )) <> 1 OR EXISTS (
        SELECT 1
          FROM ple_api.read_student_assignment_attempt_context(
              current_setting('ple_e2e.m5_context_timed_reference')::bigint
          ) AS context
         WHERE context.timer_remaining_milliseconds IS NULL
            OR context.timer_remaining_milliseconds <= 0
            OR context.timer_remaining_milliseconds > 600000
    ) THEN
        RAISE EXCEPTION 'timed Student Assignment Attempt context is invalid';
    END IF;

    IF EXISTS (
        SELECT 1
          FROM ple_api.read_student_assignment_attempt_context(
              current_setting('ple_e2e.m5_context_expired_reference')::bigint
          ) AS context
         WHERE context.timer_remaining_milliseconds <> 0
    ) THEN
        RAISE EXCEPTION 'expired Student Assignment Attempt context did not clamp at zero';
    END IF;

    IF (SELECT count(*) FROM ple_api.read_student_assignment_attempt_context(
        current_setting('ple_e2e.m5_context_submitted_reference')::bigint
    )) <> 1 THEN
        RAISE EXCEPTION 'submitted Student Assignment Attempt lost terminal context';
    END IF;

    IF EXISTS (
        SELECT 1 FROM ple_api.read_student_assignment_attempt_context(
            current_setting('ple_e2e.m5_context_historical_reference')::bigint
        )
    ) OR EXISTS (
        SELECT 1 FROM ple_api.read_student_assignment_attempt_context(0)
    ) THEN
        RAISE EXCEPTION 'Student Assignment Attempt context authorized an unavailable Attempt';
    END IF;
END
$$;
COMMIT;
