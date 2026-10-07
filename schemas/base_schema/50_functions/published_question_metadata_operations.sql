-- Functions, triggers, and views from published_question_metadata_operations.sql.

SET LOCAL ROLE ple_private_owner;

-- Ordinary owner/Sysadmin replacement for the metadata child of the current
-- Question Revision. The Revision Tuple and metadata Edit Number are both
-- checked while the lineage and metadata row are locked (ASVS 2.3.3, 8.3.1).
CREATE FUNCTION ple_private.replace_published_question_metadata(
    p_published_question_id text,
    p_expected_question_revision_number integer,
    p_expected_metadata_edit_number bigint,
    p_question_title text,
    p_question_description text,
    p_language text,
    p_tags text[],
    p_content_discipline_id uuid,
    p_content_subject_id uuid,
    p_content_topic_id uuid,
    p_content_subtopic_id uuid,
    p_question_type ple_data.question_type,
    p_bloom_cognitive_process ple_data.bloom_cognitive_process,
    p_bloom_knowledge_dimension ple_data.bloom_knowledge_dimension
) RETURNS TABLE(published_question_id text, revision_number integer, metadata_edit_number bigint)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
DECLARE
    actor_id text;
    current_revision_number integer;
    current_availability text;
    current_discipline_id uuid;
    current_backend ple_data.question_backend;
    native_question_type ple_data.question_type;
BEGIN
    IF p_published_question_id IS NULL
       OR p_expected_question_revision_number IS NULL OR p_expected_question_revision_number < 1
       OR p_expected_metadata_edit_number IS NULL OR p_expected_metadata_edit_number < 1
       OR p_question_title IS NULL OR p_question_title <> btrim(p_question_title)
       OR char_length(p_question_title) NOT BETWEEN 1 AND 512 OR p_question_title ~ '[[:cntrl:]]'
       OR p_question_description IS NULL OR p_question_description <> btrim(p_question_description)
       OR char_length(p_question_description) NOT BETWEEN 1 AND 4000 OR p_question_description ~ '[[:cntrl:]]'
       OR (p_language IS NOT NULL AND (
           p_language <> btrim(p_language) OR char_length(p_language) NOT BETWEEN 2 AND 35
       ))
       OR p_tags IS NULL OR NOT ple_data.question_metadata_tags_are_valid(p_tags)
       OR p_content_discipline_id IS NULL OR p_content_subject_id IS NULL
       OR p_question_type IS NULL
       OR (p_content_subtopic_id IS NOT NULL AND p_content_topic_id IS NULL) THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Published Question metadata replacement is invalid';
    END IF;
    IF NOT (ple_api.current_session_account_is_instructor() OR ple_api.current_session_account_is_sysadmin()) THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Question Owner or Sysadmin authority is required';
    END IF;
    actor_id := ple_api.current_session_account_id();
    IF actor_id IS NULL THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Question Owner or Sysadmin authority is required';
    END IF;
    SELECT lineage.availability::text INTO current_availability
      FROM ple_data.published_question AS lineage
     WHERE lineage.published_question_id = p_published_question_id
     FOR UPDATE;
    IF NOT FOUND OR current_availability <> 'available' THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Published Question metadata target is not available';
    END IF;
    IF NOT ple_api.current_session_account_is_sysadmin()
       AND NOT EXISTS (
           SELECT 1 FROM ple_data.question_current_owner
            WHERE question_current_owner.published_question_id = p_published_question_id
              AND owner_account_id = actor_id
       ) THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Question Owner authority is required';
    END IF;
    SELECT max(revision.revision_number) INTO current_revision_number
      FROM ple_data.question_revision AS revision
     WHERE revision.published_question_id = p_published_question_id;
    IF current_revision_number IS DISTINCT FROM p_expected_question_revision_number THEN
        RAISE EXCEPTION USING ERRCODE = '40001', MESSAGE = 'Published Question Revision Tuple is stale';
    END IF;
    SELECT source_binding.backend, source_binding.native_question_type
      INTO current_backend, native_question_type
      FROM ple_private.question_revision_source_binding AS source_binding
     WHERE source_binding.published_question_id = p_published_question_id
       AND source_binding.revision_number = p_expected_question_revision_number;
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '40001', MESSAGE = 'Published Question Revision Tuple is stale';
    END IF;
    IF current_backend = 'ple' AND native_question_type IS DISTINCT FROM p_question_type THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Native Question Type must match its source interaction';
    END IF;
    SELECT metadata.content_discipline_id INTO current_discipline_id
      FROM ple_data.question_revision_metadata AS metadata
     WHERE metadata.published_question_id = p_published_question_id
       AND metadata.revision_number = p_expected_question_revision_number
     FOR UPDATE;
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '40001', MESSAGE = 'Published Question Revision Tuple is stale';
    END IF;
    IF current_discipline_id IS DISTINCT FROM p_content_discipline_id THEN
        PERFORM ple_private.require_active_content_discipline(p_content_discipline_id);
    END IF;
    RETURN QUERY
    UPDATE ple_data.question_revision_metadata AS metadata
       SET question_title = p_question_title,
           question_description = p_question_description,
           language = COALESCE(p_language, metadata.language),
           tags = p_tags,
           content_discipline_id = p_content_discipline_id,
           content_subject_id = p_content_subject_id,
           content_topic_id = p_content_topic_id,
           content_subtopic_id = p_content_subtopic_id,
           question_type = p_question_type,
           bloom_cognitive_process = p_bloom_cognitive_process,
           bloom_knowledge_dimension = p_bloom_knowledge_dimension,
           metadata_edit_number = metadata.metadata_edit_number + 1,
           updated_at = pg_catalog.clock_timestamp()
     WHERE metadata.published_question_id = p_published_question_id
       AND metadata.revision_number = p_expected_question_revision_number
       AND metadata.metadata_edit_number = p_expected_metadata_edit_number
     RETURNING metadata.published_question_id::text, metadata.revision_number,
               metadata.metadata_edit_number;
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '40001', MESSAGE = 'Published Question metadata Edit Number is stale';
    END IF;
