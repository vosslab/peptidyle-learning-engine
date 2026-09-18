-- Functions, triggers, and views from question_pools.sql.

SET LOCAL ROLE ple_data_owner;

CREATE TRIGGER question_pool_public_id_is_reserved
BEFORE INSERT ON ple_data.question_pool
FOR EACH ROW EXECUTE FUNCTION ple_private.reserve_public_id_from_trigger(
    'question_pool', 'question_pool_id'
);

CREATE FUNCTION ple_data.reject_question_pool_immutable_change()
RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_data AS $$
BEGIN
    RAISE EXCEPTION USING ERRCODE = '55000',
        MESSAGE = 'Question Pool identity is immutable';
END
$$;

CREATE FUNCTION ple_data.validate_question_pool_lineage_update()
RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_data AS $$
BEGIN
    IF NEW.question_pool_id <> OLD.question_pool_id
       OR NEW.source_question_pool_id IS DISTINCT FROM OLD.source_question_pool_id
       OR NEW.created_at <> OLD.created_at
       OR NEW.created_in_transaction IS DISTINCT FROM OLD.created_in_transaction
       OR NEW.title IS DISTINCT FROM OLD.title
       OR NEW.description IS DISTINCT FROM OLD.description
       OR NEW.content_discipline_id IS DISTINCT FROM OLD.content_discipline_id
       OR NEW.content_subject_id IS DISTINCT FROM OLD.content_subject_id
       OR NEW.content_topic_id IS DISTINCT FROM OLD.content_topic_id
       OR NEW.content_subtopic_id IS DISTINCT FROM OLD.content_subtopic_id
       OR NEW.tags IS DISTINCT FROM OLD.tags
       OR NEW.question_pool_edit_number <> OLD.question_pool_edit_number + 1 THEN
        RAISE EXCEPTION USING ERRCODE = '55000',
            MESSAGE = 'Question Pool identity is immutable and member-list save must advance Edit Number once';
    END IF;
    RETURN NEW;
END
$$;

CREATE FUNCTION ple_data.validate_question_pool_members()
RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_data AS $$
DECLARE member_count integer; last_position integer; pool_id text;
BEGIN
    pool_id := COALESCE(NEW.question_pool_id, OLD.question_pool_id);
    SELECT count(*), max(member_position) INTO member_count, last_position
      FROM ple_data.question_pool_member
     WHERE question_pool_id = pool_id;
    IF member_count = 0 OR last_position <> member_count THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Question Pool requires a nonempty contiguous ordered member list';
    END IF;
    RETURN NULL;
END
$$;

CREATE FUNCTION ple_data.validate_question_pool_member_insert()
RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_data AS $$
DECLARE existing_count integer; backend_name ple_data.question_backend;
BEGIN
    SELECT count(*) INTO existing_count FROM ple_data.question_pool_member
     WHERE question_pool_id = NEW.question_pool_id;
    IF NEW.member_position <> existing_count + 1 THEN
        RAISE EXCEPTION USING ERRCODE = '55000',
            MESSAGE = 'Question Pool member set is ordered';
    END IF;
    SELECT revision.backend INTO backend_name
      FROM ple_data.question_revision AS revision
     WHERE revision.published_question_id = NEW.published_question_id
       AND revision.revision_number = NEW.question_revision_number;
    IF NOT ple_private.question_backend_is_supported_for_production(backend_name) THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Question Pool members require a current production Question Backend';
    END IF;
    RETURN NEW;
END
$$;

CREATE TRIGGER question_pool_identity_is_immutable
BEFORE DELETE ON ple_data.question_pool
FOR EACH ROW EXECUTE FUNCTION ple_data.reject_question_pool_immutable_change();

CREATE TRIGGER question_pool_append_updates_lineage
BEFORE UPDATE ON ple_data.question_pool
FOR EACH ROW EXECUTE FUNCTION ple_data.validate_question_pool_lineage_update();

CREATE TRIGGER question_pool_member_insert_is_ordered
BEFORE INSERT ON ple_data.question_pool_member
FOR EACH ROW EXECUTE FUNCTION ple_data.validate_question_pool_member_insert();

