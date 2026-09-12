-- Student Work roots.  An Assignment Attempt retains the effective facts that
-- later resume, submission, grading, history, disclosure, and statistics reads
-- need; current Assignment rows remain the authority only when an Attempt starts.

-- The private Student Work root owns FKs to these public Course/Assignment and
-- immutable Question Revision identities.  Current Assignment children are
-- intentionally evidence values below, not FK targets.
SET LOCAL ROLE ple_data_owner;
GRANT REFERENCES ON TABLE ple_data.student_record, ple_data.assignment,
    ple_data.question_revision TO ple_private_owner;
SET LOCAL ROLE ple_private_owner;

CREATE TABLE ple_private.student_assignment_accommodation (
    accommodation_id uuid PRIMARY KEY,
    student_record_id uuid NOT NULL REFERENCES ple_data.student_record(student_record_id),
    assignment_id uuid NOT NULL REFERENCES ple_data.assignment(assignment_id),
    available_at timestamptz,
    due_at timestamptz,
    closes_at timestamptz,
    assignment_attempt_time_limit_seconds integer,
    attempt_limit integer,
    created_at timestamptz NOT NULL,
    accommodation_edit_number bigint NOT NULL DEFAULT 1
        CHECK (accommodation_edit_number > 0),
    CHECK ((available_at IS NULL OR due_at IS NULL OR available_at <= due_at)
       AND (due_at IS NULL OR closes_at IS NULL OR due_at <= closes_at)),
    CHECK (assignment_attempt_time_limit_seconds IS NULL
       OR assignment_attempt_time_limit_seconds > 0),
    CHECK (attempt_limit IS NULL OR attempt_limit > 0),
    UNIQUE (accommodation_id, student_record_id, assignment_id),
    UNIQUE (student_record_id, assignment_id)
);

