-- Privileges from assessment_blueprint_updates.sql.

SET LOCAL ROLE ple_data_owner;

REVOKE ALL ON FUNCTION ple_data.lock_assessment_blueprint_update_destination(text, text, text) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_data.lock_assessment_blueprint_update_destination(text, text, text) TO ple_api_owner;

SET LOCAL ROLE ple_api_owner;

REVOKE ALL ON FUNCTION ple_api.load_assessment_blueprint_update(text, text) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_api.load_assessment_blueprint_update(text, text) TO ple_app, ple_data_owner;

SET LOCAL ROLE ple_data_owner;

REVOKE ALL ON FUNCTION ple_data.assessment_blueprint_update_entries_semantics(jsonb),
    ple_data.assessment_blueprint_update_equivalent(text, jsonb, jsonb) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_data.assessment_blueprint_update_equivalent(text, jsonb, jsonb) TO ple_api_owner;

REVOKE ALL ON FUNCTION ple_data.apply_assessment_blueprint_update(text, text, bigint, bigint, jsonb) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_data.apply_assessment_blueprint_update(text, text, bigint, bigint, jsonb) TO ple_api_owner;

SET LOCAL ROLE ple_api_owner;

REVOKE ALL ON FUNCTION ple_api.apply_assessment_blueprint_update(text, text, bigint, bigint, jsonb) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_api.apply_assessment_blueprint_update(text, text, bigint, bigint, jsonb) TO ple_app;

REVOKE ALL ON FUNCTION ple_api.assessment_blueprint_update_equivalent(text, text, jsonb, jsonb) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_api.assessment_blueprint_update_equivalent(text, text, jsonb, jsonb) TO ple_app;

SET LOCAL ROLE ple_data_owner;

REVOKE ALL ON FUNCTION ple_data.lock_course_blueprint_update_destination(text, text) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_data.lock_course_blueprint_update_destination(text, text) TO ple_api_owner;

SET LOCAL ROLE ple_api_owner;

REVOKE ALL ON FUNCTION ple_api.load_course_blueprint_update(text, jsonb) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_api.load_course_blueprint_update(text, jsonb) TO ple_app;

