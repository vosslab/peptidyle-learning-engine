-- Course, roster, and narrowly scoped support operations. Structures live in
-- the preceding Course modules so this file can resolve their exact roots.

SET LOCAL ROLE ple_private_owner;

CREATE TABLE ple_private.course_roster_support_capability (
    capability_id uuid PRIMARY KEY,
    sysadmin_account_id uuid NOT NULL,
    sysadmin_role text NOT NULL DEFAULT 'sysadmin' CHECK (sysadmin_role = 'sysadmin'),
    course_id uuid NOT NULL REFERENCES ple_data.course_instance (course_id),
    issuer_account_id uuid NOT NULL REFERENCES ple_private.account (account_id),
    purpose text NOT NULL CHECK (char_length(btrim(purpose)) BETWEEN 1 AND 1000),
    issued_at timestamp with time zone NOT NULL,
    expires_at timestamp with time zone NOT NULL CHECK (expires_at > issued_at),
    revoked_at timestamp with time zone,
    FOREIGN KEY (sysadmin_account_id, sysadmin_role)
        REFERENCES ple_private.account (account_id, product_role),
    CHECK (revoked_at IS NULL OR revoked_at >= issued_at)
);

CREATE FUNCTION ple_private.reject_course_roster_support_capability_change()
RETURNS trigger LANGUAGE plpgsql SET search_path = pg_catalog, ple_private AS $$
BEGIN
    IF TG_OP = 'DELETE' THEN
        RAISE EXCEPTION USING ERRCODE = '55000',
            MESSAGE = 'Course Roster Support capabilities are retained as audited evidence';
    END IF;
    IF NEW.capability_id IS DISTINCT FROM OLD.capability_id
       OR NEW.sysadmin_account_id IS DISTINCT FROM OLD.sysadmin_account_id
       OR NEW.course_id IS DISTINCT FROM OLD.course_id
       OR NEW.issuer_account_id IS DISTINCT FROM OLD.issuer_account_id
       OR NEW.purpose IS DISTINCT FROM OLD.purpose
       OR NEW.issued_at IS DISTINCT FROM OLD.issued_at
       OR NEW.expires_at IS DISTINCT FROM OLD.expires_at
       OR (OLD.revoked_at IS NOT NULL AND NEW.revoked_at IS DISTINCT FROM OLD.revoked_at) THEN
        RAISE EXCEPTION USING ERRCODE = '55000',
            MESSAGE = 'Course Roster Support capability identity is immutable';
    END IF;
    RETURN NEW;
END
$$;

CREATE TRIGGER course_roster_support_capability_is_guarded
BEFORE UPDATE OR DELETE ON ple_private.course_roster_support_capability
FOR EACH ROW EXECUTE FUNCTION ple_private.reject_course_roster_support_capability_change();

ALTER TABLE ple_private.course_roster_support_capability ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.course_roster_support_capability FORCE ROW LEVEL SECURITY;
REVOKE ALL ON TABLE ple_private.course_roster_support_capability FROM PUBLIC;
GRANT SELECT, INSERT, UPDATE (revoked_at) ON ple_private.course_roster_support_capability
    TO ple_api_owner;
GRANT USAGE ON SCHEMA ple_private TO ple_audit_owner;
GRANT REFERENCES ON TABLE ple_private.course_roster_support_capability TO ple_audit_owner;
CREATE POLICY course_roster_support_capability_private_owner_access
    ON ple_private.course_roster_support_capability FOR ALL TO ple_private_owner
    USING (true) WITH CHECK (true);
CREATE POLICY course_roster_support_capability_api_owner_access
    ON ple_private.course_roster_support_capability FOR ALL TO ple_api_owner
    USING (true) WITH CHECK (true);
REVOKE ALL ON FUNCTION ple_private.reject_course_roster_support_capability_change() FROM PUBLIC;

RESET ROLE;

SET LOCAL ROLE ple_audit_owner;

CREATE TABLE ple_audit.course_roster_support_capability_event (
    event_id uuid PRIMARY KEY,
    capability_id uuid NOT NULL REFERENCES ple_private.course_roster_support_capability (capability_id),
    course_id uuid NOT NULL REFERENCES ple_data.course_instance (course_id),
    sysadmin_account_id uuid NOT NULL REFERENCES ple_private.account (account_id),
    issuer_account_id uuid NOT NULL REFERENCES ple_private.account (account_id),
    purpose text NOT NULL CHECK (char_length(btrim(purpose)) BETWEEN 1 AND 1000),
    result text NOT NULL CHECK (result IN ('issued', 'revoked')),
    occurred_at timestamp with time zone NOT NULL
);

CREATE FUNCTION ple_audit.reject_course_roster_support_capability_event_change()
RETURNS trigger LANGUAGE plpgsql SET search_path = pg_catalog, ple_audit AS $$
BEGIN
    RAISE EXCEPTION USING ERRCODE = '55000', MESSAGE = 'Support capability audit events are immutable';
END
$$;