CREATE CONSTRAINT TRIGGER question_pool_has_members
AFTER INSERT OR DELETE ON ple_data.question_pool_member
DEFERRABLE INITIALLY DEFERRED FOR EACH ROW
EXECUTE FUNCTION ple_data.validate_question_pool_members();

-- The later typed trusted server command supplies an already checksum-validated
-- canonical public ID. This schema is not an issuer; the narrow session-bound
-- API wrapper below is the only application creation capability. Pool
-- ownership/content rules belong to the later published-Pool closure.
CREATE FUNCTION ple_data.create_question_pool(
    p_question_pool_id text,
    p_member_question_ids text[], p_member_revision_numbers integer[],
    p_interchangeability_attested boolean, p_title text, p_description text
) RETURNS TABLE (
    question_pool_id text, question_pool_edit_number bigint
)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data AS $$
DECLARE actor_id text; created_at timestamptz := pg_catalog.clock_timestamp();
    first_metadata ple_data.published_question_metadata%ROWTYPE;
BEGIN
    IF p_question_pool_id IS NULL
       OR p_question_pool_id !~ '^[0-9A-HJKMNP-TV-Z]{4}-[0-9A-HJKMNP-TV-Z]{4}$'
       OR substr(p_question_pool_id, 6, 1) <> ple_private.crockford_checksum_character(
           substr(p_question_pool_id, 1, 4) || substr(p_question_pool_id, 7, 3)
       )
       OR p_member_question_ids IS NULL OR p_member_revision_numbers IS NULL
       OR cardinality(p_member_question_ids) IS NULL OR cardinality(p_member_question_ids) = 0
       OR cardinality(p_member_question_ids) > 1024
       OR cardinality(p_member_question_ids) <> cardinality(p_member_revision_numbers)
       OR p_interchangeability_attested IS DISTINCT FROM true
       OR p_title IS NULL OR p_title <> btrim(p_title)
       OR char_length(p_title) NOT BETWEEN 1 AND 512 OR p_title ~ '[[:cntrl:]]'
       OR p_description IS NULL OR p_description <> btrim(p_description)
       OR char_length(p_description) NOT BETWEEN 1 AND 4000 OR p_description ~ '[[:cntrl:]]' THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Question Pool creation requires a canonical ID, nonempty ordered members, and interchangeability attestation';
    END IF;
    IF NOT ple_api.current_session_account_is_instructor() THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Active Instructor authority is required for Question Pool creation';
    END IF;
    actor_id := ple_api.current_session_account_id();
    -- ASVS 2.2.2/2.3.3/2.3.4/15.4.2/15.4.3: canonical-order
    -- FOR SHARE locks every matching lineage and live metadata row regardless
    -- of availability, ordering post-lock admission with lifecycle and metadata edits.
    PERFORM metadata.published_question_id
      FROM ple_data.published_question_metadata AS metadata
     JOIN ple_data.published_question AS lineage USING (published_question_id)
     WHERE metadata.published_question_id = ANY(p_member_question_ids)
     ORDER BY metadata.published_question_id
     FOR SHARE OF metadata, lineage;
    SELECT metadata.* INTO first_metadata FROM ple_data.published_question_metadata AS metadata
     WHERE metadata.published_question_id = p_member_question_ids[array_lower(p_member_question_ids, 1)];
    IF NOT FOUND OR EXISTS (
        SELECT 1 FROM unnest(p_member_question_ids) AS member(published_question_id)
        LEFT JOIN ple_data.published_question AS lineage USING (published_question_id)
        LEFT JOIN ple_data.published_question_metadata AS metadata USING (published_question_id)
        WHERE lineage.published_question_id IS NULL
           OR lineage.availability <> 'available'
           OR metadata.published_question_id IS NULL
    ) THEN
        RAISE EXCEPTION USING ERRCODE = '23503', MESSAGE = 'Question Pool member is unavailable';
    END IF;
    IF EXISTS (
        SELECT 1
          FROM unnest(p_member_question_ids, p_member_revision_numbers)
               AS member(published_question_id, revision_number)
          JOIN ple_data.question_revision AS revision
            ON revision.published_question_id = member.published_question_id
           AND revision.revision_number = member.revision_number
         WHERE NOT ple_private.question_backend_is_supported_for_production(revision.backend)
    ) THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Question Pool members require a current production Question Backend';
    END IF;
    IF EXISTS (
        SELECT 1 FROM ple_data.published_question_metadata AS metadata
         WHERE metadata.published_question_id = ANY(p_member_question_ids)
           AND (metadata.content_discipline_id <> first_metadata.content_discipline_id
                OR metadata.content_subject_id <> first_metadata.content_subject_id)
    ) THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Question Pool members must share the established Discipline and Subject';
    END IF;
    INSERT INTO ple_data.question_pool(
        question_pool_id, question_pool_edit_number, created_at,
        interchangeability_attested_by_account_id, interchangeability_attested_at,
        title, description, content_discipline_id, content_subject_id
    ) VALUES (p_question_pool_id, 1, created_at, actor_id, created_at,
        p_title, p_description, first_metadata.content_discipline_id, first_metadata.content_subject_id);
    INSERT INTO ple_data.question_pool_member(
        question_pool_id, member_position, published_question_id, question_revision_number, created_at
    )
    SELECT p_question_pool_id, member.ordinality::integer, member.published_question_id,
           p_member_revision_numbers[member.ordinality], created_at
      FROM unnest(p_member_question_ids) WITH ORDINALITY AS member(published_question_id, ordinality)
     ORDER BY member.ordinality;
    RETURN QUERY SELECT pool.question_pool_id, pool.question_pool_edit_number
      FROM ple_data.question_pool AS pool WHERE pool.question_pool_id = p_question_pool_id;
