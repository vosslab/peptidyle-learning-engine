-- Authenticated atomic Assignment Attempt start and resume.
--
-- The API accepts only server-prepared Question Pool candidates and exact
-- Question Revision pins. PostgreSQL derives the released Assignment Revision,
-- policy, attempt number, clock time, issued scoring facts, and statistics
-- eligibility in the same transaction.

SET LOCAL ROLE ple_data_owner;

CREATE POLICY assignment_data_owner_access ON ple_data.assignment
    FOR ALL TO ple_data_owner USING (true) WITH CHECK (true);
CREATE POLICY assignment_revision_data_owner_access ON ple_data.assignment_revision
    FOR ALL TO ple_data_owner USING (true) WITH CHECK (true);
CREATE POLICY assignment_private_owner_lookup ON ple_data.assignment
    FOR SELECT TO ple_private_owner USING (true);
CREATE POLICY assignment_private_owner_lock ON ple_data.assignment
    FOR UPDATE TO ple_private_owner USING (true) WITH CHECK (true);
CREATE POLICY assignment_revision_private_owner_lookup ON ple_data.assignment_revision
    FOR SELECT TO ple_private_owner USING (true);
GRANT USAGE ON SCHEMA ple_data TO ple_private_owner;
GRANT SELECT, UPDATE ON TABLE ple_data.assignment TO ple_private_owner;
GRANT SELECT ON TABLE ple_data.assignment_revision TO ple_private_owner;

RESET ROLE;

SET LOCAL ROLE ple_private_owner;

CREATE FUNCTION ple_private.start_assignment_attempt(
    p_assignment_attempt_id uuid,
    p_student_record_id uuid,
    p_assignment_id uuid,
    p_question_pool_selections jsonb,
    p_issued_questions jsonb
)
RETURNS TABLE (
    assignment_attempt_id uuid,
    attempt_number integer,
    resumed boolean
)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private
AS $$
DECLARE
    v_course_id uuid;
    v_assignment_revision_id uuid;
    v_attempt_number integer;
    v_existing_attempt_id uuid;
    v_existing_attempt_number integer;
    v_available_at timestamp with time zone;
    v_closes_at timestamp with time zone;
    v_attempt_limit integer;
    v_continuation_rule text;
    v_max_additional_attempts integer;
    v_question_pool_reuse_rule text;
    v_question_variation_rule text;
    v_completed_attempt_count integer;
    v_started_attempt_count integer;
    v_now timestamp with time zone := pg_catalog.clock_timestamp();
