-- Immutable released Assignment Revision entry and Question Pool Item snapshots.
--
-- Assignment JSON remains an authoring transport. Student Work
-- derives only from these relational, immutable released-content facts.

SET LOCAL ROLE ple_data_owner;

CREATE TABLE ple_data.assignment_revision_entry (
    assignment_revision_id uuid NOT NULL,
    assignment_entry_id uuid NOT NULL,
    assignment_content_entry_index integer NOT NULL CHECK (assignment_content_entry_index >= 0),
    entry_kind text NOT NULL CHECK (entry_kind IN ('fixed_question', 'question_pool')),
    availability text NOT NULL CHECK (availability IN ('available', 'retired')),
    scoring_rule text NOT NULL CHECK (
        scoring_rule IN ('normal', 'full_credit', 'extra_credit', 'excluded')
    ),
    point_value numeric NOT NULL CHECK (point_value >= 0),
    question_attempt_limit integer CHECK (question_attempt_limit > 0),
    question_attempt_time_limit_seconds integer CHECK (question_attempt_time_limit_seconds > 0),
    question_attempt_time_limit_grace_seconds integer CHECK (
        question_attempt_time_limit_grace_seconds >= 0
    ),
    CHECK (
        (question_attempt_time_limit_seconds IS NULL)
        = (question_attempt_time_limit_grace_seconds IS NULL)
    ),
    PRIMARY KEY (assignment_revision_id, assignment_entry_id),
    UNIQUE (assignment_revision_id, assignment_content_entry_index),
    CONSTRAINT assignment_revision_entry_revision_matches
        FOREIGN KEY (assignment_revision_id)
        REFERENCES ple_data.assignment_revision (assignment_revision_id)
);

CREATE TABLE ple_data.assignment_revision_fixed_question (
    assignment_revision_id uuid NOT NULL,
    assignment_entry_id uuid NOT NULL,
    question_id text NOT NULL,
    revision_number integer NOT NULL,
    PRIMARY KEY (assignment_revision_id, assignment_entry_id),
    CONSTRAINT assignment_revision_fixed_question_entry_matches
        FOREIGN KEY (assignment_revision_id, assignment_entry_id)
        REFERENCES ple_data.assignment_revision_entry (assignment_revision_id, assignment_entry_id),
    CONSTRAINT assignment_revision_fixed_question_revision_matches
        FOREIGN KEY (question_id, revision_number)
        REFERENCES ple_data.question_revision (question_id, revision_number)
);

CREATE TABLE ple_data.assignment_revision_question_pool (
    assignment_revision_id uuid NOT NULL,
    assignment_entry_id uuid NOT NULL,
    selection_count integer NOT NULL CHECK (selection_count > 0),
    selected_question_order text NOT NULL CHECK (
        selected_question_order IN ('question_pool_order', 'random_order')
    ),
    PRIMARY KEY (assignment_revision_id, assignment_entry_id),
    CONSTRAINT assignment_revision_question_pool_assignment_entry_matches
        FOREIGN KEY (assignment_revision_id, assignment_entry_id)
        REFERENCES ple_data.assignment_revision_entry (assignment_revision_id, assignment_entry_id)
);

CREATE TABLE ple_data.assignment_revision_question_pool_item (
    assignment_revision_id uuid NOT NULL,
    assignment_entry_id uuid NOT NULL,
    question_pool_item_id uuid NOT NULL,
    question_pool_item_index integer NOT NULL CHECK (question_pool_item_index >= 0),
    question_id text NOT NULL,
    revision_number integer NOT NULL,
    availability text NOT NULL CHECK (availability IN ('available', 'retired')),
    PRIMARY KEY (
        assignment_revision_id, assignment_entry_id, question_pool_item_id
    ),
    UNIQUE (assignment_revision_id, assignment_entry_id, question_pool_item_index),
    CONSTRAINT assignment_revision_question_pool_item_pool_matches
        FOREIGN KEY (assignment_revision_id, assignment_entry_id)
        REFERENCES ple_data.assignment_revision_question_pool (
            assignment_revision_id, assignment_entry_id
        ),
    CONSTRAINT assignment_revision_question_pool_item_version_matches
        FOREIGN KEY (question_id, revision_number)
        REFERENCES ple_data.question_revision (question_id, revision_number)
);

