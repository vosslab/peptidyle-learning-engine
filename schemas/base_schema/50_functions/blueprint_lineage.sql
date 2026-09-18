-- Functions, triggers, and views from blueprint_lineage.sql.

SET LOCAL ROLE ple_api_owner;




-- ASVS 8.2.2/3, 8.3.1/2, 14.2.6: source permission never grants access to
-- another Instructor's Private child or identity. One statement snapshot keeps
-- current visibility, names and heads coherent without locking a read-only list.
CREATE FUNCTION ple_api.list_known_blueprint_forks(p_source_reference text)
RETURNS TABLE (
    public_reference text,
    short_name text,
    long_name text,
    availability text,
    blueprint_revision_number bigint,
    source_blueprint_revision_number bigint,
    owner_display_name text
) LANGUAGE plpgsql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
DECLARE
    v_actor uuid;
    v_source_reference_number bigint;
BEGIN
    v_actor := ple_api.current_session_account_id();
    IF p_source_reference IS NULL
       OR NOT ple_private.is_canonical_prefixed_public_id(p_source_reference, 'BP')
       OR v_actor IS NULL
       OR NOT ple_api.current_session_account_is_instructor()
       OR ple_private.verified_instructor_display_name(v_actor) IS NULL THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Blueprint Course is unavailable';
    END IF;
    SELECT source.reference_number INTO v_source_reference_number
      FROM ple_data.blueprint_course AS source
     WHERE source.public_reference = p_source_reference
       AND (source.availability IN ('public', 'archived')
           OR source.owner_account_id = v_actor);
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Blueprint Course is unavailable';
    END IF;
    RETURN QUERY
    SELECT child.public_reference, child.short_name, child.long_name,
           child.availability, child.current_blueprint_revision_number,
           ancestry.source_blueprint_revision_number,
           ple_private.verified_instructor_display_name(child.owner_account_id)
      FROM ple_data.blueprint_course_fork AS ancestry
      JOIN ple_data.blueprint_course AS child
        ON child.reference_number = ancestry.blueprint_course_id
     WHERE ancestry.source_blueprint_course_reference_number = v_source_reference_number
       AND (child.availability IN ('public', 'archived')
           OR child.owner_account_id = v_actor)
     ORDER BY child.public_reference COLLATE "C";
END
$$;



-- Authorized retained reads only: comparison uses canonical JSON in the domain.
-- ASVS 8.2.2, 8.3.1/2: both lineages follow current ordinary visibility.
CREATE FUNCTION ple_api.load_blueprint_comparison_sources(
    p_left_reference text, p_right_reference text
) RETURNS TABLE (
    source_position integer,
    public_reference text,
    blueprint_revision_number bigint,
    content jsonb,
    content_checksum bytea,
    left_short_name text,
    left_long_name text,
    right_short_name text,
    right_long_name text,
    left_metadata_etag uuid,
    right_metadata_etag uuid
) LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
DECLARE
    v_actor uuid;
    v_fork ple_data.blueprint_course%ROWTYPE;
    v_source ple_data.blueprint_course%ROWTYPE;
    v_locked_reference bigint;
