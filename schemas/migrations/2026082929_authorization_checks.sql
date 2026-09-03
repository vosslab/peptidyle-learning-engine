-- Course Observer relationships, Sysadmin support capabilities, and session-derived authorization checks.
-- ASVS 8.2.1, 8.2.2, and 8.3.1.

SET LOCAL ROLE ple_private_owner;
CREATE TABLE ple_private.course_observer_relationship_event (
    course_observer_relationship_event_id uuid PRIMARY KEY,
    course_id uuid NOT NULL REFERENCES ple_data.course_instance (course_id),
    course_observer_account_id uuid NOT NULL REFERENCES ple_private.account (account_id),
    recorded_by_account_id uuid NOT NULL REFERENCES ple_private.account (account_id),
    event_kind text NOT NULL CHECK (event_kind IN ('started', 'ended')),
    occurred_at timestamp with time zone NOT NULL,
    UNIQUE (course_id, course_observer_account_id, event_kind)
);
CREATE FUNCTION ple_private.reject_course_observer_relationship_event_change()
RETURNS trigger LANGUAGE plpgsql SET search_path = pg_catalog, ple_private AS $$
BEGIN RAISE EXCEPTION USING ERRCODE = '55000', MESSAGE = 'Course Observer Relationship Events are immutable'; END
$$;
CREATE TRIGGER course_observer_relationship_event_is_immutable
BEFORE UPDATE OR DELETE ON ple_private.course_observer_relationship_event
FOR EACH ROW EXECUTE FUNCTION ple_private.reject_course_observer_relationship_event_change();
CREATE FUNCTION ple_private.validate_course_observer_relationship_event()
RETURNS trigger LANGUAGE plpgsql SET search_path = pg_catalog, ple_private AS $$
BEGIN
    PERFORM pg_catalog.pg_advisory_xact_lock(
        pg_catalog.hashtextextended(
            NEW.course_id::text || ':' || NEW.course_observer_account_id::text, 0
        )
    );
    IF NEW.event_kind = 'ended' AND NOT EXISTS (
        SELECT 1 FROM ple_private.course_observer_relationship_event AS start_event
        WHERE start_event.course_id = NEW.course_id
          AND start_event.course_observer_account_id = NEW.course_observer_account_id
          AND start_event.event_kind = 'started'
          AND start_event.occurred_at <= NEW.occurred_at
    ) THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Course Observer Relationship may end only after its exact start event';
    END IF;
    RETURN NEW;
END
$$;
CREATE TRIGGER course_observer_relationship_event_has_valid_transition
BEFORE INSERT ON ple_private.course_observer_relationship_event
FOR EACH ROW EXECUTE FUNCTION ple_private.validate_course_observer_relationship_event();
CREATE TABLE ple_private.sysadmin_support_capability (
    capability_id uuid PRIMARY KEY,
    sysadmin_account_id uuid NOT NULL,
    sysadmin_role text NOT NULL DEFAULT 'sysadmin' CHECK (sysadmin_role = 'sysadmin'),
    course_id uuid REFERENCES ple_data.course_instance (course_id),
    student_record_id uuid REFERENCES ple_data.student_record (student_record_id),
    operation_kind text NOT NULL CHECK (operation_kind IN ('course_repair', 'roster_repair', 'student_record_repair')),
    purpose text NOT NULL CHECK (char_length(btrim(purpose)) BETWEEN 1 AND 1000),
    issued_at timestamp with time zone NOT NULL,
    expires_at timestamp with time zone NOT NULL CHECK (expires_at > issued_at),
    revoked_at timestamp with time zone,
    CONSTRAINT sysadmin_support_capability_product_role_matches FOREIGN KEY (sysadmin_account_id, sysadmin_role)
        REFERENCES ple_private.account (account_id, product_role),
    CONSTRAINT sysadmin_support_capability_student_record_course_matches FOREIGN KEY (student_record_id, course_id)
        REFERENCES ple_data.student_record (student_record_id, course_id),
    CONSTRAINT sysadmin_support_capability_revocation_is_ordered CHECK (revoked_at IS NULL OR revoked_at >= issued_at)
);
ALTER TABLE ple_private.course_observer_relationship_event ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.course_observer_relationship_event FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_private.sysadmin_support_capability ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.sysadmin_support_capability FORCE ROW LEVEL SECURITY;
REVOKE ALL PRIVILEGES ON TABLE ple_private.course_observer_relationship_event,
    ple_private.sysadmin_support_capability FROM PUBLIC;
GRANT USAGE ON SCHEMA ple_private TO ple_api_owner;
GRANT SELECT ON TABLE ple_private.account,
    ple_private.authoring_workspace,
    ple_private.authoring_workspace_collaborator_event,
    ple_private.course_observer_relationship_event,
    ple_private.sysadmin_support_capability TO ple_api_owner;
