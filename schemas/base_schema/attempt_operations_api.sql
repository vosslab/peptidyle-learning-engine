-- Assignment Attempt accommodation mutation and restricted API wrappers.
-- Core private Student Work operations are defined first by attempt_operations.sql.

-- Accommodations are current teaching configuration.  An Attempt copies its
-- effective values and sources at start, while this guarded CAS path remains
-- available for later Attempts.
CREATE FUNCTION ple_private.save_student_assignment_accommodation(
    p_accommodation_id uuid,
    p_student_record_id uuid,
    p_assignment_id uuid,
    p_expected_edit_number bigint,
    p_available_at timestamptz,
    p_due_at timestamptz,
    p_closes_at timestamptz,
    p_assignment_attempt_time_limit_seconds integer,
    p_attempt_limit integer
) RETURNS TABLE (accommodation_edit_number bigint)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
DECLARE current_row ple_private.student_assignment_accommodation%ROWTYPE;
DECLARE course_id_value uuid;
BEGIN
    IF p_accommodation_id IS NULL OR p_student_record_id IS NULL OR p_assignment_id IS NULL
       OR p_expected_edit_number IS NULL OR p_expected_edit_number < 0 THEN
        RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Student Assignment Accommodation save is invalid';
    END IF;
    SELECT assignment.course_id INTO course_id_value
      FROM ple_data.assignment AS assignment
     WHERE assignment.assignment_id = p_assignment_id;
    IF NOT FOUND
       OR NOT ple_data.student_assignment_has_course_scope(p_student_record_id, p_assignment_id)
       OR NOT ple_api.current_session_account_is_course_instructor(course_id_value) THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Student Assignment Accommodation is unavailable';
    END IF;
    -- Shares the Student Work root lock so a newly accepted current
    -- accommodation and an Attempt start observe one ordering.
    PERFORM ple_private.lock_assignment_for_student_work(p_assignment_id);
    SELECT * INTO current_row FROM ple_private.student_assignment_accommodation
     WHERE student_record_id = p_student_record_id AND assignment_id = p_assignment_id
     FOR UPDATE;
    IF NOT FOUND THEN
        IF p_expected_edit_number <> 0 THEN
            RAISE EXCEPTION USING ERRCODE = '40001', MESSAGE = 'Student Assignment Accommodation Edit Number is stale';
        END IF;
        INSERT INTO ple_private.student_assignment_accommodation(
            accommodation_id, student_record_id, assignment_id, available_at, due_at, closes_at,
            assignment_attempt_time_limit_seconds, attempt_limit, created_at
        ) VALUES (
            p_accommodation_id, p_student_record_id, p_assignment_id, p_available_at, p_due_at,
            p_closes_at, p_assignment_attempt_time_limit_seconds, p_attempt_limit,
            pg_catalog.transaction_timestamp()
        ) RETURNING ple_private.student_assignment_accommodation.accommodation_edit_number
          INTO accommodation_edit_number;
        RETURN NEXT;
        RETURN;
    END IF;
    IF current_row.accommodation_id <> p_accommodation_id THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Student Assignment Accommodation identity is unavailable';
    END IF;
    IF current_row.accommodation_edit_number <> p_expected_edit_number THEN
        RAISE EXCEPTION USING ERRCODE = '40001', MESSAGE = 'Student Assignment Accommodation Edit Number is stale';
    END IF;
    IF ROW(current_row.available_at, current_row.due_at, current_row.closes_at,
           current_row.assignment_attempt_time_limit_seconds, current_row.attempt_limit)
       IS NOT DISTINCT FROM ROW(p_available_at, p_due_at, p_closes_at,
                                p_assignment_attempt_time_limit_seconds, p_attempt_limit) THEN
        accommodation_edit_number := current_row.accommodation_edit_number;
        RETURN NEXT;
        RETURN;
    END IF;
    UPDATE ple_private.student_assignment_accommodation
       SET available_at = p_available_at,
           due_at = p_due_at,
           closes_at = p_closes_at,
           assignment_attempt_time_limit_seconds = p_assignment_attempt_time_limit_seconds,
           attempt_limit = p_attempt_limit,
           accommodation_edit_number = current_row.accommodation_edit_number + 1
     WHERE accommodation_id = p_accommodation_id
 RETURNING ple_private.student_assignment_accommodation.accommodation_edit_number
      INTO accommodation_edit_number;
    RETURN NEXT;
