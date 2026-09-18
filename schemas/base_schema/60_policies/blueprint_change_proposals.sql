-- Row security policies from blueprint_change_proposals.sql.

SET LOCAL ROLE ple_data_owner;

ALTER TABLE ple_data.blueprint_change_proposal ENABLE ROW LEVEL SECURITY;

ALTER TABLE ple_data.blueprint_change_proposal FORCE ROW LEVEL SECURITY;

CREATE POLICY blueprint_change_proposal_api_owner
    ON ple_data.blueprint_change_proposal TO ple_api_owner
    USING (true) WITH CHECK (true);

ALTER TABLE ple_data.blueprint_change_proposal_acceptance ENABLE ROW LEVEL SECURITY;

ALTER TABLE ple_data.blueprint_change_proposal_acceptance FORCE ROW LEVEL SECURITY;

CREATE POLICY blueprint_change_proposal_acceptance_api_owner
    ON ple_data.blueprint_change_proposal_acceptance TO ple_api_owner
    USING (true) WITH CHECK (true);

