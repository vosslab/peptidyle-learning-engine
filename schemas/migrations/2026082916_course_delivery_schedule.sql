-- SD1 CourseInstance delivery, Course Schedule Revision, and Assignment Revision roots.

SET LOCAL ROLE ple_data_owner;
-- A Course Term changes only by creating the next immutable Course Schedule
-- Revision. Every Assignment Revision then retains the exact term revision
-- that resolved its delivery instants.
CREATE TABLE ple_data.course_schedule_revision (
    course_schedule_revision_id uuid PRIMARY KEY,
    course_id uuid NOT NULL REFERENCES ple_data.course_instance (course_id),
    revision_number bigint NOT NULL CHECK (revision_number > 0),
    term_starts_on date NOT NULL,
    term_ends_on date NOT NULL,
    course_time_zone text NOT NULL CHECK (char_length(btrim(course_time_zone)) BETWEEN 1 AND 100),
    created_at timestamp with time zone NOT NULL,
    UNIQUE (course_id, revision_number),
    UNIQUE (course_id, course_schedule_revision_id),
    CONSTRAINT course_schedule_revision_term_is_ordered
        CHECK (term_starts_on <= term_ends_on)
);
CREATE TABLE ple_data.assignment (
    assignment_id uuid PRIMARY KEY,
    course_id uuid NOT NULL REFERENCES ple_data.course_instance (course_id),
    source_blueprint_revision_id uuid NOT NULL REFERENCES ple_data.blueprint_course_revision (blueprint_revision_id),
    created_at timestamp with time zone NOT NULL,
    assignment_status text NOT NULL CHECK (assignment_status IN ('unreleased', 'released', 'closed', 'archived')),
    released_assignment_revision_id uuid,
    CONSTRAINT assignment_course_is_unique UNIQUE (course_id, assignment_id),
    CONSTRAINT assignment_id_course_is_unique UNIQUE (assignment_id, course_id)
);
-- One Assignment owns one replaceable current definition.  Its edit number is
-- the only ordinary-save precondition; it does not create teaching history.
CREATE TABLE ple_data.assignment_working_copy (
    assignment_id uuid PRIMARY KEY REFERENCES ple_data.assignment (assignment_id),
    course_id uuid NOT NULL,
    edit_number bigint NOT NULL CHECK (edit_number > 0),
    authored_definition jsonb NOT NULL CHECK (jsonb_typeof(authored_definition) = 'object'),
    assignment_title text NOT NULL CONSTRAINT assignment_working_copy_title_is_valid
        CHECK (assignment_title ~ '[^[:space:]]' AND char_length(assignment_title) <= 200),
    assignment_instructions text NOT NULL CHECK (char_length(assignment_instructions) <= 50000),
    available_at timestamp with time zone,
    due_at timestamp with time zone,
    closes_at timestamp with time zone,
    assignment_attempt_time_limit_seconds integer CHECK (assignment_attempt_time_limit_seconds IS NULL OR assignment_attempt_time_limit_seconds > 0),
    attempt_limit integer CHECK (attempt_limit IS NULL OR attempt_limit > 0),
    late_work_rule text NOT NULL CHECK (late_work_rule IN ('accept', 'mark_late', 'reject')),
    assignment_deadline_rule text NOT NULL CHECK (assignment_deadline_rule = 'auto_submit'),
    assignment_completion_rule text NOT NULL CHECK (assignment_completion_rule IN ('answer_all', 'all_correct', 'score_at_least')),
    assignment_completion_score_threshold numeric CHECK (
        (assignment_completion_rule = 'score_at_least'
            AND assignment_completion_score_threshold > 0
            AND assignment_completion_score_threshold <= 1)
        OR (assignment_completion_rule <> 'score_at_least'
            AND assignment_completion_score_threshold IS NULL)
    ),
    assignment_attempt_grade_rule text NOT NULL CHECK (assignment_attempt_grade_rule IN ('first', 'latest', 'highest', 'instructor_selected')),
    assignment_attempt_continuation_rule text NOT NULL CHECK (assignment_attempt_continuation_rule IN ('unlimited', 'capped', 'closed')),
    max_additional_assignment_attempts integer CHECK (
        (assignment_attempt_continuation_rule = 'capped'
            AND max_additional_assignment_attempts >= 0)
        OR (assignment_attempt_continuation_rule <> 'capped'
            AND max_additional_assignment_attempts IS NULL)
    ),
    question_variation_rule text NOT NULL CHECK (question_variation_rule IN ('reuse_questions_with_new_seeds', 'selected_question_variants', 'redraw_question_pools')),
    assignment_attempt_resume_rule text NOT NULL CHECK (assignment_attempt_resume_rule IN ('resumable', 'single_session')),
    assignment_question_display_rule text NOT NULL CHECK (assignment_question_display_rule IN ('all_questions', 'one_question_at_a_time')),
    assignment_navigation_rule text NOT NULL CHECK (assignment_navigation_rule IN ('free_navigation', 'forward_only')),
    assignment_question_order_rule text NOT NULL CHECK (assignment_question_order_rule IN ('authored_order', 'shuffled')),
    created_at timestamp with time zone NOT NULL,
    updated_at timestamp with time zone NOT NULL,
    CONSTRAINT assignment_working_copy_course_matches_assignment FOREIGN KEY (assignment_id, course_id)
        REFERENCES ple_data.assignment (assignment_id, course_id),
    CONSTRAINT assignment_working_copy_schedule_is_ordered CHECK (
        (available_at IS NULL OR due_at IS NULL OR available_at <= due_at)
        AND (due_at IS NULL OR closes_at IS NULL OR due_at <= closes_at)
    ),
    CONSTRAINT assignment_working_copy_update_is_ordered CHECK (updated_at >= created_at)
);
-- ASVS 8.2.2: an Assignment Release creates one immutable teaching definition
-- that Student work can pin exactly.
CREATE TABLE ple_data.assignment_revision (
    assignment_revision_id uuid PRIMARY KEY,
    assignment_id uuid NOT NULL REFERENCES ple_data.assignment (assignment_id),
    course_id uuid NOT NULL,
    course_schedule_revision_id uuid NOT NULL,
    revision_number bigint NOT NULL CHECK (revision_number > 0),
    authored_definition jsonb NOT NULL CHECK (jsonb_typeof(authored_definition) = 'object'),
    assignment_title text NOT NULL CONSTRAINT assignment_revision_title_is_valid
        CHECK (assignment_title ~ '[^[:space:]]' AND char_length(assignment_title) <= 200),
    assignment_instructions text NOT NULL CHECK (char_length(assignment_instructions) <= 50000),
    available_at timestamp with time zone,
    due_at timestamp with time zone,
    closes_at timestamp with time zone,
    assignment_attempt_time_limit_seconds integer CHECK (assignment_attempt_time_limit_seconds IS NULL OR assignment_attempt_time_limit_seconds > 0),
    attempt_limit integer CHECK (attempt_limit IS NULL OR attempt_limit > 0),
    late_work_rule text NOT NULL CHECK (late_work_rule IN ('accept', 'mark_late', 'reject')),
    assignment_deadline_rule text NOT NULL CHECK (assignment_deadline_rule = 'auto_submit'),
    assignment_completion_rule text NOT NULL CHECK (assignment_completion_rule IN ('answer_all', 'all_correct', 'score_at_least')),
    assignment_completion_score_threshold numeric CHECK (
        (assignment_completion_rule = 'score_at_least'
            AND assignment_completion_score_threshold > 0
            AND assignment_completion_score_threshold <= 1)
        OR (assignment_completion_rule <> 'score_at_least'
            AND assignment_completion_score_threshold IS NULL)
    ),
    assignment_attempt_grade_rule text NOT NULL CHECK (assignment_attempt_grade_rule IN ('first', 'latest', 'highest', 'instructor_selected')),
    assignment_attempt_continuation_rule text NOT NULL CHECK (assignment_attempt_continuation_rule IN ('unlimited', 'capped', 'closed')),
    max_additional_assignment_attempts integer CHECK (
        (assignment_attempt_continuation_rule = 'capped'
            AND max_additional_assignment_attempts >= 0)
        OR (assignment_attempt_continuation_rule <> 'capped'
            AND max_additional_assignment_attempts IS NULL)
    ),
    question_variation_rule text NOT NULL CHECK (question_variation_rule IN ('reuse_questions_with_new_seeds', 'selected_question_variants', 'redraw_question_pools')),
    assignment_attempt_resume_rule text NOT NULL CHECK (assignment_attempt_resume_rule IN ('resumable', 'single_session')),
    assignment_question_display_rule text NOT NULL CHECK (assignment_question_display_rule IN ('all_questions', 'one_question_at_a_time')),
    assignment_navigation_rule text NOT NULL CHECK (assignment_navigation_rule IN ('free_navigation', 'forward_only')),
    assignment_question_order_rule text NOT NULL CHECK (assignment_question_order_rule IN ('authored_order', 'shuffled')),
    created_at timestamp with time zone NOT NULL,
    UNIQUE (assignment_id, revision_number),
    UNIQUE (assignment_id, assignment_revision_id),
    CONSTRAINT assignment_revision_course_matches_assignment FOREIGN KEY (assignment_id, course_id)
        REFERENCES ple_data.assignment (assignment_id, course_id),
    CONSTRAINT assignment_revision_schedule_matches_course FOREIGN KEY (
        course_id, course_schedule_revision_id
    ) REFERENCES ple_data.course_schedule_revision (course_id, course_schedule_revision_id),
    CONSTRAINT assignment_revision_schedule_is_ordered CHECK (
        (available_at IS NULL OR due_at IS NULL OR available_at <= due_at)
        AND (due_at IS NULL OR closes_at IS NULL OR due_at <= closes_at)
    )
);
ALTER TABLE ple_data.assignment
    ADD CONSTRAINT assignment_released_revision_matches_assignment
    FOREIGN KEY (assignment_id, released_assignment_revision_id)
    REFERENCES ple_data.assignment_revision (assignment_id, assignment_revision_id);
