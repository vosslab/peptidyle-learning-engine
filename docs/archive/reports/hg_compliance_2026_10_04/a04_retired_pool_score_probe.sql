-- One-time Human Guidance A-04 audit diagnostic, not a permanent regression test.
-- Freshly replayed successfully on 2026-10-04; see a04_replay_pass.log.
-- Run in a disposable database installed like tests/e2e/e2e_assessment_saved_response.sh:
--   podman exec "$name" psql -X -v ON_ERROR_STOP=1 -U postgres -d "$database" \
--     -f /workspace/docs/active_plans/reports/hg_compliance_2026_10_04/a04_retired_pool_score_probe.sql
-- It uses the standard oracle's source-bound Question, then creates an
-- isolated two-entry Assessment. The assertion captures the CURRENT incorrect
-- result (2 / 2); HG requires 1 / 1 after the Pool is completely removed.
\set ON_ERROR_STOP on
\ir ../../../../tests/e2e/assessment_saved_response_oracle.sql

BEGIN;
SET CONSTRAINTS ALL DEFERRED;

-- Create an available root Pool containing the oracle's source-bound Question,
-- then create a direct Assessment with one fixed and one forked-Pool Entry.
SELECT 'A04P-' || ple_private.crockford_checksum_character('A04PXYZ') || 'XYZ'
    AS a04_source_pool_id \gset
SELECT 'A04F-' || ple_private.crockford_checksum_character('A04FXYZ') || 'XYZ'
    AS a04_fork_pool_id \gset
SELECT 'ASVR0003' || ple_private.crockford_checksum_character('ASVR0003')
    AS a04_assessment_id \gset

SET LOCAL ROLE ple_app;
SELECT set_config('ple.session_account_id', :'instructor_id', true);
SELECT question_pool_id AS a04_source_pool_id, question_pool_edit_number AS a04_source_pool_edit
  FROM ple_api.create_question_pool(
    :'a04_source_pool_id', ARRAY[:'question_id'], ARRAY[1], true,
    'A04 audit Pool', 'Disposable A04 audit Pool', ARRAY[]::text[]
  ) \gset
SELECT assessment_id AS a04_assessment_id, assessment_edit_number AS a04_edit
  FROM ple_api.create_assessment(
    :'a04_assessment_id', :'course_id', 'regular_assignment',
    'A04 audit Assessment', 'Disposable A04 audit Assessment'
  ) \gset

-- Copy the server-owned policy fields, adding the due date needed for release.
SET LOCAL ROLE ple_data_owner;
SELECT jsonb_set(
    jsonb_build_object(
      'assessment_title', snapshot.assessment_title,
      'assessment_instructions', snapshot.assessment_instructions,
      'available_at', snapshot.available_at, 'due_at', snapshot.due_at,
      'closes_at', snapshot.closes_at,
      'assessment_attempt_time_limit_seconds', snapshot.assessment_attempt_time_limit_seconds,
      'assessment_attempt_limit', snapshot.assessment_attempt_limit,
      'late_work_rule', snapshot.late_work_rule::text,
      'question_variation_rule', snapshot.question_variation_rule::text,
      'assessment_question_order_rule', snapshot.assessment_question_order_rule::text,
      'feedback_score', snapshot.feedback_score::text,
      'feedback_per_item_correctness', snapshot.feedback_per_item_correctness::text,
      'feedback_submitted_response', snapshot.feedback_submitted_response::text,
      'feedback_question_answer', snapshot.feedback_question_answer::text,
      'feedback_question_answer_explanation', snapshot.feedback_question_answer_explanation::text,
      'feedback_class_statistics', snapshot.feedback_class_statistics::text,
      'feedback_hints', snapshot.feedback_hints::text,
      'feedback_worked_solutions', snapshot.feedback_worked_solutions::text
    ), '{due_at}', to_jsonb(pg_catalog.clock_timestamp() + interval '25 hours')
  )::text AS a04_values
  FROM ple_data.assessment AS assessment
  JOIN ple_data.assessment_policy_snapshot AS snapshot
    ON snapshot.assessment_policy_snapshot_id = assessment.assessment_policy_snapshot_id
 WHERE assessment.assessment_id = :'a04_assessment_id' \gset

SET LOCAL ROLE ple_app;
SELECT assessment_edit_number AS a04_edit
  FROM ple_api.save_assessment(
    :'course_id', :'a04_assessment_id', :'a04_edit', :'a04_values'::jsonb,
    jsonb_build_array(jsonb_build_object(
      'assessmentEntryId', '73000000-0000-0000-0000-000000000101',
      'kind', 'fixed_question', 'availability', 'available', 'scoringRule', 'normal',
      'authoredPosition', 0, 'questionId', :'question_id', 'revisionNumber', 1,
      'pointsPossible', 1
    ))
  ) \gset
SELECT assessment_edit_number AS a04_edit
  FROM ple_api.import_assessment_question_pool_fork_for_ids(
    :'course_id', :'a04_assessment_id', '73000000-0000-0000-0000-000000000102',
    :'a04_edit', :'a04_fork_pool_id', :'a04_source_pool_id', :'a04_source_pool_edit',
    1, 1, 1, 'question_pool_order', 'normal'
  ) \gset
SELECT assessment_status AS a04_released_status, assessment_edit_number AS a04_edit
  FROM ple_api.release_assessment(:'course_id', :'a04_assessment_id', :'a04_edit') \gset

