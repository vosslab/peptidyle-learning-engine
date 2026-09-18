-- Functions, triggers, and views from library_discussion_operations.sql.

SET LOCAL ROLE ple_data_owner;

-- Session-authorized Library discussion and impact-notice operations.
-- All target, author, owner, and role predicates are derived in PostgreSQL.
CREATE FUNCTION ple_data.library_object_current_revision(
    p_object_kind text, p_public_object_id text, p_require_available boolean DEFAULT true
) RETURNS bigint LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_data AS $$
DECLARE v_revision bigint;
BEGIN
    IF p_object_kind = 'question' THEN
        SELECT max(revision.revision_number)::bigint INTO v_revision
          FROM ple_data.published_question AS question
          JOIN ple_data.question_revision AS revision ON revision.published_question_id = question.published_question_id
         WHERE question.published_question_id = p_public_object_id
           AND (NOT p_require_available OR question.availability = 'available');
    ELSIF p_object_kind = 'question_pool' THEN
        SELECT pool.current_revision_number INTO v_revision
          FROM ple_data.question_pool AS pool
         WHERE pool.public_question_pool_id = p_public_object_id;
    END IF;
    IF v_revision IS NULL THEN
        RAISE EXCEPTION USING ERRCODE = 'P1D01', MESSAGE = 'Library Object is unavailable';
    END IF;
    RETURN v_revision;
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
        JOIN ple_data.question_pool_revision AS revision ON revision.question_pool_id = pool.question_pool_id
         WHERE pool.public_question_pool_id = p_public_object_id AND revision.revision_number = p_revision_number)
      ELSE false END
$$;

CREATE FUNCTION ple_data.current_actor_owns_library_object(
    p_object_kind text, p_public_object_id text
) RETURNS boolean LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data AS $$
    SELECT CASE p_object_kind
      WHEN 'question' THEN EXISTS (
        SELECT 1 FROM ple_data.question_current_owner
         WHERE published_question_id = p_public_object_id
           AND owner_account_id = ple_api.current_session_account_id())
      -- Pools have no owner role.  Their administration is Sysadmin-only.
      WHEN 'question_pool' THEN false
      ELSE false END
$$;

CREATE FUNCTION ple_data.current_actor_is_library_discussion_participant()
RETURNS boolean LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private AS $$
    SELECT ple_api.current_session_account_is_instructor()
       AND ple_private.verified_instructor_display_name(
           ple_api.current_session_account_id()) IS NOT NULL
$$;

CREATE FUNCTION ple_data.current_actor_may_manage_library_discussion(
    p_object_kind text, p_public_object_id text
) RETURNS boolean LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data AS $$
    SELECT ple_api.current_session_account_has_platform_administration()
       OR (p_object_kind = 'question'
           AND ple_data.current_actor_is_library_discussion_participant()
           AND ple_data.current_actor_owns_library_object(
               p_object_kind, p_public_object_id))
$$;



-- ASVS 8.2.1-8.2.2 and 8.3.1: the trusted SQL boundary derives each
-- discussion capability from the current Account and exact Library Object.
CREATE FUNCTION ple_data.require_library_discussion_reader(
    p_object_kind text, p_public_object_id text
) RETURNS bigint LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data AS $$
DECLARE v_revision bigint;
BEGIN
    IF ple_api.current_session_account_has_platform_administration() THEN
        v_revision := ple_data.library_object_current_revision(
            p_object_kind, p_public_object_id, false);
    ELSIF ple_data.current_actor_is_library_discussion_participant() THEN
        v_revision := ple_data.library_object_current_revision(
            p_object_kind, p_public_object_id, true);
    ELSE
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Library discussion reader authority is required';
    END IF;
    RETURN v_revision;
END
$$;

CREATE FUNCTION ple_data.require_library_discussion_participant()
RETURNS text LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
DECLARE v_name text;
BEGIN
    IF NOT ple_data.current_actor_is_library_discussion_participant() THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Library discussion participation requires a vetted active Instructor';
    END IF;
    v_name := ple_private.verified_instructor_display_name(
        ple_api.current_session_account_id());
    RETURN v_name;
