-- One connected PostgreSQL acceptance for Assessment Unrelease.  The leased
-- baseline owner invokes this after it has installed the canonical schema.
-- Fixture writes use schema-owner roles solely to construct otherwise-valid
-- Student Work; the operation under test is always invoked as ple_app with a
-- current-session account identity.
\set ON_ERROR_STOP on

BEGIN;
SET CONSTRAINTS ALL DEFERRED;

SET LOCAL ROLE ple_data_owner;
SELECT encode(ple_private.ensure_assessment_policy_snapshot(
    'Unrelease target', '',
    NULL, NULL, NULL, 90, NULL,
    'accept', 'reuse_variation', 'authored_order',
    'after_submit', 'after_submit', 'after_submit', 'after_submit', 'after_submit', 'never',
    'regular_assignment'
), 'hex') AS target_snapshot_id \gset
SELECT encode(ple_private.ensure_assessment_policy_snapshot(
    'Statistics survivor', '',
    NULL, NULL, NULL, 90, NULL,
    'accept', 'reuse_variation', 'authored_order',
    'after_submit', 'after_submit', 'after_submit', 'after_submit', 'after_submit', 'never',
    'regular_assignment'
), 'hex') AS survivor_snapshot_id \gset
SELECT encode(ple_private.ensure_assessment_policy_snapshot(
    'Unrelease lock race', '',
    NULL, NULL, NULL, 90, NULL,
    'accept', 'reuse_variation', 'authored_order',
    'after_submit', 'after_submit', 'after_submit', 'after_submit', 'after_submit', 'never',
    'regular_assignment'
), 'hex') AS race_snapshot_id \gset

-- Stable fixture identities keep the assertions readable without introducing
-- a product-side test marker or alternate model.
SET LOCAL ROLE ple_private_owner;
INSERT INTO ple_private.account (account_id, product_role, created_at)
VALUES ('U00000009', 'instructor', pg_catalog.transaction_timestamp())
RETURNING account_id AS instructor_id \gset
SELECT set_config('ple.test_unrelease_instructor_id', :'instructor_id', false);
INSERT INTO ple_private.account (account_id, product_role, created_at)
VALUES ('U00000009', 'student', pg_catalog.transaction_timestamp())
RETURNING account_id AS student_id \gset
SELECT set_config('ple.test_unrelease_student_id', :'student_id', false);

SELECT 'ABCD-' || ple_private.crockford_checksum_character('ABCDEFG') || 'EFG' AS question_id \gset
SELECT set_config('ple.test_unrelease_question_id', :'question_id', false);

SET LOCAL ROLE ple_data_owner;
INSERT INTO ple_data.content_discipline (content_discipline_id, name)
VALUES ('20000000-0000-0000-0000-00000000cc01', 'Unrelease fixture discipline')
ON CONFLICT (content_discipline_id) DO NOTHING;
SELECT content_discipline_id AS discipline_id
  FROM ple_data.content_discipline
 WHERE content_discipline_id = '20000000-0000-0000-0000-00000000cc01' \gset
INSERT INTO ple_data.published_question (published_question_id, created_at)
VALUES (:'question_id', pg_catalog.transaction_timestamp());
INSERT INTO ple_data.question_revision (
    published_question_id, revision_number, backend, question_type, published_at
) VALUES (:'question_id', 1, 'ple', 'multipleChoice', pg_catalog.transaction_timestamp());

SET LOCAL ROLE ple_private_owner;
INSERT INTO ple_private.object_record (
    object_record_id, object_address, object_storage_area, object_data_class, sha256,
    size_bytes, media_type, created_at
) VALUES (
    '20000000-0000-0000-0000-000000000010',
    jsonb_build_object(
        'kind', 'questionSource',
        'questionRevision', jsonb_build_object(
            'questionId', :'question_id',
            'revisionNumber', 1
        ),
        'object', '20000000-0000-0000-0000-000000000010'::uuid
    ),
    'private-content', 'question-source', decode(repeat('10', 32), 'hex'), 1,
    'application/json', pg_catalog.transaction_timestamp()
);
INSERT INTO ple_private.question_revision_source_binding (
    published_question_id, revision_number, backend, question_format,
    source_object_record_id, source_object_checksum, created_at
) VALUES (
    :'question_id', 1, 'ple', 'pleQuestionJson',
    '20000000-0000-0000-0000-000000000010', repeat('10', 32), pg_catalog.transaction_timestamp()
);