CREATE TRIGGER course_roster_support_capability_event_is_immutable
BEFORE UPDATE OR DELETE ON ple_audit.course_roster_support_capability_event
FOR EACH ROW EXECUTE FUNCTION ple_audit.reject_course_roster_support_capability_event_change();

ALTER TABLE ple_audit.course_roster_support_capability_event ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_audit.course_roster_support_capability_event FORCE ROW LEVEL SECURITY;
REVOKE ALL ON TABLE ple_audit.course_roster_support_capability_event FROM PUBLIC;
CREATE POLICY course_roster_support_capability_event_owner_write
    ON ple_audit.course_roster_support_capability_event FOR INSERT TO ple_audit_owner WITH CHECK (true);

CREATE FUNCTION ple_audit.record_course_roster_support_capability_event(
    p_capability_id uuid, p_course_id uuid, p_sysadmin_account_id uuid,
    p_issuer_account_id uuid, p_purpose text, p_result text
)
RETURNS void LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog, ple_audit AS $$
BEGIN
    IF p_capability_id IS NULL OR p_course_id IS NULL OR p_sysadmin_account_id IS NULL
       OR p_issuer_account_id IS NULL OR p_purpose IS NULL OR p_purpose <> btrim(p_purpose)
       OR char_length(p_purpose) NOT BETWEEN 1 AND 1000 OR p_result NOT IN ('issued', 'revoked') THEN
        RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Support capability audit arguments are invalid';
    END IF;
    INSERT INTO ple_audit.course_roster_support_capability_event
    VALUES (pg_catalog.gen_random_uuid(), p_capability_id, p_course_id, p_sysadmin_account_id,
            p_issuer_account_id, p_purpose, p_result, pg_catalog.transaction_timestamp());
END
$$;

REVOKE ALL ON FUNCTION ple_audit.reject_course_roster_support_capability_event_change(),
    ple_audit.record_course_roster_support_capability_event(uuid, uuid, uuid, uuid, text, text)
    FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_audit.record_course_roster_support_capability_event(
    uuid, uuid, uuid, uuid, text, text
) TO ple_api_owner;

RESET ROLE;

SET LOCAL ROLE ple_data_owner;

CREATE POLICY course_instance_data_owner_instructor_read ON ple_data.course_instance
    FOR SELECT TO ple_data_owner
    USING (ple_api.current_session_account_is_course_instructor(course_id));
CREATE POLICY course_instance_member_read ON ple_data.course_instance
    FOR SELECT TO ple_app USING (ple_api.current_session_account_is_course_member(course_id));
CREATE POLICY course_membership_instructor_or_self_read ON ple_data.course_membership
    FOR SELECT TO ple_app USING (
        ple_api.current_session_account_is_course_instructor(course_id)
        OR ple_api.current_session_account_owns_course_membership(course_id, membership_id)
    );
CREATE POLICY student_record_instructor_or_self_read ON ple_data.student_record
    FOR SELECT TO ple_app USING (
        ple_api.current_session_account_is_course_instructor(course_id)
        OR ple_api.current_session_account_owns_student_record(course_id, student_record_id)
    );

RESET ROLE;

SET LOCAL ROLE ple_api_owner;

CREATE FUNCTION ple_api.current_session_account_has_course_roster_support(
    p_capability_id uuid, p_course_id uuid
)
RETURNS boolean LANGUAGE sql VOLATILE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private
AS $$
    SELECT p_capability_id IS NOT NULL AND EXISTS (
        SELECT 1 FROM ple_private.course_roster_support_capability AS capability
         WHERE capability.capability_id = p_capability_id AND capability.course_id = p_course_id
           AND capability.revoked_at IS NULL AND capability.expires_at > pg_catalog.clock_timestamp()
           AND capability.sysadmin_account_id = ple_api.current_session_account_id()
    )
$$;

CREATE FUNCTION ple_api.read_course_theme(p_course_id uuid)
RETURNS TABLE(course_theme text) LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data
AS $$
    SELECT course_theme FROM ple_data.course_instance
     WHERE course_id = p_course_id AND ple_api.current_session_account_is_course_member(p_course_id)
$$;

CREATE FUNCTION ple_api.update_course_theme(p_course_id uuid, p_course_theme text)
RETURNS TABLE(course_theme text) LANGUAGE sql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data
AS $$
    UPDATE ple_data.course_instance SET course_theme = p_course_theme
     WHERE course_id = p_course_id AND ple_api.current_session_account_is_course_instructor(p_course_id)
 RETURNING course_theme
$$;

CREATE FUNCTION ple_api.resolve_course_navigation(p_reference_number bigint)
RETURNS TABLE(course_id uuid) LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data
AS $$
    SELECT course_id FROM ple_data.course_instance
     WHERE reference_number = p_reference_number
       AND ple_api.current_session_account_is_course_member(course_id)
$$;

CREATE FUNCTION ple_api.read_course_summary(p_course_id uuid)
RETURNS TABLE(course_id uuid, reference_number bigint, short_name text, long_name text,
              term_starts_on date, term_ends_on date, membership_role text)
