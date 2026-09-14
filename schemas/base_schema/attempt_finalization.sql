-- Direct Student Assignment Attempt finalization after backend evaluation.

SET LOCAL ROLE ple_private_owner;

-- Captures only server-attested saved-response evidence for direct backend
-- evaluation.  The browser never selects the finalization kind: PostgreSQL
-- derives it from its clock and the retained expiry timestamp.  No lock is
-- held while the server resolves sources or calls a Question Backend.
CREATE FUNCTION ple_private.prepare_assignment_attempt_finalization(
    p_assignment_attempt_id uuid
) RETURNS TABLE (
    preparation_state text,
    finalization_kind text,
    missing_positions integer[],
    points_earned double precision,
    points_possible double precision,
    question_attempt_id uuid,
    saved_at_millis bigint,
    question_id text,
    revision_number integer,
    source_object_id uuid,
    source_object_checksum text,
    question_seed numeric,
    student_response jsonb,
    backend text,
    webwork_pg_path text
) LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
DECLARE attempt_row ple_private.assignment_attempt%ROWTYPE;
DECLARE now_value timestamptz := pg_catalog.clock_timestamp();
DECLARE resolved_kind text;
DECLARE missing integer[];
BEGIN
    SELECT * INTO attempt_row
      FROM ple_private.assignment_attempt AS attempt
     WHERE attempt.assignment_attempt_id = p_assignment_attempt_id;
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Assignment Attempt is unavailable';
    END IF;
    resolved_kind := CASE WHEN attempt_row.expires_at IS NOT NULL
                                AND now_value >= attempt_row.expires_at
                          THEN 'deadline' ELSE 'student' END;
    IF EXISTS (SELECT 1 FROM ple_private.assignment_submission AS submission
               WHERE submission.assignment_attempt_id = attempt_row.assignment_attempt_id) THEN
        RETURN QUERY
        SELECT 'already_submitted', resolved_kind, ARRAY[]::integer[],
               CASE WHEN bool_or(question_attempt.question_attempt_state <> 'closed_at_deadline'
                                  AND result.grading_result_id IS NULL)
                    THEN NULL ELSE coalesce(sum(score.points_earned), 0)::double precision END,
               CASE WHEN bool_or(question_attempt.question_attempt_state <> 'closed_at_deadline'
                                  AND result.grading_result_id IS NULL)
                    THEN NULL ELSE coalesce(sum(score.points_possible), 0)::double precision END,
               NULL::uuid, NULL::bigint, NULL::text, NULL::integer, NULL::uuid,
               NULL::text, NULL::numeric, NULL::jsonb, NULL::text, NULL::text
          FROM ple_private.issued_question AS issued
          JOIN ple_private.question_attempt AS question_attempt
            ON question_attempt.issued_question_id = issued.issued_question_id
          LEFT JOIN ple_private.grading_result AS result
            ON result.question_attempt_id = question_attempt.question_attempt_id
          JOIN ple_data.assignment_entry AS entry
            ON entry.assignment_entry_id = issued.assignment_entry_id
          CROSS JOIN LATERAL ple_private.score_recorded_credit(
              coalesce(result.normalized_credit, 0), issued.scoring_rule,
              CASE entry.entry_kind WHEN 'fixed_question' THEN entry.points_possible
                   ELSE entry.points_per_item END
          ) AS score
         WHERE issued.assignment_attempt_id = attempt_row.assignment_attempt_id;
        RETURN;
    END IF;
    IF resolved_kind = 'student' THEN
        SELECT array_agg(issued.issued_position ORDER BY issued.issued_position)
          INTO missing
          FROM ple_private.issued_question AS issued
          LEFT JOIN ple_private.question_attempt AS question_attempt
            ON question_attempt.issued_question_id = issued.issued_question_id
          LEFT JOIN ple_private.assignment_attempt_saved_response AS response
            ON response.question_attempt_id = question_attempt.question_attempt_id
         WHERE issued.assignment_attempt_id = attempt_row.assignment_attempt_id
           AND response.question_attempt_id IS NULL;
        IF missing IS NOT NULL THEN
            RETURN QUERY SELECT 'missing_responses', resolved_kind, missing,
                NULL::double precision, NULL::double precision, NULL::uuid,
                NULL::bigint, NULL::text, NULL::integer, NULL::uuid, NULL::text,
                NULL::numeric, NULL::jsonb, NULL::text, NULL::text;
            RETURN;
        END IF;
    END IF;
    IF EXISTS (
        SELECT 1
          FROM ple_private.issued_question AS issued
          JOIN ple_private.question_attempt AS question_attempt
            ON question_attempt.issued_question_id = issued.issued_question_id
          JOIN ple_private.assignment_attempt_saved_response AS response
            ON response.question_attempt_id = question_attempt.question_attempt_id
          JOIN ple_private.question_revision_source_binding AS source
            ON source.question_id = issued.question_id
           AND source.revision_number = issued.revision_number
         WHERE issued.assignment_attempt_id = attempt_row.assignment_attempt_id
           AND source.backend NOT IN ('ple', 'webwork')
    ) THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Assignment Attempt submission is unavailable';
    END IF;
    RETURN QUERY
    SELECT 'ready', resolved_kind, ARRAY[]::integer[], NULL::double precision,
           NULL::double precision, question_attempt.question_attempt_id,
           floor(extract(epoch FROM response.saved_at) * 1000)::bigint,
           issued.question_id, issued.revision_number, source.source_object_id,
           source.source_object_checksum, question_attempt.question_seed,
           response.student_response, source.backend, source.webwork_pg_path
      FROM ple_private.issued_question AS issued
      JOIN ple_private.question_attempt AS question_attempt
        ON question_attempt.issued_question_id = issued.issued_question_id
      JOIN ple_private.assignment_attempt_saved_response AS response
        ON response.question_attempt_id = question_attempt.question_attempt_id
      JOIN ple_private.question_revision_source_binding AS source
        ON source.question_id = issued.question_id
       AND source.revision_number = issued.revision_number
     WHERE issued.assignment_attempt_id = attempt_row.assignment_attempt_id
     ORDER BY issued.issued_position;
    IF NOT FOUND THEN
        RETURN QUERY SELECT 'ready', resolved_kind, ARRAY[]::integer[],
            NULL::double precision, NULL::double precision, NULL::uuid,
            NULL::bigint, NULL::text, NULL::integer, NULL::uuid, NULL::text,
            NULL::numeric, NULL::jsonb, NULL::text, NULL::text;
    END IF;
