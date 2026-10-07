-- Ordinary single-Pool metadata operations.

SET LOCAL ROLE ple_private_owner;

CREATE FUNCTION ple_private.replace_question_pool_metadata(
    p_question_pool_id text, p_expected_question_pool_metadata_edit_number bigint,
    p_title text, p_description text, p_topic_uuid uuid, p_subtopic_uuid uuid,
    p_tags text[], p_bloom_cognitive_process ple_data.bloom_cognitive_process,
    p_bloom_knowledge_dimension ple_data.bloom_knowledge_dimension
) RETURNS TABLE(question_pool_id text, question_pool_metadata_edit_number bigint)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
DECLARE
    actor_id text;
    current_pool ple_data.question_pool%ROWTYPE;
BEGIN
    IF p_question_pool_id IS NULL
       OR p_expected_question_pool_metadata_edit_number IS NULL
       OR p_expected_question_pool_metadata_edit_number < 1
       OR p_title IS NULL OR p_title <> btrim(p_title)
       OR char_length(p_title) NOT BETWEEN 1 AND 512 OR p_title ~ '[[:cntrl:]]'
       OR p_description IS NULL OR p_description <> btrim(p_description)
       OR char_length(p_description) NOT BETWEEN 1 AND 4000 OR p_description ~ '[[:cntrl:]]'
       OR p_tags IS NULL OR NOT ple_data.question_metadata_tags_are_valid(p_tags)
       OR (p_subtopic_uuid IS NOT NULL AND p_topic_uuid IS NULL) THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Question Pool metadata replacement is invalid';
    END IF;
    IF NOT (ple_api.current_session_account_is_instructor()
            OR ple_api.current_session_account_is_sysadmin()) THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Question Pool Owner or Sysadmin authority is required';
    END IF;
    actor_id := ple_api.current_session_account_id();
    IF actor_id IS NULL
       OR (ple_api.current_session_account_is_instructor()
           AND ple_private.instructor_display_name(actor_id) IS NULL) THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Question Pool Owner or Sysadmin authority is required';
    END IF;
    SELECT pool.* INTO current_pool
      FROM ple_data.question_pool AS pool
     WHERE pool.question_pool_id = p_question_pool_id
     FOR UPDATE;
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Question Pool metadata target is not available';
    END IF;
    IF NOT ple_api.current_session_account_is_sysadmin()
       AND current_pool.owner_account_id::text IS DISTINCT FROM actor_id THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Question Pool Owner authority is required';
    END IF;
    IF current_pool.question_pool_metadata_edit_number
       <> p_expected_question_pool_metadata_edit_number THEN
        RAISE EXCEPTION USING ERRCODE = '40001',
            MESSAGE = 'Question Pool metadata Edit Number is stale';
    END IF;
    RETURN QUERY
    UPDATE ple_data.question_pool AS pool
       SET title = p_title,
           description = p_description,
           content_topic_id = p_topic_uuid,
           content_subtopic_id = p_subtopic_uuid,
           tags = p_tags,
           bloom_cognitive_process = p_bloom_cognitive_process,
           bloom_knowledge_dimension = p_bloom_knowledge_dimension,
           question_pool_metadata_edit_number = pool.question_pool_metadata_edit_number + 1,
           updated_on = CURRENT_DATE
     WHERE pool.question_pool_id = p_question_pool_id
     RETURNING pool.question_pool_id::text, pool.question_pool_metadata_edit_number;
END
$$;

SET LOCAL ROLE ple_api_owner;

