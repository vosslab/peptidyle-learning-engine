-- WP-PROF-T4: authorization-first learner-work source preparation.
-- ASVS 1.2.4, 2.2.1-2.2.3, 2.3.1-2.3.4, 8.2.1-8.2.2, 8.3.1, and 8.4.1.

BEGIN;

DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'ple_learner_work_broker') THEN
        CREATE ROLE ple_learner_work_broker
            NOLOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE NOINHERIT NOREPLICATION NOBYPASSRLS;
    END IF;
END
$$;
ALTER ROLE ple_learner_work_broker
    NOLOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE NOINHERIT NOREPLICATION NOBYPASSRLS;

-- The owner is a closed capability implementation role in both directions.
DO $$
BEGIN
    IF EXISTS (
        SELECT 1 FROM pg_auth_members AS membership
         WHERE membership.roleid = 'ple_learner_work_broker'::regrole
            OR membership.member = 'ple_learner_work_broker'::regrole
    ) THEN
        RAISE EXCEPTION 'ple_learner_work_broker must not have role memberships';
    END IF;
END
$$;
REVOKE ALL ON SCHEMA public FROM ple_learner_work_broker;
GRANT USAGE ON SCHEMA public TO ple_learner_work_broker;

-- PostgreSQL 17: the attested grader login can SET, but cannot inherit or
-- administer, only the dedicated grader capability.
GRANT ple_grader TO ple_grading_reader
    WITH INHERIT FALSE, SET TRUE, ADMIN FALSE;

-- Learner-artifact writes stay with ple_app.  Source state is capability-only;
-- roster revocation retains its deliberately narrow, named columns.
REVOKE INSERT, UPDATE, DELETE ON public.course, public.course_member,
    public.course_group, public.course_group_member, public.assignment_audience_group
    FROM ple_app;
GRANT UPDATE (status, revoked_at, roster_id) ON public.course_member TO ple_app;

-- Attempt-owned, checksummed V1 source/execution evidence.  The broker can
-- read it under the existing row lock; browser-facing projections never do.
-- ASVS 1.5.2, 2.2.1-2.2.3, 2.3.3.
ALTER TABLE public.question_attempt
    ADD COLUMN issued_question_snapshot_payload jsonb NOT NULL,
    ADD COLUMN issued_question_snapshot_payload_sha256 character(64) NOT NULL,
    ADD COLUMN authored_timing_deadline timestamp with time zone,
    ADD COLUMN authored_timing_grace_seconds integer NOT NULL,
    ADD CONSTRAINT question_attempt_issued_snapshot_payload_object_check
        CHECK (jsonb_typeof(issued_question_snapshot_payload) = 'object'),
    ADD CONSTRAINT question_attempt_issued_snapshot_payload_sha256_check
        CHECK (issued_question_snapshot_payload_sha256 ~ '^[0-9a-f]{64}$'),
    ADD CONSTRAINT question_attempt_authored_timing_grace_check
        CHECK (authored_timing_grace_seconds >= 0),
    ADD CONSTRAINT question_attempt_authored_timing_shape_check
        CHECK (authored_timing_deadline IS NOT NULL OR authored_timing_grace_seconds = 0);

-- The issued source witness and its issue-time-derived timing baseline are
-- private historical evidence.  Mutable lifecycle/effective-policy changes
-- remain valid, but no later resolver can rewrite issuance authority.
CREATE FUNCTION public.ple_guard_question_attempt_issued_evidence() RETURNS trigger
    LANGUAGE plpgsql
    SET search_path TO 'pg_catalog', 'public'
    AS $$
BEGIN
    IF (NEW.issued_question_snapshot_payload,
        NEW.issued_question_snapshot_payload_sha256,
        NEW.authored_timing_deadline,
        NEW.authored_timing_grace_seconds)
       IS DISTINCT FROM
       (OLD.issued_question_snapshot_payload,
        OLD.issued_question_snapshot_payload_sha256,
        OLD.authored_timing_deadline,
        OLD.authored_timing_grace_seconds)
    THEN
        RAISE EXCEPTION 'issued question evidence is immutable' USING ERRCODE = '55000';
    END IF;
    RETURN NEW;
END
$$;
ALTER FUNCTION public.ple_guard_question_attempt_issued_evidence()
    OWNER TO ple_learner_work_broker;
REVOKE ALL ON FUNCTION public.ple_guard_question_attempt_issued_evidence() FROM PUBLIC;
CREATE TRIGGER question_attempt_issued_evidence_immutable
    BEFORE UPDATE ON public.question_attempt
    FOR EACH ROW EXECUTE FUNCTION public.ple_guard_question_attempt_issued_evidence();

