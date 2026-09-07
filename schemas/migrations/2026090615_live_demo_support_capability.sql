-- M17 closed exact-course Sysadmin support capability.  The baseline is still
-- disposable, so replace the obsolete generic repair vocabulary outright.

SET LOCAL ROLE ple_private_owner;
ALTER TABLE ple_private.sysadmin_support_capability
    DROP CONSTRAINT sysadmin_support_capability_student_record_course_matches,
    DROP COLUMN student_record_id,
    DROP CONSTRAINT sysadmin_support_capability_operation_kind_check,
    ADD COLUMN issuer_account_id uuid REFERENCES ple_private.account (account_id),
    ADD COLUMN minimum_projection text,
    ADD CONSTRAINT sysadmin_support_capability_operation_kind_check
        CHECK (operation_kind = 'course_roster_support'),
    ADD CONSTRAINT sysadmin_support_capability_minimum_projection_check
        CHECK (minimum_projection = 'course_roster');
ALTER TABLE ple_private.sysadmin_support_capability
    ALTER COLUMN course_id SET NOT NULL,
    ALTER COLUMN issuer_account_id SET NOT NULL,
    ALTER COLUMN minimum_projection SET NOT NULL;

CREATE POLICY support_capability_private_owner_issue
    ON ple_private.sysadmin_support_capability FOR INSERT TO ple_private_owner WITH CHECK (true);
CREATE POLICY support_capability_private_owner_revoke
    ON ple_private.sysadmin_support_capability FOR UPDATE TO ple_private_owner
    USING (true) WITH CHECK (true);
CREATE POLICY support_capability_private_owner_read
    ON ple_private.sysadmin_support_capability FOR SELECT TO ple_private_owner USING (true);
GRANT SELECT, INSERT, UPDATE ON ple_private.sysadmin_support_capability TO ple_private_owner;
GRANT SELECT ON ple_private.account, ple_private.account_state_event TO ple_private_owner;
-- The issuing definer locks the selected active Sysadmin while creating the
-- capability. Forced RLS requires this narrow lock policy even though no
-- Sysadmin Account attribute is changed.
CREATE POLICY account_private_owner_live_demo_support_sysadmin_lock
    ON ple_private.account FOR UPDATE TO ple_private_owner
    USING (product_role = 'sysadmin') WITH CHECK (product_role = 'sysadmin');
GRANT USAGE ON SCHEMA ple_private TO ple_audit_owner;
GRANT REFERENCES ON TABLE ple_private.account, ple_private.sysadmin_support_capability TO ple_audit_owner;
RESET ROLE;

SET LOCAL ROLE ple_data_owner;
GRANT SELECT ON ple_data.course_instance, ple_data.course_membership,
    ple_data.course_membership_event TO ple_private_owner;
GRANT EXECUTE ON FUNCTION ple_data.course_membership_is_active(uuid) TO ple_private_owner;
CREATE POLICY course_instance_private_owner_support_read
    ON ple_data.course_instance FOR SELECT TO ple_private_owner USING (true);
CREATE POLICY course_membership_private_owner_support_read
    ON ple_data.course_membership FOR SELECT TO ple_private_owner USING (true);
CREATE POLICY course_membership_event_private_owner_support_read
    ON ple_data.course_membership_event FOR SELECT TO ple_private_owner USING (true);
RESET ROLE;

