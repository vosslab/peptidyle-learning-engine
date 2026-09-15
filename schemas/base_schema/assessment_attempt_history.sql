-- Completed Student Assessment Attempt history.  This projection reads only
-- retained Assessment Attempt and Issued Question evidence; current Assessment content
-- is deliberately not an interpretation source.

-- The API owner has the existing narrow Course Membership read capability.
-- Expose only current Student Record identifiers to the private completion
-- predicate; Account state and invitations are deliberately not cohort facts.
SET LOCAL ROLE ple_api_owner;
CREATE FUNCTION ple_api.current_course_student_record_ids(p_course_id uuid)
RETURNS TABLE (student_record_id uuid)
LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_data AS $$
    SELECT membership.student_record_id
      FROM ple_data.course_membership AS membership
     WHERE membership.course_id = p_course_id
       AND membership.role = 'student'
       AND ple_data.course_membership_is_active(membership.membership_id)
$$;
REVOKE ALL ON FUNCTION ple_api.current_course_student_record_ids(uuid) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_api.current_course_student_record_ids(uuid) TO ple_private_owner;
RESET ROLE;

SET LOCAL ROLE ple_private_owner;
-- ASVS 2.3.1, 8.2.2, 15.4.2: derive cohort completion from current
-- membership and immutable Assessment Submission evidence at the read instant.
-- There is no latch, snapshot, invitation, or Account-active filter.
CREATE FUNCTION ple_private.current_student_cohort_completed_assessment(p_assessment_id uuid)
RETURNS boolean LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
    SELECT EXISTS (
               SELECT 1 FROM ple_data.assessment AS assessment
                WHERE assessment.assessment_id = p_assessment_id
           )
       AND NOT EXISTS (
               SELECT 1
                 FROM ple_data.assessment AS assessment
                 CROSS JOIN LATERAL ple_api.current_course_student_record_ids(
                     assessment.course_id
                 ) AS student
                WHERE assessment.assessment_id = p_assessment_id
                  AND NOT EXISTS (
                      SELECT 1
                        FROM ple_private.assessment_attempt AS assessment_attempt
                        JOIN ple_private.assessment_submission AS submission
                          ON submission.assessment_attempt_id = assessment_attempt.assessment_attempt_id
                       WHERE assessment_attempt.assessment_id = assessment.assessment_id
                         AND assessment_attempt.student_record_id = student.student_record_id
                  )
           )
$$;
REVOKE ALL ON FUNCTION ple_private.current_student_cohort_completed_assessment(uuid) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_private.current_student_cohort_completed_assessment(uuid) TO ple_api_owner;

