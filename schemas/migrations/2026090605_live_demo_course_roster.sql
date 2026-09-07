-- M9 Course Roster Import, invitation claim, Student Record, and revocation.
--
-- Import is the sole Student Account resolve-or-create boundary. It records a
-- pending Course Invitation and course-scoped roster metadata; claim creates
-- the Student Record and active Student Course Membership separately. Neither
-- path creates Assignment or Student-work state.

SET LOCAL ROLE ple_private_owner;

GRANT USAGE ON SCHEMA ple_data TO ple_private_owner;
GRANT REFERENCES ON TABLE ple_data.course_instance TO ple_private_owner;
GRANT USAGE ON SCHEMA ple_private TO ple_audit_owner;
GRANT REFERENCES ON TABLE ple_private.account TO ple_audit_owner;

CREATE TABLE ple_private.course_roster_profile (
    course_roster_profile_id uuid PRIMARY KEY,
    course_id uuid NOT NULL REFERENCES ple_data.course_instance (course_id),
    student_account_id uuid NOT NULL REFERENCES ple_private.account (account_id),
    roster_email text NOT NULL CHECK (
        char_length(roster_email) BETWEEN 3 AND 320
        AND roster_email = lower(btrim(roster_email))
    ),
    roster_id text NOT NULL CHECK (
        char_length(roster_id) BETWEEN 1 AND 64
        AND roster_id ~ '^[A-Za-z0-9._-]+$'
    ),
    created_at timestamp with time zone NOT NULL,
    CONSTRAINT course_roster_profile_course_student_unique UNIQUE (course_id, student_account_id),
    CONSTRAINT course_roster_profile_course_roster_id_unique UNIQUE (course_id, roster_id)
);

CREATE FUNCTION ple_private.reject_course_roster_profile_change()
RETURNS trigger LANGUAGE plpgsql
SET search_path = pg_catalog, ple_private AS $$
BEGIN
    RAISE EXCEPTION USING ERRCODE = '55000',
        MESSAGE = 'a Course Roster Profile is immutable';
END
$$;
CREATE TRIGGER course_roster_profile_is_immutable
BEFORE UPDATE OR DELETE ON ple_private.course_roster_profile
FOR EACH ROW EXECUTE FUNCTION ple_private.reject_course_roster_profile_change();

ALTER TABLE ple_private.course_roster_profile ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.course_roster_profile FORCE ROW LEVEL SECURITY;
REVOKE ALL PRIVILEGES ON TABLE ple_private.course_roster_profile FROM PUBLIC;
REVOKE ALL PRIVILEGES ON FUNCTION ple_private.reject_course_roster_profile_change() FROM PUBLIC;

CREATE POLICY account_private_owner_live_demo_student_create
    ON ple_private.account FOR INSERT TO ple_private_owner
    WITH CHECK (product_role = 'student');
CREATE POLICY account_authentication_email_private_owner_live_demo_student_lookup
    ON ple_private.account_authentication_email FOR SELECT TO ple_private_owner USING (true);
CREATE POLICY account_authentication_email_private_owner_live_demo_student_create
    ON ple_private.account_authentication_email FOR INSERT TO ple_private_owner
    WITH CHECK (
        EXISTS (
            SELECT 1
              FROM ple_private.account AS account
             WHERE account.account_id = account_authentication_email.account_id
               AND account.product_role = 'student'
        )
    );

-- This helper owns only private global Student Account resolution. The caller
-- must still establish exact Course Instructor authority in the enclosing API
-- procedure before it invokes this helper.
CREATE FUNCTION ple_private.resolve_or_create_live_demo_student_account(
    p_normalized_email text,
    p_delivery_email text
)
RETURNS uuid LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_private AS $$
DECLARE
    v_account_id uuid;
    v_created_at timestamp with time zone;
