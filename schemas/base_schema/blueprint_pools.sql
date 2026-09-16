-- Blueprint-owned Pools use exact current Assessment membership, not a second ownership table.
SET LOCAL ROLE ple_data_owner;
GRANT EXECUTE ON FUNCTION ple_data.fork_question_pool_revision(uuid,text,uuid,bigint),
    ple_data.append_question_pool_revision(uuid,uuid,text[],integer[],boolean) TO ple_api_owner;
SET LOCAL ROLE ple_api_owner;

-- A retry is resolved before any fresh child identities or member revisions are materialized.
CREATE FUNCTION ple_api.blueprint_pool_write_receipt(p_reference text, p_checksum bytea)
RETURNS TABLE (public_reference text, revision_number bigint, metadata_etag uuid, changed boolean,
    accepted_at_millis bigint) LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data AS $$
DECLARE course_row ple_data.blueprint_course%ROWTYPE;
BEGIN
    IF NOT ple_api.current_session_account_is_instructor() THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Blueprint write is unavailable';
    END IF;
    IF p_reference IS NULL THEN
        PERFORM pg_advisory_xact_lock(hashtextextended(format('ple:blueprint-course-create:%s:%s',
            ple_api.current_session_account_id(), encode(p_checksum, 'hex')), 0));
        RETURN QUERY SELECT course.public_reference, receipt.blueprint_revision_number,
            receipt.metadata_etag, true, (extract(epoch FROM receipt.accepted_at)*1000)::bigint
            FROM ple_data.blueprint_course_create_receipt AS receipt JOIN ple_data.blueprint_course AS course
            ON course.reference_number = receipt.blueprint_course_reference_number
            WHERE receipt.actor_account_id = ple_api.current_session_account_id() AND receipt.request_checksum = p_checksum;
    ELSE
        SELECT * INTO course_row FROM ple_data.blueprint_course WHERE blueprint_course.public_reference = p_reference FOR UPDATE;
        IF NOT FOUND OR course_row.owner_account_id <> ple_api.current_session_account_id()
            OR course_row.availability = 'archived' THEN
            RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Blueprint write is unavailable';
        END IF;
        RETURN QUERY SELECT p_reference, receipt.resulting_blueprint_revision_number,
            course_row.metadata_etag, receipt.changed, (extract(epoch FROM receipt.accepted_at)*1000)::bigint
            FROM ple_data.blueprint_course_save_receipt AS receipt
            WHERE receipt.blueprint_course_reference_number = course_row.reference_number
              AND receipt.actor_account_id = ple_api.current_session_account_id() AND receipt.request_checksum = p_checksum;
    END IF;
END $$;
REVOKE ALL ON FUNCTION ple_api.blueprint_pool_write_receipt(text,bytea) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_api.blueprint_pool_write_receipt(text,bytea) TO ple_app;

CREATE FUNCTION ple_api.fork_blueprint_question_pool(
    p_source_public_id text, p_source_revision bigint, p_child_id uuid, p_child_public_id text
) RETURNS bigint LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data AS $$
DECLARE source_id uuid; forked record;
BEGIN
    IF NOT ple_api.current_session_account_is_instructor() THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Blueprint Pool import is unavailable';
    END IF;
    SELECT question_pool_id INTO source_id FROM ple_data.question_pool
      WHERE public_question_pool_id = p_source_public_id;
    SELECT * INTO forked FROM ple_data.fork_question_pool_revision(
        p_child_id, p_child_public_id, source_id, p_source_revision);
    RETURN forked.revision_number;
END $$;

