-- Separate the independent later-Attempt Question Pool and Question Variation decisions.
--
-- PLE is pre-production. This is a deliberate contract cutover: the retired
-- combined rule has no data-preserving translation because it conflated two
-- independent Instructor choices.

SET LOCAL ROLE ple_data_owner;

ALTER TABLE ple_data.assignment_working_copy
    DROP COLUMN question_variation_rule,
    ADD COLUMN question_pool_reuse_rule text NOT NULL CHECK (
        question_pool_reuse_rule IN ('reuse_selection', 'select_again')
    ),
    ADD COLUMN question_variation_rule text NOT NULL CHECK (
        question_variation_rule IN ('reuse_variation', 'new_variation')
    );

ALTER TABLE ple_data.assignment_revision
    DROP COLUMN question_variation_rule,
    ADD COLUMN question_pool_reuse_rule text NOT NULL CHECK (
        question_pool_reuse_rule IN ('reuse_selection', 'select_again')
    ),
    ADD COLUMN question_variation_rule text NOT NULL CHECK (
        question_variation_rule IN ('reuse_variation', 'new_variation')
    );

COMMENT ON COLUMN ple_data.assignment_working_copy.question_pool_reuse_rule IS
    'Instructor decision for whether a later Assignment Attempt reuses its Question Pool Selection.';
COMMENT ON COLUMN ple_data.assignment_working_copy.question_variation_rule IS
    'Instructor decision for whether a later Assignment Attempt reuses each Question Variation.';
COMMENT ON COLUMN ple_data.assignment_revision.question_pool_reuse_rule IS
    'Released snapshot of the Question Pool Reuse Rule.';
COMMENT ON COLUMN ple_data.assignment_revision.question_variation_rule IS
    'Released snapshot of the Question Variation Rule.';

RESET ROLE;

SET LOCAL ROLE ple_private_owner;

ALTER TABLE ple_private.assignment_attempt
    ADD COLUMN question_pool_reuse_rule text NOT NULL CHECK (
        question_pool_reuse_rule IN ('reuse_selection', 'select_again')
    ),
    ADD COLUMN question_variation_rule text NOT NULL CHECK (
        question_variation_rule IN ('reuse_variation', 'new_variation')
    );

CREATE TABLE ple_private.question_pool_selection (
    question_pool_selection_id uuid PRIMARY KEY,
    assignment_attempt_id uuid NOT NULL REFERENCES ple_private.assignment_attempt (assignment_attempt_id),
    assignment_entry_id uuid NOT NULL,
    created_at timestamp with time zone NOT NULL,
    UNIQUE (question_pool_selection_id, assignment_attempt_id, assignment_entry_id),
    UNIQUE (assignment_attempt_id, assignment_entry_id)
);

CREATE TABLE ple_private.question_pool_selected_candidate (
    question_pool_selection_id uuid NOT NULL REFERENCES ple_private.question_pool_selection (question_pool_selection_id),
    question_pool_candidate_id uuid NOT NULL,
    selection_position integer NOT NULL CHECK (selection_position >= 0),
    question_id text NOT NULL,
    version_number integer NOT NULL,
    PRIMARY KEY (question_pool_selection_id, selection_position),
    UNIQUE (question_pool_selection_id, question_pool_candidate_id),
    UNIQUE (question_pool_selection_id, question_pool_candidate_id, question_id, version_number),
    CONSTRAINT question_pool_selected_candidate_version_matches FOREIGN KEY (question_id, version_number)
        REFERENCES ple_data.published_question_version (question_id, version_number)
);

ALTER TABLE ple_private.issued_question
    ADD COLUMN question_pool_selection_id uuid,
    ADD COLUMN question_pool_candidate_id uuid,
    ADD CONSTRAINT issued_question_pool_membership_is_complete CHECK (
        (question_pool_selection_id IS NULL AND question_pool_candidate_id IS NULL)
        OR (question_pool_selection_id IS NOT NULL AND question_pool_candidate_id IS NOT NULL)
    ),
    ADD CONSTRAINT issued_question_selection_matches_attempt_and_entry
        FOREIGN KEY (question_pool_selection_id, assignment_attempt_id, assignment_entry_id)
        REFERENCES ple_private.question_pool_selection (
            question_pool_selection_id,
            assignment_attempt_id,
            assignment_entry_id
        ),
    ADD CONSTRAINT issued_question_selection_candidate_matches_version
        FOREIGN KEY (
            question_pool_selection_id,
            question_pool_candidate_id,
            question_id,
            version_number
        ) REFERENCES ple_private.question_pool_selected_candidate (
            question_pool_selection_id,
            question_pool_candidate_id,
            question_id,
            version_number
        );

CREATE FUNCTION ple_private.reject_question_pool_selection_change()
RETURNS trigger LANGUAGE plpgsql SET search_path = pg_catalog, ple_private AS $$
BEGIN
    RAISE EXCEPTION USING ERRCODE = '23514', MESSAGE = 'Question Pool Selection is immutable';
END
$$;

CREATE TRIGGER question_pool_selection_is_immutable
BEFORE UPDATE OR DELETE ON ple_private.question_pool_selection
FOR EACH ROW EXECUTE FUNCTION ple_private.reject_question_pool_selection_change();
CREATE TRIGGER question_pool_selected_candidate_is_immutable
BEFORE UPDATE OR DELETE ON ple_private.question_pool_selected_candidate
FOR EACH ROW EXECUTE FUNCTION ple_private.reject_question_pool_selection_change();

ALTER TABLE ple_private.question_pool_selection ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.question_pool_selection FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_private.question_pool_selected_candidate ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.question_pool_selected_candidate FORCE ROW LEVEL SECURITY;
REVOKE ALL PRIVILEGES ON TABLE ple_private.question_pool_selection,
    ple_private.question_pool_selected_candidate FROM PUBLIC;
REVOKE ALL PRIVILEGES ON FUNCTION ple_private.reject_question_pool_selection_change() FROM PUBLIC;

COMMENT ON TABLE ple_private.question_pool_selection IS
    'Immutable selected Question Pool candidate set for one Assignment Attempt and Assignment Entry.';
COMMENT ON TABLE ple_private.question_pool_selected_candidate IS
    'Exact selected Question Pool candidate and Question Revision in immutable delivery order.';

RESET ROLE;
