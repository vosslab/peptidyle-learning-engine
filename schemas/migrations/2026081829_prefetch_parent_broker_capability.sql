-- Complete the learner-work broker's parent-row capability for immutable
-- prefetch private execution. The broker must lock the public parent before it
-- records or promotes the sealed child, and promotion consumes that parent.

BEGIN;

GRANT SELECT, DELETE ON TABLE public.question_prefetch TO ple_learner_work_broker;
-- PostgreSQL requires UPDATE authority to take the function's FOR KEY SHARE
-- and FOR UPDATE row locks. Restrict it to one immutable key column, matching
-- the established learner-work broker locking pattern.
GRANT UPDATE (predecessor_attempt_id)
    ON TABLE public.question_prefetch TO ple_learner_work_broker;

CREATE POLICY learner_work_broker_question_prefetch_select
    ON public.question_prefetch
    FOR SELECT
    TO ple_learner_work_broker
    USING (tenant_id = public.ple_current_tenant());

CREATE POLICY learner_work_broker_question_prefetch_delete
    ON public.question_prefetch
    FOR DELETE
    TO ple_learner_work_broker
    USING (tenant_id = public.ple_current_tenant());

DO $$
DECLARE
    relation_oid oid := 'public.question_prefetch'::regclass;
BEGIN
    IF NOT EXISTS (
        SELECT 1
          FROM pg_catalog.pg_class AS relation
         WHERE relation.oid = relation_oid
           AND relation.relrowsecurity
           AND relation.relforcerowsecurity
    ) THEN
        RAISE EXCEPTION 'question_prefetch must retain forced row-level security';
    END IF;

    IF NOT has_table_privilege('ple_learner_work_broker', relation_oid, 'SELECT')
       OR NOT has_table_privilege('ple_learner_work_broker', relation_oid, 'DELETE')
       OR has_table_privilege('ple_learner_work_broker', relation_oid, 'INSERT')
       OR has_table_privilege('ple_learner_work_broker', relation_oid, 'UPDATE')
       OR has_table_privilege('ple_learner_work_broker', relation_oid, 'TRUNCATE')
       OR has_table_privilege('ple_learner_work_broker', relation_oid, 'REFERENCES')
       OR has_table_privilege('ple_learner_work_broker', relation_oid, 'TRIGGER') THEN
        RAISE EXCEPTION 'learner-work broker question_prefetch privilege matrix is unsafe';
    END IF;

    IF NOT has_column_privilege(
        'ple_learner_work_broker', relation_oid, 'predecessor_attempt_id', 'UPDATE'
    ) OR EXISTS (
        SELECT 1
          FROM pg_catalog.pg_attribute AS attribute
         WHERE attribute.attrelid = relation_oid
           AND attribute.attnum > 0
           AND NOT attribute.attisdropped
           AND attribute.attname <> 'predecessor_attempt_id'
           AND has_column_privilege(
               'ple_learner_work_broker', relation_oid, attribute.attname, 'UPDATE'
           )
    ) THEN
        RAISE EXCEPTION 'learner-work broker question_prefetch lock capability is unsafe';
    END IF;

    IF NOT EXISTS (
        SELECT 1
          FROM pg_catalog.pg_policy AS policy
         WHERE policy.polrelid = relation_oid
           AND policy.polname = 'learner_work_broker_question_prefetch_select'
           AND policy.polcmd = 'r'
           AND policy.polpermissive
           AND 'ple_learner_work_broker'::regrole::oid = ANY (policy.polroles)
           AND pg_get_expr(policy.polqual, policy.polrelid)
               = '(tenant_id = ple_current_tenant())'
           AND policy.polwithcheck IS NULL
    ) OR NOT EXISTS (
        SELECT 1
          FROM pg_catalog.pg_policy AS policy
         WHERE policy.polrelid = relation_oid
           AND policy.polname = 'learner_work_broker_question_prefetch_delete'
           AND policy.polcmd = 'd'
           AND policy.polpermissive
           AND 'ple_learner_work_broker'::regrole::oid = ANY (policy.polroles)
           AND pg_get_expr(policy.polqual, policy.polrelid)
               = '(tenant_id = ple_current_tenant())'
           AND policy.polwithcheck IS NULL
    ) THEN
        RAISE EXCEPTION 'learner-work broker question_prefetch RLS policy matrix is unsafe';
    END IF;
END
$$;

COMMIT;
