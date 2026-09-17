-- Focused PG17 acceptance: the installation data is ordinary relational state,
-- maps all eight source checksums to exact Question Revisions, and replays
-- without duplicate graph roots.

SET LOCAL ROLE ple_data_owner;
DO $$
DECLARE
    selected_discipline uuid;
    selected_subject uuid;
BEGIN
    SELECT discipline.discipline_uuid, subject.subject_uuid
      INTO STRICT selected_discipline, selected_subject
      FROM ple_data.content_discipline AS discipline
      JOIN ple_data.content_subject_discipline AS association USING (discipline_uuid)
      JOIN ple_data.content_subject AS subject USING (subject_uuid)
     WHERE discipline.name = 'Biology' AND subject.name = 'Biochemistry';
    PERFORM set_config('ple.installation_live_demo_discipline_uuid', selected_discipline::text, true);
    PERFORM set_config('ple.installation_live_demo_subject_uuid', selected_subject::text, true);
END
$$;

SET LOCAL ROLE ple_private_owner;
DO $$
BEGIN
    IF EXISTS (
        SELECT 1 FROM (VALUES
            ('00000000-0000-0000-0000-000000000101'::uuid, 'instructor'::text),
            ('00000000-0000-0000-0000-000000000102'::uuid, 'student'::text),
            ('00000000-0000-0000-0000-000000000103'::uuid, 'student'::text),
            ('00000000-0000-0000-0000-000000000104'::uuid, 'student'::text),
            ('00000000-0000-0000-0000-000000000105'::uuid, 'sysadmin'::text),
            ('00000000-0000-0000-0000-000000000107'::uuid, 'instructor'::text)
        ) AS expected(account_id, product_role)
        LEFT JOIN ple_private.account AS account
          ON account.account_id = expected.account_id
         AND account.product_role = expected.product_role
        WHERE account.account_id IS NULL
    ) THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Live Demo fictional identities are incomplete';
    END IF;
    IF EXISTS (
        SELECT 1
          FROM (VALUES
              ('00000000-0000-0000-0000-000000000101'::uuid),
              ('00000000-0000-0000-0000-000000000102'::uuid),
              ('00000000-0000-0000-0000-000000000103'::uuid),
              ('00000000-0000-0000-0000-000000000104'::uuid),
              ('00000000-0000-0000-0000-000000000105'::uuid),
              ('00000000-0000-0000-0000-000000000107'::uuid)
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
    IF ple_private.verified_instructor_display_name(
        '00000000-0000-0000-0000-000000000101'::uuid
    ) IS DISTINCT FROM 'Elena Martinez'
       OR ple_private.verified_instructor_display_name(
           '00000000-0000-0000-0000-000000000107'::uuid
       ) IS DISTINCT FROM 'Priya Shah' THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Live Demo Instructor vetted identity is incomplete';
    END IF;
    IF EXISTS (
        SELECT 1 FROM (VALUES
            ('00000000-0000-0000-0000-000000000101'::uuid, 'elena.martinez@live-demo.invalid'::text),
            ('00000000-0000-0000-0000-000000000102'::uuid, 'mary.okafor@biology.roosevelt.edu'::text),
            ('00000000-0000-0000-0000-000000000103'::uuid, 'jack.nguyen@biology.roosevelt.edu'::text),
            ('00000000-0000-0000-0000-000000000104'::uuid, 'avery.thompson@biology.roosevelt.edu'::text),
            ('00000000-0000-0000-0000-000000000107'::uuid, 'priya.shah@live-demo.invalid'::text)
        ) AS expected(account_id, normalized_email)
        LEFT JOIN ple_private.account_authentication_email AS email
          ON email.account_id = expected.account_id
         AND email.normalized_email = expected.normalized_email
        WHERE email.account_id IS NULL
    )
       OR NOT EXISTS (SELECT 1 FROM ple_private.authoring_workspace
                       WHERE workspace_id = '00000000-0000-0000-0000-000000000201'
                         AND owner_account_id = '00000000-0000-0000-0000-000000000101')
       OR NOT EXISTS (SELECT 1 FROM ple_private.authoring_workspace
                       WHERE workspace_id = '00000000-0000-0000-0000-000000000206'
                         AND owner_account_id = '00000000-0000-0000-0000-000000000107') THEN
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
END
$$;
RESET ROLE;

SET LOCAL ROLE ple_audit_owner;
DO $$
BEGIN
    IF (SELECT count(*) FROM ple_audit.course_roster_event
         WHERE course_id = '00000000-0000-0000-0000-000000000220') <> 6
       OR EXISTS (
           SELECT 1 FROM ple_audit.course_roster_event
            WHERE course_id = '00000000-0000-0000-0000-000000000220'
              AND (student_account_id, acting_account_id, event_kind) NOT IN (
                  ('00000000-0000-0000-0000-000000000102'::uuid, '00000000-0000-0000-0000-000000000101'::uuid, 'invitation_created'),
                  ('00000000-0000-0000-0000-000000000103'::uuid, '00000000-0000-0000-0000-000000000101'::uuid, 'invitation_created'),
                  ('00000000-0000-0000-0000-000000000104'::uuid, '00000000-0000-0000-0000-000000000101'::uuid, 'invitation_created'),
                  ('00000000-0000-0000-0000-000000000102'::uuid, '00000000-0000-0000-0000-000000000102'::uuid, 'invitation_claimed'),
                  ('00000000-0000-0000-0000-000000000103'::uuid, '00000000-0000-0000-0000-000000000103'::uuid, 'invitation_claimed'),
                  ('00000000-0000-0000-0000-000000000104'::uuid, '00000000-0000-0000-0000-000000000104'::uuid, 'invitation_claimed')
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
    blueprint_reference bigint;
    expected_blueprint_assessment_reference uuid;
BEGIN
    -- ASVS 1.2.4 and 8.2.2: the installer carries only the opaque public
    -- Blueprint reference; this owner resolves its internal key exactly here.
    SELECT reference_number INTO blueprint_reference
      FROM ple_data.blueprint_course
     WHERE public_reference = current_setting(
        'ple.installation_live_demo_blueprint_public_reference'
     );
    IF blueprint_reference IS NULL THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Live Demo Blueprint public reference is unavailable';
    END IF;
    expected_blueprint_assessment_reference := current_setting(
        'ple.installation_live_demo_blueprint_assessment_reference'
    )::uuid;
    IF (SELECT count(*) FROM ple_data.blueprint_course
             WHERE reference_number = blueprint_reference) <> 1
       OR NOT EXISTS (
           SELECT 1 FROM ple_data.blueprint_course
            WHERE reference_number = blueprint_reference
              AND owner_account_id = '00000000-0000-0000-0000-000000000101'
              AND short_name = 'BCHM 301'
              AND long_name = 'Biochemistry 301: Proteins and Peptides'
              AND discipline_uuid = current_setting('ple.installation_live_demo_discipline_uuid')::uuid
              AND subject_uuid = current_setting('ple.installation_live_demo_subject_uuid')::uuid
              AND topic_uuid IS NULL AND subtopic_uuid IS NULL AND tags = ARRAY[]::text[]
              AND availability = 'public'
              AND current_blueprint_revision_number = 1
       )
       OR NOT EXISTS (SELECT 1 FROM ple_data.course_instance
                       WHERE course_id = '00000000-0000-0000-0000-000000000220'
                         AND course_short_name = 'BCHM 301'
                         AND course_long_name = 'Biochemistry 301: Proteins and Peptides'
                         AND discipline_uuid = current_setting('ple.installation_live_demo_discipline_uuid')::uuid
                         AND subject_uuid = current_setting('ple.installation_live_demo_subject_uuid')::uuid
                         AND topic_uuid IS NULL AND subtopic_uuid IS NULL AND tags = ARRAY[]::text[]
                         AND term_starts_on = date '2026-08-24' AND term_ends_on = date '2026-12-11'
                         AND blueprint_course_reference_number = blueprint_reference
                         AND blueprint_revision_number = 1)
       OR (SELECT count(*) FROM ple_data.blueprint_course_revision AS revision
            JOIN ple_data.blueprint_course AS blueprint
              ON blueprint.reference_number = revision.blueprint_course_reference_number
            WHERE blueprint.reference_number = blueprint_reference
              AND revision.blueprint_revision_number = 1) <> 1
       OR (SELECT count(*) FROM ple_data.blueprint_revision_question_pin AS pin
            JOIN ple_data.blueprint_course AS blueprint
              ON blueprint.reference_number = pin.blueprint_course_reference_number
            WHERE blueprint.reference_number = blueprint_reference
              AND pin.blueprint_revision_number = 1) <> 4
       OR EXISTS (
           WITH input AS (
               SELECT value -> 'questionRevision' ->> 'questionId' AS question_id,
                      (value -> 'questionRevision' ->> 'revisionNumber')::integer AS revision_number
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
             ON pin.blueprint_course_reference_number = blueprint_reference
            AND pin.blueprint_revision_number = 1
            AND pin.question_id = input.question_id
            AND pin.question_revision_number = input.revision_number
            WHERE pin.question_id IS NULL
       )
       OR NOT EXISTS (
           SELECT 1 FROM ple_data.blueprint_revision_assessment AS revision_assessment
            WHERE revision_assessment.blueprint_course_reference_number = blueprint_reference
              AND revision_assessment.blueprint_revision_number = 1
              AND revision_assessment.blueprint_assessment_reference
                  = expected_blueprint_assessment_reference
       )
       OR NOT EXISTS (
           SELECT 1 FROM ple_data.course_origin
            WHERE course_origin_id = '00000000-0000-0000-0000-000000000221'
              AND course_id = '00000000-0000-0000-0000-000000000220'
              AND blueprint_course_reference_number = blueprint_reference
              AND blueprint_revision_number = 1
              AND source_course_id IS NULL
       )
       OR EXISTS (
           SELECT 1 FROM (VALUES
               ('00000000-0000-0000-0000-000000000222'::uuid, '00000000-0000-0000-0000-000000000101'::uuid, 'instructor'::text, NULL::uuid),
               ('00000000-0000-0000-0000-000000000261'::uuid, '00000000-0000-0000-0000-000000000102'::uuid, 'student'::text, '00000000-0000-0000-0000-000000000251'::uuid),
               ('00000000-0000-0000-0000-000000000262'::uuid, '00000000-0000-0000-0000-000000000103'::uuid, 'student'::text, '00000000-0000-0000-0000-000000000252'::uuid),
               ('00000000-0000-0000-0000-000000000263'::uuid, '00000000-0000-0000-0000-000000000104'::uuid, 'student'::text, '00000000-0000-0000-0000-000000000253'::uuid)
           ) AS expected(membership_id, account_id, role, student_record_id)
           LEFT JOIN ple_data.course_membership AS membership
             ON membership.membership_id = expected.membership_id
            AND membership.course_id = '00000000-0000-0000-0000-000000000220'
            AND membership.account_id = expected.account_id
            AND membership.role = expected.role
            AND membership.student_record_id IS NOT DISTINCT FROM expected.student_record_id
           WHERE membership.membership_id IS NULL
       )
       OR EXISTS (
           SELECT 1 FROM (VALUES
               ('00000000-0000-0000-0000-000000000251'::uuid, '00000000-0000-0000-0000-000000000102'::uuid),
               ('00000000-0000-0000-0000-000000000252'::uuid, '00000000-0000-0000-0000-000000000103'::uuid),
               ('00000000-0000-0000-0000-000000000253'::uuid, '00000000-0000-0000-0000-000000000104'::uuid)
           ) AS expected(student_record_id, student_id)
           LEFT JOIN ple_data.student_record AS record
             ON record.student_record_id = expected.student_record_id
            AND record.course_id = '00000000-0000-0000-0000-000000000220'
            AND record.student_account_id = expected.student_id
           WHERE record.student_record_id IS NULL
       )
       OR EXISTS (
           SELECT 1 FROM (VALUES
               ('00000000-0000-0000-0000-000000000231'::uuid, '00000000-0000-0000-0000-000000000102'::uuid, 'BIO301-MARY'::text, 'Mary'::text),
               ('00000000-0000-0000-0000-000000000232'::uuid, '00000000-0000-0000-0000-000000000103'::uuid, 'BIO301-JACK'::text, 'Jack'::text),
               ('00000000-0000-0000-0000-000000000233'::uuid, '00000000-0000-0000-0000-000000000104'::uuid, 'BIO301-AVERY'::text, 'Avery'::text)
           ) AS expected(profile_id, student_id, roster_id, roster_name)
           LEFT JOIN ple_private.course_roster_profile AS profile
             ON profile.course_roster_profile_id = expected.profile_id
            AND profile.course_id = '00000000-0000-0000-0000-000000000220'
            AND profile.student_account_id = expected.student_id
            AND profile.roster_id = expected.roster_id
            AND profile.roster_name = expected.roster_name
           WHERE profile.course_roster_profile_id IS NULL
       )
       OR EXISTS (
           SELECT 1 FROM (VALUES
               ('00000000-0000-0000-0000-000000000241'::uuid, '00000000-0000-0000-0000-000000000102'::uuid),
               ('00000000-0000-0000-0000-000000000242'::uuid, '00000000-0000-0000-0000-000000000103'::uuid),
               ('00000000-0000-0000-0000-000000000243'::uuid, '00000000-0000-0000-0000-000000000104'::uuid)
           ) AS expected(invitation_id, student_id)
           LEFT JOIN ple_private.course_invitation AS invitation
             ON invitation.invitation_id = expected.invitation_id
            AND invitation.course_id = '00000000-0000-0000-0000-000000000220'
            AND invitation.target_account_id = expected.student_id
            AND invitation.membership_role = 'student'
           LEFT JOIN ple_private.course_invitation_event AS invitation_event
             ON invitation_event.invitation_id = expected.invitation_id
            AND invitation_event.event_kind = 'accepted'
            AND invitation_event.performed_by_account_id = expected.student_id
           WHERE invitation.invitation_id IS NULL
              OR invitation_event.invitation_id IS NULL
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
    blueprint_reference bigint;
    expected_blueprint_assessment_reference uuid := current_setting(
        'ple.installation_live_demo_blueprint_assessment_reference'
    )::uuid;
BEGIN
    -- ASVS 1.2.4 and 8.2.2: resolve the internal key only from the exact
    -- canonical public reference at this privileged installation boundary.
    SELECT reference_number INTO blueprint_reference
      FROM ple_data.blueprint_course
     WHERE public_reference = current_setting(
        'ple.installation_live_demo_blueprint_public_reference'
     );
    IF blueprint_reference IS NULL THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Live Demo Blueprint public reference is unavailable';
    END IF;
    IF (SELECT count(*) FROM ple_data.assessment
             WHERE assessment_id = '00000000-0000-0000-0000-000000000270'
               AND assessment_status = 'released'
               AND assessment_type = 'practice_question_assignment'
               AND course_id = '00000000-0000-0000-0000-000000000220'
               AND origin_kind = 'adopted'
               AND source_blueprint_course_reference_number = blueprint_reference
               AND source_blueprint_revision_number = 1
               AND source_blueprint_assessment_reference
                   = expected_blueprint_assessment_reference) <> 1
       OR (SELECT count(*) FROM ple_data.assessment_entry
             WHERE assessment_id = '00000000-0000-0000-0000-000000000270') <> 4
       OR EXISTS (
           SELECT 1 FROM ple_data.assessment_entry
            WHERE assessment_id = '00000000-0000-0000-0000-000000000270'
              AND (question_id !~ '^[0-9A-HJKMNP-TV-Z]{4}-[0-9A-HJKMNP-TV-Z]{4}$'
                   OR substr(question_id, 6, 1) IS DISTINCT FROM
                        ple_private.crockford_checksum_character(
                            substr(question_id, 1, 4) || substr(question_id, 7, 3)
                        ))
       )
       OR EXISTS (
           WITH input AS (
               SELECT value -> 'questionRevision' ->> 'questionId' AS question_id,
                      (value -> 'questionRevision' ->> 'revisionNumber')::integer AS revision_number,
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
             ON entry.assessment_id = '00000000-0000-0000-0000-000000000270'
            AND entry.authored_position = input.authored_position
            AND entry.entry_kind = 'fixed_question'
            AND entry.question_id = input.question_id
            AND entry.question_revision_number = input.revision_number
            WHERE entry.assessment_entry_id IS NULL
       ) THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Live Demo installation data is incomplete or uses an invalid Question identifier';
    END IF;
END
$$;
RESET ROLE;