LANGUAGE sql STABLE SECURITY DEFINER SET search_path = pg_catalog, ple_api, ple_data AS $$
    SELECT course.course_id, course.reference_number, course.course_short_name, course.course_long_name,
           course.term_starts_on, course.term_ends_on, membership.role
      FROM ple_data.course_instance AS course
      JOIN ple_data.course_membership AS membership
        ON membership.course_id = course.course_id
       AND membership.account_id = ple_api.current_session_account_id()
       AND ple_data.course_membership_is_active(membership.membership_id)
     WHERE course.course_id = p_course_id
       AND ple_api.current_session_account_is_course_member(p_course_id)
$$;

CREATE FUNCTION ple_api.create_course_instance(
    p_course_id uuid, p_origin_id uuid, p_membership_id uuid, p_event_id uuid,
    p_blueprint_reference bigint, p_blueprint_revision bigint,
    p_short_name text, p_long_name text, p_term_start date, p_term_end date,
    p_assigned_instructor_reference bigint
)
RETURNS TABLE(reference_number bigint, short_name text, long_name text, term_starts_on date,
              term_ends_on date, creator_is_assigned_instructor boolean)
LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog, ple_api, ple_data, ple_private, ple_audit AS $$
DECLARE actor uuid; assigned uuid; course_reference bigint; now_at timestamptz;
BEGIN
    IF p_course_id IS NULL OR p_origin_id IS NULL OR p_membership_id IS NULL OR p_event_id IS NULL
       OR p_blueprint_reference NOT BETWEEN 1 AND 2147483647 OR p_blueprint_revision <= 0
       OR p_short_name IS NULL OR p_short_name <> btrim(p_short_name)
       OR char_length(p_short_name) NOT BETWEEN 1 AND 200
       OR p_long_name IS NULL OR p_long_name <> btrim(p_long_name)
       OR char_length(p_long_name) NOT BETWEEN 1 AND 200
       OR p_term_start IS NULL OR p_term_end IS NULL OR p_term_end < p_term_start THEN
        RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Course Instance creation arguments are invalid';
    END IF;
    actor := ple_api.current_session_account_id();
    IF ple_api.current_session_account_is_instructor() THEN
        assigned := actor;
        IF p_assigned_instructor_reference IS NOT NULL AND NOT EXISTS (
            SELECT 1 FROM ple_private.account AS account
             WHERE account.account_id = actor
               AND account.reference_number = p_assigned_instructor_reference
        ) THEN
            RAISE EXCEPTION USING ERRCODE = '42501',
                MESSAGE = 'an Instructor may create a Course Instance only for self';
        END IF;
    ELSIF ple_api.current_session_account_is_sysadmin() THEN
        SELECT account.account_id INTO assigned FROM ple_private.account AS account
         WHERE account.reference_number = p_assigned_instructor_reference
           AND account.product_role = 'instructor';
        IF NOT FOUND THEN
            RAISE EXCEPTION USING ERRCODE = '22023',
                MESSAGE = 'the selected Assigned Instructor is unavailable';
        END IF;
    ELSE
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Course Instance creation requires an active Instructor or Sysadmin Account';
    END IF;
    IF NOT EXISTS (
        SELECT 1
          FROM ple_data.blueprint_course_revision AS revision
          JOIN ple_data.blueprint_course AS blueprint
            ON blueprint.reference_number = revision.blueprint_course_reference_number
         WHERE revision.blueprint_course_reference_number = p_blueprint_reference
           AND revision.blueprint_revision_number = p_blueprint_revision
           AND blueprint.availability = 'available'
    ) THEN
        RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Course Instance source is unavailable';
    END IF;
    now_at := pg_catalog.transaction_timestamp();
    INSERT INTO ple_data.course_instance (
        course_id, blueprint_course_reference_number, blueprint_revision_number,
        assigned_instructor_account_id, course_short_name, course_long_name,
        term_starts_on, term_ends_on, created_at
    ) VALUES (
        p_course_id, p_blueprint_reference, p_blueprint_revision, assigned,
        p_short_name, p_long_name, p_term_start, p_term_end, now_at
    ) RETURNING ple_data.course_instance.reference_number INTO course_reference;
    INSERT INTO ple_data.course_origin (
        course_origin_id, course_id, blueprint_course_reference_number,
        blueprint_revision_number, source_course_id, created_at
    ) VALUES (p_origin_id, p_course_id, p_blueprint_reference, p_blueprint_revision, NULL, now_at);
    INSERT INTO ple_data.course_membership (membership_id, course_id, account_id, role, joined_at)
    VALUES (p_membership_id, p_course_id, assigned, 'instructor', now_at);
    PERFORM ple_audit.record_course_instance_creation_event(
        p_event_id, p_course_id, course_reference, p_blueprint_reference, p_blueprint_revision,
        assigned, actor, now_at
    );
    RETURN QUERY SELECT course_reference, p_short_name, p_long_name, p_term_start, p_term_end,
        actor = assigned;
END
$$;

