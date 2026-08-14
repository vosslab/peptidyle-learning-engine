\set ON_ERROR_STOP on

-- Disposable WP-QTI-8 oracle. The entire fixture rolls back.
BEGIN;
SET LOCAL ple.tenant_id = '11111111-1111-4111-8111-0000000000b1';

DO $$
DECLARE
    grading_relation regclass := 'public.workspace_flat_question_grading'::regclass;
    stage_function regprocedure :=
        'public.ple_stage_flat_question_grading(uuid,uuid,bigint,character,uuid,character,character,character,jsonb,character)'::regprocedure;
    promote_function regprocedure :=
        'public.ple_promote_flat_question_grading(uuid,uuid,bigint,character,uuid,character,character,character,uuid,uuid,character)'::regprocedure;
    promotion_arguments text;
BEGIN
    IF NOT EXISTS (
        SELECT 1
          FROM pg_catalog.pg_class
         WHERE oid = grading_relation
           AND relrowsecurity
           AND relforcerowsecurity
    ) THEN
        RAISE EXCEPTION 'current flat grading relation does not force RLS';
    END IF;
    IF NOT EXISTS (
        SELECT 1
          FROM pg_catalog.pg_roles
         WHERE rolname = 'ple_grader'
           AND NOT rolcanlogin
           AND NOT rolbypassrls
    ) THEN
        RAISE EXCEPTION 'flat grading broker role is not least privilege';
    END IF;
    IF NOT pg_catalog.has_function_privilege('ple_app', stage_function, 'EXECUTE')
       OR NOT pg_catalog.has_function_privilege('ple_app', promote_function, 'EXECUTE') THEN
        RAISE EXCEPTION 'application role lacks a protected flat grading capability';
    END IF;
    IF pg_catalog.has_function_privilege('ple_student', stage_function, 'EXECUTE')
       OR pg_catalog.has_function_privilege('ple_student', promote_function, 'EXECUTE')
       OR pg_catalog.has_function_privilege('ple_grading_reader', stage_function, 'EXECUTE')
       OR pg_catalog.has_function_privilege('ple_grading_reader', promote_function, 'EXECUTE') THEN
        RAISE EXCEPTION 'private flat grading write capabilities are broadly executable';
    END IF;
    IF EXISTS (
        SELECT 1
          FROM pg_catalog.pg_proc AS capability
          JOIN pg_catalog.pg_roles AS owner_role
            ON owner_role.oid = capability.proowner
         WHERE capability.oid IN (stage_function::oid, promote_function::oid)
           AND (owner_role.rolname <> 'ple_grader'
                OR NOT capability.prosecdef
                OR NOT capability.proconfig @>
                    ARRAY['search_path=pg_catalog, public, pg_temp']::text[])
    ) OR EXISTS (
        SELECT 1
          FROM pg_catalog.pg_proc AS capability
          CROSS JOIN LATERAL pg_catalog.aclexplode(
              COALESCE(
                  capability.proacl,
                  pg_catalog.acldefault('f', capability.proowner)
              )
          ) AS grant_row
         WHERE capability.oid IN (stage_function::oid, promote_function::oid)
           AND grant_row.grantee = 0
           AND grant_row.privilege_type = 'EXECUTE'
    ) THEN
        RAISE EXCEPTION 'flat grading capability owner, search path, or PUBLIC grant is unsafe';
    END IF;
    IF pg_catalog.has_table_privilege('ple_app', grading_relation, 'SELECT')
       OR pg_catalog.has_table_privilege('ple_app', grading_relation, 'INSERT')
       OR pg_catalog.has_table_privilege('ple_student', grading_relation, 'SELECT')
       OR pg_catalog.has_table_privilege('ple_grading_reader', grading_relation, 'SELECT')
       OR pg_catalog.has_table_privilege('ple_grading_reader', 'public.answer_key', 'SELECT') THEN
        RAISE EXCEPTION 'private flat grading table grants are broader than the capability ABI';
    END IF;
    SELECT pg_catalog.pg_get_function_arguments(promote_function)
      INTO promotion_arguments;
    IF promotion_arguments ~
       'p_grader_payload|p_grader_payload_sha256|p_key_payload|p_key_sha256' THEN
        RAISE EXCEPTION 'publication promotion still accepts caller grading bytes';
    END IF;
