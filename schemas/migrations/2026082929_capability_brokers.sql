-- SD1 durable observer/support grants and session-derived authorization brokers.

SET LOCAL ROLE ple_private_owner;
CREATE TABLE ple_private.course_observer_grant (
    grant_id uuid PRIMARY KEY,
    course_id uuid NOT NULL REFERENCES ple_data.course_instance (course_id),
    observer_user_id uuid NOT NULL REFERENCES ple_private.account (user_id),
    issued_by_user_id uuid NOT NULL REFERENCES ple_private.account (user_id),
    issued_at timestamp with time zone NOT NULL,
    revoked_at timestamp with time zone,
    UNIQUE (course_id, observer_user_id),
    CONSTRAINT course_observer_grant_revocation_is_ordered CHECK (revoked_at IS NULL OR revoked_at >= issued_at)
);
CREATE TABLE ple_private.student_observer_grant (
    grant_id uuid PRIMARY KEY,
    course_id uuid NOT NULL REFERENCES ple_data.course_instance (course_id),
    student_id uuid NOT NULL REFERENCES ple_data.course_student (student_id),
    observer_user_id uuid NOT NULL REFERENCES ple_private.account (user_id),
    issued_by_user_id uuid NOT NULL REFERENCES ple_private.account (user_id),
    issued_at timestamp with time zone NOT NULL,
    revoked_at timestamp with time zone,
    UNIQUE (course_id, student_id, observer_user_id),
    CONSTRAINT student_observer_grant_student_course_matches FOREIGN KEY (student_id, course_id)
        REFERENCES ple_data.course_student (student_id, course_id),
    CONSTRAINT student_observer_grant_revocation_is_ordered CHECK (revoked_at IS NULL OR revoked_at >= issued_at)
);
CREATE TABLE ple_private.sysadmin_support_capability (
    capability_id uuid PRIMARY KEY,
    sysadmin_user_id uuid NOT NULL,
    sysadmin_role text NOT NULL DEFAULT 'sysadmin' CHECK (sysadmin_role = 'sysadmin'),
    course_id uuid REFERENCES ple_data.course_instance (course_id),
    student_id uuid REFERENCES ple_data.course_student (student_id),
    operation_kind text NOT NULL CHECK (operation_kind IN ('course_repair', 'roster_repair', 'student_record_repair')),
    purpose text NOT NULL CHECK (char_length(btrim(purpose)) BETWEEN 1 AND 1000),
    issued_at timestamp with time zone NOT NULL,
    expires_at timestamp with time zone NOT NULL CHECK (expires_at > issued_at),
    revoked_at timestamp with time zone,
    CONSTRAINT sysadmin_support_capability_role_matches FOREIGN KEY (sysadmin_user_id, sysadmin_role)
        REFERENCES ple_private.account (user_id, role),
    CONSTRAINT sysadmin_support_capability_student_course_matches FOREIGN KEY (student_id, course_id)
        REFERENCES ple_data.course_student (student_id, course_id),
    CONSTRAINT sysadmin_support_capability_revocation_is_ordered CHECK (revoked_at IS NULL OR revoked_at >= issued_at)
);
ALTER TABLE ple_private.course_observer_grant ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.course_observer_grant FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_private.student_observer_grant ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.student_observer_grant FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_private.sysadmin_support_capability ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.sysadmin_support_capability FORCE ROW LEVEL SECURITY;
REVOKE ALL PRIVILEGES ON TABLE ple_private.course_observer_grant,
    ple_private.student_observer_grant, ple_private.sysadmin_support_capability FROM PUBLIC;
GRANT USAGE ON SCHEMA ple_private TO ple_api_owner;
GRANT SELECT ON TABLE ple_private.account, ple_private.authoring_workspace,
    ple_private.authoring_workspace_collaborator,
    ple_private.course_observer_grant, ple_private.student_observer_grant,
    ple_private.sysadmin_support_capability TO ple_api_owner;
RESET ROLE;

SET LOCAL ROLE ple_data_owner;
GRANT USAGE ON SCHEMA ple_data TO ple_api_owner;
GRANT SELECT ON TABLE ple_data.course_membership, ple_data.course_student,
    ple_data.assignment_enrollment TO ple_api_owner;
RESET ROLE;

SET LOCAL ROLE ple_api_owner;
CREATE FUNCTION ple_api.current_actor_user_id()
RETURNS uuid LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private
AS $$
    WITH configured AS (
        SELECT pg_catalog.current_setting('ple.actor_user_id', true) AS raw_user_id
    )
    SELECT account.user_id
      FROM configured
      JOIN ple_private.account AS account
        ON configured.raw_user_id ~ '^[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}$'
       AND account.user_id = configured.raw_user_id::uuid
$$;
CREATE FUNCTION ple_api.current_actor_is_course_instructor(p_course_id uuid)
RETURNS boolean LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data
AS $$
    SELECT EXISTS (
        SELECT 1 FROM ple_data.course_membership AS membership
        WHERE membership.course_id = p_course_id
          AND membership.user_id = ple_api.current_actor_user_id()
          AND membership.role = 'instructor'
          AND membership.revoked_at IS NULL
    )
