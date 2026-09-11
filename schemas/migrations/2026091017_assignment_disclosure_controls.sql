-- Persist the seven independent Assignment disclosure timings.  Existing
-- authored choices stay intact; only future Assignment defaults narrow
-- answer-bearing feedback.
DO $$
BEGIN
    IF current_user <> 'ple_migrator' THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'migration 2026091017 must run as ple_migrator';
    END IF;
END
$$;

SET LOCAL ROLE ple_data_owner;

ALTER TABLE ple_data.assignment
    ADD COLUMN feedback_submitted_response text NOT NULL DEFAULT 'after_submit'
        CHECK (feedback_submitted_response IN (
            'during_attempt', 'after_submit', 'after_due', 'after_close', 'never'
        ));
ALTER TABLE ple_data.assignment_revision
    ADD COLUMN feedback_submitted_response text NOT NULL DEFAULT 'after_submit'
        CHECK (feedback_submitted_response IN (
            'during_attempt', 'after_submit', 'after_due', 'after_close', 'never'
        ));

ALTER TABLE ple_data.assignment
    ALTER COLUMN feedback_question_feedback SET DEFAULT 'never',
    ALTER COLUMN feedback_question_answer SET DEFAULT 'never',
    ALTER COLUMN feedback_question_answer_explanation SET DEFAULT 'never';
ALTER TABLE ple_data.assignment_revision
    ALTER COLUMN feedback_question_feedback SET DEFAULT 'never',
    ALTER COLUMN feedback_question_answer SET DEFAULT 'never',
    ALTER COLUMN feedback_question_answer_explanation SET DEFAULT 'never';

CREATE OR REPLACE FUNCTION ple_data.enforce_assignment_edit()
RETURNS trigger LANGUAGE plpgsql SET search_path = pg_catalog, ple_data AS $$
BEGIN
    IF NEW.assignment_id <> OLD.assignment_id OR NEW.course_id <> OLD.course_id
       OR NEW.source_blueprint_course_reference_number <> OLD.source_blueprint_course_reference_number
       OR NEW.source_blueprint_revision_number <> OLD.source_blueprint_revision_number
       OR NEW.created_at <> OLD.created_at OR NEW.updated_at < OLD.updated_at THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Assignment identity and timestamps are immutable or forward-only';
    END IF;
    IF ROW(NEW.assignment_title, NEW.assignment_instructions, NEW.available_at, NEW.due_at,
           NEW.closes_at, NEW.assignment_attempt_time_limit_seconds, NEW.attempt_limit,
           NEW.late_work_rule, NEW.assignment_deadline_rule, NEW.assignment_completion_rule,
           NEW.assignment_completion_score_threshold, NEW.assignment_attempt_grade_rule,
           NEW.assignment_attempt_continuation_rule, NEW.max_additional_assignment_attempts,
           NEW.question_pool_reuse_rule, NEW.question_variation_rule,
           NEW.assignment_attempt_resume_rule, NEW.assignment_question_display_rule,
           NEW.assignment_navigation_rule, NEW.assignment_question_order_rule,
           NEW.live_demo_question_selection_version, NEW.feedback_score,
           NEW.feedback_per_item_correctness, NEW.feedback_submitted_response,
           NEW.feedback_question_feedback, NEW.feedback_question_answer,
           NEW.feedback_question_answer_explanation, NEW.feedback_class_statistics)
       IS DISTINCT FROM ROW(OLD.assignment_title, OLD.assignment_instructions, OLD.available_at,
           OLD.due_at, OLD.closes_at, OLD.assignment_attempt_time_limit_seconds,
           OLD.attempt_limit, OLD.late_work_rule, OLD.assignment_deadline_rule,
           OLD.assignment_completion_rule, OLD.assignment_completion_score_threshold,
           OLD.assignment_attempt_grade_rule, OLD.assignment_attempt_continuation_rule,
           OLD.max_additional_assignment_attempts, OLD.question_pool_reuse_rule,
           OLD.question_variation_rule, OLD.assignment_attempt_resume_rule,
           OLD.assignment_question_display_rule, OLD.assignment_navigation_rule,
           OLD.assignment_question_order_rule, OLD.live_demo_question_selection_version,
           OLD.feedback_score, OLD.feedback_per_item_correctness,
           OLD.feedback_submitted_response, OLD.feedback_question_feedback,
           OLD.feedback_question_answer, OLD.feedback_question_answer_explanation,
           OLD.feedback_class_statistics) THEN
        IF NEW.assignment_edit_number <> OLD.assignment_edit_number + 1 THEN
            RAISE EXCEPTION USING ERRCODE = '23514',
                MESSAGE = 'Assignment content changes must advance exactly one Assignment Edit Number';
        END IF;
    ELSIF NEW.assignment_edit_number <> OLD.assignment_edit_number THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Assignment Edit Number changes only with authored Assignment content';
    END IF;
    RETURN NEW;
