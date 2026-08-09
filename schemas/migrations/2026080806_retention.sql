-- Initial database epoch: retention.

CREATE FUNCTION public.ple_apply_retention_api_action(p_session character, p_course uuid, p_expected_generation bigint, p_action text, p_days integer DEFAULT NULL::integer, p_disposition text DEFAULT NULL::text) RETURNS text
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'pg_catalog', 'public'
    AS $$
DECLARE current record; replay record; next_generation bigint; immediate_stage text; next_disposition text; target_state text; actor uuid;
BEGIN
    IF p_expected_generation <= 0 OR p_action NOT IN ('extend','archive','delete') THEN RETURN NULL; END IF;
    IF p_action='extend' THEN
        IF NOT public.ple_retention_authorize(p_session, p_course, true) THEN
            RAISE EXCEPTION 'retention API operation forbidden' USING ERRCODE = '42501';
        END IF;
        IF p_days NOT BETWEEN 1 AND 36500 OR p_disposition IS NOT NULL THEN RETURN NULL; END IF;
    ELSIF NOT public.ple_retention_authorize(p_session, p_course, false) THEN
        RAISE EXCEPTION 'retention API operation forbidden' USING ERRCODE = '42501';
    ELSIF p_action='archive' AND (p_days IS NOT NULL OR p_disposition NOT IN ('retain','delete')) THEN
        RETURN NULL;
    ELSIF p_action='delete' AND (p_days IS NOT NULL OR p_disposition IS NOT NULL) THEN
        RETURN NULL;
    END IF;
    SELECT user_id INTO actor FROM public.auth_session
     WHERE session_hash=p_session AND tenant_id=public.ple_current_tenant()
       AND revoked_at IS NULL AND expires_at > transaction_timestamp();
    IF actor IS NULL THEN RAISE EXCEPTION 'retention API operation forbidden' USING ERRCODE = '42501'; END IF;
    -- This lock still serializes concurrent requests, but it deliberately
    -- includes archived rows so completed archive receipts remain replayable.
    SELECT * INTO current FROM public.course_retention
     WHERE tenant_id=public.ple_current_tenant() AND course_id=p_course FOR UPDATE;
    IF NOT FOUND THEN RETURN NULL; END IF;
    IF p_action IN ('archive','delete') THEN
        SELECT * INTO replay FROM public.course_retention_api_receipt
         WHERE tenant_id=public.ple_current_tenant() AND course_id=p_course
           AND (expected_generation=p_expected_generation
                OR resulting_generation=p_expected_generation)
         ORDER BY expected_generation DESC
         LIMIT 1;
        IF FOUND THEN
            IF replay.actor_id<>actor OR replay.action<>p_action
               OR replay.assignment_disposition IS DISTINCT FROM p_disposition THEN RETURN NULL; END IF;
            SELECT state INTO target_state FROM public.course_retention_stage
             WHERE tenant_id=replay.tenant_id AND course_id=replay.course_id
               AND generation=replay.resulting_generation AND stage=replay.stage;
            IF target_state='scheduled' THEN RETURN 'scheduled'; END IF;
            IF target_state='started' THEN RETURN 'inProgress'; END IF;
            IF target_state='completed' THEN RETURN 'completed'; END IF;
            RETURN NULL;
        END IF;
    END IF;
    IF current.lifecycle <> 'active' OR current.generation<>p_expected_generation THEN RETURN NULL; END IF;
    immediate_stage := CASE p_action WHEN 'archive' THEN 'archiveStudentRecords'
                                      WHEN 'delete' THEN 'deleteStudentRecords' END;
    IF immediate_stage IS NOT NULL THEN
        SELECT state INTO target_state FROM public.course_retention_stage s
         WHERE s.tenant_id=current.tenant_id AND s.course_id=current.course_id
           AND s.generation=current.generation AND s.stage=immediate_stage;
        IF target_state='started' THEN RETURN 'inProgress'; END IF;
        IF target_state='completed' THEN RETURN 'completed'; END IF;
        IF target_state <> 'scheduled' THEN RETURN NULL; END IF;
        IF EXISTS (
            SELECT 1 FROM public.course_retention_dispatch d
             WHERE d.tenant_id=current.tenant_id AND d.course_id=current.course_id
               AND d.generation=current.generation AND d.stage=immediate_stage
        ) THEN
            IF p_action='archive' AND current.assignment_disposition IS DISTINCT FROM p_disposition THEN
                RETURN NULL;
            END IF;
            RETURN 'scheduled';
        END IF;
    END IF;
    IF EXISTS (
        SELECT 1 FROM public.course_retention_stage s
         WHERE s.tenant_id=current.tenant_id AND s.course_id=current.course_id
           AND s.generation=current.generation AND s.state NOT IN ('scheduled','completed')
    ) THEN RETURN NULL; END IF;
    next_generation := current.generation + 1;
    next_disposition := CASE WHEN p_action='archive' THEN p_disposition ELSE current.assignment_disposition END;
    INSERT INTO public.course_retention_stage (tenant_id, course_id, stage, generation, due_at, state)
    SELECT s.tenant_id, s.course_id, s.stage, next_generation,
           CASE WHEN s.state='completed' THEN s.due_at
                WHEN s.stage=immediate_stage THEN transaction_timestamp()
                WHEN p_action='extend' THEN s.due_at + p_days * interval '1 day'
                ELSE s.due_at END,
           CASE WHEN s.state='completed' THEN 'completed' ELSE 'scheduled' END
     FROM public.course_retention_stage s
     WHERE s.tenant_id=current.tenant_id AND s.course_id=current.course_id
       AND s.generation=current.generation;
    UPDATE public.course_retention_stage SET state='superseded'
     WHERE tenant_id=current.tenant_id AND course_id=current.course_id
       AND generation=current.generation AND state='scheduled';
    UPDATE public.worker_job w SET state='dead', lease_token=NULL, lease_expires_at=NULL,
        completed_at=transaction_timestamp(), last_error='permanent'
      FROM public.course_retention_dispatch d
     WHERE d.tenant_id=current.tenant_id AND d.course_id=current.course_id
       AND d.generation=current.generation AND d.job_id=w.job_id AND w.state IN ('ready','leased');
    UPDATE public.course_retention SET generation=next_generation, assignment_disposition=next_disposition
     WHERE tenant_id=current.tenant_id AND course_id=current.course_id AND generation=current.generation;
    IF immediate_stage IS NOT NULL THEN
        WITH dispatch AS (
            INSERT INTO public.course_retention_dispatch
                (tenant_id, course_id, stage, generation, job_id, dispatched_at)
            VALUES (current.tenant_id, current.course_id, immediate_stage, next_generation,
                    gen_random_uuid(), transaction_timestamp())
            RETURNING job_id
        )
        INSERT INTO public.worker_job (job_id, tenant_id, payload, state, max_attempts)
        SELECT job_id, current.tenant_id,
               jsonb_build_object('kind','retention','course',current.course_id::text,
                                  'stage',immediate_stage,'generation',next_generation),
               'ready', 3 FROM dispatch;
        INSERT INTO public.course_retention_api_receipt
            (tenant_id, course_id, expected_generation, actor_id, action, assignment_disposition,
             resulting_generation, stage)
        VALUES (current.tenant_id, current.course_id, p_expected_generation, actor, p_action,
                p_disposition, next_generation, immediate_stage);
    END IF;
    RETURN CASE WHEN immediate_stage IS NULL THEN 'changed' ELSE 'scheduled' END;
END $$;

CREATE FUNCTION public.ple_bind_course_item_analysis_course() RETURNS trigger
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'pg_catalog', 'public'
    AS $$
DECLARE
    bound_course uuid;
BEGIN
    SELECT a.course_id
      INTO bound_course
      FROM public.assignment a
     WHERE a.tenant_id = NEW.tenant_id
       AND a.assignment_id = NEW.assignment_id;
    IF NOT FOUND THEN
        RAISE EXCEPTION 'item analysis has no assignment owner' USING ERRCODE = '23503';
    END IF;
    IF NEW.course_id IS NULL THEN
        NEW.course_id = bound_course;
    ELSIF NEW.course_id IS DISTINCT FROM bound_course THEN
        RAISE EXCEPTION 'item analysis course mismatch' USING ERRCODE = '22023';
    END IF;
    RETURN NEW;
END $$;

CREATE FUNCTION public.ple_bind_course_from_attempt() RETURNS trigger
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'pg_catalog', 'public'
    AS $$
DECLARE
    candidate_count bigint;
    candidate_courses uuid[];
BEGIN
    IF NEW.tenant_id IS NULL OR NEW.attempt_id IS NULL THEN
        RAISE EXCEPTION 'invalid learner attempt insertion' USING ERRCODE = '22023';
    END IF;

    SELECT COUNT(DISTINCT a.course_id), ARRAY_AGG(DISTINCT a.course_id ORDER BY a.course_id)
      INTO candidate_count, candidate_courses
      FROM public.question_attempt q
      JOIN public.assignment_run r
        ON r.tenant_id = q.tenant_id
       AND r.run_id = q.run_id
      JOIN public.enrollment e
        ON e.tenant_id = r.tenant_id
       AND e.enrollment_id = r.enrollment_id
      JOIN public.assignment a
        ON a.tenant_id = e.tenant_id
       AND a.assignment_id = e.assignment_id
     WHERE q.tenant_id = NEW.tenant_id
       AND q.attempt_id = NEW.attempt_id;
    IF candidate_count IS NULL OR candidate_count = 0 THEN
        RAISE EXCEPTION 'invalid learner attempt insertion' USING ERRCODE = '22023';
    END IF;
    IF candidate_count > 1 THEN
        RAISE EXCEPTION 'ambiguous learner attempt ownership' USING ERRCODE = '22023';
    END IF;
    IF NEW.course_id IS NOT NULL AND NEW.course_id IS DISTINCT FROM candidate_courses[1] THEN
        RAISE EXCEPTION 'payload course mismatch' USING ERRCODE = '22023';
    END IF;
    NEW.course_id := candidate_courses[1];
    RETURN NEW;
END $$;

CREATE FUNCTION public.ple_bind_question_attempt_course() RETURNS trigger
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'pg_catalog', 'public'
    AS $$
DECLARE
    candidate_count bigint;
    candidate_courses uuid[];
BEGIN
    IF NEW.tenant_id IS NULL OR NEW.run_id IS NULL THEN
        RAISE EXCEPTION 'invalid learner run insertion' USING ERRCODE = '22023';
    END IF;

    SELECT COUNT(DISTINCT a.course_id), ARRAY_AGG(DISTINCT a.course_id ORDER BY a.course_id)
      INTO candidate_count, candidate_courses
      FROM public.assignment_run r
      JOIN public.enrollment e
        ON e.tenant_id = r.tenant_id
       AND e.enrollment_id = r.enrollment_id
      JOIN public.assignment a
        ON a.tenant_id = e.tenant_id
       AND a.assignment_id = e.assignment_id
     WHERE r.tenant_id = NEW.tenant_id
       AND r.run_id = NEW.run_id;
    IF candidate_count IS NULL OR candidate_count = 0 THEN
        RAISE EXCEPTION 'invalid learner run insertion' USING ERRCODE = '22023';
    END IF;
    IF candidate_count > 1 THEN
        RAISE EXCEPTION 'ambiguous learner run ownership' USING ERRCODE = '22023';
    END IF;
    IF NEW.course_id IS NOT NULL AND NEW.course_id IS DISTINCT FROM candidate_courses[1] THEN
        RAISE EXCEPTION 'attempt payload course mismatch' USING ERRCODE = '22023';
    END IF;
    NEW.course_id := candidate_courses[1];
    RETURN NEW;
END $$;

CREATE FUNCTION public.ple_commit_retention_work(p_tenant uuid, p_job uuid, p_token uuid, p_course uuid, p_stage text, p_generation bigint) RETURNS boolean
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'pg_catalog', 'public'
    AS $$
BEGIN
    IF p_stage = 'deleteStudentRecords' THEN
        RETURN ple_commit_delete_retention_work(p_tenant, p_job, p_token, p_course, p_stage, p_generation);
    END IF;
    RETURN ple_commit_archive_retention_work(p_tenant, p_job, p_token, p_course, p_stage, p_generation);
END $$;

CREATE FUNCTION public.ple_configure_retention_policy(p_session character, p_notify integer, p_archive integer, p_delete integer) RETURNS boolean
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'pg_catalog', 'public'
    AS $$
BEGIN
    IF NOT public.ple_retention_authorize(p_session, NULL, true) THEN RETURN false; END IF;
    INSERT INTO public.institution_retention_policy (tenant_id, notify_days, archive_days, delete_days, assignment_disposition)
    VALUES (public.ple_current_tenant(), p_notify, p_archive, p_delete, 'retain')
    ON CONFLICT (tenant_id) DO UPDATE SET notify_days=EXCLUDED.notify_days, archive_days=EXCLUDED.archive_days, delete_days=EXCLUDED.delete_days;
    RETURN true;
END $$;

CREATE FUNCTION public.ple_course_records_accessible(p_tenant uuid, p_course uuid) RETURNS boolean
    LANGUAGE plpgsql STABLE SECURITY DEFINER
    SET search_path TO 'pg_catalog', 'public'
    AS $$
DECLARE
    current_generation bigint;
BEGIN
    IF p_tenant IS NULL OR p_tenant IS DISTINCT FROM public.ple_current_tenant() THEN
        RETURN false;
    END IF;
    IF NOT EXISTS (
        SELECT 1 FROM public.course
         WHERE tenant_id = p_tenant AND course_id = p_course
    ) THEN
        RETURN false;
    END IF;

    SELECT generation INTO current_generation
      FROM public.course_retention
     WHERE tenant_id = p_tenant AND course_id = p_course;

    IF EXISTS (
        SELECT 1 FROM public.course_retention
         WHERE tenant_id = p_tenant AND course_id = p_course
           AND lifecycle IN ('archived', 'deleted')
    ) THEN
        RETURN false;
    END IF;

    IF EXISTS (
        SELECT 1 FROM public.course_retention_stage
         WHERE tenant_id = p_tenant
           AND course_id = p_course
           AND generation = current_generation
           AND stage IN ('archiveStudentRecords', 'deleteStudentRecords')
           AND state = 'started'
    ) THEN
        RETURN false;
    END IF;

    RETURN NOT EXISTS (
        SELECT 1 FROM public.course_retention_stage
         WHERE tenant_id = p_tenant
           AND course_id = p_course
           AND generation = current_generation
           AND stage = 'archiveStudentRecords'
           AND state = 'started'
    );
END $$;

CREATE FUNCTION public.ple_dispatch_due_retention_stages(p_batch integer) RETURNS bigint
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'pg_catalog', 'public'
    AS $$
DECLARE dispatched bigint;
BEGIN
    IF p_batch NOT BETWEEN 1 AND 100 THEN
        RAISE EXCEPTION 'invalid retention dispatch batch' USING ERRCODE = '22023';
    END IF;
    WITH candidate AS (
        SELECT s.tenant_id, s.course_id, s.stage, s.generation
          FROM public.course_retention_stage s
          JOIN public.course_retention r
            ON r.tenant_id=s.tenant_id AND r.course_id=s.course_id AND r.generation=s.generation
          LEFT JOIN public.course_retention_dispatch d
            ON d.tenant_id=s.tenant_id AND d.course_id=s.course_id
           AND d.stage=s.stage AND d.generation=s.generation
         WHERE r.lifecycle='active' AND s.state='scheduled'
           AND s.due_at <= transaction_timestamp() AND d.job_id IS NULL
         ORDER BY s.due_at, s.tenant_id, s.course_id, s.stage
         FOR UPDATE OF s, r SKIP LOCKED
         LIMIT p_batch
    ), dispatch AS (
        INSERT INTO public.course_retention_dispatch
            (tenant_id, course_id, stage, generation, job_id, dispatched_at)
        SELECT tenant_id, course_id, stage, generation, gen_random_uuid(), transaction_timestamp()
          FROM candidate
        RETURNING tenant_id, course_id, stage, generation, job_id
    ), jobs AS (
        INSERT INTO public.worker_job (job_id, tenant_id, payload, state, max_attempts)
        SELECT job_id, tenant_id,
               jsonb_build_object('kind','retention','course',course_id::text,
                                  'stage',stage,'generation',generation),
               'ready', 3
          FROM dispatch
        RETURNING job_id
    )
    SELECT count(*) INTO dispatched FROM jobs;
    RETURN dispatched;
END $$;

CREATE FUNCTION public.ple_end_course_retention(p_session character, p_course uuid) RETURNS boolean
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'pg_catalog', 'public'
    AS $$
DECLARE policy record;
BEGIN
    IF NOT public.ple_retention_authorize(p_session, p_course, false) THEN RETURN false; END IF;
    SELECT * INTO policy FROM public.institution_retention_policy WHERE tenant_id = public.ple_current_tenant();
    INSERT INTO public.course_retention (tenant_id, course_id, ended_at, notify_days, archive_days, delete_days, assignment_disposition, generation)
    VALUES (public.ple_current_tenant(), p_course, transaction_timestamp(), COALESCE(policy.notify_days,30), COALESCE(policy.archive_days,100), COALESCE(policy.delete_days,365), 'retain', 1)
    ON CONFLICT (tenant_id, course_id) DO NOTHING;
    INSERT INTO public.course_retention_stage (tenant_id, course_id, stage, generation, due_at)
      SELECT r.tenant_id, r.course_id, s.stage, r.generation, r.ended_at + s.days * interval '1 day'
      FROM public.course_retention r CROSS JOIN LATERAL (VALUES ('notify', r.notify_days), ('archiveStudentRecords', r.archive_days), ('deleteStudentRecords', r.delete_days)) AS s(stage, days)
      WHERE r.tenant_id=public.ple_current_tenant() AND r.course_id=p_course
    ON CONFLICT DO NOTHING;
    RETURN true;
END $$;

CREATE FUNCTION public.ple_extend_course_retention(p_session character, p_course uuid, p_days integer) RETURNS boolean
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'pg_catalog', 'public'
    AS $$
