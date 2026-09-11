-- Disposable M12 asset-delivery authorization oracle.  This is intentionally
-- not a permanent unit test: it proves the rebuilt database boundary in the
-- fresh PostgreSQL migration-acceptance runtime.
\set ON_ERROR_STOP on

CREATE TEMP TABLE m12_asset_delivery_fixture (
    case_name text PRIMARY KEY,
    assignment_id uuid NOT NULL,
    assignment_revision_id uuid NOT NULL,
    assignment_entry_id uuid NOT NULL,
    assignment_attempt_id uuid NOT NULL,
    issued_question_id uuid NOT NULL,
    question_attempt_id uuid NOT NULL,
    available_at timestamptz,
    due_at timestamptz,
    closes_at timestamptz,
    late_work_rule text NOT NULL
) ON COMMIT PRESERVE ROWS;

INSERT INTO m12_asset_delivery_fixture
SELECT case_name, gen_random_uuid(), gen_random_uuid(), gen_random_uuid(),
       gen_random_uuid(), gen_random_uuid(), gen_random_uuid(),
       available_at, due_at, closes_at, late_work_rule
  FROM (VALUES
      ('active', NULL::timestamptz, NULL::timestamptz, NULL::timestamptz, 'accept'),
      ('closed', NULL::timestamptz, NULL::timestamptz, clock_timestamp() - interval '1 minute', 'accept'),
      ('not_yet', clock_timestamp() + interval '1 minute', NULL::timestamptz, NULL::timestamptz, 'accept'),
      ('late_reject', NULL::timestamptz, clock_timestamp() - interval '1 minute', NULL::timestamptz, 'reject')
  ) AS cases(case_name, available_at, due_at, closes_at, late_work_rule);

-- The preceding released-entry oracle supplies the Course, Student Record,
-- account session, and one published PLE Question.  All M12-private facts
-- below are generated afresh and remain confined to this disposable runtime.
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
    assignment_question_order_rule, assignment_status
)
SELECT fixture.assignment_id, assignment.course_id,
       assignment.source_blueprint_course_reference_number,
       assignment.source_blueprint_revision_number, clock_timestamp(), clock_timestamp(),
       1, 'M12 delivery oracle', '', fixture.available_at, fixture.due_at,
       fixture.closes_at, NULL, NULL, fixture.late_work_rule, 'auto_submit',
       'answer_all', NULL, 'latest', 'unlimited', NULL, 'reuse_selection',
       'reuse_variation', 'resumable', 'all_questions', 'free_navigation',
       'authored_order', 'unreleased'
  FROM m12_asset_delivery_fixture AS fixture
 CROSS JOIN LATERAL (
     SELECT * FROM ple_data.assignment
      WHERE assignment_id = '00000000-0000-0000-0000-000000000110'
 ) AS assignment;
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
SELECT fixture.assignment_revision_id, fixture.assignment_id, base.course_id,
       base.course_schedule_revision_id, 1, 'M12 delivery oracle', '',
       fixture.available_at, fixture.due_at, fixture.closes_at, NULL, NULL,
       fixture.late_work_rule, 'auto_submit', 'answer_all', NULL, 'latest',
       'unlimited', NULL, 'reuse_selection', 'reuse_variation', 'resumable',
       'all_questions', 'free_navigation', 'authored_order', clock_timestamp()
  FROM m12_asset_delivery_fixture AS fixture
 CROSS JOIN LATERAL (
     SELECT * FROM ple_data.assignment_revision
      WHERE assignment_revision_id = '00000000-0000-0000-0000-000000000111'
 ) AS base;
UPDATE ple_data.assignment AS assignment
   SET assignment_status = 'released',
       released_assignment_revision_id = fixture.assignment_revision_id,
       updated_at = clock_timestamp()
  FROM m12_asset_delivery_fixture AS fixture
 WHERE assignment.assignment_id = fixture.assignment_id;
BEGIN;
INSERT INTO ple_data.assignment_revision_entry (
    assignment_revision_id, assignment_entry_id, assignment_content_entry_index,
    entry_kind, availability, scoring_rule, point_value, question_attempt_limit,
    question_attempt_time_limit_seconds, question_attempt_time_limit_grace_seconds
)
SELECT assignment_revision_id, assignment_entry_id, 0, 'fixed_question',
       'available', 'normal', 1, NULL, NULL, NULL
  FROM m12_asset_delivery_fixture;
INSERT INTO ple_data.assignment_revision_fixed_question (
    assignment_revision_id, assignment_entry_id, question_id, revision_number
)
SELECT assignment_revision_id, assignment_entry_id, 'ABC-DEF0', 1
  FROM m12_asset_delivery_fixture;
