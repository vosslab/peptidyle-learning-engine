-- M16 Sysadmin Instructor Account management.
--
-- Browser-safe account management intentionally projects only an opaque Account
-- Reference, Account State, and Last Successful Sign-In. Authentication Email,
-- credentials, Student Records, Courses, and audit evidence remain private.

SET LOCAL ROLE ple_private_owner;

-- The state-transition definer locks the exact Instructor Account before it
-- appends an Account State Event.  Forced RLS therefore needs this narrow
-- lock policy in addition to the existing private-owner lookup policy.
CREATE POLICY account_private_owner_live_demo_instructor_lock
    ON ple_private.account FOR UPDATE TO ple_private_owner
    USING (product_role = 'instructor')
    WITH CHECK (product_role = 'instructor');

-- ASVS 8.2.1 and 8.3.1: every operation checks the exact active Sysadmin
-- Account installed by the authenticated session before it reads or changes an
-- Instructor Account. Product Role stays immutable and never enters an input.
CREATE FUNCTION ple_private.require_live_demo_sysadmin_account()
RETURNS uuid
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private
AS $$
DECLARE
    v_actor_account_id uuid;
BEGIN
    v_actor_account_id := ple_api.current_session_account_id();
    -- ASVS 8.2.1: resolve the exact active Sysadmin relationship directly in
    -- this private definer. It does not depend on a separately granted API
    -- predicate and it never gives ple_app table or predicate access.
    IF v_actor_account_id IS NULL OR NOT EXISTS (
        SELECT 1
          FROM ple_private.account AS account
          JOIN LATERAL (
              SELECT event.state
                FROM ple_private.account_state_event AS event
               WHERE event.account_id = account.account_id
               ORDER BY event.occurred_at DESC, event.event_id DESC
               LIMIT 1
          ) AS current_state ON current_state.state = 'active'
         WHERE account.account_id = v_actor_account_id
           AND account.product_role = 'sysadmin'
    ) THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Active Sysadmin Account required';
    END IF;
    RETURN v_actor_account_id;
END
$$;

CREATE FUNCTION ple_private.live_demo_instructor_account_summary(p_account_id uuid)
RETURNS TABLE (
    reference_number bigint,
    state text,
    last_successful_sign_in timestamp with time zone
)
LANGUAGE sql SECURITY DEFINER
SET search_path = pg_catalog, ple_private
AS $$
    SELECT account.reference_number,
           current_state.state,
           (
               SELECT max(session.created_at)
                 FROM ple_private.authenticated_session AS session
                WHERE session.account_id = account.account_id
           )
      FROM ple_private.account AS account
      JOIN LATERAL (
          SELECT event.state
            FROM ple_private.account_state_event AS event
           WHERE event.account_id = account.account_id
           ORDER BY event.occurred_at DESC, event.event_id DESC
           LIMIT 1
      ) AS current_state ON true
     WHERE account.account_id = p_account_id
       AND account.product_role = 'instructor'
$$;

CREATE FUNCTION ple_private.list_live_demo_instructor_accounts()
RETURNS TABLE (
    reference_number bigint,
    state text,
    last_successful_sign_in timestamp with time zone
)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_private
AS $$
BEGIN
    PERFORM ple_private.require_live_demo_sysadmin_account();
    RETURN QUERY
    SELECT summary.reference_number, summary.state, summary.last_successful_sign_in
      FROM ple_private.account AS account
      CROSS JOIN LATERAL ple_private.live_demo_instructor_account_summary(account.account_id)
          AS summary
     WHERE account.product_role = 'instructor'
     ORDER BY summary.reference_number;
END
$$;

-- The established creator remains the sole Account/email/audit writer. This
-- wrapper supplies its existing normalized-email contract without exposing any
-- Authentication Email in the result. ASVS 8.2.1, 8.2.2, and 16.2.5.
CREATE FUNCTION ple_private.create_live_demo_instructor_account(p_normalized_email text)
RETURNS TABLE (
    reference_number bigint,
    state text,
    last_successful_sign_in timestamp with time zone
)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_private
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

-- ASVS 8.2.2: transition only an existing Instructor Account from Active to
-- Deactivated or Deactivated to Active. The append-only Account State Event
-- trigger retains current-session revocation for a deactivation.
CREATE FUNCTION ple_private.change_live_demo_instructor_account_state(
    p_reference_number bigint,
    p_next_state text,
    p_reason text
)
RETURNS TABLE (
    reference_number bigint,
    state text,
    last_successful_sign_in timestamp with time zone
)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_private
AS $$
DECLARE
    v_account_id uuid;
    v_current_state text;
    v_occurred_at timestamp with time zone;