-- Answer-bearing issue contracts live outside ordinary application-readable
-- lifecycle rows. ASVS 8.2.1-8.2.3 and 15.4.2: family material is created
-- atomically by the broker and is never projected by a normal Store read.
CREATE TABLE public.issued_attempt_private_execution (
    tenant_id uuid NOT NULL,
    attempt_id uuid NOT NULL,
    attempt_occurred_at timestamp with time zone NOT NULL,
    flat_required boolean NOT NULL,
    flat_payload jsonb,
    flat_payload_sha256 character(64),
    webwork_required boolean NOT NULL,
    webwork_payload jsonb,
    webwork_payload_sha256 character(64),
    webwork_replay_payload jsonb,
    webwork_replay_payload_sha256 character(64),
    qti_required boolean NOT NULL,
    qti_payload bytea,
    qti_payload_sha256 character(64),
    created_at timestamp with time zone NOT NULL DEFAULT transaction_timestamp(),
    PRIMARY KEY (tenant_id, attempt_id, attempt_occurred_at),
    FOREIGN KEY (tenant_id, attempt_id, attempt_occurred_at)
        REFERENCES public.question_attempt(tenant_id, attempt_id, occurred_at) ON DELETE CASCADE,
    CHECK ((flat_required AND flat_payload IS NOT NULL AND flat_payload_sha256 IS NOT NULL)
        OR (NOT flat_required AND flat_payload IS NULL AND flat_payload_sha256 IS NULL)),
    CHECK ((webwork_required AND webwork_payload IS NOT NULL AND webwork_payload_sha256 IS NOT NULL
        AND webwork_replay_payload IS NOT NULL AND webwork_replay_payload_sha256 IS NOT NULL)
        OR (NOT webwork_required AND webwork_payload IS NULL AND webwork_payload_sha256 IS NULL
        AND webwork_replay_payload IS NULL AND webwork_replay_payload_sha256 IS NULL)),
    CHECK ((qti_required AND qti_payload IS NOT NULL AND qti_payload_sha256 IS NOT NULL)
        OR (NOT qti_required AND qti_payload IS NULL AND qti_payload_sha256 IS NULL)),
    CHECK (qti_payload IS NULL OR octet_length(qti_payload) BETWEEN 1 AND 262144),
    CHECK ((flat_payload_sha256 IS NULL OR flat_payload_sha256 ~ '^[0-9a-f]{64}$')
        AND (webwork_payload_sha256 IS NULL OR webwork_payload_sha256 ~ '^[0-9a-f]{64}$')
        AND (webwork_replay_payload_sha256 IS NULL OR webwork_replay_payload_sha256 ~ '^[0-9a-f]{64}$')
        AND (qti_payload_sha256 IS NULL OR qti_payload_sha256 ~ '^[0-9a-f]{64}$'))
);

CREATE TABLE public.prefetch_private_execution (
    tenant_id uuid NOT NULL,
    run_id uuid NOT NULL,
    predecessor_attempt_id uuid NOT NULL,
    assignment_position integer NOT NULL,
    flat_required boolean NOT NULL,
    flat_payload jsonb,
    flat_payload_sha256 character(64),
    webwork_required boolean NOT NULL,
    webwork_payload jsonb,
    webwork_payload_sha256 character(64),
    webwork_replay_payload jsonb,
    webwork_replay_payload_sha256 character(64),
    qti_required boolean NOT NULL,
    qti_payload bytea,
    qti_payload_sha256 character(64),
    PRIMARY KEY (tenant_id, run_id, predecessor_attempt_id, assignment_position),
    FOREIGN KEY (tenant_id, run_id, predecessor_attempt_id, assignment_position)
        REFERENCES public.question_prefetch(tenant_id, run_id, predecessor_attempt_id, assignment_position)
        ON DELETE CASCADE,
    CHECK (qti_payload IS NULL OR octet_length(qti_payload) BETWEEN 1 AND 262144),
    CHECK ((flat_payload_sha256 IS NULL OR flat_payload_sha256 ~ '^[0-9a-f]{64}$')
        AND (webwork_payload_sha256 IS NULL OR webwork_payload_sha256 ~ '^[0-9a-f]{64}$')
        AND (webwork_replay_payload_sha256 IS NULL OR webwork_replay_payload_sha256 ~ '^[0-9a-f]{64}$')
        AND (qti_payload_sha256 IS NULL OR qti_payload_sha256 ~ '^[0-9a-f]{64}$'))
);
ALTER TABLE public.issued_attempt_private_execution ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.issued_attempt_private_execution FORCE ROW LEVEL SECURITY;
ALTER TABLE public.prefetch_private_execution ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.prefetch_private_execution FORCE ROW LEVEL SECURITY;
REVOKE ALL ON public.issued_attempt_private_execution, public.prefetch_private_execution
    FROM PUBLIC, ple_app, ple_student, ple_grader, ple_grading_reader;

CREATE POLICY learner_work_broker_course_tenant ON public.course
    TO ple_learner_work_broker USING (tenant_id = public.ple_current_tenant());
CREATE POLICY learner_work_broker_assignment_tenant ON public.assignment
    TO ple_learner_work_broker USING (tenant_id = public.ple_current_tenant());
CREATE POLICY learner_work_broker_member_tenant ON public.course_member
    TO ple_learner_work_broker USING (tenant_id = public.ple_current_tenant());
CREATE POLICY learner_work_broker_group_tenant ON public.course_group
    TO ple_learner_work_broker USING (tenant_id = public.ple_current_tenant());
CREATE POLICY learner_work_broker_group_member_tenant ON public.course_group_member
    TO ple_learner_work_broker USING (tenant_id = public.ple_current_tenant());
CREATE POLICY learner_work_broker_audience_tenant ON public.assignment_audience_group
    TO ple_learner_work_broker USING (tenant_id = public.ple_current_tenant());
CREATE POLICY learner_work_broker_enrollment_tenant ON public.enrollment
    TO ple_learner_work_broker USING (tenant_id = public.ple_current_tenant());
CREATE POLICY learner_work_broker_run_tenant ON public.assignment_run
    TO ple_learner_work_broker USING (tenant_id = public.ple_current_tenant());
CREATE POLICY learner_work_broker_attempt_tenant ON public.question_attempt
    TO ple_learner_work_broker USING (tenant_id = public.ple_current_tenant());
CREATE POLICY learner_work_broker_summary_tenant ON public.student_assignment_summary
    TO ple_learner_work_broker USING (tenant_id = public.ple_current_tenant());

