-- Complete database-owned Live Demo teaching graph.  The ordinary Pilot
-- publisher supplies one exact Question Revision per fixed source checksum.
-- This file deliberately creates no demo-only database concept.

SELECT set_config(
    'ple.installation_pilot_question_publications',
    :'pilot_question_publications', true
) AS ignored \gset
SELECT set_config(
    'ple.installation_pilot_publication_session_id',
    :'pilot_publication_session_id', true
) AS ignored \gset
SELECT set_config(
    'ple.installation_live_demo_blueprint_reference',
    :'live_demo_blueprint_reference', true
) AS ignored \gset
SELECT set_config(
    'ple.installation_live_demo_blueprint_assessment_reference',
    :'live_demo_blueprint_assessment_reference', true
) AS ignored \gset

SET LOCAL ROLE ple_private_owner;

DO $$
DECLARE expected_count integer := 8; actual_count integer; distinct_count integer; invalid boolean;
BEGIN
    WITH input AS (
        SELECT publication.key AS slug,
               publication.value ->> 'sourceSha256' AS source_checksum,
               publication.value -> 'questionRevision' ->> 'questionId' AS rendered_question_id,
               replace(publication.value -> 'questionRevision' ->> 'questionId', '-', '') AS question_id,
               (publication.value -> 'questionRevision' ->> 'revisionNumber')::integer AS revision_number
          FROM jsonb_each(
              current_setting('ple.installation_pilot_question_publications')::jsonb
          ) AS publication(key, value)
    )
    SELECT count(*), count(DISTINCT question_id), COALESCE(bool_or(
        slug NOT IN (
                'genetics-disorders-webwork-mc',
                'genetics-disorders-webwork-matching',
                'genetics-disorders-ple-question-json-mc',
                'genetics-disorders-ple-question-json-matching',
                'biochemistry-functional-groups-webwork-mc',
                'biochemistry-functional-groups-webwork-matching',
                'biochemistry-functional-groups-ple-question-json-mc',
                'biochemistry-functional-groups-ple-question-json-matching'
        ) OR source_checksum !~ '^[0-9a-f]{64}$'
          OR rendered_question_id !~ '^[0-9A-HJKMNP-TV-Z]{4}-[0-9A-HJKMNP-TV-Z]{4}$'
          OR question_id !~ '^[0-9A-HJKMNP-TV-Z]{8}$'
          OR revision_number <= 0
          OR NOT EXISTS (
              SELECT 1 FROM ple_private.question_revision_source_binding AS binding
               WHERE binding.question_id = input.question_id
                 AND binding.revision_number = input.revision_number
                 AND binding.source_object_checksum = input.source_checksum
          )
    ), false)
      INTO actual_count, distinct_count, invalid
      FROM input;
    IF actual_count <> expected_count OR distinct_count <> expected_count OR invalid THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Pilot publication mapping does not resolve the approved exact Question Revisions';
    END IF;
END
$$;

RESET ROLE;

SET LOCAL ROLE ple_api_owner;

DO $$
DECLARE
    blueprint_reference bigint := current_setting(
        'ple.installation_live_demo_blueprint_reference'
    )::bigint;
    expected_blueprint_assessment_reference uuid := current_setting(
        'ple.installation_live_demo_blueprint_assessment_reference'
    )::uuid;
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM ple_data.blueprint_course
         WHERE reference_number = blueprint_reference
           AND owner_account_id = '00000000-0000-0000-0000-000000000101'
           AND availability = 'available'
           AND current_blueprint_revision_number = 1
           AND EXISTS (
               SELECT 1 FROM ple_data.blueprint_revision_assessment AS revision_assessment
                WHERE revision_assessment.blueprint_course_reference_number = blueprint_reference
                  AND revision_assessment.blueprint_revision_number = 1
                  AND revision_assessment.blueprint_assessment_reference
                      = expected_blueprint_assessment_reference
           )
    ) THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Live Demo Blueprint Revision 1 is unavailable';
    END IF;
    IF EXISTS (SELECT 1 FROM ple_data.course_instance WHERE course_id = '00000000-0000-0000-0000-000000000220')
       AND NOT EXISTS (
           SELECT 1 FROM ple_data.course_instance AS course
            WHERE course.course_id = '00000000-0000-0000-0000-000000000220'
              AND course.assigned_instructor_account_id = '00000000-0000-0000-0000-000000000101'
              AND course.course_short_name = 'BCHM 301'
              AND course.course_long_name = 'Biochemistry 301: Proteins and Peptides'
              AND course.term_starts_on = date '2026-08-24'
              AND course.term_ends_on = date '2026-12-11'
              AND course.source_kind = 'adopted'
              AND course.blueprint_course_reference_number = blueprint_reference
              AND course.blueprint_revision_number = 1
       ) THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Live Demo Course root conflicts with existing product data';
    END IF;
