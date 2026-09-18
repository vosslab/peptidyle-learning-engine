-- Privileges from question_lineages.sql.

SET LOCAL ROLE ple_private_owner;

GRANT REFERENCES ON TABLE ple_private.account TO ple_data_owner;

REVOKE ALL ON FUNCTION ple_private.question_backend_is_supported_for_production(ple_data.question_backend) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_private.question_backend_is_supported_for_production(
    ple_data.question_backend
)
    TO ple_private_owner, ple_data_owner, ple_api_owner;

SET LOCAL ROLE ple_data_owner;

REVOKE ALL ON FUNCTION ple_data.question_metadata_tags_are_valid(text[]) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_data.question_metadata_tags_are_valid(text[]) TO ple_private_owner;

REVOKE ALL ON TABLE ple_data.published_question, ple_data.question_revision,
    ple_data.published_question_metadata, ple_data.question_publication_event,
    ple_data.question_availability_event FROM PUBLIC;

GRANT INSERT, SELECT ON ple_data.published_question, ple_data.published_question_metadata,
    ple_data.question_revision, ple_data.question_publication_event,
    ple_data.question_availability_event TO ple_private_owner;

GRANT UPDATE ON ple_data.published_question TO ple_private_owner;

GRANT UPDATE ON ple_data.published_question_metadata TO ple_private_owner;

GRANT SELECT ON ple_data.published_question, ple_data.published_question_metadata,
    ple_data.question_revision TO ple_api_owner;

REVOKE ALL ON FUNCTION ple_data.reject_question_lineage_immutable_change(),
    ple_data.validate_question_availability_event() FROM PUBLIC;

REVOKE ALL ON FUNCTION ple_data.set_question_availability(text, bigint, text, text, uuid) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_data.set_question_availability(text, bigint, text, text, uuid) TO ple_api_owner;

SET LOCAL ROLE ple_api_owner;

REVOKE ALL ON FUNCTION ple_api.set_question_availability(text, bigint, text, text, uuid) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_api.set_question_availability(text, bigint, text, text, uuid) TO ple_app;

