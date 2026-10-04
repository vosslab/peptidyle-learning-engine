-- Functions, triggers, and views from support_repair_capability.sql.

SET LOCAL ROLE ple_private_owner;

CREATE FUNCTION ple_private.reject_support_repair_capability_change()
RETURNS trigger LANGUAGE plpgsql SET search_path = pg_catalog, ple_private AS $$
BEGIN
    IF TG_OP = 'DELETE' THEN
        RAISE EXCEPTION USING ERRCODE = '55000',
            MESSAGE = 'Support repair capabilities are retained as audited evidence';
    END IF;
    IF NEW.support_repair_capability_id IS DISTINCT FROM OLD.support_repair_capability_id
       OR NEW.sysadmin_account_id IS DISTINCT FROM OLD.sysadmin_account_id
       OR NEW.issuer_account_id IS DISTINCT FROM OLD.issuer_account_id
       OR NEW.resource_class IS DISTINCT FROM OLD.resource_class
       OR NEW.resource_path IS DISTINCT FROM OLD.resource_path
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

SET LOCAL ROLE ple_audit_owner;

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

CREATE FUNCTION ple_audit.record_support_repair_capability_event(
    p_capability_id uuid, p_sysadmin_account_id text, p_issuer_account_id text,
    p_resource_class text, p_resource_path text, p_purpose text, p_result text
)
RETURNS uuid LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog, ple_audit AS $$
DECLARE recorded_event_id uuid;
BEGIN
    -- ASVS 2.2.1: this trusted boundary allowlists the resource class and
    -- bounds opaque input before it becomes durable audit evidence.
    IF p_capability_id IS NULL OR p_sysadmin_account_id IS NULL OR p_issuer_account_id IS NULL
       OR p_resource_class IS NULL OR p_resource_class NOT IN ('student', 'course', 'content')
       OR p_resource_path IS NULL OR p_resource_path <> btrim(p_resource_path)
       OR char_length(p_resource_path) NOT BETWEEN 1 AND 512
       OR p_resource_path ~ '[[:cntrl:]]'
       OR p_purpose IS NULL OR p_purpose <> btrim(p_purpose)
       OR char_length(p_purpose) NOT BETWEEN 1 AND 1000 OR p_purpose ~ '[[:cntrl:]]'
       OR p_result IS NULL OR p_result NOT IN ('issued', 'revoked', 'used') THEN
        RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Support repair capability audit arguments are invalid';
    END IF;
    recorded_event_id := pg_catalog.gen_random_uuid();
    INSERT INTO ple_audit.support_repair_capability_event
    VALUES (recorded_event_id, p_capability_id, p_sysadmin_account_id, p_issuer_account_id,
            p_resource_class::ple_data.support_repair_resource_class, p_resource_path, p_purpose,
            p_result::ple_data.repair_result,
            pg_catalog.transaction_timestamp());
    RETURN recorded_event_id;
END
$$;

SET LOCAL ROLE ple_api_owner;







-- Resolve only the implemented roster scope, using current public identities
-- and exact reconstruction rather than an obsolete identifier grammar.
CREATE FUNCTION ple_api.support_repair_roster_course(p_resource_path text)
RETURNS text LANGUAGE sql STABLE
SET search_path = pg_catalog, ple_data, ple_private AS $$
    SELECT course.course_instance_id
      FROM ple_data.course_instance AS course
      JOIN ple_private.course_roster_profile AS profile ON profile.course_instance_id = course.course_instance_id
     WHERE course.course_instance_id = split_part(p_resource_path, '/', 2)
       AND profile.roster_id = split_part(p_resource_path, '/', 4)
       AND p_resource_path = 'course-instance/' || course.course_instance_id || '/roster/' || profile.roster_id
$$;

-- Resolve only an existing Course Instance. The path is that Course's canonical
-- public identity, reconstructed exactly, with no roster or Student segment.
CREATE FUNCTION ple_api.support_repair_course(p_resource_path text)
RETURNS text LANGUAGE sql STABLE
SET search_path = pg_catalog, ple_data, ple_private AS $$
    SELECT course.course_instance_id
      FROM ple_data.course_instance AS course
     WHERE p_resource_path = 'course-instance/' || course.course_instance_id
$$;

-- ASVS 8.2.2: Assessment row security admits the Course Instructor, not the
-- support recipient. These two helpers run as the data owner so issuance and
-- use can resolve one exact Assessment without making the Sysadmin a member.
SET LOCAL ROLE ple_data_owner;

