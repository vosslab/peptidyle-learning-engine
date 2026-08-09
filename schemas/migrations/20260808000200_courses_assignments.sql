-- Initial database epoch: courses assignments.

SET check_function_bodies = false;

CREATE TABLE public.assignment (
    tenant_id uuid NOT NULL,
    assignment_id uuid NOT NULL,
    updated_at timestamp with time zone DEFAULT transaction_timestamp() NOT NULL,
    course_id uuid NOT NULL,
    title text NOT NULL,
    instructions text DEFAULT ''::text NOT NULL,
    lifecycle text DEFAULT 'draft'::text NOT NULL,
    visible boolean DEFAULT false NOT NULL,
    available_at timestamp with time zone,
    due_at timestamp with time zone,
    closes_at timestamp with time zone,
    late_submission_policy text DEFAULT 'accept'::text NOT NULL,
    time_limit_seconds integer,
    auto_submit boolean DEFAULT true NOT NULL,
    attempt_limit integer,
    attempt_selection_policy text DEFAULT 'highest'::text NOT NULL,
    gradebook_included boolean DEFAULT true NOT NULL,
    presentation_policy text DEFAULT 'one_at_a_time'::text NOT NULL,
    randomization_policy text DEFAULT 'fixed'::text NOT NULL,
    backtracking_allowed boolean DEFAULT true NOT NULL,
    feedback_disclosure text DEFAULT 'deferred'::text NOT NULL,
    completion_policy text DEFAULT 'answer_all'::text NOT NULL,
    completion_threshold numeric(9,8),
    continued_practice_policy text DEFAULT 'unlimited'::text NOT NULL,
    practice_max_additional_runs integer,
    variation_policy text DEFAULT 'new_seeds'::text NOT NULL,
    scoring_generation bigint DEFAULT 1 NOT NULL,
    scoring_status text DEFAULT 'current'::text NOT NULL,
    created_at timestamp with time zone DEFAULT transaction_timestamp() NOT NULL,
    revision bigint DEFAULT 1 NOT NULL,
    CONSTRAINT assignment_revision_check CHECK ((revision > 0)),
    CONSTRAINT assignment_title_check CHECK ((((char_length(title) >= 1) AND (char_length(title) <= 200)) AND (title = btrim(title)))),
    CONSTRAINT assignment_lifecycle_check CHECK ((lifecycle = ANY (ARRAY['draft'::text, 'published'::text, 'closed'::text, 'archived'::text]))),
    CONSTRAINT assignment_late_policy_check CHECK ((late_submission_policy = ANY (ARRAY['accept'::text, 'reject'::text, 'mark_late'::text]))),
    CONSTRAINT assignment_time_limit_check CHECK ((time_limit_seconds IS NULL OR time_limit_seconds > 0)),
    CONSTRAINT assignment_attempt_limit_check CHECK ((attempt_limit IS NULL OR attempt_limit > 0)),
    CONSTRAINT assignment_attempt_selection_check CHECK ((attempt_selection_policy = ANY (ARRAY['first'::text, 'last'::text, 'highest'::text, 'instructor_selected'::text]))),
    CONSTRAINT assignment_completion_policy_check CHECK ((completion_policy = ANY (ARRAY['answer_all'::text, 'all_correct'::text, 'score_at_least'::text]))),
    CONSTRAINT assignment_completion_threshold_check CHECK (((completion_policy = 'score_at_least'::text AND completion_threshold >= 0 AND completion_threshold <= 1) OR (completion_policy <> 'score_at_least'::text AND completion_threshold IS NULL))),
    CONSTRAINT assignment_continued_practice_check CHECK ((continued_practice_policy = ANY (ARRAY['unlimited'::text, 'capped'::text, 'closed'::text]))),
    CONSTRAINT assignment_practice_limit_check CHECK (((continued_practice_policy = 'capped'::text AND practice_max_additional_runs >= 0) OR (continued_practice_policy <> 'capped'::text AND practice_max_additional_runs IS NULL))),
    CONSTRAINT assignment_variation_policy_check CHECK ((variation_policy = ANY (ARRAY['new_seeds'::text, 'selected_problem_variants'::text, 'full_regeneration'::text]))),
    CONSTRAINT assignment_scoring_generation_check CHECK ((scoring_generation > 0)),
    CONSTRAINT assignment_scoring_status_check CHECK ((scoring_status = ANY (ARRAY['current'::text, 'recalculating'::text, 'failed'::text]))),
    CONSTRAINT assignment_schedule_check CHECK (((available_at IS NULL OR due_at IS NULL OR available_at <= due_at) AND (due_at IS NULL OR closes_at IS NULL OR due_at <= closes_at)))
);

