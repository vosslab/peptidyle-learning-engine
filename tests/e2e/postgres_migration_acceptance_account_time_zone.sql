-- Account-owned time-zone migration acceptance oracle.
-- Executed by the disposable PostgreSQL Migration Acceptance Runtime after
-- the established Instructor Account fixture supplies an active Sysadmin.

DO $$
BEGIN
    IF to_regclass('ple_private.account_time_zone') IS NULL
       OR NOT (SELECT relation.relrowsecurity AND relation.relforcerowsecurity
                 FROM pg_class AS relation
                WHERE relation.oid = 'ple_private.account_time_zone'::regclass)
       OR (SELECT count(*) FROM pg_policy
            WHERE polrelid = 'ple_private.account_time_zone'::regclass) <> 3
       OR has_table_privilege('ple_app', 'ple_private.account_time_zone',
                              'SELECT, INSERT, UPDATE, DELETE')
       OR has_schema_privilege('ple_app', 'ple_private', 'USAGE')
       OR NOT has_function_privilege(
           'ple_app', 'ple_api.current_account_time_zone()', 'EXECUTE'
       )
       OR has_function_privilege(
           'public', 'ple_api.current_account_time_zone()', 'EXECUTE'
       ) THEN
        RAISE EXCEPTION 'Account time-zone relation or self-read authority is not exact';
    END IF;
END
$$;

-- Every Account insert gets the established default.  Exact zone validation
-- independently rejects case changes, surrounding whitespace, and offsets.
INSERT INTO ple_private.account (account_id, product_role, created_at)
VALUES (
    '00000000-0000-0000-0000-00000000d130', 'student',
    '2026-09-10 00:00:00+00'
);

DO $$
DECLARE
    v_invalid_zone text;
BEGIN
    IF (SELECT time_zone FROM ple_private.account_time_zone
         WHERE account_id = '00000000-0000-0000-0000-00000000d130')
       IS DISTINCT FROM 'America/Chicago' THEN
        RAISE EXCEPTION 'every new Account must receive the established time-zone default';
    END IF;
    FOREACH v_invalid_zone IN ARRAY ARRAY[
        'america/chicago', 'America/Chicago ', '-06:00'
    ] LOOP
        BEGIN
            UPDATE ple_private.account_time_zone
               SET time_zone = v_invalid_zone
             WHERE account_id = '00000000-0000-0000-0000-00000000d130';
            RAISE EXCEPTION 'invalid Account time zone unexpectedly succeeded: %', v_invalid_zone;
        EXCEPTION WHEN OTHERS THEN
            IF SQLSTATE <> '22023' THEN
                RAISE;
            END IF;
        END;
    END LOOP;
END
$$;

-- The authenticated API has no caller-supplied Account target.  It reads only
-- the installed active subject, while direct private-table access remains
-- unavailable to the application capability.
DO $$
DECLARE
    v_self_zone text;
BEGIN
    BEGIN
        SET LOCAL ROLE ple_app;
        PERFORM 1 FROM ple_private.account_time_zone;
        RAISE EXCEPTION 'ple_app directly read another Account preference';
    EXCEPTION WHEN OTHERS THEN
        IF SQLSTATE <> '42501' THEN
            RAISE;
        END IF;
    END;
    RESET ROLE;

    SET LOCAL ROLE ple_app;
    PERFORM pg_catalog.set_config(
        'ple.session_account_id', '00000000-0000-0000-0000-00000000d101', true
    );
    SELECT ple_api.current_account_time_zone() INTO v_self_zone;
    RESET ROLE;
    IF v_self_zone IS DISTINCT FROM 'America/Chicago' THEN
        RAISE EXCEPTION 'authenticated Account self read did not return its own preference';
    END IF;
END
$$;

-- A real roster/claim/revocation/re-enrollment sequence copies the authorized
-- Instructor preference only when creating the Student Account.  Later
-- enrollments must not overwrite the Student's independently owned zone.
DO $$
DECLARE
    v_course_reference bigint;
    v_instructor_account_id uuid;
    v_student_account_id uuid;
    v_student_zone text;
    v_self_zone text;