SET LOCAL ROLE ple_api_owner;
INSERT INTO ple_data.course_instance (
    course_instance_id, source_kind, course_short_name, course_long_name,
    content_discipline_id, tags, term_starts_on, term_ends_on, created_at
) VALUES (
    'CINR00000' || ple_private.crockford_checksum_character('CINR00000'),
    'empty', 'UNR-1', 'Unrelease acceptance Course',
    :'discipline_id',
    ARRAY[]::text[], current_date, current_date + 1,
    pg_catalog.transaction_timestamp()
)
RETURNING course_instance_id AS course_id \gset
SELECT set_config('ple.test_unrelease_course_id', :'course_id', false);

INSERT INTO ple_data.course_membership (
    course_membership_id, course_instance_id, account_id, role, joined_at
) VALUES (
    '30000000-0000-0000-0000-000000000003',
    :'course_id', :'instructor_id', 'instructor', pg_catalog.transaction_timestamp()
);
INSERT INTO ple_data.student_record (
    student_record_id, course_instance_id, student_account_id, created_at
) VALUES (
    '30000000-0000-0000-0000-000000000002',
    :'course_id', :'student_id', pg_catalog.transaction_timestamp()
)
RETURNING student_record_id AS record_id \gset
INSERT INTO ple_data.course_membership (
    course_membership_id, course_instance_id, account_id, role, student_record_id, joined_at
) VALUES (
    '30000000-0000-0000-0000-000000000004',
    :'course_id', :'student_id', 'student', :'record_id', pg_catalog.transaction_timestamp()
);

-- Target and survivor use the same immutable Question Revision.  The survivor
-- makes retained anonymous statistics distinguishable from remaining private
-- Student Work after the target is Unreleased.
SET LOCAL ROLE ple_data_owner;
INSERT INTO ple_data.assessment (
    assessment_id, course_instance_id, origin_kind, created_at, updated_at,
    assessment_type, assessment_policy_snapshot_id, assessment_status
) VALUES (
    'ANR00001' || ple_private.crockford_checksum_character('ANR00001'),
    :'course_id', 'direct',
    pg_catalog.transaction_timestamp(), pg_catalog.transaction_timestamp(),
    'regular_assignment', decode(:'target_snapshot_id', 'hex'), 'released'
)
RETURNING assessment_id AS target_assessment_id \gset
SELECT set_config('ple.test_unrelease_target_assessment_id', :'target_assessment_id', false);
INSERT INTO ple_data.assessment (
    assessment_id, course_instance_id, origin_kind, created_at, updated_at,
    assessment_type, assessment_policy_snapshot_id, assessment_status
) VALUES (
    'ANR00002' || ple_private.crockford_checksum_character('ANR00002'),
    :'course_id', 'direct',
    pg_catalog.transaction_timestamp(), pg_catalog.transaction_timestamp(),
    'regular_assignment', decode(:'survivor_snapshot_id', 'hex'), 'released'
)
RETURNING assessment_id AS survivor_assessment_id \gset
SELECT set_config('ple.test_unrelease_survivor_assessment_id', :'survivor_assessment_id', false);
INSERT INTO ple_data.assessment (
    assessment_id, course_instance_id, origin_kind, created_at, updated_at,
    assessment_type, assessment_policy_snapshot_id, assessment_status
) VALUES (
    'ANR00003' || ple_private.crockford_checksum_character('ANR00003'),
    :'course_id', 'direct',
    pg_catalog.transaction_timestamp(), pg_catalog.transaction_timestamp(),
    'regular_assignment', decode(:'race_snapshot_id', 'hex'), 'released'
)
RETURNING assessment_id AS race_assessment_id \gset

