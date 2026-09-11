-- M14 schedule-context authority probe. The browser supplies only a local
-- wall-clock string; the connected Store resolves it in the authenticated
-- Instructor Account zone before calling the registered Store save routine.
-- Keep the foreign-Instructor denial probe independent from the primary
-- Assignment fixture. The established second Instructor has no other Course
-- Membership before the later catalog oracle runs.
BEGIN;
INSERT INTO ple_data.course_instance (
    course_id, blueprint_course_reference_number, blueprint_revision_number,
    assigned_instructor_account_id, course_title, assigned_instructor_role, created_at
) VALUES (
    '00000000-0000-0000-0000-00000000f140', 7, 1,
    '00000000-0000-0000-0000-000000000104',
    'M14 foreign Instructor fixture', 'instructor', '2026-01-01 00:00:00+00'
);
INSERT INTO ple_data.course_membership (
    membership_id, course_id, account_id, role, joined_at, student_record_id
) VALUES (
    '00000000-0000-0000-0000-00000000f141',
    '00000000-0000-0000-0000-00000000f140',
    '00000000-0000-0000-0000-000000000104',
    'instructor', '2026-01-01 00:00:00+00', NULL
);
COMMIT;

DO $$
DECLARE
    v_course_reference bigint;
    v_instructor uuid;
    v_foreign_instructor uuid;
    v_expected_term_starts_on date;
    v_expected_term_ends_on date;
    v_course_time_zone text;
    v_term_starts_on date;
    v_term_ends_on date;
    v_zone text;
    v_assignment_reference bigint;
    v_assignment_edit_number bigint;
    v_saved_edit_number bigint;
    v_expected_due_at timestamptz := TIMESTAMPTZ '2026-01-15 15:30:00.123+00';
    v_expected_due_at_millis bigint;
    v_stored_due_at timestamptz;
    v_returned_due_at_millis bigint;
