-- Current Course Assignment aggregates.  These relations describe the content
-- used for future Attempts; student_work.sql retains the facts used to
-- interpret an Attempt after a later Assignment edit.

SET LOCAL ROLE ple_data_owner;

CREATE TABLE ple_data.assignment (
    assignment_id uuid PRIMARY KEY,
    course_id uuid NOT NULL REFERENCES ple_data.course_instance(course_id),
    reference_number bigint GENERATED ALWAYS AS IDENTITY UNIQUE NOT NULL
        CHECK (reference_number BETWEEN 1 AND 2147483647),
    source_blueprint_course_reference_number bigint NOT NULL,
    source_blueprint_revision_number bigint NOT NULL CHECK (source_blueprint_revision_number > 0),
    -- BlueprintAssignmentSource: an exact immutable Blueprint Revision plus
    -- the stable Assignment member selected from that Revision.
    source_blueprint_assignment_reference uuid NOT NULL,
    created_at timestamptz NOT NULL,
    updated_at timestamptz NOT NULL,
    assignment_edit_number bigint NOT NULL DEFAULT 1 CHECK (assignment_edit_number > 0),
    assignment_title text NOT NULL CHECK (
        assignment_title ~ '[^[:space:]]' AND char_length(assignment_title) <= 200
    ),
    assignment_instructions text NOT NULL CHECK (
        assignment_instructions !~ E'\\x00' AND char_length(assignment_instructions) <= 50000
    ),
    available_at timestamptz,
    due_at timestamptz,
    closes_at timestamptz,
    assignment_attempt_time_limit_seconds integer CHECK (
        assignment_attempt_time_limit_seconds IS NULL
        OR assignment_attempt_time_limit_seconds > 0
    ),
    attempt_limit integer CHECK (attempt_limit IS NULL OR attempt_limit > 0),
    late_work_rule text NOT NULL CHECK (late_work_rule IN ('accept', 'mark_late', 'reject')),
    assignment_completion_rule text NOT NULL CHECK (
        assignment_completion_rule IN ('answer_all', 'all_correct', 'score_at_least')
    ),
    assignment_completion_score_threshold numeric CHECK (
        (assignment_completion_rule = 'score_at_least'
            AND assignment_completion_score_threshold > 0
            AND assignment_completion_score_threshold <= 1)
        OR (assignment_completion_rule <> 'score_at_least'
            AND assignment_completion_score_threshold IS NULL)
    ),
    assignment_attempt_grade_rule text NOT NULL CHECK (
        assignment_attempt_grade_rule IN ('first', 'latest', 'highest', 'instructor_selected')
    ),
    assignment_attempt_continuation_rule text NOT NULL CHECK (
        assignment_attempt_continuation_rule IN ('unlimited', 'capped', 'closed')
    ),
    max_additional_assignment_attempts integer CHECK (
        (assignment_attempt_continuation_rule = 'capped'
            AND max_additional_assignment_attempts >= 0)
        OR (assignment_attempt_continuation_rule <> 'capped'
            AND max_additional_assignment_attempts IS NULL)
    ),
    question_pool_reuse_rule text NOT NULL CHECK (
        question_pool_reuse_rule IN ('reuse_selection', 'select_again')
    ),
    question_variation_rule text NOT NULL CHECK (
        question_variation_rule IN ('reuse_variation', 'new_variation')
    ),
    assignment_attempt_resume_rule text NOT NULL CHECK (
        assignment_attempt_resume_rule IN ('resumable', 'single_session')
    ),
    assignment_question_display_rule text NOT NULL CHECK (
        assignment_question_display_rule IN ('all_questions', 'one_question_at_a_time')
    ),
    assignment_navigation_rule text NOT NULL CHECK (
        assignment_navigation_rule IN ('free_navigation', 'forward_only')
    ),
    assignment_question_order_rule text NOT NULL CHECK (
        assignment_question_order_rule IN ('authored_order', 'shuffled')
    ),
    feedback_score text NOT NULL CHECK (
        feedback_score IN ('during_attempt', 'after_submit', 'after_due', 'after_close', 'never')
    ),
    feedback_per_item_correctness text NOT NULL CHECK (
        feedback_per_item_correctness IN ('during_attempt', 'after_submit', 'after_due', 'after_close', 'never')
    ),
    feedback_submitted_response text NOT NULL CHECK (
        feedback_submitted_response IN ('during_attempt', 'after_submit', 'after_due', 'after_close', 'never')
    ),
    feedback_question_feedback text NOT NULL CHECK (
        feedback_question_feedback IN ('during_attempt', 'after_submit', 'after_due', 'after_close', 'never')
    ),
    feedback_question_answer text NOT NULL CHECK (
        feedback_question_answer IN ('during_attempt', 'after_submit', 'after_due', 'after_close', 'never')
    ),
    feedback_question_answer_explanation text NOT NULL CHECK (
        feedback_question_answer_explanation IN ('during_attempt', 'after_submit', 'after_due', 'after_close', 'never')
    ),
    feedback_class_statistics text NOT NULL CHECK (
        feedback_class_statistics IN ('during_attempt', 'after_submit', 'after_due', 'after_close', 'never')
    ),
    assignment_status text NOT NULL DEFAULT 'unreleased' CHECK (
        assignment_status IN ('unreleased', 'released', 'closed', 'archived')
    ),
    UNIQUE (course_id, reference_number),
    UNIQUE (assignment_id, course_id),
    FOREIGN KEY (
        source_blueprint_course_reference_number,
        source_blueprint_revision_number,
        source_blueprint_assignment_reference
    ) REFERENCES ple_data.blueprint_revision_assignment (
        blueprint_course_reference_number,
        blueprint_revision_number,
        blueprint_assignment_reference
    ),
    CHECK (
        (available_at IS NULL OR due_at IS NULL OR available_at <= due_at)
        AND (due_at IS NULL OR closes_at IS NULL OR due_at <= closes_at)
    ),
    CHECK (updated_at >= created_at)
);