ALTER TABLE ONLY public.assignment FORCE ROW LEVEL SECURITY;

CREATE TABLE public.assignment_item (
    tenant_id uuid NOT NULL,
    assignment_id uuid NOT NULL,
    assignment_item_id uuid NOT NULL,
    "position" integer NOT NULL,
    problem_id uuid NOT NULL,
    version_id uuid NOT NULL,
    points_possible numeric(16,4) NOT NULL,
    delivery_state text DEFAULT 'active'::text NOT NULL,
    scoring_mode text DEFAULT 'normal'::text NOT NULL,
    created_at timestamp with time zone DEFAULT transaction_timestamp() NOT NULL,
    updated_at timestamp with time zone DEFAULT transaction_timestamp() NOT NULL,
    revision bigint DEFAULT 1 NOT NULL,
    CONSTRAINT assignment_item_position_check CHECK (("position" >= 0)),
    CONSTRAINT assignment_item_points_check CHECK ((points_possible >= 0)),
    CONSTRAINT assignment_item_delivery_state_check CHECK ((delivery_state = ANY (ARRAY['active'::text, 'retired'::text]))),
    CONSTRAINT assignment_item_scoring_mode_check CHECK ((scoring_mode = ANY (ARRAY['normal'::text, 'full_credit'::text, 'extra_credit'::text, 'excluded'::text]))),
    CONSTRAINT assignment_item_retired_excluded_check CHECK ((delivery_state <> 'retired'::text OR scoring_mode = 'excluded'::text)),
    CONSTRAINT assignment_item_revision_check CHECK ((revision > 0))
);

ALTER TABLE ONLY public.assignment_item FORCE ROW LEVEL SECURITY;

CREATE TABLE public.assignment_selection_group (
    tenant_id uuid NOT NULL,
    assignment_id uuid NOT NULL,
    selection_group_id uuid NOT NULL,
    "position" integer NOT NULL,
    draw_count integer NOT NULL,
    points_per_item numeric(16,4) NOT NULL,
    ordering_policy text NOT NULL,
    algorithm_version integer NOT NULL,
    created_at timestamp with time zone DEFAULT transaction_timestamp() NOT NULL,
    updated_at timestamp with time zone DEFAULT transaction_timestamp() NOT NULL,
    revision bigint DEFAULT 1 NOT NULL,
    CONSTRAINT assignment_selection_group_position_check CHECK (("position" >= 0)),
    CONSTRAINT assignment_selection_group_draw_check CHECK ((draw_count > 0)),
    CONSTRAINT assignment_selection_group_points_check CHECK ((points_per_item >= 0)),
    CONSTRAINT assignment_selection_group_ordering_check CHECK ((ordering_policy = ANY (ARRAY['candidate_order'::text, 'randomized'::text]))),
    CONSTRAINT assignment_selection_group_algorithm_check CHECK ((algorithm_version > 0)),
    CONSTRAINT assignment_selection_group_revision_check CHECK ((revision > 0))
);

ALTER TABLE ONLY public.assignment_selection_group FORCE ROW LEVEL SECURITY;

CREATE TABLE public.assignment_selection_candidate (
    tenant_id uuid NOT NULL,
    assignment_id uuid NOT NULL,
    selection_group_id uuid NOT NULL,
    candidate_id uuid NOT NULL,
    "position" integer NOT NULL,
    problem_id uuid NOT NULL,
    version_id uuid NOT NULL,
    delivery_state text DEFAULT 'active'::text NOT NULL,
    created_at timestamp with time zone DEFAULT transaction_timestamp() NOT NULL,
    updated_at timestamp with time zone DEFAULT transaction_timestamp() NOT NULL,
    CONSTRAINT assignment_selection_candidate_state_check CHECK ((delivery_state = ANY (ARRAY['active'::text, 'retired'::text]))),
    CONSTRAINT assignment_selection_candidate_position_check CHECK (("position" >= 0))
);

ALTER TABLE ONLY public.assignment_selection_candidate FORCE ROW LEVEL SECURITY;

