-- Functions, triggers, and views from assessment_deadline_sync.sql.

SET LOCAL ROLE ple_data_owner;

-- Private synchronization of current Assessment deadlines to Course facts.
CREATE FUNCTION ple_data.synchronize_course_assessment_deadline(p_course_instance_id text)
RETURNS void LANGUAGE plpgsql
SET search_path = pg_catalog, ple_data AS $$
DECLARE
    latest_due_at timestamp with time zone;
BEGIN
    SELECT max(policy.due_at) INTO latest_due_at
      FROM ple_data.assessment AS assessment
      JOIN ple_data.assessment_policy_snapshot AS policy
        ON policy.assessment_policy_snapshot_id = assessment.assessment_policy_snapshot_id
     WHERE assessment.course_instance_id = p_course_instance_id
       AND assessment.assessment_status IN ('unreleased', 'released');
    UPDATE ple_data.course_instance AS course
       SET latest_assessment_due_at = latest_due_at,
           retention_starts_at = CASE course.retention_lifecycle_state
               WHEN 'active' THEN COALESCE(latest_due_at, course.active_until_at)
               ELSE course.retention_starts_at
           END
     WHERE course.course_instance_id = p_course_instance_id;
END
$$;

