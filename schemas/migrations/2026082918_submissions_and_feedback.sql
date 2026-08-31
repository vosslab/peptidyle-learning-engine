-- SD1 private submitted responses and Student-visible feedback release state.

SET LOCAL ROLE ple_private_owner;
CREATE TABLE ple_private.question_submission (
    submission_id uuid PRIMARY KEY,
    question_attempt_id uuid NOT NULL UNIQUE REFERENCES ple_private.question_attempt (question_attempt_id),
    submitted_at timestamp with time zone NOT NULL,
    student_response jsonb NOT NULL,
    CONSTRAINT question_submission_student_response_is_object CHECK (jsonb_typeof(student_response) = 'object')
);
CREATE TABLE ple_private.assignment_submission (
    assignment_submission_id uuid PRIMARY KEY,
    assignment_attempt_id uuid NOT NULL UNIQUE REFERENCES ple_private.assignment_attempt (assignment_attempt_id),
    submitted_at timestamp with time zone NOT NULL,
    authorized_by_account_id uuid NOT NULL,
    receipt jsonb NOT NULL CHECK (jsonb_typeof(receipt) = 'object')
);
-- ASVS 8.2.2: accepted Student Responses and finalization Receipts preserve
-- the evidence presented to grading and may not be rewritten in place.
CREATE FUNCTION ple_private.reject_submission_change()
RETURNS trigger LANGUAGE plpgsql SET search_path = pg_catalog, ple_private AS $$
BEGIN
    RAISE EXCEPTION USING ERRCODE = '23514', MESSAGE = 'accepted submission evidence is immutable';
END
$$;
CREATE TRIGGER question_submission_is_immutable
BEFORE UPDATE OR DELETE ON ple_private.question_submission
FOR EACH ROW EXECUTE FUNCTION ple_private.reject_submission_change();
CREATE TRIGGER assignment_submission_is_immutable
BEFORE UPDATE OR DELETE ON ple_private.assignment_submission
FOR EACH ROW EXECUTE FUNCTION ple_private.reject_submission_change();
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
ALTER TABLE ple_private.assignment_submission ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.assignment_submission FORCE ROW LEVEL SECURITY;
REVOKE ALL PRIVILEGES ON TABLE ple_private.question_submission, ple_private.assignment_submission, ple_private.student_feedback_release FROM PUBLIC;
REVOKE ALL PRIVILEGES ON FUNCTION ple_private.reject_submission_change() FROM PUBLIC;
COMMENT ON TABLE ple_private.question_submission IS 'Immutable accepted Student Response for one exact Question Attempt.';
RESET ROLE;