CREATE TABLE ple_private.assignment_attempt (
    assignment_attempt_id uuid PRIMARY KEY,
    reference_number bigint GENERATED ALWAYS AS IDENTITY UNIQUE
        CHECK (reference_number BETWEEN 1 AND 2147483647),
    student_record_id uuid NOT NULL REFERENCES ple_data.student_record(student_record_id),
    assignment_id uuid NOT NULL REFERENCES ple_data.assignment(assignment_id),
    attempt_number integer NOT NULL CHECK (attempt_number > 0),
    started_at timestamptz NOT NULL,
    completed_at timestamptz,
    completion_score numeric CHECK (completion_score >= 0 AND completion_score <= 1),
    assignment_title text NOT NULL CHECK (assignment_title ~ '[^[:space:]]'),
    assignment_instructions text NOT NULL CHECK (assignment_instructions !~ E'\\x00'),
    available_at timestamptz,
    due_at timestamptz,
    closes_at timestamptz,
    assignment_attempt_time_limit_seconds integer,
    attempt_limit integer,
    late_work_rule text NOT NULL CHECK (late_work_rule IN ('accept', 'mark_late', 'reject')),
    assignment_completion_rule text NOT NULL CHECK (assignment_completion_rule IN ('answer_all', 'all_correct', 'score_at_least')),
    assignment_completion_score_threshold numeric,
    assignment_attempt_grade_rule text NOT NULL CHECK (assignment_attempt_grade_rule IN ('first', 'latest', 'highest', 'instructor_selected')),
    assignment_attempt_continuation_rule text NOT NULL CHECK (assignment_attempt_continuation_rule IN ('unlimited', 'capped', 'closed')),
    max_additional_assignment_attempts integer,
    question_pool_reuse_rule text NOT NULL CHECK (question_pool_reuse_rule IN ('reuse_selection', 'select_again')),
    question_variation_rule text NOT NULL CHECK (question_variation_rule IN ('reuse_variation', 'new_variation')),
    assignment_attempt_resume_rule text NOT NULL CHECK (assignment_attempt_resume_rule IN ('resumable', 'single_session')),
    assignment_question_display_rule text NOT NULL CHECK (assignment_question_display_rule IN ('all_questions', 'one_question_at_a_time')),
    assignment_navigation_rule text NOT NULL CHECK (assignment_navigation_rule IN ('free_navigation', 'forward_only')),
    assignment_question_order_rule text NOT NULL CHECK (assignment_question_order_rule IN ('authored_order', 'shuffled')),
    feedback_score text NOT NULL CHECK (feedback_score IN ('during_attempt', 'after_submit', 'after_due', 'after_close', 'never')),
    feedback_per_item_correctness text NOT NULL CHECK (feedback_per_item_correctness IN ('during_attempt', 'after_submit', 'after_due', 'after_close', 'never')),
    feedback_submitted_response text NOT NULL CHECK (feedback_submitted_response IN ('during_attempt', 'after_submit', 'after_due', 'after_close', 'never')),
    feedback_question_feedback text NOT NULL CHECK (feedback_question_feedback IN ('during_attempt', 'after_submit', 'after_due', 'after_close', 'never')),
    feedback_question_answer text NOT NULL CHECK (feedback_question_answer IN ('during_attempt', 'after_submit', 'after_due', 'after_close', 'never')),
    feedback_question_answer_explanation text NOT NULL CHECK (feedback_question_answer_explanation IN ('during_attempt', 'after_submit', 'after_due', 'after_close', 'never')),
    feedback_class_statistics text NOT NULL CHECK (feedback_class_statistics IN ('during_attempt', 'after_submit', 'after_due', 'after_close', 'never')),
    schedule_accommodation_id uuid,
    schedule_accommodation_edit_number bigint CHECK (schedule_accommodation_edit_number > 0),
    time_limit_accommodation_id uuid,
    time_limit_accommodation_edit_number bigint CHECK (time_limit_accommodation_edit_number > 0),
    attempt_limit_accommodation_id uuid,
    attempt_limit_accommodation_edit_number bigint CHECK (attempt_limit_accommodation_edit_number > 0),
    UNIQUE (student_record_id, assignment_id, attempt_number),
    CHECK (completed_at IS NULL OR completed_at >= started_at),
    CHECK ((completed_at IS NULL) = (completion_score IS NULL)),
    CHECK ((available_at IS NULL OR due_at IS NULL OR available_at <= due_at)
       AND (due_at IS NULL OR closes_at IS NULL OR due_at <= closes_at)),
    CHECK (assignment_attempt_time_limit_seconds IS NULL OR assignment_attempt_time_limit_seconds > 0),
    CHECK (attempt_limit IS NULL OR attempt_limit > 0),
    CHECK ((assignment_completion_rule = 'score_at_least'
        AND assignment_completion_score_threshold > 0 AND assignment_completion_score_threshold <= 1)
       OR (assignment_completion_rule <> 'score_at_least' AND assignment_completion_score_threshold IS NULL)),
    CHECK ((assignment_attempt_continuation_rule = 'capped' AND max_additional_assignment_attempts >= 0)
       OR (assignment_attempt_continuation_rule <> 'capped' AND max_additional_assignment_attempts IS NULL)),
    FOREIGN KEY (schedule_accommodation_id, student_record_id, assignment_id)
        REFERENCES ple_private.student_assignment_accommodation(accommodation_id, student_record_id, assignment_id),
    FOREIGN KEY (time_limit_accommodation_id, student_record_id, assignment_id)
        REFERENCES ple_private.student_assignment_accommodation(accommodation_id, student_record_id, assignment_id),
    FOREIGN KEY (attempt_limit_accommodation_id, student_record_id, assignment_id)
        REFERENCES ple_private.student_assignment_accommodation(accommodation_id, student_record_id, assignment_id),
    CHECK ((schedule_accommodation_id IS NULL) = (schedule_accommodation_edit_number IS NULL)),
    CHECK ((time_limit_accommodation_id IS NULL) = (time_limit_accommodation_edit_number IS NULL)),
    CHECK ((attempt_limit_accommodation_id IS NULL) = (attempt_limit_accommodation_edit_number IS NULL))
);

