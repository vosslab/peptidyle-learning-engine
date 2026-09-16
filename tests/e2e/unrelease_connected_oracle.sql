-- One connected PostgreSQL acceptance for Assessment Unrelease.  The leased
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
VALUES ('ABCDXEF0', clock_timestamp());
INSERT INTO ple_data.question_revision (
    question_id, revision_number, backend, question_type, published_at
) VALUES ('ABCDXEF0', 1, 'ple', 'multipleChoice', clock_timestamp());
SET LOCAL ROLE ple_private_owner;
INSERT INTO ple_private.object_record (
    object_id, object_address, object_storage_area, object_data_class, sha256,
    size_bytes, media_type, created_at
) VALUES (
    '20000000-0000-0000-0000-000000000010',
    '{"kind":"questionSource","questionRevision":{"questionId":"ABCDXEF0","revisionNumber":1},"object":"20000000-0000-0000-0000-000000000010"}'::jsonb,
    'private-content', 'question-source', decode(repeat('10', 32), 'hex'), 1,
    'application/json', clock_timestamp()
);
INSERT INTO ple_private.question_revision_source_binding (
    question_id, revision_number, backend, question_format, source_object_id,
    source_object_checksum, created_at
) VALUES (
    'ABCDXEF0', 1, 'ple', 'pleQuestionJson',
    '20000000-0000-0000-0000-000000000010', repeat('10', 32), clock_timestamp()
);
SET LOCAL ROLE ple_api_owner;
INSERT INTO ple_data.blueprint_course (
    blueprint_id, owner_account_id, short_name, long_name,
    metadata_etag, created_at
) VALUES (
    '20000000-0000-0000-0000-000000000001',
    '10000000-0000-0000-0000-000000000001', 'UNR-1',
    'Unrelease acceptance Blueprint', '20000000-0000-0000-0000-000000000004',
    clock_timestamp()
);
INSERT INTO ple_data.blueprint_course_revision (
    blueprint_course_reference_number, blueprint_revision_number,
    content, content_checksum, saved_at
) VALUES ((SELECT reference_number FROM ple_data.blueprint_course
            WHERE blueprint_id = '20000000-0000-0000-0000-000000000001'), 1, '{}'::jsonb,
    decode(repeat('a', 64), 'hex'), clock_timestamp());
INSERT INTO ple_data.blueprint_revision_module (
    blueprint_course_reference_number, blueprint_revision_number,
    blueprint_module_reference, module_position
) VALUES ((SELECT reference_number FROM ple_data.blueprint_course
            WHERE blueprint_id = '20000000-0000-0000-0000-000000000001'),
    1, '20000000-0000-0000-0000-000000000002', 1);
INSERT INTO ple_data.blueprint_revision_assessment (
    blueprint_course_reference_number, blueprint_revision_number,
    blueprint_module_reference, blueprint_assessment_reference, assessment_position
) VALUES ((SELECT reference_number FROM ple_data.blueprint_course
            WHERE blueprint_id = '20000000-0000-0000-0000-000000000001'),
    1, '20000000-0000-0000-0000-000000000002',
    '20000000-0000-0000-0000-000000000003', 1);
INSERT INTO ple_data.blueprint_revision_event (
    blueprint_course_reference_number, blueprint_revision_number,
    actor_account_id, request_checksum, occurred_at
) VALUES ((SELECT reference_number FROM ple_data.blueprint_course
            WHERE blueprint_id = '20000000-0000-0000-0000-000000000001'),
    1, '10000000-0000-0000-0000-000000000001',
    decode(repeat('b', 64), 'hex'), clock_timestamp());
