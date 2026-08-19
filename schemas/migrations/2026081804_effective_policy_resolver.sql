-- Professor S3: one normalized policy representation and immutable active-attempt evidence.
-- This is a pre-production direct cutover.  Old policy/timing rows have no truthful
-- provenance in the normalized receipt model, so refuse them rather than invent it.

BEGIN;

DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM public.assignment)
       OR EXISTS (SELECT 1 FROM public.assignment_policy_exception)
       OR EXISTS (SELECT 1 FROM public.attempt_timing_current) THEN
        RAISE EXCEPTION
            'effective-policy cutover requires a clean pre-production database; '
            'recreate it from migrations'
            USING ERRCODE = '55000';
    END IF;
END
$$;

-- These broker functions are the only accepted functions that named the mutable
-- timing projection.  The replacement continues to validate worker payloads
-- against the current effect pointer, never against historical receipts.
CREATE OR REPLACE FUNCTION public.ple_cancel_attempt_timing_job(p_tenant uuid, p_job uuid)
RETURNS boolean
LANGUAGE plpgsql SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public'
AS $$
DECLARE changed boolean;
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
               SELECT 1 FROM public.attempt_effective_policy_current AS current_effect
                WHERE current_effect.tenant_id = p_tenant AND current_effect.job_id = p_job
           )
        RETURNING 1
    )
    SELECT EXISTS(SELECT 1 FROM completed) INTO changed;
    RETURN changed;
END
$$;

CREATE OR REPLACE FUNCTION public.ple_reschedule_attempt_timing_job(
    p_tenant uuid, p_job uuid, p_token uuid, p_payload jsonb,
    p_available_at timestamp with time zone
) RETURNS boolean
LANGUAGE plpgsql SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public'
AS $$
DECLARE changed boolean;
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
          FROM public.attempt_effective_policy_current AS current_effect
         WHERE job.job_id = p_job AND job.tenant_id = p_tenant
           AND current_effect.tenant_id = p_tenant AND current_effect.job_id = p_job
           AND current_effect.attempt_id = (p_payload ->> 'attempt')::uuid
           AND current_effect.timing_generation = (p_payload ->> 'timing_generation')::bigint
           AND ((p_token IS NULL AND job.state = 'ready')
             OR (p_token IS NOT NULL AND job.state = 'leased'
                 AND job.lease_token = p_token
                 AND job.lease_expires_at > transaction_timestamp()))
        RETURNING 1
    )
    SELECT EXISTS(SELECT 1 FROM rescheduled) INTO changed;
    RETURN changed;
END
$$;

CREATE TABLE public.assignment_effective_policy_base (
    tenant_id uuid NOT NULL,
    assignment_id uuid NOT NULL,
    course_id uuid NOT NULL,
    available_at timestamp with time zone,
    due_at timestamp with time zone,
    closes_at timestamp with time zone,
    late_submission_policy text NOT NULL,
    deadline_behavior text NOT NULL,
    time_limit_seconds integer,
    attempt_limit integer,
    updated_at timestamp with time zone NOT NULL DEFAULT transaction_timestamp(),
    CONSTRAINT assignment_effective_policy_base_pkey PRIMARY KEY (tenant_id, assignment_id),
    CONSTRAINT assignment_effective_policy_base_course_key
        UNIQUE (tenant_id, course_id, assignment_id),
    CONSTRAINT assignment_effective_policy_base_assignment_fk
        FOREIGN KEY (tenant_id, course_id, assignment_id)
        REFERENCES public.assignment(tenant_id, course_id, assignment_id) ON DELETE CASCADE,
    CONSTRAINT assignment_effective_policy_base_late_check
        CHECK (late_submission_policy IN ('accept', 'reject', 'mark_late')),
    CONSTRAINT assignment_effective_policy_base_deadline_behavior_check
        CHECK (deadline_behavior = 'auto_submit'),
    CONSTRAINT assignment_effective_policy_base_time_limit_check
        CHECK (time_limit_seconds IS NULL OR time_limit_seconds > 0),
    CONSTRAINT assignment_effective_policy_base_attempt_limit_check
        CHECK (attempt_limit IS NULL OR attempt_limit > 0),
    CONSTRAINT assignment_effective_policy_base_schedule_check
        CHECK ((available_at IS NULL OR due_at IS NULL OR available_at <= due_at)
           AND (due_at IS NULL OR closes_at IS NULL OR due_at <= closes_at)
           AND (available_at IS NULL OR closes_at IS NULL OR available_at <= closes_at))
);

CREATE TABLE public.assignment_group_schedule_offset (
    tenant_id uuid NOT NULL,
    assignment_id uuid NOT NULL,
    course_id uuid NOT NULL,
    course_group_id uuid NOT NULL,
    schedule_offset_seconds integer NOT NULL,
    created_at timestamp with time zone NOT NULL DEFAULT transaction_timestamp(),
    updated_at timestamp with time zone NOT NULL DEFAULT transaction_timestamp(),
    CONSTRAINT assignment_group_schedule_offset_pkey
        PRIMARY KEY (tenant_id, assignment_id, course_group_id),
    CONSTRAINT assignment_group_schedule_offset_assignment_fk
        FOREIGN KEY (tenant_id, course_id, assignment_id)
        REFERENCES public.assignment(tenant_id, course_id, assignment_id) ON DELETE CASCADE,
    CONSTRAINT assignment_group_schedule_offset_group_fk
        FOREIGN KEY (tenant_id, course_id, course_group_id)
        REFERENCES public.course_group(tenant_id, course_id, course_group_id) ON DELETE CASCADE,
    CONSTRAINT assignment_group_schedule_offset_bounds_check
        CHECK (schedule_offset_seconds BETWEEN -31536000 AND 31536000
           AND schedule_offset_seconds <> 0)
);

