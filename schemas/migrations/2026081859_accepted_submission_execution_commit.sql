-- WP-PROF-G1 / G1-W4: atomic accepted-submission completion commit.
--
-- The server-owned planner supplies one complete completion projection. This
-- capability verifies the current lease and every canonical evidence tuple,
-- then commits the grade and its learner, run, statistics, and recalculation
-- effects in one PostgreSQL transaction.

BEGIN;

-- ASVS 2.3.1-2.3.4, 8.2.1-8.2.3, 8.4.1, 11.4.3, 14.2.6, and 15.4.2:
-- enforce the full tenant and lease state machine beside the atomic writes.
CREATE FUNCTION public.ple_commit_accepted_submission_completion_v2(
    p_tenant_id uuid, p_worker_job_id uuid, p_lease_token uuid,
    p_submission_id uuid, p_execution_generation bigint, p_worker_id uuid,
    p_canonical_json_version smallint, p_evaluation_status text,
    p_evaluation_canonical_json text, p_evaluation_sha256 character(64),
    p_attempt_canonical_json text, p_attempt_payload jsonb,
    p_attempt_payload_sha256 character(64), p_feedback_canonical_json text,
    p_feedback_content_sha256 character(64), p_run_canonical_json text,
    p_run_payload jsonb, p_run_payload_sha256 character(64),
    p_run_current_canonical_json text,
    p_run_current_payload_sha256 character(64),
    p_run_completed_at_millis bigint,
    p_enrollment_first_completed_at_millis bigint,
    p_enrollment_current_grade_run_id uuid, p_enrollment_best_grade_run_id uuid,
    p_summary_canonical_json text, p_summary_payload jsonb,
    p_summary_payload_sha256 character(64), p_presentation_canonical_json text,
    p_presentation_payload jsonb, p_presentation_payload_sha256 character(64),
    p_presentation_required boolean, p_assignment_item_id uuid,
    p_statistics jsonb, p_expected_scoring_generation bigint,
    p_recalculation_job_id uuid, p_recalculation_max_attempts integer
) RETURNS TABLE(
    disposition text,
    resulting_execution_state text,
    resulting_evaluation_status text
)
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public', pg_temp
AS $$
DECLARE
    v_summary_keys constant text[] := ARRAY[
        'tenant', 'enrollment', 'currentScore', 'bestScore',
        'latestScore', 'completedRunCount', 'totalQuestionAttempts', 'lastActivityAt'
    ];
    v_witness public.ple_accepted_submission_execution_witness_v1%ROWTYPE;
    v_evaluation jsonb;
    v_feedback jsonb;
    v_attempt_source jsonb;
    v_run_source jsonb;
    v_run_current_source jsonb;
    v_summary_source jsonb;
    v_presentation_source jsonb;
    v_correct boolean;
    v_points_earned numeric;
    v_points_possible numeric;
    v_run_score double precision;
    v_summary_current_score double precision;
    v_summary_best_score double precision;
    v_summary_latest_score double precision;
    v_summary_completed_run_count bigint;
    v_summary_total_question_attempts bigint;
    v_summary_last_activity_at_millis bigint;
    v_expected_last_activity_at_millis bigint;
    v_expected_best_score double precision;
    v_expected_current_score double precision;
    v_expected_current_grade_run_id uuid;
    v_expected_best_grade_run_id uuid;
    v_attempt_selection_policy text;
    v_statistic jsonb;
    v_next_scoring_generation bigint;
