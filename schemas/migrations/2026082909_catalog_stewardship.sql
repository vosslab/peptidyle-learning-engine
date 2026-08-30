-- SD1 shared-catalog improvement proposals and immutable stewardship evidence.

SET LOCAL ROLE ple_private_owner;
GRANT USAGE ON SCHEMA ple_private TO ple_data_owner;
GRANT REFERENCES ON TABLE ple_private.account TO ple_data_owner;
RESET ROLE;
SET LOCAL ROLE ple_data_owner;
CREATE TABLE ple_data.question_change_proposal (
    proposal_id uuid PRIMARY KEY,
    base_problem_id uuid NOT NULL REFERENCES ple_data.published_question_version (problem_id),
    proposed_by_account_id uuid NOT NULL REFERENCES ple_private.account (account_id),
    submitted_at timestamp with time zone NOT NULL,
    patch jsonb NOT NULL,
    CONSTRAINT question_change_proposal_patch_is_object CHECK (jsonb_typeof(patch) = 'object')
);
CREATE TABLE ple_data.question_stewardship_event (
    event_id uuid PRIMARY KEY,
    proposal_id uuid REFERENCES ple_data.question_change_proposal (proposal_id),
    event_kind text NOT NULL CHECK (event_kind IN ('proposal_submitted', 'proposal_accepted', 'proposal_rejected', 'forced_correction')),
    account_id uuid NOT NULL REFERENCES ple_private.account (account_id),
    occurred_at timestamp with time zone NOT NULL,
    evidence jsonb NOT NULL,
    CONSTRAINT question_stewardship_evidence_is_object CHECK (jsonb_typeof(evidence) = 'object')
);
CREATE FUNCTION ple_data.reject_question_stewardship_change()
RETURNS trigger LANGUAGE plpgsql SET search_path = pg_catalog, ple_data AS $$
BEGIN RAISE EXCEPTION USING ERRCODE = '23514', MESSAGE = 'catalog stewardship evidence is immutable'; END
$$;
CREATE TRIGGER question_change_proposal_is_immutable BEFORE UPDATE OR DELETE ON ple_data.question_change_proposal FOR EACH ROW EXECUTE FUNCTION ple_data.reject_question_stewardship_change();
CREATE TRIGGER question_stewardship_event_is_immutable BEFORE UPDATE OR DELETE ON ple_data.question_stewardship_event FOR EACH ROW EXECUTE FUNCTION ple_data.reject_question_stewardship_change();
ALTER TABLE ple_data.question_change_proposal ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_data.question_change_proposal FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_data.question_stewardship_event ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_data.question_stewardship_event FORCE ROW LEVEL SECURITY;
REVOKE ALL PRIVILEGES ON TABLE ple_data.question_change_proposal, ple_data.question_stewardship_event FROM PUBLIC;
REVOKE ALL PRIVILEGES ON FUNCTION ple_data.reject_question_stewardship_change() FROM PUBLIC;
RESET ROLE;