END
$$;

CREATE FUNCTION ple_data.require_library_discussion_manager(
    p_object_kind text, p_public_object_id text
) RETURNS text LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
BEGIN
    -- Resolve the exact target before evaluating target-specific authority.
    PERFORM ple_data.library_object_current_revision(
        p_object_kind, p_public_object_id, false);
    IF NOT ple_data.current_actor_may_manage_library_discussion(
        p_object_kind, p_public_object_id) THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Library discussion manager authority is required';
    END IF;
    IF ple_api.current_session_account_has_platform_administration() THEN
        -- Sysadmin Accounts have no Instructor profile on this surface.
        RETURN 'Sysadmin';
    END IF;
    RETURN ple_private.verified_instructor_display_name(
        ple_api.current_session_account_id());
END
$$;

CREATE FUNCTION ple_data.create_library_improvement_thread(
    p_object_kind text, p_public_object_id text, p_body text
) RETURNS uuid LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data AS $$
DECLARE v_actor uuid := ple_api.current_session_account_id(); v_name text;
    v_revision bigint; v_thread_id uuid := pg_catalog.gen_random_uuid(); v_now timestamptz := pg_catalog.clock_timestamp();
BEGIN
    v_name := ple_data.require_library_discussion_participant();
    v_revision := ple_data.require_library_discussion_reader(
        p_object_kind, p_public_object_id);
    IF p_body IS NULL OR p_body <> btrim(p_body)
       OR char_length(p_body) NOT BETWEEN 1 AND 4000
       OR p_body ~ '[[:cntrl:]]' THEN
        RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Improvement thread is invalid';
    END IF;
    INSERT INTO ple_data.library_improvement_thread(
        library_improvement_thread_id, object_kind, public_object_id, creation_revision_number,
        created_by_account_id, created_at
    ) VALUES (v_thread_id, p_object_kind, p_public_object_id, v_revision, v_actor, v_now);
    INSERT INTO ple_data.library_improvement_post(
        post_id, library_improvement_thread_id, author_account_id, author_display_name, body, created_at
    ) VALUES (pg_catalog.gen_random_uuid(), v_thread_id, v_actor, v_name, p_body, v_now);
    RETURN v_thread_id;
END
$$;

CREATE FUNCTION ple_data.reply_to_library_improvement_thread(
    p_object_kind text, p_public_object_id text, p_thread_id uuid, p_body text
) RETURNS uuid LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data AS $$
DECLARE v_actor uuid := ple_api.current_session_account_id(); v_name text;
    v_post_id uuid := pg_catalog.gen_random_uuid(); v_thread ple_data.library_improvement_thread%ROWTYPE;
BEGIN
    v_name := ple_data.require_library_discussion_participant();
    PERFORM ple_data.require_library_discussion_reader(
        p_object_kind, p_public_object_id);
    IF p_thread_id IS NULL THEN
        RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Improvement reply is invalid';
    END IF;
    SELECT * INTO v_thread FROM ple_data.library_improvement_thread
     WHERE library_improvement_thread_id = p_thread_id AND object_kind = p_object_kind
       AND public_object_id = p_public_object_id FOR KEY SHARE;
    IF NOT FOUND THEN RAISE EXCEPTION USING ERRCODE = 'P1D01', MESSAGE = 'Improvement thread is unavailable'; END IF;
    IF p_body IS NULL OR p_body <> btrim(p_body)
       OR char_length(p_body) NOT BETWEEN 1 AND 4000 OR p_body ~ '[[:cntrl:]]' THEN
        RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Improvement reply is invalid';
    END IF;
    INSERT INTO ple_data.library_improvement_post(
        post_id, library_improvement_thread_id, author_account_id, author_display_name, body, created_at
    ) VALUES (v_post_id, p_thread_id, v_actor, v_name, p_body, pg_catalog.clock_timestamp());
    RETURN v_post_id;
