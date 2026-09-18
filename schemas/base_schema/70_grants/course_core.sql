-- Privileges from course_core.sql.

SET LOCAL ROLE ple_data_owner;

REVOKE ALL ON TABLE ple_data.course_instance, ple_data.course_origin FROM PUBLIC;

REVOKE ALL ON FUNCTION ple_data.reject_course_origin_change(),
    ple_data.enforce_course_instance_retention_schedule(),
    ple_data.reject_course_instance_source_change() FROM PUBLIC;

GRANT SELECT, INSERT ON ple_data.course_instance, ple_data.course_origin TO ple_api_owner;

GRANT UPDATE (content_discipline_id, content_subject_id, content_topic_id, content_subtopic_id, tags, metadata_etag)
    ON ple_data.course_instance TO ple_api_owner;

GRANT REFERENCES ON TABLE ple_data.course_instance TO ple_private_owner, ple_audit_owner;

GRANT REFERENCES ON TABLE ple_data.blueprint_course_revision TO ple_audit_owner;

SET LOCAL ROLE ple_audit_owner;

REVOKE ALL ON TABLE ple_audit.course_instance_creation_event FROM PUBLIC;

REVOKE ALL ON FUNCTION ple_audit.reject_course_instance_creation_event_change(),
    ple_audit.record_course_instance_creation_event(uuid, text, text, text, integer, text, text, timestamp with time zone) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_audit.record_course_instance_creation_event(uuid, text, text, text, integer, text, text, timestamp with time zone) TO ple_api_owner;

