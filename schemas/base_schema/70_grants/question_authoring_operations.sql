-- Privileges from question_authoring_operations.sql.

SET LOCAL ROLE ple_private_owner;

REVOKE ALL ON FUNCTION ple_private.ensure_own_authoring_workspace(uuid),
    ple_private.fork_published_question_to_draft(uuid, uuid, text, integer, uuid,
        uuid, jsonb, bytea, bigint, text, bigint, jsonb),
    ple_private.current_session_account_owns_draft_question(uuid),
    ple_private.list_authoring_drafts(), ple_private.load_authoring_draft(uuid),
    ple_private.create_authoring_draft(uuid, uuid, uuid, jsonb, bytea, bigint, text, bigint, text, text, text, text, text, text),
    ple_private.save_authoring_draft(uuid, bigint, uuid, jsonb, bytea, bigint, text, bigint, text, text, text, text, uuid, text),
    ple_private.save_authoring_draft_general_feedback(uuid, bigint, text),
    ple_private.delete_draft_question(uuid, bigint)
    FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_private.ensure_own_authoring_workspace(uuid),
    ple_private.fork_published_question_to_draft(uuid, uuid, text, integer, uuid,
        uuid, jsonb, bytea, bigint, text, bigint, jsonb),
    ple_private.current_session_account_owns_draft_question(uuid),
    ple_private.list_authoring_drafts(), ple_private.load_authoring_draft(uuid),
    ple_private.create_authoring_draft(uuid, uuid, uuid, jsonb, bytea, bigint, text, bigint, text, text, text, text, text, text),
    ple_private.save_authoring_draft(uuid, bigint, uuid, jsonb, bytea, bigint, text, bigint, text, text, text, text, uuid, text),
    ple_private.save_authoring_draft_general_feedback(uuid, bigint, text),
    ple_private.delete_draft_question(uuid, bigint)
    TO ple_api_owner;

SET LOCAL ROLE ple_api_owner;

REVOKE ALL ON FUNCTION ple_api.ensure_own_authoring_workspace(uuid),
    ple_api.fork_published_question_to_draft(uuid, uuid, text, integer, uuid,
        uuid, jsonb, bytea, bigint, text, bigint, jsonb),
    ple_api.current_session_account_owns_draft_question(uuid),
    ple_api.list_authoring_drafts(), ple_api.load_authoring_draft(uuid),
    ple_api.create_authoring_draft(uuid, uuid, uuid, jsonb, bytea, bigint, text, bigint, text, text, text, text, text, text),
    ple_api.save_authoring_draft(uuid, bigint, uuid, jsonb, bytea, bigint, text, bigint, text, text, text, text, uuid, text),
    ple_api.save_authoring_draft_general_feedback(uuid, bigint, text),
    ple_api.delete_draft_question(uuid, bigint)
    FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_api.ensure_own_authoring_workspace(uuid),
    ple_api.fork_published_question_to_draft(uuid, uuid, text, integer, uuid,
        uuid, jsonb, bytea, bigint, text, bigint, jsonb),
    ple_api.current_session_account_owns_draft_question(uuid),
    ple_api.list_authoring_drafts(), ple_api.load_authoring_draft(uuid),
    ple_api.create_authoring_draft(uuid, uuid, uuid, jsonb, bytea, bigint, text, bigint, text, text, text, text, text, text),
    ple_api.save_authoring_draft(uuid, bigint, uuid, jsonb, bytea, bigint, text, bigint, text, text, text, text, uuid, text),
    ple_api.save_authoring_draft_general_feedback(uuid, bigint, text),
    ple_api.delete_draft_question(uuid, bigint)
    TO ple_app;