CREATE FUNCTION ple_api.list_course_instances()
RETURNS TABLE(reference_number bigint, short_name text, long_name text, term_starts_on date,
              term_ends_on date, course_theme text)
LANGUAGE sql STABLE SECURITY DEFINER SET search_path = pg_catalog, ple_api, ple_data AS $$
    SELECT course.reference_number, course.course_short_name, course.course_long_name,
           course.term_starts_on, course.term_ends_on, course.course_theme
      FROM ple_data.course_instance AS course
      JOIN ple_data.course_membership AS membership
        ON membership.course_id = course.course_id
       AND membership.account_id = ple_api.current_session_account_id()
       AND membership.role = 'instructor'
       AND ple_data.course_membership_is_active(membership.membership_id)
     WHERE ple_api.current_session_account_is_instructor()
     ORDER BY course.course_long_name, course.reference_number
$$;

CREATE FUNCTION ple_api.load_course_instance(p_reference bigint)
RETURNS TABLE(reference_number bigint, short_name text, long_name text, term_starts_on date,
              term_ends_on date, course_theme text, is_assigned_instructor boolean,
              active_instructor_count bigint)
LANGUAGE sql STABLE SECURITY DEFINER SET search_path = pg_catalog, ple_api, ple_data AS $$
    SELECT course.reference_number, course.course_short_name, course.course_long_name,
           course.term_starts_on, course.term_ends_on, course.course_theme,
           course.assigned_instructor_account_id = ple_api.current_session_account_id(),
           (SELECT count(*) FROM ple_data.course_membership AS teammate
             WHERE teammate.course_id = course.course_id AND teammate.role = 'instructor'
               AND ple_data.course_membership_is_active(teammate.membership_id))
      FROM ple_data.course_instance AS course
      JOIN ple_data.course_membership AS membership
        ON membership.course_id = course.course_id
       AND membership.account_id = ple_api.current_session_account_id()
       AND membership.role = 'instructor'
       AND ple_data.course_membership_is_active(membership.membership_id)
     WHERE p_reference BETWEEN 1 AND 2147483647 AND course.reference_number = p_reference
       AND ple_api.current_session_account_is_instructor()
$$;

CREATE FUNCTION ple_api.list_course_creation_instructors()
RETURNS TABLE(reference_number bigint) LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private
AS $$
    SELECT reference_number FROM ple_private.account
     WHERE product_role = 'instructor' AND ple_api.current_session_account_is_sysadmin()
     ORDER BY reference_number
$$;

CREATE FUNCTION ple_api.list_course_roster(p_reference bigint, p_support uuid DEFAULT NULL)
RETURNS TABLE(roster_id text, roster_email text, state text)
LANGUAGE sql VOLATILE SECURITY DEFINER SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
    SELECT profile.roster_id, profile.roster_email,
           CASE WHEN EXISTS (
               SELECT 1 FROM ple_data.course_membership AS membership
                WHERE membership.course_id = course.course_id
                  AND membership.account_id = profile.student_account_id
                  AND membership.role = 'student'
                  AND ple_data.course_membership_is_active(membership.membership_id)
           ) THEN 'active_student' ELSE 'invitation_pending' END
      FROM ple_data.course_instance AS course
      JOIN ple_private.course_roster_profile AS profile ON profile.course_id = course.course_id
     WHERE course.reference_number = p_reference
       AND (
           EXISTS (
               SELECT 1 FROM ple_data.course_membership AS membership
                WHERE membership.course_id = course.course_id
                  AND membership.account_id = profile.student_account_id
                  AND membership.role = 'student'
                  AND ple_data.course_membership_is_active(membership.membership_id)
           )
           OR EXISTS (
               SELECT 1 FROM ple_private.course_invitation AS invitation
                WHERE invitation.course_id = course.course_id
                  AND invitation.target_account_id = profile.student_account_id
                  AND invitation.membership_role = 'student'
                  AND invitation.expires_at > pg_catalog.clock_timestamp()
                  AND NOT EXISTS (
                      SELECT 1 FROM ple_private.course_invitation_event AS event
                       WHERE event.invitation_id = invitation.invitation_id
                  )
           )
       )
       AND (ple_api.current_session_account_is_course_instructor(course.course_id)
            OR ple_api.current_session_account_has_course_roster_support(p_support, course.course_id))
     ORDER BY profile.roster_id
$$;

CREATE FUNCTION ple_api.read_course_roster_support(p_capability_id uuid)
RETURNS TABLE(roster_id text, roster_email text, state text)
LANGUAGE sql VOLATILE SECURITY DEFINER SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
    SELECT roster.roster_id, roster.roster_email, roster.state
      FROM ple_private.course_roster_support_capability AS capability
      JOIN ple_data.course_instance AS course ON course.course_id = capability.course_id
      CROSS JOIN LATERAL ple_api.list_course_roster(course.reference_number, capability.capability_id)
          AS roster
     WHERE capability.capability_id = p_capability_id
$$;

