-- Initial database epoch: activity feedback.

SET check_function_bodies = false;

CREATE TABLE public.assignment_run (
    tenant_id uuid NOT NULL,
    run_id uuid NOT NULL,
    enrollment_id uuid NOT NULL,
    run_number bigint NOT NULL,
    started_at timestamp with time zone NOT NULL,
    completed_at timestamp with time zone,
    payload jsonb NOT NULL,
    payload_sha256 character(64) NOT NULL,
    CONSTRAINT assignment_run_run_number_check CHECK ((run_number > 0))
);

ALTER TABLE ONLY public.assignment_run FORCE ROW LEVEL SECURITY;

CREATE TABLE public.assignment_run_item (
    tenant_id uuid NOT NULL,
    run_id uuid NOT NULL,
    assignment_item_id uuid NOT NULL,
    source_position integer NOT NULL,
    issued_position integer NOT NULL,
    problem_id uuid NOT NULL,
    version_id uuid NOT NULL,
    selection_group_id uuid,
    selection_seed bigint,
    delivery_status text DEFAULT 'issued'::text NOT NULL,
    issued_at timestamp with time zone DEFAULT transaction_timestamp() NOT NULL,
    CONSTRAINT assignment_run_item_source_position_check CHECK ((source_position >= 0)),
    CONSTRAINT assignment_run_item_issued_position_check CHECK ((issued_position >= 0)),
    CONSTRAINT assignment_run_item_selection_shape_check CHECK (((selection_group_id IS NULL) = (selection_seed IS NULL))),
    CONSTRAINT assignment_run_item_delivery_status_check CHECK ((delivery_status = ANY (ARRAY['issued'::text, 'active'::text, 'submitted'::text, 'cleared'::text])))
);

ALTER TABLE ONLY public.assignment_run_item FORCE ROW LEVEL SECURITY;

CREATE TABLE public.attempt_feedback (
    tenant_id uuid NOT NULL,
    attempt_id uuid NOT NULL,
    hint jsonb,
    correct_response jsonb,
    rationale jsonb,
    content_sha256 character(64) NOT NULL,
    created_at timestamp with time zone DEFAULT transaction_timestamp() NOT NULL,
    course_id uuid NOT NULL,
    CONSTRAINT attempt_feedback_correct_response_check CHECK (((correct_response IS NULL) OR (jsonb_typeof(correct_response) = 'array'::text))),
    CONSTRAINT attempt_feedback_hint_check CHECK (((hint IS NULL) OR (jsonb_typeof(hint) = 'array'::text))),
    CONSTRAINT attempt_feedback_rationale_check CHECK (((rationale IS NULL) OR (jsonb_typeof(rationale) = 'array'::text)))
);

ALTER TABLE ONLY public.attempt_feedback FORCE ROW LEVEL SECURITY;

CREATE TABLE public.record_access_log (
    tenant_id uuid NOT NULL,
    access_log_id uuid NOT NULL,
    occurred_at timestamp with time zone NOT NULL,
    payload jsonb NOT NULL,
    payload_sha256 character(64) NOT NULL,
    delivery_scope text NOT NULL,
    delivery_id uuid NOT NULL,
    course_id uuid,
    CONSTRAINT record_access_log_delivery_scope_check CHECK ((delivery_scope = ANY (ARRAY['catalog'::text, 'student_record'::text]))),
    CONSTRAINT record_access_log_delivery_scope_shape_check CHECK ((((delivery_scope = 'catalog'::text) AND (delivery_id IS NOT NULL) AND (course_id IS NULL)) OR ((delivery_scope = 'student_record'::text) AND (delivery_id IS NOT NULL) AND (course_id IS NOT NULL))))
)
PARTITION BY RANGE (occurred_at);

ALTER TABLE ONLY public.record_access_log FORCE ROW LEVEL SECURITY;

CREATE TABLE public.audit_event (
    tenant_id uuid NOT NULL,
    audit_event_id uuid NOT NULL,
    occurred_at timestamp with time zone NOT NULL,
    actor_id uuid NOT NULL,
    course_id uuid,
    action text NOT NULL,
    target_kind text NOT NULL,
    target_id uuid NOT NULL,
    payload jsonb NOT NULL,
    payload_sha256 character(64) NOT NULL,
    CONSTRAINT audit_event_action_check CHECK ((char_length(action) BETWEEN 1 AND 80 AND action = btrim(action))),
    CONSTRAINT audit_event_target_kind_check CHECK ((char_length(target_kind) BETWEEN 1 AND 80 AND target_kind = btrim(target_kind))),
    CONSTRAINT audit_event_payload_check CHECK ((jsonb_typeof(payload) = 'object'::text))
)
PARTITION BY RANGE (occurred_at);

ALTER TABLE ONLY public.audit_event FORCE ROW LEVEL SECURITY;

