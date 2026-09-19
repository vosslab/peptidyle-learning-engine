-- Focused PG17 acceptance: the installation data is ordinary relational state,
-- maps all eight source checksums to exact Question Revisions, and replays
-- without duplicate graph roots.

SET LOCAL ROLE ple_data_owner;
DO $$
DECLARE
    selected_discipline uuid;
    selected_subject uuid;
    course_id text;
    assessment_id_value text;
BEGIN
    SELECT discipline.content_discipline_id, subject.content_subject_id
      INTO STRICT selected_discipline, selected_subject
      FROM ple_data.content_discipline AS discipline
      JOIN ple_data.content_subject_discipline AS association USING (content_discipline_id)
      JOIN ple_data.content_subject AS subject USING (content_subject_id)
     WHERE discipline.name = 'Biology' AND subject.name = 'Biochemistry';
    PERFORM set_config('ple.installation_live_demo_discipline_uuid', selected_discipline::text, true);
    PERFORM set_config('ple.installation_live_demo_subject_uuid', selected_subject::text, true);
    SELECT course.course_instance_id INTO course_id
      FROM ple_data.course_instance AS course
     WHERE course.course_short_name = 'BCHM 301';
    IF course_id IS NOT NULL THEN
        PERFORM set_config('ple.installation_live_demo_course_instance_id', course_id, true);
        SELECT assessment.assessment_id INTO assessment_id_value
          FROM ple_data.assessment AS assessment
          JOIN ple_data.assessment_policy_snapshot AS policy
            ON policy.assessment_policy_snapshot_id = assessment.assessment_policy_snapshot_id
         WHERE assessment.course_instance_id = course_id
           AND policy.assessment_title = 'Chapter 1 Pilot Practice';
        IF assessment_id_value IS NOT NULL THEN
            PERFORM set_config(
                'ple.installation_live_demo_assessment_id', assessment_id_value, true
            );
        END IF;
    END IF;
END
$$;

SET LOCAL ROLE ple_private_owner;
DO $$
DECLARE
    elena text;
    mary text;
    jack text;
    avery text;
    priya text;
    sysadmin_id text;
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
    SELECT email.account_id INTO priya
      FROM ple_private.account_authentication_email AS email
      JOIN ple_private.account AS account
        ON account.account_id = email.account_id
     WHERE email.normalized_email = 'priya.shah@live-demo.invalid'
       AND account.product_role = 'instructor';
    SELECT account.account_id INTO sysadmin_id
      FROM ple_private.account AS account
     WHERE account.product_role = 'sysadmin'
     ORDER BY account.created_at, account.account_id
     LIMIT 1;
    IF elena IS NULL OR mary IS NULL OR jack IS NULL OR avery IS NULL
       OR priya IS NULL OR sysadmin_id IS NULL THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Live Demo fictional identities are incomplete';
    END IF;
    IF EXISTS (
        SELECT 1
          FROM (VALUES (elena), (mary), (jack), (avery), (priya), (sysadmin_id)
          ) AS expected(account_id)
          LEFT JOIN LATERAL (
              SELECT event.state
                FROM ple_private.account_state_event AS event
               WHERE event.account_id = expected.account_id
               ORDER BY event.occurred_at DESC, event.event_id DESC
               LIMIT 1
          ) AS current_state ON true
         WHERE current_state.state IS DISTINCT FROM 'active'
    ) THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Live Demo fictional identities are not active';
    END IF;
    IF ple_private.verified_instructor_display_name(elena)
           IS DISTINCT FROM 'Elena Martinez'
       OR ple_private.verified_instructor_display_name(priya)
           IS DISTINCT FROM 'Priya Shah' THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Live Demo Instructor vetted identity is incomplete';
    END IF;
    IF NOT EXISTS (SELECT 1 FROM ple_private.authoring_workspace
                    WHERE authoring_workspace_id = '00000000-0000-0000-0000-000000000201'
                      AND owner_account_id = elena)
       OR NOT EXISTS (SELECT 1 FROM ple_private.authoring_workspace
                       WHERE authoring_workspace_id = '00000000-0000-0000-0000-000000000206'
                         AND owner_account_id = priya) THEN
        RAISE EXCEPTION USING ERRCODE = '23514', MESSAGE = 'Live Demo authoring context is incomplete';
    END IF;
    IF current_setting('ple.installation_pilot_publication_session_id', true) IS NOT NULL
       AND NOT EXISTS (
           SELECT 1 FROM ple_private.authenticated_session
            WHERE session_id = current_setting(
                'ple.installation_pilot_publication_session_id', true
            )::uuid
              AND revoked_at IS NOT NULL
       ) THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Pilot publication session was not revoked';
    END IF;
    PERFORM set_config('ple.installation_live_demo_elena_account_id', elena, true);
    PERFORM set_config('ple.installation_live_demo_mary_account_id', mary, true);
    PERFORM set_config('ple.installation_live_demo_jack_account_id', jack, true);
    PERFORM set_config('ple.installation_live_demo_avery_account_id', avery, true);
    PERFORM set_config('ple.installation_live_demo_priya_account_id', priya, true);