END
$$;

CREATE FUNCTION ple_data.save_question_pool_members(
    p_question_pool_id text, p_expected_question_pool_edit_number bigint,
    p_member_question_ids text[], p_member_revision_numbers integer[],
    p_interchangeability_attested boolean
) RETURNS TABLE (question_pool_edit_number bigint) LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data AS $$
DECLARE pool_row ple_data.question_pool%ROWTYPE; actor_id text;
    created_at timestamptz := pg_catalog.clock_timestamp();
    current_question_ids text[]; current_revision_numbers integer[];
BEGIN
    IF p_question_pool_id IS NULL OR p_expected_question_pool_edit_number IS NULL
       OR p_member_question_ids IS NULL OR p_member_revision_numbers IS NULL
       OR cardinality(p_member_question_ids) IS NULL OR cardinality(p_member_question_ids) = 0
       OR cardinality(p_member_question_ids) <> cardinality(p_member_revision_numbers)
       OR p_interchangeability_attested IS DISTINCT FROM true THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Question Pool member save requires metadata ETag, nonempty ordered members, and interchangeability attestation';
    END IF;
    IF NOT ple_api.current_session_account_is_instructor() THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Active Instructor authority is required for Question Pool member save';
    END IF;
    SELECT * INTO pool_row FROM ple_data.question_pool
     WHERE question_pool_id = p_question_pool_id FOR UPDATE;
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '23503', MESSAGE = 'Question Pool does not exist';
    END IF;
    IF pool_row.question_pool_edit_number <> p_expected_question_pool_edit_number THEN
        RAISE EXCEPTION USING ERRCODE = '40001',
            MESSAGE = 'Question Pool metadata ETag is stale';
    END IF;
    SELECT array_agg(member.published_question_id ORDER BY member.member_position),
           array_agg(member.question_revision_number ORDER BY member.member_position)
      INTO current_question_ids, current_revision_numbers
      FROM ple_data.question_pool_member AS member
     WHERE member.question_pool_id = p_question_pool_id;
    IF current_question_ids IS NOT DISTINCT FROM p_member_question_ids
       AND current_revision_numbers IS NOT DISTINCT FROM p_member_revision_numbers THEN
        question_pool_edit_number := pool_row.question_pool_edit_number;
        RETURN NEXT;
        RETURN;
    END IF;
    -- Admission-time invariant: retained Question IDs (including changed exact
    -- pins) do not re-admit. Remove/readd checks current Question metadata.
    -- Question reclassification never changes or vetoes existing Pool state.
    PERFORM metadata.published_question_id FROM ple_data.published_question_metadata AS metadata
     WHERE metadata.published_question_id = ANY(p_member_question_ids)
       AND NOT EXISTS (
           SELECT 1 FROM ple_data.question_pool_member AS previous
            WHERE previous.question_pool_id = p_question_pool_id
              AND previous.published_question_id = metadata.published_question_id
       )
     ORDER BY metadata.published_question_id FOR SHARE;
    IF EXISTS (
        SELECT 1 FROM unnest(p_member_question_ids) AS member(published_question_id)
        LEFT JOIN ple_data.published_question_metadata AS metadata USING (published_question_id)
         WHERE NOT EXISTS (
             SELECT 1 FROM ple_data.question_pool_member AS previous
              WHERE previous.question_pool_id = p_question_pool_id
                AND previous.published_question_id = member.published_question_id
         ) AND (metadata.published_question_id IS NULL
                OR metadata.content_discipline_id <> pool_row.content_discipline_id
                OR metadata.content_subject_id <> pool_row.content_subject_id)
    ) THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'New Question Pool members must share the established Discipline and Subject';
    END IF;
    IF EXISTS (
        SELECT 1
          FROM unnest(p_member_question_ids, p_member_revision_numbers)
               AS member(published_question_id, revision_number)
          JOIN ple_data.question_revision AS revision
            ON revision.published_question_id = member.published_question_id
           AND revision.revision_number = member.revision_number
         WHERE NOT ple_private.question_backend_is_supported_for_production(revision.backend)
    ) THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Question Pool members require a current production Question Backend';
    END IF;
    actor_id := ple_api.current_session_account_id();
    DELETE FROM ple_data.question_pool_member
     WHERE question_pool_id = p_question_pool_id;
    INSERT INTO ple_data.question_pool_member(
        question_pool_id, member_position, published_question_id, question_revision_number, created_at
    )
    SELECT p_question_pool_id, member.ordinality::integer, member.published_question_id,
           p_member_revision_numbers[member.ordinality], created_at
      FROM unnest(p_member_question_ids) WITH ORDINALITY AS member(published_question_id, ordinality)
     ORDER BY member.ordinality;
    UPDATE ple_data.question_pool
       SET question_pool_edit_number = pool_row.question_pool_edit_number + 1,
           interchangeability_attested_by_account_id = actor_id,
           interchangeability_attested_at = created_at,
           updated_on = CURRENT_DATE
     WHERE question_pool_id = p_question_pool_id
     RETURNING ple_data.question_pool.question_pool_edit_number INTO question_pool_edit_number;
    RETURN NEXT;