CREATE TABLE ple_data.assignment_entry (
    assignment_entry_id uuid PRIMARY KEY,
    assignment_id uuid NOT NULL REFERENCES ple_data.assignment(assignment_id),
    authored_position integer NOT NULL CHECK (authored_position >= 0),
    entry_kind text NOT NULL CHECK (entry_kind IN ('fixed_question', 'question_pool')),
    availability text NOT NULL DEFAULT 'available' CHECK (availability IN ('available', 'retired')),
    scoring_rule text NOT NULL CHECK (scoring_rule IN ('normal', 'full_credit', 'extra_credit', 'excluded')),
    question_id text,
    question_revision_number integer,
    points_possible numeric,
    selection_count integer,
    points_per_item numeric,
    selected_question_order text,
    question_attempt_limit integer,
    question_attempt_time_limit_seconds integer,
    question_attempt_grace_seconds integer,
    UNIQUE (assignment_id, authored_position),
    UNIQUE (assignment_entry_id, assignment_id),
    FOREIGN KEY (question_id, question_revision_number)
        REFERENCES ple_data.question_revision(question_id, revision_number),
    CHECK (
        (entry_kind = 'fixed_question'
            AND question_id IS NOT NULL
            AND question_revision_number IS NOT NULL
            AND points_possible > 0
            AND selection_count IS NULL
            AND points_per_item IS NULL
            AND selected_question_order IS NULL)
        OR (entry_kind = 'question_pool'
            AND question_id IS NULL
            AND question_revision_number IS NULL
            AND points_possible IS NULL
            AND selection_count > 0
            AND points_per_item > 0
            AND selected_question_order IN ('question_pool_order', 'random_order'))
    ),
    CHECK (question_attempt_limit IS NULL OR question_attempt_limit > 0),
    CHECK (
        (question_attempt_time_limit_seconds IS NULL AND question_attempt_grace_seconds IS NULL)
        OR (question_attempt_time_limit_seconds > 0
            AND question_attempt_grace_seconds >= 0)
    )
);

CREATE TABLE ple_data.question_pool_item (
    question_pool_item_id uuid PRIMARY KEY,
    assignment_entry_id uuid NOT NULL,
    assignment_id uuid NOT NULL,
    item_position integer NOT NULL CHECK (item_position >= 0),
    question_id text NOT NULL,
    question_revision_number integer NOT NULL,
    availability text NOT NULL DEFAULT 'available' CHECK (availability IN ('available', 'retired')),
    UNIQUE (assignment_entry_id, item_position),
    UNIQUE (assignment_entry_id, question_id, question_revision_number),
    FOREIGN KEY (assignment_entry_id, assignment_id)
        REFERENCES ple_data.assignment_entry(assignment_entry_id, assignment_id),
    FOREIGN KEY (question_id, question_revision_number)
        REFERENCES ple_data.question_revision(question_id, revision_number)
);

CREATE FUNCTION ple_data.enforce_assignment_edit()
RETURNS trigger LANGUAGE plpgsql
SET search_path = pg_catalog, ple_data AS $$
BEGIN
    IF NEW.assignment_id <> OLD.assignment_id
       OR NEW.course_id <> OLD.course_id
       OR NEW.reference_number <> OLD.reference_number
       OR NEW.source_blueprint_course_reference_number <> OLD.source_blueprint_course_reference_number
       OR NEW.source_blueprint_revision_number <> OLD.source_blueprint_revision_number
       OR NEW.source_blueprint_assignment_reference <> OLD.source_blueprint_assignment_reference
       OR NEW.created_at <> OLD.created_at
       OR NEW.updated_at < OLD.updated_at THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Assignment identity is immutable and timestamps move forward';
    END IF;

    -- The guarded save operation changes the parent exactly once after it has
    -- accepted the complete normalized child candidate.  Child rows are not
    -- independently writable by a runtime role.
    IF NEW.assignment_edit_number <> OLD.assignment_edit_number
       AND NEW.assignment_edit_number <> OLD.assignment_edit_number + 1 THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Assignment changes advance exactly one Assignment Edit Number';
    END IF;
    RETURN NEW;
END
$$;

-- ASVS 2.2-2.3, 8.2-8.3, and 15.4: only the guarded current-content save
-- changes normalized Assignment Entries or Question Pool Items.  It receives
-- a closed JSON array because entries are a tagged union.  Each member has:
--
-- fixed_question: assignmentEntryId, availability, scoringRule, questionId,
-- revisionNumber, pointsPossible, questionAttemptLimit,
-- questionAttemptTimeLimitSeconds, questionAttemptGraceSeconds.
-- question_pool: assignmentEntryId, availability, scoringRule,
-- selectionCount, pointsPerItem, selectedQuestionOrder, items[].  An item has
-- questionPoolItemId, itemPosition, questionId, revisionNumber, availability.
--
-- Callers retain an ID for an unchanged member.  This permits an archived
-- exact pin to remain in a current Assignment while requiring any new or
-- replaced pin to come from an Available Question lineage.
CREATE FUNCTION ple_data.replace_assignment_entries(
    p_assignment_id uuid,
    p_entries jsonb
) RETURNS boolean
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data AS $$
DECLARE
    entry_json jsonb;
    item_json jsonb;
    entry_id uuid;
    item_id uuid;
    entry_kind text;
    entry_ids uuid[] := ARRAY[]::uuid[];
    item_ids uuid[] := ARRAY[]::uuid[];
    changed boolean := false;
    row_count integer;
    question_available boolean;
