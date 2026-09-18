-- Privileges from corrections.sql.

SET LOCAL ROLE ple_private_owner;

GRANT REFERENCES ON TABLE ple_private.account TO ple_data_owner;

SET LOCAL ROLE ple_data_owner;

REVOKE ALL ON TABLE ple_data.forced_question_correction, ple_data.question_change_event FROM PUBLIC;

REVOKE ALL ON FUNCTION ple_data.reject_forced_question_correction_change() FROM PUBLIC;

GRANT REFERENCES ON TABLE ple_data.forced_question_correction TO ple_audit_owner;

SET LOCAL ROLE ple_private_owner;

GRANT REFERENCES ON TABLE ple_private.assessment_attempt,
    ple_private.issued_question TO ple_audit_owner;

SET LOCAL ROLE ple_audit_owner;

REVOKE ALL ON TABLE ple_audit.forced_question_correction_assessment_attempt_target,
    ple_audit.forced_question_correction_issued_question_target FROM PUBLIC;

REVOKE ALL ON FUNCTION ple_audit.reject_forced_question_correction_target_change() FROM PUBLIC;

