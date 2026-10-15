-- Trusted fictional enrollment around the ordinarily adopted Biochemistry Course.
-- Reusable content and Assessment release belong to the HTTP product boundary.
SELECT set_config('ple.installation_live_demo_course_instance_id', :'live_demo_course_instance_id', true) AS ignored \gset
SET LOCAL ROLE ple_private_owner;
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


SET LOCAL ROLE ple_api_owner;
INSERT INTO ple_private.course_roster_profile (
    course_roster_profile_id, course_instance_id, student_account_id, roster_id, roster_name, created_at
) VALUES
    ('00000000-0000-0000-0000-000000000231',
     current_setting('ple.installation_live_demo_course_instance_id'),
     current_setting('ple.installation_live_demo_mary_account_id'),
     'BCHM355-MARY', 'Mary', pg_catalog.transaction_timestamp()),
    ('00000000-0000-0000-0000-000000000232',
     current_setting('ple.installation_live_demo_course_instance_id'),
     current_setting('ple.installation_live_demo_jack_account_id'),
     'BCHM355-JACK', 'Jack', pg_catalog.transaction_timestamp()),
    ('00000000-0000-0000-0000-000000000233',
     current_setting('ple.installation_live_demo_course_instance_id'),
     current_setting('ple.installation_live_demo_avery_account_id'),
     'BCHM355-AVERY', 'Avery', pg_catalog.transaction_timestamp())
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
