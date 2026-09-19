-- Functions, triggers, and views from assessment_attempt_start.sql.

SET LOCAL ROLE ple_private_owner;

-- One locked authority decides whether a Student resumes retained work or may
-- start a new Assessment Attempt. Preparation and persistence call the same
-- gate in one transaction, so current Assessment policy cannot change between
-- Question selection and immutable evidence creation.
CREATE FUNCTION ple_private.assessment_attempt_start_gate(
    p_student_record_id uuid,
    p_assessment_id text
) RETURNS TABLE (
    resumable_assessment_attempt_id uuid,
    resumable_assessment_attempt_number integer,
    evaluated_at timestamptz,
    effective_assessment_attempt_limit integer
)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
DECLARE assessment_row ple_data.assessment%ROWTYPE;
DECLARE policy_row ple_data.assessment_policy_snapshot%ROWTYPE;
DECLARE accommodation_row ple_private.student_assessment_accommodation%ROWTYPE;
DECLARE existing_assessment_attempt ple_private.assessment_attempt%ROWTYPE;
DECLARE account_id text := ple_api.current_session_account_id();
DECLARE started_assessment_attempt_count integer;
DECLARE start_decision_value text;
BEGIN
    IF p_student_record_id IS NULL OR p_assessment_id IS NULL THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Assessment Attempt start is unavailable';
    END IF;
    -- Every Student Work mutator acquires this Assessment row first. The
    -- guarded Unrelease procedure uses the identical first lock.
    PERFORM ple_private.lock_assessment_for_student_work(p_assessment_id);
    SELECT * INTO assessment_row FROM ple_data.assessment
     WHERE assessment_id = p_assessment_id;
    SELECT * INTO policy_row FROM ple_data.assessment_policy_snapshot
     WHERE assessment_policy_snapshot_id = assessment_row.assessment_policy_snapshot_id;
    IF NOT FOUND OR assessment_row.assessment_status <> 'released'
       OR account_id IS NULL
       OR NOT ple_api.current_session_account_owns_student_record(
           assessment_row.course_instance_id, p_student_record_id
       ) THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Assessment Attempt start is unavailable';
    END IF;
    SELECT * INTO accommodation_row
      FROM ple_private.student_assessment_accommodation
     WHERE student_record_id = p_student_record_id
       AND assessment_id = p_assessment_id;
    effective_assessment_attempt_limit := CASE
        WHEN assessment_row.assessment_type IN ('quiz', 'exam') THEN 1
        ELSE COALESCE(
            accommodation_row.assessment_attempt_limit,
            policy_row.assessment_attempt_limit
        )
    END;
    evaluated_at := pg_catalog.clock_timestamp();
    SELECT count(*)::integer INTO started_assessment_attempt_count
      FROM ple_private.assessment_attempt AS assessment_attempt
     WHERE assessment_attempt.student_record_id = p_student_record_id
       AND assessment_attempt.assessment_id = p_assessment_id;
    start_decision_value := ple_private.assessment_start_decision(
        assessment_row.assessment_status,
        COALESCE(accommodation_row.available_at, policy_row.available_at),
        COALESCE(accommodation_row.due_at, policy_row.due_at),
        COALESCE(accommodation_row.closes_at, policy_row.closes_at),
        effective_assessment_attempt_limit,
        started_assessment_attempt_count,
        policy_row.late_work_rule,
        evaluated_at
    );
    IF start_decision_value IN ('closed', 'not_yet_available') THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Assessment Attempt start is outside its effective availability';
    END IF;
    SELECT * INTO existing_assessment_attempt
      FROM ple_private.assessment_attempt AS candidate
     WHERE candidate.student_record_id = p_student_record_id
       AND candidate.assessment_id = p_assessment_id
       AND NOT EXISTS (
           SELECT 1 FROM ple_private.assessment_submission AS submission
            WHERE submission.assessment_attempt_id = candidate.assessment_attempt_id
       )
       AND (candidate.expires_at IS NULL OR candidate.expires_at > evaluated_at)
     ORDER BY candidate.assessment_attempt_number DESC LIMIT 1;
    -- Resume the eligible active Attempt before a current limit or late-work
    -- refusal can authorize a new Attempt; its retained deadline is unchanged.
    IF FOUND THEN
        resumable_assessment_attempt_id := existing_assessment_attempt.assessment_attempt_id;
        resumable_assessment_attempt_number := existing_assessment_attempt.assessment_attempt_number;
        RETURN NEXT;
        RETURN;
    END IF;
    IF start_decision_value = 'attempt_limit_reached' THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Assessment Attempt limit is reached';
    ELSIF start_decision_value = 'late_work_refused' THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Assessment Attempt start is outside its effective availability';
    END IF;
    -- ASVS 2.2.2, 2.3.2: new work must resolve a positive finite base.
    IF ple_data.assessment_effective_base_duration_seconds(p_assessment_id) IS NULL THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Assessment Attempt requires 1 to 250 Questions';
    END IF;
    resumable_assessment_attempt_id := NULL;
    resumable_assessment_attempt_number := NULL;
    RETURN NEXT;