CREATE TABLE ple_private.question_pool_selection (
    question_pool_selection_id uuid PRIMARY KEY,
    assignment_attempt_id uuid NOT NULL REFERENCES ple_private.assignment_attempt(assignment_attempt_id) ON DELETE CASCADE,
    -- This is the stable authored identity copied at issue time, not a foreign
    -- key to the mutable current Assignment Entry.  Released Assignment saves
    -- may replace current entries while this Student Work remains interpretable.
    assignment_entry_id uuid NOT NULL,
    created_at timestamptz NOT NULL,
    selected_question_count integer NOT NULL CHECK (selected_question_count > 0),
    reused_from_question_pool_selection_id uuid REFERENCES ple_private.question_pool_selection(question_pool_selection_id),
    UNIQUE (question_pool_selection_id, assignment_attempt_id, assignment_entry_id),
    UNIQUE (assignment_attempt_id, assignment_entry_id)
);

CREATE TABLE ple_private.question_pool_selected_item (
    question_pool_selection_id uuid NOT NULL REFERENCES ple_private.question_pool_selection(question_pool_selection_id) ON DELETE CASCADE,
    -- The historical source identity is evidence; Question Pool Items are
    -- current mutable configuration and deliberately are not its parent.
    question_pool_item_id uuid NOT NULL,
    selection_position integer NOT NULL CHECK (selection_position >= 0),
    question_id text NOT NULL,
    revision_number integer NOT NULL,
    PRIMARY KEY (question_pool_selection_id, selection_position),
    UNIQUE (question_pool_selection_id, question_pool_item_id),
    UNIQUE (question_pool_selection_id, question_pool_item_id, question_id, revision_number),
    FOREIGN KEY (question_id, revision_number) REFERENCES ple_data.question_revision(question_id, revision_number)
);

CREATE TABLE ple_private.issued_question (
    issued_question_id uuid PRIMARY KEY,
    assignment_attempt_id uuid NOT NULL REFERENCES ple_private.assignment_attempt(assignment_attempt_id) ON DELETE CASCADE,
    assignment_entry_id uuid NOT NULL,
    assignment_content_entry_index integer NOT NULL CHECK (assignment_content_entry_index >= 0),
    issued_position integer NOT NULL CHECK (issued_position >= 0),
    question_id text NOT NULL,
    revision_number integer NOT NULL,
    question_seed numeric(20, 0) NOT NULL CHECK (question_seed >= 0 AND question_seed <= 18446744073709551615),
    point_value numeric NOT NULL CHECK (point_value >= 0),
    scoring_rule text NOT NULL CHECK (scoring_rule IN ('normal', 'full_credit', 'extra_credit', 'excluded')),
    question_statistics_eligibility boolean NOT NULL,
    question_attempt_limit integer,
    question_attempt_time_limit_seconds integer,
    question_attempt_grace_seconds integer,
    question_pool_selection_id uuid,
    question_pool_item_id uuid,
    UNIQUE (assignment_attempt_id, issued_position),
    FOREIGN KEY (question_id, revision_number) REFERENCES ple_data.question_revision(question_id, revision_number),
    FOREIGN KEY (question_pool_selection_id, assignment_attempt_id, assignment_entry_id)
        REFERENCES ple_private.question_pool_selection(question_pool_selection_id, assignment_attempt_id, assignment_entry_id),
    FOREIGN KEY (question_pool_selection_id, question_pool_item_id, question_id, revision_number)
        REFERENCES ple_private.question_pool_selected_item(question_pool_selection_id, question_pool_item_id, question_id, revision_number),
    CHECK ((question_pool_selection_id IS NULL) = (question_pool_item_id IS NULL)),
    CHECK (question_attempt_limit IS NULL OR question_attempt_limit > 0),
    CHECK ((question_attempt_time_limit_seconds IS NULL AND question_attempt_grace_seconds IS NULL)
        OR (question_attempt_time_limit_seconds > 0 AND question_attempt_grace_seconds >= 0))
);

CREATE FUNCTION ple_private.assert_student_assignment_accommodation_scope()
RETURNS trigger LANGUAGE plpgsql SET search_path = pg_catalog, ple_data, ple_private AS $$
BEGIN
    IF NOT ple_data.student_assignment_has_course_scope(NEW.student_record_id, NEW.assignment_id) THEN
        RAISE EXCEPTION USING ERRCODE = '23514', MESSAGE = 'Accommodation requires a Student and Assignment in one Course';
    END IF;
    RETURN NEW;