END
$$;

DO $$
DECLARE
    course_reference bigint;
    blueprint_reference bigint := current_setting(
        'ple.installation_live_demo_blueprint_reference'
    )::bigint;
BEGIN
    INSERT INTO ple_data.course_instance (
        course_id, source_kind, blueprint_course_reference_number, blueprint_revision_number,
        assigned_instructor_account_id, course_short_name, course_long_name,
        term_starts_on, term_ends_on, created_at
    ) VALUES (
        '00000000-0000-0000-0000-000000000220', 'adopted', blueprint_reference, 1,
        '00000000-0000-0000-0000-000000000101', 'BCHM 301',
        'Biochemistry 301: Proteins and Peptides', date '2026-08-24', date '2026-12-11',
        clock_timestamp()
    ) ON CONFLICT (course_id) DO NOTHING RETURNING reference_number INTO course_reference;
    IF course_reference IS NOT NULL THEN
        INSERT INTO ple_data.course_origin (
            course_origin_id, course_id, source_kind, blueprint_course_reference_number,
            blueprint_revision_number, source_course_id, created_at
        ) VALUES (
            '00000000-0000-0000-0000-000000000221', '00000000-0000-0000-0000-000000000220',
            'adopted', blueprint_reference, 1, NULL, clock_timestamp()
        );
        INSERT INTO ple_data.course_membership (membership_id, course_id, account_id, role, joined_at)
        VALUES ('00000000-0000-0000-0000-000000000222', '00000000-0000-0000-0000-000000000220',
                '00000000-0000-0000-0000-000000000101', 'instructor', clock_timestamp());
        PERFORM ple_audit.record_course_instance_creation_event(
            '00000000-0000-0000-0000-000000000223', '00000000-0000-0000-0000-000000000220',
            course_reference, 'adopted', blueprint_reference, 1,
            '00000000-0000-0000-0000-000000000101',
            '00000000-0000-0000-0000-000000000101', clock_timestamp()
        );
    ELSE
        SELECT reference_number INTO course_reference FROM ple_data.course_instance
         WHERE course_id = '00000000-0000-0000-0000-000000000220';
    END IF;
END
$$;

INSERT INTO ple_private.course_roster_profile (
    course_roster_profile_id, course_id, student_account_id, roster_id, created_at
) VALUES
    ('00000000-0000-0000-0000-000000000231', '00000000-0000-0000-0000-000000000220', '00000000-0000-0000-0000-000000000102', 'BIO301-MARY', clock_timestamp()),
    ('00000000-0000-0000-0000-000000000232', '00000000-0000-0000-0000-000000000220', '00000000-0000-0000-0000-000000000103', 'BIO301-JACK', clock_timestamp()),
    ('00000000-0000-0000-0000-000000000233', '00000000-0000-0000-0000-000000000220', '00000000-0000-0000-0000-000000000104', 'BIO301-AVERY', clock_timestamp())
ON CONFLICT (course_roster_profile_id) DO NOTHING;