END
$$;

SET LOCAL ROLE ple_api_owner;

-- Public enum labels arrive as bound text and are cast at this trusted SQL
-- boundary before the unchanged private typed operation (ASVS 1.2.4).
CREATE FUNCTION ple_api.replace_published_question_metadata(
    p_published_question_id text,
    p_expected_question_revision_number integer,
    p_expected_metadata_edit_number bigint,
    p_question_title text,
    p_question_description text,
    p_language text,
    p_tags text[],
    p_content_discipline_id uuid,
    p_content_subject_id uuid,
    p_content_topic_id uuid,
    p_content_subtopic_id uuid,
    p_question_type text,
    p_bloom_cognitive_process text,
    p_bloom_knowledge_dimension text
) RETURNS TABLE(published_question_id text, revision_number integer, metadata_edit_number bigint)
LANGUAGE sql SECURITY DEFINER
SET search_path = pg_catalog, ple_private AS $$
    SELECT * FROM ple_private.replace_published_question_metadata(
        p_published_question_id, p_expected_question_revision_number,
        p_expected_metadata_edit_number, p_question_title, p_question_description,
        p_language, p_tags, p_content_discipline_id, p_content_subject_id,
        p_content_topic_id, p_content_subtopic_id,
        p_question_type::ple_data.question_type,
        p_bloom_cognitive_process::ple_data.bloom_cognitive_process,
        p_bloom_knowledge_dimension::ple_data.bloom_knowledge_dimension)
$$;

