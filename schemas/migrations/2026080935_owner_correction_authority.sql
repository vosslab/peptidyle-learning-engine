-- WP-R2 ledger entry: immutable Question ID publications replace the former
-- owner-correction authority. Assignment editing validates only an exact,
-- already-selected publication under a narrow RLS-aware lock broker.

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

-- The function checks the caller tenant and locks exactly the resolved
-- immutable publication. It is an assignability check, not a discovery or
-- latest-version resolver.
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
                SELECT 1
                  FROM public.catalog_tenant_grant AS grant_row
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

GRANT SELECT ON public.problem_version, public.catalog_tenant_grant
    TO ple_assignment_reference_lock_broker;
GRANT UPDATE (problem_id) ON public.problem_version
    TO ple_assignment_reference_lock_broker;
GRANT EXECUTE ON FUNCTION public.ple_current_tenant()
    TO ple_assignment_reference_lock_broker;

-- Focused editor mutations are executed by this broker rather than by a
-- publication event. Each capability locks and compares the assignment
-- revision before changing future-run definition rows. Existing
-- assignment_run_item rows retain their issued opaque publication pair.
CREATE FUNCTION public.ple_replace_assignment_fixed_item(
    p_tenant uuid,
    p_course uuid,
    p_assignment uuid,
    p_expected_revision bigint,
    p_current_item uuid,
    p_problem uuid,
    p_version uuid
) RETURNS void
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'pg_catalog', 'public'
    AS $$
DECLARE
    current_revision bigint;
    assignable_lifecycle text;
BEGIN
    IF p_tenant IS NULL OR p_course IS NULL OR p_assignment IS NULL
       OR p_expected_revision IS NULL OR p_expected_revision <= 0
       OR p_current_item IS NULL
       OR p_problem IS NULL OR p_version IS NULL
       OR p_tenant <> public.ple_current_tenant() THEN
        RAISE EXCEPTION 'invalid focused assignment replacement capability'
            USING ERRCODE = '22023';
    END IF;

    SELECT revision INTO current_revision
      FROM public.assignment
     WHERE tenant_id = p_tenant
       AND course_id = p_course
       AND assignment_id = p_assignment
     FOR UPDATE;
    IF NOT FOUND THEN
        RAISE EXCEPTION 'assignment is unavailable' USING ERRCODE = '42501';
    END IF;
    IF current_revision <> p_expected_revision THEN
        RAISE EXCEPTION 'assignment revision is stale' USING ERRCODE = '55000';
    END IF;

    PERFORM 1 FROM public.assignment_item
     WHERE tenant_id = p_tenant
       AND assignment_id = p_assignment
       AND assignment_item_id = p_current_item
     FOR UPDATE;
    IF NOT FOUND THEN
        RAISE EXCEPTION 'assignment item is unavailable' USING ERRCODE = '42501';
    END IF;
    SELECT public.ple_lock_assignable_problem_version(p_problem, p_version)
      INTO assignable_lifecycle;
    IF assignable_lifecycle IS NULL
       OR assignable_lifecycle NOT IN ('published', 'deprecated') THEN
        RAISE EXCEPTION 'assignment publication is unavailable' USING ERRCODE = '42501';
    END IF;

    PERFORM set_config('ple.assignment_edit_kind', 'replace_fixed_item', true);
    UPDATE public.assignment_item
       SET problem_id = p_problem,
           version_id = p_version,
           revision = revision + 1,
           updated_at = transaction_timestamp()
     WHERE tenant_id = p_tenant
       AND assignment_id = p_assignment
       AND assignment_item_id = p_current_item;
    UPDATE public.assignment
       SET revision = revision + 1,
           updated_at = transaction_timestamp()
     WHERE tenant_id = p_tenant AND assignment_id = p_assignment;
END
$$;

CREATE FUNCTION public.ple_add_assignment_fixed_item(
    p_tenant uuid,
    p_course uuid,
    p_assignment uuid,
    p_expected_revision bigint,
    p_item uuid,
    p_position integer,
    p_problem uuid,
    p_version uuid,
    p_points numeric,
    p_delivery_state text,
    p_scoring_mode text
) RETURNS void
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'pg_catalog', 'public'
    AS $$
DECLARE
    current_revision bigint;
    shifted_entry record;
    assignable_lifecycle text;
