-- Forced Question Correction evidence.  A correction is a narrow, immutable
-- Sysadmin record for a critical exact Question Revision replacement.  It does
-- not create a Question Change Proposal or an alternate revision lifecycle.

SET LOCAL ROLE ple_private_owner;
GRANT USAGE ON SCHEMA ple_private TO ple_data_owner;
GRANT REFERENCES ON TABLE ple_private.account TO ple_data_owner;
RESET ROLE;

SET LOCAL ROLE ple_data_owner;

CREATE TABLE ple_data.forced_question_correction (
    correction_id uuid PRIMARY KEY,
    flawed_question_id text NOT NULL,
    flawed_revision_number integer NOT NULL CHECK (flawed_revision_number > 0),
    replacement_question_id text NOT NULL,
    replacement_revision_number integer NOT NULL CHECK (replacement_revision_number > 0),
    approved_by_account_id uuid NOT NULL,
    approver_role text NOT NULL DEFAULT 'sysadmin' CHECK (approver_role = 'sysadmin'),
    approved_at timestamptz NOT NULL,
    correction_generation integer NOT NULL CHECK (correction_generation > 0),
    reason text NOT NULL CHECK (reason IN ('security_flaw', 'critical_correctness_flaw')),
    CHECK ((flawed_question_id, flawed_revision_number)
        <> (replacement_question_id, replacement_revision_number)),
    FOREIGN KEY (flawed_question_id, flawed_revision_number)
        REFERENCES ple_data.question_revision(question_id, revision_number),
    FOREIGN KEY (replacement_question_id, replacement_revision_number)
        REFERENCES ple_data.question_revision(question_id, revision_number),
    FOREIGN KEY (approved_by_account_id, approver_role)
        REFERENCES ple_private.account(account_id, product_role),
    UNIQUE (flawed_question_id, flawed_revision_number, correction_generation)
);

-- This retained event name is intentionally restricted to Forced Question
-- Correction evidence; the proposal event variants and their foreign keys are
-- absent from the baseline.
CREATE TABLE ple_data.question_change_event (
    question_change_event_id uuid PRIMARY KEY,
    forced_question_correction_id uuid NOT NULL UNIQUE
        REFERENCES ple_data.forced_question_correction(correction_id),
    recorded_by_account_id uuid NOT NULL REFERENCES ple_private.account(account_id),
    occurred_at timestamptz NOT NULL,
    evidence jsonb NOT NULL CHECK (jsonb_typeof(evidence) = 'object')
);

CREATE FUNCTION ple_data.reject_forced_question_correction_change()
RETURNS trigger LANGUAGE plpgsql
SET search_path = pg_catalog, ple_data AS $$
BEGIN
    RAISE EXCEPTION USING ERRCODE = '55000',
        MESSAGE = 'Forced Question Correction evidence is immutable';
END
$$;

CREATE TRIGGER forced_question_correction_is_immutable
BEFORE UPDATE OR DELETE ON ple_data.forced_question_correction
FOR EACH ROW EXECUTE FUNCTION ple_data.reject_forced_question_correction_change();
CREATE TRIGGER forced_question_correction_event_is_immutable
BEFORE UPDATE OR DELETE ON ple_data.question_change_event
FOR EACH ROW EXECUTE FUNCTION ple_data.reject_forced_question_correction_change();

ALTER TABLE ple_data.forced_question_correction ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_data.forced_question_correction FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_data.question_change_event ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_data.question_change_event FORCE ROW LEVEL SECURITY;
CREATE POLICY forced_question_correction_data_owner_access
    ON ple_data.forced_question_correction
    FOR ALL TO ple_data_owner USING (true) WITH CHECK (true);
CREATE POLICY forced_question_correction_event_data_owner_access
    ON ple_data.question_change_event
    FOR ALL TO ple_data_owner USING (true) WITH CHECK (true);
REVOKE ALL ON TABLE ple_data.forced_question_correction, ple_data.question_change_event FROM PUBLIC;
REVOKE ALL ON FUNCTION ple_data.reject_forced_question_correction_change() FROM PUBLIC;
GRANT USAGE ON SCHEMA ple_data TO ple_audit_owner;
GRANT REFERENCES ON TABLE ple_data.forced_question_correction TO ple_audit_owner;
RESET ROLE;

