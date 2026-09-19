-- Privileges from statistics.sql.

SET LOCAL ROLE ple_data_owner;

REVOKE ALL ON TABLE ple_data.question_revision_statistics,
    ple_data.question_pool_statistics,
    ple_data.question_pool_member_statistics FROM PUBLIC;

GRANT SELECT ON TABLE ple_data.question_revision_statistics,
    ple_data.question_pool_statistics,
    ple_data.question_pool_member_statistics TO ple_api_owner;

REVOKE ALL ON FUNCTION ple_data.increment_question_revision_statistics(text, integer, numeric, date)
    FROM PUBLIC;

REVOKE ALL ON FUNCTION ple_data.increment_question_pool_issue_statistics(text, text, date)
    FROM PUBLIC;

REVOKE ALL ON FUNCTION ple_data.drop_question_pool_member_statistics_when_unselected()
    FROM PUBLIC;

SET LOCAL ROLE ple_data_owner;

GRANT REFERENCES ON TABLE ple_data.question_revision TO ple_private_owner;

GRANT REFERENCES ON TABLE ple_data.question_pool TO ple_private_owner;

GRANT REFERENCES ON TABLE ple_data.published_question TO ple_private_owner;

SET LOCAL ROLE ple_private_owner;

REVOKE ALL ON TABLE ple_private.question_statistics_observation_receipt FROM PUBLIC;

REVOKE ALL ON FUNCTION ple_private.capture_issued_question_statistics_observation(text, uuid, uuid, timestamptz)
    FROM PUBLIC;

SET LOCAL ROLE ple_data_owner;

GRANT EXECUTE ON FUNCTION ple_data.increment_question_revision_statistics(text, integer, numeric, date)
    TO ple_private_owner;

GRANT EXECUTE ON FUNCTION ple_data.increment_question_pool_issue_statistics(text, text, date)
    TO ple_private_owner;

REVOKE ALL ON FUNCTION ple_data.question_usage_statistics_rollups(text[]) FROM PUBLIC;
REVOKE ALL ON FUNCTION ple_data.question_revision_usage_statistics(text) FROM PUBLIC;
REVOKE ALL ON FUNCTION ple_data.question_pool_usage_statistics(text) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_data.question_usage_statistics_rollups(text[]) TO ple_api_owner;
GRANT EXECUTE ON FUNCTION ple_data.question_revision_usage_statistics(text) TO ple_api_owner;
GRANT EXECUTE ON FUNCTION ple_data.question_pool_usage_statistics(text) TO ple_api_owner;

SET LOCAL ROLE ple_api_owner;

REVOKE ALL ON FUNCTION ple_api.read_question_library_usage_statistics(text[]) FROM PUBLIC;
REVOKE ALL ON FUNCTION ple_api.read_question_library_revision_usage_statistics(text)
    FROM PUBLIC;
REVOKE ALL ON FUNCTION ple_api.read_question_pool_library_usage_statistics(text) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_api.read_question_library_usage_statistics(text[]) TO ple_app;
GRANT EXECUTE ON FUNCTION ple_api.read_question_library_revision_usage_statistics(text)
    TO ple_app;
GRANT EXECUTE ON FUNCTION ple_api.read_question_pool_library_usage_statistics(text) TO ple_app;