CREATE FUNCTION ple_api.support_repair_content_course(p_resource_path text)
RETURNS text LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_data, ple_private AS $$
    SELECT course.course_instance_id
      FROM ple_data.course_instance AS course
      JOIN ple_data.assessment AS assessment
        ON assessment.course_instance_id = course.course_instance_id
     WHERE p_resource_path = 'course-instance/' || course.course_instance_id
           || '/assessment/' || assessment.assessment_id
$$;

CREATE FUNCTION ple_api.support_repair_content_facts(
    p_course_instance_id text, p_assessment_id text
)
RETURNS TABLE(assessment_id text, assessment_type text, title text, status text)
LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_data AS $$
    SELECT assessment.assessment_id::text,
           assessment.assessment_type::text,
           snapshot.assessment_title,
           assessment.assessment_status::text
      FROM ple_data.assessment AS assessment
      JOIN ple_data.assessment_policy_snapshot AS snapshot
        ON snapshot.assessment_policy_snapshot_id = assessment.assessment_policy_snapshot_id
     WHERE assessment.course_instance_id = p_course_instance_id
       AND assessment.assessment_id = p_assessment_id
       AND snapshot.assessment_title = btrim(snapshot.assessment_title)
       AND char_length(snapshot.assessment_title) BETWEEN 1 AND 200
       AND snapshot.assessment_title !~ '[[:cntrl:]]'
$$;

SET LOCAL ROLE ple_api_owner;



-- ASVS 8.2.1 and 8.3.1: this is an explicit, time-bounded support request,
-- not a membership grant. Exact issuer Course authority and canonical roster
-- scope are required here and rechecked beside the specific repair operation.
CREATE FUNCTION ple_api.issue_support_repair_capability(
    p_sysadmin_account_id text, p_resource_class text,
    p_resource_path text, p_purpose text, p_capability_id uuid
)
RETURNS TABLE(support_repair_capability_id uuid, sysadmin_account_id text, resource_class text,
              resource_path text, purpose text, expires_at_millis bigint, revoked_at_millis bigint)
LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog, ple_api, ple_audit, ple_private AS $$
DECLARE issuer text; sysadmin text; now_at timestamptz; repair_course text;
BEGIN
    IF p_sysadmin_account_id IS NULL OR p_capability_id IS NULL
       OR p_resource_class IS NULL OR p_resource_class NOT IN ('student', 'course', 'content')
       OR p_resource_path IS NULL OR p_resource_path <> btrim(p_resource_path)
       OR char_length(p_resource_path) NOT BETWEEN 1 AND 512 OR p_resource_path ~ '[[:cntrl:]]'
       OR p_purpose IS NULL OR p_purpose <> btrim(p_purpose)
       OR char_length(p_purpose) NOT BETWEEN 1 AND 1000 OR p_purpose ~ '[[:cntrl:]]' THEN
        RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Support repair capability input is invalid';
    END IF;
    IF NOT ple_api.current_session_account_is_instructor() THEN RETURN; END IF;
    issuer := ple_api.current_session_account_id();
    IF p_resource_class = 'student' THEN
        repair_course := ple_api.support_repair_roster_course(p_resource_path);
    ELSIF p_resource_class = 'course' THEN
        repair_course := ple_api.support_repair_course(p_resource_path);
    ELSIF p_resource_class = 'content' THEN
        repair_course := ple_api.support_repair_content_course(p_resource_path);
    ELSE
        RETURN;
    END IF;
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
    WHERE account.account_id = p_sysadmin_account_id AND account.user_role = 'sysadmin';
    IF NOT FOUND THEN RETURN; END IF;
    now_at := pg_catalog.transaction_timestamp();
    INSERT INTO ple_private.support_repair_capability (
        support_repair_capability_id, sysadmin_account_id, issuer_account_id, resource_class,
        resource_path, purpose, issued_at, expires_at
    ) VALUES (p_capability_id, sysadmin, issuer,
              p_resource_class::ple_data.support_repair_resource_class, p_resource_path,
              p_purpose, now_at, now_at + interval '1 hour');
    PERFORM ple_audit.record_support_repair_capability_event(
        p_capability_id, sysadmin, issuer, p_resource_class, p_resource_path, p_purpose, 'issued'
    );
    RETURN QUERY SELECT capability.support_repair_capability_id, account.account_id::text,
        capability.resource_class::text, capability.resource_path, capability.purpose,
        (extract(epoch FROM capability.expires_at) * 1000)::bigint,
        (extract(epoch FROM capability.revoked_at) * 1000)::bigint
      FROM ple_private.support_repair_capability AS capability
      JOIN ple_private.account AS account ON account.account_id = capability.sysadmin_account_id
     WHERE capability.support_repair_capability_id = p_capability_id;
