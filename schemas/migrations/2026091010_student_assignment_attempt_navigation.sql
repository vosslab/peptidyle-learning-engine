-- Student Assignment Attempt navigation is a read projection over immutable
-- issued work.  It has no mutable current-position state.
SET LOCAL ROLE ple_private_owner;
ALTER TABLE ple_private.assignment_attempt
    ADD COLUMN reference_number bigint GENERATED ALWAYS AS IDENTITY,
    ADD CONSTRAINT assignment_attempt_reference_number_is_positive
        CHECK (reference_number > 0 AND reference_number <= 2147483647),
    ADD CONSTRAINT assignment_attempt_reference_number_unique UNIQUE (reference_number);
CREATE INDEX assignment_attempt_student_reference_idx
    ON ple_private.assignment_attempt (reference_number, student_record_id);
RESET ROLE;

SET LOCAL ROLE ple_api_owner;
-- The navigation readers traverse forced-RLS private issued-work and durable
-- asset-rendition evidence.  Define them under the private owner, the same
-- tightly scoped definer boundary used by existing Student delivery reads;
-- each reader still reauthorizes the installed caller session in its query.
GRANT CREATE ON SCHEMA ple_api TO ple_private_owner;
RESET ROLE;

SET LOCAL ROLE ple_private_owner;

-- Resolve the active Attempt only through the same caller-bound, forced-RLS
-- boundary as progress and position reads.  The delivery store uses this
-- compact projection instead of querying private issued-work tables as
-- `ple_app` after session installation.
CREATE FUNCTION ple_api.read_active_student_assignment_attempt_reference(
    p_course_reference_number bigint,
    p_assignment_reference_number bigint
) RETURNS TABLE (assignment_attempt_reference_number bigint)
LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
    SELECT attempt.reference_number
      FROM ple_private.assignment_attempt AS attempt
      JOIN ple_data.assignment AS assignment
        ON assignment.assignment_id = attempt.assignment_id
      JOIN ple_data.course_instance AS course
        ON course.course_id = assignment.course_id
      JOIN ple_data.student_record AS student
        ON student.student_record_id = attempt.student_record_id
     WHERE p_course_reference_number BETWEEN 1 AND 2147483647
       AND p_assignment_reference_number BETWEEN 1 AND 2147483647
       AND course.reference_number = p_course_reference_number
       AND assignment.reference_number = p_assignment_reference_number
       AND attempt.completed_at IS NULL
       AND student.course_id = assignment.course_id
       AND student.student_account_id = ple_api.current_session_account_id()
       AND ple_api.current_session_account_owns_student_record(
             assignment.course_id, student.student_record_id
       )
       AND EXISTS (
           SELECT 1 FROM ple_data.course_membership AS membership
            WHERE membership.course_id = assignment.course_id
              AND membership.account_id = student.student_account_id
              AND membership.role = 'student'
              AND ple_data.course_membership_is_active(membership.membership_id)
       )
     ORDER BY attempt.attempt_number DESC
     LIMIT 1
$$;

CREATE FUNCTION ple_api.read_student_assignment_attempt_progress(
    p_assignment_attempt_reference_number bigint
) RETURNS TABLE (
    assignment_attempt_reference_number bigint, question_count integer,
    recommended_position integer, issued_position integer, response_state text
) LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
    WITH owned_attempt AS (
        SELECT attempt.assignment_attempt_id, attempt.reference_number
          FROM ple_private.assignment_attempt AS attempt
          JOIN ple_data.student_record AS student
            ON student.student_record_id = attempt.student_record_id
          JOIN ple_data.assignment AS assignment ON assignment.assignment_id = attempt.assignment_id
         WHERE attempt.reference_number = p_assignment_attempt_reference_number
           AND attempt.completed_at IS NULL
           AND student.course_id = assignment.course_id
           AND student.student_account_id = ple_api.current_session_account_id()
           AND ple_api.current_session_account_owns_student_record(
                 assignment.course_id, student.student_record_id
           )
           AND EXISTS (
               SELECT 1 FROM ple_data.course_membership AS membership
                WHERE membership.course_id = assignment.course_id
                  AND membership.account_id = student.student_account_id
                  AND membership.role = 'student'
                  AND ple_data.course_membership_is_active(membership.membership_id)
           )
    ), positions AS (
        SELECT owned.reference_number, issued.issued_position + 1 AS issued_position,
               CASE
                   WHEN submission.question_attempt_id IS NOT NULL THEN 'submitted'
                   WHEN question_attempt.question_attempt_state = 'closed_at_deadline' THEN 'closed'
                   ELSE 'unanswered'
               END AS response_state
          FROM owned_attempt AS owned
          JOIN ple_private.issued_question AS issued
            ON issued.assignment_attempt_id = owned.assignment_attempt_id
          JOIN ple_private.question_attempt AS question_attempt
            ON question_attempt.issued_question_id = issued.issued_question_id
          LEFT JOIN ple_private.question_submission AS submission
            ON submission.question_attempt_id = question_attempt.question_attempt_id
    )
    SELECT reference_number,
           count(*) OVER ()::integer,
           min(issued_position) FILTER (WHERE response_state = 'unanswered') OVER (),
           issued_position, response_state
      FROM positions
     ORDER BY issued_position
