-- Protected evidence recovery, not an ordinary history read or restoration.
-- The retained backend document is data for the trusted server, not an HTML
-- response. No object addresses, credentials, Profiles, or receipt blobs cross
-- this boundary. Existing Student/history/Instructor readers remain unchanged.

SET LOCAL ROLE ple_data_owner;
GRANT SELECT (archive_after_retention_start, delete_after_archive, policy_key)
    ON ple_data.course_retention_policy TO ple_api_owner;
CREATE POLICY course_retention_policy_api_owner_recovery_read
    ON ple_data.course_retention_policy FOR SELECT TO ple_api_owner USING (true);
RESET ROLE;

SET LOCAL ROLE ple_private_owner;
CREATE FUNCTION ple_private.read_archived_assessment_attempt_evidence(
    p_course_id uuid, p_assessment_attempt_reference_number bigint
) RETURNS TABLE (
    student_record_id uuid, assessment_reference_number text,
    assessment_attempt_reference_number bigint, assessment_attempt_number integer,
    started_at timestamptz, expires_at timestamptz,
    attempt_facts jsonb, submission jsonb, questions jsonb
) LANGUAGE sql VOLATILE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
    SELECT work.student_record_id, assessment.public_reference,
           work.reference_number, work.assessment_attempt_number,
           work.started_at, work.expires_at,
           jsonb_build_object(
               'assessment_title', work.assessment_title,
               'assessment_instructions', work.assessment_instructions,
               'available_at', work.available_at, 'due_at', work.due_at,
               'closes_at', work.closes_at,
               'assessment_attempt_time_limit_seconds', work.assessment_attempt_time_limit_seconds,
               'assessment_attempt_limit', work.assessment_attempt_limit,
               'late_work_rule', work.late_work_rule,
               'question_variation_rule', work.question_variation_rule,
               'assessment_question_order_rule', work.assessment_question_order_rule,
               'feedback_score', work.feedback_score,
               'feedback_per_item_correctness', work.feedback_per_item_correctness,
               'feedback_submitted_response', work.feedback_submitted_response,
               'feedback_question_answer', work.feedback_question_answer,
               'feedback_question_answer_explanation', work.feedback_question_answer_explanation,
               'feedback_class_statistics', work.feedback_class_statistics
           ),
           CASE WHEN submitted.assessment_submission_id IS NOT NULL THEN
               jsonb_build_object('submitted_at', submitted.submitted_at,
                                  'finalization_kind', submitted.finalization_kind) END,
           evidence.questions
      FROM ple_private.assessment_attempt AS work
      JOIN ple_data.assessment AS assessment ON assessment.assessment_id = work.assessment_id
      LEFT JOIN ple_private.assessment_submission AS submitted
        ON submitted.assessment_attempt_id = work.assessment_attempt_id
      CROSS JOIN LATERAL (
          SELECT COALESCE(jsonb_agg(jsonb_build_object(
              'delivery', jsonb_build_object(
                  'assessment_content_entry_index', issued.assessment_content_entry_index,
                  'issued_position', issued.issued_position,
                  'question_id', issued.question_id, 'revision_number', issued.revision_number,
                  'point_value', issued.point_value, 'scoring_rule', issued.scoring_rule,
                  'question_attempt_limit', issued.question_attempt_limit,
                  'question_attempt_time_limit_seconds', issued.question_attempt_time_limit_seconds,
                  'question_attempt_grace_seconds', issued.question_attempt_grace_seconds
              ),
              'pool', CASE WHEN pool.question_pool_selection_id IS NOT NULL THEN
                  jsonb_build_object('question_pool_id', pool.question_pool_id,
                      'question_pool_revision_number', pool.question_pool_revision_number,
                      'question_pool_member_position', selected.member_position,
                      'selection_position', selected.selection_position,
                      'selected_question_count', pool.selected_question_count) END,
              'attempt', CASE WHEN attempt.question_attempt_id IS NOT NULL THEN
                  jsonb_build_object('issued_at', attempt.issued_at,
                      'deadline_at', attempt.deadline_at, 'finalized_at', attempt.finalized_at,
                      'question_attempt_state', attempt.question_attempt_state) END,
              'reproduction', CASE WHEN attempt.question_attempt_id IS NOT NULL THEN
                  jsonb_build_object('backend_name', attempt.backend_name,
                      'backend_version', attempt.backend_version,
                      'renderer_name', attempt.renderer_name, 'renderer_version', attempt.renderer_version,
                      'grader_name', attempt.grader_name, 'grader_version', attempt.grader_version,
                      'source_object_id', attempt.source_object_id,
                      'source_object_checksum', encode(attempt.source_object_checksum, 'hex'),
                      'question_seed', attempt.question_seed::text,
                      'generated_parameter_sha256', attempt.generated_parameter_sha256,
                      'rendered_question_sha256', encode(attempt.rendered_question_sha256, 'hex'),
                      'issued_capability', attempt.issued_capability,
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
              'finalized_response', CASE WHEN response.question_response_id IS NOT NULL THEN
                  jsonb_build_object('student_response', response.student_response,
                                    'finalized_at', response.finalized_at) END,
              'grading', CASE WHEN grading.question_response_grading_id IS NOT NULL THEN
                  jsonb_build_object('grading_state', grading.grading_state,
                      'created_at', grading.created_at, 'completed_at', grading.completed_at,
                      'normalized_credit', result.normalized_credit, 'recorded_at', result.recorded_at) END
          ) ORDER BY issued.issued_position), '[]'::jsonb) AS questions
            FROM ple_private.issued_question AS issued
            LEFT JOIN ple_private.question_pool_selection AS pool
              ON pool.question_pool_selection_id = issued.question_pool_selection_id
             AND pool.assessment_attempt_id = issued.assessment_attempt_id
             AND pool.assessment_entry_id = issued.assessment_entry_id
            LEFT JOIN ple_private.question_pool_selected_item AS selected
              ON selected.question_pool_selection_id = pool.question_pool_selection_id
             AND selected.member_position = issued.question_pool_member_position
             AND selected.question_id = issued.question_id
             AND selected.revision_number = issued.revision_number
            LEFT JOIN ple_private.question_attempt AS attempt ON attempt.issued_question_id = issued.issued_question_id
            LEFT JOIN ple_private.question_revision_source_binding AS source
              ON source.question_id = issued.question_id AND source.revision_number = issued.revision_number
            LEFT JOIN ple_private.question_attempt_presentation_binding AS presentation
              ON presentation.question_attempt_id = attempt.question_attempt_id
            LEFT JOIN ple_private.assessment_attempt_saved_response AS saved
              ON saved.question_attempt_id = attempt.question_attempt_id
            LEFT JOIN ple_private.question_response AS response
              ON response.question_attempt_id = attempt.question_attempt_id
             AND response.assessment_submission_id = submitted.assessment_submission_id
            LEFT JOIN ple_private.question_response_grading AS grading
              ON grading.question_response_id = response.question_response_id
            LEFT JOIN ple_private.grading_result AS result
              ON result.question_response_grading_id = grading.question_response_grading_id
             AND result.question_response_id = response.question_response_id
             AND result.question_attempt_id = attempt.question_attempt_id
            CROSS JOIN LATERAL (
                SELECT COALESCE(jsonb_agg(jsonb_build_object(
                    'presentation_response_item_reference', item.presentation_response_item_reference,
                    'response_item_reference', item.response_item_reference
                ) ORDER BY item.presentation_response_item_reference), '[]'::jsonb) AS items
                  FROM ple_private.question_attempt_response_item_binding AS item
                 WHERE item.question_attempt_id = attempt.question_attempt_id
            ) AS response_items
            CROSS JOIN LATERAL (
                SELECT COALESCE(jsonb_agg(jsonb_build_object(
                    'asset_id', asset.asset_id,
                    'question_asset_checksum', encode(asset.question_asset_checksum, 'hex'),
                    'rendition_checksum', encode(asset.rendition_checksum, 'hex'),
                    'intrinsic_width', asset.intrinsic_width, 'intrinsic_height', asset.intrinsic_height
                ) ORDER BY asset.asset_id), '[]'::jsonb) AS items
                  FROM ple_private.question_attempt_presentation_asset_rendition AS asset
                 WHERE asset.question_attempt_id = attempt.question_attempt_id
            ) AS assets
           WHERE issued.assessment_attempt_id = work.assessment_attempt_id
      ) AS evidence
     WHERE work.reference_number = p_assessment_attempt_reference_number
       AND assessment.course_id = p_course_id
       -- ASVS 8.2.2/8.3.1: defense in depth at the private evidence seam.
       AND ple_api.current_session_account_is_instructor()
       AND ple_api.current_session_account_is_course_instructor(p_course_id)
$$;
REVOKE ALL ON FUNCTION ple_private.read_archived_assessment_attempt_evidence(uuid, bigint) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_private.read_archived_assessment_attempt_evidence(uuid, bigint) TO ple_api_owner;

-- Minimized selection is private Work, never an ordinary archived history feed.
CREATE FUNCTION ple_private.select_archived_assessment_attempt_evidence(
    p_course_id uuid, p_after_reference_number bigint, p_limit integer
) RETURNS TABLE (
    student_record_id uuid, assessment_reference_number text, assessment_title text,
    assessment_attempt_reference_number bigint, assessment_attempt_number integer,
    started_at timestamptz, submitted_at timestamptz
) LANGUAGE sql VOLATILE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
    SELECT work.student_record_id, assessment.public_reference, work.assessment_title,
           work.reference_number, work.assessment_attempt_number,
           work.started_at, submitted.submitted_at
      FROM ple_private.assessment_attempt AS work
      JOIN ple_data.assessment AS assessment ON assessment.assessment_id = work.assessment_id
      LEFT JOIN ple_private.assessment_submission AS submitted
        ON submitted.assessment_attempt_id = work.assessment_attempt_id
     WHERE assessment.course_id = p_course_id
       AND (p_after_reference_number IS NULL OR work.reference_number > p_after_reference_number)
       AND p_limit BETWEEN 1 AND 101
       -- ASVS 8.2.2/8.3.1: trusted originating Instructor and exact Course.
       AND ple_api.current_session_account_is_instructor()
       AND ple_api.current_session_account_is_course_instructor(p_course_id)
     ORDER BY work.reference_number
     LIMIT p_limit
$$;
REVOKE ALL ON FUNCTION ple_private.select_archived_assessment_attempt_evidence(uuid, bigint, integer) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_private.select_archived_assessment_attempt_evidence(uuid, bigint, integer) TO ple_api_owner;
RESET ROLE;

SET LOCAL ROLE ple_api_owner;
-- Both deliberate operations hold the same Course SHARE lock until transaction
-- end. This seam is not application-callable and conveys no caller authority.
CREATE FUNCTION ple_api.lock_archived_course_for_recovery(
    p_course_public_reference text
) RETURNS TABLE (
    course_id uuid, course_reference_number text,
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
     WHERE course.public_reference = p_course_public_reference
       AND ple_api.current_session_account_is_course_instructor(course.course_id)
     FOR SHARE;
    IF NOT FOUND THEN RETURN; END IF;
    SELECT course_row.retention_starts_at + policy.archive_after_retention_start
               + policy.delete_after_archive INTO deletion_deadline
      FROM ple_data.course_retention_policy AS policy WHERE policy.policy_key;
    -- ASVS 2.3.2/8.3.2: use current wall time AFTER any lock wait, not caller
    -- time/transaction start. Archive marking cannot extend absolute expiry.
    IF course_row.retention_lifecycle_state <> 'archived'
       OR course_row.student_data_archived_at IS NULL
       OR course_row.student_data_deleted_at IS NOT NULL
       OR deletion_deadline IS NULL
       OR pg_catalog.clock_timestamp() >= deletion_deadline
       OR NOT ple_api.current_session_account_is_instructor()
       OR NOT ple_api.current_session_account_is_course_instructor(course_row.course_id)
       THEN RETURN; END IF;
    RETURN QUERY SELECT course_row.course_id, course_row.public_reference,
        course_row.student_data_archived_at, deletion_deadline;
END $$;
REVOKE ALL ON FUNCTION ple_api.lock_archived_course_for_recovery(text) FROM PUBLIC;

CREATE FUNCTION ple_api.read_archived_assessment_attempt_for_recovery(
    p_course_public_reference text, p_assessment_attempt_reference_number bigint
) RETURNS TABLE (
    course_reference_number text, student_record_id uuid, roster_id text,
    assessment_reference_number text,
    assessment_attempt_reference_number bigint, assessment_attempt_number integer,
    started_at timestamptz, expires_at timestamptz,
    student_data_archived_at timestamptz, delete_due_at timestamptz,
    attempt_facts jsonb, submission jsonb, questions jsonb
) LANGUAGE plpgsql VOLATILE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
DECLARE recovery_course record;
BEGIN
    SELECT * INTO recovery_course
      FROM ple_api.lock_archived_course_for_recovery(p_course_public_reference);
    IF NOT FOUND THEN RETURN; END IF;
    RETURN QUERY SELECT recovery_course.course_reference_number,
        evidence.student_record_id, roster.roster_id,
        evidence.assessment_reference_number, evidence.assessment_attempt_reference_number,
        evidence.assessment_attempt_number, evidence.started_at, evidence.expires_at,
        recovery_course.student_data_archived_at, recovery_course.delete_due_at,
        evidence.attempt_facts, evidence.submission, evidence.questions
      FROM ple_private.read_archived_assessment_attempt_evidence(
          recovery_course.course_id, p_assessment_attempt_reference_number
      ) AS evidence
      JOIN ple_data.student_record AS student
        ON student.student_record_id = evidence.student_record_id
       AND student.course_id = recovery_course.course_id
      LEFT JOIN ple_private.course_roster_profile AS roster
        ON roster.course_id = student.course_id
       AND roster.student_account_id = student.student_account_id;
END $$;
REVOKE ALL ON FUNCTION ple_api.read_archived_assessment_attempt_for_recovery(text, bigint) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_api.read_archived_assessment_attempt_for_recovery(text, bigint) TO ple_app;

CREATE FUNCTION ple_api.select_archived_assessment_attempts_for_recovery(
    p_course_public_reference text, p_after_reference_number bigint, p_limit integer
) RETURNS TABLE (
    course_reference_number text, roster_id text, assessment_reference_number text,
    assessment_title text, assessment_attempt_reference_number bigint,
    assessment_attempt_number integer, started_at timestamptz, submitted_at timestamptz,
    student_data_archived_at timestamptz, delete_due_at timestamptz
) LANGUAGE plpgsql VOLATILE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
DECLARE recovery_course record;
BEGIN
    -- Bounded reference keyset pagination; callers can request page size + 1.
    -- ASVS 14.2.6: no Student Record/Account UUID or evidence document in selection.
    IF p_limit IS NULL OR p_limit NOT BETWEEN 1 AND 101
       OR (p_after_reference_number IS NOT NULL AND p_after_reference_number < 1)
       THEN RETURN; END IF;
    SELECT * INTO recovery_course
      FROM ple_api.lock_archived_course_for_recovery(p_course_public_reference);
    -- An authorized empty page is distinct from an unavailable Course; the
    -- trusted adapter conceals this generic refusal without private detail.
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Retained Work unavailable';
    END IF;
    RETURN QUERY SELECT recovery_course.course_reference_number, roster.roster_id,
        evidence.assessment_reference_number, evidence.assessment_title,
        evidence.assessment_attempt_reference_number, evidence.assessment_attempt_number,
        evidence.started_at, evidence.submitted_at,
        recovery_course.student_data_archived_at, recovery_course.delete_due_at
      FROM ple_private.select_archived_assessment_attempt_evidence(
          recovery_course.course_id, p_after_reference_number, p_limit
      ) AS evidence
      JOIN ple_data.student_record AS student
        ON student.student_record_id = evidence.student_record_id
       AND student.course_id = recovery_course.course_id
      LEFT JOIN ple_private.course_roster_profile AS roster
        ON roster.course_id = student.course_id
       AND roster.student_account_id = student.student_account_id
     ORDER BY evidence.assessment_attempt_reference_number;
END $$;
REVOKE ALL ON FUNCTION ple_api.select_archived_assessment_attempts_for_recovery(text, bigint, integer) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_api.select_archived_assessment_attempts_for_recovery(text, bigint, integer) TO ple_app;
RESET ROLE;
