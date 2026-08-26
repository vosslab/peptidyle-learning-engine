-- WP-PROF-B2: canonical ordinary-course topology foundation.
--
-- Every teaching course has one ordered module topology.  Immutable
-- whole-course-adoption rows remain provenance/evidence; they never replace
-- these ordinary-course rows as a lifecycle source.

BEGIN;

-- ASVS 2.2.1: database constraints are the trusted positive validation
-- boundary for module titles and zero-based topology positions.
CREATE TABLE public.teaching_course_module (
    tenant_id uuid NOT NULL,
    course_id uuid NOT NULL,
    course_module_id uuid NOT NULL DEFAULT gen_random_uuid(),
    position integer NOT NULL,
    title text NOT NULL,
    is_default boolean NOT NULL DEFAULT false,
    PRIMARY KEY (tenant_id, course_id, course_module_id),
    UNIQUE (tenant_id, course_id, position),
    FOREIGN KEY (tenant_id, course_id)
        REFERENCES public.course (tenant_id, course_id) ON DELETE CASCADE,
    CHECK (position >= 0),
    CHECK (char_length(title) BETWEEN 1 AND 200 AND title = btrim(title))
);

CREATE UNIQUE INDEX teaching_course_module_one_default
    ON public.teaching_course_module (tenant_id, course_id) WHERE is_default;

CREATE TABLE public.teaching_course_assignment_position (
    tenant_id uuid NOT NULL,
    course_id uuid NOT NULL,
    assignment_id uuid NOT NULL,
    course_module_id uuid NOT NULL,
    position integer NOT NULL,
    PRIMARY KEY (tenant_id, course_id, assignment_id),
    UNIQUE (tenant_id, course_id, course_module_id, position),
    FOREIGN KEY (tenant_id, course_id, assignment_id)
        REFERENCES public.assignment (tenant_id, course_id, assignment_id)
        ON DELETE CASCADE,
    FOREIGN KEY (tenant_id, course_id, course_module_id)
        REFERENCES public.teaching_course_module
        (tenant_id, course_id, course_module_id) ON DELETE RESTRICT,
    CHECK (position >= 0)
);

COMMENT ON TABLE public.teaching_course_module IS
    'Canonical module/order topology for every ordinary teaching course.';
COMMENT ON COLUMN public.teaching_course_module.is_default IS
    'Operational landing module for ordinary assignment creation, not reusable semantic content.';
COMMENT ON TABLE public.teaching_course_assignment_position IS
    'Canonical assignment membership and zero-based order within an ordinary course module.';

-- Existing courses predate explicit module rows.  The composite module key
-- makes this stable identity collision-safe per course; it derives only from
-- immutable tenant/course identities, never from title or inferred order.
INSERT INTO public.teaching_course_module (
    tenant_id, course_id, course_module_id, position, title, is_default
)
SELECT course_row.tenant_id,
       course_row.course_id,
       substr(
           encode(
               digest(
                   convert_to(
                       'WP-PROF-B2:teaching-course-module:'
                       || course_row.tenant_id::text || ':' || course_row.course_id::text,
                       'UTF8'
                   ),
                   'sha256'
               ),
               'hex'
           ),
           1,
           32
       )::uuid,
       0,
       'Assignments',
       true
  FROM public.course AS course_row;

-- Public references are the only grounded pre-topology assignment order.
INSERT INTO public.teaching_course_assignment_position (
    tenant_id, course_id, assignment_id, course_module_id, position
)
SELECT assignment_row.tenant_id,
       assignment_row.course_id,
       assignment_row.assignment_id,
       module_row.course_module_id,
       (row_number() OVER (
           PARTITION BY assignment_row.tenant_id, assignment_row.course_id
           ORDER BY assignment_row.public_id
       ) - 1)::integer
  FROM public.assignment AS assignment_row
  JOIN public.teaching_course_module AS module_row
    ON module_row.tenant_id = assignment_row.tenant_id
   AND module_row.course_id = assignment_row.course_id
   AND module_row.is_default;