CREATE FUNCTION ple_data.reject_course_schedule_revision_change()
RETURNS trigger LANGUAGE plpgsql SET search_path = pg_catalog, ple_data AS $$
BEGIN
    RAISE EXCEPTION USING ERRCODE = '23514', MESSAGE = 'Course Schedule Revision is immutable';
END
$$;
CREATE FUNCTION ple_data.reject_assignment_revision_change()
RETURNS trigger LANGUAGE plpgsql SET search_path = pg_catalog, ple_data AS $$
BEGIN
    RAISE EXCEPTION USING ERRCODE = '23514', MESSAGE = 'Assignment Revision is immutable';
END
$$;
CREATE FUNCTION ple_data.enforce_assignment_working_copy_replacement()
RETURNS trigger LANGUAGE plpgsql SET search_path = pg_catalog, ple_data AS $$
BEGIN
    IF NEW.assignment_id <> OLD.assignment_id
       OR NEW.course_id <> OLD.course_id
       OR NEW.created_at <> OLD.created_at
       OR NEW.edit_number <> OLD.edit_number + 1
       OR NEW.updated_at < OLD.updated_at THEN
        RAISE EXCEPTION USING
            ERRCODE = '23514',
            MESSAGE = 'Assignment Working Copy replacement must advance exactly one Assignment Edit Number';
    END IF;
    RETURN NEW;
