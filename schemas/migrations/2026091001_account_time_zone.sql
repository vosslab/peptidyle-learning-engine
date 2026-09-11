-- Account-owned exact IANA time-zone preferences.
--
-- Stored deadlines remain timestamptz instants. This private mutable relation
-- owns only an Account's wall-clock interpretation and display preference.

DO $$
BEGIN
    IF current_user <> 'ple_migrator' THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'migration 2026091001 must run as ple_migrator';
    END IF;
END
$$;

SET LOCAL ROLE ple_private_owner;

-- ASVS 2.2.1--2.2.2: PostgreSQL independently allow-lists its exact installed
-- IANA names; Rust performs the matching trusted-service validation.
CREATE FUNCTION ple_private.account_time_zone_is_exact_iana(p_time_zone text)
RETURNS boolean
LANGUAGE sql STABLE
SET search_path = pg_catalog
AS $$
    SELECT p_time_zone IS NOT NULL
       AND p_time_zone = btrim(p_time_zone)
       AND char_length(p_time_zone) BETWEEN 1 AND 100
       AND EXISTS (
           SELECT 1
             FROM pg_catalog.pg_timezone_names AS time_zone
            WHERE time_zone.name = p_time_zone
       )
$$;

CREATE TABLE ple_private.account_time_zone (
    account_id uuid PRIMARY KEY REFERENCES ple_private.account (account_id),
    time_zone text NOT NULL,
    CONSTRAINT account_time_zone_is_valid_iana CHECK (
        ple_private.account_time_zone_is_exact_iana(time_zone)
    )
);

CREATE FUNCTION ple_private.reject_invalid_account_time_zone()
RETURNS trigger
LANGUAGE plpgsql
SET search_path = pg_catalog, ple_private
AS $$
BEGIN
    IF NOT ple_private.account_time_zone_is_exact_iana(NEW.time_zone) THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Account time zone must be an exact known IANA name';
    END IF;
    RETURN NEW;
END
$$;

CREATE TRIGGER account_time_zone_is_exact_iana
BEFORE INSERT OR UPDATE OF time_zone ON ple_private.account_time_zone
FOR EACH ROW EXECUTE FUNCTION ple_private.reject_invalid_account_time_zone();

ALTER TABLE ple_private.account_time_zone ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.account_time_zone FORCE ROW LEVEL SECURITY;
CREATE POLICY account_time_zone_private_owner_read
    ON ple_private.account_time_zone FOR SELECT TO ple_private_owner USING (true);
CREATE POLICY account_time_zone_private_owner_create
    ON ple_private.account_time_zone FOR INSERT TO ple_private_owner WITH CHECK (true);
CREATE POLICY account_time_zone_private_owner_change
    ON ple_private.account_time_zone FOR UPDATE TO ple_private_owner
    USING (true) WITH CHECK (true);

REVOKE ALL PRIVILEGES ON TABLE ple_private.account_time_zone FROM PUBLIC;
REVOKE ALL PRIVILEGES ON FUNCTION ple_private.account_time_zone_is_exact_iana(text) FROM PUBLIC;
REVOKE ALL PRIVILEGES ON FUNCTION ple_private.reject_invalid_account_time_zone() FROM PUBLIC;
GRANT SELECT, INSERT, UPDATE ON TABLE ple_private.account_time_zone TO ple_private_owner;

COMMENT ON TABLE ple_private.account_time_zone IS
    'Mutable Account-owned exact IANA time-zone preference for wall-clock interpretation and display.';
COMMENT ON COLUMN ple_private.account_time_zone.time_zone IS
    'Exact case-sensitive IANA name, independently validated by Rust and PostgreSQL.';

-- Every Account creation path establishes a safe local default in its own
-- transaction. Student roster creation immediately replaces this default with
-- its authorized Instructor's preference below.
CREATE FUNCTION ple_private.record_default_account_time_zone()
RETURNS trigger
LANGUAGE plpgsql
SET search_path = pg_catalog, ple_private
AS $$
BEGIN
    INSERT INTO ple_private.account_time_zone (account_id, time_zone)
    VALUES (NEW.account_id, 'America/Chicago');
    RETURN NEW;
END
$$;

CREATE TRIGGER account_creation_records_default_time_zone
AFTER INSERT ON ple_private.account
FOR EACH ROW EXECUTE FUNCTION ple_private.record_default_account_time_zone();