CREATE FUNCTION ple_api.list_live_student_course_landing()
RETURNS TABLE(course_reference_number bigint, course_short_name text, course_long_name text)
LANGUAGE sql STABLE SECURITY DEFINER SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
    SELECT course.reference_number, course.course_short_name, course.course_long_name
      FROM ple_private.account AS account
      JOIN LATERAL (
          SELECT event.state
            FROM ple_private.account_state_event AS event
           WHERE event.account_id = account.account_id
           ORDER BY event.occurred_at DESC, event.event_id DESC
           LIMIT 1
      ) AS account_state ON account_state.state = 'active'
      JOIN ple_data.course_membership AS membership
        ON membership.account_id = account.account_id
       AND membership.role = 'student'
       AND ple_data.course_membership_is_active(membership.membership_id)
      JOIN ple_data.course_instance AS course ON course.course_id = membership.course_id
     WHERE account.account_id = ple_api.current_session_account_id()
       AND account.product_role = 'student'
     ORDER BY course.course_long_name, course.reference_number
$$;

CREATE FUNCTION ple_api.list_pending_student_course_invitations()
RETURNS TABLE(course_reference_number bigint, course_short_name text, course_long_name text)
LANGUAGE sql VOLATILE SECURITY DEFINER SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
    SELECT course.reference_number, course.course_short_name, course.course_long_name
      FROM ple_private.account AS account
      JOIN LATERAL (
          SELECT event.state
            FROM ple_private.account_state_event AS event
           WHERE event.account_id = account.account_id
           ORDER BY event.occurred_at DESC, event.event_id DESC
           LIMIT 1
      ) AS account_state ON account_state.state = 'active'
      JOIN LATERAL (
          SELECT DISTINCT ON (invitation.course_id) invitation.course_id
            FROM ple_private.course_invitation AS invitation
           WHERE invitation.target_account_id = account.account_id
             AND invitation.membership_role = 'student'
             AND invitation.expires_at > pg_catalog.clock_timestamp()
             AND NOT EXISTS (
                 SELECT 1 FROM ple_private.course_invitation_event AS event
                  WHERE event.invitation_id = invitation.invitation_id
             )
             AND NOT EXISTS (
                 SELECT 1 FROM ple_data.course_membership AS membership
                  WHERE membership.course_id = invitation.course_id
                    AND membership.account_id = account.account_id
                    AND membership.role = 'student'
                    AND ple_data.course_membership_is_active(membership.membership_id)
             )
           ORDER BY invitation.course_id, invitation.issued_at DESC, invitation.invitation_id DESC
      ) AS pending_invitation ON true
      JOIN ple_data.course_instance AS course
        ON course.course_id = pending_invitation.course_id
     WHERE account.account_id = ple_api.current_session_account_id()
       AND account.product_role = 'student'
     ORDER BY course.course_long_name, course.reference_number
$$;

CREATE FUNCTION ple_api.export_pending_course_invitations(
    p_course_reference_number bigint
)
RETURNS TABLE(roster_email text, roster_id text)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
DECLARE v_course_id uuid;
BEGIN
    IF p_course_reference_number NOT BETWEEN 1 AND 2147483647 THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Invitation export arguments are invalid';
    END IF;
    SELECT course.course_id INTO v_course_id
      FROM ple_data.course_instance AS course
     WHERE course.reference_number = p_course_reference_number;
    IF NOT FOUND OR NOT ple_api.current_session_account_is_course_instructor(v_course_id) THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Invitation export requires a current Instructor Course Membership';
    END IF;
    RETURN QUERY
    SELECT profile.roster_email, profile.roster_id
      FROM ple_private.course_roster_profile AS profile
      JOIN LATERAL (
          SELECT invitation.invitation_id
            FROM ple_private.course_invitation AS invitation
           WHERE invitation.course_id = profile.course_id
             AND invitation.target_account_id = profile.student_account_id
             AND invitation.membership_role = 'student'
             AND invitation.expires_at > pg_catalog.clock_timestamp()
             AND NOT EXISTS (
                 SELECT 1 FROM ple_private.course_invitation_event AS event
                  WHERE event.invitation_id = invitation.invitation_id
             )
           ORDER BY invitation.issued_at DESC, invitation.invitation_id DESC
           LIMIT 1
      ) AS pending_invitation ON true
     WHERE profile.course_id = v_course_id
       AND NOT EXISTS (
           SELECT 1 FROM ple_data.course_membership AS membership
            WHERE membership.course_id = profile.course_id
              AND membership.account_id = profile.student_account_id
              AND membership.role = 'student'
              AND ple_data.course_membership_is_active(membership.membership_id)
       )
     ORDER BY profile.roster_id;
END
$$;

CREATE FUNCTION ple_api.load_invitation_export_course(
    p_course_reference_number bigint
)
RETURNS TABLE(course_name text)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data AS $$
DECLARE v_course_id uuid;
BEGIN
    IF p_course_reference_number NOT BETWEEN 1 AND 2147483647 THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Invitation export arguments are invalid';
    END IF;
    SELECT course.course_id INTO v_course_id
      FROM ple_data.course_instance AS course
     WHERE course.reference_number = p_course_reference_number;
    IF NOT FOUND OR NOT ple_api.current_session_account_is_course_instructor(v_course_id) THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Invitation export requires a current Instructor Course Membership';
    END IF;
    RETURN QUERY
    SELECT course.course_long_name
      FROM ple_data.course_instance AS course
     WHERE course.course_id = v_course_id;