CREATE TABLE public.assignment_group_accommodation (
    tenant_id uuid NOT NULL,
    assignment_id uuid NOT NULL,
    course_id uuid NOT NULL,
    course_group_id uuid NOT NULL,
    override_kind text NOT NULL,
    available_mode text,
    available_at timestamp with time zone,
    due_mode text,
    due_at timestamp with time zone,
    closes_mode text,
    closes_at timestamp with time zone,
    time_limit_mode text,
    time_limit_seconds integer,
    attempt_limit_mode text,
    attempt_limit integer,
    created_at timestamp with time zone NOT NULL DEFAULT transaction_timestamp(),
    updated_at timestamp with time zone NOT NULL DEFAULT transaction_timestamp(),
    CONSTRAINT assignment_group_accommodation_pkey
        PRIMARY KEY (tenant_id, assignment_id, course_group_id),
    CONSTRAINT assignment_group_accommodation_assignment_fk
        FOREIGN KEY (tenant_id, course_id, assignment_id)
        REFERENCES public.assignment(tenant_id, course_id, assignment_id) ON DELETE CASCADE,
    CONSTRAINT assignment_group_accommodation_group_fk
        FOREIGN KEY (tenant_id, course_id, course_group_id)
        REFERENCES public.course_group(tenant_id, course_id, course_group_id) ON DELETE CASCADE,
    CONSTRAINT assignment_group_accommodation_kind_check
        CHECK (override_kind IN ('extend_only', 'explicit_override')),
    CONSTRAINT assignment_group_accommodation_available_check
        CHECK ((available_mode IS NULL AND available_at IS NULL)
            OR (available_mode = 'unrestricted' AND available_at IS NULL)
            OR (available_mode = 'at' AND available_at IS NOT NULL)),
    CONSTRAINT assignment_group_accommodation_due_check
        CHECK ((due_mode IS NULL AND due_at IS NULL)
            OR (due_mode = 'unrestricted' AND due_at IS NULL)
            OR (due_mode = 'at' AND due_at IS NOT NULL)),
    CONSTRAINT assignment_group_accommodation_closes_check
        CHECK ((closes_mode IS NULL AND closes_at IS NULL)
            OR (closes_mode = 'unrestricted' AND closes_at IS NULL)
            OR (closes_mode = 'at' AND closes_at IS NOT NULL)),
    CONSTRAINT assignment_group_accommodation_time_limit_check
        CHECK ((time_limit_mode IS NULL AND time_limit_seconds IS NULL)
            OR (time_limit_mode = 'unlimited' AND time_limit_seconds IS NULL)
            OR (time_limit_mode = 'value' AND time_limit_seconds > 0)),
    CONSTRAINT assignment_group_accommodation_attempt_limit_check
        CHECK ((attempt_limit_mode IS NULL AND attempt_limit IS NULL)
            OR (attempt_limit_mode = 'unlimited' AND attempt_limit IS NULL)
            OR (attempt_limit_mode = 'value' AND attempt_limit > 0)),
    CONSTRAINT assignment_group_accommodation_nonempty_check
        CHECK (available_mode IS NOT NULL OR due_mode IS NOT NULL OR closes_mode IS NOT NULL
            OR time_limit_mode IS NOT NULL OR attempt_limit_mode IS NOT NULL),
    CONSTRAINT assignment_group_accommodation_local_schedule_check
        CHECK (available_mode <> 'at' OR due_mode <> 'at' OR available_at <= due_at),
    CONSTRAINT assignment_group_accommodation_local_due_close_check
        CHECK (due_mode <> 'at' OR closes_mode <> 'at' OR due_at <= closes_at),
    CONSTRAINT assignment_group_accommodation_local_schedule_close_check
        CHECK (available_mode <> 'at' OR closes_mode <> 'at' OR available_at <= closes_at)
);

CREATE TABLE public.assignment_individual_policy_exception (
    tenant_id uuid NOT NULL,
    assignment_individual_policy_exception_id uuid NOT NULL,
    assignment_id uuid NOT NULL,
    course_id uuid NOT NULL,
    student_id uuid NOT NULL,
    override_kind text NOT NULL,
    available_mode text,
    available_at timestamp with time zone,
    due_mode text,
    due_at timestamp with time zone,
    closes_mode text,
    closes_at timestamp with time zone,
    time_limit_mode text,
    time_limit_seconds integer,
    attempt_limit_mode text,
    attempt_limit integer,
    created_at timestamp with time zone NOT NULL DEFAULT transaction_timestamp(),
    updated_at timestamp with time zone NOT NULL DEFAULT transaction_timestamp(),
    CONSTRAINT assignment_individual_policy_exception_pkey
        PRIMARY KEY (tenant_id, assignment_id, student_id),
    CONSTRAINT assignment_individual_policy_exception_identity_key
        UNIQUE (assignment_individual_policy_exception_id),
    CONSTRAINT assignment_individual_policy_exception_assignment_fk
        FOREIGN KEY (tenant_id, course_id, assignment_id)
        REFERENCES public.assignment(tenant_id, course_id, assignment_id) ON DELETE CASCADE,
    CONSTRAINT assignment_individual_policy_exception_student_fk
        FOREIGN KEY (tenant_id, student_id)
        REFERENCES public.tenant_learner_identity(tenant_id, student_id) ON DELETE CASCADE,
    CONSTRAINT assignment_individual_policy_exception_kind_check
        CHECK (override_kind IN ('extend_only', 'explicit_override')),
    CONSTRAINT assignment_individual_policy_exception_available_check
        CHECK ((available_mode IS NULL AND available_at IS NULL)
            OR (available_mode = 'unrestricted' AND available_at IS NULL)
            OR (available_mode = 'at' AND available_at IS NOT NULL)),
    CONSTRAINT assignment_individual_policy_exception_due_check
        CHECK ((due_mode IS NULL AND due_at IS NULL)
            OR (due_mode = 'unrestricted' AND due_at IS NULL)
            OR (due_mode = 'at' AND due_at IS NOT NULL)),
    CONSTRAINT assignment_individual_policy_exception_closes_check
        CHECK ((closes_mode IS NULL AND closes_at IS NULL)
            OR (closes_mode = 'unrestricted' AND closes_at IS NULL)
            OR (closes_mode = 'at' AND closes_at IS NOT NULL)),
    CONSTRAINT assignment_individual_policy_exception_time_limit_check
        CHECK ((time_limit_mode IS NULL AND time_limit_seconds IS NULL)
            OR (time_limit_mode = 'unlimited' AND time_limit_seconds IS NULL)
            OR (time_limit_mode = 'value' AND time_limit_seconds > 0)),
    CONSTRAINT assignment_individual_policy_exception_attempt_limit_check
        CHECK ((attempt_limit_mode IS NULL AND attempt_limit IS NULL)
            OR (attempt_limit_mode = 'unlimited' AND attempt_limit IS NULL)
            OR (attempt_limit_mode = 'value' AND attempt_limit > 0)),
    CONSTRAINT assignment_individual_policy_exception_nonempty_check
        CHECK (available_mode IS NOT NULL OR due_mode IS NOT NULL OR closes_mode IS NOT NULL
            OR time_limit_mode IS NOT NULL OR attempt_limit_mode IS NOT NULL),
    CONSTRAINT assignment_individual_policy_exception_local_schedule_check
        CHECK (available_mode <> 'at' OR due_mode <> 'at' OR available_at <= due_at),
    CONSTRAINT assignment_individual_policy_exception_local_due_close_check
        CHECK (due_mode <> 'at' OR closes_mode <> 'at' OR due_at <= closes_at),
    CONSTRAINT assignment_individual_policy_exception_local_schedule_close_check
        CHECK (available_mode <> 'at' OR closes_mode <> 'at' OR available_at <= closes_at)
);

