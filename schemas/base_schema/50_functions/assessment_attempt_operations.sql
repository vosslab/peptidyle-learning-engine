-- Functions, triggers, and views from assessment_attempt_operations.sql.

SET LOCAL ROLE ple_private_owner;

-- Assessment-first Student Work operations.  Delivery owns creation of a
-- Question Attempt's backend-specific reproduction bundle; this module owns
-- current Assessment selection, response persistence, and finalization.



-- The same finite calculation serves landing, access, start, and Instructor
-- previews. Apply the Student multiplier only after the authored/default base.
CREATE FUNCTION ple_private.assessment_effective_duration_seconds(
    p_assessment_id text, p_time_multiplier numeric
) RETURNS integer LANGUAGE plpgsql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_data, ple_private AS $$
DECLARE base_seconds integer;
DECLARE multiplier numeric := COALESCE(p_time_multiplier, 1);
BEGIN
    -- ASVS 2.2.1: NaN, infinities, and shortening multipliers are invalid.
    IF multiplier < 1 OR multiplier >= 'Infinity'::numeric THEN
        RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Student time multiplier is invalid';
    END IF;
    base_seconds := ple_data.assessment_effective_base_duration_seconds(p_assessment_id);
    IF base_seconds IS NULL THEN RETURN NULL; END IF;
    -- Test before multiplying: even the largest finite multiplier cannot
    -- overflow an intermediate product. Fractional seconds round upward.
    IF multiplier >= 86400::numeric / base_seconds THEN RETURN 86400; END IF;
    RETURN least(86400, ceil(base_seconds * multiplier)::integer);
END $$;

CREATE FUNCTION ple_private.lock_assessment_for_student_work(p_assessment_id text)
RETURNS void LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_data, ple_private AS $$
BEGIN
    PERFORM 1 FROM ple_data.assessment
     WHERE assessment_id = p_assessment_id FOR UPDATE;
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Assessment Student Work operation is unavailable';
    END IF;
END $$;

CREATE FUNCTION ple_private.assert_current_student_assessment_attempt(
    p_assessment_attempt_id uuid
) RETURNS ple_private.assessment_attempt LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
DECLARE result ple_private.assessment_attempt%ROWTYPE;
DECLARE course_id_value text;
BEGIN
    SELECT assessment_attempt.* INTO result
      FROM ple_private.assessment_attempt AS assessment_attempt
     WHERE assessment_attempt.assessment_attempt_id = p_assessment_attempt_id;
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Assessment Attempt is unavailable';
    END IF;
    -- Normal Student Work ends at the Course retention boundary.  Archived
    -- evidence remains reachable only through the separate retention-executor
    -- capability; an owned Assessment Attempt reference is never an ordinary read or
    -- mutator bypass after archive or deletion.  Keep this beside the
    -- ownership assertion because every current-Assessment Attempt operation shares it.
    SELECT assessment.course_instance_id INTO course_id_value
      FROM ple_data.assessment AS assessment
     WHERE assessment.assessment_id = result.assessment_id;
    IF NOT FOUND
       OR NOT ple_api.course_student_work_is_ordinarily_visible(course_id_value)
       OR NOT ple_api.current_session_account_owns_student_record(
           course_id_value, result.student_record_id
       ) THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Assessment Attempt is unavailable';
    END IF;
    RETURN result;
END $$;



-- Assessment Attempt start functions live in assessment_attempt_start.sql.

SET LOCAL ROLE ple_api_owner;



-- Course membership owns the current Student lookup.  This helper remains
-- executable only by the private Assessment Attempt boundary, so application sessions
-- cannot enumerate Student records.  The public ownership predicate continues
-- to decide whether the resulting record is usable for Student Work.
CREATE FUNCTION ple_api.current_session_student_record_id(p_course_instance_id text)
RETURNS uuid LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data AS $$
    SELECT student.student_record_id
      FROM ple_data.student_record AS student
      JOIN ple_data.course_membership AS membership
        ON membership.student_record_id = student.student_record_id
       AND membership.course_instance_id = student.course_instance_id
       AND membership.account_id = student.student_account_id
       AND membership.role = 'student'
       AND ple_data.course_membership_is_active(membership.course_membership_id)
     WHERE student.course_instance_id = p_course_instance_id
       AND student.student_account_id = ple_api.current_session_account_id()
$$;



