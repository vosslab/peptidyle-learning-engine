-- Functions, triggers, and views from blueprint_pools.sql.

SET LOCAL ROLE ple_api_owner;




-- A retry is resolved before any fresh child identities or member revisions are materialized.
CREATE FUNCTION ple_api.blueprint_pool_write_receipt(p_blueprint_course_id text, p_checksum bytea)
RETURNS TABLE (blueprint_course_id text, revision_number bigint, blueprint_edit_number bigint, changed boolean,
    accepted_at_millis bigint) LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data AS $$
DECLARE course_row ple_data.blueprint_course%ROWTYPE;
BEGIN
    IF NOT ple_api.current_session_account_is_instructor() THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Blueprint write is unavailable';
    END IF;
    IF p_blueprint_course_id IS NULL THEN
        PERFORM pg_advisory_xact_lock(hashtextextended(format('ple:blueprint-course-create:%s:%s',
            ple_api.current_session_account_id(), encode(p_checksum, 'hex')), 0));
        RETURN QUERY SELECT course.blueprint_course_id, receipt.blueprint_revision_number,
            receipt.blueprint_edit_number, true, (extract(epoch FROM receipt.accepted_at)*1000)::bigint
            FROM ple_data.blueprint_course_create_receipt AS receipt JOIN ple_data.blueprint_course AS course
            ON course.blueprint_course_id = receipt.blueprint_course_id
            WHERE receipt.actor_account_id = ple_api.current_session_account_id() AND receipt.request_checksum = p_checksum;
    ELSE
        SELECT * INTO course_row FROM ple_data.blueprint_course WHERE blueprint_course.blueprint_course_id = p_blueprint_course_id FOR UPDATE;
        IF NOT FOUND OR course_row.owner_account_id <> ple_api.current_session_account_id()
            OR course_row.availability = 'archived' THEN
            RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Blueprint write is unavailable';
        END IF;
        RETURN QUERY SELECT p_blueprint_course_id, receipt.resulting_blueprint_revision_number,
            course_row.blueprint_edit_number, receipt.changed, (extract(epoch FROM receipt.accepted_at)*1000)::bigint
            FROM ple_data.blueprint_course_save_receipt AS receipt
            WHERE receipt.blueprint_course_id = course_row.blueprint_course_id
              AND receipt.actor_account_id = ple_api.current_session_account_id() AND receipt.request_checksum = p_checksum;
    END IF;
END $$;

CREATE FUNCTION ple_api.fork_blueprint_question_pool(
    p_source_question_pool_id text, p_child_question_pool_id text
) RETURNS bigint LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data AS $$
DECLARE forked record;
BEGIN
    IF NOT ple_api.current_session_account_is_instructor() THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Blueprint Pool import is unavailable';
    END IF;
    SELECT * INTO forked FROM ple_data.fork_question_pool(
        p_child_question_pool_id, p_source_question_pool_id);
    RETURN forked.question_pool_edit_number;
END $$;

