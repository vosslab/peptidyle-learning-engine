-- A published HOTSPOT receives a fresh catalog AssetId for each immutable
-- version. Rebind only the trusted private grading envelope's public checksum
-- while copying it; the workspace asset and current workspace grading remain
-- immutable and untouched.

DROP FUNCTION public.ple_promote_flat_question_grading(
    uuid, uuid, bigint, character(64), uuid, character(64), character(64),
    character(64), uuid, uuid
);

CREATE FUNCTION public.ple_promote_flat_question_grading(
    p_tenant uuid,
    p_workspace uuid,
    p_expected_draft_revision bigint,
    p_expected_draft_payload_sha256 character(64),
    p_expected_source_object_id uuid,
    p_expected_source_payload_sha256 character(64),
    p_expected_canonical_source_sha256 character(64),
    p_expected_public_binding_sha256 character(64),
    p_problem uuid,
    p_version uuid,
    p_published_public_binding_sha256 character(64)
) RETURNS boolean
LANGUAGE plpgsql SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public', 'pg_temp'
AS $$
DECLARE
    current_grading public.workspace_flat_question_grading%ROWTYPE;
    rebound_bytes bytea;
    rebound_sha256 character(64);
    rebound_payload jsonb;
    inserted_count bigint;
BEGIN
    IF p_tenant IS NULL OR p_workspace IS NULL
       OR p_expected_draft_revision IS NULL OR p_expected_draft_payload_sha256 IS NULL
       OR p_expected_source_object_id IS NULL OR p_expected_source_payload_sha256 IS NULL
       OR p_expected_canonical_source_sha256 IS NULL OR p_expected_public_binding_sha256 IS NULL
       OR p_problem IS NULL OR p_version IS NULL OR p_published_public_binding_sha256 IS NULL
       OR p_expected_draft_revision <= 0 OR p_tenant <> public.ple_current_tenant()
       OR EXISTS (
            SELECT 1 FROM unnest(ARRAY[
                p_expected_draft_payload_sha256, p_expected_source_payload_sha256,
                p_expected_canonical_source_sha256, p_expected_public_binding_sha256,
                p_published_public_binding_sha256
            ]) AS digest WHERE digest !~ '^[0-9a-f]{64}$'::text
       )
    THEN
        RAISE EXCEPTION 'invalid flat grading publication promotion capability'
            USING ERRCODE = '22023';
    END IF;

    PERFORM 1 FROM public.workspace_draft AS draft
     WHERE draft.tenant_id = p_tenant AND draft.workspace_id = p_workspace
       AND draft.revision = p_expected_draft_revision
       AND draft.payload_sha256 = p_expected_draft_payload_sha256
     FOR UPDATE;
    IF NOT FOUND THEN RETURN false; END IF;

    PERFORM 1 FROM public.workspace_flat_question_source AS source
     WHERE source.tenant_id = p_tenant AND source.workspace_id = p_workspace
       AND source.draft_revision = p_expected_draft_revision
       AND source.draft_payload_sha256 = p_expected_draft_payload_sha256
       AND source.source_object_id = p_expected_source_object_id
       AND source.source_payload_sha256 = p_expected_source_payload_sha256
       AND source.canonical_source_sha256 = p_expected_canonical_source_sha256
       AND source.public_binding_sha256 = p_expected_public_binding_sha256
     FOR KEY SHARE;
    IF NOT FOUND THEN RETURN false; END IF;

    SELECT * INTO current_grading FROM public.workspace_flat_question_grading AS grading
     WHERE grading.tenant_id = p_tenant AND grading.workspace_id = p_workspace
       AND grading.draft_revision = p_expected_draft_revision
       AND grading.draft_payload_sha256 = p_expected_draft_payload_sha256
       AND grading.source_object_id = p_expected_source_object_id
       AND grading.source_payload_sha256 = p_expected_source_payload_sha256
       AND grading.canonical_source_sha256 = p_expected_canonical_source_sha256
       AND grading.public_binding_sha256 = p_expected_public_binding_sha256
       AND public.ple_flat_question_grading_envelope_valid(
            grading.key_payload, grading.key_sha256, grading.public_binding_sha256
       )
     FOR KEY SHARE;
    IF NOT FOUND THEN RETURN false; END IF;

    IF NOT EXISTS (
        SELECT 1 FROM public.problem AS problem_row
        JOIN public.problem_version AS candidate ON candidate.problem_id = problem_row.problem_id
        WHERE problem_row.problem_id = p_problem AND problem_row.owner_tenant_id = p_tenant
          AND candidate.version_id = p_version AND candidate.workspace_id = p_workspace
          AND candidate.backend = 'native'::text
    ) THEN RETURN false; END IF;

    rebound_bytes := convert_to(
        regexp_replace(
            convert_from(decode(current_grading.key_payload ->> 'payloadBase64', 'base64'), 'UTF8'),
            '"publicSha256":"[0-9a-f]{64}"',
            format('"publicSha256":"%s"', p_published_public_binding_sha256), 1, 1, 'c'
        ),
        'UTF8'
    );
    rebound_sha256 := encode(sha256(rebound_bytes), 'hex');
    rebound_payload := jsonb_build_object(
        'publicSha256', p_published_public_binding_sha256,
        'payloadSha256', rebound_sha256,
        'payloadBase64', replace(encode(rebound_bytes, 'base64'), E'\n', '')
    );
    IF NOT public.ple_flat_question_grading_envelope_valid(
        rebound_payload, rebound_sha256, p_published_public_binding_sha256
    ) THEN RETURN false; END IF;

    INSERT INTO public.answer_key (problem_id, version_id, key_payload, key_sha256)
    VALUES (p_problem, p_version, rebound_payload, rebound_sha256)
    ON CONFLICT (problem_id, version_id) DO NOTHING;
    GET DIAGNOSTICS inserted_count = ROW_COUNT;
    RETURN inserted_count = 1;
END
$$;

ALTER FUNCTION public.ple_promote_flat_question_grading(
    uuid, uuid, bigint, character(64), uuid, character(64), character(64),
    character(64), uuid, uuid, character(64)
) OWNER TO ple_grader;
REVOKE ALL ON FUNCTION public.ple_promote_flat_question_grading(
    uuid, uuid, bigint, character(64), uuid, character(64), character(64),
    character(64), uuid, uuid, character(64)
) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION public.ple_promote_flat_question_grading(
    uuid, uuid, bigint, character(64), uuid, character(64), character(64),
    character(64), uuid, uuid, character(64)
) TO ple_app;

-- PostgreSQL treats SELECT FOR KEY SHARE as an UPDATE-class lock. Grant the
-- security-definer owner only the non-semantic timestamp column required for
-- that lock; private grading identity and payload remain non-updatable.
GRANT UPDATE(created_at) ON TABLE public.workspace_flat_question_grading TO ple_grader;
