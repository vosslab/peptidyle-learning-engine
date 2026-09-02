-- Private Question record terminology and integrity oracle for the fresh baseline.
DO $$
DECLARE
    table_name text;
    trigger_name text;
    expected_tables text[] := ARRAY[
        'question_source',
        'draft_question_answer_key',
        'draft_question_feedback',
        'draft_question_answer_explanation',
        'draft_question_grading_input',
        'question_revision_answer_key',
        'question_revision_feedback',
        'question_revision_answer_explanation',
        'question_revision_grading_input',
        'workspace_import',
        'workspace_import_item_result',
        'workspace_import_grading_input'
    ];
BEGIN
    IF to_regclass('ple_private.draft_question_source') IS NOT NULL
        OR to_regclass('ple_private.draft_question_grading_material') IS NOT NULL
        OR to_regclass('ple_private.published_flat_question_grading') IS NOT NULL
        OR to_regclass('ple_private.published_qti_question_grading') IS NOT NULL
        OR to_regclass('ple_private.workspace_import_grading') IS NOT NULL THEN
        RAISE EXCEPTION 'generic private Question grading records remain in the baseline';
    END IF;

    FOREACH table_name IN ARRAY expected_tables LOOP
        IF NOT EXISTS (
            SELECT 1
            FROM pg_class AS relation
            JOIN pg_namespace AS namespace ON namespace.oid = relation.relnamespace
            WHERE namespace.nspname = 'ple_private'
              AND relation.relname = table_name
              AND relation.relrowsecurity
              AND relation.relforcerowsecurity
        ) THEN
            RAISE EXCEPTION 'private Question record % does not enforce RLS', table_name;
        END IF;
        IF NOT EXISTS (
            SELECT 1
            FROM pg_constraint AS table_constraint
            WHERE table_constraint.conrelid = format('ple_private.%I', table_name)::regclass
              AND pg_get_constraintdef(table_constraint.oid) LIKE '%sha256%'
        ) THEN
            RAISE EXCEPTION 'private Question record % lacks a checksum constraint', table_name;
        END IF;
    END LOOP;

    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint AS table_constraint
        WHERE table_constraint.conrelid = 'ple_private.question_source'::regclass
          AND table_constraint.contype = 'f'
          AND table_constraint.confrelid = 'ple_private.draft_question_revision'::regclass
    ) OR NOT EXISTS (
        SELECT 1 FROM pg_constraint AS table_constraint
        WHERE table_constraint.conrelid = 'ple_private.question_source'::regclass
          AND table_constraint.contype = 'f'
          AND table_constraint.confrelid = 'ple_data.question_revision'::regclass
    ) THEN
        RAISE EXCEPTION 'Question Source must be bound to Draft Question Revision or Question Revision';
    END IF;

    IF EXISTS (
        SELECT 1
          FROM information_schema.columns AS column_definition
         WHERE column_definition.table_schema = 'ple_private'
           AND column_definition.table_name = 'question_source'
           AND column_definition.column_name IN ('source_data', 'source_checksum')
    ) OR EXISTS (
        SELECT 1
          FROM information_schema.columns AS column_definition
         WHERE column_definition.table_schema = 'ple_private'
           AND column_definition.table_name = 'question_source'
           AND column_definition.column_name IN ('source_object_id', 'source_object_checksum')
           AND column_definition.is_nullable <> 'NO'
    ) THEN
        RAISE EXCEPTION 'Question Source must use one required Source Object Reference and Source Object Checksum';
    END IF;

    IF NOT EXISTS (
        SELECT 1
          FROM information_schema.columns AS column_definition
         WHERE column_definition.table_schema = 'ple_private'
           AND column_definition.table_name = 'question_source'
           AND column_definition.column_name = 'public_content_checksum'
           AND column_definition.data_type = 'text'
           AND column_definition.is_nullable = 'NO'
    ) OR EXISTS (
        SELECT 1
          FROM information_schema.columns AS column_definition
         WHERE column_definition.table_schema = 'ple_private'
           AND column_definition.table_name IN ('question_source', 'draft_question_answer_key')
           AND column_definition.column_name = 'public_binding_sha256'
    ) OR NOT EXISTS (
        SELECT 1
          FROM information_schema.columns AS column_definition
         WHERE column_definition.table_schema = 'ple_private'
           AND column_definition.table_name = 'draft_question_answer_key'
           AND column_definition.column_name = 'public_content_checksum'
           AND column_definition.data_type = 'text'
           AND column_definition.is_nullable = 'NO'
    ) OR EXISTS (
        SELECT 1
          FROM pg_proc AS procedure
          JOIN pg_namespace AS namespace ON namespace.oid = procedure.pronamespace
         WHERE namespace.nspname IN ('ple_private', 'ple_api')
           AND procedure.proname = 'register_draft_question_source'
           AND pg_get_function_arguments(procedure.oid) LIKE '%p_public_binding_sha256%'
    ) OR (SELECT count(*) FROM pg_proc AS procedure
          JOIN pg_namespace AS namespace ON namespace.oid = procedure.pronamespace
         WHERE namespace.nspname IN ('ple_private', 'ple_api')
           AND procedure.proname = 'register_draft_question_source'
           AND pg_get_function_arguments(procedure.oid) LIKE '%p_public_content_checksum%') <> 2 THEN
        RAISE EXCEPTION 'Question Source public-content checksum storage contract is incomplete';
    END IF;

    IF NOT EXISTS (
        SELECT 1
          FROM pg_class AS relation
          JOIN pg_namespace AS namespace ON namespace.oid = relation.relnamespace
         WHERE namespace.nspname = 'ple_private'
           AND relation.relname = 'object_record'
           AND relation.relrowsecurity
           AND relation.relforcerowsecurity
    ) OR NOT EXISTS (
        SELECT 1
          FROM pg_constraint AS table_constraint
         WHERE table_constraint.conrelid = 'ple_private.question_source'::regclass
           AND table_constraint.conname = 'question_source_object_record_exists'
           AND table_constraint.contype = 'f'
           AND table_constraint.confrelid = 'ple_private.object_record'::regclass
    ) OR NOT EXISTS (
        SELECT 1
          FROM pg_trigger AS trigger
         WHERE trigger.tgrelid = 'ple_private.question_source'::regclass
           AND trigger.tgname = 'question_source_object_record_matches_owner'
           AND NOT trigger.tgisinternal
    ) OR NOT EXISTS (
        SELECT 1
          FROM pg_trigger AS trigger
         WHERE trigger.tgrelid = 'ple_private.object_record'::regclass
           AND trigger.tgname = 'object_record_is_immutable'
           AND NOT trigger.tgisinternal
    ) THEN
        RAISE EXCEPTION 'Question Source Object Reference must name an immutable private Object Record';
    END IF;

    FOREACH table_name IN ARRAY ARRAY[
        'draft_question_answer_key',
        'draft_question_feedback',
        'draft_question_answer_explanation',
        'draft_question_grading_input'
    ] LOOP
        IF NOT EXISTS (
            SELECT 1 FROM pg_constraint AS table_constraint
            WHERE table_constraint.conrelid = format('ple_private.%I', table_name)::regclass
              AND table_constraint.contype = 'f'
              AND table_constraint.confrelid = 'ple_private.draft_question_revision'::regclass
        ) THEN
            RAISE EXCEPTION 'Draft Question record % is not revision-bound', table_name;
        END IF;
    END LOOP;

    FOREACH table_name IN ARRAY ARRAY[
        'question_revision_answer_key',
        'question_revision_feedback',
        'question_revision_answer_explanation',
        'question_revision_grading_input'
    ] LOOP
        IF NOT EXISTS (
            SELECT 1 FROM pg_constraint AS table_constraint
            WHERE table_constraint.conrelid = format('ple_private.%I', table_name)::regclass
              AND table_constraint.contype = 'f'
              AND table_constraint.confrelid = 'ple_data.question_revision'::regclass
        ) THEN
            RAISE EXCEPTION 'Question Revision record % is not revision-bound', table_name;
        END IF;
    END LOOP;

    FOREACH trigger_name IN ARRAY ARRAY[
        'question_source_backend_matches_question_revision',
        'question_source_is_immutable',
        'draft_question_answer_key_is_immutable',
        'draft_question_feedback_is_immutable',
        'draft_question_answer_explanation_is_immutable',
        'draft_question_grading_input_is_immutable',
        'question_revision_answer_key_is_immutable',
        'question_revision_feedback_is_immutable',
        'question_revision_answer_explanation_is_immutable',
        'question_revision_grading_input_is_immutable',
        'workspace_import_grading_input_is_immutable_after_commit',
        'workspace_import_item_result_is_immutable_after_commit'
    ] LOOP
        IF NOT EXISTS (
            SELECT 1 FROM pg_trigger
            WHERE tgname = trigger_name AND NOT tgisinternal
        ) THEN
            RAISE EXCEPTION 'private Question record immutability trigger % is missing', trigger_name;
        END IF;
    END LOOP;

    IF EXISTS (
        SELECT 1
          FROM information_schema.columns AS column_definition
         WHERE column_definition.table_schema = 'ple_private'
           AND column_definition.table_name = 'question_source'
           AND column_definition.column_name IN (
                'question_generator_id',
                'question_generator_version'
           )
    ) THEN
        RAISE EXCEPTION
            'Question Source retains an independent generator identity beside its immutable source bytes';
    END IF;

    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint AS table_constraint
        WHERE table_constraint.conrelid = 'ple_private.workspace_import_grading_input'::regclass
          AND table_constraint.contype = 'f'
          AND table_constraint.confrelid = 'ple_private.workspace_import'::regclass
    ) THEN
        RAISE EXCEPTION 'Workspace Import Question Grading Input is not import-bound';
    END IF;

    IF NOT EXISTS (
        SELECT 1
        FROM information_schema.columns AS column_definition
        WHERE column_definition.table_schema = 'ple_private'
          AND column_definition.table_name = 'workspace_import'
          AND column_definition.column_name = 'format_import_data'
          AND column_definition.data_type = 'jsonb'
          AND column_definition.is_nullable = 'NO'
    ) OR EXISTS (
        SELECT 1
        FROM information_schema.columns AS column_definition
        WHERE column_definition.table_schema = 'ple_private'
          AND column_definition.table_name = 'workspace_import'
          AND column_definition.column_name IN ('source_package_evidence', 'registry')
    ) THEN
        RAISE EXCEPTION 'Workspace Import must retain format-owned import data, not QTI-only fields';
    END IF;

    IF NOT EXISTS (
        SELECT 1
        FROM information_schema.columns AS column_definition
        WHERE column_definition.table_schema = 'ple_private'
          AND column_definition.table_name = 'workspace_import'
          AND column_definition.column_name = 'question_format'
          AND column_definition.is_nullable = 'NO'
    ) OR NOT EXISTS (
        SELECT 1 FROM pg_constraint AS table_constraint
        WHERE table_constraint.conrelid = 'ple_private.workspace_import_item_result'::regclass
          AND table_constraint.contype = 'f'
          AND table_constraint.confrelid = 'ple_private.workspace_import'::regclass
    ) THEN
        RAISE EXCEPTION 'Workspace Import Item Result must bind one Question Format-owned import';
    END IF;