-- Narrow UPDATE grants exist solely because PostgreSQL requires one to lock
-- selected rows FOR UPDATE.  No capability writes source rows.
GRANT SELECT, UPDATE (course_id) ON public.course TO ple_learner_work_broker;
GRANT SELECT, UPDATE (assignment_id) ON public.assignment TO ple_learner_work_broker;
GRANT SELECT, UPDATE (course_membership_id) ON public.course_member TO ple_learner_work_broker;
GRANT SELECT, UPDATE (course_group_id) ON public.course_group TO ple_learner_work_broker;
GRANT SELECT ON public.course_group_member, public.assignment_audience_group TO ple_learner_work_broker;
GRANT SELECT, UPDATE (enrollment_id) ON public.enrollment TO ple_learner_work_broker;
GRANT SELECT, UPDATE (run_id) ON public.assignment_run TO ple_learner_work_broker;
GRANT SELECT, UPDATE (attempt_id) ON public.question_attempt TO ple_learner_work_broker;
GRANT SELECT, UPDATE (enrollment_id) ON public.student_assignment_summary TO ple_learner_work_broker;
GRANT EXECUTE ON FUNCTION public.ple_current_tenant(),
    public.ple_course_records_accessible(uuid, uuid) TO ple_learner_work_broker;

CREATE FUNCTION public.ple_learner_work_deny_internal()
RETURNS void LANGUAGE plpgsql SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public', pg_temp AS $$
BEGIN
    RAISE EXCEPTION 'learner work is unavailable' USING ERRCODE = '42501';
END
$$;

-- A probe sees only the actor's authority row.  It neither locks nor discovers
-- target learner/run/attempt state, and all denial is concealed by the caller.
CREATE FUNCTION public.ple_learner_work_probe_authority_internal(
    p_tenant uuid, p_course uuid, p_actor uuid, p_authority_kind text
) RETURNS void LANGUAGE plpgsql SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public', pg_temp AS $$
BEGIN
    IF p_authority_kind = 'rule' THEN RETURN; END IF;
    PERFORM 1 FROM public.course_member AS member
     WHERE member.tenant_id = p_tenant AND member.course_id = p_course
       AND member.user_id = p_actor
       AND member.role = CASE WHEN p_authority_kind = 'direct_instructor'
                              THEN 'instructor' ELSE 'student' END
       AND member.status = 'active';
    IF NOT FOUND THEN PERFORM public.ple_learner_work_deny_internal(); END IF;
END
$$;

-- Lock order after the non-locking authority probe: course; assignment
-- advisory lock; assignment; groups; one sorted membership lock/recheck;
-- enrollment; run; attempt; summary.  The witness is answer-free.
CREATE FUNCTION public.ple_learner_work_prepare_internal(
    p_tenant uuid, p_course uuid, p_assignment uuid, p_learner uuid,
    p_actor uuid, p_authority_kind text, p_rule_kind text, p_run uuid, p_attempt uuid
) RETURNS TABLE(
    tenant_id uuid, course_id uuid, assignment_id uuid, authority_kind text, rule_kind text,
    actor_id uuid, authority_membership_id uuid, learner_id uuid, student_membership_id uuid,
    assignment_revision bigint, assignment_lifecycle text, audience_kind text,
    locked_audience_count bigint, locked_audience_group_ids uuid[],
    locked_current_group_count bigint, locked_current_group_ids uuid[],
    existing_enrollment_id uuid, run_id uuid, attempt_id uuid, attempt_status text,
    locked_summary_count bigint, locked_summary_enrollment_ids uuid[]
) LANGUAGE plpgsql SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public', pg_temp AS $$
DECLARE
    authority_membership uuid; target_membership uuid; target_user uuid; target_student uuid;
    enrollment uuid; locked_run uuid; locked_attempt uuid; locked_attempt_status text;
    revision bigint; lifecycle text; assignment_audience_kind text;
    audience_group_ids uuid[] := ARRAY[]::uuid[];
    current_group_ids uuid[] := ARRAY[]::uuid[];
    group_ids_to_lock uuid[] := ARRAY[]::uuid[];
    membership_ids_to_lock uuid[] := ARRAY[]::uuid[];
    summary_enrollment_ids uuid[] := ARRAY[]::uuid[];