END $$;

-- Student authorization remains at this outer boundary.  The common core is
-- deliberately identity-neutral so the worker never needs a Student session.
CREATE FUNCTION ple_private.prepare_student_assignment_attempt_finalization(
    p_assignment_attempt_reference_number bigint
) RETURNS TABLE (
    preparation_state text,
    finalization_kind text,
    missing_positions integer[],
    points_earned double precision,
    points_possible double precision,
    question_attempt_id uuid,
    saved_at_millis bigint,
    question_id text,
    revision_number integer,
    source_object_id uuid,
    source_object_checksum text,
    question_seed numeric,
    student_response jsonb,
    backend text,
    webwork_pg_path text
) LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
DECLARE attempt_row ple_private.assignment_attempt%ROWTYPE;
BEGIN
    attempt_row := ple_private.assert_current_student_attempt(
        p_assignment_attempt_reference_number
    );
    RETURN QUERY SELECT * FROM ple_private.prepare_assignment_attempt_finalization(
        attempt_row.assignment_attempt_id
    );
END $$;

CREATE FUNCTION ple_private.commit_student_assignment_attempt_finalization(
    p_assignment_attempt_reference_number bigint,
    p_finalization_kind text,
    p_evaluations jsonb
) RETURNS TABLE (points_earned double precision, points_possible double precision)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
DECLARE attempt_row ple_private.assignment_attempt%ROWTYPE;
BEGIN
    attempt_row := ple_private.assert_current_student_attempt(
        p_assignment_attempt_reference_number
    );
    RETURN QUERY SELECT * FROM ple_private.commit_assignment_attempt_finalization(
        attempt_row.assignment_attempt_id,
        p_finalization_kind,
        p_evaluations,
        CASE WHEN p_finalization_kind = 'student'
             THEN ple_api.current_session_account_id() ELSE NULL END
    );
