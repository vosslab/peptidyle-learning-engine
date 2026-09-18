-- Functions, triggers, and views from course_retention.sql.

SET LOCAL ROLE ple_data_owner;

-- Installation-wide FERPA intervals. Administrators override with
-- ALTER DATABASE ... SET ple.retention_* = '...'; there is no policy table.
CREATE FUNCTION ple_data.retention_schedule()
RETURNS TABLE (
    inactive_warning_lead_time interval,
    archive_notice_lead_time interval,
    archive_after_retention_start interval,
    delete_after_archive interval
)
LANGUAGE sql STABLE
SET search_path = pg_catalog
AS $$
    SELECT
        COALESCE(
            NULLIF(
                current_setting('ple.retention_inactive_warning_lead_time', true),
                ''
            )::interval,
            INTERVAL '14 days'
        ),
        COALESCE(
            NULLIF(
                current_setting('ple.retention_archive_notice_lead_time', true),
                ''
            )::interval,
            INTERVAL '70 days'
        ),
        COALESCE(
            NULLIF(
                current_setting('ple.retention_archive_after_retention_start', true),
                ''
            )::interval,
            INTERVAL '100 days'
        ),
        COALESCE(
            NULLIF(
                current_setting('ple.retention_delete_after_archive', true),
                ''
            )::interval,
            INTERVAL '265 days'
        )
$$;

CREATE FUNCTION ple_data.course_retention_due_actions(p_evaluated_at timestamp with time zone)
RETURNS TABLE (
    course_instance_id text,
    due_action text,
    due_at timestamp with time zone,
    archive_marked_at timestamp with time zone
)
LANGUAGE sql STABLE SET search_path = pg_catalog, ple_data
AS $$
    WITH schedule AS (
        SELECT * FROM ple_data.retention_schedule()
    ), scheduled AS (
        SELECT course.course_instance_id,
               'mark_inactive'::text AS due_action,
               course.active_until_at AS due_at,
               course.student_data_archived_at AS archive_marked_at
          FROM ple_data.course_instance AS course
         WHERE course.course_lifecycle_state = 'active'

        UNION ALL

        SELECT course.course_instance_id,
               'warn_inactive'::text AS due_action,
               course.active_until_at - schedule.inactive_warning_lead_time AS due_at,
               course.student_data_archived_at AS archive_marked_at
          FROM ple_data.course_instance AS course
          CROSS JOIN schedule
         WHERE course.course_lifecycle_state = 'active'

        UNION ALL

        SELECT course.course_instance_id,
               'notify_archive'::text,
               course.retention_starts_at + schedule.archive_after_retention_start
                   - schedule.archive_notice_lead_time,
               course.student_data_archived_at
          FROM ple_data.course_instance AS course
          CROSS JOIN schedule
         WHERE course.retention_starts_at IS NOT NULL
           AND course.student_data_archived_at IS NULL

        UNION ALL

        SELECT course.course_instance_id,
               'archive'::text,
               course.retention_starts_at + schedule.archive_after_retention_start,
               course.student_data_archived_at
          FROM ple_data.course_instance AS course
          CROSS JOIN schedule
         WHERE course.retention_starts_at IS NOT NULL
           AND course.student_data_archived_at IS NULL

        UNION ALL

        -- ASVS 14.2.4/14.2.7: deletion follows the configured absolute
        -- retention schedule; the observed archive timestamp is evidence only.
        SELECT course.course_instance_id,
               'delete'::text,
               course.retention_starts_at + schedule.archive_after_retention_start
                   + schedule.delete_after_archive,
               course.student_data_archived_at
          FROM ple_data.course_instance AS course
          CROSS JOIN schedule
         WHERE course.retention_starts_at IS NOT NULL
           AND course.student_data_deleted_at IS NULL
    )
    SELECT scheduled.course_instance_id, scheduled.due_action, scheduled.due_at,
           scheduled.archive_marked_at
      FROM scheduled
     WHERE p_evaluated_at IS NOT NULL
       AND scheduled.due_at <= p_evaluated_at
     ORDER BY scheduled.due_at, scheduled.course_instance_id, scheduled.due_action
$$;

SET LOCAL ROLE ple_api_owner;





-- The private retained-Work projections cannot read Course rows directly.
-- Keep this state predicate in the API owner's narrow, unexposed capability
-- instead of widening their table privileges.  ASVS 2.3.1.
CREATE FUNCTION ple_api.course_student_work_is_ordinarily_visible(
    p_course_instance_id text
) RETURNS boolean LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_data AS $$
    SELECT p_course_instance_id IS NOT NULL
       AND EXISTS (
           SELECT 1
             FROM ple_data.course_instance AS course
            WHERE course.course_instance_id = p_course_instance_id
              AND course.retention_lifecycle_state = 'active'
       )
$$;

