-- Functions and views for retained Library Object impact notices.

SET LOCAL ROLE ple_data_owner;

-- All target, author, owner, and role predicates are derived in PostgreSQL.
CREATE FUNCTION ple_data.library_object_current_revision_number(
    p_object_kind text, p_public_object_id text, p_require_available boolean DEFAULT true
) RETURNS bigint LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_data AS $$
DECLARE v_revision_number bigint;
BEGIN
    IF p_object_kind = 'question' THEN
        SELECT max(revision.revision_number)::bigint INTO v_revision_number
          FROM ple_data.published_question AS question
          JOIN ple_data.question_revision AS revision
            ON revision.published_question_id = question.published_question_id
         WHERE question.published_question_id = p_public_object_id
           AND (NOT p_require_available OR question.availability = 'available');
    ELSIF p_object_kind = 'question_pool' THEN
        SELECT pool.question_pool_edit_number INTO v_revision_number
          FROM ple_data.question_pool AS pool
         WHERE pool.question_pool_id = p_public_object_id;
    END IF;
    IF v_revision_number IS NULL THEN
        RAISE EXCEPTION USING ERRCODE = 'P1D01', MESSAGE = 'Library Object is unavailable';
    END IF;
    RETURN v_revision_number;
END
$$;

CREATE FUNCTION ple_data.library_object_revision_exists(
    p_object_kind text, p_public_object_id text, p_revision_number bigint
) RETURNS boolean LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_data AS $$
    SELECT CASE p_object_kind
      WHEN 'question' THEN EXISTS (
        SELECT 1 FROM ple_data.question_revision
         WHERE published_question_id = p_public_object_id AND revision_number = p_revision_number)
      WHEN 'question_pool' THEN EXISTS (
        SELECT 1 FROM ple_data.question_pool AS pool
         WHERE pool.question_pool_id = p_public_object_id
           AND p_revision_number > 0
           AND p_revision_number <= pool.question_pool_edit_number)
      ELSE false END
$$;

CREATE FUNCTION ple_data.current_actor_owns_library_object(
    p_object_kind text, p_public_object_id text
) RETURNS boolean LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data AS $$
    SELECT p_object_kind = 'question' AND EXISTS (
        SELECT 1 FROM ple_data.question_current_owner
         WHERE published_question_id = p_public_object_id
           AND owner_account_id = ple_api.current_session_account_id())
$$;

CREATE FUNCTION ple_data.require_library_impact_notice_manager(
    p_object_kind text, p_public_object_id text
) RETURNS text LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
BEGIN
    -- Resolve the exact target before evaluating target-specific authority.
    PERFORM ple_data.library_object_current_revision_number(
        p_object_kind, p_public_object_id, false);
    IF ple_api.current_session_account_has_platform_administration() THEN
        -- Pool notice authorship deliberately retains the current Sysadmin
        -- authority until the separate product decision selects a replacement.
        RETURN 'Sysadmin';
    END IF;
    IF p_object_kind = 'question'
       AND ple_api.current_session_account_is_instructor()
       AND ple_data.current_actor_owns_library_object(p_object_kind, p_public_object_id) THEN
        RETURN ple_private.instructor_display_name(ple_api.current_session_account_id());
    END IF;
    RAISE EXCEPTION USING ERRCODE = '42501',
        MESSAGE = 'Impact notice manager authority is required';
END
$$;

CREATE FUNCTION ple_data.create_library_impact_notice(
    p_object_kind text, p_public_object_id text, p_affected_revision_number bigint, p_body text
) RETURNS uuid LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data AS $$
DECLARE v_notice_id uuid := pg_catalog.gen_random_uuid();
    v_now timestamptz := pg_catalog.clock_timestamp(); v_author_display_name text;
BEGIN
    v_author_display_name := ple_data.require_library_impact_notice_manager(
        p_object_kind, p_public_object_id);
    IF p_body IS NULL OR p_body <> btrim(p_body)
       OR char_length(p_body) NOT BETWEEN 1 AND 4000 OR p_body ~ '[[:cntrl:]]'
       OR (p_affected_revision_number IS NOT NULL AND NOT ple_data.library_object_revision_exists(
           p_object_kind, p_public_object_id, p_affected_revision_number)) THEN
        RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Impact notice is invalid';
    END IF;
    INSERT INTO ple_data.library_impact_notice(
        impact_notice_id, object_kind, public_object_id, affected_revision_number,
        created_by_account_id, author_display_name, body, created_at, updated_at
    ) VALUES (
        v_notice_id, p_object_kind::ple_data.library_object_kind, p_public_object_id,
        p_affected_revision_number, ple_api.current_session_account_id(), v_author_display_name,
        p_body, v_now, v_now
    );
    RETURN v_notice_id;
