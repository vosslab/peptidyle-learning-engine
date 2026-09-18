-- Functions, triggers, and views from assessment_creation.sql.

SET LOCAL ROLE ple_data_owner;

-- Manual Course Assessment creation is a direct-origin operation. Blueprint
-- adoption owns the only trusted path that records exact Blueprint provenance.
CREATE FUNCTION ple_data.create_assessment(
    p_assessment_id text,
    p_course_reference_number text,
    p_assessment_type text,
    p_title text,
    p_instructions text
) RETURNS TABLE (
    assessment_public_reference text, assessment_edit_number bigint,
    assessment_status text, assessment_title text, assessment_instructions text
)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
DECLARE
    course_row ple_data.course_instance%ROWTYPE;
    snapshot_id ple_data.sha256_digest;
BEGIN
    -- ASVS 2.2.1-2.2.3 and 8.2.1-8.2.2: validate the closed creation shape
    -- and derive both Course authorization and direct origin in this trusted boundary.
    IF p_assessment_id IS NULL
       OR p_assessment_type IS NULL OR p_assessment_type NOT IN (
           'regular_assignment', 'practice_question_assignment', 'bonus_assignment', 'quiz', 'exam'
       )
       OR p_course_reference_number IS NULL
       OR p_title IS NULL OR p_instructions IS NULL THEN
        RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Assessment creation is invalid';
    END IF;
    SELECT * INTO course_row FROM ple_data.course_instance
     WHERE course_instance_id = p_course_reference_number;
    IF NOT FOUND OR NOT ple_api.current_session_account_is_course_instructor(course_row.course_instance_id) THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Assessment is unavailable';
    END IF;
    snapshot_id := ple_private.ensure_assessment_policy_snapshot(
        p_title, p_instructions, NULL, NULL, NULL, NULL,
        CASE WHEN p_assessment_type IN ('quiz', 'exam') THEN 1 ELSE NULL END,
        'reject', 'new_variation', 'shuffled',
        'after_submit', 'after_submit', 'after_submit',
        CASE WHEN p_assessment_type IN ('practice_question_assignment', 'quiz', 'exam')
             THEN 'after_submit' ELSE 'never' END,
        CASE WHEN p_assessment_type IN ('quiz', 'exam')
             THEN 'after_submit' ELSE 'never' END,
        'never',
        p_assessment_type::ple_data.assessment_type
    );
    INSERT INTO ple_data.assessment AS inserted (
        assessment_id, course_instance_id, origin_kind,
        source_blueprint_course_id, source_blueprint_revision_number,
        source_blueprint_assessment_reference,
        created_at, updated_at, assessment_type, assessment_policy_snapshot_id
    ) VALUES (
        p_assessment_id, course_row.course_instance_id, 'direct', NULL, NULL, NULL,
        clock_timestamp(), clock_timestamp(), p_assessment_type, snapshot_id
    ) RETURNING inserted.assessment_id, inserted.assessment_edit_number,
        inserted.assessment_status
      INTO assessment_public_reference, assessment_edit_number, assessment_status;
    assessment_title := p_title;
    assessment_instructions := p_instructions;
    RETURN NEXT;
END
$$;

