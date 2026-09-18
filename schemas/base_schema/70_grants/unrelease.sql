-- Privileges from unrelease.sql.

SET LOCAL ROLE ple_data_owner;

-- Assessment Unrelease is the sole destructive Assessment lifecycle operation.
-- It is a single Assessment-first transaction: the executor locks current
-- teaching authority and the Assessment before it observes or deletes Student
-- Work, so concurrent starts, submissions, and grading commits fail closed.
GRANT REFERENCES ON TABLE ple_data.assessment TO ple_audit_owner;

SET LOCAL ROLE ple_audit_owner;

REVOKE ALL ON TABLE ple_audit.assessment_unrelease_event FROM PUBLIC;

GRANT INSERT ON TABLE ple_audit.assessment_unrelease_event TO ple_unrelease_executor;

REVOKE ALL ON FUNCTION ple_audit.reject_assessment_unrelease_event_change() FROM PUBLIC;

SET LOCAL ROLE ple_data_owner;

GRANT SELECT ON TABLE ple_data.course_instance TO ple_unrelease_executor;

GRANT SELECT, UPDATE ON TABLE ple_data.assessment TO ple_unrelease_executor;

SET LOCAL ROLE ple_private_owner;

GRANT SELECT, DELETE ON TABLE ple_private.assessment_attempt TO ple_unrelease_executor;

GRANT SELECT ON TABLE ple_private.issued_question, ple_private.question_attempt,
    ple_private.question_response, ple_private.assessment_submission,
    ple_private.grading_result
    TO ple_unrelease_executor;

SET LOCAL ROLE ple_api_owner;

GRANT EXECUTE ON FUNCTION ple_api.current_session_account_id(),
    ple_api.current_session_account_is_course_instructor(text)
    TO ple_unrelease_executor;

SET LOCAL ROLE ple_unrelease_executor;

REVOKE ALL ON FUNCTION ple_api.read_assessment_unrelease_impact(text, text),
    ple_api.unrelease_assessment(text, text, bigint, text)
    FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_api.read_assessment_unrelease_impact(text, text),
    ple_api.unrelease_assessment(text, text, bigint, text)
    TO ple_app;

SET LOCAL ROLE ple_api_owner;

REVOKE CREATE ON SCHEMA ple_api FROM ple_unrelease_executor;