CREATE TABLE public.attempt_effective_policy_receipt (
    tenant_id uuid NOT NULL,
    attempt_id uuid NOT NULL,
    receipt_generation bigint NOT NULL,
    attempt_occurred_at timestamp with time zone NOT NULL,
    assignment_id uuid NOT NULL,
    course_id uuid NOT NULL,
    resolved_at timestamp with time zone NOT NULL DEFAULT transaction_timestamp(),
    resolved_available_at timestamp with time zone,
    resolved_due_at timestamp with time zone,
    resolved_closes_at timestamp with time zone,
    resolved_late_submission_policy text NOT NULL,
    resolved_deadline_behavior text NOT NULL,
    resolved_time_limit_seconds integer,
    resolved_attempt_limit integer,
    effective_deadline timestamp with time zone,
    effective_grace_seconds integer NOT NULL DEFAULT 0,
    auto_submit_at timestamp with time zone,
    assignment_revision bigint NOT NULL,
    sealed_at timestamp with time zone,
    CONSTRAINT attempt_effective_policy_receipt_pkey
        PRIMARY KEY (tenant_id, attempt_id, receipt_generation),
    CONSTRAINT attempt_effective_policy_receipt_binding_key
        UNIQUE (tenant_id, attempt_id, receipt_generation, course_id, assignment_id),
    CONSTRAINT attempt_effective_policy_receipt_attempt_fk
        FOREIGN KEY (tenant_id, attempt_id, attempt_occurred_at)
        REFERENCES public.question_attempt(tenant_id, attempt_id, occurred_at) ON DELETE CASCADE,
    CONSTRAINT attempt_effective_policy_receipt_assignment_fk
        FOREIGN KEY (tenant_id, course_id, assignment_id)
        REFERENCES public.assignment(tenant_id, course_id, assignment_id) ON DELETE CASCADE,
    CONSTRAINT attempt_effective_policy_receipt_generation_check CHECK (receipt_generation > 0),
    CONSTRAINT attempt_effective_policy_receipt_revision_check CHECK (assignment_revision > 0),
    CONSTRAINT attempt_effective_policy_receipt_late_check
        CHECK (resolved_late_submission_policy IN ('accept', 'reject', 'mark_late')),
    CONSTRAINT attempt_effective_policy_receipt_deadline_behavior_check
        CHECK (resolved_deadline_behavior = 'auto_submit'),
    CONSTRAINT attempt_effective_policy_receipt_limits_check
        CHECK ((resolved_time_limit_seconds IS NULL OR resolved_time_limit_seconds > 0)
           AND (resolved_attempt_limit IS NULL OR resolved_attempt_limit > 0)
           AND effective_grace_seconds >= 0),
    CONSTRAINT attempt_effective_policy_receipt_schedule_check
        CHECK ((resolved_available_at IS NULL OR resolved_due_at IS NULL
                OR resolved_available_at <= resolved_due_at)
           AND (resolved_due_at IS NULL OR resolved_closes_at IS NULL
                OR resolved_due_at <= resolved_closes_at)
           AND (resolved_available_at IS NULL OR resolved_closes_at IS NULL
                OR resolved_available_at <= resolved_closes_at)),
    CONSTRAINT attempt_effective_policy_receipt_effect_check
        CHECK ((effective_deadline IS NULL AND auto_submit_at IS NULL)
            OR (effective_deadline IS NOT NULL AND auto_submit_at IS NOT NULL
                AND auto_submit_at = effective_deadline
                    + make_interval(secs => effective_grace_seconds)))
);

CREATE TABLE public.attempt_effective_policy_receipt_field_source (
    tenant_id uuid NOT NULL,
    attempt_id uuid NOT NULL,
    receipt_generation bigint NOT NULL,
    field_name text NOT NULL,
    source_order integer NOT NULL,
    source_layer text NOT NULL,
    source_id uuid,
    CONSTRAINT attempt_effective_policy_receipt_field_source_pkey
        PRIMARY KEY (tenant_id, attempt_id, receipt_generation, field_name, source_order),
    CONSTRAINT attempt_effective_policy_receipt_field_source_receipt_fk
        FOREIGN KEY (tenant_id, attempt_id, receipt_generation)
        REFERENCES public.attempt_effective_policy_receipt
            (tenant_id, attempt_id, receipt_generation)
        ON DELETE CASCADE,
    CONSTRAINT attempt_effective_policy_receipt_field_source_field_check
        CHECK (field_name IN ('available_at', 'due_at', 'closes_at',
            'late_submission_policy', 'deadline_behavior', 'time_limit_seconds', 'attempt_limit',
            'effective_deadline', 'auto_submit_at')),
    CONSTRAINT attempt_effective_policy_receipt_field_source_order_check
        CHECK (source_order >= 0),
    CONSTRAINT attempt_effective_policy_receipt_field_source_shape_check
        CHECK ((source_layer = 'base' AND source_id IS NULL)
            OR (source_layer IN ('group_schedule_offset', 'group_accommodation',
                'individual_exception') AND source_id IS NOT NULL))
);

CREATE TABLE public.attempt_effective_policy_current (
    tenant_id uuid NOT NULL,
    attempt_id uuid NOT NULL,
    attempt_occurred_at timestamp with time zone NOT NULL,
    assignment_id uuid NOT NULL,
    course_id uuid NOT NULL,
    receipt_generation bigint NOT NULL,
    timing_generation bigint NOT NULL,
    job_id uuid,
    updated_at timestamp with time zone NOT NULL DEFAULT transaction_timestamp(),
    CONSTRAINT attempt_effective_policy_current_pkey PRIMARY KEY (tenant_id, attempt_id),
    CONSTRAINT attempt_effective_policy_current_attempt_fk
        FOREIGN KEY (tenant_id, attempt_id, attempt_occurred_at)
        REFERENCES public.question_attempt(tenant_id, attempt_id, occurred_at) ON DELETE CASCADE,
    CONSTRAINT attempt_effective_policy_current_assignment_fk
        FOREIGN KEY (tenant_id, course_id, assignment_id)
        REFERENCES public.assignment(tenant_id, course_id, assignment_id) ON DELETE CASCADE,
    CONSTRAINT attempt_effective_policy_current_receipt_fk
        FOREIGN KEY (tenant_id, attempt_id, receipt_generation, course_id, assignment_id)
        REFERENCES public.attempt_effective_policy_receipt
            (tenant_id, attempt_id, receipt_generation, course_id, assignment_id),
    CONSTRAINT attempt_effective_policy_current_job_key UNIQUE (job_id),
    CONSTRAINT attempt_effective_policy_current_generation_check
        CHECK (receipt_generation > 0 AND timing_generation > 0),
    CONSTRAINT attempt_effective_policy_current_job_fk
        FOREIGN KEY (job_id) REFERENCES public.worker_job(job_id) ON DELETE SET NULL
);

