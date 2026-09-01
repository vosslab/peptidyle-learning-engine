-- Catalog oracle for Question publication lifecycle and immutable credit facts.

DO $$
BEGIN
    IF to_regclass('ple_data.question_publication_event') IS NULL
        OR to_regclass('ple_data.question_revision_availability_event') IS NULL
        OR to_regclass('ple_data.published_question_lifecycle_event') IS NOT NULL
        OR EXISTS (
            SELECT 1 FROM information_schema.columns
            WHERE table_schema = 'ple_data' AND table_name = 'question_revision'
            AND column_name = 'lifecycle'
        )
        OR NOT EXISTS (
            SELECT 1 FROM pg_constraint
            WHERE conrelid = 'ple_data.question_publication_event'::regclass
            AND conname = 'question_publication_event_version_is_unique'
        ) OR NOT EXISTS (
            SELECT 1 FROM pg_constraint
            WHERE conrelid = 'ple_data.question_revision_availability_event'::regclass
            AND conname = 'question_revision_availability_event_kind_is_unique'
        ) OR NOT EXISTS (
            SELECT 1 FROM pg_trigger
            WHERE tgrelid = 'ple_data.question_revision_availability_event'::regclass
            AND tgname = 'question_revision_availability_event_has_valid_transition' AND NOT tgisinternal
        ) OR NOT EXISTS (
            SELECT 1 FROM pg_trigger
            WHERE tgrelid = 'ple_data.question_publication_event'::regclass
            AND tgname = 'question_publication_event_has_question_source' AND NOT tgisinternal
        ) THEN
        RAISE EXCEPTION 'Question publication and availability evidence remains conflated';
    END IF;

    IF to_regclass('ple_data.question_revision_acceptance') IS NULL
        OR to_regclass('ple_data.question_revision_authorship') IS NULL
        OR to_regclass('ple_data.question_revision_license') IS NULL
        OR to_regclass('ple_data.question_revision_citation') IS NULL
        OR to_regclass('ple_data.question_ownership_event') IS NULL
        OR to_regclass('ple_data.question_current_owner') IS NULL
        OR EXISTS (
            SELECT 1
            FROM information_schema.columns
            WHERE table_schema = 'ple_data'
              AND table_name = 'question_revision'
              AND column_name IN ('byline', 'license', 'citation', 'owner_account_id')
        )
        OR NOT EXISTS (
            SELECT 1 FROM pg_constraint
            WHERE conrelid = 'ple_data.question_revision_acceptance'::regclass
              AND conname = 'question_revision_acceptance_parent_matches'
        )
        OR NOT EXISTS (
            SELECT 1
            FROM information_schema.columns
            WHERE table_schema = 'ple_data'
              AND table_name = 'question_revision_acceptance'
              AND column_name = 'editor_account_id'
              AND is_nullable = 'NO'
        )
        OR NOT EXISTS (
            SELECT 1
            FROM information_schema.columns
            WHERE table_schema = 'ple_data'
              AND table_name = 'question_revision_acceptance'
              AND column_name = 'accepted_by_account_id'
              AND is_nullable = 'NO'
        )
        OR NOT EXISTS (
            SELECT 1
            FROM information_schema.columns
            WHERE table_schema = 'ple_data'
              AND table_name = 'question_revision_acceptance'
              AND column_name = 'accepted_at'
              AND is_nullable = 'NO'
        )
        OR NOT EXISTS (
            SELECT 1
            FROM information_schema.columns
            WHERE table_schema = 'ple_data'
              AND table_name = 'question_revision_acceptance'
              AND column_name = 'reason_for_edit'
              AND is_nullable = 'NO'
        )
        OR NOT EXISTS (
            SELECT 1 FROM pg_constraint
            WHERE conrelid = 'ple_data.question_revision_license'::regclass
              AND conname = 'question_revision_license_revision_matches'
        )
        OR NOT EXISTS (
            SELECT 1 FROM pg_constraint
            WHERE conrelid = 'ple_data.question_revision_citation'::regclass
              AND conname = 'question_revision_citation_has_text_or_url'
        )
        OR NOT EXISTS (
            SELECT 1 FROM pg_trigger
            WHERE tgrelid = 'ple_data.question_revision_acceptance'::regclass
              AND tgname = 'question_revision_acceptance_is_immutable' AND NOT tgisinternal
        )
        OR NOT EXISTS (
            SELECT 1 FROM pg_trigger
            WHERE tgrelid = 'ple_data.question_revision_acceptance'::regclass
              AND tgname = 'question_revision_acceptance_is_valid' AND NOT tgisinternal
        )
        OR NOT EXISTS (
            SELECT 1 FROM pg_trigger
            WHERE tgrelid = 'ple_data.question_ownership_event'::regclass
              AND tgname = 'question_ownership_event_has_valid_transition' AND NOT tgisinternal
        )
        OR NOT EXISTS (
            SELECT 1 FROM pg_trigger
            WHERE tgrelid = 'ple_data.question_publication_event'::regclass
              AND tgname = 'question_publication_event_has_required_credit' AND NOT tgisinternal
        )
        OR NOT EXISTS (
            SELECT 1 FROM pg_class
            WHERE oid = 'ple_data.question_revision_acceptance'::regclass
              AND relrowsecurity AND relforcerowsecurity
        )
        OR NOT EXISTS (
            SELECT 1 FROM pg_class
            WHERE oid = 'ple_data.question_revision_authorship'::regclass
              AND relrowsecurity AND relforcerowsecurity
        )
        OR NOT EXISTS (
            SELECT 1 FROM pg_class
            WHERE oid = 'ple_data.question_ownership_event'::regclass
              AND relrowsecurity AND relforcerowsecurity
        )
        OR NOT EXISTS (
            SELECT 1 FROM pg_class
            WHERE oid = 'ple_data.question_current_owner'::regclass
              AND reloptions @> ARRAY['security_invoker=true']
        ) THEN
        RAISE EXCEPTION 'Question Revision acceptance, Authorship, Owner, License, and Citation are not separate immutable publication facts';
    END IF;

    IF to_regclass('ple_private.draft_question_fork_source') IS NULL
        OR to_regclass('ple_data.question_fork_source') IS NULL
        OR NOT EXISTS (
            SELECT 1 FROM pg_constraint
            WHERE conrelid = 'ple_private.draft_question_fork_source'::regclass
              AND conname = 'draft_question_fork_source_source_revision_matches'
        )
        OR NOT EXISTS (
            SELECT 1 FROM pg_constraint
            WHERE conrelid = 'ple_data.question_fork_source'::regclass
              AND conname = 'question_fork_source_source_revision_matches'
        )
        OR NOT EXISTS (
            SELECT 1 FROM pg_trigger
            WHERE tgrelid = 'ple_private.draft_question_fork_source'::regclass
              AND tgname = 'draft_question_fork_source_is_immutable' AND NOT tgisinternal
        )
        OR NOT EXISTS (
            SELECT 1 FROM pg_trigger
            WHERE tgrelid = 'ple_data.question_fork_source'::regclass
              AND tgname = 'question_fork_source_is_immutable' AND NOT tgisinternal
        )
        OR EXISTS (
            SELECT 1 FROM pg_class
            WHERE oid IN (
                'ple_private.draft_question_fork_source'::regclass,
                'ple_data.question_fork_source'::regclass
            ) AND (NOT relrowsecurity OR NOT relforcerowsecurity)
        )
        OR to_regprocedure('ple_api.register_draft_question_fork_source(uuid,uuid,text,integer)') IS NULL
        OR to_regprocedure('ple_private.publish_question_fork_source(uuid,text)') IS NULL
        OR NOT has_function_privilege(
            'ple_app',
            'ple_api.register_draft_question_fork_source(uuid,uuid,text,integer)',
            'EXECUTE'
        )
        OR has_function_privilege(
            'ple_app',
            'ple_private.publish_question_fork_source(uuid,text)',
            'EXECUTE'
        ) THEN
        RAISE EXCEPTION 'Question Fork Source lacks its exact Draft, Published Question, or trusted-publication boundary';
    END IF;

    IF NOT EXISTS (
        SELECT 1
        FROM information_schema.columns
        WHERE table_schema = 'ple_data'
          AND table_name = 'question_revision'
          AND column_name = 'question_description'
          AND is_generated = 'ALWAYS'
    ) OR NOT EXISTS (
        SELECT 1
        FROM pg_class
        WHERE oid = 'ple_data.question_revision_question_description_search_idx'::regclass
    ) OR NOT EXISTS (
        SELECT 1
        FROM information_schema.columns
        WHERE table_schema = 'ple_api'
          AND table_name = 'published_question_summary'
          AND column_name = 'question_description'
    ) THEN
        RAISE EXCEPTION 'Question Description lacks its generated searchable publication projection';
    END IF;
