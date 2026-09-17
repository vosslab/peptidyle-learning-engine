-- Exact-Revision Bloom metadata foundation; publication/AI admission cutover is separate.
-- ASVS 2.2.1/2.2.2: both closed dimensions are required on each attached pair.
SET LOCAL ROLE ple_data_owner;
CREATE TYPE ple_data.bloom_cognitive_process AS ENUM (
    'Remember', 'Understand', 'Apply', 'Analyze', 'Evaluate', 'Create'
);
CREATE TYPE ple_data.bloom_knowledge_dimension AS ENUM (
    'Factual Knowledge', 'Conceptual Knowledge', 'Procedural Knowledge',
    'Metacognitive Knowledge'
);
GRANT USAGE ON TYPE ple_data.bloom_cognitive_process,
    ple_data.bloom_knowledge_dimension TO ple_private_owner, ple_api_owner;
CREATE TABLE ple_data.question_revision_bloom (
    question_id text NOT NULL,
    revision_number integer NOT NULL,
    cognitive_process ple_data.bloom_cognitive_process NOT NULL,
    knowledge_dimension ple_data.bloom_knowledge_dimension NOT NULL,
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
    cognitive_process ple_data.bloom_cognitive_process NOT NULL,
    knowledge_dimension ple_data.bloom_knowledge_dimension NOT NULL,
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
-- Pool Library projections expose only the exact pair and its correction
-- precondition through session-authorized SECURITY DEFINER readers.
CREATE POLICY question_pool_revision_bloom_api_projection ON ple_data.question_pool_revision_bloom
    FOR SELECT TO ple_api_owner USING (true);
CREATE POLICY question_pool_revision_bloom_private_insert ON ple_data.question_pool_revision_bloom
    FOR INSERT TO ple_private_owner WITH CHECK (true);
CREATE POLICY question_pool_revision_bloom_private_update ON ple_data.question_pool_revision_bloom
    FOR UPDATE TO ple_private_owner USING (true) WITH CHECK (true);
GRANT SELECT, INSERT ON TABLE ple_data.question_pool_revision_bloom TO ple_private_owner;
GRANT UPDATE (cognitive_process, knowledge_dimension, classification_edit_number)
    ON TABLE ple_data.question_pool_revision_bloom TO ple_private_owner;
GRANT SELECT (
    question_pool_id, revision_number, cognitive_process,
    knowledge_dimension, classification_edit_number
)
    ON TABLE ple_data.question_pool_revision_bloom TO ple_api_owner;
RESET ROLE;

-- AI preparation must finish before the short publication transaction begins.
-- This deliberately tiny, one-use receipt is not a queue, provider record, or
-- browser capability. Its fingerprint binds the prepared pair to the target
-- kind and exact immutable candidate content consumed by a Revision.
SET LOCAL ROLE ple_private_owner;
CREATE TYPE ple_private.bloom_preparation_target_kind AS ENUM (
    'question_revision', 'question_pool_revision'
);
GRANT USAGE ON TYPE ple_private.bloom_preparation_target_kind
    TO ple_data_owner, ple_api_owner;
CREATE TABLE ple_private.bloom_preparation_receipt (
    bloom_preparation_receipt_id uuid PRIMARY KEY,
    target_kind ple_private.bloom_preparation_target_kind NOT NULL,
    candidate_fingerprint bytea NOT NULL CHECK (octet_length(candidate_fingerprint) = 32),
    cognitive_process ple_data.bloom_cognitive_process NOT NULL,
    knowledge_dimension ple_data.bloom_knowledge_dimension NOT NULL
);
ALTER TABLE ple_private.bloom_preparation_receipt ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.bloom_preparation_receipt FORCE ROW LEVEL SECURITY;
REVOKE ALL ON TABLE ple_private.bloom_preparation_receipt FROM PUBLIC;
CREATE POLICY bloom_preparation_receipt_private_owner_access
    ON ple_private.bloom_preparation_receipt
    FOR ALL TO ple_private_owner USING (true) WITH CHECK (true);
GRANT SELECT, INSERT, DELETE ON TABLE ple_private.bloom_preparation_receipt TO ple_private_owner;
RESET ROLE;

SET LOCAL ROLE ple_private_owner;
-- Complete pair transport keeps either-dimension correction and CAS simple.
CREATE FUNCTION ple_private.validate_bloom_pair(p_cognitive_process text, p_knowledge_dimension text)
RETURNS void LANGUAGE plpgsql
SET search_path = pg_catalog AS $$
DECLARE
    validated_cognitive_process ple_data.bloom_cognitive_process;
    validated_knowledge_dimension ple_data.bloom_knowledge_dimension;
BEGIN
    IF p_cognitive_process IS NULL OR p_knowledge_dimension IS NULL THEN
        RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Bloom classification pair is invalid';
    END IF;
    BEGIN
        validated_cognitive_process := p_cognitive_process::ple_data.bloom_cognitive_process;
        validated_knowledge_dimension := p_knowledge_dimension::ple_data.bloom_knowledge_dimension;
    EXCEPTION WHEN invalid_text_representation THEN
        RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Bloom classification pair is invalid';
    END;
END
$$;
REVOKE ALL ON FUNCTION ple_private.validate_bloom_pair(text, text) FROM PUBLIC;

-- ASCII JSON serialization gives each candidate one portable SQL-owned digest.
-- The inputs are the immutable source facts of the resulting Revision, not a
-- browser-selected classification or an implementation/provider identity.
CREATE FUNCTION ple_private.question_revision_bloom_candidate_fingerprint(
    p_source_checksum text
) RETURNS bytea LANGUAGE sql IMMUTABLE
SET search_path = pg_catalog AS $$
    SELECT sha256(convert_to(jsonb_build_object(
        'sourceChecksum', p_source_checksum
    )::text, 'UTF8'))
$$;
CREATE FUNCTION ple_private.question_pool_revision_bloom_candidate_fingerprint(
    p_title text, p_description text,
    p_member_question_ids text[], p_member_revision_numbers integer[]
) RETURNS bytea LANGUAGE sql IMMUTABLE
SET search_path = pg_catalog AS $$
    SELECT sha256(convert_to(jsonb_build_object(
        'title', p_title,
        'description', p_description,
        'memberQuestionIds', to_jsonb(p_member_question_ids),
        'memberRevisionNumbers', to_jsonb(p_member_revision_numbers)
    )::text, 'UTF8'))
$$;
REVOKE ALL ON FUNCTION ple_private.question_revision_bloom_candidate_fingerprint(text),
    ple_private.question_pool_revision_bloom_candidate_fingerprint(text, text, text[], integer[]) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_private.question_revision_bloom_candidate_fingerprint(text),
    ple_private.question_pool_revision_bloom_candidate_fingerprint(text, text, text[], integer[])
    TO ple_data_owner, ple_api_owner;

CREATE FUNCTION ple_private.prepare_bloom_classification(
    p_bloom_preparation_receipt_id uuid,
    p_target_kind ple_private.bloom_preparation_target_kind,
    p_candidate_fingerprint bytea, p_cognitive_process text,
    p_knowledge_dimension text
) RETURNS void LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_private AS $$
BEGIN
    IF p_bloom_preparation_receipt_id IS NULL
       OR p_candidate_fingerprint IS NULL OR octet_length(p_candidate_fingerprint) <> 32 THEN
        RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Bloom preparation is invalid';
    END IF;
    PERFORM ple_private.validate_bloom_pair(p_cognitive_process, p_knowledge_dimension);
    INSERT INTO ple_private.bloom_preparation_receipt(
        bloom_preparation_receipt_id, target_kind, candidate_fingerprint,
        cognitive_process, knowledge_dimension
    ) VALUES (
        p_bloom_preparation_receipt_id, p_target_kind, p_candidate_fingerprint,
        p_cognitive_process::ple_data.bloom_cognitive_process,
        p_knowledge_dimension::ple_data.bloom_knowledge_dimension
    );
END
$$;
CREATE FUNCTION ple_private.consume_bloom_classification(
    p_bloom_preparation_receipt_id uuid,
    p_target_kind ple_private.bloom_preparation_target_kind,
    p_candidate_fingerprint bytea
) RETURNS TABLE (cognitive_process text, knowledge_dimension text)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_private AS $$
BEGIN
    IF p_bloom_preparation_receipt_id IS NULL
       OR p_candidate_fingerprint IS NULL OR octet_length(p_candidate_fingerprint) <> 32 THEN
        RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Bloom preparation reference is invalid';
    END IF;
    RETURN QUERY
    DELETE FROM ple_private.bloom_preparation_receipt
     WHERE bloom_preparation_receipt_id = p_bloom_preparation_receipt_id
       AND target_kind = p_target_kind
       AND candidate_fingerprint = p_candidate_fingerprint
     RETURNING bloom_preparation_receipt.cognitive_process::text,
               bloom_preparation_receipt.knowledge_dimension::text;
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Bloom preparation is absent, stale, or already consumed';
    END IF;
END
$$;
CREATE FUNCTION ple_private.attach_question_revision_bloom(
    p_bloom_preparation_receipt_id uuid, p_question_id text,
    p_revision_number integer, p_source_checksum text
) RETURNS void LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_data, ple_private AS $$
DECLARE bloom_pair record;
BEGIN
    SELECT * INTO bloom_pair FROM ple_private.consume_bloom_classification(
        p_bloom_preparation_receipt_id, 'question_revision',
        ple_private.question_revision_bloom_candidate_fingerprint(
            p_source_checksum));
    INSERT INTO ple_data.question_revision_bloom(
        question_id, revision_number, cognitive_process, knowledge_dimension,
        classification_edit_number
    ) VALUES (
        p_question_id, p_revision_number,
        bloom_pair.cognitive_process::ple_data.bloom_cognitive_process,
        bloom_pair.knowledge_dimension::ple_data.bloom_knowledge_dimension, 1
    );
END
$$;
CREATE FUNCTION ple_private.attach_question_pool_revision_bloom(
    p_bloom_preparation_receipt_id uuid, p_question_pool_id uuid,
    p_revision_number bigint, p_title text, p_description text,
    p_member_question_ids text[], p_member_revision_numbers integer[]
) RETURNS void LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_data, ple_private AS $$
DECLARE bloom_pair record;
BEGIN
    SELECT * INTO bloom_pair FROM ple_private.consume_bloom_classification(
        p_bloom_preparation_receipt_id, 'question_pool_revision',
        ple_private.question_pool_revision_bloom_candidate_fingerprint(
            p_title, p_description, p_member_question_ids,
            p_member_revision_numbers));
    INSERT INTO ple_data.question_pool_revision_bloom(
        question_pool_id, revision_number, cognitive_process, knowledge_dimension,
        classification_edit_number
    ) VALUES (
        p_question_pool_id, p_revision_number,
        bloom_pair.cognitive_process::ple_data.bloom_cognitive_process,
        bloom_pair.knowledge_dimension::ple_data.bloom_knowledge_dimension, 1
    );
END
$$;
REVOKE ALL ON FUNCTION ple_private.prepare_bloom_classification(
    uuid, ple_private.bloom_preparation_target_kind, bytea, text, text),
    ple_private.consume_bloom_classification(
        uuid, ple_private.bloom_preparation_target_kind, bytea),
    ple_private.attach_question_revision_bloom(uuid, text, integer, text),
    ple_private.attach_question_pool_revision_bloom(uuid, uuid, bigint, text, text, text[], integer[])
    FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_private.prepare_bloom_classification(
    uuid, ple_private.bloom_preparation_target_kind, bytea, text, text)
    TO ple_data_owner, ple_api_owner;
GRANT EXECUTE ON FUNCTION ple_private.consume_bloom_classification(
    uuid, ple_private.bloom_preparation_target_kind, bytea)
    TO ple_data_owner, ple_private_owner;
GRANT EXECUTE ON FUNCTION ple_private.attach_question_revision_bloom(uuid, text, integer, text),
    ple_private.attach_question_pool_revision_bloom(uuid, uuid, bigint, text, text, text[], integer[])
    TO ple_data_owner, ple_private_owner;

-- The application invokes only these typed wrappers.  They retain candidate
-- serialization and receipt writes inside the private SECURITY DEFINER path.
SET LOCAL ROLE ple_api_owner;
CREATE FUNCTION ple_api.prepare_question_revision_bloom_classification(
    p_bloom_preparation_receipt_id uuid, p_source_checksum text,
    p_cognitive_process text, p_knowledge_dimension text
) RETURNS void LANGUAGE sql SECURITY DEFINER
SET search_path = pg_catalog, ple_private AS $$
    SELECT ple_private.prepare_bloom_classification(
        p_bloom_preparation_receipt_id,
        'question_revision',
        ple_private.question_revision_bloom_candidate_fingerprint(
            p_source_checksum),
        p_cognitive_process, p_knowledge_dimension)
$$;
CREATE FUNCTION ple_api.prepare_question_pool_revision_bloom_classification(
    p_bloom_preparation_receipt_id uuid, p_title text, p_description text,
    p_member_question_ids text[], p_member_revision_numbers integer[],
    p_cognitive_process text, p_knowledge_dimension text
) RETURNS void LANGUAGE sql SECURITY DEFINER
SET search_path = pg_catalog, ple_private AS $$
    SELECT ple_private.prepare_bloom_classification(
        p_bloom_preparation_receipt_id,
        'question_pool_revision',
        ple_private.question_pool_revision_bloom_candidate_fingerprint(
            p_title, p_description, p_member_question_ids, p_member_revision_numbers),
        p_cognitive_process, p_knowledge_dimension)
$$;
REVOKE ALL ON FUNCTION ple_api.prepare_question_revision_bloom_classification(
    uuid, text, text, text),
    ple_api.prepare_question_pool_revision_bloom_classification(
        uuid, text, text, text[], integer[], text, text) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_api.prepare_question_revision_bloom_classification(
    uuid, text, text, text),
    ple_api.prepare_question_pool_revision_bloom_classification(
        uuid, text, text, text[], integer[], text, text) TO ple_app;
RESET ROLE;

SET LOCAL ROLE ple_private_owner;
CREATE FUNCTION ple_private.require_question_revision_bloom()
RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_data AS $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM ple_data.question_revision_bloom AS bloom
         WHERE bloom.question_id = NEW.question_id
           AND bloom.revision_number = NEW.revision_number
    ) THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Every Published Question Revision requires a Bloom classification';
    END IF;
    RETURN NULL;