END
$$;

INSERT INTO public.workspace_draft
    (tenant_id, workspace_id, payload, payload_sha256, revision)
VALUES
    ('11111111-1111-4111-8111-0000000000b1',
     '11111111-1111-4111-8111-0000000000b2',
     '{}'::jsonb, repeat('a', 64), 1);

INSERT INTO public.workspace_draft_access
    (tenant_id, workspace_id, user_id, role)
VALUES
    ('11111111-1111-4111-8111-0000000000b1',
     '11111111-1111-4111-8111-0000000000b2',
     '11111111-1111-4111-8111-0000000000b3', 'owner');

INSERT INTO public.workspace_flat_question_source
    (tenant_id, workspace_id, draft_revision, draft_payload_sha256,
     source_object_id, source_payload, source_payload_sha256,
     canonical_source_sha256, public_binding_sha256)
VALUES
    ('11111111-1111-4111-8111-0000000000b1',
     '11111111-1111-4111-8111-0000000000b2', 1, repeat('a', 64),
     '11111111-1111-4111-8111-0000000000b4', '{}'::jsonb,
     repeat('b', 64), repeat('c', 64), repeat('d', 64));

-- Only the application capability can stage current private grading.
SET LOCAL ROLE ple_app;
DO $$
DECLARE
    staged boolean;
    key_bytes bytea := pg_catalog.convert_to(repeat('x', 80), 'UTF8');
    key_sha character(64);
    key_payload jsonb;
BEGIN
    key_sha := pg_catalog.encode(pg_catalog.sha256(key_bytes), 'hex');
    key_payload := pg_catalog.jsonb_build_object(
        'publicSha256', repeat('d', 64),
        'payloadSha256', key_sha,
        'payloadBase64', replace(pg_catalog.encode(key_bytes, 'base64'), E'\n', '')
    );
    SELECT public.ple_stage_flat_question_grading(
        '11111111-1111-4111-8111-0000000000b1',
        '11111111-1111-4111-8111-0000000000b2',
        1, repeat('a', 64)::character(64),
        '11111111-1111-4111-8111-0000000000b4',
        repeat('b', 64)::character(64), repeat('c', 64)::character(64),
        repeat('d', 64)::character(64), key_payload, key_sha
    ) INTO staged;
    IF NOT staged THEN
        RAISE EXCEPTION 'initial current flat grading stage was refused';
    END IF;

    SELECT public.ple_stage_flat_question_grading(
        '11111111-1111-4111-8111-0000000000b1',
        '11111111-1111-4111-8111-0000000000b2',
        1, repeat('a', 64)::character(64),
        '11111111-1111-4111-8111-0000000000b4',
        repeat('b', 64)::character(64), repeat('c', 64)::character(64),
        repeat('d', 64)::character(64), key_payload, key_sha
    ) INTO staged;
    IF NOT staged THEN
        RAISE EXCEPTION 'exact current flat grading replay was refused';
    END IF;
END
$$;

DO $$
BEGIN
    BEGIN
        PERFORM 1 FROM public.workspace_flat_question_grading;
        RAISE EXCEPTION 'application role read current flat grading directly';
    EXCEPTION WHEN insufficient_privilege THEN
        NULL;
    END;
END
$$;
RESET ROLE;

-- A valid but divergent replay returns false and leaves the first row intact.
SET LOCAL ROLE ple_app;
DO $$
DECLARE
    staged boolean;
    key_bytes bytea := pg_catalog.convert_to(
        format('{"publicSha256":"%s","answer":"red"}', repeat('2', 64)), 'UTF8'
    );
    key_sha character(64);
    key_payload jsonb;
    stored_bytes bytea := pg_catalog.convert_to(repeat('x', 80), 'UTF8');
    stored_sha character(64);
    stored_payload jsonb;
    invalid_case record;