BEGIN
    SELECT course.reference_number, membership.account_id,
           schedule.term_starts_on, schedule.term_ends_on, schedule.course_time_zone
      INTO v_course_reference, v_instructor,
           v_expected_term_starts_on, v_expected_term_ends_on, v_course_time_zone
      FROM ple_data.course_instance AS course
      JOIN ple_data.course_membership AS membership ON membership.course_id = course.course_id
      JOIN LATERAL (
          SELECT revision.term_starts_on, revision.term_ends_on, revision.course_time_zone
            FROM ple_data.course_schedule_revision AS revision
           WHERE revision.course_id = course.course_id
           ORDER BY revision.revision_number DESC
           LIMIT 1
      ) AS schedule ON true
     WHERE membership.role = 'instructor'
       AND ple_data.course_membership_is_active(membership.membership_id)
       AND schedule.course_time_zone <> 'America/New_York'
     ORDER BY course.reference_number LIMIT 1;
    IF v_course_reference IS NULL THEN
        RAISE EXCEPTION 'M14 requires an Instructor fixture with a non-New-York legacy Course zone';
    END IF;
    SELECT foreign_membership.account_id INTO v_foreign_instructor
      FROM ple_data.course_membership AS foreign_membership
      JOIN ple_data.course_instance AS foreign_course
        ON foreign_course.course_id = foreign_membership.course_id
     WHERE foreign_membership.role = 'instructor'
       AND ple_data.course_membership_is_active(foreign_membership.membership_id)
       AND foreign_membership.account_id <> v_instructor
       AND NOT EXISTS (
           SELECT 1 FROM ple_data.course_membership AS target_membership
            JOIN ple_data.course_instance AS target_course
              ON target_course.course_id = target_membership.course_id
           WHERE target_course.reference_number = v_course_reference
             AND target_membership.account_id = foreign_membership.account_id
             AND target_membership.role = 'instructor'
             AND ple_data.course_membership_is_active(target_membership.membership_id)
       )
     ORDER BY foreign_course.reference_number
     LIMIT 1;
    IF v_foreign_instructor IS NULL THEN
        RAISE EXCEPTION 'M14 requires a real Instructor on a different Course fixture';
    END IF;

    UPDATE ple_private.account_time_zone SET time_zone = 'America/New_York'
     WHERE account_id = v_instructor;
    SET LOCAL ROLE ple_app;
    PERFORM pg_catalog.set_config('ple.session_account_id', v_instructor::text, true);
    SELECT term_starts_on, term_ends_on, account_time_zone
      INTO v_term_starts_on, v_term_ends_on, v_zone
      FROM ple_api.load_live_demo_assignment_schedule_context(v_course_reference);
    IF v_zone IS DISTINCT FROM 'America/New_York' OR v_zone = v_course_time_zone THEN
        RAISE EXCEPTION 'M14 schedule context did not use the authenticated Instructor zone';
    END IF;
    IF v_term_starts_on IS DISTINCT FROM v_expected_term_starts_on
       OR v_term_ends_on IS DISTINCT FROM v_expected_term_ends_on THEN
        RAISE EXCEPTION 'M14 schedule context did not return the Course calendar bounds';
    END IF;
    IF make_timestamptz(2026, 1, 15, 10, 30, 0.123, v_zone) IS DISTINCT FROM v_expected_due_at THEN
        RAISE EXCEPTION 'M14 Instructor-zone wall-clock interpretation did not produce the expected instant';
    END IF;
    v_expected_due_at_millis := floor(extract(epoch FROM v_expected_due_at) * 1000)::bigint;
    SELECT reference_number, assignment_edit_number
      INTO v_assignment_reference, v_assignment_edit_number
      FROM ple_api.create_live_demo_assignment(
          pg_catalog.gen_random_uuid(), v_course_reference,
          'M14 disposable schedule oracle', ''
      );
    SELECT assignment_edit_number INTO v_saved_edit_number
      FROM ple_api.save_live_demo_assignment(
          v_course_reference, v_assignment_reference, v_assignment_edit_number,
          'M14 disposable schedule oracle', '', ARRAY[]::text[],
          v_expected_due_at_millis, 'accept',
          NULL, NULL,
          'answer_all', 'highest', 'unlimited',
          'reuse_selection', 'new_variation', 'resumable',
          'one_question_at_a_time', 'free_navigation', 'authored_order',
          NULL, NULL,
          'after_submit', 'after_submit', 'after_submit',
          'after_submit', 'after_submit', 'never'
      );
    IF v_saved_edit_number <> v_assignment_edit_number + 1 THEN
        RAISE EXCEPTION 'M14 save did not honor the expected edit number';
    END IF;
    SELECT due_at_millis INTO v_returned_due_at_millis
      FROM ple_api.live_demo_assignment_workspace_rows(v_course_reference, v_assignment_reference)
     LIMIT 1;
    RESET ROLE;
    SELECT assignment.due_at INTO v_stored_due_at
      FROM ple_data.assignment AS assignment
     WHERE assignment.reference_number = v_assignment_reference;
    IF v_stored_due_at IS DISTINCT FROM v_expected_due_at
       OR v_returned_due_at_millis IS DISTINCT FROM v_expected_due_at_millis
       OR to_char(v_stored_due_at AT TIME ZONE v_zone, 'YYYY-MM-DD"T"HH24:MI:SS.MS')
          IS DISTINCT FROM '2026-01-15T10:30:00.123' THEN
        RAISE EXCEPTION 'M14 save/store round trip did not preserve the Account-zone local deadline';
    END IF;

    SET LOCAL ROLE ple_app;
    PERFORM pg_catalog.set_config('ple.session_account_id', v_foreign_instructor::text, true);
    IF EXISTS (SELECT 1 FROM ple_api.load_live_demo_assignment_schedule_context(v_course_reference)) THEN
        RAISE EXCEPTION 'M14 schedule context disclosed zone to a foreign Instructor';
    END IF;
    IF EXISTS (SELECT 1 FROM ple_api.load_live_demo_assignment_schedule_context(-9223372036854775808)) THEN
        RAISE EXCEPTION 'M14 schedule context disclosed zone for a missing Course';
    END IF;
    RESET ROLE;
END
$$;

DO $$
BEGIN
    IF has_function_privilege('public', 'ple_api.load_live_demo_assignment_schedule_context(bigint)', 'EXECUTE')
       OR NOT has_function_privilege('ple_app', 'ple_api.load_live_demo_assignment_schedule_context(bigint)', 'EXECUTE') THEN
        RAISE EXCEPTION 'M14 schedule-context function privilege is not exact';
    END IF;
END
$$;
