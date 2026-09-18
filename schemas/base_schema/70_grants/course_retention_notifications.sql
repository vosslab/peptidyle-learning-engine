-- Privileges from course_retention_notifications.sql.

SET LOCAL ROLE ple_private_owner;

REVOKE ALL ON TABLE ple_private.course_retention_notification FROM PUBLIC;

REVOKE ALL ON TABLE ple_private.course_retention_notification,
    ple_private.account, ple_private.account_state_event,
    ple_private.account_authentication_email FROM ple_course_retention_notifier;

SET LOCAL ROLE ple_data_owner;

GRANT SELECT ON ple_data.course_instance,
    ple_data.course_membership, ple_data.course_membership_event
    TO ple_course_retention_notification_owner;

GRANT EXECUTE ON FUNCTION ple_data.course_retention_due_actions(timestamp with time zone),
    ple_data.course_membership_is_active(uuid) TO ple_course_retention_notification_owner;

REVOKE ALL ON TABLE ple_data.course_instance,
    ple_data.course_membership, ple_data.course_membership_event FROM ple_course_retention_notifier;

SET LOCAL ROLE ple_private_owner;

GRANT SELECT, INSERT, UPDATE ON ple_private.course_retention_notification
    TO ple_course_retention_notification_owner;

GRANT SELECT ON ple_private.account, ple_private.account_state_event,
    ple_private.account_authentication_email TO ple_course_retention_notification_owner;

SET LOCAL ROLE ple_course_retention_notification_owner;

REVOKE ALL ON FUNCTION ple_api.claim_course_retention_notification(
        timestamp with time zone, integer
    ),
    ple_api.record_course_retention_notification_provider_acceptance(
        uuid, uuid, uuid, timestamp with time zone
    ),
    ple_api.fail_course_retention_notification_before_acceptance(
        uuid, uuid, timestamp with time zone, text
    )
    FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_api.claim_course_retention_notification(
        timestamp with time zone, integer
    ),
    ple_api.record_course_retention_notification_provider_acceptance(
        uuid, uuid, uuid, timestamp with time zone
    ),
    ple_api.fail_course_retention_notification_before_acceptance(
        uuid, uuid, timestamp with time zone, text
    )
    TO ple_course_retention_notifier;

SET LOCAL ROLE ple_api_owner;

REVOKE CREATE ON SCHEMA ple_api FROM ple_course_retention_notification_owner;