BEGIN
    IF p_entries IS NULL OR jsonb_typeof(p_entries) <> 'array'
       OR jsonb_array_length(p_entries) > 1024 THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Assignment Entries are invalid';
    END IF;

    FOR entry_json IN SELECT value FROM jsonb_array_elements(p_entries) LOOP
        IF jsonb_typeof(entry_json) <> 'object'
           OR entry_json ->> 'assignmentEntryId' !~* '^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$'
           OR entry_json ->> 'kind' NOT IN ('fixed_question', 'question_pool')
           OR entry_json ->> 'availability' NOT IN ('available', 'retired')
           OR entry_json ->> 'scoringRule' NOT IN ('normal', 'full_credit', 'extra_credit', 'excluded') THEN
            RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Assignment Entry is invalid';
        END IF;
        entry_id := (entry_json ->> 'assignmentEntryId')::uuid;
        IF entry_id = ANY (entry_ids) THEN
            RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Assignment Entries repeat an identity';
        END IF;
        entry_ids := array_append(entry_ids, entry_id);
        entry_kind := entry_json ->> 'kind';

        IF entry_kind = 'fixed_question' THEN
            IF entry_json ->> 'questionId' IS NULL
               OR entry_json ->> 'revisionNumber' !~ '^[1-9][0-9]*$'
               OR entry_json ->> 'pointsPossible' !~ '^[0-9]+(\.[0-9]+)?$' THEN
                RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Fixed Question Assignment Entry is invalid';
            END IF;
            SELECT question.availability = 'available' INTO question_available
              FROM ple_data.published_question AS question
              JOIN ple_data.question_revision AS revision
                ON revision.question_id = question.question_id
             WHERE question.question_id = entry_json ->> 'questionId'
               AND revision.revision_number = (entry_json ->> 'revisionNumber')::integer;
            IF question_available IS DISTINCT FROM true
               AND NOT EXISTS (
                   SELECT 1 FROM ple_data.assignment_entry AS existing
                    WHERE existing.assignment_id = p_assignment_id
                      AND existing.assignment_entry_id = entry_id
                      AND existing.entry_kind = 'fixed_question'
                      AND existing.question_id = entry_json ->> 'questionId'
                      AND existing.question_revision_number = (entry_json ->> 'revisionNumber')::integer
               ) THEN
                RAISE EXCEPTION USING ERRCODE = '22023',
                    MESSAGE = 'New Assignment Question pins require an Available Question';
            END IF;
            UPDATE ple_data.assignment_entry AS target
               SET authored_position = COALESCE((entry_json ->> 'authoredPosition')::integer, 0),
                   entry_kind = 'fixed_question', availability = entry_json ->> 'availability',
                   scoring_rule = entry_json ->> 'scoringRule',
                   question_id = entry_json ->> 'questionId',
                   question_revision_number = (entry_json ->> 'revisionNumber')::integer,
                   points_possible = (entry_json ->> 'pointsPossible')::numeric,
                   selection_count = NULL, points_per_item = NULL, selected_question_order = NULL,
                   question_attempt_limit = NULLIF(entry_json ->> 'questionAttemptLimit', '')::integer,
                   question_attempt_time_limit_seconds = NULLIF(entry_json ->> 'questionAttemptTimeLimitSeconds', '')::integer,
                   question_attempt_grace_seconds = NULLIF(entry_json ->> 'questionAttemptGraceSeconds', '')::integer
             WHERE target.assignment_id = p_assignment_id
               AND target.assignment_entry_id = entry_id
               AND ROW(target.authored_position, target.entry_kind, target.availability,
                       target.scoring_rule, target.question_id, target.question_revision_number,
                       target.points_possible, target.question_attempt_limit,
                       target.question_attempt_time_limit_seconds, target.question_attempt_grace_seconds)
                   IS DISTINCT FROM ROW(
                       COALESCE((entry_json ->> 'authoredPosition')::integer, 0), 'fixed_question',
                       entry_json ->> 'availability', entry_json ->> 'scoringRule',
                       entry_json ->> 'questionId', (entry_json ->> 'revisionNumber')::integer,
                       (entry_json ->> 'pointsPossible')::numeric,
                       NULLIF(entry_json ->> 'questionAttemptLimit', '')::integer,
                       NULLIF(entry_json ->> 'questionAttemptTimeLimitSeconds', '')::integer,
                       NULLIF(entry_json ->> 'questionAttemptGraceSeconds', '')::integer);
            GET DIAGNOSTICS row_count = ROW_COUNT;
            changed := changed OR row_count > 0;
            IF NOT EXISTS (
                SELECT 1 FROM ple_data.assignment_entry AS existing
                 WHERE existing.assignment_id = p_assignment_id
                   AND existing.assignment_entry_id = entry_id
            ) THEN
                INSERT INTO ple_data.assignment_entry(
                    assignment_entry_id, assignment_id, authored_position, entry_kind, availability,
                    scoring_rule, question_id, question_revision_number, points_possible,
                    question_attempt_limit, question_attempt_time_limit_seconds, question_attempt_grace_seconds
                ) VALUES (
                    entry_id, p_assignment_id, COALESCE((entry_json ->> 'authoredPosition')::integer, 0),
                    'fixed_question', entry_json ->> 'availability', entry_json ->> 'scoringRule',
                    entry_json ->> 'questionId', (entry_json ->> 'revisionNumber')::integer,
                    (entry_json ->> 'pointsPossible')::numeric,
                    NULLIF(entry_json ->> 'questionAttemptLimit', '')::integer,
                    NULLIF(entry_json ->> 'questionAttemptTimeLimitSeconds', '')::integer,
                    NULLIF(entry_json ->> 'questionAttemptGraceSeconds', '')::integer
                );
                changed := true;
            END IF;
        ELSE
            IF entry_json ->> 'selectionCount' !~ '^[1-9][0-9]*$'
               OR entry_json ->> 'pointsPerItem' !~ '^[0-9]+(\.[0-9]+)?$'
               OR entry_json ->> 'selectedQuestionOrder' NOT IN ('question_pool_order', 'random_order')
               OR jsonb_typeof(entry_json -> 'items') <> 'array'
               OR jsonb_array_length(entry_json -> 'items') > 1024 THEN
                RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Question Pool Assignment Entry is invalid';
            END IF;
            UPDATE ple_data.assignment_entry AS target
               SET authored_position = COALESCE((entry_json ->> 'authoredPosition')::integer, 0),
                   entry_kind = 'question_pool', availability = entry_json ->> 'availability',
                   scoring_rule = entry_json ->> 'scoringRule', question_id = NULL,
                   question_revision_number = NULL, points_possible = NULL,
                   selection_count = (entry_json ->> 'selectionCount')::integer,
                   points_per_item = (entry_json ->> 'pointsPerItem')::numeric,
                   selected_question_order = entry_json ->> 'selectedQuestionOrder',
                   question_attempt_limit = NULLIF(entry_json ->> 'questionAttemptLimit', '')::integer,
                   question_attempt_time_limit_seconds = NULLIF(entry_json ->> 'questionAttemptTimeLimitSeconds', '')::integer,
                   question_attempt_grace_seconds = NULLIF(entry_json ->> 'questionAttemptGraceSeconds', '')::integer
             WHERE target.assignment_id = p_assignment_id AND target.assignment_entry_id = entry_id
               AND ROW(target.authored_position, target.entry_kind, target.availability, target.scoring_rule,
                       target.selection_count, target.points_per_item, target.selected_question_order,
                       target.question_attempt_limit, target.question_attempt_time_limit_seconds,
                       target.question_attempt_grace_seconds) IS DISTINCT FROM ROW(
                       COALESCE((entry_json ->> 'authoredPosition')::integer, 0), 'question_pool',
                       entry_json ->> 'availability', entry_json ->> 'scoringRule',
                       (entry_json ->> 'selectionCount')::integer, (entry_json ->> 'pointsPerItem')::numeric,
                       entry_json ->> 'selectedQuestionOrder', NULLIF(entry_json ->> 'questionAttemptLimit', '')::integer,
                       NULLIF(entry_json ->> 'questionAttemptTimeLimitSeconds', '')::integer,
                       NULLIF(entry_json ->> 'questionAttemptGraceSeconds', '')::integer);
            GET DIAGNOSTICS row_count = ROW_COUNT;
            changed := changed OR row_count > 0;
            IF NOT EXISTS (
                SELECT 1 FROM ple_data.assignment_entry AS existing
                 WHERE existing.assignment_id = p_assignment_id
                   AND existing.assignment_entry_id = entry_id
            ) THEN
                INSERT INTO ple_data.assignment_entry(
                    assignment_entry_id, assignment_id, authored_position, entry_kind, availability,
                    scoring_rule, selection_count, points_per_item, selected_question_order,
                    question_attempt_limit, question_attempt_time_limit_seconds, question_attempt_grace_seconds
                ) VALUES (
                    entry_id, p_assignment_id, COALESCE((entry_json ->> 'authoredPosition')::integer, 0),
                    'question_pool', entry_json ->> 'availability', entry_json ->> 'scoringRule',
                    (entry_json ->> 'selectionCount')::integer, (entry_json ->> 'pointsPerItem')::numeric,
                    entry_json ->> 'selectedQuestionOrder', NULLIF(entry_json ->> 'questionAttemptLimit', '')::integer,
                    NULLIF(entry_json ->> 'questionAttemptTimeLimitSeconds', '')::integer,
                    NULLIF(entry_json ->> 'questionAttemptGraceSeconds', '')::integer
                );
                changed := true;
            END IF;
            FOR item_json IN SELECT value FROM jsonb_array_elements(entry_json -> 'items') LOOP
                IF jsonb_typeof(item_json) <> 'object'
                   OR item_json ->> 'questionPoolItemId' !~* '^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$'
                   OR item_json ->> 'itemPosition' !~ '^[0-9]+$'
                   OR item_json ->> 'revisionNumber' !~ '^[1-9][0-9]*$'
                   OR item_json ->> 'availability' NOT IN ('available', 'retired')
                   OR item_json ->> 'questionId' IS NULL THEN
                    RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Question Pool Item is invalid';
                END IF;
                item_id := (item_json ->> 'questionPoolItemId')::uuid;
                IF item_id = ANY (item_ids) THEN
                    RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Question Pool Items repeat an identity';
                END IF;
                item_ids := array_append(item_ids, item_id);
                SELECT question.availability = 'available' INTO question_available
                  FROM ple_data.published_question AS question JOIN ple_data.question_revision AS revision
                    ON revision.question_id = question.question_id
                 WHERE question.question_id = item_json ->> 'questionId'
                   AND revision.revision_number = (item_json ->> 'revisionNumber')::integer;
                IF question_available IS DISTINCT FROM true
                   AND NOT EXISTS (
                       SELECT 1 FROM ple_data.question_pool_item AS existing
                        WHERE existing.question_pool_item_id = item_id
                          AND existing.assignment_entry_id = entry_id
                          AND existing.question_id = item_json ->> 'questionId'
                          AND existing.question_revision_number = (item_json ->> 'revisionNumber')::integer
                   ) THEN
                    RAISE EXCEPTION USING ERRCODE = '22023',
                        MESSAGE = 'New Question Pool pins require an Available Question';
                END IF;
                IF EXISTS (SELECT 1 FROM ple_data.question_pool_item AS target
                    WHERE target.question_pool_item_id = item_id
                      AND (target.assignment_id <> p_assignment_id OR target.assignment_entry_id <> entry_id)) THEN
                    RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Question Pool Item belongs to another Assignment Entry';
                END IF;
                UPDATE ple_data.question_pool_item AS target
                   SET assignment_entry_id = entry_id, assignment_id = p_assignment_id,
                       item_position = (item_json ->> 'itemPosition')::integer,
                       question_id = item_json ->> 'questionId',
                       question_revision_number = (item_json ->> 'revisionNumber')::integer,
                       availability = item_json ->> 'availability'
                 WHERE target.question_pool_item_id = item_id
                   AND target.assignment_id = p_assignment_id AND target.assignment_entry_id = entry_id
                   AND ROW(target.assignment_entry_id, target.assignment_id, target.item_position,
                           target.question_id, target.question_revision_number, target.availability)
                       IS DISTINCT FROM ROW(entry_id, p_assignment_id, (item_json ->> 'itemPosition')::integer,
                           item_json ->> 'questionId', (item_json ->> 'revisionNumber')::integer,
                           item_json ->> 'availability');
                GET DIAGNOSTICS row_count = ROW_COUNT;
                changed := changed OR row_count > 0;
                IF NOT EXISTS (
                    SELECT 1 FROM ple_data.question_pool_item AS existing
                     WHERE existing.question_pool_item_id = item_id
                ) THEN
                    INSERT INTO ple_data.question_pool_item(
                        question_pool_item_id, assignment_entry_id, assignment_id, item_position,
                        question_id, question_revision_number, availability
                    ) VALUES (item_id, entry_id, p_assignment_id, (item_json ->> 'itemPosition')::integer,
                        item_json ->> 'questionId', (item_json ->> 'revisionNumber')::integer,
                        item_json ->> 'availability');
                    changed := true;
                END IF;
            END LOOP;
        END IF;
    END LOOP;
    UPDATE ple_data.question_pool_item SET availability = 'retired'
     WHERE assignment_id = p_assignment_id AND availability <> 'retired'
       AND NOT (question_pool_item_id = ANY (item_ids));
    GET DIAGNOSTICS row_count = ROW_COUNT;
    changed := changed OR row_count > 0;
    UPDATE ple_data.assignment_entry SET availability = 'retired'
     WHERE assignment_id = p_assignment_id AND availability <> 'retired'
       AND NOT (assignment_entry_id = ANY (entry_ids));
    GET DIAGNOSTICS row_count = ROW_COUNT;
    RETURN changed OR row_count > 0;
