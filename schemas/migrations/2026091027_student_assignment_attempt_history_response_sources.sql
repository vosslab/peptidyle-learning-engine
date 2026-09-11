-- Private pinned-source reader for released completed-Attempt responses.
DO $$
BEGIN
    IF current_user <> 'ple_migrator' THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'migration 2026091027 must run as ple_migrator';
    END IF;
END
$$;

SET LOCAL ROLE ple_api_owner;
GRANT CREATE ON SCHEMA ple_api TO ple_private_owner;
RESET ROLE;

SET LOCAL ROLE ple_private_owner;
CREATE FUNCTION ple_api.read_student_assignment_attempt_history_response_sources(
    p_assignment_attempt_reference_number bigint
) RETURNS TABLE (
    "position" integer, student_response jsonb, backend text, question_attempt_id text,
    question_id text, revision_number integer, source_object_id text, source_object_address jsonb,
    source_object_checksum text, webwork_pg_path text, question_seed text,
    presentation_nonce text, presentation_checksum text, question_asset_renditions jsonb
) LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
    SELECT issued.issued_position + 1, submission.student_response, binding.backend,
           question_attempt.question_attempt_id::text, issued.question_id, issued.revision_number,
           binding.source_object_id::text, source_object.object_address, binding.source_object_checksum,
           binding.webwork_pg_path, question_attempt.question_seed::text,
           presentation.presentation_nonce, presentation.presentation_checksum,
           COALESCE(asset_renditions.question_asset_renditions, '[]'::jsonb)
      FROM ple_private.assignment_attempt AS attempt
      JOIN ple_data.student_record AS student ON student.student_record_id = attempt.student_record_id
      JOIN ple_data.assignment AS assignment ON assignment.assignment_id = attempt.assignment_id
      JOIN ple_private.issued_question AS issued ON issued.assignment_attempt_id = attempt.assignment_attempt_id
      JOIN ple_private.question_attempt AS question_attempt ON question_attempt.issued_question_id = issued.issued_question_id
      LEFT JOIN ple_private.question_submission AS submission ON submission.question_attempt_id = question_attempt.question_attempt_id
      JOIN ple_private.question_attempt_presentation_binding AS presentation ON presentation.question_attempt_id = question_attempt.question_attempt_id
      JOIN ple_private.question_revision_source_binding AS binding
        ON binding.question_id = issued.question_id AND binding.revision_number = issued.revision_number
      JOIN ple_private.object_record AS source_object ON source_object.object_id = binding.source_object_id
      LEFT JOIN LATERAL (
          SELECT jsonb_agg(jsonb_build_object(
              'asset_id', presented.asset_id,
              'question_asset_checksum', encode(publication.source_object_checksum, 'hex'),
              'rendition_checksum', encode(presented.rendition_checksum, 'hex'),
              'intrinsic_width', publication.intrinsic_width,
              'intrinsic_height', publication.intrinsic_height
          ) ORDER BY presented.asset_id) AS question_asset_renditions
            FROM ple_private.question_attempt_presentation_asset_rendition AS presented
            JOIN ple_private.question_asset_publication AS publication
              ON publication.asset_id = presented.asset_id
             AND publication.question_id = issued.question_id
             AND publication.revision_number = issued.revision_number
             AND publication.public_object_checksum = presented.rendition_checksum
             AND publication.publication_state = 'ready'
            JOIN ple_data.object_delivery AS delivery
              ON delivery.delivery_id = publication.delivery_id
             AND delivery.object_id = publication.public_object_id
             AND delivery.delivery_state = 'available'
           WHERE presented.question_attempt_id = question_attempt.question_attempt_id
      ) AS asset_renditions ON true
     WHERE attempt.reference_number = p_assignment_attempt_reference_number
       AND attempt.completed_at IS NOT NULL
       AND student.course_id = assignment.course_id
       AND student.student_account_id = ple_api.current_session_account_id()
       AND ple_api.current_session_account_owns_student_record(assignment.course_id, student.student_record_id)
       AND EXISTS (
           SELECT 1 FROM ple_data.course_membership AS membership
            WHERE membership.course_id = assignment.course_id
              AND membership.account_id = student.student_account_id
              AND membership.role = 'student'
              AND ple_data.course_membership_is_active(membership.membership_id)
       )
     ORDER BY issued.issued_position
$$;

REVOKE ALL PRIVILEGES ON FUNCTION ple_api.read_student_assignment_attempt_history_response_sources(bigint) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_api.read_student_assignment_attempt_history_response_sources(bigint) TO ple_app;
RESET ROLE;