REVOKE ALL PRIVILEGES ON FUNCTION ple_private.record_default_account_time_zone() FROM PUBLIC;

-- ASVS 8.2.2 and 8.3.1: internal callers derive their subject from the
-- installed session instead of accepting an Account identifier.
CREATE FUNCTION ple_private.current_authenticated_account_time_zone()
RETURNS text
LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private
AS $$
    SELECT preference.time_zone
      FROM ple_private.account_time_zone AS preference
      JOIN ple_private.account AS account ON account.account_id = preference.account_id
      JOIN LATERAL (
          SELECT event.state
            FROM ple_private.account_state_event AS event
           WHERE event.account_id = account.account_id
           ORDER BY event.occurred_at DESC, event.event_id DESC
           LIMIT 1
      ) AS current_state ON current_state.state = 'active'
     WHERE account.account_id = ple_api.current_session_account_id()
$$;

-- The live demo has one established campus default. Existing Accounts gain a
-- preference before any authenticated wall-clock path can read one.
INSERT INTO ple_private.account_time_zone (account_id, time_zone)
SELECT account.account_id, 'America/Chicago'
  FROM ple_private.account AS account;

-- Wrap the established Sysadmin lifecycle creator without changing its
-- browser-safe summary. The Account creation trigger above establishes its
-- preference while identity remains in the immutable account table.
CREATE OR REPLACE FUNCTION ple_private.create_live_demo_instructor_account(p_normalized_email text)
RETURNS TABLE (
    reference_number bigint,
    state text,
    last_successful_sign_in timestamp with time zone
)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private
AS $$
DECLARE
    v_account_id uuid;
BEGIN
    PERFORM ple_private.require_live_demo_sysadmin_account();
    SELECT created.account_id
      INTO v_account_id
      FROM ple_private.create_instructor_account(p_normalized_email, p_normalized_email) AS created;
    RETURN QUERY
    SELECT summary.reference_number, summary.state, summary.last_successful_sign_in
      FROM ple_private.live_demo_instructor_account_summary(v_account_id) AS summary;
END
$$;

-- The initial zone is supplied only by the already-authorized roster import.
-- Existing Student Accounts return before this insert, preserving their own
-- preference across enrollment and re-enrollment.
CREATE OR REPLACE FUNCTION ple_private.resolve_or_create_live_demo_student_account(
    p_normalized_email text,
    p_delivery_email text,
    p_initial_time_zone text
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
       OR char_length(btrim(p_delivery_email)) NOT BETWEEN 3 AND 320
       OR NOT ple_private.account_time_zone_is_exact_iana(p_initial_time_zone) THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Student Account creation arguments are invalid';
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
        SELECT 1 FROM ple_private.account_authentication_email AS email
         WHERE email.normalized_email = p_normalized_email
    ) THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Student Authentication Email is unavailable';
    END IF;
    v_account_id := pg_catalog.gen_random_uuid();
    v_created_at := pg_catalog.transaction_timestamp();
    INSERT INTO ple_private.account (account_id, product_role, created_at)
    VALUES (v_account_id, 'student', v_created_at);
    UPDATE ple_private.account_time_zone
       SET time_zone = p_initial_time_zone
     WHERE account_id = v_account_id;
    INSERT INTO ple_private.account_authentication_email (
        account_id, normalized_email, delivery_email, verified_at, updated_at
    ) VALUES (
        v_account_id, p_normalized_email, p_delivery_email, v_created_at, v_created_at
    );
    RETURN v_account_id;
END
$$;

DROP FUNCTION ple_private.resolve_or_create_live_demo_student_account(text, text);

REVOKE ALL PRIVILEGES ON FUNCTION ple_private.create_live_demo_instructor_account(text) FROM PUBLIC;
REVOKE ALL PRIVILEGES ON FUNCTION ple_private.resolve_or_create_live_demo_student_account(text, text, text)
    FROM PUBLIC;
REVOKE ALL PRIVILEGES ON FUNCTION ple_private.current_authenticated_account_time_zone() FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_private.create_live_demo_instructor_account(text) TO ple_api_owner;
GRANT EXECUTE ON FUNCTION ple_private.resolve_or_create_live_demo_student_account(text, text, text)
    TO ple_api_owner;
GRANT EXECUTE ON FUNCTION ple_private.current_authenticated_account_time_zone() TO ple_api_owner;
RESET ROLE;

