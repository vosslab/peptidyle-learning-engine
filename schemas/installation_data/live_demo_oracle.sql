-- Focused installation acceptance for ordinary imported content and fictional membership.
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
       AND account.user_role = 'instructor';
    SELECT email.account_id INTO mary
      FROM ple_private.account_authentication_email AS email
      JOIN ple_private.account AS account
        ON account.account_id = email.account_id
     WHERE email.normalized_email = 'mary.okafor@biology.roosevelt.edu'
       AND account.user_role = 'student';
    SELECT email.account_id INTO jack
      FROM ple_private.account_authentication_email AS email
      JOIN ple_private.account AS account
        ON account.account_id = email.account_id
     WHERE email.normalized_email = 'jack.nguyen@biology.roosevelt.edu'
       AND account.user_role = 'student';
    SELECT email.account_id INTO avery
      FROM ple_private.account_authentication_email AS email
      JOIN ple_private.account AS account
        ON account.account_id = email.account_id
     WHERE email.normalized_email = 'avery.thompson@biology.roosevelt.edu'
       AND account.user_role = 'student';
    SELECT email.account_id INTO priya
      FROM ple_private.account_authentication_email AS email
      JOIN ple_private.account AS account
        ON account.account_id = email.account_id
     WHERE email.normalized_email = 'priya.shah@live-demo.invalid'
       AND account.user_role = 'instructor';
    SELECT account.account_id INTO sysadmin_id
      FROM ple_private.account AS account
     WHERE account.user_role = 'sysadmin'
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
    IF ple_private.instructor_display_name(elena)
           IS DISTINCT FROM 'Elena Martinez'
       OR ple_private.instructor_display_name(priya)
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
    PERFORM set_config('ple.installation_live_demo_elena_account_id', elena, true);
    PERFORM set_config('ple.installation_live_demo_mary_account_id', mary, true);
    PERFORM set_config('ple.installation_live_demo_jack_account_id', jack, true);
    PERFORM set_config('ple.installation_live_demo_avery_account_id', avery, true);
    PERFORM set_config('ple.installation_live_demo_priya_account_id', priya, true);
END
$$;
RESET ROLE;

SET LOCAL ROLE ple_data_owner;
DO $$
DECLARE
    installed_course ple_data.course_instance_id;
    blueprint ple_data.blueprint_course_id;
BEGIN
    SELECT course_instance_id, blueprint_course_id INTO STRICT installed_course, blueprint
      FROM ple_data.course_instance WHERE course_short_name = 'BCHM 355';
    IF NOT EXISTS (
        SELECT 1 FROM ple_data.course_instance AS course
        JOIN ple_data.blueprint_course AS source USING (blueprint_course_id)
        WHERE course.course_instance_id = installed_course
          AND course.source_kind = 'adopted'
          AND source.availability = 'public'
          AND source.long_name = 'Biochemistry'
    ) THEN
        RAISE EXCEPTION 'Live Demo is not adopted from ordinary public Biochemistry content';
    END IF;
    IF (SELECT count(*) FROM ple_data.course_membership
         WHERE course_instance_id = installed_course AND role = 'student') <> 3 THEN
        RAISE EXCEPTION 'Live Demo fictional Student Memberships are incomplete';
    END IF;
    IF NOT EXISTS (SELECT 1 FROM ple_data.assessment
                    WHERE course_instance_id = installed_course
                      AND origin_kind = 'adopted' AND assessment_status = 'released') THEN
        RAISE EXCEPTION 'Live Demo has no ordinarily adopted released Assessment';
    END IF;
    IF (SELECT count(*) FROM ple_data.assessment WHERE course_instance_id = installed_course)
       <> (SELECT count(*) FROM ple_data.blueprint_revision_assessment
            WHERE blueprint_course_id = blueprint AND blueprint_revision_number = 1) THEN
        RAISE EXCEPTION 'Live Demo is missing imported Blueprint Assessments';
    END IF;
END
$$;
RESET ROLE;
