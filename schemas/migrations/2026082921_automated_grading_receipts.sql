-- SD1 deterministic automated-grading operations and immutable receipts.

SET LOCAL ROLE ple_private_owner;
CREATE TABLE ple_private.automated_grading_operation (
    operation_id uuid PRIMARY KEY,
    submission_id uuid NOT NULL UNIQUE REFERENCES ple_private.question_submission (submission_id),
    state text NOT NULL CHECK (state IN ('queued', 'running', 'completed', 'failed')),
    created_at timestamp with time zone NOT NULL,
    completed_at timestamp with time zone,
    CONSTRAINT automated_grading_operation_completion_is_ordered CHECK (completed_at IS NULL OR completed_at >= created_at),
    CONSTRAINT automated_grading_operation_matches_submission UNIQUE (operation_id, submission_id)
);
CREATE TABLE ple_private.grading_result (
    grading_result_id uuid PRIMARY KEY,
    submission_id uuid NOT NULL UNIQUE,
    operation_id uuid NOT NULL UNIQUE,
    question_attempt_id uuid NOT NULL UNIQUE REFERENCES ple_private.question_attempt (question_attempt_id),
    correct boolean NOT NULL,
    points_earned numeric NOT NULL CHECK (points_earned >= 0),
    points_possible numeric NOT NULL CHECK (points_possible >= 0),
    recorded_at timestamp with time zone NOT NULL,
    CONSTRAINT grading_result_points_are_ordered CHECK (points_earned <= points_possible),
    CONSTRAINT grading_result_submission_matches_attempt
        FOREIGN KEY (submission_id, question_attempt_id)
        REFERENCES ple_private.question_submission (submission_id, question_attempt_id),
    CONSTRAINT grading_result_matches_operation_submission
        FOREIGN KEY (operation_id, submission_id)
        REFERENCES ple_private.automated_grading_operation (operation_id, submission_id),
    CONSTRAINT grading_result_operation_pair_is_unique UNIQUE (operation_id, grading_result_id)
);
GRANT USAGE ON SCHEMA ple_private TO ple_audit_owner;
GRANT REFERENCES ON TABLE ple_private.automated_grading_operation, ple_private.grading_result
    TO ple_audit_owner;
ALTER TABLE ple_private.automated_grading_operation ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.automated_grading_operation FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_private.grading_result ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.grading_result FORCE ROW LEVEL SECURITY;
REVOKE ALL PRIVILEGES ON TABLE ple_private.automated_grading_operation, ple_private.grading_result FROM PUBLIC;
RESET ROLE;
SET LOCAL ROLE ple_audit_owner;
CREATE TABLE ple_audit.automated_grading_receipt (
    receipt_id uuid PRIMARY KEY,
    operation_id uuid NOT NULL,
    grading_result_id uuid NOT NULL UNIQUE,
    committed_at timestamp with time zone NOT NULL,
    digest bytea NOT NULL CHECK (pg_catalog.octet_length(digest) = 32),
    CONSTRAINT automated_grading_receipt_matches_result_operation
        FOREIGN KEY (operation_id, grading_result_id)
        REFERENCES ple_private.grading_result (operation_id, grading_result_id)
);
ALTER TABLE ple_audit.automated_grading_receipt ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_audit.automated_grading_receipt FORCE ROW LEVEL SECURITY;
REVOKE ALL PRIVILEGES ON TABLE ple_audit.automated_grading_receipt FROM PUBLIC;
RESET ROLE;