CREATE FUNCTION ple_api.replace_question_pool_metadata(
    p_question_pool_id text, p_expected_question_pool_metadata_edit_number bigint,
    p_title text, p_description text, p_topic_uuid uuid, p_subtopic_uuid uuid,
    p_tags text[], p_bloom_cognitive_process text,
    p_bloom_knowledge_dimension text
) RETURNS TABLE(question_pool_id text, question_pool_metadata_edit_number bigint)
LANGUAGE sql SECURITY DEFINER
SET search_path = pg_catalog, ple_private AS $$
    SELECT * FROM ple_private.replace_question_pool_metadata(
        p_question_pool_id, p_expected_question_pool_metadata_edit_number,
        p_title, p_description, p_topic_uuid, p_subtopic_uuid, p_tags,
        p_bloom_cognitive_process::ple_data.bloom_cognitive_process,
        p_bloom_knowledge_dimension::ple_data.bloom_knowledge_dimension)
$$;

CREATE FUNCTION ple_api.read_current_question_pool_metadata(p_question_pool_id text)
RETURNS TABLE (
    question_pool_id text, question_pool_metadata_edit_number bigint,
    title text, description text, topic_uuid uuid, subtopic_uuid uuid, tags text[],
    bloom_cognitive_process text, bloom_knowledge_dimension text
) LANGUAGE plpgsql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
DECLARE
    actor_id text;
BEGIN
    actor_id := ple_api.current_session_account_id();
    IF actor_id IS NULL
       OR NOT (ple_api.current_session_account_is_instructor()
               OR ple_api.current_session_account_is_sysadmin())
       OR (ple_api.current_session_account_is_instructor()
           AND ple_private.instructor_display_name(actor_id) IS NULL) THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Current Question Pool metadata requires an active Library reader';
    END IF;
    RETURN QUERY
    SELECT pool.question_pool_id::text, pool.question_pool_metadata_edit_number,
           pool.title, pool.description, pool.content_topic_id, pool.content_subtopic_id,
           pool.tags, pool.bloom_cognitive_process::text, pool.bloom_knowledge_dimension::text
      FROM ple_data.question_pool AS pool
     WHERE pool.question_pool_id = p_question_pool_id;
END
$$;

SET LOCAL ROLE ple_private_owner;

CREATE FUNCTION ple_private.bulk_replace_question_pool_search_metadata(
    p_selection jsonb, p_patch jsonb
) RETURNS TABLE(question_pool_id text, question_pool_metadata_edit_number bigint)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
DECLARE
    v_selection_count integer;
    v_distinct_count integer;
    v_normalized_selection jsonb;
    v_normalized_tags text[];
    v_set_tags boolean := false;
    v_set_topic boolean := false;
    v_set_subtopic boolean := false;
    v_classification_key text;
    v_selected record;
    v_current_edit_number bigint;
    v_operation_result jsonb;
