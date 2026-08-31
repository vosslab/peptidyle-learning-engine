-- SD1 Student Assignment Attempts, Issued Questions, and Question Attempts.

SET LOCAL ROLE ple_data_owner;
GRANT USAGE ON SCHEMA ple_data TO ple_private_owner;
GRANT REFERENCES ON TABLE ple_data.student_record, ple_data.assignment,
    ple_data.assignment_revision, ple_data.published_question_version TO ple_private_owner;
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
    issued_question_id text PRIMARY KEY,
    assignment_attempt_id uuid NOT NULL REFERENCES ple_private.assignment_attempt (assignment_attempt_id),
    assignment_entry_id uuid NOT NULL,
    question_id text NOT NULL,
    version_number integer NOT NULL,
    issued_position integer NOT NULL CHECK (issued_position >= 0),
    point_value numeric NOT NULL CHECK (point_value >= 0),
    scoring_rule text NOT NULL CHECK (scoring_rule IN ('normal', 'full_credit', 'extra_credit', 'excluded')),
    CONSTRAINT issued_question_version_matches FOREIGN KEY (question_id, version_number)
        REFERENCES ple_data.published_question_version (question_id, version_number),
    CONSTRAINT issued_question_delivery_order_is_unique UNIQUE (assignment_attempt_id, issued_position)
);
CREATE TABLE ple_private.question_attempt (
    question_attempt_id uuid PRIMARY KEY,
    issued_question_id text NOT NULL REFERENCES ple_private.issued_question (issued_question_id),
    issued_at timestamp with time zone NOT NULL,
    deadline_at timestamp with time zone,
    CONSTRAINT question_attempt_deadline_is_ordered CHECK (deadline_at IS NULL OR deadline_at >= issued_at)
);
ALTER TABLE ple_private.assignment_attempt ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.assignment_attempt FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_private.issued_question ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.issued_question FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_private.question_attempt ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.question_attempt FORCE ROW LEVEL SECURITY;
REVOKE ALL PRIVILEGES ON TABLE ple_private.assignment_attempt, ple_private.issued_question, ple_private.question_attempt FROM PUBLIC;
COMMENT ON TABLE ple_private.issued_question IS 'Immutable selected Question Version and Assignment Entry evidence for one Student Assignment Attempt.';
RESET ROLE;
