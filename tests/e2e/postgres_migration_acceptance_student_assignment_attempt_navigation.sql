-- M5 Student Assignment Attempt navigation acceptance oracle.
--
-- This runs in the disposable PostgreSQL migration runtime after the released
-- Assignment Attempt fixture. It proves the SECURITY DEFINER data boundary;
-- HTTP headers and Rust presentation replay are covered by their focused
-- server tests.
\set ON_ERROR_STOP on

DO $$
BEGIN
    IF (
        SELECT count(*)
          FROM pg_proc AS procedure
          JOIN pg_namespace AS namespace ON namespace.oid = procedure.pronamespace
          JOIN pg_roles AS owner_role ON owner_role.oid = procedure.proowner
         WHERE namespace.nspname = 'ple_api'
           AND procedure.proname IN (
               'read_student_assignment_attempt_progress',
               'read_student_assignment_attempt_position'
           )
           AND owner_role.rolname = 'ple_private_owner'
           AND procedure.prosecdef
           AND array_to_string(procedure.proconfig, ',') =
               'search_path=pg_catalog, ple_api, ple_data, ple_private'
    ) <> 2
       OR NOT has_function_privilege(
           'ple_app',
           'ple_api.read_student_assignment_attempt_progress(bigint)',
           'EXECUTE'
       )
       OR NOT has_function_privilege(
           'ple_app',
           'ple_api.read_student_assignment_attempt_position(bigint,integer)',
           'EXECUTE'
       )
       OR has_function_privilege(
           'public',
           'ple_api.read_student_assignment_attempt_progress(bigint)',
           'EXECUTE'
       )
       OR has_function_privilege(
           'public',
           'ple_api.read_student_assignment_attempt_position(bigint,integer)',
           'EXECUTE'
       ) THEN
        RAISE EXCEPTION 'Student Assignment Attempt navigation procedure authority is not exact';
    END IF;
END
$$;

SELECT reference_number AS m5_attempt_reference
  FROM ple_private.assignment_attempt
 WHERE assignment_attempt_id = '00000000-0000-0000-0000-000000000114'::uuid
\gset
SELECT set_config('ple_e2e.m5_attempt_reference', :'m5_attempt_reference', false);

-- The released-Assignment fixture establishes immutable Issued Questions.
-- Navigation reads their corresponding issued Question Attempts, so build
-- those durable rows before asserting the answer-free progress projection.
SET ROLE ple_api_owner;
INSERT INTO ple_private.question_attempt (
    question_attempt_id, issued_question_id, question_seed,
    generated_parameter_sha256, issued_at, deadline_at,
    question_attempt_state, reproduction_details
)
SELECT gen_random_uuid(), issued.issued_question_id,
       (issued.issued_position + 1)::numeric,
       repeat('a', 64), clock_timestamp(), NULL,
       'open', '{}'::jsonb
  FROM ple_private.issued_question AS issued
 WHERE issued.assignment_attempt_id =
       '00000000-0000-0000-0000-000000000114'::uuid;
RESET ROLE;

-- Selected-position replay resolves the issued Question's immutable source
-- binding.  Supply the smallest PLE source record for this fixture's already
-- published Question so the reader exercises its real source boundary.
SET ROLE ple_private_owner;
INSERT INTO ple_private.object_record (
    object_id, object_address, object_storage_area, object_data_class,
    sha256, size_bytes, media_type, created_at
) VALUES (
    '00000000-0000-0000-0000-000000000150'::uuid,
    jsonb_build_object(
        'kind', 'questionSource',
        'questionRevision', jsonb_build_object(
            'questionId', 'ABC-DEF0', 'revisionNumber', 1
        ),
        'object', '00000000-0000-0000-0000-000000000150'::uuid
    ),
    'private-content', 'question-source', decode(repeat('a', 64), 'hex'),
    17, 'application/json', clock_timestamp()
) ON CONFLICT (object_id) DO NOTHING;
INSERT INTO ple_private.question_revision_source_binding (
    question_id, revision_number, backend, question_format,
    source_object_id, source_object_checksum, created_at
) VALUES (
    'ABC-DEF0', 1, 'ple', 'pleQuestionJson',
    '00000000-0000-0000-0000-000000000150'::uuid,
    repeat('a', 64), clock_timestamp()
) ON CONFLICT (question_id, revision_number) DO NOTHING;
RESET ROLE;