BEGIN
    key_sha := pg_catalog.encode(pg_catalog.sha256(key_bytes), 'hex');
    key_payload := pg_catalog.jsonb_build_object(
        'publicSha256', repeat('d', 64),
        'payloadSha256', key_sha,
        'payloadBase64', replace(pg_catalog.encode(key_bytes, 'base64'), E'\n', '')
    );
    SELECT public.ple_stage_flat_question_grading(
        '11111111-1111-4111-8111-0000000000b1',
        '11111111-1111-4111-8111-0000000000b2',
        1, repeat('a', 64)::character(64),
        '11111111-1111-4111-8111-0000000000b4',
        repeat('b', 64)::character(64), repeat('c', 64)::character(64),
        repeat('d', 64)::character(64), key_payload, key_sha
    ) INTO staged;
    IF staged THEN
        RAISE EXCEPTION 'divergent current flat grading replay was accepted';
    END IF;

    stored_sha := pg_catalog.encode(pg_catalog.sha256(stored_bytes), 'hex');
    stored_payload := pg_catalog.jsonb_build_object(
        'publicSha256', repeat('d', 64),
        'payloadSha256', stored_sha,
        'payloadBase64', replace(pg_catalog.encode(stored_bytes, 'base64'), E'\n', '')
    );

    -- Every malformed envelope is rejected through the same capability
    -- boundary. `ZE==` specifically exercises unused padding bits, while the
    -- oversized case crosses the decoded 256 KiB ceiling by one byte.
    FOR invalid_case IN
        SELECT *
          FROM (VALUES
              ('extra member', stored_payload || '{"unexpected":true}'::jsonb,
               stored_sha::text),
              ('missing member', stored_payload - 'payloadBase64', stored_sha::text),
              ('empty decoded payload', pg_catalog.jsonb_build_object(
                   'publicSha256', repeat('d', 64),
                   'payloadSha256', pg_catalog.encode(
                       pg_catalog.sha256(''::bytea), 'hex'
                   ), 'payloadBase64', ''
               ), pg_catalog.encode(pg_catalog.sha256(''::bytea), 'hex')),
              ('oversized decoded payload', pg_catalog.jsonb_build_object(
                   'publicSha256', repeat('d', 64),
                   'payloadSha256', pg_catalog.encode(pg_catalog.sha256(
                       pg_catalog.convert_to(repeat('z', 262145), 'UTF8')
                   ), 'hex'),
                   'payloadBase64', replace(pg_catalog.encode(
                       pg_catalog.convert_to(repeat('z', 262145), 'UTF8'), 'base64'
                   ), E'\n', '')
               ), pg_catalog.encode(pg_catalog.sha256(
                   pg_catalog.convert_to(repeat('z', 262145), 'UTF8')
               ), 'hex')),
              ('row checksum mismatch', stored_payload, repeat('e', 64)),
              ('decoded checksum mismatch', pg_catalog.jsonb_build_object(
                   'publicSha256', repeat('d', 64),
                   'payloadSha256', repeat('e', 64),
                   'payloadBase64', replace(
                       pg_catalog.encode(stored_bytes, 'base64'), E'\n', ''
                   )
               ), repeat('e', 64)),
              ('public binding mismatch', pg_catalog.jsonb_build_object(
                   'publicSha256', repeat('e', 64),
                   'payloadSha256', stored_sha,
                   'payloadBase64', replace(
                       pg_catalog.encode(stored_bytes, 'base64'), E'\n', ''
                   )
               ), stored_sha::text),
              ('malformed padding', pg_catalog.jsonb_build_object(
                   'publicSha256', repeat('d', 64),
                   'payloadSha256', repeat('e', 64), 'payloadBase64', 'A==='
               ), repeat('e', 64)),
              ('noncanonical padding bits', pg_catalog.jsonb_build_object(
                   'publicSha256', repeat('d', 64),
                   'payloadSha256', pg_catalog.encode(pg_catalog.sha256(
                       pg_catalog.convert_to('d', 'UTF8')
                   ), 'hex'), 'payloadBase64', 'ZE=='
               ), pg_catalog.encode(pg_catalog.sha256(
                   pg_catalog.convert_to('d', 'UTF8')
               ), 'hex'))
          ) AS invalid_cases(label, payload, key_sha)
    LOOP
        BEGIN
            PERFORM public.ple_stage_flat_question_grading(
                '11111111-1111-4111-8111-0000000000b1',
                '11111111-1111-4111-8111-0000000000b2',
                1, repeat('a', 64)::character(64),
                '11111111-1111-4111-8111-0000000000b4',
                repeat('b', 64)::character(64), repeat('c', 64)::character(64),
                repeat('d', 64)::character(64), invalid_case.payload,
                invalid_case.key_sha::character(64)
            );
            RAISE EXCEPTION '% flat grading envelope was accepted', invalid_case.label;
        EXCEPTION WHEN SQLSTATE '22023' THEN
            IF SQLERRM <> 'invalid flat grading staging capability' THEN
                RAISE EXCEPTION '% leaked a lower-level diagnostic: %',
                    invalid_case.label, SQLERRM;
            END IF;
        END;
    END LOOP;