-- Fail closed before enabling writers: all pre-existing ordinary teaching
-- records must have the canonical topology and existing schedule witness.
DO $$
BEGIN
    IF EXISTS (
        SELECT 1
          FROM public.course AS course_row
          LEFT JOIN public.teaching_course_module AS module_row
            ON module_row.tenant_id = course_row.tenant_id
           AND module_row.course_id = course_row.course_id
           AND module_row.is_default
         GROUP BY course_row.tenant_id, course_row.course_id
        HAVING count(module_row.course_module_id) <> 1
    ) OR EXISTS (
        SELECT 1
          FROM public.assignment AS assignment_row
          LEFT JOIN public.teaching_course_assignment_position AS position_row
            ON position_row.tenant_id = assignment_row.tenant_id
           AND position_row.course_id = assignment_row.course_id
           AND position_row.assignment_id = assignment_row.assignment_id
         GROUP BY assignment_row.tenant_id, assignment_row.course_id,
                  assignment_row.assignment_id
        HAVING count(position_row.assignment_id) <> 1
    ) OR EXISTS (
        SELECT 1
          FROM public.teaching_course_assignment_position AS position_row
          LEFT JOIN public.teaching_course_module AS module_row
            ON module_row.tenant_id = position_row.tenant_id
           AND module_row.course_id = position_row.course_id
           AND module_row.course_module_id = position_row.course_module_id
         WHERE module_row.course_module_id IS NULL
            OR position_row.position < 0
    ) OR EXISTS (
        SELECT 1
          FROM public.course AS course_row
          LEFT JOIN public.course_schedule_revision AS schedule_row
            ON schedule_row.tenant_id = course_row.tenant_id
           AND schedule_row.course_id = course_row.course_id
         WHERE schedule_row.course_id IS NULL
    ) THEN
        RAISE EXCEPTION 'ordinary teaching-course topology backfill is incomplete'
            USING ERRCODE = '23514';
    END IF;
END $$;

-- ASVS 8.2.1, 8.2.2, 8.4.1: only the narrow broker may act on
-- tenant-filtered topology rows; FORCE RLS also constrains the broker owner.
ALTER TABLE public.teaching_course_module ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.teaching_course_module FORCE ROW LEVEL SECURITY;
ALTER TABLE public.teaching_course_assignment_position ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.teaching_course_assignment_position FORCE ROW LEVEL SECURITY;

CREATE POLICY curriculum_adoption_teaching_module_tenant
    ON public.teaching_course_module TO ple_curriculum_adoption_broker
    USING (tenant_id = public.ple_current_tenant())
    WITH CHECK (tenant_id = public.ple_current_tenant());
CREATE POLICY curriculum_adoption_teaching_assignment_position_tenant
    ON public.teaching_course_assignment_position TO ple_curriculum_adoption_broker
    USING (tenant_id = public.ple_current_tenant())
    WITH CHECK (tenant_id = public.ple_current_tenant());
CREATE POLICY curriculum_adoption_issued_work_run_tenant ON public.assignment_run
    FOR SELECT TO ple_curriculum_adoption_broker
    USING (tenant_id = public.ple_current_tenant());
CREATE POLICY curriculum_adoption_issued_work_enrollment_tenant ON public.enrollment
    FOR SELECT TO ple_curriculum_adoption_broker
    USING (tenant_id = public.ple_current_tenant());

REVOKE ALL ON public.teaching_course_module,
    public.teaching_course_assignment_position
    FROM PUBLIC, ple_app, ple_auth, ple_student, ple_grader, ple_grading_reader,
         ple_retention_broker, ple_curriculum_schedule_revision_broker;
GRANT SELECT, INSERT, UPDATE, DELETE ON public.teaching_course_module,
    public.teaching_course_assignment_position TO ple_curriculum_adoption_broker;
GRANT SELECT ON public.assignment_run, public.enrollment
    TO ple_curriculum_adoption_broker;

