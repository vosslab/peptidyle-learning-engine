-- Functions from sysadmin_course_inspection.sql.

SET LOCAL ROLE ple_api_owner;

-- ASVS 8.2.1/8.2.2/8.2.3/8.3.1/14.2.6: only the current Sysadmin may read
-- installation Courses. The projection keeps Course identity, term, activity,
-- retention state, and active Instructor display names.
CREATE FUNCTION ple_api.list_installation_courses(
    p_query text,
    p_page_number bigint,
    p_page_size integer,
    p_sort text DEFAULT 'nameAscending',
    p_activity text[] DEFAULT ARRAY[]::text[],
    p_retention text[] DEFAULT ARRAY[]::text[]
)
RETURNS TABLE(
    course_instance_id text,
    short_name text,
    long_name text,
    term_starts_on date,
    term_ends_on date,
    course_lifecycle_state text,
    retention_lifecycle_state text,
    instructor_display_names text[],
    total_count bigint,
    resolved_page_number bigint
)
LANGUAGE plpgsql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private
AS $$
DECLARE
    v_query text;
BEGIN
    IF NOT ple_api.current_session_account_is_sysadmin() THEN
        RAISE EXCEPTION 'installation Course inspection is unavailable'
            USING ERRCODE = '42501';
    END IF;
    -- ASVS 2.2.1: the search is a bounded literal, never a wildcard pattern.
    v_query := NULLIF(btrim(p_query), '');
    IF v_query IS NOT NULL AND (
        char_length(v_query) > 200 OR v_query ~ '[[:cntrl:]]'
    ) THEN
        RAISE EXCEPTION 'installation Course query is invalid'
            USING ERRCODE = '22023';
    END IF;
    IF p_page_number IS NULL OR p_page_number < 1
       OR p_page_size IS NULL OR p_page_size NOT BETWEEN 1 AND 250
       OR p_sort IS NULL OR p_sort NOT IN ('nameAscending', 'nameDescending')
       OR p_activity IS NULL OR p_retention IS NULL
       OR EXISTS (SELECT 1 FROM unnest(p_activity) AS value WHERE value NOT IN ('active', 'inactive'))
       OR EXISTS (SELECT 1 FROM unnest(p_retention) AS value WHERE value NOT IN ('active', 'archived', 'deleted')) THEN
        RAISE EXCEPTION 'installation Course page is invalid' USING ERRCODE = '22023';
    END IF;
    RETURN QUERY
    WITH matching AS MATERIALIZED (
        SELECT course.course_instance_id,
               course.course_short_name,
               course.course_long_name,
               course.term_starts_on,
               course.term_ends_on,
               course.course_lifecycle_state,
               course.retention_lifecycle_state,
               COALESCE((
                   SELECT array_agg(named.display_name ORDER BY named.display_name)
                     FROM (
                         SELECT DISTINCT ple_private.instructor_display_name(
                                    membership.account_id::text
                                ) AS display_name
                           FROM ple_data.course_membership AS membership
                          WHERE membership.course_instance_id = course.course_instance_id
                            AND membership.role = 'instructor'
                            AND ple_data.course_membership_is_active(membership.course_membership_id)
                     ) AS named
                    WHERE named.display_name IS NOT NULL
                      AND named.display_name = btrim(named.display_name)
                      AND char_length(named.display_name) BETWEEN 1 AND 200
                      AND named.display_name !~ '[[:cntrl:]]'
               ), ARRAY[]::text[]) AS instructor_display_names
          FROM ple_data.course_instance AS course
         WHERE (v_query IS NULL
            OR upper(v_query) = course.course_instance_id::text
            OR position(lower(v_query) IN lower(course.course_short_name)) > 0
            OR position(lower(v_query) IN lower(course.course_long_name)) > 0
            OR position(lower(v_query) IN lower(course.course_instance_id::text)) > 0
            OR EXISTS (
                SELECT 1
                  FROM ple_data.course_membership AS membership
                 WHERE membership.course_instance_id = course.course_instance_id
                   AND membership.role = 'instructor'
                   AND ple_data.course_membership_is_active(membership.course_membership_id)
                   AND position(lower(v_query) IN lower(
                       ple_private.instructor_display_name(membership.account_id::text)
                   )) > 0
            ))
           AND (cardinality(p_activity) = 0 OR course.course_lifecycle_state::text = ANY(p_activity))
           AND (cardinality(p_retention) = 0 OR course.retention_lifecycle_state::text = ANY(p_retention))
    ), page_metadata AS (
        SELECT count(*)::bigint AS total_count,
               greatest(1::bigint, ceil(count(*)::numeric / p_page_size)::bigint) AS last_page
          FROM matching
    ), resolved_page AS (
        SELECT page_metadata.total_count,
               least(p_page_number, page_metadata.last_page) AS resolved_page_number
          FROM page_metadata
    ), page_rows AS (
        SELECT matching.*,
               row_number() OVER (
                   ORDER BY
                       CASE WHEN p_sort = 'nameAscending' THEN matching.course_long_name END ASC,
                       CASE WHEN p_sort = 'nameAscending' THEN matching.course_instance_id END ASC,
                       CASE WHEN p_sort = 'nameDescending' THEN matching.course_long_name END DESC,
                       CASE WHEN p_sort = 'nameDescending' THEN matching.course_instance_id END DESC
               ) AS page_order
          FROM matching
         ORDER BY
               CASE WHEN p_sort = 'nameAscending' THEN matching.course_long_name END ASC,
               CASE WHEN p_sort = 'nameAscending' THEN matching.course_instance_id END ASC,
               CASE WHEN p_sort = 'nameDescending' THEN matching.course_long_name END DESC,
               CASE WHEN p_sort = 'nameDescending' THEN matching.course_instance_id END DESC
         OFFSET (
             SELECT (resolved_page.resolved_page_number - 1) * p_page_size
               FROM resolved_page
         )
         LIMIT p_page_size
    )
    SELECT page_rows.course_instance_id::text,
           page_rows.course_short_name::text,
           page_rows.course_long_name::text,
           page_rows.term_starts_on,
           page_rows.term_ends_on,
           page_rows.course_lifecycle_state::text,
           page_rows.retention_lifecycle_state::text,
           page_rows.instructor_display_names,
           resolved_page.total_count,
           resolved_page.resolved_page_number
      FROM resolved_page
      LEFT JOIN page_rows ON true
     ORDER BY page_rows.page_order;
