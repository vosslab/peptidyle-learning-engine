-- Functions, triggers, and views from blueprint_pools.sql.

SET LOCAL ROLE ple_api_owner;




CREATE FUNCTION ple_api.blueprint_pool_members(
    p_blueprint_course_id text, p_blueprint_assessment_id uuid, p_public_pool_id text
) RETURNS TABLE (question_pool_edit_number bigint, published_question_id text, question_revision_number integer,
                 content_discipline_id uuid, content_subject_id uuid, question_type text, backend text)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data AS $$
DECLARE course_row ple_data.blueprint_course%ROWTYPE; content_value jsonb; pin_id text;
BEGIN
    -- ASVS 8.2.2 / 8.3.1: membership is checked inside the trusted database boundary.
    SELECT * INTO course_row FROM ple_data.blueprint_course
      WHERE blueprint_course_id = p_blueprint_course_id;
    IF NOT FOUND OR NOT ple_api.current_session_account_is_instructor()
       OR (course_row.owner_account_id <> ple_api.current_session_account_id()
                       AND course_row.availability NOT IN ('public', 'archived')) THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Blueprint Pool is unavailable';
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
    -- The Pool owns its fixed eligibility classification. Member Question metadata
    -- may be reclassified later without changing this current Pool contract.
    RETURN QUERY SELECT pool.question_pool_edit_number, member.published_question_id::text, member.question_revision_number,
                        pool.content_discipline_id, pool.content_subject_id,
                        pool.question_type::text, pool.backend::text
      FROM ple_data.question_pool AS pool
      JOIN ple_data.question_pool_member AS member USING (question_pool_id)
      WHERE pool.question_pool_id = p_public_pool_id;
END $$;

SET LOCAL ROLE ple_data_owner;

CREATE FUNCTION ple_data.validate_blueprint_pools() RETURNS trigger
LANGUAGE plpgsql SECURITY INVOKER SET search_path = pg_catalog, ple_data AS $$
DECLARE assessment_value jsonb; entry_value jsonb; pool_row ple_data.question_pool%ROWTYPE;
    public_id text;
BEGIN
    FOR assessment_value IN SELECT assessment FROM jsonb_array_elements(NEW.content -> 'modules') AS module,
        jsonb_array_elements(module -> 'assessments') AS assessment LOOP
        FOR entry_value IN SELECT entry FROM jsonb_array_elements(assessment_value #> '{content,entries}') AS entry
            WHERE entry ->> 'kind' = 'pool' LOOP
            public_id := entry_value ->> 'question_pool_id';
            SELECT * INTO pool_row FROM ple_data.question_pool WHERE question_pool_id = public_id;
            IF pool_row.question_pool_id IS NULL THEN
                RAISE EXCEPTION USING ERRCODE = '23503', MESSAGE = 'Blueprint Question Pool does not exist';
            END IF;
        END LOOP;
    END LOOP;
    RETURN NEW;
END $$;

CREATE TRIGGER blueprint_revision_owned_pools BEFORE INSERT ON ple_data.blueprint_course_revision
    FOR EACH ROW EXECUTE FUNCTION ple_data.validate_blueprint_pools();