CREATE TABLE public.external_tool_exchange (
    tenant_id uuid NOT NULL,
    attempt_id uuid NOT NULL,
    actor_id uuid NOT NULL,
    provider text NOT NULL,
    problem_id uuid NOT NULL,
    version_id uuid NOT NULL,
    seed bigint NOT NULL,
    source_object_id uuid NOT NULL,
    source_sha256 character(64) NOT NULL,
    integration_profile text NOT NULL,
    response_sha256 bytea NOT NULL,
    idempotency_key text NOT NULL,
    correlation bytea NOT NULL,
    state text NOT NULL,
    lease_token bytea,
    lease_expires_at timestamp with time zone,
    result_payload jsonb,
    result_sha256 character(64),
    transcript_object_id uuid,
    transcript_sha256 character(64),
    created_at timestamp with time zone DEFAULT transaction_timestamp() NOT NULL,
    updated_at timestamp with time zone DEFAULT transaction_timestamp() NOT NULL,
    retention_at timestamp with time zone,
    verification_token_sha256 bytea,
    course_id uuid NOT NULL,
    CONSTRAINT external_tool_exchange_check CHECK (((state = 'verifying'::text) = ((lease_token IS NOT NULL) AND (lease_expires_at IS NOT NULL)))),
    CONSTRAINT external_tool_exchange_check1 CHECK (((state <> 'verified_pending'::text) OR ((result_payload IS NOT NULL) AND (result_sha256 IS NOT NULL)))),
    CONSTRAINT external_tool_exchange_correlation_check CHECK (((octet_length(correlation) >= 1) AND (octet_length(correlation) <= 512))),
    CONSTRAINT external_tool_exchange_idempotency_key_check CHECK (((octet_length(idempotency_key) >= 1) AND (octet_length(idempotency_key) <= 200))),
    CONSTRAINT external_tool_exchange_integration_profile_check CHECK (((octet_length(integration_profile) >= 1) AND (octet_length(integration_profile) <= 160))),
    CONSTRAINT external_tool_exchange_lease_token_check CHECK (((lease_token IS NULL) OR (octet_length(lease_token) = 32))),
    CONSTRAINT external_tool_exchange_provider_check CHECK (((octet_length(provider) >= 1) AND (octet_length(provider) <= 160))),
    CONSTRAINT external_tool_exchange_response_sha256_check CHECK ((octet_length(response_sha256) = 32)),
    CONSTRAINT external_tool_exchange_state_check CHECK ((state = ANY (ARRAY['verifying'::text, 'verified_pending'::text, 'committed'::text]))),
    CONSTRAINT external_tool_exchange_verified_token_check CHECK (((state = 'verified_pending'::text) = (verification_token_sha256 IS NOT NULL)))
);

ALTER TABLE ONLY public.external_tool_exchange FORCE ROW LEVEL SECURITY;

CREATE TABLE public.external_tool_launch_session (
    launch_session_id uuid NOT NULL,
    tenant_id uuid NOT NULL,
    attempt_id uuid NOT NULL,
    actor_id uuid NOT NULL,
    provider text NOT NULL,
    problem_id uuid NOT NULL,
    version_id uuid NOT NULL,
    seed bigint NOT NULL,
    source_object_id uuid NOT NULL,
    source_sha256 character(64) NOT NULL,
    integration_profile text NOT NULL,
    response_sha256 bytea NOT NULL,
    token_sha256 bytea NOT NULL,
    encrypted_provider_state bytea,
    expires_at timestamp with time zone NOT NULL,
    revoked_at timestamp with time zone,
    created_at timestamp with time zone DEFAULT transaction_timestamp() NOT NULL,
    course_id uuid NOT NULL,
    CONSTRAINT external_tool_launch_session_integration_profile_check CHECK (((octet_length(integration_profile) >= 1) AND (octet_length(integration_profile) <= 160))),
    CONSTRAINT external_tool_launch_session_provider_check CHECK (((octet_length(provider) >= 1) AND (octet_length(provider) <= 160))),
    CONSTRAINT external_tool_launch_session_response_sha256_check CHECK ((octet_length(response_sha256) = 32)),
    CONSTRAINT external_tool_launch_session_token_sha256_check CHECK ((octet_length(token_sha256) = 32))
);

ALTER TABLE ONLY public.external_tool_launch_session FORCE ROW LEVEL SECURITY;

CREATE TABLE public.feedback_release (
    tenant_id uuid NOT NULL,
    attempt_id uuid NOT NULL,
    released_by uuid NOT NULL,
    released_at timestamp with time zone DEFAULT transaction_timestamp() NOT NULL
);

ALTER TABLE ONLY public.feedback_release FORCE ROW LEVEL SECURITY;

