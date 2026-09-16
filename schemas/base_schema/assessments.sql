-- Current Course Assessment aggregates.  These relations describe the content
-- used for future Assessment Attempts; student_work.sql retains the facts used to
-- interpret an Assessment Attempt after a later Assessment edit.

SET LOCAL ROLE ple_data_owner;

CREATE TABLE ple_data.assessment (
    assessment_id uuid PRIMARY KEY,
    course_id uuid NOT NULL REFERENCES ple_data.course_instance(course_id),
    reference_number bigint GENERATED ALWAYS AS IDENTITY UNIQUE NOT NULL
        CHECK (reference_number BETWEEN 1 AND 2147483647),
    public_reference text NOT NULL UNIQUE CHECK (public_reference ~ '^A[0-9ABCDEFGHJKMNPQRSTVWXYZ]{6}$'),
    origin_kind text NOT NULL CHECK (origin_kind IN ('direct', 'adopted')),
    source_blueprint_course_reference_number bigint,
    source_blueprint_revision_number bigint CHECK (source_blueprint_revision_number > 0),
    -- BlueprintAssessmentSource: an exact immutable Blueprint Revision plus
    -- the stable Assessment member selected from that Revision.
    source_blueprint_assessment_reference uuid,
    created_at timestamptz NOT NULL,
    updated_at timestamptz NOT NULL,
    assessment_edit_number bigint NOT NULL DEFAULT 1 CHECK (assessment_edit_number > 0),
    assessment_title text NOT NULL CHECK (
        assessment_title ~ '[^[:space:]]' AND char_length(assessment_title) <= 200
    ),
    assessment_type text NOT NULL CHECK (assessment_type IN (
        'regular_assignment', 'practice_question_assignment', 'bonus_assignment', 'quiz', 'exam'
    )),
    assessment_instructions text NOT NULL CHECK (
        assessment_instructions !~ E'\\x00' AND char_length(assessment_instructions) <= 50000
    ),
    available_at timestamptz,
    due_at timestamptz,
    closes_at timestamptz,
    assessment_attempt_time_limit_seconds integer CHECK (
        assessment_attempt_time_limit_seconds IS NULL
        OR assessment_attempt_time_limit_seconds > 0
    ),
    assessment_attempt_limit integer CHECK (assessment_attempt_limit IS NULL OR assessment_attempt_limit > 0),
    late_work_rule text NOT NULL CHECK (late_work_rule IN ('accept', 'mark_late', 'reject')),
    question_variation_rule text NOT NULL CHECK (
        question_variation_rule IN ('reuse_variation', 'new_variation')
    ),
    assessment_question_order_rule text NOT NULL CHECK (
        assessment_question_order_rule IN ('authored_order', 'shuffled')
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
    feedback_question_answer text NOT NULL CHECK (
        feedback_question_answer IN ('during_attempt', 'after_submit', 'after_due', 'after_close', 'never')
    ),
    feedback_question_answer_explanation text NOT NULL CHECK (
        feedback_question_answer_explanation IN ('during_attempt', 'after_submit', 'after_due', 'after_close', 'never')
    ),
    feedback_class_statistics text NOT NULL CHECK (
        feedback_class_statistics IN ('during_attempt', 'after_submit', 'after_due', 'after_close', 'never')
    ),
    assessment_status text NOT NULL DEFAULT 'unreleased' CHECK (
        assessment_status IN ('unreleased', 'released', 'closed', 'archived')
    ),
    UNIQUE (course_id, reference_number),
    UNIQUE (assessment_id, course_id),
    FOREIGN KEY (
        source_blueprint_course_reference_number,
        source_blueprint_revision_number,
        source_blueprint_assessment_reference
    ) REFERENCES ple_data.blueprint_revision_assessment (
        blueprint_course_reference_number,
        blueprint_revision_number,
        blueprint_assessment_reference
    ),
    CHECK (
        (origin_kind = 'direct'
            AND source_blueprint_course_reference_number IS NULL
            AND source_blueprint_revision_number IS NULL
            AND source_blueprint_assessment_reference IS NULL)
        OR (origin_kind = 'adopted'
            AND source_blueprint_course_reference_number IS NOT NULL
            AND source_blueprint_revision_number IS NOT NULL
            AND source_blueprint_assessment_reference IS NOT NULL)
    ),
    CHECK (updated_at >= created_at),
    CHECK (assessment_type NOT IN ('quiz', 'exam') OR assessment_attempt_limit = 1)
);

CREATE TABLE ple_data.assessment_entry (
    assessment_entry_id uuid PRIMARY KEY,
    assessment_id uuid NOT NULL REFERENCES ple_data.assessment(assessment_id),
    authored_position integer NOT NULL CHECK (authored_position >= 0),
    entry_kind text NOT NULL CHECK (entry_kind IN ('fixed_question', 'question_pool')),
    availability text NOT NULL DEFAULT 'available' CHECK (availability IN ('available', 'retired')),
    active_authored_position integer GENERATED ALWAYS AS (
        CASE WHEN availability = 'available' THEN authored_position END
    ) STORED,
    scoring_rule text NOT NULL CHECK (scoring_rule IN ('normal', 'full_credit', 'extra_credit', 'excluded')),
    question_id text,
    question_revision_number integer,
    question_pool_id uuid,
    question_pool_revision_number bigint,
    points_possible numeric CHECK (points_possible BETWEEN 0 AND 1000000000.9999 AND scale(points_possible) <= 4),
    selection_count integer,
    points_per_item numeric CHECK (points_per_item BETWEEN 0 AND 1000000000.9999 AND scale(points_per_item) <= 4),
    selected_question_order text,
    question_attempt_limit integer,
    question_attempt_time_limit_seconds integer,
    question_attempt_grace_seconds integer,
    -- ASVS 2.3.3: validate current positions after the atomic complete-content
    -- save, allowing swaps while retired Entries retain their historical positions.
    CONSTRAINT assessment_entry_active_authored_position_key
        UNIQUE (assessment_id, active_authored_position) DEFERRABLE INITIALLY DEFERRED,
    UNIQUE (assessment_entry_id, assessment_id),
    FOREIGN KEY (question_id, question_revision_number)
        REFERENCES ple_data.question_revision(question_id, revision_number),
    FOREIGN KEY (question_pool_id, question_pool_revision_number)
        REFERENCES ple_data.question_pool_revision(question_pool_id, revision_number),
    CHECK (
        (entry_kind = 'fixed_question'
            AND question_id IS NOT NULL
            AND question_revision_number IS NOT NULL
            AND question_pool_id IS NULL
            AND question_pool_revision_number IS NULL
            AND points_possible >= 0
            AND selection_count IS NULL
            AND points_per_item IS NULL
            AND selected_question_order IS NULL)
        OR (entry_kind = 'question_pool'
            AND question_id IS NULL
            AND question_revision_number IS NULL
            AND question_pool_id IS NOT NULL
            AND question_pool_revision_number IS NOT NULL
            AND points_possible IS NULL
            AND selection_count > 0
            AND points_per_item >= 0
            AND selected_question_order IN ('question_pool_order', 'random_order'))
    ),
    CHECK (question_attempt_limit IS NULL OR question_attempt_limit > 0),
    CHECK (
        (question_attempt_time_limit_seconds IS NULL AND question_attempt_grace_seconds IS NULL)
        OR (question_attempt_time_limit_seconds > 0
            AND question_attempt_grace_seconds >= 0)
    )
);

-- A Pool imported into an Assessment is owned by exactly one Assessment Entry.
-- This cyclic, deferred association deliberately makes it impossible for the
-- ordinary current-content save to attach a published Pool, another fork, or
-- an arbitrary browser-provided lineage.  The trusted import boundary creates
-- the entry, fork revision 1, and this association atomically.
CREATE TABLE ple_data.assessment_question_pool_fork (
    assessment_entry_id uuid NOT NULL,
    assessment_id uuid NOT NULL,
    question_pool_id uuid NOT NULL UNIQUE,
    origin_question_pool_revision_number bigint NOT NULL CHECK (origin_question_pool_revision_number = 1),
    PRIMARY KEY (assessment_entry_id),
    UNIQUE (assessment_entry_id, assessment_id, question_pool_id),
    FOREIGN KEY (assessment_entry_id, assessment_id)
        REFERENCES ple_data.assessment_entry(assessment_entry_id, assessment_id)
        DEFERRABLE INITIALLY DEFERRED,
    FOREIGN KEY (question_pool_id, origin_question_pool_revision_number)
        REFERENCES ple_data.question_pool_revision(question_pool_id, revision_number)
);

ALTER TABLE ple_data.assessment_entry
    ADD CONSTRAINT assessment_entry_owns_exact_question_pool_fork
    FOREIGN KEY (
        assessment_entry_id, assessment_id, question_pool_id
    ) REFERENCES ple_data.assessment_question_pool_fork (
        assessment_entry_id, assessment_id, question_pool_id
    ) DEFERRABLE INITIALLY DEFERRED;

CREATE FUNCTION ple_data.enforce_assessment_edit()
RETURNS trigger LANGUAGE plpgsql
SET search_path = pg_catalog, ple_data AS $$
BEGIN
    IF NEW.assessment_id IS DISTINCT FROM OLD.assessment_id
       OR NEW.course_id IS DISTINCT FROM OLD.course_id
       OR NEW.reference_number IS DISTINCT FROM OLD.reference_number
       OR NEW.origin_kind IS DISTINCT FROM OLD.origin_kind
       OR NEW.source_blueprint_course_reference_number
            IS DISTINCT FROM OLD.source_blueprint_course_reference_number
       OR NEW.source_blueprint_revision_number
            IS DISTINCT FROM OLD.source_blueprint_revision_number
       OR NEW.source_blueprint_assessment_reference
            IS DISTINCT FROM OLD.source_blueprint_assessment_reference
       OR NEW.created_at IS DISTINCT FROM OLD.created_at
       OR NEW.updated_at < OLD.updated_at THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Assessment identity is immutable and timestamps move forward';
    END IF;

    -- The guarded save operation changes the parent exactly once after it has
    -- accepted the complete normalized child candidate.  Child rows are not
    -- independently writable by a runtime role.
    IF NEW.assessment_edit_number <> OLD.assessment_edit_number
       AND NEW.assessment_edit_number <> OLD.assessment_edit_number + 1 THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Assessment changes advance exactly one Assessment Edit Number';
    END IF;
    RETURN NEW;
END
$$;

CREATE FUNCTION ple_data.validate_assessment_question_pool_fork()
RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_data AS $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM ple_data.question_pool AS pool
         WHERE pool.question_pool_id = NEW.question_pool_id
           AND pool.source_question_pool_id IS NOT NULL
           AND pool.source_question_pool_revision_number IS NOT NULL
    ) THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Assessment Question Pool ownership requires a fork with immutable source provenance';
    END IF;
    RETURN NEW;
