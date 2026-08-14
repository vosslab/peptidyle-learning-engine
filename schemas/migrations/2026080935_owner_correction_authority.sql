-- Owner corrections are exceptional: an original instructor may repair one
-- published question and the replacement is propagated only to future
-- assignment definitions.  Issued runs and their evidence remain immutable.

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_roles WHERE rolname = 'ple_question_correction_broker'
    ) THEN
        CREATE ROLE ple_question_correction_broker NOLOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE
            NOINHERIT NOREPLICATION BYPASSRLS;
    END IF;
END
$$;
ALTER ROLE ple_question_correction_broker NOLOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE
    NOINHERIT NOREPLICATION BYPASSRLS;
DO $$
BEGIN
    IF EXISTS (
        SELECT 1
          FROM pg_auth_members AS membership
         WHERE membership.roleid = 'ple_question_correction_broker'::regrole
            OR membership.member = 'ple_question_correction_broker'::regrole
    ) THEN
        RAISE EXCEPTION 'ple_question_correction_broker must not have role memberships';
    END IF;
END
$$;
REVOKE ALL ON SCHEMA public FROM ple_question_correction_broker;
GRANT USAGE ON SCHEMA public TO ple_question_correction_broker;

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_roles WHERE rolname = 'ple_assignment_reference_lock_broker'
    ) THEN
        CREATE ROLE ple_assignment_reference_lock_broker
            NOLOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE NOINHERIT NOREPLICATION BYPASSRLS;
    END IF;
END
$$;
ALTER ROLE ple_assignment_reference_lock_broker
    NOLOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE NOINHERIT NOREPLICATION BYPASSRLS;
DO $$
BEGIN
    IF EXISTS (
        SELECT 1
          FROM pg_auth_members AS membership
         WHERE membership.roleid = 'ple_assignment_reference_lock_broker'::regrole
            OR membership.member = 'ple_assignment_reference_lock_broker'::regrole
    ) THEN
        RAISE EXCEPTION 'ple_assignment_reference_lock_broker must not have role memberships';
    END IF;
END
$$;
REVOKE ALL ON SCHEMA public FROM ple_assignment_reference_lock_broker;
GRANT USAGE ON SCHEMA public TO ple_assignment_reference_lock_broker;

CREATE FUNCTION public.ple_owner_correction_actor(
    p_problem uuid,
    p_predecessor uuid,
    p_scope text
) RETURNS uuid
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'pg_catalog', 'public'
    AS $$
DECLARE
    actor uuid;
BEGIN
    IF p_problem IS NULL OR p_predecessor IS NULL OR p_scope IS NULL THEN
        RETURN NULL;
    END IF;
    SELECT session_row.user_id INTO actor
      FROM public.auth_session AS session_row
      JOIN public.problem AS problem_row
        ON problem_row.problem_id = p_problem
     WHERE session_row.session_hash = current_setting('ple.session_hash', true)
       AND session_row.tenant_id = public.ple_current_tenant()
       AND session_row.revoked_at IS NULL
       AND session_row.expires_at > transaction_timestamp()
       AND problem_row.owner_tenant_id = public.ple_current_tenant()
       AND problem_row.owner_user_id = session_row.user_id
       AND session_row.roles @> '["instructor"]'::jsonb
       AND EXISTS (
            SELECT 1 FROM public.problem_version AS predecessor
             WHERE predecessor.problem_id = p_problem
               AND predecessor.version_id = p_predecessor
               AND predecessor.lifecycle = 'published'
               AND predecessor.publication_scope = p_scope
       );
    RETURN actor;
END
$$;

ALTER FUNCTION public.ple_owner_correction_actor(uuid, uuid, text)
    OWNER TO ple_question_correction_broker;
REVOKE ALL ON FUNCTION public.ple_owner_correction_actor(uuid, uuid, text) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION public.ple_owner_correction_actor(uuid, uuid, text) TO ple_app;

