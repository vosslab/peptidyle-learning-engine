-- Initial database epoch: operations analytics.

SET check_function_bodies = false;

CREATE FUNCTION public.ple_bind_student_record_course() RETURNS trigger
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'pg_catalog', 'public'
    AS $$
DECLARE bound_course uuid;
BEGIN
    IF NEW.delivery_kind <> 'student_record' THEN
        RETURN NEW;
    END IF;
    IF NEW.course_id IS NULL THEN
        SELECT r.course_id INTO bound_course
          FROM public.student_export_artifact a
          JOIN public.student_export_request r ON r.export_id = a.export_id
         WHERE a.object_id = NEW.object_id
           AND r.tenant_id = NEW.tenant_id;
        IF NOT FOUND THEN
            RAISE EXCEPTION 'student-record delivery has no course owner'
                USING ERRCODE = '23503';
        END IF;
        NEW.course_id = bound_course;
    END IF;
    -- The export broker intentionally has BYPASSRLS for cross-tenant queue
    -- claims. Re-check the lifecycle predicate explicitly inside this trigger
    -- so that capability cannot recreate a protected delivery after archive
    -- preparation fenced the course.
    IF NOT public.ple_course_records_accessible(NEW.tenant_id, NEW.course_id) THEN
        RAISE EXCEPTION 'student-record delivery course is unavailable'
            USING ERRCODE = '23503';
    END IF;
    IF NEW.payload #>> '{scope,course}' IS DISTINCT FROM NEW.course_id::text THEN
        RAISE EXCEPTION 'student-record delivery course mismatch'
            USING ERRCODE = '22023';
    END IF;
    RETURN NEW;
END $$;

CREATE FUNCTION public.ple_claim_worker_job(p_token uuid, p_lease_seconds integer) RETURNS TABLE(job_id uuid, tenant_id uuid, payload jsonb, lease_token uuid, attempt_count integer)
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'pg_catalog', 'public'
    AS $$
BEGIN
    IF p_token IS NULL OR p_lease_seconds NOT BETWEEN 1 AND 900 THEN
        RAISE EXCEPTION 'invalid queue claim arguments' USING ERRCODE = '22023';
    END IF;

    -- A process that dies on its final lease cannot strand an operational row.
    UPDATE public.worker_job AS expired
       SET state = 'dead',
           lease_token = NULL,
           lease_expires_at = NULL,
           last_error = 'timed_out',
           completed_at = transaction_timestamp()
     WHERE expired.state = 'leased'
       AND expired.lease_expires_at <= transaction_timestamp()
       AND expired.attempt_count >= expired.max_attempts;

    RETURN QUERY
    WITH candidate AS (
        SELECT queued.job_id
          FROM public.worker_job AS queued
         WHERE (
                queued.state = 'ready'
            AND queued.available_at <= transaction_timestamp()
         ) OR (
                queued.state = 'leased'
            AND queued.lease_expires_at <= transaction_timestamp()
            AND queued.attempt_count < queued.max_attempts
         )
         ORDER BY queued.available_at, queued.job_id
         FOR UPDATE SKIP LOCKED
         LIMIT 1
    ), claimed AS (
        UPDATE public.worker_job AS queued
           SET state = 'leased',
               lease_token = p_token,
               lease_expires_at = transaction_timestamp()
                    + make_interval(secs => p_lease_seconds),
               attempt_count = queued.attempt_count + 1,
               last_error = NULL,
               completed_at = NULL
          FROM candidate
         WHERE queued.job_id = candidate.job_id
        RETURNING queued.job_id, queued.tenant_id, queued.payload,
                  queued.lease_token, queued.attempt_count
    )
    SELECT * FROM claimed;
END
$$;

CREATE FUNCTION public.ple_commit_export_job(p_tenant uuid, p_job uuid, p_token uuid, p_manifest uuid, p_artifacts jsonb) RETURNS text
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'pg_catalog', 'public'
    AS $$
DECLARE
    request_id uuid;
    existing jsonb;
    supplied jsonb;
BEGIN
    IF p_tenant <> public.ple_current_tenant() OR jsonb_typeof(p_artifacts) <> 'array'
       OR jsonb_array_length(p_artifacts) <> 4 THEN
        RAISE EXCEPTION 'invalid export commit capability' USING ERRCODE = '22023';
    END IF;
    SELECT export_id INTO request_id FROM public.student_export_request
     WHERE tenant_id = p_tenant AND job_id = p_job AND manifest_object_id = p_manifest FOR UPDATE;
    IF request_id IS NULL THEN RETURN NULL; END IF;
    SELECT jsonb_agg(jsonb_build_object('kind', a.kind, 'object', a.object_id::text,
          'filename', a.filename, 'mediaType', a.media_type, 'objectRecord', a.object_payload)
          ORDER BY a.kind) INTO existing
      FROM public.student_export_artifact a WHERE a.export_id = request_id AND a.delivery_id IS NOT NULL;
    SELECT jsonb_agg(jsonb_build_object('kind', x->>'kind', 'object', x->>'object',
          'filename', x->>'filename', 'mediaType', x->>'mediaType', 'objectRecord', x->'objectRecord')
          ORDER BY x->>'kind') INTO supplied FROM jsonb_array_elements(p_artifacts) x;
    IF (SELECT state FROM public.student_export_request WHERE export_id = request_id) = 'ready' THEN
        IF existing = supplied THEN RETURN 'already_committed'; END IF;
        RETURN NULL;
    END IF;
    IF NOT EXISTS (SELECT 1 FROM public.worker_job w WHERE w.job_id = p_job AND w.tenant_id = p_tenant
        AND w.state = 'leased' AND w.lease_token = p_token AND w.lease_expires_at > transaction_timestamp()
        AND w.payload = jsonb_build_object('kind','export','deliveryObject',p_manifest::text)) THEN
        RETURN NULL;
    END IF;
    IF (SELECT count(*) FROM jsonb_array_elements(p_artifacts) x) <> 4
       OR EXISTS (SELECT 1 FROM public.student_export_artifact a
                   WHERE a.export_id = request_id AND NOT EXISTS (
                       SELECT 1 FROM jsonb_array_elements(p_artifacts) x
                        WHERE x->>'kind' = a.kind AND x->>'object' = a.object_id::text)) THEN
        RETURN NULL;
    END IF;
    UPDATE public.student_export_artifact a
       SET delivery_id = a.object_id, filename = x->>'filename', media_type = x->>'mediaType',
           object_payload = x->'objectRecord'
      FROM jsonb_array_elements(p_artifacts) x
     WHERE a.export_id = request_id AND x->>'kind' = a.kind AND x->>'object' = a.object_id::text;
    INSERT INTO public.asset_delivery (delivery_id, delivery_kind, tenant_id, object_id, payload, payload_sha256)
      SELECT a.object_id, 'student_record', p_tenant, a.object_id, x->'delivery',
             x->>'deliverySha256'
        FROM public.student_export_artifact a JOIN jsonb_array_elements(p_artifacts) x
          ON x->>'kind' = a.kind AND x->>'object' = a.object_id::text
       WHERE a.export_id = request_id;
    UPDATE public.student_export_request SET state = 'ready', ready_at = transaction_timestamp()
     WHERE export_id = request_id;
    UPDATE public.worker_job SET state = 'completed', lease_token = NULL, lease_expires_at = NULL,
           completed_at = transaction_timestamp()
     WHERE job_id = p_job;
    RETURN 'committed';
END $$;

CREATE FUNCTION public.ple_commit_prepared_qti_import(p_tenant uuid, p_job uuid, p_lease_token uuid, p_workspace uuid, p_import uuid, p_source_object uuid) RETURNS boolean
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'pg_catalog', 'public'
    AS $$
DECLARE
    committed boolean;
BEGIN
    IF p_tenant IS NULL OR p_job IS NULL OR p_lease_token IS NULL
       OR p_workspace IS NULL OR p_import IS NULL OR p_source_object IS NULL
       OR p_tenant <> public.ple_current_tenant() THEN
        RAISE EXCEPTION 'invalid QTI prepared-import commit capability' USING ERRCODE = '22023';
    END IF;
    WITH eligible AS (
        SELECT job_id
          FROM public.worker_job
         WHERE job_id = p_job
           AND tenant_id = p_tenant
           AND state = 'leased'
           AND lease_token = p_lease_token
           AND lease_expires_at > transaction_timestamp()
           AND payload = jsonb_build_object(
                'kind', 'qtiImport',
                'workspace', p_workspace::text,
                'import', p_import::text,
                'sourceObject', p_source_object::text)
         FOR UPDATE
    ), promoted AS (
        UPDATE public.workspace_qti_import AS registry
           SET state = 'committed'
          FROM eligible
         WHERE registry.tenant_id = p_tenant
           AND registry.workspace_id = p_workspace
           AND registry.import_id = p_import
           AND registry.source_object_id = p_source_object
           AND registry.state = 'prepared'
        RETURNING registry.import_id
    ), completed AS (
        UPDATE public.worker_job AS job
           SET state = 'completed',
               lease_token = NULL,
               lease_expires_at = NULL,
               completed_at = transaction_timestamp()
          FROM promoted
         WHERE job.job_id = p_job
           AND job.tenant_id = p_tenant
           AND job.state = 'leased'
           AND job.lease_token = p_lease_token
           AND job.lease_expires_at > transaction_timestamp()
        RETURNING 1
    )
    SELECT EXISTS(SELECT 1 FROM completed) INTO committed;
    RETURN committed;
END
$$;