END
$$;

-- ASVS 2.2-2.3, 8.2-8.3, and 15.4: only the guarded current-content save
-- changes normalized Assessment Entries.  It receives
-- a closed JSON array because entries are a tagged union.  Each member has:
--
-- fixed_question: assessmentEntryId, availability, scoringRule, questionId,
-- revisionNumber, pointsPossible, questionAttemptLimit,
-- questionAttemptTimeLimitSeconds, questionAttemptGraceSeconds.
-- question_pool: assessmentEntryId, availability, scoringRule,
-- selectionCount, pointsPerItem, selectedQuestionOrder, questionPoolId, and
-- questionPoolRevisionNumber.  The Pool is already a distinct immutable fork;
-- this save never accepts mutable per-item membership.
--
-- Callers retain an ID for an unchanged member.  This permits an archived
-- exact pin to remain in a current Assessment while requiring any new or
-- replaced pin to come from an Available Question lineage.
CREATE FUNCTION ple_data.replace_assessment_entries(
    p_assessment_id uuid,
    p_entries jsonb
) RETURNS boolean
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data AS $$
DECLARE
    entry_json jsonb;
    entry_id uuid;
    entry_kind text;
    entry_ids uuid[] := ARRAY[]::uuid[];
    changed boolean := false;
    row_count integer;
    question_available boolean;
    question_pool_id_value uuid;