END
$$;
RESET ROLE;

-- The NOLOGIN grading broker itself cannot enumerate another tenant. Its
-- direct-write trigger also returns one generic envelope error.
SET LOCAL ROLE ple_grader;
SET LOCAL ple.tenant_id = '11111111-1111-4111-8111-0000000000ff';
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM public.workspace_flat_question_grading) THEN
        RAISE EXCEPTION 'flat grading broker enumerated a foreign tenant';
    END IF;
END
$$;
SET LOCAL ple.tenant_id = '11111111-1111-4111-8111-0000000000b1';
DO $$
BEGIN
    BEGIN
        INSERT INTO public.workspace_flat_question_grading
            (tenant_id, workspace_id, draft_revision, draft_payload_sha256,
             source_object_id, source_payload_sha256, canonical_source_sha256,
             public_binding_sha256, key_payload, key_sha256)
        VALUES
            ('11111111-1111-4111-8111-0000000000b1',
             '11111111-1111-4111-8111-0000000000b2', 1, repeat('a', 64),
             '11111111-1111-4111-8111-0000000000b4', repeat('b', 64),
             repeat('c', 64), repeat('d', 64),
             pg_catalog.jsonb_build_object(
                'publicSha256', repeat('d', 64),
                'payloadSha256', repeat('e', 64),
                'payloadBase64', 'A==='
             ), repeat('e', 64));
        RAISE EXCEPTION 'direct broker malformed-padding insert was accepted';
    EXCEPTION WHEN SQLSTATE '22023' THEN
        IF SQLERRM <> 'invalid current flat grading envelope' THEN
            RAISE EXCEPTION 'direct-write trigger leaked a lower-level diagnostic';
        END IF;
    END;

    BEGIN
        INSERT INTO public.workspace_flat_question_grading
            (tenant_id, workspace_id, draft_revision, draft_payload_sha256,
             source_object_id, source_payload_sha256, canonical_source_sha256,
             public_binding_sha256, key_payload, key_sha256)
        VALUES
            ('11111111-1111-4111-8111-0000000000b1',
             '11111111-1111-4111-8111-0000000000b2', 1, repeat('a', 64),
             '11111111-1111-4111-8111-0000000000b4', repeat('b', 64),
             repeat('c', 64), repeat('d', 64),
             pg_catalog.jsonb_build_object(
                 'publicSha256', repeat('d', 64),
                 'payloadSha256', repeat('e', 64),
                 'payloadBase64', 'eA=='
             ), repeat('e', 64));
        RAISE EXCEPTION 'direct broker checksum-mismatched insert was accepted';
    EXCEPTION WHEN SQLSTATE '22023' THEN
        IF SQLERRM <> 'invalid current flat grading envelope' THEN
            RAISE EXCEPTION 'direct checksum rejection leaked a lower-level diagnostic';
        END IF;
    END;
END
$$;
RESET ROLE;

DO $$
DECLARE
    expected_sha text := pg_catalog.encode(
        pg_catalog.sha256(pg_catalog.convert_to(repeat('x', 80), 'UTF8')),
        'hex'
    );
    stored_sha text;
BEGIN
    SELECT key_sha256 INTO stored_sha
      FROM public.workspace_flat_question_grading
     WHERE tenant_id = '11111111-1111-4111-8111-0000000000b1'
       AND workspace_id = '11111111-1111-4111-8111-0000000000b2';
    IF stored_sha <> expected_sha THEN
        RAISE EXCEPTION 'divergent stage mutated current flat grading';
    END IF;
