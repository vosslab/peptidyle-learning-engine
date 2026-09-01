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
    ADD COLUMN attempt_number integer NOT NULL CHECK (attempt_number > 0),
    ADD COLUMN question_pool_reuse_rule text NOT NULL CHECK (
        question_pool_reuse_rule IN ('reuse_selection', 'select_again')
    ),
    ADD COLUMN question_variation_rule text NOT NULL CHECK (
        question_variation_rule IN ('reuse_variation', 'new_variation')
    ),
    ADD CONSTRAINT assignment_attempt_student_assignment_number_is_unique
        UNIQUE (student_record_id, assignment_id, attempt_number);

CREATE TABLE ple_private.question_pool_selection (
    question_pool_selection_id uuid PRIMARY KEY,
    assignment_attempt_id uuid NOT NULL REFERENCES ple_private.assignment_attempt (assignment_attempt_id),
    assignment_entry_id uuid NOT NULL,
    created_at timestamp with time zone NOT NULL,
    selected_question_count integer NOT NULL CHECK (selected_question_count > 0),
    reused_from_question_pool_selection_id uuid
        REFERENCES ple_private.question_pool_selection (question_pool_selection_id),
    UNIQUE (question_pool_selection_id, assignment_attempt_id, assignment_entry_id),
    UNIQUE (assignment_attempt_id, assignment_entry_id)
);

CREATE TABLE ple_private.question_pool_selected_item (
    question_pool_selection_id uuid NOT NULL REFERENCES ple_private.question_pool_selection (question_pool_selection_id),
    question_pool_item_id uuid NOT NULL,
    selection_position integer NOT NULL CHECK (selection_position >= 0),
    question_id text NOT NULL,
    revision_number integer NOT NULL,
    PRIMARY KEY (question_pool_selection_id, selection_position),
    UNIQUE (question_pool_selection_id, question_pool_item_id),
    UNIQUE (question_pool_selection_id, question_pool_item_id, question_id, revision_number),
    CONSTRAINT question_pool_selected_item_version_matches FOREIGN KEY (question_id, revision_number)
        REFERENCES ple_data.question_revision (question_id, revision_number)
);