CREATE TABLE public.submission_evaluation (
    tenant_id uuid NOT NULL,
    attempt_id uuid NOT NULL,
    submission_id uuid NOT NULL,
    credit_fraction numeric(16,12) NOT NULL,
    correct boolean NOT NULL,
    grading_status text DEFAULT 'graded'::text NOT NULL,
    payload jsonb NOT NULL,
    payload_sha256 character(64) NOT NULL,
    evaluated_at timestamp with time zone DEFAULT transaction_timestamp() NOT NULL,
    course_id uuid NOT NULL,
    CONSTRAINT submission_evaluation_credit_check CHECK ((credit_fraction >= -1000 AND credit_fraction <= 1000)),
    CONSTRAINT submission_evaluation_grading_status_check CHECK ((grading_status = ANY (ARRAY['graded'::text, 'needs_manual_grading'::text, 'exempt'::text])))
);

ALTER TABLE ONLY public.submission_evaluation FORCE ROW LEVEL SECURITY;

CREATE TABLE public.attempt_score_current (
    tenant_id uuid NOT NULL,
    attempt_id uuid NOT NULL,
    assignment_id uuid NOT NULL,
    assignment_item_id uuid NOT NULL,
    scoring_generation bigint NOT NULL,
    earned_points numeric(16,4) NOT NULL,
    possible_points numeric(16,4) NOT NULL,
    calculated_at timestamp with time zone DEFAULT transaction_timestamp() NOT NULL,
    course_id uuid NOT NULL,
    CONSTRAINT attempt_score_current_generation_check CHECK ((scoring_generation > 0)),
    CONSTRAINT attempt_score_current_possible_check CHECK ((possible_points >= 0))
);

ALTER TABLE ONLY public.attempt_score_current FORCE ROW LEVEL SECURITY;

CREATE TABLE public.question_attempt (
    tenant_id uuid NOT NULL,
    attempt_id uuid NOT NULL,
    run_id uuid NOT NULL,
    problem_id uuid NOT NULL,
    version_id uuid NOT NULL,
    occurred_at timestamp with time zone NOT NULL,
    payload jsonb NOT NULL,
    payload_sha256 character(64) NOT NULL,
    attempt_status text DEFAULT 'in_progress'::text NOT NULL,
    submitted_at timestamp with time zone,
    assignment_position integer,
    course_id uuid NOT NULL,
    CONSTRAINT question_attempt_status_check CHECK ((attempt_status = ANY (ARRAY['in_progress'::text, 'submitted'::text, 'auto_submitted'::text, 'needs_manual_grading'::text, 'cleared'::text, 'exempt'::text]))),
    CONSTRAINT question_attempt_submission_time_check CHECK ((((attempt_status = 'in_progress'::text) AND (submitted_at IS NULL)) OR ((attempt_status = ANY (ARRAY['submitted'::text, 'auto_submitted'::text, 'needs_manual_grading'::text])) AND (submitted_at IS NOT NULL)) OR (attempt_status = ANY (ARRAY['cleared'::text, 'exempt'::text]))))
)
PARTITION BY RANGE (occurred_at);

ALTER TABLE ONLY public.question_attempt FORCE ROW LEVEL SECURITY;

CREATE TABLE public.question_prefetch (
    tenant_id uuid NOT NULL,
    run_id uuid NOT NULL,
    predecessor_attempt_id uuid NOT NULL,
    predecessor_occurred_at timestamp with time zone NOT NULL,
    assignment_position integer NOT NULL,
    created_at timestamp with time zone DEFAULT transaction_timestamp() NOT NULL,
    payload jsonb NOT NULL,
    payload_sha256 character(64) NOT NULL,
    CONSTRAINT question_prefetch_assignment_position_check CHECK ((assignment_position >= 0))
);

ALTER TABLE ONLY public.question_prefetch FORCE ROW LEVEL SECURITY;

CREATE TABLE public.submission (
    tenant_id uuid NOT NULL,
    submission_id uuid NOT NULL,
    occurred_at timestamp with time zone NOT NULL,
    payload jsonb NOT NULL,
    payload_sha256 character(64) NOT NULL,
    attempt_id uuid,
    idempotency_key text,
    course_id uuid NOT NULL
)
PARTITION BY RANGE (occurred_at);

ALTER TABLE ONLY public.submission FORCE ROW LEVEL SECURITY;

CREATE TABLE public.submission_idempotency (
    tenant_id uuid NOT NULL,
    attempt_id uuid NOT NULL,
    idempotency_key text NOT NULL,
    response_sha256 character(64) NOT NULL,
    submitted_at timestamp with time zone DEFAULT transaction_timestamp() NOT NULL,
    payload jsonb NOT NULL,
    payload_sha256 character(64) NOT NULL,
    course_id uuid NOT NULL,
    CONSTRAINT submission_idempotency_idempotency_key_check CHECK ((((octet_length(idempotency_key) >= 1) AND (octet_length(idempotency_key) <= 200)) AND (idempotency_key !~ '[[:space:][:cntrl:]]'::text)))
)
PARTITION BY HASH (tenant_id);

