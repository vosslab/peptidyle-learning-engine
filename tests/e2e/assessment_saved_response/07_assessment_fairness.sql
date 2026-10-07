-- A started Assessment keeps issued content stable.  Retiring a whole Pool
-- removes that Pool's current score from submitted, active, and later work.
BEGIN;
SET CONSTRAINTS ALL DEFERRED;

SELECT 'USFA0000' || ple_private.crockford_checksum_character('USFA0000') AS fairness_student_a_id \gset
SELECT 'USFB0000' || ple_private.crockford_checksum_character('USFB0000') AS fairness_student_b_id \gset
SELECT 'USFC0000' || ple_private.crockford_checksum_character('USFC0000') AS fairness_student_c_id \gset
SELECT 'USFD0000' || ple_private.crockford_checksum_character('USFD0000') AS fairness_student_d_id \gset
SELECT 'ASFR0007' || ple_private.crockford_checksum_character('ASFR0007') AS fairness_assessment_id \gset
SELECT 'FARP-' || ple_private.crockford_checksum_character('FARPXYZ') || 'XYZ' AS fairness_source_pool_id \gset

SET LOCAL ROLE ple_private_owner;
INSERT INTO ple_private.account (account_id, user_role, created_at)
VALUES
    (:'fairness_student_a_id', 'student', pg_catalog.transaction_timestamp()),
    (:'fairness_student_b_id', 'student', pg_catalog.transaction_timestamp()),
    (:'fairness_student_c_id', 'student', pg_catalog.transaction_timestamp()),
    (:'fairness_student_d_id', 'student', pg_catalog.transaction_timestamp());

SET LOCAL ROLE ple_api_owner;
INSERT INTO ple_data.student_record (
    student_record_id, course_instance_id, student_account_id, created_at
) VALUES
    ('73000000-0000-0000-0000-000000000401', :'course_id', :'fairness_student_a_id', pg_catalog.transaction_timestamp()),
    ('73000000-0000-0000-0000-000000000402', :'course_id', :'fairness_student_b_id', pg_catalog.transaction_timestamp()),
    ('73000000-0000-0000-0000-000000000403', :'course_id', :'fairness_student_c_id', pg_catalog.transaction_timestamp()),
    ('73000000-0000-0000-0000-000000000404', :'course_id', :'fairness_student_d_id', pg_catalog.transaction_timestamp());
INSERT INTO ple_data.course_membership (
    course_membership_id, course_instance_id, account_id, role, student_record_id, joined_at
) VALUES
    ('73000000-0000-0000-0000-000000000411', :'course_id', :'fairness_student_a_id', 'student', '73000000-0000-0000-0000-000000000401', pg_catalog.transaction_timestamp()),
    ('73000000-0000-0000-0000-000000000412', :'course_id', :'fairness_student_b_id', 'student', '73000000-0000-0000-0000-000000000402', pg_catalog.transaction_timestamp()),
    ('73000000-0000-0000-0000-000000000413', :'course_id', :'fairness_student_c_id', 'student', '73000000-0000-0000-0000-000000000403', pg_catalog.transaction_timestamp()),
    ('73000000-0000-0000-0000-000000000414', :'course_id', :'fairness_student_d_id', 'student', '73000000-0000-0000-0000-000000000404', pg_catalog.transaction_timestamp());