END $$;

CREATE FUNCTION ple_private.enforce_student_assignment_accommodation_edit()
RETURNS trigger LANGUAGE plpgsql SET search_path = pg_catalog, ple_private AS $$
BEGIN
    IF NEW.accommodation_id IS DISTINCT FROM OLD.accommodation_id
       OR NEW.student_record_id IS DISTINCT FROM OLD.student_record_id
       OR NEW.assignment_id IS DISTINCT FROM OLD.assignment_id
       OR NEW.created_at IS DISTINCT FROM OLD.created_at
       OR NEW.accommodation_edit_number <> OLD.accommodation_edit_number + 1 THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Accommodation identity is immutable and changes advance one Edit Number';
    END IF;
    RETURN NEW;
END $$;

-- The Student Work table owner needs this one relational fact while inserting
-- an Attempt.  The data owner has this narrow RLS read only to evaluate the
-- invariant.  Neither private nor application roles receive a general Student
-- record read grant.
RESET ROLE;
SET LOCAL ROLE ple_data_owner;
CREATE POLICY student_record_data_owner_assignment_scope_check ON ple_data.student_record
    FOR SELECT TO ple_data_owner USING (true);
CREATE FUNCTION ple_data.student_assignment_has_course_scope(
    p_student_record_id uuid, p_assignment_id uuid
) RETURNS boolean
LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_data AS $$
    SELECT EXISTS (
        SELECT 1
          FROM ple_data.student_record AS student
          JOIN ple_data.assignment AS assignment ON assignment.course_id = student.course_id
         WHERE student.student_record_id = $1 AND assignment.assignment_id = $2
    )
$$;
REVOKE ALL ON FUNCTION ple_data.student_assignment_has_course_scope(uuid, uuid) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_data.student_assignment_has_course_scope(uuid, uuid) TO ple_private_owner;

RESET ROLE;
SET LOCAL ROLE ple_private_owner;
CREATE FUNCTION ple_private.assert_assignment_attempt_scope()
RETURNS trigger LANGUAGE plpgsql SET search_path = pg_catalog, ple_data, ple_private AS $$
BEGIN
    IF NOT ple_data.student_assignment_has_course_scope(NEW.student_record_id, NEW.assignment_id) THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Assignment Attempt requires a Student and Assignment in one Course';
    END IF;
    RETURN NEW;
END $$;

CREATE FUNCTION ple_private.reject_student_work_delete()
RETURNS trigger LANGUAGE plpgsql SET search_path = pg_catalog, ple_private AS $$
BEGIN
    -- The executor deletes only Assignment Attempt roots.  PostgreSQL invokes
    -- descendants through foreign-key cascade triggers at a nested depth; no
    -- application role has DELETE on these tables or an application-deleteable
    -- parent in this ownership graph.
    IF current_user <> 'ple_unrelease_executor' AND pg_catalog.pg_trigger_depth() <= 1 THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Student Work deletion requires the Unrelease executor';
    END IF;
    RETURN OLD;
END $$;

