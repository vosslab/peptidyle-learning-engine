-- WP-PROF-D1: validity-governed catalog evidence and tenant-safe usage.
-- ASVS 1.2.4, 2.2.1-2.2.3, 2.3.1-2.3.4, 8.2.1-8.2.3, and 8.3.1.
BEGIN;
CREATE EXTENSION IF NOT EXISTS pgcrypto;
ALTER ROLE ple_statistics_broker
    NOLOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE NOINHERIT
    NOREPLICATION NOBYPASSRLS;
-- Issue-time scoring eligibility is the immutable source witness. The
-- backfill derives it from each run item's single canonical definition row.
ALTER TABLE public.assignment_run_item
    ADD COLUMN statistics_eligible boolean;
UPDATE public.assignment_run_item AS run_item
   SET statistics_eligible = CASE
       WHEN run_item.selection_group_id IS NULL THEN (
           SELECT item.points_possible <> 0 AND item.scoring_mode <> 'excluded'
             FROM public.assignment_item AS item
            WHERE item.tenant_id = run_item.tenant_id
              AND item.assignment_item_id = run_item.assignment_item_id
       )
       ELSE (
           SELECT selection_group.points_per_item <> 0
                  AND candidate.delivery_state = 'active'
             FROM public.assignment_selection_candidate AS candidate
             JOIN public.assignment_selection_group AS selection_group
               ON selection_group.tenant_id = candidate.tenant_id
              AND selection_group.assignment_id = candidate.assignment_id
              AND selection_group.selection_group_id = candidate.selection_group_id
            WHERE candidate.tenant_id = run_item.tenant_id
              AND candidate.candidate_id = run_item.assignment_item_id
              AND candidate.selection_group_id = run_item.selection_group_id
       )
 END;
DO $$
BEGIN
    IF EXISTS (
        SELECT 1 FROM public.assignment_run_item WHERE statistics_eligible IS NULL
    ) THEN
        RAISE EXCEPTION 'assignment run statistics eligibility backfill is incomplete';
    END IF;
END
$$;
ALTER TABLE public.assignment_run_item
    ALTER COLUMN statistics_eligible SET NOT NULL;
CREATE FUNCTION public.ple_guard_run_item_statistics_eligibility() RETURNS trigger
    LANGUAGE plpgsql
    SET search_path TO 'pg_catalog', 'public'
AS $$
BEGIN
    IF NEW.statistics_eligible IS DISTINCT FROM OLD.statistics_eligible THEN
        RAISE EXCEPTION 'issued statistics eligibility is immutable' USING ERRCODE = '55000';
    END IF;
    RETURN NEW;
END
$$;
REVOKE ALL ON FUNCTION public.ple_guard_run_item_statistics_eligibility() FROM PUBLIC;
CREATE TRIGGER assignment_run_item_statistics_eligibility_immutable
    BEFORE UPDATE OF statistics_eligible ON public.assignment_run_item
    FOR EACH ROW EXECUTE FUNCTION public.ple_guard_run_item_statistics_eligibility();
-- Publication-first indexes serve current-definition usage. Issued runs
-- remain the distinct historical evidence seam.
CREATE INDEX assignment_item_active_publication_usage_idx
    ON public.assignment_item (problem_id, version_id, tenant_id, assignment_id)
    WHERE delivery_state = 'active';
CREATE INDEX assignment_selection_candidate_active_publication_usage_idx
    ON public.assignment_selection_candidate
       (problem_id, version_id, tenant_id, assignment_id, selection_group_id)
    WHERE delivery_state = 'active';
-- Response family is browser-safe publication metadata, not a backend name.
ALTER TABLE public.problem_version ADD COLUMN response_family text;
ALTER TABLE public.catalog_search_document ADD COLUMN response_family text;
ALTER TABLE public.problem_version DISABLE TRIGGER problem_version_immutability;
UPDATE public.problem_version AS version SET response_family=payload.payload#>>'{question,response,kind}'
  FROM public.problem_version_payload payload
 WHERE payload.problem_id=version.problem_id AND payload.version_id=version.version_id;
ALTER TABLE public.problem_version ENABLE TRIGGER problem_version_immutability;
UPDATE public.catalog_search_document document SET response_family=version.response_family
  FROM public.problem_version version
 WHERE version.problem_id=document.problem_id AND version.version_id=document.version_id;
DO $$ BEGIN
    IF EXISTS (SELECT 1 FROM public.problem_version WHERE response_family IS NULL)
       OR EXISTS (SELECT 1 FROM public.catalog_search_document WHERE response_family IS NULL) THEN
        RAISE EXCEPTION 'published response-family backfill is incomplete';
    END IF;
END $$;
ALTER TABLE public.problem_version ALTER COLUMN response_family SET NOT NULL,
    ADD CONSTRAINT problem_version_response_family_check CHECK (response_family IN
        ('numeric','multipleChoice','shortText','multiBlank','matching','ordering','hotspot',
         'fileUpload','externalTool'));
ALTER TABLE public.catalog_search_document
    ALTER COLUMN response_family SET NOT NULL,
    ADD CONSTRAINT catalog_search_document_response_family_check CHECK (
        response_family IN ('numeric','multipleChoice','shortText','multiBlank',
            'matching','ordering','hotspot','fileUpload','externalTool')
    );
