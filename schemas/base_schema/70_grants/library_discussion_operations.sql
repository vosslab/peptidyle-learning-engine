-- Privileges from library_discussion_operations.sql.

SET LOCAL ROLE ple_data_owner;

REVOKE ALL ON FUNCTION ple_data.library_object_current_revision_number(text, text, boolean),
    ple_data.library_object_revision_exists(text, text, bigint),
    ple_data.current_actor_owns_library_object(text, text),
    ple_data.current_actor_is_library_discussion_participant(),
    ple_data.current_actor_may_manage_library_discussion(text, text),
    ple_data.require_library_discussion_reader(text, text),
    ple_data.require_library_discussion_participant(),
    ple_data.require_library_discussion_manager(text, text),
    ple_data.create_library_improvement_thread(text, text, text),
    ple_data.reply_to_library_improvement_thread(text, text, uuid, text),
    ple_data.edit_own_library_improvement_post(text, text, uuid, text),
    ple_data.set_library_improvement_thread_state(text, text, uuid, boolean),
    ple_data.create_library_impact_notice(text, text, bigint, text),
    ple_data.update_library_impact_notice(text, text, uuid, bigint, text),
    ple_data.cancel_library_impact_notice(text, text, uuid),
    ple_data.read_library_improvement_threads(text, text),
    ple_data.read_library_improvement_posts(uuid),
    ple_data.read_library_impact_notices(text, text),
    ple_data.read_library_object_discussion_management(text, text) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_data.create_library_improvement_thread(text, text, text),
    ple_data.reply_to_library_improvement_thread(text, text, uuid, text),
    ple_data.edit_own_library_improvement_post(text, text, uuid, text),
    ple_data.set_library_improvement_thread_state(text, text, uuid, boolean),
    ple_data.create_library_impact_notice(text, text, bigint, text),
    ple_data.update_library_impact_notice(text, text, uuid, bigint, text),
    ple_data.cancel_library_impact_notice(text, text, uuid),
    ple_data.read_library_improvement_threads(text, text),
    ple_data.read_library_improvement_posts(uuid),
    ple_data.read_library_impact_notices(text, text),
    ple_data.read_library_object_discussion_management(text, text) TO ple_api_owner;

SET LOCAL ROLE ple_api_owner;

REVOKE ALL ON FUNCTION ple_api.create_library_improvement_thread(text, text, text),
    ple_api.reply_to_library_improvement_thread(text, text, uuid, text),
    ple_api.edit_own_library_improvement_post(text, text, uuid, text),
    ple_api.set_library_improvement_thread_state(text, text, uuid, boolean),
    ple_api.create_library_impact_notice(text, text, bigint, text),
    ple_api.update_library_impact_notice(text, text, uuid, bigint, text),
    ple_api.cancel_library_impact_notice(text, text, uuid),
    ple_api.read_library_improvement_threads(text, text),
    ple_api.read_library_improvement_posts(uuid),
    ple_api.read_library_impact_notices(text, text),
    ple_api.read_library_object_discussion_management(text, text) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_api.create_library_improvement_thread(text, text, text),
    ple_api.reply_to_library_improvement_thread(text, text, uuid, text),
    ple_api.edit_own_library_improvement_post(text, text, uuid, text),
    ple_api.set_library_improvement_thread_state(text, text, uuid, boolean),
    ple_api.create_library_impact_notice(text, text, bigint, text),
    ple_api.update_library_impact_notice(text, text, uuid, bigint, text),
    ple_api.cancel_library_impact_notice(text, text, uuid),
    ple_api.read_library_improvement_threads(text, text),
    ple_api.read_library_improvement_posts(uuid),
    ple_api.read_library_impact_notices(text, text),
    ple_api.read_library_object_discussion_management(text, text) TO ple_app;

