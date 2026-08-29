-- Establish the current Student vocabulary for role-bearing persistence.
--
-- This is a clean-volume pre-production cutover. Relation identity remains
-- stable for existing grants, RLS policies, and foreign keys; only the public
-- catalog spelling changes. Function bodies that retain SQL text are rebuilt
-- from the effective catalog with their owner, ACL, and security settings
-- preserved (ASVS 1.2.4, 8.2.1, 8.2.2, 8.3.1).

BEGIN;

DO $$
BEGIN
    IF to_regclass('public.tenant_learner_identity') IS NULL
       OR to_regclass('public.catalog_discovery_learner_fingerprint_receipt') IS NULL THEN
        RAISE EXCEPTION 'Student schema vocabulary requires the exact pre-1881 catalog'
            USING ERRCODE = '55000';
    END IF;
END;
$$;

ALTER TABLE public.tenant_learner_identity RENAME TO tenant_student_identity;
ALTER TABLE public.catalog_discovery_learner_fingerprint_receipt
    RENAME TO catalog_discovery_student_fingerprint_receipt;
ALTER TABLE public.catalog_discovery_student_fingerprint_receipt
    RENAME COLUMN learner_fingerprint TO student_fingerprint;

-- Constraint names are resolved by relation and constrained columns. This
-- avoids preserving PostgreSQL's truncated generated names as source truth.
DO $$
DECLARE
    v_identity regclass := 'public.tenant_student_identity'::regclass;
    v_receipt regclass := 'public.catalog_discovery_student_fingerprint_receipt'::regclass;
    v_member regclass := 'public.course_member'::regclass;
    v_name text;
BEGIN
    SELECT conname INTO v_name
      FROM pg_constraint WHERE conrelid = v_identity AND contype = 'p';
    IF v_name IS NULL THEN
        RAISE EXCEPTION 'Student identity primary key is unavailable';
    END IF;
    EXECUTE format(
        'ALTER TABLE public.tenant_student_identity RENAME CONSTRAINT %I TO tenant_student_identity_pkey',
        v_name
    );

    SELECT conname INTO v_name FROM pg_constraint
     WHERE conrelid=v_identity AND contype='u' AND conkey=ARRAY[
        (SELECT attnum FROM pg_attribute WHERE attrelid=v_identity AND attname='tenant_id'),
        (SELECT attnum FROM pg_attribute WHERE attrelid=v_identity AND attname='student_id')
    ];
    IF v_name IS NULL THEN
        RAISE EXCEPTION 'Student identity student unique constraint is unavailable';
    END IF;
    EXECUTE format(
        'ALTER TABLE public.tenant_student_identity RENAME CONSTRAINT %I TO ' ||
            'tenant_student_identity_tenant_id_student_id_key',
        v_name
    );

    SELECT conname INTO v_name FROM pg_constraint
     WHERE conrelid=v_identity AND contype='u' AND conkey=ARRAY[
        (SELECT attnum FROM pg_attribute WHERE attrelid=v_identity AND attname='tenant_id'),
        (SELECT attnum FROM pg_attribute WHERE attrelid=v_identity AND attname='user_id'),
        (SELECT attnum FROM pg_attribute WHERE attrelid=v_identity AND attname='student_id')
    ];
    IF v_name IS NULL THEN
        RAISE EXCEPTION 'Student identity user unique constraint is unavailable';
    END IF;
    EXECUTE format(
        'ALTER TABLE public.tenant_student_identity RENAME CONSTRAINT %I TO ' ||
            'tenant_student_identity_tenant_id_user_id_student_id_key',
        v_name
    );

    SELECT conname INTO v_name FROM pg_constraint
     WHERE conrelid=v_member AND contype='f' AND confrelid=v_identity
       AND conkey=ARRAY[
        (SELECT attnum FROM pg_attribute WHERE attrelid=v_member AND attname='tenant_id'),
        (SELECT attnum FROM pg_attribute WHERE attrelid=v_member AND attname='user_id'),
        (SELECT attnum FROM pg_attribute WHERE attrelid=v_member AND attname='student_id')
    ];
    IF v_name IS NULL THEN
        RAISE EXCEPTION 'course membership Student foreign key is unavailable';
    END IF;
    EXECUTE format(
        'ALTER TABLE public.course_member RENAME CONSTRAINT %I TO course_membership_student_fkey',
        v_name
    );

    SELECT conname INTO v_name FROM pg_constraint WHERE conrelid=v_receipt AND contype='p';
    IF v_name IS NULL THEN
        RAISE EXCEPTION 'Student fingerprint receipt primary key is unavailable';
    END IF;
    EXECUTE format(
        'ALTER TABLE public.catalog_discovery_student_fingerprint_receipt ' ||
            'RENAME CONSTRAINT %I TO catalog_discovery_student_fingerprint_receipt_pkey',
        v_name
    );
    SELECT conname INTO v_name FROM pg_constraint WHERE conrelid=v_receipt AND contype='f';
    IF v_name IS NULL THEN
        RAISE EXCEPTION 'Student fingerprint receipt problem key is unavailable';
    END IF;
    EXECUTE format(
        'ALTER TABLE public.catalog_discovery_student_fingerprint_receipt ' ||
            'RENAME CONSTRAINT %I TO catalog_discovery_student_fingerprint_receipt_problem_fkey',
        v_name
    );
    SELECT conname INTO v_name FROM pg_constraint WHERE conrelid=v_receipt AND contype='c';
    IF v_name IS NULL THEN
        RAISE EXCEPTION 'Student fingerprint receipt digest check is unavailable';
    END IF;
    EXECUTE format(
        'ALTER TABLE public.catalog_discovery_student_fingerprint_receipt ' ||
            'RENAME CONSTRAINT %I TO catalog_discovery_student_fingerprint_receipt_digest_check',
        v_name
    );
