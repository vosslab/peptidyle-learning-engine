-- Privileges from question_pool_stewardship.sql.

SET LOCAL ROLE ple_data_owner;

GRANT SELECT ON ple_data.question_pool TO ple_data_owner;

REVOKE ALL ON TABLE ple_data.question_pool_star FROM PUBLIC;

REVOKE ALL ON TABLE ple_data.question_pool_watch FROM PUBLIC;

REVOKE ALL ON FUNCTION ple_data.set_current_question_pool_star(text, boolean),
    ple_data.read_current_question_pool_star(text),
    ple_data.set_current_question_pool_watch(text, boolean),
    ple_data.read_current_question_pool_watch(text) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_data.set_current_question_pool_star(text, boolean),
    ple_data.read_current_question_pool_star(text),
    ple_data.set_current_question_pool_watch(text, boolean),
    ple_data.read_current_question_pool_watch(text) TO ple_api_owner;

SET LOCAL ROLE ple_api_owner;

REVOKE ALL ON FUNCTION ple_api.set_current_question_pool_star(text, boolean),
    ple_api.read_current_question_pool_star(text),
    ple_api.set_current_question_pool_watch(text, boolean),
    ple_api.read_current_question_pool_watch(text) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_api.set_current_question_pool_star(text, boolean),
    ple_api.read_current_question_pool_star(text),
    ple_api.set_current_question_pool_watch(text, boolean),
    ple_api.read_current_question_pool_watch(text) TO ple_app;

