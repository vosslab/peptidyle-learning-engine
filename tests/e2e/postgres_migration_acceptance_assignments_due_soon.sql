-- M10 cross-Course Assignments Due Soon acceptance oracle.
--
-- The disposable setup reuses the seeded Instructor and the M14 foreign
-- Course, then rolls back its membership and Assignment mutations.  The
-- public read is the only operation performed as ple_app.
\set ON_ERROR_STOP on
\set VERBOSITY verbose

DO $$
BEGIN
    IF to_regprocedure('ple_api.list_assignments_due_soon()') IS NULL
       OR NOT has_function_privilege(
           'ple_app', 'ple_api.list_assignments_due_soon()', 'EXECUTE'
       )
       OR has_function_privilege(
           'public', 'ple_api.list_assignments_due_soon()', 'EXECUTE'
       )
       OR (SELECT pg_get_userbyid(proowner)
             FROM pg_proc
            WHERE oid = 'ple_api.list_assignments_due_soon()'::regprocedure)
          <> 'ple_api_owner' THEN
        RAISE EXCEPTION 'M10 Assignments Due Soon reader authority is not exact';
    END IF;
END
$$;

BEGIN;

-- M11's established Instructor and released Assignment provide the first
-- taught Course. M14's separate-Instructor Course becomes the second taught
-- Course only within this rollback-only oracle.
SET LOCAL ROLE ple_data_owner;
DO $$
DECLARE
    v_instructor_account_id uuid;
    v_primary_course_id uuid;
    v_secondary_course_id uuid;
    v_released_assignment_id uuid;
    v_secondary_membership_id uuid := pg_catalog.gen_random_uuid();
    v_now timestamp with time zone := pg_catalog.statement_timestamp();
BEGIN
    v_instructor_account_id := '00000000-0000-0000-0000-000000000102'::uuid;

    SELECT course.course_id, assignment.assignment_id
      INTO v_primary_course_id, v_released_assignment_id
      FROM ple_data.course_instance AS course
      JOIN ple_data.course_membership AS membership
        ON membership.course_id = course.course_id
       AND membership.account_id = v_instructor_account_id
       AND membership.role = 'instructor'
       AND ple_data.course_membership_is_active(membership.membership_id)
      JOIN ple_data.assignment AS assignment
        ON assignment.course_id = course.course_id
       AND assignment.assignment_status = 'released'
     ORDER BY course.reference_number, assignment.reference_number
     LIMIT 1;
    IF v_primary_course_id IS NULL THEN
        RAISE EXCEPTION 'M10 requires a seeded released Assignment for the established Instructor';
    END IF;

    SELECT course.course_id
      INTO v_secondary_course_id
      FROM ple_data.course_instance AS course
     WHERE course.course_long_name = 'M14 foreign Instructor fixture';
    IF v_secondary_course_id IS NULL THEN
        RAISE EXCEPTION 'M10 requires the M14 foreign Course fixture';
    END IF;

    PERFORM pg_catalog.set_config(
        'ple_e2e.m10_instructor_account_id', v_instructor_account_id::text, true
    );
    PERFORM pg_catalog.set_config(
        'ple_e2e.m10_primary_course_id', v_primary_course_id::text, true
    );
    PERFORM pg_catalog.set_config(
        'ple_e2e.m10_secondary_course_id', v_secondary_course_id::text, true
    );
    PERFORM pg_catalog.set_config(
        'ple_e2e.m10_released_assignment_id', v_released_assignment_id::text, true
    );
    PERFORM pg_catalog.set_config(
        'ple_e2e.m10_secondary_membership_id', v_secondary_membership_id::text, true
    );
    PERFORM pg_catalog.set_config('ple_e2e.m10_now', v_now::text, true);
END
$$;

-- Course Membership episodes and their immutable events are created through
-- the same restricted ple_api_owner capability used by the Course setup.
SET LOCAL ROLE ple_api_owner;
INSERT INTO ple_data.course_membership (
    membership_id, course_id, account_id, role, joined_at
) VALUES (
    current_setting('ple_e2e.m10_secondary_membership_id')::uuid,
    current_setting('ple_e2e.m10_secondary_course_id')::uuid,
    current_setting('ple_e2e.m10_instructor_account_id')::uuid,
    'instructor', current_setting('ple_e2e.m10_now')::timestamptz
);

SET LOCAL ROLE ple_data_owner;
DO $$
DECLARE
    v_primary_course_id uuid := current_setting('ple_e2e.m10_primary_course_id')::uuid;
    v_secondary_course_id uuid := current_setting('ple_e2e.m10_secondary_course_id')::uuid;
    v_released_assignment_id uuid := current_setting('ple_e2e.m10_released_assignment_id')::uuid;
    v_template ple_data.assignment%ROWTYPE;
    v_now timestamp with time zone := current_setting('ple_e2e.m10_now')::timestamptz;
    v_expected jsonb;
