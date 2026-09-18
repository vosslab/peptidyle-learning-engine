-- Functions, triggers, and views from assessment_student_time_accommodation.sql.

SET LOCAL ROLE ple_private_owner;

-- Direct Instructor, Course-scoped per-Student time configuration.
CREATE FUNCTION ple_private.student_assessment_time_configuration(
    p_student_record_id uuid, p_assessment_id text, p_roster_id text,
    p_save boolean, p_expected_edit_number bigint, p_time_multiplier numeric
) RETURNS TABLE (
    roster_id text, time_multiplier double precision, accommodation_edit_number bigint,
    base_duration_seconds integer, effective_duration_seconds integer, capped_at_24_hours boolean
) LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_private, ple_data, ple_api AS $$
DECLARE current_row ple_private.student_assessment_accommodation%ROWTYPE;
DECLARE course_id_value text;
BEGIN
    SELECT assessment.course_instance_id INTO course_id_value FROM ple_data.assessment AS assessment
     WHERE assessment.assessment_id = p_assessment_id;
    -- ASVS 8.2.1-8.2.3, 8.3.1: authorization and exact Course/Student scope
    -- precede the private lookup; no private identity is projected.
    IF course_id_value IS NULL OR p_student_record_id IS NULL
       OR NOT ple_data.student_assessment_has_course_scope(p_student_record_id, p_assessment_id)
       OR NOT ple_api.current_session_account_is_course_instructor(course_id_value) THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Student time configuration is unavailable';
    END IF;
    PERFORM ple_private.lock_assessment_for_student_work(p_assessment_id);
    SELECT * INTO current_row FROM ple_private.student_assessment_accommodation AS accommodation
     WHERE accommodation.student_record_id = p_student_record_id
       AND accommodation.assessment_id = p_assessment_id FOR UPDATE;
    IF p_save THEN
        IF p_expected_edit_number IS NULL OR p_expected_edit_number < 0
           OR (p_time_multiplier IS NOT NULL
               AND (p_time_multiplier < 1 OR p_time_multiplier >= 'Infinity'::numeric)) THEN
            RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Student time configuration is invalid';
        END IF;
        IF COALESCE(current_row.accommodation_edit_number, 0) <> p_expected_edit_number THEN
            RAISE EXCEPTION USING ERRCODE = '40001', MESSAGE = 'Student time configuration Edit Number is stale';
        END IF;
        -- Standard time with no other configuration is a real no-op, not a
        -- manufactured accommodation record. Existing date/count fields stay.
        IF current_row.accommodation_id IS NOT NULL OR p_time_multiplier IS NOT NULL THEN
            PERFORM * FROM ple_private.save_student_assessment_accommodation(
                COALESCE(current_row.accommodation_id, pg_catalog.gen_random_uuid()),
                p_student_record_id, p_assessment_id, p_expected_edit_number,
                current_row.available_at, current_row.due_at, current_row.closes_at,
                p_time_multiplier, current_row.assessment_attempt_limit
            );
            SELECT * INTO current_row FROM ple_private.student_assessment_accommodation AS accommodation
             WHERE accommodation.student_record_id = p_student_record_id
               AND accommodation.assessment_id = p_assessment_id;
        END IF;
    END IF;
    roster_id := p_roster_id;
    time_multiplier := current_row.time_multiplier::double precision;
    accommodation_edit_number := current_row.accommodation_edit_number;
    base_duration_seconds := ple_data.assessment_effective_base_duration_seconds(p_assessment_id);
    effective_duration_seconds := ple_private.assessment_effective_duration_seconds(
        p_assessment_id, current_row.time_multiplier
    );
    capped_at_24_hours := COALESCE(
        current_row.time_multiplier > 86400::numeric / base_duration_seconds, false
    );
    RETURN NEXT;
END $$;

SET LOCAL ROLE ple_api_owner;

CREATE FUNCTION ple_api.student_assessment_time_configuration(
    p_course text, p_assessment text, p_roster_id text,
    p_save boolean, p_expected_edit_number bigint, p_time_multiplier numeric
) RETURNS TABLE (
    roster_id text, time_multiplier double precision, accommodation_edit_number bigint,
    base_duration_seconds integer, effective_duration_seconds integer, capped_at_24_hours boolean
) LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_private, ple_data, ple_api AS $$
DECLARE assessment_id_value text;
DECLARE student_record_id_value uuid;
BEGIN
    IF p_roster_id IS NULL OR char_length(p_roster_id) NOT BETWEEN 1 AND 64
       OR p_roster_id !~ '^[A-Za-z0-9._-]+$' THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Student time configuration is unavailable';
    END IF;
    SELECT assessment.assessment_id, membership.student_record_id
      INTO assessment_id_value, student_record_id_value
      FROM ple_data.course_instance AS course
      JOIN ple_data.assessment AS assessment ON assessment.course_instance_id = course.course_instance_id
      JOIN ple_private.course_roster_profile AS profile ON profile.course_instance_id = course.course_instance_id
      JOIN ple_data.course_membership AS membership
        ON membership.course_instance_id = course.course_instance_id
       AND membership.account_id = profile.student_account_id AND membership.role = 'student'
     WHERE course.course_instance_id = p_course AND assessment.assessment_id = p_assessment
       AND profile.roster_id = p_roster_id
       AND ple_data.course_membership_is_active(membership.course_membership_id)
       AND ple_api.current_session_account_is_course_instructor(course.course_instance_id);
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Student time configuration is unavailable';
    END IF;
    RETURN QUERY SELECT * FROM ple_private.student_assessment_time_configuration(
        student_record_id_value, assessment_id_value, p_roster_id,
        p_save, p_expected_edit_number, p_time_multiplier
    );
END $$;

