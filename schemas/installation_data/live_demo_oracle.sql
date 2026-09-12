-- Focused PG17 acceptance: the installation data is ordinary relational state,
-- maps all eight source checksums to exact Question Revisions, and replays
-- without duplicate graph roots.

SET LOCAL ROLE ple_private_owner;
DO $$
BEGIN
    IF EXISTS (
        SELECT 1 FROM (VALUES
            ('00000000-0000-0000-0000-000000000101'::uuid, 'instructor'::text),
            ('00000000-0000-0000-0000-000000000102'::uuid, 'student'::text),
            ('00000000-0000-0000-0000-000000000103'::uuid, 'student'::text),
            ('00000000-0000-0000-0000-000000000104'::uuid, 'student'::text),
            ('00000000-0000-0000-0000-000000000105'::uuid, 'sysadmin'::text)
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
              ('00000000-0000-0000-0000-000000000105'::uuid)
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
    IF EXISTS (
        SELECT 1 FROM (VALUES
            ('00000000-0000-0000-0000-000000000101'::uuid, 'elena.martinez@live-demo.invalid'::text),
            ('00000000-0000-0000-0000-000000000102'::uuid, 'mary.okafor@live-demo.invalid'::text),
            ('00000000-0000-0000-0000-000000000103'::uuid, 'jack.nguyen@live-demo.invalid'::text),
            ('00000000-0000-0000-0000-000000000104'::uuid, 'avery.thompson@live-demo.invalid'::text)
        ) AS expected(account_id, normalized_email)
        LEFT JOIN ple_private.account_authentication_email AS email
          ON email.account_id = expected.account_id
         AND email.normalized_email = expected.normalized_email
        WHERE email.account_id IS NULL
    )
       OR NOT EXISTS (SELECT 1 FROM ple_private.authoring_workspace
                       WHERE workspace_id = '00000000-0000-0000-0000-000000000201'
                         AND owner_account_id = '00000000-0000-0000-0000-000000000101') THEN
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
    expected_content jsonb := current_setting('ple.installation_expected_blueprint_content')::jsonb;
    expected_checksum bytea := sha256(convert_to(
        current_setting('ple.installation_expected_blueprint_content'), 'UTF8'
    ));
    blueprint_reference bigint;
BEGIN
    SELECT reference_number INTO blueprint_reference
      FROM ple_data.blueprint_course
     WHERE blueprint_id = '00000000-0000-0000-0000-000000000210';
    IF (SELECT count(*) FROM ple_data.blueprint_course
             WHERE blueprint_id = '00000000-0000-0000-0000-000000000210') <> 1
       OR NOT EXISTS (
           SELECT 1 FROM ple_data.blueprint_draft
            WHERE blueprint_course_reference_number = blueprint_reference
              AND content = expected_content
              AND content_checksum = expected_checksum
       )
       OR NOT EXISTS (SELECT 1 FROM ple_data.course_instance
                       WHERE course_id = '00000000-0000-0000-0000-000000000220'
                         AND course_short_name = 'BCHM 301'
                         AND course_long_name = 'Biochemistry 301: Proteins and Peptides'
                         AND term_starts_on = date '2026-08-24' AND term_ends_on = date '2026-12-11'
                         AND blueprint_course_reference_number = (
                             SELECT reference_number FROM ple_data.blueprint_course
                              WHERE blueprint_id = '00000000-0000-0000-0000-000000000210'
                         )
                         AND blueprint_revision_number = 1)
       OR (SELECT count(*) FROM ple_data.blueprint_course_revision AS revision
            JOIN ple_data.blueprint_course AS blueprint
              ON blueprint.reference_number = revision.blueprint_course_reference_number
            WHERE blueprint.blueprint_id = '00000000-0000-0000-0000-000000000210'
              AND revision.blueprint_revision_number = 1) <> 1
       OR NOT EXISTS (
           SELECT 1 FROM ple_data.blueprint_course_revision
            WHERE blueprint_course_reference_number = blueprint_reference
              AND blueprint_revision_number = 1
              AND title = 'Biochemistry 301: Proteins and Peptides'
              AND content = expected_content
              AND content_checksum = expected_checksum
       )
       OR (SELECT count(*) FROM ple_data.blueprint_revision_question_pin AS pin
            JOIN ple_data.blueprint_course AS blueprint
              ON blueprint.reference_number = pin.blueprint_course_reference_number
            WHERE blueprint.blueprint_id = '00000000-0000-0000-0000-000000000210'
              AND pin.blueprint_revision_number = 1) <> 8
       OR EXISTS (
           WITH input AS (
               SELECT replace(value -> 'questionRevision' ->> 'questionId', '-', '') AS question_id,
                      (value -> 'questionRevision' ->> 'revisionNumber')::integer AS revision_number
                 FROM jsonb_each(
                     current_setting('ple.installation_pilot_question_publications')::jsonb
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
               ('00000000-0000-0000-0000-000000000231'::uuid, '00000000-0000-0000-0000-000000000102'::uuid, 'mary.okafor@live-demo.invalid'::text, 'BIO301-MARY'::text),
               ('00000000-0000-0000-0000-000000000232'::uuid, '00000000-0000-0000-0000-000000000103'::uuid, 'jack.nguyen@live-demo.invalid'::text, 'BIO301-JACK'::text),
               ('00000000-0000-0000-0000-000000000233'::uuid, '00000000-0000-0000-0000-000000000104'::uuid, 'avery.thompson@live-demo.invalid'::text, 'BIO301-AVERY'::text)
           ) AS expected(profile_id, student_id, roster_email, roster_id)
           LEFT JOIN ple_private.course_roster_profile AS profile
             ON profile.course_roster_profile_id = expected.profile_id
            AND profile.course_id = '00000000-0000-0000-0000-000000000220'
            AND profile.student_account_id = expected.student_id
            AND profile.roster_email = expected.roster_email
            AND profile.roster_id = expected.roster_id
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
DECLARE blueprint_reference bigint := current_setting('ple.installation_blueprint_reference_number', true)::bigint;
BEGIN
    IF (SELECT count(*) FROM ple_data.assignment
             WHERE assignment_id = '00000000-0000-0000-0000-000000000270'
               AND assignment_status = 'released'
               AND course_id = '00000000-0000-0000-0000-000000000220'
               AND source_blueprint_course_reference_number = blueprint_reference
               AND source_blueprint_revision_number = 1
               AND source_blueprint_assignment_reference = '00000000-0000-0000-0000-000000000212') <> 1
       OR (SELECT count(*) FROM ple_data.assignment_entry
             WHERE assignment_id = '00000000-0000-0000-0000-000000000270') <> 8
       OR EXISTS (
           SELECT 1 FROM ple_data.assignment_entry
            WHERE assignment_id = '00000000-0000-0000-0000-000000000270'
              AND question_id !~ '^[0-9A-HJKMNP-TV-Z]{7}$'
       )
       OR EXISTS (
           WITH input AS (
               SELECT replace(value -> 'questionRevision' ->> 'questionId', '-', '') AS question_id,
                      (value -> 'questionRevision' ->> 'revisionNumber')::integer AS revision_number,
                      row_number() OVER (ORDER BY array_position(ARRAY[
                          'genetics-disorders-webwork-mc', 'genetics-disorders-webwork-matching',
                          'genetics-disorders-ple-question-json-mc', 'genetics-disorders-ple-question-json-matching',
                          'biochemistry-functional-groups-webwork-mc', 'biochemistry-functional-groups-webwork-matching',
                          'biochemistry-functional-groups-ple-question-json-mc', 'biochemistry-functional-groups-ple-question-json-matching'
                      ], key)) - 1 AS authored_position
                 FROM jsonb_each(
                     current_setting('ple.installation_pilot_question_publications')::jsonb
                 )
           )
           SELECT 1 FROM input
           LEFT JOIN ple_data.assignment_entry AS entry
             ON entry.assignment_id = '00000000-0000-0000-0000-000000000270'
            AND entry.authored_position = input.authored_position
            AND entry.entry_kind = 'fixed_question'
            AND entry.question_id = input.question_id
            AND entry.question_revision_number = input.revision_number
            WHERE entry.assignment_entry_id IS NULL
       ) THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Live Demo installation data is incomplete or uses an invalid Question identifier';
    END IF;
END
$$;
RESET ROLE;