END
$$;

RESET ROLE;
SET LOCAL ROLE ple_api_owner;

DROP FUNCTION ple_api.live_demo_assignment_workspace_rows(bigint, bigint);
CREATE FUNCTION ple_api.live_demo_assignment_workspace_rows(
    p_course_reference_number bigint,
    p_assignment_reference_number bigint
) RETURNS TABLE (
    reference_number bigint, assignment_edit_number bigint, assignment_status text,
    assignment_title text, assignment_instructions text, due_at_millis bigint,
    late_work_rule text, assignment_attempt_time_limit_seconds integer, attempt_limit integer,
    assignment_completion_rule text, assignment_completion_score_threshold double precision,
    assignment_attempt_grade_rule text, assignment_attempt_continuation_rule text,
    max_additional_assignment_attempts integer, question_pool_reuse_rule text,
    question_variation_rule text, assignment_attempt_resume_rule text,
    assignment_question_display_rule text, assignment_navigation_rule text,
    assignment_question_order_rule text, feedback_score text,
    feedback_per_item_correctness text, feedback_submitted_response text,
    feedback_question_feedback text, feedback_question_answer text,
    feedback_question_answer_explanation text, feedback_class_statistics text,
    question_id text, question_description text, question_index integer
) LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
    SELECT a.reference_number, a.assignment_edit_number, a.assignment_status,
           a.assignment_title, a.assignment_instructions,
           floor(extract(epoch FROM a.due_at) * 1000)::bigint, a.late_work_rule,
           a.assignment_attempt_time_limit_seconds, a.attempt_limit,
           a.assignment_completion_rule, a.assignment_completion_score_threshold,
           a.assignment_attempt_grade_rule, a.assignment_attempt_continuation_rule,
           a.max_additional_assignment_attempts, a.question_pool_reuse_rule,
           a.question_variation_rule, a.assignment_attempt_resume_rule,
           a.assignment_question_display_rule, a.assignment_navigation_rule,
           a.assignment_question_order_rule, a.feedback_score,
           a.feedback_per_item_correctness, a.feedback_submitted_response,
           a.feedback_question_feedback, a.feedback_question_answer,
           a.feedback_question_answer_explanation, a.feedback_class_statistics,
           q.question_id, m.question_description, q.question_index
      FROM ple_data.assignment AS a
      JOIN ple_data.course_instance AS c ON c.course_id = a.course_id
      LEFT JOIN ple_private.live_demo_assignment_question AS q ON q.assignment_id = a.assignment_id
      LEFT JOIN ple_data.published_question_metadata AS m ON m.question_id = q.question_id
     WHERE c.reference_number = p_course_reference_number
       AND a.reference_number = p_assignment_reference_number
       AND ple_api.current_session_account_is_course_instructor(c.course_id)
     ORDER BY q.question_index NULLS LAST
$$;

DROP FUNCTION ple_api.save_live_demo_assignment(
    bigint, bigint, bigint, text, text, text[], bigint, text, integer, integer,
    text, text, text, text, text, text, text, text, text, double precision, integer,
    text, text, text, text, text, text
);
CREATE FUNCTION ple_api.save_live_demo_assignment(
    p_course_reference_number bigint, p_assignment_reference_number bigint,
    p_expected_edit_number bigint, p_title text, p_instructions text, p_question_ids text[],
    p_due_at_millis bigint, p_late_work_rule text, p_time_limit integer, p_attempt_limit integer,
    p_completion text, p_grade text, p_continuation text, p_pool_reuse text, p_variation text,
    p_resume text, p_display text, p_navigation text, p_question_order text,
    p_threshold double precision, p_max_additional integer, p_feedback_score text,
    p_feedback_correctness text, p_feedback_response text, p_feedback_question text,
    p_feedback_answer text, p_feedback_explanation text, p_feedback_statistics text
) RETURNS TABLE (
    reference_number bigint, assignment_edit_number bigint, assignment_status text,
    assignment_title text, assignment_instructions text
) LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
DECLARE
    a ple_data.assignment%ROWTYPE;
    v_changed boolean;
    i integer;
    q text;
