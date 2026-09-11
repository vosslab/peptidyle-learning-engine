-- M4 compatibility oracle for the exact seeded Assignment-create capability.
-- Resolve an active Instructor session through ple_auth, then call the same
-- four-argument ple_app function used by live-demo provisioning.
\set ON_ERROR_STOP on
\set VERBOSITY verbose

-- The older Assignment Attempt fixture predates Course Origin. Add the one
-- immutable origin fact that the live Course-creation operation always owns.
INSERT INTO ple_data.course_origin (
    course_origin_id,
    course_id,
    blueprint_course_reference_number,
    blueprint_revision_number,
    source_course_id,
    created_at,
    evidence
)
SELECT
    gen_random_uuid(),
    course.course_id,
    course.blueprint_course_reference_number,
    course.blueprint_revision_number,
    NULL,
    clock_timestamp(),
    '{}'::jsonb
  FROM ple_data.course_instance AS course
  JOIN ple_data.course_membership AS membership
    ON membership.course_id = course.course_id
 WHERE membership.role = 'instructor'
   AND ple_data.course_membership_is_active(membership.membership_id)
 ORDER BY course.reference_number
 LIMIT 1
ON CONFLICT (course_id) DO NOTHING;

INSERT INTO ple_private.authenticated_session (
    session_id, account_id, product_role, token_hash, created_at, expires_at, revoked_at
)
SELECT
    '00000000-0000-0000-0000-00000000c401'::uuid,
    membership.account_id,
    'instructor',
    decode(repeat('ef', 32), 'hex'),
    clock_timestamp(),
    clock_timestamp() + interval '1 hour',
    NULL
  FROM ple_data.course_membership AS membership
 WHERE membership.role = 'instructor'
   AND ple_data.course_membership_is_active(membership.membership_id)
 ORDER BY membership.joined_at, membership.membership_id
 LIMIT 1;

SELECT course.reference_number AS m4_course_reference
  FROM ple_data.course_instance AS course
  JOIN ple_data.course_membership AS membership
    ON membership.course_id = course.course_id
 WHERE membership.role = 'instructor'
   AND ple_data.course_membership_is_active(membership.membership_id)
 ORDER BY course.reference_number
 LIMIT 1
\gset

BEGIN;
SET LOCAL ROLE ple_auth;
SELECT session_id
  FROM ple_api.resolve_and_install_session(decode(repeat('ef', 32), 'hex'))
\gset
SET LOCAL ROLE ple_app;
SELECT reference_number AS m4_assignment_reference,
       assignment_edit_number AS m4_assignment_edit_number,
       assignment_status AS m4_assignment_status,
       assignment_title AS m4_assignment_title,
       assignment_instructions AS m4_assignment_instructions
  FROM ple_api.create_live_demo_assignment(
      gen_random_uuid(),
      :'m4_course_reference'::bigint,
      'M4 create compatibility oracle',
      ''
  )
\gset
COMMIT;

SELECT CASE
    WHEN :'m4_assignment_edit_number'::bigint = 1
     AND :'m4_assignment_status' = 'unreleased'
     AND :'m4_assignment_title' = 'M4 create compatibility oracle'
     AND :'m4_assignment_instructions' = ''
    THEN 1
    ELSE 1 / 0
END AS m4_create_returned_expected_row;

SELECT EXISTS (
    SELECT 1
      FROM ple_data.assignment AS assignment
     WHERE assignment.reference_number = :'m4_assignment_reference'::bigint
       AND assignment.assignment_status = 'unreleased'
       AND assignment.assignment_edit_number = 1
       AND assignment.late_work_rule = 'reject'
       AND assignment.assignment_attempt_grade_rule = 'highest'
       AND assignment.question_pool_reuse_rule = 'reuse_selection'
       AND assignment.question_variation_rule = 'new_variation'
       AND assignment.assignment_question_display_rule = 'one_question_at_a_time'
       AND assignment.assignment_navigation_rule = 'free_navigation'
       AND assignment.assignment_question_order_rule = 'authored_order'
       AND assignment.feedback_score = 'after_submit'
       AND assignment.feedback_per_item_correctness = 'after_submit'
       AND assignment.feedback_question_feedback = 'after_submit'
       AND assignment.feedback_question_answer = 'after_submit'
       AND assignment.feedback_question_answer_explanation = 'after_submit'
       AND assignment.feedback_class_statistics = 'never'
) AS m4_create_persisted_expected_defaults
\gset
\if :m4_create_persisted_expected_defaults
\else
SELECT 1 / (
    SELECT count(*) FROM pg_roles
     WHERE rolname = current_user AND false
);
\endif
