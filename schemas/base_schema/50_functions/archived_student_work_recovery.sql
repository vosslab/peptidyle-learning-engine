-- Functions, triggers, and views from archived_student_work_recovery.sql.

SET LOCAL ROLE ple_private_owner;

CREATE FUNCTION ple_private.read_archived_assessment_attempt_evidence(
    p_course_instance_id text, p_assessment_attempt_id uuid
) RETURNS TABLE (
    student_record_id uuid, assessment_id text,
    assessment_attempt_id uuid, assessment_attempt_number integer,
    started_at timestamptz, expires_at timestamptz,
    attempt_facts jsonb, submission jsonb, questions jsonb
) LANGUAGE sql VOLATILE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
    SELECT work.student_record_id, assessment.assessment_id,
           work.assessment_attempt_id, work.assessment_attempt_number,
           work.started_at, work.expires_at,
           jsonb_build_object(
               'assessment_title', policy.assessment_title,
               'assessment_instructions', policy.assessment_instructions,
               'available_at', policy.available_at, 'due_at', policy.due_at,
               'closes_at', policy.closes_at,
               'assessment_attempt_time_limit_seconds', policy.assessment_attempt_time_limit_seconds,
               'assessment_attempt_limit', policy.assessment_attempt_limit,
               'late_work_rule', policy.late_work_rule,
               'question_variation_rule', policy.question_variation_rule,
               'assessment_question_order_rule', policy.assessment_question_order_rule,
               'feedback_score', policy.feedback_score,
               'feedback_per_item_correctness', policy.feedback_per_item_correctness,
               'feedback_submitted_response', policy.feedback_submitted_response,
               'feedback_question_answer', policy.feedback_question_answer,
               'feedback_question_answer_explanation', policy.feedback_question_answer_explanation,
               'feedback_class_statistics', policy.feedback_class_statistics
           ),
           CASE WHEN submitted.assessment_submission_id IS NOT NULL THEN
               jsonb_build_object('submitted_at', submitted.submitted_at,
                                  'finalization_kind',
                                  ple_private.projected_finalization_kind(
                                      submitted.authorized_by_account_id)) END,
           evidence.questions
      FROM ple_private.assessment_attempt AS work
      JOIN ple_data.assessment AS assessment ON assessment.assessment_id = work.assessment_id
      JOIN ple_data.assessment_policy_snapshot AS policy
        ON policy.assessment_policy_snapshot_id = work.assessment_policy_snapshot_id
      LEFT JOIN ple_private.assessment_submission AS submitted
        ON submitted.assessment_attempt_id = work.assessment_attempt_id
      CROSS JOIN LATERAL (
          SELECT COALESCE(jsonb_agg(jsonb_build_object(
              'delivery', jsonb_build_object(
                  'assessment_content_entry_index', issued.assessment_content_entry_index,
                  'issued_position', issued.issued_position,
                  'published_question_id', issued.published_question_id, 'revision_number', issued.revision_number,
                  'point_value', snapshot.points, 'scoring_rule', snapshot.scoring_rule,
                  'question_attempt_limit', snapshot.question_attempt_limit,
                  'question_attempt_time_limit_seconds', snapshot.question_attempt_time_limit_seconds,
                  'question_attempt_grace_seconds', snapshot.question_attempt_grace_seconds
              ),
              'pool', CASE WHEN pool.question_pool_selection_id IS NOT NULL THEN
                  jsonb_build_object('question_pool_id', pool.question_pool_id,
                      'question_pool_edit_number', pool.question_pool_edit_number,
                      'question_pool_member_position', selected.member_position,
                      'selection_position', selected.selection_position) END,
              'attempt', CASE WHEN attempt.question_attempt_id IS NOT NULL THEN
                  jsonb_build_object('issued_at', attempt.issued_at,
                      'deadline_at', attempt.deadline_at, 'finalized_at', attempt.finalized_at,
                      'question_attempt_state',
                      ple_private.projected_question_attempt_state(
                          attempt.finalized_at, saved.question_attempt_id IS NOT NULL)) END,
              'reproduction', CASE WHEN attempt.question_attempt_id IS NOT NULL THEN
                  jsonb_build_object('backend_name', toolchain.backend_name,
                      'backend_version', toolchain.backend_version,
                      'renderer_name', toolchain.renderer_name, 'renderer_version', toolchain.renderer_version,
                      'grader_name', toolchain.grader_name, 'grader_version', toolchain.grader_version,
                      'source_object_record_id', attempt.source_object_record_id,
                      'source_object_checksum', encode(attempt.source_object_checksum, 'hex'),
                      'question_seed', attempt.question_seed::text,
                      'generated_parameter_sha256', attempt.generated_parameter_sha256,
                      'rendered_question_sha256', encode(attempt.rendered_question_sha256, 'hex'),
                      'issued_capability', toolchain.issued_capability,
                      'webwork_pg_path', source.webwork_pg_path) END,
              'presentation', CASE WHEN presentation.question_attempt_id IS NOT NULL THEN
                  jsonb_build_object('descriptor_version', presentation.descriptor_version,
                      'presentation_nonce', presentation.presentation_nonce,
                      'presentation_checksum', encode(presentation.presentation_checksum, 'hex'),
                      'presentation', presentation.presentation,
                      'author_content', presentation.author_content,
                      'backend_document', presentation.backend_document,
                      'response_item_bindings', response_items.items,
                      'asset_renditions', assets.items) END,
              'saved_response', CASE WHEN saved.question_attempt_id IS NOT NULL THEN
                  jsonb_build_object('student_response', saved.student_response, 'saved_at', saved.saved_at) END,
              'finalized_response', CASE WHEN saved.finalized_at IS NOT NULL THEN
                  jsonb_build_object('student_response', saved.student_response,
                                    'finalized_at', saved.finalized_at) END,
              'grading', CASE WHEN result.grading_result_id IS NOT NULL THEN
                  jsonb_build_object('grading_state', 'graded',
                      'created_at', result.recorded_at, 'completed_at', result.recorded_at,
                      'normalized_credit', result.normalized_credit, 'recorded_at', result.recorded_at) END
          ) ORDER BY issued.issued_position), '[]'::jsonb) AS questions
            FROM ple_private.issued_question AS issued
            JOIN ple_private.assessment_entry_snapshot AS snapshot
              ON snapshot.assessment_entry_snapshot_id = issued.assessment_entry_snapshot_id
            LEFT JOIN ple_private.question_pool_selection AS pool
              ON pool.question_pool_selection_id = issued.question_pool_selection_id
             AND pool.assessment_attempt_id = issued.assessment_attempt_id
             AND pool.assessment_entry_id = issued.assessment_entry_id
            LEFT JOIN ple_private.question_pool_selected_item AS selected
              ON selected.question_pool_selection_id = pool.question_pool_selection_id
             AND selected.member_position = issued.question_pool_member_position
             AND selected.published_question_id = issued.published_question_id
             AND selected.revision_number = issued.revision_number
            LEFT JOIN ple_private.question_attempt AS attempt ON attempt.issued_question_id = issued.issued_question_id
            LEFT JOIN ple_private.delivery_toolchain AS toolchain
              ON toolchain.delivery_toolchain_id = attempt.delivery_toolchain_id
            LEFT JOIN ple_private.question_revision_source_binding AS source
              ON source.published_question_id = issued.published_question_id AND source.revision_number = issued.revision_number
            LEFT JOIN ple_private.question_attempt_presentation_binding AS presentation
              ON presentation.question_attempt_id = attempt.question_attempt_id
            LEFT JOIN ple_private.assessment_attempt_saved_response AS saved
              ON saved.question_attempt_id = attempt.question_attempt_id
            LEFT JOIN ple_private.grading_result AS result
              ON result.question_attempt_id = attempt.question_attempt_id
            CROSS JOIN LATERAL (
                SELECT COALESCE(jsonb_agg(jsonb_build_object(
                    'presentation_response_item_id', item.presentation_response_item_id,
                    'response_item_id', item.response_item_id
                ) ORDER BY item.presentation_response_item_id), '[]'::jsonb) AS items
                  FROM ple_private.question_attempt_response_item_binding AS item
                 WHERE item.question_attempt_presentation_binding_id
                       = attempt.question_attempt_id
            ) AS response_items
            CROSS JOIN LATERAL (
                SELECT COALESCE(jsonb_agg(jsonb_build_object(
                    'asset_id', asset.asset_id,
                    'question_asset_checksum', encode(asset.question_asset_checksum, 'hex'),
                    'rendition_checksum', encode(asset.rendition_checksum, 'hex'),
                    'intrinsic_width', asset.intrinsic_width, 'intrinsic_height', asset.intrinsic_height
                ) ORDER BY asset.asset_id), '[]'::jsonb) AS items
                  FROM ple_private.question_attempt_presentation_asset_rendition AS asset
                 WHERE asset.question_attempt_presentation_asset_binding_id
                       = attempt.question_attempt_id
            ) AS assets
           WHERE issued.assessment_attempt_id = work.assessment_attempt_id
      ) AS evidence
     WHERE work.assessment_attempt_id = p_assessment_attempt_id
       AND assessment.course_instance_id = p_course_instance_id
       -- ASVS 8.2.2/8.3.1: defense in depth at the private evidence seam.
       AND ple_api.current_session_account_is_instructor()
       AND ple_api.current_session_account_is_course_instructor(p_course_instance_id)