END $$;

REVOKE ALL ON FUNCTION ple_private.lock_assignment_for_student_work(uuid),
    ple_private.assert_current_student_attempt(bigint),
    ple_private.start_assignment_attempt(uuid, uuid, uuid, jsonb, jsonb),
    ple_private.prepare_current_assignment_attempt_start(bigint, bigint),
    ple_private.read_started_student_assignment_attempt(uuid),
    ple_private.prepare_assignment_attempt_finalization(uuid),
    ple_private.prepare_student_assignment_attempt_finalization(bigint),
    ple_private.commit_student_assignment_attempt_finalization(bigint, text, jsonb),
    ple_private.commit_assignment_attempt_finalization(uuid, text, jsonb, uuid),
    ple_private.prepare_expired_student_assignment_attempt_finalizations(integer),
    ple_private.commit_expired_student_assignment_attempt_finalization(uuid, jsonb),
    ple_private.save_student_assignment_attempt_response(bigint, integer, jsonb),
    ple_private.read_student_assignment_attempt_progress(bigint),
    ple_private.read_student_assignment_attempt_saved_response(bigint, integer),
    ple_private.lock_question_attempt_for_grading(uuid),
    ple_private.read_student_assignment_attempt_history_evidence(bigint),
    ple_private.save_student_assignment_accommodation(uuid, uuid, uuid, bigint, timestamptz, timestamptz, timestamptz, integer, integer) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_private.lock_assignment_for_student_work(uuid),
    ple_private.start_assignment_attempt(uuid, uuid, uuid, jsonb, jsonb),
    ple_private.prepare_current_assignment_attempt_start(bigint, bigint),
    ple_private.read_started_student_assignment_attempt(uuid),
    ple_private.save_student_assignment_attempt_response(bigint, integer, jsonb),
    ple_private.prepare_student_assignment_attempt_finalization(bigint),
    ple_private.commit_student_assignment_attempt_finalization(bigint, text, jsonb),
    ple_private.prepare_expired_student_assignment_attempt_finalizations(integer),
    ple_private.commit_expired_student_assignment_attempt_finalization(uuid, jsonb),
    ple_private.read_student_assignment_attempt_progress(bigint),
    ple_private.read_student_assignment_attempt_saved_response(bigint, integer),
    ple_private.read_student_assignment_attempt_history_evidence(bigint),
    ple_private.save_student_assignment_accommodation(uuid, uuid, uuid, bigint, timestamptz, timestamptz, timestamptz, integer, integer) TO ple_api_owner;
RESET ROLE;

SET LOCAL ROLE ple_api_owner;
CREATE FUNCTION ple_api.start_assignment_attempt(uuid, uuid, uuid, jsonb, jsonb)
RETURNS TABLE (assignment_attempt_id uuid, attempt_number integer, resumed boolean)
LANGUAGE sql SECURITY DEFINER SET search_path = pg_catalog, ple_private, ple_api AS $$
    SELECT * FROM ple_private.start_assignment_attempt($1, $2, $3, $4, $5)
$$;
CREATE FUNCTION ple_api.prepare_current_assignment_attempt_start(bigint, bigint)
RETURNS TABLE (
    student_record_id uuid, assignment_id uuid, assignment_entry_id uuid,
    entry_kind text, authored_position integer, fixed_question_id text,
    fixed_revision_number integer, question_pool_item_id uuid,
    pool_question_id text, pool_revision_number integer, selection_count integer,
    pool_selection_rule text, question_pool_reuse_rule text,
    question_variation_rule text
)
LANGUAGE sql SECURITY DEFINER SET search_path = pg_catalog, ple_private, ple_api AS $$
    SELECT * FROM ple_private.prepare_current_assignment_attempt_start($1, $2)
