-- Disposable catalog and lease oracle for the Question Asset publication
-- boundary.  It uses only runtime-generated identifiers and never reads or
-- emits object bytes, source locations, answers, or credentials.
\set ON_ERROR_STOP on

DO $$
DECLARE
    output_shape text[];
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_roles
         WHERE rolname = 'ple_public_asset_publisher'
           AND NOT rolcanlogin
           AND NOT rolinherit
           AND NOT rolsuper
           AND NOT rolcreatedb
           AND NOT rolcreaterole
           AND NOT rolreplication
           AND NOT rolbypassrls
    ) THEN
        RAISE EXCEPTION 'Public Asset Publisher capability is not a restricted NOLOGIN role';
    END IF;

    IF NOT has_function_privilege(
        'ple_public_asset_publisher',
        'ple_private.claim_question_asset_publication_job(uuid,timestamp with time zone)',
        'EXECUTE'
    ) OR NOT has_function_privilege(
        'ple_public_asset_publisher',
        'ple_private.activate_question_asset_publication(uuid,uuid)',
        'EXECUTE'
    ) OR has_function_privilege(
        'ple_public_asset_publisher',
        'ple_private.select_ready_question_asset_renditions(text,integer)',
        'EXECUTE'
    ) OR has_function_privilege(
        'ple_app',
        'ple_private.claim_question_asset_publication_job(uuid,timestamp with time zone)',
        'EXECUTE'
    ) OR has_function_privilege(
        'ple_app',
        'ple_private.activate_question_asset_publication(uuid,uuid)',
        'EXECUTE'
    ) OR has_function_privilege(
        'ple_app',
        'ple_private.select_ready_question_asset_renditions(text,integer)',
        'EXECUTE'
    ) OR has_table_privilege('ple_app', 'ple_private.question_asset_publication', 'SELECT')
       OR has_table_privilege('ple_api_owner', 'ple_private.question_asset_publication', 'SELECT') THEN
        RAISE EXCEPTION 'Question Asset publication grants bypass the publisher/API boundary';
    END IF;

    IF to_regprocedure(
        'ple_api.select_live_demo_ready_question_asset_renditions(text,integer)'
    ) IS NULL OR NOT has_function_privilege(
        'ple_app',
        'ple_api.select_live_demo_ready_question_asset_renditions(text,integer)',
        'EXECUTE'
    ) OR has_function_privilege(
        'public',
        'ple_api.select_live_demo_ready_question_asset_renditions(text,integer)',
        'EXECUTE'
    ) THEN
        RAISE EXCEPTION 'ready Question Asset API wrapper is not application-only';
    END IF;

    SELECT array_agg(
        format('%s:%s', argument_name, argument_type::regtype::text)
        ORDER BY ordinal_position
    ) INTO output_shape
      FROM (
        SELECT argument_name, argument_type, ordinal_position
          FROM pg_proc AS routine
         CROSS JOIN LATERAL unnest(
              routine.proallargtypes, routine.proargmodes, routine.proargnames
          ) WITH ORDINALITY AS argument(argument_type, argument_mode, argument_name, ordinal_position)
         WHERE routine.oid = 'ple_api.select_live_demo_ready_question_asset_renditions(text,integer)'::regprocedure
           AND argument_mode IN ('o', 't')
      ) AS output_argument;
    IF output_shape IS DISTINCT FROM ARRAY[
        'asset_id:uuid',
        'question_asset_checksum:text',
        'rendition_checksum:text',
        'intrinsic_width:integer',
        'intrinsic_height:integer'
    ] THEN
        RAISE EXCEPTION 'ready Question Asset API wrapper exposes an unexpected result shape';
    END IF;
END
$$;

-- A Pending record is enough to prove that the public API has no pre-ready
-- rendition.  The publisher claim and stale-token refusal below exercise the
-- real definer functions against their current catalog contracts.
SELECT gen_random_uuid() AS publication_job_id \gset
SELECT gen_random_uuid() AS publication_asset_id \gset
SELECT gen_random_uuid() AS publication_source_object_id \gset
SELECT gen_random_uuid() AS publication_public_object_id \gset
SELECT gen_random_uuid() AS publication_delivery_id \gset
\set publication_question_id 'SRC-0001'

