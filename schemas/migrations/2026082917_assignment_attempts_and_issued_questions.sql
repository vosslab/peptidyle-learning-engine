-- SD1 Assignment Attempts, Issued Questions, and Question Attempts.

SET LOCAL ROLE ple_data_owner;
GRANT USAGE ON SCHEMA ple_data TO ple_private_owner;
GRANT REFERENCES ON TABLE ple_data.student_record, ple_data.assignment,
    ple_data.assignment_revision, ple_data.question_revision TO ple_private_owner;
RESET ROLE;
SET LOCAL ROLE ple_private_owner;
CREATE TABLE ple_private.assignment_attempt (
    assignment_attempt_id uuid PRIMARY KEY,
    student_record_id uuid NOT NULL REFERENCES ple_data.student_record (student_record_id),
    assignment_id uuid NOT NULL REFERENCES ple_data.assignment (assignment_id),
    assignment_revision_id uuid NOT NULL,
    started_at timestamp with time zone NOT NULL,
    completed_at timestamp with time zone,
    CONSTRAINT assignment_attempt_completion_is_ordered CHECK (completed_at IS NULL OR completed_at >= started_at),
    CONSTRAINT assignment_attempt_revision_belongs_to_assignment
        FOREIGN KEY (assignment_id, assignment_revision_id)
        REFERENCES ple_data.assignment_revision (assignment_id, assignment_revision_id)
);
CREATE TABLE ple_private.issued_question (
    issued_question_id uuid PRIMARY KEY,
    assignment_attempt_id uuid NOT NULL REFERENCES ple_private.assignment_attempt (assignment_attempt_id),
    assignment_entry_id uuid NOT NULL,
    question_id text NOT NULL,
    revision_number integer NOT NULL,
    issued_position integer NOT NULL CHECK (issued_position >= 0),
    point_value numeric NOT NULL CHECK (point_value >= 0),
    scoring_rule text NOT NULL CHECK (scoring_rule IN ('normal', 'full_credit', 'extra_credit', 'excluded')),
    statistics_eligible boolean NOT NULL,
    CONSTRAINT issued_question_revision_matches FOREIGN KEY (question_id, revision_number)
        REFERENCES ple_data.question_revision (question_id, revision_number),
    CONSTRAINT issued_question_delivery_order_is_unique UNIQUE (assignment_attempt_id, issued_position)
);
CREATE TRIGGER assignment_attempt_requires_released_revision
BEFORE INSERT OR UPDATE OF assignment_id, assignment_revision_id ON ple_private.assignment_attempt
FOR EACH ROW EXECUTE FUNCTION ple_data.require_released_assignment_revision();
CREATE TABLE ple_private.question_attempt (
    question_attempt_id uuid PRIMARY KEY,
    issued_question_id uuid NOT NULL REFERENCES ple_private.issued_question (issued_question_id),
    question_seed numeric(20, 0) NOT NULL CONSTRAINT question_attempt_seed_is_u64 CHECK (
        question_seed >= 0 AND question_seed <= 18446744073709551615
    ),
    generated_parameter_sha256 text NOT NULL
        CONSTRAINT question_attempt_generated_parameter_sha256_is_lowercase_hex
        CHECK (generated_parameter_sha256 ~ '^[0-9a-f]{64}$'),
    issued_at timestamp with time zone NOT NULL,
    deadline_at timestamp with time zone,
    question_attempt_state text NOT NULL CONSTRAINT question_attempt_state_is_closed CHECK (
        question_attempt_state IN ('open', 'submission_accepted', 'closed_at_deadline')
    ),
    reproduction_details jsonb NOT NULL CHECK (jsonb_typeof(reproduction_details) = 'object'),
    CONSTRAINT question_attempt_deadline_is_ordered CHECK (deadline_at IS NULL OR deadline_at >= issued_at)
);
CREATE FUNCTION ple_private.enforce_question_attempt_state_transition()
RETURNS trigger LANGUAGE plpgsql SET search_path = pg_catalog, ple_private AS $$
BEGIN
    IF OLD.question_attempt_state <> 'open'
       AND NEW.question_attempt_state <> OLD.question_attempt_state THEN
        RAISE EXCEPTION USING
            ERRCODE = '23514',
            MESSAGE = 'Question Attempt State cannot leave Submission Accepted or Closed at Deadline';
    END IF;
    RETURN NEW;
END
$$;
CREATE TRIGGER question_attempt_state_transition_is_forward_only
BEFORE UPDATE OF question_attempt_state ON ple_private.question_attempt
FOR EACH ROW EXECUTE FUNCTION ple_private.enforce_question_attempt_state_transition();
ALTER TABLE ple_private.assignment_attempt ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.assignment_attempt FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_private.issued_question ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.issued_question FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_private.question_attempt ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.question_attempt FORCE ROW LEVEL SECURITY;
REVOKE ALL PRIVILEGES ON TABLE ple_private.assignment_attempt, ple_private.issued_question, ple_private.question_attempt FROM PUBLIC;
REVOKE ALL PRIVILEGES ON FUNCTION ple_private.enforce_question_attempt_state_transition() FROM PUBLIC;
GRANT REFERENCES ON TABLE ple_private.assignment_attempt, ple_private.issued_question TO ple_audit_owner;
COMMENT ON TABLE ple_private.issued_question IS 'Immutable selected Question Revision and Assignment Entry evidence for one Assignment Attempt, including its issue-time Question Statistics Eligibility.';
COMMENT ON COLUMN ple_private.question_attempt.reproduction_details IS
    'Exact private Question Attempt Reproduction Details used for reproduction and grading.';
COMMENT ON COLUMN ple_private.question_attempt.question_seed IS
    'Exact unsigned Question Seed that reproduces this Question Attempt variation.';
COMMENT ON COLUMN ple_private.question_attempt.generated_parameter_sha256 IS
    'SHA-256 of generated parameters, retained as mismatch evidence without storing generated values.';
COMMENT ON COLUMN ple_private.question_attempt.question_attempt_state IS
    'Question Attempt State: open, submission accepted, or closed at deadline.';
RESET ROLE;
