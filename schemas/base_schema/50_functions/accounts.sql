-- Functions, triggers, and views from accounts.sql.

SET LOCAL ROLE ple_private_owner;

CREATE FUNCTION ple_private.reject_account_identity_change()
RETURNS trigger LANGUAGE plpgsql
SET search_path = pg_catalog, ple_private
AS $$
BEGIN
    IF NEW.account_id IS DISTINCT FROM OLD.account_id
       OR NEW.user_role IS DISTINCT FROM OLD.user_role
       OR NEW.created_at IS DISTINCT FROM OLD.created_at THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Account identity and User Role are immutable';
    END IF;
    RETURN NEW;
END
$$;

CREATE FUNCTION ple_private.record_initial_account_state()
RETURNS trigger LANGUAGE plpgsql
SET search_path = pg_catalog, ple_private
AS $$
BEGIN
    INSERT INTO ple_private.account_state_event (event_id, account_id, state, occurred_at)
    VALUES (pg_catalog.gen_random_uuid(), NEW.account_id, 'active', NEW.created_at);
    RETURN NEW;
END
$$;

CREATE TRIGGER account_identity_is_immutable
BEFORE UPDATE ON ple_private.account
FOR EACH ROW EXECUTE FUNCTION ple_private.reject_account_identity_change();

CREATE TRIGGER account_creation_records_active_state
AFTER INSERT ON ple_private.account
FOR EACH ROW EXECUTE FUNCTION ple_private.record_initial_account_state();

CREATE TRIGGER account_public_id_is_minted
BEFORE INSERT ON ple_private.account
FOR EACH ROW EXECUTE FUNCTION ple_private.assign_public_id('U');

CREATE FUNCTION ple_private.reject_invalid_account_time_zone()
RETURNS trigger LANGUAGE plpgsql
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

CREATE FUNCTION ple_private.record_default_account_time_zone()
RETURNS trigger LANGUAGE plpgsql
SET search_path = pg_catalog, ple_private
AS $$
BEGIN
    INSERT INTO ple_private.account_time_zone (
        account_id, time_zone, student_invitation_default_pending
    ) VALUES (NEW.account_id, 'America/Chicago', false);
    RETURN NEW;
END
$$;

CREATE TRIGGER account_time_zone_is_exact_iana
BEFORE INSERT OR UPDATE OF time_zone ON ple_private.account_time_zone
FOR EACH ROW EXECUTE FUNCTION ple_private.reject_invalid_account_time_zone();

CREATE TRIGGER account_creation_records_default_time_zone
AFTER INSERT ON ple_private.account
FOR EACH ROW EXECUTE FUNCTION ple_private.record_default_account_time_zone();

CREATE FUNCTION ple_private.record_default_account_appearance()
RETURNS trigger LANGUAGE plpgsql
SET search_path = pg_catalog, ple_private
AS $$
BEGIN
    INSERT INTO ple_private.account_appearance (account_id) VALUES (NEW.account_id);
    IF NEW.user_role = 'instructor' THEN
        INSERT INTO ple_private.instructor_personal_theme (account_id) VALUES (NEW.account_id);
    END IF;
    RETURN NEW;
END
$$;

CREATE TRIGGER account_creation_records_default_appearance
AFTER INSERT ON ple_private.account
FOR EACH ROW EXECUTE FUNCTION ple_private.record_default_account_appearance();

SET LOCAL ROLE ple_audit_owner;

CREATE FUNCTION ple_audit.reject_instructor_account_creation_event_change()
RETURNS trigger LANGUAGE plpgsql
SET search_path = pg_catalog, ple_audit
AS $$
BEGIN
    RAISE EXCEPTION USING ERRCODE = '23514',
        MESSAGE = 'Instructor Account creation evidence is immutable';
END
$$;

CREATE TRIGGER instructor_account_creation_event_is_immutable
BEFORE UPDATE OR DELETE ON ple_audit.instructor_account_creation_event
FOR EACH ROW EXECUTE FUNCTION ple_audit.reject_instructor_account_creation_event_change();