CREATE FUNCTION public.ple_complete_worker_job(p_job_id uuid, p_token uuid) RETURNS boolean
    LANGUAGE sql SECURITY DEFINER
    SET search_path TO 'pg_catalog', 'public'
    AS $$
    WITH completed AS (
        UPDATE public.worker_job
           SET state = 'completed',
               lease_token = NULL,
               lease_expires_at = NULL,
               completed_at = transaction_timestamp()
         WHERE job_id = p_job_id
           AND state = 'leased'
           AND lease_token = p_token
           AND lease_expires_at > transaction_timestamp()
        RETURNING 1
    )
    SELECT EXISTS(SELECT 1 FROM completed)
$$;

CREATE FUNCTION public.ple_cancel_attempt_timing_job(p_tenant uuid, p_job uuid) RETURNS boolean
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'pg_catalog', 'public'
    AS $$
DECLARE
    changed boolean;
BEGIN
    IF p_tenant IS NULL OR p_job IS NULL
       OR p_tenant <> public.ple_current_tenant() THEN
        RAISE EXCEPTION 'invalid attempt timing job capability' USING ERRCODE = '22023';
    END IF;
    WITH completed AS (
        UPDATE public.worker_job AS job
           SET state = 'completed', lease_token = NULL, lease_expires_at = NULL,
               completed_at = transaction_timestamp()
         WHERE job.job_id = p_job AND job.tenant_id = p_tenant
           AND job.state IN ('ready', 'leased')
           AND job.payload ->> 'kind' = 'autoSubmitAttempt'
           AND EXISTS (
               SELECT 1 FROM public.attempt_timing_current AS timing
                WHERE timing.tenant_id = p_tenant AND timing.job_id = p_job
           )
        RETURNING 1
    )
    SELECT EXISTS(SELECT 1 FROM completed) INTO changed;
    RETURN changed;
END
$$;

CREATE FUNCTION public.ple_reschedule_attempt_timing_job(
    p_tenant uuid,
    p_job uuid,
    p_token uuid,
    p_payload jsonb,
    p_available_at timestamp with time zone
) RETURNS boolean
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'pg_catalog', 'public'
    AS $$
DECLARE
    changed boolean;
BEGIN
    IF p_tenant IS NULL OR p_job IS NULL OR p_payload IS NULL
       OR p_available_at IS NULL OR p_tenant <> public.ple_current_tenant()
       OR p_payload ->> 'kind' <> 'autoSubmitAttempt' THEN
        RAISE EXCEPTION 'invalid attempt timing reschedule capability'
            USING ERRCODE = '22023';
    END IF;
    WITH rescheduled AS (
        UPDATE public.worker_job AS job
           SET payload = p_payload, state = 'ready', available_at = p_available_at,
               lease_token = NULL, lease_expires_at = NULL, last_error = NULL,
               completed_at = NULL
          FROM public.attempt_timing_current AS timing
         WHERE job.job_id = p_job AND job.tenant_id = p_tenant
           AND timing.tenant_id = p_tenant AND timing.job_id = p_job
           AND timing.attempt_id = (p_payload ->> 'attempt')::uuid
           AND timing.timing_generation = (p_payload ->> 'timing_generation')::bigint
           AND (
               (p_token IS NULL AND job.state = 'ready')
               OR (p_token IS NOT NULL AND job.state = 'leased'
                   AND job.lease_token = p_token
                   AND job.lease_expires_at > transaction_timestamp())
           )
        RETURNING 1
    )
    SELECT EXISTS(SELECT 1 FROM rescheduled) INTO changed;
    RETURN changed;
END
$$;

CREATE FUNCTION public.ple_fail_worker_job(p_job_id uuid, p_token uuid, p_failure text) RETURNS text
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'pg_catalog', 'public'
    AS $$
DECLARE
    next_state text;
BEGIN
    IF p_failure NOT IN ('transient', 'permanent', 'timed_out') THEN
        RAISE EXCEPTION 'invalid queue failure kind' USING ERRCODE = '22023';
    END IF;

    UPDATE public.worker_job
       SET state = CASE
               WHEN p_failure = 'permanent' OR attempt_count >= max_attempts THEN 'dead'
               ELSE 'ready'
           END,
           available_at = CASE
               WHEN p_failure = 'permanent' OR attempt_count >= max_attempts
                   THEN available_at
               ELSE transaction_timestamp() + make_interval(
                   secs => (1 << LEAST(GREATEST(attempt_count - 1, 0), 8))
               )
           END,
           lease_token = NULL,
           lease_expires_at = NULL,
           last_error = p_failure,
           completed_at = CASE
               WHEN p_failure = 'permanent' OR attempt_count >= max_attempts
                   THEN transaction_timestamp()
               ELSE NULL
           END
     WHERE job_id = p_job_id
       AND state = 'leased'
       AND lease_token = p_token
       AND lease_expires_at > transaction_timestamp()
    RETURNING state INTO next_state;

    IF next_state IS NULL THEN
        RETURN NULL;
    END IF;
    RETURN CASE WHEN next_state = 'dead' THEN 'dead' ELSE 'retrying' END;
END
$$;

CREATE FUNCTION public.ple_mark_failed_export_job() RETURNS trigger
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'pg_catalog', 'public'
    AS $$
BEGIN
    IF NEW.state = 'dead' THEN
        UPDATE public.student_export_request SET state = 'failed'
         WHERE job_id = NEW.job_id AND state = 'queued';
    END IF;
    RETURN NEW;
END $$;

CREATE FUNCTION public.ple_mark_failed_assignment_scoring_job() RETURNS trigger
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'pg_catalog', 'public'
    AS $$
BEGIN
    IF NEW.state = 'dead' AND NEW.payload ->> 'kind' = 'recalculateAssignment' THEN
        UPDATE public.assignment
           SET scoring_status = 'failed', updated_at = transaction_timestamp()
         WHERE tenant_id = NEW.tenant_id
           AND assignment_id = (NEW.payload ->> 'assignment')::uuid
           AND scoring_generation = (NEW.payload ->> 'generation')::bigint
           AND scoring_status = 'recalculating';
    END IF;
    RETURN NEW;
END $$;

CREATE FUNCTION public.ple_promote_qti_grading(p_tenant uuid, p_workspace uuid, p_import uuid, p_problem uuid, p_version uuid, p_item_id text) RETURNS boolean
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'pg_catalog', 'public'
    AS $$
DECLARE
    copied_count bigint;
BEGIN
    IF p_tenant IS NULL OR p_workspace IS NULL OR p_import IS NULL
       OR p_problem IS NULL OR p_version IS NULL OR p_item_id IS NULL
       OR p_tenant <> public.ple_current_tenant() THEN
        RAISE EXCEPTION 'invalid QTI publication promotion capability'
            USING ERRCODE = '22023';
    END IF;

    -- Lock the committed registry and selected safe item before copying its
    -- hidden grading bytes. A stale or foreign staging reference simply does
    -- not promote anything, allowing the application transaction to roll back.
    PERFORM 1
      FROM public.workspace_qti_import AS registry
      JOIN public.workspace_qti_import_item AS item
        ON item.tenant_id = registry.tenant_id
       AND item.workspace_id = registry.workspace_id
       AND item.import_id = registry.import_id
     WHERE registry.tenant_id = p_tenant
       AND registry.workspace_id = p_workspace
       AND registry.import_id = p_import
       AND registry.state = 'committed'
       AND item.item_id = p_item_id
     FOR KEY SHARE OF registry, item;
    IF NOT FOUND THEN
        RETURN false;
    END IF;

    INSERT INTO public.published_qti_grading
        (problem_id, version_id, item_id, payload, payload_sha256)
    SELECT p_problem, p_version, grading.item_id, grading.payload,
           grading.payload_sha256
      FROM public.workspace_qti_import_grading AS grading
     WHERE grading.tenant_id = p_tenant
       AND grading.workspace_id = p_workspace
       AND grading.import_id = p_import
       AND grading.item_id = p_item_id
    ON CONFLICT (problem_id, version_id, item_id) DO NOTHING;
    GET DIAGNOSTICS copied_count = ROW_COUNT;
    RETURN copied_count = 1;
END
$$;

