-- Privileges from object_records.sql.

SET LOCAL ROLE ple_private_owner;

REVOKE ALL ON TABLE ple_private.object_record FROM PUBLIC;

REVOKE ALL ON FUNCTION ple_private.reject_object_record_change() FROM PUBLIC;


-- Object-owned relations use this immutable identity as their foreign-key
-- target.  A delivery may remain pending while an external put is in flight,
-- so it deliberately does not impose a global Object Record FK.
GRANT REFERENCES ON ple_private.object_record TO ple_data_owner, ple_private_owner;

GRANT SELECT ON ple_private.object_record TO ple_data_owner;

GRANT SELECT, INSERT ON ple_private.object_record TO ple_api_owner;

REVOKE ALL ON FUNCTION ple_private.register_workspace_question_source_object(
    uuid, uuid, jsonb, bytea, bigint, text, bigint) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_private.register_workspace_question_source_object(
    uuid, uuid, jsonb, bytea, bigint, text, bigint) TO ple_api_owner;

SET LOCAL ROLE ple_api_owner;

REVOKE ALL ON FUNCTION ple_api.register_workspace_question_source_object(
    uuid, uuid, jsonb, bytea, bigint, text, bigint) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_api.register_workspace_question_source_object(
    uuid, uuid, jsonb, bytea, bigint, text, bigint) TO ple_app;

SET LOCAL ROLE ple_data_owner;

REVOKE ALL ON TABLE ple_data.object_delivery, ple_data.course_object_delivery FROM PUBLIC;

REVOKE ALL ON FUNCTION ple_data.require_exact_available_object_delivery_owner() FROM PUBLIC;

GRANT REFERENCES ON ple_data.object_delivery TO ple_private_owner;

SET LOCAL ROLE ple_private_owner;

REVOKE ALL ON TABLE ple_private.object_storage_check, ple_private.object_cleanup_manifest FROM PUBLIC;

GRANT REFERENCES ON ple_private.object_storage_check, ple_private.object_cleanup_manifest TO ple_audit_owner;

SET LOCAL ROLE ple_audit_owner;

REVOKE ALL ON TABLE ple_audit.object_storage_check_event,
    ple_audit.object_cleanup_receipt FROM PUBLIC;