END
$$;
CREATE FUNCTION ple_private.require_question_pool_revision_bloom()
RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_data AS $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM ple_data.question_pool_revision_bloom AS bloom
         WHERE bloom.question_pool_id = NEW.question_pool_id
           AND bloom.revision_number = NEW.revision_number
    ) THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Every Question Pool Revision requires a Bloom classification';
    END IF;
    RETURN NULL;
END
$$;
GRANT EXECUTE ON FUNCTION ple_private.require_question_revision_bloom(),
    ple_private.require_question_pool_revision_bloom() TO ple_data_owner;
RESET ROLE;
SET LOCAL ROLE ple_data_owner;
CREATE CONSTRAINT TRIGGER question_revision_requires_bloom
AFTER INSERT ON ple_data.question_revision
DEFERRABLE INITIALLY DEFERRED FOR EACH ROW
EXECUTE FUNCTION ple_private.require_question_revision_bloom();
CREATE CONSTRAINT TRIGGER question_pool_revision_requires_bloom
AFTER INSERT ON ple_data.question_pool_revision
DEFERRABLE INITIALLY DEFERRED FOR EACH ROW
EXECUTE FUNCTION ple_private.require_question_pool_revision_bloom();
RESET ROLE;
SET LOCAL ROLE ple_private_owner;
REVOKE ALL ON FUNCTION ple_private.require_question_revision_bloom(),
    ple_private.require_question_pool_revision_bloom() FROM PUBLIC;