BEGIN
    IF p_left_reference IS NULL OR p_right_reference IS NULL
       OR NOT ple_private.is_canonical_prefixed_public_id(p_left_reference, 'BP')
       OR NOT ple_private.is_canonical_prefixed_public_id(p_right_reference, 'BP')
       OR NOT ple_api.current_session_account_is_instructor() THEN
        RETURN;
    END IF;
    v_actor := ple_api.current_session_account_id();

    -- Existing save, rename and lifecycle operations lock one Course FOR UPDATE;
    -- fork creation locks its source FOR SHARE before inserting a new child.
    -- Acquire both existing Course rows in ascending stable identity order.
    -- Future selective apply must reuse this order BEFORE any fork write lock,
    -- taking its strongest needed locks initially rather than upgrading later.
    FOR v_locked_reference IN
        SELECT course.reference_number FROM ple_data.blueprint_course AS course
         WHERE course.public_reference IN (p_left_reference, p_right_reference)
         ORDER BY course.reference_number
    LOOP
        PERFORM 1 FROM ple_data.blueprint_course AS course
         WHERE course.reference_number = v_locked_reference FOR SHARE;
    END LOOP;

    -- Reread names, heads, owners and availability AFTER waiting for locks.
    SELECT course.* INTO v_fork FROM ple_data.blueprint_course AS course
     WHERE course.public_reference = p_right_reference;
    IF NOT FOUND THEN RETURN; END IF;
    SELECT course.* INTO v_source FROM ple_data.blueprint_course AS course
     WHERE course.public_reference = p_left_reference;
    IF NOT FOUND THEN RETURN; END IF;
    IF NOT ple_api.current_session_account_is_instructor()
       OR NOT (v_fork.availability IN ('public', 'archived')
           OR v_fork.owner_account_id = v_actor)
       OR NOT (v_source.availability IN ('public', 'archived')
           OR v_source.owner_account_id = v_actor) THEN
        RETURN;
    END IF;
    -- Only selected records are projected. Private ancestors establish relation
    -- internally without disclosing their identity, content or lineage size.
    IF NOT EXISTS (
        WITH RECURSIVE left_ancestors(reference_number) AS (
            SELECT v_source.reference_number
            UNION
            SELECT ancestry.source_blueprint_course_reference_number
              FROM ple_data.blueprint_course_fork AS ancestry
              JOIN left_ancestors AS ancestor ON ancestor.reference_number =
                  ancestry.blueprint_course_id
        ), right_ancestors(reference_number) AS (
            SELECT v_fork.reference_number
            UNION
            SELECT ancestry.source_blueprint_course_reference_number
              FROM ple_data.blueprint_course_fork AS ancestry
              JOIN right_ancestors AS ancestor ON ancestor.reference_number =
                  ancestry.blueprint_course_id
        )
        SELECT 1 FROM left_ancestors JOIN right_ancestors USING (reference_number)
    ) THEN RETURN; END IF;
    RETURN QUERY
    SELECT inputs.position, inputs.reference, revision.blueprint_revision_number,
           revision.content, revision.content_checksum,
           v_source.short_name, v_source.long_name, v_fork.short_name, v_fork.long_name,
           v_source.metadata_etag, v_fork.metadata_etag
      FROM (VALUES
          (0, v_source.reference_number, v_source.public_reference,
              v_source.current_blueprint_revision_number),
          (1, v_fork.reference_number, v_fork.public_reference,
              v_fork.current_blueprint_revision_number)
      ) AS inputs(position, reference_number, reference, revision_number)
      JOIN ple_data.blueprint_course_revision AS revision
        ON revision.blueprint_course_id = inputs.reference_number
       AND revision.blueprint_revision_number = inputs.revision_number
     ORDER BY inputs.position;
END
$$;



-- Trusted current-tree read for selective apply, not a client-content write.
-- ASVS 8.2.2, 8.3.1, 2.3.3: strongest locks initially, ordinary visibility
-- and all four preconditions before any Save/rename or receipt replay.
CREATE FUNCTION ple_api.load_blueprint_fork_apply_sources(
    p_source_reference text, p_source_revision bigint, p_source_etag uuid,
    p_fork_reference text, p_fork_revision bigint, p_fork_etag uuid
) RETURNS TABLE (source_position integer, content jsonb, content_checksum bytea,
    short_name text, long_name text)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
DECLARE
    v_actor uuid;
    v_origin ple_data.blueprint_course_fork%ROWTYPE;
    v_source ple_data.blueprint_course%ROWTYPE;
    v_fork ple_data.blueprint_course%ROWTYPE;
    v_reference bigint;
