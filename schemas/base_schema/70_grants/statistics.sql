-- Privileges from statistics.sql.

SET LOCAL ROLE ple_data_owner;

REVOKE ALL ON TABLE ple_data.question_revision_statistics,
    ple_data.question_revision_choice_statistics FROM PUBLIC;

GRANT SELECT ON TABLE ple_data.question_revision_statistics,
    ple_data.question_revision_choice_statistics TO ple_api_owner;



-- ASVS 8.2.1: only the trusted private capture routine may add observations.
REVOKE ALL ON FUNCTION ple_data.increment_question_revision_statistics(text, integer, boolean, text[], timestamptz)
    FROM PUBLIC;

SET LOCAL ROLE ple_audit_owner;

GRANT REFERENCES ON TABLE ple_audit.automated_grading_receipt TO ple_private_owner;

SET LOCAL ROLE ple_data_owner;

GRANT REFERENCES ON TABLE ple_data.question_revision TO ple_private_owner;

SET LOCAL ROLE ple_private_owner;

REVOKE ALL ON TABLE ple_private.question_statistics_observation_receipt,
    ple_private.question_statistics_observation_choice FROM PUBLIC;

REVOKE ALL ON FUNCTION ple_private.capture_question_statistics_observation(uuid, text[])
    FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_private.capture_question_statistics_observation(uuid, text[])
    TO ple_api_owner;

SET LOCAL ROLE ple_audit_owner;

GRANT SELECT ON TABLE ple_audit.automated_grading_receipt TO ple_private_owner;

SET LOCAL ROLE ple_data_owner;

GRANT EXECUTE ON FUNCTION ple_data.increment_question_revision_statistics(text, integer, boolean, text[], timestamptz)
    TO ple_private_owner;

SET LOCAL ROLE ple_api_owner;

REVOKE ALL ON FUNCTION ple_api.record_question_statistics_observation(uuid, text[]) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_api.record_question_statistics_observation(uuid, text[]) TO ple_api_owner;

