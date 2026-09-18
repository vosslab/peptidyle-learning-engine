-- Privileges from question_library_operations.sql.

SET LOCAL ROLE ple_private_owner;

REVOKE ALL ON FUNCTION ple_private.question_library_entries(text, integer, boolean) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_private.question_library_entries(text, integer, boolean) TO ple_api_owner;

REVOKE ALL ON FUNCTION ple_private.load_current_published_question_shared_metadata(text[])
    FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_private.load_current_published_question_shared_metadata(text[])
    TO ple_api_owner;

SET LOCAL ROLE ple_api_owner;

REVOKE ALL ON TABLE ple_api.published_question_summary FROM PUBLIC;

REVOKE ALL ON FUNCTION ple_api.list_question_library_entries(),
    ple_api.load_question_library_revision(text, integer),
    ple_api.load_current_published_question_shared_metadata(text[]) FROM PUBLIC;

GRANT SELECT ON TABLE ple_api.published_question_summary TO ple_app;

GRANT EXECUTE ON FUNCTION ple_api.list_question_library_entries(),
    ple_api.load_question_library_revision(text, integer),
    ple_api.load_current_published_question_shared_metadata(text[]) TO ple_app;

