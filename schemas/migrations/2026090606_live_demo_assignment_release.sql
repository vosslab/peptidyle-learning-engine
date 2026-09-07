-- M10 minimal Course-owned Assignment Workspace and immutable release.
--
-- This slice deliberately owns only fixed Available Published Questions.  It
-- neither creates Student work nor defines delivery/access policy.

SET LOCAL ROLE ple_data_owner;

ALTER TABLE ple_data.assignment
    ADD COLUMN reference_number bigint GENERATED ALWAYS AS IDENTITY,
    ADD COLUMN live_demo_question_selection_version bigint NOT NULL DEFAULT 1
        CHECK (live_demo_question_selection_version > 0),
    ALTER COLUMN reference_number SET NOT NULL,
    ADD CONSTRAINT assignment_reference_number_is_positive CHECK (reference_number > 0),
    ADD CONSTRAINT assignment_reference_number_unique UNIQUE (reference_number);

-- ASVS 2.3.1/2.3.3: the initial M10 fixed-question selection is ordered,
-- atomic authored Assignment content. Its private rows stay replaceable while
-- this positive version lets the existing exact Assignment Edit Number
-- invariant observe selection-only saves without a workflow bypass.
CREATE OR REPLACE FUNCTION ple_data.enforce_assignment_edit()
RETURNS trigger LANGUAGE plpgsql SET search_path = pg_catalog, ple_data AS $$
BEGIN
    IF NEW.assignment_id <> OLD.assignment_id
       OR NEW.course_id <> OLD.course_id
       OR NEW.source_blueprint_course_reference_number <> OLD.source_blueprint_course_reference_number
       OR NEW.source_blueprint_revision_number <> OLD.source_blueprint_revision_number
       OR NEW.created_at <> OLD.created_at
       OR NEW.updated_at < OLD.updated_at THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Assignment identity and timestamps are immutable or forward-only';
    END IF;
    IF ROW(
        NEW.assignment_title, NEW.assignment_instructions,
        NEW.available_at, NEW.due_at, NEW.closes_at,
        NEW.assignment_attempt_time_limit_seconds, NEW.attempt_limit,
        NEW.late_work_rule, NEW.assignment_deadline_rule,
        NEW.assignment_completion_rule, NEW.assignment_completion_score_threshold,
        NEW.assignment_attempt_grade_rule, NEW.assignment_attempt_continuation_rule,
        NEW.max_additional_assignment_attempts, NEW.question_pool_reuse_rule,
        NEW.question_variation_rule, NEW.assignment_attempt_resume_rule,
        NEW.assignment_question_display_rule, NEW.assignment_navigation_rule,
        NEW.assignment_question_order_rule, NEW.live_demo_question_selection_version
    ) IS DISTINCT FROM ROW(
        OLD.assignment_title, OLD.assignment_instructions,
        OLD.available_at, OLD.due_at, OLD.closes_at,
        OLD.assignment_attempt_time_limit_seconds, OLD.attempt_limit,
        OLD.late_work_rule, OLD.assignment_deadline_rule,
        OLD.assignment_completion_rule, OLD.assignment_completion_score_threshold,
        OLD.assignment_attempt_grade_rule, OLD.assignment_attempt_continuation_rule,
        OLD.max_additional_assignment_attempts, OLD.question_pool_reuse_rule,
        OLD.question_variation_rule, OLD.assignment_attempt_resume_rule,
        OLD.assignment_question_display_rule, OLD.assignment_navigation_rule,
        OLD.assignment_question_order_rule, OLD.live_demo_question_selection_version
    ) THEN
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

-- ASVS 8.2.1/8.2.2: deferred immutable-entry shape checks are a non-public
-- invariant over protected snapshot tables. They run as their data owner,
-- avoiding broad API reads while retaining exact released-entry validation.
ALTER FUNCTION ple_data.validate_assignment_revision_entry_shape() SECURITY DEFINER;

GRANT SELECT, INSERT, UPDATE ON TABLE ple_data.assignment, ple_data.assignment_revision,
    ple_data.assignment_revision_entry, ple_data.assignment_revision_fixed_question