CREATE TABLE public.course (
    tenant_id uuid NOT NULL,
    course_id uuid NOT NULL,
    title text NOT NULL,
    created_at timestamp with time zone DEFAULT transaction_timestamp() NOT NULL,
    updated_at timestamp with time zone DEFAULT transaction_timestamp() NOT NULL,
    CONSTRAINT course_title_check CHECK ((((char_length(title) >= 1) AND (char_length(title) <= 200)) AND (title = btrim(title))))
);

ALTER TABLE ONLY public.course FORCE ROW LEVEL SECURITY;

CREATE TABLE public.course_member (
    tenant_id uuid NOT NULL,
    course_id uuid NOT NULL,
    user_id uuid NOT NULL,
    role text NOT NULL,
    CONSTRAINT course_member_role_check CHECK ((role = ANY (ARRAY['student'::text, 'instructor'::text])))
);

ALTER TABLE ONLY public.course_member FORCE ROW LEVEL SECURITY;

CREATE TABLE public.course_group (
    tenant_id uuid NOT NULL,
    course_id uuid NOT NULL,
    course_group_id uuid NOT NULL,
    title text NOT NULL,
    created_at timestamp with time zone DEFAULT transaction_timestamp() NOT NULL,
    updated_at timestamp with time zone DEFAULT transaction_timestamp() NOT NULL,
    revision bigint DEFAULT 1 NOT NULL,
    CONSTRAINT course_group_title_check CHECK ((char_length(title) BETWEEN 1 AND 200 AND title = btrim(title))),
    CONSTRAINT course_group_revision_check CHECK ((revision > 0))
);

ALTER TABLE ONLY public.course_group FORCE ROW LEVEL SECURITY;

CREATE TABLE public.course_group_member (
    tenant_id uuid NOT NULL,
    course_id uuid NOT NULL,
    course_group_id uuid NOT NULL,
    user_id uuid NOT NULL
);

ALTER TABLE ONLY public.course_group_member FORCE ROW LEVEL SECURITY;

CREATE TABLE public.enrollment (
    tenant_id uuid NOT NULL,
    enrollment_id uuid NOT NULL,
    assignment_id uuid NOT NULL,
    student_id uuid NOT NULL,
    payload jsonb NOT NULL,
    payload_sha256 character(64) NOT NULL,
    user_id uuid
);

ALTER TABLE ONLY public.enrollment FORCE ROW LEVEL SECURITY;

CREATE TABLE public.assignment_policy_exception (
    tenant_id uuid NOT NULL,
    assignment_policy_exception_id uuid NOT NULL,
    course_id uuid NOT NULL,
    assignment_id uuid NOT NULL,
    student_id uuid,
    course_group_id uuid,
    available_mode text,
    available_at timestamp with time zone,
    closes_mode text,
    closes_at timestamp with time zone,
    time_limit_mode text,
    time_limit_seconds integer,
    attempt_limit_mode text,
    attempt_limit integer,
    created_at timestamp with time zone DEFAULT transaction_timestamp() NOT NULL,
    updated_at timestamp with time zone DEFAULT transaction_timestamp() NOT NULL,
    revision bigint DEFAULT 1 NOT NULL,
    CONSTRAINT assignment_policy_exception_target_check CHECK (((student_id IS NULL) <> (course_group_id IS NULL))),
    CONSTRAINT assignment_policy_exception_available_check CHECK (((available_mode IS NULL AND available_at IS NULL) OR (available_mode = 'unrestricted' AND available_at IS NULL) OR (available_mode = 'at' AND available_at IS NOT NULL))),
    CONSTRAINT assignment_policy_exception_closes_check CHECK (((closes_mode IS NULL AND closes_at IS NULL) OR (closes_mode = 'unrestricted' AND closes_at IS NULL) OR (closes_mode = 'at' AND closes_at IS NOT NULL))),
    CONSTRAINT assignment_policy_exception_time_limit_check CHECK (((time_limit_mode IS NULL AND time_limit_seconds IS NULL) OR (time_limit_mode = 'unlimited' AND time_limit_seconds IS NULL) OR (time_limit_mode = 'value' AND time_limit_seconds > 0))),
    CONSTRAINT assignment_policy_exception_attempt_limit_check CHECK (((attempt_limit_mode IS NULL AND attempt_limit IS NULL) OR (attempt_limit_mode = 'unlimited' AND attempt_limit IS NULL) OR (attempt_limit_mode = 'value' AND attempt_limit > 0))),
    CONSTRAINT assignment_policy_exception_nonempty_check CHECK ((available_mode IS NOT NULL OR closes_mode IS NOT NULL OR time_limit_mode IS NOT NULL OR attempt_limit_mode IS NOT NULL)),
    CONSTRAINT assignment_policy_exception_schedule_check CHECK ((available_mode <> 'at' OR closes_mode <> 'at' OR available_at <= closes_at)),
    CONSTRAINT assignment_policy_exception_revision_check CHECK ((revision > 0))
);