SET LOCAL ROLE ple_api_owner;
REVOKE CREATE ON SCHEMA ple_api FROM ple_private_owner;
RESET ROLE;

-- Preserve current active delivery and admit the exact same owned completed
-- Attempt for a response image reproduced by the history reader.
SET LOCAL ROLE ple_private_owner;
CREATE OR REPLACE FUNCTION ple_private.resolve_ready_question_asset_delivery(
    p_asset_id uuid
) RETURNS TABLE (
    question_id text, revision_number integer, asset_id uuid, public_object_id uuid,
    rendition_checksum bytea
) LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
    SELECT publication.question_id, publication.revision_number, publication.asset_id,
           publication.public_object_id, publication.public_object_checksum
      FROM ple_private.question_asset_publication AS publication
      JOIN ple_data.object_delivery AS delivery
        ON delivery.delivery_id = publication.delivery_id
       AND delivery.object_id = publication.public_object_id
      JOIN ple_data.question_asset_delivery AS asset_delivery
        ON asset_delivery.delivery_id = publication.delivery_id
       AND asset_delivery.object_id = publication.public_object_id
       AND asset_delivery.question_id = publication.question_id
       AND asset_delivery.revision_number = publication.revision_number
       AND asset_delivery.asset_id = publication.asset_id
      JOIN ple_private.object_record AS public_record
        ON public_record.object_id = publication.public_object_id
      JOIN ple_private.question_attempt_presentation_asset_rendition AS presented_asset
        ON presented_asset.asset_id = publication.asset_id
       AND presented_asset.rendition_checksum = publication.public_object_checksum
      JOIN ple_private.question_attempt_presentation_asset_binding AS presented_assets
        ON presented_assets.question_attempt_id = presented_asset.question_attempt_id
      JOIN ple_private.question_attempt_presentation_binding AS presentation
        ON presentation.question_attempt_id = presented_asset.question_attempt_id
     WHERE publication.asset_id = p_asset_id
       AND publication.publication_state = 'ready'
       AND delivery.delivery_state = 'available'
       AND delivery.sha256 = publication.public_object_checksum
       AND delivery.media_type = publication.verified_media_type
       AND delivery.byte_length = publication.public_byte_length
       AND public_record.object_storage_area = 'public-assets'
       AND public_record.object_data_class = 'question-asset'
       AND public_record.sha256 = publication.public_object_checksum
       AND public_record.size_bytes = publication.public_byte_length
       AND public_record.media_type = publication.verified_media_type
       AND (
            ple_api.current_session_account_is_instructor()
            OR EXISTS (
                SELECT 1
                  FROM ple_private.issued_question AS issued
                  JOIN ple_private.assignment_attempt AS attempt
                    ON attempt.assignment_attempt_id = issued.assignment_attempt_id
                  JOIN ple_private.question_attempt AS question_attempt
                    ON question_attempt.issued_question_id = issued.issued_question_id
                   AND question_attempt.question_attempt_id = presented_asset.question_attempt_id
                  JOIN ple_data.assignment AS assignment
                    ON assignment.assignment_id = attempt.assignment_id
                  JOIN ple_data.course_instance AS course
                    ON course.course_id = assignment.course_id
                 WHERE issued.question_id = publication.question_id
                   AND issued.revision_number = publication.revision_number
                   AND ple_api.current_session_account_owns_student_record(
                       assignment.course_id, attempt.student_record_id
                   )
                   AND (
                       (attempt.completed_at IS NULL AND (
                           SELECT access_decision.start_decision = 'may_start'
                             FROM ple_api.live_demo_assignment_access(
                                 course.reference_number, assignment.reference_number
                             ) AS access_decision
                       ))
                       OR (
                           attempt.completed_at IS NOT NULL
                           AND EXISTS (
                               SELECT 1 FROM ple_data.student_record AS student
                               JOIN ple_data.course_membership AS membership
                                 ON membership.course_id = assignment.course_id
                                AND membership.account_id = student.student_account_id
                                AND membership.role = 'student'
                                AND ple_data.course_membership_is_active(membership.membership_id)
                              WHERE student.student_record_id = attempt.student_record_id
                                AND student.course_id = assignment.course_id
                                AND student.student_account_id = ple_api.current_session_account_id()
                           )
                       )
                   )
            )
       )
$$;
REVOKE ALL PRIVILEGES ON FUNCTION ple_private.resolve_ready_question_asset_delivery(uuid) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_private.resolve_ready_question_asset_delivery(uuid) TO ple_api_owner;
RESET ROLE;