CREATE FUNCTION ple_private.reject_assignment_attempt_rewrite()
RETURNS trigger LANGUAGE plpgsql SET search_path = pg_catalog, ple_private AS $$
BEGIN
    IF NEW.assignment_attempt_id IS DISTINCT FROM OLD.assignment_attempt_id
       OR NEW.reference_number IS DISTINCT FROM OLD.reference_number
       OR NEW.student_record_id IS DISTINCT FROM OLD.student_record_id
       OR NEW.assignment_id IS DISTINCT FROM OLD.assignment_id
       OR NEW.attempt_number IS DISTINCT FROM OLD.attempt_number
       OR NEW.started_at IS DISTINCT FROM OLD.started_at
       OR ROW(NEW.assignment_title, NEW.assignment_instructions, NEW.available_at, NEW.due_at, NEW.closes_at,
              NEW.assignment_attempt_time_limit_seconds, NEW.attempt_limit, NEW.late_work_rule,
              NEW.assignment_completion_rule, NEW.assignment_completion_score_threshold,
              NEW.assignment_attempt_grade_rule, NEW.assignment_attempt_continuation_rule,
              NEW.max_additional_assignment_attempts, NEW.question_pool_reuse_rule, NEW.question_variation_rule,
              NEW.assignment_attempt_resume_rule, NEW.assignment_question_display_rule,
              NEW.assignment_navigation_rule, NEW.assignment_question_order_rule, NEW.feedback_score,
              NEW.feedback_per_item_correctness, NEW.feedback_submitted_response, NEW.feedback_question_feedback,
              NEW.feedback_question_answer, NEW.feedback_question_answer_explanation, NEW.feedback_class_statistics,
              NEW.schedule_accommodation_id, NEW.schedule_accommodation_edit_number,
              NEW.time_limit_accommodation_id, NEW.time_limit_accommodation_edit_number,
              NEW.attempt_limit_accommodation_id, NEW.attempt_limit_accommodation_edit_number)
           IS DISTINCT FROM ROW(OLD.assignment_title, OLD.assignment_instructions, OLD.available_at, OLD.due_at, OLD.closes_at,
              OLD.assignment_attempt_time_limit_seconds, OLD.attempt_limit, OLD.late_work_rule,
              OLD.assignment_completion_rule, OLD.assignment_completion_score_threshold,
              OLD.assignment_attempt_grade_rule, OLD.assignment_attempt_continuation_rule,
              OLD.max_additional_assignment_attempts, OLD.question_pool_reuse_rule, OLD.question_variation_rule,
              OLD.assignment_attempt_resume_rule, OLD.assignment_question_display_rule,
              OLD.assignment_navigation_rule, OLD.assignment_question_order_rule, OLD.feedback_score,
              OLD.feedback_per_item_correctness, OLD.feedback_submitted_response, OLD.feedback_question_feedback,
              OLD.feedback_question_answer, OLD.feedback_question_answer_explanation, OLD.feedback_class_statistics,
              OLD.schedule_accommodation_id, OLD.schedule_accommodation_edit_number,
              OLD.time_limit_accommodation_id, OLD.time_limit_accommodation_edit_number,
              OLD.attempt_limit_accommodation_id, OLD.attempt_limit_accommodation_edit_number)
       OR OLD.completed_at IS NOT NULL THEN
        RAISE EXCEPTION USING ERRCODE = '55000', MESSAGE = 'Assignment Attempt evidence is immutable after creation';
    END IF;
    RETURN NEW;
END $$;

CREATE FUNCTION ple_private.reject_immutable_student_work_change()
RETURNS trigger LANGUAGE plpgsql SET search_path = pg_catalog, ple_private AS $$
BEGIN RAISE EXCEPTION USING ERRCODE = '55000', MESSAGE = 'Student Work evidence is immutable'; END $$;

CREATE FUNCTION ple_private.validate_question_pool_selection_reuse()
RETURNS trigger LANGUAGE plpgsql SET search_path = pg_catalog, ple_private AS $$
BEGIN
    IF NEW.reused_from_question_pool_selection_id IS NOT NULL AND NOT EXISTS (
        SELECT 1 FROM ple_private.question_pool_selection AS earlier
        JOIN ple_private.assignment_attempt AS earlier_attempt ON earlier_attempt.assignment_attempt_id = earlier.assignment_attempt_id
        JOIN ple_private.assignment_attempt AS current_attempt ON current_attempt.assignment_attempt_id = NEW.assignment_attempt_id
        WHERE earlier.question_pool_selection_id = NEW.reused_from_question_pool_selection_id
          AND earlier.assignment_entry_id = NEW.assignment_entry_id
          AND earlier_attempt.student_record_id = current_attempt.student_record_id
          AND earlier_attempt.assignment_id = current_attempt.assignment_id
          AND earlier_attempt.attempt_number < current_attempt.attempt_number
    ) THEN RAISE EXCEPTION USING ERRCODE = '23514', MESSAGE = 'Reused Question Pool Selection requires earlier Student Work for the same Assignment Entry'; END IF;
    RETURN NEW;
END $$;

