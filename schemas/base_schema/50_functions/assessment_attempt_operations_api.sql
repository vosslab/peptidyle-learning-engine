-- Functions, triggers, and views from assessment_attempt_operations_api.sql.

-- Assessment Attempt accommodation mutation and restricted API wrappers.
-- Core private Student Work operations are defined first by assessment_attempt_operations.sql.

-- Accommodations are current teaching configuration.  An Assessment Attempt copies its
-- effective values and sources at start, while this guarded CAS path remains
-- available for later Assessment Attempts.
CREATE FUNCTION ple_private.save_student_assessment_accommodation(
    p_accommodation_id uuid,
    p_student_record_id uuid,
    p_assessment_id text,
    p_expected_edit_number bigint,
    p_available_at timestamptz,
    p_due_at timestamptz,
    p_closes_at timestamptz,
    p_time_multiplier numeric,
    p_assessment_attempt_limit integer
) RETURNS TABLE (accommodation_edit_number bigint)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
DECLARE current_row ple_private.student_assessment_accommodation%ROWTYPE;
DECLARE course_id_value text;
BEGIN
    IF p_accommodation_id IS NULL OR p_student_record_id IS NULL OR p_assessment_id IS NULL
       OR p_expected_edit_number IS NULL OR p_expected_edit_number < 0 THEN
        RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Student Assessment Accommodation save is invalid';
    END IF;
    SELECT assessment.course_instance_id INTO course_id_value
      FROM ple_data.assessment AS assessment
     WHERE assessment.assessment_id = p_assessment_id;
    IF NOT FOUND
       OR NOT ple_data.student_assessment_has_course_scope(p_student_record_id, p_assessment_id)
       OR NOT ple_api.current_session_account_is_course_instructor(course_id_value) THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Student Assessment Accommodation is unavailable';
    END IF;
    -- Shares the Student Work root lock so a newly accepted current
    -- accommodation and an Assessment Attempt start observe one ordering.
    PERFORM ple_private.lock_assessment_for_student_work(p_assessment_id);
    SELECT * INTO current_row FROM ple_private.student_assessment_accommodation
     WHERE student_record_id = p_student_record_id AND assessment_id = p_assessment_id
     FOR UPDATE;
    IF NOT FOUND THEN
        IF p_expected_edit_number <> 0 THEN
            RAISE EXCEPTION USING ERRCODE = '40001', MESSAGE = 'Student Assessment Accommodation Edit Number is stale';
        END IF;
        INSERT INTO ple_private.student_assessment_accommodation(
            accommodation_id, course_instance_id, student_record_id, assessment_id, available_at, due_at, closes_at,
            time_multiplier, assessment_attempt_limit, created_at
        ) VALUES (
            p_accommodation_id, course_id_value, p_student_record_id, p_assessment_id, p_available_at, p_due_at,
            p_closes_at, p_time_multiplier, p_assessment_attempt_limit,
            pg_catalog.transaction_timestamp()
        ) RETURNING ple_private.student_assessment_accommodation.accommodation_edit_number
          INTO accommodation_edit_number;
        RETURN NEXT;
        RETURN;
    END IF;
    IF current_row.accommodation_id <> p_accommodation_id THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Student Assessment Accommodation identity is unavailable';
    END IF;
    IF current_row.accommodation_edit_number <> p_expected_edit_number THEN
        RAISE EXCEPTION USING ERRCODE = '40001', MESSAGE = 'Student Assessment Accommodation Edit Number is stale';
    END IF;
    IF ROW(current_row.available_at, current_row.due_at, current_row.closes_at,
           current_row.time_multiplier, current_row.assessment_attempt_limit)
       IS NOT DISTINCT FROM ROW(p_available_at, p_due_at, p_closes_at,
                                p_time_multiplier, p_assessment_attempt_limit) THEN
        accommodation_edit_number := current_row.accommodation_edit_number;
        RETURN NEXT;
        RETURN;
    END IF;
    UPDATE ple_private.student_assessment_accommodation
       SET available_at = p_available_at,
           due_at = p_due_at,
           closes_at = p_closes_at,
           time_multiplier = p_time_multiplier,
           assessment_attempt_limit = p_assessment_attempt_limit,
           accommodation_edit_number = current_row.accommodation_edit_number + 1
     WHERE accommodation_id = p_accommodation_id
 RETURNING ple_private.student_assessment_accommodation.accommodation_edit_number
      INTO accommodation_edit_number;
    RETURN NEXT;