INSERT INTO ple_private.course_invitation (
    invitation_id, course_id, target_account_id, membership_role,
    inviting_instructor_account_id, inviting_instructor_role, issued_at, expires_at
) VALUES
    ('00000000-0000-0000-0000-000000000241', '00000000-0000-0000-0000-000000000220', '00000000-0000-0000-0000-000000000102', 'student', '00000000-0000-0000-0000-000000000101', 'instructor', clock_timestamp(), clock_timestamp() + interval '365 days'),
    ('00000000-0000-0000-0000-000000000242', '00000000-0000-0000-0000-000000000220', '00000000-0000-0000-0000-000000000103', 'student', '00000000-0000-0000-0000-000000000101', 'instructor', clock_timestamp(), clock_timestamp() + interval '365 days'),
    ('00000000-0000-0000-0000-000000000243', '00000000-0000-0000-0000-000000000220', '00000000-0000-0000-0000-000000000104', 'student', '00000000-0000-0000-0000-000000000101', 'instructor', clock_timestamp(), clock_timestamp() + interval '365 days')
ON CONFLICT (invitation_id) DO NOTHING;

INSERT INTO ple_data.student_record (student_record_id, course_id, student_account_id, created_at) VALUES
    ('00000000-0000-0000-0000-000000000251', '00000000-0000-0000-0000-000000000220', '00000000-0000-0000-0000-000000000102', clock_timestamp()),
    ('00000000-0000-0000-0000-000000000252', '00000000-0000-0000-0000-000000000220', '00000000-0000-0000-0000-000000000103', clock_timestamp()),
    ('00000000-0000-0000-0000-000000000253', '00000000-0000-0000-0000-000000000220', '00000000-0000-0000-0000-000000000104', clock_timestamp())
ON CONFLICT (student_record_id) DO NOTHING;

INSERT INTO ple_data.course_membership (
    membership_id, course_id, account_id, role, student_record_id, joined_at
) VALUES
    ('00000000-0000-0000-0000-000000000261', '00000000-0000-0000-0000-000000000220', '00000000-0000-0000-0000-000000000102', 'student', '00000000-0000-0000-0000-000000000251', clock_timestamp()),
    ('00000000-0000-0000-0000-000000000262', '00000000-0000-0000-0000-000000000220', '00000000-0000-0000-0000-000000000103', 'student', '00000000-0000-0000-0000-000000000252', clock_timestamp()),
    ('00000000-0000-0000-0000-000000000263', '00000000-0000-0000-0000-000000000220', '00000000-0000-0000-0000-000000000104', 'student', '00000000-0000-0000-0000-000000000253', clock_timestamp())
ON CONFLICT (membership_id) DO NOTHING;

SET LOCAL ROLE ple_data_owner;

DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM ple_data.assessment WHERE assessment_id = '00000000-0000-0000-0000-000000000270')
       AND (
           NOT EXISTS (
               SELECT 1 FROM ple_data.assessment
                WHERE assessment_id = '00000000-0000-0000-0000-000000000270'
                  AND course_id = '00000000-0000-0000-0000-000000000220'
                  AND assessment_status = 'released'
                  AND assessment_title = 'Chapter 1 Pilot Practice'
           )
           OR (SELECT count(*) FROM ple_data.assessment_entry
                WHERE assessment_id = '00000000-0000-0000-0000-000000000270') <> 4
           OR EXISTS (
               WITH input AS (
                   SELECT replace(value -> 'questionRevision' ->> 'questionId', '-', '') AS question_id,
                          (value -> 'questionRevision' ->> 'revisionNumber')::integer AS revision_number,
                          row_number() OVER (ORDER BY array_position(ARRAY[
                              'genetics-disorders-ple-question-json-mc', 'genetics-disorders-ple-question-json-matching',
                              'biochemistry-functional-groups-ple-question-json-mc', 'biochemistry-functional-groups-ple-question-json-matching'
                          ], key)) - 1 AS position
                     FROM jsonb_each(current_setting('ple.installation_pilot_question_publications')::jsonb)
                    WHERE key IN (
                        'genetics-disorders-ple-question-json-mc', 'genetics-disorders-ple-question-json-matching',
                        'biochemistry-functional-groups-ple-question-json-mc', 'biochemistry-functional-groups-ple-question-json-matching'
                    )
               )
               SELECT 1 FROM input
               LEFT JOIN ple_data.assessment_entry AS entry
                 ON entry.assessment_id = '00000000-0000-0000-0000-000000000270'
                AND entry.authored_position = input.position
                AND entry.entry_kind = 'fixed_question'
                AND entry.question_id = input.question_id
                AND entry.question_revision_number = input.revision_number
               WHERE entry.assessment_entry_id IS NULL
           )
       ) THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Live Demo Assessment conflicts with existing product data';
    END IF;