-- Trigger names intentionally sort before curriculum_course_schedule_revision.
-- The module topology trigger therefore observes no schedule row for a just
-- created course and leaves its normal course trigger to establish revision 1.
CREATE FUNCTION public.ple_create_course_default_module_v1()
RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public', pg_temp AS $$
BEGIN
    INSERT INTO public.teaching_course_module (
        tenant_id, course_id, position, title, is_default
    ) VALUES (NEW.tenant_id, NEW.course_id, 0, 'Assignments', true);
    RETURN NEW;
END $$;

-- ASVS 2.3.4: the locked default module serializes appends within one
-- ordinary course, preventing concurrent writers from claiming one position.
CREATE FUNCTION public.ple_attach_assignment_to_default_module_v1()
RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public', pg_temp AS $$
DECLARE
    v_module uuid;
    v_position integer;
BEGIN
    SELECT module_row.course_module_id INTO v_module
      FROM public.teaching_course_module AS module_row
     WHERE module_row.tenant_id = NEW.tenant_id
       AND module_row.course_id = NEW.course_id
       AND module_row.is_default
     FOR UPDATE;
    IF NOT FOUND THEN
        RAISE EXCEPTION 'ordinary course default module is unavailable'
            USING ERRCODE = '23503';
    END IF;

    SELECT COALESCE(max(position_row.position) + 1, 0) INTO v_position
      FROM public.teaching_course_assignment_position AS position_row
     WHERE position_row.tenant_id = NEW.tenant_id
       AND position_row.course_id = NEW.course_id
       AND position_row.course_module_id = v_module;

    INSERT INTO public.teaching_course_assignment_position (
        tenant_id, course_id, assignment_id, course_module_id, position
    ) VALUES (
        NEW.tenant_id, NEW.course_id, NEW.assignment_id, v_module, v_position
    );
    RETURN NEW;
END $$;

-- PostgreSQL may reach the course-to-module cascade before its independent
-- course-to-assignment cascade.  Clear only the derived position rows first,
-- so a course teardown remains atomic while a direct module delete stays
-- protected by the module FK's ON DELETE RESTRICT action.
CREATE FUNCTION public.ple_remove_course_assignment_positions_v1()
RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public', pg_temp AS $$
BEGIN
    DELETE FROM public.teaching_course_assignment_position AS position_row
     WHERE position_row.tenant_id = OLD.tenant_id
       AND position_row.course_id = OLD.course_id;
    RETURN OLD;
END $$;

-- The partial index prevents two defaults; this deferred constraint supplies
-- the complementary at-least-one condition while permitting a transactional
-- default handoff and the course-delete cascade.
CREATE FUNCTION public.ple_require_course_default_module_v1()
RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public', pg_temp AS $$
DECLARE v_tenant uuid; v_course uuid;
BEGIN
    IF TG_OP = 'DELETE' THEN
        v_tenant := OLD.tenant_id;
        v_course := OLD.course_id;
    ELSE
        v_tenant := NEW.tenant_id;
        v_course := NEW.course_id;
    END IF;

    IF NOT EXISTS (
        SELECT 1
          FROM public.course AS course_row
         WHERE course_row.tenant_id = v_tenant
           AND course_row.course_id = v_course
    ) THEN
        RETURN NULL;
    END IF;

    IF (SELECT count(*)
          FROM public.teaching_course_module AS module_row
         WHERE module_row.tenant_id = v_tenant
           AND module_row.course_id = v_course
           AND module_row.is_default) <> 1
    THEN
        RAISE EXCEPTION 'ordinary course must retain exactly one default module'
            USING ERRCODE = '23514';
    END IF;
    RETURN NULL;
END $$;