INSERT INTO ple_data.assessment_entry (
    assessment_entry_id, assessment_id, authored_position, entry_kind,
    availability, scoring_rule
) VALUES
    ('40000000-0000-0000-0000-000000000011', :'target_assessment_id',
     0, 'fixed_question', 'available', 'normal'),
    ('40000000-0000-0000-0000-000000000012', :'survivor_assessment_id',
     0, 'fixed_question', 'available', 'normal'),
    ('40000000-0000-0000-0000-000000000013', :'race_assessment_id',
     0, 'fixed_question', 'available', 'normal');
INSERT INTO ple_data.assessment_entry_question (
    assessment_entry_id, assessment_id, published_question_id,
    question_revision_number, points_possible
) VALUES
    ('40000000-0000-0000-0000-000000000011', :'target_assessment_id', :'question_id', 1, 1),
    ('40000000-0000-0000-0000-000000000012', :'survivor_assessment_id', :'question_id', 1, 1),
    ('40000000-0000-0000-0000-000000000013', :'race_assessment_id', :'question_id', 1, 1);

-- Start both Attempts through the ordinary restricted path.  The fixture then
-- adds the lower-level submission/grading receipts needed to exercise the
-- destructive closure and retained anonymous statistics.
SET LOCAL ROLE ple_app;
SELECT set_config('ple.session_account_id', :'student_id', true);
SELECT * FROM ple_api.start_assessment_attempt(
    '50000000-0000-0000-0000-000000000001',
    :'record_id',
    :'target_assessment_id',
    '[]'::jsonb,
    jsonb_build_array(jsonb_build_object(
        'issued_question_id', '50000000-0000-0000-0000-000000000011',
        'assessment_entry_id', '40000000-0000-0000-0000-000000000011',
        'issued_position', 0,
        'published_question_id', :'question_id',
        'revision_number', 1
    ))
);
SELECT * FROM ple_api.start_assessment_attempt(
    '50000000-0000-0000-0000-000000000002',
    :'record_id',
    :'survivor_assessment_id',
    '[]'::jsonb,
    jsonb_build_array(jsonb_build_object(
        'issued_question_id', '50000000-0000-0000-0000-000000000012',
        'assessment_entry_id', '40000000-0000-0000-0000-000000000012',
        'issued_position', 0,
        'published_question_id', :'question_id',
        'revision_number', 1
    ))
);
RESET ROLE;
SET LOCAL ROLE ple_private_owner;
INSERT INTO ple_private.question_attempt (
    course_instance_id, question_attempt_id, issued_question_id, issued_at,
    delivery_toolchain_id, rendered_question_sha256
)
SELECT issued.course_instance_id, '50000000-0000-0000-0000-000000000021',
       issued.issued_question_id, pg_catalog.transaction_timestamp(),
       ple_private.ensure_delivery_toolchain('ple', '1', NULL, NULL, 'ple', '1', 'not_applicable'),
       decode(repeat('b', 64), 'hex')
  FROM ple_private.issued_question AS issued
 WHERE issued.issued_question_id = '50000000-0000-0000-0000-000000000011';
INSERT INTO ple_private.question_attempt (
    course_instance_id, question_attempt_id, issued_question_id, issued_at,
    delivery_toolchain_id, rendered_question_sha256
)
SELECT issued.course_instance_id, '50000000-0000-0000-0000-000000000022',
       issued.issued_question_id, pg_catalog.transaction_timestamp(),
       ple_private.ensure_delivery_toolchain('ple', '1', NULL, NULL, 'ple', '1', 'not_applicable'),
       decode(repeat('d', 64), 'hex')
  FROM ple_private.issued_question AS issued
 WHERE issued.issued_question_id = '50000000-0000-0000-0000-000000000012';