BEGIN
    PERFORM set_config('ple.assignment_edit_kind', '', true);
    IF p_tenant IS NULL OR p_course IS NULL OR p_assignment IS NULL
       OR p_expected_revision IS NULL OR p_expected_revision <= 0
       OR p_item IS NULL OR p_position IS NULL OR p_position < 0
       OR p_problem IS NULL OR p_version IS NULL OR p_points IS NULL
       OR p_delivery_state IS NULL OR p_scoring_mode IS NULL
       OR p_tenant <> public.ple_current_tenant() THEN
        RAISE EXCEPTION 'invalid focused assignment addition capability'
            USING ERRCODE = '22023';
    END IF;

    SELECT revision INTO current_revision
      FROM public.assignment
     WHERE tenant_id = p_tenant
       AND course_id = p_course
       AND assignment_id = p_assignment
     FOR UPDATE;
    IF NOT FOUND THEN
        RAISE EXCEPTION 'assignment is unavailable' USING ERRCODE = '42501';
    END IF;
    IF current_revision <> p_expected_revision THEN
        RAISE EXCEPTION 'assignment revision is stale' USING ERRCODE = '55000';
    END IF;
    IF EXISTS (
        SELECT 1
          FROM public.assignment_run AS run
          JOIN public.enrollment AS enrollment
            ON enrollment.tenant_id = run.tenant_id
           AND enrollment.enrollment_id = run.enrollment_id
         WHERE enrollment.tenant_id = p_tenant
           AND enrollment.assignment_id = p_assignment
    ) THEN
        RAISE EXCEPTION 'fixed-item addition requires no issued runs'
            USING ERRCODE = '55000';
    END IF;
    IF EXISTS (
        SELECT 1 FROM public.assignment_item
         WHERE tenant_id = p_tenant AND assignment_item_id = p_item
    ) THEN
        RAISE EXCEPTION 'new assignment item identity is already in use'
            USING ERRCODE = '23505';
    END IF;

    SELECT public.ple_lock_assignable_problem_version(p_problem, p_version)
      INTO assignable_lifecycle;
    IF assignable_lifecycle IS NULL
       OR assignable_lifecycle NOT IN ('published', 'deprecated') THEN
        RAISE EXCEPTION 'assignment publication is unavailable' USING ERRCODE = '42501';
    END IF;

    FOR shifted_entry IN
        SELECT entry_kind, entry_id
          FROM (
              SELECT 'item'::text AS entry_kind, assignment_item_id AS entry_id, position
                FROM public.assignment_item
               WHERE tenant_id = p_tenant
                 AND assignment_id = p_assignment
                 AND position >= p_position
              UNION ALL
              SELECT 'group'::text AS entry_kind, selection_group_id AS entry_id, position
                FROM public.assignment_selection_group
               WHERE tenant_id = p_tenant
                 AND assignment_id = p_assignment
                 AND position >= p_position
          ) AS occupied_position
         ORDER BY position DESC, entry_kind DESC
    LOOP
        IF shifted_entry.entry_kind = 'item' THEN
            UPDATE public.assignment_item
               SET position = position + 1,
                   revision = revision + 1,
                   updated_at = transaction_timestamp()
             WHERE tenant_id = p_tenant
               AND assignment_item_id = shifted_entry.entry_id;
        ELSE
            UPDATE public.assignment_selection_group
               SET position = position + 1,
                   revision = revision + 1,
                   updated_at = transaction_timestamp()
             WHERE tenant_id = p_tenant
               AND selection_group_id = shifted_entry.entry_id;
        END IF;
    END LOOP;
    INSERT INTO public.assignment_item (
        tenant_id, assignment_id, assignment_item_id, position, problem_id,
        version_id, points_possible, delivery_state, scoring_mode
    ) VALUES (
        p_tenant, p_assignment, p_item, p_position, p_problem, p_version,
        p_points, p_delivery_state, p_scoring_mode
    );
    UPDATE public.assignment
       SET revision = revision + 1,
           updated_at = transaction_timestamp()
     WHERE tenant_id = p_tenant AND assignment_id = p_assignment;
END
$$;

CREATE FUNCTION public.ple_remove_assignment_fixed_item(
    p_tenant uuid,
    p_course uuid,
    p_assignment uuid,
    p_expected_revision bigint,
    p_item uuid
) RETURNS void
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'pg_catalog', 'public'
    AS $$
DECLARE
    current_revision bigint;
    removed_position integer;
    shifted_entry record;
