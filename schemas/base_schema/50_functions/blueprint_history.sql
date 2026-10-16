-- Whole-result discovery of the two immutable Blueprint history sequences.

SET LOCAL ROLE ple_api_owner;

CREATE FUNCTION ple_api.list_blueprint_history(
    p_blueprint_course_id text, p_kind text, p_query text, p_sort text,
    p_direction text, p_availability text[], p_after_key text,
    p_after_sequence bigint, p_page_size integer
)
RETURNS TABLE (
    continuation_key text, revision_number bigint, recorded_at_ms bigint,
    short_name text, long_name text, availability text,
    content_discipline_id uuid, content_subject_id uuid, content_topic_id uuid,
    content_subtopic_id uuid, tags text[], theme_id text
)
LANGUAGE plpgsql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private
AS $$
BEGIN
    -- ASVS 2.2.1/2: the query and keyset are bounded even for direct SQL callers.
    IF p_kind IS NULL OR p_kind NOT IN ('revisions', 'metadata')
       OR p_sort IS NULL OR p_sort NOT IN ('sequence', 'recorded_at', 'name')
       OR (p_kind = 'revisions' AND p_sort = 'name')
       OR p_direction IS NULL OR p_direction NOT IN ('ascending', 'descending')
       OR p_page_size IS NULL OR p_page_size NOT BETWEEN 1 AND 100
       OR p_query IS NULL OR length(p_query) > 120
       OR p_availability IS NULL OR array_length(p_availability, 1) > 2
       OR EXISTS (SELECT 1 FROM unnest(p_availability) AS value WHERE value IS NULL OR value NOT IN ('public', 'archived'))
       OR (p_kind = 'revisions' AND cardinality(p_availability) > 0)
       OR ((p_after_key IS NULL) <> (p_after_sequence IS NULL))
       OR (p_after_key IS NOT NULL AND (length(p_after_key) > 500 OR p_after_sequence <= 0)) THEN
        RAISE EXCEPTION 'invalid Blueprint history page' USING ERRCODE = '22023';
    END IF;
    -- ASVS 8.2.1/2/3: every page is checked against current ordinary visibility.
    IF NOT EXISTS (
        SELECT 1 FROM ple_data.blueprint_course AS course
         WHERE ple_private.is_canonical_prefixed_public_id(p_blueprint_course_id, 'BP')
           AND course.blueprint_course_id = p_blueprint_course_id
           AND ple_api.current_session_account_is_instructor()
           AND course.availability IN ('public', 'archived')
    ) THEN
        RAISE EXCEPTION 'Blueprint not found' USING ERRCODE = '42501';
    END IF;
    IF p_kind = 'revisions' THEN
        RETURN QUERY
        WITH matched AS (
            SELECT revision.blueprint_revision_number::bigint AS sequence,
                   revision.saved_at AS occurred_at,
                   CASE WHEN p_sort = 'sequence'
                        THEN lpad(revision.blueprint_revision_number::text, 20, '0')
                        ELSE to_char(revision.saved_at AT TIME ZONE 'UTC', 'YYYYMMDDHH24MISS.US')
                   END COLLATE "C" AS sort_key
              FROM ple_data.blueprint_course_revision AS revision
             WHERE revision.blueprint_course_id = p_blueprint_course_id
               AND (p_query = '' OR position(lower(p_query) IN lower(revision.blueprint_revision_number::text)) > 0)
        )
        SELECT json_build_array(row.sort_key, row.sequence)::text,
               row.sequence, (extract(epoch FROM row.occurred_at) * 1000)::bigint,
               NULL::text, NULL::text, NULL::text,
               NULL::uuid, NULL::uuid, NULL::uuid, NULL::uuid, NULL::text[], NULL::text
          FROM matched AS row
         WHERE p_after_key IS NULL
            OR (p_direction = 'ascending' AND (row.sort_key, row.sequence) > (p_after_key COLLATE "C", p_after_sequence))
            OR (p_direction = 'descending' AND (row.sort_key, row.sequence) < (p_after_key COLLATE "C", p_after_sequence))
         ORDER BY CASE WHEN p_direction = 'ascending' THEN row.sort_key END ASC,
                  CASE WHEN p_direction = 'ascending' THEN row.sequence END ASC,
                  CASE WHEN p_direction = 'descending' THEN row.sort_key END DESC,
                  CASE WHEN p_direction = 'descending' THEN row.sequence END DESC
         LIMIT p_page_size + 1;
    ELSE
        RETURN QUERY
        WITH matched AS (
            SELECT event.blueprint_edit_number AS sequence,
                   event.occurred_at,
                   event.short_name, event.long_name, event.availability,
                   event.content_discipline_id, event.content_subject_id,
                   event.content_topic_id, event.content_subtopic_id,
                   event.tags, event.theme_id,
                   CASE p_sort
                       WHEN 'sequence' THEN lpad(event.blueprint_edit_number::text, 20, '0')
                       WHEN 'recorded_at' THEN to_char(event.occurred_at AT TIME ZONE 'UTC', 'YYYYMMDDHH24MISS.US')
                       ELSE lower(event.long_name)
                   END COLLATE "C" AS sort_key
              FROM ple_data.blueprint_metadata_event AS event
             WHERE event.blueprint_course_id = p_blueprint_course_id
               AND (p_query = '' OR position(lower(p_query) IN lower(event.short_name)) > 0
                    OR position(lower(p_query) IN lower(event.long_name)) > 0
                    OR position(lower(p_query) IN lower(event.theme_id)) > 0)
               AND (cardinality(p_availability) = 0 OR event.availability::text = ANY(p_availability))
        )
        SELECT json_build_array(row.sort_key, row.sequence)::text,
               row.sequence, (extract(epoch FROM row.occurred_at) * 1000)::bigint,
               row.short_name, row.long_name, row.availability::text,
               row.content_discipline_id, row.content_subject_id,
               row.content_topic_id, row.content_subtopic_id,
               row.tags, row.theme_id
          FROM matched AS row
         WHERE p_after_key IS NULL
            OR (p_direction = 'ascending' AND (row.sort_key, row.sequence) > (p_after_key COLLATE "C", p_after_sequence))
            OR (p_direction = 'descending' AND (row.sort_key, row.sequence) < (p_after_key COLLATE "C", p_after_sequence))
         ORDER BY CASE WHEN p_direction = 'ascending' THEN row.sort_key END ASC,
                  CASE WHEN p_direction = 'ascending' THEN row.sequence END ASC,
                  CASE WHEN p_direction = 'descending' THEN row.sort_key END DESC,
                  CASE WHEN p_direction = 'descending' THEN row.sequence END DESC
         LIMIT p_page_size + 1;
    END IF;