CREATE FUNCTION ple_audit.record_instructor_account_creation_event(
    p_created_instructor_account_id text,
    p_created_by_sysadmin_account_id text
)
RETURNS void LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_audit
AS $$
BEGIN
    IF p_created_instructor_account_id IS NULL
       OR p_created_by_sysadmin_account_id IS NULL THEN
        RAISE EXCEPTION USING ERRCODE = '22004',
            MESSAGE = 'Instructor Account creation evidence requires subject and actor';
    END IF;
    INSERT INTO ple_audit.instructor_account_creation_event (
        event_id, created_instructor_account_id, created_by_sysadmin_account_id, occurred_at
    ) VALUES (
        pg_catalog.gen_random_uuid(), p_created_instructor_account_id,
        p_created_by_sysadmin_account_id, pg_catalog.transaction_timestamp()
    ) ON CONFLICT (created_instructor_account_id) DO NOTHING;
    IF NOT FOUND AND NOT EXISTS (
        SELECT 1 FROM ple_audit.instructor_account_creation_event
         WHERE created_instructor_account_id = p_created_instructor_account_id
           AND created_by_sysadmin_account_id = p_created_by_sysadmin_account_id
    ) THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Instructor Account creation evidence conflicts with immutable history';
    END IF;
END
$$;

SET LOCAL ROLE ple_private_owner;

CREATE FUNCTION ple_private.require_current_sysadmin_account()
RETURNS text LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private
AS $$
DECLARE v_account_id text;
BEGIN
    v_account_id := ple_api.current_session_account_id();
    -- C10 consumes C24's one platform-administration authority rather than
    -- recreating a parallel Account-role predicate. Course help stays scoped
    -- to C25/C26 capability operations, never this account boundary.
    IF v_account_id IS NULL
       OR NOT ple_api.current_session_account_has_platform_administration() THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Active Sysadmin Account required';
    END IF;
    RETURN v_account_id;
END
$$;

CREATE FUNCTION ple_private.current_authenticated_account_time_zone()
RETURNS text LANGUAGE plpgsql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private
AS $$
DECLARE v_account_id text;
BEGIN
    v_account_id := ple_api.current_session_account_id();
    IF v_account_id IS NULL OR NOT (
        ple_api.current_session_account_has_active_role('student')
        OR ple_api.current_session_account_has_active_role('instructor')
        OR ple_api.current_session_account_has_active_role('sysadmin')
    ) THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Active Account required';
    END IF;
    RETURN (
        SELECT preference.time_zone FROM ple_private.account_time_zone AS preference
        JOIN ple_private.account AS account ON account.account_id = preference.account_id
        WHERE account.account_id = v_account_id
    );
END
$$;

CREATE FUNCTION ple_private.apply_student_invitation_time_zone_default(
    p_student_account_id text, p_inviting_instructor_account_id text
)
RETURNS void LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_private
AS $$
BEGIN
    UPDATE ple_private.account_time_zone AS student_preference
       SET time_zone = instructor_preference.time_zone,
           student_invitation_default_pending = false
      FROM ple_private.account_time_zone AS instructor_preference
      JOIN ple_private.account AS instructor
        ON instructor.account_id = instructor_preference.account_id
       AND instructor.user_role = 'instructor'
      JOIN ple_private.account AS student
        ON student.account_id = p_student_account_id AND student.user_role = 'student'
     WHERE student_preference.account_id = student.account_id
       AND student_preference.student_invitation_default_pending
       AND instructor.account_id = p_inviting_instructor_account_id;
END
$$;