CREATE FUNCTION public.ple_guard_attempt_effective_policy_receipt() RETURNS trigger
LANGUAGE plpgsql
SET search_path TO 'pg_catalog', 'public'
AS $$
BEGIN
    IF TG_OP = 'INSERT' THEN
        IF NEW.sealed_at IS NOT NULL THEN
            RAISE EXCEPTION 'effective-policy receipt must be appended unsealed'
                USING ERRCODE = '23514';
        END IF;
        IF NOT EXISTS (
            SELECT 1
              FROM public.question_attempt attempt
              JOIN public.assignment_run run
                ON run.tenant_id = attempt.tenant_id AND run.run_id = attempt.run_id
              JOIN public.enrollment enrollment
                ON enrollment.tenant_id = run.tenant_id
               AND enrollment.enrollment_id = run.enrollment_id
              JOIN public.assignment assignment
                ON assignment.tenant_id = enrollment.tenant_id
               AND assignment.assignment_id = enrollment.assignment_id
             WHERE attempt.tenant_id = NEW.tenant_id
               AND attempt.attempt_id = NEW.attempt_id
               AND attempt.occurred_at = NEW.attempt_occurred_at
               AND assignment.course_id = NEW.course_id
               AND assignment.assignment_id = NEW.assignment_id
        ) THEN
            RAISE EXCEPTION 'effective-policy receipt does not match its attempt assignment'
                USING ERRCODE = '23503';
        END IF;
        RETURN NEW;
    END IF;
    IF TG_OP = 'DELETE' AND current_user = 'ple_retention_broker' THEN
        RETURN OLD;
    END IF;
    IF TG_OP <> 'UPDATE' THEN
        RAISE EXCEPTION 'effective-policy receipt deletion is retention-owned'
            USING ERRCODE = '55000';
    END IF;
    IF OLD.sealed_at IS NOT NULL OR NEW.sealed_at IS NULL
       OR NEW.sealed_at < OLD.resolved_at
       OR (to_jsonb(NEW) - 'sealed_at') IS DISTINCT FROM (to_jsonb(OLD) - 'sealed_at') THEN
        RAISE EXCEPTION 'effective-policy receipts are append-only and seal once'
            USING ERRCODE = '55000';
    END IF;
    IF EXISTS (
        SELECT 1
          FROM (VALUES ('available_at'), ('due_at'), ('closes_at'),
                       ('time_limit_seconds'), ('attempt_limit'),
                       ('late_submission_policy'), ('deadline_behavior')) AS required(field_name)
         WHERE NOT EXISTS (
             SELECT 1 FROM public.attempt_effective_policy_receipt_field_source source
              WHERE source.tenant_id = OLD.tenant_id
                AND source.attempt_id = OLD.attempt_id
                AND source.receipt_generation = OLD.receipt_generation
                AND source.field_name = required.field_name
         )
    ) THEN
        RAISE EXCEPTION 'cannot seal an effective-policy receipt without complete field provenance'
            USING ERRCODE = '23514';
    END IF;
    RETURN NEW;
END
$$;

CREATE FUNCTION public.ple_guard_attempt_effective_policy_current() RETURNS trigger
LANGUAGE plpgsql
SET search_path TO 'pg_catalog', 'public'
AS $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM public.attempt_effective_policy_receipt receipt
         WHERE receipt.tenant_id = NEW.tenant_id AND receipt.attempt_id = NEW.attempt_id
           AND receipt.receipt_generation = NEW.receipt_generation
           AND receipt.sealed_at IS NOT NULL
    ) THEN
        RAISE EXCEPTION 'active effective-policy pointer requires an exact sealed receipt'
            USING ERRCODE = '23514';
    END IF;
    RETURN NEW;
END
$$;

CREATE FUNCTION public.ple_guard_attempt_effective_policy_source() RETURNS trigger
LANGUAGE plpgsql
SET search_path TO 'pg_catalog', 'public'
AS $$
BEGIN
    IF TG_OP = 'INSERT' THEN
        IF EXISTS (
            SELECT 1 FROM public.attempt_effective_policy_receipt receipt
             WHERE receipt.tenant_id = NEW.tenant_id AND receipt.attempt_id = NEW.attempt_id
               AND receipt.receipt_generation = NEW.receipt_generation
               AND receipt.sealed_at IS NOT NULL
        ) THEN
            RAISE EXCEPTION 'cannot add provenance to a sealed effective-policy receipt'
                USING ERRCODE = '55000';
        END IF;
        RETURN NEW;
    END IF;
    IF TG_OP = 'DELETE' AND current_user = 'ple_retention_broker' THEN
        RETURN OLD;
    END IF;
    IF TG_OP = 'UPDATE' THEN
        RAISE EXCEPTION 'effective-policy field sources are immutable' USING ERRCODE = '55000';
    END IF;
    IF EXISTS (
        SELECT 1 FROM public.attempt_effective_policy_receipt receipt
         WHERE receipt.tenant_id = NEW.tenant_id AND receipt.attempt_id = NEW.attempt_id
           AND receipt.receipt_generation = NEW.receipt_generation
           AND receipt.sealed_at IS NOT NULL
    ) THEN
        RAISE EXCEPTION 'cannot add provenance to a sealed effective-policy receipt'
            USING ERRCODE = '55000';
    END IF;
    RETURN NEW;
END
$$;

CREATE TRIGGER attempt_effective_policy_receipt_immutable
BEFORE INSERT OR UPDATE OR DELETE ON public.attempt_effective_policy_receipt
FOR EACH ROW EXECUTE FUNCTION public.ple_guard_attempt_effective_policy_receipt();
CREATE TRIGGER attempt_effective_policy_receipt_source_immutable
BEFORE INSERT OR UPDATE OR DELETE ON public.attempt_effective_policy_receipt_field_source
FOR EACH ROW EXECUTE FUNCTION public.ple_guard_attempt_effective_policy_source();
CREATE TRIGGER attempt_effective_policy_current_sealed_receipt
BEFORE INSERT OR UPDATE OF receipt_generation ON public.attempt_effective_policy_current
FOR EACH ROW EXECUTE FUNCTION public.ple_guard_attempt_effective_policy_current();