ALTER TABLE ONLY public.assignment_policy_exception FORCE ROW LEVEL SECURITY;

CREATE TABLE public.student_assignment_summary (
    tenant_id uuid NOT NULL,
    enrollment_id uuid NOT NULL,
    payload jsonb NOT NULL,
    payload_sha256 character(64) NOT NULL,
    updated_at timestamp with time zone DEFAULT transaction_timestamp() NOT NULL
);

ALTER TABLE ONLY public.student_assignment_summary FORCE ROW LEVEL SECURITY;

ALTER TABLE ONLY public.assignment
    ADD CONSTRAINT assignment_pkey PRIMARY KEY (tenant_id, assignment_id);

ALTER TABLE ONLY public.assignment
    ADD CONSTRAINT assignment_course_identity_key UNIQUE (tenant_id, course_id, assignment_id);

ALTER TABLE ONLY public.assignment_item
    ADD CONSTRAINT assignment_item_pkey PRIMARY KEY (tenant_id, assignment_item_id);

ALTER TABLE ONLY public.assignment_item
    ADD CONSTRAINT assignment_item_position_key UNIQUE (tenant_id, assignment_id, "position");

ALTER TABLE ONLY public.assignment_selection_group
    ADD CONSTRAINT assignment_selection_group_pkey PRIMARY KEY (tenant_id, selection_group_id);

ALTER TABLE ONLY public.assignment_selection_group
    ADD CONSTRAINT assignment_selection_group_assignment_key UNIQUE (tenant_id, assignment_id, selection_group_id);

ALTER TABLE ONLY public.assignment_selection_group
    ADD CONSTRAINT assignment_selection_group_position_key UNIQUE (tenant_id, assignment_id, "position");

ALTER TABLE ONLY public.assignment_selection_candidate
    ADD CONSTRAINT assignment_selection_candidate_pkey PRIMARY KEY (tenant_id, candidate_id);

ALTER TABLE ONLY public.assignment_selection_candidate
    ADD CONSTRAINT assignment_selection_candidate_version_key UNIQUE (tenant_id, selection_group_id, problem_id, version_id);

ALTER TABLE ONLY public.assignment_selection_candidate
    ADD CONSTRAINT assignment_selection_candidate_position_key UNIQUE (tenant_id, selection_group_id, "position");

ALTER TABLE ONLY public.course_member
    ADD CONSTRAINT course_member_pkey PRIMARY KEY (tenant_id, course_id, user_id);

ALTER TABLE ONLY public.course_group
    ADD CONSTRAINT course_group_pkey PRIMARY KEY (tenant_id, course_group_id);

ALTER TABLE ONLY public.course_group
    ADD CONSTRAINT course_group_course_identity_key UNIQUE (tenant_id, course_id, course_group_id);

ALTER TABLE ONLY public.course_group_member
    ADD CONSTRAINT course_group_member_pkey PRIMARY KEY (tenant_id, course_group_id, user_id);

ALTER TABLE ONLY public.course
    ADD CONSTRAINT course_pkey PRIMARY KEY (tenant_id, course_id);

ALTER TABLE ONLY public.enrollment
    ADD CONSTRAINT enrollment_pkey PRIMARY KEY (tenant_id, enrollment_id);

ALTER TABLE ONLY public.enrollment
    ADD CONSTRAINT enrollment_assignment_student_key UNIQUE (tenant_id, assignment_id, student_id);

ALTER TABLE ONLY public.assignment_policy_exception
    ADD CONSTRAINT assignment_policy_exception_pkey PRIMARY KEY (tenant_id, assignment_policy_exception_id);

ALTER TABLE public.enrollment
    ADD CONSTRAINT enrollment_user_required_check CHECK ((user_id IS NOT NULL)) NOT VALID;

ALTER TABLE ONLY public.student_assignment_summary
    ADD CONSTRAINT student_assignment_summary_pkey PRIMARY KEY (tenant_id, enrollment_id);

CREATE INDEX assignment_course_page_idx ON public.assignment USING btree (tenant_id, course_id, assignment_id);

CREATE INDEX assignment_item_assignment_idx ON public.assignment_item USING btree (tenant_id, assignment_id, "position") WHERE (delivery_state = 'active'::text);