-- Do not manufacture a second creation revision: the course trigger that
-- creates course_schedule_revision has not fired when the default module is
-- inserted.  Any later meaningful topology mutation invalidates the witness.
CREATE FUNCTION public.ple_bump_course_topology_schedule_revision_v1()
RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public', pg_temp AS $$
DECLARE v_tenant uuid; v_course uuid;
BEGIN
    IF TG_OP = 'DELETE' THEN
        v_tenant := OLD.tenant_id;
        v_course := OLD.course_id;
    ELSE
        v_tenant := NEW.tenant_id;
        v_course := NEW.course_id;
    END IF;

    IF TG_OP = 'UPDATE' THEN
        IF TG_TABLE_NAME = 'teaching_course_module' THEN
            IF ROW(NEW.position, NEW.title, NEW.is_default)
                   IS NOT DISTINCT FROM ROW(OLD.position, OLD.title, OLD.is_default)
            THEN
                RETURN NEW;
            END IF;
        ELSIF TG_TABLE_NAME = 'teaching_course_assignment_position' THEN
            IF ROW(NEW.course_module_id, NEW.position)
                   IS NOT DISTINCT FROM ROW(OLD.course_module_id, OLD.position)
            THEN
                RETURN NEW;
            END IF;
        END IF;
    END IF;

    IF NOT EXISTS (
        SELECT 1
          FROM public.course_schedule_revision AS schedule_row
         WHERE schedule_row.tenant_id = v_tenant
           AND schedule_row.course_id = v_course
    ) THEN
        IF TG_OP = 'DELETE' THEN
            RETURN OLD;
        END IF;
        RETURN NEW;
    END IF;

    PERFORM public.ple_advance_course_schedule_revision_v1(
        v_tenant, v_course, false, current_user::name
    );
    IF TG_OP = 'DELETE' THEN
        RETURN OLD;
    END IF;
    RETURN NEW;
END $$;

-- Assignment insertion first receives an ordinary topology row.  The existing
-- base-policy INSERT trigger then sees that attachment and does not create a
-- second schedule revision for the same logical assignment creation.
CREATE OR REPLACE FUNCTION public.ple_bump_assignment_schedule_revision_v1()
RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public', pg_temp AS $$
BEGIN
    IF TG_OP = 'INSERT' AND EXISTS (
        SELECT 1
          FROM public.teaching_course_assignment_position AS position_row
         WHERE position_row.tenant_id = NEW.tenant_id
           AND position_row.course_id = NEW.course_id
           AND position_row.assignment_id = NEW.assignment_id
    ) THEN
        RETURN NEW;
    END IF;

    IF TG_OP = 'INSERT'
       OR ROW(NEW.available_at, NEW.due_at, NEW.closes_at)
          IS DISTINCT FROM ROW(OLD.available_at, OLD.due_at, OLD.closes_at)
    THEN
        PERFORM public.ple_advance_course_schedule_revision_v1(
            NEW.tenant_id, NEW.course_id, false, current_user::name
        );
    END IF;
    RETURN NEW;
END $$;

-- An enrollment is only an entitlement.  A run rooted through that enrollment
-- is the authoritative issued-work fence for course term shifts.
CREATE FUNCTION public.ple_curriculum_adoption_course_has_issued_work_v1(
    p_tenant uuid, p_course uuid
) RETURNS boolean LANGUAGE plpgsql STABLE SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public', pg_temp AS $$
BEGIN
    IF p_tenant IS NULL OR p_course IS NULL
       OR p_tenant IS DISTINCT FROM public.ple_current_tenant()
    THEN
        RETURN false;
    END IF;

    RETURN EXISTS (
        SELECT 1
          FROM public.assignment_run AS run_row
          JOIN public.enrollment AS enrollment_row
            ON enrollment_row.tenant_id = run_row.tenant_id
           AND enrollment_row.enrollment_id = run_row.enrollment_id
          JOIN public.assignment AS assignment_row
            ON assignment_row.tenant_id = enrollment_row.tenant_id
           AND assignment_row.course_id = enrollment_row.course_id
           AND assignment_row.assignment_id = enrollment_row.assignment_id
         WHERE run_row.tenant_id = p_tenant
           AND enrollment_row.course_id = p_course
    );
