-- Global account identity, current state, profile preference, and account audit.
-- Credential material and sessions are owned by authentication.sql.

SET LOCAL ROLE ple_private_owner;

CREATE TABLE ple_private.account (
    account_id uuid PRIMARY KEY,
    reference_number bigint GENERATED ALWAYS AS IDENTITY UNIQUE,
    product_role text NOT NULL CHECK (product_role IN ('student', 'instructor', 'sysadmin')),
    created_at timestamp with time zone NOT NULL,
    CONSTRAINT account_product_role_is_unique UNIQUE (account_id, product_role)
);

CREATE TABLE ple_private.account_state_event (
    event_id uuid PRIMARY KEY,
    account_id uuid NOT NULL REFERENCES ple_private.account (account_id),
    state text NOT NULL CHECK (state IN ('active', 'deactivated', 'closed')),
    occurred_at timestamp with time zone NOT NULL,
    reason text,
    CONSTRAINT account_state_event_reason_is_present_for_nonactive_state CHECK (
        state = 'active' OR char_length(btrim(reason)) BETWEEN 1 AND 1000
    )
);

CREATE FUNCTION ple_private.reject_account_identity_change()
RETURNS trigger LANGUAGE plpgsql
SET search_path = pg_catalog, ple_private
AS $$
BEGIN
    IF NEW.account_id IS DISTINCT FROM OLD.account_id
       OR NEW.reference_number IS DISTINCT FROM OLD.reference_number
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

CREATE FUNCTION ple_private.account_time_zone_is_exact_iana(p_time_zone text)
RETURNS boolean LANGUAGE sql STABLE
SET search_path = pg_catalog
AS $$
    SELECT p_time_zone IS NOT NULL
       AND p_time_zone = btrim(p_time_zone)
       AND char_length(p_time_zone) BETWEEN 1 AND 100
       AND EXISTS (
           SELECT 1 FROM pg_catalog.pg_timezone_names AS zone
            WHERE zone.name = p_time_zone
       )
$$;

CREATE TABLE ple_private.account_time_zone (
    account_id uuid PRIMARY KEY REFERENCES ple_private.account (account_id),
    time_zone text NOT NULL CHECK (ple_private.account_time_zone_is_exact_iana(time_zone))
);

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
    INSERT INTO ple_private.account_time_zone (account_id, time_zone)
    VALUES (NEW.account_id, 'America/Chicago');
    RETURN NEW;
END
$$;

CREATE TRIGGER account_time_zone_is_exact_iana
BEFORE INSERT OR UPDATE OF time_zone ON ple_private.account_time_zone
FOR EACH ROW EXECUTE FUNCTION ple_private.reject_invalid_account_time_zone();

CREATE TRIGGER account_creation_records_default_time_zone
AFTER INSERT ON ple_private.account
FOR EACH ROW EXECUTE FUNCTION ple_private.record_default_account_time_zone();

ALTER TABLE ple_private.account ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.account FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_private.account_state_event ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.account_state_event FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_private.account_time_zone ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.account_time_zone FORCE ROW LEVEL SECURITY;

CREATE POLICY account_private_owner_access ON ple_private.account
    FOR ALL TO ple_private_owner USING (true) WITH CHECK (true);
CREATE POLICY account_state_event_private_owner_access ON ple_private.account_state_event
    FOR ALL TO ple_private_owner USING (true) WITH CHECK (true);
CREATE POLICY account_time_zone_private_owner_access ON ple_private.account_time_zone
    FOR ALL TO ple_private_owner USING (true) WITH CHECK (true);
CREATE POLICY account_api_owner_access ON ple_private.account
    FOR SELECT TO ple_api_owner USING (true);
CREATE POLICY account_state_event_api_owner_access ON ple_private.account_state_event
    FOR SELECT TO ple_api_owner USING (true);

REVOKE ALL PRIVILEGES ON TABLE ple_private.account, ple_private.account_state_event,
    ple_private.account_time_zone FROM PUBLIC;
REVOKE ALL PRIVILEGES ON FUNCTION ple_private.reject_account_identity_change(),
    ple_private.record_initial_account_state(), ple_private.account_time_zone_is_exact_iana(text),
    ple_private.reject_invalid_account_time_zone(), ple_private.record_default_account_time_zone()
    FROM PUBLIC;
