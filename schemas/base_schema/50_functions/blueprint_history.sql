-- Whole-result discovery of the two immutable Blueprint history sequences.

SET LOCAL ROLE ple_api_owner;

CREATE FUNCTION ple_api.list_blueprint_history(
    p_blueprint_course_id text, p_kind text, p_query text, p_sort text,
    p_direction text, p_availability text[], p_page_number bigint, p_page_size bigint
)
RETURNS TABLE (
    revision_number bigint, recorded_at_ms bigint,
    short_name text, long_name text, availability text,
    content_discipline_id uuid, content_subject_id uuid, content_topic_id uuid,
    content_subtopic_id uuid, tags text[], theme_id text,
    total_count bigint, resolved_page_number bigint
)
LANGUAGE plpgsql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private
AS $$
BEGIN
    IF p_kind IS NULL OR p_kind NOT IN ('revisions', 'metadata')
       OR p_sort IS NULL OR p_sort NOT IN ('sequence', 'recorded_at', 'name')
       OR (p_kind = 'revisions' AND p_sort = 'name')
       OR p_direction IS NULL OR p_direction NOT IN ('ascending', 'descending')
       OR p_page_number IS NULL OR p_page_number NOT BETWEEN 1 AND 4294967295
       OR p_page_size IS NULL OR p_page_size NOT BETWEEN 1 AND 100
       OR p_query IS NULL OR length(p_query) > 120
       OR p_availability IS NULL OR cardinality(p_availability) > 2
       OR array_position(p_availability, NULL) IS NOT NULL
       OR EXISTS (SELECT 1 FROM unnest(p_availability) AS value WHERE value NOT IN ('active', 'archived'))
       OR (p_kind = 'revisions' AND cardinality(p_availability) > 0) THEN
        RAISE EXCEPTION 'invalid Blueprint history page' USING ERRCODE = '22023';
    END IF;
    -- ASVS 8.2.1/2/3: every page is checked against current ordinary visibility.
    IF NOT EXISTS (
        SELECT 1 FROM ple_data.blueprint_course AS course
         WHERE ple_private.is_canonical_prefixed_public_id(p_blueprint_course_id, 'BP')
           AND course.blueprint_course_id = p_blueprint_course_id
           AND ple_api.current_session_account_is_instructor()
           AND course.availability IN ('active', 'archived')
    ) THEN
        RAISE EXCEPTION 'Blueprint not found' USING ERRCODE = '42501';
    END IF;
    IF p_kind = 'revisions' THEN
        RETURN QUERY
        WITH matched AS MATERIALIZED (
            SELECT revision.blueprint_revision_number::bigint AS sequence,
                   revision.saved_at AS occurred_at,
                   CASE WHEN p_sort = 'sequence'
                        THEN lpad(revision.blueprint_revision_number::text, 20, '0')
                        ELSE to_char(revision.saved_at AT TIME ZONE 'UTC', 'YYYYMMDDHH24MISS.US')
                   END COLLATE "C" AS sort_key
              FROM ple_data.blueprint_course_revision AS revision
             WHERE revision.blueprint_course_id = p_blueprint_course_id
               AND (p_query = '' OR position(lower(p_query) IN lower(revision.blueprint_revision_number::text)) > 0)
        ), counts AS (
            SELECT count(*)::bigint AS total_count FROM matched
        ), resolved AS (
            SELECT counts.total_count,
                   LEAST(p_page_number, GREATEST(1::bigint, ceil(counts.total_count::numeric / p_page_size)::bigint)) AS resolved_page_number
              FROM counts
        )
        SELECT page.sequence, (extract(epoch FROM page.occurred_at) * 1000)::bigint,
               NULL::text, NULL::text, NULL::text,
               NULL::uuid, NULL::uuid, NULL::uuid, NULL::uuid, NULL::text[], NULL::text,
               resolved.total_count, resolved.resolved_page_number
          FROM resolved
          LEFT JOIN LATERAL (
              SELECT matched.* FROM matched
               ORDER BY CASE WHEN p_direction = 'ascending' THEN matched.sort_key END ASC,
                        CASE WHEN p_direction = 'ascending' THEN matched.sequence END ASC,
                        CASE WHEN p_direction = 'descending' THEN matched.sort_key END DESC,
                        CASE WHEN p_direction = 'descending' THEN matched.sequence END DESC
               OFFSET ((SELECT resolved.resolved_page_number FROM resolved) - 1) * p_page_size
               LIMIT p_page_size
          ) AS page ON true
         ORDER BY CASE WHEN p_direction = 'ascending' THEN page.sort_key END ASC,
                  CASE WHEN p_direction = 'ascending' THEN page.sequence END ASC,
                  CASE WHEN p_direction = 'descending' THEN page.sort_key END DESC,
                  CASE WHEN p_direction = 'descending' THEN page.sequence END DESC;
    ELSE
        RETURN QUERY
        WITH matched AS MATERIALIZED (
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
        ), counts AS (
            SELECT count(*)::bigint AS total_count FROM matched
        ), resolved AS (
            SELECT counts.total_count,
                   LEAST(p_page_number, GREATEST(1::bigint, ceil(counts.total_count::numeric / p_page_size)::bigint)) AS resolved_page_number
              FROM counts
        )
        SELECT page.sequence, (extract(epoch FROM page.occurred_at) * 1000)::bigint,
               page.short_name, page.long_name, page.availability::text,
               page.content_discipline_id, page.content_subject_id,
               page.content_topic_id, page.content_subtopic_id,
               page.tags, page.theme_id,
               resolved.total_count, resolved.resolved_page_number
          FROM resolved
          LEFT JOIN LATERAL (
              SELECT matched.* FROM matched
               ORDER BY CASE WHEN p_direction = 'ascending' THEN matched.sort_key END ASC,
                        CASE WHEN p_direction = 'ascending' THEN matched.sequence END ASC,
                        CASE WHEN p_direction = 'descending' THEN matched.sort_key END DESC,
                        CASE WHEN p_direction = 'descending' THEN matched.sequence END DESC
               OFFSET ((SELECT resolved.resolved_page_number FROM resolved) - 1) * p_page_size
               LIMIT p_page_size
          ) AS page ON true
         ORDER BY CASE WHEN p_direction = 'ascending' THEN page.sort_key END ASC,
                  CASE WHEN p_direction = 'ascending' THEN page.sequence END ASC,
                  CASE WHEN p_direction = 'descending' THEN page.sort_key END DESC,
                  CASE WHEN p_direction = 'descending' THEN page.sequence END DESC;
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
           AND course.availability IN ('active', 'archived')
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