END
$$;

DO $$
DECLARE
    new_assessment_id uuid;
    expected_blueprint_assessment_reference uuid := current_setting(
        'ple.installation_live_demo_blueprint_assessment_reference'
    )::uuid;
BEGIN
    INSERT INTO ple_data.assessment (
        assessment_id, course_id, source_blueprint_course_reference_number,
        source_blueprint_revision_number, source_blueprint_assessment_reference,
        created_at, updated_at, assessment_title, assessment_instructions,
        assessment_attempt_time_limit_seconds, late_work_rule, assessment_completion_rule,
        assessment_attempt_grade_rule, assessment_attempt_continuation_rule,
        question_pool_reuse_rule, question_variation_rule, assessment_attempt_resume_rule,
        assessment_question_display_rule, assessment_navigation_rule,
        assessment_question_order_rule, feedback_score, feedback_per_item_correctness,
        feedback_submitted_response, feedback_question_feedback, feedback_question_answer,
        feedback_question_answer_explanation, feedback_class_statistics
    ) VALUES (
        '00000000-0000-0000-0000-000000000270',
        '00000000-0000-0000-0000-000000000220',
        (SELECT blueprint_course_reference_number FROM ple_data.course_instance WHERE course_id = '00000000-0000-0000-0000-000000000220'),
        1, expected_blueprint_assessment_reference, clock_timestamp(), clock_timestamp(),
        'Chapter 1 Pilot Practice', 'Complete the four reviewed Chapter 1 practice questions.',
        1800, 'accept', 'answer_all', 'highest', 'unlimited', 'reuse_selection',
        'new_variation', 'resumable', 'one_question_at_a_time', 'free_navigation',
        'authored_order', 'after_submit', 'after_submit', 'after_submit', 'after_submit',
        'never', 'never', 'never'
    ) ON CONFLICT (assessment_id) DO NOTHING
    RETURNING assessment_id INTO new_assessment_id;
    IF new_assessment_id IS NULL THEN
        RETURN;
    END IF;
    INSERT INTO ple_data.assessment_entry (
        assessment_entry_id, assessment_id, authored_position, entry_kind, scoring_rule,
        question_id, question_revision_number, points_possible
    )
    SELECT ('00000000-0000-0000-0000-00000000028' || input_position)::uuid,
           new_assessment_id, input_position::integer,
           'fixed_question', 'normal', input.question_id, input.revision_number, 1
      FROM (
          SELECT publication.key AS slug,
                 replace(publication.value -> 'questionRevision' ->> 'questionId', '-', '') AS question_id,
                 (publication.value -> 'questionRevision' ->> 'revisionNumber')::integer AS revision_number,
                 row_number() OVER (ORDER BY array_position(ARRAY[
                     'genetics-disorders-ple-question-json-mc', 'genetics-disorders-ple-question-json-matching',
                     'biochemistry-functional-groups-ple-question-json-mc', 'biochemistry-functional-groups-ple-question-json-matching'
                 ], publication.key)) - 1 AS input_position
            FROM jsonb_each(current_setting('ple.installation_pilot_question_publications')::jsonb)
                 AS publication(key, value)
           WHERE publication.key IN (
               'genetics-disorders-ple-question-json-mc', 'genetics-disorders-ple-question-json-matching',
               'biochemistry-functional-groups-ple-question-json-mc', 'biochemistry-functional-groups-ple-question-json-matching'
           )
      ) AS input;
    PERFORM ple_data.validate_assessment_release(new_assessment_id);
    UPDATE ple_data.assessment SET assessment_status = 'released',
        assessment_edit_number = assessment_edit_number + 1, updated_at = clock_timestamp()
     WHERE assessment_id = new_assessment_id;