END
$$;

-- An ordinary draft/source edit deletes the source and cascades stale grading.
SET LOCAL ROLE ple_app;
UPDATE public.workspace_draft
   SET payload = '{"edited":true}'::jsonb,
       payload_sha256 = repeat('e', 64),
       revision = 2,
       updated_at = transaction_timestamp()
 WHERE tenant_id = '11111111-1111-4111-8111-0000000000b1'
   AND workspace_id = '11111111-1111-4111-8111-0000000000b2';
RESET ROLE;

DO $$
BEGIN
    IF EXISTS (
        SELECT 1 FROM public.workspace_flat_question_source
         WHERE tenant_id = '11111111-1111-4111-8111-0000000000b1'
           AND workspace_id = '11111111-1111-4111-8111-0000000000b2'
    ) OR EXISTS (
        SELECT 1 FROM public.workspace_flat_question_grading
         WHERE tenant_id = '11111111-1111-4111-8111-0000000000b1'
           AND workspace_id = '11111111-1111-4111-8111-0000000000b2'
    ) THEN
        RAISE EXCEPTION 'ordinary draft edit retained stale source or grading';
    END IF;
END
$$;

SET LOCAL ROLE ple_app;
INSERT INTO public.workspace_flat_question_source
    (tenant_id, workspace_id, draft_revision, draft_payload_sha256,
     source_object_id, source_payload, source_payload_sha256,
     canonical_source_sha256, public_binding_sha256)
VALUES
    ('11111111-1111-4111-8111-0000000000b1',
     '11111111-1111-4111-8111-0000000000b2', 2, repeat('e', 64),
     '11111111-1111-4111-8111-0000000000b5', '{"edited":true}'::jsonb,
     repeat('f', 64), repeat('1', 64), repeat('2', 64));
RESET ROLE;

-- Seed two unpublished-in-this-transaction catalog candidates. One proves
-- missing current grading refuses; the other receives the stored-only copy.
INSERT INTO public.problem
    (problem_id, question_id, owner_tenant_id, owner_user_id, visibility, license)
VALUES
    ('11111111-1111-4111-8111-0000000000b6',
     'K8R4XWA',
     '11111111-1111-4111-8111-0000000000b1',
     '11111111-1111-4111-8111-0000000000b3', 'public', 'cc0'),
    ('11111111-1111-4111-8111-0000000000b8',
     'D5N7Q2M',
     '11111111-1111-4111-8111-0000000000b1',
     '11111111-1111-4111-8111-0000000000b3', 'public', 'cc0'),
    ('11111111-1111-4111-8111-0000000000c6',
     'Z3P8H6F',
     '11111111-1111-4111-8111-0000000000ff',
     '11111111-1111-4111-8111-0000000000b3', 'public', 'cc0');

INSERT INTO public.problem_version
    (problem_id, version_id, version_number, content_sha256, workspace_id,
     title, backend, metadata, publication_scope, authors)
VALUES
    ('11111111-1111-4111-8111-0000000000b6',
     '11111111-1111-4111-8111-0000000000b7', 1, repeat('3', 64),
     '11111111-1111-4111-8111-0000000000b2', 'Absent grading probe', 'native',
     '{"language":"en-US","license":{"kind":"cc0"},"taxonomy":[],"tags":[]}',
     'public', '["11111111-1111-4111-8111-0000000000b3"]'),
    ('11111111-1111-4111-8111-0000000000b8',
     '11111111-1111-4111-8111-0000000000b9', 1, repeat('4', 64),
     '11111111-1111-4111-8111-0000000000b2', 'Stored grading probe', 'native',
     '{"language":"en-US","license":{"kind":"cc0"},"taxonomy":[],"tags":[]}',
     'public', '["11111111-1111-4111-8111-0000000000b3"]'),
    ('11111111-1111-4111-8111-0000000000c6',
     '11111111-1111-4111-8111-0000000000c7', 1, repeat('6', 64),
     '11111111-1111-4111-8111-0000000000b2', 'Foreign owner probe', 'native',
     '{"language":"en-US","license":{"kind":"cc0"},"taxonomy":[],"tags":[]}',
     'public', '["11111111-1111-4111-8111-0000000000b3"]');