DECLARE current record; next_generation bigint;
BEGIN
    IF NOT public.ple_retention_authorize(p_session, p_course, true) THEN
        RAISE EXCEPTION 'retention schedule operation forbidden' USING ERRCODE = '42501';
    END IF;
    IF p_days NOT BETWEEN 1 AND 36500 THEN RETURN false; END IF;
    SELECT * INTO current FROM public.course_retention
      WHERE tenant_id=public.ple_current_tenant() AND course_id=p_course AND lifecycle='active' FOR UPDATE;
    IF NOT FOUND OR EXISTS (
        SELECT 1 FROM public.course_retention_stage s
         WHERE s.tenant_id=current.tenant_id AND s.course_id=current.course_id
           AND s.generation=current.generation AND s.state='started'
    ) THEN RETURN false; END IF;
    IF EXISTS (
        SELECT 1 FROM public.course_retention_stage s
         WHERE s.tenant_id=current.tenant_id AND s.course_id=current.course_id
           AND s.generation=current.generation AND s.state NOT IN ('scheduled','completed')
    ) THEN RETURN false; END IF;
    next_generation := current.generation + 1;
    UPDATE public.course_retention_stage SET state='superseded'
      WHERE tenant_id=current.tenant_id AND course_id=current.course_id
        AND generation=current.generation AND state='scheduled';
    UPDATE public.worker_job w SET state='dead', lease_token=NULL, lease_expires_at=NULL,
        completed_at=transaction_timestamp(), last_error='permanent'
      FROM public.course_retention_dispatch d
      WHERE d.tenant_id=current.tenant_id AND d.course_id=current.course_id
        AND d.generation=current.generation AND w.job_id=d.job_id AND w.state IN ('ready','leased');
    INSERT INTO public.course_retention_stage (tenant_id, course_id, stage, generation, due_at, state)
      SELECT s.tenant_id, s.course_id, s.stage, next_generation,
             CASE WHEN s.state='completed' THEN s.due_at ELSE s.due_at + p_days * interval '1 day' END,
             CASE WHEN s.state='completed' THEN 'completed' ELSE 'scheduled' END
        FROM public.course_retention_stage s
       WHERE s.tenant_id=current.tenant_id AND s.course_id=current.course_id
         AND s.generation=current.generation
       ORDER BY s.stage;
    UPDATE public.course_retention SET generation=next_generation
      WHERE tenant_id=current.tenant_id AND course_id=current.course_id AND generation=current.generation;
    RETURN FOUND;
END $$;

CREATE FUNCTION public.ple_fence_assignment_definition_write() RETURNS trigger
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'pg_catalog', 'public'
    AS $$
DECLARE
    owner_tenant uuid;
    owner_course uuid;
BEGIN
    IF TG_TABLE_NAME = 'assignment' THEN
        owner_tenant := NEW.tenant_id;
        owner_course := NEW.course_id;
        IF TG_OP = 'UPDATE' THEN
            IF OLD.course_id IS DISTINCT FROM NEW.course_id THEN
                RAISE EXCEPTION 'assignment course ownership is immutable'
                    USING ERRCODE = '22023';
            END IF;
        END IF;
    ELSIF TG_TABLE_NAME = 'assignment_item' THEN
        SELECT a.tenant_id, a.course_id
          INTO owner_tenant, owner_course
          FROM public.assignment a
         WHERE a.tenant_id = NEW.tenant_id
           AND a.assignment_id = NEW.assignment_id;
    ELSE
        RAISE EXCEPTION 'unsupported assignment definition fence table'
            USING ERRCODE = '22023';
    END IF;

    IF owner_tenant IS NULL
       OR owner_course IS NULL
       OR NOT public.ple_lock_course_write(owner_tenant, owner_course, true)
    THEN
        RAISE EXCEPTION 'assignment definition course is unavailable'
            USING ERRCODE = '23503';
    END IF;
    RETURN NEW;
END $$;

CREATE FUNCTION public.ple_fence_learner_record_write() RETURNS trigger
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

CREATE FUNCTION public.ple_lock_course_write(p_tenant uuid, p_course uuid, p_definition boolean) RETURNS boolean
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'pg_catalog', 'public'
    AS $$
DECLARE
    current_generation bigint;
    current_lifecycle text;
    current_disposition text;
BEGIN
    IF p_tenant IS NULL
       OR p_course IS NULL
       OR p_tenant IS DISTINCT FROM public.ple_current_tenant()
       OR NOT EXISTS (
           SELECT 1
             FROM public.course c
            WHERE c.tenant_id = p_tenant
              AND c.course_id = p_course
       )
    THEN
        RETURN false;
    END IF;

    SELECT r.generation, r.lifecycle, r.assignment_disposition
      INTO current_generation, current_lifecycle, current_disposition
      FROM public.course_retention r
     WHERE r.tenant_id = p_tenant
       AND r.course_id = p_course
     FOR SHARE;
    IF NOT FOUND THEN
        RETURN true;
    END IF;

    IF p_definition THEN
        RETURN current_lifecycle = 'active' OR current_disposition = 'retain';
    END IF;
    IF current_lifecycle <> 'active' THEN
        RETURN false;
    END IF;

    RETURN NOT EXISTS (
        SELECT 1
          FROM public.course_retention_stage s
         WHERE s.tenant_id = p_tenant
           AND s.course_id = p_course
           AND s.generation = current_generation
           AND s.stage IN ('archiveStudentRecords', 'deleteStudentRecords')
           AND s.state = 'started'
    );
END $$;

CREATE FUNCTION public.ple_prepare_retention_work(p_tenant uuid, p_job uuid, p_token uuid, p_course uuid, p_stage text, p_generation bigint) RETURNS jsonb
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'pg_catalog', 'public'
    AS $$
BEGIN
    IF p_stage = 'deleteStudentRecords' THEN
        RETURN ple_prepare_delete_retention_work(p_tenant, p_job, p_token, p_course, p_stage, p_generation);
    END IF;
    RETURN ple_prepare_archive_retention_work(p_tenant, p_job, p_token, p_course, p_stage, p_generation);
END $$;

CREATE FUNCTION public.ple_read_course_retention(p_session character, p_course uuid) RETURNS TABLE(ended_at_millis bigint, notify_days integer, archive_days integer, delete_days integer, assignment_disposition text, generation bigint, lifecycle text)
    LANGUAGE sql SECURITY DEFINER
    SET search_path TO 'pg_catalog', 'public'
    AS $$
    SELECT floor(extract(epoch FROM r.ended_at) * 1000)::bigint, r.notify_days, r.archive_days, r.delete_days, r.assignment_disposition, r.generation, r.lifecycle
    FROM public.course_retention r
    WHERE public.ple_retention_authorize(p_session, p_course, false)
      AND r.tenant_id = public.ple_current_tenant() AND r.course_id = p_course
$$;

CREATE FUNCTION public.ple_read_retention_notification(p_session character, p_course uuid) RETURNS TABLE(intent text, created_at_millis bigint)
    LANGUAGE sql SECURITY DEFINER
    SET search_path TO 'pg_catalog', 'public'
    AS $$
    SELECT n.intent, floor(extract(epoch FROM n.created_at) * 1000)::bigint
      FROM public.course_retention r
      JOIN public.course_retention_notification n
        ON n.tenant_id=r.tenant_id AND n.course_id=r.course_id
     WHERE public.ple_retention_authorize(p_session, p_course, false)
       AND r.tenant_id=public.ple_current_tenant() AND r.course_id=p_course
     ORDER BY n.generation DESC, n.created_at DESC
     LIMIT 1
$$;

CREATE FUNCTION public.ple_retention_authorize(p_session character, p_course uuid DEFAULT NULL::uuid, p_admin_only boolean DEFAULT false) RETURNS boolean
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'pg_catalog', 'public'
    AS $$
DECLARE actor uuid; roles jsonb;
BEGIN
    PERFORM set_config('ple.session_hash', p_session, true);
    SELECT user_id, auth_session.roles INTO actor, roles FROM public.auth_session
     WHERE session_hash = p_session AND revoked_at IS NULL AND expires_at > transaction_timestamp();
    IF actor IS NULL OR NOT (SELECT tenant_id = public.ple_current_tenant() FROM public.auth_session WHERE session_hash = p_session) THEN RETURN false; END IF;
    IF p_course IS NOT NULL AND NOT EXISTS (SELECT 1 FROM public.course WHERE tenant_id = public.ple_current_tenant() AND course_id = p_course) THEN RETURN false; END IF;
    IF roles @> '["administrator"]'::jsonb THEN RETURN true; END IF;
    IF p_admin_only OR p_course IS NULL THEN RETURN false; END IF;
    RETURN EXISTS (SELECT 1 FROM public.course_member WHERE tenant_id = public.ple_current_tenant()
                   AND course_id = p_course AND user_id = actor AND role = 'instructor');
END $$;

CREATE FUNCTION public.ple_set_archive_disposition(p_session character, p_course uuid, p_disposition text) RETURNS boolean
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'pg_catalog', 'public'
    AS $$
DECLARE current record;
BEGIN
    IF NOT public.ple_retention_authorize(p_session, p_course, false) THEN
        RAISE EXCEPTION 'retention schedule operation forbidden' USING ERRCODE = '42501';
    END IF;
    IF p_disposition NOT IN ('retain','delete') THEN RETURN false; END IF;
    SELECT * INTO current FROM public.course_retention
      WHERE tenant_id=public.ple_current_tenant() AND course_id=p_course AND lifecycle='active' FOR UPDATE;
    IF NOT FOUND OR NOT EXISTS (
        SELECT 1 FROM public.course_retention_stage s
         WHERE s.tenant_id=current.tenant_id AND s.course_id=current.course_id
           AND s.generation=current.generation AND s.stage='archiveStudentRecords'
           AND s.state='scheduled'
    ) THEN RETURN false; END IF;
    UPDATE public.course_retention SET assignment_disposition=p_disposition
      WHERE tenant_id=current.tenant_id AND course_id=current.course_id AND generation=current.generation;
    RETURN FOUND;
END $$;

CREATE FUNCTION public.ple_commit_archive_retention_work(p_tenant uuid, p_job uuid, p_token uuid, p_course uuid, p_stage text, p_generation bigint) RETURNS boolean
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'pg_catalog', 'public'
    AS $$
DECLARE
    lifecycle text;
    prepared_count bigint;
    manifest_count bigint;
BEGIN
    IF p_tenant IS NULL
       OR p_course IS NULL
       OR p_tenant IS DISTINCT FROM public.ple_current_tenant()
       OR p_generation <= 0
       OR p_stage NOT IN ('notify', 'archiveStudentRecords', 'deleteStudentRecords') THEN
        RETURN false;
    END IF;

    PERFORM 1
      FROM public.worker_job w
      JOIN public.course_retention_dispatch d
        ON d.tenant_id = w.tenant_id AND d.course_id = p_course AND d.stage = p_stage
       AND d.generation = p_generation AND d.job_id = w.job_id
      JOIN public.course_retention r
        ON r.tenant_id = d.tenant_id AND r.course_id = d.course_id
       AND r.generation = d.generation
     WHERE w.job_id = p_job
       AND w.tenant_id = p_tenant
       AND w.state = 'leased'
       AND w.lease_token = p_token
       AND w.lease_expires_at > transaction_timestamp()
       AND w.payload = jsonb_build_object('kind', 'retention', 'course', p_course::text,
                                          'stage', p_stage, 'generation', p_generation)
     FOR UPDATE OF w, r;
    IF NOT FOUND THEN
        RETURN false;
    END IF;

    PERFORM 1
      FROM public.course_retention_stage s
     WHERE s.tenant_id = p_tenant
       AND s.course_id = p_course
       AND s.stage = p_stage
       AND s.generation = p_generation
       AND s.state = 'started'
       AND s.job_id = p_job
       AND s.lease_token = p_token
     FOR UPDATE;
    IF NOT FOUND THEN
        RETURN false;
    END IF;

    -- All required checks complete before mutation.
    IF p_stage = 'notify' THEN
        INSERT INTO public.course_retention_notification (tenant_id, course_id, generation, intent)
        VALUES (p_tenant,p_course,p_generation,'archive') ON CONFLICT DO NOTHING;
    ELSIF p_stage IN ('archiveStudentRecords', 'deleteStudentRecords') THEN
        SELECT m.object_count,
               COALESCE((
                   SELECT COUNT(*)
                     FROM public.course_retention_cleanup_manifest_object o
                    WHERE o.tenant_id = m.tenant_id
                      AND o.course_id = m.course_id
                      AND o.generation = m.generation
                      AND o.stage = m.stage
               ), 0)
          INTO prepared_count, manifest_count
          FROM public.course_retention_cleanup_manifest m
         WHERE m.tenant_id = p_tenant
           AND m.course_id = p_course
           AND m.generation = p_generation
           AND m.stage = p_stage
           AND m.job_id = p_job
           AND m.state = 'prepared'
         FOR UPDATE;
        IF NOT FOUND OR prepared_count IS NULL THEN
            RETURN false;
        END IF;

        IF manifest_count <> prepared_count THEN
            RETURN false;
        END IF;

        IF p_stage = 'archiveStudentRecords' THEN
            SELECT lifecycle INTO lifecycle
              FROM public.course_retention m
             WHERE m.tenant_id = p_tenant
               AND m.course_id = p_course
               AND m.generation = p_generation
               AND m.lifecycle = 'active'
             FOR UPDATE;
            IF NOT FOUND OR lifecycle <> 'active' THEN
                RETURN false;
            END IF;
        END IF;
    ELSE
        RETURN false;
    END IF;

    IF p_stage IN ('archiveStudentRecords', 'deleteStudentRecords') THEN
        UPDATE public.course_retention_cleanup_manifest
           SET state = 'completed', completed_at = transaction_timestamp()
         WHERE tenant_id = p_tenant
           AND course_id = p_course
           AND generation = p_generation
           AND stage = p_stage
           AND job_id = p_job
           AND state = 'prepared';
        IF NOT FOUND THEN
            RAISE EXCEPTION 'failed to finalize retention manifest for stage %', p_stage
                USING ERRCODE = '45000';
        END IF;
    END IF;

    UPDATE public.course_retention_stage SET state = 'completed'
      WHERE tenant_id = p_tenant
        AND course_id = p_course
        AND stage = p_stage
        AND generation = p_generation
        AND state = 'started'
        AND job_id = p_job
        AND lease_token = p_token;
    IF NOT FOUND THEN
        RAISE EXCEPTION 'failed to finalize retention stage for stage %', p_stage
            USING ERRCODE = '45000';
    END IF;

    IF p_stage = 'archiveStudentRecords' THEN
        UPDATE public.course_retention
           SET lifecycle = 'archived'
         WHERE tenant_id = p_tenant
           AND course_id = p_course
           AND generation = p_generation
           AND lifecycle = 'active';
        IF NOT FOUND THEN
            RAISE EXCEPTION 'failed to transition course retention lifecycle for stage %', p_stage
                USING ERRCODE = '45000';
        END IF;
    END IF;

    UPDATE public.worker_job
       SET state = 'completed', lease_token = NULL, lease_expires_at = NULL,
           completed_at = transaction_timestamp()
       WHERE job_id = p_job
       AND tenant_id = p_tenant
       AND state = 'leased'
       AND lease_token = p_token
       AND payload = jsonb_build_object('kind', 'retention', 'course', p_course::text,
                                      'stage', p_stage, 'generation', p_generation);
    IF NOT FOUND THEN
        RAISE EXCEPTION 'failed to complete retention worker job for stage %', p_stage
            USING ERRCODE = '45000';
    END IF;

    RETURN true;
END $$;

CREATE FUNCTION public.ple_prepare_archive_retention_work(p_tenant uuid, p_job uuid, p_token uuid, p_course uuid, p_stage text, p_generation bigint) RETURNS jsonb
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'pg_catalog', 'public'
    AS $$
DECLARE
    manifest jsonb;
    existing_object_count bigint;
    existing_object_rows bigint;
    existing_state text;
    existing_job uuid;
    object_count bigint;
    object_ids uuid[];