CREATE FUNCTION public.ple_question_statistics_view(p_problem uuid, p_version uuid) RETURNS TABLE(cohort_size bigint, difficulty_index double precision, attempts_mean double precision, time_median_seconds_estimate bigint, discrimination_index double precision)
    LANGUAGE sql STABLE SECURITY DEFINER
    SET search_path TO 'pg_catalog', 'public'
    AS $$
    SELECT aggregate.cohort_size,
           aggregate.score_sum / aggregate.cohort_size::double precision,
           aggregate.attempts_sum / aggregate.cohort_size::double precision,
           CASE
               WHEN aggregate.duration_histogram[1] >= (aggregate.cohort_size + 1) / 2 THEN 1
               WHEN aggregate.duration_histogram[1] + aggregate.duration_histogram[2] >= (aggregate.cohort_size + 1) / 2 THEN 5
               WHEN aggregate.duration_histogram[1] + aggregate.duration_histogram[2] + aggregate.duration_histogram[3] >= (aggregate.cohort_size + 1) / 2 THEN 15
               WHEN aggregate.duration_histogram[1] + aggregate.duration_histogram[2] + aggregate.duration_histogram[3] + aggregate.duration_histogram[4] >= (aggregate.cohort_size + 1) / 2 THEN 30
               WHEN aggregate.duration_histogram[1] + aggregate.duration_histogram[2] + aggregate.duration_histogram[3] + aggregate.duration_histogram[4] + aggregate.duration_histogram[5] >= (aggregate.cohort_size + 1) / 2 THEN 60
               WHEN aggregate.duration_histogram[1] + aggregate.duration_histogram[2] + aggregate.duration_histogram[3] + aggregate.duration_histogram[4] + aggregate.duration_histogram[5] + aggregate.duration_histogram[6] >= (aggregate.cohort_size + 1) / 2 THEN 120
               WHEN aggregate.duration_histogram[1] + aggregate.duration_histogram[2] + aggregate.duration_histogram[3] + aggregate.duration_histogram[4] + aggregate.duration_histogram[5] + aggregate.duration_histogram[6] + aggregate.duration_histogram[7] >= (aggregate.cohort_size + 1) / 2 THEN 300
               WHEN aggregate.duration_histogram[1] + aggregate.duration_histogram[2] + aggregate.duration_histogram[3] + aggregate.duration_histogram[4] + aggregate.duration_histogram[5] + aggregate.duration_histogram[6] + aggregate.duration_histogram[7] + aggregate.duration_histogram[8] >= (aggregate.cohort_size + 1) / 2 THEN 900
               WHEN aggregate.duration_histogram[1] + aggregate.duration_histogram[2] + aggregate.duration_histogram[3] + aggregate.duration_histogram[4] + aggregate.duration_histogram[5] + aggregate.duration_histogram[6] + aggregate.duration_histogram[7] + aggregate.duration_histogram[8] + aggregate.duration_histogram[9] >= (aggregate.cohort_size + 1) / 2 THEN 3600
               ELSE 86400
           END,
           CASE WHEN aggregate.scored_cohort_size < 5 OR aggregate.score_m2 <= 0 OR aggregate.rest_score_m2 <= 0 THEN NULL
               ELSE GREATEST(-1::double precision, LEAST(1::double precision,
                    aggregate.score_rest_co_moment / sqrt(aggregate.score_m2 * aggregate.rest_score_m2))) END
      FROM public.question_statistics_aggregate AS aggregate
      JOIN public.problem_version AS visible_version
        ON visible_version.problem_id = aggregate.problem_id
       AND visible_version.version_id = aggregate.version_id
     WHERE aggregate.problem_id = p_problem
       AND aggregate.version_id = p_version
       AND aggregate.cohort_size >= 5;
$$;

CREATE FUNCTION public.ple_ready_worker_queue_depth() RETURNS bigint
    LANGUAGE sql SECURITY DEFINER
    SET search_path TO 'pg_catalog', 'public'
    AS $$
    SELECT count(*)::bigint
      FROM public.worker_job
     WHERE state = 'ready'
       AND available_at <= transaction_timestamp()
$$;

CREATE FUNCTION public.ple_record_question_statistics(p_tenant uuid, p_enrollment uuid, p_first_completed_run uuid, p_attempt uuid, p_problem uuid, p_version uuid, p_score double precision, p_attempts bigint, p_duration_seconds bigint, p_rest_score double precision, p_observation_sha256 bytea) RETURNS boolean
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'pg_catalog', 'public'
    AS $$
DECLARE
    inserted boolean;
    duration_bins bigint[];
    stored_run uuid;
    stored_attempt uuid;
    stored_observation_sha256 bytea;
BEGIN
    IF p_tenant IS NULL OR p_enrollment IS NULL OR p_first_completed_run IS NULL
       OR p_attempt IS NULL OR p_problem IS NULL OR p_version IS NULL
       OR p_tenant <> public.ple_current_tenant()
       OR p_score IS NULL OR NOT public.ple_statistics_canonical_float(p_score)
       OR p_score < 0 OR p_score > 1 OR p_attempts IS NULL OR p_attempts < 1
       OR p_duration_seconds IS NULL OR p_duration_seconds < 0
       OR p_observation_sha256 IS NULL OR octet_length(p_observation_sha256) <> 32
       OR (p_rest_score IS NOT NULL AND (
            NOT public.ple_statistics_canonical_float(p_rest_score)
            OR p_rest_score < 0 OR p_rest_score > 1
       )) THEN
        RAISE EXCEPTION 'invalid statistics contribution' USING ERRCODE = '22023';
    END IF;
    IF NOT EXISTS (
        SELECT 1 FROM public.assignment_run AS run
         WHERE run.tenant_id = p_tenant AND run.run_id = p_first_completed_run
           AND run.enrollment_id = p_enrollment AND run.completed_at IS NOT NULL
           AND run.payload->>'mode' = 'assigned'
           AND NOT EXISTS (
               SELECT 1 FROM public.assignment_run AS earlier_run
                WHERE earlier_run.tenant_id = run.tenant_id
                  AND earlier_run.enrollment_id = run.enrollment_id
                  AND earlier_run.completed_at IS NOT NULL
                  AND earlier_run.run_number < run.run_number
           )
    ) OR NOT EXISTS (
        SELECT 1 FROM public.question_attempt AS attempt
         WHERE attempt.tenant_id = p_tenant AND attempt.attempt_id = p_attempt
           AND attempt.run_id = p_first_completed_run
    ) OR NOT EXISTS (
        SELECT 1 FROM public.question_attempt AS contributed_attempt
         WHERE contributed_attempt.tenant_id = p_tenant
           AND contributed_attempt.run_id = p_first_completed_run
           AND contributed_attempt.payload->>'problem' = p_problem::text
           AND contributed_attempt.payload->>'questionVersion' = p_version::text
    ) THEN
        RAISE EXCEPTION 'statistics receipt activity binding is invalid' USING ERRCODE = '22023';
    END IF;

    INSERT INTO public.question_statistics_contribution_receipt (
        tenant_id, enrollment_id, first_completed_run_id, attempt_id, observation_sha256, problem_id, version_id
    ) VALUES (p_tenant, p_enrollment, p_first_completed_run, p_attempt, p_observation_sha256, p_problem, p_version)
    ON CONFLICT DO NOTHING
    RETURNING true INTO inserted;
    IF NOT COALESCE(inserted, false) THEN
        SELECT first_completed_run_id, attempt_id, observation_sha256
          INTO stored_run, stored_attempt, stored_observation_sha256
          FROM public.question_statistics_contribution_receipt
         WHERE tenant_id = p_tenant AND enrollment_id = p_enrollment
           AND problem_id = p_problem AND version_id = p_version;
        IF stored_run = p_first_completed_run AND stored_attempt = p_attempt
           AND stored_observation_sha256 = p_observation_sha256 THEN
            RETURN false;
        END IF;
        RAISE EXCEPTION 'statistics contribution receipt conflicts' USING ERRCODE = '23505';
    END IF;

    duration_bins := ARRAY[
        CASE WHEN p_duration_seconds <= 1 THEN 1 ELSE 0 END,
        CASE WHEN p_duration_seconds BETWEEN 2 AND 5 THEN 1 ELSE 0 END,
        CASE WHEN p_duration_seconds BETWEEN 6 AND 15 THEN 1 ELSE 0 END,
        CASE WHEN p_duration_seconds BETWEEN 16 AND 30 THEN 1 ELSE 0 END,
        CASE WHEN p_duration_seconds BETWEEN 31 AND 60 THEN 1 ELSE 0 END,
        CASE WHEN p_duration_seconds BETWEEN 61 AND 120 THEN 1 ELSE 0 END,
        CASE WHEN p_duration_seconds BETWEEN 121 AND 300 THEN 1 ELSE 0 END,
        CASE WHEN p_duration_seconds BETWEEN 301 AND 900 THEN 1 ELSE 0 END,
        CASE WHEN p_duration_seconds BETWEEN 901 AND 3600 THEN 1 ELSE 0 END,
        CASE WHEN p_duration_seconds > 3600 THEN 1 ELSE 0 END
    ];

    INSERT INTO public.question_statistics_aggregate AS aggregate (
        problem_id, version_id, cohort_size, score_sum, attempts_sum,
        duration_histogram_version, duration_histogram, scored_cohort_size,
        score_mean, rest_score_mean, score_m2, rest_score_m2, score_rest_co_moment
    ) VALUES (
        p_problem, p_version, 1, p_score, p_attempts, 1, duration_bins,
        CASE WHEN p_rest_score IS NULL THEN 0 ELSE 1 END,
        CASE WHEN p_rest_score IS NULL THEN 0 ELSE p_score END,
        COALESCE(p_rest_score, 0), 0, 0, 0
    )
    ON CONFLICT (problem_id, version_id) DO UPDATE SET
        cohort_size = aggregate.cohort_size + 1,
        score_sum = aggregate.score_sum + EXCLUDED.score_sum,
        attempts_sum = aggregate.attempts_sum + EXCLUDED.attempts_sum,
        duration_histogram = ARRAY[
            aggregate.duration_histogram[1] + EXCLUDED.duration_histogram[1],
            aggregate.duration_histogram[2] + EXCLUDED.duration_histogram[2],
            aggregate.duration_histogram[3] + EXCLUDED.duration_histogram[3],
            aggregate.duration_histogram[4] + EXCLUDED.duration_histogram[4],
            aggregate.duration_histogram[5] + EXCLUDED.duration_histogram[5],
            aggregate.duration_histogram[6] + EXCLUDED.duration_histogram[6],
            aggregate.duration_histogram[7] + EXCLUDED.duration_histogram[7],
            aggregate.duration_histogram[8] + EXCLUDED.duration_histogram[8],
            aggregate.duration_histogram[9] + EXCLUDED.duration_histogram[9],
            aggregate.duration_histogram[10] + EXCLUDED.duration_histogram[10]
        ],
        score_mean = CASE WHEN aggregate.scored_cohort_size + EXCLUDED.scored_cohort_size = 0 THEN 0
            ELSE aggregate.score_mean + (EXCLUDED.score_mean - aggregate.score_mean)
                * EXCLUDED.scored_cohort_size::double precision
                / (aggregate.scored_cohort_size + EXCLUDED.scored_cohort_size)::double precision END,
        rest_score_mean = CASE WHEN aggregate.scored_cohort_size + EXCLUDED.scored_cohort_size = 0 THEN 0
            ELSE aggregate.rest_score_mean + (EXCLUDED.rest_score_mean - aggregate.rest_score_mean)
                * EXCLUDED.scored_cohort_size::double precision
                / (aggregate.scored_cohort_size + EXCLUDED.scored_cohort_size)::double precision END,
        score_m2 = CASE WHEN aggregate.scored_cohort_size + EXCLUDED.scored_cohort_size = 0 THEN 0
            ELSE aggregate.score_m2 + EXCLUDED.score_m2
                + (EXCLUDED.score_mean - aggregate.score_mean) ^ 2
                * aggregate.scored_cohort_size::double precision * EXCLUDED.scored_cohort_size::double precision
                / (aggregate.scored_cohort_size + EXCLUDED.scored_cohort_size)::double precision END,
        rest_score_m2 = CASE WHEN aggregate.scored_cohort_size + EXCLUDED.scored_cohort_size = 0 THEN 0
            ELSE aggregate.rest_score_m2 + EXCLUDED.rest_score_m2
                + (EXCLUDED.rest_score_mean - aggregate.rest_score_mean) ^ 2
                * aggregate.scored_cohort_size::double precision * EXCLUDED.scored_cohort_size::double precision
                / (aggregate.scored_cohort_size + EXCLUDED.scored_cohort_size)::double precision END,
        score_rest_co_moment = CASE WHEN aggregate.scored_cohort_size + EXCLUDED.scored_cohort_size = 0 THEN 0
            ELSE aggregate.score_rest_co_moment + EXCLUDED.score_rest_co_moment
                + (EXCLUDED.score_mean - aggregate.score_mean)
                * (EXCLUDED.rest_score_mean - aggregate.rest_score_mean)
                * aggregate.scored_cohort_size::double precision * EXCLUDED.scored_cohort_size::double precision
                / (aggregate.scored_cohort_size + EXCLUDED.scored_cohort_size)::double precision END,
        scored_cohort_size = aggregate.scored_cohort_size + EXCLUDED.scored_cohort_size;
    RETURN true;
