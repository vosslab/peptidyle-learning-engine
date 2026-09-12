-- One connected PostgreSQL acceptance for Assignment Unrelease.  The leased
-- baseline owner invokes this after it has installed the canonical schema.
-- Fixture writes use schema-owner roles solely to construct otherwise-valid
-- Student Work; the operation under test is always invoked as ple_app with a
-- current-session account identity.
\set ON_ERROR_STOP on

BEGIN;
SET CONSTRAINTS ALL DEFERRED;

-- Stable fixture identities keep the assertions readable without introducing
-- a product-side test marker or alternate model.
SET LOCAL ROLE ple_private_owner;
INSERT INTO ple_private.account (account_id, product_role, created_at) VALUES
    ('10000000-0000-0000-0000-000000000001', 'instructor', clock_timestamp()),
    ('10000000-0000-0000-0000-000000000002', 'student', clock_timestamp());

SET LOCAL ROLE ple_data_owner;
INSERT INTO ple_data.published_question (question_id, created_at)
VALUES ('ABCDEF0', clock_timestamp());
INSERT INTO ple_data.question_revision (question_id, revision_number, backend, published_at)
VALUES ('ABCDEF0', 1, 'ple', clock_timestamp());
SET LOCAL ROLE ple_api_owner;
INSERT INTO ple_data.blueprint_course (
    blueprint_id, reference_number, owner_account_id, created_at
) OVERRIDING SYSTEM VALUE VALUES (
    '20000000-0000-0000-0000-000000000001', 1,
    '10000000-0000-0000-0000-000000000001', clock_timestamp()
);
INSERT INTO ple_data.blueprint_course_revision (
    blueprint_course_reference_number, blueprint_revision_number, title,
    content, content_checksum, published_at
) VALUES (1, 1, 'Unrelease acceptance Blueprint', '{}'::jsonb,
    decode(repeat('a', 64), 'hex'), clock_timestamp());
INSERT INTO ple_data.blueprint_revision_module (
    blueprint_course_reference_number, blueprint_revision_number,
    blueprint_module_reference, module_position
) VALUES (1, 1, '20000000-0000-0000-0000-000000000002', 1);
INSERT INTO ple_data.blueprint_revision_assignment (
    blueprint_course_reference_number, blueprint_revision_number,
    blueprint_module_reference, blueprint_assignment_reference, assignment_position
) VALUES (1, 1, '20000000-0000-0000-0000-000000000002',
    '20000000-0000-0000-0000-000000000003', 1);
INSERT INTO ple_data.course_instance (
    course_id, reference_number, blueprint_course_reference_number,
    blueprint_revision_number, assigned_instructor_account_id,
    assigned_instructor_role, course_short_name, course_long_name,
    term_starts_on, term_ends_on, created_at
) OVERRIDING SYSTEM VALUE VALUES (
    '30000000-0000-0000-0000-000000000001', 1, 1, 1,
    '10000000-0000-0000-0000-000000000001', 'instructor', 'UNR-1',
    'Unrelease acceptance Course', current_date, current_date + 1,
    clock_timestamp()
);
INSERT INTO ple_data.student_record (
    student_record_id, course_id, student_account_id, created_at
) VALUES ('30000000-0000-0000-0000-000000000002',
    '30000000-0000-0000-0000-000000000001',
    '10000000-0000-0000-0000-000000000002', clock_timestamp());
INSERT INTO ple_data.course_membership (
    membership_id, course_id, account_id, role, student_record_id, joined_at
) VALUES
    ('30000000-0000-0000-0000-000000000003',
     '30000000-0000-0000-0000-000000000001',
     '10000000-0000-0000-0000-000000000001', 'instructor', NULL, clock_timestamp()),
    ('30000000-0000-0000-0000-000000000004',
     '30000000-0000-0000-0000-000000000001',
     '10000000-0000-0000-0000-000000000002', 'student',
     '30000000-0000-0000-0000-000000000002', clock_timestamp());