REVOKE EXECUTE ON FUNCTION ple_private.require_question_revision_bloom(),
    ple_private.require_question_pool_revision_bloom() FROM ple_data_owner;
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
    validated_cognitive_process ple_data.bloom_cognitive_process;
    validated_knowledge_dimension ple_data.bloom_knowledge_dimension;
BEGIN
    -- ASVS 8.2.1/8.2.2/8.3.1: same current active-Instructor exact-read
    -- boundary as Library readers; no invented ownership or fork restriction.
    IF NOT ple_api.current_session_account_is_instructor() THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Bloom correction requires an active Instructor';
    END IF;
    IF p_question_id IS NULL OR p_question_id !~ '^[0-9A-HJKMNP-TV-Z]{4}-[0-9A-HJKMNP-TV-Z]{4}$'
       OR substr(p_question_id, 6, 1) <> ple_private.crockford_checksum_character(
           substr(p_question_id, 1, 4) || substr(p_question_id, 7, 3)
       )
       OR p_revision_number IS NULL OR p_revision_number <= 0
       OR p_expected_classification_edit_number IS NULL
       OR p_expected_classification_edit_number <= 0 THEN
        RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Bloom correction reference is invalid';
    END IF;
    PERFORM ple_private.validate_bloom_pair(p_cognitive_process, p_knowledge_dimension);
    validated_cognitive_process := p_cognitive_process::ple_data.bloom_cognitive_process;
    validated_knowledge_dimension := p_knowledge_dimension::ple_data.bloom_knowledge_dimension;
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
    IF validated_cognitive_process IS DISTINCT FROM current_bloom.cognitive_process
       OR validated_knowledge_dimension IS DISTINCT FROM current_bloom.knowledge_dimension THEN
        UPDATE ple_data.question_revision_bloom AS bloom
           SET cognitive_process = validated_cognitive_process,
               knowledge_dimension = validated_knowledge_dimension,
               classification_edit_number = bloom.classification_edit_number + 1
         WHERE bloom.question_id = p_question_id
           AND bloom.revision_number = p_revision_number
         RETURNING bloom.* INTO current_bloom;
    END IF;
    RETURN QUERY SELECT current_bloom.cognitive_process::text, current_bloom.knowledge_dimension::text,
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
    validated_cognitive_process ple_data.bloom_cognitive_process;
    validated_knowledge_dimension ple_data.bloom_knowledge_dimension;
