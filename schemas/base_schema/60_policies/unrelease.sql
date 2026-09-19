-- Row security policies from unrelease.sql.

SET LOCAL ROLE ple_audit_owner;

ALTER TABLE ple_audit.assessment_unrelease_event ENABLE ROW LEVEL SECURITY;

ALTER TABLE ple_audit.assessment_unrelease_event FORCE ROW LEVEL SECURITY;

CREATE POLICY assessment_unrelease_event_audit_owner_access
    ON ple_audit.assessment_unrelease_event
    FOR ALL TO ple_audit_owner USING (true) WITH CHECK (true);

CREATE POLICY assessment_unrelease_event_executor_insert
    ON ple_audit.assessment_unrelease_event
    FOR INSERT TO ple_unrelease_executor WITH CHECK (true);

SET LOCAL ROLE ple_data_owner;

CREATE POLICY course_instance_unrelease_executor_read
    ON ple_data.course_instance
    FOR SELECT TO ple_unrelease_executor USING (true);

CREATE POLICY assessment_unrelease_executor_access
    ON ple_data.assessment
    FOR ALL TO ple_unrelease_executor USING (true) WITH CHECK (true);

SET LOCAL ROLE ple_private_owner;

CREATE POLICY assessment_attempt_unrelease_executor_access
    ON ple_private.assessment_attempt
    FOR ALL TO ple_unrelease_executor USING (true) WITH CHECK (true);

CREATE POLICY issued_question_unrelease_executor_read
    ON ple_private.issued_question
    FOR SELECT TO ple_unrelease_executor USING (true);

CREATE POLICY question_attempt_unrelease_executor_read
    ON ple_private.question_attempt
    FOR SELECT TO ple_unrelease_executor USING (true);

CREATE POLICY saved_response_unrelease_executor_read
    ON ple_private.assessment_attempt_saved_response
    FOR SELECT TO ple_unrelease_executor USING (true);

CREATE POLICY assessment_submission_unrelease_executor_read
    ON ple_private.assessment_submission
    FOR SELECT TO ple_unrelease_executor USING (true);

CREATE POLICY grading_result_unrelease_executor_read
    ON ple_private.grading_result
    FOR SELECT TO ple_unrelease_executor USING (true);