BEGIN
    IF p_tenant IS NULL OR p_course IS NULL OR p_assignment IS NULL
       OR p_tenant IS DISTINCT FROM public.ple_current_tenant()
       OR p_authority_kind NOT IN ('student_self_service', 'student_self', 'direct_instructor', 'rule')
       OR (p_authority_kind = 'rule' AND p_rule_kind NOT IN ('imported_grade', 'automated_grader'))
       OR (p_authority_kind <> 'rule' AND (p_rule_kind IS NOT NULL OR p_actor IS NULL))
       OR (p_authority_kind IN ('student_self_service', 'student_self')
           AND (p_learner IS NULL OR p_actor IS DISTINCT FROM p_learner))
       OR (p_authority_kind IN ('direct_instructor', 'rule') AND p_learner IS NULL) THEN
        PERFORM public.ple_learner_work_deny_internal();
    END IF;

    PERFORM public.ple_learner_work_probe_authority_internal(
        p_tenant, p_course, p_actor, p_authority_kind);
    PERFORM 1 FROM public.course AS course_row
     WHERE course_row.tenant_id = p_tenant AND course_row.course_id = p_course FOR UPDATE;
    IF NOT FOUND OR NOT public.ple_course_records_accessible(p_tenant, p_course) THEN
        PERFORM public.ple_learner_work_deny_internal();
    END IF;
    PERFORM pg_advisory_xact_lock(hashtextextended(p_tenant::text || ':' || p_assignment::text, 0));
    SELECT assignment_row.revision, assignment_row.lifecycle, assignment_row.audience_kind
      INTO revision, lifecycle, assignment_audience_kind
      FROM public.assignment AS assignment_row
     WHERE assignment_row.tenant_id = p_tenant AND assignment_row.course_id = p_course
       AND assignment_row.assignment_id = p_assignment FOR UPDATE;
    IF NOT FOUND THEN PERFORM public.ple_learner_work_deny_internal(); END IF;

    -- Only an authorized, locked course/assignment may reveal target bindings.
    IF p_attempt IS NOT NULL THEN
        SELECT run_row.run_id, enrollment_row.enrollment_id, enrollment_row.course_membership_id,
               enrollment_row.user_id, enrollment_row.student_id
          INTO locked_run, enrollment, target_membership, target_user, target_student
          FROM public.question_attempt AS attempt_row
          JOIN public.assignment_run AS run_row
            ON run_row.tenant_id = attempt_row.tenant_id AND run_row.run_id = attempt_row.run_id
          JOIN public.enrollment AS enrollment_row
            ON enrollment_row.tenant_id = run_row.tenant_id
           AND enrollment_row.enrollment_id = run_row.enrollment_id
         WHERE attempt_row.tenant_id = p_tenant AND attempt_row.attempt_id = p_attempt
           AND attempt_row.course_id = p_course AND enrollment_row.course_id = p_course
           AND enrollment_row.assignment_id = p_assignment;
    ELSIF p_run IS NOT NULL THEN
        SELECT run_row.run_id, enrollment_row.enrollment_id, enrollment_row.course_membership_id,
               enrollment_row.user_id, enrollment_row.student_id
          INTO locked_run, enrollment, target_membership, target_user, target_student
          FROM public.assignment_run AS run_row
          JOIN public.enrollment AS enrollment_row
            ON enrollment_row.tenant_id = run_row.tenant_id
           AND enrollment_row.enrollment_id = run_row.enrollment_id
         WHERE run_row.tenant_id = p_tenant AND run_row.run_id = p_run
           AND enrollment_row.course_id = p_course AND enrollment_row.assignment_id = p_assignment;
    ELSE
        SELECT member.course_membership_id, member.user_id, member.student_id
          INTO target_membership, target_user, target_student
          FROM public.course_member AS member
         WHERE member.tenant_id = p_tenant AND member.course_id = p_course
           AND member.user_id = p_learner AND member.role = 'student' AND member.status = 'active';
    END IF;
    IF NOT FOUND OR (p_authority_kind IN ('student_self_service', 'student_self')
                     AND target_user IS DISTINCT FROM p_actor) THEN
        PERFORM public.ple_learner_work_deny_internal();
    END IF;

    SELECT COALESCE(array_agg(DISTINCT audience.course_group_id ORDER BY audience.course_group_id),
                    ARRAY[]::uuid[]) INTO audience_group_ids
      FROM public.assignment_audience_group AS audience
     WHERE audience.tenant_id = p_tenant AND audience.course_id = p_course
       AND audience.assignment_id = p_assignment;
    SELECT COALESCE(array_agg(DISTINCT group_member.course_group_id ORDER BY group_member.course_group_id),
                    ARRAY[]::uuid[]) INTO current_group_ids
      FROM public.course_group_member AS group_member
     WHERE group_member.tenant_id = p_tenant AND group_member.course_id = p_course
       AND group_member.course_membership_id = target_membership;
    SELECT COALESCE(array_agg(DISTINCT group_id ORDER BY group_id), ARRAY[]::uuid[])
      INTO group_ids_to_lock FROM unnest(audience_group_ids || current_group_ids) AS group_id;
    PERFORM 1 FROM public.course_group AS group_row
     WHERE group_row.tenant_id = p_tenant AND group_row.course_id = p_course
       AND group_row.course_group_id = ANY(group_ids_to_lock)
     ORDER BY group_row.course_group_id FOR UPDATE;

    SELECT COALESCE(array_agg(DISTINCT member.course_membership_id ORDER BY member.course_membership_id),
                    ARRAY[]::uuid[]) INTO membership_ids_to_lock
      FROM public.course_member AS member
     WHERE member.tenant_id = p_tenant AND member.course_id = p_course
       AND (member.course_membership_id = target_membership
            OR (p_authority_kind <> 'rule' AND member.user_id = p_actor
                AND member.role = CASE WHEN p_authority_kind = 'direct_instructor'
                                       THEN 'instructor' ELSE 'student' END
                AND member.status = 'active'));
    PERFORM 1 FROM public.course_member AS member
     WHERE member.tenant_id = p_tenant AND member.course_id = p_course
       AND member.course_membership_id = ANY(membership_ids_to_lock)
     ORDER BY member.course_membership_id FOR UPDATE;

    IF p_authority_kind <> 'rule' THEN
        SELECT member.course_membership_id INTO authority_membership
          FROM public.course_member AS member
         WHERE member.tenant_id = p_tenant AND member.course_id = p_course
           AND member.user_id = p_actor
           AND member.role = CASE WHEN p_authority_kind = 'direct_instructor'
                                  THEN 'instructor' ELSE 'student' END
           AND member.status = 'active'
         ORDER BY member.course_membership_id LIMIT 1;
        IF NOT FOUND THEN PERFORM public.ple_learner_work_deny_internal(); END IF;
    END IF;
    PERFORM 1 FROM public.course_member AS member
     WHERE member.tenant_id = p_tenant AND member.course_id = p_course
       AND member.course_membership_id = target_membership
       AND member.user_id = target_user AND member.student_id = target_student
       AND member.role = 'student' AND member.status = 'active';
    IF NOT FOUND THEN PERFORM public.ple_learner_work_deny_internal(); END IF;

    IF enrollment IS NULL THEN
        SELECT enrollment_row.enrollment_id INTO enrollment
          FROM public.enrollment AS enrollment_row
         WHERE enrollment_row.tenant_id = p_tenant AND enrollment_row.course_id = p_course
           AND enrollment_row.assignment_id = p_assignment
           AND enrollment_row.course_membership_id = target_membership
         ORDER BY enrollment_row.enrollment_id FOR UPDATE;
    ELSE
        PERFORM 1 FROM public.enrollment AS enrollment_row
         WHERE enrollment_row.tenant_id = p_tenant AND enrollment_row.enrollment_id = enrollment
           AND enrollment_row.course_id = p_course AND enrollment_row.assignment_id = p_assignment
           AND enrollment_row.course_membership_id = target_membership
           AND enrollment_row.user_id = target_user FOR UPDATE;
        IF NOT FOUND THEN PERFORM public.ple_learner_work_deny_internal(); END IF;
    END IF;
    IF locked_run IS NOT NULL THEN
        PERFORM 1 FROM public.assignment_run AS run_row
         WHERE run_row.tenant_id = p_tenant AND run_row.run_id = locked_run
           AND run_row.enrollment_id = enrollment FOR UPDATE;
        IF NOT FOUND THEN PERFORM public.ple_learner_work_deny_internal(); END IF;
    END IF;
    IF p_attempt IS NOT NULL THEN
        SELECT attempt_row.attempt_status INTO locked_attempt_status
          FROM public.question_attempt AS attempt_row
         WHERE attempt_row.tenant_id = p_tenant AND attempt_row.attempt_id = p_attempt
           AND attempt_row.run_id = locked_run AND attempt_row.course_id = p_course FOR UPDATE;
        IF NOT FOUND THEN PERFORM public.ple_learner_work_deny_internal(); END IF;
        locked_attempt := p_attempt;
    END IF;
    SELECT COALESCE(array_agg(DISTINCT summary.enrollment_id ORDER BY summary.enrollment_id),
                    ARRAY[]::uuid[]) INTO summary_enrollment_ids
      FROM (SELECT summary_row.enrollment_id
              FROM public.student_assignment_summary AS summary_row
             WHERE summary_row.tenant_id = p_tenant AND summary_row.enrollment_id = enrollment
             ORDER BY summary_row.enrollment_id FOR UPDATE) AS summary;

    RETURN QUERY SELECT p_tenant, p_course, p_assignment, p_authority_kind, p_rule_kind, p_actor,
        authority_membership, target_user, target_membership, revision, lifecycle,
        assignment_audience_kind, cardinality(audience_group_ids)::bigint, audience_group_ids,
        cardinality(current_group_ids)::bigint, current_group_ids, enrollment, locked_run,
        locked_attempt, locked_attempt_status, cardinality(summary_enrollment_ids)::bigint,
        summary_enrollment_ids;