END
$$;
RESET ROLE;

SET LOCAL ROLE ple_audit_owner;
DO $$
DECLARE
    course_id text := current_setting('ple.installation_live_demo_course_instance_id', true);
    elena text := current_setting('ple.installation_live_demo_elena_account_id', true);
    mary text := current_setting('ple.installation_live_demo_mary_account_id', true);
    jack text := current_setting('ple.installation_live_demo_jack_account_id', true);
    avery text := current_setting('ple.installation_live_demo_avery_account_id', true);
BEGIN
    IF course_id IS NULL
       OR (SELECT count(*) FROM ple_audit.course_roster_event
            WHERE course_instance_id = course_id) <> 6
       OR EXISTS (
           SELECT 1 FROM ple_audit.course_roster_event
            WHERE course_instance_id = course_id
              AND (student_account_id, acting_account_id, event_kind) NOT IN (
                  (mary, elena, 'invitation_created'),
                  (jack, elena, 'invitation_created'),
                  (avery, elena, 'invitation_created'),
                  (mary, mary, 'invitation_claimed'),
                  (jack, jack, 'invitation_claimed'),
                  (avery, avery, 'invitation_claimed')
              )
       ) THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Live Demo Course roster audit facts are not exact';
    END IF;
END
$$;
RESET ROLE;

SET LOCAL ROLE ple_api_owner;
DO $$
DECLARE
    blueprint_id text;
    course_id text;
    elena text := current_setting('ple.installation_live_demo_elena_account_id');
    mary text := current_setting('ple.installation_live_demo_mary_account_id');
    jack text := current_setting('ple.installation_live_demo_jack_account_id');
    avery text := current_setting('ple.installation_live_demo_avery_account_id');
    expected_blueprint_assessment_id uuid;
