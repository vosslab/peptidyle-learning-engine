-- Forward migration: bind every issued presentation and retain bounded,
-- answer-free WeBWorK replay state for one-call grading.

ALTER TABLE public.question_attempt
    ADD COLUMN presentation_descriptor_version smallint,
    ADD COLUMN presentation_nonce bytea,
    ADD COLUMN presentation_digest bytea,
    ADD CONSTRAINT question_attempt_presentation_all_or_none_check CHECK (
        (presentation_descriptor_version IS NULL
         AND presentation_nonce IS NULL
         AND presentation_digest IS NULL)
        OR
        (presentation_descriptor_version IS NOT NULL
         AND presentation_nonce IS NOT NULL
         AND presentation_digest IS NOT NULL)
    ),
    ADD CONSTRAINT question_attempt_presentation_version_check CHECK (
        presentation_descriptor_version IS NULL
        OR presentation_descriptor_version = 1
    ),
    ADD CONSTRAINT question_attempt_presentation_nonce_check CHECK (
        presentation_nonce IS NULL OR octet_length(presentation_nonce) = 16
    ),
    ADD CONSTRAINT question_attempt_presentation_digest_check CHECK (
        presentation_digest IS NULL OR octet_length(presentation_digest) = 32
    );

-- The release maintenance gate requires zero prefetch rows before applying
-- this migration, so no default or synthetic backfill is permitted here.
ALTER TABLE public.question_prefetch
    ADD COLUMN presentation_descriptor_version smallint NOT NULL,
    ADD COLUMN presentation_nonce bytea NOT NULL,
    ADD COLUMN presentation_digest bytea NOT NULL,
    ADD CONSTRAINT question_prefetch_presentation_version_check CHECK (
        presentation_descriptor_version = 1
    ),
    ADD CONSTRAINT question_prefetch_presentation_nonce_check CHECK (
        octet_length(presentation_nonce) = 16
    ),
    ADD CONSTRAINT question_prefetch_presentation_digest_check CHECK (
        octet_length(presentation_digest) = 32
    );

ALTER TABLE public.submission_idempotency
    RENAME COLUMN response_sha256 TO request_sha256;

ALTER TABLE public.submission_idempotency
    ADD COLUMN request_contract_version smallint DEFAULT 0 NOT NULL,
    ADD CONSTRAINT submission_idempotency_request_contract_version_check CHECK (
        request_contract_version IN (0, 1)
    );

-- Historical rows are now explicitly contract 0. New writers must always
-- provide their contract version, so an omitted value fails closed.
ALTER TABLE public.submission_idempotency
    ALTER COLUMN request_contract_version DROP DEFAULT;

CREATE TABLE public.webwork_grade_replay_state (
    tenant_id uuid NOT NULL,
    attempt_id uuid NOT NULL,
    attempt_occurred_at timestamp with time zone NOT NULL,
    course_id uuid NOT NULL,
    problem_id uuid NOT NULL,
    version_id uuid NOT NULL,
    source_object_id uuid NOT NULL,
    source_sha256 character(64) NOT NULL,
    seed numeric(20,0) NOT NULL,
    renderer_id text NOT NULL,
    renderer_version text NOT NULL,
    presentation_digest bytea NOT NULL,
    state_version smallint NOT NULL,
    mapping jsonb NOT NULL,
    mapping_sha256 character(64) NOT NULL,
    created_at timestamp with time zone DEFAULT transaction_timestamp() NOT NULL,
    CONSTRAINT webwork_grade_replay_state_state_version_check CHECK (state_version = 1),
    CONSTRAINT webwork_grade_replay_state_seed_check CHECK (
        seed >= 0 AND seed <= 18446744073709551615
    ),
    CONSTRAINT webwork_grade_replay_state_source_sha256_check CHECK (
        source_sha256 ~ '^[0-9a-f]{64}$'
    ),
    CONSTRAINT webwork_grade_replay_state_renderer_id_check CHECK (
        octet_length(renderer_id) BETWEEN 1 AND 128
    ),
    CONSTRAINT webwork_grade_replay_state_renderer_version_check CHECK (
        octet_length(renderer_version) BETWEEN 1 AND 128
    ),
    CONSTRAINT webwork_grade_replay_state_presentation_digest_check CHECK (
        octet_length(presentation_digest) = 32
    ),
    CONSTRAINT webwork_grade_replay_state_mapping_check CHECK (
        jsonb_typeof(mapping) = 'object'
        AND jsonb_typeof(mapping -> 'items') = 'array'
        AND jsonb_array_length(mapping -> 'items') BETWEEN 1 AND 32
        AND octet_length(mapping::text) <= 32768
    ),
    CONSTRAINT webwork_grade_replay_state_mapping_sha256_check CHECK (
        mapping_sha256 ~ '^[0-9a-f]{64}$'
    )
);