END $$;

SET LOCAL ROLE ple_api_owner;

CREATE FUNCTION ple_api.start_assessment_attempt(uuid, uuid, text, jsonb, jsonb)
RETURNS TABLE (assessment_attempt_id uuid, assessment_attempt_number integer, resumed boolean)
LANGUAGE sql SECURITY DEFINER SET search_path = pg_catalog, ple_private, ple_api AS $$
    SELECT * FROM ple_private.start_assessment_attempt($1, $2, $3, $4, $5)
$$;

CREATE FUNCTION ple_api.prepare_current_assessment_attempt_start_decision(text, text)
RETURNS TABLE (
    resumable_assessment_attempt_id uuid,
    resumable_assessment_attempt_number integer
)
LANGUAGE sql SECURITY DEFINER SET search_path = pg_catalog, ple_private, ple_api, ple_data AS $$
    SELECT * FROM ple_private.prepare_current_assessment_attempt_start_decision(
        (SELECT course.course_instance_id FROM ple_data.course_instance AS course
          WHERE course.course_instance_id = $1),
        $2
    )
$$;

CREATE FUNCTION ple_api.prepare_current_assessment_attempt_start(text, text)
RETURNS TABLE (
    student_record_id uuid, assessment_id text, assessment_entry_id uuid,
    entry_kind text, authored_position integer, fixed_question_id text,
    fixed_revision_number integer, question_pool_id text,
    question_pool_edit_number bigint, member_position integer,
    pool_question_id text, pool_question_revision_number integer, question_backend text,
    selection_count integer,
    pool_selection_rule text,
    question_variation_rule text, assessment_question_order_rule text
)
LANGUAGE sql SECURITY DEFINER SET search_path = pg_catalog, ple_private, ple_api, ple_data AS $$
    SELECT * FROM ple_private.prepare_current_assessment_attempt_start(
        (SELECT course.course_instance_id FROM ple_data.course_instance AS course
          WHERE course.course_instance_id = $1),
        $2
    )
$$;

CREATE FUNCTION ple_api.read_started_student_assessment_attempt(uuid)
RETURNS TABLE (
    assessment_attempt_id uuid, course_instance_id text,
    assessment_id text, assessment_attempt_number integer,
    assessment_title text, assessment_instructions text
)
LANGUAGE sql SECURITY DEFINER SET search_path = pg_catalog, ple_private, ple_api AS $$
    SELECT * FROM ple_private.read_started_student_assessment_attempt($1)
$$;

CREATE FUNCTION ple_api.save_student_assessment_attempt_response(uuid, integer, jsonb)
RETURNS TABLE (assessment_attempt_id uuid, issued_position integer, response_state text)
LANGUAGE sql SECURITY DEFINER SET search_path = pg_catalog, ple_private, ple_api AS $$
    SELECT assessment_attempt_id, issued_position + 1, response_state
      FROM ple_private.save_student_assessment_attempt_response($1, $2 - 1, $3)
$$;

CREATE FUNCTION ple_api.prepare_student_assessment_attempt_finalization(uuid)
RETURNS TABLE (
    preparation_state text, finalization_kind text,
    points_earned double precision, points_possible double precision,
    question_attempt_id uuid, saved_at_millis bigint, published_question_id text,
    revision_number integer, source_object_record_id uuid, source_object_checksum text,
    question_seed numeric, generated_parameter_sha256 text, student_response jsonb, backend text, webwork_pg_path text
)
LANGUAGE sql SECURITY DEFINER SET search_path = pg_catalog, ple_private, ple_api AS $$
    SELECT * FROM ple_private.prepare_student_assessment_attempt_finalization($1)