BEGIN
    -- ASVS 2.2.1 and 8.3.1: trusted persistence validates the closed timing
    -- vocabulary before changing the Instructor-authorized Assignment record.
    IF p_expected_edit_number <= 0 OR p_question_ids IS NULL OR cardinality(p_question_ids) > 25
       OR cardinality(p_question_ids) <> cardinality(ARRAY(
           SELECT DISTINCT value FROM unnest(p_question_ids) AS value
       ))
       OR p_late_work_rule NOT IN ('accept', 'mark_late', 'reject')
       OR p_time_limit <= 0 OR p_attempt_limit <= 0
       OR p_completion NOT IN ('answer_all', 'all_correct', 'score_at_least')
       OR p_grade NOT IN ('first', 'latest', 'highest', 'instructor_selected')
       OR p_continuation NOT IN ('unlimited', 'capped', 'closed')
       OR p_pool_reuse NOT IN ('reuse_selection', 'select_again')
       OR p_variation NOT IN ('reuse_variation', 'new_variation')
       OR p_resume NOT IN ('resumable', 'single_session')
       OR p_display NOT IN ('all_questions', 'one_question_at_a_time')
       OR p_navigation NOT IN ('free_navigation', 'forward_only')
       OR p_question_order NOT IN ('authored_order', 'shuffled')
       OR p_threshold IS NOT NULL AND (p_threshold < 0 OR p_threshold > 1)
       OR p_completion = 'score_at_least' AND p_threshold IS NULL
       OR p_continuation = 'capped' AND (p_max_additional IS NULL OR p_max_additional < 0)
       OR p_feedback_score NOT IN ('during_attempt', 'after_submit', 'after_due', 'after_close', 'never')
       OR p_feedback_correctness NOT IN ('during_attempt', 'after_submit', 'after_due', 'after_close', 'never')
       OR p_feedback_response NOT IN ('during_attempt', 'after_submit', 'after_due', 'after_close', 'never')
       OR p_feedback_question NOT IN ('during_attempt', 'after_submit', 'after_due', 'after_close', 'never')
       OR p_feedback_answer NOT IN ('during_attempt', 'after_submit', 'after_due', 'after_close', 'never')
       OR p_feedback_explanation NOT IN ('during_attempt', 'after_submit', 'after_due', 'after_close', 'never')
       OR p_feedback_statistics NOT IN ('during_attempt', 'after_submit', 'after_due', 'after_close', 'never') THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Assignment Workspace save arguments are invalid';
    END IF;

    SELECT assignment.* INTO a
      FROM ple_data.course_instance AS c
      JOIN ple_data.assignment AS assignment ON assignment.course_id = c.course_id
     WHERE c.reference_number = p_course_reference_number
       AND assignment.reference_number = p_assignment_reference_number
     FOR UPDATE OF assignment;
    IF NOT FOUND OR NOT ple_api.current_session_account_is_course_instructor(a.course_id) THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Assignment Workspace requires a current Instructor Course Membership';
    END IF;
    IF a.assignment_edit_number <> p_expected_edit_number OR a.assignment_status <> 'unreleased' THEN
        RAISE EXCEPTION USING ERRCODE = '40001', MESSAGE = 'Assignment Workspace changed';
    END IF;

    FOREACH q IN ARRAY p_question_ids LOOP
        IF NOT EXISTS (
            SELECT 1 FROM ple_api.published_question_summary AS s
            JOIN LATERAL (
                SELECT e.availability FROM ple_data.question_revision_availability_event AS e
                 WHERE e.question_id = s.question_id
                   AND e.revision_number = s.latest_question_revision_number
                 ORDER BY e.occurred_at DESC, e.event_id DESC LIMIT 1
            ) AS x ON x.availability = 'available'
            WHERE s.question_id = q
        ) THEN
            RAISE EXCEPTION USING ERRCODE = '22023',
                MESSAGE = 'Assignment Workspace selects an unavailable Published Question';
        END IF;
    END LOOP;
    SELECT COALESCE(array_agg(question_id ORDER BY question_index), ARRAY[]::text[])
           IS DISTINCT FROM p_question_ids INTO v_changed
      FROM ple_private.live_demo_assignment_question
     WHERE assignment_id = a.assignment_id;
    IF v_changed THEN
        DELETE FROM ple_private.live_demo_assignment_question WHERE assignment_id = a.assignment_id;
        FOR i IN 1..COALESCE(cardinality(p_question_ids), 0) LOOP
            INSERT INTO ple_private.live_demo_assignment_question
            VALUES (a.assignment_id, p_question_ids[i], i - 1);
        END LOOP;
    END IF;

    UPDATE ple_data.assignment AS updated_assignment
       SET assignment_title = p_title, assignment_instructions = p_instructions,
           due_at = CASE WHEN p_due_at_millis IS NULL THEN NULL
                         ELSE to_timestamp(p_due_at_millis::double precision / 1000) END,
           late_work_rule = p_late_work_rule, assignment_attempt_time_limit_seconds = p_time_limit,
           attempt_limit = p_attempt_limit, assignment_completion_rule = p_completion,
           assignment_completion_score_threshold = p_threshold, assignment_attempt_grade_rule = p_grade,
           assignment_attempt_continuation_rule = p_continuation,
           max_additional_assignment_attempts = p_max_additional,
           question_pool_reuse_rule = p_pool_reuse, question_variation_rule = p_variation,
           assignment_attempt_resume_rule = p_resume,
           assignment_question_display_rule = p_display, assignment_navigation_rule = p_navigation,
           assignment_question_order_rule = p_question_order, feedback_score = p_feedback_score,
           feedback_per_item_correctness = p_feedback_correctness,
           feedback_submitted_response = p_feedback_response,
           feedback_question_feedback = p_feedback_question, feedback_question_answer = p_feedback_answer,
           feedback_question_answer_explanation = p_feedback_explanation,
           feedback_class_statistics = p_feedback_statistics,
           assignment_edit_number = updated_assignment.assignment_edit_number + 1,
           live_demo_question_selection_version = updated_assignment.live_demo_question_selection_version
               + CASE WHEN v_changed THEN 1 ELSE 0 END,
           updated_at = transaction_timestamp()
     WHERE updated_assignment.assignment_id = a.assignment_id
     RETURNING updated_assignment.reference_number, updated_assignment.assignment_edit_number,
               updated_assignment.assignment_status, updated_assignment.assignment_title,
               updated_assignment.assignment_instructions
          INTO reference_number, assignment_edit_number, assignment_status, assignment_title,
               assignment_instructions;
    RETURN NEXT;