CREATE FUNCTION ple_private.resolve_or_create_student_account(
    p_normalized_email text, p_delivery_email text
)
RETURNS text LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_private
AS $$
DECLARE v_account_id text; v_now timestamptz := pg_catalog.transaction_timestamp();
BEGIN
    IF p_normalized_email IS NULL
       OR char_length(p_normalized_email) NOT BETWEEN 3 AND 320
       OR p_normalized_email IS DISTINCT FROM lower(btrim(p_normalized_email))
       OR p_delivery_email IS NULL OR char_length(btrim(p_delivery_email)) NOT BETWEEN 3 AND 320
       OR p_normalized_email !~ '^[^@[:space:]]+@([a-z0-9-]+\.)+edu$' THEN
        RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Student Account input is invalid';
    END IF;
    SELECT account.account_id INTO v_account_id
    FROM ple_private.account_authentication_email AS email
    JOIN ple_private.account AS account ON account.account_id = email.account_id
    WHERE email.normalized_email = p_normalized_email AND account.user_role = 'student';
    IF FOUND THEN RETURN v_account_id; END IF;
    IF EXISTS (SELECT 1 FROM ple_private.account_authentication_email WHERE normalized_email = p_normalized_email) THEN
        RAISE EXCEPTION USING ERRCODE = '23505', MESSAGE = 'Student Authentication Email is unavailable';
    END IF;
    INSERT INTO ple_private.account AS new_account (account_id, user_role, created_at)
    VALUES ('U00000009', 'student', v_now)
    RETURNING new_account.account_id INTO v_account_id;
    UPDATE ple_private.account_time_zone
       SET student_invitation_default_pending = true
     WHERE account_id = v_account_id;
    INSERT INTO ple_private.account_authentication_email (
        account_id, normalized_email, delivery_email, verified_at, updated_at
    ) VALUES (v_account_id, p_normalized_email, p_delivery_email, v_now, v_now);
    RETURN v_account_id;
END
$$;

CREATE FUNCTION ple_private.create_instructor_account(
    p_normalized_email text, p_delivery_email text, p_first_name text,
    p_last_name text, p_affiliation text
)
RETURNS TABLE (account_id text, created_at timestamp with time zone)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_audit, ple_private
AS $$
DECLARE v_account_id text; v_actor_account_id text; v_created_at timestamptz;
BEGIN
    IF p_normalized_email IS NULL
       OR char_length(p_normalized_email) NOT BETWEEN 3 AND 320
       OR p_normalized_email IS DISTINCT FROM lower(btrim(p_normalized_email))
       OR p_delivery_email IS NULL OR char_length(btrim(p_delivery_email)) NOT BETWEEN 3 AND 320
       OR p_first_name IS NULL OR p_first_name IS DISTINCT FROM btrim(p_first_name)
       OR char_length(p_first_name) NOT BETWEEN 1 AND 100 OR p_first_name ~ '[[:cntrl:]]'
       OR p_last_name IS NULL OR p_last_name IS DISTINCT FROM btrim(p_last_name)
       OR char_length(p_last_name) NOT BETWEEN 1 AND 100 OR p_last_name ~ '[[:cntrl:]]'
       OR p_affiliation IS NULL OR p_affiliation IS DISTINCT FROM btrim(p_affiliation)
       OR char_length(p_affiliation) NOT BETWEEN 1 AND 300 OR p_affiliation ~ '[[:cntrl:]]' THEN
        RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Invalid Instructor Account input';
    END IF;
    v_actor_account_id := ple_private.require_current_sysadmin_account();
    -- ASVS 2.2.1, 2.2.2, 2.3.3, 8.2.1, and 8.3.1: validate every setup
    -- field before the one atomic account, email, Profile, and audit write;
    -- only the installed active Sysadmin session supplies the actor.
    v_created_at := pg_catalog.transaction_timestamp();
    INSERT INTO ple_private.account AS new_account (account_id, user_role, created_at)
    VALUES ('U00000009', 'instructor', v_created_at)
    RETURNING new_account.account_id INTO v_account_id;
    INSERT INTO ple_private.account_authentication_email (
        account_id, normalized_email, delivery_email, verified_at, updated_at
    ) VALUES (v_account_id, p_normalized_email, p_delivery_email, v_created_at, v_created_at);
    INSERT INTO ple_private.instructor_profile (
        account_id, first_name, last_name, affiliation, created_at, updated_at
    ) VALUES (v_account_id, p_first_name, p_last_name, p_affiliation, v_created_at, v_created_at);
    PERFORM ple_audit.record_instructor_account_creation_event(
        v_account_id, v_actor_account_id
    );
    RETURN QUERY SELECT v_account_id, v_created_at;
END
$$;