INSERT INTO ple_private.assessment_attempt_saved_response (
    course_instance_id, question_attempt_id, student_response, saved_at
)
SELECT attempt.course_instance_id, attempt.question_attempt_id, '{}'::jsonb, pg_catalog.transaction_timestamp()
  FROM ple_private.question_attempt AS attempt
 WHERE attempt.question_attempt_id IN (
     '50000000-0000-0000-0000-000000000021',
     '50000000-0000-0000-0000-000000000022'
 );
INSERT INTO ple_private.assessment_submission (
    course_instance_id, assessment_submission_id, assessment_attempt_id, submitted_at,
    authorized_by_account_id
) VALUES
    (:'course_id', '50000000-0000-0000-0000-000000000041',
     '50000000-0000-0000-0000-000000000001', pg_catalog.transaction_timestamp(), :'student_id'),
    (:'course_id', '50000000-0000-0000-0000-000000000042',
     '50000000-0000-0000-0000-000000000002', pg_catalog.transaction_timestamp(), :'student_id');
UPDATE ple_private.assessment_attempt_saved_response
   SET finalized_at = pg_catalog.transaction_timestamp(),
       assessment_submission_id = '50000000-0000-0000-0000-000000000041'
 WHERE question_attempt_id = '50000000-0000-0000-0000-000000000021';
UPDATE ple_private.assessment_attempt_saved_response
   SET finalized_at = pg_catalog.transaction_timestamp(),
       assessment_submission_id = '50000000-0000-0000-0000-000000000042'
 WHERE question_attempt_id = '50000000-0000-0000-0000-000000000022';
UPDATE ple_private.question_attempt
   SET finalized_at = pg_catalog.transaction_timestamp()
 WHERE question_attempt_id IN (
     '50000000-0000-0000-0000-000000000021',
     '50000000-0000-0000-0000-000000000022'
 );
INSERT INTO ple_private.grading_result (
    course_instance_id, grading_result_id, question_attempt_id, normalized_credit, recorded_at
) VALUES
    (:'course_id', '50000000-0000-0000-0000-000000000071',
     '50000000-0000-0000-0000-000000000021', 1, pg_catalog.transaction_timestamp()),
    (:'course_id', '50000000-0000-0000-0000-000000000072',
     '50000000-0000-0000-0000-000000000022', 1, pg_catalog.transaction_timestamp());
SET LOCAL ROLE ple_audit_owner;
INSERT INTO ple_audit.automated_grading_receipt (
    automated_grading_receipt_id, course_instance_id, grading_result_id, committed_at,
    automated_grading_receipt_checksum
) VALUES
    ('50000000-0000-0000-0000-000000000081', :'course_id',
     '50000000-0000-0000-0000-000000000071', pg_catalog.transaction_timestamp(), decode(repeat('e', 64), 'hex')),
    ('50000000-0000-0000-0000-000000000082', :'course_id',
     '50000000-0000-0000-0000-000000000072', pg_catalog.transaction_timestamp(), decode(repeat('f', 64), 'hex'));
SET LOCAL ROLE ple_private_owner;
SELECT ple_private.capture_issued_question_statistics_observation(
    :'course_id', '50000000-0000-0000-0000-000000000011',
    '50000000-0000-0000-0000-000000000041', pg_catalog.transaction_timestamp()
);
SELECT ple_private.capture_issued_question_statistics_observation(
    :'course_id', '50000000-0000-0000-0000-000000000012',
    '50000000-0000-0000-0000-000000000042', pg_catalog.transaction_timestamp()
);
-- Repeating a committed Issued Question observation must not count twice.
SELECT ple_private.capture_issued_question_statistics_observation(
    :'course_id', '50000000-0000-0000-0000-000000000011',
    '50000000-0000-0000-0000-000000000041', pg_catalog.transaction_timestamp()
);
SET LOCAL ROLE ple_data_owner;
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM ple_data.question_revision_statistics
         WHERE published_question_id = current_setting('ple.test_unrelease_question_id') AND revision_number = 1
           AND issued_count = 2 AND answered_count = 2 AND correct_count = 2
           AND blank_count = 0
    ) THEN
        RAISE EXCEPTION 'production statistics capture did not count each accepted grade exactly once';
    END IF;
