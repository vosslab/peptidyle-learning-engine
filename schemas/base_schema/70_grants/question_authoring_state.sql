-- Privileges from question_authoring_state.sql.

SET LOCAL ROLE ple_data_owner;

GRANT REFERENCES ON TABLE ple_data.question_revision TO ple_private_owner;

GRANT REFERENCES ON TABLE ple_data.published_question TO ple_private_owner;

SET LOCAL ROLE ple_private_owner;

REVOKE ALL ON FUNCTION ple_private.current_session_is_authoring_workspace_owner(uuid),
    ple_private.current_session_can_access_authoring_workspace(uuid),
    ple_private.question_revision_has_source_binding(text, integer)
    FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_private.current_session_is_authoring_workspace_owner(uuid),
    ple_private.current_session_can_access_authoring_workspace(uuid) TO ple_api_owner;

GRANT EXECUTE ON FUNCTION ple_private.question_revision_has_source_binding(text, integer)
    TO ple_data_owner;

REVOKE ALL ON ALL TABLES IN SCHEMA ple_private FROM PUBLIC;

REVOKE ALL ON FUNCTION ple_private.question_source_binding_fields_are_valid(
    ple_data.question_backend, ple_data.question_format,
    text, text, text, text, boolean),
    ple_private.reject_immutable_question_source_change(),
    ple_private.reject_draft_question_fork_source_change(),
    ple_private.validate_question_revision_source_binding(),
    ple_private.validate_authoring_workspace_collaborator_event(),
    ple_private.reject_authoring_workspace_collaborator_event_change(),
    ple_private.reject_committed_workspace_import_item_result_change(),
    ple_private.validate_draft_question_source_binding_object_record(),
    ple_private.validate_question_revision_source_binding_object_record() FROM PUBLIC;