CREATE INDEX assignment_selection_candidate_group_idx ON public.assignment_selection_candidate USING btree (tenant_id, selection_group_id, "position", candidate_id) WHERE (delivery_state = 'active'::text);

CREATE INDEX course_member_user_courses_idx ON public.course_member USING btree (tenant_id, user_id, course_id);

CREATE INDEX course_group_member_user_idx ON public.course_group_member USING btree (tenant_id, course_id, user_id, course_group_id);

CREATE UNIQUE INDEX assignment_policy_exception_student_key ON public.assignment_policy_exception USING btree (tenant_id, assignment_id, student_id) WHERE (student_id IS NOT NULL);

CREATE UNIQUE INDEX assignment_policy_exception_group_key ON public.assignment_policy_exception USING btree (tenant_id, assignment_id, course_group_id) WHERE (course_group_id IS NOT NULL);

CREATE INDEX assignment_policy_exception_course_idx ON public.assignment_policy_exception USING btree (tenant_id, course_id, assignment_id);

CREATE INDEX enrollment_gradebook_summary_page_idx ON public.enrollment USING btree (tenant_id, assignment_id, enrollment_id);

CREATE UNIQUE INDEX enrollment_user_assignment_idx ON public.enrollment USING btree (tenant_id, assignment_id, user_id) WHERE (user_id IS NOT NULL);

ALTER TABLE ONLY public.assignment
    ADD CONSTRAINT assignment_course_fk FOREIGN KEY (tenant_id, course_id) REFERENCES public.course(tenant_id, course_id) NOT VALID;

ALTER TABLE ONLY public.assignment_item
    ADD CONSTRAINT assignment_item_problem_id_version_id_fkey FOREIGN KEY (problem_id, version_id) REFERENCES public.problem_version(problem_id, version_id);

ALTER TABLE ONLY public.assignment_item
    ADD CONSTRAINT assignment_item_tenant_id_assignment_id_fkey FOREIGN KEY (tenant_id, assignment_id) REFERENCES public.assignment(tenant_id, assignment_id) ON DELETE CASCADE;

ALTER TABLE ONLY public.assignment_selection_group
    ADD CONSTRAINT assignment_selection_group_assignment_fkey FOREIGN KEY (tenant_id, assignment_id) REFERENCES public.assignment(tenant_id, assignment_id) ON DELETE CASCADE;

ALTER TABLE ONLY public.assignment_selection_candidate
    ADD CONSTRAINT assignment_selection_candidate_group_fkey FOREIGN KEY (tenant_id, assignment_id, selection_group_id) REFERENCES public.assignment_selection_group(tenant_id, assignment_id, selection_group_id) ON DELETE CASCADE;

ALTER TABLE ONLY public.assignment_selection_candidate
    ADD CONSTRAINT assignment_selection_candidate_version_fkey FOREIGN KEY (problem_id, version_id) REFERENCES public.problem_version(problem_id, version_id) ON DELETE RESTRICT;

ALTER TABLE ONLY public.course_member
    ADD CONSTRAINT course_member_tenant_id_course_id_fkey FOREIGN KEY (tenant_id, course_id) REFERENCES public.course(tenant_id, course_id) ON DELETE CASCADE;

ALTER TABLE ONLY public.course_group
    ADD CONSTRAINT course_group_course_fkey FOREIGN KEY (tenant_id, course_id) REFERENCES public.course(tenant_id, course_id) ON DELETE CASCADE;

ALTER TABLE ONLY public.course_group_member
    ADD CONSTRAINT course_group_member_group_fkey FOREIGN KEY (tenant_id, course_id, course_group_id) REFERENCES public.course_group(tenant_id, course_id, course_group_id) ON DELETE CASCADE;

ALTER TABLE ONLY public.course_group_member
    ADD CONSTRAINT course_group_member_course_member_fkey FOREIGN KEY (tenant_id, course_id, user_id) REFERENCES public.course_member(tenant_id, course_id, user_id) ON DELETE CASCADE;

ALTER TABLE ONLY public.enrollment
    ADD CONSTRAINT enrollment_tenant_id_assignment_id_fkey FOREIGN KEY (tenant_id, assignment_id) REFERENCES public.assignment(tenant_id, assignment_id);

ALTER TABLE ONLY public.assignment_policy_exception
    ADD CONSTRAINT assignment_policy_exception_assignment_fkey FOREIGN KEY (tenant_id, course_id, assignment_id) REFERENCES public.assignment(tenant_id, course_id, assignment_id) ON DELETE CASCADE;

