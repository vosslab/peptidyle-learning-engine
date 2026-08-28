-- WP-PROF-G1 / G1-W5: immutable causal evidence for score invalidation.
--
-- A recalculation generation is derived data, never an unexplained queue row.
-- This table records only identity and causality: it intentionally contains no
-- learner response, evaluation, score, feedback, or private worker material.

BEGIN;

CREATE TABLE public.scoring_invalidation_origin (
    tenant_id uuid NOT NULL,
    origin_id uuid NOT NULL,
    course_id uuid NOT NULL,
    assignment_id uuid NOT NULL,
    scoring_generation bigint NOT NULL,
    recalculation_job_id uuid NOT NULL,
    grading_operation_id bigint NOT NULL,
    origin_kind text NOT NULL,
    actor_id uuid,
    occurred_at timestamp with time zone NOT NULL DEFAULT transaction_timestamp(),
    CONSTRAINT scoring_invalidation_origin_pkey
        PRIMARY KEY (tenant_id, origin_kind, origin_id),
    CONSTRAINT scoring_invalidation_origin_generation_unique
        UNIQUE (tenant_id, assignment_id, scoring_generation),
    CONSTRAINT scoring_invalidation_origin_job_unique
        UNIQUE (tenant_id, recalculation_job_id),
    CONSTRAINT scoring_invalidation_origin_operation_unique
        UNIQUE (tenant_id, grading_operation_id),
    CONSTRAINT scoring_invalidation_origin_generation_check CHECK (scoring_generation > 0),
    CONSTRAINT scoring_invalidation_origin_kind_check CHECK (
        origin_kind = ANY ('{instructor_recalculation,assignment_definition,
            manual_grade,learner_support,accepted_submission_completion}')
    ),
    CONSTRAINT scoring_invalidation_origin_actor_shape_check CHECK (
        (origin_kind = ANY ('{instructor_recalculation,assignment_definition,
            manual_grade,learner_support}') AND actor_id IS NOT NULL)
        OR (origin_kind = 'accepted_submission_completion' AND actor_id IS NULL)
    ),
    CONSTRAINT scoring_invalidation_origin_assignment_fk
        FOREIGN KEY (tenant_id, course_id, assignment_id)
        REFERENCES public.assignment(tenant_id, course_id, assignment_id),
    CONSTRAINT scoring_invalidation_origin_job_fk
        FOREIGN KEY (tenant_id, recalculation_job_id)
        REFERENCES public.worker_job(tenant_id, job_id)
        DEFERRABLE INITIALLY DEFERRED,
    CONSTRAINT scoring_invalidation_origin_operation_fk
        FOREIGN KEY (tenant_id, course_id, grading_operation_id)
        REFERENCES public.grading_operation(tenant_id, course_id, grading_operation_id)
        DEFERRABLE INITIALLY DEFERRED
);

CREATE INDEX scoring_invalidation_origin_assignment_time_idx
    ON public.scoring_invalidation_origin (
        tenant_id, course_id, assignment_id, occurred_at, origin_id
    );

ALTER TABLE public.scoring_invalidation_origin ENABLE ROW LEVEL SECURITY;
ALTER TABLE ONLY public.scoring_invalidation_origin FORCE ROW LEVEL SECURITY;

-- A causal record is immutable once accepted.  The retention package remains
-- the sole future owner of deletion policy, rather than making this table a
-- mutable operational log.
CREATE FUNCTION public.ple_reject_scoring_invalidation_origin_mutation()
RETURNS trigger
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public', pg_temp
AS $$
BEGIN
    RAISE EXCEPTION 'scoring invalidation origin is immutable' USING ERRCODE = '55000';
END;
$$;

REVOKE ALL ON FUNCTION public.ple_reject_scoring_invalidation_origin_mutation()
    FROM PUBLIC;

CREATE TRIGGER scoring_invalidation_origin_immutable
    BEFORE UPDATE OR DELETE ON public.scoring_invalidation_origin
    FOR EACH ROW EXECUTE FUNCTION public.ple_reject_scoring_invalidation_origin_mutation();

COMMIT;