ALTER TABLE ONLY public.submission_idempotency FORCE ROW LEVEL SECURITY;

CREATE TABLE public.submission_next_attempt (
    tenant_id uuid NOT NULL,
    predecessor_attempt_id uuid NOT NULL,
    next_attempt_id uuid,
    next_attempt_occurred_at timestamp with time zone,
    finalized_at timestamp with time zone DEFAULT transaction_timestamp() NOT NULL,
    CONSTRAINT submission_next_attempt_check CHECK (((next_attempt_id IS NULL) = (next_attempt_occurred_at IS NULL)))
);

ALTER TABLE ONLY public.submission_next_attempt FORCE ROW LEVEL SECURITY;

CREATE TABLE public.submission_receipt_snapshot (
    tenant_id uuid NOT NULL,
    attempt_id uuid NOT NULL,
    run_payload jsonb NOT NULL,
    run_payload_sha256 character(64) NOT NULL,
    summary_payload jsonb NOT NULL,
    summary_payload_sha256 character(64) NOT NULL,
    created_at timestamp with time zone DEFAULT transaction_timestamp() NOT NULL,
    CONSTRAINT submission_receipt_snapshot_run_payload_check CHECK ((jsonb_typeof(run_payload) = 'object'::text)),
    CONSTRAINT submission_receipt_snapshot_summary_payload_check CHECK ((jsonb_typeof(summary_payload) = 'object'::text))
);

ALTER TABLE ONLY public.submission_receipt_snapshot FORCE ROW LEVEL SECURITY;

ALTER TABLE ONLY public.assignment_run
    ADD CONSTRAINT assignment_run_pkey PRIMARY KEY (tenant_id, run_id);

ALTER TABLE ONLY public.assignment_run
    ADD CONSTRAINT assignment_run_tenant_id_enrollment_id_run_number_key UNIQUE (tenant_id, enrollment_id, run_number);

ALTER TABLE ONLY public.assignment_run_item
    ADD CONSTRAINT assignment_run_item_pkey PRIMARY KEY (tenant_id, run_id, issued_position);

ALTER TABLE ONLY public.assignment_run_item
    ADD CONSTRAINT assignment_run_item_identity_key UNIQUE (tenant_id, run_id, assignment_item_id);

ALTER TABLE ONLY public.attempt_feedback
    ADD CONSTRAINT attempt_feedback_pkey PRIMARY KEY (tenant_id, attempt_id);

ALTER TABLE ONLY public.record_access_log
    ADD CONSTRAINT record_access_log_pkey PRIMARY KEY (tenant_id, access_log_id, occurred_at);

ALTER TABLE ONLY public.audit_event
    ADD CONSTRAINT audit_event_pkey PRIMARY KEY (tenant_id, audit_event_id, occurred_at);

ALTER TABLE ONLY public.external_tool_exchange
    ADD CONSTRAINT external_tool_exchange_pkey PRIMARY KEY (tenant_id, attempt_id);

ALTER TABLE ONLY public.external_tool_launch_session
    ADD CONSTRAINT external_tool_launch_session_pkey PRIMARY KEY (launch_session_id);

ALTER TABLE ONLY public.external_tool_launch_session
    ADD CONSTRAINT external_tool_launch_session_tenant_id_launch_session_id_key UNIQUE (tenant_id, launch_session_id);

ALTER TABLE ONLY public.feedback_release
    ADD CONSTRAINT feedback_release_pkey PRIMARY KEY (tenant_id, attempt_id);

ALTER TABLE ONLY public.submission_evaluation
    ADD CONSTRAINT submission_evaluation_pkey PRIMARY KEY (tenant_id, attempt_id);

ALTER TABLE ONLY public.attempt_score_current
    ADD CONSTRAINT attempt_score_current_pkey PRIMARY KEY (tenant_id, attempt_id);

ALTER TABLE ONLY public.question_attempt
    ADD CONSTRAINT question_attempt_pkey PRIMARY KEY (tenant_id, attempt_id, occurred_at);

ALTER TABLE public.question_attempt
    ADD CONSTRAINT question_attempt_position_required_check CHECK (((assignment_position IS NOT NULL) AND (assignment_position >= 0))) NOT VALID;

ALTER TABLE ONLY public.question_prefetch
    ADD CONSTRAINT question_prefetch_pkey PRIMARY KEY (tenant_id, run_id, predecessor_attempt_id, assignment_position);

ALTER TABLE ONLY public.submission
    ADD CONSTRAINT submission_pkey PRIMARY KEY (tenant_id, submission_id, occurred_at);