$$;

CREATE FUNCTION ple_api.commit_student_assessment_attempt_finalization(uuid, text, jsonb)
RETURNS TABLE (points_earned double precision, points_possible double precision)
LANGUAGE sql SECURITY DEFINER SET search_path = pg_catalog, ple_private, ple_api AS $$
    SELECT * FROM ple_private.commit_student_assessment_attempt_finalization($1, $2, $3)
$$;

CREATE FUNCTION ple_api.prepare_expired_student_assessment_attempt_finalizations(integer)
RETURNS TABLE (
    assessment_attempt_id uuid, question_attempt_id uuid, saved_at_millis bigint,
    published_question_id text, revision_number integer, source_object_record_id uuid,
    source_object_checksum text, question_seed numeric, generated_parameter_sha256 text, student_response jsonb,
    backend text, webwork_pg_path text
)
LANGUAGE sql SECURITY DEFINER SET search_path = pg_catalog, ple_private, ple_api AS $$
    SELECT * FROM ple_private.prepare_expired_student_assessment_attempt_finalizations($1)
$$;

CREATE FUNCTION ple_api.commit_expired_student_assessment_attempt_finalization(uuid, jsonb)
RETURNS TABLE (points_earned double precision, points_possible double precision)
LANGUAGE sql SECURITY DEFINER SET search_path = pg_catalog, ple_private, ple_api AS $$
    SELECT * FROM ple_private.commit_expired_student_assessment_attempt_finalization($1, $2)
$$;

CREATE FUNCTION ple_api.read_student_assessment_attempt_progress(uuid)
RETURNS TABLE (assessment_attempt_id uuid, question_count integer,
    recommended_position integer, issued_position integer, response_state text)
LANGUAGE sql SECURITY DEFINER SET search_path = pg_catalog, ple_private, ple_api AS $$
    SELECT assessment_attempt_id, question_count,
           recommended_position + 1, issued_position + 1, response_state
      FROM ple_private.read_student_assessment_attempt_progress($1)
$$;

CREATE FUNCTION ple_api.read_student_assessment_attempt_saved_response(uuid, integer)
RETURNS TABLE (issued_position integer, student_response jsonb)
LANGUAGE sql SECURITY DEFINER SET search_path = pg_catalog, ple_private, ple_api AS $$
    SELECT issued_position + 1, student_response
      FROM ple_private.read_student_assessment_attempt_saved_response($1, $2 - 1)
$$;

CREATE FUNCTION ple_api.read_student_assessment_attempt_history_evidence(uuid)
RETURNS TABLE (assessment_attempt_id uuid, assessment_title text, assessment_instructions text,
    issued_position integer, published_question_id text, revision_number integer, question_seed numeric,
    generated_parameter_sha256 text,
    question_attempt_limit integer, question_attempt_time_limit_seconds integer,
    question_attempt_grace_seconds integer, question_attempt_state text, student_response jsonb)
LANGUAGE sql SECURITY DEFINER SET search_path = pg_catalog, ple_private, ple_api AS $$
    SELECT assessment_attempt_id, assessment_title, assessment_instructions,
           issued_position + 1, published_question_id, revision_number, question_seed,
           generated_parameter_sha256,
           question_attempt_limit, question_attempt_time_limit_seconds,
           question_attempt_grace_seconds, question_attempt_state, student_response
      FROM ple_private.read_student_assessment_attempt_history_evidence($1)
$$;

CREATE FUNCTION ple_api.save_student_assessment_accommodation(
    uuid, uuid, text, bigint, timestamptz, timestamptz, timestamptz, numeric, integer
) RETURNS TABLE (accommodation_edit_number bigint)
LANGUAGE sql SECURITY DEFINER SET search_path = pg_catalog, ple_private, ple_api AS $$
    SELECT * FROM ple_private.save_student_assessment_accommodation($1, $2, $3, $4, $5, $6, $7, $8, $9)
$$;

