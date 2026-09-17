-- Student Work roots.  An Assessment Attempt retains the effective facts that
-- later resume, submission, grading, history, disclosure, and statistics reads
-- need; current Assessment rows remain the authority only when an Assessment Attempt starts.

-- The private Student Work root owns FKs to these public Course/Assessment and
-- immutable Question Revision identities.  Current Assessment children are
-- intentionally evidence values below, not FK targets.
SET LOCAL ROLE ple_data_owner;
GRANT REFERENCES ON TABLE ple_data.student_record, ple_data.assessment,
    ple_data.question_revision, ple_data.question_pool_revision TO ple_private_owner;
SET LOCAL ROLE ple_private_owner;

CREATE TABLE ple_private.student_assessment_accommodation (
    accommodation_id uuid PRIMARY KEY,
    student_record_id uuid NOT NULL REFERENCES ple_data.student_record(student_record_id),
    assessment_id uuid NOT NULL REFERENCES ple_data.assessment(assessment_id),
    available_at timestamptz,
    due_at timestamptz,
    closes_at timestamptz,
    time_multiplier numeric,
    assessment_attempt_limit integer,
    created_at timestamptz NOT NULL,
    accommodation_edit_number bigint NOT NULL DEFAULT 1
        CHECK (accommodation_edit_number > 0),
    CHECK ((available_at IS NULL OR due_at IS NULL OR available_at <= due_at)
       AND (due_at IS NULL OR closes_at IS NULL OR due_at <= closes_at)),
    CHECK (time_multiplier IS NULL
       OR (time_multiplier >= 1 AND time_multiplier < 'Infinity'::numeric)),
    CHECK (assessment_attempt_limit IS NULL OR assessment_attempt_limit > 0),
    UNIQUE (accommodation_id, student_record_id, assessment_id),
    UNIQUE (student_record_id, assessment_id)
);

CREATE TABLE ple_private.assessment_attempt (
    assessment_attempt_id uuid PRIMARY KEY,
    reference_number bigint GENERATED ALWAYS AS IDENTITY UNIQUE
        CHECK (reference_number BETWEEN 1 AND 2147483647),
    student_record_id uuid NOT NULL REFERENCES ple_data.student_record(student_record_id),
    assessment_id uuid NOT NULL REFERENCES ple_data.assessment(assessment_id),
    assessment_attempt_number integer NOT NULL CHECK (assessment_attempt_number > 0),
    started_at timestamptz NOT NULL,
    expires_at timestamptz,
    assessment_title text NOT NULL CHECK (assessment_title ~ '[^[:space:]]'),
    assessment_instructions text NOT NULL CHECK (assessment_instructions !~ E'\\x00'),
    available_at timestamptz,
    due_at timestamptz,
    closes_at timestamptz,
    assessment_attempt_time_limit_seconds integer,
    assessment_attempt_limit integer,
    late_work_rule text NOT NULL CHECK (late_work_rule IN ('accept', 'mark_late', 'reject')),
    question_variation_rule text NOT NULL CHECK (question_variation_rule IN ('reuse_variation', 'new_variation')),
    assessment_question_order_rule text NOT NULL CHECK (assessment_question_order_rule IN ('authored_order', 'shuffled')),
    feedback_score text NOT NULL CHECK (feedback_score IN ('during_attempt', 'after_submit', 'after_due', 'after_close', 'never')),
    feedback_per_item_correctness text NOT NULL CHECK (feedback_per_item_correctness IN ('during_attempt', 'after_submit', 'after_due', 'after_close', 'never')),
    feedback_submitted_response text NOT NULL CHECK (feedback_submitted_response IN ('during_attempt', 'after_submit', 'after_due', 'after_close', 'never')),
    feedback_question_answer text NOT NULL CHECK (feedback_question_answer IN ('during_attempt', 'after_submit', 'after_due', 'after_close', 'never')),
    feedback_question_answer_explanation text NOT NULL CHECK (feedback_question_answer_explanation IN ('during_attempt', 'after_submit', 'after_due', 'after_close', 'never')),
    feedback_class_statistics text NOT NULL CHECK (feedback_class_statistics IN ('during_attempt', 'after_submit', 'after_due', 'after_close', 'never')),
    schedule_accommodation_id uuid,
    schedule_accommodation_edit_number bigint CHECK (schedule_accommodation_edit_number > 0),
    time_limit_accommodation_id uuid,
    time_limit_accommodation_edit_number bigint CHECK (time_limit_accommodation_edit_number > 0),
    assessment_attempt_limit_accommodation_id uuid,
    assessment_attempt_limit_accommodation_edit_number bigint CHECK (assessment_attempt_limit_accommodation_edit_number > 0),
    UNIQUE (student_record_id, assessment_id, assessment_attempt_number),
    CHECK (expires_at IS NULL OR expires_at >= started_at),
    CHECK ((available_at IS NULL OR due_at IS NULL OR available_at <= due_at)
       AND (due_at IS NULL OR closes_at IS NULL OR due_at <= closes_at)),
    CHECK (assessment_attempt_time_limit_seconds IS NULL OR assessment_attempt_time_limit_seconds > 0),
    CHECK (assessment_attempt_limit IS NULL OR assessment_attempt_limit > 0),
    FOREIGN KEY (schedule_accommodation_id, student_record_id, assessment_id)
        REFERENCES ple_private.student_assessment_accommodation(accommodation_id, student_record_id, assessment_id),
    FOREIGN KEY (time_limit_accommodation_id, student_record_id, assessment_id)
        REFERENCES ple_private.student_assessment_accommodation(accommodation_id, student_record_id, assessment_id),
    FOREIGN KEY (assessment_attempt_limit_accommodation_id, student_record_id, assessment_id)
        REFERENCES ple_private.student_assessment_accommodation(accommodation_id, student_record_id, assessment_id),
    CHECK ((schedule_accommodation_id IS NULL) = (schedule_accommodation_edit_number IS NULL)),
    CHECK ((time_limit_accommodation_id IS NULL) = (time_limit_accommodation_edit_number IS NULL)),
    CHECK ((assessment_attempt_limit_accommodation_id IS NULL) = (assessment_attempt_limit_accommodation_edit_number IS NULL))
);