BEGIN
    IF p_tenant IS NULL OR p_tenant IS DISTINCT FROM public.ple_current_tenant()
       OR p_generation <= 0
       OR p_stage NOT IN ('notify', 'archiveStudentRecords', 'deleteStudentRecords') THEN
        RAISE EXCEPTION 'invalid retention worker capability' USING ERRCODE = '22023';
    END IF;

    IF p_stage = 'deleteStudentRecords' THEN
        RETURN NULL;
    END IF;

    -- Validate the exact scheduled worker binding before any rows can be touched.
    PERFORM 1 FROM public.worker_job w
      JOIN public.course_retention r
        ON r.tenant_id = w.tenant_id AND r.course_id = p_course
      JOIN public.course_retention_dispatch d
        ON d.tenant_id = w.tenant_id AND d.course_id = p_course AND d.stage = p_stage
       AND d.generation = p_generation AND d.job_id = w.job_id
     WHERE w.job_id = p_job
       AND w.tenant_id = p_tenant
       AND w.state = 'leased'
       AND w.lease_token = p_token
       AND w.lease_expires_at > transaction_timestamp()
       AND r.generation = p_generation
       AND w.payload = jsonb_build_object('kind','retention','course',p_course::text,
           'stage',p_stage,'generation',p_generation)
     FOR UPDATE OF w, r;
    IF NOT FOUND THEN
        RETURN NULL;
    END IF;

    -- Stage start must match the scheduled stage row or an already-bound lease.
    PERFORM 1 FROM public.course_retention_stage s
      WHERE s.tenant_id = p_tenant
        AND s.course_id = p_course
        AND s.stage = p_stage
        AND s.generation = p_generation
        AND s.due_at <= transaction_timestamp()
        AND (s.state = 'scheduled' OR (s.state = 'started' AND s.job_id = p_job))
      FOR UPDATE;
    IF NOT FOUND THEN
        RETURN NULL;
    END IF;

    IF p_stage = 'notify' THEN
        UPDATE public.course_retention_stage SET state = 'started', job_id = p_job,
            lease_token = p_token, claimed_at = transaction_timestamp()
          WHERE tenant_id = p_tenant AND course_id = p_course AND stage = p_stage
            AND generation = p_generation;
        IF NOT FOUND THEN
            RETURN NULL;
        END IF;
        RETURN jsonb_build_object('kind', 'notify');
    END IF;

    -- Lease renewals replay the previously derived manifest for the same job.
    SELECT
        m.state,
        m.job_id,
        m.object_count,
        COALESCE((
            SELECT COUNT(*)
              FROM public.course_retention_cleanup_manifest_object o
             WHERE o.tenant_id = m.tenant_id
               AND o.course_id = m.course_id
               AND o.generation = m.generation
               AND o.stage = m.stage
        ), 0)
      INTO existing_state, existing_job, existing_object_count, existing_object_rows
      FROM public.course_retention_cleanup_manifest m
      WHERE m.tenant_id = p_tenant
        AND m.course_id = p_course
        AND m.generation = p_generation
        AND m.stage = p_stage
      FOR UPDATE;
    IF FOUND THEN
        IF existing_state <> 'prepared' OR existing_job IS DISTINCT FROM p_job THEN
            RETURN NULL;
        END IF;
        IF existing_object_count IS DISTINCT FROM existing_object_rows THEN
            RETURN NULL;
        END IF;

        UPDATE public.course_retention_stage SET state = 'started', job_id = p_job,
            lease_token = p_token, claimed_at = transaction_timestamp()
          WHERE tenant_id = p_tenant AND course_id = p_course
            AND generation = p_generation AND stage = p_stage;
        IF NOT FOUND THEN
            RETURN NULL;
        END IF;

        SELECT COALESCE(jsonb_agg(object_id::text ORDER BY object_id), '[]'::jsonb)
          INTO manifest
          FROM public.course_retention_cleanup_manifest_object
         WHERE tenant_id = p_tenant
           AND course_id = p_course
           AND generation = p_generation
           AND stage = p_stage;

        RETURN jsonb_build_object('kind', 'cleanup', 'objects', manifest);
    END IF;

    -- Bind this stage to this lease before any delivery delete side effects.
    UPDATE public.course_retention_stage SET state = 'started', job_id = p_job,
        lease_token = p_token, claimed_at = transaction_timestamp()
      WHERE tenant_id = p_tenant AND course_id = p_course
        AND generation = p_generation AND stage = p_stage;
    IF NOT FOUND THEN
        RETURN NULL;
    END IF;

    -- Export artifacts are queued by students and may contain full payloads;
    -- reject malformed rows before deliveries are revoked.
    UPDATE public.worker_job w
       SET state='dead', lease_token=NULL, lease_expires_at=NULL,
           completed_at=transaction_timestamp(), last_error='permanent'
      FROM public.student_export_request r
     WHERE r.tenant_id = p_tenant
       AND r.course_id = p_course
       AND r.job_id = w.job_id
       AND r.state = 'queued'
       AND w.state IN ('ready', 'leased');
    UPDATE public.student_export_request
       SET state = 'failed'
     WHERE tenant_id = p_tenant AND course_id = p_course AND state = 'queued';

    IF EXISTS (
        SELECT 1
          FROM public.student_export_request r
          JOIN public.student_export_artifact a
            ON a.export_id = r.export_id
         WHERE r.tenant_id = p_tenant
           AND r.course_id = p_course
           AND a.object_payload IS NOT NULL
           AND (
               jsonb_typeof(a.object_payload) <> 'object'
               OR a.object_payload->>'id' <> a.object_id::text
               OR a.object_payload->>'bucket' <> 'student-records'
               OR a.object_payload->>'category' <> 'export'
               OR jsonb_typeof(a.object_payload->'key') <> 'object'
               OR a.object_payload->'key'->>'kind' <> 'studentRecord'
               OR a.object_payload->'key'->>'tenant' <> p_tenant::text
               OR a.object_payload->'key'->>'object' <> a.object_id::text
           )
    ) THEN
        RAISE EXCEPTION 'invalid student-record retention manifest' USING ERRCODE = '22023';
    END IF;

    WITH manifest_object AS (
        SELECT DISTINCT object_id
          FROM (
              SELECT a.object_id AS object_id
                FROM public.student_export_request r
                JOIN public.student_export_artifact a
                  ON a.export_id = r.export_id
               WHERE r.tenant_id = p_tenant
                 AND r.course_id = p_course
              UNION
              SELECT et.transcript_object_id AS object_id
                FROM public.external_tool_exchange et
                JOIN public.question_attempt qa
                  ON qa.tenant_id = et.tenant_id
                 AND qa.attempt_id = et.attempt_id
                JOIN public.assignment_run ar
                  ON ar.tenant_id = qa.tenant_id
                 AND ar.run_id = qa.run_id
                JOIN public.enrollment e
                  ON e.tenant_id = ar.tenant_id
                 AND e.enrollment_id = ar.enrollment_id
                JOIN public.assignment a
                  ON a.tenant_id = e.tenant_id
                 AND a.assignment_id = e.assignment_id
               WHERE et.tenant_id = p_tenant
                 AND a.course_id = p_course
                 AND et.transcript_object_id IS NOT NULL
          ) AS combined
         WHERE object_id IS NOT NULL
    )
    SELECT
        COALESCE(jsonb_agg(object_id::text ORDER BY object_id), '[]'::jsonb),
        COALESCE(array_agg(object_id ORDER BY object_id), ARRAY[]::uuid[]),
        COUNT(*)
      INTO manifest, object_ids, object_count
      FROM manifest_object;

    INSERT INTO public.course_retention_cleanup_manifest
        (tenant_id, course_id, generation, stage, job_id, state, object_count, prepared_at)
    VALUES (p_tenant, p_course, p_generation, p_stage, p_job, 'prepared', object_count,
            transaction_timestamp());

    INSERT INTO public.course_retention_cleanup_manifest_object
        (tenant_id, course_id, generation, stage, object_id)
    SELECT p_tenant, p_course, p_generation, p_stage, object_id
      FROM unnest(object_ids) AS object_id;

    DELETE FROM public.asset_delivery d
      USING public.student_export_request r
      JOIN public.student_export_artifact a
        ON a.export_id = r.export_id
     WHERE r.tenant_id = p_tenant
       AND r.course_id = p_course
       AND d.tenant_id = p_tenant
       AND d.delivery_id = a.object_id;

    RETURN jsonb_build_object('kind', 'cleanup', 'objects', manifest);
END $$;

CREATE FUNCTION public.ple_commit_delete_retention_work(p_tenant uuid, p_job uuid, p_token uuid, p_course uuid, p_stage text, p_generation bigint) RETURNS boolean
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'pg_catalog', 'public'
    AS $$
DECLARE
    prepared_count bigint;
    manifest_count bigint;
    current_lifecycle text;
    frozen_assignment_disposition text;
BEGIN
    IF p_tenant IS NULL
       OR p_tenant IS DISTINCT FROM public.ple_current_tenant()
       OR p_course IS NULL
       OR p_generation <= 0
       OR p_stage <> 'deleteStudentRecords'
    THEN
        RAISE EXCEPTION 'invalid retention worker capability' USING ERRCODE = '22023';
    END IF;

    PERFORM 1
      FROM public.worker_job w
      JOIN public.course_retention_dispatch d
        ON d.tenant_id = w.tenant_id
       AND d.course_id = p_course
       AND d.stage = p_stage
       AND d.generation = p_generation
       AND d.job_id = w.job_id
      JOIN public.course_retention r
        ON r.tenant_id = d.tenant_id
       AND r.course_id = d.course_id
       AND r.generation = d.generation
     WHERE w.job_id = p_job
       AND w.tenant_id = p_tenant
       AND w.state = 'leased'
       AND w.lease_token = p_token
       AND w.lease_expires_at > transaction_timestamp()
       AND w.payload = jsonb_build_object('kind', 'retention', 'course', p_course::text,
                                          'stage', p_stage, 'generation', p_generation)
     FOR UPDATE OF w, r;
    IF NOT FOUND THEN
        RETURN false;
    END IF;

    PERFORM 1
      FROM public.course_retention_stage s
     WHERE s.tenant_id = p_tenant
       AND s.course_id = p_course
       AND s.stage = p_stage
       AND s.generation = p_generation
       AND s.state = 'started'
       AND s.job_id = p_job
       AND s.lease_token = p_token
     FOR UPDATE;
    IF NOT FOUND THEN
        RETURN false;
    END IF;

    SELECT r.lifecycle, r.assignment_disposition
      INTO current_lifecycle, frozen_assignment_disposition
      FROM public.course_retention r
     WHERE r.tenant_id = p_tenant
       AND r.course_id = p_course
       AND r.generation = p_generation
     FOR UPDATE;
    IF NOT FOUND OR current_lifecycle <> 'archived' THEN
        RETURN false;
    END IF;

    SELECT m.object_count,
           COALESCE((
               SELECT COUNT(*)
                 FROM public.course_retention_cleanup_manifest_object o
                WHERE o.tenant_id = m.tenant_id
                  AND o.course_id = m.course_id
                  AND o.generation = m.generation
                  AND o.stage = m.stage
           ), 0)
      INTO prepared_count, manifest_count
      FROM public.course_retention_cleanup_manifest m
     WHERE m.tenant_id = p_tenant
       AND m.course_id = p_course
       AND m.stage = p_stage
       AND m.generation = p_generation
       AND m.job_id = p_job
       AND m.state = 'prepared'
     FOR UPDATE;
    IF NOT FOUND OR prepared_count IS NULL THEN
        RETURN false;
    END IF;
    IF prepared_count IS DISTINCT FROM manifest_count THEN
        RAISE EXCEPTION 'prepared manifest rows do not match manifest object count';
    END IF;

    -- The producer triggers share-lock this course's retention row. The
    -- worker's existing row lock therefore freezes only this course while the
    -- indexed work-set tables drive the FK-safe purge below.
    DELETE FROM public.feedback_release fr
     WHERE EXISTS (
         SELECT 1
           FROM public.attempt_feedback af
          WHERE af.tenant_id = fr.tenant_id
            AND af.attempt_id = fr.attempt_id
            AND af.course_id = p_course
            AND af.tenant_id = p_tenant
     );

    DELETE FROM public.submission_receipt_snapshot srs
     WHERE EXISTS (
         SELECT 1
           FROM public.submission_idempotency si
          WHERE si.tenant_id = srs.tenant_id
            AND si.course_id = p_course
            AND si.attempt_id = srs.attempt_id
            AND si.tenant_id = p_tenant
     );

    DELETE FROM public.submission_next_attempt sna
     WHERE sna.tenant_id = p_tenant
       AND (
           EXISTS (
               SELECT 1
                 FROM public.course_retention_purge_attempt s
                WHERE s.tenant_id = p_tenant
                  AND s.course_id = p_course
                  AND s.generation = p_generation
                  AND s.stage = p_stage
                  AND s.attempt_id = sna.predecessor_attempt_id
           )
           OR EXISTS (
               SELECT 1
                 FROM public.course_retention_purge_attempt s
                WHERE s.tenant_id = p_tenant
                  AND s.course_id = p_course
                  AND s.generation = p_generation
                  AND s.stage = p_stage
                  AND s.attempt_id = sna.next_attempt_id
           )
       );

    DELETE FROM public.question_statistics_contribution_receipt qsr
     WHERE qsr.tenant_id = p_tenant
       AND (
           EXISTS (
               SELECT 1
                 FROM public.course_retention_purge_run s
                WHERE s.tenant_id = p_tenant
                  AND s.course_id = p_course
                  AND s.generation = p_generation
                  AND s.stage = p_stage
                  AND s.run_id = qsr.first_completed_run_id
           )
           OR EXISTS (
               SELECT 1
                 FROM public.course_retention_purge_attempt s
                WHERE s.tenant_id = p_tenant
                  AND s.course_id = p_course
                  AND s.generation = p_generation
                  AND s.stage = p_stage
                  AND s.attempt_id = qsr.attempt_id
           )
       );

    DELETE FROM public.question_prefetch qp
     WHERE qp.tenant_id = p_tenant
       AND (
           EXISTS (
               SELECT 1
                 FROM public.course_retention_purge_run s
                WHERE s.tenant_id = p_tenant
                  AND s.course_id = p_course
                  AND s.generation = p_generation
                  AND s.stage = p_stage
                  AND s.run_id = qp.run_id
           )
           OR EXISTS (
               SELECT 1
                 FROM public.course_retention_purge_attempt s
                WHERE s.tenant_id = p_tenant
                  AND s.course_id = p_course
                  AND s.generation = p_generation
                  AND s.stage = p_stage
                  AND s.attempt_id = qp.predecessor_attempt_id
           )
       );

    DELETE FROM public.external_tool_launch_session
     WHERE tenant_id = p_tenant
       AND course_id = p_course;

    DELETE FROM public.external_tool_exchange
     WHERE tenant_id = p_tenant
       AND course_id = p_course;

    DELETE FROM public.attempt_feedback
     WHERE tenant_id = p_tenant
       AND course_id = p_course;

    DELETE FROM public.attempt_score_current
     WHERE tenant_id = p_tenant
       AND course_id = p_course;

    DELETE FROM public.manual_grade_receipt
     WHERE tenant_id = p_tenant
       AND course_id = p_course;

    DELETE FROM public.submission_evaluation
     WHERE tenant_id = p_tenant
       AND course_id = p_course;

    DELETE FROM public.submission
     WHERE tenant_id = p_tenant
       AND course_id = p_course;

    DELETE FROM public.submission_idempotency
     WHERE tenant_id = p_tenant
       AND course_id = p_course;

    DELETE FROM public.worker_job w
     USING public.course_retention_purge_attempt purge
     WHERE purge.tenant_id = p_tenant
       AND purge.course_id = p_course
       AND purge.generation = p_generation
       AND purge.stage = p_stage
       AND w.tenant_id = purge.tenant_id
       AND w.payload ->> 'kind' = 'autoSubmitAttempt'
       AND (w.payload ->> 'attempt')::uuid = purge.attempt_id;

    DELETE FROM public.question_attempt
     WHERE tenant_id = p_tenant
       AND course_id = p_course;

    -- This course-local projection contains learner-derived aggregates even
    -- though it holds no learner identities, so delete it before retaining
    -- or deleting its assignment definition.
    DELETE FROM public.course_item_analysis_current
     WHERE tenant_id = p_tenant
       AND course_id = p_course;

    DELETE FROM public.course_item_analysis_staging
     WHERE tenant_id = p_tenant
       AND course_id = p_course;

    DELETE FROM public.worker_job w
     WHERE w.tenant_id = p_tenant
       AND w.payload ->> 'kind' = 'recalculateCourseItemAnalysis'
       AND EXISTS (
           SELECT 1
             FROM public.assignment a
            WHERE a.tenant_id = w.tenant_id
              AND a.assignment_id = (w.payload ->> 'assignment')::uuid
              AND a.course_id = p_course
       );

    DELETE FROM public.student_assignment_summary sas
     WHERE EXISTS (
         SELECT 1
           FROM public.enrollment e
           JOIN public.assignment a
             ON a.tenant_id = e.tenant_id
            AND a.assignment_id = e.assignment_id
          WHERE e.tenant_id = sas.tenant_id
            AND e.enrollment_id = sas.enrollment_id
            AND a.course_id = p_course
            AND sas.tenant_id = p_tenant
     );

    DELETE FROM public.assignment_run ar
     WHERE EXISTS (
         SELECT 1
           FROM public.enrollment e
           JOIN public.assignment a
             ON a.tenant_id = e.tenant_id
            AND a.assignment_id = e.assignment_id
          WHERE e.tenant_id = ar.tenant_id
            AND e.enrollment_id = ar.enrollment_id
            AND a.course_id = p_course
            AND ar.tenant_id = p_tenant
     );

    DELETE FROM public.enrollment e
     WHERE EXISTS (
         SELECT 1
           FROM public.assignment a
          WHERE a.tenant_id = e.tenant_id
            AND a.assignment_id = e.assignment_id
            AND a.course_id = p_course
            AND e.tenant_id = p_tenant
     );

    DELETE FROM public.record_access_log
     WHERE tenant_id = p_tenant
       AND delivery_scope = 'student_record'
       AND course_id = p_course;

    DELETE FROM public.audit_event
     WHERE tenant_id = p_tenant
       AND course_id = p_course;

    DELETE FROM public.asset_delivery
     WHERE tenant_id = p_tenant
       AND delivery_kind = 'student_record'
       AND course_id = p_course;

    DELETE FROM public.student_export_artifact a
     WHERE EXISTS (
         SELECT 1
           FROM public.course_retention_purge_export s
          WHERE s.tenant_id = p_tenant
            AND s.course_id = p_course
            AND s.generation = p_generation
            AND s.stage = p_stage
            AND s.export_id = a.export_id
     );

    DELETE FROM public.student_export_request
     WHERE tenant_id = p_tenant
       AND course_id = p_course;

    DELETE FROM public.worker_job w
     WHERE w.tenant_id = p_tenant
       AND EXISTS (
           SELECT 1
             FROM public.course_retention_purge_export s
            WHERE s.tenant_id = p_tenant
              AND s.course_id = p_course
              AND s.generation = p_generation
              AND s.stage = p_stage
              AND s.job_id = w.job_id
       );

    DELETE FROM public.assignment_policy_exception
     WHERE tenant_id = p_tenant
       AND course_id = p_course
       AND student_id IS NOT NULL;

    DELETE FROM public.course_group_member
     WHERE tenant_id = p_tenant
       AND course_id = p_course;

    DELETE FROM public.course_member
     WHERE tenant_id = p_tenant
       AND course_id = p_course
       AND role = 'student';

    IF frozen_assignment_disposition = 'delete' THEN
        DELETE FROM public.assignment_item ap
         USING public.assignment a
         WHERE ap.tenant_id = a.tenant_id
           AND ap.assignment_id = a.assignment_id
           AND a.tenant_id = p_tenant
           AND a.course_id = p_course;

        DELETE FROM public.assignment a
         WHERE a.tenant_id = p_tenant
           AND a.course_id = p_course;
    END IF;

    IF EXISTS (
        SELECT 1 FROM public.feedback_release fr
         WHERE fr.tenant_id = p_tenant
           AND EXISTS (
               SELECT 1
                 FROM public.course_retention_purge_attempt s
                WHERE s.tenant_id = p_tenant
                  AND s.course_id = p_course
                  AND s.generation = p_generation
                  AND s.stage = p_stage
                  AND s.attempt_id = fr.attempt_id
           )
        UNION ALL
        SELECT 1 FROM public.submission_receipt_snapshot srs
         WHERE srs.tenant_id = p_tenant
           AND EXISTS (
               SELECT 1
                 FROM public.course_retention_purge_attempt s
                WHERE s.tenant_id = p_tenant
                  AND s.course_id = p_course
                  AND s.generation = p_generation
                  AND s.stage = p_stage
                  AND s.attempt_id = srs.attempt_id
           )
        UNION ALL
        SELECT 1 FROM public.submission_next_attempt sna
         WHERE sna.tenant_id = p_tenant
           AND (
               EXISTS (
                   SELECT 1
                     FROM public.course_retention_purge_attempt s
                    WHERE s.tenant_id = p_tenant
                      AND s.course_id = p_course
                      AND s.generation = p_generation
                      AND s.stage = p_stage
                      AND s.attempt_id = sna.predecessor_attempt_id
               )
               OR EXISTS (
                   SELECT 1
                     FROM public.course_retention_purge_attempt s
                    WHERE s.tenant_id = p_tenant
                      AND s.course_id = p_course
                      AND s.generation = p_generation
                      AND s.stage = p_stage
                      AND s.attempt_id = sna.next_attempt_id
               )
           )
        UNION ALL
        SELECT 1 FROM public.question_statistics_contribution_receipt qsr
         WHERE qsr.tenant_id = p_tenant
           AND (
               EXISTS (
                   SELECT 1
                     FROM public.course_retention_purge_run s
                    WHERE s.tenant_id = p_tenant
                      AND s.course_id = p_course
                      AND s.generation = p_generation
                      AND s.stage = p_stage
                      AND s.run_id = qsr.first_completed_run_id
               )
               OR EXISTS (
                   SELECT 1
                     FROM public.course_retention_purge_attempt s
                    WHERE s.tenant_id = p_tenant
                      AND s.course_id = p_course
                      AND s.generation = p_generation
                      AND s.stage = p_stage
                      AND s.attempt_id = qsr.attempt_id
               )
           )
        UNION ALL
        SELECT 1 FROM public.question_prefetch qp
         WHERE qp.tenant_id = p_tenant
           AND (
               EXISTS (
                   SELECT 1
                     FROM public.course_retention_purge_run s
                    WHERE s.tenant_id = p_tenant
                      AND s.course_id = p_course
                      AND s.generation = p_generation
                      AND s.stage = p_stage
                      AND s.run_id = qp.run_id
               )
               OR EXISTS (
                   SELECT 1
                     FROM public.course_retention_purge_attempt s
                    WHERE s.tenant_id = p_tenant
                      AND s.course_id = p_course
                      AND s.generation = p_generation
                      AND s.stage = p_stage
                      AND s.attempt_id = qp.predecessor_attempt_id
               )
           )
        UNION ALL
        SELECT 1 FROM public.external_tool_launch_session e
         WHERE e.tenant_id = p_tenant
           AND e.course_id = p_course
        UNION ALL
        SELECT 1 FROM public.external_tool_exchange e
         WHERE e.tenant_id = p_tenant
           AND e.course_id = p_course
        UNION ALL
        SELECT 1 FROM public.attempt_feedback af
         WHERE af.tenant_id = p_tenant
           AND af.course_id = p_course
        UNION ALL
        SELECT 1 FROM public.submission s
         WHERE s.tenant_id = p_tenant
           AND s.course_id = p_course
        UNION ALL
        SELECT 1 FROM public.manual_grade_receipt receipt
         WHERE receipt.tenant_id = p_tenant
           AND receipt.course_id = p_course
        UNION ALL
        SELECT 1 FROM public.submission_evaluation ge
         WHERE ge.tenant_id = p_tenant
           AND ge.course_id = p_course
        UNION ALL
        SELECT 1 FROM public.attempt_score_current score
         WHERE score.tenant_id = p_tenant
           AND score.course_id = p_course
        UNION ALL
        SELECT 1 FROM public.submission_idempotency si
         WHERE si.tenant_id = p_tenant
           AND si.course_id = p_course
        UNION ALL
        SELECT 1 FROM public.question_attempt qa
         WHERE qa.tenant_id = p_tenant
           AND qa.course_id = p_course
        UNION ALL
        SELECT 1 FROM public.course_item_analysis_current analysis
         WHERE analysis.tenant_id = p_tenant
           AND analysis.course_id = p_course
        UNION ALL
        SELECT 1 FROM public.course_item_analysis_staging analysis
         WHERE analysis.tenant_id = p_tenant
           AND analysis.course_id = p_course
        UNION ALL
        SELECT 1 FROM public.worker_job w
         WHERE w.tenant_id = p_tenant
           AND w.payload ->> 'kind' = 'recalculateCourseItemAnalysis'
           AND EXISTS (
               SELECT 1
                 FROM public.assignment a
                WHERE a.tenant_id = w.tenant_id
                  AND a.assignment_id = (w.payload ->> 'assignment')::uuid
                  AND a.course_id = p_course
           )
        UNION ALL
        SELECT 1 FROM public.attempt_timing_current timing
         WHERE timing.tenant_id = p_tenant
           AND timing.course_id = p_course
        UNION ALL
        SELECT 1 FROM public.worker_job w
         WHERE w.tenant_id = p_tenant
           AND w.payload ->> 'kind' = 'autoSubmitAttempt'
           AND EXISTS (
               SELECT 1 FROM public.course_retention_purge_attempt purge
                WHERE purge.tenant_id = p_tenant
                  AND purge.course_id = p_course
                  AND purge.generation = p_generation
                  AND purge.stage = p_stage
                  AND purge.attempt_id = (w.payload ->> 'attempt')::uuid
           )
        UNION ALL
        SELECT 1 FROM public.student_assignment_summary sas
         WHERE EXISTS (
            SELECT 1
              FROM public.enrollment e
              JOIN public.assignment a
                ON a.tenant_id = e.tenant_id
               AND a.assignment_id = e.assignment_id
             WHERE e.tenant_id = sas.tenant_id
               AND sas.tenant_id = p_tenant
               AND e.enrollment_id = sas.enrollment_id
               AND a.course_id = p_course
         )
        UNION ALL
        SELECT 1 FROM public.assignment_run ar
         WHERE EXISTS (
           SELECT 1
              FROM public.enrollment e
              JOIN public.assignment a
                ON a.tenant_id = e.tenant_id
               AND a.assignment_id = e.assignment_id
               WHERE e.tenant_id = ar.tenant_id
                 AND ar.tenant_id = p_tenant
                 AND e.enrollment_id = ar.enrollment_id
                 AND a.course_id = p_course
         )
        UNION ALL
        SELECT 1 FROM public.enrollment e
         WHERE EXISTS (
            SELECT 1
              FROM public.assignment a
             WHERE a.tenant_id = e.tenant_id
               AND a.assignment_id = e.assignment_id
               AND a.tenant_id = p_tenant
               AND a.course_id = p_course
         )
        UNION ALL
        SELECT 1 FROM public.record_access_log ae
         WHERE ae.tenant_id = p_tenant
           AND ae.delivery_scope = 'student_record'
           AND ae.course_id = p_course
        UNION ALL
        SELECT 1 FROM public.audit_event ae
         WHERE ae.tenant_id = p_tenant
           AND ae.course_id = p_course
        UNION ALL
        SELECT 1 FROM public.asset_delivery ad
         WHERE ad.tenant_id = p_tenant
           AND ad.delivery_kind = 'student_record'
           AND ad.course_id = p_course
        UNION ALL
        SELECT 1 FROM public.student_export_artifact sae
         WHERE EXISTS (
             SELECT 1
               FROM public.course_retention_purge_export s
              WHERE s.tenant_id = p_tenant
                AND s.course_id = p_course
                AND s.generation = p_generation
                AND s.stage = p_stage
                AND s.export_id = sae.export_id
         )
        UNION ALL
        SELECT 1 FROM public.student_export_request ser
         WHERE ser.tenant_id = p_tenant
           AND ser.course_id = p_course
        UNION ALL
        SELECT 1 FROM public.worker_job w
         WHERE w.tenant_id = p_tenant
           AND EXISTS (
               SELECT 1
                 FROM public.course_retention_purge_export s
                WHERE s.tenant_id = p_tenant
                  AND s.course_id = p_course
                  AND s.generation = p_generation
                  AND s.stage = p_stage
                  AND s.job_id = w.job_id
           )
        UNION ALL
        SELECT 1 FROM public.course_member cm
         WHERE cm.tenant_id = p_tenant
           AND cm.course_id = p_course
           AND cm.role = 'student'
        UNION ALL
        SELECT 1 FROM public.course_group_member cgm
         WHERE cgm.tenant_id = p_tenant
           AND cgm.course_id = p_course
        UNION ALL
        SELECT 1 FROM public.assignment_policy_exception exception
         WHERE exception.tenant_id = p_tenant
           AND exception.course_id = p_course
           AND exception.student_id IS NOT NULL
        UNION ALL
        SELECT 1 FROM public.assignment_item ap
         WHERE frozen_assignment_disposition = 'delete'
           AND EXISTS (
               SELECT 1
                 FROM public.assignment a
                WHERE a.tenant_id = ap.tenant_id
                  AND a.assignment_id = ap.assignment_id
                  AND a.course_id = p_course
           )
        UNION ALL
        SELECT 1 FROM public.assignment a
         WHERE frozen_assignment_disposition = 'delete'
           AND a.tenant_id = p_tenant
           AND a.course_id = p_course
    ) THEN
        RAISE EXCEPTION 'delete retention commit left residual learner rows';
    END IF;

    -- The work sets contain educational-record identities, so successful
    -- deletion erases them before the durable coarse tombstone is written.
    DELETE FROM public.course_retention_purge_export
     WHERE tenant_id = p_tenant
       AND course_id = p_course
       AND generation = p_generation
       AND stage = p_stage;
    DELETE FROM public.course_retention_purge_attempt
     WHERE tenant_id = p_tenant
       AND course_id = p_course
       AND generation = p_generation
       AND stage = p_stage;
    DELETE FROM public.course_retention_purge_run
     WHERE tenant_id = p_tenant
       AND course_id = p_course
       AND generation = p_generation
       AND stage = p_stage;

    UPDATE public.course_retention_cleanup_manifest
       SET state = 'completed',
           completed_at = transaction_timestamp()
     WHERE tenant_id = p_tenant
       AND course_id = p_course
       AND generation = p_generation
       AND stage = p_stage
       AND job_id = p_job
       AND state = 'prepared';
    IF NOT FOUND THEN
        RAISE EXCEPTION 'failed to finalize delete manifest';
    END IF;

    UPDATE public.course_retention_stage
       SET state = 'completed'
     WHERE tenant_id = p_tenant
       AND course_id = p_course
       AND stage = p_stage
       AND generation = p_generation
       AND state = 'started'
       AND job_id = p_job
       AND lease_token = p_token;
    IF NOT FOUND THEN
        RAISE EXCEPTION 'failed to complete delete retention stage';
    END IF;

    UPDATE public.course_retention
       SET lifecycle = 'deleted'
     WHERE tenant_id = p_tenant
       AND course_id = p_course
       AND generation = p_generation
       AND lifecycle = 'archived';
    IF NOT FOUND THEN
        RAISE EXCEPTION 'failed to mark retention lifecycle deleted';
    END IF;

    UPDATE public.worker_job
       SET state = 'completed',
           lease_token = NULL,
           lease_expires_at = NULL,
           completed_at = transaction_timestamp()
     WHERE job_id = p_job
       AND tenant_id = p_tenant
       AND state = 'leased'
       AND lease_token = p_token
       AND payload = jsonb_build_object('kind', 'retention', 'course', p_course::text,
                                       'stage', p_stage, 'generation', p_generation);
    IF NOT FOUND THEN
        RAISE EXCEPTION 'failed to mark delete worker job complete';
    END IF;

    RETURN true;
