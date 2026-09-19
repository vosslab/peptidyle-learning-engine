-- Privileges from question_publication_operations.sql.

SET LOCAL ROLE ple_private_owner;

REVOKE ALL ON FUNCTION ple_private.bind_draft_question_source(
    uuid, bigint, uuid, text, text, text, text, uuid, text) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_private.bind_draft_question_source(
    uuid, bigint, uuid, text, text, text, text, uuid, text) TO ple_api_owner;

REVOKE ALL ON FUNCTION ple_private.publish_question_revision(
    uuid, bigint, uuid, text, integer, uuid, jsonb, bytea, bigint, text, bigint, text, uuid, jsonb) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_private.publish_question_revision(
    uuid, bigint, uuid, text, integer, uuid, jsonb, bytea, bigint, text, bigint, text, uuid, jsonb) TO ple_api_owner;

SET LOCAL ROLE ple_api_owner;

REVOKE ALL ON FUNCTION ple_api.current_session_account_is_authoring_workspace_owner(uuid),
    ple_api.current_session_account_can_access_authoring_workspace(uuid) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_api.current_session_account_is_authoring_workspace_owner(uuid),
    ple_api.current_session_account_can_access_authoring_workspace(uuid)
    TO ple_app, ple_auth, ple_student, ple_data_owner, ple_private_owner;

REVOKE ALL ON FUNCTION ple_api.bind_draft_question_source(
    uuid, bigint, uuid, text, text, text, text, uuid, text) FROM PUBLIC;

REVOKE ALL ON FUNCTION ple_api.publish_question_revision(
    uuid, bigint, uuid, text, integer, uuid, jsonb, bytea, bigint, text, bigint, text, uuid, jsonb) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_api.bind_draft_question_source(
    uuid, bigint, uuid, text, text, text, text, uuid, text) TO ple_app;

GRANT EXECUTE ON FUNCTION ple_api.publish_question_revision(
    uuid, bigint, uuid, text, integer, uuid, jsonb, bytea, bigint, text, bigint, text, uuid, jsonb) TO ple_app;

SET LOCAL ROLE ple_private_owner;

REVOKE ALL ON FUNCTION ple_private.load_draft_question_publication_source(uuid, bigint, uuid),
    ple_private.publish_new_question_lineage(uuid, bigint, uuid, text, uuid, jsonb, bytea, bigint,
        text, bigint, jsonb, text[], uuid, uuid, uuid, uuid, text, text, uuid, uuid, uuid, jsonb) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_private.load_draft_question_publication_source(uuid, bigint, uuid),
    ple_private.publish_new_question_lineage(uuid, bigint, uuid, text, uuid, jsonb, bytea, bigint,
        text, bigint, jsonb, text[], uuid, uuid, uuid, uuid, text, text, uuid, uuid, uuid, jsonb) TO ple_api_owner;

SET LOCAL ROLE ple_api_owner;

REVOKE ALL ON FUNCTION ple_api.load_draft_question_publication_source(uuid, bigint, uuid),
    ple_api.publish_new_question_lineage(uuid, bigint, uuid, text, uuid, jsonb, bytea, bigint,
        text, bigint, jsonb, text[], uuid, uuid, uuid, uuid, text, text, uuid, uuid, uuid, jsonb) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_api.load_draft_question_publication_source(uuid, bigint, uuid),
    ple_api.publish_new_question_lineage(uuid, bigint, uuid, text, uuid, jsonb, bytea, bigint,
        text, bigint, jsonb, text[], uuid, uuid, uuid, uuid, text, text, uuid, uuid, uuid, jsonb) TO ple_app;

