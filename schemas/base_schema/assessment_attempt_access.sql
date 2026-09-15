-- Authenticated Student Assessment route reads.  Current Assessment state
-- determines whether another Assessment Attempt can start; existing Assessment Attempts provide
-- their own retained interpretation evidence.

SET LOCAL ROLE ple_private_owner;

-- Read decisions use statement_timestamp() so every field in one response has
-- one evaluation instant. Student Work mutations use clock_timestamp() at the
-- operation boundary. The browser formats these facts and never grants access.
-- ASVS 2.1.2, 2.2.3, 8.1.3, 8.3.1: document and enforce the combined timing
-- rules at the trusted PostgreSQL boundary.
CREATE FUNCTION ple_private.assessment_start_decision(
    p_assessment_status text,
    p_available_at timestamptz,
    p_due_at timestamptz,
    p_closes_at timestamptz,
    p_assessment_attempt_limit integer,
    p_started_assessment_attempt_count integer,
    p_late_work_rule text,
    p_evaluated_at timestamptz
) RETURNS text
LANGUAGE plpgsql IMMUTABLE SET search_path = pg_catalog AS $$
BEGIN
    -- ASVS 2.2.1: reject values outside the closed stored-policy contract.
    IF p_assessment_status IS NULL
       OR p_assessment_status NOT IN ('unreleased', 'released', 'closed', 'archived')
       OR p_late_work_rule IS NULL
       OR p_late_work_rule NOT IN ('accept', 'mark_late', 'reject')
       OR p_started_assessment_attempt_count IS NULL
       OR p_started_assessment_attempt_count < 0
       OR (p_assessment_attempt_limit IS NOT NULL AND p_assessment_attempt_limit <= 0)
       OR p_evaluated_at IS NULL THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Assessment Start Decision inputs are invalid';
    END IF;
    IF p_assessment_status <> 'released' THEN
        RETURN 'closed';
    ELSIF p_closes_at IS NOT NULL AND p_evaluated_at >= p_closes_at THEN
        RETURN 'closed';
    ELSIF p_available_at IS NOT NULL AND p_evaluated_at < p_available_at THEN
        RETURN 'not_yet_available';
    ELSIF p_assessment_attempt_limit IS NOT NULL
          AND p_started_assessment_attempt_count >= p_assessment_attempt_limit THEN
        RETURN 'attempt_limit_reached';
    ELSIF p_late_work_rule = 'reject'
          AND p_due_at IS NOT NULL
          AND p_evaluated_at > p_due_at THEN
        RETURN 'late_work_refused';
    END IF;
    RETURN 'may_start';
END $$;