END
$$;

-- Facets exclude the availability refinement but retain the same text search and visibility.
CREATE FUNCTION ple_api.blueprint_history_availability_facets(
    p_blueprint_course_id text, p_query text
)
RETURNS TABLE (availability text, result_count bigint)
LANGUAGE plpgsql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private
AS $$
BEGIN
    IF p_query IS NULL OR length(p_query) > 120 THEN
        RAISE EXCEPTION 'invalid Blueprint history query' USING ERRCODE = '22023';
    END IF;
    IF NOT EXISTS (
        SELECT 1 FROM ple_data.blueprint_course AS course
         WHERE ple_private.is_canonical_prefixed_public_id(p_blueprint_course_id, 'BP')
           AND course.blueprint_course_id = p_blueprint_course_id
           AND ple_api.current_session_account_is_instructor()
           AND course.availability IN ('public', 'archived')
    ) THEN
        RAISE EXCEPTION 'Blueprint not found' USING ERRCODE = '42501';
    END IF;
    RETURN QUERY
    SELECT event.availability::text, count(*)::bigint
      FROM ple_data.blueprint_metadata_event AS event
     WHERE event.blueprint_course_id = p_blueprint_course_id
       AND (p_query = '' OR position(lower(p_query) IN lower(event.short_name)) > 0
            OR position(lower(p_query) IN lower(event.long_name)) > 0
            OR position(lower(p_query) IN lower(event.theme_id)) > 0)
     GROUP BY event.availability;
END
$$;