SET LOCAL ROLE ple_private_owner;
GRANT USAGE ON SCHEMA ple_private TO ple_audit_owner;
GRANT REFERENCES ON TABLE ple_private.assignment_attempt,
    ple_private.issued_question TO ple_audit_owner;
RESET ROLE;

SET LOCAL ROLE ple_audit_owner;

CREATE TABLE ple_audit.forced_question_correction_attempt_target (
    correction_id uuid NOT NULL REFERENCES ple_data.forced_question_correction(correction_id),
    assignment_attempt_id uuid NOT NULL
        REFERENCES ple_private.assignment_attempt(assignment_attempt_id) ON DELETE CASCADE,
    PRIMARY KEY (correction_id, assignment_attempt_id)
);
CREATE TABLE ple_audit.forced_question_correction_issued_question_target (
    correction_id uuid NOT NULL REFERENCES ple_data.forced_question_correction(correction_id),
    issued_question_id uuid NOT NULL
        REFERENCES ple_private.issued_question(issued_question_id) ON DELETE CASCADE,
    PRIMARY KEY (correction_id, issued_question_id)
);
CREATE FUNCTION ple_audit.reject_forced_question_correction_target_change()
RETURNS trigger LANGUAGE plpgsql
SET search_path = pg_catalog, ple_audit AS $$
BEGIN
    IF TG_OP = 'DELETE'
       AND TG_TABLE_NAME IN (
           'forced_question_correction_attempt_target',
           'forced_question_correction_issued_question_target'
       ) THEN
        -- FK actions run under the referenced-table owner.  The ordinary
        -- direct-delete path is still restricted to the executor; a rooted
        -- Student Work cascade reaches this trigger at nested depth.
        IF current_user = 'ple_unrelease_executor'
           OR pg_catalog.pg_trigger_depth() > 1 THEN
            RETURN OLD;
        END IF;
    END IF;
    RAISE EXCEPTION USING ERRCODE = '55000',
        MESSAGE = 'Forced Question Correction target evidence is immutable';
END
$$;

CREATE TRIGGER forced_question_correction_attempt_target_is_immutable
BEFORE UPDATE OR DELETE ON ple_audit.forced_question_correction_attempt_target
FOR EACH ROW EXECUTE FUNCTION ple_audit.reject_forced_question_correction_target_change();
CREATE TRIGGER forced_question_correction_issued_question_target_is_immutable
BEFORE UPDATE OR DELETE ON ple_audit.forced_question_correction_issued_question_target
FOR EACH ROW EXECUTE FUNCTION ple_audit.reject_forced_question_correction_target_change();
ALTER TABLE ple_audit.forced_question_correction_attempt_target ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_audit.forced_question_correction_attempt_target FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_audit.forced_question_correction_issued_question_target ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_audit.forced_question_correction_issued_question_target FORCE ROW LEVEL SECURITY;
CREATE POLICY correction_attempt_target_audit_owner_access
    ON ple_audit.forced_question_correction_attempt_target
    FOR ALL TO ple_audit_owner USING (true) WITH CHECK (true);
CREATE POLICY correction_issued_question_target_audit_owner_access
    ON ple_audit.forced_question_correction_issued_question_target
    FOR ALL TO ple_audit_owner USING (true) WITH CHECK (true);
REVOKE ALL ON TABLE ple_audit.forced_question_correction_attempt_target,
    ple_audit.forced_question_correction_issued_question_target FROM PUBLIC;
REVOKE ALL ON FUNCTION ple_audit.reject_forced_question_correction_target_change() FROM PUBLIC;
RESET ROLE;

SET LOCAL ROLE ple_data_owner;
COMMENT ON TABLE ple_data.forced_question_correction IS
    'Immutable Sysadmin-approved critical correction from one exact Question Revision to another.';
COMMENT ON TABLE ple_data.question_change_event IS
    'Immutable Forced Question Correction event; Question Change Proposal events are not persisted.';
RESET ROLE;