BEGIN
    IF p_normalized_email IS NULL
       OR char_length(p_normalized_email) NOT BETWEEN 3 AND 320
       OR p_normalized_email IS DISTINCT FROM lower(btrim(p_normalized_email))
       OR p_delivery_email IS NULL
       OR char_length(btrim(p_delivery_email)) NOT BETWEEN 3 AND 320 THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Student Authentication Email is invalid';
    END IF;

    SELECT account.account_id
      INTO v_account_id
      FROM ple_private.account_authentication_email AS email
      JOIN ple_private.account AS account ON account.account_id = email.account_id
      JOIN LATERAL (
          SELECT event.state
            FROM ple_private.account_state_event AS event
           WHERE event.account_id = account.account_id
           ORDER BY event.occurred_at DESC, event.event_id DESC
           LIMIT 1
      ) AS current_state ON current_state.state = 'active'
     WHERE email.normalized_email = p_normalized_email
       AND account.product_role = 'student';
    IF FOUND THEN
        RETURN v_account_id;
    END IF;

    IF EXISTS (
        SELECT 1
          FROM ple_private.account_authentication_email AS email
         WHERE email.normalized_email = p_normalized_email
    ) THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Student Authentication Email is unavailable';
    END IF;

    v_account_id := pg_catalog.gen_random_uuid();
    v_created_at := pg_catalog.transaction_timestamp();
    INSERT INTO ple_private.account (account_id, product_role, created_at)
    VALUES (v_account_id, 'student', v_created_at);
    INSERT INTO ple_private.account_authentication_email (
        account_id, normalized_email, delivery_email, verified_at, updated_at
    ) VALUES (
        v_account_id, p_normalized_email, p_delivery_email, v_created_at, v_created_at
    );
    RETURN v_account_id;
END
$$;

GRANT USAGE ON SCHEMA ple_private TO ple_api_owner;
GRANT SELECT, INSERT ON TABLE ple_private.course_roster_profile,
    ple_private.course_invitation, ple_private.course_invitation_event TO ple_api_owner;
GRANT UPDATE ON TABLE ple_private.course_invitation TO ple_api_owner;
CREATE POLICY course_roster_profile_api_owner_read
    ON ple_private.course_roster_profile FOR SELECT TO ple_api_owner USING (true);
CREATE POLICY course_roster_profile_api_owner_create
    ON ple_private.course_roster_profile FOR INSERT TO ple_api_owner WITH CHECK (true);
CREATE POLICY course_invitation_api_owner_m9_read
    ON ple_private.course_invitation FOR SELECT TO ple_api_owner USING (true);
CREATE POLICY course_invitation_api_owner_m9_create
    ON ple_private.course_invitation FOR INSERT TO ple_api_owner WITH CHECK (true);
CREATE POLICY course_invitation_api_owner_m9_lock
    ON ple_private.course_invitation FOR UPDATE TO ple_api_owner
    USING (true) WITH CHECK (false);
CREATE POLICY course_invitation_event_api_owner_m9_create
    ON ple_private.course_invitation_event FOR INSERT TO ple_api_owner WITH CHECK (true);
REVOKE ALL PRIVILEGES ON FUNCTION ple_private.resolve_or_create_live_demo_student_account(text, text)
    FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_private.resolve_or_create_live_demo_student_account(text, text)
    TO ple_api_owner;

RESET ROLE;

SET LOCAL ROLE ple_data_owner;

GRANT SELECT, INSERT ON TABLE ple_data.student_record TO ple_api_owner;
GRANT USAGE ON SCHEMA ple_data TO ple_audit_owner;
GRANT REFERENCES ON TABLE ple_data.course_instance TO ple_audit_owner;
CREATE POLICY student_record_api_owner_live_demo_create
    ON ple_data.student_record FOR INSERT TO ple_api_owner WITH CHECK (true);

RESET ROLE;

SET LOCAL ROLE ple_audit_owner;

CREATE TABLE ple_audit.course_roster_event (
    course_roster_event_id uuid PRIMARY KEY,
    course_id uuid NOT NULL REFERENCES ple_data.course_instance (course_id),
    student_account_id uuid NOT NULL REFERENCES ple_private.account (account_id),
    acting_account_id uuid NOT NULL REFERENCES ple_private.account (account_id),
    event_kind text NOT NULL CHECK (event_kind IN (
        'invitation_created', 'invitation_claimed', 'student_access_revoked'
    )),
    occurred_at timestamp with time zone NOT NULL
);
CREATE FUNCTION ple_audit.reject_course_roster_event_change()
RETURNS trigger LANGUAGE plpgsql
SET search_path = pg_catalog, ple_audit AS $$
BEGIN
    RAISE EXCEPTION USING ERRCODE = '23514',
        MESSAGE = 'a Course Roster Event is immutable';
