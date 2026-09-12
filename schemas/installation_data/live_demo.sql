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
          OR rendered_question_id !~ '^[0-9A-HJKMNP-TV-Z]{3}-[0-9A-HJKMNP-TV-Z]{4}$'
          OR question_id !~ '^[0-9A-HJKMNP-TV-Z]{7}$'
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
DECLARE blueprint_exists boolean;
BEGIN
    SELECT EXISTS (
        SELECT 1 FROM ple_data.blueprint_course
         WHERE blueprint_id = '00000000-0000-0000-0000-000000000210'
    ) INTO blueprint_exists;
    IF blueprint_exists AND NOT EXISTS (
        SELECT 1 FROM ple_data.blueprint_course
         WHERE blueprint_id = '00000000-0000-0000-0000-000000000210'
           AND owner_account_id = '00000000-0000-0000-0000-000000000101'
           AND availability = 'available'
    ) THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Live Demo Blueprint root conflicts with existing product data';
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
              AND course.blueprint_course_reference_number = (
                  SELECT reference_number FROM ple_data.blueprint_course
                   WHERE blueprint_id = '00000000-0000-0000-0000-000000000210'
              )
              AND course.blueprint_revision_number = 1
       ) THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Live Demo Course root conflicts with existing product data';
    END IF;
END
$$;

INSERT INTO ple_data.blueprint_course (blueprint_id, owner_account_id, created_at)
VALUES ('00000000-0000-0000-0000-000000000210',
        '00000000-0000-0000-0000-000000000101', clock_timestamp())
ON CONFLICT (blueprint_id) DO NOTHING;

INSERT INTO ple_data.blueprint_draft (
    blueprint_course_reference_number, content, content_checksum, updated_at
)
WITH pilot_question_input AS (
    SELECT publication.key AS slug,
           replace(publication.value -> 'questionRevision' ->> 'questionId', '-', '') AS question_id,
           (publication.value -> 'questionRevision' ->> 'revisionNumber')::integer AS revision_number
      FROM jsonb_each(:'pilot_question_publications'::jsonb) AS publication(key, value)
), content AS (
    SELECT jsonb_build_object(
        'title', 'Biochemistry 301: Proteins and Peptides',
        'modules', jsonb_build_array(jsonb_build_object(
            'blueprint_module_reference', '00000000-0000-0000-0000-000000000211',
            'label', 'Chapter 1 reviewed practice',
            'assignments', jsonb_build_array(jsonb_build_object(
                'blueprint_assignment_reference', '00000000-0000-0000-0000-000000000212',
                'content', jsonb_build_object(
                    'title', 'Chapter 1 Pilot Practice',
                    'instructions', 'Complete the eight reviewed Chapter 1 practice questions.',
                    'entries', (
                        SELECT jsonb_agg(jsonb_build_object(
                            'kind', 'fixed',
                            'question_revision', jsonb_build_object(
                                'questionId', input.question_id,
                                'revisionNumber', input.revision_number
                            )
                        ) ORDER BY array_position(ARRAY[
                            'genetics-disorders-webwork-mc', 'genetics-disorders-webwork-matching',
                            'genetics-disorders-ple-question-json-mc', 'genetics-disorders-ple-question-json-matching',
                            'biochemistry-functional-groups-webwork-mc', 'biochemistry-functional-groups-webwork-matching',
                            'biochemistry-functional-groups-ple-question-json-mc', 'biochemistry-functional-groups-ple-question-json-matching'
                        ], input.slug))
                        FROM pilot_question_input AS input
                    )
                )
            ))
        ))) AS value
)
SELECT course.reference_number, content.value,
       sha256(convert_to(content.value::text, 'UTF8')), clock_timestamp()
  FROM ple_data.blueprint_course AS course
 CROSS JOIN content
 CROSS JOIN LATERAL (
     SELECT set_config(
         'ple.installation_expected_blueprint_content', content.value::text, true
     )
 ) AS expected_content(ignored)
 WHERE course.blueprint_id = '00000000-0000-0000-0000-000000000210'
