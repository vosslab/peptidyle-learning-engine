-- Row security policies from assessment_attempt_interaction.sql.

SET LOCAL ROLE ple_private_owner;

ALTER TABLE ple_private.question_attempt ENABLE ROW LEVEL SECURITY;

ALTER TABLE ple_private.question_attempt FORCE ROW LEVEL SECURITY;

ALTER TABLE ple_private.assessment_attempt_saved_response ENABLE ROW LEVEL SECURITY;

ALTER TABLE ple_private.assessment_attempt_saved_response FORCE ROW LEVEL SECURITY;

ALTER TABLE ple_private.question_response ENABLE ROW LEVEL SECURITY;

ALTER TABLE ple_private.question_response FORCE ROW LEVEL SECURITY;

ALTER TABLE ple_private.assessment_submission ENABLE ROW LEVEL SECURITY;

ALTER TABLE ple_private.assessment_submission FORCE ROW LEVEL SECURITY;

CREATE POLICY question_attempt_private_owner_access ON ple_private.question_attempt FOR ALL TO ple_private_owner USING (true) WITH CHECK (true);


-- Narrow API SECURITY DEFINER delivery routines resolve a Student-owned native
-- response through this evidence. The API owner has no login and ple_app
-- receives only procedure execution, so the authorization boundary remains
-- the procedure predicates in delivery.sql.
CREATE POLICY question_attempt_api_owner_delivery_read ON ple_private.question_attempt
    FOR SELECT TO ple_api_owner USING (true);

CREATE POLICY saved_response_private_owner_access ON ple_private.assessment_attempt_saved_response FOR ALL TO ple_private_owner USING (true) WITH CHECK (true);

CREATE POLICY question_response_private_owner_access ON ple_private.question_response FOR ALL TO ple_private_owner USING (true) WITH CHECK (true);

CREATE POLICY assessment_submission_private_owner_access ON ple_private.assessment_submission FOR ALL TO ple_private_owner USING (true) WITH CHECK (true);