END $$;

-- The worker receives only expired, unsubmitted Attempt candidates.  The
-- limit applies before response expansion, so one Attempt with many saved
-- responses cannot starve other expired Attempts.  ASVS 2.2.1, 2.3.1.
CREATE FUNCTION ple_private.prepare_expired_student_assignment_attempt_finalizations(
    p_limit integer
) RETURNS TABLE (
    assignment_attempt_id uuid,
    question_attempt_id uuid,
    saved_at_millis bigint,
    question_id text,
    revision_number integer,
    source_object_id uuid,
    source_object_checksum text,
    question_seed numeric,
    student_response jsonb,
    backend text,
    webwork_pg_path text
) LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
BEGIN
    IF p_limit IS NULL OR p_limit NOT BETWEEN 1 AND 1000 THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Expired Assignment Attempt finalization limit is invalid';
    END IF;
    RETURN QUERY
    WITH candidates AS (
        SELECT attempt.assignment_attempt_id
          FROM ple_private.assignment_attempt AS attempt
         WHERE attempt.expires_at IS NOT NULL
           AND attempt.expires_at <= pg_catalog.clock_timestamp()
           AND NOT EXISTS (
               SELECT 1 FROM ple_private.assignment_submission AS submission
                WHERE submission.assignment_attempt_id = attempt.assignment_attempt_id
           )
         ORDER BY attempt.expires_at, attempt.assignment_attempt_id
         LIMIT p_limit
    )
    SELECT candidate.assignment_attempt_id,
           prepared.question_attempt_id, prepared.saved_at_millis,
           prepared.question_id, prepared.revision_number,
           prepared.source_object_id, prepared.source_object_checksum,
           prepared.question_seed, prepared.student_response,
           prepared.backend, prepared.webwork_pg_path
      FROM candidates AS candidate
      CROSS JOIN LATERAL ple_private.prepare_assignment_attempt_finalization(
          candidate.assignment_attempt_id
      ) AS prepared
     WHERE prepared.preparation_state = 'ready'
       AND prepared.finalization_kind = 'deadline';
END $$;

CREATE FUNCTION ple_private.commit_expired_student_assignment_attempt_finalization(
    p_assignment_attempt_id uuid,
    p_evaluations jsonb
) RETURNS TABLE (points_earned double precision, points_possible double precision)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
BEGIN
    RETURN QUERY SELECT * FROM ple_private.commit_assignment_attempt_finalization(
        p_assignment_attempt_id, 'deadline', p_evaluations, NULL
    );
END $$;