ON CONFLICT (blueprint_course_reference_number) DO NOTHING;

DO $$
DECLARE
    expected_content jsonb := current_setting('ple.installation_expected_blueprint_content')::jsonb;
    expected_checksum bytea := sha256(convert_to(
        current_setting('ple.installation_expected_blueprint_content'), 'UTF8'
    ));
    blueprint_reference bigint;
BEGIN
    SELECT reference_number INTO blueprint_reference
      FROM ple_data.blueprint_course
     WHERE blueprint_id = '00000000-0000-0000-0000-000000000210';
    IF NOT EXISTS (
        SELECT 1 FROM ple_data.blueprint_draft
         WHERE blueprint_course_reference_number = blueprint_reference
           AND content = expected_content
           AND content_checksum = expected_checksum
    ) OR EXISTS (
        SELECT 1 FROM ple_data.blueprint_course_revision
         WHERE blueprint_course_reference_number = blueprint_reference
           AND blueprint_revision_number = 1
           AND (content <> expected_content OR content_checksum <> expected_checksum)
    ) THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Live Demo Blueprint content conflicts with existing product data';
    END IF;
END
$$;

INSERT INTO ple_data.blueprint_draft_question_pin
SELECT course.reference_number, pins.content_path, pins.question_id, pins.question_revision_number
  FROM ple_data.blueprint_course AS course
  JOIN ple_data.blueprint_draft AS draft
    ON draft.blueprint_course_reference_number = course.reference_number
 CROSS JOIN LATERAL ple_data.blueprint_content_question_pins(draft.content) AS pins
 WHERE course.blueprint_id = '00000000-0000-0000-0000-000000000210'
ON CONFLICT DO NOTHING;

INSERT INTO ple_data.blueprint_draft_module
SELECT course.reference_number, module.blueprint_module_reference, module.module_position
  FROM ple_data.blueprint_course AS course
  JOIN ple_data.blueprint_draft AS draft
    ON draft.blueprint_course_reference_number = course.reference_number
 CROSS JOIN LATERAL ple_data.blueprint_content_modules(draft.content) AS module
 WHERE course.blueprint_id = '00000000-0000-0000-0000-000000000210'
ON CONFLICT DO NOTHING;

INSERT INTO ple_data.blueprint_draft_assignment
SELECT course.reference_number, assignment.blueprint_module_reference,
       assignment.blueprint_assignment_reference, assignment.assignment_position
  FROM ple_data.blueprint_course AS course
  JOIN ple_data.blueprint_draft AS draft
    ON draft.blueprint_course_reference_number = course.reference_number
 CROSS JOIN LATERAL ple_data.blueprint_content_assignments(draft.content) AS assignment
 WHERE course.blueprint_id = '00000000-0000-0000-0000-000000000210'
ON CONFLICT DO NOTHING;

INSERT INTO ple_data.blueprint_course_revision (
    blueprint_course_reference_number, blueprint_revision_number, title,
    content, content_checksum, published_at
)
SELECT course.reference_number, 1, draft.content ->> 'title',
       draft.content, draft.content_checksum, clock_timestamp()
  FROM ple_data.blueprint_course AS course
  JOIN ple_data.blueprint_draft AS draft
    ON draft.blueprint_course_reference_number = course.reference_number
 WHERE course.blueprint_id = '00000000-0000-0000-0000-000000000210'
ON CONFLICT DO NOTHING;

INSERT INTO ple_data.blueprint_revision_question_pin
SELECT revision.blueprint_course_reference_number, revision.blueprint_revision_number,
       pin.content_path, pin.question_id, pin.question_revision_number
  FROM ple_data.blueprint_course_revision AS revision
  JOIN ple_data.blueprint_course AS course
    ON course.reference_number = revision.blueprint_course_reference_number
  JOIN ple_data.blueprint_draft_question_pin AS pin
    ON pin.blueprint_course_reference_number = revision.blueprint_course_reference_number
 WHERE course.blueprint_id = '00000000-0000-0000-0000-000000000210'
   AND revision.blueprint_revision_number = 1
ON CONFLICT DO NOTHING;