SET LOCAL ROLE ple_audit_owner;
CREATE TABLE ple_audit.sysadmin_support_capability_event (
    event_id uuid PRIMARY KEY,
    capability_id uuid NOT NULL REFERENCES ple_private.sysadmin_support_capability (capability_id),
    course_id uuid NOT NULL REFERENCES ple_data.course_instance (course_id),
    sysadmin_account_id uuid NOT NULL REFERENCES ple_private.account (account_id),
    issuer_account_id uuid NOT NULL REFERENCES ple_private.account (account_id),
    operation_kind text NOT NULL CHECK (operation_kind = 'course_roster_support'),
    purpose text NOT NULL CHECK (char_length(btrim(purpose)) BETWEEN 1 AND 1000),
    result text NOT NULL CHECK (result IN ('issued', 'revoked')),
    occurred_at timestamp with time zone NOT NULL
);
ALTER TABLE ple_audit.sysadmin_support_capability_event ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_audit.sysadmin_support_capability_event FORCE ROW LEVEL SECURITY;
CREATE POLICY sysadmin_support_capability_event_audit_owner_create
    ON ple_audit.sysadmin_support_capability_event FOR INSERT TO ple_audit_owner WITH CHECK (true);
CREATE FUNCTION ple_audit.reject_sysadmin_support_capability_event_change()
RETURNS trigger LANGUAGE plpgsql SET search_path = pg_catalog, ple_audit AS $$
BEGIN RAISE EXCEPTION USING ERRCODE = '55000', MESSAGE = 'Support capability audit events are immutable'; END $$;
CREATE TRIGGER sysadmin_support_capability_event_is_immutable
BEFORE UPDATE OR DELETE ON ple_audit.sysadmin_support_capability_event
FOR EACH ROW EXECUTE FUNCTION ple_audit.reject_sysadmin_support_capability_event_change();
CREATE FUNCTION ple_audit.record_live_demo_support_capability_event(
    p_capability_id uuid, p_course_id uuid, p_sysadmin_account_id uuid,
    p_issuer_account_id uuid, p_operation_kind text, p_purpose text, p_result text
) RETURNS void LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog, ple_audit AS $$
BEGIN
    IF p_capability_id IS NULL OR p_course_id IS NULL OR p_sysadmin_account_id IS NULL
       OR p_issuer_account_id IS NULL OR p_operation_kind <> 'course_roster_support'
       OR p_purpose IS NULL OR p_purpose <> btrim(p_purpose)
       OR char_length(p_purpose) NOT BETWEEN 1 AND 1000 OR p_result NOT IN ('issued', 'revoked') THEN
        RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Support capability audit arguments are invalid';
    END IF;
    INSERT INTO ple_audit.sysadmin_support_capability_event VALUES (
        pg_catalog.gen_random_uuid(), p_capability_id, p_course_id, p_sysadmin_account_id,
        p_issuer_account_id, p_operation_kind, p_purpose, p_result, pg_catalog.transaction_timestamp()
    );
END $$;
REVOKE ALL PRIVILEGES ON TABLE ple_audit.sysadmin_support_capability_event FROM PUBLIC;
REVOKE ALL PRIVILEGES ON FUNCTION ple_audit.record_live_demo_support_capability_event(uuid, uuid, uuid, uuid, text, text, text) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_audit.record_live_demo_support_capability_event(uuid, uuid, uuid, uuid, text, text, text) TO ple_private_owner;
RESET ROLE;

SET LOCAL ROLE ple_private_owner;
CREATE FUNCTION ple_private.require_live_demo_current_course_instructor(p_course_id uuid)
RETURNS uuid LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
DECLARE v_actor uuid;
BEGIN
    v_actor := ple_api.current_session_account_id();
    IF v_actor IS NULL OR NOT EXISTS (
        SELECT 1 FROM ple_data.course_membership AS membership
        JOIN ple_private.account AS account ON account.account_id = membership.account_id
        JOIN LATERAL (SELECT event.state FROM ple_private.account_state_event AS event
            WHERE event.account_id = account.account_id ORDER BY event.occurred_at DESC, event.event_id DESC LIMIT 1) AS state ON state.state = 'active'
        WHERE membership.course_id = p_course_id AND membership.account_id = v_actor
          AND membership.role = 'instructor' AND ple_data.course_membership_is_active(membership.membership_id)
    ) THEN RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Current Course Instructor required'; END IF;
    RETURN v_actor;
END $$;

