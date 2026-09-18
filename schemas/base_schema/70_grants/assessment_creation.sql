-- Privileges from assessment_creation.sql.

SET LOCAL ROLE ple_data_owner;

REVOKE ALL ON FUNCTION ple_data.create_assessment(text, text, text, text, text) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_data.create_assessment(text, text, text, text, text)
    TO ple_api_owner;

