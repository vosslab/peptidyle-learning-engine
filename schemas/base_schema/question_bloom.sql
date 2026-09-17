-- Exact-Revision Bloom metadata foundation; publication/AI admission cutover is separate.
-- ASVS 2.2.1/2.2.2: both closed dimensions are required on each attached pair.
SET LOCAL ROLE ple_data_owner;
CREATE TABLE ple_data.question_revision_bloom (
    question_id text NOT NULL,
    revision_number integer NOT NULL,
    cognitive_process text NOT NULL CHECK (cognitive_process IN (
        'Remember', 'Understand', 'Apply', 'Analyze', 'Evaluate', 'Create')),
    knowledge_dimension text NOT NULL CHECK (knowledge_dimension IN (
        'Factual Knowledge', 'Conceptual Knowledge', 'Procedural Knowledge',
        'Metacognitive Knowledge')),
    classification_edit_number bigint NOT NULL CHECK (classification_edit_number > 0),
    PRIMARY KEY (question_id, revision_number),
    FOREIGN KEY (question_id, revision_number)
        REFERENCES ple_data.question_revision (question_id, revision_number)
);
ALTER TABLE ple_data.question_revision_bloom ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_data.question_revision_bloom FORCE ROW LEVEL SECURITY;
REVOKE ALL ON TABLE ple_data.question_revision_bloom FROM PUBLIC;
CREATE POLICY question_revision_bloom_owner_access ON ple_data.question_revision_bloom
    FOR ALL TO ple_data_owner USING (true) WITH CHECK (true);
CREATE POLICY question_revision_bloom_private_read ON ple_data.question_revision_bloom
    FOR SELECT TO ple_private_owner USING (true);
CREATE POLICY question_revision_bloom_private_insert ON ple_data.question_revision_bloom
    FOR INSERT TO ple_private_owner WITH CHECK (true);
CREATE POLICY question_revision_bloom_private_update ON ple_data.question_revision_bloom
    FOR UPDATE TO ple_private_owner USING (true) WITH CHECK (true);
GRANT SELECT, INSERT ON TABLE ple_data.question_revision_bloom TO ple_private_owner;
GRANT UPDATE (cognitive_process, knowledge_dimension, classification_edit_number)
    ON TABLE ple_data.question_revision_bloom TO ple_private_owner;
CREATE TABLE ple_data.question_pool_revision_bloom (
    question_pool_id uuid NOT NULL,
    revision_number bigint NOT NULL,
    cognitive_process text NOT NULL CHECK (cognitive_process IN (
        'Remember', 'Understand', 'Apply', 'Analyze', 'Evaluate', 'Create')),
    knowledge_dimension text NOT NULL CHECK (knowledge_dimension IN (
        'Factual Knowledge', 'Conceptual Knowledge', 'Procedural Knowledge',
        'Metacognitive Knowledge')),
    classification_edit_number bigint NOT NULL CHECK (classification_edit_number > 0),
    PRIMARY KEY (question_pool_id, revision_number),
    FOREIGN KEY (question_pool_id, revision_number)
        REFERENCES ple_data.question_pool_revision (question_pool_id, revision_number)
);
ALTER TABLE ple_data.question_pool_revision_bloom ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_data.question_pool_revision_bloom FORCE ROW LEVEL SECURITY;
REVOKE ALL ON TABLE ple_data.question_pool_revision_bloom FROM PUBLIC;
CREATE POLICY question_pool_revision_bloom_owner_access ON ple_data.question_pool_revision_bloom
    FOR ALL TO ple_data_owner USING (true) WITH CHECK (true);
CREATE POLICY question_pool_revision_bloom_private_read ON ple_data.question_pool_revision_bloom
    FOR SELECT TO ple_private_owner USING (true);
CREATE POLICY question_pool_revision_bloom_private_insert ON ple_data.question_pool_revision_bloom
    FOR INSERT TO ple_private_owner WITH CHECK (true);