END
$$;
CREATE TRIGGER course_roster_event_is_immutable
BEFORE UPDATE OR DELETE ON ple_audit.course_roster_event
FOR EACH ROW EXECUTE FUNCTION ple_audit.reject_course_roster_event_change();
ALTER TABLE ple_audit.course_roster_event ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_audit.course_roster_event FORCE ROW LEVEL SECURITY;
CREATE POLICY course_roster_event_audit_owner_create
    ON ple_audit.course_roster_event FOR INSERT TO ple_audit_owner WITH CHECK (true);
REVOKE ALL PRIVILEGES ON TABLE ple_audit.course_roster_event FROM PUBLIC;
REVOKE ALL PRIVILEGES ON FUNCTION ple_audit.reject_course_roster_event_change() FROM PUBLIC;

CREATE FUNCTION ple_audit.record_live_demo_course_roster_event(
    p_course_id uuid,
    p_student_account_id uuid,
    p_acting_account_id uuid,
    p_event_kind text
)
RETURNS void LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_audit AS $$
BEGIN
    IF p_course_id IS NULL OR p_student_account_id IS NULL OR p_acting_account_id IS NULL
       OR p_event_kind NOT IN ('invitation_created', 'invitation_claimed', 'student_access_revoked') THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Course Roster Event arguments are invalid';
    END IF;
    INSERT INTO ple_audit.course_roster_event (
        course_roster_event_id, course_id, student_account_id,
        acting_account_id, event_kind, occurred_at
    ) VALUES (
        pg_catalog.gen_random_uuid(), p_course_id, p_student_account_id,
        p_acting_account_id, p_event_kind, pg_catalog.transaction_timestamp()
    );
END
$$;
REVOKE ALL PRIVILEGES ON FUNCTION ple_audit.record_live_demo_course_roster_event(uuid, uuid, uuid, text)
    FROM PUBLIC;
GRANT USAGE ON SCHEMA ple_audit TO ple_api_owner;
GRANT EXECUTE ON FUNCTION ple_audit.record_live_demo_course_roster_event(uuid, uuid, uuid, text)
    TO ple_api_owner;

RESET ROLE;

SET LOCAL ROLE ple_api_owner;

CREATE FUNCTION ple_api.import_live_demo_course_roster(
    p_course_reference_number bigint,
    p_normalized_emails text[],
    p_delivery_emails text[],
    p_roster_ids text[]
)
RETURNS TABLE (roster_id text, roster_email text, state text)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private, ple_audit AS $$
DECLARE
    v_course_id uuid;
    v_actor_account_id uuid;
    v_student_account_id uuid;
    v_profile_id uuid;
    v_active_membership_id uuid;
    v_pending_invitation_id uuid;
    v_index integer;
    v_occurred_at timestamp with time zone;
