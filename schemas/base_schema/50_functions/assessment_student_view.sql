-- Functions, triggers, and views from assessment_student_view.sql.

SET LOCAL ROLE ple_private_owner;

-- Read-only Instructor Student View source authorization.
--
-- The Assessment manifest reuses the existing authorized workspace projection.
-- This function is the narrower second read: it rechecks the current Assessment
-- Edit Number and one exact fixed pin or exact Assessment-owned Pool member.
CREATE FUNCTION ple_private.load_instructor_student_view_source_binding(
    p_published_question_id text,
    p_question_revision_number integer
) RETURNS TABLE (
    backend text,
    source_object_record_id uuid,
    source_object_checksum text,
    source_media_type text,
    webwork_pg_path text
)
LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_private AS $$
    SELECT source.backend,
           source.source_object_record_id,
           source.source_object_checksum,
           object_record.media_type,
           source.webwork_pg_path
      FROM ple_private.question_revision_source_binding AS source
      JOIN ple_private.object_record AS object_record
        ON object_record.object_record_id = source.source_object_record_id
       AND encode(object_record.sha256, 'hex') = source.source_object_checksum
     WHERE source.published_question_id = p_published_question_id
       AND source.revision_number = p_question_revision_number
$$;

SET LOCAL ROLE ple_api_owner;






-- ASVS 8.3.1: the answer-free preview resolves the same finite base as start,
-- only after current Course Instructor authorization; workspace keeps authored NULL.
CREATE FUNCTION ple_api.read_instructor_student_view_duration_seconds(
    p_course_public_reference text, p_assessment_public_reference text
) RETURNS integer LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data AS $$
    SELECT ple_data.assessment_effective_base_duration_seconds(assessment.assessment_id)
      FROM ple_data.assessment AS assessment
      JOIN ple_data.course_instance AS course ON course.course_instance_id = assessment.course_instance_id
     WHERE course.course_instance_id = p_course_public_reference
       AND assessment.assessment_id = p_assessment_public_reference
       AND ple_api.current_session_account_is_course_instructor(course.course_instance_id)
$$;

CREATE FUNCTION ple_api.load_instructor_student_view_question_source(
    p_course_public_reference text,
    p_assessment_public_reference text,
    p_expected_assessment_edit_number bigint,
    p_authored_position integer,
    p_published_question_id text,
    p_question_revision_number integer
) RETURNS TABLE (
    backend text,
    source_object_record_id uuid,
    source_object_checksum text,
    source_media_type text,
    webwork_pg_path text,
    question_asset_renditions jsonb
)
LANGUAGE plpgsql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
DECLARE
    assessment_row ple_data.assessment%ROWTYPE;
BEGIN
    -- ASVS 2.2.1 and 8.2.2: reject invalid locator combinations before
    -- resolving only the exact Course-owned Assessment authorized below.
    IF p_expected_assessment_edit_number IS NULL
       OR p_expected_assessment_edit_number <= 0
       OR p_authored_position IS NULL
       OR p_authored_position < 0
       OR p_published_question_id IS NULL
       OR p_question_revision_number IS NULL
       OR p_question_revision_number <= 0 THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Instructor Student View source locator is invalid';
    END IF;

    -- ASVS 8.2.1-8.2.3 and 8.3.1: the current authenticated Account must
    -- have a direct Instructor relationship to this exact Course Instance.
    SELECT assessment.* INTO assessment_row
      FROM ple_data.course_instance AS course
      JOIN ple_data.assessment AS assessment
        ON assessment.course_instance_id = course.course_instance_id
     WHERE course.course_instance_id = p_course_public_reference
       AND assessment.assessment_id = p_assessment_public_reference
       AND ple_api.current_session_account_is_course_instructor(course.course_instance_id);
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Instructor Student View source is unavailable';
    END IF;
    IF assessment_row.assessment_edit_number IS DISTINCT FROM
       p_expected_assessment_edit_number THEN
        RAISE EXCEPTION USING ERRCODE = '40001',
            MESSAGE = 'Instructor Student View Assessment Edit Number is stale';
    END IF;

    -- ASVS 1.2.4 and 2.3.1: use typed equality predicates for the complete
    -- locator. A caller cannot substitute another position, Pool Revision,
    -- member, Question ID, or Question Revision.
    IF NOT EXISTS (
        SELECT 1
         FROM ple_data.assessment_entry AS entry
         WHERE entry.assessment_id = assessment_row.assessment_id
           AND entry.authored_position = p_authored_position
           AND entry.availability = 'available'
           AND (
               (entry.entry_kind = 'fixed_question'
                AND EXISTS (
                    SELECT 1
                      FROM ple_data.assessment_entry_question AS question
                     WHERE question.assessment_entry_id = entry.assessment_entry_id
                       AND question.published_question_id = p_published_question_id
                       AND question.question_revision_number = p_question_revision_number
                ))
               OR
               (entry.entry_kind = 'question_pool'
                AND EXISTS (
                    SELECT 1
                      FROM ple_data.assessment_entry_pool AS pool_entry
                      JOIN ple_data.question_pool_member AS member
                        ON member.question_pool_id = pool_entry.question_pool_id
                     WHERE pool_entry.assessment_entry_id = entry.assessment_entry_id
                       AND member.published_question_id = p_published_question_id
                       AND member.question_revision_number = p_question_revision_number
                ))
           )
    ) THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Instructor Student View source is unavailable';
    END IF;

    RETURN QUERY
    SELECT source.backend,
           source.source_object_record_id,
           source.source_object_checksum,
           source.source_media_type,
           source.webwork_pg_path,
           COALESCE((
               SELECT jsonb_agg(
                   jsonb_build_object(
                       'asset_id', rendition.asset_id,
                       'question_asset_checksum', rendition.question_asset_checksum,
                       'rendition_checksum', rendition.rendition_checksum,
                       'intrinsic_width', rendition.intrinsic_width,
                       'intrinsic_height', rendition.intrinsic_height
                   ) ORDER BY rendition.asset_id
               )
                 FROM ple_api.select_ready_question_asset_renditions(
                     p_published_question_id, p_question_revision_number
                 ) AS rendition
           ), '[]'::jsonb)
      FROM ple_private.load_instructor_student_view_source_binding(
          p_published_question_id, p_question_revision_number
      ) AS source;

    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Instructor Student View Question source is invalid';
    END IF;
END
$$;