END
$$;

CREATE FUNCTION ple_data.validate_question_pool_item_parent()
RETURNS trigger LANGUAGE plpgsql
SET search_path = pg_catalog, ple_data AS $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM ple_data.assignment_entry
         WHERE assignment_entry_id = NEW.assignment_entry_id
           AND assignment_id = NEW.assignment_id
           AND entry_kind = 'question_pool'
    ) THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Question Pool Item requires its owning Question Pool Assignment Entry';
    END IF;
    RETURN NEW;
END
$$;

CREATE FUNCTION ple_data.validate_assignment_release(p_assignment_id uuid)
RETURNS void LANGUAGE plpgsql STABLE
SET search_path = pg_catalog, ple_data AS $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM ple_data.assignment_entry
         WHERE assignment_id = p_assignment_id AND availability = 'available'
    ) THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Assignment Release requires an available Assignment Entry';
    END IF;
    IF EXISTS (
        SELECT 1
          FROM ple_data.assignment_entry AS entry
         WHERE entry.assignment_id = p_assignment_id
           AND entry.availability = 'available'
           AND entry.entry_kind = 'question_pool'
           AND entry.selection_count > (
               SELECT count(*)
                 FROM ple_data.question_pool_item AS item
                WHERE item.assignment_entry_id = entry.assignment_entry_id
                  AND item.availability = 'available'
           )
    ) THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Question Pool Release requires enough available pinned Question Pool Items';
    END IF;
    IF EXISTS (
        SELECT 1 FROM ple_data.assignment
         WHERE assignment_id = p_assignment_id
           AND assignment_attempt_time_limit_seconds IS NULL
    ) THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Assignment Release requires a whole-Attempt time limit';
    END IF;