CREATE POLICY question_pool_revision_bloom_private_update ON ple_data.question_pool_revision_bloom
    FOR UPDATE TO ple_private_owner USING (true) WITH CHECK (true);
GRANT SELECT, INSERT ON TABLE ple_data.question_pool_revision_bloom TO ple_private_owner;
GRANT UPDATE (cognitive_process, knowledge_dimension, classification_edit_number)
    ON TABLE ple_data.question_pool_revision_bloom TO ple_private_owner;
RESET ROLE;

SET LOCAL ROLE ple_private_owner;
-- Complete pair transport keeps either-dimension correction and CAS simple.
CREATE FUNCTION ple_private.validate_bloom_pair(p_cognitive_process text, p_knowledge_dimension text)
RETURNS void LANGUAGE plpgsql
SET search_path = pg_catalog AS $$
BEGIN
    IF p_cognitive_process IS NULL OR p_knowledge_dimension IS NULL
       OR p_cognitive_process NOT IN (
           'Remember', 'Understand', 'Apply', 'Analyze', 'Evaluate', 'Create')
       OR p_knowledge_dimension NOT IN (
           'Factual Knowledge', 'Conceptual Knowledge', 'Procedural Knowledge',
           'Metacognitive Knowledge') THEN
        RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Bloom classification pair is invalid';
    END IF;
END
$$;
REVOKE ALL ON FUNCTION ple_private.validate_bloom_pair(text, text) FROM PUBLIC;

-- Trusted initialization seam only. No browser/application capability and no upsert:
-- a late initializer cannot overwrite an Instructor correction. The caller owns
-- binding prepared classification to exact source; SQL makes no model-origin claim.
CREATE FUNCTION ple_private.initialize_question_revision_bloom(
    p_question_id text, p_revision_number integer,
    p_cognitive_process text, p_knowledge_dimension text
) RETURNS void LANGUAGE sql SECURITY DEFINER
SET search_path = pg_catalog, ple_data AS $$
    INSERT INTO ple_data.question_revision_bloom (
        question_id, revision_number, cognitive_process, knowledge_dimension,
        classification_edit_number)
    VALUES (p_question_id, p_revision_number, p_cognitive_process, p_knowledge_dimension, 1)
$$;
REVOKE ALL ON FUNCTION ple_private.initialize_question_revision_bloom(
    text, integer, text, text) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_private.initialize_question_revision_bloom(
    text, integer, text, text) TO ple_data_owner;

-- Trusted initialization seam only. No browser/application capability and no upsert:
-- a late initializer cannot overwrite an Instructor correction. The caller owns
-- binding prepared classification to exact source; SQL makes no model-origin claim.
CREATE FUNCTION ple_private.initialize_question_pool_revision_bloom(
    p_question_pool_id uuid, p_revision_number bigint,
    p_cognitive_process text, p_knowledge_dimension text
) RETURNS void LANGUAGE sql SECURITY DEFINER
SET search_path = pg_catalog, ple_data AS $$
    INSERT INTO ple_data.question_pool_revision_bloom (
        question_pool_id, revision_number, cognitive_process, knowledge_dimension,
        classification_edit_number)
    VALUES (p_question_pool_id, p_revision_number, p_cognitive_process, p_knowledge_dimension, 1)
$$;
REVOKE ALL ON FUNCTION ple_private.initialize_question_pool_revision_bloom(
    uuid, bigint, text, text) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_private.initialize_question_pool_revision_bloom(
    uuid, bigint, text, text) TO ple_data_owner;
RESET ROLE;

SET LOCAL ROLE ple_private_owner;
CREATE FUNCTION ple_private.correct_question_revision_bloom(
    p_question_id text, p_revision_number integer,
    p_expected_classification_edit_number bigint,
    p_cognitive_process text, p_knowledge_dimension text
) RETURNS TABLE (cognitive_process text, knowledge_dimension text, classification_edit_number bigint)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
DECLARE
    current_bloom ple_data.question_revision_bloom%ROWTYPE;
