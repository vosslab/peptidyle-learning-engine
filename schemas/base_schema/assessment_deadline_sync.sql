-- Private synchronization of current Assessment deadlines to Course facts.

SET LOCAL ROLE ple_data_owner;

CREATE FUNCTION ple_data.synchronize_course_assessment_deadline(p_course_id uuid)
RETURNS void LANGUAGE plpgsql
SET search_path = pg_catalog, ple_data AS $$
DECLARE
    latest_due_at timestamp with time zone;
BEGIN
    SELECT max(assessment.due_at) INTO latest_due_at
      FROM ple_data.assessment AS assessment
     WHERE assessment.course_id = p_course_id
       AND assessment.assessment_status IN ('unreleased', 'released');
    UPDATE ple_data.course_instance AS course
       SET latest_assessment_due_at = latest_due_at,
           retention_starts_at = CASE course.retention_lifecycle_state
               WHEN 'active' THEN COALESCE(latest_due_at, course.active_until_at)
               ELSE course.retention_starts_at
           END
     WHERE course.course_id = p_course_id;
END
$$;

REVOKE ALL ON FUNCTION ple_data.synchronize_course_assessment_deadline(uuid) FROM PUBLIC;

RESET ROLE;
