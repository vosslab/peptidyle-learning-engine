-- SD1 private submitted responses and Student-visible feedback release state.

SET LOCAL ROLE ple_private_owner;
CREATE TABLE ple_private.question_submission (
    submission_id uuid PRIMARY KEY,
    attempt_id uuid NOT NULL UNIQUE REFERENCES ple_private.question_attempt (attempt_id),
    submitted_at timestamp with time zone NOT NULL,
    response jsonb NOT NULL,
    grading_receipt jsonb,
    CONSTRAINT question_submission_response_is_object CHECK (jsonb_typeof(response) = 'object'),
    CONSTRAINT question_submission_receipt_is_object CHECK (grading_receipt IS NULL OR jsonb_typeof(grading_receipt) = 'object')
);
CREATE TABLE ple_private.student_feedback_release (
    release_id uuid PRIMARY KEY,
    submission_id uuid NOT NULL REFERENCES ple_private.question_submission (submission_id),
    released_at timestamp with time zone NOT NULL,
    projection jsonb NOT NULL CHECK (jsonb_typeof(projection) = 'object'),
    CONSTRAINT student_feedback_release_is_unique UNIQUE (submission_id, released_at)
);
ALTER TABLE ple_private.question_submission ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.question_submission FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_private.student_feedback_release ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.student_feedback_release FORCE ROW LEVEL SECURITY;
REVOKE ALL PRIVILEGES ON TABLE ple_private.question_submission, ple_private.student_feedback_release FROM PUBLIC;
COMMENT ON TABLE ple_private.question_submission IS 'Private submitted Student response and server-only grading receipt.';
RESET ROLE;