$$;
CREATE FUNCTION ple_api.read_started_student_assignment_attempt(uuid)
RETURNS TABLE (
    assignment_attempt_reference_number bigint, course_reference_number bigint,
    assignment_reference_number bigint, attempt_number integer,
    assignment_title text, assignment_instructions text
)
LANGUAGE sql SECURITY DEFINER SET search_path = pg_catalog, ple_private, ple_api AS $$
    SELECT * FROM ple_private.read_started_student_assignment_attempt($1)
$$;
CREATE FUNCTION ple_api.save_student_assignment_attempt_response(bigint, integer, jsonb)
RETURNS TABLE (assignment_attempt_reference_number bigint, issued_position integer, response_state text)
LANGUAGE sql SECURITY DEFINER SET search_path = pg_catalog, ple_private, ple_api AS $$
    SELECT assignment_attempt_reference_number, issued_position + 1, response_state
      FROM ple_private.save_student_assignment_attempt_response($1, $2 - 1, $3)
$$;
CREATE FUNCTION ple_api.prepare_student_assignment_attempt_finalization(bigint)
RETURNS TABLE (
    preparation_state text, finalization_kind text, missing_positions integer[],
    points_earned double precision, points_possible double precision,
    question_attempt_id uuid, saved_at_millis bigint, question_id text,
    revision_number integer, source_object_id uuid, source_object_checksum text,
    question_seed numeric, student_response jsonb, backend text, webwork_pg_path text
)
LANGUAGE sql SECURITY DEFINER SET search_path = pg_catalog, ple_private, ple_api AS $$
    SELECT * FROM ple_private.prepare_student_assignment_attempt_finalization($1)
$$;
CREATE FUNCTION ple_api.commit_student_assignment_attempt_finalization(bigint, text, jsonb)
RETURNS TABLE (points_earned double precision, points_possible double precision)
LANGUAGE sql SECURITY DEFINER SET search_path = pg_catalog, ple_private, ple_api AS $$
    SELECT * FROM ple_private.commit_student_assignment_attempt_finalization($1, $2, $3)
$$;
CREATE FUNCTION ple_api.prepare_expired_student_assignment_attempt_finalizations(integer)
RETURNS TABLE (
    assignment_attempt_id uuid, question_attempt_id uuid, saved_at_millis bigint,
    question_id text, revision_number integer, source_object_id uuid,
    source_object_checksum text, question_seed numeric, student_response jsonb,
    backend text, webwork_pg_path text
)
LANGUAGE sql SECURITY DEFINER SET search_path = pg_catalog, ple_private, ple_api AS $$
    SELECT * FROM ple_private.prepare_expired_student_assignment_attempt_finalizations($1)
$$;
CREATE FUNCTION ple_api.commit_expired_student_assignment_attempt_finalization(uuid, jsonb)
RETURNS TABLE (points_earned double precision, points_possible double precision)
LANGUAGE sql SECURITY DEFINER SET search_path = pg_catalog, ple_private, ple_api AS $$
    SELECT * FROM ple_private.commit_expired_student_assignment_attempt_finalization($1, $2)
$$;
CREATE FUNCTION ple_api.read_student_assignment_attempt_progress(bigint)
RETURNS TABLE (assignment_attempt_reference_number bigint, question_count integer,
    recommended_position integer, issued_position integer, response_state text)
LANGUAGE sql SECURITY DEFINER SET search_path = pg_catalog, ple_private, ple_api AS $$
    SELECT assignment_attempt_reference_number, question_count,
           recommended_position + 1, issued_position + 1, response_state
      FROM ple_private.read_student_assignment_attempt_progress($1)