END $$;

ALTER FUNCTION public.ple_create_course_default_module_v1()
    OWNER TO ple_curriculum_adoption_broker;
ALTER FUNCTION public.ple_attach_assignment_to_default_module_v1()
    OWNER TO ple_curriculum_adoption_broker;
ALTER FUNCTION public.ple_remove_course_assignment_positions_v1()
    OWNER TO ple_curriculum_adoption_broker;
ALTER FUNCTION public.ple_require_course_default_module_v1()
    OWNER TO ple_curriculum_adoption_broker;
ALTER FUNCTION public.ple_bump_course_topology_schedule_revision_v1()
    OWNER TO ple_curriculum_adoption_broker;
ALTER FUNCTION public.ple_bump_assignment_schedule_revision_v1()
    OWNER TO ple_curriculum_adoption_broker;
ALTER FUNCTION public.ple_curriculum_adoption_course_has_issued_work_v1(uuid, uuid)
    OWNER TO ple_curriculum_adoption_broker;

COMMENT ON FUNCTION public.ple_curriculum_adoption_course_has_issued_work_v1(uuid, uuid) IS
    'True exactly when an assignment run exists through an enrollment in the course.';

REVOKE ALL ON FUNCTION public.ple_create_course_default_module_v1(),
    public.ple_attach_assignment_to_default_module_v1(),
    public.ple_remove_course_assignment_positions_v1(),
    public.ple_require_course_default_module_v1(),
    public.ple_bump_course_topology_schedule_revision_v1(),
    public.ple_bump_assignment_schedule_revision_v1()
    FROM PUBLIC, ple_app, ple_auth, ple_student, ple_grader, ple_grading_reader,
         ple_retention_broker, ple_curriculum_adoption_broker;
REVOKE ALL ON FUNCTION public.ple_curriculum_adoption_course_has_issued_work_v1(uuid, uuid)
    FROM PUBLIC, ple_app, ple_auth, ple_student, ple_grader, ple_grading_reader,
         ple_retention_broker;
GRANT EXECUTE ON FUNCTION public.ple_curriculum_adoption_course_has_issued_work_v1(uuid, uuid)
    TO ple_curriculum_adoption_broker;

CREATE TRIGGER curriculum_course_default_module
AFTER INSERT ON public.course
FOR EACH ROW EXECUTE FUNCTION public.ple_create_course_default_module_v1();
CREATE TRIGGER curriculum_course_remove_module_positions
BEFORE DELETE ON public.course
FOR EACH ROW EXECUTE FUNCTION public.ple_remove_course_assignment_positions_v1();
CREATE TRIGGER curriculum_assignment_default_module_position
AFTER INSERT ON public.assignment
FOR EACH ROW EXECUTE FUNCTION public.ple_attach_assignment_to_default_module_v1();
CREATE CONSTRAINT TRIGGER curriculum_course_module_exactly_one_default
AFTER INSERT OR UPDATE OR DELETE ON public.teaching_course_module
DEFERRABLE INITIALLY DEFERRED FOR EACH ROW
EXECUTE FUNCTION public.ple_require_course_default_module_v1();
CREATE TRIGGER curriculum_course_module_schedule_revision
AFTER INSERT OR DELETE OR UPDATE OF position, title, is_default
ON public.teaching_course_module
FOR EACH ROW EXECUTE FUNCTION public.ple_bump_course_topology_schedule_revision_v1();
CREATE TRIGGER curriculum_assignment_position_schedule_revision
AFTER INSERT OR DELETE OR UPDATE OF course_module_id, position
ON public.teaching_course_assignment_position
FOR EACH ROW EXECUTE FUNCTION public.ple_bump_course_topology_schedule_revision_v1();

-- Migration 1846 adds the term-shift coalescer after its materializer holds
-- the course schedule row.  A caller-settable GUC is not part of this design:
-- ordinary assignment writers always retain their witness invalidation.

