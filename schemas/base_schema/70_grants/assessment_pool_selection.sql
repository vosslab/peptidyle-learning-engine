-- Privileges from assessment_pool_selection.sql.

SET LOCAL ROLE ple_data_owner;

REVOKE ALL ON FUNCTION ple_data.update_assessment_question_pool_selection_count(text, uuid, bigint, integer) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_data.update_assessment_question_pool_selection_count(text, uuid, bigint, integer) TO ple_api_owner;

SET LOCAL ROLE ple_api_owner;

REVOKE ALL ON FUNCTION ple_api.update_assessment_question_pool_selection_count(
    text, text, uuid, bigint, integer
) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_api.update_assessment_question_pool_selection_count(
    text, text, uuid, bigint, integer
) TO ple_app;