BEGIN
    -- ASVS 1.2.4 and 8.2.2: the installer carries only the canonical public
    -- Blueprint Course ID; this owner resolves that key exactly here.
    blueprint_id := current_setting(
        'ple.installation_live_demo_blueprint_course_id'
    );
    IF blueprint_id IS NULL OR NOT EXISTS (
        SELECT 1 FROM ple_data.blueprint_course
         WHERE blueprint_course_id = blueprint_id
    ) THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Live Demo Blueprint public reference is unavailable';
    END IF;
    expected_blueprint_assessment_id := current_setting(
        'ple.installation_live_demo_blueprint_assessment_id'
    )::uuid;
    SELECT course.course_instance_id INTO course_id
      FROM ple_data.course_instance AS course
     WHERE course.course_short_name = 'BCHM 301';
    IF (SELECT count(*) FROM ple_data.blueprint_course
             WHERE blueprint_course_id = blueprint_id) <> 1
       OR NOT EXISTS (
           SELECT 1 FROM ple_data.blueprint_course
            WHERE blueprint_course_id = blueprint_id
              AND owner_account_id = elena
              AND short_name = 'BCHM 301'
              AND long_name = 'Biochemistry 301: Proteins and Peptides'
              AND content_discipline_id = current_setting('ple.installation_live_demo_discipline_uuid')::uuid
              AND content_subject_id = current_setting('ple.installation_live_demo_subject_uuid')::uuid
              AND content_topic_id IS NULL AND content_subtopic_id IS NULL AND tags = ARRAY[]::text[]
              AND availability = 'public'
              AND current_blueprint_revision_number = 1
       )
       OR (SELECT count(*) FROM ple_data.course_instance
            WHERE course_short_name = 'BCHM 301') <> 1
       OR NOT EXISTS (SELECT 1 FROM ple_data.course_instance
                       WHERE course_instance_id = course_id
                         AND course_short_name = 'BCHM 301'
                         AND course_long_name = 'Biochemistry 301: Proteins and Peptides'
                         AND content_discipline_id = current_setting('ple.installation_live_demo_discipline_uuid')::uuid
                         AND content_subject_id = current_setting('ple.installation_live_demo_subject_uuid')::uuid
                         AND content_topic_id IS NULL AND content_subtopic_id IS NULL AND tags = ARRAY[]::text[]
                         AND term_starts_on = date '2026-08-24' AND term_ends_on = date '2026-12-11'
                         AND blueprint_course_id = blueprint_id
                         AND blueprint_revision_number = 1)
       OR (SELECT count(*) FROM ple_data.blueprint_course_revision AS revision
            WHERE revision.blueprint_course_id = blueprint_id
              AND revision.blueprint_revision_number = 1) <> 1
       OR (SELECT count(*) FROM ple_data.blueprint_revision_question_pin AS pin
            WHERE pin.blueprint_course_id = blueprint_id
              AND pin.blueprint_revision_number = 1) <> 4
       OR EXISTS (
           WITH input AS (
               SELECT value -> 'questionRevisionTuple' ->> 'questionId' AS published_question_id,
                      (value -> 'questionRevisionTuple' ->> 'revisionNumber')::integer AS revision_number
                 FROM jsonb_each(
                     current_setting('ple.installation_pilot_question_publications')::jsonb
                 )
                WHERE key IN (
                    'genetics-disorders-ple-question-json-mc',
                    'genetics-disorders-ple-question-json-matching',
                    'biochemistry-functional-groups-ple-question-json-mc',
                    'biochemistry-functional-groups-ple-question-json-matching'
                )
           )
           SELECT 1 FROM input
           LEFT JOIN ple_data.blueprint_revision_question_pin AS pin
             ON pin.blueprint_course_id = blueprint_id
            AND pin.blueprint_revision_number = 1
            AND pin.published_question_id = input.published_question_id
            AND pin.question_revision_number = input.revision_number
            WHERE pin.published_question_id IS NULL
       )
       OR NOT EXISTS (
           SELECT 1 FROM ple_data.blueprint_revision_assessment AS revision_assessment
            WHERE revision_assessment.blueprint_course_id = blueprint_id
              AND revision_assessment.blueprint_revision_number = 1
              AND revision_assessment.blueprint_assessment_id
                  = expected_blueprint_assessment_id
       )
       OR NOT EXISTS (
           SELECT 1 FROM ple_data.course_origin
            WHERE course_origin_id = '00000000-0000-0000-0000-000000000221'
              AND course_instance_id = course_id
              AND blueprint_course_id = blueprint_id
              AND blueprint_revision_number = 1
              AND source_course_instance_id IS NULL
       )
       OR EXISTS (
           SELECT 1 FROM (VALUES
               ('00000000-0000-0000-0000-000000000222'::uuid, elena, 'instructor'::text, NULL::uuid),
               ('00000000-0000-0000-0000-000000000261'::uuid, mary, 'student'::text, '00000000-0000-0000-0000-000000000251'::uuid),
               ('00000000-0000-0000-0000-000000000262'::uuid, jack, 'student'::text, '00000000-0000-0000-0000-000000000252'::uuid),
               ('00000000-0000-0000-0000-000000000263'::uuid, avery, 'student'::text, '00000000-0000-0000-0000-000000000253'::uuid)
           ) AS expected(course_membership_id, account_id, role, student_record_id)
           LEFT JOIN ple_data.course_membership AS membership
             ON membership.course_membership_id = expected.course_membership_id
            AND membership.course_instance_id = course_id
            AND membership.account_id = expected.account_id
            AND membership.role = expected.role
            AND membership.student_record_id IS NOT DISTINCT FROM expected.student_record_id
           WHERE membership.course_membership_id IS NULL
       )
       OR EXISTS (
           SELECT 1 FROM (VALUES
               ('00000000-0000-0000-0000-000000000251'::uuid, mary),
               ('00000000-0000-0000-0000-000000000252'::uuid, jack),
               ('00000000-0000-0000-0000-000000000253'::uuid, avery)
           ) AS expected(student_record_id, student_id)
           LEFT JOIN ple_data.student_record AS record
             ON record.student_record_id = expected.student_record_id
            AND record.course_instance_id = course_id
            AND record.student_account_id = expected.student_id
           WHERE record.student_record_id IS NULL
       )
       OR EXISTS (
           SELECT 1 FROM (VALUES
               ('00000000-0000-0000-0000-000000000231'::uuid, mary, 'BIO301-MARY'::text, 'Mary'::text),
               ('00000000-0000-0000-0000-000000000232'::uuid, jack, 'BIO301-JACK'::text, 'Jack'::text),
               ('00000000-0000-0000-0000-000000000233'::uuid, avery, 'BIO301-AVERY'::text, 'Avery'::text)
           ) AS expected(profile_id, student_id, roster_id, roster_name)
           LEFT JOIN ple_private.course_roster_profile AS profile
             ON profile.course_roster_profile_id = expected.profile_id
            AND profile.course_instance_id = course_id
            AND profile.student_account_id = expected.student_id
            AND profile.roster_id = expected.roster_id
            AND profile.roster_name = expected.roster_name
           WHERE profile.course_roster_profile_id IS NULL
       )
       OR EXISTS (
           SELECT 1 FROM (VALUES
               ('00000000-0000-0000-0000-000000000241'::uuid, mary),
               ('00000000-0000-0000-0000-000000000242'::uuid, jack),
               ('00000000-0000-0000-0000-000000000243'::uuid, avery)
           ) AS expected(course_invitation_id, student_id)
           LEFT JOIN ple_private.course_invitation AS invitation
             ON invitation.course_invitation_id = expected.course_invitation_id
            AND invitation.course_instance_id = course_id
            AND invitation.target_account_id = expected.student_id
            AND invitation.membership_role = 'student'
           LEFT JOIN ple_private.course_invitation_event AS invitation_event
             ON invitation_event.course_invitation_id = expected.course_invitation_id
            AND invitation_event.event_kind = 'accepted'
            AND invitation_event.performed_by_account_id = expected.student_id
           WHERE invitation.course_invitation_id IS NULL
              OR invitation_event.course_invitation_id IS NULL
       ) THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Live Demo Blueprint or Course is incomplete';
    END IF;