CREATE TABLE ple_private.question_pool_selection (
    question_pool_selection_id uuid PRIMARY KEY,
    assessment_attempt_id uuid NOT NULL REFERENCES ple_private.assessment_attempt(assessment_attempt_id) ON DELETE CASCADE,
    -- This is the stable authored identity copied at issue time, not a foreign
    -- key to the mutable current Assessment Entry.  Released Assessment saves
    -- may replace current entries while this Student Work remains interpretable.
    assessment_entry_id uuid NOT NULL,
    question_pool_id uuid NOT NULL,
    question_pool_revision_number bigint NOT NULL,
    created_at timestamptz NOT NULL,
    selected_question_count integer NOT NULL CHECK (selected_question_count > 0),
    UNIQUE (question_pool_selection_id, assessment_attempt_id, assessment_entry_id),
    UNIQUE (assessment_attempt_id, assessment_entry_id),
    FOREIGN KEY (question_pool_id, question_pool_revision_number)
        REFERENCES ple_data.question_pool_revision(question_pool_id, revision_number)
);

CREATE TABLE ple_private.question_pool_selected_item (
    question_pool_selection_id uuid NOT NULL REFERENCES ple_private.question_pool_selection(question_pool_selection_id) ON DELETE CASCADE,
    member_position integer NOT NULL CHECK (member_position > 0),
    selection_position integer NOT NULL CHECK (selection_position >= 0),
    question_id text NOT NULL,
    revision_number integer NOT NULL,
    PRIMARY KEY (question_pool_selection_id, selection_position),
    UNIQUE (question_pool_selection_id, member_position),
    UNIQUE (question_pool_selection_id, member_position, question_id, revision_number),
    FOREIGN KEY (question_id, revision_number) REFERENCES ple_data.question_revision(question_id, revision_number)
);