BEGIN

    -- Remove incidental seeded deadlines from the two Courses. The controlled
    -- rows below are the complete eligible projection for this statement.
    UPDATE ple_data.assignment
       SET due_at = NULL,
           assignment_edit_number = assignment_edit_number + 1,
           updated_at = v_now
     WHERE course_id IN (v_primary_course_id, v_secondary_course_id)
       AND due_at IS NOT NULL;

    SELECT * INTO v_template
      FROM ple_data.assignment
     WHERE assignment_id = v_released_assignment_id;

    UPDATE ple_data.assignment
       SET assignment_title = 'M10 released current Assignment',
           due_at = v_now + interval '1 hour',
           assignment_edit_number = assignment_edit_number + 1,
           updated_at = v_now
     WHERE assignment_id = v_released_assignment_id;

    INSERT INTO ple_data.assignment (
        assignment_id, course_id, source_blueprint_course_reference_number,
        source_blueprint_revision_number, created_at, updated_at,
        assignment_edit_number, assignment_title, assignment_instructions,
        available_at, due_at, closes_at, assignment_attempt_time_limit_seconds,
        attempt_limit, late_work_rule, assignment_deadline_rule,
        assignment_completion_rule, assignment_completion_score_threshold,
        assignment_attempt_grade_rule, assignment_attempt_continuation_rule,
        max_additional_assignment_attempts, question_pool_reuse_rule,
        question_variation_rule, assignment_attempt_resume_rule,
        assignment_question_display_rule, assignment_navigation_rule,
        assignment_question_order_rule, assignment_status,
        released_assignment_revision_id
    )
    SELECT pg_catalog.gen_random_uuid(), fixture.course_id,
           v_template.source_blueprint_course_reference_number,
           v_template.source_blueprint_revision_number, v_now, v_now,
           1, fixture.assignment_title, '', NULL,
           v_now + fixture.due_offset, NULL, NULL, NULL,
           'reject', 'auto_submit', 'answer_all', NULL, 'highest', 'unlimited',
           NULL, 'reuse_selection', 'new_variation', 'resumable',
           'one_question_at_a_time', 'free_navigation', 'shuffled',
           fixture.assignment_status, NULL
      FROM (VALUES
          (v_secondary_course_id, 'M10 second Course Assignment', 'unreleased', interval '2 hours'),
          (v_primary_course_id, 'M10 past Assignment', 'unreleased', interval '-1 hour'),
          (v_primary_course_id, 'M10 null-due Assignment', 'unreleased', NULL::interval),
          (v_primary_course_id, 'M10 outside-window Assignment', 'unreleased', interval '8 days'),
          (v_primary_course_id, 'M10 closed Assignment', 'closed', interval '3 hours'),
          (v_primary_course_id, 'M10 archived Assignment', 'archived', interval '4 hours')
      ) AS fixture(course_id, assignment_title, assignment_status, due_offset);

    SELECT jsonb_agg(
               jsonb_build_object(
                   'course_reference_number', course.reference_number,
                   'course_long_name', course.course_long_name,
                   'assignment_reference_number', assignment.reference_number,
                   'assignment_title', assignment.assignment_title,
                   'assignment_status', assignment.assignment_status,
                   'due_at_millis', floor(extract(epoch FROM assignment.due_at) * 1000)::bigint
               )
               ORDER BY assignment.due_at, course.reference_number, assignment.reference_number
           )
      INTO v_expected
      FROM ple_data.assignment AS assignment
      JOIN ple_data.course_instance AS course ON course.course_id = assignment.course_id
     WHERE assignment.assignment_title IN (
         'M10 released current Assignment', 'M10 second Course Assignment'
     );

    PERFORM pg_catalog.set_config('ple_e2e.m10_expected_projection', v_expected::text, true);
END
$$;

SET LOCAL ROLE ple_auth;
SELECT ple_api.resolve_and_install_session(decode(repeat('f1', 32), 'hex'));
SET LOCAL ROLE ple_app;

DO $$
DECLARE
    v_actual jsonb;
    v_expected jsonb;
BEGIN
    SELECT jsonb_agg(
               jsonb_build_object(
                   'course_reference_number', item.course_reference_number,
                   'course_long_name', item.course_long_name,
                   'assignment_reference_number', item.assignment_reference_number,
                   'assignment_title', item.assignment_title,
                   'assignment_status', item.assignment_status,
                   'due_at_millis', item.due_at_millis
               )
               ORDER BY item.due_at_millis, item.course_reference_number,
                        item.assignment_reference_number
           )
      INTO v_actual
      FROM ple_api.list_assignments_due_soon() AS item;

    v_expected := current_setting('ple_e2e.m10_expected_projection')::jsonb;

    IF v_actual IS DISTINCT FROM v_expected
       OR EXISTS (
           SELECT 1
             FROM ple_api.list_assignments_due_soon() AS item,
                  LATERAL jsonb_object_keys(to_jsonb(item)) AS key(name)
            WHERE key.name NOT IN (
                'course_reference_number', 'course_long_name',
                'assignment_reference_number', 'assignment_title',
                'assignment_status', 'due_at_millis'
            )
       ) THEN
        RAISE EXCEPTION 'M10 Assignments Due Soon projection, window, status, or disclosure boundary is not exact';
    END IF;
END
$$;

-- Ending the second Course membership must remove that Course immediately
-- without changing the first Course's eligible Assignment.
SET LOCAL ROLE ple_api_owner;
INSERT INTO ple_data.course_membership_event (
    course_membership_event_id, membership_id, event_kind, occurred_at, reason
) VALUES (
    pg_catalog.gen_random_uuid(),
    current_setting('ple_e2e.m10_secondary_membership_id')::uuid,
    'ended', pg_catalog.clock_timestamp(),
    'M10 Assignments Due Soon membership revocation acceptance'
);
SET LOCAL ROLE ple_app;

DO $$
BEGIN
    IF EXISTS (
        SELECT 1
          FROM ple_api.list_assignments_due_soon() AS item
         WHERE item.assignment_title = 'M10 second Course Assignment'
    ) OR NOT EXISTS (
        SELECT 1
          FROM ple_api.list_assignments_due_soon() AS item
         WHERE item.assignment_title = 'M10 released current Assignment'
    ) THEN
        RAISE EXCEPTION 'M10 Assignments Due Soon did not drop revoked Course membership rows';
    END IF;
END
$$;

ROLLBACK;
