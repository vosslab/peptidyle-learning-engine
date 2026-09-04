-- WP-SD1-A-QSOM1-P1 PostgreSQL behavior oracle. The preceding Question
-- records oracle supplies Draft Question 905 at Edit Number 3 with its exact
-- current WeBWorK Source Binding and immutable source Object Record.

DO $$
BEGIN
    IF to_regprocedure(
        'ple_api.load_draft_question_publication_source(uuid,bigint,uuid)'
    ) IS NULL
       OR NOT has_function_privilege(
           'ple_app',
           'ple_api.load_draft_question_publication_source(uuid,bigint,uuid)',
           'EXECUTE'
       )
       OR has_function_privilege(
           'ple_app',
           'ple_private.load_draft_question_publication_source(uuid,bigint,uuid)',
           'EXECUTE'
       )
       OR to_regprocedure(
        'ple_api.publish_new_question_lineage(uuid,bigint,uuid,text,uuid,jsonb,bytea,bigint,text,bigint,jsonb,text,text,uuid,uuid,uuid)'
    ) IS NULL
       OR NOT has_function_privilege(
           'ple_app',
           'ple_api.publish_new_question_lineage(uuid,bigint,uuid,text,uuid,jsonb,bytea,bigint,text,bigint,jsonb,text,text,uuid,uuid,uuid)',
           'EXECUTE'
       )
       OR has_function_privilege(
           'ple_app',
           'ple_private.publish_new_question_lineage(uuid,bigint,uuid,text,uuid,jsonb,bytea,bigint,text,bigint,jsonb,text,text,uuid,uuid,uuid)',
           'EXECUTE'
       ) THEN
        RAISE EXCEPTION 'Question Publication trusted function boundary is incomplete';
    END IF;
END;
$$;

BEGIN;
SET LOCAL ROLE ple_app;
SELECT pg_catalog.set_config(
    'ple.session_account_id', '00000000-0000-0000-0000-000000000901', true
);
DO $$
DECLARE
    source_record record;
BEGIN
    SELECT * INTO STRICT source_record
      FROM ple_api.load_draft_question_publication_source(
          '00000000-0000-0000-0000-000000000905', 3,
          '00000000-0000-0000-0000-000000000902'
      );
    IF source_record.object_id <> '00000000-0000-0000-0000-000000000903'
       OR source_record.object_address <> jsonb_build_object(
           'kind', 'workspaceQuestionSource',
           'workspace', '00000000-0000-0000-0000-000000000902'::uuid,
           'object', '00000000-0000-0000-0000-000000000903'::uuid
       )
       OR source_record.sha256 <> decode(repeat('ab', 32), 'hex')
       OR source_record.size_bytes <> 17
       OR source_record.media_type <> 'application/json'
       OR source_record.created_at_millis <> 1777603200000 THEN
        RAISE EXCEPTION 'Draft Question Publication Source resolution changed exact Object Record fields';
    END IF;
END;
$$;
DO $$
BEGIN
    PERFORM * FROM ple_api.load_draft_question_publication_source(
        '00000000-0000-0000-0000-000000000905', 2,
        '00000000-0000-0000-0000-000000000902'
    );
    RAISE EXCEPTION 'Draft Question Publication Source accepted a stale Edit Number';
EXCEPTION WHEN serialization_failure THEN NULL;
END;
$$;
COMMIT;

INSERT INTO ple_private.account (account_id, product_role, created_at)
VALUES ('00000000-0000-0000-0000-000000000914', 'student', '2026-09-03T00:00:00Z');
BEGIN;
SET LOCAL ROLE ple_app;
SELECT pg_catalog.set_config(
    'ple.session_account_id', '00000000-0000-0000-0000-000000000914', true
);
DO $$
BEGIN
    PERFORM * FROM ple_api.load_draft_question_publication_source(
        '00000000-0000-0000-0000-000000000905', 3,
        '00000000-0000-0000-0000-000000000902'
    );
    RAISE EXCEPTION 'Draft Question Publication Source accepted a Student Account';
EXCEPTION WHEN insufficient_privilege THEN NULL;
END;
$$;
COMMIT;

BEGIN;
SET LOCAL ROLE ple_app;
SELECT pg_catalog.set_config(
    'ple.session_account_id', '00000000-0000-0000-0000-000000000901', true
);

DO $$
BEGIN
    PERFORM ple_api.publish_new_question_lineage(
        '00000000-0000-0000-0000-000000000905', 2,
        '00000000-0000-0000-0000-000000000902', 'NEW-0001',
        '00000000-0000-0000-0000-000000000913',
        jsonb_build_object(
            'kind', 'questionSource',
            'questionRevision', jsonb_build_object(
                'questionId', 'NEW-0001', 'revisionNumber', 1
            ),
            'object', '00000000-0000-0000-0000-000000000913'::uuid
        ),
        decode(repeat('ab', 32), 'hex'), 17, 'application/json', 1788477000000,
        jsonb_build_array('Current Instructor'), 'CC-BY-4.0',
        'Initial reviewed publication',
        '00000000-0000-0000-0000-000000000970',
        '00000000-0000-0000-0000-000000000971',
        '00000000-0000-0000-0000-000000000972'
    );
    RAISE EXCEPTION 'Question Publication accepted a stale Draft Question Edit Number';
EXCEPTION WHEN serialization_failure THEN NULL;
END;
$$;