COMMIT;
INSERT INTO ple_private.assignment_attempt (
    assignment_attempt_id, student_record_id, assignment_id, assignment_revision_id,
    started_at, completed_at, attempt_number, question_pool_reuse_rule,
    question_variation_rule
)
SELECT fixture.assignment_attempt_id, '00000000-0000-0000-0000-000000000106',
       fixture.assignment_id, fixture.assignment_revision_id, clock_timestamp(),
       NULL, 1, 'reuse_selection', 'reuse_variation'
  FROM m12_asset_delivery_fixture AS fixture;
INSERT INTO ple_private.issued_question (
    issued_question_id, assignment_attempt_id, assignment_entry_id, question_id,
    revision_number, issued_position, point_value, scoring_rule,
    question_statistics_eligibility
)
SELECT issued_question_id, assignment_attempt_id, assignment_entry_id, 'ABC-DEF0',
       1, 0, 1, 'normal', true
  FROM m12_asset_delivery_fixture;
INSERT INTO ple_private.question_attempt (
    question_attempt_id, issued_question_id, question_seed,
    generated_parameter_sha256, issued_at, deadline_at, question_attempt_state,
    reproduction_details
)
SELECT question_attempt_id, issued_question_id, 1, repeat('a', 64),
       clock_timestamp(), NULL, 'open', '{}'::jsonb
  FROM m12_asset_delivery_fixture;
INSERT INTO ple_private.question_attempt_presentation_binding (
    question_attempt_id, descriptor_version, presentation_nonce, presentation_checksum
)
SELECT question_attempt_id, 1, md5(question_attempt_id::text), repeat('b', 64)
  FROM m12_asset_delivery_fixture;
SELECT gen_random_uuid() AS m12_asset_id \gset
SELECT gen_random_uuid() AS m12_source_object_id \gset
SELECT gen_random_uuid() AS m12_public_object_id \gset
SELECT gen_random_uuid() AS m12_delivery_id \gset
SELECT gen_random_uuid() AS m12_job_id \gset
SELECT gen_random_uuid() AS m12_lease_token \gset

BEGIN;
INSERT INTO ple_private.object_record (
    object_id, object_address, object_storage_area, object_data_class, sha256,
    size_bytes, media_type, created_at
) VALUES (
    :'m12_source_object_id'::uuid,
    jsonb_build_object('kind', 'restrictedQuestionAsset', 'questionRevision',
        jsonb_build_object('questionId', 'ABC-DEF0', 'revisionNumber', 1),
        'asset', :'m12_asset_id'::uuid, 'object', :'m12_source_object_id'::uuid),
    'private-content', 'question-asset', decode(repeat('a', 64), 'hex'), 17,
    'image/png', clock_timestamp()
);
INSERT INTO ple_private.job (
    job_id, job_kind, job_target_kind, question_id, revision_number, generation,
    payload, state, available_at, max_attempts, created_at
) VALUES (
    :'m12_job_id'::uuid, 'publish_public_assets', 'public_asset_publication',
    'ABC-DEF0', 1, 1, '{}'::jsonb, 'ready', clock_timestamp() - interval '1 second',
    1, clock_timestamp()
);
INSERT INTO ple_data.object_delivery (
    delivery_id, object_id, sha256, media_type, byte_length, delivery_state, registered_at
) VALUES (
    :'m12_delivery_id'::uuid, :'m12_public_object_id'::uuid,
    decode(repeat('c', 64), 'hex'), 'image/png', 17, 'pending', clock_timestamp()
);
INSERT INTO ple_data.question_asset_delivery (
    delivery_id, object_id, question_id, revision_number, asset_id
) VALUES (
    :'m12_delivery_id'::uuid, :'m12_public_object_id'::uuid, 'ABC-DEF0', 1,
    :'m12_asset_id'::uuid
);
INSERT INTO ple_private.question_asset_publication (
    question_id, revision_number, asset_id, source_object_id, source_object_checksum,
    public_object_id, public_object_checksum, public_byte_length, verified_media_type,
    intrinsic_width, intrinsic_height, delivery_id, job_id, publication_state
) VALUES (
    'ABC-DEF0', 1, :'m12_asset_id'::uuid, :'m12_source_object_id'::uuid,
    decode(repeat('a', 64), 'hex'), :'m12_public_object_id'::uuid,
    decode(repeat('c', 64), 'hex'), 17, 'image/png', 1, 1,
    :'m12_delivery_id'::uuid, :'m12_job_id'::uuid, 'pending'
);
COMMIT;

