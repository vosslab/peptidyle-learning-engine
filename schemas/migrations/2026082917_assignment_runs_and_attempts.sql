-- SD1 private Student runs and exact immutable question-attempt evidence.

SET LOCAL ROLE ple_data_owner;
GRANT USAGE ON SCHEMA ple_data TO ple_private_owner;
GRANT REFERENCES ON TABLE ple_data.assignment_enrollment, ple_data.published_question_version TO ple_private_owner;
RESET ROLE;
SET LOCAL ROLE ple_private_owner;
CREATE TABLE ple_private.assignment_run (
    run_id uuid PRIMARY KEY,
    enrollment_id uuid NOT NULL REFERENCES ple_data.assignment_enrollment (enrollment_id),
    started_at timestamp with time zone NOT NULL,
    submitted_at timestamp with time zone,
    status text NOT NULL CHECK (status IN ('active', 'submitted', 'abandoned')),
    CONSTRAINT assignment_run_terminal_state_matches CHECK (
        (status = 'active' AND submitted_at IS NULL) OR (status <> 'active' AND submitted_at IS NOT NULL AND submitted_at >= started_at)
    )
);
CREATE TABLE ple_private.question_attempt (
    attempt_id uuid PRIMARY KEY,
    run_id uuid NOT NULL REFERENCES ple_private.assignment_run (run_id),
    problem_id uuid NOT NULL REFERENCES ple_data.published_question_version (problem_id),
    version_id uuid NOT NULL REFERENCES ple_data.published_question_version (version_id),
    issued_at timestamp with time zone NOT NULL,
    submitted_at timestamp with time zone,
    response jsonb,
    CONSTRAINT question_attempt_submission_is_ordered CHECK (submitted_at IS NULL OR submitted_at >= issued_at)
);
ALTER TABLE ple_private.assignment_run ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.assignment_run FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_private.question_attempt ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.question_attempt FORCE ROW LEVEL SECURITY;
REVOKE ALL PRIVILEGES ON TABLE ple_private.assignment_run, ple_private.question_attempt FROM PUBLIC;
COMMENT ON TABLE ple_private.question_attempt IS 'FERPA-bearing private attempt state pinned to one immutable published question version.';
RESET ROLE;
