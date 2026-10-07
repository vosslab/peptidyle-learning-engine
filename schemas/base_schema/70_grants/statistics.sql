-- Privileges from statistics.sql.

SET LOCAL ROLE ple_data_owner;

REVOKE ALL ON TABLE ple_data.question_revision_statistics,
    ple_data.question_pool_statistics FROM PUBLIC;

GRANT SELECT ON TABLE ple_data.question_revision_statistics,
    ple_data.question_pool_statistics TO ple_api_owner;

SET LOCAL ROLE ple_data_owner;

GRANT REFERENCES ON TABLE ple_data.question_revision TO ple_private_owner;

GRANT REFERENCES ON TABLE ple_data.question_pool TO ple_private_owner;

GRANT REFERENCES ON TABLE ple_data.published_question TO ple_private_owner;

REVOKE ALL ON FUNCTION ple_data.record_question_revision_delivery(
    ple_data.question_family_id, integer, bigint, date
) FROM PUBLIC;
REVOKE ALL ON FUNCTION ple_data.record_question_pool_delivery(
    ple_data.question_family_id, bigint, date
) FROM PUBLIC;
REVOKE ALL ON FUNCTION ple_data.record_question_revision_outcome(
    ple_data.question_family_id, integer, numeric, bigint, bigint, bigint, bigint, bigint, date
) FROM PUBLIC;
REVOKE ALL ON FUNCTION ple_data.record_question_pool_outcome(
    ple_data.question_family_id, numeric, date
) FROM PUBLIC;
REVOKE ALL ON FUNCTION ple_data.record_question_pool_contributor_floors(
    ple_data.question_family_id, bigint, bigint, bigint, bigint
) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_data.record_question_revision_delivery(
    ple_data.question_family_id, integer, bigint, date
) TO ple_private_owner;
GRANT EXECUTE ON FUNCTION ple_data.record_question_pool_delivery(
    ple_data.question_family_id, bigint, date
) TO ple_private_owner;
GRANT EXECUTE ON FUNCTION ple_data.record_question_revision_outcome(
    ple_data.question_family_id, integer, numeric, bigint, bigint, bigint, bigint, bigint, date
) TO ple_private_owner;
GRANT EXECUTE ON FUNCTION ple_data.record_question_pool_outcome(
    ple_data.question_family_id, numeric, date
) TO ple_private_owner;
GRANT EXECUTE ON FUNCTION ple_data.record_question_pool_contributor_floors(
    ple_data.question_family_id, bigint, bigint, bigint, bigint
) TO ple_private_owner;

SET LOCAL ROLE ple_private_owner;

REVOKE ALL ON TABLE ple_private.question_statistics_observation_receipt FROM PUBLIC;

REVOKE ALL ON FUNCTION ple_private.capture_issued_question_statistics_observation(text, uuid, uuid, timestamptz)
    FROM PUBLIC;

SET LOCAL ROLE ple_data_owner;

REVOKE ALL ON FUNCTION ple_data.question_usage_statistics_rollups(text[]) FROM PUBLIC;
REVOKE ALL ON FUNCTION ple_data.question_revision_usage_statistics(text) FROM PUBLIC;
REVOKE ALL ON FUNCTION ple_data.question_pool_usage_statistics(text) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_data.question_usage_statistics_rollups(text[]) TO ple_api_owner;
GRANT EXECUTE ON FUNCTION ple_data.question_revision_usage_statistics(text) TO ple_api_owner;
GRANT EXECUTE ON FUNCTION ple_data.question_pool_usage_statistics(text) TO ple_api_owner;

SET LOCAL ROLE ple_api_owner;

REVOKE ALL ON FUNCTION ple_api.count_statistics_contributors(uuid[], text[])
    FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_api.count_statistics_contributors(uuid[], text[])
    TO ple_private_owner;

REVOKE ALL ON FUNCTION ple_api.read_question_library_usage_statistics(text[]) FROM PUBLIC;
REVOKE ALL ON FUNCTION ple_api.read_question_library_revision_usage_statistics(text)
    FROM PUBLIC;
REVOKE ALL ON FUNCTION ple_api.read_question_pool_library_usage_statistics(text) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_api.read_question_library_usage_statistics(text[]) TO ple_app;
GRANT EXECUTE ON FUNCTION ple_api.read_question_library_revision_usage_statistics(text)
    TO ple_app;
GRANT EXECUTE ON FUNCTION ple_api.read_question_pool_library_usage_statistics(text) TO ple_app;