SET LOCAL ROLE ple_app;
SELECT set_config('ple.session_account_id', :'instructor_id', true);
SELECT question_pool_id AS fairness_source_pool_id, question_pool_edit_number AS fairness_source_pool_edit
  FROM ple_api.create_question_pool(
    :'fairness_source_pool_id', ARRAY[:'question_id', :'replacement_question_id'], ARRAY[1, 1],
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
        'partial_credit_enabled', snapshot.partial_credit_enabled,
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
  FROM ple_api.save_assessment(
    :'course_id', :'fairness_assessment_id', :'fairness_edit', :'fairness_values'::jsonb,
    jsonb_build_array(
        jsonb_build_object(
            'assessmentEntryId', '73000000-0000-0000-0000-000000000421',
            'kind', 'fixed_question', 'availability', 'available', 'scoringRule', 'normal',
            'authoredPosition', 0, 'questionId', :'question_id', 'revisionNumber', 1,
            'pointsPossible', 1
        ),
        jsonb_build_object(
            'assessmentEntryId', '73000000-0000-0000-0000-000000000422',
            'kind', 'question_pool', 'availability', 'available', 'scoringRule', 'normal',
            'authoredPosition', 1, 'selectionCount', 1, 'pointsPerItem', 1,
            'questionPoolId', :'fairness_source_pool_id'
        )
    )
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
        'assessment_entry_id', '73000000-0000-0000-0000-000000000422',
        'question_pool_id', :'fairness_source_pool_id',
        'question_pool_edit_number', :'fairness_source_pool_edit'::bigint,
        'selected_items', jsonb_build_array(jsonb_build_object(
            'published_question_id', :'question_id', 'revision_number', 1
        ))
    )),
    jsonb_build_array(
        jsonb_build_object('issued_question_id', '73000000-0000-0000-0000-000000000433', 'assessment_entry_id', '73000000-0000-0000-0000-000000000421', 'issued_position', 0, 'published_question_id', :'question_id', 'revision_number', 1),
        jsonb_build_object('issued_question_id', '73000000-0000-0000-0000-000000000434', 'assessment_entry_id', '73000000-0000-0000-0000-000000000422', 'issued_position', 1, 'published_question_id', :'question_id', 'revision_number', 1, 'question_pool_selection_id', '73000000-0000-0000-0000-000000000432')
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
        'assessment_entry_id', '73000000-0000-0000-0000-000000000422',
        'question_pool_id', :'fairness_source_pool_id',
        'question_pool_edit_number', :'fairness_source_pool_edit'::bigint,
        'selected_items', jsonb_build_array(jsonb_build_object(
            'published_question_id', :'question_id', 'revision_number', 1
        ))
    )),
    jsonb_build_array(
        jsonb_build_object('issued_question_id', '73000000-0000-0000-0000-000000000443', 'assessment_entry_id', '73000000-0000-0000-0000-000000000421', 'issued_position', 0, 'published_question_id', :'question_id', 'revision_number', 1),
        jsonb_build_object('issued_question_id', '73000000-0000-0000-0000-000000000444', 'assessment_entry_id', '73000000-0000-0000-0000-000000000422', 'issued_position', 1, 'published_question_id', :'question_id', 'revision_number', 1, 'question_pool_selection_id', '73000000-0000-0000-0000-000000000442')
    )
  ) \gset

-- The issued Assessment rejects changing its own requested count. The shared
-- Pool rejects removing an issued Question while allowing an unissued member
-- to be removed for future selections.
SELECT set_config('ple.session_account_id', :'instructor_id', true);
SELECT set_config('ple.course_id', :'course_id', true);
SELECT set_config('ple.question_id', :'question_id', true);
SELECT set_config('ple.replacement_question_id', :'replacement_question_id', true);
SELECT set_config('ple.fairness_assessment_id', :'fairness_assessment_id', true);
SELECT set_config('ple.fairness_source_pool_id', :'fairness_source_pool_id', true);
SELECT set_config('ple.fairness_source_pool_edit', :'fairness_source_pool_edit', true);
SELECT set_config('ple.fairness_values', :'fairness_values', true);
SELECT set_config('ple.fairness_edit', :'fairness_edit', true);
DO $$
DECLARE rejected_content boolean := false;
BEGIN
    BEGIN
        PERFORM * FROM ple_api.save_assessment(
            current_setting('ple.course_id'), current_setting('ple.fairness_assessment_id'),
            current_setting('ple.fairness_edit')::bigint,
            current_setting('ple.fairness_values')::jsonb,
            jsonb_build_array(
                jsonb_build_object('assessmentEntryId', '73000000-0000-0000-0000-000000000421', 'kind', 'fixed_question', 'availability', 'available', 'scoringRule', 'normal', 'authoredPosition', 0, 'questionId', current_setting('ple.question_id'), 'revisionNumber', 1, 'pointsPossible', 1),
                jsonb_build_object('assessmentEntryId', '73000000-0000-0000-0000-000000000422', 'kind', 'question_pool', 'availability', 'available', 'scoringRule', 'normal', 'authoredPosition', 1, 'selectionCount', 2, 'pointsPerItem', 1, 'questionPoolId', current_setting('ple.fairness_source_pool_id'))
            )
        );
    EXCEPTION WHEN object_not_in_prerequisite_state THEN rejected_content := true;
    END;
    IF NOT rejected_content THEN
        RAISE EXCEPTION 'started Assessment accepted a changed Pool selection count';
    END IF;