END;
$$;

-- Publication accepts only a revision with its immutable acceptance facts.
-- `private_question_records.sql` has already registered this revision's exact
-- Question Source and its Instructor Account.
BEGIN;

INSERT INTO ple_data.question_revision_acceptance (
    question_id, revision_number, parent_revision_number, editor_account_id,
    accepted_by_account_id, accepted_at, reason_for_edit
) VALUES (
    'SRC-0001', 1, NULL, '00000000-0000-0000-0000-000000000901',
    '00000000-0000-0000-0000-000000000901', '2026-08-31T00:00:00Z',
    'Initial Question Revision acceptance'
);

DO $$
BEGIN
    BEGIN
        UPDATE ple_data.question_revision_acceptance
           SET reason_for_edit = 'A changed reason is a distinct accepted revision'
         WHERE question_id = 'SRC-0001' AND revision_number = 1;
        RAISE EXCEPTION 'Question Revision acceptance unexpectedly changed';
    EXCEPTION WHEN SQLSTATE '55000' THEN
        NULL;
    END;
END;
$$;

INSERT INTO ple_data.question_revision_authorship (
    question_id, revision_number, author_position, author_display_name, author_account_id
) VALUES (
    'SRC-0001', 1, 1, 'Question Source owner',
    '00000000-0000-0000-0000-000000000901'
);
INSERT INTO ple_data.question_revision_license (
    question_id, revision_number, spdx_expression
) VALUES ('SRC-0001', 1, 'CC-BY-4.0');
INSERT INTO ple_data.question_ownership_event (
    question_ownership_event_id, question_id, owner_account_id,
    recorded_by_account_id, event_kind, occurred_at
) VALUES (
    '00000000-0000-0000-0000-000000000915', 'SRC-0001',
    '00000000-0000-0000-0000-000000000901',
    '00000000-0000-0000-0000-000000000901', 'initial', '2026-08-31T00:00:00Z'
);
INSERT INTO ple_data.question_publication_event (
    event_id, question_id, revision_number, published_at
) VALUES (
    '00000000-0000-0000-0000-000000000916', 'SRC-0001', 1,
    '2026-08-31T00:00:00Z'
);