END
$$;

-- The Object Record writer accepts only the caller's exact workspace source
-- address.  The staged-database oracle exercises the capability as ple_app;
-- it does not grant that role ambient private-table access.
BEGIN;

INSERT INTO ple_private.account (account_id, role, created_at)
VALUES ('00000000-0000-0000-0000-000000000901', 'instructor', '2026-08-31T00:00:00Z');
INSERT INTO ple_private.authoring_workspace (
    workspace_id, owner_account_id, created_at
) VALUES (
    '00000000-0000-0000-0000-000000000902',
    '00000000-0000-0000-0000-000000000901',
    '2026-08-31T00:00:00Z'
);

SET LOCAL ROLE ple_app;
SELECT pg_catalog.set_config(
    'ple.session_account_id', '00000000-0000-0000-0000-000000000901', true
);
SELECT ple_api.register_workspace_question_source_object(
    '00000000-0000-0000-0000-000000000902',
    '00000000-0000-0000-0000-000000000903',
    jsonb_build_object(
        'kind', 'workspaceQuestionSource',
        'workspace', '00000000-0000-0000-0000-000000000902'::uuid,
        'object', '00000000-0000-0000-0000-000000000903'::uuid
    ),
    decode(repeat('ab', 32), 'hex'), 17, 'application/json', 1777603200000
);
-- A retry after a bytes-first write crash accepts only the identical record.
SELECT ple_api.register_workspace_question_source_object(
    '00000000-0000-0000-0000-000000000902',
    '00000000-0000-0000-0000-000000000903',
    jsonb_build_object(
        'kind', 'workspaceQuestionSource',
        'workspace', '00000000-0000-0000-0000-000000000902'::uuid,
        'object', '00000000-0000-0000-0000-000000000903'::uuid
    ),
    decode(repeat('ab', 32), 'hex'), 17, 'application/json', 1777603200000
);
DO $$
BEGIN
    PERFORM ple_api.register_workspace_question_source_object(
        '00000000-0000-0000-0000-000000000902',
        '00000000-0000-0000-0000-000000000904',
        jsonb_build_object(
            'kind', 'temporary',
            'object', '00000000-0000-0000-0000-000000000904'::uuid
        ),
        decode(repeat('cd', 32), 'hex'), 17, 'application/json', 1777603200000
    );
    RAISE EXCEPTION 'Workspace Question Source Object capability accepted a mismatched address';
