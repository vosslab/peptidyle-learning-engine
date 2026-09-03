-- Mutable Draft Question and immutable Question Revision record oracle.
DO $$
BEGIN
    IF to_regclass('ple_private.draft_question_revision') IS NOT NULL
       OR EXISTS (
           SELECT 1 FROM information_schema.columns
            WHERE table_schema = 'ple_private' AND column_name LIKE 'draft_question_revision%'
       ) THEN
        RAISE EXCEPTION 'Draft Question Revision table or column remains';
    END IF;
    IF NOT EXISTS (
        SELECT 1 FROM information_schema.columns
         WHERE table_schema = 'ple_private' AND table_name = 'draft_question'
           AND column_name = 'draft_question_edit_number' AND data_type = 'bigint'
           AND is_nullable = 'NO'
    ) OR NOT EXISTS (
        SELECT 1 FROM pg_constraint
         WHERE conrelid = 'ple_private.question_source_registration'::regclass
           AND confrelid = 'ple_private.draft_question'::regclass
    ) OR EXISTS (
        SELECT 1 FROM pg_constraint
         WHERE conrelid = 'ple_private.question_source_registration'::regclass
           AND confrelid::regclass::text = 'ple_private.draft_question_revision'
    ) THEN
        RAISE EXCEPTION 'Draft Question ownership or Edit Number contract is incomplete';
    END IF;
    IF EXISTS (
        SELECT 1 FROM pg_class
        WHERE oid = ANY (ARRAY[
            to_regclass('ple_private.draft_question_answer_key'),
            to_regclass('ple_private.draft_question_feedback'),
            to_regclass('ple_private.draft_question_answer_explanation'),
            to_regclass('ple_private.draft_question_grading_input'),
            to_regclass('ple_private.question_revision_answer_key'),
            to_regclass('ple_private.question_revision_feedback'),
            to_regclass('ple_private.question_revision_answer_explanation'),
            to_regclass('ple_private.question_revision_grading_input'),
            to_regclass('ple_private.workspace_import_grading_input')
        ])
    ) THEN
        RAISE EXCEPTION 'Retired generic Question Source sidecar table remains';
    END IF;
    IF to_regprocedure('ple_private.copy_draft_question_source_registration_to_question_revision(uuid,integer,text,integer,text,uuid,jsonb,bytea,bigint,text,bigint)') IS NOT NULL THEN
        RAISE EXCEPTION 'unmounted Draft Question Source Registration copy helper remains';
    END IF;
END
$$;

INSERT INTO ple_private.account (account_id, product_role, created_at)
VALUES ('00000000-0000-0000-0000-000000000901', 'instructor', '2026-08-31T00:00:00Z');
INSERT INTO ple_private.authoring_workspace (workspace_id, owner_account_id, created_at)
VALUES ('00000000-0000-0000-0000-000000000902', '00000000-0000-0000-0000-000000000901', '2026-08-31T00:00:00Z');
INSERT INTO ple_private.draft_question (
    draft_question_uuid, workspace_id, draft_question_edit_number, title,
    question_content, created_at, updated_at
) VALUES (
    '00000000-0000-0000-0000-000000000905',
    '00000000-0000-0000-0000-000000000902', 1, 'Object-backed source',
    '{}'::jsonb, '2026-08-31T00:00:00Z', '2026-08-31T00:00:00Z'
);

