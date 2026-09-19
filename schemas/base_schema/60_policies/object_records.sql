-- Row security policies from object_records.sql.

SET LOCAL ROLE ple_private_owner;

ALTER TABLE ple_private.object_record ENABLE ROW LEVEL SECURITY;

ALTER TABLE ple_private.object_record FORCE ROW LEVEL SECURITY;

CREATE POLICY object_record_private_owner_access ON ple_private.object_record
    FOR ALL TO ple_private_owner USING (true) WITH CHECK (true);

CREATE POLICY object_record_data_owner_read_access ON ple_private.object_record
    FOR SELECT TO ple_data_owner USING (true);

CREATE POLICY object_record_api_owner_insert_access ON ple_private.object_record
    FOR ALL TO ple_api_owner USING (true) WITH CHECK (true);

SET LOCAL ROLE ple_data_owner;

ALTER TABLE ple_data.object_delivery ENABLE ROW LEVEL SECURITY;

ALTER TABLE ple_data.object_delivery FORCE ROW LEVEL SECURITY;

ALTER TABLE ple_data.course_object_delivery ENABLE ROW LEVEL SECURITY;

ALTER TABLE ple_data.course_object_delivery FORCE ROW LEVEL SECURITY;

CREATE POLICY object_delivery_data_owner_access ON ple_data.object_delivery
    FOR ALL TO ple_data_owner USING (true) WITH CHECK (true);

CREATE POLICY course_object_delivery_data_owner_access ON ple_data.course_object_delivery
    FOR ALL TO ple_data_owner USING (true) WITH CHECK (true);

SET LOCAL ROLE ple_private_owner;

ALTER TABLE ple_private.object_storage_check ENABLE ROW LEVEL SECURITY;

ALTER TABLE ple_private.object_storage_check FORCE ROW LEVEL SECURITY;

ALTER TABLE ple_private.object_cleanup_manifest ENABLE ROW LEVEL SECURITY;

ALTER TABLE ple_private.object_cleanup_manifest FORCE ROW LEVEL SECURITY;

CREATE POLICY object_storage_check_private_owner_access ON ple_private.object_storage_check
    FOR ALL TO ple_private_owner USING (true) WITH CHECK (true);

CREATE POLICY object_cleanup_manifest_private_owner_access ON ple_private.object_cleanup_manifest
    FOR ALL TO ple_private_owner USING (true) WITH CHECK (true);

SET LOCAL ROLE ple_audit_owner;

ALTER TABLE ple_audit.object_storage_check_event ENABLE ROW LEVEL SECURITY;

ALTER TABLE ple_audit.object_storage_check_event FORCE ROW LEVEL SECURITY;

ALTER TABLE ple_audit.object_cleanup_receipt ENABLE ROW LEVEL SECURITY;

ALTER TABLE ple_audit.object_cleanup_receipt FORCE ROW LEVEL SECURITY;

CREATE POLICY object_storage_check_event_audit_owner_access ON ple_audit.object_storage_check_event
    FOR ALL TO ple_audit_owner USING (true) WITH CHECK (true);

CREATE POLICY object_cleanup_receipt_audit_owner_access ON ple_audit.object_cleanup_receipt
    FOR ALL TO ple_audit_owner USING (true) WITH CHECK (true);