BEGIN
    IF p_entries IS NULL OR jsonb_typeof(p_entries) <> 'array'
       OR jsonb_array_length(p_entries) > 1024 THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Assessment Entries are invalid';
    END IF;

    FOR entry_json IN SELECT value FROM jsonb_array_elements(p_entries) LOOP
        IF jsonb_typeof(entry_json) <> 'object'
           OR entry_json ->> 'assessmentEntryId' !~* '^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$'
           OR entry_json ->> 'kind' NOT IN ('fixed_question', 'question_pool')
           OR entry_json ->> 'availability' NOT IN ('available', 'retired')
           OR entry_json ->> 'scoringRule' NOT IN ('normal', 'full_credit', 'extra_credit', 'excluded') THEN
            RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Assessment Entry is invalid';
        END IF;
        entry_id := (entry_json ->> 'assessmentEntryId')::uuid;
        IF entry_id = ANY (entry_ids) THEN
            RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Assessment Entries repeat an identity';
        END IF;
        entry_ids := array_append(entry_ids, entry_id);
        entry_kind := entry_json ->> 'kind';

        IF entry_kind = 'fixed_question' THEN
            IF entry_json ->> 'questionId' IS NULL
               OR entry_json ->> 'revisionNumber' !~ '^[1-9][0-9]*$'
               OR entry_json ->> 'pointsPossible' !~ '^[0-9]{1,10}(\.[0-9]{1,4})?$'
               OR (entry_json ->> 'pointsPossible')::numeric > 1000000000.9999 THEN
                RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Fixed Question Assessment Entry is invalid';
            END IF;
            SELECT question.availability = 'available' INTO question_available
              FROM ple_data.published_question AS question
              JOIN ple_data.question_revision AS revision
                ON revision.question_id = question.question_id
             WHERE question.question_id = entry_json ->> 'questionId'
               AND revision.revision_number = (entry_json ->> 'revisionNumber')::integer;
            IF question_available IS DISTINCT FROM true
               AND NOT EXISTS (
                   SELECT 1 FROM ple_data.assessment_entry AS existing
                    WHERE existing.assessment_id = p_assessment_id
                      AND existing.assessment_entry_id = entry_id
                      AND existing.entry_kind = 'fixed_question'
                      AND existing.question_id = entry_json ->> 'questionId'
                      AND existing.question_revision_number = (entry_json ->> 'revisionNumber')::integer
               ) THEN
                RAISE EXCEPTION USING ERRCODE = '22023',
                    MESSAGE = 'New Assessment Question pins require an Available Question';
            END IF;
            IF EXISTS (
                SELECT 1 FROM ple_data.assessment_entry AS existing
                 WHERE existing.assessment_id = p_assessment_id
                   AND existing.assessment_entry_id = entry_id
                   AND existing.entry_kind <> 'fixed_question'
            ) THEN
                RAISE EXCEPTION USING ERRCODE = '22023',
                    MESSAGE = 'Assessment Entry kind is immutable';
            END IF;
            UPDATE ple_data.assessment_entry AS target
               SET authored_position = COALESCE((entry_json ->> 'authoredPosition')::integer, 0),
                   entry_kind = 'fixed_question', availability = entry_json ->> 'availability',
                   scoring_rule = entry_json ->> 'scoringRule',
                   question_id = entry_json ->> 'questionId',
                   question_revision_number = (entry_json ->> 'revisionNumber')::integer,
                   question_pool_id = NULL, question_pool_revision_number = NULL,
                   points_possible = (entry_json ->> 'pointsPossible')::numeric,
                   selection_count = NULL, points_per_item = NULL, selected_question_order = NULL,
                   question_attempt_limit = NULLIF(entry_json ->> 'questionAttemptLimit', '')::integer,
                   question_attempt_time_limit_seconds = NULLIF(entry_json ->> 'questionAttemptTimeLimitSeconds', '')::integer,
                   question_attempt_grace_seconds = NULLIF(entry_json ->> 'questionAttemptGraceSeconds', '')::integer
             WHERE target.assessment_id = p_assessment_id
               AND target.assessment_entry_id = entry_id
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
                SELECT 1 FROM ple_data.assessment_entry AS existing
                 WHERE existing.assessment_id = p_assessment_id
                   AND existing.assessment_entry_id = entry_id
            ) THEN
                INSERT INTO ple_data.assessment_entry(
                    assessment_entry_id, assessment_id, authored_position, entry_kind, availability,
                    scoring_rule, question_id, question_revision_number, points_possible,
                    question_attempt_limit, question_attempt_time_limit_seconds, question_attempt_grace_seconds
                ) VALUES (
                    entry_id, p_assessment_id, COALESCE((entry_json ->> 'authoredPosition')::integer, 0),
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
               OR entry_json ->> 'pointsPerItem' !~ '^[0-9]{1,10}(\.[0-9]{1,4})?$'
               OR (entry_json ->> 'pointsPerItem')::numeric > 1000000000.9999
               OR entry_json ->> 'selectedQuestionOrder' NOT IN ('question_pool_order', 'random_order')
               OR entry_json ->> 'questionPoolId' !~ '^[0-9A-HJKMNP-TV-Z]{8}$'
               OR entry_json ->> 'questionPoolRevisionNumber' !~ '^[1-9][0-9]*$' THEN
                RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Question Pool Assessment Entry is invalid';
            END IF;
            SELECT pool.question_pool_id INTO question_pool_id_value
              FROM ple_data.question_pool AS pool
             WHERE pool.public_question_pool_id = entry_json ->> 'questionPoolId';
            IF NOT FOUND OR NOT EXISTS (
                SELECT 1 FROM ple_data.question_pool_revision AS pool_revision
                 WHERE pool_revision.question_pool_id = question_pool_id_value
                   AND pool_revision.revision_number
                       = (entry_json ->> 'questionPoolRevisionNumber')::bigint
                   AND (entry_json ->> 'selectionCount')::integer <= pool_revision.member_count
            ) THEN
                RAISE EXCEPTION USING ERRCODE = '22023',
                    MESSAGE = 'Question Pool Assessment Entry is invalid';
            END IF;
            -- An Assessment Pool is its own immutable fork lineage.  The
            -- ordinary complete-content save may alter only Entry policy;
            -- it cannot turn an Entry into a Pool, rebind it to another
            -- lineage, or select an arbitrary historical/current Revision.
            -- Membership changes use the append-only fork command below.
            IF NOT EXISTS (
                SELECT 1
                  FROM ple_data.assessment_entry AS existing
                  JOIN ple_data.assessment_question_pool_fork AS owned
                    ON owned.assessment_entry_id = existing.assessment_entry_id
                   AND owned.assessment_id = existing.assessment_id
                   AND owned.question_pool_id = existing.question_pool_id
                 WHERE existing.assessment_id = p_assessment_id
                   AND existing.assessment_entry_id = entry_id
                   AND existing.entry_kind = 'question_pool'
                   AND existing.question_pool_id = question_pool_id_value
                   AND existing.question_pool_revision_number
                       = (entry_json ->> 'questionPoolRevisionNumber')::bigint
            ) THEN
                RAISE EXCEPTION USING ERRCODE = '22023',
                    MESSAGE = 'Assessment Question Pool membership requires an immutable fork command';
            END IF;
            UPDATE ple_data.assessment_entry AS target
               SET authored_position = COALESCE((entry_json ->> 'authoredPosition')::integer, 0),
                   entry_kind = 'question_pool', availability = entry_json ->> 'availability',
                   scoring_rule = entry_json ->> 'scoringRule', question_id = NULL,
                   question_revision_number = NULL, points_possible = NULL,
                   question_pool_id = question_pool_id_value,
                   question_pool_revision_number = (entry_json ->> 'questionPoolRevisionNumber')::bigint,
                   selection_count = (entry_json ->> 'selectionCount')::integer,
                   points_per_item = (entry_json ->> 'pointsPerItem')::numeric,
                   selected_question_order = entry_json ->> 'selectedQuestionOrder',
                   question_attempt_limit = NULLIF(entry_json ->> 'questionAttemptLimit', '')::integer,
                   question_attempt_time_limit_seconds = NULLIF(entry_json ->> 'questionAttemptTimeLimitSeconds', '')::integer,
                   question_attempt_grace_seconds = NULLIF(entry_json ->> 'questionAttemptGraceSeconds', '')::integer
             WHERE target.assessment_id = p_assessment_id AND target.assessment_entry_id = entry_id
               AND ROW(target.authored_position, target.entry_kind, target.availability, target.scoring_rule,
                       target.selection_count, target.points_per_item, target.selected_question_order,
                       target.question_pool_id, target.question_pool_revision_number,
                       target.question_attempt_limit, target.question_attempt_time_limit_seconds,
                       target.question_attempt_grace_seconds) IS DISTINCT FROM ROW(
                       COALESCE((entry_json ->> 'authoredPosition')::integer, 0), 'question_pool',
                       entry_json ->> 'availability', entry_json ->> 'scoringRule',
                       (entry_json ->> 'selectionCount')::integer, (entry_json ->> 'pointsPerItem')::numeric,
                       entry_json ->> 'selectedQuestionOrder',
                       target.question_pool_id, target.question_pool_revision_number,
                       NULLIF(entry_json ->> 'questionAttemptLimit', '')::integer,
                       NULLIF(entry_json ->> 'questionAttemptTimeLimitSeconds', '')::integer,
                       NULLIF(entry_json ->> 'questionAttemptGraceSeconds', '')::integer);
            GET DIAGNOSTICS row_count = ROW_COUNT;
            changed := changed OR row_count > 0;
        END IF;
    END LOOP;
    UPDATE ple_data.assessment_entry SET availability = 'retired'
     WHERE assessment_id = p_assessment_id AND availability <> 'retired'
       AND NOT (assessment_entry_id = ANY (entry_ids));
    GET DIAGNOSTICS row_count = ROW_COUNT;
    RETURN changed OR row_count > 0;
END
$$;

-- The mutable scalar values are an exact object with the Assessment table's
-- non-identity fields.  Keeping the tagged child collection separate avoids
-- generic snapshot persistence while one operation validates the full result.
CREATE FUNCTION ple_data.save_assessment(
    p_course_reference_number bigint,
    p_assessment_reference_number bigint,
    p_expected_edit_number bigint,
    p_values jsonb,
    p_entries jsonb
) RETURNS TABLE (
    assessment_reference_number bigint, assessment_edit_number bigint,
    assessment_status text, assessment_title text, assessment_instructions text
)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data AS $$
DECLARE
    course_row ple_data.course_instance%ROWTYPE;
    current_assessment ple_data.assessment%ROWTYPE;
    candidate ple_data.assessment%ROWTYPE;
    entries_changed boolean;
    values_changed boolean;
    allowed_keys text[] := ARRAY[
        'assessment_title', 'assessment_instructions', 'available_at', 'due_at', 'closes_at',
        'assessment_attempt_time_limit_seconds', 'assessment_attempt_limit', 'late_work_rule',
        'question_variation_rule',
        'assessment_question_order_rule', 'feedback_score',
        'feedback_per_item_correctness', 'feedback_submitted_response', 'feedback_question_answer',
        'feedback_question_answer_explanation', 'feedback_class_statistics'
    ];
BEGIN
    IF p_course_reference_number NOT BETWEEN 1 AND 2147483647
       OR p_assessment_reference_number NOT BETWEEN 1 AND 2147483647
       OR p_expected_edit_number IS NULL OR p_expected_edit_number <= 0
       OR p_values IS NULL OR jsonb_typeof(p_values) <> 'object'
       OR EXISTS (
           SELECT 1 FROM jsonb_object_keys(p_values) AS key WHERE key <> ALL (allowed_keys)
       ) OR EXISTS (
           SELECT 1 FROM unnest(allowed_keys) AS key WHERE NOT p_values ? key
       ) THEN
        RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Assessment save is invalid';
    END IF;
    SELECT course.* INTO course_row
      FROM ple_data.course_instance AS course
     WHERE course.reference_number = p_course_reference_number
       AND ple_api.current_session_account_is_course_instructor(course.course_id)
     FOR UPDATE;
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Assessment is unavailable';
    END IF;
    SELECT assessment.* INTO current_assessment
      FROM ple_data.assessment AS assessment
     WHERE assessment.course_id = course_row.course_id
       AND assessment.reference_number = p_assessment_reference_number
     FOR UPDATE;
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Assessment is unavailable';
    END IF;
    IF current_assessment.assessment_edit_number IS DISTINCT FROM p_expected_edit_number THEN
        RAISE EXCEPTION USING ERRCODE = '40001', MESSAGE = 'Assessment Edit Number is stale';
    END IF;
    SELECT * INTO candidate FROM jsonb_populate_record(current_assessment, p_values);
    IF candidate.due_at > course_row.active_until_at THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Assessment due date is after the Course Active cutoff';
    END IF;
    values_changed := ROW(
        candidate.assessment_title, candidate.assessment_instructions, candidate.available_at,
        candidate.due_at, candidate.closes_at, candidate.assessment_attempt_time_limit_seconds,
        candidate.assessment_attempt_limit, candidate.late_work_rule,
        candidate.question_variation_rule,
        candidate.assessment_question_order_rule,
        candidate.feedback_score, candidate.feedback_per_item_correctness,
        candidate.feedback_submitted_response,
        candidate.feedback_question_answer, candidate.feedback_question_answer_explanation,
        candidate.feedback_class_statistics
    ) IS DISTINCT FROM ROW(
        current_assessment.assessment_title, current_assessment.assessment_instructions,
        current_assessment.available_at, current_assessment.due_at, current_assessment.closes_at,
        current_assessment.assessment_attempt_time_limit_seconds, current_assessment.assessment_attempt_limit,
        current_assessment.late_work_rule, current_assessment.question_variation_rule,
        current_assessment.assessment_question_order_rule, current_assessment.feedback_score,
        current_assessment.feedback_per_item_correctness,
        current_assessment.feedback_submitted_response,
        current_assessment.feedback_question_answer,
        current_assessment.feedback_question_answer_explanation,
        current_assessment.feedback_class_statistics
    );
    entries_changed := ple_data.replace_assessment_entries(current_assessment.assessment_id, p_entries);
    IF values_changed OR entries_changed THEN
        UPDATE ple_data.assessment AS updated SET
            assessment_title = candidate.assessment_title,
            assessment_instructions = candidate.assessment_instructions,
            available_at = candidate.available_at, due_at = candidate.due_at,
            closes_at = candidate.closes_at,
            assessment_attempt_time_limit_seconds = candidate.assessment_attempt_time_limit_seconds,
            assessment_attempt_limit = candidate.assessment_attempt_limit, late_work_rule = candidate.late_work_rule,
            question_variation_rule = candidate.question_variation_rule,
            assessment_question_order_rule = candidate.assessment_question_order_rule,
            feedback_score = candidate.feedback_score,
            feedback_per_item_correctness = candidate.feedback_per_item_correctness,
            feedback_submitted_response = candidate.feedback_submitted_response,
            feedback_question_answer = candidate.feedback_question_answer,
            feedback_question_answer_explanation = candidate.feedback_question_answer_explanation,
            feedback_class_statistics = candidate.feedback_class_statistics,
            assessment_edit_number = updated.assessment_edit_number + 1,
            updated_at = clock_timestamp()
         WHERE updated.assessment_id = current_assessment.assessment_id
        RETURNING updated.reference_number, updated.assessment_edit_number,
            updated.assessment_status, updated.assessment_title, updated.assessment_instructions
          INTO assessment_reference_number, assessment_edit_number, assessment_status,
               assessment_title, assessment_instructions;
        IF assessment_status = 'released' THEN
            PERFORM ple_data.validate_assessment_release(
                current_assessment.assessment_id,
                transaction_timestamp(),
                candidate.due_at IS DISTINCT FROM current_assessment.due_at
            );
        END IF;
        PERFORM ple_data.synchronize_course_assessment_deadline(course_row.course_id);
    ELSE
        assessment_reference_number := current_assessment.reference_number;
        assessment_edit_number := current_assessment.assessment_edit_number;
        assessment_status := current_assessment.assessment_status;
        assessment_title := current_assessment.assessment_title;
        assessment_instructions := current_assessment.assessment_instructions;
    END IF;
    RETURN NEXT;
END
$$;

CREATE FUNCTION ple_data.save_assessment_inline(
    p_course_reference_number bigint, p_assessment_reference_number bigint,
    p_expected_edit_number bigint, p_title text, p_due_at timestamptz
) RETURNS TABLE (
    assessment_reference_number bigint, assessment_title text, due_at_millis bigint,
    assessment_status text, assessment_edit_number bigint
)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data AS $$
DECLARE
    course_row ple_data.course_instance%ROWTYPE;
    assessment_row ple_data.assessment%ROWTYPE;
BEGIN
    IF p_course_reference_number NOT BETWEEN 1 AND 2147483647
       OR p_assessment_reference_number NOT BETWEEN 1 AND 2147483647
       OR p_expected_edit_number IS NULL OR p_expected_edit_number <= 0 OR p_title IS NULL THEN
        RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Assessment inline save is invalid';
    END IF;
    SELECT course.* INTO course_row
      FROM ple_data.course_instance AS course
     WHERE course.reference_number = p_course_reference_number
       AND ple_api.current_session_account_is_course_instructor(course.course_id)
     FOR UPDATE;
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Assessment is unavailable';
    END IF;
    SELECT assessment.* INTO assessment_row
      FROM ple_data.assessment AS assessment
     WHERE assessment.course_id = course_row.course_id
       AND assessment.reference_number = p_assessment_reference_number
     FOR UPDATE;
    IF NOT FOUND THEN RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Assessment is unavailable'; END IF;
    IF assessment_row.assessment_edit_number IS DISTINCT FROM p_expected_edit_number THEN
        RAISE EXCEPTION USING ERRCODE = '40001', MESSAGE = 'Assessment Edit Number is stale';
    END IF;
    IF p_due_at > course_row.active_until_at THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Assessment due date is after the Course Active cutoff';
    END IF;
    IF ROW(assessment_row.assessment_title, assessment_row.due_at) IS DISTINCT FROM ROW(p_title, p_due_at) THEN
        UPDATE ple_data.assessment AS updated SET assessment_title = p_title, due_at = p_due_at,
            assessment_edit_number = updated.assessment_edit_number + 1, updated_at = clock_timestamp()
         WHERE updated.assessment_id = assessment_row.assessment_id
        RETURNING updated.reference_number, updated.assessment_title,
            CASE WHEN updated.due_at IS NULL THEN NULL
                 ELSE floor(extract(epoch FROM updated.due_at) * 1000)::bigint END,
            updated.assessment_status, updated.assessment_edit_number
          INTO assessment_reference_number, assessment_title, due_at_millis,
               assessment_status, assessment_edit_number;
        IF assessment_status = 'released' THEN
            PERFORM ple_data.validate_assessment_release(
                assessment_row.assessment_id,
                transaction_timestamp(),
                p_due_at IS DISTINCT FROM assessment_row.due_at
            );
        END IF;
        PERFORM ple_data.synchronize_course_assessment_deadline(course_row.course_id);
    ELSE
        assessment_reference_number := assessment_row.reference_number; assessment_title := assessment_row.assessment_title;
        due_at_millis := CASE WHEN assessment_row.due_at IS NULL THEN NULL ELSE floor(extract(epoch FROM assessment_row.due_at) * 1000)::bigint END;
        assessment_status := assessment_row.assessment_status; assessment_edit_number := assessment_row.assessment_edit_number;
    END IF;
    RETURN NEXT;
END
$$;

-- Policy-only persistence keeps Question Entries and title outside this write boundary.
CREATE FUNCTION ple_data.save_assessment_policies(
    p_course_reference_number bigint, p_assessment_reference_number bigint,
    p_expected_edit_number bigint, p_policies jsonb
) RETURNS TABLE (
    assessment_reference_number bigint, assessment_edit_number bigint,
    assessment_status text, assessment_title text, assessment_instructions text
)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data AS $$
DECLARE course_row ple_data.course_instance%ROWTYPE;
    current_assessment ple_data.assessment%ROWTYPE; candidate ple_data.assessment%ROWTYPE;
    values_changed boolean;
    allowed_keys text[] := ARRAY[
        'assessment_instructions', 'available_at', 'due_at', 'closes_at',
        'assessment_attempt_time_limit_seconds', 'assessment_attempt_limit', 'late_work_rule',
        'question_variation_rule',
        'assessment_question_order_rule', 'feedback_score',
        'feedback_per_item_correctness', 'feedback_submitted_response',
        'feedback_question_answer', 'feedback_question_answer_explanation', 'feedback_class_statistics'];
BEGIN
    IF p_course_reference_number NOT BETWEEN 1 AND 2147483647
       OR p_assessment_reference_number NOT BETWEEN 1 AND 2147483647
       OR p_expected_edit_number IS NULL OR p_expected_edit_number <= 0
       OR p_policies IS NULL OR jsonb_typeof(p_policies) <> 'object'
       OR EXISTS (SELECT 1 FROM jsonb_object_keys(p_policies) AS key WHERE key <> ALL (allowed_keys))
       OR EXISTS (SELECT 1 FROM unnest(allowed_keys) AS key WHERE NOT p_policies ? key) THEN
        RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Assessment policy save is invalid';
    END IF;
    SELECT course.* INTO course_row
      FROM ple_data.course_instance AS course
     WHERE course.reference_number = p_course_reference_number
       AND ple_api.current_session_account_is_course_instructor(course.course_id)
     FOR UPDATE;
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Assessment is unavailable';
    END IF;
    SELECT assessment.* INTO current_assessment
      FROM ple_data.assessment AS assessment
     WHERE assessment.course_id = course_row.course_id
       AND assessment.reference_number = p_assessment_reference_number
     FOR UPDATE;
    IF NOT FOUND THEN RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Assessment is unavailable'; END IF;
    IF current_assessment.assessment_edit_number IS DISTINCT FROM p_expected_edit_number THEN
        RAISE EXCEPTION USING ERRCODE = '40001', MESSAGE = 'Assessment Edit Number is stale';
    END IF;
    SELECT * INTO candidate FROM jsonb_populate_record(current_assessment, p_policies);
    IF candidate.due_at > course_row.active_until_at THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Assessment due date is after the Course Active cutoff';
    END IF;
    values_changed := ROW(candidate.assessment_instructions, candidate.available_at, candidate.due_at,
        candidate.closes_at, candidate.assessment_attempt_time_limit_seconds, candidate.assessment_attempt_limit,
        candidate.late_work_rule, candidate.question_variation_rule, candidate.assessment_question_order_rule, candidate.feedback_score, candidate.feedback_per_item_correctness,
        candidate.feedback_submitted_response, candidate.feedback_question_answer,
        candidate.feedback_question_answer_explanation, candidate.feedback_class_statistics)
      IS DISTINCT FROM ROW(current_assessment.assessment_instructions, current_assessment.available_at,
        current_assessment.due_at, current_assessment.closes_at, current_assessment.assessment_attempt_time_limit_seconds,
        current_assessment.assessment_attempt_limit, current_assessment.late_work_rule,
        current_assessment.question_variation_rule,
        current_assessment.assessment_question_order_rule,
        current_assessment.feedback_score, current_assessment.feedback_per_item_correctness,
        current_assessment.feedback_submitted_response,
        current_assessment.feedback_question_answer, current_assessment.feedback_question_answer_explanation,
        current_assessment.feedback_class_statistics);
    IF values_changed THEN
        UPDATE ple_data.assessment AS updated SET
          assessment_instructions = candidate.assessment_instructions, available_at = candidate.available_at,
          due_at = candidate.due_at, closes_at = candidate.closes_at,
          assessment_attempt_time_limit_seconds = candidate.assessment_attempt_time_limit_seconds,
          assessment_attempt_limit = candidate.assessment_attempt_limit, late_work_rule = candidate.late_work_rule,
          question_variation_rule = candidate.question_variation_rule,
          assessment_question_order_rule = candidate.assessment_question_order_rule,
          feedback_score = candidate.feedback_score, feedback_per_item_correctness = candidate.feedback_per_item_correctness,
          feedback_submitted_response = candidate.feedback_submitted_response,
          feedback_question_answer = candidate.feedback_question_answer,
          feedback_question_answer_explanation = candidate.feedback_question_answer_explanation,
          feedback_class_statistics = candidate.feedback_class_statistics,
          assessment_edit_number = updated.assessment_edit_number + 1, updated_at = clock_timestamp()
          WHERE updated.assessment_id = current_assessment.assessment_id
        RETURNING updated.reference_number, updated.assessment_edit_number, updated.assessment_status,
          updated.assessment_title, updated.assessment_instructions INTO assessment_reference_number,
          assessment_edit_number, assessment_status, assessment_title, assessment_instructions;
        IF assessment_status = 'released' THEN
            PERFORM ple_data.validate_assessment_release(
                current_assessment.assessment_id,
                transaction_timestamp(),
                candidate.due_at IS DISTINCT FROM current_assessment.due_at
            );
        END IF;
        PERFORM ple_data.synchronize_course_assessment_deadline(course_row.course_id);
    ELSE
        assessment_reference_number := current_assessment.reference_number; assessment_edit_number := current_assessment.assessment_edit_number;
        assessment_status := current_assessment.assessment_status; assessment_title := current_assessment.assessment_title;
        assessment_instructions := current_assessment.assessment_instructions;
    END IF;
    RETURN NEXT;
END
$$;

CREATE FUNCTION ple_data.release_assessment(
    p_course_reference_number bigint,
    p_assessment_reference_number bigint,
    p_expected_edit_number bigint
) RETURNS TABLE (
    assessment_reference_number bigint, assessment_title text,
    assessment_status text, assessment_edit_number bigint
)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data AS $$
DECLARE
    course_row ple_data.course_instance%ROWTYPE;
    assessment_row ple_data.assessment%ROWTYPE;
BEGIN
    IF p_course_reference_number NOT BETWEEN 1 AND 2147483647
       OR p_assessment_reference_number NOT BETWEEN 1 AND 2147483647
       OR p_expected_edit_number IS NULL OR p_expected_edit_number <= 0 THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Assessment is unavailable';
    END IF;
    SELECT course.* INTO course_row
      FROM ple_data.course_instance AS course
     WHERE course.reference_number = p_course_reference_number
       AND ple_api.current_session_account_is_course_instructor(course.course_id)
     FOR UPDATE;
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Assessment is unavailable';
    END IF;
    SELECT assessment.* INTO assessment_row
      FROM ple_data.assessment AS assessment
     WHERE assessment.course_id = course_row.course_id
       AND assessment.reference_number = p_assessment_reference_number
     FOR UPDATE;
    IF NOT FOUND THEN RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Assessment is unavailable'; END IF;
    IF assessment_row.assessment_edit_number IS DISTINCT FROM p_expected_edit_number THEN
        RAISE EXCEPTION USING ERRCODE = '40001', MESSAGE = 'Assessment Edit Number is stale';
    END IF;
    IF assessment_row.assessment_status <> 'unreleased' THEN
        RAISE EXCEPTION USING ERRCODE = '55000', MESSAGE = 'Assessment is not Unreleased';
    END IF;
    PERFORM ple_data.validate_assessment_release(
        assessment_row.assessment_id, transaction_timestamp(), true
    );
    UPDATE ple_data.assessment AS updated SET assessment_status = 'released',
        assessment_edit_number = updated.assessment_edit_number + 1,
        updated_at = clock_timestamp()
     WHERE updated.assessment_id = assessment_row.assessment_id
    RETURNING updated.reference_number, updated.assessment_title, updated.assessment_status,
        updated.assessment_edit_number
      INTO assessment_reference_number, assessment_title, assessment_status, assessment_edit_number;
    PERFORM ple_data.synchronize_course_assessment_deadline(course_row.course_id);
    RETURN NEXT;
END
$$;

CREATE TRIGGER assessment_edit_is_exact
BEFORE UPDATE ON ple_data.assessment
FOR EACH ROW EXECUTE FUNCTION ple_data.enforce_assessment_edit();
CREATE TRIGGER assessment_human_reference_is_minted
BEFORE INSERT ON ple_data.assessment
FOR EACH ROW EXECUTE FUNCTION ple_private.assign_human_reference('A');
CREATE TRIGGER assessment_question_pool_fork_requires_fork_provenance
BEFORE INSERT OR UPDATE ON ple_data.assessment_question_pool_fork
FOR EACH ROW EXECUTE FUNCTION ple_data.validate_assessment_question_pool_fork();

CREATE INDEX assessment_course_due_idx ON ple_data.assessment(course_id, due_at, reference_number)
    WHERE assessment_status IN ('unreleased', 'released');
CREATE INDEX assessment_due_soon_idx ON ple_data.assessment(due_at, course_id, reference_number)
    WHERE assessment_status IN ('unreleased', 'released') AND due_at IS NOT NULL;
CREATE INDEX assessment_entry_current_idx ON ple_data.assessment_entry(assessment_id, authored_position)
    WHERE availability = 'available';

ALTER TABLE ple_data.assessment ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_data.assessment FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_data.assessment_entry ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_data.assessment_entry FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_data.assessment_question_pool_fork ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_data.assessment_question_pool_fork FORCE ROW LEVEL SECURITY;

CREATE POLICY assessment_data_owner_access ON ple_data.assessment
    FOR ALL TO ple_data_owner USING (true) WITH CHECK (true);
CREATE POLICY assessment_entry_data_owner_access ON ple_data.assessment_entry
    FOR ALL TO ple_data_owner USING (true) WITH CHECK (true);
CREATE POLICY assessment_question_pool_fork_data_owner_access ON ple_data.assessment_question_pool_fork
    FOR ALL TO ple_data_owner USING (true) WITH CHECK (true);
CREATE POLICY assessment_private_owner_lookup ON ple_data.assessment
    FOR SELECT TO ple_private_owner USING (true);
-- Student Work starts, saves, and finalizes lock their Assessment root first.
-- PostgreSQL requires UPDATE on one selected column for SELECT FOR UPDATE;
-- this grants no general Assessment mutation capability.
CREATE POLICY assessment_private_owner_student_work_root_lock ON ple_data.assessment
    FOR UPDATE TO ple_private_owner USING (true) WITH CHECK (true);
CREATE POLICY assessment_entry_private_owner_lookup ON ple_data.assessment_entry
    FOR SELECT TO ple_private_owner USING (true);
CREATE POLICY assessment_question_pool_fork_private_owner_lookup ON ple_data.assessment_question_pool_fork
    FOR SELECT TO ple_private_owner USING (true);
CREATE POLICY assessment_api_owner_read ON ple_data.assessment
    FOR SELECT TO ple_api_owner
    USING (ple_api.current_session_account_is_course_instructor(course_id));
CREATE POLICY assessment_entry_api_owner_read ON ple_data.assessment_entry
    FOR SELECT TO ple_api_owner
    USING (EXISTS (
        SELECT 1 FROM ple_data.assessment
         WHERE assessment_id = assessment_entry.assessment_id
           AND ple_api.current_session_account_is_course_instructor(course_id)
    ));
CREATE POLICY assessment_question_pool_fork_api_owner_read ON ple_data.assessment_question_pool_fork
    FOR SELECT TO ple_api_owner
    USING (EXISTS (
        SELECT 1 FROM ple_data.assessment
         WHERE assessment_id = assessment_question_pool_fork.assessment_id
           AND ple_api.current_session_account_is_course_instructor(course_id)
    ));

REVOKE ALL ON TABLE ple_data.assessment, ple_data.assessment_entry,
    ple_data.assessment_question_pool_fork FROM PUBLIC;
REVOKE ALL ON FUNCTION ple_data.enforce_assessment_edit(),
    ple_data.validate_assessment_question_pool_fork(),
    ple_data.replace_assessment_entries(uuid, jsonb),
    ple_data.save_assessment(bigint, bigint, bigint, jsonb, jsonb),
    ple_data.save_assessment_inline(bigint, bigint, bigint, text, timestamptz),
    ple_data.save_assessment_policies(bigint, bigint, bigint, jsonb),
    ple_data.release_assessment(bigint, bigint, bigint)
    FROM PUBLIC;
GRANT SELECT ON ple_data.assessment, ple_data.assessment_entry, ple_data.assessment_question_pool_fork
    TO ple_private_owner;
GRANT UPDATE (assessment_id) ON TABLE ple_data.assessment TO ple_private_owner;
GRANT SELECT ON ple_data.assessment, ple_data.assessment_entry, ple_data.assessment_question_pool_fork
    TO ple_api_owner;
GRANT EXECUTE ON FUNCTION ple_data.save_assessment(bigint, bigint, bigint, jsonb, jsonb),
    ple_data.save_assessment_inline(bigint, bigint, bigint, text, timestamptz),
    ple_data.save_assessment_policies(bigint, bigint, bigint, jsonb),
    ple_data.release_assessment(bigint, bigint, bigint)
    TO ple_api_owner;

SET LOCAL ROLE ple_data_owner;

COMMENT ON TABLE ple_data.assessment IS
    'Current Course Assessment aggregate with qualified Edit Number; released saves govern future Assessment Attempts.';
COMMENT ON TABLE ple_data.assessment_entry IS
    'Stable current Assessment Entry identity and exact fixed Question Revision pin or Question Pool policy.';
COMMENT ON TABLE ple_data.assessment_question_pool_fork IS
    'One Assessment Entry-owned fork Pool lineage; origin revision 1 is retained while the Entry pins later immutable fork revisions.';

RESET ROLE;