BEGIN
    SELECT course.reference_number, membership.account_id
      INTO v_course_reference, v_instructor_account_id
      FROM ple_data.course_instance AS course
      JOIN ple_data.course_membership AS membership ON membership.course_id = course.course_id
     WHERE membership.role = 'instructor'
       AND ple_data.course_membership_is_active(membership.membership_id)
     ORDER BY course.reference_number
     LIMIT 1;
    IF v_course_reference IS NULL OR v_instructor_account_id IS NULL THEN
        RAISE EXCEPTION 'Account time-zone oracle requires an active Instructor Course Membership';
    END IF;

    UPDATE ple_private.account_time_zone
       SET time_zone = 'America/New_York'
     WHERE account_id = v_instructor_account_id;

    SET LOCAL ROLE ple_app;
    PERFORM pg_catalog.set_config('ple.session_account_id', v_instructor_account_id::text, true);
    PERFORM ple_api.import_live_demo_course_roster(
        v_course_reference,
        ARRAY['atz1-student@example.test'],
        ARRAY['atz1-student@example.test'],
        ARRAY['ATZ1']
    );
    RESET ROLE;

    SELECT account.account_id INTO v_student_account_id
      FROM ple_private.account AS account
      JOIN ple_private.account_authentication_email AS email ON email.account_id = account.account_id
     WHERE email.normalized_email = 'atz1-student@example.test'
       AND account.product_role = 'student';
    SELECT time_zone INTO v_student_zone
      FROM ple_private.account_time_zone
     WHERE account_id = v_student_account_id;
    IF v_student_zone IS DISTINCT FROM 'America/New_York' THEN
        RAISE EXCEPTION 'new Student did not copy the authorized Instructor preference';
    END IF;

    SET LOCAL ROLE ple_app;
    PERFORM pg_catalog.set_config('ple.session_account_id', v_student_account_id::text, true);
    PERFORM ple_api.claim_live_demo_course_invitation(
        '00000000-0000-0000-0000-00000000d131',
        '00000000-0000-0000-0000-00000000d132',
        '00000000-0000-0000-0000-00000000d133', v_course_reference
    );
    RESET ROLE;

    SET LOCAL ROLE ple_app;
    PERFORM pg_catalog.set_config('ple.session_account_id', v_instructor_account_id::text, true);
    PERFORM ple_api.revoke_live_demo_course_roster_entry(
        '00000000-0000-0000-0000-00000000d134', v_course_reference, 'ATZ1'
    );
    RESET ROLE;

    UPDATE ple_private.account_time_zone
       SET time_zone = 'America/Los_Angeles'
     WHERE account_id = v_instructor_account_id;
    SET LOCAL ROLE ple_app;
    PERFORM pg_catalog.set_config('ple.session_account_id', v_instructor_account_id::text, true);
    PERFORM ple_api.import_live_demo_course_roster(
        v_course_reference,
        ARRAY['atz1-student@example.test'],
        ARRAY['atz1-student@example.test'],
        ARRAY['ATZ1']
    );
    RESET ROLE;

    SET LOCAL ROLE ple_app;
    PERFORM pg_catalog.set_config('ple.session_account_id', v_student_account_id::text, true);
    PERFORM ple_api.claim_live_demo_course_invitation(
        '00000000-0000-0000-0000-00000000d135',
        '00000000-0000-0000-0000-00000000d136',
        '00000000-0000-0000-0000-00000000d137', v_course_reference
    );
    SELECT ple_api.current_account_time_zone() INTO v_self_zone;
    RESET ROLE;

    IF v_self_zone IS DISTINCT FROM 'America/New_York'
       OR (SELECT time_zone FROM ple_private.account_time_zone
            WHERE account_id = v_student_account_id) IS DISTINCT FROM 'America/New_York' THEN
        RAISE EXCEPTION 'Student re-enrollment reset the copied-once Account preference';
    END IF;
END
$$;
