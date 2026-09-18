-- Privileges from assessment_attempt_interaction.sql.

SET LOCAL ROLE ple_private_owner;

REVOKE ALL ON TABLE ple_private.question_attempt, ple_private.assessment_attempt_saved_response,
    ple_private.question_response, ple_private.assessment_submission FROM PUBLIC;

GRANT SELECT ON TABLE ple_private.question_attempt TO ple_api_owner;

REVOKE ALL ON FUNCTION ple_private.enforce_question_attempt_state_transition(),
    ple_private.validate_question_attempt_issue(), ple_private.enforce_question_response_state(),
    ple_private.reject_student_work_update(), ple_private.enforce_saved_response_change() FROM PUBLIC;