END
$$;

-- This private construction primitive has no application/API grant. Its
-- authorized wrappers retain the source Pool's actual interchangeability
-- attestation; creating a fork does not re-attest the source member set.
CREATE FUNCTION ple_data.construct_question_pool_fork(
    p_question_pool_id text,
    p_source_question_pool_id text
) RETURNS TABLE (
    question_pool_id text, question_pool_edit_number bigint
)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data AS $$
DECLARE created_at timestamptz := pg_catalog.clock_timestamp();
DECLARE source_metadata ple_data.question_pool%ROWTYPE;
BEGIN
    IF p_question_pool_id IS NULL
       OR p_question_pool_id !~ '^[0-9A-HJKMNP-TV-Z]{4}-[0-9A-HJKMNP-TV-Z]{4}$'
       OR substr(p_question_pool_id, 6, 1) <> ple_private.crockford_checksum_character(
           substr(p_question_pool_id, 1, 4) || substr(p_question_pool_id, 7, 3)
       )
       OR p_source_question_pool_id IS NULL THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Question Pool fork is invalid';
    END IF;
    SELECT source_pool.* INTO source_metadata FROM ple_data.question_pool AS source_pool
     WHERE source_pool.question_pool_id = p_source_question_pool_id FOR SHARE;
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '23503',
            MESSAGE = 'Question Pool source does not exist';
    END IF;
    IF EXISTS (
        SELECT 1
          FROM ple_data.question_pool_member AS member
          JOIN ple_data.question_revision AS revision
            ON revision.published_question_id = member.published_question_id
           AND revision.revision_number = member.question_revision_number
         WHERE member.question_pool_id = p_source_question_pool_id
           AND NOT ple_private.question_backend_is_supported_for_production(revision.backend)
    ) THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Question Pool source has an unavailable Question Backend';
    END IF;
    INSERT INTO ple_data.question_pool(
        question_pool_id, question_pool_edit_number,
        source_question_pool_id, created_at,
        interchangeability_attested_by_account_id, interchangeability_attested_at,
        title, description, content_discipline_id, content_subject_id, content_topic_id, content_subtopic_id, tags
    ) VALUES (
        p_question_pool_id, 1,
        p_source_question_pool_id, created_at,
        source_metadata.interchangeability_attested_by_account_id,
        source_metadata.interchangeability_attested_at,
        source_metadata.title, source_metadata.description, source_metadata.content_discipline_id,
        source_metadata.content_subject_id, source_metadata.content_topic_id, source_metadata.content_subtopic_id,
        source_metadata.tags
    );
    INSERT INTO ple_data.question_pool_member(
        question_pool_id, member_position, published_question_id, question_revision_number, created_at
    )
    SELECT p_question_pool_id, member.member_position, member.published_question_id,
           member.question_revision_number, created_at
      FROM ple_data.question_pool_member AS member
     WHERE member.question_pool_id = p_source_question_pool_id
     ORDER BY member.member_position;
    RETURN QUERY SELECT pool.question_pool_id, pool.question_pool_edit_number
      FROM ple_data.question_pool AS pool WHERE pool.question_pool_id = p_question_pool_id;