TO ple_api_owner;
-- The M10 Assignment Reference is database-issued, so the defining API role
-- needs the identity sequence capability in addition to Assignment INSERT.
GRANT USAGE ON SEQUENCE ple_data.assignment_reference_number_seq TO ple_api_owner;
CREATE POLICY assignment_api_owner_live_demo_m10
    ON ple_data.assignment FOR ALL TO ple_api_owner USING (true) WITH CHECK (true);
CREATE POLICY assignment_revision_api_owner_live_demo_m10
    ON ple_data.assignment_revision FOR ALL TO ple_api_owner USING (true) WITH CHECK (true);
CREATE POLICY assignment_revision_entry_api_owner_live_demo_m10
    ON ple_data.assignment_revision_entry FOR INSERT TO ple_api_owner WITH CHECK (true);
CREATE POLICY assignment_revision_fixed_question_api_owner_live_demo_m10
    ON ple_data.assignment_revision_fixed_question FOR INSERT TO ple_api_owner WITH CHECK (true);

RESET ROLE;

SET LOCAL ROLE ple_private_owner;

CREATE TABLE ple_private.live_demo_assignment_question (
    assignment_id uuid NOT NULL REFERENCES ple_data.assignment (assignment_id),
    question_id text NOT NULL REFERENCES ple_data.published_question (question_id),
    question_index integer NOT NULL CHECK (question_index >= 0),
    PRIMARY KEY (assignment_id, question_id),
    UNIQUE (assignment_id, question_index)
);
ALTER TABLE ple_private.live_demo_assignment_question ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.live_demo_assignment_question FORCE ROW LEVEL SECURITY;
REVOKE ALL PRIVILEGES ON TABLE ple_private.live_demo_assignment_question FROM PUBLIC;
GRANT SELECT, INSERT, DELETE ON TABLE ple_private.live_demo_assignment_question TO ple_api_owner;
CREATE POLICY live_demo_assignment_question_api_owner_m10
    ON ple_private.live_demo_assignment_question FOR ALL TO ple_api_owner USING (true) WITH CHECK (true);

RESET ROLE;

SET LOCAL ROLE ple_api_owner;

CREATE FUNCTION ple_api.live_demo_assignment_workspace_rows(
    p_course_reference_number bigint,
    p_assignment_reference_number bigint
)
RETURNS TABLE (
    reference_number bigint, assignment_edit_number bigint, assignment_status text,
    assignment_title text, assignment_instructions text, due_at_millis bigint, late_work_rule text,
    question_id text,
    question_description text, question_index integer
)
LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
    SELECT assignment.reference_number, assignment.assignment_edit_number, assignment.assignment_status,
           assignment.assignment_title, assignment.assignment_instructions,
           floor(extract(epoch FROM assignment.due_at) * 1000)::bigint,
           assignment.late_work_rule,
           selected.question_id, metadata.question_description, selected.question_index
      FROM ple_data.assignment AS assignment
      JOIN ple_data.course_instance AS course ON course.course_id = assignment.course_id
      LEFT JOIN ple_private.live_demo_assignment_question AS selected
        ON selected.assignment_id = assignment.assignment_id
      LEFT JOIN ple_data.published_question_metadata AS metadata
        ON metadata.question_id = selected.question_id
     WHERE course.reference_number = p_course_reference_number
       AND assignment.reference_number = p_assignment_reference_number
       AND ple_api.current_session_account_is_course_instructor(course.course_id)
     ORDER BY selected.question_index NULLS LAST
$$;

CREATE FUNCTION ple_api.list_live_demo_assignment_picker(p_course_reference_number bigint)
RETURNS TABLE (question_id text, question_description text)
LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data AS $$
    SELECT summary.question_id, summary.question_description
      FROM ple_data.course_instance AS course
      CROSS JOIN ple_api.published_question_summary AS summary
      JOIN LATERAL (
          SELECT event.availability
            FROM ple_data.question_revision_availability_event AS event
           WHERE event.question_id = summary.question_id
             AND event.revision_number = summary.latest_question_revision_number
           ORDER BY event.occurred_at DESC, event.event_id DESC LIMIT 1
      ) AS availability ON availability.availability = 'available'
     WHERE course.reference_number = p_course_reference_number
       AND ple_api.current_session_account_is_course_instructor(course.course_id)
     ORDER BY summary.question_title, summary.question_id
     LIMIT 100