END
$$;
RESET ROLE;

SET LOCAL ROLE ple_data_owner;
DO $$
DECLARE
    blueprint_id text;
    course_id text;
    assessment_id_value text;
    expected_blueprint_assessment_id uuid := current_setting(
        'ple.installation_live_demo_blueprint_assessment_id'
    )::uuid;
BEGIN
    -- ASVS 1.2.4 and 8.2.2: resolve the Blueprint Course from the exact
    -- canonical public ID at this privileged installation boundary.
    blueprint_id := current_setting(
        'ple.installation_live_demo_blueprint_course_id'
    );
    IF blueprint_id IS NULL OR NOT EXISTS (
        SELECT 1 FROM ple_data.blueprint_course
         WHERE blueprint_course_id = blueprint_id
    ) THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Live Demo Blueprint public reference is unavailable';
    END IF;
    SELECT course.course_instance_id INTO course_id
      FROM ple_data.course_instance AS course
     WHERE course.course_short_name = 'BCHM 301';
    SELECT assessment.assessment_id INTO assessment_id_value
      FROM ple_data.assessment AS assessment
      JOIN ple_data.assessment_policy_snapshot AS policy
        ON policy.assessment_policy_snapshot_id = assessment.assessment_policy_snapshot_id
     WHERE assessment.course_instance_id = course_id
       AND policy.assessment_title = 'Chapter 1 Pilot Practice';
    IF (SELECT count(*) FROM ple_data.assessment
             WHERE assessment_id = assessment_id_value
               AND assessment_status = 'released'
               AND assessment_type = 'practice_question_assignment'
               AND course_instance_id = course_id
               AND origin_kind = 'adopted'
               AND source_blueprint_course_id = blueprint_id
               AND source_blueprint_revision_number = 1
               AND source_blueprint_assessment_id
                   = expected_blueprint_assessment_id) <> 1
       OR (SELECT count(*) FROM ple_data.assessment_entry
             WHERE assessment_id = assessment_id_value) <> 4
       OR EXISTS (
           SELECT 1 FROM ple_data.assessment_entry
            WHERE assessment_id = assessment_id_value
              AND (published_question_id !~ '^[0-9A-HJKMNP-TV-Z]{4}-[0-9A-HJKMNP-TV-Z]{4}$'
                   OR substr(published_question_id, 6, 1) IS DISTINCT FROM
                        ple_private.crockford_checksum_character(
                            substr(published_question_id, 1, 4) || substr(published_question_id, 7, 3)
                        ))
       )
       OR EXISTS (
           WITH input AS (
               SELECT value -> 'questionRevisionTuple' ->> 'questionId' AS published_question_id,
                      (value -> 'questionRevisionTuple' ->> 'revisionNumber')::integer AS revision_number,
                      row_number() OVER (ORDER BY array_position(ARRAY[
                          'genetics-disorders-ple-question-json-mc',
                          'genetics-disorders-ple-question-json-matching',
                          'biochemistry-functional-groups-ple-question-json-mc',
                          'biochemistry-functional-groups-ple-question-json-matching'
                      ], key)) - 1 AS authored_position
                 FROM jsonb_each(
                     current_setting('ple.installation_pilot_question_publications')::jsonb
                 )
                WHERE key IN (
                    'genetics-disorders-ple-question-json-mc',
                    'genetics-disorders-ple-question-json-matching',
                    'biochemistry-functional-groups-ple-question-json-mc',
                    'biochemistry-functional-groups-ple-question-json-matching'
                )
           )
           SELECT 1 FROM input
           LEFT JOIN ple_data.assessment_entry AS entry
             ON entry.assessment_id = assessment_id_value
            AND entry.authored_position = input.authored_position
            AND entry.entry_kind = 'fixed_question'
           LEFT JOIN ple_data.assessment_entry_question AS question
             ON question.assessment_entry_id = entry.assessment_entry_id
            AND question.published_question_id = input.published_question_id
            AND question.question_revision_number = input.revision_number
            WHERE question.assessment_entry_id IS NULL
       ) THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Live Demo installation data is incomplete or uses an invalid Question identifier';
    END IF;
END
$$;
RESET ROLE;