-- Target and survivor use the same immutable Question Revision.  The survivor
-- makes statistics rebuilding observable rather than trusting a deletion-side
-- counter.
SET LOCAL ROLE ple_data_owner;
INSERT INTO ple_data.assignment (
    assignment_id, reference_number, course_id,
    source_blueprint_course_reference_number, source_blueprint_revision_number,
    source_blueprint_assignment_reference, created_at, updated_at,
    assignment_title, assignment_instructions, late_work_rule,
    assignment_completion_rule, assignment_attempt_grade_rule,
    assignment_attempt_continuation_rule, question_pool_reuse_rule,
    question_variation_rule, assignment_attempt_resume_rule,
    assignment_question_display_rule, assignment_navigation_rule,
    assignment_question_order_rule, feedback_score, feedback_per_item_correctness,
    feedback_submitted_response, feedback_question_feedback, feedback_question_answer,
    feedback_question_answer_explanation, feedback_class_statistics, assignment_status
) OVERRIDING SYSTEM VALUE VALUES
    ('40000000-0000-0000-0000-000000000001', 1,
     '30000000-0000-0000-0000-000000000001', 1, 1,
     '20000000-0000-0000-0000-000000000003', clock_timestamp(), clock_timestamp(),
     'Unrelease target', '', 'accept', 'answer_all', 'latest', 'unlimited',
     'reuse_selection', 'reuse_variation', 'resumable', 'all_questions',
     'free_navigation', 'authored_order', 'after_submit', 'after_submit',
     'after_submit', 'after_submit', 'after_submit', 'after_submit', 'never', 'released'),
    ('40000000-0000-0000-0000-000000000002', 2,
     '30000000-0000-0000-0000-000000000001', 1, 1,
     '20000000-0000-0000-0000-000000000003', clock_timestamp(), clock_timestamp(),
     'Statistics survivor', '', 'accept', 'answer_all', 'latest', 'unlimited',
     'reuse_selection', 'reuse_variation', 'resumable', 'all_questions',
     'free_navigation', 'authored_order', 'after_submit', 'after_submit',
     'after_submit', 'after_submit', 'after_submit', 'after_submit', 'never', 'released'),
    ('40000000-0000-0000-0000-000000000003', 3,
     '30000000-0000-0000-0000-000000000001', 1, 1,
     '20000000-0000-0000-0000-000000000003', clock_timestamp(), clock_timestamp(),
     'Unrelease lock race', '', 'accept', 'answer_all', 'latest', 'unlimited',
     'reuse_selection', 'reuse_variation', 'resumable', 'all_questions',
     'free_navigation', 'authored_order', 'after_submit', 'after_submit',
     'after_submit', 'after_submit', 'after_submit', 'after_submit', 'never', 'released');
INSERT INTO ple_data.assignment_entry (
    assignment_entry_id, assignment_id, authored_position, entry_kind,
    availability, scoring_rule, question_id, question_revision_number, points_possible
) VALUES
    ('40000000-0000-0000-0000-000000000011', '40000000-0000-0000-0000-000000000001',
     0, 'fixed_question', 'available', 'normal', 'ABCDEF0', 1, 1),
    ('40000000-0000-0000-0000-000000000012', '40000000-0000-0000-0000-000000000002',
     0, 'fixed_question', 'available', 'normal', 'ABCDEF0', 1, 1),
    ('40000000-0000-0000-0000-000000000013', '40000000-0000-0000-0000-000000000003',
     0, 'fixed_question', 'available', 'normal', 'ABCDEF0', 1, 1);

-- Start both Attempts through the ordinary restricted path.  The fixture then
-- adds the lower-level submission/grading receipts needed to exercise the
-- destructive closure and statistics rebuild.
SET LOCAL ROLE ple_app;
SELECT set_config('ple.session_account_id', '10000000-0000-0000-0000-000000000002', true);
SELECT * FROM ple_api.start_assignment_attempt(
    '50000000-0000-0000-0000-000000000001',
    '30000000-0000-0000-0000-000000000002',
    '40000000-0000-0000-0000-000000000001',
    '[]'::jsonb,
    '[{"issued_question_id":"50000000-0000-0000-0000-000000000011","assignment_entry_id":"40000000-0000-0000-0000-000000000011","issued_position":0,"question_id":"ABCDEF0","revision_number":1,"question_seed":"7"}]'::jsonb
);
SELECT * FROM ple_api.start_assignment_attempt(
    '50000000-0000-0000-0000-000000000002',
    '30000000-0000-0000-0000-000000000002',
    '40000000-0000-0000-0000-000000000002',
    '[]'::jsonb,
    '[{"issued_question_id":"50000000-0000-0000-0000-000000000012","assignment_entry_id":"40000000-0000-0000-0000-000000000012","issued_position":0,"question_id":"ABCDEF0","revision_number":1,"question_seed":"8"}]'::jsonb
);
RESET ROLE;
SET LOCAL ROLE ple_private_owner;
INSERT INTO ple_private.question_attempt (
    question_attempt_id, issued_question_id, question_seed, generated_parameter_sha256,
    issued_at, submitted_at, question_attempt_state, backend_name, backend_version,
    grader_name, grader_version, rendered_question_sha256, issued_capability
) VALUES
    ('50000000-0000-0000-0000-000000000021', '50000000-0000-0000-0000-000000000011', 7,
     repeat('a', 64), transaction_timestamp(), transaction_timestamp(), 'submission_accepted', 'ple', '1',
     'ple', '1', decode(repeat('b', 64), 'hex'), 'not_applicable'),
    ('50000000-0000-0000-0000-000000000022', '50000000-0000-0000-0000-000000000012', 8,
     repeat('c', 64), transaction_timestamp(), transaction_timestamp(), 'submission_accepted', 'ple', '1',
     'ple', '1', decode(repeat('d', 64), 'hex'), 'not_applicable');