GRANT USAGE ON SCHEMA ple_private TO ple_api_owner;
GRANT SELECT ON ple_private.account, ple_private.account_state_event TO ple_api_owner;
GRANT USAGE ON SCHEMA ple_private TO ple_data_owner;
GRANT REFERENCES ON TABLE ple_private.account TO ple_data_owner;
GRANT USAGE ON SCHEMA ple_private TO ple_audit_owner;
GRANT REFERENCES ON TABLE ple_private.account TO ple_audit_owner;

RESET ROLE;

SET LOCAL ROLE ple_audit_owner;

CREATE TABLE ple_audit.instructor_account_creation_event (
    event_id uuid PRIMARY KEY,
    created_instructor_account_id uuid NOT NULL,
    created_instructor_product_role text NOT NULL DEFAULT 'instructor'
        CHECK (created_instructor_product_role = 'instructor'),
    created_by_sysadmin_account_id uuid NOT NULL,
    created_by_sysadmin_product_role text NOT NULL DEFAULT 'sysadmin'
        CHECK (created_by_sysadmin_product_role = 'sysadmin'),
    occurred_at timestamp with time zone NOT NULL,
    UNIQUE (created_instructor_account_id),
    FOREIGN KEY (created_instructor_account_id, created_instructor_product_role)
        REFERENCES ple_private.account (account_id, product_role),
    FOREIGN KEY (created_by_sysadmin_account_id, created_by_sysadmin_product_role)
        REFERENCES ple_private.account (account_id, product_role)
);

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
    p_created_instructor_account_id uuid,
    p_created_by_sysadmin_account_id uuid
)
RETURNS void LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_audit
AS $$
BEGIN
    IF p_created_instructor_account_id IS NULL OR p_created_by_sysadmin_account_id IS NULL THEN
        RAISE EXCEPTION USING ERRCODE = '22004',
            MESSAGE = 'Instructor Account creation evidence requires subject and actor';
    END IF;
    INSERT INTO ple_audit.instructor_account_creation_event (
        event_id, created_instructor_account_id, created_by_sysadmin_account_id, occurred_at
    ) VALUES (
        pg_catalog.gen_random_uuid(), p_created_instructor_account_id,
        p_created_by_sysadmin_account_id, pg_catalog.transaction_timestamp()
    );
END
$$;

ALTER TABLE ple_audit.instructor_account_creation_event ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_audit.instructor_account_creation_event FORCE ROW LEVEL SECURITY;
CREATE POLICY instructor_account_creation_event_audit_owner_insert
    ON ple_audit.instructor_account_creation_event FOR INSERT TO ple_audit_owner WITH CHECK (true);
REVOKE ALL PRIVILEGES ON TABLE ple_audit.instructor_account_creation_event FROM PUBLIC;
REVOKE ALL PRIVILEGES ON FUNCTION ple_audit.reject_instructor_account_creation_event_change(),
    ple_audit.record_instructor_account_creation_event(uuid, uuid) FROM PUBLIC;
GRANT USAGE ON SCHEMA ple_audit TO ple_private_owner;
GRANT EXECUTE ON FUNCTION ple_audit.record_instructor_account_creation_event(uuid, uuid)
    TO ple_private_owner;

RESET ROLE;

SET LOCAL ROLE ple_private_owner;

CREATE FUNCTION ple_private.require_current_sysadmin_account()
RETURNS uuid LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private
AS $$
DECLARE v_account_id uuid;
BEGIN
    v_account_id := ple_api.current_session_account_id();
    IF v_account_id IS NULL OR NOT EXISTS (
        SELECT 1 FROM ple_private.account AS account
        JOIN LATERAL (
            SELECT state_event.state FROM ple_private.account_state_event AS state_event
             WHERE state_event.account_id = account.account_id
             ORDER BY state_event.occurred_at DESC, state_event.event_id DESC LIMIT 1
        ) AS current_state ON current_state.state = 'active'
        WHERE account.account_id = v_account_id AND account.product_role = 'sysadmin'
    ) THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Active Sysadmin Account required';
    END IF;
    RETURN v_account_id;
END
$$;

CREATE FUNCTION ple_private.current_authenticated_account_time_zone()
RETURNS text LANGUAGE plpgsql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private
AS $$
BEGIN
    RETURN (
        SELECT preference.time_zone FROM ple_private.account_time_zone AS preference
        JOIN ple_private.account AS account ON account.account_id = preference.account_id
        WHERE account.account_id = ple_api.current_session_account_id()
    );
END
$$;

