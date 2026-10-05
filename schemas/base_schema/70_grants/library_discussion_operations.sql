-- Privileges from library impact notices.

SET LOCAL ROLE ple_data_owner;

REVOKE ALL ON FUNCTION ple_data.library_object_current_revision_number(text, text, boolean),
    ple_data.library_object_revision_exists(text, text, bigint),
    ple_data.current_actor_owns_library_object(text, text),
    ple_data.require_library_impact_notice_manager(text, text),
    ple_data.create_library_impact_notice(text, text, bigint, text),
    ple_data.update_library_impact_notice(text, text, uuid, bigint, text),
    ple_data.cancel_library_impact_notice(text, text, uuid) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_data.create_library_impact_notice(text, text, bigint, text),
    ple_data.update_library_impact_notice(text, text, uuid, bigint, text),
    ple_data.cancel_library_impact_notice(text, text, uuid) TO ple_api_owner;

SET LOCAL ROLE ple_api_owner;

REVOKE ALL ON FUNCTION ple_api.create_library_impact_notice(text, text, bigint, text),
    ple_api.update_library_impact_notice(text, text, uuid, bigint, text),
    ple_api.cancel_library_impact_notice(text, text, uuid) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_api.create_library_impact_notice(text, text, bigint, text),
    ple_api.update_library_impact_notice(text, text, uuid, bigint, text),
    ple_api.cancel_library_impact_notice(text, text, uuid) TO ple_app;