INSERT INTO ple_data.blueprint_revision_module
SELECT revision.blueprint_course_reference_number, revision.blueprint_revision_number,
       module.blueprint_module_reference, module.module_position
  FROM ple_data.blueprint_course_revision AS revision
  JOIN ple_data.blueprint_course AS course
    ON course.reference_number = revision.blueprint_course_reference_number
  JOIN ple_data.blueprint_draft_module AS module
    ON module.blueprint_course_reference_number = revision.blueprint_course_reference_number
 WHERE course.blueprint_id = '00000000-0000-0000-0000-000000000210'
   AND revision.blueprint_revision_number = 1
ON CONFLICT DO NOTHING;

INSERT INTO ple_data.blueprint_revision_assignment
SELECT revision.blueprint_course_reference_number, revision.blueprint_revision_number,
       assignment.blueprint_module_reference, assignment.blueprint_assignment_reference,
       assignment.assignment_position
  FROM ple_data.blueprint_course_revision AS revision
  JOIN ple_data.blueprint_course AS course
    ON course.reference_number = revision.blueprint_course_reference_number
  JOIN ple_data.blueprint_draft_assignment AS assignment
    ON assignment.blueprint_course_reference_number = revision.blueprint_course_reference_number
 WHERE course.blueprint_id = '00000000-0000-0000-0000-000000000210'
   AND revision.blueprint_revision_number = 1
ON CONFLICT DO NOTHING;

INSERT INTO ple_data.blueprint_publication_event (
    blueprint_course_reference_number, blueprint_revision_number, actor_account_id,
    request_checksum, occurred_at
)
SELECT course.reference_number, 1, '00000000-0000-0000-0000-000000000101',
       decode('8e441bad293e233213221aa4d56ed2b502c89cb7d555269f94fc453a1cbe0f6c', 'hex'),
       clock_timestamp()
  FROM ple_data.blueprint_course AS course
 WHERE course.blueprint_id = '00000000-0000-0000-0000-000000000210'
ON CONFLICT DO NOTHING;

INSERT INTO ple_data.blueprint_availability_event (
    blueprint_course_reference_number, actor_account_id, availability, edit_number, occurred_at
)
SELECT course.reference_number, '00000000-0000-0000-0000-000000000101',
       'available', 1, clock_timestamp()
  FROM ple_data.blueprint_course AS course
 WHERE course.blueprint_id = '00000000-0000-0000-0000-000000000210'
ON CONFLICT DO NOTHING;

DO $$
DECLARE course_reference bigint; blueprint_reference bigint;
BEGIN
    SELECT reference_number INTO blueprint_reference FROM ple_data.blueprint_course
     WHERE blueprint_id = '00000000-0000-0000-0000-000000000210';
    INSERT INTO ple_data.course_instance (
        course_id, blueprint_course_reference_number, blueprint_revision_number,
        assigned_instructor_account_id, course_short_name, course_long_name,
        term_starts_on, term_ends_on, created_at
    ) VALUES (
        '00000000-0000-0000-0000-000000000220', blueprint_reference, 1,
        '00000000-0000-0000-0000-000000000101', 'BCHM 301',
        'Biochemistry 301: Proteins and Peptides', date '2026-08-24', date '2026-12-11',
        clock_timestamp()
    ) ON CONFLICT (course_id) DO NOTHING RETURNING reference_number INTO course_reference;
    IF course_reference IS NOT NULL THEN
        INSERT INTO ple_data.course_origin VALUES (
            '00000000-0000-0000-0000-000000000221', '00000000-0000-0000-0000-000000000220',
            blueprint_reference, 1, NULL, clock_timestamp()
        );
        INSERT INTO ple_data.course_membership (membership_id, course_id, account_id, role, joined_at)
        VALUES ('00000000-0000-0000-0000-000000000222', '00000000-0000-0000-0000-000000000220',
                '00000000-0000-0000-0000-000000000101', 'instructor', clock_timestamp());
        PERFORM ple_audit.record_course_instance_creation_event(
            '00000000-0000-0000-0000-000000000223', '00000000-0000-0000-0000-000000000220',
            course_reference, blueprint_reference, 1,
            '00000000-0000-0000-0000-000000000101',
            '00000000-0000-0000-0000-000000000101', clock_timestamp()
        );
    ELSE
        SELECT reference_number INTO course_reference FROM ple_data.course_instance
         WHERE course_id = '00000000-0000-0000-0000-000000000220';
    END IF;
    PERFORM set_config('ple.installation_course_reference_number', course_reference::text, true);
    PERFORM set_config('ple.installation_blueprint_reference_number', blueprint_reference::text, true);