ALTER TABLE ONLY public.assignment_policy_exception
    ADD CONSTRAINT assignment_policy_exception_student_fkey FOREIGN KEY (tenant_id, assignment_id, student_id) REFERENCES public.enrollment(tenant_id, assignment_id, student_id) ON DELETE CASCADE;

ALTER TABLE ONLY public.assignment_policy_exception
    ADD CONSTRAINT assignment_policy_exception_group_fkey FOREIGN KEY (tenant_id, course_id, course_group_id) REFERENCES public.course_group(tenant_id, course_id, course_group_id) ON DELETE CASCADE;

ALTER TABLE ONLY public.student_assignment_summary
    ADD CONSTRAINT student_assignment_summary_tenant_id_enrollment_id_fkey FOREIGN KEY (tenant_id, enrollment_id) REFERENCES public.enrollment(tenant_id, enrollment_id) ON DELETE CASCADE;

CREATE FUNCTION public.ple_guard_assignment_position() RETURNS trigger
    LANGUAGE plpgsql
    SET search_path TO 'pg_catalog', 'public'
    AS $$
BEGIN
    PERFORM 1
      FROM public.assignment
     WHERE tenant_id = NEW.tenant_id
       AND assignment_id = NEW.assignment_id
     FOR UPDATE;
    IF TG_TABLE_NAME = 'assignment_item' THEN
        IF EXISTS (
            SELECT 1 FROM public.assignment_selection_group other
             WHERE other.tenant_id = NEW.tenant_id
               AND other.assignment_id = NEW.assignment_id
               AND other.position = NEW.position
        ) THEN
            RAISE EXCEPTION 'assignment position is already occupied'
                USING ERRCODE = '23505';
        END IF;
    ELSIF EXISTS (
        SELECT 1 FROM public.assignment_item other
         WHERE other.tenant_id = NEW.tenant_id
           AND other.assignment_id = NEW.assignment_id
           AND other.position = NEW.position
    ) THEN
        RAISE EXCEPTION 'assignment position is already occupied'
            USING ERRCODE = '23505';
    END IF;
    RETURN NEW;
END
$$;

CREATE FUNCTION public.ple_guard_assignment_content_lock() RETURNS trigger
    LANGUAGE plpgsql
    SET search_path TO 'pg_catalog', 'public'
    AS $$
DECLARE row_tenant uuid := COALESCE(NEW.tenant_id, OLD.tenant_id);
DECLARE row_assignment uuid := COALESCE(NEW.assignment_id, OLD.assignment_id);
DECLARE content_changed boolean;
BEGIN
    IF TG_OP = 'UPDATE' THEN
        content_changed := (NEW.problem_id, NEW.version_id)
            IS DISTINCT FROM (OLD.problem_id, OLD.version_id);
    ELSE
        content_changed := true;
    END IF;
    IF content_changed AND EXISTS (
        SELECT 1 FROM public.assignment_run run
         JOIN public.enrollment enrollment
           ON enrollment.tenant_id = run.tenant_id
          AND enrollment.enrollment_id = run.enrollment_id
         WHERE enrollment.tenant_id = row_tenant
           AND enrollment.assignment_id = row_assignment
    ) THEN
        RAISE EXCEPTION 'assignment content is locked after the first student run'
            USING ERRCODE = '55000';
    END IF;
    IF TG_OP = 'DELETE' THEN
        RETURN OLD;
    END IF;
    RETURN NEW;
END
$$;

CREATE FUNCTION public.ple_validate_selection_capacity() RETURNS trigger
    LANGUAGE plpgsql
    SET search_path TO 'pg_catalog', 'public'
    AS $$
DECLARE group_tenant uuid := COALESCE(NEW.tenant_id, OLD.tenant_id);
DECLARE group_id uuid := COALESCE(NEW.selection_group_id, OLD.selection_group_id);
DECLARE required_count integer;
DECLARE available_count integer;
BEGIN
    SELECT draw_count INTO required_count
      FROM public.assignment_selection_group
     WHERE tenant_id = group_tenant
       AND selection_group_id = group_id;
    IF NOT FOUND THEN
        RETURN NULL;
    END IF;
    SELECT count(*)::integer INTO available_count
      FROM public.assignment_selection_candidate
     WHERE tenant_id = group_tenant
       AND selection_group_id = group_id
       AND delivery_state = 'active';
    IF available_count < required_count THEN
        RAISE EXCEPTION 'selection group has fewer active candidates than its draw count'
            USING ERRCODE = '23514';
    END IF;
    RETURN NULL;