BEGIN
    IF p_tenant_id IS NULL
       OR p_worker_job_id IS NULL
       OR p_lease_token IS NULL
       OR p_submission_id IS NULL
       OR p_execution_generation IS NULL
       OR p_execution_generation <= 0
       OR p_worker_id IS NULL
       OR p_canonical_json_version IS DISTINCT FROM 1
       OR p_evaluation_status IS DISTINCT FROM 'graded'
       OR p_evaluation_canonical_json IS NULL
       OR octet_length(p_evaluation_canonical_json) NOT BETWEEN 1 AND 4096
       OR p_evaluation_sha256 IS NULL
       OR p_evaluation_sha256 !~ '^[0-9a-f]{64}$'
       OR p_attempt_canonical_json IS NULL
       OR octet_length(p_attempt_canonical_json) NOT BETWEEN 1 AND 524288
       OR p_attempt_payload IS NULL
       OR p_attempt_payload_sha256 IS NULL
       OR p_attempt_payload_sha256 !~ '^[0-9a-f]{64}$'
       OR p_feedback_canonical_json IS NULL
       OR octet_length(p_feedback_canonical_json) NOT BETWEEN 1 AND 65536
       OR p_feedback_content_sha256 IS NULL
       OR p_feedback_content_sha256 !~ '^[0-9a-f]{64}$'
       OR p_run_canonical_json IS NULL
       OR octet_length(p_run_canonical_json) NOT BETWEEN 1 AND 524288
       OR p_run_payload IS NULL
       OR p_run_payload_sha256 IS NULL
       OR p_run_payload_sha256 !~ '^[0-9a-f]{64}$'
       OR p_run_current_canonical_json IS NULL
       OR octet_length(p_run_current_canonical_json) NOT BETWEEN 1 AND 524288
       OR p_run_current_payload_sha256 IS NULL
       OR p_run_current_payload_sha256 !~ '^[0-9a-f]{64}$'
       OR p_summary_canonical_json IS NULL
       OR octet_length(p_summary_canonical_json) NOT BETWEEN 1 AND 524288
       OR p_summary_payload IS NULL
       OR p_summary_payload_sha256 IS NULL
       OR p_summary_payload_sha256 !~ '^[0-9a-f]{64}$'
       OR p_presentation_required IS NULL
       OR (
            p_presentation_required
            AND (
                p_presentation_canonical_json IS NULL
                OR octet_length(p_presentation_canonical_json)
                    NOT BETWEEN 1 AND 524288
                OR p_presentation_payload IS NULL
                OR p_presentation_payload_sha256 IS NULL
                OR p_presentation_payload_sha256 !~ '^[0-9a-f]{64}$'
            )
       )
       OR (
            NOT p_presentation_required
            AND (
                p_presentation_canonical_json IS NOT NULL
                OR p_presentation_payload IS NOT NULL
                OR p_presentation_payload_sha256 IS NOT NULL
            )
       )
       OR p_assignment_item_id IS NULL
       OR p_statistics IS NULL
       OR jsonb_typeof(p_statistics) IS DISTINCT FROM 'array'
       OR jsonb_array_length(p_statistics) > 1024
       OR p_expected_scoring_generation IS NULL
       OR p_expected_scoring_generation <= 0
       OR p_recalculation_job_id IS NULL
       OR p_recalculation_max_attempts IS NULL
       OR p_recalculation_max_attempts NOT BETWEEN 1 AND 20
       OR p_tenant_id IS DISTINCT FROM public.ple_current_tenant()
    THEN
        RAISE EXCEPTION 'accepted-submission completion arguments are invalid'
            USING ERRCODE = '22023';
    END IF;

    -- Each immutable digest attests the exact UTF-8 canonical source, never a
    -- database-reserialized JSONB value.
    IF p_evaluation_sha256 IS DISTINCT FROM encode(
        pg_catalog.sha256(convert_to(p_evaluation_canonical_json, 'UTF8')),
        'hex'
    ) OR p_attempt_payload_sha256 IS DISTINCT FROM encode(
        pg_catalog.sha256(convert_to(p_attempt_canonical_json, 'UTF8')),
        'hex'
    ) OR p_feedback_content_sha256 IS DISTINCT FROM encode(
        pg_catalog.sha256(convert_to(p_feedback_canonical_json, 'UTF8')),
        'hex'
    ) OR p_run_payload_sha256 IS DISTINCT FROM encode(
        pg_catalog.sha256(convert_to(p_run_canonical_json, 'UTF8')),
        'hex'
    ) OR p_run_current_payload_sha256 IS DISTINCT FROM encode(
        pg_catalog.sha256(convert_to(p_run_current_canonical_json, 'UTF8')),
        'hex'
    ) OR p_summary_payload_sha256 IS DISTINCT FROM encode(
        pg_catalog.sha256(convert_to(p_summary_canonical_json, 'UTF8')),
        'hex'
    ) OR (
        p_presentation_required
        AND p_presentation_payload_sha256 IS DISTINCT FROM encode(
            pg_catalog.sha256(
                convert_to(p_presentation_canonical_json, 'UTF8')
            ),
            'hex'
        )
    ) THEN
        RAISE EXCEPTION 'accepted-submission completion digest is invalid'
            USING ERRCODE = '22023';
    END IF;

    BEGIN
        v_evaluation := p_evaluation_canonical_json::jsonb;
        v_attempt_source := p_attempt_canonical_json::jsonb;
        v_feedback := p_feedback_canonical_json::jsonb;
        v_run_source := p_run_canonical_json::jsonb;
        v_run_current_source := p_run_current_canonical_json::jsonb;
        v_summary_source := p_summary_canonical_json::jsonb;
        v_presentation_source := CASE
            WHEN p_presentation_required
                THEN p_presentation_canonical_json::jsonb
            ELSE NULL
        END;
    EXCEPTION
        WHEN invalid_text_representation OR numeric_value_out_of_range THEN
            RAISE EXCEPTION 'accepted-submission canonical evidence is invalid'
                USING ERRCODE = '22023';
    END;

    IF v_attempt_source IS DISTINCT FROM p_attempt_payload
       OR v_run_source IS DISTINCT FROM p_run_payload
       OR v_run_current_source IS DISTINCT FROM p_run_payload
       OR v_summary_source IS DISTINCT FROM p_summary_payload
       OR v_presentation_source IS DISTINCT FROM p_presentation_payload
    THEN
        RAISE EXCEPTION 'accepted-submission canonical source disagrees with projection'
            USING ERRCODE = '22023';
    END IF;

    IF jsonb_typeof(v_evaluation) IS DISTINCT FROM 'object'
       OR NOT v_evaluation ?& ARRAY[
            'correct',
            'pointsEarned',
            'pointsPossible'
       ]
       OR v_evaluation - ARRAY[
            'correct',
            'pointsEarned',
            'pointsPossible'
       ] <> '{}'::jsonb
       OR jsonb_typeof(v_evaluation -> 'correct') IS DISTINCT FROM 'boolean'
       OR jsonb_typeof(v_evaluation -> 'pointsEarned') IS DISTINCT FROM 'number'
       OR jsonb_typeof(v_evaluation -> 'pointsPossible') IS DISTINCT FROM 'number'
    THEN
        RAISE EXCEPTION 'accepted-submission result shape is invalid'
            USING ERRCODE = '22023';
    END IF;

    BEGIN
        v_correct := (v_evaluation ->> 'correct')::boolean;
        v_points_earned := (v_evaluation ->> 'pointsEarned')::numeric;
        v_points_possible := (v_evaluation ->> 'pointsPossible')::numeric;
    EXCEPTION
        WHEN invalid_text_representation OR numeric_value_out_of_range THEN
            RAISE EXCEPTION 'accepted-submission result scalars are invalid'
                USING ERRCODE = '22023';
    END;

    IF v_points_possible <= 0
       OR v_points_earned / v_points_possible NOT BETWEEN -1000 AND 1000
    THEN
        RAISE EXCEPTION 'accepted-submission result scalars are out of range'
            USING ERRCODE = '22023';
    END IF;

    IF jsonb_typeof(v_feedback) IS DISTINCT FROM 'array'
       OR jsonb_array_length(v_feedback) <> 3
       OR EXISTS (
            SELECT 1
              FROM jsonb_array_elements(v_feedback) AS entry(value)
             WHERE jsonb_typeof(entry.value) NOT IN ('null', 'array')
       )
    THEN
        RAISE EXCEPTION 'accepted-submission feedback evidence is invalid'
            USING ERRCODE = '22023';
    END IF;

    IF jsonb_typeof(v_summary_source) IS DISTINCT FROM 'object'
       OR NOT v_summary_source ?& v_summary_keys
       OR v_summary_source - v_summary_keys <> '{}'::jsonb
       OR jsonb_typeof(v_summary_source -> 'tenant') IS DISTINCT FROM 'string'
       OR jsonb_typeof(v_summary_source -> 'enrollment') IS DISTINCT FROM 'string'
       OR jsonb_typeof(v_summary_source -> 'currentScore') NOT IN ('null', 'number')
       OR jsonb_typeof(v_summary_source -> 'bestScore') NOT IN ('null', 'number')
       OR jsonb_typeof(v_summary_source -> 'latestScore') NOT IN ('null', 'number')
       OR jsonb_typeof(v_summary_source -> 'completedRunCount')
            IS DISTINCT FROM 'number'
       OR jsonb_typeof(v_summary_source -> 'totalQuestionAttempts')
            IS DISTINCT FROM 'number'
       OR jsonb_typeof(v_summary_source -> 'lastActivityAt') NOT IN ('null', 'number')
       OR (v_summary_source ->> 'completedRunCount') !~ '^(0|[1-9][0-9]*)$'
       OR (v_summary_source ->> 'totalQuestionAttempts') !~ '^(0|[1-9][0-9]*)$'
       OR (
            v_summary_source -> 'lastActivityAt' <> 'null'::jsonb
            AND (v_summary_source ->> 'lastActivityAt') !~ '^-?(0|[1-9][0-9]*)$'
       )
    THEN
        RAISE EXCEPTION 'accepted-submission summary shape is invalid'
            USING ERRCODE = '22023';
    END IF;

    BEGIN
        v_summary_current_score := CASE
            WHEN v_summary_source -> 'currentScore' = 'null'::jsonb THEN NULL
            ELSE (v_summary_source ->> 'currentScore')::double precision
        END;
        v_summary_best_score := CASE
            WHEN v_summary_source -> 'bestScore' = 'null'::jsonb THEN NULL
            ELSE (v_summary_source ->> 'bestScore')::double precision
        END;
        v_summary_latest_score := CASE
            WHEN v_summary_source -> 'latestScore' = 'null'::jsonb THEN NULL
            ELSE (v_summary_source ->> 'latestScore')::double precision
        END;
        v_summary_completed_run_count :=
            (v_summary_source ->> 'completedRunCount')::bigint;
        v_summary_total_question_attempts :=
            (v_summary_source ->> 'totalQuestionAttempts')::bigint;
        v_summary_last_activity_at_millis := CASE
            WHEN v_summary_source -> 'lastActivityAt' = 'null'::jsonb THEN NULL
            ELSE (v_summary_source ->> 'lastActivityAt')::bigint
        END;
    EXCEPTION
        WHEN invalid_text_representation OR numeric_value_out_of_range THEN
            RAISE EXCEPTION 'accepted-submission summary scalars are invalid'
                USING ERRCODE = '22023';
    END;

    IF (
        v_summary_current_score IS NOT NULL
        AND v_summary_current_score NOT BETWEEN -1000 AND 1000
    ) OR (
        v_summary_best_score IS NOT NULL
        AND v_summary_best_score NOT BETWEEN -1000 AND 1000
    ) OR (
        v_summary_latest_score IS NOT NULL
        AND v_summary_latest_score NOT BETWEEN -1000 AND 1000
    ) OR v_summary_completed_run_count NOT BETWEEN 0 AND 4294967295
       OR v_summary_total_question_attempts < 0
    THEN
        RAISE EXCEPTION 'accepted-submission summary scalars are out of range'
            USING ERRCODE = '22023';
    END IF;

    -- Repeat the complete lease and accepted-response witness in the same
    -- transaction that writes the aggregate. A stale or racing holder gets a
    -- closed, answer-free disposition and changes no row.
    SELECT witness, assignment.attempt_selection_policy
      INTO v_witness, v_attempt_selection_policy
      FROM public.ple_accepted_submission_execution_witness_v1 AS witness
      JOIN public.grading_execution AS execution
        ON (execution.tenant_id, execution.attempt_id) =
           (witness.tenant_id, witness.attempt_id)
      JOIN public.worker_job AS job
        ON (job.tenant_id, job.job_id) =
           (witness.tenant_id, witness.current_job_id)
      JOIN public.submission_evaluation AS evaluation
        ON (evaluation.tenant_id, evaluation.attempt_id, evaluation.submission_id) =
           (witness.tenant_id, witness.attempt_id, witness.submission_id)
      JOIN public.question_attempt AS attempt
        ON (attempt.tenant_id, attempt.attempt_id) =
           (witness.tenant_id, witness.attempt_id)
      JOIN public.assignment_run AS run
        ON (run.tenant_id, run.run_id) =
           (witness.tenant_id, witness.run_id)
      JOIN public.enrollment AS enrollment
        ON (enrollment.tenant_id, enrollment.enrollment_id) =
           (witness.tenant_id, witness.enrollment_id)
      JOIN public.student_assignment_summary AS summary
        ON (summary.tenant_id, summary.enrollment_id) =
           (witness.tenant_id, witness.enrollment_id)
      JOIN public.assignment AS assignment
        ON (assignment.tenant_id, assignment.assignment_id) =
           (witness.tenant_id, witness.assignment_id)
     WHERE witness.tenant_id = p_tenant_id
       AND witness.current_job_id = p_worker_job_id
       AND witness.submission_id = p_submission_id
       AND witness.execution_generation = p_execution_generation
       AND witness.active_worker_id = p_worker_id
       AND witness.execution_state = 'running'
       AND witness.job_state = 'leased'
       AND witness.lease_token = p_lease_token
       AND witness.lease_expires_at > transaction_timestamp()
       AND job.payload = jsonb_build_object(
            'kind',
            'gradeAcceptedSubmission',
            'attempt',
            witness.attempt_id::text,
            'submission',
            witness.submission_id::text,
            'execution_generation',
            witness.execution_generation
       )
       AND witness.retention_lifecycle = 'active'
       AND witness.grading_status = 'automated_pending'
       AND witness.automated_result_canonical_json IS NULL
       AND witness.automated_result_sha256 IS NULL
       AND witness.run_completed_at IS NULL
       AND attempt.attempt_status = 'in_progress'
       AND attempt.submitted_at IS NULL
       AND witness.scoring_generation = p_expected_scoring_generation
       AND assignment.revision = witness.assignment_revision
       AND assignment.scoring_generation = p_expected_scoring_generation
    FOR UPDATE OF execution, job, evaluation, attempt, run, enrollment, summary;

    IF NOT FOUND THEN
        RETURN QUERY
        SELECT
            'claim_no_longer_active',
            NULL::text,
            NULL::text;
        RETURN;
    END IF;

    IF p_attempt_payload ->> 'id' IS DISTINCT FROM v_witness.attempt_id::text
       OR p_attempt_payload ->> 'tenant' IS DISTINCT FROM p_tenant_id::text
       OR p_attempt_payload ->> 'run' IS DISTINCT FROM v_witness.run_id::text
       OR p_attempt_payload -> 'response' IS DISTINCT FROM 'null'::jsonb
       OR p_attempt_payload ->> 'status' IS DISTINCT FROM 'submitted'
       OR p_attempt_payload -> 'result' IS DISTINCT FROM v_evaluation
       OR (
            (p_attempt_payload - ARRAY['response', 'status', 'result'])
                #- '{timer,submittedAt}'
       ) IS DISTINCT FROM (
            (v_witness.attempt_payload - ARRAY['response', 'status', 'result'])
                #- '{timer,submittedAt}'
       )
       OR p_attempt_payload #>> '{timer,submittedAt}'
            IS DISTINCT FROM v_witness.accepted_millis::text
    THEN
        RAISE EXCEPTION 'accepted-submission attempt plan disagrees with locked evidence'
            USING ERRCODE = '22023';
    END IF;

    IF p_run_payload ->> 'id' IS DISTINCT FROM v_witness.run_id::text
       OR (p_run_payload - ARRAY['completedAt', 'score'])
            IS DISTINCT FROM
          (v_witness.run_payload - ARRAY['completedAt', 'score'])
       OR p_run_payload #>> '{completedAt}'
            IS DISTINCT FROM p_run_completed_at_millis::text
    THEN
        RAISE EXCEPTION 'accepted-submission run plan disagrees with locked evidence'
            USING ERRCODE = '22023';
    END IF;

    BEGIN
        v_run_score := CASE
            WHEN p_run_payload -> 'score' = 'null'::jsonb THEN NULL
            ELSE (p_run_payload ->> 'score')::double precision
        END;
    EXCEPTION
        WHEN invalid_text_representation OR numeric_value_out_of_range THEN
            RAISE EXCEPTION 'accepted-submission run score is invalid'
                USING ERRCODE = '22023';
    END;

    IF (
        p_run_completed_at_millis IS NULL
        AND (
            p_run_payload -> 'completedAt' IS DISTINCT FROM 'null'::jsonb
            OR p_run_payload -> 'score' IS DISTINCT FROM 'null'::jsonb
            OR v_run_score IS NOT NULL
        )
    ) OR (
        p_run_completed_at_millis IS NOT NULL
        AND (
            p_run_completed_at_millis IS DISTINCT FROM v_witness.accepted_millis
            OR jsonb_typeof(p_run_payload -> 'score') IS DISTINCT FROM 'number'
            OR v_run_score NOT BETWEEN -1000 AND 1000
        )
    ) THEN
        RAISE EXCEPTION 'accepted-submission run transition is invalid'
            USING ERRCODE = '22023';
    END IF;

    v_expected_last_activity_at_millis := CASE
        WHEN v_witness.summary_last_activity_at_millis IS NULL
            THEN v_witness.accepted_millis
        ELSE GREATEST(
            v_witness.summary_last_activity_at_millis,
            v_witness.accepted_millis
        )
    END;

    IF v_witness.total_question_attempts = 9223372036854775807 THEN
        RAISE EXCEPTION 'question-attempt count cannot advance'
            USING ERRCODE = '22023';
    END IF;

    IF p_summary_payload ->> 'tenant' IS DISTINCT FROM p_tenant_id::text
       OR p_summary_payload ->> 'enrollment'
            IS DISTINCT FROM v_witness.enrollment_id::text
       OR v_summary_total_question_attempts
            IS DISTINCT FROM v_witness.total_question_attempts + 1
       OR v_summary_last_activity_at_millis
            IS DISTINCT FROM v_expected_last_activity_at_millis
    THEN
        RAISE EXCEPTION 'accepted-submission summary identity or activity is invalid'
            USING ERRCODE = '22023';
    END IF;

    IF p_run_completed_at_millis IS NULL THEN
        IF v_summary_current_score
                IS DISTINCT FROM v_witness.summary_current_score
           OR v_summary_best_score
                IS DISTINCT FROM v_witness.summary_best_score
           OR v_summary_latest_score
                IS DISTINCT FROM v_witness.summary_latest_score
           OR v_summary_completed_run_count
                IS DISTINCT FROM v_witness.completed_run_count
           OR p_enrollment_first_completed_at_millis
                IS DISTINCT FROM floor(
                    extract(epoch FROM v_witness.first_completed_at) * 1000
                )::bigint
           OR p_enrollment_current_grade_run_id
                IS DISTINCT FROM v_witness.current_grade_run_id
           OR p_enrollment_best_grade_run_id
                IS DISTINCT FROM v_witness.best_grade_run_id
        THEN
            RAISE EXCEPTION 'incomplete run changed completion projections'
                USING ERRCODE = '22023';
        END IF;

        IF jsonb_array_length(p_statistics) <> 0 THEN
            RAISE EXCEPTION 'incomplete run supplied statistics evidence'
                USING ERRCODE = '22023';
        END IF;
    ELSE
        IF v_witness.completed_run_count >= 4294967295 THEN
            RAISE EXCEPTION 'completed-run count cannot advance'
                USING ERRCODE = '22023';
        END IF;

        v_expected_best_score := CASE
            WHEN v_witness.summary_best_score IS NULL THEN v_run_score
            ELSE GREATEST(v_witness.summary_best_score, v_run_score)
        END;
        v_expected_current_score := CASE v_attempt_selection_policy
            WHEN 'first' THEN COALESCE(
                v_witness.summary_current_score,
                v_run_score
            )
            WHEN 'last' THEN v_run_score
            WHEN 'highest' THEN v_expected_best_score
            WHEN 'instructor_selected' THEN v_witness.summary_current_score
        END;
        v_expected_current_grade_run_id := CASE v_attempt_selection_policy
            WHEN 'first' THEN CASE
                WHEN v_witness.completed_run_count = 0 THEN v_witness.run_id
                ELSE v_witness.current_grade_run_id
            END
            WHEN 'last' THEN v_witness.run_id
            WHEN 'highest' THEN CASE
                WHEN v_witness.summary_best_score IS NULL
                     OR v_run_score > v_witness.summary_best_score
                    THEN v_witness.run_id
                ELSE v_witness.current_grade_run_id
            END
            WHEN 'instructor_selected' THEN v_witness.current_grade_run_id
        END;
        v_expected_best_grade_run_id := CASE
            WHEN v_witness.summary_best_score IS NULL
                 OR v_witness.best_grade_run_id IS NULL
                 OR v_run_score > v_witness.summary_best_score
                THEN v_witness.run_id
            ELSE v_witness.best_grade_run_id
        END;

        IF v_summary_completed_run_count
                IS DISTINCT FROM v_witness.completed_run_count + 1
           OR v_summary_latest_score IS DISTINCT FROM v_run_score
           OR v_summary_best_score IS DISTINCT FROM v_expected_best_score
           OR v_summary_current_score IS DISTINCT FROM v_expected_current_score
           OR p_enrollment_first_completed_at_millis IS DISTINCT FROM CASE
                WHEN v_witness.first_completed_at IS NULL
                    THEN v_witness.accepted_millis
                ELSE floor(
                    extract(epoch FROM v_witness.first_completed_at) * 1000
                )::bigint
           END
           OR p_enrollment_current_grade_run_id
                IS DISTINCT FROM v_expected_current_grade_run_id
           OR p_enrollment_best_grade_run_id
                IS DISTINCT FROM v_expected_best_grade_run_id
        THEN
            RAISE EXCEPTION 'completed run projection is incoherent'
                USING ERRCODE = '22023';
        END IF;

        IF jsonb_array_length(p_statistics) > 0
           AND (
                v_witness.completed_run_count <> 0
                OR v_witness.run_payload ->> 'mode' IS DISTINCT FROM 'assigned'
           )
        THEN
            RAISE EXCEPTION 'statistics evidence is not first assigned completion'
                USING ERRCODE = '22023';
        END IF;
    END IF;

    IF NOT EXISTS (
        SELECT 1
          FROM public.assignment_run_item AS item
         WHERE item.tenant_id = p_tenant_id
           AND item.run_id = v_witness.run_id
           AND item.assignment_item_id = p_assignment_item_id
           AND item.issued_position = v_witness.assignment_position
    ) OR p_presentation_required IS DISTINCT FROM v_witness.presentation_required
       OR (
            p_presentation_required
            AND p_presentation_payload
                IS DISTINCT FROM v_witness.presentation_payload
       )
    THEN
        RAISE EXCEPTION 'accepted-submission item or presentation evidence is invalid'
            USING ERRCODE = '22023';
    END IF;

    -- Lock and advance the exact assignment generation before applying the
    -- projection. Any later exception rolls this enqueueing back with every
    -- other completion effect.
    SELECT public.ple_enqueue_assignment_recalculation(
        p_tenant_id,
        v_witness.assignment_id,
        p_recalculation_job_id,
        p_recalculation_max_attempts
    ) INTO v_next_scoring_generation;

    IF v_next_scoring_generation IS DISTINCT FROM p_expected_scoring_generation + 1 THEN
        RAISE EXCEPTION 'assignment scoring generation changed during completion'
            USING ERRCODE = '40001';
    END IF;

    -- Complete the answer-free lifecycle while leaving the immutable issued
    -- attempt payload and accepted private response untouched.
    UPDATE public.question_attempt AS attempt
       SET attempt_status = 'submitted',
           submitted_at = to_timestamp(
               v_witness.accepted_millis::double precision / 1000
           )
     WHERE attempt.tenant_id = p_tenant_id
       AND attempt.attempt_id = v_witness.attempt_id
       AND attempt.attempt_status = 'in_progress'
       AND attempt.submitted_at IS NULL;

    IF NOT FOUND THEN
        RAISE EXCEPTION 'accepted-submission attempt lifecycle changed'
            USING ERRCODE = '40001';
    END IF;

    INSERT INTO public.attempt_feedback(
        tenant_id,
        attempt_id,
        hint,
        correct_response,
        rationale,
        content_canonical_json,
        content_canonical_json_version,
        content_sha256,
        course_id
    ) VALUES (
        p_tenant_id,
        v_witness.attempt_id,
        NULLIF(v_feedback -> 0, 'null'::jsonb),
        NULLIF(v_feedback -> 1, 'null'::jsonb),
        NULLIF(v_feedback -> 2, 'null'::jsonb),
        p_feedback_canonical_json,
        p_canonical_json_version,
        p_feedback_content_sha256,
        v_witness.course_id
    );

    UPDATE public.submission_evaluation AS evaluation
       SET grading_status = 'graded',
           correct = v_correct,
           credit_fraction = v_points_earned / v_points_possible,
           payload = v_evaluation,
           payload_sha256 = p_evaluation_sha256,
           automated_result_canonical_json = p_evaluation_canonical_json,
           automated_result_sha256 = p_evaluation_sha256,
           automated_result_canonical_json_version = p_canonical_json_version,
           evaluated_at = transaction_timestamp(),
           evaluation_revision = evaluation.evaluation_revision + 1
     WHERE evaluation.tenant_id = p_tenant_id
       AND evaluation.attempt_id = v_witness.attempt_id
       AND evaluation.submission_id = p_submission_id
       AND evaluation.grading_status = 'automated_pending'
       AND evaluation.automated_result_canonical_json IS NULL
       AND evaluation.automated_result_sha256 IS NULL;

    IF NOT FOUND THEN
        RAISE EXCEPTION 'accepted-submission evaluation lifecycle changed'
            USING ERRCODE = '40001';
    END IF;

    UPDATE public.assignment_run AS run
       SET payload = p_run_payload,
           payload_sha256 = p_run_current_payload_sha256,
           completed_at = to_timestamp(
               p_run_completed_at_millis::double precision / 1000
           )
     WHERE run.tenant_id = p_tenant_id
       AND run.run_id = v_witness.run_id
       AND run.payload IS NOT DISTINCT FROM v_witness.run_payload
       AND run.payload_sha256 IS NOT DISTINCT FROM v_witness.run_payload_sha256
       AND run.completed_at IS NOT DISTINCT FROM v_witness.run_completed_at;

    IF NOT FOUND THEN
        RAISE EXCEPTION 'accepted-submission run lifecycle changed'
            USING ERRCODE = '40001';
    END IF;

    UPDATE public.enrollment AS enrollment
       SET first_completed_at = to_timestamp(
               p_enrollment_first_completed_at_millis::double precision / 1000
           ),
           current_grade_run_id = p_enrollment_current_grade_run_id,
           best_grade_run_id = p_enrollment_best_grade_run_id
     WHERE enrollment.tenant_id = p_tenant_id
       AND enrollment.enrollment_id = v_witness.enrollment_id
       AND enrollment.first_completed_at
            IS NOT DISTINCT FROM v_witness.first_completed_at
       AND enrollment.current_grade_run_id
            IS NOT DISTINCT FROM v_witness.current_grade_run_id
       AND enrollment.best_grade_run_id
            IS NOT DISTINCT FROM v_witness.best_grade_run_id;

    IF NOT FOUND THEN
        RAISE EXCEPTION 'accepted-submission enrollment lifecycle changed'
            USING ERRCODE = '40001';
    END IF;

    UPDATE public.student_assignment_summary AS summary
       SET current_score = v_summary_current_score,
           best_score = v_summary_best_score,
           latest_score = v_summary_latest_score,
           completed_run_count = v_summary_completed_run_count,
           total_question_attempts = v_summary_total_question_attempts,
           last_activity_at = to_timestamp(
               v_summary_last_activity_at_millis::double precision / 1000
           ),
           updated_at = transaction_timestamp()
     WHERE summary.tenant_id = p_tenant_id
       AND summary.enrollment_id = v_witness.enrollment_id
       AND summary.current_score
            IS NOT DISTINCT FROM v_witness.summary_current_score
       AND summary.best_score IS NOT DISTINCT FROM v_witness.summary_best_score
       AND summary.latest_score IS NOT DISTINCT FROM v_witness.summary_latest_score
       AND summary.completed_run_count = v_witness.completed_run_count
       AND summary.total_question_attempts = v_witness.total_question_attempts
       AND floor(extract(epoch FROM summary.last_activity_at) * 1000)::bigint
            IS NOT DISTINCT FROM v_witness.summary_last_activity_at_millis;

    IF NOT FOUND THEN
        RAISE EXCEPTION 'accepted-submission summary lifecycle changed'
            USING ERRCODE = '40001';
    END IF;

    INSERT INTO public.submission_receipt_snapshot(
        tenant_id,
        attempt_id,
        canonical_json_version,
        receipt_attempt_canonical_json,
        receipt_attempt_payload,
        receipt_attempt_payload_sha256,
        run_canonical_json,
        run_payload,
        run_payload_sha256,
        summary_canonical_json,
        summary_payload,
        summary_payload_sha256,
        presentation_canonical_json,
        presentation_payload,
        presentation_payload_sha256,
        presentation_required
    ) VALUES (
        p_tenant_id,
        v_witness.attempt_id,
        p_canonical_json_version,
        p_attempt_canonical_json,
        p_attempt_payload,
        p_attempt_payload_sha256,
        p_run_canonical_json,
        p_run_payload,
        p_run_payload_sha256,
        p_summary_canonical_json,
        p_summary_payload,
        p_summary_payload_sha256,
        p_presentation_canonical_json,
        p_presentation_payload,
        p_presentation_payload_sha256,
        p_presentation_required
    );

    FOR v_statistic IN
        SELECT entry.value
          FROM jsonb_array_elements(p_statistics) AS entry(value)
    LOOP
        PERFORM public.ple_record_question_statistics(
            p_tenant_id,
            v_witness.enrollment_id,
            v_witness.run_id,
            (v_statistic ->> 'attemptId')::uuid,
            (v_statistic ->> 'problemId')::uuid,
            (v_statistic ->> 'versionId')::uuid,
            (v_statistic ->> 'normalizedScore')::double precision,
            (v_statistic ->> 'attempts')::bigint,
            (v_statistic ->> 'durationSeconds')::bigint,
            (v_statistic ->> 'restScore')::double precision,
            decode(v_statistic ->> 'observationSha256', 'hex')
        );
    END LOOP;

    UPDATE public.grading_execution AS execution
       SET state = 'completed',
           active_worker_id = NULL,
           updated_at = transaction_timestamp()
     WHERE execution.tenant_id = p_tenant_id
       AND execution.attempt_id = v_witness.attempt_id
       AND execution.current_job_id = p_worker_job_id
       AND execution.submission_id = p_submission_id
       AND execution.execution_generation = p_execution_generation
       AND execution.state = 'running'
       AND execution.active_worker_id = p_worker_id;

    IF NOT FOUND THEN
        RAISE EXCEPTION 'accepted-submission execution lease changed'
            USING ERRCODE = '40001';
    END IF;

    UPDATE public.worker_job AS job
       SET state = 'completed',
           lease_token = NULL,
           lease_expires_at = NULL,
           completed_at = transaction_timestamp()
     WHERE job.tenant_id = p_tenant_id
       AND job.job_id = p_worker_job_id
       AND job.state = 'leased'
       AND job.lease_token = p_lease_token
       AND job.lease_expires_at > transaction_timestamp();

    IF NOT FOUND THEN
        RAISE EXCEPTION 'accepted-submission worker lease changed'
            USING ERRCODE = '40001';
    END IF;

    INSERT INTO public.grading_execution_receipt(
        tenant_id,
        receipt_id,
        attempt_id,
        submission_id,
        submission_occurred_at,
        course_id,
        execution_generation,
        resulting_state,
        worker_id
    ) VALUES (
        p_tenant_id,
        gen_random_uuid(),
        v_witness.attempt_id,
        p_submission_id,
        v_witness.submission_occurred_at,
        v_witness.course_id,
        p_execution_generation,
        'completed',
        p_worker_id
    );

    RETURN QUERY
    SELECT
        'committed',
        'completed',
        'graded';
