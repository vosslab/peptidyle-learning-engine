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
            AND tgname = 'question_publication_event_has_question_source_binding' AND NOT tgisinternal
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
            SELECT 1 FROM pg_constraint
            WHERE conrelid = 'ple_data.question_revision_authorship'::regclass
              AND conname = 'question_revision_authorship_position_is_bounded'
        )
        OR NOT EXISTS (
            SELECT 1 FROM pg_constraint
            WHERE conrelid = 'ple_data.question_revision_authorship'::regclass
              AND conname = 'question_revision_authorship_display_name_is_reviewed'
        )
        OR NOT EXISTS (
            SELECT 1 FROM information_schema.columns
            WHERE table_schema = 'ple_data'
              AND table_name = 'question_revision_authorship'
              AND column_name = 'author_account_id'
              AND is_nullable = 'YES'
        )
        OR NOT EXISTS (
            SELECT 1 FROM pg_constraint
            WHERE conrelid = 'ple_data.question_revision_authorship'::regclass
              AND contype = 'f'
              AND confrelid = 'ple_private.account'::regclass
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
        RAISE EXCEPTION 'Question Revision acceptance, Question Authorship, Question Owner, Question License, and Question Citation are not separate immutable publication facts';
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

    IF to_regclass('ple_data.published_question_metadata') IS NULL
        OR EXISTS (
            SELECT 1 FROM information_schema.columns
             WHERE table_schema = 'ple_data' AND table_name = 'question_revision'
               AND column_name IN ('public_metadata', 'question_description')
        ) OR NOT EXISTS (
            SELECT 1 FROM pg_class
             WHERE oid = 'ple_data.published_question_metadata_search_idx'::regclass
        ) OR NOT EXISTS (
            SELECT 1 FROM information_schema.columns
             WHERE table_schema = 'ple_api' AND table_name = 'published_question_summary'
               AND column_name = 'question_title'
        ) OR NOT EXISTS (
            SELECT 1 FROM information_schema.columns
             WHERE table_schema = 'ple_api' AND table_name = 'published_question_summary'
               AND column_name = 'question_description'
        ) OR pg_get_viewdef('ple_api.published_question_summary'::regclass, true)
            NOT LIKE '%published_question_metadata%'
          OR pg_get_viewdef('ple_api.published_question_summary'::regclass, true)
            LIKE '%draft_question%' THEN
        RAISE EXCEPTION 'Published Question Metadata does not own searchable discovery fields';
    END IF;
END;
$$;

INSERT INTO ple_private.account (account_id, product_role, created_at) VALUES
    ('00000000-0000-0000-0000-000000000930', 'instructor', '2026-08-31T00:00:00Z'),
    ('00000000-0000-0000-0000-000000000931', 'instructor', '2026-08-31T00:00:00Z');
INSERT INTO ple_private.account_state_event (
    event_id, account_id, state, occurred_at, reason
) VALUES (
    '00000000-0000-0000-0000-000000000932',
    '00000000-0000-0000-0000-000000000931',
    'closed', '2026-09-01T00:00:00Z', 'Closed owner validation fixture'
);

DO $$
DECLARE
    validation_constraint text;
    validation_message text;
BEGIN
    BEGIN
        INSERT INTO ple_data.question_revision_authorship (
            question_id, revision_number, author_position, author_display_name
        ) VALUES ('SRC-0001', 1, 17, 'Bounded author position');
        RAISE EXCEPTION 'Question Authorship accepted a seventeenth Question Author';
    EXCEPTION WHEN check_violation THEN
        GET STACKED DIAGNOSTICS validation_constraint = CONSTRAINT_NAME;
        IF validation_constraint IS DISTINCT FROM
            'question_revision_authorship_position_is_bounded' THEN
            RAISE;
        END IF;
    END;

    BEGIN
        INSERT INTO ple_data.question_revision_authorship (
            question_id, revision_number, author_position, author_display_name
        ) VALUES ('SRC-0001', 1, 2, ' Unreviewed whitespace');
        RAISE EXCEPTION 'Question Authorship accepted an untrimmed Question Author display name';
    EXCEPTION WHEN check_violation THEN
        NULL;
    END;

    BEGIN
        INSERT INTO ple_data.question_revision_authorship (
            question_id, revision_number, author_position, author_display_name
        ) VALUES ('SRC-0001', 1, 2, 'Control' || chr(7));
        RAISE EXCEPTION 'Question Authorship accepted a control-bearing Question Author display name';
    EXCEPTION WHEN check_violation THEN
        NULL;
    END;

    BEGIN
        INSERT INTO ple_data.question_revision_authorship (
            question_id, revision_number, author_position, author_display_name, author_account_id
        ) VALUES (
            'SRC-0001', 1, 2, 'Missing Account',
            '00000000-0000-0000-0000-000000000999'
        );
        RAISE EXCEPTION 'Question Authorship accepted an unknown Account reference';
    EXCEPTION WHEN foreign_key_violation THEN
        NULL;
    END;

    BEGIN
        INSERT INTO ple_data.question_revision_authorship (
            question_id, revision_number, author_position, author_display_name
        ) VALUES
            ('SRC-0001', 1, 1, 'First Question Author'),
            ('SRC-0001', 1, 3, 'Third Question Author');
        INSERT INTO ple_data.question_publication_event (
            event_id, question_id, revision_number, published_at
        ) VALUES (
            '00000000-0000-0000-0000-000000000917', 'SRC-0001', 1,
            '2026-08-31T00:00:00Z'
        );
        SET CONSTRAINTS ALL IMMEDIATE;
        RAISE EXCEPTION 'Question Publication accepted noncontiguous Question Author positions';
    EXCEPTION WHEN check_violation THEN
        GET STACKED DIAGNOSTICS validation_message = MESSAGE_TEXT;
        IF validation_message IS DISTINCT FROM
            'Question Publication requires contiguous Question Author positions' THEN
            RAISE;
        END IF;
    END;
END;
$$;

-- Publication accepts only a revision with its immutable acceptance facts.
-- `question_records.sql` has already registered this revision's exact
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
INSERT INTO ple_data.question_revision_authorship (
    question_id, revision_number, author_position, author_display_name
) VALUES (
    'SRC-0001', 1, 2, 'External Question Author'
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

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1
        FROM ple_data.question_revision_authorship AS authorship
        WHERE authorship.question_id = 'SRC-0001'
          AND authorship.revision_number = 1
          AND authorship.author_position = 2
          AND authorship.author_account_id IS NULL
    ) OR NOT EXISTS (
        SELECT 1
        FROM ple_data.question_current_owner AS owner
        WHERE owner.question_id = 'SRC-0001'
          AND owner.owner_account_id = '00000000-0000-0000-0000-000000000901'
    ) THEN
        RAISE EXCEPTION 'Question Authorship Account references and Question Owner authority were conflated';
    END IF;
END;
$$;

DO $$
BEGIN
    BEGIN
        INSERT INTO ple_data.question_ownership_event (
            question_ownership_event_id, question_id, owner_account_id,
            recorded_by_account_id, event_kind, occurred_at
        ) VALUES (
            '00000000-0000-0000-0000-000000000933', 'SRC-0001',
            '00000000-0000-0000-0000-000000000930',
            '00000000-0000-0000-0000-000000000930', 'transferred',
            '2026-09-01T01:00:00Z'
        );
        RAISE EXCEPTION 'a non-owner recorded a Question Owner transfer';
    EXCEPTION WHEN check_violation THEN NULL;
    END;

    BEGIN
        INSERT INTO ple_data.question_ownership_event (
            question_ownership_event_id, question_id, owner_account_id,
            recorded_by_account_id, event_kind, occurred_at
        ) VALUES (
            '00000000-0000-0000-0000-000000000934', 'SRC-0001',
            '00000000-0000-0000-0000-000000000931',
            '00000000-0000-0000-0000-000000000901', 'transferred',
            '2026-09-01T01:00:00Z'
        );
        RAISE EXCEPTION 'Question Ownership transferred to a non-active Instructor Account';
    EXCEPTION WHEN check_violation THEN NULL;
    END;
END;
$$;
INSERT INTO ple_data.question_ownership_event (
    question_ownership_event_id, question_id, owner_account_id,
    recorded_by_account_id, event_kind, occurred_at
) VALUES (
    '00000000-0000-0000-0000-000000000935', 'SRC-0001',
    '00000000-0000-0000-0000-000000000930',
    '00000000-0000-0000-0000-000000000901', 'transferred',
    '2026-09-01T01:00:00Z'
);
INSERT INTO ple_data.question_ownership_event (
    question_ownership_event_id, question_id, owner_account_id,
    recorded_by_account_id, event_kind, occurred_at
) VALUES (
    '00000000-0000-0000-0000-000000000936', 'SRC-0001',
    '00000000-0000-0000-0000-000000000901',
    '00000000-0000-0000-0000-000000000930', 'transferred',
    '2026-09-01T02:00:00Z'
);
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1
          FROM ple_data.question_current_owner AS owner
         WHERE owner.question_id = 'SRC-0001'
           AND owner.owner_account_id = '00000000-0000-0000-0000-000000000901'
    ) THEN
        RAISE EXCEPTION 'Question Owner did not follow the ordered accepted transfer chain';
    END IF;