END
$$;
CREATE FUNCTION ple_data.require_released_assignment_revision()
RETURNS trigger
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = pg_catalog, ple_data AS $$
BEGIN
    IF NOT EXISTS (
        SELECT 1
        FROM ple_data.assignment
        WHERE assignment_id = NEW.assignment_id
          AND assignment_status = 'released'
          AND released_assignment_revision_id = NEW.assignment_revision_id
    ) THEN
        RAISE EXCEPTION USING
            ERRCODE = '23514',
            MESSAGE = 'Assignment Attempt requires the exact Released Assignment Revision';
    END IF;
    RETURN NEW;
END
$$;
CREATE TRIGGER course_schedule_revision_is_immutable
BEFORE UPDATE OR DELETE ON ple_data.course_schedule_revision
FOR EACH ROW EXECUTE FUNCTION ple_data.reject_course_schedule_revision_change();
CREATE TRIGGER assignment_revision_is_immutable
BEFORE UPDATE OR DELETE ON ple_data.assignment_revision
FOR EACH ROW EXECUTE FUNCTION ple_data.reject_assignment_revision_change();
CREATE TRIGGER assignment_working_copy_replacement_is_exact
BEFORE UPDATE ON ple_data.assignment_working_copy
FOR EACH ROW EXECUTE FUNCTION ple_data.enforce_assignment_working_copy_replacement();
ALTER TABLE ple_data.assignment ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_data.assignment FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_data.assignment_working_copy ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_data.assignment_working_copy FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_data.course_schedule_revision ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_data.course_schedule_revision FORCE ROW LEVEL SECURITY;
REVOKE ALL PRIVILEGES ON TABLE ple_data.assignment FROM PUBLIC;
REVOKE ALL PRIVILEGES ON TABLE ple_data.course_schedule_revision, ple_data.assignment_working_copy, ple_data.assignment_revision FROM PUBLIC;
REVOKE ALL PRIVILEGES ON FUNCTION ple_data.reject_course_schedule_revision_change() FROM PUBLIC;
REVOKE ALL PRIVILEGES ON FUNCTION ple_data.reject_assignment_revision_change() FROM PUBLIC;
REVOKE ALL PRIVILEGES ON FUNCTION ple_data.enforce_assignment_working_copy_replacement() FROM PUBLIC;
REVOKE ALL PRIVILEGES ON FUNCTION ple_data.require_released_assignment_revision() FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_data.require_released_assignment_revision() TO ple_private_owner;
COMMENT ON TABLE ple_data.course_schedule_revision IS 'Immutable Course Term snapshot used to resolve Course Instance delivery schedules.';
COMMENT ON TABLE ple_data.assignment IS 'Stable Course Instance-owned Assignment identity, source, Assignment Status, and selected Released Assignment Revision.';
COMMENT ON TABLE ple_data.assignment_working_copy IS 'The one replaceable current Instructor-authored Assignment definition, guarded by Assignment Edit Number.';
COMMENT ON TABLE ple_data.assignment_revision IS 'Immutable Assignment definition and exact resolved delivery schedule for one Course Schedule Revision.';
RESET ROLE;
