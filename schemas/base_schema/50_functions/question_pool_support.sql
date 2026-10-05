-- Instructor read and replacement for one Question Pool's optional support texts.
-- ASVS 2.3.3 and 15.4.2: compare and advance the metadata token under the Pool row lock.
-- The command writes the Pool only. It does not update question_revision.

SET LOCAL ROLE ple_private_owner;

CREATE FUNCTION ple_private.stored_question_pool_support_text(p_value text)
RETURNS text
LANGUAGE plpgsql
SET search_path = pg_catalog AS $$
DECLARE v_stored text;
BEGIN
    -- ASVS 2.2.1: blank text is absent. Stored text is trimmed, bounded, and control-free.
    v_stored := NULLIF(btrim(p_value), '');
    IF v_stored IS NOT NULL AND (
        char_length(v_stored) > 4000 OR v_stored ~ '[[:cntrl:]]'
    ) THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Question Pool support text is invalid';
    END IF;
    RETURN v_stored;
END
$$;

CREATE FUNCTION ple_private.require_question_pool_support_instructor(p_question_pool_id text)
RETURNS void
LANGUAGE plpgsql
SET search_path = pg_catalog, ple_api, ple_private AS $$
BEGIN
    -- ASVS 8.2.1: only an active Instructor may read or replace Pool support.
    IF NOT ple_api.current_session_account_is_instructor()
       OR ple_api.current_session_account_id() IS NULL
       OR ple_private.instructor_display_name(ple_api.current_session_account_id()) IS NULL THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Question Pool support requires an active Instructor';
    END IF;
    IF p_question_pool_id IS NULL
       OR p_question_pool_id !~ '^[0-9A-HJKMNP-TV-Z]{4}-[0-9A-HJKMNP-TV-Z]{4}$'
       OR substr(p_question_pool_id, 6, 1) IS DISTINCT FROM ple_private.crockford_checksum_character(
            substr(p_question_pool_id, 1, 4) || substr(p_question_pool_id, 7, 3)
       ) THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Question Pool support target is invalid';
    END IF;
END
$$;

CREATE FUNCTION ple_private.read_question_pool_ple_managed_support(p_question_pool_id text)
RETURNS TABLE (
    question_pool_id text,
    question_pool_metadata_edit_number bigint,
    hint text,
    general_feedback text,
    worked_solution text
)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
BEGIN
    PERFORM ple_private.require_question_pool_support_instructor(p_question_pool_id);
    -- ASVS 8.2.3 and 14.2.6: return the Pool texts only. Member Questions stay unread.
    RETURN QUERY
    SELECT pool.question_pool_id::text,
           pool.question_pool_metadata_edit_number,
           pool.hint,
           pool.general_feedback,
           pool.worked_solution
      FROM ple_data.question_pool AS pool
     WHERE pool.question_pool_id = p_question_pool_id;
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Question Pool support target is not available';
    END IF;
END
$$;

CREATE FUNCTION ple_private.save_question_pool_ple_managed_support(
    p_question_pool_id text,
    p_expected_question_pool_metadata_edit_number bigint,
    p_hint text,
    p_general_feedback text,
    p_worked_solution text
) RETURNS TABLE (
    question_pool_id text,
    question_pool_metadata_edit_number bigint,
    hint text,
    general_feedback text,
    worked_solution text
)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
DECLARE
    v_current_edit_number bigint;
    v_hint text;
    v_general_feedback text;
    v_worked_solution text;
BEGIN
    PERFORM ple_private.require_question_pool_support_instructor(p_question_pool_id);
    IF p_expected_question_pool_metadata_edit_number IS NULL
       OR p_expected_question_pool_metadata_edit_number < 1 THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Question Pool metadata Edit Number is invalid';
    END IF;
    v_hint := ple_private.stored_question_pool_support_text(p_hint);
    v_general_feedback := ple_private.stored_question_pool_support_text(p_general_feedback);
    v_worked_solution := ple_private.stored_question_pool_support_text(p_worked_solution);
    -- ASVS 2.3.4 and 15.4.2: serialize the check and replacement at the Pool row.
    SELECT pool.question_pool_metadata_edit_number INTO v_current_edit_number
      FROM ple_data.question_pool AS pool
     WHERE pool.question_pool_id = p_question_pool_id
     FOR UPDATE;
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Question Pool support target is not available';
    END IF;
    IF v_current_edit_number <> p_expected_question_pool_metadata_edit_number THEN
        RAISE EXCEPTION USING ERRCODE = '40001',
            MESSAGE = 'Question Pool metadata Edit Number is stale';
    END IF;
    -- Membership identity stays unchanged; immutable question_revision is not updated.
    RETURN QUERY
    UPDATE ple_data.question_pool AS pool
       SET question_pool_metadata_edit_number = pool.question_pool_metadata_edit_number + 1,
           hint = v_hint,
           general_feedback = v_general_feedback,
           worked_solution = v_worked_solution,
           updated_on = CURRENT_DATE
     WHERE pool.question_pool_id = p_question_pool_id
    RETURNING pool.question_pool_id::text,
              pool.question_pool_metadata_edit_number,
              pool.hint,
              pool.general_feedback,
              pool.worked_solution;
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Question Pool support target is not available';
    END IF;
END
$$;

SET LOCAL ROLE ple_api_owner;

CREATE FUNCTION ple_api.read_question_pool_ple_managed_support(p_question_pool_id text)
RETURNS TABLE (
    question_pool_id text,
    question_pool_metadata_edit_number bigint,
    hint text,
    general_feedback text,
    worked_solution text
)
LANGUAGE sql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private AS $$
    SELECT question_pool_id, question_pool_metadata_edit_number, hint, general_feedback, worked_solution
      FROM ple_private.read_question_pool_ple_managed_support(p_question_pool_id)
$$;

CREATE FUNCTION ple_api.save_question_pool_ple_managed_support(
    p_question_pool_id text,
    p_expected_question_pool_metadata_edit_number bigint,
    p_hint text,
    p_general_feedback text,
    p_worked_solution text
) RETURNS TABLE (
    question_pool_id text,
    question_pool_metadata_edit_number bigint,
    hint text,
    general_feedback text,
    worked_solution text
)
LANGUAGE sql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private AS $$
    SELECT question_pool_id, question_pool_metadata_edit_number, hint, general_feedback, worked_solution
      FROM ple_private.save_question_pool_ple_managed_support(
          p_question_pool_id,
          p_expected_question_pool_metadata_edit_number,
          p_hint,
          p_general_feedback,
          p_worked_solution
      )
$$;