CREATE FUNCTION ple_data.reject_assignment_revision_entry_change()
RETURNS trigger LANGUAGE plpgsql SET search_path = pg_catalog, ple_data AS $$
BEGIN
    RAISE EXCEPTION USING ERRCODE = '23514',
        MESSAGE = 'released Assignment Revision entries are immutable';
END
$$;

CREATE FUNCTION ple_data.validate_assignment_revision_entry_shape()
RETURNS trigger LANGUAGE plpgsql SET search_path = pg_catalog, ple_data AS $$
DECLARE
    target_revision_id uuid := COALESCE(NEW.assignment_revision_id, OLD.assignment_revision_id);
    target_entry_id uuid := COALESCE(NEW.assignment_entry_id, OLD.assignment_entry_id);
    target_entry_kind text;
    fixed_question_count integer;
    question_pool_count integer;
    question_pool_item_count integer;
    required_question_pool_item_count integer;
BEGIN
    SELECT entry.entry_kind
      INTO target_entry_kind
      FROM ple_data.assignment_revision_entry AS entry
     WHERE entry.assignment_revision_id = target_revision_id
       AND entry.assignment_entry_id = target_entry_id;
    IF target_entry_kind IS NULL THEN
        RETURN NULL;
    END IF;

    SELECT count(*)::integer
      INTO fixed_question_count
      FROM ple_data.assignment_revision_fixed_question AS fixed_question
     WHERE fixed_question.assignment_revision_id = target_revision_id
       AND fixed_question.assignment_entry_id = target_entry_id;
    SELECT count(*)::integer
      INTO question_pool_count
      FROM ple_data.assignment_revision_question_pool AS question_pool
     WHERE question_pool.assignment_revision_id = target_revision_id
       AND question_pool.assignment_entry_id = target_entry_id;
    SELECT count(*)::integer
      INTO question_pool_item_count
      FROM ple_data.assignment_revision_question_pool_item AS item
     WHERE item.assignment_revision_id = target_revision_id
       AND item.assignment_entry_id = target_entry_id;

    IF target_entry_kind = 'fixed_question'
       AND (
           fixed_question_count <> 1
           OR question_pool_count <> 0
           OR question_pool_item_count <> 0
       ) THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'a Fixed Question Assignment Entry requires exactly one fixed Question pin';
    END IF;
    IF target_entry_kind = 'question_pool' THEN
        SELECT question_pool.selection_count
          INTO required_question_pool_item_count
          FROM ple_data.assignment_revision_question_pool AS question_pool
         WHERE question_pool.assignment_revision_id = target_revision_id
           AND question_pool.assignment_entry_id = target_entry_id;
        IF question_pool_count <> 1
           OR fixed_question_count <> 0
           OR question_pool_item_count < COALESCE(required_question_pool_item_count, 1) THEN
            RAISE EXCEPTION USING ERRCODE = '23514',
                MESSAGE = 'a Question Pool Assignment Entry requires enough exact Question Pool Items';
        END IF;
    END IF;
    RETURN NULL;
END
$$;

CREATE TRIGGER assignment_revision_entry_is_immutable
BEFORE UPDATE OR DELETE ON ple_data.assignment_revision_entry
FOR EACH ROW EXECUTE FUNCTION ple_data.reject_assignment_revision_entry_change();
CREATE TRIGGER assignment_revision_fixed_question_is_immutable
BEFORE UPDATE OR DELETE ON ple_data.assignment_revision_fixed_question
FOR EACH ROW EXECUTE FUNCTION ple_data.reject_assignment_revision_entry_change();
CREATE TRIGGER assignment_revision_question_pool_is_immutable
BEFORE UPDATE OR DELETE ON ple_data.assignment_revision_question_pool
FOR EACH ROW EXECUTE FUNCTION ple_data.reject_assignment_revision_entry_change();
CREATE TRIGGER assignment_revision_question_pool_item_is_immutable
BEFORE UPDATE OR DELETE ON ple_data.assignment_revision_question_pool_item
FOR EACH ROW EXECUTE FUNCTION ple_data.reject_assignment_revision_entry_change();