EXCEPTION
    WHEN invalid_parameter_value THEN NULL;
END
$$;

COMMIT;

DO $$
BEGIN
    SET LOCAL ROLE ple_api_owner;
    PERFORM ple_private.transfer_draft_question_source_to_question_revision(
        '00000000-0000-0000-0000-000000000913',
        '00000000-0000-0000-0000-000000000905', 2,
        'SRC-0001', 1, NULL,
        '00000000-0000-0000-0000-000000000914',
        jsonb_build_object(
            'kind', 'questionSource',
            'questionRevision', jsonb_build_object('questionId', 'SRC-0001', 'revisionNumber', 1),
            'object', '00000000-0000-0000-0000-000000000914'::uuid
        ),
        decode(repeat('ab', 32), 'hex'), 17, 'application/json', 1777603200000
    );
    RAISE EXCEPTION 'Question Source publication accepted a nonexistent Draft Question Revision Number';
EXCEPTION
    WHEN check_violation THEN NULL;
END
$$;

-- QTI keeps its Workspace Import ID while the Question is a draft. Publication
-- transfers the exact QTI package item identifier but never that private ID.
INSERT INTO ple_private.workspace_import (
    workspace_id, import_id, question_format, format_import_data,
    format_import_data_sha256, item_registry, item_registry_sha256,
    grading_input_sha256, state, staged_at
) VALUES (
    '00000000-0000-0000-0000-000000000902',
    '00000000-0000-0000-0000-000000000915', 'qti', '{}'::jsonb,
    repeat('a1', 32), '{}'::jsonb, repeat('b2', 32), repeat('c3', 32),
    'staged', '2026-08-31T00:00:00Z'
);
INSERT INTO ple_private.draft_question (draft_question_uuid, workspace_id, created_at)
VALUES (
    '00000000-0000-0000-0000-000000000916',
    '00000000-0000-0000-0000-000000000902', '2026-08-31T00:00:00Z'
);
INSERT INTO ple_private.draft_question_revision (
    draft_question_revision_uuid, draft_question_uuid, revision_number, title,
    question_content, created_at
) VALUES (
    '00000000-0000-0000-0000-000000000917',
    '00000000-0000-0000-0000-000000000916', 1, 'QTI source', '{}'::jsonb,
    '2026-08-31T00:00:00Z'
);
BEGIN;
SET LOCAL ROLE ple_app;
SELECT pg_catalog.set_config(
    'ple.session_account_id', '00000000-0000-0000-0000-000000000901', true
);
SELECT ple_api.register_workspace_question_source_object(
    '00000000-0000-0000-0000-000000000902',
    '00000000-0000-0000-0000-000000000918',
    jsonb_build_object(
        'kind', 'workspaceQuestionSource',
        'workspace', '00000000-0000-0000-0000-000000000902'::uuid,
        'object', '00000000-0000-0000-0000-000000000918'::uuid
    ),
    decode(repeat('d4', 32), 'hex'), 17, 'application/xml', 1777603200000
);
SELECT ple_api.register_draft_question_source(
    '00000000-0000-0000-0000-000000000919',
    '00000000-0000-0000-0000-000000000916', 1,
    '00000000-0000-0000-0000-000000000902',
    'qti', 'qti', 'multipleChoice',
    NULL, 'qti-item-17', '00000000-0000-0000-0000-000000000915', NULL, NULL, NULL,
    '00000000-0000-0000-0000-000000000918', repeat('d4', 32), repeat('e5', 32)
);
DO $$
BEGIN
    PERFORM ple_api.register_draft_question_source(
        '00000000-0000-0000-0000-000000000919',
        '00000000-0000-0000-0000-000000000916', 1,
        '00000000-0000-0000-0000-000000000902',
        'qti', 'qti', 'multipleChoice',
        NULL, 'qti-item-17', '00000000-0000-0000-0000-000000000915', NULL, NULL, 'profile-v1',
        '00000000-0000-0000-0000-000000000918', repeat('d4', 32), repeat('e5', 32)
    );
    RAISE EXCEPTION 'Question Source registration accepted an iMathAS Profile for QTI';