BEGIN;
SET LOCAL ROLE ple_app;
SELECT pg_catalog.set_config('ple.session_account_id', '00000000-0000-0000-0000-000000000901', true);
SELECT ple_api.register_workspace_question_source_object(
    '00000000-0000-0000-0000-000000000902',
    '00000000-0000-0000-0000-000000000903',
    jsonb_build_object('kind', 'workspaceQuestionSource',
        'workspace', '00000000-0000-0000-0000-000000000902'::uuid,
        'object', '00000000-0000-0000-0000-000000000903'::uuid),
    decode(repeat('ab', 32), 'hex'), 17, 'application/json', 1777603200000
);
SELECT ple_api.register_draft_question_source_registration(
    '00000000-0000-0000-0000-000000000905', 1,
    '00000000-0000-0000-0000-000000000902', 'ple', 'pleQuestionJson',
    NULL, NULL, NULL, NULL, NULL, NULL,
    '00000000-0000-0000-0000-000000000903', repeat('ab', 32)
);
-- Exact prior request retries after its one accepted edit; current matching
-- facts are also a no-op.
SELECT ple_api.register_draft_question_source_registration(
    '00000000-0000-0000-0000-000000000905', 1,
    '00000000-0000-0000-0000-000000000902', 'ple', 'pleQuestionJson',
    NULL, NULL, NULL, NULL, NULL, NULL,
    '00000000-0000-0000-0000-000000000903', repeat('ab', 32)
);
SELECT ple_api.register_draft_question_source_registration(
    '00000000-0000-0000-0000-000000000905', 2,
    '00000000-0000-0000-0000-000000000902', 'ple', 'pleQuestionJson',
    NULL, NULL, NULL, NULL, NULL, NULL,
    '00000000-0000-0000-0000-000000000903', repeat('ab', 32)
);
DO $$
BEGIN
    PERFORM ple_api.register_draft_question_source_registration(
        '00000000-0000-0000-0000-000000000905', 1,
        '00000000-0000-0000-0000-000000000902', 'webwork', 'webworkPg',
        'set/object-backed.pg', NULL, NULL, NULL, NULL, NULL,
        '00000000-0000-0000-0000-000000000903', repeat('ab', 32));
    RAISE EXCEPTION 'Draft Question Source Registration accepted stale mismatched facts';
EXCEPTION WHEN serialization_failure THEN NULL;
END $$;
SELECT ple_api.register_draft_question_source_registration(
    '00000000-0000-0000-0000-000000000905', 2,
    '00000000-0000-0000-0000-000000000902', 'webwork', 'webworkPg',
    'set/object-backed.pg', NULL, NULL, NULL, NULL, NULL,
    '00000000-0000-0000-0000-000000000903', repeat('ab', 32)
);
DO $$
BEGIN
    PERFORM ple_api.register_draft_question_source_registration(
        '00000000-0000-0000-0000-000000000905', 3,
        '00000000-0000-0000-0000-000000000909', 'ple', 'pleQuestionJson',
        NULL, NULL, NULL, NULL, NULL, NULL,
        '00000000-0000-0000-0000-000000000903', repeat('ab', 32));
    RAISE EXCEPTION 'Draft Question Source Registration accepted an unauthorized workspace';
EXCEPTION WHEN insufficient_privilege THEN NULL;
END $$;
COMMIT;

DO $$
BEGIN
    IF (SELECT draft_question_edit_number FROM ple_private.draft_question
         WHERE draft_question_uuid = '00000000-0000-0000-0000-000000000905') <> 3
       OR (SELECT count(*) FROM ple_private.question_source_registration
            WHERE draft_question_uuid = '00000000-0000-0000-0000-000000000905') <> 1 THEN
        RAISE EXCEPTION 'Draft Question Source Registration CAS did not apply exactly once per changed facts';
    END IF;
END $$;

-- Published fixtures use direct immutable records; publication workflow remains unmounted.
INSERT INTO ple_data.published_question (question_id, created_at)
VALUES ('SRC-0001', '2026-08-31T00:00:00Z');
INSERT INTO ple_data.question_revision (question_id, revision_number, backend, published_at, public_metadata)
VALUES ('SRC-0001', 1, 'ple', '2026-08-31T00:00:00Z',
    jsonb_build_object('questionDescription', 'Published object-backed source'));
INSERT INTO ple_private.object_record (
    object_id, object_address, object_storage_area, object_data_class, sha256,
    size_bytes, media_type, created_at
) VALUES (
    '00000000-0000-0000-0000-000000000911',
    jsonb_build_object('kind', 'questionSource', 'questionRevision',
        jsonb_build_object('questionId', 'SRC-0001', 'revisionNumber', 1),
        'object', '00000000-0000-0000-0000-000000000911'::uuid),
    'private-content', 'question-source', decode(repeat('ab', 32), 'hex'), 17,
    'application/json', '2026-08-31T00:00:00Z'
);
INSERT INTO ple_private.question_source_registration (
    question_id, revision_number, backend, question_format,
    source_object_id, source_object_checksum, created_at, updated_at
) VALUES (
    'SRC-0001', 1, 'ple', 'pleQuestionJson',
    '00000000-0000-0000-0000-000000000911', repeat('ab', 32),
    '2026-08-31T00:00:00Z', '2026-08-31T00:00:00Z'
);
