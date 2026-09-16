-- Narrow, audited repair-capability registry.  A capability is a request for
-- one existing Course-local Student roster profile; it never confers Course
-- membership. Course/content repair authority remains future implementation.

SET LOCAL ROLE ple_private_owner;

CREATE TABLE ple_private.support_repair_capability (
    capability_id uuid PRIMARY KEY,
    sysadmin_account_id uuid NOT NULL REFERENCES ple_private.account (account_id),
    sysadmin_role text NOT NULL DEFAULT 'sysadmin' CHECK (sysadmin_role = 'sysadmin'),
    issuer_account_id uuid NOT NULL REFERENCES ple_private.account (account_id),
    resource_class text NOT NULL CHECK (resource_class = 'student'),
    resource_reference text NOT NULL CHECK (
        resource_reference = btrim(resource_reference)
        AND char_length(resource_reference) BETWEEN 1 AND 512
        AND resource_reference !~ '[[:cntrl:]]'
    ),
    purpose text NOT NULL CHECK (purpose = btrim(purpose) AND char_length(purpose) BETWEEN 1 AND 1000
        AND purpose !~ '[[:cntrl:]]'),
    issued_at timestamp with time zone NOT NULL,
    expires_at timestamp with time zone NOT NULL CHECK (expires_at > issued_at),
    revoked_at timestamp with time zone,
    FOREIGN KEY (sysadmin_account_id, sysadmin_role)
        REFERENCES ple_private.account (account_id, product_role),
    CHECK (revoked_at IS NULL OR revoked_at >= issued_at)
);

CREATE FUNCTION ple_private.reject_support_repair_capability_change()
RETURNS trigger LANGUAGE plpgsql SET search_path = pg_catalog, ple_private AS $$
BEGIN
    IF TG_OP = 'DELETE' THEN
        RAISE EXCEPTION USING ERRCODE = '55000',
            MESSAGE = 'Support repair capabilities are retained as audited evidence';
    END IF;
    IF NEW.capability_id IS DISTINCT FROM OLD.capability_id
       OR NEW.sysadmin_account_id IS DISTINCT FROM OLD.sysadmin_account_id
       OR NEW.issuer_account_id IS DISTINCT FROM OLD.issuer_account_id
       OR NEW.resource_class IS DISTINCT FROM OLD.resource_class
       OR NEW.resource_reference IS DISTINCT FROM OLD.resource_reference
       OR NEW.purpose IS DISTINCT FROM OLD.purpose
       OR NEW.issued_at IS DISTINCT FROM OLD.issued_at
       OR NEW.expires_at IS DISTINCT FROM OLD.expires_at
       OR (OLD.revoked_at IS NOT NULL AND NEW.revoked_at IS DISTINCT FROM OLD.revoked_at) THEN
        RAISE EXCEPTION USING ERRCODE = '55000',
            MESSAGE = 'Support repair capability identity is immutable';
    END IF;
    RETURN NEW;
END
$$;

CREATE TRIGGER support_repair_capability_is_guarded
BEFORE UPDATE OR DELETE ON ple_private.support_repair_capability
FOR EACH ROW EXECUTE FUNCTION ple_private.reject_support_repair_capability_change();

ALTER TABLE ple_private.support_repair_capability ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.support_repair_capability FORCE ROW LEVEL SECURITY;
REVOKE ALL ON TABLE ple_private.support_repair_capability FROM PUBLIC;
GRANT SELECT, INSERT, UPDATE (revoked_at) ON ple_private.support_repair_capability TO ple_api_owner;
GRANT USAGE ON SCHEMA ple_private TO ple_audit_owner;
GRANT REFERENCES ON TABLE ple_private.support_repair_capability TO ple_audit_owner;
CREATE POLICY support_repair_capability_private_owner_access
    ON ple_private.support_repair_capability FOR ALL TO ple_private_owner
    USING (true) WITH CHECK (true);
CREATE POLICY support_repair_capability_api_owner_access
    ON ple_private.support_repair_capability FOR ALL TO ple_api_owner
    USING (true) WITH CHECK (true);
REVOKE ALL ON FUNCTION ple_private.reject_support_repair_capability_change() FROM PUBLIC;

RESET ROLE;

SET LOCAL ROLE ple_audit_owner;