-- Publisher-only state transition: test data never manufactures Ready rows.
SET ROLE ple_public_asset_publisher;
SELECT job_id FROM ple_private.claim_question_asset_publication_job(
    :'m12_lease_token'::uuid, clock_timestamp() + interval '60 seconds'
) WHERE job_id = :'m12_job_id'::uuid \gset
SELECT ple_private.activate_question_asset_publication(
    :'job_id'::uuid, :'m12_lease_token'::uuid
);
RESET ROLE;

-- A valid Pending sibling and a valid Ready-but-unbound sibling must remain
-- indistinguishable from absent to the resolver, even while the exact active
-- attempt below has its separate Ready rendition.
SELECT gen_random_uuid() AS m12_pending_asset_id \gset
SELECT gen_random_uuid() AS m12_pending_source_object_id \gset
SELECT gen_random_uuid() AS m12_pending_public_object_id \gset
SELECT gen_random_uuid() AS m12_pending_delivery_id \gset
SELECT gen_random_uuid() AS m12_pending_job_id \gset
SELECT gen_random_uuid() AS m12_unbound_asset_id \gset
SELECT gen_random_uuid() AS m12_unbound_source_object_id \gset
SELECT gen_random_uuid() AS m12_unbound_public_object_id \gset
SELECT gen_random_uuid() AS m12_unbound_delivery_id \gset
SELECT gen_random_uuid() AS m12_unbound_job_id \gset
SELECT gen_random_uuid() AS m12_unbound_lease_token \gset
BEGIN;
INSERT INTO ple_private.object_record (object_id, object_address, object_storage_area, object_data_class, sha256, size_bytes, media_type, created_at)
VALUES
(:'m12_pending_source_object_id'::uuid, jsonb_build_object('kind','restrictedQuestionAsset','questionRevision',jsonb_build_object('questionId','ABC-DEF0','revisionNumber',1),'asset',:'m12_pending_asset_id'::uuid,'object',:'m12_pending_source_object_id'::uuid), 'private-content','question-asset',decode(repeat('d',64),'hex'),17,'image/png',clock_timestamp()),
(:'m12_unbound_source_object_id'::uuid, jsonb_build_object('kind','restrictedQuestionAsset','questionRevision',jsonb_build_object('questionId','ABC-DEF0','revisionNumber',1),'asset',:'m12_unbound_asset_id'::uuid,'object',:'m12_unbound_source_object_id'::uuid), 'private-content','question-asset',decode(repeat('e',64),'hex'),17,'image/png',clock_timestamp());
INSERT INTO ple_private.job (job_id, job_kind, job_target_kind, question_id, revision_number, generation, payload, state, available_at, max_attempts, created_at)
VALUES
(:'m12_pending_job_id'::uuid,'publish_public_assets','public_asset_publication','ABC-DEF0',1,1,'{}', 'ready',clock_timestamp()-interval '1 second',1,clock_timestamp()),
(:'m12_unbound_job_id'::uuid,'publish_public_assets','public_asset_publication','ABC-DEF0',1,1,'{}', 'ready',clock_timestamp()-interval '2 seconds',1,clock_timestamp());
INSERT INTO ple_data.object_delivery (delivery_id, object_id, sha256, media_type, byte_length, delivery_state, registered_at)
VALUES
(:'m12_pending_delivery_id'::uuid,:'m12_pending_public_object_id'::uuid,decode(repeat('f',64),'hex'),'image/png',17,'pending',clock_timestamp()),
(:'m12_unbound_delivery_id'::uuid,:'m12_unbound_public_object_id'::uuid,decode(repeat('9',64),'hex'),'image/png',17,'pending',clock_timestamp());
INSERT INTO ple_data.question_asset_delivery (delivery_id, object_id, question_id, revision_number, asset_id)
VALUES
(:'m12_pending_delivery_id'::uuid,:'m12_pending_public_object_id'::uuid,'ABC-DEF0',1,:'m12_pending_asset_id'::uuid),
(:'m12_unbound_delivery_id'::uuid,:'m12_unbound_public_object_id'::uuid,'ABC-DEF0',1,:'m12_unbound_asset_id'::uuid);
INSERT INTO ple_private.question_asset_publication (question_id, revision_number, asset_id, source_object_id, source_object_checksum, public_object_id, public_object_checksum, public_byte_length, verified_media_type, intrinsic_width, intrinsic_height, delivery_id, job_id, publication_state)
VALUES
('ABC-DEF0',1,:'m12_pending_asset_id'::uuid,:'m12_pending_source_object_id'::uuid,decode(repeat('d',64),'hex'),:'m12_pending_public_object_id'::uuid,decode(repeat('f',64),'hex'),17,'image/png',1,1,:'m12_pending_delivery_id'::uuid,:'m12_pending_job_id'::uuid,'pending'),
('ABC-DEF0',1,:'m12_unbound_asset_id'::uuid,:'m12_unbound_source_object_id'::uuid,decode(repeat('e',64),'hex'),:'m12_unbound_public_object_id'::uuid,decode(repeat('9',64),'hex'),17,'image/png',1,1,:'m12_unbound_delivery_id'::uuid,:'m12_unbound_job_id'::uuid,'pending');
COMMIT;
SET ROLE ple_public_asset_publisher;
SELECT job_id FROM ple_private.claim_question_asset_publication_job(:'m12_unbound_lease_token'::uuid, clock_timestamp() + interval '60 seconds') WHERE job_id = :'m12_unbound_job_id'::uuid \gset
SELECT ple_private.activate_question_asset_publication(:'job_id'::uuid, :'m12_unbound_lease_token'::uuid);
RESET ROLE;

