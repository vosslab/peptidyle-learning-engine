-- Privileges from archived_student_work_recovery.sql.

SET LOCAL ROLE ple_data_owner;

-- Protected evidence recovery, not an ordinary history read or restoration.
-- The retained backend document is data for the trusted server, not an HTML
-- response. No object addresses, credentials, Profiles, or receipt blobs cross
-- this boundary. Existing Student/history/Instructor readers remain unchanged.
GRANT SELECT (archive_after_retention_start, delete_after_archive, policy_key)
    ON ple_data.course_retention_policy TO ple_api_owner;

SET LOCAL ROLE ple_private_owner;

REVOKE ALL ON FUNCTION ple_private.read_archived_assessment_attempt_evidence(uuid, bigint) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_private.read_archived_assessment_attempt_evidence(uuid, bigint) TO ple_api_owner;

REVOKE ALL ON FUNCTION ple_private.select_archived_assessment_attempt_evidence(uuid, bigint, integer) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_private.select_archived_assessment_attempt_evidence(uuid, bigint, integer) TO ple_api_owner;

SET LOCAL ROLE ple_api_owner;

REVOKE ALL ON FUNCTION ple_api.lock_archived_course_for_recovery(text) FROM PUBLIC;

REVOKE ALL ON FUNCTION ple_api.read_archived_assessment_attempt_for_recovery(text, bigint) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_api.read_archived_assessment_attempt_for_recovery(text, bigint) TO ple_app;

REVOKE ALL ON FUNCTION ple_api.select_archived_assessment_attempts_for_recovery(text, bigint, integer) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_api.select_archived_assessment_attempts_for_recovery(text, bigint, integer) TO ple_app;