END;
$$;

-- Latest Question Revision is derived from immutable acceptance evidence,
-- rather than from Question Revision Availability or a mutable pointer.
INSERT INTO ple_data.question_revision (
    question_id, revision_number, backend, published_at
) VALUES ('SRC-0001', 2, 'ple', '2026-09-01T00:00:00Z');
INSERT INTO ple_data.question_revision_acceptance (
    question_id, revision_number, parent_revision_number, editor_account_id,
    accepted_by_account_id, accepted_at, reason_for_edit
) VALUES (
    'SRC-0001', 2, 1, '00000000-0000-0000-0000-000000000901',
    '00000000-0000-0000-0000-000000000901', '2026-09-01T00:00:00Z',
    'Accepted successor Question Revision'
);

DO $$
DECLARE
    latest_revision_number integer;
    visible_question_count integer;
BEGIN
    SET LOCAL ROLE ple_app;
    PERFORM pg_catalog.set_config(
        'ple.session_account_id', '00000000-0000-0000-0000-000000000901', true
    );
    SELECT summary.latest_question_revision_number
      INTO latest_revision_number
      FROM ple_api.published_question_summary AS summary
     WHERE summary.question_id = 'SRC-0001';
    IF latest_revision_number IS DISTINCT FROM 2 THEN
        RAISE EXCEPTION 'Question Summary did not derive the greatest accepted Question Revision Number';
    END IF;

    PERFORM pg_catalog.set_config(
        'ple.session_account_id', '00000000-0000-0000-0000-000000000930', true
    );
    SELECT count(*)
      INTO visible_question_count
      FROM ple_api.published_question_summary AS summary
     WHERE summary.question_id = 'SRC-0001';
    IF visible_question_count IS DISTINCT FROM 1 THEN
        RAISE EXCEPTION 'Question Library visibility was restricted to the Question Owner';
    END IF;

    PERFORM pg_catalog.set_config(
        'ple.session_account_id', '00000000-0000-0000-0000-000000000931', true
    );
    IF EXISTS (
        SELECT 1
          FROM ple_api.published_question_summary AS summary
         WHERE summary.question_id = 'SRC-0001'
    ) THEN
        RAISE EXCEPTION 'Question Library visibility admitted a non-active Instructor Account';
    END IF;
