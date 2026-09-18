-- Functions, triggers, and views from assessment_attempt_finalization.sql.

SET LOCAL ROLE ple_private_owner;

-- Direct Student Assessment Attempt finalization after backend evaluation.



-- Captures only server-attested saved-response evidence for direct backend
-- evaluation.  The browser never selects the finalization kind: PostgreSQL
-- derives it from its clock and the retained expiry timestamp.  No lock is
-- held while the server resolves sources or calls a Question Backend.
CREATE FUNCTION ple_private.prepare_assessment_attempt_finalization(
    p_assessment_attempt_id uuid
) RETURNS TABLE (
    preparation_state text,
    finalization_kind text,
    points_earned double precision,
    points_possible double precision,
    question_attempt_id uuid,
    saved_at_millis bigint,
    question_id text,
    revision_number integer,
    source_object_id uuid,
    source_object_checksum text,
    question_seed numeric, generated_parameter_sha256 text,
    student_response jsonb,
    backend text,
    webwork_pg_path text
) LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
DECLARE assessment_attempt_row ple_private.assessment_attempt%ROWTYPE;
DECLARE now_value timestamptz := pg_catalog.clock_timestamp();
DECLARE resolved_kind text;
BEGIN
    SELECT * INTO assessment_attempt_row
      FROM ple_private.assessment_attempt AS assessment_attempt
     WHERE assessment_attempt.assessment_attempt_id = p_assessment_attempt_id;
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Assessment Attempt is unavailable';
    END IF;
    resolved_kind := CASE WHEN assessment_attempt_row.expires_at IS NOT NULL
                                AND now_value >= assessment_attempt_row.expires_at
                          THEN 'deadline' ELSE 'student' END;
    IF EXISTS (SELECT 1 FROM ple_private.assessment_submission AS submission
               WHERE submission.assessment_attempt_id = assessment_attempt_row.assessment_attempt_id) THEN
        RETURN QUERY
        SELECT 'already_submitted', resolved_kind,
               CASE WHEN bool_or(question_attempt.question_attempt_state <> 'closed_unanswered'
                                  AND result.grading_result_id IS NULL)
                    THEN NULL ELSE coalesce(sum(score.points_earned), 0)::double precision END,
               CASE WHEN bool_or(question_attempt.question_attempt_state <> 'closed_unanswered'
                                  AND result.grading_result_id IS NULL)
                    THEN NULL ELSE coalesce(sum(score.points_possible), 0)::double precision END,
               NULL::uuid, NULL::bigint, NULL::text, NULL::integer, NULL::uuid,
               NULL::text, NULL::numeric, NULL::text, NULL::jsonb, NULL::text, NULL::text
          FROM ple_private.issued_question AS issued
          JOIN ple_private.question_attempt AS question_attempt
            ON question_attempt.issued_question_id = issued.issued_question_id
          LEFT JOIN ple_private.grading_result AS result
            ON result.question_attempt_id = question_attempt.question_attempt_id
          JOIN ple_data.assessment_entry AS entry
            ON entry.assessment_entry_id = issued.assessment_entry_id
          CROSS JOIN LATERAL ple_private.score_recorded_credit(
              result.normalized_credit, issued.scoring_rule,
              CASE entry.entry_kind WHEN 'fixed_question' THEN entry.points_possible
                   ELSE entry.points_per_item END
          ) AS score
         WHERE issued.assessment_attempt_id = assessment_attempt_row.assessment_attempt_id;
        RETURN;
    END IF;
    IF EXISTS (
        SELECT 1
          FROM ple_private.issued_question AS issued
          JOIN ple_private.question_attempt AS question_attempt
            ON question_attempt.issued_question_id = issued.issued_question_id
          JOIN ple_private.question_revision_source_binding AS source
            ON source.question_id = issued.question_id
           AND source.revision_number = issued.revision_number
         WHERE issued.assessment_attempt_id = assessment_attempt_row.assessment_attempt_id
           AND NOT ple_private.question_backend_is_supported_for_production(source.backend)
    ) THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Assessment Attempt submission is unavailable';
    END IF;
    RETURN QUERY
    SELECT 'ready', resolved_kind, NULL::double precision,
           NULL::double precision, question_attempt.question_attempt_id,
           floor(extract(epoch FROM response.saved_at) * 1000)::bigint,
           issued.question_id, issued.revision_number, source.source_object_id,
           source.source_object_checksum, question_attempt.question_seed,
           question_attempt.generated_parameter_sha256,
           response.student_response, source.backend, source.webwork_pg_path
      FROM ple_private.issued_question AS issued
      JOIN ple_private.question_attempt AS question_attempt
        ON question_attempt.issued_question_id = issued.issued_question_id
      JOIN ple_private.assessment_attempt_saved_response AS response
        ON response.question_attempt_id = question_attempt.question_attempt_id
      JOIN ple_private.question_revision_source_binding AS source
        ON source.question_id = issued.question_id
       AND source.revision_number = issued.revision_number
     WHERE issued.assessment_attempt_id = assessment_attempt_row.assessment_attempt_id
     ORDER BY issued.issued_position;
    IF NOT FOUND THEN
        RETURN QUERY SELECT 'ready', resolved_kind,
            NULL::double precision, NULL::double precision, NULL::uuid,
            NULL::bigint, NULL::text, NULL::integer, NULL::uuid, NULL::text,
            NULL::numeric, NULL::text, NULL::jsonb, NULL::text, NULL::text;
    END IF;