END
$$;

CREATE FUNCTION public.ple_prepare_entitlement_materialization(
    p_tenant uuid, p_course uuid, p_assignment uuid, p_learner uuid,
    p_authority_kind text, p_actor uuid
) RETURNS TABLE(
    tenant_id uuid, course_id uuid, assignment_id uuid, authority_kind text, actor_id uuid,
    authority_membership_id uuid, learner_id uuid, student_membership_id uuid,
    assignment_revision bigint, assignment_lifecycle text, audience_kind text,
    locked_audience_count bigint, locked_audience_group_ids uuid[],
    locked_current_group_count bigint, locked_current_group_ids uuid[], existing_enrollment_id uuid
) LANGUAGE plpgsql SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public', pg_temp AS $$
BEGIN
    IF p_tenant IS NULL OR p_course IS NULL OR p_assignment IS NULL OR p_learner IS NULL
       OR p_actor IS NULL OR p_authority_kind NOT IN ('student_self_service', 'direct_instructor') THEN
        PERFORM public.ple_learner_work_deny_internal();
    END IF;
    RETURN QUERY SELECT prepared.tenant_id, prepared.course_id, prepared.assignment_id,
        prepared.authority_kind, prepared.actor_id, prepared.authority_membership_id,
        prepared.learner_id, prepared.student_membership_id, prepared.assignment_revision,
        prepared.assignment_lifecycle, prepared.audience_kind, prepared.locked_audience_count,
        prepared.locked_audience_group_ids, prepared.locked_current_group_count,
        prepared.locked_current_group_ids, prepared.existing_enrollment_id
      FROM public.ple_learner_work_prepare_internal(
        p_tenant, p_course, p_assignment, p_learner, p_actor, p_authority_kind,
        NULL, NULL, NULL) AS prepared;
END
$$;

