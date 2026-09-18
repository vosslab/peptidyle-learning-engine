-- Privileges from profile_media.sql.

SET LOCAL ROLE ple_data_owner;

REVOKE ALL ON ple_data.profile_image_delivery FROM PUBLIC;

REVOKE ALL ON FUNCTION ple_data.validate_profile_image_delivery_object_record() FROM PUBLIC;

GRANT REFERENCES (delivery_id, object_id, profile_image_id) ON ple_data.profile_image_delivery
    TO ple_private_owner;

REVOKE ALL ON ple_data.provided_avatar FROM PUBLIC;

GRANT SELECT, REFERENCES (provided_avatar_id) ON ple_data.provided_avatar TO ple_private_owner;

SET LOCAL ROLE ple_private_owner;

REVOKE ALL ON ple_private.account_avatar, ple_private.profile_image_work FROM PUBLIC;

REVOKE ALL ON FUNCTION ple_private.validate_account_avatar_profile_image(),
    ple_private.validate_account_avatar_provided_avatar() FROM PUBLIC;

REVOKE ALL ON FUNCTION ple_private.record_initial_account_avatar() FROM PUBLIC;

SET LOCAL ROLE ple_data_owner;

GRANT SELECT, INSERT, UPDATE ON ple_data.object_delivery, ple_data.profile_image_delivery TO ple_api_owner;

GRANT SELECT ON ple_data.provided_avatar TO ple_api_owner;

SET LOCAL ROLE ple_private_owner;

GRANT SELECT, INSERT, UPDATE ON ple_private.account_avatar, ple_private.profile_image_work,
    ple_private.object_storage_check,
    ple_private.object_cleanup_manifest TO ple_api_owner;

SET LOCAL ROLE ple_audit_owner;

GRANT INSERT ON ple_audit.object_storage_check_event, ple_audit.object_cleanup_receipt TO ple_api_owner;

SET LOCAL ROLE ple_private_owner;

REVOKE ALL PRIVILEGES ON FUNCTION ple_private.list_instructor_account_avatar_summaries() FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_private.list_instructor_account_avatar_summaries() TO ple_api_owner;

SET LOCAL ROLE ple_api_owner;

REVOKE ALL ON FUNCTION ple_api.list_instructor_account_avatar_summaries(), ple_api.current_account_avatar(), ple_api.select_provided_account_avatar(text), ple_api.prepare_account_profile_image(uuid,uuid,bytea,bigint), ple_api.complete_account_profile_image_put(uuid), ple_api.require_account_profile_image_repair(uuid), ple_api.prepare_account_profile_image_deletion(uuid), ple_api.complete_account_profile_image_deletion(uuid), ple_api.require_account_profile_image_deletion_repair(uuid), ple_api.record_account_profile_image_cleanup_check(uuid,boolean,bytea), ple_api.finalize_account_profile_image(uuid), ple_api.resolve_current_account_profile_image(uuid) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_api.list_instructor_account_avatar_summaries(), ple_api.current_account_avatar(), ple_api.select_provided_account_avatar(text), ple_api.prepare_account_profile_image(uuid,uuid,bytea,bigint), ple_api.complete_account_profile_image_put(uuid), ple_api.require_account_profile_image_repair(uuid), ple_api.prepare_account_profile_image_deletion(uuid), ple_api.complete_account_profile_image_deletion(uuid), ple_api.require_account_profile_image_deletion_repair(uuid), ple_api.record_account_profile_image_cleanup_check(uuid,boolean,bytea), ple_api.finalize_account_profile_image(uuid), ple_api.resolve_current_account_profile_image(uuid) TO ple_app;

