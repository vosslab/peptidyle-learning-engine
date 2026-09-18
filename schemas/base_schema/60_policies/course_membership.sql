-- Row security policies from course_membership.sql.

SET LOCAL ROLE ple_data_owner;

ALTER TABLE ple_data.student_record ENABLE ROW LEVEL SECURITY;

ALTER TABLE ple_data.student_record FORCE ROW LEVEL SECURITY;

ALTER TABLE ple_data.course_membership ENABLE ROW LEVEL SECURITY;

ALTER TABLE ple_data.course_membership FORCE ROW LEVEL SECURITY;

ALTER TABLE ple_data.course_membership_event ENABLE ROW LEVEL SECURITY;

ALTER TABLE ple_data.course_membership_event FORCE ROW LEVEL SECURITY;

CREATE POLICY student_record_api_owner_access ON ple_data.student_record FOR ALL TO ple_api_owner USING (true) WITH CHECK (true);

CREATE POLICY course_membership_api_owner_access ON ple_data.course_membership FOR ALL TO ple_api_owner USING (true) WITH CHECK (true);

CREATE POLICY course_membership_event_api_owner_access ON ple_data.course_membership_event FOR ALL TO ple_api_owner USING (true) WITH CHECK (true);


-- The deferred invariant executes as ple_data_owner.  Forced RLS therefore
-- needs this narrow read capability for the three relations it verifies.
CREATE POLICY course_instance_data_owner_assigned_instructor_check ON ple_data.course_instance
    FOR SELECT TO ple_data_owner USING (true);

CREATE POLICY course_membership_data_owner_assigned_instructor_check ON ple_data.course_membership
    FOR SELECT TO ple_data_owner USING (true);

CREATE POLICY course_membership_event_data_owner_assigned_instructor_check ON ple_data.course_membership_event
    FOR SELECT TO ple_data_owner USING (true);

