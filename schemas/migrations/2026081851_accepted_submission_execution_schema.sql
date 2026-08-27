-- WP-PROF-G1: accepted-submission execution schema foundation.
-- Later W4 migrations install authority and callable operations.

BEGIN;

-- The sealed definer role starts unassumable and membership-free.
-- ASVS 8.1-8.4: later migrations grant only explicit machine capabilities.
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1
          FROM pg_catalog.pg_roles
         WHERE rolname = 'ple_accepted_submission_execution_worker'
    ) THEN
        CREATE ROLE ple_accepted_submission_execution_worker
            NOLOGIN
            NOINHERIT
            NOSUPERUSER
            NOCREATEDB
            NOCREATEROLE
            NOREPLICATION
            NOBYPASSRLS;
    END IF;

    IF EXISTS (
        SELECT 1
          FROM pg_catalog.pg_auth_members AS membership
         WHERE membership.roleid =
               'ple_accepted_submission_execution_worker'::regrole
            OR membership.member =
               'ple_accepted_submission_execution_worker'::regrole
    ) THEN
        RAISE EXCEPTION
            'accepted-submission execution worker must have no memberships';
    END IF;
END $$;

ALTER ROLE ple_accepted_submission_execution_worker
    NOLOGIN
    NOINHERIT
    NOSUPERUSER
    NOCREATEDB
    NOCREATEROLE
    NOREPLICATION
    NOBYPASSRLS;

REVOKE ALL ON SCHEMA public FROM ple_accepted_submission_execution_worker;

-- The synchronous API optimization receives a dedicated SET-only caller role.
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1
          FROM pg_catalog.pg_roles
         WHERE rolname = 'ple_accepted_submission_execution_fast_path'
    ) THEN
        CREATE ROLE ple_accepted_submission_execution_fast_path
            NOLOGIN
            NOINHERIT
            NOSUPERUSER
            NOCREATEDB
            NOCREATEROLE
            NOREPLICATION
            NOBYPASSRLS;
    END IF;

    IF EXISTS (
        SELECT 1
          FROM pg_catalog.pg_auth_members AS membership
         WHERE membership.roleid =
               'ple_accepted_submission_execution_fast_path'::regrole
            OR membership.member =
               'ple_accepted_submission_execution_fast_path'::regrole
    ) THEN
        RAISE EXCEPTION
            'accepted-submission fast-path caller must have no memberships';
    END IF;
END $$;

ALTER ROLE ple_accepted_submission_execution_fast_path
    NOLOGIN
    NOINHERIT
    NOSUPERUSER
    NOCREATEDB
    NOCREATEROLE
    NOREPLICATION
    NOBYPASSRLS;

REVOKE ALL ON SCHEMA public
    FROM ple_accepted_submission_execution_fast_path;
GRANT USAGE ON SCHEMA public
    TO ple_accepted_submission_execution_fast_path;
REVOKE ALL ON ALL TABLES IN SCHEMA public
    FROM ple_accepted_submission_execution_fast_path;
REVOKE ALL ON ALL SEQUENCES IN SCHEMA public
    FROM ple_accepted_submission_execution_fast_path;
REVOKE ALL ON ALL FUNCTIONS IN SCHEMA public
    FROM ple_accepted_submission_execution_fast_path;

-- This is the first canonical-evidence baseline. Rebuild pre-production data
-- rather than inventing source text for an earlier JSONB projection.
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM public.submission_receipt_snapshot)
       OR EXISTS (SELECT 1 FROM public.attempt_feedback) THEN
        RAISE EXCEPTION
            'W4 canonical evidence requires empty receipt and feedback tables; rebuild data'
            USING ERRCODE = '55000';
    END IF;
END $$;

ALTER TABLE public.grading_execution
    ADD COLUMN active_worker_id uuid;