-- Static replacement of the accepted purge graph with only normalized relation
-- substitutions.
CREATE OR REPLACE FUNCTION public.ple_commit_delete_retention_work_before_passwordless_identity(p_tenant
uuid, p_job uuid, p_token uuid, p_course uuid, p_stage text, p_generation bigint)
RETURNS boolean LANGUAGE plpgsql SECURITY DEFINER SET search_path TO 'pg_catalog',
'public' AS $$ DECLARE prepared_count bigint; manifest_count bigint; current_lifecycle
text; frozen_assignment_disposition text; BEGIN IF p_tenant IS NULL OR p_tenant
IS DISTINCT FROM public.ple_current_tenant() OR p_course IS NULL OR p_generation
<= 0 OR p_stage <> 'deleteStudentRecords' THEN RAISE EXCEPTION 'invalid retention worker capability'
USING ERRCODE = '22023'; END IF; PERFORM 1 FROM public.worker_job w JOIN public.course_retention_dispatch
d ON d.tenant_id = w.tenant_id AND d.course_id = p_course AND d.stage = p_stage
AND d.generation = p_generation AND d.job_id = w.job_id JOIN public.course_retention
r ON r.tenant_id = d.tenant_id AND r.course_id = d.course_id AND r.generation
= d.generation WHERE w.job_id = p_job AND w.tenant_id = p_tenant AND w.state
= 'leased' AND w.lease_token = p_token AND w.lease_expires_at > transaction_timestamp()
AND w.payload = jsonb_build_object('kind', 'retention', 'course', p_course::text,
'stage', p_stage, 'generation', p_generation) FOR UPDATE OF w, r; IF NOT FOUND
THEN RETURN false; END IF; PERFORM 1 FROM public.course_retention_stage s WHERE
s.tenant_id = p_tenant AND s.course_id = p_course AND s.stage = p_stage AND
s.generation = p_generation AND s.state = 'started' AND s.job_id = p_job AND
s.lease_token = p_token FOR UPDATE; IF NOT FOUND THEN RETURN false; END IF;
SELECT r.lifecycle, r.assignment_disposition INTO current_lifecycle, frozen_assignment_disposition
FROM public.course_retention r WHERE r.tenant_id = p_tenant AND r.course_id
= p_course AND r.generation = p_generation FOR UPDATE; IF NOT FOUND OR current_lifecycle
<> 'archived' THEN RETURN false; END IF; SELECT m.object_count, COALESCE(( SELECT
COUNT(*) FROM public.course_retention_cleanup_manifest_object o WHERE o.tenant_id
= m.tenant_id AND o.course_id = m.course_id AND o.generation = m.generation
AND o.stage = m.stage ), 0) INTO prepared_count, manifest_count FROM public.course_retention_cleanup_manifest
m WHERE m.tenant_id = p_tenant AND m.course_id = p_course AND m.stage = p_stage
AND m.generation = p_generation AND m.job_id = p_job AND m.state = 'prepared'
FOR UPDATE; IF NOT FOUND OR prepared_count IS NULL THEN RETURN false; END IF;
IF prepared_count IS DISTINCT FROM manifest_count THEN
RAISE EXCEPTION 'prepared manifest rows do not match manifest object count';
END IF; DELETE FROM public.feedback_release fr WHERE EXISTS ( SELECT 1 FROM
public.attempt_feedback af WHERE af.tenant_id = fr.tenant_id AND af.attempt_id
= fr.attempt_id AND af.course_id = p_course AND af.tenant_id = p_tenant ); DELETE
FROM public.submission_receipt_snapshot srs WHERE EXISTS ( SELECT 1 FROM public.submission_idempotency
si WHERE si.tenant_id = srs.tenant_id AND si.course_id = p_course AND si.attempt_id
= srs.attempt_id AND si.tenant_id = p_tenant ); DELETE FROM public.submission_next_attempt
sna WHERE sna.tenant_id = p_tenant AND ( EXISTS ( SELECT 1 FROM public.course_retention_purge_attempt
s WHERE s.tenant_id = p_tenant AND s.course_id = p_course AND s.generation =
p_generation AND s.stage = p_stage AND s.attempt_id = sna.predecessor_attempt_id
) OR EXISTS ( SELECT 1 FROM public.course_retention_purge_attempt s WHERE s.tenant_id
= p_tenant AND s.course_id = p_course AND s.generation = p_generation AND s.stage
= p_stage AND s.attempt_id = sna.next_attempt_id ) ); DELETE FROM public.question_statistics_contribution_receipt
qsr WHERE qsr.tenant_id = p_tenant AND ( EXISTS ( SELECT 1 FROM public.course_retention_purge_run
s WHERE s.tenant_id = p_tenant AND s.course_id = p_course AND s.generation =
p_generation AND s.stage = p_stage AND s.run_id = qsr.first_completed_run_id
) OR EXISTS ( SELECT 1 FROM public.course_retention_purge_attempt s WHERE s.tenant_id
= p_tenant AND s.course_id = p_course AND s.generation = p_generation AND s.stage
= p_stage AND s.attempt_id = qsr.attempt_id ) ); DELETE FROM public.question_prefetch
qp WHERE qp.tenant_id = p_tenant AND ( EXISTS ( SELECT 1 FROM public.course_retention_purge_run
s WHERE s.tenant_id = p_tenant AND s.course_id = p_course AND s.generation =
p_generation AND s.stage = p_stage AND s.run_id = qp.run_id ) OR EXISTS ( SELECT
1 FROM public.course_retention_purge_attempt s WHERE s.tenant_id = p_tenant
AND s.course_id = p_course AND s.generation = p_generation AND s.stage = p_stage
AND s.attempt_id = qp.predecessor_attempt_id ) ); DELETE FROM public.external_tool_launch_session
WHERE tenant_id = p_tenant AND course_id = p_course; DELETE FROM public.external_tool_exchange
WHERE tenant_id = p_tenant AND course_id = p_course; DELETE FROM public.attempt_feedback
WHERE tenant_id = p_tenant AND course_id = p_course; DELETE FROM public.attempt_score_current
WHERE tenant_id = p_tenant AND course_id = p_course; DELETE FROM public.manual_grade_receipt
WHERE tenant_id = p_tenant AND course_id = p_course; DELETE FROM public.submission_evaluation
WHERE tenant_id = p_tenant AND course_id = p_course; DELETE FROM public.submission
WHERE tenant_id = p_tenant AND course_id = p_course; DELETE FROM public.submission_idempotency
WHERE tenant_id = p_tenant AND course_id = p_course; DELETE FROM public.worker_job
w USING public.course_retention_purge_attempt purge WHERE purge.tenant_id =
p_tenant AND purge.course_id = p_course AND purge.generation = p_generation
AND purge.stage = p_stage AND w.tenant_id = purge.tenant_id AND w.payload ->>
'kind' = 'autoSubmitAttempt' AND (w.payload ->> 'attempt')::uuid = purge.attempt_id;
DELETE FROM public.attempt_effective_policy_current WHERE tenant_id = p_tenant
AND course_id = p_course; DELETE FROM public.attempt_effective_policy_receipt_field_source
source USING public.attempt_effective_policy_receipt receipt WHERE receipt.tenant_id
= p_tenant AND receipt.course_id = p_course AND source.tenant_id = receipt.tenant_id
AND source.attempt_id = receipt.attempt_id AND source.receipt_generation = receipt.receipt_generation;
DELETE FROM public.attempt_effective_policy_receipt WHERE tenant_id = p_tenant
AND course_id = p_course; DELETE FROM public.question_attempt WHERE tenant_id
= p_tenant AND course_id = p_course; DELETE FROM public.course_item_analysis_current
WHERE tenant_id = p_tenant AND course_id = p_course; DELETE FROM public.course_item_analysis_staging
WHERE tenant_id = p_tenant AND course_id = p_course; DELETE FROM public.worker_job
w WHERE w.tenant_id = p_tenant AND w.payload ->> 'kind' = 'recalculateCourseItemAnalysis'
AND EXISTS ( SELECT 1 FROM public.assignment a WHERE a.tenant_id = w.tenant_id
AND a.assignment_id = (w.payload ->> 'assignment')::uuid AND a.course_id = p_course
); DELETE FROM public.student_assignment_summary sas WHERE EXISTS ( SELECT 1
FROM public.enrollment e JOIN public.assignment a ON a.tenant_id = e.tenant_id
AND a.assignment_id = e.assignment_id WHERE e.tenant_id = sas.tenant_id AND
e.enrollment_id = sas.enrollment_id AND a.course_id = p_course AND sas.tenant_id
= p_tenant ); DELETE FROM public.assignment_run ar WHERE EXISTS ( SELECT 1 FROM
public.enrollment e JOIN public.assignment a ON a.tenant_id = e.tenant_id AND
a.assignment_id = e.assignment_id WHERE e.tenant_id = ar.tenant_id AND e.enrollment_id
= ar.enrollment_id AND a.course_id = p_course AND ar.tenant_id = p_tenant );
DELETE FROM public.enrollment e WHERE EXISTS ( SELECT 1 FROM public.assignment
a WHERE a.tenant_id = e.tenant_id AND a.assignment_id = e.assignment_id AND
a.course_id = p_course AND e.tenant_id = p_tenant ); DELETE FROM public.record_access_log
WHERE tenant_id = p_tenant AND delivery_scope = 'student_record' AND course_id
= p_course; DELETE FROM public.audit_event WHERE tenant_id = p_tenant AND course_id
= p_course; DELETE FROM public.asset_delivery WHERE tenant_id = p_tenant AND
delivery_kind = 'student_record' AND course_id = p_course; DELETE FROM public.student_export_artifact
a WHERE EXISTS ( SELECT 1 FROM public.course_retention_purge_export s WHERE
s.tenant_id = p_tenant AND s.course_id = p_course AND s.generation = p_generation
AND s.stage = p_stage AND s.export_id = a.export_id ); DELETE FROM public.student_export_request
WHERE tenant_id = p_tenant AND course_id = p_course; DELETE FROM public.worker_job
w WHERE w.tenant_id = p_tenant AND EXISTS ( SELECT 1 FROM public.course_retention_purge_export
s WHERE s.tenant_id = p_tenant AND s.course_id = p_course AND s.generation =
p_generation AND s.stage = p_stage AND s.job_id = w.job_id ); DELETE FROM public.assignment_individual_policy_exception
WHERE tenant_id = p_tenant AND course_id = p_course AND student_id IS NOT NULL;
DELETE FROM public.course_group_member WHERE tenant_id = p_tenant AND course_id
= p_course; DELETE FROM public.course_member WHERE tenant_id = p_tenant AND
course_id = p_course AND role = 'student'; IF frozen_assignment_disposition
= 'delete' THEN DELETE FROM public.assignment_item ap USING public.assignment
a WHERE ap.tenant_id = a.tenant_id AND ap.assignment_id = a.assignment_id AND
a.tenant_id = p_tenant AND a.course_id = p_course; DELETE FROM public.assignment
a WHERE a.tenant_id = p_tenant AND a.course_id = p_course; END IF; IF EXISTS
( SELECT 1 FROM public.feedback_release fr WHERE fr.tenant_id = p_tenant AND
EXISTS ( SELECT 1 FROM public.course_retention_purge_attempt s WHERE s.tenant_id
= p_tenant AND s.course_id = p_course AND s.generation = p_generation AND s.stage
= p_stage AND s.attempt_id = fr.attempt_id ) UNION ALL SELECT 1 FROM public.submission_receipt_snapshot
srs WHERE srs.tenant_id = p_tenant AND EXISTS ( SELECT 1 FROM public.course_retention_purge_attempt
s WHERE s.tenant_id = p_tenant AND s.course_id = p_course AND s.generation =
p_generation AND s.stage = p_stage AND s.attempt_id = srs.attempt_id ) UNION
ALL SELECT 1 FROM public.submission_next_attempt sna WHERE sna.tenant_id = p_tenant
AND ( EXISTS ( SELECT 1 FROM public.course_retention_purge_attempt s WHERE s.tenant_id
= p_tenant AND s.course_id = p_course AND s.generation = p_generation AND s.stage
= p_stage AND s.attempt_id = sna.predecessor_attempt_id ) OR EXISTS ( SELECT
1 FROM public.course_retention_purge_attempt s WHERE s.tenant_id = p_tenant
AND s.course_id = p_course AND s.generation = p_generation AND s.stage = p_stage
AND s.attempt_id = sna.next_attempt_id ) ) UNION ALL SELECT 1 FROM public.question_statistics_contribution_receipt
qsr WHERE qsr.tenant_id = p_tenant AND ( EXISTS ( SELECT 1 FROM public.course_retention_purge_run
s WHERE s.tenant_id = p_tenant AND s.course_id = p_course AND s.generation =
p_generation AND s.stage = p_stage AND s.run_id = qsr.first_completed_run_id
) OR EXISTS ( SELECT 1 FROM public.course_retention_purge_attempt s WHERE s.tenant_id
= p_tenant AND s.course_id = p_course AND s.generation = p_generation AND s.stage
= p_stage AND s.attempt_id = qsr.attempt_id ) ) UNION ALL SELECT 1 FROM public.question_prefetch
qp WHERE qp.tenant_id = p_tenant AND ( EXISTS ( SELECT 1 FROM public.course_retention_purge_run
s WHERE s.tenant_id = p_tenant AND s.course_id = p_course AND s.generation =
p_generation AND s.stage = p_stage AND s.run_id = qp.run_id ) OR EXISTS ( SELECT
1 FROM public.course_retention_purge_attempt s WHERE s.tenant_id = p_tenant
AND s.course_id = p_course AND s.generation = p_generation AND s.stage = p_stage
AND s.attempt_id = qp.predecessor_attempt_id ) ) UNION ALL SELECT 1 FROM public.external_tool_launch_session
e WHERE e.tenant_id = p_tenant AND e.course_id = p_course UNION ALL SELECT 1
FROM public.external_tool_exchange e WHERE e.tenant_id = p_tenant AND e.course_id
= p_course UNION ALL SELECT 1 FROM public.attempt_feedback af WHERE af.tenant_id
= p_tenant AND af.course_id = p_course UNION ALL SELECT 1 FROM public.submission
s WHERE s.tenant_id = p_tenant AND s.course_id = p_course UNION ALL SELECT 1
FROM public.manual_grade_receipt receipt WHERE receipt.tenant_id = p_tenant
AND receipt.course_id = p_course UNION ALL SELECT 1 FROM public.submission_evaluation
ge WHERE ge.tenant_id = p_tenant AND ge.course_id = p_course UNION ALL SELECT
1 FROM public.attempt_score_current score WHERE score.tenant_id = p_tenant AND
score.course_id = p_course UNION ALL SELECT 1 FROM public.submission_idempotency
si WHERE si.tenant_id = p_tenant AND si.course_id = p_course UNION ALL SELECT
1 FROM public.question_attempt qa WHERE qa.tenant_id = p_tenant AND qa.course_id
= p_course UNION ALL SELECT 1 FROM public.course_item_analysis_current analysis
WHERE analysis.tenant_id = p_tenant AND analysis.course_id = p_course UNION
ALL SELECT 1 FROM public.course_item_analysis_staging analysis WHERE analysis.tenant_id
= p_tenant AND analysis.course_id = p_course UNION ALL SELECT 1 FROM public.attempt_effective_policy_receipt
receipt WHERE receipt.tenant_id = p_tenant AND receipt.course_id = p_course
UNION ALL SELECT 1 FROM public.attempt_effective_policy_receipt_field_source
source JOIN public.attempt_effective_policy_receipt receipt ON receipt.tenant_id
= source.tenant_id AND receipt.attempt_id = source.attempt_id AND receipt.receipt_generation
= source.receipt_generation WHERE receipt.tenant_id = p_tenant AND receipt.course_id
= p_course UNION ALL SELECT 1 FROM public.worker_job w WHERE w.tenant_id = p_tenant
AND w.payload ->> 'kind' = 'recalculateCourseItemAnalysis' AND EXISTS ( SELECT
1 FROM public.assignment a WHERE a.tenant_id = w.tenant_id AND a.assignment_id
= (w.payload ->> 'assignment')::uuid AND a.course_id = p_course ) UNION ALL
SELECT 1 FROM public.attempt_effective_policy_current timing WHERE timing.tenant_id
= p_tenant AND timing.course_id = p_course UNION ALL SELECT 1 FROM public.worker_job
w WHERE w.tenant_id = p_tenant AND w.payload ->> 'kind' = 'autoSubmitAttempt'
AND EXISTS ( SELECT 1 FROM public.course_retention_purge_attempt purge WHERE
purge.tenant_id = p_tenant AND purge.course_id = p_course AND purge.generation
= p_generation AND purge.stage = p_stage AND purge.attempt_id = (w.payload ->>
'attempt')::uuid ) UNION ALL SELECT 1 FROM public.student_assignment_summary
sas WHERE EXISTS ( SELECT 1 FROM public.enrollment e JOIN public.assignment
a ON a.tenant_id = e.tenant_id AND a.assignment_id = e.assignment_id WHERE e.tenant_id
= sas.tenant_id AND sas.tenant_id = p_tenant AND e.enrollment_id = sas.enrollment_id
AND a.course_id = p_course ) UNION ALL SELECT 1 FROM public.assignment_run ar
WHERE EXISTS ( SELECT 1 FROM public.enrollment e JOIN public.assignment a ON
a.tenant_id = e.tenant_id AND a.assignment_id = e.assignment_id WHERE e.tenant_id
= ar.tenant_id AND ar.tenant_id = p_tenant AND e.enrollment_id = ar.enrollment_id
AND a.course_id = p_course ) UNION ALL SELECT 1 FROM public.enrollment e WHERE
EXISTS ( SELECT 1 FROM public.assignment a WHERE a.tenant_id = e.tenant_id AND
a.assignment_id = e.assignment_id AND a.tenant_id = p_tenant AND a.course_id
= p_course ) UNION ALL SELECT 1 FROM public.record_access_log ae WHERE ae.tenant_id
= p_tenant AND ae.delivery_scope = 'student_record' AND ae.course_id = p_course
UNION ALL SELECT 1 FROM public.audit_event ae WHERE ae.tenant_id = p_tenant
AND ae.course_id = p_course UNION ALL SELECT 1 FROM public.asset_delivery ad
WHERE ad.tenant_id = p_tenant AND ad.delivery_kind = 'student_record' AND ad.course_id
= p_course UNION ALL SELECT 1 FROM public.student_export_artifact sae WHERE
EXISTS ( SELECT 1 FROM public.course_retention_purge_export s WHERE s.tenant_id
= p_tenant AND s.course_id = p_course AND s.generation = p_generation AND s.stage
= p_stage AND s.export_id = sae.export_id ) UNION ALL SELECT 1 FROM public.student_export_request
ser WHERE ser.tenant_id = p_tenant AND ser.course_id = p_course UNION ALL SELECT
1 FROM public.worker_job w WHERE w.tenant_id = p_tenant AND EXISTS ( SELECT
1 FROM public.course_retention_purge_export s WHERE s.tenant_id = p_tenant AND
s.course_id = p_course AND s.generation = p_generation AND s.stage = p_stage
AND s.job_id = w.job_id ) UNION ALL SELECT 1 FROM public.course_member cm WHERE
cm.tenant_id = p_tenant AND cm.course_id = p_course AND cm.role = 'student'
UNION ALL SELECT 1 FROM public.course_group_member cgm WHERE cgm.tenant_id =
p_tenant AND cgm.course_id = p_course UNION ALL SELECT 1 FROM public.assignment_individual_policy_exception
exception WHERE exception.tenant_id = p_tenant AND exception.course_id = p_course
AND exception.student_id IS NOT NULL UNION ALL SELECT 1 FROM public.assignment_item
ap WHERE frozen_assignment_disposition = 'delete' AND EXISTS ( SELECT 1 FROM
public.assignment a WHERE a.tenant_id = ap.tenant_id AND a.assignment_id = ap.assignment_id
AND a.course_id = p_course ) UNION ALL SELECT 1 FROM public.assignment a WHERE
frozen_assignment_disposition = 'delete' AND a.tenant_id = p_tenant AND a.course_id
= p_course ) THEN RAISE EXCEPTION 'delete retention commit left residual learner rows';
END IF; DELETE FROM public.course_retention_purge_export WHERE tenant_id = p_tenant
AND course_id = p_course AND generation = p_generation AND stage = p_stage;
DELETE FROM public.course_retention_purge_attempt WHERE tenant_id = p_tenant
AND course_id = p_course AND generation = p_generation AND stage = p_stage;
DELETE FROM public.course_retention_purge_run WHERE tenant_id = p_tenant AND
course_id = p_course AND generation = p_generation AND stage = p_stage; UPDATE
public.course_retention_cleanup_manifest SET state = 'completed', completed_at
= transaction_timestamp() WHERE tenant_id = p_tenant AND course_id = p_course
AND generation = p_generation AND stage = p_stage AND job_id = p_job AND state
= 'prepared'; IF NOT FOUND THEN RAISE EXCEPTION 'failed to finalize delete manifest';
END IF; UPDATE public.course_retention_stage SET state = 'completed' WHERE tenant_id
= p_tenant AND course_id = p_course AND stage = p_stage AND generation = p_generation
AND state = 'started' AND job_id = p_job AND lease_token = p_token; IF NOT FOUND
THEN RAISE EXCEPTION 'failed to complete delete retention stage'; END IF; UPDATE
public.course_retention SET lifecycle = 'deleted' WHERE tenant_id = p_tenant
AND course_id = p_course AND generation = p_generation AND lifecycle = 'archived';
IF NOT FOUND THEN RAISE EXCEPTION 'failed to mark retention lifecycle deleted';
END IF; UPDATE public.worker_job SET state = 'completed', lease_token = NULL,
lease_expires_at = NULL, completed_at = transaction_timestamp() WHERE job_id
= p_job AND tenant_id = p_tenant AND state = 'leased' AND lease_token = p_token
AND payload = jsonb_build_object('kind', 'retention', 'course', p_course::text,
'stage', p_stage, 'generation', p_generation); IF NOT FOUND THEN RAISE EXCEPTION
'failed to mark delete worker job complete'; END IF; RETURN true; END $$;
ALTER FUNCTION public.ple_commit_delete_retention_work_before_passwordless_identity(
    uuid, uuid, uuid, uuid, text, bigint
) OWNER TO ple_retention_broker;
REVOKE ALL ON FUNCTION public.ple_commit_delete_retention_work_before_passwordless_identity(
    uuid, uuid, uuid, uuid, text, bigint
) FROM PUBLIC;
DROP TRIGGER assignment_policy_exception_retention_fence ON public.assignment_policy_exception;
DROP POLICY retention_broker_assignment_policy_exception_tenant_delete
    ON public.assignment_policy_exception;