END $$;
RESET ROLE;
SET LOCAL ROLE ple_api_owner;
DO $$
DECLARE rejected_issued_question_removal boolean := false;
BEGIN
    BEGIN
        PERFORM * FROM ple_data.save_question_pool_members(
            current_setting('ple.fairness_source_pool_id'),
            current_setting('ple.fairness_source_pool_edit')::bigint,
            ARRAY[current_setting('ple.replacement_question_id')], ARRAY[1]
        );
    EXCEPTION WHEN check_violation THEN
        rejected_issued_question_removal := true;
    END;
    IF NOT rejected_issued_question_removal THEN
        RAISE EXCEPTION 'shared Pool accepted removal of a Question already issued by an available Assessment';
    END IF;
    IF (SELECT question_pool_edit_number
          FROM ple_data.question_pool
         WHERE question_pool_id = current_setting('ple.fairness_source_pool_id'))
           IS DISTINCT FROM current_setting('ple.fairness_source_pool_edit')::bigint
       OR (SELECT count(*)
             FROM ple_data.question_pool_member AS member
            WHERE member.question_pool_id = current_setting('ple.fairness_source_pool_id'))
           IS DISTINCT FROM 2
       OR NOT EXISTS (
            SELECT 1 FROM ple_data.question_pool_member AS member
             WHERE member.question_pool_id = current_setting('ple.fairness_source_pool_id')
               AND member.published_question_id = current_setting('ple.question_id')
               AND member.question_revision_number = 1
       )
       OR NOT EXISTS (
            SELECT 1 FROM ple_data.question_pool_member AS member
             WHERE member.question_pool_id = current_setting('ple.fairness_source_pool_id')
               AND member.published_question_id = current_setting('ple.replacement_question_id')
               AND member.question_revision_number = 1
       ) THEN
        RAISE EXCEPTION 'rejected Pool removal changed the shared Pool members or Edit Number';
    END IF;
END $$;

-- Removing the unissued member is allowed.  The updated Pool keeps the issued
-- Question available for future selection under its new Edit Number.
SELECT question_pool_edit_number AS fairness_pool_edit
  FROM ple_data.save_question_pool_members(
      current_setting('ple.fairness_source_pool_id'),
      current_setting('ple.fairness_source_pool_edit')::bigint,
      ARRAY[current_setting('ple.question_id')], ARRAY[1]
  ) \gset
SELECT set_config('ple.fairness_pool_edit', :'fairness_pool_edit', true);
DO $$
BEGIN
    IF (SELECT question_pool_edit_number
          FROM ple_data.question_pool
         WHERE question_pool_id = current_setting('ple.fairness_source_pool_id'))
           IS DISTINCT FROM current_setting('ple.fairness_pool_edit')::bigint
       OR current_setting('ple.fairness_pool_edit')::bigint
           IS DISTINCT FROM current_setting('ple.fairness_source_pool_edit')::bigint + 1
       OR (SELECT count(*)
             FROM ple_data.question_pool_member AS member
            WHERE member.question_pool_id = current_setting('ple.fairness_source_pool_id'))
           IS DISTINCT FROM 1
       OR NOT EXISTS (
            SELECT 1 FROM ple_data.question_pool_member AS member
             WHERE member.question_pool_id = current_setting('ple.fairness_source_pool_id')
               AND member.published_question_id = current_setting('ple.question_id')
               AND member.question_revision_number = 1
       ) THEN
        RAISE EXCEPTION 'legal removal did not save exactly the unissued Pool member at the next Edit Number';
    END IF;