ALTER TABLE public.submission
    ADD CONSTRAINT submission_attempt_required_check CHECK ((attempt_id IS NOT NULL)) NOT VALID;

ALTER TABLE ONLY public.submission_idempotency
    ADD CONSTRAINT submission_idempotency_pkey PRIMARY KEY (tenant_id, attempt_id);

ALTER TABLE public.submission
    ADD CONSTRAINT submission_idempotency_required_check CHECK ((idempotency_key IS NOT NULL)) NOT VALID;

ALTER TABLE ONLY public.submission_next_attempt
    ADD CONSTRAINT submission_next_attempt_pkey PRIMARY KEY (tenant_id, predecessor_attempt_id);

ALTER TABLE ONLY public.submission_receipt_snapshot
    ADD CONSTRAINT submission_receipt_snapshot_pkey PRIMARY KEY (tenant_id, attempt_id);

CREATE UNIQUE INDEX assignment_run_one_active_idx ON public.assignment_run USING btree (tenant_id, enrollment_id) WHERE (completed_at IS NULL);

CREATE INDEX attempt_feedback_course_idx ON public.attempt_feedback USING btree (tenant_id, course_id, attempt_id);

CREATE INDEX record_access_log_tenant_course_time_idx ON public.record_access_log USING btree (tenant_id, course_id, occurred_at) WHERE (delivery_scope = 'student_record'::text);

CREATE INDEX record_access_log_tenant_time_idx ON public.record_access_log USING btree (tenant_id, occurred_at);

CREATE INDEX audit_event_tenant_course_time_idx ON public.audit_event USING btree (tenant_id, course_id, occurred_at);

CREATE INDEX audit_event_tenant_time_idx ON public.audit_event USING btree (tenant_id, occurred_at);

CREATE INDEX audit_event_tenant_action_idx ON public.audit_event USING btree (tenant_id, audit_event_id, occurred_at);

CREATE INDEX external_tool_exchange_course_idx ON public.external_tool_exchange USING btree (tenant_id, course_id, attempt_id, transcript_object_id);

CREATE INDEX external_tool_exchange_lease_idx ON public.external_tool_exchange USING btree (state, lease_expires_at) WHERE (state = 'verifying'::text);

CREATE INDEX external_tool_exchange_retention_idx ON public.external_tool_exchange USING btree (tenant_id, retention_at) WHERE (transcript_object_id IS NOT NULL);

CREATE INDEX external_tool_launch_session_course_idx ON public.external_tool_launch_session USING btree (tenant_id, course_id, attempt_id);

CREATE INDEX external_tool_launch_session_lookup_idx ON public.external_tool_launch_session USING btree (tenant_id, attempt_id, actor_id, expires_at);

CREATE INDEX feedback_release_tenant_released_at_idx ON public.feedback_release USING btree (tenant_id, released_at, attempt_id);

CREATE INDEX assignment_run_item_assignment_item_idx ON public.assignment_run_item USING btree (tenant_id, assignment_item_id, run_id);

CREATE INDEX submission_evaluation_course_idx ON public.submission_evaluation USING btree (tenant_id, course_id, attempt_id);

CREATE INDEX attempt_score_current_assignment_idx ON public.attempt_score_current USING btree (tenant_id, assignment_id, assignment_item_id, attempt_id);

CREATE INDEX question_attempt_lookup_idx ON public.question_attempt USING btree (tenant_id, attempt_id);

CREATE INDEX question_attempt_problem_idx ON public.question_attempt USING btree (problem_id, version_id, occurred_at);

CREATE INDEX question_attempt_course_idx ON public.question_attempt USING btree (tenant_id, course_id, run_id, attempt_id, occurred_at);

CREATE INDEX question_attempt_run_summary_cursor_idx ON public.question_attempt USING btree (tenant_id, run_id, assignment_position, attempt_id);

CREATE INDEX question_attempt_run_position_idx ON public.question_attempt USING btree (tenant_id, run_id, assignment_position, occurred_at, attempt_id);

CREATE INDEX question_attempt_run_idx ON public.question_attempt USING btree (tenant_id, run_id, occurred_at);

CREATE INDEX question_prefetch_predecessor_idx ON public.question_prefetch USING btree (tenant_id, predecessor_attempt_id);

CREATE INDEX question_prefetch_run_idx ON public.question_prefetch USING btree (tenant_id, run_id, predecessor_attempt_id);

CREATE INDEX submission_course_idx ON public.submission USING btree (tenant_id, course_id, attempt_id, occurred_at);

CREATE INDEX submission_tenant_time_idx ON public.submission USING btree (tenant_id, occurred_at);

CREATE INDEX submission_idempotency_course_idx ON public.submission_idempotency USING btree (tenant_id, course_id, attempt_id);

CREATE INDEX submission_idempotency_time_idx ON public.submission_idempotency USING btree (tenant_id, submitted_at, attempt_id);