$$;



-- Minimized selection is private Work, never an ordinary archived history feed.
CREATE FUNCTION ple_private.select_archived_assessment_attempt_evidence(
    p_course_instance_id text, p_after_assessment_attempt_id uuid, p_limit integer
) RETURNS TABLE (
    student_record_id uuid, assessment_id text, assessment_title text,
    assessment_attempt_id uuid, assessment_attempt_number integer,
    started_at timestamptz, submitted_at timestamptz
) LANGUAGE sql VOLATILE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
    SELECT work.student_record_id, assessment.assessment_id, policy.assessment_title,
           work.assessment_attempt_id, work.assessment_attempt_number,
           work.started_at, submitted.submitted_at
      FROM ple_private.assessment_attempt AS work
      JOIN ple_data.assessment AS assessment ON assessment.assessment_id = work.assessment_id
      JOIN ple_data.assessment_policy_snapshot AS policy
        ON policy.assessment_policy_snapshot_id = work.assessment_policy_snapshot_id
      LEFT JOIN ple_private.assessment_submission AS submitted
        ON submitted.assessment_attempt_id = work.assessment_attempt_id
     WHERE assessment.course_instance_id = p_course_instance_id
       AND (p_after_assessment_attempt_id IS NULL OR work.assessment_attempt_id > p_after_assessment_attempt_id)
       AND p_limit BETWEEN 1 AND 101
       -- ASVS 8.2.2/8.3.1: trusted originating Instructor and exact Course.
       AND ple_api.current_session_account_is_instructor()
       AND ple_api.current_session_account_is_course_instructor(p_course_instance_id)
     ORDER BY work.assessment_attempt_id
     LIMIT p_limit
