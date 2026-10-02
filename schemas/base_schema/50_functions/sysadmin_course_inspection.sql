-- Functions from sysadmin_course_inspection.sql.

SET LOCAL ROLE ple_api_owner;

-- ASVS 8.2.1/8.2.2/8.2.3/8.3.1/14.2.6: only the current Sysadmin may read
-- installation Courses. The projection keeps Course identity, term, activity,
-- retention state, and active Instructor display names.
CREATE FUNCTION ple_api.list_installation_courses(
    p_query text,
    p_after_long_name text,
    p_after_course_instance_id text,
    p_page_size integer
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
    cursor_long_name text
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
    IF p_page_size NOT BETWEEN 1 AND 250
       OR (p_after_long_name IS NULL) <> (p_after_course_instance_id IS NULL) THEN
        RAISE EXCEPTION 'installation Course page is invalid' USING ERRCODE = '22023';
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
                     SELECT DISTINCT ple_private.verified_instructor_display_name(
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
           ), ARRAY[]::text[]),
           course.course_long_name::text
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
               AND position(
                       lower(v_query) IN lower(
                           ple_private.verified_instructor_display_name(membership.account_id::text)
                       )
                   ) > 0
        ))
       AND (p_after_long_name IS NULL OR
            (course.course_long_name, course.course_instance_id) >
            (p_after_long_name, p_after_course_instance_id))
     ORDER BY course.course_long_name, course.course_instance_id
     LIMIT p_page_size + 1;
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
                     SELECT DISTINCT ple_private.verified_instructor_display_name(
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