-- A public Instructor name comes from the Account Profile created after
-- outside vetting. It is not a role, status, or approval projection.
CREATE FUNCTION ple_private.instructor_display_name(p_instructor_account_id text)
RETURNS text LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_private
AS $$
    SELECT profile.first_name || ' ' || profile.last_name
      FROM ple_private.instructor_profile AS profile
     WHERE profile.account_id = p_instructor_account_id
$$;

CREATE FUNCTION ple_private.instructor_affiliation(p_instructor_account_id text)
RETURNS text LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_private
AS $$
    SELECT profile.affiliation
      FROM ple_private.instructor_profile AS profile
     WHERE profile.account_id = p_instructor_account_id
$$;

-- This private value reaches only the configured server-side mail adapter.
-- The caller must be the installed active Sysadmin; browser DTOs never carry it.
CREATE FUNCTION ple_private.instructor_setup_email_destination(p_account_id text)
RETURNS text LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_private
AS $$
DECLARE v_delivery_email text;
BEGIN
    PERFORM ple_private.require_current_sysadmin_account();
    SELECT email.delivery_email INTO v_delivery_email
      FROM ple_private.account AS account
      JOIN ple_private.account_authentication_email AS email ON email.account_id = account.account_id
     WHERE account.account_id = p_account_id AND account.user_role = 'instructor';
    RETURN v_delivery_email;
END
$$;

CREATE FUNCTION ple_private.instructor_account_summary(p_account_id text)
RETURNS TABLE (account_id text, state text, last_successful_sign_in timestamp with time zone)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_private
AS $$
BEGIN
    RETURN QUERY
    SELECT account.account_id::text, current_state.state::text,
           (SELECT max(session.created_at) FROM ple_private.authenticated_session AS session
             WHERE session.account_id = account.account_id)
      FROM ple_private.account AS account
      JOIN LATERAL (
          SELECT state_event.state FROM ple_private.account_state_event AS state_event
           WHERE state_event.account_id = account.account_id
           ORDER BY state_event.occurred_at DESC, state_event.event_id DESC LIMIT 1
      ) AS current_state ON true
     WHERE account.account_id = p_account_id AND account.user_role = 'instructor';
END
$$;

CREATE FUNCTION ple_private.list_instructor_accounts()
RETURNS TABLE (account_id text, state text, last_successful_sign_in timestamp with time zone)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_private
AS $$
BEGIN
    PERFORM ple_private.require_current_sysadmin_account();
    RETURN QUERY
    SELECT summary.account_id, summary.state, summary.last_successful_sign_in
    FROM ple_private.account AS account
    CROSS JOIN LATERAL ple_private.instructor_account_summary(account.account_id) AS summary
    WHERE account.user_role = 'instructor' ORDER BY summary.account_id;
END
$$;

CREATE FUNCTION ple_private.create_instructor_account_summary(
    p_normalized_email text, p_first_name text, p_last_name text, p_affiliation text
)
RETURNS TABLE (account_id text, state text, last_successful_sign_in timestamp with time zone)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_private
AS $$
DECLARE v_account_id text;
BEGIN
    SELECT created.account_id INTO v_account_id
    FROM ple_private.create_instructor_account(
        p_normalized_email, p_normalized_email, p_first_name, p_last_name, p_affiliation
    ) AS created;
    RETURN QUERY SELECT * FROM ple_private.instructor_account_summary(v_account_id);
END
$$;