ALTER TABLE public.submission_evaluation
    ADD COLUMN automated_result_canonical_json text,
    ADD COLUMN automated_result_sha256 character(64),
    ADD COLUMN automated_result_canonical_json_version smallint,
    ADD CONSTRAINT submission_evaluation_automated_result_pair_check CHECK (
        (automated_result_canonical_json IS NULL) =
            (automated_result_sha256 IS NULL)
        AND (automated_result_canonical_json IS NULL) =
            (automated_result_canonical_json_version IS NULL)
    ),
    ADD CONSTRAINT submission_evaluation_automated_result_size_check CHECK (
        automated_result_canonical_json IS NULL
        OR octet_length(automated_result_canonical_json) BETWEEN 1 AND 524288
    ),
    ADD CONSTRAINT submission_evaluation_automated_result_sha256_check CHECK (
        automated_result_sha256 IS NULL
        OR (
            automated_result_sha256 ~ '^[0-9a-f]{64}$'
            AND automated_result_canonical_json_version = 1
            AND automated_result_sha256 = encode(
                pg_catalog.sha256(
                    convert_to(automated_result_canonical_json, 'UTF8')
                ),
                'hex'
            )
        )
    );

ALTER TABLE public.submission_receipt_snapshot
    ADD COLUMN receipt_attempt_payload jsonb,
    ADD COLUMN receipt_attempt_canonical_json text,
    ADD COLUMN receipt_attempt_payload_sha256 character(64),
    ADD COLUMN run_canonical_json text,
    ADD COLUMN summary_canonical_json text,
    ADD COLUMN presentation_canonical_json text,
    ADD COLUMN canonical_json_version smallint NOT NULL,
    ADD CONSTRAINT submission_receipt_snapshot_attempt_payload_pair_check CHECK (
        (receipt_attempt_payload IS NULL) =
            (receipt_attempt_payload_sha256 IS NULL)
    ),
    ADD CONSTRAINT submission_receipt_snapshot_attempt_payload_shape_check CHECK (
        receipt_attempt_payload IS NULL
        OR (
            jsonb_typeof(receipt_attempt_payload) = 'object'
            AND receipt_attempt_payload ? 'id'
            AND receipt_attempt_payload ? 'tenant'
            AND receipt_attempt_payload ? 'response'
            AND receipt_attempt_payload ? 'status'
            AND receipt_attempt_payload -> 'response' = 'null'::jsonb
            AND receipt_attempt_payload ->> 'status' IN (
                'submitted',
                'auto_submitted',
                'needs_manual_grading',
                'exempt'
            )
        )
    ),
    ADD CONSTRAINT submission_receipt_snapshot_attempt_payload_sha256_check CHECK (
        receipt_attempt_payload_sha256 IS NULL
        OR receipt_attempt_payload_sha256 ~ '^[0-9a-f]{64}$'
    ),
    ADD CONSTRAINT submission_receipt_snapshot_canonical_json_version_check CHECK (
        canonical_json_version = 1
    ),
    ADD CONSTRAINT submission_receipt_snapshot_canonical_source_check CHECK (
        (receipt_attempt_canonical_json IS NULL) =
            (receipt_attempt_payload IS NULL)
        AND (run_canonical_json IS NULL) = (run_payload IS NULL)
        AND (summary_canonical_json IS NULL) = (summary_payload IS NULL)
        AND (presentation_canonical_json IS NULL) =
            (presentation_payload IS NULL)
        AND (
            receipt_attempt_canonical_json IS NULL
            OR octet_length(receipt_attempt_canonical_json) BETWEEN 1 AND 524288
        )
        AND (
            run_canonical_json IS NULL
            OR octet_length(run_canonical_json) BETWEEN 1 AND 524288
        )
        AND (
            summary_canonical_json IS NULL
            OR octet_length(summary_canonical_json) BETWEEN 1 AND 524288
        )
        AND (
            presentation_canonical_json IS NULL
            OR octet_length(presentation_canonical_json) BETWEEN 1 AND 524288
        )
        AND (
            receipt_attempt_canonical_json IS NULL
            OR receipt_attempt_canonical_json::jsonb
                IS NOT DISTINCT FROM receipt_attempt_payload
        )
        AND (
            run_canonical_json IS NULL
            OR run_canonical_json::jsonb IS NOT DISTINCT FROM run_payload
        )
        AND (
            summary_canonical_json IS NULL
            OR summary_canonical_json::jsonb IS NOT DISTINCT FROM summary_payload
        )
        AND (
            presentation_canonical_json IS NULL
            OR presentation_canonical_json::jsonb
                IS NOT DISTINCT FROM presentation_payload
        )
        AND (
            receipt_attempt_canonical_json IS NULL
            OR receipt_attempt_payload_sha256 = encode(
                pg_catalog.sha256(
                    convert_to(receipt_attempt_canonical_json, 'UTF8')
                ),
                'hex'
            )
        )
        AND (
            run_canonical_json IS NULL
            OR run_payload_sha256 = encode(
                pg_catalog.sha256(convert_to(run_canonical_json, 'UTF8')),
                'hex'
            )
        )
        AND (
            summary_canonical_json IS NULL
            OR summary_payload_sha256 = encode(
                pg_catalog.sha256(convert_to(summary_canonical_json, 'UTF8')),
                'hex'
            )
        )
        AND (
            presentation_canonical_json IS NULL
            OR presentation_payload_sha256 = encode(
                pg_catalog.sha256(
                    convert_to(presentation_canonical_json, 'UTF8')
                ),
                'hex'
            )
        )
    );

