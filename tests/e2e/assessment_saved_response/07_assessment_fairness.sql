-- A started Assessment keeps issued content stable.  Retiring a whole Pool
-- removes that Pool's current score from submitted, active, and later work.
BEGIN;
SET CONSTRAINTS ALL DEFERRED;

SELECT 'USFA0000' || ple_private.crockford_checksum_character('USFA0000') AS fairness_student_a_id \gset
SELECT 'USFB0000' || ple_private.crockford_checksum_character('USFB0000') AS fairness_student_b_id \gset
SELECT 'USFC0000' || ple_private.crockford_checksum_character('USFC0000') AS fairness_student_c_id \gset
SELECT 'ASVR0003' || ple_private.crockford_checksum_character('ASVR0003') AS fairness_assessment_id \gset
SELECT 'FARP-' || ple_private.crockford_checksum_character('FARPXYZ') || 'XYZ' AS fairness_source_pool_id \gset
SELECT 'FARF-' || ple_private.crockford_checksum_character('FARFXYZ') || 'XYZ' AS fairness_fork_pool_id \gset

SET LOCAL ROLE ple_private_owner;
INSERT INTO ple_private.account (account_id, user_role, created_at)
VALUES
    (:'fairness_student_a_id', 'student', pg_catalog.transaction_timestamp()),
    (:'fairness_student_b_id', 'student', pg_catalog.transaction_timestamp()),
    (:'fairness_student_c_id', 'student', pg_catalog.transaction_timestamp());

SET LOCAL ROLE ple_api_owner;
INSERT INTO ple_data.student_record (
    student_record_id, course_instance_id, student_account_id, created_at
) VALUES
    ('73000000-0000-0000-0000-000000000401', :'course_id', :'fairness_student_a_id', pg_catalog.transaction_timestamp()),
    ('73000000-0000-0000-0000-000000000402', :'course_id', :'fairness_student_b_id', pg_catalog.transaction_timestamp()),
    ('73000000-0000-0000-0000-000000000403', :'course_id', :'fairness_student_c_id', pg_catalog.transaction_timestamp());
INSERT INTO ple_data.course_membership (
    course_membership_id, course_instance_id, account_id, role, student_record_id, joined_at
) VALUES
    ('73000000-0000-0000-0000-000000000411', :'course_id', :'fairness_student_a_id', 'student', '73000000-0000-0000-0000-000000000401', pg_catalog.transaction_timestamp()),
    ('73000000-0000-0000-0000-000000000412', :'course_id', :'fairness_student_b_id', 'student', '73000000-0000-0000-0000-000000000402', pg_catalog.transaction_timestamp()),
    ('73000000-0000-0000-0000-000000000413', :'course_id', :'fairness_student_c_id', 'student', '73000000-0000-0000-0000-000000000403', pg_catalog.transaction_timestamp());

SET LOCAL ROLE ple_app;
SELECT set_config('ple.session_account_id', :'instructor_id', true);
SELECT question_pool_id AS fairness_source_pool_id, question_pool_edit_number AS fairness_source_pool_edit
  FROM ple_api.create_question_pool(
    :'fairness_source_pool_id', ARRAY[:'question_id', :'replacement_question_id'], ARRAY[1, 1], true,
    'Fairness Pool', 'Disposable Assessment fairness Pool', ARRAY[]::text[]
  ) \gset
SELECT assessment_id AS fairness_assessment_id, assessment_edit_number AS fairness_edit
  FROM ple_api.create_assessment(
    :'fairness_assessment_id', :'course_id', 'regular_assignment',
    'Fairness Assessment', 'Disposable Assessment fairness receipt'
  ) \gset

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
        'feedback_per_item_correctness', snapshot.feedback_per_item_correctness::text,
        'feedback_submitted_response', snapshot.feedback_submitted_response::text,
        'feedback_question_answer', snapshot.feedback_question_answer::text,
        'feedback_question_answer_explanation', snapshot.feedback_question_answer_explanation::text,
        'feedback_class_statistics', snapshot.feedback_class_statistics::text,
        'feedback_hints', snapshot.feedback_hints::text,
        'feedback_worked_solutions', snapshot.feedback_worked_solutions::text
    ), '{due_at}', to_jsonb(pg_catalog.clock_timestamp() + interval '25 hours')
)::text AS fairness_values
  FROM ple_data.assessment AS assessment
  JOIN ple_data.assessment_policy_snapshot AS snapshot
    ON snapshot.assessment_policy_snapshot_id = assessment.assessment_policy_snapshot_id
 WHERE assessment.assessment_id = :'fairness_assessment_id' \gset