CREATE FUNCTION ple_private.validate_question_pool_selected_item_count()
RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_private AS $$
DECLARE selection_id uuid := CASE WHEN TG_OP = 'DELETE' THEN OLD.question_pool_selection_id ELSE NEW.question_pool_selection_id END;
BEGIN
    IF EXISTS (SELECT 1 FROM ple_private.question_pool_selection AS selection
        WHERE selection.question_pool_selection_id = selection_id
          AND selection.selected_question_count <> (SELECT count(*) FROM ple_private.question_pool_selected_item WHERE question_pool_selection_id = selection_id)) THEN
        RAISE EXCEPTION USING ERRCODE = '23514', MESSAGE = 'Question Pool Selection requires its exact selected Item count';
    END IF;
    RETURN NULL;
END $$;

CREATE FUNCTION ple_private.validate_question_pool_selection_issues()
RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_private AS $$
DECLARE selection_id uuid := CASE WHEN TG_OP = 'DELETE' THEN OLD.question_pool_selection_id ELSE NEW.question_pool_selection_id END;
BEGIN
    IF EXISTS (
        (SELECT item.question_pool_item_id, item.question_id, item.revision_number
           FROM ple_private.question_pool_selected_item AS item
          WHERE item.question_pool_selection_id = selection_id
         EXCEPT
         SELECT issued.question_pool_item_id, issued.question_id, issued.revision_number
           FROM ple_private.issued_question AS issued
          WHERE issued.question_pool_selection_id = selection_id)
        UNION ALL
        (SELECT issued.question_pool_item_id, issued.question_id, issued.revision_number
           FROM ple_private.issued_question AS issued
          WHERE issued.question_pool_selection_id = selection_id
         EXCEPT
         SELECT item.question_pool_item_id, item.question_id, item.revision_number
           FROM ple_private.question_pool_selected_item AS item
          WHERE item.question_pool_selection_id = selection_id)
    ) THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Question Pool Selection and Issued Questions require one exact shared item set';
    END IF;
    RETURN NULL;
END $$;

CREATE TRIGGER student_assignment_accommodation_has_course_scope BEFORE INSERT OR UPDATE
ON ple_private.student_assignment_accommodation FOR EACH ROW EXECUTE FUNCTION ple_private.assert_student_assignment_accommodation_scope();
CREATE TRIGGER assignment_attempt_has_course_scope BEFORE INSERT ON ple_private.assignment_attempt
FOR EACH ROW EXECUTE FUNCTION ple_private.assert_assignment_attempt_scope();
CREATE TRIGGER assignment_attempt_retains_evidence BEFORE UPDATE ON ple_private.assignment_attempt
FOR EACH ROW EXECUTE FUNCTION ple_private.reject_assignment_attempt_rewrite();
CREATE TRIGGER assignment_attempt_delete_is_guarded BEFORE DELETE ON ple_private.assignment_attempt
FOR EACH ROW EXECUTE FUNCTION ple_private.reject_student_work_delete();
CREATE TRIGGER student_assignment_accommodation_edit_is_guarded BEFORE UPDATE
ON ple_private.student_assignment_accommodation
FOR EACH ROW EXECUTE FUNCTION ple_private.enforce_student_assignment_accommodation_edit();
CREATE TRIGGER question_pool_selection_is_immutable BEFORE UPDATE ON ple_private.question_pool_selection
FOR EACH ROW EXECUTE FUNCTION ple_private.reject_immutable_student_work_change();
CREATE TRIGGER question_pool_selection_delete_is_guarded BEFORE DELETE ON ple_private.question_pool_selection
FOR EACH ROW EXECUTE FUNCTION ple_private.reject_student_work_delete();
CREATE TRIGGER question_pool_selected_item_is_immutable BEFORE UPDATE ON ple_private.question_pool_selected_item
FOR EACH ROW EXECUTE FUNCTION ple_private.reject_immutable_student_work_change();
CREATE TRIGGER question_pool_selected_item_delete_is_guarded BEFORE DELETE ON ple_private.question_pool_selected_item
FOR EACH ROW EXECUTE FUNCTION ple_private.reject_student_work_delete();
CREATE TRIGGER issued_question_is_immutable BEFORE UPDATE ON ple_private.issued_question
FOR EACH ROW EXECUTE FUNCTION ple_private.reject_immutable_student_work_change();
CREATE TRIGGER issued_question_delete_is_guarded BEFORE DELETE ON ple_private.issued_question
FOR EACH ROW EXECUTE FUNCTION ple_private.reject_student_work_delete();
CREATE TRIGGER question_pool_selection_reuse_is_valid BEFORE INSERT ON ple_private.question_pool_selection
FOR EACH ROW EXECUTE FUNCTION ple_private.validate_question_pool_selection_reuse();
CREATE CONSTRAINT TRIGGER question_pool_selection_has_exact_item_count AFTER INSERT ON ple_private.question_pool_selection
DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION ple_private.validate_question_pool_selected_item_count();
CREATE CONSTRAINT TRIGGER question_pool_selected_item_count_is_exact AFTER INSERT OR UPDATE OR DELETE ON ple_private.question_pool_selected_item
DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION ple_private.validate_question_pool_selected_item_count();
CREATE CONSTRAINT TRIGGER question_pool_selection_issues_are_exact AFTER INSERT OR UPDATE OR DELETE ON ple_private.question_pool_selected_item
DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION ple_private.validate_question_pool_selection_issues();
CREATE CONSTRAINT TRIGGER issued_question_pool_source_is_exact AFTER INSERT OR UPDATE OR DELETE ON ple_private.issued_question
DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION ple_private.validate_question_pool_selection_issues();