SET LOCAL ROLE ple_api_owner;

-- ASVS 8.2.2 and 8.3.1: no Account ID enters this self-only read; the trusted
-- database derives the exact active subject from the installed session.
CREATE FUNCTION ple_api.current_account_time_zone()
RETURNS text
LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private
AS $$ SELECT ple_private.current_authenticated_account_time_zone() $$;

-- Roster import establishes Course Instructor authority before it captures the
-- actor's own preference and delegates Student Account creation.
CREATE OR REPLACE FUNCTION ple_api.import_live_demo_course_roster(
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
    v_instructor_time_zone text;
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
    SELECT course.course_id INTO v_course_id
      FROM ple_data.course_instance AS course
     WHERE course.reference_number = p_course_reference_number;
    IF NOT FOUND OR NOT ple_api.current_session_account_is_course_instructor(v_course_id) THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Course Roster Import requires a current Instructor Course Membership';
    END IF;
    v_actor_account_id := ple_api.current_session_account_id();
    v_instructor_time_zone := ple_private.current_authenticated_account_time_zone();
    IF v_instructor_time_zone IS NULL THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Course Roster Import requires an Account time zone';
    END IF;
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
            p_normalized_emails[v_index], p_delivery_emails[v_index], v_instructor_time_zone
        );
        INSERT INTO ple_private.course_roster_profile (
            course_roster_profile_id, course_id, student_account_id,
            roster_email, roster_id, created_at
        ) VALUES (
            pg_catalog.gen_random_uuid(), v_course_id, v_student_account_id,
            p_normalized_emails[v_index], p_roster_ids[v_index], v_occurred_at
        ) ON CONFLICT (course_id, student_account_id) DO NOTHING;
        SELECT profile.course_roster_profile_id INTO v_profile_id
          FROM ple_private.course_roster_profile AS profile
         WHERE profile.course_id = v_course_id AND profile.student_account_id = v_student_account_id;
        IF NOT FOUND OR EXISTS (
            SELECT 1 FROM ple_private.course_roster_profile AS profile
             WHERE profile.course_roster_profile_id = v_profile_id
               AND (profile.roster_id <> p_roster_ids[v_index]
                    OR profile.roster_email <> p_normalized_emails[v_index])
        ) THEN
            RAISE EXCEPTION USING ERRCODE = '23505',
                MESSAGE = 'Course Roster Import conflicts with existing course roster metadata';
        END IF;
        SELECT membership.membership_id INTO v_active_membership_id
          FROM ple_data.course_membership AS membership
         WHERE membership.course_id = v_course_id AND membership.account_id = v_student_account_id
           AND membership.role = 'student'
           AND ple_data.course_membership_is_active(membership.membership_id)
         LIMIT 1;
        IF FOUND THEN
            roster_id := p_roster_ids[v_index]; roster_email := p_normalized_emails[v_index];
            state := 'active_student'; RETURN NEXT; CONTINUE;
        END IF;
        SELECT invitation.invitation_id INTO v_pending_invitation_id
          FROM ple_private.course_invitation AS invitation
         WHERE invitation.course_id = v_course_id
           AND invitation.target_account_id = v_student_account_id
           AND invitation.membership_role = 'student'
           AND invitation.expires_at > pg_catalog.clock_timestamp()
           AND NOT EXISTS (
               SELECT 1 FROM ple_private.course_invitation_event AS event
                WHERE event.invitation_id = invitation.invitation_id
           )
         ORDER BY invitation.issued_at DESC, invitation.invitation_id DESC LIMIT 1;
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
        roster_id := p_roster_ids[v_index]; roster_email := p_normalized_emails[v_index];
        state := 'invitation_pending'; RETURN NEXT;
    END LOOP;
END
$$;

REVOKE ALL PRIVILEGES ON FUNCTION ple_api.current_account_time_zone() FROM PUBLIC;
REVOKE ALL PRIVILEGES ON FUNCTION ple_api.import_live_demo_course_roster(bigint, text[], text[], text[])
    FROM PUBLIC;
GRANT USAGE ON SCHEMA ple_api TO ple_app;
GRANT EXECUTE ON FUNCTION ple_api.current_account_time_zone() TO ple_app;
GRANT EXECUTE ON FUNCTION ple_api.import_live_demo_course_roster(bigint, text[], text[], text[])
    TO ple_app;
RESET ROLE;