END $$;

CREATE FUNCTION public.ple_prepare_delete_retention_work(p_tenant uuid, p_job uuid, p_token uuid, p_course uuid, p_stage text, p_generation bigint) RETURNS jsonb
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'pg_catalog', 'public'
    AS $$
DECLARE
    current_lifecycle text;
    existing_state text;
    existing_job uuid;
    existing_object_count bigint;
    manifest_count bigint;
    manifest jsonb;
    object_count bigint;
BEGIN
    IF p_tenant IS NULL
       OR p_tenant IS DISTINCT FROM public.ple_current_tenant()
       OR p_generation <= 0
       OR p_course IS NULL
       OR p_token IS NULL
       OR p_job IS NULL
       OR p_stage <> 'deleteStudentRecords'
    THEN
        RETURN NULL;
    END IF;

    PERFORM 1
      FROM public.worker_job w
      JOIN public.course_retention r
        ON r.tenant_id = w.tenant_id AND r.course_id = p_course
      JOIN public.course_retention_dispatch d
        ON d.tenant_id = w.tenant_id
       AND d.course_id = p_course
       AND d.stage = p_stage
       AND d.generation = p_generation
       AND d.job_id = w.job_id
     WHERE w.job_id = p_job
       AND w.tenant_id = p_tenant
       AND w.state = 'leased'
       AND w.lease_token = p_token
       AND w.lease_expires_at > transaction_timestamp()
       AND w.payload = jsonb_build_object('kind', 'retention', 'course', p_course::text,
                                          'stage', p_stage, 'generation', p_generation)
     FOR UPDATE OF w;
    IF NOT FOUND THEN
        RETURN NULL;
    END IF;

    PERFORM 1
      FROM public.course_retention_stage s
     WHERE s.tenant_id = p_tenant
       AND s.course_id = p_course
       AND s.stage = p_stage
       AND s.generation = p_generation
       AND s.due_at <= transaction_timestamp()
       AND (s.state = 'scheduled' OR (s.state = 'started' AND s.job_id = p_job))
     FOR UPDATE;
    IF NOT FOUND THEN
        RETURN NULL;
    END IF;

    SELECT r.lifecycle INTO current_lifecycle
      FROM public.course_retention r
     WHERE r.tenant_id = p_tenant
       AND r.course_id = p_course
       AND r.generation = p_generation
     FOR UPDATE;
    IF NOT FOUND OR current_lifecycle NOT IN ('active', 'archived') THEN
        RETURN NULL;
    END IF;

    SELECT m.state,
           m.job_id,
           m.object_count,
           COALESCE((
               SELECT COUNT(*)
                 FROM public.course_retention_cleanup_manifest_object o
                WHERE o.tenant_id = m.tenant_id
                  AND o.course_id = m.course_id
                  AND o.generation = m.generation
                  AND o.stage = m.stage
           ), 0)
      INTO existing_state,
           existing_job,
           existing_object_count,
           manifest_count
      FROM public.course_retention_cleanup_manifest m
     WHERE m.tenant_id = p_tenant
       AND m.course_id = p_course
       AND m.stage = p_stage
       AND m.generation = p_generation
     FOR UPDATE;
    IF FOUND THEN
        IF existing_state <> 'prepared'
           OR existing_job IS DISTINCT FROM p_job
           OR existing_object_count IS DISTINCT FROM manifest_count THEN
            RETURN NULL;
        END IF;
        IF EXISTS (
            (SELECT ar.run_id
               FROM public.assignment_run ar
               JOIN public.enrollment e
                 ON e.tenant_id = ar.tenant_id
                AND e.enrollment_id = ar.enrollment_id
               JOIN public.assignment a
                 ON a.tenant_id = e.tenant_id
                AND a.assignment_id = e.assignment_id
              WHERE ar.tenant_id = p_tenant
                AND a.course_id = p_course
             EXCEPT
             SELECT s.run_id
               FROM public.course_retention_purge_run s
              WHERE s.tenant_id = p_tenant
                AND s.course_id = p_course
                AND s.generation = p_generation
                AND s.stage = p_stage)
            UNION ALL
            (SELECT s.run_id
               FROM public.course_retention_purge_run s
              WHERE s.tenant_id = p_tenant
                AND s.course_id = p_course
                AND s.generation = p_generation
                AND s.stage = p_stage
             EXCEPT
             SELECT ar.run_id
               FROM public.assignment_run ar
               JOIN public.enrollment e
                 ON e.tenant_id = ar.tenant_id
                AND e.enrollment_id = ar.enrollment_id
               JOIN public.assignment a
                 ON a.tenant_id = e.tenant_id
                AND a.assignment_id = e.assignment_id
              WHERE ar.tenant_id = p_tenant
                AND a.course_id = p_course)
        ) OR EXISTS (
            (SELECT qa.attempt_id
               FROM public.question_attempt qa
              WHERE qa.tenant_id = p_tenant
                AND qa.course_id = p_course
             EXCEPT
             SELECT s.attempt_id
               FROM public.course_retention_purge_attempt s
              WHERE s.tenant_id = p_tenant
                AND s.course_id = p_course
                AND s.generation = p_generation
                AND s.stage = p_stage)
            UNION ALL
            (SELECT s.attempt_id
               FROM public.course_retention_purge_attempt s
              WHERE s.tenant_id = p_tenant
                AND s.course_id = p_course
                AND s.generation = p_generation
                AND s.stage = p_stage
             EXCEPT
             SELECT qa.attempt_id
               FROM public.question_attempt qa
              WHERE qa.tenant_id = p_tenant
                AND qa.course_id = p_course)
        ) OR EXISTS (
            (SELECT r.export_id, r.job_id
               FROM public.student_export_request r
              WHERE r.tenant_id = p_tenant
                AND r.course_id = p_course
             EXCEPT
             SELECT s.export_id, s.job_id
               FROM public.course_retention_purge_export s
              WHERE s.tenant_id = p_tenant
                AND s.course_id = p_course
                AND s.generation = p_generation
                AND s.stage = p_stage)
            UNION ALL
            (SELECT s.export_id, s.job_id
               FROM public.course_retention_purge_export s
              WHERE s.tenant_id = p_tenant
                AND s.course_id = p_course
                AND s.generation = p_generation
                AND s.stage = p_stage
             EXCEPT
             SELECT r.export_id, r.job_id
               FROM public.student_export_request r
              WHERE r.tenant_id = p_tenant
                AND r.course_id = p_course)
        ) THEN
            RETURN NULL;
        END IF;

        UPDATE public.course_retention_stage
           SET state = 'started',
               job_id = p_job,
               lease_token = p_token,
               claimed_at = transaction_timestamp()
         WHERE tenant_id = p_tenant
           AND course_id = p_course
           AND stage = p_stage
           AND generation = p_generation;
        IF NOT FOUND THEN
            RAISE EXCEPTION 'failed to bind delete retention stage';
        END IF;

        SELECT COALESCE(jsonb_agg(object_id::text ORDER BY object_id), '[]'::jsonb)
          INTO manifest
          FROM public.course_retention_cleanup_manifest_object
         WHERE tenant_id = p_tenant
           AND course_id = p_course
           AND generation = p_generation
           AND stage = p_stage;
        RETURN jsonb_build_object('kind', 'cleanup', 'objects', manifest);
    END IF;

    IF EXISTS (
        SELECT 1
          FROM public.student_export_request r
          JOIN public.student_export_artifact a
            ON a.export_id = r.export_id
         WHERE r.tenant_id = p_tenant
           AND r.course_id = p_course
           AND a.object_payload IS NOT NULL
           AND (
               jsonb_typeof(a.object_payload) <> 'object'
               OR a.object_payload->>'id' <> a.object_id::text
               OR a.object_payload->>'bucket' <> 'student-records'
               OR a.object_payload->>'category' <> 'export'
               OR jsonb_typeof(a.object_payload->'key') <> 'object'
               OR a.object_payload->'key'->>'kind' <> 'studentRecord'
               OR a.object_payload->'key'->>'tenant' <> p_tenant::text
               OR a.object_payload->'key'->>'object' <> a.object_id::text
           )
    ) THEN
        RAISE EXCEPTION 'invalid student-record retention manifest';
    END IF;

    IF EXISTS (
        SELECT 1 FROM public.external_tool_exchange et
         WHERE et.tenant_id = p_tenant
           AND et.course_id = p_course
           AND et.transcript_object_id IS NOT NULL
           AND NOT EXISTS (
               SELECT 1
                 FROM public.question_attempt qa
                 JOIN public.assignment_run ar
                   ON ar.tenant_id = qa.tenant_id
                  AND ar.run_id = qa.run_id
                 JOIN public.enrollment e
                   ON e.tenant_id = ar.tenant_id
                  AND e.enrollment_id = ar.enrollment_id
                 JOIN public.assignment a
                   ON a.tenant_id = e.tenant_id
                  AND a.assignment_id = e.assignment_id
                WHERE qa.tenant_id = et.tenant_id
                  AND qa.attempt_id = et.attempt_id
                  AND a.course_id = p_course
           )
    ) THEN
        RAISE EXCEPTION 'cannot verify external tool transcript ownership';
    END IF;

    WITH manifest_object AS (
        SELECT DISTINCT a.object_id AS object_id
          FROM public.student_export_request r
          JOIN public.student_export_artifact a
            ON a.export_id = r.export_id
         WHERE r.tenant_id = p_tenant
           AND r.course_id = p_course
        UNION
        SELECT DISTINCT d.object_id AS object_id

          FROM public.asset_delivery d
         WHERE d.tenant_id = p_tenant
           AND d.course_id = p_course
           AND d.delivery_kind = 'student_record'
        UNION
        SELECT DISTINCT et.transcript_object_id AS object_id
          FROM public.external_tool_exchange et
         WHERE et.tenant_id = p_tenant
           AND et.course_id = p_course
           AND et.transcript_object_id IS NOT NULL
    )
    SELECT COALESCE(jsonb_agg(object_id::text ORDER BY object_id), '[]'::jsonb),
           COUNT(*)
      INTO manifest, object_count
      FROM manifest_object;

    UPDATE public.course_retention_stage
       SET state = 'started',
           job_id = p_job,
           lease_token = p_token,
           claimed_at = transaction_timestamp()
     WHERE tenant_id = p_tenant
       AND course_id = p_course
       AND stage = p_stage
       AND generation = p_generation;
    IF NOT FOUND THEN
        RAISE EXCEPTION 'failed to bind delete retention stage';
    END IF;

    UPDATE public.worker_job w
       SET state = 'dead',
           lease_token = NULL,
           lease_expires_at = NULL,
           completed_at = transaction_timestamp(),
           last_error = 'permanent'
      FROM public.student_export_request r
     WHERE r.tenant_id = p_tenant
       AND r.course_id = p_course
       AND r.job_id = w.job_id
       AND r.state = 'queued'
       AND w.state IN ('ready', 'leased');

    UPDATE public.student_export_request
       SET state = 'failed'
     WHERE tenant_id = p_tenant
       AND course_id = p_course
       AND state = 'queued';

    INSERT INTO public.course_retention_cleanup_manifest
        (tenant_id, course_id, generation, stage, job_id, state,
         object_count, prepared_at)
    VALUES (p_tenant, p_course, p_generation, p_stage, p_job, 'prepared',
            object_count, transaction_timestamp());

    INSERT INTO public.course_retention_cleanup_manifest_object
        (tenant_id, course_id, generation, stage, object_id)
    SELECT p_tenant, p_course, p_generation, p_stage, manifest_object.object_id
      FROM (
          SELECT a.object_id
            FROM public.student_export_request r
            JOIN public.student_export_artifact a
              ON a.export_id = r.export_id
           WHERE r.tenant_id = p_tenant
             AND r.course_id = p_course
          UNION
          SELECT d.object_id
            FROM public.asset_delivery d
           WHERE d.tenant_id = p_tenant
             AND d.course_id = p_course
             AND d.delivery_kind = 'student_record'
          UNION
          SELECT et.transcript_object_id
            FROM public.external_tool_exchange et
           WHERE et.tenant_id = p_tenant
             AND et.course_id = p_course
             AND et.transcript_object_id IS NOT NULL
      ) AS manifest_object;

    INSERT INTO public.course_retention_purge_run
        (tenant_id, course_id, generation, stage, run_id)
    SELECT p_tenant, p_course, p_generation, p_stage, ar.run_id
      FROM public.assignment_run ar
      JOIN public.enrollment e
        ON e.tenant_id = ar.tenant_id
       AND e.enrollment_id = ar.enrollment_id
      JOIN public.assignment a
        ON a.tenant_id = e.tenant_id
       AND a.assignment_id = e.assignment_id
     WHERE ar.tenant_id = p_tenant
       AND a.course_id = p_course;

    INSERT INTO public.course_retention_purge_attempt
        (tenant_id, course_id, generation, stage, attempt_id)
    SELECT p_tenant, p_course, p_generation, p_stage, qa.attempt_id
      FROM public.question_attempt qa
     WHERE qa.tenant_id = p_tenant
       AND qa.course_id = p_course;

    INSERT INTO public.course_retention_purge_export
        (tenant_id, course_id, generation, stage, export_id, job_id)
    SELECT p_tenant, p_course, p_generation, p_stage, r.export_id, r.job_id
      FROM public.student_export_request r
     WHERE r.tenant_id = p_tenant
       AND r.course_id = p_course;

    DELETE FROM public.asset_delivery
     WHERE tenant_id = p_tenant
       AND delivery_kind = 'student_record'
       AND course_id = p_course;

    IF current_lifecycle = 'active' THEN
        UPDATE public.course_retention
           SET lifecycle = 'archived'
         WHERE tenant_id = p_tenant
           AND course_id = p_course
           AND generation = p_generation
           AND lifecycle = 'active';
        IF NOT FOUND THEN
            RAISE EXCEPTION 'failed to archive retention lifecycle before delete work';
        END IF;
    END IF;

    RETURN jsonb_build_object('kind', 'cleanup', 'objects', manifest);