CREATE FUNCTION ple_private.change_instructor_account_state(
    p_account_id text, p_next_state text, p_reason text
)
RETURNS TABLE (account_id text, state text, last_successful_sign_in timestamp with time zone)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_private
AS $$
DECLARE v_account_id text; v_current_state text;
BEGIN
    PERFORM ple_private.require_current_sysadmin_account();
    IF p_next_state NOT IN ('active', 'deactivated') OR p_account_id IS NULL THEN
        RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Instructor Account state input is invalid';
    END IF;
    SELECT account.account_id, state_event.state INTO v_account_id, v_current_state
    FROM ple_private.account AS account
    JOIN LATERAL (
        SELECT event.state FROM ple_private.account_state_event AS event
         WHERE event.account_id = account.account_id
         ORDER BY event.occurred_at DESC, event.event_id DESC LIMIT 1
    ) AS state_event ON true
    WHERE account.account_id = p_account_id AND account.user_role = 'instructor'
    FOR UPDATE OF account;
    IF NOT FOUND OR v_current_state = p_next_state THEN RETURN; END IF;
    IF p_next_state = 'deactivated' AND char_length(btrim(p_reason)) NOT BETWEEN 1 AND 1000 THEN
        RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Instructor deactivation requires a reason';
    END IF;
    INSERT INTO ple_private.account_state_event (event_id, account_id, state, occurred_at, reason)
    VALUES (pg_catalog.gen_random_uuid(), v_account_id, p_next_state::ple_data.account_state,
            pg_catalog.transaction_timestamp(), CASE WHEN p_next_state = 'active' THEN NULL ELSE p_reason END);
    RETURN QUERY SELECT * FROM ple_private.instructor_account_summary(v_account_id);
END
$$;

CREATE FUNCTION ple_private.update_current_authenticated_account_time_zone(p_time_zone text)
RETURNS text LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private
AS $$
DECLARE v_account_id text;
BEGIN
    -- ASVS 2.2.1--2.2.2 and 8.2.2: positive IANA validation and the installed
    -- session are the only accepted mutation inputs. No Account ID or role is
    -- browser-controlled at this boundary.
    IF NOT ple_private.account_time_zone_is_exact_iana(p_time_zone) THEN
        RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Account time zone is invalid';
    END IF;
    v_account_id := ple_api.current_session_account_id();
    IF v_account_id IS NULL OR NOT (
        ple_api.current_session_account_has_active_role('student')
        OR ple_api.current_session_account_has_active_role('instructor')
        OR ple_api.current_session_account_has_active_role('sysadmin')
    ) THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Active Account required';
    END IF;
    UPDATE ple_private.account_time_zone
       SET time_zone = p_time_zone,
           student_invitation_default_pending = CASE
               WHEN EXISTS (
                   SELECT 1 FROM ple_private.account AS account
                    WHERE account.account_id = v_account_id
                      AND account.user_role = 'student'
               ) THEN false
               ELSE student_invitation_default_pending
           END
     WHERE account_id = v_account_id
    RETURNING time_zone INTO p_time_zone;
    RETURN p_time_zone;
END
$$;

CREATE FUNCTION ple_private.current_authenticated_account_appearance()
RETURNS TABLE(display_mode_preference text, personal_theme text)
LANGUAGE plpgsql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private
AS $$
DECLARE v_account_id text;
BEGIN
    v_account_id := ple_api.current_session_account_id();
    IF v_account_id IS NULL OR NOT (
        ple_api.current_session_account_has_active_role('student')
        OR ple_api.current_session_account_has_active_role('instructor')
        OR ple_api.current_session_account_has_active_role('sysadmin')
    ) THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Active Account required';
    END IF;
    RETURN QUERY
    SELECT appearance.display_mode_preference::text,
           CASE WHEN account.user_role = 'instructor' THEN personal.theme_id END
      FROM ple_private.account AS account
      JOIN ple_private.account_appearance AS appearance ON appearance.account_id = account.account_id
      LEFT JOIN ple_private.instructor_personal_theme AS personal ON personal.account_id = account.account_id
     WHERE account.account_id = v_account_id;
END
$$;

CREATE FUNCTION ple_private.update_current_authenticated_account_display_mode_preference(
    p_display_mode_preference text
)
RETURNS text LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private
AS $$
DECLARE v_account_id text;
BEGIN
    -- ASVS 1.2.4 and 2.2.1--2.2.2: this parameterized procedure accepts only
    -- the two closed display values or NULL, and derives its subject from the
    -- installed session rather than any browser-controlled Account or role.
    IF p_display_mode_preference IS NOT NULL
       AND p_display_mode_preference NOT IN ('light', 'dark') THEN
        RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Display mode preference is invalid';
    END IF;
    v_account_id := ple_api.current_session_account_id();
    IF v_account_id IS NULL OR NOT (
        ple_api.current_session_account_has_active_role('student')
        OR ple_api.current_session_account_has_active_role('instructor')
        OR ple_api.current_session_account_has_active_role('sysadmin')
    ) THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Active Account required';
    END IF;
    UPDATE ple_private.account_appearance
       SET display_mode_preference = p_display_mode_preference::ple_data.display_mode,
           updated_at = pg_catalog.transaction_timestamp()
     WHERE account_id = v_account_id
 RETURNING display_mode_preference::text INTO p_display_mode_preference;
    RETURN p_display_mode_preference;