$$;

-- The Assignment Workspace resolves and projects local policy through the
-- exact latest Course Schedule Revision that Assignment Release snapshots.
CREATE FUNCTION ple_api.load_live_demo_assignment_course_term(p_course_reference_number bigint)
RETURNS TABLE (term_starts_on date, term_ends_on date, course_time_zone text)
LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data AS $$
    SELECT schedule.term_starts_on, schedule.term_ends_on, schedule.course_time_zone
      FROM ple_data.course_instance AS course
      JOIN LATERAL (
          SELECT revision.term_starts_on, revision.term_ends_on, revision.course_time_zone
            FROM ple_data.course_schedule_revision AS revision
           WHERE revision.course_id = course.course_id
           ORDER BY revision.revision_number DESC
           LIMIT 1
      ) AS schedule ON true
     WHERE course.reference_number = p_course_reference_number
       AND ple_api.current_session_account_is_course_instructor(course.course_id)
$$;

CREATE FUNCTION ple_api.create_live_demo_assignment(
    p_assignment_id uuid, p_course_reference_number bigint, p_title text, p_instructions text
)
RETURNS TABLE (reference_number bigint, assignment_edit_number bigint, assignment_status text,
               assignment_title text, assignment_instructions text)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data AS $$
DECLARE v_course ple_data.course_instance%ROWTYPE; v_origin ple_data.course_origin%ROWTYPE;
BEGIN
    IF p_assignment_id IS NULL OR p_title IS NULL OR p_instructions IS NULL THEN
        RAISE EXCEPTION USING ERRCODE='22023', MESSAGE='Assignment Workspace arguments are invalid';
    END IF;
    SELECT course.* INTO v_course
      FROM ple_data.course_instance AS course
     WHERE course.reference_number = p_course_reference_number;
    IF NOT FOUND OR NOT ple_api.current_session_account_is_course_instructor(v_course.course_id) THEN
        RAISE EXCEPTION USING ERRCODE='42501', MESSAGE='Assignment Workspace requires a current Instructor Course Membership';
    END IF;
    SELECT * INTO v_origin FROM ple_data.course_origin WHERE course_id=v_course.course_id;
    INSERT INTO ple_data.assignment (
        assignment_id, course_id, source_blueprint_course_reference_number, source_blueprint_revision_number,
        created_at, updated_at, assignment_edit_number, assignment_title, assignment_instructions,
        available_at, due_at, closes_at, assignment_attempt_time_limit_seconds, attempt_limit,
        late_work_rule, assignment_deadline_rule, assignment_completion_rule,
        assignment_completion_score_threshold, assignment_attempt_grade_rule,
        assignment_attempt_continuation_rule, max_additional_assignment_attempts,
        question_pool_reuse_rule, question_variation_rule, assignment_attempt_resume_rule, assignment_question_display_rule,
        assignment_navigation_rule, assignment_question_order_rule, assignment_status
    ) VALUES (
        p_assignment_id, v_course.course_id, v_origin.blueprint_course_reference_number, v_origin.blueprint_revision_number,
        transaction_timestamp(), transaction_timestamp(), 1, p_title, p_instructions,
        NULL, NULL, NULL, NULL, NULL, 'accept', 'auto_submit', 'answer_all', NULL,
        'latest', 'unlimited', NULL, 'reuse_selection', 'reuse_variation', 'resumable', 'all_questions',
        'free_navigation', 'authored_order', 'unreleased'
    ) RETURNING assignment.reference_number, assignment.assignment_edit_number, assignment.assignment_status,
        assignment.assignment_title, assignment.assignment_instructions
    INTO reference_number, assignment_edit_number, assignment_status, assignment_title, assignment_instructions;
    RETURN NEXT;
END $$;

CREATE FUNCTION ple_api.save_live_demo_assignment(
    p_course_reference_number bigint, p_assignment_reference_number bigint, p_expected_edit_number bigint,
    p_title text, p_instructions text, p_question_ids text[],
    p_due_at_millis bigint, p_late_work_rule text
)
RETURNS TABLE (reference_number bigint, assignment_edit_number bigint, assignment_status text,
               assignment_title text, assignment_instructions text)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
