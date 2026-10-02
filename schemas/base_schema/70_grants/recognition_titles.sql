-- Privileges from recognition_titles.sql.

SET LOCAL ROLE ple_api_owner;

REVOKE ALL ON FUNCTION ple_api.load_recognition_titles(text[], text[]) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_api.load_recognition_titles(text[], text[]) TO ple_app;