CREATE FUNCTION ple_api.blueprint_pool_members(
    p_blueprint_course_id text, p_blueprint_assessment_id uuid, p_public_pool_id text, p_write boolean,
    p_expected_blueprint_revision_number bigint DEFAULT NULL, p_expected_pool_edit_number bigint DEFAULT NULL
) RETURNS TABLE (question_pool_edit_number bigint, published_question_id text, question_revision_number integer)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data AS $$
DECLARE course_row ple_data.blueprint_course%ROWTYPE; content_value jsonb; pin_id text;
BEGIN
    -- ASVS 8.2.2 / 8.3.1: membership is checked inside the trusted database boundary.
    SELECT * INTO course_row FROM ple_data.blueprint_course
      WHERE blueprint_course_id = p_blueprint_course_id FOR UPDATE;
    IF NOT FOUND OR NOT ple_api.current_session_account_is_instructor()
       OR (p_write AND (course_row.owner_account_id <> ple_api.current_session_account_id()
                       OR course_row.availability = 'archived'))
       OR (NOT p_write AND course_row.owner_account_id <> ple_api.current_session_account_id()
                       AND course_row.availability NOT IN ('public', 'archived')) THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Blueprint Pool is unavailable';
    END IF;
    IF p_write AND course_row.current_blueprint_revision_number IS DISTINCT FROM p_expected_blueprint_revision_number THEN
        RAISE EXCEPTION USING ERRCODE = '40001', MESSAGE = 'Blueprint Revision precondition is stale';
    END IF;
    SELECT content INTO content_value FROM ple_data.blueprint_course_revision
      WHERE blueprint_course_id = course_row.blueprint_course_id
        AND blueprint_revision_number = course_row.current_blueprint_revision_number;
    SELECT entry ->> 'question_pool_id' INTO pin_id
      FROM jsonb_array_elements(content_value -> 'modules') AS module,
           jsonb_array_elements(module -> 'assessments') AS assessment,
           jsonb_array_elements(assessment #> '{content,entries}') AS entry
      WHERE (assessment ->> 'blueprint_assessment_id')::uuid = p_blueprint_assessment_id
        AND entry ->> 'question_pool_id' = p_public_pool_id;
    IF pin_id IS NULL THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Blueprint Assessment Pool membership is unavailable';
    END IF;
    IF p_write AND EXISTS (
        SELECT 1 FROM ple_data.question_pool AS pool
         WHERE pool.question_pool_id = p_public_pool_id
           AND pool.question_pool_edit_number IS DISTINCT FROM p_expected_pool_edit_number
    ) THEN
        RAISE EXCEPTION USING ERRCODE = '40001', MESSAGE = 'Question Pool Edit Number is stale';
    END IF;
    RETURN QUERY SELECT pool.question_pool_edit_number, member.published_question_id, member.question_revision_number
      FROM ple_data.question_pool AS pool
      JOIN ple_data.question_pool_member AS member USING (question_pool_id)
      WHERE pool.question_pool_id = p_public_pool_id
      ORDER BY member.member_position;
END $$;

CREATE FUNCTION ple_api.append_blueprint_pool_members(
    p_blueprint_course_id text, p_blueprint_assessment_id uuid, p_expected_blueprint_revision_number bigint,
    p_public_pool_id text, p_expected_pool_edit_number bigint,
    p_question_ids text[], p_revision_numbers integer[], p_attested boolean
) RETURNS bigint LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data AS $$
DECLARE pool_row ple_data.question_pool%ROWTYPE; appended record;
BEGIN
    PERFORM * FROM ple_api.blueprint_pool_members(p_blueprint_course_id, p_blueprint_assessment_id,
        p_public_pool_id, true, p_expected_blueprint_revision_number, p_expected_pool_edit_number);
    SELECT * INTO pool_row FROM ple_data.question_pool WHERE question_pool_id = p_public_pool_id;
    SELECT * INTO appended FROM ple_data.save_question_pool_members(pool_row.question_pool_id,
        pool_row.question_pool_edit_number, p_question_ids, p_revision_numbers, p_attested);
    RETURN appended.question_pool_edit_number;
END $$;

SET LOCAL ROLE ple_data_owner;

CREATE FUNCTION ple_data.validate_blueprint_owned_pools() RETURNS trigger
LANGUAGE plpgsql SECURITY INVOKER SET search_path = pg_catalog, ple_data AS $$
DECLARE assessment_value jsonb; entry_value jsonb; prior_content jsonb; pool_row ple_data.question_pool%ROWTYPE;
    prior_id text; used_ids text[] := ARRAY[]::text[]; public_id text;
BEGIN
    SELECT content INTO prior_content FROM ple_data.blueprint_course_revision
        WHERE blueprint_course_id = NEW.blueprint_course_id
          AND blueprint_revision_number < NEW.blueprint_revision_number
        ORDER BY blueprint_revision_number DESC LIMIT 1;
    FOR assessment_value IN SELECT assessment FROM jsonb_array_elements(NEW.content -> 'modules') AS module,
        jsonb_array_elements(module -> 'assessments') AS assessment LOOP
        FOR entry_value IN SELECT entry FROM jsonb_array_elements(assessment_value #> '{content,entries}') AS entry
            WHERE entry ->> 'kind' = 'pool' LOOP
            public_id := entry_value ->> 'question_pool_id';
            IF public_id = ANY(used_ids) THEN
                RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'A Blueprint owned Pool may occur only once';
            END IF;
            used_ids := array_append(used_ids, public_id);
            SELECT * INTO pool_row FROM ple_data.question_pool WHERE question_pool_id = public_id;
            IF NOT EXISTS (SELECT 1 FROM ple_data.question_pool_member AS member
                WHERE member.question_pool_id = pool_row.question_pool_id
                HAVING count(*) >= (entry_value ->> 'selection_count')::integer) THEN
                RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Blueprint Pool selection exceeds current membership';
            END IF;
            SELECT entry ->> 'question_pool_id' INTO prior_id
                FROM jsonb_array_elements(prior_content -> 'modules') AS module,
                     jsonb_array_elements(module -> 'assessments') AS assessment,
                     jsonb_array_elements(assessment #> '{content,entries}') AS entry
                WHERE assessment ->> 'blueprint_assessment_id' = assessment_value ->> 'blueprint_assessment_id'
                  AND entry ->> 'question_pool_id' = public_id;
            -- ASVS 8.2.2: unchanged membership or a fresh transaction-local fork is the only write authority.
            IF pool_row.question_pool_id IS NULL OR pool_row.source_question_pool_id IS NULL OR
                (prior_id IS NULL AND (
                    pool_row.created_in_transaction <> pg_current_xact_id()
                    OR EXISTS (SELECT 1 FROM ple_data.assessment_question_pool_fork AS owned
                        WHERE owned.question_pool_id = pool_row.question_pool_id)
                    OR EXISTS (SELECT 1 FROM ple_data.blueprint_course_revision AS revision,
                        LATERAL ple_data.blueprint_content_pools(revision.content) AS pool
                        WHERE pool.question_pool_id = public_id))) THEN
                RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Blueprint Pool ownership is unavailable';
            END IF;
        END LOOP;
    END LOOP;
    RETURN NEW;
END $$;

CREATE TRIGGER blueprint_revision_owned_pools BEFORE INSERT ON ple_data.blueprint_course_revision
    FOR EACH ROW EXECUTE FUNCTION ple_data.validate_blueprint_owned_pools();