$$;

SET LOCAL ROLE ple_api_owner;





-- Both deliberate operations hold the same Course SHARE lock until transaction
-- end. This seam is not application-callable and conveys no caller authority.
CREATE FUNCTION ple_api.lock_archived_course_for_recovery(
    p_course_instance_id text
) RETURNS TABLE (
    course_instance_id text,
    student_data_archived_at timestamptz, delete_due_at timestamptz
) LANGUAGE plpgsql VOLATILE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
DECLARE course_row ple_data.course_instance%ROWTYPE;
DECLARE deletion_deadline timestamptz;
BEGIN
    -- ASVS 8.2.1: deny other Product Roles before acquiring a Course lock.
    IF NOT ple_api.current_session_account_is_instructor() THEN RETURN; END IF;
    -- ASVS 2.3.3/2.3.4: SHARE conflicts with retention's Course FOR UPDATE,
    -- including non-key lifecycle writes. Existing Course-media definer
    -- privileges support this lock; no new Course UPDATE grant/helper needed.
    SELECT course.* INTO course_row FROM ple_data.course_instance AS course
     WHERE course.course_instance_id = p_course_instance_id
       AND ple_api.current_session_account_is_course_instructor(course.course_instance_id)
     FOR SHARE;
    IF NOT FOUND THEN RETURN; END IF;
    SELECT course_row.retention_starts_at + schedule.archive_after_retention_start
               + schedule.delete_after_archive INTO deletion_deadline
      FROM ple_data.retention_schedule() AS schedule;
    -- ASVS 2.3.2/8.3.2: use current wall time AFTER any lock wait, not caller
    -- time/transaction start. Archive marking cannot extend absolute expiry.
    IF course_row.retention_lifecycle_state <> 'archived'
       OR course_row.student_data_archived_at IS NULL
       OR course_row.student_data_deleted_at IS NOT NULL
       OR deletion_deadline IS NULL
       OR pg_catalog.clock_timestamp() >= deletion_deadline
       OR NOT ple_api.current_session_account_is_instructor()
       OR NOT ple_api.current_session_account_is_course_instructor(course_row.course_instance_id)
       THEN RETURN; END IF;
    RETURN QUERY SELECT course_row.course_instance_id,
        course_row.student_data_archived_at, deletion_deadline;