-- Owner reads the complete issued sequence. The public projection contains
-- only the documented answer-free fields and starts as unanswered.
BEGIN;
SET ROLE ple_auth;
SELECT ple_api.resolve_and_install_session(decode(repeat('ab', 32), 'hex'));
SET ROLE ple_app;
DO $$
BEGIN
    IF (SELECT count(*)
          FROM ple_api.read_student_assignment_attempt_progress(
              current_setting('ple_e2e.m5_attempt_reference')::bigint
          )) <> 2
       OR EXISTS (
           SELECT 1
             FROM ple_api.read_student_assignment_attempt_progress(
                 current_setting('ple_e2e.m5_attempt_reference')::bigint
             ) AS progress
            WHERE progress.question_count <> 2
               OR progress.recommended_position <> 1
               OR progress.issued_position NOT IN (1, 2)
               OR progress.response_state <> 'unanswered'
       ) THEN
        RAISE EXCEPTION 'owned Student Assignment Attempt progress is not the exact unanswered sequence';
    END IF;
    IF EXISTS (
        SELECT 1
          FROM ple_api.read_student_assignment_attempt_progress(
              current_setting('ple_e2e.m5_attempt_reference')::bigint
          ) AS progress,
          LATERAL jsonb_object_keys(to_jsonb(progress)) AS key(name)
         WHERE key.name NOT IN (
             'assignment_attempt_reference_number', 'question_count',
             'recommended_position', 'issued_position', 'response_state'
         )
            OR key.name ~* '(uuid|id|nonce|checksum|source|key|answer|score|correct)'
    ) THEN
        RAISE EXCEPTION 'Student Assignment Attempt progress exposed a private or answer-bearing field';
    END IF;
END
$$;
COMMIT;

-- Bind the two issued PLE attempts exactly once so the selected-position
-- reader can be exercised through its real private replay-source procedure.
SET ROLE ple_private_owner;
INSERT INTO ple_private.question_attempt_presentation_binding (
    question_attempt_id, descriptor_version, presentation_nonce, presentation_checksum
)
SELECT question_attempt.question_attempt_id, 1,
       CASE issued.issued_position
           WHEN 0 THEN repeat('1', 32)
           ELSE repeat('2', 32)
       END,
       repeat('a', 64)
  FROM ple_private.question_attempt AS question_attempt
  JOIN ple_private.issued_question AS issued
    ON issued.issued_question_id = question_attempt.issued_question_id
 WHERE issued.assignment_attempt_id = '00000000-0000-0000-0000-000000000114'::uuid;
RESET ROLE;

-- Both bounded, 1-based selected positions resolve only for the owner. The
-- result is server-private replay input; out-of-range positions have no row.
BEGIN;
SET ROLE ple_auth;
SELECT ple_api.resolve_and_install_session(decode(repeat('ab', 32), 'hex'));
SET ROLE ple_app;
DO $$
BEGIN
    IF (SELECT count(*)
          FROM ple_api.read_student_assignment_attempt_position(
              current_setting('ple_e2e.m5_attempt_reference')::bigint, 1
          )) <> 1
       OR (SELECT count(*)
             FROM ple_api.read_student_assignment_attempt_position(
                 current_setting('ple_e2e.m5_attempt_reference')::bigint, 2
             )) <> 1
       OR EXISTS (
           SELECT 1
             FROM ple_api.read_student_assignment_attempt_position(
                 current_setting('ple_e2e.m5_attempt_reference')::bigint, 0
             )
       )
       OR EXISTS (
           SELECT 1
             FROM ple_api.read_student_assignment_attempt_position(
                 current_setting('ple_e2e.m5_attempt_reference')::bigint, 3
             )
       ) THEN
        RAISE EXCEPTION 'Student Assignment Attempt selected-position bounds are not exact';
    END IF;
END
$$;
COMMIT;

-- Navigation state follows the M5 lifecycle: a Student saves each working
-- response, then finalizes the complete Assignment Attempt. Keep the terminal
-- probe transactional so later shared-fixture oracles retain an active Attempt.
BEGIN;
SET ROLE ple_auth;
SELECT ple_api.resolve_and_install_session(decode(repeat('ab', 32), 'hex'));
SET ROLE ple_app;
DO $$
BEGIN
    PERFORM 1 FROM ple_api.save_student_assignment_attempt_response(
        current_setting('ple_e2e.m5_attempt_reference')::bigint,
        1, '{"choice":"first"}'::jsonb
    );
    IF NOT EXISTS (
        SELECT 1
          FROM ple_api.read_student_assignment_attempt_progress(
              current_setting('ple_e2e.m5_attempt_reference')::bigint
          ) AS progress
         WHERE progress.issued_position = 1
           AND progress.response_state = 'saved'
           AND progress.recommended_position = 2
    ) THEN
        RAISE EXCEPTION 'saved Student response did not advance navigation state';
    END IF;

    PERFORM 1 FROM ple_api.save_student_assignment_attempt_response(
        current_setting('ple_e2e.m5_attempt_reference')::bigint,
        2, '{"choice":"second"}'::jsonb
    );
    -- Finalization changes the Attempt lifecycle.  Run it in its own statement
    -- before the STABLE progress and selected-position readers below, so this
    -- oracle observes the terminal database state rather than planner order.
    IF NOT EXISTS (
        SELECT 1 FROM ple_api.finalize_student_assignment_attempt(
            current_setting('ple_e2e.m5_attempt_reference')::bigint
        ) AS result
         WHERE result.submission_state = 'submitted'
           AND result.missing_positions = ARRAY[]::integer[]
    ) THEN
        RAISE EXCEPTION 'Assignment Attempt finalization did not accept both saved responses';
    END IF;

    IF (SELECT count(*)
            FROM ple_api.read_student_assignment_attempt_progress(
                current_setting('ple_e2e.m5_attempt_reference')::bigint
            )
         ) <> 2
      OR (SELECT count(DISTINCT progress.issued_position)
            FROM ple_api.read_student_assignment_attempt_progress(
                current_setting('ple_e2e.m5_attempt_reference')::bigint
            ) AS progress
         ) <> 2
      OR EXISTS (
        SELECT 1
          FROM ple_api.read_student_assignment_attempt_progress(
              current_setting('ple_e2e.m5_attempt_reference')::bigint
          ) AS progress
         WHERE progress.question_count <> 2
            OR progress.recommended_position IS NOT NULL
            OR progress.issued_position NOT IN (1, 2)
            OR progress.response_state <> 'submitted'
    ) OR EXISTS (
        SELECT 1
          FROM ple_api.read_student_assignment_attempt_position(
              current_setting('ple_e2e.m5_attempt_reference')::bigint, 1
          )
    ) OR EXISTS (
        SELECT 1
          FROM ple_api.read_student_assignment_attempt_position(
              current_setting('ple_e2e.m5_attempt_reference')::bigint, 2
          )
    ) THEN
        RAISE EXCEPTION 'Assignment Attempt finalization did not produce terminal navigation state';
    END IF;