INSERT INTO ple_data.course_instance (
    course_id, source_kind, blueprint_course_reference_number,
    blueprint_revision_number, assigned_instructor_account_id,
    assigned_instructor_role, course_short_name, course_long_name,
    term_starts_on, term_ends_on, created_at
) VALUES (
    '30000000-0000-0000-0000-000000000001', 'adopted',
    (SELECT reference_number FROM ple_data.blueprint_course
      WHERE blueprint_id = '20000000-0000-0000-0000-000000000001'), 1,
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
INSERT INTO ple_data.assessment (
    assessment_id, course_id, origin_kind, assessment_type,
    source_blueprint_course_reference_number, source_blueprint_revision_number,
    source_blueprint_assessment_reference, created_at, updated_at,
    assessment_title, assessment_instructions, late_work_rule,
    assessment_attempt_grade_rule, question_variation_rule,
    assessment_attempt_resume_rule,
    assessment_question_display_rule, assessment_navigation_rule,
    assessment_question_order_rule, feedback_score, feedback_per_item_correctness,
    feedback_submitted_response, feedback_question_answer,
    feedback_question_answer_explanation, feedback_class_statistics, assessment_status
) VALUES
    ('40000000-0000-0000-0000-000000000001',
     '30000000-0000-0000-0000-000000000001', 'adopted', 'regular_assignment',
     (SELECT reference_number FROM ple_data.blueprint_course
       WHERE blueprint_id = '20000000-0000-0000-0000-000000000001'), 1,
     '20000000-0000-0000-0000-000000000003', clock_timestamp(), clock_timestamp(),
     'Unrelease target', '', 'accept', 'latest',
     'reuse_variation', 'resumable', 'all_questions',
     'free_navigation', 'authored_order', 'after_submit', 'after_submit',
     'after_submit', 'after_submit', 'after_submit', 'never', 'released'),
    ('40000000-0000-0000-0000-000000000002',
     '30000000-0000-0000-0000-000000000001', 'adopted', 'regular_assignment',
     (SELECT reference_number FROM ple_data.blueprint_course
       WHERE blueprint_id = '20000000-0000-0000-0000-000000000001'), 1,
     '20000000-0000-0000-0000-000000000003', clock_timestamp(), clock_timestamp(),
     'Statistics survivor', '', 'accept', 'latest',
     'reuse_variation', 'resumable', 'all_questions',
     'free_navigation', 'authored_order', 'after_submit', 'after_submit',
     'after_submit', 'after_submit', 'after_submit', 'never', 'released'),
    ('40000000-0000-0000-0000-000000000003',
     '30000000-0000-0000-0000-000000000001', 'adopted', 'regular_assignment',
     (SELECT reference_number FROM ple_data.blueprint_course
       WHERE blueprint_id = '20000000-0000-0000-0000-000000000001'), 1,
     '20000000-0000-0000-0000-000000000003', clock_timestamp(), clock_timestamp(),
     'Unrelease lock race', '', 'accept', 'latest',
     'reuse_variation', 'resumable', 'all_questions',
     'free_navigation', 'authored_order', 'after_submit', 'after_submit',
     'after_submit', 'after_submit', 'after_submit', 'never', 'released');
INSERT INTO ple_data.assessment_entry (
    assessment_entry_id, assessment_id, authored_position, entry_kind,
    availability, scoring_rule, question_id, question_revision_number, points_possible
) VALUES
    ('40000000-0000-0000-0000-000000000011', '40000000-0000-0000-0000-000000000001',
     0, 'fixed_question', 'available', 'normal', 'ABCDXEF0', 1, 1),
    ('40000000-0000-0000-0000-000000000012', '40000000-0000-0000-0000-000000000002',
     0, 'fixed_question', 'available', 'normal', 'ABCDXEF0', 1, 1),
    ('40000000-0000-0000-0000-000000000013', '40000000-0000-0000-0000-000000000003',
     0, 'fixed_question', 'available', 'normal', 'ABCDXEF0', 1, 1);

-- Start both Attempts through the ordinary restricted path.  The fixture then
-- adds the lower-level submission/grading receipts needed to exercise the
-- destructive closure and statistics rebuild.
SET LOCAL ROLE ple_app;
SELECT set_config('ple.session_account_id', '10000000-0000-0000-0000-000000000002', true);
SELECT * FROM ple_api.start_assessment_attempt(
    '50000000-0000-0000-0000-000000000001',
    '30000000-0000-0000-0000-000000000002',
    '40000000-0000-0000-0000-000000000001',
    '[]'::jsonb,
    '[{"issued_question_id":"50000000-0000-0000-0000-000000000011","assessment_entry_id":"40000000-0000-0000-0000-000000000011","issued_position":0,"question_id":"ABCDXEF0","revision_number":1}]'::jsonb
);
SELECT * FROM ple_api.start_assessment_attempt(
    '50000000-0000-0000-0000-000000000002',
    '30000000-0000-0000-0000-000000000002',
    '40000000-0000-0000-0000-000000000002',
    '[]'::jsonb,
    '[{"issued_question_id":"50000000-0000-0000-0000-000000000012","assessment_entry_id":"40000000-0000-0000-0000-000000000012","issued_position":0,"question_id":"ABCDXEF0","revision_number":1}]'::jsonb
);
RESET ROLE;
SET LOCAL ROLE ple_private_owner;
INSERT INTO ple_private.question_attempt (
    question_attempt_id, issued_question_id, issued_at, submitted_at,
    question_attempt_state, backend_name, backend_version,
    grader_name, grader_version, rendered_question_sha256, issued_capability
) VALUES
    ('50000000-0000-0000-0000-000000000021', '50000000-0000-0000-0000-000000000011',
     transaction_timestamp(), transaction_timestamp(), 'submission_accepted', 'ple', '1',
     'ple', '1', decode(repeat('b', 64), 'hex'), 'not_applicable'),
    ('50000000-0000-0000-0000-000000000022', '50000000-0000-0000-0000-000000000012',
     transaction_timestamp(), transaction_timestamp(), 'submission_accepted', 'ple', '1',
     'ple', '1', decode(repeat('d', 64), 'hex'), 'not_applicable');
INSERT INTO ple_private.assessment_attempt_saved_response (question_attempt_id, student_response, saved_at)
VALUES ('50000000-0000-0000-0000-000000000021', '{}'::jsonb, clock_timestamp()),
       ('50000000-0000-0000-0000-000000000022', '{}'::jsonb, clock_timestamp());
INSERT INTO ple_private.question_submission (submission_id, question_attempt_id, submitted_at, student_response)
VALUES ('50000000-0000-0000-0000-000000000031', '50000000-0000-0000-0000-000000000021', transaction_timestamp(), '{}'::jsonb),
       ('50000000-0000-0000-0000-000000000032', '50000000-0000-0000-0000-000000000022', transaction_timestamp(), '{}'::jsonb);
INSERT INTO ple_private.assessment_submission (assessment_submission_id, assessment_attempt_id, submitted_at, finalization_kind, authorized_by_account_id, receipt)
VALUES ('50000000-0000-0000-0000-000000000041', '50000000-0000-0000-0000-000000000001', clock_timestamp(), 'student', '10000000-0000-0000-0000-000000000002', '{}'::jsonb),
       ('50000000-0000-0000-0000-000000000042', '50000000-0000-0000-0000-000000000002', clock_timestamp(), 'student', '10000000-0000-0000-0000-000000000002', '{}'::jsonb);
INSERT INTO ple_private.question_submission_grading (
    question_submission_grading_id, submission_id, grading_state, created_at, completed_at
) VALUES
    ('50000000-0000-0000-0000-000000000061', '50000000-0000-0000-0000-000000000031', 'graded', clock_timestamp(), clock_timestamp()),
    ('50000000-0000-0000-0000-000000000062', '50000000-0000-0000-0000-000000000032', 'graded', clock_timestamp(), clock_timestamp());
INSERT INTO ple_private.grading_result (grading_result_id, submission_id, question_submission_grading_id, question_attempt_id, normalized_credit, recorded_at)
VALUES ('50000000-0000-0000-0000-000000000071', '50000000-0000-0000-0000-000000000031', '50000000-0000-0000-0000-000000000061', '50000000-0000-0000-0000-000000000021', 1, clock_timestamp()),
       ('50000000-0000-0000-0000-000000000072', '50000000-0000-0000-0000-000000000032', '50000000-0000-0000-0000-000000000062', '50000000-0000-0000-0000-000000000022', 1, clock_timestamp());

SET LOCAL ROLE ple_audit_owner;
INSERT INTO ple_audit.automated_grading_receipt (automated_grading_receipt_id, question_submission_grading_id, grading_result_id, committed_at, automated_grading_receipt_checksum)
VALUES ('50000000-0000-0000-0000-000000000081', '50000000-0000-0000-0000-000000000061', '50000000-0000-0000-0000-000000000071', clock_timestamp(), decode(repeat('e', 64), 'hex')),
       ('50000000-0000-0000-0000-000000000082', '50000000-0000-0000-0000-000000000062', '50000000-0000-0000-0000-000000000072', clock_timestamp(), decode(repeat('f', 64), 'hex'));
SET LOCAL ROLE ple_private_owner;
INSERT INTO ple_private.question_statistics_observation_receipt (automated_grading_receipt_id, question_attempt_id, question_id, revision_number, correct, observed_at)
VALUES ('50000000-0000-0000-0000-000000000081', '50000000-0000-0000-0000-000000000021', 'ABCDXEF0', 1, true, clock_timestamp()),
       ('50000000-0000-0000-0000-000000000082', '50000000-0000-0000-0000-000000000022', 'ABCDXEF0', 1, true, clock_timestamp());
SET LOCAL ROLE ple_data_owner;
SELECT ple_data.rebuild_question_revision_statistics('ABCDXEF0', 1, clock_timestamp());
COMMIT;

-- Error cases are real public calls.  Each verifies the rejected transaction
-- leaves current state, retained evidence, statistics, and audit unchanged.
BEGIN;
SET LOCAL ROLE ple_data_owner;
SELECT set_config('ple.test_unrelease_course_reference', public_reference, true)
  FROM ple_data.course_instance
 WHERE course_id = '30000000-0000-0000-0000-000000000001';
SELECT set_config(
    'ple.test_unrelease_target_reference', public_reference, true
) FROM ple_data.assessment
 WHERE assessment_id = '40000000-0000-0000-0000-000000000001';
SET LOCAL ROLE ple_app;
SELECT set_config('ple.session_account_id', '10000000-0000-0000-0000-000000000002', true);
DO $$
BEGIN
    PERFORM * FROM ple_api.read_assessment_unrelease_impact(
        current_setting('ple.test_unrelease_course_reference'),
        current_setting('ple.test_unrelease_target_reference')
    );
    RAISE EXCEPTION 'foreign unrelease impact was enumerated';
EXCEPTION WHEN insufficient_privilege THEN NULL;
END $$;
SELECT set_config('ple.session_account_id', '10000000-0000-0000-0000-000000000001', true);
DO $$
BEGIN
    PERFORM * FROM ple_api.unrelease_assessment(
        current_setting('ple.test_unrelease_course_reference'),
        current_setting('ple.test_unrelease_target_reference'),
        2, 'Unrelease target'
    );
    RAISE EXCEPTION 'stale Unrelease Edit Number was accepted';
EXCEPTION WHEN serialization_failure THEN NULL;
END $$;
DO $$
BEGIN
    PERFORM * FROM ple_api.unrelease_assessment(
        current_setting('ple.test_unrelease_course_reference'),
        current_setting('ple.test_unrelease_target_reference'),
        1, 'wrong title'
    );
    RAISE EXCEPTION 'wrong Unrelease confirmation title was accepted';
EXCEPTION WHEN invalid_parameter_value THEN NULL;
END $$;
RESET ROLE;
SET LOCAL ROLE ple_data_owner;
DO $$
BEGIN
    IF (SELECT assessment_status FROM ple_data.assessment
         WHERE assessment_id = '40000000-0000-0000-0000-000000000001') <> 'released'
       OR (SELECT assessment_edit_number FROM ple_data.assessment
           WHERE assessment_id = '40000000-0000-0000-0000-000000000001') <> 1 THEN
        RAISE EXCEPTION 'a rejected Unrelease changed current Assessment state';
    END IF;
END $$;
SET LOCAL ROLE ple_private_owner;
DO $$
BEGIN
    IF (SELECT count(*) FROM ple_private.assessment_attempt WHERE assessment_id = '40000000-0000-0000-0000-000000000001') <> 1 THEN
        RAISE EXCEPTION 'a rejected Unrelease changed Student Work';
    END IF;
END $$;
SET LOCAL ROLE ple_audit_owner;
DO $$
BEGIN
    IF (SELECT count(*) FROM ple_audit.assessment_unrelease_event) <> 0 THEN
        RAISE EXCEPTION 'a rejected Unrelease wrote audit evidence';
    END IF;
END $$;
COMMIT;

-- The accepted public transition returns aggregate impact only, removes the
-- rooted closure, retains the current Assessment/entry and shared Revision,
-- rebuilds the shared Revision statistics from the survivor, and records a
-- redacted aggregate audit receipt.
BEGIN;
SET LOCAL ROLE ple_data_owner;
SELECT set_config('ple.test_unrelease_course_reference', public_reference, true)
  FROM ple_data.course_instance
 WHERE course_id = '30000000-0000-0000-0000-000000000001';
SELECT set_config(
    'ple.test_unrelease_target_reference', public_reference, true
) FROM ple_data.assessment
 WHERE assessment_id = '40000000-0000-0000-0000-000000000001';
SET LOCAL ROLE ple_app;
SELECT set_config('ple.session_account_id', '10000000-0000-0000-0000-000000000001', true);
DO $$
DECLARE result record;
BEGIN
    SELECT * INTO result FROM ple_api.unrelease_assessment(
        current_setting('ple.test_unrelease_course_reference'),
        current_setting('ple.test_unrelease_target_reference'),
        1, 'Unrelease target'
    );
    IF result.assessment_status <> 'unreleased' OR result.assessment_edit_number <> 2
       OR result.assessment_attempt_count <> 1 OR result.question_submission_count <> 1
       OR result.assessment_submission_count <> 1 OR result.grading_result_count <> 1 THEN
        RAISE EXCEPTION 'accepted Unrelease returned an incorrect aggregate receipt';
    END IF;
END $$;
RESET ROLE;
SET LOCAL ROLE ple_data_owner;
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM ple_data.assessment
                   WHERE assessment_id = '40000000-0000-0000-0000-000000000001'
                     AND assessment_status = 'unreleased'
                     AND assessment_edit_number = 2)
       OR NOT EXISTS (SELECT 1 FROM ple_data.assessment_entry WHERE assessment_id = '40000000-0000-0000-0000-000000000001')
       OR NOT EXISTS (SELECT 1 FROM ple_data.question_revision WHERE question_id = 'ABCDXEF0' AND revision_number = 1) THEN
        RAISE EXCEPTION 'Unrelease did not preserve current Assessment or shared Question state';
    END IF;