END $$;



-- Student authorization remains at this outer boundary.  The common core is
-- deliberately identity-neutral so the worker never needs a Student session.
CREATE FUNCTION ple_private.prepare_student_assessment_attempt_finalization(
    p_assessment_attempt_reference_number bigint
) RETURNS TABLE (
    preparation_state text,
    finalization_kind text,
    points_earned double precision,
    points_possible double precision,
    question_attempt_id uuid,
    saved_at_millis bigint,
    question_id text,
    revision_number integer,
    source_object_id uuid,
    source_object_checksum text,
    question_seed numeric, generated_parameter_sha256 text,
    student_response jsonb,
    backend text,
    webwork_pg_path text
) LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
DECLARE assessment_attempt_row ple_private.assessment_attempt%ROWTYPE;
BEGIN
    assessment_attempt_row := ple_private.assert_current_student_assessment_attempt(
        p_assessment_attempt_reference_number
    );
    RETURN QUERY SELECT * FROM ple_private.prepare_assessment_attempt_finalization(
        assessment_attempt_row.assessment_attempt_id
    );
END $$;

CREATE FUNCTION ple_private.commit_student_assessment_attempt_finalization(
    p_assessment_attempt_reference_number bigint,
    p_finalization_kind text,
    p_evaluations jsonb
) RETURNS TABLE (points_earned double precision, points_possible double precision)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
DECLARE assessment_attempt_row ple_private.assessment_attempt%ROWTYPE;
BEGIN
    assessment_attempt_row := ple_private.assert_current_student_assessment_attempt(
        p_assessment_attempt_reference_number
    );
    RETURN QUERY SELECT * FROM ple_private.commit_assessment_attempt_finalization(
        assessment_attempt_row.assessment_attempt_id,
        p_finalization_kind,
        p_evaluations,
        CASE WHEN p_finalization_kind = 'student'
             THEN ple_api.current_session_account_id() ELSE NULL END
    );
END $$;



-- The worker receives only expired, unsubmitted Assessment Attempt candidates.  The
-- limit applies before response expansion, so one Assessment Attempt with many saved
-- responses cannot starve other expired Assessment Attempts.  ASVS 2.2.1, 2.3.1.
CREATE FUNCTION ple_private.prepare_expired_student_assessment_attempt_finalizations(
    p_limit integer
) RETURNS TABLE (
    assessment_attempt_id uuid,
    question_attempt_id uuid,
    saved_at_millis bigint,
    question_id text,
    revision_number integer,
    source_object_id uuid,
    source_object_checksum text,
    question_seed numeric, generated_parameter_sha256 text,
    student_response jsonb,
    backend text,
    webwork_pg_path text
) LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
BEGIN
    IF p_limit IS NULL OR p_limit NOT BETWEEN 1 AND 1000 THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Expired Assessment Attempt finalization limit is invalid';
    END IF;
    RETURN QUERY
    WITH candidates AS (
        SELECT assessment_attempt.assessment_attempt_id
          FROM ple_private.assessment_attempt AS assessment_attempt
         WHERE assessment_attempt.expires_at IS NOT NULL
           AND assessment_attempt.expires_at <= pg_catalog.clock_timestamp()
           AND NOT EXISTS (
               SELECT 1 FROM ple_private.assessment_submission AS submission
                WHERE submission.assessment_attempt_id = assessment_attempt.assessment_attempt_id
           )
         ORDER BY assessment_attempt.expires_at, assessment_attempt.assessment_attempt_id
         LIMIT p_limit
    )
    SELECT candidate.assessment_attempt_id,
           prepared.question_attempt_id, prepared.saved_at_millis,
           prepared.question_id, prepared.revision_number,
           prepared.source_object_id, prepared.source_object_checksum,
           prepared.question_seed, prepared.generated_parameter_sha256,
           prepared.student_response,
           prepared.backend, prepared.webwork_pg_path
      FROM candidates AS candidate
      CROSS JOIN LATERAL ple_private.prepare_assessment_attempt_finalization(
          candidate.assessment_attempt_id
      ) AS prepared
     WHERE prepared.preparation_state = 'ready'
       AND prepared.finalization_kind = 'deadline';