EXCEPTION
    WHEN invalid_parameter_value THEN NULL;
END
$$;
COMMIT;
INSERT INTO ple_data.published_question (question_id, created_at)
VALUES ('QTX-0001', '2026-08-31T00:00:00Z');
INSERT INTO ple_data.question_revision (
    question_id, revision_number, backend, published_at, public_metadata
) VALUES (
    'QTX-0001', 1, 'qti', '2026-08-31T00:00:00Z',
    jsonb_build_object('questionDescription', 'Published QTI source')
);
BEGIN;
SET LOCAL ROLE ple_api_owner;
SELECT ple_private.transfer_draft_question_source_to_question_revision(
    '00000000-0000-0000-0000-000000000920',
    '00000000-0000-0000-0000-000000000916', 1,
    'QTX-0001', 1, NULL,
    '00000000-0000-0000-0000-000000000921',
    jsonb_build_object(
        'kind', 'questionSource',
        'questionRevision', jsonb_build_object('questionId', 'QTX-0001', 'revisionNumber', 1),
        'object', '00000000-0000-0000-0000-000000000921'::uuid
    ),
    decode(repeat('d4', 32), 'hex'), 17, 'application/xml', 1777603200000
);
COMMIT;
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM ple_private.question_source AS source
         WHERE source.question_id = 'QTX-0001'
           AND source.revision_number = 1
           AND source.qti_package_item_identifier = 'qti-item-17'
           AND source.workspace_import_id IS NULL
    ) THEN
        RAISE EXCEPTION 'Question Revision Source retained a draft-only Workspace Import ID';
    END IF;