END $$;
COMMIT;

-- Error cases are real public calls.  Each verifies the rejected transaction
-- leaves current state, retained evidence, statistics, and audit unchanged.
BEGIN;
SET LOCAL ROLE ple_app;
SELECT set_config('ple.session_account_id', :'student_id', true);
DO $$
BEGIN
    PERFORM * FROM ple_api.read_assessment_unrelease_impact(
        current_setting('ple.test_unrelease_course_id'), current_setting('ple.test_unrelease_target_assessment_id')
    );
    RAISE EXCEPTION 'foreign unrelease impact was enumerated';
EXCEPTION WHEN insufficient_privilege THEN NULL;
END $$;
SELECT set_config('ple.session_account_id', :'instructor_id', true);
DO $$
BEGIN
    PERFORM * FROM ple_api.unrelease_assessment(
        current_setting('ple.test_unrelease_course_id'), current_setting('ple.test_unrelease_target_assessment_id'),
        2, 'Unrelease target'
    );
    RAISE EXCEPTION 'stale Unrelease Edit Number was accepted';
EXCEPTION WHEN serialization_failure THEN NULL;
END $$;
DO $$
BEGIN
    PERFORM * FROM ple_api.unrelease_assessment(
        current_setting('ple.test_unrelease_course_id'), current_setting('ple.test_unrelease_target_assessment_id'),
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
         WHERE assessment_id = current_setting('ple.test_unrelease_target_assessment_id')) <> 'released'
       OR (SELECT assessment_edit_number FROM ple_data.assessment
           WHERE assessment_id = current_setting('ple.test_unrelease_target_assessment_id')) <> 1 THEN
        RAISE EXCEPTION 'a rejected Unrelease changed current Assessment state';
    END IF;
END $$;
SET LOCAL ROLE ple_private_owner;
DO $$
BEGIN
    IF (SELECT count(*) FROM ple_private.assessment_attempt
         WHERE assessment_id = current_setting('ple.test_unrelease_target_assessment_id')) <> 1 THEN
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
-- retains both anonymous observations despite private evidence deletion, and records a
-- redacted aggregate audit receipt.
BEGIN;
SET LOCAL ROLE ple_app;
SELECT set_config('ple.session_account_id', :'instructor_id', true);
DO $$
DECLARE result record;
BEGIN
    SELECT * INTO result FROM ple_api.unrelease_assessment(
        current_setting('ple.test_unrelease_course_id'), current_setting('ple.test_unrelease_target_assessment_id'),
        1, 'Unrelease target'
    );
    IF result.assessment_status <> 'unreleased' OR result.assessment_edit_number <> 2
       OR result.assessment_attempt_count <> 1 OR result.finalized_saved_response_count <> 1
       OR result.assessment_submission_count <> 1 OR result.grading_result_count <> 1 THEN
        RAISE EXCEPTION 'accepted Unrelease returned an incorrect aggregate receipt';
    END IF;
END $$;
RESET ROLE;
SET LOCAL ROLE ple_data_owner;
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM ple_data.assessment
                   WHERE assessment_id = current_setting('ple.test_unrelease_target_assessment_id')
                     AND assessment_status = 'unreleased'
                     AND assessment_edit_number = 2)
       OR NOT EXISTS (SELECT 1 FROM ple_data.assessment_entry
                       WHERE assessment_id = current_setting('ple.test_unrelease_target_assessment_id'))
       OR NOT EXISTS (SELECT 1 FROM ple_data.question_revision
                       WHERE published_question_id = current_setting('ple.test_unrelease_question_id')
                         AND revision_number = 1) THEN
        RAISE EXCEPTION 'Unrelease did not preserve current Assessment or shared Question state';
    END IF;