ALTER TABLE ONLY public.webwork_grade_replay_state
    ADD CONSTRAINT webwork_grade_replay_state_pkey PRIMARY KEY (
        tenant_id,
        attempt_id,
        attempt_occurred_at
    );

ALTER TABLE ONLY public.webwork_grade_replay_state
    ADD CONSTRAINT webwork_grade_replay_state_attempt_fk FOREIGN KEY (
        tenant_id,
        attempt_id,
        attempt_occurred_at
    ) REFERENCES public.question_attempt (
        tenant_id,
        attempt_id,
        occurred_at
    ) ON DELETE CASCADE;

ALTER TABLE public.webwork_grade_replay_state
    ADD CONSTRAINT webwork_grade_replay_state_course_fk FOREIGN KEY (
        tenant_id,
        course_id
    ) REFERENCES public.course (tenant_id, course_id),
    ADD CONSTRAINT webwork_grade_replay_state_version_fk FOREIGN KEY (
        problem_id,
        version_id
    ) REFERENCES public.problem_version (problem_id, version_id) ON DELETE RESTRICT;

CREATE INDEX webwork_grade_replay_state_retention_idx
    ON public.webwork_grade_replay_state (tenant_id, course_id, created_at);

ALTER TABLE public.webwork_grade_replay_state ENABLE ROW LEVEL SECURITY;
ALTER TABLE ONLY public.webwork_grade_replay_state FORCE ROW LEVEL SECURITY;

CREATE POLICY webwork_grade_replay_state_tenant ON public.webwork_grade_replay_state
    USING (
        tenant_id = public.ple_current_tenant()
        AND public.ple_course_records_accessible(tenant_id, course_id)
    )
    WITH CHECK (
        tenant_id = public.ple_current_tenant()
        AND public.ple_course_records_accessible(tenant_id, course_id)
    );

CREATE POLICY retention_broker_webwork_grade_replay_state_select
    ON public.webwork_grade_replay_state FOR SELECT TO ple_retention_broker
    USING (tenant_id = public.ple_current_tenant());

CREATE POLICY retention_broker_webwork_grade_replay_state_delete
    ON public.webwork_grade_replay_state FOR DELETE TO ple_retention_broker
    USING (tenant_id = public.ple_current_tenant());

-- Extend the accepted learner-record fence with the new attempt-bound table.
-- The body is intentionally restated in this forward migration: accepted
-- migrations remain immutable, and PostgreSQL trigger functions have no
-- additive branch-alter syntax.
CREATE OR REPLACE FUNCTION public.ple_fence_learner_record_write() RETURNS trigger
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'pg_catalog', 'public'
    AS $$
DECLARE
    owner_tenant uuid;
    owner_course uuid;
    related_course uuid;
    record_table text;
