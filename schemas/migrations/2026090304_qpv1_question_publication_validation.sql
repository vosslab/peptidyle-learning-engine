-- Exact Question Publication Validation ownership for proposal evidence.

SET LOCAL ROLE ple_data_owner;

ALTER TABLE ple_data.question_change_proposal_revision
    RENAME COLUMN publication_validation TO question_publication_validation;

ALTER TABLE ple_data.question_change_proposal_revision
    RENAME CONSTRAINT question_change_proposal_revision_publication_validation_check
    TO question_change_proposal_question_publication_validation_check;

COMMENT ON COLUMN ple_data.question_change_proposal_revision.question_publication_validation IS
    'Calculated Question Publication Validation for this immutable Question Change Proposal Revision.';

RESET ROLE;