END $$;

CREATE FUNCTION public.ple_statistics_aggregate_valid(p_cohort bigint, p_score_sum double precision, p_attempts bigint, p_histogram bigint[], p_scored bigint, p_score_mean double precision, p_rest_mean double precision, p_score_m2 double precision, p_rest_m2 double precision, p_co_moment double precision) RETURNS boolean
    LANGUAGE plpgsql IMMUTABLE STRICT
    SET search_path TO 'pg_catalog'
    AS $$
DECLARE
    p_histogram_total bigint;
    p_count double precision;
    p_tolerance double precision;
    p_max_score_m2 double precision;
    p_max_rest_m2 double precision;
    p_paired_score_sum double precision;
    p_unpaired bigint;
BEGIN
    IF p_cohort < 0 OR p_attempts < p_cohort OR p_scored < 0 OR p_scored > p_cohort
       OR array_length(p_histogram, 1) <> 10 OR array_position(p_histogram, NULL) IS NOT NULL
       OR NOT (0 <= ALL(p_histogram))
       OR NOT public.ple_statistics_canonical_float(p_score_sum)
       OR NOT public.ple_statistics_canonical_float(p_score_mean)
       OR NOT public.ple_statistics_canonical_float(p_rest_mean)
       OR NOT public.ple_statistics_canonical_float(p_score_m2)
       OR NOT public.ple_statistics_canonical_float(p_rest_m2)
       OR NOT public.ple_statistics_canonical_float(p_co_moment)
       OR p_score_sum < 0 OR p_score_sum > p_cohort::double precision
       OR p_score_mean < 0 OR p_score_mean > 1 OR p_rest_mean < 0 OR p_rest_mean > 1
       OR p_score_m2 < 0 OR p_rest_m2 < 0 THEN
        RETURN false;
    END IF;
    p_histogram_total := p_histogram[1] + p_histogram[2] + p_histogram[3] + p_histogram[4]
        + p_histogram[5] + p_histogram[6] + p_histogram[7] + p_histogram[8]
        + p_histogram[9] + p_histogram[10];
    IF p_histogram_total <> p_cohort THEN
        RETURN false;
    END IF;
    IF p_cohort = 0 THEN
        RETURN p_score_sum = 0 AND p_attempts = 0 AND p_scored = 0
            AND p_score_mean = 0 AND p_rest_mean = 0 AND p_score_m2 = 0
            AND p_rest_m2 = 0 AND p_co_moment = 0;
    END IF;
    IF p_scored = 0 THEN
        RETURN p_score_mean = 0 AND p_rest_mean = 0 AND p_score_m2 = 0
            AND p_rest_m2 = 0 AND p_co_moment = 0;
    END IF;
    IF p_scored = 1 AND (p_score_m2 <> 0 OR p_rest_m2 <> 0 OR p_co_moment <> 0) THEN
        RETURN false;
    END IF;
    p_count := p_scored::double precision;
    p_tolerance := 256 * 2.220446049250313e-16 * GREATEST(p_count, p_cohort::double precision, 1);
    p_max_score_m2 := p_count * p_score_mean * (1 - p_score_mean);
    p_max_rest_m2 := p_count * p_rest_mean * (1 - p_rest_mean);
    IF p_score_m2 > p_max_score_m2 + p_tolerance
       OR p_rest_m2 > p_max_rest_m2 + p_tolerance
       OR abs(p_co_moment) > sqrt(p_score_m2 * p_rest_m2) + p_tolerance THEN
        RETURN false;
    END IF;
    p_paired_score_sum := p_count * p_score_mean;
    p_unpaired := p_cohort - p_scored;
    RETURN p_score_sum + p_tolerance >= p_paired_score_sum
       AND p_score_sum <= p_paired_score_sum + p_unpaired::double precision + p_tolerance;
END $$;

CREATE FUNCTION public.ple_statistics_canonical_float(p_value double precision) RETURNS boolean
    LANGUAGE sql IMMUTABLE STRICT
    SET search_path TO 'pg_catalog'
    AS $$
    SELECT p_value::text NOT IN ('NaN', 'Infinity', '-Infinity')
       AND encode(float8send(p_value), 'hex') <> '8000000000000000'
$$;

ALTER FUNCTION public.ple_bind_student_record_course() OWNER TO ple_queue_broker;

ALTER FUNCTION public.ple_claim_worker_job(uuid, integer) OWNER TO ple_queue_broker;

ALTER FUNCTION public.ple_cancel_attempt_timing_job(uuid, uuid) OWNER TO ple_queue_broker;

ALTER FUNCTION public.ple_commit_export_job(uuid, uuid, uuid, uuid, jsonb) OWNER TO ple_queue_broker;

ALTER FUNCTION public.ple_commit_prepared_qti_import(uuid, uuid, uuid, uuid, uuid, uuid) OWNER TO ple_qti_staging_broker;

ALTER FUNCTION public.ple_complete_worker_job(uuid, uuid) OWNER TO ple_queue_broker;

ALTER FUNCTION public.ple_fail_worker_job(uuid, uuid, text) OWNER TO ple_queue_broker;

ALTER FUNCTION public.ple_mark_failed_export_job() OWNER TO ple_queue_broker;

ALTER FUNCTION public.ple_promote_qti_grading(uuid, uuid, uuid, uuid, uuid, text) OWNER TO ple_qti_staging_broker;

ALTER FUNCTION public.ple_question_statistics_view(uuid, uuid) OWNER TO ple_statistics_broker;

ALTER FUNCTION public.ple_ready_worker_queue_depth() OWNER TO ple_queue_broker;

ALTER FUNCTION public.ple_reschedule_attempt_timing_job(uuid, uuid, uuid, jsonb, timestamp with time zone) OWNER TO ple_queue_broker;

ALTER FUNCTION public.ple_record_question_statistics(uuid, uuid, uuid, uuid, uuid, uuid, double precision, bigint, bigint, double precision, bytea) OWNER TO ple_statistics_broker;

ALTER FUNCTION public.ple_statistics_aggregate_valid(bigint, double precision, bigint, bigint[], bigint, double precision, double precision, double precision, double precision, double precision) OWNER TO ple_statistics_broker;

ALTER FUNCTION public.ple_statistics_canonical_float(double precision) OWNER TO ple_statistics_broker;

CREATE TABLE public.asset_delivery (
    delivery_id uuid NOT NULL,
    delivery_kind text NOT NULL,
    tenant_id uuid,
    object_id uuid NOT NULL,
    problem_id uuid,
    version_id uuid,
    asset_id uuid,
    payload jsonb NOT NULL,
    payload_sha256 character(64) NOT NULL,
    created_at timestamp with time zone DEFAULT transaction_timestamp() NOT NULL,
    course_id uuid,
    CONSTRAINT asset_delivery_check CHECK ((((delivery_kind = 'catalog'::text) AND (tenant_id IS NULL) AND (problem_id IS NOT NULL) AND (version_id IS NOT NULL) AND (asset_id IS NOT NULL) AND (delivery_id = asset_id)) OR ((delivery_kind = 'student_record'::text) AND (tenant_id IS NOT NULL) AND (problem_id IS NULL) AND (version_id IS NULL) AND (asset_id IS NULL) AND (delivery_id = object_id)))),
    CONSTRAINT asset_delivery_course_shape CHECK ((((delivery_kind = 'catalog'::text) AND (course_id IS NULL)) OR ((delivery_kind = 'student_record'::text) AND (course_id IS NOT NULL)))),
    CONSTRAINT asset_delivery_delivery_kind_check CHECK ((delivery_kind = ANY (ARRAY['catalog'::text, 'student_record'::text])))
);

ALTER TABLE ONLY public.asset_delivery FORCE ROW LEVEL SECURITY;