BEGIN
    -- ASVS 8.2.1/8.2.2/8.3.1: same current active-Instructor exact-read
    -- boundary as Library readers; no invented ownership or fork restriction.
    IF NOT ple_api.current_session_account_is_instructor() THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Bloom correction requires an active Instructor';
    END IF;
    IF p_question_id IS NULL OR p_question_id !~ '^[0-9A-HJKMNP-TV-Z]{8}$'
       OR p_revision_number IS NULL OR p_revision_number <= 0
       OR p_expected_classification_edit_number IS NULL
       OR p_expected_classification_edit_number <= 0 THEN
        RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Bloom correction reference is invalid';
    END IF;
    PERFORM ple_private.validate_bloom_pair(p_cognitive_process, p_knowledge_dimension);
    IF NOT EXISTS (SELECT 1 FROM ple_private.question_library_entries(
        p_question_id, p_revision_number, false)) THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Bloom correction target is unavailable';
    END IF;
    -- ASVS 15.4.2: pair-level row lock and CAS serialize either-dimension edits.
    SELECT bloom.* INTO current_bloom FROM ple_data.question_revision_bloom AS bloom
     WHERE bloom.question_id = p_question_id
       AND bloom.revision_number = p_revision_number FOR UPDATE;
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Bloom correction target is unavailable';
    END IF;
    IF current_bloom.classification_edit_number <> p_expected_classification_edit_number THEN
        RAISE EXCEPTION USING ERRCODE = '40001', MESSAGE = 'Bloom classification Edit Number is stale';
    END IF;
    -- An exact no-op keeps its token, but a stale no-op still refuses.
    IF p_cognitive_process IS DISTINCT FROM current_bloom.cognitive_process
       OR p_knowledge_dimension IS DISTINCT FROM current_bloom.knowledge_dimension THEN
        UPDATE ple_data.question_revision_bloom AS bloom
           SET cognitive_process = p_cognitive_process,
               knowledge_dimension = p_knowledge_dimension,
               classification_edit_number = bloom.classification_edit_number + 1
         WHERE bloom.question_id = p_question_id
           AND bloom.revision_number = p_revision_number
         RETURNING bloom.* INTO current_bloom;
    END IF;
    RETURN QUERY SELECT current_bloom.cognitive_process, current_bloom.knowledge_dimension,
                        current_bloom.classification_edit_number;
END
$$;
REVOKE ALL ON FUNCTION ple_private.correct_question_revision_bloom(
    text, integer, bigint, text, text) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_private.correct_question_revision_bloom(
    text, integer, bigint, text, text) TO ple_api_owner;
RESET ROLE;

SET LOCAL ROLE ple_api_owner;
CREATE FUNCTION ple_api.correct_question_revision_bloom(
    p_question_id text, p_revision_number integer,
    p_expected_classification_edit_number bigint,
    p_cognitive_process text, p_knowledge_dimension text
) RETURNS TABLE (cognitive_process text, knowledge_dimension text, classification_edit_number bigint)
LANGUAGE sql SECURITY DEFINER
SET search_path = pg_catalog, ple_private AS $$
    SELECT * FROM ple_private.correct_question_revision_bloom(
        p_question_id, p_revision_number, p_expected_classification_edit_number,
        p_cognitive_process, p_knowledge_dimension)
$$;
REVOKE ALL ON FUNCTION ple_api.correct_question_revision_bloom(
    text, integer, bigint, text, text) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_api.correct_question_revision_bloom(
    text, integer, bigint, text, text) TO ple_app;
RESET ROLE;

SET LOCAL ROLE ple_private_owner;
CREATE FUNCTION ple_private.correct_question_pool_revision_bloom(
    p_public_question_pool_id text, p_revision_number bigint,
    p_expected_classification_edit_number bigint,
    p_cognitive_process text, p_knowledge_dimension text
) RETURNS TABLE (cognitive_process text, knowledge_dimension text, classification_edit_number bigint)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
DECLARE
    current_bloom ple_data.question_pool_revision_bloom%ROWTYPE;
    target_pool_id uuid;