END
$$;

CREATE FUNCTION ple_api.import_course_roster(
    p_reference bigint, p_normalized text[], p_delivery text[], p_roster text[]
)
RETURNS TABLE(roster_id text, roster_email text, state text)
LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog, ple_api, ple_data, ple_private, ple_audit AS $$
DECLARE course uuid; actor uuid; student uuid; item integer; now_at timestamptz;
BEGIN
    IF p_reference NOT BETWEEN 1 AND 2147483647 OR cardinality(p_normalized) NOT BETWEEN 1 AND 50
       OR cardinality(p_normalized) IS DISTINCT FROM cardinality(p_delivery)
       OR cardinality(p_normalized) IS DISTINCT FROM cardinality(p_roster) THEN
        RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Course Roster Import arguments are invalid';
    END IF;
    SELECT course_id INTO course FROM ple_data.course_instance WHERE reference_number = p_reference;
    IF NOT FOUND OR NOT ple_api.current_session_account_is_course_instructor(course) THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Course Roster Import requires a current Instructor Course Membership';
    END IF;
    actor := ple_api.current_session_account_id();
    now_at := pg_catalog.transaction_timestamp();
    FOR item IN 1..cardinality(p_normalized) LOOP
        IF p_normalized[item] IS NULL OR p_delivery[item] IS NULL OR p_roster[item] IS NULL
           OR p_roster[item] !~ '^[A-Za-z0-9._-]+$' THEN
            RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Course Roster Import row is invalid';
        END IF;
        student := ple_private.resolve_or_create_student_account(p_normalized[item], p_delivery[item]);
        INSERT INTO ple_private.course_roster_profile
        VALUES (pg_catalog.gen_random_uuid(), course, student, p_normalized[item], p_roster[item], now_at)
        ON CONFLICT (course_id, student_account_id) DO NOTHING;
        IF EXISTS (
            SELECT 1 FROM ple_data.course_membership AS membership
             WHERE membership.course_id = course AND membership.account_id = student
               AND membership.role = 'student' AND ple_data.course_membership_is_active(membership.membership_id)
        ) THEN
            roster_id := p_roster[item]; roster_email := p_normalized[item]; state := 'active_student';
            RETURN NEXT; CONTINUE;
        END IF;
        IF NOT EXISTS (
            SELECT 1 FROM ple_private.course_invitation AS invitation
             WHERE invitation.course_id = course AND invitation.target_account_id = student
               AND invitation.membership_role = 'student'
               AND invitation.expires_at > pg_catalog.clock_timestamp()
               AND NOT EXISTS (
                   SELECT 1 FROM ple_private.course_invitation_event AS event
                    WHERE event.invitation_id = invitation.invitation_id
               )
        ) THEN
            INSERT INTO ple_private.course_invitation
            VALUES (pg_catalog.gen_random_uuid(), course, student, 'student', now_at, now_at + interval '7 days');
            PERFORM ple_audit.record_course_roster_event(course, student, actor, 'invitation_created');
        END IF;
        roster_id := p_roster[item]; roster_email := p_normalized[item]; state := 'invitation_pending';
        RETURN NEXT;
    END LOOP;
END
$$;

CREATE FUNCTION ple_api.claim_course_invitation(
    p_student_record uuid, p_membership uuid, p_event uuid, p_reference bigint
)
RETURNS TABLE(active_student_membership boolean)
LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog, ple_api, ple_data, ple_private, ple_audit AS $$
DECLARE course uuid; student uuid; invitation uuid; existing_record uuid; now_at timestamptz;
BEGIN
    student := ple_api.current_session_account_id();
    SELECT course_id INTO course FROM ple_data.course_instance WHERE reference_number = p_reference;
    IF NOT FOUND OR student IS NULL THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Course Invitation is unavailable';
    END IF;
    IF EXISTS (
        SELECT 1 FROM ple_data.course_membership
         WHERE course_id = course AND account_id = student AND role = 'student'
           AND ple_data.course_membership_is_active(membership_id)
    ) THEN
        RETURN QUERY SELECT true;
        RETURN;
    END IF;
    SELECT invitation_id INTO invitation FROM ple_private.course_invitation
     WHERE course_id = course AND target_account_id = student AND membership_role = 'student'
       AND expires_at > pg_catalog.clock_timestamp()
       AND NOT EXISTS (
           SELECT 1 FROM ple_private.course_invitation_event AS event
            WHERE event.invitation_id = course_invitation.invitation_id
       )
     ORDER BY issued_at DESC LIMIT 1 FOR UPDATE;
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Course Invitation is unavailable';
    END IF;
    now_at := pg_catalog.transaction_timestamp();
    INSERT INTO ple_data.student_record VALUES (p_student_record, course, student, now_at)
    ON CONFLICT (course_id, student_account_id) DO NOTHING;
    SELECT student_record_id INTO existing_record FROM ple_data.student_record
     WHERE course_id = course AND student_account_id = student;
    INSERT INTO ple_data.course_membership
    VALUES (p_membership, course, student, 'student', existing_record, now_at);
    INSERT INTO ple_private.course_invitation_event
    VALUES (p_event, invitation, 'accepted', student, now_at, 'student accepted Course Invitation');
    PERFORM ple_audit.record_course_roster_event(course, student, student, 'invitation_claimed');
    RETURN QUERY SELECT true;
