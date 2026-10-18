-- ASVS 8.3.1: one trusted, session-authorized predicate for pages and facet counts.
SET LOCAL ROLE ple_api_owner;

CREATE FUNCTION ple_api.matching_blueprint_course_ids(
    p_include_archived boolean, p_active_only boolean, p_promoted_only boolean, p_query text,
    p_discipline_uuid uuid, p_subject_uuid uuid, p_topic_uuid uuid,
    p_subtopic_uuid uuid, p_cross_discipline boolean, p_tag text,
    p_discipline_uuids uuid[], p_subject_uuids uuid[], p_topic_uuids uuid[],
    p_subtopic_uuids uuid[], p_tags text[]
)
RETURNS TABLE (blueprint_course_id text)
LANGUAGE plpgsql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private
AS $$
BEGIN
    IF (p_active_only AND p_include_archived)
       OR p_query IS NULL OR length(p_query) > 256 OR p_query ~ '[[:cntrl:]]'
       OR p_tag IS NULL OR p_tag <> btrim(p_tag) OR length(p_tag) > 120
       OR p_tag ~ '[[:cntrl:]]'
       OR p_discipline_uuids IS NULL OR p_subject_uuids IS NULL
       OR p_topic_uuids IS NULL OR p_subtopic_uuids IS NULL OR p_tags IS NULL
       OR cardinality(p_discipline_uuids) > 20 OR cardinality(p_subject_uuids) > 20
       OR cardinality(p_topic_uuids) > 20 OR cardinality(p_subtopic_uuids) > 20
       OR cardinality(p_tags) > 20 OR array_position(p_discipline_uuids, NULL) IS NOT NULL
       OR array_position(p_subject_uuids, NULL) IS NOT NULL
       OR array_position(p_topic_uuids, NULL) IS NOT NULL
       OR array_position(p_subtopic_uuids, NULL) IS NOT NULL
       OR array_position(p_tags, NULL) IS NOT NULL
       OR EXISTS (SELECT 1 FROM unnest(p_tags) AS tag
                   WHERE length(tag) > 120 OR tag = '' OR tag <> btrim(tag)
                      OR tag ~ '[[:cntrl:]]')
       OR (p_subject_uuid IS NOT NULL AND p_discipline_uuid IS NULL)
       OR (p_topic_uuid IS NOT NULL AND p_subject_uuid IS NULL)
       OR (p_subtopic_uuid IS NOT NULL AND p_topic_uuid IS NULL)
       OR (p_cross_discipline AND (p_discipline_uuid IS NULL OR p_subject_uuid IS NULL)) THEN
        RAISE EXCEPTION 'invalid Blueprint discovery filter' USING ERRCODE = '22023';
    END IF;
    IF (p_discipline_uuid IS NOT NULL AND NOT EXISTS (
            SELECT 1 FROM ple_api.list_content_disciplines_including_retired() AS item
             WHERE item.content_discipline_id = p_discipline_uuid))
       OR (p_subject_uuid IS NOT NULL AND NOT EXISTS (
            SELECT 1 FROM ple_api.list_content_subjects(p_discipline_uuid) AS item
             WHERE item.content_subject_id = p_subject_uuid))
       OR (p_topic_uuid IS NOT NULL AND NOT EXISTS (
            SELECT 1 FROM ple_api.list_content_topics(p_subject_uuid) AS item
             WHERE item.content_topic_id = p_topic_uuid))
       OR (p_subtopic_uuid IS NOT NULL AND NOT EXISTS (
            SELECT 1 FROM ple_api.list_content_subtopics(p_topic_uuid) AS item
             WHERE item.content_subtopic_id = p_subtopic_uuid)) THEN
        RAISE EXCEPTION 'invalid Blueprint classification filter' USING ERRCODE = '22023';
    END IF;
    -- ASVS 1.2.4: query text is a literal LIKE value; arrays remain bound data.
    RETURN QUERY
    SELECT course.blueprint_course_id::text
      FROM ple_data.blueprint_course AS course
     WHERE ple_api.current_session_account_is_instructor()
       AND (NOT p_active_only OR course.availability = 'active')
       AND (NOT p_promoted_only OR course.promoted)
       AND (p_cross_discipline OR p_discipline_uuid IS NULL
            OR course.content_discipline_id = p_discipline_uuid)
       AND (p_subject_uuid IS NULL OR course.content_subject_id = p_subject_uuid)
       AND (p_topic_uuid IS NULL OR course.content_topic_id = p_topic_uuid)
       AND (p_subtopic_uuid IS NULL OR course.content_subtopic_id = p_subtopic_uuid)
       AND (p_tag = '' OR p_tag = ANY(course.tags))
       AND (cardinality(p_discipline_uuids) = 0
            OR course.content_discipline_id = ANY(p_discipline_uuids))
       AND (cardinality(p_subject_uuids) = 0
            OR course.content_subject_id = ANY(p_subject_uuids))
       AND (cardinality(p_topic_uuids) = 0
            OR course.content_topic_id = ANY(p_topic_uuids))
       AND (cardinality(p_subtopic_uuids) = 0
            OR course.content_subtopic_id = ANY(p_subtopic_uuids))
       AND (cardinality(p_tags) = 0 OR course.tags && p_tags)
       AND (p_query = '' OR course.short_name ILIKE
            '%' || replace(replace(replace(p_query, '\', '\\'), '%', '\%'), '_', '\_') || '%'
            OR course.long_name ILIKE
            '%' || replace(replace(replace(p_query, '\', '\\'), '%', '\%'), '_', '\_') || '%')
       AND (course.availability = 'active'
            OR (p_include_archived AND course.availability = 'archived'));
END
$$;

-- Count complete matching courses with the target column's own facet omitted.
-- Selected values are returned separately even when they fall beyond the choice page.
CREATE FUNCTION ple_api.blueprint_course_facet_choices(
    p_field text, p_include_archived boolean, p_active_only boolean,
    p_promoted_only boolean, p_query text, p_discipline_uuid uuid,
    p_subject_uuid uuid, p_topic_uuid uuid, p_subtopic_uuid uuid,
    p_cross_discipline boolean, p_tag text, p_discipline_uuids uuid[],
    p_subject_uuids uuid[], p_topic_uuids uuid[], p_subtopic_uuids uuid[],
    p_tags text[], p_choice_search text, p_after_label text, p_after_key text,
    p_limit integer
)
RETURNS TABLE (choice_key text, choice_label text, choice_count bigint, selected boolean)
LANGUAGE plpgsql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private
AS $$
BEGIN
    IF p_field IS NULL OR p_field NOT IN ('disciplineUuids', 'subjectUuids', 'topicUuids',
                       'subtopicUuids', 'tags')
       OR p_limit IS NULL OR p_limit < 1 OR p_limit > 101 OR p_choice_search IS NULL
       OR length(p_choice_search) > 128 OR p_choice_search ~ '[[:cntrl:]]'
       OR (p_after_label IS NULL) <> (p_after_key IS NULL)
       OR length(p_after_label) > 500 OR length(p_after_key) > 120 THEN
        RAISE EXCEPTION 'invalid Blueprint facet page' USING ERRCODE = '22023';
    END IF;
    RETURN QUERY
    WITH hits AS MATERIALIZED (
        SELECT hit.blueprint_course_id
          FROM ple_api.matching_blueprint_course_ids(
              p_include_archived, p_active_only, p_promoted_only, p_query,
              p_discipline_uuid, p_subject_uuid, p_topic_uuid, p_subtopic_uuid,
              p_cross_discipline, p_tag,
              CASE WHEN p_field = 'disciplineUuids' THEN ARRAY[]::uuid[] ELSE p_discipline_uuids END,
              CASE WHEN p_field = 'subjectUuids' THEN ARRAY[]::uuid[] ELSE p_subject_uuids END,
              CASE WHEN p_field = 'topicUuids' THEN ARRAY[]::uuid[] ELSE p_topic_uuids END,
              CASE WHEN p_field = 'subtopicUuids' THEN ARRAY[]::uuid[] ELSE p_subtopic_uuids END,
              CASE WHEN p_field = 'tags' THEN ARRAY[]::text[] ELSE p_tags END
          ) AS hit
    ), values_by_course AS (
        SELECT hit.blueprint_course_id, value.choice_key
          FROM hits AS hit
          JOIN ple_data.blueprint_course AS course
            ON course.blueprint_course_id = hit.blueprint_course_id
          CROSS JOIN LATERAL unnest(CASE p_field
              WHEN 'disciplineUuids' THEN ARRAY[course.content_discipline_id::text]
              WHEN 'subjectUuids' THEN ARRAY[course.content_subject_id::text]
              WHEN 'topicUuids' THEN ARRAY[course.content_topic_id::text]
              WHEN 'subtopicUuids' THEN ARRAY[course.content_subtopic_id::text]
              ELSE course.tags END) AS value(choice_key)
         WHERE value.choice_key IS NOT NULL
    ), counted AS (
        SELECT value.choice_key, count(DISTINCT value.blueprint_course_id)::bigint AS choice_count
          FROM values_by_course AS value GROUP BY value.choice_key
    ), selected_keys AS (
        SELECT DISTINCT key.choice_key FROM unnest(CASE p_field
            WHEN 'disciplineUuids' THEN ARRAY(SELECT value::text FROM unnest(p_discipline_uuids) AS value)
            WHEN 'subjectUuids' THEN ARRAY(SELECT value::text FROM unnest(p_subject_uuids) AS value)
            WHEN 'topicUuids' THEN ARRAY(SELECT value::text FROM unnest(p_topic_uuids) AS value)
            WHEN 'subtopicUuids' THEN ARRAY(SELECT value::text FROM unnest(p_subtopic_uuids) AS value)
            ELSE p_tags END) AS key(choice_key)
    ), all_keys AS (
        SELECT counted.choice_key FROM counted
        UNION SELECT selected_keys.choice_key FROM selected_keys
    ), labeled AS (
        SELECT key.choice_key,
               COALESCE(
                   CASE WHEN p_field <> 'tags' THEN
                       ple_private.content_classification_facet_name(p_field, key.choice_key)
                   END, key.choice_key) AS choice_label,
               COALESCE(counted.choice_count, 0)::bigint AS choice_count,
               selected_keys.choice_key IS NOT NULL AS selected
          FROM all_keys AS key
          LEFT JOIN counted ON counted.choice_key = key.choice_key
          LEFT JOIN selected_keys ON selected_keys.choice_key = key.choice_key
    ), choice_page AS (
        SELECT labeled.* FROM labeled
         WHERE (p_choice_search = '' OR lower(labeled.choice_label) LIKE
                '%' || replace(replace(replace(p_choice_search, '\', '\\'), '%', '\%'), '_', '\_') || '%')
           AND (p_after_key IS NULL OR
                (labeled.choice_label COLLATE "C", labeled.choice_key COLLATE "C") >
                (p_after_label COLLATE "C", p_after_key COLLATE "C"))
         ORDER BY labeled.choice_label COLLATE "C", labeled.choice_key COLLATE "C"
         LIMIT p_limit
    )
    SELECT page.choice_key, page.choice_label, page.choice_count, false
      FROM choice_page AS page
    UNION ALL
    SELECT labeled.choice_key, labeled.choice_label, labeled.choice_count, true
      FROM labeled WHERE labeled.selected;
END
$$;