END $$;

SET default_tablespace = '';

SET default_table_access_method = heap;

ALTER FUNCTION public.ple_apply_retention_api_action(character, uuid, bigint, text, integer, text) OWNER TO ple_retention_broker;

ALTER FUNCTION public.ple_bind_course_from_attempt() OWNER TO ple_retention_broker;

ALTER FUNCTION public.ple_bind_course_item_analysis_course() OWNER TO ple_retention_broker;

ALTER FUNCTION public.ple_bind_question_attempt_course() OWNER TO ple_retention_broker;

ALTER FUNCTION public.ple_commit_retention_work(uuid, uuid, uuid, uuid, text, bigint) OWNER TO ple_retention_broker;

ALTER FUNCTION public.ple_configure_retention_policy(character, integer, integer, integer) OWNER TO ple_retention_broker;

ALTER FUNCTION public.ple_course_records_accessible(uuid, uuid) OWNER TO ple_retention_broker;

ALTER FUNCTION public.ple_dispatch_due_retention_stages(integer) OWNER TO ple_retention_broker;

ALTER FUNCTION public.ple_end_course_retention(character, uuid) OWNER TO ple_retention_broker;

ALTER FUNCTION public.ple_extend_course_retention(character, uuid, integer) OWNER TO ple_retention_broker;

ALTER FUNCTION public.ple_fence_assignment_definition_write() OWNER TO ple_retention_broker;

ALTER FUNCTION public.ple_fence_learner_record_write() OWNER TO ple_retention_broker;

ALTER FUNCTION public.ple_lock_course_write(uuid, uuid, boolean) OWNER TO ple_retention_broker;

ALTER FUNCTION public.ple_prepare_retention_work(uuid, uuid, uuid, uuid, text, bigint) OWNER TO ple_retention_broker;

ALTER FUNCTION public.ple_read_course_retention(character, uuid) OWNER TO ple_retention_broker;

ALTER FUNCTION public.ple_read_retention_notification(character, uuid) OWNER TO ple_retention_broker;

ALTER FUNCTION public.ple_retention_authorize(character, uuid, boolean) OWNER TO ple_retention_broker;

ALTER FUNCTION public.ple_set_archive_disposition(character, uuid, text) OWNER TO ple_retention_broker;

ALTER FUNCTION public.ple_commit_archive_retention_work(uuid, uuid, uuid, uuid, text, bigint) OWNER TO ple_retention_broker;

ALTER FUNCTION public.ple_prepare_archive_retention_work(uuid, uuid, uuid, uuid, text, bigint) OWNER TO ple_retention_broker;

ALTER FUNCTION public.ple_commit_delete_retention_work(uuid, uuid, uuid, uuid, text, bigint) OWNER TO ple_retention_broker;

ALTER FUNCTION public.ple_prepare_delete_retention_work(uuid, uuid, uuid, uuid, text, bigint) OWNER TO ple_retention_broker;

CREATE TABLE public.course_retention (
    tenant_id uuid NOT NULL,
    course_id uuid NOT NULL,
    ended_at timestamp with time zone NOT NULL,
    notify_days integer NOT NULL,
    archive_days integer NOT NULL,
    delete_days integer NOT NULL,
    assignment_disposition text NOT NULL,
    generation bigint NOT NULL,
    lifecycle text DEFAULT 'active'::text NOT NULL,
    CONSTRAINT course_retention_archive_days_check CHECK (((archive_days >= 1) AND (archive_days <= 36500))),
    CONSTRAINT course_retention_assignment_disposition_check CHECK ((assignment_disposition = ANY (ARRAY['retain'::text, 'delete'::text]))),
    CONSTRAINT course_retention_check CHECK (((notify_days < archive_days) AND (archive_days < delete_days))),
    CONSTRAINT course_retention_delete_days_check CHECK (((delete_days >= 1) AND (delete_days <= 36500))),
    CONSTRAINT course_retention_generation_check CHECK ((generation > 0)),
    CONSTRAINT course_retention_lifecycle_check CHECK ((lifecycle = ANY (ARRAY['active'::text, 'archived'::text, 'deleted'::text]))),
    CONSTRAINT course_retention_notify_days_check CHECK (((notify_days >= 1) AND (notify_days <= 36500)))
);

ALTER TABLE ONLY public.course_retention FORCE ROW LEVEL SECURITY;

CREATE TABLE public.course_retention_api_receipt (
    tenant_id uuid NOT NULL,
    course_id uuid NOT NULL,
    expected_generation bigint NOT NULL,
    actor_id uuid NOT NULL,
    action text NOT NULL,
    assignment_disposition text,
    resulting_generation bigint NOT NULL,
    stage text NOT NULL,
    created_at timestamp with time zone DEFAULT transaction_timestamp() NOT NULL,
    CONSTRAINT course_retention_api_receipt_action_check CHECK ((action = ANY (ARRAY['archive'::text, 'delete'::text]))),
    CONSTRAINT course_retention_api_receipt_assignment_disposition_check CHECK ((assignment_disposition = ANY (ARRAY['retain'::text, 'delete'::text]))),
    CONSTRAINT course_retention_api_receipt_expected_generation_check CHECK ((expected_generation > 0)),
    CONSTRAINT course_retention_api_receipt_resulting_generation_check CHECK ((resulting_generation > 0)),
    CONSTRAINT course_retention_api_receipt_stage_check CHECK ((stage = ANY (ARRAY['archiveStudentRecords'::text, 'deleteStudentRecords'::text])))
);

ALTER TABLE ONLY public.course_retention_api_receipt FORCE ROW LEVEL SECURITY;

CREATE TABLE public.course_retention_cleanup_manifest (
    tenant_id uuid NOT NULL,
    course_id uuid NOT NULL,
    generation bigint NOT NULL,
    stage text NOT NULL,
    job_id uuid NOT NULL,
    state text NOT NULL,
    object_count bigint NOT NULL,
    prepared_at timestamp with time zone DEFAULT transaction_timestamp() NOT NULL,
    completed_at timestamp with time zone,
    CONSTRAINT course_retention_cleanup_manifest_generation_check CHECK ((generation > 0)),
    CONSTRAINT course_retention_cleanup_manifest_object_count_check CHECK ((object_count >= 0)),
    CONSTRAINT course_retention_cleanup_manifest_stage_check CHECK ((stage = ANY (ARRAY['archiveStudentRecords'::text, 'deleteStudentRecords'::text]))),
    CONSTRAINT course_retention_cleanup_manifest_state_check CHECK ((state = ANY (ARRAY['prepared'::text, 'completed'::text])))
);

ALTER TABLE ONLY public.course_retention_cleanup_manifest FORCE ROW LEVEL SECURITY;

CREATE TABLE public.course_retention_cleanup_manifest_object (
    tenant_id uuid NOT NULL,
    course_id uuid NOT NULL,
    generation bigint NOT NULL,
    stage text NOT NULL,
    object_id uuid NOT NULL,
    CONSTRAINT course_retention_cleanup_manifest_object_check CHECK (((tenant_id IS NOT NULL) AND (course_id IS NOT NULL) AND (object_id IS NOT NULL)))
);

ALTER TABLE ONLY public.course_retention_cleanup_manifest_object FORCE ROW LEVEL SECURITY;

CREATE TABLE public.course_retention_dispatch (
    tenant_id uuid NOT NULL,
    course_id uuid NOT NULL,
    stage text NOT NULL,
    generation bigint NOT NULL,
    job_id uuid NOT NULL,
    dispatched_at timestamp with time zone DEFAULT transaction_timestamp() NOT NULL,
    CONSTRAINT course_retention_dispatch_generation_check CHECK ((generation > 0)),
    CONSTRAINT course_retention_dispatch_stage_check CHECK ((stage = ANY (ARRAY['notify'::text, 'archiveStudentRecords'::text, 'deleteStudentRecords'::text])))
);

ALTER TABLE ONLY public.course_retention_dispatch FORCE ROW LEVEL SECURITY;

CREATE TABLE public.course_retention_notification (
    tenant_id uuid NOT NULL,
    course_id uuid NOT NULL,
    generation bigint NOT NULL,
    intent text NOT NULL,
    created_at timestamp with time zone DEFAULT transaction_timestamp() NOT NULL,
    CONSTRAINT course_retention_notification_generation_check CHECK ((generation > 0)),
    CONSTRAINT course_retention_notification_intent_check CHECK ((intent = ANY (ARRAY['archive'::text, 'delete'::text, 'extend'::text])))
);

ALTER TABLE ONLY public.course_retention_notification FORCE ROW LEVEL SECURITY;

CREATE TABLE public.course_retention_purge_attempt (
    tenant_id uuid NOT NULL,
    course_id uuid NOT NULL,
    generation bigint NOT NULL,
    stage text NOT NULL,
    attempt_id uuid NOT NULL,
    CONSTRAINT course_retention_purge_attempt_stage_check CHECK ((stage = 'deleteStudentRecords'::text))
);

ALTER TABLE ONLY public.course_retention_purge_attempt FORCE ROW LEVEL SECURITY;

CREATE TABLE public.course_retention_purge_export (
    tenant_id uuid NOT NULL,
    course_id uuid NOT NULL,
    generation bigint NOT NULL,
    stage text NOT NULL,
    export_id uuid NOT NULL,
    job_id uuid NOT NULL,
    CONSTRAINT course_retention_purge_export_stage_check CHECK ((stage = 'deleteStudentRecords'::text))
);

ALTER TABLE ONLY public.course_retention_purge_export FORCE ROW LEVEL SECURITY;

CREATE TABLE public.course_retention_purge_run (
    tenant_id uuid NOT NULL,
    course_id uuid NOT NULL,
    generation bigint NOT NULL,
    stage text NOT NULL,
    run_id uuid NOT NULL,
    CONSTRAINT course_retention_purge_run_stage_check CHECK ((stage = 'deleteStudentRecords'::text))
);

ALTER TABLE ONLY public.course_retention_purge_run FORCE ROW LEVEL SECURITY;

CREATE TABLE public.course_retention_stage (
    tenant_id uuid NOT NULL,
    course_id uuid NOT NULL,
    stage text NOT NULL,
    generation bigint NOT NULL,
    due_at timestamp with time zone NOT NULL,
    state text DEFAULT 'scheduled'::text NOT NULL,
    job_id uuid,
    lease_token uuid,
    claimed_at timestamp with time zone,
    CONSTRAINT course_retention_stage_generation_check CHECK ((generation > 0)),
    CONSTRAINT course_retention_stage_stage_check CHECK ((stage = ANY (ARRAY['notify'::text, 'archiveStudentRecords'::text, 'deleteStudentRecords'::text]))),
    CONSTRAINT course_retention_stage_state_check CHECK ((state = ANY (ARRAY['scheduled'::text, 'started'::text, 'completed'::text, 'superseded'::text])))
);

ALTER TABLE ONLY public.course_retention_stage FORCE ROW LEVEL SECURITY;

CREATE TABLE public.institution_retention_policy (
    tenant_id uuid NOT NULL,
    notify_days integer NOT NULL,
    archive_days integer NOT NULL,
    delete_days integer NOT NULL,
    assignment_disposition text DEFAULT 'retain'::text NOT NULL,
    CONSTRAINT institution_retention_policy_archive_days_check CHECK (((archive_days >= 1) AND (archive_days <= 36500))),
    CONSTRAINT institution_retention_policy_assignment_disposition_check CHECK ((assignment_disposition = ANY (ARRAY['retain'::text, 'delete'::text]))),
    CONSTRAINT institution_retention_policy_check CHECK (((notify_days < archive_days) AND (archive_days < delete_days))),
    CONSTRAINT institution_retention_policy_delete_days_check CHECK (((delete_days >= 1) AND (delete_days <= 36500))),
    CONSTRAINT institution_retention_policy_notify_days_check CHECK (((notify_days >= 1) AND (notify_days <= 36500)))
);

ALTER TABLE ONLY public.institution_retention_policy FORCE ROW LEVEL SECURITY;

ALTER TABLE ONLY public.course_retention_api_receipt
    ADD CONSTRAINT course_retention_api_receipt_pkey PRIMARY KEY (tenant_id, course_id, expected_generation);

ALTER TABLE ONLY public.course_retention_cleanup_manifest_object
    ADD CONSTRAINT course_retention_cleanup_manifest_object_pkey PRIMARY KEY (tenant_id, course_id, generation, stage, object_id);

ALTER TABLE ONLY public.course_retention_cleanup_manifest
    ADD CONSTRAINT course_retention_cleanup_manifest_pkey PRIMARY KEY (tenant_id, course_id, generation, stage);

ALTER TABLE ONLY public.course_retention_dispatch
    ADD CONSTRAINT course_retention_dispatch_job_id_key UNIQUE (job_id);

ALTER TABLE ONLY public.course_retention_dispatch
    ADD CONSTRAINT course_retention_dispatch_pkey PRIMARY KEY (tenant_id, course_id, stage, generation);

ALTER TABLE ONLY public.course_retention_notification
    ADD CONSTRAINT course_retention_notification_pkey PRIMARY KEY (tenant_id, course_id, generation);

ALTER TABLE ONLY public.course_retention
    ADD CONSTRAINT course_retention_pkey PRIMARY KEY (tenant_id, course_id);

ALTER TABLE ONLY public.course_retention_purge_attempt
    ADD CONSTRAINT course_retention_purge_attempt_pkey PRIMARY KEY (tenant_id, course_id, generation, stage, attempt_id);

ALTER TABLE ONLY public.course_retention_purge_export
    ADD CONSTRAINT course_retention_purge_export_pkey PRIMARY KEY (tenant_id, course_id, generation, stage, export_id);

ALTER TABLE ONLY public.course_retention_purge_export
    ADD CONSTRAINT course_retention_purge_export_tenant_id_course_id_generatio_key UNIQUE (tenant_id, course_id, generation, stage, job_id);

ALTER TABLE ONLY public.course_retention_purge_run
    ADD CONSTRAINT course_retention_purge_run_pkey PRIMARY KEY (tenant_id, course_id, generation, stage, run_id);

ALTER TABLE ONLY public.course_retention_stage
    ADD CONSTRAINT course_retention_stage_pkey PRIMARY KEY (tenant_id, course_id, stage, generation);

ALTER TABLE ONLY public.institution_retention_policy
    ADD CONSTRAINT institution_retention_policy_pkey PRIMARY KEY (tenant_id);

