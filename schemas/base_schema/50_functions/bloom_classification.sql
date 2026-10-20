-- Server-only initial Bloom assignment. No new classification state or queue.

SET LOCAL ROLE ple_private_owner;

CREATE FUNCTION ple_private.bloom_classification_candidate(
    p_kind text, p_id text, p_revision integer
) RETURNS SETOF jsonb
LANGUAGE sql SECURITY DEFINER
SET search_path = pg_catalog, ple_data, ple_private AS $$
    WITH candidates AS (
        SELECT 'question'::text AS kind, m.published_question_id::text AS id,
               m.revision_number AS revision, m.metadata_edit_number AS edit_number,
               0::bigint AS membership_edit_number,
               m.question_title AS title, m.question_description AS description
          FROM ple_data.question_revision_metadata AS m
          JOIN ple_data.published_question AS q USING (published_question_id)
         WHERE m.bloom_cognitive_process IS NULL AND m.bloom_knowledge_dimension IS NULL
           AND q.availability = 'available'
        UNION ALL
        SELECT 'pool', p.question_pool_id::text, 0, p.question_pool_metadata_edit_number,
               p.question_pool_edit_number, p.title, p.description
          FROM ple_data.question_pool AS p
    ), page AS (
        SELECT * FROM candidates
         WHERE kind = p_kind AND id = p_id AND revision = p_revision
    )
    SELECT jsonb_build_object(
        'kind', c.kind, 'id', c.id, 'revision', c.revision, 'edit_number', c.edit_number,
        'membership_edit_number', c.membership_edit_number,
        'title', c.title, 'description', c.description,
        'cognitive', CASE WHEN c.kind = 'pool' THEN (
            SELECT m.bloom_cognitive_process
              FROM ple_data.question_pool_member AS pm
              JOIN ple_data.question_revision_metadata AS m
                ON m.published_question_id = pm.published_question_id
               AND m.revision_number = pm.question_revision_number
             WHERE pm.question_pool_id = c.id AND m.bloom_cognitive_process IS NOT NULL
             GROUP BY m.bloom_cognitive_process
             ORDER BY count(*) DESC, m.bloom_cognitive_process LIMIT 1) END,
        'knowledge', CASE WHEN c.kind = 'pool' THEN (
            SELECT m.bloom_knowledge_dimension
              FROM ple_data.question_pool_member AS pm
              JOIN ple_data.question_revision_metadata AS m
                ON m.published_question_id = pm.published_question_id
               AND m.revision_number = pm.question_revision_number
             WHERE pm.question_pool_id = c.id AND m.bloom_knowledge_dimension IS NOT NULL
             GROUP BY m.bloom_knowledge_dimension
             ORDER BY count(*) DESC, m.bloom_knowledge_dimension LIMIT 1) END,
        'sources', COALESCE((SELECT jsonb_agg(jsonb_build_object(
            'question_id', member.id, 'revision', member.revision,
            'backend', b.backend, 'object_id', b.source_object_record_id,
            'checksum', b.source_object_checksum, 'webwork_pg_path', b.webwork_pg_path
        ) ORDER BY member.id, member.revision)
          FROM (
              SELECT c.id AS id, c.revision AS revision WHERE c.kind = 'question'
          ) AS member
          LEFT JOIN ple_private.question_revision_source_binding AS b
            ON b.published_question_id = member.id AND b.revision_number = member.revision), '[]'::jsonb)
    ) FROM page AS c ORDER BY c.kind, c.id, c.revision
$$;

CREATE FUNCTION ple_private.save_bloom_classification(
    p_kind text, p_id text, p_revision integer, p_edit_number bigint,
    p_membership_edit_number bigint, p_cognitive text, p_knowledge text
) RETURNS boolean
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_data, ple_private AS $$
BEGIN
    IF p_kind = 'question' AND (p_cognitive IS NULL OR p_knowledge IS NULL) THEN
        RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Initial Bloom requires both dimensions';
    END IF;
    IF p_kind = 'question' THEN
        UPDATE ple_data.question_revision_metadata AS m
           SET bloom_cognitive_process = p_cognitive::ple_data.bloom_cognitive_process,
               bloom_knowledge_dimension = p_knowledge::ple_data.bloom_knowledge_dimension,
               metadata_edit_number = m.metadata_edit_number + 1,
               updated_at = pg_catalog.clock_timestamp()
         WHERE m.published_question_id = p_id AND m.revision_number = p_revision
           AND m.metadata_edit_number = p_edit_number
           AND m.bloom_cognitive_process IS NULL AND m.bloom_knowledge_dimension IS NULL;
    ELSIF p_kind = 'pool' THEN
        UPDATE ple_data.question_pool AS p
           SET bloom_cognitive_process = p_cognitive::ple_data.bloom_cognitive_process,
               bloom_knowledge_dimension = p_knowledge::ple_data.bloom_knowledge_dimension,
               question_pool_metadata_edit_number = p.question_pool_metadata_edit_number + 1,
               updated_on = CURRENT_DATE
         WHERE p.question_pool_id = p_id
           AND p.question_pool_metadata_edit_number = p_edit_number
           AND p.question_pool_edit_number = p_membership_edit_number
           AND (p.bloom_cognitive_process IS DISTINCT FROM p_cognitive::ple_data.bloom_cognitive_process
                OR p.bloom_knowledge_dimension IS DISTINCT FROM p_knowledge::ple_data.bloom_knowledge_dimension);
    ELSE
        RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Invalid Bloom target';
    END IF;
    RETURN FOUND;
END
$$;

SET LOCAL ROLE ple_api_owner;

CREATE FUNCTION ple_api.bloom_classification_candidate(text, text, integer)
RETURNS SETOF jsonb LANGUAGE sql SECURITY DEFINER
SET search_path = pg_catalog, ple_private AS $$
    SELECT * FROM ple_private.bloom_classification_candidate($1, $2, $3)
$$;

CREATE FUNCTION ple_api.save_bloom_classification(text, text, integer, bigint, bigint, text, text)
RETURNS boolean LANGUAGE sql SECURITY DEFINER
SET search_path = pg_catalog, ple_private AS $$
    SELECT ple_private.save_bloom_classification($1, $2, $3, $4, $5, $6, $7)
$$;