END
$$;

CREATE FUNCTION ple_api.revoke_support_repair_capability(p_capability_id uuid)
RETURNS TABLE(support_repair_capability_id uuid, sysadmin_account_id text, resource_class text,
              resource_path text, purpose text, expires_at_millis bigint, revoked_at_millis bigint)
LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog, ple_api, ple_audit, ple_private AS $$
DECLARE issuer text; capability ple_private.support_repair_capability%ROWTYPE;
BEGIN
    IF p_capability_id IS NULL OR NOT ple_api.current_session_account_is_instructor() THEN RETURN; END IF;
    issuer := ple_api.current_session_account_id();
    SELECT stored.* INTO capability FROM ple_private.support_repair_capability AS stored
     WHERE stored.support_repair_capability_id = p_capability_id FOR UPDATE OF stored;
    IF NOT FOUND OR capability.issuer_account_id <> issuer OR capability.revoked_at IS NOT NULL THEN RETURN; END IF;
    UPDATE ple_private.support_repair_capability AS stored SET revoked_at = pg_catalog.transaction_timestamp()
     WHERE stored.support_repair_capability_id = p_capability_id;
    PERFORM ple_audit.record_support_repair_capability_event(
        capability.support_repair_capability_id, capability.sysadmin_account_id, issuer,
        capability.resource_class::text,
        capability.resource_path, capability.purpose, 'revoked'
    );
    RETURN QUERY SELECT stored.support_repair_capability_id, account.account_id::text,
        stored.resource_class::text,
        stored.resource_path, stored.purpose,
        (extract(epoch FROM stored.expires_at) * 1000)::bigint,
        (extract(epoch FROM stored.revoked_at) * 1000)::bigint
      FROM ple_private.support_repair_capability AS stored
      JOIN ple_private.account AS account ON account.account_id = stored.sysadmin_account_id
     WHERE stored.support_repair_capability_id = p_capability_id;
END
$$;

CREATE FUNCTION ple_api.record_support_repair_capability_use(
    p_capability_id uuid, p_resource_class text, p_resource_path text
)
RETURNS TABLE(audit_event_id uuid, support_repair_capability_id uuid, resource_class text,
              resource_path text, used_at_millis bigint)
LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog, ple_api, ple_audit, ple_private AS $$
DECLARE capability ple_private.support_repair_capability%ROWTYPE; event_id uuid; now_at timestamptz; repair_course text;
BEGIN
    IF p_capability_id IS NULL OR p_resource_class IS NULL
       OR p_resource_class NOT IN ('student', 'course', 'content')
       OR p_resource_path IS NULL OR p_resource_path <> btrim(p_resource_path)
       OR char_length(p_resource_path) NOT BETWEEN 1 AND 512 OR p_resource_path ~ '[[:cntrl:]]'
       OR NOT ple_api.current_session_account_has_platform_administration() THEN RETURN; END IF;
    -- FOR UPDATE keeps revocation and the C26 repair transaction ordered.
    SELECT stored.* INTO capability FROM ple_private.support_repair_capability AS stored
     WHERE stored.support_repair_capability_id = p_capability_id
       AND stored.resource_class = p_resource_class::ple_data.support_repair_resource_class
       AND stored.resource_path = p_resource_path AND stored.revoked_at IS NULL
       AND stored.expires_at > pg_catalog.clock_timestamp()
       AND stored.sysadmin_account_id = ple_api.current_session_account_id()
     FOR UPDATE OF stored;
    IF NOT FOUND THEN RETURN; END IF;
    IF capability.resource_class = 'student' THEN
        repair_course := ple_api.support_repair_roster_course(capability.resource_path);
    ELSIF capability.resource_class = 'course' THEN
        repair_course := ple_api.support_repair_course(capability.resource_path);
    ELSIF capability.resource_class = 'content' THEN
        repair_course := ple_api.support_repair_content_course(capability.resource_path);
    ELSE
        RETURN;
    END IF;
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
         AND membership.course_instance_id = repair_course AND membership.role = 'instructor'
         AND ple_data.course_membership_is_active(membership.course_membership_id)
        WHERE issuer.account_id = capability.issuer_account_id AND issuer.user_role = 'instructor'
    ) THEN RETURN; END IF;
    event_id := ple_audit.record_support_repair_capability_event(
        capability.support_repair_capability_id, capability.sysadmin_account_id, capability.issuer_account_id,
        capability.resource_class::text, capability.resource_path, capability.purpose, 'used'
    );
    now_at := pg_catalog.transaction_timestamp();
    RETURN QUERY SELECT event_id, capability.support_repair_capability_id, capability.resource_class::text,
        capability.resource_path, (extract(epoch FROM now_at) * 1000)::bigint;