CREATE TABLE public.question_statistics_aggregate (
    problem_id uuid NOT NULL,
    version_id uuid NOT NULL,
    cohort_size bigint NOT NULL,
    score_sum double precision NOT NULL,
    attempts_sum bigint NOT NULL,
    duration_histogram_version smallint NOT NULL,
    duration_histogram bigint[] NOT NULL,
    scored_cohort_size bigint NOT NULL,
    score_mean double precision NOT NULL,
    rest_score_mean double precision NOT NULL,
    score_m2 double precision NOT NULL,
    rest_score_m2 double precision NOT NULL,
    score_rest_co_moment double precision NOT NULL,
    CONSTRAINT question_statistics_aggregate_check CHECK (((score_sum >= (0)::double precision) AND (score_sum <= (cohort_size)::double precision))),
    CONSTRAINT question_statistics_aggregate_check1 CHECK ((attempts_sum >= cohort_size)),
    CONSTRAINT question_statistics_aggregate_check2 CHECK ((cohort_size = (((((((((duration_histogram[1] + duration_histogram[2]) + duration_histogram[3]) + duration_histogram[4]) + duration_histogram[5]) + duration_histogram[6]) + duration_histogram[7]) + duration_histogram[8]) + duration_histogram[9]) + duration_histogram[10]))),
    CONSTRAINT question_statistics_aggregate_check3 CHECK ((scored_cohort_size <= cohort_size)),
    CONSTRAINT question_statistics_aggregate_check4 CHECK (public.ple_statistics_aggregate_valid(cohort_size, score_sum, attempts_sum, duration_histogram, scored_cohort_size, score_mean, rest_score_mean, score_m2, rest_score_m2, score_rest_co_moment)),
    CONSTRAINT question_statistics_aggregate_cohort_size_check CHECK ((cohort_size >= 0)),
    CONSTRAINT question_statistics_aggregate_duration_histogram_check CHECK ((array_length(duration_histogram, 1) = 10)),
    CONSTRAINT question_statistics_aggregate_duration_histogram_check1 CHECK ((array_position(duration_histogram, NULL::bigint) IS NULL)),
    CONSTRAINT question_statistics_aggregate_duration_histogram_check2 CHECK ((0 <= ALL (duration_histogram))),
    CONSTRAINT question_statistics_aggregate_duration_histogram_version_check CHECK ((duration_histogram_version = 1)),
    CONSTRAINT question_statistics_aggregate_rest_score_m2_check CHECK (((rest_score_m2)::text <> ALL (ARRAY['NaN'::text, 'Infinity'::text, '-Infinity'::text]))),
    CONSTRAINT question_statistics_aggregate_rest_score_m2_check1 CHECK ((rest_score_m2 >= (0)::double precision)),
    CONSTRAINT question_statistics_aggregate_rest_score_mean_check CHECK (((rest_score_mean)::text <> ALL (ARRAY['NaN'::text, 'Infinity'::text, '-Infinity'::text]))),
    CONSTRAINT question_statistics_aggregate_rest_score_mean_check1 CHECK (((rest_score_mean >= (0)::double precision) AND (rest_score_mean <= (1)::double precision))),
    CONSTRAINT question_statistics_aggregate_score_m2_check CHECK (((score_m2)::text <> ALL (ARRAY['NaN'::text, 'Infinity'::text, '-Infinity'::text]))),
    CONSTRAINT question_statistics_aggregate_score_m2_check1 CHECK ((score_m2 >= (0)::double precision)),
    CONSTRAINT question_statistics_aggregate_score_mean_check CHECK (((score_mean)::text <> ALL (ARRAY['NaN'::text, 'Infinity'::text, '-Infinity'::text]))),
    CONSTRAINT question_statistics_aggregate_score_mean_check1 CHECK (((score_mean >= (0)::double precision) AND (score_mean <= (1)::double precision))),
    CONSTRAINT question_statistics_aggregate_score_rest_co_moment_check CHECK (((score_rest_co_moment)::text <> ALL (ARRAY['NaN'::text, 'Infinity'::text, '-Infinity'::text]))),
    CONSTRAINT question_statistics_aggregate_score_sum_check CHECK (((score_sum)::text <> ALL (ARRAY['NaN'::text, 'Infinity'::text, '-Infinity'::text]))),
    CONSTRAINT question_statistics_aggregate_scored_cohort_size_check CHECK ((scored_cohort_size >= 0))
);

CREATE TABLE public.question_statistics_contribution_receipt (
    tenant_id uuid NOT NULL,
    enrollment_id uuid NOT NULL,
    first_completed_run_id uuid NOT NULL,
    attempt_id uuid NOT NULL,
    observation_sha256 bytea NOT NULL,
    problem_id uuid NOT NULL,
    version_id uuid NOT NULL,
    created_at timestamp with time zone DEFAULT transaction_timestamp() NOT NULL,
    CONSTRAINT question_statistics_contribution_recei_observation_sha256_check CHECK ((octet_length(observation_sha256) = 32))
);

ALTER TABLE ONLY public.question_statistics_contribution_receipt FORCE ROW LEVEL SECURITY;

CREATE TABLE public.student_export_artifact (
    export_id uuid NOT NULL,
    kind text NOT NULL,
    object_id uuid NOT NULL,
    delivery_id uuid,
    filename text,
    media_type text,
    object_payload jsonb,
    CONSTRAINT student_export_artifact_check CHECK ((((delivery_id IS NULL) AND (filename IS NULL) AND (media_type IS NULL) AND (object_payload IS NULL)) OR ((delivery_id = object_id) AND (filename IS NOT NULL) AND (media_type IS NOT NULL) AND (object_payload IS NOT NULL)))),
    CONSTRAINT student_export_artifact_kind_check CHECK ((kind = ANY (ARRAY['docx'::text, 'pdf'::text, 'accessibleDocx'::text, 'accessiblePdf'::text])))
);

ALTER TABLE ONLY public.student_export_artifact FORCE ROW LEVEL SECURITY;

CREATE TABLE public.student_export_request (
    export_id uuid NOT NULL,
    tenant_id uuid NOT NULL,
    course_id uuid NOT NULL,
    assignment_id uuid NOT NULL,
    requester_id uuid NOT NULL,
    job_id uuid NOT NULL,
    manifest_object_id uuid NOT NULL,
    frozen_payload jsonb NOT NULL,
    frozen_payload_sha256 bytea NOT NULL,
    state text NOT NULL,
    created_at timestamp with time zone DEFAULT transaction_timestamp() NOT NULL,
    ready_at timestamp with time zone,
    CONSTRAINT student_export_request_check CHECK ((((state = 'ready'::text) AND (ready_at IS NOT NULL)) OR ((state <> 'ready'::text) AND (ready_at IS NULL)))),
    CONSTRAINT student_export_request_frozen_payload_check CHECK ((jsonb_typeof(frozen_payload) = 'object'::text)),
    CONSTRAINT student_export_request_frozen_payload_sha256_check CHECK ((octet_length(frozen_payload_sha256) = 32)),
    CONSTRAINT student_export_request_state_check CHECK ((state = ANY (ARRAY['queued'::text, 'ready'::text, 'failed'::text])))
);

ALTER TABLE ONLY public.student_export_request FORCE ROW LEVEL SECURITY;