ALTER TABLE ple_private.issued_question
    ADD COLUMN question_pool_selection_id uuid,
    ADD COLUMN question_pool_item_id uuid,
    ADD CONSTRAINT issued_question_pool_membership_is_complete CHECK (
        (question_pool_selection_id IS NULL AND question_pool_item_id IS NULL)
        OR (question_pool_selection_id IS NOT NULL AND question_pool_item_id IS NOT NULL)
    ),
    ADD CONSTRAINT issued_question_selection_matches_attempt_and_entry
        FOREIGN KEY (question_pool_selection_id, assignment_attempt_id, assignment_entry_id)
        REFERENCES ple_private.question_pool_selection (
            question_pool_selection_id,
            assignment_attempt_id,
            assignment_entry_id
        ),
    ADD CONSTRAINT issued_question_selection_entry_matches_version
        FOREIGN KEY (
            question_pool_selection_id,
            question_pool_item_id,
            question_id,
            revision_number
        ) REFERENCES ple_private.question_pool_selected_item (
            question_pool_selection_id,
            question_pool_item_id,
            question_id,
            revision_number
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
CREATE TRIGGER question_pool_selected_item_is_immutable
BEFORE UPDATE OR DELETE ON ple_private.question_pool_selected_item
FOR EACH ROW EXECUTE FUNCTION ple_private.reject_question_pool_selection_change();

CREATE FUNCTION ple_private.validate_question_pool_selection_reuse()
RETURNS trigger LANGUAGE plpgsql SET search_path = pg_catalog, ple_private AS $$
BEGIN
    IF NEW.reused_from_question_pool_selection_id IS NOT NULL
       AND NOT EXISTS (
           SELECT 1
           FROM ple_private.question_pool_selection AS earlier_selection
           JOIN ple_private.assignment_attempt AS earlier_attempt
             ON earlier_attempt.assignment_attempt_id = earlier_selection.assignment_attempt_id
           JOIN ple_private.assignment_attempt AS current_attempt
             ON current_attempt.assignment_attempt_id = NEW.assignment_attempt_id
           WHERE earlier_selection.question_pool_selection_id =
                   NEW.reused_from_question_pool_selection_id
             AND earlier_selection.assignment_entry_id = NEW.assignment_entry_id
             AND earlier_attempt.student_record_id = current_attempt.student_record_id
             AND earlier_attempt.assignment_id = current_attempt.assignment_id
             AND earlier_attempt.attempt_number < current_attempt.attempt_number
       ) THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'reused Question Pool Selection must belong to an earlier Assignment Attempt for the same Student and Assignment Entry';
    END IF;
    RETURN NEW;
END
$$;

CREATE TRIGGER question_pool_selection_reuse_has_exact_student_and_assignment_history
BEFORE INSERT ON ple_private.question_pool_selection
FOR EACH ROW EXECUTE FUNCTION ple_private.validate_question_pool_selection_reuse();

CREATE FUNCTION ple_private.validate_question_pool_selected_item_count()
RETURNS trigger LANGUAGE plpgsql SET search_path = pg_catalog, ple_private AS $$
DECLARE
    target_selection_id uuid := COALESCE(
        NEW.question_pool_selection_id, OLD.question_pool_selection_id
    );
BEGIN
    IF EXISTS (
        SELECT 1
        FROM ple_private.question_pool_selection AS selection
        WHERE selection.question_pool_selection_id = target_selection_id
          AND selection.selected_question_count <> (
              SELECT count(*)::integer
              FROM ple_private.question_pool_selected_item AS item
              WHERE item.question_pool_selection_id = target_selection_id
          )
    ) THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Question Pool Selection must retain its exact selected Item count';
    END IF;
    RETURN NULL;
END
$$;

CREATE CONSTRAINT TRIGGER question_pool_selection_has_exact_entry_count
AFTER INSERT ON ple_private.question_pool_selection
DEFERRABLE INITIALLY DEFERRED
FOR EACH ROW EXECUTE FUNCTION ple_private.validate_question_pool_selected_item_count();
CREATE CONSTRAINT TRIGGER question_pool_selected_item_count_matches_selection
AFTER INSERT OR UPDATE OR DELETE ON ple_private.question_pool_selected_item
DEFERRABLE INITIALLY DEFERRED
FOR EACH ROW EXECUTE FUNCTION ple_private.validate_question_pool_selected_item_count();

ALTER TABLE ple_private.question_pool_selection ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.question_pool_selection FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_private.question_pool_selected_item ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.question_pool_selected_item FORCE ROW LEVEL SECURITY;
REVOKE ALL PRIVILEGES ON TABLE ple_private.question_pool_selection,
    ple_private.question_pool_selected_item FROM PUBLIC;
REVOKE ALL PRIVILEGES ON FUNCTION ple_private.reject_question_pool_selection_change() FROM PUBLIC;
REVOKE ALL PRIVILEGES ON FUNCTION ple_private.validate_question_pool_selection_reuse() FROM PUBLIC;
REVOKE ALL PRIVILEGES ON FUNCTION ple_private.validate_question_pool_selected_item_count() FROM PUBLIC;

COMMENT ON TABLE ple_private.question_pool_selection IS
    'Immutable selected Question Pool Item set for one Assignment Attempt and Assignment Entry.';
COMMENT ON COLUMN ple_private.question_pool_selection.reused_from_question_pool_selection_id IS
    'Earlier same-Student Assignment Attempt Selection whose exact Items this Selection retained.';
COMMENT ON COLUMN ple_private.question_pool_selection.selected_question_count IS
    'Exact number of selected Item rows, checked at transaction commit.';
COMMENT ON TABLE ple_private.question_pool_selected_item IS
    'Exact selected Question Pool Item and Question Revision in immutable delivery order.';

RESET ROLE;