RESET ROLE;

SET LOCAL ROLE ple_data_owner;
GRANT USAGE ON SCHEMA ple_data TO ple_api_owner;
GRANT SELECT ON TABLE ple_data.course_membership, ple_data.course_membership_event,
    ple_data.student_record TO ple_api_owner;
RESET ROLE;

SET LOCAL ROLE ple_api_owner;
CREATE FUNCTION ple_api.current_session_account_id()
RETURNS uuid LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private
AS $$
    WITH configured AS (
        SELECT pg_catalog.current_setting('ple.session_account_id', true) AS raw_account_id
    )
    SELECT account.account_id
      FROM configured
      JOIN ple_private.account AS account
        ON configured.raw_account_id ~ '^[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}$'
       AND account.account_id = configured.raw_account_id::uuid
$$;
CREATE FUNCTION ple_api.current_session_account_is_course_instructor(p_course_id uuid)
RETURNS boolean LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data
AS $$
    SELECT EXISTS (
        SELECT 1 FROM ple_data.course_membership AS membership
        WHERE membership.course_id = p_course_id
          AND membership.account_id = ple_api.current_session_account_id()
          AND membership.role = 'instructor'
          AND ple_data.course_membership_is_active(membership.membership_id)
    )
$$;
CREATE FUNCTION ple_api.current_session_account_is_course_member(p_course_id uuid)
RETURNS boolean LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data
AS $$
    SELECT EXISTS (
        SELECT 1 FROM ple_data.course_membership AS membership
        WHERE membership.course_id = p_course_id
          AND membership.account_id = ple_api.current_session_account_id()
          AND ple_data.course_membership_is_active(membership.membership_id)
    )
$$;
CREATE FUNCTION ple_api.current_session_account_owns_course_membership(
    p_course_id uuid, p_membership_id uuid
)
RETURNS boolean LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data
AS $$
    SELECT EXISTS (
        SELECT 1 FROM ple_data.course_membership AS membership
        WHERE membership.course_id = p_course_id
          AND membership.membership_id = p_membership_id
          AND membership.account_id = ple_api.current_session_account_id()
          AND ple_data.course_membership_is_active(membership.membership_id)
    )
$$;
CREATE FUNCTION ple_api.current_session_account_owns_student_record(p_course_id uuid, p_student_record_id uuid)
RETURNS boolean LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data
AS $$
    SELECT EXISTS (
        SELECT 1
        FROM ple_data.student_record AS student
        JOIN ple_data.course_membership AS membership
          ON membership.student_record_id = student.student_record_id
        WHERE student.course_id = p_course_id
          AND student.student_record_id = p_student_record_id
          AND student.student_account_id = ple_api.current_session_account_id()
          AND membership.account_id = student.student_account_id
          AND membership.role = 'student'
          AND ple_data.course_membership_is_active(membership.membership_id)
    )
$$;
CREATE FUNCTION ple_api.current_session_account_owns_workspace(p_workspace_id uuid)
RETURNS boolean LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private
AS $$
    SELECT EXISTS (
        SELECT 1 FROM ple_private.authoring_workspace AS workspace
        WHERE workspace.workspace_id = p_workspace_id
          AND workspace.owner_account_id = ple_api.current_session_account_id()
          AND workspace.revoked_at IS NULL
    )
$$;
CREATE FUNCTION ple_api.current_session_account_can_access_workspace(p_workspace_id uuid)
RETURNS boolean LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private
AS $$
    SELECT ple_api.current_session_account_owns_workspace(p_workspace_id)
        OR EXISTS (
            SELECT 1
            FROM ple_private.authoring_workspace AS workspace
            JOIN ple_private.authoring_workspace_collaborator_event AS collaborator
              ON collaborator.workspace_id = workspace.workspace_id
            WHERE workspace.workspace_id = p_workspace_id
              AND workspace.revoked_at IS NULL
              AND collaborator.collaborator_account_id = ple_api.current_session_account_id()
              AND collaborator.event_kind = 'started'
              AND NOT EXISTS (
                  SELECT 1
                    FROM ple_private.authoring_workspace_collaborator_event AS end_event
                   WHERE end_event.workspace_id = collaborator.workspace_id
                     AND end_event.collaborator_account_id = collaborator.collaborator_account_id
                     AND end_event.event_kind = 'ended'
              )
        )