CREATE CONSTRAINT TRIGGER assignment_revision_entry_has_exact_shape
AFTER INSERT ON ple_data.assignment_revision_entry
DEFERRABLE INITIALLY DEFERRED
FOR EACH ROW EXECUTE FUNCTION ple_data.validate_assignment_revision_entry_shape();
CREATE CONSTRAINT TRIGGER assignment_revision_fixed_question_matches_entry_kind
AFTER INSERT OR UPDATE OR DELETE ON ple_data.assignment_revision_fixed_question
DEFERRABLE INITIALLY DEFERRED
FOR EACH ROW EXECUTE FUNCTION ple_data.validate_assignment_revision_entry_shape();
CREATE CONSTRAINT TRIGGER assignment_revision_question_pool_matches_entry_kind
AFTER INSERT OR UPDATE OR DELETE ON ple_data.assignment_revision_question_pool
DEFERRABLE INITIALLY DEFERRED
FOR EACH ROW EXECUTE FUNCTION ple_data.validate_assignment_revision_entry_shape();
CREATE CONSTRAINT TRIGGER assignment_revision_question_pool_item_count_is_sufficient
AFTER INSERT OR UPDATE OR DELETE ON ple_data.assignment_revision_question_pool_item
DEFERRABLE INITIALLY DEFERRED
FOR EACH ROW EXECUTE FUNCTION ple_data.validate_assignment_revision_entry_shape();

ALTER TABLE ple_data.assignment_revision_entry ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_data.assignment_revision_entry FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_data.assignment_revision_fixed_question ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_data.assignment_revision_fixed_question FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_data.assignment_revision_question_pool ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_data.assignment_revision_question_pool FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_data.assignment_revision_question_pool_item ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_data.assignment_revision_question_pool_item FORCE ROW LEVEL SECURITY;
CREATE POLICY assignment_revision_entry_data_owner_access
    ON ple_data.assignment_revision_entry FOR ALL TO ple_data_owner USING (true) WITH CHECK (true);
CREATE POLICY assignment_revision_fixed_question_data_owner_access
    ON ple_data.assignment_revision_fixed_question FOR ALL TO ple_data_owner USING (true) WITH CHECK (true);
CREATE POLICY assignment_revision_question_pool_data_owner_access
    ON ple_data.assignment_revision_question_pool FOR ALL TO ple_data_owner USING (true) WITH CHECK (true);
CREATE POLICY assignment_revision_question_pool_item_data_owner_access
    ON ple_data.assignment_revision_question_pool_item FOR ALL TO ple_data_owner USING (true) WITH CHECK (true);
CREATE POLICY assignment_revision_entry_private_lookup
    ON ple_data.assignment_revision_entry FOR SELECT TO ple_private_owner USING (true);
CREATE POLICY assignment_revision_fixed_question_private_lookup
    ON ple_data.assignment_revision_fixed_question FOR SELECT TO ple_private_owner USING (true);
CREATE POLICY assignment_revision_question_pool_private_lookup
    ON ple_data.assignment_revision_question_pool FOR SELECT TO ple_private_owner USING (true);
CREATE POLICY assignment_revision_question_pool_item_private_lookup
    ON ple_data.assignment_revision_question_pool_item FOR SELECT TO ple_private_owner USING (true);
GRANT USAGE ON SCHEMA ple_data TO ple_private_owner;
GRANT SELECT ON TABLE ple_data.assignment_revision_entry,
    ple_data.assignment_revision_fixed_question,
    ple_data.assignment_revision_question_pool,
    ple_data.assignment_revision_question_pool_item TO ple_private_owner;
REVOKE ALL PRIVILEGES ON TABLE ple_data.assignment_revision_entry,
    ple_data.assignment_revision_fixed_question,
    ple_data.assignment_revision_question_pool,
    ple_data.assignment_revision_question_pool_item FROM PUBLIC;
