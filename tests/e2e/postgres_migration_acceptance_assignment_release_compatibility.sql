-- M4 compatibility oracle for the installed-session Assignment release call.
-- It follows the create and selected-Question fixtures and invokes the exact
-- SECURITY DEFINER procedure used by the Live Demo Stage.RELEASE request.
\set ON_ERROR_STOP on
\set VERBOSITY verbose

DO $$
BEGIN
    IF to_regprocedure(
        'ple_api.release_live_demo_assignment(uuid,bigint,bigint,bigint)'
    ) IS NULL
       OR NOT has_function_privilege(
           'ple_app',
           'ple_api.release_live_demo_assignment(uuid,bigint,bigint,bigint)',
           'EXECUTE'
       )
       OR has_function_privilege(
           'public',
           'ple_api.release_live_demo_assignment(uuid,bigint,bigint,bigint)',
           'EXECUTE'
       ) THEN
        RAISE EXCEPTION 'Assignment release procedure authority is not exact';
    END IF;
END
$$;

INSERT INTO ple_private.authenticated_session (
    session_id, account_id, product_role, token_hash, created_at, expires_at, revoked_at
)
SELECT
    '00000000-0000-0000-0000-00000000c403'::uuid,
    membership.account_id,
    'instructor',
    decode(repeat('f1', 32), 'hex'),
    clock_timestamp(),
    clock_timestamp() + interval '1 hour',
    NULL
  FROM ple_data.course_membership AS membership
 WHERE membership.role = 'instructor'
   AND ple_data.course_membership_is_active(membership.membership_id)
 ORDER BY membership.joined_at, membership.membership_id
 LIMIT 1;

SELECT course.reference_number AS m4_release_course_reference,
       assignment.reference_number AS m4_release_assignment_reference,
       assignment.assignment_edit_number AS m4_release_expected_edit_number
  FROM ple_data.assignment AS assignment
  JOIN ple_data.course_instance AS course
    ON course.course_id = assignment.course_id
 WHERE assignment.assignment_status = 'unreleased'
   AND EXISTS (
       SELECT 1
         FROM ple_private.live_demo_assignment_question AS selected
        WHERE selected.assignment_id = assignment.assignment_id
   )
 ORDER BY assignment.reference_number DESC
 LIMIT 1
\gset

BEGIN;
SET LOCAL ROLE ple_auth;
SELECT session_id
  FROM ple_api.resolve_and_install_session(decode(repeat('f1', 32), 'hex'))
\gset
SET LOCAL ROLE ple_app;
SELECT reference_number AS m4_release_returned_reference,
       revision_number AS m4_release_returned_revision_number
  FROM ple_api.release_live_demo_assignment(
      gen_random_uuid(),
      :'m4_release_course_reference'::bigint,
      :'m4_release_assignment_reference'::bigint,
      :'m4_release_expected_edit_number'::bigint
  )
\gset
COMMIT;

SELECT CASE
    WHEN :'m4_release_returned_reference'::bigint =
             :'m4_release_assignment_reference'::bigint
     AND :'m4_release_returned_revision_number'::bigint = 1
    THEN 1
    ELSE 1 / 0
END AS m4_release_returned_expected_receipt;

SELECT EXISTS (
    SELECT 1
      FROM ple_data.assignment AS assignment
      JOIN ple_data.assignment_revision AS revision
        ON revision.assignment_revision_id = assignment.released_assignment_revision_id
       AND revision.assignment_id = assignment.assignment_id
       AND revision.course_id = assignment.course_id
     WHERE assignment.reference_number = :'m4_release_assignment_reference'::bigint
       AND assignment.assignment_status = 'released'
       AND revision.revision_number = 1
       AND (
           SELECT count(*)
             FROM ple_data.assignment_revision_fixed_question AS fixed_question
            WHERE fixed_question.assignment_revision_id = revision.assignment_revision_id
       ) = 1
) AS m4_release_persisted_expected_state
\gset
\if :m4_release_persisted_expected_state
\else
SELECT 1 / 0;
\endif

-- Release must preserve the current M4 policy/default snapshot, including every
-- independently controlled disclosure field.  The preceding selected-Question
-- fixture deliberately changes late-work to accept, so compare the released
-- snapshot to current Assignment state before checking the remaining defaults.
SELECT EXISTS (
    SELECT 1
      FROM ple_data.assignment AS assignment
      JOIN ple_data.assignment_revision AS revision
        ON revision.assignment_revision_id = assignment.released_assignment_revision_id
     WHERE assignment.reference_number = :'m4_release_assignment_reference'::bigint
       AND revision.late_work_rule IS NOT DISTINCT FROM assignment.late_work_rule
       AND revision.assignment_deadline_rule IS NOT DISTINCT FROM assignment.assignment_deadline_rule
       AND revision.assignment_completion_rule IS NOT DISTINCT FROM assignment.assignment_completion_rule
       AND revision.assignment_attempt_continuation_rule IS NOT DISTINCT FROM assignment.assignment_attempt_continuation_rule
       AND revision.assignment_question_display_rule IS NOT DISTINCT FROM assignment.assignment_question_display_rule
       AND revision.assignment_navigation_rule IS NOT DISTINCT FROM assignment.assignment_navigation_rule
       AND revision.assignment_question_order_rule IS NOT DISTINCT FROM assignment.assignment_question_order_rule
       AND revision.assignment_attempt_time_limit_seconds IS NOT DISTINCT FROM assignment.assignment_attempt_time_limit_seconds
       AND revision.feedback_score IS NOT DISTINCT FROM assignment.feedback_score
       AND revision.feedback_per_item_correctness IS NOT DISTINCT FROM assignment.feedback_per_item_correctness
       AND revision.feedback_submitted_response IS NOT DISTINCT FROM assignment.feedback_submitted_response
       AND revision.feedback_question_feedback IS NOT DISTINCT FROM assignment.feedback_question_feedback
       AND revision.feedback_question_answer IS NOT DISTINCT FROM assignment.feedback_question_answer
       AND revision.feedback_question_answer_explanation IS NOT DISTINCT FROM assignment.feedback_question_answer_explanation
       AND revision.feedback_class_statistics IS NOT DISTINCT FROM assignment.feedback_class_statistics
       AND revision.late_work_rule = 'accept'
       AND revision.assignment_deadline_rule = 'auto_submit'
       AND revision.assignment_completion_rule = 'answer_all'
       AND revision.assignment_attempt_continuation_rule = 'unlimited'
       AND revision.assignment_question_display_rule = 'one_question_at_a_time'
       AND revision.assignment_navigation_rule = 'free_navigation'
       AND revision.assignment_question_order_rule = 'authored_order'
       AND revision.assignment_attempt_time_limit_seconds = 1800
       AND revision.feedback_score = 'after_submit'
       AND revision.feedback_per_item_correctness = 'after_submit'
       AND revision.feedback_submitted_response = 'after_submit'
       AND revision.feedback_question_feedback = 'after_submit'
       AND revision.feedback_question_answer = 'after_submit'
       AND revision.feedback_question_answer_explanation = 'after_submit'
       AND revision.feedback_class_statistics = 'never'
) AS m4_release_snapshot_keeps_policy_and_disclosure_defaults
\gset
\if :m4_release_snapshot_keeps_policy_and_disclosure_defaults
\else
SELECT 1 / 0;
\endif