END $$;

CREATE FUNCTION ple_private.commit_expired_student_assessment_attempt_finalization(
    p_assessment_attempt_id uuid,
    p_evaluations jsonb
) RETURNS TABLE (points_earned double precision, points_possible double precision)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
BEGIN
    RETURN QUERY SELECT * FROM ple_private.commit_assessment_attempt_finalization(
        p_assessment_attempt_id, 'deadline', p_evaluations, NULL
    );
END $$;



-- Accepts one previously prepared native/WeBWorK evaluation set.  The source
-- call happened before this transaction; this transaction verifies every
-- saved-response version before it creates any immutable submission evidence.
CREATE FUNCTION ple_private.commit_assessment_attempt_finalization(
    p_assessment_attempt_id uuid,
    p_finalization_kind text,
    p_evaluations jsonb,
    p_authorized_by_account_id uuid
) RETURNS TABLE (points_earned double precision, points_possible double precision)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
DECLARE assessment_attempt_row ple_private.assessment_attempt%ROWTYPE;
DECLARE now_value timestamptz := pg_catalog.clock_timestamp();
DECLARE resolved_kind text;
DECLARE accepted_count integer;
DECLARE expected_count integer;
DECLARE evaluation_row record;
DECLARE question_response_id_value uuid;
DECLARE assessment_submission_id_value uuid := pg_catalog.gen_random_uuid();
BEGIN
    IF p_assessment_attempt_id IS NULL
       OR p_finalization_kind NOT IN ('student', 'deadline')
       OR ((p_finalization_kind = 'student') IS DISTINCT FROM
           (p_authorized_by_account_id IS NOT NULL))
       OR jsonb_typeof(p_evaluations) <> 'array' THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Assessment Attempt finalization facts are invalid';
    END IF;
    SELECT * INTO assessment_attempt_row
      FROM ple_private.assessment_attempt AS assessment_attempt
     WHERE assessment_attempt.assessment_attempt_id = p_assessment_attempt_id;
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Assessment Attempt is unavailable';
    END IF;
    PERFORM ple_private.lock_assessment_for_student_work(assessment_attempt_row.assessment_id);
    SELECT * INTO assessment_attempt_row FROM ple_private.assessment_attempt AS assessment_attempt
     WHERE assessment_attempt.assessment_attempt_id = assessment_attempt_row.assessment_attempt_id FOR UPDATE;
    now_value := pg_catalog.clock_timestamp();
    resolved_kind := CASE WHEN assessment_attempt_row.expires_at IS NOT NULL
                                AND now_value >= assessment_attempt_row.expires_at
                          THEN 'deadline' ELSE 'student' END;
    IF p_finalization_kind <> resolved_kind THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Assessment Attempt finalization is no longer current';
    END IF;
    IF EXISTS (SELECT 1 FROM ple_private.assessment_submission AS submission
               WHERE submission.assessment_attempt_id = assessment_attempt_row.assessment_attempt_id) THEN
        RETURN QUERY
        SELECT CASE WHEN bool_or(question_attempt.question_attempt_state <> 'closed_unanswered'
                                      AND result.grading_result_id IS NULL)
                         THEN NULL ELSE coalesce(sum(score.points_earned), 0)::double precision END,
               CASE WHEN bool_or(question_attempt.question_attempt_state <> 'closed_unanswered'
                                      AND result.grading_result_id IS NULL)
                         THEN NULL ELSE coalesce(sum(score.points_possible), 0)::double precision END
          FROM ple_private.issued_question AS issued
          JOIN ple_private.question_attempt AS question_attempt
            ON question_attempt.issued_question_id = issued.issued_question_id
          LEFT JOIN ple_private.grading_result AS result
            ON result.question_attempt_id = question_attempt.question_attempt_id
          JOIN ple_data.assessment_entry AS entry
            ON entry.assessment_entry_id = issued.assessment_entry_id
          CROSS JOIN LATERAL ple_private.score_recorded_credit(
              result.normalized_credit, issued.scoring_rule,
              CASE entry.entry_kind WHEN 'fixed_question' THEN entry.points_possible
                   ELSE entry.points_per_item END
          ) AS score
         WHERE issued.assessment_attempt_id = assessment_attempt_row.assessment_attempt_id;
        RETURN;
    END IF;
    SELECT count(*)::integer INTO expected_count
      FROM ple_private.issued_question AS issued
      JOIN ple_private.question_attempt AS question_attempt
        ON question_attempt.issued_question_id = issued.issued_question_id
      JOIN ple_private.assessment_attempt_saved_response AS response
        ON response.question_attempt_id = question_attempt.question_attempt_id
     WHERE issued.assessment_attempt_id = assessment_attempt_row.assessment_attempt_id;
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
             JOIN ple_private.assessment_attempt_saved_response AS response
               ON response.question_attempt_id = question_attempt.question_attempt_id
             LEFT JOIN jsonb_to_recordset(p_evaluations) AS submitted_evaluation(
                 question_attempt_id uuid, saved_at_millis bigint, student_response jsonb,
                 normalized_credit numeric
             ) ON submitted_evaluation.question_attempt_id = question_attempt.question_attempt_id
                AND submitted_evaluation.saved_at_millis = floor(extract(epoch FROM response.saved_at) * 1000)::bigint
                AND submitted_evaluation.student_response = response.student_response
            WHERE issued.assessment_attempt_id = assessment_attempt_row.assessment_attempt_id
              AND submitted_evaluation.question_attempt_id IS NULL
       ) THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Assessment Attempt saved responses changed';
    END IF;
    INSERT INTO ple_private.assessment_submission(
        assessment_submission_id, assessment_attempt_id, submitted_at,
        finalization_kind, authorized_by_account_id, receipt
    ) VALUES (
        assessment_submission_id_value, assessment_attempt_row.assessment_attempt_id, now_value,
        resolved_kind,
        p_authorized_by_account_id,
        jsonb_build_object('submissionState', 'submitted', 'finalizationKind', resolved_kind)
    );
    UPDATE ple_private.question_attempt AS question_attempt
       SET question_attempt_state = CASE WHEN EXISTS (
                   SELECT 1 FROM ple_private.assessment_attempt_saved_response AS response
                    WHERE response.question_attempt_id = question_attempt.question_attempt_id
               ) THEN 'response_finalized' ELSE 'closed_unanswered' END,
           finalized_at = CASE WHEN EXISTS (
                   SELECT 1 FROM ple_private.assessment_attempt_saved_response AS response
                    WHERE response.question_attempt_id = question_attempt.question_attempt_id
               ) THEN now_value ELSE NULL END
      FROM ple_private.issued_question AS issued
     WHERE question_attempt.issued_question_id = issued.issued_question_id
       AND issued.assessment_attempt_id = assessment_attempt_row.assessment_attempt_id
       AND question_attempt.question_attempt_state = 'open';
    FOR evaluation_row IN SELECT * FROM jsonb_to_recordset(p_evaluations) AS submitted_evaluation(
        question_attempt_id uuid, saved_at_millis bigint, student_response jsonb,
        normalized_credit numeric
    ) LOOP
        question_response_id_value := pg_catalog.gen_random_uuid();
        INSERT INTO ple_private.question_response(
            question_response_id, assessment_submission_id, question_attempt_id, finalized_at, student_response
        )
        SELECT question_response_id_value, assessment_submission_id_value, response.question_attempt_id, now_value,
               response.student_response
          FROM ple_private.assessment_attempt_saved_response AS response
         WHERE response.question_attempt_id = evaluation_row.question_attempt_id;
        IF NOT FOUND THEN
            RAISE EXCEPTION USING ERRCODE = '42501',
                MESSAGE = 'Assessment Attempt saved responses changed';
        END IF;
        PERFORM ple_private.record_direct_automated_grading_result(
            question_response_id_value, evaluation_row.question_attempt_id,
            evaluation_row.normalized_credit, now_value
        );
    END LOOP;
    RETURN QUERY
    SELECT coalesce(sum(score.points_earned), 0)::double precision,
           coalesce(sum(score.points_possible), 0)::double precision
      FROM ple_private.issued_question AS issued
      JOIN ple_private.question_attempt AS question_attempt
        ON question_attempt.issued_question_id = issued.issued_question_id
      LEFT JOIN ple_private.grading_result AS result
        ON result.question_attempt_id = question_attempt.question_attempt_id
      JOIN ple_data.assessment_entry AS entry
        ON entry.assessment_entry_id = issued.assessment_entry_id
      CROSS JOIN LATERAL ple_private.score_recorded_credit(
          result.normalized_credit, issued.scoring_rule,
          CASE entry.entry_kind WHEN 'fixed_question' THEN entry.points_possible
               ELSE entry.points_per_item END
      ) AS score
     WHERE issued.assessment_attempt_id = assessment_attempt_row.assessment_attempt_id;
END $$;

