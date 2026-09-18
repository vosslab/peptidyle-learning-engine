-- Privileges from course_media.sql.

SET LOCAL ROLE ple_data_owner;

REVOKE ALL ON ple_data.course_banner, ple_data.course_banner_rendition,
    ple_data.course_banner_delivery FROM PUBLIC;

REVOKE ALL ON FUNCTION ple_data.validate_course_banner_source_object_record(),
    ple_data.validate_course_banner_rendition_object_record(),
    ple_data.validate_course_banner_delivery_object_record() FROM PUBLIC;

GRANT REFERENCES ON ple_data.course_banner, ple_data.course_banner_rendition TO ple_private_owner;

SET LOCAL ROLE ple_private_owner;

REVOKE ALL ON ple_private.course_banner_upload, ple_private.course_banner_storage_subject,
    ple_private.course_banner_work, ple_private.course_banner_prepared_presentation FROM PUBLIC;

SET LOCAL ROLE ple_data_owner;

GRANT SELECT, INSERT, UPDATE ON ple_data.course_instance, ple_data.course_banner,
    ple_data.course_banner_rendition, ple_data.course_banner_delivery,
    ple_data.object_delivery TO ple_api_owner;

SET LOCAL ROLE ple_private_owner;

GRANT SELECT, INSERT, UPDATE ON ple_private.course_banner_upload,
    ple_private.course_banner_storage_subject, ple_private.course_banner_work,
    ple_private.course_banner_prepared_presentation, ple_private.object_storage_check,
    ple_private.object_cleanup_manifest TO ple_api_owner;

SET LOCAL ROLE ple_audit_owner;

GRANT INSERT ON ple_audit.object_storage_check_event, ple_audit.object_cleanup_receipt TO ple_api_owner;

SET LOCAL ROLE ple_api_owner;

REVOKE ALL ON FUNCTION ple_api.read_course_banner(uuid), ple_api.resolve_current_course_banner(uuid),
    ple_api.stage_course_banner_upload(uuid,uuid,uuid,text,bigint,bytea,integer,integer,bigint),
    ple_api.finalize_course_banner_upload_stage(uuid,uuid), ple_api.read_staged_course_banner_upload(uuid,uuid),
    ple_api.prepare_course_banner_promotion(uuid,uuid,uuid,text,text,uuid,bytea,bigint,text,integer,integer,uuid,bytea,bigint,text,integer,integer),
    ple_api.complete_prepared_course_banner_object(uuid,uuid,uuid), ple_api.prepare_course_banner_object_deletion(uuid),
    ple_api.complete_course_banner_object_deletion(uuid), ple_api.require_course_banner_object_repair(uuid,uuid,uuid),
    ple_api.require_course_banner_deletion_repair(uuid), ple_api.record_course_banner_cleanup_check(uuid,boolean,bytea),
    ple_api.finalize_course_banner_promotion(uuid,uuid,uuid), ple_api.prepare_course_banner_removal(uuid) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_api.read_course_banner(uuid), ple_api.resolve_current_course_banner(uuid),
    ple_api.stage_course_banner_upload(uuid,uuid,uuid,text,bigint,bytea,integer,integer,bigint),
    ple_api.finalize_course_banner_upload_stage(uuid,uuid), ple_api.read_staged_course_banner_upload(uuid,uuid),
    ple_api.prepare_course_banner_promotion(uuid,uuid,uuid,text,text,uuid,bytea,bigint,text,integer,integer,uuid,bytea,bigint,text,integer,integer),
    ple_api.complete_prepared_course_banner_object(uuid,uuid,uuid), ple_api.prepare_course_banner_object_deletion(uuid),
    ple_api.complete_course_banner_object_deletion(uuid), ple_api.require_course_banner_object_repair(uuid,uuid,uuid),
    ple_api.require_course_banner_deletion_repair(uuid), ple_api.record_course_banner_cleanup_check(uuid,boolean,bytea),
    ple_api.finalize_course_banner_promotion(uuid,uuid,uuid), ple_api.prepare_course_banner_removal(uuid) TO ple_app;

