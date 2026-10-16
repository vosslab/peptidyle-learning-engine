-- Privileges from question_library_operations.sql.

SET LOCAL ROLE ple_private_owner;

REVOKE ALL ON FUNCTION ple_private.question_library_entries(text, integer, boolean) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_private.question_library_entries(text, integer, boolean) TO ple_api_owner;

REVOKE ALL ON FUNCTION ple_private.search_question_library_entries(
    text, jsonb, text[], text[], text[], text[], text[], uuid[], uuid[], uuid[], uuid[], boolean,
    text[], text[], text[], text[], boolean, text, text, text, text, text, bigint, text, integer, text, bigint
) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_private.search_question_library_entries(
    text, jsonb, text[], text[], text[], text[], text[], uuid[], uuid[], uuid[], uuid[], boolean,
    text[], text[], text[], text[], boolean, text, text, text, text, text, bigint, text, integer, text, bigint
) TO ple_api_owner;

REVOKE ALL ON FUNCTION ple_private.matching_question_library_entries(
    text, jsonb, text[], text[], text[], text[], text[], uuid[], uuid[], uuid[], uuid[], boolean,
    text[], text[], text[], text[], boolean, text, text, text
) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_private.matching_question_library_entries(
    text, jsonb, text[], text[], text[], text[], text[], uuid[], uuid[], uuid[], uuid[], boolean,
    text[], text[], text[], text[], boolean, text, text, text
) TO ple_api_owner;

REVOKE ALL ON FUNCTION ple_private.question_library_facet_choices(
    text, jsonb, text[], text[], text[], text[], text[], uuid[], uuid[], uuid[], uuid[], boolean,
    text[], text[], text[], text[], boolean, text, text, text, text, text, text, integer, text[], text
) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_private.question_library_facet_choices(
    text, jsonb, text[], text[], text[], text[], text[], uuid[], uuid[], uuid[], uuid[], boolean,
    text[], text[], text[], text[], boolean, text, text, text, text, text, text, integer, text[], text
) TO ple_api_owner;

REVOKE ALL ON FUNCTION ple_private.load_library_object_kind(text) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_private.load_library_object_kind(text) TO ple_api_owner;

REVOKE ALL ON FUNCTION ple_private.load_current_published_question_shared_metadata(text[])
    FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_private.load_current_published_question_shared_metadata(text[])
    TO ple_api_owner;

SET LOCAL ROLE ple_api_owner;

REVOKE ALL ON TABLE ple_api.published_question_summary FROM PUBLIC;

REVOKE ALL ON FUNCTION ple_api.list_question_library_entries(),
    ple_api.load_question_library_lineage(text),
    ple_api.search_question_library_entries(
        text, jsonb, text[], text[], text[], text[], text[], uuid[], uuid[], uuid[], uuid[], boolean,
        text[], text[], text[], text[], boolean, text, text, text, text, text, bigint, text, integer, text, bigint
    ),
    ple_api.question_library_facet_choices(
        text, jsonb, text[], text[], text[], text[], text[], uuid[], uuid[], uuid[], uuid[], boolean,
        text[], text[], text[], text[], boolean, text, text, text, text, text, text, integer, text[], text
    ),
    ple_api.load_library_object_kind(text),
    ple_api.load_question_library_revision(text, integer),
    ple_api.load_current_published_question_shared_metadata(text[]) FROM PUBLIC;

GRANT SELECT ON TABLE ple_api.published_question_summary TO ple_app;

GRANT EXECUTE ON FUNCTION ple_api.list_question_library_entries(),
    ple_api.load_question_library_lineage(text),
    ple_api.search_question_library_entries(
        text, jsonb, text[], text[], text[], text[], text[], uuid[], uuid[], uuid[], uuid[], boolean,
        text[], text[], text[], text[], boolean, text, text, text, text, text, bigint, text, integer, text, bigint
    ),
    ple_api.question_library_facet_choices(
        text, jsonb, text[], text[], text[], text[], text[], uuid[], uuid[], uuid[], uuid[], boolean,
        text[], text[], text[], text[], boolean, text, text, text, text, text, text, integer, text[], text
    ),
    ple_api.load_library_object_kind(text),
    ple_api.load_question_library_revision(text, integer),
    ple_api.load_current_published_question_shared_metadata(text[]) TO ple_app;

SET LOCAL ROLE ple_data_owner;
REVOKE ALL ON FUNCTION ple_data.question_use_global_counts(text, integer) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_data.question_use_global_counts(text, integer)
    TO ple_api_owner;

SET LOCAL ROLE ple_api_owner;
REVOKE ALL ON FUNCTION ple_api.read_question_use_details(text, integer) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_api.read_question_use_details(text, integer) TO ple_app;