DECLARE v_assignment ple_data.assignment%ROWTYPE; v_course_id uuid; v_question_id text; v_index integer;
        v_selection_changed boolean; v_authored_changed boolean;
BEGIN
    IF p_expected_edit_number <= 0 OR p_question_ids IS NULL OR cardinality(p_question_ids) > 25
       OR p_late_work_rule NOT IN ('accept', 'mark_late', 'reject')
       OR cardinality(p_question_ids) <> cardinality(ARRAY(SELECT DISTINCT value FROM unnest(p_question_ids) AS value)) THEN
        RAISE EXCEPTION USING ERRCODE='22023', MESSAGE='Assignment Workspace save arguments are invalid';
    END IF;
    SELECT assignment.* INTO v_assignment
      FROM ple_data.course_instance AS course JOIN ple_data.assignment AS assignment ON assignment.course_id=course.course_id
     WHERE course.reference_number=p_course_reference_number AND assignment.reference_number=p_assignment_reference_number FOR UPDATE OF assignment;
    v_course_id := v_assignment.course_id;
    IF NOT FOUND OR NOT ple_api.current_session_account_is_course_instructor(v_course_id) THEN
        RAISE EXCEPTION USING ERRCODE='42501', MESSAGE='Assignment Workspace requires a current Instructor Course Membership';
    END IF;
    IF v_assignment.assignment_edit_number <> p_expected_edit_number OR v_assignment.assignment_status <> 'unreleased' THEN
        RAISE EXCEPTION USING ERRCODE='40001', MESSAGE='Assignment Workspace changed';
    END IF;
    FOREACH v_question_id IN ARRAY p_question_ids LOOP
        IF NOT EXISTS (
            SELECT 1 FROM ple_api.published_question_summary AS summary
             JOIN LATERAL (SELECT event.availability FROM ple_data.question_revision_availability_event AS event WHERE event.question_id=summary.question_id AND event.revision_number=summary.latest_question_revision_number ORDER BY event.occurred_at DESC,event.event_id DESC LIMIT 1) availability
               ON availability.availability='available'
             WHERE summary.question_id=v_question_id
        ) THEN RAISE EXCEPTION USING ERRCODE='22023', MESSAGE='Assignment Workspace selects an unavailable Published Question'; END IF;
    END LOOP;
    SELECT COALESCE(array_agg(selected.question_id ORDER BY selected.question_index), ARRAY[]::text[])
      IS DISTINCT FROM p_question_ids
      INTO v_selection_changed
      FROM ple_private.live_demo_assignment_question AS selected
     WHERE selected.assignment_id = v_assignment.assignment_id;
    v_authored_changed := v_selection_changed
       OR v_assignment.assignment_title IS DISTINCT FROM p_title
       OR v_assignment.assignment_instructions IS DISTINCT FROM p_instructions
       OR v_assignment.due_at IS DISTINCT FROM CASE
            WHEN p_due_at_millis IS NULL THEN NULL
            ELSE pg_catalog.to_timestamp(p_due_at_millis::double precision / 1000)
          END
       OR v_assignment.late_work_rule IS DISTINCT FROM p_late_work_rule;
    IF NOT v_authored_changed THEN
        reference_number := v_assignment.reference_number;
        assignment_edit_number := v_assignment.assignment_edit_number;
        assignment_status := v_assignment.assignment_status;
        assignment_title := v_assignment.assignment_title;
        assignment_instructions := v_assignment.assignment_instructions;
        RETURN NEXT;
        RETURN;
    END IF;
    IF v_selection_changed THEN
        DELETE FROM ple_private.live_demo_assignment_question WHERE assignment_id=v_assignment.assignment_id;
        FOR v_index IN 1..COALESCE(cardinality(p_question_ids), 0) LOOP
            INSERT INTO ple_private.live_demo_assignment_question (assignment_id, question_id, question_index)
            VALUES (v_assignment.assignment_id, p_question_ids[v_index], v_index-1);
        END LOOP;
    END IF;
    -- ASVS 2.3.1/2.3.3: one direct-Instructor CAS save atomically advances
    -- authored content, including resolved Due at and its Late Work Rule.
    UPDATE ple_data.assignment AS assignment
       SET assignment_title = p_title, assignment_instructions = p_instructions,
           assignment_edit_number = assignment.assignment_edit_number + 1,
           live_demo_question_selection_version = assignment.live_demo_question_selection_version
               + CASE WHEN v_selection_changed THEN 1 ELSE 0 END,
           due_at = CASE WHEN p_due_at_millis IS NULL THEN NULL
                ELSE pg_catalog.to_timestamp(p_due_at_millis::double precision / 1000) END,
           late_work_rule = p_late_work_rule,
           updated_at = transaction_timestamp()
     WHERE assignment.assignment_id = v_assignment.assignment_id
     RETURNING assignment.reference_number, assignment.assignment_edit_number, assignment.assignment_status,
        assignment.assignment_title, assignment.assignment_instructions
     INTO reference_number, assignment_edit_number, assignment_status, assignment_title, assignment_instructions;
    RETURN NEXT;