-- Private Assessment Attempt readers need stable Course route/display facts but do not
-- receive direct access to the Course relation.  These API-owner functions
-- are executable only by that trusted private boundary.
CREATE FUNCTION ple_api.course_instance_id_for_assessment_attempt(p_course_instance_id text)
RETURNS text LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_data AS $$
    SELECT course.course_instance_id
      FROM ple_data.course_instance AS course
     WHERE course.course_instance_id = p_course_instance_id
$$;

CREATE FUNCTION ple_api.course_display_for_assessment_attempt(p_course_instance_id text)
RETURNS TABLE (
    course_instance_id text,
    course_short_name text,
    course_long_name text,
    course_theme text
) LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_data AS $$
    SELECT course.course_instance_id::text,
           course.course_short_name,
           course.course_long_name,
           course.course_theme_id
      FROM ple_data.course_instance AS course
     WHERE course.course_instance_id = p_course_instance_id
$$;

SET LOCAL ROLE ple_private_owner;




-- The start response reads its title and instructions from the retained
-- Assessment Attempt evidence. Course and Assessment public references are stable
-- route identities, while authored content is never re-read from mutable
-- Assessment configuration after an Assessment Attempt exists.
CREATE FUNCTION ple_private.read_started_student_assessment_attempt(
    p_assessment_attempt_id uuid
) RETURNS TABLE (
    assessment_attempt_id uuid,
    course_instance_id text,
    assessment_id text,
    assessment_attempt_number integer,
    assessment_title text,
    assessment_instructions text
)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
BEGIN
    IF p_assessment_attempt_id IS NULL THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Assessment Attempt is unavailable';
    END IF;
    RETURN QUERY
    SELECT assessment_attempt.assessment_attempt_id,
           course.course_instance_id::text,
           assessment.assessment_id::text,
           assessment_attempt.assessment_attempt_number,
           policy.assessment_title,
           policy.assessment_instructions
      FROM ple_private.assessment_attempt AS assessment_attempt
      JOIN ple_data.assessment AS assessment ON assessment.assessment_id = assessment_attempt.assessment_id
      JOIN ple_data.assessment_policy_snapshot AS policy
        ON policy.assessment_policy_snapshot_id = assessment_attempt.assessment_policy_snapshot_id
      JOIN LATERAL ple_api.course_display_for_assessment_attempt(assessment.course_instance_id) AS course ON true
     WHERE assessment_attempt.assessment_attempt_id = p_assessment_attempt_id
       AND ple_api.current_session_account_owns_student_record(
           assessment.course_instance_id, assessment_attempt.student_record_id
       );
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Assessment Attempt is unavailable';
    END IF;
END $$;

CREATE FUNCTION ple_private.save_student_assessment_attempt_response(
    p_assessment_attempt_id uuid, p_issued_position integer, p_student_response jsonb
) RETURNS TABLE (assessment_attempt_id uuid, issued_position integer, response_state text)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
DECLARE assessment_attempt_row ple_private.assessment_attempt%ROWTYPE;
DECLARE question_attempt_id_value uuid;
DECLARE now_value timestamptz;
BEGIN
    assessment_attempt_row := ple_private.assert_current_student_assessment_attempt(
        p_assessment_attempt_id
    );
    -- Match finalization's Assessment -> Assessment Attempt -> Question Attempt
    -- lock order. A pre-expiry save therefore commits before a worker can
    -- finalize this Attempt, and the worker must observe the saved response.
    -- ASVS 2.3.3, 2.3.4: accepted Student Work and deadline submission are
    -- serialized rather than resolved from competing snapshots.
    PERFORM ple_private.lock_assessment_for_student_work(assessment_attempt_row.assessment_id);
    SELECT * INTO assessment_attempt_row
      FROM ple_private.assessment_attempt AS assessment_attempt
     WHERE assessment_attempt.assessment_attempt_id = assessment_attempt_row.assessment_attempt_id
     FOR UPDATE;
    now_value := pg_catalog.clock_timestamp();
    IF EXISTS (
        SELECT 1 FROM ple_private.assessment_submission AS submission
         WHERE submission.assessment_attempt_id = assessment_attempt_row.assessment_attempt_id
    ) OR (assessment_attempt_row.expires_at IS NOT NULL AND now_value >= assessment_attempt_row.expires_at) THEN
        assessment_attempt_id := assessment_attempt_row.assessment_attempt_id;
        issued_position := p_issued_position;
        response_state := 'expired';
        RETURN NEXT;
        RETURN;
    END IF;
    IF p_issued_position < 0 OR jsonb_typeof(p_student_response) <> 'object' THEN
        RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Student response arguments are invalid';
    END IF;
    SELECT question_attempt.question_attempt_id INTO question_attempt_id_value
      FROM ple_private.issued_question AS issued
      JOIN ple_private.question_attempt ON question_attempt.issued_question_id = issued.issued_question_id
      JOIN ple_private.question_revision_source_binding AS source
        ON source.published_question_id = issued.published_question_id
       AND source.revision_number = issued.revision_number
     WHERE issued.assessment_attempt_id = assessment_attempt_row.assessment_attempt_id
       AND issued.issued_position = p_issued_position
       AND question_attempt.finalized_at IS NULL
       AND ple_private.question_backend_is_supported_for_production(source.backend)
     FOR UPDATE OF question_attempt;
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Issued Question is unavailable';
    END IF;
    now_value := pg_catalog.clock_timestamp();
    IF EXISTS (
        SELECT 1 FROM ple_private.assessment_submission AS submission
         WHERE submission.assessment_attempt_id = assessment_attempt_row.assessment_attempt_id
    ) OR (assessment_attempt_row.expires_at IS NOT NULL AND now_value >= assessment_attempt_row.expires_at) THEN
        assessment_attempt_id := assessment_attempt_row.assessment_attempt_id;
        issued_position := p_issued_position;
        response_state := 'expired';
        RETURN NEXT;
        RETURN;
    END IF;
    INSERT INTO ple_private.assessment_attempt_saved_response(
        course_instance_id, question_attempt_id, student_response, saved_at)
    VALUES (assessment_attempt_row.course_instance_id, question_attempt_id_value, p_student_response, now_value)
    ON CONFLICT (course_instance_id, question_attempt_id) DO UPDATE
       SET student_response = EXCLUDED.student_response, saved_at = EXCLUDED.saved_at;
    assessment_attempt_id := assessment_attempt_row.assessment_attempt_id;
    issued_position := p_issued_position;
    response_state := 'saved';
    RETURN NEXT;