$$;
CREATE FUNCTION ple_api.current_actor_is_course_student(p_course_id uuid, p_student_id uuid)
RETURNS boolean LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data
AS $$
    SELECT EXISTS (
        SELECT 1
        FROM ple_data.course_student AS student
        JOIN ple_data.course_membership AS membership ON membership.membership_id = student.membership_id
        WHERE student.course_id = p_course_id
          AND student.student_id = p_student_id
          AND membership.user_id = ple_api.current_actor_user_id()
          AND membership.role = 'student'
          AND membership.revoked_at IS NULL
    )
$$;
CREATE FUNCTION ple_api.current_actor_owns_workspace(p_workspace_id uuid)
RETURNS boolean LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private
AS $$
    SELECT EXISTS (
        SELECT 1 FROM ple_private.authoring_workspace AS workspace
        WHERE workspace.workspace_id = p_workspace_id
          AND workspace.owner_user_id = ple_api.current_actor_user_id()
          AND workspace.revoked_at IS NULL
    )
$$;
CREATE FUNCTION ple_api.current_actor_can_access_workspace(p_workspace_id uuid)
RETURNS boolean LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private
AS $$
    SELECT ple_api.current_actor_owns_workspace(p_workspace_id)
        OR EXISTS (
            SELECT 1
            FROM ple_private.authoring_workspace AS workspace
            JOIN ple_private.authoring_workspace_collaborator AS collaborator
              ON collaborator.workspace_id = workspace.workspace_id
            WHERE workspace.workspace_id = p_workspace_id
              AND workspace.revoked_at IS NULL
              AND collaborator.user_id = ple_api.current_actor_user_id()
        )
$$;
CREATE FUNCTION ple_api.current_actor_has_course_observer_grant(p_course_id uuid)
RETURNS boolean LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private
AS $$
    SELECT EXISTS (
        SELECT 1 FROM ple_private.course_observer_grant AS grant_record
        WHERE grant_record.course_id = p_course_id
          AND grant_record.observer_user_id = ple_api.current_actor_user_id()
          AND grant_record.revoked_at IS NULL
    )
$$;
CREATE FUNCTION ple_api.current_actor_has_student_observer_grant(p_course_id uuid, p_student_id uuid)
RETURNS boolean LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private
AS $$
    SELECT EXISTS (
        SELECT 1 FROM ple_private.student_observer_grant AS grant_record
        WHERE grant_record.course_id = p_course_id
          AND grant_record.student_id = p_student_id
          AND grant_record.observer_user_id = ple_api.current_actor_user_id()
          AND grant_record.revoked_at IS NULL
    )
$$;
CREATE FUNCTION ple_api.current_actor_has_support_capability(
    p_capability_id uuid, p_course_id uuid, p_student_id uuid, p_operation_kind text
)
RETURNS boolean LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private
AS $$
    SELECT EXISTS (
        SELECT 1 FROM ple_private.sysadmin_support_capability AS capability
        WHERE capability.capability_id = p_capability_id
          AND capability.sysadmin_user_id = ple_api.current_actor_user_id()
          AND capability.course_id IS NOT DISTINCT FROM p_course_id
          AND capability.student_id IS NOT DISTINCT FROM p_student_id
          AND capability.operation_kind = p_operation_kind
          AND capability.revoked_at IS NULL
          AND capability.expires_at > pg_catalog.clock_timestamp()
    )
$$;
REVOKE ALL PRIVILEGES ON FUNCTION ple_api.current_actor_user_id() FROM PUBLIC;
REVOKE ALL PRIVILEGES ON FUNCTION ple_api.current_actor_is_course_instructor(uuid) FROM PUBLIC;
REVOKE ALL PRIVILEGES ON FUNCTION ple_api.current_actor_is_course_student(uuid, uuid) FROM PUBLIC;
REVOKE ALL PRIVILEGES ON FUNCTION ple_api.current_actor_owns_workspace(uuid) FROM PUBLIC;
REVOKE ALL PRIVILEGES ON FUNCTION ple_api.current_actor_can_access_workspace(uuid) FROM PUBLIC;
REVOKE ALL PRIVILEGES ON FUNCTION ple_api.current_actor_has_course_observer_grant(uuid) FROM PUBLIC;
REVOKE ALL PRIVILEGES ON FUNCTION ple_api.current_actor_has_student_observer_grant(uuid, uuid) FROM PUBLIC;
REVOKE ALL PRIVILEGES ON FUNCTION ple_api.current_actor_has_support_capability(uuid, uuid, uuid, text) FROM PUBLIC;
GRANT USAGE ON SCHEMA ple_api TO ple_app, ple_auth, ple_student, ple_grader, ple_worker;
GRANT EXECUTE ON FUNCTION ple_api.current_actor_user_id(),
    ple_api.current_actor_is_course_instructor(uuid),
    ple_api.current_actor_is_course_student(uuid, uuid),
    ple_api.current_actor_owns_workspace(uuid),
    ple_api.current_actor_can_access_workspace(uuid),
    ple_api.current_actor_has_course_observer_grant(uuid),
    ple_api.current_actor_has_student_observer_grant(uuid, uuid),
    ple_api.current_actor_has_support_capability(uuid, uuid, uuid, text)
    TO ple_app, ple_auth, ple_student, ple_grader, ple_worker;
RESET ROLE;
