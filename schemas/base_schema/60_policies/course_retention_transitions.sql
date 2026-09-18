-- Row security policies from course_retention_transitions.sql.

SET LOCAL ROLE ple_data_owner;

CREATE POLICY course_instance_retention_executor_access
    ON ple_data.course_instance FOR ALL TO ple_course_retention_executor
    USING (true) WITH CHECK (true);

CREATE POLICY course_retention_policy_executor_read
    ON ple_data.course_retention_policy FOR SELECT TO ple_course_retention_executor
    USING (true);

CREATE POLICY assessment_retention_executor_read
    ON ple_data.assessment FOR SELECT TO ple_course_retention_executor
    USING (true);

CREATE POLICY assessment_retention_executor_student_work_root_lock
    ON ple_data.assessment FOR UPDATE TO ple_course_retention_executor
    USING (true) WITH CHECK (true);

CREATE POLICY student_record_retention_executor_access
    ON ple_data.student_record FOR ALL TO ple_course_retention_executor
    USING (true) WITH CHECK (true);

CREATE POLICY course_membership_retention_executor_access
    ON ple_data.course_membership FOR ALL TO ple_course_retention_executor
    USING (true) WITH CHECK (true);

CREATE POLICY course_membership_event_retention_executor_access
    ON ple_data.course_membership_event FOR ALL TO ple_course_retention_executor
    USING (true) WITH CHECK (true);

SET LOCAL ROLE ple_private_owner;

CREATE POLICY assessment_attempt_retention_executor_access
    ON ple_private.assessment_attempt FOR ALL TO ple_course_retention_executor
    USING (true) WITH CHECK (true);

CREATE POLICY student_assessment_accommodation_retention_executor_access
    ON ple_private.student_assessment_accommodation FOR ALL TO ple_course_retention_executor
    USING (true) WITH CHECK (true);

CREATE POLICY course_invitation_retention_executor_access
    ON ple_private.course_invitation FOR ALL TO ple_course_retention_executor
    USING (true) WITH CHECK (true);

CREATE POLICY course_invitation_event_retention_executor_access
    ON ple_private.course_invitation_event FOR ALL TO ple_course_retention_executor
    USING (true) WITH CHECK (true);

CREATE POLICY course_roster_profile_retention_executor_access
    ON ple_private.course_roster_profile FOR ALL TO ple_course_retention_executor
    USING (true) WITH CHECK (true);

SET LOCAL ROLE ple_audit_owner;

CREATE POLICY course_roster_event_retention_executor_access
    ON ple_audit.course_roster_event FOR ALL TO ple_course_retention_executor
    USING (true) WITH CHECK (true);

