-- Current scalar Course Theme for each Course Instance.
--
-- The default is the invariant: existing and newly-created courses receive
-- grass without a backfill or an application-supplied creation value.

SET LOCAL ROLE ple_data_owner;

ALTER TABLE ple_data.course_instance
    ADD COLUMN course_theme text NOT NULL DEFAULT 'grass';

-- The API-owner function is the only runtime writer; its RLS policy repeats
-- the exact current-Instructor predicate so a future SQL call cannot widen
-- the function's WHERE clause into cross-course access.
GRANT UPDATE (course_theme) ON TABLE ple_data.course_instance TO ple_api_owner;
CREATE POLICY course_instance_api_owner_course_theme_update
    ON ple_data.course_instance FOR UPDATE TO ple_api_owner
    USING (ple_api.current_session_account_is_course_instructor(course_id))
    WITH CHECK (ple_api.current_session_account_is_course_instructor(course_id));

RESET ROLE;

SET LOCAL ROLE ple_api_owner;

-- ASVS 1.2.3 and 8.2.2: conceal a Course from nonmembers by filtering on the
-- installed session's current membership at the data access boundary.
CREATE FUNCTION ple_api.read_course_theme(p_course_id uuid)
RETURNS TABLE(course_theme text) LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data
AS $$
    SELECT course.course_theme
      FROM ple_data.course_instance AS course
     WHERE course.course_id = p_course_id
       AND ple_api.current_session_account_is_course_member(p_course_id)
$$;

-- ASVS 1.2.3 and 8.2.2: only a current Instructor Course Membership may
-- replace this Course-scoped setting; the predicate is evaluated in SQL, not
-- trusted from an HTTP role gate.
CREATE FUNCTION ple_api.update_course_theme(p_course_id uuid, p_course_theme text)
RETURNS TABLE(course_theme text) LANGUAGE sql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data
AS $$
    UPDATE ple_data.course_instance AS course
       SET course_theme = p_course_theme
     WHERE course.course_id = p_course_id
       AND ple_api.current_session_account_is_course_instructor(p_course_id)
 RETURNING course.course_theme
$$;

REVOKE ALL ON FUNCTION ple_api.read_course_theme(uuid) FROM PUBLIC;
REVOKE ALL ON FUNCTION ple_api.update_course_theme(uuid, text) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_api.read_course_theme(uuid),
    ple_api.update_course_theme(uuid, text) TO ple_app;

RESET ROLE;