CREATE INDEX course_retention_cleanup_manifest_object_count_idx ON public.course_retention_cleanup_manifest USING btree (tenant_id, course_id, generation, stage, object_count) WHERE (state = 'prepared'::text);

CREATE INDEX course_retention_stage_due_idx ON public.course_retention_stage USING btree (due_at, tenant_id, course_id) WHERE (state = 'scheduled'::text);

CREATE INDEX course_retention_stage_work_idx ON public.course_retention_stage USING btree (tenant_id, course_id, generation, stage) WHERE (state = ANY (ARRAY['scheduled'::text, 'started'::text]));

ALTER TABLE ONLY public.course_retention_api_receipt
    ADD CONSTRAINT course_retention_api_receipt_tenant_id_course_id_fkey FOREIGN KEY (tenant_id, course_id) REFERENCES public.course_retention(tenant_id, course_id) ON DELETE CASCADE;

ALTER TABLE ONLY public.course_retention_cleanup_manifest_object
    ADD CONSTRAINT course_retention_cleanup_man_tenant_id_course_id_generati_fkey1 FOREIGN KEY (tenant_id, course_id, generation, stage) REFERENCES public.course_retention_cleanup_manifest(tenant_id, course_id, generation, stage) ON DELETE CASCADE;

ALTER TABLE ONLY public.course_retention_cleanup_manifest
    ADD CONSTRAINT course_retention_cleanup_mani_tenant_id_course_id_generati_fkey FOREIGN KEY (tenant_id, course_id, generation, stage) REFERENCES public.course_retention_stage(tenant_id, course_id, generation, stage) ON DELETE CASCADE;

ALTER TABLE ONLY public.course_retention_cleanup_manifest
    ADD CONSTRAINT course_retention_cleanup_manifest_job_id_fkey FOREIGN KEY (job_id) REFERENCES public.worker_job(job_id);

ALTER TABLE ONLY public.course_retention_cleanup_manifest
    ADD CONSTRAINT course_retention_cleanup_manifest_tenant_id_course_id_fkey FOREIGN KEY (tenant_id, course_id) REFERENCES public.course(tenant_id, course_id);

ALTER TABLE ONLY public.course_retention_dispatch
    ADD CONSTRAINT course_retention_dispatch_job_id_fkey FOREIGN KEY (job_id) REFERENCES public.worker_job(job_id) DEFERRABLE INITIALLY DEFERRED;

ALTER TABLE ONLY public.course_retention_dispatch
    ADD CONSTRAINT course_retention_dispatch_tenant_id_course_id_stage_genera_fkey FOREIGN KEY (tenant_id, course_id, stage, generation) REFERENCES public.course_retention_stage(tenant_id, course_id, stage, generation);

ALTER TABLE ONLY public.course_retention_notification
    ADD CONSTRAINT course_retention_notification_tenant_id_course_id_fkey FOREIGN KEY (tenant_id, course_id) REFERENCES public.course_retention(tenant_id, course_id) ON DELETE CASCADE;

ALTER TABLE ONLY public.course_retention_purge_attempt
    ADD CONSTRAINT course_retention_purge_attemp_tenant_id_course_id_generati_fkey FOREIGN KEY (tenant_id, course_id, generation, stage) REFERENCES public.course_retention_cleanup_manifest(tenant_id, course_id, generation, stage) ON DELETE CASCADE;

ALTER TABLE ONLY public.course_retention_purge_export
    ADD CONSTRAINT course_retention_purge_export_tenant_id_course_id_generati_fkey FOREIGN KEY (tenant_id, course_id, generation, stage) REFERENCES public.course_retention_cleanup_manifest(tenant_id, course_id, generation, stage) ON DELETE CASCADE;

ALTER TABLE ONLY public.course_retention_purge_run
    ADD CONSTRAINT course_retention_purge_run_tenant_id_course_id_generation__fkey FOREIGN KEY (tenant_id, course_id, generation, stage) REFERENCES public.course_retention_cleanup_manifest(tenant_id, course_id, generation, stage) ON DELETE CASCADE;

ALTER TABLE ONLY public.course_retention_stage
    ADD CONSTRAINT course_retention_stage_tenant_id_course_id_fkey FOREIGN KEY (tenant_id, course_id) REFERENCES public.course_retention(tenant_id, course_id) ON DELETE CASCADE;

ALTER TABLE ONLY public.course_retention
    ADD CONSTRAINT course_retention_tenant_id_course_id_fkey FOREIGN KEY (tenant_id, course_id) REFERENCES public.course(tenant_id, course_id);

ALTER TABLE public.course_retention ENABLE ROW LEVEL SECURITY;

ALTER TABLE public.course_retention_api_receipt ENABLE ROW LEVEL SECURITY;

ALTER TABLE public.course_retention_cleanup_manifest ENABLE ROW LEVEL SECURITY;

ALTER TABLE public.course_retention_cleanup_manifest_object ENABLE ROW LEVEL SECURITY;

ALTER TABLE public.course_retention_dispatch ENABLE ROW LEVEL SECURITY;

ALTER TABLE public.course_retention_notification ENABLE ROW LEVEL SECURITY;

CREATE POLICY course_retention_notification_tenant ON public.course_retention_notification USING ((tenant_id = public.ple_current_tenant())) WITH CHECK ((tenant_id = public.ple_current_tenant()));

ALTER TABLE public.course_retention_purge_attempt ENABLE ROW LEVEL SECURITY;

ALTER TABLE public.course_retention_purge_export ENABLE ROW LEVEL SECURITY;

ALTER TABLE public.course_retention_purge_run ENABLE ROW LEVEL SECURITY;

ALTER TABLE public.course_retention_stage ENABLE ROW LEVEL SECURITY;

CREATE POLICY course_retention_stage_tenant ON public.course_retention_stage USING ((tenant_id = public.ple_current_tenant())) WITH CHECK ((tenant_id = public.ple_current_tenant()));

CREATE POLICY course_retention_tenant ON public.course_retention USING ((tenant_id = public.ple_current_tenant())) WITH CHECK ((tenant_id = public.ple_current_tenant()));

ALTER TABLE public.institution_retention_policy ENABLE ROW LEVEL SECURITY;

CREATE POLICY institution_retention_policy_tenant ON public.institution_retention_policy USING ((tenant_id = public.ple_current_tenant())) WITH CHECK ((tenant_id = public.ple_current_tenant()));

CREATE POLICY retention_api_receipt_tenant ON public.course_retention_api_receipt USING ((tenant_id = public.ple_current_tenant())) WITH CHECK ((tenant_id = public.ple_current_tenant()));

CREATE POLICY retention_broker_api_receipt ON public.course_retention_api_receipt TO ple_retention_broker USING ((tenant_id = public.ple_current_tenant())) WITH CHECK ((tenant_id = public.ple_current_tenant()));

CREATE POLICY retention_broker_course_schedule ON public.course_retention FOR SELECT TO ple_retention_broker USING (true);

CREATE POLICY retention_broker_dispatch ON public.course_retention_dispatch TO ple_retention_broker USING (true) WITH CHECK (true);

CREATE POLICY retention_broker_stage_schedule ON public.course_retention_stage TO ple_retention_broker USING (true) WITH CHECK (true);

CREATE POLICY retention_cleanup_manifest_broker ON public.course_retention_cleanup_manifest TO ple_retention_broker USING ((tenant_id = public.ple_current_tenant())) WITH CHECK ((tenant_id = public.ple_current_tenant()));

CREATE POLICY retention_cleanup_manifest_object_broker ON public.course_retention_cleanup_manifest_object TO ple_retention_broker USING ((tenant_id = public.ple_current_tenant())) WITH CHECK ((tenant_id = public.ple_current_tenant()));

CREATE POLICY retention_dispatch_tenant ON public.course_retention_dispatch USING ((tenant_id = public.ple_current_tenant())) WITH CHECK ((tenant_id = public.ple_current_tenant()));

CREATE POLICY retention_purge_attempt_broker ON public.course_retention_purge_attempt TO ple_retention_broker USING ((tenant_id = public.ple_current_tenant())) WITH CHECK ((tenant_id = public.ple_current_tenant()));

CREATE POLICY retention_purge_export_broker ON public.course_retention_purge_export TO ple_retention_broker USING ((tenant_id = public.ple_current_tenant())) WITH CHECK ((tenant_id = public.ple_current_tenant()));

CREATE POLICY retention_purge_run_broker ON public.course_retention_purge_run TO ple_retention_broker USING ((tenant_id = public.ple_current_tenant())) WITH CHECK ((tenant_id = public.ple_current_tenant()));

GRANT SELECT,INSERT,UPDATE ON TABLE public.course_retention TO ple_retention_broker;

GRANT SELECT,INSERT ON TABLE public.course_retention_api_receipt TO ple_retention_broker;

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.course_retention_cleanup_manifest TO ple_retention_broker;

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.course_retention_cleanup_manifest_object TO ple_retention_broker;

GRANT SELECT,INSERT,UPDATE ON TABLE public.course_retention_dispatch TO ple_retention_broker;

GRANT SELECT,INSERT ON TABLE public.course_retention_notification TO ple_retention_broker;

GRANT SELECT,INSERT,DELETE ON TABLE public.course_retention_purge_attempt TO ple_retention_broker;

GRANT SELECT,INSERT,DELETE ON TABLE public.course_retention_purge_export TO ple_retention_broker;

GRANT SELECT,INSERT,DELETE ON TABLE public.course_retention_purge_run TO ple_retention_broker;

GRANT SELECT,INSERT,UPDATE ON TABLE public.course_retention_stage TO ple_retention_broker;

GRANT SELECT,INSERT,UPDATE ON TABLE public.institution_retention_policy TO ple_retention_broker;

REVOKE ALL ON FUNCTION public.ple_apply_retention_api_action(p_session character, p_course uuid, p_expected_generation bigint, p_action text, p_days integer, p_disposition text) FROM PUBLIC;
GRANT ALL ON FUNCTION public.ple_apply_retention_api_action(p_session character, p_course uuid, p_expected_generation bigint, p_action text, p_days integer, p_disposition text) TO ple_app;

REVOKE ALL ON FUNCTION public.ple_bind_course_from_attempt() FROM PUBLIC;

REVOKE ALL ON FUNCTION public.ple_bind_course_item_analysis_course() FROM PUBLIC;

REVOKE ALL ON FUNCTION public.ple_bind_question_attempt_course() FROM PUBLIC;

REVOKE ALL ON FUNCTION public.ple_commit_retention_work(p_tenant uuid, p_job uuid, p_token uuid, p_course uuid, p_stage text, p_generation bigint) FROM PUBLIC;
GRANT ALL ON FUNCTION public.ple_commit_retention_work(p_tenant uuid, p_job uuid, p_token uuid, p_course uuid, p_stage text, p_generation bigint) TO ple_app;

REVOKE ALL ON FUNCTION public.ple_configure_retention_policy(p_session character, p_notify integer, p_archive integer, p_delete integer) FROM PUBLIC;
GRANT ALL ON FUNCTION public.ple_configure_retention_policy(p_session character, p_notify integer, p_archive integer, p_delete integer) TO ple_app;

REVOKE ALL ON FUNCTION public.ple_course_records_accessible(p_tenant uuid, p_course uuid) FROM PUBLIC;
GRANT ALL ON FUNCTION public.ple_course_records_accessible(p_tenant uuid, p_course uuid) TO ple_app;
GRANT ALL ON FUNCTION public.ple_course_records_accessible(p_tenant uuid, p_course uuid) TO ple_student;
GRANT ALL ON FUNCTION public.ple_course_records_accessible(p_tenant uuid, p_course uuid) TO ple_queue_broker;

REVOKE ALL ON FUNCTION public.ple_dispatch_due_retention_stages(p_batch integer) FROM PUBLIC;
GRANT ALL ON FUNCTION public.ple_dispatch_due_retention_stages(p_batch integer) TO ple_app;

REVOKE ALL ON FUNCTION public.ple_end_course_retention(p_session character, p_course uuid) FROM PUBLIC;
GRANT ALL ON FUNCTION public.ple_end_course_retention(p_session character, p_course uuid) TO ple_app;

REVOKE ALL ON FUNCTION public.ple_extend_course_retention(p_session character, p_course uuid, p_days integer) FROM PUBLIC;
GRANT ALL ON FUNCTION public.ple_extend_course_retention(p_session character, p_course uuid, p_days integer) TO ple_app;

REVOKE ALL ON FUNCTION public.ple_fence_assignment_definition_write() FROM PUBLIC;

REVOKE ALL ON FUNCTION public.ple_fence_learner_record_write() FROM PUBLIC;

REVOKE ALL ON FUNCTION public.ple_lock_course_write(p_tenant uuid, p_course uuid, p_definition boolean) FROM PUBLIC;

REVOKE ALL ON FUNCTION public.ple_prepare_retention_work(p_tenant uuid, p_job uuid, p_token uuid, p_course uuid, p_stage text, p_generation bigint) FROM PUBLIC;
GRANT ALL ON FUNCTION public.ple_prepare_retention_work(p_tenant uuid, p_job uuid, p_token uuid, p_course uuid, p_stage text, p_generation bigint) TO ple_app;

REVOKE ALL ON FUNCTION public.ple_read_course_retention(p_session character, p_course uuid) FROM PUBLIC;
GRANT ALL ON FUNCTION public.ple_read_course_retention(p_session character, p_course uuid) TO ple_app;

REVOKE ALL ON FUNCTION public.ple_read_retention_notification(p_session character, p_course uuid) FROM PUBLIC;
GRANT ALL ON FUNCTION public.ple_read_retention_notification(p_session character, p_course uuid) TO ple_app;

REVOKE ALL ON FUNCTION public.ple_retention_authorize(p_session character, p_course uuid, p_admin_only boolean) FROM PUBLIC;
-- Shared course-manager authorization predicate for tenant-scoped read APIs.
GRANT EXECUTE ON FUNCTION public.ple_retention_authorize(p_session character, p_course uuid, p_admin_only boolean) TO ple_app;

REVOKE ALL ON FUNCTION public.ple_set_archive_disposition(p_session character, p_course uuid, p_disposition text) FROM PUBLIC;
GRANT ALL ON FUNCTION public.ple_set_archive_disposition(p_session character, p_course uuid, p_disposition text) TO ple_app;

REVOKE ALL ON FUNCTION public.ple_commit_archive_retention_work(p_tenant uuid, p_job uuid, p_token uuid, p_course uuid, p_stage text, p_generation bigint) FROM PUBLIC;

REVOKE ALL ON FUNCTION public.ple_prepare_archive_retention_work(p_tenant uuid, p_job uuid, p_token uuid, p_course uuid, p_stage text, p_generation bigint) FROM PUBLIC;

REVOKE ALL ON FUNCTION public.ple_commit_delete_retention_work(p_tenant uuid, p_job uuid, p_token uuid, p_course uuid, p_stage text, p_generation bigint) FROM PUBLIC;

REVOKE ALL ON FUNCTION public.ple_prepare_delete_retention_work(p_tenant uuid, p_job uuid, p_token uuid, p_course uuid, p_stage text, p_generation bigint) FROM PUBLIC;

CREATE TRIGGER asset_delivery_retention_fence BEFORE INSERT OR UPDATE ON public.asset_delivery FOR EACH ROW EXECUTE FUNCTION public.ple_fence_learner_record_write();

CREATE TRIGGER assignment_item_retention_fence BEFORE INSERT OR UPDATE ON public.assignment_item FOR EACH ROW EXECUTE FUNCTION public.ple_fence_assignment_definition_write();

CREATE TRIGGER assignment_retention_fence BEFORE INSERT OR UPDATE ON public.assignment FOR EACH ROW EXECUTE FUNCTION public.ple_fence_assignment_definition_write();

CREATE TRIGGER assignment_run_retention_fence BEFORE INSERT OR UPDATE ON public.assignment_run FOR EACH ROW EXECUTE FUNCTION public.ple_fence_learner_record_write();

CREATE TRIGGER assignment_run_item_retention_fence BEFORE INSERT OR UPDATE ON public.assignment_run_item FOR EACH ROW EXECUTE FUNCTION public.ple_fence_learner_record_write();

CREATE TRIGGER course_item_analysis_current_bind_course BEFORE INSERT OR UPDATE ON public.course_item_analysis_current FOR EACH ROW EXECUTE FUNCTION public.ple_bind_course_item_analysis_course();

CREATE TRIGGER course_item_analysis_current_retention_fence BEFORE INSERT OR UPDATE ON public.course_item_analysis_current FOR EACH ROW EXECUTE FUNCTION public.ple_fence_learner_record_write();

CREATE TRIGGER course_item_analysis_staging_bind_course BEFORE INSERT OR UPDATE ON public.course_item_analysis_staging FOR EACH ROW EXECUTE FUNCTION public.ple_bind_course_item_analysis_course();

CREATE TRIGGER course_item_analysis_staging_retention_fence BEFORE INSERT OR UPDATE ON public.course_item_analysis_staging FOR EACH ROW EXECUTE FUNCTION public.ple_fence_learner_record_write();

CREATE TRIGGER attempt_feedback_bind_course BEFORE INSERT OR UPDATE ON public.attempt_feedback FOR EACH ROW EXECUTE FUNCTION public.ple_bind_course_from_attempt();

CREATE TRIGGER attempt_feedback_retention_fence BEFORE INSERT OR UPDATE ON public.attempt_feedback FOR EACH ROW EXECUTE FUNCTION public.ple_fence_learner_record_write();

CREATE TRIGGER record_access_log_retention_fence BEFORE INSERT OR UPDATE ON public.record_access_log FOR EACH ROW EXECUTE FUNCTION public.ple_fence_learner_record_write();

CREATE TRIGGER audit_event_retention_fence BEFORE INSERT OR UPDATE ON public.audit_event FOR EACH ROW EXECUTE FUNCTION public.ple_fence_learner_record_write();

CREATE TRIGGER course_member_retention_fence BEFORE INSERT OR UPDATE ON public.course_member FOR EACH ROW EXECUTE FUNCTION public.ple_fence_learner_record_write();

CREATE TRIGGER course_group_member_retention_fence BEFORE INSERT OR UPDATE ON public.course_group_member FOR EACH ROW EXECUTE FUNCTION public.ple_fence_learner_record_write();

CREATE TRIGGER assignment_policy_exception_retention_fence BEFORE INSERT OR UPDATE ON public.assignment_policy_exception FOR EACH ROW EXECUTE FUNCTION public.ple_fence_learner_record_write();

CREATE TRIGGER enrollment_retention_fence BEFORE INSERT OR UPDATE ON public.enrollment FOR EACH ROW EXECUTE FUNCTION public.ple_fence_learner_record_write();

CREATE TRIGGER external_tool_exchange_bind_course BEFORE INSERT OR UPDATE ON public.external_tool_exchange FOR EACH ROW EXECUTE FUNCTION public.ple_bind_course_from_attempt();