END
$$;

-- iMathAS keeps exact Deployment and Item References on the draft while the
-- iMathAS Profile is pinned only at publication.
INSERT INTO ple_private.draft_question (draft_question_uuid, workspace_id, created_at)
VALUES (
    '00000000-0000-0000-0000-000000000926',
    '00000000-0000-0000-0000-000000000902', '2026-08-31T00:00:00Z'
);
INSERT INTO ple_private.draft_question_revision (
    draft_question_revision_uuid, draft_question_uuid, revision_number, title,
    question_content, created_at
) VALUES (
    '00000000-0000-0000-0000-000000000927',
    '00000000-0000-0000-0000-000000000926', 1, 'iMathAS source', '{}'::jsonb,
    '2026-08-31T00:00:00Z'
);
BEGIN;
SET LOCAL ROLE ple_app;
SELECT pg_catalog.set_config(
    'ple.session_account_id', '00000000-0000-0000-0000-000000000901', true
);
SELECT ple_api.register_workspace_question_source_object(
    '00000000-0000-0000-0000-000000000902',
    '00000000-0000-0000-0000-000000000928',
    jsonb_build_object(
        'kind', 'workspaceQuestionSource',
        'workspace', '00000000-0000-0000-0000-000000000902'::uuid,
        'object', '00000000-0000-0000-0000-000000000928'::uuid
    ),
    decode(repeat('f6', 32), 'hex'), 17, 'application/json', 1777603200000
);
SELECT ple_api.register_draft_question_source(
    '00000000-0000-0000-0000-000000000929',
    '00000000-0000-0000-0000-000000000926', 1,
    '00000000-0000-0000-0000-000000000902',
    'imathas', 'imathas', 'multipleChoice',
    NULL, NULL, NULL, 'deployment-17', 'item-17', NULL,
    '00000000-0000-0000-0000-000000000928', repeat('f6', 32), repeat('a7', 32)
);
COMMIT;
INSERT INTO ple_data.published_question (question_id, created_at)
VALUES ('MTH-0001', '2026-08-31T00:00:00Z');
INSERT INTO ple_data.question_revision (
    question_id, revision_number, backend, published_at, public_metadata
) VALUES (
    'MTH-0001', 1, 'imathas', '2026-08-31T00:00:00Z',
    jsonb_build_object('questionDescription', 'Published iMathAS source')
);
DO $$
BEGIN
    SET LOCAL ROLE ple_api_owner;
    PERFORM ple_private.transfer_draft_question_source_to_question_revision(
        '00000000-0000-0000-0000-000000000930',
        '00000000-0000-0000-0000-000000000926', 1,
        'MTH-0001', 1, NULL,
        '00000000-0000-0000-0000-000000000931',
        jsonb_build_object(
            'kind', 'questionSource',
            'questionRevision', jsonb_build_object('questionId', 'MTH-0001', 'revisionNumber', 1),
            'object', '00000000-0000-0000-0000-000000000931'::uuid
        ),
        decode(repeat('f6', 32), 'hex'), 17, 'application/json', 1777603200000
    );
    RAISE EXCEPTION 'iMathAS Question Source publication accepted a missing iMathAS Profile';