END
$$;

CREATE FUNCTION ple_api.revoke_course_roster_entry(
    p_event uuid, p_reference bigint, p_roster_id text
)
RETURNS void LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private, ple_audit AS $$
DECLARE course uuid; student uuid; membership uuid; invitation uuid; actor uuid; now_at timestamptz;
BEGIN
    SELECT course_id INTO course FROM ple_data.course_instance WHERE reference_number = p_reference;
    IF NOT FOUND OR NOT ple_api.current_session_account_is_course_instructor(course) THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Course roster entry is unavailable';
    END IF;
    SELECT student_account_id INTO student FROM ple_private.course_roster_profile
     WHERE course_id = course AND roster_id = p_roster_id;
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Course roster entry is unavailable';
    END IF;
    actor := ple_api.current_session_account_id(); now_at := pg_catalog.transaction_timestamp();
    SELECT membership_id INTO membership FROM ple_data.course_membership
     WHERE course_id = course AND account_id = student AND role = 'student'
       AND ple_data.course_membership_is_active(membership_id) LIMIT 1;
    IF FOUND THEN
        INSERT INTO ple_data.course_membership_event
        VALUES (p_event, membership, 'ended', now_at, 'Instructor revoked Student course access');
        PERFORM ple_audit.record_course_roster_event(course, student, actor, 'student_access_revoked');
        RETURN;
    END IF;
    SELECT invitation_id INTO invitation FROM ple_private.course_invitation
     WHERE course_id = course AND target_account_id = student AND membership_role = 'student'
       AND expires_at > now_at
       AND NOT EXISTS (
           SELECT 1 FROM ple_private.course_invitation_event AS event
            WHERE event.invitation_id = course_invitation.invitation_id
       )
     ORDER BY issued_at DESC LIMIT 1 FOR UPDATE;
    IF FOUND THEN
        INSERT INTO ple_private.course_invitation_event
        VALUES (p_event, invitation, 'revoked', actor, now_at, 'Instructor revoked pending Course Invitation');
        PERFORM ple_audit.record_course_roster_event(course, student, actor, 'student_access_revoked');
    END IF;
END
$$;

CREATE FUNCTION ple_api.issue_course_roster_support(
    p_course_reference_number bigint, p_sysadmin_reference_number bigint, p_purpose text, p_capability_id uuid
)
RETURNS TABLE(capability_id uuid, course_reference_number bigint, sysadmin_reference_number bigint,
              purpose text, expires_at_millis bigint, revoked_at_millis bigint)
LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog, ple_api, ple_audit, ple_data, ple_private AS $$
DECLARE course uuid; issuer uuid; sysadmin uuid; now_at timestamptz;
BEGIN
    IF p_course_reference_number NOT BETWEEN 1 AND 2147483647
       OR p_sysadmin_reference_number NOT BETWEEN 1 AND 2147483647 OR p_capability_id IS NULL
       OR p_purpose IS NULL OR p_purpose <> btrim(p_purpose)
       OR char_length(p_purpose) NOT BETWEEN 1 AND 1000 THEN
        RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Support capability input is invalid';
    END IF;
    SELECT course_id INTO course FROM ple_data.course_instance WHERE reference_number = p_course_reference_number;
    IF NOT FOUND OR NOT ple_api.current_session_account_is_course_instructor(course) THEN RETURN; END IF;
    issuer := ple_api.current_session_account_id();
    SELECT account.account_id INTO sysadmin FROM ple_private.account AS account
    JOIN LATERAL (
        SELECT event.state FROM ple_private.account_state_event AS event
         WHERE event.account_id = account.account_id
         ORDER BY event.occurred_at DESC, event.event_id DESC LIMIT 1
    ) AS state_event ON state_event.state = 'active'
    WHERE account.reference_number = p_sysadmin_reference_number AND account.product_role = 'sysadmin'
    FOR KEY SHARE OF account;
    IF NOT FOUND THEN RETURN; END IF;
    now_at := pg_catalog.transaction_timestamp();
    INSERT INTO ple_private.course_roster_support_capability (
        capability_id, sysadmin_account_id, course_id, issuer_account_id, purpose, issued_at, expires_at
    ) VALUES (p_capability_id, sysadmin, course, issuer, p_purpose, now_at, now_at + interval '1 hour');
    PERFORM ple_audit.record_course_roster_support_capability_event(
        p_capability_id, course, sysadmin, issuer, p_purpose, 'issued'
    );
    RETURN QUERY
    SELECT capability.capability_id, instance.reference_number, account.reference_number,
           capability.purpose, (extract(epoch FROM capability.expires_at) * 1000)::bigint,
           (extract(epoch FROM capability.revoked_at) * 1000)::bigint
      FROM ple_private.course_roster_support_capability AS capability
      JOIN ple_data.course_instance AS instance ON instance.course_id = capability.course_id
      JOIN ple_private.account AS account ON account.account_id = capability.sysadmin_account_id
     WHERE capability.capability_id = p_capability_id;