CREATE FUNCTION ple_private.read_student_assessment_access(
    p_course_reference_number bigint,
    p_assessment_reference_number text
) RETURNS TABLE (
    start_decision text,
    assessment_title text,
    assessment_type text,
    question_count integer,
    points_possible double precision,
    assessment_attempt_time_limit_seconds integer,
    available_at timestamptz,
    due_at timestamptz,
    closes_at timestamptz,
    assessment_attempt_limit integer,
    late_work_rule text,
    evaluated_at timestamptz,
    display_time_zone text,
    previous_assessment_attempts jsonb
)
LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
DECLARE assessment_row ple_data.assessment%ROWTYPE; accommodation_row ple_private.student_assessment_accommodation%ROWTYPE;
DECLARE student_record_id_value uuid; active_assessment_attempt ple_private.assessment_attempt%ROWTYPE;
DECLARE evaluation_time timestamptz := pg_catalog.statement_timestamp(); started_assessment_attempt_count integer;
BEGIN
    IF p_course_reference_number NOT BETWEEN 1 AND 2147483647 THEN RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Assessment is unavailable'; END IF;
    SELECT assessment.* INTO assessment_row FROM ple_data.assessment AS assessment WHERE assessment.public_reference = p_assessment_reference_number;
    IF NOT FOUND OR ple_api.course_reference_number_for_assessment_attempt(assessment_row.course_id) IS DISTINCT FROM p_course_reference_number THEN RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Assessment is unavailable'; END IF;
    -- ASVS 8.2.2, 8.2.3, 14.2.6: return only this session Student's
    -- effective policy values, never accommodation identity or another record.
    SELECT ple_api.current_session_student_record_id(assessment_row.course_id) INTO student_record_id_value;
    IF student_record_id_value IS NULL OR NOT ple_api.current_session_account_owns_student_record(assessment_row.course_id, student_record_id_value) THEN RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Assessment is unavailable'; END IF;
    SELECT * INTO accommodation_row FROM ple_private.student_assessment_accommodation WHERE student_record_id = student_record_id_value AND assessment_id = assessment_row.assessment_id;
    available_at := COALESCE(accommodation_row.available_at, assessment_row.available_at);
    due_at := COALESCE(accommodation_row.due_at, assessment_row.due_at);
    closes_at := COALESCE(accommodation_row.closes_at, assessment_row.closes_at);
    assessment_attempt_limit := CASE
        WHEN assessment_row.assessment_type IN ('quiz', 'exam') THEN 1
        ELSE COALESCE(
            accommodation_row.assessment_attempt_limit,
            assessment_row.assessment_attempt_limit
        )
    END;
    late_work_rule := assessment_row.late_work_rule;
    assessment_type := assessment_row.assessment_type;
    evaluated_at := evaluation_time;
    SELECT preference.time_zone INTO display_time_zone
      FROM ple_private.account_time_zone AS preference
     WHERE preference.account_id = ple_api.current_session_account_id();
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '55000',
            MESSAGE = 'Student Account time zone is unavailable';
    END IF;
    SELECT * INTO active_assessment_attempt FROM ple_private.assessment_attempt
     WHERE student_record_id = student_record_id_value
       AND assessment_id = assessment_row.assessment_id
       AND NOT EXISTS (SELECT 1 FROM ple_private.assessment_submission AS submission
                        WHERE submission.assessment_attempt_id = assessment_attempt_id)
       AND (expires_at IS NULL OR expires_at > evaluation_time)
     ORDER BY assessment_attempt_number DESC LIMIT 1;
    IF FOUND THEN
        assessment_title := active_assessment_attempt.assessment_title; assessment_attempt_time_limit_seconds := active_assessment_attempt.assessment_attempt_time_limit_seconds;
        SELECT count(*)::integer,
               COALESCE(sum(ple_private.grade_contribution_points_possible(
                   assessment_row.assessment_type,
                   issued.scoring_rule,
                   CASE entry.entry_kind
                       WHEN 'fixed_question' THEN entry.points_possible
                       ELSE entry.points_per_item
                   END
               )), 0)::double precision
          INTO question_count, points_possible
          FROM ple_private.issued_question AS issued
          JOIN ple_data.assessment_entry AS entry
            ON entry.assessment_entry_id = issued.assessment_entry_id
         WHERE issued.assessment_attempt_id = active_assessment_attempt.assessment_attempt_id;
    ELSE
        assessment_title := assessment_row.assessment_title; assessment_attempt_time_limit_seconds := COALESCE(accommodation_row.assessment_attempt_time_limit_seconds, assessment_row.assessment_attempt_time_limit_seconds);
        SELECT COALESCE(sum(CASE entry.entry_kind WHEN 'fixed_question' THEN 1 ELSE entry.selection_count END), 0)::integer,
               COALESCE(sum(ple_private.grade_contribution_points_possible(
                   assessment_row.assessment_type,
                   entry.scoring_rule,
                   CASE entry.entry_kind
                       WHEN 'fixed_question' THEN entry.points_possible
                       ELSE entry.points_per_item * entry.selection_count
                   END
               )), 0)::double precision
          INTO question_count, points_possible
          FROM ple_data.assessment_entry AS entry
         WHERE entry.assessment_id = assessment_row.assessment_id
           AND entry.availability = 'available';
    END IF;
    SELECT count(*)::integer INTO started_assessment_attempt_count
      FROM ple_private.assessment_attempt AS assessment_attempt
     WHERE assessment_attempt.student_record_id = student_record_id_value
       AND assessment_attempt.assessment_id = assessment_row.assessment_id;
    start_decision := ple_private.assessment_start_decision(
        assessment_row.assessment_status,
        available_at,
        due_at,
        closes_at,
        assessment_attempt_limit,
        started_assessment_attempt_count,
        late_work_rule,
        evaluation_time
    );
    SELECT COALESCE(jsonb_agg(jsonb_build_object('assessmentAttempt', 'R-' || assessment_attempt.reference_number, 'attemptNumber', assessment_attempt.assessment_attempt_number, 'state', CASE WHEN submission.assessment_attempt_id IS NULL THEN 'closed' ELSE 'submitted' END) ORDER BY assessment_attempt.assessment_attempt_number DESC, assessment_attempt.reference_number DESC), '[]'::jsonb) INTO previous_assessment_attempts FROM ple_private.assessment_attempt AS assessment_attempt LEFT JOIN ple_private.assessment_submission AS submission ON submission.assessment_attempt_id = assessment_attempt.assessment_attempt_id WHERE assessment_attempt.student_record_id = student_record_id_value AND assessment_attempt.assessment_id = assessment_row.assessment_id AND (submission.assessment_attempt_id IS NOT NULL OR (assessment_attempt.expires_at IS NOT NULL AND assessment_attempt.expires_at <= evaluation_time));
    RETURN NEXT;
