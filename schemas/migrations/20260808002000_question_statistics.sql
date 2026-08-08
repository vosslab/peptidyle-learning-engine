-- MOD-STATS: retention-safe, incremental statistics for immutable versions.
-- The shared aggregate contains only mergeable sufficient statistics.  The
-- tenant receipt establishes exactly-once contribution while its activity
-- records still exist; retention deletes the receipt, not the aggregate.

DO $$
BEGIN
    CREATE ROLE ple_statistics_broker
        NOLOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE NOINHERIT NOBYPASSRLS;
EXCEPTION WHEN duplicate_object THEN NULL;
END
$$;

-- PostgreSQL treats NaN unusually in comparisons and permits negative zero.
-- The server snapshot contract canonicalizes both, so storage does too.
CREATE FUNCTION ple_statistics_canonical_float(p_value double precision)
RETURNS boolean LANGUAGE sql IMMUTABLE STRICT SET search_path = pg_catalog AS $$
    SELECT p_value::text NOT IN ('NaN', 'Infinity', '-Infinity')
       AND encode(float8send(p_value), 'hex') <> '8000000000000000'
$$;

CREATE FUNCTION ple_statistics_aggregate_valid(
    p_cohort bigint,
    p_score_sum double precision,
    p_attempts bigint,
    p_histogram bigint[],
    p_scored bigint,
    p_score_mean double precision,
    p_rest_mean double precision,
    p_score_m2 double precision,
    p_rest_m2 double precision,
    p_co_moment double precision
) RETURNS boolean LANGUAGE plpgsql IMMUTABLE STRICT SET search_path = pg_catalog AS $$
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
ALTER FUNCTION ple_statistics_canonical_float(double precision) OWNER TO ple_statistics_broker;
REVOKE ALL ON FUNCTION ple_statistics_canonical_float(double precision) FROM PUBLIC;
ALTER FUNCTION ple_statistics_aggregate_valid(bigint, double precision, bigint, bigint[], bigint,
    double precision, double precision, double precision, double precision, double precision)
    OWNER TO ple_statistics_broker;
REVOKE ALL ON FUNCTION ple_statistics_aggregate_valid(bigint, double precision, bigint, bigint[], bigint,
    double precision, double precision, double precision, double precision, double precision) FROM PUBLIC;

CREATE TABLE question_statistics_aggregate (
    problem_id uuid NOT NULL,
    version_id uuid NOT NULL,
    cohort_size bigint NOT NULL CHECK (cohort_size >= 0),
    score_sum double precision NOT NULL
        CHECK (score_sum::text NOT IN ('NaN', 'Infinity', '-Infinity'))
        CHECK (score_sum >= 0 AND score_sum <= cohort_size::double precision),
    attempts_sum bigint NOT NULL CHECK (attempts_sum >= cohort_size),
    duration_histogram_version smallint NOT NULL CHECK (duration_histogram_version = 1),
    duration_histogram bigint[] NOT NULL
        CHECK (array_length(duration_histogram, 1) = 10)
        CHECK (array_position(duration_histogram, NULL) IS NULL)
        CHECK (0 <= ALL(duration_histogram))
        CHECK (
            cohort_size = duration_histogram[1] + duration_histogram[2]
                + duration_histogram[3] + duration_histogram[4] + duration_histogram[5]
                + duration_histogram[6] + duration_histogram[7] + duration_histogram[8]
                + duration_histogram[9] + duration_histogram[10]
        ),
    scored_cohort_size bigint NOT NULL CHECK (scored_cohort_size >= 0),
    score_mean double precision NOT NULL
        CHECK (score_mean::text NOT IN ('NaN', 'Infinity', '-Infinity'))
        CHECK (score_mean >= 0 AND score_mean <= 1),
    rest_score_mean double precision NOT NULL
        CHECK (rest_score_mean::text NOT IN ('NaN', 'Infinity', '-Infinity'))
        CHECK (rest_score_mean >= 0 AND rest_score_mean <= 1),
    score_m2 double precision NOT NULL
        CHECK (score_m2::text NOT IN ('NaN', 'Infinity', '-Infinity'))
        CHECK (score_m2 >= 0),
    rest_score_m2 double precision NOT NULL
        CHECK (rest_score_m2::text NOT IN ('NaN', 'Infinity', '-Infinity'))
        CHECK (rest_score_m2 >= 0),
    score_rest_co_moment double precision NOT NULL
        CHECK (score_rest_co_moment::text NOT IN ('NaN', 'Infinity', '-Infinity')),
    PRIMARY KEY (problem_id, version_id),
    FOREIGN KEY (problem_id, version_id)
        REFERENCES problem_version(problem_id, version_id) ON DELETE RESTRICT,
    CHECK (scored_cohort_size <= cohort_size),
    CHECK (public.ple_statistics_aggregate_valid(
        cohort_size, score_sum, attempts_sum, duration_histogram, scored_cohort_size,
        score_mean, rest_score_mean, score_m2, rest_score_m2, score_rest_co_moment
    ))
);