END
$$;

CREATE FUNCTION public.ple_guard_assignment_retirement() RETURNS trigger
    LANGUAGE plpgsql
    SET search_path TO 'pg_catalog', 'public'
    AS $$
DECLARE retired_item uuid;
BEGIN
    IF TG_TABLE_NAME = 'assignment_item' THEN
        retired_item := NEW.assignment_item_id;
    ELSIF TG_TABLE_NAME = 'assignment_selection_candidate' THEN
        retired_item := NEW.candidate_id;
    ELSE
        RAISE EXCEPTION 'unsupported assignment retirement table'
            USING ERRCODE = '22023';
    END IF;
    IF OLD.delivery_state = 'active' AND NEW.delivery_state = 'retired'
       AND EXISTS (
            SELECT 1
              FROM public.assignment_run_item delivered
              JOIN public.assignment_run run
                ON run.tenant_id = delivered.tenant_id
               AND run.run_id = delivered.run_id
              JOIN public.enrollment enrollment
                ON enrollment.tenant_id = run.tenant_id
               AND enrollment.enrollment_id = run.enrollment_id
              JOIN public.question_attempt attempt
                ON attempt.tenant_id = delivered.tenant_id
               AND attempt.run_id = delivered.run_id
               AND attempt.assignment_position = delivered.issued_position
             WHERE delivered.tenant_id = NEW.tenant_id
               AND enrollment.assignment_id = NEW.assignment_id
               AND delivered.assignment_item_id = retired_item
               AND attempt.attempt_status = 'in_progress'
       )
    THEN
        RAISE EXCEPTION 'an active attempt blocks Delete and Regrade'
            USING ERRCODE = '55000';
    END IF;
    RETURN NEW;
END
$$;

CREATE TRIGGER assignment_item_position_guard
    BEFORE INSERT OR UPDATE OF assignment_id, position ON public.assignment_item
    FOR EACH ROW EXECUTE FUNCTION public.ple_guard_assignment_position();

CREATE TRIGGER assignment_selection_group_position_guard
    BEFORE INSERT OR UPDATE OF assignment_id, position ON public.assignment_selection_group
    FOR EACH ROW EXECUTE FUNCTION public.ple_guard_assignment_position();

CREATE TRIGGER assignment_item_content_lock
    BEFORE INSERT OR DELETE OR UPDATE OF problem_id, version_id ON public.assignment_item
    FOR EACH ROW EXECUTE FUNCTION public.ple_guard_assignment_content_lock();

CREATE TRIGGER assignment_selection_candidate_content_lock
    BEFORE INSERT OR DELETE OR UPDATE OF problem_id, version_id ON public.assignment_selection_candidate
    FOR EACH ROW EXECUTE FUNCTION public.ple_guard_assignment_content_lock();

CREATE TRIGGER assignment_item_retirement_guard
    BEFORE UPDATE OF delivery_state ON public.assignment_item
    FOR EACH ROW EXECUTE FUNCTION public.ple_guard_assignment_retirement();

CREATE TRIGGER assignment_selection_candidate_retirement_guard
    BEFORE UPDATE OF delivery_state ON public.assignment_selection_candidate
    FOR EACH ROW EXECUTE FUNCTION public.ple_guard_assignment_retirement();

CREATE CONSTRAINT TRIGGER assignment_selection_group_capacity
    AFTER INSERT OR UPDATE OF draw_count ON public.assignment_selection_group
    DEFERRABLE INITIALLY DEFERRED
    FOR EACH ROW EXECUTE FUNCTION public.ple_validate_selection_capacity();

CREATE CONSTRAINT TRIGGER assignment_selection_candidate_capacity
    AFTER INSERT OR DELETE OR UPDATE OF delivery_state ON public.assignment_selection_candidate
    DEFERRABLE INITIALLY DEFERRED
    FOR EACH ROW EXECUTE FUNCTION public.ple_validate_selection_capacity();

ALTER TABLE public.assignment ENABLE ROW LEVEL SECURITY;

CREATE POLICY assignment_app_tenant ON public.assignment TO ple_app USING ((tenant_id = public.ple_current_tenant())) WITH CHECK ((tenant_id = public.ple_current_tenant()));

ALTER TABLE public.assignment_item ENABLE ROW LEVEL SECURITY;