ALTER TABLE public.submission_receipt_snapshot
    ALTER COLUMN receipt_attempt_payload SET NOT NULL,
    ALTER COLUMN receipt_attempt_canonical_json SET NOT NULL,
    ALTER COLUMN receipt_attempt_payload_sha256 SET NOT NULL,
    ALTER COLUMN run_canonical_json SET NOT NULL,
    ALTER COLUMN summary_canonical_json SET NOT NULL;

ALTER TABLE public.attempt_feedback
    ADD COLUMN content_canonical_json text NOT NULL,
    ADD COLUMN content_canonical_json_version smallint NOT NULL,
    ADD CONSTRAINT attempt_feedback_content_canonical_json_check CHECK (
        content_canonical_json_version = 1
        AND octet_length(content_canonical_json) BETWEEN 1 AND 65536
        AND content_sha256 = encode(
            pg_catalog.sha256(convert_to(content_canonical_json, 'UTF8')),
            'hex'
        )
        AND content_canonical_json::jsonb IS NOT DISTINCT FROM
            jsonb_build_array(hint, correct_response, rationale)
    );

-- Schema self-verification. Connected W7b evidence exercises each constraint.
DO $$
DECLARE
    v_entry text;
    v_parts text[];
BEGIN
    IF NOT EXISTS (
        SELECT 1
          FROM pg_catalog.pg_roles
         WHERE rolname = 'ple_accepted_submission_execution_worker'
           AND NOT rolcanlogin
           AND NOT rolinherit
           AND NOT rolsuper
           AND NOT rolcreatedb
           AND NOT rolcreaterole
           AND NOT rolreplication
           AND NOT rolbypassrls
    ) OR EXISTS (
        SELECT 1
          FROM pg_catalog.pg_auth_members AS membership
         WHERE membership.roleid =
               'ple_accepted_submission_execution_worker'::regrole
            OR membership.member =
               'ple_accepted_submission_execution_worker'::regrole
    ) THEN
        RAISE EXCEPTION 'accepted-submission worker role is unsafe';
    END IF;

    IF NOT EXISTS (
        SELECT 1
          FROM pg_catalog.pg_roles
         WHERE rolname = 'ple_accepted_submission_execution_fast_path'
           AND NOT rolcanlogin
           AND NOT rolinherit
           AND NOT rolsuper
           AND NOT rolcreatedb
           AND NOT rolcreaterole
           AND NOT rolreplication
           AND NOT rolbypassrls
    ) OR EXISTS (
        SELECT 1
          FROM pg_catalog.pg_auth_members AS membership
         WHERE membership.roleid =
               'ple_accepted_submission_execution_fast_path'::regrole
            OR membership.member =
               'ple_accepted_submission_execution_fast_path'::regrole
    ) THEN
        RAISE EXCEPTION 'accepted-submission fast-path role is unsafe';
    END IF;

    IF NOT has_schema_privilege(
        'ple_accepted_submission_execution_fast_path',
        'public',
        'USAGE'
    ) OR has_schema_privilege(
        'ple_accepted_submission_execution_fast_path',
        'public',
        'CREATE'
    ) OR NOT EXISTS (
        SELECT 1
          FROM pg_catalog.pg_namespace AS namespace_row
          CROSS JOIN LATERAL pg_catalog.aclexplode(namespace_row.nspacl) AS acl
         WHERE namespace_row.nspname = 'public'
           AND acl.grantee =
               'ple_accepted_submission_execution_fast_path'::regrole
           AND acl.privilege_type = 'USAGE'
           AND NOT acl.is_grantable
    ) OR EXISTS (
        SELECT 1
          FROM pg_catalog.pg_namespace AS namespace_row
          CROSS JOIN LATERAL pg_catalog.aclexplode(namespace_row.nspacl) AS acl
         WHERE namespace_row.nspname = 'public'
           AND acl.grantee =
               'ple_accepted_submission_execution_fast_path'::regrole
           AND (
               acl.privilege_type <> 'USAGE'
               OR acl.is_grantable
           )
    ) THEN
        RAISE EXCEPTION 'accepted-submission fast-path schema authority is unsafe';
    END IF;

    IF EXISTS (
        SELECT 1
          FROM pg_catalog.pg_class AS relation_row
          JOIN pg_catalog.pg_namespace AS namespace_row
            ON namespace_row.oid = relation_row.relnamespace
          CROSS JOIN LATERAL pg_catalog.aclexplode(relation_row.relacl) AS acl
         WHERE namespace_row.nspname = 'public'
           AND acl.grantee =
               'ple_accepted_submission_execution_fast_path'::regrole
    ) OR EXISTS (
        SELECT 1
          FROM pg_catalog.pg_proc AS procedure_row
          JOIN pg_catalog.pg_namespace AS namespace_row
            ON namespace_row.oid = procedure_row.pronamespace
          CROSS JOIN LATERAL pg_catalog.aclexplode(procedure_row.proacl) AS acl
         WHERE namespace_row.nspname = 'public'
           AND acl.grantee =
               'ple_accepted_submission_execution_fast_path'::regrole
    ) THEN
        RAISE EXCEPTION 'accepted-submission fast-path direct authority is unsafe';
    END IF;

    FOREACH v_entry IN ARRAY ARRAY[
        'public.grading_execution:active_worker_id:uuid:f',
        'public.submission_evaluation:automated_result_canonical_json:text:f',
        'public.submission_evaluation:automated_result_sha256:character(64):f',
        'public.submission_evaluation:automated_result_canonical_json_version:smallint:f',
        'public.submission_receipt_snapshot:receipt_attempt_payload:jsonb:t',
        'public.submission_receipt_snapshot:receipt_attempt_canonical_json:text:t',
        'public.submission_receipt_snapshot:receipt_attempt_payload_sha256:character(64):t',
        'public.submission_receipt_snapshot:run_canonical_json:text:t',
        'public.submission_receipt_snapshot:summary_canonical_json:text:t',
        'public.submission_receipt_snapshot:presentation_canonical_json:text:f',
        'public.submission_receipt_snapshot:canonical_json_version:smallint:t',
        'public.attempt_feedback:content_canonical_json:text:t',
        'public.attempt_feedback:content_canonical_json_version:smallint:t'
    ] LOOP
        v_parts := string_to_array(v_entry, ':');
        IF NOT EXISTS (
            SELECT 1
              FROM pg_catalog.pg_attribute AS attribute_row
             WHERE attribute_row.attrelid = v_parts[1]::regclass
               AND attribute_row.attname = v_parts[2]
               AND pg_catalog.format_type(
                   attribute_row.atttypid,
                   attribute_row.atttypmod
               ) = v_parts[3]
               AND attribute_row.attnotnull = v_parts[4]::boolean
               AND NOT attribute_row.attisdropped
               AND NOT EXISTS (
                   SELECT 1
                     FROM pg_catalog.pg_attrdef AS default_row
                    WHERE default_row.adrelid = attribute_row.attrelid
                      AND default_row.adnum = attribute_row.attnum
               )
        ) THEN
            RAISE EXCEPTION 'invalid W4 schema column %', v_entry;
        END IF;
    END LOOP;

    FOREACH v_entry IN ARRAY ARRAY[
        'public.submission_evaluation:submission_evaluation_automated_result_pair_check',
        'public.submission_evaluation:submission_evaluation_automated_result_size_check',
        'public.submission_evaluation:submission_evaluation_automated_result_sha256_check',
        'public.submission_receipt_snapshot:submission_receipt_snapshot_attempt_payload_pair_check',
        'public.submission_receipt_snapshot:' ||
            'submission_receipt_snapshot_attempt_payload_shape_check',
        'public.submission_receipt_snapshot:' ||
            'submission_receipt_snapshot_attempt_payload_sha256_check',
        'public.submission_receipt_snapshot:' ||
            'submission_receipt_snapshot_canonical_json_version_check',
        'public.submission_receipt_snapshot:submission_receipt_snapshot_canonical_source_check',
        'public.attempt_feedback:attempt_feedback_content_canonical_json_check'
    ] LOOP
        v_parts := string_to_array(v_entry, ':');
        IF NOT EXISTS (
            SELECT 1
              FROM pg_catalog.pg_constraint AS constraint_row
             WHERE constraint_row.conrelid = v_parts[1]::regclass
               AND constraint_row.conname = v_parts[2]
               AND constraint_row.contype = 'c'
               AND constraint_row.convalidated
               AND NOT constraint_row.connoinherit
               AND pg_catalog.pg_get_constraintdef(
                   constraint_row.oid,
                   true
               ) LIKE 'CHECK (%'
        ) THEN
            RAISE EXCEPTION 'invalid W4 schema constraint %', v_entry;
        END IF;
    END LOOP;

