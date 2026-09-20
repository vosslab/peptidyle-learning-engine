-- Functions, triggers, and views from assessments.sql.

SET LOCAL ROLE ple_data_owner;

CREATE FUNCTION ple_data.enforce_assessment_edit()
RETURNS trigger LANGUAGE plpgsql
SET search_path = pg_catalog, ple_data AS $$
BEGIN
    IF NEW.assessment_id IS DISTINCT FROM OLD.assessment_id
       OR NEW.course_instance_id IS DISTINCT FROM OLD.course_instance_id
       OR NEW.origin_kind IS DISTINCT FROM OLD.origin_kind
       OR NEW.source_blueprint_course_id
            IS DISTINCT FROM OLD.source_blueprint_course_id
       OR NEW.source_blueprint_revision_number
            IS DISTINCT FROM OLD.source_blueprint_revision_number
       OR NEW.source_blueprint_assessment_id
            IS DISTINCT FROM OLD.source_blueprint_assessment_id
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
-- questionPoolEditNumber.  The Pool is already a distinct immutable fork;
-- this save never accepts mutable per-item membership.
--
-- Callers retain an ID for an unchanged member.  This permits an archived
-- exact pin to remain in a current Assessment while requiring any new or
-- replaced pin to come from an Available Question lineage.
CREATE FUNCTION ple_data.replace_assessment_entries(
    p_assessment_id text,
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
    question_backend_supported boolean;
    question_pool_id_value text;
BEGIN
    IF p_entries IS NULL OR jsonb_typeof(p_entries) <> 'array'
       OR jsonb_array_length(p_entries) > 1024 THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Assessment Entries are invalid';
    END IF;

    -- ASVS 2.2.1, 2.2.2: trusted authoring bounds the delivered count only.
    IF (SELECT COALESCE(sum(CASE value ->> 'kind'
                WHEN 'fixed_question' THEN 1
                WHEN 'question_pool' THEN (value ->> 'selectionCount')::bigint
                ELSE 0 END), 0)
          FROM jsonb_array_elements(p_entries)
         WHERE value ->> 'availability' = 'available') > 250 THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Assessment may contain at most 250 Questions';
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
               OR entry_json ->> 'questionId' !~ '^[0-9ABCDEFGHJKMNPQRSTVWXYZ]{4}-[0-9ABCDEFGHJKMNPQRSTVWXYZ]{4}$'
               OR substr(entry_json ->> 'questionId', 6, 1)
                    <> ple_private.crockford_checksum_character(
                        substr(entry_json ->> 'questionId', 1, 4)
                        || substr(entry_json ->> 'questionId', 7, 3)
                    )
               OR entry_json ->> 'revisionNumber' !~ '^[1-9][0-9]*$'
               OR entry_json ->> 'pointsPossible' !~ '^[0-9]{1,10}(\.[0-9]{1,4})?$'
               OR (entry_json ->> 'pointsPossible')::numeric > 1000000000.9999 THEN
                RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Fixed Question Assessment Entry is invalid';
            END IF;
            SELECT question.availability = 'available',
                   ple_private.question_backend_is_supported_for_production(revision.backend)
              INTO question_available, question_backend_supported
              FROM ple_data.published_question AS question
              JOIN ple_data.question_revision AS revision
                ON revision.published_question_id = question.published_question_id
             WHERE question.published_question_id = entry_json ->> 'questionId'
               AND revision.revision_number = (entry_json ->> 'revisionNumber')::integer;
            IF question_available IS DISTINCT FROM true
               AND NOT EXISTS (
                   SELECT 1 FROM ple_data.assessment_entry AS existing
                   JOIN ple_data.assessment_entry_question AS question
                     ON question.assessment_entry_id = existing.assessment_entry_id
                    WHERE existing.assessment_id = p_assessment_id
                      AND existing.assessment_entry_id = entry_id
                      AND existing.entry_kind = 'fixed_question'
                      AND question.published_question_id = entry_json ->> 'questionId'
                      AND question.question_revision_number = (entry_json ->> 'revisionNumber')::integer
               ) THEN
                RAISE EXCEPTION USING ERRCODE = '22023',
                    MESSAGE = 'New Assessment Question pins require an Available Question';
            END IF;
            IF question_backend_supported IS DISTINCT FROM true
               AND NOT EXISTS (
                   SELECT 1 FROM ple_data.assessment_entry AS existing
                   JOIN ple_data.assessment_entry_question AS question
                     ON question.assessment_entry_id = existing.assessment_entry_id
                    WHERE existing.assessment_id = p_assessment_id
                      AND existing.assessment_entry_id = entry_id
                      AND existing.entry_kind = 'fixed_question'
                      AND question.published_question_id = entry_json ->> 'questionId'
                      AND question.question_revision_number = (entry_json ->> 'revisionNumber')::integer
               ) THEN
                RAISE EXCEPTION USING ERRCODE = '22023',
                    MESSAGE = 'New Assessment Question pins require a current production Question Backend';
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
                   availability = entry_json ->> 'availability',
                   scoring_rule = entry_json ->> 'scoringRule',
                   question_attempt_limit = NULLIF(entry_json ->> 'questionAttemptLimit', '')::integer,
                   question_attempt_time_limit_seconds = NULLIF(entry_json ->> 'questionAttemptTimeLimitSeconds', '')::integer,
                   question_attempt_grace_seconds = NULLIF(entry_json ->> 'questionAttemptGraceSeconds', '')::integer
             WHERE target.assessment_id = p_assessment_id
               AND target.assessment_entry_id = entry_id
               AND ROW(target.authored_position, target.availability, target.scoring_rule,
                       target.question_attempt_limit, target.question_attempt_time_limit_seconds,
                       target.question_attempt_grace_seconds)
                   IS DISTINCT FROM ROW(
                       COALESCE((entry_json ->> 'authoredPosition')::integer, 0),
                       entry_json ->> 'availability', entry_json ->> 'scoringRule',
                       NULLIF(entry_json ->> 'questionAttemptLimit', '')::integer,
                       NULLIF(entry_json ->> 'questionAttemptTimeLimitSeconds', '')::integer,
                       NULLIF(entry_json ->> 'questionAttemptGraceSeconds', '')::integer);
            GET DIAGNOSTICS row_count = ROW_COUNT;
            changed := changed OR row_count > 0;
            UPDATE ple_data.assessment_entry_question AS question
               SET published_question_id = entry_json ->> 'questionId',
                   question_revision_number = (entry_json ->> 'revisionNumber')::integer,
                   points_possible = (entry_json ->> 'pointsPossible')::numeric
             WHERE question.assessment_entry_id = entry_id
               AND question.assessment_id = p_assessment_id
               AND ROW(question.published_question_id, question.question_revision_number,
                       question.points_possible)
                   IS DISTINCT FROM ROW(
                       entry_json ->> 'questionId', (entry_json ->> 'revisionNumber')::integer,
                       (entry_json ->> 'pointsPossible')::numeric);
            GET DIAGNOSTICS row_count = ROW_COUNT;
            changed := changed OR row_count > 0;
            IF NOT EXISTS (
                SELECT 1 FROM ple_data.assessment_entry AS existing
                 WHERE existing.assessment_id = p_assessment_id
                   AND existing.assessment_entry_id = entry_id
            ) THEN
                INSERT INTO ple_data.assessment_entry(
                    assessment_entry_id, assessment_id, authored_position, entry_kind, availability,
                    scoring_rule, question_attempt_limit, question_attempt_time_limit_seconds,
                    question_attempt_grace_seconds
                ) VALUES (
                    entry_id, p_assessment_id, COALESCE((entry_json ->> 'authoredPosition')::integer, 0),
                    'fixed_question', entry_json ->> 'availability', entry_json ->> 'scoringRule',
                    NULLIF(entry_json ->> 'questionAttemptLimit', '')::integer,
                    NULLIF(entry_json ->> 'questionAttemptTimeLimitSeconds', '')::integer,
                    NULLIF(entry_json ->> 'questionAttemptGraceSeconds', '')::integer
                );
                INSERT INTO ple_data.assessment_entry_question(
                    assessment_entry_id, assessment_id, published_question_id,
                    question_revision_number, points_possible
                ) VALUES (
                    entry_id, p_assessment_id, entry_json ->> 'questionId',
                    (entry_json ->> 'revisionNumber')::integer,
                    (entry_json ->> 'pointsPossible')::numeric
                );
                changed := true;
            END IF;
        ELSE
            IF entry_json ->> 'selectionCount' !~ '^[1-9][0-9]*$'
               OR entry_json ->> 'pointsPerItem' !~ '^[0-9]{1,10}(\.[0-9]{1,4})?$'
               OR (entry_json ->> 'pointsPerItem')::numeric > 1000000000.9999
               OR entry_json ->> 'selectedQuestionOrder' NOT IN ('question_pool_order', 'random_order')
               OR entry_json ->> 'questionPoolId' !~ '^[0-9ABCDEFGHJKMNPQRSTVWXYZ]{4}-[0-9ABCDEFGHJKMNPQRSTVWXYZ]{4}$'
               OR substr(entry_json ->> 'questionPoolId', 6, 1)
                    <> ple_private.crockford_checksum_character(
                        substr(entry_json ->> 'questionPoolId', 1, 4)
                        || substr(entry_json ->> 'questionPoolId', 7, 3)
                    )
            THEN
                RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Question Pool Assessment Entry is invalid';
            END IF;
            SELECT pool.question_pool_id INTO question_pool_id_value
              FROM ple_data.question_pool AS pool
             WHERE pool.question_pool_id = entry_json ->> 'questionPoolId';
            IF NOT FOUND OR NOT EXISTS (
                SELECT 1 FROM ple_data.question_pool_member AS member
                 WHERE member.question_pool_id = question_pool_id_value
                HAVING count(*) >= (entry_json ->> 'selectionCount')::integer
            ) THEN
                RAISE EXCEPTION USING ERRCODE = '22023',
                    MESSAGE = 'Question Pool Assessment Entry is invalid';
            END IF;
            IF NOT EXISTS (
                SELECT 1
                  FROM ple_data.assessment_entry AS existing
                  JOIN ple_data.assessment_entry_pool AS pool_entry
                    ON pool_entry.assessment_entry_id = existing.assessment_entry_id
                  JOIN ple_data.assessment_question_pool_fork AS owned
                    ON owned.assessment_entry_id = existing.assessment_entry_id
                   AND owned.assessment_id = existing.assessment_id
                   AND owned.question_pool_id = pool_entry.question_pool_id
                 WHERE existing.assessment_id = p_assessment_id
                   AND existing.assessment_entry_id = entry_id
                   AND existing.entry_kind = 'question_pool'
                   AND pool_entry.question_pool_id = question_pool_id_value
            ) AND EXISTS (
                SELECT 1
                  FROM ple_data.question_pool_member AS member
                  JOIN ple_data.question_revision AS revision
                    ON revision.published_question_id = member.published_question_id
                   AND revision.revision_number = member.question_revision_number
                 WHERE member.question_pool_id = question_pool_id_value
                   AND NOT ple_private.question_backend_is_supported_for_production(revision.backend)
            ) THEN
                RAISE EXCEPTION USING ERRCODE = '22023',
                    MESSAGE = 'New Assessment Question Pools require current production Question Backends';
            END IF;
            -- An Assessment Pool is its own immutable fork lineage.  The
            -- ordinary complete-content save may alter only Entry policy;
            -- it cannot turn an Entry into a Pool, rebind it to another
            -- lineage, or select an arbitrary historical/current Revision.
            -- Membership changes use the append-only fork command below.
            IF NOT EXISTS (
                SELECT 1
                  FROM ple_data.assessment_entry AS existing
                  JOIN ple_data.assessment_entry_pool AS pool_entry
                    ON pool_entry.assessment_entry_id = existing.assessment_entry_id
                  JOIN ple_data.assessment_question_pool_fork AS owned
                    ON owned.assessment_entry_id = existing.assessment_entry_id
                   AND owned.assessment_id = existing.assessment_id
                   AND owned.question_pool_id = pool_entry.question_pool_id
                 WHERE existing.assessment_id = p_assessment_id
                   AND existing.assessment_entry_id = entry_id
                   AND existing.entry_kind = 'question_pool'
                   AND pool_entry.question_pool_id = question_pool_id_value
            ) THEN
                RAISE EXCEPTION USING ERRCODE = '22023',
                    MESSAGE = 'Assessment Question Pool membership requires an immutable fork command';
            END IF;
            UPDATE ple_data.assessment_entry AS target
               SET authored_position = COALESCE((entry_json ->> 'authoredPosition')::integer, 0),
                   availability = entry_json ->> 'availability',
                   scoring_rule = entry_json ->> 'scoringRule',
                   question_attempt_limit = NULLIF(entry_json ->> 'questionAttemptLimit', '')::integer,
                   question_attempt_time_limit_seconds = NULLIF(entry_json ->> 'questionAttemptTimeLimitSeconds', '')::integer,
                   question_attempt_grace_seconds = NULLIF(entry_json ->> 'questionAttemptGraceSeconds', '')::integer
             WHERE target.assessment_id = p_assessment_id AND target.assessment_entry_id = entry_id
               AND ROW(target.authored_position, target.availability, target.scoring_rule,
                       target.question_attempt_limit, target.question_attempt_time_limit_seconds,
                       target.question_attempt_grace_seconds) IS DISTINCT FROM ROW(
                       COALESCE((entry_json ->> 'authoredPosition')::integer, 0),
                       entry_json ->> 'availability', entry_json ->> 'scoringRule',
                       NULLIF(entry_json ->> 'questionAttemptLimit', '')::integer,
                       NULLIF(entry_json ->> 'questionAttemptTimeLimitSeconds', '')::integer,
                       NULLIF(entry_json ->> 'questionAttemptGraceSeconds', '')::integer);
            GET DIAGNOSTICS row_count = ROW_COUNT;
            changed := changed OR row_count > 0;
            UPDATE ple_data.assessment_entry_pool AS pool_entry
               SET selection_count = (entry_json ->> 'selectionCount')::integer,
                   points_per_item = (entry_json ->> 'pointsPerItem')::numeric,
                   selected_question_order = entry_json ->> 'selectedQuestionOrder'
             WHERE pool_entry.assessment_entry_id = entry_id
               AND pool_entry.assessment_id = p_assessment_id
               AND ROW(pool_entry.selection_count, pool_entry.points_per_item,
                       pool_entry.selected_question_order) IS DISTINCT FROM ROW(
                       (entry_json ->> 'selectionCount')::integer,
                       (entry_json ->> 'pointsPerItem')::numeric,
                       entry_json ->> 'selectedQuestionOrder');
            GET DIAGNOSTICS row_count = ROW_COUNT;
            changed := changed OR row_count > 0;
        END IF;
    END LOOP;
    UPDATE ple_data.assessment_entry SET availability = 'retired'
     WHERE assessment_id = p_assessment_id AND availability <> 'retired'
       AND NOT (assessment_entry_id = ANY (entry_ids));
    GET DIAGNOSTICS row_count = ROW_COUNT;
    PERFORM ple_private.ensure_assessment_entry_snapshot(
        entry.entry_kind, entry.scoring_rule,
        COALESCE(question.points_possible, pool_entry.points_per_item),
        question.published_question_id, question.question_revision_number,
        pool_entry.question_pool_id,
        entry.question_attempt_limit, entry.question_attempt_time_limit_seconds,
        entry.question_attempt_grace_seconds
    )
      FROM ple_data.assessment_entry AS entry
      LEFT JOIN ple_data.assessment_entry_question AS question
        ON question.assessment_entry_id = entry.assessment_entry_id
      LEFT JOIN ple_data.assessment_entry_pool AS pool_entry
        ON pool_entry.assessment_entry_id = entry.assessment_entry_id
     WHERE entry.assessment_id = p_assessment_id
       AND entry.assessment_entry_id = ANY (entry_ids);
    RETURN changed OR row_count > 0;
END
$$;



-- The mutable scalar values are an exact object with the Assessment table's
-- non-identity fields.  Keeping the tagged child collection separate avoids
-- generic snapshot persistence while one operation validates the full result.
CREATE FUNCTION ple_data.save_assessment(
    p_course_instance_id text,
    p_assessment_id text,
    p_expected_assessment_edit_number bigint,
    p_values jsonb,
    p_entries jsonb
) RETURNS TABLE (
    assessment_id text, assessment_edit_number bigint,
    assessment_status text, assessment_title text, assessment_instructions text
)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
DECLARE
    course_row ple_data.course_instance%ROWTYPE;
    current_assessment ple_data.assessment%ROWTYPE;
    current_snapshot ple_data.assessment_policy_snapshot%ROWTYPE;
    candidate ple_data.assessment_policy_snapshot%ROWTYPE;
    snapshot_id ple_data.sha256_digest;
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
    IF p_course_instance_id IS NULL
       OR p_assessment_id IS NULL
       OR p_expected_assessment_edit_number IS NULL OR p_expected_assessment_edit_number <= 0
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
     WHERE course.course_instance_id = p_course_instance_id
       AND ple_api.current_session_account_is_course_instructor(course.course_instance_id)
     FOR UPDATE;
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Assessment is unavailable';
    END IF;
    SELECT assessment.* INTO current_assessment
      FROM ple_data.assessment AS assessment
     WHERE assessment.course_instance_id = course_row.course_instance_id
       AND assessment.assessment_id = p_assessment_id
     FOR UPDATE;
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Assessment is unavailable';
    END IF;
    IF current_assessment.assessment_edit_number IS DISTINCT FROM p_expected_assessment_edit_number THEN
        RAISE EXCEPTION USING ERRCODE = '40001', MESSAGE = 'Assessment Edit Number is stale';
    END IF;
    SELECT * INTO current_snapshot
      FROM ple_data.assessment_policy_snapshot
     WHERE assessment_policy_snapshot_id = current_assessment.assessment_policy_snapshot_id;
    SELECT * INTO candidate FROM jsonb_populate_record(current_snapshot, p_values);
    IF candidate.due_at > course_row.active_until_at THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Assessment due date is after the Course Active cutoff';
    END IF;
    snapshot_id := ple_private.ensure_assessment_policy_snapshot(
        candidate.assessment_title, candidate.assessment_instructions,
        candidate.available_at, candidate.due_at, candidate.closes_at,
        candidate.assessment_attempt_time_limit_seconds, candidate.assessment_attempt_limit,
        candidate.late_work_rule, candidate.question_variation_rule,
        candidate.assessment_question_order_rule, candidate.feedback_score,
        candidate.feedback_per_item_correctness, candidate.feedback_submitted_response,
        candidate.feedback_question_answer, candidate.feedback_question_answer_explanation,
        candidate.feedback_class_statistics, current_assessment.assessment_type
    );
    values_changed := snapshot_id IS DISTINCT FROM current_assessment.assessment_policy_snapshot_id;
    entries_changed := ple_data.replace_assessment_entries(current_assessment.assessment_id, p_entries);
    IF values_changed OR entries_changed THEN
        UPDATE ple_data.assessment AS updated SET
            assessment_policy_snapshot_id = snapshot_id,
            assessment_edit_number = updated.assessment_edit_number + 1,
            updated_at = clock_timestamp()
         WHERE updated.assessment_id = current_assessment.assessment_id
        RETURNING updated.assessment_id, updated.assessment_edit_number,
            updated.assessment_status
          INTO assessment_id, assessment_edit_number, assessment_status;
        assessment_title := candidate.assessment_title;
        assessment_instructions := candidate.assessment_instructions;
        IF assessment_status = 'released' THEN
            PERFORM ple_data.validate_assessment_release(
                current_assessment.assessment_id,
                transaction_timestamp(),
                candidate.due_at IS DISTINCT FROM current_snapshot.due_at
            );
        END IF;
        PERFORM ple_data.synchronize_course_assessment_deadline(course_row.course_instance_id);
    ELSE
        assessment_id := current_assessment.assessment_id;
        assessment_edit_number := current_assessment.assessment_edit_number;
        assessment_status := current_assessment.assessment_status;
        assessment_title := current_snapshot.assessment_title;
        assessment_instructions := current_snapshot.assessment_instructions;
    END IF;
    RETURN NEXT;
END
$$;

CREATE FUNCTION ple_data.save_assessment_inline(
    p_course_instance_id text, p_assessment_id text,
    p_expected_assessment_edit_number bigint, p_title text, p_due_at timestamptz
) RETURNS TABLE (
    assessment_id text, assessment_title text, due_at_millis bigint,
    assessment_status text, assessment_edit_number bigint
)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
DECLARE
    course_row ple_data.course_instance%ROWTYPE;
    assessment_row ple_data.assessment%ROWTYPE;
    current_snapshot ple_data.assessment_policy_snapshot%ROWTYPE;
    snapshot_id ple_data.sha256_digest;
BEGIN
    IF p_course_instance_id IS NULL
       OR p_assessment_id IS NULL
       OR p_expected_assessment_edit_number IS NULL OR p_expected_assessment_edit_number <= 0 OR p_title IS NULL THEN
        RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Assessment inline save is invalid';
    END IF;
    SELECT course.* INTO course_row
      FROM ple_data.course_instance AS course
     WHERE course.course_instance_id = p_course_instance_id
       AND ple_api.current_session_account_is_course_instructor(course.course_instance_id)
     FOR UPDATE;
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Assessment is unavailable';
    END IF;
    SELECT assessment.* INTO assessment_row
      FROM ple_data.assessment AS assessment
     WHERE assessment.course_instance_id = course_row.course_instance_id
       AND assessment.assessment_id = p_assessment_id
     FOR UPDATE;
    IF NOT FOUND THEN RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Assessment is unavailable'; END IF;
    IF assessment_row.assessment_edit_number IS DISTINCT FROM p_expected_assessment_edit_number THEN
        RAISE EXCEPTION USING ERRCODE = '40001', MESSAGE = 'Assessment Edit Number is stale';
    END IF;
    IF p_due_at > course_row.active_until_at THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Assessment due date is after the Course Active cutoff';
    END IF;
    SELECT * INTO current_snapshot
      FROM ple_data.assessment_policy_snapshot
     WHERE assessment_policy_snapshot_id = assessment_row.assessment_policy_snapshot_id;
    IF ROW(current_snapshot.assessment_title, current_snapshot.due_at)
         IS DISTINCT FROM ROW(p_title, p_due_at) THEN
        snapshot_id := ple_private.ensure_assessment_policy_snapshot(
            p_title, current_snapshot.assessment_instructions,
            current_snapshot.available_at, p_due_at, current_snapshot.closes_at,
            current_snapshot.assessment_attempt_time_limit_seconds,
            current_snapshot.assessment_attempt_limit,
            current_snapshot.late_work_rule, current_snapshot.question_variation_rule,
            current_snapshot.assessment_question_order_rule, current_snapshot.feedback_score,
            current_snapshot.feedback_per_item_correctness,
            current_snapshot.feedback_submitted_response,
            current_snapshot.feedback_question_answer,
            current_snapshot.feedback_question_answer_explanation,
            current_snapshot.feedback_class_statistics, assessment_row.assessment_type
        );
        UPDATE ple_data.assessment AS updated SET
            assessment_policy_snapshot_id = snapshot_id,
            assessment_edit_number = updated.assessment_edit_number + 1,
            updated_at = clock_timestamp()
         WHERE updated.assessment_id = assessment_row.assessment_id
        RETURNING updated.assessment_id, updated.assessment_status, updated.assessment_edit_number
          INTO assessment_id, assessment_status, assessment_edit_number;
        assessment_title := p_title;
        due_at_millis := CASE WHEN p_due_at IS NULL THEN NULL
            ELSE floor(extract(epoch FROM p_due_at) * 1000)::bigint END;
        IF assessment_status = 'released' THEN
            PERFORM ple_data.validate_assessment_release(
                assessment_row.assessment_id,
                transaction_timestamp(),
                p_due_at IS DISTINCT FROM current_snapshot.due_at
            );
        END IF;
        PERFORM ple_data.synchronize_course_assessment_deadline(course_row.course_instance_id);
    ELSE
        assessment_id := assessment_row.assessment_id;
        assessment_title := current_snapshot.assessment_title;
        due_at_millis := CASE WHEN current_snapshot.due_at IS NULL THEN NULL
            ELSE floor(extract(epoch FROM current_snapshot.due_at) * 1000)::bigint END;
        assessment_status := assessment_row.assessment_status;
        assessment_edit_number := assessment_row.assessment_edit_number;
    END IF;
    RETURN NEXT;
END
$$;



-- Policy-only persistence keeps Question Entries and title outside this write boundary.
CREATE FUNCTION ple_data.save_assessment_policies(
    p_course_instance_id text, p_assessment_id text,
    p_expected_assessment_edit_number bigint, p_policies jsonb
) RETURNS TABLE (
    assessment_id text, assessment_edit_number bigint,
    assessment_status text, assessment_title text, assessment_instructions text
)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
DECLARE course_row ple_data.course_instance%ROWTYPE;
    current_assessment ple_data.assessment%ROWTYPE;
    current_snapshot ple_data.assessment_policy_snapshot%ROWTYPE;
    candidate ple_data.assessment_policy_snapshot%ROWTYPE;
    snapshot_id ple_data.sha256_digest;
    values_changed boolean;
    allowed_keys text[] := ARRAY[
        'assessment_instructions', 'available_at', 'due_at', 'closes_at',
        'assessment_attempt_time_limit_seconds', 'assessment_attempt_limit', 'late_work_rule',
        'question_variation_rule',
        'assessment_question_order_rule', 'feedback_score',
        'feedback_per_item_correctness', 'feedback_submitted_response',
        'feedback_question_answer', 'feedback_question_answer_explanation', 'feedback_class_statistics'];
BEGIN
    IF p_course_instance_id IS NULL
       OR p_assessment_id IS NULL
       OR p_expected_assessment_edit_number IS NULL OR p_expected_assessment_edit_number <= 0
       OR p_policies IS NULL OR jsonb_typeof(p_policies) <> 'object'
       OR EXISTS (SELECT 1 FROM jsonb_object_keys(p_policies) AS key WHERE key <> ALL (allowed_keys))
       OR EXISTS (SELECT 1 FROM unnest(allowed_keys) AS key WHERE NOT p_policies ? key) THEN
        RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Assessment policy save is invalid';
    END IF;
    SELECT course.* INTO course_row
      FROM ple_data.course_instance AS course
     WHERE course.course_instance_id = p_course_instance_id
       AND ple_api.current_session_account_is_course_instructor(course.course_instance_id)
     FOR UPDATE;
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Assessment is unavailable';
    END IF;
    SELECT assessment.* INTO current_assessment
      FROM ple_data.assessment AS assessment
     WHERE assessment.course_instance_id = course_row.course_instance_id
       AND assessment.assessment_id = p_assessment_id
     FOR UPDATE;
    IF NOT FOUND THEN RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Assessment is unavailable'; END IF;
    IF current_assessment.assessment_edit_number IS DISTINCT FROM p_expected_assessment_edit_number THEN
        RAISE EXCEPTION USING ERRCODE = '40001', MESSAGE = 'Assessment Edit Number is stale';
    END IF;
    SELECT * INTO current_snapshot
      FROM ple_data.assessment_policy_snapshot
     WHERE assessment_policy_snapshot_id = current_assessment.assessment_policy_snapshot_id;
    SELECT * INTO candidate FROM jsonb_populate_record(current_snapshot, p_policies);
    IF candidate.due_at > course_row.active_until_at THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Assessment due date is after the Course Active cutoff';
    END IF;
    snapshot_id := ple_private.ensure_assessment_policy_snapshot(
        candidate.assessment_title, candidate.assessment_instructions,
        candidate.available_at, candidate.due_at, candidate.closes_at,
        candidate.assessment_attempt_time_limit_seconds, candidate.assessment_attempt_limit,
        candidate.late_work_rule, candidate.question_variation_rule,
        candidate.assessment_question_order_rule, candidate.feedback_score,
        candidate.feedback_per_item_correctness, candidate.feedback_submitted_response,
        candidate.feedback_question_answer, candidate.feedback_question_answer_explanation,
        candidate.feedback_class_statistics, current_assessment.assessment_type
    );
    values_changed := snapshot_id IS DISTINCT FROM current_assessment.assessment_policy_snapshot_id;
    IF values_changed THEN
        UPDATE ple_data.assessment AS updated SET
          assessment_policy_snapshot_id = snapshot_id,
          assessment_edit_number = updated.assessment_edit_number + 1, updated_at = clock_timestamp()
          WHERE updated.assessment_id = current_assessment.assessment_id
        RETURNING updated.assessment_id, updated.assessment_edit_number, updated.assessment_status
          INTO assessment_id, assessment_edit_number, assessment_status;
        assessment_title := candidate.assessment_title;
        assessment_instructions := candidate.assessment_instructions;
        IF assessment_status = 'released' THEN
            PERFORM ple_data.validate_assessment_release(
                current_assessment.assessment_id,
                transaction_timestamp(),
                candidate.due_at IS DISTINCT FROM current_snapshot.due_at
            );
        END IF;
        PERFORM ple_data.synchronize_course_assessment_deadline(course_row.course_instance_id);
    ELSE
        assessment_id := current_assessment.assessment_id;
        assessment_edit_number := current_assessment.assessment_edit_number;
        assessment_status := current_assessment.assessment_status;
        assessment_title := current_snapshot.assessment_title;
        assessment_instructions := current_snapshot.assessment_instructions;
    END IF;
    RETURN NEXT;
END
$$;

CREATE FUNCTION ple_data.release_assessment(
    p_course_instance_id text,
    p_assessment_id text,
    p_expected_assessment_edit_number bigint
) RETURNS TABLE (
    assessment_id text, assessment_title text,
    assessment_status text, assessment_edit_number bigint
)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data AS $$
DECLARE
    course_row ple_data.course_instance%ROWTYPE;
    assessment_row ple_data.assessment%ROWTYPE;
BEGIN
    IF p_course_instance_id IS NULL
       OR p_assessment_id IS NULL
       OR p_expected_assessment_edit_number IS NULL OR p_expected_assessment_edit_number <= 0 THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Assessment is unavailable';
    END IF;
    SELECT course.* INTO course_row
      FROM ple_data.course_instance AS course
     WHERE course.course_instance_id = p_course_instance_id
       AND ple_api.current_session_account_is_course_instructor(course.course_instance_id)
     FOR UPDATE;
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Assessment is unavailable';
    END IF;
    SELECT assessment.* INTO assessment_row
      FROM ple_data.assessment AS assessment
     WHERE assessment.course_instance_id = course_row.course_instance_id
       AND assessment.assessment_id = p_assessment_id
     FOR UPDATE;
    IF NOT FOUND THEN RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Assessment is unavailable'; END IF;
    IF assessment_row.assessment_edit_number IS DISTINCT FROM p_expected_assessment_edit_number THEN
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
    RETURNING updated.assessment_id, updated.assessment_status, updated.assessment_edit_number
      INTO assessment_id, assessment_status, assessment_edit_number;
    SELECT policy.assessment_title INTO assessment_title
      FROM ple_data.assessment_policy_snapshot AS policy
     WHERE policy.assessment_policy_snapshot_id = assessment_row.assessment_policy_snapshot_id;
    PERFORM ple_data.synchronize_course_assessment_deadline(course_row.course_instance_id);
    RETURN NEXT;
END
$$;

CREATE TRIGGER assessment_edit_is_exact
BEFORE UPDATE ON ple_data.assessment
FOR EACH ROW EXECUTE FUNCTION ple_data.enforce_assessment_edit();

CREATE TRIGGER assessment_public_id_is_minted
BEFORE INSERT ON ple_data.assessment
FOR EACH ROW EXECUTE FUNCTION ple_private.assign_public_id('A');

CREATE TRIGGER assessment_question_pool_fork_requires_fork_provenance
BEFORE INSERT OR UPDATE ON ple_data.assessment_question_pool_fork
FOR EACH ROW EXECUTE FUNCTION ple_data.validate_assessment_question_pool_fork();