END
$$;

CREATE FUNCTION ple_data.create_assignment(
    p_assignment_id uuid,
    p_course_reference_number bigint,
    p_blueprint_assignment_reference uuid,
    p_title text,
    p_instructions text
) RETURNS TABLE (
    assignment_reference_number bigint, assignment_edit_number bigint,
    assignment_status text, assignment_title text, assignment_instructions text
)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data AS $$
DECLARE course_row ple_data.course_instance%ROWTYPE;
BEGIN
    IF p_assignment_id IS NULL OR p_blueprint_assignment_reference IS NULL
       OR p_course_reference_number NOT BETWEEN 1 AND 2147483647
       OR p_title IS NULL OR p_instructions IS NULL THEN
        RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Assignment creation is invalid';
    END IF;
    SELECT * INTO course_row FROM ple_data.course_instance
     WHERE reference_number = p_course_reference_number;
    IF NOT FOUND OR NOT ple_api.current_session_account_is_course_instructor(course_row.course_id) THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Assignment is unavailable';
    END IF;
    INSERT INTO ple_data.assignment AS inserted (
        assignment_id, course_id, source_blueprint_course_reference_number,
        source_blueprint_revision_number, source_blueprint_assignment_reference,
        created_at, updated_at, assignment_title, assignment_instructions,
        late_work_rule, assignment_completion_rule, assignment_attempt_grade_rule,
        assignment_attempt_continuation_rule, question_pool_reuse_rule, question_variation_rule,
        assignment_attempt_resume_rule, assignment_question_display_rule,
        assignment_navigation_rule, assignment_question_order_rule, feedback_score,
        feedback_per_item_correctness, feedback_submitted_response,
        feedback_question_feedback, feedback_question_answer,
        feedback_question_answer_explanation, feedback_class_statistics
    ) VALUES (
        p_assignment_id, course_row.course_id, course_row.blueprint_course_reference_number,
        course_row.blueprint_revision_number, p_blueprint_assignment_reference,
        clock_timestamp(), clock_timestamp(), p_title, p_instructions,
        'reject', 'answer_all', 'highest', 'unlimited', 'reuse_selection', 'new_variation',
        'resumable', 'one_question_at_a_time', 'free_navigation', 'shuffled',
        'after_submit', 'after_submit', 'after_submit', 'after_submit', 'after_submit',
        'after_submit', 'after_submit'
    ) RETURNING inserted.reference_number, inserted.assignment_edit_number,
        inserted.assignment_status, inserted.assignment_title, inserted.assignment_instructions
      INTO assignment_reference_number, assignment_edit_number, assignment_status,
           assignment_title, assignment_instructions;
    RETURN NEXT;
END
$$;