INSERT INTO public.problem_version_payload
    (problem_id, version_id, payload, payload_sha256)
VALUES
    ('11111111-1111-4111-8111-0000000000b8',
     '11111111-1111-4111-8111-0000000000b9',
     '{"question":{"source":{"backend":"native","family":"flat_single_choice_v2"}}}',
     repeat('5', 64));

SET LOCAL ROLE ple_app;
DO $$
DECLARE promoted boolean;
BEGIN
    SELECT public.ple_promote_flat_question_grading(
        '11111111-1111-4111-8111-0000000000b1',
        '11111111-1111-4111-8111-0000000000b2',
        2, repeat('e', 64)::character(64),
        '11111111-1111-4111-8111-0000000000b5',
        repeat('f', 64)::character(64), repeat('1', 64)::character(64),
        repeat('2', 64)::character(64),
        '11111111-1111-4111-8111-0000000000b6',
        '11111111-1111-4111-8111-0000000000b7', repeat('2', 64)::character(64)
    ) INTO promoted;
    IF promoted THEN
        RAISE EXCEPTION 'publication without current grading was accepted';
    END IF;
END
$$;
RESET ROLE;

DO $$
BEGIN
    IF EXISTS (
        SELECT 1 FROM public.answer_key
         WHERE problem_id = '11111111-1111-4111-8111-0000000000b6'
           AND version_id = '11111111-1111-4111-8111-0000000000b7'
    ) THEN
        RAISE EXCEPTION 'absent grading refusal mutated answer_key';
    END IF;
END
$$;

SET LOCAL ROLE ple_app;
DO $$
DECLARE
    staged boolean;
    key_bytes bytea := pg_catalog.convert_to(
        format('{"publicSha256":"%s","answer":"red"}', repeat('2', 64)), 'UTF8'
    );
    key_sha character(64);
    key_payload jsonb;
BEGIN
    key_sha := pg_catalog.encode(pg_catalog.sha256(key_bytes), 'hex');
    key_payload := pg_catalog.jsonb_build_object(
        'publicSha256', repeat('2', 64),
        'payloadSha256', key_sha,
        'payloadBase64', replace(pg_catalog.encode(key_bytes, 'base64'), E'\n', '')
    );
    SELECT public.ple_stage_flat_question_grading(
        '11111111-1111-4111-8111-0000000000b1',
        '11111111-1111-4111-8111-0000000000b2',
        2, repeat('e', 64)::character(64),
        '11111111-1111-4111-8111-0000000000b5',
        repeat('f', 64)::character(64), repeat('1', 64)::character(64),
        repeat('2', 64)::character(64), key_payload, key_sha
    ) INTO staged;
    IF NOT staged THEN
        RAISE EXCEPTION 'replacement source grading stage was refused';
    END IF;
END
$$;
RESET ROLE;

-- Stale selectors and a public candidate owned by another tenant both refuse
-- before answer-key insertion, even though current grading exists.
SET LOCAL ROLE ple_app;
DO $$
DECLARE promoted boolean;
BEGIN
    SELECT public.ple_promote_flat_question_grading(
        '11111111-1111-4111-8111-0000000000b1',
        '11111111-1111-4111-8111-0000000000b2',
        1, repeat('a', 64)::character(64),
        '11111111-1111-4111-8111-0000000000b4',
        repeat('b', 64)::character(64), repeat('c', 64)::character(64),
        repeat('d', 64)::character(64),
        '11111111-1111-4111-8111-0000000000b8',
        '11111111-1111-4111-8111-0000000000b9', repeat('2', 64)::character(64)
    ) INTO promoted;
    IF promoted THEN
        RAISE EXCEPTION 'stale grading publication selectors were accepted';
    END IF;

    SELECT public.ple_promote_flat_question_grading(
        '11111111-1111-4111-8111-0000000000b1',
        '11111111-1111-4111-8111-0000000000b2',
        2, repeat('e', 64)::character(64),
        '11111111-1111-4111-8111-0000000000b5',
        repeat('f', 64)::character(64), repeat('1', 64)::character(64),
        repeat('2', 64)::character(64),
        '11111111-1111-4111-8111-0000000000c6',
        '11111111-1111-4111-8111-0000000000c7', repeat('2', 64)::character(64)
    ) INTO promoted;
    IF promoted THEN
        RAISE EXCEPTION 'cross-tenant grading publication was accepted';
    END IF;
