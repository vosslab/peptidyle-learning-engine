-- Current Question and Pool titles for Instructor recognition.
-- Titles stay on the lineage. Blueprint documents keep public IDs only.

SET LOCAL ROLE ple_api_owner;

CREATE FUNCTION ple_api.load_recognition_titles(
    p_question_ids text[],
    p_pool_ids text[]
) RETURNS TABLE (
    record_kind text,
    public_id text,
    title text
)
LANGUAGE plpgsql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data AS $$
DECLARE
    question_count integer;
    pool_count integer;
BEGIN
    -- ASVS 8.2.1: the installed session is the caller. A missing lineage is omitted.
    IF ple_api.current_session_account_id() IS NULL
       OR NOT ple_api.current_session_account_is_instructor() THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Recognition titles require an active Instructor Account';
    END IF;
    IF p_question_ids IS NULL OR p_pool_ids IS NULL THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Recognition title selection is invalid';
    END IF;
    question_count := cardinality(p_question_ids);
    pool_count := cardinality(p_pool_ids);
    IF question_count > 1000
       OR pool_count > 1000
       OR (question_count = 0 AND pool_count = 0)
       OR question_count <> (
           SELECT count(DISTINCT requested.public_id)
             FROM unnest(p_question_ids) AS requested(public_id)
       )
       OR pool_count <> (
           SELECT count(DISTINCT requested.public_id)
             FROM unnest(p_pool_ids) AS requested(public_id)
       ) THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Recognition title selection is invalid';
    END IF;
    RETURN QUERY
    SELECT 'question'::text,
           metadata.published_question_id::text,
           metadata.question_title
      FROM ple_data.published_question_metadata AS metadata
     WHERE metadata.published_question_id = ANY (p_question_ids::ple_data.question_family_id[]);
    RETURN QUERY
    SELECT 'pool'::text,
           pool.question_pool_id::text,
           pool.title
      FROM ple_data.question_pool AS pool
     WHERE pool.question_pool_id = ANY (p_pool_ids::ple_data.question_family_id[]);
END
$$;
