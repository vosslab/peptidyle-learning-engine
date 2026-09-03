-- Disposable authenticated inputs for the iMathAS Question Backend Session Store oracle.
-- This extends the existing Assignment Attempt fixture with its exact immutable
-- Question Source and one production-shaped API login.

BEGIN;
-- The PostgreSQL Migration Acceptance Runtime bootstrap authority creates this
-- disposable fixture.
-- The assertions below then enter the production-shaped restricted API path.
INSERT INTO ple_private.object_record (
    object_id, object_address, object_storage_area, object_data_class, sha256,
    size_bytes, media_type, created_at
) VALUES (
    '00000000-0000-0000-0000-00000000f202',
    jsonb_build_object(
        'kind', 'questionSource',
        'questionRevision', jsonb_build_object('questionId', 'ABC-DEF0', 'revisionNumber', 1),
        'object', '00000000-0000-0000-0000-00000000f202'::uuid
    ),
    'private-content', 'question-source', decode(repeat('aa', 32), 'hex'),
    17, 'application/json', pg_catalog.clock_timestamp()
);
INSERT INTO ple_private.question_source (
    question_source_uuid, question_id, revision_number, backend, question_format,
    question_type, webwork_pg_path, qti_package_item_identifier,
    imathas_deployment_reference, imathas_item_reference, imathas_profile,
    source_object_id, source_object_checksum,
    public_content_checksum, created_at, updated_at
) VALUES (
    '00000000-0000-0000-0000-00000000f201', 'ABC-DEF0', 1, 'ple',
    'pleQuestionJson', 'multipleChoice', NULL, NULL, NULL, NULL, NULL,
    '00000000-0000-0000-0000-00000000f202', repeat('aa', 32), repeat('cc', 32),
    pg_catalog.clock_timestamp(), pg_catalog.clock_timestamp()
);
INSERT INTO ple_private.question_attempt (
    question_attempt_id, issued_question_id, question_seed, generated_parameter_sha256,
    issued_at, deadline_at, question_attempt_state, reproduction_details
) VALUES (
    '00000000-0000-0000-0000-00000000f205',
    '00000000-0000-5000-8000-000000000115', 1, repeat('ab', 32),
    pg_catalog.clock_timestamp(), NULL, 'open', '{}'::jsonb
), (
    '00000000-0000-0000-0000-00000000f207',
    '00000000-0000-5000-8000-000000000115', 1, repeat('ac', 32),
    pg_catalog.clock_timestamp(), NULL, 'open', '{}'::jsonb
);
-- This second issued Question preserves the same active Question identity while
-- freezing a deliberately ineligible statistics decision for the Store oracle.
INSERT INTO ple_private.assignment_attempt (
    assignment_attempt_id, student_record_id, assignment_id, assignment_revision_id,
    started_at, completed_at, attempt_number, question_pool_reuse_rule, question_variation_rule
)
SELECT
    '00000000-0000-0000-0000-00000000f210', student_record_id, assignment_id,
    assignment_revision_id, pg_catalog.clock_timestamp(), NULL, attempt_number + 100,
    question_pool_reuse_rule, question_variation_rule
FROM ple_private.assignment_attempt
WHERE assignment_attempt_id = '00000000-0000-0000-0000-000000000114';
INSERT INTO ple_private.issued_question (
    issued_question_id, assignment_attempt_id, assignment_entry_id, question_id,
    revision_number, issued_position, point_value, scoring_rule, statistics_eligible,
    question_pool_selection_id, question_pool_item_id
)
SELECT
    '00000000-0000-0000-0000-00000000f209',
    '00000000-0000-0000-0000-00000000f210', assignment_entry_id, question_id,
    revision_number, issued_position, point_value, scoring_rule, false,
    question_pool_selection_id, question_pool_item_id
FROM ple_private.issued_question
WHERE issued_question_id = '00000000-0000-5000-8000-000000000115';
INSERT INTO ple_private.question_attempt (
    question_attempt_id, issued_question_id, question_seed, generated_parameter_sha256,
    issued_at, deadline_at, question_attempt_state, reproduction_details
) VALUES (
    '00000000-0000-0000-0000-00000000f208',
    '00000000-0000-0000-0000-00000000f209', 1, repeat('ad', 32),
    pg_catalog.clock_timestamp(), NULL, 'open', '{}'::jsonb
);
-- A separate eligible Question Attempt keeps the statistics scenario independent
-- from the existing lifecycle and persistence scenarios.
INSERT INTO ple_private.assignment_attempt (
    assignment_attempt_id, student_record_id, assignment_id, assignment_revision_id,
    started_at, completed_at, attempt_number, question_pool_reuse_rule, question_variation_rule
)
SELECT
    '00000000-0000-0000-0000-00000000f212', student_record_id, assignment_id,
    assignment_revision_id, pg_catalog.clock_timestamp(), NULL, attempt_number + 200,
    question_pool_reuse_rule, question_variation_rule