CREATE FUNCTION public.ple_prepare_student_run_work(
    p_tenant uuid, p_course uuid, p_assignment uuid, p_actor uuid, p_run uuid
) RETURNS TABLE(
    tenant_id uuid, course_id uuid, assignment_id uuid, authority_kind text, actor_id uuid,
    authority_membership_id uuid, learner_id uuid, student_membership_id uuid,
    assignment_revision bigint, assignment_lifecycle text, audience_kind text,
    locked_audience_count bigint, locked_audience_group_ids uuid[],
    locked_current_group_count bigint, locked_current_group_ids uuid[], existing_enrollment_id uuid,
    run_id uuid, locked_summary_count bigint, locked_summary_enrollment_ids uuid[]
) LANGUAGE plpgsql SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public', pg_temp AS $$
BEGIN
    -- A public run wrapper never treats a null routing ID as entitlement work.
    IF p_tenant IS NULL OR p_course IS NULL OR p_assignment IS NULL
       OR p_actor IS NULL OR p_run IS NULL THEN
        PERFORM public.ple_learner_work_deny_internal();
    END IF;
    RETURN QUERY SELECT prepared.tenant_id, prepared.course_id, prepared.assignment_id,
        prepared.authority_kind, prepared.actor_id, prepared.authority_membership_id,
        prepared.learner_id, prepared.student_membership_id, prepared.assignment_revision,
        prepared.assignment_lifecycle, prepared.audience_kind, prepared.locked_audience_count,
        prepared.locked_audience_group_ids, prepared.locked_current_group_count,
        prepared.locked_current_group_ids, prepared.existing_enrollment_id, prepared.run_id,
        prepared.locked_summary_count, prepared.locked_summary_enrollment_ids
      FROM public.ple_learner_work_prepare_internal(
        p_tenant, p_course, p_assignment, p_actor, p_actor, 'student_self',
        NULL, p_run, NULL) AS prepared;
END
$$;

CREATE FUNCTION public.ple_prepare_attempt_work(
    p_tenant uuid, p_course uuid, p_assignment uuid, p_actor uuid,
    p_attempt uuid, p_authority_kind text
) RETURNS TABLE(
    tenant_id uuid, course_id uuid, assignment_id uuid, authority_kind text, actor_id uuid,
    authority_membership_id uuid, learner_id uuid, student_membership_id uuid,
    assignment_revision bigint, assignment_lifecycle text, audience_kind text,
    locked_audience_count bigint, locked_audience_group_ids uuid[],
    locked_current_group_count bigint, locked_current_group_ids uuid[], existing_enrollment_id uuid,
    run_id uuid, attempt_id uuid, attempt_status text, locked_summary_count bigint,
    locked_summary_enrollment_ids uuid[]
) LANGUAGE plpgsql SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public', pg_temp AS $$
BEGIN
    IF p_tenant IS NULL OR p_course IS NULL OR p_assignment IS NULL
       OR p_actor IS NULL OR p_attempt IS NULL
       OR p_authority_kind NOT IN ('student_self', 'direct_instructor') THEN
        PERFORM public.ple_learner_work_deny_internal();
    END IF;
    RETURN QUERY SELECT prepared.tenant_id, prepared.course_id, prepared.assignment_id,
        prepared.authority_kind, prepared.actor_id, prepared.authority_membership_id,
        prepared.learner_id, prepared.student_membership_id, prepared.assignment_revision,
        prepared.assignment_lifecycle, prepared.audience_kind, prepared.locked_audience_count,
        prepared.locked_audience_group_ids, prepared.locked_current_group_count,
        prepared.locked_current_group_ids, prepared.existing_enrollment_id, prepared.run_id,
        prepared.attempt_id, prepared.attempt_status, prepared.locked_summary_count,
        prepared.locked_summary_enrollment_ids
      FROM public.ple_learner_work_prepare_internal(
        p_tenant, p_course, p_assignment, p_actor, p_actor, p_authority_kind,
        NULL, NULL, p_attempt) AS prepared;
END
$$;

CREATE FUNCTION public.ple_prepare_rule_entitlement_materialization(
    p_tenant uuid, p_course uuid, p_assignment uuid, p_learner uuid, p_rule_kind text
) RETURNS TABLE(
    tenant_id uuid, course_id uuid, assignment_id uuid, rule_kind text, learner_id uuid,
    student_membership_id uuid, assignment_revision bigint, assignment_lifecycle text,
    audience_kind text, locked_audience_count bigint, locked_audience_group_ids uuid[],
    locked_current_group_count bigint, locked_current_group_ids uuid[], existing_enrollment_id uuid
) LANGUAGE plpgsql SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public', pg_temp AS $$
BEGIN
    IF p_tenant IS NULL OR p_course IS NULL OR p_assignment IS NULL OR p_learner IS NULL
       OR p_rule_kind NOT IN ('imported_grade', 'automated_grader') THEN
        PERFORM public.ple_learner_work_deny_internal();
    END IF;
    RETURN QUERY SELECT prepared.tenant_id, prepared.course_id, prepared.assignment_id,
        prepared.rule_kind, prepared.learner_id, prepared.student_membership_id,
        prepared.assignment_revision, prepared.assignment_lifecycle, prepared.audience_kind,
        prepared.locked_audience_count, prepared.locked_audience_group_ids,
        prepared.locked_current_group_count, prepared.locked_current_group_ids,
        prepared.existing_enrollment_id
      FROM public.ple_learner_work_prepare_internal(
        p_tenant, p_course, p_assignment, p_learner, NULL, 'rule', p_rule_kind,
        NULL, NULL) AS prepared;
END
$$;

ALTER FUNCTION public.ple_learner_work_deny_internal() OWNER TO ple_learner_work_broker;
ALTER FUNCTION public.ple_learner_work_probe_authority_internal(uuid, uuid, uuid, text)
    OWNER TO ple_learner_work_broker;
ALTER FUNCTION public.ple_learner_work_prepare_internal(uuid, uuid, uuid, uuid, uuid, text, text, uuid, uuid)
    OWNER TO ple_learner_work_broker;
