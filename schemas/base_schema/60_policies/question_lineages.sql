-- Row security policies from question_lineages.sql.

SET LOCAL ROLE ple_data_owner;

ALTER TABLE ple_data.published_question ENABLE ROW LEVEL SECURITY;

ALTER TABLE ple_data.published_question FORCE ROW LEVEL SECURITY;

ALTER TABLE ple_data.question_revision ENABLE ROW LEVEL SECURITY;

ALTER TABLE ple_data.question_revision FORCE ROW LEVEL SECURITY;

ALTER TABLE ple_data.published_question_metadata ENABLE ROW LEVEL SECURITY;

ALTER TABLE ple_data.published_question_metadata FORCE ROW LEVEL SECURITY;

ALTER TABLE ple_data.question_publication_event ENABLE ROW LEVEL SECURITY;

ALTER TABLE ple_data.question_publication_event FORCE ROW LEVEL SECURITY;

ALTER TABLE ple_data.question_availability_event ENABLE ROW LEVEL SECURITY;

ALTER TABLE ple_data.question_availability_event FORCE ROW LEVEL SECURITY;

CREATE POLICY published_question_data_owner_access ON ple_data.published_question
    FOR ALL TO ple_data_owner USING (true) WITH CHECK (true);

CREATE POLICY question_revision_data_owner_access ON ple_data.question_revision
    FOR ALL TO ple_data_owner USING (true) WITH CHECK (true);

CREATE POLICY published_question_metadata_data_owner_access ON ple_data.published_question_metadata
    FOR ALL TO ple_data_owner USING (true) WITH CHECK (true);

CREATE POLICY question_publication_event_data_owner_access ON ple_data.question_publication_event
    FOR ALL TO ple_data_owner USING (true) WITH CHECK (true);

CREATE POLICY question_availability_event_data_owner_access ON ple_data.question_availability_event
    FOR ALL TO ple_data_owner USING (true) WITH CHECK (true);

CREATE POLICY published_question_private_publication_insert ON ple_data.published_question
    FOR INSERT TO ple_private_owner WITH CHECK (true);


-- Successor publication serializes its immutable revision number by locking
-- the stable lineage.  This is deliberately read-only: only the dedicated
-- availability transition owned by ple_data_owner changes lineage state.
CREATE POLICY published_question_private_publication_read ON ple_data.published_question
    FOR SELECT TO ple_private_owner USING (true);

CREATE POLICY published_question_private_publication_lock ON ple_data.published_question
    FOR UPDATE TO ple_private_owner USING (true) WITH CHECK (false);

CREATE POLICY published_question_metadata_private_publication_insert
    ON ple_data.published_question_metadata FOR INSERT TO ple_private_owner WITH CHECK (true);

CREATE POLICY published_question_metadata_private_publication_read
    ON ple_data.published_question_metadata FOR SELECT TO ple_private_owner USING (true);

CREATE POLICY published_question_metadata_private_publication_update
    ON ple_data.published_question_metadata FOR UPDATE TO ple_private_owner
    USING (true) WITH CHECK (true);

CREATE POLICY question_revision_private_publication_insert ON ple_data.question_revision
    FOR INSERT TO ple_private_owner WITH CHECK (true);

CREATE POLICY question_revision_private_publication_read ON ple_data.question_revision
    FOR SELECT TO ple_private_owner USING (true);

CREATE POLICY question_publication_event_private_publication_insert
    ON ple_data.question_publication_event FOR INSERT TO ple_private_owner WITH CHECK (true);

CREATE POLICY question_availability_event_private_publication_insert
    ON ple_data.question_availability_event FOR INSERT TO ple_private_owner WITH CHECK (true);

CREATE POLICY published_question_api_summary_read ON ple_data.published_question
    FOR SELECT TO ple_api_owner USING (true);

CREATE POLICY published_question_metadata_api_summary_read ON ple_data.published_question_metadata
    FOR SELECT TO ple_api_owner USING (true);

CREATE POLICY question_revision_api_summary_read ON ple_data.question_revision
    FOR SELECT TO ple_api_owner USING (true);