END
$$;

CREATE FUNCTION ple_private.update_current_authenticated_instructor_personal_theme(p_theme text)
RETURNS text LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private
AS $$
DECLARE v_account_id text;
BEGIN
    -- ASVS 1.2.4 and 2.2.1--2.2.2: a parameterized closed theme value is
    -- independently checked here; the authenticated Instructor is derived
    -- exclusively from the installed server-side session.
    IF p_theme IS NULL OR NOT EXISTS (SELECT 1 FROM ple_data.theme WHERE theme_id = p_theme) THEN
        RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Personal theme is invalid';
    END IF;
    v_account_id := ple_api.current_session_account_id();
    IF v_account_id IS NULL
       OR NOT ple_api.current_session_account_has_active_role('instructor') THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Active Instructor Account required';
    END IF;
    UPDATE ple_private.instructor_personal_theme
       SET theme_id = p_theme, updated_at = pg_catalog.transaction_timestamp()
     WHERE account_id = v_account_id
 RETURNING theme_id INTO p_theme;
    IF p_theme IS NULL THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Active Instructor Account required';
    END IF;
    RETURN p_theme;
END
$$;

SET LOCAL ROLE ple_api_owner;

CREATE FUNCTION ple_api.current_account_time_zone()
RETURNS text LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private
AS $$ SELECT ple_private.current_authenticated_account_time_zone() $$;

CREATE FUNCTION ple_api.update_current_account_time_zone(p_time_zone text)
RETURNS text LANGUAGE sql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private
AS $$ SELECT ple_private.update_current_authenticated_account_time_zone(p_time_zone) $$;

CREATE FUNCTION ple_api.current_account_appearance()
RETURNS TABLE(display_mode_preference text, personal_theme text)
LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private
AS $$ SELECT * FROM ple_private.current_authenticated_account_appearance() $$;

CREATE FUNCTION ple_api.update_current_account_display_mode_preference(p_display_mode_preference text)
RETURNS text LANGUAGE sql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private
AS $$ SELECT ple_private.update_current_authenticated_account_display_mode_preference(
    p_display_mode_preference
) $$;

CREATE FUNCTION ple_api.update_current_instructor_personal_theme(p_theme text)
RETURNS text LANGUAGE sql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private
AS $$ SELECT ple_private.update_current_authenticated_instructor_personal_theme(p_theme) $$;

CREATE FUNCTION ple_api.list_instructor_accounts()
RETURNS TABLE (account_id text, state text, last_successful_sign_in timestamp with time zone)
LANGUAGE sql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private
AS $$ SELECT * FROM ple_private.list_instructor_accounts() $$;

CREATE FUNCTION ple_api.create_instructor_account(
    p_normalized_email text, p_first_name text, p_last_name text, p_affiliation text
)
RETURNS TABLE (account_id text, state text, last_successful_sign_in timestamp with time zone)
LANGUAGE sql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private
AS $$ SELECT * FROM ple_private.create_instructor_account_summary(
    p_normalized_email, p_first_name, p_last_name, p_affiliation
) $$;

CREATE FUNCTION ple_api.instructor_setup_email_destination(p_account_id text)
RETURNS text LANGUAGE sql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private
AS $$ SELECT ple_private.instructor_setup_email_destination(p_account_id) $$;

CREATE FUNCTION ple_api.change_instructor_account_state(
    p_account_id text, p_next_state text, p_reason text
)
RETURNS TABLE (account_id text, state text, last_successful_sign_in timestamp with time zone)
LANGUAGE sql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private
AS $$ SELECT * FROM ple_private.change_instructor_account_state(
    p_account_id, p_next_state, p_reason
) $$;