BEGIN;
INSERT INTO ple_private.object_record (
    object_id, object_address, object_storage_area, object_data_class, sha256,
    size_bytes, media_type, created_at
) VALUES (
    :'publication_source_object_id'::uuid,
    jsonb_build_object(
        'kind', 'restrictedQuestionAsset',
        'questionRevision', jsonb_build_object(
            'questionId', :'publication_question_id', 'revisionNumber', 1
        ),
        'asset', :'publication_asset_id'::uuid,
        'object', :'publication_source_object_id'::uuid
    ),
    'private-content', 'question-asset', decode(repeat('ab', 32), 'hex'),
    17, 'image/png', clock_timestamp()
);
INSERT INTO ple_private.job (
    job_id, job_kind, job_target_kind, question_id, revision_number,
    generation, payload, state, available_at, max_attempts, created_at
) VALUES (
    :'publication_job_id'::uuid, 'publish_public_assets', 'public_asset_publication',
    :'publication_question_id', 1, 1, '{}'::jsonb, 'ready',
    clock_timestamp() - interval '1 second', 2, clock_timestamp()
);

INSERT INTO ple_data.object_delivery (
    delivery_id, object_id, sha256, media_type, byte_length, delivery_state, registered_at
) VALUES (
    :'publication_delivery_id'::uuid, :'publication_public_object_id'::uuid,
    decode(repeat('cd', 32), 'hex'), 'image/png', 17, 'pending', clock_timestamp()
);
INSERT INTO ple_data.question_asset_delivery (
    delivery_id, object_id, question_id, revision_number, asset_id
) VALUES (
    :'publication_delivery_id'::uuid, :'publication_public_object_id'::uuid,
    :'publication_question_id', 1, :'publication_asset_id'::uuid
);

INSERT INTO ple_private.question_asset_publication (
    question_id, revision_number, asset_id, source_object_id, source_object_checksum,
    public_object_id, public_object_checksum, public_byte_length, verified_media_type,
    intrinsic_width, intrinsic_height, delivery_id, job_id, publication_state
) VALUES (
    :'publication_question_id', 1, :'publication_asset_id'::uuid,
    :'publication_source_object_id'::uuid, decode(repeat('ab', 32), 'hex'),
    :'publication_public_object_id'::uuid, decode(repeat('cd', 32), 'hex'),
    17, 'image/png', 1, 1, :'publication_delivery_id'::uuid,
    :'publication_job_id'::uuid, 'pending'
);
COMMIT;

SELECT set_config('ple_e2e.publication_question_id', :'publication_question_id', false) \gset
SET ROLE ple_app;
DO $$
BEGIN
    IF EXISTS (
        SELECT 1 FROM ple_api.select_live_demo_ready_question_asset_renditions(
            current_setting('ple_e2e.publication_question_id'), 1
        )
    ) THEN
        RAISE EXCEPTION 'ready Question Asset API wrapper exposed a Pending rendition';
    END IF;
END
$$;
RESET ROLE;

SELECT gen_random_uuid() AS publication_lease_token \gset
SELECT set_config('ple_e2e.publication_lease_token', :'publication_lease_token', false) \gset
SET ROLE ple_public_asset_publisher;
SELECT job_id AS claimed_publication_job_id
  FROM ple_private.claim_question_asset_publication_job(
      :'publication_lease_token'::uuid, clock_timestamp() + interval '60 seconds'
  ) \gset
SELECT set_config('ple_e2e.publication_claimed_job_id', :'claimed_publication_job_id', false) \gset
DO $$
BEGIN
    PERFORM ple_private.activate_question_asset_publication(
        current_setting('ple_e2e.publication_claimed_job_id')::uuid, gen_random_uuid()
    );
    RAISE EXCEPTION 'Question Asset Publication accepted a stale lease token';
EXCEPTION WHEN insufficient_privilege THEN
    NULL;
END
$$;
RESET ROLE;

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1
          FROM ple_private.question_asset_publication AS publication
          JOIN ple_private.job AS job USING (job_id)
         WHERE publication.job_id = current_setting('ple_e2e.publication_claimed_job_id')::uuid
           AND publication.publication_state = 'pending'
           AND job.state = 'leased'
           AND job.lease_token = current_setting('ple_e2e.publication_lease_token')::uuid
           AND job.lease_expires_at > clock_timestamp()
    ) THEN
        RAISE EXCEPTION 'stale Question Asset Publication activation changed the current lease';
    END IF;
END
$$;
