-- Functions, triggers, and views from accounts.sql.

SET LOCAL ROLE ple_private_owner;

CREATE FUNCTION ple_private.reject_account_identity_change()
RETURNS trigger LANGUAGE plpgsql
SET search_path = pg_catalog, ple_private
AS $$
BEGIN
    IF NEW.account_id IS DISTINCT FROM OLD.account_id
       OR NEW.product_role IS DISTINCT FROM OLD.product_role
       OR NEW.created_at IS DISTINCT FROM OLD.created_at THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Account identity and Product Role are immutable';
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

CREATE TRIGGER account_human_reference_is_minted
BEFORE INSERT ON ple_private.account
FOR EACH ROW EXECUTE FUNCTION ple_private.assign_human_reference('U');

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

CREATE FUNCTION ple_audit.reject_instructor_identity_vetting_decision_change()
RETURNS trigger LANGUAGE plpgsql
SET search_path = pg_catalog, ple_audit
AS $$
BEGIN
    RAISE EXCEPTION USING ERRCODE = '23514',
        MESSAGE = 'Instructor identity vetting decisions are immutable';
END
$$;

CREATE TRIGGER instructor_identity_vetting_decision_is_immutable
BEFORE UPDATE OR DELETE ON ple_audit.instructor_identity_vetting_decision
FOR EACH ROW EXECUTE FUNCTION ple_audit.reject_instructor_identity_vetting_decision_change();

CREATE FUNCTION ple_audit.record_instructor_account_creation_event(
    p_created_instructor_account_id text,
    p_created_by_sysadmin_account_id text,
    p_vetting_decision_id uuid
)
RETURNS void LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_audit
AS $$
BEGIN
    IF p_created_instructor_account_id IS NULL
       OR p_created_by_sysadmin_account_id IS NULL
       OR p_vetting_decision_id IS NULL THEN
        RAISE EXCEPTION USING ERRCODE = '22004',
            MESSAGE = 'Instructor Account creation evidence requires subject, actor, and vetting decision';
    END IF;
    INSERT INTO ple_audit.instructor_account_creation_event (
        event_id, created_instructor_account_id, created_by_sysadmin_account_id,
        instructor_identity_vetting_decision_id, occurred_at
    ) VALUES (
        pg_catalog.gen_random_uuid(), p_created_instructor_account_id,
        p_created_by_sysadmin_account_id, p_vetting_decision_id,
        pg_catalog.transaction_timestamp()
    ) ON CONFLICT (created_instructor_account_id) DO NOTHING;
    IF NOT FOUND AND NOT EXISTS (
        SELECT 1 FROM ple_audit.instructor_account_creation_event
         WHERE created_instructor_account_id = p_created_instructor_account_id
           AND created_by_sysadmin_account_id = p_created_by_sysadmin_account_id
           AND instructor_identity_vetting_decision_id = p_vetting_decision_id
    ) THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Instructor Account creation evidence conflicts with immutable history';
    END IF;
END
$$;