CREATE FUNCTION ple_private.live_demo_support_capability_receipt(p_capability_id uuid)
RETURNS TABLE (capability_id uuid, course_reference_number bigint, sysadmin_reference_number bigint, purpose text, expires_at_millis bigint, revoked_at_millis bigint)
LANGUAGE sql SECURITY DEFINER SET search_path = pg_catalog, ple_data, ple_private AS $$
    SELECT capability.capability_id, course.reference_number, sysadmin.reference_number, capability.purpose,
       (extract(epoch FROM capability.expires_at) * 1000)::bigint,
       (extract(epoch FROM capability.revoked_at) * 1000)::bigint
    FROM ple_private.sysadmin_support_capability AS capability
    JOIN ple_data.course_instance AS course ON course.course_id = capability.course_id
    JOIN ple_private.account AS sysadmin ON sysadmin.account_id = capability.sysadmin_account_id
    WHERE capability.capability_id = p_capability_id
$$;

CREATE FUNCTION ple_private.issue_live_demo_course_roster_support(p_course_reference_number bigint, p_sysadmin_reference_number bigint, p_purpose text, p_capability_id uuid)
RETURNS TABLE (capability_id uuid, course_reference_number bigint, sysadmin_reference_number bigint, purpose text, expires_at_millis bigint, revoked_at_millis bigint)
LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog, ple_api, ple_data, ple_private, ple_audit AS $$
DECLARE v_course uuid; v_issuer uuid; v_sysadmin uuid; v_now timestamptz;
BEGIN
    IF p_course_reference_number NOT BETWEEN 1 AND 2147483647 OR p_sysadmin_reference_number NOT BETWEEN 1 AND 2147483647
       OR p_capability_id IS NULL OR p_purpose IS NULL OR p_purpose <> btrim(p_purpose) OR char_length(p_purpose) NOT BETWEEN 1 AND 1000 THEN
       RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Support capability arguments are invalid'; END IF;
    SELECT course_id INTO v_course FROM ple_data.course_instance WHERE reference_number = p_course_reference_number;
    IF NOT FOUND THEN RETURN; END IF;
    v_issuer := ple_private.require_live_demo_current_course_instructor(v_course);
    SELECT account.account_id INTO v_sysadmin FROM ple_private.account AS account
      JOIN LATERAL (SELECT event.state FROM ple_private.account_state_event AS event WHERE event.account_id = account.account_id ORDER BY event.occurred_at DESC, event.event_id DESC LIMIT 1) AS state ON state.state = 'active'
      WHERE account.reference_number = p_sysadmin_reference_number AND account.product_role = 'sysadmin' FOR KEY SHARE OF account;
    IF NOT FOUND THEN RETURN; END IF;
    v_now := pg_catalog.transaction_timestamp();
    INSERT INTO ple_private.sysadmin_support_capability (capability_id, sysadmin_account_id, course_id, operation_kind, purpose, issuer_account_id, minimum_projection, issued_at, expires_at)
    VALUES (p_capability_id, v_sysadmin, v_course, 'course_roster_support', p_purpose, v_issuer, 'course_roster', v_now, v_now + interval '1 hour');
    PERFORM ple_audit.record_live_demo_support_capability_event(p_capability_id, v_course, v_sysadmin, v_issuer, 'course_roster_support', p_purpose, 'issued');
    RETURN QUERY SELECT * FROM ple_private.live_demo_support_capability_receipt(p_capability_id);
END $$;