END
$$;

INSERT INTO ple_private.course_roster_profile (
    course_roster_profile_id, course_id, student_account_id, roster_email, roster_id, created_at
) VALUES
    ('00000000-0000-0000-0000-000000000231', '00000000-0000-0000-0000-000000000220', '00000000-0000-0000-0000-000000000102', 'mary.okafor@live-demo.invalid', 'BIO301-MARY', clock_timestamp()),
    ('00000000-0000-0000-0000-000000000232', '00000000-0000-0000-0000-000000000220', '00000000-0000-0000-0000-000000000103', 'jack.nguyen@live-demo.invalid', 'BIO301-JACK', clock_timestamp()),
    ('00000000-0000-0000-0000-000000000233', '00000000-0000-0000-0000-000000000220', '00000000-0000-0000-0000-000000000104', 'avery.thompson@live-demo.invalid', 'BIO301-AVERY', clock_timestamp())
ON CONFLICT (course_roster_profile_id) DO NOTHING;

INSERT INTO ple_private.course_invitation (
    invitation_id, course_id, target_account_id, membership_role, issued_at, expires_at
) VALUES
    ('00000000-0000-0000-0000-000000000241', '00000000-0000-0000-0000-000000000220', '00000000-0000-0000-0000-000000000102', 'student', clock_timestamp(), clock_timestamp() + interval '365 days'),
    ('00000000-0000-0000-0000-000000000242', '00000000-0000-0000-0000-000000000220', '00000000-0000-0000-0000-000000000103', 'student', clock_timestamp(), clock_timestamp() + interval '365 days'),
    ('00000000-0000-0000-0000-000000000243', '00000000-0000-0000-0000-000000000220', '00000000-0000-0000-0000-000000000104', 'student', clock_timestamp(), clock_timestamp() + interval '365 days')
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
    IF EXISTS (SELECT 1 FROM ple_data.assignment WHERE assignment_id = '00000000-0000-0000-0000-000000000270')
       AND (
           NOT EXISTS (
               SELECT 1 FROM ple_data.assignment
                WHERE assignment_id = '00000000-0000-0000-0000-000000000270'
                  AND course_id = '00000000-0000-0000-0000-000000000220'
                  AND assignment_status = 'released'
                  AND assignment_title = 'Chapter 1 Pilot Practice'
           )
           OR (SELECT count(*) FROM ple_data.assignment_entry
                WHERE assignment_id = '00000000-0000-0000-0000-000000000270') <> 8
           OR EXISTS (
               WITH input AS (
                   SELECT replace(value -> 'questionRevision' ->> 'questionId', '-', '') AS question_id,
                          (value -> 'questionRevision' ->> 'revisionNumber')::integer AS revision_number,
                          row_number() OVER (ORDER BY array_position(ARRAY[
                              'genetics-disorders-webwork-mc', 'genetics-disorders-webwork-matching',
                              'genetics-disorders-ple-question-json-mc', 'genetics-disorders-ple-question-json-matching',
                              'biochemistry-functional-groups-webwork-mc', 'biochemistry-functional-groups-webwork-matching',
                              'biochemistry-functional-groups-ple-question-json-mc', 'biochemistry-functional-groups-ple-question-json-matching'
                          ], key)) - 1 AS position
                     FROM jsonb_each(current_setting('ple.installation_pilot_question_publications')::jsonb)
               )
               SELECT 1 FROM input
               LEFT JOIN ple_data.assignment_entry AS entry
                 ON entry.assignment_id = '00000000-0000-0000-0000-000000000270'
                AND entry.authored_position = input.position
                AND entry.entry_kind = 'fixed_question'
                AND entry.question_id = input.question_id
                AND entry.question_revision_number = input.revision_number
               WHERE entry.assignment_entry_id IS NULL
           )
       ) THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Live Demo Assignment conflicts with existing product data';
    END IF;