-- The mutable scalar values are an exact object with the Assignment table's
-- non-identity fields.  Keeping the tagged child collection separate avoids
-- generic snapshot persistence while one operation validates the full result.
CREATE FUNCTION ple_data.save_assignment(
    p_course_reference_number bigint,
    p_assignment_reference_number bigint,
    p_expected_edit_number bigint,
    p_values jsonb,
    p_entries jsonb
) RETURNS TABLE (
    assignment_reference_number bigint, assignment_edit_number bigint,
    assignment_status text, assignment_title text, assignment_instructions text
)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data AS $$
DECLARE
    current_assignment ple_data.assignment%ROWTYPE;
    candidate ple_data.assignment%ROWTYPE;
    entries_changed boolean;
    values_changed boolean;
    allowed_keys text[] := ARRAY[
        'assignment_title', 'assignment_instructions', 'available_at', 'due_at', 'closes_at',
        'assignment_attempt_time_limit_seconds', 'attempt_limit', 'late_work_rule',
        'assignment_completion_rule', 'assignment_completion_score_threshold',
        'assignment_attempt_grade_rule', 'assignment_attempt_continuation_rule',
        'max_additional_assignment_attempts', 'question_pool_reuse_rule', 'question_variation_rule',
        'assignment_attempt_resume_rule', 'assignment_question_display_rule',
        'assignment_navigation_rule', 'assignment_question_order_rule', 'feedback_score',
        'feedback_per_item_correctness', 'feedback_submitted_response',
        'feedback_question_feedback', 'feedback_question_answer',
        'feedback_question_answer_explanation', 'feedback_class_statistics'
    ];
BEGIN
    IF p_course_reference_number NOT BETWEEN 1 AND 2147483647
       OR p_assignment_reference_number NOT BETWEEN 1 AND 2147483647
       OR p_expected_edit_number IS NULL OR p_expected_edit_number <= 0
       OR p_values IS NULL OR jsonb_typeof(p_values) <> 'object'
       OR EXISTS (
           SELECT 1 FROM jsonb_object_keys(p_values) AS key WHERE key <> ALL (allowed_keys)
       ) OR EXISTS (
           SELECT 1 FROM unnest(allowed_keys) AS key WHERE NOT p_values ? key
       ) THEN
        RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Assignment save is invalid';
    END IF;
    SELECT assignment.* INTO current_assignment
      FROM ple_data.course_instance AS course
      JOIN ple_data.assignment AS assignment ON assignment.course_id = course.course_id
     WHERE course.reference_number = p_course_reference_number
       AND assignment.reference_number = p_assignment_reference_number
       AND ple_api.current_session_account_is_course_instructor(course.course_id)
     FOR UPDATE OF assignment;
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Assignment is unavailable';
    END IF;
    IF current_assignment.assignment_edit_number IS DISTINCT FROM p_expected_edit_number THEN
        RAISE EXCEPTION USING ERRCODE = '40001', MESSAGE = 'Assignment Edit Number is stale';
    END IF;
    SELECT * INTO candidate FROM jsonb_populate_record(current_assignment, p_values);
    values_changed := ROW(
        candidate.assignment_title, candidate.assignment_instructions, candidate.available_at,
        candidate.due_at, candidate.closes_at, candidate.assignment_attempt_time_limit_seconds,
        candidate.attempt_limit, candidate.late_work_rule, candidate.assignment_completion_rule,
        candidate.assignment_completion_score_threshold, candidate.assignment_attempt_grade_rule,
        candidate.assignment_attempt_continuation_rule, candidate.max_additional_assignment_attempts,
        candidate.question_pool_reuse_rule, candidate.question_variation_rule,
        candidate.assignment_attempt_resume_rule, candidate.assignment_question_display_rule,
        candidate.assignment_navigation_rule, candidate.assignment_question_order_rule,
        candidate.feedback_score, candidate.feedback_per_item_correctness,
        candidate.feedback_submitted_response, candidate.feedback_question_feedback,
        candidate.feedback_question_answer, candidate.feedback_question_answer_explanation,
        candidate.feedback_class_statistics
    ) IS DISTINCT FROM ROW(
        current_assignment.assignment_title, current_assignment.assignment_instructions,
        current_assignment.available_at, current_assignment.due_at, current_assignment.closes_at,
        current_assignment.assignment_attempt_time_limit_seconds, current_assignment.attempt_limit,
        current_assignment.late_work_rule, current_assignment.assignment_completion_rule,
        current_assignment.assignment_completion_score_threshold,
        current_assignment.assignment_attempt_grade_rule,
        current_assignment.assignment_attempt_continuation_rule,
        current_assignment.max_additional_assignment_attempts,
        current_assignment.question_pool_reuse_rule, current_assignment.question_variation_rule,
        current_assignment.assignment_attempt_resume_rule,
        current_assignment.assignment_question_display_rule,
        current_assignment.assignment_navigation_rule,
        current_assignment.assignment_question_order_rule, current_assignment.feedback_score,
        current_assignment.feedback_per_item_correctness,
        current_assignment.feedback_submitted_response,
        current_assignment.feedback_question_feedback,
        current_assignment.feedback_question_answer,
        current_assignment.feedback_question_answer_explanation,
        current_assignment.feedback_class_statistics
    );
    entries_changed := ple_data.replace_assignment_entries(current_assignment.assignment_id, p_entries);
    IF values_changed OR entries_changed THEN
        UPDATE ple_data.assignment AS updated SET
            assignment_title = candidate.assignment_title,
            assignment_instructions = candidate.assignment_instructions,
            available_at = candidate.available_at, due_at = candidate.due_at,
            closes_at = candidate.closes_at,
            assignment_attempt_time_limit_seconds = candidate.assignment_attempt_time_limit_seconds,
            attempt_limit = candidate.attempt_limit, late_work_rule = candidate.late_work_rule,
            assignment_completion_rule = candidate.assignment_completion_rule,
            assignment_completion_score_threshold = candidate.assignment_completion_score_threshold,
            assignment_attempt_grade_rule = candidate.assignment_attempt_grade_rule,
            assignment_attempt_continuation_rule = candidate.assignment_attempt_continuation_rule,
            max_additional_assignment_attempts = candidate.max_additional_assignment_attempts,
            question_pool_reuse_rule = candidate.question_pool_reuse_rule,
            question_variation_rule = candidate.question_variation_rule,
            assignment_attempt_resume_rule = candidate.assignment_attempt_resume_rule,
            assignment_question_display_rule = candidate.assignment_question_display_rule,
            assignment_navigation_rule = candidate.assignment_navigation_rule,
            assignment_question_order_rule = candidate.assignment_question_order_rule,
            feedback_score = candidate.feedback_score,
            feedback_per_item_correctness = candidate.feedback_per_item_correctness,
            feedback_submitted_response = candidate.feedback_submitted_response,
            feedback_question_feedback = candidate.feedback_question_feedback,
            feedback_question_answer = candidate.feedback_question_answer,
            feedback_question_answer_explanation = candidate.feedback_question_answer_explanation,
            feedback_class_statistics = candidate.feedback_class_statistics,
            assignment_edit_number = updated.assignment_edit_number + 1,
            updated_at = clock_timestamp()
         WHERE updated.assignment_id = current_assignment.assignment_id
        RETURNING updated.reference_number, updated.assignment_edit_number,
            updated.assignment_status, updated.assignment_title, updated.assignment_instructions
          INTO assignment_reference_number, assignment_edit_number, assignment_status,
               assignment_title, assignment_instructions;
        IF assignment_status = 'released' THEN
            PERFORM ple_data.validate_assignment_release(current_assignment.assignment_id);
        END IF;
    ELSE
        assignment_reference_number := current_assignment.reference_number;
        assignment_edit_number := current_assignment.assignment_edit_number;
        assignment_status := current_assignment.assignment_status;
        assignment_title := current_assignment.assignment_title;
        assignment_instructions := current_assignment.assignment_instructions;
    END IF;
    RETURN NEXT;