CREATE INDEX assignment_attempt_student_assignment_lookup_idx
    ON ple_private.assignment_attempt(student_record_id, assignment_id, attempt_number DESC);
CREATE INDEX issued_question_attempt_position_idx ON ple_private.issued_question(assignment_attempt_id, issued_position);

ALTER TABLE ple_private.student_assignment_accommodation ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.student_assignment_accommodation FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_private.assignment_attempt ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.assignment_attempt FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_private.question_pool_selection ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.question_pool_selection FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_private.question_pool_selected_item ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.question_pool_selected_item FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_private.issued_question ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.issued_question FORCE ROW LEVEL SECURITY;
REVOKE ALL ON TABLE ple_private.student_assignment_accommodation, ple_private.assignment_attempt,
    ple_private.question_pool_selection, ple_private.question_pool_selected_item, ple_private.issued_question FROM PUBLIC;
CREATE POLICY student_assignment_accommodation_private_owner_access ON ple_private.student_assignment_accommodation FOR ALL TO ple_private_owner USING (true) WITH CHECK (true);
CREATE POLICY assignment_attempt_private_owner_access ON ple_private.assignment_attempt FOR ALL TO ple_private_owner USING (true) WITH CHECK (true);
CREATE POLICY question_pool_selection_private_owner_access ON ple_private.question_pool_selection FOR ALL TO ple_private_owner USING (true) WITH CHECK (true);
CREATE POLICY question_pool_selected_item_private_owner_access ON ple_private.question_pool_selected_item FOR ALL TO ple_private_owner USING (true) WITH CHECK (true);
CREATE POLICY issued_question_private_owner_access ON ple_private.issued_question FOR ALL TO ple_private_owner USING (true) WITH CHECK (true);
REVOKE ALL ON FUNCTION ple_private.assert_student_assignment_accommodation_scope(), ple_private.assert_assignment_attempt_scope(),
    ple_private.enforce_student_assignment_accommodation_edit(), ple_private.reject_student_work_delete(), ple_private.reject_assignment_attempt_rewrite(),
    ple_private.reject_immutable_student_work_change(), ple_private.validate_question_pool_selection_reuse(),
    ple_private.validate_question_pool_selected_item_count(),
    ple_private.validate_question_pool_selection_issues() FROM PUBLIC;

COMMENT ON TABLE ple_private.assignment_attempt IS 'Immutable effective Assignment evidence for one Student Work occurrence; only completion may be recorded once.';
COMMENT ON TABLE ple_private.issued_question IS 'Exact Assignment Entry identity, Question Revision, seed, per-question policy, scoring, statistics, and pool-selection evidence for one issued position.';

RESET ROLE;
