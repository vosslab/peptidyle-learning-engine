-- Privileges from blueprint_lineage.sql.

SET LOCAL ROLE ple_api_owner;

REVOKE ALL ON FUNCTION ple_api.list_known_blueprint_forks(text) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_api.list_known_blueprint_forks(text) TO ple_app;

REVOKE ALL ON FUNCTION ple_api.load_blueprint_comparison_sources(text, text) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_api.load_blueprint_comparison_sources(text, text) TO ple_app;

REVOKE ALL ON FUNCTION ple_api.load_blueprint_fork_apply_sources(text, bigint, bigint, text, bigint, bigint) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_api.load_blueprint_fork_apply_sources(text, bigint, bigint, text, bigint, bigint) TO ple_app;

REVOKE ALL ON FUNCTION ple_api.load_blueprint_fork_source(text, bigint, bytea) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_api.load_blueprint_fork_source(text, bigint, bytea) TO ple_app;

REVOKE ALL PRIVILEGES ON TABLE
    ple_data.blueprint_course_fork, ple_data.blueprint_course_fork_receipt FROM PUBLIC;

REVOKE ALL ON FUNCTION ple_api.fork_blueprint_course(text, text, bigint, bytea, jsonb, bytea) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_api.fork_blueprint_course(text, text, bigint, bytea, jsonb, bytea)
TO ple_app;