CREATE TABLE ple_private.issued_question (
    issued_question_id uuid PRIMARY KEY,
    assessment_attempt_id uuid NOT NULL REFERENCES ple_private.assessment_attempt(assessment_attempt_id) ON DELETE CASCADE,
    assessment_entry_id uuid NOT NULL,
    assessment_content_entry_index integer NOT NULL CHECK (assessment_content_entry_index >= 0),
    issued_position integer NOT NULL CHECK (issued_position >= 0),
    question_id text NOT NULL,
    revision_number integer NOT NULL,
    -- This pre-render source-selection record is not completed reproduction
    -- evidence. Native PLE JSON retains no seed; renderer-backed Questions
    -- retain only the seed needed to obtain their later genuine hash.
    question_seed numeric(20, 0) CHECK (question_seed >= 0 AND question_seed <= 18446744073709551615),
    point_value numeric NOT NULL CHECK (point_value >= 0),
    scoring_rule text NOT NULL CHECK (scoring_rule IN ('normal', 'full_credit', 'extra_credit', 'excluded')),
    question_statistics_eligibility boolean NOT NULL,
    question_attempt_limit integer,
    question_attempt_time_limit_seconds integer,
    question_attempt_grace_seconds integer,
    question_pool_selection_id uuid,
    question_pool_member_position integer,
    UNIQUE (assessment_attempt_id, issued_position),
    FOREIGN KEY (question_id, revision_number) REFERENCES ple_data.question_revision(question_id, revision_number),
    FOREIGN KEY (question_pool_selection_id, assessment_attempt_id, assessment_entry_id)
        REFERENCES ple_private.question_pool_selection(question_pool_selection_id, assessment_attempt_id, assessment_entry_id),
    FOREIGN KEY (question_pool_selection_id, question_pool_member_position, question_id, revision_number)
        REFERENCES ple_private.question_pool_selected_item(question_pool_selection_id, member_position, question_id, revision_number),
    CHECK ((question_pool_selection_id IS NULL) = (question_pool_member_position IS NULL)),
    CHECK (question_attempt_limit IS NULL OR question_attempt_limit > 0),
    CHECK ((question_attempt_time_limit_seconds IS NULL AND question_attempt_grace_seconds IS NULL)
        OR (question_attempt_time_limit_seconds > 0 AND question_attempt_grace_seconds >= 0))
);

CREATE FUNCTION ple_private.assert_student_assessment_accommodation_scope()
RETURNS trigger LANGUAGE plpgsql SET search_path = pg_catalog, ple_data, ple_private AS $$
BEGIN
    IF NOT ple_data.student_assessment_has_course_scope(NEW.student_record_id, NEW.assessment_id) THEN
        RAISE EXCEPTION USING ERRCODE = '23514', MESSAGE = 'Accommodation requires a Student and Assessment in one Course';
    END IF;
    IF NEW.assessment_attempt_limit IS NOT NULL
       AND NEW.assessment_attempt_limit <> 1
       AND EXISTS (
           SELECT 1 FROM ple_data.assessment AS assessment
            WHERE assessment.assessment_id = NEW.assessment_id
              AND assessment.assessment_type IN ('quiz', 'exam')
       ) THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Quiz and Exam accommodations retain exactly one Assessment Attempt';
    END IF;
    RETURN NEW;
END $$;

CREATE FUNCTION ple_private.enforce_student_assessment_accommodation_edit()
RETURNS trigger LANGUAGE plpgsql SET search_path = pg_catalog, ple_private AS $$
BEGIN
    IF NEW.accommodation_id IS DISTINCT FROM OLD.accommodation_id
       OR NEW.student_record_id IS DISTINCT FROM OLD.student_record_id
       OR NEW.assessment_id IS DISTINCT FROM OLD.assessment_id
       OR NEW.created_at IS DISTINCT FROM OLD.created_at
       OR NEW.accommodation_edit_number <> OLD.accommodation_edit_number + 1 THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Accommodation identity is immutable and changes advance one Edit Number';
    END IF;
    RETURN NEW;
END $$;

-- The Student Work table owner needs this one relational fact while inserting
-- an Assessment Attempt.  The data owner has this narrow RLS read only to evaluate the
-- invariant.  Neither private nor application roles receive a general Student
-- record read grant.
RESET ROLE;
SET LOCAL ROLE ple_data_owner;
CREATE POLICY student_record_data_owner_assessment_scope_check ON ple_data.student_record
    FOR SELECT TO ple_data_owner USING (true);
CREATE FUNCTION ple_data.student_assessment_has_course_scope(
    p_student_record_id uuid, p_assessment_id uuid
) RETURNS boolean
LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_data AS $$
    SELECT EXISTS (
        SELECT 1
          FROM ple_data.student_record AS student
          JOIN ple_data.assessment AS assessment ON assessment.course_id = student.course_id
         WHERE student.student_record_id = $1 AND assessment.assessment_id = $2
    )
$$;
REVOKE ALL ON FUNCTION ple_data.student_assessment_has_course_scope(uuid, uuid) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_data.student_assessment_has_course_scope(uuid, uuid) TO ple_private_owner;