CREATE TABLE public.worker_job (
    job_id uuid NOT NULL,
    tenant_id uuid NOT NULL,
    payload jsonb NOT NULL,
    state text NOT NULL,
    available_at timestamp with time zone DEFAULT transaction_timestamp() NOT NULL,
    lease_token uuid,
    lease_expires_at timestamp with time zone,
    attempt_count integer DEFAULT 0 NOT NULL,
    max_attempts integer NOT NULL,
    last_error text,
    created_at timestamp with time zone DEFAULT transaction_timestamp() NOT NULL,
    completed_at timestamp with time zone,
    CONSTRAINT worker_job_attempt_count_check CHECK ((attempt_count >= 0)),
    CONSTRAINT worker_job_check CHECK ((attempt_count <= max_attempts)),
    CONSTRAINT worker_job_check1 CHECK ((((state = 'leased'::text) AND (lease_token IS NOT NULL) AND (lease_expires_at IS NOT NULL) AND (completed_at IS NULL)) OR ((state = 'ready'::text) AND (lease_token IS NULL) AND (lease_expires_at IS NULL) AND (completed_at IS NULL)) OR ((state = 'completed'::text) AND (lease_token IS NULL) AND (lease_expires_at IS NULL) AND (completed_at IS NOT NULL)) OR ((state = 'dead'::text) AND (lease_token IS NULL) AND (lease_expires_at IS NULL) AND (completed_at IS NOT NULL) AND (last_error IS NOT NULL)))),
    CONSTRAINT worker_job_last_error_check CHECK (((last_error IS NULL) OR (last_error = ANY (ARRAY['transient'::text, 'permanent'::text, 'timed_out'::text])))),
    CONSTRAINT worker_job_max_attempts_check CHECK (((max_attempts >= 1) AND (max_attempts <= 20))),
    CONSTRAINT worker_job_payload_kind_check CHECK (
        CASE payload ->> 'kind'
            WHEN 'render' THEN
                payload ?& ARRAY['kind', 'reference', 'seed']
                AND payload - ARRAY['kind', 'reference', 'seed'] = '{}'::jsonb
                AND jsonb_typeof(payload -> 'reference') = 'object'
                AND (payload -> 'reference') ?& ARRAY['problem', 'version']
                AND (payload -> 'reference') - ARRAY['problem', 'version'] = '{}'::jsonb
                AND (payload #>> '{reference,problem}') ~* '^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$'
                AND (payload #>> '{reference,version}') ~* '^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$'
                AND jsonb_typeof(payload -> 'seed') = 'number'
                AND (payload ->> 'seed') ~ '^(0|[1-9][0-9]{0,19})$'
                AND (payload ->> 'seed')::numeric <= 18446744073709551615
            WHEN 'export' THEN
                payload ?& ARRAY['kind', 'deliveryObject']
                AND payload - ARRAY['kind', 'deliveryObject'] = '{}'::jsonb
                AND (payload ->> 'deliveryObject') ~* '^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$'
            WHEN 'import' THEN
                payload ?& ARRAY['kind', 'sourceObject']
                AND payload - ARRAY['kind', 'sourceObject'] = '{}'::jsonb
                AND (payload ->> 'sourceObject') ~* '^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$'
            WHEN 'qtiImport' THEN
                payload ?& ARRAY['kind', 'workspace', 'import', 'sourceObject']
                AND payload - ARRAY['kind', 'workspace', 'import', 'sourceObject'] = '{}'::jsonb
                AND (payload ->> 'workspace') ~* '^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$'
                AND (payload ->> 'import') ~* '^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$'
                AND (payload ->> 'sourceObject') ~* '^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$'
            WHEN 'retention' THEN
                payload ?& ARRAY['kind', 'course', 'stage', 'generation']
                AND payload - ARRAY['kind', 'course', 'stage', 'generation'] = '{}'::jsonb
                AND (payload ->> 'course') ~* '^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$'
                AND (payload ->> 'stage') = ANY (ARRAY['notify', 'archiveStudentRecords', 'deleteStudentRecords'])
                AND jsonb_typeof(payload -> 'generation') = 'number'
                AND (payload ->> 'generation') ~ '^[1-9][0-9]{0,18}$'
                AND (payload ->> 'generation')::numeric <= 9223372036854775807
            WHEN 'recalculateAssignment' THEN
                payload ?& ARRAY['kind', 'assignment', 'generation']
                AND payload - ARRAY['kind', 'assignment', 'generation'] = '{}'::jsonb
                AND (payload ->> 'assignment') ~* '^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$'
                AND jsonb_typeof(payload -> 'generation') = 'number'
                AND (payload ->> 'generation') ~ '^[1-9][0-9]{0,18}$'
                AND (payload ->> 'generation')::numeric <= 9223372036854775807
            WHEN 'autoSubmitAttempt' THEN
                payload ?& ARRAY['kind', 'attempt', 'timing_generation']
                AND payload - ARRAY['kind', 'attempt', 'timing_generation'] = '{}'::jsonb
                AND (payload ->> 'attempt') ~* '^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$'
                AND jsonb_typeof(payload -> 'timing_generation') = 'number'
                AND (payload ->> 'timing_generation') ~ '^[1-9][0-9]{0,18}$'
                AND (payload ->> 'timing_generation')::numeric <= 9223372036854775807
            ELSE false
        END
    ),
    CONSTRAINT worker_job_payload_object_check CHECK ((jsonb_typeof(payload) = 'object'::text)),
    CONSTRAINT worker_job_state_check CHECK ((state = ANY (ARRAY['ready'::text, 'leased'::text, 'completed'::text, 'dead'::text])))
);

ALTER TABLE ONLY public.worker_job FORCE ROW LEVEL SECURITY;

CREATE TABLE public.attempt_timing_current (
    tenant_id uuid NOT NULL,
    attempt_id uuid NOT NULL,
    attempt_occurred_at timestamp with time zone NOT NULL,
    assignment_id uuid NOT NULL,
    course_id uuid NOT NULL,
    authored_deadline timestamp with time zone,
    authored_grace_seconds integer NOT NULL,
    effective_deadline timestamp with time zone,
    effective_grace_seconds integer NOT NULL,
    auto_submit_at timestamp with time zone,
    resolution_kind text NOT NULL,
    timing_generation bigint NOT NULL,
    job_id uuid,
    updated_at timestamp with time zone DEFAULT transaction_timestamp() NOT NULL,
    CONSTRAINT attempt_timing_current_authored_grace_check CHECK (authored_grace_seconds >= 0),
    CONSTRAINT attempt_timing_current_effective_grace_check CHECK (effective_grace_seconds >= 0),
    CONSTRAINT attempt_timing_current_generation_check CHECK (timing_generation > 0),
    CONSTRAINT attempt_timing_current_resolution_check CHECK (
        resolution_kind = ANY (ARRAY['untimed'::text, 'authored_question'::text,
            'assignment_time_limit'::text, 'due_at'::text, 'closes_at'::text])
    ),
    CONSTRAINT attempt_timing_current_shape_check CHECK (
        (effective_deadline IS NULL AND effective_grace_seconds = 0
            AND auto_submit_at IS NULL AND resolution_kind = 'untimed')
        OR
        (effective_deadline IS NOT NULL AND auto_submit_at IS NOT NULL
            AND resolution_kind <> 'untimed'
            AND auto_submit_at = effective_deadline
                + make_interval(secs => effective_grace_seconds))
    )
);

ALTER TABLE ONLY public.attempt_timing_current FORCE ROW LEVEL SECURITY;

CREATE TABLE public.assignment_scoring_staging (
    tenant_id uuid NOT NULL,
    job_id uuid NOT NULL,
    assignment_id uuid NOT NULL,
    scoring_generation bigint NOT NULL,
    attempt_count bigint NOT NULL,
    enrollment_count bigint NOT NULL,
    prepared_at timestamp with time zone DEFAULT transaction_timestamp() NOT NULL,
    CONSTRAINT assignment_scoring_staging_generation_check CHECK ((scoring_generation > 0)),
    CONSTRAINT assignment_scoring_staging_counts_check CHECK ((attempt_count >= 0 AND enrollment_count >= 0)),
    PRIMARY KEY (tenant_id, job_id)
);

ALTER TABLE ONLY public.assignment_scoring_staging FORCE ROW LEVEL SECURITY;

CREATE TABLE public.assignment_attempt_score_staging (
    tenant_id uuid NOT NULL,
    job_id uuid NOT NULL,
    assignment_id uuid NOT NULL,
    scoring_generation bigint NOT NULL,
    attempt_id uuid NOT NULL,
    assignment_item_id uuid NOT NULL,
    earned_points numeric(20,4) NOT NULL,
    possible_points numeric(20,4) NOT NULL,
    course_id uuid NOT NULL,
    CONSTRAINT assignment_attempt_score_staging_generation_check CHECK ((scoring_generation > 0)),
    CONSTRAINT assignment_attempt_score_staging_possible_check CHECK ((possible_points >= 0)),
    PRIMARY KEY (tenant_id, job_id, attempt_id)
);

ALTER TABLE ONLY public.assignment_attempt_score_staging FORCE ROW LEVEL SECURITY;

CREATE TABLE public.assignment_summary_staging (
    tenant_id uuid NOT NULL,
    job_id uuid NOT NULL,
    assignment_id uuid NOT NULL,
    scoring_generation bigint NOT NULL,
    enrollment_id uuid NOT NULL,
    summary_payload jsonb NOT NULL,
    summary_payload_sha256 character(64) NOT NULL,
    enrollment_payload jsonb NOT NULL,
    enrollment_payload_sha256 character(64) NOT NULL,
    CONSTRAINT assignment_summary_staging_generation_check CHECK ((scoring_generation > 0)),
    CONSTRAINT assignment_summary_staging_payload_check CHECK ((jsonb_typeof(summary_payload) = 'object'::text AND jsonb_typeof(enrollment_payload) = 'object'::text)),
    PRIMARY KEY (tenant_id, job_id, enrollment_id)
);

ALTER TABLE ONLY public.assignment_summary_staging FORCE ROW LEVEL SECURITY;

ALTER TABLE ONLY public.asset_delivery
    ADD CONSTRAINT asset_delivery_asset_id_key UNIQUE (asset_id);

ALTER TABLE ONLY public.asset_delivery
    ADD CONSTRAINT asset_delivery_object_id_key UNIQUE (object_id);

ALTER TABLE ONLY public.asset_delivery
    ADD CONSTRAINT asset_delivery_pkey PRIMARY KEY (delivery_id);

ALTER TABLE ONLY public.question_statistics_aggregate
    ADD CONSTRAINT question_statistics_aggregate_pkey PRIMARY KEY (problem_id, version_id);

ALTER TABLE ONLY public.question_statistics_contribution_receipt
    ADD CONSTRAINT question_statistics_contribution_receipt_pkey PRIMARY KEY (tenant_id, enrollment_id, problem_id, version_id);

ALTER TABLE ONLY public.student_export_artifact
    ADD CONSTRAINT student_export_artifact_object_id_key UNIQUE (object_id);

ALTER TABLE ONLY public.student_export_artifact
    ADD CONSTRAINT student_export_artifact_pkey PRIMARY KEY (export_id, kind);

ALTER TABLE ONLY public.student_export_request
    ADD CONSTRAINT student_export_request_job_id_key UNIQUE (job_id);

ALTER TABLE ONLY public.student_export_request
    ADD CONSTRAINT student_export_request_manifest_object_id_key UNIQUE (manifest_object_id);

ALTER TABLE ONLY public.student_export_request
    ADD CONSTRAINT student_export_request_pkey PRIMARY KEY (export_id);

ALTER TABLE ONLY public.worker_job
    ADD CONSTRAINT worker_job_pkey PRIMARY KEY (job_id);

ALTER TABLE ONLY public.attempt_timing_current
    ADD CONSTRAINT attempt_timing_current_pkey PRIMARY KEY (tenant_id, attempt_id);

ALTER TABLE ONLY public.attempt_timing_current
    ADD CONSTRAINT attempt_timing_current_job_key UNIQUE (job_id);

CREATE INDEX asset_delivery_catalog_version_idx ON public.asset_delivery USING btree (problem_id, version_id) WHERE (delivery_kind = 'catalog'::text);

CREATE INDEX asset_delivery_course_idx ON public.asset_delivery USING btree (tenant_id, course_id, delivery_id) WHERE (delivery_kind = 'student_record'::text);

CREATE INDEX asset_delivery_tenant_idx ON public.asset_delivery USING btree (tenant_id, delivery_id) WHERE (delivery_kind = 'student_record'::text);

CREATE INDEX question_statistics_receipt_attempt_idx ON public.question_statistics_contribution_receipt USING btree (tenant_id, attempt_id);

CREATE INDEX question_statistics_receipt_run_idx ON public.question_statistics_contribution_receipt USING btree (tenant_id, first_completed_run_id);

CREATE INDEX student_export_request_course_idx ON public.student_export_request USING btree (tenant_id, course_id, export_id, job_id);

CREATE UNIQUE INDEX student_export_request_tenant_manifest_idx ON public.student_export_request USING btree (tenant_id, manifest_object_id);

CREATE INDEX worker_job_claim_ready_idx ON public.worker_job USING btree (available_at, job_id) WHERE (state = 'ready'::text);

CREATE INDEX worker_job_expired_lease_idx ON public.worker_job USING btree (lease_expires_at, job_id) WHERE (state = 'leased'::text);

CREATE INDEX worker_job_tenant_lookup_idx ON public.worker_job USING btree (tenant_id, job_id);

CREATE INDEX attempt_timing_current_assignment_idx ON public.attempt_timing_current
    (tenant_id, assignment_id, attempt_id);

CREATE INDEX attempt_timing_current_course_idx ON public.attempt_timing_current
    (tenant_id, course_id, attempt_id);

CREATE UNIQUE INDEX worker_job_assignment_scoring_generation_idx ON public.worker_job
    (tenant_id, ((payload ->> 'assignment')::uuid), ((payload ->> 'generation')::bigint))
    WHERE (payload ->> 'kind' = 'recalculateAssignment');

CREATE INDEX assignment_attempt_score_staging_assignment_idx ON public.assignment_attempt_score_staging
    (tenant_id, assignment_id, scoring_generation, job_id, attempt_id);

CREATE INDEX assignment_summary_staging_assignment_idx ON public.assignment_summary_staging
    (tenant_id, assignment_id, scoring_generation, job_id, enrollment_id);

CREATE INDEX assignment_scoring_staging_assignment_idx ON public.assignment_scoring_staging
    (tenant_id, assignment_id, scoring_generation, job_id);

CREATE TRIGGER asset_delivery_bind_student_course BEFORE INSERT ON public.asset_delivery FOR EACH ROW EXECUTE FUNCTION public.ple_bind_student_record_course();

CREATE TRIGGER worker_job_export_failure AFTER UPDATE OF state ON public.worker_job FOR EACH ROW EXECUTE FUNCTION public.ple_mark_failed_export_job();

CREATE TRIGGER worker_job_assignment_scoring_failure AFTER UPDATE OF state ON public.worker_job FOR EACH ROW EXECUTE FUNCTION public.ple_mark_failed_assignment_scoring_job();

ALTER TABLE ONLY public.asset_delivery
    ADD CONSTRAINT asset_delivery_course_fk FOREIGN KEY (tenant_id, course_id) REFERENCES public.course(tenant_id, course_id);

ALTER TABLE ONLY public.asset_delivery
    ADD CONSTRAINT asset_delivery_problem_id_version_id_fkey FOREIGN KEY (problem_id, version_id) REFERENCES public.problem_version(problem_id, version_id);

ALTER TABLE ONLY public.question_statistics_aggregate
    ADD CONSTRAINT question_statistics_aggregate_problem_id_version_id_fkey FOREIGN KEY (problem_id, version_id) REFERENCES public.problem_version(problem_id, version_id) ON DELETE RESTRICT;

ALTER TABLE ONLY public.question_statistics_contribution_receipt
    ADD CONSTRAINT question_statistics_contribut_tenant_id_first_completed_ru_fkey FOREIGN KEY (tenant_id, first_completed_run_id) REFERENCES public.assignment_run(tenant_id, run_id) ON DELETE CASCADE;

ALTER TABLE ONLY public.question_statistics_contribution_receipt
    ADD CONSTRAINT question_statistics_contribution_r_tenant_id_enrollment_id_fkey FOREIGN KEY (tenant_id, enrollment_id) REFERENCES public.enrollment(tenant_id, enrollment_id) ON DELETE CASCADE;

ALTER TABLE ONLY public.question_statistics_contribution_receipt
    ADD CONSTRAINT question_statistics_contribution_rec_problem_id_version_id_fkey FOREIGN KEY (problem_id, version_id) REFERENCES public.problem_version(problem_id, version_id) ON DELETE RESTRICT;

ALTER TABLE ONLY public.question_statistics_contribution_receipt
    ADD CONSTRAINT question_statistics_contribution_rece_tenant_id_attempt_id_fkey FOREIGN KEY (tenant_id, attempt_id) REFERENCES public.submission_idempotency(tenant_id, attempt_id) ON DELETE CASCADE;

ALTER TABLE ONLY public.student_export_artifact
    ADD CONSTRAINT student_export_artifact_export_id_fkey FOREIGN KEY (export_id) REFERENCES public.student_export_request(export_id);

ALTER TABLE ONLY public.student_export_request
    ADD CONSTRAINT student_export_request_course_fk FOREIGN KEY (tenant_id, course_id) REFERENCES public.course(tenant_id, course_id);

ALTER TABLE ONLY public.student_export_request
    ADD CONSTRAINT student_export_request_job_id_fkey FOREIGN KEY (job_id) REFERENCES public.worker_job(job_id);

ALTER TABLE ONLY public.attempt_timing_current
    ADD CONSTRAINT attempt_timing_current_attempt_fk
    FOREIGN KEY (tenant_id, attempt_id, attempt_occurred_at)
    REFERENCES public.question_attempt(tenant_id, attempt_id, occurred_at) ON DELETE CASCADE;

ALTER TABLE ONLY public.attempt_timing_current
    ADD CONSTRAINT attempt_timing_current_assignment_fk
    FOREIGN KEY (tenant_id, assignment_id)
    REFERENCES public.assignment(tenant_id, assignment_id) ON DELETE CASCADE;

ALTER TABLE ONLY public.attempt_timing_current
    ADD CONSTRAINT attempt_timing_current_course_fk
    FOREIGN KEY (tenant_id, course_id)
    REFERENCES public.course(tenant_id, course_id);

ALTER TABLE ONLY public.attempt_timing_current
    ADD CONSTRAINT attempt_timing_current_job_fk
    FOREIGN KEY (job_id) REFERENCES public.worker_job(job_id) ON DELETE SET NULL;

ALTER TABLE ONLY public.assignment_attempt_score_staging
    ADD CONSTRAINT assignment_attempt_score_staging_job_fk FOREIGN KEY (job_id) REFERENCES public.worker_job(job_id) ON DELETE CASCADE;

ALTER TABLE ONLY public.assignment_attempt_score_staging
    ADD CONSTRAINT assignment_attempt_score_staging_assignment_fk FOREIGN KEY (tenant_id, assignment_id) REFERENCES public.assignment(tenant_id, assignment_id) ON DELETE CASCADE;

ALTER TABLE ONLY public.assignment_attempt_score_staging
    ADD CONSTRAINT assignment_attempt_score_staging_course_fk FOREIGN KEY (tenant_id, course_id) REFERENCES public.course(tenant_id, course_id);

ALTER TABLE ONLY public.assignment_scoring_staging
    ADD CONSTRAINT assignment_scoring_staging_job_fk FOREIGN KEY (job_id) REFERENCES public.worker_job(job_id) ON DELETE CASCADE;

ALTER TABLE ONLY public.assignment_scoring_staging
    ADD CONSTRAINT assignment_scoring_staging_assignment_fk FOREIGN KEY (tenant_id, assignment_id) REFERENCES public.assignment(tenant_id, assignment_id) ON DELETE CASCADE;

ALTER TABLE ONLY public.assignment_summary_staging
    ADD CONSTRAINT assignment_summary_staging_job_fk FOREIGN KEY (job_id) REFERENCES public.worker_job(job_id) ON DELETE CASCADE;

ALTER TABLE ONLY public.assignment_summary_staging
    ADD CONSTRAINT assignment_summary_staging_assignment_fk FOREIGN KEY (tenant_id, assignment_id) REFERENCES public.assignment(tenant_id, assignment_id) ON DELETE CASCADE;

ALTER TABLE ONLY public.assignment_summary_staging
    ADD CONSTRAINT assignment_summary_staging_enrollment_fk FOREIGN KEY (tenant_id, enrollment_id) REFERENCES public.enrollment(tenant_id, enrollment_id) ON DELETE CASCADE;

ALTER TABLE public.asset_delivery ENABLE ROW LEVEL SECURITY;

ALTER TABLE public.question_statistics_contribution_receipt ENABLE ROW LEVEL SECURITY;

CREATE POLICY question_statistics_contribution_receipt_broker ON public.question_statistics_contribution_receipt TO ple_statistics_broker USING ((tenant_id = public.ple_current_tenant())) WITH CHECK ((tenant_id = public.ple_current_tenant()));

CREATE POLICY question_statistics_contribution_receipt_tenant ON public.question_statistics_contribution_receipt USING ((tenant_id = public.ple_current_tenant())) WITH CHECK ((tenant_id = public.ple_current_tenant()));

CREATE POLICY ret_broker_stats_receipt_del ON public.question_statistics_contribution_receipt FOR DELETE TO ple_retention_broker USING ((tenant_id = public.ple_current_tenant()));

CREATE POLICY ret_broker_stats_receipt_sel ON public.question_statistics_contribution_receipt FOR SELECT TO ple_retention_broker USING ((tenant_id = public.ple_current_tenant()));

ALTER TABLE public.student_export_artifact ENABLE ROW LEVEL SECURITY;

ALTER TABLE public.student_export_request ENABLE ROW LEVEL SECURITY;

ALTER TABLE public.worker_job ENABLE ROW LEVEL SECURITY;

ALTER TABLE public.attempt_timing_current ENABLE ROW LEVEL SECURITY;

ALTER TABLE public.assignment_attempt_score_staging ENABLE ROW LEVEL SECURITY;

ALTER TABLE public.assignment_summary_staging ENABLE ROW LEVEL SECURITY;

ALTER TABLE public.assignment_scoring_staging ENABLE ROW LEVEL SECURITY;

CREATE POLICY worker_job_tenant_insert ON public.worker_job FOR INSERT TO ple_app WITH CHECK (((tenant_id = public.ple_current_tenant()) AND (state = 'ready'::text)));

CREATE POLICY worker_job_tenant_select ON public.worker_job FOR SELECT TO ple_app USING ((tenant_id = public.ple_current_tenant()));

CREATE POLICY attempt_timing_current_tenant ON public.attempt_timing_current TO ple_app
    USING ((tenant_id = public.ple_current_tenant()))
    WITH CHECK ((tenant_id = public.ple_current_tenant()));

CREATE POLICY ret_broker_attempt_timing_del ON public.attempt_timing_current FOR DELETE
    TO ple_retention_broker USING ((tenant_id = public.ple_current_tenant()));

CREATE POLICY ret_broker_attempt_timing_sel ON public.attempt_timing_current FOR SELECT
    TO ple_retention_broker USING ((tenant_id = public.ple_current_tenant()));

CREATE POLICY assignment_attempt_score_staging_tenant ON public.assignment_attempt_score_staging TO ple_app USING ((tenant_id = public.ple_current_tenant())) WITH CHECK ((tenant_id = public.ple_current_tenant()));

CREATE POLICY assignment_summary_staging_tenant ON public.assignment_summary_staging TO ple_app USING ((tenant_id = public.ple_current_tenant())) WITH CHECK ((tenant_id = public.ple_current_tenant()));

CREATE POLICY assignment_scoring_staging_tenant ON public.assignment_scoring_staging TO ple_app USING ((tenant_id = public.ple_current_tenant())) WITH CHECK ((tenant_id = public.ple_current_tenant()));

-- Stable catalog read boundary. The only optional relationship is the
-- disclosure-gated statistics projection; callers no longer rebuild this
-- join or risk bypassing its tenant-aware reader.
CREATE VIEW public.catalog_search_view
WITH (security_invoker = true)
AS
SELECT document.*,
       statistics.cohort_size,
       statistics.difficulty_index,
       statistics.attempts_mean,
       statistics.time_median_seconds_estimate,
       statistics.discrimination_index,
       (statistics.cohort_size IS NOT NULL) AS statistics_available
  FROM public.catalog_search_document AS document
  LEFT JOIN LATERAL public.ple_question_statistics_view(
      document.problem_id,
      document.version_id
  ) AS statistics ON true;

GRANT SELECT,DELETE ON TABLE public.asset_delivery TO ple_retention_broker;
GRANT INSERT ON TABLE public.asset_delivery TO ple_queue_broker;

GRANT SELECT,INSERT,UPDATE ON TABLE public.question_statistics_aggregate TO ple_statistics_broker;

GRANT SELECT,INSERT ON TABLE public.question_statistics_contribution_receipt TO ple_statistics_broker;
GRANT SELECT,DELETE ON TABLE public.question_statistics_contribution_receipt TO ple_retention_broker;

GRANT SELECT,DELETE ON TABLE public.student_export_artifact TO ple_retention_broker;
GRANT SELECT,UPDATE ON TABLE public.student_export_artifact TO ple_queue_broker;

GRANT SELECT,DELETE,UPDATE ON TABLE public.student_export_request TO ple_retention_broker;
GRANT SELECT,UPDATE ON TABLE public.student_export_request TO ple_queue_broker;

GRANT SELECT,INSERT ON TABLE public.worker_job TO ple_app;
GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.attempt_timing_current TO ple_app;
GRANT SELECT ON TABLE public.attempt_timing_current TO ple_queue_broker;
GRANT SELECT,DELETE ON TABLE public.attempt_timing_current TO ple_retention_broker;
GRANT SELECT,INSERT,DELETE ON TABLE public.assignment_attempt_score_staging TO ple_app;
GRANT SELECT,INSERT,DELETE ON TABLE public.assignment_summary_staging TO ple_app;
GRANT SELECT,INSERT,DELETE ON TABLE public.assignment_scoring_staging TO ple_app;
GRANT SELECT,UPDATE ON TABLE public.worker_job TO ple_queue_broker;
GRANT SELECT,UPDATE ON TABLE public.worker_job TO ple_qti_staging_broker;
GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.worker_job TO ple_retention_broker;

GRANT ALL ON FUNCTION public.ple_current_tenant() TO ple_queue_broker;

GRANT SELECT ON TABLE public.catalog_search_view TO ple_app, ple_student;

REVOKE ALL ON FUNCTION public.ple_bind_student_record_course() FROM PUBLIC;

REVOKE ALL ON FUNCTION public.ple_claim_worker_job(p_token uuid, p_lease_seconds integer) FROM PUBLIC;
GRANT ALL ON FUNCTION public.ple_claim_worker_job(p_token uuid, p_lease_seconds integer) TO ple_app;

REVOKE ALL ON FUNCTION public.ple_cancel_attempt_timing_job(p_tenant uuid, p_job uuid) FROM PUBLIC;
GRANT ALL ON FUNCTION public.ple_cancel_attempt_timing_job(p_tenant uuid, p_job uuid) TO ple_app;

REVOKE ALL ON FUNCTION public.ple_commit_export_job(p_tenant uuid, p_job uuid, p_token uuid, p_manifest uuid, p_artifacts jsonb) FROM PUBLIC;
GRANT ALL ON FUNCTION public.ple_commit_export_job(p_tenant uuid, p_job uuid, p_token uuid, p_manifest uuid, p_artifacts jsonb) TO ple_app;

REVOKE ALL ON FUNCTION public.ple_commit_prepared_qti_import(p_tenant uuid, p_job uuid, p_lease_token uuid, p_workspace uuid, p_import uuid, p_source_object uuid) FROM PUBLIC;
GRANT ALL ON FUNCTION public.ple_commit_prepared_qti_import(p_tenant uuid, p_job uuid, p_lease_token uuid, p_workspace uuid, p_import uuid, p_source_object uuid) TO ple_app;

REVOKE ALL ON FUNCTION public.ple_complete_worker_job(p_job_id uuid, p_token uuid) FROM PUBLIC;
GRANT ALL ON FUNCTION public.ple_complete_worker_job(p_job_id uuid, p_token uuid) TO ple_app;

REVOKE ALL ON FUNCTION public.ple_fail_worker_job(p_job_id uuid, p_token uuid, p_failure text) FROM PUBLIC;
GRANT ALL ON FUNCTION public.ple_fail_worker_job(p_job_id uuid, p_token uuid, p_failure text) TO ple_app;

REVOKE ALL ON FUNCTION public.ple_mark_failed_export_job() FROM PUBLIC;

REVOKE ALL ON FUNCTION public.ple_promote_qti_grading(p_tenant uuid, p_workspace uuid, p_import uuid, p_problem uuid, p_version uuid, p_item_id text) FROM PUBLIC;
GRANT ALL ON FUNCTION public.ple_promote_qti_grading(p_tenant uuid, p_workspace uuid, p_import uuid, p_problem uuid, p_version uuid, p_item_id text) TO ple_app;

REVOKE ALL ON FUNCTION public.ple_question_statistics_view(p_problem uuid, p_version uuid) FROM PUBLIC;
GRANT ALL ON FUNCTION public.ple_question_statistics_view(p_problem uuid, p_version uuid) TO ple_app;

REVOKE ALL ON FUNCTION public.ple_ready_worker_queue_depth() FROM PUBLIC;
GRANT ALL ON FUNCTION public.ple_ready_worker_queue_depth() TO ple_app;

REVOKE ALL ON FUNCTION public.ple_reschedule_attempt_timing_job(p_tenant uuid, p_job uuid, p_token uuid, p_payload jsonb, p_available_at timestamp with time zone) FROM PUBLIC;
GRANT ALL ON FUNCTION public.ple_reschedule_attempt_timing_job(p_tenant uuid, p_job uuid, p_token uuid, p_payload jsonb, p_available_at timestamp with time zone) TO ple_app;

REVOKE ALL ON FUNCTION public.ple_record_question_statistics(p_tenant uuid, p_enrollment uuid, p_first_completed_run uuid, p_attempt uuid, p_problem uuid, p_version uuid, p_score double precision, p_attempts bigint, p_duration_seconds bigint, p_rest_score double precision, p_observation_sha256 bytea) FROM PUBLIC;
GRANT ALL ON FUNCTION public.ple_record_question_statistics(p_tenant uuid, p_enrollment uuid, p_first_completed_run uuid, p_attempt uuid, p_problem uuid, p_version uuid, p_score double precision, p_attempts bigint, p_duration_seconds bigint, p_rest_score double precision, p_observation_sha256 bytea) TO ple_app;

REVOKE ALL ON FUNCTION public.ple_statistics_aggregate_valid(p_cohort bigint, p_score_sum double precision, p_attempts bigint, p_histogram bigint[], p_scored bigint, p_score_mean double precision, p_rest_mean double precision, p_score_m2 double precision, p_rest_m2 double precision, p_co_moment double precision) FROM PUBLIC;

REVOKE ALL ON FUNCTION public.ple_statistics_canonical_float(p_value double precision) FROM PUBLIC;