END
$$;

CREATE FUNCTION ple_data.update_library_impact_notice(
    p_object_kind text, p_public_object_id text, p_impact_notice_id uuid,
    p_affected_revision_number bigint, p_body text
) RETURNS void LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_data AS $$
DECLARE v_notice ple_data.library_impact_notice%ROWTYPE;
BEGIN
    PERFORM ple_data.require_library_impact_notice_manager(
        p_object_kind, p_public_object_id);
    IF p_impact_notice_id IS NULL THEN
        RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Impact notice is invalid';
    END IF;
    SELECT * INTO v_notice FROM ple_data.library_impact_notice
     WHERE impact_notice_id = p_impact_notice_id AND object_kind::text = p_object_kind
       AND public_object_id = p_public_object_id FOR UPDATE;
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = 'P1D01', MESSAGE = 'Impact notice is unavailable';
    END IF;
    IF v_notice.state <> 'active' THEN
        RAISE EXCEPTION USING ERRCODE = '55000', MESSAGE = 'Impact notice cannot be updated';
    END IF;
    IF p_body IS NULL OR p_body <> btrim(p_body)
       OR char_length(p_body) NOT BETWEEN 1 AND 4000 OR p_body ~ '[[:cntrl:]]'
       OR (p_affected_revision_number IS NOT NULL
           AND NOT ple_data.library_object_revision_exists(
               v_notice.object_kind::text, v_notice.public_object_id,
               p_affected_revision_number)) THEN
        RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Impact notice is invalid';
    END IF;
    IF v_notice.affected_revision_number IS NOT DISTINCT FROM p_affected_revision_number
       AND v_notice.body = p_body THEN
        RETURN;
    END IF;
    UPDATE ple_data.library_impact_notice SET affected_revision_number = p_affected_revision_number,
        body = p_body, updated_at = pg_catalog.clock_timestamp()
     WHERE impact_notice_id = p_impact_notice_id;
END
$$;

CREATE FUNCTION ple_data.cancel_library_impact_notice(
    p_object_kind text, p_public_object_id text, p_impact_notice_id uuid
) RETURNS void LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data AS $$
DECLARE v_notice ple_data.library_impact_notice%ROWTYPE; v_cancelled_at timestamptz;
BEGIN
    PERFORM ple_data.require_library_impact_notice_manager(
        p_object_kind, p_public_object_id);
    IF p_impact_notice_id IS NULL THEN
        RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Impact notice is invalid';
    END IF;
    SELECT * INTO v_notice FROM ple_data.library_impact_notice
     WHERE impact_notice_id = p_impact_notice_id AND object_kind::text = p_object_kind
       AND public_object_id = p_public_object_id FOR UPDATE;
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = 'P1D01', MESSAGE = 'Impact notice is unavailable';
    END IF;
    IF v_notice.state = 'cancelled' THEN
        RETURN;
    END IF;
    v_cancelled_at := pg_catalog.clock_timestamp();
    UPDATE ple_data.library_impact_notice SET state = 'cancelled',
        cancelled_by_account_id = ple_api.current_session_account_id(),
        cancelled_at = v_cancelled_at, updated_at = v_cancelled_at
     WHERE impact_notice_id = p_impact_notice_id;
END
$$;

SET LOCAL ROLE ple_api_owner;

CREATE FUNCTION ple_api.create_library_impact_notice(text, text, bigint, text)
RETURNS uuid LANGUAGE sql SECURITY DEFINER SET search_path = pg_catalog, ple_data AS $$
    SELECT ple_data.create_library_impact_notice($1, $2, $3, $4) $$;

CREATE FUNCTION ple_api.update_library_impact_notice(text, text, uuid, bigint, text)
RETURNS void LANGUAGE sql SECURITY DEFINER SET search_path = pg_catalog, ple_data AS $$
    SELECT ple_data.update_library_impact_notice($1, $2, $3, $4, $5) $$;

CREATE FUNCTION ple_api.cancel_library_impact_notice(text, text, uuid)
RETURNS void LANGUAGE sql SECURITY DEFINER SET search_path = pg_catalog, ple_data AS $$
    SELECT ple_data.cancel_library_impact_notice($1, $2, $3) $$;
