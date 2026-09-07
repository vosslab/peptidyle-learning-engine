-- M11 Student Assignment Access and answer-free fixed-question issuance.
--
-- This adapter delegates durable Attempt creation to the established atomic
-- Student Work transaction.  M10 releases fixed Published Questions only;
-- pools, Student Responses, and submissions stay with their later milestones.

-- ASVS 8.2.1/8.2.2: the API role already has relation-level SELECT from M10, but forced RLS
-- admitted only insertion on the immutable snapshot entries.  M11's trusted
-- SECURITY DEFINER start procedure needs these exact released fixed pins to
-- construct one Student-owned issue set.  It receives no private source,
-- Answer Key, grading input, or mutable assignment-selection capability.
SET LOCAL ROLE ple_data_owner;

CREATE POLICY assignment_revision_entry_api_owner_live_demo_m11_read
    ON ple_data.assignment_revision_entry FOR SELECT TO ple_api_owner USING (true);
CREATE POLICY assignment_revision_fixed_question_api_owner_live_demo_m11_read
    ON ple_data.assignment_revision_fixed_question FOR SELECT TO ple_api_owner USING (true);

RESET ROLE;

SET LOCAL ROLE ple_api_owner;

CREATE FUNCTION ple_api.live_demo_assignment_access(
    p_course_reference_number bigint,
    p_assignment_reference_number bigint
)
RETURNS TABLE (start_decision text)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
DECLARE
    v_course_id uuid;
    v_assignment ple_data.assignment%ROWTYPE;
    v_student_record_id uuid;
    v_revision ple_data.assignment_revision%ROWTYPE;
    v_started_attempt_count integer;
    v_unfinished_attempt_exists boolean;
    v_now timestamp with time zone := pg_catalog.clock_timestamp();
BEGIN
    SELECT assignment.*
      INTO v_assignment
      FROM ple_data.course_instance AS course
      JOIN ple_data.assignment AS assignment ON assignment.course_id = course.course_id
     WHERE course.reference_number = p_course_reference_number
       AND assignment.reference_number = p_assignment_reference_number;
    v_course_id := v_assignment.course_id;
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Assignment is unavailable';
    END IF;
    SELECT student_record.student_record_id INTO v_student_record_id
      FROM ple_data.student_record AS student_record
     WHERE student_record.course_id = v_course_id
       AND student_record.student_account_id = ple_api.current_session_account_id();
    IF NOT FOUND OR NOT ple_api.current_session_account_owns_student_record(v_course_id, v_student_record_id) THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Assignment is unavailable';
    END IF;
    SELECT revision.* INTO v_revision
      FROM ple_data.assignment_revision AS revision
     WHERE revision.assignment_revision_id = v_assignment.released_assignment_revision_id;
    IF v_assignment.assignment_status <> 'released' OR NOT FOUND THEN
        start_decision := 'closed';
    ELSIF v_revision.available_at IS NOT NULL AND v_now < v_revision.available_at THEN
        start_decision := 'not_yet_available';
    ELSIF v_revision.closes_at IS NOT NULL AND v_now >= v_revision.closes_at THEN
        start_decision := 'closed';
    ELSIF v_revision.due_at IS NOT NULL AND v_now > v_revision.due_at
          AND v_revision.late_work_rule = 'reject' THEN
        start_decision := 'late_work_refused';
    ELSE
        SELECT EXISTS (
            SELECT 1 FROM ple_private.assignment_attempt AS attempt
             WHERE attempt.student_record_id = v_student_record_id
               AND attempt.assignment_id = v_assignment.assignment_id
               AND attempt.completed_at IS NULL
        ) INTO v_unfinished_attempt_exists;
        IF v_unfinished_attempt_exists THEN
            start_decision := 'may_start';
            RETURN NEXT;
            RETURN;
        END IF;
        SELECT count(*)::integer INTO v_started_attempt_count
          FROM ple_private.assignment_attempt AS attempt
         WHERE attempt.student_record_id = v_student_record_id
           AND attempt.assignment_id = v_assignment.assignment_id;
        IF v_revision.attempt_limit IS NOT NULL AND v_started_attempt_count >= v_revision.attempt_limit THEN
            start_decision := 'attempt_limit_reached';
        ELSE
            start_decision := 'may_start';
        END IF;
    END IF;
    RETURN NEXT;
END
$$;