RESET ROLE;
SET LOCAL ROLE ple_private_owner;
CREATE FUNCTION ple_private.assert_assessment_attempt_scope()
RETURNS trigger LANGUAGE plpgsql SET search_path = pg_catalog, ple_data, ple_private AS $$
BEGIN
    IF NOT ple_data.student_assessment_has_course_scope(NEW.student_record_id, NEW.assessment_id) THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Assessment Attempt requires a Student and Assessment in one Course';
    END IF;
    RETURN NEW;
END $$;

CREATE FUNCTION ple_private.reject_student_work_delete()
RETURNS trigger LANGUAGE plpgsql SET search_path = pg_catalog, ple_private AS $$
BEGIN
    -- The executor deletes only Assessment Attempt roots.  PostgreSQL invokes
    -- descendants through foreign-key cascade triggers at a nested depth; no
    -- application role has DELETE on these tables or an application-deleteable
    -- parent in this ownership graph.
    IF current_user <> 'ple_unrelease_executor' AND pg_catalog.pg_trigger_depth() <= 1 THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Student Work deletion requires the Unrelease executor';
    END IF;
    RETURN OLD;
END $$;

CREATE FUNCTION ple_private.reject_assessment_attempt_rewrite()
RETURNS trigger LANGUAGE plpgsql SET search_path = pg_catalog, ple_private AS $$
BEGIN
    IF NEW.assessment_attempt_id IS DISTINCT FROM OLD.assessment_attempt_id
       OR NEW.reference_number IS DISTINCT FROM OLD.reference_number
       OR NEW.student_record_id IS DISTINCT FROM OLD.student_record_id
       OR NEW.assessment_id IS DISTINCT FROM OLD.assessment_id
       OR NEW.assessment_attempt_number IS DISTINCT FROM OLD.assessment_attempt_number
       OR NEW.started_at IS DISTINCT FROM OLD.started_at
       OR NEW.expires_at IS DISTINCT FROM OLD.expires_at
       OR ROW(NEW.assessment_title, NEW.assessment_instructions, NEW.available_at, NEW.due_at, NEW.closes_at,
              NEW.assessment_attempt_time_limit_seconds, NEW.assessment_attempt_limit, NEW.late_work_rule,
              NEW.question_variation_rule,
              NEW.assessment_question_order_rule, NEW.feedback_score,
              NEW.feedback_per_item_correctness, NEW.feedback_submitted_response,
              NEW.feedback_question_answer, NEW.feedback_question_answer_explanation, NEW.feedback_class_statistics,
              NEW.schedule_accommodation_id, NEW.schedule_accommodation_edit_number,
              NEW.time_limit_accommodation_id, NEW.time_limit_accommodation_edit_number,
              NEW.assessment_attempt_limit_accommodation_id, NEW.assessment_attempt_limit_accommodation_edit_number)
           IS DISTINCT FROM ROW(OLD.assessment_title, OLD.assessment_instructions, OLD.available_at, OLD.due_at, OLD.closes_at,
              OLD.assessment_attempt_time_limit_seconds, OLD.assessment_attempt_limit, OLD.late_work_rule,
              OLD.question_variation_rule,
              OLD.assessment_question_order_rule, OLD.feedback_score,
              OLD.feedback_per_item_correctness, OLD.feedback_submitted_response,
              OLD.feedback_question_answer, OLD.feedback_question_answer_explanation, OLD.feedback_class_statistics,
              OLD.schedule_accommodation_id, OLD.schedule_accommodation_edit_number,
              OLD.time_limit_accommodation_id, OLD.time_limit_accommodation_edit_number,
              OLD.assessment_attempt_limit_accommodation_id, OLD.assessment_attempt_limit_accommodation_edit_number) THEN
        RAISE EXCEPTION USING ERRCODE = '55000', MESSAGE = 'Assessment Attempt evidence is immutable after creation';
    END IF;
    RETURN NEW;
END $$;

CREATE FUNCTION ple_private.reject_immutable_student_work_change()
RETURNS trigger LANGUAGE plpgsql SET search_path = pg_catalog, ple_private AS $$
BEGIN RAISE EXCEPTION USING ERRCODE = '55000', MESSAGE = 'Student Work evidence is immutable'; END $$;