CREATE FUNCTION ple_private.read_student_assessment_attempt_history(
    p_assessment_attempt_reference_number bigint
) RETURNS TABLE (
    course_id uuid, assessment_reference_number text, assessment_title text, assessment_type text,
    assessment_attempt_number integer,
    state text, questions jsonb, feedback_rule jsonb, due_at_millis bigint,
    closes_at_millis bigint, submitted_at_millis bigint, evaluated_at_millis bigint,
    all_students_completed boolean,
    grading_is_current boolean, grading_results jsonb
) LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private, ple_audit AS $$
    WITH owned_assessment_attempt AS (
        SELECT assessment_attempt.assessment_attempt_id, assessment_attempt.assessment_id,
               assessment.course_id,
               assessment.public_reference AS assessment_reference_number,
               assessment_attempt.assessment_title,
               assessment.assessment_type,
               assessment_attempt.assessment_attempt_number,
               assessment_attempt.due_at,
               assessment_attempt.closes_at,
               jsonb_build_object(
                   'score', assessment_attempt.feedback_score,
                   'per_item_correctness', assessment_attempt.feedback_per_item_correctness,
                   'submitted_response', assessment_attempt.feedback_submitted_response,
                   'question_feedback', assessment_attempt.feedback_question_feedback,
                   'question_answer', assessment_attempt.feedback_question_answer,
                   'question_answer_explanation', assessment_attempt.feedback_question_answer_explanation,
                   'class_statistics', assessment_attempt.feedback_class_statistics
               ) AS feedback_rule
          FROM ple_private.assessment_attempt AS assessment_attempt
          JOIN ple_data.assessment AS assessment
            ON assessment.assessment_id = assessment_attempt.assessment_id
         WHERE assessment_attempt.reference_number = p_assessment_attempt_reference_number
           AND EXISTS (
               SELECT 1 FROM ple_private.assessment_submission AS submission
                WHERE submission.assessment_attempt_id = assessment_attempt.assessment_attempt_id
           )
           AND ple_api.current_session_account_owns_student_record(
               assessment.course_id, assessment_attempt.student_record_id
           )
           -- ASVS 2.3.1: archive is an ordered retention transition.  Once
           -- it occurs, an otherwise-owned Assessment Attempt reference is not a path
           -- back into ordinary Student history.
           AND ple_api.course_student_work_is_ordinarily_visible(assessment.course_id)
    )
    SELECT owned.course_id,
           owned.assessment_reference_number,
           owned.assessment_title,
           owned.assessment_type,
           owned.assessment_attempt_number,
           CASE WHEN assessment_submission.assessment_attempt_id IS NOT NULL
                THEN 'submitted'::text ELSE 'closed'::text END,
           questions.questions,
           owned.feedback_rule,
           floor(extract(epoch FROM owned.due_at) * 1000)::bigint,
           floor(extract(epoch FROM owned.closes_at) * 1000)::bigint,
           floor(extract(epoch FROM assessment_submission.submitted_at) * 1000)::bigint,
           floor(extract(epoch FROM pg_catalog.statement_timestamp()) * 1000)::bigint,
           ple_private.current_student_cohort_completed_assessment(
               owned.assessment_id
           ),
           grading.grading_is_current,
           grading.grading_results
      FROM owned_assessment_attempt AS owned
      LEFT JOIN ple_private.assessment_submission AS assessment_submission
        ON assessment_submission.assessment_attempt_id = owned.assessment_attempt_id
      CROSS JOIN LATERAL (
          SELECT COALESCE(jsonb_agg(jsonb_build_object(
                     'position', issued.issued_position + 1,
                     'questionId', issued.question_id,
                     'revisionNumber', issued.revision_number,
                     'responseState', CASE question_attempt.question_attempt_state
                         WHEN 'submission_accepted' THEN 'submitted'
                         ELSE 'closed'
                     END
                 ) ORDER BY issued.issued_position), '[]'::jsonb) AS questions
            FROM ple_private.issued_question AS issued
            JOIN ple_private.question_attempt
              ON question_attempt.issued_question_id = issued.issued_question_id
           WHERE issued.assessment_attempt_id = owned.assessment_attempt_id
      ) AS questions
      CROSS JOIN LATERAL (
          SELECT count(*) > 0
                     AND bool_and(
                         question_attempt.question_attempt_state = 'closed_unanswered'
                         OR (
                             result.grading_result_id IS NOT NULL
                             AND grading_state.grading_state = 'graded'
                             AND ple_api.has_automated_grading_receipt(
                                 grading_state.question_submission_grading_id,
                                 result.grading_result_id
                             )
                         )
                     ) AS grading_is_current,
                 CASE WHEN count(*) > 0
                           AND bool_and(
                               question_attempt.question_attempt_state = 'closed_unanswered'
                               OR (
                                   result.grading_result_id IS NOT NULL
                                   AND grading_state.grading_state = 'graded'
                                   AND ple_api.has_automated_grading_receipt(
                                       grading_state.question_submission_grading_id,
                                       result.grading_result_id
                                   )
                               )
                           )
                      THEN jsonb_agg(jsonb_build_object(
                          'position', issued.issued_position + 1,
                          'correct', coalesce(result.normalized_credit = 1, false),
                          'pointsEarned', score.points_earned,
                          'pointsPossible', score.points_possible
                      ) ORDER BY issued.issued_position)
                      ELSE '[]'::jsonb END AS grading_results
            FROM ple_private.issued_question AS issued
            JOIN ple_private.question_attempt
              ON question_attempt.issued_question_id = issued.issued_question_id
            LEFT JOIN ple_private.question_submission AS submission
              ON submission.question_attempt_id = question_attempt.question_attempt_id
            LEFT JOIN ple_private.question_submission_grading AS grading_state
              ON grading_state.submission_id = submission.submission_id
            LEFT JOIN ple_private.grading_result AS result
              ON result.question_submission_grading_id = grading_state.question_submission_grading_id
             AND result.submission_id = submission.submission_id
             AND result.question_attempt_id = question_attempt.question_attempt_id
            JOIN ple_data.assessment_entry AS entry
              ON entry.assessment_entry_id = issued.assessment_entry_id
            CROSS JOIN LATERAL ple_private.score_recorded_credit(
                coalesce(result.normalized_credit, 0), issued.scoring_rule,
                CASE entry.entry_kind
                    WHEN 'fixed_question' THEN entry.points_possible
                    ELSE entry.points_per_item
                END
            ) AS score
           WHERE issued.assessment_attempt_id = owned.assessment_attempt_id
      ) AS grading
