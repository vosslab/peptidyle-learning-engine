-- SD1 deterministic automated-grading operations and immutable receipts.

SET LOCAL ROLE ple_private_owner;
CREATE TABLE ple_private.automated_grading_operation (
    operation_id uuid PRIMARY KEY,
    submission_id uuid NOT NULL UNIQUE REFERENCES ple_private.question_submission (submission_id),
    state text NOT NULL CHECK (state IN ('queued', 'running', 'completed', 'failed')),
    created_at timestamp with time zone NOT NULL,
    completed_at timestamp with time zone,
    CONSTRAINT automated_grading_operation_completion_is_ordered CHECK (completed_at IS NULL OR completed_at >= created_at)
);
GRANT USAGE ON SCHEMA ple_private TO ple_audit_owner;
GRANT REFERENCES ON TABLE ple_private.automated_grading_operation TO ple_audit_owner;
ALTER TABLE ple_private.automated_grading_operation ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.automated_grading_operation FORCE ROW LEVEL SECURITY;
REVOKE ALL PRIVILEGES ON TABLE ple_private.automated_grading_operation FROM PUBLIC;
RESET ROLE;
SET LOCAL ROLE ple_audit_owner;
CREATE TABLE ple_audit.automated_grading_receipt (
    receipt_id uuid PRIMARY KEY,
    operation_id uuid NOT NULL UNIQUE REFERENCES ple_private.automated_grading_operation (operation_id),
    committed_at timestamp with time zone NOT NULL,
    outcome jsonb NOT NULL CHECK (jsonb_typeof(outcome) = 'object'),
    digest bytea NOT NULL CHECK (pg_catalog.octet_length(digest) = 32)
);
ALTER TABLE ple_audit.automated_grading_receipt ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_audit.automated_grading_receipt FORCE ROW LEVEL SECURITY;
REVOKE ALL PRIVILEGES ON TABLE ple_audit.automated_grading_receipt FROM PUBLIC;
RESET ROLE;
