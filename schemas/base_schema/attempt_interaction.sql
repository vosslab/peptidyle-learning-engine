-- Question-Attempt interaction and submission evidence below an Assignment Attempt.

SET LOCAL ROLE ple_private_owner;

CREATE TABLE ple_private.question_attempt (
    question_attempt_id uuid PRIMARY KEY,
    issued_question_id uuid NOT NULL UNIQUE REFERENCES ple_private.issued_question(issued_question_id) ON DELETE CASCADE,
    question_seed numeric(20, 0) NOT NULL CHECK (question_seed >= 0 AND question_seed <= 18446744073709551615),
    generated_parameter_sha256 text NOT NULL CHECK (generated_parameter_sha256 ~ '^[0-9a-f]{64}$'),
    issued_at timestamptz NOT NULL,
    deadline_at timestamptz,
    submitted_at timestamptz,
    question_attempt_state text NOT NULL CHECK (question_attempt_state IN ('open', 'submission_accepted', 'closed_at_deadline')),
    backend_name text NOT NULL CHECK (char_length(btrim(backend_name)) BETWEEN 1 AND 100),
    backend_version text NOT NULL CHECK (char_length(btrim(backend_version)) BETWEEN 1 AND 100),
    renderer_name text,
    renderer_version text,
    source_object_id uuid REFERENCES ple_private.object_record(object_id),
    source_object_checksum bytea CHECK (source_object_checksum IS NULL OR octet_length(source_object_checksum) = 32),
    grader_name text NOT NULL CHECK (char_length(btrim(grader_name)) BETWEEN 1 AND 100),
    grader_version text NOT NULL CHECK (char_length(btrim(grader_version)) BETWEEN 1 AND 100),
    rendered_question_sha256 bytea NOT NULL CHECK (octet_length(rendered_question_sha256) = 32),
    issued_capability text NOT NULL CHECK (issued_capability IN ('question_presentation', 'ple_question_json_presentation', 'webwork_presentation', 'not_applicable')),
    CHECK (deadline_at IS NULL OR deadline_at >= issued_at),
    CHECK ((renderer_name IS NULL) = (renderer_version IS NULL)),
    CHECK ((source_object_id IS NULL) = (source_object_checksum IS NULL)),
    CHECK ((question_attempt_state = 'submission_accepted') = (submitted_at IS NOT NULL)),
    CHECK (submitted_at IS NULL OR submitted_at >= issued_at)
);

CREATE TABLE ple_private.assignment_attempt_saved_response (
    question_attempt_id uuid PRIMARY KEY REFERENCES ple_private.question_attempt(question_attempt_id) ON DELETE CASCADE,
    student_response jsonb NOT NULL CHECK (jsonb_typeof(student_response) = 'object'),
    saved_at timestamptz NOT NULL
);

CREATE TABLE ple_private.question_submission (
    submission_id uuid PRIMARY KEY,
    question_attempt_id uuid NOT NULL UNIQUE REFERENCES ple_private.question_attempt(question_attempt_id) ON DELETE CASCADE,
    submitted_at timestamptz NOT NULL,
    student_response jsonb NOT NULL CHECK (jsonb_typeof(student_response) = 'object'),
    UNIQUE (submission_id, question_attempt_id)
);

CREATE TABLE ple_private.assignment_submission (
    assignment_submission_id uuid PRIMARY KEY,
    assignment_attempt_id uuid NOT NULL UNIQUE REFERENCES ple_private.assignment_attempt(assignment_attempt_id) ON DELETE CASCADE,
    submitted_at timestamptz NOT NULL,
    authorized_by_account_id uuid NOT NULL REFERENCES ple_private.account(account_id),
    receipt jsonb NOT NULL CHECK (jsonb_typeof(receipt) = 'object')
);