CREATE INDEX submission_next_attempt_next_idx ON public.submission_next_attempt USING btree (tenant_id, next_attempt_id) WHERE (next_attempt_id IS NOT NULL);

ALTER TABLE ONLY public.assignment_run
    ADD CONSTRAINT assignment_run_tenant_id_enrollment_id_fkey FOREIGN KEY (tenant_id, enrollment_id) REFERENCES public.enrollment(tenant_id, enrollment_id);

ALTER TABLE ONLY public.assignment_run_item
    ADD CONSTRAINT assignment_run_item_run_fkey FOREIGN KEY (tenant_id, run_id) REFERENCES public.assignment_run(tenant_id, run_id) ON DELETE CASCADE;

ALTER TABLE ONLY public.assignment_run_item
    ADD CONSTRAINT assignment_run_item_version_fkey FOREIGN KEY (problem_id, version_id) REFERENCES public.problem_version(problem_id, version_id) ON DELETE RESTRICT;

ALTER TABLE ONLY public.attempt_feedback
    ADD CONSTRAINT attempt_feedback_course_fk FOREIGN KEY (tenant_id, course_id) REFERENCES public.course(tenant_id, course_id);

ALTER TABLE public.record_access_log
    ADD CONSTRAINT record_access_log_delivery_course_fk FOREIGN KEY (tenant_id, course_id) REFERENCES public.course(tenant_id, course_id);

ALTER TABLE public.audit_event
    ADD CONSTRAINT audit_event_course_fk FOREIGN KEY (tenant_id, course_id) REFERENCES public.course(tenant_id, course_id);

ALTER TABLE ONLY public.external_tool_exchange
    ADD CONSTRAINT external_tool_exchange_course_fk FOREIGN KEY (tenant_id, course_id) REFERENCES public.course(tenant_id, course_id);

ALTER TABLE ONLY public.external_tool_launch_session
    ADD CONSTRAINT external_tool_launch_session_course_fk FOREIGN KEY (tenant_id, course_id) REFERENCES public.course(tenant_id, course_id);

ALTER TABLE ONLY public.feedback_release
    ADD CONSTRAINT feedback_release_tenant_id_attempt_id_fkey FOREIGN KEY (tenant_id, attempt_id) REFERENCES public.attempt_feedback(tenant_id, attempt_id) ON DELETE RESTRICT DEFERRABLE INITIALLY DEFERRED;

ALTER TABLE public.submission_evaluation
    ADD CONSTRAINT submission_evaluation_course_fk FOREIGN KEY (tenant_id, course_id) REFERENCES public.course(tenant_id, course_id);

ALTER TABLE ONLY public.attempt_score_current
    ADD CONSTRAINT attempt_score_current_assignment_fk FOREIGN KEY (tenant_id, assignment_id) REFERENCES public.assignment(tenant_id, assignment_id) ON DELETE RESTRICT;

ALTER TABLE ONLY public.attempt_score_current
    ADD CONSTRAINT attempt_score_current_course_fk FOREIGN KEY (tenant_id, course_id) REFERENCES public.course(tenant_id, course_id);

ALTER TABLE public.question_attempt
    ADD CONSTRAINT question_attempt_course_fk FOREIGN KEY (tenant_id, course_id) REFERENCES public.course(tenant_id, course_id);

ALTER TABLE public.question_attempt
    ADD CONSTRAINT question_attempt_tenant_id_run_id_fkey FOREIGN KEY (tenant_id, run_id) REFERENCES public.assignment_run(tenant_id, run_id);

ALTER TABLE public.question_attempt
    ADD CONSTRAINT question_attempt_version_fk FOREIGN KEY (problem_id, version_id) REFERENCES public.problem_version(problem_id, version_id) ON DELETE RESTRICT;

ALTER TABLE ONLY public.question_prefetch
    ADD CONSTRAINT question_prefetch_tenant_id_predecessor_attempt_id_predece_fkey FOREIGN KEY (tenant_id, predecessor_attempt_id, predecessor_occurred_at) REFERENCES public.question_attempt(tenant_id, attempt_id, occurred_at);

ALTER TABLE ONLY public.question_prefetch
    ADD CONSTRAINT question_prefetch_tenant_id_run_id_fkey FOREIGN KEY (tenant_id, run_id) REFERENCES public.assignment_run(tenant_id, run_id);

ALTER TABLE public.submission
    ADD CONSTRAINT submission_course_fk FOREIGN KEY (tenant_id, course_id) REFERENCES public.course(tenant_id, course_id);

ALTER TABLE public.submission_idempotency
    ADD CONSTRAINT submission_idempotency_course_fk FOREIGN KEY (tenant_id, course_id) REFERENCES public.course(tenant_id, course_id);