CREATE FUNCTION ple_private.revoke_live_demo_course_roster_support(p_course_reference_number bigint, p_capability_id uuid)
RETURNS TABLE (capability_id uuid, course_reference_number bigint, sysadmin_reference_number bigint, purpose text, expires_at_millis bigint, revoked_at_millis bigint)
LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog, ple_api, ple_data, ple_private, ple_audit AS $$
DECLARE v_course uuid; v_issuer uuid; v_capability ple_private.sysadmin_support_capability%ROWTYPE;
BEGIN
    IF p_course_reference_number NOT BETWEEN 1 AND 2147483647 OR p_capability_id IS NULL THEN RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Support capability arguments are invalid'; END IF;
    SELECT course_id INTO v_course FROM ple_data.course_instance WHERE reference_number = p_course_reference_number;
    IF NOT FOUND THEN RETURN; END IF;
    v_issuer := ple_private.require_live_demo_current_course_instructor(v_course);
    SELECT * INTO v_capability
      FROM ple_private.sysadmin_support_capability AS capability
     WHERE capability.capability_id = p_capability_id AND capability.course_id = v_course
     FOR UPDATE;
    IF NOT FOUND OR v_capability.issuer_account_id <> v_issuer OR v_capability.operation_kind <> 'course_roster_support' OR v_capability.revoked_at IS NOT NULL THEN RETURN; END IF;
    UPDATE ple_private.sysadmin_support_capability AS capability
       SET revoked_at = pg_catalog.transaction_timestamp()
     WHERE capability.capability_id = p_capability_id;
    PERFORM ple_audit.record_live_demo_support_capability_event(p_capability_id, v_course, v_capability.sysadmin_account_id, v_issuer, 'course_roster_support', v_capability.purpose, 'revoked');
    RETURN QUERY SELECT * FROM ple_private.live_demo_support_capability_receipt(p_capability_id);
END $$;
REVOKE ALL PRIVILEGES ON FUNCTION ple_private.require_live_demo_current_course_instructor(uuid) FROM PUBLIC;
REVOKE ALL PRIVILEGES ON FUNCTION ple_private.live_demo_support_capability_receipt(uuid) FROM PUBLIC;
REVOKE ALL PRIVILEGES ON FUNCTION ple_private.issue_live_demo_course_roster_support(bigint, bigint, text, uuid) FROM PUBLIC;
REVOKE ALL PRIVILEGES ON FUNCTION ple_private.revoke_live_demo_course_roster_support(bigint, uuid) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_private.issue_live_demo_course_roster_support(bigint, bigint, text, uuid), ple_private.revoke_live_demo_course_roster_support(bigint, uuid) TO ple_api_owner;
RESET ROLE;

SET LOCAL ROLE ple_api_owner;
DROP FUNCTION ple_api.current_session_account_has_support_capability(uuid, uuid, uuid, text);
CREATE FUNCTION ple_api.issue_live_demo_course_roster_support(p_course_reference_number bigint, p_sysadmin_reference_number bigint, p_purpose text, p_capability_id uuid)
RETURNS TABLE (capability_id uuid, course_reference_number bigint, sysadmin_reference_number bigint, purpose text, expires_at_millis bigint, revoked_at_millis bigint)
LANGUAGE sql SECURITY DEFINER SET search_path = pg_catalog, ple_private AS $$ SELECT * FROM ple_private.issue_live_demo_course_roster_support(p_course_reference_number, p_sysadmin_reference_number, p_purpose, p_capability_id) $$;
CREATE FUNCTION ple_api.revoke_live_demo_course_roster_support(p_course_reference_number bigint, p_capability_id uuid)
RETURNS TABLE (capability_id uuid, course_reference_number bigint, sysadmin_reference_number bigint, purpose text, expires_at_millis bigint, revoked_at_millis bigint)
LANGUAGE sql SECURITY DEFINER SET search_path = pg_catalog, ple_private AS $$ SELECT * FROM ple_private.revoke_live_demo_course_roster_support(p_course_reference_number, p_capability_id) $$;
REVOKE ALL PRIVILEGES ON FUNCTION ple_api.issue_live_demo_course_roster_support(bigint, bigint, text, uuid), ple_api.revoke_live_demo_course_roster_support(bigint, uuid) FROM PUBLIC;
GRANT USAGE ON SCHEMA ple_api TO ple_app;
GRANT EXECUTE ON FUNCTION ple_api.issue_live_demo_course_roster_support(bigint, bigint, text, uuid), ple_api.revoke_live_demo_course_roster_support(bigint, uuid) TO ple_app;