END $$;

CREATE FUNCTION ple_private.read_active_student_assessment_attempt_reference(p_course_reference_number bigint, p_assessment_reference_number text)
RETURNS TABLE (assessment_attempt_reference_number bigint) LANGUAGE sql STABLE SECURITY DEFINER SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
    SELECT assessment_attempt.reference_number FROM ple_private.assessment_attempt AS assessment_attempt JOIN ple_data.assessment AS assessment ON assessment.assessment_id = assessment_attempt.assessment_id WHERE p_course_reference_number BETWEEN 1 AND 2147483647 AND ple_api.course_reference_number_for_assessment_attempt(assessment.course_id) = p_course_reference_number AND assessment.public_reference = p_assessment_reference_number AND NOT EXISTS (SELECT 1 FROM ple_private.assessment_submission AS submission WHERE submission.assessment_attempt_id = assessment_attempt.assessment_attempt_id) AND (assessment_attempt.expires_at IS NULL OR assessment_attempt.expires_at > pg_catalog.statement_timestamp()) AND ple_api.current_session_account_owns_student_record(assessment.course_id, assessment_attempt.student_record_id) ORDER BY assessment_attempt.assessment_attempt_number DESC LIMIT 1
$$;

CREATE FUNCTION ple_private.read_reusable_question_pool_selection(p_assessment_id uuid, p_student_record_id uuid, p_assessment_entry_id uuid)
RETURNS TABLE (question_pool_selection_id uuid, selection_position integer, question_pool_id uuid, question_pool_public_id text, question_pool_revision_number bigint, member_position integer, question_id text, revision_number integer)
LANGUAGE plpgsql STABLE SECURITY DEFINER SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
DECLARE course_id_value uuid;
BEGIN
    SELECT assessment.course_id INTO course_id_value FROM ple_data.assessment AS assessment WHERE assessment.assessment_id = p_assessment_id;
    IF NOT FOUND OR NOT ple_api.current_session_account_owns_student_record(course_id_value, p_student_record_id) THEN RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Question Pool Selection is unavailable'; END IF;
    RETURN QUERY
        WITH latest AS (
            SELECT selection.question_pool_selection_id
            FROM ple_private.question_pool_selection AS selection
            JOIN ple_private.assessment_attempt AS assessment_attempt
                ON assessment_attempt.assessment_attempt_id = selection.assessment_attempt_id
            WHERE assessment_attempt.student_record_id = p_student_record_id
                AND assessment_attempt.assessment_id = p_assessment_id
                AND selection.assessment_entry_id = p_assessment_entry_id
            ORDER BY assessment_attempt.assessment_attempt_number DESC,
                selection.created_at DESC,
                selection.question_pool_selection_id DESC
            LIMIT 1
        )
        SELECT selected.question_pool_selection_id,
            selected.selection_position,
            selection.question_pool_id,
            pool.public_question_pool_id,
            selection.question_pool_revision_number,
            selected.member_position,
            selected.question_id,
            selected.revision_number
        FROM latest
        JOIN ple_private.question_pool_selection AS selection
            ON selection.question_pool_selection_id = latest.question_pool_selection_id
        JOIN ple_data.question_pool AS pool ON pool.question_pool_id = selection.question_pool_id
        JOIN ple_private.question_pool_selected_item AS selected
            ON selected.question_pool_selection_id = latest.question_pool_selection_id
        ORDER BY selected.selection_position;
END $$;