END $$;

CREATE FUNCTION ple_api.validate_live_demo_assignment_release(p_course_reference_number bigint, p_assignment_reference_number bigint)
RETURNS TABLE (issue text)
LANGUAGE plpgsql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
DECLARE v_assignment_id uuid; v_course_id uuid;
BEGIN
    SELECT assignment.assignment_id, course.course_id INTO v_assignment_id, v_course_id FROM ple_data.course_instance course JOIN ple_data.assignment assignment ON assignment.course_id=course.course_id WHERE course.reference_number=p_course_reference_number AND assignment.reference_number=p_assignment_reference_number;
    IF NOT FOUND OR NOT ple_api.current_session_account_is_course_instructor(v_course_id) THEN RAISE EXCEPTION USING ERRCODE='42501', MESSAGE='Assignment release requires a current Instructor Course Membership'; END IF;
    IF NOT EXISTS (SELECT 1 FROM ple_private.live_demo_assignment_question WHERE assignment_id=v_assignment_id) THEN issue := 'no_published_questions'; RETURN NEXT; END IF;
    IF EXISTS (SELECT 1 FROM ple_private.live_demo_assignment_question selected LEFT JOIN ple_api.published_question_summary summary ON summary.question_id=selected.question_id LEFT JOIN LATERAL (SELECT event.availability FROM ple_data.question_revision_availability_event event WHERE event.question_id=summary.question_id AND event.revision_number=summary.latest_question_revision_number ORDER BY event.occurred_at DESC,event.event_id DESC LIMIT 1) availability ON true WHERE selected.assignment_id=v_assignment_id AND (summary.question_id IS NULL OR availability.availability IS DISTINCT FROM 'available')) THEN issue := 'question_unavailable'; RETURN NEXT; END IF;
END $$;