END
$$;

RESET ROLE;

SET LOCAL ROLE ple_api_owner;

INSERT INTO ple_private.course_invitation_event (
    course_invitation_event_id, invitation_id, event_kind, performed_by_account_id, occurred_at, reason
) VALUES
    ('00000000-0000-0000-0000-000000000291', '00000000-0000-0000-0000-000000000241', 'accepted', '00000000-0000-0000-0000-000000000102', clock_timestamp(), 'student accepted Course Invitation'),
    ('00000000-0000-0000-0000-000000000292', '00000000-0000-0000-0000-000000000242', 'accepted', '00000000-0000-0000-0000-000000000103', clock_timestamp(), 'student accepted Course Invitation'),
    ('00000000-0000-0000-0000-000000000293', '00000000-0000-0000-0000-000000000243', 'accepted', '00000000-0000-0000-0000-000000000104', clock_timestamp(), 'student accepted Course Invitation')
ON CONFLICT (course_invitation_event_id) DO NOTHING;

RESET ROLE;

SET LOCAL ROLE ple_audit_owner;
INSERT INTO ple_audit.course_roster_event (
    course_roster_event_id, course_id, student_account_id, acting_account_id, event_kind, occurred_at
) VALUES
    ('00000000-0000-0000-0000-000000000281', '00000000-0000-0000-0000-000000000220', '00000000-0000-0000-0000-000000000102', '00000000-0000-0000-0000-000000000101', 'invitation_created', clock_timestamp()),
    ('00000000-0000-0000-0000-000000000282', '00000000-0000-0000-0000-000000000220', '00000000-0000-0000-0000-000000000103', '00000000-0000-0000-0000-000000000101', 'invitation_created', clock_timestamp()),
    ('00000000-0000-0000-0000-000000000283', '00000000-0000-0000-0000-000000000220', '00000000-0000-0000-0000-000000000104', '00000000-0000-0000-0000-000000000101', 'invitation_created', clock_timestamp()),
    ('00000000-0000-0000-0000-000000000284', '00000000-0000-0000-0000-000000000220', '00000000-0000-0000-0000-000000000102', '00000000-0000-0000-0000-000000000102', 'invitation_claimed', clock_timestamp()),
    ('00000000-0000-0000-0000-000000000285', '00000000-0000-0000-0000-000000000220', '00000000-0000-0000-0000-000000000103', '00000000-0000-0000-0000-000000000103', 'invitation_claimed', clock_timestamp()),
    ('00000000-0000-0000-0000-000000000286', '00000000-0000-0000-0000-000000000220', '00000000-0000-0000-0000-000000000104', '00000000-0000-0000-0000-000000000104', 'invitation_claimed', clock_timestamp())
ON CONFLICT DO NOTHING;
RESET ROLE;

SET LOCAL ROLE ple_private_owner;
DO $$
DECLARE v_session_id uuid := current_setting('ple.installation_pilot_publication_session_id')::uuid;
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM ple_private.authenticated_session
         WHERE authenticated_session.session_id = v_session_id
           AND authenticated_session.account_id = '00000000-0000-0000-0000-000000000101'
           AND authenticated_session.product_role = 'instructor'
    ) THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Pilot publication session is not the supplied temporary Instructor session';
    END IF;
    UPDATE ple_private.authenticated_session
       SET revoked_at = coalesce(revoked_at, clock_timestamp())
     WHERE authenticated_session.session_id = v_session_id;
END
$$;
RESET ROLE;

-- This installation-data layer intentionally ends before Assessment Attempts, response
-- presentation, submission, grading, and statistics: each has external or
-- worker-owned effects and remains with its existing owner path.