$$;

REVOKE ALL ON FUNCTION ple_private.read_student_assessment_attempt_history(bigint) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_private.read_student_assessment_attempt_history(bigint) TO ple_api_owner;
RESET ROLE;

SET LOCAL ROLE ple_api_owner;
CREATE FUNCTION ple_api.read_student_assessment_attempt_history(
    p_assessment_attempt_reference_number bigint
) RETURNS TABLE (
    course_reference_number text, course_short_name text, course_long_name text, course_theme text,
    assessment_reference_number text, assessment_title text, assessment_type text,
    assessment_attempt_number integer,
    state text, questions jsonb, feedback_rule jsonb, due_at_millis bigint,
    closes_at_millis bigint, submitted_at_millis bigint, evaluated_at_millis bigint,
    all_students_completed boolean,
    grading_is_current boolean, grading_results jsonb
) LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_data, ple_private AS $$
    SELECT course.public_reference,
           course.course_short_name,
           course.course_long_name,
           course.course_theme,
           history.assessment_reference_number,
           history.assessment_title,
           history.assessment_type,
           history.assessment_attempt_number,
           history.state,
           history.questions,
           history.feedback_rule,
           history.due_at_millis,
           history.closes_at_millis,
           history.submitted_at_millis,
           history.evaluated_at_millis,
           history.all_students_completed,
           history.grading_is_current,
           history.grading_results
      FROM ple_private.read_student_assessment_attempt_history(
               p_assessment_attempt_reference_number
           ) AS history
      JOIN ple_data.course_instance AS course ON course.course_id = history.course_id
$$;

REVOKE ALL ON FUNCTION ple_api.read_student_assessment_attempt_history(bigint) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_api.read_student_assessment_attempt_history(bigint) TO ple_app;
RESET ROLE;

