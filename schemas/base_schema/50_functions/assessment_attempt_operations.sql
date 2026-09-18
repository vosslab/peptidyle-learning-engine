-- Functions, triggers, and views from assessment_attempt_operations.sql.

SET LOCAL ROLE ple_private_owner;

-- Assessment-first Student Work operations.  Delivery owns creation of a
-- Question Attempt's backend-specific reproduction bundle; this module owns
-- current Assessment selection, response persistence, and finalization.



-- The same finite calculation serves landing, access, start, and Instructor
-- previews. Apply the Student multiplier only after the authored/default base.
CREATE FUNCTION ple_private.assessment_effective_duration_seconds(
    p_assessment_id text, p_time_multiplier numeric
) RETURNS integer LANGUAGE plpgsql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_data, ple_private AS $$
DECLARE base_seconds integer;
DECLARE multiplier numeric := COALESCE(p_time_multiplier, 1);
BEGIN
    -- ASVS 2.2.1: NaN, infinities, and shortening multipliers are invalid.
    IF multiplier < 1 OR multiplier >= 'Infinity'::numeric THEN
        RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Student time multiplier is invalid';
    END IF;
    base_seconds := ple_data.assessment_effective_base_duration_seconds(p_assessment_id);
    IF base_seconds IS NULL THEN RETURN NULL; END IF;
    -- Test before multiplying: even the largest finite multiplier cannot
    -- overflow an intermediate product. Fractional seconds round upward.
    IF multiplier >= 86400::numeric / base_seconds THEN RETURN 86400; END IF;
    RETURN least(86400, ceil(base_seconds * multiplier)::integer);
END $$;

CREATE FUNCTION ple_private.lock_assessment_for_student_work(p_assessment_id text)
RETURNS void LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_data, ple_private AS $$
BEGIN
    PERFORM 1 FROM ple_data.assessment
     WHERE assessment_id = p_assessment_id FOR UPDATE;
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Assessment Student Work operation is unavailable';
    END IF;
END $$;

CREATE FUNCTION ple_private.assert_current_student_assessment_attempt(
    p_assessment_attempt_id uuid
) RETURNS ple_private.assessment_attempt LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
DECLARE result ple_private.assessment_attempt%ROWTYPE;
DECLARE course_id_value text;
BEGIN
    SELECT assessment_attempt.* INTO result
      FROM ple_private.assessment_attempt AS assessment_attempt
     WHERE assessment_attempt.assessment_attempt_id = p_assessment_attempt_id;
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Assessment Attempt is unavailable';
    END IF;
    -- Normal Student Work ends at the Course retention boundary.  Archived
    -- evidence remains reachable only through the separate retention-executor
    -- capability; an owned Assessment Attempt reference is never an ordinary read or
    -- mutator bypass after archive or deletion.  Keep this beside the
    -- ownership assertion because every current-Assessment Attempt operation shares it.
    SELECT assessment.course_instance_id INTO course_id_value
      FROM ple_data.assessment AS assessment
     WHERE assessment.assessment_id = result.assessment_id;
    IF NOT FOUND
       OR NOT ple_api.course_student_work_is_ordinarily_visible(course_id_value)
       OR NOT ple_api.current_session_account_owns_student_record(
           course_id_value, result.student_record_id
       ) THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Assessment Attempt is unavailable';
    END IF;
    RETURN result;