END;
$$;

ALTER FUNCTION public.ple_commit_accepted_submission_completion_v2(
    uuid, uuid, uuid, uuid, bigint, uuid,
    smallint, text, text, character, text, jsonb,
    character, text, character, text, jsonb, character,
    text, character, bigint, bigint, uuid, uuid,
    text, jsonb, character, text, jsonb, character,
    boolean, uuid, jsonb, bigint, uuid, integer
) OWNER TO ple_accepted_submission_execution_worker;

-- Normalize every creation-time non-owner ACL, including grants introduced by
-- ALTER DEFAULT PRIVILEGES, before installing the closed caller set.
DO $$
DECLARE
    v_function regprocedure :=
        'public.ple_commit_accepted_submission_completion_v2('
        'uuid,uuid,uuid,uuid,bigint,uuid,smallint,text,text,character,'
        'text,jsonb,character,text,character,text,jsonb,character,text,character,bigint,'
        'bigint,uuid,uuid,text,jsonb,character,text,jsonb,character,boolean,uuid,jsonb,'
        'bigint,uuid,integer)'::regprocedure;
    v_signature text;
    v_grantee record;
BEGIN
    SELECT format('%I.%I(%s)', namespace_row.nspname, procedure_row.proname,
                  pg_get_function_identity_arguments(procedure_row.oid))
      INTO v_signature
      FROM pg_catalog.pg_proc AS procedure_row
      JOIN pg_catalog.pg_namespace AS namespace_row
        ON namespace_row.oid = procedure_row.pronamespace
     WHERE procedure_row.oid = v_function;

    FOR v_grantee IN
        SELECT DISTINCT acl.grantee, role_row.rolname
          FROM pg_catalog.pg_proc AS procedure_row
          CROSS JOIN LATERAL pg_catalog.aclexplode(
              COALESCE(procedure_row.proacl,
                       pg_catalog.acldefault('f', procedure_row.proowner))
          ) AS acl
          LEFT JOIN pg_catalog.pg_roles AS role_row
                 ON role_row.oid = acl.grantee
         WHERE procedure_row.oid = v_function
           AND acl.grantee <> procedure_row.proowner
    LOOP
        IF v_grantee.grantee = 0 THEN
            EXECUTE format(
                'REVOKE ALL PRIVILEGES ON FUNCTION %s FROM PUBLIC', v_signature
            );
        ELSE
            EXECUTE format(
                'REVOKE ALL PRIVILEGES ON FUNCTION %s FROM %I',
                v_signature, v_grantee.rolname
            );
        END IF;
    END LOOP;

    EXECUTE format(
        'GRANT EXECUTE ON FUNCTION %s TO %I, %I',
        v_signature,
        'ple_accepted_submission_execution',
        'ple_accepted_submission_execution_fast_path'
    );

    -- Prove the frozen interface and the complete closed non-owner ACL without
    -- coupling the migration to function-body text or a named deny list.
    IF EXISTS (
        SELECT 1
          FROM pg_catalog.pg_proc AS procedure_row
         WHERE procedure_row.oid = v_function
           AND (
                procedure_row.pronargs <> 36
                OR procedure_row.proowner
                    <> 'ple_accepted_submission_execution_worker'::regrole
                OR NOT procedure_row.prosecdef
                OR procedure_row.proconfig IS DISTINCT FROM
                    ARRAY['search_path=pg_catalog, public, pg_temp']
           )
    ) OR EXISTS (
        WITH expected(grantee, privilege_type, is_grantable) AS (
            VALUES
                (
                    'ple_accepted_submission_execution'::regrole::oid,
                    'EXECUTE'::text, false
                ),
                (
                    'ple_accepted_submission_execution_fast_path'::regrole::oid,
                    'EXECUTE'::text, false
                )
        ),
        actual AS (
            SELECT acl.grantee, acl.privilege_type, acl.is_grantable
              FROM pg_catalog.pg_proc AS procedure_row
              CROSS JOIN LATERAL pg_catalog.aclexplode(
                  COALESCE(procedure_row.proacl,
                           pg_catalog.acldefault('f', procedure_row.proowner))
              ) AS acl
             WHERE procedure_row.oid = v_function
               AND acl.grantee <> procedure_row.proowner
        )
        SELECT 1
          FROM (
              (SELECT * FROM expected EXCEPT SELECT * FROM actual)
              UNION ALL
              (SELECT * FROM actual EXCEPT SELECT * FROM expected)
          ) AS acl_difference
    )
    THEN
        RAISE EXCEPTION 'accepted-submission completion capability is unsafe';
    END IF;
END;
$$;

COMMIT;
