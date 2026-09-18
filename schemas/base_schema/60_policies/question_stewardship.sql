-- Row security policies from question_stewardship.sql.

SET LOCAL ROLE ple_private_owner;

CREATE POLICY account_question_stewardship_data_read ON ple_private.account
    FOR SELECT TO ple_data_owner USING (true);

CREATE POLICY account_state_question_stewardship_data_read ON ple_private.account_state_event
    FOR SELECT TO ple_data_owner USING (true);

SET LOCAL ROLE ple_data_owner;

ALTER TABLE ple_data.question_revision_acceptance ENABLE ROW LEVEL SECURITY;

ALTER TABLE ple_data.question_revision_acceptance FORCE ROW LEVEL SECURITY;

ALTER TABLE ple_data.question_revision_authorship ENABLE ROW LEVEL SECURITY;

ALTER TABLE ple_data.question_revision_authorship FORCE ROW LEVEL SECURITY;

ALTER TABLE ple_data.question_revision_license ENABLE ROW LEVEL SECURITY;

ALTER TABLE ple_data.question_revision_license FORCE ROW LEVEL SECURITY;

ALTER TABLE ple_data.question_revision_citation ENABLE ROW LEVEL SECURITY;

ALTER TABLE ple_data.question_revision_citation FORCE ROW LEVEL SECURITY;

ALTER TABLE ple_data.question_ownership_event ENABLE ROW LEVEL SECURITY;

ALTER TABLE ple_data.question_ownership_event FORCE ROW LEVEL SECURITY;

ALTER TABLE ple_data.question_fork_source ENABLE ROW LEVEL SECURITY;

ALTER TABLE ple_data.question_fork_source FORCE ROW LEVEL SECURITY;

ALTER TABLE ple_data.question_star ENABLE ROW LEVEL SECURITY;

ALTER TABLE ple_data.question_star FORCE ROW LEVEL SECURITY;

ALTER TABLE ple_data.question_watch ENABLE ROW LEVEL SECURITY;

ALTER TABLE ple_data.question_watch FORCE ROW LEVEL SECURITY;

CREATE POLICY question_stewardship_data_owner_access ON ple_data.question_revision_acceptance
    FOR ALL TO ple_data_owner USING (true) WITH CHECK (true);

CREATE POLICY question_authorship_data_owner_access ON ple_data.question_revision_authorship
    FOR ALL TO ple_data_owner USING (true) WITH CHECK (true);

CREATE POLICY question_license_data_owner_access ON ple_data.question_revision_license
    FOR ALL TO ple_data_owner USING (true) WITH CHECK (true);

CREATE POLICY question_citation_data_owner_access ON ple_data.question_revision_citation
    FOR ALL TO ple_data_owner USING (true) WITH CHECK (true);

CREATE POLICY question_ownership_data_owner_access ON ple_data.question_ownership_event
    FOR ALL TO ple_data_owner USING (true) WITH CHECK (true);

CREATE POLICY question_fork_source_data_owner_access ON ple_data.question_fork_source
    FOR ALL TO ple_data_owner USING (true) WITH CHECK (true);

CREATE POLICY question_star_data_owner_access ON ple_data.question_star
    FOR ALL TO ple_data_owner USING (true) WITH CHECK (true);

CREATE POLICY question_watch_data_owner_access ON ple_data.question_watch
    FOR ALL TO ple_data_owner USING (true) WITH CHECK (true);

CREATE POLICY question_revision_acceptance_private_publication_insert
    ON ple_data.question_revision_acceptance FOR INSERT TO ple_private_owner WITH CHECK (true);

CREATE POLICY question_revision_authorship_private_publication_insert
    ON ple_data.question_revision_authorship FOR INSERT TO ple_private_owner WITH CHECK (true);

CREATE POLICY question_revision_license_private_publication_insert
    ON ple_data.question_revision_license FOR INSERT TO ple_private_owner WITH CHECK (true);

CREATE POLICY question_ownership_event_private_publication_insert
    ON ple_data.question_ownership_event FOR INSERT TO ple_private_owner WITH CHECK (true);

CREATE POLICY question_fork_source_private_publication_insert
    ON ple_data.question_fork_source FOR INSERT TO ple_private_owner WITH CHECK (true);

CREATE POLICY question_revision_acceptance_private_publication_read
    ON ple_data.question_revision_acceptance FOR SELECT TO ple_private_owner USING (true);

CREATE POLICY question_revision_authorship_private_publication_read
    ON ple_data.question_revision_authorship FOR SELECT TO ple_private_owner USING (true);

CREATE POLICY question_revision_license_private_publication_read
    ON ple_data.question_revision_license FOR SELECT TO ple_private_owner USING (true);

CREATE POLICY question_ownership_event_private_publication_read
    ON ple_data.question_ownership_event FOR SELECT TO ple_private_owner USING (true);

CREATE POLICY question_revision_acceptance_api_summary_read
    ON ple_data.question_revision_acceptance FOR SELECT TO ple_api_owner USING (true);