INSERT INTO ple_private.assignment_attempt_saved_response (question_attempt_id, student_response, saved_at)
VALUES ('50000000-0000-0000-0000-000000000021', '{}'::jsonb, clock_timestamp()),
       ('50000000-0000-0000-0000-000000000022', '{}'::jsonb, clock_timestamp());
INSERT INTO ple_private.question_submission (submission_id, question_attempt_id, submitted_at, student_response)
VALUES ('50000000-0000-0000-0000-000000000031', '50000000-0000-0000-0000-000000000021', transaction_timestamp(), '{}'::jsonb),
       ('50000000-0000-0000-0000-000000000032', '50000000-0000-0000-0000-000000000022', transaction_timestamp(), '{}'::jsonb);
INSERT INTO ple_private.assignment_submission (assignment_submission_id, assignment_attempt_id, submitted_at, authorized_by_account_id, receipt)
VALUES ('50000000-0000-0000-0000-000000000041', '50000000-0000-0000-0000-000000000001', clock_timestamp(), '10000000-0000-0000-0000-000000000002', '{}'::jsonb),
       ('50000000-0000-0000-0000-000000000042', '50000000-0000-0000-0000-000000000002', clock_timestamp(), '10000000-0000-0000-0000-000000000002', '{}'::jsonb);
INSERT INTO ple_private.job (job_id, job_kind, job_target_kind, question_submission_id, worker_kind, payload, state, available_at, max_attempts, created_at)
VALUES ('50000000-0000-0000-0000-000000000051', 'grade_accepted_submission', 'question_submission', '50000000-0000-0000-0000-000000000031', 'native_ple_grading', '{}'::jsonb, 'ready', clock_timestamp(), 1, clock_timestamp()),
       ('50000000-0000-0000-0000-000000000052', 'grade_accepted_submission', 'question_submission', '50000000-0000-0000-0000-000000000032', 'native_ple_grading', '{}'::jsonb, 'ready', clock_timestamp(), 1, clock_timestamp());
INSERT INTO ple_private.question_submission_grading (question_submission_grading_id, submission_id, job_id, grading_state, created_at)
VALUES ('50000000-0000-0000-0000-000000000061', '50000000-0000-0000-0000-000000000031', '50000000-0000-0000-0000-000000000051', 'pending', clock_timestamp()),
       ('50000000-0000-0000-0000-000000000062', '50000000-0000-0000-0000-000000000032', '50000000-0000-0000-0000-000000000052', 'pending', clock_timestamp());
INSERT INTO ple_private.grading_result (grading_result_id, submission_id, question_submission_grading_id, question_attempt_id, correct, points_earned, points_possible, recorded_at)
VALUES ('50000000-0000-0000-0000-000000000071', '50000000-0000-0000-0000-000000000031', '50000000-0000-0000-0000-000000000061', '50000000-0000-0000-0000-000000000021', true, 1, 1, clock_timestamp()),
       ('50000000-0000-0000-0000-000000000072', '50000000-0000-0000-0000-000000000032', '50000000-0000-0000-0000-000000000062', '50000000-0000-0000-0000-000000000022', true, 1, 1, clock_timestamp());