BEGIN
    -- ASVS 8.2.1/8.2.2/8.3.1: same current active-Instructor exact-read
    -- boundary as Library readers; no invented ownership or fork restriction.
    IF NOT ple_api.current_session_account_is_instructor() THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Bloom correction requires an active Instructor';
    END IF;
    IF p_public_question_pool_id IS NULL OR p_public_question_pool_id !~ '^[0-9A-HJKMNP-TV-Z]{4}-[0-9A-HJKMNP-TV-Z]{4}$'
       OR substr(p_public_question_pool_id, 6, 1) <> ple_private.crockford_checksum_character(
           substr(p_public_question_pool_id, 1, 4) || substr(p_public_question_pool_id, 7, 3)
       )
       OR p_revision_number IS NULL OR p_revision_number <= 0
       OR p_expected_classification_edit_number IS NULL
       OR p_expected_classification_edit_number <= 0 THEN
        RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Bloom correction reference is invalid';
    END IF;
    PERFORM ple_private.validate_bloom_pair(p_cognitive_process, p_knowledge_dimension);
    validated_cognitive_process := p_cognitive_process::ple_data.bloom_cognitive_process;
    validated_knowledge_dimension := p_knowledge_dimension::ple_data.bloom_knowledge_dimension;
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
    IF validated_cognitive_process IS DISTINCT FROM current_bloom.cognitive_process
       OR validated_knowledge_dimension IS DISTINCT FROM current_bloom.knowledge_dimension THEN
        UPDATE ple_data.question_pool_revision_bloom AS bloom
           SET cognitive_process = validated_cognitive_process,
               knowledge_dimension = validated_knowledge_dimension,
               classification_edit_number = bloom.classification_edit_number + 1
         WHERE bloom.question_pool_id = target_pool_id
           AND bloom.revision_number = p_revision_number
         RETURNING bloom.* INTO current_bloom;
    END IF;
    RETURN QUERY SELECT current_bloom.cognitive_process::text, current_bloom.knowledge_dimension::text,
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