BEGIN
    record_table := CASE
        WHEN TG_TABLE_NAME ~ '^question_attempt_[0-9]{4}_[0-9]{2}$'
             OR TG_TABLE_NAME = 'question_attempt_default'
            THEN 'question_attempt'
        WHEN TG_TABLE_NAME ~ '^submission_[0-9]{4}_[0-9]{2}$'
             OR TG_TABLE_NAME = 'submission_default'
            THEN 'submission'
        WHEN TG_TABLE_NAME ~ '^submission_idempotency_p[0-9]+$'
            THEN 'submission_idempotency'
        WHEN TG_TABLE_NAME ~ '^record_access_log_[0-9]{4}_[0-9]{2}$'
             OR TG_TABLE_NAME = 'record_access_log_default'
            THEN 'record_access_log'
        WHEN TG_TABLE_NAME ~ '^audit_event_[0-9]{4}_[0-9]{2}$'
             OR TG_TABLE_NAME = 'audit_event_default'
            THEN 'audit_event'
        ELSE TG_TABLE_NAME
    END;

    IF record_table IN (
        'question_attempt',
        'submission',
        'submission_evaluation',
        'manual_grade_receipt',
        'attempt_score_current',
        'attempt_timing_current',
        'submission_idempotency',
        'attempt_feedback',
        'external_tool_exchange',
        'external_tool_launch_session',
        'webwork_grade_replay_state',
        'student_export_request',
        'course_group_member',
        'assignment_policy_exception',
        'course_item_analysis_current',
        'course_item_analysis_staging'
    ) THEN
        owner_tenant := NEW.tenant_id;
        owner_course := NEW.course_id;
        IF TG_OP = 'UPDATE' THEN
            IF OLD.course_id IS DISTINCT FROM NEW.course_id THEN
                RAISE EXCEPTION 'learner record course ownership is immutable'
                    USING ERRCODE = '22023';
            END IF;
        END IF;
    ELSIF record_table = 'asset_delivery' THEN
        IF NEW.delivery_kind <> 'student_record' THEN
            RETURN NEW;
        END IF;
        owner_tenant := NEW.tenant_id;
        owner_course := NEW.course_id;
        IF TG_OP = 'UPDATE' THEN
            IF OLD.course_id IS DISTINCT FROM NEW.course_id THEN
                RAISE EXCEPTION 'student delivery course ownership is immutable'
                    USING ERRCODE = '22023';
            END IF;
        END IF;
    ELSIF record_table = 'record_access_log' THEN
        IF NEW.delivery_scope <> 'student_record' THEN
            RETURN NEW;
        END IF;
        owner_tenant := NEW.tenant_id;
        owner_course := NEW.course_id;
        IF TG_OP = 'UPDATE' THEN
            IF OLD.course_id IS DISTINCT FROM NEW.course_id THEN
                RAISE EXCEPTION 'student audit course ownership is immutable'
                    USING ERRCODE = '22023';
            END IF;
        END IF;
    ELSIF record_table = 'audit_event' THEN
        IF NEW.course_id IS NULL THEN
            RETURN NEW;
        END IF;
        owner_tenant := NEW.tenant_id;
        owner_course := NEW.course_id;
        IF TG_OP = 'UPDATE' AND OLD.course_id IS DISTINCT FROM NEW.course_id THEN
            RAISE EXCEPTION 'course audit ownership is immutable'
                USING ERRCODE = '22023';
        END IF;
    ELSIF record_table = 'course_member' THEN
        IF TG_OP = 'INSERT' AND NEW.role <> 'student' THEN
            RETURN NEW;
        END IF;
        IF TG_OP = 'UPDATE' THEN
            IF OLD.role <> 'student' AND NEW.role <> 'student' THEN
                RETURN NEW;
            END IF;
        END IF;
        owner_tenant := NEW.tenant_id;
        owner_course := NEW.course_id;
    ELSIF record_table IN ('assignment_item', 'enrollment') THEN
        SELECT a.tenant_id, a.course_id
          INTO owner_tenant, owner_course
          FROM public.assignment a
         WHERE a.tenant_id = NEW.tenant_id
           AND a.assignment_id = NEW.assignment_id;
    ELSIF record_table IN ('student_assignment_summary', 'assignment_run') THEN
        SELECT a.tenant_id, a.course_id
          INTO owner_tenant, owner_course
          FROM public.enrollment e
          JOIN public.assignment a
            ON a.tenant_id = e.tenant_id
           AND a.assignment_id = e.assignment_id
         WHERE e.tenant_id = NEW.tenant_id
           AND e.enrollment_id = NEW.enrollment_id;
    ELSIF record_table = 'assignment_run_item' THEN
        SELECT a.tenant_id, a.course_id
          INTO owner_tenant, owner_course
          FROM public.assignment_run ar
          JOIN public.enrollment e
            ON e.tenant_id = ar.tenant_id
           AND e.enrollment_id = ar.enrollment_id
          JOIN public.assignment a
            ON a.tenant_id = e.tenant_id
           AND a.assignment_id = e.assignment_id
         WHERE ar.tenant_id = NEW.tenant_id
           AND ar.run_id = NEW.run_id;
    ELSIF record_table = 'feedback_release' THEN
        SELECT af.tenant_id, af.course_id
          INTO owner_tenant, owner_course
          FROM public.attempt_feedback af
         WHERE af.tenant_id = NEW.tenant_id
           AND af.attempt_id = NEW.attempt_id;
    ELSIF record_table = 'submission_receipt_snapshot' THEN
        SELECT si.tenant_id, si.course_id
          INTO owner_tenant, owner_course
          FROM public.submission_idempotency si
         WHERE si.tenant_id = NEW.tenant_id
           AND si.attempt_id = NEW.attempt_id;
    ELSIF record_table = 'submission_next_attempt' THEN
        SELECT si.tenant_id, si.course_id
          INTO owner_tenant, owner_course
          FROM public.submission_idempotency si
         WHERE si.tenant_id = NEW.tenant_id
           AND si.attempt_id = NEW.predecessor_attempt_id;
        IF NEW.next_attempt_id IS NOT NULL THEN
            SELECT qa.course_id
              INTO related_course
              FROM public.question_attempt qa
             WHERE qa.tenant_id = NEW.tenant_id
               AND qa.attempt_id = NEW.next_attempt_id;
            IF NOT FOUND OR related_course IS DISTINCT FROM owner_course THEN
                RAISE EXCEPTION 'successor attempt crosses a course boundary'
                    USING ERRCODE = '22023';
            END IF;
        END IF;
    ELSIF record_table = 'question_prefetch' THEN
        SELECT a.tenant_id, a.course_id
          INTO owner_tenant, owner_course
          FROM public.assignment_run ar
          JOIN public.enrollment e
            ON e.tenant_id = ar.tenant_id
           AND e.enrollment_id = ar.enrollment_id
          JOIN public.assignment a
            ON a.tenant_id = e.tenant_id
           AND a.assignment_id = e.assignment_id
         WHERE ar.tenant_id = NEW.tenant_id
           AND ar.run_id = NEW.run_id;
        SELECT qa.course_id
          INTO related_course
          FROM public.question_attempt qa
         WHERE qa.tenant_id = NEW.tenant_id
           AND qa.attempt_id = NEW.predecessor_attempt_id;
        IF NOT FOUND OR related_course IS DISTINCT FROM owner_course THEN
            RAISE EXCEPTION 'prefetch predecessor crosses a course boundary'
                USING ERRCODE = '22023';
        END IF;
    ELSIF record_table = 'question_statistics_contribution_receipt' THEN
        SELECT si.tenant_id, si.course_id
          INTO owner_tenant, owner_course
          FROM public.submission_idempotency si
         WHERE si.tenant_id = NEW.tenant_id
           AND si.attempt_id = NEW.attempt_id;
        SELECT a.course_id
          INTO related_course
          FROM public.assignment_run ar
          JOIN public.enrollment e
            ON e.tenant_id = ar.tenant_id
           AND e.enrollment_id = ar.enrollment_id
          JOIN public.assignment a
            ON a.tenant_id = e.tenant_id
           AND a.assignment_id = e.assignment_id
         WHERE ar.tenant_id = NEW.tenant_id
           AND ar.run_id = NEW.first_completed_run_id
           AND e.enrollment_id = NEW.enrollment_id;
        IF NOT FOUND OR related_course IS DISTINCT FROM owner_course THEN
            RAISE EXCEPTION 'statistics receipt crosses a course boundary'
                USING ERRCODE = '22023';
        END IF;
    ELSIF record_table = 'student_export_artifact' THEN
        SELECT r.tenant_id, r.course_id
          INTO owner_tenant, owner_course
          FROM public.student_export_request r
         WHERE r.export_id = NEW.export_id;
    ELSE
        RAISE EXCEPTION 'unsupported learner record fence table'
            USING ERRCODE = '22023';
    END IF;

    IF owner_tenant IS NULL
       OR owner_course IS NULL
       OR NOT public.ple_lock_course_write(owner_tenant, owner_course, false)
    THEN
        RAISE EXCEPTION 'learner record course is unavailable'
            USING ERRCODE = '23503';
    END IF;
    RETURN NEW;
END $$;

REVOKE ALL ON FUNCTION public.ple_fence_learner_record_write() FROM PUBLIC;

CREATE TRIGGER webwork_grade_replay_state_bind_course
    BEFORE INSERT ON public.webwork_grade_replay_state
    FOR EACH ROW EXECUTE FUNCTION public.ple_bind_course_from_attempt();

CREATE TRIGGER webwork_grade_replay_state_retention_fence
    BEFORE INSERT ON public.webwork_grade_replay_state
    FOR EACH ROW EXECUTE FUNCTION public.ple_fence_learner_record_write();

GRANT SELECT,INSERT,DELETE ON TABLE public.webwork_grade_replay_state TO ple_app;
GRANT SELECT,DELETE ON TABLE public.webwork_grade_replay_state TO ple_retention_broker;