END
$$;

CREATE FUNCTION ple_data.edit_own_library_improvement_post(
    p_object_kind text, p_public_object_id text, p_post_id uuid, p_body text
) RETURNS void LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data AS $$
DECLARE v_actor uuid := ple_api.current_session_account_id();
    v_post record;
BEGIN
    PERFORM ple_data.require_library_discussion_participant();
    PERFORM ple_data.require_library_discussion_reader(
        p_object_kind, p_public_object_id);
    IF p_post_id IS NULL THEN
        RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Improvement post is invalid';
    END IF;
    SELECT post.author_account_id, thread.object_kind, thread.public_object_id
      INTO v_post
      FROM ple_data.library_improvement_post AS post
      JOIN ple_data.library_improvement_thread AS thread ON thread.library_improvement_thread_id = post.library_improvement_thread_id
     WHERE post.post_id = p_post_id AND thread.object_kind = p_object_kind
       AND thread.public_object_id = p_public_object_id
     FOR UPDATE OF post;
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = 'P1D01', MESSAGE = 'Improvement post is unavailable';
    END IF;
    IF v_post.author_account_id <> v_actor THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Only the post author may edit this post';
    END IF;
    IF p_body IS NULL OR p_body <> btrim(p_body)
       OR char_length(p_body) NOT BETWEEN 1 AND 4000 OR p_body ~ '[[:cntrl:]]' THEN
        RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Improvement post is invalid';
    END IF;
    UPDATE ple_data.library_improvement_post SET body = p_body, updated_at = pg_catalog.clock_timestamp()
     WHERE post_id = p_post_id;
END
$$;

CREATE FUNCTION ple_data.set_library_improvement_thread_state(
    p_object_kind text, p_public_object_id text, p_thread_id uuid, p_resolved boolean
) RETURNS void LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data AS $$
DECLARE v_thread ple_data.library_improvement_thread%ROWTYPE;
BEGIN
    PERFORM ple_data.require_library_discussion_manager(
        p_object_kind, p_public_object_id);
    IF p_thread_id IS NULL OR p_resolved IS NULL THEN
        RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Improvement thread state is invalid';
    END IF;
    SELECT * INTO v_thread FROM ple_data.library_improvement_thread
     WHERE library_improvement_thread_id = p_thread_id AND object_kind = p_object_kind
       AND public_object_id = p_public_object_id FOR UPDATE;
    IF NOT FOUND THEN RAISE EXCEPTION USING ERRCODE = 'P1D01', MESSAGE = 'Improvement thread is unavailable'; END IF;
    IF p_resolved THEN
        UPDATE ple_data.library_improvement_thread SET state = 'resolved',
            resolved_by_account_id = ple_api.current_session_account_id(), resolved_at = pg_catalog.clock_timestamp()
         WHERE library_improvement_thread_id = p_thread_id AND state <> 'resolved';
    ELSE
        UPDATE ple_data.library_improvement_thread SET state = 'open',
            resolved_by_account_id = NULL, resolved_at = NULL
         WHERE library_improvement_thread_id = p_thread_id AND state <> 'open';
    END IF;
END
$$;

CREATE FUNCTION ple_data.create_library_impact_notice(
    p_object_kind text, p_public_object_id text, p_affected_revision_number bigint, p_body text
) RETURNS uuid LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data AS $$
DECLARE v_notice_id uuid := pg_catalog.gen_random_uuid(); v_now timestamptz := pg_catalog.clock_timestamp();
    v_author_display_name text;