CREATE FUNCTION ple_private.enforce_question_attempt_state_transition()
RETURNS trigger LANGUAGE plpgsql SET search_path = pg_catalog, ple_private AS $$
BEGIN
    IF NEW.question_attempt_id IS DISTINCT FROM OLD.question_attempt_id
       OR NEW.issued_question_id IS DISTINCT FROM OLD.issued_question_id
       OR NEW.question_seed IS DISTINCT FROM OLD.question_seed
       OR NEW.generated_parameter_sha256 IS DISTINCT FROM OLD.generated_parameter_sha256
       OR NEW.issued_at IS DISTINCT FROM OLD.issued_at
       OR NEW.deadline_at IS DISTINCT FROM OLD.deadline_at
       OR NEW.backend_name IS DISTINCT FROM OLD.backend_name
       OR NEW.backend_version IS DISTINCT FROM OLD.backend_version
       OR NEW.renderer_name IS DISTINCT FROM OLD.renderer_name
       OR NEW.renderer_version IS DISTINCT FROM OLD.renderer_version
       OR NEW.source_object_id IS DISTINCT FROM OLD.source_object_id
       OR NEW.source_object_checksum IS DISTINCT FROM OLD.source_object_checksum
       OR NEW.grader_name IS DISTINCT FROM OLD.grader_name
       OR NEW.grader_version IS DISTINCT FROM OLD.grader_version
       OR NEW.rendered_question_sha256 IS DISTINCT FROM OLD.rendered_question_sha256
       OR NEW.issued_capability IS DISTINCT FROM OLD.issued_capability THEN
        RAISE EXCEPTION USING ERRCODE = '55000', MESSAGE = 'Question Attempt reproduction evidence is immutable';
    END IF;
    IF OLD.question_attempt_state <> 'open' AND NEW.question_attempt_state <> OLD.question_attempt_state THEN
        RAISE EXCEPTION USING ERRCODE = '23514', MESSAGE = 'Question Attempt State is forward-only';
    END IF;
    IF NEW.question_attempt_state = 'open' AND NEW.submitted_at IS NOT NULL THEN
        RAISE EXCEPTION USING ERRCODE = '23514', MESSAGE = 'Open Question Attempt has no submission time';
    END IF;
    RETURN NEW;
END $$;

CREATE FUNCTION ple_private.validate_question_attempt_issue()
RETURNS trigger LANGUAGE plpgsql SET search_path = pg_catalog, ple_private AS $$
DECLARE issued ple_private.issued_question%ROWTYPE;
BEGIN
    SELECT * INTO issued FROM ple_private.issued_question
     WHERE issued_question_id = NEW.issued_question_id;
    IF NOT FOUND OR issued.question_seed <> NEW.question_seed THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Question Attempt requires its Issued Question exact seed';
    END IF;
    IF issued.question_attempt_time_limit_seconds IS NOT NULL
       AND NEW.deadline_at IS DISTINCT FROM NEW.issued_at
            + pg_catalog.make_interval(secs => issued.question_attempt_time_limit_seconds
                + issued.question_attempt_grace_seconds) THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Question Attempt deadline must retain its issued per-question limit and grace';
    END IF;
    RETURN NEW;
END $$;

CREATE FUNCTION ple_private.enforce_question_submission_state()
RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_private AS $$
DECLARE question_attempt ple_private.question_attempt%ROWTYPE;
BEGIN
    SELECT * INTO question_attempt FROM ple_private.question_attempt WHERE question_attempt_id = NEW.question_attempt_id;
    IF NOT FOUND OR question_attempt.question_attempt_state <> 'submission_accepted'
       OR question_attempt.submitted_at IS DISTINCT FROM NEW.submitted_at THEN
        RAISE EXCEPTION USING ERRCODE = '23514', MESSAGE = 'Question Submission requires its exact accepted Question Attempt state';
    END IF;
    RETURN NEW;
END $$;

CREATE FUNCTION ple_private.reject_student_work_update()
RETURNS trigger LANGUAGE plpgsql SET search_path = pg_catalog, ple_private AS $$
BEGIN RAISE EXCEPTION USING ERRCODE = '55000', MESSAGE = 'Student Work evidence is immutable'; END $$;

CREATE FUNCTION ple_private.enforce_saved_response_change()
RETURNS trigger LANGUAGE plpgsql SET search_path = pg_catalog, ple_private AS $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM ple_private.question_attempt
         WHERE question_attempt_id = NEW.question_attempt_id
           AND question_attempt_state = 'open'
    ) THEN
        RAISE EXCEPTION USING ERRCODE = '55000',
            MESSAGE = 'Saved Student Response changes require an open Question Attempt';
    END IF;
    RETURN NEW;
END $$;