BEGIN
    PERFORM ple_private.require_live_demo_sysadmin_account();
    IF p_reference_number NOT BETWEEN 1 AND 2147483647
       OR p_next_state NOT IN ('active', 'deactivated')
       OR (p_next_state = 'deactivated' AND (
           p_reason IS NULL OR p_reason <> btrim(p_reason)
           OR char_length(p_reason) NOT BETWEEN 1 AND 1000
       ))
       OR (p_next_state = 'active' AND p_reason IS NOT NULL) THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Instructor Account State arguments are invalid';
    END IF;

    SELECT account.account_id
      INTO v_account_id
      FROM ple_private.account AS account
     WHERE account.reference_number = p_reference_number
       AND account.product_role = 'instructor'
     FOR UPDATE;
    IF NOT FOUND THEN
        RETURN;
    END IF;
    SELECT event.state
      INTO v_current_state
      FROM ple_private.account_state_event AS event
     WHERE event.account_id = v_account_id
     ORDER BY event.occurred_at DESC, event.event_id DESC
     LIMIT 1;
    IF (p_next_state = 'deactivated' AND v_current_state <> 'active')
       OR (p_next_state = 'active' AND v_current_state <> 'deactivated') THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Instructor Account State transition is invalid';
    END IF;

    v_occurred_at := pg_catalog.transaction_timestamp();
    INSERT INTO ple_private.account_state_event (
        event_id, account_id, state, occurred_at, reason
    ) VALUES (
        pg_catalog.gen_random_uuid(), v_account_id, p_next_state, v_occurred_at, p_reason
    );
    RETURN QUERY
    SELECT summary.reference_number, summary.state, summary.last_successful_sign_in
      FROM ple_private.live_demo_instructor_account_summary(v_account_id) AS summary;
END
$$;

REVOKE ALL PRIVILEGES ON FUNCTION ple_private.require_live_demo_sysadmin_account() FROM PUBLIC;
REVOKE ALL PRIVILEGES ON FUNCTION ple_private.live_demo_instructor_account_summary(uuid) FROM PUBLIC;
REVOKE ALL PRIVILEGES ON FUNCTION ple_private.list_live_demo_instructor_accounts() FROM PUBLIC;
REVOKE ALL PRIVILEGES ON FUNCTION ple_private.create_live_demo_instructor_account(text) FROM PUBLIC;
REVOKE ALL PRIVILEGES ON FUNCTION ple_private.change_live_demo_instructor_account_state(
    bigint, text, text
) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_private.list_live_demo_instructor_accounts() TO ple_api_owner;
GRANT EXECUTE ON FUNCTION ple_private.create_live_demo_instructor_account(text) TO ple_api_owner;
GRANT EXECUTE ON FUNCTION ple_private.change_live_demo_instructor_account_state(
    bigint, text, text
) TO ple_api_owner;
RESET ROLE;

SET LOCAL ROLE ple_api_owner;

-- The M8 creation picker was the only consumer of its earlier generic
-- Sysadmin predicate. Keep the exact active-account check at that operation
-- and remove the globally named predicate, so account management does not
-- preserve an unnecessary authority surface.
CREATE OR REPLACE FUNCTION ple_api.list_live_demo_course_creation_instructors()
RETURNS TABLE (reference_number bigint)
LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private
AS $$
    SELECT account.reference_number
      FROM ple_private.account AS account
      JOIN LATERAL (
          SELECT event.state
            FROM ple_private.account_state_event AS event
           WHERE event.account_id = account.account_id
           ORDER BY event.occurred_at DESC, event.event_id DESC
           LIMIT 1
      ) AS current_state ON current_state.state = 'active'
     WHERE EXISTS (
         SELECT 1
           FROM ple_private.account AS actor
           JOIN LATERAL (
               SELECT event.state
                 FROM ple_private.account_state_event AS event
                WHERE event.account_id = actor.account_id
                ORDER BY event.occurred_at DESC, event.event_id DESC
                LIMIT 1
           ) AS actor_state ON actor_state.state = 'active'
          WHERE actor.account_id = ple_api.current_session_account_id()
            AND actor.product_role = 'sysadmin'
     )
       AND account.product_role = 'instructor'
     ORDER BY account.reference_number
$$;
DROP FUNCTION ple_api.current_session_account_is_sysadmin();

CREATE FUNCTION ple_api.list_live_demo_instructor_accounts()
RETURNS TABLE (
    reference_number bigint,
    state text,
    last_successful_sign_in timestamp with time zone
)
LANGUAGE sql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private
AS $$
    SELECT * FROM ple_private.list_live_demo_instructor_accounts()
$$;

CREATE FUNCTION ple_api.create_live_demo_instructor_account(p_normalized_email text)
RETURNS TABLE (
    reference_number bigint,
    state text,
    last_successful_sign_in timestamp with time zone
)
LANGUAGE sql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private
AS $$
    SELECT * FROM ple_private.create_live_demo_instructor_account(p_normalized_email)
$$;

CREATE FUNCTION ple_api.change_live_demo_instructor_account_state(
    p_reference_number bigint,
    p_next_state text,
    p_reason text
)
RETURNS TABLE (
    reference_number bigint,
    state text,
    last_successful_sign_in timestamp with time zone
)
LANGUAGE sql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private
AS $$
    SELECT * FROM ple_private.change_live_demo_instructor_account_state(
        p_reference_number, p_next_state, p_reason
    )
$$;

REVOKE ALL PRIVILEGES ON FUNCTION ple_api.list_live_demo_instructor_accounts() FROM PUBLIC;
REVOKE ALL PRIVILEGES ON FUNCTION ple_api.create_live_demo_instructor_account(text) FROM PUBLIC;
REVOKE ALL PRIVILEGES ON FUNCTION ple_api.change_live_demo_instructor_account_state(
    bigint, text, text
) FROM PUBLIC;
GRANT USAGE ON SCHEMA ple_api TO ple_app;
GRANT EXECUTE ON FUNCTION ple_api.list_live_demo_instructor_accounts() TO ple_app;
GRANT EXECUTE ON FUNCTION ple_api.create_live_demo_instructor_account(text) TO ple_app;
GRANT EXECUTE ON FUNCTION ple_api.change_live_demo_instructor_account_state(
    bigint, text, text
) TO ple_app;
RESET ROLE;
