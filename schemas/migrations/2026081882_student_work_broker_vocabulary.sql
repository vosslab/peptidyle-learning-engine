-- Student-work broker vocabulary for the clean-volume live stack.
-- The role, policies, immutable-write fence, and preparation rowsets have one
-- Student-oriented vocabulary while preserving the established capability graph.

BEGIN;

ALTER ROLE ple_learner_work_broker RENAME TO ple_student_work_broker;
ALTER ROLE ple_student_work_broker
    NOLOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE NOINHERIT NOREPLICATION NOBYPASSRLS;

ALTER FUNCTION public.ple_fence_learner_record_write() RENAME TO ple_fence_student_record_write;
ALTER FUNCTION public.ple_learner_work_deny_internal() RENAME TO ple_student_work_deny_internal;
ALTER FUNCTION public.ple_learner_work_probe_authority_internal(uuid, uuid, uuid, text)
    RENAME TO ple_student_work_probe_authority_internal;
ALTER FUNCTION public.ple_learner_work_prepare_internal(
    uuid, uuid, uuid, uuid, uuid, text, text, uuid, uuid
) RENAME TO ple_student_work_prepare_internal;

ALTER POLICY learner_work_broker_course_tenant ON public.course
    RENAME TO student_work_broker_course_tenant;
ALTER POLICY student_work_broker_course_tenant ON public.course TO ple_student_work_broker;
ALTER POLICY learner_work_broker_assignment_tenant ON public.assignment
    RENAME TO student_work_broker_assignment_tenant;
ALTER POLICY student_work_broker_assignment_tenant ON public.assignment TO ple_student_work_broker;
ALTER POLICY learner_work_broker_member_tenant ON public.course_member
    RENAME TO student_work_broker_member_tenant;
ALTER POLICY student_work_broker_member_tenant ON public.course_member TO ple_student_work_broker;
ALTER POLICY learner_work_broker_group_tenant ON public.course_group
    RENAME TO student_work_broker_group_tenant;
ALTER POLICY student_work_broker_group_tenant ON public.course_group TO ple_student_work_broker;
ALTER POLICY learner_work_broker_group_member_tenant ON public.course_group_member
    RENAME TO student_work_broker_group_member_tenant;
ALTER POLICY student_work_broker_group_member_tenant ON public.course_group_member
    TO ple_student_work_broker;
ALTER POLICY learner_work_broker_audience_tenant ON public.assignment_audience_group
    RENAME TO student_work_broker_audience_tenant;
ALTER POLICY student_work_broker_audience_tenant ON public.assignment_audience_group
    TO ple_student_work_broker;
ALTER POLICY learner_work_broker_enrollment_tenant ON public.enrollment
    RENAME TO student_work_broker_enrollment_tenant;
ALTER POLICY student_work_broker_enrollment_tenant ON public.enrollment TO ple_student_work_broker;
ALTER POLICY learner_work_broker_run_tenant ON public.assignment_run
    RENAME TO student_work_broker_run_tenant;
ALTER POLICY student_work_broker_run_tenant ON public.assignment_run TO ple_student_work_broker;
ALTER POLICY learner_work_broker_attempt_tenant ON public.question_attempt
    RENAME TO student_work_broker_attempt_tenant;
ALTER POLICY student_work_broker_attempt_tenant ON public.question_attempt TO ple_student_work_broker;
ALTER POLICY learner_work_broker_summary_tenant ON public.student_assignment_summary
    RENAME TO student_work_broker_summary_tenant;
ALTER POLICY student_work_broker_summary_tenant ON public.student_assignment_summary
    TO ple_student_work_broker;
ALTER POLICY learner_work_broker_question_prefetch_select ON public.question_prefetch
    RENAME TO student_work_broker_question_prefetch_select;
ALTER POLICY student_work_broker_question_prefetch_select ON public.question_prefetch
    TO ple_student_work_broker;
ALTER POLICY learner_work_broker_question_prefetch_delete ON public.question_prefetch
    RENAME TO student_work_broker_question_prefetch_delete;
ALTER POLICY student_work_broker_question_prefetch_delete ON public.question_prefetch
    TO ple_student_work_broker;

-- The existing definitions are the authoritative security-definer bodies.
-- Rewriting their role-bound identifiers here keeps the migration compact and
-- preserves every independent change made to their established capability logic.
CREATE TEMPORARY TABLE student_work_function_source (
    sort_order integer PRIMARY KEY,
    function_identity regprocedure NOT NULL,
    definition text NOT NULL
) ON COMMIT DROP;