END
$$;

-- Importing a reusable Pool into an Assessment never aliases the published
-- lineage. The ordinary route remains Instructor-only.
CREATE FUNCTION ple_data.fork_question_pool_revision(
    p_question_pool_id text,
    p_source_question_pool_id text
) RETURNS TABLE (
    question_pool_id text, question_pool_edit_number bigint
)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data AS $$
BEGIN
    IF p_question_pool_id IS NULL
       OR p_question_pool_id !~ '^[0-9A-HJKMNP-TV-Z]{4}-[0-9A-HJKMNP-TV-Z]{4}$'
       OR substr(p_question_pool_id, 6, 1) <> ple_private.crockford_checksum_character(
           substr(p_question_pool_id, 1, 4) || substr(p_question_pool_id, 7, 3)
       )
       OR p_source_question_pool_id IS NULL
       OR NOT ple_api.current_session_account_is_instructor() THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Question Pool fork is unavailable';
    END IF;
    RETURN QUERY SELECT * FROM ple_data.construct_question_pool_fork(
        p_question_pool_id, p_source_question_pool_id
    );
END
$$;

-- Course adoption is the one additional internal context where a Sysadmin may
-- create a Course for an assigned Instructor. It has no standalone API grant:
-- the already-authorized atomic Course creation boundary is its only caller.
CREATE FUNCTION ple_data.fork_question_pool_revision_for_course_adoption(
    p_question_pool_id text,
    p_source_question_pool_id text
) RETURNS TABLE (
    question_pool_id text, question_pool_edit_number bigint
)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data AS $$
BEGIN
    IF NOT (ple_api.current_session_account_is_instructor()
            OR ple_api.current_session_account_is_sysadmin()) THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Course adoption Question Pool fork is unavailable';
    END IF;
    RETURN QUERY SELECT * FROM ple_data.construct_question_pool_fork(
        p_question_pool_id, p_source_question_pool_id
    );
END
$$;

SET LOCAL ROLE ple_api_owner;

-- The application receives only this session-bound capability. The data-owner
-- procedure remains private, and it derives active Instructor authority and
-- the attesting Account from the installed session rather than browser input.
CREATE FUNCTION ple_api.create_question_pool(
    p_question_pool_id text,
    p_member_question_ids text[], p_member_revision_numbers integer[],
    p_interchangeability_attested boolean, p_title text, p_description text
) RETURNS TABLE (
    question_pool_id text, question_pool_edit_number bigint
) LANGUAGE sql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data AS $$
    SELECT * FROM ple_data.create_question_pool(
        p_question_pool_id, p_member_question_ids,
        p_member_revision_numbers, p_interchangeability_attested, p_title, p_description)
$$;

SET LOCAL ROLE ple_data_owner;