END
$$;
ROLLBACK;

-- A foreign Student has a valid session but no result.
SELECT gen_random_uuid() AS m5_foreign_account_id \gset
SELECT gen_random_uuid() AS m5_foreign_session_id \gset
SET ROLE ple_private_owner;
INSERT INTO ple_private.account (account_id, product_role, created_at)
VALUES (:'m5_foreign_account_id'::uuid, 'student', clock_timestamp());
INSERT INTO ple_private.authenticated_session (
    session_id, account_id, product_role, token_hash, created_at, expires_at, revoked_at
) VALUES (
    :'m5_foreign_session_id'::uuid, :'m5_foreign_account_id'::uuid, 'student',
    decode(repeat('cd', 32), 'hex'), clock_timestamp(),
    clock_timestamp() + interval '1 hour', NULL
);
RESET ROLE;
BEGIN;
SET ROLE ple_auth;
SELECT ple_api.resolve_and_install_session(decode(repeat('cd', 32), 'hex'));
SET ROLE ple_app;
DO $$
BEGIN
    IF EXISTS (
        SELECT 1 FROM ple_api.read_student_assignment_attempt_progress(
            current_setting('ple_e2e.m5_attempt_reference')::bigint
        )
    ) OR EXISTS (
        SELECT 1 FROM ple_api.read_student_assignment_attempt_position(
            current_setting('ple_e2e.m5_attempt_reference')::bigint, 1
        )
    ) THEN
        RAISE EXCEPTION 'foreign Student read another Student Assignment Attempt';
    END IF;
END
$$;
COMMIT;

-- Current membership is rechecked on every read. Keep this revocation probe
-- transactional so later shared-fixture oracles retain their active episode.
BEGIN;
SET ROLE ple_api_owner;
INSERT INTO ple_data.course_membership_event (
    course_membership_event_id, membership_id, event_kind, occurred_at, reason
) VALUES (
    gen_random_uuid(), '00000000-0000-0000-0000-000000000108'::uuid,
    'ended', clock_timestamp(), 'M5 revocation authorization acceptance'
);
RESET ROLE;
SET ROLE ple_auth;
SELECT ple_api.resolve_and_install_session(decode(repeat('ab', 32), 'hex'));
SET ROLE ple_app;
DO $$
BEGIN
    IF EXISTS (
        SELECT 1 FROM ple_api.read_student_assignment_attempt_progress(
            current_setting('ple_e2e.m5_attempt_reference')::bigint
        )
    ) OR EXISTS (
        SELECT 1 FROM ple_api.read_student_assignment_attempt_position(
            current_setting('ple_e2e.m5_attempt_reference')::bigint, 1
        )
    ) THEN
        RAISE EXCEPTION 'revoked Student membership retained Assignment Attempt access';
    END IF;
END
$$;
ROLLBACK;

-- An externally completed Attempt without an Assignment submission exposes
-- neither active navigation progress nor a selected position. Keep this
-- terminal-state probe transactional for the shared downstream fixtures.
BEGIN;
SET ROLE ple_private_owner;
UPDATE ple_private.assignment_attempt
   SET completed_at = clock_timestamp()
 WHERE assignment_attempt_id = '00000000-0000-0000-0000-000000000114'::uuid;
RESET ROLE;
BEGIN;
SET ROLE ple_auth;
SELECT ple_api.resolve_and_install_session(decode(repeat('ab', 32), 'hex'));
SET ROLE ple_app;
DO $$
BEGIN
    IF EXISTS (
        SELECT 1 FROM ple_api.read_student_assignment_attempt_progress(
            current_setting('ple_e2e.m5_attempt_reference')::bigint
        )
    ) OR EXISTS (
        SELECT 1 FROM ple_api.read_student_assignment_attempt_position(
            current_setting('ple_e2e.m5_attempt_reference')::bigint, 1
        )
    ) THEN
        RAISE EXCEPTION 'completed Student Assignment Attempt remained readable';
    END IF;
END
$$;
ROLLBACK;