END $$;

CREATE FUNCTION ple_api.read_archived_assessment_attempt_for_recovery(
    p_course_instance_id text, p_assessment_attempt_id uuid
) RETURNS TABLE (
    course_instance_id text, student_record_id uuid, roster_id text,
    assessment_id text,
    assessment_attempt_id uuid, assessment_attempt_number integer,
    started_at timestamptz, expires_at timestamptz,
    student_data_archived_at timestamptz, delete_due_at timestamptz,
    attempt_facts jsonb, submission jsonb, questions jsonb
) LANGUAGE plpgsql VOLATILE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
DECLARE recovery_course record;
BEGIN
    SELECT * INTO recovery_course
      FROM ple_api.lock_archived_course_for_recovery(p_course_instance_id);
    IF NOT FOUND THEN RETURN; END IF;
    RETURN QUERY SELECT recovery_course.course_instance_id,
        evidence.student_record_id, roster.roster_id,
        evidence.assessment_id, evidence.assessment_attempt_id,
        evidence.assessment_attempt_number, evidence.started_at, evidence.expires_at,
        recovery_course.student_data_archived_at, recovery_course.delete_due_at,
        evidence.attempt_facts, evidence.submission, evidence.questions
      FROM ple_private.read_archived_assessment_attempt_evidence(
          recovery_course.course_instance_id, p_assessment_attempt_id
      ) AS evidence
      JOIN ple_data.student_record AS student
        ON student.student_record_id = evidence.student_record_id
       AND student.course_instance_id = recovery_course.course_instance_id
      LEFT JOIN ple_private.course_roster_profile AS roster
        ON roster.course_instance_id = student.course_instance_id
       AND roster.student_account_id = student.student_account_id;
END $$;

CREATE FUNCTION ple_api.select_archived_assessment_attempts_for_recovery(
    p_course_instance_id text, p_after_assessment_attempt_id uuid, p_limit integer
) RETURNS TABLE (
    course_instance_id text, roster_id text, assessment_id text,
    assessment_title text, assessment_attempt_id uuid,
    assessment_attempt_number integer, started_at timestamptz, submitted_at timestamptz,
    student_data_archived_at timestamptz, delete_due_at timestamptz
) LANGUAGE plpgsql VOLATILE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
DECLARE recovery_course record;
BEGIN
    -- Bounded identity keyset pagination; callers can request page size + 1.
    -- ASVS 14.2.6: no Student Record/Account UUID or evidence document in selection.
    IF p_limit IS NULL OR p_limit NOT BETWEEN 1 AND 101
       THEN RETURN; END IF;
    SELECT * INTO recovery_course
      FROM ple_api.lock_archived_course_for_recovery(p_course_instance_id);
    -- An authorized empty page is distinct from an unavailable Course; the
    -- trusted adapter conceals this generic refusal without private detail.
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Retained Work unavailable';
    END IF;
    RETURN QUERY SELECT recovery_course.course_instance_id, roster.roster_id,
        evidence.assessment_id, evidence.assessment_title,
        evidence.assessment_attempt_id, evidence.assessment_attempt_number,
        evidence.started_at, evidence.submitted_at,
        recovery_course.student_data_archived_at, recovery_course.delete_due_at
      FROM ple_private.select_archived_assessment_attempt_evidence(
          recovery_course.course_instance_id, p_after_assessment_attempt_id, p_limit
      ) AS evidence
      JOIN ple_data.student_record AS student
        ON student.student_record_id = evidence.student_record_id
       AND student.course_instance_id = recovery_course.course_instance_id
      LEFT JOIN ple_private.course_roster_profile AS roster
        ON roster.course_instance_id = student.course_instance_id
       AND roster.student_account_id = student.student_account_id
     ORDER BY evidence.assessment_attempt_id;
END $$;