-- A deliberately narrow, answer-free public-reference projection. It names
-- no internal UUID and does not make Pool content, membership, selection,
-- ownership, or lifecycle state visible. C355 may consume these stable IDs;
-- it must supply all selection semantics separately.
CREATE FUNCTION ple_data.list_published_content_identities()
RETURNS TABLE (
    content_kind text,
    public_id text,
    current_revision_number bigint
) LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data AS $$
BEGIN
    IF NOT ple_api.current_session_account_is_instructor() THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Active Instructor authority is required for published content identities';
    END IF;
    RETURN QUERY
    SELECT 'question'::text,
           question.published_question_id,
           max(revision.revision_number)::bigint
      FROM ple_data.published_question AS question
      JOIN ple_data.question_revision AS revision ON revision.published_question_id = question.published_question_id
     GROUP BY question.published_question_id
    UNION ALL
    SELECT 'pool'::text,
           pool.question_pool_id,
           pool.question_pool_edit_number
      FROM ple_data.question_pool AS pool
     ORDER BY 1, 2;
END
$$;

SET LOCAL ROLE ple_api_owner;

CREATE FUNCTION ple_api.list_published_content_identities()
RETURNS TABLE (
    content_kind text,
    public_id text,
    current_revision_number bigint
) LANGUAGE sql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data AS $$
    SELECT * FROM ple_data.list_published_content_identities()
$$;

-- The server resolves an author-visible reusable Pool identity once, under
-- the installed Instructor session. It never accepts a caller-selected
-- Revision; every published Pool lineage, including a child fork, is reusable.
CREATE FUNCTION ple_api.resolve_current_published_question_pool(p_question_pool_id text)
RETURNS TABLE (question_pool_id text, question_pool_edit_number bigint)
LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data AS $$
    SELECT pool.question_pool_id, pool.question_pool_edit_number
      FROM ple_data.question_pool AS pool
     WHERE ple_api.current_session_account_is_instructor()
       AND pool.question_pool_id = p_question_pool_id
$$;

