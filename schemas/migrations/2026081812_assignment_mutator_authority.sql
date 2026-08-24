-- WP-PROF-T4: assignment definition mutations are a single durable authority.
-- ASVS 1.2.4, 2.2.1, 2.3.3, 8.2.1, and 8.4.1: parameterized capabilities,
-- closed input shapes, atomic revision transitions, and tenant-bound authority.

DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'ple_assignment_mutator_broker') THEN
        CREATE ROLE ple_assignment_mutator_broker
            NOLOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE NOINHERIT NOREPLICATION NOBYPASSRLS;
    END IF;
END
$$;
ALTER ROLE ple_assignment_mutator_broker
    NOLOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE NOINHERIT NOREPLICATION NOBYPASSRLS;
DO $$
BEGIN
    IF EXISTS (
        SELECT 1 FROM pg_auth_members
         WHERE roleid = 'ple_assignment_mutator_broker'::regrole
            OR member = 'ple_assignment_mutator_broker'::regrole
    ) THEN
        RAISE EXCEPTION 'ple_assignment_mutator_broker must not have role memberships';
    END IF;
END
$$;
REVOKE ALL ON SCHEMA public FROM ple_assignment_mutator_broker;
GRANT USAGE ON SCHEMA public TO ple_assignment_mutator_broker;

-- The application can read these definition inputs but cannot mutate them
-- outside an actor-authorized capability below.  Scoring and gradebook tables
-- are deliberately absent from this list.
REVOKE INSERT, UPDATE, DELETE ON public.assignment,
    public.assignment_item, public.assignment_selection_group,
    public.assignment_selection_candidate, public.assignment_audience_group,
    public.assignment_effective_policy_base, public.assignment_group_schedule_offset,
    public.assignment_group_accommodation, public.assignment_individual_policy_exception
    FROM ple_app;

CREATE POLICY assignment_mutator_assignment_tenant ON public.assignment
    TO ple_assignment_mutator_broker
    USING (tenant_id = public.ple_current_tenant())
    WITH CHECK (tenant_id = public.ple_current_tenant());
CREATE POLICY assignment_mutator_item_tenant ON public.assignment_item
    TO ple_assignment_mutator_broker
    USING (tenant_id = public.ple_current_tenant())
    WITH CHECK (tenant_id = public.ple_current_tenant());
CREATE POLICY assignment_mutator_selection_group_tenant ON public.assignment_selection_group
    TO ple_assignment_mutator_broker
    USING (tenant_id = public.ple_current_tenant())
    WITH CHECK (tenant_id = public.ple_current_tenant());
CREATE POLICY assignment_mutator_selection_candidate_tenant ON public.assignment_selection_candidate
    TO ple_assignment_mutator_broker
    USING (tenant_id = public.ple_current_tenant())
    WITH CHECK (tenant_id = public.ple_current_tenant());
CREATE POLICY assignment_mutator_audience_tenant ON public.assignment_audience_group
    TO ple_assignment_mutator_broker
    USING (tenant_id = public.ple_current_tenant())
    WITH CHECK (tenant_id = public.ple_current_tenant());
CREATE POLICY assignment_mutator_base_policy_tenant ON public.assignment_effective_policy_base
    TO ple_assignment_mutator_broker
    USING (tenant_id = public.ple_current_tenant())
    WITH CHECK (tenant_id = public.ple_current_tenant());
CREATE POLICY assignment_mutator_offset_tenant ON public.assignment_group_schedule_offset
    TO ple_assignment_mutator_broker
    USING (tenant_id = public.ple_current_tenant())
    WITH CHECK (tenant_id = public.ple_current_tenant());
CREATE POLICY assignment_mutator_accommodation_tenant ON public.assignment_group_accommodation
    TO ple_assignment_mutator_broker
    USING (tenant_id = public.ple_current_tenant())
    WITH CHECK (tenant_id = public.ple_current_tenant());
CREATE POLICY assignment_mutator_individual_tenant ON public.assignment_individual_policy_exception
    TO ple_assignment_mutator_broker
    USING (tenant_id = public.ple_current_tenant())
    WITH CHECK (tenant_id = public.ple_current_tenant());
CREATE POLICY assignment_mutator_member_tenant ON public.course_member
    TO ple_assignment_mutator_broker
    USING (tenant_id = public.ple_current_tenant());
-- Teaching-settings validation uses the authoritative course term.  This is
-- read-only authority; `FOR UPDATE` needs the narrow column privilege below
-- to hold the term stable through the assignment mutation.
CREATE POLICY assignment_mutator_course_tenant ON public.course
    TO ple_assignment_mutator_broker
    USING (tenant_id = public.ple_current_tenant());
-- The active-attempt preparation capability may identify and lock only the
-- tenant's current attempt rows.  It cannot read learner payload through any
-- app surface, and it receives no mutation authority beyond PostgreSQL's
-- required lock column below.
CREATE POLICY assignment_mutator_active_attempt_question_tenant ON public.question_attempt
    TO ple_assignment_mutator_broker
    USING (tenant_id = public.ple_current_tenant());
CREATE POLICY assignment_mutator_active_attempt_run_tenant ON public.assignment_run
    TO ple_assignment_mutator_broker
    USING (tenant_id = public.ple_current_tenant());
CREATE POLICY assignment_mutator_active_attempt_enrollment_tenant ON public.enrollment
    TO ple_assignment_mutator_broker
    USING (tenant_id = public.ple_current_tenant());

GRANT SELECT, INSERT, UPDATE, DELETE ON public.assignment,
    public.assignment_item, public.assignment_selection_group,
    public.assignment_selection_candidate, public.assignment_audience_group,
    public.assignment_effective_policy_base, public.assignment_group_schedule_offset,
    public.assignment_group_accommodation, public.assignment_individual_policy_exception
    TO ple_assignment_mutator_broker;
GRANT SELECT ON public.course, public.course_member, public.course_group, public.tenant_learner_identity,
    public.problem_version, public.catalog_tenant_grant TO ple_assignment_mutator_broker;
GRANT SELECT ON public.assignment_run, public.enrollment TO ple_assignment_mutator_broker;
-- PostgreSQL requires UPDATE privilege for SELECT FOR UPDATE even when this
-- capability never changes the membership row.
GRANT UPDATE (course_membership_id) ON public.course_member TO ple_assignment_mutator_broker;
GRANT UPDATE (course_id) ON public.course TO ple_assignment_mutator_broker;
GRANT SELECT, UPDATE (attempt_id) ON public.question_attempt TO ple_assignment_mutator_broker;
GRANT EXECUTE ON FUNCTION public.ple_current_tenant(), public.ple_lock_assignable_problem_version(uuid, uuid)
    TO ple_assignment_mutator_broker;

-- Retire the old actorless capabilities.  They are intentionally not wrapped:
-- retaining a second callable authority surface would reintroduce the bypass.
REVOKE ALL ON FUNCTION public.ple_replace_assignment_fixed_item(uuid, uuid, uuid, bigint, uuid, uuid, uuid) FROM PUBLIC, ple_app;
REVOKE ALL ON FUNCTION public.ple_add_assignment_fixed_item(uuid, uuid, uuid, bigint, uuid, integer, uuid, uuid, numeric, text, text) FROM PUBLIC, ple_app;
REVOKE ALL ON FUNCTION public.ple_remove_assignment_fixed_item(uuid, uuid, uuid, bigint, uuid) FROM PUBLIC, ple_app;
DROP FUNCTION public.ple_replace_assignment_fixed_item(uuid, uuid, uuid, bigint, uuid, uuid, uuid);
DROP FUNCTION public.ple_add_assignment_fixed_item(uuid, uuid, uuid, bigint, uuid, integer, uuid, uuid, numeric, text, text);
DROP FUNCTION public.ple_remove_assignment_fixed_item(uuid, uuid, uuid, bigint, uuid);
ALTER FUNCTION public.ple_invalidate_rehearsals_for_assignment(uuid, uuid, uuid, uuid, bigint, bigint)
    RENAME TO ple_invalidate_rehearsals_for_assignment_legacy_actor;