-- Reproduction facts are private retained evidence.  The server receives them
-- only for an owned completed Assessment Attempt and only after at least one teaching
-- content field is releasable from that Assessment Attempt's copied feedback policy.
-- The selected Question Revision and source binding are immutable, so this
-- reader never interprets a past Assessment Attempt through current Assessment content.
SET LOCAL ROLE ple_private_owner;
CREATE FUNCTION ple_private.read_student_assessment_attempt_history_response_sources(
    p_assessment_attempt_reference_number bigint
) RETURNS TABLE (
    "position" integer, assessment_attempt_id uuid, student_response jsonb, backend text, question_attempt_id uuid,
    question_id text, revision_number integer, general_feedback text, source_object_id uuid, source_object_address jsonb,
    source_object_checksum text, webwork_pg_path text, question_seed text,
    generated_parameter_sha256 text,
    presentation_nonce text, presentation_checksum text, presentation jsonb,
    question_asset_renditions jsonb, response_item_bindings jsonb
) LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
    WITH owned_assessment_attempt AS (
        SELECT assessment_attempt.assessment_attempt_id,
               assessment_attempt.assessment_id,
               assessment.assessment_type,
               assessment_attempt.feedback_submitted_response,
               assessment_attempt.feedback_question_feedback,
               assessment_attempt.feedback_question_answer,
               assessment_attempt.feedback_question_answer_explanation,
               assessment_attempt.due_at,
               assessment_attempt.closes_at,
               assessment_submission.submitted_at AS assessment_submitted_at,
               ple_private.current_student_cohort_completed_assessment(
                   assessment_attempt.assessment_id
               ) AS all_students_completed
          FROM ple_private.assessment_attempt AS assessment_attempt
          JOIN ple_data.assessment AS assessment
            ON assessment.assessment_id = assessment_attempt.assessment_id
          JOIN ple_private.assessment_submission
            ON assessment_submission.assessment_attempt_id = assessment_attempt.assessment_attempt_id
         WHERE assessment_attempt.reference_number = p_assessment_attempt_reference_number
           -- ASVS 8.2.2 and 8.3.1: the trusted authorization helper proves
           -- the exact Course, Student Record, session Account, and active
           -- Student Membership without widening this private reader's table
           -- privileges.
           AND ple_api.current_session_account_owns_student_record(
               assessment.course_id, assessment_attempt.student_record_id
           )
           -- ASVS 2.3.1: richer response-source history obeys the exact
           -- ordinary-visibility boundary too.
           AND ple_api.course_student_work_is_ordinarily_visible(assessment.course_id)
    ), released_assessment_attempt AS (
        SELECT owned.*,
               (
                   owned.feedback_submitted_response = 'during_attempt'
                   OR (owned.feedback_submitted_response = 'after_submit'
                       AND owned.assessment_submitted_at IS NOT NULL)
                   OR (owned.feedback_submitted_response = 'after_due'
                       AND owned.due_at IS NOT NULL
                       AND pg_catalog.statement_timestamp() >= owned.due_at)
                   OR (owned.feedback_submitted_response = 'after_close'
                       AND owned.closes_at IS NOT NULL
                       AND pg_catalog.statement_timestamp() >= owned.closes_at)
               ) AS submitted_response_is_released,
               (
                   owned.feedback_question_feedback = 'during_attempt'
                   OR (owned.feedback_question_feedback = 'after_submit'
                       AND owned.assessment_submitted_at IS NOT NULL)
                   OR (owned.feedback_question_feedback = 'after_due'
                       AND owned.due_at IS NOT NULL
                       AND pg_catalog.statement_timestamp() >= owned.due_at)
                   OR (owned.feedback_question_feedback = 'after_close'
                       AND owned.closes_at IS NOT NULL
                       AND pg_catalog.statement_timestamp() >= owned.closes_at)
                   OR (
                       (owned.assessment_type NOT IN ('quiz', 'exam')
                           OR owned.all_students_completed)
                       AND (
                           owned.feedback_question_answer = 'during_attempt'
                           OR (owned.feedback_question_answer = 'after_submit'
                               AND owned.assessment_submitted_at IS NOT NULL)
                           OR (owned.feedback_question_answer = 'after_due'
                               AND owned.due_at IS NOT NULL
                               AND pg_catalog.statement_timestamp() >= owned.due_at)
                           OR (owned.feedback_question_answer = 'after_close'
                               AND owned.closes_at IS NOT NULL
                               AND pg_catalog.statement_timestamp() >= owned.closes_at)
                           OR owned.feedback_question_answer_explanation = 'during_attempt'
                           OR (owned.feedback_question_answer_explanation = 'after_submit'
                               AND owned.assessment_submitted_at IS NOT NULL)
                           OR (owned.feedback_question_answer_explanation = 'after_due'
                               AND owned.due_at IS NOT NULL
                               AND pg_catalog.statement_timestamp() >= owned.due_at)
                           OR (owned.feedback_question_answer_explanation = 'after_close'
                               AND owned.closes_at IS NOT NULL
                               AND pg_catalog.statement_timestamp() >= owned.closes_at)
                       )
                   )
               ) AS teaching_content_is_released
          FROM owned_assessment_attempt AS owned
    )
    SELECT issued.issued_position,
           released.assessment_attempt_id,
           CASE WHEN released.submitted_response_is_released
                THEN submission.student_response END,
           binding.backend,
           question_attempt.question_attempt_id,
           issued.question_id,
           issued.revision_number,
           revision.general_feedback,
           binding.source_object_id,
           source_object.object_address,
           binding.source_object_checksum,
           binding.webwork_pg_path,
           question_attempt.question_seed::text,
           question_attempt.generated_parameter_sha256,
           presentation.presentation_nonce,
           presentation.presentation_checksum,
           presentation.presentation,
           COALESCE(asset_renditions.question_asset_renditions, '[]'::jsonb),
           COALESCE(response_item_bindings.response_item_bindings, '[]'::jsonb)
      FROM released_assessment_attempt AS released
      JOIN ple_private.issued_question AS issued
        ON issued.assessment_attempt_id = released.assessment_attempt_id
      JOIN ple_private.question_attempt
        ON question_attempt.issued_question_id = issued.issued_question_id
      JOIN ple_data.question_revision AS revision
        ON revision.question_id = issued.question_id
       AND revision.revision_number = issued.revision_number
      LEFT JOIN ple_private.question_submission AS submission
        ON submission.question_attempt_id = question_attempt.question_attempt_id
      JOIN ple_private.question_attempt_presentation_binding AS presentation
        ON presentation.question_attempt_id = question_attempt.question_attempt_id
      LEFT JOIN ple_private.question_revision_source_binding AS binding
        ON binding.question_id = issued.question_id
       AND binding.revision_number = issued.revision_number
      LEFT JOIN ple_private.object_record AS source_object
        ON source_object.object_id = binding.source_object_id
      LEFT JOIN LATERAL (
          SELECT jsonb_agg(jsonb_build_object(
              'asset_id', presented.asset_id,
              'question_asset_checksum', encode(presented.question_asset_checksum, 'hex'),
              'rendition_checksum', encode(presented.rendition_checksum, 'hex'),
              'intrinsic_width', presented.intrinsic_width,
              'intrinsic_height', presented.intrinsic_height
          ) ORDER BY presented.asset_id) AS question_asset_renditions
            FROM ple_private.question_attempt_presentation_asset_rendition AS presented
           WHERE presented.question_attempt_id = question_attempt.question_attempt_id
      ) AS asset_renditions ON true
      LEFT JOIN LATERAL (
          SELECT jsonb_agg(jsonb_build_object(
              'presentation_response_item_reference', response_item.presentation_response_item_reference,
              'response_item_reference', response_item.response_item_reference
          ) ORDER BY response_item.presentation_response_item_reference) AS response_item_bindings
            FROM ple_private.question_attempt_response_item_binding AS response_item
           WHERE response_item.question_attempt_id = question_attempt.question_attempt_id
      ) AS response_item_bindings ON true
     WHERE released.submitted_response_is_released
        OR released.teaching_content_is_released
     ORDER BY issued.issued_position