-- ASVS V1.2/V2.2/V8.3: parameters remain typed SQL values, bounds are
-- enforced at the capability boundary, and active-Instructor-or-Sysadmin read authorization is
-- checked before globally published Pool facts are projected.
CREATE FUNCTION ple_api.list_published_question_pools(
    p_after text, p_page_size integer,
    p_discipline_uuid uuid, p_subject_uuid uuid, p_topic_uuid uuid,
    p_subtopic_uuid uuid, p_cross_discipline boolean,
    p_terms jsonb, p_tags text[],
    p_bloom_cognitive_process text, p_bloom_knowledge_dimension text
)
RETURNS TABLE (
    question_pool_id text,
    question_pool_edit_number bigint,
    member_count integer,
    title text, description text, content_discipline_id uuid, discipline_name text,
    discipline_is_retired boolean, content_subject_id uuid,
    content_topic_id uuid, content_subtopic_id uuid, tags text[],
    bloom_cognitive_process text, bloom_knowledge_dimension text,
    bloom_classification_edit_number bigint,
    bloom_cognitive_process_counts bigint[], bloom_knowledge_dimension_counts bigint[]
) LANGUAGE plpgsql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data AS $$
BEGIN
    -- ASVS 8.2.1: preserve empty unauthorized discovery before invoking
    -- the shared vocabulary readers, whose own boundary rejects those sessions.
    IF NOT (ple_api.current_session_account_is_instructor()
            OR ple_api.current_session_account_has_platform_administration()) THEN
        RETURN;
    END IF;
    IF p_page_size NOT BETWEEN 1 AND 100
       OR (p_subject_uuid IS NOT NULL AND p_discipline_uuid IS NULL)
       OR (p_topic_uuid IS NOT NULL AND p_subject_uuid IS NULL)
       OR (p_subtopic_uuid IS NOT NULL AND p_topic_uuid IS NULL)
       OR (p_cross_discipline AND (p_discipline_uuid IS NULL OR p_subject_uuid IS NULL))
       OR (p_bloom_cognitive_process IS NOT NULL AND p_bloom_cognitive_process NOT IN (
           'Remember', 'Understand', 'Apply', 'Analyze', 'Evaluate', 'Create'
       ))
       OR (p_bloom_knowledge_dimension IS NOT NULL AND p_bloom_knowledge_dimension NOT IN (
           'Factual Knowledge', 'Conceptual Knowledge', 'Procedural Knowledge',
           'Metacognitive Knowledge'
       )) THEN
        RETURN;
    END IF;
    RETURN QUERY
    WITH filtered AS MATERIALIZED (
        SELECT pool.question_pool_id,
               pool.question_pool_edit_number,
               (SELECT count(*)::integer FROM ple_data.question_pool_member AS member
                 WHERE member.question_pool_id = pool.question_pool_id) AS member_count,
               pool.title, pool.description, pool.content_discipline_id, discipline.name AS discipline_name,
               discipline.is_retired AS discipline_is_retired, pool.content_subject_id,
               pool.content_topic_id, pool.content_subtopic_id, pool.tags,
               bloom.cognitive_process::text AS bloom_cognitive_process,
               bloom.knowledge_dimension::text AS bloom_knowledge_dimension,
               bloom.classification_edit_number
          FROM ple_data.question_pool AS pool
          LEFT JOIN ple_data.question_pool_bloom AS bloom
            ON bloom.question_pool_id = pool.question_pool_id
          -- ASVS 8.2.2/8.2.3: reuse authorized vocabulary projections. Every
          -- predicate below describes this Pool and its own pair, never a member.
          LEFT JOIN ple_api.list_content_disciplines_including_retired() AS discipline
            ON discipline.content_discipline_id = pool.content_discipline_id
          LEFT JOIN LATERAL ple_api.list_content_subjects(pool.content_discipline_id) AS subject
            ON subject.content_subject_id = pool.content_subject_id
          LEFT JOIN LATERAL ple_api.list_content_topics(pool.content_subject_id) AS topic
            ON topic.content_topic_id = pool.content_topic_id
          LEFT JOIN LATERAL ple_api.list_content_subtopics(pool.content_topic_id) AS subtopic
            ON subtopic.content_subtopic_id = pool.content_subtopic_id
         WHERE (ple_api.current_session_account_is_instructor()
                OR ple_api.current_session_account_has_platform_administration())
           AND (p_discipline_uuid IS NULL OR p_cross_discipline
                OR pool.content_discipline_id = p_discipline_uuid)
           AND (p_subject_uuid IS NULL OR pool.content_subject_id = p_subject_uuid)
           AND (p_topic_uuid IS NULL OR pool.content_topic_id = p_topic_uuid)
           AND (p_subtopic_uuid IS NULL OR pool.content_subtopic_id = p_subtopic_uuid)
           AND (p_bloom_cognitive_process IS NULL
                OR bloom.cognitive_process::text = p_bloom_cognitive_process)
           AND (p_bloom_knowledge_dimension IS NULL
                OR bloom.knowledge_dimension::text = p_bloom_knowledge_dimension)
           -- ASVS 1.2.4: static bound predicates; %, _ and SQL syntax are literal data.
           AND (cardinality(p_tags) = 0 OR EXISTS (
               SELECT 1 FROM unnest(pool.tags) AS tag(value)
                WHERE lower(btrim(regexp_replace(tag.value, '[[:space:]]+', ' ', 'g'))) = ANY(p_tags)
           ))
           AND NOT EXISTS (
               SELECT 1 FROM jsonb_to_recordset(p_terms)
                    AS term(field text, value text, excluded boolean)
                WHERE term.value = ''
                   OR term.field NOT IN ('any', 'discipline', 'subject', 'topic', 'subtopic', 'tags')
                   OR term.excluded IS NULL OR term.value IS NULL OR term.field IS NULL
                   OR (EXISTS (
                       SELECT 1 FROM (
                           SELECT pool.title AS value WHERE term.field = 'any'
                           UNION ALL SELECT pool.description WHERE term.field = 'any'
                           UNION ALL SELECT discipline.name WHERE term.field IN ('any', 'discipline')
                           UNION ALL SELECT subject.name WHERE term.field IN ('any', 'subject')
                           UNION ALL SELECT topic.name WHERE term.field IN ('any', 'topic')
                           UNION ALL SELECT subtopic.name WHERE term.field IN ('any', 'subtopic')
                           UNION ALL SELECT tag.value FROM unnest(pool.tags) AS tag(value)
                                WHERE term.field IN ('any', 'tags')
                       ) AS candidate
                       WHERE strpos(lower(candidate.value), term.value) > 0
                   ) = term.excluded)
           )
    ), page AS (
        SELECT matched.* FROM filtered AS matched
         WHERE (p_after IS NULL OR matched.question_pool_id > p_after)
           AND (p_after IS NULL OR (
               p_after ~ '^[0-9A-HJKMNP-TV-Z]{4}-[0-9A-HJKMNP-TV-Z]{4}$'
               AND substr(p_after, 6, 1) = ple_private.crockford_checksum_character(
                   substr(p_after, 1, 4) || substr(p_after, 7, 3)
               )
           ))
         ORDER BY matched.question_pool_id
         LIMIT p_page_size + 1
    ), aggregates AS (
        SELECT ARRAY[
                   count(*) FILTER (WHERE matched.bloom_cognitive_process = 'Remember'),
                   count(*) FILTER (WHERE matched.bloom_cognitive_process = 'Understand'),
                   count(*) FILTER (WHERE matched.bloom_cognitive_process = 'Apply'),
                   count(*) FILTER (WHERE matched.bloom_cognitive_process = 'Analyze'),
                   count(*) FILTER (WHERE matched.bloom_cognitive_process = 'Evaluate'),
                   count(*) FILTER (WHERE matched.bloom_cognitive_process = 'Create')
               ]::bigint[] AS cognitive_counts,
               ARRAY[
                   count(*) FILTER (WHERE matched.bloom_knowledge_dimension = 'Factual Knowledge'),
                   count(*) FILTER (WHERE matched.bloom_knowledge_dimension = 'Conceptual Knowledge'),
                   count(*) FILTER (WHERE matched.bloom_knowledge_dimension = 'Procedural Knowledge'),
                   count(*) FILTER (WHERE matched.bloom_knowledge_dimension = 'Metacognitive Knowledge')
               ]::bigint[] AS knowledge_counts
          FROM filtered AS matched
    )
    SELECT page.question_pool_id, page.question_pool_edit_number, page.member_count,
           page.title, page.description, page.content_discipline_id, page.discipline_name,
           page.discipline_is_retired, page.content_subject_id, page.content_topic_id,
           page.content_subtopic_id, page.tags, page.bloom_cognitive_process,
           page.bloom_knowledge_dimension, page.classification_edit_number,
           aggregates.cognitive_counts, aggregates.knowledge_counts
      FROM aggregates
      LEFT JOIN page ON true
     ORDER BY page.question_pool_id NULLS LAST;