END
$$;

CREATE FUNCTION ple_data.save_assignment_inline(
    p_course_reference_number bigint, p_assignment_reference_number bigint,
    p_expected_edit_number bigint, p_title text, p_due_at timestamptz
) RETURNS TABLE (
    assignment_reference_number bigint, assignment_title text, due_at_millis bigint,
    assignment_status text, assignment_edit_number bigint
)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data AS $$
DECLARE assignment_row ple_data.assignment%ROWTYPE;
BEGIN
    IF p_course_reference_number NOT BETWEEN 1 AND 2147483647
       OR p_assignment_reference_number NOT BETWEEN 1 AND 2147483647
       OR p_expected_edit_number IS NULL OR p_expected_edit_number <= 0 OR p_title IS NULL THEN
        RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Assignment inline save is invalid';
    END IF;
    SELECT assignment.* INTO assignment_row FROM ple_data.course_instance AS course
      JOIN ple_data.assignment AS assignment ON assignment.course_id = course.course_id
     WHERE course.reference_number = p_course_reference_number
       AND assignment.reference_number = p_assignment_reference_number
       AND ple_api.current_session_account_is_course_instructor(course.course_id)
     FOR UPDATE OF assignment;
    IF NOT FOUND THEN RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Assignment is unavailable'; END IF;
    IF assignment_row.assignment_edit_number IS DISTINCT FROM p_expected_edit_number THEN
        RAISE EXCEPTION USING ERRCODE = '40001', MESSAGE = 'Assignment Edit Number is stale';
    END IF;
    IF ROW(assignment_row.assignment_title, assignment_row.due_at) IS DISTINCT FROM ROW(p_title, p_due_at) THEN
        UPDATE ple_data.assignment AS updated SET assignment_title = p_title, due_at = p_due_at,
            assignment_edit_number = updated.assignment_edit_number + 1, updated_at = clock_timestamp()
         WHERE updated.assignment_id = assignment_row.assignment_id
        RETURNING updated.reference_number, updated.assignment_title,
            CASE WHEN updated.due_at IS NULL THEN NULL
                 ELSE floor(extract(epoch FROM updated.due_at) * 1000)::bigint END,
            updated.assignment_status, updated.assignment_edit_number
          INTO assignment_reference_number, assignment_title, due_at_millis,
               assignment_status, assignment_edit_number;
        IF assignment_status = 'released' THEN PERFORM ple_data.validate_assignment_release(assignment_row.assignment_id); END IF;
    ELSE
        assignment_reference_number := assignment_row.reference_number; assignment_title := assignment_row.assignment_title;
        due_at_millis := CASE WHEN assignment_row.due_at IS NULL THEN NULL ELSE floor(extract(epoch FROM assignment_row.due_at) * 1000)::bigint END;
        assignment_status := assignment_row.assignment_status; assignment_edit_number := assignment_row.assignment_edit_number;
    END IF;
    RETURN NEXT;
END
$$;

CREATE FUNCTION ple_data.release_assignment(
    p_course_reference_number bigint,
    p_assignment_reference_number bigint,
    p_expected_edit_number bigint
) RETURNS TABLE (
    assignment_reference_number bigint, assignment_title text,
    assignment_status text, assignment_edit_number bigint
)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data AS $$
DECLARE assignment_row ple_data.assignment%ROWTYPE;
BEGIN
    IF p_course_reference_number NOT BETWEEN 1 AND 2147483647
       OR p_assignment_reference_number NOT BETWEEN 1 AND 2147483647
       OR p_expected_edit_number IS NULL OR p_expected_edit_number <= 0 THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Assignment is unavailable';
    END IF;
    SELECT assignment.* INTO assignment_row
      FROM ple_data.course_instance AS course
      JOIN ple_data.assignment AS assignment ON assignment.course_id = course.course_id
     WHERE course.reference_number = p_course_reference_number
       AND assignment.reference_number = p_assignment_reference_number
       AND ple_api.current_session_account_is_course_instructor(course.course_id)
     FOR UPDATE OF assignment;
    IF NOT FOUND THEN RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Assignment is unavailable'; END IF;
    IF assignment_row.assignment_edit_number IS DISTINCT FROM p_expected_edit_number THEN
        RAISE EXCEPTION USING ERRCODE = '40001', MESSAGE = 'Assignment Edit Number is stale';
    END IF;
    IF assignment_row.assignment_status <> 'unreleased' THEN
        RAISE EXCEPTION USING ERRCODE = '55000', MESSAGE = 'Assignment is not Unreleased';
    END IF;
    PERFORM ple_data.validate_assignment_release(assignment_row.assignment_id);
    UPDATE ple_data.assignment AS updated SET assignment_status = 'released',
        assignment_edit_number = updated.assignment_edit_number + 1,
        updated_at = clock_timestamp()
     WHERE updated.assignment_id = assignment_row.assignment_id
    RETURNING updated.reference_number, updated.assignment_title, updated.assignment_status,
        updated.assignment_edit_number
      INTO assignment_reference_number, assignment_title, assignment_status, assignment_edit_number;
    RETURN NEXT;
