-- SD1 private submitted responses and Student-visible feedback release state.

SET LOCAL ROLE ple_private_owner;
CREATE TABLE ple_private.question_submission (
    submission_id uuid PRIMARY KEY,
    question_attempt_id uuid NOT NULL UNIQUE REFERENCES ple_private.question_attempt (question_attempt_id),
    submitted_at timestamp with time zone NOT NULL,
    student_response jsonb NOT NULL,
    CONSTRAINT question_submission_student_response_is_object CHECK (jsonb_typeof(student_response) = 'object'),
    CONSTRAINT question_submission_id_and_attempt_are_unique
        UNIQUE (submission_id, question_attempt_id)
);
CREATE TABLE ple_private.assignment_submission (
    assignment_submission_id uuid PRIMARY KEY,
    assignment_attempt_id uuid NOT NULL UNIQUE REFERENCES ple_private.assignment_attempt (assignment_attempt_id),
    submitted_at timestamp with time zone NOT NULL,
    authorized_by_account_id uuid NOT NULL,
    receipt jsonb NOT NULL CHECK (jsonb_typeof(receipt) = 'object')
);
CREATE FUNCTION ple_private.enforce_question_attempt_submission_state()
RETURNS trigger LANGUAGE plpgsql SET search_path = pg_catalog, ple_private AS $$
DECLARE
    has_submission boolean;
BEGIN
    SELECT EXISTS (
        SELECT 1
        FROM ple_private.question_submission
        WHERE question_attempt_id = NEW.question_attempt_id
    ) INTO has_submission;
    IF (NEW.question_attempt_state = 'submission_accepted') <> has_submission THEN
        RAISE EXCEPTION USING
            ERRCODE = '23514',
            MESSAGE = 'Submission Accepted requires one Question Submission and other states require none';
    END IF;
    RETURN NULL;
END
$$;
CREATE FUNCTION ple_private.enforce_question_submission_attempt_state()
RETURNS trigger LANGUAGE plpgsql SET search_path = pg_catalog, ple_private AS $$
DECLARE
    attempt_state text;
    attempt_issued_at timestamp with time zone;
BEGIN
    SELECT question_attempt_state, issued_at
      INTO attempt_state, attempt_issued_at
      FROM ple_private.question_attempt
     WHERE question_attempt_id = NEW.question_attempt_id;
    IF attempt_state <> 'submission_accepted' THEN
        RAISE EXCEPTION USING
            ERRCODE = '23514',
            MESSAGE = 'Question Submission requires Submission Accepted Question Attempt State';
    END IF;
    IF NEW.submitted_at < attempt_issued_at THEN
        RAISE EXCEPTION USING
            ERRCODE = '23514',
            MESSAGE = 'Question Submission cannot precede Question Attempt issuance';
    END IF;
    RETURN NULL;
END
$$;
CREATE CONSTRAINT TRIGGER question_attempt_submission_state_is_exact
AFTER INSERT OR UPDATE OF question_attempt_state ON ple_private.question_attempt
DEFERRABLE INITIALLY DEFERRED
FOR EACH ROW EXECUTE FUNCTION ple_private.enforce_question_attempt_submission_state();
CREATE CONSTRAINT TRIGGER question_submission_requires_accepted_attempt_state
AFTER INSERT ON ple_private.question_submission
DEFERRABLE INITIALLY DEFERRED
FOR EACH ROW EXECUTE FUNCTION ple_private.enforce_question_submission_attempt_state();
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
ALTER TABLE ple_private.question_submission ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.question_submission FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_private.assignment_submission ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.assignment_submission FORCE ROW LEVEL SECURITY;
REVOKE ALL PRIVILEGES ON TABLE ple_private.question_submission, ple_private.assignment_submission FROM PUBLIC;
REVOKE ALL PRIVILEGES ON FUNCTION ple_private.reject_submission_change() FROM PUBLIC;
REVOKE ALL PRIVILEGES ON FUNCTION ple_private.enforce_question_attempt_submission_state() FROM PUBLIC;
REVOKE ALL PRIVILEGES ON FUNCTION ple_private.enforce_question_submission_attempt_state() FROM PUBLIC;
COMMENT ON TABLE ple_private.question_submission IS 'Immutable accepted Student Response for one exact Question Attempt.';
RESET ROLE;