END $$;
RESET ROLE;
SET LOCAL ROLE ple_private_owner;
DO $$
DECLARE expected record;
BEGIN
    FOR expected IN
        SELECT * FROM (VALUES
            ('73000000-0000-0000-0000-000000000432'::uuid,
             '73000000-0000-0000-0000-000000000431'::uuid,
             '73000000-0000-0000-0000-000000000434'::uuid),
            ('73000000-0000-0000-0000-000000000442'::uuid,
             '73000000-0000-0000-0000-000000000441'::uuid,
             '73000000-0000-0000-0000-000000000444'::uuid)
        ) AS prior(question_pool_selection_id, assessment_attempt_id, issued_question_id)
    LOOP
        IF NOT EXISTS (
            SELECT 1
              FROM ple_private.question_pool_selection AS selection
             WHERE selection.question_pool_selection_id = expected.question_pool_selection_id
               AND selection.assessment_attempt_id = expected.assessment_attempt_id
               AND selection.question_pool_id = current_setting('ple.fairness_source_pool_id')
               AND selection.question_pool_edit_number
                   = current_setting('ple.fairness_source_pool_edit')::bigint
        )
           OR (SELECT count(*)
                 FROM ple_private.question_pool_selected_item AS selected
                WHERE selected.question_pool_selection_id = expected.question_pool_selection_id)
              IS DISTINCT FROM 1
           OR NOT EXISTS (
                SELECT 1
                  FROM ple_private.question_pool_selected_item AS selected
                 WHERE selected.question_pool_selection_id = expected.question_pool_selection_id
                   AND selected.published_question_id = current_setting('ple.question_id')
                   AND selected.revision_number = 1
           )
           OR (SELECT count(*)
                 FROM ple_private.issued_question AS issued
                WHERE issued.question_pool_selection_id = expected.question_pool_selection_id)
              IS DISTINCT FROM 1
           OR NOT EXISTS (
                SELECT 1
                  FROM ple_private.issued_question AS issued
                 WHERE issued.question_pool_selection_id = expected.question_pool_selection_id
                   AND issued.issued_question_id = expected.issued_question_id
                   AND issued.published_question_id = current_setting('ple.question_id')
                   AND issued.revision_number = 1
           ) THEN
            RAISE EXCEPTION 'Pool edit changed the exact Question tuple selected by existing Attempt %',
                expected.assessment_attempt_id;
        END IF;
    END LOOP;
END $$;
RESET ROLE;
SET LOCAL ROLE ple_app;
SELECT set_config('ple.fairness_pool_edit', :'fairness_pool_edit', true);

-- A later Attempt sees the edited Pool while its Assessment Entry is still
-- available, and records the new Edit Number and its sole exact member tuple.
SELECT set_config('ple.session_account_id', :'fairness_student_d_id', true);
SELECT assessment_attempt_id AS fairness_attempt_d
  FROM ple_api.start_assessment_attempt(
    '73000000-0000-0000-0000-000000000461', '73000000-0000-0000-0000-000000000404', :'fairness_assessment_id',
    jsonb_build_array(jsonb_build_object(
        'question_pool_selection_id', '73000000-0000-0000-0000-000000000462',
        'assessment_entry_id', '73000000-0000-0000-0000-000000000422',
        'question_pool_id', :'fairness_source_pool_id',
        'question_pool_edit_number', :'fairness_pool_edit'::bigint,
        'selected_items', jsonb_build_array(jsonb_build_object(
            'published_question_id', :'question_id', 'revision_number', 1
        ))
    )),
    jsonb_build_array(
        jsonb_build_object(
            'issued_question_id', '73000000-0000-0000-0000-000000000463',
            'assessment_entry_id', '73000000-0000-0000-0000-000000000421',
            'issued_position', 0, 'published_question_id', :'question_id', 'revision_number', 1
        ),
        jsonb_build_object(
            'issued_question_id', '73000000-0000-0000-0000-000000000464',
            'assessment_entry_id', '73000000-0000-0000-0000-000000000422',
            'issued_position', 1, 'published_question_id', :'question_id',
            'revision_number', 1, 'question_pool_selection_id', '73000000-0000-0000-0000-000000000462'
        )
    )
  ) \gset
RESET ROLE;
SET LOCAL ROLE ple_private_owner;
DO $$
BEGIN
    IF (SELECT count(*)
          FROM ple_private.question_pool_selection AS selection
         WHERE selection.question_pool_selection_id = '73000000-0000-0000-0000-000000000462')
           IS DISTINCT FROM 1
       OR (SELECT count(*)
             FROM ple_private.question_pool_selected_item AS selected
            WHERE selected.question_pool_selection_id = '73000000-0000-0000-0000-000000000462')
           IS DISTINCT FROM 1
       OR (SELECT count(*)
             FROM ple_private.issued_question AS issued
            WHERE issued.question_pool_selection_id = '73000000-0000-0000-0000-000000000462')
           IS DISTINCT FROM 1
       OR NOT EXISTS (
        SELECT 1
          FROM ple_private.question_pool_selection AS selection
          JOIN ple_private.question_pool_selected_item AS selected USING (question_pool_selection_id)
          JOIN ple_private.issued_question AS issued USING (question_pool_selection_id)
         WHERE selection.question_pool_selection_id = '73000000-0000-0000-0000-000000000462'
           AND selection.assessment_attempt_id = '73000000-0000-0000-0000-000000000461'
           AND selection.question_pool_id = current_setting('ple.fairness_source_pool_id')
           AND selection.question_pool_edit_number = current_setting('ple.fairness_pool_edit')::bigint
           AND selected.published_question_id = current_setting('ple.question_id')
           AND selected.revision_number = 1
           AND issued.issued_question_id = '73000000-0000-0000-0000-000000000464'
           AND issued.published_question_id = current_setting('ple.question_id')
           AND issued.revision_number = 1
    ) THEN
        RAISE EXCEPTION 'later Attempt did not retain the current Pool Edit Number and exact selected tuple';
    END IF;