DROP POLICY retention_broker_assignment_policy_exception_tenant_select
    ON public.assignment_policy_exception;
DROP TABLE public.assignment_policy_exception;

DROP TRIGGER attempt_timing_current_retention_fence ON public.attempt_timing_current;
DROP POLICY attempt_timing_current_tenant ON public.attempt_timing_current;
DROP POLICY ret_broker_attempt_timing_del ON public.attempt_timing_current;
DROP POLICY ret_broker_attempt_timing_sel ON public.attempt_timing_current;
DROP TABLE public.attempt_timing_current;

ALTER TABLE public.assignment
    DROP CONSTRAINT assignment_schedule_check,
    DROP CONSTRAINT assignment_late_policy_check,
    DROP CONSTRAINT assignment_time_limit_check,
    DROP CONSTRAINT assignment_attempt_limit_check,
    DROP COLUMN visible,
    DROP COLUMN available_at,
    DROP COLUMN due_at,
    DROP COLUMN closes_at,
    DROP COLUMN late_submission_policy,
    DROP COLUMN time_limit_seconds,
    DROP COLUMN auto_submit,
    DROP COLUMN attempt_limit;

ALTER TABLE public.assignment_effective_policy_base ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.assignment_effective_policy_base FORCE ROW LEVEL SECURITY;
ALTER TABLE public.assignment_group_schedule_offset ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.assignment_group_schedule_offset FORCE ROW LEVEL SECURITY;
ALTER TABLE public.assignment_group_accommodation ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.assignment_group_accommodation FORCE ROW LEVEL SECURITY;
ALTER TABLE public.assignment_individual_policy_exception ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.assignment_individual_policy_exception FORCE ROW LEVEL SECURITY;
ALTER TABLE public.attempt_effective_policy_receipt ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.attempt_effective_policy_receipt FORCE ROW LEVEL SECURITY;
ALTER TABLE public.attempt_effective_policy_receipt_field_source ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.attempt_effective_policy_receipt_field_source FORCE ROW LEVEL SECURITY;
ALTER TABLE public.attempt_effective_policy_current ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.attempt_effective_policy_current FORCE ROW LEVEL SECURITY;