END $$;
SET LOCAL ROLE ple_private_owner;
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM ple_private.assessment_attempt WHERE assessment_id = '40000000-0000-0000-0000-000000000001')
       OR EXISTS (SELECT 1 FROM ple_private.question_submission WHERE submission_id = '50000000-0000-0000-0000-000000000031')
       OR EXISTS (SELECT 1 FROM ple_private.assessment_submission WHERE assessment_submission_id = '50000000-0000-0000-0000-000000000041')
       OR EXISTS (SELECT 1 FROM ple_private.grading_result WHERE grading_result_id = '50000000-0000-0000-0000-000000000071')
       OR EXISTS (SELECT 1 FROM ple_private.question_statistics_observation_receipt WHERE automated_grading_receipt_id = '50000000-0000-0000-0000-000000000081')
       OR NOT EXISTS (SELECT 1 FROM ple_private.assessment_attempt WHERE assessment_id = '40000000-0000-0000-0000-000000000002') THEN
        RAISE EXCEPTION 'Unrelease did not preserve and delete the expected roots';
    END IF;
END $$;
SET LOCAL ROLE ple_data_owner;
DO $$
BEGIN
    IF (SELECT accepted_graded_attempt_count FROM ple_data.question_revision_statistics WHERE question_id = 'ABCDXEF0' AND revision_number = 1) <> 1
       OR (SELECT correct_count FROM ple_data.question_revision_statistics WHERE question_id = 'ABCDXEF0' AND revision_number = 1) <> 1 THEN
        RAISE EXCEPTION 'Unrelease did not rebuild Question Revision statistics from survivors';
    END IF;