END $$;

DO $$
BEGIN
    -- No W4 callable grading capability exists at the schema boundary.
    IF to_regprocedure(
        'public.ple_claim_accepted_submission_execution_transition_v1('
        'uuid,uuid,uuid,uuid,uuid,uuid,integer)'
    ) IS NOT NULL OR to_regprocedure(
        'public.ple_claim_accepted_submission_execution_v1(uuid,uuid,integer)'
    ) IS NOT NULL OR to_regprocedure(
        'public.ple_claim_exact_accepted_submission_execution_v1('
        'uuid,uuid,uuid,uuid,uuid,uuid,integer)'
    ) IS NOT NULL OR to_regprocedure(
        'public.ple_read_accepted_submission_evaluation_v1(uuid,uuid,uuid,uuid)'
    ) IS NOT NULL OR to_regprocedure(
        'public.ple_load_accepted_submission_execution_v2('
        'uuid,uuid,uuid,uuid,bigint,uuid)'
    ) IS NOT NULL OR to_regprocedure(
        'public.ple_lock_accepted_submission_completion_v1('
        'uuid,uuid,uuid,uuid,bigint,uuid)'
    ) IS NOT NULL OR to_regprocedure(
        'public.ple_commit_accepted_submission_completion_v2('
        'uuid,uuid,uuid,uuid,bigint,uuid,smallint,text,text,character,text,jsonb,'
        'character,text,character,text,jsonb,character,text,character,bigint,bigint,'
        'uuid,uuid,text,jsonb,character,text,jsonb,character,boolean,uuid,jsonb,bigint,'
        'uuid,integer)'
    ) IS NOT NULL OR to_regprocedure(
        'public.ple_fail_accepted_submission_execution_v1('
        'uuid,uuid,uuid,uuid,bigint,uuid,text,text)'
    ) IS NOT NULL THEN
        RAISE EXCEPTION 'W4 callable capability appeared before its owner migration';
    END IF;
END $$;

COMMIT;