-- Pool-member selection is immutable Student Work evidence.  The start
-- operation writes the exact C353 QuestionRevisionReferences in the same
-- transaction as its Assessment Attempt; this read accepts only an opaque Assessment Attempt route
-- reference and derives the owning Student from the installed session.
-- ASVS 2.2.1, 2.2.2, 2.3.1: do not accept a browser-selected Student,
-- Assessment, Pool member, or revision, and do not let one Student read
-- another Student's retained selection.
CREATE FUNCTION ple_private.read_student_assessment_attempt_pool_selection(
    p_assessment_attempt_reference_number bigint
) RETURNS TABLE (
    assessment_attempt_reference_number bigint,
    assessment_entry_id uuid,
    question_pool_selection_id uuid,
    selection_position integer,
    question_pool_id uuid,
    question_pool_public_id text,
    question_pool_revision_number bigint,
    member_position integer,
    question_id text,
    revision_number integer
)
LANGUAGE plpgsql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private AS $$
DECLARE assessment_attempt_row ple_private.assessment_attempt%ROWTYPE;
BEGIN
    assessment_attempt_row := ple_private.assert_current_student_assessment_attempt(
        p_assessment_attempt_reference_number
    );
    RETURN QUERY
    SELECT assessment_attempt_row.reference_number,
           selection.assessment_entry_id,
           selection.question_pool_selection_id,
           selected.selection_position,
           selection.question_pool_id,
           pool.public_question_pool_id,
           selection.question_pool_revision_number,
           selected.member_position,
           selected.question_id,
           selected.revision_number
      FROM ple_private.question_pool_selection AS selection
      JOIN ple_data.question_pool AS pool ON pool.question_pool_id = selection.question_pool_id
      JOIN ple_private.question_pool_selected_item AS selected
        ON selected.question_pool_selection_id = selection.question_pool_selection_id
     WHERE selection.assessment_attempt_id = assessment_attempt_row.assessment_attempt_id
     ORDER BY selection.assessment_entry_id, selected.selection_position;
END $$;

CREATE FUNCTION ple_private.read_student_assessment_attempt_context(p_assessment_attempt_reference_number bigint)
RETURNS TABLE (assessment_attempt_reference_number bigint, assessment_attempt_number integer, course_reference_number text, course_short_name text, course_long_name text, course_theme text, assessment_reference_number text, assessment_title text, display_time_zone text, expires_at_millis bigint, timer_remaining_milliseconds bigint)
LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
DECLARE assessment_attempt_id_value uuid;
DECLARE evaluation_time timestamptz := pg_catalog.statement_timestamp();
BEGIN
    SELECT assessment_attempt.assessment_attempt_id INTO assessment_attempt_id_value
      FROM ple_private.assessment_attempt AS assessment_attempt
      JOIN ple_data.assessment AS assessment
        ON assessment.assessment_id = assessment_attempt.assessment_id
     WHERE p_assessment_attempt_reference_number BETWEEN 1 AND 2147483647
       AND assessment_attempt.reference_number = p_assessment_attempt_reference_number
       AND ple_api.current_session_account_owns_student_record(
           assessment.course_id, assessment_attempt.student_record_id
       );
    IF NOT FOUND THEN
        RETURN;
    END IF;
    RETURN QUERY
    SELECT assessment_attempt.reference_number, assessment_attempt.assessment_attempt_number, course.course_reference_number,
           course.course_short_name, course.course_long_name, course.course_theme,
           assessment.public_reference, assessment_attempt.assessment_title,
           preference.time_zone,
           CASE WHEN assessment_attempt.expires_at IS NULL THEN NULL
                ELSE floor(extract(epoch FROM assessment_attempt.expires_at) * 1000)::bigint END,
           CASE WHEN assessment_attempt.expires_at IS NULL THEN NULL
                ELSE greatest(0::bigint, floor(extract(epoch FROM (
                    assessment_attempt.expires_at - evaluation_time
                )) * 1000)::bigint) END
      FROM ple_private.assessment_attempt AS assessment_attempt
      JOIN ple_data.assessment AS assessment ON assessment.assessment_id = assessment_attempt.assessment_id
      JOIN ple_private.account_time_zone AS preference
        ON preference.account_id = ple_api.current_session_account_id()
      JOIN LATERAL ple_api.course_display_for_assessment_attempt(assessment.course_id) AS course ON true
     WHERE assessment_attempt.assessment_attempt_id = assessment_attempt_id_value;
END
$$;

