-- Privileges from assessment_attempt_interaction.sql.

SET LOCAL ROLE ple_private_owner;

REVOKE ALL ON TABLE ple_private.delivery_toolchain, ple_private.question_attempt,
    ple_private.assessment_attempt_saved_response, ple_private.assessment_submission FROM PUBLIC;

GRANT SELECT ON TABLE ple_private.question_attempt TO ple_api_owner;

REVOKE ALL ON FUNCTION ple_private.projected_question_attempt_state(timestamptz, boolean),
    ple_private.projected_finalization_kind(text),
    ple_private.ensure_delivery_toolchain(
        ple_data.question_backend, text, text, text, text, text, ple_data.issued_capability
    ),
    ple_private.enforce_question_attempt_finalization(),
    ple_private.validate_question_attempt_issue(),
    ple_private.reject_student_work_update(), ple_private.enforce_saved_response_change() FROM PUBLIC;