CREATE TRIGGER question_attempt_state_is_forward_only BEFORE UPDATE ON ple_private.question_attempt
FOR EACH ROW EXECUTE FUNCTION ple_private.enforce_question_attempt_state_transition();
CREATE TRIGGER question_attempt_has_exact_issue BEFORE INSERT OR UPDATE OF issued_question_id, question_seed, issued_at, deadline_at ON ple_private.question_attempt
FOR EACH ROW EXECUTE FUNCTION ple_private.validate_question_attempt_issue();
CREATE TRIGGER question_attempt_delete_is_guarded BEFORE DELETE ON ple_private.question_attempt
FOR EACH ROW EXECUTE FUNCTION ple_private.reject_student_work_delete();
CREATE TRIGGER saved_response_changes_require_open_attempt BEFORE UPDATE ON ple_private.assignment_attempt_saved_response
FOR EACH ROW EXECUTE FUNCTION ple_private.enforce_saved_response_change();
CREATE TRIGGER saved_response_delete_is_guarded BEFORE DELETE ON ple_private.assignment_attempt_saved_response
FOR EACH ROW EXECUTE FUNCTION ple_private.reject_student_work_delete();
CREATE TRIGGER question_submission_is_immutable BEFORE UPDATE ON ple_private.question_submission
FOR EACH ROW EXECUTE FUNCTION ple_private.reject_student_work_update();
CREATE TRIGGER question_submission_delete_is_guarded BEFORE DELETE ON ple_private.question_submission
FOR EACH ROW EXECUTE FUNCTION ple_private.reject_student_work_delete();
CREATE TRIGGER assignment_submission_is_immutable BEFORE UPDATE ON ple_private.assignment_submission
FOR EACH ROW EXECUTE FUNCTION ple_private.reject_student_work_update();
CREATE TRIGGER assignment_submission_delete_is_guarded BEFORE DELETE ON ple_private.assignment_submission
FOR EACH ROW EXECUTE FUNCTION ple_private.reject_student_work_delete();
CREATE CONSTRAINT TRIGGER question_submission_matches_attempt AFTER INSERT OR UPDATE ON ple_private.question_submission
DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION ple_private.enforce_question_submission_state();

CREATE INDEX assignment_submission_attempt_idx ON ple_private.assignment_submission(assignment_attempt_id);
CREATE INDEX question_submission_attempt_idx ON ple_private.question_submission(question_attempt_id);

ALTER TABLE ple_private.question_attempt ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.question_attempt FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_private.assignment_attempt_saved_response ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.assignment_attempt_saved_response FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_private.question_submission ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.question_submission FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_private.assignment_submission ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.assignment_submission FORCE ROW LEVEL SECURITY;
REVOKE ALL ON TABLE ple_private.question_attempt, ple_private.assignment_attempt_saved_response,
    ple_private.question_submission, ple_private.assignment_submission FROM PUBLIC;
CREATE POLICY question_attempt_private_owner_access ON ple_private.question_attempt FOR ALL TO ple_private_owner USING (true) WITH CHECK (true);
-- Narrow API SECURITY DEFINER delivery routines resolve a Student-owned native
-- submission through this evidence.  The API owner has no login and ple_app
-- receives only procedure execution, so the authorization boundary remains
-- the procedure predicates in delivery.sql.
CREATE POLICY question_attempt_api_owner_delivery_read ON ple_private.question_attempt
    FOR SELECT TO ple_api_owner USING (true);
CREATE POLICY saved_response_private_owner_access ON ple_private.assignment_attempt_saved_response FOR ALL TO ple_private_owner USING (true) WITH CHECK (true);
CREATE POLICY question_submission_private_owner_access ON ple_private.question_submission FOR ALL TO ple_private_owner USING (true) WITH CHECK (true);
CREATE POLICY assignment_submission_private_owner_access ON ple_private.assignment_submission FOR ALL TO ple_private_owner USING (true) WITH CHECK (true);
GRANT SELECT ON TABLE ple_private.question_attempt TO ple_api_owner;
REVOKE ALL ON FUNCTION ple_private.enforce_question_attempt_state_transition(),
    ple_private.validate_question_attempt_issue(), ple_private.enforce_question_submission_state(),
    ple_private.reject_student_work_update(), ple_private.enforce_saved_response_change() FROM PUBLIC;

COMMENT ON TABLE ple_private.question_attempt IS 'Exact reproduction and operational evidence for one Issued Question; mutable current Question content is not an interpretation source.';
COMMENT ON TABLE ple_private.assignment_attempt_saved_response IS 'Private current response state before submission; root-owned by its Question Attempt.';

RESET ROLE;