$$;
CREATE FUNCTION ple_api.current_session_account_has_course_observer_relationship(p_course_id uuid)
RETURNS boolean LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private
AS $$
    SELECT EXISTS (
        SELECT 1
        FROM ple_private.course_observer_relationship_event AS relationship
        JOIN ple_private.account AS account
          ON account.account_id = relationship.course_observer_account_id
         AND account.product_role = 'instructor'
        WHERE relationship.course_id = p_course_id
          AND relationship.course_observer_account_id = ple_api.current_session_account_id()
          AND relationship.event_kind = 'started'
          AND NOT EXISTS (
              SELECT 1 FROM ple_private.course_observer_relationship_event AS end_event
              WHERE end_event.course_id = relationship.course_id
                AND end_event.course_observer_account_id = relationship.course_observer_account_id
                AND end_event.event_kind = 'ended'
          )
          AND NOT EXISTS (
              SELECT 1
              FROM ple_data.course_membership AS membership
              WHERE membership.course_id = relationship.course_id
                AND membership.account_id = relationship.course_observer_account_id
                AND membership.role = 'instructor'
                AND ple_data.course_membership_is_active(membership.membership_id)
          )
    )
$$;
CREATE FUNCTION ple_api.current_session_account_has_support_capability(
    p_capability_id uuid, p_course_id uuid, p_student_record_id uuid, p_operation_kind text
)
RETURNS boolean LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private
AS $$
    SELECT EXISTS (
        SELECT 1 FROM ple_private.sysadmin_support_capability AS capability
        WHERE capability.capability_id = p_capability_id
          AND capability.sysadmin_account_id = ple_api.current_session_account_id()
          AND capability.course_id IS NOT DISTINCT FROM p_course_id
          AND capability.student_record_id IS NOT DISTINCT FROM p_student_record_id
          AND capability.operation_kind = p_operation_kind
          AND capability.revoked_at IS NULL
          AND capability.expires_at > pg_catalog.clock_timestamp()
    )
$$;
REVOKE ALL PRIVILEGES ON FUNCTION ple_api.current_session_account_id() FROM PUBLIC;
REVOKE ALL PRIVILEGES ON FUNCTION ple_api.current_session_account_is_course_instructor(uuid) FROM PUBLIC;
REVOKE ALL PRIVILEGES ON FUNCTION ple_api.current_session_account_is_course_member(uuid) FROM PUBLIC;
REVOKE ALL PRIVILEGES ON FUNCTION ple_api.current_session_account_owns_course_membership(uuid, uuid) FROM PUBLIC;
REVOKE ALL PRIVILEGES ON FUNCTION ple_api.current_session_account_owns_student_record(uuid, uuid) FROM PUBLIC;
REVOKE ALL PRIVILEGES ON FUNCTION ple_api.current_session_account_owns_workspace(uuid) FROM PUBLIC;
REVOKE ALL PRIVILEGES ON FUNCTION ple_api.current_session_account_can_access_workspace(uuid) FROM PUBLIC;
REVOKE ALL PRIVILEGES ON FUNCTION ple_api.current_session_account_has_course_observer_relationship(uuid) FROM PUBLIC;
REVOKE ALL PRIVILEGES ON FUNCTION ple_api.current_session_account_has_support_capability(uuid, uuid, uuid, text) FROM PUBLIC;
GRANT USAGE ON SCHEMA ple_api TO ple_app, ple_auth, ple_student;
GRANT EXECUTE ON FUNCTION ple_api.current_session_account_id(),
    ple_api.current_session_account_is_course_instructor(uuid),
    ple_api.current_session_account_is_course_member(uuid),
    ple_api.current_session_account_owns_course_membership(uuid, uuid),
    ple_api.current_session_account_owns_student_record(uuid, uuid),
    ple_api.current_session_account_owns_workspace(uuid),
    ple_api.current_session_account_can_access_workspace(uuid),
    ple_api.current_session_account_has_course_observer_relationship(uuid),
    ple_api.current_session_account_has_support_capability(uuid, uuid, uuid, text)
    TO ple_app, ple_auth, ple_student;
-- The data and private schema owners compile these predicates into their own
-- RLS policies in the following migration. They remain non-login owners, not
-- runtime application capabilities.
GRANT EXECUTE ON FUNCTION ple_api.current_session_account_id(),
    ple_api.current_session_account_is_course_instructor(uuid),
    ple_api.current_session_account_is_course_member(uuid),
    ple_api.current_session_account_owns_course_membership(uuid, uuid),
    ple_api.current_session_account_owns_student_record(uuid, uuid),
    ple_api.current_session_account_owns_workspace(uuid),
    ple_api.current_session_account_can_access_workspace(uuid),
    ple_api.current_session_account_has_course_observer_relationship(uuid)
    TO ple_data_owner, ple_private_owner;
RESET ROLE;
