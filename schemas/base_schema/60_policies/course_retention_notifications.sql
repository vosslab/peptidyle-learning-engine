-- Row security policies from course_retention_notifications.sql.

SET LOCAL ROLE ple_private_owner;

ALTER TABLE ple_private.course_retention_notification ENABLE ROW LEVEL SECURITY;

ALTER TABLE ple_private.course_retention_notification FORCE ROW LEVEL SECURITY;

CREATE POLICY course_retention_notification_private_owner_access
    ON ple_private.course_retention_notification FOR ALL TO ple_private_owner
    USING (true) WITH CHECK (true);

SET LOCAL ROLE ple_data_owner;

CREATE POLICY course_instance_retention_notification_owner_read
    ON ple_data.course_instance FOR SELECT
    TO ple_course_retention_notification_owner USING (true);

CREATE POLICY course_retention_policy_notification_owner_read
    ON ple_data.course_retention_policy FOR SELECT
    TO ple_course_retention_notification_owner USING (true);

CREATE POLICY course_membership_retention_notification_owner_read
    ON ple_data.course_membership FOR SELECT
    TO ple_course_retention_notification_owner USING (true);

CREATE POLICY course_membership_event_retention_notification_owner_read
    ON ple_data.course_membership_event FOR SELECT
    TO ple_course_retention_notification_owner USING (true);

SET LOCAL ROLE ple_private_owner;

CREATE POLICY course_retention_notification_owner_access
    ON ple_private.course_retention_notification FOR ALL
    TO ple_course_retention_notification_owner USING (true) WITH CHECK (true);

CREATE POLICY account_retention_notification_owner_read ON ple_private.account
    FOR SELECT TO ple_course_retention_notification_owner USING (true);

CREATE POLICY account_state_event_retention_notification_owner_read
    ON ple_private.account_state_event FOR SELECT
    TO ple_course_retention_notification_owner USING (true);

CREATE POLICY account_authentication_email_retention_notification_owner_read
    ON ple_private.account_authentication_email FOR SELECT
    TO ple_course_retention_notification_owner USING (true);