REVOKE ALL ON FUNCTION ple_private.assessment_start_decision(text, timestamptz, timestamptz, timestamptz, integer, integer, text, timestamptz), ple_private.read_student_assessment_access(bigint, text), ple_private.read_active_student_assessment_attempt_reference(bigint, text), ple_private.read_reusable_question_pool_selection(uuid, uuid, uuid), ple_private.read_student_assessment_attempt_pool_selection(bigint), ple_private.read_student_assessment_attempt_context(bigint) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_private.read_student_assessment_access(bigint, text), ple_private.read_active_student_assessment_attempt_reference(bigint, text), ple_private.read_reusable_question_pool_selection(uuid, uuid, uuid), ple_private.read_student_assessment_attempt_pool_selection(bigint), ple_private.read_student_assessment_attempt_context(bigint) TO ple_api_owner;
RESET ROLE;

SET LOCAL ROLE ple_api_owner;
CREATE FUNCTION ple_api.read_student_assessment_access(text, text) RETURNS TABLE (start_decision text, assessment_title text, assessment_type text, question_count integer, points_possible double precision, assessment_attempt_time_limit_seconds integer, available_at timestamptz, due_at timestamptz, closes_at timestamptz, assessment_attempt_limit integer, late_work_rule text, evaluated_at timestamptz, display_time_zone text, previous_assessment_attempts jsonb) LANGUAGE sql SECURITY DEFINER SET search_path = pg_catalog, ple_private, ple_api, ple_data AS $$ SELECT * FROM ple_private.read_student_assessment_access((SELECT reference_number FROM ple_data.course_instance WHERE public_reference = $1), $2) $$;
CREATE FUNCTION ple_api.read_active_student_assessment_attempt_reference(text, text) RETURNS TABLE (assessment_attempt_reference_number bigint) LANGUAGE sql STABLE SECURITY DEFINER SET search_path = pg_catalog, ple_private, ple_api, ple_data AS $$ SELECT * FROM ple_private.read_active_student_assessment_attempt_reference((SELECT reference_number FROM ple_data.course_instance WHERE public_reference = $1), $2) $$;
CREATE FUNCTION ple_api.read_reusable_question_pool_selection(uuid, uuid, uuid) RETURNS TABLE (question_pool_selection_id uuid, selection_position integer, question_pool_id uuid, question_pool_public_id text, question_pool_revision_number bigint, member_position integer, question_id text, revision_number integer) LANGUAGE sql STABLE SECURITY DEFINER SET search_path = pg_catalog, ple_private, ple_api AS $$ SELECT * FROM ple_private.read_reusable_question_pool_selection($1, $2, $3) $$;
CREATE FUNCTION ple_api.read_student_assessment_attempt_pool_selection(bigint) RETURNS TABLE (assessment_attempt_reference_number bigint, assessment_entry_id uuid, question_pool_selection_id uuid, selection_position integer, question_pool_id uuid, question_pool_public_id text, question_pool_revision_number bigint, member_position integer, question_id text, revision_number integer) LANGUAGE sql SECURITY DEFINER SET search_path = pg_catalog, ple_private, ple_api AS $$ SELECT * FROM ple_private.read_student_assessment_attempt_pool_selection($1) $$;
CREATE FUNCTION ple_api.read_student_assessment_attempt_context(bigint) RETURNS TABLE (assessment_attempt_reference_number bigint, assessment_attempt_number integer, course_reference_number text, course_short_name text, course_long_name text, course_theme text, assessment_reference_number text, assessment_title text, display_time_zone text, expires_at_millis bigint, timer_remaining_milliseconds bigint) LANGUAGE sql SECURITY DEFINER SET search_path = pg_catalog, ple_private, ple_api AS $$ SELECT * FROM ple_private.read_student_assessment_attempt_context($1) $$;
REVOKE ALL ON FUNCTION ple_api.read_student_assessment_access(text, text), ple_api.read_active_student_assessment_attempt_reference(text, text), ple_api.read_reusable_question_pool_selection(uuid, uuid, uuid), ple_api.read_student_assessment_attempt_pool_selection(bigint), ple_api.read_student_assessment_attempt_context(bigint) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_api.read_student_assessment_access(text, text), ple_api.read_active_student_assessment_attempt_reference(text, text), ple_api.read_reusable_question_pool_selection(uuid, uuid, uuid), ple_api.read_student_assessment_attempt_pool_selection(bigint), ple_api.read_student_assessment_attempt_context(bigint) TO ple_app;
RESET ROLE;