ALTER TABLE ONLY public.submission_next_attempt
    ADD CONSTRAINT submission_next_attempt_tenant_id_next_attempt_id_next_att_fkey FOREIGN KEY (tenant_id, next_attempt_id, next_attempt_occurred_at) REFERENCES public.question_attempt(tenant_id, attempt_id, occurred_at);

ALTER TABLE ONLY public.submission_next_attempt
    ADD CONSTRAINT submission_next_attempt_tenant_id_predecessor_attempt_id_fkey FOREIGN KEY (tenant_id, predecessor_attempt_id) REFERENCES public.submission_idempotency(tenant_id, attempt_id);

ALTER TABLE ONLY public.submission_receipt_snapshot
    ADD CONSTRAINT submission_receipt_snapshot_tenant_id_attempt_id_fkey FOREIGN KEY (tenant_id, attempt_id) REFERENCES public.submission_idempotency(tenant_id, attempt_id) ON DELETE RESTRICT DEFERRABLE INITIALLY DEFERRED;

ALTER TABLE public.assignment_run ENABLE ROW LEVEL SECURITY;

CREATE POLICY assignment_run_statistics_broker ON public.assignment_run TO ple_statistics_broker USING ((tenant_id = public.ple_current_tenant()));

ALTER TABLE public.assignment_run_item ENABLE ROW LEVEL SECURITY;

ALTER TABLE public.attempt_feedback ENABLE ROW LEVEL SECURITY;

ALTER TABLE public.record_access_log ENABLE ROW LEVEL SECURITY;

CREATE POLICY record_access_log_tenant ON public.record_access_log USING ((tenant_id = public.ple_current_tenant())) WITH CHECK ((tenant_id = public.ple_current_tenant()));

ALTER TABLE public.audit_event ENABLE ROW LEVEL SECURITY;

CREATE POLICY audit_event_tenant ON public.audit_event USING ((tenant_id = public.ple_current_tenant())) WITH CHECK ((tenant_id = public.ple_current_tenant()));

ALTER TABLE public.external_tool_exchange ENABLE ROW LEVEL SECURITY;

ALTER TABLE public.external_tool_launch_session ENABLE ROW LEVEL SECURITY;

ALTER TABLE public.feedback_release ENABLE ROW LEVEL SECURITY;

CREATE POLICY feedback_release_tenant ON public.feedback_release USING (((tenant_id = public.ple_current_tenant()) AND (EXISTS ( SELECT 1
   FROM public.attempt_feedback f
  WHERE ((f.tenant_id = feedback_release.tenant_id) AND (f.attempt_id = feedback_release.attempt_id)))))) WITH CHECK (((tenant_id = public.ple_current_tenant()) AND (EXISTS ( SELECT 1
   FROM public.attempt_feedback f
  WHERE ((f.tenant_id = feedback_release.tenant_id) AND (f.attempt_id = feedback_release.attempt_id))))));

ALTER TABLE public.submission_evaluation ENABLE ROW LEVEL SECURITY;

CREATE POLICY submission_evaluation_tenant ON public.submission_evaluation USING ((tenant_id = public.ple_current_tenant())) WITH CHECK ((tenant_id = public.ple_current_tenant()));

ALTER TABLE public.attempt_score_current ENABLE ROW LEVEL SECURITY;

CREATE POLICY attempt_score_current_tenant ON public.attempt_score_current USING ((tenant_id = public.ple_current_tenant())) WITH CHECK ((tenant_id = public.ple_current_tenant()));

ALTER TABLE public.question_attempt ENABLE ROW LEVEL SECURITY;

CREATE POLICY question_attempt_statistics_broker ON public.question_attempt TO ple_statistics_broker USING ((tenant_id = public.ple_current_tenant()));

ALTER TABLE public.question_prefetch ENABLE ROW LEVEL SECURITY;

ALTER TABLE public.submission ENABLE ROW LEVEL SECURITY;

ALTER TABLE public.submission_idempotency ENABLE ROW LEVEL SECURITY;

ALTER TABLE public.submission_next_attempt ENABLE ROW LEVEL SECURITY;

ALTER TABLE public.submission_receipt_snapshot ENABLE ROW LEVEL SECURITY;

CREATE POLICY submission_receipt_snapshot_tenant ON public.submission_receipt_snapshot USING ((tenant_id = public.ple_current_tenant())) WITH CHECK ((tenant_id = public.ple_current_tenant()));

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.assignment_run TO ple_app;
GRANT SELECT ON TABLE public.assignment_run TO ple_student;
GRANT SELECT ON TABLE public.assignment_run TO ple_statistics_broker;
GRANT SELECT,DELETE ON TABLE public.assignment_run TO ple_retention_broker;

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.assignment_run_item TO ple_app;
GRANT SELECT ON TABLE public.assignment_run_item TO ple_student;
GRANT SELECT,DELETE ON TABLE public.assignment_run_item TO ple_retention_broker;