CREATE FUNCTION ple_private.resolve_or_create_student_account(
    p_normalized_email text, p_delivery_email text
)
RETURNS uuid LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_private
AS $$
DECLARE v_account_id uuid; v_now timestamptz := pg_catalog.transaction_timestamp();
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
    v_account_id := pg_catalog.gen_random_uuid();
    INSERT INTO ple_private.account (account_id, product_role, created_at)
    VALUES (v_account_id, 'student', v_now);
    INSERT INTO ple_private.account_authentication_email (
        account_id, normalized_email, delivery_email, verified_at, updated_at
    ) VALUES (v_account_id, p_normalized_email, p_delivery_email, v_now, v_now);
    RETURN v_account_id;
END
$$;

CREATE FUNCTION ple_private.create_instructor_account(p_normalized_email text, p_delivery_email text)
RETURNS TABLE (account_id uuid, created_at timestamp with time zone)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_audit, ple_private
AS $$
DECLARE v_account_id uuid; v_actor_account_id uuid; v_created_at timestamptz;
BEGIN
    IF p_normalized_email IS NULL
       OR char_length(p_normalized_email) NOT BETWEEN 3 AND 320
       OR p_normalized_email IS DISTINCT FROM lower(btrim(p_normalized_email))
       OR p_delivery_email IS NULL OR char_length(btrim(p_delivery_email)) NOT BETWEEN 3 AND 320 THEN
        RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Invalid Instructor Account input';
    END IF;
    v_actor_account_id := ple_private.require_current_sysadmin_account();
    v_account_id := pg_catalog.gen_random_uuid();
    v_created_at := pg_catalog.transaction_timestamp();
    INSERT INTO ple_private.account (account_id, product_role, created_at)
    VALUES (v_account_id, 'instructor', v_created_at);
    INSERT INTO ple_private.account_authentication_email (
        account_id, normalized_email, delivery_email, verified_at, updated_at
    ) VALUES (v_account_id, p_normalized_email, p_delivery_email, v_created_at, v_created_at);
    PERFORM ple_audit.record_instructor_account_creation_event(v_account_id, v_actor_account_id);
    RETURN QUERY SELECT v_account_id, v_created_at;
END
$$;

CREATE FUNCTION ple_private.instructor_account_summary(p_account_id uuid)
RETURNS TABLE (reference_number bigint, state text, last_successful_sign_in timestamp with time zone)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_private
AS $$
BEGIN
    RETURN QUERY
    SELECT account.reference_number, current_state.state,
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
RETURNS TABLE (reference_number bigint, state text, last_successful_sign_in timestamp with time zone)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_private
AS $$
BEGIN
    PERFORM ple_private.require_current_sysadmin_account();
    RETURN QUERY
    SELECT summary.reference_number, summary.state, summary.last_successful_sign_in
    FROM ple_private.account AS account
    CROSS JOIN LATERAL ple_private.instructor_account_summary(account.account_id) AS summary
    WHERE account.product_role = 'instructor' ORDER BY summary.reference_number;
END
$$;

CREATE FUNCTION ple_private.create_instructor_account_summary(p_normalized_email text)
RETURNS TABLE (reference_number bigint, state text, last_successful_sign_in timestamp with time zone)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_private
AS $$
DECLARE v_account_id uuid;
BEGIN
    SELECT created.account_id INTO v_account_id
    FROM ple_private.create_instructor_account(p_normalized_email, p_normalized_email) AS created;
    RETURN QUERY SELECT * FROM ple_private.instructor_account_summary(v_account_id);
END
$$;

CREATE FUNCTION ple_private.change_instructor_account_state(
    p_reference_number bigint, p_next_state text, p_reason text
)
RETURNS TABLE (reference_number bigint, state text, last_successful_sign_in timestamp with time zone)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_private
AS $$
DECLARE v_account_id uuid; v_current_state text;
BEGIN
    PERFORM ple_private.require_current_sysadmin_account();
    IF p_next_state NOT IN ('active', 'deactivated') OR p_reference_number IS NULL THEN
        RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Instructor Account state input is invalid';
    END IF;
    SELECT account.account_id, state_event.state INTO v_account_id, v_current_state
    FROM ple_private.account AS account
    JOIN LATERAL (
        SELECT event.state FROM ple_private.account_state_event AS event
         WHERE event.account_id = account.account_id
         ORDER BY event.occurred_at DESC, event.event_id DESC LIMIT 1
    ) AS state_event ON true
    WHERE account.reference_number = p_reference_number AND account.product_role = 'instructor'
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