END;
$$;

DO $$
BEGIN
    SET LOCAL ROLE ple_app;
    IF EXISTS (SELECT 1 FROM ple_api.published_question_summary) THEN
        RAISE EXCEPTION 'Question Summary exposed Question Library rows without an Instructor session';
    END IF;
END;
$$;

-- An authorized Instructor binds a Draft Question to one exact source revision.
-- A future authorized publication operation atomically creates its complete
-- Question Revision-owned Source Binding/object evidence and may record
-- the matching immutable Question Fork Source for a separate published lineage.
INSERT INTO ple_private.draft_question (
    draft_question_uuid, workspace_id, draft_question_edit_number, created_at, updated_at
) VALUES (
    '00000000-0000-0000-0000-000000000920',
    '00000000-0000-0000-0000-000000000902', 1,
    '2026-08-31T00:00:00Z', '2026-08-31T00:00:00Z'
);
INSERT INTO ple_private.draft_question_metadata (
    draft_question_uuid, question_title, question_description, created_at, updated_at
) VALUES (
    '00000000-0000-0000-0000-000000000920', 'Forked source fixture',
    'Forked source fixture', '2026-08-31T00:00:00Z', '2026-08-31T00:00:00Z'
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
SELECT ple_api.bind_draft_question_source(
    '00000000-0000-0000-0000-000000000920', 1,
    '00000000-0000-0000-0000-000000000902',
    'ple', 'pleQuestionJson',
    NULL, NULL, NULL, NULL,
    '00000000-0000-0000-0000-000000000922', repeat('ab', 32)
);
SELECT ple_api.register_draft_question_fork_source(
    '00000000-0000-0000-0000-000000000920',
    '00000000-0000-0000-0000-000000000902', 'SRC-0001', 1
);
COMMIT;

INSERT INTO ple_data.published_question (question_id, created_at)
VALUES ('FRK-0001', '2026-08-31T00:00:00Z');
INSERT INTO ple_data.question_revision (
    question_id, revision_number, backend, published_at
) VALUES ('FRK-0001', 1, 'ple', '2026-08-31T00:00:00Z');
INSERT INTO ple_data.published_question_metadata (
    question_id, question_title, question_description, created_at, updated_at
) VALUES (
    'FRK-0001', 'Forked source fixture Question', 'Forked source fixture Question',
    '2026-08-31T00:00:00Z', '2026-08-31T00:00:00Z'
);

-- Published Question fixtures insert their immutable lineage directly. A future
-- authorized publication operation atomically creates the complete Question
-- Revision-owned Question Revision Source Binding, its object evidence, and bounded
-- metadata as applicable. Backend/Format-specific private artifacts are derived
-- or stored only when that backend requires them; no universal generic private
-- sidecar is required.

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