END $$;

CREATE FUNCTION ple_private.read_student_assessment_attempt_progress(
    p_assessment_attempt_id uuid
) RETURNS TABLE (
    assessment_attempt_id uuid, question_count integer,
    recommended_position integer, issued_position integer, response_state text
) LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private AS $$
DECLARE assessment_attempt_row ple_private.assessment_attempt%ROWTYPE;
BEGIN
    assessment_attempt_row := ple_private.assert_current_student_assessment_attempt(p_assessment_attempt_id);
    RETURN QUERY
    SELECT assessment_attempt_row.assessment_attempt_id, count(*) OVER ()::integer,
           min(issued.issued_position) FILTER (WHERE question_attempt.finalized_at IS NULL
               AND response.question_attempt_id IS NULL) OVER (),
           issued.issued_position,
           CASE WHEN question_attempt.finalized_at IS NOT NULL
                     AND response.question_attempt_id IS NOT NULL THEN 'submitted'
                WHEN question_attempt.finalized_at IS NOT NULL THEN 'closed'
                WHEN response.question_attempt_id IS NOT NULL THEN 'saved' ELSE 'unanswered' END
      FROM ple_private.issued_question AS issued
      JOIN ple_private.question_attempt ON question_attempt.issued_question_id = issued.issued_question_id
      LEFT JOIN ple_private.assessment_attempt_saved_response AS response ON response.question_attempt_id = question_attempt.question_attempt_id
     WHERE issued.assessment_attempt_id = assessment_attempt_row.assessment_attempt_id
     ORDER BY issued.issued_position;
END $$;



-- A saved response is mutable working state for one currently active,
-- student-owned Assessment Attempt.  This reader resolves only the retained Issued
-- Question and Question Attempt evidence; it does not consult mutable
-- Assessment configuration.
CREATE FUNCTION ple_private.read_student_assessment_attempt_saved_response(
    p_assessment_attempt_id uuid,
    p_issued_position integer
) RETURNS TABLE (issued_position integer, student_response jsonb)
LANGUAGE plpgsql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
DECLARE assessment_attempt_row ple_private.assessment_attempt%ROWTYPE;
BEGIN
    IF p_issued_position < 0 THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Saved Student response is unavailable';
    END IF;

    assessment_attempt_row := ple_private.assert_current_student_assessment_attempt(
        p_assessment_attempt_id
    );
    RETURN QUERY
    SELECT issued.issued_position,
           response.student_response
      FROM ple_private.issued_question AS issued
      JOIN ple_private.question_attempt AS question_attempt
        ON question_attempt.issued_question_id = issued.issued_question_id
      LEFT JOIN ple_private.assessment_attempt_saved_response AS response
        ON response.question_attempt_id = question_attempt.question_attempt_id
     WHERE issued.assessment_attempt_id = assessment_attempt_row.assessment_attempt_id
       AND issued.issued_position = p_issued_position;
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Saved Student response is unavailable';
    END IF;
