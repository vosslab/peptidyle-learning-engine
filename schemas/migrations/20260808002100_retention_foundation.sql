-- MOD-RETENTION R2: tenant policy, immutable course-end schedule snapshots,
-- and private future-stage rows. Worker execution and destructive purge land later.

CREATE TABLE institution_retention_policy (
    tenant_id uuid PRIMARY KEY,
    notify_days integer NOT NULL CHECK (notify_days BETWEEN 1 AND 36500),
    archive_days integer NOT NULL CHECK (archive_days BETWEEN 1 AND 36500),
    delete_days integer NOT NULL CHECK (delete_days BETWEEN 1 AND 36500),
    assignment_disposition text NOT NULL DEFAULT 'retain'
        CHECK (assignment_disposition IN ('retain', 'delete')),
    CHECK (notify_days < archive_days AND archive_days < delete_days)
);

CREATE TABLE course_retention (
    tenant_id uuid NOT NULL,
    course_id uuid NOT NULL,
    ended_at timestamptz NOT NULL,
    notify_days integer NOT NULL CHECK (notify_days BETWEEN 1 AND 36500),
    archive_days integer NOT NULL CHECK (archive_days BETWEEN 1 AND 36500),
    delete_days integer NOT NULL CHECK (delete_days BETWEEN 1 AND 36500),
    assignment_disposition text NOT NULL CHECK (assignment_disposition IN ('retain', 'delete')),
    generation bigint NOT NULL CHECK (generation > 0),
    lifecycle text NOT NULL DEFAULT 'active' CHECK (lifecycle IN ('active', 'archived', 'deleted')),
    PRIMARY KEY (tenant_id, course_id),
    FOREIGN KEY (tenant_id, course_id) REFERENCES course(tenant_id, course_id),
    CHECK (notify_days < archive_days AND archive_days < delete_days)
);

CREATE TABLE course_retention_stage (
    tenant_id uuid NOT NULL,
    course_id uuid NOT NULL,
    stage text NOT NULL CHECK (stage IN ('notify', 'archiveStudentRecords', 'deleteStudentRecords')),
    generation bigint NOT NULL CHECK (generation > 0),
    due_at timestamptz NOT NULL,
    state text NOT NULL DEFAULT 'scheduled' CHECK (state IN ('scheduled', 'started', 'completed', 'superseded')),
    PRIMARY KEY (tenant_id, course_id, stage, generation),
    FOREIGN KEY (tenant_id, course_id) REFERENCES course_retention(tenant_id, course_id) ON DELETE CASCADE
);
CREATE INDEX course_retention_stage_due_idx
    ON course_retention_stage (due_at, tenant_id, course_id)
    WHERE state = 'scheduled';

ALTER TABLE institution_retention_policy ENABLE ROW LEVEL SECURITY;
ALTER TABLE institution_retention_policy FORCE ROW LEVEL SECURITY;
ALTER TABLE course_retention ENABLE ROW LEVEL SECURITY;
ALTER TABLE course_retention FORCE ROW LEVEL SECURITY;
ALTER TABLE course_retention_stage ENABLE ROW LEVEL SECURITY;
ALTER TABLE course_retention_stage FORCE ROW LEVEL SECURITY;

CREATE POLICY institution_retention_policy_tenant ON institution_retention_policy
    USING (tenant_id = ple_current_tenant()) WITH CHECK (tenant_id = ple_current_tenant());
CREATE POLICY course_retention_tenant ON course_retention
    USING (tenant_id = ple_current_tenant()) WITH CHECK (tenant_id = ple_current_tenant());
CREATE POLICY course_retention_stage_tenant ON course_retention_stage
    USING (tenant_id = ple_current_tenant()) WITH CHECK (tenant_id = ple_current_tenant());

DO $$ BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'ple_retention_broker') THEN
        CREATE ROLE ple_retention_broker NOLOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE NOINHERIT NOBYPASSRLS;
    END IF;
END $$;
GRANT EXECUTE ON FUNCTION ple_current_tenant() TO ple_retention_broker;
GRANT SELECT ON auth_session, course, course_member TO ple_retention_broker;
GRANT SELECT, INSERT, UPDATE ON institution_retention_policy, course_retention, course_retention_stage
    TO ple_retention_broker;
REVOKE ALL ON institution_retention_policy, course_retention, course_retention_stage
    FROM PUBLIC, ple_app, ple_student, ple_grader, ple_queue_broker;

CREATE FUNCTION ple_retention_authorize(p_session char(64), p_course uuid DEFAULT NULL, p_admin_only boolean DEFAULT false)
RETURNS boolean LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog, public AS $$
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
ALTER FUNCTION ple_retention_authorize(char, uuid, boolean) OWNER TO ple_retention_broker;

CREATE FUNCTION ple_configure_retention_policy(p_session char(64), p_notify integer, p_archive integer, p_delete integer)
RETURNS boolean LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog, public AS $$
BEGIN
    IF NOT public.ple_retention_authorize(p_session, NULL, true) THEN RETURN false; END IF;
    INSERT INTO public.institution_retention_policy (tenant_id, notify_days, archive_days, delete_days, assignment_disposition)
    VALUES (public.ple_current_tenant(), p_notify, p_archive, p_delete, 'retain')
    ON CONFLICT (tenant_id) DO UPDATE SET notify_days=EXCLUDED.notify_days, archive_days=EXCLUDED.archive_days, delete_days=EXCLUDED.delete_days;
    RETURN true;
END $$;
ALTER FUNCTION ple_configure_retention_policy(char, integer, integer, integer) OWNER TO ple_retention_broker;

CREATE FUNCTION ple_end_course_retention(p_session char(64), p_course uuid)
RETURNS boolean LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog, public AS $$
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
ALTER FUNCTION ple_end_course_retention(char, uuid) OWNER TO ple_retention_broker;
CREATE FUNCTION ple_read_course_retention(p_session char(64), p_course uuid)
RETURNS TABLE (ended_at_millis bigint, notify_days integer, archive_days integer, delete_days integer, assignment_disposition text, generation bigint, lifecycle text)
LANGUAGE sql VOLATILE SECURITY DEFINER SET search_path = pg_catalog, public AS $$
    SELECT floor(extract(epoch FROM r.ended_at) * 1000)::bigint, r.notify_days, r.archive_days, r.delete_days, r.assignment_disposition, r.generation, r.lifecycle
    FROM public.course_retention r
    WHERE public.ple_retention_authorize(p_session, p_course, false)
      AND r.tenant_id = public.ple_current_tenant() AND r.course_id = p_course
$$;
ALTER FUNCTION ple_read_course_retention(char, uuid) OWNER TO ple_retention_broker;
REVOKE ALL ON FUNCTION ple_retention_authorize(char, uuid, boolean), ple_configure_retention_policy(char, integer, integer, integer), ple_end_course_retention(char, uuid), ple_read_course_retention(char, uuid) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_configure_retention_policy(char, integer, integer, integer), ple_end_course_retention(char, uuid), ple_read_course_retention(char, uuid) TO ple_app;