END $$;
SET LOCAL ROLE ple_private_owner;
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM ple_private.assessment_attempt
                WHERE assessment_id = current_setting('ple.test_unrelease_target_assessment_id'))
       OR EXISTS (SELECT 1 FROM ple_private.assessment_attempt_saved_response
                   WHERE question_attempt_id = '50000000-0000-0000-0000-000000000021')
       OR EXISTS (SELECT 1 FROM ple_private.assessment_submission
                   WHERE assessment_submission_id = '50000000-0000-0000-0000-000000000041')
       OR EXISTS (SELECT 1 FROM ple_private.grading_result
                   WHERE grading_result_id = '50000000-0000-0000-0000-000000000071')
       OR EXISTS (SELECT 1 FROM ple_private.question_statistics_observation_receipt
                   WHERE issued_question_id = '50000000-0000-0000-0000-000000000011')
       OR NOT EXISTS (SELECT 1 FROM ple_private.question_statistics_observation_receipt
                       WHERE issued_question_id = '50000000-0000-0000-0000-000000000012')
       OR NOT EXISTS (SELECT 1 FROM ple_private.assessment_attempt
                       WHERE assessment_id = current_setting('ple.test_unrelease_survivor_assessment_id')) THEN
        RAISE EXCEPTION 'Unrelease did not preserve and delete the expected roots';
    END IF;
END $$;
SET LOCAL ROLE ple_data_owner;
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM ple_data.question_revision_statistics
         WHERE published_question_id = current_setting('ple.test_unrelease_question_id') AND revision_number = 1
           AND issued_count = 2 AND answered_count = 2 AND correct_count = 2
           AND blank_count = 0
    ) THEN
        RAISE EXCEPTION 'Unrelease changed retained anonymous Question Revision statistics';
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
-- audit record or alter retained statistics.  Recapturing the deleted grade
-- also cannot resurrect private evidence or increment anonymous totals.
BEGIN;
SET LOCAL ROLE ple_app;
SELECT set_config('ple.session_account_id', :'instructor_id', true);
DO $$
BEGIN
    PERFORM * FROM ple_api.unrelease_assessment(
        current_setting('ple.test_unrelease_course_id'), current_setting('ple.test_unrelease_target_assessment_id'),
        2, 'Unrelease target'
    );
    RAISE EXCEPTION 'Unreleased Assessment accepted a second Unrelease';
EXCEPTION WHEN object_not_in_prerequisite_state THEN NULL;
END $$;
RESET ROLE;
SET LOCAL ROLE ple_private_owner;
SELECT ple_private.capture_issued_question_statistics_observation(
    :'course_id', '50000000-0000-0000-0000-000000000011',
    '50000000-0000-0000-0000-000000000041', pg_catalog.transaction_timestamp()
);
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM ple_private.question_statistics_observation_receipt
                WHERE issued_question_id = '50000000-0000-0000-0000-000000000011') THEN
        RAISE EXCEPTION 'recapture resurrected deleted private statistics evidence';
    END IF;
END $$;
SET LOCAL ROLE ple_data_owner;
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM ple_data.question_revision_statistics
         WHERE published_question_id = current_setting('ple.test_unrelease_question_id') AND revision_number = 1
           AND issued_count = 2 AND answered_count = 2 AND correct_count = 2
           AND blank_count = 0
    ) THEN
        RAISE EXCEPTION 'rejected repeat or deleted-grade recapture changed retained statistics';
    END IF;
END $$;
SET LOCAL ROLE ple_audit_owner;
DO $$
BEGIN
    IF (SELECT count(*) FROM ple_audit.assessment_unrelease_event) <> 1 THEN
        RAISE EXCEPTION 'rejected lifecycle repeat wrote an audit event';
    END IF;
END $$;
COMMIT;

SELECT 'unrelease_connected_oracle_pass';