$$;
REVOKE ALL ON FUNCTION ple_private.read_student_assessment_attempt_history_response_sources(bigint)
    FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_private.read_student_assessment_attempt_history_response_sources(bigint)
    TO ple_api_owner;
RESET ROLE;

SET LOCAL ROLE ple_api_owner;
CREATE FUNCTION ple_api.read_student_assessment_attempt_history_response_sources(
    p_assessment_attempt_reference_number bigint
) RETURNS TABLE (
    "position" integer, assessment_attempt_id uuid, student_response jsonb, backend text, question_attempt_id uuid,
    question_id text, revision_number integer, general_feedback text, source_object_id uuid, source_object_address jsonb,
    source_object_checksum text, webwork_pg_path text, question_seed text,
    generated_parameter_sha256 text,
    presentation_nonce text, presentation_checksum text, presentation jsonb,
    question_asset_renditions jsonb, response_item_bindings jsonb
) LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private AS $$
    SELECT "position" + 1, assessment_attempt_id, student_response, backend, question_attempt_id,
           question_id, revision_number, general_feedback, source_object_id, source_object_address,
           source_object_checksum, webwork_pg_path, question_seed,
           generated_parameter_sha256,
           presentation_nonce, presentation_checksum, presentation, question_asset_renditions,
           response_item_bindings
      FROM ple_private.read_student_assessment_attempt_history_response_sources($1)