ALTER FUNCTION public.ple_prepare_entitlement_materialization(uuid, uuid, uuid, uuid, text, uuid)
    OWNER TO ple_learner_work_broker;
ALTER FUNCTION public.ple_prepare_student_run_work(uuid, uuid, uuid, uuid, uuid)
    OWNER TO ple_learner_work_broker;
ALTER FUNCTION public.ple_prepare_attempt_work(uuid, uuid, uuid, uuid, uuid, text)
    OWNER TO ple_learner_work_broker;
ALTER FUNCTION public.ple_prepare_rule_entitlement_materialization(uuid, uuid, uuid, uuid, text)
    OWNER TO ple_learner_work_broker;

REVOKE ALL ON FUNCTION public.ple_learner_work_deny_internal(),
    public.ple_learner_work_probe_authority_internal(uuid, uuid, uuid, text),
    public.ple_learner_work_prepare_internal(uuid, uuid, uuid, uuid, uuid, text, text, uuid, uuid)
    FROM PUBLIC, ple_app, ple_grader, ple_grading_reader;
GRANT EXECUTE ON FUNCTION public.ple_learner_work_deny_internal(),
    public.ple_learner_work_probe_authority_internal(uuid, uuid, uuid, text),
    public.ple_learner_work_prepare_internal(uuid, uuid, uuid, uuid, uuid, text, text, uuid, uuid)
    TO ple_learner_work_broker;
REVOKE ALL ON FUNCTION public.ple_prepare_entitlement_materialization(uuid, uuid, uuid, uuid, text, uuid),
    public.ple_prepare_student_run_work(uuid, uuid, uuid, uuid, uuid),
    public.ple_prepare_attempt_work(uuid, uuid, uuid, uuid, uuid, text),
    public.ple_prepare_rule_entitlement_materialization(uuid, uuid, uuid, uuid, text)
    FROM PUBLIC, ple_app, ple_grader, ple_grading_reader;
GRANT EXECUTE ON FUNCTION public.ple_prepare_entitlement_materialization(uuid, uuid, uuid, uuid, text, uuid),
    public.ple_prepare_student_run_work(uuid, uuid, uuid, uuid, uuid),
    public.ple_prepare_attempt_work(uuid, uuid, uuid, uuid, uuid, text) TO ple_app;
GRANT EXECUTE ON FUNCTION public.ple_prepare_rule_entitlement_materialization(uuid, uuid, uuid, uuid, text)
    TO ple_grader;

-- Fresh installations prove the entire role, function, policy, and grant matrix.
DO $$
DECLARE
    function_identity text;
    source_relation text;