END
$$;

CREATE FUNCTION ple_api.installation_course_facet_choices(
    p_query text, p_activity text[], p_retention text[]
) RETURNS TABLE(field_name text, choice_key text, matching_count bigint)
LANGUAGE plpgsql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private
AS $$
DECLARE v_query text := NULLIF(btrim(p_query), '');
BEGIN
    IF NOT ple_api.current_session_account_is_sysadmin() THEN
        RAISE EXCEPTION 'installation Course inspection is unavailable' USING ERRCODE = '42501';
    END IF;
    IF (v_query IS NOT NULL AND (char_length(v_query) > 200 OR v_query ~ '[[:cntrl:]]'))
       OR EXISTS (SELECT 1 FROM unnest(p_activity) AS value WHERE value NOT IN ('active', 'inactive'))
       OR EXISTS (SELECT 1 FROM unnest(p_retention) AS value WHERE value NOT IN ('active', 'archived', 'deleted')) THEN
        RAISE EXCEPTION 'installation Course facet is invalid' USING ERRCODE = '22023';
    END IF;
    RETURN QUERY
    WITH matching AS (
        SELECT course.course_lifecycle_state::text AS activity,
               course.retention_lifecycle_state::text AS retention
          FROM ple_data.course_instance AS course
         WHERE (v_query IS NULL
            OR upper(v_query) = course.course_instance_id::text
            OR position(lower(v_query) IN lower(course.course_short_name)) > 0
            OR position(lower(v_query) IN lower(course.course_long_name)) > 0
            OR position(lower(v_query) IN lower(course.course_instance_id::text)) > 0
            OR EXISTS (
                SELECT 1 FROM ple_data.course_membership AS membership
                 WHERE membership.course_instance_id = course.course_instance_id
                   AND membership.role = 'instructor'
                   AND ple_data.course_membership_is_active(membership.course_membership_id)
                   AND position(lower(v_query) IN lower(
                       ple_private.instructor_display_name(membership.account_id::text)
                   )) > 0
            ))
    )
    SELECT 'activity'::text, choice.key, count(matching.activity)::bigint
      FROM (VALUES ('active'), ('inactive')) AS choice(key)
      LEFT JOIN matching ON matching.activity = choice.key
        AND (cardinality(p_retention) = 0 OR matching.retention = ANY(p_retention))
     GROUP BY choice.key
    UNION ALL
    SELECT 'retention'::text, choice.key, count(matching.retention)::bigint
      FROM (VALUES ('active'), ('archived'), ('deleted')) AS choice(key)
      LEFT JOIN matching ON matching.retention = choice.key
        AND (cardinality(p_activity) = 0 OR matching.activity = ANY(p_activity))
     GROUP BY choice.key;
END
$$;

CREATE FUNCTION ple_api.load_installation_course(p_course_instance_id text)
RETURNS TABLE(
    course_instance_id text,
    short_name text,
    long_name text,
    term_starts_on date,
    term_ends_on date,
    course_lifecycle_state text,
    retention_lifecycle_state text,
    instructor_display_names text[]
)
LANGUAGE plpgsql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private
AS $$
BEGIN
    -- ASVS 8.2.1/8.2.2: refuse before the id check so a non-Sysadmin cannot
    -- distinguish an invalid id from a missing Course.
    IF NOT ple_api.current_session_account_is_sysadmin() THEN
        RAISE EXCEPTION 'installation Course inspection is unavailable'
            USING ERRCODE = '42501';
    END IF;
    IF NOT ple_private.is_canonical_prefixed_public_id(p_course_instance_id, 'CI') THEN
        RAISE EXCEPTION 'installation Course id is invalid'
            USING ERRCODE = '22023';
    END IF;
    RETURN QUERY
    SELECT course.course_instance_id::text,
           course.course_short_name::text,
           course.course_long_name::text,
           course.term_starts_on,
           course.term_ends_on,
           course.course_lifecycle_state::text,
           course.retention_lifecycle_state::text,
           COALESCE((
               SELECT array_agg(named.display_name ORDER BY named.display_name)
                 FROM (
                     SELECT DISTINCT ple_private.instructor_display_name(
                                membership.account_id::text
                            ) AS display_name
                       FROM ple_data.course_membership AS membership
                      WHERE membership.course_instance_id = course.course_instance_id
                        AND membership.role = 'instructor'
                        AND ple_data.course_membership_is_active(membership.course_membership_id)
                 ) AS named
                WHERE named.display_name IS NOT NULL
                  AND named.display_name = btrim(named.display_name)
                  AND char_length(named.display_name) BETWEEN 1 AND 200
                  AND named.display_name !~ '[[:cntrl:]]'
           ), ARRAY[]::text[])
      FROM ple_data.course_instance AS course
     WHERE course.course_instance_id = p_course_instance_id;
END
$$;
