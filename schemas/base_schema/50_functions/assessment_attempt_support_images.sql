-- Support image retention is called only after the trusted server projects
-- actual returned support fields. Disclosure timing stays in the domain.

SET LOCAL ROLE ple_private_owner;

CREATE FUNCTION ple_private.register_student_assessment_attempt_support_images(
    p_assessment_attempt_id uuid, p_issued_position integer,
    p_published_question_id text, p_revision_number integer, p_images jsonb
) RETURNS void LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
DECLARE assessment_id_value text;
DECLARE course_instance_id_value text;
DECLARE question_attempt_id_value uuid;
BEGIN
    IF p_images IS NULL OR jsonb_typeof(p_images) <> 'array' THEN
        RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Disclosed support images are invalid';
    END IF;
    -- ASVS 8.2.2/8.3.1: authorization is bound to the original Student,
    -- exact completed Attempt, current membership, and ordinary visibility.
    SELECT assessment.assessment_id INTO assessment_id_value
      FROM ple_private.assessment_attempt AS attempt
      JOIN ple_data.assessment AS assessment ON assessment.assessment_id = attempt.assessment_id
     WHERE attempt.assessment_attempt_id = p_assessment_attempt_id
       AND ple_api.current_session_account_owns_student_record(
           assessment.course_instance_id, attempt.student_record_id)
       AND ple_api.course_student_work_is_ordinarily_visible(assessment.course_instance_id);
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Disclosed support images are unavailable';
    END IF;
    -- Match Student Work's Assessment-first lock so Unrelease cannot race
    -- image retention. Repeat the owner/visibility proof after the lock.
    PERFORM 1 FROM ple_data.assessment
     WHERE assessment_id = assessment_id_value FOR UPDATE;
    SELECT attempt.course_instance_id, question_attempt.question_attempt_id
      INTO course_instance_id_value, question_attempt_id_value
      FROM ple_private.assessment_attempt AS attempt
      JOIN ple_data.assessment AS assessment ON assessment.assessment_id = attempt.assessment_id
      JOIN ple_private.assessment_submission AS submission
        ON submission.course_instance_id = attempt.course_instance_id
       AND submission.assessment_attempt_id = attempt.assessment_attempt_id
      JOIN ple_private.issued_question AS issued
        ON issued.course_instance_id = attempt.course_instance_id
       AND issued.assessment_attempt_id = attempt.assessment_attempt_id
      JOIN ple_private.question_attempt AS question_attempt
        ON question_attempt.course_instance_id = issued.course_instance_id
       AND question_attempt.issued_question_id = issued.issued_question_id
      JOIN ple_private.question_attempt_presentation_binding AS presentation
        ON presentation.course_instance_id = question_attempt.course_instance_id
       AND presentation.question_attempt_id = question_attempt.question_attempt_id
      JOIN ple_private.question_revision_source_binding AS source
        ON source.published_question_id = issued.published_question_id
       AND source.revision_number = issued.revision_number
       AND source.backend = 'ple'
     WHERE attempt.assessment_attempt_id = p_assessment_attempt_id
       AND issued.issued_position = p_issued_position
       AND issued.published_question_id = p_published_question_id
       AND issued.revision_number = p_revision_number
       AND ple_api.current_session_account_owns_student_record(
           assessment.course_instance_id, attempt.student_record_id)
       AND ple_api.course_student_work_is_ordinarily_visible(assessment.course_instance_id);
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Disclosed support images are unavailable';
    END IF;
    -- ASVS 8.2.3: the compiler-authored source-relative name and tuple must
    -- agree with the exact immutable Revision's Ready rendition registry.
    IF EXISTS (
        SELECT 1
          FROM jsonb_to_recordset(p_images) AS supplied(
              source_relative_name text, question_image_asset_id uuid, question_image_checksum text)
         WHERE supplied.source_relative_name IS NULL
            OR supplied.question_image_asset_id IS NULL
            OR supplied.question_image_checksum IS NULL
            OR NOT EXISTS (
                SELECT 1 FROM ple_private.select_ready_question_image_renditions(
                    p_published_question_id, p_revision_number) AS ready
                 WHERE ready.source_relative_name = supplied.source_relative_name
                   AND ready.question_image_asset_id = supplied.question_image_asset_id
                   AND ready.question_image_checksum = supplied.question_image_checksum
            )
    ) THEN
        RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Disclosed support images are invalid';
    END IF;
    IF jsonb_array_length(p_images) = 0 THEN RETURN; END IF;
    INSERT INTO ple_private.question_attempt_presentation_asset_binding(course_instance_id, question_attempt_id)
    VALUES (course_instance_id_value, question_attempt_id_value) ON CONFLICT DO NOTHING;
    INSERT INTO ple_private.question_attempt_presentation_image_rendition(
        course_instance_id, question_attempt_id, question_image_asset_id,
        question_image_checksum, rendition_checksum, intrinsic_width, intrinsic_height, is_disclosed_support)
    SELECT DISTINCT course_instance_id_value, question_attempt_id_value, ready.question_image_asset_id,
           decode(ready.question_image_checksum, 'hex'), decode(ready.rendition_checksum, 'hex'),
           ready.intrinsic_width, ready.intrinsic_height, true
      FROM jsonb_to_recordset(p_images) AS supplied(
          source_relative_name text, question_image_asset_id uuid, question_image_checksum text)
      JOIN ple_private.select_ready_question_image_renditions(
          p_published_question_id, p_revision_number) AS ready
        ON ready.source_relative_name = supplied.source_relative_name
       AND ready.question_image_asset_id = supplied.question_image_asset_id
       AND ready.question_image_checksum = supplied.question_image_checksum
    ON CONFLICT DO NOTHING;
END $$;

SET LOCAL ROLE ple_api_owner;

CREATE FUNCTION ple_api.register_student_assessment_attempt_support_images(
    p_assessment_attempt_id uuid, p_issued_position integer,
    p_published_question_id text, p_revision_number integer, p_images jsonb
) RETURNS void LANGUAGE sql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private AS $$
    SELECT ple_private.register_student_assessment_attempt_support_images(
        p_assessment_attempt_id, p_issued_position - 1, p_published_question_id, p_revision_number, p_images)
$$;
