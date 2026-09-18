-- Privileges from course_blueprint_publication.sql.

SET LOCAL ROLE ple_data_owner;

GRANT SELECT, INSERT ON TABLE ple_data.blueprint_course_instance_source TO ple_api_owner;

REVOKE ALL PRIVILEGES ON TABLE ple_data.blueprint_course_instance_source FROM PUBLIC;

REVOKE ALL ON FUNCTION ple_data.reject_blueprint_course_instance_source_change() FROM PUBLIC;

REVOKE ALL ON FUNCTION
    ple_data.lock_course_blueprint_publication_questions(uuid) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION
    ple_data.lock_course_blueprint_publication_questions(uuid) TO ple_api_owner;

REVOKE ALL ON FUNCTION
    ple_data.course_blueprint_publication_assessment_content(uuid),
    ple_data.course_blueprint_publication_snapshot(uuid) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION
    ple_data.course_blueprint_publication_assessment_content(uuid),
    ple_data.course_blueprint_publication_snapshot(uuid) TO ple_api_owner;

SET LOCAL ROLE ple_api_owner;

REVOKE ALL ON FUNCTION
    ple_api.load_course_blueprint_publication_source(text),
    ple_api.course_blueprint_publication_receipt(text, bytea),
    ple_api.create_blueprint_from_course_instance(
        text, uuid, jsonb, uuid, bytea, text, text, jsonb, bytea,
        uuid, uuid, uuid, uuid, text[]
    ) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION
    ple_api.load_course_blueprint_publication_source(text),
    ple_api.course_blueprint_publication_receipt(text, bytea),
    ple_api.create_blueprint_from_course_instance(
        text, uuid, jsonb, uuid, bytea, text, text, jsonb, bytea,
        uuid, uuid, uuid, uuid, text[]
    ) TO ple_app;

