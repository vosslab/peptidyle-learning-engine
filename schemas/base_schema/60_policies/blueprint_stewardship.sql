-- Row security policies from blueprint_stewardship.sql.

SET LOCAL ROLE ple_data_owner;

-- Blueprint Course lineage stewardship.
--
-- Stars and Watches deliberately reference the stable Blueprint Course
-- identity, not an immutable Revision. A later Save therefore preserves an
-- Instructor's chosen relationship without copying or rewriting records.



-- The stewardship definer functions must validate the stable Blueprint
-- identity and lifecycle without granting the application principal a table
-- read. This is deliberately narrower than C409's future Star projection.
CREATE POLICY blueprint_stewardship_data_read ON ple_data.blueprint_course
    FOR SELECT TO ple_data_owner USING (true);


-- The lifecycle trigger reads only the preceding immutable metadata event to
-- classify the three documented transitions. It has no application caller.
CREATE POLICY blueprint_stewardship_metadata_event_data_read
    ON ple_data.blueprint_metadata_event FOR SELECT TO ple_data_owner USING (true);

ALTER TABLE ple_data.blueprint_course_star ENABLE ROW LEVEL SECURITY;

ALTER TABLE ple_data.blueprint_course_star FORCE ROW LEVEL SECURITY;

ALTER TABLE ple_data.blueprint_course_watch ENABLE ROW LEVEL SECURITY;

ALTER TABLE ple_data.blueprint_course_watch FORCE ROW LEVEL SECURITY;

CREATE POLICY blueprint_course_star_data_owner_access
    ON ple_data.blueprint_course_star FOR ALL TO ple_data_owner
    USING (true) WITH CHECK (true);

CREATE POLICY blueprint_course_watch_data_owner_access
    ON ple_data.blueprint_course_watch FOR ALL TO ple_data_owner
    USING (true) WITH CHECK (true);

SET LOCAL ROLE ple_private_owner;

CREATE POLICY account_blueprint_watch_notification_data_read ON ple_private.account
    FOR SELECT TO ple_data_owner USING (true);

CREATE POLICY account_state_blueprint_watch_notification_data_read
    ON ple_private.account_state_event FOR SELECT TO ple_data_owner USING (true);

ALTER TABLE ple_private.blueprint_course_watch_notification ENABLE ROW LEVEL SECURITY;

ALTER TABLE ple_private.blueprint_course_watch_notification FORCE ROW LEVEL SECURITY;

CREATE POLICY blueprint_course_watch_notification_private_owner_access
    ON ple_private.blueprint_course_watch_notification FOR ALL TO ple_private_owner
    USING (true) WITH CHECK (true);

CREATE POLICY blueprint_course_watch_notification_data_materialization_insert
    ON ple_private.blueprint_course_watch_notification FOR INSERT TO ple_data_owner
    WITH CHECK (true);

CREATE POLICY blueprint_course_watch_notification_data_read
    ON ple_private.blueprint_course_watch_notification FOR SELECT TO ple_data_owner
    USING (true);

