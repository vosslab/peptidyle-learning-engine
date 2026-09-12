-- Session-derived account and authoring predicates. Course-specific
-- authorization follows the Course roots in course_operations.sql.

SET LOCAL ROLE ple_api_owner;

CREATE FUNCTION ple_api.current_session_account_id()
RETURNS uuid LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private
AS $$
    WITH configured AS (
        SELECT pg_catalog.current_setting('ple.session_account_id', true) AS raw_account_id
    ), parsed AS (
        SELECT CASE
            WHEN raw_account_id ~ '^[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}$'
            THEN raw_account_id::uuid
        END AS account_id
        FROM configured
    )
    SELECT account.account_id
      FROM parsed
      JOIN ple_private.account AS account
        ON account.account_id = parsed.account_id
$$;

CREATE FUNCTION ple_api.current_session_account_has_active_role(p_role text)
RETURNS boolean LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private
AS $$
    SELECT EXISTS (
        SELECT 1
          FROM ple_private.account AS account
          JOIN LATERAL (
              SELECT event.state
                FROM ple_private.account_state_event AS event
               WHERE event.account_id = account.account_id
               ORDER BY event.occurred_at DESC, event.event_id DESC
               LIMIT 1
          ) AS state_event ON state_event.state = 'active'
         WHERE account.account_id = ple_api.current_session_account_id()
           AND account.product_role = p_role
    )
$$;

CREATE FUNCTION ple_api.current_session_account_is_instructor()
RETURNS boolean LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api
AS $$ SELECT ple_api.current_session_account_has_active_role('instructor') $$;

CREATE FUNCTION ple_api.current_session_account_is_sysadmin()
RETURNS boolean LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api
AS $$ SELECT ple_api.current_session_account_has_active_role('sysadmin') $$;

CREATE FUNCTION ple_api.current_session_account_is_course_instructor(p_course_id uuid)
RETURNS boolean LANGUAGE plpgsql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data
AS $$
DECLARE allowed boolean;
BEGIN
    IF p_course_id IS NULL THEN RETURN false; END IF;
    EXECUTE
        'SELECT EXISTS (
            SELECT 1 FROM ple_data.course_membership AS membership
             WHERE membership.course_id = $1
               AND membership.account_id = ple_api.current_session_account_id()
               AND membership.role = ''instructor''
               AND ple_data.course_membership_is_active(membership.membership_id)
         )'
    INTO allowed USING p_course_id;
    RETURN allowed;
END
$$;

CREATE FUNCTION ple_api.current_session_account_is_course_member(p_course_id uuid)
RETURNS boolean LANGUAGE plpgsql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data
AS $$
DECLARE allowed boolean;
BEGIN
    IF p_course_id IS NULL THEN RETURN false; END IF;
    EXECUTE
        'SELECT EXISTS (
            SELECT 1 FROM ple_data.course_membership AS membership
             WHERE membership.course_id = $1
               AND membership.account_id = ple_api.current_session_account_id()
               AND ple_data.course_membership_is_active(membership.membership_id)
         )'
    INTO allowed USING p_course_id;
    RETURN allowed;
END
$$;

CREATE FUNCTION ple_api.current_session_account_owns_course_membership(
    p_course_id uuid, p_membership_id uuid
)
RETURNS boolean LANGUAGE plpgsql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data
AS $$
DECLARE allowed boolean;
BEGIN
    IF p_course_id IS NULL OR p_membership_id IS NULL THEN RETURN false; END IF;
    EXECUTE
        'SELECT EXISTS (
            SELECT 1 FROM ple_data.course_membership AS membership
             WHERE membership.course_id = $1 AND membership.membership_id = $2
               AND membership.account_id = ple_api.current_session_account_id()
               AND ple_data.course_membership_is_active(membership.membership_id)
         )'
    INTO allowed USING p_course_id, p_membership_id;
    RETURN allowed;
END
$$;

CREATE FUNCTION ple_api.current_session_account_owns_student_record(
    p_course_id uuid, p_student_record_id uuid
)
RETURNS boolean LANGUAGE plpgsql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data
AS $$
DECLARE allowed boolean;
BEGIN
    IF p_course_id IS NULL OR p_student_record_id IS NULL THEN RETURN false; END IF;
    EXECUTE
        'SELECT EXISTS (
            SELECT 1
              FROM ple_data.student_record AS student
              JOIN ple_data.course_membership AS membership
                ON membership.student_record_id = student.student_record_id
             WHERE student.course_id = $1 AND student.student_record_id = $2
               AND student.student_account_id = ple_api.current_session_account_id()
               AND membership.account_id = student.student_account_id
               AND membership.role = ''student''
               AND ple_data.course_membership_is_active(membership.membership_id)
         )'
    INTO allowed USING p_course_id, p_student_record_id;
    RETURN allowed;
END
$$;

REVOKE ALL PRIVILEGES ON FUNCTION ple_api.current_session_account_id(),
    ple_api.current_session_account_has_active_role(text),
    ple_api.current_session_account_is_instructor(),
    ple_api.current_session_account_is_sysadmin(),
    ple_api.current_session_account_is_course_instructor(uuid),
    ple_api.current_session_account_is_course_member(uuid),
    ple_api.current_session_account_owns_course_membership(uuid, uuid),
    ple_api.current_session_account_owns_student_record(uuid, uuid) FROM PUBLIC;
GRANT USAGE ON SCHEMA ple_api TO ple_app, ple_auth, ple_student, ple_data_owner, ple_private_owner;
GRANT EXECUTE ON FUNCTION ple_api.current_session_account_id(),
    ple_api.current_session_account_has_active_role(text),
    ple_api.current_session_account_is_instructor(),
    ple_api.current_session_account_is_sysadmin(),
    ple_api.current_session_account_is_course_instructor(uuid),
    ple_api.current_session_account_is_course_member(uuid),
    ple_api.current_session_account_owns_course_membership(uuid, uuid),
    ple_api.current_session_account_owns_student_record(uuid, uuid)
    TO ple_app, ple_auth, ple_student, ple_data_owner, ple_private_owner;

RESET ROLE;