END $$;



-- Grading workers call this before recording a result.  It deliberately takes
-- the same Assessment row lock as student mutations so Unrelease either wins
-- before work begins or waits for the already-authorized commit to finish.
CREATE FUNCTION ple_private.lock_question_attempt_for_grading(p_question_attempt_id uuid)
RETURNS TABLE (
    question_attempt_id uuid, issued_question_id uuid, assessment_attempt_id uuid,
    assessment_id text, published_question_id text, revision_number integer, question_seed numeric,
    generated_parameter_sha256 text
) LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_data, ple_private AS $$
DECLARE assessment_id_value text;
BEGIN
    SELECT assessment_attempt.assessment_id INTO assessment_id_value
      FROM ple_private.question_attempt AS question_attempt
      JOIN ple_private.issued_question AS issued ON issued.issued_question_id = question_attempt.issued_question_id
      JOIN ple_private.assessment_attempt AS assessment_attempt ON assessment_attempt.assessment_attempt_id = issued.assessment_attempt_id
     WHERE question_attempt.question_attempt_id = p_question_attempt_id;
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Question Attempt grading target is unavailable';
    END IF;
    PERFORM ple_private.lock_assessment_for_student_work(assessment_id_value);
    RETURN QUERY
    SELECT question_attempt.question_attempt_id, issued.issued_question_id,
           assessment_attempt.assessment_attempt_id, assessment_attempt.assessment_id, issued.published_question_id,
           issued.revision_number, question_attempt.question_seed,
           question_attempt.generated_parameter_sha256
      FROM ple_private.question_attempt AS question_attempt
      JOIN ple_private.issued_question AS issued ON issued.issued_question_id = question_attempt.issued_question_id
      JOIN ple_private.assessment_attempt AS assessment_attempt ON assessment_attempt.assessment_attempt_id = issued.assessment_attempt_id
     WHERE question_attempt.question_attempt_id = p_question_attempt_id
     FOR KEY SHARE OF question_attempt, issued, assessment_attempt;
END $$;

CREATE FUNCTION ple_private.read_student_assessment_attempt_history_evidence(
    p_assessment_attempt_id uuid
) RETURNS TABLE (
    assessment_attempt_id uuid, assessment_title text, assessment_instructions text,
    issued_position integer, published_question_id text, revision_number integer, question_seed numeric,
    generated_parameter_sha256 text,
    question_attempt_limit integer, question_attempt_time_limit_seconds integer,
    question_attempt_grace_seconds integer, question_attempt_state text, student_response jsonb
) LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private, ple_data AS $$
DECLARE assessment_attempt_row ple_private.assessment_attempt%ROWTYPE;
BEGIN
    assessment_attempt_row := ple_private.assert_current_student_assessment_attempt(p_assessment_attempt_id);
    RETURN QUERY
    SELECT assessment_attempt_row.assessment_attempt_id, policy.assessment_title, policy.assessment_instructions,
           issued.issued_position, issued.published_question_id, issued.revision_number,
           question_attempt.question_seed, question_attempt.generated_parameter_sha256,
           snapshot.question_attempt_limit, snapshot.question_attempt_time_limit_seconds,
           snapshot.question_attempt_grace_seconds,
           ple_private.projected_question_attempt_state(
               question_attempt.finalized_at, response.question_attempt_id IS NOT NULL),
           response.student_response
      FROM ple_private.issued_question AS issued
      JOIN ple_private.assessment_entry_snapshot AS snapshot
        ON snapshot.assessment_entry_snapshot_id = issued.assessment_entry_snapshot_id
      JOIN ple_data.assessment_policy_snapshot AS policy
        ON policy.assessment_policy_snapshot_id = assessment_attempt_row.assessment_policy_snapshot_id
      JOIN ple_private.question_attempt ON question_attempt.issued_question_id = issued.issued_question_id
      LEFT JOIN ple_private.assessment_attempt_saved_response AS response ON response.question_attempt_id = question_attempt.question_attempt_id
     WHERE issued.assessment_attempt_id = assessment_attempt_row.assessment_attempt_id
     ORDER BY issued.issued_position;
END $$;