-- Issue and finalize both Entries through the normal Student boundaries.
SELECT set_config('ple.session_account_id', :'student_id', true);
SELECT assessment_attempt_id AS a04_attempt_id
  FROM ple_api.start_assessment_attempt(
    '73000000-0000-0000-0000-000000000110',
    '73000000-0000-0000-0000-000000000201', :'a04_assessment_id',
    jsonb_build_array(jsonb_build_object(
      'question_pool_selection_id', '73000000-0000-0000-0000-000000000111',
      'assessment_entry_id', '73000000-0000-0000-0000-000000000102',
      'selected_items', jsonb_build_array(1)
    )),
    jsonb_build_array(
      jsonb_build_object(
        'issued_question_id', '73000000-0000-0000-0000-000000000112',
        'assessment_entry_id', '73000000-0000-0000-0000-000000000101',
        'issued_position', 0, 'published_question_id', :'question_id', 'revision_number', 1
      ),
      jsonb_build_object(
        'issued_question_id', '73000000-0000-0000-0000-000000000113',
        'assessment_entry_id', '73000000-0000-0000-0000-000000000102',
        'issued_position', 1, 'published_question_id', :'question_id', 'revision_number', 1,
        'question_pool_selection_id', '73000000-0000-0000-0000-000000000111',
        'question_pool_member_position', 1
      )
    )
  ) \gset

RESET ROLE;
SET LOCAL ROLE ple_private_owner;
INSERT INTO ple_private.question_attempt (
    course_instance_id, question_attempt_id, issued_question_id, issued_at,
    delivery_toolchain_id, rendered_question_sha256
)
SELECT issued.course_instance_id,
       CASE issued.issued_question_id
         WHEN '73000000-0000-0000-0000-000000000112'::uuid THEN '73000000-0000-0000-0000-000000000114'::uuid
         ELSE '73000000-0000-0000-0000-000000000115'::uuid
       END,
       issued.issued_question_id, pg_catalog.transaction_timestamp(),
       ple_private.ensure_delivery_toolchain('ple', '1', NULL, NULL, 'ple', '1', 'not_applicable'),
       decode(repeat('ab', 32), 'hex')
  FROM ple_private.issued_question AS issued
 WHERE issued.assessment_attempt_id = :'a04_attempt_id'::uuid;

SET LOCAL ROLE ple_app;
SELECT set_config('ple.session_account_id', :'student_id', true);
SELECT response_state FROM ple_api.save_student_assessment_attempt_response(
    :'a04_attempt_id'::uuid, 1, '{"choice":"fixed"}'::jsonb
);
SELECT response_state FROM ple_api.save_student_assessment_attempt_response(
    :'a04_attempt_id'::uuid, 2, '{"choice":"pool"}'::jsonb
);
SELECT points_earned, points_possible FROM ple_api.commit_student_assessment_attempt_finalization(
  :'a04_attempt_id'::uuid, 'student',
  (SELECT jsonb_agg(jsonb_build_object(
    'question_attempt_id', prepared.question_attempt_id, 'saved_at_millis', prepared.saved_at_millis,
    'student_response', prepared.student_response, 'normalized_credit', 1
  )) FROM ple_api.prepare_student_assessment_attempt_finalization(:'a04_attempt_id'::uuid) AS prepared
     WHERE prepared.preparation_state = 'ready' AND prepared.question_attempt_id IS NOT NULL)
);

-- Remove only the Pool through normal save, preserving the fixed Entry.
SELECT set_config('ple.session_account_id', :'instructor_id', true);
SELECT * FROM ple_api.save_assessment(
  :'course_id', :'a04_assessment_id', :'a04_edit', :'a04_values'::jsonb,
  jsonb_build_array(jsonb_build_object(
    'assessmentEntryId', '73000000-0000-0000-0000-000000000101',
    'kind', 'fixed_question', 'availability', 'available', 'scoringRule', 'normal',
    'authoredPosition', 0, 'questionId', :'question_id', 'revisionNumber', 1, 'pointsPossible', 1
  ))
);

-- HG requires 1 / 1 now. Current code retains the retired Pool's 1 / 1.
SELECT set_config('ple.session_account_id', :'student_id', true);
SELECT points_earned, points_possible FROM ple_api.prepare_student_assessment_attempt_finalization(
  :'a04_attempt_id'::uuid
) WHERE preparation_state = 'already_submitted' \gset
SELECT set_config('ple.a04_released_status', :'a04_released_status', true);
SELECT set_config('ple.a04_points_earned', :'points_earned', true);
SELECT set_config('ple.a04_points_possible', :'points_possible', true);
DO $$
BEGIN
  IF current_setting('ple.a04_released_status') <> 'released'
     OR current_setting('ple.a04_points_earned')::numeric <> 2
     OR current_setting('ple.a04_points_possible')::numeric <> 2 THEN
    RAISE EXCEPTION 'A04 current behavior changed: status=% earned=% possible=%',
      current_setting('ple.a04_released_status'), current_setting('ple.a04_points_earned'),
      current_setting('ple.a04_points_possible');
  END IF;
  RAISE NOTICE 'a04_retired_pool_still_scores_existing_attempt | 1 (removed Pool contributes 1 / 1; total remains 2 / 2)';
END $$;
ROLLBACK;