EXCEPTION
    WHEN invalid_parameter_value THEN NULL;
END
$$;
BEGIN;
SET LOCAL ROLE ple_api_owner;
SELECT ple_private.transfer_draft_question_source_to_question_revision(
    '00000000-0000-0000-0000-000000000930',
    '00000000-0000-0000-0000-000000000926', 1,
    'MTH-0001', 1, 'imathas-profile-v1',
    '00000000-0000-0000-0000-000000000931',
    jsonb_build_object(
        'kind', 'questionSource',
        'questionRevision', jsonb_build_object('questionId', 'MTH-0001', 'revisionNumber', 1),
        'object', '00000000-0000-0000-0000-000000000931'::uuid
    ),
    decode(repeat('f6', 32), 'hex'), 17, 'application/json', 1777603200000
);
COMMIT;
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM ple_private.question_source AS source
         WHERE source.question_id = 'MTH-0001'
           AND source.revision_number = 1
           AND source.imathas_deployment_reference = 'deployment-17'
           AND source.imathas_item_reference = 'item-17'
           AND source.imathas_profile = 'imathas-profile-v1'
           AND source.workspace_import_id IS NULL
    ) THEN
        RAISE EXCEPTION 'Question Revision Source did not preserve the exact pinned iMathAS fields';
    END IF;
END
$$;

DO $$
BEGIN
    IF (SELECT count(*) FROM ple_private.object_record
        WHERE object_id = '00000000-0000-0000-0000-000000000903') <> 1 THEN
        RAISE EXCEPTION 'Workspace Question Source Object registration did not persist exactly one immutable Object Record';
    END IF;
END
$$;

