-- Complete database-owned Live Demo teaching graph.  The ordinary Pilot
-- publisher supplies one exact Question Revision per fixed source checksum.
-- This file deliberately creates no demo-only database concept.

\if :{?pilot_question_publications}
\else
\echo live_demo.sql requires publisher psql variables; skipping teaching graph
\quit
\endif

SELECT set_config(
    'ple.installation_pilot_question_publications',
    :'pilot_question_publications', true
) AS ignored \gset
SELECT set_config(
    'ple.installation_pilot_publication_session_id',
    :'pilot_publication_session_id', true
) AS ignored \gset
SELECT set_config(
    'ple.installation_live_demo_blueprint_course_id',
    :'live_demo_blueprint_course_id', true
) AS ignored \gset
SELECT set_config(
    'ple.installation_live_demo_blueprint_assessment_id',
    :'live_demo_blueprint_assessment_id', true
) AS ignored \gset

SET LOCAL ROLE ple_data_owner;
DO $$
DECLARE
    selected_discipline uuid;
    selected_subject uuid;
BEGIN
    SELECT discipline.content_discipline_id, subject.content_subject_id
      INTO STRICT selected_discipline, selected_subject
      FROM ple_data.content_discipline AS discipline
      JOIN ple_data.content_subject_discipline AS association USING (content_discipline_id)
      JOIN ple_data.content_subject AS subject USING (content_subject_id)
     WHERE discipline.name = 'Biology' AND subject.name = 'Biochemistry';
    PERFORM set_config('ple.installation_live_demo_discipline_uuid', selected_discipline::text, true);
    PERFORM set_config('ple.installation_live_demo_subject_uuid', selected_subject::text, true);
END
$$;

SET LOCAL ROLE ple_private_owner;