BEGIN
    v_actor := ple_api.current_session_account_id();
    IF v_actor IS NULL OR NOT ple_api.current_session_account_is_instructor() THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Blueprint Course is unavailable';
    END IF;
    SELECT ancestry.* INTO v_origin FROM ple_data.blueprint_course_fork AS ancestry
      JOIN ple_data.blueprint_course AS fork_course
        ON fork_course.reference_number = ancestry.blueprint_course_id
      JOIN ple_data.blueprint_course AS source_course
        ON source_course.reference_number = ancestry.source_blueprint_course_reference_number
     WHERE fork_course.public_reference = p_fork_reference
       AND source_course.public_reference = p_source_reference;
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Blueprint Course is unavailable';
    END IF;
    FOR v_reference IN SELECT course.reference_number FROM ple_data.blueprint_course AS course
      WHERE course.reference_number IN (v_origin.blueprint_course_id,
          v_origin.source_blueprint_course_reference_number) ORDER BY course.reference_number
    LOOP
        IF v_reference = v_origin.blueprint_course_id THEN
            PERFORM 1 FROM ple_data.blueprint_course WHERE reference_number = v_reference FOR UPDATE;
        ELSE
            PERFORM 1 FROM ple_data.blueprint_course WHERE reference_number = v_reference FOR SHARE;
        END IF;
    END LOOP;
    SELECT course.* INTO STRICT v_source FROM ple_data.blueprint_course AS course
      WHERE course.reference_number = v_origin.source_blueprint_course_reference_number;
    SELECT course.* INTO STRICT v_fork FROM ple_data.blueprint_course AS course
      WHERE course.reference_number = v_origin.blueprint_course_id;
    IF NOT ple_api.current_session_account_is_instructor()
       OR v_fork.owner_account_id <> v_actor
       OR NOT (v_source.availability IN ('public', 'archived') OR v_source.owner_account_id = v_actor) THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Blueprint Course is unavailable';
    END IF;
    IF v_fork.availability = 'archived' THEN
        RAISE EXCEPTION USING ERRCODE = '55000', MESSAGE = 'Archived Blueprint Course is read-only';
    END IF;
    IF p_source_revision IS NULL OR p_fork_revision IS NULL
       OR p_source_etag IS NULL OR p_fork_etag IS NULL
       OR v_source.current_blueprint_revision_number <> p_source_revision
       OR v_fork.current_blueprint_revision_number <> p_fork_revision
       OR v_source.metadata_etag <> p_source_etag OR v_fork.metadata_etag <> p_fork_etag THEN
        RAISE EXCEPTION USING ERRCODE = '40001', MESSAGE = 'Blueprint fork apply precondition is stale';
    END IF;
    RETURN QUERY SELECT inputs.position, revision.content, revision.content_checksum,
        inputs.current_short_name, inputs.current_long_name
      FROM (VALUES (0, v_source.reference_number, p_source_revision, v_source.short_name, v_source.long_name),
          (1, v_fork.reference_number, p_fork_revision, v_fork.short_name, v_fork.long_name))
          AS inputs(position, reference_number, revision_number, current_short_name, current_long_name)
      JOIN ple_data.blueprint_course_revision AS revision
        ON revision.blueprint_course_id = inputs.reference_number
       AND revision.blueprint_revision_number = inputs.revision_number ORDER BY inputs.position;
END
$$;



-- Trusted fork assembly reads an exact immutable source under the same source
-- lifecycle lock and request-retry lock held until the child write commits.
CREATE FUNCTION ple_api.load_blueprint_fork_source(
    p_source_reference text, p_source_revision_number bigint, p_request_checksum bytea
) RETURNS TABLE (content jsonb, content_checksum bytea)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data AS $$
DECLARE
    v_actor uuid;
    v_source_reference bigint;
BEGIN
    IF p_source_reference IS NULL
       OR NOT ple_private.is_canonical_prefixed_public_id(p_source_reference, 'BP')
       OR p_source_revision_number IS NULL OR p_source_revision_number <= 0
       OR p_request_checksum IS NULL OR octet_length(p_request_checksum) <> 32
       OR NOT ple_api.current_session_account_is_instructor() THEN
        RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Blueprint Course fork is invalid';
    END IF;
    v_actor := ple_api.current_session_account_id();
    PERFORM pg_catalog.pg_advisory_xact_lock(pg_catalog.hashtextextended(
        pg_catalog.format('ple:blueprint-course-fork:%s:%s', v_actor,
            pg_catalog.encode(p_request_checksum, 'hex')), 0));
    IF EXISTS (SELECT 1 FROM ple_data.blueprint_course_fork_receipt AS receipt
        WHERE receipt.actor_account_id = v_actor AND receipt.request_checksum = p_request_checksum) THEN
        RETURN QUERY SELECT NULL::jsonb, NULL::bytea;
        RETURN;
    END IF;
    SELECT course.reference_number INTO v_source_reference
      FROM ple_data.blueprint_course AS course
     WHERE course.public_reference = p_source_reference
       AND course.availability IN ('public', 'archived') FOR SHARE;
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Blueprint Course fork requires a Public or Archived source';
    END IF;
    RETURN QUERY SELECT revision.content, revision.content_checksum
      FROM ple_data.blueprint_course_revision AS revision
     WHERE revision.blueprint_course_id = v_source_reference
       AND revision.blueprint_revision_number = p_source_revision_number;
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '23503',
            MESSAGE = 'Blueprint Course fork source Revision is unavailable';
    END IF;
