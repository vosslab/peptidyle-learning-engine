-- Privileges from course_blueprint_adoption.sql.

SET LOCAL ROLE ple_api_owner;

REVOKE ALL ON FUNCTION ple_api.load_course_instance_blueprint(bigint, bigint) FROM PUBLIC;


-- The data-owner adoption validator has no broad read policy on forced-RLS
-- Blueprint relations. It may invoke this already lifecycle-gated exact
-- Revision reader while materializing its child transaction.
GRANT EXECUTE ON FUNCTION ple_api.load_course_instance_blueprint(bigint, bigint) TO ple_data_owner;

REVOKE ALL ON FUNCTION ple_api.load_course_instance_blueprint(text, bigint) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_api.load_course_instance_blueprint(text, bigint) TO ple_app;

REVOKE ALL ON FUNCTION ple_api.load_blueprint_assessment_copy_source(bigint, bigint) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_api.load_blueprint_assessment_copy_source(bigint, bigint) TO ple_data_owner;

SET LOCAL ROLE ple_data_owner;

REVOKE ALL ON FUNCTION ple_data.append_course_assessments(uuid, bigint, bigint, jsonb) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_data.append_course_assessments(uuid, bigint, bigint, jsonb) TO ple_api_owner;

REVOKE ALL ON FUNCTION ple_data.initialize_course_assessments(uuid, bigint, bigint, jsonb) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_data.initialize_course_assessments(uuid, bigint, bigint, jsonb) TO ple_api_owner;

SET LOCAL ROLE ple_api_owner;

REVOKE ALL ON FUNCTION ple_api.list_blueprint_daughter_course_ids(text),
    ple_api.append_new_blueprint_assessments(text, bigint, bigint, uuid, jsonb) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_api.list_blueprint_daughter_course_ids(text) TO ple_app;



-- Only this complete Save operation is callable by the application role.
REVOKE ALL ON FUNCTION ple_api.save_blueprint_course(text, bigint, bytea, jsonb, bytea) FROM PUBLIC, ple_app;

REVOKE ALL ON FUNCTION ple_api.save_blueprint_course(text, bigint, bytea, jsonb, bytea, jsonb) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_api.save_blueprint_course(text, bigint, bytea, jsonb, bytea, jsonb) TO ple_app;