DO $$
BEGIN
    PERFORM ple_api.publish_new_question_lineage(
        '00000000-0000-0000-0000-000000000905', 3,
        '00000000-0000-0000-0000-000000000902', 'NEW-0001',
        '00000000-0000-0000-0000-000000000913',
        jsonb_build_object(
            'kind', 'questionSource',
            'questionRevision', jsonb_build_object(
                'questionId', 'NEW-0001', 'revisionNumber', 1
            ),
            'object', '00000000-0000-0000-0000-000000000913'::uuid
        ),
        decode(repeat('ab', 32), 'hex'), 17, 'application/json', 1788477000000,
        jsonb_build_array(17), 'CC-BY-4.0', 'Initial reviewed publication',
        '00000000-0000-0000-0000-000000000970',
        '00000000-0000-0000-0000-000000000971',
        '00000000-0000-0000-0000-000000000972'
    );
    RAISE EXCEPTION 'Question Publication coerced a non-string Question Author';
EXCEPTION WHEN invalid_parameter_value THEN NULL;
END;
$$;

DO $$
BEGIN
    PERFORM ple_api.publish_new_question_lineage(
        '00000000-0000-0000-0000-000000000905', 3,
        '00000000-0000-0000-0000-000000000909', 'NEW-0001',
        '00000000-0000-0000-0000-000000000913',
        '{}'::jsonb, decode(repeat('ab', 32), 'hex'), 17,
        'application/json', 1788477000000,
        jsonb_build_array('Current Instructor'), 'CC-BY-4.0',
        'Initial reviewed publication',
        '00000000-0000-0000-0000-000000000970',
        '00000000-0000-0000-0000-000000000971',
        '00000000-0000-0000-0000-000000000972'
    );
    RAISE EXCEPTION 'Question Publication accepted another workspace';
EXCEPTION WHEN insufficient_privilege THEN NULL;
END;
$$;

SELECT ple_api.publish_new_question_lineage(
    '00000000-0000-0000-0000-000000000905', 3,
    '00000000-0000-0000-0000-000000000902', 'NEW-0001',
    '00000000-0000-0000-0000-000000000913',
    jsonb_build_object(
        'kind', 'questionSource',
        'questionRevision', jsonb_build_object(
            'questionId', 'NEW-0001', 'revisionNumber', 1
        ),
        'object', '00000000-0000-0000-0000-000000000913'::uuid
    ),
    decode(repeat('ab', 32), 'hex'), 17, 'application/json', 1788477000000,
    jsonb_build_array('Current Instructor', 'External Contributor'),
    'CC-BY-4.0', 'Initial reviewed publication',
    '00000000-0000-0000-0000-000000000970',
    '00000000-0000-0000-0000-000000000971',
    '00000000-0000-0000-0000-000000000972'
);
COMMIT;

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1
          FROM ple_data.published_question_metadata AS metadata
          JOIN ple_data.question_revision AS revision USING (question_id)
          JOIN ple_private.question_revision_source_binding AS binding
            ON binding.question_id = revision.question_id
           AND binding.revision_number = revision.revision_number
          JOIN ple_private.object_record AS record
            ON record.object_id = binding.source_object_id
         WHERE metadata.question_id = 'NEW-0001'
           AND metadata.question_title = 'Object-backed source'
           AND metadata.question_description = 'Draft source fixture'
           AND revision.revision_number = 1
           AND revision.backend = 'webwork'
           AND binding.question_format = 'webworkPg'
           AND binding.webwork_pg_path = 'set/object-backed.pg'
           AND binding.source_object_id = '00000000-0000-0000-0000-000000000913'
           AND record.object_address = jsonb_build_object(
               'kind', 'questionSource',
               'questionRevision', jsonb_build_object(
                   'questionId', 'NEW-0001', 'revisionNumber', 1
               ),
               'object', '00000000-0000-0000-0000-000000000913'::uuid
           )
    ) OR (SELECT count(*) FROM ple_data.question_revision_authorship
           WHERE question_id = 'NEW-0001' AND revision_number = 1) <> 2
       OR EXISTS (
           SELECT 1 FROM ple_data.question_revision_authorship
            WHERE question_id = 'NEW-0001' AND revision_number = 1
              AND author_account_id IS NOT NULL
       )
       OR NOT EXISTS (
           SELECT 1 FROM ple_data.question_revision_license
            WHERE question_id = 'NEW-0001' AND revision_number = 1
              AND spdx_expression = 'CC-BY-4.0'
       ) OR NOT EXISTS (
           SELECT 1 FROM ple_data.question_publication_event
            WHERE question_id = 'NEW-0001' AND revision_number = 1
       ) OR NOT EXISTS (
           SELECT 1 FROM ple_data.question_revision_availability_event
            WHERE question_id = 'NEW-0001' AND revision_number = 1
              AND availability = 'available'
       ) THEN
        RAISE EXCEPTION 'Question Publication did not create one complete Published Question aggregate';
    END IF;
END;
$$;

-- Draft cleanup cannot remove or mutate any Published Question fact.
DELETE FROM ple_private.draft_question
 WHERE draft_question_uuid = '00000000-0000-0000-0000-000000000905';
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM ple_data.published_question_metadata
         WHERE question_id = 'NEW-0001'
    ) OR NOT EXISTS (
        SELECT 1 FROM ple_private.question_revision_source_binding
         WHERE question_id = 'NEW-0001' AND revision_number = 1
    ) OR EXISTS (
        SELECT 1 FROM ple_private.draft_question_source_binding
         WHERE draft_question_uuid = '00000000-0000-0000-0000-000000000905'
    ) THEN
        RAISE EXCEPTION 'Published Question storage retains a Draft Question dependency';
    END IF;
END;
$$;