GRANT SELECT,INSERT ON TABLE public.attempt_feedback TO ple_app;
GRANT SELECT,DELETE ON TABLE public.attempt_feedback TO ple_retention_broker;

GRANT SELECT,INSERT ON TABLE public.record_access_log TO ple_app;
GRANT SELECT,DELETE ON TABLE public.record_access_log TO ple_retention_broker;

GRANT SELECT,INSERT ON TABLE public.audit_event TO ple_app;
GRANT SELECT,DELETE ON TABLE public.audit_event TO ple_retention_broker;

GRANT SELECT,INSERT,UPDATE ON TABLE public.external_tool_exchange TO ple_app;
GRANT SELECT,DELETE ON TABLE public.external_tool_exchange TO ple_retention_broker;

GRANT SELECT,INSERT,UPDATE ON TABLE public.external_tool_launch_session TO ple_app;
GRANT SELECT,DELETE ON TABLE public.external_tool_launch_session TO ple_retention_broker;

GRANT SELECT,INSERT ON TABLE public.feedback_release TO ple_app;
GRANT SELECT,DELETE ON TABLE public.feedback_release TO ple_retention_broker;

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.submission_evaluation TO ple_app;
GRANT SELECT,DELETE ON TABLE public.submission_evaluation TO ple_retention_broker;

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.attempt_score_current TO ple_app;
GRANT SELECT,DELETE ON TABLE public.attempt_score_current TO ple_retention_broker;

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.question_attempt TO ple_app;
GRANT SELECT ON TABLE public.question_attempt TO ple_student;
GRANT SELECT ON TABLE public.question_attempt TO ple_statistics_broker;
GRANT SELECT,DELETE ON TABLE public.question_attempt TO ple_retention_broker;

GRANT SELECT,INSERT,DELETE ON TABLE public.question_prefetch TO ple_app;
GRANT SELECT,DELETE ON TABLE public.question_prefetch TO ple_retention_broker;

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.submission TO ple_app;
GRANT SELECT,DELETE ON TABLE public.submission TO ple_retention_broker;

GRANT SELECT,INSERT ON TABLE public.submission_idempotency TO ple_app;
GRANT SELECT,DELETE ON TABLE public.submission_idempotency TO ple_retention_broker;

GRANT SELECT,INSERT ON TABLE public.submission_next_attempt TO ple_app;
GRANT SELECT,DELETE ON TABLE public.submission_next_attempt TO ple_retention_broker;

GRANT SELECT,INSERT ON TABLE public.submission_receipt_snapshot TO ple_app;
GRANT SELECT,DELETE ON TABLE public.submission_receipt_snapshot TO ple_retention_broker;

DO $$
BEGIN
    FOR partition_number IN 0..15 LOOP
        EXECUTE format(
            'CREATE TABLE public.%I PARTITION OF public.submission_idempotency '
            'FOR VALUES WITH (MODULUS 16, REMAINDER %s)',
            'submission_idempotency_p' || partition_number,
            partition_number
        );
    END LOOP;
END
$$;

CREATE FUNCTION public.ple_ensure_activity_partitions(
    p_first_month date,
    p_month_count integer
)
RETURNS void
LANGUAGE plpgsql
SET search_path = pg_catalog, public AS $$
DECLARE
    parent_name text;
    month_start date;
    month_end date;
    month_offset integer;
BEGIN
    IF p_first_month <> date_trunc('month', p_first_month)::date
       OR p_month_count < 1
       OR p_month_count > 36
    THEN
        RAISE EXCEPTION 'activity partition request must start on a month and span 1..36 months';
    END IF;

    FOREACH parent_name IN ARRAY ARRAY[
        'question_attempt',
        'submission',
        'record_access_log',
        'audit_event'
    ] LOOP
        FOR month_offset IN 0..p_month_count - 1 LOOP
            month_start := (p_first_month + month_offset * interval '1 month')::date;
            month_end := (month_start + interval '1 month')::date;
            EXECUTE format(
                'CREATE TABLE IF NOT EXISTS public.%I PARTITION OF public.%I '
                'FOR VALUES FROM (%L) TO (%L)',
                parent_name || '_' || to_char(month_start, 'YYYY_MM'),
                parent_name,
                month_start,
                month_end
            );
        END LOOP;
        EXECUTE format(
            'CREATE TABLE IF NOT EXISTS public.%I PARTITION OF public.%I DEFAULT',
            parent_name || '_default',
            parent_name
        );
    END LOOP;
END
$$;

REVOKE ALL ON FUNCTION public.ple_ensure_activity_partitions(date, integer) FROM PUBLIC;

SELECT public.ple_ensure_activity_partitions(
    (date_trunc('month', current_date) - interval '1 month')::date,
    26
);