COMMIT;

-- An authorized Instructor binds a Draft Question to one exact source revision.
-- Only the trusted publication coordinator can bind that same source to a
-- separate Published Question lineage.
INSERT INTO ple_private.draft_question (
    draft_question_uuid, workspace_id, created_at
) VALUES (
    '00000000-0000-0000-0000-000000000920',
    '00000000-0000-0000-0000-000000000902', '2026-08-31T00:00:00Z'
);
INSERT INTO ple_private.draft_question_revision (
    draft_question_revision_uuid, draft_question_uuid, revision_number, title,
    question_content, created_at
) VALUES (
    '00000000-0000-0000-0000-000000000921',
    '00000000-0000-0000-0000-000000000920', 1, 'Forked source fixture',
    '{}'::jsonb, '2026-08-31T00:00:00Z'
);

BEGIN;
SET LOCAL ROLE ple_app;
SELECT pg_catalog.set_config(
    'ple.session_account_id', '00000000-0000-0000-0000-000000000901', true
);
SELECT ple_api.register_workspace_question_source_object(
    '00000000-0000-0000-0000-000000000902',
    '00000000-0000-0000-0000-000000000922',
    jsonb_build_object(
        'kind', 'workspaceQuestionSource',
        'workspace', '00000000-0000-0000-0000-000000000902'::uuid,
        'object', '00000000-0000-0000-0000-000000000922'::uuid
    ),
    decode(repeat('ab', 32), 'hex'), 17, 'application/json', 1777603200000
);
SELECT ple_api.register_draft_question_source(
    '00000000-0000-0000-0000-000000000923',
    '00000000-0000-0000-0000-000000000920', 1,
    '00000000-0000-0000-0000-000000000902',
    'ple', 'pleQuestionJson', 'multipleChoice',
    jsonb_build_object('backend', 'ple'),
    '00000000-0000-0000-0000-000000000922', repeat('ab', 32), repeat('cd', 32)
);
SELECT ple_api.register_draft_question_fork_source(
    '00000000-0000-0000-0000-000000000920',
    '00000000-0000-0000-0000-000000000902', 'SRC-0001', 1
);
COMMIT;

INSERT INTO ple_data.published_question (question_id, created_at)
VALUES ('FRK-0001', '2026-08-31T00:00:00Z');
INSERT INTO ple_data.question_revision (
    question_id, revision_number, backend, published_at, public_metadata
) VALUES (
    'FRK-0001', 1, 'ple', '2026-08-31T00:00:00Z',
    jsonb_build_object('questionDescription', 'Forked source fixture Question')
);

SET ROLE ple_api_owner;
SELECT ple_private.transfer_draft_question_source_to_question_revision(
    '00000000-0000-0000-0000-000000000924',
    '00000000-0000-0000-0000-000000000920', 1,
    'FRK-0001', 1,
    '00000000-0000-0000-0000-000000000925',
    jsonb_build_object(
        'kind', 'questionSource',
        'questionRevision', jsonb_build_object('questionId', 'FRK-0001', 'revisionNumber', 1),
        'object', '00000000-0000-0000-0000-000000000925'::uuid
    ),
    decode(repeat('ab', 32), 'hex'), 17, 'application/json', 1777603200000
);
RESET ROLE;

DO $$
BEGIN
    BEGIN
        UPDATE ple_data.question_fork_source
           SET source_revision_number = 2
         WHERE forked_question_id = 'FRK-0001';
        RAISE EXCEPTION 'Question Fork Source unexpectedly changed';
    EXCEPTION WHEN SQLSTATE '55000' THEN
        NULL;
    END;
END;
$$;

DO $$
BEGIN
    SET LOCAL ROLE ple_app;
    PERFORM ple_private.publish_question_fork_source(
        '00000000-0000-0000-0000-000000000920', 'FRK-0001'
    );
    RAISE EXCEPTION 'Question Fork publication helper was exposed to ple_app';
EXCEPTION
    WHEN insufficient_privilege THEN NULL;
END;
$$;