END
$$;

DO $$
DECLARE new_assignment_id uuid;
BEGIN
    INSERT INTO ple_data.assignment (
        assignment_id, course_id, source_blueprint_course_reference_number,
        source_blueprint_revision_number, source_blueprint_assignment_reference,
        created_at, updated_at, assignment_title, assignment_instructions,
        assignment_attempt_time_limit_seconds, late_work_rule, assignment_completion_rule,
        assignment_attempt_grade_rule, assignment_attempt_continuation_rule,
        question_pool_reuse_rule, question_variation_rule, assignment_attempt_resume_rule,
        assignment_question_display_rule, assignment_navigation_rule,
        assignment_question_order_rule, feedback_score, feedback_per_item_correctness,
        feedback_submitted_response, feedback_question_feedback, feedback_question_answer,
        feedback_question_answer_explanation, feedback_class_statistics
    ) VALUES (
        '00000000-0000-0000-0000-000000000270',
        '00000000-0000-0000-0000-000000000220',
        (SELECT blueprint_course_reference_number FROM ple_data.course_instance WHERE course_id = '00000000-0000-0000-0000-000000000220'),
        1, '00000000-0000-0000-0000-000000000212', clock_timestamp(), clock_timestamp(),
        'Chapter 1 Pilot Practice', 'Complete the eight reviewed Chapter 1 practice questions.',
        1800, 'accept', 'answer_all', 'highest', 'unlimited', 'reuse_selection',
        'new_variation', 'resumable', 'one_question_at_a_time', 'free_navigation',
        'authored_order', 'after_submit', 'after_submit', 'after_submit', 'after_submit',
        'never', 'never', 'never'
    ) ON CONFLICT (assignment_id) DO NOTHING
    RETURNING assignment_id INTO new_assignment_id;
    IF new_assignment_id IS NULL THEN
        RETURN;
    END IF;
    INSERT INTO ple_data.assignment_entry (
        assignment_entry_id, assignment_id, authored_position, entry_kind, scoring_rule,
        question_id, question_revision_number, points_possible
    )
    SELECT ('00000000-0000-0000-0000-00000000028' || input_position)::uuid,
           new_assignment_id, input_position::integer,
           'fixed_question', 'normal', input.question_id, input.revision_number, 1
      FROM (
          SELECT publication.key AS slug,
                 replace(publication.value -> 'questionRevision' ->> 'questionId', '-', '') AS question_id,
                 (publication.value -> 'questionRevision' ->> 'revisionNumber')::integer AS revision_number,
                 row_number() OVER (ORDER BY array_position(ARRAY[
                     'genetics-disorders-webwork-mc', 'genetics-disorders-webwork-matching',
                     'genetics-disorders-ple-question-json-mc', 'genetics-disorders-ple-question-json-matching',
                     'biochemistry-functional-groups-webwork-mc', 'biochemistry-functional-groups-webwork-matching',
                     'biochemistry-functional-groups-ple-question-json-mc', 'biochemistry-functional-groups-ple-question-json-matching'
                 ], publication.key)) - 1 AS input_position
            FROM jsonb_each(current_setting('ple.installation_pilot_question_publications')::jsonb)
                 AS publication(key, value)
      ) AS input;
    PERFORM ple_data.validate_assignment_release(new_assignment_id);
    UPDATE ple_data.assignment SET assignment_status = 'released',
        assignment_edit_number = assignment_edit_number + 1, updated_at = clock_timestamp()
     WHERE assignment_id = new_assignment_id;
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

-- This installation-data layer intentionally ends before Attempts, response
-- presentation, submission, grading, and statistics: each has external or
-- worker-owned effects and remains with its existing owner path.