CREATE FUNCTION ple_private.validate_question_pool_selected_item_member()
RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_data, ple_private AS $$
BEGIN
    IF NOT EXISTS (
        SELECT 1
          FROM ple_private.question_pool_selection AS selection
          JOIN ple_data.question_pool_revision_member AS member
            ON member.question_pool_id = selection.question_pool_id
           AND member.revision_number = selection.question_pool_revision_number
         WHERE selection.question_pool_selection_id = NEW.question_pool_selection_id
           AND member.member_position = NEW.member_position
           AND member.question_id = NEW.question_id
           AND member.question_revision_number = NEW.revision_number
    ) THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Selected Question Pool member must match its retained exact Pool Revision';
    END IF;
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
        (SELECT item.member_position, item.question_id, item.revision_number
           FROM ple_private.question_pool_selected_item AS item
          WHERE item.question_pool_selection_id = selection_id
         EXCEPT
         SELECT issued.question_pool_member_position, issued.question_id, issued.revision_number
           FROM ple_private.issued_question AS issued
          WHERE issued.question_pool_selection_id = selection_id)
        UNION ALL
        (SELECT issued.question_pool_member_position, issued.question_id, issued.revision_number
           FROM ple_private.issued_question AS issued
          WHERE issued.question_pool_selection_id = selection_id
         EXCEPT
         SELECT item.member_position, item.question_id, item.revision_number
           FROM ple_private.question_pool_selected_item AS item
          WHERE item.question_pool_selection_id = selection_id)
    ) THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Question Pool Selection and Issued Questions require one exact shared item set';
    END IF;
    RETURN NULL;
END $$;

-- The source binding is immutable, so this durable Student Work fact may be
-- validated at issue time.  A native PLE source has no generator seed;
-- renderer-backed WeBWorK and iMathAS sources require one.  This trusted
-- database boundary enforces the cross-field reproduction rule (ASVS 2.2.3).
CREATE FUNCTION ple_private.validate_issued_question_reproduction()
RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_private AS $$
DECLARE source_backend text;
BEGIN
    SELECT backend INTO source_backend
      FROM ple_private.question_revision_source_binding
     WHERE question_id = NEW.question_id
       AND revision_number = NEW.revision_number;
    IF NOT FOUND OR NOT ple_private.question_backend_is_supported_for_production(source_backend)
       OR (source_backend = 'ple' AND NEW.question_seed IS NOT NULL)
       OR (source_backend = 'webwork' AND NEW.question_seed IS NULL) THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Issued Question reproduction must match its source backend';
    END IF;
    RETURN NEW;
END $$;

CREATE TRIGGER student_assessment_accommodation_has_course_scope BEFORE INSERT OR UPDATE
ON ple_private.student_assessment_accommodation FOR EACH ROW EXECUTE FUNCTION ple_private.assert_student_assessment_accommodation_scope();
CREATE TRIGGER assessment_attempt_has_course_scope BEFORE INSERT ON ple_private.assessment_attempt
FOR EACH ROW EXECUTE FUNCTION ple_private.assert_assessment_attempt_scope();
CREATE TRIGGER assessment_attempt_retains_evidence BEFORE UPDATE ON ple_private.assessment_attempt
FOR EACH ROW EXECUTE FUNCTION ple_private.reject_assessment_attempt_rewrite();
CREATE TRIGGER assessment_attempt_delete_is_guarded BEFORE DELETE ON ple_private.assessment_attempt
FOR EACH ROW EXECUTE FUNCTION ple_private.reject_student_work_delete();
CREATE TRIGGER student_assessment_accommodation_edit_is_guarded BEFORE UPDATE
ON ple_private.student_assessment_accommodation
FOR EACH ROW EXECUTE FUNCTION ple_private.enforce_student_assessment_accommodation_edit();
CREATE TRIGGER question_pool_selection_is_immutable BEFORE UPDATE ON ple_private.question_pool_selection
FOR EACH ROW EXECUTE FUNCTION ple_private.reject_immutable_student_work_change();
CREATE TRIGGER question_pool_selection_delete_is_guarded BEFORE DELETE ON ple_private.question_pool_selection
FOR EACH ROW EXECUTE FUNCTION ple_private.reject_student_work_delete();
CREATE TRIGGER question_pool_selected_item_is_immutable BEFORE UPDATE ON ple_private.question_pool_selected_item
FOR EACH ROW EXECUTE FUNCTION ple_private.reject_immutable_student_work_change();
CREATE TRIGGER question_pool_selected_item_delete_is_guarded BEFORE DELETE ON ple_private.question_pool_selected_item
FOR EACH ROW EXECUTE FUNCTION ple_private.reject_student_work_delete();
CREATE TRIGGER question_pool_selected_item_matches_retained_pool_revision BEFORE INSERT
ON ple_private.question_pool_selected_item FOR EACH ROW
EXECUTE FUNCTION ple_private.validate_question_pool_selected_item_member();
CREATE TRIGGER issued_question_is_immutable BEFORE UPDATE ON ple_private.issued_question
FOR EACH ROW EXECUTE FUNCTION ple_private.reject_immutable_student_work_change();
CREATE TRIGGER issued_question_reproduction_matches_source BEFORE INSERT OR UPDATE OF question_id, revision_number, question_seed
ON ple_private.issued_question FOR EACH ROW EXECUTE FUNCTION ple_private.validate_issued_question_reproduction();
CREATE TRIGGER issued_question_delete_is_guarded BEFORE DELETE ON ple_private.issued_question
FOR EACH ROW EXECUTE FUNCTION ple_private.reject_student_work_delete();
CREATE CONSTRAINT TRIGGER question_pool_selection_has_exact_item_count AFTER INSERT ON ple_private.question_pool_selection
DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION ple_private.validate_question_pool_selected_item_count();
CREATE CONSTRAINT TRIGGER question_pool_selected_item_count_is_exact AFTER INSERT OR UPDATE OR DELETE ON ple_private.question_pool_selected_item
DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION ple_private.validate_question_pool_selected_item_count();
CREATE CONSTRAINT TRIGGER question_pool_selection_issues_are_exact AFTER INSERT OR UPDATE OR DELETE ON ple_private.question_pool_selected_item
DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION ple_private.validate_question_pool_selection_issues();
CREATE CONSTRAINT TRIGGER issued_question_pool_source_is_exact AFTER INSERT OR UPDATE OR DELETE ON ple_private.issued_question
DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION ple_private.validate_question_pool_selection_issues();