SET LOCAL ROLE ple_audit_owner;
INSERT INTO ple_audit.automated_grading_receipt (automated_grading_receipt_id, question_submission_grading_id, grading_result_id, committed_at, automated_grading_receipt_checksum)
VALUES ('50000000-0000-0000-0000-000000000081', '50000000-0000-0000-0000-000000000061', '50000000-0000-0000-0000-000000000071', clock_timestamp(), decode(repeat('e', 64), 'hex')),
       ('50000000-0000-0000-0000-000000000082', '50000000-0000-0000-0000-000000000062', '50000000-0000-0000-0000-000000000072', clock_timestamp(), decode(repeat('f', 64), 'hex'));
SET LOCAL ROLE ple_private_owner;
INSERT INTO ple_private.question_statistics_observation_receipt (automated_grading_receipt_id, question_attempt_id, question_id, revision_number, correct, observed_at)
VALUES ('50000000-0000-0000-0000-000000000081', '50000000-0000-0000-0000-000000000021', 'ABCDEF0', 1, true, clock_timestamp()),
       ('50000000-0000-0000-0000-000000000082', '50000000-0000-0000-0000-000000000022', 'ABCDEF0', 1, true, clock_timestamp());
SET LOCAL ROLE ple_data_owner;
SELECT ple_data.rebuild_question_revision_statistics('ABCDEF0', 1, clock_timestamp());
COMMIT;

-- Error cases are real public calls.  Each verifies the rejected transaction
-- leaves current state, retained evidence, statistics, and audit unchanged.
BEGIN;
SET LOCAL ROLE ple_app;
SELECT set_config('ple.session_account_id', '10000000-0000-0000-0000-000000000002', true);
DO $$
BEGIN
    PERFORM * FROM ple_api.read_assignment_unrelease_impact(1, 1);
    RAISE EXCEPTION 'foreign unrelease impact was enumerated';
EXCEPTION WHEN insufficient_privilege THEN NULL;
END $$;
SELECT set_config('ple.session_account_id', '10000000-0000-0000-0000-000000000001', true);
DO $$
BEGIN
    PERFORM * FROM ple_api.unrelease_assignment(1, 1, 2, 'Unrelease target');
    RAISE EXCEPTION 'stale Unrelease Edit Number was accepted';
EXCEPTION WHEN serialization_failure THEN NULL;
END $$;
DO $$
BEGIN
    PERFORM * FROM ple_api.unrelease_assignment(1, 1, 1, 'wrong title');
    RAISE EXCEPTION 'wrong Unrelease confirmation title was accepted';
EXCEPTION WHEN invalid_parameter_value THEN NULL;
END $$;
RESET ROLE;
SET LOCAL ROLE ple_data_owner;
DO $$
BEGIN
    IF (SELECT assignment_status FROM ple_data.assignment WHERE reference_number = 1) <> 'released'
       OR (SELECT assignment_edit_number FROM ple_data.assignment WHERE reference_number = 1) <> 1 THEN
        RAISE EXCEPTION 'a rejected Unrelease changed current Assignment state';
    END IF;
END $$;
SET LOCAL ROLE ple_private_owner;
DO $$
BEGIN
    IF (SELECT count(*) FROM ple_private.assignment_attempt WHERE assignment_id = '40000000-0000-0000-0000-000000000001') <> 1 THEN
        RAISE EXCEPTION 'a rejected Unrelease changed Student Work';
    END IF;
END $$;
SET LOCAL ROLE ple_audit_owner;
DO $$
BEGIN
    IF (SELECT count(*) FROM ple_audit.assignment_unrelease_event) <> 0 THEN
        RAISE EXCEPTION 'a rejected Unrelease wrote audit evidence';
    END IF;
END $$;
COMMIT;

-- The accepted public transition returns aggregate impact only, removes the
-- rooted closure, retains the current Assignment/entry and shared Revision,
-- rebuilds the shared Revision statistics from the survivor, and records a
-- redacted aggregate audit receipt.
BEGIN;
SET LOCAL ROLE ple_app;
SELECT set_config('ple.session_account_id', '10000000-0000-0000-0000-000000000001', true);
DO $$
DECLARE result record;
BEGIN
    SELECT * INTO result FROM ple_api.unrelease_assignment(1, 1, 1, 'Unrelease target');
    IF result.assignment_status <> 'unreleased' OR result.assignment_edit_number <> 2
       OR result.assignment_attempt_count <> 1 OR result.question_submission_count <> 1
       OR result.assignment_submission_count <> 1 OR result.grading_result_count <> 1 THEN
        RAISE EXCEPTION 'accepted Unrelease returned an incorrect aggregate receipt';
    END IF;
