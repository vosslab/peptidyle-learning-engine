-- M4 compatibility oracle for the installed-session Assignment selection save.
-- It mirrors the seeded provisioner's title, instructions, due date, late-work
-- rule, and ordered selection while expanding omitted JSON members through the
-- current server defaults before calling the same ple_app trusted function.
\set ON_ERROR_STOP on
\set VERBOSITY verbose

-- The direct Course-creation procedure supplies Course Origin.  The compact
-- migration fixture predates that operation, so establish the same immutable
-- prerequisite before exercising the real Assignment Workspace boundary.
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
    '00000000-0000-0000-0000-00000000c402'::uuid,
    membership.account_id,
    'instructor',
    decode(repeat('f0', 32), 'hex'),
    clock_timestamp(),
    clock_timestamp() + interval '1 hour',
    NULL
  FROM ple_data.course_membership AS membership
 WHERE membership.role = 'instructor'
   AND ple_data.course_membership_is_active(membership.membership_id)
 ORDER BY membership.joined_at, membership.membership_id
 LIMIT 1;

SELECT course.reference_number AS m4_selection_course_reference
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
  FROM ple_api.resolve_and_install_session(decode(repeat('f0', 32), 'hex'))
\gset
SET LOCAL ROLE ple_app;

-- Start from the exact pre-selection state the provisioner reaches after its
-- Assignment stage, then save the seeded selection through the current
-- 27-argument Store contract.  NEW-0001 is the available immutable Question
-- supplied by the preceding publication oracle in this disposable fixture.
SELECT reference_number AS m4_selection_assignment_reference,
       assignment_edit_number AS m4_selection_initial_edit_number
  FROM ple_api.create_live_demo_assignment(
      gen_random_uuid(),
      :'m4_selection_course_reference'::bigint,
      'Peptide Structure Practice',
      'Complete the four practice questions on peptide structure and properties.'
  )
\gset

SELECT reference_number AS m4_selection_saved_reference,
       assignment_edit_number AS m4_selection_saved_edit_number,
       assignment_status AS m4_selection_saved_status,
       assignment_title AS m4_selection_saved_title,
       assignment_instructions AS m4_selection_saved_instructions
  FROM ple_api.save_live_demo_assignment(
      :'m4_selection_course_reference'::bigint,
      :'m4_selection_assignment_reference'::bigint,
      :'m4_selection_initial_edit_number'::bigint,
      'Peptide Structure Practice',
      'Complete the four practice questions on peptide structure and properties.',
      ARRAY['NEW-0001']::text[],
      NULL,
      'accept',
      NULL,
      NULL,
      'answer_all',
      'highest',
      'unlimited',
      'reuse_selection',
      'new_variation',
      'resumable',
      'one_question_at_a_time',
      'free_navigation',
      'authored_order',
      NULL,
      NULL,
      'after_submit',
      'after_submit',
      'after_submit',
      'after_submit',
      'after_submit',
      'never'
  )
\gset
COMMIT;

SELECT CASE
    WHEN :'m4_selection_saved_reference'::bigint = :'m4_selection_assignment_reference'::bigint
     AND :'m4_selection_saved_edit_number'::bigint = 2
     AND :'m4_selection_saved_status' = 'unreleased'
     AND :'m4_selection_saved_title' = 'Peptide Structure Practice'
     AND :'m4_selection_saved_instructions' =
         'Complete the four practice questions on peptide structure and properties.'
    THEN 1
    ELSE 1 / 0
END AS m4_selection_returned_expected_row;

WITH selected AS (
    SELECT array_agg(question_id ORDER BY question_index) AS question_ids,
           array_agg(question_index ORDER BY question_index) AS question_indexes
      FROM ple_private.live_demo_assignment_question AS selected
      JOIN ple_data.assignment AS assignment
        ON assignment.assignment_id = selected.assignment_id
     WHERE assignment.reference_number = :'m4_selection_assignment_reference'::bigint
), persisted AS (
    SELECT 1
      FROM ple_data.assignment AS assignment
     WHERE assignment.reference_number = :'m4_selection_assignment_reference'::bigint
       AND assignment_edit_number = 2
       AND live_demo_question_selection_version = 2
       AND late_work_rule = 'accept'
       AND due_at IS NULL
       AND assignment_question_display_rule = 'one_question_at_a_time'
       AND assignment_navigation_rule = 'free_navigation'
       AND assignment_question_order_rule = 'authored_order'
       AND feedback_score = 'after_submit'
       AND feedback_per_item_correctness = 'after_submit'
       AND feedback_question_feedback = 'after_submit'
       AND feedback_question_answer = 'after_submit'
       AND feedback_question_answer_explanation = 'after_submit'
       AND feedback_class_statistics = 'never'
)
SELECT selected.question_ids = ARRAY['NEW-0001']::text[]
       AND selected.question_indexes = ARRAY[0]::integer[]
       AND EXISTS (SELECT 1 FROM persisted) AS m4_selection_persisted_expected_state
  FROM selected
\gset
\if :m4_selection_persisted_expected_state
\else
SELECT 1 / 0;
\endif

WITH expected AS (
    SELECT *
      FROM (VALUES
          ('assignment'::text, 'feedback_score'::text, 'after_submit'::text),
          ('assignment', 'feedback_per_item_correctness', 'after_submit'),
          ('assignment', 'feedback_question_feedback', 'after_submit'),
          ('assignment', 'feedback_question_answer', 'after_submit'),
          ('assignment', 'feedback_question_answer_explanation', 'after_submit'),
          ('assignment', 'feedback_class_statistics', 'never'),
          ('assignment_revision', 'feedback_score', 'after_submit'),
          ('assignment_revision', 'feedback_per_item_correctness', 'after_submit'),
          ('assignment_revision', 'feedback_question_feedback', 'after_submit'),
          ('assignment_revision', 'feedback_question_answer', 'after_submit'),
          ('assignment_revision', 'feedback_question_answer_explanation', 'after_submit'),
          ('assignment_revision', 'feedback_class_statistics', 'never')
      ) AS defaults(table_name, column_name, default_value)
), actual AS (
    SELECT expected.table_name, expected.column_name
      FROM expected
      JOIN information_schema.columns AS column_definition
        ON column_definition.table_schema = 'ple_data'
       AND column_definition.table_name = expected.table_name
       AND column_definition.column_name = expected.column_name
     WHERE column_definition.column_default LIKE '%' || expected.default_value || '%'
)
SELECT (SELECT count(*) FROM actual) = 12 AS m4_current_and_revision_defaults
\gset
\if :m4_current_and_revision_defaults
\else
SELECT 1 / 0;
\endif