CREATE FUNCTION ple_private.bulk_replace_published_question_metadata(
    p_selection jsonb, p_patch jsonb
) RETURNS TABLE(published_question_id text, metadata_edit_number bigint)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
DECLARE
    actor_id text;
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
    classification_key text;
    set_discipline boolean := false;
    set_subject boolean := false;
    set_topic boolean := false;
    set_subtopic boolean := false;
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
    IF ple_private.instructor_display_name(actor_id) IS NULL THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Bulk Published Question metadata requires an active Instructor';
    END IF;
    IF jsonb_array_length(p_selection) NOT BETWEEN 1 AND 1000
       OR EXISTS (
           SELECT 1 FROM jsonb_array_elements(p_selection) AS element(value)
            WHERE jsonb_typeof(value) <> 'object'
               OR NOT (value ? 'questionId' AND value ? 'metadataEditNumber')
               OR EXISTS (SELECT 1 FROM jsonb_object_keys(value) AS key(name)
                           WHERE name NOT IN (
                               'questionId', 'metadataEditNumber', 'questionTitle', 'questionDescription'
                           ))
               OR jsonb_typeof(value -> 'questionId') <> 'string'
               OR value ->> 'questionId' !~ '^[0-9A-HJKMNP-TV-Z]{4}-[0-9A-HJKMNP-TV-Z]{4}$'
               OR substr(value ->> 'questionId', 6, 1) IS DISTINCT FROM
                    ple_private.crockford_checksum_character(
                        substr(value ->> 'questionId', 1, 4)
                        || substr(value ->> 'questionId', 7, 3)
                    )
               OR jsonb_typeof(value -> 'metadataEditNumber') <> 'number'
               OR value ->> 'metadataEditNumber' !~ '^[1-9][0-9]{0,17}$'
               OR (value ? 'questionTitle' AND (
                   jsonb_typeof(value -> 'questionTitle') <> 'string'
                   OR value ->> 'questionTitle' <> btrim(value ->> 'questionTitle')
                   OR char_length(value ->> 'questionTitle') NOT BETWEEN 1 AND 512
                   OR value ->> 'questionTitle' ~ '[[:cntrl:]]'
               ))
               OR (value ? 'questionDescription' AND (
                   jsonb_typeof(value -> 'questionDescription') <> 'string'
                   OR value ->> 'questionDescription' <> btrim(value ->> 'questionDescription')
                   OR char_length(value ->> 'questionDescription') NOT BETWEEN 1 AND 4000
                   OR value ->> 'questionDescription' ~ '[[:cntrl:]]'
               ))
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
    SELECT jsonb_agg(jsonb_strip_nulls(jsonb_build_object(
        'questionId', value ->> 'questionId',
        'metadataEditNumber', (value ->> 'metadataEditNumber')::bigint,
        'questionTitle', CASE WHEN value ? 'questionTitle' THEN value -> 'questionTitle' END,
        'questionDescription', CASE WHEN value ? 'questionDescription' THEN value -> 'questionDescription' END
    )) ORDER BY value ->> 'questionId')
      INTO normalized_selection
      FROM jsonb_array_elements(p_selection) AS element(value);

    IF p_patch IS NULL OR jsonb_typeof(p_patch) <> 'object'
       OR (SELECT count(*) FROM jsonb_object_keys(p_patch)) > 5
       OR EXISTS (SELECT 1 FROM jsonb_object_keys(p_patch) AS key(name)
                  WHERE name NOT IN ('tags', 'disciplineUuid', 'subjectUuid', 'topicUuid', 'subtopicUuid'))
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
            MESSAGE = 'Bulk Published Question metadata patch is invalid';
    END IF;
    set_tags := p_patch ? 'tags';
    -- ASVS 2.2.1/2.2.2: UUID selections are data, not vocabulary names.
    FOREACH classification_key IN ARRAY ARRAY['disciplineUuid', 'subjectUuid', 'topicUuid', 'subtopicUuid']
    LOOP
        IF p_patch ? classification_key AND (
            (p_patch -> classification_key = 'null'::jsonb
             AND classification_key IN ('disciplineUuid', 'subjectUuid'))
            OR (p_patch -> classification_key <> 'null'::jsonb AND (
                jsonb_typeof(p_patch -> classification_key) <> 'string'
                OR p_patch ->> classification_key !~ '^[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}$'
            ))
        ) THEN
            RAISE EXCEPTION USING ERRCODE = '22023',
                MESSAGE = 'Bulk Published Question classification selection is invalid';
        END IF;
    END LOOP;
    set_discipline := p_patch ? 'disciplineUuid';
    set_subject := p_patch ? 'subjectUuid';
    set_topic := p_patch ? 'topicUuid';
    set_subtopic := p_patch ? 'subtopicUuid';
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
    IF (SELECT count(*) FROM jsonb_object_keys(normalized_patch)) = 0
       AND NOT EXISTS (
           SELECT 1 FROM jsonb_array_elements(normalized_selection) AS element(value)
            WHERE value ? 'questionTitle' OR value ? 'questionDescription'
       ) THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Bulk Published Question metadata patch is invalid';
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
          FROM ple_data.question_revision_metadata AS metadata
          JOIN ple_data.published_question AS lineage ON lineage.published_question_id = metadata.published_question_id
         WHERE metadata.published_question_id = selected.selected_question_id
           AND metadata.revision_number = (
               SELECT max(revision.revision_number)
                 FROM ple_data.question_revision AS revision
                WHERE revision.published_question_id = metadata.published_question_id
           )
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
    IF set_discipline AND EXISTS (
        SELECT 1
          FROM ple_data.question_revision_metadata AS metadata
         WHERE metadata.published_question_id IN (
             SELECT value ->> 'questionId'
               FROM jsonb_array_elements(normalized_selection) AS element(value)
         )
           AND metadata.revision_number = (
               SELECT max(revision.revision_number)
                 FROM ple_data.question_revision AS revision
                WHERE revision.published_question_id = metadata.published_question_id
           )
           AND metadata.content_discipline_id IS DISTINCT FROM (normalized_patch ->> 'disciplineUuid')::uuid
    ) THEN
        PERFORM ple_private.require_active_content_discipline(
            (normalized_patch ->> 'disciplineUuid')::uuid);
    END IF;

    WITH updated AS (
        UPDATE ple_data.question_revision_metadata AS metadata
           SET question_title = COALESCE(replacement.question_title, metadata.question_title),
               question_description = COALESCE(replacement.question_description, metadata.question_description),
               tags = CASE WHEN set_tags THEN normalized_tags ELSE metadata.tags END,
               content_discipline_id = CASE WHEN set_discipline THEN (normalized_patch ->> 'disciplineUuid')::uuid ELSE metadata.content_discipline_id END,
               content_subject_id = CASE WHEN set_subject THEN (normalized_patch ->> 'subjectUuid')::uuid ELSE metadata.content_subject_id END,
               content_topic_id = CASE WHEN set_topic THEN (normalized_patch ->> 'topicUuid')::uuid ELSE metadata.content_topic_id END,
               content_subtopic_id = CASE WHEN set_subtopic THEN (normalized_patch ->> 'subtopicUuid')::uuid ELSE metadata.content_subtopic_id END,
               metadata_edit_number = metadata.metadata_edit_number + 1,
               updated_at = pg_catalog.clock_timestamp()
          FROM (
              SELECT value ->> 'questionId' AS published_question_id,
                     CASE WHEN value ? 'questionTitle' THEN value ->> 'questionTitle' END AS question_title,
                     CASE WHEN value ? 'questionDescription' THEN value ->> 'questionDescription' END AS question_description
                FROM jsonb_array_elements(normalized_selection) AS element(value)
          ) AS replacement
         WHERE metadata.published_question_id = replacement.published_question_id
           AND metadata.revision_number = (
               SELECT max(revision.revision_number)
                 FROM ple_data.question_revision AS revision
                WHERE revision.published_question_id = metadata.published_question_id
           )
         RETURNING metadata.published_question_id, metadata.metadata_edit_number
    )
    SELECT jsonb_agg(jsonb_build_object(
        'questionId', updated.published_question_id,
        'metadataEditNumber', updated.metadata_edit_number
    ) ORDER BY updated.published_question_id)
      INTO operation_result
      FROM updated AS updated;
    RETURN QUERY
    SELECT result.value ->> 'questionId', (result.value ->> 'metadataEditNumber')::bigint
      FROM jsonb_array_elements(operation_result) WITH ORDINALITY AS result(value, position)
     ORDER BY result.position;
END
$$;

SET LOCAL ROLE ple_api_owner;

CREATE FUNCTION ple_api.bulk_replace_published_question_metadata(
    p_selection jsonb, p_patch jsonb
) RETURNS TABLE(published_question_id text, metadata_edit_number bigint)
LANGUAGE sql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private AS $$
    SELECT * FROM ple_private.bulk_replace_published_question_metadata(
        p_selection, p_patch)
$$;