END $$;
RESET ROLE;
SET LOCAL ROLE ple_app;
SELECT set_config('ple.session_account_id', :'instructor_id', true);

SELECT assessment_edit_number AS fairness_edit
  FROM ple_api.save_assessment(
    :'course_id', :'fairness_assessment_id', :'fairness_edit', :'fairness_values'::jsonb,
    jsonb_build_array(
        jsonb_build_object('assessmentEntryId', '73000000-0000-0000-0000-000000000422', 'kind', 'question_pool', 'availability', 'available', 'scoringRule', 'normal', 'authoredPosition', 0, 'selectionCount', 1, 'pointsPerItem', 2, 'questionPoolId', :'fairness_source_pool_id'),
        jsonb_build_object('assessmentEntryId', '73000000-0000-0000-0000-000000000421', 'kind', 'fixed_question', 'availability', 'available', 'scoringRule', 'normal', 'authoredPosition', 1, 'questionId', :'question_id', 'revisionNumber', 1, 'pointsPossible', 1)
    )
  ) \gset
SELECT set_config('ple.fairness_pool_edit', :'fairness_pool_edit', true);
RESET ROLE;
SET LOCAL ROLE ple_api_owner;
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM ple_data.question_pool
         WHERE question_pool_id = current_setting('ple.fairness_source_pool_id')
           AND question_pool_edit_number = current_setting('ple.fairness_pool_edit')::bigint
           AND license = 'CC0-1.0'
    ) THEN
        RAISE EXCEPTION 'shared Pool member removal did not preserve the calculated license';
    END IF;
END $$;
RESET ROLE;
SET LOCAL ROLE ple_app;

-- Retiring the whole Pool leaves its Entry so issued work stays interpretable,
-- but the current resolver assigns it zero earned and possible points.
SELECT assessment_edit_number AS fairness_edit
  FROM ple_api.save_assessment(
    :'course_id', :'fairness_assessment_id', :'fairness_edit', :'fairness_values'::jsonb,
    jsonb_build_array(
        jsonb_build_object('assessmentEntryId', '73000000-0000-0000-0000-000000000422', 'kind', 'question_pool', 'availability', 'retired', 'scoringRule', 'normal', 'authoredPosition', 0, 'selectionCount', 1, 'pointsPerItem', 2, 'questionPoolId', :'fairness_source_pool_id'),
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
                jsonb_build_object('assessmentEntryId', '73000000-0000-0000-0000-000000000422', 'kind', 'question_pool', 'availability', 'available', 'scoringRule', 'normal', 'authoredPosition', 0, 'selectionCount', 1, 'pointsPerItem', 2, 'questionPoolId', current_setting('ple.fairness_source_pool_id')),
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
    IF current_setting('ple.fairness_released') IS DISTINCT FROM 'released'
       OR current_setting('ple.fairness_a_earned')::numeric IS DISTINCT FROM 1
       OR current_setting('ple.fairness_a_possible')::numeric IS DISTINCT FROM 1
       OR current_setting('ple.fairness_b_count') IS DISTINCT FROM '1'
       OR current_setting('ple.fairness_b_possible')::numeric IS DISTINCT FROM 1
       OR current_setting('ple.fairness_c_count') IS DISTINCT FROM '1'
       OR current_setting('ple.fairness_c_possible')::numeric IS DISTINCT FROM 1 THEN
        RAISE EXCEPTION 'Assessment fairness receipt failed released=% A=%/% B=%/% C=%/%',
            current_setting('ple.fairness_released'), current_setting('ple.fairness_a_earned'), current_setting('ple.fairness_a_possible'),
            current_setting('ple.fairness_b_count'), current_setting('ple.fairness_b_possible'),
            current_setting('ple.fairness_c_count'), current_setting('ple.fairness_c_possible');
    END IF;
    RAISE NOTICE 'assessment_fairness_submitted_active_and_future_attempts_exclude_retired_pool';
END $$;
ROLLBACK;