CREATE INDEX assessment_attempt_student_assessment_lookup_idx
    ON ple_private.assessment_attempt(student_record_id, assessment_id, assessment_attempt_number DESC);
CREATE INDEX assessment_attempt_expiry_sweep_idx
    ON ple_private.assessment_attempt(expires_at, assessment_attempt_id)
    WHERE expires_at IS NOT NULL;

ALTER TABLE ple_private.student_assessment_accommodation ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.student_assessment_accommodation FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_private.assessment_attempt ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.assessment_attempt FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_private.question_pool_selection ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.question_pool_selection FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_private.question_pool_selected_item ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.question_pool_selected_item FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_private.issued_question ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.issued_question FORCE ROW LEVEL SECURITY;
REVOKE ALL ON TABLE ple_private.student_assessment_accommodation, ple_private.assessment_attempt,
    ple_private.question_pool_selection, ple_private.question_pool_selected_item, ple_private.issued_question FROM PUBLIC;
CREATE POLICY student_assessment_accommodation_private_owner_access ON ple_private.student_assessment_accommodation FOR ALL TO ple_private_owner USING (true) WITH CHECK (true);
CREATE POLICY assessment_attempt_private_owner_access ON ple_private.assessment_attempt FOR ALL TO ple_private_owner USING (true) WITH CHECK (true);
CREATE POLICY question_pool_selection_private_owner_access ON ple_private.question_pool_selection FOR ALL TO ple_private_owner USING (true) WITH CHECK (true);
CREATE POLICY question_pool_selected_item_private_owner_access ON ple_private.question_pool_selected_item FOR ALL TO ple_private_owner USING (true) WITH CHECK (true);
CREATE POLICY issued_question_private_owner_access ON ple_private.issued_question FOR ALL TO ple_private_owner USING (true) WITH CHECK (true);
REVOKE ALL ON FUNCTION ple_private.assert_student_assessment_accommodation_scope(), ple_private.assert_assessment_attempt_scope(),
    ple_private.enforce_student_assessment_accommodation_edit(), ple_private.reject_student_work_delete(), ple_private.reject_assessment_attempt_rewrite(),
    ple_private.reject_immutable_student_work_change(),
    ple_private.validate_question_pool_selected_item_count(), ple_private.validate_question_pool_selected_item_member(),
    ple_private.validate_question_pool_selection_issues(),
    ple_private.validate_issued_question_reproduction() FROM PUBLIC;

COMMENT ON TABLE ple_private.assessment_attempt IS 'Immutable effective Assessment evidence for one Student Work occurrence; its immutable Assessment Submission is the sole completion authority.';
COMMENT ON TABLE ple_private.issued_question IS 'Pre-render source-selection record: exact Assessment Entry identity, Question Revision, optional renderer seed, per-question policy, scoring, statistics, and pool-selection evidence for one issued position.';

RESET ROLE;