END
$$;

CREATE FUNCTION ple_api.fork_blueprint_course(
    p_blueprint_id uuid,
    p_source_reference text,
    p_source_revision_number bigint,
    p_request_checksum bytea,
    p_content jsonb,
    p_content_checksum bytea
) RETURNS TABLE (
    public_reference text,
    blueprint_revision_number bigint,
    metadata_etag uuid,
    accepted_at timestamp with time zone
) LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
DECLARE
    v_actor uuid;
    v_source ple_data.blueprint_course%ROWTYPE;
    v_source_revision ple_data.blueprint_course_revision%ROWTYPE;
    v_child_reference bigint;
    v_child_revision bigint := 1;
    v_now timestamp with time zone;
    v_metadata_etag uuid;
    v_source_reference_number bigint;
    v_source_module jsonb;
    v_child_module jsonb;
    v_source_assessment jsonb;
    v_child_assessment jsonb;
    v_module_position integer;
    v_assessment_position integer;
    v_entry_position integer;
    v_source_entry jsonb;
    v_child_entry jsonb;
    v_source_pool ple_data.question_pool%ROWTYPE;
    v_child_pool ple_data.question_pool%ROWTYPE;
    v_source_pool_revision bigint;
BEGIN
    IF p_blueprint_id IS NULL
       OR NOT ple_private.is_canonical_prefixed_public_id(p_source_reference, 'BP')
       OR p_source_revision_number <= 0
       OR octet_length(p_request_checksum) <> 32
       OR NOT ple_api.current_session_account_is_instructor() THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Blueprint Course fork is invalid';
    END IF;
    v_actor := ple_api.current_session_account_id();
    SELECT course.reference_number INTO v_source_reference_number
      FROM ple_data.blueprint_course AS course
     WHERE course.public_reference = p_source_reference;
    PERFORM pg_catalog.pg_advisory_xact_lock(pg_catalog.hashtextextended(
        pg_catalog.format('ple:blueprint-course-fork:%s:%s', v_actor,
            pg_catalog.encode(p_request_checksum, 'hex')), 0));
    SELECT course.public_reference, 1, receipt.metadata_etag,
           receipt.accepted_at
      INTO public_reference, blueprint_revision_number, metadata_etag, accepted_at
      FROM ple_data.blueprint_course_fork_receipt AS receipt
      JOIN ple_data.blueprint_course AS course
        ON course.reference_number = receipt.blueprint_course_id
     WHERE receipt.actor_account_id = v_actor
       AND receipt.request_checksum = p_request_checksum;
    IF FOUND THEN
        RETURN NEXT;
        RETURN;
    END IF;

    -- Lifecycle is evaluated at the transaction boundary. A source that is
    -- changed to Private concurrently cannot be forked after this lock.
    SELECT * INTO v_source
      FROM ple_data.blueprint_course AS source_course
     WHERE source_course.reference_number = v_source_reference_number
       AND source_course.availability IN ('public', 'archived')
     FOR SHARE;
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Blueprint Course fork requires a Public or Archived source';
    END IF;
    SELECT * INTO v_source_revision
      FROM ple_data.blueprint_course_revision AS source_revision
     WHERE source_revision.blueprint_course_id = v_source_reference_number
       AND source_revision.blueprint_revision_number = p_source_revision_number;
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '23503',
            MESSAGE = 'Blueprint Course fork source Revision is unavailable';
    END IF;

    PERFORM ple_data.validate_blueprint_content(p_content);
    IF p_content IS NULL OR p_content_checksum IS NULL
       OR octet_length(p_content_checksum) <> 32
       OR jsonb_array_length(p_content -> 'modules') <>
          jsonb_array_length(v_source_revision.content -> 'modules')
       OR EXISTS (
           SELECT 1 FROM ple_data.blueprint_content_modules(p_content) AS child
           JOIN ple_data.blueprint_content_modules(v_source_revision.content) AS source
             USING (blueprint_module_reference))
       OR EXISTS (
           SELECT 1 FROM ple_data.blueprint_content_assessments(p_content) AS child
           JOIN ple_data.blueprint_content_assessments(v_source_revision.content) AS source
             USING (blueprint_assessment_reference)) THEN
        RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Blueprint fork content is invalid';
    END IF;
    -- Local identities and owned Pool identities differ. Every other authored
    -- value, exact source member pin, attestation and position remains equal.
    FOR v_source_module, v_module_position IN
        SELECT module, ordinality::integer FROM jsonb_array_elements(v_source_revision.content -> 'modules')
            WITH ORDINALITY AS modules(module, ordinality)
    LOOP
        v_child_module := p_content -> 'modules' -> (v_module_position - 1);
        IF (v_child_module - 'blueprint_module_reference' - 'assessments') <>
           (v_source_module - 'blueprint_module_reference' - 'assessments')
           OR jsonb_array_length(v_child_module -> 'assessments') <>
              jsonb_array_length(v_source_module -> 'assessments') THEN
            RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Blueprint fork content differs from source';
        END IF;
        FOR v_source_assessment, v_assessment_position IN
            SELECT assessment, ordinality::integer FROM jsonb_array_elements(v_source_module -> 'assessments')
                WITH ORDINALITY AS assessments(assessment, ordinality)
        LOOP
            v_child_assessment := v_child_module -> 'assessments' -> (v_assessment_position - 1);
            IF jsonb_array_length(v_child_assessment #> '{content,entries}') <>
               jsonb_array_length(v_source_assessment #> '{content,entries}') THEN
                RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Blueprint fork content differs from source';
            END IF;
            FOR v_source_entry, v_entry_position IN
                SELECT entry, ordinality::integer FROM jsonb_array_elements(v_source_assessment #> '{content,entries}')
                    WITH ORDINALITY AS entries(entry, ordinality)
            LOOP
                v_child_entry := v_child_assessment #> ARRAY['content','entries',(v_entry_position - 1)::text];
                IF v_source_entry ? 'question_pool_revision' THEN
                    SELECT * INTO v_source_pool FROM ple_data.question_pool
                     WHERE public_question_pool_id = v_source_entry #>> '{question_pool_revision,questionPoolId}';
                    SELECT * INTO v_child_pool FROM ple_data.question_pool
                     WHERE public_question_pool_id = v_child_entry #>> '{question_pool_revision,questionPoolId}';
                    v_source_pool_revision := (v_source_entry #>> '{question_pool_revision,revisionNumber}')::bigint;
                    IF v_child_pool.question_pool_id IS NULL
                       OR v_child_pool.source_question_pool_id IS DISTINCT FROM v_source_pool.question_pool_id
                       OR v_child_pool.source_question_pool_revision_number IS DISTINCT FROM v_source_pool_revision
                       OR (v_child_entry #>> '{question_pool_revision,revisionNumber}')::bigint IS DISTINCT FROM 1
                       OR NOT EXISTS (
                           SELECT 1 FROM ple_data.question_pool_revision AS child
                           JOIN ple_data.question_pool_revision AS source
                             ON source.question_pool_id = v_source_pool.question_pool_id
                            AND source.revision_number = v_source_pool_revision
                          WHERE child.question_pool_id = v_child_pool.question_pool_id AND child.revision_number = 1
                            AND child.member_count = source.member_count
                            AND child.interchangeability_attested_by_account_id = source.interchangeability_attested_by_account_id
                            AND child.interchangeability_attested_at = source.interchangeability_attested_at)
                       OR EXISTS (
                           SELECT 1 FROM
                               (SELECT member_position, published_question_id, question_revision_number
                                  FROM ple_data.question_pool_revision_member
                                 WHERE question_pool_id = v_child_pool.question_pool_id AND revision_number = 1) AS child
                           FULL JOIN
                               (SELECT member_position, published_question_id, question_revision_number
                                  FROM ple_data.question_pool_revision_member
                                 WHERE question_pool_id = v_source_pool.question_pool_id AND revision_number = v_source_pool_revision) AS source
                             USING (member_position)
                           WHERE child.published_question_id IS DISTINCT FROM source.published_question_id
                              OR child.question_revision_number IS DISTINCT FROM source.question_revision_number) THEN
                        RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Blueprint fork Pool differs from exact source';
                    END IF;
                    v_child_assessment := jsonb_set(v_child_assessment,
                        ARRAY['content','entries',(v_entry_position - 1)::text,'question_pool_revision'],
                        v_source_entry -> 'question_pool_revision');
                END IF;
            END LOOP;
            IF (v_child_assessment - 'blueprint_assessment_reference') <>
               (v_source_assessment - 'blueprint_assessment_reference') THEN
                RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Blueprint fork content differs from source';
            END IF;
        END LOOP;
    END LOOP;
    v_now := pg_catalog.clock_timestamp();
    v_metadata_etag := pg_catalog.gen_random_uuid();
    INSERT INTO ple_data.blueprint_course AS child (
        blueprint_id, owner_account_id, short_name, long_name, availability,
        metadata_etag, current_blueprint_revision_number, created_at,
        content_discipline_id, content_subject_id, content_topic_id, content_subtopic_id, tags
    ) VALUES (
        p_blueprint_id, v_actor, v_source.short_name, v_source.long_name, 'private',
        v_metadata_etag, 1, v_now,
        v_source.content_discipline_id, v_source.content_subject_id, v_source.content_topic_id,
        v_source.content_subtopic_id, v_source.tags
    ) RETURNING child.reference_number INTO v_child_reference;
    INSERT INTO ple_data.blueprint_course_revision (
        blueprint_course_id, blueprint_revision_number,
        content, content_checksum, saved_at
    ) VALUES (
        v_child_reference, v_child_revision, p_content,
        p_content_checksum, v_now
    );
    INSERT INTO ple_data.blueprint_revision_question_pin
    SELECT v_child_reference, v_child_revision, pin.content_path,
           pin.published_question_id, pin.question_revision_number
      FROM ple_data.blueprint_content_question_pins(p_content) AS pin;
    INSERT INTO ple_data.blueprint_revision_module
    SELECT v_child_reference, v_child_revision, member.blueprint_module_reference,
           member.module_position
      FROM ple_data.blueprint_content_modules(p_content) AS member;
    INSERT INTO ple_data.blueprint_revision_assessment
    SELECT v_child_reference, v_child_revision, member.blueprint_module_reference,
           member.blueprint_assessment_reference, member.assessment_position
      FROM ple_data.blueprint_content_assessments(p_content) AS member;
    INSERT INTO ple_data.blueprint_revision_event (
        blueprint_course_id, blueprint_revision_number, actor_account_id,
        request_checksum, occurred_at
    ) VALUES (
        v_child_reference, v_child_revision, v_actor, p_request_checksum, v_now
    );
    INSERT INTO ple_data.blueprint_metadata_event (
        blueprint_course_id, actor_account_id, short_name, long_name,
        availability, metadata_etag, occurred_at,
        content_discipline_id, content_subject_id, content_topic_id, content_subtopic_id, tags
    ) VALUES (
        v_child_reference, v_actor, v_source.short_name, v_source.long_name,
        'private', v_metadata_etag, v_now,
        v_source.content_discipline_id, v_source.content_subject_id, v_source.content_topic_id,
        v_source.content_subtopic_id, v_source.tags
    );
    INSERT INTO ple_data.blueprint_course_fork VALUES (
        v_child_reference, v_source_reference_number, p_source_revision_number, v_now
    );
    INSERT INTO ple_data.blueprint_course_fork_receipt VALUES (
        v_actor, p_request_checksum, v_child_reference, v_source_reference_number,
        p_source_revision_number, v_metadata_etag, v_now
    );
    SELECT course.public_reference INTO public_reference
      FROM ple_data.blueprint_course AS course
     WHERE course.reference_number = v_child_reference;
    blueprint_revision_number := v_child_revision;
    metadata_etag := v_metadata_etag;
    accepted_at := v_now;
    RETURN NEXT;
END
$$;