-- Assignment authors may use public or institution-granted current catalog
-- versions, but direct ple_app locking under RLS lacks the UPDATE privilege
-- PostgreSQL requires for FOR SHARE. This broker-owned function performs the
-- exact visibility check and retains a share lock through the caller's
-- assignment transaction; Rust remains responsible for lifecycle policy.
CREATE FUNCTION public.ple_lock_assignable_problem_version(
    p_problem uuid,
    p_version uuid
) RETURNS text
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'pg_catalog', 'public'
    AS $$
DECLARE lifecycle_value text;
BEGIN
    IF p_problem IS NULL
       OR p_version IS NULL
       OR public.ple_current_tenant() IS NULL THEN
        RETURN NULL;
    END IF;

    SELECT version_row.lifecycle INTO lifecycle_value
      FROM public.problem_version AS version_row
     WHERE version_row.problem_id = p_problem
       AND version_row.version_id = p_version
       AND (
            version_row.publication_scope = 'public'
            OR EXISTS (
                SELECT 1 FROM public.catalog_tenant_grant AS grant_row
                 WHERE grant_row.tenant_id = public.ple_current_tenant()
                   AND grant_row.problem_id = p_problem
                   AND grant_row.version_id = p_version
            )
       )
     FOR SHARE;
    RETURN lifecycle_value;
END
$$;

ALTER FUNCTION public.ple_lock_assignable_problem_version(uuid, uuid)
    OWNER TO ple_assignment_reference_lock_broker;
REVOKE ALL ON FUNCTION public.ple_lock_assignable_problem_version(uuid, uuid) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION public.ple_lock_assignable_problem_version(uuid, uuid) TO ple_app;

DROP POLICY problem_version_app_insert ON public.problem_version;
CREATE POLICY problem_version_app_insert ON public.problem_version
    FOR INSERT TO ple_app
    WITH CHECK (
        public.ple_problem_owned_by_current_tenant(problem_id)
        AND (
            previous_version_id IS NULL
            OR (
                lifecycle = 'published'
                AND lifecycle_reason IS NULL
                AND public.ple_owner_correction_actor(
                    problem_id, previous_version_id, publication_scope
                ) IS NOT NULL
            )
        )
    );

CREATE OR REPLACE FUNCTION public.ple_guard_assignment_content_lock() RETURNS trigger
    LANGUAGE plpgsql
    SET search_path TO 'pg_catalog', 'public'
    AS $$