CREATE TABLE ple_audit.support_repair_capability_event (
    event_id uuid PRIMARY KEY,
    capability_id uuid NOT NULL REFERENCES ple_private.support_repair_capability (capability_id),
    sysadmin_account_id uuid NOT NULL REFERENCES ple_private.account (account_id),
    issuer_account_id uuid NOT NULL REFERENCES ple_private.account (account_id),
    resource_class text NOT NULL CHECK (resource_class = 'student'),
    resource_reference text NOT NULL CHECK (resource_reference = btrim(resource_reference)
        AND char_length(resource_reference) BETWEEN 1 AND 512 AND resource_reference !~ '[[:cntrl:]]'),
    purpose text NOT NULL CHECK (purpose = btrim(purpose) AND char_length(purpose) BETWEEN 1 AND 1000
        AND purpose !~ '[[:cntrl:]]'),
    result text NOT NULL CHECK (result IN ('issued', 'revoked', 'used')),
    occurred_at timestamp with time zone NOT NULL
);

CREATE FUNCTION ple_audit.reject_support_repair_capability_event_change()
RETURNS trigger LANGUAGE plpgsql SET search_path = pg_catalog, ple_audit AS $$
BEGIN
    RAISE EXCEPTION USING ERRCODE = '55000',
        MESSAGE = 'Support repair capability audit events are immutable';
END
$$;

CREATE TRIGGER support_repair_capability_event_is_immutable
BEFORE UPDATE OR DELETE ON ple_audit.support_repair_capability_event
FOR EACH ROW EXECUTE FUNCTION ple_audit.reject_support_repair_capability_event_change();

ALTER TABLE ple_audit.support_repair_capability_event ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_audit.support_repair_capability_event FORCE ROW LEVEL SECURITY;
REVOKE ALL ON TABLE ple_audit.support_repair_capability_event FROM PUBLIC;
CREATE POLICY support_repair_capability_event_owner_write
    ON ple_audit.support_repair_capability_event FOR INSERT TO ple_audit_owner WITH CHECK (true);

CREATE FUNCTION ple_audit.record_support_repair_capability_event(
    p_capability_id uuid, p_sysadmin_account_id uuid, p_issuer_account_id uuid,
    p_resource_class text, p_resource_reference text, p_purpose text, p_result text
)
RETURNS uuid LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog, ple_audit AS $$
DECLARE recorded_event_id uuid;
BEGIN
    -- ASVS 2.2.1: this trusted boundary allowlists the resource class and
    -- bounds opaque input before it becomes durable audit evidence.
    IF p_capability_id IS NULL OR p_sysadmin_account_id IS NULL OR p_issuer_account_id IS NULL
       OR p_resource_class IS NULL OR p_resource_class <> 'student'
       OR p_resource_reference IS NULL OR p_resource_reference <> btrim(p_resource_reference)
       OR char_length(p_resource_reference) NOT BETWEEN 1 AND 512
       OR p_resource_reference ~ '[[:cntrl:]]'
       OR p_purpose IS NULL OR p_purpose <> btrim(p_purpose)
       OR char_length(p_purpose) NOT BETWEEN 1 AND 1000 OR p_purpose ~ '[[:cntrl:]]'
       OR p_result IS NULL OR p_result NOT IN ('issued', 'revoked', 'used') THEN
        RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Support repair capability audit arguments are invalid';
    END IF;
    recorded_event_id := pg_catalog.gen_random_uuid();
    INSERT INTO ple_audit.support_repair_capability_event
    VALUES (recorded_event_id, p_capability_id, p_sysadmin_account_id, p_issuer_account_id,
            p_resource_class, p_resource_reference, p_purpose, p_result,
            pg_catalog.transaction_timestamp());
    RETURN recorded_event_id;
END
$$;

REVOKE ALL ON FUNCTION ple_audit.reject_support_repair_capability_event_change(),
    ple_audit.record_support_repair_capability_event(uuid, uuid, uuid, text, text, text, text)
    FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_audit.record_support_repair_capability_event(
    uuid, uuid, uuid, text, text, text, text
) TO ple_api_owner;

RESET ROLE;

SET LOCAL ROLE ple_api_owner;

-- Resolve only the implemented roster scope, using current public identities
-- and exact reconstruction rather than an obsolete identifier grammar.
CREATE FUNCTION ple_api.support_repair_roster_course(p_resource_reference text)
RETURNS uuid LANGUAGE sql STABLE
SET search_path = pg_catalog, ple_data, ple_private AS $$
    SELECT course.course_id
      FROM ple_data.course_instance AS course
      JOIN ple_private.course_roster_profile AS profile ON profile.course_id = course.course_id
     WHERE course.public_reference = split_part(p_resource_reference, '/', 2)
       AND profile.roster_id = split_part(p_resource_reference, '/', 4)
       AND p_resource_reference = 'course-instance/' || course.public_reference || '/roster/' || profile.roster_id
