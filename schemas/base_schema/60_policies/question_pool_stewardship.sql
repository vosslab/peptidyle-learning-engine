-- Row security policies from question_pool_stewardship.sql.

SET LOCAL ROLE ple_data_owner;

-- Stable Question Pool Stars and actor-private Watches, across immutable Revisions.
-- ASVS 8.2.1/8.3.1: no caller-supplied Account or Watch directory capability.
CREATE POLICY question_pool_stewardship_data_read ON ple_data.question_pool
    FOR SELECT TO ple_data_owner USING (true);

ALTER TABLE ple_data.question_pool_star ENABLE ROW LEVEL SECURITY;

ALTER TABLE ple_data.question_pool_star FORCE ROW LEVEL SECURITY;

CREATE POLICY question_pool_star_data_owner_access ON ple_data.question_pool_star
    FOR ALL TO ple_data_owner USING (true) WITH CHECK (true);

ALTER TABLE ple_data.question_pool_watch ENABLE ROW LEVEL SECURITY;

ALTER TABLE ple_data.question_pool_watch FORCE ROW LEVEL SECURITY;

CREATE POLICY question_pool_watch_data_owner_access ON ple_data.question_pool_watch
    FOR ALL TO ple_data_owner USING (true) WITH CHECK (true);