BEGIN
    PERFORM set_config('ple.assignment_edit_kind', '', true);
    IF p_tenant IS NULL OR p_course IS NULL OR p_assignment IS NULL
       OR p_expected_revision IS NULL OR p_expected_revision <= 0
       OR p_item IS NULL OR p_tenant <> public.ple_current_tenant() THEN
        RAISE EXCEPTION 'invalid focused assignment removal capability'
            USING ERRCODE = '22023';
    END IF;

    SELECT revision INTO current_revision
      FROM public.assignment
     WHERE tenant_id = p_tenant
       AND course_id = p_course
       AND assignment_id = p_assignment
     FOR UPDATE;
    IF NOT FOUND THEN
        RAISE EXCEPTION 'assignment is unavailable' USING ERRCODE = '42501';
    END IF;
    IF current_revision <> p_expected_revision THEN
        RAISE EXCEPTION 'assignment revision is stale' USING ERRCODE = '55000';
    END IF;
    IF EXISTS (
        SELECT 1
          FROM public.assignment_run AS run
          JOIN public.enrollment AS enrollment
            ON enrollment.tenant_id = run.tenant_id
           AND enrollment.enrollment_id = run.enrollment_id
         WHERE enrollment.tenant_id = p_tenant
           AND enrollment.assignment_id = p_assignment
    ) THEN
        RAISE EXCEPTION 'fixed-item removal requires no issued runs'
            USING ERRCODE = '55000';
    END IF;

    SELECT position INTO removed_position
      FROM public.assignment_item
     WHERE tenant_id = p_tenant
       AND assignment_id = p_assignment
       AND assignment_item_id = p_item
     FOR UPDATE;
    IF NOT FOUND THEN
        RAISE EXCEPTION 'assignment item is unavailable' USING ERRCODE = '42501';
    END IF;

    DELETE FROM public.assignment_item
     WHERE tenant_id = p_tenant
       AND assignment_id = p_assignment
       AND assignment_item_id = p_item;
    FOR shifted_entry IN
        SELECT entry_kind, entry_id
          FROM (
              SELECT 'item'::text AS entry_kind, assignment_item_id AS entry_id, position
                FROM public.assignment_item
               WHERE tenant_id = p_tenant
                 AND assignment_id = p_assignment
                 AND position > removed_position
              UNION ALL
              SELECT 'group'::text AS entry_kind, selection_group_id AS entry_id, position
                FROM public.assignment_selection_group
               WHERE tenant_id = p_tenant
                 AND assignment_id = p_assignment
                 AND position > removed_position
          ) AS occupied_position
         ORDER BY position, entry_kind
    LOOP
        IF shifted_entry.entry_kind = 'item' THEN
            UPDATE public.assignment_item
               SET position = position - 1,
                   revision = revision + 1,
                   updated_at = transaction_timestamp()
             WHERE tenant_id = p_tenant
               AND assignment_item_id = shifted_entry.entry_id;
        ELSE
            UPDATE public.assignment_selection_group
               SET position = position - 1,
                   revision = revision + 1,
                   updated_at = transaction_timestamp()
             WHERE tenant_id = p_tenant
               AND selection_group_id = shifted_entry.entry_id;
        END IF;
    END LOOP;
    UPDATE public.assignment
       SET revision = revision + 1,
           updated_at = transaction_timestamp()
     WHERE tenant_id = p_tenant AND assignment_id = p_assignment;
END
$$;

ALTER FUNCTION public.ple_replace_assignment_fixed_item(uuid, uuid, uuid, bigint, uuid, uuid, uuid)
    OWNER TO ple_assignment_reference_lock_broker;
ALTER FUNCTION public.ple_add_assignment_fixed_item(uuid, uuid, uuid, bigint, uuid, integer, uuid, uuid, numeric, text, text)
    OWNER TO ple_assignment_reference_lock_broker;
ALTER FUNCTION public.ple_remove_assignment_fixed_item(uuid, uuid, uuid, bigint, uuid)
    OWNER TO ple_assignment_reference_lock_broker;
REVOKE ALL ON FUNCTION public.ple_replace_assignment_fixed_item(uuid, uuid, uuid, bigint, uuid, uuid, uuid) FROM PUBLIC;
REVOKE ALL ON FUNCTION public.ple_add_assignment_fixed_item(uuid, uuid, uuid, bigint, uuid, integer, uuid, uuid, numeric, text, text) FROM PUBLIC;
REVOKE ALL ON FUNCTION public.ple_remove_assignment_fixed_item(uuid, uuid, uuid, bigint, uuid) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION public.ple_replace_assignment_fixed_item(uuid, uuid, uuid, bigint, uuid, uuid, uuid) TO ple_app;
GRANT EXECUTE ON FUNCTION public.ple_add_assignment_fixed_item(uuid, uuid, uuid, bigint, uuid, integer, uuid, uuid, numeric, text, text) TO ple_app;
GRANT EXECUTE ON FUNCTION public.ple_remove_assignment_fixed_item(uuid, uuid, uuid, bigint, uuid) TO ple_app;

GRANT SELECT, UPDATE (revision, updated_at) ON public.assignment
    TO ple_assignment_reference_lock_broker;
GRANT SELECT, INSERT, DELETE, UPDATE (problem_id, version_id, position, revision, updated_at)
    ON public.assignment_item TO ple_assignment_reference_lock_broker;
GRANT SELECT, UPDATE (position, revision, updated_at)
    ON public.assignment_selection_group TO ple_assignment_reference_lock_broker;
GRANT SELECT ON public.assignment_run, public.enrollment
    TO ple_assignment_reference_lock_broker;
