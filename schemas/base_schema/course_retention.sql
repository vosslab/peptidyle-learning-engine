-- Operational Course-retention policy.  Course-core owns Course facts; this
-- module only derives due work from those facts and the current policy.

SET LOCAL ROLE ple_data_owner;

CREATE TABLE ple_data.course_retention_policy (
    policy_key boolean PRIMARY KEY DEFAULT true CHECK (policy_key),
    inactive_warning_lead_time interval NOT NULL CHECK (inactive_warning_lead_time > INTERVAL '0'),
    archive_notice_lead_time interval NOT NULL CHECK (archive_notice_lead_time > INTERVAL '0'),
    archive_after_retention_start interval NOT NULL
        CHECK (archive_after_retention_start > INTERVAL '0'),
    delete_after_archive interval NOT NULL CHECK (delete_after_archive > INTERVAL '0'),
    CHECK (archive_notice_lead_time <= archive_after_retention_start)
);

-- These are operational defaults, not product-policy constants.  The retention
-- start is the latest Assessment deadline: archive at day 100, with a day-30
-- notice expressed as a 70-day lead (100 - 70), then delete at day 365 through
-- the 265-day interval after the same absolute archive cutoff (100 + 265).
-- The independent 14-day inactive-Course warning remains unchanged.
-- A deployment administrator can set its local FERPA periods before retention
-- processing.
INSERT INTO ple_data.course_retention_policy (
    inactive_warning_lead_time,
    archive_notice_lead_time,
    archive_after_retention_start,
    delete_after_archive
) VALUES (
    INTERVAL '14 days',
    INTERVAL '70 days',
    INTERVAL '100 days',
    INTERVAL '265 days'
);

CREATE FUNCTION ple_data.course_retention_due_actions(p_evaluated_at timestamp with time zone)
RETURNS TABLE (
    course_id uuid,
    due_action text,
    due_at timestamp with time zone,
    archive_marked_at timestamp with time zone
)
LANGUAGE sql STABLE SET search_path = pg_catalog, ple_data
AS $$
    WITH policy AS (
        SELECT inactive_warning_lead_time, archive_notice_lead_time,
               archive_after_retention_start, delete_after_archive
          FROM ple_data.course_retention_policy
         WHERE policy_key
    ), scheduled AS (
        SELECT course.course_id,
               'mark_inactive'::text AS due_action,
               course.active_until_at AS due_at,
               course.student_data_archived_at AS archive_marked_at
          FROM ple_data.course_instance AS course
         WHERE course.course_lifecycle_state = 'active'

        UNION ALL

        SELECT course.course_id,
               'warn_inactive'::text AS due_action,
               course.active_until_at - policy.inactive_warning_lead_time AS due_at,
               course.student_data_archived_at AS archive_marked_at
          FROM ple_data.course_instance AS course
          CROSS JOIN policy
         WHERE course.course_lifecycle_state = 'active'

        UNION ALL

        SELECT course.course_id,
               'notify_archive'::text,
               course.retention_starts_at + policy.archive_after_retention_start
                   - policy.archive_notice_lead_time,
               course.student_data_archived_at
          FROM ple_data.course_instance AS course
          CROSS JOIN policy
         WHERE course.retention_starts_at IS NOT NULL
           AND course.student_data_archived_at IS NULL

        UNION ALL

        SELECT course.course_id,
               'archive'::text,
               course.retention_starts_at + policy.archive_after_retention_start,
               course.student_data_archived_at
          FROM ple_data.course_instance AS course
          CROSS JOIN policy
         WHERE course.retention_starts_at IS NOT NULL
           AND course.student_data_archived_at IS NULL

        UNION ALL

        -- ASVS 14.2.4/14.2.7: deletion follows the configured absolute
        -- retention schedule; the observed archive timestamp is evidence only.
        SELECT course.course_id,
               'delete'::text,
               course.retention_starts_at + policy.archive_after_retention_start
                   + policy.delete_after_archive,
               course.student_data_archived_at
          FROM ple_data.course_instance AS course
          CROSS JOIN policy
         WHERE course.retention_starts_at IS NOT NULL
           AND course.student_data_deleted_at IS NULL
    )
    SELECT scheduled.course_id, scheduled.due_action, scheduled.due_at,
           scheduled.archive_marked_at
      FROM scheduled
     WHERE p_evaluated_at IS NOT NULL
       AND scheduled.due_at <= p_evaluated_at
     ORDER BY scheduled.due_at, scheduled.course_id, scheduled.due_action
$$;

ALTER TABLE ple_data.course_retention_policy ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_data.course_retention_policy FORCE ROW LEVEL SECURITY;
REVOKE ALL ON TABLE ple_data.course_retention_policy FROM PUBLIC;
REVOKE ALL ON FUNCTION ple_data.course_retention_due_actions(timestamp with time zone) FROM PUBLIC;

CREATE POLICY course_retention_policy_data_owner_access
    ON ple_data.course_retention_policy FOR ALL TO ple_data_owner
    USING (true) WITH CHECK (true);

RESET ROLE;

-- The private retained-Work projections cannot read Course rows directly.
-- Keep this state predicate in the API owner's narrow, unexposed capability
-- instead of widening their table privileges.  ASVS 2.3.1.
SET LOCAL ROLE ple_api_owner;
CREATE FUNCTION ple_api.course_student_work_is_ordinarily_visible(
    p_course_id uuid
) RETURNS boolean LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_data AS $$
    SELECT p_course_id IS NOT NULL
       AND EXISTS (
           SELECT 1
             FROM ple_data.course_instance AS course
            WHERE course.course_id = p_course_id
              AND course.retention_lifecycle_state = 'active'
       )
$$;
REVOKE ALL ON FUNCTION ple_api.course_student_work_is_ordinarily_visible(uuid) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_api.course_student_work_is_ordinarily_visible(uuid)
    TO ple_private_owner;
RESET ROLE;
