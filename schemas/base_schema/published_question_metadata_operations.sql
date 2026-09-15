-- Bulk metadata is deliberately a narrow Question Library command.  It does
-- not accept source, Revision, availability, ownership, or arbitrary JSON
-- mutations.  One transaction locks and validates the whole selection before
-- writing, then returns its result in canonical Question-ID order (ASVS 2.3.3).
SET LOCAL ROLE ple_private_owner;
CREATE FUNCTION ple_private.bulk_replace_published_question_metadata(
    p_selection jsonb, p_patch jsonb
) RETURNS TABLE(question_id text, metadata_edit_number bigint)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
DECLARE
    actor_id uuid;
    selection_count integer;
    distinct_count integer;
    normalized_selection jsonb;
    normalized_patch jsonb;
    operation_result jsonb;
    selected record;
    current_metadata_edit_number bigint;
    current_availability text;
    normalized_tags text[];
    set_tags boolean := false;
    set_subject boolean := false;
    set_topic boolean := false;
BEGIN
    IF p_selection IS NULL OR jsonb_typeof(p_selection) <> 'array'
       OR NOT ple_api.current_session_account_is_instructor() THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Bulk Published Question metadata requires an active Instructor';
    END IF;
    actor_id := ple_api.current_session_account_id();
    IF actor_id IS NULL THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Bulk Published Question metadata requires an active Instructor';
    END IF;
    IF ple_private.verified_instructor_display_name(actor_id) IS NULL THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Bulk Published Question metadata requires a vetted Instructor';
    END IF;
    IF jsonb_array_length(p_selection) NOT BETWEEN 1 AND 1000
       OR EXISTS (
           SELECT 1 FROM jsonb_array_elements(p_selection) AS element(value)
            WHERE jsonb_typeof(value) <> 'object'
               OR NOT (value ? 'questionId' AND value ? 'metadataEditNumber')
               OR EXISTS (SELECT 1 FROM jsonb_object_keys(value) AS key(name)
                           WHERE name NOT IN ('questionId', 'metadataEditNumber'))
               OR jsonb_typeof(value -> 'questionId') <> 'string'
               OR value ->> 'questionId' !~ '^[0-9A-HJKMNP-TV-Z]{8}$'
               OR jsonb_typeof(value -> 'metadataEditNumber') <> 'number'
               OR value ->> 'metadataEditNumber' !~ '^[1-9][0-9]{0,17}$'
       ) THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Bulk Published Question metadata selection is invalid';
    END IF;
    SELECT count(*), count(DISTINCT value ->> 'questionId')
      INTO selection_count, distinct_count
      FROM jsonb_array_elements(p_selection) AS element(value);
    IF selection_count <> distinct_count THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Bulk Published Question metadata selection contains duplicate Question IDs';
    END IF;
    SELECT jsonb_agg(jsonb_build_object(
        'questionId', value ->> 'questionId',
        'metadataEditNumber', (value ->> 'metadataEditNumber')::bigint
    ) ORDER BY value ->> 'questionId')
      INTO normalized_selection
      FROM jsonb_array_elements(p_selection) AS element(value);

    IF p_patch IS NULL OR jsonb_typeof(p_patch) <> 'object'
       OR (SELECT count(*) FROM jsonb_object_keys(p_patch)) NOT BETWEEN 1 AND 3
       OR EXISTS (SELECT 1 FROM jsonb_object_keys(p_patch) AS key(name)
                  WHERE name NOT IN ('tags', 'subject', 'topic'))
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
       OR (p_patch ? 'subject' AND p_patch -> 'subject' <> 'null'::jsonb
           AND (jsonb_typeof(p_patch -> 'subject') <> 'string'
                OR p_patch ->> 'subject' <> btrim(p_patch ->> 'subject')
                OR char_length(p_patch ->> 'subject') NOT BETWEEN 1 AND 120
                OR p_patch ->> 'subject' ~ '[[:cntrl:]]'))
       OR (p_patch ? 'topic' AND p_patch -> 'topic' <> 'null'::jsonb
           AND (jsonb_typeof(p_patch -> 'topic') <> 'string'
                OR p_patch ->> 'topic' <> btrim(p_patch ->> 'topic')
                OR char_length(p_patch ->> 'topic') NOT BETWEEN 1 AND 120
                OR p_patch ->> 'topic' ~ '[[:cntrl:]]')) THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Bulk Published Question metadata patch is invalid';
    END IF;
    set_tags := p_patch ? 'tags';
    set_subject := p_patch ? 'subject';
    set_topic := p_patch ? 'topic';
    IF set_tags THEN
        SELECT array_agg(value ORDER BY value) INTO normalized_tags
          FROM jsonb_array_elements_text(p_patch -> 'tags') AS tag(value);
        normalized_tags := coalesce(normalized_tags, ARRAY[]::text[]);
        IF NOT ple_data.question_metadata_tags_are_valid(normalized_tags) THEN
            RAISE EXCEPTION USING ERRCODE = '22023',
                MESSAGE = 'Bulk Published Question metadata tags are invalid';
        END IF;
        normalized_patch := jsonb_set(p_patch, ARRAY['tags'], to_jsonb(normalized_tags), false);
    ELSE
        normalized_patch := p_patch;
    END IF;
    -- Lock every target in canonical Question-ID order (ASVS 2.3.4) and
    -- validate all of them before the first write. An unavailable, missing, or forbidden
    -- target deliberately has the same whole-operation refusal surface.
    FOR selected IN
        SELECT value ->> 'questionId' AS selected_question_id,
               (value ->> 'metadataEditNumber')::bigint AS expected_metadata_edit_number
          FROM jsonb_array_elements(normalized_selection) AS element(value)
         ORDER BY value ->> 'questionId'
    LOOP
        SELECT metadata.metadata_edit_number, lineage.availability
          INTO current_metadata_edit_number, current_availability
          FROM ple_data.published_question_metadata AS metadata
          JOIN ple_data.published_question AS lineage ON lineage.question_id = metadata.question_id
         WHERE metadata.question_id = selected.selected_question_id
         FOR UPDATE OF metadata, lineage;
        IF NOT FOUND OR current_availability <> 'available' THEN
            RAISE EXCEPTION USING ERRCODE = '42501',
                MESSAGE = 'Bulk Published Question metadata target is not available';
        END IF;
        IF current_metadata_edit_number <> selected.expected_metadata_edit_number THEN
            RAISE EXCEPTION USING ERRCODE = '40001',
                MESSAGE = 'Bulk Published Question metadata Edit Number is stale';
        END IF;
    END LOOP;

    WITH updated AS (
        UPDATE ple_data.published_question_metadata AS metadata
           SET tags = CASE WHEN set_tags THEN normalized_tags ELSE metadata.tags END,
               subject = CASE WHEN set_subject THEN normalized_patch ->> 'subject' ELSE metadata.subject END,
               topic = CASE WHEN set_topic THEN normalized_patch ->> 'topic' ELSE metadata.topic END,
               metadata_edit_number = metadata.metadata_edit_number + 1,
               updated_at = pg_catalog.clock_timestamp()
         WHERE metadata.question_id IN (
             SELECT value ->> 'questionId'
               FROM jsonb_array_elements(normalized_selection) AS element(value)
         )
         RETURNING metadata.question_id, metadata.metadata_edit_number
    )
    SELECT jsonb_agg(jsonb_build_object(
        'questionId', updated.question_id,
        'metadataEditNumber', updated.metadata_edit_number
    ) ORDER BY updated.question_id)
      INTO operation_result
      FROM updated AS updated;
    RETURN QUERY
    SELECT result.value ->> 'questionId', (result.value ->> 'metadataEditNumber')::bigint
      FROM jsonb_array_elements(operation_result) WITH ORDINALITY AS result(value, position)
     ORDER BY result.position;
END
$$;
REVOKE ALL ON FUNCTION ple_private.bulk_replace_published_question_metadata(jsonb, jsonb)
    FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_private.bulk_replace_published_question_metadata(jsonb, jsonb)
    TO ple_api_owner;
RESET ROLE;

SET LOCAL ROLE ple_api_owner;
CREATE FUNCTION ple_api.bulk_replace_published_question_metadata(
    p_selection jsonb, p_patch jsonb
) RETURNS TABLE(question_id text, metadata_edit_number bigint)
LANGUAGE sql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private AS $$
    SELECT * FROM ple_private.bulk_replace_published_question_metadata(
        p_selection, p_patch)
$$;
REVOKE ALL ON FUNCTION ple_api.bulk_replace_published_question_metadata(jsonb, jsonb)
    FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_api.bulk_replace_published_question_metadata(jsonb, jsonb)
    TO ple_app;
RESET ROLE;