END
$$;

CREATE TRIGGER assignment_edit_is_exact
BEFORE UPDATE ON ple_data.assignment
FOR EACH ROW EXECUTE FUNCTION ple_data.enforce_assignment_edit();
CREATE TRIGGER question_pool_item_parent_is_valid
BEFORE INSERT OR UPDATE ON ple_data.question_pool_item
FOR EACH ROW EXECUTE FUNCTION ple_data.validate_question_pool_item_parent();

CREATE INDEX assignment_course_due_idx ON ple_data.assignment(course_id, due_at, reference_number)
    WHERE assignment_status IN ('unreleased', 'released');
CREATE INDEX assignment_due_soon_idx ON ple_data.assignment(due_at, course_id, reference_number)
    WHERE assignment_status IN ('unreleased', 'released') AND due_at IS NOT NULL;
CREATE INDEX assignment_entry_current_idx ON ple_data.assignment_entry(assignment_id, authored_position)
    WHERE availability = 'available';
CREATE INDEX question_pool_item_current_idx ON ple_data.question_pool_item(assignment_entry_id, item_position)
    WHERE availability = 'available';

ALTER TABLE ple_data.assignment ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_data.assignment FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_data.assignment_entry ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_data.assignment_entry FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_data.question_pool_item ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_data.question_pool_item FORCE ROW LEVEL SECURITY;

CREATE POLICY assignment_data_owner_access ON ple_data.assignment
    FOR ALL TO ple_data_owner USING (true) WITH CHECK (true);
CREATE POLICY assignment_entry_data_owner_access ON ple_data.assignment_entry
    FOR ALL TO ple_data_owner USING (true) WITH CHECK (true);
CREATE POLICY question_pool_item_data_owner_access ON ple_data.question_pool_item
    FOR ALL TO ple_data_owner USING (true) WITH CHECK (true);
CREATE POLICY assignment_private_owner_lookup ON ple_data.assignment
    FOR SELECT TO ple_private_owner USING (true);
CREATE POLICY assignment_entry_private_owner_lookup ON ple_data.assignment_entry
    FOR SELECT TO ple_private_owner USING (true);
CREATE POLICY question_pool_item_private_owner_lookup ON ple_data.question_pool_item
    FOR SELECT TO ple_private_owner USING (true);
CREATE POLICY assignment_api_owner_read ON ple_data.assignment
    FOR SELECT TO ple_api_owner
    USING (ple_api.current_session_account_is_course_instructor(course_id));
CREATE POLICY assignment_entry_api_owner_read ON ple_data.assignment_entry
    FOR SELECT TO ple_api_owner
    USING (EXISTS (
        SELECT 1 FROM ple_data.assignment
         WHERE assignment_id = assignment_entry.assignment_id
           AND ple_api.current_session_account_is_course_instructor(course_id)
    ));
CREATE POLICY question_pool_item_api_owner_read ON ple_data.question_pool_item
    FOR SELECT TO ple_api_owner
    USING (EXISTS (
        SELECT 1 FROM ple_data.assignment
         WHERE assignment_id = question_pool_item.assignment_id
           AND ple_api.current_session_account_is_course_instructor(course_id)
    ));

REVOKE ALL ON TABLE ple_data.assignment, ple_data.assignment_entry,
    ple_data.question_pool_item FROM PUBLIC;
REVOKE ALL ON FUNCTION ple_data.enforce_assignment_edit(),
    ple_data.validate_question_pool_item_parent(), ple_data.validate_assignment_release(uuid),
    ple_data.replace_assignment_entries(uuid, jsonb),
    ple_data.create_assignment(uuid, bigint, uuid, text, text),
    ple_data.save_assignment(bigint, bigint, bigint, jsonb, jsonb),
    ple_data.save_assignment_inline(bigint, bigint, bigint, text, timestamptz),
    ple_data.release_assignment(bigint, bigint, bigint)
    FROM PUBLIC;
GRANT SELECT ON ple_data.assignment, ple_data.assignment_entry, ple_data.question_pool_item
    TO ple_private_owner;
GRANT SELECT ON ple_data.assignment, ple_data.assignment_entry, ple_data.question_pool_item
    TO ple_api_owner;
GRANT EXECUTE ON FUNCTION ple_data.create_assignment(uuid, bigint, uuid, text, text),
    ple_data.save_assignment(bigint, bigint, bigint, jsonb, jsonb),
    ple_data.save_assignment_inline(bigint, bigint, bigint, text, timestamptz),
    ple_data.release_assignment(bigint, bigint, bigint),
    ple_data.validate_assignment_release(uuid)
    TO ple_api_owner;

COMMENT ON TABLE ple_data.assignment IS
    'Current Course Assignment aggregate with qualified Edit Number; released saves govern future Attempts.';
COMMENT ON TABLE ple_data.assignment_entry IS
    'Stable current Assignment Entry identity and exact fixed Question Revision pin or Question Pool policy.';
COMMENT ON TABLE ple_data.question_pool_item IS
    'Stable exact Question Revision pin eligible for one current Question Pool.';

RESET ROLE;