INSERT INTO ple_private.question_attempt_presentation_asset_binding (question_attempt_id)
SELECT question_attempt_id FROM m12_asset_delivery_fixture;
INSERT INTO ple_private.question_attempt_presentation_asset_rendition (
    question_attempt_id, asset_id, rendition_checksum
)
SELECT question_attempt_id, :'m12_asset_id'::uuid, decode(repeat('c', 64), 'hex')
  FROM m12_asset_delivery_fixture;

-- The real session installer and API resolver prove active access.  It returns
-- no object address, source, bytes, or identifier to this oracle's output.
BEGIN;
SET ROLE ple_auth;
SELECT ple_api.resolve_and_install_session(decode(repeat('ab', 32), 'hex')) \gset
SET ROLE ple_app;
SELECT set_config('ple_e2e.m12_asset_id', :'m12_asset_id', false) AS m12_session_asset_id \gset
DO $$
BEGIN
    IF (SELECT count(*) FROM ple_api.resolve_ready_live_demo_question_asset(
        current_setting('ple_e2e.m12_asset_id')::uuid
    )) <> 1 THEN
        RAISE EXCEPTION 'active exact Question Attempt did not resolve its Ready rendition';
    END IF;
END $$;
SELECT set_config('ple_e2e.m12_pending_asset_id', :'m12_pending_asset_id', false) AS m12_pending_session_asset_id \gset
SELECT set_config('ple_e2e.m12_unbound_asset_id', :'m12_unbound_asset_id', false) AS m12_unbound_session_asset_id \gset
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM ple_api.resolve_ready_live_demo_question_asset(
        current_setting('ple_e2e.m12_pending_asset_id')::uuid
    )) OR EXISTS (SELECT 1 FROM ple_api.resolve_ready_live_demo_question_asset(
        current_setting('ple_e2e.m12_unbound_asset_id')::uuid
    )) THEN
        RAISE EXCEPTION 'Pending or unbound Question Asset sibling resolved';
    END IF;
END $$;
COMMIT;

-- A Student without this Course Record and a Sysadmin have authentic sessions
-- but no resolver result.  An Active Instructor is intentionally not used as
-- a denial case: Question Asset delivery follows the global Question Library
-- Instructor visibility authority.
SELECT gen_random_uuid() AS m12_wrong_student_account_id \gset
SELECT gen_random_uuid() AS m12_wrong_sysadmin_account_id \gset
SELECT gen_random_uuid() AS m12_wrong_student_session_id \gset
SELECT gen_random_uuid() AS m12_wrong_sysadmin_session_id \gset
RESET ROLE;
INSERT INTO ple_private.account (account_id, product_role, created_at) VALUES
    (:'m12_wrong_student_account_id'::uuid, 'student', clock_timestamp()),
    (:'m12_wrong_sysadmin_account_id'::uuid, 'sysadmin', clock_timestamp());
INSERT INTO ple_private.authenticated_session (session_id, account_id, product_role, token_hash, created_at, expires_at, revoked_at) VALUES
    (:'m12_wrong_student_session_id'::uuid, :'m12_wrong_student_account_id'::uuid, 'student', decode(repeat('1',64),'hex'), clock_timestamp(), clock_timestamp()+interval '1 hour', NULL),
    (:'m12_wrong_sysadmin_session_id'::uuid, :'m12_wrong_sysadmin_account_id'::uuid, 'sysadmin', decode(repeat('2',64),'hex'), clock_timestamp(), clock_timestamp()+interval '1 hour', NULL);