END
$$;

CREATE FUNCTION ple_api.read_current_published_question_pool(p_question_pool_id text)
RETURNS TABLE (
    question_pool_id text,
    question_pool_edit_number bigint,
    member_position integer,
    published_question_id text,
    question_revision_number integer,
    title text, description text, content_discipline_id uuid, discipline_name text,
    discipline_is_retired boolean, content_subject_id uuid,
    content_topic_id uuid, content_subtopic_id uuid, tags text[],
    bloom_cognitive_process text, bloom_knowledge_dimension text,
    bloom_classification_edit_number bigint
) LANGUAGE plpgsql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data AS $$
BEGIN
    RETURN QUERY
    SELECT pool.question_pool_id,
           pool.question_pool_edit_number, member.member_position,
           member.published_question_id,
           member.question_revision_number,
           pool.title, pool.description, pool.content_discipline_id, discipline.name,
           discipline.is_retired, pool.content_subject_id,
           pool.content_topic_id, pool.content_subtopic_id, pool.tags,
           bloom.cognitive_process::text, bloom.knowledge_dimension::text,
           bloom.classification_edit_number
      FROM ple_data.question_pool AS pool
      LEFT JOIN ple_data.question_pool_bloom AS bloom
        ON bloom.question_pool_id = pool.question_pool_id
      JOIN ple_data.question_pool_member AS member
        ON member.question_pool_id = pool.question_pool_id
      JOIN LATERAL ple_api.list_content_disciplines_including_retired() AS discipline
        ON discipline.content_discipline_id = pool.content_discipline_id
     WHERE (ple_api.current_session_account_is_instructor()
            OR ple_api.current_session_account_has_platform_administration())
       AND pool.question_pool_id = p_question_pool_id
     ORDER BY member.member_position;
END
$$;