-- Accepts one previously prepared native/WeBWorK evaluation set.  The source
-- call happened before this transaction; this transaction verifies every
-- saved-response version before it creates any immutable submission evidence.
CREATE FUNCTION ple_private.commit_assignment_attempt_finalization(
    p_assignment_attempt_id uuid,
    p_finalization_kind text,
    p_evaluations jsonb,
    p_authorized_by_account_id uuid
) RETURNS TABLE (points_earned double precision, points_possible double precision)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
DECLARE attempt_row ple_private.assignment_attempt%ROWTYPE;
DECLARE now_value timestamptz := pg_catalog.clock_timestamp();
DECLARE resolved_kind text;
DECLARE accepted_count integer;
DECLARE expected_count integer;
DECLARE evaluation_row record;
DECLARE submission_id_value uuid;
BEGIN
    IF p_assignment_attempt_id IS NULL
       OR p_finalization_kind NOT IN ('student', 'deadline')
       OR ((p_finalization_kind = 'student') IS DISTINCT FROM
           (p_authorized_by_account_id IS NOT NULL))
       OR jsonb_typeof(p_evaluations) <> 'array' THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Assignment Attempt finalization facts are invalid';
    END IF;
    SELECT * INTO attempt_row
      FROM ple_private.assignment_attempt AS attempt
     WHERE attempt.assignment_attempt_id = p_assignment_attempt_id;
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Assignment Attempt is unavailable';
    END IF;
    PERFORM ple_private.lock_assignment_for_student_work(attempt_row.assignment_id);
    SELECT * INTO attempt_row FROM ple_private.assignment_attempt AS attempt
     WHERE attempt.assignment_attempt_id = attempt_row.assignment_attempt_id FOR UPDATE;
    now_value := pg_catalog.clock_timestamp();
    resolved_kind := CASE WHEN attempt_row.expires_at IS NOT NULL
                                AND now_value >= attempt_row.expires_at
                          THEN 'deadline' ELSE 'student' END;
    IF p_finalization_kind <> resolved_kind THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Assignment Attempt finalization is no longer current';
    END IF;
    IF EXISTS (SELECT 1 FROM ple_private.assignment_submission AS submission
               WHERE submission.assignment_attempt_id = attempt_row.assignment_attempt_id) THEN
        RETURN QUERY
        SELECT CASE WHEN bool_or(question_attempt.question_attempt_state <> 'closed_at_deadline'
                                      AND result.grading_result_id IS NULL)
                         THEN NULL ELSE coalesce(sum(score.points_earned), 0)::double precision END,
               CASE WHEN bool_or(question_attempt.question_attempt_state <> 'closed_at_deadline'
                                      AND result.grading_result_id IS NULL)
                         THEN NULL ELSE coalesce(sum(score.points_possible), 0)::double precision END
          FROM ple_private.issued_question AS issued
          JOIN ple_private.question_attempt AS question_attempt
            ON question_attempt.issued_question_id = issued.issued_question_id
          LEFT JOIN ple_private.grading_result AS result
            ON result.question_attempt_id = question_attempt.question_attempt_id
          JOIN ple_data.assignment_entry AS entry
            ON entry.assignment_entry_id = issued.assignment_entry_id
          CROSS JOIN LATERAL ple_private.score_recorded_credit(
              coalesce(result.normalized_credit, 0), issued.scoring_rule,
              CASE entry.entry_kind WHEN 'fixed_question' THEN entry.points_possible
                   ELSE entry.points_per_item END
          ) AS score
         WHERE issued.assignment_attempt_id = attempt_row.assignment_attempt_id;
        RETURN;
    END IF;
    SELECT count(*)::integer INTO expected_count
      FROM ple_private.issued_question AS issued
      JOIN ple_private.question_attempt AS question_attempt
        ON question_attempt.issued_question_id = issued.issued_question_id
      JOIN ple_private.assignment_attempt_saved_response AS response
        ON response.question_attempt_id = question_attempt.question_attempt_id
     WHERE issued.assignment_attempt_id = attempt_row.assignment_attempt_id;
    SELECT count(*)::integer INTO accepted_count
      FROM jsonb_to_recordset(p_evaluations) AS submitted_evaluation(
          question_attempt_id uuid, saved_at_millis bigint, student_response jsonb,
          normalized_credit numeric
      )
     WHERE submitted_evaluation.question_attempt_id IS NOT NULL
       AND submitted_evaluation.saved_at_millis IS NOT NULL
       AND submitted_evaluation.student_response IS NOT NULL
       AND submitted_evaluation.normalized_credit BETWEEN 0 AND 1;
    IF accepted_count <> expected_count OR accepted_count <> jsonb_array_length(p_evaluations)
       OR EXISTS (
           SELECT 1
             FROM ple_private.issued_question AS issued
             JOIN ple_private.question_attempt AS question_attempt
               ON question_attempt.issued_question_id = issued.issued_question_id
             JOIN ple_private.assignment_attempt_saved_response AS response
               ON response.question_attempt_id = question_attempt.question_attempt_id
             LEFT JOIN jsonb_to_recordset(p_evaluations) AS submitted_evaluation(
                 question_attempt_id uuid, saved_at_millis bigint, student_response jsonb,
                 normalized_credit numeric
             ) ON submitted_evaluation.question_attempt_id = question_attempt.question_attempt_id
                AND submitted_evaluation.saved_at_millis = floor(extract(epoch FROM response.saved_at) * 1000)::bigint
                AND submitted_evaluation.student_response = response.student_response
            WHERE issued.assignment_attempt_id = attempt_row.assignment_attempt_id
              AND submitted_evaluation.question_attempt_id IS NULL
       ) THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Assignment Attempt saved responses changed';
    END IF;
    IF resolved_kind = 'student' AND EXISTS (
        SELECT 1 FROM ple_private.issued_question AS issued
          LEFT JOIN ple_private.question_attempt AS question_attempt
            ON question_attempt.issued_question_id = issued.issued_question_id
          LEFT JOIN ple_private.assignment_attempt_saved_response AS response
            ON response.question_attempt_id = question_attempt.question_attempt_id
         WHERE issued.assignment_attempt_id = attempt_row.assignment_attempt_id
           AND response.question_attempt_id IS NULL
    ) THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Assignment Attempt has missing responses';
    END IF;
    INSERT INTO ple_private.assignment_submission(
        assignment_submission_id, assignment_attempt_id, submitted_at,
        finalization_kind, authorized_by_account_id, receipt
    ) VALUES (
        pg_catalog.gen_random_uuid(), attempt_row.assignment_attempt_id, now_value,
        resolved_kind,
        p_authorized_by_account_id,
        jsonb_build_object('submissionState', 'submitted', 'finalizationKind', resolved_kind)
    );
    UPDATE ple_private.question_attempt AS question_attempt
       SET question_attempt_state = CASE WHEN EXISTS (
                   SELECT 1 FROM ple_private.assignment_attempt_saved_response AS response
                    WHERE response.question_attempt_id = question_attempt.question_attempt_id
               ) THEN 'submission_accepted' ELSE 'closed_at_deadline' END,
           submitted_at = CASE WHEN EXISTS (
                   SELECT 1 FROM ple_private.assignment_attempt_saved_response AS response
                    WHERE response.question_attempt_id = question_attempt.question_attempt_id
               ) THEN now_value ELSE NULL END
      FROM ple_private.issued_question AS issued
     WHERE question_attempt.issued_question_id = issued.issued_question_id
       AND issued.assignment_attempt_id = attempt_row.assignment_attempt_id
       AND question_attempt.question_attempt_state = 'open';
    FOR evaluation_row IN SELECT * FROM jsonb_to_recordset(p_evaluations) AS submitted_evaluation(
        question_attempt_id uuid, saved_at_millis bigint, student_response jsonb,
        normalized_credit numeric
    ) LOOP
        submission_id_value := pg_catalog.gen_random_uuid();
        INSERT INTO ple_private.question_submission(
            submission_id, question_attempt_id, submitted_at, student_response
        )
        SELECT submission_id_value, response.question_attempt_id, now_value,
               response.student_response
          FROM ple_private.assignment_attempt_saved_response AS response
         WHERE response.question_attempt_id = evaluation_row.question_attempt_id;
        IF NOT FOUND THEN
            RAISE EXCEPTION USING ERRCODE = '42501',
                MESSAGE = 'Assignment Attempt saved responses changed';
        END IF;
        PERFORM ple_private.record_direct_automated_grading_result(
            submission_id_value, evaluation_row.question_attempt_id,
            evaluation_row.normalized_credit, now_value
        );
    END LOOP;
    IF expected_count = 0 THEN
        UPDATE ple_private.assignment_attempt
           SET completed_at = now_value
         WHERE assignment_attempt_id = attempt_row.assignment_attempt_id
           AND completed_at IS NULL;
    END IF;
    RETURN QUERY
    SELECT coalesce(sum(score.points_earned), 0)::double precision,
           coalesce(sum(score.points_possible), 0)::double precision
      FROM ple_private.issued_question AS issued
      JOIN ple_private.question_attempt AS question_attempt
        ON question_attempt.issued_question_id = issued.issued_question_id
      LEFT JOIN ple_private.grading_result AS result
        ON result.question_attempt_id = question_attempt.question_attempt_id
      JOIN ple_data.assignment_entry AS entry
        ON entry.assignment_entry_id = issued.assignment_entry_id
      CROSS JOIN LATERAL ple_private.score_recorded_credit(
          coalesce(result.normalized_credit, 0), issued.scoring_rule,
          CASE entry.entry_kind WHEN 'fixed_question' THEN entry.points_possible
               ELSE entry.points_per_item END
      ) AS score
     WHERE issued.assignment_attempt_id = attempt_row.assignment_attempt_id;
END $$;
