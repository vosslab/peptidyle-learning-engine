-- Row security policies from corrections.sql.

SET LOCAL ROLE ple_data_owner;

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

SET LOCAL ROLE ple_audit_owner;

ALTER TABLE ple_audit.forced_question_correction_assessment_attempt_target ENABLE ROW LEVEL SECURITY;

ALTER TABLE ple_audit.forced_question_correction_assessment_attempt_target FORCE ROW LEVEL SECURITY;

ALTER TABLE ple_audit.forced_question_correction_issued_question_target ENABLE ROW LEVEL SECURITY;

ALTER TABLE ple_audit.forced_question_correction_issued_question_target FORCE ROW LEVEL SECURITY;

CREATE POLICY correction_assessment_attempt_target_audit_owner_access
    ON ple_audit.forced_question_correction_assessment_attempt_target
    FOR ALL TO ple_audit_owner USING (true) WITH CHECK (true);

CREATE POLICY correction_issued_question_target_audit_owner_access
    ON ple_audit.forced_question_correction_issued_question_target
    FOR ALL TO ple_audit_owner USING (true) WITH CHECK (true);