CREATE FUNCTION ple_audit.record_completed_instructor_identity_vetting_decision(
    p_normalized_email text,
    p_verified_instructor_display_name text,
    p_completed_by_sysadmin_account_id text
)
RETURNS uuid LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_audit
AS $$
DECLARE v_decision_id uuid; v_existing_display_name text;
BEGIN
    IF p_normalized_email IS NULL
       OR char_length(p_normalized_email) NOT BETWEEN 3 AND 320
       OR p_normalized_email IS DISTINCT FROM lower(btrim(p_normalized_email))
       OR p_verified_instructor_display_name IS NULL
       OR p_verified_instructor_display_name IS DISTINCT FROM btrim(p_verified_instructor_display_name)
       OR char_length(p_verified_instructor_display_name) NOT BETWEEN 1 AND 200
       OR p_verified_instructor_display_name ~ '[[:cntrl:]]'
       OR p_completed_by_sysadmin_account_id IS NULL THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Instructor identity vetting decision input is invalid';
    END IF;

    -- ASVS 2.3.1 and 2.3.3: a concurrent replay returns the one immutable
    -- completed decision rather than writing a second approval fact.
    INSERT INTO ple_audit.instructor_identity_vetting_decision (
        decision_id, normalized_email, verified_instructor_display_name,
        completed_by_sysadmin_account_id, completed_at
    ) VALUES (
        pg_catalog.gen_random_uuid(), p_normalized_email, p_verified_instructor_display_name,
        p_completed_by_sysadmin_account_id, pg_catalog.transaction_timestamp()
    ) ON CONFLICT (normalized_email) DO NOTHING
    RETURNING decision_id INTO v_decision_id;

    IF v_decision_id IS NULL THEN
        SELECT decision_id, verified_instructor_display_name
          INTO v_decision_id, v_existing_display_name
        FROM ple_audit.instructor_identity_vetting_decision
        WHERE normalized_email = p_normalized_email;
        IF v_existing_display_name IS DISTINCT FROM p_verified_instructor_display_name THEN
            RAISE EXCEPTION USING ERRCODE = '23514',
                MESSAGE = 'Completed Instructor identity vetting display name is immutable';
        END IF;
    END IF;
    RETURN v_decision_id;
END
$$;

CREATE FUNCTION ple_audit.completed_instructor_identity_vetting_decision(
    p_decision_id uuid, p_normalized_email text
)
RETURNS boolean LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_audit
AS $$
    SELECT p_decision_id IS NOT NULL
       AND p_normalized_email IS NOT NULL
       AND EXISTS (
           SELECT 1 FROM ple_audit.instructor_identity_vetting_decision
           WHERE decision_id = p_decision_id AND normalized_email = p_normalized_email
       )
$$;



-- This is callable only by the server-owned internal wrapper below.  It does
-- not grant browser clients or ordinary application logins any audit read.
CREATE FUNCTION ple_audit.verified_instructor_display_name(p_instructor_account_id text)
RETURNS text LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_audit, ple_private
AS $$
    SELECT decision.verified_instructor_display_name
      FROM ple_audit.instructor_account_creation_event AS creation
      JOIN ple_audit.instructor_identity_vetting_decision AS decision
        ON decision.decision_id = creation.instructor_identity_vetting_decision_id
     WHERE creation.created_instructor_account_id = p_instructor_account_id
       AND creation.created_instructor_product_role = 'instructor'
$$;

SET LOCAL ROLE ple_private_owner;

CREATE FUNCTION ple_private.require_current_sysadmin_account()
RETURNS uuid LANGUAGE plpgsql SECURITY DEFINER
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
       AND instructor.product_role = 'instructor'
      JOIN ple_private.account AS student
        ON student.account_id = p_student_account_id AND student.product_role = 'student'
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
       OR p_delivery_email IS NULL OR char_length(btrim(p_delivery_email)) NOT BETWEEN 3 AND 320 THEN
        RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Student Account input is invalid';
    END IF;
    SELECT account.account_id INTO v_account_id
    FROM ple_private.account_authentication_email AS email
    JOIN ple_private.account AS account ON account.account_id = email.account_id
    WHERE email.normalized_email = p_normalized_email AND account.product_role = 'student';
    IF FOUND THEN RETURN v_account_id; END IF;
    IF EXISTS (SELECT 1 FROM ple_private.account_authentication_email WHERE normalized_email = p_normalized_email) THEN
        RAISE EXCEPTION USING ERRCODE = '23505', MESSAGE = 'Student Authentication Email is unavailable';
    END IF;
    INSERT INTO ple_private.account (account_id, product_role, created_at)
    VALUES ('U00000009', 'student', v_now)
    RETURNING account_id INTO v_account_id;
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
    p_normalized_email text, p_delivery_email text, p_vetting_decision_id uuid
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
       OR p_delivery_email IS NULL OR char_length(btrim(p_delivery_email)) NOT BETWEEN 3 AND 320 THEN
        RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Invalid Instructor Account input';
    END IF;
    v_actor_account_id := ple_private.require_current_sysadmin_account();
    -- ASVS 2.2.1, 2.3.1, and 5.3.2: reject an absent, invalid, or
    -- mismatched decision before an Account or credential write can occur.
    PERFORM ple_private.require_completed_instructor_identity_vetting(
        p_vetting_decision_id, p_normalized_email
    );
    v_created_at := pg_catalog.transaction_timestamp();
    INSERT INTO ple_private.account (account_id, product_role, created_at)
    VALUES ('U00000009', 'instructor', v_created_at)
    RETURNING account_id INTO v_account_id;
    INSERT INTO ple_private.account_authentication_email (
        account_id, normalized_email, delivery_email, verified_at, updated_at
    ) VALUES (v_account_id, p_normalized_email, p_delivery_email, v_created_at, v_created_at);
    PERFORM ple_audit.record_instructor_account_creation_event(
        v_account_id, v_actor_account_id, p_vetting_decision_id
    );
    RETURN QUERY SELECT v_account_id, v_created_at;
