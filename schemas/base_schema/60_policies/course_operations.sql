-- Row security policies from course_operations.sql.

SET LOCAL ROLE ple_data_owner;

ALTER TABLE ple_data.course_theme ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_data.course_theme FORCE ROW LEVEL SECURITY;
CREATE POLICY course_theme_read ON ple_data.course_theme
    FOR SELECT TO ple_data_owner, ple_api_owner, ple_app USING (true);

-- Course, roster, and narrowly scoped support operations. Structures live in
-- the preceding Course modules so this file can resolve their exact roots.
CREATE POLICY course_instance_data_owner_instructor_read ON ple_data.course_instance
    FOR SELECT TO ple_data_owner
    USING (ple_api.current_session_account_is_course_instructor(course_instance_id));

CREATE POLICY course_instance_member_read ON ple_data.course_instance
    FOR SELECT TO ple_app USING (ple_api.current_session_account_is_course_member(course_instance_id));

CREATE POLICY course_membership_instructor_or_self_read ON ple_data.course_membership
    FOR SELECT TO ple_app USING (
        ple_api.current_session_account_is_course_instructor(course_instance_id)
        OR ple_api.current_session_account_owns_course_membership(course_instance_id, course_membership_id)
    );

CREATE POLICY student_record_instructor_or_self_read ON ple_data.student_record
    FOR SELECT TO ple_app USING (
        ple_api.current_session_account_is_course_instructor(course_instance_id)
        OR ple_api.current_session_account_owns_student_record(course_instance_id, student_record_id)
    );