END $$;



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
    IF assessment_row.course_instance_id IS DISTINCT FROM (
        SELECT student.course_instance_id FROM ple_data.student_record AS student
         WHERE student.student_record_id = p_student_record_id
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
            question_pool_id, question_pool_edit_number, created_at, selected_question_count
        ) VALUES (
            assessment_row.course_instance_id, selection_id, p_assessment_attempt_id, selection_entry_id,
            entry_row.question_pool_id, entry_row.question_pool_edit_number, now_value,
            entry_row.selection_count
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

SET LOCAL ROLE ple_api_owner;



-- Course membership owns the current Student lookup.  This helper remains
-- executable only by the private Assessment Attempt boundary, so application sessions
-- cannot enumerate Student records.  The public ownership predicate continues
-- to decide whether the resulting record is usable for Student Work.
CREATE FUNCTION ple_api.current_session_student_record_id(p_course_instance_id text)
RETURNS uuid LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data AS $$
    SELECT student.student_record_id
      FROM ple_data.student_record AS student
      JOIN ple_data.course_membership AS membership
        ON membership.student_record_id = student.student_record_id
       AND membership.course_instance_id = student.course_instance_id
       AND membership.account_id = student.student_account_id
       AND membership.role = 'student'
       AND ple_data.course_membership_is_active(membership.course_membership_id)
     WHERE student.course_instance_id = p_course_instance_id
       AND student.student_account_id = ple_api.current_session_account_id()
$$;



-- Private Assessment Attempt readers need stable Course route/display facts but do not
-- receive direct access to the Course relation.  These API-owner functions
-- are executable only by that trusted private boundary.
CREATE FUNCTION ple_api.course_reference_number_for_assessment_attempt(p_course_instance_id text)
RETURNS text LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_data AS $$
    SELECT course.course_instance_id
      FROM ple_data.course_instance AS course
     WHERE course.course_instance_id = p_course_instance_id
$$;

CREATE FUNCTION ple_api.course_display_for_assessment_attempt(p_course_instance_id text)
RETURNS TABLE (
    course_reference_number text,
    course_short_name text,
    course_long_name text,
    course_theme text
) LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_data AS $$
    SELECT course.course_instance_id,
           course.course_short_name,
           course.course_long_name,
           course.course_theme_id
      FROM ple_data.course_instance AS course
     WHERE course.course_instance_id = p_course_instance_id
$$;

SET LOCAL ROLE ple_private_owner;





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
    p_course_reference_number text,
    p_assessment_public_reference text
) RETURNS TABLE (
    resumable_assessment_attempt_id uuid,
    resumable_assessment_attempt_number integer
)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
DECLARE assessment_row ple_data.assessment%ROWTYPE;
DECLARE student_record_id_value uuid;
BEGIN
    IF p_course_reference_number IS NULL
       OR p_assessment_public_reference IS NULL THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Assessment Attempt start is unavailable';
    END IF;
    SELECT assessment.* INTO assessment_row
      FROM ple_data.assessment AS assessment
     WHERE assessment.assessment_id = p_assessment_public_reference;
    IF NOT FOUND OR ple_api.course_reference_number_for_assessment_attempt(
        assessment_row.course_instance_id
    ) IS DISTINCT FROM p_course_reference_number THEN
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
    p_course_reference_number text,
    p_assessment_public_reference text
) RETURNS TABLE (
    student_record_id uuid,
    assessment_id text,
    assessment_entry_id uuid,
    entry_kind text,
    authored_position integer,
    fixed_question_id text,
    fixed_revision_number integer,
    question_pool_id text,
    question_pool_public_id text,
    question_pool_edit_number bigint,
    member_position integer,
    pool_question_id text,
    pool_revision_number integer,
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
    IF p_course_reference_number IS NULL
       OR p_assessment_public_reference IS NULL THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Assessment Attempt start is unavailable';
    END IF;

    SELECT assessment.* INTO assessment_row
      FROM ple_data.assessment AS assessment
     WHERE assessment.assessment_id = p_assessment_public_reference;
    IF NOT FOUND OR ple_api.course_reference_number_for_assessment_attempt(
        assessment_row.course_instance_id
    ) IS DISTINCT FROM p_course_reference_number THEN
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
           pool.question_pool_id,
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



-- The start response reads its title and instructions from the retained
-- Assessment Attempt evidence. Course and Assessment public references are stable
-- route identities, while authored content is never re-read from mutable
-- Assessment configuration after an Assessment Attempt exists.
CREATE FUNCTION ple_private.read_started_student_assessment_attempt(
    p_assessment_attempt_id uuid
) RETURNS TABLE (
    assessment_attempt_id uuid,
    course_reference_number text,
    assessment_reference_number text,
    assessment_attempt_number integer,
    assessment_title text,
    assessment_instructions text
)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
BEGIN
    IF p_assessment_attempt_id IS NULL THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Assessment Attempt is unavailable';
    END IF;
    RETURN QUERY
    SELECT assessment_attempt.assessment_attempt_id,
           course.course_instance_id,
           assessment.assessment_id,
           assessment_attempt.assessment_attempt_number,
           policy.assessment_title,
           policy.assessment_instructions
      FROM ple_private.assessment_attempt AS assessment_attempt
      JOIN ple_data.assessment AS assessment ON assessment.assessment_id = assessment_attempt.assessment_id
      JOIN ple_data.assessment_policy_snapshot AS policy
        ON policy.assessment_policy_snapshot_id = assessment_attempt.assessment_policy_snapshot_id
      JOIN LATERAL ple_api.course_display_for_assessment_attempt(assessment.course_instance_id) AS course ON true
     WHERE assessment_attempt.assessment_attempt_id = p_assessment_attempt_id
       AND ple_api.current_session_account_owns_student_record(
           assessment.course_instance_id, assessment_attempt.student_record_id
       );
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Assessment Attempt is unavailable';
    END IF;
END $$;

CREATE FUNCTION ple_private.save_student_assessment_attempt_response(
    p_assessment_attempt_id uuid, p_issued_position integer, p_student_response jsonb
) RETURNS TABLE (assessment_attempt_id uuid, issued_position integer, response_state text)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
DECLARE assessment_attempt_row ple_private.assessment_attempt%ROWTYPE;
DECLARE question_attempt_id_value uuid;
DECLARE now_value timestamptz;
BEGIN
    assessment_attempt_row := ple_private.assert_current_student_assessment_attempt(
        p_assessment_attempt_id
    );
    -- Match finalization's Assessment -> Assessment Attempt -> Question Attempt
    -- lock order. A pre-expiry save therefore commits before a worker can
    -- finalize this Attempt, and the worker must observe the saved response.
    -- ASVS 2.3.3, 2.3.4: accepted Student Work and deadline submission are
    -- serialized rather than resolved from competing snapshots.
    PERFORM ple_private.lock_assessment_for_student_work(assessment_attempt_row.assessment_id);
    SELECT * INTO assessment_attempt_row
      FROM ple_private.assessment_attempt AS assessment_attempt
     WHERE assessment_attempt.assessment_attempt_id = assessment_attempt_row.assessment_attempt_id
     FOR UPDATE;
    now_value := pg_catalog.clock_timestamp();
    IF EXISTS (
        SELECT 1 FROM ple_private.assessment_submission AS submission
         WHERE submission.assessment_attempt_id = assessment_attempt_row.assessment_attempt_id
    ) OR (assessment_attempt_row.expires_at IS NOT NULL AND now_value >= assessment_attempt_row.expires_at) THEN
        assessment_attempt_id := assessment_attempt_row.assessment_attempt_id;
        issued_position := p_issued_position;
        response_state := 'expired';
        RETURN NEXT;
        RETURN;
    END IF;
    IF p_issued_position < 0 OR jsonb_typeof(p_student_response) <> 'object' THEN
        RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Student response arguments are invalid';
    END IF;
    SELECT question_attempt.question_attempt_id INTO question_attempt_id_value
      FROM ple_private.issued_question AS issued
      JOIN ple_private.question_attempt ON question_attempt.issued_question_id = issued.issued_question_id
      JOIN ple_private.question_revision_source_binding AS source
        ON source.published_question_id = issued.published_question_id
       AND source.revision_number = issued.revision_number
     WHERE issued.assessment_attempt_id = assessment_attempt_row.assessment_attempt_id
       AND issued.issued_position = p_issued_position
       AND question_attempt.question_attempt_state = 'open'
       AND ple_private.question_backend_is_supported_for_production(source.backend)
     FOR UPDATE OF question_attempt;
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Issued Question is unavailable';
    END IF;
    now_value := pg_catalog.clock_timestamp();
    IF EXISTS (
        SELECT 1 FROM ple_private.assessment_submission AS submission
         WHERE submission.assessment_attempt_id = assessment_attempt_row.assessment_attempt_id
    ) OR (assessment_attempt_row.expires_at IS NOT NULL AND now_value >= assessment_attempt_row.expires_at) THEN
        assessment_attempt_id := assessment_attempt_row.assessment_attempt_id;
        issued_position := p_issued_position;
        response_state := 'expired';
        RETURN NEXT;
        RETURN;
    END IF;
    INSERT INTO ple_private.assessment_attempt_saved_response(
        course_instance_id, question_attempt_id, student_response, saved_at)
    VALUES (assessment_attempt_row.course_instance_id, question_attempt_id_value, p_student_response, now_value)
    ON CONFLICT (course_instance_id, question_attempt_id) DO UPDATE
       SET student_response = EXCLUDED.student_response, saved_at = EXCLUDED.saved_at;
    assessment_attempt_id := assessment_attempt_row.assessment_attempt_id;
    issued_position := p_issued_position;
    response_state := 'saved';
    RETURN NEXT;
END $$;

CREATE FUNCTION ple_private.read_student_assessment_attempt_progress(
    p_assessment_attempt_id uuid
) RETURNS TABLE (
    assessment_attempt_id uuid, question_count integer,
    recommended_position integer, issued_position integer, response_state text
) LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private AS $$
DECLARE assessment_attempt_row ple_private.assessment_attempt%ROWTYPE;
BEGIN
    assessment_attempt_row := ple_private.assert_current_student_assessment_attempt(p_assessment_attempt_id);
    RETURN QUERY
    SELECT assessment_attempt_row.assessment_attempt_id, count(*) OVER ()::integer,
           min(issued.issued_position) FILTER (WHERE question_attempt.question_attempt_state = 'open'
               AND response.question_attempt_id IS NULL) OVER (),
           issued.issued_position,
           CASE WHEN question_attempt.question_attempt_state = 'response_finalized' THEN 'submitted'
                WHEN question_attempt.question_attempt_state = 'closed_unanswered' THEN 'closed'
                WHEN response.question_attempt_id IS NOT NULL THEN 'saved' ELSE 'unanswered' END
      FROM ple_private.issued_question AS issued
      JOIN ple_private.question_attempt ON question_attempt.issued_question_id = issued.issued_question_id
      LEFT JOIN ple_private.assessment_attempt_saved_response AS response ON response.question_attempt_id = question_attempt.question_attempt_id
     WHERE issued.assessment_attempt_id = assessment_attempt_row.assessment_attempt_id
     ORDER BY issued.issued_position;
END $$;



-- A saved response is mutable working state for one currently active,
-- student-owned Assessment Attempt.  This reader resolves only the retained Issued
-- Question and Question Attempt evidence; it does not consult mutable
-- Assessment configuration.
CREATE FUNCTION ple_private.read_student_assessment_attempt_saved_response(
    p_assessment_attempt_id uuid,
    p_issued_position integer
) RETURNS TABLE (issued_position integer, student_response jsonb)
LANGUAGE plpgsql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
DECLARE assessment_attempt_row ple_private.assessment_attempt%ROWTYPE;
BEGIN
    IF p_issued_position < 0 THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Saved Student response is unavailable';
    END IF;

    assessment_attempt_row := ple_private.assert_current_student_assessment_attempt(
        p_assessment_attempt_id
    );
    RETURN QUERY
    SELECT issued.issued_position,
           response.student_response
      FROM ple_private.issued_question AS issued
      JOIN ple_private.question_attempt AS question_attempt
        ON question_attempt.issued_question_id = issued.issued_question_id
      LEFT JOIN ple_private.assessment_attempt_saved_response AS response
        ON response.question_attempt_id = question_attempt.question_attempt_id
     WHERE issued.assessment_attempt_id = assessment_attempt_row.assessment_attempt_id
       AND issued.issued_position = p_issued_position;
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Saved Student response is unavailable';
    END IF;
END $$;



-- Grading workers call this before recording a result.  It deliberately takes
-- the same Assessment row lock as student mutations so Unrelease either wins
-- before work begins or waits for the already-authorized commit to finish.
CREATE FUNCTION ple_private.lock_question_attempt_for_grading(p_question_attempt_id uuid)
RETURNS TABLE (
    question_attempt_id uuid, issued_question_id uuid, assessment_attempt_id uuid,
    assessment_id text, published_question_id text, revision_number integer, question_seed numeric,
    generated_parameter_sha256 text
) LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_data, ple_private AS $$
DECLARE assessment_id_value text;
BEGIN
    SELECT assessment_attempt.assessment_id INTO assessment_id_value
      FROM ple_private.question_attempt AS question_attempt
      JOIN ple_private.issued_question AS issued ON issued.issued_question_id = question_attempt.issued_question_id
      JOIN ple_private.assessment_attempt AS assessment_attempt ON assessment_attempt.assessment_attempt_id = issued.assessment_attempt_id
     WHERE question_attempt.question_attempt_id = p_question_attempt_id;
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Question Attempt grading target is unavailable';
    END IF;
    PERFORM ple_private.lock_assessment_for_student_work(assessment_id_value);
    RETURN QUERY
    SELECT question_attempt.question_attempt_id, issued.issued_question_id,
           assessment_attempt.assessment_attempt_id, assessment_attempt.assessment_id, issued.published_question_id,
           issued.revision_number, question_attempt.question_seed,
           question_attempt.generated_parameter_sha256
      FROM ple_private.question_attempt AS question_attempt
      JOIN ple_private.issued_question AS issued ON issued.issued_question_id = question_attempt.issued_question_id
      JOIN ple_private.assessment_attempt AS assessment_attempt ON assessment_attempt.assessment_attempt_id = issued.assessment_attempt_id
     WHERE question_attempt.question_attempt_id = p_question_attempt_id
     FOR KEY SHARE OF question_attempt, issued, assessment_attempt;
END $$;

CREATE FUNCTION ple_private.read_student_assessment_attempt_history_evidence(
    p_assessment_attempt_id uuid
) RETURNS TABLE (
    assessment_attempt_id uuid, assessment_title text, assessment_instructions text,
    issued_position integer, published_question_id text, revision_number integer, question_seed numeric,
    generated_parameter_sha256 text,
    question_attempt_limit integer, question_attempt_time_limit_seconds integer,
    question_attempt_grace_seconds integer, question_attempt_state text, student_response jsonb
) LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private, ple_data AS $$
DECLARE assessment_attempt_row ple_private.assessment_attempt%ROWTYPE;
BEGIN
    assessment_attempt_row := ple_private.assert_current_student_assessment_attempt(p_assessment_attempt_id);
    RETURN QUERY
    SELECT assessment_attempt_row.assessment_attempt_id, policy.assessment_title, policy.assessment_instructions,
           issued.issued_position, issued.published_question_id, issued.revision_number,
           question_attempt.question_seed, question_attempt.generated_parameter_sha256,
           snapshot.question_attempt_limit, snapshot.question_attempt_time_limit_seconds,
           snapshot.question_attempt_grace_seconds, question_attempt.question_attempt_state,
           COALESCE(submission.student_response, response.student_response)
      FROM ple_private.issued_question AS issued
      JOIN ple_private.assessment_entry_snapshot AS snapshot
        ON snapshot.assessment_entry_snapshot_id = issued.assessment_entry_snapshot_id
      JOIN ple_data.assessment_policy_snapshot AS policy
        ON policy.assessment_policy_snapshot_id = assessment_attempt_row.assessment_policy_snapshot_id
      JOIN ple_private.question_attempt ON question_attempt.issued_question_id = issued.issued_question_id
      LEFT JOIN ple_private.assessment_attempt_saved_response AS response ON response.question_attempt_id = question_attempt.question_attempt_id
      LEFT JOIN ple_private.question_response AS submission ON submission.question_attempt_id = question_attempt.question_attempt_id
     WHERE issued.assessment_attempt_id = assessment_attempt_row.assessment_attempt_id
     ORDER BY issued.issued_position;
END $$;

