-- SD1 immutable forced-question corrections and recalculation evidence.

SET LOCAL ROLE ple_private_owner;
GRANT USAGE ON SCHEMA ple_private TO ple_data_owner;
GRANT REFERENCES ON TABLE ple_private.account TO ple_data_owner;
RESET ROLE;

SET LOCAL ROLE ple_data_owner;
CREATE TABLE ple_data.forced_question_correction (
    correction_id uuid PRIMARY KEY,
    flawed_problem_id uuid NOT NULL REFERENCES ple_data.published_question_version (problem_id),
    replacement_problem_id uuid NOT NULL REFERENCES ple_data.published_question_version (problem_id),
    approved_by uuid NOT NULL,
    approver_role text NOT NULL DEFAULT 'sysadmin' CHECK (approver_role = 'sysadmin'),
    approved_at timestamp with time zone NOT NULL,
    generation integer NOT NULL CHECK (generation > 0),
    reason text NOT NULL CHECK (reason IN ('security_flaw', 'critical_correctness_flaw')),
    remediation jsonb NOT NULL CHECK (jsonb_typeof(remediation) = 'object'),
    CONSTRAINT forced_question_correction_versions_differ CHECK (flawed_problem_id <> replacement_problem_id),
    CONSTRAINT forced_question_correction_approver_role_matches FOREIGN KEY (approved_by, approver_role)
        REFERENCES ple_private.account (user_id, role),
    CONSTRAINT forced_question_correction_flawed_generation_is_unique UNIQUE (flawed_problem_id, generation)
);
CREATE FUNCTION ple_data.reject_forced_question_correction_change()
RETURNS trigger LANGUAGE plpgsql SET search_path = pg_catalog, ple_data AS $$
BEGIN
    RAISE EXCEPTION USING ERRCODE = '23514', MESSAGE = 'a forced question correction is immutable';
END
$$;
CREATE TRIGGER forced_question_correction_is_immutable
BEFORE UPDATE OR DELETE ON ple_data.forced_question_correction
FOR EACH ROW EXECUTE FUNCTION ple_data.reject_forced_question_correction_change();
ALTER TABLE ple_data.forced_question_correction ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_data.forced_question_correction FORCE ROW LEVEL SECURITY;
REVOKE ALL PRIVILEGES ON TABLE ple_data.forced_question_correction FROM PUBLIC;
REVOKE ALL PRIVILEGES ON FUNCTION ple_data.reject_forced_question_correction_change() FROM PUBLIC;
GRANT USAGE ON SCHEMA ple_data TO ple_audit_owner;
GRANT REFERENCES ON TABLE ple_data.course_instance, ple_data.forced_question_correction TO ple_audit_owner;
RESET ROLE;

SET LOCAL ROLE ple_audit_owner;
CREATE TABLE ple_audit.correction_recalculation_evidence (
    evidence_id uuid PRIMARY KEY,
    correction_id uuid NOT NULL REFERENCES ple_data.forced_question_correction (correction_id),
    course_id uuid NOT NULL REFERENCES ple_data.course_instance (course_id),
    generation integer NOT NULL CHECK (generation > 0),
    recorded_at timestamp with time zone NOT NULL,
    outcome jsonb NOT NULL CHECK (jsonb_typeof(outcome) = 'object'),
    digest bytea NOT NULL CHECK (pg_catalog.octet_length(digest) = 32),
    CONSTRAINT correction_recalculation_evidence_is_unique UNIQUE (correction_id, course_id, generation)
);
ALTER TABLE ple_audit.correction_recalculation_evidence ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_audit.correction_recalculation_evidence FORCE ROW LEVEL SECURITY;
REVOKE ALL PRIVILEGES ON TABLE ple_audit.correction_recalculation_evidence FROM PUBLIC;
RESET ROLE;