INSERT INTO student_work_function_source (sort_order, function_identity, definition)
SELECT source.sort_order, source.function_identity,
       pg_get_functiondef(source.function_identity)
  FROM (VALUES
        (10, 'public.ple_student_work_deny_internal()'::regprocedure),
        (20, 'public.ple_student_work_probe_authority_internal(uuid,uuid,uuid,text)'::regprocedure),
        (30, 'public.ple_student_work_prepare_internal(uuid,uuid,uuid,uuid,uuid,text,text,uuid,uuid)'::regprocedure),
        (40, 'public.ple_prepare_entitlement_materialization(uuid,uuid,uuid,uuid,text,uuid)'::regprocedure),
        (50, 'public.ple_prepare_student_run_work(uuid,uuid,uuid,uuid,uuid)'::regprocedure),
        (60, 'public.ple_prepare_attempt_work(uuid,uuid,uuid,uuid,uuid,text)'::regprocedure),
        (70, 'public.ple_prepare_rule_entitlement_materialization(uuid,uuid,uuid,uuid,text)'::regprocedure),
        (80, 'public.ple_prepare_sealed_private_execution(uuid,uuid,uuid,uuid,uuid)'::regprocedure),
        (90, 'public.ple_verify_native_private_execution_shape(uuid,uuid)'::regprocedure),
        (100, (
            'public.ple_write_issued_attempt_private_execution(uuid,uuid,boolean,jsonb,' ||
            'character,boolean,jsonb,character,jsonb,character,boolean,bytea,character)'
        )::regprocedure),
        (110, (
            'public.ple_write_prefetch_private_execution(uuid,uuid,uuid,integer,boolean,' ||
            'jsonb,character,boolean,jsonb,character,jsonb,character,boolean,bytea,character)'
        )::regprocedure),
        (120, 'public.ple_promote_prefetch_private_execution(uuid,uuid,uuid,uuid,integer)'::regprocedure)
       ) AS source(sort_order, function_identity);

-- Return-column names define the public preparation row type.  Recreate only
-- the four affected wrappers and their three private dependencies; the sealed
-- reader and private-execution writers retain their identities via OR REPLACE.
DROP FUNCTION public.ple_prepare_entitlement_materialization(uuid, uuid, uuid, uuid, text, uuid);
DROP FUNCTION public.ple_prepare_student_run_work(uuid, uuid, uuid, uuid, uuid);
DROP FUNCTION public.ple_prepare_attempt_work(uuid, uuid, uuid, uuid, uuid, text);
DROP FUNCTION public.ple_prepare_rule_entitlement_materialization(uuid, uuid, uuid, uuid, text);
DROP FUNCTION public.ple_student_work_prepare_internal(
    uuid, uuid, uuid, uuid, uuid, text, text, uuid, uuid
);
DROP FUNCTION public.ple_student_work_probe_authority_internal(uuid, uuid, uuid, text);
DROP FUNCTION public.ple_student_work_deny_internal();

DO $$
DECLARE
    source record;
    rewritten_definition text;
BEGIN
    FOR source IN SELECT * FROM student_work_function_source ORDER BY sort_order LOOP
        rewritten_definition := replace(source.definition, 'ple_learner_work_', 'ple_student_work_');
        rewritten_definition := replace(rewritten_definition, 'p_learner', 'p_student');
        rewritten_definition := replace(rewritten_definition, 'learner_id', 'student_id');
        rewritten_definition := replace(rewritten_definition, 'learner_not_active', 'student_not_active');
        rewritten_definition := replace(rewritten_definition, 'learner work', 'Student work');
        rewritten_definition := replace(rewritten_definition, 'learner-work', 'Student-work');
        EXECUTE rewritten_definition;
    END LOOP;
END
$$;

ALTER FUNCTION public.ple_student_work_deny_internal() OWNER TO ple_student_work_broker;
ALTER FUNCTION public.ple_student_work_probe_authority_internal(uuid, uuid, uuid, text)
    OWNER TO ple_student_work_broker;
ALTER FUNCTION public.ple_student_work_prepare_internal(
    uuid, uuid, uuid, uuid, uuid, text, text, uuid, uuid
) OWNER TO ple_student_work_broker;
ALTER FUNCTION public.ple_prepare_entitlement_materialization(uuid, uuid, uuid, uuid, text, uuid)
    OWNER TO ple_student_work_broker;