INSERT INTO ple_private.draft_question (draft_question_uuid, workspace_id, created_at)
VALUES (
    '00000000-0000-0000-0000-000000000905',
    '00000000-0000-0000-0000-000000000902',
    '2026-08-31T00:00:00Z'
);
INSERT INTO ple_private.draft_question_revision (
    draft_question_revision_uuid, draft_question_uuid, revision_number, title, question_content, created_at
) VALUES (
    '00000000-0000-0000-0000-000000000906',
    '00000000-0000-0000-0000-000000000905',
    1, 'Object-backed source', '{}'::jsonb, '2026-08-31T00:00:00Z'
);
BEGIN;
SET LOCAL ROLE ple_app;
SELECT pg_catalog.set_config(
    'ple.session_account_id', '00000000-0000-0000-0000-000000000901', true
);
SELECT ple_api.register_draft_question_source(
    '00000000-0000-0000-0000-000000000907',
    '00000000-0000-0000-0000-000000000905', 1,
    '00000000-0000-0000-0000-000000000902',
    'ple', 'pleQuestionJson', 'multipleChoice',
    NULL, NULL, NULL, NULL, NULL, NULL,
    '00000000-0000-0000-0000-000000000903', repeat('ab', 32), repeat('ef', 32)
);
-- A retry mints no second record and returns the established source identity.
SELECT ple_api.register_draft_question_source(
    '00000000-0000-0000-0000-000000000908',
    '00000000-0000-0000-0000-000000000905', 1,
    '00000000-0000-0000-0000-000000000902',
    'ple', 'pleQuestionJson', 'multipleChoice',
    NULL, NULL, NULL, NULL, NULL, NULL,
    '00000000-0000-0000-0000-000000000903', repeat('ab', 32), repeat('ef', 32)
);
DO $$
BEGIN
    PERFORM ple_api.register_draft_question_source(
        '00000000-0000-0000-0000-000000000908',
        '00000000-0000-0000-0000-000000000905', 1,
        '00000000-0000-0000-0000-000000000902',
        'ple', 'pleQuestionJson', 'multipleChoice',
        NULL, NULL, NULL, NULL, NULL, NULL,
        '00000000-0000-0000-0000-000000000903', repeat('ab', 32), repeat('cd', 32)
    );
    RAISE EXCEPTION 'Draft Question Source registration accepted different immutable facts';
EXCEPTION
    WHEN unique_violation THEN NULL;
END
$$;
-- The authorized registration boundary rejects a missing backend-owned field
-- before any immutable source record is created.
DO $$
BEGIN
    PERFORM ple_api.register_draft_question_source(
        '00000000-0000-0000-0000-000000000908',
        '00000000-0000-0000-0000-000000000905', 1,
        '00000000-0000-0000-0000-000000000902',
        'webwork', 'webworkPg', 'multipleChoice',
        NULL, NULL, NULL, NULL, NULL, NULL,
        '00000000-0000-0000-0000-000000000903', repeat('ab', 32), repeat('ef', 32)
    );
    RAISE EXCEPTION 'Draft Question Source registration accepted a WeBWorK Question without its WeBWorK PG Path';
EXCEPTION
    WHEN invalid_parameter_value THEN NULL;
END
$$;
DO $$
BEGIN
    PERFORM ple_api.register_draft_question_source(
        '00000000-0000-0000-0000-000000000908',
        '00000000-0000-0000-0000-000000000905', 2,
        '00000000-0000-0000-0000-000000000902',
        'ple', 'pleQuestionJson', 'multipleChoice',
        NULL, NULL, NULL, NULL, NULL, NULL,
        '00000000-0000-0000-0000-000000000903', repeat('ab', 32), repeat('ef', 32)
    );
    RAISE EXCEPTION 'Draft Question Source registration accepted a nonexistent Draft Question Revision Number';
EXCEPTION
    WHEN check_violation THEN NULL;
END
$$;
DO $$
BEGIN
    PERFORM ple_api.register_draft_question_source(
        '00000000-0000-0000-0000-000000000908',
        '00000000-0000-0000-0000-000000000905', 1,
        '00000000-0000-0000-0000-000000000909',
        'ple', 'pleQuestionJson', 'multipleChoice',
        NULL, NULL, NULL, NULL, NULL, NULL,
        '00000000-0000-0000-0000-000000000903', repeat('ab', 32), repeat('ef', 32)
    );
    RAISE EXCEPTION 'Draft Question Source registration accepted an unauthorized workspace';
EXCEPTION
    WHEN insufficient_privilege THEN NULL;