BEGIN
    -- ASVS 8.2.1/8.2.2/8.3.1: same current active-Instructor exact-read
    -- boundary as Library readers; no invented ownership or fork restriction.
    IF NOT ple_api.current_session_account_is_instructor() THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Bloom correction requires an active Instructor';
    END IF;
    IF p_public_question_pool_id IS NULL OR p_public_question_pool_id !~ '^[0-9A-HJKMNP-TV-Z]{8}$'
       OR p_revision_number IS NULL OR p_revision_number <= 0
       OR p_expected_classification_edit_number IS NULL
       OR p_expected_classification_edit_number <= 0 THEN
        RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Bloom correction reference is invalid';
    END IF;
    PERFORM ple_private.validate_bloom_pair(p_cognitive_process, p_knowledge_dimension);
    SELECT pool.question_pool_id INTO target_pool_id
      FROM ple_data.question_pool AS pool
      JOIN ple_data.question_pool_revision AS revision
        ON revision.question_pool_id = pool.question_pool_id
       AND revision.revision_number = p_revision_number
     WHERE pool.public_question_pool_id = p_public_question_pool_id;
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Bloom correction target is unavailable';
    END IF;
    -- ASVS 15.4.2: pair-level row lock and CAS serialize either-dimension edits.
    SELECT bloom.* INTO current_bloom FROM ple_data.question_pool_revision_bloom AS bloom
     WHERE bloom.question_pool_id = target_pool_id
       AND bloom.revision_number = p_revision_number FOR UPDATE;
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Bloom correction target is unavailable';
    END IF;
    IF current_bloom.classification_edit_number <> p_expected_classification_edit_number THEN
        RAISE EXCEPTION USING ERRCODE = '40001', MESSAGE = 'Bloom classification Edit Number is stale';
    END IF;
    -- An exact no-op keeps its token, but a stale no-op still refuses.
    IF p_cognitive_process IS DISTINCT FROM current_bloom.cognitive_process
       OR p_knowledge_dimension IS DISTINCT FROM current_bloom.knowledge_dimension THEN
        UPDATE ple_data.question_pool_revision_bloom AS bloom
           SET cognitive_process = p_cognitive_process,
               knowledge_dimension = p_knowledge_dimension,
               classification_edit_number = bloom.classification_edit_number + 1
         WHERE bloom.question_pool_id = target_pool_id
           AND bloom.revision_number = p_revision_number
         RETURNING bloom.* INTO current_bloom;
    END IF;
    RETURN QUERY SELECT current_bloom.cognitive_process, current_bloom.knowledge_dimension,
                        current_bloom.classification_edit_number;
END
$$;
REVOKE ALL ON FUNCTION ple_private.correct_question_pool_revision_bloom(
    text, bigint, bigint, text, text) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_private.correct_question_pool_revision_bloom(
    text, bigint, bigint, text, text) TO ple_api_owner;
RESET ROLE;

SET LOCAL ROLE ple_api_owner;
CREATE FUNCTION ple_api.correct_question_pool_revision_bloom(
    p_public_question_pool_id text, p_revision_number bigint,
    p_expected_classification_edit_number bigint,
    p_cognitive_process text, p_knowledge_dimension text
) RETURNS TABLE (cognitive_process text, knowledge_dimension text, classification_edit_number bigint)
LANGUAGE sql SECURITY DEFINER
SET search_path = pg_catalog, ple_private AS $$
    SELECT * FROM ple_private.correct_question_pool_revision_bloom(
        p_public_question_pool_id, p_revision_number, p_expected_classification_edit_number,
        p_cognitive_process, p_knowledge_dimension)
$$;
REVOKE ALL ON FUNCTION ple_api.correct_question_pool_revision_bloom(
    text, bigint, bigint, text, text) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_api.correct_question_pool_revision_bloom(
    text, bigint, bigint, text, text) TO ple_app;
RESET ROLE;