ALTER FUNCTION public.ple_prepare_student_run_work(uuid, uuid, uuid, uuid, uuid)
    OWNER TO ple_student_work_broker;
ALTER FUNCTION public.ple_prepare_attempt_work(uuid, uuid, uuid, uuid, uuid, text)
    OWNER TO ple_student_work_broker;
ALTER FUNCTION public.ple_prepare_rule_entitlement_materialization(uuid, uuid, uuid, uuid, text)
    OWNER TO ple_student_work_broker;

REVOKE ALL ON FUNCTION public.ple_student_work_deny_internal(),
    public.ple_student_work_probe_authority_internal(uuid, uuid, uuid, text),
    public.ple_student_work_prepare_internal(uuid, uuid, uuid, uuid, uuid, text, text, uuid, uuid)
    FROM PUBLIC, ple_app, ple_grader, ple_grading_reader;
GRANT EXECUTE ON FUNCTION public.ple_student_work_deny_internal(),
    public.ple_student_work_probe_authority_internal(uuid, uuid, uuid, text),
    public.ple_student_work_prepare_internal(uuid, uuid, uuid, uuid, uuid, text, text, uuid, uuid)
    TO ple_student_work_broker;
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

DO $$
DECLARE
    function_identity regprocedure;
BEGIN
    IF EXISTS (SELECT 1 FROM pg_auth_members AS membership
                WHERE membership.roleid = 'ple_student_work_broker'::regrole
                   OR membership.member = 'ple_student_work_broker'::regrole) THEN
        RAISE EXCEPTION 'ple_student_work_broker must not have role memberships';
    END IF;
    IF EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'ple_student_work_broker'
                AND (rolcanlogin OR rolsuper OR rolcreatedb OR rolcreaterole OR rolinherit
                     OR rolreplication OR rolbypassrls)) THEN
        RAISE EXCEPTION 'unsafe Student-work broker attributes';
    END IF;
    IF (SELECT count(*) FROM pg_policies
         WHERE schemaname = 'public' AND policyname LIKE 'student_work_broker_%') <> 12 THEN
        RAISE EXCEPTION 'unsafe Student-work broker policy inventory';
    END IF;
    IF EXISTS (SELECT 1 FROM pg_policies
                WHERE schemaname = 'public' AND policyname LIKE 'learner_work_broker_%') THEN
        RAISE EXCEPTION 'legacy broker policies remain';
    END IF;
    FOREACH function_identity IN ARRAY ARRAY[
        'public.ple_student_work_deny_internal()'::regprocedure,
        'public.ple_student_work_probe_authority_internal(uuid,uuid,uuid,text)'::regprocedure,
        'public.ple_student_work_prepare_internal(uuid,uuid,uuid,uuid,uuid,text,text,uuid,uuid)'::regprocedure,
        'public.ple_prepare_entitlement_materialization(uuid,uuid,uuid,uuid,text,uuid)'::regprocedure,
        'public.ple_prepare_student_run_work(uuid,uuid,uuid,uuid,uuid)'::regprocedure,
        'public.ple_prepare_attempt_work(uuid,uuid,uuid,uuid,uuid,text)'::regprocedure,
        'public.ple_prepare_rule_entitlement_materialization(uuid,uuid,uuid,uuid,text)'::regprocedure
    ] LOOP
        IF NOT EXISTS (SELECT 1 FROM pg_proc AS procedure
                        WHERE procedure.oid = function_identity
                          AND procedure.proowner = 'ple_student_work_broker'::regrole
                          AND procedure.prosecdef
                          AND procedure.proconfig @> ARRAY['search_path=pg_catalog, public, pg_temp'])
           OR has_function_privilege('public', function_identity, 'EXECUTE') THEN
            RAISE EXCEPTION 'unsafe Student-work function inventory';
        END IF;
    END LOOP;
    IF NOT EXISTS (SELECT 1 FROM pg_trigger AS trigger
                    JOIN pg_proc AS procedure ON procedure.oid = trigger.tgfoid
                   WHERE trigger.tgname = 'question_attempt_retention_fence'
                     AND procedure.oid = 'public.ple_fence_student_record_write()'::regprocedure) THEN
        RAISE EXCEPTION 'Student record-write fence is not bound to its trigger';
    END IF;
END
$$;

COMMIT;