CREATE TRIGGER external_tool_exchange_retention_fence BEFORE INSERT OR UPDATE ON public.external_tool_exchange FOR EACH ROW EXECUTE FUNCTION public.ple_fence_learner_record_write();

CREATE TRIGGER external_tool_launch_session_bind_course BEFORE INSERT OR UPDATE ON public.external_tool_launch_session FOR EACH ROW EXECUTE FUNCTION public.ple_bind_course_from_attempt();

CREATE TRIGGER external_tool_launch_session_retention_fence BEFORE INSERT OR UPDATE ON public.external_tool_launch_session FOR EACH ROW EXECUTE FUNCTION public.ple_fence_learner_record_write();

CREATE TRIGGER feedback_release_retention_fence BEFORE INSERT OR UPDATE ON public.feedback_release FOR EACH ROW EXECUTE FUNCTION public.ple_fence_learner_record_write();

CREATE TRIGGER submission_evaluation_bind_course BEFORE INSERT OR UPDATE ON public.submission_evaluation FOR EACH ROW EXECUTE FUNCTION public.ple_bind_course_from_attempt();

CREATE TRIGGER submission_evaluation_retention_fence BEFORE INSERT OR UPDATE ON public.submission_evaluation FOR EACH ROW EXECUTE FUNCTION public.ple_fence_learner_record_write();

CREATE TRIGGER manual_grade_receipt_bind_course BEFORE INSERT OR UPDATE ON public.manual_grade_receipt FOR EACH ROW EXECUTE FUNCTION public.ple_bind_course_from_attempt();

CREATE TRIGGER manual_grade_receipt_retention_fence BEFORE INSERT OR UPDATE ON public.manual_grade_receipt FOR EACH ROW EXECUTE FUNCTION public.ple_fence_learner_record_write();

CREATE TRIGGER attempt_score_current_retention_fence BEFORE INSERT OR UPDATE ON public.attempt_score_current FOR EACH ROW EXECUTE FUNCTION public.ple_fence_learner_record_write();

CREATE TRIGGER attempt_timing_current_retention_fence BEFORE INSERT OR UPDATE ON public.attempt_timing_current FOR EACH ROW EXECUTE FUNCTION public.ple_fence_learner_record_write();

CREATE TRIGGER question_attempt_bind_course BEFORE INSERT OR UPDATE ON public.question_attempt FOR EACH ROW EXECUTE FUNCTION public.ple_bind_question_attempt_course();

CREATE TRIGGER question_attempt_retention_fence BEFORE INSERT OR UPDATE ON public.question_attempt FOR EACH ROW EXECUTE FUNCTION public.ple_fence_learner_record_write();

CREATE TRIGGER question_prefetch_retention_fence BEFORE INSERT OR UPDATE ON public.question_prefetch FOR EACH ROW EXECUTE FUNCTION public.ple_fence_learner_record_write();

CREATE TRIGGER statistics_receipt_retention_fence BEFORE INSERT OR UPDATE ON public.question_statistics_contribution_receipt FOR EACH ROW EXECUTE FUNCTION public.ple_fence_learner_record_write();

CREATE TRIGGER student_assignment_summary_retention_fence BEFORE INSERT OR UPDATE ON public.student_assignment_summary FOR EACH ROW EXECUTE FUNCTION public.ple_fence_learner_record_write();

CREATE TRIGGER student_export_artifact_retention_fence BEFORE INSERT OR UPDATE ON public.student_export_artifact FOR EACH ROW EXECUTE FUNCTION public.ple_fence_learner_record_write();

CREATE TRIGGER student_export_request_retention_fence BEFORE INSERT ON public.student_export_request FOR EACH ROW EXECUTE FUNCTION public.ple_fence_learner_record_write();

CREATE TRIGGER submission_bind_course BEFORE INSERT OR UPDATE ON public.submission FOR EACH ROW EXECUTE FUNCTION public.ple_bind_course_from_attempt();

CREATE TRIGGER submission_idempotency_bind_course BEFORE INSERT OR UPDATE ON public.submission_idempotency FOR EACH ROW EXECUTE FUNCTION public.ple_bind_course_from_attempt();

CREATE TRIGGER submission_idempotency_retention_fence BEFORE INSERT OR UPDATE ON public.submission_idempotency FOR EACH ROW EXECUTE FUNCTION public.ple_fence_learner_record_write();

CREATE TRIGGER submission_next_attempt_retention_fence BEFORE INSERT OR UPDATE ON public.submission_next_attempt FOR EACH ROW EXECUTE FUNCTION public.ple_fence_learner_record_write();

CREATE TRIGGER submission_receipt_snapshot_retention_fence BEFORE INSERT OR UPDATE ON public.submission_receipt_snapshot FOR EACH ROW EXECUTE FUNCTION public.ple_fence_learner_record_write();

CREATE TRIGGER submission_retention_fence BEFORE INSERT OR UPDATE ON public.submission FOR EACH ROW EXECUTE FUNCTION public.ple_fence_learner_record_write();

CREATE POLICY asset_delivery_app_insert ON public.asset_delivery FOR INSERT TO ple_app WITH CHECK ((((delivery_kind = 'catalog'::text) AND (course_id IS NULL) AND (EXISTS ( SELECT 1
   FROM public.problem_version visible_version
  WHERE ((visible_version.problem_id = asset_delivery.problem_id) AND (visible_version.version_id = asset_delivery.version_id))))) OR ((delivery_kind = 'student_record'::text) AND (tenant_id = public.ple_current_tenant()) AND public.ple_course_records_accessible(tenant_id, course_id))));

CREATE POLICY asset_delivery_export_broker_insert ON public.asset_delivery FOR INSERT TO ple_queue_broker WITH CHECK (((delivery_kind = 'student_record'::text) AND (tenant_id = public.ple_current_tenant()) AND public.ple_course_records_accessible(tenant_id, course_id)));

CREATE POLICY asset_delivery_visible_select ON public.asset_delivery FOR SELECT TO ple_app USING ((((delivery_kind = 'catalog'::text) AND (EXISTS ( SELECT 1
   FROM public.problem_version visible_version
  WHERE ((visible_version.problem_id = asset_delivery.problem_id) AND (visible_version.version_id = asset_delivery.version_id))))) OR ((delivery_kind = 'student_record'::text) AND (tenant_id = public.ple_current_tenant()) AND public.ple_course_records_accessible(tenant_id, course_id))));

CREATE POLICY assignment_item_student_records ON public.assignment_item FOR SELECT TO ple_student USING (((tenant_id = public.ple_current_tenant()) AND (EXISTS ( SELECT 1
   FROM public.assignment a
  WHERE ((a.tenant_id = assignment_item.tenant_id) AND (a.assignment_id = assignment_item.assignment_id) AND public.ple_course_records_accessible(a.tenant_id, a.course_id))))));

CREATE POLICY assignment_run_tenant ON public.assignment_run USING (((tenant_id = public.ple_current_tenant()) AND (EXISTS ( SELECT 1
   FROM (public.enrollment e
     JOIN public.assignment a ON (((a.tenant_id = e.tenant_id) AND (a.assignment_id = e.assignment_id))))
  WHERE ((e.tenant_id = assignment_run.tenant_id) AND (e.enrollment_id = assignment_run.enrollment_id) AND public.ple_course_records_accessible(a.tenant_id, a.course_id)))))) WITH CHECK (((tenant_id = public.ple_current_tenant()) AND (EXISTS ( SELECT 1
   FROM (public.enrollment e
     JOIN public.assignment a ON (((a.tenant_id = e.tenant_id) AND (a.assignment_id = e.assignment_id))))
  WHERE ((e.tenant_id = assignment_run.tenant_id) AND (e.enrollment_id = assignment_run.enrollment_id) AND public.ple_course_records_accessible(a.tenant_id, a.course_id))))));

CREATE POLICY assignment_run_item_tenant ON public.assignment_run_item USING (((tenant_id = public.ple_current_tenant()) AND (EXISTS ( SELECT 1
   FROM public.assignment_run r
  WHERE ((r.tenant_id = assignment_run_item.tenant_id) AND (r.run_id = assignment_run_item.run_id)))))) WITH CHECK (((tenant_id = public.ple_current_tenant()) AND (EXISTS ( SELECT 1
   FROM public.assignment_run r
  WHERE ((r.tenant_id = assignment_run_item.tenant_id) AND (r.run_id = assignment_run_item.run_id))))));

CREATE POLICY assignment_student_records ON public.assignment FOR SELECT TO ple_student USING (((tenant_id = public.ple_current_tenant()) AND public.ple_course_records_accessible(tenant_id, course_id)));

CREATE POLICY attempt_feedback_tenant ON public.attempt_feedback USING (((tenant_id = public.ple_current_tenant()) AND (EXISTS ( SELECT 1
   FROM (((public.question_attempt q
     JOIN public.assignment_run r ON (((r.tenant_id = q.tenant_id) AND (r.run_id = q.run_id))))
     JOIN public.enrollment e ON (((e.tenant_id = r.tenant_id) AND (e.enrollment_id = r.enrollment_id))))
     JOIN public.assignment a ON (((a.tenant_id = e.tenant_id) AND (a.assignment_id = e.assignment_id))))
  WHERE ((q.tenant_id = attempt_feedback.tenant_id) AND (q.attempt_id = attempt_feedback.attempt_id) AND public.ple_course_records_accessible(a.tenant_id, a.course_id)))))) WITH CHECK (((tenant_id = public.ple_current_tenant()) AND (EXISTS ( SELECT 1
   FROM (((public.question_attempt q
     JOIN public.assignment_run r ON (((r.tenant_id = q.tenant_id) AND (r.run_id = q.run_id))))
     JOIN public.enrollment e ON (((e.tenant_id = r.tenant_id) AND (e.enrollment_id = r.enrollment_id))))
     JOIN public.assignment a ON (((a.tenant_id = e.tenant_id) AND (a.assignment_id = e.assignment_id))))
  WHERE ((q.tenant_id = attempt_feedback.tenant_id) AND (q.attempt_id = attempt_feedback.attempt_id) AND public.ple_course_records_accessible(a.tenant_id, a.course_id))))));

CREATE POLICY enrollment_tenant ON public.enrollment USING (((tenant_id = public.ple_current_tenant()) AND (EXISTS ( SELECT 1
   FROM public.assignment a
  WHERE ((a.tenant_id = enrollment.tenant_id) AND (a.assignment_id = enrollment.assignment_id) AND public.ple_course_records_accessible(a.tenant_id, a.course_id)))))) WITH CHECK (((tenant_id = public.ple_current_tenant()) AND (EXISTS ( SELECT 1
   FROM public.assignment a
  WHERE ((a.tenant_id = enrollment.tenant_id) AND (a.assignment_id = enrollment.assignment_id) AND public.ple_course_records_accessible(a.tenant_id, a.course_id))))));

CREATE POLICY external_tool_exchange_tenant ON public.external_tool_exchange USING (((tenant_id = public.ple_current_tenant()) AND (EXISTS ( SELECT 1
   FROM (((public.question_attempt q
     JOIN public.assignment_run r ON (((r.tenant_id = q.tenant_id) AND (r.run_id = q.run_id))))
     JOIN public.enrollment e ON (((e.tenant_id = r.tenant_id) AND (e.enrollment_id = r.enrollment_id))))
     JOIN public.assignment a ON (((a.tenant_id = e.tenant_id) AND (a.assignment_id = e.assignment_id))))
  WHERE ((q.tenant_id = external_tool_exchange.tenant_id) AND (q.attempt_id = external_tool_exchange.attempt_id) AND public.ple_course_records_accessible(a.tenant_id, a.course_id)))))) WITH CHECK (((tenant_id = public.ple_current_tenant()) AND (EXISTS ( SELECT 1
   FROM (((public.question_attempt q
     JOIN public.assignment_run r ON (((r.tenant_id = q.tenant_id) AND (r.run_id = q.run_id))))
     JOIN public.enrollment e ON (((e.tenant_id = r.tenant_id) AND (e.enrollment_id = r.enrollment_id))))
     JOIN public.assignment a ON (((a.tenant_id = e.tenant_id) AND (a.assignment_id = e.assignment_id))))
  WHERE ((q.tenant_id = external_tool_exchange.tenant_id) AND (q.attempt_id = external_tool_exchange.attempt_id) AND public.ple_course_records_accessible(a.tenant_id, a.course_id))))));

CREATE POLICY external_tool_launch_session_tenant ON public.external_tool_launch_session USING (((tenant_id = public.ple_current_tenant()) AND (EXISTS ( SELECT 1
   FROM (((public.question_attempt q
     JOIN public.assignment_run r ON (((r.tenant_id = q.tenant_id) AND (r.run_id = q.run_id))))
     JOIN public.enrollment e ON (((e.tenant_id = r.tenant_id) AND (e.enrollment_id = r.enrollment_id))))
     JOIN public.assignment a ON (((a.tenant_id = e.tenant_id) AND (a.assignment_id = e.assignment_id))))
  WHERE ((q.tenant_id = external_tool_launch_session.tenant_id) AND (q.attempt_id = external_tool_launch_session.attempt_id) AND public.ple_course_records_accessible(a.tenant_id, a.course_id)))))) WITH CHECK (((tenant_id = public.ple_current_tenant()) AND (EXISTS ( SELECT 1
   FROM (((public.question_attempt q
     JOIN public.assignment_run r ON (((r.tenant_id = q.tenant_id) AND (r.run_id = q.run_id))))
     JOIN public.enrollment e ON (((e.tenant_id = r.tenant_id) AND (e.enrollment_id = r.enrollment_id))))
     JOIN public.assignment a ON (((a.tenant_id = e.tenant_id) AND (a.assignment_id = e.assignment_id))))
  WHERE ((q.tenant_id = external_tool_launch_session.tenant_id) AND (q.attempt_id = external_tool_launch_session.attempt_id) AND public.ple_course_records_accessible(a.tenant_id, a.course_id))))));

CREATE POLICY question_attempt_tenant ON public.question_attempt USING (((tenant_id = public.ple_current_tenant()) AND (EXISTS ( SELECT 1
   FROM ((public.assignment_run r
     JOIN public.enrollment e ON (((e.tenant_id = r.tenant_id) AND (e.enrollment_id = r.enrollment_id))))
     JOIN public.assignment a ON (((a.tenant_id = e.tenant_id) AND (a.assignment_id = e.assignment_id))))
  WHERE ((r.tenant_id = question_attempt.tenant_id) AND (r.run_id = question_attempt.run_id) AND public.ple_course_records_accessible(a.tenant_id, a.course_id)))))) WITH CHECK (((tenant_id = public.ple_current_tenant()) AND (EXISTS ( SELECT 1
   FROM ((public.assignment_run r
     JOIN public.enrollment e ON (((e.tenant_id = r.tenant_id) AND (e.enrollment_id = r.enrollment_id))))
     JOIN public.assignment a ON (((a.tenant_id = e.tenant_id) AND (a.assignment_id = e.assignment_id))))
  WHERE ((r.tenant_id = question_attempt.tenant_id) AND (r.run_id = question_attempt.run_id) AND public.ple_course_records_accessible(a.tenant_id, a.course_id))))));

CREATE POLICY question_prefetch_tenant ON public.question_prefetch USING (((tenant_id = public.ple_current_tenant()) AND (EXISTS ( SELECT 1
   FROM ((public.assignment_run r
     JOIN public.enrollment e ON (((e.tenant_id = r.tenant_id) AND (e.enrollment_id = r.enrollment_id))))
     JOIN public.assignment a ON (((a.tenant_id = e.tenant_id) AND (a.assignment_id = e.assignment_id))))
  WHERE ((r.tenant_id = question_prefetch.tenant_id) AND (r.run_id = question_prefetch.run_id) AND public.ple_course_records_accessible(a.tenant_id, a.course_id)))))) WITH CHECK (((tenant_id = public.ple_current_tenant()) AND (EXISTS ( SELECT 1
   FROM ((public.assignment_run r
     JOIN public.enrollment e ON (((e.tenant_id = r.tenant_id) AND (e.enrollment_id = r.enrollment_id))))
     JOIN public.assignment a ON (((a.tenant_id = e.tenant_id) AND (a.assignment_id = e.assignment_id))))
  WHERE ((r.tenant_id = question_prefetch.tenant_id) AND (r.run_id = question_prefetch.run_id) AND public.ple_course_records_accessible(a.tenant_id, a.course_id))))));

CREATE POLICY retention_broker_asset_delivery ON public.asset_delivery FOR SELECT TO ple_retention_broker USING (((delivery_kind = 'student_record'::text) AND (tenant_id = public.ple_current_tenant())));

CREATE POLICY retention_broker_asset_delivery_delete ON public.asset_delivery FOR DELETE TO ple_retention_broker USING (((delivery_kind = 'student_record'::text) AND (tenant_id = public.ple_current_tenant())));

CREATE POLICY retention_broker_asset_delivery_tenant_delete ON public.asset_delivery FOR DELETE TO ple_retention_broker USING (((tenant_id = public.ple_current_tenant()) AND (delivery_kind = 'student_record'::text)));

CREATE POLICY retention_broker_asset_delivery_tenant_select ON public.asset_delivery FOR SELECT TO ple_retention_broker USING (((tenant_id = public.ple_current_tenant()) AND (delivery_kind = 'student_record'::text)));

CREATE POLICY retention_broker_assignment_item_tenant_delete ON public.assignment_item FOR DELETE TO ple_retention_broker USING ((tenant_id = public.ple_current_tenant()));

CREATE POLICY retention_broker_assignment_item_tenant_select ON public.assignment_item FOR SELECT TO ple_retention_broker USING ((tenant_id = public.ple_current_tenant()));

-- Replace the bootstrap tenant-only policies with course lifecycle-aware
-- policies once the retention helper is available.
DROP POLICY course_item_analysis_current_tenant ON public.course_item_analysis_current;

CREATE POLICY course_item_analysis_current_tenant ON public.course_item_analysis_current TO ple_app
    USING ((tenant_id = public.ple_current_tenant()) AND public.ple_course_records_accessible(tenant_id, course_id))
    WITH CHECK ((tenant_id = public.ple_current_tenant()) AND public.ple_course_records_accessible(tenant_id, course_id));

CREATE POLICY retention_broker_course_item_analysis_current_delete ON public.course_item_analysis_current
    FOR DELETE TO ple_retention_broker USING ((tenant_id = public.ple_current_tenant()));

CREATE POLICY retention_broker_course_item_analysis_current_select ON public.course_item_analysis_current
    FOR SELECT TO ple_retention_broker USING ((tenant_id = public.ple_current_tenant()));

DROP POLICY course_item_analysis_staging_tenant ON public.course_item_analysis_staging;

CREATE POLICY course_item_analysis_staging_tenant ON public.course_item_analysis_staging TO ple_app
    USING ((tenant_id = public.ple_current_tenant()) AND public.ple_course_records_accessible(tenant_id, course_id))
    WITH CHECK ((tenant_id = public.ple_current_tenant()) AND public.ple_course_records_accessible(tenant_id, course_id));

