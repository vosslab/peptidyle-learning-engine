-- Privileges from blueprint_operations.sql.

SET LOCAL ROLE ple_api_owner;

REVOKE ALL ON FUNCTION ple_private.create_blueprint_course(text, bytea, text, text, jsonb, bytea, uuid, uuid, uuid, uuid, text[], uuid) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_private.create_blueprint_course(text, bytea, text, text, jsonb, bytea, uuid, uuid, uuid, uuid, text[], uuid) TO ple_api_owner;

SET LOCAL ROLE ple_private_owner;

REVOKE CREATE ON SCHEMA ple_private FROM ple_api_owner;

SET LOCAL ROLE ple_api_owner;

REVOKE ALL ON FUNCTION ple_api.update_blueprint_classification(text, uuid, uuid, uuid, uuid, uuid, text[]) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_api.update_blueprint_classification(text, uuid, uuid, uuid, uuid, uuid, text[]) TO ple_app;

REVOKE ALL PRIVILEGES ON FUNCTION
    ple_api.create_blueprint_course(text, bytea, text, text, jsonb, bytea, uuid, uuid, uuid, uuid, text[]),
    ple_api.save_blueprint_course(text, bigint, bytea, jsonb, bytea),
    ple_api.rename_blueprint_course(text, uuid, text, text),
    ple_api.set_blueprint_availability(text, uuid, text, text),
    ple_api.list_blueprint_courses(boolean, boolean, boolean, text, text, text, integer, uuid, uuid, uuid, uuid, boolean), ple_api.load_blueprint_course(text),
    ple_api.load_blueprint_revision(text, bigint) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION
    ple_api.create_blueprint_course(text, bytea, text, text, jsonb, bytea, uuid, uuid, uuid, uuid, text[]),
    ple_api.rename_blueprint_course(text, uuid, text, text),
    ple_api.set_blueprint_availability(text, uuid, text, text),
    ple_api.list_blueprint_courses(boolean, boolean, boolean, text, text, text, integer, uuid, uuid, uuid, uuid, boolean), ple_api.load_blueprint_course(text),
    ple_api.load_blueprint_revision(text, bigint) TO ple_app;

REVOKE ALL ON FUNCTION ple_api.load_blueprint_promotion(text),
    ple_api.set_blueprint_promotion(text, uuid, boolean) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_api.load_blueprint_promotion(text),
    ple_api.set_blueprint_promotion(text, uuid, boolean) TO ple_app;