-- This is tenant-owned bookkeeping, not analytics data.  Its cascades let
-- retention remove identifying activity without ever touching the aggregate.
CREATE TABLE question_statistics_contribution_receipt (
    tenant_id uuid NOT NULL,
    enrollment_id uuid NOT NULL,
    first_completed_run_id uuid NOT NULL,
    attempt_id uuid NOT NULL,
    observation_sha256 bytea NOT NULL CHECK (octet_length(observation_sha256) = 32),
    problem_id uuid NOT NULL,
    version_id uuid NOT NULL,
    created_at timestamptz NOT NULL DEFAULT transaction_timestamp(),
    PRIMARY KEY (tenant_id, enrollment_id, problem_id, version_id),
    FOREIGN KEY (tenant_id, enrollment_id)
        REFERENCES enrollment(tenant_id, enrollment_id) ON DELETE CASCADE,
    FOREIGN KEY (tenant_id, first_completed_run_id)
        REFERENCES assignment_run(tenant_id, run_id) ON DELETE CASCADE,
    FOREIGN KEY (tenant_id, attempt_id)
        REFERENCES submission_idempotency(tenant_id, attempt_id) ON DELETE CASCADE,
    FOREIGN KEY (problem_id, version_id)
        REFERENCES problem_version(problem_id, version_id) ON DELETE RESTRICT
);

ALTER TABLE question_statistics_contribution_receipt ENABLE ROW LEVEL SECURITY;
ALTER TABLE question_statistics_contribution_receipt FORCE ROW LEVEL SECURITY;
CREATE POLICY question_statistics_contribution_receipt_tenant
    ON question_statistics_contribution_receipt
    USING (tenant_id = ple_current_tenant())
    WITH CHECK (tenant_id = ple_current_tenant());
CREATE POLICY question_statistics_contribution_receipt_broker
    ON question_statistics_contribution_receipt TO ple_statistics_broker
    USING (tenant_id = ple_current_tenant())
    WITH CHECK (tenant_id = ple_current_tenant());

-- The safe reader needs the same catalog visibility predicate as the app.
CREATE POLICY problem_version_statistics_visible_select ON problem_version
    FOR SELECT TO ple_statistics_broker
    USING (
        publication_scope = 'public'
        OR EXISTS (
            SELECT 1 FROM catalog_tenant_grant AS grant_row
            WHERE grant_row.problem_id = problem_version.problem_id
              AND grant_row.version_id = problem_version.version_id
              AND grant_row.tenant_id = ple_current_tenant()
        )
    );
CREATE POLICY catalog_tenant_grant_statistics_visible_select ON catalog_tenant_grant
    FOR SELECT TO ple_statistics_broker
    USING (tenant_id = ple_current_tenant());
CREATE POLICY assignment_run_statistics_broker ON assignment_run TO ple_statistics_broker
    USING (tenant_id = ple_current_tenant());
CREATE POLICY question_attempt_statistics_broker ON question_attempt TO ple_statistics_broker
    USING (tenant_id = ple_current_tenant());

GRANT USAGE ON SCHEMA public TO ple_statistics_broker;
GRANT EXECUTE ON FUNCTION ple_current_tenant() TO ple_statistics_broker;
GRANT EXECUTE ON FUNCTION ple_statistics_canonical_float(double precision) TO ple_statistics_broker;
GRANT EXECUTE ON FUNCTION ple_statistics_aggregate_valid(bigint, double precision, bigint,
    bigint[], bigint, double precision, double precision, double precision, double precision,
    double precision) TO ple_statistics_broker;
GRANT SELECT ON problem_version, catalog_tenant_grant TO ple_statistics_broker;
GRANT SELECT ON assignment_run, question_attempt TO ple_statistics_broker;
GRANT SELECT, INSERT ON question_statistics_contribution_receipt TO ple_statistics_broker;
GRANT SELECT, INSERT, UPDATE ON question_statistics_aggregate TO ple_statistics_broker;
REVOKE ALL ON question_statistics_aggregate FROM PUBLIC, ple_app, ple_student, ple_auth,
    ple_grader, ple_qti_grader, ple_queue_broker;
REVOKE ALL ON question_statistics_contribution_receipt FROM PUBLIC, ple_app, ple_student,
    ple_auth, ple_grader, ple_qti_grader, ple_queue_broker;

-- Only Store's private submission-completion seam may call this broker.  It
-- accepts one collapsed observation and atomically records its receipt before
-- merging the stable Welford/Chan sufficient terms.
CREATE FUNCTION ple_record_question_statistics(
    p_tenant uuid,
    p_enrollment uuid,
    p_first_completed_run uuid,
    p_attempt uuid,
    p_problem uuid,
    p_version uuid,
    p_score double precision,
    p_attempts bigint,
    p_duration_seconds bigint,
    p_rest_score double precision,
    p_observation_sha256 bytea
) RETURNS boolean LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog, public AS $$
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
ALTER FUNCTION ple_record_question_statistics(uuid, uuid, uuid, uuid, uuid, uuid,
    double precision, bigint, bigint, double precision, bytea) OWNER TO ple_statistics_broker;
REVOKE ALL ON FUNCTION ple_record_question_statistics(uuid, uuid, uuid, uuid, uuid, uuid,
    double precision, bigint, bigint, double precision, bytea) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_record_question_statistics(uuid, uuid, uuid, uuid, uuid, uuid,
    double precision, bigint, bigint, double precision, bytea) TO ple_app;

-- This is the only reader granted to the application.  It returns no row
-- below the global k=5 floor and cannot disclose a version invisible to the
-- active tenant's ordinary catalog policy.
CREATE FUNCTION ple_question_statistics_view(p_problem uuid, p_version uuid)
RETURNS TABLE (
    cohort_size bigint,
    difficulty_index double precision,
    attempts_mean double precision,
    time_median_seconds_estimate bigint,
    discrimination_index double precision
) LANGUAGE sql STABLE SECURITY DEFINER SET search_path = pg_catalog, public AS $$
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
ALTER FUNCTION ple_question_statistics_view(uuid, uuid) OWNER TO ple_statistics_broker;
REVOKE ALL ON FUNCTION ple_question_statistics_view(uuid, uuid) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_question_statistics_view(uuid, uuid) TO ple_app;