CREATE OR REPLACE FUNCTION public.ple_project_catalog_search_document() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path TO 'pg_catalog','public' AS $$
DECLARE stable_question_id character(7); rendered_byline text;
BEGIN
    SELECT question_id INTO stable_question_id FROM public.problem WHERE problem_id=NEW.problem_id;
    SELECT string_agg(value,', ' ORDER BY ordinality) INTO rendered_byline
      FROM unnest(NEW.public_byline) WITH ORDINALITY name(value,ordinality);
    INSERT INTO public.catalog_search_document(problem_id,version_id,question_id,title,backend,
        metadata,publication_scope,lifecycle,lifecycle_reason,public_byline,
        derived_from_problem_id,derived_from_version_id,published_at,byline_text,question_type,
        language,license,taxonomy,keywords,capabilities,search_text,response_family)
    VALUES(NEW.problem_id,NEW.version_id,stable_question_id,NEW.title,NEW.backend,NEW.metadata,
        NEW.publication_scope,NEW.lifecycle,NEW.lifecycle_reason,NEW.public_byline,
        NEW.derived_from_problem_id,NEW.derived_from_version_id,NEW.created_at,rendered_byline,
        NEW.backend,coalesce(NEW.metadata->>'language','und'),
        coalesce(NEW.metadata#>>'{license,kind}','unknown'),
        coalesce(NEW.metadata->'taxonomy','[]'::jsonb),coalesce(NEW.metadata->'tags','[]'::jsonb),
        NEW.capabilities,to_tsvector('simple',concat_ws(' ',NEW.title,rendered_byline,NEW.metadata::text)),
        NEW.response_family)
    ON CONFLICT(problem_id,version_id) DO UPDATE SET lifecycle=EXCLUDED.lifecycle,
        lifecycle_reason=EXCLUDED.lifecycle_reason,updated_at=transaction_timestamp();
    RETURN NEW;
END $$;
-- The established statistics tables remain the sole independent-learner
-- accumulator. `cohort_size` is the accepted first-attempt learner count.
DROP VIEW public.catalog_search_view;
DELETE FROM public.question_statistics_contribution_receipt;
DELETE FROM public.question_statistics_aggregate;
ALTER TABLE public.question_statistics_aggregate
    ADD COLUMN response_family text NOT NULL,
    ADD CONSTRAINT question_statistics_aggregate_response_family_check
        CHECK (response_family IN ('numeric','multipleChoice','shortText','multiBlank',
            'matching','ordering','hotspot','fileUpload','externalTool'));
ALTER TABLE public.question_statistics_contribution_receipt
    ADD COLUMN issued_position integer NOT NULL,
    ADD COLUMN response_family text NOT NULL,
    ADD COLUMN contribution_disposition text NOT NULL,
    ADD CONSTRAINT question_statistics_contribution_receipt_position_check
        CHECK (issued_position >= 0),
    ADD CONSTRAINT question_statistics_contribution_receipt_family_check
        CHECK (response_family IN ('numeric','multipleChoice','shortText','multiBlank',
            'matching','ordering','hotspot','fileUpload','externalTool')),
    ADD CONSTRAINT question_statistics_contribution_receipt_disposition_check
        CHECK (contribution_disposition IN ('accepted','duplicateLearner'));
CREATE VIEW public.catalog_search_view WITH (security_invoker=true) AS
SELECT document.problem_id,document.version_id,document.question_id,document.title,
       document.backend,document.metadata,document.publication_scope,document.lifecycle,
       document.lifecycle_reason,document.public_byline,document.derived_from_problem_id,
       document.derived_from_version_id,document.published_at,document.byline_text,
       document.question_type,document.language,document.license,document.taxonomy,
       document.keywords,document.capabilities,document.search_text,document.quality_signal,
       document.updated_at,statistics.cohort_size,statistics.difficulty_index,
       statistics.attempts_mean,statistics.time_median_seconds_estimate,
       statistics.discrimination_index,(statistics.cohort_size IS NOT NULL) statistics_available,
       document.catalog_sequence,document.normalized_search_text,document.response_family
  FROM public.catalog_search_document document
  LEFT JOIN LATERAL public.ple_question_statistics_view(
      document.problem_id,document.version_id) statistics ON true;
GRANT SELECT ON public.catalog_search_view TO ple_app,ple_student;
DROP TRIGGER question_statistics_disclosure_projection
    ON public.question_statistics_aggregate;
DROP FUNCTION public.ple_record_catalog_statistics_disclosure();
DROP TABLE public.catalog_statistics_disclosure;
CREATE TABLE public.catalog_discovery_learner_fingerprint_receipt (
    problem_id uuid NOT NULL, version_id uuid NOT NULL,
    learner_fingerprint bytea NOT NULL,
    first_valid_contribution_at timestamptz NOT NULL DEFAULT transaction_timestamp(),
    PRIMARY KEY (problem_id, version_id, learner_fingerprint),
    FOREIGN KEY (problem_id, version_id) REFERENCES public.problem_version(problem_id,version_id)
        ON DELETE RESTRICT,
    CHECK (octet_length(learner_fingerprint)=32)
);
ALTER TABLE public.catalog_discovery_learner_fingerprint_receipt ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.catalog_discovery_learner_fingerprint_receipt FORCE ROW LEVEL SECURITY;
CREATE POLICY catalog_discovery_learner_fingerprint_statistics_select
    ON public.catalog_discovery_learner_fingerprint_receipt FOR SELECT
    TO ple_statistics_broker USING (true);
CREATE POLICY catalog_discovery_learner_fingerprint_statistics_insert
    ON public.catalog_discovery_learner_fingerprint_receipt FOR INSERT
    TO ple_statistics_broker WITH CHECK (true);
-- Only an anonymous, domain-separated digest crosses the trusted activity
-- validation boundary.  It supports exact distinct-course counting without
-- retaining tenant, course, instructor, or learner identity in this relation.
CREATE TABLE public.catalog_discovery_course_fingerprint_receipt (
    problem_id uuid NOT NULL,
    version_id uuid NOT NULL,
    course_fingerprint bytea NOT NULL,
    first_valid_contribution_at timestamp with time zone NOT NULL
        DEFAULT transaction_timestamp(),
    PRIMARY KEY (problem_id, version_id, course_fingerprint),
    FOREIGN KEY (problem_id, version_id)
        REFERENCES public.problem_version(problem_id, version_id) ON DELETE RESTRICT,
    CONSTRAINT catalog_discovery_course_fingerprint_digest_check
        CHECK (octet_length(course_fingerprint) = 32)
);
ALTER TABLE public.catalog_discovery_course_fingerprint_receipt
    ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.catalog_discovery_course_fingerprint_receipt
    FORCE ROW LEVEL SECURITY;
CREATE POLICY catalog_discovery_course_fingerprint_statistics_select
    ON public.catalog_discovery_course_fingerprint_receipt
    FOR SELECT TO ple_statistics_broker USING (true);
CREATE POLICY catalog_discovery_course_fingerprint_statistics_insert
    ON public.catalog_discovery_course_fingerprint_receipt
    FOR INSERT TO ple_statistics_broker WITH CHECK (true);
-- Each disclosed row is an immutable catalog event.  Search chooses the most
-- recent row not newer than its cursor boundary; no identity-bearing key is
-- stored here.  Formula v1 measures evidence strength from disclosed course
-- breadth, first-attempt cohort size, and only positive discrimination.  It
-- deliberately does not treat difficulty as pedagogical goodness.
CREATE TABLE public.catalog_discovery_evidence_revision (
    problem_id uuid NOT NULL,
    version_id uuid NOT NULL,
    evidence_sequence bigint NOT NULL,
    formula_version smallint NOT NULL,
    response_family text NOT NULL,
    course_count bigint NOT NULL,
    first_attempt_count bigint NOT NULL,
    difficulty_index double precision NOT NULL,
    attempts_mean double precision NOT NULL,
    time_median_seconds_estimate bigint NOT NULL,
    discrimination_index double precision,
    quality_signal numeric(12,6) NOT NULL,
    evidence_at timestamp with time zone NOT NULL,
    PRIMARY KEY (problem_id, version_id, evidence_sequence),
    UNIQUE (evidence_sequence),
    FOREIGN KEY (problem_id, version_id)
        REFERENCES public.problem_version(problem_id, version_id) ON DELETE RESTRICT,
    CONSTRAINT catalog_discovery_evidence_revision_formula_check
        CHECK (formula_version = 1),
    CONSTRAINT catalog_discovery_evidence_revision_family_check
        CHECK (response_family IN ('numeric','multipleChoice','shortText','multiBlank',
            'matching','ordering','hotspot','fileUpload','externalTool')),
    CONSTRAINT catalog_discovery_evidence_revision_course_check CHECK (course_count >= 2),
    CONSTRAINT catalog_discovery_evidence_revision_cohort_check CHECK (first_attempt_count >= 5),
    CONSTRAINT catalog_discovery_evidence_revision_difficulty_check
        CHECK (difficulty_index >= 0 AND difficulty_index <= 1),
    CONSTRAINT catalog_discovery_evidence_revision_attempts_check CHECK (attempts_mean >= 1),
    CONSTRAINT catalog_discovery_evidence_revision_time_check
        CHECK (time_median_seconds_estimate >= 0),
    CONSTRAINT catalog_discovery_evidence_revision_discrimination_check
        CHECK (discrimination_index IS NULL OR discrimination_index BETWEEN -1 AND 1),
    CONSTRAINT catalog_discovery_evidence_revision_quality_check
        CHECK (quality_signal >= 0)
);
ALTER TABLE public.catalog_discovery_evidence_revision ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.catalog_discovery_evidence_revision FORCE ROW LEVEL SECURITY;
CREATE INDEX catalog_discovery_evidence_revision_latest_idx
    ON public.catalog_discovery_evidence_revision
       (problem_id, version_id, evidence_sequence DESC);
CREATE POLICY catalog_discovery_evidence_revision_visible_select
    ON public.catalog_discovery_evidence_revision FOR SELECT TO ple_app, ple_student
    USING (EXISTS (
        SELECT 1 FROM public.problem_version AS visible_version
         WHERE visible_version.problem_id = catalog_discovery_evidence_revision.problem_id
           AND visible_version.version_id = catalog_discovery_evidence_revision.version_id
           AND visible_version.lifecycle = 'published'
    ));
CREATE POLICY catalog_discovery_evidence_revision_statistics_select
    ON public.catalog_discovery_evidence_revision
    FOR SELECT TO ple_statistics_broker USING (true);
CREATE POLICY catalog_discovery_evidence_revision_statistics_insert
    ON public.catalog_discovery_evidence_revision
    FOR INSERT TO ple_statistics_broker WITH CHECK (true);
GRANT SELECT ON public.catalog_discovery_evidence_revision TO ple_app, ple_student;
CREATE FUNCTION public.ple_catalog_discovery_evidence_at(
    p_problem uuid, p_version uuid, p_event_boundary bigint
) RETURNS TABLE (
    evidence_sequence bigint, formula_version smallint, response_family text,
    course_count bigint, first_attempt_count bigint,
    difficulty_index double precision, attempts_mean double precision,
    time_median_seconds_estimate bigint, discrimination_index double precision,
    quality_signal numeric, evidence_at timestamp with time zone
) LANGUAGE sql STABLE
SET search_path TO 'pg_catalog', 'public'
AS $$
    SELECT revision.evidence_sequence, revision.formula_version,
           revision.response_family, revision.course_count,
           revision.first_attempt_count, revision.difficulty_index,
           revision.attempts_mean, revision.time_median_seconds_estimate,
           revision.discrimination_index, revision.quality_signal,
           revision.evidence_at
      FROM public.catalog_discovery_evidence_revision AS revision
     WHERE p_problem IS NOT NULL AND p_version IS NOT NULL
       AND p_event_boundary IS NOT NULL AND p_event_boundary >= 0
       AND revision.problem_id = p_problem
       AND revision.version_id = p_version
       AND revision.evidence_sequence <= p_event_boundary
     ORDER BY revision.evidence_sequence DESC
     LIMIT 1
$$;
REVOKE ALL ON FUNCTION public.ple_catalog_discovery_evidence_at(uuid, uuid, bigint)
    FROM PUBLIC;
GRANT EXECUTE ON FUNCTION public.ple_catalog_discovery_evidence_at(uuid, uuid, bigint)
    TO ple_app, ple_student;
-- The statistics broker already owns the cross-tenant anonymous aggregate.
-- The replacement function validates all identity-bearing inputs against the
-- immutable issued graph before it writes any anonymous state.
GRANT SELECT, INSERT ON public.catalog_discovery_course_fingerprint_receipt
    TO ple_statistics_broker;
GRANT SELECT, INSERT ON public.catalog_discovery_learner_fingerprint_receipt
    TO ple_statistics_broker;
GRANT SELECT, INSERT ON public.catalog_discovery_evidence_revision
    TO ple_statistics_broker;
GRANT SELECT ON public.enrollment, public.assignment,
    public.assignment_run_item, public.submission_evaluation
    TO ple_statistics_broker;
GRANT SELECT ON public.catalog_search_document TO ple_statistics_broker;
CREATE POLICY discovery_statistics_document_select ON public.catalog_search_document
    FOR SELECT TO ple_statistics_broker USING (true);
CREATE POLICY discovery_statistics_enrollment_select ON public.enrollment
    FOR SELECT TO ple_statistics_broker
    USING (tenant_id = public.ple_current_tenant());
CREATE POLICY discovery_statistics_assignment_select ON public.assignment
    FOR SELECT TO ple_statistics_broker
    USING (tenant_id = public.ple_current_tenant());
CREATE POLICY discovery_statistics_run_item_select ON public.assignment_run_item
    FOR SELECT TO ple_statistics_broker
    USING (tenant_id = public.ple_current_tenant());
CREATE POLICY discovery_statistics_evaluation_select ON public.submission_evaluation
    FOR SELECT TO ple_statistics_broker
    USING (tenant_id = public.ple_current_tenant());
GRANT UPDATE (quality_signal, updated_at) ON public.catalog_search_document
    TO ple_statistics_broker;
CREATE OR REPLACE FUNCTION public.ple_record_question_statistics(
    p_tenant uuid, p_enrollment uuid, p_first_completed_run uuid, p_attempt uuid,
    p_problem uuid, p_version uuid, p_score double precision, p_attempts bigint,
    p_duration_seconds bigint, p_rest_score double precision,
    p_observation_sha256 bytea
) RETURNS boolean
LANGUAGE plpgsql SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public'
AS $$
DECLARE
    v_course uuid;
    v_student uuid;
    v_family text;
    v_credit double precision;
    v_issued_position integer;
    v_inserted boolean;
    v_stored record;
    v_course_fingerprint bytea;
    v_learner_fingerprint bytea;
    v_new_learner boolean;
    v_course_count bigint;
    v_aggregate public.question_statistics_aggregate%ROWTYPE;
    v_duration_bins bigint[];
    v_discrimination double precision;
    v_median bigint;
    v_sequence bigint;
    v_quality numeric(12,6);
BEGIN
    IF p_tenant IS NULL OR p_enrollment IS NULL OR p_first_completed_run IS NULL
       OR p_attempt IS NULL OR p_problem IS NULL OR p_version IS NULL
       OR p_tenant IS DISTINCT FROM public.ple_current_tenant()
       OR p_score IS NULL OR NOT public.ple_statistics_canonical_float(p_score)
       OR p_score < 0 OR p_score > 1
       OR p_attempts IS NULL OR p_attempts < 1
       OR p_duration_seconds IS NULL OR p_duration_seconds < 0
       OR p_observation_sha256 IS NULL OR octet_length(p_observation_sha256) <> 32
       OR (p_rest_score IS NOT NULL AND (
            NOT public.ple_statistics_canonical_float(p_rest_score)
            OR p_rest_score < 0 OR p_rest_score > 1
       )) THEN
        RAISE EXCEPTION 'invalid discovery evidence contribution' USING ERRCODE = '22023';
    END IF;
    SELECT assignment.course_id, enrollment.student_id, document.response_family,
           evaluation.credit_fraction::double precision,
           run_item.issued_position
      INTO v_course, v_student, v_family, v_credit, v_issued_position
      FROM public.assignment_run AS run
      JOIN public.enrollment AS enrollment
       ON enrollment.tenant_id = run.tenant_id
       AND enrollment.enrollment_id = run.enrollment_id
       AND enrollment.enrollment_id = p_enrollment
      JOIN public.assignment AS assignment
        ON assignment.tenant_id = enrollment.tenant_id
       AND assignment.assignment_id = enrollment.assignment_id
      JOIN public.catalog_search_document AS document
        ON document.problem_id = p_problem AND document.version_id = p_version
      JOIN public.assignment_run_item AS run_item
       ON run_item.tenant_id = run.tenant_id
       AND run_item.run_id = run.run_id
      JOIN public.question_attempt AS attempt
        ON attempt.tenant_id = run.tenant_id
       AND attempt.run_id = run.run_id
       AND attempt.attempt_id = p_attempt
       AND attempt.assignment_position = run_item.issued_position
       AND attempt.problem_id = run_item.problem_id
       AND attempt.version_id = run_item.version_id
      JOIN public.submission_evaluation AS evaluation
        ON evaluation.tenant_id = attempt.tenant_id
       AND evaluation.attempt_id = attempt.attempt_id
       AND evaluation.grading_status = 'graded'
       AND evaluation.credit_fraction IS NOT NULL
     WHERE run.tenant_id = p_tenant
       AND run.run_id = p_first_completed_run
       AND run.completed_at IS NOT NULL
       AND run.payload ->> 'mode' = 'assigned'
       AND NOT EXISTS (
           SELECT 1 FROM public.assignment_run AS earlier_run
            WHERE earlier_run.tenant_id = run.tenant_id
              AND earlier_run.enrollment_id = run.enrollment_id
              AND earlier_run.completed_at IS NOT NULL
              AND earlier_run.payload ->> 'mode' = 'assigned'
              AND earlier_run.run_number < run.run_number
       )
       AND run_item.problem_id = p_problem
       AND run_item.version_id = p_version
       AND run_item.statistics_eligible
       AND attempt.attempt_status IN ('submitted', 'auto_submitted')
       AND run_item.issued_position = (
           SELECT min(candidate.issued_position)
             FROM public.assignment_run_item AS candidate
            WHERE candidate.tenant_id = run_item.tenant_id
              AND candidate.run_id = run_item.run_id
              AND candidate.problem_id = run_item.problem_id
              AND candidate.version_id = run_item.version_id
              AND candidate.statistics_eligible
       )
       AND attempt.attempt_id = (
           SELECT earlier.attempt_id
             FROM public.question_attempt AS earlier
             JOIN public.submission_evaluation AS earlier_evaluation
               ON earlier_evaluation.tenant_id = earlier.tenant_id
              AND earlier_evaluation.attempt_id = earlier.attempt_id
              AND earlier_evaluation.grading_status = 'graded'
              AND earlier_evaluation.credit_fraction IS NOT NULL
            WHERE earlier.tenant_id = run_item.tenant_id
              AND earlier.run_id = run_item.run_id
              AND earlier.assignment_position = run_item.issued_position
              AND earlier.problem_id = run_item.problem_id
              AND earlier.version_id = run_item.version_id
              AND earlier.attempt_status IN ('submitted', 'auto_submitted')
            ORDER BY earlier.submitted_at, earlier.occurred_at, earlier.attempt_id
            LIMIT 1
       );
    IF NOT FOUND OR v_family NOT IN ('numeric','multipleChoice','shortText','multiBlank',
        'matching','ordering','hotspot','fileUpload','externalTool')
       OR v_credit IS DISTINCT FROM p_score THEN
        RAISE EXCEPTION 'discovery evidence activity binding is invalid'
            USING ERRCODE = '22023';
    END IF;
    SELECT issued_position, attempt_id AS first_scored_attempt_id, response_family,
           observation_sha256 INTO v_stored
      FROM public.question_statistics_contribution_receipt
     WHERE tenant_id=p_tenant AND enrollment_id=p_enrollment
       AND problem_id=p_problem AND version_id=p_version;
    IF FOUND THEN
        IF v_stored.issued_position=v_issued_position
           AND v_stored.first_scored_attempt_id=p_attempt
           AND v_stored.response_family=v_family
           AND v_stored.observation_sha256=p_observation_sha256 THEN RETURN false; END IF;
        RAISE EXCEPTION 'discovery evidence contribution conflicts' USING ERRCODE='23505';
    END IF;
    v_learner_fingerprint := digest(convert_to(
        'ple-catalog-learner-evidence-v1:'||p_tenant::text||':'||v_student::text,'UTF8'),
        'sha256');
    INSERT INTO public.catalog_discovery_learner_fingerprint_receipt
        (problem_id,version_id,learner_fingerprint)
    VALUES (p_problem,p_version,v_learner_fingerprint) ON CONFLICT DO NOTHING
    RETURNING true INTO v_new_learner;
    INSERT INTO public.question_statistics_contribution_receipt (
        tenant_id, enrollment_id, first_completed_run_id, attempt_id,
        observation_sha256, problem_id, version_id, issued_position,
        response_family, contribution_disposition
    ) VALUES (
        p_tenant, p_enrollment, p_first_completed_run, p_attempt,
        p_observation_sha256, p_problem, p_version, v_issued_position, v_family,
        CASE WHEN COALESCE(v_new_learner,false) THEN 'accepted' ELSE 'duplicateLearner' END
    ) ON CONFLICT DO NOTHING
    RETURNING true INTO v_inserted;
    IF NOT COALESCE(v_inserted, false) THEN
        SELECT issued_position, attempt_id AS first_scored_attempt_id, response_family,
               observation_sha256
          INTO v_stored
          FROM public.question_statistics_contribution_receipt
         WHERE tenant_id = p_tenant AND enrollment_id = p_enrollment
           AND problem_id = p_problem AND version_id = p_version;
        IF v_stored.issued_position = v_issued_position
           AND v_stored.first_scored_attempt_id = p_attempt
           AND v_stored.response_family = v_family
           AND v_stored.observation_sha256 = p_observation_sha256 THEN
            RETURN false;
        END IF;
        RAISE EXCEPTION 'discovery evidence contribution conflicts'
            USING ERRCODE = '23505';
    END IF;
    IF NOT COALESCE(v_new_learner,false) THEN RETURN true; END IF;
    v_course_fingerprint := digest(
        convert_to(
            'ple-catalog-course-evidence-v1:' || p_tenant::text || ':' || v_course::text,
            'UTF8'
        ),
        'sha256'
    );
    INSERT INTO public.catalog_discovery_course_fingerprint_receipt (
        problem_id, version_id, course_fingerprint
    ) VALUES (p_problem, p_version, v_course_fingerprint)
    ON CONFLICT DO NOTHING;
    v_duration_bins := ARRAY[
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
        problem_id, version_id, response_family, cohort_size,
        score_sum, attempts_sum, duration_histogram_version,
        duration_histogram, scored_cohort_size, score_mean, rest_score_mean,
        score_m2, rest_score_m2, score_rest_co_moment
    ) VALUES (
        p_problem, p_version, v_family, 1, p_score, p_attempts, 1,
        v_duration_bins, CASE WHEN p_rest_score IS NULL THEN 0 ELSE 1 END,
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
        score_mean = CASE
            WHEN aggregate.scored_cohort_size + EXCLUDED.scored_cohort_size = 0 THEN 0
            ELSE aggregate.score_mean + (EXCLUDED.score_mean - aggregate.score_mean)
                * EXCLUDED.scored_cohort_size::double precision
                / (aggregate.scored_cohort_size + EXCLUDED.scored_cohort_size)::double precision
        END,
        rest_score_mean = CASE
            WHEN aggregate.scored_cohort_size + EXCLUDED.scored_cohort_size = 0 THEN 0
            ELSE aggregate.rest_score_mean
                + (EXCLUDED.rest_score_mean - aggregate.rest_score_mean)
                * EXCLUDED.scored_cohort_size::double precision
                / (aggregate.scored_cohort_size + EXCLUDED.scored_cohort_size)::double precision
        END,
        score_m2 = CASE
            WHEN aggregate.scored_cohort_size + EXCLUDED.scored_cohort_size = 0 THEN 0
            ELSE aggregate.score_m2 + EXCLUDED.score_m2
                + (EXCLUDED.score_mean - aggregate.score_mean) ^ 2
                * aggregate.scored_cohort_size::double precision
                * EXCLUDED.scored_cohort_size::double precision
                / (aggregate.scored_cohort_size + EXCLUDED.scored_cohort_size)::double precision
        END,
        rest_score_m2 = CASE
            WHEN aggregate.scored_cohort_size + EXCLUDED.scored_cohort_size = 0 THEN 0
            ELSE aggregate.rest_score_m2 + EXCLUDED.rest_score_m2
                + (EXCLUDED.rest_score_mean - aggregate.rest_score_mean) ^ 2
                * aggregate.scored_cohort_size::double precision
                * EXCLUDED.scored_cohort_size::double precision
                / (aggregate.scored_cohort_size + EXCLUDED.scored_cohort_size)::double precision
        END,
        score_rest_co_moment = CASE
            WHEN aggregate.scored_cohort_size + EXCLUDED.scored_cohort_size = 0 THEN 0
            ELSE aggregate.score_rest_co_moment + EXCLUDED.score_rest_co_moment
                + (EXCLUDED.score_mean - aggregate.score_mean)
                * (EXCLUDED.rest_score_mean - aggregate.rest_score_mean)
                * aggregate.scored_cohort_size::double precision
                * EXCLUDED.scored_cohort_size::double precision
                / (aggregate.scored_cohort_size + EXCLUDED.scored_cohort_size)::double precision
        END,
        scored_cohort_size = aggregate.scored_cohort_size + EXCLUDED.scored_cohort_size
    WHERE aggregate.response_family = EXCLUDED.response_family
    RETURNING * INTO v_aggregate;
    IF NOT FOUND THEN
        RAISE EXCEPTION 'discovery evidence response family conflicts'
            USING ERRCODE = '23505';
    END IF;
    SELECT count(*) INTO v_course_count
      FROM public.catalog_discovery_course_fingerprint_receipt
     WHERE problem_id = p_problem AND version_id = p_version;
    IF v_aggregate.cohort_size < 5 OR v_course_count < 2 THEN
        RETURN true;
    END IF;
    v_discrimination := CASE
        WHEN v_aggregate.scored_cohort_size < 5
          OR v_aggregate.score_m2 <= 0 OR v_aggregate.rest_score_m2 <= 0 THEN NULL
        ELSE GREATEST(-1::double precision, LEAST(1::double precision,
            v_aggregate.score_rest_co_moment
            / sqrt(v_aggregate.score_m2 * v_aggregate.rest_score_m2)))
    END;
    v_median := CASE
        WHEN v_aggregate.duration_histogram[1] >= (v_aggregate.cohort_size + 1) / 2 THEN 1
        WHEN v_aggregate.duration_histogram[1] + v_aggregate.duration_histogram[2] >= (v_aggregate.cohort_size + 1) / 2 THEN 5
        WHEN v_aggregate.duration_histogram[1] + v_aggregate.duration_histogram[2] + v_aggregate.duration_histogram[3] >= (v_aggregate.cohort_size + 1) / 2 THEN 15
        WHEN v_aggregate.duration_histogram[1] + v_aggregate.duration_histogram[2] + v_aggregate.duration_histogram[3] + v_aggregate.duration_histogram[4] >= (v_aggregate.cohort_size + 1) / 2 THEN 30
        WHEN v_aggregate.duration_histogram[1] + v_aggregate.duration_histogram[2] + v_aggregate.duration_histogram[3] + v_aggregate.duration_histogram[4] + v_aggregate.duration_histogram[5] >= (v_aggregate.cohort_size + 1) / 2 THEN 60
        WHEN v_aggregate.duration_histogram[1] + v_aggregate.duration_histogram[2] + v_aggregate.duration_histogram[3] + v_aggregate.duration_histogram[4] + v_aggregate.duration_histogram[5] + v_aggregate.duration_histogram[6] >= (v_aggregate.cohort_size + 1) / 2 THEN 120
        WHEN v_aggregate.duration_histogram[1] + v_aggregate.duration_histogram[2] + v_aggregate.duration_histogram[3] + v_aggregate.duration_histogram[4] + v_aggregate.duration_histogram[5] + v_aggregate.duration_histogram[6] + v_aggregate.duration_histogram[7] >= (v_aggregate.cohort_size + 1) / 2 THEN 300
        WHEN v_aggregate.duration_histogram[1] + v_aggregate.duration_histogram[2] + v_aggregate.duration_histogram[3] + v_aggregate.duration_histogram[4] + v_aggregate.duration_histogram[5] + v_aggregate.duration_histogram[6] + v_aggregate.duration_histogram[7] + v_aggregate.duration_histogram[8] >= (v_aggregate.cohort_size + 1) / 2 THEN 900
        WHEN v_aggregate.duration_histogram[1] + v_aggregate.duration_histogram[2] + v_aggregate.duration_histogram[3] + v_aggregate.duration_histogram[4] + v_aggregate.duration_histogram[5] + v_aggregate.duration_histogram[6] + v_aggregate.duration_histogram[7] + v_aggregate.duration_histogram[8] + v_aggregate.duration_histogram[9] >= (v_aggregate.cohort_size + 1) / 2 THEN 3600
        ELSE 86400
    END;
    v_quality := round((
        ln(1 + v_course_count::double precision)
        + ln(1 + v_aggregate.cohort_size::double precision)
        + GREATEST(COALESCE(v_discrimination, 0), 0)
    )::numeric, 6);
    v_sequence := nextval('public.catalog_search_publication_sequence');
    INSERT INTO public.catalog_discovery_evidence_revision (
        problem_id, version_id, evidence_sequence, formula_version,
        response_family, course_count, first_attempt_count, difficulty_index,
        attempts_mean, time_median_seconds_estimate, discrimination_index,
        quality_signal, evidence_at
    ) VALUES (
        p_problem, p_version, v_sequence, 1, v_family, v_course_count,
        v_aggregate.cohort_size,
        v_aggregate.score_sum / v_aggregate.cohort_size::double precision,
        v_aggregate.attempts_sum::double precision
            / v_aggregate.cohort_size::double precision,
        v_median, v_discrimination, v_quality, transaction_timestamp()
    );
    UPDATE public.catalog_search_document
       SET quality_signal = v_quality, updated_at = transaction_timestamp()
     WHERE problem_id = p_problem AND version_id = p_version;
    RETURN true;
END
$$;
ALTER FUNCTION public.ple_record_question_statistics(
    uuid, uuid, uuid, uuid, uuid, uuid, double precision, bigint, bigint,
    double precision, bytea
) OWNER TO ple_statistics_broker;
REVOKE ALL ON FUNCTION public.ple_record_question_statistics(
    uuid, uuid, uuid, uuid, uuid, uuid, double precision, bigint, bigint,
    double precision, bytea
) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION public.ple_record_question_statistics(
    uuid, uuid, uuid, uuid, uuid, uuid, double precision, bigint, bigint,
    double precision, bytea
) TO ple_app;
-- Preserve the established reader signature while making the immutable D1
-- revision, rather than the private mutable accumulator, its disclosure
-- authority.  This suppresses otherwise adequate single-course cohorts.
CREATE OR REPLACE FUNCTION public.ple_question_statistics_view(
    p_problem uuid, p_version uuid
) RETURNS TABLE (
    cohort_size bigint, difficulty_index double precision,
    attempts_mean double precision, time_median_seconds_estimate bigint,
    discrimination_index double precision
) LANGUAGE sql STABLE SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public'
AS $$
    SELECT revision.first_attempt_count, revision.difficulty_index,
           revision.attempts_mean, revision.time_median_seconds_estimate,
           revision.discrimination_index
      FROM public.catalog_discovery_evidence_revision AS revision
      JOIN public.problem_version AS visible_version
        ON visible_version.problem_id = revision.problem_id
       AND visible_version.version_id = revision.version_id
     WHERE revision.problem_id = p_problem
       AND revision.version_id = p_version
     ORDER BY revision.evidence_sequence DESC
     LIMIT 1
$$;
-- Actor-owned names remain a tenant-scoped capability and never enter the
-- catalog search view, evidence revision, filters, or facets.
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'ple_catalog_usage_broker') THEN
        CREATE ROLE ple_catalog_usage_broker
            NOLOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE NOINHERIT
            NOREPLICATION NOBYPASSRLS;
    END IF;
END
$$;
ALTER ROLE ple_catalog_usage_broker
    NOLOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE NOINHERIT
    NOREPLICATION NOBYPASSRLS;
DO $$
DECLARE
    membership_edge record;
BEGIN
    FOR membership_edge IN
        SELECT parent_role.rolname AS parent_name, member_role.rolname AS member_name
          FROM pg_catalog.pg_auth_members AS membership
          JOIN pg_catalog.pg_roles AS parent_role ON parent_role.oid = membership.roleid
          JOIN pg_catalog.pg_roles AS member_role ON member_role.oid = membership.member
         WHERE membership.member IN (
                   'ple_statistics_broker'::regrole,
                   'ple_catalog_usage_broker'::regrole
               )
            OR membership.roleid IN (
                   'ple_statistics_broker'::regrole,
                   'ple_catalog_usage_broker'::regrole
               )
    LOOP
        EXECUTE format(
            'REVOKE %I FROM %I',
            membership_edge.parent_name,
            membership_edge.member_name
        );
    END LOOP;
END
$$;
REVOKE ALL ON SCHEMA public FROM ple_catalog_usage_broker;
GRANT USAGE ON SCHEMA public TO ple_catalog_usage_broker;
GRANT EXECUTE ON FUNCTION public.ple_current_tenant() TO ple_catalog_usage_broker;
GRANT EXECUTE ON FUNCTION public.ple_course_records_accessible(uuid,uuid)
    TO ple_catalog_usage_broker;
-- ASVS 8.1.1: catalog discovery derives the actor and discovery authority
-- only from one persisted, active same-tenant session.  Approval remains an
-- Instructor eligibility requirement; a persisted Sysadmin role may browse
-- without gaining any ambient course-record authority.
CREATE FUNCTION public.ple_catalog_discovery_actor(
    p_session character(64), p_tenant uuid
) RETURNS TABLE (user_id uuid)
    LANGUAGE sql STABLE SECURITY DEFINER
    SET search_path TO 'pg_catalog', 'public'
AS $$
    SELECT session.user_id
      FROM public.auth_session AS session
     WHERE p_session IS NOT NULL
       AND p_tenant IS NOT NULL
       AND session.session_hash = p_session
       AND session.tenant_id = p_tenant
       AND session.tenant_id = public.ple_current_tenant()
       AND session.revoked_at IS NULL
       AND session.expires_at > transaction_timestamp()
       AND (
           session.roles @> '["sysadmin"]'::jsonb
           OR (
               session.roles @> '["instructor"]'::jsonb
               AND public.ple_instructor_approval_eligible(session.user_id)
           )
       )
$$;
ALTER FUNCTION public.ple_catalog_discovery_actor(character,uuid)
    OWNER TO ple_teaching_authority_broker;
REVOKE ALL ON FUNCTION public.ple_catalog_discovery_actor(character,uuid) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION public.ple_catalog_discovery_actor(character,uuid)
    TO ple_catalog_usage_broker;
CREATE POLICY catalog_usage_broker_grant_select ON public.catalog_tenant_grant
    FOR SELECT TO ple_catalog_usage_broker
    USING (tenant_id = public.ple_current_tenant());
CREATE POLICY catalog_usage_broker_version_select ON public.problem_version
    FOR SELECT TO ple_catalog_usage_broker
    USING (publication_scope = 'public' OR EXISTS (
        SELECT 1 FROM public.catalog_tenant_grant AS grant_row
         WHERE grant_row.tenant_id = public.ple_current_tenant()
           AND grant_row.problem_id = problem_version.problem_id
           AND grant_row.version_id = problem_version.version_id
    ));
CREATE POLICY catalog_usage_broker_document_select ON public.catalog_search_document
    FOR SELECT TO ple_catalog_usage_broker
    USING (EXISTS (
        SELECT 1 FROM public.problem_version AS visible_version
         WHERE visible_version.problem_id = catalog_search_document.problem_id
           AND visible_version.version_id = catalog_search_document.version_id
           AND visible_version.lifecycle = 'published'
    ));
CREATE POLICY catalog_usage_broker_course_select ON public.course
    FOR SELECT TO ple_catalog_usage_broker
    USING (tenant_id = public.ple_current_tenant());
CREATE POLICY catalog_usage_broker_member_select ON public.course_member
    FOR SELECT TO ple_catalog_usage_broker
    USING (tenant_id = public.ple_current_tenant());
CREATE POLICY catalog_usage_broker_assignment_select ON public.assignment
    FOR SELECT TO ple_catalog_usage_broker
    USING (tenant_id = public.ple_current_tenant());
CREATE POLICY catalog_usage_broker_item_select ON public.assignment_item
    FOR SELECT TO ple_catalog_usage_broker
    USING (tenant_id = public.ple_current_tenant());
CREATE POLICY catalog_usage_broker_candidate_select
    ON public.assignment_selection_candidate
    FOR SELECT TO ple_catalog_usage_broker
    USING (tenant_id = public.ple_current_tenant());
GRANT SELECT ON public.catalog_tenant_grant, public.problem_version,
    public.catalog_search_document, public.course, public.course_member,
    public.assignment, public.assignment_item,
    public.assignment_selection_candidate
    TO ple_catalog_usage_broker;
CREATE FUNCTION public.ple_instructor_catalog_course_usage(
    p_tenant uuid, p_session character(64), p_question_id character(7),
    p_after_course_reference integer, p_limit integer
) RETURNS TABLE (
    course_reference integer, course_title text, assignment_count bigint,
    fixed_reference_count bigint, pool_candidate_count bigint
) LANGUAGE sql STABLE SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public'
AS $$
    WITH actor AS (
        SELECT subject.user_id
          FROM public.ple_catalog_discovery_actor(p_session, p_tenant) AS subject
         WHERE p_tenant IS NOT NULL
           AND p_tenant = public.ple_current_tenant()
           AND p_session IS NOT NULL
           AND p_question_id IS NOT NULL
           AND p_limit BETWEEN 1 AND 100
           AND (p_after_course_reference IS NULL OR p_after_course_reference >= 0)
    ), publication AS (
        SELECT document.problem_id, document.version_id
          FROM public.catalog_search_document AS document
         WHERE document.question_id = p_question_id
           AND document.lifecycle = 'published'
    ), reference AS (
        SELECT item.tenant_id, item.assignment_id, item.problem_id,
               item.version_id, 1::bigint AS fixed_count, 0::bigint AS pool_count
          FROM public.assignment_item AS item
         WHERE item.delivery_state = 'active'
        UNION ALL
        SELECT candidate.tenant_id, candidate.assignment_id,
               candidate.problem_id, candidate.version_id,
               0::bigint, 1::bigint
          FROM public.assignment_selection_candidate AS candidate
         WHERE candidate.delivery_state = 'active'
    )
    SELECT course.public_id, course.title,
           count(DISTINCT assignment.assignment_id)::bigint,
           sum(reference.fixed_count)::bigint,
           sum(reference.pool_count)::bigint
      FROM actor
      JOIN public.course_member AS membership
        ON membership.tenant_id = p_tenant
       AND membership.user_id = actor.user_id
       AND membership.role = 'instructor'
       AND membership.status = 'active'
      JOIN public.course AS course
        ON course.tenant_id = membership.tenant_id
       AND course.course_id = membership.course_id
      JOIN public.assignment AS assignment
        ON assignment.tenant_id = course.tenant_id
       AND assignment.course_id = course.course_id
      JOIN reference
        ON reference.tenant_id = assignment.tenant_id
       AND reference.assignment_id = assignment.assignment_id
      JOIN publication
        ON publication.problem_id = reference.problem_id
       AND publication.version_id = reference.version_id
     WHERE public.ple_course_records_accessible(course.tenant_id,course.course_id)
       AND (p_after_course_reference IS NULL
        OR course.public_id > p_after_course_reference)
     GROUP BY course.public_id, course.title
     ORDER BY course.public_id
     LIMIT p_limit
$$;
ALTER FUNCTION public.ple_instructor_catalog_course_usage(
    uuid, character, character, integer, integer
) OWNER TO ple_catalog_usage_broker;
REVOKE ALL ON FUNCTION public.ple_instructor_catalog_course_usage(
    uuid, character, character, integer, integer
) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION public.ple_instructor_catalog_course_usage(
    uuid, character, character, integer, integer
) TO ple_app;
CREATE FUNCTION public.ple_instructor_catalog_usage_summary(
    p_tenant uuid, p_session character(64), p_question_id character(7)
) RETURNS TABLE (
    institution_course_count bigint, institution_assignment_count bigint,
    own_course_count bigint, own_assignment_count bigint
) LANGUAGE sql STABLE SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public'
AS $$
    WITH actor AS (
        SELECT subject.user_id
          FROM public.ple_catalog_discovery_actor(p_session, p_tenant) AS subject
         WHERE p_tenant IS NOT NULL
           AND p_tenant = public.ple_current_tenant()
           AND p_session IS NOT NULL
           AND p_question_id IS NOT NULL
    ), publication AS (
        SELECT document.problem_id, document.version_id
          FROM public.catalog_search_document AS document
         WHERE document.question_id = p_question_id
           AND document.lifecycle = 'published'
    ), reference AS (
        SELECT item.tenant_id, item.assignment_id, item.problem_id, item.version_id
          FROM public.assignment_item AS item
         WHERE item.delivery_state = 'active'
        UNION
        SELECT candidate.tenant_id, candidate.assignment_id,
               candidate.problem_id, candidate.version_id
          FROM public.assignment_selection_candidate AS candidate
         WHERE candidate.delivery_state = 'active'
    ), institution_usage AS (
        SELECT assignment.course_id, assignment.assignment_id
          FROM reference
          JOIN publication USING (problem_id, version_id)
          JOIN public.assignment AS assignment
            ON assignment.tenant_id = reference.tenant_id
           AND assignment.assignment_id = reference.assignment_id
         WHERE assignment.tenant_id = p_tenant
    ), own_usage AS (
        SELECT usage.course_id, usage.assignment_id
          FROM institution_usage AS usage
          JOIN public.course AS own_course
            ON own_course.tenant_id=p_tenant AND own_course.course_id=usage.course_id
          JOIN public.course_member AS membership
            ON membership.tenant_id = p_tenant
           AND membership.course_id = usage.course_id
           AND membership.role = 'instructor'
           AND membership.status = 'active'
          JOIN actor ON actor.user_id = membership.user_id
         WHERE public.ple_course_records_accessible(
             own_course.tenant_id,own_course.course_id)
    )
    SELECT count(DISTINCT institution_usage.course_id)::bigint,
           count(DISTINCT institution_usage.assignment_id)::bigint,
           (SELECT count(DISTINCT course_id)::bigint FROM own_usage),
           (SELECT count(DISTINCT assignment_id)::bigint FROM own_usage)
      FROM actor
      LEFT JOIN institution_usage ON true
$$;
ALTER FUNCTION public.ple_instructor_catalog_usage_summary(
    uuid, character, character
) OWNER TO ple_catalog_usage_broker;
REVOKE ALL ON FUNCTION public.ple_instructor_catalog_usage_summary(
    uuid, character, character
) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION public.ple_instructor_catalog_usage_summary(
    uuid, character, character
) TO ple_app;
-- Closed capability assertions make accidental privilege broadening a failed
-- migration rather than a latent application defect.
DO $$
BEGIN
    IF EXISTS (
        SELECT 1 FROM pg_roles
         WHERE rolname = 'ple_catalog_usage_broker'
           AND (rolcanlogin OR rolsuper OR rolinherit OR rolbypassrls)
    ) OR has_function_privilege(
        'public',
        'public.ple_record_question_statistics(uuid,uuid,uuid,uuid,uuid,uuid,double precision,bigint,bigint,double precision,bytea)'::regprocedure,
        'EXECUTE'
    ) OR has_function_privilege(
        'public',
        'public.ple_catalog_discovery_actor(character,uuid)'::regprocedure,
        'EXECUTE'
    ) OR has_function_privilege(
        'ple_app',
        'public.ple_catalog_discovery_actor(character,uuid)'::regprocedure,
        'EXECUTE'
    ) OR NOT has_function_privilege(
        'ple_catalog_usage_broker',
        'public.ple_catalog_discovery_actor(character,uuid)'::regprocedure,
        'EXECUTE'
    ) OR has_function_privilege(
        'ple_catalog_usage_broker',
        'public.ple_target_session_subject(character,uuid)'::regprocedure,
        'EXECUTE'
    ) OR has_function_privilege(
        'ple_catalog_usage_broker',
        'public.ple_instructor_approval_eligible(uuid)'::regprocedure,
        'EXECUTE'
    ) OR has_function_privilege(
        'public',
        'public.ple_instructor_catalog_course_usage(uuid,character,character,integer,integer)'::regprocedure,
        'EXECUTE'
    ) OR has_function_privilege(
        'public',
        'public.ple_instructor_catalog_usage_summary(uuid,character,character)'::regprocedure,
        'EXECUTE'
    ) OR has_table_privilege(
        'ple_app', 'public.catalog_discovery_course_fingerprint_receipt', 'SELECT'
    ) OR has_table_privilege(
        'ple_app', 'public.catalog_discovery_learner_fingerprint_receipt', 'SELECT'
    ) THEN
        RAISE EXCEPTION 'catalog discovery evidence authority is unsafe';
    END IF;
END
$$;
COMMIT;