REVOKE ALL PRIVILEGES ON FUNCTION ple_private.require_current_sysadmin_account(),
    ple_private.current_authenticated_account_time_zone(),
    ple_private.resolve_or_create_student_account(text, text),
    ple_private.create_instructor_account(text, text),
    ple_private.instructor_account_summary(uuid),
    ple_private.list_instructor_accounts(),
    ple_private.create_instructor_account_summary(text),
    ple_private.change_instructor_account_state(bigint, text, text) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_private.create_instructor_account(text, text),
    ple_private.create_instructor_account_summary(text),
    ple_private.list_instructor_accounts(),
    ple_private.change_instructor_account_state(bigint, text, text),
    ple_private.current_authenticated_account_time_zone(),
    ple_private.resolve_or_create_student_account(text, text) TO ple_api_owner;

RESET ROLE;

SET LOCAL ROLE ple_api_owner;

CREATE FUNCTION ple_api.current_account_time_zone()
RETURNS text LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private
AS $$ SELECT ple_private.current_authenticated_account_time_zone() $$;

CREATE FUNCTION ple_api.list_instructor_accounts()
RETURNS TABLE (reference_number bigint, state text, last_successful_sign_in timestamp with time zone)
LANGUAGE sql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private
AS $$ SELECT * FROM ple_private.list_instructor_accounts() $$;

CREATE FUNCTION ple_api.create_instructor_account(p_normalized_email text)
RETURNS TABLE (reference_number bigint, state text, last_successful_sign_in timestamp with time zone)
LANGUAGE sql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private
AS $$ SELECT * FROM ple_private.create_instructor_account_summary(p_normalized_email) $$;

CREATE FUNCTION ple_api.change_instructor_account_state(
    p_reference_number bigint, p_next_state text, p_reason text
)
RETURNS TABLE (reference_number bigint, state text, last_successful_sign_in timestamp with time zone)
LANGUAGE sql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private
AS $$ SELECT * FROM ple_private.change_instructor_account_state(
    p_reference_number, p_next_state, p_reason
) $$;

CREATE FUNCTION ple_api.read_instructor_profile()
RETURNS text LANGUAGE plpgsql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private
AS $$
DECLARE v_account_id uuid;
BEGIN
    v_account_id := ple_api.current_session_account_id();
    RETURN (
        SELECT preference.time_zone FROM ple_private.account_time_zone AS preference
        JOIN ple_private.account AS account ON account.account_id = preference.account_id
        JOIN LATERAL (
            SELECT state_event.state FROM ple_private.account_state_event AS state_event
             WHERE state_event.account_id = account.account_id
             ORDER BY state_event.occurred_at DESC, state_event.event_id DESC LIMIT 1
        ) AS current_state ON current_state.state = 'active'
        WHERE account.account_id = v_account_id AND account.product_role = 'instructor'
    );
END
$$;

CREATE FUNCTION ple_api.update_instructor_profile(p_time_zone text)
RETURNS text LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private
AS $$
DECLARE v_account_id uuid;
BEGIN
    IF NOT ple_private.account_time_zone_is_exact_iana(p_time_zone) THEN
        RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Instructor Profile time zone is invalid';
    END IF;
    v_account_id := ple_api.current_session_account_id();
    IF NOT EXISTS (
        SELECT 1 FROM ple_private.account AS account
        JOIN LATERAL (
            SELECT state_event.state FROM ple_private.account_state_event AS state_event
             WHERE state_event.account_id = account.account_id
             ORDER BY state_event.occurred_at DESC, state_event.event_id DESC LIMIT 1
        ) AS current_state ON current_state.state = 'active'
        WHERE account.account_id = v_account_id AND account.product_role = 'instructor'
    ) THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Active Instructor Account required';
    END IF;
    UPDATE ple_private.account_time_zone SET time_zone = p_time_zone WHERE account_id = v_account_id
    RETURNING time_zone INTO p_time_zone;
    RETURN p_time_zone;
END
$$;

REVOKE ALL PRIVILEGES ON FUNCTION ple_api.current_account_time_zone(),
    ple_api.list_instructor_accounts(), ple_api.create_instructor_account(text),
    ple_api.change_instructor_account_state(bigint, text, text),
    ple_api.read_instructor_profile(), ple_api.update_instructor_profile(text) FROM PUBLIC;
GRANT USAGE ON SCHEMA ple_api TO ple_app;
GRANT EXECUTE ON FUNCTION ple_api.current_account_time_zone(),
    ple_api.list_instructor_accounts(), ple_api.create_instructor_account(text),
    ple_api.change_instructor_account_state(bigint, text, text),
    ple_api.read_instructor_profile(), ple_api.update_instructor_profile(text) TO ple_app;

RESET ROLE;