REVOKE ALL PRIVILEGES ON FUNCTION ple_data.reject_assignment_revision_entry_change() FROM PUBLIC;
REVOKE ALL PRIVILEGES ON FUNCTION ple_data.validate_assignment_revision_entry_shape() FROM PUBLIC;
COMMENT ON TABLE ple_data.assignment_revision_entry IS
    'Immutable ordered Assignment Entry snapshot for one released Assignment Revision, including Question Attempt controls.';
COMMENT ON TABLE ple_data.assignment_revision_fixed_question IS
    'Exact Published Question Revision pin for one fixed Assignment Entry.';
COMMENT ON TABLE ple_data.assignment_revision_question_pool IS
    'Released Question Pool selection rule for one Assignment Entry.';
COMMENT ON TABLE ple_data.assignment_revision_question_pool_item IS
    'Exact eligible Question Pool Item and Published Question Revision for one released Assignment Entry.';

RESET ROLE;

SET LOCAL ROLE ple_private_owner;

CREATE POLICY assignment_attempt_private_owner_access
    ON ple_private.assignment_attempt FOR ALL TO ple_private_owner USING (true) WITH CHECK (true);
CREATE POLICY question_pool_selection_private_owner_access
    ON ple_private.question_pool_selection FOR ALL TO ple_private_owner USING (true) WITH CHECK (true);
CREATE POLICY question_pool_selected_item_private_owner_access
    ON ple_private.question_pool_selected_item FOR ALL TO ple_private_owner USING (true) WITH CHECK (true);
CREATE POLICY issued_question_private_owner_access
    ON ple_private.issued_question FOR ALL TO ple_private_owner USING (true) WITH CHECK (true);

CREATE FUNCTION ple_private.validate_question_pool_selection_entry()
RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_private, ple_data AS $$
BEGIN
    IF NOT EXISTS (
        SELECT 1
        FROM ple_private.assignment_attempt AS attempt
        JOIN ple_data.assignment_revision_entry AS entry
          ON entry.assignment_revision_id = attempt.assignment_revision_id
         AND entry.assignment_entry_id = NEW.assignment_entry_id
         AND entry.entry_kind = 'question_pool'
        WHERE attempt.assignment_attempt_id = NEW.assignment_attempt_id
    ) THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Question Pool Selection must name a Question Pool Assignment Entry in its exact Released Assignment Revision';
    END IF;
    RETURN NEW;
END
$$;

CREATE FUNCTION ple_private.validate_question_pool_selected_item_source()
RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_private, ple_data AS $$
DECLARE
    reused_from_selection_id uuid;
BEGIN
    SELECT selection.reused_from_question_pool_selection_id
      INTO reused_from_selection_id
      FROM ple_private.question_pool_selection AS selection
     WHERE selection.question_pool_selection_id = NEW.question_pool_selection_id;
    IF reused_from_selection_id IS NOT NULL THEN
        IF NOT EXISTS (
            SELECT 1
            FROM ple_private.question_pool_selected_item AS earlier_item
            WHERE earlier_item.question_pool_selection_id = reused_from_selection_id
              AND earlier_item.selection_position = NEW.selection_position
              AND earlier_item.question_pool_item_id = NEW.question_pool_item_id
              AND earlier_item.question_id = NEW.question_id
              AND earlier_item.revision_number = NEW.revision_number
        ) THEN
            RAISE EXCEPTION USING ERRCODE = '23514',
                MESSAGE = 'reused Question Pool Selection must retain each earlier selected Item in its exact order';
        END IF;
        RETURN NEW;
    END IF;

    IF NOT EXISTS (
        SELECT 1
        FROM ple_private.question_pool_selection AS selection
        JOIN ple_private.assignment_attempt AS attempt
          ON attempt.assignment_attempt_id = selection.assignment_attempt_id
        JOIN ple_data.assignment_revision_question_pool_item AS item
          ON item.assignment_revision_id = attempt.assignment_revision_id
         AND item.assignment_entry_id = selection.assignment_entry_id
         AND item.question_pool_item_id = NEW.question_pool_item_id
         AND item.question_id = NEW.question_id
         AND item.revision_number = NEW.revision_number
         AND item.availability = 'available'
        WHERE selection.question_pool_selection_id = NEW.question_pool_selection_id
    ) THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Question Pool Selection Item must be an available exact Item in its Released Assignment Revision';
    END IF;
    RETURN NEW;
