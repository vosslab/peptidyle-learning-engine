-- Row security policies from course_media.sql.

SET LOCAL ROLE ple_data_owner;

ALTER TABLE ple_data.course_banner ENABLE ROW LEVEL SECURITY;

ALTER TABLE ple_data.course_banner FORCE ROW LEVEL SECURITY;

ALTER TABLE ple_data.course_banner_rendition ENABLE ROW LEVEL SECURITY;

ALTER TABLE ple_data.course_banner_rendition FORCE ROW LEVEL SECURITY;

ALTER TABLE ple_data.course_banner_delivery ENABLE ROW LEVEL SECURITY;

ALTER TABLE ple_data.course_banner_delivery FORCE ROW LEVEL SECURITY;

CREATE POLICY course_banner_data_owner_access ON ple_data.course_banner
    FOR ALL TO ple_data_owner USING (true) WITH CHECK (true);

CREATE POLICY course_banner_rendition_data_owner_access ON ple_data.course_banner_rendition
    FOR ALL TO ple_data_owner USING (true) WITH CHECK (true);

CREATE POLICY course_banner_delivery_data_owner_access ON ple_data.course_banner_delivery
    FOR ALL TO ple_data_owner USING (true) WITH CHECK (true);

SET LOCAL ROLE ple_private_owner;

ALTER TABLE ple_private.course_banner_upload ENABLE ROW LEVEL SECURITY;

ALTER TABLE ple_private.course_banner_upload FORCE ROW LEVEL SECURITY;

ALTER TABLE ple_private.course_banner_storage_subject ENABLE ROW LEVEL SECURITY;

ALTER TABLE ple_private.course_banner_storage_subject FORCE ROW LEVEL SECURITY;

ALTER TABLE ple_private.course_banner_work ENABLE ROW LEVEL SECURITY;

ALTER TABLE ple_private.course_banner_work FORCE ROW LEVEL SECURITY;

ALTER TABLE ple_private.course_banner_prepared_presentation ENABLE ROW LEVEL SECURITY;

ALTER TABLE ple_private.course_banner_prepared_presentation FORCE ROW LEVEL SECURITY;

CREATE POLICY course_banner_upload_private_owner_access ON ple_private.course_banner_upload
    FOR ALL TO ple_private_owner USING (true) WITH CHECK (true);

CREATE POLICY course_banner_subject_private_owner_access ON ple_private.course_banner_storage_subject
    FOR ALL TO ple_private_owner USING (true) WITH CHECK (true);

CREATE POLICY course_banner_work_private_owner_access ON ple_private.course_banner_work
    FOR ALL TO ple_private_owner USING (true) WITH CHECK (true);

CREATE POLICY course_banner_presentation_private_owner_access ON ple_private.course_banner_prepared_presentation
    FOR ALL TO ple_private_owner USING (true) WITH CHECK (true);

SET LOCAL ROLE ple_data_owner;

CREATE POLICY course_instance_api_owner_course_media ON ple_data.course_instance
    FOR ALL TO ple_api_owner USING (true) WITH CHECK (true);

CREATE POLICY course_banner_api_owner_course_media ON ple_data.course_banner
    FOR ALL TO ple_api_owner USING (true) WITH CHECK (true);

CREATE POLICY course_banner_rendition_api_owner_course_media ON ple_data.course_banner_rendition
    FOR ALL TO ple_api_owner USING (true) WITH CHECK (true);

CREATE POLICY course_banner_delivery_api_owner_course_media ON ple_data.course_banner_delivery
    FOR ALL TO ple_api_owner USING (true) WITH CHECK (true);

CREATE POLICY object_delivery_api_owner_course_media ON ple_data.object_delivery
    FOR ALL TO ple_api_owner USING (true) WITH CHECK (true);

SET LOCAL ROLE ple_private_owner;

CREATE POLICY course_banner_upload_api_owner_course_media ON ple_private.course_banner_upload
    FOR ALL TO ple_api_owner USING (true) WITH CHECK (true);

CREATE POLICY course_banner_subject_api_owner_course_media ON ple_private.course_banner_storage_subject
    FOR ALL TO ple_api_owner USING (true) WITH CHECK (true);

CREATE POLICY course_banner_work_api_owner_course_media ON ple_private.course_banner_work
    FOR ALL TO ple_api_owner USING (true) WITH CHECK (true);

CREATE POLICY course_banner_presentation_api_owner_course_media ON ple_private.course_banner_prepared_presentation
    FOR ALL TO ple_api_owner USING (true) WITH CHECK (true);

CREATE POLICY object_storage_check_api_owner_course_media ON ple_private.object_storage_check
    FOR ALL TO ple_api_owner USING (true) WITH CHECK (true);

CREATE POLICY object_cleanup_manifest_api_owner_course_media ON ple_private.object_cleanup_manifest
    FOR ALL TO ple_api_owner USING (true) WITH CHECK (true);

SET LOCAL ROLE ple_audit_owner;

CREATE POLICY object_storage_check_event_api_owner_course_media ON ple_audit.object_storage_check_event
    FOR INSERT TO ple_api_owner WITH CHECK (true);

CREATE POLICY object_cleanup_receipt_api_owner_course_media ON ple_audit.object_cleanup_receipt
    FOR INSERT TO ple_api_owner WITH CHECK (true);