END
$$;

CREATE FUNCTION ple_api.revoke_course_roster_support(
    p_course_reference_number bigint, p_capability_id uuid
)
RETURNS TABLE(capability_id uuid, course_reference_number bigint, sysadmin_reference_number bigint,
              purpose text, expires_at_millis bigint, revoked_at_millis bigint)
LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog, ple_api, ple_audit, ple_data, ple_private AS $$
DECLARE course uuid; issuer uuid; capability ple_private.course_roster_support_capability%ROWTYPE;
BEGIN
    SELECT course_id INTO course FROM ple_data.course_instance WHERE reference_number = p_course_reference_number;
    IF NOT FOUND OR NOT ple_api.current_session_account_is_course_instructor(course) THEN RETURN; END IF;
    issuer := ple_api.current_session_account_id();
    SELECT * INTO capability FROM ple_private.course_roster_support_capability
     WHERE capability_id = p_capability_id AND course_id = course FOR UPDATE;
    IF NOT FOUND OR capability.issuer_account_id <> issuer OR capability.revoked_at IS NOT NULL THEN RETURN; END IF;
    UPDATE ple_private.course_roster_support_capability SET revoked_at = pg_catalog.transaction_timestamp()
     WHERE capability_id = p_capability_id;
    PERFORM ple_audit.record_course_roster_support_capability_event(
        p_capability_id, course, capability.sysadmin_account_id, issuer, capability.purpose, 'revoked'
    );
    RETURN QUERY
    SELECT stored.capability_id, instance.reference_number, account.reference_number,
           stored.purpose, (extract(epoch FROM stored.expires_at) * 1000)::bigint,
           (extract(epoch FROM stored.revoked_at) * 1000)::bigint
      FROM ple_private.course_roster_support_capability AS stored
      JOIN ple_data.course_instance AS instance ON instance.course_id = stored.course_id
      JOIN ple_private.account AS account ON account.account_id = stored.sysadmin_account_id
     WHERE stored.capability_id = p_capability_id;
END
$$;

REVOKE ALL ON FUNCTION ple_api.current_session_account_has_course_roster_support(uuid, uuid),
    ple_api.read_course_theme(uuid), ple_api.update_course_theme(uuid, text),
    ple_api.resolve_course_navigation(bigint), ple_api.read_course_summary(uuid),
    ple_api.create_course_instance(uuid, uuid, uuid, uuid, bigint, bigint, text, text, date, date, bigint),
    ple_api.list_course_instances(), ple_api.load_course_instance(bigint),
    ple_api.list_course_creation_instructors(), ple_api.list_course_roster(bigint, uuid),
    ple_api.read_course_roster_support(uuid), ple_api.list_live_student_course_landing(),
    ple_api.list_pending_student_course_invitations(),
    ple_api.export_pending_course_invitations(bigint),
    ple_api.load_invitation_export_course(bigint),
    ple_api.import_course_roster(bigint, text[], text[], text[]),
    ple_api.claim_course_invitation(uuid, uuid, uuid, bigint),
    ple_api.revoke_course_roster_entry(uuid, bigint, text),
    ple_api.issue_course_roster_support(bigint, bigint, text, uuid),
    ple_api.revoke_course_roster_support(bigint, uuid) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_api.current_session_account_has_course_roster_support(uuid, uuid),
    ple_api.read_course_theme(uuid), ple_api.update_course_theme(uuid, text),
    ple_api.resolve_course_navigation(bigint), ple_api.read_course_summary(uuid),
    ple_api.create_course_instance(uuid, uuid, uuid, uuid, bigint, bigint, text, text, date, date, bigint),
    ple_api.list_course_instances(), ple_api.load_course_instance(bigint),
    ple_api.list_course_creation_instructors(), ple_api.list_course_roster(bigint, uuid),
    ple_api.read_course_roster_support(uuid), ple_api.list_live_student_course_landing(),
    ple_api.list_pending_student_course_invitations(),
    ple_api.export_pending_course_invitations(bigint),
    ple_api.load_invitation_export_course(bigint),
    ple_api.import_course_roster(bigint, text[], text[], text[]),
    ple_api.claim_course_invitation(uuid, uuid, uuid, bigint),
    ple_api.revoke_course_roster_entry(uuid, bigint, text),
    ple_api.issue_course_roster_support(bigint, bigint, text, uuid),
    ple_api.revoke_course_roster_support(bigint, uuid) TO ple_app;

RESET ROLE;