BEGIN
    v_author_display_name := ple_data.require_library_discussion_manager(
        p_object_kind, p_public_object_id);
    -- ASVS 8.2.2: target authority is resolved before the target-specific
    -- affected-Revision check, so validation cannot become an object oracle.
    IF p_body IS NULL
       OR p_body <> btrim(p_body) OR char_length(p_body) NOT BETWEEN 1 AND 4000 OR p_body ~ '[[:cntrl:]]'
       OR (p_affected_revision_number IS NOT NULL AND NOT ple_data.library_object_revision_exists(
           p_object_kind, p_public_object_id, p_affected_revision_number)) THEN
        RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Impact notice is invalid';
    END IF;
    INSERT INTO ple_data.library_impact_notice(
        impact_notice_id, object_kind, public_object_id, affected_revision_number,
        created_by_account_id, author_display_name, body, created_at, updated_at
    ) VALUES (
        v_notice_id, p_object_kind, p_public_object_id, p_affected_revision_number,
        ple_api.current_session_account_id(), v_author_display_name, p_body, v_now, v_now
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
    PERFORM ple_data.require_library_discussion_manager(
        p_object_kind, p_public_object_id);
    IF p_impact_notice_id IS NULL THEN
        RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Impact notice is invalid';
    END IF;
    SELECT * INTO v_notice FROM ple_data.library_impact_notice
     WHERE impact_notice_id = p_impact_notice_id AND object_kind = p_object_kind
       AND public_object_id = p_public_object_id FOR UPDATE;
    IF NOT FOUND THEN RAISE EXCEPTION USING ERRCODE = 'P1D01', MESSAGE = 'Impact notice is unavailable'; END IF;
    IF v_notice.state <> 'active' THEN
        RAISE EXCEPTION USING ERRCODE = '55000', MESSAGE = 'Impact notice cannot be updated';
    END IF;
    IF p_body IS NULL OR p_body <> btrim(p_body)
       OR char_length(p_body) NOT BETWEEN 1 AND 4000 OR p_body ~ '[[:cntrl:]]'
       OR (p_affected_revision_number IS NOT NULL
           AND NOT ple_data.library_object_revision_exists(
               v_notice.object_kind, v_notice.public_object_id,
               p_affected_revision_number)) THEN
        RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Impact notice is invalid';
    END IF;
    -- A confirmed no-op retains the existing timestamp and emits no Watch activity.
    -- This check follows the lock and authorization above, preserving concealment.
    IF v_notice.affected_revision_number IS NOT DISTINCT FROM p_affected_revision_number
       AND v_notice.body = p_body THEN
        RETURN;
    END IF;
    UPDATE ple_data.library_impact_notice SET affected_revision_number = p_affected_revision_number,
        body = p_body, updated_at = pg_catalog.clock_timestamp() WHERE impact_notice_id = p_impact_notice_id;
END
$$;

CREATE FUNCTION ple_data.cancel_library_impact_notice(
    p_object_kind text, p_public_object_id text, p_impact_notice_id uuid
)
RETURNS void LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data AS $$
DECLARE v_notice ple_data.library_impact_notice%ROWTYPE; v_cancelled_at timestamptz;
BEGIN
    PERFORM ple_data.require_library_discussion_manager(
        p_object_kind, p_public_object_id);
    IF p_impact_notice_id IS NULL THEN RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Impact notice is invalid'; END IF;
    SELECT * INTO v_notice FROM ple_data.library_impact_notice
     WHERE impact_notice_id = p_impact_notice_id AND object_kind = p_object_kind
       AND public_object_id = p_public_object_id FOR UPDATE;
    IF NOT FOUND THEN RAISE EXCEPTION USING ERRCODE = 'P1D01', MESSAGE = 'Impact notice is unavailable'; END IF;
    IF v_notice.state = 'cancelled' THEN RETURN; END IF;
    v_cancelled_at := pg_catalog.clock_timestamp();
    UPDATE ple_data.library_impact_notice SET state = 'cancelled',
        cancelled_by_account_id = ple_api.current_session_account_id(),
        cancelled_at = v_cancelled_at, updated_at = v_cancelled_at
     WHERE impact_notice_id = p_impact_notice_id;
END
$$;

CREATE FUNCTION ple_data.read_library_improvement_threads(
    p_object_kind text, p_public_object_id text
) RETURNS TABLE (
    library_improvement_thread_id uuid, creation_revision_number bigint, state text, created_at_millis bigint,
    resolved_at_millis bigint, viewer_may_resolve boolean
) LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data AS $$
BEGIN
    PERFORM ple_data.require_library_discussion_reader(
        p_object_kind, p_public_object_id);
    RETURN QUERY SELECT thread.library_improvement_thread_id::uuid AS library_improvement_thread_id,
        thread.creation_revision_number, thread.state,
        floor(extract(epoch FROM thread.created_at) * 1000)::bigint,
        CASE WHEN thread.resolved_at IS NULL THEN NULL ELSE floor(extract(epoch FROM thread.resolved_at) * 1000)::bigint END,
        ple_data.current_actor_may_manage_library_discussion(
            thread.object_kind, thread.public_object_id)
      FROM ple_data.library_improvement_thread AS thread
     WHERE thread.object_kind = p_object_kind AND thread.public_object_id = p_public_object_id
     ORDER BY thread.created_at, thread.library_improvement_thread_id;
END
$$;

CREATE FUNCTION ple_data.read_library_improvement_posts(p_thread_id uuid)
RETURNS TABLE (
    post_id uuid, author_display_name text, body text, created_at_millis bigint,
    updated_at_millis bigint, viewer_may_edit boolean
) LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data AS $$
DECLARE v_thread ple_data.library_improvement_thread%ROWTYPE;
BEGIN
    SELECT * INTO v_thread FROM ple_data.library_improvement_thread WHERE library_improvement_thread_id = p_thread_id;
    IF NOT FOUND THEN RAISE EXCEPTION USING ERRCODE = 'P1D01', MESSAGE = 'Improvement thread is unavailable'; END IF;
    PERFORM ple_data.require_library_discussion_reader(
        v_thread.object_kind, v_thread.public_object_id);
    RETURN QUERY SELECT post.post_id, post.author_display_name, post.body,
        floor(extract(epoch FROM post.created_at) * 1000)::bigint,
        CASE WHEN post.updated_at IS NULL THEN NULL ELSE floor(extract(epoch FROM post.updated_at) * 1000)::bigint END,
        post.author_account_id = ple_api.current_session_account_id()
          AND ple_data.current_actor_is_library_discussion_participant()
      FROM ple_data.library_improvement_post AS post
     WHERE post.library_improvement_thread_id = p_thread_id ORDER BY post.created_at, post.post_id;
END
$$;

CREATE FUNCTION ple_data.read_library_impact_notices(
    p_object_kind text, p_public_object_id text
) RETURNS TABLE (
    impact_notice_id uuid, affected_revision_number bigint, author_display_name text, body text,
    state text, created_at_millis bigint, updated_at_millis bigint, cancelled_at_millis bigint,
    viewer_may_manage boolean
) LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data AS $$
BEGIN
    PERFORM ple_data.require_library_discussion_reader(
        p_object_kind, p_public_object_id);
    RETURN QUERY SELECT notice.impact_notice_id, notice.affected_revision_number, notice.author_display_name,
        notice.body, notice.state, floor(extract(epoch FROM notice.created_at) * 1000)::bigint,
        floor(extract(epoch FROM notice.updated_at) * 1000)::bigint,
        CASE WHEN notice.cancelled_at IS NULL THEN NULL ELSE floor(extract(epoch FROM notice.cancelled_at) * 1000)::bigint END,
        notice.state = 'active'
          AND ple_data.current_actor_may_manage_library_discussion(
              notice.object_kind, notice.public_object_id)
      FROM ple_data.library_impact_notice AS notice
     WHERE notice.object_kind = p_object_kind AND notice.public_object_id = p_public_object_id
     ORDER BY notice.created_at DESC, notice.impact_notice_id;
END
$$;

CREATE FUNCTION ple_data.read_library_object_discussion_management(
    p_object_kind text, p_public_object_id text
) RETURNS boolean LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data AS $$
BEGIN
    PERFORM ple_data.require_library_discussion_reader(
        p_object_kind, p_public_object_id);
    RETURN ple_data.current_actor_may_manage_library_discussion(
        p_object_kind, p_public_object_id);
END
$$;

SET LOCAL ROLE ple_api_owner;

CREATE FUNCTION ple_api.create_library_improvement_thread(text, text, text)
RETURNS uuid LANGUAGE sql SECURITY DEFINER SET search_path = pg_catalog, ple_data AS $$
    SELECT ple_data.create_library_improvement_thread($1, $2, $3) $$;

CREATE FUNCTION ple_api.reply_to_library_improvement_thread(text, text, uuid, text)
RETURNS uuid LANGUAGE sql SECURITY DEFINER SET search_path = pg_catalog, ple_data AS $$
    SELECT ple_data.reply_to_library_improvement_thread($1, $2, $3, $4) $$;

CREATE FUNCTION ple_api.edit_own_library_improvement_post(text, text, uuid, text)
RETURNS void LANGUAGE sql SECURITY DEFINER SET search_path = pg_catalog, ple_data AS $$
    SELECT ple_data.edit_own_library_improvement_post($1, $2, $3, $4) $$;

CREATE FUNCTION ple_api.set_library_improvement_thread_state(text, text, uuid, boolean)
RETURNS void LANGUAGE sql SECURITY DEFINER SET search_path = pg_catalog, ple_data AS $$
    SELECT ple_data.set_library_improvement_thread_state($1, $2, $3, $4) $$;

CREATE FUNCTION ple_api.create_library_impact_notice(text, text, bigint, text)
RETURNS uuid LANGUAGE sql SECURITY DEFINER SET search_path = pg_catalog, ple_data AS $$
    SELECT ple_data.create_library_impact_notice($1, $2, $3, $4) $$;

CREATE FUNCTION ple_api.update_library_impact_notice(text, text, uuid, bigint, text)
RETURNS void LANGUAGE sql SECURITY DEFINER SET search_path = pg_catalog, ple_data AS $$
    SELECT ple_data.update_library_impact_notice($1, $2, $3, $4, $5) $$;

CREATE FUNCTION ple_api.cancel_library_impact_notice(text, text, uuid)
RETURNS void LANGUAGE sql SECURITY DEFINER SET search_path = pg_catalog, ple_data AS $$
    SELECT ple_data.cancel_library_impact_notice($1, $2, $3) $$;

CREATE FUNCTION ple_api.read_library_improvement_threads(text, text)
RETURNS TABLE(library_improvement_thread_id uuid, creation_revision_number bigint, state text, created_at_millis bigint,
    resolved_at_millis bigint, viewer_may_resolve boolean)
LANGUAGE sql SECURITY DEFINER SET search_path = pg_catalog, ple_data AS $$
    SELECT * FROM ple_data.read_library_improvement_threads($1, $2) $$;

CREATE FUNCTION ple_api.read_library_improvement_posts(uuid)
RETURNS TABLE(post_id uuid, author_display_name text, body text, created_at_millis bigint,
    updated_at_millis bigint, viewer_may_edit boolean)
LANGUAGE sql SECURITY DEFINER SET search_path = pg_catalog, ple_data AS $$
    SELECT * FROM ple_data.read_library_improvement_posts($1) $$;

CREATE FUNCTION ple_api.read_library_impact_notices(text, text)
RETURNS TABLE(impact_notice_id uuid, affected_revision_number bigint, author_display_name text, body text,
    state text, created_at_millis bigint, updated_at_millis bigint, cancelled_at_millis bigint,
    viewer_may_manage boolean)
LANGUAGE sql SECURITY DEFINER SET search_path = pg_catalog, ple_data AS $$
    SELECT * FROM ple_data.read_library_impact_notices($1, $2) $$;

CREATE FUNCTION ple_api.read_library_object_discussion_management(text, text)
RETURNS boolean LANGUAGE sql SECURITY DEFINER SET search_path = pg_catalog, ple_data AS $$
    SELECT ple_data.read_library_object_discussion_management($1, $2) $$;