CREATE FUNCTION ple_api.release_live_demo_assignment(
    p_assignment_revision_id uuid, p_course_reference_number bigint, p_assignment_reference_number bigint, p_expected_edit_number bigint
)
RETURNS TABLE (reference_number bigint, revision_number bigint)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
DECLARE v_assignment ple_data.assignment%ROWTYPE; v_course_id uuid; v_schedule_id uuid; v_next_revision bigint; v_selected record; v_entry_id uuid; v_issue text;
BEGIN
    SELECT assignment.* INTO v_assignment FROM ple_data.course_instance course JOIN ple_data.assignment assignment ON assignment.course_id=course.course_id WHERE course.reference_number=p_course_reference_number AND assignment.reference_number=p_assignment_reference_number FOR UPDATE OF assignment;
    v_course_id := v_assignment.course_id;
    IF NOT FOUND OR NOT ple_api.current_session_account_is_course_instructor(v_course_id) THEN RAISE EXCEPTION USING ERRCODE='42501', MESSAGE='Assignment release requires a current Instructor Course Membership'; END IF;
    IF v_assignment.assignment_edit_number <> p_expected_edit_number OR v_assignment.assignment_status <> 'unreleased' THEN RAISE EXCEPTION USING ERRCODE='40001', MESSAGE='Assignment Workspace changed'; END IF;
    SELECT issue INTO v_issue FROM ple_api.validate_live_demo_assignment_release(p_course_reference_number,p_assignment_reference_number) LIMIT 1;
    IF v_issue IS NOT NULL THEN RAISE EXCEPTION USING ERRCODE='22023', MESSAGE='Assignment release validation failed'; END IF;
    SELECT schedule.course_schedule_revision_id INTO v_schedule_id
      FROM ple_data.course_schedule_revision AS schedule
     WHERE schedule.course_id = v_course_id
     ORDER BY schedule.revision_number DESC LIMIT 1;
    SELECT COALESCE(max(revision.revision_number), 0) + 1 INTO v_next_revision
      FROM ple_data.assignment_revision AS revision
     WHERE revision.assignment_id = v_assignment.assignment_id;
    INSERT INTO ple_data.assignment_revision (
        assignment_revision_id, assignment_id, course_id, course_schedule_revision_id, revision_number,
        assignment_title, assignment_instructions, available_at, due_at, closes_at,
        assignment_attempt_time_limit_seconds, attempt_limit, late_work_rule, assignment_deadline_rule,
        assignment_completion_rule, assignment_completion_score_threshold, assignment_attempt_grade_rule,
        assignment_attempt_continuation_rule, max_additional_assignment_attempts, question_pool_reuse_rule,
        question_variation_rule, assignment_attempt_resume_rule, assignment_question_display_rule,
        assignment_navigation_rule, assignment_question_order_rule, created_at
    ) SELECT
        p_assignment_revision_id, assignment_id, course_id, v_schedule_id, v_next_revision,
        assignment_title, assignment_instructions, available_at, due_at, closes_at,
        assignment_attempt_time_limit_seconds, attempt_limit, late_work_rule, assignment_deadline_rule,
        assignment_completion_rule, assignment_completion_score_threshold, assignment_attempt_grade_rule,
        assignment_attempt_continuation_rule, max_additional_assignment_attempts, question_pool_reuse_rule,
        question_variation_rule, assignment_attempt_resume_rule, assignment_question_display_rule,
        assignment_navigation_rule, assignment_question_order_rule, transaction_timestamp()
      FROM ple_data.assignment
     WHERE assignment_id = v_assignment.assignment_id;
    FOR v_selected IN SELECT selected.question_id, selected.question_index, summary.latest_question_revision_number FROM ple_private.live_demo_assignment_question selected JOIN ple_api.published_question_summary summary ON summary.question_id=selected.question_id WHERE selected.assignment_id=v_assignment.assignment_id ORDER BY selected.question_index LOOP
        v_entry_id:=pg_catalog.gen_random_uuid();
        INSERT INTO ple_data.assignment_revision_entry VALUES (p_assignment_revision_id,v_entry_id,v_selected.question_index,'fixed_question','available','normal',1,NULL,NULL,NULL);
        INSERT INTO ple_data.assignment_revision_fixed_question VALUES (p_assignment_revision_id,v_entry_id,v_selected.question_id,v_selected.latest_question_revision_number);
    END LOOP;
    UPDATE ple_data.assignment SET assignment_status='released', released_assignment_revision_id=p_assignment_revision_id, updated_at=transaction_timestamp() WHERE assignment_id=v_assignment.assignment_id;
    reference_number:=v_assignment.reference_number; revision_number:=v_next_revision; RETURN NEXT;
END $$;

REVOKE ALL PRIVILEGES ON FUNCTION ple_api.live_demo_assignment_workspace_rows(bigint,bigint), ple_api.list_live_demo_assignment_picker(bigint), ple_api.load_live_demo_assignment_course_term(bigint), ple_api.create_live_demo_assignment(uuid,bigint,text,text), ple_api.save_live_demo_assignment(bigint,bigint,bigint,text,text,text[],bigint,text), ple_api.validate_live_demo_assignment_release(bigint,bigint), ple_api.release_live_demo_assignment(uuid,bigint,bigint,bigint) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_api.live_demo_assignment_workspace_rows(bigint,bigint), ple_api.list_live_demo_assignment_picker(bigint), ple_api.load_live_demo_assignment_course_term(bigint), ple_api.create_live_demo_assignment(uuid,bigint,text,text), ple_api.save_live_demo_assignment(bigint,bigint,bigint,text,text,text[],bigint,text), ple_api.validate_live_demo_assignment_release(bigint,bigint), ple_api.release_live_demo_assignment(uuid,bigint,bigint,bigint) TO ple_app;

RESET ROLE;