END $$;
SET LOCAL ROLE ple_audit_owner;
DO $$
DECLARE event_json jsonb;
BEGIN
    SELECT to_jsonb(event) INTO event_json FROM ple_audit.assessment_unrelease_event AS event;
    IF (SELECT count(*) FROM ple_audit.assessment_unrelease_event) <> 1
       OR event_json ? 'student_record_id' OR event_json ? 'student_response'
       OR event_json ? 'points_earned'
       OR (event_json ->> 'assessment_attempt_count') <> '1' THEN
        RAISE EXCEPTION 'Unrelease audit receipt is not one redacted aggregate event';
    END IF;
END $$;
COMMIT;

-- A repeated lifecycle action remains a conflict and cannot create a second
-- audit record or alter the surviving statistics.
BEGIN;
SET LOCAL ROLE ple_data_owner;
SELECT set_config('ple.test_unrelease_course_reference', public_reference, true)
  FROM ple_data.course_instance
 WHERE course_id = '30000000-0000-0000-0000-000000000001';
SELECT set_config(
    'ple.test_unrelease_target_reference', public_reference, true
) FROM ple_data.assessment
 WHERE assessment_id = '40000000-0000-0000-0000-000000000001';
SET LOCAL ROLE ple_app;
SELECT set_config('ple.session_account_id', '10000000-0000-0000-0000-000000000001', true);
DO $$
BEGIN
    PERFORM * FROM ple_api.unrelease_assessment(
        current_setting('ple.test_unrelease_course_reference'),
        current_setting('ple.test_unrelease_target_reference'),
        2, 'Unrelease target'
    );
    RAISE EXCEPTION 'Unreleased Assessment accepted a second Unrelease';
EXCEPTION WHEN object_not_in_prerequisite_state THEN NULL;
END $$;
RESET ROLE;
SET LOCAL ROLE ple_audit_owner;
DO $$
BEGIN
    IF (SELECT count(*) FROM ple_audit.assessment_unrelease_event) <> 1 THEN
        RAISE EXCEPTION 'rejected lifecycle repeat wrote an audit event';
    END IF;
END $$;
COMMIT;

SELECT 'unrelease_connected_oracle_pass';