CREATE FUNCTION ple_api.start_live_demo_assignment(
    p_course_reference_number bigint,
    p_assignment_reference_number bigint
)
RETURNS TABLE (
    attempt_number integer,
    resumed boolean,
    assignment_title text,
    assignment_instructions text,
    question_id text,
    question_description text,
    issued_position integer
)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
DECLARE
    v_course_id uuid;
    v_assignment ple_data.assignment%ROWTYPE;
    v_student_record_id uuid;
    v_attempt record;
    v_issued_questions jsonb;
    v_revision ple_data.assignment_revision%ROWTYPE;
    v_now timestamp with time zone;
BEGIN
    SELECT assignment.*
      INTO v_assignment
      FROM ple_data.course_instance AS course
      JOIN ple_data.assignment AS assignment ON assignment.course_id = course.course_id
     WHERE course.reference_number = p_course_reference_number
       AND assignment.reference_number = p_assignment_reference_number
     FOR UPDATE OF assignment;
    v_course_id := v_assignment.course_id;
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Assignment is unavailable';
    END IF;
    SELECT student_record.student_record_id INTO v_student_record_id
      FROM ple_data.student_record AS student_record
     WHERE student_record.course_id = v_course_id
       AND student_record.student_account_id = ple_api.current_session_account_id();
    IF NOT FOUND OR NOT ple_api.current_session_account_owns_student_record(v_course_id, v_student_record_id) THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Assignment is unavailable';
    END IF;
    -- ASVS 2.3.1/2.3.3: the assignment lock remains held through the delegated atomic start.
    -- Thus the due/reject decision and the insert share one transaction and
    -- cannot cross a policy boundary between a read-only access check and issue.
    SELECT revision.* INTO v_revision
      FROM ple_data.assignment_revision AS revision
     WHERE revision.assignment_revision_id = v_assignment.released_assignment_revision_id;
    v_now := pg_catalog.clock_timestamp();
    IF v_assignment.assignment_status <> 'released' OR NOT FOUND
       OR (v_revision.available_at IS NOT NULL AND v_now < v_revision.available_at)
       OR (v_revision.closes_at IS NOT NULL AND v_now >= v_revision.closes_at)
       OR (v_revision.due_at IS NOT NULL AND v_now > v_revision.due_at
           AND v_revision.late_work_rule = 'reject') THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Assignment cannot start at this time';
    END IF;
    SELECT jsonb_agg(jsonb_build_object(
        'issued_question_id', pg_catalog.gen_random_uuid(),
        'assignment_entry_id', fixed_question.assignment_entry_id,
        'issued_position', entry.assignment_content_entry_index,
        'question_id', fixed_question.question_id,
        'revision_number', fixed_question.revision_number,
        'question_pool_selection_id', NULL,
        'question_pool_item_id', NULL
    ) ORDER BY entry.assignment_content_entry_index)
      INTO v_issued_questions
      FROM ple_data.assignment_revision_entry AS entry
      JOIN ple_data.assignment_revision_fixed_question AS fixed_question
        ON fixed_question.assignment_revision_id = entry.assignment_revision_id
       AND fixed_question.assignment_entry_id = entry.assignment_entry_id
     WHERE entry.assignment_revision_id = v_assignment.released_assignment_revision_id;
    IF v_issued_questions IS NULL THEN
        RAISE EXCEPTION USING ERRCODE = '23514', MESSAGE = 'Released Assignment has no issuable Question';
    END IF;
    SELECT * INTO v_attempt
      FROM ple_private.start_assignment_attempt(
          pg_catalog.gen_random_uuid(), v_student_record_id, v_assignment.assignment_id,
          '[]'::jsonb, v_issued_questions
      );
    RETURN QUERY
    SELECT v_attempt.attempt_number, v_attempt.resumed,
           revision.assignment_title, revision.assignment_instructions,
           fixed_question.question_id, metadata.question_description,
           entry.assignment_content_entry_index
      FROM ple_data.assignment_revision AS revision
      JOIN ple_data.assignment_revision_entry AS entry
        ON entry.assignment_revision_id = revision.assignment_revision_id
      JOIN ple_data.assignment_revision_fixed_question AS fixed_question
        ON fixed_question.assignment_revision_id = entry.assignment_revision_id
       AND fixed_question.assignment_entry_id = entry.assignment_entry_id
      JOIN ple_data.published_question_metadata AS metadata
        ON metadata.question_id = fixed_question.question_id
     WHERE revision.assignment_revision_id = v_assignment.released_assignment_revision_id
     ORDER BY entry.assignment_content_entry_index;
END
$$;

REVOKE ALL PRIVILEGES ON FUNCTION ple_api.live_demo_assignment_access(bigint, bigint),
    ple_api.start_live_demo_assignment(bigint, bigint) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_api.live_demo_assignment_access(bigint, bigint),
    ple_api.start_live_demo_assignment(bigint, bigint) TO ple_app;

RESET ROLE;