DO $$
DECLARE expected_count integer := 8; actual_count integer; distinct_count integer; invalid boolean;
BEGIN
    WITH input AS (
        SELECT publication.key AS slug,
               publication.value ->> 'sourceSha256' AS source_checksum,
               publication.value -> 'questionRevisionTuple' ->> 'questionId' AS published_question_id,
               (publication.value -> 'questionRevisionTuple' ->> 'revisionNumber')::integer AS revision_number
          FROM jsonb_each(
              current_setting('ple.installation_pilot_question_publications')::jsonb
          ) AS publication(key, value)
    )
    SELECT count(*), count(DISTINCT published_question_id), COALESCE(bool_or(
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
          OR published_question_id !~ '^[0-9A-HJKMNP-TV-Z]{4}-[0-9A-HJKMNP-TV-Z]{4}$'
          OR substr(published_question_id, 6, 1) IS DISTINCT FROM
                ple_private.crockford_checksum_character(
                    substr(published_question_id, 1, 4) || substr(published_question_id, 7, 3)
                )
          OR revision_number <= 0
          OR NOT EXISTS (
              SELECT 1 FROM ple_private.question_revision_source_binding AS binding
               WHERE binding.published_question_id = input.published_question_id
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

DO $$
DECLARE
    elena text;
    mary text;
    jack text;
    avery text;
BEGIN
    SELECT email.account_id INTO elena
      FROM ple_private.account_authentication_email AS email
      JOIN ple_private.account AS account
        ON account.account_id = email.account_id
     WHERE email.normalized_email = 'elena.martinez@live-demo.invalid'
       AND account.product_role = 'instructor';
    SELECT email.account_id INTO mary
      FROM ple_private.account_authentication_email AS email
      JOIN ple_private.account AS account
        ON account.account_id = email.account_id
     WHERE email.normalized_email = 'mary.okafor@biology.roosevelt.edu'
       AND account.product_role = 'student';
    SELECT email.account_id INTO jack
      FROM ple_private.account_authentication_email AS email
      JOIN ple_private.account AS account
        ON account.account_id = email.account_id
     WHERE email.normalized_email = 'jack.nguyen@biology.roosevelt.edu'
       AND account.product_role = 'student';
    SELECT email.account_id INTO avery
      FROM ple_private.account_authentication_email AS email
      JOIN ple_private.account AS account
        ON account.account_id = email.account_id
     WHERE email.normalized_email = 'avery.thompson@biology.roosevelt.edu'
       AND account.product_role = 'student';
    IF elena IS NULL OR mary IS NULL OR jack IS NULL OR avery IS NULL THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Live Demo fictional identities are incomplete';
    END IF;
    PERFORM set_config('ple.installation_live_demo_elena_account_id', elena, true);
    PERFORM set_config('ple.installation_live_demo_mary_account_id', mary, true);
    PERFORM set_config('ple.installation_live_demo_jack_account_id', jack, true);
    PERFORM set_config('ple.installation_live_demo_avery_account_id', avery, true);
END
$$;

RESET ROLE;

SET LOCAL ROLE ple_api_owner;

DO $$
DECLARE
    v_blueprint_course_id text;
    elena text;
    expected_blueprint_assessment_id uuid := current_setting(
        'ple.installation_live_demo_blueprint_assessment_id'
    )::uuid;
BEGIN
    -- ASVS 1.2.4 and 8.2.2: the installer carries the canonical Blueprint
    -- Course public ID; never decode it as an internal UUID.
    v_blueprint_course_id := current_setting(
        'ple.installation_live_demo_blueprint_course_id'
    );
    elena := current_setting('ple.installation_live_demo_elena_account_id');
    IF NOT EXISTS (
        SELECT 1 FROM ple_data.blueprint_course
         WHERE blueprint_course_id = v_blueprint_course_id
    ) THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Live Demo Blueprint Course ID is unavailable';
    END IF;
    IF NOT EXISTS (
        SELECT 1 FROM ple_data.blueprint_course
         WHERE blueprint_course_id = v_blueprint_course_id
           AND owner_account_id = elena
           AND availability = 'public'
           AND current_blueprint_revision_number = 1
           AND EXISTS (
               SELECT 1 FROM ple_data.blueprint_revision_assessment AS revision_assessment
                WHERE revision_assessment.blueprint_course_id = v_blueprint_course_id
                  AND revision_assessment.blueprint_revision_number = 1
                  AND revision_assessment.blueprint_assessment_id
                      = expected_blueprint_assessment_id
           )
    ) THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Live Demo Blueprint Revision 1 is unavailable';
    END IF;
    IF EXISTS (SELECT 1 FROM ple_data.course_instance WHERE course_short_name = 'BCHM 301')
       AND NOT EXISTS (
           SELECT 1 FROM ple_data.course_instance AS course
            WHERE course.course_short_name = 'BCHM 301'
              AND course.course_long_name = 'Biochemistry 301: Proteins and Peptides'
              AND course.term_starts_on = date '2026-08-24'
              AND course.term_ends_on = date '2026-12-11'
              AND course.source_kind = 'adopted'
              AND course.blueprint_course_id = v_blueprint_course_id
              AND course.blueprint_revision_number = 1
              AND EXISTS (
                  SELECT 1 FROM ple_data.course_membership AS membership
                   WHERE membership.course_instance_id = course.course_instance_id
                     AND membership.account_id = elena
                     AND membership.role = 'instructor'
              )
       ) THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Live Demo Course root conflicts with existing product data';
    END IF;
END
$$;

DO $$
DECLARE
    v_course_instance_id text;
    v_blueprint_course_id text;
    elena text;
    selected_discipline uuid;
    selected_subject uuid;
    created_at_value timestamptz;
    active_until_value timestamptz;
    course_placeholder constant text := 'CI0000000Y';
BEGIN
    selected_discipline := current_setting('ple.installation_live_demo_discipline_uuid')::uuid;
    selected_subject := current_setting('ple.installation_live_demo_subject_uuid')::uuid;
    v_blueprint_course_id := current_setting(
        'ple.installation_live_demo_blueprint_course_id'
    );
    elena := current_setting('ple.installation_live_demo_elena_account_id');
    IF v_blueprint_course_id IS NULL OR NOT EXISTS (
        SELECT 1 FROM ple_data.blueprint_course
         WHERE blueprint_course_id = v_blueprint_course_id
    ) THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Live Demo Blueprint Course ID is unavailable';
    END IF;
    SELECT course.course_instance_id INTO v_course_instance_id
      FROM ple_data.course_instance AS course
     WHERE course.course_short_name = 'BCHM 301';
    IF v_course_instance_id IS NULL THEN
        created_at_value := pg_catalog.transaction_timestamp();
        active_until_value := (
            (created_at_value AT TIME ZONE 'UTC') + interval '6 months'
        ) AT TIME ZONE 'UTC';
        INSERT INTO ple_data.course_instance (
            course_instance_id, source_kind, blueprint_course_id, blueprint_revision_number,
            course_short_name, course_long_name,
            term_starts_on, term_ends_on, created_at, active_until_at, retention_starts_at,
            content_discipline_id, content_subject_id, content_topic_id, content_subtopic_id, tags
        ) VALUES (
            course_placeholder, 'adopted', v_blueprint_course_id, 1,
            'BCHM 301', 'Biochemistry 301: Proteins and Peptides',
            date '2026-08-24', date '2026-12-11',
            created_at_value, active_until_value, active_until_value,
            selected_discipline, selected_subject, NULL, NULL, ARRAY[]::text[]
        ) RETURNING course_instance_id INTO v_course_instance_id;
        INSERT INTO ple_data.course_origin (
            course_origin_id, course_instance_id, source_kind, blueprint_course_id,
            blueprint_revision_number, source_course_instance_id, created_at
        ) VALUES (
            '00000000-0000-0000-0000-000000000221', v_course_instance_id,
            'adopted', v_blueprint_course_id, 1, NULL, created_at_value
        );
        INSERT INTO ple_data.course_membership (
            course_membership_id, course_instance_id, account_id, role, joined_at
        ) VALUES (
            '00000000-0000-0000-0000-000000000222', v_course_instance_id,
            elena, 'instructor', created_at_value
        );
        PERFORM ple_audit.record_course_instance_creation_event(
            '00000000-0000-0000-0000-000000000223', v_course_instance_id,
            'adopted', v_blueprint_course_id, 1,
            elena, elena, created_at_value
        );
    ELSE
        IF NOT EXISTS (SELECT 1 FROM ple_data.course_instance
                        WHERE course_instance_id = v_course_instance_id
                          AND content_discipline_id = selected_discipline
                          AND content_subject_id = selected_subject
                          AND content_topic_id IS NULL AND content_subtopic_id IS NULL
                          AND tags = ARRAY[]::text[]) THEN
            RAISE EXCEPTION USING ERRCODE = '23514',
                MESSAGE = 'Live Demo Course classification conflicts with authored metadata';
        END IF;
    END IF;
    PERFORM set_config('ple.installation_live_demo_course_instance_id', v_course_instance_id, true);
END
$$;

INSERT INTO ple_private.course_roster_profile (
    course_roster_profile_id, course_instance_id, student_account_id, roster_id, roster_name, created_at
) VALUES
    ('00000000-0000-0000-0000-000000000231',
     current_setting('ple.installation_live_demo_course_instance_id'),
     current_setting('ple.installation_live_demo_mary_account_id'),
     'BIO301-MARY', 'Mary', pg_catalog.transaction_timestamp()),
    ('00000000-0000-0000-0000-000000000232',
     current_setting('ple.installation_live_demo_course_instance_id'),
     current_setting('ple.installation_live_demo_jack_account_id'),
     'BIO301-JACK', 'Jack', pg_catalog.transaction_timestamp()),
    ('00000000-0000-0000-0000-000000000233',
     current_setting('ple.installation_live_demo_course_instance_id'),
     current_setting('ple.installation_live_demo_avery_account_id'),
     'BIO301-AVERY', 'Avery', pg_catalog.transaction_timestamp())
ON CONFLICT (course_roster_profile_id) DO NOTHING;

INSERT INTO ple_private.course_invitation (
    course_invitation_id, course_instance_id, target_account_id, membership_role,
    inviting_instructor_account_id, inviting_instructor_role, issued_at, expires_at
) VALUES
    ('00000000-0000-0000-0000-000000000241',
     current_setting('ple.installation_live_demo_course_instance_id'),
     current_setting('ple.installation_live_demo_mary_account_id'),
     'student', current_setting('ple.installation_live_demo_elena_account_id'),
     'instructor', pg_catalog.transaction_timestamp(), pg_catalog.transaction_timestamp() + interval '365 days'),
    ('00000000-0000-0000-0000-000000000242',
     current_setting('ple.installation_live_demo_course_instance_id'),
     current_setting('ple.installation_live_demo_jack_account_id'),
     'student', current_setting('ple.installation_live_demo_elena_account_id'),
     'instructor', pg_catalog.transaction_timestamp(), pg_catalog.transaction_timestamp() + interval '365 days'),
    ('00000000-0000-0000-0000-000000000243',
     current_setting('ple.installation_live_demo_course_instance_id'),
     current_setting('ple.installation_live_demo_avery_account_id'),
     'student', current_setting('ple.installation_live_demo_elena_account_id'),
     'instructor', pg_catalog.transaction_timestamp(), pg_catalog.transaction_timestamp() + interval '365 days')
ON CONFLICT (course_invitation_id) DO NOTHING;

INSERT INTO ple_data.student_record (student_record_id, course_instance_id, student_account_id, created_at) VALUES
    ('00000000-0000-0000-0000-000000000251',
     current_setting('ple.installation_live_demo_course_instance_id'),
     current_setting('ple.installation_live_demo_mary_account_id'), pg_catalog.transaction_timestamp()),
    ('00000000-0000-0000-0000-000000000252',
     current_setting('ple.installation_live_demo_course_instance_id'),
     current_setting('ple.installation_live_demo_jack_account_id'), pg_catalog.transaction_timestamp()),
    ('00000000-0000-0000-0000-000000000253',
     current_setting('ple.installation_live_demo_course_instance_id'),
     current_setting('ple.installation_live_demo_avery_account_id'), pg_catalog.transaction_timestamp())
ON CONFLICT (student_record_id) DO NOTHING;

INSERT INTO ple_data.course_membership (
    course_membership_id, course_instance_id, account_id, role, student_record_id, joined_at
) VALUES
    ('00000000-0000-0000-0000-000000000261',
     current_setting('ple.installation_live_demo_course_instance_id'),
     current_setting('ple.installation_live_demo_mary_account_id'),
     'student', '00000000-0000-0000-0000-000000000251', pg_catalog.transaction_timestamp()),
    ('00000000-0000-0000-0000-000000000262',
     current_setting('ple.installation_live_demo_course_instance_id'),
     current_setting('ple.installation_live_demo_jack_account_id'),
     'student', '00000000-0000-0000-0000-000000000252', pg_catalog.transaction_timestamp()),
    ('00000000-0000-0000-0000-000000000263',
     current_setting('ple.installation_live_demo_course_instance_id'),
     current_setting('ple.installation_live_demo_avery_account_id'),
     'student', '00000000-0000-0000-0000-000000000253', pg_catalog.transaction_timestamp())
ON CONFLICT (course_membership_id) DO NOTHING;

SET LOCAL ROLE ple_data_owner;

DO $$
DECLARE
    v_course_instance_id text := current_setting('ple.installation_live_demo_course_instance_id');
    assessment_id_value text;
BEGIN
    SELECT assessment.assessment_id INTO assessment_id_value
      FROM ple_data.assessment AS assessment
      JOIN ple_data.assessment_policy_snapshot AS policy
        ON policy.assessment_policy_snapshot_id = assessment.assessment_policy_snapshot_id
     WHERE assessment.course_instance_id = v_course_instance_id
       AND policy.assessment_title = 'Chapter 1 Pilot Practice';
    IF assessment_id_value IS NOT NULL
       AND (
           NOT EXISTS (
               SELECT 1 FROM ple_data.assessment AS assessment
                JOIN ple_data.assessment_policy_snapshot AS policy
                  ON policy.assessment_policy_snapshot_id = assessment.assessment_policy_snapshot_id
                WHERE assessment.assessment_id = assessment_id_value
                  AND assessment.course_instance_id = v_course_instance_id
                  AND assessment.assessment_status = 'released'
                  AND assessment.assessment_type = 'practice_question_assignment'
                  AND policy.assessment_title = 'Chapter 1 Pilot Practice'
           )
           OR (SELECT count(*) FROM ple_data.assessment_entry
                WHERE assessment_id = assessment_id_value) <> 4
           OR EXISTS (
               WITH input AS (
                   SELECT value -> 'questionRevisionTuple' ->> 'questionId' AS published_question_id,
                          (value -> 'questionRevisionTuple' ->> 'revisionNumber')::integer AS revision_number,
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
                 ON entry.assessment_id = assessment_id_value
                AND entry.authored_position = input.position
                AND entry.entry_kind = 'fixed_question'
               LEFT JOIN ple_data.assessment_entry_question AS question
                 ON question.assessment_entry_id = entry.assessment_entry_id
                AND question.published_question_id = input.published_question_id
                AND question.question_revision_number = input.revision_number
               WHERE question.assessment_entry_id IS NULL
           )
       ) THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Live Demo Assessment conflicts with existing product data';
    END IF;
END
$$;

DO $$
DECLARE
    v_course_instance_id text := current_setting('ple.installation_live_demo_course_instance_id');
    new_assessment_id text;
    assessment_placeholder constant text := 'A0000000A';
    expected_blueprint_assessment_id uuid := current_setting(
        'ple.installation_live_demo_blueprint_assessment_id'
    )::uuid;
BEGIN
    SELECT assessment.assessment_id INTO new_assessment_id
      FROM ple_data.assessment AS assessment
      JOIN ple_data.assessment_policy_snapshot AS policy
        ON policy.assessment_policy_snapshot_id = assessment.assessment_policy_snapshot_id
     WHERE assessment.course_instance_id = v_course_instance_id
       AND policy.assessment_title = 'Chapter 1 Pilot Practice';
    IF new_assessment_id IS NOT NULL THEN
        PERFORM set_config(
            'ple.installation_live_demo_assessment_id', new_assessment_id, true
        );
        RETURN;
    END IF;
    INSERT INTO ple_data.assessment (
        assessment_id, course_instance_id, origin_kind, source_blueprint_course_id,
        source_blueprint_revision_number, source_blueprint_assessment_id,
        created_at, updated_at, assessment_type, assessment_policy_snapshot_id
    ) VALUES (
        assessment_placeholder,
        v_course_instance_id,
        'adopted',
        (SELECT blueprint_course_id FROM ple_data.course_instance WHERE course_instance_id = v_course_instance_id),
        1, expected_blueprint_assessment_id, pg_catalog.transaction_timestamp(), pg_catalog.transaction_timestamp(),
        'practice_question_assignment',
        ple_private.ensure_assessment_policy_snapshot(
            'Chapter 1 Pilot Practice',
            'Complete the four reviewed Chapter 1 practice questions.',
            NULL,
            (SELECT active_until_at FROM ple_data.course_instance
              WHERE course_instance_id = v_course_instance_id),
            NULL,
            1800, NULL, 'accept', 'new_variation', 'authored_order',
            'after_submit', 'after_submit', 'after_submit',
            'never', 'never', 'never',
            'practice_question_assignment'
        )
    ) RETURNING assessment_id INTO new_assessment_id;
    INSERT INTO ple_data.assessment_entry (
        assessment_entry_id, assessment_id, authored_position, entry_kind, scoring_rule
    )
    SELECT ('00000000-0000-0000-0000-00000000028' || input_position)::uuid,
           new_assessment_id, input_position::integer,
           'fixed_question', 'normal'
      FROM (
          SELECT publication.key AS slug,
                 publication.value -> 'questionRevisionTuple' ->> 'questionId' AS published_question_id,
                 (publication.value -> 'questionRevisionTuple' ->> 'revisionNumber')::integer AS revision_number,
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
    INSERT INTO ple_data.assessment_entry_question (
        assessment_entry_id, assessment_id, published_question_id, question_revision_number, points_possible
    )
    SELECT ('00000000-0000-0000-0000-00000000028' || input_position)::uuid,
           new_assessment_id, input.published_question_id, input.revision_number, 1
      FROM (
          SELECT publication.key AS slug,
                 publication.value -> 'questionRevisionTuple' ->> 'questionId' AS published_question_id,
                 (publication.value -> 'questionRevisionTuple' ->> 'revisionNumber')::integer AS revision_number,
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
    PERFORM ple_data.validate_assessment_release(
        new_assessment_id, transaction_timestamp(), true
    );
    UPDATE ple_data.assessment SET assessment_status = 'released',
        assessment_edit_number = assessment_edit_number + 1, updated_at = pg_catalog.transaction_timestamp()
     WHERE assessment_id = new_assessment_id;
    PERFORM ple_data.synchronize_course_assessment_deadline(v_course_instance_id);
    PERFORM set_config(
        'ple.installation_live_demo_assessment_id', new_assessment_id, true
    );
END
$$;

RESET ROLE;

SET LOCAL ROLE ple_api_owner;

INSERT INTO ple_private.course_invitation_event (
    course_invitation_event_id, course_invitation_id, event_kind, performed_by_account_id, occurred_at, reason
) VALUES
    ('00000000-0000-0000-0000-000000000291', '00000000-0000-0000-0000-000000000241', 'accepted',
     current_setting('ple.installation_live_demo_mary_account_id'), pg_catalog.transaction_timestamp(),
     'student accepted Course Invitation'),
    ('00000000-0000-0000-0000-000000000292', '00000000-0000-0000-0000-000000000242', 'accepted',
     current_setting('ple.installation_live_demo_jack_account_id'), pg_catalog.transaction_timestamp(),
     'student accepted Course Invitation'),
    ('00000000-0000-0000-0000-000000000293', '00000000-0000-0000-0000-000000000243', 'accepted',
     current_setting('ple.installation_live_demo_avery_account_id'), pg_catalog.transaction_timestamp(),
     'student accepted Course Invitation')
ON CONFLICT (course_invitation_event_id) DO NOTHING;

RESET ROLE;

SET LOCAL ROLE ple_audit_owner;
INSERT INTO ple_audit.course_roster_event (
    course_roster_event_id, course_instance_id, student_account_id, acting_account_id, event_kind, occurred_at
) VALUES
    ('00000000-0000-0000-0000-000000000281',
     current_setting('ple.installation_live_demo_course_instance_id'),
     current_setting('ple.installation_live_demo_mary_account_id'),
     current_setting('ple.installation_live_demo_elena_account_id'),
     'invitation_created', pg_catalog.transaction_timestamp()),
    ('00000000-0000-0000-0000-000000000282',
     current_setting('ple.installation_live_demo_course_instance_id'),
     current_setting('ple.installation_live_demo_jack_account_id'),
     current_setting('ple.installation_live_demo_elena_account_id'),
     'invitation_created', pg_catalog.transaction_timestamp()),
    ('00000000-0000-0000-0000-000000000283',
     current_setting('ple.installation_live_demo_course_instance_id'),
     current_setting('ple.installation_live_demo_avery_account_id'),
     current_setting('ple.installation_live_demo_elena_account_id'),
     'invitation_created', pg_catalog.transaction_timestamp()),
    ('00000000-0000-0000-0000-000000000284',
     current_setting('ple.installation_live_demo_course_instance_id'),
     current_setting('ple.installation_live_demo_mary_account_id'),
     current_setting('ple.installation_live_demo_mary_account_id'),
     'invitation_claimed', pg_catalog.transaction_timestamp()),
    ('00000000-0000-0000-0000-000000000285',
     current_setting('ple.installation_live_demo_course_instance_id'),
     current_setting('ple.installation_live_demo_jack_account_id'),
     current_setting('ple.installation_live_demo_jack_account_id'),
     'invitation_claimed', pg_catalog.transaction_timestamp()),
    ('00000000-0000-0000-0000-000000000286',
     current_setting('ple.installation_live_demo_course_instance_id'),
     current_setting('ple.installation_live_demo_avery_account_id'),
     current_setting('ple.installation_live_demo_avery_account_id'),
     'invitation_claimed', pg_catalog.transaction_timestamp())
ON CONFLICT DO NOTHING;
RESET ROLE;

SET LOCAL ROLE ple_private_owner;
DO $$
DECLARE v_session_id uuid := current_setting('ple.installation_pilot_publication_session_id')::uuid;
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM ple_private.authenticated_session
         WHERE authenticated_session.session_id = v_session_id
           AND authenticated_session.account_id
               = current_setting('ple.installation_live_demo_elena_account_id')
           AND authenticated_session.product_role = 'instructor'
    ) THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Pilot publication session is not the supplied temporary Instructor session';
    END IF;
    UPDATE ple_private.authenticated_session
       SET revoked_at = coalesce(revoked_at, pg_catalog.transaction_timestamp())
     WHERE authenticated_session.session_id = v_session_id;
END
$$;
RESET ROLE;

-- This installation-data layer intentionally ends before Assessment Attempts, response
-- presentation, submission, grading, and statistics: each has external or
-- worker-owned effects and remains with its existing owner path.