END;
$$;

ALTER INDEX public.course_roster_member_learner_fk_idx
    RENAME TO course_roster_member_student_fk_idx;

ALTER POLICY tenant_learner_identity_app ON public.tenant_student_identity
    RENAME TO tenant_student_identity_app;
ALTER POLICY tenant_learner_identity_retention ON public.tenant_student_identity
    RENAME TO tenant_student_identity_retention;
ALTER POLICY catalog_discovery_learner_fingerprint_statistics_select
    ON public.catalog_discovery_student_fingerprint_receipt
    RENAME TO catalog_discovery_student_fingerprint_statistics_select;
ALTER POLICY catalog_discovery_learner_fingerprint_statistics_insert
    ON public.catalog_discovery_student_fingerprint_receipt
    RENAME TO catalog_discovery_student_fingerprint_statistics_insert;

-- PLE's course-creation and catalog-statistics functions retain PL/pgSQL
-- source text. Recreate their effective definitions after the relation rename,
-- preserving their authority attributes while replacing every role-bearing
-- catalog spelling they own.
DO $$
DECLARE
    v_proc regprocedure;
    v_definition text;
    v_rebuilt text;
    v_owner oid;
    v_acl aclitem[];
    v_config text[];
    v_security boolean;
BEGIN
    FOR v_proc IN
        SELECT p.oid::regprocedure FROM pg_proc AS p
         JOIN pg_namespace AS n ON n.oid=p.pronamespace
         WHERE n.nspname='public'
           AND pg_get_functiondef(p.oid) LIKE '%tenant_learner_identity%'
    LOOP
        SELECT pg_get_functiondef(v_proc), proowner, proacl, proconfig, prosecdef
          INTO v_definition, v_owner, v_acl, v_config, v_security
          FROM pg_proc WHERE oid=v_proc;
        v_rebuilt := replace(replace(replace(v_definition,
            'tenant_learner_identity', 'tenant_student_identity'),
            'tenant_learner_identity_pkey', 'tenant_student_identity_pkey'),
            'learner identity', 'Student identity');
        EXECUTE v_rebuilt;
        IF NOT EXISTS (
            SELECT 1 FROM pg_proc
             WHERE oid=v_proc AND proowner=v_owner
               AND proacl IS NOT DISTINCT FROM v_acl
               AND proconfig IS NOT DISTINCT FROM v_config
               AND prosecdef=v_security
        ) THEN
            RAISE EXCEPTION 'Student identity function authority changed for %', v_proc;
        END IF;
    END LOOP;

    v_proc :=
        ('public.ple_record_question_statistics(uuid,uuid,uuid,uuid,uuid,uuid,' ||
         'double precision,bigint,bigint,double precision,bytea)')::regprocedure;
    SELECT pg_get_functiondef(v_proc), proowner, proacl, proconfig, prosecdef
      INTO v_definition, v_owner, v_acl, v_config, v_security FROM pg_proc WHERE oid=v_proc;
    v_rebuilt := replace(replace(replace(v_definition,
        'catalog_discovery_learner_fingerprint_receipt',
        'catalog_discovery_student_fingerprint_receipt'),
        'learner_fingerprint', 'student_fingerprint'),
        'ple-catalog-learner-evidence-v1:', 'ple-catalog-student-evidence-v1:');
    EXECUTE v_rebuilt;
    IF NOT EXISTS (
        SELECT 1 FROM pg_proc
         WHERE oid=v_proc AND proowner=v_owner
           AND proacl IS NOT DISTINCT FROM v_acl
           AND proconfig IS NOT DISTINCT FROM v_config
           AND prosecdef=v_security
    ) THEN
        RAISE EXCEPTION 'Student fingerprint function authority changed';
    END IF;
END;
$$;

-- The Instructor operation row has a stable function identity but a
-- role-bearing result label. ALTER FUNCTION cannot rename that output field;
-- rebuild the effective implementation from its current definition.
DO $$
DECLARE
    v_proc regprocedure :=
        ('public.ple_list_instructor_grading_operations_v1(uuid,character,uuid,uuid,' ||
         'text,text,integer,integer)')::regprocedure;
    v_definition text;
    v_rebuilt text;
    v_owner oid;
    v_acl aclitem[];
    v_config text[];
    v_security boolean;
BEGIN
    SELECT pg_get_functiondef(v_proc), proowner, proacl, proconfig, prosecdef
      INTO v_definition, v_owner, v_acl, v_config, v_security FROM pg_proc WHERE oid=v_proc;
    v_rebuilt := replace(replace(v_definition, 'learner_display_name', 'student_display_name'),
        '''Learner''', '''Student''');
    EXECUTE v_rebuilt;
    IF NOT EXISTS (
        SELECT 1 FROM pg_proc
         WHERE oid=v_proc AND proowner=v_owner
           AND proacl IS NOT DISTINCT FROM v_acl
           AND proconfig IS NOT DISTINCT FROM v_config
           AND prosecdef=v_security
    ) THEN
        RAISE EXCEPTION 'Instructor grading-operation function authority changed';
    END IF;
END;
$$;

COMMIT;