END
$$;

CREATE OR REPLACE FUNCTION ple_api.release_live_demo_assignment(
    p_assignment_revision_id uuid, p_course_reference_number bigint,
    p_assignment_reference_number bigint, p_expected_edit_number bigint
) RETURNS TABLE(reference_number bigint, revision_number bigint)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
DECLARE
    a ple_data.assignment%ROWTYPE;
    cid uuid;
    sid uuid;
    next_revision bigint;
    selected record;
    entry uuid;
    v_issue text;
BEGIN
    SELECT assignment.* INTO a
      FROM ple_data.course_instance AS c
      JOIN ple_data.assignment AS assignment ON assignment.course_id = c.course_id
     WHERE c.reference_number = p_course_reference_number
       AND assignment.reference_number = p_assignment_reference_number
     FOR UPDATE OF assignment;
    cid := a.course_id;
    IF NOT FOUND OR NOT ple_api.current_session_account_is_course_instructor(cid) THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Assignment release requires a current Instructor Course Membership';
    END IF;
    IF a.assignment_edit_number <> p_expected_edit_number OR a.assignment_status <> 'unreleased' THEN
        RAISE EXCEPTION USING ERRCODE = '40001', MESSAGE = 'Assignment Workspace changed';
    END IF;
    SELECT validation.issue INTO v_issue
      FROM ple_api.validate_live_demo_assignment_release(
          p_course_reference_number, p_assignment_reference_number
      ) AS validation LIMIT 1;
    IF v_issue IS NOT NULL THEN
        RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Assignment release validation failed';
    END IF;
    SELECT schedule.course_schedule_revision_id INTO sid
      FROM ple_data.course_schedule_revision AS schedule
     WHERE schedule.course_id = cid ORDER BY schedule.revision_number DESC LIMIT 1;
    SELECT COALESCE(max(revision.revision_number), 0) + 1 INTO next_revision
      FROM ple_data.assignment_revision AS revision WHERE revision.assignment_id = a.assignment_id;

    INSERT INTO ple_data.assignment_revision(
        assignment_revision_id, assignment_id, course_id, course_schedule_revision_id,
        revision_number, assignment_title, assignment_instructions, available_at, due_at, closes_at,
        assignment_attempt_time_limit_seconds, attempt_limit, late_work_rule, assignment_deadline_rule,
        assignment_completion_rule, assignment_completion_score_threshold,
        assignment_attempt_grade_rule, assignment_attempt_continuation_rule,
        max_additional_assignment_attempts, question_pool_reuse_rule, question_variation_rule,
        assignment_attempt_resume_rule, assignment_question_display_rule, assignment_navigation_rule,
        assignment_question_order_rule, feedback_score, feedback_per_item_correctness,
        feedback_submitted_response, feedback_question_feedback, feedback_question_answer,
        feedback_question_answer_explanation, feedback_class_statistics, created_at
    ) SELECT
        p_assignment_revision_id, assignment_id, course_id, sid, next_revision,
        assignment_title, assignment_instructions, available_at, due_at, closes_at,
        assignment_attempt_time_limit_seconds, attempt_limit, late_work_rule, assignment_deadline_rule,
        assignment_completion_rule, assignment_completion_score_threshold,
        assignment_attempt_grade_rule, assignment_attempt_continuation_rule,
        max_additional_assignment_attempts, question_pool_reuse_rule, question_variation_rule,
        assignment_attempt_resume_rule, assignment_question_display_rule, assignment_navigation_rule,
        assignment_question_order_rule, feedback_score, feedback_per_item_correctness,
        feedback_submitted_response, feedback_question_feedback, feedback_question_answer,
        feedback_question_answer_explanation, feedback_class_statistics, transaction_timestamp()
      FROM ple_data.assignment WHERE assignment_id = a.assignment_id;
    FOR selected IN
        SELECT q.question_id, q.question_index, s.latest_question_revision_number
          FROM ple_private.live_demo_assignment_question AS q
          JOIN ple_api.published_question_summary AS s ON s.question_id = q.question_id
         WHERE q.assignment_id = a.assignment_id ORDER BY q.question_index
    LOOP
        entry := gen_random_uuid();
        INSERT INTO ple_data.assignment_revision_entry VALUES(
            p_assignment_revision_id, entry, selected.question_index, 'fixed_question',
            'available', 'normal', 1, NULL, NULL, NULL
        );
        INSERT INTO ple_data.assignment_revision_fixed_question VALUES(
            p_assignment_revision_id, entry, selected.question_id,
            selected.latest_question_revision_number
        );
    END LOOP;
    UPDATE ple_data.assignment
       SET assignment_status = 'released', released_assignment_revision_id = p_assignment_revision_id,
           updated_at = transaction_timestamp()
     WHERE assignment_id = a.assignment_id;
    reference_number := a.reference_number;
    revision_number := next_revision;
    RETURN NEXT;
END
$$;

REVOKE ALL PRIVILEGES ON FUNCTION ple_api.live_demo_assignment_workspace_rows(bigint, bigint),
    ple_api.save_live_demo_assignment(
        bigint, bigint, bigint, text, text, text[], bigint, text, integer, integer,
        text, text, text, text, text, text, text, text, text, double precision, integer,
        text, text, text, text, text, text, text
    ),
    ple_api.release_live_demo_assignment(uuid, bigint, bigint, bigint) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_api.live_demo_assignment_workspace_rows(bigint, bigint),
    ple_api.save_live_demo_assignment(
        bigint, bigint, bigint, text, text, text[], bigint, text, integer, integer,
        text, text, text, text, text, text, text, text, text, double precision, integer,
        text, text, text, text, text, text, text
    ),
    ple_api.release_live_demo_assignment(uuid, bigint, bigint, bigint) TO ple_app;

RESET ROLE;