$$;
REVOKE ALL ON FUNCTION ple_api.support_repair_roster_course(text) FROM PUBLIC;

-- ASVS 8.2.1 and 8.3.1: this is an explicit, time-bounded support request,
-- not a membership grant. Exact issuer Course authority and canonical roster
-- scope are required here and rechecked beside the specific repair operation.
CREATE FUNCTION ple_api.issue_support_repair_capability(
    p_sysadmin_public_reference text, p_resource_class text,
    p_resource_reference text, p_purpose text, p_capability_id uuid
)
RETURNS TABLE(capability_id uuid, sysadmin_public_reference text, resource_class text,
              resource_reference text, purpose text, expires_at_millis bigint, revoked_at_millis bigint)
LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog, ple_api, ple_audit, ple_private AS $$
DECLARE issuer uuid; sysadmin uuid; now_at timestamptz; repair_course uuid;
BEGIN
    IF p_sysadmin_public_reference IS NULL OR p_capability_id IS NULL
       OR p_resource_class IS NULL OR p_resource_class <> 'student'
       OR p_resource_reference IS NULL OR p_resource_reference <> btrim(p_resource_reference)
       OR char_length(p_resource_reference) NOT BETWEEN 1 AND 512 OR p_resource_reference ~ '[[:cntrl:]]'
       OR p_purpose IS NULL OR p_purpose <> btrim(p_purpose)
       OR char_length(p_purpose) NOT BETWEEN 1 AND 1000 OR p_purpose ~ '[[:cntrl:]]' THEN
        RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Support repair capability input is invalid';
    END IF;
    IF NOT ple_api.current_session_account_is_instructor() THEN RETURN; END IF;
    issuer := ple_api.current_session_account_id();
    repair_course := ple_api.support_repair_roster_course(p_resource_reference);
    -- ASVS 8.2.2/8.4.1: global Instructor status is not Course authority.
    IF repair_course IS NULL OR NOT ple_api.current_session_account_is_course_instructor(repair_course) THEN RETURN; END IF;
    -- ASVS 8.2.1: resolve the immutable Account identity with SELECT only.
    -- The capability's Account foreign keys protect the recipient identity
    -- during insertion; an explicit KEY SHARE would duplicate that protection
    -- and require Account UPDATE authority. Account State is separate event
    -- evidence, not an Account key protected by that lock.
    SELECT account.account_id INTO sysadmin FROM ple_private.account AS account
    JOIN LATERAL (
        SELECT event.state FROM ple_private.account_state_event AS event
         WHERE event.account_id = account.account_id
         ORDER BY event.occurred_at DESC, event.event_id DESC LIMIT 1
    ) AS state_event ON state_event.state = 'active'
    WHERE account.public_reference = p_sysadmin_public_reference AND account.product_role = 'sysadmin';
    IF NOT FOUND THEN RETURN; END IF;
    now_at := pg_catalog.transaction_timestamp();
    INSERT INTO ple_private.support_repair_capability (
        capability_id, sysadmin_account_id, issuer_account_id, resource_class,
        resource_reference, purpose, issued_at, expires_at
    ) VALUES (p_capability_id, sysadmin, issuer, p_resource_class, p_resource_reference,
              p_purpose, now_at, now_at + interval '1 hour');
    PERFORM ple_audit.record_support_repair_capability_event(
        p_capability_id, sysadmin, issuer, p_resource_class, p_resource_reference, p_purpose, 'issued'
    );
    RETURN QUERY SELECT capability.capability_id, account.public_reference,
        capability.resource_class, capability.resource_reference, capability.purpose,
        (extract(epoch FROM capability.expires_at) * 1000)::bigint,
        (extract(epoch FROM capability.revoked_at) * 1000)::bigint
      FROM ple_private.support_repair_capability AS capability
      JOIN ple_private.account AS account ON account.account_id = capability.sysadmin_account_id
     WHERE capability.capability_id = p_capability_id;
END
$$;

CREATE FUNCTION ple_api.revoke_support_repair_capability(p_capability_id uuid)
RETURNS TABLE(capability_id uuid, sysadmin_public_reference text, resource_class text,
              resource_reference text, purpose text, expires_at_millis bigint, revoked_at_millis bigint)
LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog, ple_api, ple_audit, ple_private AS $$
DECLARE issuer uuid; capability ple_private.support_repair_capability%ROWTYPE;
BEGIN
    IF p_capability_id IS NULL OR NOT ple_api.current_session_account_is_instructor() THEN RETURN; END IF;
    issuer := ple_api.current_session_account_id();
    SELECT stored.* INTO capability FROM ple_private.support_repair_capability AS stored
     WHERE stored.capability_id = p_capability_id FOR UPDATE OF stored;
    IF NOT FOUND OR capability.issuer_account_id <> issuer OR capability.revoked_at IS NOT NULL THEN RETURN; END IF;
    UPDATE ple_private.support_repair_capability AS stored SET revoked_at = pg_catalog.transaction_timestamp()
     WHERE stored.capability_id = p_capability_id;
    PERFORM ple_audit.record_support_repair_capability_event(
        capability.capability_id, capability.sysadmin_account_id, issuer, capability.resource_class,
        capability.resource_reference, capability.purpose, 'revoked'
    );
    RETURN QUERY SELECT stored.capability_id, account.public_reference, stored.resource_class,
        stored.resource_reference, stored.purpose,
        (extract(epoch FROM stored.expires_at) * 1000)::bigint,
        (extract(epoch FROM stored.revoked_at) * 1000)::bigint
      FROM ple_private.support_repair_capability AS stored
      JOIN ple_private.account AS account ON account.account_id = stored.sysadmin_account_id
     WHERE stored.capability_id = p_capability_id;
END
$$;

CREATE FUNCTION ple_api.record_support_repair_capability_use(
    p_capability_id uuid, p_resource_class text, p_resource_reference text
)
RETURNS TABLE(audit_event_id uuid, capability_id uuid, resource_class text,
              resource_reference text, used_at_millis bigint)
LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog, ple_api, ple_audit, ple_private AS $$
DECLARE capability ple_private.support_repair_capability%ROWTYPE; event_id uuid; now_at timestamptz; repair_course uuid;
BEGIN
    IF p_capability_id IS NULL OR p_resource_class IS NULL
       OR p_resource_class <> 'student'
       OR p_resource_reference IS NULL OR p_resource_reference <> btrim(p_resource_reference)
       OR char_length(p_resource_reference) NOT BETWEEN 1 AND 512 OR p_resource_reference ~ '[[:cntrl:]]'
       OR NOT ple_api.current_session_account_has_platform_administration() THEN RETURN; END IF;
    -- FOR UPDATE keeps revocation and the C26 repair transaction ordered.
    SELECT stored.* INTO capability FROM ple_private.support_repair_capability AS stored
     WHERE stored.capability_id = p_capability_id AND stored.resource_class = p_resource_class
       AND stored.resource_reference = p_resource_reference AND stored.revoked_at IS NULL
       AND stored.expires_at > pg_catalog.clock_timestamp()
       AND stored.sysadmin_account_id = ple_api.current_session_account_id()
     FOR UPDATE OF stored;
    IF NOT FOUND THEN RETURN; END IF;
    repair_course := ple_api.support_repair_roster_course(capability.resource_reference);
    -- ASVS 8.3.2/8.3.3: recheck the ORIGINAL issuer, not the recipient
    -- Sysadmin or another current co-Instructor, before recording use.
    IF repair_course IS NULL OR NOT EXISTS (
        SELECT 1 FROM ple_private.account AS issuer
        JOIN LATERAL (
            SELECT event.state FROM ple_private.account_state_event AS event
             WHERE event.account_id = issuer.account_id
             ORDER BY event.occurred_at DESC, event.event_id DESC LIMIT 1
        ) AS current_state ON current_state.state = 'active'
        JOIN ple_data.course_membership AS membership ON membership.account_id = issuer.account_id
         AND membership.course_id = repair_course AND membership.role = 'instructor'
         AND ple_data.course_membership_is_active(membership.membership_id)
        WHERE issuer.account_id = capability.issuer_account_id AND issuer.product_role = 'instructor'
    ) THEN RETURN; END IF;
    event_id := ple_audit.record_support_repair_capability_event(
        capability.capability_id, capability.sysadmin_account_id, capability.issuer_account_id,
        capability.resource_class, capability.resource_reference, capability.purpose, 'used'
    );
    now_at := pg_catalog.transaction_timestamp();
    RETURN QUERY SELECT event_id, capability.capability_id, capability.resource_class,
        capability.resource_reference, (extract(epoch FROM now_at) * 1000)::bigint;
END
$$;

REVOKE ALL ON FUNCTION ple_api.issue_support_repair_capability(text, text, text, text, uuid),
    ple_api.revoke_support_repair_capability(uuid),
    ple_api.record_support_repair_capability_use(uuid, text, text) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_api.issue_support_repair_capability(text, text, text, text, uuid),
    ple_api.revoke_support_repair_capability(uuid),
    ple_api.record_support_repair_capability_use(uuid, text, text) TO ple_app;

RESET ROLE;