BEGIN
    IF p_course_reference_number NOT BETWEEN 1 AND 2147483647
       OR cardinality(p_normalized_emails) NOT BETWEEN 1 AND 50
       OR cardinality(p_normalized_emails) IS DISTINCT FROM cardinality(p_delivery_emails)
       OR cardinality(p_normalized_emails) IS DISTINCT FROM cardinality(p_roster_ids) THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Course Roster Import arguments are invalid';
    END IF;
    SELECT course.course_id
      INTO v_course_id
      FROM ple_data.course_instance AS course
     WHERE course.reference_number = p_course_reference_number;
    IF NOT FOUND OR NOT ple_api.current_session_account_is_course_instructor(v_course_id) THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Course Roster Import requires a current Instructor Course Membership';
    END IF;
    v_actor_account_id := ple_api.current_session_account_id();
    v_occurred_at := pg_catalog.transaction_timestamp();

    FOR v_index IN 1 .. cardinality(p_normalized_emails) LOOP
        IF p_normalized_emails[v_index] IS NULL
           OR p_delivery_emails[v_index] IS NULL
           OR p_roster_ids[v_index] IS NULL
           OR char_length(p_roster_ids[v_index]) NOT BETWEEN 1 AND 64
           OR p_roster_ids[v_index] !~ '^[A-Za-z0-9._-]+$' THEN
            RAISE EXCEPTION USING ERRCODE = '22023',
                MESSAGE = 'Course Roster Import row is invalid';
        END IF;
        v_student_account_id := ple_private.resolve_or_create_live_demo_student_account(
            p_normalized_emails[v_index], p_delivery_emails[v_index]
        );
        INSERT INTO ple_private.course_roster_profile (
            course_roster_profile_id, course_id, student_account_id,
            roster_email, roster_id, created_at
        ) VALUES (
            pg_catalog.gen_random_uuid(), v_course_id, v_student_account_id,
            p_normalized_emails[v_index], p_roster_ids[v_index], v_occurred_at
        ) ON CONFLICT (course_id, student_account_id) DO NOTHING;
        SELECT profile.course_roster_profile_id
          INTO v_profile_id
          FROM ple_private.course_roster_profile AS profile
         WHERE profile.course_id = v_course_id
           AND profile.student_account_id = v_student_account_id;
        IF NOT FOUND OR EXISTS (
            SELECT 1 FROM ple_private.course_roster_profile AS profile
             WHERE profile.course_roster_profile_id = v_profile_id
               AND (profile.roster_id <> p_roster_ids[v_index]
                    OR profile.roster_email <> p_normalized_emails[v_index])
        ) THEN
            RAISE EXCEPTION USING ERRCODE = '23505',
                MESSAGE = 'Course Roster Import conflicts with existing course roster metadata';
        END IF;

        SELECT membership.membership_id
          INTO v_active_membership_id
          FROM ple_data.course_membership AS membership
         WHERE membership.course_id = v_course_id
           AND membership.account_id = v_student_account_id
           AND membership.role = 'student'
           AND ple_data.course_membership_is_active(membership.membership_id)
         LIMIT 1;
        IF FOUND THEN
            roster_id := p_roster_ids[v_index];
            roster_email := p_normalized_emails[v_index];
            state := 'active_student';
            RETURN NEXT;
            CONTINUE;
        END IF;

        SELECT invitation.invitation_id
          INTO v_pending_invitation_id
          FROM ple_private.course_invitation AS invitation
         WHERE invitation.course_id = v_course_id
           AND invitation.target_account_id = v_student_account_id
           AND invitation.membership_role = 'student'
           AND invitation.expires_at > pg_catalog.clock_timestamp()
           AND NOT EXISTS (
               SELECT 1 FROM ple_private.course_invitation_event AS event
                WHERE event.invitation_id = invitation.invitation_id
           )
         ORDER BY invitation.issued_at DESC, invitation.invitation_id DESC
         LIMIT 1;
        IF NOT FOUND THEN
            v_pending_invitation_id := pg_catalog.gen_random_uuid();
            INSERT INTO ple_private.course_invitation (
                invitation_id, course_id, target_account_id, membership_role, issued_at, expires_at
            ) VALUES (
                v_pending_invitation_id, v_course_id, v_student_account_id, 'student',
                v_occurred_at, v_occurred_at + interval '7 days'
            );
            PERFORM ple_audit.record_live_demo_course_roster_event(
                v_course_id, v_student_account_id, v_actor_account_id, 'invitation_created'
            );
        END IF;
        roster_id := p_roster_ids[v_index];
        roster_email := p_normalized_emails[v_index];
        state := 'invitation_pending';
        RETURN NEXT;
    END LOOP;
END
$$;