END $$;

CREATE FUNCTION ple_private.start_assessment_attempt(
    p_assessment_attempt_id uuid,
    p_student_record_id uuid,
    p_assessment_id text,
    p_selections jsonb,
    p_issued_questions jsonb
) RETURNS TABLE (assessment_attempt_id uuid, assessment_attempt_number integer, resumed boolean)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
DECLARE assessment_row ple_data.assessment%ROWTYPE;
DECLARE policy_row ple_data.assessment_policy_snapshot%ROWTYPE;
DECLARE now_value timestamptz;
DECLARE effective_assessment_attempt_limit integer;
DECLARE effective_duration_seconds integer;
DECLARE start_gate record;
DECLARE next_assessment_attempt_number integer;
DECLARE selection jsonb;
DECLARE issued jsonb;
DECLARE entry_row record;
DECLARE entry_snapshot_id ple_data.sha256_digest;
DECLARE accommodation_row ple_private.student_assessment_accommodation%ROWTYPE;
DECLARE selection_id uuid;
DECLARE selection_entry_id uuid;
DECLARE issued_entry_id uuid;
DECLARE issued_selection_id uuid;
DECLARE issued_member_position integer;
DECLARE issued_source ple_private.question_revision_source_binding%ROWTYPE;
BEGIN
    IF p_assessment_attempt_id IS NULL OR p_student_record_id IS NULL OR p_assessment_id IS NULL
       OR jsonb_typeof(p_selections) <> 'array' OR jsonb_typeof(p_issued_questions) <> 'array' THEN
        RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Assessment Attempt start arguments are invalid';
    END IF;

    -- ASVS 2.3.3, 15.4.2: authorization, timing checks, and creation stay in
    -- one locked transaction so the decision cannot race the accepted write.
    SELECT * INTO start_gate FROM ple_private.assessment_attempt_start_gate(
        p_student_record_id, p_assessment_id
    );
    IF start_gate.resumable_assessment_attempt_id IS NOT NULL THEN
        assessment_attempt_id := start_gate.resumable_assessment_attempt_id;
        assessment_attempt_number := start_gate.resumable_assessment_attempt_number;
        resumed := true;
        RETURN NEXT;
        RETURN;
    END IF;
    now_value := start_gate.evaluated_at;
    effective_assessment_attempt_limit := start_gate.effective_assessment_attempt_limit;
    SELECT * INTO assessment_row FROM ple_data.assessment WHERE assessment_id = p_assessment_id;
    SELECT * INTO policy_row FROM ple_data.assessment_policy_snapshot
     WHERE assessment_policy_snapshot_id = assessment_row.assessment_policy_snapshot_id;
    SELECT * INTO accommodation_row
      FROM ple_private.student_assessment_accommodation
     WHERE student_record_id = p_student_record_id AND assessment_id = p_assessment_id;
    effective_duration_seconds := ple_private.assessment_effective_duration_seconds(
        p_assessment_id, accommodation_row.time_multiplier
    );
    IF effective_duration_seconds IS NULL THEN
        RAISE EXCEPTION USING ERRCODE = '23514', MESSAGE = 'Assessment Attempt requires a finite duration';
    END IF;
    SELECT COALESCE(max(assessment_attempt_row.assessment_attempt_number), 0) + 1 INTO next_assessment_attempt_number
      FROM ple_private.assessment_attempt AS assessment_attempt_row
     WHERE assessment_attempt_row.student_record_id = p_student_record_id
       AND assessment_attempt_row.assessment_id = p_assessment_id;
    IF effective_assessment_attempt_limit IS NOT NULL
       AND next_assessment_attempt_number > effective_assessment_attempt_limit THEN
        RAISE EXCEPTION USING ERRCODE = '23514', MESSAGE = 'Assessment Attempt limit is reached';
    END IF;
    IF NOT ple_data.student_assessment_has_course_scope(
        p_student_record_id, p_assessment_id
    ) THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Assessment Attempt requires a Student and Assessment in one Course';
    END IF;
    INSERT INTO ple_private.assessment_attempt(
        course_instance_id, assessment_attempt_id, student_record_id, assessment_id, assessment_attempt_number, started_at, expires_at,
        assessment_policy_snapshot_id,
        schedule_accommodation_id, schedule_accommodation_edit_number,
        time_limit_accommodation_id, time_limit_accommodation_edit_number,
        assessment_attempt_limit_accommodation_id, assessment_attempt_limit_accommodation_edit_number
    ) VALUES (
        assessment_row.course_instance_id, p_assessment_attempt_id, p_student_record_id, p_assessment_id, next_assessment_attempt_number, now_value,
        -- ASVS 2.3.2, 8.3.1: one immutable server-owned expiration applies
        -- every effective timing limit that authorizes Student interaction.
        least(
            now_value + pg_catalog.make_interval(secs => effective_duration_seconds),
            COALESCE(accommodation_row.closes_at, policy_row.closes_at),
            CASE WHEN policy_row.late_work_rule = 'reject'
                THEN COALESCE(accommodation_row.due_at, policy_row.due_at)
            END
        ),
        assessment_row.assessment_policy_snapshot_id,
        CASE WHEN accommodation_row.available_at IS NOT NULL OR accommodation_row.due_at IS NOT NULL
               OR accommodation_row.closes_at IS NOT NULL THEN accommodation_row.accommodation_id END,
        CASE WHEN accommodation_row.available_at IS NOT NULL OR accommodation_row.due_at IS NOT NULL
               OR accommodation_row.closes_at IS NOT NULL THEN accommodation_row.accommodation_edit_number END,
        CASE WHEN accommodation_row.time_multiplier IS NOT NULL THEN accommodation_row.accommodation_id END,
        CASE WHEN accommodation_row.time_multiplier IS NOT NULL THEN accommodation_row.accommodation_edit_number END,
        CASE WHEN assessment_row.assessment_type NOT IN ('quiz', 'exam')
                  AND accommodation_row.assessment_attempt_limit IS NOT NULL
             THEN accommodation_row.accommodation_id END,
        CASE WHEN assessment_row.assessment_type NOT IN ('quiz', 'exam')
                  AND accommodation_row.assessment_attempt_limit IS NOT NULL
             THEN accommodation_row.accommodation_edit_number END
    );

    FOR selection IN SELECT value FROM jsonb_array_elements(p_selections) LOOP
        selection_id := (selection ->> 'question_pool_selection_id')::uuid;
        selection_entry_id := (selection ->> 'assessment_entry_id')::uuid;
        SELECT entry.assessment_entry_id, entry.assessment_id, entry.entry_kind,
               entry.availability, entry.scoring_rule, entry.authored_position,
               entry.question_attempt_limit, entry.question_attempt_time_limit_seconds,
               entry.question_attempt_grace_seconds,
               pool_entry.question_pool_id, pool.question_pool_edit_number AS question_pool_edit_number,
               pool_entry.selection_count, pool_entry.points_per_item,
               pool_entry.selected_question_order
          INTO entry_row
          FROM ple_data.assessment_entry AS entry
          JOIN ple_data.assessment_entry_pool AS pool_entry
            ON pool_entry.assessment_entry_id = entry.assessment_entry_id
          JOIN ple_data.question_pool AS pool
            ON pool.question_pool_id = pool_entry.question_pool_id
         WHERE entry.assessment_entry_id = selection_entry_id AND entry.assessment_id = p_assessment_id
           AND entry.entry_kind = 'question_pool' AND entry.availability = 'available';
        IF NOT FOUND OR selection_id IS NULL
           OR jsonb_typeof(selection -> 'selected_items') <> 'array'
           OR jsonb_array_length(selection -> 'selected_items') <> entry_row.selection_count THEN
            RAISE EXCEPTION USING ERRCODE = '23514', MESSAGE = 'Question Pool Selection is not current released Assessment content';
        END IF;
        INSERT INTO ple_private.question_pool_selection(
            course_instance_id, question_pool_selection_id, assessment_attempt_id, assessment_entry_id,
            question_pool_id, question_pool_edit_number, created_at
        ) VALUES (
            assessment_row.course_instance_id, selection_id, p_assessment_attempt_id, selection_entry_id,
            entry_row.question_pool_id, entry_row.question_pool_edit_number, now_value
        );
        INSERT INTO ple_private.question_pool_selected_item(
            course_instance_id, question_pool_selection_id, member_position, selection_position, published_question_id, revision_number
        )
        SELECT assessment_row.course_instance_id, selection_id, (item.value #>> '{}')::integer, item.ordinality - 1,
               pool.published_question_id, pool.question_revision_number
          FROM jsonb_array_elements(selection -> 'selected_items') WITH ORDINALITY AS item(value, ordinality)
          JOIN ple_data.question_pool_member AS pool
            ON pool.question_pool_id = entry_row.question_pool_id
           AND pool.member_position = (item.value #>> '{}')::integer;
        IF (SELECT count(*) FROM ple_private.question_pool_selected_item WHERE question_pool_selection_id = selection_id)
             <> entry_row.selection_count THEN
            RAISE EXCEPTION USING ERRCODE = '23514', MESSAGE = 'Question Pool Selection contains an unavailable or repeated Item';
        END IF;
    END LOOP;

    FOR issued IN SELECT value FROM jsonb_array_elements(p_issued_questions) LOOP
        issued_entry_id := (issued ->> 'assessment_entry_id')::uuid;
        issued_selection_id := NULLIF(issued ->> 'question_pool_selection_id', '')::uuid;
        issued_member_position := NULLIF(issued ->> 'question_pool_member_position', '')::integer;
        SELECT entry.assessment_entry_id, entry.assessment_id, entry.entry_kind,
               entry.availability, entry.scoring_rule, entry.authored_position,
               entry.question_attempt_limit, entry.question_attempt_time_limit_seconds,
               entry.question_attempt_grace_seconds,
               question.published_question_id, question.question_revision_number,
               question.points_possible,
               pool_entry.question_pool_id,
               pool_entry.selection_count, pool_entry.points_per_item
          INTO entry_row
          FROM ple_data.assessment_entry AS entry
          LEFT JOIN ple_data.assessment_entry_question AS question
            ON question.assessment_entry_id = entry.assessment_entry_id
          LEFT JOIN ple_data.assessment_entry_pool AS pool_entry
            ON pool_entry.assessment_entry_id = entry.assessment_entry_id
         WHERE entry.assessment_entry_id = issued_entry_id AND entry.assessment_id = p_assessment_id
           AND entry.availability = 'available';
        IF NOT FOUND OR (issued ->> 'issued_question_id') IS NULL THEN
            RAISE EXCEPTION USING ERRCODE = '23514', MESSAGE = 'Issued Question is not current released Assessment content';
        END IF;
        IF entry_row.entry_kind = 'fixed_question' THEN
            IF issued_selection_id IS NOT NULL OR issued_member_position IS NOT NULL
               OR issued ->> 'published_question_id' <> entry_row.published_question_id
               OR (issued ->> 'revision_number')::integer <> entry_row.question_revision_number THEN
                RAISE EXCEPTION USING ERRCODE = '23514', MESSAGE = 'Fixed Issued Question does not match its current Assessment Entry';
            END IF;
        ELSE
            IF issued_selection_id IS NULL OR issued_member_position IS NULL OR NOT EXISTS (
                SELECT 1 FROM ple_private.question_pool_selected_item AS selected
                 WHERE selected.question_pool_selection_id = issued_selection_id
                   AND selected.member_position = issued_member_position
                   AND selected.published_question_id = issued ->> 'published_question_id'
                   AND selected.revision_number = (issued ->> 'revision_number')::integer
            ) THEN
                RAISE EXCEPTION USING ERRCODE = '23514', MESSAGE = 'Pooled Issued Question does not match its selected Item';
            END IF;
        END IF;
        SELECT * INTO issued_source
          FROM ple_private.question_revision_source_binding AS source
         WHERE source.published_question_id = issued ->> 'published_question_id'
           AND source.revision_number = (issued ->> 'revision_number')::integer;
        IF NOT FOUND OR NOT ple_private.question_backend_is_supported_for_production(issued_source.backend)
           OR (issued_source.backend = 'ple' AND (issued ->> 'question_seed') IS NOT NULL)
           OR (issued_source.backend = 'webwork'
               AND ((issued ->> 'question_seed') IS NULL
                    OR (issued ->> 'question_seed') !~ '^(0|[1-9][0-9]{0,19})$'
                    OR (issued ->> 'question_seed')::numeric > 18446744073709551615)) THEN
            RAISE EXCEPTION USING ERRCODE = '23514',
                MESSAGE = 'Issued Question reproduction does not match its source backend';
        END IF;
        entry_snapshot_id := ple_private.ensure_assessment_entry_snapshot(
            entry_row.entry_kind, entry_row.scoring_rule,
            CASE WHEN entry_row.entry_kind = 'fixed_question'
                 THEN entry_row.points_possible ELSE entry_row.points_per_item END,
            entry_row.published_question_id, entry_row.question_revision_number,
            entry_row.question_pool_id,
            entry_row.question_attempt_limit, entry_row.question_attempt_time_limit_seconds,
            entry_row.question_attempt_grace_seconds
        );
        INSERT INTO ple_private.issued_question(
            course_instance_id, issued_question_id, assessment_attempt_id, assessment_entry_id,
            assessment_entry_snapshot_id,
            assessment_content_entry_index, issued_position, published_question_id, revision_number,
            question_seed, question_statistics_eligibility,
            question_pool_selection_id, question_pool_member_position
        ) VALUES (
            assessment_row.course_instance_id, (issued ->> 'issued_question_id')::uuid, p_assessment_attempt_id, issued_entry_id,
            entry_snapshot_id,
            entry_row.authored_position, (issued ->> 'issued_position')::integer,
            issued ->> 'published_question_id', (issued ->> 'revision_number')::integer,
            (issued ->> 'question_seed')::numeric,
            entry_row.scoring_rule <> 'excluded',
            issued_selection_id, issued_member_position
        );
    END LOOP;
    IF EXISTS (
        SELECT 1 FROM ple_data.assessment_entry AS entry
         WHERE entry.assessment_id = p_assessment_id AND entry.availability = 'available'
           AND entry.entry_kind = 'question_pool'
           AND NOT EXISTS (
               SELECT 1 FROM ple_private.question_pool_selection AS selection
                WHERE selection.assessment_attempt_id = p_assessment_attempt_id
                  AND selection.assessment_entry_id = entry.assessment_entry_id
           )
    ) OR EXISTS (
        SELECT 1 FROM ple_data.assessment_entry AS entry
          JOIN ple_data.assessment_entry_question AS question
            ON question.assessment_entry_id = entry.assessment_entry_id
         WHERE entry.assessment_id = p_assessment_id AND entry.availability = 'available'
           AND entry.entry_kind = 'fixed_question'
           AND NOT EXISTS (
               SELECT 1 FROM ple_private.issued_question AS issued
                WHERE issued.assessment_attempt_id = p_assessment_attempt_id
                  AND issued.assessment_entry_id = entry.assessment_entry_id
                  AND issued.published_question_id = question.published_question_id
                  AND issued.revision_number = question.question_revision_number
           )
    ) THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Assessment Attempt must retain the complete current released issue set';
    END IF;
    IF NOT EXISTS (
        SELECT 1
          FROM ple_private.issued_question AS retained_issued_question
         WHERE retained_issued_question.assessment_attempt_id = p_assessment_attempt_id
    ) THEN
        RAISE EXCEPTION USING ERRCODE = '23514', MESSAGE = 'Assessment Attempt requires issued Questions';
    END IF;
    assessment_attempt_id := p_assessment_attempt_id;
    assessment_attempt_number := next_assessment_attempt_number;
    resumed := false;
    RETURN NEXT;
END $$;

-- This prepares, but does not mint, a Student Work root.  The application
-- keeps cryptographic randomness at its existing boundary, selects an exact
-- set of available pool Items, and immediately calls start_assessment_attempt
-- in the same transaction.  Taking the Assessment lock here preserves the
-- one Student Work lock order while the selection is prepared; the canonical
-- start function repeats the current-state checks before it writes evidence.
-- ASVS 2.2.1, 2.3.1, and 8.2.1: the authenticated database boundary resolves
-- the Student and route references rather than accepting either identity from
-- the browser.
CREATE FUNCTION ple_private.prepare_current_assessment_attempt_start_decision(
    p_course_instance_id text,
    p_assessment_id text
) RETURNS TABLE (
    resumable_assessment_attempt_id uuid,
    resumable_assessment_attempt_number integer
)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
DECLARE assessment_row ple_data.assessment%ROWTYPE;
DECLARE student_record_id_value uuid;
BEGIN
    IF p_course_instance_id IS NULL
       OR p_assessment_id IS NULL THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Assessment Attempt start is unavailable';
    END IF;
    SELECT assessment.* INTO assessment_row
      FROM ple_data.assessment AS assessment
     WHERE assessment.assessment_id = p_assessment_id;
    IF NOT FOUND OR ple_api.course_instance_id_for_assessment_attempt(
        assessment_row.course_instance_id
    ) IS DISTINCT FROM p_course_instance_id THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Assessment Attempt start is unavailable';
    END IF;
    SELECT ple_api.current_session_student_record_id(assessment_row.course_instance_id)
      INTO student_record_id_value;
    RETURN QUERY
    SELECT gate.resumable_assessment_attempt_id,
           gate.resumable_assessment_attempt_number
      FROM ple_private.assessment_attempt_start_gate(
          student_record_id_value, assessment_row.assessment_id
      ) AS gate;
END $$;

CREATE FUNCTION ple_private.prepare_current_assessment_attempt_start(
    p_course_instance_id text,
    p_assessment_id text
) RETURNS TABLE (
    student_record_id uuid,
    assessment_id text,
    assessment_entry_id uuid,
    entry_kind text,
    authored_position integer,
    fixed_question_id text,
    fixed_revision_number integer,
    question_pool_id text,
    question_pool_edit_number bigint,
    member_position integer,
    pool_question_id text,
    pool_question_revision_number integer,
    question_backend text,
    selection_count integer,
    pool_selection_rule text,
    question_variation_rule text,
    assessment_question_order_rule text
)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
DECLARE assessment_row ple_data.assessment%ROWTYPE;
DECLARE policy_row ple_data.assessment_policy_snapshot%ROWTYPE;
DECLARE student_record_id_value uuid;
BEGIN
    IF p_course_instance_id IS NULL
       OR p_assessment_id IS NULL THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Assessment Attempt start is unavailable';
    END IF;

    SELECT assessment.* INTO assessment_row
      FROM ple_data.assessment AS assessment
     WHERE assessment.assessment_id = p_assessment_id;
    IF NOT FOUND OR ple_api.course_instance_id_for_assessment_attempt(
        assessment_row.course_instance_id
    ) IS DISTINCT FROM p_course_instance_id THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Assessment Attempt start is unavailable';
    END IF;

    -- This is the first Student Work lock.  The route lookup above performs
    -- no child lock; all subsequent selection reads are stable until the
    -- surrounding start transaction commits or rolls back.
    PERFORM ple_private.lock_assessment_for_student_work(assessment_row.assessment_id);
    SELECT assessment.* INTO assessment_row
      FROM ple_data.assessment AS assessment
     WHERE assessment.assessment_id = assessment_row.assessment_id;
    SELECT * INTO policy_row FROM ple_data.assessment_policy_snapshot
     WHERE assessment_policy_snapshot_id = assessment_row.assessment_policy_snapshot_id;
    SELECT ple_api.current_session_student_record_id(assessment_row.course_instance_id)
      INTO student_record_id_value;
    IF assessment_row.assessment_status <> 'released' OR NOT FOUND
       OR NOT ple_api.current_session_account_owns_student_record(
           assessment_row.course_instance_id, student_record_id_value
       ) THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Assessment Attempt start is unavailable';
    END IF;

    RETURN QUERY
    SELECT student_record_id_value,
           assessment_row.assessment_id,
           entry.assessment_entry_id,
           entry.entry_kind,
           entry.authored_position,
           question.published_question_id,
           question.question_revision_number,
           pool_entry.question_pool_id,
           pool.question_pool_edit_number,
           item.member_position,
           item.published_question_id,
           item.question_revision_number,
           COALESCE(fixed_source.backend, pool_source.backend),
           pool_entry.selection_count,
           pool_entry.selected_question_order,
           policy_row.question_variation_rule,
           policy_row.assessment_question_order_rule
      FROM ple_data.assessment_entry AS entry
      LEFT JOIN ple_data.assessment_entry_question AS question
        ON question.assessment_entry_id = entry.assessment_entry_id
      LEFT JOIN ple_data.assessment_entry_pool AS pool_entry
        ON pool_entry.assessment_entry_id = entry.assessment_entry_id
      LEFT JOIN ple_private.question_revision_source_binding AS fixed_source
        ON fixed_source.published_question_id = question.published_question_id
       AND fixed_source.revision_number = question.question_revision_number
      LEFT JOIN ple_data.question_pool AS pool ON pool.question_pool_id = pool_entry.question_pool_id
      LEFT JOIN ple_data.question_pool_member AS item
        ON item.question_pool_id = pool_entry.question_pool_id
      LEFT JOIN ple_private.question_revision_source_binding AS pool_source
        ON pool_source.published_question_id = item.published_question_id
       AND pool_source.revision_number = item.question_revision_number
     WHERE entry.assessment_id = assessment_row.assessment_id
       AND entry.availability = 'available'
       AND (entry.entry_kind = 'fixed_question' OR item.member_position IS NOT NULL)
     ORDER BY entry.authored_position, item.member_position NULLS FIRST;
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Assessment Attempt start is unavailable';
    END IF;
END $$;