END
$$;
RESET ROLE;

DO $$
BEGIN
    IF EXISTS (
        SELECT 1 FROM public.answer_key
         WHERE (problem_id, version_id) IN (
             ('11111111-1111-4111-8111-0000000000b8'::uuid,
              '11111111-1111-4111-8111-0000000000b9'::uuid),
             ('11111111-1111-4111-8111-0000000000c6'::uuid,
              '11111111-1111-4111-8111-0000000000c7'::uuid)
         )
    ) THEN
        RAISE EXCEPTION 'refused publication selector mutated answer_key';
    END IF;
END
$$;

-- Prove transaction rollback removes the answer-key copy, then promote again.
SAVEPOINT before_flat_grading_promotion;
SET LOCAL ROLE ple_app;
DO $$
DECLARE promoted boolean;
BEGIN
    SELECT public.ple_promote_flat_question_grading(
        '11111111-1111-4111-8111-0000000000b1',
        '11111111-1111-4111-8111-0000000000b2',
        2, repeat('e', 64)::character(64),
        '11111111-1111-4111-8111-0000000000b5',
        repeat('f', 64)::character(64), repeat('1', 64)::character(64),
        repeat('2', 64)::character(64),
        '11111111-1111-4111-8111-0000000000b8',
        '11111111-1111-4111-8111-0000000000b9', repeat('3', 64)::character(64)
    ) INTO promoted;
    IF NOT promoted THEN
        RAISE EXCEPTION 'stored-only publication promotion was refused';
    END IF;
END
$$;
RESET ROLE;
ROLLBACK TO SAVEPOINT before_flat_grading_promotion;

DO $$
BEGIN
    IF EXISTS (
        SELECT 1 FROM public.answer_key
         WHERE problem_id = '11111111-1111-4111-8111-0000000000b8'
           AND version_id = '11111111-1111-4111-8111-0000000000b9'
    ) THEN
        RAISE EXCEPTION 'rolled-back promotion retained an answer key';
    END IF;
END
$$;

SET LOCAL ROLE ple_app;
DO $$
DECLARE promoted boolean;
BEGIN
    SELECT public.ple_promote_flat_question_grading(
        '11111111-1111-4111-8111-0000000000b1',
        '11111111-1111-4111-8111-0000000000b2',
        2, repeat('e', 64)::character(64),
        '11111111-1111-4111-8111-0000000000b5',
        repeat('f', 64)::character(64), repeat('1', 64)::character(64),
        repeat('2', 64)::character(64),
        '11111111-1111-4111-8111-0000000000b8',
        '11111111-1111-4111-8111-0000000000b9', repeat('3', 64)::character(64)
    ) INTO promoted;
    IF NOT promoted THEN
        RAISE EXCEPTION 'stored-only publication promotion was refused after rollback';
    END IF;
END
$$;
RESET ROLE;

DO $$
DECLARE
    expected_current_bytes bytea := pg_catalog.convert_to(
        format('{"publicSha256":"%s","answer":"red"}', repeat('2', 64)), 'UTF8'
    );
    expected_published_bytes bytea := pg_catalog.convert_to(
        format('{"publicSha256":"%s","answer":"red"}', repeat('3', 64)), 'UTF8'
    );
    expected_current_sha text := pg_catalog.encode(
        pg_catalog.sha256(expected_current_bytes), 'hex'
    );
    expected_published_sha text := pg_catalog.encode(
        pg_catalog.sha256(expected_published_bytes), 'hex'
    );
    current_payload jsonb;
    current_sha text;
    published_payload jsonb;
    published_sha text;
