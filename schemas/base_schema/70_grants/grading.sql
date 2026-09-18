-- Privileges from grading.sql.

SET LOCAL ROLE ple_private_owner;

GRANT REFERENCES ON TABLE ple_private.grading_result TO ple_audit_owner;

GRANT EXECUTE ON FUNCTION ple_private.reject_student_work_delete() TO ple_audit_owner;

SET LOCAL ROLE ple_audit_owner;

REVOKE ALL ON TABLE ple_audit.automated_grading_receipt FROM PUBLIC;

GRANT SELECT ON ple_audit.automated_grading_receipt TO ple_api_owner;

GRANT SELECT, INSERT ON ple_audit.automated_grading_receipt TO ple_private_owner;

SET LOCAL ROLE ple_private_owner;

REVOKE ALL ON TABLE ple_private.question_response_grading, ple_private.grading_result FROM PUBLIC;