$$;
REVOKE ALL ON FUNCTION ple_api.read_student_assessment_attempt_history_response_sources(bigint)
    FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_api.read_student_assessment_attempt_history_response_sources(bigint)
    TO ple_app;
RESET ROLE;

-- Archived Course Student Work is deliberately absent from ordinary Student
-- and Instructor projections, but C208's no-login retention capability may
-- verify the still-retained Work immediately before its scheduled deletion.
-- This is a protected pre-delete boundary, not a recovery interface, queue,
-- or state machine.  ASVS 2.3.1 keeps it reachable only after archive and
-- before the one-way delete transition.
SET LOCAL ROLE ple_private_owner;
CREATE FUNCTION ple_private.read_course_student_work_for_retention(
    p_student_record_id uuid
) RETURNS TABLE (
    assessment_attempt_reference_number bigint,
    student_record_id uuid,
    assessment_id uuid,
    assessment_attempt_started_at timestamptz,
    assessment_submitted_at timestamptz
) LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_data, ple_private AS $$
    SELECT assessment_attempt.reference_number,
           assessment_attempt.student_record_id,
           assessment_attempt.assessment_id,
           assessment_attempt.started_at,
           submission.submitted_at
      FROM ple_private.assessment_attempt AS assessment_attempt
      LEFT JOIN ple_private.assessment_submission AS submission
        ON submission.assessment_attempt_id = assessment_attempt.assessment_attempt_id
     WHERE assessment_attempt.student_record_id = p_student_record_id
     ORDER BY assessment_attempt.reference_number
$$;
REVOKE ALL ON FUNCTION ple_private.read_course_student_work_for_retention(uuid)
    FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_private.read_course_student_work_for_retention(uuid)
    TO ple_api_owner;
RESET ROLE;

SET LOCAL ROLE ple_api_owner;
CREATE FUNCTION ple_api.read_archived_course_student_work_for_retention(
    p_course_id uuid
) RETURNS TABLE (
    assessment_attempt_reference_number bigint,
    student_record_id uuid,
    assessment_id uuid,
    assessment_attempt_started_at timestamptz,
    assessment_submitted_at timestamptz,
    student_data_archived_at timestamptz
) LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
    SELECT work.assessment_attempt_reference_number,
           work.student_record_id,
           work.assessment_id,
           work.assessment_attempt_started_at,
           work.assessment_submitted_at,
           course.student_data_archived_at
      FROM ple_data.course_instance AS course
      JOIN ple_data.student_record AS student
        ON student.course_id = course.course_id
      CROSS JOIN LATERAL ple_private.read_course_student_work_for_retention(student.student_record_id) AS work
     WHERE course.course_id = p_course_id
       AND course.retention_lifecycle_state = 'archived'
       AND course.student_data_archived_at IS NOT NULL
       AND course.student_data_deleted_at IS NULL
$$;
REVOKE ALL ON FUNCTION ple_api.read_archived_course_student_work_for_retention(uuid)
    FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_api.read_archived_course_student_work_for_retention(uuid)
    TO ple_course_retention_executor;
RESET ROLE;