END
$$;

CREATE FUNCTION ple_private.validate_issued_question_source()
RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_private, ple_data AS $$
BEGIN
    IF NEW.question_pool_selection_id IS NULL THEN
        IF NOT EXISTS (
            SELECT 1
            FROM ple_private.assignment_attempt AS attempt
            JOIN ple_data.assignment_revision_entry AS entry
              ON entry.assignment_revision_id = attempt.assignment_revision_id
             AND entry.assignment_entry_id = NEW.assignment_entry_id
             AND entry.entry_kind = 'fixed_question'
             AND entry.point_value = NEW.point_value
             AND entry.scoring_rule = NEW.scoring_rule
            JOIN ple_data.assignment_revision_fixed_question AS fixed_question
              ON fixed_question.assignment_revision_id = entry.assignment_revision_id
             AND fixed_question.assignment_entry_id = entry.assignment_entry_id
             AND fixed_question.question_id = NEW.question_id
             AND fixed_question.revision_number = NEW.revision_number
            WHERE attempt.assignment_attempt_id = NEW.assignment_attempt_id
        ) THEN
            RAISE EXCEPTION USING ERRCODE = '23514',
                MESSAGE = 'fixed Issued Question must match its exact Released Assignment Entry';
        END IF;
        RETURN NEW;
    END IF;

    IF NOT EXISTS (
        SELECT 1
        FROM ple_private.question_pool_selection AS selection
        JOIN ple_private.assignment_attempt AS attempt
          ON attempt.assignment_attempt_id = selection.assignment_attempt_id
        JOIN ple_data.assignment_revision_entry AS entry
          ON entry.assignment_revision_id = attempt.assignment_revision_id
         AND entry.assignment_entry_id = selection.assignment_entry_id
         AND entry.entry_kind = 'question_pool'
         AND entry.point_value = NEW.point_value
         AND entry.scoring_rule = NEW.scoring_rule
        WHERE selection.question_pool_selection_id = NEW.question_pool_selection_id
          AND selection.assignment_attempt_id = NEW.assignment_attempt_id
          AND selection.assignment_entry_id = NEW.assignment_entry_id
    ) THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'pooled Issued Question must match its exact Released Assignment Entry';
    END IF;
    RETURN NEW;
END
$$;

CREATE TRIGGER question_pool_selection_matches_released_pool_assignment_entry
BEFORE INSERT ON ple_private.question_pool_selection
FOR EACH ROW EXECUTE FUNCTION ple_private.validate_question_pool_selection_entry();
CREATE TRIGGER question_pool_selected_item_matches_released_pool_item
BEFORE INSERT ON ple_private.question_pool_selected_item
FOR EACH ROW EXECUTE FUNCTION ple_private.validate_question_pool_selected_item_source();
CREATE TRIGGER issued_question_matches_released_assignment_entry
BEFORE INSERT ON ple_private.issued_question
FOR EACH ROW EXECUTE FUNCTION ple_private.validate_issued_question_source();

REVOKE ALL PRIVILEGES ON FUNCTION ple_private.validate_question_pool_selection_entry() FROM PUBLIC;
REVOKE ALL PRIVILEGES ON FUNCTION ple_private.validate_question_pool_selected_item_source() FROM PUBLIC;
REVOKE ALL PRIVILEGES ON FUNCTION ple_private.validate_issued_question_source() FROM PUBLIC;
COMMENT ON FUNCTION ple_private.validate_question_pool_selection_entry() IS
    'Requires each Selection to use a Question Pool Assignment Entry in the exact Attempt Revision.';
COMMENT ON FUNCTION ple_private.validate_question_pool_selected_item_source() IS
    'Requires a new Selection to use available Revision Items or an exact earlier Selection copy.';
COMMENT ON FUNCTION ple_private.validate_issued_question_source() IS
    'Requires every Issued Question to retain its exact released Entry, Question Revision, points, and scoring rule.';

RESET ROLE;
