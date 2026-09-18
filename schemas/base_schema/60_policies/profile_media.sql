-- Row security policies from profile_media.sql.

SET LOCAL ROLE ple_data_owner;

ALTER TABLE ple_data.profile_image_delivery ENABLE ROW LEVEL SECURITY;

ALTER TABLE ple_data.profile_image_delivery FORCE ROW LEVEL SECURITY;

CREATE POLICY profile_image_delivery_data_owner_access ON ple_data.profile_image_delivery
    FOR ALL TO ple_data_owner USING (true) WITH CHECK (true);

ALTER TABLE ple_data.provided_avatar ENABLE ROW LEVEL SECURITY;

ALTER TABLE ple_data.provided_avatar FORCE ROW LEVEL SECURITY;

CREATE POLICY provided_avatar_data_owner_access ON ple_data.provided_avatar
    FOR ALL TO ple_data_owner USING (true) WITH CHECK (true);

CREATE POLICY provided_avatar_private_owner_read ON ple_data.provided_avatar
    FOR SELECT TO ple_private_owner USING (true);

SET LOCAL ROLE ple_private_owner;

ALTER TABLE ple_private.account_avatar ENABLE ROW LEVEL SECURITY;

ALTER TABLE ple_private.account_avatar FORCE ROW LEVEL SECURITY;

ALTER TABLE ple_private.profile_image_work ENABLE ROW LEVEL SECURITY;

ALTER TABLE ple_private.profile_image_work FORCE ROW LEVEL SECURITY;

CREATE POLICY account_avatar_private_owner_access ON ple_private.account_avatar
    FOR ALL TO ple_private_owner USING (true) WITH CHECK (true);

CREATE POLICY profile_image_work_private_owner_access ON ple_private.profile_image_work
    FOR ALL TO ple_private_owner USING (true) WITH CHECK (true);

SET LOCAL ROLE ple_data_owner;

CREATE POLICY object_delivery_api_owner_profile_media ON ple_data.object_delivery
    FOR ALL TO ple_api_owner USING (true) WITH CHECK (true);

CREATE POLICY profile_image_delivery_api_owner_profile_media ON ple_data.profile_image_delivery
    FOR ALL TO ple_api_owner USING (true) WITH CHECK (true);

CREATE POLICY provided_avatar_api_owner_profile_media ON ple_data.provided_avatar
    FOR SELECT TO ple_api_owner USING (true);

SET LOCAL ROLE ple_private_owner;

CREATE POLICY account_avatar_api_owner_profile_media ON ple_private.account_avatar
    FOR ALL TO ple_api_owner USING (true) WITH CHECK (true);

CREATE POLICY profile_image_work_api_owner_profile_media ON ple_private.profile_image_work
    FOR ALL TO ple_api_owner USING (true) WITH CHECK (true);

CREATE POLICY storage_check_api_owner_profile_media ON ple_private.object_storage_check
    FOR ALL TO ple_api_owner USING (true) WITH CHECK (true);

CREATE POLICY cleanup_manifest_api_owner_profile_media ON ple_private.object_cleanup_manifest
    FOR ALL TO ple_api_owner USING (true) WITH CHECK (true);

SET LOCAL ROLE ple_audit_owner;

CREATE POLICY storage_check_event_api_owner_profile_media ON ple_audit.object_storage_check_event
    FOR INSERT TO ple_api_owner WITH CHECK (true);

CREATE POLICY cleanup_receipt_api_owner_profile_media ON ple_audit.object_cleanup_receipt
    FOR INSERT TO ple_api_owner WITH CHECK (true);