CREATE POLICY retention_broker_course_item_analysis_staging_delete ON public.course_item_analysis_staging
    FOR DELETE TO ple_retention_broker USING ((tenant_id = public.ple_current_tenant()));

CREATE POLICY retention_broker_course_item_analysis_staging_select ON public.course_item_analysis_staging
    FOR SELECT TO ple_retention_broker USING ((tenant_id = public.ple_current_tenant()));

CREATE POLICY retention_broker_assignment_run_tenant_delete ON public.assignment_run FOR DELETE TO ple_retention_broker USING ((tenant_id = public.ple_current_tenant()));

CREATE POLICY retention_broker_assignment_run_tenant_select ON public.assignment_run FOR SELECT TO ple_retention_broker USING ((tenant_id = public.ple_current_tenant()));

CREATE POLICY retention_broker_assignment_tenant_delete ON public.assignment FOR DELETE TO ple_retention_broker USING ((tenant_id = public.ple_current_tenant()));

CREATE POLICY retention_broker_assignment_tenant_select ON public.assignment FOR SELECT TO ple_retention_broker USING ((tenant_id = public.ple_current_tenant()));

CREATE POLICY retention_broker_attempt_feedback_tenant_delete ON public.attempt_feedback FOR DELETE TO ple_retention_broker USING ((tenant_id = public.ple_current_tenant()));

CREATE POLICY retention_broker_attempt_feedback_tenant_select ON public.attempt_feedback FOR SELECT TO ple_retention_broker USING ((tenant_id = public.ple_current_tenant()));

CREATE POLICY retention_broker_record_access_log_tenant_delete ON public.record_access_log FOR DELETE TO ple_retention_broker USING (((tenant_id = public.ple_current_tenant()) AND (delivery_scope = 'student_record'::text)));

CREATE POLICY retention_broker_record_access_log_tenant_select ON public.record_access_log FOR SELECT TO ple_retention_broker USING (((tenant_id = public.ple_current_tenant()) AND (delivery_scope = 'student_record'::text)));

CREATE POLICY retention_broker_audit_event_tenant_delete ON public.audit_event FOR DELETE TO ple_retention_broker USING ((tenant_id = public.ple_current_tenant()));

CREATE POLICY retention_broker_audit_event_tenant_select ON public.audit_event FOR SELECT TO ple_retention_broker USING ((tenant_id = public.ple_current_tenant()));

CREATE POLICY retention_broker_course_member_tenant_delete ON public.course_member FOR DELETE TO ple_retention_broker USING ((tenant_id = public.ple_current_tenant()));

CREATE POLICY retention_broker_course_member_tenant_select ON public.course_member FOR SELECT TO ple_retention_broker USING ((tenant_id = public.ple_current_tenant()));

CREATE POLICY retention_broker_course_group_member_tenant_delete ON public.course_group_member FOR DELETE TO ple_retention_broker USING ((tenant_id = public.ple_current_tenant()));

CREATE POLICY retention_broker_course_group_member_tenant_select ON public.course_group_member FOR SELECT TO ple_retention_broker USING ((tenant_id = public.ple_current_tenant()));

CREATE POLICY retention_broker_assignment_policy_exception_tenant_delete ON public.assignment_policy_exception FOR DELETE TO ple_retention_broker USING ((tenant_id = public.ple_current_tenant()));

CREATE POLICY retention_broker_assignment_policy_exception_tenant_select ON public.assignment_policy_exception FOR SELECT TO ple_retention_broker USING ((tenant_id = public.ple_current_tenant()));

CREATE POLICY retention_broker_enrollment_tenant_delete ON public.enrollment FOR DELETE TO ple_retention_broker USING ((tenant_id = public.ple_current_tenant()));

CREATE POLICY retention_broker_enrollment_tenant_select ON public.enrollment FOR SELECT TO ple_retention_broker USING ((tenant_id = public.ple_current_tenant()));

CREATE POLICY retention_broker_export_artifact ON public.student_export_artifact FOR SELECT TO ple_retention_broker USING ((EXISTS ( SELECT 1
   FROM public.student_export_request r
  WHERE ((r.export_id = student_export_artifact.export_id) AND (r.tenant_id = public.ple_current_tenant())))));

CREATE POLICY retention_broker_export_artifact_tenant_delete ON public.student_export_artifact FOR DELETE TO ple_retention_broker USING ((EXISTS ( SELECT 1
   FROM public.student_export_request r
  WHERE ((r.export_id = student_export_artifact.export_id) AND (r.tenant_id = public.ple_current_tenant())))));

CREATE POLICY retention_broker_export_artifact_tenant_select ON public.student_export_artifact FOR SELECT TO ple_retention_broker USING ((EXISTS ( SELECT 1
   FROM public.student_export_request r
  WHERE ((r.export_id = student_export_artifact.export_id) AND (r.tenant_id = public.ple_current_tenant())))));

CREATE POLICY retention_broker_export_request ON public.student_export_request FOR SELECT TO ple_retention_broker USING ((tenant_id = public.ple_current_tenant()));

CREATE POLICY retention_broker_export_request_tenant_delete ON public.student_export_request FOR DELETE TO ple_retention_broker USING ((tenant_id = public.ple_current_tenant()));

CREATE POLICY retention_broker_export_request_tenant_select ON public.student_export_request FOR SELECT TO ple_retention_broker USING ((tenant_id = public.ple_current_tenant()));

CREATE POLICY retention_broker_export_request_update ON public.student_export_request FOR UPDATE TO ple_retention_broker USING ((tenant_id = public.ple_current_tenant())) WITH CHECK ((tenant_id = public.ple_current_tenant()));

CREATE POLICY retention_broker_external_tool_exchange ON public.external_tool_exchange FOR SELECT TO ple_retention_broker USING ((tenant_id = public.ple_current_tenant()));

CREATE POLICY retention_broker_external_tool_exchange_tenant_delete ON public.external_tool_exchange FOR DELETE TO ple_retention_broker USING ((tenant_id = public.ple_current_tenant()));

CREATE POLICY retention_broker_external_tool_exchange_tenant_select ON public.external_tool_exchange FOR SELECT TO ple_retention_broker USING ((tenant_id = public.ple_current_tenant()));

CREATE POLICY retention_broker_external_tool_launch_session_tenant_delete ON public.external_tool_launch_session FOR DELETE TO ple_retention_broker USING ((tenant_id = public.ple_current_tenant()));

CREATE POLICY retention_broker_external_tool_launch_session_tenant_select ON public.external_tool_launch_session FOR SELECT TO ple_retention_broker USING ((tenant_id = public.ple_current_tenant()));

CREATE POLICY retention_broker_feedback_release_tenant_delete ON public.feedback_release FOR DELETE TO ple_retention_broker USING ((tenant_id = public.ple_current_tenant()));

CREATE POLICY retention_broker_feedback_release_tenant_select ON public.feedback_release FOR SELECT TO ple_retention_broker USING ((tenant_id = public.ple_current_tenant()));

CREATE POLICY retention_broker_submission_evaluation_tenant_delete ON public.submission_evaluation FOR DELETE TO ple_retention_broker USING ((tenant_id = public.ple_current_tenant()));

CREATE POLICY retention_broker_submission_evaluation_tenant_select ON public.submission_evaluation FOR SELECT TO ple_retention_broker USING ((tenant_id = public.ple_current_tenant()));

CREATE POLICY retention_broker_manual_grade_receipt_tenant_delete ON public.manual_grade_receipt FOR DELETE TO ple_retention_broker USING ((tenant_id = public.ple_current_tenant()));

CREATE POLICY retention_broker_manual_grade_receipt_tenant_select ON public.manual_grade_receipt FOR SELECT TO ple_retention_broker USING ((tenant_id = public.ple_current_tenant()));

CREATE POLICY retention_broker_attempt_score_current_tenant_delete ON public.attempt_score_current FOR DELETE TO ple_retention_broker USING ((tenant_id = public.ple_current_tenant()));

CREATE POLICY retention_broker_attempt_score_current_tenant_select ON public.attempt_score_current FOR SELECT TO ple_retention_broker USING ((tenant_id = public.ple_current_tenant()));

CREATE POLICY retention_broker_assignment_run_item_tenant_delete ON public.assignment_run_item FOR DELETE TO ple_retention_broker USING ((tenant_id = public.ple_current_tenant()));

CREATE POLICY retention_broker_assignment_run_item_tenant_select ON public.assignment_run_item FOR SELECT TO ple_retention_broker USING ((tenant_id = public.ple_current_tenant()));

CREATE POLICY retention_broker_question_attempt_tenant_delete ON public.question_attempt FOR DELETE TO ple_retention_broker USING ((tenant_id = public.ple_current_tenant()));

CREATE POLICY retention_broker_question_attempt_tenant_select ON public.question_attempt FOR SELECT TO ple_retention_broker USING ((tenant_id = public.ple_current_tenant()));

CREATE POLICY retention_broker_question_prefetch_tenant_delete ON public.question_prefetch FOR DELETE TO ple_retention_broker USING ((tenant_id = public.ple_current_tenant()));

CREATE POLICY retention_broker_question_prefetch_tenant_select ON public.question_prefetch FOR SELECT TO ple_retention_broker USING ((tenant_id = public.ple_current_tenant()));

CREATE POLICY retention_broker_student_assignment_summary_tenant_delete ON public.student_assignment_summary FOR DELETE TO ple_retention_broker USING ((tenant_id = public.ple_current_tenant()));

CREATE POLICY retention_broker_student_assignment_summary_tenant_select ON public.student_assignment_summary FOR SELECT TO ple_retention_broker USING ((tenant_id = public.ple_current_tenant()));

CREATE POLICY retention_broker_submission_idempotency_tenant_delete ON public.submission_idempotency FOR DELETE TO ple_retention_broker USING ((tenant_id = public.ple_current_tenant()));

CREATE POLICY retention_broker_submission_idempotency_tenant_select ON public.submission_idempotency FOR SELECT TO ple_retention_broker USING ((tenant_id = public.ple_current_tenant()));

CREATE POLICY retention_broker_submission_next_attempt_tenant_delete ON public.submission_next_attempt FOR DELETE TO ple_retention_broker USING ((tenant_id = public.ple_current_tenant()));

CREATE POLICY retention_broker_submission_next_attempt_tenant_select ON public.submission_next_attempt FOR SELECT TO ple_retention_broker USING ((tenant_id = public.ple_current_tenant()));

CREATE POLICY retention_broker_submission_receipt_snapshot_tenant_delete ON public.submission_receipt_snapshot FOR DELETE TO ple_retention_broker USING ((tenant_id = public.ple_current_tenant()));

CREATE POLICY retention_broker_submission_receipt_snapshot_tenant_select ON public.submission_receipt_snapshot FOR SELECT TO ple_retention_broker USING ((tenant_id = public.ple_current_tenant()));

CREATE POLICY retention_broker_submission_tenant_delete ON public.submission FOR DELETE TO ple_retention_broker USING ((tenant_id = public.ple_current_tenant()));

CREATE POLICY retention_broker_submission_tenant_select ON public.submission FOR SELECT TO ple_retention_broker USING ((tenant_id = public.ple_current_tenant()));

CREATE POLICY retention_broker_worker_job ON public.worker_job FOR SELECT TO ple_retention_broker USING ((tenant_id = public.ple_current_tenant()));

CREATE POLICY retention_broker_worker_job_insert ON public.worker_job FOR INSERT TO ple_retention_broker WITH CHECK (true);

CREATE POLICY retention_broker_worker_job_tenant_delete ON public.worker_job FOR DELETE TO ple_retention_broker USING ((tenant_id = public.ple_current_tenant()));

CREATE POLICY retention_broker_worker_job_tenant_select ON public.worker_job FOR SELECT TO ple_retention_broker USING ((tenant_id = public.ple_current_tenant()));

CREATE POLICY retention_broker_worker_job_update ON public.worker_job FOR UPDATE TO ple_retention_broker USING ((tenant_id = public.ple_current_tenant())) WITH CHECK ((tenant_id = public.ple_current_tenant()));

CREATE POLICY student_assignment_summary_tenant ON public.student_assignment_summary USING (((tenant_id = public.ple_current_tenant()) AND (EXISTS ( SELECT 1
   FROM (public.enrollment e
     JOIN public.assignment a ON (((a.tenant_id = e.tenant_id) AND (a.assignment_id = e.assignment_id))))
  WHERE ((e.tenant_id = student_assignment_summary.tenant_id) AND (e.enrollment_id = student_assignment_summary.enrollment_id) AND public.ple_course_records_accessible(a.tenant_id, a.course_id)))))) WITH CHECK (((tenant_id = public.ple_current_tenant()) AND (EXISTS ( SELECT 1
   FROM (public.enrollment e
     JOIN public.assignment a ON (((a.tenant_id = e.tenant_id) AND (a.assignment_id = e.assignment_id))))
  WHERE ((e.tenant_id = student_assignment_summary.tenant_id) AND (e.enrollment_id = student_assignment_summary.enrollment_id) AND public.ple_course_records_accessible(a.tenant_id, a.course_id))))));

CREATE POLICY student_export_artifact_tenant ON public.student_export_artifact USING ((EXISTS ( SELECT 1
   FROM public.student_export_request r
  WHERE ((r.export_id = student_export_artifact.export_id) AND public.ple_course_records_accessible(r.tenant_id, r.course_id)))));

CREATE POLICY student_export_request_tenant ON public.student_export_request USING (((tenant_id = public.ple_current_tenant()) AND public.ple_course_records_accessible(tenant_id, course_id))) WITH CHECK (((tenant_id = public.ple_current_tenant()) AND public.ple_course_records_accessible(tenant_id, course_id)));

CREATE POLICY submission_idempotency_tenant ON public.submission_idempotency USING (((tenant_id = public.ple_current_tenant()) AND (EXISTS ( SELECT 1
   FROM (((public.question_attempt q
     JOIN public.assignment_run r ON (((r.tenant_id = q.tenant_id) AND (r.run_id = q.run_id))))
     JOIN public.enrollment e ON (((e.tenant_id = r.tenant_id) AND (e.enrollment_id = r.enrollment_id))))
     JOIN public.assignment a ON (((a.tenant_id = e.tenant_id) AND (a.assignment_id = e.assignment_id))))
  WHERE ((q.tenant_id = submission_idempotency.tenant_id) AND (q.attempt_id = submission_idempotency.attempt_id) AND public.ple_course_records_accessible(a.tenant_id, a.course_id)))))) WITH CHECK (((tenant_id = public.ple_current_tenant()) AND (EXISTS ( SELECT 1
   FROM (((public.question_attempt q
     JOIN public.assignment_run r ON (((r.tenant_id = q.tenant_id) AND (r.run_id = q.run_id))))
     JOIN public.enrollment e ON (((e.tenant_id = r.tenant_id) AND (e.enrollment_id = r.enrollment_id))))
     JOIN public.assignment a ON (((a.tenant_id = e.tenant_id) AND (a.assignment_id = e.assignment_id))))
  WHERE ((q.tenant_id = submission_idempotency.tenant_id) AND (q.attempt_id = submission_idempotency.attempt_id) AND public.ple_course_records_accessible(a.tenant_id, a.course_id))))));

CREATE POLICY submission_next_attempt_tenant ON public.submission_next_attempt USING (((tenant_id = public.ple_current_tenant()) AND (EXISTS ( SELECT 1
   FROM (((public.question_attempt q
     JOIN public.assignment_run r ON (((r.tenant_id = q.tenant_id) AND (r.run_id = q.run_id))))
     JOIN public.enrollment e ON (((e.tenant_id = r.tenant_id) AND (e.enrollment_id = r.enrollment_id))))
     JOIN public.assignment a ON (((a.tenant_id = e.tenant_id) AND (a.assignment_id = e.assignment_id))))
  WHERE ((q.tenant_id = submission_next_attempt.tenant_id) AND (q.attempt_id = submission_next_attempt.predecessor_attempt_id) AND public.ple_course_records_accessible(a.tenant_id, a.course_id)))))) WITH CHECK (((tenant_id = public.ple_current_tenant()) AND (EXISTS ( SELECT 1
   FROM (((public.question_attempt q
     JOIN public.assignment_run r ON (((r.tenant_id = q.tenant_id) AND (r.run_id = q.run_id))))
     JOIN public.enrollment e ON (((e.tenant_id = r.tenant_id) AND (e.enrollment_id = r.enrollment_id))))
     JOIN public.assignment a ON (((a.tenant_id = e.tenant_id) AND (a.assignment_id = e.assignment_id))))
  WHERE ((q.tenant_id = submission_next_attempt.tenant_id) AND (q.attempt_id = submission_next_attempt.predecessor_attempt_id) AND public.ple_course_records_accessible(a.tenant_id, a.course_id))))));

CREATE POLICY submission_tenant ON public.submission USING (((tenant_id = public.ple_current_tenant()) AND (EXISTS ( SELECT 1
   FROM (((public.question_attempt q
     JOIN public.assignment_run r ON (((r.tenant_id = q.tenant_id) AND (r.run_id = q.run_id))))
     JOIN public.enrollment e ON (((e.tenant_id = r.tenant_id) AND (e.enrollment_id = r.enrollment_id))))
     JOIN public.assignment a ON (((a.tenant_id = e.tenant_id) AND (a.assignment_id = e.assignment_id))))
  WHERE ((q.tenant_id = submission.tenant_id) AND (q.attempt_id = submission.attempt_id) AND public.ple_course_records_accessible(a.tenant_id, a.course_id)))))) WITH CHECK (((tenant_id = public.ple_current_tenant()) AND (EXISTS ( SELECT 1
   FROM (((public.question_attempt q
     JOIN public.assignment_run r ON (((r.tenant_id = q.tenant_id) AND (r.run_id = q.run_id))))
     JOIN public.enrollment e ON (((e.tenant_id = r.tenant_id) AND (e.enrollment_id = r.enrollment_id))))
     JOIN public.assignment a ON (((a.tenant_id = e.tenant_id) AND (a.assignment_id = e.assignment_id))))
  WHERE ((q.tenant_id = submission.tenant_id) AND (q.attempt_id = submission.attempt_id) AND public.ple_course_records_accessible(a.tenant_id, a.course_id))))));

-- QTI profile evidence and provenance are author-owned import lineage, not
-- learner-course records. Course-retention cleanup must neither enumerate nor
-- erase them; deleting a draft deliberately clears only its current origin
-- through the catalog foreign key, while published lineage remains immutable.
REVOKE ALL ON TABLE
    public.workspace_qti_profile_import_evidence,
    public.workspace_qti_profile_item_evidence,
    public.workspace_flat_import_origin,
    public.workspace_flat_import_choice_map,
    public.published_flat_import_origin,
    public.published_flat_import_choice_map
FROM ple_retention_broker;