CREATE FUNCTION ple_api.current_session_account_has_live_demo_course_roster_support(
    p_capability_id uuid, p_course_id uuid
) RETURNS boolean LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private AS $$
    SELECT p_capability_id IS NOT NULL AND EXISTS (
        SELECT 1 FROM ple_private.sysadmin_support_capability AS capability
        WHERE capability.capability_id = p_capability_id
          AND capability.course_id = p_course_id
          AND capability.operation_kind = 'course_roster_support'
          AND capability.minimum_projection = 'course_roster'
          AND capability.revoked_at IS NULL
          AND capability.expires_at > pg_catalog.clock_timestamp()
          AND capability.sysadmin_account_id = ple_api.current_session_account_id()
    )
$$;
REVOKE ALL PRIVILEGES ON FUNCTION ple_api.current_session_account_has_live_demo_course_roster_support(uuid, uuid) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_api.current_session_account_has_live_demo_course_roster_support(uuid, uuid) TO ple_app;

CREATE OR REPLACE FUNCTION ple_api.list_live_demo_course_roster(
    p_course_reference_number bigint, p_support_capability_id uuid DEFAULT NULL
) RETURNS TABLE (roster_id text, roster_email text, state text)
LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
    SELECT profile.roster_id, profile.roster_email,
           CASE WHEN EXISTS (SELECT 1 FROM ple_data.course_membership AS membership
                              WHERE membership.course_id = course.course_id AND membership.account_id = profile.student_account_id
                                AND membership.role = 'student' AND ple_data.course_membership_is_active(membership.membership_id))
                THEN 'active_student' ELSE 'invitation_pending' END
      FROM ple_data.course_instance AS course
      JOIN ple_private.course_roster_profile AS profile ON profile.course_id = course.course_id
     WHERE course.reference_number = p_course_reference_number
       AND (ple_api.current_session_account_is_course_instructor(course.course_id)
            OR ple_api.current_session_account_has_live_demo_course_roster_support(p_support_capability_id, course.course_id))
       AND (EXISTS (SELECT 1 FROM ple_data.course_membership AS membership WHERE membership.course_id = course.course_id
                     AND membership.account_id = profile.student_account_id AND membership.role = 'student'
                     AND ple_data.course_membership_is_active(membership.membership_id))
            OR EXISTS (SELECT 1 FROM ple_private.course_invitation AS invitation WHERE invitation.course_id = course.course_id
                       AND invitation.target_account_id = profile.student_account_id AND invitation.membership_role = 'student'
                       AND invitation.expires_at > pg_catalog.clock_timestamp()
                       AND NOT EXISTS (SELECT 1 FROM ple_private.course_invitation_event AS event WHERE event.invitation_id = invitation.invitation_id)))
     ORDER BY profile.roster_id
$$;
REVOKE ALL PRIVILEGES ON FUNCTION ple_api.list_live_demo_course_roster(bigint, uuid) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_api.list_live_demo_course_roster(bigint, uuid) TO ple_app;
CREATE FUNCTION ple_api.list_live_demo_support_course_roster(p_capability_id uuid)
RETURNS TABLE (roster_id text, roster_email text, state text)
LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
    SELECT * FROM ple_api.list_live_demo_course_roster(
        (SELECT course.reference_number FROM ple_private.sysadmin_support_capability AS capability
          JOIN ple_data.course_instance AS course ON course.course_id = capability.course_id
         WHERE capability.capability_id = p_capability_id),
        p_capability_id)
$$;
REVOKE ALL PRIVILEGES ON FUNCTION ple_api.list_live_demo_support_course_roster(uuid) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_api.list_live_demo_support_course_roster(uuid) TO ple_app;
RESET ROLE;