CREATE FUNCTION ple_api.blueprint_pool_members(
    p_reference text, p_assessment uuid, p_public_pool_id text, p_write boolean,
    p_expected_revision bigint DEFAULT NULL, p_expected_pool_revision bigint DEFAULT NULL
) RETURNS TABLE (pool_revision bigint, question_id text, question_revision_number integer)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data AS $$
DECLARE course_row ple_data.blueprint_course%ROWTYPE; content_value jsonb; pin_revision bigint;
BEGIN
    -- ASVS 8.2.2 / 8.3.1: membership is checked inside the trusted database boundary.
    SELECT * INTO course_row FROM ple_data.blueprint_course
      WHERE public_reference = p_reference FOR UPDATE;
    IF NOT FOUND OR NOT ple_api.current_session_account_is_instructor()
       OR (p_write AND (course_row.owner_account_id <> ple_api.current_session_account_id()
                       OR course_row.availability = 'archived'))
       OR (NOT p_write AND course_row.owner_account_id <> ple_api.current_session_account_id()
                       AND course_row.availability NOT IN ('public', 'archived')) THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Blueprint Pool is unavailable';
    END IF;
    IF p_write AND course_row.current_blueprint_revision_number IS DISTINCT FROM p_expected_revision THEN
        RAISE EXCEPTION USING ERRCODE = '40001', MESSAGE = 'Blueprint Revision precondition is stale';
    END IF;
    SELECT content INTO content_value FROM ple_data.blueprint_course_revision
      WHERE blueprint_course_reference_number = course_row.reference_number
        AND blueprint_revision_number = course_row.current_blueprint_revision_number;
    SELECT (entry #>> '{question_pool_revision,revisionNumber}')::bigint INTO pin_revision
      FROM jsonb_array_elements(content_value -> 'modules') AS module,
           jsonb_array_elements(module -> 'assessments') AS assessment,
           jsonb_array_elements(assessment #> '{content,entries}') AS entry
      WHERE (assessment ->> 'blueprint_assessment_reference')::uuid = p_assessment
        AND replace(entry #>> '{question_pool_revision,questionPoolId}', '-', '') = p_public_pool_id;
    IF pin_revision IS NULL OR (p_write AND pin_revision IS DISTINCT FROM p_expected_pool_revision) THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Blueprint Assessment Pool membership is unavailable';
    END IF;
    RETURN QUERY SELECT pin_revision, member.question_id, member.question_revision_number
      FROM ple_data.question_pool AS pool
      JOIN ple_data.question_pool_revision_member AS member USING (question_pool_id)
      WHERE pool.public_question_pool_id = p_public_pool_id AND member.revision_number = pin_revision
      ORDER BY member.member_position;
END $$;

CREATE FUNCTION ple_api.append_blueprint_pool_revision(
    p_reference text, p_assessment uuid, p_expected_revision bigint,
    p_public_pool_id text, p_expected_pool_revision bigint,
    p_question_ids text[], p_revision_numbers integer[], p_attested boolean
) RETURNS bigint LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data AS $$
DECLARE pool_row ple_data.question_pool%ROWTYPE; appended record;
BEGIN
    PERFORM * FROM ple_api.blueprint_pool_members(p_reference, p_assessment,
        p_public_pool_id, true, p_expected_revision, p_expected_pool_revision);
    SELECT * INTO pool_row FROM ple_data.question_pool WHERE public_question_pool_id = p_public_pool_id;
    SELECT * INTO appended FROM ple_data.append_question_pool_revision(pool_row.question_pool_id,
        pool_row.metadata_etag, p_question_ids, p_revision_numbers, p_attested);
    RETURN appended.revision_number;
END $$;

REVOKE ALL ON FUNCTION ple_api.fork_blueprint_question_pool(text,bigint,uuid,text),
    ple_api.blueprint_pool_members(text,uuid,text,boolean,bigint,bigint),
    ple_api.append_blueprint_pool_revision(text,uuid,bigint,text,bigint,text[],integer[],boolean) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_api.fork_blueprint_question_pool(text,bigint,uuid,text),
    ple_api.blueprint_pool_members(text,uuid,text,boolean,bigint,bigint),
    ple_api.append_blueprint_pool_revision(text,uuid,bigint,text,bigint,text[],integer[],boolean) TO ple_app;
RESET ROLE;

SET LOCAL ROLE ple_data_owner;
CREATE FUNCTION ple_data.validate_blueprint_owned_pools() RETURNS trigger
LANGUAGE plpgsql SECURITY INVOKER SET search_path = pg_catalog, ple_data AS $$
DECLARE assessment_value jsonb; entry_value jsonb; prior_content jsonb; pool_row ple_data.question_pool%ROWTYPE;
    prior_pin bigint; next_pin bigint; used_ids text[] := ARRAY[]::text[]; public_id text;
BEGIN
    SELECT content INTO prior_content FROM ple_data.blueprint_course_revision
        WHERE blueprint_course_reference_number = NEW.blueprint_course_reference_number
          AND blueprint_revision_number < NEW.blueprint_revision_number
        ORDER BY blueprint_revision_number DESC LIMIT 1;
    FOR assessment_value IN SELECT assessment FROM jsonb_array_elements(NEW.content -> 'modules') AS module,
        jsonb_array_elements(module -> 'assessments') AS assessment LOOP
        FOR entry_value IN SELECT entry FROM jsonb_array_elements(assessment_value #> '{content,entries}') AS entry
            WHERE entry ->> 'kind' = 'pool' LOOP
            public_id := replace(entry_value #>> '{question_pool_revision,questionPoolId}', '-', '');
            next_pin := (entry_value #>> '{question_pool_revision,revisionNumber}')::bigint;
            IF public_id = ANY(used_ids) THEN
                RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'A Blueprint owned Pool may occur only once';
            END IF;
            used_ids := array_append(used_ids, public_id);
            SELECT * INTO pool_row FROM ple_data.question_pool WHERE public_question_pool_id = public_id;
            IF NOT EXISTS (SELECT 1 FROM ple_data.question_pool_revision AS revision
                WHERE revision.question_pool_id = pool_row.question_pool_id
                  AND revision.revision_number = next_pin
                  AND revision.member_count >= (entry_value ->> 'selection_count')::integer) THEN
                RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Blueprint Pool selection exceeds exact Revision membership';
            END IF;
            SELECT (entry #>> '{question_pool_revision,revisionNumber}')::bigint INTO prior_pin
                FROM jsonb_array_elements(prior_content -> 'modules') AS module,
                     jsonb_array_elements(module -> 'assessments') AS assessment,
                     jsonb_array_elements(assessment #> '{content,entries}') AS entry
                WHERE assessment ->> 'blueprint_assessment_reference' = assessment_value ->> 'blueprint_assessment_reference'
                  AND replace(entry #>> '{question_pool_revision,questionPoolId}', '-', '') = public_id;
            -- ASVS 8.2.2: unchanged membership or a fresh transaction-local fork is the only write authority.
            IF pool_row.question_pool_id IS NULL OR pool_row.source_question_pool_id IS NULL OR
                (prior_pin IS NOT NULL AND next_pin <> prior_pin AND next_pin <> pool_row.current_revision_number) OR
                (prior_pin IS NULL AND (next_pin <> 1 OR NOT EXISTS (
                    SELECT 1 FROM ple_data.question_pool_revision AS initial_revision
                    WHERE initial_revision.question_pool_id = pool_row.question_pool_id
                      AND initial_revision.revision_number = 1
                      AND initial_revision.created_in_transaction = pg_current_xact_id())
                    OR EXISTS (SELECT 1 FROM ple_data.assessment_question_pool_fork AS owned
                        WHERE owned.question_pool_id = pool_row.question_pool_id)
                    OR EXISTS (SELECT 1 FROM ple_data.blueprint_course_revision AS revision,
                        LATERAL ple_data.blueprint_content_pool_pins(revision.content) AS pin
                        WHERE pin.public_question_pool_id = public_id))) THEN
                RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Blueprint Pool ownership is unavailable';
            END IF;
        END LOOP;
    END LOOP;
    RETURN NEW;
END $$;
CREATE TRIGGER blueprint_revision_owned_pools BEFORE INSERT ON ple_data.blueprint_course_revision
    FOR EACH ROW EXECUTE FUNCTION ple_data.validate_blueprint_owned_pools();
REVOKE ALL ON FUNCTION ple_data.validate_blueprint_owned_pools() FROM PUBLIC;
RESET ROLE;