END
$$;
COMMIT;

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1
         FROM ple_private.question_source
         WHERE question_source_uuid = '00000000-0000-0000-0000-000000000907'
           AND backend = 'ple'
           AND webwork_pg_path IS NULL
           AND qti_package_item_identifier IS NULL
           AND workspace_import_id IS NULL
           AND imathas_deployment_reference IS NULL
           AND imathas_item_reference IS NULL
           AND imathas_profile IS NULL
           AND source_object_id = '00000000-0000-0000-0000-000000000903'
           AND source_object_checksum = repeat('ab', 32)
    ) THEN
        RAISE EXCEPTION 'Question Source did not retain its exact Source Object Reference and Source Object Checksum';
    END IF;
END
$$;

INSERT INTO ple_data.published_question (question_id, created_at)
VALUES ('SRC-0001', '2026-08-31T00:00:00Z');
INSERT INTO ple_data.question_revision (
    question_id, revision_number, backend, published_at, public_metadata
) VALUES (
    'SRC-0001', 1, 'ple', '2026-08-31T00:00:00Z',
    jsonb_build_object('questionDescription', 'Published object-backed source')
);

-- The publication coordinator records a new Revision-owned Object Record and
-- source relationship only after the identical bytes have been copied.
BEGIN;
SET LOCAL ROLE ple_api_owner;
SELECT ple_private.transfer_draft_question_source_to_question_revision(
    '00000000-0000-0000-0000-000000000910',
    '00000000-0000-0000-0000-000000000905', 1,
    'SRC-0001', 1, NULL,
    '00000000-0000-0000-0000-000000000911',
    jsonb_build_object(
        'kind', 'questionSource',
        'questionRevision', jsonb_build_object('questionId', 'SRC-0001', 'revisionNumber', 1),
        'object', '00000000-0000-0000-0000-000000000911'::uuid
    ),
    decode(repeat('ab', 32), 'hex'), 17, 'application/json', 1777603200000
);
-- A retry after copied-byte registration returns the established source identity.
SELECT ple_private.transfer_draft_question_source_to_question_revision(
    '00000000-0000-0000-0000-000000000912',
    '00000000-0000-0000-0000-000000000905', 1,
    'SRC-0001', 1, NULL,
    '00000000-0000-0000-0000-000000000911',
    jsonb_build_object(
        'kind', 'questionSource',
        'questionRevision', jsonb_build_object('questionId', 'SRC-0001', 'revisionNumber', 1),
        'object', '00000000-0000-0000-0000-000000000911'::uuid
    ),
    decode(repeat('ab', 32), 'hex'), 17, 'application/json', 1777603200000
);
COMMIT;

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1
          FROM ple_private.question_source AS source
          JOIN ple_private.object_record AS object_record
            ON object_record.object_id = source.source_object_id
         WHERE source.question_id = 'SRC-0001'
           AND source.revision_number = 1
           AND source.question_source_uuid = '00000000-0000-0000-0000-000000000910'
           AND source.source_object_checksum = repeat('ab', 32)
           AND object_record.object_data_class = 'question-source'
           AND object_record.object_address = jsonb_build_object(
                'kind', 'questionSource',
                'questionRevision', jsonb_build_object('questionId', 'SRC-0001', 'revisionNumber', 1),
                'object', '00000000-0000-0000-0000-000000000911'::uuid
           )
    ) THEN
        RAISE EXCEPTION 'Question Revision publication did not transfer exact Question Source bytes to its exact Object Address';
    END IF;
END
$$;

DO $$
BEGIN
    SET LOCAL ROLE ple_app;
    PERFORM ple_private.transfer_draft_question_source_to_question_revision(
        '00000000-0000-0000-0000-000000000913',
        '00000000-0000-0000-0000-000000000905', 1,
        'SRC-0001', 1, NULL,
        '00000000-0000-0000-0000-000000000914',
        jsonb_build_object(
            'kind', 'questionSource',
            'questionRevision', jsonb_build_object('questionId', 'SRC-0001', 'revisionNumber', 1),
            'object', '00000000-0000-0000-0000-000000000914'::uuid
        ),
        decode(repeat('ab', 32), 'hex'), 17, 'application/json', 1777603200000
    );
    RAISE EXCEPTION 'Question Source publication helper was exposed to ple_app';
EXCEPTION
    WHEN insufficient_privilege THEN NULL;
END
$$;