REVOKE ALL ON FUNCTION public.ple_invalidate_rehearsals_for_assignment_legacy_actor(uuid, uuid, uuid, uuid, bigint, bigint) FROM PUBLIC, ple_app;

CREATE FUNCTION public.ple_assignment_mutator_require_editor(
    p_tenant uuid, p_actor uuid, p_course uuid, p_assignment uuid, p_expected_revision bigint
) RETURNS bigint
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'pg_catalog', 'public', pg_temp
    AS $$
DECLARE actual bigint;
BEGIN
    IF p_tenant IS NULL OR p_actor IS NULL OR p_course IS NULL OR p_assignment IS NULL
       OR p_expected_revision IS NULL OR p_expected_revision <= 0
       OR p_tenant IS DISTINCT FROM public.ple_current_tenant() THEN
        RAISE EXCEPTION 'invalid assignment mutation capability' USING ERRCODE = '22023';
    END IF;
    PERFORM 1 FROM public.course WHERE tenant_id=p_tenant AND course_id=p_course FOR KEY SHARE;
    IF NOT FOUND THEN RAISE EXCEPTION 'course is unavailable' USING ERRCODE='42501'; END IF;
    PERFORM pg_advisory_xact_lock(hashtextextended(p_tenant::text || ':' || p_assignment::text, 0));
    SELECT revision INTO actual FROM public.assignment
     WHERE tenant_id = p_tenant AND assignment_id = p_assignment AND course_id = p_course FOR UPDATE;
    IF NOT FOUND THEN RAISE EXCEPTION 'assignment is unavailable' USING ERRCODE = '42501'; END IF;
    PERFORM 1 FROM public.course_member
     WHERE tenant_id = p_tenant AND course_id = p_course AND user_id = p_actor
       AND role = 'instructor' AND status = 'active'
     ORDER BY course_membership_id FOR UPDATE;
    IF NOT FOUND THEN RAISE EXCEPTION 'active direct instructor authority is required' USING ERRCODE = '42501'; END IF;
    IF actual <> p_expected_revision THEN RAISE EXCEPTION 'assignment revision is stale' USING ERRCODE = '55000'; END IF;
    RETURN actual;
END
$$;

-- This broker-only primitive locks exactly the source-scoped active run rows
-- and returns opaque identifiers, never protected subjects or evidence.  Its
-- transaction-scoped locks remain held while the Store hydrates and verifies.
CREATE FUNCTION public.ple_lock_active_rehearsal_source_internal(
    p_tenant uuid,p_course uuid,p_assignment uuid,p_membership uuid
) RETURNS TABLE(locked_rehearsal_count bigint,locked_rehearsal_run_ids uuid[])
LANGUAGE plpgsql SECURITY DEFINER SET search_path TO 'pg_catalog', 'public', pg_temp AS $$
DECLARE identifiers uuid[];
BEGIN
    IF p_tenant IS NULL OR p_tenant IS DISTINCT FROM public.ple_current_tenant()
       OR ((p_course IS NOT NULL)::integer + (p_assignment IS NOT NULL)::integer
           + (p_membership IS NOT NULL)::integer) <> 1 THEN
        RAISE EXCEPTION 'invalid active rehearsal source lock' USING ERRCODE='22023';
    END IF;
    PERFORM 1 FROM public.rehearsal_run WHERE tenant_id=p_tenant AND lifecycle='active'
      AND ((p_course IS NOT NULL AND course_id=p_course)
        OR (p_assignment IS NOT NULL AND assignment_id=p_assignment)
        OR (p_membership IS NOT NULL AND direct_instructor_membership_id=p_membership))
      ORDER BY rehearsal_run_id FOR UPDATE;
    SELECT COALESCE(array_agg(rehearsal_run_id ORDER BY rehearsal_run_id),ARRAY[]::uuid[])
      INTO identifiers FROM public.rehearsal_run WHERE tenant_id=p_tenant AND lifecycle='active'
       AND ((p_course IS NOT NULL AND course_id=p_course)
         OR (p_assignment IS NOT NULL AND assignment_id=p_assignment)
         OR (p_membership IS NOT NULL AND direct_instructor_membership_id=p_membership));
    RETURN QUERY SELECT cardinality(identifiers)::bigint,identifiers;
END
$$;

-- Replace the unaccepted scalar prepare surface.  This execute-only app
-- capability returns a source revision plus the broker-locked opaque run list.
DROP FUNCTION IF EXISTS public.ple_prepare_assignment_rehearsal_verification(uuid,uuid,uuid,uuid,bigint);
CREATE FUNCTION public.ple_prepare_assignment_rehearsal_verification(
    p_tenant uuid,p_actor uuid,p_course uuid,p_assignment uuid,p_expected_revision bigint
) RETURNS TABLE(assignment_revision bigint,locked_rehearsal_count bigint,locked_rehearsal_run_ids uuid[])
LANGUAGE plpgsql SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public', pg_temp AS $$
DECLARE revision_value bigint; membership_value uuid; count_value bigint; identifiers uuid[];
BEGIN
    revision_value := public.ple_assignment_mutator_require_editor(
        p_tenant,p_actor,p_course,p_assignment,p_expected_revision);
    SELECT course_membership_id INTO membership_value FROM public.course_member
     WHERE tenant_id=p_tenant AND course_id=p_course AND user_id=p_actor
       AND role='instructor' AND status='active' ORDER BY course_membership_id LIMIT 1;
    SELECT helper.locked_rehearsal_count,helper.locked_rehearsal_run_ids INTO count_value,identifiers
      FROM public.ple_lock_active_rehearsal_source_internal(p_tenant,NULL,p_assignment,NULL) helper;
    RETURN QUERY SELECT revision_value,count_value,identifiers;
END
$$;

-- This actor-authorized preparation capability returns only the exact active
-- attempt witness for a just-mutated assignment revision.  The locks survive
-- through the transaction while Rust deterministically re-resolves policy;
-- this function neither recalculates nor mutates learner work.
CREATE FUNCTION public.ple_prepare_assignment_active_attempt_reresolution(
    p_tenant uuid,p_actor uuid,p_course uuid,p_assignment uuid,p_new_revision bigint
) RETURNS TABLE(assignment_lifecycle text,assignment_revision bigint,active_attempt_count bigint,active_attempt_ids uuid[])
LANGUAGE plpgsql SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public', pg_temp AS $$
DECLARE lifecycle_value text; revision_value bigint; identifiers uuid[];
BEGIN
    revision_value := public.ple_assignment_mutator_require_editor(
        p_tenant,p_actor,p_course,p_assignment,p_new_revision);
    SELECT assignment.lifecycle INTO lifecycle_value
      FROM public.assignment
     WHERE assignment.tenant_id=p_tenant AND assignment.course_id=p_course
       AND assignment.assignment_id=p_assignment;
    IF lifecycle_value NOT IN ('draft','published','closed','archived') THEN
        RAISE EXCEPTION 'assignment lifecycle is unavailable' USING ERRCODE='55000';
    END IF;
    PERFORM 1
      FROM public.question_attempt AS attempt
      JOIN public.assignment_run AS run
        ON run.tenant_id=attempt.tenant_id AND run.run_id=attempt.run_id
      JOIN public.enrollment AS enrollment
        ON enrollment.tenant_id=run.tenant_id AND enrollment.enrollment_id=run.enrollment_id
     WHERE attempt.tenant_id=p_tenant AND attempt.course_id=p_course
       AND enrollment.assignment_id=p_assignment AND attempt.attempt_status='in_progress'
     ORDER BY attempt.attempt_id FOR UPDATE OF attempt;
    SELECT COALESCE(array_agg(attempt.attempt_id ORDER BY attempt.attempt_id),ARRAY[]::uuid[])
      INTO identifiers
      FROM public.question_attempt AS attempt
      JOIN public.assignment_run AS run
        ON run.tenant_id=attempt.tenant_id AND run.run_id=attempt.run_id
      JOIN public.enrollment AS enrollment
        ON enrollment.tenant_id=run.tenant_id AND enrollment.enrollment_id=run.enrollment_id
     WHERE attempt.tenant_id=p_tenant AND attempt.course_id=p_course
       AND enrollment.assignment_id=p_assignment AND attempt.attempt_status='in_progress';
    RETURN QUERY SELECT lifecycle_value,revision_value,cardinality(identifiers)::bigint,identifiers;