END $$;
RESET ROLE;
SET LOCAL ROLE ple_data_owner;
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM ple_data.assignment WHERE reference_number = 1 AND assignment_status = 'unreleased' AND assignment_edit_number = 2)
       OR NOT EXISTS (SELECT 1 FROM ple_data.assignment_entry WHERE assignment_id = '40000000-0000-0000-0000-000000000001')
       OR NOT EXISTS (SELECT 1 FROM ple_data.question_revision WHERE question_id = 'ABCDEF0' AND revision_number = 1) THEN
        RAISE EXCEPTION 'Unrelease did not preserve current Assignment or shared Question state';
    END IF;
END $$;
SET LOCAL ROLE ple_private_owner;
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM ple_private.assignment_attempt WHERE assignment_id = '40000000-0000-0000-0000-000000000001')
       OR EXISTS (SELECT 1 FROM ple_private.question_submission WHERE submission_id = '50000000-0000-0000-0000-000000000031')
       OR EXISTS (SELECT 1 FROM ple_private.assignment_submission WHERE assignment_submission_id = '50000000-0000-0000-0000-000000000041')
       OR EXISTS (SELECT 1 FROM ple_private.grading_result WHERE grading_result_id = '50000000-0000-0000-0000-000000000071')
       OR EXISTS (SELECT 1 FROM ple_private.job WHERE job_id = '50000000-0000-0000-0000-000000000051')
       OR EXISTS (SELECT 1 FROM ple_private.question_statistics_observation_receipt WHERE automated_grading_receipt_id = '50000000-0000-0000-0000-000000000081')
       OR NOT EXISTS (SELECT 1 FROM ple_private.assignment_attempt WHERE assignment_id = '40000000-0000-0000-0000-000000000002') THEN
        RAISE EXCEPTION 'Unrelease did not preserve and delete the expected roots';
    END IF;
END $$;
SET LOCAL ROLE ple_data_owner;
DO $$
BEGIN
    IF (SELECT accepted_graded_attempt_count FROM ple_data.question_revision_statistics WHERE question_id = 'ABCDEF0' AND revision_number = 1) <> 1
       OR (SELECT correct_count FROM ple_data.question_revision_statistics WHERE question_id = 'ABCDEF0' AND revision_number = 1) <> 1 THEN
        RAISE EXCEPTION 'Unrelease did not rebuild Question Revision statistics from survivors';
    END IF;
END $$;
SET LOCAL ROLE ple_audit_owner;
DO $$
DECLARE event_json jsonb;
BEGIN
    SELECT to_jsonb(event) INTO event_json FROM ple_audit.assignment_unrelease_event AS event;
    IF (SELECT count(*) FROM ple_audit.assignment_unrelease_event) <> 1
       OR event_json ? 'student_record_id' OR event_json ? 'student_response'
       OR event_json ? 'points_earned'
       OR (event_json ->> 'assignment_attempt_count') <> '1' THEN
        RAISE EXCEPTION 'Unrelease audit receipt is not one redacted aggregate event';
    END IF;
END $$;
COMMIT;

-- A repeated lifecycle action remains a conflict and cannot create a second
-- audit record or alter the surviving statistics.
BEGIN;
SET LOCAL ROLE ple_app;
SELECT set_config('ple.session_account_id', '10000000-0000-0000-0000-000000000001', true);
DO $$
BEGIN
    PERFORM * FROM ple_api.unrelease_assignment(1, 1, 2, 'Unrelease target');
    RAISE EXCEPTION 'Unreleased Assignment accepted a second Unrelease';
EXCEPTION WHEN object_not_in_prerequisite_state THEN NULL;
END $$;
RESET ROLE;
SET LOCAL ROLE ple_audit_owner;
DO $$
BEGIN
    IF (SELECT count(*) FROM ple_audit.assignment_unrelease_event) <> 1 THEN
        RAISE EXCEPTION 'rejected lifecycle repeat wrote an audit event';
    END IF;
END $$;
COMMIT;

SELECT 'unrelease_connected_oracle_pass';