END
$$;

CREATE FUNCTION ple_private.complete_instructor_identity_vetting(
    p_normalized_email text, p_verified_instructor_display_name text
)
RETURNS uuid LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_audit, ple_private
AS $$
DECLARE v_actor_account_id text;
BEGIN
    IF p_normalized_email IS NULL
       OR char_length(p_normalized_email) NOT BETWEEN 3 AND 320
       OR p_normalized_email IS DISTINCT FROM lower(btrim(p_normalized_email))
       OR p_verified_instructor_display_name IS NULL
       OR p_verified_instructor_display_name IS DISTINCT FROM btrim(p_verified_instructor_display_name)
       OR char_length(p_verified_instructor_display_name) NOT BETWEEN 1 AND 200
       OR p_verified_instructor_display_name ~ '[[:cntrl:]]' THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Instructor identity vetting input is invalid';
    END IF;
    -- ASVS 8.2.1 and 8.3.1: only the installed session determines the
    -- Sysadmin actor; no request field can select an approving role or Account.
    v_actor_account_id := ple_private.require_current_sysadmin_account();
    RETURN ple_audit.record_completed_instructor_identity_vetting_decision(
        p_normalized_email, p_verified_instructor_display_name, v_actor_account_id
    );
END
$$;



-- Internal-only source for the two later Star projections.  It is not exposed
-- through ple_api: their own authorized procedures must select it after they
-- establish a published item and active Instructor viewer.
CREATE FUNCTION ple_private.verified_instructor_display_name(p_instructor_account_id text)
RETURNS text LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_audit, ple_private
AS $$ SELECT ple_audit.verified_instructor_display_name(p_instructor_account_id) $$;

CREATE FUNCTION ple_private.require_completed_instructor_identity_vetting(
    p_decision_id uuid, p_normalized_email text
)
RETURNS void LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_audit, ple_private
AS $$
BEGIN
    IF NOT ple_audit.completed_instructor_identity_vetting_decision(
        p_decision_id, p_normalized_email
    ) THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Completed Instructor identity vetting decision is required';
    END IF;
END
$$;

CREATE FUNCTION ple_private.instructor_account_summary(p_account_id text)
RETURNS TABLE (public_reference text, state text, last_successful_sign_in timestamp with time zone)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_private
AS $$
BEGIN
    RETURN QUERY
    SELECT account.account_id, current_state.state,
           (SELECT max(session.created_at) FROM ple_private.authenticated_session AS session
             WHERE session.account_id = account.account_id)
      FROM ple_private.account AS account
      JOIN LATERAL (
          SELECT state_event.state FROM ple_private.account_state_event AS state_event
           WHERE state_event.account_id = account.account_id
           ORDER BY state_event.occurred_at DESC, state_event.event_id DESC LIMIT 1
      ) AS current_state ON true
     WHERE account.account_id = p_account_id AND account.product_role = 'instructor';
END
$$;