DECLARE row_tenant uuid := COALESCE(NEW.tenant_id, OLD.tenant_id);
DECLARE row_assignment uuid := COALESCE(NEW.assignment_id, OLD.assignment_id);
DECLARE content_changed boolean;
DECLARE is_owner_correction boolean := false;
BEGIN
    IF TG_OP = 'UPDATE' THEN
        content_changed := (NEW.problem_id, NEW.version_id)
            IS DISTINCT FROM (OLD.problem_id, OLD.version_id);
        is_owner_correction := content_changed
            AND pg_trigger_depth() > 1
            AND current_user = 'ple_question_correction_broker'
            AND NEW.problem_id = OLD.problem_id
            AND EXISTS (
                SELECT 1 FROM public.problem_version AS correction
                 WHERE correction.problem_id = OLD.problem_id
                   AND correction.version_id = NEW.version_id
                   AND correction.previous_version_id = OLD.version_id
                   AND correction.lifecycle = 'published'
            );
    ELSE
        content_changed := true;
    END IF;
    IF content_changed AND NOT is_owner_correction AND EXISTS (
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
    IF TG_OP = 'DELETE' THEN RETURN OLD; END IF;
    RETURN NEW;
END
$$;

CREATE OR REPLACE FUNCTION public.ple_propagate_owner_question_correction() RETURNS trigger
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'pg_catalog', 'public'
    AS $$
DECLARE actor uuid;
DECLARE audit_payload jsonb;
DECLARE source_tenant uuid;
DECLARE target_tenant uuid;
BEGIN
    IF NEW.previous_version_id IS NULL THEN RETURN NEW; END IF;
    SELECT public.ple_owner_correction_actor(
        NEW.problem_id, NEW.previous_version_id, NEW.publication_scope
    ) INTO actor;
    IF actor IS NULL THEN
        RAISE EXCEPTION 'owner correction is not authorized' USING ERRCODE = '42501';
    END IF;
    source_tenant := public.ple_current_tenant();
    IF source_tenant IS NULL THEN
        RAISE EXCEPTION 'owner correction requires a tenant context' USING ERRCODE = '42501';
    END IF;
    PERFORM 1 FROM public.problem_version
     WHERE problem_id = NEW.problem_id AND version_id = NEW.previous_version_id
       AND lifecycle = 'published'
     FOR UPDATE;
    IF NOT FOUND THEN
        RAISE EXCEPTION 'owner correction predecessor is not current' USING ERRCODE = '42501';
    END IF;
    UPDATE public.problem_version
       SET lifecycle = 'archived', lifecycle_reason = 'Superseded by an owner correction'
     WHERE problem_id = NEW.problem_id AND version_id = NEW.previous_version_id
       AND lifecycle = 'published';
    INSERT INTO public.catalog_tenant_grant (tenant_id, problem_id, version_id)
    SELECT tenant_id, NEW.problem_id, NEW.version_id
      FROM public.catalog_tenant_grant
     WHERE problem_id = NEW.problem_id AND version_id = NEW.previous_version_id
    ON CONFLICT DO NOTHING;
    FOR target_tenant IN
        SELECT referenced.tenant_id
          FROM (
              SELECT item.tenant_id
                FROM public.assignment_item AS item
               WHERE item.problem_id = NEW.problem_id
                 AND item.version_id = NEW.previous_version_id
              UNION
              SELECT candidate.tenant_id
                FROM public.assignment_selection_candidate AS candidate
               WHERE candidate.problem_id = NEW.problem_id
                 AND candidate.version_id = NEW.previous_version_id
          ) AS referenced
         ORDER BY referenced.tenant_id
    LOOP
        PERFORM set_config('ple.tenant_id', target_tenant::text, true);
        WITH changed_items AS (
            UPDATE public.assignment_item AS item
               SET version_id = NEW.version_id, revision = item.revision + 1,
                   updated_at = transaction_timestamp()
             WHERE item.tenant_id = target_tenant
               AND item.problem_id = NEW.problem_id
               AND item.version_id = NEW.previous_version_id
            RETURNING item.tenant_id, item.assignment_id
        ), changed_candidates AS (
            UPDATE public.assignment_selection_candidate AS candidate
               SET version_id = NEW.version_id, updated_at = transaction_timestamp()
             WHERE candidate.tenant_id = target_tenant
               AND candidate.problem_id = NEW.problem_id
               AND candidate.version_id = NEW.previous_version_id
            RETURNING candidate.tenant_id, candidate.assignment_id
        ), changed_assignments AS (
            SELECT tenant_id, assignment_id FROM changed_items
            UNION
            SELECT tenant_id, assignment_id FROM changed_candidates
        ), updated_assignments AS (
            UPDATE public.assignment AS assignment
               SET revision = assignment.revision + 1, updated_at = transaction_timestamp()
              FROM changed_assignments AS changed
             WHERE assignment.tenant_id = target_tenant
               AND assignment.tenant_id = changed.tenant_id
               AND assignment.assignment_id = changed.assignment_id
            RETURNING changed.tenant_id, changed.assignment_id
        )
        INSERT INTO public.audit_event (tenant_id, audit_event_id, occurred_at, actor_id,
            action, target_kind, target_id, payload, payload_sha256)
        SELECT changed.tenant_id, gen_random_uuid(), transaction_timestamp(), actor,
            'catalog.ownerCorrectionPropagated', 'assignment', changed.assignment_id,
            jsonb_build_object(
                'predecessorVersionId', NEW.previous_version_id,
                'successorVersionId', NEW.version_id,
                'questionId', (
                    SELECT question_id FROM public.problem WHERE problem_id = NEW.problem_id
                ),
                'assignmentId', changed.assignment_id
            ),
            encode(pg_catalog.sha256(convert_to(jsonb_build_object(
                'predecessorVersionId', NEW.previous_version_id,
                'successorVersionId', NEW.version_id,
                'questionId', (
                    SELECT question_id FROM public.problem WHERE problem_id = NEW.problem_id
                ),
                'assignmentId', changed.assignment_id
            )::text, 'UTF8')), 'hex')
          FROM updated_assignments AS changed;
    END LOOP;
    PERFORM set_config('ple.tenant_id', source_tenant::text, true);
    audit_payload := jsonb_build_object(
        'predecessorVersionId', NEW.previous_version_id,
        'successorVersionId', NEW.version_id,
        'questionId', (
            SELECT question_id FROM public.problem WHERE problem_id = NEW.problem_id
        )
    );
    INSERT INTO public.audit_event (tenant_id, audit_event_id, occurred_at, actor_id,
        action, target_kind, target_id, payload, payload_sha256)
    VALUES (source_tenant, gen_random_uuid(), transaction_timestamp(), actor,
        'catalog.ownerCorrectionPublished', 'problemVersion', NEW.version_id, audit_payload,
        encode(pg_catalog.sha256(convert_to(audit_payload::text, 'UTF8')), 'hex'));
    RETURN NEW;
END
$$;

ALTER FUNCTION public.ple_propagate_owner_question_correction()
    OWNER TO ple_question_correction_broker;
REVOKE ALL ON FUNCTION public.ple_propagate_owner_question_correction() FROM PUBLIC;

GRANT SELECT ON public.auth_session, public.problem, public.problem_version
    TO ple_question_correction_broker;
GRANT SELECT, INSERT ON public.catalog_tenant_grant TO ple_question_correction_broker;
GRANT UPDATE (lifecycle, lifecycle_reason) ON public.problem_version
    TO ple_question_correction_broker;
GRANT SELECT, UPDATE (revision, updated_at) ON public.assignment TO ple_question_correction_broker;
GRANT SELECT, UPDATE (version_id, revision, updated_at) ON public.assignment_item
    TO ple_question_correction_broker;
GRANT SELECT, UPDATE (version_id, updated_at) ON public.assignment_selection_candidate
    TO ple_question_correction_broker;
GRANT SELECT ON public.assignment_run, public.enrollment TO ple_question_correction_broker;
GRANT INSERT ON public.audit_event TO ple_question_correction_broker;
GRANT EXECUTE ON FUNCTION public.ple_current_tenant() TO ple_question_correction_broker;
GRANT SELECT ON public.problem_version, public.catalog_tenant_grant
    TO ple_assignment_reference_lock_broker;
GRANT UPDATE (problem_id) ON public.problem_version TO ple_assignment_reference_lock_broker;
GRANT EXECUTE ON FUNCTION public.ple_current_tenant() TO ple_assignment_reference_lock_broker;
REVOKE SELECT, UPDATE(lifecycle, lifecycle_reason) ON public.problem_version
    FROM ple_catalog_ownership_broker;
REVOKE SELECT, UPDATE(revision, updated_at) ON public.assignment
    FROM ple_catalog_ownership_broker;
REVOKE SELECT, UPDATE(version_id, revision, updated_at) ON public.assignment_item
    FROM ple_catalog_ownership_broker;
REVOKE SELECT, UPDATE(version_id, updated_at) ON public.assignment_selection_candidate
    FROM ple_catalog_ownership_broker;