CREATE POLICY assignment_effective_policy_base_tenant ON public.assignment_effective_policy_base
    TO ple_app USING (tenant_id = public.ple_current_tenant())
    WITH CHECK (tenant_id = public.ple_current_tenant());
CREATE POLICY assignment_group_schedule_offset_tenant ON public.assignment_group_schedule_offset
    TO ple_app USING (tenant_id = public.ple_current_tenant())
    WITH CHECK (tenant_id = public.ple_current_tenant());
CREATE POLICY assignment_group_accommodation_tenant ON public.assignment_group_accommodation
    TO ple_app USING (tenant_id = public.ple_current_tenant())
    WITH CHECK (tenant_id = public.ple_current_tenant());
CREATE POLICY assignment_individual_policy_exception_tenant
    ON public.assignment_individual_policy_exception
    TO ple_app USING (tenant_id = public.ple_current_tenant())
    WITH CHECK (tenant_id = public.ple_current_tenant());
CREATE POLICY attempt_effective_policy_receipt_tenant ON public.attempt_effective_policy_receipt
    TO ple_app USING (tenant_id = public.ple_current_tenant())
    WITH CHECK (tenant_id = public.ple_current_tenant());
CREATE POLICY attempt_effective_policy_receipt_field_source_tenant
    ON public.attempt_effective_policy_receipt_field_source TO ple_app
    USING (tenant_id = public.ple_current_tenant())
    WITH CHECK (tenant_id = public.ple_current_tenant());