CREATE FUNCTION ple_api.list_live_demo_course_roster(p_course_reference_number bigint)
RETURNS TABLE (roster_id text, roster_email text, state text)
LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
    SELECT profile.roster_id, profile.roster_email,
           CASE
               WHEN EXISTS (
                   SELECT 1
                     FROM ple_data.course_membership AS membership
                    WHERE membership.course_id = course.course_id
                      AND membership.account_id = profile.student_account_id
                      AND membership.role = 'student'
                      AND ple_data.course_membership_is_active(membership.membership_id)
               ) THEN 'active_student'
               ELSE 'invitation_pending'
           END
      FROM ple_data.course_instance AS course
      JOIN ple_private.course_roster_profile AS profile ON profile.course_id = course.course_id
     WHERE course.reference_number = p_course_reference_number
       AND ple_api.current_session_account_is_course_instructor(course.course_id)
       AND (
           EXISTS (
               SELECT 1
                 FROM ple_data.course_membership AS membership
                WHERE membership.course_id = course.course_id
                  AND membership.account_id = profile.student_account_id
                  AND membership.role = 'student'
                  AND ple_data.course_membership_is_active(membership.membership_id)
           )
           OR EXISTS (
               SELECT 1
                 FROM ple_private.course_invitation AS invitation
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
     ORDER BY profile.roster_id
$$;

CREATE FUNCTION ple_api.claim_live_demo_course_invitation(
    p_student_record_id uuid,
    p_membership_id uuid,
    p_invitation_event_id uuid,
    p_course_reference_number bigint
)
RETURNS TABLE (active_student_membership boolean)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private, ple_audit AS $$
DECLARE
    v_course_id uuid;
    v_student_account_id uuid;
    v_student_record_id uuid;
    v_invitation_id uuid;
    v_expires_at timestamp with time zone;
BEGIN
    IF p_student_record_id IS NULL OR p_membership_id IS NULL OR p_invitation_event_id IS NULL
       OR p_course_reference_number NOT BETWEEN 1 AND 2147483647 THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Course Invitation claim arguments are invalid';
    END IF;
    v_student_account_id := ple_api.current_session_account_id();
    IF NOT EXISTS (
        SELECT 1
          FROM ple_private.account AS account
          JOIN LATERAL (
              SELECT event.state
                FROM ple_private.account_state_event AS event
               WHERE event.account_id = account.account_id
               ORDER BY event.occurred_at DESC, event.event_id DESC
               LIMIT 1
          ) AS current_state ON current_state.state = 'active'
         WHERE account.account_id = v_student_account_id
           AND account.product_role = 'student'
    ) THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Course Invitation claim requires an active Student Account';
    END IF;
    SELECT course.course_id
      INTO v_course_id
      FROM ple_data.course_instance AS course
     WHERE course.reference_number = p_course_reference_number;
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Course Invitation is unavailable';
    END IF;
    IF EXISTS (
        SELECT 1
          FROM ple_data.course_membership AS membership
         WHERE membership.course_id = v_course_id
           AND membership.account_id = v_student_account_id
           AND membership.role = 'student'
           AND ple_data.course_membership_is_active(membership.membership_id)
    ) THEN
        RETURN QUERY SELECT true;
        RETURN;
    END IF;
    SELECT invitation.invitation_id, invitation.expires_at
      INTO v_invitation_id, v_expires_at
      FROM ple_private.course_invitation AS invitation
     WHERE invitation.course_id = v_course_id
       AND invitation.target_account_id = v_student_account_id
       AND invitation.membership_role = 'student'
       AND invitation.expires_at > pg_catalog.clock_timestamp()
       AND NOT EXISTS (
           SELECT 1 FROM ple_private.course_invitation_event AS event
            WHERE event.invitation_id = invitation.invitation_id
       )
     ORDER BY invitation.issued_at DESC, invitation.invitation_id DESC
     LIMIT 1
     FOR UPDATE;
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Course Invitation is unavailable';
    END IF;
    INSERT INTO ple_data.student_record (
        student_record_id, course_id, student_account_id, created_at
    ) VALUES (
        p_student_record_id, v_course_id, v_student_account_id, pg_catalog.transaction_timestamp()
    ) ON CONFLICT (course_id, student_account_id) DO NOTHING;
    SELECT student_record.student_record_id
      INTO v_student_record_id
      FROM ple_data.student_record AS student_record
     WHERE student_record.course_id = v_course_id
       AND student_record.student_account_id = v_student_account_id;
    INSERT INTO ple_data.course_membership (
        membership_id, course_id, account_id, role, joined_at, student_record_id
    ) VALUES (
        p_membership_id, v_course_id, v_student_account_id, 'student',
        pg_catalog.transaction_timestamp(), v_student_record_id
    );
    INSERT INTO ple_private.course_invitation_event (
        course_invitation_event_id, invitation_id, event_kind,
        performed_by_account_id, occurred_at, reason
    ) VALUES (
        p_invitation_event_id, v_invitation_id, 'accepted', v_student_account_id,
        pg_catalog.transaction_timestamp(), 'student accepted Course Invitation'
    );
    PERFORM ple_audit.record_live_demo_course_roster_event(
        v_course_id, v_student_account_id, v_student_account_id, 'invitation_claimed'
    );
    RETURN QUERY SELECT true;
END
$$;

CREATE FUNCTION ple_api.revoke_live_demo_course_roster_entry(
    p_membership_event_id uuid,
    p_course_reference_number bigint,
    p_roster_id text
)
RETURNS void LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private, ple_audit AS $$
DECLARE
    v_course_id uuid;
    v_actor_account_id uuid;
    v_student_account_id uuid;
    v_membership_id uuid;
    v_invitation_id uuid;
BEGIN
    IF p_membership_event_id IS NULL
       OR p_course_reference_number NOT BETWEEN 1 AND 2147483647
       OR p_roster_id IS NULL OR char_length(p_roster_id) NOT BETWEEN 1 AND 64
       OR p_roster_id !~ '^[A-Za-z0-9._-]+$' THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Course roster revocation arguments are invalid';
    END IF;
    SELECT course.course_id
      INTO v_course_id
      FROM ple_data.course_instance AS course
     WHERE course.reference_number = p_course_reference_number;
    IF NOT FOUND OR NOT ple_api.current_session_account_is_course_instructor(v_course_id) THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Course roster revocation requires a current Instructor Course Membership';
    END IF;
    SELECT profile.student_account_id
      INTO v_student_account_id
      FROM ple_private.course_roster_profile AS profile
     WHERE profile.course_id = v_course_id AND profile.roster_id = p_roster_id;
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Course roster entry is unavailable';
    END IF;
    v_actor_account_id := ple_api.current_session_account_id();
    SELECT membership.membership_id
      INTO v_membership_id
      FROM ple_data.course_membership AS membership
     WHERE membership.course_id = v_course_id
       AND membership.account_id = v_student_account_id
       AND membership.role = 'student'
       AND ple_data.course_membership_is_active(membership.membership_id)
     LIMIT 1;
    IF FOUND THEN
        INSERT INTO ple_data.course_membership_event (
            course_membership_event_id, membership_id, event_kind, occurred_at, reason
        ) VALUES (
            p_membership_event_id, v_membership_id, 'ended', pg_catalog.transaction_timestamp(),
            'Instructor revoked Student course access'
        );
        PERFORM ple_audit.record_live_demo_course_roster_event(
            v_course_id, v_student_account_id, v_actor_account_id, 'student_access_revoked'
        );
        RETURN;
    END IF;
    SELECT invitation.invitation_id
      INTO v_invitation_id
      FROM ple_private.course_invitation AS invitation
     WHERE invitation.course_id = v_course_id
       AND invitation.target_account_id = v_student_account_id
       AND invitation.membership_role = 'student'
       AND invitation.expires_at > pg_catalog.clock_timestamp()
       AND NOT EXISTS (
           SELECT 1 FROM ple_private.course_invitation_event AS event
            WHERE event.invitation_id = invitation.invitation_id
       )
     ORDER BY invitation.issued_at DESC, invitation.invitation_id DESC
     LIMIT 1
     FOR UPDATE;
    IF FOUND THEN
        INSERT INTO ple_private.course_invitation_event (
            course_invitation_event_id, invitation_id, event_kind,
            performed_by_account_id, occurred_at, reason
        ) VALUES (
            p_membership_event_id, v_invitation_id, 'revoked', v_actor_account_id,
            pg_catalog.transaction_timestamp(), 'Instructor revoked pending Course Invitation'
        );
        PERFORM ple_audit.record_live_demo_course_roster_event(
            v_course_id, v_student_account_id, v_actor_account_id, 'student_access_revoked'
        );
    END IF;
END
$$;

REVOKE ALL PRIVILEGES ON FUNCTION ple_api.import_live_demo_course_roster(bigint, text[], text[], text[])
    FROM PUBLIC;
REVOKE ALL PRIVILEGES ON FUNCTION ple_api.list_live_demo_course_roster(bigint) FROM PUBLIC;
REVOKE ALL PRIVILEGES ON FUNCTION ple_api.claim_live_demo_course_invitation(uuid, uuid, uuid, bigint)
    FROM PUBLIC;
REVOKE ALL PRIVILEGES ON FUNCTION ple_api.revoke_live_demo_course_roster_entry(uuid, bigint, text)
    FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_api.import_live_demo_course_roster(bigint, text[], text[], text[]) TO ple_app;
GRANT EXECUTE ON FUNCTION ple_api.list_live_demo_course_roster(bigint) TO ple_app;
GRANT EXECUTE ON FUNCTION ple_api.claim_live_demo_course_invitation(uuid, uuid, uuid, bigint) TO ple_app;
GRANT EXECUTE ON FUNCTION ple_api.revoke_live_demo_course_roster_entry(uuid, bigint, text) TO ple_app;

RESET ROLE;