CREATE FUNCTION ple_private.list_instructor_accounts()
RETURNS TABLE (public_reference text, state text, last_successful_sign_in timestamp with time zone)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_private
AS $$
BEGIN
    PERFORM ple_private.require_current_sysadmin_account();
    RETURN QUERY
    SELECT summary.account_id, summary.state, summary.last_successful_sign_in
    FROM ple_private.account AS account
    CROSS JOIN LATERAL ple_private.instructor_account_summary(account.account_id) AS summary
    WHERE account.product_role = 'instructor' ORDER BY summary.account_id;
END
$$;

CREATE FUNCTION ple_private.create_instructor_account_summary(
    p_normalized_email text, p_vetting_decision_id uuid
)
RETURNS TABLE (public_reference text, state text, last_successful_sign_in timestamp with time zone)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_private
AS $$
DECLARE v_account_id text;
BEGIN
    SELECT created.account_id INTO v_account_id
    FROM ple_private.create_instructor_account(
        p_normalized_email, p_normalized_email, p_vetting_decision_id
    ) AS created;
    RETURN QUERY SELECT * FROM ple_private.instructor_account_summary(v_account_id);
END
$$;

CREATE FUNCTION ple_private.change_instructor_account_state(
    p_public_reference text, p_next_state text, p_reason text
)
RETURNS TABLE (public_reference text, state text, last_successful_sign_in timestamp with time zone)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_private
AS $$
DECLARE v_account_id text; v_current_state text;
BEGIN
    PERFORM ple_private.require_current_sysadmin_account();
    IF p_next_state NOT IN ('active', 'deactivated') OR p_public_reference IS NULL THEN
        RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Instructor Account state input is invalid';
    END IF;
    SELECT account.account_id, state_event.state INTO v_account_id, v_current_state
    FROM ple_private.account AS account
    JOIN LATERAL (
        SELECT event.state FROM ple_private.account_state_event AS event
         WHERE event.account_id = account.account_id
         ORDER BY event.occurred_at DESC, event.event_id DESC LIMIT 1
    ) AS state_event ON true
    WHERE account.account_id = p_public_reference AND account.product_role = 'instructor'
    FOR UPDATE OF account;
    IF NOT FOUND OR v_current_state = p_next_state THEN RETURN; END IF;
    IF p_next_state = 'deactivated' AND char_length(btrim(p_reason)) NOT BETWEEN 1 AND 1000 THEN
        RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Instructor deactivation requires a reason';
    END IF;
    INSERT INTO ple_private.account_state_event (event_id, account_id, state, occurred_at, reason)
    VALUES (pg_catalog.gen_random_uuid(), v_account_id, p_next_state,
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
                      AND account.product_role = 'student'
               ) THEN false
               ELSE student_invitation_default_pending
           END
     WHERE account_id = v_account_id
    RETURNING time_zone INTO p_time_zone;
    RETURN p_time_zone;
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

CREATE FUNCTION ple_api.list_instructor_accounts()
RETURNS TABLE (public_reference text, state text, last_successful_sign_in timestamp with time zone)
LANGUAGE sql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private
AS $$ SELECT * FROM ple_private.list_instructor_accounts() $$;

CREATE FUNCTION ple_api.create_instructor_account(
    p_normalized_email text, p_vetting_decision_id uuid
)
RETURNS TABLE (public_reference text, state text, last_successful_sign_in timestamp with time zone)
LANGUAGE sql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private
AS $$ SELECT * FROM ple_private.create_instructor_account_summary(
    p_normalized_email, p_vetting_decision_id
) $$;

CREATE FUNCTION ple_api.complete_instructor_identity_vetting(
    p_normalized_email text, p_verified_instructor_display_name text
)
RETURNS uuid LANGUAGE sql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private
AS $$ SELECT ple_private.complete_instructor_identity_vetting(
    p_normalized_email, p_verified_instructor_display_name
) $$;

CREATE FUNCTION ple_api.change_instructor_account_state(
    p_public_reference text, p_next_state text, p_reason text
)
RETURNS TABLE (public_reference text, state text, last_successful_sign_in timestamp with time zone)
LANGUAGE sql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private
AS $$ SELECT * FROM ple_private.change_instructor_account_state(
    p_public_reference, p_next_state, p_reason
) $$;