END
$$;

-- ASVS 8.2.1/8.2.2/8.2.3/8.3.1/14.2.6/16.5.3: one exact Course after the
-- capability is consumed. The projection is Course identity, term, activity,
-- retention state, and active Instructor display names. It returns no Student
-- record and does not create Course membership. Missing authority returns no row.
CREATE FUNCTION ple_api.read_course_repair_support(
    p_capability_id uuid, p_course_instance_id text
)
RETURNS TABLE(
    course_instance_id text,
    short_name text,
    long_name text,
    term_starts_on date,
    term_ends_on date,
    course_lifecycle_state text,
    retention_lifecycle_state text,
    instructor_display_names text[]
)
LANGUAGE plpgsql VOLATILE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_audit, ple_data, ple_private AS $$
DECLARE canonical_resource_path text;
BEGIN
    IF p_capability_id IS NULL
       OR p_course_instance_id IS NULL
       OR NOT ple_private.is_canonical_prefixed_public_id(p_course_instance_id, 'CI')
       OR NOT ple_api.current_session_account_has_platform_administration() THEN
        RETURN;
    END IF;
    canonical_resource_path := 'course-instance/' || p_course_instance_id;
    PERFORM ple_api.record_support_repair_capability_use(
        p_capability_id, 'course', canonical_resource_path
    );
    IF NOT FOUND THEN
        RETURN;
    END IF;
    RETURN QUERY
    SELECT course.course_instance_id::text,
           course.course_short_name::text,
           course.course_long_name::text,
           course.term_starts_on,
           course.term_ends_on,
           course.course_lifecycle_state::text,
           course.retention_lifecycle_state::text,
           COALESCE((
               SELECT array_agg(named.display_name ORDER BY named.display_name)
                 FROM (
                     SELECT DISTINCT ple_private.verified_instructor_display_name(
                                membership.account_id::text
                            ) AS display_name
                       FROM ple_data.course_membership AS membership
                      WHERE membership.course_instance_id = course.course_instance_id
                        AND membership.role = 'instructor'
                        AND ple_data.course_membership_is_active(membership.course_membership_id)
                 ) AS named
                WHERE named.display_name IS NOT NULL
                  AND named.display_name = btrim(named.display_name)
                  AND char_length(named.display_name) BETWEEN 1 AND 200
                  AND named.display_name !~ '[[:cntrl:]]'
           ), ARRAY[]::text[])
      FROM ple_data.course_instance AS course
     WHERE course.course_instance_id = p_course_instance_id;
END
$$;

-- ASVS 8.2.1/8.2.2/8.2.3/8.3.1/14.2.6/16.5.3: one Assessment in the named
-- Course after the capability is consumed. The projection is the Assessment
-- identity, type, title, and status. It returns no instructions, Questions,
-- answers, or Student Work, and it does not create Course membership.
CREATE FUNCTION ple_api.read_course_content_repair_support(
    p_capability_id uuid, p_course_instance_id text, p_assessment_id text
)
RETURNS TABLE(
    assessment_id text,
    assessment_type text,
    title text,
    status text
)
LANGUAGE plpgsql VOLATILE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_audit, ple_data, ple_private AS $$
DECLARE canonical_resource_path text;
BEGIN
    IF p_capability_id IS NULL
       OR p_course_instance_id IS NULL
       OR p_assessment_id IS NULL
       OR NOT ple_private.is_canonical_prefixed_public_id(p_course_instance_id, 'CI')
       OR NOT ple_private.is_canonical_prefixed_public_id(p_assessment_id, 'A')
       OR NOT ple_api.current_session_account_has_platform_administration() THEN
        RETURN;
    END IF;
    canonical_resource_path := 'course-instance/' || p_course_instance_id
        || '/assessment/' || p_assessment_id;
    PERFORM ple_api.record_support_repair_capability_use(
        p_capability_id, 'content', canonical_resource_path
    );
    IF NOT FOUND THEN
        RETURN;
    END IF;
    RETURN QUERY
    SELECT facts.assessment_id, facts.assessment_type, facts.title, facts.status
      FROM ple_api.support_repair_content_facts(p_course_instance_id, p_assessment_id) AS facts;
END
$$;