BEGIN
    -- ASVS 8.2.1: only an active Instructor session may change Pool search metadata.
    IF p_selection IS NULL OR jsonb_typeof(p_selection) <> 'array'
       OR NOT ple_api.current_session_account_is_instructor() THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Question Pool search metadata requires an active Instructor';
    END IF;
    IF ple_api.current_session_account_id() IS NULL
       OR ple_private.instructor_display_name(ple_api.current_session_account_id()) IS NULL THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Question Pool search metadata requires an active Instructor';
    END IF;
    IF jsonb_array_length(p_selection) NOT BETWEEN 1 AND 1000
       OR EXISTS (
           SELECT 1 FROM jsonb_array_elements(p_selection) AS element(value)
            WHERE jsonb_typeof(value) <> 'object'
               OR NOT (value ? 'questionPoolId' AND value ? 'questionPoolMetadataEditNumber')
               OR EXISTS (SELECT 1 FROM jsonb_object_keys(value) AS key(name)
                           WHERE name NOT IN ('questionPoolId', 'questionPoolMetadataEditNumber'))
               OR jsonb_typeof(value -> 'questionPoolId') <> 'string'
               OR value ->> 'questionPoolId' !~ '^[0-9A-HJKMNP-TV-Z]{4}-[0-9A-HJKMNP-TV-Z]{4}$'
               OR substr(value ->> 'questionPoolId', 6, 1) IS DISTINCT FROM
                    ple_private.crockford_checksum_character(
                        substr(value ->> 'questionPoolId', 1, 4)
                        || substr(value ->> 'questionPoolId', 7, 3)
                    )
               OR jsonb_typeof(value -> 'questionPoolMetadataEditNumber') <> 'number'
               OR value ->> 'questionPoolMetadataEditNumber' !~ '^[1-9][0-9]{0,17}$'
       ) THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Question Pool search metadata selection is invalid';
    END IF;
    SELECT count(*), count(DISTINCT value ->> 'questionPoolId')
      INTO v_selection_count, v_distinct_count
      FROM jsonb_array_elements(p_selection) AS element(value);
    IF v_selection_count <> v_distinct_count THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Question Pool search metadata selection contains duplicate Pool IDs';
    END IF;
    SELECT jsonb_agg(jsonb_build_object(
        'questionPoolId', value ->> 'questionPoolId',
        'questionPoolMetadataEditNumber', (value ->> 'questionPoolMetadataEditNumber')::bigint
    ) ORDER BY value ->> 'questionPoolId')
      INTO v_normalized_selection
      FROM jsonb_array_elements(p_selection) AS element(value);
    -- ASVS 2.2.1: Discipline and Subject are not fields of this command.
    IF p_patch IS NULL OR jsonb_typeof(p_patch) <> 'object'
       OR (SELECT count(*) FROM jsonb_object_keys(p_patch)) NOT BETWEEN 1 AND 3
       OR EXISTS (SELECT 1 FROM jsonb_object_keys(p_patch) AS key(name)
                  WHERE name NOT IN ('tags', 'topicUuid', 'subtopicUuid'))
       OR (p_patch ? 'tags' AND jsonb_typeof(p_patch -> 'tags') <> 'array')
       OR (p_patch ? 'tags' AND EXISTS (
           SELECT 1 FROM jsonb_array_elements(p_patch -> 'tags') AS tag(value)
            WHERE jsonb_typeof(value) <> 'string'
               OR value #>> '{}' <> btrim(value #>> '{}')
               OR char_length(value #>> '{}') NOT BETWEEN 1 AND 120
               OR value #>> '{}' ~ '[[:cntrl:]]'
       ))
       OR (p_patch ? 'tags' AND (
           SELECT count(*) FROM jsonb_array_elements_text(p_patch -> 'tags')
       ) <> (
           SELECT count(DISTINCT value)
             FROM jsonb_array_elements_text(p_patch -> 'tags') AS tag(value)
       ))
       THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Question Pool search metadata patch is invalid';
    END IF;
    FOREACH v_classification_key IN ARRAY ARRAY['topicUuid', 'subtopicUuid']
    LOOP
        IF p_patch ? v_classification_key AND p_patch -> v_classification_key <> 'null'::jsonb AND (
            jsonb_typeof(p_patch -> v_classification_key) <> 'string'
            OR p_patch ->> v_classification_key !~ '^[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}$'
        ) THEN
            RAISE EXCEPTION USING ERRCODE = '22023',
                MESSAGE = 'Question Pool search metadata classification is invalid';
        END IF;
    END LOOP;
    v_set_tags := p_patch ? 'tags';
    v_set_topic := p_patch ? 'topicUuid';
    v_set_subtopic := p_patch ? 'subtopicUuid';
    IF v_set_tags THEN
        SELECT array_agg(value ORDER BY value) INTO v_normalized_tags
          FROM jsonb_array_elements_text(p_patch -> 'tags') AS tag(value);
        v_normalized_tags := coalesce(v_normalized_tags, ARRAY[]::text[]);
        IF NOT ple_data.question_metadata_tags_are_valid(v_normalized_tags) THEN
            RAISE EXCEPTION USING ERRCODE = '22023',
                MESSAGE = 'Question Pool search metadata tags are invalid';
        END IF;
    END IF;
    -- ASVS 2.3.4 and 15.4.2: lock every Pool in canonical ID order before comparing.
    FOR v_selected IN
        SELECT value ->> 'questionPoolId' AS selected_pool_id,
               (value ->> 'questionPoolMetadataEditNumber')::bigint AS expected_edit_number
          FROM jsonb_array_elements(v_normalized_selection) AS element(value)
         ORDER BY value ->> 'questionPoolId'
    LOOP
        SELECT pool.question_pool_metadata_edit_number INTO v_current_edit_number
          FROM ple_data.question_pool AS pool
         WHERE pool.question_pool_id = v_selected.selected_pool_id
           AND pool.owner_account_id::text = ple_api.current_session_account_id()
         FOR UPDATE;
        IF NOT FOUND THEN
            RAISE EXCEPTION USING ERRCODE = '42501',
                MESSAGE = 'Question Pool search metadata target is not available';
        END IF;
        IF v_current_edit_number <> v_selected.expected_edit_number THEN
            RAISE EXCEPTION USING ERRCODE = '40001',
            MESSAGE = 'Question Pool metadata Edit Number is stale';
        END IF;
    END LOOP;
    WITH updated AS (
        UPDATE ple_data.question_pool AS pool
           SET question_pool_metadata_edit_number = pool.question_pool_metadata_edit_number + 1,
               tags = CASE WHEN v_set_tags THEN v_normalized_tags ELSE pool.tags END,
               content_topic_id = CASE
                   WHEN v_set_topic AND p_patch -> 'topicUuid' = 'null'::jsonb THEN NULL
                   WHEN v_set_topic THEN (p_patch ->> 'topicUuid')::uuid
                   ELSE pool.content_topic_id
               END,
               content_subtopic_id = CASE
                   WHEN v_set_topic AND (
                       p_patch -> 'topicUuid' = 'null'::jsonb
                       OR (p_patch ->> 'topicUuid')::uuid IS DISTINCT FROM pool.content_topic_id
                   ) AND NOT (v_set_subtopic AND p_patch -> 'subtopicUuid' <> 'null'::jsonb)
                   THEN NULL
                   WHEN v_set_subtopic AND p_patch -> 'subtopicUuid' = 'null'::jsonb THEN NULL
                   WHEN v_set_subtopic THEN (p_patch ->> 'subtopicUuid')::uuid
                   ELSE pool.content_subtopic_id
               END,
               updated_on = CURRENT_DATE
          FROM (
              SELECT value ->> 'questionPoolId' AS question_pool_id
                FROM jsonb_array_elements(v_normalized_selection) AS element(value)
          ) AS selected
         WHERE pool.question_pool_id = selected.question_pool_id
         RETURNING pool.question_pool_id, pool.question_pool_metadata_edit_number
    )
    SELECT jsonb_agg(jsonb_build_object(
        'questionPoolId', updated.question_pool_id,
        'questionPoolMetadataEditNumber', updated.question_pool_metadata_edit_number
    ) ORDER BY updated.question_pool_id)
      INTO v_operation_result
      FROM updated AS updated;
    IF v_operation_result IS NULL
       OR jsonb_typeof(v_operation_result) <> 'array'
       OR jsonb_array_length(v_operation_result) <> v_selection_count THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Question Pool search metadata target is not available';
    END IF;
    RETURN QUERY
    SELECT result.value ->> 'questionPoolId', (result.value ->> 'questionPoolMetadataEditNumber')::bigint
      FROM jsonb_array_elements(v_operation_result) WITH ORDINALITY AS result(value, position)
     ORDER BY result.position;
END
$$;

SET LOCAL ROLE ple_api_owner;

CREATE FUNCTION ple_api.bulk_replace_question_pool_search_metadata(
    p_selection jsonb, p_patch jsonb
) RETURNS TABLE(question_pool_id text, question_pool_metadata_edit_number bigint)
LANGUAGE sql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private AS $$
    SELECT * FROM ple_private.bulk_replace_question_pool_search_metadata(p_selection, p_patch)
$$;
