-- Row security policies from course_roster.sql.

SET LOCAL ROLE ple_private_owner;

ALTER TABLE ple_private.course_invitation ENABLE ROW LEVEL SECURITY;

ALTER TABLE ple_private.course_invitation FORCE ROW LEVEL SECURITY;

ALTER TABLE ple_private.course_invitation_event ENABLE ROW LEVEL SECURITY;

ALTER TABLE ple_private.course_invitation_event FORCE ROW LEVEL SECURITY;

ALTER TABLE ple_private.course_roster_profile ENABLE ROW LEVEL SECURITY;

ALTER TABLE ple_private.course_roster_profile FORCE ROW LEVEL SECURITY;

CREATE POLICY course_invitation_api_owner_access ON ple_private.course_invitation FOR ALL TO ple_api_owner USING(true) WITH CHECK(true);

CREATE POLICY course_invitation_event_api_owner_access ON ple_private.course_invitation_event FOR ALL TO ple_api_owner USING(true) WITH CHECK(true);

CREATE POLICY course_roster_profile_api_owner_access ON ple_private.course_roster_profile FOR ALL TO ple_api_owner USING(true) WITH CHECK(true);

CREATE POLICY course_invitation_private_owner_read ON ple_private.course_invitation
    FOR SELECT TO ple_private_owner USING (true);

CREATE POLICY course_invitation_event_private_owner_read ON ple_private.course_invitation_event
    FOR SELECT TO ple_private_owner USING (true);

SET LOCAL ROLE ple_audit_owner;

ALTER TABLE ple_audit.course_roster_event ENABLE ROW LEVEL SECURITY;

ALTER TABLE ple_audit.course_roster_event FORCE ROW LEVEL SECURITY;

CREATE POLICY course_roster_event_owner_write ON ple_audit.course_roster_event FOR INSERT TO ple_audit_owner WITH CHECK(true);

CREATE POLICY course_roster_event_owner_read ON ple_audit.course_roster_event FOR SELECT TO ple_audit_owner USING(true);