BEGIN
    SELECT key_payload, key_sha256 INTO current_payload, current_sha
      FROM public.workspace_flat_question_grading
     WHERE tenant_id = '11111111-1111-4111-8111-0000000000b1'
       AND workspace_id = '11111111-1111-4111-8111-0000000000b2';
    SELECT key_payload, key_sha256 INTO published_payload, published_sha
      FROM public.answer_key
     WHERE problem_id = '11111111-1111-4111-8111-0000000000b8'
       AND version_id = '11111111-1111-4111-8111-0000000000b9';
    IF current_sha <> expected_current_sha
       OR current_payload ->> 'publicSha256' <> repeat('2', 64)
       OR pg_catalog.decode(current_payload ->> 'payloadBase64', 'base64')
            <> expected_current_bytes
       OR published_sha <> expected_published_sha
       OR published_payload ->> 'publicSha256' <> repeat('3', 64)
       OR published_payload ->> 'payloadSha256' <> expected_published_sha
       OR pg_catalog.decode(published_payload ->> 'payloadBase64', 'base64')
            <> expected_published_bytes
    THEN
        RAISE EXCEPTION 'publication did not rebind exact grading to its published checksum';
    END IF;

    BEGIN
        UPDATE public.answer_key
           SET key_sha256 = repeat('0', 64)
         WHERE problem_id = '11111111-1111-4111-8111-0000000000b8'
           AND version_id = '11111111-1111-4111-8111-0000000000b9';
        RAISE EXCEPTION 'published answer key update was accepted';
    EXCEPTION WHEN SQLSTATE '55000' THEN
        NULL;
    END;
    BEGIN
        DELETE FROM public.answer_key
         WHERE problem_id = '11111111-1111-4111-8111-0000000000b8'
           AND version_id = '11111111-1111-4111-8111-0000000000b9';
        RAISE EXCEPTION 'published answer key delete was accepted';
    EXCEPTION WHEN SQLSTATE '55000' THEN
        NULL;
    END;
END
$$;

-- Publication cleanup removes current workspace material but not immutable key.
SET LOCAL ROLE ple_app;
DELETE FROM public.workspace_draft
 WHERE tenant_id = '11111111-1111-4111-8111-0000000000b1'
   AND workspace_id = '11111111-1111-4111-8111-0000000000b2';
RESET ROLE;

DO $$
BEGIN
    IF EXISTS (
        SELECT 1 FROM public.workspace_flat_question_grading
         WHERE tenant_id = '11111111-1111-4111-8111-0000000000b1'
           AND workspace_id = '11111111-1111-4111-8111-0000000000b2'
    ) OR NOT EXISTS (
        SELECT 1 FROM public.answer_key
         WHERE problem_id = '11111111-1111-4111-8111-0000000000b8'
           AND version_id = '11111111-1111-4111-8111-0000000000b9'
    ) THEN
        RAISE EXCEPTION 'publication cleanup lost or retained the wrong grading state';
    END IF;
END
$$;

SET LOCAL ROLE ple_student;
DO $$
BEGIN
    BEGIN
        PERFORM 1 FROM public.workspace_flat_question_grading;
        RAISE EXCEPTION 'student role read current flat grading directly';
    EXCEPTION WHEN insufficient_privilege THEN
        NULL;
    END;
END
$$;
RESET ROLE;

SET LOCAL ROLE ple_grading_reader;
DO $$
DECLARE
    expected_sha text := pg_catalog.encode(
        pg_catalog.sha256(pg_catalog.convert_to(
            format('{"publicSha256":"%s","answer":"red"}', repeat('3', 64)),
            'UTF8'
        )),
        'hex'
    );
    capability_sha text;
BEGIN
    BEGIN
        PERFORM 1 FROM public.workspace_flat_question_grading;
        RAISE EXCEPTION 'grading reader read current flat grading directly';
    EXCEPTION WHEN insufficient_privilege THEN
        NULL;
    END;
    BEGIN
        PERFORM 1 FROM public.answer_key;
        RAISE EXCEPTION 'grading reader read answer_key directly';
    EXCEPTION WHEN insufficient_privilege THEN
        NULL;
    END;

    SELECT key_sha256 INTO capability_sha
      FROM public.ple_flat_question_grading_material(
        '11111111-1111-4111-8111-0000000000b1',
        '11111111-1111-4111-8111-0000000000b8',
        '11111111-1111-4111-8111-0000000000b9'
      );
    IF capability_sha <> expected_sha THEN
        RAISE EXCEPTION 'published grader capability did not return rebound grading';
    END IF;
END
$$;
RESET ROLE;

ROLLBACK;