-- Treat the topology as another B2-owned capability boundary.  Installation
-- fails closed if a future edit opens table/function authority or weakens RLS.
DO $$
DECLARE
    v_relation text;
    v_role text;
    v_function regprocedure;
BEGIN
    IF EXISTS (
        SELECT 1
          FROM pg_roles
         WHERE rolname = 'ple_curriculum_adoption_broker'
           AND (rolcanlogin OR rolinherit OR rolbypassrls OR rolsuper
                OR rolcreatedb OR rolcreaterole OR rolreplication)
    ) OR EXISTS (
        SELECT 1
          FROM pg_auth_members
         WHERE roleid = 'ple_curriculum_adoption_broker'::regrole
            OR member = 'ple_curriculum_adoption_broker'::regrole
    ) THEN
        RAISE EXCEPTION 'curriculum adoption topology broker role is unsafe';
    END IF;

    IF EXISTS (
        SELECT 1
          FROM pg_class AS relation_row
          JOIN pg_namespace AS namespace ON namespace.oid = relation_row.relnamespace
         WHERE namespace.nspname = 'public'
           AND relation_row.relname = ANY (ARRAY[
               'teaching_course_module', 'teaching_course_assignment_position'
           ])
           AND (NOT relation_row.relrowsecurity OR NOT relation_row.relforcerowsecurity)
    ) THEN
        RAISE EXCEPTION 'curriculum adoption topology relation is not forced-RLS';
    END IF;

    FOREACH v_role IN ARRAY ARRAY[
        'public', 'ple_app', 'ple_auth', 'ple_student', 'ple_grader',
        'ple_grading_reader', 'ple_retention_broker',
        'ple_curriculum_schedule_revision_broker'
    ] LOOP
        FOREACH v_relation IN ARRAY ARRAY[
            'teaching_course_module', 'teaching_course_assignment_position'
        ] LOOP
            IF has_table_privilege(v_role, 'public.' || v_relation, 'SELECT')
               OR has_table_privilege(v_role, 'public.' || v_relation, 'INSERT')
               OR has_table_privilege(v_role, 'public.' || v_relation, 'UPDATE')
               OR has_table_privilege(v_role, 'public.' || v_relation, 'DELETE')
               OR has_table_privilege(v_role, 'public.' || v_relation, 'TRUNCATE')
               OR has_table_privilege(v_role, 'public.' || v_relation, 'REFERENCES')
               OR has_table_privilege(v_role, 'public.' || v_relation, 'TRIGGER')
            THEN
                RAISE EXCEPTION 'curriculum adoption topology authority leaked to %', v_role;
            END IF;
        END LOOP;
    END LOOP;

    FOREACH v_relation IN ARRAY ARRAY[
        'teaching_course_module', 'teaching_course_assignment_position'
    ] LOOP
        IF NOT has_table_privilege(
               'ple_curriculum_adoption_broker', 'public.' || v_relation, 'SELECT'
           )
           OR NOT has_table_privilege(
               'ple_curriculum_adoption_broker', 'public.' || v_relation, 'INSERT'
           )
           OR NOT has_table_privilege(
               'ple_curriculum_adoption_broker', 'public.' || v_relation, 'UPDATE'
           )
           OR NOT has_table_privilege(
               'ple_curriculum_adoption_broker', 'public.' || v_relation, 'DELETE'
           )
           OR has_table_privilege(
               'ple_curriculum_adoption_broker', 'public.' || v_relation, 'TRUNCATE'
           )
           OR has_table_privilege(
               'ple_curriculum_adoption_broker', 'public.' || v_relation, 'REFERENCES'
           )
           OR has_table_privilege(
               'ple_curriculum_adoption_broker', 'public.' || v_relation, 'TRIGGER'
           )
        THEN
            RAISE EXCEPTION 'curriculum adoption topology broker grants are unsafe';
        END IF;
    END LOOP;

    FOREACH v_relation IN ARRAY ARRAY['assignment_run', 'enrollment'] LOOP
        IF NOT has_table_privilege(
               'ple_curriculum_adoption_broker', 'public.' || v_relation, 'SELECT'
           )
           OR has_table_privilege(
               'ple_curriculum_adoption_broker', 'public.' || v_relation, 'INSERT'
           )
           OR has_table_privilege(
               'ple_curriculum_adoption_broker', 'public.' || v_relation, 'UPDATE'
           )
           OR has_table_privilege(
               'ple_curriculum_adoption_broker', 'public.' || v_relation, 'DELETE'
           )
           OR has_table_privilege(
               'ple_curriculum_adoption_broker', 'public.' || v_relation, 'TRUNCATE'
           )
           OR has_table_privilege(
               'ple_curriculum_adoption_broker', 'public.' || v_relation, 'REFERENCES'
           )
           OR has_table_privilege(
               'ple_curriculum_adoption_broker', 'public.' || v_relation, 'TRIGGER'
           )
        THEN
            RAISE EXCEPTION 'curriculum adoption issued-work authority is unsafe';
        END IF;
    END LOOP;

    FOREACH v_function IN ARRAY ARRAY[
        'public.ple_create_course_default_module_v1()'::regprocedure,
        'public.ple_attach_assignment_to_default_module_v1()'::regprocedure,
        'public.ple_remove_course_assignment_positions_v1()'::regprocedure,
        'public.ple_require_course_default_module_v1()'::regprocedure,
        'public.ple_bump_course_topology_schedule_revision_v1()'::regprocedure,
        'public.ple_bump_assignment_schedule_revision_v1()'::regprocedure,
        'public.ple_curriculum_adoption_course_has_issued_work_v1(uuid,uuid)'::regprocedure
    ] LOOP
        IF (SELECT pg_get_userbyid(proowner) FROM pg_proc WHERE oid = v_function)
           <> 'ple_curriculum_adoption_broker'
        THEN
            RAISE EXCEPTION 'curriculum adoption topology function ownership is unsafe';
        END IF;
    END LOOP;

    FOREACH v_function IN ARRAY ARRAY[
        'public.ple_create_course_default_module_v1()'::regprocedure,
        'public.ple_attach_assignment_to_default_module_v1()'::regprocedure,
        'public.ple_remove_course_assignment_positions_v1()'::regprocedure,
        'public.ple_require_course_default_module_v1()'::regprocedure,
        'public.ple_bump_course_topology_schedule_revision_v1()'::regprocedure,
        'public.ple_bump_assignment_schedule_revision_v1()'::regprocedure
    ] LOOP
        FOREACH v_role IN ARRAY ARRAY[
            'public', 'ple_app', 'ple_auth', 'ple_student', 'ple_grader',
            'ple_grading_reader', 'ple_retention_broker',
            'ple_curriculum_adoption_broker'
        ] LOOP
            IF has_function_privilege(v_role, v_function, 'EXECUTE') THEN
                RAISE EXCEPTION 'curriculum adoption topology helper leaked to %', v_role;
            END IF;
        END LOOP;
    END LOOP;

    IF NOT has_function_privilege(
           'ple_curriculum_adoption_broker',
           'public.ple_curriculum_adoption_course_has_issued_work_v1(uuid,uuid)'::regprocedure,
           'EXECUTE'
       ) THEN
        RAISE EXCEPTION 'curriculum adoption issued-work predicate is unavailable';
    END IF;
    FOREACH v_role IN ARRAY ARRAY[
        'public', 'ple_app', 'ple_auth', 'ple_student', 'ple_grader',
        'ple_grading_reader', 'ple_retention_broker'
    ] LOOP
        IF has_function_privilege(
               v_role,
               'public.ple_curriculum_adoption_course_has_issued_work_v1(uuid,uuid)'::regprocedure,
               'EXECUTE'
           ) THEN
            RAISE EXCEPTION 'curriculum adoption issued-work predicate leaked to %', v_role;
        END IF;
    END LOOP;
END $$;

COMMIT;
