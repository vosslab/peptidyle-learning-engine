-- Row security policies from jobs.sql.

SET LOCAL ROLE ple_private_owner;

ALTER TABLE ple_private.job ENABLE ROW LEVEL SECURITY;

ALTER TABLE ple_private.job FORCE ROW LEVEL SECURITY;

CREATE POLICY job_private_owner_access ON ple_private.job
    FOR ALL TO ple_private_owner USING (true) WITH CHECK (true);