FROM ple_private.assignment_attempt
WHERE assignment_attempt_id = '00000000-0000-0000-0000-000000000114';
INSERT INTO ple_private.issued_question (
    issued_question_id, assignment_attempt_id, assignment_entry_id, question_id,
    revision_number, issued_position, point_value, scoring_rule, statistics_eligible,
    question_pool_selection_id, question_pool_item_id
)
SELECT
    '00000000-0000-0000-0000-00000000f213',
    '00000000-0000-0000-0000-00000000f212', assignment_entry_id, question_id,
    revision_number, issued_position, point_value, scoring_rule, true,
    question_pool_selection_id, question_pool_item_id
FROM ple_private.issued_question
WHERE issued_question_id = '00000000-0000-5000-8000-000000000115';
INSERT INTO ple_private.question_attempt (
    question_attempt_id, issued_question_id, question_seed, generated_parameter_sha256,
    issued_at, deadline_at, question_attempt_state, reproduction_details
) VALUES (
    '00000000-0000-0000-0000-00000000f214',
    '00000000-0000-0000-0000-00000000f213', 1, repeat('ae', 32),
    pg_catalog.clock_timestamp(), NULL, 'open', '{}'::jsonb
);
INSERT INTO ple_private.authenticated_session (
    session_id, account_id, role, token_hash, created_at, expires_at
) VALUES (
    '00000000-0000-0000-0000-00000000f203',
    '00000000-0000-0000-0000-000000000101', 'student',
    decode('425ed4e4a36b30ea21b90e21c712c649e8214c29b7eaf68089d1039c6e55384c', 'hex'), pg_catalog.clock_timestamp(),
    pg_catalog.clock_timestamp() + interval '1 hour'
);
COMMIT;

CREATE ROLE ple_api_login LOGIN NOINHERIT NOSUPERUSER NOCREATEDB NOCREATEROLE
    NOREPLICATION NOBYPASSRLS CONNECTION LIMIT 8 PASSWORD 'imathasquestionbackendoracle';
CREATE ROLE ple_worker_login LOGIN NOINHERIT NOSUPERUSER NOCREATEDB NOCREATEROLE
    NOREPLICATION NOBYPASSRLS CONNECTION LIMIT 8 PASSWORD 'imathasquestionbackendworkeroracle';
GRANT ple_app TO ple_api_login WITH INHERIT FALSE, SET TRUE, ADMIN FALSE;
GRANT ple_auth TO ple_api_login WITH INHERIT FALSE, SET TRUE, ADMIN FALSE;
GRANT ple_imathas_question_backend_grading_worker TO ple_worker_login
    WITH INHERIT FALSE, SET TRUE, ADMIN FALSE;
GRANT CONNECT ON DATABASE ple_e2e_baseline TO ple_api_login, ple_worker_login;

-- ASVS 8.2.1-8.2.3: only the trusted grading commit invokes the recorder.
DO $$
BEGIN
    IF (
        SELECT count(*)
        FROM pg_proc AS procedure
        JOIN pg_namespace AS namespace ON namespace.oid = procedure.pronamespace
        JOIN pg_roles AS owner_role ON owner_role.oid = procedure.proowner
        WHERE namespace.nspname = 'ple_api'
          AND procedure.proname = 'record_question_statistics_observation'
          AND pg_get_function_identity_arguments(procedure.oid) =
              'p_automated_grading_receipt_id uuid, p_eligible_choice_ids text[]'
          AND owner_role.rolname = 'ple_api_owner'
          AND procedure.prosecdef
          AND array_to_string(procedure.proconfig, ',') LIKE 'search_path=pg_catalog,%'
    ) <> 1
    OR EXISTS (
        SELECT 1
        FROM unnest(
            ARRAY['public', 'ple_app', 'ple_imathas_question_backend_grading_worker']::name[]
        ) AS capability(role)
        WHERE has_function_privilege(
            capability.role,
            'ple_api.record_question_statistics_observation(uuid,text[])',
            'EXECUTE'
        )
    ) THEN
        RAISE EXCEPTION 'Question Statistics Observation recorder authority is not exact';
    END IF;
END
$$;