END
$$;

CREATE FUNCTION public.ple_assignment_mutator_closed_object(p_value jsonb, p_allowed text[], p_limit integer)
RETURNS void LANGUAGE plpgsql SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public', pg_temp AS $$
BEGIN
    IF p_value IS NULL OR jsonb_typeof(p_value) <> 'object'
       OR octet_length(p_value::text) > p_limit
       OR EXISTS (SELECT 1 FROM jsonb_object_keys(p_value) AS key WHERE NOT key = ANY(p_allowed)) THEN
        RAISE EXCEPTION 'assignment mutation JSON is not a closed bounded object' USING ERRCODE = '22023';
    END IF;
END
$$;

-- Convert only canonical JSON millisecond integers.  Keeping this converter
-- beside the capability prevents a browser number from becoming an implicitly
-- rounded timestamp at the database authority boundary.
CREATE FUNCTION public.ple_assignment_mutator_millis(p_value jsonb)
RETURNS timestamptz LANGUAGE plpgsql SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public', pg_temp AS $$
DECLARE millis bigint;
BEGIN
    IF p_value = 'null'::jsonb THEN RETURN NULL; END IF;
    IF jsonb_typeof(p_value) <> 'number' OR (p_value #>> '{}') !~ '^-?[0-9]+$' THEN
        RAISE EXCEPTION 'assignment timestamp must be an exact millisecond integer' USING ERRCODE = '22023';
    END IF;
    millis := (p_value #>> '{}')::bigint;
    RETURN to_timestamp(millis::numeric / 1000);
EXCEPTION WHEN numeric_value_out_of_range OR datetime_field_overflow THEN
    RAISE EXCEPTION 'assignment timestamp is outside range' USING ERRCODE = '22023';
END
$$;

CREATE FUNCTION public.ple_invalidate_rehearsals_for_assignment_internal(
    p_tenant uuid, p_course uuid, p_assignment uuid, p_old_revision bigint, p_new_revision bigint,
    p_locked_rehearsal_count bigint
) RETURNS bigint
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'pg_catalog', 'public', pg_temp
    AS $$
DECLARE changed bigint;
BEGIN
    IF p_tenant IS NULL OR p_course IS NULL OR p_assignment IS NULL OR p_old_revision <= 0
       OR p_new_revision <> p_old_revision + 1
       OR p_locked_rehearsal_count IS NULL OR p_locked_rehearsal_count < 0
       OR p_tenant IS DISTINCT FROM public.ple_current_tenant() THEN
        RAISE EXCEPTION 'invalid internal rehearsal revision invalidation' USING ERRCODE = '22023';
    END IF;
    PERFORM 1 FROM public.assignment WHERE tenant_id = p_tenant AND course_id = p_course
       AND assignment_id = p_assignment AND revision = p_new_revision FOR UPDATE;
    IF NOT FOUND THEN RAISE EXCEPTION 'assignment revision transition is unavailable' USING ERRCODE = '55000'; END IF;
    PERFORM 1 FROM public.rehearsal_run WHERE tenant_id = p_tenant AND assignment_id = p_assignment
       AND assignment_revision = p_old_revision AND lifecycle = 'active' ORDER BY rehearsal_run_id FOR UPDATE;
    INSERT INTO public.rehearsal_submission_claim_event
        (tenant_id, rehearsal_run_id, claim_id, sequence, operation_id, generation, phase)
    SELECT root.tenant_id, root.rehearsal_run_id, root.claim_id, latest.sequence + 1,
           latest.operation_id, latest.generation, 'revokedStaleRevision'
      FROM public.rehearsal_submission_claim_root root
      JOIN public.rehearsal_run run ON run.tenant_id = root.tenant_id
       AND run.rehearsal_run_id = root.rehearsal_run_id
      CROSS JOIN LATERAL (SELECT event.sequence, event.operation_id, event.generation
        FROM public.rehearsal_submission_claim_event event
       WHERE event.tenant_id = root.tenant_id AND event.rehearsal_run_id = root.rehearsal_run_id
         AND event.claim_id = root.claim_id ORDER BY event.sequence DESC LIMIT 1) latest
     WHERE root.tenant_id = p_tenant AND run.assignment_id = p_assignment
       AND run.assignment_revision = p_old_revision AND run.lifecycle = 'active'
       AND EXISTS (SELECT 1 FROM public.rehearsal_submission_claim_event event
        WHERE event.tenant_id = root.tenant_id AND event.rehearsal_run_id = root.rehearsal_run_id
          AND event.claim_id = root.claim_id AND event.sequence = latest.sequence
          AND event.phase IN ('prepared', 'gradingDispatched'));
    UPDATE public.rehearsal_run SET lifecycle = 'discardedStaleRevision',
        terminal_at = public.ple_rehearsal_now(), updated_at = public.ple_rehearsal_now()
     WHERE tenant_id = p_tenant AND assignment_id = p_assignment
       AND assignment_revision = p_old_revision AND lifecycle = 'active';
    GET DIAGNOSTICS changed = ROW_COUNT;
    IF changed <> p_locked_rehearsal_count THEN
        RAISE EXCEPTION 'verified rehearsal count changed during assignment mutation' USING ERRCODE='55000';
    END IF;
    RETURN changed;
END
$$;

CREATE FUNCTION public.ple_apply_verified_assignment_definition_revision(
    p_tenant uuid, p_course uuid, p_assignment uuid, p_expected_revision bigint,
    p_locked_rehearsal_count bigint
) RETURNS TABLE(old_revision bigint, new_revision bigint)
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'pg_catalog', 'public', pg_temp
    AS $$
DECLARE actual bigint;
BEGIN
    IF p_tenant IS NULL OR p_course IS NULL OR p_assignment IS NULL OR p_expected_revision <= 0
       OR p_locked_rehearsal_count IS NULL OR p_locked_rehearsal_count < 0
       OR p_tenant IS DISTINCT FROM public.ple_current_tenant() THEN
        RAISE EXCEPTION 'invalid assignment revision transition' USING ERRCODE = '22023';
    END IF;
    SELECT revision INTO actual FROM public.assignment WHERE tenant_id = p_tenant
       AND course_id = p_course AND assignment_id = p_assignment FOR UPDATE;
    IF NOT FOUND OR actual <> p_expected_revision THEN
        RAISE EXCEPTION 'assignment revision is stale' USING ERRCODE = '55000';
    END IF;
    UPDATE public.assignment SET revision = revision + 1, updated_at = transaction_timestamp()
     WHERE tenant_id = p_tenant AND assignment_id = p_assignment AND revision = actual;
    IF NOT FOUND THEN RAISE EXCEPTION 'assignment revision compare-and-swap failed' USING ERRCODE = '55000'; END IF;
    PERFORM public.ple_invalidate_rehearsals_for_assignment_internal(
        p_tenant, p_course, p_assignment, actual, actual + 1, p_locked_rehearsal_count);
    old_revision := actual; new_revision := actual + 1; RETURN NEXT;
END
$$;

CREATE FUNCTION public.ple_assignment_mutator_finish(
    p_tenant uuid, p_course uuid, p_assignment uuid, p_expected_revision bigint,
    p_locked_rehearsal_count bigint
) RETURNS bigint LANGUAGE sql SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public', pg_temp AS $$
SELECT new_revision FROM public.ple_apply_verified_assignment_definition_revision($1, $2, $3, $4, $5)
$$;

CREATE FUNCTION public.ple_replace_assignment_fixed_item(
    p_tenant uuid, p_actor uuid, p_course uuid, p_assignment uuid, p_expected_revision bigint,
    p_current_item uuid, p_problem uuid, p_version uuid,p_locked_rehearsal_count bigint
) RETURNS bigint LANGUAGE plpgsql SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public', pg_temp AS $$
DECLARE lifecycle text;
BEGIN
    PERFORM public.ple_assignment_mutator_require_editor(p_tenant,p_actor,p_course,p_assignment,p_expected_revision);
    IF p_current_item IS NULL OR p_problem IS NULL OR p_version IS NULL THEN
        RAISE EXCEPTION 'invalid focused assignment replacement' USING ERRCODE = '22023'; END IF;
    PERFORM 1 FROM public.assignment_item WHERE tenant_id=p_tenant AND assignment_id=p_assignment
       AND assignment_item_id=p_current_item FOR UPDATE;
    IF NOT FOUND THEN RAISE EXCEPTION 'assignment item is unavailable' USING ERRCODE='42501'; END IF;
    SELECT public.ple_lock_assignable_problem_version(p_problem,p_version) INTO lifecycle;
    IF lifecycle NOT IN ('published','deprecated') THEN RAISE EXCEPTION 'assignment publication is unavailable' USING ERRCODE='42501'; END IF;
    UPDATE public.assignment_item SET problem_id=p_problem, version_id=p_version, revision=revision+1,
       updated_at=transaction_timestamp() WHERE tenant_id=p_tenant AND assignment_item_id=p_current_item;
    RETURN public.ple_assignment_mutator_finish(p_tenant,p_course,p_assignment,p_expected_revision,p_locked_rehearsal_count);
END
$$;

CREATE FUNCTION public.ple_add_assignment_fixed_item(
    p_tenant uuid, p_actor uuid, p_course uuid, p_assignment uuid, p_expected_revision bigint,
    p_item uuid, p_position integer, p_problem uuid, p_version uuid, p_points numeric,
    p_delivery_state text, p_scoring_mode text,p_locked_rehearsal_count bigint
) RETURNS bigint LANGUAGE plpgsql SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public', pg_temp AS $$
DECLARE lifecycle text; entry record;
BEGIN
    PERFORM public.ple_assignment_mutator_require_editor(p_tenant,p_actor,p_course,p_assignment,p_expected_revision);
    IF p_item IS NULL OR p_position IS NULL OR p_position < 0 OR p_problem IS NULL OR p_version IS NULL
       OR p_points IS NULL OR p_delivery_state NOT IN ('active','retired')
       OR p_scoring_mode NOT IN ('normal','full_credit','extra_credit','excluded') THEN
        RAISE EXCEPTION 'invalid focused assignment addition' USING ERRCODE='22023'; END IF;
    IF EXISTS (SELECT 1 FROM public.assignment_item WHERE tenant_id=p_tenant AND assignment_item_id=p_item)
       OR EXISTS (SELECT 1 FROM public.assignment_run run JOIN public.enrollment enrollment
         ON enrollment.tenant_id=run.tenant_id AND enrollment.enrollment_id=run.enrollment_id
        WHERE enrollment.tenant_id=p_tenant AND enrollment.assignment_id=p_assignment) THEN
        RAISE EXCEPTION 'focused addition conflicts with existing assignment state' USING ERRCODE='55000'; END IF;
    SELECT public.ple_lock_assignable_problem_version(p_problem,p_version) INTO lifecycle;
    IF lifecycle NOT IN ('published','deprecated') THEN RAISE EXCEPTION 'assignment publication is unavailable' USING ERRCODE='42501'; END IF;
    FOR entry IN SELECT entry_kind, entry_id FROM (SELECT 'item'::text entry_kind,assignment_item_id entry_id,position FROM public.assignment_item WHERE tenant_id=p_tenant AND assignment_id=p_assignment AND position>=p_position UNION ALL SELECT 'group',selection_group_id,position FROM public.assignment_selection_group WHERE tenant_id=p_tenant AND assignment_id=p_assignment AND position>=p_position) AS positions ORDER BY position DESC,entry_kind DESC LOOP
        IF entry.entry_kind='item' THEN UPDATE public.assignment_item SET position=position+1,revision=revision+1,updated_at=transaction_timestamp() WHERE tenant_id=p_tenant AND assignment_item_id=entry.entry_id;
        ELSE UPDATE public.assignment_selection_group SET position=position+1,revision=revision+1,updated_at=transaction_timestamp() WHERE tenant_id=p_tenant AND selection_group_id=entry.entry_id; END IF;
    END LOOP;
    INSERT INTO public.assignment_item (tenant_id,assignment_id,assignment_item_id,position,problem_id,version_id,points_possible,delivery_state,scoring_mode)
    VALUES(p_tenant,p_assignment,p_item,p_position,p_problem,p_version,p_points,p_delivery_state,p_scoring_mode);
    RETURN public.ple_assignment_mutator_finish(p_tenant,p_course,p_assignment,p_expected_revision,p_locked_rehearsal_count);
END
$$;

CREATE FUNCTION public.ple_remove_assignment_fixed_item(
    p_tenant uuid,p_actor uuid,p_course uuid,p_assignment uuid,p_expected_revision bigint,p_item uuid,p_locked_rehearsal_count bigint
) RETURNS bigint LANGUAGE plpgsql SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public', pg_temp AS $$
DECLARE removed integer; entry record;
BEGIN
    PERFORM public.ple_assignment_mutator_require_editor(p_tenant,p_actor,p_course,p_assignment,p_expected_revision);
    IF p_item IS NULL THEN RAISE EXCEPTION 'invalid focused assignment removal' USING ERRCODE='22023'; END IF;
    IF EXISTS (SELECT 1 FROM public.assignment_run run JOIN public.enrollment enrollment ON enrollment.tenant_id=run.tenant_id AND enrollment.enrollment_id=run.enrollment_id WHERE enrollment.tenant_id=p_tenant AND enrollment.assignment_id=p_assignment) THEN RAISE EXCEPTION 'fixed-item removal requires no issued runs' USING ERRCODE='55000'; END IF;
    SELECT position INTO removed FROM public.assignment_item WHERE tenant_id=p_tenant AND assignment_id=p_assignment AND assignment_item_id=p_item FOR UPDATE;
    IF NOT FOUND THEN RAISE EXCEPTION 'assignment item is unavailable' USING ERRCODE='42501'; END IF;
    DELETE FROM public.assignment_item WHERE tenant_id=p_tenant AND assignment_id=p_assignment AND assignment_item_id=p_item;
    FOR entry IN SELECT entry_kind,entry_id FROM (SELECT 'item'::text entry_kind,assignment_item_id entry_id,position FROM public.assignment_item WHERE tenant_id=p_tenant AND assignment_id=p_assignment AND position>removed UNION ALL SELECT 'group',selection_group_id,position FROM public.assignment_selection_group WHERE tenant_id=p_tenant AND assignment_id=p_assignment AND position>removed) positions ORDER BY position,entry_kind LOOP
        IF entry.entry_kind='item' THEN UPDATE public.assignment_item SET position=position-1,revision=revision+1,updated_at=transaction_timestamp() WHERE tenant_id=p_tenant AND assignment_item_id=entry.entry_id;
        ELSE UPDATE public.assignment_selection_group SET position=position-1,revision=revision+1,updated_at=transaction_timestamp() WHERE tenant_id=p_tenant AND selection_group_id=entry.entry_id; END IF;
    END LOOP;
    RETURN public.ple_assignment_mutator_finish(p_tenant,p_course,p_assignment,p_expected_revision,p_locked_rehearsal_count);
END
$$;

-- Settings are a complete closed `AssignmentTeachingSettings`, emitted by the
-- Rust command codec. ASVS 1.5.2, 2.2.1, 2.2.3, and 2.3.3: validate each
-- nested value and its combined schedule before a single mutation begins.
CREATE FUNCTION public.ple_put_assignment_teaching_settings(
    p_tenant uuid,p_actor uuid,p_course uuid,p_assignment uuid,p_expected_revision bigint,p_settings jsonb,p_locked_rehearsal_count bigint
) RETURNS bigint LANGUAGE plpgsql SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public', pg_temp AS $$
DECLARE policy jsonb; current_lifecycle text; next_lifecycle text; instructions_value text;
DECLARE available_value timestamptz; due_value timestamptz; closes_value timestamptz;
DECLARE late_value text; deadline_value text; time_limit_value integer; attempt_limit_value integer;
DECLARE term_start date; term_end date; course_zone text;
BEGIN
    PERFORM public.ple_assignment_mutator_require_editor(p_tenant,p_actor,p_course,p_assignment,p_expected_revision);
    PERFORM public.ple_assignment_mutator_closed_object(
        p_settings,ARRAY['lifecycle','instructions','basePolicy'],524288);
    IF NOT (p_settings ?& ARRAY['lifecycle','instructions','basePolicy'])
       OR jsonb_typeof(p_settings->'lifecycle') <> 'string'
       OR jsonb_typeof(p_settings->'instructions') <> 'string' THEN
        RAISE EXCEPTION 'assignment teaching settings are incomplete' USING ERRCODE='22023';
    END IF;
    policy := p_settings->'basePolicy';
    PERFORM public.ple_assignment_mutator_closed_object(
        policy,ARRAY['availableAt','dueAt','closesAt','lateSubmission','deadlineBehavior','timeLimitSeconds','attemptLimit'],262144);
    IF NOT (policy ?& ARRAY['availableAt','dueAt','closesAt','lateSubmission','deadlineBehavior','timeLimitSeconds','attemptLimit']) THEN
        RAISE EXCEPTION 'assignment base policy is incomplete' USING ERRCODE='22023';
    END IF;
    instructions_value := p_settings->>'instructions';
    IF char_length(instructions_value) > 50000 THEN
        RAISE EXCEPTION 'assignment instructions exceed the maximum length' USING ERRCODE='22023';
    END IF;
    next_lifecycle := p_settings->>'lifecycle';
    IF next_lifecycle NOT IN ('draft','published','closed','archived') THEN
        RAISE EXCEPTION 'assignment lifecycle is invalid' USING ERRCODE='22023';
    END IF;
    late_value := policy->>'lateSubmission';
    deadline_value := policy->>'deadlineBehavior';
    IF late_value NOT IN ('accept','markLate','reject') OR deadline_value <> 'autoSubmit' THEN
        RAISE EXCEPTION 'assignment base policy enum is invalid' USING ERRCODE='22023';
    END IF;
    available_value := public.ple_assignment_mutator_millis(policy->'availableAt');
    due_value := public.ple_assignment_mutator_millis(policy->'dueAt');
    closes_value := public.ple_assignment_mutator_millis(policy->'closesAt');
    IF jsonb_typeof(policy->'timeLimitSeconds') = 'null' THEN time_limit_value := NULL;
    ELSIF jsonb_typeof(policy->'timeLimitSeconds') = 'number'
       AND (policy->>'timeLimitSeconds') ~ '^[0-9]+$'
       AND (policy->>'timeLimitSeconds')::numeric BETWEEN 1 AND 2147483647 THEN
        time_limit_value := (policy->>'timeLimitSeconds')::integer;
    ELSE RAISE EXCEPTION 'assignment time limit is invalid' USING ERRCODE='22023'; END IF;
    IF jsonb_typeof(policy->'attemptLimit') = 'null' THEN attempt_limit_value := NULL;
    ELSIF jsonb_typeof(policy->'attemptLimit') = 'number'
       AND (policy->>'attemptLimit') ~ '^[0-9]+$'
       AND (policy->>'attemptLimit')::numeric BETWEEN 1 AND 2147483647 THEN
        attempt_limit_value := (policy->>'attemptLimit')::integer;
    ELSE RAISE EXCEPTION 'assignment attempt limit is invalid' USING ERRCODE='22023'; END IF;
    IF (available_value IS NOT NULL AND due_value IS NOT NULL AND available_value > due_value)
       OR (due_value IS NOT NULL AND closes_value IS NOT NULL AND due_value > closes_value)
       OR (available_value IS NOT NULL AND closes_value IS NOT NULL AND available_value > closes_value) THEN
        RAISE EXCEPTION 'assignment base policy schedule is invalid' USING ERRCODE='22023';
    END IF;
    SELECT lifecycle INTO current_lifecycle FROM public.assignment
     WHERE tenant_id=p_tenant AND course_id=p_course AND assignment_id=p_assignment FOR UPDATE;
    IF NOT FOUND OR NOT ((current_lifecycle='draft' AND next_lifecycle IN ('draft','published','archived'))
       OR (current_lifecycle='published' AND next_lifecycle IN ('published','closed','archived'))
       OR (current_lifecycle='closed' AND next_lifecycle IN ('closed','published','archived'))
       OR (current_lifecycle='archived' AND next_lifecycle='archived')) THEN
        RAISE EXCEPTION 'assignment lifecycle transition is invalid' USING ERRCODE='22023';
    END IF;
    SELECT term_start_date,term_end_date,time_zone INTO term_start,term_end,course_zone
      FROM public.course WHERE tenant_id=p_tenant AND course_id=p_course FOR UPDATE;
    IF NOT FOUND THEN RAISE EXCEPTION 'course is unavailable' USING ERRCODE='42501'; END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_timezone_names WHERE name=course_zone) THEN
        RAISE EXCEPTION 'course time zone is invalid' USING ERRCODE='55000';
    END IF;
    IF (available_value IS NOT NULL AND (available_value AT TIME ZONE course_zone)::date NOT BETWEEN term_start AND term_end)
       OR (due_value IS NOT NULL AND (due_value AT TIME ZONE course_zone)::date NOT BETWEEN term_start AND term_end)
       OR (closes_value IS NOT NULL AND (closes_value AT TIME ZONE course_zone)::date NOT BETWEEN term_start AND term_end) THEN
        RAISE EXCEPTION 'assignment base policy is outside the course term' USING ERRCODE='22023';
    END IF;
    UPDATE public.assignment SET lifecycle=next_lifecycle,instructions=instructions_value,
        updated_at=transaction_timestamp() WHERE tenant_id=p_tenant AND assignment_id=p_assignment;
    INSERT INTO public.assignment_effective_policy_base
        (tenant_id,assignment_id,course_id,available_at,due_at,closes_at,late_submission_policy,deadline_behavior,time_limit_seconds,attempt_limit)
    VALUES (p_tenant,p_assignment,p_course,available_value,due_value,closes_value,
        CASE late_value WHEN 'markLate' THEN 'mark_late' ELSE late_value END,'auto_submit',time_limit_value,attempt_limit_value)
    ON CONFLICT (tenant_id,assignment_id) DO UPDATE SET course_id=EXCLUDED.course_id,
        available_at=EXCLUDED.available_at,due_at=EXCLUDED.due_at,closes_at=EXCLUDED.closes_at,
        late_submission_policy=EXCLUDED.late_submission_policy,deadline_behavior=EXCLUDED.deadline_behavior,
        time_limit_seconds=EXCLUDED.time_limit_seconds,attempt_limit=EXCLUDED.attempt_limit,
        updated_at=transaction_timestamp();
    RETURN public.ple_assignment_mutator_finish(p_tenant,p_course,p_assignment,p_expected_revision,p_locked_rehearsal_count);
END
$$;

CREATE FUNCTION public.ple_replace_assignment_definition(
    p_tenant uuid,p_actor uuid,p_course uuid,p_assignment uuid,p_expected_revision bigint,p_definition jsonb,p_base_policy jsonb,p_locked_rehearsal_count bigint
) RETURNS TABLE(revision bigint,scoring_generation bigint,scoring_status text)
LANGUAGE plpgsql SECURITY DEFINER SET search_path TO 'pg_catalog', 'public', pg_temp AS $$
BEGIN
    PERFORM public.ple_assignment_mutator_require_editor(p_tenant,p_actor,p_course,p_assignment,p_expected_revision);
    PERFORM public.ple_assignment_mutator_closed_object(p_definition,ARRAY['title','instructions','lifecycle'],262144);
    PERFORM public.ple_assignment_mutator_closed_object(p_base_policy,ARRAY['availableAt','dueAt','closesAt','lateSubmissionPolicy','deadlineBehavior','timeLimitSeconds','attemptLimit'],65536);
    UPDATE public.assignment SET title=COALESCE(p_definition->>'title',title),instructions=COALESCE(p_definition->>'instructions',instructions),lifecycle=COALESCE(p_definition->>'lifecycle',lifecycle),updated_at=transaction_timestamp() WHERE tenant_id=p_tenant AND assignment_id=p_assignment;
    UPDATE public.assignment_effective_policy_base SET available_at=CASE WHEN p_base_policy ? 'availableAt' THEN to_timestamp((p_base_policy->>'availableAt')::double precision/1000) ELSE available_at END,due_at=CASE WHEN p_base_policy ? 'dueAt' THEN to_timestamp((p_base_policy->>'dueAt')::double precision/1000) ELSE due_at END,closes_at=CASE WHEN p_base_policy ? 'closesAt' THEN to_timestamp((p_base_policy->>'closesAt')::double precision/1000) ELSE closes_at END,late_submission_policy=COALESCE(p_base_policy->>'lateSubmissionPolicy',late_submission_policy),deadline_behavior=COALESCE(p_base_policy->>'deadlineBehavior',deadline_behavior),time_limit_seconds=CASE WHEN p_base_policy ? 'timeLimitSeconds' THEN (p_base_policy->>'timeLimitSeconds')::integer ELSE time_limit_seconds END,attempt_limit=CASE WHEN p_base_policy ? 'attemptLimit' THEN (p_base_policy->>'attemptLimit')::integer ELSE attempt_limit END,updated_at=transaction_timestamp() WHERE tenant_id=p_tenant AND assignment_id=p_assignment;
    revision:=public.ple_assignment_mutator_finish(p_tenant,p_course,p_assignment,p_expected_revision,p_locked_rehearsal_count);
    SELECT assignment_row.scoring_generation,assignment_row.scoring_status
      INTO scoring_generation,scoring_status
      FROM public.assignment AS assignment_row
     WHERE assignment_row.tenant_id=p_tenant AND assignment_row.assignment_id=p_assignment;
    RETURN NEXT;
END
$$;

CREATE FUNCTION public.ple_put_assignment_group_schedule_offset(p_tenant uuid,p_actor uuid,p_course uuid,p_assignment uuid,p_expected_revision bigint,p_group uuid,p_seconds integer,p_locked_rehearsal_count bigint) RETURNS bigint LANGUAGE plpgsql SECURITY DEFINER SET search_path TO 'pg_catalog', 'public', pg_temp AS $$
BEGIN PERFORM public.ple_assignment_mutator_require_editor(p_tenant,p_actor,p_course,p_assignment,p_expected_revision); IF p_group IS NULL OR p_seconds NOT BETWEEN -31536000 AND 31536000 OR p_seconds=0 THEN RAISE EXCEPTION 'invalid schedule offset' USING ERRCODE='22023'; END IF; INSERT INTO public.assignment_group_schedule_offset(tenant_id,assignment_id,course_id,course_group_id,schedule_offset_seconds) VALUES(p_tenant,p_assignment,p_course,p_group,p_seconds) ON CONFLICT(tenant_id,assignment_id,course_group_id) DO UPDATE SET schedule_offset_seconds=EXCLUDED.schedule_offset_seconds,updated_at=transaction_timestamp(); RETURN public.ple_assignment_mutator_finish(p_tenant,p_course,p_assignment,p_expected_revision,p_locked_rehearsal_count); END $$;
CREATE FUNCTION public.ple_delete_assignment_group_schedule_offset(p_tenant uuid,p_actor uuid,p_course uuid,p_assignment uuid,p_expected_revision bigint,p_group uuid,p_locked_rehearsal_count bigint) RETURNS bigint LANGUAGE plpgsql SECURITY DEFINER SET search_path TO 'pg_catalog', 'public', pg_temp AS $$
BEGIN PERFORM public.ple_assignment_mutator_require_editor(p_tenant,p_actor,p_course,p_assignment,p_expected_revision); DELETE FROM public.assignment_group_schedule_offset WHERE tenant_id=p_tenant AND assignment_id=p_assignment AND course_group_id=p_group; IF NOT FOUND THEN RAISE EXCEPTION 'schedule offset is unavailable' USING ERRCODE='42501'; END IF; RETURN public.ple_assignment_mutator_finish(p_tenant,p_course,p_assignment,p_expected_revision,p_locked_rehearsal_count); END $$;

-- Accommodation payloads use the same closed patch encoding as the Rust
-- command codec.  The table constraints remain the final relational guard.
CREATE FUNCTION public.ple_put_assignment_group_accommodation(p_tenant uuid,p_actor uuid,p_course uuid,p_assignment uuid,p_expected_revision bigint,p_group uuid,p_settings jsonb,p_locked_rehearsal_count bigint) RETURNS bigint LANGUAGE plpgsql SECURITY DEFINER SET search_path TO 'pg_catalog', 'public', pg_temp AS $$
BEGIN PERFORM public.ple_assignment_mutator_require_editor(p_tenant,p_actor,p_course,p_assignment,p_expected_revision); IF p_group IS NULL THEN RAISE EXCEPTION 'invalid accommodation group' USING ERRCODE='22023'; END IF; PERFORM public.ple_assignment_mutator_closed_object(p_settings,ARRAY['overrideKind','availableMode','availableAt','dueMode','dueAt','closesMode','closesAt','timeLimitMode','timeLimitSeconds','attemptLimitMode','attemptLimit'],65536); INSERT INTO public.assignment_group_accommodation(tenant_id,assignment_id,course_id,course_group_id,override_kind,available_mode,available_at,due_mode,due_at,closes_mode,closes_at,time_limit_mode,time_limit_seconds,attempt_limit_mode,attempt_limit) VALUES(p_tenant,p_assignment,p_course,p_group,p_settings->>'overrideKind',p_settings->>'availableMode',CASE WHEN p_settings ? 'availableAt' THEN to_timestamp((p_settings->>'availableAt')::double precision/1000) END,p_settings->>'dueMode',CASE WHEN p_settings ? 'dueAt' THEN to_timestamp((p_settings->>'dueAt')::double precision/1000) END,p_settings->>'closesMode',CASE WHEN p_settings ? 'closesAt' THEN to_timestamp((p_settings->>'closesAt')::double precision/1000) END,p_settings->>'timeLimitMode',CASE WHEN p_settings ? 'timeLimitSeconds' THEN (p_settings->>'timeLimitSeconds')::integer END,p_settings->>'attemptLimitMode',CASE WHEN p_settings ? 'attemptLimit' THEN (p_settings->>'attemptLimit')::integer END) ON CONFLICT(tenant_id,assignment_id,course_group_id) DO UPDATE SET override_kind=EXCLUDED.override_kind,available_mode=EXCLUDED.available_mode,available_at=EXCLUDED.available_at,due_mode=EXCLUDED.due_mode,due_at=EXCLUDED.due_at,closes_mode=EXCLUDED.closes_mode,closes_at=EXCLUDED.closes_at,time_limit_mode=EXCLUDED.time_limit_mode,time_limit_seconds=EXCLUDED.time_limit_seconds,attempt_limit_mode=EXCLUDED.attempt_limit_mode,attempt_limit=EXCLUDED.attempt_limit,updated_at=transaction_timestamp(); RETURN public.ple_assignment_mutator_finish(p_tenant,p_course,p_assignment,p_expected_revision,p_locked_rehearsal_count); END $$;
CREATE FUNCTION public.ple_delete_assignment_group_accommodation(p_tenant uuid,p_actor uuid,p_course uuid,p_assignment uuid,p_expected_revision bigint,p_group uuid,p_locked_rehearsal_count bigint) RETURNS bigint LANGUAGE plpgsql SECURITY DEFINER SET search_path TO 'pg_catalog', 'public', pg_temp AS $$
BEGIN PERFORM public.ple_assignment_mutator_require_editor(p_tenant,p_actor,p_course,p_assignment,p_expected_revision); DELETE FROM public.assignment_group_accommodation WHERE tenant_id=p_tenant AND assignment_id=p_assignment AND course_group_id=p_group; IF NOT FOUND THEN RAISE EXCEPTION 'accommodation is unavailable' USING ERRCODE='42501'; END IF; RETURN public.ple_assignment_mutator_finish(p_tenant,p_course,p_assignment,p_expected_revision,p_locked_rehearsal_count); END $$;

CREATE FUNCTION public.ple_put_assignment_individual_exception(p_tenant uuid,p_actor uuid,p_course uuid,p_assignment uuid,p_expected_revision bigint,p_exception uuid,p_student uuid,p_settings jsonb,p_locked_rehearsal_count bigint) RETURNS bigint LANGUAGE plpgsql SECURITY DEFINER SET search_path TO 'pg_catalog', 'public', pg_temp AS $$
BEGIN PERFORM public.ple_assignment_mutator_require_editor(p_tenant,p_actor,p_course,p_assignment,p_expected_revision); IF p_exception IS NULL OR p_student IS NULL THEN RAISE EXCEPTION 'invalid individual exception' USING ERRCODE='22023'; END IF; PERFORM public.ple_assignment_mutator_closed_object(p_settings,ARRAY['overrideKind','availableMode','availableAt','dueMode','dueAt','closesMode','closesAt','timeLimitMode','timeLimitSeconds','attemptLimitMode','attemptLimit'],65536); INSERT INTO public.assignment_individual_policy_exception(tenant_id,assignment_individual_policy_exception_id,assignment_id,course_id,student_id,override_kind,available_mode,available_at,due_mode,due_at,closes_mode,closes_at,time_limit_mode,time_limit_seconds,attempt_limit_mode,attempt_limit) VALUES(p_tenant,p_exception,p_assignment,p_course,p_student,p_settings->>'overrideKind',p_settings->>'availableMode',CASE WHEN p_settings ? 'availableAt' THEN to_timestamp((p_settings->>'availableAt')::double precision/1000) END,p_settings->>'dueMode',CASE WHEN p_settings ? 'dueAt' THEN to_timestamp((p_settings->>'dueAt')::double precision/1000) END,p_settings->>'closesMode',CASE WHEN p_settings ? 'closesAt' THEN to_timestamp((p_settings->>'closesAt')::double precision/1000) END,p_settings->>'timeLimitMode',CASE WHEN p_settings ? 'timeLimitSeconds' THEN (p_settings->>'timeLimitSeconds')::integer END,p_settings->>'attemptLimitMode',CASE WHEN p_settings ? 'attemptLimit' THEN (p_settings->>'attemptLimit')::integer END) ON CONFLICT(tenant_id,assignment_id,student_id) DO UPDATE SET assignment_individual_policy_exception_id=EXCLUDED.assignment_individual_policy_exception_id,override_kind=EXCLUDED.override_kind,available_mode=EXCLUDED.available_mode,available_at=EXCLUDED.available_at,due_mode=EXCLUDED.due_mode,due_at=EXCLUDED.due_at,closes_mode=EXCLUDED.closes_mode,closes_at=EXCLUDED.closes_at,time_limit_mode=EXCLUDED.time_limit_mode,time_limit_seconds=EXCLUDED.time_limit_seconds,attempt_limit_mode=EXCLUDED.attempt_limit_mode,attempt_limit=EXCLUDED.attempt_limit,updated_at=transaction_timestamp(); RETURN public.ple_assignment_mutator_finish(p_tenant,p_course,p_assignment,p_expected_revision,p_locked_rehearsal_count); END $$;
CREATE FUNCTION public.ple_delete_assignment_individual_exception(p_tenant uuid,p_actor uuid,p_course uuid,p_assignment uuid,p_expected_revision bigint,p_student uuid,p_locked_rehearsal_count bigint) RETURNS bigint LANGUAGE plpgsql SECURITY DEFINER SET search_path TO 'pg_catalog', 'public', pg_temp AS $$
BEGIN PERFORM public.ple_assignment_mutator_require_editor(p_tenant,p_actor,p_course,p_assignment,p_expected_revision); DELETE FROM public.assignment_individual_policy_exception WHERE tenant_id=p_tenant AND assignment_id=p_assignment AND student_id=p_student; IF NOT FOUND THEN RAISE EXCEPTION 'individual exception is unavailable' USING ERRCODE='42501'; END IF; RETURN public.ple_assignment_mutator_finish(p_tenant,p_course,p_assignment,p_expected_revision,p_locked_rehearsal_count); END $$;

ALTER FUNCTION public.ple_assignment_mutator_require_editor(uuid,uuid,uuid,uuid,bigint) OWNER TO ple_assignment_mutator_broker;
ALTER FUNCTION public.ple_lock_active_rehearsal_source_internal(uuid,uuid,uuid,uuid) OWNER TO ple_rehearsal_broker;
ALTER FUNCTION public.ple_prepare_assignment_rehearsal_verification(uuid,uuid,uuid,uuid,bigint) OWNER TO ple_assignment_mutator_broker;
ALTER FUNCTION public.ple_prepare_assignment_active_attempt_reresolution(uuid,uuid,uuid,uuid,bigint) OWNER TO ple_assignment_mutator_broker;
ALTER FUNCTION public.ple_assignment_mutator_closed_object(jsonb,text[],integer) OWNER TO ple_assignment_mutator_broker;
ALTER FUNCTION public.ple_assignment_mutator_millis(jsonb) OWNER TO ple_assignment_mutator_broker;
ALTER FUNCTION public.ple_apply_verified_assignment_definition_revision(uuid,uuid,uuid,bigint,bigint) OWNER TO ple_assignment_mutator_broker;
ALTER FUNCTION public.ple_assignment_mutator_finish(uuid,uuid,uuid,bigint,bigint) OWNER TO ple_assignment_mutator_broker;
ALTER FUNCTION public.ple_replace_assignment_fixed_item(uuid,uuid,uuid,uuid,bigint,uuid,uuid,uuid,bigint) OWNER TO ple_assignment_mutator_broker;
ALTER FUNCTION public.ple_add_assignment_fixed_item(uuid,uuid,uuid,uuid,bigint,uuid,integer,uuid,uuid,numeric,text,text,bigint) OWNER TO ple_assignment_mutator_broker;
ALTER FUNCTION public.ple_remove_assignment_fixed_item(uuid,uuid,uuid,uuid,bigint,uuid,bigint) OWNER TO ple_assignment_mutator_broker;
ALTER FUNCTION public.ple_put_assignment_teaching_settings(uuid,uuid,uuid,uuid,bigint,jsonb,bigint) OWNER TO ple_assignment_mutator_broker;
ALTER FUNCTION public.ple_replace_assignment_definition(uuid,uuid,uuid,uuid,bigint,jsonb,jsonb,bigint) OWNER TO ple_assignment_mutator_broker;
ALTER FUNCTION public.ple_put_assignment_group_schedule_offset(uuid,uuid,uuid,uuid,bigint,uuid,integer,bigint) OWNER TO ple_assignment_mutator_broker;
ALTER FUNCTION public.ple_delete_assignment_group_schedule_offset(uuid,uuid,uuid,uuid,bigint,uuid,bigint) OWNER TO ple_assignment_mutator_broker;
ALTER FUNCTION public.ple_put_assignment_group_accommodation(uuid,uuid,uuid,uuid,bigint,uuid,jsonb,bigint) OWNER TO ple_assignment_mutator_broker;
ALTER FUNCTION public.ple_delete_assignment_group_accommodation(uuid,uuid,uuid,uuid,bigint,uuid,bigint) OWNER TO ple_assignment_mutator_broker;
ALTER FUNCTION public.ple_put_assignment_individual_exception(uuid,uuid,uuid,uuid,bigint,uuid,uuid,jsonb,bigint) OWNER TO ple_assignment_mutator_broker;
ALTER FUNCTION public.ple_delete_assignment_individual_exception(uuid,uuid,uuid,uuid,bigint,uuid,bigint) OWNER TO ple_assignment_mutator_broker;
ALTER FUNCTION public.ple_invalidate_rehearsals_for_assignment_internal(uuid,uuid,uuid,bigint,bigint,bigint) OWNER TO ple_rehearsal_broker;

REVOKE ALL ON FUNCTION public.ple_assignment_mutator_require_editor(uuid,uuid,uuid,uuid,bigint), public.ple_assignment_mutator_closed_object(jsonb,text[],integer), public.ple_assignment_mutator_millis(jsonb), public.ple_apply_verified_assignment_definition_revision(uuid,uuid,uuid,bigint,bigint), public.ple_assignment_mutator_finish(uuid,uuid,uuid,bigint,bigint), public.ple_invalidate_rehearsals_for_assignment_internal(uuid,uuid,uuid,bigint,bigint,bigint) FROM PUBLIC, ple_app;
REVOKE ALL ON FUNCTION public.ple_lock_active_rehearsal_source_internal(uuid,uuid,uuid,uuid) FROM PUBLIC, ple_app;
REVOKE ALL ON FUNCTION public.ple_prepare_assignment_rehearsal_verification(uuid,uuid,uuid,uuid,bigint) FROM PUBLIC;
REVOKE ALL ON FUNCTION public.ple_prepare_assignment_active_attempt_reresolution(uuid,uuid,uuid,uuid,bigint) FROM PUBLIC;
REVOKE ALL ON FUNCTION public.ple_replace_assignment_fixed_item(uuid,uuid,uuid,uuid,bigint,uuid,uuid,uuid,bigint), public.ple_add_assignment_fixed_item(uuid,uuid,uuid,uuid,bigint,uuid,integer,uuid,uuid,numeric,text,text,bigint), public.ple_remove_assignment_fixed_item(uuid,uuid,uuid,uuid,bigint,uuid,bigint), public.ple_put_assignment_teaching_settings(uuid,uuid,uuid,uuid,bigint,jsonb,bigint), public.ple_replace_assignment_definition(uuid,uuid,uuid,uuid,bigint,jsonb,jsonb,bigint), public.ple_put_assignment_group_schedule_offset(uuid,uuid,uuid,uuid,bigint,uuid,integer,bigint), public.ple_delete_assignment_group_schedule_offset(uuid,uuid,uuid,uuid,bigint,uuid,bigint), public.ple_put_assignment_group_accommodation(uuid,uuid,uuid,uuid,bigint,uuid,jsonb,bigint), public.ple_delete_assignment_group_accommodation(uuid,uuid,uuid,uuid,bigint,uuid,bigint), public.ple_put_assignment_individual_exception(uuid,uuid,uuid,uuid,bigint,uuid,uuid,jsonb,bigint), public.ple_delete_assignment_individual_exception(uuid,uuid,uuid,uuid,bigint,uuid,bigint) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION public.ple_invalidate_rehearsals_for_assignment_internal(uuid,uuid,uuid,bigint,bigint,bigint) TO ple_assignment_mutator_broker;
GRANT EXECUTE ON FUNCTION public.ple_lock_active_rehearsal_source_internal(uuid,uuid,uuid,uuid) TO ple_rehearsal_broker,ple_assignment_mutator_broker,ple_retention_broker;
GRANT EXECUTE ON FUNCTION public.ple_prepare_assignment_rehearsal_verification(uuid,uuid,uuid,uuid,bigint), public.ple_prepare_assignment_active_attempt_reresolution(uuid,uuid,uuid,uuid,bigint), public.ple_replace_assignment_fixed_item(uuid,uuid,uuid,uuid,bigint,uuid,uuid,uuid,bigint), public.ple_add_assignment_fixed_item(uuid,uuid,uuid,uuid,bigint,uuid,integer,uuid,uuid,numeric,text,text,bigint), public.ple_remove_assignment_fixed_item(uuid,uuid,uuid,uuid,bigint,uuid,bigint), public.ple_put_assignment_teaching_settings(uuid,uuid,uuid,uuid,bigint,jsonb,bigint), public.ple_replace_assignment_definition(uuid,uuid,uuid,uuid,bigint,jsonb,jsonb,bigint), public.ple_put_assignment_group_schedule_offset(uuid,uuid,uuid,uuid,bigint,uuid,integer,bigint), public.ple_delete_assignment_group_schedule_offset(uuid,uuid,uuid,uuid,bigint,uuid,bigint), public.ple_put_assignment_group_accommodation(uuid,uuid,uuid,uuid,bigint,uuid,jsonb,bigint), public.ple_delete_assignment_group_accommodation(uuid,uuid,uuid,uuid,bigint,uuid,bigint), public.ple_put_assignment_individual_exception(uuid,uuid,uuid,uuid,bigint,uuid,uuid,jsonb,bigint), public.ple_delete_assignment_individual_exception(uuid,uuid,uuid,uuid,bigint,uuid,bigint) TO ple_app;

DO $$
BEGIN
    IF has_table_privilege('ple_app','public.assignment','INSERT,UPDATE,DELETE')
       OR has_table_privilege('ple_app','public.assignment_item','INSERT,UPDATE,DELETE')
       OR has_table_privilege('ple_app','public.assignment_effective_policy_base','INSERT,UPDATE,DELETE')
       OR has_function_privilege('ple_app','public.ple_invalidate_rehearsals_for_assignment_internal(uuid,uuid,uuid,bigint,bigint,bigint)','EXECUTE')
       OR has_function_privilege('ple_app','public.ple_lock_active_rehearsal_source_internal(uuid,uuid,uuid,uuid)','EXECUTE')
       OR to_regprocedure('public.ple_replace_assignment_fixed_item(uuid,uuid,uuid,bigint,uuid,uuid,uuid)') IS NOT NULL
       OR to_regprocedure('public.ple_add_assignment_fixed_item(uuid,uuid,uuid,bigint,uuid,integer,uuid,uuid,numeric,text,text)') IS NOT NULL
       OR to_regprocedure('public.ple_remove_assignment_fixed_item(uuid,uuid,uuid,bigint,uuid)') IS NOT NULL
       OR EXISTS (SELECT 1 FROM pg_proc procedure JOIN pg_namespace namespace ON namespace.oid=procedure.pronamespace
          CROSS JOIN LATERAL aclexplode(COALESCE(procedure.proacl,acldefault('f',procedure.proowner))) privilege
          WHERE namespace.nspname='public' AND procedure.oid='public.ple_prepare_assignment_rehearsal_verification(uuid,uuid,uuid,uuid,bigint)'::regprocedure
            AND privilege.grantee=0 AND privilege.privilege_type='EXECUTE') THEN
        RAISE EXCEPTION 'assignment mutator authority grant inventory is unsafe';
    END IF;
    IF EXISTS (SELECT 1 FROM pg_proc procedure JOIN pg_namespace namespace ON namespace.oid=procedure.pronamespace
       CROSS JOIN LATERAL aclexplode(COALESCE(procedure.proacl,acldefault('f',procedure.proowner))) privilege
       WHERE namespace.nspname='public' AND procedure.oid='public.ple_lock_active_rehearsal_source_internal(uuid,uuid,uuid,uuid)'::regprocedure
         AND privilege.grantee=0 AND privilege.privilege_type='EXECUTE')
       OR EXISTS (SELECT 1 FROM pg_proc procedure JOIN pg_namespace namespace ON namespace.oid=procedure.pronamespace
          CROSS JOIN LATERAL aclexplode(COALESCE(procedure.proacl,acldefault('f',procedure.proowner))) privilege
          WHERE namespace.nspname='public' AND procedure.oid='public.ple_prepare_assignment_active_attempt_reresolution(uuid,uuid,uuid,uuid,bigint)'::regprocedure
            AND privilege.grantee=0 AND privilege.privilege_type='EXECUTE')
       OR EXISTS (SELECT 1 FROM pg_proc procedure WHERE procedure.oid='public.ple_prepare_assignment_rehearsal_verification(uuid,uuid,uuid,uuid,bigint)'::regprocedure AND NOT procedure.proretset)
       OR EXISTS (SELECT 1 FROM pg_proc procedure WHERE procedure.oid='public.ple_prepare_assignment_active_attempt_reresolution(uuid,uuid,uuid,uuid,bigint)'::regprocedure AND NOT procedure.proretset)
       OR NOT has_function_privilege('ple_app','public.ple_prepare_assignment_active_attempt_reresolution(uuid,uuid,uuid,uuid,bigint)','EXECUTE') THEN
        RAISE EXCEPTION 'assignment rehearsal preparation capability inventory is unsafe';
    END IF;
END
$$;
