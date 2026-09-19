-- Privileges from assessment_pool_forks.sql.

SET LOCAL ROLE ple_data_owner;

REVOKE ALL ON FUNCTION
    ple_data.import_assessment_question_pool_fork(text, uuid, bigint, text, text, integer, integer, numeric, text, text),
    ple_data.append_assessment_question_pool_fork_members(text, uuid, bigint, bigint, text[], integer[], boolean)
    FROM PUBLIC;

GRANT EXECUTE ON FUNCTION
    ple_data.import_assessment_question_pool_fork(text, uuid, bigint, text, text, integer, integer, numeric, text, text),
    ple_data.append_assessment_question_pool_fork_members(text, uuid, bigint, bigint, text[], integer[], boolean)
    TO ple_api_owner;

SET LOCAL ROLE ple_api_owner;

REVOKE ALL ON FUNCTION ple_api.import_assessment_question_pool_fork(text, uuid, bigint, text, text, integer, integer, numeric, text, text) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_api.import_assessment_question_pool_fork(text, uuid, bigint, text, text, integer, integer, numeric, text, text) TO ple_app;

REVOKE ALL ON FUNCTION ple_api.import_assessment_question_pool_fork_for_ids(text, text, uuid, bigint, text, text, integer, integer, numeric, text, text) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_api.import_assessment_question_pool_fork_for_ids(text, text, uuid, bigint, text, text, integer, integer, numeric, text, text) TO ple_app;

REVOKE ALL ON FUNCTION ple_api.read_assessment_question_pool_fork(text, text, uuid) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_api.read_assessment_question_pool_fork(text, text, uuid) TO ple_app;

REVOKE ALL ON FUNCTION ple_api.append_assessment_question_pool_fork_members(text, uuid, bigint, bigint, text[], integer[], boolean) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_api.append_assessment_question_pool_fork_members(text, uuid, bigint, bigint, text[], integer[], boolean) TO ple_app;

REVOKE ALL ON FUNCTION ple_api.append_assessment_question_pool_fork_members_for_course(text, text, uuid, bigint, bigint, text[], integer[], boolean) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_api.append_assessment_question_pool_fork_members_for_course(text, text, uuid, bigint, bigint, text[], integer[], boolean) TO ple_app;