-- The Store fixture proves its exact Student/Attempt/Question Source join before
-- the Rust process exercises the same authenticated procedure boundary.
BEGIN;
SET LOCAL ROLE ple_auth;
SELECT session_id FROM ple_api.resolve_and_install_session(
    decode('425ed4e4a36b30ea21b90e21c712c649e8214c29b7eaf68089d1039c6e55384c', 'hex')
);
SET LOCAL ROLE ple_api_owner;
SELECT
    EXISTS (SELECT 1 FROM ple_private.question_attempt WHERE question_attempt_id = '00000000-0000-0000-0000-00000000f205') AS has_question_attempt,
    EXISTS (
        SELECT 1
        FROM ple_private.question_attempt AS question_attempt
        JOIN ple_private.issued_question AS issued_question
          ON issued_question.issued_question_id = question_attempt.issued_question_id
        WHERE question_attempt.question_attempt_id = '00000000-0000-0000-0000-00000000f208'
          AND NOT issued_question.statistics_eligible
    ) AS has_ineligible_question_attempt,
    EXISTS (SELECT 1 FROM ple_private.issued_question WHERE issued_question_id = '00000000-0000-5000-8000-000000000115') AS has_issued_question,
    EXISTS (SELECT 1 FROM ple_private.question_source WHERE question_id = 'ABC-DEF0' AND revision_number = 1 AND source_object_id = '00000000-0000-0000-0000-00000000f202') AS has_question_source,
    ple_api.current_session_account_owns_student_record('00000000-0000-0000-0000-000000000105', '00000000-0000-0000-0000-000000000106') AS owns_student_record;
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1
        FROM ple_private.question_attempt AS question_attempt
        JOIN ple_private.issued_question AS issued_question
          ON issued_question.issued_question_id = question_attempt.issued_question_id
        JOIN ple_private.assignment_attempt AS assignment_attempt
          ON assignment_attempt.assignment_attempt_id = issued_question.assignment_attempt_id
        JOIN ple_data.student_record AS student
          ON student.student_record_id = assignment_attempt.student_record_id
        JOIN ple_data.assignment AS assignment
          ON assignment.assignment_id = assignment_attempt.assignment_id
        JOIN ple_private.question_source AS question_source
          ON question_source.question_id = issued_question.question_id
         AND question_source.revision_number = issued_question.revision_number
         AND question_source.source_object_id = '00000000-0000-0000-0000-00000000f202'
         AND question_source.source_object_checksum = repeat('aa', 32)
        WHERE question_attempt.question_attempt_id = '00000000-0000-0000-0000-00000000f205'
          AND student.student_account_id = '00000000-0000-0000-0000-000000000101'
          AND student.course_id = '00000000-0000-0000-0000-000000000105'
          AND assignment.course_id = '00000000-0000-0000-0000-000000000105'
          AND assignment_attempt.assignment_id = '00000000-0000-0000-0000-000000000110'
          AND issued_question.question_id = 'ABC-DEF0'
          AND issued_question.revision_number = 1
          AND question_attempt.question_seed = 1
          AND ple_api.current_session_account_owns_student_record(
              '00000000-0000-0000-0000-000000000105', assignment_attempt.student_record_id
          )
    ) THEN
        RAISE EXCEPTION 'iMathAS Question Backend Session fixture does not resolve the exact active Student context';
    END IF;
END
$$;
SET LOCAL ROLE ple_app;
SELECT ple_api.create_imathas_question_backend_session(
    '00000000-0000-0000-0000-00000000f206',
    '00000000-0000-0000-0000-000000000105',
    '00000000-0000-0000-0000-000000000110',
    '00000000-0000-0000-0000-00000000f205',
    'self-hosted-imathas', 'fixture-item', 'ABC-DEF0', 1,
    '00000000-0000-0000-0000-00000000f202', decode(repeat('aa', 32), 'hex'),
    'scored-embed', 1, 'all_or_nothing', 1, repeat('c', 64), decode(repeat('01', 32), 'hex'),
    decode(repeat('03', 32), 'hex'),
    convert_to('aa.' || repeat('b', 64), 'UTF8'),
    pg_catalog.clock_timestamp() - interval '5 seconds',
    pg_catalog.clock_timestamp() + interval '180 seconds',
    'imathas-question-backend-oracle-2026', decode(repeat('07', 24), 'hex'), decode(repeat('ef', 17), 'hex')
);
COMMIT;