$$;
CREATE FUNCTION ple_api.read_student_assignment_attempt_saved_response(bigint, integer)
RETURNS TABLE (issued_position integer, student_response jsonb)
LANGUAGE sql SECURITY DEFINER SET search_path = pg_catalog, ple_private, ple_api AS $$
    SELECT issued_position + 1, student_response
      FROM ple_private.read_student_assignment_attempt_saved_response($1, $2 - 1)
$$;
CREATE FUNCTION ple_api.read_student_assignment_attempt_history_evidence(bigint)
RETURNS TABLE (assignment_attempt_reference_number bigint, assignment_title text, assignment_instructions text,
    issued_position integer, question_id text, revision_number integer, question_seed numeric,
    question_attempt_limit integer, question_attempt_time_limit_seconds integer,
    question_attempt_grace_seconds integer, question_attempt_state text, student_response jsonb)
LANGUAGE sql SECURITY DEFINER SET search_path = pg_catalog, ple_private, ple_api AS $$
    SELECT assignment_attempt_reference_number, assignment_title, assignment_instructions,
           issued_position + 1, question_id, revision_number, question_seed,
           question_attempt_limit, question_attempt_time_limit_seconds,
           question_attempt_grace_seconds, question_attempt_state, student_response
      FROM ple_private.read_student_assignment_attempt_history_evidence($1)
$$;
CREATE FUNCTION ple_api.save_student_assignment_accommodation(
    uuid, uuid, uuid, bigint, timestamptz, timestamptz, timestamptz, integer, integer
) RETURNS TABLE (accommodation_edit_number bigint)
LANGUAGE sql SECURITY DEFINER SET search_path = pg_catalog, ple_private, ple_api AS $$
    SELECT * FROM ple_private.save_student_assignment_accommodation($1, $2, $3, $4, $5, $6, $7, $8, $9)
$$;
REVOKE ALL ON FUNCTION ple_api.start_assignment_attempt(uuid, uuid, uuid, jsonb, jsonb),
    ple_api.prepare_current_assignment_attempt_start(bigint, bigint),
    ple_api.read_started_student_assignment_attempt(uuid),
    ple_api.save_student_assignment_attempt_response(bigint, integer, jsonb),
    ple_api.prepare_student_assignment_attempt_finalization(bigint),
    ple_api.commit_student_assignment_attempt_finalization(bigint, text, jsonb),
    ple_api.prepare_expired_student_assignment_attempt_finalizations(integer),
    ple_api.commit_expired_student_assignment_attempt_finalization(uuid, jsonb),
    ple_api.read_student_assignment_attempt_progress(bigint),
    ple_api.read_student_assignment_attempt_saved_response(bigint, integer),
    ple_api.read_student_assignment_attempt_history_evidence(bigint),
    ple_api.save_student_assignment_accommodation(uuid, uuid, uuid, bigint, timestamptz, timestamptz, timestamptz, integer, integer)
    FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_api.start_assignment_attempt(uuid, uuid, uuid, jsonb, jsonb),
    ple_api.prepare_current_assignment_attempt_start(bigint, bigint),
    ple_api.read_started_student_assignment_attempt(uuid),
    ple_api.save_student_assignment_attempt_response(bigint, integer, jsonb),
    ple_api.prepare_student_assignment_attempt_finalization(bigint),
    ple_api.commit_student_assignment_attempt_finalization(bigint, text, jsonb),
    ple_api.read_student_assignment_attempt_progress(bigint),
    ple_api.read_student_assignment_attempt_saved_response(bigint, integer),
    ple_api.read_student_assignment_attempt_history_evidence(bigint),
    ple_api.save_student_assignment_accommodation(uuid, uuid, uuid, bigint, timestamptz, timestamptz, timestamptz, integer, integer) TO ple_app;
GRANT USAGE ON SCHEMA ple_api TO ple_assignment_attempt_expiry_worker;
GRANT EXECUTE ON FUNCTION ple_api.prepare_expired_student_assignment_attempt_finalizations(integer),
    ple_api.commit_expired_student_assignment_attempt_finalization(uuid, jsonb)
    TO ple_assignment_attempt_expiry_worker;
RESET ROLE;