BEGIN
    IF jsonb_typeof(p_question_pool_selections) <> 'array'
       OR jsonb_typeof(p_issued_questions) <> 'array' THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Assignment Attempt start requires Selection and Issued Question arrays';
    END IF;

    SELECT assignment.course_id, assignment.released_assignment_revision_id,
           revision.available_at, revision.closes_at, revision.attempt_limit,
           revision.assignment_attempt_continuation_rule,
           revision.max_additional_assignment_attempts,
           revision.question_pool_reuse_rule, revision.question_variation_rule
      INTO v_course_id, v_assignment_revision_id, v_available_at, v_closes_at,
           v_attempt_limit, v_continuation_rule, v_max_additional_attempts,
           v_question_pool_reuse_rule, v_question_variation_rule
      FROM ple_data.assignment AS assignment
      JOIN ple_data.assignment_revision AS revision
        ON revision.assignment_id = assignment.assignment_id
       AND revision.assignment_revision_id = assignment.released_assignment_revision_id
     WHERE assignment.assignment_id = p_assignment_id
       AND assignment.assignment_status = 'released'
     FOR UPDATE OF assignment;

    IF v_course_id IS NULL
       OR NOT ple_api.current_session_account_owns_student_record(
           v_course_id, p_student_record_id
       ) THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Assignment Attempt start requires active Student ownership of a Released Assignment';
    END IF;
    IF (v_available_at IS NOT NULL AND v_now < v_available_at)
       OR (v_closes_at IS NOT NULL AND v_now >= v_closes_at) THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Assignment Attempt start is outside the Released Assignment availability window';
    END IF;

    PERFORM pg_catalog.pg_advisory_xact_lock(
        pg_catalog.hashtextextended(p_student_record_id::text || ':' || p_assignment_id::text, 0)
    );
    SELECT existing.assignment_attempt_id, existing.attempt_number
      INTO v_existing_attempt_id, v_existing_attempt_number
      FROM ple_private.assignment_attempt AS existing
     WHERE existing.student_record_id = p_student_record_id
       AND existing.assignment_id = p_assignment_id
       AND existing.completed_at IS NULL
     ORDER BY existing.attempt_number DESC
     LIMIT 1
     FOR UPDATE;
    IF v_existing_attempt_id IS NOT NULL THEN
        RETURN QUERY SELECT v_existing_attempt_id, v_existing_attempt_number, true;
        RETURN;
    END IF;

    SELECT count(*)::integer,
           count(*) FILTER (WHERE completed_at IS NOT NULL)::integer
      INTO v_started_attempt_count, v_completed_attempt_count
      FROM ple_private.assignment_attempt
     WHERE student_record_id = p_student_record_id
       AND assignment_id = p_assignment_id;
    IF v_attempt_limit IS NOT NULL AND v_started_attempt_count >= v_attempt_limit THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Assignment Attempt Limit does not allow another Attempt';
    END IF;
    IF v_completed_attempt_count > 0
       AND (
           v_continuation_rule = 'closed'
           OR (
               v_continuation_rule = 'capped'
               AND v_completed_attempt_count - 1 >= v_max_additional_attempts
           )
       ) THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Assignment Attempt Continuation Rule does not allow another Attempt';
    END IF;

    SELECT COALESCE(max(existing.attempt_number), 0) + 1
      INTO v_attempt_number
      FROM ple_private.assignment_attempt AS existing
     WHERE existing.student_record_id = p_student_record_id
       AND existing.assignment_id = p_assignment_id;

    INSERT INTO ple_private.assignment_attempt (
        assignment_attempt_id, student_record_id, assignment_id, assignment_revision_id,
        started_at, completed_at, attempt_number, question_pool_reuse_rule, question_variation_rule
    ) VALUES (
        p_assignment_attempt_id, p_student_record_id, p_assignment_id, v_assignment_revision_id,
        v_now, NULL, v_attempt_number, v_question_pool_reuse_rule, v_question_variation_rule
    );

    IF EXISTS (
        SELECT 1
          FROM jsonb_to_recordset(p_question_pool_selections) AS input (
              question_pool_selection_id uuid,
              assignment_entry_id uuid,
              reused_from_question_pool_selection_id uuid,
              selected_candidates jsonb
          )
         WHERE jsonb_typeof(input.selected_candidates) <> 'array'
    ) THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'each prepared Question Pool Selection requires a candidate array';
    END IF;

    IF EXISTS (
        (SELECT question_pool.assignment_entry_id
           FROM ple_data.assignment_revision_question_pool AS question_pool
          WHERE question_pool.assignment_revision_id = v_assignment_revision_id
         EXCEPT
         SELECT input.assignment_entry_id
           FROM jsonb_to_recordset(p_question_pool_selections) AS input (
               question_pool_selection_id uuid,
               assignment_entry_id uuid,
               reused_from_question_pool_selection_id uuid,
               selected_candidates jsonb
           ))
        UNION ALL
        (SELECT input.assignment_entry_id
           FROM jsonb_to_recordset(p_question_pool_selections) AS input (
               question_pool_selection_id uuid,
               assignment_entry_id uuid,
               reused_from_question_pool_selection_id uuid,
               selected_candidates jsonb
           )
         EXCEPT
         SELECT question_pool.assignment_entry_id
           FROM ple_data.assignment_revision_question_pool AS question_pool
          WHERE question_pool.assignment_revision_id = v_assignment_revision_id)
    ) THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'prepared Question Pool Selections must cover each exact Released Assignment Entry once';
    END IF;

    IF EXISTS (
        SELECT 1
          FROM ple_data.assignment_revision_question_pool AS question_pool
          JOIN jsonb_to_recordset(p_question_pool_selections) AS input (
              question_pool_selection_id uuid,
              assignment_entry_id uuid,
              reused_from_question_pool_selection_id uuid,
              selected_candidates jsonb
          ) ON input.assignment_entry_id = question_pool.assignment_entry_id
         WHERE question_pool.assignment_revision_id = v_assignment_revision_id
           AND jsonb_array_length(input.selected_candidates) <> question_pool.selection_count
    ) THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'prepared Question Pool Selection count must match its Released Assignment Entry';
    END IF;

    INSERT INTO ple_private.question_pool_selection (
        question_pool_selection_id, assignment_attempt_id, assignment_entry_id,
        created_at, selected_question_count, reused_from_question_pool_selection_id
    )
    SELECT input.question_pool_selection_id, p_assignment_attempt_id, input.assignment_entry_id,
           v_now, jsonb_array_length(input.selected_candidates),
           input.reused_from_question_pool_selection_id
      FROM jsonb_to_recordset(p_question_pool_selections) AS input (
          question_pool_selection_id uuid,
          assignment_entry_id uuid,
          reused_from_question_pool_selection_id uuid,
          selected_candidates jsonb
      );

    INSERT INTO ple_private.question_pool_selected_candidate (
        question_pool_selection_id, question_pool_candidate_id, selection_position,
        question_id, revision_number
    )
    SELECT input.question_pool_selection_id, candidate.question_pool_candidate_id,
           candidate.selection_position, candidate.question_id, candidate.revision_number
      FROM jsonb_to_recordset(p_question_pool_selections) AS input (
          question_pool_selection_id uuid,
          assignment_entry_id uuid,
          reused_from_question_pool_selection_id uuid,
          selected_candidates jsonb
      )
      CROSS JOIN LATERAL jsonb_to_recordset(input.selected_candidates) AS candidate (
          question_pool_candidate_id uuid,
          selection_position integer,
          question_id text,
          revision_number integer
      );

    INSERT INTO ple_private.issued_question (
        issued_question_id, assignment_attempt_id, assignment_entry_id, question_id,
        revision_number, issued_position, point_value, scoring_rule, statistics_eligible,
        question_pool_selection_id, question_pool_candidate_id
    )
    SELECT input.issued_question_id, p_assignment_attempt_id, input.assignment_entry_id,
           input.question_id, input.revision_number, input.issued_position,
           entry.point_value, entry.scoring_rule,
           entry.scoring_rule = 'normal' AND entry.point_value > 0,
           input.question_pool_selection_id, input.question_pool_candidate_id
      FROM jsonb_to_recordset(p_issued_questions) AS input (
          issued_question_id uuid,
          assignment_entry_id uuid,
          issued_position integer,
          question_id text,
          revision_number integer,
          question_pool_selection_id uuid,
          question_pool_candidate_id uuid
      )
      JOIN ple_data.assignment_revision_entry AS entry
        ON entry.assignment_revision_id = v_assignment_revision_id
       AND entry.assignment_entry_id = input.assignment_entry_id;

    IF NOT EXISTS (
        SELECT 1
          FROM ple_private.issued_question AS issued_question
         WHERE issued_question.assignment_attempt_id = p_assignment_attempt_id
    ) OR EXISTS (
        WITH expected AS (
            SELECT fixed_question.assignment_entry_id, NULL::uuid AS question_pool_candidate_id,
                   fixed_question.question_id, fixed_question.revision_number
              FROM ple_data.assignment_revision_fixed_question AS fixed_question
             WHERE fixed_question.assignment_revision_id = v_assignment_revision_id
            UNION ALL
            SELECT selection.assignment_entry_id, candidate.question_pool_candidate_id,
                   candidate.question_id, candidate.revision_number
              FROM ple_private.question_pool_selection AS selection
              JOIN ple_private.question_pool_selected_candidate AS candidate
                ON candidate.question_pool_selection_id = selection.question_pool_selection_id
             WHERE selection.assignment_attempt_id = p_assignment_attempt_id
        ), actual AS (
            SELECT issued.assignment_entry_id, issued.question_pool_candidate_id,
                   issued.question_id, issued.revision_number
              FROM ple_private.issued_question AS issued
             WHERE issued.assignment_attempt_id = p_assignment_attempt_id
        )
        SELECT 1
          FROM (
              (SELECT * FROM expected EXCEPT SELECT * FROM actual)
              UNION ALL
              (SELECT * FROM actual EXCEPT SELECT * FROM expected)
          ) AS difference
    ) THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Issued Questions must cover the exact fixed pins and selected Question Pool candidates';
    END IF;

    RETURN QUERY SELECT p_assignment_attempt_id, v_attempt_number, false;
END
$$;

REVOKE ALL PRIVILEGES ON FUNCTION ple_private.start_assignment_attempt(
    uuid, uuid, uuid, jsonb, jsonb
) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_private.start_assignment_attempt(uuid, uuid, uuid, jsonb, jsonb)
    TO ple_api_owner;

RESET ROLE;

SET LOCAL ROLE ple_api_owner;

CREATE FUNCTION ple_api.start_assignment_attempt(
    p_assignment_attempt_id uuid,
    p_student_record_id uuid,
    p_assignment_id uuid,
    p_question_pool_selections jsonb,
    p_issued_questions jsonb
)
RETURNS TABLE (
    assignment_attempt_id uuid,
    attempt_number integer,
    resumed boolean
)
LANGUAGE sql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private
AS $$
    SELECT * FROM ple_private.start_assignment_attempt(
        p_assignment_attempt_id, p_student_record_id, p_assignment_id,
        p_question_pool_selections, p_issued_questions
    )
$$;

REVOKE ALL PRIVILEGES ON FUNCTION ple_api.start_assignment_attempt(
    uuid, uuid, uuid, jsonb, jsonb
) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_api.start_assignment_attempt(uuid, uuid, uuid, jsonb, jsonb)
    TO ple_app;

RESET ROLE;