CREATE POLICY assignment_item_app_tenant ON public.assignment_item TO ple_app USING ((tenant_id = public.ple_current_tenant())) WITH CHECK ((tenant_id = public.ple_current_tenant()));

ALTER TABLE public.assignment_selection_group ENABLE ROW LEVEL SECURITY;

CREATE POLICY assignment_selection_group_app_tenant ON public.assignment_selection_group TO ple_app USING ((tenant_id = public.ple_current_tenant())) WITH CHECK ((tenant_id = public.ple_current_tenant()));

ALTER TABLE public.assignment_selection_candidate ENABLE ROW LEVEL SECURITY;

CREATE POLICY assignment_selection_candidate_app_tenant ON public.assignment_selection_candidate TO ple_app USING ((tenant_id = public.ple_current_tenant())) WITH CHECK ((tenant_id = public.ple_current_tenant()));

ALTER TABLE public.course ENABLE ROW LEVEL SECURITY;

ALTER TABLE public.course_member ENABLE ROW LEVEL SECURITY;

CREATE POLICY course_member_tenant ON public.course_member USING ((tenant_id = public.ple_current_tenant())) WITH CHECK ((tenant_id = public.ple_current_tenant()));

ALTER TABLE public.course_group ENABLE ROW LEVEL SECURITY;

CREATE POLICY course_group_tenant ON public.course_group USING ((tenant_id = public.ple_current_tenant())) WITH CHECK ((tenant_id = public.ple_current_tenant()));

ALTER TABLE public.course_group_member ENABLE ROW LEVEL SECURITY;

CREATE POLICY course_group_member_tenant ON public.course_group_member USING ((tenant_id = public.ple_current_tenant())) WITH CHECK ((tenant_id = public.ple_current_tenant()));

CREATE POLICY course_tenant ON public.course USING ((tenant_id = public.ple_current_tenant())) WITH CHECK ((tenant_id = public.ple_current_tenant()));

ALTER TABLE public.enrollment ENABLE ROW LEVEL SECURITY;

ALTER TABLE public.assignment_policy_exception ENABLE ROW LEVEL SECURITY;

CREATE POLICY assignment_policy_exception_tenant ON public.assignment_policy_exception USING ((tenant_id = public.ple_current_tenant())) WITH CHECK ((tenant_id = public.ple_current_tenant()));

ALTER TABLE public.student_assignment_summary ENABLE ROW LEVEL SECURITY;

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.assignment TO ple_app;
GRANT SELECT ON TABLE public.assignment TO ple_student;
GRANT SELECT,DELETE ON TABLE public.assignment TO ple_retention_broker;

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.assignment_item TO ple_app;
GRANT SELECT ON TABLE public.assignment_item TO ple_student;
GRANT SELECT,DELETE ON TABLE public.assignment_item TO ple_retention_broker;

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.assignment_selection_group TO ple_app;
GRANT SELECT ON TABLE public.assignment_selection_group TO ple_student;
GRANT SELECT,DELETE ON TABLE public.assignment_selection_group TO ple_retention_broker;

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.assignment_selection_candidate TO ple_app;
GRANT SELECT ON TABLE public.assignment_selection_candidate TO ple_student;
GRANT SELECT,DELETE ON TABLE public.assignment_selection_candidate TO ple_retention_broker;

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.course TO ple_app;
GRANT SELECT ON TABLE public.course TO ple_student;
GRANT SELECT ON TABLE public.course TO ple_retention_broker;

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.course_member TO ple_app;
GRANT SELECT ON TABLE public.course_member TO ple_student;
GRANT SELECT,DELETE ON TABLE public.course_member TO ple_retention_broker;

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.course_group TO ple_app;
GRANT SELECT,DELETE ON TABLE public.course_group TO ple_retention_broker;

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.course_group_member TO ple_app;
GRANT SELECT,DELETE ON TABLE public.course_group_member TO ple_retention_broker;

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.enrollment TO ple_app;
GRANT SELECT ON TABLE public.enrollment TO ple_student;
GRANT SELECT,DELETE ON TABLE public.enrollment TO ple_retention_broker;

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.assignment_policy_exception TO ple_app;
GRANT SELECT,DELETE ON TABLE public.assignment_policy_exception TO ple_retention_broker;

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.student_assignment_summary TO ple_app;
GRANT SELECT ON TABLE public.student_assignment_summary TO ple_student;
GRANT SELECT,DELETE ON TABLE public.student_assignment_summary TO ple_retention_broker;