CREATE POLICY attempt_effective_policy_current_tenant ON public.attempt_effective_policy_current
    TO ple_app USING (tenant_id = public.ple_current_tenant())
    WITH CHECK (tenant_id = public.ple_current_tenant());
CREATE POLICY attempt_effective_policy_current_queue ON public.attempt_effective_policy_current
    FOR SELECT TO ple_queue_broker USING (tenant_id = public.ple_current_tenant());
CREATE POLICY attempt_effective_policy_receipt_retention_select
    ON public.attempt_effective_policy_receipt FOR SELECT TO ple_retention_broker
    USING (tenant_id = public.ple_current_tenant());
CREATE POLICY attempt_effective_policy_receipt_retention_delete
    ON public.attempt_effective_policy_receipt FOR DELETE TO ple_retention_broker
    USING (tenant_id = public.ple_current_tenant());
CREATE POLICY attempt_effective_policy_receipt_field_source_retention_select
    ON public.attempt_effective_policy_receipt_field_source FOR SELECT TO ple_retention_broker
    USING (tenant_id = public.ple_current_tenant());
CREATE POLICY attempt_effective_policy_receipt_field_source_retention_delete
    ON public.attempt_effective_policy_receipt_field_source FOR DELETE TO ple_retention_broker
    USING (tenant_id = public.ple_current_tenant());
CREATE POLICY attempt_effective_policy_current_retention_select
    ON public.attempt_effective_policy_current FOR SELECT TO ple_retention_broker
    USING (tenant_id = public.ple_current_tenant());
CREATE POLICY attempt_effective_policy_current_retention_delete
    ON public.attempt_effective_policy_current FOR DELETE TO ple_retention_broker
    USING (tenant_id = public.ple_current_tenant());

GRANT SELECT, INSERT, UPDATE, DELETE ON public.assignment_effective_policy_base,
    public.assignment_group_schedule_offset, public.assignment_group_accommodation,
    public.assignment_individual_policy_exception,
    public.attempt_effective_policy_current TO ple_app;
GRANT SELECT, INSERT, UPDATE (sealed_at) ON public.attempt_effective_policy_receipt TO ple_app;
GRANT SELECT, INSERT ON public.attempt_effective_policy_receipt_field_source TO ple_app;
GRANT SELECT, DELETE ON public.attempt_effective_policy_receipt,
    public.attempt_effective_policy_receipt_field_source, public.attempt_effective_policy_current
    TO ple_retention_broker;
GRANT SELECT ON public.attempt_effective_policy_current TO ple_queue_broker;

COMMIT;
