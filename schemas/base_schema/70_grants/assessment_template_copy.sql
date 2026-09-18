-- Privileges from assessment_template_copy.sql.

SET LOCAL ROLE ple_data_owner;

REVOKE ALL ON FUNCTION ple_data.create_assessment_from_template_values(text, text, text, text, text, integer, integer, text, text, text, text, text, text, text, text, text) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_data.create_assessment_from_template_values(text, text, text, text, text, integer, integer, text, text, text, text, text, text, text, text, text) TO ple_api_owner;

SET LOCAL ROLE ple_api_owner;

REVOKE ALL ON FUNCTION ple_api.create_assessment_from_template(text, text, uuid, text)
    FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_api.create_assessment_from_template(text, text, uuid, text)
    TO ple_app;