$$;

-- The server receives private source and replay facts only after this function
-- has re-authorized the public reference, ownership, current membership, and
-- one bounded 1-based position.  Browser callers never supply UUIDs.
CREATE FUNCTION ple_api.read_student_assignment_attempt_position(
    p_assignment_attempt_reference_number bigint, p_position integer
) RETURNS TABLE (
    backend text, question_attempt_id uuid, question_id text, revision_number integer, source_object_id uuid,
    source_object_address jsonb, source_object_checksum text, webwork_pg_path text,
    question_seed numeric, presentation_nonce text, presentation_checksum text,
    question_asset_renditions jsonb
) LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
    SELECT binding.backend, question_attempt.question_attempt_id, issued.question_id, issued.revision_number,
           binding.source_object_id, source_object.object_address,
           binding.source_object_checksum, binding.webwork_pg_path,
           question_attempt.question_seed, presentation.presentation_nonce,
           presentation.presentation_checksum,
           COALESCE(asset_renditions.question_asset_renditions, '[]'::jsonb)
      FROM ple_private.assignment_attempt AS attempt
      JOIN ple_data.student_record AS student ON student.student_record_id = attempt.student_record_id
      JOIN ple_data.assignment AS assignment ON assignment.assignment_id = attempt.assignment_id
      JOIN ple_private.issued_question AS issued ON issued.assignment_attempt_id = attempt.assignment_attempt_id
      JOIN ple_private.question_attempt AS question_attempt ON question_attempt.issued_question_id = issued.issued_question_id
      JOIN ple_private.question_attempt_presentation_binding AS presentation ON presentation.question_attempt_id = question_attempt.question_attempt_id
      JOIN ple_private.question_revision_source_binding AS binding
        ON binding.question_id = issued.question_id AND binding.revision_number = issued.revision_number
      JOIN ple_private.object_record AS source_object ON source_object.object_id = binding.source_object_id
      LEFT JOIN LATERAL (
          SELECT jsonb_agg(
              jsonb_build_object(
                  'asset_id', presented.asset_id,
                  'question_asset_checksum', encode(publication.source_object_checksum, 'hex'),
                  'rendition_checksum', encode(presented.rendition_checksum, 'hex'),
                  'intrinsic_width', publication.intrinsic_width,
                  'intrinsic_height', publication.intrinsic_height
              ) ORDER BY presented.asset_id
          ) AS question_asset_renditions
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
       AND p_position BETWEEN 1 AND 2147483647
       AND issued.issued_position = p_position - 1
       AND attempt.completed_at IS NULL
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
$$;
REVOKE ALL PRIVILEGES ON FUNCTION ple_api.read_active_student_assignment_attempt_reference(bigint, bigint),
    ple_api.read_student_assignment_attempt_progress(bigint),
    ple_api.read_student_assignment_attempt_position(bigint, integer) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_api.read_active_student_assignment_attempt_reference(bigint, bigint),
    ple_api.read_student_assignment_attempt_progress(bigint),
    ple_api.read_student_assignment_attempt_position(bigint, integer) TO ple_app;
RESET ROLE;

SET LOCAL ROLE ple_api_owner;
REVOKE CREATE ON SCHEMA ple_api FROM ple_private_owner;
RESET ROLE;