SET LOCAL ROLE ple_app;
SELECT assessment_edit_number AS fairness_edit
  FROM ple_api.save_assessment(
    :'course_id', :'fairness_assessment_id', :'fairness_edit', :'fairness_values'::jsonb,
    jsonb_build_array(jsonb_build_object(
        'assessmentEntryId', '73000000-0000-0000-0000-000000000421',
        'kind', 'fixed_question', 'availability', 'available', 'scoringRule', 'normal',
        'authoredPosition', 0, 'questionId', :'question_id', 'revisionNumber', 1,
        'pointsPossible', 1
    ))
  ) \gset
SELECT assessment_edit_number AS fairness_edit
  FROM ple_api.import_assessment_question_pool_fork_for_ids(
    :'course_id', :'fairness_assessment_id', '73000000-0000-0000-0000-000000000422',
    :'fairness_edit', :'fairness_fork_pool_id', :'fairness_source_pool_id', :'fairness_source_pool_edit',
    1, 1, 1, 'question_pool_order', 'normal'
  ) \gset
SELECT assessment_status AS fairness_released, assessment_edit_number AS fairness_edit
  FROM ple_api.release_assessment(:'course_id', :'fairness_assessment_id', :'fairness_edit') \gset

-- Student A submits before retirement; Student B remains active with the
-- same issued Pool Question.  Their immutable issue records are identical in
-- kind, while their current score paths differ (history versus active access).
SELECT set_config('ple.session_account_id', :'fairness_student_a_id', true);
SELECT assessment_attempt_id AS fairness_attempt_a
  FROM ple_api.start_assessment_attempt(
    '73000000-0000-0000-0000-000000000431', '73000000-0000-0000-0000-000000000401', :'fairness_assessment_id',
    jsonb_build_array(jsonb_build_object(
        'question_pool_selection_id', '73000000-0000-0000-0000-000000000432',
        'assessment_entry_id', '73000000-0000-0000-0000-000000000422', 'selected_items', jsonb_build_array(1)
    )),
    jsonb_build_array(
        jsonb_build_object('issued_question_id', '73000000-0000-0000-0000-000000000433', 'assessment_entry_id', '73000000-0000-0000-0000-000000000421', 'issued_position', 0, 'published_question_id', :'question_id', 'revision_number', 1),
        jsonb_build_object('issued_question_id', '73000000-0000-0000-0000-000000000434', 'assessment_entry_id', '73000000-0000-0000-0000-000000000422', 'issued_position', 1, 'published_question_id', :'question_id', 'revision_number', 1, 'question_pool_selection_id', '73000000-0000-0000-0000-000000000432', 'question_pool_member_position', 1)
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
         WHEN '73000000-0000-0000-0000-000000000433'::uuid THEN '73000000-0000-0000-0000-000000000435'::uuid
         ELSE '73000000-0000-0000-0000-000000000436'::uuid
       END,
       issued.issued_question_id, pg_catalog.transaction_timestamp(),
       ple_private.ensure_delivery_toolchain('ple', '1', NULL, NULL, 'ple', '1', 'not_applicable'),
       decode(repeat('ab', 32), 'hex')
  FROM ple_private.issued_question AS issued
 WHERE issued.assessment_attempt_id = :'fairness_attempt_a'::uuid;

SET LOCAL ROLE ple_app;
SELECT set_config('ple.session_account_id', :'fairness_student_a_id', true);
SELECT response_state FROM ple_api.save_student_assessment_attempt_response(:'fairness_attempt_a'::uuid, 1, '{"choice":"fixed"}'::jsonb);
SELECT response_state FROM ple_api.save_student_assessment_attempt_response(:'fairness_attempt_a'::uuid, 2, '{"choice":"pool"}'::jsonb);
SELECT points_earned, points_possible FROM ple_api.commit_student_assessment_attempt_finalization(
    :'fairness_attempt_a'::uuid, 'student',
    (SELECT jsonb_agg(jsonb_build_object(
        'question_attempt_id', prepared.question_attempt_id, 'saved_at_millis', prepared.saved_at_millis,
        'student_response', prepared.student_response, 'normalized_credit', 1
    )) FROM ple_api.prepare_student_assessment_attempt_finalization(:'fairness_attempt_a'::uuid) AS prepared
       WHERE prepared.preparation_state = 'ready' AND prepared.question_attempt_id IS NOT NULL)
) \gset

SELECT set_config('ple.session_account_id', :'fairness_student_b_id', true);
SELECT assessment_attempt_id AS fairness_attempt_b
  FROM ple_api.start_assessment_attempt(
    '73000000-0000-0000-0000-000000000441', '73000000-0000-0000-0000-000000000402', :'fairness_assessment_id',
    jsonb_build_array(jsonb_build_object(
        'question_pool_selection_id', '73000000-0000-0000-0000-000000000442',
        'assessment_entry_id', '73000000-0000-0000-0000-000000000422', 'selected_items', jsonb_build_array(1)
    )),
    jsonb_build_array(
        jsonb_build_object('issued_question_id', '73000000-0000-0000-0000-000000000443', 'assessment_entry_id', '73000000-0000-0000-0000-000000000421', 'issued_position', 0, 'published_question_id', :'question_id', 'revision_number', 1),
        jsonb_build_object('issued_question_id', '73000000-0000-0000-0000-000000000444', 'assessment_entry_id', '73000000-0000-0000-0000-000000000422', 'issued_position', 1, 'published_question_id', :'question_id', 'revision_number', 1, 'question_pool_selection_id', '73000000-0000-0000-0000-000000000442', 'question_pool_member_position', 1)
    )
  ) \gset

-- The issued Assessment accepts a point/order update, but rejects a changed
-- Pool entry and removal of the issued Pool member.  It does retain removal of
-- the never-issued second member.
SELECT set_config('ple.session_account_id', :'instructor_id', true);
SELECT set_config('ple.course_id', :'course_id', true);
SELECT set_config('ple.question_id', :'question_id', true);
SELECT set_config('ple.replacement_question_id', :'replacement_question_id', true);
SELECT set_config('ple.fairness_assessment_id', :'fairness_assessment_id', true);
SELECT set_config('ple.fairness_source_pool_id', :'fairness_source_pool_id', true);
SELECT set_config('ple.fairness_fork_pool_id', :'fairness_fork_pool_id', true);
SELECT set_config('ple.fairness_values', :'fairness_values', true);
SELECT set_config('ple.fairness_edit', :'fairness_edit', true);
DO $$
DECLARE rejected_content boolean := false; rejected_issued_member boolean := false;
BEGIN
    BEGIN
        PERFORM * FROM ple_api.save_assessment(
            current_setting('ple.course_id'), current_setting('ple.fairness_assessment_id'), current_setting('ple.fairness_edit')::bigint,
            current_setting('ple.fairness_values')::jsonb,
            jsonb_build_array(
                jsonb_build_object('assessmentEntryId', '73000000-0000-0000-0000-000000000421', 'kind', 'fixed_question', 'availability', 'available', 'scoringRule', 'normal', 'authoredPosition', 0, 'questionId', current_setting('ple.question_id'), 'revisionNumber', 1, 'pointsPossible', 1),
                jsonb_build_object('assessmentEntryId', '73000000-0000-0000-0000-000000000422', 'kind', 'question_pool', 'availability', 'available', 'scoringRule', 'normal', 'authoredPosition', 1, 'selectionCount', 1, 'pointsPerItem', 1, 'selectedQuestionOrder', 'question_pool_order', 'questionPoolId', current_setting('ple.fairness_source_pool_id'))
            )
        );
    EXCEPTION WHEN object_not_in_prerequisite_state THEN rejected_content := true;
    END;
    BEGIN
        PERFORM * FROM ple_api.append_assessment_question_pool_fork_members(
            current_setting('ple.fairness_assessment_id'), '73000000-0000-0000-0000-000000000422',
            current_setting('ple.fairness_edit')::bigint, 1,
            ARRAY[current_setting('ple.replacement_question_id')], ARRAY[1], true
        );
    EXCEPTION WHEN object_not_in_prerequisite_state THEN rejected_issued_member := true;
    END;
    IF NOT rejected_content OR NOT rejected_issued_member THEN
        RAISE EXCEPTION 'started Assessment fairness guard did not reject changed content=% issued member=%', rejected_content, rejected_issued_member;
    END IF;
END $$;
SELECT assessment_edit_number AS fairness_edit
  FROM ple_api.save_assessment(
    :'course_id', :'fairness_assessment_id', :'fairness_edit', :'fairness_values'::jsonb,
    jsonb_build_array(
        jsonb_build_object('assessmentEntryId', '73000000-0000-0000-0000-000000000422', 'kind', 'question_pool', 'availability', 'available', 'scoringRule', 'normal', 'authoredPosition', 0, 'selectionCount', 1, 'pointsPerItem', 2, 'selectedQuestionOrder', 'question_pool_order', 'questionPoolId', :'fairness_fork_pool_id'),
        jsonb_build_object('assessmentEntryId', '73000000-0000-0000-0000-000000000421', 'kind', 'fixed_question', 'availability', 'available', 'scoringRule', 'normal', 'authoredPosition', 1, 'questionId', :'question_id', 'revisionNumber', 1, 'pointsPossible', 1)
    )
  ) \gset
SELECT question_pool_edit_number AS fairness_fork_edit, assessment_edit_number AS fairness_edit
  FROM ple_api.append_assessment_question_pool_fork_members(
    :'fairness_assessment_id', '73000000-0000-0000-0000-000000000422', :'fairness_edit', 1,
    ARRAY[:'question_id'], ARRAY[1], true
  ) \gset

-- Retiring the whole Pool leaves its Entry so issued work stays interpretable,
-- but the current resolver assigns it zero earned and possible points.
SELECT assessment_edit_number AS fairness_edit
  FROM ple_api.save_assessment(
    :'course_id', :'fairness_assessment_id', :'fairness_edit', :'fairness_values'::jsonb,
    jsonb_build_array(
        jsonb_build_object('assessmentEntryId', '73000000-0000-0000-0000-000000000422', 'kind', 'question_pool', 'availability', 'retired', 'scoringRule', 'normal', 'authoredPosition', 0, 'selectionCount', 1, 'pointsPerItem', 2, 'selectedQuestionOrder', 'question_pool_order', 'questionPoolId', :'fairness_fork_pool_id'),
        jsonb_build_object('assessmentEntryId', '73000000-0000-0000-0000-000000000421', 'kind', 'fixed_question', 'availability', 'available', 'scoringRule', 'normal', 'authoredPosition', 1, 'questionId', :'question_id', 'revisionNumber', 1, 'pointsPossible', 1)
    )
  ) \gset

-- Retirement is one-way after issue: an Instructor cannot reactivate the
-- retired Pool and expose it to later Students.
SELECT set_config('ple.session_account_id', :'instructor_id', true);
SELECT set_config('ple.fairness_edit', :'fairness_edit', true);
DO $$
BEGIN
    BEGIN
        PERFORM * FROM ple_api.save_assessment(
            current_setting('ple.course_id'), current_setting('ple.fairness_assessment_id'), current_setting('ple.fairness_edit')::bigint,
            current_setting('ple.fairness_values')::jsonb,
            jsonb_build_array(
                jsonb_build_object('assessmentEntryId', '73000000-0000-0000-0000-000000000422', 'kind', 'question_pool', 'availability', 'available', 'scoringRule', 'normal', 'authoredPosition', 0, 'selectionCount', 1, 'pointsPerItem', 2, 'selectedQuestionOrder', 'question_pool_order', 'questionPoolId', current_setting('ple.fairness_fork_pool_id')),
                jsonb_build_object('assessmentEntryId', '73000000-0000-0000-0000-000000000421', 'kind', 'fixed_question', 'availability', 'available', 'scoringRule', 'normal', 'authoredPosition', 1, 'questionId', current_setting('ple.question_id'), 'revisionNumber', 1, 'pointsPossible', 1)
            )
        );
        RAISE EXCEPTION 'retired Pool was reactivated after Assessment issue';
    EXCEPTION WHEN object_not_in_prerequisite_state THEN NULL;
    END;
END $$;

SELECT set_config('ple.session_account_id', :'fairness_student_a_id', true);
SELECT coalesce(sum((item ->> 'pointsEarned')::numeric), 0)::text AS fairness_a_earned,
       coalesce(sum((item ->> 'pointsPossible')::numeric), 0)::text AS fairness_a_possible
  FROM ple_api.read_student_assessment_attempt_history(:'fairness_attempt_a'::uuid) AS history,
       jsonb_array_elements(history.grading_results) AS item \gset
SELECT set_config('ple.session_account_id', :'fairness_student_b_id', true);
SELECT question_count::text AS fairness_b_count, points_possible::text AS fairness_b_possible
  FROM ple_api.read_student_assessment_access(:'course_id', :'fairness_assessment_id') \gset

-- Student C starts after retirement.  The Pool is no longer issuable, and the
-- later Attempt's access total matches the retired result seen by A and B.
SELECT set_config('ple.session_account_id', :'fairness_student_c_id', true);
SELECT assessment_attempt_id AS fairness_attempt_c
  FROM ple_api.start_assessment_attempt(
    '73000000-0000-0000-0000-000000000451', '73000000-0000-0000-0000-000000000403', :'fairness_assessment_id',
    '[]'::jsonb,
    jsonb_build_array(jsonb_build_object(
        'issued_question_id', '73000000-0000-0000-0000-000000000452', 'assessment_entry_id', '73000000-0000-0000-0000-000000000421',
        'issued_position', 0, 'published_question_id', :'question_id', 'revision_number', 1
    ))
  ) \gset
SELECT question_count::text AS fairness_c_count, points_possible::text AS fairness_c_possible
  FROM ple_api.read_student_assessment_access(:'course_id', :'fairness_assessment_id') \gset

SELECT set_config('ple.fairness_released', :'fairness_released', true);
SELECT set_config('ple.fairness_a_earned', :'fairness_a_earned', true);
SELECT set_config('ple.fairness_a_possible', :'fairness_a_possible', true);
SELECT set_config('ple.fairness_b_count', :'fairness_b_count', true);
SELECT set_config('ple.fairness_b_possible', :'fairness_b_possible', true);
SELECT set_config('ple.fairness_c_count', :'fairness_c_count', true);
SELECT set_config('ple.fairness_c_possible', :'fairness_c_possible', true);
DO $$
BEGIN
    IF current_setting('ple.fairness_released') <> 'released'
       OR current_setting('ple.fairness_a_earned')::numeric <> 1
       OR current_setting('ple.fairness_a_possible')::numeric <> 1
       OR current_setting('ple.fairness_b_count') <> '1'
       OR current_setting('ple.fairness_b_possible')::numeric <> 1
       OR current_setting('ple.fairness_c_count') <> '1'
       OR current_setting('ple.fairness_c_possible')::numeric <> 1 THEN
        RAISE EXCEPTION 'Assessment fairness receipt failed released=% A=%/% B=%/% C=%/%',
            current_setting('ple.fairness_released'), current_setting('ple.fairness_a_earned'), current_setting('ple.fairness_a_possible'),
            current_setting('ple.fairness_b_count'), current_setting('ple.fairness_b_possible'),
            current_setting('ple.fairness_c_count'), current_setting('ple.fairness_c_possible');
    END IF;
    RAISE NOTICE 'assessment_fairness_submitted_active_and_future_attempts_exclude_retired_pool';
END $$;
ROLLBACK;
