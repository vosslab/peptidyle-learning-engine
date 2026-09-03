-- Question Library stewardship and immutable evidence.

SET LOCAL ROLE ple_private_owner;
GRANT USAGE ON SCHEMA ple_private TO ple_data_owner;
GRANT REFERENCES ON TABLE ple_private.account TO ple_data_owner;
RESET ROLE;
SET LOCAL ROLE ple_data_owner;
CREATE TABLE ple_data.question_change_proposal (
    proposal_id uuid PRIMARY KEY,
    proposed_by_account_id uuid NOT NULL REFERENCES ple_private.account (account_id),
    created_at timestamp with time zone NOT NULL
);
CREATE TABLE ple_data.question_change_proposal_revision (
    proposal_revision_id uuid PRIMARY KEY,
    proposal_id uuid NOT NULL REFERENCES ple_data.question_change_proposal (proposal_id),
    revision_number integer NOT NULL CHECK (revision_number > 0),
    base_question_id text NOT NULL,
    base_revision_number integer NOT NULL CHECK (base_revision_number > 0),
    patch jsonb NOT NULL CHECK (jsonb_typeof(patch) = 'object'),
    publication_validation jsonb NOT NULL CHECK (jsonb_typeof(publication_validation) = 'object'),
    semantic_impact jsonb NOT NULL CHECK (jsonb_typeof(semantic_impact) = 'object'),
    grading_impact jsonb NOT NULL CHECK (jsonb_typeof(grading_impact) = 'object'),
    created_at timestamp with time zone NOT NULL,
    CONSTRAINT question_change_proposal_revision_number_is_unique UNIQUE (proposal_id, revision_number),
    CONSTRAINT question_change_proposal_revision_reference_is_unique UNIQUE (proposal_id, proposal_revision_id),
    CONSTRAINT question_change_proposal_revision_base_revision_matches FOREIGN KEY (base_question_id, base_revision_number)
        REFERENCES ple_data.question_revision (question_id, revision_number)
);
CREATE FUNCTION ple_data.reject_question_stewardship_change()
RETURNS trigger LANGUAGE plpgsql SET search_path = pg_catalog, ple_data AS $$
BEGIN RAISE EXCEPTION USING ERRCODE = '23514', MESSAGE = 'Question stewardship evidence is immutable'; END
$$;
CREATE TRIGGER question_change_proposal_is_immutable BEFORE UPDATE OR DELETE ON ple_data.question_change_proposal FOR EACH ROW EXECUTE FUNCTION ple_data.reject_question_stewardship_change();
CREATE TRIGGER question_change_proposal_revision_is_immutable BEFORE UPDATE OR DELETE ON ple_data.question_change_proposal_revision FOR EACH ROW EXECUTE FUNCTION ple_data.reject_question_stewardship_change();
ALTER TABLE ple_data.question_change_proposal ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_data.question_change_proposal FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_data.question_change_proposal_revision ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_data.question_change_proposal_revision FORCE ROW LEVEL SECURITY;
REVOKE ALL PRIVILEGES ON TABLE ple_data.question_change_proposal, ple_data.question_change_proposal_revision FROM PUBLIC;
REVOKE ALL PRIVILEGES ON FUNCTION ple_data.reject_question_stewardship_change() FROM PUBLIC;
RESET ROLE;