-- ASVS 2.3.1 and 2.3.3: the received iMathAS Question Backend Result Token becomes durable only on
-- an Exchange after verified consumption; a Session never reserves it.
BEGIN;
SET LOCAL ROLE ple_private_owner;
DO $$
BEGIN
    IF EXISTS (
        SELECT 1
        FROM pg_catalog.pg_attribute
        WHERE attrelid = 'ple_private.imathas_question_backend_session'::regclass
          AND attname = 'imathas_result_token_sha256' AND NOT attisdropped
    ) OR NOT EXISTS (
        SELECT 1
        FROM pg_catalog.pg_attribute
        WHERE attrelid = 'ple_private.imathas_result_exchange'::regclass
          AND attname = 'imathas_result_token_sha256' AND NOT attisdropped
          AND atttypid = 'bytea'::pg_catalog.regtype
    ) OR NOT EXISTS (
        SELECT 1
        FROM pg_catalog.pg_constraint
        WHERE conrelid = 'ple_private.imathas_result_exchange'::regclass
          AND conname = 'imathas_result_exchange_state_matches'
          AND pg_catalog.pg_get_constraintdef(oid) LIKE '%imathas_result_token_sha256%'
    ) OR NOT EXISTS (
        SELECT 1
        FROM pg_catalog.pg_constraint
        WHERE conrelid = 'ple_private.imathas_result_exchange'::regclass
          AND pg_catalog.pg_get_constraintdef(oid)
              LIKE '%octet_length(imathas_result_token_sha256) = 32%'
    ) THEN
        RAISE EXCEPTION 'iMathAS Question Backend Result Token schema boundary is not exact';
    END IF;
END
$$;
COMMIT;

-- A Challenge is part of the Session's write-once iMathAS Question Backend correlation. Probe
-- the production transition trigger directly after the authenticated procedure
-- creates this fixture, then leave the row available for the Store oracle.
BEGIN;
SET LOCAL ROLE ple_api_owner;
DO $$
DECLARE
    challenge_target_is_visible boolean := false;
    challenge_mutation_was_refused boolean := false;
BEGIN
    SELECT EXISTS (
        SELECT 1
        FROM ple_private.imathas_question_backend_session
        WHERE imathas_question_backend_session_id = '00000000-0000-0000-0000-00000000f206'
    ) INTO challenge_target_is_visible;
    IF NOT challenge_target_is_visible THEN
        RAISE EXCEPTION
            'iMathAS Question Backend Session Challenge target is not visible to ple_api_owner';
    END IF;
    BEGIN
        UPDATE ple_private.imathas_question_backend_session
        SET imathas_question_backend_session_challenge = decode(repeat('04', 32), 'hex')
        WHERE imathas_question_backend_session_id = '00000000-0000-0000-0000-00000000f206';
    EXCEPTION
        WHEN check_violation THEN
            challenge_mutation_was_refused := true;
    END;
    IF NOT challenge_mutation_was_refused THEN
        RAISE EXCEPTION
            'iMathAS Question Backend Session accepted a mutable Challenge';
    END IF;
END
$$;
COMMIT;

-- Each iMathAS Question Backend Grading Context member is a write-once Session binding.
-- These static probes exercise the production trigger as ple_api_owner.
BEGIN;
SET LOCAL ROLE ple_api_owner;
DO $$
BEGIN
    BEGIN
        UPDATE ple_private.imathas_question_backend_session
        SET question_attempt_id = '00000000-0000-0000-0000-00000000f206'
        WHERE imathas_question_backend_session_id = '00000000-0000-0000-0000-00000000f206';
        RAISE EXCEPTION 'iMathAS Question Backend Session accepted a mutable Question Attempt';
    EXCEPTION WHEN check_violation THEN NULL;
    END;
    BEGIN
        UPDATE ple_private.imathas_question_backend_session
        SET question_id = 'ABC-DEF1'
        WHERE imathas_question_backend_session_id = '00000000-0000-0000-0000-00000000f206';
        RAISE EXCEPTION 'iMathAS Question Backend Session accepted a mutable Question ID';
    EXCEPTION WHEN check_violation THEN NULL;
    END;
    BEGIN
        UPDATE ple_private.imathas_question_backend_session
        SET revision_number = 2
        WHERE imathas_question_backend_session_id = '00000000-0000-0000-0000-00000000f206';
        RAISE EXCEPTION 'iMathAS Question Backend Session accepted a mutable Question Revision Number';
    EXCEPTION WHEN check_violation THEN NULL;
    END;
    BEGIN
        UPDATE ple_private.imathas_question_backend_session
        SET question_seed = 2
        WHERE imathas_question_backend_session_id = '00000000-0000-0000-0000-00000000f206';
        RAISE EXCEPTION 'iMathAS Question Backend Session accepted a mutable Question Seed';
    EXCEPTION WHEN check_violation THEN NULL;
    END;
END
$$;
COMMIT;