BEGIN;
SET ROLE ple_auth;
SELECT ple_api.resolve_and_install_session(decode(repeat('1',64),'hex')) AS m12_wrong_student_installed \gset
SET ROLE ple_app;
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM ple_api.resolve_ready_live_demo_question_asset(
        current_setting('ple_e2e.m12_asset_id')::uuid
    )) THEN RAISE EXCEPTION 'wrong Student resolved a Question Asset'; END IF;
END $$;
COMMIT;
BEGIN;
SET ROLE ple_auth;
SELECT ple_api.resolve_and_install_session(decode(repeat('2',64),'hex')) AS m12_wrong_instructor_installed \gset
SET ROLE ple_app;
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM ple_api.resolve_ready_live_demo_question_asset(
        current_setting('ple_e2e.m12_asset_id')::uuid
    )) THEN RAISE EXCEPTION 'Sysadmin resolved a Question Asset'; END IF;
    BEGIN
        EXECUTE 'SELECT 1 FROM ple_private.question_attempt_presentation_asset_rendition LIMIT 1';
        RAISE EXCEPTION 'ple_app read a private Question Asset binding directly';
    EXCEPTION WHEN insufficient_privilege THEN NULL;
    END;
END $$;
COMMIT;

-- Restore the exact Student session before the completed/availability matrix.
BEGIN;
SET ROLE ple_auth;
SELECT ple_api.resolve_and_install_session(decode(repeat('ab',32),'hex')) AS m12_exact_student_installed \gset
SET ROLE ple_app;
COMMIT;

-- The completed-Attempt branch retains only the owned exact Ready rendition.
-- Keep completion and revocation transactional so the shared active fixture is
-- restored after this oracle has exercised the completed authority path.
BEGIN;
RESET ROLE;
UPDATE ple_private.assignment_attempt
   SET completed_at = clock_timestamp()
 WHERE assignment_attempt_id = (
     SELECT assignment_attempt_id FROM m12_asset_delivery_fixture WHERE case_name = 'active'
 );
SET ROLE ple_auth;
SELECT ple_api.resolve_and_install_session(decode(repeat('ab',32),'hex'));
SET ROLE ple_app;
DO $$
BEGIN
    IF (SELECT count(*) FROM ple_api.resolve_ready_live_demo_question_asset(
        current_setting('ple_e2e.m12_asset_id')::uuid
    )) <> 1 THEN
        RAISE EXCEPTION 'owned completed Question Attempt did not resolve its exact Ready rendition';
    END IF;
END $$;
SET ROLE ple_auth;
SELECT ple_api.resolve_and_install_session(decode(repeat('1',64),'hex'));
SET ROLE ple_app;
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM ple_api.resolve_ready_live_demo_question_asset(
        current_setting('ple_e2e.m12_asset_id')::uuid
    )) THEN
        RAISE EXCEPTION 'foreign Student resolved a completed Question Asset';
    END IF;
END $$;
RESET ROLE;
SET ROLE ple_api_owner;
INSERT INTO ple_data.course_membership_event (
    course_membership_event_id, membership_id, event_kind, occurred_at, reason
) VALUES (
    gen_random_uuid(), '00000000-0000-0000-0000-000000000108'::uuid,
    'ended', clock_timestamp(), 'M12 completed Question Asset revocation acceptance'
);
SET ROLE ple_auth;
SELECT ple_api.resolve_and_install_session(decode(repeat('ab',32),'hex'));
SET ROLE ple_app;
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM ple_api.resolve_ready_live_demo_question_asset(
        current_setting('ple_e2e.m12_asset_id')::uuid
    )) THEN
        RAISE EXCEPTION 'revoked Student resolved a completed Question Asset';
    END IF;
END $$;
ROLLBACK;

-- Direct table reads stay denied to the public application capability while
-- the one fixed resolver remains executable.
RESET ROLE;
DO $$
BEGIN
    IF NOT has_function_privilege('ple_app', 'ple_api.resolve_ready_live_demo_question_asset(uuid)', 'EXECUTE')
       OR has_table_privilege('ple_app', 'ple_private.question_attempt_presentation_asset_rendition', 'SELECT')
       OR has_table_privilege('ple_app', 'ple_private.question_asset_publication', 'SELECT') THEN
        RAISE EXCEPTION 'Question Asset delivery API/table boundary is not least privilege';
    END IF;
END $$;
RESET ROLE;
SELECT 'question_asset_delivery_oracle_pass';
