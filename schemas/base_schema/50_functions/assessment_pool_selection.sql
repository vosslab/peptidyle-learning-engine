-- Functions, triggers, and views from assessment_pool_selection.sql.

SET LOCAL ROLE ple_data_owner;

-- Count-only editing for one current Assessment-owned Question Pool entry.



-- ASVS 2.2.1, 2.3.3, 8.2.2, 8.2.3, 8.3.1, and 15.4.2: this
-- transaction-scoped command locks the Assessment root before validating its
-- Edit Number and exact owned Pool fork.  It changes only selection_count and
-- advances the parent Assessment Edit Number exactly once.
CREATE FUNCTION ple_data.update_assessment_question_pool_selection_count(
    p_assessment_id uuid,
    p_assessment_entry_id uuid,
    p_expected_assessment_edit_number bigint,
    p_selection_count integer
) RETURNS TABLE (
    assessment_entry_id uuid,
    selection_count integer,
    assessment_edit_number bigint
)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data AS $$
DECLARE assessment_row ple_data.assessment%ROWTYPE;
DECLARE entry_member_count integer;
BEGIN
    IF p_assessment_id IS NULL OR p_assessment_entry_id IS NULL
       OR p_expected_assessment_edit_number IS NULL
       OR p_expected_assessment_edit_number <= 0
       OR p_selection_count IS NULL OR p_selection_count <= 0 THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Assessment Question Pool selection count change is invalid';
    END IF;

    SELECT assessment.* INTO assessment_row
      FROM ple_data.assessment AS assessment
     WHERE assessment.assessment_id = p_assessment_id
       AND assessment.assessment_status = 'unreleased'
       AND ple_api.current_session_account_is_instructor()
       AND ple_api.current_session_account_is_course_instructor(assessment.course_id)
     FOR UPDATE;
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Assessment Question Pool selection count change is unavailable';
    END IF;

    IF assessment_row.assessment_edit_number
        IS DISTINCT FROM p_expected_assessment_edit_number THEN
        RAISE EXCEPTION USING ERRCODE = '40001',
            MESSAGE = 'Assessment Edit Number is stale';
    END IF;

    SELECT pool_revision.member_count INTO entry_member_count
      FROM ple_data.assessment_entry AS entry
      JOIN ple_data.assessment_question_pool_fork AS owned
        ON owned.assessment_entry_id = entry.assessment_entry_id
       AND owned.assessment_id = entry.assessment_id
       AND owned.question_pool_id = entry.question_pool_id
      JOIN ple_data.question_pool_revision AS pool_revision
        ON pool_revision.question_pool_id = entry.question_pool_id
       AND pool_revision.revision_number = entry.question_pool_revision_number
     WHERE entry.assessment_entry_id = p_assessment_entry_id
       AND entry.assessment_id = p_assessment_id
       AND entry.entry_kind = 'question_pool'
       AND entry.availability = 'available'
     FOR UPDATE OF entry;
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Assessment Question Pool selection count change is unavailable';
    END IF;

    IF p_selection_count > entry_member_count THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Assessment Question Pool selection exceeds fork member count';
    END IF;

    UPDATE ple_data.assessment_entry AS entry
       SET selection_count = p_selection_count
     WHERE entry.assessment_entry_id = p_assessment_entry_id
       AND entry.assessment_id = p_assessment_id;

    -- ASVS 2.2.2, 2.3.3: the count-only command cannot bypass the full-save bound.
    IF ple_data.assessment_delivered_question_count(p_assessment_id) > 250 THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Assessment may contain at most 250 Questions';
    END IF;

    UPDATE ple_data.assessment AS assessment
       SET assessment_edit_number = assessment.assessment_edit_number + 1,
           updated_at = pg_catalog.clock_timestamp()
     WHERE assessment.assessment_id = p_assessment_id
     RETURNING assessment.assessment_edit_number INTO assessment_edit_number;

    assessment_entry_id := p_assessment_entry_id;
    selection_count := p_selection_count;
    RETURN NEXT;
END
$$;

SET LOCAL ROLE ple_api_owner;





-- The application supplies only canonical Course/Assessment references, the
-- stable Entry UUID, the expected Assessment Edit Number, and a positive
-- count.  Pool identity, Pool Revision, and exact member count stay derived.
CREATE FUNCTION ple_api.update_assessment_question_pool_selection_count(
    p_course_reference text,
    p_assessment_reference text,
    p_assessment_entry_id uuid,
    p_expected_assessment_edit_number bigint,
    p_selection_count integer
) RETURNS TABLE (
    assessment_entry_id uuid,
    selection_count integer,
    assessment_edit_number bigint
)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data AS $$
DECLARE assessment_id_value uuid;
BEGIN
    -- ASVS 1.2.4 and 2.2.1: values remain typed parameters, and the public
    -- references use the same closed canonical shapes as their stored rows.
    IF NOT ple_private.is_canonical_prefixed_public_id(p_course_reference, 'CI')
       OR NOT ple_private.is_canonical_prefixed_public_id(p_assessment_reference, 'A') THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Assessment Question Pool selection count change is invalid';
    END IF;

    SELECT assessment.assessment_id INTO assessment_id_value
      FROM ple_data.course_instance AS course
      JOIN ple_data.assessment AS assessment ON assessment.course_id = course.course_id
     WHERE course.public_reference = p_course_reference
       AND assessment.public_reference = p_assessment_reference
       AND ple_api.current_session_account_is_instructor()
       AND ple_api.current_session_account_is_course_instructor(course.course_id);
    IF NOT FOUND THEN
        -- ASVS 16.5.1: one concealed outcome covers missing and unauthorized
        -- Course, Assessment, and session combinations.
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Assessment Question Pool selection count change is unavailable';
    END IF;

    RETURN QUERY
    SELECT * FROM ple_data.update_assessment_question_pool_selection_count(
        assessment_id_value,
        p_assessment_entry_id,
        p_expected_assessment_edit_number,
        p_selection_count
    );
END
$$;

