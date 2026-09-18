-- Privileges from course_membership.sql.

SET LOCAL ROLE ple_data_owner;

REVOKE ALL ON TABLE ple_data.student_record, ple_data.course_membership, ple_data.course_membership_event FROM PUBLIC;

GRANT SELECT, INSERT ON ple_data.student_record, ple_data.course_membership, ple_data.course_membership_event TO ple_api_owner;

REVOKE ALL ON FUNCTION ple_data.reject_course_membership_change(), ple_data.reject_course_membership_event_change(), ple_data.record_course_membership_start(), ple_data.assert_student_membership_record(), ple_data.assert_course_membership_event_transition(), ple_data.assert_assigned_instructor_membership() FROM PUBLIC;

REVOKE ALL ON FUNCTION ple_data.course_membership_is_active(uuid) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_data.course_membership_is_active(uuid) TO ple_api_owner, ple_app, ple_private_owner, ple_data_owner;