BEGIN
    IF EXISTS (SELECT 1 FROM pg_auth_members AS membership
                WHERE membership.roleid = 'ple_learner_work_broker'::regrole
                   OR membership.member = 'ple_learner_work_broker'::regrole) THEN
        RAISE EXCEPTION 'ple_learner_work_broker must not have role memberships';
    END IF;
    IF EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'ple_learner_work_broker'
                AND (rolcanlogin OR rolsuper OR rolcreatedb OR rolcreaterole OR rolinherit
                     OR rolreplication OR rolbypassrls)) THEN
        RAISE EXCEPTION 'unsafe learner-work broker attributes';
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_auth_members AS membership
                    WHERE membership.roleid = 'ple_grader'::regrole
                      AND membership.member = 'ple_grading_reader'::regrole
                      AND NOT membership.admin_option AND NOT membership.inherit_option
                      AND membership.set_option)
       OR EXISTS (SELECT 1 FROM pg_auth_members AS membership
                   WHERE membership.member = 'ple_grading_reader'::regrole
                     AND membership.roleid <> 'ple_grader'::regrole) THEN
        RAISE EXCEPTION 'unsafe grader capability membership';
    END IF;
    FOREACH function_identity IN ARRAY ARRAY[
        'public.ple_learner_work_deny_internal()',
        'public.ple_learner_work_probe_authority_internal(uuid,uuid,uuid,text)',
        'public.ple_learner_work_prepare_internal(uuid,uuid,uuid,uuid,uuid,text,text,uuid,uuid)',
        'public.ple_prepare_entitlement_materialization(uuid,uuid,uuid,uuid,text,uuid)',
        'public.ple_prepare_student_run_work(uuid,uuid,uuid,uuid,uuid)',
        'public.ple_prepare_attempt_work(uuid,uuid,uuid,uuid,uuid,text)',
        'public.ple_prepare_rule_entitlement_materialization(uuid,uuid,uuid,uuid,text)'
    ] LOOP
        IF NOT EXISTS (SELECT 1 FROM pg_proc AS procedure
                        WHERE procedure.oid = function_identity::regprocedure
                          AND procedure.proowner = 'ple_learner_work_broker'::regrole
                          AND procedure.prosecdef
                          AND procedure.proconfig @> ARRAY['search_path=pg_catalog, public, pg_temp'])
           OR EXISTS (SELECT 1
                        FROM pg_proc AS procedure
                        CROSS JOIN LATERAL aclexplode(
                            COALESCE(procedure.proacl, acldefault('f', procedure.proowner))
                        ) AS privilege
                       WHERE procedure.oid = function_identity::regprocedure
                         AND privilege.grantee = 0
                         AND privilege.privilege_type = 'EXECUTE') THEN
            RAISE EXCEPTION 'unsafe learner-work function inventory';
        END IF;
    END LOOP;
    IF has_function_privilege('ple_app', 'public.ple_prepare_rule_entitlement_materialization(uuid,uuid,uuid,uuid,text)', 'EXECUTE')
       OR has_function_privilege('ple_grading_reader', 'public.ple_prepare_rule_entitlement_materialization(uuid,uuid,uuid,uuid,text)', 'EXECUTE')
       OR NOT has_function_privilege('ple_grader', 'public.ple_prepare_rule_entitlement_materialization(uuid,uuid,uuid,uuid,text)', 'EXECUTE')
       OR has_function_privilege('ple_grader', 'public.ple_prepare_entitlement_materialization(uuid,uuid,uuid,uuid,text,uuid)', 'EXECUTE')
       OR has_function_privilege('ple_grader', 'public.ple_prepare_student_run_work(uuid,uuid,uuid,uuid,uuid)', 'EXECUTE')
       OR has_function_privilege('ple_grader', 'public.ple_prepare_attempt_work(uuid,uuid,uuid,uuid,uuid,text)', 'EXECUTE')
       OR has_function_privilege('ple_grading_reader', 'public.ple_prepare_entitlement_materialization(uuid,uuid,uuid,uuid,text,uuid)', 'EXECUTE')
       OR has_function_privilege('ple_grading_reader', 'public.ple_prepare_student_run_work(uuid,uuid,uuid,uuid,uuid)', 'EXECUTE')
       OR has_function_privilege('ple_grading_reader', 'public.ple_prepare_attempt_work(uuid,uuid,uuid,uuid,uuid,text)', 'EXECUTE')
       OR NOT has_function_privilege('ple_app', 'public.ple_prepare_entitlement_materialization(uuid,uuid,uuid,uuid,text,uuid)', 'EXECUTE')
       OR NOT has_function_privilege('ple_app', 'public.ple_prepare_student_run_work(uuid,uuid,uuid,uuid,uuid)', 'EXECUTE')
       OR NOT has_function_privilege('ple_app', 'public.ple_prepare_attempt_work(uuid,uuid,uuid,uuid,uuid,text)', 'EXECUTE')
       OR has_function_privilege('ple_app', 'public.ple_learner_work_prepare_internal(uuid,uuid,uuid,uuid,uuid,text,text,uuid,uuid)', 'EXECUTE')
       OR has_function_privilege('ple_grader', 'public.ple_learner_work_prepare_internal(uuid,uuid,uuid,uuid,uuid,text,text,uuid,uuid)', 'EXECUTE') THEN
        RAISE EXCEPTION 'unsafe learner-work function grants';
    END IF;
    FOREACH source_relation IN ARRAY ARRAY[
        'public.course', 'public.assignment', 'public.course_member', 'public.course_group',
        'public.course_group_member', 'public.assignment_audience_group'
    ] LOOP
        IF has_table_privilege('ple_app', source_relation, 'UPDATE') THEN
            RAISE EXCEPTION 'ple_app has broad learner-work source update on %', source_relation;
        END IF;
    END LOOP;
    IF NOT has_column_privilege('ple_learner_work_broker', 'public.course', 'course_id', 'UPDATE')
       OR NOT has_column_privilege('ple_learner_work_broker', 'public.assignment', 'assignment_id', 'UPDATE')
       OR NOT has_column_privilege('ple_learner_work_broker', 'public.course_member', 'course_membership_id', 'UPDATE')
       OR NOT has_column_privilege('ple_learner_work_broker', 'public.course_group', 'course_group_id', 'UPDATE')
       OR NOT has_column_privilege('ple_learner_work_broker', 'public.enrollment', 'enrollment_id', 'UPDATE')
       OR NOT has_column_privilege('ple_learner_work_broker', 'public.assignment_run', 'run_id', 'UPDATE')
       OR NOT has_column_privilege('ple_learner_work_broker', 'public.question_attempt', 'attempt_id', 'UPDATE')
       OR NOT has_column_privilege('ple_learner_work_broker', 'public.student_assignment_summary', 'enrollment_id', 'UPDATE')
       OR NOT has_function_privilege('ple_learner_work_broker', 'public.ple_course_records_accessible(uuid,uuid)', 'EXECUTE')
       OR (SELECT count(*) FROM pg_policies
            WHERE schemaname = 'public' AND policyname LIKE 'learner_work_broker_%') <> 10 THEN
        RAISE EXCEPTION 'unsafe learner-work broker source inventory';
    END IF;
    IF NOT EXISTS (
        SELECT 1 FROM pg_trigger AS trigger
        JOIN pg_proc AS procedure ON procedure.oid = trigger.tgfoid
        WHERE trigger.tgrelid = 'public.question_attempt'::regclass
          AND trigger.tgname = 'question_attempt_issued_evidence_immutable'
          AND NOT trigger.tgisinternal
          AND procedure.oid = 'public.ple_guard_question_attempt_issued_evidence()'::regprocedure
          AND procedure.proowner = 'ple_learner_work_broker'::regrole
          AND procedure.proconfig @> ARRAY['search_path=pg_catalog, public']
    ) OR EXISTS (
        SELECT 1 FROM pg_proc AS procedure
        CROSS JOIN LATERAL aclexplode(
            COALESCE(procedure.proacl, acldefault('f', procedure.proowner))
        ) AS privilege
        WHERE procedure.oid = 'public.ple_guard_question_attempt_issued_evidence()'::regprocedure
          AND privilege.grantee = 0 AND privilege.privilege_type = 'EXECUTE'
    ) THEN
        RAISE EXCEPTION 'unsafe immutable issued evidence guard';
    END IF;
END
$$;

COMMIT;
